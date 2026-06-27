# 09. Android Implementation Specification

## 1. Baseline

```text
minSdk      26
compileSdk  37
targetSdk   36
JDK         17
Kotlin      bootstrap 시점의 stable line
AGP         current stable 9.x line selected at bootstrap and locked
UI          Jetpack Compose
```

빌드 도구의 patch version은 저장소 생성 시 공식 호환표에 맞춰 잠그고 Renovate/Dependabot 자동 merge를 금지한다.

Android 17/API 37이 2026-06-24 stable로 공개되었으므로 compile/test matrix에는
API 37을 포함한다. target SDK 36은 최초 beta의 의도적인 N-1 기준이며, Play 제출 전
당시 target API 정책과 Android 17 behavior change 검증 결과를 보고 37 전환 여부를
별도 결정한다.

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

    <uses-permission
        android:name="android.permission.BLUETOOTH_SCAN"
        android:usesPermissionFlags="neverForLocation" />
    <uses-permission android:name="android.permission.BLUETOOTH_CONNECT" />
    <uses-permission android:name="android.permission.BLUETOOTH_ADVERTISE" />

    <uses-permission android:name="android.permission.FOREGROUND_SERVICE" />
    <uses-permission android:name="android.permission.FOREGROUND_SERVICE_CONNECTED_DEVICE" />
    <uses-permission android:name="android.permission.POST_NOTIFICATIONS" />
    <uses-permission android:name="android.permission.RECEIVE_BOOT_COMPLETED" />

    <!-- API 26-30: BLE scan permission prerequisite. API 31+: request only for GPS attachment. -->
    <uses-permission android:name="android.permission.ACCESS_FINE_LOCATION" />

    <application
        android:allowBackup="false"
        android:fullBackupContent="false"
        android:dataExtractionRules="@xml/data_extraction_rules"
        ...>
        <service
            android:name=".service.EmergencyRelayService"
            android:exported="false"
            android:foregroundServiceType="connectedDevice" />
        <receiver
            android:name=".service.RelayBootReceiver"
            android:enabled="true"
            android:exported="false">
            <intent-filter>
                <action android:name="android.intent.action.BOOT_COMPLETED" />
            </intent-filter>
        </receiver>
    </application>
</manifest>
```

`data_extraction_rules.xml`은 cloud-backup과 device-transfer에서 root domain 전체를
exclude한다. wrapped master key, encrypted private keys, contacts, message metadata와
relay DB가 OS/OEM backup으로 외부 복제되지 않게 한다.

```xml
<data-extraction-rules>
    <cloud-backup>
        <exclude domain="root" path="." />
    </cloud-backup>
    <device-transfer>
        <exclude domain="root" path="." />
    </device-transfer>
</data-extraction-rules>
```

`neverForLocation` 사용이 주변 BLE 결과를 제한하는 기기/OS 사례가 있는지 실기기에서 확인한다.

- API 26~30에서는 BLE scan 자체가 `ACCESS_FINE_LOCATION` runtime permission을
  요구하므로 legacy OS 제약을 명확히 설명한다.
- API 31 이상에서는 BLE 권한과 GPS 위치 첨부 권한을 별도 흐름으로 요청한다.
- 모든 API에서 위치 권한 없이 SOS 작성과 manual location 입력은 가능하다.
- API 26~30에서 위치 권한을 거부하면 SOS는 로컬 저장할 수 있지만 BLE 전송은
  `통신 기능 중지됨` 상태다.

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

### API 29–30 background BLE policy

Android 10–11에서 background scan이 location으로 간주되는 조합은 `ACCESS_BACKGROUND_LOCATION`이 필요할 수 있다. 상용 1.0은 다음 두 variant를 명확히 분리한다.

- Play `offlineRelease`: background location 승인 없이 정책상 가능한 foreground connected-device relay만 제공하고, API 29–30에서 OS가 scan을 중단하면 UI에 제한을 표시한다.
- 별도 기관/현장 배포 `fieldTestRelease`: 법률·스토어 정책 검토와 명시적 사용자 교육을 거쳐 background location을 선언할 수 있다.

`neverForLocation`은 일부 beacon 결과를 필터링할 수 있으므로 필수 device matrix에서 service UUID 광고 발견률을 측정한다. permission이 없을 때 기능을 몰래 degrade하지 않고 상태와 해결 방법을 표시한다.

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
notification은 `VISIBILITY_PRIVATE`로 만들고 lock-screen public version은
`재난 통신 중계 동작 중`만 표시한다. peer/bundle count는 잠금 해제 후에만 보인다.

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

GATT characteristic operation당 Android가 허용한 payload 크기로 outer segment한다. 구현은 임의 레이아웃을 만들지 않고 `spec/ble-wire-v1.md`의 16-byte header와 big-endian 규칙을 그대로 사용한다.

- logical frame 최대 64 KiB
- segment count 최대 1024
- reassembly timeout 10초
- frame ID는 link 안에서 재사용하지 않음
- duplicate segment는 bytes가 같을 때만 idempotent 허용
- gap/out-of-order는 bitmap으로 수용하되 conflicting segment, length mismatch, reserved bit는 link 종료

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
                when (val accepted = adapter.enqueue(command)) {
                    is CommandEnqueueResult.Accepted -> Unit
                    is CommandEnqueueResult.Rejected -> events.send(
                        CoordinatorEvent.Transport(accepted.toFailureEvent(command.commandId))
                    )
                }
            }
        }
    }
}
```

- dispatcher는 `Dispatchers.Default.limitedParallelism(1)` 또는 dedicated thread
- `adapter.execute`는 GATT operation 완료를 기다리지 않고 enqueue/즉시 실패만 반환
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
receipt 도착 시각은 `내 기기가 확인을 받은 시각`으로만 표시하고 상대 기기의 실제
수신 시각이라고 표현하지 않는다.

### SOS

- category
- people/severe injury count
- description
- optional location/manual location
- recipients 1~16명; 수신자별 별도 암호화/전송 상태 표시
- long-press 1.5초 send
- TalkBack/스위치 접근에서는 long-press를 유일한 경로로 쓰지 않고
  `구조 요청 보내기` accessibility action + 짧은 확인 sheet를 제공
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

`RelayBootReceiver`는 user-unlocked `BOOT_COMPLETED`에서 저장된 사용자 intent만
확인한다. 현재 OS의 foreground-service start 조건과 Bluetooth permission이
충족될 때만 connected-device FGS를 시작한다. 불가능하면 자동 우회하지 않고
`중계 재개 필요` 상태/notification을 남긴다.

## 12. Manufacturer quirks

`AndroidBleQuirkRegistry`는 모델별 hardcode보다 capability/실패 기반 fallback 우선.

- 연결당 MTU 요청 1회. Android 14+ 후속 요청 무시 동작을 고려해 협상 callback 또는 기본 ATT payload를 사용하고 다단계 재요청 fallback 금지
- write without response stall → write with response fallback
- advertise unsupported → scan-only node; UI 표시
- simultaneous scan/advertise failure → time slicing
- GATT 133 계열 → close, refresh 금지(private API), jitter retry

## 13. Source references

- Android Bluetooth permissions
- Android BLE background communication
- Android foreground service connected-device requirements
- Android 16/API 36 및 Android 17/API 37 Bluetooth/background behavior changes

정확한 URL은 `docs/15-references.md`에 기록한다.
