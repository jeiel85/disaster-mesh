# Incident Response Runbook

Status: **TEMPLATE — incident roles and private channels not assigned**

## Severity and first action

| Severity | Examples | First action |
|---|---|---|
| SEV-0 | key/plaintext exposure, auth/signature bypass, destructive loss, unsafe safety claim | halt rollout, preserve evidence, page incident owner |
| SEV-1 | widespread relay/receipt failure, repeated blocked state on supported devices | halt expansion, triage candidate and rollback |
| SEV-2 | limited OEM degradation with workaround | publish known issue and plan fix |
| SEV-3 | cosmetic/documentation | normal backlog |

## Procedure

1. Record detection time, reporter route, version/commit/artifact hash, and synthetic reproduction.
2. Restrict sensitive evidence; never request message bodies, exact locations, keys, raw DBs, or bundles.
3. Assess confidentiality, integrity, availability, safety wording, migration, and peer compatibility.
4. Choose halt, safe rollback, forward fix, peer block, or signed advisory; name the decision owner.
5. Validate the fix with a regression test and the applicable release gates.
6. Publish user action without promising delivery or data recovery.
7. Retain timeline, affected versions, root cause, detection gap, evidence hashes, and closure approval.

Offline operation has no server kill switch. Store halt and signed update/advisory paths must be
rehearsed before launch.

## Tabletop record

Scenario: ____  Candidate: ____  Commander: ____  Security: ____  Operations: ____  Date: ____  Result: ____
