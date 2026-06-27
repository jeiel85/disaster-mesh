# Session Handoff — Commercial Baseline v2.0.0-rc1

## Current status

The design is upgraded from an implementation-oriented v1.0.1 to a commercial implementation baseline. Goal 0 bootstrap may start, but Goal 1 feature work must pass Goal 0.5 contract freeze first.

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

Execute `/goal 0` and `/goal 0.5`. Do not start Bluetooth product behavior until
`python tools/validate_design_bundle.py` passes and Goal 0.5 acceptance evidence exists.
For a packaged release artifact, also run
`python tools/validate_design_bundle.py --distribution` outside a Git checkout.

## Stable 1.0 boundary

Stable release additionally requires external protocol/crypto review, MASVS mapping, device/soak/migration evidence, privacy/legal review, staged rollout/rollback drill, and a completed `docs/22-go-live-checklist.md`.
