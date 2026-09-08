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
| `.wt/R5.5-WIRE-PROJECTION-TRACE` | `9051aade` | superseded prototype | Four modified and two untracked code paths implement only the old v0.8 host slot, with an untracked execution packet. Accepted `dc9cff48` reauthors the feature in v1, adds contained-path admission, and mirrors it into every shipped vendor copy. The prototype is not equivalent and has no promotable delta; retain only as rejected pre-correction evidence. |

The unexpected twelfth worktree is dispositioned, not cleaned. No removal or
move is authorized here.

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

### Report-level disposition ledger

The **80 nonempty top-level cache families** are exhaustively mapped before
the finer report census. Wildcards here are closed prefix classes, not an
assumption about future directories.

| exact family scope | families | accepted commit/node | family disposition |
|---|---:|---|---|
| `agent-plugins-spec` | 1 | R8 client/plugin architecture | equivalent upstream/research snapshot; retain provenance |
| `agents` | 1 | R1–R8 accepted nodes | mixed worker archive; report-level outcomes are mapped below and retained as historical evidence |
| `multi-user-planning-package-*` | 2 | MUP-6 | corrected/superseded by the accepted package; retain independent implementation/review evidence |
| `r4*` | 25 | R4.0/R4.1 T1–T10 | corrected/superseded by accepted atom results; retain design, mutation and review evidence |
| `r5-native` | 1 | R5.1–R5.5 | accepted route plus superseded recon/reviews; retain native ABI evidence |
| `r74-*` | 29 | R7.4 A6–A15 | corrected/superseded by accepted R7.4 atoms; retain commissioning and review evidence |
| `r75-*` | 12 | R7.5 P2/P3 | corrected/superseded by accepted R7.5 atoms; retain streaming/tool-boundary evidence |
| `r8*` | 3 | R8 clients/package/deploy | accepted architecture/probe plus superseded worker reviews; retain client evidence |
| `review-r34-*` | 2 | R3.4 gates | accepted independent review evidence; retain exactly |
| `scrape-release` | 1 | scrape release | rejected as acceptance evidence because the promised report is absent; retain the incomplete outcome |
| `sol-to-opus-handoff-preflight` | 1 | MUP-6 / FINAL-ARCHAEOLOGY | accepted emergency handoff evidence; retain exactly |
| `zai-glm-e2e` | 1 | ZAI-GLM | one accepted case and four non-winning cases dispositioned below; retain operational evidence |
| `zai-glm-launcher-contract` | 1 | ZAI launcher contract | accepted fake-token contract evidence; retain fixtures without reading token bodies |

These closed scopes total **80/80** currently nonempty cache families.

The report census is exact and reproducible: every nonempty `.md`/`.txt` file
under `cache/` whose basename contains `report`, `review`, `notes`, `followup`,
`continue`, or `CURRENT-LIFECYCLE-CAMPAIGN-STATE`, excluding packet inputs, is
in one scope below, as are the three nonempty JSON review/summary artifacts. A
scope groups reports only where every member has the same accepted node and
disposition. “Superseded” applies to the report as a proposed or intermediate
result, not to the retained historical evidence file.

| exact report scope | count | accepted commit/node | report disposition and retention reason |
|---|---:|---|---|
| `cache/agents/R1-*/*` | 1 | accepted R1 line | corrected/superseded by the landed R1 result; retain the derived-order repair evidence |
| `cache/agents/R2*/*` | 24 | R2.5–R2.8 | design, implementation, repair and review reports are corrected/superseded by the accepted R2 nodes on `main`; retain the iteration and refusal evidence |
| `cache/agents/R3*/*`, `cache/agents/OPUS-R3*/*` | 44 | R3.2–R3.4 | corrected/superseded by the accepted R3 nodes; retain design, paused-attempt, repair and verifier-review history |
| `cache/agents/R4*/*` | 4 | R4/R4.1 | architecture/design/review inputs are superseded by the accepted R4 implementation and later corrections; retain rationale evidence |
| `cache/agents/R5*/*` | 1 | R5 native | scouting input superseded by the accepted R5 wire/loader/artifact route; retain architecture evidence |
| `cache/agents/R6*/*` | 2 | R6/R6.2B | reconnaissance inputs superseded by accepted R6 atoms; retain boundary evidence |
| `cache/agents/R7*/*` | 6 | R7/R7.1/R7.3 | repair/review/design reports corrected or superseded by accepted R7 commits; retain acceptance history |
| `cache/agents/R8*/*` | 8 | R8/R8.1/R8.2B | implementation/review/specmap/design reports corrected or superseded by accepted R8 commits; retain acceptance history |
| `cache/agents/RETROSPECTIVE-SPEC-HARVEST/*` | 1 | FINAL-SPEC-PROMOTION | superseded by tracked spec-promotion results; retain the harvest inventory |
| `cache/multi-user-planning-package-*/*` | 2 | MUP-6 | corrected/superseded by the accepted multi-user-planning package; retain independent implementation/review evidence |
| human-readable `cache/r4*/*` `.md`/`.txt` reports matching the basename filter and packet exclusion above | 49 | R4.0/R4.1 T1–T10 | all reports are corrected/superseded by accepted atom results; retain mutation, correction and review history. T6b's last cache-only ruling is now authoritative in ABI §6.3 |
| `cache/r4-0-impl-claudez/run-review.json`, `cache/r4-t6b-seam-claudez/run-current-review.json` | 2 | R4.0 / R4.1 T6b | machine review artifacts corrected/superseded with their sibling review reports by accepted atom results; retain without treating them as authority |
| `cache/r5-native/R5.4-R5.5-ROUTE-REVIEW.md` | 1 | accepted R5.4/R5.5 route | accepted decision evidence; retain exactly under M-012 |
| `cache/r5-native/*` except the route review | 8 | R5.1–R5.3 | recon and central reviews are corrected/superseded by accepted R5 commits; retain the review chain |
| `cache/r74-*/*` | 47 | R7.4 A6–A15 | intermediate design, implementation, follow-up, audit and review reports are corrected/superseded by accepted R7.4 atoms; retain bounded-publish/lease/state evidence |
| `cache/r75-*/*` | 16 | R7.5 P2/P3 | intermediate recon/design/continuation/implementation reports are corrected/superseded by accepted R7.5 atoms; retain streaming/artifact/tool-boundary evidence |
| `cache/r8/*`, `cache/r8-clients/*` | 14 | R8 clients/package/deploy | worker and central-review reports are corrected/superseded by accepted R8 client commits; retain client integration evidence |
| `cache/review-r34-*/*` | 2 | R3.4 review gates | accepted independent review evidence; retain exactly |
| `cache/sol-to-opus-handoff-preflight/CURRENT-LIFECYCLE-CAMPAIGN-STATE.md` | 1 | MUP-6 / FINAL-ARCHAEOLOGY | accepted emergency handoff snapshot; retain exactly |
| accepted `cache/zai-glm-e2e/.../worker/WORKER-REPORT-E2E.md` | 1 | ZAI-GLM | accepted `GLM_E2E_OK` evidence; retain exactly with its summary |
| accepted `cache/zai-glm-e2e/.../live-pass-1-summary.json` | 1 | ZAI-GLM | accepted machine pass/session/model-usage summary; retain exactly without opening raw logs |

The scopes total **235/235** nonempty report/review/summary artifacts: 232
human-readable reports and three machine JSON summaries. Packet inputs, raw
JSONL, binary logs, token files and live semaphore/lock files are not reports
and are not opened or counted. `cache/scrape-release/GLM-SCRAPE-AB.md` is
additionally indexed as a misnamed, incomplete worker outcome: reject it as
acceptance evidence because it is not the promised `GLM-REPORT-AB.md`; retain
it only to explain that failed delivery.

### Mixed, obsolete or operational cache families

| family | disposition |
|---|---|
| `zai-glm-e2e/case-20260904T124554643Z-8b6d77dfcd314cda842422e895c829d1` | rejected as acceptance evidence: no nonempty report or pass summary exists; retain its raw operational corpus without opening it |
| `zai-glm-e2e/case-20260904T124628892Z-b4a01e12b40242e79d0e83299b0b4231` | rejected as acceptance evidence: no nonempty report or pass summary exists; superseded by the accepted case and retained as operational failure history |
| `zai-glm-e2e/case-20260904T125131290Z-c268a8520ba14c1f94e681b75484143c` | rejected as acceptance evidence: no nonempty report or pass summary exists; superseded by the accepted case and retained as operational failure history |
| `zai-glm-e2e/case-20260904T130412989Z-ef3a33f1f1104f21846bd2bbcfd92f65` | rejected as acceptance evidence: no nonempty report or pass summary exists; later than but does not replace the exact accepted case, so retain as non-winning operational history |
| `scrape-release` | obsolete/incomplete worker packet: `GLM-SCRAPE-AB.md` is not the promised `GLM-REPORT-AB.md`; product acceptance must be read from main, not inferred from this file |
| `chatgpt-cargo-slots` | live ephemeral semaphore files; never clean while any campaign command may be running |
| `zai-glm-launcher-contract` fake binaries/token fixture | test evidence only; filenames explicitly identify fake data, but no secret content was read |

## Promotion result and closure

Promoted now:

1. R1–R10 have explicit current dispositions in
   `neworder2/memory/EXTRACTION-PROCESS.md`.
2. R4/R5/R6/R8 linked-mutation and install-lock gotchas missing from that
   method were promoted there.
3. The dangling references to rejected `neworder2/report.md` were removed;
   this index now records the v1 source and rejection.
4. T6b's opaque-error causal-chain ruling is a four-field decision in
   `R4-TRANSFORM-PLAN-ABI-v0.1.md` §6.3; B-115 is closed.
5. All 235 nonempty report/review/summary artifacts have report-level
   dispositions, all five ZAI cases have case-level dispositions, and the
   dirty TRACE prototype is explicitly superseded by corrected commit
   `dc9cff48`.

No new root `BACKLOG.md` entry is needed: the archaeology dispositions are
complete, and cleanup is deliberately outside this atom rather than a product
bug.

No archaeology classification blocker remains. Cleanup is still a separate,
unauthorized action: branches, worktrees, cache reports, raw operational logs,
and live cargo-slot semaphore files all remain in place. Semaphore/lock state
must never be inferred stale while campaign commands may run.
