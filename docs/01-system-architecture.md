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
- routing/scheduling은 deterministic하고, ID/crypto randomness는 명시적
  `EntropySource` dependency를 통해서만 소비

### BLE Transport

- 광고·스캔·GATT 연결
- 링크 framing, MTU, chunk write/notify
- Noise handshake byte transport
- bundle 의미를 해석하지 않음

### Rust Protocol Storage

- Rust core가 bundle, payload, contact, receipt, tombstone SQLite를 직접 소유한다.
- token grant escrow, transfer commit, sender available-token 변경을 transaction으로 처리한다.
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

### Clock and entropy

- production Rust core는 OS CSPRNG adapter를 사용한다.
- simulator/vector tests는 seed가 기록된 deterministic entropy adapter를 주입한다.
- test deterministic entropy feature는 release build에서 compile-time 거부한다.
- wall/elapsed time은 SystemEvent 또는 open boot context로 명시적으로 전달한다.

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
9. relay transfer 전 token grant escrow 생성, commit ACK 후 grant 확정
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
- relay copy의 receiver 몫은 BUNDLE_META 전에 persistent grant escrow로 예약한다.
- ACK를 잃으면 grant를 `UNCERTAIN`으로 유지하고 다른 peer에게 재사용하지 않는다.
- 같은 peer와 같은 grant ID로 reconciliation하여 receiver ledger의 commit 여부를 확인한다.
- receiver와 sender 양쪽에 copy가 존재하는 것은 허용하지만 token grant 총량은 증가하지 않는다.
- DB corruption 감지 시 원본 파일을 보존하고 읽기 전용 복구/export 화면으로 진입한다.

## 10. 확장 지점

`TransportAdapter` 인터페이스를 유지해 다음을 추가한다.

- Wi-Fi Aware
- local Wi-Fi socket
- Linux BLE relay
- LoRa gateway

상위 DME, routing, storage는 transport 종류를 알지 않는다.
