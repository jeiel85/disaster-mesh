# Bundle Contents — v2.0.0-rc1

## Normative implementation baseline

- `README.md` — source-of-truth order and implementation entry
- `docs/00-product-requirements.md` through `docs/22-go-live-checklist.md`
- `docs/adr/` — accepted architectural decisions
- `spec/` — exact DME/BLE wire and CDDL contracts
- `contracts/` — protocol constants, persisted states, Rust/Kotlin facade contracts
- `schemas/sqlite_v1.sql` — initial production schema
- `schemas/schema_invariants.sql` — integrity assertions
- `test-vectors/` — vector generation/manifest requirements and required case catalog
- `prompts/` — implementation goals, including Goal 0.5 contract freeze and Goal 7 launch
- `docs/dependency-review.md` — lockfile/SBOM 기반 의존성 승인 register
- `SECURITY.md`, `SUPPORT.md` — 공개 보안 신고·지원 경계
- `policies/` — 개인정보 초안과 store disclosure consistency gate
- `release/` — signed release evidence manifest schema
- `tools/validate_design_bundle.py` — mechanical validation and inventory/hash generation;
  pass `--distribution` when validating a packaged artifact that must not contain `.git`

## Non-normative

- `archive/` — superseded design snapshots for history only
- `docs/16-design-review-v2.0.0-rc1.md` — current resolved review and implementation gates

Implementation must not use an archived monolithic design when a separated normative file exists.
