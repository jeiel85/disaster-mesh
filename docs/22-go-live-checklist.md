# 22. Commercial Go-Live Checklist

Release candidate: __________  Commit: __________  Artifact SHA-256: __________

Current decision (2026-06-30): **NO-GO — candidate not assigned**

Evidence index:

- machine decision: `release/readiness-status.json`
- commercial gate record: `reports/goal-07-commercial-gates.md`
- MASVS map: `reports/masvs-evidence-map.md`
- migration/rollback: `reports/migration-rollback.md`
- legal/safety: `reports/legal-safety-review.md`
- rollout and incident procedures: `release/ROLLOUT_RUNBOOK.md`, `release/INCIDENT_RESPONSE_RUNBOOK.md`
- signing procedure: `release/SIGNING_RUNBOOK.md`

The checklist remains unsigned until every item is bound to one production candidate.

## Product and safety — Product owner

- [ ] scope/limitations/store listing match the binary
- [ ] official emergency-service or guaranteed-delivery implication absent
- [ ] onboarding, SOS, cancel, key-loss, permission degradation copy approved
- [ ] Korean/English and accessibility acceptance complete

## Protocol and data — Core lead

- [ ] CDDL/wire/state/schema validator passes
- [ ] golden and invalid vectors generated from reviewed implementation
- [ ] receipt recursion, cancel reorder, replay bitmap, token escrow tests pass
- [ ] N-1 migration, interrupted migration, downgrade behavior pass
- [ ] committed data loss and invariant violation 0

## Android/device — Mobile lead

- [ ] release manifest allowlist and no-INTERNET assertion pass
- [ ] API 26/30/31/34/36/37 supported matrix complete
- [ ] Android 14+ MTU one-request behavior verified
- [ ] OEM scan/advertise/screen-off/permission paths documented
- [ ] 8h normal and 24h fixed relay soak pass

## Security/privacy — Security owner

- [ ] external protocol/crypto review closed
- [ ] MASVS mapping evidence complete
- [ ] critical/high findings 0
- [ ] SBOM/dependency/license/secret scan pass; `docs/dependency-review.md` contains no unreviewed production dependency
- [ ] privacy policy/Data Safety/diagnostic contents match artifact
- [ ] backup exclusion, notification/log/export redaction verified

## Operations/support — Release owner

- [ ] signed tag/artifacts/provenance and symbol archive created
- [ ] release evidence manifest validates against `release/release-manifest.schema.json` and is signed
- [ ] staged rollout percentages, observation windows, rollback owner set
- [ ] `SECURITY.md` private report route and `SUPPORT.md` owner/channel tested
- [ ] SEV-0/SEV-1 tabletop and rollback drill complete
- [ ] known issues/support device list published

## Legal/compliance — Authorized reviewer

- [ ] target markets and store policies reviewed at RC date
- [ ] privacy draft publisher/contact/effective-date fields completed; privacy/location/consumer/emergency wording reviewed
- [ ] third-party/open-source notices complete
- [ ] no unreviewed regulated-service claim

## Final decision

- [ ] GO — all required items complete; waivers attached and none affect P0/P1
- [x] NO-GO — release blocked; see `release/readiness-status.json`

Approvals: Product ____  Core ____  Android ____  QA ____  Security ____  Operations ____  Legal ____
