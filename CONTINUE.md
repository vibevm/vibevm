# CONTINUE — cold-resume checkpoint

_Written 2026-07-25 (session end: fact-grain campaign day three, part 2 —
the owner scope ruling + PROP-005). `spec/WAL.md` is the canonical living
state and supersedes this snapshot wherever they diverge._

## TL;DR

Progress-Control campaign (PROP-043, plan
`spec/terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.md`), **Phase B at fact
grain, near the finish line.** Two things happened in this closing
stretch: **(1) the owner ruled `spec/terraforms`, `spec/research`,
`spec/neworder` OUT of scope** (old refactorings/research — historical
records, not living contracts; ruling verbatim in the plan LOG §9;
`progress.toml` narrowed, commit `8901cd05`) — scope is now **59 files /
4 894 facts**; **(2) PROP-005 was marked** (273→278 facts, batch 19,
`e9d330f8`) plus the forgotten PROP-030 markup finally committed
(`ff4ca088`). Earlier the same day: the 18-file B2 modules sweep
(batches 8–18) and the GitVerse diagnosis. **Remaining to close Phase
B: 14 files, ~1 280 facts** (list below). `check` carries **12 expected
errors** (design/README pre-amendment markup — burns at its re-mark).
Journal fully closed (b2-prop-003 was opened and closed as NOT STARTED
— zero edits). GitHub synced; GitVerse SSH still down (clean ancestor,
plain re-fan on recovery, never `--force`).

## How to start the next session (exact)

Открой сессию фразой вида:

> Продолжаем спец-актуализационную кампанию (PROP-043). Прочитай
> CONTINUE.md, затем campaigns/progress-2026-08/run/RESUME.md и LOG §9
> плана. Продолжай B2-хвост фазы B по списку из CONTINUE.md, начиная с
> PROP-003. Разметка — Fable, без fractality; DRIFT — субагентом opus5.

Then the executor does, in order:

1. Boot per `CLAUDE.md`; read this file, `run/RESUME.md`, plan LOG §9.
2. Journal `step-start` for `b2-prop-003` (it was opened and closed
   NOT-STARTED last session — open a fresh step), mark
   `spec/modules/vibe-resolver/PROP-003-dep-evolution.md` (881 lines,
   310 facts; **its §2.2 carries a SUPERSEDED-by-PROP-017 blockquote —
   re-form per F-015**; the vocabulary sections are impl'd via resolvo,
   the libsolv §2.2/§3.x sections are superseded → spec/done per the
   PROP-001/PROP-026 arc pattern), verify (`check` stays at 12,
   corpus row 0/0), `step-done`, commit.
3. Continue the worklist (per-file loop identical): PROP-002 (359;
   registry contract, impl'd, has superseded-parts too) → design
   cluster: `design/README.md` re-mark (18 unmarked + **burns the last
   12 expected errors** → after it `check` must be **0**),
   `structural-loader` (20), `loading-and-boot-model` (74),
   `action-system` (78), `workspace-and-qualified-naming` (99),
   `tui-visual-language` (175) → boot pair `00-core.md` (33) /
   `90-user.md` (31) — **user-owned: additive markers ONLY, zero
   re-forming, zero deconstruction** (owner scope ruling keeps them
   observed) → manual-tests MT-01 (28) / MT-02 (23) / MT-03 (16; the
   manual-tests genre: steps are units; Expected paragraphs too) →
   `discipline/README.md` (16).
4. **Phase B exit:** when the corpus reads 0 unmarked / 0 issues
   everywhere and `check` = 0 errors, run
   `cargo run -q -p vibe-cli --bin vibe -- progress check --exhaustive`
   (must be green) + `bash tools/self-check.sh` (REAL exit code), write
   the Phase-B boundary entry in the plan LOG §9 (commit map, gate
   state, predictions check per §5), refresh the plan status line,
   journal a `{"kind":"phase"}` event is NOT hand-written — the phase
   lane derives from the journal (DRIFT-003); ask the owner before
   declaring Phase C open (phase transitions are the owner's call).
5. Batch commits ~1–3 files; `vibe progress resume` before each commit;
   `cargo xtask mirror` at checkpoints (gitverse will FAIL while its
   SSH is down — expected; github must go `ok`). Prefix with
   `GIT_SSH_COMMAND="ssh -o ConnectTimeout=15 -o BatchMode=yes"` to
   bound the gitverse attempts.

## The per-file loop (proven, do not improvise)

journal `step-start` (`{"kind":"step-start","id":"b2-<slug>","step_type":"mark-file","target":"<real path> (fact mark)","actor":"fable","ts":"<UTC>"}`
— verify the filename BEFORE writing the entry) → Read the file → write
python replace-scripts into the scratchpad (`rep(old,new)` with
`assert a in s` + `assert s.count(a)==1`; write back via
`open(p,'w',encoding='utf-8',newline='\n')`; patterns must copy the
file's REAL line wrapping) → run → `progress check` (stays at the
expected-error count) + `progress scan` → corpus row **0 unmarked / 0
issues** → journal `step-done` with counts AFTER the scan → commit
(`docs(spec): B2 batch N — …`, campaign `run/` included).

## Marking conventions (established; follow verbatim)

- Registers: `##UPPER-SLUG` = normative fact, `##kebab` =
  narrative/service. Preamble splits at bold-key boundaries:
  `##milestone-line` / `##status-line` / `##related` / … ; bare
  `spec://` line → `##self-uri`.
- `` `req rN` `` lines → `##req-<section-slug>`; `` `design rN` `` →
  `##design-<slug>`.
- **Decision-paragraph idiom:** first sentence stays the lead unit;
  remaining facts deconstruct into items (sense- and
  wording-preserving; semantic edits FORBIDDEN in Phase B).
- Tables: every non-empty body cell marked; `ROW-*` id in the first
  cell; an em-dash placeholder cell counts as non-empty.
- Checkbox lists: anchor precedes `[x]` (`- ##ACC-X [x] …`).
- Blockquotes cannot carry anchors (F-015): re-form as a plain
  anchored paragraph **verbatim** (or a fence for display formulas).
- Stage/state records the DOC's claims: impl'd content `@impl/done`,
  drafted spec `@spec/done`, open questions `@spec/work`, unexecuted
  phase plans `@impl/plan`, superseded arcs `@spec/done` vs
  authoritative `@impl/done`. Stale headers → LEDGER (F-025 class),
  never reworded.
- Heading/fact ids share one namespace per doc — dodge collisions with
  a suffix or different name (`-req` precedent).
- Traps: a wrapped continuation line must never open with `+`/`-`/`*`
  (5 strikes this session, counter caught each); bash heredocs break on
  big python scripts — Write the script to the scratchpad and run it.

## State

- Branch `main`; **GitHub synced** through `ff4ca088` + this
  checkpoint's commits; working tree clean (untracked `.zcode/` only).
- **GitVerse SSH down** (banner-exchange timeout all session). Verified
  via HTTPS `ls-remote`: strict ancestor, no foreign commits — plain
  `cargo xtask mirror` re-fan when the link recovers; NEVER `--force`.
- `progress check` = 12 expected errors (design/README);
  `progress scan`: 59 files, 1 280/4 894 unmarked.
- Ledger: 29 findings (F-023…F-029 new this day — the stale-header
  family + the dangling PROP-043 launcher ref; one Phase C/D sweep
  fixes all seven). Opus/DRIFT queue EMPTY.
- No `/goal` active (was set, then cleared by the owner).

## Standing decisions in force (unchanged)

Fact-exhaustive granularity (PROP-043 §3.9); anchored-when-marked;
two registers; scope = **59 files** (include-only enumeration in
`progress.toml`; terraforms/research/neworder OUT per the 2026-07-25
ruling; WAL + generated boot pair OUT; authored boot pair IN,
additive-only); verdicts in cache/baseline never in markup; markers
record the DOC's claims; **no fractality for this campaign** (Fable =
markup/verification/review/task-authoring, Opus = DRIFT coding);
coder-tier engine pin `claude-opus-5`; campaign zone excluded; the
surface is "dashboard"; the four CLAUDE.md rules bind every executor.

## Recent commits (newest first)

```
ff4ca088 docs(spec): B2 — PROP-030 marked at fact grain (late commit)
e9d330f8 docs(spec): B2 batch 19 — PROP-005 marked at fact grain
8901cd05 chore(progress): drop terraforms/research/neworder from scope (owner ruling)
122ba06b docs(wal): session-end checkpoint — B2 at 32/35, three giants remain
55b48d1b docs(continue): cold-resume checkpoint — B2 modules sweep, 32/35
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
```

## Repository map (top level)

- `spec/` — the corpus: `common/` DONE; `modules/` 33/35 (PROP-003,
  PROP-002 remain); `design/` 6 files remain; `boot/` authored pair
  remains; `manual-tests/` 3 remain; `discipline/README` remains;
  `terraforms/` `research/` `neworder/` OUT of scope; `WAL.md` out of
  scan scope.
- `crates/` — `progress-core` (fact scanner), `vibe-spec` (fact-aware
  compiler), `vibe-cli` (`commands/progress.rs`), the rest of vibe.
- `campaigns/progress-2026-08/` — `tasks/` (DRIFT-001…005 done),
  `run/` (journal, state/*.json — findings 29 rows, RESUME, cache).
- `tools/progress-dashboard/serve.mjs`; `tools/self-check.sh`.
- `.claude/agents/opus5.md` — the coder-tier agent type.

## Quick start

```bash
bash tools/self-check.sh                                  # the floor (REAL exit code)
cargo run -q -p vibe-cli --bin vibe -- progress check     # markup gate (12 expected design/README errors)
cargo run -q -p vibe-cli --bin vibe -- progress scan      # refresh state
cargo run -q -p vibe-cli --bin vibe -- progress resume    # regenerate RESUME.md
node tools/progress-dashboard/serve.mjs                   # dashboard
cargo xtask mirror                                        # fan-out (gitverse fails while SSH is down — expected)
```
