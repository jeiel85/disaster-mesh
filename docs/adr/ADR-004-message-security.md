# ADR-004: HPKE Payload Encryption plus Ed25519 Signature

Status: Accepted pending external review

## Decision

DME payload는 RFC 9180 HPKE Base(X25519/HKDF-SHA256/ChaCha20Poly1305)로 recipient에게 암호화하고, 암호문 내부 plaintext에 Ed25519 sender signature를 포함한다.

## Rationale

- 표준화된 hybrid encryption
- relay에게 sender identity를 숨김
- offline contact public key만으로 송신 가능
- cross-platform test vectors 작성 가능

## Limitation

recipient long-term key compromise 후 과거 ciphertext의 완전한 forward secrecy를 보장하지 않는다.

receipt/cancel target과 sender identity도 암호문 내부에 있으므로 v1 relay는 control
message를 검증하거나 대상 원본을 삭제하지 않는다. 최종 endpoint만 이를 처리한다.
