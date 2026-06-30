# MASVS Evidence Map

Status: **PARTIAL — independent assessment not executed**  
Candidate/hash: **not assigned**

This map points to implementation evidence; it is not an OWASP certification or a
penetration-test result. The external reviewer must bind observations to the final binary,
version, commit, device, and test method.

| MASVS area | Current implementation evidence | Remaining evidence |
|---|---|---|
| STORAGE | Keystore wrapping, DMEV envelopes, backup exclusion, read-only corruption check | Final APK backup verification, rooted/device extraction test, deletion exercise |
| CRYPTO | HPKE/X25519/Ed25519/ChaCha20-Poly1305, Noise XX, deterministic vectors | Independent protocol/crypto review and key-lifecycle test |
| AUTH | Signed contact cards, safety numbers, key-change/revoke state | Device UI spoofing and identity-reset assessment |
| NETWORK | No-INTERNET manifest gate, BLE Noise link, endpoint E2EE, malformed-peer isolation | Final APK dynamic socket/BLE adversarial test |
| PLATFORM | Component/permission allowlist, foreground relay, redacted notifications/export | MASTG-style exported-component, IPC, clipboard, screenshot, and permission tests |
| CODE | Rust lint/tests, size bounds, FFI error boundary, six fuzz targets | Retained Linux fuzz campaign, coverage review, final binary static analysis |
| RESILIENCE | Signed-evidence schema and commercial release gate | Production signing, signature verification, tamper/root scope review |
| PRIVACY | Data map, explicit location selection, no telemetry, fixed diagnostic schema | Final privacy/Data Safety diff and legal approval |

Exit requires the criteria in `docs/20-security-verification-plan.md`: critical/high findings
zero, bounded treatment of medium findings, regression evidence, and exact candidate hashes.
