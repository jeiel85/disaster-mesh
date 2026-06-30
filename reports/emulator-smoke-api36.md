# Android Emulator API 36 Smoke Evidence

Status: **PASS**  
Tested: 2026-06-30T15:52:05+09:00
Source commit: `ab3b973122113ef65be58ce350ba4981722b5656`
Source dirty at test start: **false**  
AVD: `disastermesh-api36-smoke` (`emulator-5554`)  
Platform: Android 16 / API 36 / x86_64 Pixel 6 profile  
Package: `org.disastermesh.android.dev` version `0.2.0-dev`

## Runtime results

- [x] `DevDebug` APK built, debug-signed, installed, and cold-started
- [x] onboarding and limitation copy rendered; API 36 nearby-device/notification grants accepted
- [x] Material 3 home status dashboard, settings/app information, and local contact identity rendered
- [x] settings replayed the safety notice and returned through onboarding to the home screen
- [x] Android system Back returned from contacts to the app home
- [x] completed onboarding remained dismissed after force-stop and process restart
- [x] Keystore/database-backed contact QR survived force-stop and process restart
- [x] foreground standby relay service started with persistent notification and stopped cleanly
- [x] diagnostic preview opened, DocumentsUI saved the ZIP, and the four-entry schema parsed
- [x] `metadata.json` reported API 36 and app version `0.2.0-dev`
- [x] app remained the top resumed activity and the cleared crash buffer had no fatal app crash
- [x] `:app:connectedDevDebugAndroidTest` ran 1/1 test successfully on this AVD

Identity QR SHA-256 (content not retained):
`b27c40ae90a0ce5f0d2463e513b1be8026fcf1c38a204f911ce7fba3b808d646`

## Evidence files

| File | Bytes | SHA-256 |
|---|---:|---|
| `reports/evidence/emulator-api36/01-onboarding.png` | 142496 | `cae05309c44495d343738944b3963cd3fff5d3ac14b9d6670e21a684d24332a9` |
| `reports/evidence/emulator-api36/02-home.png` | 150793 | `944ddf22b06a699a86087aee76ca4710bc8b3910116d5b19cc99bad355850efc` |
| `reports/evidence/emulator-api36/03-relay.png` | 85366 | `d82a9b746a840a6c6543fb05c4f8b78cea7beb75812a79c1d2024664bd5f78d0` |
| `reports/evidence/emulator-api36/04-diagnostics.png` | 83867 | `d60ba5241dbee77ef59c806c3cfceb15601420ec884cc0040173304a1fc3d689` |
| `reports/evidence/emulator-api36/05-settings.png` | 143285 | `342274b74a45a56f99442ac9356af02ace216caf3e3dc8c7a07f1939aad79d4e` |
| `reports/evidence/emulator-api36/result.json` | 1079 | `8dd98f002bb5057978c2918d44b04268b645de16faa891f470e76300153c5fe3` |

## Product-shell additions verified

1. Onboarding completion persists locally and the user can deliberately replay the safety notice
   from settings.
2. The home and settings screens surface Bluetooth, local encrypted-storage, trusted-contact,
   protocol, privacy, and limitation states without implying that physical BLE acceptance passed.
3. System Back still maps feature screens to the app home, and diagnostic copy consistently uses
   the accurate “limited diagnostic” wording.

## Scope boundary

This evidence proves API 36 emulator lifecycle/platform behavior only. It does not prove physical
BLE scan/advertise/GATT/MTU, direct or multi-hop radio transfer, OEM screen-off execution,
battery/thermal behavior, or the physical compatibility matrix. Those release gates remain open.
