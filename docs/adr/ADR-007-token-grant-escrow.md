# ADR-007: Persistent Token Grant Escrow

Status: Accepted

## Decision

Binary Spray-and-Wait relay 전송은 BUNDLE_META 전 receiver token 몫을 persistent
grant ledger에 escrow한다. grant는 random 16-byte ID를 가지며 sender/receiver가
같은 ID로 commit 상태를 reconciliation한다.

## Rationale

ACK 후에만 sender token을 줄이면 receiver commit 뒤 ACK가 유실될 때 sender가 같은
token을 다른 peer에게 다시 복제해 token conservation이 깨진다. 보수적 escrow는
전달률 일부를 희생하지만 복제 상한을 유지한다.

## States

- RESERVED: 전송 전 sender가 token을 escrow
- UNCERTAIN: receiver commit 여부를 알 수 없음
- TRANSFERRED/COMMITTED: receiver ledger에서 같은 grant commit 확인
- RELEASED: receiver가 commit하지 않았음이 명시적으로 확인되어 sender에 복원

## Consequence

- timeout만으로 grant를 release하지 않는다.
- same peer/static-link fingerprint와 재접촉하면 같은 grant ID로 확인한다.
- peer link key가 바뀌거나 다시 만나지 못하면 uncertain token은 만료까지 사용할 수 없다.
- token conservation은 protocol을 따르는 노드에 대한 invariant다. 악성 receiver가
  받은 ciphertext를 임의 복제하는 행위까지 암호학적으로 제한하지는 못한다.
