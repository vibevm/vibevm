# Campaign 3 · Ф1 — packets & budgets (report)

_2026-07-11 18:54. Phase Ф1 of the Stage B descent-core plan. Ships
**D-C3-2** (the packet + budget surface); **D-C3-3** (boundary behaviors)
is scoped forward to Ф2. Floor green at every slice._

## What was done

Four commit-sized slices, each floor-green:

- **Ф1.1 `context_from` access-list** (`35a378c`, D-C3-2) — `ContextSpec`
  gains `context_from: Vec<RunId>`: a child sees a prior run's RESULT
  files only when that run-id is named; default empty. Isolation is the
  default; only named results cross (the fold law). Anti
  orchestration-collapse by construction (FD-2/FD-3).
- **Ф1.2a `output_schema` field** (`d91780d`, D-C3-2) — `OutputSpec`
  gains `output_schema: Option<String>` (raw JSON Schema; string keeps
  core dep-light).
- **Ф1.2b output_schema validation at the collect seam** (`12b9824`,
  D-C3-2) — the pod validates the result against the schema with
  jsonschema 0.47.0; the verdict (checked / valid / per-violation report,
  `at <JSON-Pointer>: <message>`) rides into `status.json` under
  `schema_gate`. Format-gate-then-quality (FD-15). Pumps extracted to
  `pump.rs` for the file budget.
- **Ф1.3 budget lattice** (`19c33e9`, D-C3-2 / RD-4) — `BudgetSpec` grows
  to the six-axis lattice + wall-clock: max_depth, max_agent_calls,
  max_call_tokens, max_global_calls, max_spend_micros (currency in
  micro-USD), on top of wall_secs / max_turns / max_output_tokens. New
  axes default 0 = unlimited.

## Decisions taken

- **Currency is integer micro-USD** (`max_spend_micros`), not f64 — keeps
  `BudgetSpec` `Eq` and never stores money in a float.
- **output_schema retry-on-violation → Ф3** (§9 ledger). The seam
  produces the retry-feedback report now; the automatic one-retry is
  re-dispatch at the orchestration layer (need-gate re-spawn), not a
  pod-local re-invoke loop — a pod rewrite would violate §10.5.
- **D-C3-3 boundary behaviors → Ф2** (§9 ledger). Per-verb cap behaviors
  enforce against the need-gate's verbs and the budget caps, which don't
  exist until Ф2. Shipping profile config without the gate would be dead
  surface. D-C3-3 lands with the need-gate + depth guard, using the Ф1.3
  lattice.
- **New fields are `#[serde(default)]`** so schema stays 1 and existing
  packets/watchdog are unchanged until admission enforces the new axes.

## Left undone / open

- D-C3-3 boundary behaviors (→ Ф2).
- output_schema auto-retry (→ Ф3).
- **Enforcement of the new budget axes** (max_depth, max_agent_calls,
  max_call_tokens, max_global_calls, max_spend_micros) — the fields are
  surface; the watchdog/admission enforce wall_secs + max_output_tokens
  today. Depth enforcement lands in Ф2 (admission); the spend/call
  enforcement rides the MC watchdog extension (Ф2/Ф4). Tracked.

## Косяки / висяки (honest)

- The new budget axes are declared but not yet enforced — a packet can
  set `max_spend_micros` today and nothing checks it. This is deliberate
  (surface before enforcement, like output_schema Ф1.2a→b), but a reader
  must not mistake the field's presence for a live cap. The Ф2/Ф4
  enforcement closes it.
- `main.rs` hit the 600-line file budget and forced a mid-slice pump
  extraction; the split is clean but was reactive, not planned — a
  cheaper habit is watching file length before a seam edit crosses it.

## Next

Ф2 — the need-gate (D-C3-1): one auditable typed verdict
(inline|route|fold-local|spawn|escalate) + journaled reason + the
fixed-order decision procedure (§10.3); the `delegation-rules` package
(its own Cargo workspace) with the policy columns, availability masking,
capability-class rows, and depth guard; D-C3-10 routing policy data; and
D-C3-3 boundary behaviors folded in.
