# PROP-039: the vibevm action system — `vibe-actions` {#root}

**Status:** DRAFT — requirements, 2026-07-15 (owner-commissioned). The **contract** for a new crate
`vibe-actions`: a frontend-agnostic, addressable, programmatically-drivable behaviour layer.
**Related:** design-doc [`spec/design/action-system.md`](../../design/action-system.md) (the *why* +
architecture); the clean-room study
[`spec/research/action-systems-vscode-idea.md`](../../research/action-systems-vscode-idea.md)
(obligations DO1–DO18, deltas Δ1–Δ16); [PROP-037](../vibe-cli/PROP-037-tree-tui.md) (the `vibe tree`
TUI — the first consumer, revised by Spec 2); the `qualified-naming` and `addressable-specs` flows
(the address discipline this extends to behaviour). Mandate + acceptance:
[`spec://vibevm/research/ACTION-SYSTEM-RESEARCH-PLAN#mandate`](../../research/ACTION-SYSTEM-RESEARCH-PLAN-v0.1.md#mandate).
**Language:** the shipped UI is English; a real i18n mechanism ships (§8) with English the default,
mandatory-complete locale.

This contract is deliberately **granular and addressable** (owner directive): every feature is its
own `{#anchor}` REQ cited by the code via `specmark`. A REQ is the unit of work. Each Δ from the
study maps to one or more REQs here.

---

## 1. Overview & the frontend-agnostic invariant {#overview}

REQ. `vibe-actions` is a standalone Rust crate that models **what a UI can do** as first-class,
addressable **Actions**, independent of any rendering technology. It is the behaviour-layer twin of
`spec://`: `spec://` addresses facts, `action://` addresses behaviour.

REQ {#no-render-dep}. The crate has **zero rendering dependencies** — it must not depend on
`ratatui`, `crossterm`, any GUI/DOM toolkit, or any terminal type. This invariant is what makes every
surface (the TUI, a future web UI, an IDE plugin) and the headless **AIUI** (§11.3) possible; it is
verified by a dependency-graph check in the crate's gate. Nothing in `core / model / controller`
logic may reference a rendering type.

## 2. Addressing {#addressing}

### 2.1 The address grammar {#address-grammar}

REQ. An action is named by an **`ActionAddr`** with the textual form
`action://<group>/<name>[?<params>]`, where `<group>` is a dotted namespace (e.g. `vibe.tree`,
`core`), `<name>` is a dotted/kebab identifier (e.g. `copy.markdown`, `search.everywhere`), and the
optional `<params>` is a `&`-separated `key=value` query carrying invocation parameters (§5).
`(group, name)` is the identity; the query is *not* part of identity. Examples:
`action://vibe.tree/copy.markdown`, `action://vibe.tree/sort?by=name&dir=asc`,
`action://core/search.everywhere`.

REQ {#address-parse}. `ActionAddr` round-trips: `parse(display(a)) == a` for the identity part, and
parsing rejects a malformed address with a typed error (never a panic).

### 2.2 Uniqueness, rename & alias {#address-uniqueness}

REQ. `(group, name)` is **globally unique** within a running registry (§4.1). A **rename** is a *new
identity*; the old address is retired as a **tombstone** or kept as an explicit **alias** that
resolves to the new one — an address that has ever been published is never silently repurposed
(the `qualified-naming` rule, applied to behaviour). Aliases are resolvable and enumerable.

## 3. The Action value {#action-value}

### 3.1 Fields {#action-fields}

REQ. An **`Action`** carries exactly: its `ActionAddr`; a **`Presentation`** (§3.3); a **`ParamSchema`**
(§5, possibly empty); an **enablement** predicate (§6.2); an **`invoke`** (§7.1); a **`Capability`**
(§7.2); and **`SearchMeta`** (§10.4 — synonyms/aliases/abbreviations/keywords). No other state.

### 3.2 The immutable resolved snapshot {#action-snapshot}

REQ. What a surface renders or an AI observes for an action is an **immutable resolved snapshot**
(address + resolved presentation + resolved enablement for a given context), never a live mutable
object. Change is delivered by re-resolution, not mutation (avoids the incumbents' live-mutable
`Presentation`/`Action` hazards).

### 3.3 Presentation & the mandatory human-legibility discipline {#presentation}

REQ. `Presentation` carries a **name** and a **description**, plus an optional icon/glyph and
optional category. Both name and description are **mandatory, non-empty, meaningful** localizable
messages (§8) — an action with an empty or placeholder name/description is a **contract violation**
caught by the legibility gate (§8.4). Both are **first-class searchable fields** (§10 — the fallback
match lane). This is the owner's founding discipline (DO5): filling them is part of building any UI
on this system, not optional decoration.

## 4. The registry {#registry}

### 4.1 Registration & collision {#registry-collision}

REQ. Actions register into an enumerable **`Registry`**. Registering a `(group, name)` already present
is a **hard, deterministic error** (not a silent override, not a log-and-drop) — a *collision* in the
`qualified-naming` sense, distinct from a *conflict*. Registration returns a typed result; the error
names both the incumbent and the newcomer.

REQ {#registry-override}. Layered override, if ever needed, is an **explicit** operation
(`override_of(addr)`) with uniform semantics and is itself collision-checked — never an accidental
consequence of registration order.

### 4.2 Referential integrity {#registry-integrity}

REQ. Any reference to an action **by address** from another structure (a keymap binding §9, a menu
placement, a Search-Everywhere action provider §10.4) is **validated at registration/build time** —
a reference to an unregistered address **fails loud** then, never silently at click time (fixes the
incumbents' string-reference-with-no-check gap, DO2).

### 4.3 Enumeration {#registry-enumeration}

REQ. The registry is **fully enumerable** — every registered action (and alias) is reachable by
iteration. This backs the enumerable-registry golden (§12.2), Search Everywhere's action provider
(§10.4), and the AIUI `list_actions` (§11.3).

## 5. Parameters {#parameters}

### 5.1 The schema {#param-schema}

REQ. An action declares a **typed, serialisable, named-parameter schema** (`ParamSchema`): each
parameter has a name, a type, optionality, and an optional default. Parameters are **named**, not
positional. An action with no inputs declares an empty schema. Simple parameters are expressible in
the address query (§2.1); richer ones are passed as structured `ParamValues`.

### 5.2 Validation {#param-validation}

REQ. `ParamValues` are **validated against the schema at invoke time** (§7.1): a type mismatch, a
missing required parameter, or an unknown parameter is a typed error before the action body runs
(fixes the incumbents' `unknown[]` / phantom-`DataKey` gap, DO3).

## 6. Context & enablement {#context}

### 6.1 The typed context snapshot {#context-snapshot}

REQ. Enablement and invocation read a **typed context snapshot** — a `TypeId`-keyed typemap (`Ctx`)
holding strongly-typed values a surface has published (e.g. the current selection, the active mode).
Reading a key yields `Option<&T>` with the real type `T` — no stringly keys, no unchecked casts, no
nullable-`Object` (fixes VSCode's stringly `when` and IntelliJ's phantom `DataKey`, DO4).

### 6.2 The enablement predicate {#enablement}

REQ. An action's enablement is a **pure, fast function** `Fn(&Ctx) -> Enablement`, where
`Enablement { visible: bool, enabled: bool, reason: Option<Localized> }`. It **must not** render,
block, touch a UI thread, or mutate — it is evaluated over an immutable snapshot, so there is no
EDT/BGT hazard (the class of bug that is IntelliJ's biggest pain). Visibility (hide) and enablement
(grey-out) are the two independent axes (kept from VSCode's `when` vs `precondition`).

### 6.3 Introspection {#context-introspection}

REQ. The context is **introspectable**: a surface (and the AIUI) can enumerate which keys a `Ctx`
carries, and a disabled action exposes its **`reason`** ("why disabled") — neither is possible in the
incumbents. This backs both the legibility of the UI and the AIUI.

## 7. Invocation {#invocation}

### 7.1 `invoke` — the primary interface {#invoke}

REQ. `invoke(addr, params, ctx) -> Future<InvokeResult>` is **the** way an action runs; a key press,
a menu click, a Search-Everywhere selection, and an AIUI call are all thin callers of it. It is
**async**, returns a **typed result/error** (`InvokeResult`), validates params (§5.2) and the
capability (§7.2) first, and is **cancellable** (a cancellation token threads through). The result is
first-class (not recovered out-of-band as in IntelliJ).

### 7.2 The capability scope {#capabilities}

REQ. Each action declares a **`Capability`** (e.g. `Safe`, `Mutating`, `Dangerous`), and `invoke`
checks it against the caller's granted scope before running. This is inert for the trusted local TUI
but is the seam a networked surface (a future web UI) or an AIUI needs to refuse an out-of-scope
action (DO14). No action bypasses the check.

## 8. i18n {#i18n}

### 8.1 The address-keyed catalogue {#i18n-catalogue}

REQ. Presentation strings are **`Msg { key, default_en }`** where `key` is **derived from the address**
(`action.<group>/<name>.name`, `.description`) — one canonical mapping, no second key namespace to
drift (IntelliJ's strength) — and `default_en` is the **inline English** default carried at the
declaration site (VSCode's strength: self-documenting, always present). A `Catalogue { locale,
entries, parent }` resolves a key through a parent chain terminating in an `en` catalogue **seeded
from the inline defaults**, so a release lookup can never miss (no sentinel, no panic). The on-disk
catalogue format is Fluent (`fluent`-family), with named args + plurals.

### 8.2 The resolved label keeps the original {#i18n-resolved}

REQ. A resolved label is **`ResolvedLabel { value, original_en }`** — the localized value plus the
English original. Search Everywhere (§10) indexes **both**, so a user typing the English name finds
an action under any locale (copies VSCode `localize2`).

### 8.3 English default & locale swap {#i18n-fallback}

REQ. English is the **default, mandatory-complete** locale and the terminating fallback. Other locales
may lag and fall back silently. Locale switch is atomic (`ArcSwap<Catalogue>`); a package may ship
`locales/<lang>.ftl`, and a dedicated language-pack package may override, merged by explicit priority
(language-pack > package locale > inline English).

### 8.4 The human-legibility gate {#legibility-gate}

REQ (floor gate). A CI/floor gate enumerates the registry (§4.3) and asserts, **against the English
surface**, that every action's `name` and `description` are present, non-empty, and non-placeholder,
and that every derived `MessageKey` resolves in the shipped `en` catalogue. A violation fails the
floor, exactly as untested domain logic does. Other locales are not gated (they may lag). A `pseudo`
locale QA build surfaces un-externalized strings.

## 9. Keymap {#keymap}

### 9.1 Bindings {#keymap-bindings}

REQ. A **`Keymap`** maps a key (or chord) to a `(ActionAddr, ParamValues)` binding, gated by an
optional enablement/`when`-equivalent over the typed context. Bindings reference actions **by address**
and are subject to referential-integrity (§4.2).

### 9.2 The pure resolver {#keymap-resolver}

REQ. Resolution is a **pure function** returning a **3-state result** — `NoMatch | NeedMoreChords |
Found(addr, params)` (copies VSCode's `ResultKind`). Ambiguity (several bindings on one key) is
resolved by enablement + weight (IntelliJ's first-enabled model). **Chord timers, IME, and focus
walking live in the surface adapter**, never in the resolver.

### 9.3 Conflicts are surfaced {#keymap-conflicts}

REQ. A binding conflict (two enabled bindings competing for one key/chord) is **surfaced** via an
introspection API — not resolved silently (fixes the incumbents' silent/advisory-only handling, DO11).

## 10. Search Everywhere {#search-everywhere}

The acceptance feature: one window that searches packages, every package-card field, and all actions,
with a hybrid "All" tab + per-category tabs, invoking a found action in place.

### 10.1 The provider trait {#se-provider}

REQ. A searchable source is a **`SearchProvider`** implementing the **two-phase** contract:
`enumerate(query) -> stream of cheap keys` (streamed, cancellable, scope-aware — never a full
materialization) and `resolve(key) -> hits` (heavy items, produced **only for matched keys**). It also
supplies identity + tab presentation (`id`, `group_name`, `sort_weight`, `separate_tab`), an
`ItemAccessor` (`{label, description, key}` for the one shared ranker), an `on_selected(hit, mods) ->
Close | Stay` (navigate/act), and a `render_row(hit) -> RowDescriptor` (§10.5). This is the minimal
"searchable structured universe" contract distilled from IntelliJ; it is open enough that new sources
plug in without touching the engine.

### 10.2 The engine {#se-engine}

REQ. The engine, on each keystroke: **debounces** (~90–120 ms) and **cancels** the prior run; selects
the active provider set (one for a category tab, all filter-enabled for "All") with a per-provider cap
(single 30 / All 15); has each provider **enumerate** keys, **matches** them with one scorer, and
**resolves** only survivors. Hits are ranked on **one commensurable scale** (§10.3), **recency-weighted**
with an **exact-match floor**, **deduped keeping the higher score** across providers, and drained
**round-robin** from per-provider bounded queues into one flat list (the single-threaded adaptation of
IntelliJ's back-pressured merge). A per-provider **"more…"** row re-queries that provider and **freezes**
the rows above (no reshuffle under the cursor).

### 10.3 Matching, ranking & the fallback lane {#se-ranking}

REQ. One scorer produces **both a score and the highlight ranges** (fixes VSCode's two-engine
mismatch, DO7). The match-tier ladder is: exact → prefix → CamelCase/subsequence → substring →
**word-in-name/description** (the fallback lane — the owner's "search by name and description when
other criteria don't match"). Ordering is score DESC, tie → provider `sort_weight` DESC. Providers
must emit scores on the **one shared scale** (the make-or-break hybrid-list rule).

### 10.4 Providers at ship + reserved {#se-providers}

REQ. Three providers ship: **`PackageProvider`** (keys = package FQNs from the `PackageTree`; navigate
= reveal the node); **`PackageFieldProvider`** (keys = **every field of every package detail card** —
name, version, kind, license, load-type, origin, path, deps, diagnostics…; navigate = open the card on
that field — the owner's "search inside all card fields"); **`ActionProvider`** (keys = action address
+ name + description + `SearchMeta` synonyms/aliases/abbreviations; `on_selected` = **invoke** the
action — perform→`Close`, a toggle→`Stay`; disabled actions render greyed with their `reason` and
right-aligned keybinding). A **`StructureProvider`** (AI-Native specmap spec/code nodes) is **reserved
against the same trait** and added later **without engine changes** (DO16).

### 10.5 The normalized renderer {#se-renderer}

REQ. All result rows render through **one** normalized `RowDescriptor { icon?, primary, secondary
(e.g. a keybinding), group, enabled, kind }`, so every category looks uniform (the study's ADAPT note;
avoids IntelliJ's per-provider renderers).

### 10.6 Tabs {#se-tabs}

REQ. Tabs are built **from the providers**: sort by `sort_weight`; prepend an **"All"** tab when there
is more than one provider; one tab per `separate_tab` provider. The "All" tab searches every
filter-enabled provider and carries a **category checkbox filter**; a single tab restricts to its
provider and hides group headers. `Tab`/`Shift-Tab` cycle. This is the IntelliJ hybrid-All + per-tab
UX the owner named.

## 11. Surfaces & the AIUI {#surfaces}

### 11.1 The Surface trait {#surface-trait}

REQ. A **`Surface`** is the adapter seam between the core and a concrete frontend: it `present`s a
`ModelView` and yields `Event`s. A visual surface renders; a headless surface's `present` is a no-op.
No core / model / controller code depends on a `Surface` implementation or a rendering type (§1
invariant).

### 11.2 The serialisable model view {#model-view}

REQ. The observable UI state is a **serialisable `ModelView`** snapshot (focus, open modals, visible
rows, current tree/selection, the active tab, the set of enabled actions with their addresses +
reasons). It is a pure projection of the Model and carries **no rendering types** — so an AI reads
structured state, never pixels.

### 11.3 The headless AIUI surface {#aiui}

REQ (designed-for; **not built now**). The core supports a **headless AIUI** surface offering:
`list_actions(filter?)` (enumerate the registry with live enablement + reasons + params),
`invoke(addr, args)` (the same `invoke` as §7.1), `state() -> ModelView` (§11.2), and
`search(query, tab?)` (drive §10 programmatically). Because enablement is pure + introspectable, the
model is serialisable, and invocation is address-based, this surface is a thin adapter with a no-op
`present`. The architecture must keep it a thin adapter — it is **prototyped on the TUI**, and a
future in-process API / JSON-RPC / MCP binding realises it. This is the founding AIUI goal (DO18): the
headless surface is the **reference**; visual surfaces are projections.

## 12. Discipline & gates {#discipline}

### 12.1 AI-Native Rust {#ai-native}

REQ. The crate follows the AI-Native Rust discipline (`spec://org.vibevm.ai-native/core-ai-native`):
every module `scope!`s its governing REQ anchor from this PROP; a REQ is the unit of work; the
`conform` baseline for the crate is empty (zero slack).

### 12.2 The floor gates {#gates}

REQ. Two gates beyond the standard floor: the **human-legibility gate** (§8.4) and an
**enumerable-registry golden** that asserts every registered action resolves, has a valid address,
carries mandatory presentation, and is reachable by enumeration. Both are part of `self-check`.

## 13. Non-goals {#non-goals}

- **The AIUI surface itself is not built now** (§11.3) — the architecture is designed for it; the TUI
  is the prototype.
- **Web / VSCode / JetBrains / Zed surfaces are not built now** — the crate is designed so they are
  additional `Surface` adapters.
- **An ML reranker for Search Everywhere is deferred** — the ranking (§10.3) leaves the exact-match
  floor as the slot an ML pass would sit above.
- **Localization content (non-English catalogues) is not shipped now** — the mechanism ships; English
  is the only mandatory-complete locale (§8.3).

## 14. Decision records {#decisions}

The load-bearing decisions (four-field: Decision · Why · Rejected · Revisit) are recorded at their
governing anchors above and narrated in the design-doc
[`spec/design/action-system.md#decisions`](../../design/action-system.md#decisions) (D1–D10). Summary
of the bindings: URI address `action://` (§2.1, D1); collision-erroring registry (§4.1, D2); typed
context + pure enablement (§6, D3); programmatic-invocation-primary + the AIUI reference surface (§7,
§11.3, D4); the two-phase provider Search Everywhere (§10, D5); address-keyed i18n with English
inline + `{value, original_en}` (§8, D6); the English-only legibility gate (§8.4, D7); one normalized
renderer (§10.5, D8); the pure 3-state keymap resolver (§9.2, D9); one commensurable recency-weighted
ranker (§10.3, D10). All owner review points RP1–RP5 are resolved
(`spec://vibevm/research/ACTION-SYSTEM-RESEARCH-PLAN#review-points`).
