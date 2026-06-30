# 20. Security Verification Plan

## 1. Verification layers

| Layer | Owner | Evidence |
|---|---|---|
| design/threat model | security architect | reviewed ADRs, abuse cases |
| protocol/crypto | external reviewer | report + closed findings |
| Rust parsers/state | core team | property/fuzz/coverage |
| Android platform | mobile security tester | MASTG-style test record |
| supply chain | release engineer | SBOM, signature, provenance |
| field behavior | QA/operations | device/soak/exercise report |

## 2. OWASP MASVS mapping

구현 근거와 미실행 외부 시험은 `reports/masvs-evidence-map.md`에서 분리해 추적한다.
해당 표는 독립 평가나 인증을 대신하지 않는다.

- STORAGE: Keystore wrapping, DMEV column envelope, backup exclusion, deletion/reset.
- CRYPTO: vetted primitives, nonce uniqueness, deterministic vectors, key separation/rotation.
- AUTH: contact QR signature, safety number, key-change/revoke state.
- NETWORK: Noise link, HPKE endpoint payload, no INTERNET permission, malformed peer isolation.
- PLATFORM: exported components, permissions, FGS, notification redaction, IPC/file provider review.
- CODE: memory/size bounds, no panic across FFI, fuzzing, static analysis.
- RESILIENCE: release signing, tamper awareness where practical; no claim against rooted/compromised OS.
- PRIVACY: data inventory, consented location, no telemetry, diagnostic minimization.

## 3. Mandatory attack cases

- modify every AAD field including hop limit
- forged sender/recipient keys, recipient hash substitution
- receipt-of-receipt attempt and cancel receipt loop
- cancel before original, conflicting cancel, target collision
- replay at max, inside 4096 window, outside window, DB rollback
- token grant ACK loss, same-grant reconciliation, duplicate-other-grant
- segment conflict, frame-ID collision, sequence wrap, credit overflow
- resume with wrong peer/hash/chunk layout
- zip traversal, logcat leakage, clipboard/screenshot/notification exposure review
- rooted device scope documented, not falsely claimed secure

## 4. External review exit criteria

- critical/high 0 open.
- medium findings have fix or signed time-bound risk acceptance; none may enable plaintext/key loss, auth bypass, token inflation, destructive migration.
- every fixed finding has automated regression test or documented reason it cannot.
- report scope/version/hash matches production candidate.

## 5. Vulnerability handling

- `SECURITY.md` provides private report route and supported versions.
- acknowledge/triage/disclosure targets are published but described as targets, not guarantees.
- no production secrets or real user payload are requested from reporters.
- CVE/GHSA handling is used when appropriate; protocol incompatibility and user action are clearly stated.
