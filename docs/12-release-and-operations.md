# 12. Release, CI/CD and Field Operations

## 1. Branch/release

- `main`: protected, release-candidate quality; required checks와 2-person review 없이는 merge 금지
- feature branch + PR
- protocol change는 `protocol-change` label과 ADR 필수
- release tag: `android-v0.x.y`, protocol은 별도 `dme-v1`; signed annotated tag와 immutable release manifest 사용

## 2. CI jobs

### Rust

```text
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace
cargo test protocol_vectors
cargo deny check
cargo audit
cargo llvm-cov
fuzz smoke
```

### Android

```text
assembleOfflineRelease
lintOfflineRelease
testOfflineReleaseUnitTest
connectedDevDebugAndroidTest
manifest permission assertion
native symbol packaging assertion
```

### Supply chain

- dependency lockfiles commit
- Gradle dependency verification
- cargo vendor 또는 release source snapshot 검토
- CycloneDX/SPDX SBOM
- secret scanning
- signed provenance/attestation

## 3. Manifest permission gate

CI가 offline release manifest에 다음이 없음을 검증한다.

```text
android.permission.INTERNET
ACCESS_NETWORK_STATE
Firebase/analytics providers
advertising ID permission
```

필수 BLE/FGS/location/notification/boot-recovery 권한만 allowlist한다.
또한 `allowBackup=false`, `fullBackupContent=false`, data-extraction root exclusion을
assert한다.

## 4. Release artifacts

- signed AAB/APK
- F-Droid-compatible source tarball
- SBOM
- mapping/native debug symbols (비공개 또는 controlled)
- protocol test vectors
- threat model
- battery/compatibility report
- known limitations
- reproducible build instructions

## 5. Version compatibility

앱 정보 화면:

```text
App version
Protocol major/minor
DME version
BLE-CLA version
DB schema
Rust core commit
```

protocol major가 다른 peer와는 연결하지 않는다.

## 6. Crash policy

인터넷 없는 product이므로 자동 crash upload 없음.

- local crash marker
- 다음 실행에서 설명
- 사용자 선택 diagnostic ZIP export
- export 전 포함 항목 미리보기
- 메시지 본문/위치/키 제외

## 7. Fixed relay operating guide

공기계 운영 조건:

- 충전기와 보조배터리
- 화면 잠금
- 고정 릴레이 모드
- Bluetooth ON
- 앱 persistent notification 확인
- 하루 1회 저장량/열/서비스 상태 확인
- 자동 OS update/reboot 일정 관리

운영자가 메시지 내용을 열람하는 기능은 제공하지 않는다.

## 8. Disaster exercise checklist

훈련 전:

- 앱/키/연락처 사전 배포
- 연락처 안전번호 확인
- 기기 충전
- 고정 릴레이 위치 계획
- 메시지 전달 비보장 교육

훈련 중:

- 실시간 외부망을 끈 시험군 분리
- 기기 이동/contact schedule 기록
- P0/P1/P2 샘플 발송
- battery/thermal/relay interruption 기록

훈련 후:

- diagnostic export 수집은 자발적 동의
- message body 수집 금지
- delivery/latency/bytes/battery 분석
- 정책값 변경은 ADR로 기록

## 9. Stable 1.0 전 필수 검토

- 암호/프로토콜 외부 리뷰
- Android BLE/background 전문 리뷰
- 개인정보 영향 검토
- 재난 대응 UX 전문가 검토
- 제한된 현장 훈련
- 안전 문구 법률 검토

## 10. Commercial distribution gates

- Play Console/F-Droid metadata와 실제 manifest·Data Safety·privacy policy를 release마다 대조
- latest target SDK requirement는 release candidate 날짜 기준 공식 문서에서 재검증
- 앱 서명 key는 offline/HSM-backed 보관, upload key와 분리, recovery procedure 연 1회 점검
- internal → closed beta → staged production 순서; 5%/20%/50%/100% 단계별 최소 관찰 window와 rollback owner 지정
- protocol-major를 올린 release는 mixed-version field test 없이 production 확대 금지
- 심각한 offline-only defect는 서버 kill switch가 없으므로 store halt, signed advisory, in-app local notice bundle 전략을 사전에 준비
- 지원 범위, 데이터 복구 불가 조건, 기기/OEM 제한을 store listing과 앱 안에서 동일하게 표시

## 11. Incident response

- SECURITY.md에 private report 경로
- 취약점 triage severity
- protocol key compromise 시 key update/revoke 안내
- 위험한 version의 peer 연결 차단 capability
- offline 환경을 고려한 앱 내 security notice package는 향후 signed authority message로 검토

## 12. Release evidence retention

각 release에 다음을 최소 5년 또는 프로젝트 운영 기간 중 더 긴 기간 보존한다.

- source commit, signed tag, build environment lock, artifact SHA-256
- AAB/APK, symbols, SBOM, provenance, dependency review
- protocol golden vectors와 compatibility report
- migration/soak/device-matrix/security/field-exercise 결과
- 승인자, 승인 시각, known-risk waiver, rollback 판단

민감한 test data는 합성 데이터만 사용하고 실제 사용자 메시지·위치는 release evidence에 포함하지 않는다.
