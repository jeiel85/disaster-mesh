# Support Policy

## Support boundary

지원 범위는 설치, 권한, Bluetooth 상태, 연락처 키 교환, 진단 export, 업데이트, 데이터 초기화와 알려진 기기 제약 안내다. 지원은 메시지 전달, 구조 기관 접수 또는 대응을 보장하지 않는다.

## Required public information before launch

첫 production release 전에 다음 값이 store listing, 앱의 도움말, release note에서 동일해야 한다.

- 지원 채널과 운영 주체
- 지원 언어 및 운영 시간
- 현재 지원 Android/API 및 검증 OEM/device matrix
- 지원 중인 app/protocol version
- 데이터 복구 불가 조건
- security report와 일반 support의 분리된 경로

운영 주체와 연락처가 정해지지 않은 상태에서는 public production rollout을 승인하지 않는다.

## Data minimization in support

사용자에게 다음 자료를 요청하지 않는다.

- 메시지 본문 또는 정확한 위치
- identity/private keys 또는 safety number 전체
- raw database, relay bundle, partial transfer file
- 다른 사용자의 contact card without consent

`export_diagnostics`가 생성한 redacted ZIP과 앱/OS/device/version 정보만 기본 자료로 사용한다. 추가 자료가 정말 필요하면 목적, 보존 기간, 삭제 방법과 대안을 먼저 설명한다.

## Severity

| Severity | Example | Initial response target |
|---|---|---:|
| S0 | key/plaintext exposure, destructive data loss, unsafe SOS claim | same business day |
| S1 | widespread send/receive failure on supported devices | 1 business day |
| S2 | limited device/UX defect with workaround | 3 business days |
| S3 | question, cosmetic issue, unsupported configuration | 5 business days |

## Unsupported cases

- rooted/modified OS에서의 key confidentiality 보장
- OS가 Bluetooth를 강제로 끄거나 vendor가 background execution을 차단한 상태
- 사전 key 교환 없이 임의 수신자 검색
- 주변 relay path가 전혀 없는 환경
- 삭제되었거나 Keystore에서 복구 불가능해진 설치 identity
