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
- [x] Deterministic CBOR enforced on encode and validated on decode
- [x] DM-BP7-1 block order/flags/CRC fixed
- [x] Packet/body size limits checked before allocation
- [x] HPKE AAD includes immutable hop_limit and matches spec/dme-aad-v1.cddl
- [x] Golden vectors committed

## Routing

- [x] token grant escrow before relay transfer
- [x] uncertain grant is never reused after ACK loss
- [x] same-grant reconciliation is idempotent
- [x] Direct destination bypasses token restriction
- [x] hop/age never decrease
- [x] receipt terminal/non-recursive; cancel reorder/pending control/idempotency tested
- [x] verified local P0/P1 protected pool
- [x] ingress peer quota

## Android

- [x] INTERNET absent from offlineRelease
- [ ] allowBackup/fullBackupContent/dataExtractionRules exclusions verified
- [x] BLE callbacks contain no blocking work
- [ ] single coordinator actor owns core calls
- [x] foreground service user-started and visible
- [ ] permission revoke/BT off paths tested
- [ ] physical-device BLE tests performed; command_id correlation and Android 14+ MTU rule verified

## Security

- [x] secrets excluded from Debug/log/export
- [x] master key wrapped in Keystore
- [x] test deterministic RNG absent in release
- [ ] parser fuzz targets running
- [ ] dependency review and SBOM
- [x] product limitations shown in UI

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
