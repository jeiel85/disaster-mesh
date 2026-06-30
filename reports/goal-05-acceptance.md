# Goal 5 Acceptance Record

Status: **BLOCKED — physical campaign not executed**  
Prepared: 2026-06-30

## Automated evidence

- Rust CHECK_IN, PRIVATE_SOS without location, manual-location SOS, and CANCEL
  encrypt/decrypt/state tests pass.
- SQLite committed queues, token grants, and partial transfers survive reopen.
- Android policy tests cover standby/emergency/fixed, battery `<20%`, battery
  `<10%`, thermal severe, and low-storage behavior.
- Dev build, four native ABIs, lint, offline permission policy, and deterministic
  product wording are automated gates.

## Evidence still required

- [ ] Named device/OEM/OS/build identifiers recorded
- [ ] Eight-hour screen-off standby and emergency runs completed
- [ ] Start/end battery, drain per hour, thermal events, and radio duty recorded
- [ ] Process kill and reboot recovery observed on device
- [ ] Bluetooth-off and permission-revoke recovery observed without crash
- [ ] Storage-low and Keystore-loss user-visible recovery captured
- [ ] Queue/receipt/cancel results reconciled after the run
- [ ] Tester and reviewer sign-off recorded

This file is a gate record, not a substitute for measurements. Goal 5 must not
be marked COMPLETE until every physical evidence item above is populated from
real devices.
