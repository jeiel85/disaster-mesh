#!/usr/bin/env python3
"""Generate a deterministic CycloneDX source-dependency SBOM from committed locks."""

from __future__ import annotations

import argparse
import json
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def cargo_components() -> list[dict]:
    lock = tomllib.loads((ROOT / "Cargo.lock").read_text("utf-8"))
    components = []
    for package in lock["package"]:
        source = package.get("source", "")
        if not source.startswith("registry+"):
            continue
        name, version = package["name"], package["version"]
        component = {
            "type": "library",
            "bom-ref": f"pkg:cargo/{name}@{version}",
            "name": name,
            "version": version,
            "purl": f"pkg:cargo/{name}@{version}",
        }
        if checksum := package.get("checksum"):
            component["hashes"] = [{"alg": "SHA-256", "content": checksum}]
        components.append(component)
    return components


def gradle_components() -> list[dict]:
    coordinates: set[tuple[str, str, str]] = set()
    for lock_path in (ROOT / "apps" / "android").glob("**/gradle.lockfile"):
        for line in lock_path.read_text("utf-8").splitlines():
            if not line or line.startswith("#") or "=" not in line:
                continue
            coordinate, configurations = line.split("=", 1)
            if "releaseruntimeclasspath" not in configurations.lower():
                continue
            parts = coordinate.split(":")
            if len(parts) != 3:
                continue
            coordinates.add(tuple(parts))
    return [
        {
            "type": "library",
            "bom-ref": f"pkg:maven/{group}/{name}@{version}",
            "group": group,
            "name": name,
            "version": version,
            "purl": f"pkg:maven/{group}/{name}@{version}",
        }
        for group, name, version in sorted(coordinates)
    ]


def generate() -> dict:
    components = cargo_components() + gradle_components()
    components.sort(key=lambda component: component["bom-ref"])
    return {
        "bomFormat": "CycloneDX",
        "specVersion": "1.6",
        "version": 1,
        "metadata": {
            "component": {
                "type": "application",
                "bom-ref": "pkg:github/jeiel85/disaster-mesh",
                "name": "DisasterMesh",
                "version": "0.2.0-beta-source",
            },
            "properties": [
                {"name": "disastermesh:evidence-status", "value": "UNSIGNED_SOURCE_INVENTORY"},
                {"name": "disastermesh:scope", "value": "Cargo.lock and Android release runtime lockfiles"},
            ],
            "tools": {"components": [{"type": "application", "name": "tools/generate_sbom.py"}]},
        },
        "components": components,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, default=ROOT / "release" / "evidence" / "beta-source-sbom.cdx.json")
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    encoded = json.dumps(generate(), ensure_ascii=False, indent=2, sort_keys=True) + "\n"
    if args.check:
        if not args.output.is_file() or args.output.read_text("utf-8") != encoded:
            raise SystemExit(f"SBOM is stale: {args.output}")
    else:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(encoded, encoding="utf-8", newline="\n")
    print(f"CycloneDX SBOM verified: {args.output}" if args.check else f"CycloneDX SBOM written: {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
