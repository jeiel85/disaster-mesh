# 17. Commercial Readiness Baseline

## 1. Release definition

“상용화 가능”은 기능이 동작한다는 뜻이 아니라 다음 다섯 증거가 동시에 존재하는 상태다.

1. protocol/storage 계약이 기계 판독 가능하고 독립 구현 간 호환된다.
2. 사용자 데이터 손실·오인·보안 사고의 실패 경로가 설계되고 시험된다.
3. 배포 artifact, 개인정보 고지, 권한, store metadata가 실제 동작과 일치한다.
4. 운영자가 장애를 진단·중단·rollback·공지할 수 있다.
5. 제품·QA·보안·운영·법률 책임자가 release evidence를 승인한다.

## 2. Product editions

| Edition | Distribution | Network permission | Background behavior |
|---|---|---|---|
| offlineRelease | Play/F-Droid | INTERNET 없음 | 공식 Android 정책 범위의 foreground relay |
| fieldTestRelease | 기관/훈련 controlled | INTERNET 없음 | 별도 정책 승인 하에 API 29–30 background location 가능 |
| devDebug | 개발자 | debug 전용 | synthetic keys/logging, production data 금지 |

build type 간 application ID, signing key, backup rule, log policy를 분리한다. debug artifact가 production DB를 열 수 없도록 key namespace와 DB filename을 분리한다.

## 3. Commercial launch blockers

### P0 — 하나라도 있으면 출시 금지

- plaintext/key/location leakage, signature/AAD bypass, token inflation
- receipt recursion, replay acceptance outside policy, cancel spoofing
- committed message loss, destructive migration, silent Keystore reset
- unsupported protocol peer와 unsafe interoperability
- manifest/privacy declaration mismatch
- 공식 구조 접수 또는 전달 보장으로 오인되는 UX

### P1 — production rollout 전 0건

- supported device에서 반복 가능한 background relay failure
- 접근성으로 SOS 전송/취소가 불가능
- diagnostic export redaction failure
- rollback/runbook 미검증

## 4. Support policy

- 지원 채널, 응답 목표, 지원 언어, 지원 Android/OEM 목록을 release note에 게시한다.
- “지원”은 전달 성공 보장이 아니라 앱 설치·키/권한·진단·데이터 초기화 안내 범위다.
- 실제 사용자 메시지·정확한 위치·개인키를 support ticket에 첨부하도록 요구하지 않는다.
- 데이터 복구 가능성을 과장하지 않는다. Keystore/DB key 손실은 복구 불가일 수 있다.

## 5. Localization and accessibility

- 1.0 필수 locale: Korean, English.
- protocol enum/encoding은 locale과 무관하다.
- TalkBack traversal, dynamic font 200%, contrast, 48dp target, 색상 외 상태, SOS long-press 대체 동작을 실기기 시험한다.
- bidi/control character를 제거하고 사용자 입력은 NFC normalization한다.

## 6. Monetization boundary

1.0 offline edition에는 광고, tracking, account, subscription SDK를 넣지 않는다. 후원/유료 판매를 도입해도 core protocol과 private data path에 third-party SDK를 연결하지 않는다. 결제·라이선스가 필요하면 별도 distribution wrapper ADR과 privacy review가 선행되어야 한다.
