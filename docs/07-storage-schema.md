# 07. Storage Schema and Transactions

## 1. 결정

프로토콜 상태의 단일 소유자는 Rust core다. Rust core가 전용 SQLite DB를 열고 transaction을 직접 수행한다.

Android 측 저장 범위:

- UI preference
- 권한 안내 상태
- Keystore로 wrap된 DB master key

Rust SQLite 범위:

- identity/inbound routing slot/contact/replay window
- message/bundle/payload
- transfer/token grant/receipt/tombstone
- peer encounter/rate limit
- boot/runtime checkpoint for bundle-age recovery
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
- default synchronous FULL in emergency/fixed relay, NORMAL in standby
- busy_timeout 3초
- auto_vacuum INCREMENTAL
- user_version 기반 migration

`PRAGMA user_version`이 migration dispatcher의 authoritative integer다.
`schema_meta.schema_version`은 diagnostics/export 표시용 mirror이며 migration
transaction 끝에서 같은 값으로 갱신한다. 둘이 다르면 startup을 중단하고
`RECOVERY_MODE`로 진입한다.

standby가 NORMAL이어도 TX-02 outbound message, TX-03 inbound commit,
TX-04 token grant reservation, TX-05 relay result와 migration은 transaction 시작 전
`synchronous=FULL`을 적용하고 종료 후 mode default로 복원한다. diagnostics/peer
statistics만 NORMAL durability를 허용한다.

## 3. 암호화 범위

- relay bundle bytes: 이미 E2EE ciphertext이므로 DB에 그대로 저장 가능
- contact display name, local plaintext message, private key blob: column-level XChaCha20-Poly1305
- exact envelope, nonce, AAD, key rotation은 `docs/06-security-and-threat-model.md`의 Local encrypted-value envelope를 따른다.
- primary key와 table/column을 AAD에 포함해 row swap을 차단한다.
- SQLite page 전체 암호화는 1.0 필수가 아니며 필요 시 SQLCipher ADR을 추가한다.

## 4. 핵심 transaction

### TX-01 Identity bootstrap

- identity row 생성
- ACTIVE inbound routing slot 생성
- schema/meta 생성
- 모두 성공하거나 전부 rollback

### TX-02 Outbound message

- 모든 수신자와 trust/key 상태를 먼저 검증
- 각 recipient contact의 outbound sender sequence를 1씩 원자적으로 증가
- 다중 수신이면 send group 생성
- 수신자별 message/bundle/payload insert
- diagnostic counter update

CHECK_IN/PRIVATE_SOS 다중 수신은 recipient마다 별도 HPKE ciphertext와 bundle을
만들며 전체 생성 transaction은 all-or-nothing이다.

### TX-03 Inbound bundle commit

- partial hash 검증
- duplicate/tombstone 재검사
- bundles/payload insert
- receiver local hop/age/token variant를 encode하고 `wire_sha256` 저장
- peer ingress accounting
- partial row delete
- 최종 수신자면 decrypt result message insert + receipt bundle insert
- contact replay max sequence와 4096-bit sliding bitmap, message/packet dedup window 갱신

filesystem의 partial 삭제는 SQLite transaction에 포함될 수 없다. 먼저 temp file을
fsync하고 검증된 bytes를 DB transaction으로 commit한 뒤, commit 성공 후 partial
file을 best-effort 삭제한다. crash로 남은 orphan partial은 startup cleanup이 제거한다.

### TX-04 Relay token grant reservation

- token split 계산
- sender bundle available `copy_tokens`를 sender 몫으로 감소
- sender local BP variant와 `wire_sha256`을 sender 몫 token으로 갱신
- receiver 몫을 `token_grants(RESERVED)`로 insert
- BUNDLE_META는 transaction commit 이후에만 전송

### TX-05 Relay copy result

- transfer state update
- ACK COMMITTED/COMMITTED_SAME_GRANT이면 grant `TRANSFERRED`
- commit 여부 불명 timeout이면 grant `UNCERTAIN`
- 명시적 non-commit 결과이면 grant `RELEASED` 후 available token 복원
- peer success/failure stat update

`RESERVED`와 `UNCERTAIN` grant token은 다른 peer에게 offer할 수 없다. terminal
bundle cleanup도 grant reconciliation 보존 기간 전에는 ledger를 삭제하지 않는다.

`token_grants` 코드:

- direction 0 SENDER, 1 RECEIVER
- state 0 RESERVED, 1 UNCERTAIN, 2 TRANSFERRED/COMMITTED, 3 RELEASED
- `retain_until_ms`는 최소 packet lifetime + 24시간이며 최대 8일

### TX-06 Receipt/cancel

- 최종 endpoint가 control message decrypt/signature 검증
- target bundle state update
- payload 삭제 조건 확인
- tombstone insert
- local message state update

relay는 암호화된 receipt/cancel의 target을 알 수 없으므로 이 transaction을 실행하지 않는다.

### TX-09 Boot age recovery

- 같은 boot ID이면 기존 elapsedRealtime 기준 유지
- 다른 boot ID이면 persisted wall checkpoint와 current wall delta 검증
- 유효하면 모든 pre-boot bundle의 stored age를 materialize하고 새 boot 기준으로 rebasing
- wall regression/missing checkpoint이면 pre-boot bundle을 `AGE_UNCERTAIN` 상태로 변경
- `AGE_UNCERTAIN` bundle은 offer/decrypt delivery 대상이 아니며 diagnostics/export만 허용

## 5. Migration 정책

- 모든 migration은 forward-only
- DB가 binary보다 높은 schema version이면 앱은 read-only 오류 화면
- migration 전 SQLite backup API 또는 checkpoint 후 `VACUUM INTO`로 일관된
  snapshot을 생성한다.
- DB 크기 + 16 MiB safety margin만큼 여유 공간이 없으면 migration을 시작하지 않고
  사용자에게 저장공간 확보를 요청한다.
- WAL 사용 중 main DB 파일만 raw copy하지 않는다.
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
6. contact별 replay window를 최신 256개로 trim
7. terminal token grant ledger를 packet lifetime + 24시간 이후 정리
8. incremental vacuum 최대 1000 pages

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
