# Session Handoff — Commercial Baseline v2.0.0-rc1

## Current status

The design is upgraded from an implementation-oriented v1.0.1 to a commercial implementation baseline. Goal 0 bootstrap and the Goal 0.5 normative contract freeze completed on 2026-06-29. Goal 1 protocol-core work is unblocked.

Goal 0 evidence includes Rust format/clippy/tests, Android release lint and unit
tests, all three bootstrap variants, a four-ABI UniFFI package, merged-manifest
policy assertions, and an API 36 emulator instrumentation call to Rust `version()`.

## Closed launch-blocking design gaps

- immutable hop_limit is authenticated in DME AAD
- receipt-of-receipt is prohibited; cancel receipt is terminal
- cancel-before-original is persisted and verified
- replay uses a 4096-bit persisted sliding bitmap
- BLE outer/encrypted framing and all control payloads are exact
- async GATT commands use command_id correlation and one in-flight operation/link
- resume identity includes peer + exact wire hash + chunk layout
- transfer/grant commit evidence is persisted
- local encryption envelope and key-loss behavior are exact
- persisted numeric states are centralized and checked by SQL

## Commercial governance added

- lockfile/SBOM-based dependency review register
- public security reporting and support boundary documents
- privacy/store disclosure release inputs without invented publisher data
- signed release evidence JSON schema
- machine-readable required test-vector case catalog

## Next command

Execute `/goal 1`. Goal 0.5 acceptance evidence includes generated TOML
constants, CDDL conformance, SQLite migrations/invariants, BLE golden/invalid
headers, replay properties, terminal control tests, Android command correlation,
and source/distribution design-bundle validation.

## Stable 1.0 boundary

Stable release additionally requires external protocol/crypto review, MASVS mapping, device/soak/migration evidence, privacy/legal review, staged rollout/rollback drill, and a completed `docs/22-go-live-checklist.md`.
