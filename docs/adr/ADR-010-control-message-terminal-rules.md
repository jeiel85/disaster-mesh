# ADR-010: Terminal Receipt and Reordered Cancel Handling

Status: Accepted — 2026-06-27

## Decision

DELIVERY_RECEIPT never generates another receipt. CANCEL may generate one terminal receipt. A verified cancel that arrives before its original is stored in `pending_controls` and applied before the original body is exposed.

## Consequence

Receipt storms are impossible by construction; cancel processing requires sender/target persistence and reorder tests.
