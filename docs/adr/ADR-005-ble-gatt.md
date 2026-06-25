# ADR-005: BLE GATT as Initial Convergence Layer

Status: Accepted

## Decision

양쪽 Android가 scan/advertise와 central/peripheral 역할을 수행하고, deterministic beacon arbitration으로 한 링크를 만든다. Control/Data characteristics와 Noise XX secure session을 사용한다.

## Consequence

- 기기별 GATT quirk와 MTU fallback 필요
- legacy 31-byte 광고는 Flags + 128-bit Service Data(10 bytes)로 제한
- 대용량 파일 제외
- transport adapter로 격리해 향후 Wi-Fi Aware 등을 추가
