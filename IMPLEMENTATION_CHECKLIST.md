# Implementation Checklist

## Before coding

- [ ] Read README and docs 00–10
- [ ] Accept or replace each ADR explicitly
- [ ] Generate final project UUIDs only if intentionally changing specified UUIDs
- [ ] Select and record dependency versions/audit status
- [ ] Create protocol version constants in one crate
- [ ] Configure offlineRelease manifest assertion
- [ ] Configure OS/OEM backup and data-transfer exclusion assertion

## Protocol

- [ ] CDDL files are used by tests
- [ ] Deterministic CBOR enforced on encode and validated on decode
- [ ] DM-BP7-1 block order/flags/CRC fixed
- [ ] Packet/body size limits checked before allocation
- [ ] HPKE AAD and Ed25519 signature inputs match the specification
- [ ] Golden vectors committed

## Routing

- [ ] token grant escrow before relay transfer
- [ ] uncertain grant is never reused after ACK loss
- [ ] same-grant reconciliation is idempotent
- [ ] Direct destination bypasses token restriction
- [ ] hop/age never decrease
- [ ] receipt/cancel idempotent
- [ ] verified local P0/P1 protected pool
- [ ] ingress peer quota

## Android

- [ ] INTERNET absent from offlineRelease
- [ ] allowBackup/fullBackupContent/dataExtractionRules exclusions verified
- [ ] BLE callbacks contain no blocking work
- [ ] single coordinator actor owns core calls
- [ ] foreground service user-started and visible
- [ ] permission revoke/BT off paths tested
- [ ] physical-device BLE tests performed

## Security

- [ ] secrets excluded from Debug/log/export
- [ ] master key wrapped in Keystore
- [ ] test deterministic RNG absent in release
- [ ] parser fuzz targets running
- [ ] dependency review and SBOM
- [ ] product limitations shown in UI

## Release

- [ ] DB migration tests
- [ ] compatibility matrix
- [ ] battery/screen-off report
- [ ] threat model updated
- [ ] external review findings handled
- [ ] safety wording reviewed
