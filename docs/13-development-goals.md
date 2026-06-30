# 13. Development Goals

각 Goal은 이전 Goal의 결과 위에서만 시작한다. 동시에 여러 Goal을 진행하지 않는다.

## Android App Version 0.2.0 — Product Shell

Goal 0–7의 protocol/security 구현 위에 영속 onboarding, Material 3 상태 dashboard,
설정/앱 정보 화면, 정확한 degraded-state/safety copy와 API 36 emulator 회귀 검증을
추가한다. DME/BLE-CLA major와 DB schema는 v1을 유지한다. 이 app version 증가는
physical BLE·field·commercial gate 완료를 의미하지 않는다.

## Goal 0 — Repository Bootstrap

**상태: 완료 (2026-06-29).** Rust/Android clean build, release manifest gate,
4-ABI native packaging, dependency locks, 그리고 API 36 emulator에서 UniFFI
`version()` instrumentation 호출을 검증했다.

### 결과물

- Rust workspace와 Android multi-module project
- CI skeleton
- version catalog/lockfiles
- protocol/spec 문서 복사
- `offlineRelease` manifest permission assertion
- architecture tests

### 작업

1. root Cargo workspace 생성
2. `mesh-types`, `mesh-codec`, `mesh-crypto`, `mesh-bundle`, `mesh-routing`, `mesh-store`, `mesh-engine`, `mesh-sim`, `mesh-ffi`
3. Android modules 생성
4. Rust Android targets build pipeline
5. single UniFFI facade AAR/native packaging
6. formatting/lint/test CI
7. offline manifest에 INTERNET 미포함 test

### 완료 조건

- clean checkout에서 Rust tests와 Android assemble 성공
- dummy Rust `version()`을 Android instrumentation에서 호출
- release manifest allowlist test 통과
- dependency versions가 lockfile에 고정

## Goal 0.5 — Normative Contract Freeze

**Status: COMPLETE (2026-06-29).** Contract constants are generated into Rust
and Kotlin, all CDDL is compiled by `cddl 0.10.5` and exercised with
representative CBOR, SQLite v1 and invariant SQL execute in tests, and the BLE,
replay, receipt/cancel, and command-correlation pure models have golden,
invalid, reorder, and property coverage. Both source and distribution bundle
validation pass. External security/release review remains a later release gate,
not an open protocol interpretation.

### 결과물

- `spec/dme-aad-v1.cddl`, `spec/ble-wire-v1.md`, complete BLE control CDDL
- `contracts/state_codes.toml`과 generated enum consistency test
- patched initial SQLite schema and schema invariant test
- exact Rust/Android command correlation contract
- design bundle validator passing

### 완료 조건

- hop-limit tamper vector failure
- receipt recursion exhaustive test plan
- cancel reorder storage model review
- replay bitmap property proof/tests
- independent implementer가 BLE byte stream을 추가 질문 없이 작성 가능
- Goal 1–4의 P0 open decision 0

## Goal 1 — Protocol Core and Simulator

**Status: COMPLETE (2026-06-29).** Validated fixed-size IDs and bounded values,
Core Deterministic CBOR, canonical Base32 EIDs, the RFC 9171 indefinite outer
array and DM-BP7-1 block/CRC profile, SQLite v1 forward-only migration and
persistent grant escrow, routing/quota/eviction rules, and the deterministic
SIM-001 through SIM-005 suite are implemented. The 100-node seeded scenario,
lost-ACK token conservation, same-grant idempotency, malformed input, and four
Android ABI native builds pass.

### 결과물

- ID/value types
- deterministic CBOR
- DM-BP7-1 bundle profile
- routing block parser/encoder
- SQLite v1 migration
- store-carry-forward simulator

### 작업

- CDDL 기반 validation
- bundle age/hop/count/token
- direct + binary spray-and-wait
- persistent token grant escrow model
- queue score/eviction
- receipt/cancel state model은 암호화 없이 synthetic payload로 먼저 구현

### 완료 조건

- A와 C가 동시에 연결되지 않는 SIM-001 통과
- 100-node deterministic simulation snapshot test
- invalid CBOR/BP 입력 panic 0
- ACK 유실을 포함한 token grant conservation property 통과

## Goal 2 — Identity, Contact and E2EE

**Status: COMPLETE (2026-06-29).** Separate Ed25519 identity, X25519 HPKE and
X25519 Noise keys, signed contact cards/QR, display and safety identifiers,
trust/key-change handling, RFC 9180 HPKE-protected DME, DMEV local encryption,
and real golden/invalid vectors are implemented. Cross-process vectors,
wrong-recipient/AAD/signature rejection, secret redaction/zeroization, and the
offlineRelease test-randomness exclusion gate pass.

### 결과물

- identity bootstrap
- contact card encode/decode/QR string
- safety number
- HPKE seal/open
- Ed25519 signature
- golden test vectors
- local key encryption

### 완료 조건

- 다른 구현/CLI에서 vector 검증 가능
- AAD 한 필드 변조 시 decrypt 실패
- wrong recipient/signature 실패
- private key/log redaction test
- receiver long-term key compromise limitation 문서와 UI 문구 존재

## Goal 3 — Android Direct BLE

**Status: IMPLEMENTATION FOUNDATION COMPLETE; PHYSICAL ACCEPTANCE BLOCKED
(2026-06-30).** Exact legacy advertising/GATT contracts, API-level permission
policy, callback backpressure, MTU/segmentation/reassembly, Noise XX session
cryptography, Keystore-wrapped database master key, persistent identity/contact
state, UniFFI runtime, and direct encrypted bundle commit are implemented and
covered by host tests. The app provides onboarding, QR-string contact import,
trust display, and receipt-safe conversation labels. A real central/peripheral
device session, camera QR capture, radio-loss recovery, and the two-device
airplane-mode acceptance campaign remain required; this status is not a claim
that the physical Goal 3 completion criteria passed.

### 결과물

- permissions/onboarding
- advertise/scan
- role arbitration
- GATT central/server
- frame segmentation
- Noise XX handshake
- encrypted session hello/slots/inventory
- direct bundle transfer

### 완료 조건

- data/Wi-Fi/SIM off 상태 두 Android 실기기 전송
- B packet capture에서 DME plaintext 없음
- interrupted transfer가 corrupt commit을 만들지 않음
- permission revoke/BT off crash 없음

## Goal 4 — Multi-hop Relay

**Status: AUTOMATED IMPLEMENTATION COMPLETE; PHYSICAL ACCEPTANCE BLOCKED
(2026-06-30).** The complete encrypted control-payload family now has bounded
deterministic-CBOR codecs; relay preparation persists token escrow before
exposing metadata; partial chunks are file-backed, bitmap-resumable across DB
reopen, conflict checked, hash verified, quota bounded, and atomically promoted
to the bundle store. Lost-ACK uncertainty and same-grant reconciliation remain
idempotent. A deterministic campaign runs 50 A→B, separation, B→C deliveries
with an undecrypting relay and later receipt return, while existing flood,
protected-floor, cancel/reorder, quota, and scoring tests continue to pass.
This is simulation evidence only: 50 controlled three-device radio runs and
their capture/report artifacts are still mandatory before Goal 4 is COMPLETE.

### 결과물

- relay queue
- explicit inventory pages
- offer/request/meta/chunk/commit/ACK
- persistent token grant escrow/reconciliation
- receipt/cancel
- partial resume
- quota/rate/eviction

### 완료 조건

- A→B, 분리, B→C 50회 반복 성공
- B는 decrypt 불가
- ACK 유실 시 uncertain grant 재사용 없음, same-grant reconciliation idempotent
- P2/위조 P0 flood에서 verified-local protected floor 유지
- receipt가 역방향 접촉으로 A에 도착

## Goal 5 — Disaster UX and Persistent Relay

**Status: AUTOMATED IMPLEMENTATION COMPLETE; SOAK ACCEPTANCE BLOCKED
(2026-06-30).** CHECK_IN, PRIVATE_SOS, optional manual location, outbound cancel
suppression/tombstone, fixed product routing policies, receipt-safe UI labels,
accessible product screens, and explicit non-guarantee wording are wired through
Rust, UniFFI, and Android. A user-started connected-device foreground service
persists the chosen standby/emergency/fixed mode, displays an ongoing stop
action, and only restores that prior opt-in after reboot. Deterministic policy
tests cover normal duty cycles, battery below 20% and 10%, thermal severe, and
low storage; persisted queue/partial-transfer reopen tests cover process-state
loss. No location permission is required to create SOS. The required physical
screen-off eight-hour campaign, OEM background behavior, power measurements,
and process/reboot fault-injection report remain unexecuted release gates.

### 결과물

- check-in/private SOS/location/cancel
- standby/emergency/fixed modes
- connectedDevice foreground service
- notification/state recovery
- battery/thermal policy
- relay diagnostics

### 완료 조건

- screen-off 8h 시험 보고서
- process kill/restart queue 복구
- battery <10%, thermal severe 정책 동작
- UI가 receipt 전 `전송 완료`를 표시하지 않음
- 위치 권한 없이 SOS 가능

## Goal 6 — Hardening and Public Beta

**Status: AUTOMATED HARDENING IMPLEMENTED; PUBLIC BETA BLOCKED (2026-06-30).**
Six libFuzzer targets cover deterministic CBOR, BP/DME payloads, BLE frames,
stateful outer reassembly, encrypted control payloads, contact/plaintext/local
envelopes; Linux nightly smoke execution is an enforced CI job. Read-only DB
health inspection detects corruption without replacement, and a fixed-schema,
size-bounded diagnostic ZIP has a user preview/export flow and type-level
exclusion of message, location, contact, key, DB, peer, and packet data. A
deterministic CycloneDX source SBOM, dependency gates, unsigned artifact/native
symbol evidence job, GitHub provenance attestation, and Play/F-Droid metadata
are present. Local Windows GNU cannot run libFuzzer's Windows MSVC shim, so
fuzz execution evidence comes from Linux CI. Physical compatibility/battery/
field campaigns and external critical/high review closure remain mandatory;
therefore the product is neither a completed public beta nor stable.

### 결과물

- fuzz targets
- SBOM/cargo deny/audit
- diagnostic export
- DB corruption recovery
- compatibility matrix
- threat model/security docs
- Play/F-Droid packaging

### 완료 조건

- release thresholds 전부 충족
- 외부 protocol/security review 중대 문제 해결
- known limitations와 safety wording 공개
- critical/high dependency advisory 0 또는 승인 예외

## Goal 7 — Commercial Release Readiness

### 구현 상태 — 2026-06-30

상용 출시를 위한 자동 NO-GO 게이트, signed-evidence 입력 조건, rollout/rollback,
incident response, signing runbook, MASVS evidence map과 법률·migration 기록 양식을
구현했다. 현재 `release/readiness-status.json`의 결정은 **NO-GO**다. 실기기·soak·현장
시험, 200/200 controlled acceptance, 외부 security/penetration review, 법률 승인,
실제 support/security channel, production signing과 조직 승인 없이는 Goal 7 완료 또는
상용 출시로 표시하지 않는다.

### 결과물

- privacy policy/data safety/support/security policy
- OWASP MASVS mapping and external review closure
- signed SBOM/provenance/reproducible build evidence
- full device matrix, 24h relay soak, migration/rollback report
- staged rollout and incident response runbook
- legal/safety wording review record

### 완료 조건

- `docs/22-go-live-checklist.md` required 항목 100%
- critical/high security findings 0, P0/P1 defects 0
- 200/200 controlled direct/multi-hop acceptance
- production artifact와 문서/manifest/privacy declarations 일치
- rollback owner와 support escalation 실제 리허설 완료

## Goal 8 — iOS and Fixed Relay

1.0 Android 출시 이후 진행.

- SwiftUI/Core Bluetooth adapter
- same Rust core/DB
- iOS behavior matrix
- Linux/Raspberry Pi relay
- field exercise tooling
