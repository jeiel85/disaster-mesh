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

legacy advertising의 31-byte 제한을 넘지 않도록 primary advertisement에는
128-bit Service Data AD structure 하나만 넣는다. device name과 TX power는 넣지 않는다.
Flags 3 bytes + Service Data AD 28 bytes가 되어 정확히 31 bytes다.

Service Data 10 bytes:

| offset | size | field |
|---:|---:|---|
| 0 | 1 | protocol major = 1 |
| 1 | 1 | state bits |
| 2 | 8 | ephemeral beacon ID |

State bits:

- bit0: GATT server available
- bit1: relay enabled
- bits 2..3: mode (`00 standby`, `01 emergency`, `10 fixed`, `11 reserved`)
- bits 4..5: queue load (`00 empty`, `01 low`, `10 medium`, `11 high`)
- bits 6..7: reserved

Noise, inventory paging, resume bitmap 같은 상세 capability는 VERSION_HELLO에서
협상한다. Service Data가 OEM 문제로 실패하면 service UUID only 광고로 fallback하고,
beacon ID를 받지 못한 peer에는 random role fallback만 사용한다.

금지 정보:

- identity ID
- stable node ID
- contact slot
- location
- exact bundle count

`ephemeral beacon ID`는 CSPRNG 8 bytes이며 10분 ±2분 jitter마다 회전한다. 활성 링크 동안은 회전해도 기존 session을 끊지 않는다.
byte-exact advertisement size와 `ADVERTISE_FAILED_DATA_TOO_LARGE` fallback을 Android
unit/instrumentation test로 검증한다.

## 4. Central/Peripheral 역할 중복 방지

두 기기가 서로 발견하면 다음 규칙을 적용한다.

1. local beacon ID와 remote beacon ID를 unsigned lexicographic 비교
2. 더 작은 beacon ID를 가진 노드가 central 연결을 시도
3. 5초 내 연결이 없고 양쪽 모두 다시 광고되면 0~1500ms random fallback으로 어느 쪽이든 시도 가능
4. 이미 같은 remote session fingerprint와 연결 중이면 두 번째 링크 거부

remote beacon ID가 service-UUID-only fallback 때문에 없거나 beacon ID가 충돌하면
Bluetooth address/OS handle을 안정 ID로 사용하지 말고 random fallback한다.

## 5. GATT 초기화

연결 후:

- Android central은 연결당 MTU 요청을 최대 1회 수행한다. Android 14+에서는 첫 요청이 517로 승격되고 같은 ACL 연결의 후속 요청이 무시될 수 있으므로 517→247 반복 요청 fallback을 사용하지 않는다.
- `onMtuChanged` 또는 기본 ATT MTU로 확정된 실제 application payload를 사용한다.
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
- remote max 값은 CDDL 범위와 local hard cap을 먼저 검증한 뒤 min으로 협상한다.
- `chunk_count == ceil(total_size / chunk_size)`가 아니면 BUNDLE_META를 거부한다.

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

Noise ciphertext가 GATT 한도보다 크면 outer BLE segment로 나눈다. segment는 link-local이며 encrypted frame sequence와 별도다. byte order, segment header, frame ID, duplicate/gap 처리의 normative 정의는 `spec/ble-wire-v1.md`를 따른다.

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
| 0x1D | RESUME_QUERY |
| 0x1E | RESUME_STATE |

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
- 내 기기가 소유한 ACTIVE slot과 아직 유효한 GRACE slot만 보낸다.
- v1.0 기본은 ACTIVE slot 하나다.
- 연락처에서 가져온 destination slot은 절대 이 frame에 포함하지 않는다.
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
  token_grant_id16_or_null,
  packet_id16,
  total_size,
  sha256,
  chunk_size,
  chunk_count,
  proposed_receiver_tokens,
  sender_remaining_tokens_after_reservation
]
```

- 직접 최종 전달이면 `token_grant_id = null`, `proposed_receiver_tokens = 1`,
  `sender_remaining_tokens_after_reservation = sender current available tokens`이며
  token escrow를 만들지 않는다. 최종 수신 copy는 relay offer 대상이 아니다.
- relay copy이면 sender는 BUNDLE_META 전에 persistent token grant escrow를 생성한다.
- 같은 grant의 retry/status reconciliation에는 같은 `token_grant_id`를 재사용한다.
- relay 전송 bytes의 Disaster Routing Block `copy_tokens`는
  `proposed_receiver_tokens`와 같아야 한다.
- sender의 local bundle variant는 `sender_remaining_tokens_after_reservation` 값을 사용한다.
- BUNDLE_META `sha256`은 mutable age/token을 반영한 이번 전송의 exact BP bytes hash다.

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
[ transfer_id16, packet_id16, status, accepted_tokens, committed_payload_sha256 ]
```

status:

- 1 COMMITTED
- 2 COMMITTED_SAME_GRANT
- 3 DUPLICATE_OTHER_GRANT
- 4 REJECTED_INVALID
- 5 REJECTED_QUOTA
- 6 REJECTED_EXPIRED
- 7 RETRY_LATER

`accepted_tokens`는 COMMITTED/COMMITTED_SAME_GRANT에서 persisted routing-block
token 값이며 그 외에는 0이다. `committed_payload_sha256`은 commit 계열에서
immutable DME payload hash를 넣고 그 외에는 null이다. exact transfer bytes는
BUNDLE_COMMIT의 `total_sha256`으로 이미 검증하며, receiver는 commit 과정에서
hop/age local variant를 다시 encode할 수 있다.

relay copy token은 BUNDLE_META 전 sender DB transaction에서 available token에서
persistent grant escrow로 이동한다. `COMMITTED` 또는 `COMMITTED_SAME_GRANT` ACK를
받으면 grant를 `TRANSFERRED`로 확정한다. commit 이후 ACK가 유실되면 grant는
`UNCERTAIN`으로 남고 다른 peer에게 재사용하지 않는다. 같은 peer와 재접촉했을 때
같은 grant ID로 meta/commit을 재시도하면 receiver는 stored grant ledger를 조회해
`COMMITTED_SAME_GRANT`를 반환한다.

명시적 reject나 `DUPLICATE_OTHER_GRANT`에서 receiver가 해당 grant를 commit하지
않았음이 확인되면 grant를 release할 수 있다. BUNDLE_COMMIT 전후가 불명확한 timeout은
release하지 않는다.

## 15. Control payload와 credit flow control

정확한 payload는 `spec/ble-control-v1.cddl`을 따른다.

- `CREDIT_UPDATE = [stream_id, granted_bytes, credit_sequence]`
- sender는 stream별 granted total에서 실제 전송 bytes를 차감하며 credit sequence 역행·중복을 무시한다.
- 초기 credit: `4 * negotiated_chunk_size`, 최대 outstanding credit 256 KiB.
- receiver가 temp write와 bitmap durability를 완료한 뒤에만 credit를 보충한다.
- credit 0에서 data write without response를 시작하지 않는다.
- control frame에는 4 KiB 별도 logical reserve를 두며 data credit에 포함하지 않는다.
- `PING/PONG`은 동일 8-byte nonce를 echo한다.
- `ERROR`와 `GOODBYE`는 상세 내부 오류나 식별자를 노출하지 않는다.

## 16. Resume

v1 resume은 process-local transfer ID가 아니라 다음 identity tuple 기준이다.

```text
packet_id + direction + peer_link_hash + exact_wire_sha256 + chunk_size + chunk_count
```

- partial transfer는 기본 10분 유지한다.
- sender가 동일 exact wire variant로 `RESUME_QUERY(packet_id, expected_total_sha256)`를 보낸다.
- receiver는 tuple이 일치할 때만 `RESUME_STATE`로 durable bitmap을 반환한다.
- bitmap 길이는 `ceil(chunk_count/8)`이고 unused high bits는 0이어야 한다.
- hash/chunk layout/peer identity가 다르면 partial을 폐기하고 처음부터 전송한다.
- chunk count 최대 1024, bitmap 최대 128 bytes.
- resume 상태 응답 후에도 BUNDLE_META와 최종 BUNDLE_COMMIT hash 검증을 생략하지 않는다.

Goal 3의 direct transfer는 safe full retry만 허용할 수 있지만, Goal 4 승인 전에는 위 resume 계약과 crash recovery를 구현해야 한다.

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
