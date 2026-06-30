#!/usr/bin/env python3
from __future__ import annotations

import argparse
from pathlib import Path
import re
import sys


ROOT = Path(__file__).resolve().parents[1]


def require(text: str, needle: str, label: str, errors: list[str]) -> None:
    if needle not in text:
        errors.append(f"{label}: missing required statement: {needle}")


def main() -> int:
    parser = argparse.ArgumentParser(description="Check source, listing, and privacy claim alignment.")
    parser.add_argument(
        "--require-publishable",
        action="store_true",
        help="also reject unresolved legal publisher placeholders",
    )
    args = parser.parse_args()
    errors: list[str] = []

    android = ROOT / "apps/android"
    for manifest in android.rglob("src/*/AndroidManifest.xml"):
        text = manifest.read_text("utf-8")
        for permission in ("android.permission.INTERNET", "android.permission.ACCESS_NETWORK_STATE"):
            if permission in text:
                errors.append(f"{manifest.relative_to(ROOT)}: forbidden offline permission: {permission}")

    forbidden_sdks = ("firebase", "crashlytics", "play-services-ads", "sentry", "appcenter")
    for build_file in android.rglob("build.gradle.kts"):
        lowered = build_file.read_text("utf-8").lower()
        for marker in forbidden_sdks:
            if marker in lowered:
                errors.append(f"{build_file.relative_to(ROOT)}: forbidden SDK marker: {marker}")

    privacy = (ROOT / "policies/PRIVACY_POLICY_DRAFT.md").read_text("utf-8")
    governance = (ROOT / "docs/18-privacy-and-data-governance.md").read_text("utf-8")
    diagnostic = (
        ROOT
        / "apps/android/domain/src/main/kotlin/org/disastermesh/android/domain/DiagnosticArchive.kt"
    ).read_text("utf-8")
    listing_en = (ROOT / "fastlane/metadata/android/en-US/full_description.txt").read_text("utf-8")
    listing_ko = (ROOT / "fastlane/metadata/android/ko-KR/full_description.txt").read_text("utf-8")

    require(privacy, "`INTERNET` permission 없이", "privacy draft", errors)
    require(privacy, "analytics SDK", "privacy draft", errors)
    require(privacy, "공식 긴급 신고 채널이 아니며", "privacy draft", errors)
    require(governance, "최대 1 MiB", "data governance", errors)
    require(diagnostic, "it.size <= 1_048_576", "diagnostic archive", errors)
    require(diagnostic, 'listOf("README.txt", "metadata.json", "relay.txt", "events.csv")', "diagnostic archive", errors)
    require(listing_en, "may never happen", "English store listing", errors)
    require(listing_en, "does not replace emergency services", "English store listing", errors)
    require(listing_en, "no account, cloud service, advertising, analytics SDK, or internet permission", "English store listing", errors)
    require(listing_ko, "영원히 전달되지 않을 수 있습니다", "Korean store listing", errors)
    require(listing_ko, "공식 구조 접수를 대체하지 않습니다", "Korean store listing", errors)
    require(listing_ko, "광고, 분석 SDK, 인터넷 권한을 사용하지 않습니다", "Korean store listing", errors)

    if args.require_publishable and re.search(r"\{\{[A-Z0-9_]+\}\}", privacy):
        errors.append("privacy draft still contains publisher/contact/effective-date placeholders")

    if errors:
        print("FAIL")
        for message in errors:
            print(f"- {message}")
        return 1
    print("PASS: offline source, diagnostic limit, privacy draft, and store claims are aligned")
    return 0


if __name__ == "__main__":
    sys.exit(main())
