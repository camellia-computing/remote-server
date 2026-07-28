+# Security policy

Report suspected vulnerabilities privately through GitHub Security Advisories
for `camellia-computing/remote-server`. Do not open a public issue containing
credentials, private keys, exploit details, customer addresses, or recordings.

Only the current unreleased `main` baseline is supported. Reports should
include the affected commit, platform, configuration with secrets removed,
reproduction steps, and impact.

Production operators must use an immutable verified image digest, protect the
server private key and device-verification token, keep proxy-header trust off
unless the proxy overwrites those headers, restrict listener exposure, and
back up the complete state as one encrypted recovery point. A malformed key,
unsafe secret file, invalid API origin, or inconsistent recovery point must
fail closed.
