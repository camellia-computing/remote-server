# Environment reference

Both long-lived binaries accept command-line flags and canonical environment variables. Command-line values win. Environment and configuration-file names are uppercase, underscore-separated, and must use the `CAMELLIA_REMOTE_` prefix; no historical aliases are accepted.

## Shared settings

| Variable | Default | Contract |
| --- | ---: | --- |
| `CAMELLIA_REMOTE_BIND` | all interfaces | one local IPv4 or IPv6 bind address |
| `CAMELLIA_REMOTE_KEY` | generated/loaded | explicit validated key, or `_` to use persisted key state |
| `CAMELLIA_REMOTE_TRUST_PROXY_HEADERS` | `N` | `Y` requires a non-empty trusted proxy CIDR list |
| `CAMELLIA_REMOTE_TRUSTED_PROXY_CIDRS` | empty | comma-separated final proxy IPv4/IPv6 CIDRs; `/0`, duplicates and more than 64 entries are rejected |
| `CAMELLIA_REMOTE_LOG_FILTER` | `info` | bounded Flexi Logger filter |

## Identity service

| Variable | Default | Contract |
| --- | ---: | --- |
| `CAMELLIA_REMOTE_IDENTITY_PORT` | `21116` | base TCP/UDP port; NAT is −1 and WebSocket is +2 |
| `CAMELLIA_REMOTE_RELAY_SERVERS` | empty | comma-separated public relay hosts |
| `CAMELLIA_REMOTE_API_SERVER` | loopback port 21114 | canonical HTTP(S) management origin |
| `CAMELLIA_REMOTE_DB_URL` | `./camellia-remote.sqlite3` | identity SQLite state path |
| `CAMELLIA_REMOTE_DEVICE_VERIFICATION_TOKEN_FILE` | empty | bounded, non-symlink regular file containing the shared API authentication and deployment-assertion HMAC secret |
| `CAMELLIA_REMOTE_DEVICE_VERIFICATION_TOKEN` | empty | direct secret alternative used for the same assertion contract; never set together with the file variable |
| `CAMELLIA_REMOTE_ALWAYS_USE_RELAY` | `N` | disable direct connections when `Y` |
| `CAMELLIA_REMOTE_ALLOW_UNMANAGED_DEVICES` | `N` | emergency/development bypass; must remain `N` in managed production |
| `CAMELLIA_REMOTE_MAX_DATABASE_CONNECTIONS` | implementation default | bounded SQLite connection pool |
| `CAMELLIA_REMOTE_MAX_RENDEZVOUS_CONNECTIONS` | implementation default | bounded rendezvous connection concurrency |

## Relay service

| Variable | Default | Contract |
| --- | ---: | --- |
| `CAMELLIA_REMOTE_RELAY_PORT` | `21117` | TCP relay port; WebSocket is +2 |
| `CAMELLIA_REMOTE_LIMIT_SPEED` | `4Mb` | per-connection relay limit |
| `CAMELLIA_REMOTE_TOTAL_BANDWIDTH` | `1Gb` | total relay limit |
| `CAMELLIA_REMOTE_SINGLE_BANDWIDTH` | `16Mb` | single-IP relay limit |

The included systemd and Compose definitions use these names. Configuration files containing unprefixed or obsolete keys fail closed.

Proxy address headers apply only to WebSocket listeners. A TCP peer must match
`CAMELLIA_REMOTE_TRUSTED_PROXY_CIDRS` before either header is consulted.
`X-Forwarded-For` is parsed as a bounded chain from right to left, skipping
trusted proxy hops until the nearest untrusted client is found; a chain made
only of trusted proxy addresses is rejected. `X-Real-IP` is accepted as a
single bare IP; when both headers are present their resolved client IP must
agree. Malformed or duplicate headers from a trusted proxy reject the upgrade.
Headers from an untrusted direct peer are ignored.
