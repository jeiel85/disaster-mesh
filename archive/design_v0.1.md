# Disaster Mesh – Production Design Bundle

> 상태: 설계 기준안 v0.1  
> 목적: 이동통신망·인터넷·공유기 없이 주변 스마트폰과 중계 노드만으로 긴급 메시지를 저장·운반·전달하는 오픈소스 재난 통신 시스템

---

## 1. 프로젝트 정의

이 프로젝트는 일반 채팅 앱이 아니다. 재난으로 통신 인프라가 중단되었을 때 주변 노드가 임시 네트워크를 구성하고, 수신자가 현재 연결되어 있지 않더라도 암호화된 메시지를 보관했다가 이후 접촉하는 노드를 통해 전달하는 **BLE 기반 DTN(Delay/Disruption-Tolerant Network)** 이다.

핵심 원칙은 다음과 같다.

1. 중앙 서버와 계정을 요구하지 않는다.
2. 인터넷이 없어도 핵심 기능이 완전히 동작한다.
3. 연결 경로가 동시에 존재하지 않아도 Store–Carry–Forward 방식으로 전달한다.
4. 중계 노드는 메시지 본문을 복호화할 수 없다.
5. 실시간 전달을 약속하지 않고 최종 전달 가능성을 높인다.
6. 짧은 텍스트, 생존 확인, 구조 요청을 파일·이미지보다 우선한다.
7. 배터리, 저장공간, 무선 혼잡이 제한된 환경을 전제로 한다.
8. 암호화 알고리즘을 자체 발명하지 않는다.
9. 재난 통신의 보조 수단으로 표현하며 전달을 보장한다고 홍보하지 않는다.

### 한 문장 정의

> 통신 인프라가 끊긴 재난 상황에서 사람과 기기의 이동을 전달 경로로 활용하는 종단간 암호화 기반의 지연 허용 메시 네트워크.

---

## 2. 범위

### 2.1 1.0 필수 범위

- Android 우선 지원
- BLE를 이용한 주변 노드 발견
- 두 기기 직접 통신
- 3개 이상 노드의 다중 홉 전달
- 수신자가 없을 때 메시지 임시 보관
- 이후 접촉 시 Store–Carry–Forward
- 1:1 종단간 암호화 메시지
- 가족·지인 생존 확인 메시지
- 신뢰 연락처 대상 비공개 SOS
- 선택적 오프라인 GPS 위치 첨부
- 메시지 TTL, 홉 제한, 복제 예산
- 중복 제거, 전달 영수증, 취소 메시지
- Android 상시 릴레이 모드
- 로컬 전용 저장소
- 인터넷 권한 없는 Android 빌드
- 수동 진단 로그 내보내기

### 2.2 1.0 제외 범위

- 일반 공개 채팅방
- 익명 사용자 간 자유 메시지
- 이미지·음성·동영상 전송
- 전화·실시간 음성 통화
- 실시간 위치 추적
- 전 세계 고유 닉네임 서버
- Signal과 동등하다고 주장하는 전방향 안전성
- 공식 재난 문자 대체
- 전달 시간 또는 전달 성공 보장
- iOS를 필수 중계 백본으로 사용

### 2.3 향후 범위

- iOS 전면 사용 및 제한적 백그라운드 릴레이
- Linux/Raspberry Pi 고정 중계기
- 기존 Android 공기계를 이용한 대피소 릴레이
- Wi-Fi Aware 또는 로컬 Wi-Fi 전송 어댑터
- BPv7 게이트웨이 및 외부 DTN 연동
- 구조기관 공개키 기반 서명 공지
- 검증된 구조대 대상 공개 SOS
- 다중 기기 사용자 신원
- 일회용 프리키 기반 강화된 전방향 안전성
- LoRa 또는 별도 무선 장비 게이트웨이

---

## 3. 표준 및 프로토콜 전략

### 3.1 기본 전략

완전히 새로운 번들 규격을 만들지 않고 다음 계층으로 구성한다.

1. **번들 형식:** IETF Bundle Protocol Version 7(BPv7)
2. **애플리케이션 페이로드:** Disaster Message Envelope(DME) v1
3. **근거리 전송:** BLE Convergence Layer Adapter(BLE-CLA) v1
4. **라우팅:** Direct Delivery + Binary Spray-and-Wait
5. **세션 보안:** Noise XX 계열의 링크 세션 암호화
6. **메시지 보안:** 애플리케이션 계층 종단간 암호화

BPv7은 번들 형식과 Store–Carry–Forward 의미를 제공하지만, 어떤 이웃에게 전달할지와 BLE로 어떻게 운반할지는 규정하지 않는다. 이 프로젝트가 해당 부분을 구현한다.

### 3.2 BPv7 적용 범위

필수 사용 요소:

- Primary Bundle Block
- Payload Block
- Bundle Age Block
- Hop Count Block
- Lifetime
- Status/receipt를 위한 애플리케이션 번들

초기 버전에서는 다음을 사용하지 않는다.

- 대용량 BP 단편화
- 완전한 외부 BP 노드 상호운용
- BPSec 완전 구현
- 인터넷 기반 TCP Convergence Layer

종단간 보호는 초기에는 DME 애플리케이션 페이로드 내부에서 수행한다. 향후 BP 생태계와 상호운용할 때 BPSec 지원을 별도 추가한다.

### 3.3 준수 표현

- BPv7 파서와 번들 형식 테스트가 통과하기 전에는 “BPv7 호환”이라고 표시하지 않는다.
- BLE-CLA는 프로젝트 고유 규격이므로 별도 공개 명세와 테스트 벡터를 제공한다.
- 암호화는 구현과 외부 검토가 완료되기 전까지 “감사 완료”라고 표현하지 않는다.

---

## 4. 전체 아키텍처

```text
┌──────────────────────────────────────────────────────────┐
│ Android / iOS UI                                         │
│ 메시지 · 연락처 · SOS · 네트워크 상태 · 릴레이 모드      │
├──────────────────────────────────────────────────────────┤
│ Application Use Cases                                    │
│ SendMessage · SendSOS · CheckIn · Cancel · Receipt       │
├──────────────────────────────────────────────────────────┤
│ Mesh Runtime                                             │
│ Peer Scheduler · Session Manager · Queue Policy          │
├──────────────────────────────────────────────────────────┤
│ Shared Rust Core                                         │
│ BPv7 · DME · E2EE · Routing · Dedup · Policy Engine      │
├──────────────────────────────┬───────────────────────────┤
│ Transport Adapters           │ Platform Adapters         │
│ BLE-CLA                      │ Storage · Key Vault       │
│ Future: Wi-Fi / LoRa         │ Clock · GPS · Diagnostics │
├──────────────────────────────┴───────────────────────────┤
│ Android Bluetooth / iOS Core Bluetooth                   │
└──────────────────────────────────────────────────────────┘
```

### 4.1 설계 원칙

- 프로토콜·암호화·라우팅 코어는 UI와 분리한다.
- BLE는 교체 가능한 Transport Adapter로 취급한다.
- 코어는 가능한 한 결정적인 상태 머신으로 작성한다.
- 무선 콜백에서 라우팅 상태를 직접 변경하지 않고 단일 Actor/Event Loop로 직렬화한다.
- 모든 영구 상태 변경은 트랜잭션으로 기록한다.
- 앱 종료와 재시작 후에도 보관 번들과 전달 상태를 복구한다.

---

## 5. 노드 역할

### 5.1 Mobile User Node

일반 사용자의 스마트폰이다.

- 메시지 생성 및 수신
- 주변 노드 검색
- 암호문 중계
- 제한된 저장공간 제공
- 배터리 상태에 따른 스캔 조절

### 5.2 Relay-Only Node

충전기에 연결된 공기계 또는 전용 단말이다.

- 화면이 꺼져도 Android Foreground Service로 동작
- 높은 스캔·광고 빈도
- 더 큰 중계 저장공간
- 사용자 메시지 UI 없이 릴레이 기능만 제공 가능

### 5.3 Fixed Relay Node

향후 Raspberry Pi/Linux 장치로 제공한다.

- 대피소·병원·주민센터 등에 배치
- BLE 다중 연결과 큰 저장공간
- 전원 복구 시 게이트웨이 역할 가능
- 스마트폰보다 안정적인 장시간 중계

### 5.4 Authority Node

검증된 기관 또는 구조대 노드다.

- 사전 등록된 기관 공개키로 공지 서명
- 일반 사용자 앱은 서명을 검증해 표시
- 기관 노드라고 주장하는 이름만으로 신뢰하지 않는다.

---

## 6. 신원 및 연락처

### 6.1 설치별 키

최초 실행 시 다음 키를 로컬에서 생성한다.

- Ed25519: 신원 서명
- X25519: 메시지 암호화용 장기 공개키
- Local Storage Master Key: 로컬 데이터 암호화

개인키는 Android Keystore 또는 iOS Keychain/Secure Enclave 지원 범위에 저장한다.

### 6.2 사용자 ID

```text
UserId = Base32(Truncate(SHA-256(Ed25519 public key))) + checksum
```

표시 예시:

```text
RM-7K4P-2D9Q-XC3M
```

표시 ID는 사람이 비교하기 위한 축약형이다. 내부에서는 전체 공개키 지문을 사용한다.

### 6.3 연락처 교환

서버 검색은 제공하지 않는다. 신뢰 연락처는 다음 방법으로 등록한다.

1. 대면 QR 스캔
2. QR 문자열 파일 공유
3. 안전번호 직접 비교

QR Contact Card:

```text
protocol_version
identity_signing_public_key
message_encryption_public_key
recipient_slot_id
optional_display_name
capabilities
checksum
```

### 6.4 Recipient Slot

외부 번들에 공개키나 실제 사용자 ID를 직접 넣지 않는다. 각 연락처마다 무작위 `recipient_slot_id`를 발급한다.

- 송신자는 상대가 제공한 slot을 목적지 EID에 사용한다.
- 중계기는 slot이 누구인지 알 수 없다.
- 즉시 이웃은 암호화된 세션에서 자신이 수신 가능한 slot 목록을 제시한다.
- 1.0에서는 slot이 장기간 유지되어 연결 가능성 메타데이터가 남는다.
- 향후 회전형 routing tag로 개선한다.

### 6.5 다중 기기

1.0에서는 설치 한 개가 하나의 신원이다. 다중 기기 동기화와 사용자 계정 개념은 제외한다. 향후 사람 신원 키가 여러 기기 키에 위임 인증서를 발급하는 구조로 확장한다.

---

## 7. 메시지 유형

| 유형 | 공개 여부 | 기본 우선순위 | 목적 |
|---|---|---:|---|
| DIRECT_TEXT | E2EE | P2 | 신뢰 연락처에게 짧은 텍스트 |
| CHECK_IN | E2EE | P1 | 생존·안전 상태 확인 |
| PRIVATE_SOS | E2EE | P0 | 지정 연락처에게 구조 요청 |
| LOCATION_UPDATE | E2EE | P1 | 선택적 위치와 이동 방향 |
| DELIVERY_RECEIPT | E2EE/서명 | P0 | 최종 수신 확인 |
| CANCEL | E2EE/서명 | P0 | SOS 또는 메시지 취소 |
| AUTHORITY_ALERT | 서명된 공개문 | P0 | 검증된 기관 공지 |
| KEY_UPDATE | E2EE/서명 | P1 | 연락처 키 변경 |

### 7.1 1.0 크기 정책

- 텍스트 중심
- 단일 DME 암호화 페이로드 최대 8 KiB
- PRIVATE_SOS 권장 최대 1 KiB
- 이미지·음성·첨부파일 금지
- 긴 메시지는 앱에서 발송 전에 축약을 유도

수치는 필드 테스트에 따라 변경 가능한 초기값이다.

---

## 8. DME v1 애플리케이션 페이로드

### 8.1 외부 라우팅 영역

BPv7 Primary Block과 확장 블록이 담당한다.

```text
source_eid          = opaque sender routing EID
 destination_eid    = opaque recipient slot EID
lifetime            = signed maximum lifetime
bundle_age          = accumulated age
hop_count / limit   = relay-modifiable count and fixed limit
payload             = DME ciphertext
```

### 8.2 암호화 내부 영역

```text
DmePlaintext {
  version
  message_type
  conversation_id
  message_id
  sender_identity_public_key
  recipient_identity_public_key_hash
  sender_sequence
  created_wall_time_optional
  content
  location_optional
  reply_to_optional
  signature
}
```

### 8.3 암호화 봉투

```text
DmeCiphertext {
  suite_id
  ephemeral_x25519_public_key
  nonce
  ciphertext
  authentication_tag
}
```

AEAD Associated Data에는 다음 불변 정보를 포함한다.

- 프로토콜 버전
- bundle ID
- destination slot
- lifetime
- message type
- payload length

릴레이가 암호문을 다른 목적지나 다른 번들 헤더에 붙이면 수신자가 검증을 거부한다.

---

## 9. 암호화 설계

### 9.1 목표

- 중계 노드는 본문을 읽을 수 없다.
- 메시지 변조와 위조를 검출한다.
- QR로 확인한 연락처 신원을 검증한다.
- 수동 키 교환 후 서버 없이 작동한다.
- 링크 메타데이터를 수동 도청자로부터 추가 보호한다.

### 9.2 메시지 계층

권장 암호 스위트:

- X25519: 키 합의
- HKDF-SHA-256: 메시지별 키 파생
- XChaCha20-Poly1305: 페이로드 AEAD
- Ed25519: 송신자 서명

송신자는 메시지마다 임시 X25519 키를 생성하고 암호화 후 임시 개인키를 삭제한다.

### 9.3 링크 세션

BLE 연결 후 Noise XX 패턴을 이용해 임시 링크 세션을 만든다.

링크 세션은 다음을 보호한다.

- 이웃의 slot 목록
- 인벤토리 요약
- 번들 요청 목록
- 링크 프레임과 전송 상태

최종 메시지 본문은 링크 세션과 별개로 종단간 암호화되어야 한다.

### 9.4 전방향 안전성에 대한 정직한 범위

1.0의 per-message ephemeral-static 방식은 송신자 임시키 삭제 효과는 있지만, 수신자의 장기 X25519 개인키가 나중에 유출되면 기록된 과거 암호문이 노출될 가능성이 있다.

따라서 1.0은 다음과 같이 표현한다.

> 종단간 암호화와 메시지 인증을 제공하지만 Signal과 동등한 완전한 전방향 안전성을 보장하지 않는다.

향후 signed prekey와 one-time prekey 풀을 추가하고 별도 보안 검토 후 강화된 전방향 안전성을 표방한다.

### 9.5 해결하지 못하는 위협

- 수신자 또는 송신자 기기 자체가 탈취된 경우
- 사용자가 악성 QR을 신뢰한 경우
- 무선 전파 방해 또는 Bluetooth 강제 비활성화
- 악성 중계기가 메시지를 버리는 행위
- 충분한 중계 노드가 존재하지 않는 경우
- 안정적인 전송 시간 보장
- 중앙 신원기관 없는 환경에서의 완전한 Sybil 방지

---

## 10. BLE-CLA v1

### 10.1 발견

BLE 광고에는 최소 정보만 넣는다.

```text
service_uuid
protocol_major
capability_bits
ephemeral_beacon_id
load_class
```

장기 사용자 ID, 연락처, 위치, 보관 메시지 수는 광고하지 않는다.

`ephemeral_beacon_id`는 주기적으로 변경해 수동 추적 가능성을 낮춘다.

### 10.2 연결 순서

```text
1. 주변 서비스 발견
2. 최근 접촉·백오프·배터리 기준으로 연결 후보 선정
3. GATT 연결
4. 프로토콜 버전 및 MTU 협상
5. Noise 링크 핸드셰이크
6. Capability 교환
7. 수신 가능한 recipient slot 교환
8. Inventory digest 교환
9. 누락 번들 요청
10. 우선순위와 예산에 따라 번들 청크 전송
11. Commit hash 검증
12. 세션 통계 기록 후 연결 종료
```

### 10.3 프레임 종류

```text
HELLO
NOISE_HANDSHAKE
CAPABILITIES
RECIPIENT_SLOTS
INVENTORY_SUMMARY
BUNDLE_OFFER
BUNDLE_REQUEST
BUNDLE_META
BUNDLE_CHUNK
BUNDLE_COMMIT
TRANSFER_ACK
ERROR
GOODBYE
```

### 10.4 청크 전송

- GATT MTU에 맞춰 동적 분할
- 각 번들에 전체 payload hash 포함
- 링크 프레임 sequence number 사용
- 누락 chunk 요청 지원
- 전체 hash가 일치한 후에만 번들을 저장 완료 상태로 전환
- 부분 번들은 세션 종료 후 제한 시간 동안만 유지

### 10.5 연결 폭주 방지

- 광고 시작 시 무작위 지연
- 최근 접촉 노드 cooldown
- peer별 exponential backoff
- 동시에 유지할 연결 수 제한
- 세션별 시간·바이트 예산
- 전송할 내용이 없으면 빠르게 종료

---

## 11. 라우팅

### 11.1 기본 알고리즘

**Direct Delivery + Binary Spray-and-Wait**를 기본으로 한다.

1. 이웃이 최종 목적지이면 즉시 직접 전달한다.
2. 목적지가 아니고 copy budget이 2 이상이면 예산을 절반으로 나눠 복제한다.
3. copy budget이 1이면 원칙적으로 목적지를 만날 때까지 보관한다.
4. P0 메시지는 설정된 범위에서 추가 기회 전달을 허용할 수 있다.

예시:

```text
A: copy budget 8
A → B: A=4, B=4
B → D: B=2, D=2
D → E: D=1, E=1
```

### 11.2 기본 정책값

아래 값은 시뮬레이션과 현장 테스트로 튜닝할 초기값이다.

| 유형 | TTL | Hop limit | Copy budget |
|---|---:|---:|---:|
| PRIVATE_SOS | 24시간 | 16 | 12 |
| CHECK_IN | 48시간 | 12 | 8 |
| DIRECT_TEXT | 72시간 | 12 | 6 |
| RECEIPT/CANCEL | 7일 | 16 | 12 |
| AUTHORITY_ALERT | 48시간 | 16 | 정책별 |

### 11.3 중복 제거

- 기본 키: BPv7 bundle ID
- 영구 dedup table 유지
- 이웃 교환용 Bloom filter 사용 가능
- Bloom filter 오탐으로 중요한 번들이 누락되지 않도록 고우선순위 번들은 명시 ID 재검증
- 만료된 ID tombstone은 제한 기간 유지

### 11.4 메시지 수명

신뢰할 수 있는 인터넷 시각이 없으므로 BPv7 Bundle Age Block을 사용한다.

- 정확한 시각을 알고 있으면 creation time과 current time을 사용
- 시각 정확성을 보장할 수 없으면 각 노드가 보관한 monotonic elapsed time을 bundle age에 누적
- 악성 노드가 번들 수명을 조작할 가능성은 완전히 제거할 수 없으므로 수신 노드는 로컬 최대 보관 한도를 추가 적용

### 11.5 전달 영수증

최종 수신자는 `DELIVERY_RECEIPT` 번들을 생성한다.

- 원본 bundle ID
- 수신자 서명
- 수신 시각 또는 local age
- receipt ID

영수증을 받은 노드는 해당 원본 번들을 삭제하고 tombstone을 남긴다. 발신자는 영수증이 돌아와야 “최종 전달됨”으로 표시한다.

### 11.6 상태 표시 용어

- `기기에 보관됨`
- `중계 노드 1개 이상에 복제됨`
- `수신 기기에 전달됨`
- `전달 영수증 확인됨`
- `만료됨`
- `취소 전파 중`

“보냄”만 표시해 전달이 완료된 것처럼 오해시키지 않는다.

---

## 12. 큐, 저장공간, 혼잡 제어

### 12.1 저장 구조

```text
bundles
bundle_payloads
bundle_copies
peer_encounters
delivery_receipts
tombstones
contacts
keys
relay_statistics
```

릴레이가 보관하는 payload는 종단간 암호문이다. 사용자가 수신한 평문 메시지는 별도 로컬 암호화 영역에 저장한다.

### 12.2 기본 저장 정책

- 기본 중계 저장공간: 32 MiB
- 사용자가 16–256 MiB 범위에서 조절
- 최소 25%는 P0/P1용으로 예약
- 단일 source 또는 이웃이 전체 저장공간을 독점하지 못하도록 quota 적용

### 12.3 삭제 순서

저장공간 부족 시:

1. 손상·검증 실패 번들
2. 만료 번들
3. 전달 영수증으로 완료된 번들
4. 낮은 우선순위의 오래된 번들
5. 동일 source가 quota를 초과한 번들
6. P0/P1은 마지막에 삭제

### 12.4 혼잡 방지

- TTL
- Hop limit
- Copy budget
- peer/session별 byte budget
- source별 보관 quota
- packet size 제한
- invalid packet 즉시 폐기
- rate limit
- randomized backoff
- 최근 접촉 및 전달 효율 기반 peer 선택

---

## 13. 악용 방지

중앙 서버가 없으므로 완전한 신원 통제와 Sybil 차단은 불가능하다. 1.0은 피해를 제한하는 방향으로 설계한다.

### 13.1 기본 정책

- 신뢰 연락처의 1:1 메시지만 사용자에게 자동 표시
- unknown source 본문은 자동 복호화·알림하지 않음
- public anonymous chat 미지원
- 공개 SOS 미지원
- 기관 공지는 사전 신뢰한 기관 공개키 서명 필요
- peer별 토큰 버킷
- source key별 bundle quota
- 최대 payload 및 bundle lifetime 강제
- 반복 오류 peer cooldown
- 사용자가 로컬에서 peer/source 차단 가능

### 13.2 공개 SOS를 나중으로 미루는 이유

누구나 구조 요청을 전파할 수 있게 하면 재난 상황에서 스팸, 허위 신고, 저장공간 고갈 공격이 발생할 수 있다. 구조기관과 현장 파일럿을 통해 인증·우선순위 정책을 검증한 뒤 추가한다.

---

## 14. 재난 UX

### 14.1 홈 화면

```text
[재난 통신 모드: 켜짐]
주변 노드: 4
최근 10분 접촉 노드: 9
중계 보관: 18개 / 1.8 MiB
배터리 모드: 긴급

[생존 알림 보내기]
[도움 요청]
[가족에게 메시지]
[중계 상태]
```

### 14.2 모드

#### 대기 모드

- 낮은 스캔 빈도
- 평상시 준비 상태
- 배터리 우선

#### 긴급 모드

- 사용자가 명시적으로 활성화
- Android Foreground Service
- 스캔 및 광고 빈도 증가
- 상시 알림 표시
- 고우선순위 메시지 우선

#### 고정 중계 모드

- 충전 중 사용 권장
- 높은 duty cycle
- 넓은 저장공간
- 사용자 메시지 알림 최소화
- 발열·배터리 상태에 따라 자동 감속

### 14.3 SOS 입력

- 의료 지원
- 매몰·고립
- 화재·연기
- 실종자
- 식수·식량
- 기타

입력 항목:

- 짧은 설명
- 인원수
- 부상 여부
- 위치 첨부 여부
- 마지막 이동 방향
- 배터리 잔량 공유 여부
- 수신 연락처

오발송을 막기 위해 길게 누르기 또는 2단계 확인을 사용하되, 접근성을 해치지 않는다.

### 14.4 위치

- GPS는 인터넷 없이도 위성 수신이 가능하나 초기 위치 확보가 느릴 수 있다.
- 위치는 기본적으로 첨부하지 않는다.
- 사용자가 선택한 수신자에게만 E2EE로 포함한다.
- 수동 위치 설명을 항상 제공한다.

### 14.5 안전 문구

앱 내부에 항상 다음 의미를 명확히 표시한다.

> 이 앱은 통신 인프라가 끊긴 상황에서 메시지 전달 가능성을 높이는 보조 수단입니다. 주변 중계 노드와 접촉 경로가 없으면 메시지가 전달되지 않으며, 긴급 구조를 보장하지 않습니다.

---

## 15. Android 구현

### 15.1 기술

- Kotlin
- Jetpack Compose
- Foreground Service
- BluetoothLeScanner
- BluetoothGatt / BluetoothGattServer
- Room 또는 암호화된 SQLite adapter
- Android Keystore
- Rust core JNI/UniFFI bridge

### 15.2 권한

버전별로 정확히 분기한다.

- BLUETOOTH_SCAN
- BLUETOOTH_CONNECT
- BLUETOOTH_ADVERTISE
- FOREGROUND_SERVICE
- FOREGROUND_SERVICE_CONNECTED_DEVICE
- 선택적 위치 첨부 시 ACCESS_FINE_LOCATION

BLE-only core 빌드는 `INTERNET` 권한을 요청하지 않는다.

### 15.3 백그라운드 정책

- 긴급·중계 모드는 사용자가 앱 전면에서 직접 시작
- 상시 알림 제공
- 서비스가 종료되면 UI와 로컬 알림으로 명확히 표시
- 제조사별 배터리 최적화로 인한 종료를 진단하되 무조건적인 최적화 해제 유도는 피함
- 프로세스 재생성 시 보관 큐와 서비스 상태 복구

### 15.4 Android 릴레이 기준

Android를 1차 중계 백본으로 삼는다. 다만 제조사와 OS 버전에 따라 백그라운드 동작 차이가 있으므로 최소 5개 제조사·여러 OS 버전에서 실제 화면 꺼짐 테스트를 수행한다.

---

## 16. iOS 구현

### 16.1 기술

- Swift
- SwiftUI
- Core Bluetooth central/peripheral
- Background Modes: bluetooth-central, bluetooth-peripheral
- State Restoration
- Keychain
- Rust core bridge
- iOS 26 이상에서 적합할 경우 Live Activity 연계 실험

### 16.2 제품 정책

- iOS는 전면 실행 중 완전 기능
- 백그라운드 릴레이는 best-effort
- OS가 앱을 장시간 계속 실행한다고 가정하지 않음
- iOS 노드만으로 재난망의 지속성을 보장하지 않음
- Android 및 고정 릴레이를 네트워크 백본으로 권장

### 16.3 출시 전 검증

- 잠금 화면
- 저전력 모드
- 앱 background/suspended
- 앱 강제 종료 후 상태
- Bluetooth 껐다 켜기
- 재부팅
- Live Activity 종료
- 여러 iPhone 모델과 OS 버전

---

## 17. 공유 Rust Core

### 17.1 책임

- BPv7 encode/decode
- DME encode/decode
- 암호화 및 서명
- 번들 검증
- 라우팅 결정
- TTL/hop/copy budget
- dedup
- 큐 우선순위
- test vector 생성
- 시뮬레이터와 동일 코드 사용

### 17.2 코어 API 예시

```text
CoreEvent
- PeerSessionOpened
- PeerCapabilitiesReceived
- PeerSlotsReceived
- InventoryReceived
- FrameReceived
- TimerTick
- LocalMessageCreated
- StorageLoaded

CoreAction
- SendFrame
- PersistBundle
- UpdateBundle
- DeleteBundle
- NotifyUser
- SchedulePeerCooldown
- CloseSession
```

### 17.3 결정성

동일한 상태와 동일한 이벤트 순서가 주어지면 동일한 CoreAction을 출력하도록 한다. 이를 통해 시뮬레이션, 재현 테스트, fuzzing을 쉽게 만든다.

---

## 18. 저장 및 개인정보

### 18.1 원칙

- 계정 없음
- 서버 없음
- 광고 없음
- 분석 SDK 없음
- Firebase 없음
- 원격 설정 없음
- 연락처 주소록 자동 업로드 없음
- 인터넷 권한 없음(BLE-only 빌드)

### 18.2 로컬 보안

- 릴레이 payload는 E2EE 암호문 그대로 저장
- 수신 평문은 device master key로 별도 암호화
- 로그에 메시지 본문, 공개키 전체, 위치를 기록하지 않음
- 진단 export는 사용자 동의 후 생성
- 앱 잠금 및 생체 인증 선택 지원
- 긴급 데이터 삭제 기능은 오작동 방지 UX와 함께 제공

### 18.3 보존

- 릴레이 번들은 receipt 또는 TTL로 삭제
- 사용자의 받은 메시지는 사용자 정책에 따라 보존
- 위치가 포함된 SOS는 더 짧은 기본 보존 기간 적용

---

## 19. 테스트 전략

### 19.1 단위 테스트

- BPv7 parser/encoder
- canonical encoding
- DME round trip
- signature verification
- AEAD tamper rejection
- TTL and Bundle Age
- hop limit
- copy budget split
- dedup
- eviction policy
- receipt and cancel

### 19.2 프로토콜 테스트 벡터

저장소에 언어 독립 테스트 벡터를 공개한다.

```text
input_plaintext.cbor
contact_card.cbor
bundle.bpv7
expected_ciphertext.bin
expected_bundle_id.txt
expected_validation.json
```

암호화 nonce가 무작위인 경우 고정 테스트 전용 키와 nonce를 사용한다.

### 19.3 시뮬레이터

Rust로 노드와 접촉 그래프 시뮬레이터를 먼저 만든다.

측정 지표:

- 최종 전달률
- p50/p95 전달 지연
- 평균 복제 수
- 중복 바이트
- 노드별 저장공간
- P0 메시지 전달률
- 악성 노드 비율별 성능
- 배터리 비용 모델

시나리오:

- 선형 A-B-C
- C가 나중에 B와 접촉
- 네트워크 분할 후 재결합
- 10/50/100/500 노드
- 특정 노드 이동 집중
- 릴레이 30% 이탈
- 악성 노드가 번들 드롭
- 스팸 노드가 quota 공격

### 19.4 실제 기기 테스트

- 서로 다른 제조사 Android
- 서로 다른 OS 버전
- 화면 꺼짐 8시간 이상
- 재부팅 및 프로세스 종료
- 3, 5, 10개 기기 다중 홉
- 철근 콘크리트 건물
- 계단·복도·지하 공간
- 저전력 모드
- 배터리 15% 이하
- Bluetooth on/off 반복

### 19.5 보안 테스트

- parser fuzzing
- malformed CBOR/BPv7
- oversized length
- replay
- duplicate
- bundle ID collision attempt
- signature substitution
- destination swap
- corrupted chunk
- invalid Noise transcript
- key replacement
- QR tampering
- storage extraction

### 19.6 외부 검토

Stable 1.0 전에 다음을 요구한다.

- 독립 암호 프로토콜 리뷰
- Android 백그라운드 정책 리뷰
- 개인정보·위협 모델 리뷰
- 재난 대응 전문가 UX 검토
- 공개 베타와 이슈 대응 기간

---

## 20. 관측성과 진단

서버 원격 분석을 사용하지 않는다.

로컬 지표:

- 발견 peer 수
- 연결 성공·실패
- 평균 세션 시간
- 전송 번들/바이트
- 중복 거절 수
- 만료/삭제 수
- 배터리 사용량
- 최근 Foreground Service 종료 원인

진단 export 시:

- 메시지 본문 제외
- 정확한 위치 제외
- 공개키와 node ID 해시 처리
- 사용자가 내보낼 항목 확인

---

## 21. 저장소 구조

```text
/
├─ README.md
├─ LICENSE
├─ SECURITY.md
├─ THREAT_MODEL.md
├─ CONTRIBUTING.md
├─ CODE_OF_CONDUCT.md
├─ docs/
│  ├─ architecture.md
│  ├─ protocol-dme-v1.md
│  ├─ protocol-ble-cla-v1.md
│  ├─ routing.md
│  ├─ disaster-ux.md
│  └─ adr/
├─ spec/
│  ├─ dme-v1.cddl
│  ├─ contact-card-v1.cddl
│  └─ test-vectors/
├─ core/
│  ├─ bpv7/
│  ├─ crypto/
│  ├─ dme/
│  ├─ routing/
│  ├─ policy/
│  ├─ simulator/
│  └─ ffi/
├─ apps/
│  ├─ android/
│  └─ ios/
├─ relays/
│  └─ linux/
├─ tools/
│  ├─ protocol-inspector/
│  └─ test-vector-generator/
└─ .github/
   └─ workflows/
```

---

## 22. 오픈소스 정책

### 권장 라이선스

- 코드: Apache-2.0
- 프로토콜 문서: CC BY 4.0
- 예제·테스트 벡터: CC0 또는 Apache-2.0

Apache-2.0은 특허 조항과 상업적 활용 가능성을 제공해 프로토콜 채택에 유리하다.

### 필수 프로젝트 문서

- SECURITY.md
- THREAT_MODEL.md
- 프로토콜 버전 정책
- 호환성 표
- 취약점 제보 절차
- 암호화 범위와 비보장 범위
- 재난 안전 면책과 사용 지침
- SBOM
- 의존성 라이선스 목록

### 참조 구현 사용

Bitchat, Briar, bp7-rs, dtn7-rs 등은 구조와 테스트 아이디어를 참고할 수 있다. 코드를 가져올 때는 버전, 라이선스, 보안 상태를 별도로 검토하고 출처를 보존한다.

---

## 23. CI/CD와 품질 게이트

### Pull Request 필수 검사

- cargo fmt
- cargo clippy -D warnings
- cargo test
- cargo audit
- cargo deny
- parser fuzz smoke test
- Android lint
- Android unit test
- protocol test vectors
- dependency license scan
- secret scan

### Release 필수 조건

- protocol version 고정
- migration test
- upgrade/downgrade test
- 최소 지원 OS 테스트
- background relay test report
- battery benchmark report
- threat model 갱신
- known limitations 문서화
- reproducible build 또는 빌드 provenance
- 서명된 APK/AAB
- F-Droid metadata 준비

---

## 24. 단계별 개발 목표

### Goal 0 — 프로토콜 명세와 시뮬레이터

결과물:

- BPv7 사용 범위 문서
- DME v1 CDDL
- BLE-CLA v1 초안
- Rust core workspace
- A-B-C Store–Carry–Forward 시뮬레이터
- Direct Delivery + Binary Spray-and-Wait
- TTL/hop/copy budget/dedup

완료 기준:

- A와 C가 동시에 연결되지 않아도 B의 이동 후 전달
- 동일 번들이 무한 순환하지 않음
- TTL 만료 후 제거
- 100노드 시뮬레이션 자동 테스트

### Goal 1 — Android 두 기기 직접 E2EE

결과물:

- Compose 기본 UI
- BLE advertise/scan/GATT
- Noise 링크 세션
- QR 연락처 등록
- 1:1 DME 메시지 암호화
- 로컬 저장

완료 기준:

- 인터넷·SIM·공유기 없이 두 Android 기기 전송
- 중간 packet capture에서 본문 확인 불가
- 변조 packet 거절
- 앱 재시작 후 메시지와 키 복구

### Goal 2 — Android 다중 홉 DTN

결과물:

- Relay queue
- inventory exchange
- bundle chunking
- copy budget
- receipt/tombstone
- 3–10기기 다중 홉

완료 기준:

- A-B 연결 후 분리, 이후 B-C 연결 시 C 전달
- B가 본문 복호화 불가
- receipt가 A로 돌아오면 완료 표시
- 중복 전송량과 저장량 측정 가능

### Goal 3 — 재난 UX와 릴레이 모드

결과물:

- 생존 확인
- 비공개 SOS
- 선택적 위치
- 취소 메시지
- 대기/긴급/고정 릴레이 모드
- foreground notification
- 저장공간·배터리 정책

완료 기준:

- 화면 잠금 상태에서 Android relay 지속
- 서비스 종료를 사용자가 인지 가능
- P0 메시지가 P2보다 먼저 전달
- 저장공간 압박에서 P0 예약 영역 유지

### Goal 4 — 하드닝과 공개 Android 베타

결과물:

- parser fuzzing
- test vector 공개
- 위협 모델
- 진단 export
- 보안/배터리 보고서
- F-Droid 및 Play 배포 준비

완료 기준:

- 주요 제조사 실기기 장시간 테스트
- crash-free field test
- 알려진 보안 한계 공개
- 외부 프로토콜 리뷰 반영
- 재난 통신 보조 수단 면책 명확화

### Goal 5 — iOS 클라이언트

결과물:

- SwiftUI 앱
- Core Bluetooth transport
- State Restoration
- Rust core 공유
- Android 상호운용

완료 기준:

- Android↔iOS 직접 전송
- Android→relay→iOS 다중 홉
- 전면·백그라운드·잠금 상태별 동작 표 공개
- iOS가 상시 중계된다고 오해시키지 않는 UX

### Goal 6 — 고정 릴레이와 현장 파일럿

결과물:

- Linux/Raspberry Pi relay
- relay-only 관리 UI
- 기관 서명 공지
- 대피소 시나리오 파일럿

완료 기준:

- 스마트폰만 사용한 경우보다 전달률 개선 입증
- 전원 재시작 후 큐 복구
- 기관 키 검증
- 현장 운영 매뉴얼 작성

---

## 25. 바이브 코딩용 `/goal` 프롬프트

### /goal 0

```text
/goal
Create the protocol-first foundation for an open-source disaster DTN messenger. Build a Rust workspace implementing the selected BPv7 bundle subset, DME v1 deterministic CBOR schema, bundle age, hop count, TTL, deduplication, Direct Delivery, and Binary Spray-and-Wait. Include an event-driven deterministic core and a simulator that proves A can send to C through B even when A and C are never simultaneously connected. Add property tests, malformed-input tests, and documented protocol decisions. Do not implement UI or Bluetooth yet. Do not claim full BPv7 compatibility until conformance tests pass.
```

### /goal 1

```text
/goal
Build the Android direct-communication client for the disaster DTN project. Use Kotlin and Jetpack Compose with a Rust core bridge. Implement BLE advertising, scanning, GATT client/server roles, version negotiation, Noise XX link sessions, QR contact exchange, local key generation, DME v1 end-to-end encrypted one-to-one text messages, encrypted local persistence, and clear delivery states. The app must operate with mobile data, Wi-Fi, and SIM unavailable and must not request Android INTERNET permission. Add unit and instrumentation tests on at least two physical Android devices.
```

### /goal 2

```text
/goal
Implement multi-hop store-carry-forward delivery on Android. Add relay storage, peer inventory exchange, BLE chunk transfer, bundle commit verification, copy budgets, hop limits, bundle age, receipt bundles, tombstones, queue eviction, per-peer quotas, and randomized backoff. Demonstrate A-to-B transfer, separation, then B-to-C delivery without A and C meeting. Ensure B cannot decrypt the payload. Add a local diagnostics screen showing peer encounters, bundle state, duplicates, expirations, and transfer bytes without exposing message contents.
```

### /goal 3

```text
/goal
Turn the Android prototype into a disaster-focused product. Add trusted-contact check-in, private SOS, optional offline GPS location, SOS cancellation, priority queues, standby/emergency/fixed-relay modes, Android connected-device foreground service, persistent notification, process restart recovery, battery-aware duty cycling, storage reservation for P0/P1 messages, accessibility, and explicit non-guaranteed-delivery language. Exclude public chat, anonymous public SOS, images, voice, analytics, ads, accounts, and cloud services.
```

### /goal 4

```text
/goal
Harden the disaster DTN Android application for a public open-source beta. Add parser fuzzing, protocol test vectors, dependency and license auditing, SBOM generation, threat-model documentation, privacy-preserving diagnostic export, migration tests, background relay endurance tests, battery benchmarks, crash recovery, corrupted-database handling, and release CI. Prepare Play and F-Droid builds while preserving a BLE-only build without INTERNET permission. Document every known limitation and do not describe the app as guaranteed emergency communication.
```

### /goal 5

```text
/goal
Create an interoperable iOS client using SwiftUI, Core Bluetooth, state restoration, Keychain, and the shared Rust protocol core. Support foreground direct messaging and best-effort background BLE behavior within Apple platform rules. Validate Android-to-iOS direct and relayed delivery. Publish a behavior matrix for foreground, background, suspended, force-quit, low-power, locked-screen, and Live Activity states. Do not treat iOS as the sole always-on relay backbone.
```

---

## 26. 주요 리스크

| 리스크 | 영향 | 대응 |
|---|---|---|
| iOS 장시간 백그라운드 제한 | 중계망 단절 | Android·고정 릴레이를 백본으로 설계 |
| 제조사별 Android 절전 정책 | 서비스 중단 | foreground service, 복구, 실기기 매트릭스 |
| 사용자가 적음 | 전달 경로 부재 | 사전 설치 캠페인, 가족 단위, 고정 릴레이 |
| BLE 혼잡 | 지연·충돌 | 백오프, 연결 제한, 작은 payload, 예산 |
| 악성 중계기의 드롭 | 전달 실패 | 복수 복제, 영수증, best-effort 명시 |
| 스팸·Sybil | 저장공간 고갈 | public chat 제외, quota, rate limit |
| 암호 설계 오류 | 기밀성 상실 | 검증된 primitive, test vector, 외부 리뷰 |
| 재난 앱에 대한 과신 | 안전 위험 | 전달 보장 금지, 명확한 상태·면책 |
| 배터리 고갈 | 노드 이탈 | 모드 분리, adaptive duty cycle, 충전 릴레이 |
| 시계 불일치 | TTL 오류 | BPv7 Bundle Age와 monotonic elapsed time |

---

## 27. 출시 성공 기준

Stable 1.0은 기능 완성이 아니라 다음 조건 충족을 의미한다.

1. Android에서 인터넷 없이 3개 이상 노드의 Store–Carry–Forward가 반복 재현된다.
2. 중계 노드와 로컬 packet capture가 메시지 본문을 복호화하지 못한다.
3. 변조·재생·중복·초과 크기 packet이 안전하게 거부된다.
4. 화면 잠금과 장시간 실행에서 relay 동작 보고서가 공개된다.
5. 전달 상태가 보관·복제·최종 수신·영수증으로 구분된다.
6. 메시지 전달을 보장하지 않는다는 점이 제품 전반에 표시된다.
7. 프로토콜 명세, 테스트 벡터, 위협 모델, 보안 제보 절차가 공개된다.
8. 외부 보안 리뷰의 중대 문제를 해결한다.
9. 앱은 계정, 광고, 분석, 중앙 메시지 서버 없이 작동한다.
10. 재난 대응 전문가 또는 관련 단체의 제한된 파일럿 평가를 거친다.

---

## 28. 참고 기준 문서

- IETF RFC 9171 — Bundle Protocol Version 7
- IETF RFC 9172 — Bundle Protocol Security
- IETF RFC 9173 — Default Security Contexts for BPSec
- Android Developers — Communicate in the background with BLE
- Android Developers — Foreground service types and connectedDevice
- Apple Developer — Core Bluetooth Background Processing
- Apple Developer — Core Bluetooth
- Bitchat protocol whitepaper and source repositories
- bp7-rs and dtn7-rs source repositories

---

## 29. 최종 기술 결정 요약

```text
제품 목적       재난 전용 오프라인 긴급 메시지
1차 플랫폼      Android
2차 플랫폼      iOS
핵심 네트워크   BLE 기반 DTN
번들 형식       BPv7
앱 페이로드     DME v1 / deterministic CBOR
라우팅          Direct Delivery + Binary Spray-and-Wait
메시지 보안     X25519 + HKDF-SHA-256 + XChaCha20-Poly1305 + Ed25519
링크 보안       Noise XX
연락처 등록     QR 및 안전번호 비교
중계 저장       암호문만 저장
서버/계정       없음
Android 인터넷 권한  없음(BLE-only 1.0)
공개 채팅       미지원
공개 SOS        초기 미지원
첨부파일        초기 미지원
핵심 가치       보장된 실시간 통신이 아니라 단절 환경의 최종 전달 가능성 향상
```
