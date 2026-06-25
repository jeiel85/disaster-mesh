# /goal 1 — Protocol Core and Simulator

```text
/goal
Implement the protocol-first core defined in this design bundle. Add validated ID/value types, RFC 8949 core deterministic CBOR, the RFC 9171 outer indefinite bundle array, CDDL validation hooks, the DM-BP7-1 constrained profile with Payload Block number 1, Bundle Age, Hop Count, private Disaster Routing Block type 192, CRC32C, packet deduplication, SQLite schema v1 and forward-only migration. Implement Direct Delivery plus Binary Spray-and-Wait with persistent token-grant escrow, same-grant reconciliation, exact token split, queue score, TTL, hop, quota, tombstone and eviction rules. Build a deterministic contact-graph simulator using the same routing code and injected seeded entropy. Add SIM-001 through SIM-005, unit tests, property tests and malformed-input tests. Do not add real encryption or Bluetooth. Complete only when A can deliver to C through B without A and C ever being connected, token conservation still holds after lost ACKs, uncertain grants are never reused, expired bundles are never offered, and a 100-node deterministic scenario passes in CI.
```
