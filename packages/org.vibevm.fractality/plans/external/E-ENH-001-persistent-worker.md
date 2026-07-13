# E-ENH-001 — persistent / resident worker (reuse one warm worker across tasks)

- **Status:** open — filed for discussion; candidate for promotion into the
  fractality postponed registry (`plans/postponed.md`, a PP-NNN slice) or a
  plan phase. Sibling to the E-BUG channel, but this is an **enhancement /
  capability gap**, not a defect: today's one-shot model works correctly; it is
  merely costly for small, frequent delegations.
- **Filed:** 2026-07-13.
- **Component:** `fractality-mission-control` (admission / launch),
  `fractality-pod` (supervision loop), `fractality-backend-claude-code` (worker
  invocation), `fractality-core` (run-state machine).
- **Motivation / found by:** a host vibevm session, 2026-07-13. The owner lifted
  the paid-run gate and directed "use fractality for every action, the more the
  better," then asked directly: *can fractality keep a worker alive across tasks
  so small actions don't each pay a cold spawn? If not, that's a needed
  improvement.* This note is the investigation's answer, distilled so a
  fractality session can act without re-deriving it.

## Verdict

**No persistent worker exists.** By explicit design today, **one run == one pod
== one worker**, all one-shot. Every admitted run spawns a fresh detached pod and
a fresh `claude --print` headless worker, and tears both down when that single
task ends. There is no warm-worker / pool / reuse / keep-alive mechanism, and no
plan phase that builds one — only a single aspirational study note (DC4).

## What I wanted

Reuse one warm worker for many small delegated actions: keep a resident worker,
feed it task after task, and pay the pod + worker startup cost **once** — so
that small, frequent delegations become cheap enough to prefer over doing them
inline.

## What I got (current lifecycle — cites)

- **One pod per run, by contract:** `fractality-core/src/run.rs:3` ("A run is one
  worker lifecycle under one pod (D3)"), `run.rs:154-159` (`PodBinding`, "one pod
  per run"); `fractality-pod/src/main.rs:2-3` ("One pod per run. The pod spawns
  the worker from a `WorkerSpec`.").
- **Launch chain:** CLI `run`/`spawn` (`fractality-cli/src/swarm.rs:84,140`) →
  mission-control admission `tick` (`fractality-mission-control/src/admission.rs:270`)
  → `launch` (`:322`) → `launch_pod` (`:357-383`) spawns a fresh detached
  `fractality-pod --run-spec` process per run.
- **One-shot worker:** the pod spawns exactly one worker
  (`fractality-pod/src/main.rs:249`, `supervise::spawn`), which is
  `claude --print` headless (`fractality-backend-claude-code/src/invocation.rs:92-104`)
  — argv carries **no** `--resume` / `--continue` / session-id flag. The pod runs
  one supervision loop (`main.rs:279-326`), ends when `child.wait()` returns,
  reports `Exit`, logs "worker exited; pod done" (`main.rs:464`), and the **pod
  process then exits** (`main.rs:74-80`).
- **Terminal states are final:** `run.rs:36-70` — Completed / Failed / Killed /
  Escalated have no outgoing transitions; a settled worker can never be revived.
- **`max_concurrent` is NOT a pool:** `admission.rs:1,291-295` — a per-profile
  slot limit with FIFO queueing; a freed slot lets the next queued run spawn a
  *new* pod. No process is retained.
- **The only "stays resident" primitive is intra-task:** the `ask_boss` broker
  parks the *same* worker mid-task, blocked on one tool call
  (`fractality-cli/src/broker.rs:8-9,201-240`) until `fractality answer` lands. It
  keeps one process alive to finish its *one* task — not reuse across tasks.

## Why they differ (the only existing aspiration)

`spec/refs/notes/codex-first-study.md:43-46` (decision DC4): *"one-shot-then-resume
becomes run + follow-up on a live pod … a parked **or completed** run can take a
follow-up without a cold restart; the pod owns the session."* This is exactly the
intent — but it lives in an inspiration-only study note feeding a future phase, is
**not** carried into any plan (IGNITION / RLM / INITIATIVE / ADVISOR / PROP-001
contain no worker-reuse item), and is contradicted by the shipped one-shot model.
The postponed registry (PP-001..005) has nothing on worker reuse.

## Ideas on the fix (hook points)

A persistent worker requires decoupling the **worker process** from the **run**
(today welded 1:1 by D3). Concretely:

- **`fractality-backend-claude-code/src/invocation.rs:85-122` (`build_argv`)** —
  the one-shot `claude --print` surface. A warm worker needs Claude Code's
  streaming-input mode (`--input-format stream-json`) or `--resume <session>` so a
  live process can accept follow-up prompts.
- **`fractality-pod/src/main.rs:249,279-326,439-465`** — today the pod supervises
  one worker then dies. A persistent pod would loop, feeding successive task
  prompts to the resident worker instead of exiting after one.
- **`fractality-mission-control/src/admission.rs:357-383` (`launch_pod`) +
  `:270-317` (`tick`) + slot logic `:291-295`** — a pool manager would check for /
  hand off to a warm pod here instead of always spawning fresh; the slot model
  extends to a resident-worker binding.
- **`fractality-core/src/run.rs:36-70` (`RunState`)** — decouple worker-process
  lifetime from run lifetime so one warm process serves many runs.
- **`fractality-cli/src/broker.rs:201-240` (`ask_boss` park/resume)** — the
  existing "worker stays alive, resumes on a signal" primitive; generalise it so a
  worker, after delivering a result, waits for the *next task* rather than exiting.

## Workaround (today)

For small, frequent actions, per-task fractality spawn costs more than the action
saves (a cold pod plus a one-shot `claude -p`), so prefer inline execution — or
the free `route` / `gate` verbs — and reserve fractality spawns for substantial
one-shot work where the startup cost amortises. This note is the request to close
that gap so the "use fractality for every action" directive becomes economical at
the small end too.

## References

- Host directive: the "Delegation-first" ledger in the host `CLAUDE.md` (2026-07-13
  grant — fractality runs pre-authorised, no longer treated as paid).
- Related: DC4 (`spec/refs/notes/codex-first-study.md:43-46`); the `ask_boss`
  broker; the postponed registry (`plans/postponed.md`).
