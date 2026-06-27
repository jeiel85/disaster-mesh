# /goal 0.5 — Normative Contract Freeze

Implement no product feature. Reconcile and test the design contracts first.

Required work:

- load protocol/state TOML as generated constants or CI-checked sources
- validate all CDDL with the selected validator
- execute sqlite_v1.sql and schema_invariants.sql in CI
- implement byte-level BLE header codecs from spec/ble-wire-v1.md with golden/invalid cases
- implement replay bitmap pure model and property tests
- define receipt terminal and cancel reorder pure/state tests
- define command_id correlation model for Android fake adapter
- run `python tools/validate_design_bundle.py` in the repository
- run `python tools/validate_design_bundle.py --distribution` against packaged artifacts

Exit only when every P0 contract has zero open interpretation and the validator passes.
