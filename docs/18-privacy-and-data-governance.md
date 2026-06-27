# 18. Privacy and Data Governance

## 1. Data inventory

| Data | Location | Encryption | Network exposure | Retention |
|---|---|---|---|---|
| identity private keys | local DB wrapped blob | Keystore + DMEV envelope | 없음 | identity reset까지 |
| contact public keys/slot | local DB | integrity + sensitive name encrypted | QR와 encrypted link | contact delete까지 |
| local message body/location | local DB | DMEV envelope | recipient HPKE ciphertext만 | user delete/expiry policy |
| relay bundle | local DB | endpoint E2EE ciphertext | BLE peers | TTL/quota/receipt policy |
| peer encounter metric | local DB | pseudonymous hash | export 시 redacted | 기본 14일 |
| diagnostic event | local DB/export | plaintext code only, no content | user-initiated export | 기본 14일 |

## 2. Data processing rules

- 서버 수집, analytics, advertising ID, remote crash upload를 하지 않는다.
- 정확한 위치는 매 전송마다 사용자가 포함을 선택한 경우에만 DME ciphertext에 들어간다.
- relay는 source identity나 body를 복호화하지 않는다.
- display name은 로컬 편의 정보이며 신원 검증 근거가 아니다.
- diagnostic export는 항목 preview, 최대 4 MiB, entry allowlist, ZIP traversal 방지를 적용한다.

## 3. User controls

- conversation 삭제: local plaintext와 UI index 삭제; 이미 relay된 ciphertext 원격 삭제 보장 없음.
- relay cache 삭제: relay bundle/partial/grant terminal data를 정책에 따라 정리.
- contact 삭제: 새 송신 차단; 과거 메시지 처리와 tombstone retention을 명시.
- identity reset: 모든 private key와 contact trust를 파괴하며 되돌릴 수 없음을 2단계 확인.
- uninstall/OS data clear: cloud recovery 없음. 재설치 identity는 새 사용자로 취급.

## 4. Retention defaults

- relay bundle: protocol TTL + terminal cleanup.
- pending cancel/tombstone/grant evidence: 대상 lifetime + 24h, 최대 8일.
- diagnostics/peer metrics: 14일 rolling.
- local conversation: 사용자 삭제 또는 설정된 local retention; 자동 삭제 기본 OFF.
- corrupt/quarantined bytes: 원문 export 금지, 24h 후 삭제하며 hash/code만 유지.

## 5. Privacy policy truth table

Privacy policy와 store Data Safety는 다음 실제 artifact 검증에서 생성한다.

- manifest permissions
- dependency/SBOM의 SDK 목록
- runtime network socket assertion
- exported component 목록
- location code path와 user consent UI
- backup/data extraction configuration
- diagnostic export contents

문구를 코드보다 먼저 확정하지 않는다. release마다 diff를 검토한다.

## 6. Legal review boundary

이 문서는 법률 자문이 아니다. 출시 국가별 개인정보, 위치정보, 소비자 보호, 긴급 서비스 오인, 암호화 수출/배포, 오픈소스 고지 의무를 법률 담당자가 검토하고 결과를 release evidence에 남긴다.
