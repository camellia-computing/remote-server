# Release process

Remote Server uses one review-driven state machine. Maintainers do not select a
version, source ref, image name, or registry from a dispatch form.

## State machine

1. A successful `CI` run on `main` starts `Release Manager`.
2. The Release App calculates the next stable SemVer from conventional commits,
   updates only the reviewed version/changelog files, and opens or refreshes
   `release/next`.
3. The exact PR head must pass `CI / Required` and receive a current-head human
   approval from a repository writer or administrator. Active change requests
   block promotion.
4. The App squash-merges the exact head, verifies the resulting `main` push CI,
   creates an App-authored draft Release, and creates the lightweight
   `vMAJOR.MINOR.PATCH` tag at that exact commit.
5. The tag starts `publish-release.yml`. A manual dispatch is recovery-only and
   accepts an existing managed tag while executing trusted control code from
   `main`.
6. The workflow freezes one Linux amd64/arm64 OCI layout, scans those exact
   bytes, creates SPDX SBOM and provenance evidence, and records the pinned
   Remote Protocol commit. Formal image construction does not import or export
   a mutable GitHub Actions build cache. Trivy scans the extracted OCI layout;
   the original archive remains unchanged as the frozen publication input.
7. Only after the candidate is frozen does the protected `release` environment
   authorize publication. Every configured registry receives the identical
   digest, immutable version/full-commit aliases, and a keyless Cosign
   signature.
8. All release evidence is checksummed, keylessly signed, uploaded by the App,
   downloaded again, and verified before the Release becomes immutable.
   Completion and `latest` are then reconciled to the highest completed stable
   version.

The first formal release is `v1.0.0`. A failed run is re-entrant: it may resume
only the same App-authored draft or verify an already immutable publication.
Conflicting refs, bytes, authors, approvals, digests, aliases, or evidence fail
closed. Published tags, assets, and registry aliases are never moved.

An incomplete publication must retain `release:pending`. After the exact
immutable `release-complete:<SHA>` marker is validated, recovery accepts the
authorizing PR both immediately before and after label cleanup while still
revalidating its identity, reviewed head, approval, merge topology and tag.

## Registry contract

`CONTAINER_REGISTRY_MAP` maps logical repository IDs to reviewed GHCR and
Docker Hub names. Each configured target is published; an empty target is
recorded as `not-configured` and skipped. At least one registry is required for
this image-only service release. Docker Hub credentials are required only when
its target is configured. Repository and organization renames therefore change
hosted mappings, not workflow source.

Deploy `image@sha256:<digest>` from `release-evidence.json`. Do not deploy a
floating alias. `latest` is discovery convenience only.

## Signing and verification

Server images do not use desktop/mobile certificates. Trust consists of:

- the immutable OCI digest and exact amd64/arm64 platform digests;
- BuildKit/Syft SBOM and GitHub provenance;
- keyless Cosign signatures bound to `publish-release.yml`;
- signed checksums and byte-for-byte GitHub Release readback; and
- the exact `CI / Required`, Release PR, reviewer, and environment records.

Private application TLS certificates remain an operations concern and do not
alter artifact identity. Rollback selects a previously completed digest; it
does not rewrite a tag or database state.

GitHub exposes the complete repository merge-policy fields and draft Releases
only to a caller with push access. Hosted policy and managed-draft lookups
therefore use one repository-scoped App token with short-lived Contents write
permission; the trusted authorization command performs only reads. Exact CI
run lookups use the separate job token constrained to Contents, Actions, and
pull-request read. Repository metadata scripts receive neither token, and no
token receives Actions or Workflows write permission.
