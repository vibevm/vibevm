# fractality — WAL (project continuation state)

_Updated: 2026-07-11 ~18:20 (**Campaign 3 Stage B EXECUTION underway —
70%-context checkpoint**). One session: RP-C3-1 ruled (Option B),
Ф0 spikes CLOSED (all seams green), Ф1.1 packet `context_from`
LANDED floor-green. Paused at the 70%-context boundary per the
owner's standing rule; next session resumes at Ф1.2. Prior:
2026-07-11 earlier (two research stages closed + Stage B draft +
executor guide)._

## Current state

- **Stage B COMMISSIONED — Option B (descent + ascent).** RP-C3-1
  ruled 2026-07-11 (owner: «Вариант 1. Вариант плана - B … Вариант
  C с адвайзором - отдельная задача, запланируй»). Recorded in plan
  §1, §8. Advisor (Option C / V4) postponed → **PP-003**.
- **Goal (owner, standing):** «довести кампанию 3 … рабочий RLM со
  всеми паттернами … сделай это всё» — the WHOLE Stage B plan
  (Ф0→Ф7). **70%-context rule:** at ~70% consumed, checkpoint and
  ask for a restart (this checkpoint is exactly that).
- **Ф0 spikes CLOSED** (no commits; report
  `reports/2026-11-07-18-12-campaign3-f0-spikes.md`). s1
  schema-validate ran green: **jsonschema 0.47.0 compiles on rustc
  1.93.1**, validates, violation shape `at <JSON-Pointer>:
  <message>` (API: `validator_for` → `is_valid`/`iter_errors` +
  `err.instance_path()`). s2 (FileRef slice), s3 (settings-injection
  promotion), s4 (escalated-outcome) green by inspection/design — all
  compose from existing machinery.
- **Ф1.1 LANDED** (`35a378c`): `ContextSpec.context_from: Vec<RunId>`
  — the D-C3-2 access-list (isolation-by-default; only named results
  cross; fold law). `#[serde(default)]`, schema stays 1, golden
  snapshot + specmap re-minted in-commit.
- **Floor green:** 165 tests / conform 0 / specmap **170 units / 63
  items / 63 edges / 0 orphans**. Real `~/.fractality` untouched.
- **Live tracker:** `reports/2026-11-07-17-52-rlmplan-state-plan.md`
  (goal, 70%-rule, seam reconnaissance, slice plan, delegation
  scoreboard). Started dashboard:
  `reports/2026-11-07-16-56-rlmplan-started-plan.md`. Paused
  dashboard added this checkpoint.
- Session commit chain: `c3039c0` commission → `3b71d9d` PP-003 →
  `0255c9d` started-dashboard → `db1e0d1` state-plan → `c1151bb` Ф0
  close → `35a378c` Ф1.1 → checkpoint commits.

## Next (resume here, Ф1.2 first)

Reading order for the resuming session: workspace `CLAUDE.md` → this
WAL → `CONTINUE.md` → the paused-plan dashboard
(`…-rlmplan-paused-plan.md`) → the live state-plan tracker (has the
seam reconnaissance so you need not re-read the crates) → plan §10
(BINDING). Then continue the slice plan:

1. **Ф1.2** `OutputSpec.output_schema` (raw JSON string in dep-light
   core) + validation at the pod/collect seam (add jsonschema 0.47.0
   to `fractality-pod`, `validator_for`/`iter_errors`) + **one
   retry-on-violation** (pod re-invokes the worker once with the
   violation report appended — scope carefully; the re-invoke touches
   pod/supervise, not yet read).
2. **Ф1.3** `BudgetSpec` six axes + wall-clock (RD-4): depth /
   per_agent_calls / per_call_token_ceiling / cumulative_tokens /
   currency / global_calls (0 = unlimited convention holds).
3. **Ф1.4** D-C3-3 boundary behaviors per verb (MC + profiles).
4. Then Ф2 (need-gate + delegation-rules own workspace) … Ф7.
   Task list #2–#8 mirrors the phases.

Each slice = one commit, floor green after each (specmap re-mint
in-commit on line-number drift). **Floor/test runs = backgrounded
cargo** (opencode unreliable today — see below).

## Constraints (do not violate without discussion)

- Host Rules 1–4; **plan §10 executor guide is BINDING**; clean-room
  §10.4 (never open refs/src|papers|articles while coding); delegation
  law + live-observation (first-output ≤3 min); I1–I7; no Python in
  shipped code; cwd law; commit heredoc; editor-tool edits; specmap
  re-mint law; scratch homes; no `*install*` test binaries; F15 (stop
  MC before builds).
- **Ф6 paid trial arms — RP-C3-2 PRE-AUTHORIZED 2026-07-11** (owner:
  «я прямо сейчас разрешаю делать эти платные прогоны»). Arms fire
  after MT-C3-01 pre-registration is committed (§10.7 still binds the
  pre-reg-first order); budget posture confirmed at Ф6. MT-C2-05 stays
  RP5-gated (unruled) — MT-C3-01 is this campaign's first trial.
- Fugu benchmark numbers are Sakana-reported — mechanism evidence
  only.

## Delegation scoreboard (session)

Delegated 1 (attempted): s1 schema spike → opencode/GLM glm-5.2.
**FAILED twice** — external_directory reject on a nested cargo
project, then a silent launch stall (7 min, 0 artifacts). Killed per
the live-observation law; s1 done boss-side. **Field data (Phase-5):**
opencode is unreliable for cargo spikes on this box today; use
in-place `cargo init --vcs none .` if retried, and prefer backgrounded
cargo for floor/test runs (reliable notification path). Kept
(boss): all seam reconnaissance + design + every doc (architecture /
plan authorship — never-delegate) — appropriate, since Ф1+ code is
discipline-bound seam work.
