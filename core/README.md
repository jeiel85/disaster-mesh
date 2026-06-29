# Rust core

The Rust 2024 workspace owns protocol and persistence state. Goal 1 implements
validated types, deterministic CBOR, DM-BP7-1, SQLite token escrow, routing and
the deterministic contact-graph simulator. Cryptographic identity and E2EE are
introduced in Goal 2.

`mesh-ffi` is the only crate exported to platform code.
