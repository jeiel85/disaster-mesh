# 02. Domain Model and Invariants

## 1. 기본 ID

| 타입 | 형식 | 생성 |
|---|---|---|
| `IdentityId` | SHA-256(Ed25519 pub), 32 bytes | 설치 시 |
| `DisplayId` | identity hash 앞 10 bytes Base32 + checksum | 표시용 |
| `ContactId` | UUIDv7, 16 bytes | 연락처 저장 시 |
| `ConversationId` | random 16 bytes | 최초 대화 시 |
| `MessageId` | UUIDv7 또는 CSPRNG 16 bytes | 메시지 생성 시 |
| `PacketId` | CSPRNG 16 bytes | DME 생성 시; 앱 dedup 기준 |
| `BpIdentityHash` | SHA-256(BPv7 source EID || creation timestamp tuple), 32 bytes | BP 생성 후 |
| `WireBundleHash` | SHA-256(exact transmitted BP bytes), 32 bytes | 전송 variant마다 |
| `RoutingSlot` | CSPRNG 16 bytes | 자신의 수신 slot 발급 시 |
| `PeerSessionId` | CSPRNG 16 bytes | 링크 연결 시 |
| `TransferId` | CSPRNG 16 bytes | bundle 전송 시 |
| `TokenGrantId` | CSPRNG 16 bytes | relay token escrow 생성 시 |

UUIDv7 구현 의존성을 피하고 싶으면 16-byte CSPRNG ID로 통일해도 된다. 프로토콜 wire에서 시간 정렬을 요구하지 않는다.

`WireBundleHash`는 age, hop count, copy token이 변경될 때 달라질 수 있으므로
dedup identity로 사용하지 않는다. 앱 dedup은 `PacketId`, BP identity 충돌 검사는
`BpIdentityHash`를 사용한다.

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
  destination_routing_slot
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

`destination_routing_slot`은 연락처 카드 소유자가 수신을 위해 발급한 slot이다.
내 기기가 소유한 slot은 Contact가 아니라 `InboundRoutingSlot` aggregate로 관리한다.

### InboundRoutingSlot

```text
InboundRoutingSlot {
  routing_slot
  state             # ACTIVE / GRACE / RETIRED
  key_version
  created_at
  retire_after?
}
```

불변식:

- Contact Card에는 `ACTIVE` slot 하나만 포함한다.
- BLE `ROUTING_SLOTS`에는 내 기기가 소유한 `ACTIVE`와 유효한 `GRACE` slot만 보낸다.
- 다른 연락처의 destination slot을 peer에게 공개하지 않는다.
- v1.0은 rotation UI를 제공하지 않지만 저장 모델은 향후 grace rotation을 허용한다.

### Bundle

```text
Bundle {
  packet_id
  bp_identity_hash
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
  packet_id
  send_group_id?
  delivery_state
  created_at_local
  received_at_local?
}
```

CHECK_IN과 PRIVATE_SOS에서 여러 수신자를 선택하면 한 사용자 동작을
`SendGroup`으로 묶되, 수신자마다 별도의 `MessageId`, `PacketId`, HPKE ciphertext,
BP bundle을 만든다. 하나의 ciphertext를 여러 수신자에게 재사용하지 않는다.

`delivery_state`의 numeric value는 `contracts/state_codes.toml`이 유일한 기준이다.

- outbound: `OUTBOUND_STORED`, `OUTBOUND_RELAYED`, `OUTBOUND_RECEIPT_CONFIRMED`, `OUTBOUND_CANCEL_PROPAGATING`, `OUTBOUND_CANCELED_CONFIRMED`, `OUTBOUND_CANCELED_UNCONFIRMED`, `OUTBOUND_EXPIRED`, `FAILED_LOCAL`
- inbound: `INBOUND_RECEIVED`, `INBOUND_CANCELED`
- security/error: `QUARANTINED`

발신자는 receipt를 검증하기 전 목적지 수신을 추정하지 않는다. UI의 “전달됨”은 오직 `OUTBOUND_RECEIPT_CONFIRMED`에만 사용한다.

## 4. 상태 불변식

1. 같은 `PacketId`는 payload hash가 동일해야 한다. 다르면 충돌/공격으로 격리한다.
2. `COMMITTED` bundle만 라우팅 offer 대상이다.
3. 만료 bundle은 최종 수신자에게도 전달하지 않는다.
4. 최종 발신 endpoint가 receipt를 복호화·검증하면 자신의 원본 bundle을 더 이상 offer하지 않는다.
5. cancel은 최종 수신 endpoint만 복호화·검증한다. relay는 cancel을 운반하지만 대상 원본을 해석하거나 삭제하지 않는다.
6. relay 노드는 DME plaintext를 저장하지 않는다.
7. 최종 수신자 판정은 routing slot match 후에도 HPKE open과 recipient key hash 검증까지 완료해야 한다.
8. relay token grant 생성 시 sender의 available token을 같은 DB transaction에서 escrow로 이동한다.
9. uncertain grant는 ACK 유실 후 다른 peer에게 재사용하지 않는다.
10. priority는 relay가 상향 조정할 수 없다.
11. lifetime, hop limit, source route, destination slot은 immutable header로 취급하고 DME AAD에 인증한다.
12. `DELIVERY_RECEIPT`는 어떤 경우에도 다시 receipt를 생성하지 않는다.
13. `CANCEL`은 원본보다 먼저 도착할 수 있으며 검증된 sender/target을 pending control로 보존한다.
14. replay 수락 여부는 contact별 4096-bit persisted sliding window로 판정한다.
15. 상태·오류의 numeric code는 machine-readable contract와 DB CHECK를 거치지 않고 추가하지 않는다.

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
