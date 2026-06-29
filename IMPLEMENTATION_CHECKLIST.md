# Implementation Checklist

## Before coding

- [ ] Read README and docs 00–22; archive is non-normative
- [ ] Accept or replace each ADR explicitly
- [ ] Generate final project UUIDs only if intentionally changing specified UUIDs
- [ ] Populate `docs/dependency-review.md` from actual Cargo/Gradle lockfiles and approve every critical dependency
- [x] Generate/check constants and persisted state codes from contracts/*.toml
- [x] Configure offlineRelease manifest assertion
- [x] Configure OS/OEM backup and data-transfer exclusion assertion

## Protocol

- [x] CDDL files are used by tests
- [ ] Deterministic CBOR enforced on encode and validated on decode
- [ ] DM-BP7-1 block order/flags/CRC fixed
- [ ] Packet/body size limits checked before allocation
- [ ] HPKE AAD includes immutable hop_limit and matches spec/dme-aad-v1.cddl
- [ ] Golden vectors committed

## Routing

- [ ] token grant escrow before relay transfer
- [ ] uncertain grant is never reused after ACK loss
- [ ] same-grant reconciliation is idempotent
- [ ] Direct destination bypasses token restriction
- [ ] hop/age never decrease
- [x] receipt terminal/non-recursive; cancel reorder/pending control/idempotency tested
- [ ] verified local P0/P1 protected pool
- [ ] ingress peer quota

## Android

- [ ] INTERNET absent from offlineRelease
- [ ] allowBackup/fullBackupContent/dataExtractionRules exclusions verified
- [ ] BLE callbacks contain no blocking work
- [ ] single coordinator actor owns core calls
- [ ] foreground service user-started and visible
- [ ] permission revoke/BT off paths tested
- [ ] physical-device BLE tests performed; command_id correlation and Android 14+ MTU rule verified

## Security

- [ ] secrets excluded from Debug/log/export
- [ ] master key wrapped in Keystore
- [ ] test deterministic RNG absent in release
- [ ] parser fuzz targets running
- [ ] dependency review and SBOM
- [ ] product limitations shown in UI

## Release

- [x] DB migration tests
- [ ] compatibility matrix
- [ ] battery/screen-off report
- [ ] threat model updated
- [ ] external review findings handled
- [ ] safety wording reviewed

## Commercial release

- [ ] privacy policy/Data Safety/manifest/SBOM are mutually consistent
- [ ] external protocol/crypto review closed
- [ ] MASVS mapping and penetration evidence complete
- [ ] 8h normal + 24h fixed relay soak complete
- [ ] migration, interrupted migration, downgrade, Keystore loss tested
- [ ] staged rollout/rollback/support/incident drill complete
- [ ] `SECURITY.md` private reporting route and `SUPPORT.md` real owner/channel tested
- [ ] privacy draft publisher/contact/effective-date fields completed and legally reviewed
- [ ] signed release evidence manifest validates against `release/release-manifest.schema.json`
- [ ] `docs/22-go-live-checklist.md` signed
