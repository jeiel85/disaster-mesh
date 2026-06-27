# Design Bundle Changelog

## v2.0.0-rc1 — 2026-06-27

### Protocol and security

- authenticated `hop_limit` in DME AAD
- terminal receipt policy preventing receipt recursion
- reordered and spoof-resistant cancel handling
- persisted 4096-bit replay bitmap
- exact BLE segment/control/resume contracts
- local encrypted-column envelope with row-bound AAD
- capability and Unicode/bidi validation policy

### Persistence and FFI

- unified persisted state codes and SQL checks
- pending control, replay, transfer-resume and token-grant evidence schema
- GATT/platform `command_id` completion correlation
- process-restart and Keystore/corruption recovery contracts

### Commercial readiness

- production release, privacy, support, incident, field-operation and go-live gates
- security verification and requirements traceability
- dependency review register, public security/support policies
- release evidence JSON schema and machine-readable vector case catalog
- archived superseded monolithic documents and excluded `.git` metadata from distribution artifacts

### Validation

- SQLite schema execution and integrity checks
- TOML/JSON parse checks
- critical contract presence and placeholder scan
- generated file list and SHA-256 manifest

This is a release-candidate **design baseline**, not evidence that an implementation has passed device, security, legal or field validation. Those are explicit stable-release gates.
