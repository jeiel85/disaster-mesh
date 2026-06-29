#!/usr/bin/env python3
"""Verify the Goal 0 Android module set and dependency direction."""

from __future__ import annotations

import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
ANDROID = ROOT / "apps" / "android"
EXPECTED_MODULES = {
    "app",
    "core-bridge",
    "domain",
    "security-keystore",
    "transport-ble",
    "service-relay",
    "feature-onboarding",
    "feature-home",
    "feature-contacts",
    "feature-conversation",
    "feature-checkin",
    "feature-sos",
    "feature-relay-status",
    "feature-diagnostics",
    "test-fixtures",
}
PROJECT_DEPENDENCY = re.compile(r'(\w+Implementation|implementation|api)\(project\(":([^"]+)"\)\)')


def dependencies(module: str) -> list[tuple[str, str]]:
    build_file = ANDROID / module / "build.gradle.kts"
    return PROJECT_DEPENDENCY.findall(build_file.read_text(encoding="utf-8"))


def main() -> int:
    settings = (ANDROID / "settings.gradle.kts").read_text(encoding="utf-8")
    declared = set(re.findall(r'":([a-z0-9-]+)"', settings))
    errors: list[str] = []

    if declared != EXPECTED_MODULES:
        errors.append(
            f"module set mismatch: missing={sorted(EXPECTED_MODULES - declared)}, "
            f"extra={sorted(declared - EXPECTED_MODULES)}"
        )

    for module in sorted(EXPECTED_MODULES):
        if not (ANDROID / module / "build.gradle.kts").is_file():
            errors.append(f"missing build file for :{module}")

    for module in sorted(name for name in EXPECTED_MODULES if name.startswith("feature-")):
        deps = dependencies(module)
        if ("implementation", "domain") not in deps:
            errors.append(f":{module} must depend on :domain")
        forbidden = [target for _, target in deps if target.startswith("feature-")]
        if forbidden:
            errors.append(f":{module} directly depends on feature modules: {forbidden}")

    for module in ("security-keystore", "transport-ble", "service-relay"):
        if ("implementation", "domain") not in dependencies(module):
            errors.append(f":{module} must depend on :domain")

    app_main_deps = [
        target
        for configuration, target in dependencies("app")
        if configuration in {"implementation", "api"}
    ]
    allowed_app_deps = {
        "service-relay",
        *(name for name in EXPECTED_MODULES if name.startswith("feature-")),
    }
    unexpected = sorted(set(app_main_deps) - allowed_app_deps)
    if unexpected:
        errors.append(f":app has forbidden main dependencies: {unexpected}")

    if dependencies("core-bridge"):
        errors.append(":core-bridge must contain only generated UniFFI/JNA integration")

    if errors:
        print("Android architecture verification failed:", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1

    print(f"Android architecture verified: {len(EXPECTED_MODULES)} modules")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
