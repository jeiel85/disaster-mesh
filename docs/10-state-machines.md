# 10. State Machines

## 1. Relay service

```text
STOPPED
  └─ user enables → STARTING
STARTING
  ├─ permissions/bluetooth ready → ACTIVE
  └─ failure → BLOCKED
ACTIVE
  ├─ battery/thermal policy → THROTTLED
  ├─ bluetooth off → BLOCKED
  ├─ OS/service destruction → RECOVERING
  └─ user stops → STOPPING
THROTTLED
  ├─ recovered → ACTIVE
  └─ user stops → STOPPING
BLOCKED
  ├─ condition fixed + user intent retained → STARTING
  └─ user stops → STOPPED
RECOVERING
  ├─ engine/db opened → ACTIVE or THROTTLED
  └─ unrecoverable → BLOCKED
STOPPING
  └─ links closed, scan/advertise stopped → STOPPED
```

서비스가 ACTIVE가 아니면 UI에서 이유를 명확히 표시한다.

## 2. Peer link

```text
DISCOVERED
  ├─ role arbitration win → CONNECTING
  └─ cooldown/no work → DEFERRED
CONNECTING
  ├─ opened → NEGOTIATING
  ├─ timeout → FAILED
  └─ duplicate → CLOSED
NEGOTIATING
  ├─ version ok → NOISE_HANDSHAKE
  └─ incompatible → CLOSING
NOISE_HANDSHAKE
  ├─ success → SECURE_SESSION
  └─ failure/timeout → CLOSING
SECURE_SESSION
  ├─ hello/slots done → INVENTORY
  └─ violation → CLOSING
INVENTORY
  ├─ requests exist → TRANSFERRING
  └─ no work → CLOSING
TRANSFERRING
  ├─ budget remains → INVENTORY
  ├─ complete/budget exhausted → CLOSING
  └─ transport error → FAILED
CLOSING → CLOSED
FAILED → cooldown → CLOSED
```

모든 state는 entry timestamp와 timeout을 가진다.

## 3. Inbound transfer

```text
OFFERED
  ├─ accepted → META_EXPECTED
  └─ rejected → TERMINAL
META_EXPECTED
  ├─ valid meta → RECEIVING
  └─ invalid → REJECTED
RECEIVING
  ├─ all chunks → VERIFYING
  ├─ timeout → PARTIAL
  └─ invalid chunk → REJECTED
VERIFYING
  ├─ hash/BP valid → COMMITTING
  └─ mismatch → REJECTED
COMMITTING
  ├─ DB commit → COMMITTED
  └─ quota/race duplicate → DUPLICATE/REJECTED
COMMITTED
  └─ send ACK → TERMINAL
PARTIAL
  ├─ resume within 10m → RECEIVING
  └─ expires → TERMINAL
```

ACK는 DB commit 이후에만 전송한다.

## 4. Outbound transfer

```text
AVAILABLE
  ├─ direct destination requests → META_SENT(no grant)
  └─ relay requests → GRANT_RESERVED
GRANT_RESERVED
  ├─ persistent escrow committed → META_SENT
  └─ DB failure → AVAILABLE
META_SENT
  ├─ credit → SENDING
  ├─ explicit pre-commit reject → RELEASING_GRANT
  └─ timeout after possible commit → UNCERTAIN_COMMIT
SENDING
  ├─ all chunks → COMMIT_SENT
  └─ disconnect → UNCERTAIN_COMMIT
COMMIT_SENT
  ├─ ACK COMMITTED → FINALIZING
  ├─ ACK COMMITTED_SAME_GRANT → FINALIZING
  ├─ ACK DUPLICATE_OTHER_GRANT/reject → RELEASING_GRANT
  └─ timeout → UNCERTAIN_COMMIT
FINALIZING
  ├─ grant TRANSFERRED transaction success → AVAILABLE or WAIT_ONLY
  └─ DB failure → RECOVERY_REQUIRED
RELEASING_GRANT
  ├─ grant RELEASED transaction success → AVAILABLE
  └─ DB failure → RECOVERY_REQUIRED
UNCERTAIN_COMMIT
  ├─ same peer + same grant reconciliation → COMMIT_SENT
  └─ no peer → retain escrow, do not reuse
WAIT_ONLY
  └─ direct destination encountered → transfer allowed
```

`available copy_tokens == 1`이면 WAIT_ONLY. `RESERVED` 또는 `UNCERTAIN` grant token은
available token으로 계산하지 않는다.

## 5. Outbound message

```text
DRAFT (memory-only; DB에 저장하지 않음)
  ├─ validation/encryption success → OUTBOUND_STORED
  └─ error → FAILED_LOCAL
OUTBOUND_STORED
  ├─ first relay commit → OUTBOUND_RELAYED
  ├─ verified receipt → OUTBOUND_RECEIPT_CONFIRMED
  ├─ user cancel → OUTBOUND_CANCEL_PROPAGATING
  └─ expiry → OUTBOUND_EXPIRED
OUTBOUND_RELAYED
  ├─ verified receipt → OUTBOUND_RECEIPT_CONFIRMED
  ├─ user cancel → OUTBOUND_CANCEL_PROPAGATING
  └─ expiry → OUTBOUND_EXPIRED
OUTBOUND_CANCEL_PROPAGATING
  ├─ verified receipt for cancel packet → OUTBOUND_CANCELED_CONFIRMED
  └─ cancel expiry → OUTBOUND_CANCELED_UNCONFIRMED
```

## 6. Contact trust

```text
IMPORTED_UNVERIFIED
  ├─ safety number compared → VERIFIED
  ├─ revoked → REVOKED
  └─ same identity new key → KEY_CHANGED
VERIFIED
  ├─ key update valid and user confirms → VERIFIED(new version)
  ├─ unexpected key → KEY_CHANGED
  └─ revoke → REVOKED
KEY_CHANGED
  ├─ in-person verify → VERIFIED
  └─ revoke → REVOKED
REVOKED
  └─ no automatic transition
```

P0 send to UNVERIFIED/KEY_CHANGED requires explicit blocking warning; default disallow.

## 7. Engine startup

```text
CLOSED → OPENING_DB → MIGRATING → LOADING_KEYS → RECOVERING_TRANSFERS → READY
```

- any key failure → `KEY_BLOCKED`
- newer DB version → `READ_ONLY_INCOMPATIBLE`
- corruption → `RECOVERY_MODE`
- reboot age checkpoint invalid → affected bundles `AGE_UNCERTAIN`, engine remains READY
- READY 이전 transport event는 bounded queue에 보관하거나 adapter 시작을 지연한다.

## 8. Numeric state ownership

모든 persisted state numeric code는 `contracts/state_codes.toml`이 유일한 기준이다. Rust/Kotlin enum과 SQL CHECK는 이 파일에서 생성하거나 CI에서 대조한다. unknown persisted code는 default branch로 흡수하지 않고 DB open을 `UNSUPPORTED_STATE_CODE`로 중단한다.
