#!/usr/bin/env python3
"""Enforce the offline release permission and backup policy."""

from __future__ import annotations

import argparse
import sys
import xml.etree.ElementTree as ET
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
ANDROID_NS = "{http://schemas.android.com/apk/res/android}"
ALLOWED_PERMISSIONS = {
    "android.permission.ACCESS_FINE_LOCATION",
    "android.permission.BLUETOOTH",
    "android.permission.BLUETOOTH_ADMIN",
    "android.permission.BLUETOOTH_ADVERTISE",
    "android.permission.BLUETOOTH_CONNECT",
    "android.permission.BLUETOOTH_SCAN",
    "android.permission.FOREGROUND_SERVICE",
    "android.permission.FOREGROUND_SERVICE_CONNECTED_DEVICE",
    "android.permission.POST_NOTIFICATIONS",
    "android.permission.RECEIVE_BOOT_COMPLETED",
}
FORBIDDEN_NAMES = {
    "android.permission.ACCESS_NETWORK_STATE",
    "android.permission.INTERNET",
    "com.google.android.gms.permission.AD_ID",
}


def default_manifest() -> Path:
    intermediates = ROOT / "apps" / "android" / "app" / "build" / "intermediates"
    candidates = [
        path
        for path in intermediates.glob("**/AndroidManifest.xml")
        if "offlineRelease" in str(path)
    ]
    if candidates:
        return max(candidates, key=lambda path: path.stat().st_mtime_ns)
    return ROOT / "apps" / "android" / "app" / "src" / "main" / "AndroidManifest.xml"


def verify_backup_rules(errors: list[str]) -> None:
    rules_path = (
        ROOT
        / "apps"
        / "android"
        / "app"
        / "src"
        / "main"
        / "res"
        / "xml"
        / "data_extraction_rules.xml"
    )
    rules = ET.parse(rules_path).getroot()
    for section_name in ("cloud-backup", "device-transfer"):
        section = rules.find(section_name)
        excludes = [] if section is None else section.findall("exclude")
        if not any(
            item.get("domain") == "root" and item.get("path") == "."
            for item in excludes
        ):
            errors.append(f"{section_name} must exclude the complete root domain")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("manifest", nargs="?", type=Path)
    args = parser.parse_args()
    manifest_path = (args.manifest or default_manifest()).resolve()
    manifest = ET.parse(manifest_path).getroot()
    errors: list[str] = []

    permissions = {
        node.get(ANDROID_NS + "name")
        for node in manifest.findall("uses-permission")
    }
    permissions.discard(None)
    package_name = manifest.get("package") or "org.disastermesh.android"
    internal_receiver_permission = (
        f"{package_name}.DYNAMIC_RECEIVER_NOT_EXPORTED_PERMISSION"
    )
    allowed_permissions = ALLOWED_PERMISSIONS | {internal_receiver_permission}
    forbidden = sorted(permissions & FORBIDDEN_NAMES)
    unexpected = sorted(permissions - allowed_permissions)
    missing = sorted(ALLOWED_PERMISSIONS - permissions)
    if forbidden:
        errors.append(f"forbidden permissions present: {forbidden}")
    if unexpected:
        errors.append(f"permissions outside allowlist: {unexpected}")
    if missing:
        errors.append(f"required permissions missing: {missing}")

    application = manifest.find("application")
    if application is None:
        errors.append("application element is missing")
    else:
        expected = {
            "allowBackup": "false",
            "fullBackupContent": "false",
            "dataExtractionRules": "@xml/data_extraction_rules",
        }
        for attribute, value in expected.items():
            actual = application.get(ANDROID_NS + attribute)
            if actual != value:
                errors.append(f"android:{attribute} must be {value!r}, got {actual!r}")

        remote_sdk_tokens = ("analytics", "firebase", "advertising", "crashlytics")
        for component in list(application):
            component_name = (component.get(ANDROID_NS + "name") or "").lower()
            if any(token in component_name for token in remote_sdk_tokens):
                errors.append(f"remote SDK component is forbidden: {component_name}")

    internal_permission = next(
        (
            node
            for node in manifest.findall("permission")
            if node.get(ANDROID_NS + "name") == internal_receiver_permission
        ),
        None,
    )
    if internal_receiver_permission in permissions and (
        internal_permission is None
        or internal_permission.get(ANDROID_NS + "protectionLevel") != "signature"
    ):
        errors.append("AndroidX internal receiver permission must be signature-protected")

    verify_backup_rules(errors)

    if errors:
        print(f"Manifest policy verification failed for {manifest_path}:", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1

    print(f"Offline manifest policy verified: {manifest_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
