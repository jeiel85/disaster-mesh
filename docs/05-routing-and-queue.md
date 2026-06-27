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

relay 전송을 시작할 때 receiver 몫은 sender의 available token에서 persistent grant
escrow로 이동한다. receiver commit 전에는 grant가 `RESERVED`, commit 결과가
불명확하면 `UNCERTAIN`이며 어느 상태에서도 다른 peer에게 재사용하지 않는다.

예를 들어 token 6에서 `(sender=3, receiver=3)` grant를 만들면 sender bundle의
available token은 즉시 3이 되고 grant ledger가 3을 보유한다. receiver가 commit하면
grant는 `TRANSFERRED`, 명시적으로 commit하지 않았음이 확인되면 `RELEASED`가 되어
sender available token으로 돌아갈 수 있다.

## 3. 수신 요청 판정

```text
if tombstoned(packet_id): reject
if already_committed(packet_id): duplicate
if expired(bundle): reject expired
if malformed summary: reject
if size/session/partial-storage limit exceeded: skip or retry later
if ingress peer rate limited: skip
if destination slot belongs to me:
    request DIRECT_DESTINATION regardless of copy_tokens
else if relay disabled:
    skip
else if hop_count + 1 >= hop_limit:
    skip
else if peer copy_tokens < 2:
    skip
else if relay quota unavailable:
    skip
else if source rate limited:
    skip
else:
    request RELAY_COPY
```

`hop_count + 1 == hop_limit`인 copy는 receiver가 최종 목적지가 아닐 경우 요청하지 않는다.
직접 목적지는 copy token과 relay hop restriction만 우회하며 size, session budget,
partial-storage cap, ingress-peer rate limit, 만료 검사를 우회하지 않는다. slot을 아는
peer의 direct-destination flood가 protected local storage를 소진하지 못해야 한다.

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
- current boot ID와 마지막 wall/elapsed checkpoint를 함께 기록
- offer 직전에:

```text
current_age = stored_age_ms + (elapsedRealtimeNow - received_elapsed_realtime_ms)
```

- forward encode 시 Bundle Age block에 current_age 반영
- process restart가 같은 boot ID이면 elapsedRealtime 차이를 그대로 사용한다.
- reboot 후에는 persisted wall checkpoint와 새 wall time이 모두 존재하고
  nondecreasing일 때 그 차이를 age에 더한다. delta가 최대 lifetime 이상이면 삭제한다.
- wall clock이 역행하거나 checkpoint가 없으면 pre-reboot bundle을
  `AGE_UNCERTAIN`으로 격리하고 offer하지 않는다. 임의의 작은 penalty로 전달을 계속해
  TTL을 과소평가하지 않는다.
- local hard max lifetime을 초과하면 삭제

신뢰된 secure clock이 없으므로 기기 소유자가 wall clock을 조작해도 항상 탐지하는
보안 속성은 아니다. 탐지 가능한 regression/missing checkpoint에는 fail-closed하고,
정상 OS clock에 대한 expiry 정책으로 취급한다.

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
2. local message와 replay bitmap을 원자적으로 commit
3. `should_generate_receipt(message_type)` 평가
4. DIRECT_TEXT/CHECK_IN/PRIVATE_SOS/LOCATION_UPDATE에는 receipt를 1회 생성
5. DELIVERY_RECEIPT에는 receipt를 절대 생성하지 않음
6. CANCEL에는 cancel 확인 정책에 따라 1회 생성 가능
7. original packet tombstone을 즉시 만들지 않는다. 다른 copy가 도착해도 duplicate 처리한다.

발신 endpoint:

- 자신에게 암호화된 receipt를 복호화하고 recipient signature를 검증한다.
- 검증되면 original packet을 `DELIVERED` 처리하고 자신의 payload를 삭제할 수 있다.
- tombstone 생성
- receipt 자체는 발신 목적지에 전달될 때까지 유지

relay:

- receipt/cancel class hint로 우선순위를 줄 수는 있지만 target packet ID나 서명을 검증할 수 없다.
- control bundle을 운반할 뿐 original relay copy를 삭제하거나 tombstone 처리하지 않는다.

## 9. Cancel 처리

- 발신 endpoint는 cancel 생성 즉시 자신의 원본 offer를 중단하고 local tombstone을 만든다.
- 최종 수신 endpoint는 decrypt/signature 검증 후 cancel sender identity를 보존한다.
- 원본이 이미 있으면 sender identity와 target packet/message ID를 대조하고 `INBOUND_CANCELED`로 전환한다.
- 원본이 아직 없으면 `pending_controls`에 cancel packet/message, target packet/message, verified sender identity, expiry를 저장한다.
- 원본이 나중에 도착하면 같은 transaction에서 pending cancel과 대조한 뒤 본문 표시 전에 취소 상태로 commit한다.
- sender/target이 충돌하는 cancel은 원본 상태를 변경하지 않고 quarantine/diagnostic 처리한다.
- relay는 cancel을 일반 P0 bundle로 운반하며 원본을 삭제하지 않는다.
- 이미 읽은 내용의 삭제는 보장하지 않는다.
- cancel bundle은 원본보다 긴 TTL을 가질 수 있다.

## 10. 저장 quota

기본 total 32 MiB.

```text
local protected pool  8 MiB
per-source hard cap   4 MiB
per-peer/day ingest   8 MiB
per-peer/day unverified direct 2 MiB
single bundle         12 KiB
cipher payload         8 KiB
partial transfers      4 MiB total
```

`local protected pool`은 자신의 outbound P0/P1과 최종 수신자로서 decrypt/signature
검증을 마친 P0/P1에만 사용한다. relay는 outer priority를 검증할 수 없으므로
relay P0/P1은 일반 relay pool에서 높은 eviction/offer 순위만 받고 protected pool을
소비하지 못한다.

slot match만 된 direct bundle도 decrypt/signature 검증 전에는 unverified direct
quota와 partial pool만 사용한다. 검증 후에만 local protected pool로 승격한다.

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
8. relay P1
9. relay P0
10. 검증된 local P1, protected floor를 유지
11. 검증된 local P0, 전체 시스템이 disk-full인 경우에만 가장 오래된 것

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
