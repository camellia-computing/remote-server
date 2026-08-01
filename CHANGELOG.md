# Changelog

## [1.0.1] - 2026-08-01

### Fixes

- fix(release): harden candidate recovery (#12) (`d24b60c0d28f`)
- fix(release): preserve read-only draft authorization (#13) (`132a0a034df4`)
- fix(release): preserve managed draft visibility (#14) (`d8a8ea49deaa`)
- fix(release): preserve completed recovery lifecycle (#16) (`f0b93a4d46eb`)
- fix(release): scan extracted OCI layout (#17) (`b06564177350`)
- fix(release): verify platform manifests independently (#18) (`56fbeb278f73`)
- fix(release): reconcile incomplete registry aliases (#19) (`6a8095e3cfe6`)
- fix(release): make completion cleanup reentrant (#20) (`10f3afd9c15a`)
- fix(release): authorize merged PR label cleanup (#22) (`5a8f006517f8`)

## [1.0.0] - 2026-07-31

### Features

- feat: establish Camellia Remote Server production baseline (`86d63813e2f9`)
- feat(release): automate verified image publication (#6) (`f40d088f35cb`)

### Fixes

- security: consume redacted protocol revision (`309d510aebd3`)
- security: consume trust-revoking protocol revision (`999e6fc32a3e`)
- security: consume current-format protocol revision (`7311a7f9bba7`)
- fix(release): resolve GitHub App bot identity (#8) (`b750a2ea4f6f`)
- fix(release): preserve empty commit bodies (#9) (`c4999c1a7ce1`)
- fix(release): preserve porcelain status columns (#10) (`6a4fd80c7b2f`)

### Other changes

- chore: pin validated protocol revision (`c93a5149d970`)
- ci: gate runtime images on vulnerability scans (#2) (`906de544fd86`)
- ci: publish verified immutable server releases (#3) (`da8f36b44444`)
- chore: synchronize and enforce the reviewed protocol source (#4) (`dbbc5a04c3f5`)
- refactor(security): adopt maintained protocol cryptography (#5) (`df1e6d83052a`)
