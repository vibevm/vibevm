# Study note — codex-first (clean-room) {#root}

_Source: `steipete/agent-scripts` `skills/codex-first/SKILL.md`, MIT,
pin `d6ed98c` (2026-07-09). **Inspiration-only** (host clean-room
directive). This note records what the source achieves and which
decisions fractality takes; no text or code is carried over. The
Phase 5 `delegation-rules` package is authored from THIS note, never
from the source file._

## What the source achieves

A single-page routing rule for one delegation relationship: an
expensive judgment model routes hands-on coding to a cheaper flat-rate
implementer, keeps design/spec/review for itself, and always reviews
the delegated output. It carries five parts: a rationale (why delegate
at all), a route table (delegate-this / keep-that), an invoke recipe
(temp-file prompt, one-shot + resume), a prompt contract (the delegate
starts with zero context), and an economics line (what actually gets
saved). Its core heuristic: if the prompt reads as a work order,
delegate; if writing the prompt forces the decisions, it *is* design —
keep it.

## What we keep (decisions)

- **DC1 — the work-order/design heuristic is the seed of our matrix,
  but not its center.** We keep the insight; we replace the single
  heuristic with a decidable calculus (see the plan's Phase 5): the
  center is **"delegate when verification is cheaper than
  generation."** That reframes routing as an economic asymmetry the
  boss can actually evaluate, not a vibe.
- **DC2 — the keep-set is a hard boundary, not a preference.** Design,
  ambiguity-as-design, tiny edits, session-tool/secret work,
  destructive/irreversible ops, and *review of delegated output* stay
  with the boss. fractality encodes several of these as mechanism, not
  advice: secrets are unreachable by a worker (I1/I6), destructive ops
  and unlisted tools fail closed at the pod broker (D18), review is the
  boss's half of every packet's acceptance contract.
- **DC3 — zero-context prompting becomes the task packet.** The source
  hand-writes a temp-file prompt each time; we make the packet a typed,
  versioned artifact (D7) carrying goal + paths + constraints +
  non-goals + acceptance-as-command + output contract. Same discipline,
  made a schema instead of a habit.
- **DC4 — one-shot-then-resume becomes run + follow-up on a live pod.**
  The source's "resume, don't re-run" economy maps onto our pod: a
  parked or completed run can take a follow-up without a cold restart;
  the pod owns the session (D3/D18).
- **DC5 — "delegate output is advisory; prove it" becomes the
  acceptance block.** The source demands proof output and a real diff
  read; we make acceptance commands part of the packet, run them in the
  worktree, and record pass/fail — the boss reviews the diff as a
  contributor PR (D14, Phase 6 dogfood acceptance).
- **DC6 — "after N failed rounds, take over" is a real rule.** We keep
  a bounded-retry posture (a budget in the packet; the boss reclaims
  the task rather than ping-ponging) — R4's fallback.

## Where we go further (the mandated improvement)

- **A decidable matrix, not a two-bucket list.** Four axes — task size
  × error cost × context transferability × verifiability — yield a
  routing verdict per task; the Phase 5 prediction is that it decides
  ≥ 8/10 real tasks without a judgment call. The source lists examples;
  we produce a decision procedure.
- **Model-aware routing, not one delegate.** The source has exactly one
  target (Codex). We route across a profile's model slots — big vs
  small — with per-model playbooks (GLM-5.2 coarse-grained one-shot;
  GLM-5-Turbo Haiku-class mechanical) and an extensible `_template` for
  future backends (Codex, VibeVM Pixel). Routing is data, not prose.
- **Tariff hygiene as mechanism.** The source is silent on provider
  quotas. We deny web tools to workers by profile and fetch documents
  locally once (D12), because the GLM plan's MCP-call budget is a
  metered resource — a fact the source's flat-rate assumption never
  faced.
- **Metering closes the loop.** The source asserts the economics; we
  measure them (MC journal / `stats`), so "did delegation actually pay"
  is an answer, not a claim — and the Campaign-2 initiative system
  reads that same signal.
- **Harness-neutral framing.** The source is explicitly "Claude Code
  sessions only." Our matrix and playbooks name roles (boss / worker /
  backend), so the same policy survives when the boss is Opus inside a
  different harness (I4).

## Non-adoptions (named)

- The source's `--yolo` house default is explicitly rejected for v0.1
  (RP4): the D18 allowlist + broker stack is our way of life.
- The exact invoke recipe (Codex CLI flags, `command codex`, fnm) is
  Codex-specific and not ported; our invoke surface is the backend
  adapter + `fractality run` (D2/D13).
- `maintainer-orchestrator` (the source's multi-repo escalation) is out
  of scope; our swarm/tree (Phase 4) is the multi-worker story.
