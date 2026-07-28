use crate::common::*;
use crate::database;
use camellia_remote_protocol::{
    anyhow::Context as _,
    bytes::Bytes,
    log,
    tokio::sync::{Mutex, RwLock},
    ResultType,
};
use serde_derive::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    collections::HashSet,
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};

type IpBlockMap = HashMap<String, ((u32, Instant), (HashSet<String>, Instant))>;
type IpChangesMap = HashMap<String, (Instant, HashMap<String, i32>)>;
lazy_static::lazy_static! {
    pub(crate) static ref IP_BLOCKER: Mutex<IpBlockMap> = Default::default();
    pub(crate) static ref IP_CHANGES: Mutex<IpChangesMap> = Default::default();
}
pub const IP_CHANGE_DUR: u64 = 180;
pub const IP_CHANGE_DUR_X2: u64 = IP_CHANGE_DUR * 2;
pub const DAY_SECONDS: u64 = 3600 * 24;
pub const IP_BLOCK_DUR: u64 = 60;
const IP_REQUEST_LIMIT: u32 = 30;
const IP_DISTINCT_ID_LIMIT: usize = 300;
const MAX_TRACKED_IP_BLOCKERS: usize = 65_536;
const MAX_TRACKED_IP_CHANGES: usize = 100_000;
const MAX_IPS_PER_CHANGE_ENTRY: usize = 512;
const DEFAULT_MAX_CACHED_PEERS: usize = 100_000;
const PEER_CACHE_ACTIVE_GRACE_SECS: u64 = 60;

fn window_expired(now: Instant, started_at: Instant, seconds: u64) -> bool {
    now.saturating_duration_since(started_at) >= Duration::from_secs(seconds)
}

fn allow_ip_request_in(
    blockers: &mut IpBlockMap,
    ip: &str,
    id: &str,
    now: Instant,
    max_tracked_ips: usize,
) -> bool {
    if !blockers.contains_key(ip) {
        if blockers.len() >= max_tracked_ips {
            blockers.retain(|_, (requests, ids)| {
                !window_expired(now, requests.1, IP_BLOCK_DUR)
                    || !window_expired(now, ids.1, DAY_SECONDS)
            });
        }
        if blockers.len() >= max_tracked_ips {
            return false;
        }
        blockers.insert(ip.to_owned(), ((0, now), (HashSet::with_capacity(1), now)));
    }

    let Some(entry) = blockers.get_mut(ip) else {
        return false;
    };
    if window_expired(now, entry.0 .1, IP_BLOCK_DUR) {
        entry.0 = (0, now);
    }
    if entry.0 .0 >= IP_REQUEST_LIMIT {
        return false;
    }
    entry.0 .0 += 1;

    if window_expired(now, entry.1 .1, DAY_SECONDS) {
        entry.1 = (HashSet::with_capacity(1), now);
    }
    if !entry.1 .0.contains(id) {
        if entry.1 .0.len() >= IP_DISTINCT_ID_LIMIT {
            return false;
        }
        entry.1 .0.insert(id.to_owned());
    }
    true
}

pub(crate) async fn allow_ip_request(ip: &str, id: &str) -> bool {
    let mut blockers = IP_BLOCKER.lock().await;
    allow_ip_request_in(
        &mut blockers,
        ip,
        id,
        Instant::now(),
        MAX_TRACKED_IP_BLOCKERS,
    )
}

fn record_ip_change_in(
    changes: &mut IpChangesMap,
    id: &str,
    ip: &str,
    now: Instant,
    max_tracked_ids: usize,
) -> bool {
    if !changes.contains_key(id) {
        if changes.len() >= max_tracked_ids {
            changes
                .retain(|_, (started_at, _)| !window_expired(now, *started_at, IP_CHANGE_DUR_X2));
        }
        if changes.len() >= max_tracked_ids {
            return false;
        }
        changes.insert(id.to_owned(), (now, HashMap::with_capacity(1)));
    }

    let Some(entry) = changes.get_mut(id) else {
        return false;
    };
    if window_expired(now, entry.0, IP_CHANGE_DUR) {
        entry.0 = now;
        entry.1.clear();
    }
    if let Some(count) = entry.1.get_mut(ip) {
        *count = count.saturating_add(1);
        return true;
    }
    if entry.1.len() >= MAX_IPS_PER_CHANGE_ENTRY {
        return false;
    }
    entry.1.insert(ip.to_owned(), 1);
    true
}

pub(crate) async fn record_ip_change(id: &str, ip: &str) -> bool {
    let mut changes = IP_CHANGES.lock().await;
    let accepted =
        record_ip_change_in(&mut changes, id, ip, Instant::now(), MAX_TRACKED_IP_CHANGES);
    if !accepted {
        log::warn!("IP change diagnostics capacity reached for peer {id}");
    }
    accepted
}

pub(crate) async fn cleanup_transient_state() -> (usize, usize) {
    let now = Instant::now();
    let mut blockers = IP_BLOCKER.lock().await;
    let blockers_before = blockers.len();
    blockers.retain(|_, (requests, ids)| {
        !window_expired(now, requests.1, IP_BLOCK_DUR) || !window_expired(now, ids.1, DAY_SECONDS)
    });
    let blockers_removed = blockers_before - blockers.len();
    drop(blockers);

    let mut changes = IP_CHANGES.lock().await;
    let changes_before = changes.len();
    changes.retain(|_, (started_at, _)| !window_expired(now, *started_at, IP_CHANGE_DUR_X2));
    (blockers_removed, changes_before - changes.len())
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub(crate) struct PeerInfo {
    #[serde(default)]
    pub(crate) ip: String,
}

pub(crate) struct Peer {
    pub(crate) socket_addr: SocketAddr,
    pub(crate) last_reg_time: Instant,
    last_access_time: Instant,
    pub(crate) guid: Vec<u8>,
    pub(crate) uuid: Bytes,
    pub(crate) pk: Bytes,
    pub(crate) persistence_in_progress: bool,
    pub(crate) deployment_verified_at: Option<Instant>,
    // pub(crate) user: Option<Vec<u8>>,
    pub(crate) info: PeerInfo,
    // pub(crate) disabled: bool,
    pub(crate) reg_pk: (u32, Instant), // how often register_pk
}

impl Default for Peer {
    fn default() -> Self {
        Self {
            socket_addr: SocketAddr::from(([0, 0, 0, 0], 0)),
            last_reg_time: get_expired_time(),
            last_access_time: Instant::now(),
            guid: Vec::new(),
            uuid: Bytes::new(),
            pk: Bytes::new(),
            persistence_in_progress: false,
            deployment_verified_at: None,
            info: Default::default(),
            // user: None,
            // disabled: false,
            reg_pk: (0, get_expired_time()),
        }
    }
}

pub(crate) type LockPeer = Arc<RwLock<Peer>>;

#[derive(Clone)]
pub(crate) struct PeerMap {
    map: Arc<RwLock<HashMap<String, LockPeer>>>,
    pub(crate) db: database::Database,
    max_cached_peers: usize,
}

impl PeerMap {
    pub(crate) async fn new() -> ResultType<Self> {
        let db = get_arg_opt("DB_URL").unwrap_or_else(|| {
            let mut db = "camellia-remote.sqlite3".to_owned();
            #[cfg(all(windows, not(debug_assertions)))]
            {
                if let Some(path) = camellia_remote_protocol::config::Config::icon_path().parent() {
                    db = format!("{}\\{}", path.to_str().unwrap_or("."), db);
                }
            }
            #[cfg(not(windows))]
            {
                db = format!("./{db}");
            }
            db
        });
        log::info!("DB_URL={}", db);
        let max_cached_peers = crate::common::get_bounded_usize_arg(
            "MAX_CACHED_PEERS",
            DEFAULT_MAX_CACHED_PEERS,
            1_024,
            1_000_000,
        )?;
        log::info!("MAX_CACHED_PEERS={max_cached_peers}");
        let pm = Self {
            map: Default::default(),
            db: database::Database::new(&db).await?,
            max_cached_peers,
        };
        Ok(pm)
    }

    #[inline]
    pub(crate) async fn get(&self, id: &str) -> ResultType<Option<LockPeer>> {
        let p = self.map.read().await.get(id).cloned();
        if let Some(peer) = p {
            peer.write().await.last_access_time = Instant::now();
            return Ok(Some(peer));
        }

        let stored_peer = self
            .db
            .get_peer(id)
            .await
            .with_context(|| format!("Unable to load peer {id} from the database"))?;
        if let Some(v) = stored_peer {
            let info = serde_json::from_str::<PeerInfo>(&v.info)
                .with_context(|| format!("Peer {id} has malformed persisted metadata"))?;
            let peer = Peer {
                guid: v.guid,
                uuid: v.uuid.into(),
                pk: v.pk.into(),
                // user: v.user,
                info,
                // disabled: v.status == Some(0),
                ..Default::default()
            };
            let peer = Arc::new(RwLock::new(peer));
            return Ok(self.cache_peer(id.to_owned(), peer).await);
        }
        Ok(None)
    }

    #[inline]
    pub(crate) async fn get_or(&self, id: &str) -> ResultType<Option<LockPeer>> {
        if let Some(p) = self.get(id).await? {
            return Ok(Some(p));
        }
        let tmp = LockPeer::default();
        Ok(self.cache_peer(id.to_owned(), tmp).await)
    }

    #[inline]
    pub(crate) async fn get_in_memory(&self, id: &str) -> Option<LockPeer> {
        let peer = self.map.read().await.get(id).cloned();
        if let Some(peer) = &peer {
            peer.write().await.last_access_time = Instant::now();
        }
        peer
    }

    async fn cache_peer(&self, id: String, peer: LockPeer) -> Option<LockPeer> {
        {
            let map = self.map.read().await;
            if let Some(existing) = map.get(&id) {
                return Some(existing.clone());
            }
            if map.len() < self.max_cached_peers {
                drop(map);
                let mut map = self.map.write().await;
                if let Some(existing) = map.get(&id) {
                    return Some(existing.clone());
                }
                if map.len() < self.max_cached_peers {
                    map.insert(id, peer.clone());
                    return Some(peer);
                }
            }
        }

        self.evict_inactive(PEER_CACHE_ACTIVE_GRACE_SECS).await;
        let mut map = self.map.write().await;
        if let Some(existing) = map.get(&id) {
            return Some(existing.clone());
        }
        if map.len() >= self.max_cached_peers {
            log::warn!("Peer cache capacity reached; rejected {}", id);
            return None;
        }
        map.insert(id, peer.clone());
        Some(peer)
    }

    pub(crate) async fn evict_inactive(&self, idle_secs: u64) -> usize {
        let snapshot = self
            .map
            .read()
            .await
            .iter()
            .map(|(id, peer)| (id.clone(), peer.clone()))
            .collect::<Vec<_>>();
        let mut candidates = Vec::new();
        for (id, peer) in &snapshot {
            let peer = peer.read().await;
            if peer.last_access_time.elapsed().as_secs() >= idle_secs
                && peer.last_reg_time.elapsed().as_secs() >= PEER_CACHE_ACTIVE_GRACE_SECS
            {
                candidates.push((id.clone(), peer.last_access_time));
            }
        }
        if candidates.is_empty() {
            return 0;
        }

        let mut map = self.map.write().await;
        let before = map.len();
        for (id, last_access_time) in candidates {
            let should_remove = map.get(&id).is_some_and(|cached| {
                Arc::strong_count(cached) <= 2
                    && cached
                        .try_read()
                        .is_ok_and(|peer| peer.last_access_time == last_access_time)
            });
            if should_remove {
                map.remove(&id);
            }
        }
        before - map.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ip_request_limit_counts_the_first_request_and_enforces_exact_cap() {
        let now = Instant::now();
        let mut blockers = IpBlockMap::new();

        for _ in 0..IP_REQUEST_LIMIT {
            assert!(allow_ip_request_in(
                &mut blockers,
                "192.0.2.1",
                "device01",
                now,
                16
            ));
        }
        assert!(!allow_ip_request_in(
            &mut blockers,
            "192.0.2.1",
            "device01",
            now,
            16
        ));
        assert_eq!(blockers["192.0.2.1"].0 .0, IP_REQUEST_LIMIT);
        assert_eq!(blockers["192.0.2.1"].1 .0.len(), 1);

        assert!(allow_ip_request_in(
            &mut blockers,
            "192.0.2.1",
            "device01",
            now + Duration::from_secs(IP_BLOCK_DUR),
            16
        ));
    }

    #[test]
    fn ip_request_limit_enforces_distinct_id_cap() {
        let started_at = Instant::now();
        let mut blockers = IpBlockMap::new();

        for index in 0..IP_DISTINCT_ID_LIMIT {
            let window = index as u64 / IP_REQUEST_LIMIT as u64;
            assert!(allow_ip_request_in(
                &mut blockers,
                "192.0.2.1",
                &format!("device{index:03}"),
                started_at + Duration::from_secs(window * IP_BLOCK_DUR),
                16
            ));
        }
        let next_window = IP_DISTINCT_ID_LIMIT as u64 / IP_REQUEST_LIMIT as u64;
        assert!(!allow_ip_request_in(
            &mut blockers,
            "192.0.2.1",
            "one-device-too-many",
            started_at + Duration::from_secs(next_window * IP_BLOCK_DUR),
            16
        ));
    }

    #[test]
    fn ip_request_table_fails_closed_then_reclaims_expired_entries() {
        let started_at = Instant::now();
        let mut blockers = IpBlockMap::new();
        assert!(allow_ip_request_in(
            &mut blockers,
            "192.0.2.1",
            "device01",
            started_at,
            2
        ));
        assert!(allow_ip_request_in(
            &mut blockers,
            "192.0.2.2",
            "device02",
            started_at,
            2
        ));
        assert!(!allow_ip_request_in(
            &mut blockers,
            "192.0.2.3",
            "device03",
            started_at,
            2
        ));

        assert!(allow_ip_request_in(
            &mut blockers,
            "192.0.2.3",
            "device03",
            started_at + Duration::from_secs(DAY_SECONDS),
            2
        ));
        assert_eq!(blockers.len(), 1);
    }

    #[test]
    fn ip_change_table_is_bounded_and_resets_expired_entries() {
        let started_at = Instant::now();
        let mut changes = IpChangesMap::new();
        assert!(record_ip_change_in(
            &mut changes,
            "device01",
            "192.0.2.1",
            started_at,
            1
        ));
        assert!(!record_ip_change_in(
            &mut changes,
            "device02",
            "192.0.2.2",
            started_at,
            1
        ));
        assert!(record_ip_change_in(
            &mut changes,
            "device02",
            "192.0.2.2",
            started_at + Duration::from_secs(IP_CHANGE_DUR_X2),
            1
        ));
        assert!(changes.contains_key("device02"));
        assert!(!changes.contains_key("device01"));
    }
}
