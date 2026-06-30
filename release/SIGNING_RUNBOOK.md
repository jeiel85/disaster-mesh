# Production Signing and Evidence Runbook

Status: **TEMPLATE — production keys and signers not configured in this repository**

1. Build from a reviewed signed tag in a clean pinned environment.
2. Keep the app-signing key offline or HSM-backed; separate it from the store upload key.
3. Generate signed AAB/APK, source snapshot, symbols, test vectors, and artifact-level SBOM.
4. Record SHA-256, size, signature verification, build environment, and provenance for every artifact.
5. Populate `release/release-manifest.schema.json` with real test evidence and six role approvals.
6. Canonicalize and sign the manifest with the organization-approved mechanism.
7. Independently verify the tag, app signature, provenance, SBOM, manifest signature, and source commit.
8. Retain the manifest, detached signature, verification transcript, public verification material,
   artifacts, and approvals under access-controlled release evidence storage.

Expected generated inputs under the production evidence directory are:

- `release-manifest.json`
- `release-manifest.json.sig`
- `release-manifest.signature-verification.txt`

File presence is not cryptographic verification. The verification transcript must name the tool,
key identity/fingerprint, command, result, candidate, and artifact hashes. No example or unsigned
Goal 6 artifact may be promoted as production evidence.
