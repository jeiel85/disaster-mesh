# 12. Release, CI/CD and Field Operations

## 1. Branch/release

- `main`: always releasable
- feature branch + PR
- protocol change는 `protocol-change` label과 ADR 필수
- release tag: `android-v0.x.y`, protocol은 별도 `dme-v1`

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

## 10. Incident response

- SECURITY.md에 private report 경로
- 취약점 triage severity
- protocol key compromise 시 key update/revoke 안내
- 위험한 version의 peer 연결 차단 capability
- offline 환경을 고려한 앱 내 security notice package는 향후 signed authority message로 검토
