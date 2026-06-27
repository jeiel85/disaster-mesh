# ADR-009: Authenticate Immutable Hop Limit

Status: Accepted — 2026-06-27

## Decision

`hop_limit` is included in DME v1 HPKE AAD. `hop_count` remains mutable and excluded.

## Consequence

Any relay change to hop limit causes AAD hash/HPKE open failure. Golden invalid vectors must mutate hop limit independently.
