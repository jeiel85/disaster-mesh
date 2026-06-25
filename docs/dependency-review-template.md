# Dependency Review Template

| Field | Value |
|---|---|
| Crate/library | |
| Version/commit | |
| Purpose | |
| License | |
| Maintainer/repository | |
| Last release/activity | |
| Transitive dependency count | |
| Unsafe code | |
| Formal/external audit | |
| Known advisories | |
| Alternatives reviewed | |
| Pinning strategy | |
| Update owner | |
| Decision | APPROVE / REJECT / CONDITIONAL |
| Notes | |

## Security-sensitive dependencies

각각 별도 검토:

- HPKE implementation
- Ed25519/X25519 implementation
- Noise implementation
- CBOR parser
- BPv7 parser
- SQLite binding
- UniFFI
- QR/Base45 parser

“Rust로 작성됨” 또는 “유명 crate”라는 이유만으로 승인하지 않는다.
