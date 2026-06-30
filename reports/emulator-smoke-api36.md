# Android Emulator API 36 Smoke Evidence

Status: **PASS**  
Tested: 2026-06-30T15:21:26+09:00  
Source commit: `00088fb3e765a055678fe6f7a84bce7a7db65eaf`  
Source dirty at test start: **false**  
AVD: `disastermesh-api36-smoke` (`emulator-5554`)  
Platform: Android 16 / API 36 / x86_64 Pixel 6 profile  
Package: `org.disastermesh.android.dev` version `0.1.0-dev`

## Runtime results

- [x] `DevDebug` APK built, debug-signed, installed, and cold-started
- [x] onboarding and limitation copy rendered; API 36 nearby-device/notification grants accepted
- [x] home and local contact identity screens rendered
- [x] Android system Back returned from contacts to the app home
- [x] Keystore/database-backed contact QR survived force-stop and process restart
- [x] foreground standby relay service started with persistent notification and stopped cleanly
- [x] diagnostic preview opened, DocumentsUI saved the ZIP, and the four-entry schema parsed
- [x] `metadata.json` reported API 36 and app version `0.1.0-dev`
- [x] app remained the top resumed activity and the cleared crash buffer had no fatal app crash
- [x] `:app:connectedDevDebugAndroidTest` ran 1/1 test successfully on this AVD

Identity QR SHA-256 (content not retained):
`a359232cfaf0bf50487a1bdde3e9a0562ea016ae5ffa862d50307495ea10b811`

## Evidence files

| File | Bytes | SHA-256 |
|---|---:|---|
| `reports/evidence/emulator-api36/01-onboarding.png` | 105698 | `72e5488ed4d75808b40cb99336ca9ccc1ad995fd1ac6f63851869cb2ee304a64` |
| `reports/evidence/emulator-api36/02-home.png` | 81381 | `c780e8ae4d6c87333d1f5af11192f1dd64211a742e601891ff605f37a69b79f0` |
| `reports/evidence/emulator-api36/03-relay.png` | 82535 | `13038f77cca3fb90ad8c168c1d32537771529f0a080d3a3f144a068edd3318b1` |
| `reports/evidence/emulator-api36/04-diagnostics.png` | 80684 | `65696651f61a799ec381f3b90eac24b96d664834d59e4a407fb4926cd7e96de9` |
| `reports/evidence/emulator-api36/result.json` | 959 | `da55cf8f521702596df2a04382007add7155e388978b683b3fbfbdb55172d05e` |

## Findings closed during the run

1. System Back from the contacts screen exited the Activity. `BackHandler` now maps feature
   screens back to the app home (conversation maps to contacts), and the VM regression passed.
2. The diagnostic UI claimed anonymity while exporting device metadata. Home, preview, archive
   copy, and the policy consistency gate now use the accurate “limited diagnostic” wording.

## Scope boundary

This evidence proves API 36 emulator lifecycle/platform behavior only. It does not prove physical
BLE scan/advertise/GATT/MTU, direct or multi-hop radio transfer, OEM screen-off execution,
battery/thermal behavior, or the physical compatibility matrix. Those release gates remain open.
