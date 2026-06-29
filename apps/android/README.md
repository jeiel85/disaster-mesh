# Android application

Goal 0 establishes the Android module graph, build variants, release manifest
policy, and the single UniFFI bridge. Protocol, routing and SQLite behavior now
live in the Rust core; Bluetooth product behavior and messaging UI remain absent.

Supported bootstrap variants:

- `offlineRelease`: production-shaped, no network permission
- `fieldTestRelease`: controlled field build, no network permission
- `devDebug`: non-production development build

The Rust facade is compiled for all four supported Android ABIs by
`:core-bridge:buildRustAndroid` and packaged into the bridge AAR.
