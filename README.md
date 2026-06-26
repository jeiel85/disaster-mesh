<div align="center">

<img src="assets/banner.svg" alt="DisasterMesh — Encrypted Offline Emergency Messaging" width="100%"/>

# DisasterMesh

**종단간 암호화 · 오프라인 우선 · BLE 전용 · 서버 없음 · 인터넷 불필요**

*End-to-end encrypted offline mesh messaging when infrastructure fails*

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-Android%208.0%2B%20%28API%2026%29-3DDC84.svg?logo=android&logoColor=white)](https://developer.android.com)
[![Kotlin](https://img.shields.io/badge/Kotlin-Jetpack%20Compose-7F52FF.svg?logo=kotlin&logoColor=white)](https://kotlinlang.org)
[![Rust](https://img.shields.io/badge/Rust-2024%20Edition-orange.svg?logo=rust&logoColor=white)](https://rust-lang.org)
[![Encryption](https://img.shields.io/badge/Encryption-HPKE%20%2B%20Ed25519-red.svg?logo=letsencrypt&logoColor=white)](docs/06-security-and-threat-model.md)
[![Protocol](https://img.shields.io/badge/Protocol-BPv7%20RFC%209171-informational.svg)](docs/03-protocol-dme-v1.md)
[![Transport](https://img.shields.io/badge/Transport-BLE%20GATT%20Only-blue.svg?logo=bluetooth&logoColor=white)](docs/04-protocol-ble-cla-v1.md)
[![Status](https://img.shields.io/badge/Status-Design%20Complete%20v1.0.1-success.svg)](docs/16-design-review-v1.0.1.md)

[**Landing Page**](https://jeiel85.github.io/disaster-mesh)&nbsp;·&nbsp;[**Specification**](docs/)&nbsp;·&nbsp;[**Architecture**](docs/01-system-architecture.md)&nbsp;·&nbsp;[**Security Model**](docs/06-security-and-threat-model.md)&nbsp;·&nbsp;[**Known Limitations**](docs/14-known-limitations.md)

</div>

---

DisasterMesh is an **Android-first, serverless, offline-first** emergency communication system that works when cellular networks, internet, and infrastructure fail. Using Bluetooth Low Energy (BLE), devices form a self-organizing peer-to-peer mesh network that stores, carries, and relays end-to-end encrypted messages across multiple hops — with no server, no internet, and no plaintext visible to relays.

> **재난 상황에서** 기지국·인터넷·공유기 없이, 주변 스마트폰과 고정 릴레이만으로  
> 종단간 암호화된 재난 메시지를 **저장·운반·전달**하는 Android 우선 오픈소스 시스템.  
> Bluetooth 전용 · 서버 없음 · 중계 노드가 내용을 읽을 수 없음.

---

## Why This Exists

When earthquakes, floods, hurricanes, or power failures strike, cellular networks become congested or go offline entirely — right when people need to communicate about safety, location, and rescue needs. Standard messaging apps stop working. **DisasterMesh continues working:**

| Problem | DisasterMesh |
|---|---|
| Cellular tower is down | BLE radio in your pocket still works |
| No internet connection | No internet permission declared in release APK |
| Server is unreachable | No server — pure peer-to-peer mesh |
| Messages may be intercepted | HPKE + Ed25519 end-to-end encryption |
| Relay may peek at content | Relay nodes only forward ciphertext they cannot decrypt |
| Process killed mid-transfer | SQLite store-and-forward survives restarts |
| SOS drowned in regular traffic | P0 priority + 12 copy tokens for emergency messages |

---

## Features

| Feature | Detail |
|---|---|
| **BLE Mesh Transport** | Android BLE Central + Peripheral (GATT Server), Noise_XX handshake per link |
| **End-to-End Encryption** | RFC 9180 HPKE Base: X25519 / HKDF-SHA256 / ChaCha20Poly1305 |
| **Sender Authentication** | Ed25519 signatures embedded inside ciphertext |
| **Multi-hop Routing** | Binary Spray-and-Wait with persistent token grant escrow |
| **SOS Priority (P0)** | Highest priority queue, 12 copy tokens, 16 hop limit, 24h TTL |
| **Store and Forward** | SQLite persistence survives process kills; delivers when peers meet later |
| **Privacy by Default** | No INTERNET permission in release; relay nodes see only ciphertext |
| **Contact Verification** | QR code in-person exchange with Ed25519 public keys |
| **Delivery Receipts** | Signed receipts route back to sender through the mesh |
| **No Dependencies** | Pure BLE; no TCP, no UDP, no Wi-Fi, no cloud |

---

## Architecture

```
┌──────────────────── Android App (Kotlin) ───────────────────────┐
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │           Jetpack Compose UI Screens                        │ │
│  │  SendMessage · CheckIn · SOS · ContactBook · RelayStatus    │ │
│  └─────────────────────────┬───────────────────────────────────┘ │
│                             │                                     │
│  ┌──────────────┐  ┌────────▼──────────┐  ┌────────────────────┐ │
│  │  Foreground  │  │   MeshCoordinator │  │   BlePlatformAdap  │ │
│  │   Service    │  │   (Kotlin bridge) │  │   Central/Periph   │ │
│  └──────────────┘  └────────┬──────────┘  └────────────────────┘ │
│                             │  UniFFI FFI                         │
├─────────────────────────────┼───────────────────────────────────-┤
│                    ┌────────▼──────────┐                          │
│                    │    MeshEngine     │  (Rust 2024)             │
│  ┌─────────────────┴─────────────────────────────────────────┐   │
│  │  mesh-types  │  mesh-codec  │  mesh-crypto  │  mesh-bundle │   │
│  │  mesh-routing│  mesh-store  │  mesh-engine  │  mesh-sim    │   │
│  │                          mesh-ffi                          │   │
│  └────────────────────────────────────────────────────────────┘   │
│                                                                   │
└────────────────┬──────────────────────────────┬──────────────────┘
                 │                              │
          ┌──────▼───────┐             ┌────────▼────────┐
          │    SQLite    │             │  Android        │
          │  (Rust owns) │             │  Keystore       │
          └──────────────┘             └─────────────────┘
```

**Key principle:** Rust owns everything below the FFI boundary — protocol encoding, cryptography, routing decisions, and the database. Kotlin handles only platform surfaces: BLE radio, UI, and key-wrapping via Android Keystore.

---

## Tech Stack

| Layer | Technology | Rationale |
|---|---|---|
| **UI** | Kotlin + Jetpack Compose | Modern Android-native declarative UI |
| **Core** | Rust 2024 Edition | Memory safety, deterministic codec, no GC pauses |
| **FFI** | UniFFI (single facade crate) | Type-safe Rust ↔ Kotlin binding |
| **Transport** | Android BLE GATT Central + Peripheral | Sole transport; zero internet permission in release |
| **Link Security** | Noise_XX_25519_ChaChaPoly_BLAKE2s | Mutual auth + forward secrecy per BLE session |
| **Message Encryption** | RFC 9180 HPKE Base: X25519/HKDF-SHA256/ChaCha20Poly1305 | Asymmetric E2EE; relay sees only ciphertext |
| **Authentication** | Ed25519 signatures | Compact, fast, embedded in ciphertext |
| **Bundle Protocol** | BPv7 (RFC 9171) — DM-BP7-1 profile | DTN standard; store-and-forward semantics |
| **Serialization** | Deterministic CBOR (RFC 8949) | Compact binary; canonical form for signature coverage |
| **Schema** | CDDL | Machine-verifiable protocol schema |
| **Routing** | Binary Spray-and-Wait + Direct Delivery | Proven DTN algorithm with copy-token escrow |
| **Storage** | SQLite — Rust-owned | Persistent across restarts; encrypted master key |
| **Key Storage** | Android Keystore AES-256 | DB master key never leaves secure enclave |

---

## Message Types

| Type | Priority | TTL | Copy Tokens | Max Payload |
|---|---|---|---|---|
| `PRIVATE_SOS` | **P0** | 24 h | 12 | 7,800 bytes |
| `DELIVERY_RECEIPT` | **P0** | 7 d | — | — |
| `CANCEL` | **P0** | 7 d | — | — |
| `CHECK_IN` | P1 | 48 h | 8 | 7,800 bytes |
| `LOCATION_UPDATE` | P1 | 24 h | 6 | — |
| `DIRECT_TEXT` | P2 | 72 h | 6 | 7,800 bytes |

> P0 messages are always scheduled before P1, P1 before P2. The relay queue enforces this at every BLE transfer opportunity.

---

## Project Structure

```
disaster-mesh/
│
├── docs/                          # 17 technical specification documents
│   ├── adr/                       # 8 locked Architectural Decision Records
│   │   ├── ADR-001-android-first.md
│   │   ├── ADR-002-rust-owns-protocol-db.md
│   │   ├── ADR-003-bpv7-profile.md
│   │   ├── ADR-004-message-security.md
│   │   ├── ADR-005-ble-gatt.md
│   │   ├── ADR-006-spray-and-wait.md
│   │   ├── ADR-007-token-grant-escrow.md
│   │   └── ADR-008-endpoint-only-control.md
│   ├── 00-product-requirements.md
│   ├── 01-system-architecture.md
│   ├── 02-domain-model.md
│   ├── 03-protocol-dme-v1.md
│   ├── 04-protocol-ble-cla-v1.md
│   ├── 05-routing-and-queue.md
│   ├── 06-security-and-threat-model.md
│   ├── 07-storage-schema.md
│   ├── 08-rust-core-contract.md
│   ├── 09-android-implementation.md
│   ├── 10-state-machines.md
│   ├── 11-testing-and-acceptance.md
│   ├── 12-release-and-operations.md
│   ├── 13-development-goals.md
│   ├── 14-known-limitations.md
│   ├── 15-references.md
│   ├── 16-design-review-v1.0.1.md
│   └── index.html                 # Landing page (GitHub Pages)
│
├── spec/                          # CDDL schema definitions
│   ├── dme-v1.cddl                # Disaster Message Envelope
│   ├── ble-control-v1.cddl        # BLE control frames
│   ├── contact-card-v1.cddl       # QR contact exchange
│   └── disaster-routing-block-v1.cddl
│
├── schemas/
│   └── sqlite_v1.sql              # Initial SQLite schema (19 tables)
│
├── contracts/                     # Platform-agnostic interface stubs
│   ├── protocol_constants.toml    # BLE UUIDs, timeouts, TTLs, limits
│   ├── rust_facade.rs             # Rust engine API sketch
│   └── android_interfaces.kt      # Kotlin coordinator/adapter interfaces
│
├── prompts/                       # Implementation goal prompts
│   ├── goal-00-bootstrap.md
│   ├── goal-01-protocol-core.md
│   ├── goal-02-security.md
│   ├── goal-03-android-direct-ble.md
│   ├── goal-04-multihop.md
│   ├── goal-05-disaster-product.md
│   └── goal-06-hardening.md
│
├── test-vectors/                  # Cryptographic test vector specs
├── assets/
│   └── banner.svg                 # Project banner image
├── IMPLEMENTATION_CHECKLIST.md
├── LICENSE                        # Apache 2.0
└── README.md
```

---

## Documentation

| # | Document | Description |
|---|---|---|
| 00 | [Product Requirements](docs/00-product-requirements.md) | 18 FR + 12 NFR with acceptance criteria |
| 01 | [System Architecture](docs/01-system-architecture.md) | Module boundaries, Rust/Android separation |
| 02 | [Domain Model](docs/02-domain-model.md) | IDs, entities, aggregates, invariants |
| 03 | [Protocol: DME v1](docs/03-protocol-dme-v1.md) | BPv7 profile, DME envelope, HPKE, Ed25519 |
| 04 | [Protocol: BLE CLA v1](docs/04-protocol-ble-cla-v1.md) | GATT UUIDs, frames, Noise handshake |
| 05 | [Routing & Queue](docs/05-routing-and-queue.md) | Spray-and-Wait, token escrow, TTL/hop rules |
| 06 | [Security & Threat Model](docs/06-security-and-threat-model.md) | Threats, mitigations, key management, release gates |
| 07 | [Storage Schema](docs/07-storage-schema.md) | 19-table SQLite schema, transactions, encryption |
| 08 | [Rust Core Contract](docs/08-rust-core-contract.md) | FFI API signatures, engine commands, event model |
| 09 | [Android Implementation](docs/09-android-implementation.md) | Manifest, BLE permissions, module structure |
| 10 | [State Machines](docs/10-state-machines.md) | Service, link, transfer, message lifecycle FSMs |
| 11 | [Testing & Acceptance](docs/11-testing-and-acceptance.md) | Unit, integration, real-device, security test matrix |
| 12 | [Release & Operations](docs/12-release-and-operations.md) | CI gates, field relay setup, incident response |
| 13 | [Development Goals](docs/13-development-goals.md) | 7-phase implementation plan with completion criteria |
| 14 | [Known Limitations](docs/14-known-limitations.md) | 13 public limitations and forbidden marketing claims |
| 15 | [References](docs/15-references.md) | Verified primary sources for all standards cited |
| 16 | [Design Review v1.0.1](docs/16-design-review-v1.0.1.md) | Multi-agent adversarial review — zero start-blockers |

---

## Implementation Roadmap

| Goal | Focus | Key Completion Test |
|---|---|---|
| **Goal 0** | Rust workspace · Android modules · CI · no logic | `cargo test` passes; instrumentation test invokes Rust facade |
| **Goal 1** | Types · CBOR codec · routing · 100-node simulator | A→B→C simulated delivery; token conservation verified |
| **Goal 2** | Identity · HPKE · Ed25519 · QR contact · test vectors | Golden cryptographic test vectors pass |
| **Goal 3** | Direct BLE transfer · GATT · Noise handshake | Two physical Android devices exchange E2EE message |
| **Goal 4** | Multi-hop relay · token escrow · ACK recovery · receipts | 50× A→B→C cycles; B cannot decrypt payload |
| **Goal 5** | Check-in/SOS UX · battery · foreground service · relay mode | 8h battery report; process kill recovery; thermal test |
| **Goal 6** | Fuzz targets · SBOM · external audit · beta packaging | External review clearance; Play Store / F-Droid ready |

**Current status:** Design review complete (v1.0.1) — zero start-blockers. Goal 0 is next.

---

## Architectural Decisions

Eight locked ADRs define the constraints that everything else is built around:

| ADR | Decision | Rationale |
|---|---|---|
| [ADR-001](docs/adr/ADR-001-android-first.md) | Android first; iOS/Linux relay in v1.1 | Maximize initial reach on single platform |
| [ADR-002](docs/adr/ADR-002-rust-owns-protocol-db.md) | Rust core owns protocol, crypto, and SQLite | Single source of truth; no Kotlin/Rust drift |
| [ADR-003](docs/adr/ADR-003-bpv7-profile.md) | BPv7 constrained profile; private block type 192 | DTN standard; interoperability foundation |
| [ADR-004](docs/adr/ADR-004-message-security.md) | HPKE Base + Ed25519; no Double Ratchet in v1 | Simplicity + external audit feasibility |
| [ADR-005](docs/adr/ADR-005-ble-gatt.md) | BLE GATT exclusively; no TCP/UDP fallback | Zero INTERNET permission; minimal attack surface |
| [ADR-006](docs/adr/ADR-006-spray-and-wait.md) | Binary Spray-and-Wait with copy tokens | Proven DTN algorithm; bounded resource use |
| [ADR-007](docs/adr/ADR-007-token-grant-escrow.md) | Persistent token grant escrow | Prevents token inflation after ACK loss |
| [ADR-008](docs/adr/ADR-008-endpoint-only-control.md) | Only sender can revoke; relays ignore cancel targets | Prevents relay-level censorship of messages |

---

## Security Notes

> **This app is a delivery probability aid, not a guaranteed emergency communication system.**

- Messages may not arrive if no relay path exists or all devices are off
- Forward secrecy is **not** provided in v1.0 (HPKE single-shot; not ratcheting)
- Metadata is not anonymous — message size, priority, and timestamps are visible to relays
- GPS requires clear sky and a recent fix; indoors it may be unavailable
- Cancellation does not guarantee removal from already-relayed copies

The design passed an internal multi-agent adversarial review (v1.0.1) with zero start-blockers. **External cryptographic and protocol audit is required before any production deployment.** See [`docs/06-security-and-threat-model.md`](docs/06-security-and-threat-model.md) and [`docs/14-known-limitations.md`](docs/14-known-limitations.md).

**Forbidden marketing claims:** "guaranteed delivery", "real-time without networks", "completely anonymous", "unhackable", "official emergency response", "equal to Signal", "zero battery impact".

---

## Contributing

This project is in the design phase. All protocol changes require:

- Updated CDDL schema in `spec/`
- Updated test vectors in `test-vectors/`
- ADR amendment or new ADR if the decision is architectural
- Updated `IMPLEMENTATION_CHECKLIST.md` completion gates

See [`docs/13-development-goals.md`](docs/13-development-goals.md) for the full implementation guide.

1. Fork the repository
2. Review the [specification](docs/) and [implementation checklist](IMPLEMENTATION_CHECKLIST.md)
3. Pick a goal from [`docs/13-development-goals.md`](docs/13-development-goals.md)
4. Open a pull request referencing the relevant requirement IDs

---

## License

Copyright 2026 The DisasterMesh Authors

Licensed under the [Apache License 2.0](LICENSE).

> Protocol and security properties may change before stable v1.0. See [`docs/14-known-limitations.md`](docs/14-known-limitations.md) for the complete list of limitations and [`docs/06-security-and-threat-model.md`](docs/06-security-and-threat-model.md) for the full release gate requirements.

---

<div align="center">
<sub>
BLE Service UUID: <code>6f1d0001-8f6b-4d5b-9c61-57c43d4d4d31</code> &nbsp;·&nbsp;
Android API 26+ &nbsp;·&nbsp; Rust 2024 Edition &nbsp;·&nbsp; Apache 2.0
</sub>
</div>
