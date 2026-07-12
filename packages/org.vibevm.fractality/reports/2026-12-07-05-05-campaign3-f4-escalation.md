# Campaign 3 · Stage B · Ф4 — Escalation (D-C3-6) — phase report

_Written 2026-07-12 ~05:05. Owner-facing narrative; the plan §9 ledger is
the canonical commit map, the WAL the living state. Phase **COMPLETE**._

## TL;DR

The ascent is in. A run can now hand its **whole task** up the tree —
`escalated(reason, needs)` — as a first-class terminal outcome (not a
failure), and that hand-up is visible to the parent that awaited it and to
the human at the top. D-C3-6 landed end to end across four floor-green
slices plus one forced refactor:

| slice | what | commit |
|---|---|---|
| Ф4.1 | core outcome: terminal `RunState::Escalated`, `EscalationRecord`, typed `Event::Escalated` + fold, `escalated` metrics counter | `e13ddbf` |
| Ф4.2 | the climb: exit code 5, `fractality escalations` inbox with call-tree-root attribution, run-summary lines | `6ed04e6` |
| (refactor) | carve the MC pod leg into `http_pods.rs` + `pod_leg.rs` (headroom) | `2ce35f8` |
| Ф4.3a | `POST /v0/runs/:id/escalate` endpoint (`http_escalate.rs`) + `McClient::escalate` + integration tests | `3f9a2e4` |
| Ф4.3b | the `escalate` MCP tool in the broker — the worker's own surface | `0bf4242` |

Floor green at every boundary (test-gate now 211, conform 0, specmap
clean). Real `~/.fractality` never touched.

## What was done

Escalation generalises the D18 question/answer park channel from *single
questions* to *whole tasks*, with one deliberate divergence: it is
**terminal**. A parked question suspends a run and resumes it on the
boss's answer; an escalation ends the run — the task itself moves up. So
the shape is D18's, but the state machine and the semantics differ:

- **Core (Ф4.1).** `RunState::Escalated` is a new terminal state reachable
  only from `running` and `waiting_on_boss`. `EscalationRecord{reason,
  needs}` rides `RunRecord`; `Event::Escalated` is a typed terminal event
  next to `completed`/`killed`, folded in `journal_fold.rs`. Metrics gained
  a dedicated `escalated` counter so a terminal escalation is never
  miscounted as `open` and never folded into `failed`.
- **Climb (Ф4.2).** `state_code` gives escalation its own CLI exit code
  (5) so a parent awaiting a child can tell "escalated" from "failed" and
  re-escalate rather than treat it as breakage. `fractality escalations`
  is the boss's inbox — the ascent twin of `fractality questions` — and it
  attributes each escalated run to the **root** of its call tree via a
  `parent`-edge walk (`root_of`, with a dangling-parent stop and a cycle
  guard), so the human sees which top-level task each escalation belongs
  to. The run summary shows reason/needs wherever a run is printed.
- **Worker surface (Ф4.3a + Ф4.3b).** A `POST /v0/runs/:id/escalate`
  endpoint records the event and persists `escalation.md` on the plane;
  `McClient::escalate` is the client verb; the broker's MCP server serves a
  second tool, `escalate(reason, needs)`, whose description tells a worker
  WHEN to reach for it. A worker calls it, the run goes terminal, and the
  tool result tells the worker to stop.

## Ideas & reflections

- **The park channel was the right prior.** Reusing D18's shape (bus event
  + plane file + broker tool) meant escalation slotted in with almost no
  new concepts — the only genuinely new idea is "terminal outcome that
  climbs," and even the climb reuses the existing `parent` edges and the
  state filter (`runs(Escalated)` needed no new endpoint).
- **The exit code is the load-bearing part of the climb.** Everything else
  (the inbox, the summary lines) is ergonomics; the thing that actually
  lets a *tree* climb is that a parent's `wait`/`run` returns 5, distinct
  from 1. That is the seam a recursive ascent hangs off.
- **Escalation as a first-class, non-failure outcome is a product edge.**
  Most swarm frameworks collapse "I can't do this" into failure; making it
  a named outcome with a reason and an ask is what lets the Silo-regime
  task (P-C3-d) escalate-and-score-better instead of fanning out and
  saturating.

## Decisions taken (the main thing)

1. **`Escalated` is TERMINAL, not a park.** The Ф0-s4 design said so and it
   holds: the run that escalates is done; the *record* climbs, the run does
   not resume. This is the one deliberate divergence from the D18 channel.
2. **Edges: `running`/`waiting_on_boss → escalated` only.** `queued` and
   `starting` cannot escalate — nothing has engaged the task yet; "this
   needs to go up" there is a gate-time `route`/`escalate` verdict (D-C3-1),
   not a run outcome. Minimal set (§10.8); widening to `starting` is
   reserved for a future result-status exit and is a backward-compatible
   edge-add.
3. **Worker expresses escalation via an MCP tool, NOT a result-status
   exit.** This was the open Ф0-s4 question. The tool fits the state
   machine (the worker is `running` when it decides, so `running →
   escalated` is clean) and matches the plan's "generalise the D18
   channel" framing. A result-status exit would fight the machine (the pod's
   Exit event completes the run *first*, and a terminal run cannot then be
   re-marked escalated). Recorded in the §9 ledger.
4. **A new `escalated` metrics counter, not a remap.** Escalation is
   counted apart from completed/failed/killed/open so the Ф6 trial
   scoreboard reads it as its own signal.
5. **The pod leg was carved into its own cells** (`http_pods.rs`,
   `pod_leg.rs`) to make budget headroom before adding the escalate route
   and verb — the standing "carve before adding" rule. Pure move, no
   behaviour change.
6. **Delegation mechanism switched: opencode → CC+z.ai** (owner-prompted).
   See below — the single most consequential process decision of the phase.

## The delegation switch (process — carries into the rest of the plan)

opencode/GLM stalled again this session (booted, model turn produced no
tool output, killed at ~3 min). The owner pointed out the obvious: launch
GLM the way **fractality itself** does — headless Claude Code (`claude -p`)
at the z.ai Anthropic-compatible gateway, the exact recipe this workspace's
`backend-claude-code/envbuild.rs` builds. It worked first try: GLM
(glm-5.2) carved the pod leg out of `http.rs` (600→379), cleaned every
unused import via a clippy loop, and self-verified — a clean diff on
review. The recipe, the secrets-safe token handling, and the owner's watch
heuristics (silent >5 min ⇒ kill; active ⇒ wait to 30 min) are recorded in
the state-plan tracker's delegation scoreboard. **This supersedes the
opencode recipe** for Ф5→Ф7 and PP-003: mechanical carves, bulk edits, and
run-and-report all go to CC+z.ai GLM now; the boss keeps seam design and
reviews every diff.

## What is left undone / висяки (honest)

- **Worker-stop is cooperative, not enforced.** After `escalate` the run is
  terminal and the tool result says "stop," but nothing *forces* the worker
  process to end its turn. A worker that keeps working wastes tokens on a
  dead run (its later events are absorbed as a kill-tail — harmless, but
  wasteful). Enforcing it (pod reaps the worker on the terminal transition)
  is a pod feature, deferred. Flagged in the Ф4.3b commit body.
- **No automatic re-dispatch of an escalation.** `fractality escalations`
  SHOWS the boss what came up; acting on it (re-spawn with more capability /
  a bigger window, or escalate further to the human) is still a manual boss
  step. The recursive climb is *emergent* (each level's worker decides), not
  yet a fabric primitive. Likely a delegation-rules concern (Phase 5) — an
  `on_escalation` policy — not v1.
- **The climb's root attribution is client-side over `runs(None)`.** Fine
  for v1's small registry; a large registry would want a server-side
  subtree query. Noted, not urgent.
- **`escalation.md` has no reader.** It is written to the plane (I2) but no
  verb renders it yet; `fractality escalations` reads the record, not the
  file. Symmetric with `question.md`/`answer.md` — acceptable.
- **`state_code` exit 5 is not yet exercised by a real parent tree.** The
  unit is there; an end-to-end "child escalates, parent observes 5, climbs"
  scenario is trial-time (Ф6) material.

## Global / strategic

- **The ascent completes the descent→ascent symmetry** the mandate asked
  for (Option B). With Ф1–Ф4 done the RLM now has: packets + budgets, the
  need-gate, the descent verbs, and the escalation channel. What remains is
  acceptance gating (Ф5), the trial (Ф6), and close (Ф7).
- **The delegation switch is the strategic win of the phase.** It turns the
  boss's scarcest resource back into a reviewer-of-delegated-work instead of
  a doer, using fractality's own mechanism — the pilot eating its own dog
  food. Every remaining phase should lean on it hard.
- **Conform's cell budget keeps forcing good structure.** Three splits so
  far this campaign (journal, pod leg ×2) — each left the tree cleaner. The
  budget is doing its job as a design pressure, not just a lint.

## Next

**Ф5 — acceptance / PP-002 fold-in** (RD-11, FD-9): acceptance verdicts can
gate run-tree completion (verifier-accept), and an acceptance packet on an
empty/workless tree is refused (no cold verification). Then Ф6 (trial —
paid arms behind the MT-C3-01 pre-registration gate, RP-C3-2 pre-authorised)
and Ф7 (close). After the whole Stage B plan: PP-003 (Option C advisor
slice, D-C3-7).
