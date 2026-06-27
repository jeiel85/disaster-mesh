# ADR-011: Persisted 4096-bit Replay Window

Status: Accepted — 2026-06-27

## Decision

Each contact keeps max sender sequence plus a 4096-bit sliding bitmap. Message/packet IDs remain secondary dedup keys.

## Consequence

The acceptance window and storage representation are aligned; delayed messages within the window are accepted once.
