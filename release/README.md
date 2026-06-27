# Release Evidence

실제 구현 repository의 release job은 `release-manifest.schema.json`에 맞는 manifest를 생성하고 서명한다. 이 설계 묶음에는 존재하지 않는 build artifact나 승인을 만들어 넣지 않는다.

## Required flow

1. clean, pinned build environment에서 signed tag를 checkout한다.
2. AAB/APK/source/vectors/report와 SBOM을 생성한다.
3. 모든 artifact SHA-256과 signature verification 결과를 기록한다.
4. test evidence ID를 `docs/11-testing-and-acceptance.md` 및 `docs/22-go-live-checklist.md`와 연결한다.
5. P0/P1 known risk는 manifest에 넣어 waiver할 수 없다.
6. 제품·엔지니어링·QA·보안·운영 승인을 받고, 적용되는 경우 법률 승인을 추가한다.
7. canonical JSON 또는 조직이 정한 signed envelope로 manifest를 서명해 artifact와 함께 보존한다.

`*.example.json`을 production evidence로 재사용하지 않는다. 승인자 이름·서명·법적 주체는 실제 조직 정보만 사용한다.
