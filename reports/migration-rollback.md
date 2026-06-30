# Migration and Rollback Evidence

Status: **BLOCKED — physical recovery exercise not executed**  
Candidate/hash: **not assigned**  
Owner/date: **not assigned**

## Automated baseline

- [x] Schema creation and migration tests preserve committed records and invariants
- [x] Newer/unsupported schema is detected without destructive replacement
- [x] Corruption health check is read-only and does not silently reset the database

## Candidate exercise

- [ ] N-1 production-like encrypted database upgrades successfully
- [ ] Process termination at each migration checkpoint resumes or fails closed without data loss
- [ ] Downgrade opens no newer schema and gives an actionable blocked state
- [ ] Corrupt database remains intact for user-authorized recovery/reset
- [ ] Keystore loss never falls back to plaintext or a replacement key
- [ ] Rollback compatibility with protocol/storage state is documented
- [ ] Artifact hashes, synthetic fixture hashes, device/API, logs, and result are attached

No production rollback is authorized from this template alone.
