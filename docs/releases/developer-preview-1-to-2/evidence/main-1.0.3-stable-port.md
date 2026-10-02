# Stable 1.0.1–1.0.3 changes in `next`

Source tags on the public GitVerse mirror: `v1.0.1` at `f53a5b23f`,
`v1.0.2` at `c10082e8c`, and `v1.0.3` at `16bdc30cb`. The `next` line
remains the mutable 1.1.0 product. No commit in this port publishes that
line or changes its version back to a stable tag.

The [1.0.1 port record](main-1.0.1-hotfix-port.md) accounts for the ten
commits between the `next` fork point (`30d217bfc`) and `v1.0.1`. Seven
earlier commits in `v1.0.0..v1.0.1` are already ancestors of `next`.

| Stable commit | Intent and `next` disposition |
| --- | --- |
| `306162afb` | Make the image publishing step name independent of its version; already included in `b6458a573` as recorded by the 1.0.1 port. |
| `c10082e8c` | Remote Streamable HTTP and user-scoped MCP packages with native client registration and owned receipts; ported in `e0354ed5e`, retaining local stdio and next's migration authority. |
| `d5d169068` | Identify version/download/checkout resolution work and the actual install destination; ported in `3e0a558ad`. |
| `d0b5e2504` | Global MCP progress, agent/server subset choice, selective owned removal and injected native directories; ported in `216595355`. |
| `2e511f4d9` | Suspend init progress for the full input dialog; ported in `30702c6ce`. |
| `6a50de54d` | Root project/package role, group and safe typed manifest writing; ported in `065831085` with current next layout, project `sources` and nested `--link` preserved. |
| `e31234290` | Explain the group/name coordinate before role and metadata prompts; final owner refinement ported in the same coherent root-identity commit `065831085`. |
| `16bdc30cb` | Release 1.0.3 version pins and publication defaults remain specific to the stable line. Direct lockfile comparison found 35 local 1.0.2→1.0.3 version changes, zero external row or selection changes. `f8184b7ad` records the stable source tag and byte-identical historical changelog sections, without implying binary assets or publishing next. 1.1.0 pins and the no-publication hold remain authoritative. |

Accepted focused evidence so far: general resolution 6 registry observation,
14 existing walk, 5 package-source, 3 CLI confirmation and 3 doctest cases;
remote MCP base 66 parent-side manifest, ownership, native-client, global,
local-stdio and negative/doctest cases; MCP follow-up 60 parent-side
selection/confirmation/registration/injected-path cases; root identity 9
unit plus 26 `cli_init` integration cases in primary, with the unchanged
long-input progress consumer 1/1 in the worker. Init's new long-input case and the
existing nested-unwind case both pass under LLVM; the latter fails under the
machine's optional Cranelift backend even on the unchanged `next` baseline.
The source/target identity is retained in local commits and the permanent
spec anchors; no full release panel or mutation campaign was run.

Traceability generation on the integrated tree reports zero suspect and
unresolved host edges, with 113 pre-existing gated orphans from the larger
`next` campaign. Its broad derived-map delta remains outside these atomic
stable-port commits while that baseline is reconciled. The scoped port ran
no full release panel, public source push, image workflow or mutation campaign.
