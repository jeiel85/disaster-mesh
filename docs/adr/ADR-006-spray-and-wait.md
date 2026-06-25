# ADR-006: Binary Spray-and-Wait

Status: Accepted

## Decision

v1 relay routing은 Direct Delivery + Binary Spray-and-Wait로 고정한다.

## Rationale

- epidemic flooding보다 저장·무선 비용이 제한됨
- 중앙 topology 없이 구현 가능
- token invariant를 property-test할 수 있음

## Consequence

- 최적 경로를 보장하지 않음
- token/TTL 값은 simulator와 field exercise로 조정
- relay 전송 전 receiver 몫을 persistent token grant escrow로 예약
- ACK 유실로 uncertain 상태가 된 grant는 reconciliation 전 재사용하지 않음
