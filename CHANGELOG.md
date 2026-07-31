# Changelog

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
