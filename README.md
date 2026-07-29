# Camellia Remote Server

Camellia Remote Server provides the identity/rendezvous and relay data plane for
Camellia Remote. It is an AGPL-3.0-only derivative of RustDesk Server OSS and
uses the separately versioned `camellia-remote-protocol` submodule.

The repository is a clean pre-release baseline. It does not import old runtime
databases, old package names, or compatibility service units.

## Components

| Binary | Purpose | Default listeners |
|---|---|---|
| `camellia-remote-identity` | device registration, rendezvous, NAT testing and WebSocket rendezvous | 21115/TCP, 21116/TCP+UDP, 21118/TCP |
| `camellia-remote-relay` | encrypted session relay and WebSocket relay | 21117/TCP, 21119/TCP |
| `camellia-remote-utils` | key generation, key validation, diagnostics and TCP health checks | none |

Build and verify the exact locked graph with Rust 1.97.1:

```bash
cargo build --locked --release --bins
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace --all-features
```

## Configuration

Command-line arguments override environment values. A bounded INI-style
`.env` file in the working directory is also supported. Production services
should use a root-owned environment file and secret files instead.

Important settings:

- `CAMELLIA_REMOTE_KEY`: `_` or `-` loads or atomically generates
  `id_ed25519`/`id_ed25519.pub`; an explicit identity key must be a valid
  Ed25519 private key.
- `CAMELLIA_REMOTE_IDENTITY_PORT`: identity base port, default 21116.
- `CAMELLIA_REMOTE_RELAY_PORT`: relay base port, default 21117.
- `CAMELLIA_REMOTE_RELAY_SERVERS`: public relay `host:port` values advertised to clients.
- `CAMELLIA_REMOTE_API_SERVER`: bare management API origin. Non-loopback HTTP is rejected.
- `CAMELLIA_REMOTE_DEVICE_VERIFICATION_TOKEN_FILE`: shared high-entropy verification secret.
- `CAMELLIA_REMOTE_TRUST_PROXY_HEADERS`: keep `N` unless a trusted proxy overwrites client
  address headers.
- `CAMELLIA_REMOTE_DB_URL`: identity SQLite path. The clean default is
  `./camellia-remote.sqlite3`.

See [the complete environment reference](docs/environment-variables.md).

## Production deployment

The Compose model requires an immutable image digest and runs both services as
UID/GID 10001 with no capabilities, a read-only root filesystem, bounded PIDs,
health checks, and one shared persistent state volume.

```bash
export CAMELLIA_REMOTE_SERVER_IMAGE='ghcr.io/camellia-computing/remote-server@sha256:<digest>'
export CAMELLIA_REMOTE_RELAY_ADDRESS='remote.example.com:21117'
export CAMELLIA_REMOTE_API_SERVER='https://api.example.com'
export CAMELLIA_REMOTE_DEVICE_VERIFICATION_TOKEN_FILE='/secure/device-verification-token'
docker compose up -d
```

For a native deployment, install the three release binaries, create the
unprivileged `camellia-remote` account, install
`deploy/systemd/*.service`, and place a mode-0600 configuration at
`/etc/camellia-remote/server.env`. Start the relay before the identity
service:

```bash
sudo systemctl enable --now camellia-remote-relay.service
sudo systemctl enable --now camellia-remote-identity.service
```

TLS should terminate at a hardened reverse proxy where WebSocket ingress is
needed. Expose only the required ports and retain host/firewall rate controls.
Never put the private server key or device-verification token in an image,
repository, log, or command history.

## Availability and recovery

The initial production target is one region with a 99.9% service objective,
RPO no greater than one hour, and RTO no greater than four hours. At least
hourly, take an encrypted, integrity-checked snapshot of the entire persistent
state as one recovery point: SQLite database, Ed25519 key pair, and any
operator-managed configuration. Store the image digest, protocol submodule
commit, application commit, and snapshot checksum beside it.

Quarterly and after storage/release changes, restore into an isolated host,
verify key permissions, run `camellia-remote-utils validatekeypair`, start
both services from the recorded image digest, and exercise registration,
rendezvous, direct/relay connection, and management API verification. A
rollback changes the immutable image digest; it never edits the database or
moves a release tag.

## Release flow

The cross-repository release decision and residual production gates are recorded in the
[Camellia Remote production-readiness audit](https://github.com/camellia-computing/remote-client/blob/main/docs/production-readiness-audit.md).
The exact candidate, publication, recovery, and independent-verification
contract is documented in [the release process](docs/release-process.md).

Pull requests and `main` pushes run formatting, Clippy, all tests, metadata
checks, release-state-machine regression tests, the production image build,
vulnerability scanning, Compose validation, systemd hardening checks, and one
stable `CI / Required` aggregate gate. The manual Release workflow accepts only
a commit reachable from `main`; publication additionally requires the trusted
default-branch workflow, a successful push CI run, validated hosted policy, and
the protected `release` environment.

A published run builds a deterministic native bundle, publishes a multi-
architecture GHCR image by digest with SBOM and provenance, signs the digest
with keyless Cosign, attests release files, and has the Release App create and
read back the canonical immutable `vMAJOR.MINOR.PATCH` GitHub Release. The
Linux archive explicitly uses provenance-only trust and no platform
certificate. Moving tags, overwriting image aliases, mutable `latest` aliases,
and rebuilding published assets are not part of the release contract.

## License and provenance

Camellia changes and the inherited server code are licensed
[AGPL-3.0-only](LICENSE). Upstream copyright and attribution are retained in
[NOTICE](NOTICE), with the exact import recorded in
[SOURCE_PROVENANCE.json](SOURCE_PROVENANCE.json). Network deployment therefore
carries AGPL source-availability obligations. Required CI rejects any mismatch
between that record and the checked-out protocol submodule.
