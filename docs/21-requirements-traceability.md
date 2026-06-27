# 21. Requirements Traceability Matrix

Every implementation PR names at least one requirement ID and one acceptance evidence ID.

| Requirement | Primary design | Machine contract | Acceptance evidence | Goal |
|---|---|---|---|---|
| FR-001/002 | docs/06 | contact-card CDDL | crypto/contact vectors | 2 |
| FR-003/004 | docs/04,09 | ble-wire/control CDDL | E2E-001, device matrix | 3 |
| FR-005/006 | docs/05 | routing block/schema | E2E-002/003 | 4 |
| FR-007/008 | docs/00,03 | dme-v1 CDDL | codec/UX tests | 2/5 |
| FR-009 | docs/03,05 | dme-aad/state constants | hop/age/token properties | 1/4 |
| FR-010 | docs/03,05 | dme/control rules | receipt terminal tests | 1/4 |
| FR-011 | docs/03,05,07 | pending_controls | E2E-007 | 1/4 |
| FR-012 | docs/01,07,19 | sqlite schema | process/reboot recovery | 1/5 |
| FR-013 | docs/09 | state codes | screen-off/soak | 5/6 |
| FR-014/021 | docs/18 | export allowlist | redaction fuzz/instrumentation | 5/6 |
| FR-020/022 | docs/18,19 | schema/open contract | reset/corruption/upgrade E2E | 5/6 |
| FR-023 | docs/09,17 | UI contract | accessibility/SOS tests | 5 |
| FR-024 | docs/06,10 | contact state codes | key-change tests | 2/5 |
| NFR-001/013 | docs/09,18 | manifest gate | socket/permission assertion | 0/6 |
| NFR-002/003 | docs/03,06 | CDDL/AAD | crypto vectors/fuzz | 2/6 |
| NFR-004/015 | docs/07,19 | SQLite invariants | recovery/migration E2E | 1/6 |
| NFR-005/009 | docs/05 | protocol constants | quota/battery/thermal tests | 4/6 |
| NFR-006/007 | docs/08,10 | state/protocol constants | deterministic/compat tests | 1/3 |
| NFR-008/019 | docs/12,19 | diagnostic codes | support/runbook drill | 6/7 |
| NFR-010/020 | docs/09,17 | text validation contract | accessibility/bidi tests | 5/6 |
| NFR-011/017 | docs/12 | lock/SBOM/provenance | release evidence | 0/7 |
| NFR-012 | docs/14,17 | reviewed copy catalog | UX/store legal review | 5/7 |
| NFR-014 | docs/18 | delete/reset APIs | deletion verification | 5/6 |
| NFR-016 | docs/20 | security checklist | external report | 6/7 |
| NFR-018 | docs/11,19 | device matrix | 8h/24h reports | 6/7 |

Missing traceability is a Definition-of-Ready failure, not documentation debt deferred after implementation.
