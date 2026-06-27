# 19. Operational Readiness and Field Runbook

## 1. Health model

앱은 다음 health state를 사용자와 운영자에게 표시한다.

- READY: 권한/Bluetooth/service/DB/key 정상
- DEGRADED: scan-only, advertise-only, low battery, thermal throttle, OEM restriction
- BLOCKED: permission denied, Bluetooth off, storage full, unsupported migration
- RECOVERY_REQUIRED: DB corruption, Keystore key loss, invariant failure

“켜짐”만 표시하지 말고 마지막 정상 scan/advertise/contact 시각과 중단 원인 code를 보여준다. 상대 수나 메시지 수는 잠금화면 public notification에 표시하지 않는다.

## 2. Fixed relay daily checklist

- 전원/배터리/열 상태
- Bluetooth와 persistent notification
- READY/DEGRADED/BLOCKED 상태
- storage quota와 cleanup 실패
- 마지막 scan/advertise activity
- OS update/reboot 이력

본문, contact name, exact destination은 운영 UI에서 볼 수 없다.

## 3. Recovery matrix

| Failure | Automatic action | User/operator action | Data rule |
|---|---|---|---|
| process kill | committed queue reopen | 없음 | loss 0 |
| reboot | age checkpoint validation | relay resume 확인 | invalid age fail-closed |
| DB busy/full | stop ingress, cleanup | storage 확보 | existing protected data 유지 |
| DB corruption | read-only copy, quick_check evidence | export/reset 선택 | overwrite 금지 |
| Keystore loss | engine open 차단 | reset 안내 | decrypt fallback 금지 |
| GATT repeated failure | jitter cooldown | OEM guidance | queued bundles 유지 |
| permission revoke | service BLOCKED | permission 재승인 | 데이터 유지 |
| incompatible schema | downgrade 차단 | supported version 설치/export | destructive auto-migration 금지 |

## 4. Rollout/rollback

- closed field cohort에서 최소 7일 또는 정의된 접촉 횟수 검증.
- production staged rollout의 각 단계는 crash가 아니라 local diagnostic export와 support signal도 검토한다.
- rollback 가능 여부를 DB migration 전에 결정한다. irreversible migration이면 이전 binary 실행을 명시적으로 차단한다.
- protocol major rollback은 peer mixed-version test 없이는 허용하지 않는다.

## 5. Incident severity

- SEV-0: key/plaintext leakage, signature bypass, destructive data loss — 배포 즉시 중단.
- SEV-1: relay/receipt correctness defect, widespread blocked state — rollout halt and hotfix.
- SEV-2: OEM-specific degradation with workaround — known issue/update.
- SEV-3: cosmetic/documentation.

각 incident는 timeline, affected versions, detection gap, user action, fix vector, regression test를 기록한다.
