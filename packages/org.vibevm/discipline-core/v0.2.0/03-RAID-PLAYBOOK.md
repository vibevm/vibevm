# The Raid Playbook — Layered Refactoring Sweeps
**Discipline v0.2 · status: BETA · T1**

*The macro-rhythm of the Discipline. Inline triggers (the micro-rhythm) apply cards per-edit where cheap; a RAID applies a set of cards across a whole layer when per-edit triggers cannot keep up — because attention budget is exhausted, or a new card is adopted repo-wide. A raid is itself spec-driven and follows the same gate discipline as the original terraform.*

## 0. When a raid, not an inline trigger

Inline (edit-time) handling is always preferred. Escalate to a raid when:
- a card's trigger is **raid-mode** by nature (e.g. "naming uniformity across a crate" — not worth firing per keystroke, only meaningful in bulk);
- a **new card is adopted** and must be applied to existing code repo-wide;
- **debt accumulates** past a threshold (the A6 debt ledger trips a tripwire);
- the swarm's **attention budget is structurally insufficient** for a class of cross-cutting concern, so it is swept periodically instead of held active.

## 1. Raid plan skeleton (every raid is authored to this shape)

1. **Scope & freeze.** Which layer(s)/crates are in scope; which surfaces are frozen for the raid's duration. Frozen surfaces may not change except by the raid.
2. **Card set & order.** The cards to apply, **topologically sorted** by their Band-3 `raid_role.order` dependencies. Example ordering constraint: naming-uniformity (Class B/names) BEFORE contract-extraction (Class C), because contracts cite names; differential-oracle (Class D) wraps every behavior-changing card as a gate.
3. **Per-layer phases.** The sweep proceeds layer by layer (seams → cells → registry → tests, for Rust), each phase gated green before the next begins. This is the owner's "refactor everything by layers."
4. **Batch units & checkpoints.** Per-cell or per-crate batches; each batch has a green-gate checkpoint. The raid is **WAL-backed and resumable** — a crash or pause never loses progress, and the raid is never one giant diff (R3-013 determinism; phantom-diff avoidance).
5. **Differential safety.** Every card application that changes behavior carries its Class-D oracle. The raid **cannot move behavior silently** — a behavior change without a passing oracle blocks the batch.
6. **Exit criteria.** All targeted cards' checkers green across scope; the raid's debt ledger at zero; a **raid REPORT** (modeled on the terraform REPORT) listing what the sweep learned — including cards that misfired, false-positive triggers, and routines that overloaded weak readers. The REPORT feeds card revision (cards are beta, revised on pilot evidence only).

## 2. Roles in a raid

- **Strong author/orchestrator** — authors the raid plan, sets scope and order, adjudicates review-mode triggers.
- **Weak swarm** — executes per-batch routines (the Band-3 extract of each card), one batch per agent, meeting only at the merge (R3-013: parallel agents share no state; the merge is the contention point).
- **The toolchain** — runs checkers per batch (conform tiers, `cargo test -p <cell>`, oracles), emits structured diagnostics (Class F) as the agents' percepts.

## 3. Relationship to the original terraform
The first vibevm terraform (PLAYBOOK-TERRAFORM-VIBEVM-v0.2) proved the gate-and-phase machinery: phases −1…6, frozen baselines, green gates, a closing REPORT of what the discipline learned about itself. A raid is that machinery, generalized and repeatable: not a one-time greenfield-to-disciplined migration, but the standing mechanism for applying any card-set across any layer, any time. The vibevm-specific adoption of THIS discipline (in `vibevm-terraform/`) is itself executed as a sequence of raids.

## 4. Cadence
- **Micro (continuous):** inline triggers in the per-cell loop.
- **Gate (per-merge):** gate-mode triggers at the cell's verification gate.
- **Raid (scheduled/on-adoption):** layered sweeps per this playbook.
- **Review (as-flagged):** review-mode triggers escalated to a stronger reader.

Together these answer "when do we switch on rethinking and refactoring": continuously where cheap, in planned sweeps where not.
