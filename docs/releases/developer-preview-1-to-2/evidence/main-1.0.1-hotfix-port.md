# Main 1.0.1 hotfix port into next

Source: public `vibevm/vibevm` main at `306162afbe187ecb4a66608e8bcfd1ed8ae3d10d`,
starting with `c8784ffca631b0d787a4731e5dcb0bc5d4617d20` (11 commits).
The `next` branch keeps the mutable product and active owned-package version
1.1.0. This port has not been pushed or published.

| Main commit | Next disposition |
| --- | --- |
| `c8784ffca` | Ported nested Markdown item UTF-8/task-box parser fix in `722a0f2ee`. |
| `d00ed4867` | Ported documentation corpus pins in `722a0f2ee`, adapted to the existing 54-page `v1.1.0` package. |
| `d478b32b0` | Ported nested boot-zone desubstitution fixpoint in `b942b96d3`. |
| `8c922f850` | Ported top-level `run` progress inventory in `b6458a573`. |
| `bbe5865a6` | Ported static-mirror catalog search in `d7601b96d`, sharing the server's tokeniser/ranking and retaining the existing catalog reader. |
| `7db08cda5` | Ported lifecycle verify-fence golden in `b6458a573`. |
| `232e04445` | Ported progress-independent stderr assertions in `b6458a573`; the scrape case is Windows-only. |
| `359518cc6` | Ported Russian documentation judging exemption in `b6458a573`. |
| `7bdff4376` | Ported section-based requirements format assertion and closed B-175 in `b6458a573`. |
| `f53a5b23f` | Release metadata, lockfiles, installer and workflow version substitutions for 1.0.1 were excluded; they conflict with the 1.1.0 next version contract. |
| `306162afb` | Ported the version-neutral Docker workflow step name in `b6458a573`; no workflow was run. |

Focused proof: nested specdoc regressions 2/2 and documentation corpus 8/8;
workspace nested-zone 1/1, replay 9/9 and inheritance parity 4/4;
wire search 8/8, registry static-search unit/integration 6/6 plus existing
search 7/7, index search 3/3 and CLI static-mirror search 1/1. The exact
requirements-format, judging-exemption, CLI progress inventory, lifecycle
golden and progress-sidecar tests each passed. Package-scoped formatting and
`git diff --check` passed. The Windows-only scrape stderr test was compiled on
Linux but selected zero runnable cases; native Windows proof remains open.

The imported `d478b32b0` fix is relevant evidence for planned M-06-G.3, but
that task still requires its own next fixture for generated slot exclusion and
workspace source-member nonmutation. The 1.0.1 port does not accept M-06-G.3
or waive its Windows checks.
