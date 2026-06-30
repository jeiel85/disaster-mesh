from __future__ import annotations

import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]


class ReleaseManifestValidatorTest(unittest.TestCase):
    def test_complete_manifest_inputs_pass(self) -> None:
        digest = "a" * 64
        commit = "b" * 40
        manifest = {
            "schema_version": 1,
            "release_id": "android-v1.0.0",
            "source_commit": commit,
            "signed_tag": "android-v1.0.0",
            "build_started_at": "2026-06-30T00:00:00Z",
            "app_version": {
                "version_name": "1.0.0",
                "version_code": 1,
                "min_sdk": 26,
                "target_sdk": 36,
                "compile_sdk": 37,
            },
            "protocol": {
                "dme": "1",
                "ble_cla": "1",
                "bp_profile": "DM-BP7-1",
                "state_contract_sha256": digest,
            },
            "db_schema": 1,
            "artifacts": [
                {
                    "name": "candidate.apk",
                    "kind": "apk",
                    "sha256": digest,
                    "size_bytes": 1,
                    "signature_verified": True,
                }
            ],
            "sbom": {
                "format": "CycloneDX",
                "sha256": digest,
                "dependency_review_id": "review-1",
            },
            "test_evidence": [
                {"id": "acceptance", "result": "pass", "evidence_sha256": digest}
            ],
            "approvals": [
                {
                    "role": role,
                    "subject": f"test-{role}",
                    "approved_at": "2026-06-30T00:00:00Z",
                    "signature": "synthetic-test-signature",
                }
                for role in ("product", "engineering", "qa", "security", "operations", "legal")
            ],
            "known_risks": [],
        }
        with tempfile.TemporaryDirectory() as directory:
            temp = Path(directory)
            manifest_path = temp / "manifest.json"
            signature_path = temp / "manifest.sig"
            verification_path = temp / "verification.txt"
            manifest_path.write_text(json.dumps(manifest), "utf-8")
            signature_path.write_bytes(b"synthetic detached signature fixture")
            verification_path.write_text("VERIFIED synthetic test fixture", "utf-8")
            result = subprocess.run(
                [
                    sys.executable,
                    str(ROOT / "tools/validate_release_manifest.py"),
                    str(manifest_path),
                    "--signature",
                    str(signature_path),
                    "--verification",
                    str(verification_path),
                    "--expected-commit",
                    commit,
                    "--expected-tag",
                    "android-v1.0.0",
                ],
                cwd=ROOT,
                text=True,
                capture_output=True,
            )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_missing_manifest_is_rejected(self) -> None:
        result = subprocess.run(
            [
                sys.executable,
                str(ROOT / "tools/validate_release_manifest.py"),
                "missing.json",
                "--signature",
                "missing.sig",
                "--verification",
                "missing.txt",
            ],
            cwd=ROOT,
            text=True,
            capture_output=True,
        )
        self.assertNotEqual(result.returncode, 0)


if __name__ == "__main__":
    unittest.main()
