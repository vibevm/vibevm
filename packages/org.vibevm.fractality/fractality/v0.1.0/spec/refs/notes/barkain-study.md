# Study note — barkain workflow-orchestration (clean-room) {#root}

_Source: `barkain/claude-code-workflow-orchestration`, MIT, pin `175311b`
(2026-06-20). **Inspiration-only** (host clean-room directive; INVENTORY
row S2). This note records what the source achieves and which decisions
fractality takes; no text or code is carried over. The Campaign-2
initiative system is authored from THIS note, never from the source.
Owner's standing verdict (2026-07-09, verbatim): «barkain был просто как
ранний прототип одного из решений» / «та ерунда, что написана у barkain
это ужасное питоноговно, и так делать не нужно» — so the note harvests
design lessons, positive AND negative, and never the implementation._

_Provenance: surveyed 2026-07-10 by a big-slot delegate over a sandboxed
copy (concept-level survey, no text carried); the boss spot-checked the
load-bearing claims against the source (hook registry: 12 command
entries across 6 lifecycle events; the 7-tool work-primitive set
excluding `Read`; the Stop-hook `decision: "block"` forced continuation;
plan persistence to a state file). This note is the durable artifact._

## What the source achieves

A Claude Code plugin that turns the main agent into an orchestrator by
**layered initiative mechanisms** graded from ambient to coercive:

- a tiny SessionStart-injected routing stub (~1 KB) that lazy-loads the
  full orchestration playbook only when its delegate command runs;
- a per-turn **escalating nudge** when the main agent uses one of seven
  work-shaped tools directly (never blocks — exit 0 stderr, louder with
  each violation; the counter zeroes when the agent delegates, so
  choosing the right path cleans the slate);
- advisory hints on task-graph order and decomposition depth;
- completion-time reminders that weave verification in (spawn the
  verifier agent after each subagent finishes);
- a Stop-hook **forced continuation** after plan approval (`decision:
  "block"` + plan recovered from a state file) — its one true coercive
  mechanism besides a Python-write lint gate;
- a per-turn state fence (UserPromptSubmit wipes counters and markers);
- an ambient **statusline** whose context-fill bar, rate-limit
  percentages, and cost readout argue visually for offloading work to
  isolated-context subagents.

State is a bag of small JSON/marker files under the project's
`.claude/state/`; the hooks are twelve near-identical Python scripts run
via `uv`; richer validation designs exist only as deprecated docs. The
system's honest self-description: initiative rests on model compliance —
almost everything is advisory, and a willful model can talk past it.

## What we keep (decisions)

- **BD1 — the escalating per-turn counter becomes our contextual
  trigger, not our center.** barkain's primary driver is punitive
  escalation; our center stays the owner's scoreboard-instead-of-
  coercion. We keep the *observation*: count work-shaped tool events
  (Bash/Edit/Write/MultiEdit/NotebookEdit — reads never count) per boss
  session, zero the counter on a real delegation, and inject a nudge
  only past a threshold with a cooldown. Reward the right path; never
  block.
- **BD2 — the ambient statusline is adopted; the content flips from
  fear to wins.** barkain shows context filling up; we show the
  scoreboard — delegated N, outcomes, parked questions, quota burn —
  the owner's «scoreboard вместо принуждения» made ambient.
- **BD3 — the lazy SessionStart stub is adopted.** A compact live
  injection at session start (scoreboard line + parked questions +
  a pointer to the matrix/skill), with the full corpus staying in the
  static boot snippet and the skill. Injection is data (live numbers);
  law stays in the spec.
- **BD4 — worker/subagent exemption is right.** Initiative machinery
  must detect fractality workers and exit silently (they ARE the
  delegation; nudging them is noise and latency). Detection rides the
  injected `FRACTALITY_*` worker env (I1), not heuristics.
- **BD5 — the per-turn fence is adopted for nudge state.** Nudge
  counters and cooldowns are turn/session-scoped and reset cheaply;
  durable telemetry lives in MC and never resets (I3). No stale-marker
  cleanup rituals — MC state has an owner and a lifecycle.
- **BD6 — verification-weaving is adopted in spirit.** barkain reminds
  the lead to verify after each subagent; our packets already carry
  acceptance commands (DC5), so the initiative layer surfaces
  *unreviewed collected runs* on the scoreboard instead of nagging
  per-completion.

## Where we go further (the mandated improvement)

- **Metrics-fed, truthful nudges.** barkain counts violations; we read
  real economics from MC (I3) — delegated runs, outcomes, tokens, cost,
  parked questions — so an injection cites facts, not just rules.
- **Harness-neutral core (I4).** The policy engine is Rust behind
  neutral verbs (`fractality hook <event>` / `statusline` /
  `scoreboard`); Claude Code specifics live in one thin adapter. barkain
  is CC-only by construction (Python hooks, CC plugin manifest).
- **Attributed sessions.** barkain has no notion of whether delegation
  actually happened beyond a marker file; we register the boss session
  in MC and stamp every spawn with its origin, so the scoreboard is a
  measurement, not an honor system.
- **Tested policy.** Their prompt-regex contracts and state machine are
  untested prose; our engine decisions and hook I/O are unit- and
  golden-tested under the floor.

## Non-adoptions (named)

- **Stop-forced continuation as workflow glue** — the most coercive and
  most fragile mechanism (their own docs contradict its behavior; CC
  caps consecutive stop-blocks). Our Stop hook may interrupt a stop
  ONLY for an unacknowledged parked worker question — a bounded,
  deduplicated alert, not a workflow engine.
- **State as a bag of racy JSON files** under `.claude/state/` — no
  locking, no atomicity, manual `rm` recovery. Rejected wholesale: MC is
  the bus and the store (I2/I3).
- **The script zoo** — twelve near-identical interpreter scripts with
  copy-pasted platform boilerplate. Rejected: one binary, one engine,
  subcommand per event (DEF-1's «hook target is a fractality
  subcommand, never a script zoo», confirmed by the field).
- **Keyword agent-matching and the binding-contract plan protocol** —
  we route by the delegation matrix (data, not keyword heuristics) and
  supervise through MC (packets, budgets, trees), not prose contracts.
- **Bash rewrite / output compression** (`token_rewrite`,
  `compact_run`) — token hygiene, not initiative; brittle per-tool
  output parsing. Out of Campaign-2 scope; if ever wanted, it is its
  own flow with its own study.
- **The Python lint gate** (their only hard block) — we have the floor;
  worker output is gated by acceptance commands and project gates, not
  by a hook.
