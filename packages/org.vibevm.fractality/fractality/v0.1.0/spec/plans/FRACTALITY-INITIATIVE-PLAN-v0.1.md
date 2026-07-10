# FRACTALITY-INITIATIVE-PLAN v0.1 — Campaign 2: the initiative system (scoreboard-driven delegation for a cold boss)

_Status: **PLANNED → EXECUTING** (commissioned 2026-07-10: «Начинай
Campaign 2» + «Goal set: сделать Campaign 2» — the owner ordered
execution, not just authoring; phases land autonomously per host Rule 4,
review points below still gate what only the owner can rule) · written
2026-07-10 against host tree `a7695ab` (clean, mirrors synced) ·
Format: `flow:org.vibevm/campaign-plans` (one file, five roles) ·
cold-executable: any phase boundary is a safe stop; the floor is green at
every boundary. Lineage: drains DEF-1 (+ two named IGNITION leftovers)
from [`FRACTALITY-IGNITION-PLAN-v0.1.md`](FRACTALITY-IGNITION-PLAN-v0.1.md) §15._

## 2. Execution record — (prepended at close)

_Empty at authoring. The executing session prepends commit range,
per-phase deltas, and the verdict on every prediction at close._

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
Texts are templates in the engine filled from MC facts; thresholds and
cooldowns in `~/.fractality/initiative.toml` (machine-scoped, like
profiles; env `FRACTALITY_INITIATIVE=off` is the kill switch). The
delegation matrix's axes/verdict procedure gains a machine-readable
`matrix.toml` in the delegation-rules package; `fractality route
--error-cost … --context … --verify … --size …` prints verdict + slot +
playbook budget defaults (`--json` for machines). The MD stays the
normative prose; the TOML is its executable form; **a golden test pins
the TOML to the MD's 10 worked verdicts** — drift fails the floor.
Rejected: NL task classification in the engine (an LLM call per prompt
— cost, latency, and a second brain to argue with; the boss IS the
classifier, the engine hands it the procedure).

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

- **RP1 — trial authorization (OPEN).** Paid cold sessions on the Max
  plan: recommend 2 runs per arm (4 total), menu of 6 eligible + 2
  distractor tasks, boss model = the real boss (Opus-class), budget cap
  per run. The owner rules count, model, and timing before Ф6 arms run.
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

- **Ф0 — (pending)**

## 15. Deferrals ledger (seeds Campaign 3+)

- Savings-estimation methodology (honest "what delegation saved") —
  needs its own measurement design (D7).
- PostToolBatch as a second mid-turn injection channel — after nudge
  fatigue data exists.
- Learned auto-answer policy suggestions (full D18 layer 2) — after the
  Ф5 slice generates field data.
- Cross-harness adapters (VibeVM Pixel, Codex boss) — after I4's core
  proves out on CC.
- RLM protocol — Campaign 3 (DEF-2, unchanged).
- Output-compression flow study — only on its own mandate (§10).
