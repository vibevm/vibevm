# FRACTALITY-INITIATIVE-PLAN v0.1 — Campaign 2: the initiative system (scoreboard-driven delegation for a cold boss)

_Status: **CLOSED 2026-07-10 (all seven phases executed; both trial
arms run and scored; predictions ruled — P1 confirmed, P3 falsified
with channel analysis; owner sign-off on the MT index pending)**
(commissioned 2026-07-10: «Начинай Campaign 2» + «Goal set: сделать
Campaign 2»; resumed and closed under the owner goal «campaign 2
должен быть завершен», 2026-07-10) · written
2026-07-10 against host tree `a7695ab` (clean, mirrors synced) ·
Format: `flow:org.vibevm/campaign-plans` (one file, five roles) ·
cold-executable: any phase boundary is a safe stop; the floor is green at
every boundary. Lineage: drains DEF-1 (+ two named IGNITION leftovers)
from [`FRACTALITY-IGNITION-PLAN-v0.1.md`](FRACTALITY-IGNITION-PLAN-v0.1.md) §15._

## 2. Execution record — (prepended at close)

**Commit range:** `47412ad` (campaign open) → the close commits,
19 fractality-scoped commits total (15 planned; see P7). Executed
across two sessions on 2026-07-10: Ф0–Ф5 + the Ф6 pre-registration
in the first; the paid arms, scoring, and Ф7 in the second (resumed
under the owner goal «campaign 2 должен быть завершен»).

**Per-phase deltas vs plan:** Ф1/Ф3/Ф4 each folded their planned
multi-commit shape into one feat commit (compile-coupled cells;
ledgered per phase, never absorbed). Ф2 gained one discovered split
(`mc_cmd.rs`, the conform 600-line budget). Ф5 deliberately dropped
the `answer --rule` config-writing flag (hand-editing profiles.toml
is the honest v1 surface; ledgered). Ф6 ran with a GLM-served cold
boss per RP1 (3+3 runs, cap 8 untouched — zero technical repeats);
the runner, staging, menu, and scoring were committed and frozen
BEFORE any run. Ф7 closed same-day.

**The trial's headline numbers (frozen scoring, MT-C2-01/-04):**
Arm A (snippet only) pooled **3/18 ≈ 16.7%** (runs: 1/6 · 2/6 · 0/6);
Arm B (+ initiative hooks) pooled **0/18 = 0%** (0/6 × 3). Distractor
delegations: 0 in both arms. Boss work quality stayed high in both
arms (6/6 attempted everywhere; artifacts complete in clean-finish
runs).

**Verdict on every prediction:**

- **P1 CONFIRMED** — 16.7% < 50%: the cold-delegation gap is real.
- **P2 CONFIRMED** (Ф0, 4/4 live seam facts on CC 2.1.202).
- **P3 FALSIFIED** on both clauses (0% < 80%; 0 < A+30). Channel
  analysis (the falsifier's obligation): the SessionStart scoreboard
  DELIVERED but showed an empty board (fresh home — zero social
  proof); the UserPromptSubmit nudge is STRUCTURALLY SILENT in
  single-prompt `-p` sessions (the only prompt arrives at slate=0;
  thresholds crossed at 34/44/45 with no later prompt to inject
  into); Stop push correctly silent (nothing parked). Shared
  confound: the scratch env breaks MSVC auto-detect, so bosses in
  BOTH arms hand-fixed the linker and cited "workers can't
  self-verify" as a rational keep reason. Iteration belongs to a
  follow-up campaign (see §15).
- **P4 CONFIRMED** — hook bench MC-warm, n=20: post-tool-use
  P95 = 51 ms (< 100); user-prompt-submit P95 = 49 ms (< 200).
  RP2's recommendation (counter default-ON) stands on data.
- **P5 CONFIRMED** (Ф4, matrix goldens 10/10).
- **P6 CONFIRMED** — statusline P95 = 58 ms (< 150), MC warm.
- **P7 CONFIRMED** — 19 commits ≤ 26, every fold/split ledgered.
- **P8 CONFIRMED** — close-time grep audit: the initiative crate has
  zero fs writes; the only initiative-path writes are the sanctioned
  `CLAUDE_ENV_FILE` append (hook.rs `export_session_env`) and the
  owned settings entries at install time. I3 survived its first
  consumer.

**What the campaign actually proved:** the fabric's mechanics work
end to end under a cold boss (hooks fire, sessions attribute,
counters count, workers spawn/complete when chosen), and the
scoreboard-first posture is honest — but *visibility alone does not
move cold propensity in headless single-prompt sessions*. The two
strongest observed levers on delegation were (a) the boss's belief
that workers can self-verify (broken sandbox ⇒ no delegation), and
(b) prompt seams that actually fire mid-work. Both are named,
actionable deltas in §15.

## 3. The mandate (owner, verbatim)

Commission (2026-07-10): «Начинай Campaign 2» · «Goal set: сделать
Campaign 2».

Seed, from the IGNITION mandate (2026-07-09):

> «Но меня очень беспокоит, что если мы не сделаем какую-то систему
> инициативы для Opus, он просто не будет использовать спавн наших
> агентов. В принципе, чем меньше заточки на Claude Code тем лучше […]
> будет Opus внутри других агентов (включая наш собственный будущий
> агент VibeVM Pixel который будет работать специально для Opus)»

> «5. Делаем систему, которая вынуждает модель чаще и более креативно
> использовать агентов. […] Clean Room Implementation [barkain workflow
> orchestration] но это дискуссионное. Конечно же на Rust […]»

The scoreboard addendum (owner, 2026-07-09, verbatim):

> «идея про "scoreboard вместо принуждения": mission control может
> сохранять метаданные профилировки и ориентироваться по ним, и
> отдавать эти метаданные всем, кому нужно.»

The interim enforcement this campaign automates (workspace contract,
recorded 2026-07-09):

> "Enforcement until fractality automates it (this is Campaign 2's
> scoreboard, run by hand today): every session-end WAL checkpoint
> records *delegated: N tasks (what) / kept: why*."

Also binding: the owner directive to consult the codex-first lessons
(2026-07-10: «не забывай про уроки, извлеченные из codex-first») — DC1
(delegate when verification is cheaper than generation), DC5
(acceptance proves), the metering loop ("did delegation pay" is an
answer, not a claim), and I4 harness-neutral framing all carry into
this design.

### 3b. What "initiative system" means here

A boss session should *reach for the fabric on its own*: see its wins,
notice its parked workers, and get a well-timed, fact-citing nudge when
it starts doing grunt work itself — without coercion, without a script
zoo, and without any accounting outside mission-control. DEF-1's shape,
quoted: "agent-neutral policy core fed by MC metrics (I3); per-harness
adapters at the edges (Claude Code hooks first — hook target is a
fractality subcommand, never a script zoo); scoreboard-first (visible
wins), contextual injection by threshold, P6 baseline to beat."

## 4. Target arithmetic

**Baseline (verified 2026-07-10, this box):**

- 6 crates / 3 binaries / **11 public CLI verbs** (`mc ps show run spawn
  wait tree kill questions answer stats`) + hidden `mcp-broker`.
- Metrics dimensions: totals, by_state, by_profile, by_model, by_day
  (`metrics.rs`); **no session concept anywhere in core** (grep-verified).
- Claude Code hooks installed on this box: **0**. Boss surface: boot
  snippet 75 v1 + `fractality-delegate` skill v1 (static prose, no live
  data).
- Delegation matrix: markdown policy only (no machine-readable form).
- Manual-test index: MT-01…05 recorded and signed off.
- Floor: green — conform 0 findings (6/6 gated), specmap 16 units / 47
  items / 0 orphans, ~120 tests, clippy -D warnings.
- P6 (IGNITION): warm-session delegation = 100% (2/2) — the tooling
  works; the cold-session propensity is UNMEASURED.

**Exit state:**

- 7 crates (+`fractality-initiative`) / 3 binaries / **18 public verbs**
  (+`session`, `scoreboard`, `route`, `hook`, `statusline`, `harness`,
  `fetch`).
- MC: sessions registry + session events + per-session aggregates;
  journal event kinds extended additively; `stats` gains the monthly
  quota rollup (IGNITION §15 leftover, absorbed).
- This box: **one owned hook-entry set live** in the workspace's
  `.claude/settings.local.json` (installed by `fractality harness
  install claude-code`, removable, `disableAllHooks`-respecting).
- `matrix.toml` in delegation-rules + `route` verb; golden test pins
  TOML ↔ the matrix's 10 worked verdicts.
- Boot snippet 75 v2 + skill v2 (live-loop aware).
- MT index +4: MT-C2-01 (cold baseline), MT-C2-02 (hooks live),
  MT-C2-03 (nudge + question push), MT-C2-04 (initiative-on trial).
- Both trial arms measured and recorded; floor green; ≤ 26 commits with
  drift ledgered.

Every baseline unit is either unchanged (binaries, MT-01…05) or mapped
to a phase below; nothing else moves.

## 5. Current-state facts (verified 2026-07-10; do not re-discover)

- Host tree `a7695ab`, clean, `main` == `origin/main`. `claude` CLI
  **2.1.202**; MC daemon stopped; the three binaries build in
  `target/debug/`.
- **Claude Code hook facts** (from the local `refs/src/cc-docs/`
  snapshot, 2026-07-09, CC 2.1.202; extraction:
  `refs/src/cc-docs/EXTRACT-cc-hooks-initiative.md`, boss-spot-checked):
  - `SessionStart` (matchers `startup|resume|clear|compact`):
    `hookSpecificOutput.additionalContext` is inserted before the first
    prompt; plain stdout also becomes context; `initialUserMessage`
    creates the first turn in `-p` mode (hooks.md:949) — the trial
    harness lever. Hook env carries **`CLAUDE_ENV_FILE`**
    (hooks.md:979): `export` lines written there persist into every
    later Bash command — the session-attribution lever.
  - `UserPromptSubmit`: additionalContext injected alongside the prompt;
    **default timeout 30 s** (output discarded on timeout — never block
    the prompt path).
  - `PostToolUse`: fires after every tool call; additionalContext
    possible; cannot block. `PostToolBatch` fires once per parallel
    batch, additionalContext possible.
  - `Stop`: `hookSpecificOutput.additionalContext` is
    **continue-feedback — it prevents the stop and keeps the turn
    going** (use ONLY for deliberate interrupts); `stop_hook_active`
    guards loops; CC caps consecutive stop-blocks at 8.
  - `Notification` types include `permission_prompt` and `idle_prompt`
    (`notification_type` field); no decision control.
  - Common stdin fields: `session_id`, `transcript_path`, `cwd`,
    `hook_event_name`, `permission_mode`, `prompt_id`, `effort`.
  - Output strings (additionalContext etc.) capped at **10 000 chars**
    (hooks.md:733).
  - Hooks live in `~/.claude/settings.json` / `.claude/settings.json` /
    `.claude/settings.local.json` (precedence: managed > CLI --settings
    > local > project > user); matching hooks run in parallel;
    `disableAllHooks` kills hooks AND custom statusline;
    `CLAUDE_PROJECT_DIR` is exported to hook processes.
  - Statusline: `statusLine` settings key `{type: "command", command,
    padding?, refreshInterval?}` (settings.md:303); **the exact stdin
    JSON contract is NOT in the local snapshot** (`/en/statusline` page
    was not captured) — Ф0.s2 closes this.
  - `-p` mode: hooks fire (unless `--bare`); `permissionDecision:
    "defer"` is `-p`-only; `--append-system-prompt`, `--session-id`,
    `--max-turns`, `--max-budget-usd` available — the trial harness
    toolbox.
- **Drift found while writing:** the glm-5.2 playbook and D12 reference
  `fractality fetch` — **the verb does not exist in the built binary**
  (IGNITION cut it silently). Repaired in Ф3 (c8); until then documents
  are downloaded with plain curl and recorded.
- **Worker env law (I1) interaction:** worker envs are whitelist-built;
  `FRACTALITY_RUN_ID` parents worker-side spawns (per `spawn --help`).
  `FRACTALITY_BOSS_SESSION` must NOT enter the worker whitelist —
  Ф1 extends the I1 env test to pin this.
- **opencode delegate law (measured today, twice):** non-interactive
  `opencode run` auto-rejects reads outside the launch cwd
  (`external_directory`) — delegate inputs must be copied under the
  scratch cwd; heartbeats must be `echo` commands. Recorded in the
  workspace contract.
- barkain studied → [`../refs/notes/barkain-study.md`](../refs/notes/barkain-study.md)
  (BD1–BD6 keeps; named non-adoptions). codex-first lessons in
  [`../refs/notes/codex-first-study.md`](../refs/notes/codex-first-study.md)
  (DC1–DC6).
- Boot-slot grid: 75 (fractality) and 77 (delegation-rules) taken; no
  new slot needed — snippet 75 v2 absorbs the initiative surface.

## 6. Decisions

### D1 — the loop: observe → score → surface; scoreboard-first, never coercive
MC accumulates the truth (I3); a pure engine renders and decides; thin
adapters surface at session seams. Default posture is **visible wins +
threshold-gated contextual nudges**; nothing blocks, ever (owner's
scoreboard-instead-of-coercion; barkain BD1/BD5 non-adoption of forced
continuation). Rejected: PreToolUse ask/deny gating of grunt work
(coercion; annoys precisely the sessions it should win over); prompt-only
exhortation (IGNITION already ships it — snippet 75 — and the unmeasured
cold gap is the whole point).

### D2 — boss-session attribution: CLAUDE_ENV_FILE export, fallbacks named
`fractality hook session-start` registers the session with MC and writes
`export FRACTALITY_BOSS_SESSION=<ulid>` to `CLAUDE_ENV_FILE`; every
subsequent `fractality` CLI invocation in that session reads the var and
stamps `origin_session` onto runs it creates. Idempotent per CC
`session_id` (resume/clear re-begin returns the same open session).
- (α) cwd-keyed "active session" lookup in MC — ambiguous under
  concurrent sessions in one directory; kept only as the *fallback*
  when the env var is absent (spawn records `origin: cwd-inferred`).
- (β) explicit `--session` flag everywhere — friction the boss will not
  pay; rejected as the primary path (the flag exists for scripts).
- (γ) `CLAUDE_ENV_FILE` export (**CHOSEN**) — mechanism documented
  (hooks.md:979), zero-friction, exact.
Workers never see `FRACTALITY_BOSS_SESSION` (I1 whitelist; tested) —
worker-side spawns keep attributing via `FRACTALITY_RUN_ID` parenting.

### D3 — MC additions: sessions are facts, not policy
New MC surface (additive): `POST /v0/sessions` (begin, idempotent by
`{harness, external_id}`) · `POST /v0/sessions/:id/events` (work_tool,
nudge_sent, question_alert, ended…) · `GET /v0/sessions/:id` ·
`GET /v0/sessions/:id/metrics` (session-scoped MetricsBucket +
initiative counters) · `GET /v0/sessions?open=true`. Journal event kinds
extend additively (replay-compatible). MC stores facts and counters
ONLY; all policy (thresholds, cooldowns, texts) lives in the engine —
MC stays lean, policy iterates without daemon redeploys, I3 holds (one
store, no shadow accounting; nudge/cooldown state is session events,
never files).
Rejected: policy evaluation inside MC (couples daemon releases to
prompt-tuning); local state files à la barkain `.claude/state/` (racy,
unowned — BD non-adoption).

### D4 — the CC adapter: one binary, owned settings entries
Everything Claude-Code-specific is `fractality-cli` verbs:
`fractality hook <session-start|user-prompt-submit|post-tool-use|stop|
session-end>` (stdin: CC hook JSON → stdout: CC hook JSON),
`fractality statusline`, `fractality harness install|status|remove
claude-code`. Hook config is written as entries whose `command` string
is the fractality binary path + verb — **the command string IS the
ownership marker** (the managed-blocks law adapted to JSON: we create/
update/remove exactly the entries we recognize as ours by deterministic
scan, and never touch a byte of anyone else's config; malformed JSON →
hard stop, precise report, no auto-repair).
- Write target default: **`.claude/settings.local.json`** (machine-
  scoped, gitignored — initiative posture is per-developer, like
  `~/.fractality/profiles.toml`); `--project` opt-in writes
  `.claude/settings.json` for teams that want it committed (RP3).
- **Availability law:** MC down / not installed / anything unexpected →
  every hook verb exits 0 with empty output and a debug-log line. A
  broken initiative system must never break a boss session.
Rejected: shipping a CC plugin (marketplace/version coupling, second
distribution channel); interpreter scripts (the script zoo, banned by
DEF-1's own wording); writing `~/.claude/settings.json` (user-global
blast radius).

### D5 — injection policy: which seam says what
- **SessionStart** (`startup|resume|clear|compact`): compact scoreboard
  (≤ ~6 lines): delegated/completed counts + last-7-days line, parked
  questions with ages, quota line, one pointer at the matrix/skill.
  Live data only — the law stays in snippet 75.
- **UserPromptSubmit**: threshold-gated, ≤ 2 lines, at most one nudge
  per cooldown window; fires only when (a) unacknowledged parked
  questions exist, or (b) the session's work-tool counter since the
  last delegation ≥ threshold (default 7), or (c) quota alarm. Cites
  facts + the verb to run (`fractality questions` / the skill). 30 s
  timeout budget means: one MC round-trip, no retries.
- **PostToolUse** (matcher `Bash|Edit|Write|MultiEdit|NotebookEdit`):
  counter event to MC, **no output** (BD1: reads never count; the
  counter zeroes on a fractality spawn in the session). Latency budget
  P4 gates this hook's default-on status (RP2).
- **Stop**: normally silent accounting (turn end). Emits
  continue-feedback ONLY for an unacknowledged parked question
  (once per question — MC ack event dedupes; respects
  `stop_hook_active`). This is the question-push teeth (DEF-1 cargo),
  bounded exactly as barkain's forced-continuation is not.
- **SessionEnd**: close the session record; write nothing.
- **Statusline**: one ambient line — `frl: N deleg · M done · K parked
  · quota X%` (+ `mc: down` dim state). Refresh via `refreshInterval`.
Rejected: PostToolBatch injection in v1 (a second mid-turn channel
before nudge fatigue is understood); Notification hook (fires on CC's
own notifications, not external events — useless for MC pushes);
SessionStart `initialUserMessage` outside the trial harness.

### D6 — nudge/scoreboard content is data; routing becomes data
_(rewritten in place mid-Ф4, 2026-07-10 — the original said the
machine-readable matrix lives in the delegation-rules package; that
splits one fact across two artifacts that WILL drift. Single-source
wins.)_
Texts are templates in the engine filled from MC facts; thresholds and
cooldowns in `~/.fractality/initiative.toml` (machine-scoped, like
profiles; env `FRACTALITY_INITIATIVE=off` is the kill switch; sample:
`spec/examples/initiative.sample.toml`). The delegation matrix's
axes/verdict procedure gains a machine-readable form that ships
**inside the engine crate** (`fractality-initiative/src/matrix.toml`,
embedded via include_str!, with a Rust constant table as its parsed
form and a drift tripwire between the two); the delegation-rules MD
stays the normative prose and CITES the executable form. `fractality
route --error-cost … --context … --verify … --size …` prints verdict +
slot + scenario (`--json` for machines). **The engine's tests pin the
calculus to the MD's worked-verdicts table** — drift fails the floor.
Rejected: NL task classification in the engine (an LLM call per prompt
— cost, latency, and a second brain to argue with; the boss IS the
classifier, the engine hands it the procedure); a package-shipped TOML
+ an embedded copy (two sources, one truth — the exact drift disease
this campaign exists to kill).

### D7 — the scoreboard is strictly factual
v1 renders only measured facts: delegated runs (session/day/week),
outcomes, tokens+cost by slot, wall time, parked questions with ages,
web-quota burn (monthly rollup lands here — IGNITION leftover),
unreviewed-collected count (BD6), and a streak line. **No invented
"saved X boss-tokens" numbers** — economics claims need a measurement
basis we do not have yet; a dishonest scoreboard would poison exactly
the trust the initiative system runs on. (Savings estimation, if ever,
is a later campaign with its own methodology.)

### D8 — crate layout: engine crate + verb edges
New crate `fractality-initiative`: pure policy/render engine (scoreboard
formatting, nudge selection, threshold/cooldown evaluation, route
calculus, statusline line) — unit-tested, no I/O beyond types. CLI verbs
do I/O (MC client, stdin/stdout JSON); MC gains the session cells.
Rejected: a separate package (DEF-4: split at API stability, not
before); logic inside the CLI crate (untestable); logic inside MC (D3).

### D9 — the trial protocol: measure cold, then measure again
One fixed **task menu** (6 eligible grunt tasks + 2 distractor
judgment tasks, drawn from real backlog shapes: templated tests, a
bounded sweep, doc extraction, fixture generation, a module draft with
acceptance, a mechanical refactor); one scripted cold-session harness
(`claude -p` with `--session-id`, `initialUserMessage` carrying the
menu, hooks per arm, fixed budget caps). Arm A: snippet 75 v1 + skill,
NO initiative (the honest cold baseline IGNITION could not measure).
Arm B: same everything + initiative installed. Metric per arm:
delegated-eligible / eligible (MC attribution decides "delegated";
transcript decides "attempted"). Pre-registered success criteria = P1/P3
below, recorded in MT-C2-01/-04 BEFORE the runs; agent pre-runs, owner
signs off (manual-tests law). Paid-run count and timing: **RP1**.
Rejected: measuring in this authoring session (delegation-primed — the
exact contamination IGNITION's P6 caveat named); a synthetic toy repo
with fake tasks (external validity ≈ 0).

## 7. Predictions (checked one by one at close)

- **P1** — the cold baseline (Arm A) delegates **< 50%** of eligible
  tasks: the owner's fear is real and measurable. (Falsifier: ≥ 50% —
  then snippet 75 alone already carries propensity and the initiative
  system's value is re-argued from the B−A delta.)
- **P2** — all four seam facts hold live on CC 2.1.202: SessionStart
  additionalContext reaches the model; UserPromptSubmit
  additionalContext reaches the model; `CLAUDE_ENV_FILE` exports
  persist into later Bash calls; the statusline command receives JSON
  carrying at least `session_id`. (Falsifier: any one fails → the
  affected decision D2/D5 is rewritten in place before Ф1 opens.)
- **P3** — Arm B (initiative on) delegates **≥ 80%** of eligible tasks
  AND **≥ Arm A + 30 points**. (Falsifier: either miss → the campaign
  report analyses which channel failed; thresholds/texts iterate in a
  follow-up, honestly recorded.)
- **P4** — hook overhead on this box: `post-tool-use` P95 **< 100 ms**,
  `user-prompt-submit` P95 **< 200 ms**, MC warm. (Falsifier:
  post-tool-use over budget → the counter hook ships default-OFF and
  prompt-time MC inference substitutes — fallback named in R5/RP2.)
- **P5** — `matrix.toml` + `route` reproduce the matrix's worked table
  **10/10** (golden). (Falsifier: any mismatch is a bug in exactly one
  of the two — conflict protocol decides which.)
- **P6** — the statusline line renders **< 150 ms** warm (MC up).
- **P7** — the campaign lands in **≤ 26 commits** (15 planned; the
  IGNITION P7 lesson priced in), drift ledgered at each boundary.
- **P8** — zero shadow state: the initiative path writes nothing
  outside MC API calls + `CLAUDE_ENV_FILE` (+ the owned settings
  entries at install time). Audited by grep at close; I3 survives its
  first consumer.

## 8. Phases

**Ф0 — spikes and probes (no commits).**
Steps: **s1** hook live-probe pack on this box (a scratch project +
tiny `-p` runs on the cheapest model: SessionStart/UserPromptSubmit
additionalContext visibility, CLAUDE_ENV_FILE persistence,
per-event stdin capture, `--include-hook-events` observation; measure
exe-spawn + hook overhead for P4) · **s2** capture `/en/statusline`
into `refs/src/cc-docs/statusline.md` (plain download this once — the
`fetch` verb lands in Ф3) and extract the stdin/refresh facts ·
**s3** author the trial menu + harness script skeleton (no paid arm
runs) · **s4** attribution seam checks (worker-env whitelist audit for
`FRACTALITY_BOSS_SESSION`; concurrent-session env behavior) · **s5**
settings-entry ownership probe (install/update/remove idempotence on a
scratch `.claude/settings.local.json`; `disableAllHooks` interplay).
Exit: probe results tabled in §5-appendix; every red spike rewrites its
decision in place before Ф1. Gate: host floor untouched (no tree
changes).

**Ф1 — sessions + attribution.** Core DTOs (SessionRecord, session
events), MC registry/journal/API cells, CLI `session begin|end|show
|ls` (thin), run stamping (`origin_session` via env, cwd fallback), I1
test extension (worker env never carries the var). Planned commits:
`feat(fractality): sessions — MC registry, journal, API` ·
`feat(fractality): session attribution — env stamp + session verbs`.
Exit: floor green; a hand-driven session begin → spawn → metrics shows
attribution end to end (recorded in the ledger).

**Ф2 — scoreboard engine + verbs.** `fractality-initiative` crate
(render + policy calculus, pure); `scoreboard` verb (`--session|
--global`, `--json|--line`); per-session metrics endpoint consumption;
`stats` monthly quota rollup (absorbs the IGNITION leftover). Planned:
`feat(fractality): initiative engine — scoreboard + session metrics` ·
`feat(fractality): scoreboard verb + stats month rollup`.
Exit: floor green; scoreboard renders real IGNITION-era journal data.

**Ф3 — the Claude Code adapter.** `hook <event>` verbs (stdin/stdout
JSON, availability law, worker exemption), `statusline` verb, `harness
install|status|remove claude-code` (owned entries, RP3 default), and
`fetch <url> --out <path>` (D12 repair; rustls feature joins here as
D11 anticipated). Golden tests for hook I/O; MT-C2-02 authored and
pre-run (hooks live in a scratch project). Planned:
`feat(fractality): cc hook adapter — hook verbs + availability law` ·
`feat(fractality): harness install — owned settings entries` ·
`feat(fractality): statusline verb` · `feat(fractality): fetch — local
document intake (D12)`.
Exit: floor green; hooks installed on a scratch project inject a live
scoreboard at SessionStart (recorded).

**Ф4 — nudges + routing-as-data + question push.** Work-tool counter
events; threshold/cooldown engine wired to `user-prompt-submit`;
`matrix.toml` + `route` verb + 10/10 goldens (delegation-rules package
edit rides this commit); Stop-hook parked-question continue-feedback
with MC ack dedupe. MT-C2-03 authored and pre-run. Planned:
`feat(fractality): routing-as-data — matrix.toml + route verb` ·
`feat(fractality): nudge policy — thresholds, cooldowns, injection` ·
`feat(fractality): question push — stop alerts with acks`.
Exit: floor green; a staged scratch session shows: counter crosses
threshold → next prompt carries the nudge; parked question interrupts
one stop exactly once.

**Ф5 — answer-rules slice (D18 layer 2, thin).** Profile-level
auto-answer patterns for permission escalations (`fractality answer
--rule` / rules in profiles.toml), evaluated by MC at park time; hits
auto-resume and are journaled as `auto_answered`; misses park as today.
Planned: `feat(fractality): answer rules — profile auto-answer
patterns`. Exit: floor green; a staged escalation matching a rule
resumes without a question; a non-matching one parks.

**Ф6 — the trial + boss surface v2.** RP1-gated paid arms per D9;
record both rates; snippet 75 v2 + skill v2 (live-loop aware: scoreboard
citation, question triage, route verb); MT-C2-01/-04 recorded; reports.
Planned: `docs(fractality): MT-C2 procedures + trial records` ·
`feat(fractality): boss surface v2 — snippet 75 + skill update`.
Exit: floor green; both arms recorded; owner sign-off on the MT index.

**Ф7 — close.** §2 execution record, §14 ledgers, reports/, WAL,
CONTINUE, WORKSPACES row, backlog entries. Planned:
`docs(fractality): campaign 2 close — ledgers, reports, WAL`.

## 9. Risks and fallbacks

- **R1 — CC hook schema drift across versions.** Detection: probe
  results pinned to 2.1.202; `harness status` prints the CC version it
  last verified. Fallback: adapter tolerates unknown fields, exits 0 on
  parse surprises (availability law); a version bump re-runs Ф0.s1.
- **R2 — MC down at hook time.** By design, constant: hooks exit 0
  silently; statusline shows `mc: down`. The boss session is never
  hostage to the fabric.
- **R3 — nudge fatigue / prompt pollution.** Thresholds + cooldown +
  2-line cap + kill switch; the trial's Arm B measures acceptance, not
  just delegation rate (nudges shown vs acted-on recorded in MC).
- **R4 — trial validity (small N, contamination).** Pre-registered
  criteria (P1/P3) in the MT files BEFORE runs; fixed menu; the
  authoring session never runs an arm; owner rules N (RP1). Worst case:
  the numbers are honest and small — recorded as such.
- **R5 — per-tool-call exe spawn too slow on Windows.** Measured in
  Ф0.s1 (P4). Fallback: post-tool-use hook ships default-OFF; the
  counter derives at prompt time from MC run/tool history instead
  (coarser, zero per-call cost).
- **R6 — concurrent boss sessions, one cwd.** Env attribution wins;
  cwd fallback only when env is absent, marked `cwd-inferred` in the
  record (never silently trusted for the trial numbers).
- **R7 — settings file malformed / hand-edited around our entries.**
  Deterministic entry scan; on anything unrecognizable: hard stop,
  precise report, no auto-repair (managed-blocks law).

## 10. Non-goals (named, with disposition)

- **No coercive mode** — no PreToolUse deny/ask gating of grunt work.
  Rejected outright (owner's scoreboard stance); revisit only on owner
  word.
- **No output-compression / Bash-rewrite layer** (barkain 2.4) — token
  hygiene, not initiative; brittle. Rejected for this campaign; a
  future flow may study it on its own mandate.
- **No worker-side initiative** — I5 stands; workers stay
  uninstrumented.
- **No savings estimator** on the scoreboard (D7) — deferred until a
  measurement methodology exists; candidate future campaign.
- **No GUI / web dashboard** — the scoreboard is CLI/statusline; GUIs
  read the same MC API later (I3 guarantees they can).
- **No federation / cross-box sessions** — single box, as IGNITION.
- **No full dynamic permission brokering** — only the Ф5 rules slice;
  learned/suggested policies deferred (needs field data the slice will
  generate).
- **No RLM** — Campaign 3 (DEF-2).
- **No publish** — RP3 (host) stays owner-gated.

## 11. Quick-start for the executing session

```sh
cd packages/org.vibevm.fractality/fractality/v0.1.0
git log --oneline -3        # expect this plan's commit on top of a7695ab+
# the floor (cwd law — run FROM this directory):
/c/Users/olegc/gits/vibevm/packages/org.vibevm/rust-ai-native-lang/v0.7.0/target/debug/rust-ai-native.exe floor
# baseline arithmetic:
./target/debug/fractality.exe --help | grep -c '^  [a-z]'   # 11 public verbs (+help) at baseline
grep -ri "session" crates/fractality-core/src/ | wc -l       # 0 at baseline
```

## 12. Whole-campaign acceptance

```sh
cd packages/org.vibevm.fractality/fractality/v0.1.0
<floor command> && echo FLOOR-GREEN                     # exit 0
./target/debug/fractality.exe scoreboard --json | head  # renders from MC
./target/debug/fractality.exe route --error-cost reversible \
  --context compilable --verify mechanical --size S     # → delegate/small
./target/debug/fractality.exe harness status claude-code # reports owned entries
grep -rn "FRACTALITY_BOSS_SESSION" crates/fractality-pod/src/worker_env.rs  # whitelist-excluded (test cites it)
# MT index: MT-C2-01..04 recorded; both trial arms carry numbers.
```

## 13. Review points

- **RP1 — trial authorization (RESOLVED 2026-07-10, owner verbatim:
  «авторизую тебя делать платные армы через GLM, подбери не супер
  большое но достаточное количество ранов»).** The paid arms run with
  a **GLM-5.2-served Claude Code as the cold boss** (worker-style
  clean env + the z.ai gateway — flat-rate; the Max subscription is
  not burned on the experiment). Validity caveat, recorded: a GLM
  boss is a proxy for the real Opus-class boss — the A↔B delta on
  identical arms is the trustworthy number; absolute rates compare to
  P6's warm-real-boss 100% only loosely. Count, per the mandate's
  "sufficient, not huge": **3 runs per arm (6 total), technical-repeat
  cap 8** — 18 eligible-task decisions per arm over the 6-task menu.
  D9's harness mechanics adapt accordingly (the boss process is
  hand-spawned with the I1-style clean env, fractality on PATH, its
  own scratch home; arm B additionally gets `harness install` +
  FRACTALITY_HOME exported into the session env).
- **RP2 — post-tool-use counter default (OPEN).** Recommend ON with the
  5-tool matcher + kill switch, GATED on P4's measurement (< 100 ms
  P95); if P4 falsifies, ships OFF with the prompt-time fallback.
- **RP3 — settings write target default (OPEN).** Recommend
  `.claude/settings.local.json` (machine-scoped, no repo pollution);
  `--project` flag for committed team config. Owner may flip the
  default.
- **RP4 — execution mode (RESOLVED 2026-07-10, owner verbatim: «Goal
  set: сделать Campaign 2»).** The campaign executes in-session;
  phases land autonomously per host Rule 4; RP1–RP3 still stop for the
  owner.

## 14. Execution ledger

_Filled at each phase boundary: commit map (hash → planned subject),
what each commit confirmed or falsified, drift notes._

- **Ф0 — EXECUTED 2026-07-10, all spikes green, no commits (as
  planned).**
  - **s1 (hook live-probe, CC 2.1.202, haiku `-p`, scratch project):
    P2 CONFIRMED 4/4** — SessionStart `additionalContext` reached the
    model; UserPromptSubmit `additionalContext` reached the model;
    `CLAUDE_ENV_FILE` export persisted into a later Bash call
    (`PROBE_ENV=fractality-probe-42` echoed); hooks loaded from
    `.claude/settings.local.json` in `-p` (s5 co-proven). Captured
    stdin schemas: SessionStart `{cwd, hook_event_name, session_id,
    source, transcript_path}`; UserPromptSubmit adds `{prompt,
    prompt_id, permission_mode}`; PostToolUse adds `{tool_name,
    tool_input, tool_response, tool_use_id, duration_ms}` (**F21:
    `duration_ms` is served on a plate** — the work-tool counter can
    weigh events by duration, not just count); Stop adds
    `{stop_hook_active, last_assistant_message, background_tasks,
    session_crons}`. **F20: the statusline command does NOT run in
    `-p` mode** (no capture despite configuration) — statusline is an
    interactive-session surface; the trial arms won't see it, the live
    boss will. Probe turn: 11.4 s wall, 2 turns, $0.026.
  - **s1-latency (P4 proxy): warm `fractality.exe` spawn ≈ 6 ms**
    (5.7–12 ms over 5 runs) — the per-tool-call hook budget (<100 ms)
    holds with an order of magnitude to spare before the engine + one
    localhost RTT are added (**F22**).
  - **s2 (statusline contract):** `/en/statusline` captured to
    `refs/src/cc-docs/statusline.md` (plain curl this once — the
    `fetch` verb lands Ф3). Facts: stdin JSON carries
    `model.display_name`, `workspace.current_dir/project_dir`,
    `cost.total_cost_usd`, `context_window.used_percentage` (+ sizes),
    `session_id`, `exceeds_200k_tokens`; updates are event-driven and
    go quiet when idle — `refreshInterval` (min 1 s) re-runs on a
    timer, exactly what parked-question ages need; ANSI colors
    supported; `padding` exists. D5's statusline design stands.
  - **s3 (trial skeleton):** menu shape (E1–E6 eligible + D1–D2
    distractors), staging, run mechanics, and pre-registered scoring
    drafted (session scratch; binding MT texts land in Ф6 as planned).
  - **s4 (attribution seam):** `worker_env.rs` audit — the worker env
    is clean-slate + two deliberate injections (`FRACTALITY_HOME`,
    PATH head); `FRACTALITY_BOSS_SESSION` cannot leak by construction;
    the Ф1 test pins its absence. Concurrent-session behavior:
    env-file export is per-session by construction
    (`~/.claude/session-env/<session-id>/…` observed), so R6's env
    path is per-session-correct.
  - **Verdict:** no decision rewrites needed; Ф1 opens.

- **Ф1 — EXECUTED 2026-07-10. Commit map:**
  - `1c9757b` — `feat(fractality): sessions — the attribution spine
    (C2 Ф1)`. **Drift vs plan: one feat commit instead of the two
    planned** — the field addition, the MC subsystem, and the CLI
    attribution surface compile only together; splitting them would
    have manufactured a non-building intermediate commit for
    ceremony's sake. Ledgered, not absorbed (P7 bookkeeping).
  - Confirmed: D2 (env-var attribution composes — `session begin`
    prints one id; the I1 pin holds: the var never enters a worker
    env), D3 (facts-only counters fold deterministically; the sibling
    sessions.jsonl replays with the run journal untouched — restart
    test green), the BD1 slate-clean rule (a Delegated note zeroes
    `work_tools_since_delegation`, history survives).
  - Floor at the boundary: **all green** — 118 tests (7 new session
    integration tests, delegated to the big slot and verified by the
    boss), conform 0 findings (6/6 gated), specmap 18 units / 52
    items / 52 edges / 0 orphans, clippy `-D warnings`, test-gate
    xfail-strict. Live smoke through the built binary on a scratch
    home (F16): begin → ls → show → end → mc stop.
  - Session delegation scoreboard so far: delegated 3 (barkain
    survey; cc-docs extraction; the Ф1 session-test suite — all
    GLM-5.2, all delivered first landing), kept with cause: seam
    design, spec/plan prose, the http.rs budget split, reviews.
  - Next: Ф2 (scoreboard engine + verbs).

- **Ф2 — EXECUTED 2026-07-10. Commit map:**
  - `6f5788a` — `feat(fractality): initiative engine — scoreboard
    render (C2 Ф2)` (planned subject #1: the seventh crate, pure and
    clock-free; render_line / render_board / month_web_calls; gated
    in conform from birth).
  - `6d8397e` — `feat(fractality): scoreboard verb + stats month
    rollup (C2 Ф2)` (planned subject #2 + a discovered split: main.rs
    crossed the 600-line budget with the new verb surface and the
    conform gate caught it — the mc verb family moved to its own
    `mc_cmd.rs` cell; ledgered as in-phase discovered work, not
    silent).
  - Confirmed: D7 (the board renders only measured facts — the test
    pins exact strings), the engine/shell separation (D8: five unit
    tests, zero I/O in the crate), the quota-rollup absorption
    (IGNITION §15 leftover closed).
  - Floor at the boundary: **all green** — conform 0 findings (7/7
    gated), specmap 18 units / 55 items / 55 edges / 0 orphans,
    test-gate xfail-strict (123 tests). Live smoke: `scoreboard`,
    `scoreboard --line`, and the `stats` month line against a scratch
    home with an env-attributed session.
  - Next: Ф3 (the Claude Code adapter).

- **Ф3 — EXECUTED 2026-07-10. Commit map:**
  - `4e2c71c` — `feat(fractality): cc adapter — hooks, statusline,
    harness, fetch (C2 Ф3)`. **Drift: one feat commit instead of the
    four planned** (one wiring file interleaves every verb; the Ф1
    precedent applies) — ledgered.
  - Confirmed: D4 (availability law — hooks exit 0 on every failure;
    connect-only, never an autostart), D5's SessionStart shape (live
    board injected + CLAUDE_ENV_FILE export written), BD2 (the
    statusline strip), the JSON-adapted managed-blocks ownership
    (foreign entries and a foreign statusLine survive install AND
    remove — unit-pinned), the D12 repair (`fetch` exists; reqwest's
    0.13 TLS feature rename caught exactly as D11 anticipated).
  - The conform gate earned its keep twice mid-phase: the main.rs
    600-line budget (split → `mc_cmd.rs`, landed with Ф2's second
    commit) and two domain `.expect()`s in harness.rs (fixed through
    the error path, not deviated).
  - Floor at the boundary: **all green** — conform 0 (7/7 gated),
    specmap 18 units / 59 items / 59 edges / 0 orphans, test-gate
    xfail-strict. Live smoke: the full install → hooks → remove →
    `{}` → fetch-over-TLS cycle on a scratch project + scratch home.
  - MT-C2-02 authored (recorded runs land with the Ф6 index).
  - **Owner directive landed mid-phase (recorded verbatim in the
    workspace contract):** per-phase reports in `reports/` with the
    dated filename convention; Ф0–Ф3 reports written retroactively
    in this phase's close.
  - Delegation scoreboard this phase: delegated 1 (fetch.rs cell →
    glm-5-turbo, delivered; one clippy collapse-if fixed at review),
    kept: hook/statusline/harness cells (seam design + the
    availability and ownership laws), wiring, smoke.
  - Next: Ф4 (nudges + routing-as-data + question push).

- **Ф4 — EXECUTED 2026-07-10. Commit map:**
  - `2b24288` — `feat(fractality): initiative live — nudges, question
    push, routing-as-data (C2 Ф4)`. Drift: one feat commit instead of
    the three planned (the shared hook cell + engine surface; the
    Ф1/Ф3 precedent), ledgered.
  - Confirmed: **P5 (the §worked table 10/10 as goldens)**; the BD1
    slate + cooldown semantics (fold-stamped anchor, deterministic on
    replay); the bounded question push (once per question, ack folded,
    `stop_hook_active` respected); D6 as rewritten (single-source
    matrix inside the engine; the DECISION-MATRIX spec cites its
    executable form; specmap resolves the new namespace via an
    external_specs sibling root — 0 warnings).
  - Mid-phase decision rewrite: **D6** (matrix location) — in place,
    with the reason (two artifacts would drift); the delegation-rules
    MD edit rides the feat commit.
  - Findings: the stale-daemon-binary smoke trap (hooks talk to the
    sibling MC exe — hook smokes rebuild `--workspace`; the F15
    family grows a corollary, recorded in MT-C2-03); a boss-side cwd
    violation on a delegate launch (killed, relaunched pinned — the
    live-observation protocol did its job).
  - Floor at the boundary: **all green** — conform 0 (7/7; the
    delegate's domain `.expect` was restructured into total matches,
    and route.rs split its goldens to tests/route_goldens.rs along
    the 600-line budget), specmap 18 units / 62 items / 62 edges / 0
    orphans / 0 warnings, test-gate xfail-strict.
  - Delegation scoreboard this phase: delegated 1 (the route slice →
    GLM-5.2 over a sandboxed spec copy; one relaunch after the cwd
    slip; accepted after review with two boss-side fixes), kept:
    nudge policy, core fold fields, hook wiring, the specmap
    namespace decision.
  - Next: Ф5 (the answer-rules slice).

- **Ф5 — EXECUTED 2026-07-10. Commit map:**
  - `337ea86` — `feat(fractality): answer rules — profile auto-answers
    (C2 Ф5)` (the planned subject, on plan: one commit).
  - Confirmed: the staged escalation semantics exactly as the exit
    criterion demanded — a matching question resumes without a
    question (both facts journaled, provenance on the Answer via the
    additive `auto_rule` field), a non-matching one parks as before.
    The `answer --rule` config-writing flag named by the plan text was
    deliberately NOT shipped (hand-editing profiles.toml is the honest
    v1 surface) — drift ledgered, not absorbed.
  - The http.rs budget forced the question/answer leg into its own
    cell (http_questions.rs) — the third gate-forced seam this
    campaign, each along a real responsibility line.
  - Floor at the boundary: **all green** — conform 0 (7/7), specmap
    18 units / 63 items / 63 edges / 0 orphans, test-gate
    xfail-strict.
  - Next: Ф6 — the trial (RP1 RESOLVED: GLM-served cold boss, 3 runs
    per arm, cap 8) + boss surface v2.

- **Ф6 — EXECUTED 2026-07-10 (pre-registration in the first session;
  the paid arms fired and scored in the resumed session).**
  Pre-registration landed committed and frozen BEFORE any run: the
  staging fixture (`spec/manual-tests/trial/staging/` — the
  standalone `mini_logfmt` crate with the eight tasks baked in), the
  neutral menu (no delegation words anywhere — discovery rides the
  snippet), the runner (`trial/run-arm.sh`: worker-shaped clean boss
  env, per-run scratch home + git-initialized project copy, secrets
  never echoed, arm `b` = `harness install`), MT-C2-01/-04 with the
  scoring FROZEN (eligible set, attempted/delegated definitions,
  pooled metric, P1/P3 thresholds), and boss surface v2 (snippet 75:
  the `route` verb + the scoreboard section; the skill: route-as-verb
  + bus-side counters).
  - **The arms:** six paid GLM-boss runs (3+3), zero technical
    repeats (cap 8 untouched), all under the live-observation law
    (20 s telemetry polls; two near-stall GLM turns, no derailments).
    Wall ≈ 2 h 10 m total; per-run 18–25 min; two runs hit the 1500 s
    wall, two hit max-turns, two finished clean.
  - **Numbers (frozen scoring):** Arm A pooled **3/18 ≈ 16.7%**
    (1/6 · 2/6 · 0/6); Arm B pooled **0/18 = 0%**. Distractor
    delegations 0 in both arms. **P1 CONFIRMED; P3 FALSIFIED** —
    full channel analysis in §2 and MT-C2-04.
  - **The adapter proved itself live in `-p`:** every arm-B run
    installed 5 hooks + statusLine, registered a session, counted
    work-tools (slates 34/45/44), injected the scoreboard
    (text verified in the transcript), and exited 0 into a working
    boss session — the availability law held; hook records in
    `sessions.txt`; no boss session was ever broken by the fabric.
  - **Discovered facts:** (F23) the UserPromptSubmit nudge channel
    is structurally silent in single-prompt `-p` sessions — the only
    prompt arrives at slate=0; (F24) the scratch `env -i` breaks
    rustc's MSVC toolchain auto-detect — every boss hand-fixed the
    linker (vcvars wrapper) and two bosses cited worker
    self-verification doubt as a keep reason (a staging defect,
    shared by both arms); (F25) a fresh-home scoreboard is an EMPTY
    scoreboard — zero social proof at exactly the moment the
    injection fires.
  - Run artifacts under `target/trial-results/arm-{a,b}-run-{1..3}/`
    (transcripts, runs.json, sessions.txt, proj-final trees) —
    uncommitted by design (target/ is build state; the recorded
    facts live in the MTs).
  - P4/P6 benches (MC warm, n=20): post-tool-use P95 51 ms;
    user-prompt-submit P95 49 ms; statusline P95 58 ms — all under
    budget; **P4 and P6 CONFIRMED**.
  - Floor at the boundary: green (run pinned to the workspace cwd —
    one boss-side cwd violation on the first floor attempt, caught
    by the specmap error and relaunched pinned; the cwd law's second
    strike this campaign, both ledgered).
  - Owner sign-off on the MT index: **pending** (the one close item
    only the owner can perform).

- **Ф7 — EXECUTED 2026-07-10 (the close).** §2 execution record
  prepended; every prediction ruled (P1/P2/P4/P5/P6/P7/P8 confirmed,
  P3 falsified with channel analysis); P8's shadow-state grep audit
  run at close (initiative crate: zero fs writes; sanctioned writes
  only); §15 deferrals extended with the trial's actionable deltas +
  the standing leftovers; reports: Ф6 trial narrative +
  campaign-close report + completed-plan dashboard; WAL / CONTINUE /
  WORKSPACES refreshed. Planned close commit shape adapted: two
  commits (trial records; campaign close), ledgered.

## 15. Deferrals ledger (seeds Campaign 3+)

_From the trial's falsifier analysis (the P3 levers, in priority
order — these are the initiative system's actual next moves):_

- **DEF-C2-1 — a mid-work injection seam that actually fires in
  headless sessions (F23).** UserPromptSubmit never re-fires in
  single-prompt `-p`; the threshold nudge needs a channel that
  exists mid-turn: PostToolBatch additionalContext (already deferred
  once — now with field data), or a Stop-hook continue-feedback
  nudge tier (bounded like the question push), or per-tool
  additionalContext on threshold crossing. Needs its own fatigue
  measurement; this is the follow-up campaign's core.
- **DEF-C2-2 — worker self-verification credibility (F24).** The
  strongest observed keep-reason was "workers can't verify here".
  Two halves: (a) staging must ship a working toolchain (the trial
  sandbox's MSVC auto-detect broke under `env -i` — fix the fixture
  before any re-run); (b) the product half: `route`/scoreboard
  should surface *verification capability* facts ("workers run
  cargo test green in this repo: yes/no, last proven <when>"), so
  the boss's doubt is answered by data, not assumption.
- **DEF-C2-3 — the empty-board cold-start problem (F25).** A fresh
  home renders "0 runs all-time" at the exact moment the SessionStart
  injection fires — zero social proof. Options to study: seed the
  board with fleet-wide (not home-local) facts, or reshape the
  cold-board template to lead with the route verb + a worked example
  instead of zero counters.
- **DEF-C2-4 — trial re-run design.** Multi-prompt (interactive or
  scripted multi-turn) sessions so the nudge channel exists; fixed
  toolchain; consider N>3 and an Opus-class boss when budget allows
  (the RP1 validity caveat).

_Standing (pre-trial) deferrals:_

- Savings-estimation methodology (honest "what delegation saved") —
  needs its own measurement design (D7).
- PostToolBatch as a second mid-turn injection channel — folded into
  DEF-C2-1 above.
- Learned auto-answer policy suggestions (full D18 layer 2) — after the
  Ф5 slice generates field data.
- Cross-harness adapters (VibeVM Pixel, Codex boss) — after I4's core
  proves out on CC.
- RLM protocol — Campaign 3 (DEF-2, unchanged).
- Output-compression flow study — only on its own mandate (§10).

_IGNITION-era leftovers carried by the Ф5/Ф4 reports (named here so
the next mandate can drain them):_ a hook debug channel (availability
law currently swallows failures silently — fine for the boss, blind
for the developer), session TTL reaping (open sessions never expire),
per-packet answer rules (Ф5 shipped profile-level only), an
`auto_answered` counter in `stats`, quota plan limits (the rollup
reports burn; nothing enforces a ceiling).
