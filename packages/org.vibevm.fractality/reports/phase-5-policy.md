# Phase 5 report — the policy layer: delegation-rules v0.1.0

_Campaign: FRACTALITY-IGNITION v0.1 · Phase 5 · executed 2026-07-10.
The plan's §14 ledger stays the canonical commit map._

## What the phase delivered

A second package in the group:
`packages/org.vibevm.fractality/delegation-rules/v0.1.0` — kind=flow,
UPL-1.0, boot snippet slot 77, authored **clean-room from the
codex-first study note (DC1–DC6) plus this campaign's own field
data** — never from the studied source text (clean-room law held).

- **`DECISION-MATRIX.md`** — the routing calculus. One law: *delegate
  when verification is cheaper than generation* (the study's
  work-order/design heuristic kept as seed, replaced as center). Four
  flat-valued axes (error cost, context transferability,
  verifiability, size), a five-step first-match verdict procedure
  (three KEEP-gates, then slot routing), the hard never-delegate set,
  packet sizing guidance, the bounded-retry rule (small → big → boss
  reclaims, never more), and the boss-as-reviewer loop wired to the
  packet's `task.acceptance`. Routing names **slots** (`big`/`small`),
  never vendors — the policy survives future backends (I4).
- **Playbooks** — `glm-5.2.md` and `glm-5-turbo.md`, field-calibrated
  (each blind spot cites what it cost this campaign: MSRV drift,
  poisoned cwd, silent stalls, constraint-conflict creativity), plus
  `_template.md` as the extension schema demanding field data before a
  new model routes real work.
- **Boot snippet 77** — the always-on rules for a consuming boss: the
  law, the never-delegate set, the two work-order scenarios, four
  Never lines.
- Standing rule honored: the package's `vibe.toml` requires
  `flow:org.vibevm.world/redbook` ^0.2.0 and `stack:org.vibevm.ai-native/rust-ai-native`
  ^0.7.0, materialised with the working-tree vibe (26 packages into
  its own `vibedeps/`, boot artifacts generated) — the manifest parsed
  and resolved on the first try, which is itself a pilot data point
  for vibevm.

## The phase prediction — measured

_"The matrix yields a decidable verdict for ≥ 8 of 10 randomly drawn
recent host-repo tasks."_ Checked against the ten most recent real
commits (the campaign's own task stream — each task's true context is
fully known, making the check honest):

| Task (commit) | Verdict | Matched reality? |
|---|---|---|
| Cargo.lock tail chore | KEEP (tiny edit) | yes (boss, 1 min) |
| Phase 3 ledger | KEEP (untransferable/genre) | yes |
| MT-01 document | KEEP (boss pen, manual-tests law) | yes |
| Acceptance runner + exit families | KEEP (judgment × M: seam design) | yes |
| WAL checkpoint | KEEP (session state) | yes |
| Stream-json goldens | DELEGATE big (compilable × mechanical × M) | yes — was delegated, green |
| Collection/metering/sync run | KEEP (judgment × M–L) | yes |
| Phase 2 ledger | KEEP | yes |
| Worktree integration tests | DELEGATE big (compilable × mechanical × M) | yes — was delegated, green |
| F14 spawn fix | KEEP (context untransferable: diagnosis IS the task) | yes |

**10/10 decided with no judgment call — prediction PASS** (bar was
8/10). More telling: the two tasks the matrix routes to workers are
exactly the two that were delegated in the field, and both landed
green first try.

## Important decisions

1. **A decidable procedure, not a heuristic pile.** First-match rule
   order (irreversible → untransferable → judgment×small → route) is
   what makes 10/10 possible; any "weigh the factors" formulation
   would have failed the prediction by construction.
2. **The never-delegate set is enumerated in the boot snippet**, not
   just the matrix — the always-on rules must survive a session that
   never opens the full document.
3. **Blind spots are paid-for only**: playbooks refuse hypothetical
   weaknesses; every entry cites a real incident from the campaign.
   `_template.md` enforces the same bar on future models ("a playbook
   without field data is a hypothesis and must say so").
4. **Requires kept despite being vacuous today** (prose-only package):
   the standing rule is owner-worded and unambiguous; the note in
   `vibe.toml` records why it still binds (the day policy becomes
   routing-as-data consumed by MC, the discipline is already there).

## Strange things / surprises

- The freshly authored matrix, applied retroactively, **agrees with
  every routing decision this campaign actually made** — including the
  ones made before the matrix existed. Good sign the calculus encodes
  the practiced instinct rather than an aspiration; also a caution:
  the validation set and the authoring experience overlap, so the
  first genuinely foreign task stream (Phase 6 dogfood, Campaign 2)
  is the real test.
- vibevm pilot note: `vibe install` inside a *sibling package of the
  same group* worked first try against the local registry — the group
  resolves its own neighbors without special-casing.

## Deliberately deferred / unfinished (named)

- **Routing-as-data**: the axes are prose; a `routing.toml` consumed
  by MC (auto-suggesting a slot per packet) is Campaign-2 material —
  the initiative system needs exactly that signal.
- **codex.md playbook**: the `_template` exists; no Codex backend, no
  card (the matrix's model-neutrality carries until then).
- **Measured economics**: the matrix asserts the verification-cost
  asymmetry; `stats` (Phase 6) starts *measuring* it (metering closes
  the loop — the study note's mandated improvement #4).

## Delegation scoreboard (the law's ledger)

- **Delegated this phase: 0** — with cause: the entire phase is
  spec/policy authoring, the matrix's own never-delegate set
  (architecture/spec authoring; ambiguity-as-design). Delegating the
  delegation policy would be the joke version of the failure mode the
  law exists to prevent.
