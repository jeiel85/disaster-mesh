# ADR-013: Correlate Every Asynchronous Platform Command

Status: Accepted — 2026-06-27

## Decision

Every asynchronous platform command carries a monotonic `command_id`; Android permits one in-flight GATT operation per link and emits exactly one completion/failure event.

## Consequence

The core never infers write success from API acceptance and can deterministically recover failed command batches.
