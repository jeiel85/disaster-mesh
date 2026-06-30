# 23. Android Emulator Testing

## 1. Purpose

The repository includes a Windows/PowerShell path that creates an Android 16 (API 36)
x86_64 Pixel 6 AVD, boots it with WHPX acceleration, installs the `devDebug` APK, and
drives a runtime smoke test through ADB and UI Automator.

The smoke test proves app lifecycle and platform integration. It does **not** replace
physical-device BLE acceptance because the standard Android Emulator does not reproduce a
real controller's scan, advertise, GATT, MTU, radio coexistence, screen-off, or OEM behavior.

## 2. Prerequisites

- Windows virtualization enabled and `emulator-check accel` reports WHPX usable
- JDK 17 (`JAVA_HOME`, or a Temurin JDK under `%USERPROFILE%\.local`)
- Android SDK command-line tools, emulator, and platform-tools
- Rust toolchain/Android targets and `cargo-ndk` required by the Android build
- MinGW64 on `PATH` for the configured Windows GNU Rust host

The setup script installs `system-images;android-36;default;x86_64` if it is absent. SDK
licenses must be accepted by the operator.

## 3. Create and boot the AVD

From the repository root:

```powershell
pwsh -File tools/setup_android_emulator.ps1 -Action All
```

The default AVD is `disastermesh-api36-smoke` on `emulator-5554`. It starts headless. To
open the interactive emulator window instead:

```powershell
pwsh -File tools/setup_android_emulator.ps1 -Action Start -Window
```

Other lifecycle commands:

```powershell
pwsh -File tools/setup_android_emulator.ps1 -Action Status
pwsh -File tools/setup_android_emulator.ps1 -Action Stop
pwsh -File tools/setup_android_emulator.ps1 -Action Start -WipeData
```

`-WipeData` is destructive to that AVD's app data and is never the default.

## 4. Build, install, and smoke test

```powershell
pwsh -File tools/smoke_android_emulator.ps1
```

The script performs and asserts all of the following:

1. build and debug-sign `app-dev-debug.apk`, then install it;
2. grant API 36 nearby-device and notification permissions;
3. cold-start the launcher activity and pass onboarding;
4. verify the home and contact identity screens;
5. verify Android system Back returns from contacts to the app home;
6. start the foreground relay service, observe its persistent notification, and stop it;
7. preview and save the diagnostic ZIP through DocumentsUI, then verify its four-entry schema;
8. force-stop/restart the process and verify the Keystore/database-backed contact identity is stable;
9. require DisasterMesh to remain foreground with no fatal app crash in the cleared crash buffer.

By default the smoke run clears only `org.disastermesh.android.dev` data for deterministic
results. Use `-PreserveAppData` only when intentionally testing an existing emulator state.
Use `-SkipBuild` to reuse an already-built APK.

Evidence is written to `reports/evidence/emulator-api36/`: four screenshots and a JSON result
containing the source commit, dirty-state flag, AVD/API/package versions, identity hash, checks,
and explicit physical-BLE limitations.

## 5. Instrumentation test

With the AVD online, run the Rust/UniFFI device instrumentation test as a separate gate:

```powershell
cd apps/android
./gradlew :app:connectedDevDebugAndroidTest --no-parallel
```

Passing emulator evidence may close emulator-scoped lifecycle/platform checks only. Goal 3/4
direct and multi-hop BLE gates, OEM compatibility, and battery/thermal soak remain physical
device evidence.
