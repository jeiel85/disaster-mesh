# 06. Security and Threat Model

## 1. 보호 목표

- relay와 수동 도청자가 메시지 본문·정확한 위치를 읽지 못한다.
- 수신자가 발신자 서명을 검증할 수 있다.
- header/payload 조작을 검출한다.
- 재생·중복·저장공간 공격의 피해를 제한한다.
- 서버·CA 없이 대면 연락처 인증을 제공한다.

## 2. 비보장

- 무선 방해 대응
- 악성 relay의 선택적 drop 방지
- 충분한 접촉 경로가 없는 경우 전달
- 탈취·루팅된 최종 사용자 기기의 평문 보호
- 중앙 등록 없는 Sybil 완전 방지
- stable routing slot에 대한 완전한 메타데이터 익명성
- Signal Double Ratchet 수준의 지속적 forward secrecy

## 3. Trust boundary

```text
Trusted:
- local UI process after device unlock
- Rust core binary built from verified source
- Android Keystore implementation within platform assumptions
- in-person verified contact public keys

Untrusted:
- all BLE peers
- relay storage
- radio channel
- imported files/QR before validation
- wall clock
- external diagnostic recipient
```

## 4. 키 종류

| 키 | 용도 | 저장 |
|---|---|---|
| Ed25519 identity key | message signature | encrypted key blob; wrapping key in Keystore |
| X25519 HPKE key | recipient encryption | encrypted key blob; wrapping key in Keystore |
| X25519 Noise static key | link authentication only | encrypted key blob |
| DB master key 256-bit | local plaintext-at-rest | wrapped by Keystore AES key |
| ephemeral HPKE sender key | message seal | memory only, immediate zeroize |
| Noise session keys | link frames | memory only |

Android Keystore가 Ed25519/X25519를 직접 모든 지원 OS에서 일관되게 제공한다고 가정하지 않는다. Rust가 private key를 생성하고, raw private key blob을 random DB master key로 암호화하며 DB master key만 Keystore AES-GCM key로 wrap하는 방식을 기준으로 한다.

## 5. Local key wrapping

1. Android Keystore에 non-exportable AES-256 key alias `dm_local_wrap_v1` 생성
2. Rust CSPRNG로 32-byte DB master key 생성
3. Keystore AES-GCM으로 master key wrap
4. wrapped key, IV, key version만 preferences/DB header에 저장
5. private keys와 평문 message rows는 master key에서 HKDF로 분리한 subkey로 XChaCha20-Poly1305 암호화

subkey context:

```text
DisasterMesh/local/private-keys/v1
DisasterMesh/local/messages/v1
DisasterMesh/local/diagnostics/v1
```

Keystore invalidation 시 자동 새 identity 생성하지 않는다. 복구 불가 상태를 사용자에게 알리고 encrypted relay data만 삭제 가능하게 한다.

## 6. Contact Card

QR payload는 아래 ASCII 형식으로 고정한다.

```text
DM1:<RFC9285-BASE45-UPPERCASE>~<CRC32C-LOWERHEX-8>
```

- Base45 입력은 exact deterministic CBOR contact-card bytes
- checksum은 같은 CBOR bytes의 CRC32C
- 전체 QR text 최대 512 ASCII bytes
- `~`는 Base45 alphabet에 포함되지 않는 유일한 checksum separator다.
- prefix, separator 수, Base45 alphabet, checksum 길이가 다르면 decode 전에 거부

내용:

- protocol version
- Ed25519 public key
- X25519 HPKE public key
- inbound routing slot
- optional display name
- key version
- card signature

card signature input:

```text
UTF8("DisasterMesh/CONTACT/1") ||
SHA256(contact_card_without_signature_cbor)
```

검증:

- 길이/CBOR/CDDL
- self-signature
- display ID/checksum
- duplicate/key change
- 화면 안전번호 비교

QR import는 항상 `UNVERIFIED`다. 대면 확인 상태 전환은 별도 core API가 안전번호를
재계산하고 사용자가 확인한 표시값과 일치시킨 뒤 수행한다.

안전번호:

```text
SHA256(
  UTF8("DisasterMesh/SAFETY/1") ||
  min(A identity pub, B identity pub) ||
  max(A identity pub, B identity pub)
)
```

앞 60 bits를 RFC 4648 Base32 12글자로 표시하고 `XXXX-XXXX-XXXX`로 묶는다.
padding은 쓰지 않는다. 양쪽 앱이 동일한 public-key 정렬 규칙을 써야 한다.

## 7. Message security

- HPKE Base는 recipient confidentiality를 제공한다.
- Ed25519 signature는 sender authentication을 제공한다.
- sender identity/signature는 ciphertext 내부라 relay에 노출되지 않는다.
- AAD가 destination, lifetime, priority, packet ID, immutable `hop_limit`을 묶는다.
- copy token, hop count, age는 mutable이라 AAD에서 제외한다.
- receipt/cancel target과 sender identity도 ciphertext 내부이므로 v1 relay는 control
  message를 검증하거나 대상 원본을 삭제하지 않는다.

### Key compromise

- recipient long-term X25519 private key가 유출되면 기록된 과거 HPKE ciphertext가 위험할 수 있다.
- v1은 이를 명시한다.
- v1.1에서 signed prekey/one-time prekey 또는 검증된 ratchet 도입을 별도 ADR로 평가한다.

## 8. Link security

Noise XX는 연결 시 양쪽 static link key를 교환하고 forward-secret transport key를 만든다. 하지만 처음 만난 relay의 static key를 사전에 신뢰하지 않으므로 링크는 기밀성/무결성 채널이지 사람 identity 인증이 아니다.

- peer link key fingerprint는 encounter history에 hash로 저장
- 갑작스러운 key 변화는 진단 이벤트일 뿐 메시지 contact key 변화와 혼동하지 않는다.
- Noise failure 상세는 remote에 노출하지 않는다.

## 9. 공격과 대응

| 공격 | 대응 | 잔여 위험 |
|---|---|---|
| BLE sniffing | Noise + payload E2EE | traffic timing/size 노출 |
| relay DB 탈취 | ciphertext only | destination slot/priority 노출 |
| payload 변조 | CRC, hash, HPKE AEAD, signature | drop 가능 |
| replay | packet ID, tombstone, sender sequence | storage pressure |
| fake identity | in-person QR/safety number | 사용자의 잘못된 신뢰 |
| Sybil | peer/source quota, no public chat | 완전 차단 불가 |
| bundle/direct-slot flood | size/TTL/rate/unverified-direct quota | P0 위장 메타데이터 |
| priority inflation | encrypted signed inner value, peer quota, protected pool 제한 | relay는 검증 전 우선 처리 가능 |
| selective drop | multiple copies/receipts | 보장 불가 |
| malformed parser | size/depth limits, fuzzing | 구현 버그 |
| key substitution QR | self-signature + safety compare | QR만 원격 전달 시 MITM |
| log leakage | structured redaction | 사용자 screenshot/export |
| DB rollback | contact별 sequence window, message IDs | 완전한 secure monotonic counter 없음 |

## 10. Logging rules

절대 기록 금지:

- plaintext body
- private keys
- full contact public keys
- exact latitude/longitude
- full routing slot
- HPKE decrypted bytes

허용:

- packet ID 앞 6 bytes hash 표기
- peer link fingerprint 앞 6 bytes
- error category
- size, duration, counts
- coarse mode/battery bucket

## 11. Secure coding rules

- Rust `unsafe`는 FFI boundary 이외 금지; 사용 시 ADR/리뷰
- secret type에 `Debug` 구현 금지
- zeroize 가능한 buffer 사용
- parsing 전에 allocation upper bound 확인
- crypto error를 `InvalidCiphertext` 하나로 축약
- random은 OS CSPRNG만
- nonce 직접 증가/재사용 설계 금지; HPKE/Noise library가 관리
- ACK 유실 후 uncertain token grant를 다른 peer에게 재사용 금지
- 테스트 전용 고정 key/nonce는 production feature flag에서 컴파일 불가

## 12. Local encrypted-value envelope

민감 column은 다음 versioned binary envelope를 사용한다.

```text
magic "DMEV" (4) | version u8=1 | key_version u16-be | nonce24 | ciphertext_and_tag
```

- primitive: XChaCha20-Poly1305
- per-column key: `HKDF-SHA256(db_master_key, salt=identity_hash, info="DisasterMesh/DB/1" || table || 0x00 || column || 0x00 || key_version)`
- AAD: deterministic CBOR `[schema_version, table_name, column_name, primary_key_bytes, key_version]`
- 같은 nonce/key 조합 재사용 금지; nonce는 CSPRNG 24 bytes
- row/column 간 ciphertext 복사는 AAD mismatch로 실패해야 한다.
- decrypt failure는 빈 값으로 대체하지 않고 row를 `CORRUPT_ENCRYPTED_VALUE`로 격리한다.
- key rotation은 새 key_version으로 re-encrypt 후 transaction commit, 이전 wrapped key는 전체 migration 검증 후 폐기한다.
- DB가 있고 Keystore/wrapped master key가 없으면 자동 초기화하지 않는다. read-only recovery 안내와 명시적 reset만 제공한다.

## 13. Contact capabilities and display-name safety

Contact card의 `capabilities`는 서명 대상이다. bit registry는 `contracts/protocol_constants.toml`에 고정하며 reserved bit는 송신 시 0, 수신 시 ignore-but-preserve 하지 않고 reject한다. 표시 이름은 NFC normalization 후 저장하고 bidi override/isolate control, NUL, 비표시 제어문자를 제거한다. 네트워크 신뢰 판단에는 표시 이름을 사용하지 않는다.

## 14. Mobile security verification baseline

상용 release는 OWASP MASVS의 STORAGE, CRYPTO, AUTH, NETWORK, PLATFORM, CODE, RESILIENCE, PRIVACY 범주를 `docs/20-security-verification-plan.md`에 매핑한다. 체크리스트 자체가 보안 인증을 의미하지 않으며 외부 리뷰 결과와 재현 증거를 release artifact로 보존한다.

## 15. 보안 출시 게이트

- protocol test vector 공개
- cargo-fuzz 최소 24시간 campaign 결과
- QR/parser/CBOR/BPv7 fuzz corpus
- dependency advisory 0 critical/high 또는 문서화된 예외
- 외부 crypto/protocol review
- threat model 최신화
- known limitation UI 반영
- 취약점 제보 이메일/SECURITY.md
