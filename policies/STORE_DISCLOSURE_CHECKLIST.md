# Store Disclosure Consistency Checklist

release candidate의 실제 manifest, binary, UI와 store 답변을 대조한다. 이전 release의 답변을 복사해 승인하지 않는다.

## Artifact facts

- [ ] `offlineRelease` final merged manifest에 INTERNET/analytics/ads permission/provider가 없다.
- [ ] backup와 device transfer exclusion이 final artifact에 적용된다.
- [ ] location/Bluetooth/notification/foreground-service permission은 기능과 Android API별 설명이 있다.
- [ ] native libraries, SBOM, privacy policy의 data access가 일치한다.
- [ ] debug/test logging, deterministic RNG, test keys가 release artifact에 없다.

## Store and user disclosures

- [ ] 앱이 공식 emergency service가 아니며 delivery를 보장하지 않는다고 명확히 표시한다.
- [ ] 메시지·선택 위치가 recipient와 relay peer를 통해 전달되는 방식을 설명한다.
- [ ] 계정·서버 telemetry·광고·analytics가 없는 현재 edition의 사실과 답변이 일치한다.
- [ ] 사용자가 실행하는 diagnostic export와 외부 공유는 user-initiated flow로 구분한다.
- [ ] 삭제·identity reset·Keystore loss와 복구 불가 조건을 설명한다.
- [ ] 지원 기기/언어/접근성/known limitation이 release note와 일치한다.
- [ ] privacy/security/support URL이 실제로 접근 가능하고 담당자가 응답 시험을 완료했다.

## Approval evidence

| Channel | Artifact SHA-256 | Manifest diff reviewed | Privacy/Data Safety reviewed | Legal owner | Date |
|---|---|---|---|---|---|
| Play |  |  |  |  |  |
| F-Droid/source |  |  |  |  |  |
