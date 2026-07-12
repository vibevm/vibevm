# fractality — WAL (project continuation state)

_Updated: 2026-07-12 (mid-session — the big Stage C + PP-004 push).
Campaign 3 Stage B is COMPLETE (all Ф0–Ф7 closed, prior WAL); this session
executes the owner's five-task goal: **1) clean stuck branches ✓ 2) a
validated Stage C 3) PP-004 follow-ups 4) PP-001 5) PP-002**, autonomous
with compaction, delegating grunt to GLM (CC+z.ai). Substantial machinery +
all three trials armed have landed + pushed; the PP-004 gated re-run is
FIRING now. The canonical living state; supersedes CONTINUE.md on divergence._

## Where the goal stands (this session)

- **Task 1 — stuck branches: DONE.** 11 `fractality/<ULID>` trial-worktree
  branches + 5 gigabyte temp trees removed; `git branch` = `main` only.
- **Task 2 — validated Stage C: machinery IN, trial armed.**
  - `fractality advise` verb (`a1a9403`) — the V4 advisor channel's CLI
    surface (marks `output.advice`, sync-runs like `run`, MC applies the
    RD-10 caller-class bar). New `advise.rs` cell; `run_once` → `pub(crate)`.
  - Advisor ladder as routing data (`0740bc3`) — `ClassPolicy.advisor_class`
    (medium→strong, strong→strong, weak→none) + `advisor_class_for`, guidance
    gated by `advisor_enabled`.
  - **MT-C3-02 help/hurt trial PRE-REGISTERED** (`91cc156`) — the paired-arm
    (ALONE vs ADVISED) design, honestly scoped to the ONE point two GLM tiers
    can serve (glm-5.2 advising glm-5-turbo); the weak-caller falsifier arm
    needs a 3rd tier, deferred. **Build (run-advise.sh + uncertain menu +
    scorer) + fire = the remaining Stage C step.**
  - Deferred: C-3 uncertainty-trigger doc (wants measured thresholds from the
    help/hurt trial).
- **Task 3 — PP-004: machinery IN, trial FIRING.**
  - Turn caps raised (`d601eb2`) — packet default `max_turns` 40→80, runner
    boss `--max-turns` 50→100 (item 1).
  - `fractality decisions` read verb (`28b47b3`) — the need-gate journal
    readable, P-C3-a as a hard count (item 4).
  - Menu tasks 9 (schema→P-C3-b) + 10 (Silo→P-C3-d) + MT-C3-03 pre-reg
    (`7623a05`); runner arm `g2` + `score-g2.py` (`dbdb030`, **delegated to
    GLM glm-5.2, $0.97, reviewed clean**).
  - **MT-C3-03 gated re-run FIRING now** (arm g2 ×3, background) — save +
    score + verdicts pending its completion.
- **Task 4 — PP-001: ARMED.** RP5 resolved (`d28f2c4`) — owner ruling «3+3
  GLM cold boss cap 8» recorded verbatim in MT-C2-05 §RP5. Reuses the arm-a/b
  runner. Fire + score pending.
- **Task 5 — PP-002: NOT STARTED.** DEF-C2-2b-full worker credibility on the
  boss surface (acceptance-schema plumbing). Note: `RunRecord.collected`
  already carries `acceptance_passed/total` — the missing half is
  aggregating a credibility fact + surfacing it. Design questions in
  `plans/postponed/PP-002-…`.

## Evidence-preservation convention (owner directive, 2026-07-12 — NEW, binding)

The owner caught that paid trial results were written only to the gitignored
`target/trial-results/` — one `cargo clean` from wasted. Now a **standing,
non-optional rule** (workspace `CLAUDE.md` §"Preserve valuable test/run
evidence"):

- After every trial fire (and, by judgment, any important/long run whose
  results carry value): `bash spec/manual-tests/trial/save-results.sh
  <group-description>`, fill the scaffolded group README, commit.
- Layout: **dated groups of dated runs** — `<год-число-месяц>-<HH-MM>-<name>`
  (report convention), each group dir carrying a `README.md` (what the test
  was + summary results, amended when analysis lands). `proj-final/` excluded
  (huge + reproducible); transcripts gzipped.
- The nine prior paid runs (C2 arm-a/b + C3 arm-g) are preserved + restructured
  (`552b559`, `9932b9c`); rule + broadening (`ea6e0b7`). A run is not "done"
  until its evidence is committed.

## Next — finish the firing, then the deferred trials + PP-002

Reading order to resume: workspace `CLAUDE.md` → this WAL → `CONTINUE.md` →
the MT pre-regs (MT-C3-02, MT-C3-03, MT-C2-05) → `plans/postponed/PP-002/004`.

1. **Collect the FIRING MT-C3-03 gated re-run** — when arm g2 ×3 finishes:
   `save-results.sh c3-mt-c3-03-gated-rerun`, fill the group README,
   `python spec/manual-tests/trial/score-g2.py`, record verdicts in MT-C3-03
   "Recorded runs" (P-C3-a as decision counts, P-C3-b schema gate, P-C3-d
   escalation), commit.
2. **Build + fire the MT-C3-02 help/hurt trial** — `run-advise.sh` (caller =
   small model + advise/alone preamble swap), the uncertain-task menu with
   hidden acceptances over `trial/staging`, `score-advise.py`; fire alone×3 +
   advised×3; save (`advise-help-hurt` group) + score + verdicts.
3. **Fire the PP-001 MT-C2-05 re-run** — `run-arm.sh {a,b} {1,2,3}` (RP5
   resolved), save (`c2-mt-c2-05-rerun` group) + score by MT-C2-01 rules +
   rule PR1–PR3.
4. **PP-002 plumbing** — acceptance-backed credibility on the boss surface.
5. **C-3 uncertainty-trigger doc** — with thresholds measured from the
   help/hurt trial.

Each code slice = one commit, floor green after each (backgrounded cargo — but
NEVER during a live trial fire: Windows locks the running `.exe`s). Trials
fire in the background; save-results is mandatory after each.

## Constraints (do not violate without discussion)

- Host Rules 1–4; **plan §10 executor guide BINDING**; clean-room §10.4;
  delegation law + live-observation; no Python in shipped code (trial
  runner/scorer are legitimate test tooling); commit heredoc; editor-tool
  edits (PS 5.1 UTF-8); specmap re-mint in-commit on scoped-file change (MT
  docs + trial assets are NOT specmap-indexed — verified); domain code no
  `unwrap`/`expect` (conform); 600-line conform cell budget; F15 (stop MC
  before builds).
- **All paid trial arms PRE-AUTHORIZED this goal** (owner 2026-07-12:
  «Авторизую все платные прогоны и автономию до конца текущего goal»). RP5 for
  MT-C2-05 resolved (3+3 GLM cold boss cap 8).
- **Preserve valuable test evidence — ALWAYS** (see the convention section).

## Delegation scoreboard (session)

- **Delegated & succeeded:** the PP-004 arm-g2 runner + `score-g2.py` build →
  GLM glm-5.2 via CC+z.ai ($0.97). Precise discipline-light work order; GLM
  self-verified (`bash -n`, `ast.parse`), correctly declined to recreate an
  existing file. Reviewed clean (arm concat, decisions-collection line,
  verdict-key match vs the kebab-case serde). The model for run-and-report +
  mechanical trial-asset builds.
- **Kept boss-side (legitimately):** all seam/experiment design — the advise
  verb + advisor ladder (V4 seam), the three MT pre-registrations (spec
  authoring), the uncertain-task design, the evidence-preservation convention
  (owner-facing surface); trial FIRING via backgrounded harness (token-cheap,
  the notification is the "don't babysit") + review of the delegated diff.
- **Mechanism:** CC+z.ai GLM (`claude -p --model glm-5.2[1m]` at the z.ai
  Anthropic gateway) — the proven delegation path; also what the trial arms
  run on. Never echo the token (`$(cat …)` → env var).
