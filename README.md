# Disaster Mesh — Implementation Design Bundle v1.0.1

> 목적: 이동통신망·인터넷·공유기 없이, 주변 스마트폰과 고정 릴레이만으로 종단간 암호화된 재난 메시지를 저장·운반·전달하는 Android 우선 오픈소스 시스템을 구현한다.
>
> 문서 상태: **구현 기준선(Implementation Baseline)**  
> 기준일: 2026-06-25  
> 코드명: `DisasterMesh`  
> 예시 Application ID: `org.disastermesh.android`

v1.0.1은 최초 교차검토에서 확인된 BPv7 block numbering, token ACK 유실,
수신 routing slot 저장, endpoint-only receipt/cancel, BLE legacy advertising 크기
문제를 수정한 기준선이다.

## 이 묶음이 이전 설계서와 다른 점

이전 문서는 제품 방향과 아키텍처의 타당성을 설명하는 수준이었다. 이 묶음은 개발자가 별도 해석 없이 첫 커밋부터 기능 구현을 시작하도록 다음 항목을 고정한다.

- 모듈 경계와 책임
- 프로토콜 필드, 타입, 길이, 검증 규칙
- BLE 광고·GATT 서비스·프레임 레이아웃·타임아웃
- 라우팅, 복제 토큰, 큐 우선순위, 삭제 규칙
- 키 생성·QR 연락처·메시지 암호화 절차
- SQLite 테이블·인덱스·트랜잭션 경계
- Rust Core와 Kotlin 플랫폼 계층의 API 계약
- 서비스·세션·메시지·전송 상태 머신
- 단계별 `/goal`과 각 단계의 완료 조건
- 단위·통합·실기기·보안 테스트 케이스

## 고정된 1.0 기술 기준

| 항목 | 결정 |
|---|---|
| 1차 플랫폼 | Android 8.0/API 26 이상 |
| 빌드 기준 | compile SDK 37, target SDK 36, JDK 17 |
| UI | Kotlin + Jetpack Compose |
| 플랫폼 통신 | Android BLE Central + Peripheral(GATT Server) |
| 공유 코어 | Rust 2024 Edition |
| FFI | UniFFI 한 개의 통합 facade crate |
| 번들 | BPv7의 제한된 프로파일 + DME v1 payload |
| 앱 직렬화 | Core Deterministic CBOR |
| 스키마 | CDDL |
| 메시지 암호화 | RFC 9180 HPKE Base: X25519/HKDF-SHA256/ChaCha20Poly1305 |
| 송신자 인증 | Ed25519 서명, 암호문 내부 포함 |
| 링크 보안 | `Noise_XX_25519_ChaChaPoly_BLAKE2s` |
| 라우팅 | Direct Delivery + Binary Spray-and-Wait + persistent token grant escrow |
| 저장 | SQLite; 키는 Android Keystore로 감싼 DB master key 사용 |
| 네트워크 권한 | BLE-only release에는 `INTERNET` 미선언 |
| 첨부 | 1.0 미지원; encoded DME payload 8 KiB 이하 |

> 암호 라이브러리와 BPv7 라이브러리는 외부 감사 여부를 확인하고 잠금 파일·SBOM에 고정한다. 라이브러리를 사용한다고 시스템 전체가 자동으로 감사 완료되는 것은 아니다.

## 문서 읽는 순서

1. `docs/00-product-requirements.md`
2. `docs/01-system-architecture.md`
3. `docs/02-domain-model.md`
4. `docs/03-protocol-dme-v1.md`
5. `docs/04-protocol-ble-cla-v1.md`
6. `docs/05-routing-and-queue.md`
7. `docs/06-security-and-threat-model.md`
8. `docs/07-storage-schema.md`
9. `docs/08-rust-core-contract.md`
10. `docs/09-android-implementation.md`
11. `docs/10-state-machines.md`
12. `docs/11-testing-and-acceptance.md`
13. `docs/12-release-and-operations.md`
14. `docs/13-development-goals.md`
15. `docs/14-known-limitations.md`
16. `docs/15-references.md`
17. `docs/16-design-review-v1.0.1.md`

## 구현 시작 명령

첫 번째 구현 단계는 Bluetooth가 아니다. 아래 순서로 시작한다.

```text
Goal 0: repository/bootstrap
Goal 1: deterministic protocol core + simulator
Goal 2: identity/contact/E2EE test vectors
Goal 3: Android direct BLE transfer
Goal 4: multi-hop relay
Goal 5: disaster UX and persistent relay
Goal 6: hardening and beta
```

첫 커밋에서 생성할 최상위 구조:

```text
/
├─ Cargo.toml
├─ rust-toolchain.toml
├─ gradle/
├─ gradlew
├─ settings.gradle.kts
├─ apps/android/
├─ core/
│  ├─ mesh-types/
│  ├─ mesh-codec/
│  ├─ mesh-crypto/
│  ├─ mesh-bundle/
│  ├─ mesh-routing/
│  ├─ mesh-engine/
│  ├─ mesh-store/
│  ├─ mesh-sim/
│  └─ mesh-ffi/
├─ spec/
├─ test-vectors/
└─ docs/
```

## Definition of Ready

기능 구현 티켓은 다음 조건을 충족해야 시작할 수 있다.

- 관련 요구사항 ID가 있다.
- 입력·출력·오류가 정의되어 있다.
- 저장소 변경과 마이그레이션 여부가 정해져 있다.
- 로그에 남겨도 되는 값과 금지 값이 정해져 있다.
- 단위 테스트와 실기기 완료 조건이 있다.
- 배터리·권한·백그라운드 영향이 검토되었다.

## Definition of Done

- happy path와 실패 경로 테스트가 통과한다.
- 프로토콜 변경이면 CDDL, 테스트 벡터, 버전 정책이 함께 갱신된다.
- DB 변경이면 migration 및 downgrade 거부 테스트가 있다.
- 메시지 본문·정확한 위치·개인키가 로그에 남지 않는다.
- Android lint, Rust fmt/clippy/test, cargo deny/audit가 통과한다.
- 실기기 BLE 검증이 필요한 기능은 에뮬레이터 테스트만으로 완료 처리하지 않는다.

## 중요한 제품 문구

> 이 앱은 재난 상황에서 주변 기기를 이용해 메시지 전달 가능성을 높이는 보조 수단입니다. 주변 중계 경로가 없거나 기기가 꺼져 있으면 메시지가 전달되지 않을 수 있으며, 구조 요청의 접수와 대응을 보장하지 않습니다.

## 라이선스

이 프로젝트는 [Apache License 2.0](LICENSE) 하에 배포되는 오픈소스다.

Copyright 2026 The DisasterMesh Authors

설계는 아직 외부 crypto/protocol 리뷰 전이다(`docs/14-known-limitations.md`,
`docs/06-security-and-threat-model.md`의 출시 게이트 참고). 안정 1.0 이전에는
프로토콜·보안 속성이 변경될 수 있다.
