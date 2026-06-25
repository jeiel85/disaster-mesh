# ADR-008: Endpoint-Only Receipt and Cancel Processing

Status: Accepted

## Decision

DELIVERY_RECEIPT와 CANCEL은 일반 DME처럼 최종 수신자에게 HPKE 암호화한다.
Relay는 outer class/priority만 보고 전달하며 target packet, sender identity,
signature를 해석하거나 검증하지 않는다.

## Rationale

target ID와 identity를 relay가 검증할 수 있게 공개하면 기존 metadata 최소화 목표와
충돌한다. 암호문 내부에 유지하면서 relay 삭제까지 요구하는 것은 구현 불가능하다.

## Consequence

- receipt를 받은 원 발신 endpoint만 자신의 원본을 delivered/tombstone 처리한다.
- cancel을 받은 최종 수신 endpoint만 sender/target을 검증하고 UI를 취소 상태로 바꾼다.
- 중간 relay의 기존 원본 copy는 TTL/일반 eviction까지 남을 수 있다.
