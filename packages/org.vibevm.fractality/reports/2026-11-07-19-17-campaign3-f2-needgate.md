# Campaign 3 · Ф2 — need-gate + routing policy (report)

_2026-07-11 19:17. Ships the need-gate's decision machinery — **D-C3-1**
(the procedure) and **D-C3-10** (the capability-class routing policy) —
with goldens (§6). The gate's invocation, enforcement, and availability
masking are scoped to Ф3, where the verbs use it. Floor green each slice._

## What was done

- **Ф2.1 need-gate decision procedure** (`5adcceb`, D-C3-1) —
  `needgate::decide(&GateInputs) -> Decision` implements plan §10.3 in
  fixed order, first match wins: O(1)→inline, fits-window+single-skill→
  route (never decompose what fits — RD-2), cross-chunk→escalate or
  route-to-biggest-window (Silo, RD-6), decomposable→spawn under the
  cap / fold-local at it (D-C3-3 at-cap), else fold-local. Pure over
  boolean signals + caps, so every verdict is a unit test; verdicts
  serialize kebab-case for the journal; the reason is journaled verbatim.
- **Ф2.2 routing policy** (`011ef6c`, D-C3-10) — `RoutingPolicy` maps
  weak/medium/strong capability CLASSES (never model names — FD-16) to a
  ClassPolicy row (max_depth, experimental-depth2 flag, advisor bar).
  Authored as data in the delegation-rules package
  (`routing-policy.toml`, §10.6); the compiled `Default` mirrors it and
  is the per-class fallback. Classes are `Ord` (the advisor bar is a real
  `≥ medium`). Hand-authored data in v1 (RD-20 defers a learned router).
- **Ф2.3 profile capability_class** (`14f97b8`, D-C3-10) — a profile
  declares which class its workers present; defaults to `medium`.

## Decisions taken

- **The gate is a pure library** — the decision procedure and policy are
  data + a total function; the boss/MC call site stays a thin wrapper.
- **Gate WIRING → Ф3** (§9 ledger): the invocation (a `fractality gate`
  CLI surface + journaling the decision tuple), the enforcement
  (admission's spawn-past-cap depth guard — D-C3-3), and availability
  masking (FD-8) land in Ф3, where the spawn/route verbs use the gate.
- **`no-unwrap-in-domain`** forced the routing fallback to a total match
  (`compiled_default_for`) instead of a map+expect — a better shape
  anyway (single source of truth for the compiled rows).

## Left undone / open (→ Ф3)

- The `fractality gate` invocation surface + the decision journal event
  (D-C3-8 decision tuple).
- Admission enforcement of the depth cap (spawn-past-cap → structured
  refusal) + the other budget axes (D-C3-3 boundaries).
- Availability masking — route over the usable-profile subset (FD-8).
- Reused `CoreError::PacketSchema` for `RoutingPolicy`'s foreign-schema
  error — a dedicated variant is a cheap later refinement (noted).

## Косяки / висяки (honest)

- The gate has no caller yet — it is a tested library. This is deliberate
  (machinery before wiring, faithful to §6's "goldens for the policy
  table"), but a reader must not mistake a green `decide` for a live
  descent: nothing invokes it until Ф3.
- The routing policy's authored TOML and the compiled default are kept in
  sync by hand (no golden cross-package test, to avoid coupling
  fractality-core's tests to the delegation-rules path). Review must
  catch drift until a loader test lands.

## Next

Ф3 — descent verbs (D-C3-4, D-C3-5): `await any|all|named` + parallel
siblings + mid-task profile alternation; sibling isolation by default
(visibility only via `context_from`); a designated merge node; refuse
near-duplicate child specs. Plus the deferred gate wiring: invocation,
the admission depth guard, and availability masking.
