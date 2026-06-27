# 16. Design Review Resolution v1.0.1

Date: 2026-06-25

## Status

이 문서는 역사적 v1.0.1 review record다. v2.0.0-rc1에서는 당시 “Goal별 후속 처리”로 남겼던 항목을 상용 launch blocker로 승격해 해결했다. 현재 구현 기준은 README의 normative source order를 따른다.

v1.0.1 separated documents, CDDL, contracts, and `schemas/sqlite_v1.sql` are the
implementation baseline. `FULL_IMPLEMENTATION_DESIGN.md` is a superseded v1.0
snapshot and must not be used for implementation.

## Resolved blockers

1. BPv7 Payload Block number fixed to 1; outer bundle indefinite array explicitly allowed.
2. ACK-loss token inflation replaced by persistent token-grant escrow and reconciliation.
3. Receipt/cancel processing limited to final endpoints; relays do not delete target copies.
4. Owned inbound routing slots and contact destination slots separated in the data model.
5. Legacy BLE advertisement fixed to the 31-byte boundary with a service-UUID-only fallback.
6. Encoded DME and inner HPKE ciphertext limits separated as 8192/8118 bytes.
7. Contact QR framing, signature domain separation, and safety-number format fixed.
8. Replay state moved to contact-scoped sender sequences and a persisted 256-entry window.
9. Multi-recipient check-in/SOS defined as one send group with per-recipient ciphertext/bundle.
10. Verified-local protected storage separated from unauthenticated relay priority metadata.
11. Direct-destination traffic kept subject to ingress/session/partial-storage limits.
12. Reboot bundle-age recovery made fail-closed when the wall checkpoint is invalid.
13. GPS and manual locations split into an explicit wire union.
14. Android baseline updated to compile SDK 37, target SDK 36 with API 37 tests.
15. BLE control CDDL occurrence indicators normalized from the whitespace-separated
    `* 32` / `* 16` form to the unambiguous `0*32` / `0*16` form (RFC 8610 §3.2).
    The bounds are unchanged; only the spec-ambiguous encoding was corrected.

## Verification performed

- SQLite schema executed successfully in an in-memory SQLite engine.
- Foreign-key check returned no violations on the empty initial schema.
- `PRAGMA user_version` is 1 and matches the initial schema metadata value.
- TOML and JSON schema files parsed successfully.
- DME CBOR size arithmetic confirms an 8,118-byte HPKE ciphertext produces an
  8,192-byte encoded DME ciphertext envelope at the maximum.
- Repository-wide stale-term searches were used to remove superseded ACK,
  routing-slot, payload-size, and Android baseline wording.

## Final consistency sweep (2026-06-25, re-run)

- SQLite schema re-executed: `user_version=1`, `foreign_key_check` empty,
  `quick_check` ok, 19 non-internal tables, `bundles.creation_sequence` is BLOB.
- TOML and manifest JSON schema re-parsed; protocol size/type constants match the docs.
- DME size arithmetic re-derived from CBOR headers: an 8,118-byte HPKE ciphertext
  yields exactly an 8,192-byte encoded `DmeCiphertext` (1+1+1+34+34+8121).
- All 14 resolved blockers confirmed propagated across docs, CDDL, schema, contracts,
  ADRs, and goal prompts. `FULL_IMPLEMENTATION_DESIGN.md` retains the superseded values
  by intent and carries the do-not-implement warning header.
- One defect found and fixed during the sweep: blocker 15 (CDDL occurrence indicators).
- Open naming variance (non-blocking): the verified-in-person flag is
  `safety_number_verified` in the domain model, `safety_verified` in the SQLite schema,
  and surfaces as `displayed_safety_number` in the Rust facade. These are the same
  concept at different layers; reconcile field names during Goal 2.

## Go/no-go review and pre-Goal-0 fixes (2026-06-25)

A multi-agent adversarial review (10 dimension reviewers, per-finding verification,
architect synthesis) assessed implementation-start readiness. Verdict: **ready to
start, zero true start-blockers**. 27 confirmed-real findings, all either fillable
contract/doc omissions with a canonical value elsewhere in the bundle, cosmetic
naming mismatches, or wire-format details scoped to Goal 3/4.

Three mechanical reconciliations were applied before Goal 0 (no design decisions):

16. `mesh-store` crate added to every enumeration so all lists agree on 9 crates
    (`README.md`, `docs/01` §4 already had it; added to `prompts/goal-00-bootstrap.md`,
    `docs/13` Goal 0 task list, and the `docs/01` dependency graph; README tree order
    aligned to dependency order store-before-engine).
17. `[routing.location_update]` added to `contracts/protocol_constants.toml`
    (priority 1, ttl 86400 s = 24h, hop 12, tokens 8) matching `docs/05` §5; it is a
    distinct policy from CHECK_IN (48h) and must not fall back to it.
18. `token_grants.token_count` CHECK widened from `1..15` to `1..16` so a CDDL-valid
    `proposed-receiver-tokens`/`accepted-tokens` value (ceiling 16) cannot pass the
    wire schema yet abort the relay INSERT.

Remaining verified findings are tracked for their relevant Goal: delivery_state /
open state-code enum reconciliation (Goal 1 state machines), contact-card signed
`capabilities` prose (Goal 2 vectors), BLE control frame payloads
CREDIT_UPDATE/PING/PONG/ERROR/GOODBYE and resume-by-packet_id keying (Goal 3/4).

## Implementation gates

- Accept or replace ADR-007 and ADR-008 before Goal 1.
- Generate real protocol vectors from implementation; do not hand-author crypto output.
- Validate CDDL files with the selected CDDL tool in Goal 1 CI.
- Perform external protocol/security review before stable 1.0.
