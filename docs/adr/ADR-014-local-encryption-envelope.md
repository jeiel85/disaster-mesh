# ADR-014: Versioned Local Encryption Envelope

Status: Accepted — 2026-06-27

## Decision

Sensitive SQLite values use the DMEV v1 XChaCha20-Poly1305 envelope with table/column/primary-key AAD and explicit key version.

## Consequence

Row swapping and silent decrypt fallback are prohibited; key loss triggers explicit recovery/reset rather than automatic data destruction.
