# Release process

Camellia Remote Server has one manual workflow with two deliberately separate
modes. `publish=false` builds the native candidate without release credentials,
registry writes, tags, or a protected-environment approval. `publish=true` is
the only formal publication path.

## Formal publication contract

A formal run must use `.github/workflows/release.yml` from the default branch.
Its source commit must be reachable from that branch and have a successful
`push` run of `.github/workflows/ci.yml`. The configured Release App must
confirm the hosted merge policy and immutable-Release setting before the
protected `release` environment asks a non-initiating Remote team reviewer for
approval.

The workflow then:

1. builds a deterministic Linux x86-64 native archive from the locked graph;
2. builds an OCI index for Linux amd64 and arm64 and pushes it by digest;
3. scans the exact digest and verifies runtime version and OCI source labels;
4. signs the digest with keyless Cosign under the default-branch workflow
   identity;
5. creates `vX.Y.Z` and `sha-<commit>` aliases only when absent, or verifies
   that existing aliases resolve to the same signed digest;
6. creates checksums, public release metadata, release notes, and GitHub OIDC
   attestations;
7. uses the Release App to create or resume an exact draft, read every asset
   back byte-for-byte, publish it, wait for immutable state, and repeat the
   public readback.

The native archive has no platform code-signing certificate. Its explicit
trust mode is `provenance-only`: SHA-256 plus GitHub OIDC attestation. The OCI
digest is the deployment identity and additionally has a keyless Cosign
signature. No private signing certificate belongs in this repository.

## Recovery and conflict handling

Re-running the same version is safe only in a compatible state:

- no aliases and no Release: build and publish normally;
- one signed immutable alias: verify it and create only the missing alias;
- two matching signed aliases: reuse their digest after runtime and scan
  checks;
- compatible App-authored draft: replace draft assets, verify, and publish;
- compatible immutable Release: perform a read-only public byte comparison and
  succeed;
- any mismatched digest, source label, signer, uploader, tag, title, asset,
  checksum, note, platform set, or Release author: fail without replacement.

An image alias or GitHub tag created outside this workflow is not adopted.
Published assets, tags, and image aliases are never moved to repair a release;
increment the version for new bytes.

## Independent verification

Use values from `release-metadata.json`, not a mutable discovery alias:

```bash
sha256sum --check SHA256SUMS
gh attestation verify --repo camellia-computing/remote-server \
  camellia-remote-server-<version>-linux-x86_64.tar.gz
cosign verify \
  --certificate-identity \
  'https://github.com/camellia-computing/remote-server/.github/workflows/release.yml@refs/heads/main' \
  --certificate-oidc-issuer 'https://token.actions.githubusercontent.com' \
  'ghcr.io/camellia-computing/remote-server@sha256:<digest>'
docker buildx imagetools inspect \
  'ghcr.io/camellia-computing/remote-server@sha256:<digest>'
```

Store the release URL, source commit, CI run, digest, approval record, checksum
result, attestation result, and Cosign result in the release evidence record.
