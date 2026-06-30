#!/usr/bin/env python3
from __future__ import annotations

import argparse
from pathlib import Path
import hashlib
import json
import re
import shutil
import sqlite3
import subprocess
import sys
import tomllib

from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[1]
errors: list[str] = []
notes: list[str] = []

parser = argparse.ArgumentParser(description='Validate the DisasterMesh design baseline.')
parser.add_argument(
    '--distribution',
    action='store_true',
    help='also require that the bundle contains no Git metadata',
)
args = parser.parse_args()

required = [
    'README.md', 'CHANGELOG.md', 'SECURITY.md', 'SUPPORT.md',
    'contracts/protocol_constants.toml', 'contracts/state_codes.toml',
    'spec/dme-aad-v1.cddl', 'spec/ble-control-v1.cddl', 'spec/ble-wire-v1.md',
    'schemas/sqlite_v1.sql', 'schemas/schema_invariants.sql',
    'docs/dependency-review.md', 'docs/22-go-live-checklist.md',
    'policies/PRIVACY_POLICY_DRAFT.md', 'policies/STORE_DISCLOSURE_CHECKLIST.md',
    'release/release-manifest.schema.json', 'release/readiness-status.json',
    'release/ROLLOUT_RUNBOOK.md', 'release/INCIDENT_RESPONSE_RUNBOOK.md',
    'release/SIGNING_RUNBOOK.md', 'reports/goal-07-commercial-gates.md',
    'reports/masvs-evidence-map.md', 'reports/migration-rollback.md',
    'reports/legal-safety-review.md', 'tools/check_release_readiness.py',
    'tools/check_policy_consistency.py',
    'tools/validate_release_manifest.py',
    'tools/tests/test_release_tools.py',
    'test-vectors/cases.schema.json', 'test-vectors/cases.json',
]
for rel in required:
    if not (ROOT / rel).is_file():
        errors.append(f'missing:{rel}')
if args.distribution and (ROOT / '.git').exists():
    errors.append('distribution bundle must not contain .git')

# Machine-readable contracts.
for rel in ['contracts/protocol_constants.toml', 'contracts/state_codes.toml']:
    try:
        tomllib.loads((ROOT / rel).read_text('utf-8'))
    except Exception as exc:
        errors.append(f'toml:{rel}:{exc}')

json_files = [
    'test-vectors/manifest.schema.json',
    'test-vectors/cases.schema.json',
    'test-vectors/cases.json',
    'release/release-manifest.schema.json',
]
json_files.extend(
    path.relative_to(ROOT).as_posix()
    for path in sorted((ROOT / 'test-vectors').glob('*-manifest.json'))
)
parsed_json: dict[str, object] = {}
for rel in json_files:
    try:
        parsed_json[rel] = json.loads((ROOT / rel).read_text('utf-8'))
    except Exception as exc:
        errors.append(f'json:{rel}:{exc}')

for rel in ['test-vectors/manifest.schema.json', 'test-vectors/cases.schema.json', 'release/release-manifest.schema.json']:
    if rel in parsed_json:
        try:
            Draft202012Validator.check_schema(parsed_json[rel])
        except Exception as exc:
            errors.append(f'json-schema:{rel}:{exc}')
if 'test-vectors/cases.schema.json' in parsed_json and 'test-vectors/cases.json' in parsed_json:
    try:
        Draft202012Validator(parsed_json['test-vectors/cases.schema.json']).validate(parsed_json['test-vectors/cases.json'])
        case_ids = [item['id'] for item in parsed_json['test-vectors/cases.json']['cases']]
        if len(case_ids) != len(set(case_ids)):
            errors.append('vector case IDs are not unique')
    except Exception as exc:
        errors.append(f'json-instance:test-vectors/cases.json:{exc}')
if 'test-vectors/manifest.schema.json' in parsed_json:
    for rel in sorted(name for name in parsed_json if name.endswith('-manifest.json')):
        try:
            Draft202012Validator(parsed_json['test-vectors/manifest.schema.json']).validate(parsed_json[rel])
        except Exception as exc:
            errors.append(f'json-instance:{rel}:{exc}')

# SQLite schema and invariants.
sql = (ROOT / 'schemas/sqlite_v1.sql').read_text('utf-8')
try:
    db = sqlite3.connect(':memory:')
    db.execute('PRAGMA foreign_keys=ON')
    db.executescript(sql)
    result = db.execute('PRAGMA quick_check').fetchone()[0]
    if result != 'ok':
        errors.append(f'sqlite quick_check:{result}')
    fk = db.execute('PRAGMA foreign_key_check').fetchall()
    if fk:
        errors.append(f'initial foreign key violations:{len(fk)}')
    if db.execute('PRAGMA user_version').fetchone()[0] != 1:
        errors.append('user_version != 1')

    cols = {row[1] for row in db.execute('PRAGMA table_info(transfers)')}
    for col in ['peer_link_hash', 'expected_wire_sha256', 'meta_fingerprint', 'resume_expires_at_ms']:
        if col not in cols:
            errors.append(f'transfers missing {col}')
    replay_cols = {row[1] for row in db.execute('PRAGMA table_info(contact_replay_state)')}
    if 'seen_bitmap' not in replay_cols:
        errors.append('replay bitmap missing')
    tables = {row[0] for row in db.execute("SELECT name FROM sqlite_master WHERE type='table'")}
    for table in ['pending_controls', 'token_grants', 'receipts', 'transfers']:
        if table not in tables:
            errors.append(f'table missing:{table}')

    invariant_text = (ROOT / 'schemas/schema_invariants.sql').read_text('utf-8')
    statements = [s.strip() for s in re.sub(r'--.*', '', invariant_text).split(';') if s.strip()]
    for index, statement in enumerate(statements, 1):
        rows = db.execute(statement).fetchall()
        if rows:
            errors.append(f'schema invariant {index} returned {len(rows)} row(s)')
finally:
    try:
        db.close()
    except Exception:
        pass

# Cross-document critical rules.
combined = '\n'.join(path.read_text('utf-8', errors='replace') for path in ROOT.glob('docs/*.md'))
if 'hop-limit' not in (ROOT / 'spec/dme-aad-v1.cddl').read_text('utf-8'):
    errors.append('AAD missing hop-limit')
if 'DELIVERY_RECEIPT' not in combined or '절대 생성하지 않음' not in combined:
    errors.append('receipt terminal rule missing')
if '4096-bit' not in combined:
    errors.append('replay window rule missing')
if 'command_id' not in (ROOT / 'contracts/rust_facade.rs').read_text('utf-8'):
    errors.append('platform command correlation missing')

goal_text = (ROOT / 'docs/13-development-goals.md').read_text('utf-8')
goal_ids = re.findall(r'^## Goal ([0-9]+(?:\.[0-9]+)?)\b', goal_text, re.MULTILINE)
duplicate_goal_ids = sorted({goal_id for goal_id in goal_ids if goal_ids.count(goal_id) > 1})
if duplicate_goal_ids:
    errors.append(f'duplicate development goal IDs:{",".join(duplicate_goal_ids)}')

readiness = subprocess.run(
    [sys.executable, str(ROOT / 'tools/check_release_readiness.py')],
    cwd=ROOT,
    text=True,
    capture_output=True,
)
if readiness.returncode != 0:
    errors.append(f'release-readiness:{readiness.stdout.strip() or readiness.stderr.strip()}')

disclosures = subprocess.run(
    [sys.executable, str(ROOT / 'tools/check_policy_consistency.py')],
    cwd=ROOT,
    text=True,
    capture_output=True,
)
if disclosures.returncode != 0:
    errors.append(f'policy-consistency:{disclosures.stdout.strip() or disclosures.stderr.strip()}')

# Normative machine files must not contain unfinished markers.
for path in list(ROOT.glob('spec/*')) + list(ROOT.glob('contracts/*')) + list(ROOT.glob('schemas/*')):
    if path.is_file() and re.search(r'\b(TODO|TBD|FIXME)\b', path.read_text('utf-8', errors='replace')):
        errors.append(f'unfinished marker:{path.relative_to(ROOT)}')

# Referenced bundle paths must exist. Ignore globs and source-tree paths not shipped in this design bundle.
path_pattern = re.compile(r'(?<![A-Za-z0-9_.-])((?:docs|spec|contracts|schemas|test-vectors|release|policies)/[A-Za-z0-9._/-]+\.(?:md|toml|sql|cddl|json|rs|kt|txt))')
for md in ROOT.rglob('*.md'):
    if 'archive' in md.parts:
        continue
    text = md.read_text('utf-8', errors='replace')
    for match in path_pattern.finditer(text):
        rel = match.group(1).rstrip('.,:;)`\'\"')
        if any(ch in rel for ch in '*{}'):
            continue
        if not (ROOT / rel).exists():
            errors.append(f'broken path reference:{md.relative_to(ROOT)}:{rel}')

# CDDL: use an installed validator when available; otherwise do lexical structural checks and report the limitation.
cddl_files = sorted(ROOT.glob('spec/*.cddl'))
cddl_cmd = shutil.which('cddl')
if cddl_cmd:
    for path in cddl_files:
        proc = subprocess.run(
            [cddl_cmd, '--ci', 'compile-cddl', '--cddl', str(path)],
            text=True,
            capture_output=True,
        )
        if proc.returncode != 0:
            errors.append(f'cddl:{path.relative_to(ROOT)}:{proc.stderr.strip() or proc.stdout.strip()}')
else:
    for path in cddl_files:
        text = re.sub(r';.*', '', path.read_text('utf-8', errors='replace'))
        for left, right in [('(', ')'), ('[', ']'), ('{', '}')]:
            if text.count(left) != text.count(right):
                errors.append(f'cddl-unbalanced:{path.relative_to(ROOT)}:{left}{right}')
        if '=' not in text:
            errors.append(f'cddl-no-rule:{path.relative_to(ROOT)}')
    notes.append('CDDL executable not installed; lexical balance checks only')

# Generate deterministic source inventory before hashing it. Goal 0 introduces
# build trees and generated JNI outputs, none of which belong in a source
# distribution manifest.
excluded_inventory_dirs = {'.git', '.gradle', '.kotlin', '__pycache__', 'build', 'target'}


def belongs_in_inventory(path: Path) -> bool:
    relative = path.relative_to(ROOT)
    if not path.is_file() or any(part in excluded_inventory_dirs for part in relative.parts):
        return False
    return relative.parts[:6] != (
        'apps', 'android', 'core-bridge', 'src', 'main', 'jniLibs'
    )


all_paths = sorted(p for p in ROOT.rglob('*') if belongs_in_inventory(p))
filelist_lines = [p.relative_to(ROOT).as_posix() for p in all_paths]
(ROOT / 'FILELIST.txt').write_text('\n'.join(filelist_lines) + '\n', 'utf-8')

manifest: list[str] = []
for path in all_paths:
    if path.name == 'FILE_SHA256SUMS.txt':
        continue
    manifest.append(f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {path.relative_to(ROOT).as_posix()}")
(ROOT / 'FILE_SHA256SUMS.txt').write_text('\n'.join(manifest) + '\n', 'utf-8')

if errors:
    print('FAIL')
    for error in sorted(set(errors)):
        print('-', error)
    for note in notes:
        print('NOTE:', note)
    sys.exit(1)
print(f'PASS: {len(manifest)} files; SQLite/TOML/JSON/schema/path/contract checks completed')
for note in notes:
    print('NOTE:', note)
