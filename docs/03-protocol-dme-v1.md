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
| creation sequence | CSPRNG u64 encoded as 8-byte big-endian DB value; 같은 source EID에서 중복 금지 |
| lifetime | milliseconds, message policy 범위 내 |
| fragment fields | 없음 |

메시지별 source EID를 사용해 안정적인 발신자 EID 노출을 줄인다. 실제 발신자 identity는 암호문 내부에 있다.

EID의 16-byte 값은 RFC 4648 Base32 lowercase, padding 없이 26글자로 encode한다.
decoder는 uppercase, padding, 다른 길이와 non-canonical trailing bits를 거부한다.

### 3.2 Canonical Blocks

정해진 순서:

1. Bundle Age Block: type 7, block number 2, CRC32C
2. Hop Count Block: type 10, block number 3, CRC32C
3. Disaster Routing Block: private type 192, block number 4, CRC32C
4. Payload Block: type 1, block number 1, CRC32C

모든 노드는 type 192를 이해해야 한다. 해당 block의 processing flag는 `delete bundle if block cannot be processed`를 설정한다.

BPv7 전체 bundle은 RFC 9171에 따라 CBOR indefinite-length array로 encode하고
Payload Block 뒤의 break stop code로 끝낸다. 각 primary/canonical block과
block-type-specific DME 구조는 definite-length canonical encoding을 사용한다.
Payload Block의 block number는 순서와 무관하게 항상 1이다.

### 3.3 Disaster Routing Block v1

릴레이가 읽는 최소 메타데이터다.

```text
[
  version: 1,
  packet_id: bstr .size 16,
  message_class_hint: uint .ge 1 .le 5,
  priority: uint .le 3,
  copy_tokens: uint .ge 1 .le 16,
  payload_size: uint .ge 1 .le 8192,
  payload_sha256: bstr .size 32
]
```

- `packet_id`는 애플리케이션 dedup 키다.
- `copy_tokens`만 복제 과정에서 변경 가능하다.
- priority와 class hint는 메타데이터로 노출된다.
- relay가 priority, class hint 또는 다른 AAD 필드를 변경하면 수신자의 AAD hash
  검증/HPKE open이 실패한다. relay가 이 필드를 합법적으로 변경할 수 없다.

class hint mapping:

- 1 DIRECT: DIRECT_TEXT
- 2 CHECK_IN: CHECK_IN, LOCATION_UPDATE
- 3 SOS: PRIVATE_SOS
- 4 RECEIPT: DELIVERY_RECEIPT
- 5 CANCEL: CANCEL

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

크기 정의:

- BP Payload Block data, 즉 encoded `DmeCiphertext`: 최대 8,192 bytes
- 그 안의 HPKE `ciphertext` bstr: 최대 8,118 bytes
- 최종 encoded `DmeCiphertext`가 8,192 bytes를 넘으면 메시지 생성을 거부한다.

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
| message_type | uint | 1..6; code 7은 v1에서 reserved/reject |
| packet_id | 16-byte bstr | routing block과 같아야 함 |
| message_id | 16-byte bstr | sender가 생성 |
| conversation_id | 16-byte bstr | contact별 대화 |
| sender signing pub | 32-byte bstr | Ed25519 |
| sender HPKE pub | 32-byte bstr | X25519 |
| recipient hash | 32-byte bstr | SHA-256(recipient signing pub) |
| sender_sequence | uint64 | sender-recipient contact 관계별 단조 증가; v1 허용 범위 0..2^63-1 |
| reply slot | 16-byte bstr | sender가 소유한 현재 inbound slot; receipt 목적지 |
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
| 7 | KEY_UPDATE reserved; v1 decoder MUST reject |

### DIRECT_TEXT body

```text
[ text: tstr, reply_to: null / bstr .size 16 ]
```

- 사용자 기준 최대 2,000 Unicode scalar values
- UTF-8 encoded text 최대 7,800 bytes
- 모든 wrapper/서명/HPKE overhead를 포함한 최종 DME payload 크기 제한을 추가로 적용

### CHECK_IN body

```text
[
  status: 1..5,
  people_count: 1..99,
  note: tstr .size (0..2000),
  location: null / Location,
  battery_percent: null / 0..100
]
```

### PRIVATE_SOS body

```text
[
  category: 1..6,
  description: tstr .size (1..3200),
  people_count: 1..99,
  severe_injury_count: 0..99,
  location: null / Location,
  movement_direction: tstr .size (0..400),
  battery_percent: null / 0..100
]
```

`severe_injury_count <= people_count`를 semantic validation으로 강제한다.
CHECK_IN note, SOS description/movement는 각각 500/800/100 Unicode scalar limit도
별도로 검증한다. CDDL `.size` 값은 UTF-8 byte upper bound다.

### Location

GPS 위치:

```text
[
  kind: 1,
  latitude_e7: int,
  longitude_e7: int,
  accuracy_meters: uint .le 50000,
  altitude_meters: null / int,
  captured_before_send_ms: uint .le 86400000,
  note: tstr .size (0..800)
]
```

수동 위치:

```text
[
  kind: 2,
  description: tstr .size (1..800)
]
```

- 위도 범위 `-900000000..900000000`
- 경도 범위 `-1800000000..1800000000`
- 정확도 50 km 초과 GPS fix는 첨부하지 않고 수동 위치 입력으로 전환한다.
- location note/description은 최대 200 Unicode scalar values와 UTF-8 800 bytes다.
- `captured_before_send_ms`는 위치 capture부터 DME 생성까지 같은 boot의 monotonic
  차이이며 최대 24시간이다. 다른 기기에서 의미 없는 raw elapsedRealtime 값은 보내지 않는다.

### DELIVERY_RECEIPT body

```text
[
  original_packet_id: bstr .size 16,
  original_message_id: bstr .size 16,
  receipt_status: 1,  # delivered
  receiver_note: null / tstr .size (0..400)
]
```

receiver의 `elapsedRealtime`은 다른 기기에서 의미가 없으므로 wire에 넣지 않는다.
발신 UI는 receipt를 로컬에서 수신한 시각만 표시하며 실제 목적지 도착 시각으로
오인하지 않는다.
receiver note는 최대 100 Unicode scalar values와 UTF-8 400 bytes다.

receipt signer identity는 원본 메시지의 recipient identity와 같아야 한다.

### CANCEL body

```text
[
  target_packet_id: bstr .size 16,
  target_message_id: bstr .size 16,
  cancel_reason: 1..4
]
```

취소는 이미 수신자가 읽은 정보를 원격 삭제한다고 보장하지 않는다. v1 relay는
암호문 안의 target ID와 sender identity를 볼 수 없으므로 cancel을 일반 P0 bundle로
운반할 뿐 원본 relay copy를 삭제하지 않는다. 최종 수신 endpoint만 cancel을
복호화·검증하고 UI를 `취소됨`으로 바꾼다. 발신 endpoint는 cancel 생성 즉시 자신의
원본 offer를 중단한다.

cancel signer identity는 원본 메시지의 sender identity와 같아야 한다.

## 9. Sender sequence/replay

- sender가 해당 recipient contact로 보내는 `sender_sequence`는 message 생성
  transaction에서 1 증가한다.
- 해당 contact의 sender sequence가 `2^63-1`에 도달하면 자동 wrap하지 않고
  그 contact 송신을 차단한 뒤 key/contact rotation 정책 결정을 요구한다.
- 수신자는 contact별 최대 sequence와 최근 256개 message ID window를 저장한다.
- 낮은 sequence라도 처음 보는 packet은 DTN 재정렬 때문에 즉시 거부하지 않는다.
- 동일 message ID/packet ID는 중복 거부한다.
- 최대 sequence보다 4096 이상 뒤처진 새 message는 `STALE_REPLAY_SUSPECTED`로 격리한다.

## 10. 생성 절차

```text
validate user input
load contact and identity
allocate recipient-contact sender_sequence in transaction
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
- encoded DME payload 최대 8 KiB
- HPKE ciphertext bstr 최대 8,118 bytes
- CBOR nesting 최대 12
- collection 항목 최대 32
- text는 valid UTF-8만
- BPv7 outer bundle array만 RFC 9171 요구에 따라 indefinite-length 허용
- DME, routing block, BLE control과 각 BP block 내부에는 indefinite-length item 금지
- duplicate map key 금지; v1은 가능한 한 array schema 사용
- unknown major version 거부
- unknown minor field는 v1 array 구조에서 허용하지 않음

## 12. 버전 정책

- protocol major는 광고와 HELLO에 포함
- major가 다르면 연결 후 `UNSUPPORTED_VERSION`으로 종료
- minor 기능은 capability bit로 협상
- DME v1 decoder는 v2 payload를 추측해 읽지 않는다.
- wire 변경 시 golden vector를 추가한다.

Contact Card의 exact QR framing, signature domain separation과 safety number 표시는
`docs/06-security-and-threat-model.md`를 따른다.
