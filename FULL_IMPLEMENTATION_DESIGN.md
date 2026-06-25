# Disaster Mesh — Full Implementation Design v1.0 (superseded snapshot)

> This combined file is a superseded v1.0 snapshot. Do not implement from it.
> The separated v1.0.1 files listed in `CONTENTS.md` are the implementation source of truth.



---

<!-- SOURCE: README.md -->

# Disaster Mesh — Implementation Design Bundle v1.0

> 목적: 이동통신망·인터넷·공유기 없이, 주변 스마트폰과 고정 릴레이만으로 종단간 암호화된 재난 메시지를 저장·운반·전달하는 Android 우선 오픈소스 시스템을 구현한다.
>
> 문서 상태: **구현 기준선(Implementation Baseline)**  
> 기준일: 2026-06-25  
> 코드명: `DisasterMesh`  
> 예시 Application ID: `org.disastermesh.android`

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
| 빌드 기준 | compile/target SDK 36, JDK 17 |
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
| 라우팅 | Direct Delivery + Binary Spray-and-Wait |
| 저장 | SQLite; 키는 Android Keystore로 감싼 DB master key 사용 |
| 네트워크 권한 | BLE-only release에는 `INTERNET` 미선언 |
| 첨부 | 1.0 미지원; 암호화 payload 8 KiB 이하 |

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


---

<!-- SOURCE: IMPLEMENTATION_CHECKLIST.md -->

# Implementation Checklist

## Before coding

- [ ] Read README and docs 00–10
- [ ] Accept or replace each ADR explicitly
- [ ] Generate final project UUIDs only if intentionally changing specified UUIDs
- [ ] Select and record dependency versions/audit status
- [ ] Create protocol version constants in one crate
- [ ] Configure offlineRelease manifest assertion

## Protocol

- [ ] CDDL files are used by tests
- [ ] Deterministic CBOR enforced on encode and validated on decode
- [ ] DM-BP7-1 block order/flags/CRC fixed
- [ ] Packet/body size limits checked before allocation
- [ ] HPKE AAD and Ed25519 signature inputs match the specification
- [ ] Golden vectors committed

## Routing

- [ ] ACK-before-token-change invariant
- [ ] Direct destination bypasses token restriction
- [ ] hop/age never decrease
- [ ] receipt/cancel idempotent
- [ ] P0/P1 storage reservation
- [ ] ingress peer quota

## Android

- [ ] INTERNET absent from offlineRelease
- [ ] BLE callbacks contain no blocking work
- [ ] single coordinator actor owns core calls
- [ ] foreground service user-started and visible
- [ ] permission revoke/BT off paths tested
- [ ] physical-device BLE tests performed

## Security

- [ ] secrets excluded from Debug/log/export
- [ ] master key wrapped in Keystore
- [ ] test deterministic RNG absent in release
- [ ] parser fuzz targets running
- [ ] dependency review and SBOM
- [ ] product limitations shown in UI

## Release

- [ ] DB migration tests
- [ ] compatibility matrix
- [ ] battery/screen-off report
- [ ] threat model updated
- [ ] external review findings handled
- [ ] safety wording reviewed


---

<!-- SOURCE: docs/00-product-requirements.md -->

# 00. Product Requirements

## 1. 제품 목표

통신 인프라가 끊긴 재난 상황에서 사용자가 사전에 등록한 신뢰 연락처에게 짧은 메시지, 생존 상태, 비공개 구조 요청을 전송한다. 수신자가 현재 근처에 없어도 다른 참여 기기가 암호문을 제한적으로 복제·보관·운반하여 이후 접촉 시 전달한다.

## 2. 운영 가정

- 이동통신 데이터와 인터넷이 없을 수 있다.
- Wi-Fi AP가 없을 수 있다.
- GPS 위성 수신은 가능할 수 있으나 위치 확정 시간이 길 수 있다.
- Bluetooth는 사용자가 켜야 한다.
- 모든 노드가 항상 켜져 있거나 협조적이라고 가정하지 않는다.
- 노드 간 접촉은 짧고 불규칙하다.
- 정확한 벽시계가 없을 수 있다.
- 사용자는 앱을 재난 이전에 설치하고 연락처 키를 교환했을 가능성이 높다.

## 3. 핵심 사용자

### U1. 일반 사용자

- 가족에게 생존 여부를 보낸다.
- 짧은 개인 메시지를 보낸다.
- 부상·고립 정보와 위치를 신뢰 연락처에게 보낸다.
- 자신의 기기가 다른 사람의 암호문을 중계하도록 허용한다.

### U2. 고정 릴레이 운영자

- 충전 중인 공기계에 고정 릴레이 모드를 켠다.
- 저장량, 최근 접촉, 서비스 정상 여부만 확인한다.
- 중계 메시지 내용은 볼 수 없다.

### U3. 검증 기관 운영자(1.1 이후)

- 사전 배포한 기관 키로 공지를 서명한다.
- 일반 사용자 메시지보다 높은 정책 우선순위를 요청할 수 있으나, 앱은 로컬 정책으로 제한한다.

## 4. 기능 요구사항

| ID | 요구사항 | 1.0 |
|---|---|---:|
| FR-001 | 서버 없이 설치별 신원 키를 생성한다. | 필수 |
| FR-002 | QR로 연락처 공개키와 라우팅 슬롯을 교환한다. | 필수 |
| FR-003 | 인터넷 없이 BLE로 주변 노드를 발견한다. | 필수 |
| FR-004 | 두 Android 기기 간 직접 E2EE 메시지를 전달한다. | 필수 |
| FR-005 | A-B 접촉 후 B-C 접촉으로 다중 홉 전달한다. | 필수 |
| FR-006 | 중계 노드는 암호문만 저장한다. | 필수 |
| FR-007 | DIRECT_TEXT, CHECK_IN, PRIVATE_SOS를 지원한다. | 필수 |
| FR-008 | 위치 첨부는 명시적 동의 시에만 포함한다. | 필수 |
| FR-009 | TTL, hop limit, copy token을 강제한다. | 필수 |
| FR-010 | 최종 수신 영수증을 반대 방향 DTN으로 보낸다. | 필수 |
| FR-011 | 취소 메시지를 별도 서명 메시지로 전파한다. | 필수 |
| FR-012 | 앱 재시작 후 미전달 큐를 복구한다. | 필수 |
| FR-013 | 대기·긴급·고정 릴레이 모드를 제공한다. | 필수 |
| FR-014 | 진단 정보를 본문 없이 내보낸다. | 필수 |
| FR-015 | 공개 익명 채팅을 제공하지 않는다. | 필수 |
| FR-016 | 첨부파일·음성·영상 전송을 제공하지 않는다. | 필수 |
| FR-017 | iOS 직접 통신 및 best-effort relay를 지원한다. | 1.1 |
| FR-018 | 서명된 기관 공지를 지원한다. | 1.1 |
| FR-019 | Linux/Raspberry Pi 릴레이를 지원한다. | 1.2 |

## 5. 비기능 요구사항

| ID | 요구사항 | 기준 |
|---|---|---|
| NFR-001 | 오프라인 독립성 | core release에 계정·서버·INTERNET 권한 없음 |
| NFR-002 | 기밀성 | 릴레이 DB·BLE 패킷 캡처에서 본문 복호화 불가 |
| NFR-003 | 무결성 | 1비트 변조도 검증 실패 |
| NFR-004 | 복구성 | 프로세스 강제 종료 후 committed bundle 100% 복구 |
| NFR-005 | 제한된 자원 | 기본 relay quota 32 MiB, payload 최대 8 KiB |
| NFR-006 | 결정성 | 동일 상태·이벤트 순서에 동일 CoreAction 출력 |
| NFR-007 | 호환성 | protocol major 불일치 시 안전 종료, minor는 capability 협상 |
| NFR-008 | 관측성 | 본문 없는 로컬 지표와 export 제공 |
| NFR-009 | 배터리 | 모드별 duty cycle과 열·저전력 감속 정책 적용 |
| NFR-010 | 접근성 | TalkBack, 48dp 터치, 색상 외 상태 표현 |
| NFR-011 | 공급망 | SBOM, dependency lock, cargo deny/audit |
| NFR-012 | 안전 문구 | 전달·구조 보장으로 오인할 표현 금지 |

## 6. 1.0 메시지 종류

### DIRECT_TEXT

- UTF-8 1~2,000자
- 우선순위 P2
- TTL 72시간
- copy token 6

### CHECK_IN

상태 enum:

- `SAFE`
- `INJURED_STABLE`
- `NEED_ASSISTANCE`
- `EVACUATING`
- `UNKNOWN`

추가 필드:

- 인원수 1~99
- 짧은 메모 0~500자
- 위치 선택
- 배터리 잔량 선택

기본 우선순위 P1, TTL 48시간, copy token 8.

### PRIVATE_SOS

분류 enum:

- `MEDICAL`
- `TRAPPED`
- `FIRE_SMOKE`
- `MISSING_PERSON`
- `WATER_FOOD`
- `OTHER`

필드:

- 설명 1~800자
- 인원수 1~99
- 중상자 수 0~99
- 위치 선택
- 마지막 이동 방향 0~100자
- 배터리 잔량 선택

기본 우선순위 P0, TTL 24시간, copy token 12.

## 7. 사용자 흐름

### 최초 실행

1. 안전 문구 확인
2. Bluetooth 권한 설명 및 요청
3. 설치 신원 키 생성
4. 표시 이름 입력(로컬 UI용, 네트워크상 신뢰 근거 아님)
5. 대기 모드 기본 OFF
6. 연락처 추가 안내

### 연락처 추가

1. 사용자 A가 QR 표시
2. B가 QR 스캔
3. B가 안전번호 12자리 표시
4. 양쪽 화면의 안전번호를 대면 비교
5. B가 연락처 저장
6. 반대 방향도 별도 수행하거나 양방향 카드 교환 절차 사용

### 메시지 보내기

1. 연락처 선택
2. 메시지 작성
3. 앱이 로컬 bundle 생성·암호화·서명
4. 상태 `STORED_LOCAL`
5. 주변 세션에서 copy token 정책에 따라 복제
6. 최종 수신 영수증 회수 시 `RECEIPT_CONFIRMED`

## 8. 명시적 비목표

- 전화망·위성통신 대체
- 119/112 공식 신고 접수 보장
- 실시간 채팅 경험
- 익명 대규모 방송
- 서버 기반 연락처 검색
- 사용자의 실제 법적 신원 검증
- 악성 무선 방해 해결
- 경로가 전혀 없을 때 전달

## 9. 1.0 출시 승인 조건

- 3대 Android 기기로 A→B, 이후 B→C 전달을 50회 연속 재현한다.
- 중계 B에서 평문과 연락처 표시명을 얻을 수 없다.
- 메시지 변조, 재생, 초과 크기, 만료, hop 초과를 거부한다.
- 화면 꺼짐 8시간 시험에서 relay 상태와 중단 원인을 기록한다.
- 최소 API 26, 30, 31, 34, 36 실기기 호환 결과를 문서화한다.
- 외부 보안 리뷰 전에는 stable 1.0으로 홍보하지 않는다.


---

<!-- SOURCE: docs/01-system-architecture.md -->

# 01. System Architecture

## 1. 실행 구조

```text
┌──────────────── Android App Process ────────────────┐
│ Compose UI / ViewModels                             │
│        │ intents / state                            │
│ Domain Use Cases                                    │
│        │                                             │
│ MeshCoordinator (single coroutine actor)            │
│   ├─ RustCoreFacade (protocol SQLite owner)         │
│   ├─ BleTransportManager                            │
│   ├─ KeyVault                                       │
│   ├─ LocationProvider                               │
│   └─ Diagnostics                                    │
│        │                                             │
│ EmergencyRelayService (foreground when active)      │
└──────────────────────────────────────────────────────┘

Rust core:
Types → Codec → Crypto → Bundle → Routing → Engine → FFI
```

## 2. 책임 분리

### UI/ViewModel

- 사용자 입력 검증
- 권한 요청 화면
- 상태 표현
- 메시지 본문은 UI와 Rust core의 encrypted local message store에서만 취급
- BLE 콜백이나 라우팅 규칙을 포함하지 않음

### MeshCoordinator

- 앱의 단일 오케스트레이터
- 모든 외부 이벤트를 `Channel<MeshPlatformEvent>`에 넣어 순차 처리
- Rust core action을 플랫폼 작업으로 변환
- 동일 peer의 중복 연결 방지
- 서비스 모드와 배터리 정책 적용

### Rust Core

- 네트워크에 독립적인 결정 로직
- CBOR/BPv7 encode/decode
- 암호화·서명·검증
- 라우팅·복제 토큰·TTL·dedup
- protocol SQLite transaction과 migration 수행
- 플랫폼 API 직접 호출 금지

### BLE Transport

- 광고·스캔·GATT 연결
- 링크 framing, MTU, chunk write/notify
- Noise handshake byte transport
- bundle 의미를 해석하지 않음

### Rust Protocol Storage

- Rust core가 bundle, payload, contact, receipt, tombstone SQLite를 직접 소유한다.
- copy-token 변경과 transfer commit을 단일 트랜잭션으로 처리한다.
- 평문 메시지는 column-level 암호화하고 relay ciphertext와 논리적으로 분리한다.
- Android는 DB master key unwrap과 UI preference만 담당한다.

### KeyVault

- Ed25519/X25519 private material의 암호화 보관
- Android Keystore key로 local master key wrap/unwrap
- 개인키 export 미지원

## 3. Android 모듈

```text
apps/android/
├─ app
├─ core-bridge
├─ domain
├─ security-keystore
├─ transport-ble
├─ service-relay
├─ feature-onboarding
├─ feature-home
├─ feature-contacts
├─ feature-conversation
├─ feature-checkin
├─ feature-sos
├─ feature-relay-status
├─ feature-diagnostics
└─ test-fixtures
```

의존 방향:

```text
feature-* → domain ← security/transport adapters
app → feature-* + service-relay
core-bridge → generated UniFFI bindings
transport-ble → domain transport interfaces
```

feature 모듈끼리 직접 참조하지 않는다.

## 4. Rust workspace

```text
core/
├─ mesh-types       # ID, enums, validated value objects
├─ mesh-codec       # deterministic CBOR/CDDL helpers
├─ mesh-crypto      # HPKE, Ed25519, Noise adapter interface
├─ mesh-bundle      # BPv7 constrained profile + DME
├─ mesh-routing     # offer/accept/copy/eviction decisions
├─ mesh-store       # SQLite schema, migration and transactions
├─ mesh-engine      # event → actions deterministic actor
├─ mesh-sim         # encounter graph simulator
└─ mesh-ffi         # one UniFFI facade, no business logic
```

의존 방향:

```text
mesh-types
  ↑
mesh-codec  mesh-crypto
  ↑          ↑
mesh-bundle
  ↑
mesh-routing
  ↑
mesh-engine
  ↑
mesh-ffi / mesh-sim
```

## 5. 프로세스와 동시성

### 단일 writer 원칙

- `MeshCoordinator`만 mesh DB의 네트워크 상태를 변경한다.
- BLE callback은 byte와 peer handle을 event queue에 넣고 즉시 반환한다.
- UI 송신 요청도 동일 queue를 통과한다.
- protocol DB read/write는 Rust engine API로만 수행한다. Android UI는 snapshot DTO를 Flow로 변환한다.

### 주요 coroutine scope

- `applicationScope`: DB·identity lifecycle
- `relayServiceScope`: scan/advertise/session lifecycle
- `sessionScope(peerSessionId)`: 해당 BLE 연결 종료 시 cancel
- `viewModelScope`: 화면 상태만

### 금지

- GATT callback 내부 DB write
- 여러 세션이 동일 bundle copy token을 동시에 감소
- FFI callback에서 blocking I/O
- GlobalScope

## 6. 이벤트 처리

```text
PlatformEvent
├─ AppStarted
├─ RelayModeChanged
├─ BluetoothStateChanged
├─ PeerDiscovered
├─ LinkConnected
├─ LinkBytesReceived
├─ LinkWritable
├─ LinkClosed
├─ LocalMessageRequested
├─ LocationResolved
├─ TimerTick
├─ BatteryChanged
└─ StorageRecovered

CoreAction
├─ PersistTransaction
├─ OpenLink
├─ CloseLink
├─ SendLinkFrame
├─ StartScan
├─ StopScan
├─ StartAdvertise
├─ StopAdvertise
├─ NotifyMessageState
├─ NotifyIncomingMessage
├─ ScheduleTimer
└─ EmitDiagnostic
```

## 7. 데이터 흐름: 송신

1. UI가 `SendDirectText` use case 호출
2. domain validation
3. KeyVault에서 recipient public key와 sender key handle 획득
4. Rust core가 DME plaintext 생성·서명·HPKE seal
5. BPv7 bundle 생성
6. Rust engine이 `bundles + payload + message`를 한 SQLite transaction으로 저장
7. UI 상태 `STORED_LOCAL`
8. 다음 peer session에서 offer
9. transfer commit 후 copy token transaction 갱신
10. receipt 수신 시 delivered state 갱신

## 8. 데이터 흐름: 수신/중계

1. peer가 bundle meta offer
2. local dedup/tombstone/quota/recipient 여부 평가
3. 요청한 bundle의 chunk 수신
4. 임시 파일/row에 write
5. payload hash와 BPv7 validation
6. 최종 수신자이면 HPKE open + signature verify
7. 원자적 commit
8. 수신자면 receipt bundle 생성
9. relay면 ciphertext 상태로 queue에 추가

## 9. 장애 복구

- `RECEIVING` transfer는 마지막 activity 후 10분이면 삭제
- `COMMITTED` bundle은 앱 재시작 시 항상 queue 재구성
- `SENDING`은 세션 종료 시 `AVAILABLE`로 환원
- copy token은 receiver commit ACK를 받은 transaction에서만 나눈다.
- ACK를 잃으면 sender와 receiver 양쪽에 copy가 존재할 수 있다. 이는 허용하며 bundle ID dedup으로 해결한다.
- DB corruption 감지 시 원본 파일을 보존하고 읽기 전용 복구/export 화면으로 진입한다.

## 10. 확장 지점

`TransportAdapter` 인터페이스를 유지해 다음을 추가한다.

- Wi-Fi Aware
- local Wi-Fi socket
- Linux BLE relay
- LoRa gateway

상위 DME, routing, storage는 transport 종류를 알지 않는다.


---

<!-- SOURCE: docs/02-domain-model.md -->

# 02. Domain Model and Invariants

## 1. 기본 ID

| 타입 | 형식 | 생성 |
|---|---|---|
| `IdentityId` | SHA-256(Ed25519 pub), 32 bytes | 설치 시 |
| `DisplayId` | identity hash 앞 10 bytes Base32 + checksum | 표시용 |
| `ContactId` | UUIDv7, 16 bytes | 연락처 저장 시 |
| `ConversationId` | random 16 bytes | 최초 대화 시 |
| `MessageId` | UUIDv7 또는 CSPRNG 16 bytes | 메시지 생성 시 |
| `BundleId` | SHA-256(canonical primary fields || payload), 32 bytes | 암호화 후 |
| `RoutingSlot` | CSPRNG 16 bytes | 연락처 카드 발급 시 |
| `PeerSessionId` | CSPRNG 16 bytes | 링크 연결 시 |
| `TransferId` | CSPRNG 16 bytes | bundle 전송 시 |

UUIDv7 구현 의존성을 피하고 싶으면 16-byte CSPRNG ID로 통일해도 된다. 프로토콜 wire에서 시간 정렬을 요구하지 않는다.

## 2. 값 객체

### Priority

```text
P0 = 0  # SOS, receipt, cancel
P1 = 1  # check-in, location
P2 = 2  # direct text
P3 = 3  # reserved
```

숫자가 작을수록 높은 우선순위다.

### BundleLifetime

- 최소 60초
- 최대 7일
- message type 정책보다 길게 지정 불가

### CopyTokens

- 1~16
- 0은 저장 가능한 상태가 아님
- 직접 최종 전달은 token 값과 무관하게 허용
- relay 복제는 token >= 2일 때만 허용

### HopCount

- 수신 commit 시 1 증가
- 생성 bundle은 0
- `hop_count >= hop_limit`이면 최종 수신자 외에는 요청하지 않음

## 3. Aggregate

### Identity

```text
Identity {
  identity_id
  signing_public_key
  signing_private_key_handle
  hpke_public_key
  hpke_private_key_handle
  local_display_name
  created_at_local
  key_version
}
```

불변식:

- 개인키 raw bytes는 UI/DB 일반 row에 노출하지 않는다.
- identity key rotation은 기존 identity와 서명된 연결 문서 없이 자동 수행하지 않는다.

### Contact

```text
Contact {
  contact_id
  display_name
  identity_signing_public_key
  hpke_public_key
  identity_fingerprint
  outbound_routing_slot
  trust_state
  safety_number_verified
  created_at
  revoked_at?
}
```

`trust_state`:

- `UNVERIFIED`
- `VERIFIED_IN_PERSON`
- `KEY_CHANGED`
- `REVOKED`

불변식:

- `KEY_CHANGED` 연락처로 P0 송신 전 경고한다.
- 동일 identity fingerprint의 중복 연락처는 merge 안내한다.

### Bundle

```text
Bundle {
  bundle_id
  source_route
  destination_slot
  protocol_version
  message_class_hint
  priority
  lifetime_seconds
  age_millis
  hop_count
  hop_limit
  copy_tokens
  payload_hash
  payload_size
  state
  custody_flags
}
```

`message_class_hint`는 `DIRECT`, `CHECK_IN`, `SOS`, `RECEIPT`, `CANCEL` 정도만 노출한다. 완전한 메시지 type과 본문은 암호문 내부에 있다. P0 우선순위를 위해 최소 분류 정보가 노출되는 메타데이터 trade-off를 수용한다.

### LocalMessage

```text
LocalMessage {
  message_id
  conversation_id
  direction
  message_type
  contact_id
  plaintext_encrypted_at_rest
  bundle_id
  delivery_state
  created_at_local
  received_at_local?
}
```

`delivery_state`:

- `DRAFT`
- `STORED_LOCAL`
- `RELAYED_AT_LEAST_ONCE`
- `RECEIVED_BY_DESTINATION_LOCAL_KNOWLEDGE`
- `RECEIPT_CONFIRMED`
- `CANCEL_PROPAGATING`
- `EXPIRED`
- `FAILED_LOCAL`

일반적으로 발신자는 `RECEIVED_BY_DESTINATION_LOCAL_KNOWLEDGE`를 직접 알 수 없으므로 receipt가 도착하기 전 UI에는 사용하지 않는다. 수신자 로컬에서만 원본 상태 추적에 활용한다.

## 4. 상태 불변식

1. 같은 `BundleId`는 payload가 동일해야 한다. 다르면 충돌/공격으로 격리한다.
2. `COMMITTED` bundle만 라우팅 offer 대상이다.
3. 만료 bundle은 최종 수신자에게도 전달하지 않는다.
4. receipt가 검증되면 원본 bundle을 더 이상 offer하지 않는다.
5. cancel이 검증되면 대상 메시지를 UI에서 취소 표시하고 relay 원본을 tombstone 처리한다.
6. relay 노드는 DME plaintext를 저장하지 않는다.
7. 최종 수신자 판정은 routing slot match 후에도 HPKE open과 recipient key hash 검증까지 완료해야 한다.
8. copy token 변경과 transfer commit 기록은 하나의 DB transaction이다.
9. priority는 relay가 상향 조정할 수 없다.
10. lifetime, hop limit, source route, destination slot은 immutable header로 취급한다.

## 5. 오류 taxonomy

```text
ProtocolError
- UnsupportedMajorVersion
- MalformedCbor
- NonDeterministicEncoding
- InvalidBpv7Bundle
- InvalidFieldLength
- PayloadTooLarge
- Expired
- HopLimitExceeded
- Duplicate
- Tombstoned
- HashMismatch
- SignatureInvalid
- HpkeOpenFailed
- RecipientMismatch
- QuotaExceeded
- RateLimited
- UnsupportedCapability

TransportError
- BluetoothUnavailable
- PermissionDenied
- AdvertiseFailed
- ScanFailed
- GattConnectTimeout
- MtuNegotiationFailed
- NoiseHandshakeFailed
- FrameSequenceError
- WriteTimeout
- PeerClosed

StorageError
- DatabaseLocked
- DatabaseCorrupt
- TransactionFailed
- KeyUnavailable
- DiskFull
```

외부 peer에는 상세 crypto 오류를 구분해 보내지 않는다. `INVALID_BUNDLE` 같은 일반 오류만 보내 oracle을 줄인다.


---

<!-- SOURCE: docs/03-protocol-dme-v1.md -->

# 03. DME v1 and BPv7 Profile

## 1. 목적

DME(Disaster Message Envelope) v1은 재난 메시지의 애플리케이션 의미, 발신자 인증, 수신자 암호화를 정의한다. 외부 운반 단위는 BPv7 제한 프로파일을 사용한다.

## 2. 표준 기준

- BP wire format: RFC 9171
- BPv7 최신 등록 변경: RFC 9713, RFC 9758 확인
- deterministic CBOR: RFC 8949 core deterministic encoding
- CDDL: RFC 8610
- HPKE: RFC 9180

1.0은 BPSec 완전 구현을 주장하지 않는다. 애플리케이션 payload의 E2EE가 보안 경계다.

## 3. BPv7 제한 프로파일 `DM-BP7-1`

### 3.1 Primary Block

| 필드 | 값/규칙 |
|---|---|
| version | `7` |
| bundle flags | must-not-fragment=1, all status-report flags=0, fragment=0, admin-record=0 |
| CRC type | CRC32C |
| destination | `dtn://dm/r/{routingSlotBase32}` |
| source | `dtn://dm/s/{randomSourceBase32}`; 메시지별 random 16 bytes |
| report-to | `dtn:none` |
| creation timestamp time | 신뢰 시각이 없으면 `0` |
| creation sequence | CSPRNG u64; 같은 source EID에서 중복 금지 |
| lifetime | milliseconds, message policy 범위 내 |
| fragment fields | 없음 |

메시지별 source EID를 사용해 안정적인 발신자 EID 노출을 줄인다. 실제 발신자 identity는 암호문 내부에 있다.

### 3.2 Canonical Blocks

정해진 순서:

1. Bundle Age Block: type 7, block number 1, CRC32C
2. Hop Count Block: type 10, block number 2, CRC32C
3. Disaster Routing Block: private type 192, block number 3, CRC32C
4. Payload Block: type 1, block number 4, CRC32C

모든 노드는 type 192를 이해해야 한다. 해당 block의 processing flag는 `delete bundle if block cannot be processed`를 설정한다.

### 3.3 Disaster Routing Block v1

릴레이가 읽는 최소 메타데이터다.

```text
[
  version: 1,
  packet_id: bstr .size 16,
  message_class_hint: uint .le 5,
  priority: uint .le 3,
  copy_tokens: uint .ge 1 .le 16,
  payload_size: uint .le 8192,
  payload_sha256: bstr .size 32
]
```

- `packet_id`는 애플리케이션 dedup 키다.
- `copy_tokens`만 복제 과정에서 변경 가능하다.
- priority와 class hint는 메타데이터로 노출된다.
- relay가 priority를 변경해도 최종 수신자는 암호문 내부 서명값과 비교해 조작 여부를 기록한다.

## 4. DME Ciphertext payload

Payload block data는 아래 deterministic CBOR 구조다.

```text
DmeCiphertext = [
  version,
  suite_id,
  encapsulated_key,
  aad_hash,
  ciphertext
]
```

v1 고정값:

- `version = 1`
- `suite_id = 1`
- KEM: DHKEM(X25519, HKDF-SHA256)
- KDF: HKDF-SHA256
- AEAD: ChaCha20Poly1305

`encapsulated_key` 길이는 선택한 HPKE suite의 결과 길이를 따른다. v1 X25519에서는 32 bytes로 검증한다.

## 5. AAD

HPKE `info`:

```text
UTF8("DisasterMesh/DME/1") || packet_id
```

AEAD AAD는 아래 deterministic CBOR bytes다.

```text
[
  protocol_major=1,
  packet_id,
  destination_slot,
  message_class_hint,
  priority,
  lifetime_millis,
  random_source_id,
  creation_sequence
]
```

다음은 AAD에 넣지 않는다.

- age
- hop count
- copy tokens
- CRC

이 값들은 릴레이에서 합법적으로 변한다.

`aad_hash`는 SHA-256(AAD bytes)다. 수신자는 외부 header로 AAD를 재구성하고 hash 일치 후 HPKE open을 수행한다.

## 6. DME Plaintext

암호화 전 deterministic CBOR:

```text
DmePlaintext = [
  version,
  message_type,
  packet_id,
  message_id,
  conversation_id,
  sender_signing_public_key,
  sender_hpke_public_key,
  recipient_identity_hash,
  sender_sequence,
  reply_routing_slot,
  created_time_optional,
  body,
  signature
]
```

### 필드 규칙

| 필드 | 타입 | 규칙 |
|---|---|---|
| version | uint | 1 |
| message_type | uint | 1..7 |
| packet_id | 16-byte bstr | routing block과 같아야 함 |
| message_id | 16-byte bstr | sender가 생성 |
| conversation_id | 16-byte bstr | contact별 대화 |
| sender signing pub | 32-byte bstr | Ed25519 |
| sender HPKE pub | 32-byte bstr | X25519 |
| recipient hash | 32-byte bstr | SHA-256(recipient signing pub) |
| sender_sequence | uint64 | identity별 단조 증가; DB transaction |
| reply slot | 16-byte bstr | receipt/cancel reply 목적지 |
| created time | null or uint64 | Unix ms; UI 참고용, TTL 근거 아님 |
| body | message-type union | 아래 정의 |
| signature | 64-byte bstr | Ed25519 |

## 7. Signature input

서명 대상은 `signature` 필드를 제외한 DME plaintext의 deterministic CBOR bytes와 AAD hash다.

```text
signature_input =
  UTF8("DisasterMesh/DME-SIGN/1") ||
  SHA256(dme_plaintext_without_signature_cbor) ||
  aad_hash
```

검증 순서:

1. 외부 BP/CRC 검증
2. AAD 재구성 및 `aad_hash` 일치
3. HPKE open
4. CDDL 및 canonical CBOR 검증
5. packet ID, recipient hash, message class/priority 일치
6. Ed25519 signature 검증
7. contact trust state 확인
8. replay/sequence 정책 확인

## 8. Message type codes

| code | type |
|---:|---|
| 1 | DIRECT_TEXT |
| 2 | CHECK_IN |
| 3 | PRIVATE_SOS |
| 4 | LOCATION_UPDATE |
| 5 | DELIVERY_RECEIPT |
| 6 | CANCEL |
| 7 | KEY_UPDATE reserved |

### DIRECT_TEXT body

```text
[ text: tstr .size (1..2000), reply_to: null / bstr .size 16 ]
```

### CHECK_IN body

```text
[
  status: 1..5,
  people_count: 1..99,
  note: tstr .size (0..500),
  location: null / Location,
  battery_percent: null / 0..100
]
```

### PRIVATE_SOS body

```text
[
  category: 1..6,
  description: tstr .size (1..800),
  people_count: 1..99,
  severe_injury_count: 0..99,
  location: null / Location,
  movement_direction: tstr .size (0..100),
  battery_percent: null / 0..100
]
```

### Location

```text
[
  latitude_e7: int,
  longitude_e7: int,
  accuracy_meters: uint .le 50000,
  altitude_meters: null / int,
  captured_elapsed_ms: uint,
  manual_description: tstr .size (0..200)
]
```

- 위도 범위 `-900000000..900000000`
- 경도 범위 `-1800000000..1800000000`
- 정확도 50 km 초과 위치는 좌표 대신 manual description만 허용

### DELIVERY_RECEIPT body

```text
[
  original_packet_id: bstr .size 16,
  original_message_id: bstr .size 16,
  receipt_status: 1,  # delivered
  received_elapsed_ms: uint,
  receiver_note: null / tstr .size (0..100)
]
```

### CANCEL body

```text
[
  target_packet_id: bstr .size 16,
  target_message_id: bstr .size 16,
  cancel_reason: 1..4
]
```

취소는 이미 수신자가 읽은 정보를 원격 삭제한다고 보장하지 않는다. 이후 중계와 UI 강조를 중단하는 의미다.

## 9. Sender sequence/replay

- sender identity별 `sender_sequence`는 message 생성 transaction에서 1 증가한다.
- 수신자는 contact별 최대 sequence와 최근 256개 message ID window를 저장한다.
- 낮은 sequence라도 처음 보는 packet은 DTN 재정렬 때문에 즉시 거부하지 않는다.
- 동일 message ID/packet ID는 중복 거부한다.
- 최대 sequence보다 4096 이상 뒤처진 새 message는 `STALE_REPLAY_SUSPECTED`로 격리한다.

## 10. 생성 절차

```text
validate user input
load contact and identity
allocate sender_sequence in transaction
create packet_id/message_id
build immutable routing fields
build DME body
encode plaintext without signature deterministically
sign hash + aad_hash
encode full plaintext
HPKE seal to contact hpke public key
create routing block with payload hash/size
create BPv7 bundle with age/hop/routing/payload blocks
validate own encoded bundle
persist atomically
zeroize transient plaintext buffers where feasible
```

## 11. 파서 제한

- 전체 bundle 최대 12 KiB
- payload ciphertext 최대 8 KiB
- CBOR nesting 최대 12
- collection 항목 최대 32
- text는 valid UTF-8만
- indefinite-length CBOR 금지
- duplicate map key 금지; v1은 가능한 한 array schema 사용
- unknown major version 거부
- unknown minor field는 v1 array 구조에서 허용하지 않음

## 12. 버전 정책

- protocol major는 광고와 HELLO에 포함
- major가 다르면 연결 후 `UNSUPPORTED_VERSION`으로 종료
- minor 기능은 capability bit로 협상
- DME v1 decoder는 v2 payload를 추측해 읽지 않는다.
- wire 변경 시 golden vector를 추가한다.


---

<!-- SOURCE: docs/04-protocol-ble-cla-v1.md -->

# 04. BLE Convergence Layer Adapter v1

## 1. 목표

BLE-CLA는 주변 노드 발견, 안전한 링크 세션, bundle offer/request/chunk transfer만 담당한다. 메시지 의미와 라우팅 결정을 수행하지 않는다.

## 2. UUID

프로젝트 공개 전에 UUID를 한 번 생성해 영구 고정한다. 이 문서의 구현 기준 UUID:

```text
Service:            6f1d0001-8f6b-4d5b-9c61-57c43d4d4d31
Control RX:         6f1d0002-8f6b-4d5b-9c61-57c43d4d4d31
Control TX:         6f1d0003-8f6b-4d5b-9c61-57c43d4d4d31
Data RX:            6f1d0004-8f6b-4d5b-9c61-57c43d4d4d31
Data TX:            6f1d0005-8f6b-4d5b-9c61-57c43d4d4d31
```

- RX는 central이 peripheral에 write
- TX는 peripheral이 central에 indicate/notify
- Control TX는 indication 선호
- Data TX는 notification 사용
- Data RX는 write without response를 기본으로 하되 credit flow control 적용

## 3. 광고 payload

BLE 광고에는 service UUID와 manufacturer/service data를 넣는다.

Service Data 12 bytes:

| offset | size | field |
|---:|---:|---|
| 0 | 1 | protocol major = 1 |
| 1 | 1 | capability bits low |
| 2 | 1 | mode/load bits |
| 3 | 1 | reserved = 0 |
| 4 | 8 | ephemeral beacon ID |

Capability bits:

- bit0: GATT server
- bit1: relay enabled
- bit2: fixed relay
- bit3: supports Noise XX
- bit4: supports inventory paging
- bit5: supports resume bitmap

Mode/load byte:

- bits 0..1: mode (`00 standby`, `01 emergency`, `10 fixed`, `11 reserved`)
- bits 2..3: queue load (`00 empty`, `01 low`, `10 medium`, `11 high`)
- bits 4..7 reserved

금지 정보:

- identity ID
- stable node ID
- contact slot
- location
- exact bundle count

`ephemeral beacon ID`는 CSPRNG 8 bytes이며 10분 ±2분 jitter마다 회전한다. 활성 링크 동안은 회전해도 기존 session을 끊지 않는다.

## 4. Central/Peripheral 역할 중복 방지

두 기기가 서로 발견하면 다음 규칙을 적용한다.

1. local beacon ID와 remote beacon ID를 unsigned lexicographic 비교
2. 더 작은 beacon ID를 가진 노드가 central 연결을 시도
3. 5초 내 연결이 없고 양쪽 모두 다시 광고되면 0~1500ms random fallback으로 어느 쪽이든 시도 가능
4. 이미 같은 remote session fingerprint와 연결 중이면 두 번째 링크 거부

beacon ID 충돌이면 Bluetooth address/OS handle을 안정 ID로 사용하지 말고 random fallback한다.

## 5. GATT 초기화

연결 후:

- Android central은 API가 허용하면 MTU 517 요청
- 실제 frame payload는 양쪽이 보고한 `max_att_payload`의 최솟값
- 최소 지원 ATT application payload 20 bytes
- 20 bytes 미만이면 연결 종료
- PHY 변경은 최적화이며 필수 아님
- connection priority HIGH는 transfer 중에만 요청하고 종료 후 BALANCED로 환원

## 6. Plain pre-handshake frame

Noise 완료 전에는 아래 frame type만 허용한다.

```text
0x01 VERSION_HELLO
0x02 NOISE_MESSAGE
0x03 PLAIN_ERROR
```

Frame header 8 bytes, big-endian:

| offset | size | field |
|---:|---:|---|
| 0 | 1 | magic `0xD7` |
| 1 | 1 | frame type |
| 2 | 1 | flags |
| 3 | 1 | header version = 1 |
| 4 | 2 | payload length |
| 6 | 2 | sequence modulo 65536 |

payload length가 현재 characteristic 한도를 넘으면 frame 자체를 segment한다. pre-handshake 최대 logical frame은 512 bytes.

## 7. VERSION_HELLO

CBOR array:

```text
[
  protocol_major,
  protocol_minor,
  beacon_id,
  max_control_frame,
  max_data_chunk,
  capabilities,
  random_session_nonce
]
```

- major 불일치: `PLAIN_ERROR(UNSUPPORTED_VERSION)` 후 종료
- 양쪽 hello bytes의 canonical hash를 Noise prologue에 포함

## 8. Noise session

고정 protocol name:

```text
Noise_XX_25519_ChaChaPoly_BLAKE2s
```

- Noise static key는 설치 identity와 별도로 생성한 link key
- QR 연락처 신원과 Noise static key를 동일시하지 않는다.
- handshake prologue:

```text
SHA256("DisasterMesh/BLE-CLA/1" || initiatorHello || responderHello)
```

XX 3-message handshake 완료 후:

- send/receive cipher state 획득
- 64-bit encrypted frame counter를 각 방향 0부터 사용
- 재사용/역행 counter 즉시 링크 종료
- session key material은 연결 종료 시 zeroize

## 9. Encrypted frame

Noise transport plaintext 안에 다음 frame을 넣는다.

Header 16 bytes:

| offset | size | field |
|---:|---:|---|
| 0 | 1 | frame type |
| 1 | 1 | flags |
| 2 | 2 | reserved=0 |
| 4 | 4 | stream ID |
| 8 | 4 | sequence |
| 12 | 4 | payload length |

Noise ciphertext가 GATT 한도보다 크면 outer BLE segment로 나눈다. segment는 link-local이며 encrypted frame sequence와 별도다.

## 10. Encrypted frame types

| code | frame |
|---:|---|
| 0x10 | SESSION_HELLO |
| 0x11 | ROUTING_SLOTS |
| 0x12 | INVENTORY_PAGE |
| 0x13 | BUNDLE_REQUEST |
| 0x14 | BUNDLE_META |
| 0x15 | BUNDLE_CHUNK |
| 0x16 | BUNDLE_COMMIT |
| 0x17 | TRANSFER_ACK |
| 0x18 | CREDIT_UPDATE |
| 0x19 | PING |
| 0x1A | PONG |
| 0x1B | ERROR |
| 0x1C | GOODBYE |

## 11. SESSION_HELLO

```text
[
  session_id: bstr16,
  negotiated_minor,
  node_capabilities,
  mode,
  max_concurrent_streams,
  max_session_bytes,
  max_session_seconds,
  current_age_resolution_ms
]
```

v1 기본:

- max concurrent transfer stream: 1
- emergency/fixed session byte budget: 256 KiB
- standby session byte budget: 64 KiB
- session timeout: 30초 standby, 90초 emergency/fixed

## 12. ROUTING_SLOTS

최종 수신자 직접 전달 판정용이다.

```text
[ page, is_last, [slot16, slot16, ...] ]
```

- 한 page 최대 32개
- 링크 encryption 후에만 교환
- 연락처 수가 많으면 current + previous rotation slots만
- v1 slot은 장기이므로 접촉 peer가 반복 관찰할 수 있다는 한계가 있다.

## 13. Inventory exchange

v1은 Bloom filter를 필수로 쓰지 않는다. false positive와 복구 복잡성을 줄이기 위해 명시적인 offer page를 사용한다.

`INVENTORY_PAGE`:

```text
[
  page_token: uint,
  is_last: bool,
  entries: [* BundleSummary]
]

BundleSummary = [
  packet_id16,
  destination_slot16,
  priority,
  remaining_lifetime_seconds,
  hop_count,
  hop_limit,
  copy_tokens,
  total_bundle_bytes
]
```

- 최대 32 entries/page
- 순서는 routing score 내림차순
- 상대 slot에 직접 해당하는 bundle을 항상 첫 page에 넣는다.
- peer가 가진 packet ID는 요청하지 않는다.

`BUNDLE_REQUEST`:

```text
[ [packet_id16, requested_reason], ... ]
```

reason:

- 1 DIRECT_DESTINATION
- 2 RELAY_COPY
- 3 RECEIPT_OR_CANCEL

한 request 최대 16개.

## 14. Bundle transfer

### BUNDLE_META

```text
[
  transfer_id16,
  packet_id16,
  total_size,
  sha256,
  chunk_size,
  chunk_count,
  proposed_receiver_tokens,
  sender_remaining_tokens_after_commit
]
```

### BUNDLE_CHUNK

binary payload:

```text
transfer_id16 || chunk_index_u32 || chunk_crc32c_u32 || bytes
```

### BUNDLE_COMMIT

```text
[ transfer_id16, packet_id16, total_sha256 ]
```

### TRANSFER_ACK

```text
[ transfer_id16, packet_id16, status, accepted_tokens, persisted_hash ]
```

status:

- 1 COMMITTED
- 2 DUPLICATE
- 3 REJECTED_INVALID
- 4 REJECTED_QUOTA
- 5 REJECTED_EXPIRED
- 6 RETRY_LATER

copy token split은 `COMMITTED` ACK 후 sender DB transaction에서 확정한다. `DUPLICATE`는 token을 이동하지 않는다.

## 15. Credit flow control

- receiver가 `CREDIT_UPDATE(bytes)`를 보낸 만큼만 data frame 전송
- 초기 credit: `4 * negotiated_chunk_size`
- receiver가 DB/temp write 완료할 때 credit 보충
- credit 0에서 write without response 금지
- control frame은 별도 소량 reserve를 둔다.

## 16. Resume

v1 resume은 같은 transfer ID가 아니라 packet ID 기준이다.

- partial transfer는 10분 유지
- 재연결 시 receiver가 `BUNDLE_REQUEST`와 함께 optional received bitmap hash 제공
- sender가 BUNDLE_META 후 receiver bitmap을 요청
- chunk count 최대 1024
- bitmap 불일치 시 처음부터 재전송

초기 Goal 3에서는 resume을 미구현하고 전체 재전송해도 된다. Goal 4 완료 전에 구현한다.

## 17. 타임아웃

| 항목 | 기본 |
|---|---:|
| GATT connect | 12초 |
| service discovery | 8초 |
| hello exchange | 5초 |
| Noise handshake | 10초 |
| encrypted frame idle | 15초 |
| chunk ACK/credit idle | 10초 |
| full session | 30/90초 |
| peer cooldown success | 30초 |
| peer cooldown no work | 2분 |
| peer backoff failure | 5초 → 최대 10분 |

모든 값은 remote peer별 jitter ±20% 적용.

## 18. 세션 종료 사유

- NO_COMMON_VERSION
- NO_COMMON_CAPABILITY
- NO_USEFUL_BUNDLES
- SESSION_BUDGET_EXHAUSTED
- LOW_BATTERY
- PROTOCOL_VIOLATION
- NOISE_FAILURE
- TRANSPORT_FAILURE
- USER_DISABLED

상대에게 세부 보안 오류를 과도하게 노출하지 않는다.


---

<!-- SOURCE: docs/05-routing-and-queue.md -->

# 05. Routing, Queue and Congestion Control

## 1. 알고리즘

v1은 `Direct Delivery + Binary Spray-and-Wait`를 사용한다.

- 상대가 목적지 slot을 보유하면 copy token과 무관하게 직접 전달한다.
- 상대가 목적지가 아니면 local copy token이 2 이상일 때만 relay copy를 제안한다.
- token을 절반으로 분할한다.
- token 1인 copy는 목적지를 만날 때까지 wait한다.

## 2. Token split

```rust
fn split_tokens(tokens: u8) -> Option<(u8, u8)> {
    if tokens < 2 { return None; }
    let receiver = tokens / 2;
    let sender = tokens - receiver;
    Some((sender, receiver))
}
```

예:

| before | sender after | receiver |
|---:|---:|---:|
| 2 | 1 | 1 |
| 3 | 2 | 1 |
| 6 | 3 | 3 |
| 7 | 4 | 3 |
| 12 | 6 | 6 |

receiver commit 전에는 sender token을 변경하지 않는다.

## 3. 수신 요청 판정

```text
if tombstoned(packet_id): reject
if already_committed(packet_id): duplicate
if expired(bundle): reject expired
if malformed summary: reject
if destination slot belongs to me:
    request DIRECT_DESTINATION regardless of copy_tokens
else if relay disabled:
    skip
else if hop_count + 1 >= hop_limit:
    skip
else if peer copy_tokens < 2:
    skip
else if quota unavailable:
    skip
else if source/peer rate limited:
    skip
else:
    request RELAY_COPY
```

`hop_count + 1 == hop_limit`인 copy는 receiver가 최종 목적지가 아닐 경우 요청하지 않는다.

## 4. Offer score

높은 점수를 먼저 offer한다.

```text
score =
  priority_weight
+ direct_destination_bonus
+ control_message_bonus
+ expiry_urgency
+ age_bonus
+ size_efficiency
- recently_offered_penalty
- peer_failure_penalty
```

고정 정수식:

```text
priority_weight:
  P0 1_000_000
  P1   500_000
  P2   100_000
  P3         0

direct_destination_bonus = 2_000_000
receipt_or_cancel_bonus   =   750_000
expiry_urgency            = max(0, 100_000 - remaining_seconds)
age_bonus                  = min(age_minutes, 10_000)
size_efficiency            = max(0, 8192 - size_bytes)
recently_offered_penalty   = 100_000 if same peer within 10 min
peer_failure_penalty       = failure_count_24h * 10_000, cap 100_000
```

점수 동률이면 packet ID lexicographic 순으로 결정해 테스트 재현성을 유지한다.

## 5. 기본 정책

| type | priority | TTL | hop limit | tokens |
|---|---:|---:|---:|---:|
| PRIVATE_SOS | P0 | 24h | 16 | 12 |
| DELIVERY_RECEIPT | P0 | 7d | 16 | 12 |
| CANCEL | P0 | 7d | 16 | 12 |
| CHECK_IN | P1 | 48h | 12 | 8 |
| LOCATION_UPDATE | P1 | 24h | 12 | 8 |
| DIRECT_TEXT | P2 | 72h | 12 | 6 |

앱 UI에서 사용자가 priority, hop, tokens를 임의 조절하지 못한다.

## 6. Age 계산

- bundle 저장 시 `received_elapsed_realtime_ms` 기록
- offer 직전에:

```text
current_age = stored_age_ms + (elapsedRealtimeNow - received_elapsed_realtime_ms)
```

- forward encode 시 Bundle Age block에 current_age 반영
- reboot 후 monotonic 기준이 끊기므로 shutdown/recovery wall time 차이를 보수적으로 더한다.
- wall clock이 역행하거나 신뢰 불가하면 `reboot_age_penalty = max(boot_gap_estimate, 5 minutes)` 적용
- local hard max lifetime을 초과하면 삭제

## 7. Dedup/tombstone

Dedup key는 `packet_id16`과 BP identity hash를 모두 사용한다.

- packet ID 같고 payload hash 같음: duplicate
- packet ID 같고 payload hash 다름: conflict quarantine + peer penalty
- receipt/cancel 처리 후 tombstone 생성
- tombstone 기본 보존: 원본 lifetime + 24시간, 최대 8일
- P0 packet tombstone은 최소 48시간

## 8. Receipt 처리

수신자:

1. payload decrypt/signature verify
2. local message commit
3. receipt bundle 생성
4. original packet tombstone을 즉시 만들지 않는다. 다른 copy가 도착해도 duplicate 처리한다.

발신자 또는 relay:

- 검증된 receipt를 받으면 original packet을 `DELIVERED` 처리
- relay copy는 payload 삭제 가능
- tombstone 생성
- receipt 자체는 발신 목적지에 전달될 때까지 유지

## 9. Cancel 처리

- cancel sender identity가 원본 sender identity와 같아야 한다.
- target packet/message ID가 일치해야 한다.
- relay는 원본을 삭제하고 tombstone 생성
- 수신 UI는 `취소됨` 표시; 이미 읽은 내용의 삭제 보장은 하지 않음
- cancel bundle은 원본보다 긴 TTL을 가질 수 있다.

## 10. 저장 quota

기본 total 32 MiB.

```text
P0/P1 reserved        8 MiB
per-source hard cap   4 MiB
per-peer/day ingest   8 MiB
single bundle         12 KiB
cipher payload         8 KiB
partial transfers      4 MiB total
```

source identity를 relay가 모를 수 있으므로 외부 random source EID와 ingress peer를 함께 quota key로 사용한다. 악성 peer가 source를 계속 바꿀 수 있으므로 peer/day quota가 필수다.

## 11. Eviction

정확한 순서:

1. invalid/quarantined
2. expired
3. receipt-confirmed originals
4. canceled originals
5. stale partial transfers
6. P3 oldest/lowest score
7. P2 oldest/lowest score
8. P1, 단 reserved floor 아래로 내리지 않음
9. P0, 전체 시스템이 disk-full인 경우에만 가장 오래된 것

사용자 자신의 outbound P0는 relay copy보다 우선한다.

## 12. Peer scheduling

peer 후보 score:

```text
+ has_direct_destination_hint 1000
+ fixed_relay                 300
+ new_peer                    100
+ prior_success_rate * 100
- recent_contact_penalty      500
- consecutive_failures * 100
- low_battery_peer_hint       100
```

- 동시에 1개 active GATT session을 v1 기본으로 한다.
- 고정 릴레이/고성능 기기에서 2개를 실험할 수 있으나 1.0 기본 OFF.

## 13. Battery policy

| 상태 | scan | advertise | relay |
|---|---|---|---|
| Standby, >30% | 10초 scan / 50초 sleep | low duty | P0/P1/P2 |
| Emergency, >20% | 20초 scan / 10초 sleep | active | all |
| Fixed + charging | near continuous with jitter | active | all |
| Battery 10~20% | duty 50% 감속 | 유지 | P0/P1 우선 |
| Battery <10% | 5초/5분 | sparse | own P0 + direct only |
| thermal severe | stop scan 5분 | sparse | current transfer finish then stop |

Android OS가 실제 주기를 조정할 수 있으므로 값은 목표 정책이며 정확한 wake-up을 보장하지 않는다.

## 14. 시뮬레이터 필수 지표

- delivery ratio by priority
- p50/p95 latency
- replicas per delivered packet
- bytes per successful delivery
- drops by reason
- storage high-water mark
- peer contact utilization
- receipt return ratio
- battery cost proxy

1.0 정책값은 10/50/100/500 node 시나리오 결과를 근거로 조정한다.


---

<!-- SOURCE: docs/06-security-and-threat-model.md -->

# 06. Security and Threat Model

## 1. 보호 목표

- relay와 수동 도청자가 메시지 본문·정확한 위치를 읽지 못한다.
- 수신자가 발신자 서명을 검증할 수 있다.
- header/payload 조작을 검출한다.
- 재생·중복·저장공간 공격의 피해를 제한한다.
- 서버·CA 없이 대면 연락처 인증을 제공한다.

## 2. 비보장

- 무선 방해 대응
- 악성 relay의 선택적 drop 방지
- 충분한 접촉 경로가 없는 경우 전달
- 탈취·루팅된 최종 사용자 기기의 평문 보호
- 중앙 등록 없는 Sybil 완전 방지
- stable routing slot에 대한 완전한 메타데이터 익명성
- Signal Double Ratchet 수준의 지속적 forward secrecy

## 3. Trust boundary

```text
Trusted:
- local UI process after device unlock
- Rust core binary built from verified source
- Android Keystore implementation within platform assumptions
- in-person verified contact public keys

Untrusted:
- all BLE peers
- relay storage
- radio channel
- imported files/QR before validation
- wall clock
- external diagnostic recipient
```

## 4. 키 종류

| 키 | 용도 | 저장 |
|---|---|---|
| Ed25519 identity key | message signature | encrypted key blob; wrapping key in Keystore |
| X25519 HPKE key | recipient encryption | encrypted key blob; wrapping key in Keystore |
| X25519 Noise static key | link authentication only | encrypted key blob |
| DB master key 256-bit | local plaintext-at-rest | wrapped by Keystore AES key |
| ephemeral HPKE sender key | message seal | memory only, immediate zeroize |
| Noise session keys | link frames | memory only |

Android Keystore가 Ed25519/X25519를 직접 모든 지원 OS에서 일관되게 제공한다고 가정하지 않는다. Rust가 private key를 생성하고, raw private key blob을 random DB master key로 암호화하며 DB master key만 Keystore AES-GCM key로 wrap하는 방식을 기준으로 한다.

## 5. Local key wrapping

1. Android Keystore에 non-exportable AES-256 key alias `dm_local_wrap_v1` 생성
2. Rust CSPRNG로 32-byte DB master key 생성
3. Keystore AES-GCM으로 master key wrap
4. wrapped key, IV, key version만 preferences/DB header에 저장
5. private keys와 평문 message rows는 master key에서 HKDF로 분리한 subkey로 XChaCha20-Poly1305 암호화

subkey context:

```text
DisasterMesh/local/private-keys/v1
DisasterMesh/local/messages/v1
DisasterMesh/local/diagnostics/v1
```

Keystore invalidation 시 자동 새 identity 생성하지 않는다. 복구 불가 상태를 사용자에게 알리고 encrypted relay data만 삭제 가능하게 한다.

## 6. Contact Card

QR payload는 `DM1:` prefix + Base45(CBOR bytes) + checksum 형태를 권장한다.

내용:

- protocol version
- Ed25519 public key
- X25519 HPKE public key
- outbound routing slot
- optional display name
- key version
- card signature

card signature는 card의 signature 필드 제외 canonical bytes를 identity key로 서명한다.

검증:

- 길이/CBOR/CDDL
- self-signature
- display ID/checksum
- duplicate/key change
- 화면 안전번호 비교

안전번호:

```text
SHA256(min(A identity pub, B identity pub) || max(...))
```

앞 60 bits를 12자리 5-bit word/decimal groups로 표시한다. 양쪽 앱이 동일한 정렬 규칙을 써야 한다.

## 7. Message security

- HPKE Base는 recipient confidentiality를 제공한다.
- Ed25519 signature는 sender authentication을 제공한다.
- sender identity/signature는 ciphertext 내부라 relay에 노출되지 않는다.
- AAD가 destination, lifetime, priority, packet ID를 묶는다.
- copy token/hop/age는 mutable이라 AAD 제외.

### Key compromise

- recipient long-term X25519 private key가 유출되면 기록된 과거 HPKE ciphertext가 위험할 수 있다.
- v1은 이를 명시한다.
- v1.1에서 signed prekey/one-time prekey 또는 검증된 ratchet 도입을 별도 ADR로 평가한다.

## 8. Link security

Noise XX는 연결 시 양쪽 static link key를 교환하고 forward-secret transport key를 만든다. 하지만 처음 만난 relay의 static key를 사전에 신뢰하지 않으므로 링크는 기밀성/무결성 채널이지 사람 identity 인증이 아니다.

- peer link key fingerprint는 encounter history에 hash로 저장
- 갑작스러운 key 변화는 진단 이벤트일 뿐 메시지 contact key 변화와 혼동하지 않는다.
- Noise failure 상세는 remote에 노출하지 않는다.

## 9. 공격과 대응

| 공격 | 대응 | 잔여 위험 |
|---|---|---|
| BLE sniffing | Noise + payload E2EE | traffic timing/size 노출 |
| relay DB 탈취 | ciphertext only | destination slot/priority 노출 |
| payload 변조 | CRC, hash, HPKE AEAD, signature | drop 가능 |
| replay | packet ID, tombstone, sender sequence | storage pressure |
| fake identity | in-person QR/safety number | 사용자의 잘못된 신뢰 |
| Sybil | peer/source quota, no public chat | 완전 차단 불가 |
| bundle flood | size/TTL/rate/quota | P0 위장 메타데이터 |
| priority inflation | encrypted signed inner value, local caps | relay는 검증 전 우선 처리 가능 |
| selective drop | multiple copies/receipts | 보장 불가 |
| malformed parser | size/depth limits, fuzzing | 구현 버그 |
| key substitution QR | self-signature + safety compare | QR만 원격 전달 시 MITM |
| log leakage | structured redaction | 사용자 screenshot/export |
| DB rollback | sequence window, message IDs | 완전한 secure monotonic counter 없음 |

## 10. Logging rules

절대 기록 금지:

- plaintext body
- private keys
- full contact public keys
- exact latitude/longitude
- full routing slot
- HPKE decrypted bytes

허용:

- packet ID 앞 6 bytes hash 표기
- peer link fingerprint 앞 6 bytes
- error category
- size, duration, counts
- coarse mode/battery bucket

## 11. Secure coding rules

- Rust `unsafe`는 FFI boundary 이외 금지; 사용 시 ADR/리뷰
- secret type에 `Debug` 구현 금지
- zeroize 가능한 buffer 사용
- parsing 전에 allocation upper bound 확인
- crypto error를 `InvalidCiphertext` 하나로 축약
- random은 OS CSPRNG만
- nonce 직접 증가/재사용 설계 금지; HPKE/Noise library가 관리
- 테스트 전용 고정 key/nonce는 production feature flag에서 컴파일 불가

## 12. 보안 출시 게이트

- protocol test vector 공개
- cargo-fuzz 최소 24시간 campaign 결과
- QR/parser/CBOR/BPv7 fuzz corpus
- dependency advisory 0 critical/high 또는 문서화된 예외
- 외부 crypto/protocol review
- threat model 최신화
- known limitation UI 반영
- 취약점 제보 이메일/SECURITY.md


---

<!-- SOURCE: docs/07-storage-schema.md -->

# 07. Storage Schema and Transactions

## 1. 결정

프로토콜 상태의 단일 소유자는 Rust core다. Rust core가 전용 SQLite DB를 열고 transaction을 직접 수행한다.

Android 측 저장 범위:

- UI preference
- 권한 안내 상태
- Keystore로 wrap된 DB master key

Rust SQLite 범위:

- identity/contact
- message/bundle/payload
- transfer/receipt/tombstone
- peer encounter/rate limit
- diagnostics counters

이 결정은 Kotlin↔Rust 사이의 다단계 DB transaction과 copy-token race를 제거한다.

## 2. DB 파일

```text
filesDir/mesh/mesh-v1.sqlite3
filesDir/mesh/partials/{transfer_id}.part
filesDir/mesh/quarantine/{packet_id}.bin
```

- WAL mode
- foreign_keys ON
- synchronous FULL in emergency/fixed relay, NORMAL in standby
- busy_timeout 3초
- auto_vacuum INCREMENTAL
- user_version 기반 migration

## 3. 암호화 범위

- relay bundle bytes: 이미 E2EE ciphertext이므로 DB에 그대로 저장 가능
- contact display name, local plaintext message, private key blob: column-level XChaCha20-Poly1305
- 암호화 nonce와 ciphertext를 함께 저장
- key derivation context를 table/column별로 분리
- SQLite page 전체 암호화는 1.0 필수가 아니며 필요 시 SQLCipher ADR을 추가한다.

## 4. 핵심 transaction

### TX-01 Identity bootstrap

- identity row 생성
- sender sequence 0
- routing slot 생성
- schema/meta 생성
- 모두 성공하거나 전부 rollback

### TX-02 Outbound message

- sender sequence +1
- messages insert
- bundles insert
- bundle_payloads insert
- diagnostic counter update

### TX-03 Inbound bundle commit

- partial hash 검증
- duplicate/tombstone 재검사
- bundles/payload insert
- peer ingress accounting
- partial row/file delete
- 최종 수신자면 decrypt result message insert + receipt bundle insert

### TX-04 Relay copy commit ACK

- transfer state committed
- sender bundle copy_tokens 감소
- peer success stat update

ACK 전에는 token 변경 금지.

### TX-05 Receipt/cancel

- control message 검증
- target bundle state update
- payload 삭제 조건 확인
- tombstone insert
- local message state update

## 5. Migration 정책

- 모든 migration은 forward-only
- DB가 binary보다 높은 schema version이면 앱은 read-only 오류 화면
- migration 전 `.bak` copy 생성은 DB가 64 MiB 미만일 때 수행
- migration 실패 시 원본 유지
- destructive migration 금지
- protocol version과 DB schema version은 독립 관리

## 6. Cleanup

앱 시작, 15분 timer, 저장공간 임계치 진입 시 실행:

1. expired partials
2. expired bundles
3. delivered/canceled payloads
4. tombstone expiry
5. diagnostic events retention
6. incremental vacuum 최대 1000 pages

한 번의 cleanup은 200 rows 또는 2초에서 중단하고 다음 tick에 재개한다.

## 7. Query API

UI에 raw SQL을 노출하지 않는다.

- `list_contacts(cursor, limit)`
- `list_conversations(cursor, limit)`
- `list_messages(conversation_id, before, limit)`
- `get_relay_status()`
- `get_outbound_status(message_id)`
- `list_diagnostics(since, limit)`

limit 최대 100.


---

<!-- SOURCE: docs/08-rust-core-contract.md -->

# 08. Rust Core and FFI Contract

## 1. 설계 결정

- Rust core가 protocol DB와 모든 protocol state를 소유한다.
- Android는 한 개의 serial actor에서만 `MeshEngine`을 호출한다.
- FFI facade crate는 하나만 배포한다. 여러 UniFFI library를 앱에 중복 포함하지 않는다.
- BLE platform은 raw byte transport다. Noise/frame/session state는 Rust가 소유한다.

## 2. Engine lifecycle

```text
KeyVault unwrap master key
        ↓
MeshEngine::open(config, db_path, master_key, boot_context)
        ↓
recover incomplete sessions/transfers
        ↓
handle events sequentially
        ↓
shutdown() → flush WAL, zeroize secrets
```

master key는 FFI 호출 직후 Rust-owned secret buffer로 이동하고 Kotlin byte array는 overwrite한다.

## 3. Public facade

```rust
pub struct MeshEngine;

impl MeshEngine {
    pub fn open(config: EngineConfig, secrets: OpenSecrets) -> Result<Self, CoreError>;
    pub fn bootstrap_identity(&self, display_name: String) -> Result<IdentitySummary, CoreError>;
    pub fn identity_summary(&self) -> Result<IdentitySummary, CoreError>;
    pub fn export_contact_card(&self) -> Result<Vec<u8>, CoreError>;
    pub fn import_contact_card(&self, encoded: Vec<u8>, verified: bool) -> Result<ContactSummary, CoreError>;

    pub fn create_direct_text(&self, draft: DirectTextDraft) -> Result<CreateMessageResult, CoreError>;
    pub fn create_check_in(&self, draft: CheckInDraft) -> Result<CreateMessageResult, CoreError>;
    pub fn create_private_sos(&self, draft: PrivateSosDraft) -> Result<CreateMessageResult, CoreError>;
    pub fn cancel_message(&self, message_id: Id16, reason: CancelReason) -> Result<CreateMessageResult, CoreError>;

    pub fn handle_transport_event(&self, event: TransportEvent) -> Result<Vec<PlatformCommand>, CoreError>;
    pub fn handle_system_event(&self, event: SystemEvent) -> Result<Vec<PlatformCommand>, CoreError>;

    pub fn home_snapshot(&self) -> Result<HomeSnapshot, CoreError>;
    pub fn list_contacts(&self, cursor: Option<Vec<u8>>, limit: u32) -> Result<ContactPage, CoreError>;
    pub fn list_conversations(&self, cursor: Option<Vec<u8>>, limit: u32) -> Result<ConversationPage, CoreError>;
    pub fn list_messages(&self, conversation: Id16, before_ms: Option<u64>, limit: u32) -> Result<MessagePage, CoreError>;
    pub fn diagnostics_snapshot(&self) -> Result<DiagnosticsSnapshot, CoreError>;
    pub fn export_diagnostics(&self, options: DiagnosticExportOptions) -> Result<Vec<u8>, CoreError>;
    pub fn shutdown(&self) -> Result<(), CoreError>;
}
```

모든 method는 blocking 가능성이 있으므로 Android actor의 dedicated dispatcher에서 호출한다. UI main thread 호출 금지.

## 4. TransportEvent

```rust
pub enum TransportEvent {
    PeerDiscovered {
        peer_handle: String,
        beacon_id: [u8; 8],
        capabilities: u32,
        rssi: i16,
    },
    LinkOpened {
        link_id: u64,
        peer_handle: String,
        role: LinkRole,
        max_write_bytes: u32,
    },
    BytesReceived {
        link_id: u64,
        channel: LinkChannel,
        bytes: Vec<u8>,
    },
    LinkWritable { link_id: u64, channel: LinkChannel },
    LinkClosed { link_id: u64, reason: LinkCloseReason },
    ConnectFailed { peer_handle: String, category: TransportFailure },
}
```

`peer_handle`은 OS process lifetime 동안만 유효한 opaque string이다. 영구 identity로 저장하지 않는다.

## 5. SystemEvent

```rust
pub enum SystemEvent {
    ModeChanged(RelayMode),
    BluetoothStateChanged(BluetoothState),
    PermissionsChanged(PermissionSnapshot),
    BatteryChanged { percent: u8, charging: bool, thermal: ThermalState },
    TimerTick { elapsed_ms: u64, wall_ms: Option<u64> },
    AppForegroundChanged(bool),
    LocationResult { request_id: Id16, result: LocationResult },
    StoragePressure(StoragePressure),
}
```

## 6. PlatformCommand

```rust
pub enum PlatformCommand {
    StartScan(ScanPolicy),
    StopScan,
    StartAdvertising(AdvertisementData),
    UpdateAdvertising(AdvertisementData),
    StopAdvertising,
    Connect { peer_handle: String },
    Disconnect { link_id: u64, reason: String },
    SendBytes { link_id: u64, channel: LinkChannel, bytes: Vec<u8>, with_response: bool },
    RequestConnectionPriority { link_id: u64, high: bool },
    RequestLocation { request_id: Id16, timeout_seconds: u32 },
    ShowIncomingMessage { message_id: Id16 },
    UpdatePersistentNotification(NotificationModel),
    ShowLocalAlert(LocalAlert),
    ScheduleWake { after_ms: u64, reason: WakeReason },
}
```

platform은 command 실행 결과를 다시 event로 돌려준다. 실행 성공을 core가 추측하지 않는다.

## 7. Command execution order

하나의 return batch는 배열 순서대로 실행한다.

- `SendBytes` 실패 시 이후 같은 link command를 중단하고 LinkClosed/ConnectFailed event를 보낸다.
- notification 실패는 transport command를 중단하지 않는다.
- Start/Stop scan/advertise는 idempotent adapter가 처리한다.

## 8. Core error

UI-safe error code와 internal diagnostic code를 분리한다.

```rust
pub struct CoreError {
    pub public_code: PublicErrorCode,
    pub internal_code: u32,
    pub retryable: bool,
}
```

오류 message string에 secret/packet bytes를 포함하지 않는다.

## 9. Pure decision functions

아래는 DB나 clock에 직접 접근하지 않는 pure function으로 구현한다.

```rust
validate_dme(bytes, context) -> ValidatedDme
route_decision(local_bundle, peer_context, policy) -> RouteDecision
eviction_plan(store_snapshot, required_bytes, policy) -> Vec<PacketId>
peer_score(peer_stats, policy) -> i64
next_scan_policy(mode, battery, thermal) -> ScanPolicy
```

property test의 핵심 대상이다.

## 10. Crate dependency baseline

실제 버전은 lockfile에 고정하고 업데이트 PR에서만 변경한다.

필요 범주:

- CBOR: deterministic encoding을 통제 가능한 crate
- BPv7: `bp7` crate 검토 후 wrapper; conformance gap을 자체 테스트
- HPKE: RFC 9180 구현 crate; 외부 감사 여부 별도 기록
- Ed25519/X25519: RustCrypto 계열
- Noise: maintained Noise framework implementation
- SQLite: rusqlite
- FFI: UniFFI
- secret zeroization: zeroize/secrecy 계열
- property test: proptest
- fuzz: cargo-fuzz/libFuzzer

crate 이름만 보고 보안성을 가정하지 않으며 `docs/dependency-review.md`에 버전·license·audit 상태를 기록한다.


---

<!-- SOURCE: docs/09-android-implementation.md -->

# 09. Android Implementation Specification

## 1. Baseline

```text
minSdk      26
compileSdk  36
targetSdk   36
JDK         17
Kotlin      2.4.x stable line
AGP         current stable 9.x line selected at bootstrap and locked
UI          Jetpack Compose
```

빌드 도구의 patch version은 저장소 생성 시 공식 호환표에 맞춰 잠그고 Renovate/Dependabot 자동 merge를 금지한다.

## 2. Application variants

### `offlineRelease`

- `INTERNET` permission 없음
- analytics 없음
- crash upload 없음
- core production variant

### `devDebug`

- optional local protocol inspector
- adb logcat diagnostics
- test-only deterministic random provider 허용
- production signing 불가

### `fieldTestRelease`

- offlineRelease와 동일 권한
- 상세 로컬 지표 화면
- export bundle에 build/test metadata 포함

## 3. Manifest

```xml
<manifest>
    <uses-feature
        android:name="android.hardware.bluetooth_le"
        android:required="true" />

    <uses-permission android:name="android.permission.BLUETOOTH" android:maxSdkVersion="30" />
    <uses-permission android:name="android.permission.BLUETOOTH_ADMIN" android:maxSdkVersion="30" />
    <uses-permission android:name="android.permission.ACCESS_FINE_LOCATION" android:maxSdkVersion="30" />

    <uses-permission
        android:name="android.permission.BLUETOOTH_SCAN"
        android:usesPermissionFlags="neverForLocation" />
    <uses-permission android:name="android.permission.BLUETOOTH_CONNECT" />
    <uses-permission android:name="android.permission.BLUETOOTH_ADVERTISE" />

    <uses-permission android:name="android.permission.FOREGROUND_SERVICE" />
    <uses-permission android:name="android.permission.FOREGROUND_SERVICE_CONNECTED_DEVICE" />
    <uses-permission android:name="android.permission.POST_NOTIFICATIONS" />

    <!-- Only request at runtime when user attaches GPS. -->
    <uses-permission android:name="android.permission.ACCESS_FINE_LOCATION" />

    <application ...>
        <service
            android:name=".service.EmergencyRelayService"
            android:exported="false"
            android:foregroundServiceType="connectedDevice" />
    </application>
</manifest>
```

`neverForLocation` 사용이 주변 BLE 결과를 제한하는 기기/OS 사례가 있는지 실기기에서 확인한다. 위치 첨부 기능 때문에 location permission은 별도 런타임 흐름으로 요청하되 BLE 권한 설명과 섞지 않는다.

## 4. Permission flow

```text
App start
 ├─ Bluetooth unsupported → unsupported screen
 ├─ Bluetooth off → system enable guidance
 ├─ permission missing → rationale → request
 ├─ notification denied → relay can run but persistent visibility caveat 표시
 └─ ready
```

권한 거부 시:

- 메시지 작성·연락처 관리는 가능
- relay OFF
- 홈에 `통신 기능 중지됨` 표시
- 설정 이동 버튼 제공
- 반복 팝업 금지

## 5. Foreground service

### Start 조건

- 사용자가 앱 화면에서 긴급/고정 릴레이 모드를 명시적으로 켬
- 필수 Bluetooth 권한 있음
- Bluetooth ON

### Service responsibilities

- `START_STICKY` 사용 여부는 제조사 시험 후 결정; 무조건 재시작을 안전 보장으로 표현하지 않음
- 즉시 `startForeground()`
- Coordinator actor 생성/연결
- scan/advertise policy command 실행
- persistent notification 갱신
- task removal, process recreation 상태 복구

### Notification

필수 내용:

```text
재난 통신 중계 중
모드: 긴급 | 주변 접촉: 4 | 보관: 12개
[중지] [상태 보기]
```

본문·연락처·정확한 위치를 표시하지 않는다.

## 6. BLE adapter

### Components

```text
BlePlatformAdapterImpl
├─ BleScanner
├─ BleAdvertiser
├─ GattCentralClient
├─ GattPeripheralServer
├─ LinkRegistry
├─ ByteSegmenter
└─ AndroidBleQuirkRegistry
```

### GATT callback rules

- callback 즉시 immutable event 생성
- actor channel `trySend`
- DB/FFI/blocking crypto 호출 금지
- event queue overflow 시 link를 안전 종료하고 진단 기록

### LinkRegistry

키:

- local numeric `linkId: Long`
- OS BluetoothDevice는 registry 내부에서만 보유
- Rust에는 opaque `peerHandle`과 linkId만 전달

중복 링크:

- 동일 beacon/session fingerprint의 두 번째 link를 닫는다.
- central/peripheral 양쪽 callback 경쟁을 mutex로 직렬화한다.

### Byte segmentation

GATT characteristic operation당 Android가 허용한 payload 크기로 outer segment:

```text
segment header:
magic 1 | logical_frame_id 4 | segment_index 2 | segment_count 2 | bytes
```

- logical frame 최대 64 KiB
- segment count 최대 1024
- reassembly timeout 10초
- 같은 frame ID collision 시 link 종료

## 7. Coordinator actor

```kotlin
class MeshCoordinatorImpl(
    private val engine: MeshEngine,
    private val adapter: BlePlatformAdapter,
    private val dispatcher: CoroutineDispatcher,
) {
    private val events = Channel<CoordinatorEvent>(capacity = 256)

    suspend fun run() = withContext(dispatcher) {
        for (event in events) {
            val commands = when (event) {
                is CoordinatorEvent.Transport -> engine.handleTransportEvent(event.value)
                is CoordinatorEvent.System -> engine.handleSystemEvent(event.value)
            }
            commands.forEach { command ->
                val result = adapter.execute(command)
                result.toFollowUpEvent()?.let { events.send(it) }
            }
        }
    }
}
```

- dispatcher는 `Dispatchers.Default.limitedParallelism(1)` 또는 dedicated thread
- generated FFI object는 coordinator 밖에서 호출 금지
- 256 queue overflow 정책: 저우선 RSSI discovery event drop, link bytes는 drop하지 않고 link close

## 8. UI screens

### Onboarding

- product limitation
- Bluetooth permission
- identity creation
- contact exchange tutorial

### Home

- mode state
- surrounding peer count(정확한 사람 수가 아니라 발견 기기 수)
- stored relay bundles/bytes
- own pending messages
- check-in/SOS entry

### Contacts

- verified/unverified/key-changed 표시
- QR display/scan
- safety number
- revoke

### Conversation

상태 라벨:

- 기기에 보관됨
- 중계망에 복제됨
- 전달 확인됨
- 만료됨
- 취소 전파 중

`전송 완료`는 receipt 전에는 사용하지 않는다.

### SOS

- category
- people/severe injury count
- description
- optional location/manual location
- recipients
- long-press 1.5초 send
- send 후 즉시 cancel button 제공

### Relay status

- current mode
- scan/advertise running
- active link
- stored counts by priority
- battery/thermal throttling
- last service interruption

## 9. Location

- Fused provider나 network location을 전제로 하지 않는다.
- Android platform GPS provider를 사용할 수 있어야 한다.
- 20초 timeout 기본
- 마지막 위치를 자동 첨부하지 않는다.
- 사용자가 accuracy와 capture time을 확인 후 첨부
- 위치 실패 시 manual description으로 계속 진행

## 10. Keystore bootstrap

```text
if wrap key absent:
  generate AES/GCM 256 Keystore key
if wrapped master key absent:
  random 32 bytes
  wrap with Keystore
  store wrapped blob
unwrap master key
pass once to Rust MeshEngine.open
zero Kotlin buffer
```

사용자가 생체 인증 앱 잠금을 켠 경우 UI plaintext 접근에만 적용한다. relay service가 잠긴 상태에서도 ciphertext를 중계할 수 있어야 한다.

## 11. Process recovery

앱 시작 시:

1. unwrap key
2. open Rust engine
3. DB integrity quick check
4. incomplete outgoing/incoming transfer recovery
5. mode preference 읽기
6. 사용자가 이전에 relay mode를 켰고 OS 정책상 시작 가능하면 명시적 notification과 함께 service 복구
7. 불가능하면 홈에 중단 상태 표시

## 12. Manufacturer quirks

`AndroidBleQuirkRegistry`는 모델별 hardcode보다 capability/실패 기반 fallback 우선.

- MTU 517 실패 → 247 → default
- write without response stall → write with response fallback
- advertise unsupported → scan-only node; UI 표시
- simultaneous scan/advertise failure → time slicing
- GATT 133 계열 → close, refresh 금지(private API), jitter retry

## 13. Source references

- Android Bluetooth permissions
- Android BLE background communication
- Android foreground service connected-device requirements
- Android 16/API 36 Bluetooth behavior changes

정확한 URL은 `docs/15-references.md`에 기록한다.


---

<!-- SOURCE: docs/10-state-machines.md -->

# 10. State Machines

## 1. Relay service

```text
STOPPED
  └─ user enables → STARTING
STARTING
  ├─ permissions/bluetooth ready → ACTIVE
  └─ failure → BLOCKED
ACTIVE
  ├─ battery/thermal policy → THROTTLED
  ├─ bluetooth off → BLOCKED
  ├─ OS/service destruction → RECOVERING
  └─ user stops → STOPPING
THROTTLED
  ├─ recovered → ACTIVE
  └─ user stops → STOPPING
BLOCKED
  ├─ condition fixed + user intent retained → STARTING
  └─ user stops → STOPPED
RECOVERING
  ├─ engine/db opened → ACTIVE or THROTTLED
  └─ unrecoverable → BLOCKED
STOPPING
  └─ links closed, scan/advertise stopped → STOPPED
```

서비스가 ACTIVE가 아니면 UI에서 이유를 명확히 표시한다.

## 2. Peer link

```text
DISCOVERED
  ├─ role arbitration win → CONNECTING
  └─ cooldown/no work → DEFERRED
CONNECTING
  ├─ opened → NEGOTIATING
  ├─ timeout → FAILED
  └─ duplicate → CLOSED
NEGOTIATING
  ├─ version ok → NOISE_HANDSHAKE
  └─ incompatible → CLOSING
NOISE_HANDSHAKE
  ├─ success → SECURE_SESSION
  └─ failure/timeout → CLOSING
SECURE_SESSION
  ├─ hello/slots done → INVENTORY
  └─ violation → CLOSING
INVENTORY
  ├─ requests exist → TRANSFERRING
  └─ no work → CLOSING
TRANSFERRING
  ├─ budget remains → INVENTORY
  ├─ complete/budget exhausted → CLOSING
  └─ transport error → FAILED
CLOSING → CLOSED
FAILED → cooldown → CLOSED
```

모든 state는 entry timestamp와 timeout을 가진다.

## 3. Inbound transfer

```text
OFFERED
  ├─ accepted → META_EXPECTED
  └─ rejected → TERMINAL
META_EXPECTED
  ├─ valid meta → RECEIVING
  └─ invalid → REJECTED
RECEIVING
  ├─ all chunks → VERIFYING
  ├─ timeout → PARTIAL
  └─ invalid chunk → REJECTED
VERIFYING
  ├─ hash/BP valid → COMMITTING
  └─ mismatch → REJECTED
COMMITTING
  ├─ DB commit → COMMITTED
  └─ quota/race duplicate → DUPLICATE/REJECTED
COMMITTED
  └─ send ACK → TERMINAL
PARTIAL
  ├─ resume within 10m → RECEIVING
  └─ expires → TERMINAL
```

ACK는 DB commit 이후에만 전송한다.

## 4. Outbound transfer

```text
AVAILABLE
  └─ peer requests → META_SENT
META_SENT
  ├─ credit → SENDING
  └─ reject/timeout → AVAILABLE
SENDING
  ├─ all chunks → COMMIT_SENT
  └─ disconnect → AVAILABLE
COMMIT_SENT
  ├─ ACK COMMITTED → FINALIZING
  ├─ ACK DUPLICATE → AVAILABLE(no token change)
  └─ timeout → AVAILABLE
FINALIZING
  ├─ token transaction success → AVAILABLE or WAIT_ONLY
  └─ DB failure → RECOVERY_REQUIRED
WAIT_ONLY
  └─ direct destination encountered → transfer allowed
```

`copy_tokens == 1`이면 WAIT_ONLY.

## 5. Outbound message

```text
DRAFT
  ├─ validation/encryption success → STORED_LOCAL
  └─ error → FAILED_LOCAL
STORED_LOCAL
  ├─ first relay commit → RELAYED
  ├─ receipt → RECEIPT_CONFIRMED
  ├─ user cancel → CANCEL_PROPAGATING
  └─ expiry → EXPIRED
RELAYED
  ├─ receipt → RECEIPT_CONFIRMED
  ├─ cancel → CANCEL_PROPAGATING
  └─ expiry → EXPIRED
CANCEL_PROPAGATING
  ├─ cancel receipt optional → CANCELED
  └─ cancel expiry → CANCELED_UNCONFIRMED
```

## 6. Contact trust

```text
IMPORTED_UNVERIFIED
  ├─ safety number compared → VERIFIED
  ├─ revoked → REVOKED
  └─ same identity new key → KEY_CHANGED
VERIFIED
  ├─ key update valid and user confirms → VERIFIED(new version)
  ├─ unexpected key → KEY_CHANGED
  └─ revoke → REVOKED
KEY_CHANGED
  ├─ in-person verify → VERIFIED
  └─ revoke → REVOKED
REVOKED
  └─ no automatic transition
```

P0 send to UNVERIFIED/KEY_CHANGED requires explicit blocking warning; default disallow.

## 7. Engine startup

```text
CLOSED → OPENING_DB → MIGRATING → LOADING_KEYS → RECOVERING_TRANSFERS → READY
```

- any key failure → `KEY_BLOCKED`
- newer DB version → `READ_ONLY_INCOMPATIBLE`
- corruption → `RECOVERY_MODE`
- READY 이전 transport event는 bounded queue에 보관하거나 adapter 시작을 지연한다.


---

<!-- SOURCE: docs/11-testing-and-acceptance.md -->

# 11. Testing and Acceptance

## 1. 테스트 피라미드

```text
Pure Rust unit/property tests      가장 많음
Rust DB integration tests
Protocol golden vectors
Android adapter unit tests
Android instrumentation tests
Physical-device BLE tests
Field exercises                   가장 적지만 출시 필수
```

## 2. Rust unit tests

### Codec

- deterministic CBOR 동일 입력 동일 bytes
- non-canonical integer/length 거부
- indefinite collection 거부
- text limit/UTF-8
- unknown major version

### Crypto

- contact card self-signature
- HPKE seal/open round trip
- wrong recipient key failure
- AAD field 한 개 변경 시 failure
- Ed25519 signature substitution
- ciphertext truncation
- sender key mismatch

### Routing

- token split conservation
- token 1 relay 금지
- direct destination always allowed
- hop boundary
- expired rejection
- priority score ordering
- deterministic tie break
- quota/eviction reserved floor

### State

- ACK 전 token 불변
- duplicate ACK idempotent
- process recovery transitions
- receipt/cancel idempotent
- conflict packet quarantine

## 3. Property tests

필수 property:

```text
sum(split_tokens(n)) == n
receiver_tokens >= 1 and sender_tokens >= 1
hop_count never decreases
age never decreases within a boot
expired bundle is never offered
receipt-confirmed bundle is never offered
encode(decode(valid)) == canonical(valid)
invalid input never panics
```

## 4. Fuzz targets

- BPv7 bundle parser
- private routing block parser
- DME ciphertext parser
- DME plaintext parser after synthetic decrypt
- contact card parser
- BLE pre-handshake frame
- encrypted frame plaintext parser
- chunk reassembler
- SQLite diagnostic import/export parser

각 target:

- CI smoke 60초
- nightly 30분
- release candidate campaign 누적 24시간 이상

## 5. Golden vectors

`test-vectors/`에 최소 다음을 둔다.

```text
contact-card-v1.json
contact-card-v1.cbor.hex
direct-text-plaintext.json
direct-text-plaintext.cbor.hex
direct-text-aad.hex
direct-text-hpke.enc.hex
direct-text-ciphertext.cbor.hex
direct-text-bpv7.hex
receipt-bpv7.hex
invalid/
```

random 입력은 vector generator에서 고정한다. production path에서 고정 random feature가 활성화되지 않도록 compile-time guard를 둔다.

## 6. Simulator scenarios

### SIM-001 Linear delayed path

- t0 A-B contact
- t10 A leaves
- t20 B-C contact
- expected: C receives, B cannot decrypt

### SIM-002 Network partition/rejoin

- 50 nodes, two partitions
- bridge nodes meet after 2h
- P0/P1/P2 delivery ratio 측정

### SIM-003 Churn

- 100 nodes
- 30% random shutdown
- 10% malicious drop
- compare tokens 4/8/12

### SIM-004 Flood attacker

- one peer sends max-size unique packets
- verify peer/day quota and P0 reserved floor

### SIM-005 Clock disorder

- wall clocks ±24h
- reboot events
- age monotonic and hard expiry 검증

## 7. Android unit tests

- permission state reducer
- service state reducer
- command executor idempotency
- queue overflow policy
- segment/reassembly
- notification redaction
- model mapping

## 8. Instrumentation

- Keystore wrap/unwrap
- process recreation
- DB migration
- foreground service start/stop
- Bluetooth off/on state
- permission revoke while active
- low storage callback
- location timeout/manual fallback

BLE 자체는 fake adapter와 physical-device suite를 분리한다.

## 9. Physical device matrix

최소 범주:

- Samsung API 26/28급 구형기
- Samsung 최신 API 36
- Google Pixel API 31/34/36
- Xiaomi/Redmi 계열
- OnePlus/Oppo 계열
- 저가형 3GB RAM 기기

각 기기 기록:

- advertise 지원
- simultaneous scan/advertise
- max negotiated MTU
- screen-off relay 1h/8h
- battery drain/h
- thermal behavior
- reboot recovery

## 10. End-to-end acceptance cases

### E2E-001 Direct text

- Wi-Fi/data/SIM off
- A/B Bluetooth on
- QR verified contacts
- message delivered + receipt returns

### E2E-002 Multi-hop

- A와 C는 동시에 범위 내에 있지 않음
- A→B commit
- A Bluetooth off/이탈
- B→C commit
- C decrypt, receipt 생성
- 역방향 접촉으로 A receipt 확인

### E2E-003 Relay confidentiality

B에서:

- DB copy
- logcat
- BLE packet capture

검사 결과 plaintext, exact location, sender display name 없음.

### E2E-004 Interrupted chunk

- 50% 전송에서 거리 이탈
- partial retained
- 재접촉 resume 또는 safe full retry
- corrupted duplicate commit 없음

### E2E-005 Storage pressure

- quota 16 MiB로 설정
- P2 flood
- own P0 생성
- P0 저장 성공, P2 eviction

### E2E-006 Permission loss

- active relay 중 Bluetooth permission revoke
- crash 없음
- service BLOCKED
- persistent/user-visible state

## 11. Release thresholds

- parser fuzz: crash/UB 0
- unit/integration: 100% pass
- critical state/routing modules line coverage 목표 90% 이상
- 3-device multi-hop: 50/50 성공(통제 환경)
- 8h screen-off: 중단 원인 미기록 silent failure 0
- corrupted DB recovery: user data overwrite 0
- P0 reserved quota invariant violations 0
- known critical/high security issue 0

전달 지연과 실제 재난 전달률은 환경 의존이므로 고정 SLA로 출시 조건을 표현하지 않는다.


---

<!-- SOURCE: docs/12-release-and-operations.md -->

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

필수 BLE/FGS/location 권한만 allowlist한다.

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


---

<!-- SOURCE: docs/13-development-goals.md -->

# 13. Development Goals

각 Goal은 이전 Goal의 결과 위에서만 시작한다. 동시에 여러 Goal을 진행하지 않는다.

## Goal 0 — Repository Bootstrap

### 결과물

- Rust workspace와 Android multi-module project
- CI skeleton
- version catalog/lockfiles
- protocol/spec 문서 복사
- `offlineRelease` manifest permission assertion
- architecture tests

### 작업

1. root Cargo workspace 생성
2. `mesh-types`, `mesh-codec`, `mesh-crypto`, `mesh-bundle`, `mesh-routing`, `mesh-engine`, `mesh-sim`, `mesh-ffi`
3. Android modules 생성
4. Rust Android targets build pipeline
5. single UniFFI facade AAR/native packaging
6. formatting/lint/test CI
7. offline manifest에 INTERNET 미포함 test

### 완료 조건

- clean checkout에서 Rust tests와 Android assemble 성공
- dummy Rust `version()`을 Android instrumentation에서 호출
- release manifest allowlist test 통과
- dependency versions가 lockfile에 고정

## Goal 1 — Protocol Core and Simulator

### 결과물

- ID/value types
- deterministic CBOR
- DM-BP7-1 bundle profile
- routing block parser/encoder
- SQLite v1 migration
- store-carry-forward simulator

### 작업

- CDDL 기반 validation
- bundle age/hop/count/token
- direct + binary spray-and-wait
- queue score/eviction
- receipt/cancel state model은 암호화 없이 synthetic payload로 먼저 구현

### 완료 조건

- A와 C가 동시에 연결되지 않는 SIM-001 통과
- 100-node deterministic simulation snapshot test
- invalid CBOR/BP 입력 panic 0
- token conservation property 통과

## Goal 2 — Identity, Contact and E2EE

### 결과물

- identity bootstrap
- contact card encode/decode/QR string
- safety number
- HPKE seal/open
- Ed25519 signature
- golden test vectors
- local key encryption

### 완료 조건

- 다른 구현/CLI에서 vector 검증 가능
- AAD 한 필드 변조 시 decrypt 실패
- wrong recipient/signature 실패
- private key/log redaction test
- receiver long-term key compromise limitation 문서와 UI 문구 존재

## Goal 3 — Android Direct BLE

### 결과물

- permissions/onboarding
- advertise/scan
- role arbitration
- GATT central/server
- frame segmentation
- Noise XX handshake
- encrypted session hello/slots/inventory
- direct bundle transfer

### 완료 조건

- data/Wi-Fi/SIM off 상태 두 Android 실기기 전송
- B packet capture에서 DME plaintext 없음
- interrupted transfer가 corrupt commit을 만들지 않음
- permission revoke/BT off crash 없음

## Goal 4 — Multi-hop Relay

### 결과물

- relay queue
- explicit inventory pages
- offer/request/meta/chunk/commit/ACK
- token transaction
- receipt/cancel
- partial resume
- quota/rate/eviction

### 완료 조건

- A→B, 분리, B→C 50회 반복 성공
- B는 decrypt 불가
- ACK 유실/중복 idempotent
- P2 flood에서 P0 reserved floor 유지
- receipt가 역방향 접촉으로 A에 도착

## Goal 5 — Disaster UX and Persistent Relay

### 결과물

- check-in/private SOS/location/cancel
- standby/emergency/fixed modes
- connectedDevice foreground service
- notification/state recovery
- battery/thermal policy
- relay diagnostics

### 완료 조건

- screen-off 8h 시험 보고서
- process kill/restart queue 복구
- battery <10%, thermal severe 정책 동작
- UI가 receipt 전 `전송 완료`를 표시하지 않음
- 위치 권한 없이 SOS 가능

## Goal 6 — Hardening and Public Beta

### 결과물

- fuzz targets
- SBOM/cargo deny/audit
- diagnostic export
- DB corruption recovery
- compatibility matrix
- threat model/security docs
- Play/F-Droid packaging

### 완료 조건

- release thresholds 전부 충족
- 외부 protocol/security review 중대 문제 해결
- known limitations와 safety wording 공개
- critical/high dependency advisory 0 또는 승인 예외

## Goal 7 — iOS and Fixed Relay

1.0 Android 출시 이후 진행.

- SwiftUI/Core Bluetooth adapter
- same Rust core/DB
- iOS behavior matrix
- Linux/Raspberry Pi relay
- field exercise tooling


---

<!-- SOURCE: docs/14-known-limitations.md -->

# 14. Known Limitations

## 사용자에게 반드시 공개할 항목

1. 주변에 앱을 실행하는 중계 기기가 없으면 전달되지 않는다.
2. 메시지는 수분·수시간·수일 후 전달되거나 영원히 전달되지 않을 수 있다.
3. Bluetooth가 꺼지거나 OS가 앱을 중단하면 중계가 멈춘다.
4. 악성 중계기는 메시지를 읽기 어렵지만 조용히 버릴 수 있다.
5. 목적지 slot, 메시지 크기, 우선순위, 접촉 시각 등 일부 메타데이터는 노출될 수 있다.
6. v1은 수신자 장기 키 유출 후 과거 암호문에 대한 완전한 forward secrecy를 보장하지 않는다.
7. 취소 메시지는 이미 읽은 내용을 상대 기기에서 강제 삭제하지 않는다.
8. GPS는 인터넷 없이 동작할 수 있지만 실내·지하·초기 위치에서 실패하거나 느릴 수 있다.
9. iOS 백그라운드 동작은 Android 고정 릴레이와 동일한 지속성을 보장하지 않는다.
10. 이 앱은 공식 긴급 신고·재난 문자·무전망을 대체하지 않는다.

## 엔지니어링 한계

- BLE GATT는 제조사별 차이가 크다.
- 다수 노드가 밀집하면 연결 시도와 광고 충돌이 증가한다.
- 장기 routing slot은 peer별 linkability를 남긴다.
- source EID를 메시지별 random으로 만들어도 traffic correlation은 가능하다.
- relay copy token은 전달률과 배터리/저장량 trade-off다.
- BPv7 constrained profile은 완전한 외부 BPA 상호운용을 목표로 하지 않는다.
- private block type 192는 본 프로젝트 내부 프로파일에 한정한다.

## 금지 마케팅 표현

- “어디서나 반드시 전달”
- “통신망이 없어도 실시간 통화”
- “완전 익명”
- “해킹 불가능”
- “공식 구조 요청 접수”
- “Signal과 동일한 보안”
- “배터리 영향 없음”


---

<!-- SOURCE: docs/15-references.md -->

# 15. Verified References

확인일: 2026-06-25

## DTN and wire formats

- RFC 9171 — Bundle Protocol Version 7  
  https://www.rfc-editor.org/rfc/rfc9171.html
- RFC 9172 — Bundle Protocol Security  
  https://www.rfc-editor.org/rfc/rfc9172.html
- RFC 9173 — Default Security Contexts for BPSec  
  https://www.rfc-editor.org/rfc/rfc9173.html
- RFC 9713 — BPv7 Administrative Record Types Registry update  
  https://www.rfc-editor.org/rfc/rfc9713.html
- RFC 9758 — Updates to the `ipn` URI Scheme  
  https://www.rfc-editor.org/rfc/rfc9758.html
- RFC 8949 — Concise Binary Object Representation; deterministic encoding requirements  
  https://www.rfc-editor.org/rfc/rfc8949.html
- RFC 8610 — Concise Data Definition Language  
  https://www.rfc-editor.org/rfc/rfc8610.html
- RFC 9180 — Hybrid Public Key Encryption  
  https://www.rfc-editor.org/rfc/rfc9180.html

## Link security

- The Noise Protocol Framework, Revision 34  
  https://noiseprotocol.org/noise.html

## Android

- Bluetooth permissions  
  https://developer.android.com/develop/connectivity/bluetooth/bt-permissions
- Communicate in the background with BLE  
  https://developer.android.com/develop/connectivity/bluetooth/ble/background
- Foreground service types required on Android 14+  
  https://developer.android.com/about/versions/14/changes/fgs-types-required
- Data transfer background options / connected device foreground service  
  https://developer.android.com/develop/background-work/background-tasks/data-transfer-options
- Android 16/API 36 behavior changes  
  https://developer.android.com/about/versions/16/behavior-changes-16
- BluetoothLeScanner PendingIntent scan API  
  https://developer.android.com/reference/android/bluetooth/le/BluetoothLeScanner
- Google Play target API requirements  
  https://developer.android.com/google/play/requirements/target-sdk

## Apple future implementation

- Core Bluetooth  
  https://developer.apple.com/documentation/corebluetooth
- Core Bluetooth Background Processing for iOS Apps  
  https://developer.apple.com/library/archive/documentation/NetworkingInternetWeb/Conceptual/CoreBluetooth_concepts/CoreBluetoothBackgroundProcessingForIOSApps/PerformingTasksWhileYourAppIsInTheBackground.html
- Configuring background execution modes  
  https://developer.apple.com/documentation/xcode/configuring-background-execution-modes

## Implementation references

- dtn7/bp7-rs primary repository  
  https://github.com/dtn7/bp7-rs
- Mozilla UniFFI primary repository  
  https://github.com/mozilla/uniffi-rs
- UniFFI user guide  
  https://mozilla.github.io/uniffi-rs/

## Current toolchain reference

- Android 16 is API level 36  
  https://developer.android.com/about/versions/16/behavior-changes-16
- Kotlin release process/current stable line  
  https://kotlinlang.org/docs/releases.html
- Android Gradle Plugin roadmap  
  https://developer.android.com/build/releases/gradle-plugin-roadmap

## Interpretation notes

- BPv7 defines bundle format and store-carry-forward behavior but leaves route selection and convergence-layer choice to the implementation.
- Android supports background BLE use cases, but process lifetime and foreground-service restrictions must be handled explicitly.
- iOS Core Bluetooth background modes do not imply unlimited always-on execution; behavior must be tested and documented by state.
- HPKE and Noise are building blocks. Using a conforming library does not replace a full protocol/security review.


---

<!-- SOURCE: docs/adr/ADR-001-android-first.md -->

# ADR-001: Android First

Status: Accepted

## Decision

1.0의 완전한 relay backbone은 Android만 대상으로 한다. iOS는 1.1에서 직접 통신과 best-effort background relay로 추가한다.

## Rationale

- Android는 connected-device foreground service와 사용자 가시적 지속 실행 모델을 제공한다.
- iOS background BLE는 상태·OS 정책에 따라 동작이 제한된다.
- 두 플랫폼을 동시에 시작하면 프로토콜 문제와 OS lifecycle 문제를 분리하기 어렵다.

## Consequence

- protocol/core는 iOS를 고려해 Rust로 공유한다.
- Android-specific API는 adapter 밖으로 새지 않게 한다.


---

<!-- SOURCE: docs/adr/ADR-002-rust-owns-protocol-db.md -->

# ADR-002: Rust Core Owns the Protocol Database

Status: Accepted

## Decision

Rust core가 SQLite protocol database를 직접 소유한다. Android Room은 protocol state에 사용하지 않는다.

## Rationale

- copy-token commit과 bundle commit의 원자성
- Android/iOS/Linux 간 동일 migration/logic
- FFI를 통한 세부 repository callback 제거
- simulator와 production state model의 일치

## Consequence

- UI query DTO를 FFI로 제공한다.
- DB master key는 Android Keystore가 unwrap한 뒤 Rust에 전달한다.
- Rust SQLite Android/iOS packaging을 CI에서 검증해야 한다.


---

<!-- SOURCE: docs/adr/ADR-003-bpv7-profile.md -->

# ADR-003: Constrained BPv7 Profile

Status: Accepted

## Decision

BPv7 전체 daemon을 앱에 넣지 않고 RFC 9171 wire format의 제한 프로파일 `DM-BP7-1`을 사용한다.

## Included

- primary block
- bundle age
- hop count
- payload
- private routing block type 192
- CRC32C

## Excluded

- fragmentation
- BP status reports
- custody transfer
- full BPSec in v1
- TCPCL

## Consequence

- 완전한 외부 BPA interoperability를 주장하지 않는다.
- bp7-rs 사용 여부와 관계없이 golden vectors/conformance tests를 유지한다.


---

<!-- SOURCE: docs/adr/ADR-004-message-security.md -->

# ADR-004: HPKE Payload Encryption plus Ed25519 Signature

Status: Accepted pending external review

## Decision

DME payload는 RFC 9180 HPKE Base(X25519/HKDF-SHA256/ChaCha20Poly1305)로 recipient에게 암호화하고, 암호문 내부 plaintext에 Ed25519 sender signature를 포함한다.

## Rationale

- 표준화된 hybrid encryption
- relay에게 sender identity를 숨김
- offline contact public key만으로 송신 가능
- cross-platform test vectors 작성 가능

## Limitation

recipient long-term key compromise 후 과거 ciphertext의 완전한 forward secrecy를 보장하지 않는다.


---

<!-- SOURCE: docs/adr/ADR-005-ble-gatt.md -->

# ADR-005: BLE GATT as Initial Convergence Layer

Status: Accepted

## Decision

양쪽 Android가 scan/advertise와 central/peripheral 역할을 수행하고, deterministic beacon arbitration으로 한 링크를 만든다. Control/Data characteristics와 Noise XX secure session을 사용한다.

## Consequence

- 기기별 GATT quirk와 MTU fallback 필요
- 대용량 파일 제외
- transport adapter로 격리해 향후 Wi-Fi Aware 등을 추가


---

<!-- SOURCE: docs/adr/ADR-006-spray-and-wait.md -->

# ADR-006: Binary Spray-and-Wait

Status: Accepted

## Decision

v1 relay routing은 Direct Delivery + Binary Spray-and-Wait로 고정한다.

## Rationale

- epidemic flooding보다 저장·무선 비용이 제한됨
- 중앙 topology 없이 구현 가능
- token invariant를 property-test할 수 있음

## Consequence

- 최적 경로를 보장하지 않음
- token/TTL 값은 simulator와 field exercise로 조정
