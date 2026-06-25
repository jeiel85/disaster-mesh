# DisasterMesh Design Review Handoff

Date: 2026-06-25 (updated session 2)
Status: Design review COMPLETE — go/no-go verdict is GO (zero start-blockers). Paused
before Goal 0 at the user's request; resume with Goal 0 when the user is ready.
Scope so far: Design/CDDL/SQLite/contracts only. No Rust or Android implementation started.

## Current state (read first)

- Design bundle v1.0.1 is consistency-verified and START-READY. A multi-agent
  adversarial go/no-go review (recorded in `docs/16`) found zero true start-blockers.
- Three pre-Goal-0 mechanical fixes were applied and pushed: mesh-store added to all
  crate enumerations (9 crates), `[routing.location_update]` added to the constants
  TOML, and `token_grants.token_count` CHECK widened to `1..16`. See `docs/16`
  items 16-18.
- Git: repo initialized, default branch `master`, pushed to PUBLIC GitHub
  **github.com/jeiel85/disaster_mesh** under Apache-2.0. Working tree clean.
- THE NEXT STEP IS GOAL 0 (repo bootstrap): Rust 9-crate workspace + Android
  multi-module + CI, driven by `prompts/goal-00-bootstrap.md`. No protocol logic yet.
- Remaining verified review findings are non-blocking and tracked per Goal in
  `docs/16` (state-enum reconciliation -> Goal 1; contact-card capabilities -> Goal 2;
  BLE control frame payloads + resume-by-packet_id -> Goal 3/4).

## Next-session entry

Continue from `D:\Project\disaster_mesh`.

Read in this order:

1. `README.md`
2. `docs/16-design-review-v1.0.1.md`
3. this file
4. `prompts/goal-00-bootstrap.md` (the next action)
5. `docs/03-protocol-dme-v1.md`
6. `docs/04-protocol-ble-cla-v1.md`
7. `docs/05-routing-and-queue.md`
8. `docs/07-storage-schema.md`
9. `schemas/sqlite_v1.sql`

Design/schema consistency validation is finished. The next scope is Goal 0
implementation — start it only when the user asks.

## Completed decisions

- Baseline renamed to design bundle v1.0.1.
- BPv7 Payload Block number fixed to 1; outer bundle indefinite array documented.
- ACK-loss token inflation replaced by persistent token-grant escrow.
- Same-grant reconciliation and UNCERTAIN token handling documented.
- Receipt/cancel are endpoint-only; relays do not inspect target IDs or delete originals.
- Owned inbound routing slots separated from contact destination slots.
- Legacy BLE advertising constrained to 31 bytes with UUID-only fallback.
- DME envelope and inner HPKE ciphertext limits separated: 8192/8118 bytes.
- Contact QR fixed as `DM1:<Base45>~<CRC32C>` with domain-separated signature.
- Contact import always starts UNVERIFIED; in-person verification is a separate API.
- Sender sequence made contact-scoped with persisted replay state/window.
- Multi-recipient check-in/SOS creates per-recipient message, packet and ciphertext.
- Verified-local protected storage separated from unverified relay priority.
- Direct-destination traffic remains subject to ingress and partial-storage limits.
- Reboot bundle-age recovery fails closed to AGE_UNCERTAIN on invalid checkpoints.
- GPS/manual location wire values split into an explicit union.
- Cross-device raw elapsedRealtime values removed from receipt/location wire fields.
- Android baseline set to compile SDK 37, target SDK 36, with API 37 testing.
- Android backup/device-transfer exclusion and legacy permission behavior documented.
- User text fields now have Unicode-scalar and UTF-8-byte limits.

## Files added

- `docs/16-design-review-v1.0.1.md`
- `docs/adr/ADR-007-token-grant-escrow.md`
- `docs/adr/ADR-008-endpoint-only-control.md`
- `SESSION_HANDOFF.md`

## Important modified areas

- `README.md`, `CONTENTS.md`, `FILELIST.txt`, `IMPLEMENTATION_CHECKLIST.md`
- `docs/00-product-requirements.md` through `docs/15-references.md`
- ADR-003 through ADR-006
- `spec/dme-v1.cddl`, `spec/contact-card-v1.cddl`, `spec/ble-control-v1.cddl`
- `schemas/sqlite_v1.sql`
- `contracts/protocol_constants.toml`
- `contracts/android_interfaces.kt`
- goal prompts 00 through 04
- `test-vectors/README.md`

`FULL_IMPLEMENTATION_DESIGN.md` is intentionally retained as a superseded v1.0
snapshot with a warning at the top. Separated files are the source of truth.

## Verification already completed

- `schemas/sqlite_v1.sql` executed successfully in Python SQLite in-memory.
- `PRAGMA user_version` returned 1.
- `PRAGMA foreign_key_check` returned an empty list.
- Initial schema currently creates 19 non-internal tables.
- TOML and JSON schema files parsed successfully.
- DME max-size arithmetic checked:
  8118-byte HPKE ciphertext + deterministic CBOR envelope = 8192 bytes.
- `FILELIST.txt` matched the actual tracked design file list, excluding the combined
  superseded snapshot by intent.
- CDDL CLI validation was not run because `cddl`, `cddlcat`, and `zcbor` are not installed.

## Remaining work before declaring design review complete

1. Run one final repository-wide stale-term and contradiction sweep. — DONE 2026-06-25.
2. Re-run SQLite execution after the latest schema edits. — DONE, all PASS.
3. Re-run TOML/JSON parsing and DME size arithmetic. — DONE, all PASS.
4. Manually inspect final file contents because this directory is not a Git repository.
   — DONE via direct reads of all separated docs, CDDL, schema, contracts, prompts.
5. Verify CDDL syntax with a selected validator during Goal 1; do not install a dependency
   merely for this paused design review. — STILL DEFERRED to Goal 1 CI. One occurrence-syntax
   defect was found by manual inspection and fixed (see below); a real validator run is still
   owed in Goal 1.
6. Update `docs/16-design-review-v1.0.1.md` if the final sweep finds additional decisions.
   — DONE: added blocker 15 (CDDL occurrence) and a "Final consistency sweep" section.
7. Report that no commit/push was possible because `D:\Project\disaster_mesh` has no `.git`.
   — CONFIRMED: not a git repo, so no commit/push performed.

## Final sweep result (2026-06-25)

Design review is consistent and ready for Goal 0. Findings:

- All 14 prior resolved blockers are consistently propagated across the separated docs,
  CDDL, `schemas/sqlite_v1.sql`, contracts, ADRs, and goal prompts 00-06.
- The QR `:`→`~` separator change is fully applied; no stale `:` checksum form remains
  outside the superseded snapshot.
- One genuine defect fixed: `spec/ble-control-v1.cddl` used the RFC 8610-ambiguous
  `* 32` / `* 16` occurrence form (space between `*` and the count). Changed to `0*32`
  / `0*16`. Bounds unchanged; only the encoding was disambiguated. 3 lines:
  `routing-slots`, `inventory-page.entries`, `bundle-request`.
- `FULL_IMPLEMENTATION_DESIGN.md` is the only file still carrying old values
  (Payload Block number 4, `outbound_routing_slot`, compile/target SDK 36, `DM1:` colon
  checksum). This is intentional — it has the superseded do-not-implement header and is
  excluded from `FILELIST.txt` as the source-of-truth set.

## Non-blocking item to reconcile during implementation

The verified-in-person flag has three layer-local names: `safety_number_verified`
(domain model), `safety_verified` (SQLite column), `displayed_safety_number` (Rust facade).
Same concept; pick one wire/DB name during Goal 2. Not a design contradiction.
