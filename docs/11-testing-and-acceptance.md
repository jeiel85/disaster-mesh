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
- DIRECT_TEXT 2,000-scalar and 7,800-byte dual limit boundary
- all user text fields enforce both Unicode-scalar and UTF-8-byte limits
- unknown major version
- BP EID Base32 lowercase/no-padding canonical form

### Crypto

- contact card self-signature
- contact QR exact framing/Base45/CRC32C and 512-byte cap
- safety number cross-order symmetry and Base32 formatting
- contact import cannot directly set verified state
- HPKE seal/open round trip
- wrong recipient key failure
- AAD field 한 개 변경 시 failure
- Ed25519 signature substitution
- ciphertext truncation
- sender key mismatch
- contact별 sender sequence 독립성과 delayed-message replay window
- location은 captured-before-send duration만 encode하고 raw device elapsed time은 제외
- PRIVATE_SOS severe injury count cannot exceed people count
- receipt/cancel signer must match the original endpoint role

### Routing

- token split conservation
- sender/receiver BP routing-block token variant와 BUNDLE_META 값 일치
- token 1 relay 금지
- direct destination always allowed
- hop boundary
- expired rejection
- priority score ordering
- deterministic tie break
- quota/eviction verified-local protected floor
- direct-destination flood still obeys unverified ingress quota

### State

- relay transfer 전 grant escrow 원자성
- ACK 유실 후 uncertain grant 재사용 금지
- same-grant reconciliation과 duplicate ACK idempotent
- process recovery transitions
- same-boot age recovery and reboot AGE_UNCERTAIN fail-closed transition
- receipt/cancel idempotent
- conflict packet quarantine

## 3. Property tests

필수 property:

```text
sum(split_tokens(n)) == n
receiver_tokens >= 1 and sender_tokens >= 1
relay-copy available_tokens across committed copies + pending reserved/uncertain grants == initial tokens
uncertain grant tokens are never offered to another peer
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
- BLE negotiated-limit and chunk-count consistency validation
- chunk reassembler
- diagnostic export redaction and ZIP entry-name generator
- diagnostic export 4 MiB cap and truncation manifest

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
- verify peer/day quota and verified-local protected floor

### SIM-005 Clock disorder

- wall clocks ±24h
- reboot events
- age monotonic, valid checkpoint rebasing, invalid checkpoint offer 금지 검증

## 7. Android unit tests

- permission state reducer
- API 26-30 legacy BLE/location permission and API 31+ separated permission flow
- service state reducer
- command executor idempotency
- command executor does not block actor until GATT completion
- queue overflow policy
- legacy advertisement byte size <= 31 and fallback behavior
- segment/reassembly
- notification redaction
- lock-screen public notification contains no peer/bundle counts
- manifest backup/data-extraction exclusion
- model mapping

## 8. Instrumentation

- Keystore wrap/unwrap
- process recreation
- reboot recovery eligibility and user-visible resume-required fallback
- DB migration
- migration backup free-space gate and WAL-consistent snapshot
- PRAGMA user_version/schema_meta mismatch recovery
- foreground service start/stop
- Bluetooth off/on state
- permission revoke while active
- low storage callback
- location timeout/manual fallback
- SOS long-press와 TalkBack accessibility send path

BLE 자체는 fake adapter와 physical-device suite를 분리한다.

## 9. Physical device matrix

최소 범주:

- Samsung API 26/28급 구형기
- Samsung 최신 API 36/37
- Google Pixel API 31/34/36/37
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
- A의 token grant escrow/transfer state 확인
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
- verified-local protected quota invariant violations 0
- known critical/high security issue 0

전달 지연과 실제 재난 전달률은 환경 의존이므로 고정 SLA로 출시 조건을 표현하지 않는다.
