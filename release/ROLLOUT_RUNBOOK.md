# Commercial Rollout and Rollback Runbook

Status: **TEMPLATE — rollout owner not assigned**

## Preconditions

Do not start unless `tools/check_release_readiness.py --require-ready` passes, the signed
manifest identifies the exact artifact, P0/P1 defects are zero, and support/security routes
have been tested. Record the store release ID, artifact SHA-256, protocol/database versions,
owner, deputy, start time, and rollback compatibility decision.

## Stages

| Stage | Cohort | Minimum observation | Expansion evidence |
|---|---:|---:|---|
| internal | named testers | 24 hours | install/update, diagnostics, no P0/P1 |
| closed | approved field cohort | 7 days | controlled contacts, device/soak/support review |
| production 1 | 5% | 24 hours | no halt condition; owner signs expansion |
| production 2 | 20% | 48 hours | same plus OEM and migration review |
| production 3 | 50% | 72 hours | same plus support trend review |
| production 4 | 100% | ongoing | final decision recorded |

Percentages and windows are minimums, not automatic expansion triggers.

## Halt conditions

Immediately halt expansion for suspected key/plaintext leakage, signature/replay/cancel/token
bypass, destructive migration or committed-message loss, unsafe SOS wording, manifest/privacy
mismatch, or a repeated supported-device background relay failure. SEV-0 invokes store halt,
incident command, signed advisory preparation, and evidence preservation.

## Rollback decision

1. Stop rollout; record the exact candidate and time.
2. Determine whether the previous binary can safely open the current DB and protocol state.
3. If compatible, publish only the previously verified signed artifact through the store.
4. If incompatible, block downgrade and prepare a signed forward fix plus user guidance.
5. Never delete or replace a database to make rollback appear successful.
6. Verify install/update, queue preservation, peer compatibility, and disclosure consistency.

## Rehearsal record

Candidate: ____  Owner: ____  Deputy: ____  Date: ____  Result: ____  Evidence SHA-256: ____
