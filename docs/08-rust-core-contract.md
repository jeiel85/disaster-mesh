# 08. Rust Core and FFI Contract

## 1. 설계 결정

- Rust core가 protocol DB와 모든 protocol state를 소유한다.
- Android는 한 개의 serial actor에서만 `MeshEngine`을 호출한다.
- FFI facade crate는 하나만 배포한다. 여러 UniFFI library를 앱에 중복 포함하지 않는다.
- BLE platform은 raw byte transport다. Noise/frame/session state는 Rust가 소유한다.
- clock과 entropy는 core 내부의 명시적 adapter boundary이며 테스트에서 대체 가능하다.

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
    pub fn import_contact_card(&self, encoded: Vec<u8>) -> Result<ContactSummary, CoreError>;
    pub fn verify_contact_in_person(
        &self,
        contact_id: Id16,
        displayed_safety_number: String,
    ) -> Result<ContactSummary, CoreError>;

    pub fn create_direct_text(&self, draft: DirectTextDraft) -> Result<CreateMessageResult, CoreError>;
    pub fn create_check_in(&self, draft: CheckInDraft) -> Result<CreateMessageBatchResult, CoreError>;
    pub fn create_private_sos(&self, draft: PrivateSosDraft) -> Result<CreateMessageBatchResult, CoreError>;
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
diagnostic export는 최대 4 MiB이며 raw DB, bundle bytes, partial/quarantine files,
contact keys/slots와 plaintext를 포함하지 않는다. 한도를 넘으면 truncation manifest를
포함하고 오래된 redacted event부터 생략한다.

`CheckInDraft`와 `PrivateSosDraft`는 1~16개의 `recipient_contact_ids`를 받는다.
core는 수신자마다 별도 message/packet/HPKE bundle을 생성하고 하나의
`send_group_id`로 묶어 반환한다. 한 수신자라도 validation/encryption/persist에
실패하면 전체 batch를 rollback한다.

contact import는 항상 `UNVERIFIED`로 저장한다. `verify_contact_in_person`은 core가
다시 계산한 `XXXX-XXXX-XXXX` 안전번호와 전달된 표시값이 정확히 일치할 때만
`VERIFIED_IN_PERSON`으로 전환한다. import API에 verification boolean을 두지 않는다.

## 4. TransportEvent

```rust
pub enum TransportEvent {
    PeerDiscovered {
        peer_handle: String,
        beacon_id: Option<[u8; 8]>,
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
    CommandCompleted { command_id: u64, link_id: Option<u64>, result: CommandCompletion },
    CommandFailed { command_id: u64, link_id: Option<u64>, category: TransportFailure },
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
    StartScan { command_id: u64, policy: ScanPolicy },
    StopScan { command_id: u64 },
    StartAdvertising { command_id: u64, data: AdvertisementData },
    UpdateAdvertising { command_id: u64, data: AdvertisementData },
    StopAdvertising { command_id: u64 },
    Connect { command_id: u64, peer_handle: String },
    Disconnect { command_id: u64, link_id: u64, reason: LinkCloseReason },
    RequestMtu { command_id: u64, link_id: u64, requested: u16 },
    ConfigureNotifications { command_id: u64, link_id: u64, channel: LinkChannel, enabled: bool },
    SendBytes { command_id: u64, link_id: u64, channel: LinkChannel, bytes: Vec<u8>, with_response: bool },
    RequestConnectionPriority { command_id: u64, link_id: u64, high: bool },
    RequestLocation { command_id: u64, request_id: Id16, timeout_seconds: u32 },
    ShowIncomingMessage { command_id: u64, message_id: Id16 },
    UpdatePersistentNotification { command_id: u64, model: NotificationModel },
    ShowLocalAlert { command_id: u64, model: LocalAlert },
    ScheduleWake { command_id: u64, after_ms: u64, reason: WakeReason },
}
```

모든 비동기 platform command는 session 내 단조 증가 `command_id`를 가진다. platform은 command 접수 여부와 실제 완료를 구분한다.

- `enqueue(command)`는 큐 삽입 또는 즉시 거부만 반환한다.
- 실제 GATT/scan/advertise 완료는 동일 `command_id`의 `CommandCompleted` 또는 `CommandFailed`로 돌아온다.
- link별 GATT operation은 최대 1개만 in-flight이며 callback이 일치하지 않으면 protocol violation으로 link를 닫는다.
- process restart 후 이전 command ID는 재사용하지 않으며 모든 link command는 실패로 복구한다.
- core는 completion event 없이 write 성공을 추측하거나 credit를 차감 확정하지 않는다.

## 7. Command execution order

하나의 return batch는 배열 순서대로 실행한다.

- `SendBytes`/MTU/notification command 실패 시 이후 같은 link의 queued GATT command를 중단하고 각 command에 실패 event를 반환한 뒤 LinkClosed event를 보낸다.
- notification 실패는 transport command를 중단하지 않는다.
- Start/Stop scan/advertise는 idempotent adapter가 처리하고 같은 command_id를 두 번 완료하지 않는다.

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
