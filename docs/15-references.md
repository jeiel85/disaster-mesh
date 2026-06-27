# 15. Verified References

확인일: 2026-06-27

## DTN and wire formats

- RFC 9171 — Bundle Protocol Version 7  
  https://www.rfc-editor.org/rfc/rfc9171.html
- RFC 9172 — Bundle Protocol Security  
  https://www.rfc-editor.org/rfc/rfc9172.html
- RFC 9173 — Default Security Contexts for BPSec  
  https://www.rfc-editor.org/rfc/rfc9173.html
- RFC 9713 — BPv7 Administrative Record Types Registry update  
  https://www.rfc-editor.org/rfc/rfc9713.html
- RFC 9758 — Updates to the `ipn` URI Scheme  
  https://www.rfc-editor.org/rfc/rfc9758.html
- RFC 8949 — Concise Binary Object Representation; deterministic encoding requirements  
  https://www.rfc-editor.org/rfc/rfc8949.html
- RFC 4648 — Base-N Encodings  
  https://www.rfc-editor.org/rfc/rfc4648.html
- RFC 8610 — Concise Data Definition Language  
  https://www.rfc-editor.org/rfc/rfc8610.html
- RFC 9180 — Hybrid Public Key Encryption  
  https://www.rfc-editor.org/rfc/rfc9180.html
- RFC 9285 — Base45 Data Encoding  
  https://www.rfc-editor.org/rfc/rfc9285.html

## Link security

- The Noise Protocol Framework, Revision 34  
  https://noiseprotocol.org/noise.html

## Android

- Bluetooth permissions  
  https://developer.android.com/develop/connectivity/bluetooth/bt-permissions
- Communicate in the background with BLE  
  https://developer.android.com/develop/connectivity/bluetooth/ble/background
- Foreground service types required on Android 14+  
  https://developer.android.com/about/versions/14/changes/fgs-types-required
- Data transfer background options / connected device foreground service  
  https://developer.android.com/develop/background-work/background-tasks/data-transfer-options
- Android 16/API 36 behavior changes  
  https://developer.android.com/about/versions/16/behavior-changes-16
- Android 17/API 37 stable release  
  https://developer.android.com/about/versions/17/blog-release
- BluetoothLeScanner PendingIntent scan API  
  https://developer.android.com/reference/android/bluetooth/le/BluetoothLeScanner
- Google Play target API requirements  
  https://developer.android.com/google/play/requirements/target-sdk

## Apple future implementation

- Core Bluetooth  
  https://developer.apple.com/documentation/corebluetooth
- Core Bluetooth Background Processing for iOS Apps  
  https://developer.apple.com/library/archive/documentation/NetworkingInternetWeb/Conceptual/CoreBluetooth_concepts/CoreBluetoothBackgroundProcessingForIOSApps/PerformingTasksWhileYourAppIsInTheBackground.html
- Configuring background execution modes  
  https://developer.apple.com/documentation/xcode/configuring-background-execution-modes

## Implementation references

- dtn7/bp7-rs primary repository  
  https://github.com/dtn7/bp7-rs
- Mozilla UniFFI primary repository  
  https://github.com/mozilla/uniffi-rs
- UniFFI user guide  
  https://mozilla.github.io/uniffi-rs/

## Current toolchain reference

- Android 16 is API level 36  
  https://developer.android.com/about/versions/16/behavior-changes-16
- Android 17 is API level 37  
  https://developer.android.com/about/versions/17/blog-release
- Kotlin release process/current stable line  
  https://kotlinlang.org/docs/releases.html
- Android Gradle Plugin roadmap  
  https://developer.android.com/build/releases/gradle-plugin-roadmap

## Interpretation notes

- BPv7 defines bundle format and store-carry-forward behavior but leaves route selection and convergence-layer choice to the implementation.
- Android supports background BLE use cases, but process lifetime and foreground-service restrictions must be handled explicitly.
- iOS Core Bluetooth background modes do not imply unlimited always-on execution; behavior must be tested and documented by state.
- HPKE and Noise are building blocks. Using a conforming library does not replace a full protocol/security review.

## Mobile application security

- OWASP Mobile Application Security Verification Standard (MASVS)  
  https://mas.owasp.org/MASVS/
- OWASP Mobile Application Security Testing Guide (MASTG)  
  https://mas.owasp.org/MASTG/

## Release-time verification rule

Target SDK, Play location/background policy, Android behavior 문서는 변경 가능성이 있으므로 설계 문서의 숫자를 그대로 신뢰하지 않는다. 각 release candidate에서 공식 페이지의 최신 요구사항을 확인하고 검증 날짜와 결과를 release evidence에 기록한다.
