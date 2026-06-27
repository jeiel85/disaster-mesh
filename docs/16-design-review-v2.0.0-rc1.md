# 16. Design Review Resolution v2.0.0-rc1

Date: 2026-06-27

## Status

This bundle is the commercial implementation baseline. Goal 0 repository bootstrap may start immediately. Goal 1–4 feature work requires Goal 0.5 normative contract freeze and passing `python tools/validate_design_bundle.py`. Packaged artifacts additionally require the validator's `--distribution` mode.

Historical v1.0/v1.0.1 material is in `archive/` and is non-normative.

## Resolved P0 findings

1. `hop_limit` is authenticated in DME HPKE AAD and has a standalone CDDL contract.
2. DELIVERY_RECEIPT is terminal and cannot generate a receipt; a CANCEL receipt is also terminal.
3. Cancel-before-original is persisted with verified sender and target identifiers.
4. Replay acceptance uses a contact-scoped persisted 4096-bit sliding bitmap.
5. BLE outer segmentation, frame sequence, stream rules, control payloads, credit and resume are byte-exact.
6. Every asynchronous Android command has a `command_id`; one GATT operation is in flight per link.
7. Partial resume identity includes peer hash, exact wire hash, chunk size/count and expiry.
8. Token grant terminal records retain payload/wire hashes, accepted tokens and commit time.
9. Local encrypted columns use a versioned DMEV envelope with row/column-bound AAD.
10. Persisted state codes are centralized in `contracts/state_codes.toml` and checked in SQLite.
11. Android 14+ MTU behavior is modeled as one request per ACL connection, not cascading requests.
12. API 29–30 background BLE/location policy is separated by distribution variant.

## Commercial additions

- privacy/data inventory, deletion and identity-reset behavior
- operational health, recovery, staged rollout and incident severity
- OWASP MASVS mapping and external review exit criteria
- requirements-to-contract-to-test traceability
- signed release evidence and multi-owner go-live checklist
- 8h normal-device and 24h fixed-relay soak requirements

## Mechanical verification

The bundled validator performs:

- TOML and JSON parsing
- initial SQLite schema creation and `PRAGMA quick_check`
- required table/column checks for replay, cancel and resume
- terminal receipt, authenticated hop-limit and replay-window rule presence
- placeholder scan in normative machine contracts
- SHA-256 file manifest generation

A passing validator proves internal bundle mechanics only. It does not replace CDDL validation, implementation tests, security review, device testing or legal/store approval.

## Remaining implementation gates

- select and lock dependencies with license/audit review
- generate cryptographic golden vectors from code, not by hand
- implement and prove every state/transaction invariant
- perform external protocol/crypto and Android security review
- complete device matrix, field exercise, privacy/legal review and go-live signatures
