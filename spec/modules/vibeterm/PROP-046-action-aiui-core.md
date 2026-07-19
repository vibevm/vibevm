# PROP-046 — the vibeterm action/AIUI core {#root}

**Status:** DRAFT (v0.1, 2026-07-19) — pre-MVP architectural sketch; the normative REQs the engine cells
are traceable to. **Module:** `vibeterm`. **Genre:** contract. **Lore:**
[`architecture.md`](architecture.md) §1–§6. **Source:**
[`research/vibeterm/vibeterm-ui-architecture-findings-v0.1.md`](../../../research/vibeterm/vibeterm-ui-architecture-findings-v0.1.md)
§2/§3/§8 (deltas D1–D10, D13, D15). **Related:** PROP-044 (shell), PROP-047 (ModelView/MVC + transport),
[PROP-039](../vibe-actions/PROP-039-action-system.md) (provenance — the Rust twin this conforms to).

This contract governs the **vibeterm action system** — a render-free, addressable, programmatically-
drivable behaviour layer, **self-contained in TypeScript**, conformance-tested against the Rust
`vibe-actions` identity-grammar (§10). It is the behaviour-layer twin of `spec://`: `spec://` addresses
facts, `action://` addresses behaviour.

## 1. Overview & the render-free invariant {#overview}

REQ. `vibeterm-core` is a standalone TypeScript module that models **what the shell can do** as first-class,
addressable **Actions**, independent of any rendering technology.

REQ {#no-render-dep}. The engine imports **no rendering dependencies** — not `solid-js`, not the DOM, not
`electron`, not `@xterm/*`. This invariant is enforced by a **dependency-boundary lint** on the floor (the
TS twin of [PROP-039 §1 `#no-render-dep`](../vibe-actions/PROP-039-action-system.md#overview)). Nothing in
`engine/**` may reference a rendering type.

## 2. Addressing {#addressing}

REQ {#address-grammar}. An action is named by an **`ActionAddr`** with the textual form
`action://<group>/<name>[?<params>]`; `group = vibeterm`; `(group, name)` is the identity; the query is
not. `parse(display(a)) === a` for the identity part; parsing rejects malformed input with a typed
`Result`, never a throw.

REQ {#address-uniqueness}. `(group, name)` is globally unique within a registry; a rename is a new
identity; the old address retires as a tombstone or alias. A published address is never silently repurposed.

## 3. The Action value {#action-value}

REQ {#action-fields}. An `Action` carries exactly: its `ActionAddr`; a `Presentation` (§`#presentation`); a
`ParamSchema` (PROP-047-adjacent, possibly empty); an enablement predicate; an `invoke`; a `Capability`;
`SearchMeta`. No other state.

REQ {#action-snapshot}. What a surface renders or an AI observes is an **immutable resolved snapshot**
(address + resolved presentation + resolved enablement for a context); change is delivered by
re-resolution, not mutation.

REQ {#presentation}. `Presentation` carries a **name** and a **description**, both mandatory, non-empty,
meaningful, localizable (§`#i18n-catalogue`). An action with an empty or placeholder name/description is a
contract violation caught by the legibility gate (§`#legibility-gate`). Both are first-class searchable
fields (PROP-047 SE).

## 4. The Registry {#registry}

REQ {#registry-collision}. Actions register into an enumerable registry. Registering a `(group, name)`
already present is a **hard, deterministic error** naming both the incumbent and the newcomer; never a
silent override.

REQ {#registry-integrity}. Any address reference (a keymap binding, a menu placement, an SE action
provider) is validated at registration/build time — a dangling reference fails loud then, never silently
at click time.

REQ {#registry-enumeration}. The registry is fully enumerable; this backs the legibility golden, Search
Everywhere, and `list_actions` (§`#aiui`).

## 5. Context & enablement {#context}

REQ {#context-snapshot}. Enablement and invocation read a **typed context snapshot** — a branded-symbol-
keyed `Map` (`Ctx`) holding strongly-typed values a surface has published. Reading a key yields the real
type; no stringly keys, no unchecked casts.

REQ {#enablement}. An action's enablement is a **pure, fast function** `(ctx: Ctx) => Enablement`, where
`Enablement { visible: boolean; enabled: boolean; reason: ResolvedLabel | null }`. It must not render,
block, touch a UI thread, or mutate — it is evaluated over an immutable snapshot.

REQ {#context-introspection}. The context is introspectable: enumerate which keys a `Ctx` carries; a
disabled action exposes its `reason` ("why disabled").

## 6. Invocation & capability — incl. the AI-peer surface {#invocation}

REQ {#invoke}. `invoke(addr, params, ctx, caller: CallerScope, signal?: AbortSignal): Promise<InvokeResult>`
is **the** way an action runs; a key press, a list click, an SE selection, and an AIUI call are all thin
callers. It validates params and capability first, then runs; it is cancellable via the `AbortSignal`.

REQ {#capabilities}. Each action declares a `Capability` ∈ {`Safe`, `Mutating`, `Dangerous`}; `invoke`
checks it against the caller's granted scope before running. No action bypasses the check.

REQ {#caller-scope}. A `CallerScope { identity, trust, granted: Set<Capability> }` flows into every
`invoke`. The scope is **granted by the host**, not asserted by the caller; the engine never trusts a
self-reported scope.

REQ {#prompt-on-dangerous}. For a non-trusted caller, a `Dangerous` action does not run on `invoke` alone —
it returns `ConfirmationRequired` and runs only after a separate `action://vibeterm/confirm?addr` from a
trusted surface.

REQ {#audit}. Every `invoke` of a `Mutating`/`Dangerous` action emits an audit record (address, caller,
outcome), observable on the model plane.

REQ {#scope-refuse}. Scope escalation is **REFUSE** — an error, never a warning. The engine refuses any
out-of-scope invoke.

## 7. i18n {#i18n}

REQ {#i18n-catalogue}. Presentation strings are `Msg { key, default_en }` where `key` is **derived from
the address** (`action.vibeterm/<name>.name` / `.description`) and `default_en` is the inline English
default. A `Catalogue { locale, entries, parent }` resolves through a parent chain terminating in `en`
seeded from the inline defaults, so a release lookup never misses.

REQ {#i18n-resolved}. A resolved label is `ResolvedLabel { value, original_en }`; Search Everywhere indexes
both, so a user typing the English name finds an action under any locale.

REQ {#i18n-fallback}. English is the default, mandatory-complete locale and the terminating fallback. The
active locale switches **reactively, at runtime, no reload** (a signal holding the active `Catalogue`).

REQ {#legibility-gate}. A CI/floor gate enumerates the registry and asserts every action's `name` and
`description` are present, non-empty, non-placeholder, and every derived key resolves in the shipped `en`
catalogue.

REQ {#ru-locale}. Russian ships from the start; the catalogue structure is binding and the locale set
grows freely.

## 8. The AI-UI surface {#aiui}

REQ {#aiui}. The headless AIUI offers four verbs against the same engine: `list_actions(filter?)`,
`invoke(addr, args)`, `state() -> ModelView` (PROP-047 §`#tree`), `search(query, tab?)`. It is a thin
adapter with a no-op render; it is the **reference** surface.

REQ {#plane-unification}. The four verbs fold onto the **model plane**; the legacy `--control` terminal
plane stays single-view (frozen, PROP-044 §8); CDP stays observation-only. There is one AIUI surface, not
two.

REQ {#cli-target-scope}. The `vibe aiui` CLI addresses both the shell (`invoke vibeterm/*`) and the hosted
`vibe tree` (`invoke vibe.tree/*`) through one verb set, scoped by target.

REQ {#evaluation-matrix}. The AI-UI is measured (not asserted) against the evaluation matrix
([findings §4](../../../research/vibeterm/vibeterm-ui-architecture-findings-v0.1.md#eval-matrix)); a task
the AI cannot do is a design obligation, not a silent gap.

## 9. Identity-grammar conformance {#conformance}

REQ {#conformance-grammar}. The address grammar, the `ModelView` schema, the AIUI verb set, the SE provider
contract, the i18n key scheme, and the capability lattice are an **identity-grammar spec**, a normative
document under `spec/modules/vibeterm/`, shared with the Rust `vibe-actions`. The grammar is provenance —
no crate imports the other.

REQ {#conformance-golden}. A fixture-driven **conformance golden** is run in CI on **both** the Rust and
the TS floors: a fixture registry, a fixture `Ctx` + expected `Enablement`, a fixture `invoke` sequence +
expected `ModelView` deltas, a fixture SE query + expected rankings. Drift fails the floor on the
offending side.

REQ {#conformance-ci}. The conformance golden is part of `self-check` (Rust) and the TS floor (`vitest`);
both must be green at every commit.

## 10. Discipline {#discipline}

REQ {#ai-native}. The engine follows the **AI-Native TypeScript** discipline: cells with single
registration points and no sibling-cell coupling; `scope!`/specmark-equivalent traceability to this PROP's
anchors; strict `tsconfig`; branded types; `Result` errors; `vitest` cell tests; green `tsc` + `specmap` at
every commit.

## 11. Non-goals (pre-MVP sketch) {#non-goals}

- A complete `vibe term` shell — only the action/AIUI core + the tab-create/select slice ship in the
  pre-MVP (PROP-044 §7 governs the rest).
- Per-tab AIUI over the wire — prototyped in-engine; the network/JSON-RPC binding is deferred.
- Non-English locale content beyond `ru` — the mechanism ships; locales grow freely.
- The full evaluation-matrix measurement — the matrix is drafted; measurement is a build-phase task.
