# OpenCode V1 archive — bounded redistribution-review card

2026-11-19. Candidate only; architect owns redistribution disposition. No corporate drop inclusion, publication, extraction, installation or execution of the archive has occurred.

## Exact subject and integrity

- Official repository: `anomalyco/opencode`.
- Release/tag: `v1.19.6`; GitHub release ID `448257298`, published `2026-11-17T07:21:32Z`.
- Asset: `opencode-linux-x64.tar.gz`, asset ID `757980715`; Linux x86_64, default glibc variant. Not musl, ARM64 or baseline variant.
- Official metadata: `https://api.github.com/repos/anomalyco/opencode/releases/tags/v1.19.6`.
- Official asset: `https://github.com/anomalyco/opencode/releases/download/v1.19.6/opencode-linux-x64.tar.gz`.
- Size: `79,862,500` bytes.
- SHA-256: `47734e08968b30cc22a0bffb6d9feeed76ad4aa743213a12122040392d09a057`.
- Verification: metadata `assets[].digest` and `size` independently compared with the entire privately downloaded archive; both match. No signature verification or reproducible-build claim.
- Scope: a package-acquisition candidate, NOT admission of this version to the existing OpenCode native carrier. No V2 substitution.

## Release LICENSE / NOTICE

Exact tag source `https://raw.githubusercontent.com/anomalyco/opencode/v1.19.6/LICENSE` is MIT, copyright 2025 opencode. Retain its complete copyright and permission text if the reviewed distribution is authorized. This root grant alone is not a complete bundled-component disposition.

Retained LICENSE SHA-256: `625f0f619133f89bbbb2abe37369613dfa1885eba1e50d02170deb62bb42cb6b` (1,065 bytes). A request for root `NOTICE` returned HTTP error; no root NOTICE was obtained. This does not prove notices are unnecessary or absent everywhere in the source tree.

## Actual archive inventory — no extraction

Python standard-library tar inventory, run with `-I` from the trusted project cwd, found exactly one member:

| Name | Type | Uncompressed size | Mode |
|---|---|---:|---|
| `opencode` | regular file, no link | 215,845,040 bytes | 0755 |

There is no separate LICENSE, NOTICE, SBOM, dependency manifest or third-party notice file among the archive members. Embedded binary notices were not inspected. The executable has not been extracted or executed.

## Component evidence and unresolved closure

Exact-tag `packages/opencode/script/build.ts` invokes `Bun.build` with `compile`, includes a generated embedded web UI unless disabled, imports the OpenTUI Solid transform, embeds tree-sitter worker content, and installs platform-native OpenTUI/Parcel watcher/fff-bun inputs. This establishes build-input candidates, not a complete binary composition/SBOM.

The complete declared direct dependency inventory is retained verbatim as `metadata/packages__opencode__package.json` in the private review root. Notable declared families include Bun compile runtime; OpenTUI; Parcel watcher; fff-bun; tree-sitter Bash/PowerShell and web-tree-sitter; photon-node; Solid; AI SDK and provider adapters; AWS credential providers; MCP SDK; Effect; OpenTelemetry; Octokit; GitLab/auth plugins; SQLite-related ORM; ZIP implementation; and workspace OpenCode packages. Workspace/catalog entries require the exact lock/catalog closure; a manifest inventory does not settle which components are statically embedded, dynamically loaded, downloaded later or omitted in this exact build.

**Not established:** complete transitive/runtime closure; actual embedded component versions/licenses; corresponding notice texts; external native-tool requirements; offline operational closure beyond a future `--version` test; minimum CPU/libc suitability; binary signature; three-OS acceptance. Do not represent these unknowns as cleared or infer them from the root MIT license.

## Retained evidence and next gate

Private owned review root: `/data/git/vibevm-corp-agent-slice-20261119T154555Z/`. Downloaded archive is alone in `archive-input/`; fetched untrusted metadata is in `metadata/`; no interpreter/build is launched from either. `evidence/acquisition-card.json` retains exact asset fields, computed hash/size, source-file hashes and member inventory. Scripts/control remain separate.

Architect reviews the exact card and decides whether additional component/notice evidence or a different acquisition candidate is needed. Until then **HOLD corporate redistribution/drop inclusion**. Synthetic offline package lifecycle may be tested separately, explicitly not as OpenCode installation. Accepted APP-VERSIONS is reused, not reopened; final SYS parity and six-agent/platform acceptance remain unsatisfied.
