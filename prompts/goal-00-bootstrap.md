# /goal 0 — Repository Bootstrap

```text
/goal
Create the initial production repository for DisasterMesh, an Android-first, serverless disaster DTN messenger. Follow the design bundle exactly. Create a Rust 2024 workspace with mesh-types, mesh-codec, mesh-crypto, mesh-bundle, mesh-routing, mesh-engine, mesh-sim, and one mesh-ffi facade. Create the Android multi-module project under apps/android with applicationId org.disastermesh.android, minSdk 26, compileSdk 37, targetSdk 36, JDK 17, the bootstrap-date stable Kotlin/Compose toolchain, and an offlineRelease variant that does not declare INTERNET permission. Disable Android cloud backup and device transfer using allowBackup, fullBackupContent and dataExtractionRules assertions. Integrate a minimal UniFFI version() call into an Android instrumentation test. Add Rust and Android CI, dependency locking, formatting, lint, test, cargo deny/audit placeholders, and release-manifest permission/backup policy tests. Do not implement Bluetooth, cryptography, messaging UI, or routing behavior yet. Finish only when a clean checkout builds Rust and Android and the instrumentation test calls the Rust facade successfully.
```
