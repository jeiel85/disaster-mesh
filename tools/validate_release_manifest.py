#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from pathlib import Path
import re
import sys

from jsonschema import Draft202012Validator, FormatChecker


ROOT = Path(__file__).resolve().parents[1]
ALL_APPROVAL_ROLES = {"product", "engineering", "qa", "security", "operations", "legal"}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Validate production release evidence inputs.")
    parser.add_argument("manifest", type=Path)
    parser.add_argument("--signature", type=Path, required=True)
    parser.add_argument("--verification", type=Path, required=True)
    parser.add_argument("--expected-commit")
    parser.add_argument("--expected-tag")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    errors: list[str] = []
    try:
        schema = json.loads((ROOT / "release/release-manifest.schema.json").read_text("utf-8"))
        manifest = json.loads(args.manifest.read_text("utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        print(f"FAIL: cannot read release manifest: {exc}")
        return 1

    validator = Draft202012Validator(schema, format_checker=FormatChecker())
    for validation_error in sorted(validator.iter_errors(manifest), key=lambda item: list(item.path)):
        location = "/".join(str(part) for part in validation_error.path) or "$"
        errors.append(f"schema:{location}:{validation_error.message}")

    if not isinstance(manifest, dict):
        print("FAIL")
        for message in errors:
            print(f"- {message}")
        return 1

    approvals = manifest.get("approvals", [])
    if not isinstance(approvals, list):
        approvals = []
    artifacts = manifest.get("artifacts", [])
    if not isinstance(artifacts, list):
        artifacts = []
    roles = {approval.get("role") for approval in approvals if isinstance(approval, dict)}
    missing_roles = sorted(ALL_APPROVAL_ROLES - roles)
    if missing_roles:
        errors.append(f"missing approval roles: {', '.join(missing_roles)}")
    if any(
        not isinstance(artifact, dict) or not artifact.get("signature_verified")
        for artifact in artifacts
    ):
        errors.append("every release artifact must have signature_verified=true")
    if args.expected_commit and manifest.get("source_commit") != args.expected_commit:
        errors.append("source_commit does not match the checked-out commit")
    if args.expected_tag and manifest.get("signed_tag") != args.expected_tag:
        errors.append("signed_tag does not match the release tag")

    try:
        signature = args.signature.read_bytes()
    except OSError as exc:
        errors.append(f"cannot read detached signature: {exc}")
    else:
        if not signature:
            errors.append("detached signature is empty")

    try:
        verification = args.verification.read_text("utf-8").strip()
    except (OSError, UnicodeDecodeError) as exc:
        errors.append(f"cannot read verification transcript: {exc}")
    else:
        if not verification:
            errors.append("verification transcript is empty")
        if re.search(r"\b(TODO|TBD|PLACEHOLDER|UNVERIFIED)\b", verification, re.IGNORECASE):
            errors.append("verification transcript contains an unfinished marker")

    if errors:
        print("FAIL")
        for message in errors:
            print(f"- {message}")
        return 1
    print(f"PASS: production release manifest validated: {args.manifest}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
