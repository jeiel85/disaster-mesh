# 14. Known Limitations

## 사용자에게 반드시 공개할 항목

1. 주변에 앱을 실행하는 중계 기기가 없으면 전달되지 않는다.
2. 메시지는 수분·수시간·수일 후 전달되거나 영원히 전달되지 않을 수 있다.
3. Bluetooth가 꺼지거나 OS가 앱을 중단하면 중계가 멈춘다.
4. 악성 중계기는 메시지를 읽기 어렵지만 조용히 버릴 수 있다.
5. 목적지 slot, 메시지 크기, 우선순위, 접촉 시각 등 일부 메타데이터는 노출될 수 있다.
6. v1은 수신자 장기 키 유출 후 과거 암호문에 대한 완전한 forward secrecy를 보장하지 않는다.
7. 취소 메시지는 이미 읽은 내용을 상대 기기에서 강제 삭제하지 않으며, v1 relay에 저장된 기존 copy도 삭제하지 않는다.
8. GPS는 인터넷 없이 동작할 수 있지만 실내·지하·초기 위치에서 실패하거나 느릴 수 있다.
9. iOS 백그라운드 동작은 Android 고정 릴레이와 동일한 지속성을 보장하지 않는다.
10. 이 앱은 공식 긴급 신고·재난 문자·무전망을 대체하지 않는다.
11. 재부팅 시 wall-clock checkpoint가 역행하거나 유실되면 TTL을 과소평가하지 않기
    위해 기존 bundle을 AGE_UNCERTAIN으로 격리하며, 이 경우 전달 가능한 메시지가
    조기 중단될 수 있다.
12. secure monotonic clock이 재부팅 사이에 유지되지 않으므로 탐지되지 않은 수동
    wall-clock 조작까지 TTL이 정확하다고 보장하지 않는다.
13. OS/OEM backup과 device-transfer를 차단하므로 앱 데이터와 identity를 새 기기로
    자동 이전할 수 없다. v1은 private-key export를 제공하지 않는다.

## 엔지니어링 한계

- BLE GATT는 제조사별 차이가 크다.
- 다수 노드가 밀집하면 연결 시도와 광고 충돌이 증가한다.
- 장기 inbound routing slot은 서로 다른 접촉과 destination bundle 사이의 linkability를 남긴다.
- source EID를 메시지별 random으로 만들어도 traffic correlation은 가능하다.
- relay copy token은 전달률과 배터리/저장량 trade-off다.
- ACK 유실로 uncertain 상태가 된 token grant는 보수적으로 재사용하지 않아 전달률이 낮아질 수 있다.
- relay는 outer P0/P1 priority를 인증할 수 없으므로 local protected storage pool을 사용할 수 없다.
- BPv7 constrained profile은 완전한 외부 BPA 상호운용을 목표로 하지 않는다.
- private block type 192는 본 프로젝트 내부 프로파일에 한정한다.

## 금지 마케팅 표현

- “어디서나 반드시 전달”
- “통신망이 없어도 실시간 통화”
- “완전 익명”
- “해킹 불가능”
- “공식 구조 요청 접수”
- “Signal과 동일한 보안”
- “배터리 영향 없음”
