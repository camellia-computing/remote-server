# Dynamic boundary testing

The normal Required suite runs the deterministic malformed-rendezvous corpus
and the existing database concurrency tests as ordinary Rust unit tests. Inputs,
iterations, and thread counts are fixed so this layer stays fast and repeatable.

The scheduled `Dynamic Boundaries` workflow uses
`nightly-2026-08-15` with `-Zbuild-std` on Linux x86_64. It builds first,
then gives each selected test a separate ten-minute execution budget:

- ASan checks production rendezvous parsing with malformed bounded messages;
- TSan checks concurrent database insertion and reads.

Sanitizer ABI mismatch bypasses are forbidden. A sanitizer report fails the
job, and no logs or test inputs are uploaded. These jobs do not establish
Windows/macOS sanitizer support or exercise an external production database;
those remain platform and system-test responsibilities.
