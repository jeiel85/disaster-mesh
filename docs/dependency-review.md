# Dependency Review Register

이 문서는 `Cargo.lock`, Gradle version catalog와 release SBOM에 실제로 포함된 의존성을 승인하는 상용 릴리스 기록이다. 설계 단계에서 최신 버전을 추정해 적지 않는다. **Goal 0에서 lockfile이 생성된 뒤 정확한 버전·checksum·license·upstream 상태를 채우고, Goal 0.5에서 승인되지 않으면 기능 구현을 시작하지 않는다.**

## 1. 승인 원칙

- primary upstream repository와 공식 package registry를 확인한다.
- exact version과 artifact checksum은 lockfile/SBOM에서 가져온다.
- direct dependency뿐 아니라 native/crypto/network transitive dependency도 검토한다.
- 암호·직렬화·SQLite·FFI·BLE 경계 의존성은 `critical`로 분류한다.
- 유지보수 중단, yanked release, known critical vulnerability, license 불일치가 있으면 release를 차단한다.
- “감사된 crate를 사용했다”는 사실을 제품 전체 보안 감사로 표현하지 않는다.
- 의존성 변경 PR은 protocol golden vector, migration, ABI compatibility에 미치는 영향을 기록한다.

## 2. 필수 기록 필드

| Field | Required | Meaning |
|---|---:|---|
| ecosystem/name | Yes | Cargo/Gradle package identity |
| exact_version | Yes | lockfile의 exact version |
| checksum/source_commit | Yes | registry checksum 또는 vendored source commit |
| direct_or_transitive | Yes | 직접/전이 의존성 |
| criticality | Yes | critical/high/normal/dev-only |
| purpose | Yes | 제품에서 사용하는 기능 |
| license_expression | Yes | SPDX expression 및 NOTICE 의무 |
| upstream_url | Yes | primary source |
| maintenance_evidence | Yes | 최근 release/issue/maintainer 상태 확인일 |
| security_evidence | Yes | advisory DB, audit report 또는 검토 근거 |
| unsafe/native_surface | Yes | unsafe code, JNI/NDK, system library 경계 |
| deterministic_build | Yes | lock/vendor/reproducibility 영향 |
| data_access | Yes | key/plaintext/location/DB 접근 여부 |
| decision | Yes | approved / conditional / rejected |
| reviewer/date | Yes | 승인자와 ISO 날짜 |
| expiry | Yes | 다음 재검토일 또는 release마다 재검토 |

## 3. Goal 0 후보 범주

아래 행은 **선정 완료가 아니라 구현자가 비교·승인해야 하는 범주**다. exact package와 version은 lockfile 생성 후 추가한다.

| Category | Required capability | Commercial acceptance evidence |
|---|---|---|
| deterministic CBOR | canonical/deterministic encoder, duplicate-key rejection, bounded decoder | cross-implementation vectors, malformed corpus |
| BPv7 profile | RFC 9171 primary block/canonical block support or a narrowly audited local encoder | profile conformance vectors, unsupported block rejection |
| HPKE | RFC 9180 Base mode, X25519/HKDF-SHA256/ChaCha20Poly1305 | RFC vectors plus DME AAD/signature vectors |
| Ed25519/X25519 | strict signature verification, constant-time primitives where applicable | invalid-key/signature vectors, upstream review |
| Noise | exact `Noise_XX_25519_ChaChaPoly_BLAKE2s` suite | transcript/interoperability/rekey tests |
| SQLite | bundled/system decision documented, WAL, backup and corruption semantics | migration, power-loss and corruption tests |
| UniFFI | Kotlin bindings and Android ABI support | generated API review, exception and byte ownership tests |
| secret handling | zeroization/redaction wrappers | memory/logging review and test-only feature exclusion |
| property/fuzz | deterministic replayable failures | CI seed retention, corpus and sanitizer jobs |

## 4. Review table

이 표는 bootstrap 이후 실제 값으로 채운다. 빈 행이 남아 있으면 `Goal 0.5`는 실패다.

| Ecosystem/name | Version | Checksum/commit | Purpose | Criticality | License | Security/maintenance evidence | Decision | Reviewer/date |
|---|---|---|---|---|---|---|---|---|
| _Generated from lockfile_ |  |  |  |  |  |  |  |  |

## 5. Automated gates

- `cargo deny check`와 `cargo audit` 결과를 release evidence에 보존한다.
- Gradle dependency verification metadata를 commit하고 checksum 변경은 별도 review를 요구한다.
- SBOM package 목록과 이 register의 production direct/critical dependencies가 일치해야 한다.
- forbidden dependency allowlist는 package, reason, owner, expiry를 포함해야 한다.
- debug/test dependency가 `offlineRelease` artifact 또는 native library에 포함되지 않았음을 검사한다.

## 6. 변경 승인

다음 변경은 2인 review와 보안 reviewer 승인이 필요하다.

- crypto, CBOR, BPv7, Noise, SQLite, FFI package 교체 또는 major update
- native binary/AAR 추가
- INTERNET permission 또는 원격 SDK가 필요한 dependency 추가
- key/plaintext/location을 읽는 dependency 추가
- license가 copyleft 또는 source-disclosure 의무를 도입하는 변경
