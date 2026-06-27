# Design Bundle Validation Report

Baseline: `v2.0.0-rc1`  
Validation date: `2026-06-27`

## Automated checks completed

- all required commercial, protocol, contract, schema and policy files present
- distribution mode (`tools/validate_design_bundle.py --distribution`) verifies that
  packaged artifacts contain no `.git` directory; repository validation omits this packaging-only gate
- TOML contracts parse successfully
- JSON documents parse; JSON schemas pass Draft 2020-12 schema checks
- required vector case catalog validates against its schema and has unique IDs
- SQLite v1 schema creates successfully with foreign keys enabled
- SQLite `quick_check` returns `ok`; initial foreign-key and custom invariant queries return zero violations
- critical schema columns/tables for replay, reordered cancel, transfer resume and token-grant evidence exist
- DME AAD contains immutable hop limit
- terminal receipt, 4096-bit replay and platform command-correlation rules are present
- normative machine files contain no TODO/TBD/FIXME markers
- referenced bundle files exist
- deterministic file inventory and SHA-256 manifest are generated

## Validation limitation

A standalone CDDL executable was not installed in the validation environment. The bundle validator therefore performed lexical delimiter/rule checks only. **Goal 0.5 must install/pin a real CDDL validator or compile the CDDL through the selected implementation tooling and make that check mandatory in CI.** This release candidate must not be described as independently CDDL-validated yet.

## Evidence not claimable from a design bundle

The following remain mandatory stable-release evidence and cannot be fabricated before implementation:

- generated cryptographic/BPv7/BLE golden and invalid vectors
- independent protocol and cryptography review
- Android/OEM BLE, screen-off, reboot and foreground-service device matrix
- 8-hour normal-device and 24-hour fixed-relay soak reports
- migration, corruption, Keystore-loss and power-interruption results
- penetration/MASVS verification evidence
- signed AAB/APK, SBOM, provenance and release manifest
- actual publisher, privacy, support, security contact and legal approvals
- controlled field-exercise results

The design is therefore a **commercial implementation baseline**, not a certification of an unbuilt product.
