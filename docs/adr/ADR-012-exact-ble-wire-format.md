# ADR-012: Exact BLE Wire and Resume Contract

Status: Accepted — 2026-06-27

## Decision

`spec/ble-wire-v1.md` and `spec/ble-control-v1.cddl` are normative. Resume identity includes peer, packet, exact wire hash, and chunk layout.

## Consequence

Independent Android/Rust implementations can interoperate without inventing framing details. Any incompatible change requires protocol versioning.
