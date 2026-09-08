# Final archaeology index — lifecycle/extensions campaign

Read-only census made on 2026-09-08 for plan atom `FINAL-ARCHAEOLOGY` and
mandate M-012. Audit base: local `main` at `ea737d37`. This index authorizes no
deletion: a branch, worktree, report or cache family remains until its recorded
promotion/equivalence/rejection is accepted and cleanup is separately chosen.

## Classification vocabulary

- **equivalent** — exact stable patch equivalence, or an identified `main`
  reauthoring that contains the branch result plus named corrections.
- **promote** — unique facts remain useful and have an authoritative target.
- **obsolete** — an abandoned topology/design whose useful rule is already in
  the named current implementation or whose rejection is explicit.
- **retain** — accepted audit evidence remains intentionally local.
- **unverifiable** — filenames/status prove evidence exists, but the allowed
  audit did not establish a report-level acceptance or rejection.

## Twenty-seven local branches not merged by ancestry

`git cherry main <branch>` supplied stable patch equivalence. A same-subject
main commit was additionally compared on the branch commit's touched paths;
"reauthored" below never means an unexamined subject-only guess.

| branch | patch census | disposition | main evidence / remaining action |
|---|---:|---|---|
| `codex/campaign-corpus-migration` | 0 unique / 2 equivalent | equivalent | exact stable patch equivalence |
| `codex/r2-5-freshness` | 0 / 1 | equivalent | exact stable patch equivalence |
| `codex/r2-7-query` | 0 / 1 | equivalent | exact stable patch equivalence |
| `codex/r2-8-commission` | 1 / 0 | equivalent | reauthored as `3137990c`; main drops two overclaimed spec edges |
| `codex/r3-2-parse` | 0 / 3 | equivalent | exact stable patch equivalence |
| `codex/r3-artifact-unit` | 1 / 0 | equivalent | reauthored as `6f3fa61a`; touched product files match |
| `codex/r3-assemble-v2` | 1 / 1 | equivalent | unique commit reauthored as `302a3509`; the other is patch-equivalent |
| `codex/r3-embed` | 0 / 1 | equivalent | exact stable patch equivalence |
| `codex/r3-merge` | 0 / 1 | equivalent | exact stable patch equivalence |
| `codex/r3-verifier` | 0 / 1 | equivalent | exact stable patch equivalence |
| `codex/r34-writer` | 0 / 1 | equivalent | exact stable patch equivalence |
| `codex/r4-comment-codec` | 1 / 0 | equivalent | reauthored as `fbbd5140`; main adds the governing spec scope |
| `codex/r6-ir-wire` | 0 / 1 | equivalent | exact stable patch equivalence |
| `codex/r7-agent-cli` | 1 / 0 | equivalent | reauthored/corrected as `26929050`; later R7 commits supersede its old combined perimeter |
| `codex/r7-provider` | 1 / 1 | equivalent | unique wire commit reauthored as `f42334ff`; other commit patch-equivalent |
| `codex/r8-mechanism-grammar` | 0 / 1 | equivalent | exact stable patch equivalence |
| `codex/r8-package-binding` | 0 / 1 | equivalent | exact stable patch equivalence |
| `codex/r8-receipt-repair` | 0 / 1 | equivalent | exact stable patch equivalence |
| `codex/r8-skill` | 1 / 0 | equivalent | reauthored as `c0fa49be`; main adds spec metadata |
| `cultural-backup` | 20 / 0 | promote then obsolete | rejected v1 backup at `0eb32026`; R1–R10/gotchas harvested into `neworder2/memory/EXTRACTION-PROCESS.md`; no code cherry-pick |
| `wt/E10-W1-CONFIG-V2` | 2 / 0 | obsolete | old `packages/`/v0.x topology; current v1 engine contains the per-language config (`97688de0` plus later rules) |
| `wt/E10-W2A-RUST-FE` | 2 / 1 | obsolete | W2A equivalent to `aa4b3a72`; remaining W1 staging superseded by current v1 engine |
| `wt/E10-W2B-GO-TS-FE` | 2 / 1 | obsolete | W2B equivalent to `51e350bf`; remaining W1 staging superseded by current v1 engine |
| `wt/r34-command-core` | 0 / 1 | equivalent | exact stable patch equivalence |
| `wt/r34-install-lifecycle` | 0 / 1 | equivalent | exact stable patch equivalence |
| `wt/r34-threading` | 1 / 0 | equivalent | reauthored as `be04a184`; main adds explicit traced/untraced metadata |
| `wt/r34-wire-state` | 0 / 2 | equivalent | exact stable patch equivalence |

No unique lifecycle/extensions product patch remains on these refs. The only
promotion was cultural-run wisdom; its old product tree remains deliberately
rejected in favor of `cultural-refactor` and the current extraction method.
The v1 concept count/commit table, parked-specmap state, one already-fixed
broken link and legacy bare-address spellings are historical run facts, not
portable current rules, and are deliberately not promoted.

## Local refs already merged

`git for-each-ref --merged=main refs/heads` reports **132 local refs including
`main`** (131 non-main refs). They are one **equivalent-by-ancestry** class: no
patch promotion is required. Four remote refs (`origin`, `origin/main`,
`github`, `github/main`) are tracked separately and are not included in 132.
Names are intentionally not duplicated here; Git is the exhaustive index for
this ancestry class. Deletion remains outside this atom.

## Auxiliary worktrees

The packet expected eleven auxiliaries. The live Git registry reports twelve,
so this index records all twelve rather than hiding the drift. Eleven have an
equivalent one-commit HEAD and only an untracked packet/report pair; those
reports remain local evidence and were not read during this audit.

| worktree | HEAD | product disposition | local evidence disposition |
|---|---|---|---|
| `.wt/R5.3-GATE` | `8cdad7fd` | patch-equivalent | retain `PACKET-R5.3-GATE.md` and `WORKER-REPORT-R5.3-GATE.md` until report-level indexing |
| `.wt/R5.3-WIRING` | `ef4c16b2` | patch-equivalent | retain packet/report pair |
| `.wt/R5.5-INVOKE-ARTIFACT` | `c32a77fb` | patch-equivalent | retain packet/report pair |
| `.wt/R5.5-INVOKE-GATE` | `661bc577` | patch-equivalent | retain packet/report pair |
| `.wt/R5.5-INVOKE-LOADER` | `0ef5d42d` | patch-equivalent | retain packet/report pair |
| `.wt/R5.5-INVOKE-MANAGER` | `39eeced5` | patch-equivalent | retain packet/report pair |
| `.wt/R5.5-INVOKE-SDK` | `5883fb77` | patch-equivalent | retain packet/report pair |
| `.wt/R5.5-WIRE` | `fc9eb0d8` | patch-equivalent | retain packet/report pair |
| `.wt/R5.5-WIRE-GATE` | `18715315` | patch-equivalent | retain packet/report pair |
| `.wt/R5.5-WIRE-PROJECTION` | `dd50a080` | patch-equivalent | retain packet/report pair |
| `.wt/R5.5-WIRE-PROJECTION-TRACE-V1` | `2dec1d7a` | patch-equivalent | retain packet/report pair |
| `.wt/R5.5-WIRE-PROJECTION-TRACE` | `9051aade` | obsolete prototype, not cleanup-ready | HEAD is an ancestor, but seven dirty paths contain the v0.8 shared-JTD prototype and packet; accepted v1 implementation is `dc9cff48`. Retain until its untracked `shared.rs`/tests are explicitly compared with the v1 implementation or rejected. |

The twelfth worktree is the remaining archaeology blocker; no removal or move
is authorized here.

## Cache-family index

Only filenames, family counts and explicitly referenced accepted reports were
read. Raw JSONL/transcripts, token-bearing files and arbitrary worker outputs
were not opened. Empty directories carry no evidence and are omitted.

### Promoted upstream/research snapshots

| family | campaign owner | disposition |
|---|---|---|
| `agent-plugins-spec` | R8 client/plugin architecture | equivalent; upstream commit `ff8ab5e392cc87bd88d87c060815a87490e51003` and conclusions are recorded in the architecture; retain until final inspection |

### Exact accepted artifacts retained by mandate

| family / exact artifact | accepted node | disposition |
|---|---|---|
| `sol-to-opus-handoff-preflight/CURRENT-LIFECYCLE-CAMPAIGN-STATE.md` | `MUP-6`, `FINAL-ARCHAEOLOGY` | retain: accepted emergency/handoff archaeology evidence |
| `r5-native/R5.4-R5.5-ROUTE-REVIEW.md` | R5.4/R5.5 route nodes | retain: accepted serial-route decision evidence; decisions are implemented on main |
| `zai-glm-e2e/case-20260904T125538100Z-b7002b380e2841568aa5859c5eabf5c4/worker/WORKER-REPORT-E2E.md` | `ZAI-GLM` | retain: exact accepted `GLM_E2E_OK` evidence |
| `zai-glm-e2e/case-20260904T125538100Z-b7002b380e2841568aa5859c5eabf5c4/live-pass-1-summary.json` | `ZAI-GLM` | retain: accepted pass/session/model-usage summary |

### Accepted campaign families retained pending report-level disposition

Every exact nonempty family is named below. Product decisions are accepted in
the named campaign/node, but the family may mix implementation, review,
correction and rejected-run evidence. Therefore **retain** is exact and
"every report is accepted" is deliberately not claimed.

| owner/node | nonempty cache families | disposition |
|---|---|---|
| MUP | `multi-user-planning-package-claudez`, `multi-user-planning-package-opus` | retain historical implementation/review evidence; package result is accepted |
| R2/R3/R4/R5/R6/R7/R8 early worker archive | `agents` | retain; 128 files/93 report-named files are mixed and remain unverifiable without a per-report map |
| R4 | `r4`, `r4-0-impl-claudez`, `r4-0-inventory-opus`, `r4-1-activation-claudez`, `r4-1-inventory-opus`, `r4-1-owner-controls-claudez`, `r4-1-transform-plan-opus`, `r4-1-unit-transaction-claudez`, `r4-1-world-adapter-claudez` | retain historical R4 evidence |
| R4.1 atoms | `r4-t1-config-digest-claudez`, `r4-t1-t3-correction-claudez`, `r4-t10`, `r4-t2-design-opus`, `r4-t2-transform-plan-claudez`, `r4-t3-selector-claudez`, `r4-t4-artifact-plan-recon-claudez`, `r4-t5-behavior-recon-claudez`, `r4-t6-positions-recon-claudez`, `r4-t6b-reds-claudez`, `r4-t6b-seam-claudez`, `r4-t6c`, `r4-t7`, `r4-t7-subject-wire-recon-claudez`, `r4-t8`, `r4-t9` | retain; accepted implementation is on main, report-level reasons remain evidence |
| R5 native | `r5-native` | retain; exact route review above is accepted, remaining reviews/recon are historical |
| R7.4 | `r74-a12`, `r74-a13`, `r74-a14`, `r74-a15-design`, `r74-a15a-package-source`, `r74-a15b-command`, `r74-a15c1-hosted-backend`, `r74-a15c2-tool-output`, `r74-a9`, `r74-bounded-publish-verify`, `r74-bounded-read`, `r74-cli-goldens`, `r74-dependency-audit`, `r74-lease`, `r74-lease-opus-review`, `r74-lease-plumbing`, `r74-mcp-audit`, `r74-opus-adjudication`, `r74-projection`, `r74-selected-map`, `r74-selected-plumbing`, `r74-selected-resume-repair`, `r74-selected-state`, `r74-state-audit`, `r74-state-io`, `r74-state-opus-review`, `r74-tasks-reader`, `r74-tool-output`, `r74-wire` | retain mixed accepted/rejected worker evidence; R7.4 product is on main |
| R7.5 | `r75-p2-a4-audit`, `r75-p2-a4b-design`, `r75-p2-a4b-implementation`, `r75-p2-a4c-design`, `r75-p2-a4c0-streaming`, `r75-p2-a4c1-artifacts`, `r75-p2-a5a-opus`, `r75-p2-a5b-opus`, `r75-p2-recon`, `r75-p3-cli-opus`, `r75-p3-mcp-opus`, `r75-p3-pdsa-opus` | retain; acceptance/corrections are on main, individual reports not globally indexed |
| R8 | `r8`, `r8-client-probe-marketplace`, `r8-clients` | retain architecture, probe and worker evidence; product is on main |
| R3.4 review | `review-r34-conform`, `review-r34-exact-event` | retain accepted review packets/reports |
| ZAI launcher | `zai-glm-launcher-contract` | retain fake-token/launcher contract evidence; package result is accepted |

### Mixed, obsolete or operational cache families

| family | disposition |
|---|---|
| `zai-glm-e2e` except the exact accepted case above | unverifiable/mixed; retain failed or superseded attempts until a per-case disposition exists |
| `scrape-release` | obsolete/incomplete worker packet: `GLM-SCRAPE-AB.md` is not the promised `GLM-REPORT-AB.md`; product acceptance must be read from main, not inferred from this file |
| `chatgpt-cargo-slots` | live ephemeral semaphore files; never clean while any campaign command may be running |
| `zai-glm-launcher-contract` fake binaries/token fixture | test evidence only; filenames explicitly identify fake data, but no secret content was read |

## Promotion result and remaining blockers

Promoted now:

1. R1–R10 have explicit current dispositions in
   `neworder2/memory/EXTRACTION-PROCESS.md`.
2. R4/R5/R6/R8 linked-mutation and install-lock gotchas missing from that
   method were promoted there.
3. The dangling references to rejected `neworder2/report.md` were removed;
   this index now records the v1 source and rejection.

No new root `BACKLOG.md` entry is needed: the remaining work is archaeology
evidence classification, recorded here rather than an unstarted product bug.

Blockers before any cleanup:

- the unexpected twelfth worktree's untracked shared-JTD prototype/tests need
  explicit equivalence or rejection against accepted `dc9cff48`;
- root backlog B-115 identifies
  `cache/r4-t6b-reds-claudez/REVIEW-NOTES-t6b-round1.md` as the sole rationale
  for still-unpromoted T6b decisions, notably why opaque
  `TransformCompileError::source()` retains its private typed source; promote
  those rulings into the governing R4 ABI/PROP decision record before cleanup;
- `agents` and the mixed R4/R7/R8 report families lack per-report
  accepted/corrected/rejected mapping;
- non-winning ZAI E2E cases lack explicit per-case disposition;
- live cargo-slot semaphore files cannot be treated as stale while commands
  may still run.
