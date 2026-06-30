# Goal 6 Public Beta Gate Record

Status: **BLOCKED**  
Prepared: 2026-06-30

## Automated gates implemented

- [x] Six parser/reassembly libFuzzer targets and Linux nightly smoke workflow
- [x] Release golden-vector verification
- [x] DB corruption detection without silent reset
- [x] Fixed-schema redacted diagnostic ZIP and user-selected export
- [x] cargo deny/audit, Gradle locking/checksum verification
- [x] Deterministic CycloneDX source dependency SBOM
- [x] Unsigned APK/SBOM/native-symbol evidence and provenance workflow
- [x] Offline manifest and test-RNG marker artifact gates
- [x] Play/F-Droid metadata prepared with non-guarantee wording

## Blocking evidence

- [ ] Linux fuzz workflow has a retained successful run for this commit
- [ ] Supported-device compatibility matrix is populated from physical devices
- [ ] Eight-hour normal and 24-hour fixed-relay battery reports pass
- [ ] Controlled field exercises meet the thresholds in docs/11
- [ ] External protocol/security review critical and high findings are closed
- [ ] Penetration/MASVS evidence is reviewed
- [ ] Signed beta artifact, artifact-level SBOM, and final provenance are retained

The source tree is hardening-ready, not approved for public beta distribution.
