# CONTINUE — cold-resume checkpoint

_Written 2026-07-25 (session end: Phase B CLOSED, corpus fully marked;
Phase L handed off to the next session by the owner). `spec/WAL.md` is
the canonical living state and supersedes this snapshot wherever they
diverge._

## TL;DR

Progress-Control campaign (PROP-043, plan
`spec/terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.md`): **Phase B is
CLOSED with the exit gate green in full** — 58 files, **4 880/4 880
facts marked**, `progress check --exhaustive` clean (0 errors), floor
`self-check` all green (real exit 0), boundary entry in the plan LOG §9
(`a1bb2111`), GitHub synced. Three owner rulings landed this session:
**(1) Phase L (legacy relocation)** inserted between B and C —
`spec/terraforms` `spec/research` `spec/neworder` `spec/discipline`
leave the spec tree for root **`legacy-spec/`** after their inbound
references are inventoried and the referenced facts are ported into the
living corpus; **(2) spec/discipline dropped from the markup corpus**
(the Discipline lives in the ai-native packages; scope 59 → 58);
**(3) the campaign plan itself STAYS in `spec/terraforms/`** («Я
передумал. План этой кампании пока переносить не нужно…») — the §6
monthly recurrence needs the plan + instruction set in place, so **L5
excludes the plan file** from the move. Phase L execution was handed
off wholesale to the next session («Перенеси все эти активности в
следующую сессию»). Ledger: **35 findings** (F-030…F-034 new).
Opus/DRIFT queue EMPTY. Journal fully closed. GitVerse SSH still down
(clean ancestor; plain re-fan on recovery, never `--force`).

## How to start the next session (exact)

Открой сессию фразой вида:

> Продолжаем спец-актуализационную кампанию (PROP-043). Прочитай
> CONTINUE.md, затем campaigns/progress-2026-08/run/RESUME.md и хвост
> LOG §9 плана. **Открывай Phase L (legacy relocation)** и работай
> L1→L5 по секции Phase L плана. Разметка и перенос фактов — Fable,
> без fractality; механический bulk — субагентом opus5 только если
> расчёт delegation-rules это оправдает.

Then the executor does, in order:

1. Boot per `CLAUDE.md`; read this file, `run/RESUME.md`, the plan §5
   **Phase L** section + LOG §9 tail (the three 2026-07-25 rulings).
2. **Open the phase in the journal** (the owner's phrase above is the
   authorization): append
   `{"kind":"phase","value":"L","ts":"<UTC>"}` to
   `campaigns/progress-2026-08/run/journal.jsonl` (DRIFT-003 lane:
   append-only, last wins), then a `step-start` for `l1-inventory`.
3. **L1 — reference inventory.** Every reference into the four legacy
   dirs from (a) the living corpus (`spec/common` `spec/design`
   `spec/modules` `spec/manual-tests` `spec/boot`), (b) crates
   (specmark scopes, `#[spec(...)]`, `spec://` URIs, doctests), (c)
   root docs (`CLAUDE.md`/`AGENTS.md`/`GEMINI.md`, `README`,
   `ROADMAP.md`, `VIBEVM-SPEC.md` — the last is owner-frozen: report,
   don't edit). Cross-refs *between* the four dirs don't count.
   Starter greps (run from repo root; refine as needed):

   ```bash
   rg -n "(\.\./)+(terraforms|research|neworder|discipline)/" spec/common spec/design spec/modules spec/manual-tests spec/boot --glob '*.md'
   rg -n "spec://vibevm/(terraforms|research|neworder|discipline)/" spec crates --glob '!spec/terraforms/**' --glob '!spec/research/**' --glob '!spec/neworder/**' --glob '!spec/discipline/**'
   rg -n "spec/(terraforms|research|neworder|discipline)/" crates --type rust
   rg -n "(terraforms|research|neworder|discipline)" CLAUDE.md AGENTS.md GEMINI.md README.md ROADMAP.md 2>/dev/null
   ```

   Write the inventory into the journal step-done (counts per referrer)
   and keep the raw list in the scratchpad; it drives L2–L5.
4. **L2 — fact discovery:** per reference, identify the fact(s) cited
   at the target (often a §-anchor or a whole doc; the fact grain of
   the target text decides).
5. **L3 — relocation:** port those facts into
   `common/design/modules/manual-tests`. Creating new specs is allowed
   (owner grant 2026-07-25: «придется создать какие-то новые
   спецификации - создавай, это не проблема»). Genre law applies:
   normative → PROP/contract; lore → design.
6. **L4 — markup:** every ported fact gets `##anchor` + marker at its
   new home (conventions below); new files enter `progress.toml`
   (include-only enumeration — add explicit paths/globs); referrers
   repoint to the new anchors. `check` must stay 0.
7. **L5 — relocation:** when the L1 greps return zero live inbound
   refs, `git mv spec/terraforms spec/research spec/neworder
   spec/discipline` → `legacy-spec/` — **EXCLUDING
   `spec/terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.md`** (owner
   ruling: the active plan stays; move the rest of terraforms around
   it). Campaign-zone pointers that named moved files update in the
   same commit.
8. **Phase L exit gate:** the L1 greps re-run clean; `progress check
   --exhaustive` green over the (grown) scope; `bash
   tools/self-check.sh` REAL exit 0 (specmap included — repointed
   scopes must resolve); the §4 boundary entry in the plan LOG; plan
   status line refreshed. **Ask the owner before opening Phase C.**
9. Journal step per unit of work; batch commits ~1–3 items; `vibe
   progress resume` before each commit; `cargo xtask mirror` at
   checkpoints with
   `GIT_SSH_COMMAND="ssh -o ConnectTimeout=15 -o BatchMode=yes"`
   (gitverse fails while its SSH is down — expected; github must go
   `ok`).

## The per-file loop (proven, do not improvise)

journal `step-start` (`{"kind":"step-start","id":"<slug>","step_type":"…","target":"<real path>","actor":"fable","ts":"<UTC>"}`
— verify the filename BEFORE writing the entry) → Read the file → write
python replace-scripts into the scratchpad (`rep(old,new)` with
`assert a in s` + `assert s.count(a)==1`; write back via
`open(p,'w',encoding='utf-8',newline='\n')`; patterns must copy the
file's REAL line wrapping — a chunk aborts atomically before write on
assert failure) → run → `progress check` (stays 0) + `progress scan` →
corpus row **0 unmarked / 0 issues** → journal `step-done` with counts
AFTER the scan → commit (campaign `run/` included).

## Marking conventions (established; follow verbatim)

- Registers: `##UPPER-SLUG` = normative fact, `##kebab` =
  narrative/service; design docs use the `@doc/*` stage.
- Preamble splits at bold-key boundaries (`##milestone-line` /
  `##status-line` / `##related` / …); bare `spec://` line → `##self-uri`.
- `` `req rN` `` lines → own unit (`##<slug>-req` when the heading
  already owns the `req-*` name — the collision-dodge suffix);
  `` `design rN` `` → `##<slug>-design`.
- Decision-paragraph idiom: first sentence stays the lead unit;
  remaining facts deconstruct into items (sense- and
  wording-preserving; semantic edits FORBIDDEN in markup passes).
- Tables: every non-empty body cell marked; `##ROW-*` id in the first
  cell; em-dash placeholder cells count as non-empty.
- Checkbox lists: anchor precedes `[x]`/`[ ]`; a genuinely pending
  box (unsigned sign-off) is `@impl/work`.
- Blockquotes cannot carry anchors (F-015): re-form as a plain
  anchored paragraph **verbatim**.
- Manual-tests genre: step items and their indented **Expected**
  paragraphs are separate units (`##STEP-N-*` / `##EXP-N-*`); a
  post-fence continuation inside a step is its own unit.
- A continuation paragraph attached to a list item makes the item's
  mid-text marker invalid — deconstruct the continuation into anchored
  sub-items (the counter catches this exactly).
- User-owned files (`spec/boot/00-core.md`, `90-user.md`): ADDITIVE
  ONLY — anchors + markers inserted in place; never re-form.
- Stage/state records the DOC's claims: impl'd content `@impl/done`,
  drafted spec `@spec/done`, open questions `@spec/work`, unexecuted
  phase plans `@impl/plan`, superseded arcs `@spec/done` vs
  authoritative `@impl/done`. Stale headers → LEDGER (F-024 family),
  never reworded. Verdicts live in cache/baseline, never in markup.
- Heading/fact ids share one namespace per doc — dodge collisions with
  a suffix or a different name.

## State

- Branch `main`; **GitHub synced** through `dc39cf1d` + this
  checkpoint's commits; working tree clean (untracked `.zcode/` only).
- **GitVerse SSH down** (banner-exchange timeout, all of 2026-07-25).
  Strict ancestor verified via HTTPS `ls-remote` — plain
  `cargo xtask mirror` re-fan when the link recovers; NEVER `--force`.
- `progress check` = **0 errors** (and must stay 0);
  `progress scan`: **58 files, 4 880 facts, 0 unmarked**.
- Ledger: **35 findings** (`run/state/findings.json`): the
  F-024…F-034 stale-header family + internal-drift rows — Phase C/D
  material, one sweep fixes the family. Opus/DRIFT queue EMPTY.
- Journal fully closed; the phase lane reads **B** until the next
  session appends the `L` phase event on the owner's phrase.

## Standing decisions in force (unchanged unless noted)

Fact-exhaustive granularity (PROP-043 §3.9); anchored-when-marked; two
registers; scope = **58 files** (include-only in `progress.toml`;
terraforms/research/neworder OUT 2026-07-25 a.m., discipline OUT
2026-07-25 p.m.; WAL + generated boot pair OUT; authored boot pair IN,
additive-only); **Phase order B → L → C** (owner amendment 2026-07-25;
verification must cover the ported facts); **the campaign plan stays in
`spec/terraforms/`** (owner reversal 2026-07-25 — monthly recurrence
per plan §6 consumes it in place; L5 excludes it); new specs may be
created during L3 (owner grant); **no fractality for this campaign**
(Fable = markup/porting/verification/review, Opus = DRIFT coding);
coder-tier engine pin `claude-opus-5`; campaign zone excluded from
scan; the four CLAUDE.md rules bind every executor.

## Recent commits (newest first)

```
dc39cf1d docs(wal): Phase B closed — checkpoint before the owner's Phase L call
a1bb2111 docs(spec): Phase B boundary — exit gate green, corpus fully marked
7d9dd964 docs(spec): B2 batch 26 — manual-tests close; the corpus is fully marked
ae10dca2 docs(spec): B2 batch 25 — authored boot pair marked additively
1c48019a chore(progress): drop spec/discipline from scope (owner ruling)
91fde06c docs(spec): B2 batch 24 — design cluster closes 6/6
d1a09275 docs(spec): B2 batch 23 — two design records marked at fact grain
cb6e55b0 docs(spec): B2 batch 22 — design/README re-mark burns the last errors
9328becb docs(spec): B2 batch 21 — PROP-002 marked; modules cluster closes 35/35
c0147947 docs(spec): record owner amendment — Phase L legacy relocation
d596c631 docs(spec): B2 batch 20 — PROP-003 marked at fact grain
f486433f docs(wal): session-end checkpoint — scope 59 files, Phase B closing
```

## Repository map (top level)

- `spec/` — the corpus, FULLY MARKED: `common/` + `modules/` (35) +
  `design/` (6) + `manual-tests/` (3) + authored `boot/` pair;
  `terraforms/` `research/` `neworder/` `discipline/` are OUT of scan
  scope and queued for Phase L relocation to `legacy-spec/` (the
  campaign plan file stays); `WAL.md` out of scan scope.
- `crates/` — `progress-core` (fact scanner), `vibe-spec` (fact-aware
  compiler), `vibe-cli` (`commands/progress.rs`), the rest of vibe.
- `campaigns/progress-2026-08/` — `tasks/` (DRIFT-001…005 done),
  `run/` (journal, state/*.json — findings 35 rows, RESUME, cache).
- `tools/progress-dashboard/serve.mjs`; `tools/self-check.sh`.
- `.claude/agents/opus5.md` — the coder-tier agent type.

## Quick start

```bash
bash tools/self-check.sh                                  # the floor (REAL exit code)
cargo run -q -p vibe-cli --bin vibe -- progress check     # markup gate (must be 0)
cargo run -q -p vibe-cli --bin vibe -- progress scan      # refresh state
cargo run -q -p vibe-cli --bin vibe -- progress resume    # regenerate RESUME.md
node tools/progress-dashboard/serve.mjs                   # dashboard
cargo xtask mirror                                        # fan-out (gitverse fails while SSH is down — expected)
```
