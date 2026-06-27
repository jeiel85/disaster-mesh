# DisasterMesh Privacy Policy — Release Draft

> **게시 전 필수 입력:** `{{PUBLISHER_LEGAL_NAME}}`, `{{CONTACT_ADDRESS}}`, `{{PRIVACY_CONTACT}}`, 적용 국가·배포 채널·효력일. 이 값이 비어 있으면 production release 금지. 현지 법률 검토 없이 이 초안을 최종 법률 문서로 표시하지 않는다.

## Product data flow

DisasterMesh offline edition은 계정 서버, 광고 SDK, analytics SDK, 원격 crash upload와 `INTERNET` permission 없이 동작하도록 설계된다. 메시지와 선택한 위치는 수신 연락처의 공개키로 암호화되며, 주변 relay는 제한된 암호문 bundle만 저장·전달한다.

## Data stored on the device

- 설치 identity와 암호화 key material
- 사용자가 QR로 추가한 contact card와 trust state
- 작성·수신 메시지, 전달 상태, 선택한 위치
- 암호화된 relay bundles와 partial transfer state
- 본문을 포함하지 않는 제한된 진단 이벤트

민감한 local columns와 private key blob은 Android Keystore로 감싼 master key에서 파생한 key로 암호화한다. 화면 잠금이 해제된 기기, rooted OS 또는 메모리 침해까지 완전하게 방어한다고 주장하지 않는다.

## Data sharing

앱은 사용자의 전송 동작에 따라 Bluetooth로 다음을 공유한다.

- protocol/version/capability와 회전 식별자
- E2EE message bundle
- relay가 전달을 결정하는 데 필요한 최소 routing metadata

relay는 message plaintext를 정상 protocol 경로에서 복호화할 수 없다. recipient는 사용자가 보낸 message와 선택한 위치를 복호화할 수 있다.

## Location

위치는 사용자가 해당 message에서 명시적으로 선택한 경우에만 payload에 포함한다. OS permission과 background Bluetooth/location 정책은 Android 버전에 따라 다르며, 앱은 권한 목적을 요청 전에 설명한다. 위치를 analytics 또는 광고에 사용하지 않는다.

## Diagnostics

자동 원격 수집은 하지 않는다. 사용자가 선택해 redacted diagnostic ZIP을 export할 수 있으며, export 전에 포함 항목을 미리 보여준다. export에는 message plaintext, exact location, keys, raw DB와 bundle bytes가 포함되지 않아야 한다.

## Retention and deletion

message TTL과 relay quota에 따라 암호문 bundle이 자동 삭제될 수 있다. 사용자는 앱 안에서 message/contact/relay data를 삭제하고 설치 identity를 초기화할 수 있다. flash storage와 OS backup의 물리적 완전 삭제를 보장하지 않으며, offline release는 Android backup/transfer를 비활성화한다.

## Network recipients and cross-border transfer

운영자가 제어하는 application server로 데이터를 전송하지 않는다. 사용자가 물리적으로 접촉하는 peer device로 Bluetooth data가 전달될 수 있고, peer owner와 위치는 publisher가 통제하지 않는다. 배포 지역 법률상 이 동작의 고지 방식은 출시 전 법률 검토한다.

## Children, emergency services and safety

이 앱은 공식 긴급 신고 채널이 아니며 구조 요청의 접수·전달·대응을 보장하지 않는다. 아동 대상 서비스로 별도 설계되지 않았다. 배포 지역에서 필요한 연령 고지와 보호자 동의 요건은 publisher가 결정한다.

## Contact and changes

Privacy inquiries: `{{PRIVACY_CONTACT}}`  
Publisher: `{{PUBLISHER_LEGAL_NAME}}`  
Effective date: `{{EFFECTIVE_DATE}}`

중대한 data-flow 변경, INTERNET permission 또는 third-party SDK 도입 시 이 정책, Data Safety/store disclosure, threat model과 사용자 동의를 함께 재검토한다.
