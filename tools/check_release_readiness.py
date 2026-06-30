#!/usr/bin/env python3
from __future__ import annotations

import argparse
from datetime import datetime
import json
from pathlib import Path, PurePosixPath
import re
import sys


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_STATUS = ROOT / "release/readiness-status.json"
REQUIRED_GATES = {
    "controlled-acceptance",
    "device-compatibility",
    "external-security-review",
    "final-approvals",
    "legal-privacy-review",
    "masvs-penetration",
    "migration-recovery",
    "release-signing-provenance",
    "rollout-incident-drill",
    "soak-field-exercise",
    "support-security-channels",
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Validate the machine-readable commercial release decision."
    )
    parser.add_argument("--status", type=Path, default=DEFAULT_STATUS)
    parser.add_argument(
        "--require-ready",
        action="store_true",
        help="fail unless every required gate has approved evidence and decision is GO",
    )
    return parser.parse_args()


def error(message: str, errors: list[str]) -> None:
    errors.append(message)


def valid_evidence_path(value: object, gate_id: str, errors: list[str]) -> bool:
    if not isinstance(value, str) or not value:
        error(f"{gate_id}: evidence paths must be non-empty strings", errors)
        return False
    path = PurePosixPath(value)
    if path.is_absolute() or ".." in path.parts:
        error(f"{gate_id}: unsafe evidence path: {value}", errors)
        return False
    if not (ROOT / Path(*path.parts)).is_file():
        error(f"{gate_id}: missing evidence file: {value}", errors)
        return False
    return True


def validate_timestamp(value: object, gate_id: str, errors: list[str]) -> None:
    if not isinstance(value, str):
        error(f"{gate_id}: approved_at is required for a passed gate", errors)
        return
    try:
        timestamp = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError:
        error(f"{gate_id}: approved_at is not an ISO-8601 timestamp", errors)
        return
    if timestamp.tzinfo is None:
        error(f"{gate_id}: approved_at must include a timezone", errors)


def main() -> int:
    args = parse_args()
    errors: list[str] = []
    try:
        data = json.loads(args.status.read_text("utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        print(f"FAIL: cannot read readiness status: {exc}")
        return 1

    if data.get("schema_version") != 1:
        error("schema_version must be 1", errors)
    decision = data.get("decision")
    if decision not in {"GO", "NO-GO"}:
        error("decision must be GO or NO-GO", errors)
    candidate = data.get("release_candidate")
    if candidate is not None and (not isinstance(candidate, str) or not candidate.strip()):
        error("release_candidate must be null or a non-empty string", errors)
    if isinstance(candidate, str) and not re.fullmatch(
        r"android-v[0-9]+\.[0-9]+\.[0-9]+(?:-[A-Za-z0-9.-]+)?", candidate
    ):
        error("release_candidate must be an android-v semantic version tag", errors)

    gates = data.get("gates")
    if not isinstance(gates, dict):
        error("gates must be an object", errors)
        gates = {}
    gate_ids = set(gates)
    missing = sorted(REQUIRED_GATES - gate_ids)
    extra = sorted(gate_ids - REQUIRED_GATES)
    if missing:
        error(f"missing gates: {', '.join(missing)}", errors)
    if extra:
        error(f"unknown gates: {', '.join(extra)}", errors)

    passed = 0
    for gate_id in sorted(REQUIRED_GATES & gate_ids):
        gate = gates[gate_id]
        if not isinstance(gate, dict):
            error(f"{gate_id}: gate must be an object", errors)
            continue
        status = gate.get("status")
        if status not in {"blocked", "pass"}:
            error(f"{gate_id}: status must be blocked or pass", errors)
            continue
        evidence = gate.get("evidence")
        if not isinstance(evidence, list):
            error(f"{gate_id}: evidence must be an array", errors)
            evidence = []
        for value in evidence:
            valid_evidence_path(value, gate_id, errors)
        if status == "blocked":
            if not isinstance(gate.get("blocker"), str) or not gate["blocker"].strip():
                error(f"{gate_id}: blocked gate must explain blocker", errors)
            continue

        passed += 1
        if not evidence:
            error(f"{gate_id}: passed gate needs retained evidence", errors)
        if not isinstance(gate.get("approved_by"), str) or not gate["approved_by"].strip():
            error(f"{gate_id}: approved_by is required for a passed gate", errors)
        validate_timestamp(gate.get("approved_at"), gate_id, errors)

    ready = (
        not errors
        and decision == "GO"
        and isinstance(candidate, str)
        and bool(candidate.strip())
        and passed == len(REQUIRED_GATES)
    )
    if decision == "GO" and not ready:
        error("GO is invalid until a candidate is named and every required gate passes", errors)
        ready = False

    if errors:
        print("FAIL")
        for message in errors:
            print(f"- {message}")
        return 1

    print(
        f"VALID: {decision}; {passed}/{len(REQUIRED_GATES)} commercial gates passed"
    )
    if args.require_ready and not ready:
        print("BLOCKED: commercial release requires a GO decision and all gate evidence")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
