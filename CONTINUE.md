# CONTINUE — cold-resume checkpoint

_Written 2026-07-25 (session end: fact-grain campaign day three — the B2
modules sweep, batches 8–18). `spec/WAL.md` is the canonical living state
and supersedes this snapshot wherever they diverge._

## TL;DR

Progress-Control campaign (PROP-043, plan
`spec/terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.md`) is deep in **Phase B
at fact grain**. This session marked **18 modules files** (batches 8–18,
commits `b27336ae`…`1e7dff01`): PROP-015/034/027/036/030/011/012/010/040/
038/008/001/009/017/043/035/037/007 — **B2 stands at 32/35**; only the
three giants **PROP-005 (928 lines), PROP-003 (881), PROP-002 (736)**
remain in `spec/modules`, then `spec/design` / `spec/research` /
`spec/terraforms` (incl. the two pilot files carrying the wave's 40
expected errors). ~1 540 units became ~1 770 anchored facts; corpus:
94 files, 5 225/8 589 facts unmarked. Every marked file reads **0
unmarked / 0 issues**; `check` carries **only the 40 expected pilot
errors**. Seven new ledger rows (F-023…F-029), all one family: **stale
headers vs shipped reality** — plus one dangling cross-ref. **GitVerse
SSH is down all session** (banner-exchange timeout) — GitHub carries
everything; GitVerse is a clean ancestor, re-fan when the link recovers,
NEVER `--force`. The journal is clean — every step closed.

## State

- Branch `main`; **GitHub synced** through the session-end commits
  (`cargo xtask mirror` — github ok, gitverse FAIL on network). Working
  tree clean (untracked `.zcode/` only).
- **GitVerse**: SSH link down (banner-exchange timeout, all session).
  Verified at session start via HTTPS `ls-remote`: host `main` =
  `78322dac` — a **strict ancestor**, tags identical, **no foreign
  commits**. Verdict: transient link degradation, NOT divergence. Recovery
  = plain `cargo xtask mirror` when SSH works again. Standing red line:
  never `--force`. Useful trick: prefix the mirror with
  `GIT_SSH_COMMAND="ssh -o ConnectTimeout=15 -o BatchMode=yes"` — bounds
  each gitverse attempt to ~15 s and the fan-out still pushes github
  (failures are collected per-target, not abort-on-first).
- Floor: not re-run this session (docs-only markup edits + campaign
  state); `progress check` = 40 expected errors, `scan` green.
- Campaign journal clean (b2-prop-015…b2-prop-007 all step-done, counts
  written after the verifying scan).

## Resume recipe (exact)

1. Boot per `CLAUDE.md`, then read
   `campaigns/progress-2026-08/run/RESUME.md` and the plan LOG §9
   (`spec/terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.md`).
2. **B2 remainder — 3 modules files, then the clusters:**
   `spec/modules/vibe-index/PROP-005-package-index.md` (928 lines) →
   `spec/modules/vibe-resolver/PROP-003-dep-evolution.md` (881) →
   `spec/modules/vibe-registry/PROP-002-decentralized-registry.md` (736);
   then `spec/design/`, `spec/research/`, `spec/terraforms/` — including
   the re-mark of the two pilot files (`SHRINK-PLAN-v0.1.md` 28 errors,
   `design/README.md` 12) that burn down the 40 expected `check` errors.
3. **Per file (the proven loop):** journal `step-start`
   (`{"kind":"step-start","id":"b2-<slug>","step_type":"mark-file","target":"<path> (fact mark)","actor":"fable","ts":"<UTC>"}`
   appended to `campaigns/progress-2026-08/run/journal.jsonl`) → Read the
   file → write a python replace-script into the scratchpad (exact-match
   `rep(old, new)` with `assert in` + `assert count==1`, write via
   `open(p,'w',encoding='utf-8',newline='\n')`) → run → `cargo run -q -p
   vibe-cli --bin vibe -- progress check` (must stay at the expected-error
   count) + `scan` → corpus row must read **0 unmarked / 0 issues** →
   journal `step-done` with counts AFTER the scan. Batch commits of ~1–2
   files at this density (`docs(spec): B2 batch N — …`), campaign `run/`
   state + `resume` regeneration included; `cargo xtask mirror` at
   checkpoints.
4. **Marking conventions established this session** (follow them; the
   full trap list is below): Decision-paragraph idiom — first sentence
   stays the lead unit, the rest deconstructs into items; `##req-<slug>`
   / `##design-<slug>` for `` `req r1` ``-type lines; `##self-uri` for
   bare spec:// lines; ##status-line/##related/##milestone-line preamble
   split at bold-key boundaries; UPPER = normative, kebab =
   narrative/service; tables — every non-empty body cell marked, `ROW-*`
   id in the first cell (an em-dash placeholder cell counts as
   non-empty); checkbox lists — the anchor precedes `[x]`; unexecuted
   phase plans mint `@impl/plan`; superseded arcs mark spec/done vs
   authoritative impl/done; stale headers are LEDGER material (F-025
   class), never reworded in Phase B.
5. New DRIFT tasks (queue currently EMPTY): author per
   `spec/modules/vibe-progress/templates/impl-task.md`, spawn with
   `subagent_type: "opus5"`; review gate = read the diff, run the floor
   with the REAL exit code, close task file + INDEX + tasks.json +
   journal.

## Non-obvious findings from this session (the trap list)

- **The plus-wrap phantom struck 5 more times** (PROP-036/001×2/017) —
  a wrapped continuation line opening with `+ ` parses as a list item;
  the unmarked counter caught every one instantly (§5-B prediction keeps
  confirming). Also new: an **em-dash placeholder cell** (`| — |`) is a
  countable non-empty cell — mark it.
- **Blockquotes**: two more re-formed (PROP-034 lattice → fence,
  PROP-036 decision-record → bold lead + list, PROP-007 nesting
  principle → plain anchored paragraph verbatim, PROP-009 boot formula →
  fence). Keep the text verbatim when re-forming — only the `>`
  decoration goes.
- **Heading/fact one-namespace dodges**: `{#skill-include}` →
  `INCLUDE-DECL` (PROP-015), `{#show-origins}` → `show-origins-req`
  (PROP-040, the PROP-041 `-req` precedent).
- **The stale-header family is systemic**: F-025 (PROP-008), F-026
  (PROP-009), F-027 (PROP-017), F-028 (PROP-035), F-029 (PROP-007 —
  cross-doc: PROP-009 shipped its "remaining piece" the next day), plus
  F-024 (PROP-038 §2.7 vs §5) and F-023 (PROP-036 §2.13 cites PROP-043
  as a GUI-launcher home — dangling, the number was later taken by
  Progress Control). A Phase C/D sweep should fix all seven in one
  batch.
- **Bash heredoc quoting breaks on large python scripts** — write the
  script to the scratchpad with the Write tool and `python <file>`
  instead. Exact-match patterns must respect the file's real line
  wrapping (three MISS-retries this session were wrap-position guesses).
- **Journal step-start target names**: verify the real filename BEFORE
  writing the entry (two mangled entries had to be `sed`-fixed in place:
  PROP-034/PROP-036 name guesses).
- **`vibe progress resume` regenerates RESUME.md** — run it before each
  batch commit so `run/` state stays coherent.

## Standing decisions in force (unchanged; long form at the anchors)

Fact-exhaustive granularity (PROP-043 §3.9); anchored-when-marked (§3.8);
two anchor registers (UPPER/kebab); scope = 94 files (include-only
enumeration; WAL + generated boot pair OUT); verdicts live in
cache/baseline, never in markup; markers record the DOC's claims —
reality mismatches go to the ledger; **no fractality for this campaign**
(Fable = markup/verification/review/task-authoring, Opus = DRIFT coding);
coder-tier engine pin `claude-opus-5` (`.claude/agents/opus5.md`, live);
campaign zone excluded from scans/packaging; the surface is "dashboard";
the four CLAUDE.md rules bind every executor.

## Recent commits (this session, newest first)

```
1e7dff01 docs(spec): B2 batch 18 — PROP-007 marked at fact grain
af72e4ec docs(spec): B2 batch 17 — PROP-037 marked at fact grain
5ac3a38e docs(spec): B2 batch 16 — PROP-035 marked at fact grain
4fceb4b9 docs(spec): B2 batch 15 — PROP-043 marks itself at fact grain
438608f3 docs(spec): B2 batch 14 — PROP-009/017 marked at fact grain
d5b002fa docs(spec): B2 batch 13 — PROP-008/001 marked at fact grain
8264f11f docs(spec): B2 batch 12 — PROP-038 marked at fact grain
d916dd07 docs(spec): B2 batch 11 — PROP-010/040 marked at fact grain
0bfe20f9 docs(spec): B2 batch 10 — PROP-011/012 marked at fact grain
00f7a6e1 docs(spec): B2 batch 9 — PROP-027/036 marked at fact grain
b27336ae docs(spec): B2 batch 8 — PROP-015/034 marked at fact grain
b6421400 docs(wal): flag the GitVerse fan-out failure for next-session reconciliation
f1d5d47f docs(wal): session-end checkpoint — DRIFT loop 5/5, engine pin recorded
b2540f19 docs(continue): cold-resume checkpoint — fact-links complete, B2 at 14/35
5d567efd docs(spec): PROP-035 §7.3 heading-repeat precision; F-022 resolved
```

## Repository map (top level)

- `spec/` — the corpus: `common/` (12 PROPs, DONE), `modules/` (35 files,
  **32 marked**; PROP-005/003/002 remain), `design/` `research/`
  `terraforms/` (next clusters incl. the two pilot re-marks), `boot/`
  (authored 00-core/90-user + generated STATIC/INDEX), `WAL.md` (out of
  scan scope).
- `crates/` — host workspace: `progress-core` (fact scanner), `vibe-spec`
  (fact-aware compiler), `vibe-cli` (`commands/progress.rs`), the rest of
  vibe.
- `campaigns/progress-2026-08/` — `tasks/` (DRIFT-001…005 done),
  `run/` (journal, state/*.json — findings now 29 rows, RESUME, cache),
  `deferrals.md`, `harvest/`.
- `tools/progress-dashboard/serve.mjs` — dashboard; `tools/self-check.sh`
  — the floor.
- `.claude/agents/opus5.md` — the coder-tier agent type.

## Quick start

```bash
bash tools/self-check.sh                                  # the floor (REAL exit code)
cargo run -q -p vibe-cli --bin vibe -- progress check     # markup gate (40 expected pilot errors)
cargo run -q -p vibe-cli --bin vibe -- progress scan      # refresh state
cargo run -q -p vibe-cli --bin vibe -- progress resume    # regenerate RESUME.md
node tools/progress-dashboard/serve.mjs                   # dashboard
cargo xtask mirror                                        # fan-out (gitverse fails while SSH is down — expected)
```
