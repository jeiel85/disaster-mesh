# Rust core

The Rust 2024 workspace owns protocol and persistence state. Goal 0 establishes
crate boundaries only; protocol, cryptographic, routing, and storage behavior is
intentionally deferred to later goals.

`mesh-ffi` is the only crate exported to platform code.
