# /goal 3 — Android Direct BLE

```text
/goal
Build the Android direct BLE transport and minimal direct-messaging UI. Implement the exact UUIDs, 31-byte legacy advertisement layout, service-UUID-only fallback, ephemeral beacon rotation, optional-beacon central-role arbitration, GATT server/client characteristics, MTU fallback, outer segmentation, timeouts and error categories from BLE-CLA v1. Keep Android as a raw transport adapter: implement VERSION_HELLO, Noise_XX_25519_ChaChaPoly_BLAKE2s, encrypted frame state and protocol parsing inside Rust. Add onboarding, Bluetooth permission flow, QR contact import/export, contacts, one-to-one text composition, message state display and local persistence. The offlineRelease manifest must not contain INTERNET. Test on at least two physical Android devices with data, Wi-Fi and SIM connectivity unavailable. Complete only when the advertisement cannot exceed 31 bytes, fallback behavior is tested, direct E2EE transfer and receipt work, a packet capture cannot reveal plaintext, Bluetooth/permission loss does not crash, and interrupted transfers never create committed corrupt bundles.
```
