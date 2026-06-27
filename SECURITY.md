# Security Policy

## Supported versions

보안 수정은 현재 production release와 바로 이전 minor release에 제공한다. protocol-major 호환을 안전하게 유지할 수 없는 취약점은 peer 차단 또는 강제 업데이트 advisory를 동반할 수 있다. 실제 지원 버전 표는 첫 public release에서 채우며, 비어 있는 상태로 public production release를 게시하지 않는다.

## Private vulnerability reporting

공개 issue에 취약점 세부 정보, 실제 사용자 메시지, 위치, key material 또는 DB를 올리지 않는다.

상용 공개 전 repository owner는 다음 중 하나의 **비공개 신고 경로를 실제로 활성화하고 release metadata에 표시해야 한다.**

1. GitHub Private Vulnerability Reporting, 또는
2. 프로젝트 도메인의 전용 security mailbox와 공개 PGP key.

비공개 경로가 검증되지 않으면 `docs/22-go-live-checklist.md`의 release gate가 실패한다. 이 설계 묶음은 존재하지 않는 이메일 주소를 임의로 기재하지 않는다.

## Reporter response targets

| Stage | Target |
|---|---:|
| automated acknowledgement | 1 business day |
| human triage | 3 business days |
| severity and reproduction decision | 7 calendar days |
| remediation plan for confirmed critical issue | 72 hours after confirmation |
| coordinated disclosure date | reporter와 협의, 기본 90일 이내 |

목표 시간은 수정 완료 보장이 아니라 응답·판단 목표다. 재난 안전 또는 key/plaintext 노출 위험은 즉시 escalation한다.

## Scope

우선순위가 높은 신고:

- E2EE plaintext/key compromise
- signature/AAD/replay/cancel bypass
- token inflation 또는 relay quota bypass
- protocol parser memory safety/DoS
- destructive migration 또는 silent identity reset
- diagnostic export redaction failure
- release artifact/signing/supply-chain compromise

## Handling rules

- 합성 데이터와 최소 재현 절차를 우선한다.
- 실제 피해자 데이터 제공을 요구하지 않는다.
- 취약점 record는 접근 제어하고 release evidence에는 민감한 exploit material을 넣지 않는다.
- 수정에는 regression test, affected-version matrix, protocol compatibility 판단, rollback/advisory owner가 포함된다.
- 보안 수정도 서명된 정상 release pipeline을 우회하지 않는다.
