# fractality — cold-resume checkpoint

_Written 2026-07-12 mid-session (the five-task goal push). `WAL.md` (same
directory) is the canonical living state and supersedes this snapshot on
divergence. Resume with `восстанови сессию fractality` (report-then-wait)._

## TL;DR

Executing the owner's five-task goal — **1) clean stuck branches · 2) a
validated Stage C · 3) PP-004 follow-ups · 4) PP-001 · 5) PP-002** —
autonomously with compaction, delegating grunt to GLM (CC+z.ai). A Stop-hook
holds the session on this goal until all five are genuinely done.

**Done: tasks 1, 3, 5.** **Task 4 is FIRING now** (PP-001, background).
**Task 2 has its machinery + pre-registration; its help/hurt trial build +
fire + the C-3 doc remain.** ~24 commits, all on `main`, pushed to both
remotes (gitverse canonical + github mirror) at the latest. Floor green
throughout (test-gate 223). Two paid trials fired this session (the PP-004
gated re-run + PP-001 in flight); a big new evidence-preservation convention
landed at the owner's direction.

## Where each task stands

- **Task 1 — stuck branches: DONE.** 11 `fractality/<ULID>` trial-worktree
  branches + 5 gigabyte temp trees removed; `git branch` = `main` only.
- **Task 2 — validated Stage C: machinery + pre-reg IN; trial + C-3 REMAIN.**
  - `fractality advise` verb (`a1a9403`) — V4 advisor channel CLI surface.
  - Advisor ladder as routing data (`0740bc3`) — `ClassPolicy.advisor_class`
    + `advisor_class_for`.
  - MT-C3-02 help/hurt trial **pre-registered** (`91cc156`) — paired arm
    (ALONE vs ADVISED), one caller tier (glm-5.2 advises glm-5-turbo; the
    weak-caller falsifier arm needs a 3rd tier, deferred).
  - **REMAINING:** build `run-advise.sh` + the uncertain-task `menu-advise.md`
    (4 tasks with hidden acceptances over `trial/staging`, designed below) +
    2 preambles + `score-advise.py`; fire alone×3 + advised×3; save + score.
    Then **C-3** the uncertainty-trigger doc (thresholds measured from the
    trial).
- **Task 3 — PP-004: DONE.** Caps↑ (`d601eb2`), `decisions` verb (`28b47b3`),
  menu tasks 9/10 + MT-C3-03 (`7623a05`), arm g2 + scorer (`dbdb030`, GLM-built
  $0.97). Gated re-run **fired + scored** (`23ab679`): P-C3-a CONFIRMED as a
  hard count (80% route/inline), P-C3-b SUPPORTED (boss set output_schema),
  P-C3-d CONFIRMED (Silo task → 2 escalate verdicts). Pool 38.1% (run 3 a
  technical failure). Evidence in `reports/trial-results/2026-12-07-11-03-…`.
- **Task 4 — PP-001: FIRING NOW.** RP5 resolved (`d28f2c4`, 3+3 GLM cold boss
  cap 8). Arm a/b ×3 firing in the background (`fire-pp001.log`), auto-saves
  to a `c2-mt-c2-05-initiative-rerun` group at the end. **On resume: check
  the fire completed, then score by MT-C2-01 rules + rule PR1–PR3 into
  MT-C2-05 "Recorded runs" + fill the group README + commit.**
- **Task 5 — PP-002: DONE.** The credibility query (`core::worker_credibility`
  → `CredibilityFact`, `ae8544f`) + the surface (`c85d032`): the cold board
  (SessionStart hook) + `fractality scoreboard` show "workers self-verify
  here: acceptance N/N green, last proven <age> (profile X)" when a real
  completed-green acceptance backs it (D7). Answers the Ф6 F24 keep-reason.

## Active work in flight

**PP-001 firing** (background task; `scratchpad/fire-pp001.log`): 6 GLM cold
boss runs (~25 min each, ~2.5 h total). Cannot run `floor`/`cargo` while it
fires — Windows locks the running `.exe`s. When it prints `ALL-PP001-DONE`
the evidence is auto-preserved under `reports/trial-results/<dated>-c2-mt-c2-05-initiative-rerun/`;
score with the MT-C2-01 rubric + the fatigue facts (nudges by reason, the
slate at each nudge) and rule PR1–PR3.

## Next-steps recipe (cold start)

1. **Collect PP-001** (if the fire finished): read `fire-pp001.log` RESULT
   lines; the group is auto-saved; score per MT-C2-01, fill MT-C2-05 §Recorded
   runs + the group README, commit. (If it did not finish, `bash
   spec/manual-tests/trial/run-arm.sh {a,b} {n}` the missing runs.)
2. **Build + fire the MT-C3-02 help/hurt trial** (task 2). Author, over
   `trial/staging` (mini_logfmt):
   - `menu-advise.md` — 4 genuinely-uncertain tasks with hidden acceptances:
     (A) `first_unique` preserving FIRST-SEEN order (a HashSet reorders →
     fails); (B) `count_records` where a trailing newline is NOT a record;
     (C) a `merge_pairs(&a,&b)->Record` that composes into a fold (by-value
     dead-ends); (D) `render` round-tripping an empty-value pair `k=`.
   - `preamble-alone.md` / `preamble-advised.md` (advised tells the caller to
     `fractality advise` a rung up before committing).
   - `run-advise.sh` (mirror `run-arm.sh`, caller = `small` model, preamble
     swap, collect the `advise` runs) + `score-advise.py`.
   - Fire `alone {1,2,3}` + `advised {1,2,3}`; save (`advise-help-hurt`
     group) + score (PR-adv-1 help, PR-adv-2 mechanism, PR-adv-3 no-hurt).
   - Delegatable to GLM (CC+z.ai) — a discipline-light content build.
3. **C-3** — the uncertainty-trigger doc (spec), thresholds from the trial.

## Non-obvious findings this session

- **`target/trial-results/` is GITIGNORED** — paid-run evidence must be saved
  to `reports/trial-results/` or a `cargo clean` wastes the money. Now a
  binding rule (workspace CLAUDE.md §"Preserve valuable test/run evidence"):
  after every trial fire (and any important/long run, by judgment) run
  `save-results.sh <group-description>` + fill the scaffolded group README +
  commit. Layout: **dated groups of dated runs** (год-число-месяц-время), a
  README at every level with its own meaning (not on pure replicates).
- **specmap tracks cell LINE SPANS** — editing ANY scoped `.rs` file shifts
  spans and drifts specmap even with no cell added/removed; re-mint in-commit
  (`rust-ai-native specmap`). MT docs + trial assets are NOT indexed.
- **The `glm` profile has only 2 tiers** (glm-5.2 big, glm-5-turbo small) — the
  help/hurt trial's two-point RD-10 inversion needs three; it fires the one
  point two tiers serve (5.2 advises 5-turbo), the rest deferred.
- **`cargo fmt` is safe** (rustfmt handles UTF-8) — the PS-5.1 corruption
  quirk is only PowerShell's Get/Set-Content, not rustfmt.
- **Mirror non-ff can be transient** — gitverse rejected a push as "non-ff"
  while merely being behind (an ancestor of local); `git merge-base
  --is-ancestor origin/main main` confirmed ff-safe and a plain `git push
  origin main` synced it. Never `--force`.

## Repository map (workspace)

`packages/org.vibevm.fractality/` — `CLAUDE.md` (contract, now with the
evidence rule), `WAL.md`, this file, `WORKSPACES.md` row, `VIBEVM-BACKLOG.md`;
**`plans/`** (postponed.md + PP-001/002/003/004); **`reports/`** (per-phase
narratives + **`trial-results/`** — the committed paid-run evidence, dated
groups). `fractality/v0.1.0/` — the Cargo workspace: `crates/{core,
mission-control, pod, mc-client, backend-claude-code, cli, initiative}`;
`spec/` (PROP-001, VISION, plans/**RLM** + **ADVISOR**, manual-tests/**MT-C3-01/02/03**
+ **MT-C2-05** + the trial harness `run-arm.sh`/`save-results.sh`/`score-g2.py`).
New this session: core `credibility.rs`, cli `advise.rs`.

## Decisions / policy in force

- Host Rules 1–4; plan §10 executor guide BINDING; clean-room §10.4; the
  delegation law + live-observation; no Python in shipped code (trial
  runners/scorers are test tooling); commit heredoc; editor-tool edits (PS
  5.1); specmap re-mint on ANY scoped-file change; F15; domain code no
  unwrap/expect; 600-line conform cell budget.
- **All paid trial arms PRE-AUTHORIZED this goal** (owner: «Авторизую все
  платные прогоны и автономию до конца текущего goal»). RP5 = 3+3 GLM cold
  boss cap 8.
- **Preserve valuable test evidence — ALWAYS** (the new convention).
- Never `floor`/`cargo` while a trial fires (Windows `.exe` lock).

## Recent commit chain (newest first)

```
c85d032 feat(fractality): surface worker credibility on the cold board (PP-002 done)
ae8544f feat(fractality): the worker-credibility query (PP-002 core)
23744f3 docs(fractality): mark PP-004 done in the registry
23ab679 test(fractality): fire + score the PP-004 gated re-run (MT-C3-03)
9932b9c test(fractality): date + group the trial-results, one README per group
552b559 test(fractality): preserve paid trial evidence durably + make it a rule
dbdb030 test(fractality): the PP-004 gated re-run runner + scorer (arm g2)
7623a05 docs(fractality): arm the PP-004 gated re-run — MT-C3-03 + the two menu tasks
d28f2c4 docs(fractality): resolve MT-C2-05 RP5 — the initiative re-run is armed
91cc156 docs(fractality): pre-register MT-C3-02 — the advisor help/hurt trial
d601eb2 feat(fractality): raise the turn caps for trial completion (PP-004 item 1)
28b47b3 feat(fractality): the `fractality decisions` read verb (PP-004 item 4)
0740bc3 feat(fractality): Stage C — the advisor ladder as routing data (V4)
a1a9403 feat(fractality): Stage C — the `fractality advise` verb (D-C3-7)
```

## Quick-start

```sh
cd packages/org.vibevm.fractality && head -40 WAL.md
cd fractality/v0.1.0
# floor (ALWAYS from v0.1.0; NEVER while a trial fires):
/c/Users/olegc/gits/vibevm/packages/org.vibevm/rust-ai-native-lang/v0.7.0/target/debug/rust-ai-native.exe floor
# delegate to GLM (CC+z.ai): claude -p '<task>' --model glm-5.2[1m] at the z.ai gateway
```

Resume phrase: `восстанови сессию fractality` (report-then-wait).
