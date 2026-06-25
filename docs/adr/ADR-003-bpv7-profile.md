# ADR-003: Constrained BPv7 Profile

Status: Accepted

## Decision

BPv7 전체 daemon을 앱에 넣지 않고 RFC 9171 wire format의 제한 프로파일 `DM-BP7-1`을 사용한다.

## Included

- primary block
- bundle age
- hop count
- payload
- private routing block type 192
- CRC32C

Wire invariants:

- Payload Block number is always 1.
- Bundle Age, Hop Count, private routing blocks use numbers 2, 3, 4.
- RFC 9171 outer bundle indefinite-length array is allowed; DME/private structures remain definite-length canonical CBOR.

## Excluded

- fragmentation
- BP status reports
- custody transfer
- full BPSec in v1
- TCPCL

## Consequence

- 완전한 외부 BPA interoperability를 주장하지 않는다.
- bp7-rs 사용 여부와 관계없이 golden vectors/conformance tests를 유지한다.
