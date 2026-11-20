# Fresh native supplier slots — 2026-11-21

## Scope and boundary

Candidate canonical package inputs for `org.vibevm.agents/pi@1.6.0`, `codex@0.171.0`, `opencode@2.3.2`, `claude-code@2.1.329` and the existing `qwen@0.28.0` slot. Each has Linux x64, macOS Intel x64 and Windows x64 options. Historical slots remain unchanged. The Qwen slot retains its existing Linux, macOS arm64 and package-owned Windows bootstrap; only the macOS Intel option is added.

Release selection used retained official release metadata, excluding prerelease/draft/SDK entries. OpenCode CLI 2.3.2 supersedes the preliminary 1.19.6 delivery selection. Claude Code native children are exact-version official npm artifacts with verified SHA-512 registry integrity and recorded SHA-256. GitHub artifacts match supplier release size/digest. No private profile, authentication store or local corporate-kit content is a package input.

## Evidence

Public staging: `/data/research/vibevm/six-agent-release-staging-20261121-v4ldg2kw`.

- `PUBLIC-ASSET-ACQUISITION.json` and `CLAUDE-QWEN-ACQUISITION.json`: 15 acquired or exact-byte reused artifacts, verified digests.
- `VERIFIED-NATIVE-INPUTS.json`: archive members and executable identities, ELF/Mach-O/PE architecture inspection; supplier execution was false in that inventory.
- `OFFLINE-CATALOG-INPUT-MAPPING.json`: exact coordinates, versions, asset basenames, digests and drop-relative paths; SHA-256 `7ca2fa7cb756b911e178082f40146a7a0f6b9b66fb74d0a4413d535abb6329fb`.
- `CANONICAL-MANIFEST-BINDING-R2.json`: all 15 archive basenames/digests match source manifests. R1 is retained. Architect's real mirror admission found inconsistent Qwen `platform_name` values; corrected the Intel option to `macOS`, retaining `arch = "x86_64"`.
- `/data/git/vibevm-keyring-storage-20261120T0100Z/fresh-slots-build-r2/`: corrected-source isolated offline compilation, exit 0, followed by the exact test artifact in filesystem/PID/network isolation: **1 passed, 0 failed, 0 ignored, 0 filtered**. The test parses all five canonical manifests, selects all three x64 platforms and refuses cross-platform option IDs and Windows arm64. R1 compilation predates the correction and is not the corrected-subject gate.

## Not claimed

These gates do not establish complete third-party/component redistribution clearance, foreign-native execution, an offline install or a final corporate kit. Existing offline artifact lookup is reused; no new resolver is introduced. Architect owns actual local catalog/drop integration and public offline CLI installation checks. Builder does not read or rewrite the private kit. Hermes source+wheels and native-settings adapters are separate unfinished results. No external publication or push occurred. Independent acceptance and commit are pending.
