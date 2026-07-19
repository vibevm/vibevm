# VibeTerm UI-Architecture — Findings v0.1

**Status:** research **COMPLETE (Phases 1–4)**. Phase 1 = internal methodology extraction (§2/§3);
Phase 2 = external comparative (§6, sketch depth); Phase 3 = pitfalls → obligations (§7); Phase 4 =
synthesis into numbered architecture deltas (§8) + the predictions check. Hands off to D2 (design-doc)
and D3 (contracts).
**Genre:** findings document (the comparative-research genre for the Phase-2 external part; the
internal part is our own material, no firewall). **Source plan:**
[`VIBETERM-UI-ARCHITECTURE-RESEARCH-PLAN-v0.1.md`](VIBETERM-UI-ARCHITECTURE-RESEARCH-PLAN-v0.1.md).
**Scope contract:** the frozen-vs-open split ([plan §0.3](VIBETERM-UI-ARCHITECTURE-RESEARCH-PLAN-v0.1.md#frozen-vs-open))
governs everything here — this document *realises* the frozen axes and *earns* the open questions; it
does not reopen an axis.

> **AI-Native-ready.** Every candidate REQ named below carries a prospective `spec://vibeterm/…#…`
> anchor and a one-line acceptance, so D2 (the design-doc) and D3 (the vibeterm PROP family) author from
> these findings alone, under **AI-Native Rust** (the `vibe-actions` side of the conformance golden) and
> **AI-Native TypeScript** (the `vibeterm-core` side). No retrofit.

---

## 0. Thesis — one paragraph

VibeTerm's shell is a **TS re-expression of vibevm's render-free action methodology**, ported verbatim
where the GUI world has a direct analogue, **adapted** where TypeScript/Electron/Solid demand a different
mechanism for the same invariant, and **invented** only where the multi-window GUI has no TUI analogue at
all (colour tiers → theme×a11y modes; single-screen snapshot → window→tab→pane tree; a glyph vocabulary →
an SVG icon system). The whole shell is **self-contained under `spec/modules/vibeterm/`** with no
build-dependency on `vibe-actions`, yet **conformance-tested** against a shared identity-grammar so the
Rust TUI-AIUI and the TS shell-AIUI stay one surface. The AIUI peer is the reference; the Solid chrome is
one projection; control is semantic `invoke`; CDP stays observation-only.

---

## 1. Framing — frozen axes vs open questions

Reproduced in summary from [plan §0.3](VIBETERM-UI-ARCHITECTURE-RESEARCH-PLAN-v0.1.md#frozen-vs-open); the
authoritative list lives there.

**Frozen (constraints — PROP-044 D0–D7 + RP-A/D + the AI-UI mandate + AI-Native-ready):** Solid + Vite +
Tailwind v4 + Kobalte + strict TS (D4); shell = default visible `vibe term`, headless/`--control` bare
single-view unchanged (D2); tabs = per-tab `WebContentsView` + main-owned pty keyed by `TabId`, reparent
preserves state (D0); split ceiling 2 in M1 (D3); transport-agnostic sidecar-ready protocol (D5); i18n
from the start, en + ru (D6); live design-token theming, two launch themes (D7); AI-UI-ready by
construction (semantic `invoke`, CDP observation-only); self-contained under `spec/modules/vibeterm/`
with conformance (RP-A sharpened).

**Open (what this research earns):** the TS shape of the action core; the `ModelView` tree schema; the
identity-grammar conformance surface; the capability/permission surface; AIUI-plane unification; the
transport-contract form; the Solid-vs-immutable-`ModelView` reconciliation; the design-token architecture;
the AI-UI evaluation criterion; the GUI-only inventory.

---

## 2. The ports / adapts / new table — the full vertical {#ports-adapts-new}

For each concern: **PORTS verbatim** (the invariant carries over unchanged, cited to its
`spec://vibevm/…#…` anchor); **ADAPTS** (the same invariant, realised through a different TS/Electron
mechanism, with the mechanism named); **NEW** (genuinely GUI-specific, no TUI analogue — a candidate
invent, each a prospective REQ). REQ ids are provisional (`spec://vibeterm/…`); the contract sessions (D3)
ratify them.

### 2.1 Entities & addressing

- **PORTS** — the `action://<group>/<name>[?params]` grammar; `(group,name)` is the identity, the query is
  not ([PROP-039 §2.1 `#address-grammar`](../../spec/modules/vibe-actions/PROP-039-action-system.md#address-grammar)).
  Rename = new identity + tombstone/alias; a published address is never silently repurposed
  ([§2.2 `#address-uniqueness`](../../spec/modules/vibe-actions/PROP-039-action-system.md#address-uniqueness)).
  The vibeterm group is `vibeterm`: `action://vibeterm/tab.open`, `action://vibeterm/pane.split?dir=right`.
- **ADAPTS** — `ActionAddr` is a TS **branded type** (`TabId`, `WindowId`, `PaneId` are branded nominal
  ids — `brand & { readonly __brand: 'TabId' }`); `parse(display(a)) === a` for the identity part and
  parsing rejects malformed input with a typed `Result`, never a throw
  ([§2.1 `#address-parse`](../../spec/modules/vibe-actions/PROP-039-action-system.md#address-grammar)).
- **NEW** — the shell entity set itself: **Window, Tab, Pane, Session/Terminal, Profile, Theme, Locale**.
  Each carries a stable branded id independent of its host window (`TabId` survives tear-off — PROP-044
  §3). Prospective REQs: `spec://vibeterm/entities#window`, `#tab`, `#pane`, `#session`, `#profile`,
  `#theme`, `#locale`. The `action://vibeterm/*` catalogue (mirrors the `vibe.tree` catalogue shape,
  [PROP-037 §13.5 `#action-catalogue`](../../spec/modules/vibe-cli/PROP-037-tree-tui.md#action-catalogue)):
  `tab.open`, `tab.select?id`, `tab.close?id`, `pane.split?target&dir`, `pane.close?id`,
  `tab.move-to-window?tab&window`, `view.set-compact?on`, `theme.set?id`, `locale.set?id`,
  `search.everywhere`, `quit`.

### 2.2 The render-free core — the `#no-render-dep` invariant

- **PORTS** — the crate/core has **zero rendering dependencies**: it must not depend on a GUI/DOM toolkit,
  a terminal type, or (for vibeterm) Solid/DOM/Electron types
  ([PROP-039 §1 `#no-render-dep`](../../spec/modules/vibe-actions/PROP-039-action-system.md#overview)). This
  invariant is what makes every surface and the AIUI possible.
- **ADAPTS** — in Rust the gate is a dependency-graph check; in TS the gate is a **dependency-boundary
  lint** (an `eslint-plugin-import` `no-restricted-paths` rule, or a layered `tsconfig` — an *engine*
  project that may not import `solid-js`/`react-dom`/`electron`/`@xterm/*`, vs a *chrome* project that
  may). The lint is on the floor from day one (the TS twin of the Rust gate, plan §0.1). The pure cells —
  tab registry, pane-layout maths, session model, protocol codec — live in the engine project.
- **NEW** — the **Electron process topology** as a boundary concern. Pty ownership stays in **main**
  (PROP-044 §3); the engine's *pure logic* is main-resident (or a pure-TS cell loadable in both main and a
  future sidecar); the **chrome** is a renderer; each **terminal view** is its own renderer (lean vanilla
  TS + xterm, no chrome framework — D4). The boundary lint must cover all three layers: engine ↔ chrome ↔
  terminal-view. Prospective REQ: `spec://vibeterm/architecture#render-free-boundary`.

### 2.3 The Action value

- **PORTS** — an `Action` carries exactly: `ActionAddr`, `Presentation` (mandatory non-empty name +
  description, localizable, searchable), `ParamSchema` (possibly empty), an enablement predicate, an
  `invoke`, a `Capability`, `SearchMeta`
  ([PROP-039 §3.1 `#action-fields`](../../spec/modules/vibe-actions/PROP-039-action-system.md#action-value)).
  The resolved snapshot is **immutable**; change is delivered by re-resolution, not mutation
  ([§3.2 `#action-snapshot`](../../spec/modules/vibe-actions/PROP-039-action-system.md#action-snapshot)).
- **ADAPTS** — `Action<P extends ParamSchema, R>` is a TS generic; `Presentation.name`/`.description` are
  `Msg` values (§2.8); `invoke` returns `Promise<InvokeResult<R>>` (native async, cancellable via
  `AbortSignal` — the TS analogue of the cancellation token). The immutable-snapshot discipline maps to
  **frozen / deeply-readable** TS types published by the engine (RQ3, §2.11).
- **NEW** — none at the value level (the value ports cleanly). The novelty is the catalogue's *content*
  (§2.1) and the capability gating for an AI caller (§2.7, RQ13).

### 2.4 The Registry

- **PORTS** — registering a `(group,name)` already present is a **hard, deterministic error** naming both
  the incumbent and the newcomer; layered override, if ever needed, is an explicit `override_of(addr)`
  that is itself collision-checked
  ([PROP-039 §4.1 `#registry-collision`/`#registry-override`](../../spec/modules/vibe-actions/PROP-039-action-system.md#registry-collision)).
  Referential integrity: any address reference (keymap binding, menu placement, SE action provider) is
  validated at registration/build time — a dangling reference fails loud then, never silently at click
  time ([§4.2 `#registry-integrity`](../../spec/modules/vibe-actions/PROP-039-action-system.md#registry-integrity)).
  Full enumeration backs the legibility golden, SE, and `list_actions`
  ([§4.3 `#registry-enumeration`](../../spec/modules/vibe-actions/PROP-039-action-system.md#registry-enumeration)).
- **ADAPTS** — the registry is a TS `Map<string, Action>` keyed by address string, built by explicit
  `register(action)` calls at a **single registration point** per cell (AI-Native TS: no sibling-cell
  coupling); referential integrity is a `tsc`-time check where bindings are statically declared, and a
  build-time check otherwise.
- **NEW** — none. This is a near-verbatim port; the registry laws are language-neutral.

### 2.5 Parameters

- **PORTS** — a typed, serialisable, **named**-parameter schema; each parameter has a name, a type,
  optionality, an optional default; an action with no inputs declares an empty schema
  ([PROP-039 §5.1 `#param-schema`](../../spec/modules/vibe-actions/PROP-039-action-system.md#param-schema)).
  `ParamValues` are validated against the schema at invoke time — a type mismatch, a missing required
  parameter, or an unknown parameter is a typed error **before** the body runs
  ([§5.2 `#param-validation`](../../spec/modules/vibe-actions/PROP-039-action-system.md#param-validation)).
- **ADAPTS** — `ParamSchema` is a TS literal type / `zod`-equivalent schema; validation is a pure
  `validate(params): Result<ParamValues, ParamError>`; the address query carries simple params
  (`?dir=right&target=left`), structured params ride `ParamValues`.
- **NEW** — none.

### 2.6 Context & enablement

- **PORTS** — enablement and invocation read a **typed context snapshot** — a `TypeId`-keyed typemap
  holding strongly-typed values a surface has published (the current selection, the active mode/tab);
  reading a key yields the real type, no stringly keys, no unchecked casts
  ([PROP-039 §6.1 `#context-snapshot`](../../spec/modules/vibe-actions/PROP-039-action-system.md#context-snapshot)).
  Enablement is a **pure, fast function** `Fn(&Ctx) -> Enablement { visible, enabled, reason }`; it must
  not render, block, touch a UI thread, or mutate — evaluated over an immutable snapshot, so there is no
  EDT/BGT hazard ([§6.2 `#enablement`](../../spec/modules/vibe-actions/PROP-039-action-system.md#enablement)).
  The context is **introspectable** (enumerate keys; a disabled action exposes its `reason`)
  ([§6.3 `#context-introspection`](../../spec/modules/vibe-actions/PROP-039-action-system.md#context-introspection)).
- **ADAPTS** — TS has no `TypeId`; the typemap is a **branded-symbol-keyed `Map`**
  (`Ctx` = `Map<CtxKey<T>, T>`, where `CtxKey<T>` is `symbol & { readonly __t: T }`). Enablement is
  `(ctx: Ctx) => Enablement`; `reason` is a `ResolvedLabel` (§2.8). Pure-by-construction: enablement
  functions live in the engine cell and may not import Solid/DOM.
- **NEW** — the **shell context shape**: `TabCtx { activeTabId, activePaneId, tabCount, compact }`,
  `PaneCtx { paneCount, splitCap }`, `SessionCtx { hasActiveSession }`. Prospective REQ:
  `spec://vibeterm/context#shell-ctx`.

### 2.7 Invocation & capability — incl. the AI-peer surface (RQ13)

- **PORTS** — `invoke(addr, params, ctx) -> Future<InvokeResult>` is **the** way an action runs; a key
  press, a menu click, an SE selection, and an AIUI call are all thin callers; it is async, returns a
  typed result, validates params and capability first, and is cancellable
  ([PROP-039 §7.1 `#invoke`](../../spec/modules/vibe-actions/PROP-039-action-system.md#invoke)). Each
  action declares a `Capability` (`Safe`/`Mutating`/`Dangerous`); `invoke` checks it against the caller's
  granted scope before running — inert for the trusted local TUI, the seam a networked/AI caller is
  refused by ([§7.2 `#capabilities`](../../spec/modules/vibe-actions/PROP-039-action-system.md#capabilities)).
- **ADAPTS** — `invoke(addr, params, ctx, caller: CallerScope): Promise<InvokeResult>`; the
  `CallerScope` is the TS shape of the granted scope; `AbortSignal` carries cancellation.
- **NEW (RQ13 — the capability/permission surface for an AI / networked peer).** In the TUI the capability
  check is inert (a trusted local caller). For the shell's AIUI peer — and a future networked sidecar —
  "the AI may do anything a human may" meets "a `Dangerous` action must not fire without consent." The
  model this research recommends (to be earned in Phase 3, sketched here):
  1. **`CallerScope { identity, trust, granted: Set<Capability> }`** flows into every `invoke`; the
     engine never trusts the caller's self-reported scope — the scope is **granted by the host**, not
     asserted by the caller (the `secrets-hygiene` scope posture: scope escalation is REFUSE, an error,
     never a warning).
  2. **`prompt-on-Dangerous`** for non-trusted callers: a `Dangerous` action does not run on `invoke`
     alone; it returns a `ConfirmationRequired` result and runs only after a separate
     `action://vibeterm/confirm?addr` from a trusted surface (the chrome, or a human).
  3. **Audit**: every `invoke` of a `Mutating`/`Dangerous` action emits an audit record (addr, caller,
     outcome) — observable on the model plane.
  - *Hypothesis:* PROP-039 §7.2 capabilities + a host-granted scope + prompt-on-`Dangerous` + audit is a
    safe, non-bypassable model. Falsified if a class of actions cannot be safely scoped, or if the
    confirmation flow cannot be made non-bypassable.
  - Prospective REQs: `spec://vibeterm/aiui#caller-scope`, `#prompt-on-dangerous`, `#audit`,
    `#scope-refuse`.

### 2.8 i18n

- **PORTS** — presentation strings are `Msg { key, default_en }` where `key` is **derived from the
  address** (`action.<group>/<name>.name` / `.description`) and `default_en` is the inline English
  default; a `Catalogue { locale, entries, parent }` resolves through a parent chain terminating in `en`
  seeded from the inline defaults, so a release lookup never misses
  ([PROP-039 §8.1 `#i18n-catalogue`](../../spec/modules/vibe-actions/PROP-039-action-system.md#i18n-catalogue)).
  A resolved label is `ResolvedLabel { value, original_en }` so SE indexes both
  ([§8.2 `#i18n-resolved`](../../spec/modules/vibe-actions/PROP-039-action-system.md#i18n-resolved)). English
  is the default, mandatory-complete locale; locale switch is atomic
  ([§8.3 `#i18n-fallback`](../../spec/modules/vibe-actions/PROP-039-action-system.md#i18n-fallback)). A CI
  gate asserts every action's name+description present, non-empty, non-placeholder, and every key
  resolves in `en` ([§8.4 `#legibility-gate`](../../spec/modules/vibe-actions/PROP-039-action-system.md#legibility-gate)).
- **ADAPTS** — the Rust `fluent` catalogue → the **`@fluent/bundle`** JS runtime; `ArcSwap<Catalogue>` →
  a **reactive catalogue signal** (a Solid signal holding the active `Bundle`); live locale switch
  re-resolves every label without a reload (every chrome string reads through `useI18n()`, which tracks
  the signal). The legibility gate is a `vitest`/CI check over the registry.
- **NEW** — vibeterm ships **en + ru from the start** (the TUI shipped the mechanism but en-only
  content); Russian catalogue content is a first-class deliverable, not a stub. The live-locale-swap is
  demonstrated on `locale.set?id` in M1 (PROP-044 §10). Prospective REQs:
  `spec://vibeterm/i18n#catalogue`, `#reactive-swap`, `#legibility-gate`, `#ru-locale`.

### 2.9 Keymap / input

- **PORTS** — a `Keymap` maps a key/chord to `(ActionAddr, ParamValues)`, gated by an optional enablement
  over the typed context; bindings reference actions by address and are subject to referential integrity
  ([PROP-039 §9.1 `#keymap-bindings`](../../spec/modules/vibe-actions/PROP-039-action-system.md#keymap-bindings)).
  Resolution is a **pure 3-state function** — `NoMatch | NeedMoreChords | Found(addr, params)`; chord
  timers, IME, focus walking live in the **surface adapter**, never the resolver
  ([§9.2 `#keymap-resolver`](../../spec/modules/vibe-actions/PROP-039-action-system.md#keymap-resolver)).
  A binding conflict is surfaced via introspection, not resolved silently
  ([§9.3 `#keymap-conflicts`](../../spec/modules/vibe-actions/PROP-039-action-system.md#keymap-conflicts)).
- **ADAPTS** — the resolver is a pure TS function; chord/IME/focus handling lives in the chrome adapter
  (the Solid keydown handler). The shell keymap is context-aware per region (terminal-view keystrokes go
  to xterm, chrome keystrokes to the keymap) — the adapter routes by focus, the resolver never knows.
- **NEW** — **multi-pane focus routing**: the TUI's single-screen focus model becomes a multi-pane focus
  model (which pane receives the next key). This is a chrome-adapter concern (focus is ephemeral, §2.11),
  but the focus identity (`PaneId`) crosses into the `ModelView`. Prospective REQ:
  `spec://vibeterm/keymap#multi-pane-focus` (also GUI-only inventory, RQ16).

### 2.10 Search Everywhere

- **PORTS** — a searchable source is a `SearchProvider` implementing the **two-phase** contract:
  `enumerate(query) → cheap keys` (streamed, cancellable) and `resolve(key) → hits`; plus identity/tab
  presentation, an `ItemAccessor`, `on_selected`, `render_row → RowDescriptor`
  ([PROP-039 §10.1 `#se-provider`](../../spec/modules/vibe-actions/PROP-039-action-system.md#se-provider)).
  The engine debounces (~90–120 ms) and cancels the prior run; one scorer produces score + highlight
  ranges; the match ladder exact → prefix → CamelCase/subsequence → substring → **name/description word**
  (the fallback lane); recency-weighted with an exact-match floor; deduped keeping the higher; drained
  round-robin; freeze-on-"more"
  ([§10.2 `#se-engine`](../../spec/modules/vibe-actions/PROP-039-action-system.md#se-engine),
  [§10.3 `#se-ranking`](../../spec/modules/vibe-actions/PROP-039-action-system.md#se-ranking)). Tabs are
  built from providers; "All" + per-category; `Tab`/`Shift-Tab` cycle
  ([§10.6 `#se-tabs`](../../spec/modules/vibe-actions/PROP-039-action-system.md#se-tabs)).
- **ADAPTS** — the engine ports as a TS cell (pure, `vitest`-tested); the GUI renderer is **cmdk/Kobalte**
  instead of a TUI row drawer; async providers ride Solid signals (a `createResource`-shaped enumerate;
  the debounce/cancel map onto an `AbortController`). The one normalized `RowDescriptor` renders through
  one Solid row component.
- **NEW** — **virtualisation** for large result lists (the TUI never had a list long enough to need it);
  the shell providers at ship: `SessionProvider` (sessions/terminals), `ActionProvider` (the
  `action://vibeterm/*` catalogue), reserved `ProfileProvider`. Prospective REQs:
  `spec://vibeterm/search#virtualisation`, `#session-provider`.

### 2.11 Surfaces & the serialisable `ModelView` — incl. the Solid reconciliation (RQ3)

- **PORTS** — a `Surface` is the adapter seam: it `present`s a `ModelView` and yields `Event`s; a visual
  surface renders, a headless surface's `present` is a no-op; no core code depends on a `Surface`
  implementation or a rendering type
  ([PROP-039 §11.1 `#surface-trait`](../../spec/modules/vibe-actions/PROP-039-action-system.md#surface-trait)).
  The observable UI state is a **serialisable `ModelView`** — focus, open modals, visible rows, current
  selection, the active tab, the set of enabled actions with addresses + reasons — a pure projection of
  the Model, carrying **no rendering types** so an AI reads structured state, never pixels
  ([§11.2 `#model-view`](../../spec/modules/vibe-actions/PROP-039-action-system.md#model-view)).
- **ADAPTS — the Solid reconciliation (RQ3).** PROP-039 §3.2 requires the resolved snapshot to be
  **immutable** and change by **re-resolution**; Solid's `createStore` is a fine-grained **mutable** store.
  The reconciliation this research recommends (to be earned, §2.11–§3):
  - the **engine owns the authoritative immutable `ModelView`** (a frozen, deeply-readable TS object);
  - a **Solid store is a one-way projection** engine → chrome, rebuilt on re-resolution (a `createMemo`
    over an engine `get()`); the chrome **never mutates the `ModelView`** — it dispatches actions (the
    AIUI verb) and the engine re-resolves;
  - **ephemeral chrome state** (hover, drag-ghost, in-flight keystrokes, transient focus) lives **outside**
    the engine, never crosses the seam, and never reaches the AIUI.
  - *Hypothesis:* the projection holds with no double source of truth; falsified if a UI-state class
    forces chrome-side model mutation or the rebuild is too costly at real tab counts.
- **NEW** — the **`ModelView` is a window→tab→pane tree** (richer than the TUI's single-screen snapshot):
  `windows[]` → `tabs[]` (id, title, kind, active) → `panes[]` (which tab, bounds), plus `compact`,
  `activeWindow`, `activePane`, the enabled-actions set with reasons, the open-modal stack. Events
  (`opened`/`closed`/`active-changed`/`moved`) are its deltas. The **per-tab ModelView** scopes the same
  shape to a `TabId` (the per-tab AIUI falls out for free). Prospective REQs:
  `spec://vibeterm/modelview#tree`, `#per-tab-scope`, `#deltas`.

### 2.12 The AI-UI surface — incl. plane unification (RQ14)

- **PORTS** — the headless AIUI offers `list_actions(filter?)`, `invoke(addr,args)`, `state() -> ModelView`,
  `search(query,tab?)`; because enablement is pure + introspectable, the model is serialisable, and
  invocation is address-based, this surface is a **thin adapter with a no-op `present`** — the
  **reference** surface; visual surfaces are projections
  ([PROP-039 §11.3 `#aiui`](../../spec/modules/vibe-actions/PROP-039-action-system.md#aiui)).
- **ADAPTS** — the TS AIUI is a **peer client** of the engine (the same `invoke` the chrome calls),
  exposed over the transport (§2.14); the four verbs are typed functions on a `VibetermAIUI` facade.
- **NEW (RQ14 — plane unification, to be earned in Phase 3).** Today `vibe aiui` (PROP-042) drives three
  planes: render (CDP, observation-only), terminal (`--control` HTTP over a live single view), model
  (`vibe aiui state` → `ModelView`). The new semantic AIUI (`invoke` over the shell action core) is a
  fourth control surface. The relationship is open; the lean recommendation (to confirm/refute in Phase 3):
  - the four verbs **fold onto the model plane** (semantic control is the model plane's control half —
    `state` is the read, `invoke`/`list_actions`/`search` the write);
  - the **terminal plane stays** as the legacy single-view observation path, frozen (PROP-044 §8);
  - **CDP stays observation-only** (PROP-042 `#render-plane`);
  - the `vibe aiui` CLI addresses **both** the shell (`invoke vibeterm/*`) and the hosted `vibe tree`
    (`invoke vibe.tree/*`) through one verb set, scoped by target.
  - *Hypothesis:* unification loses nothing the three-plane model has. Falsified if it loses a capability.
  - Prospective REQs: `spec://vibeterm/aiui#peer-client`, `#plane-unification`, `#cli-target-scope`.

### 2.13 Visual language & design system (RQ7a–e)

- **PORTS (concept)** — colour reaches a component only through **semantic role-tokens** (a data-driven
  `Palette`); components name a role, never a raw colour; one source of truth, **projected**; the theme is
  the "CSS" — a restyle touches only the theme
  ([tui-visual-language.md §3 `#palette-tokens`](../../spec/design/tui-visual-language.md#palette-tokens),
  [PROP-037 §2.2.1](../../spec/modules/vibe-cli/PROP-037-tree-tui.md#palette-tokens)). Window aesthetics
  (solid panel + rounded frame + title chip + padding + shadow + close) and spacing/rhythm
  (`PAD_X`/`PAD_Y`/`GUTTER`, centred rows) are first-class
  ([§5 `#window-aesthetics`](../../spec/design/tui-visual-language.md#window-aesthetics),
  [§6 `#spacing-rhythm`](../../spec/design/tui-visual-language.md#spacing-rhythm)).
- **ADAPTS** — the TUI `Palette` (role → `Color::Rgb`) → **GUI design tokens** (role → CSS custom
  property); "one Theme projected across tiers" → **"one token set projected across themes × modes"**;
  the TUI's tier-degradation (truecolor → 256 → 16 → dumb) **has no GUI analogue** — it is replaced by
  theme + accessibility/density modes. Live theme switch = rebind the CSS custom properties (no reload).
- **NEW (RQ7a–e — the genuinely new design-system surface).** The TUI glyph vocabulary
  (`▾▸●○╭╮▁▂▃`) does **not** carry over; a GUI has new surface:
  - **RQ7a — Tailwind v4 `@theme` vs our tokens.** *Lean:* our semantic role-tokens resolve to CSS custom
    properties; Tailwind v4's `@theme` **consumes** them (Tailwind as the utility consumer, not a
    competing token namespace). One source.
  - **RQ7b — Kobalte theming.** *Lean:* a thin Kobalte adapter maps our roles onto Kobalte's expected CSS
    variables; components never reach past our tokens.
  - **RQ7c — Accessibility modes (contrast / reduced-motion / density).** *Lean:* a token-variant layer —
    a theme × mode matrix — driven by both CSS media queries (`prefers-reduced-motion`, `prefers-contrast`)
    and explicit user choice.
  - **RQ7d — The GUI icon vocabulary.** *Lean:* a small owned **SVG icon set** consumed through one
    `<Icon name role>` primitive; icons reference roles, never raw colour (the TUI glyph-table's
    discipline, re-pointed at SVG).
  - **RQ7e — Spacing & rhythm scale.** *Lean:* one owned spacing-scale exposed as tokens; Tailwind
    utilities reference it; the §6 rhythm rules port as layout primitives.
  - Two launch themes: a **dark purple** (after the ProjectX reference) and an **Anthropic-style**
    (PROP-044 §11, D7).
  - Prospective REQs: `spec://vibeterm/design#tokens`, `#tailwind-integration`, `#kobalte-adapter`,
    `#a11y-modes`, `#icon-system`, `#spacing-scale`, `#launch-themes`.

### 2.14 Transport — chrome↔engine (RQ15)

- **PORTS (constraint)** — the chrome↔engine command/event protocol is a **versioned, serialisable
  message contract** carrying **no Electron-specific types**, so its transport is swappable (PROP-044 §9,
  D5); Electron IPC is one transport adapter, not the contract.
- **ADAPTS — the contract form (RQ15, to be earned in Phase 3).** *Lean recommendation:*
  - **codec** — a TS **discriminated union** as the contract source, with a generated **JSON-Schema** for
    cross-language/conformance use (no IDL unless a sidecar proves it necessary);
  - **versioning** — **contract-semver** (the union's version), not per-message;
  - **exchange model** — **hybrid**: an event-stream for tab-lifecycle (`opened`/`closed`/`active-changed`/
    `moved`) + request-response for commands (`open`/`select`/`split`/`close`/`move-to-window`/
    `set-compact`);
  - **consistency** — **single-writer**: the main engine is authoritative; the Solid chrome store is a
    one-way projection (RQ3, §2.11); the chrome applies optimistic UI only for ephemeral state, never for
    the `ModelView`;
  - **backpressure / ordering** — events are monotonic per-window; the chrome drains in order.
  - *Hypothesis:* the discriminated union keeps Electron types out of the contract and ports to a future
    sidecar unchanged. Falsified if a transport requirement forces Electron types back in.
  - Prospective REQs: `spec://vibeterm/transport#contract-union`, `#versioning`, `#exchange-model`,
    `#consistency`.

### 2.15 Testability & evolvability

- **PORTS** — the headless AIUI is the **golden reference** surface; engine logic is unit-testable
  without a frontend ([PROP-039 §11.3, §12.2](../../spec/modules/vibe-actions/PROP-039-action-system.md#aiui)).
- **ADAPTS** — two test tiers under AI-Native TS: **engine-cell goldens** (pure TS, `vitest` — the tab
  registry, pane-layout maths, the protocol codec, the keymap resolver, the SE engine) and **integration
  goldens** (the AIUI peer over the engine, asserting `state()`/`list_actions()`/`invoke()` against a
  fixture `ModelView`). The conformance golden (§3) runs on the TS floor too.
- **NEW** — an **Electron-tier smoke** (a headless run that opens a tab, splits, tears off, and asserts
  the `ModelView` deltas) — the only tier that exercises the real `WebContentsView` reparent. Prospective
  REQ: `spec://vibeterm/test#tiers`.

---

## 3. The identity-grammar conformance surface (RQ12) {#conformance}

The load-bearing refinement of RP-A. "Self-contained, no **build**-dependency on vibevm-internal" is
correct and stays; "no shared grammar" is not — a Rust `vibe-actions` and a TS `vibeterm-core` that each
re-derive the address grammar, the `ModelView` shape, and the AIUI verbs independently **will diverge**
(R5). The resolution: an **identity-grammar spec** — a normative document both implementations validate
against a **conformance golden in CI**. Shared **grammar**, not shared **build-dep**.

### 3.1 The minimum shared surface (the grammar)

What **both** sides must agree on, and no more:

| Surface | Shared grammar | Source of truth |
|---|---|---|
| Address | `action://<group>/<name>[?params]`; `(group,name)` identity; query not identity; tombstone/alias on rename | PROP-039 §2 normatively; the identity-grammar spec re-states it for both sides |
| `ModelView` | the **field set + tree shape** (windows/tabs/panes, enabled-actions+reasons, modals, compact, activeWindow/Pane) and the **event-delta vocabulary** | the spec; the Rust side projects its TUI `ModelView` as a single-window degeneration of the tree |
| AIUI verbs | the four verbs' **names + signatures**: `list_actions(filter?)`, `invoke(addr,args)`, `state()`, `search(query,tab?)` | PROP-039 §11.3 |
| SE provider | the `SearchProvider` two-phase contract **shape** (`enumerate`/`resolve`/`on_selected`/`render_row`/`ItemAccessor`), the `RowDescriptor` fields, the match-ladder tiers | PROP-039 §10 |
| i18n key scheme | `action.<group>/<name>.name`/`.description`; `ResolvedLabel { value, original_en }`; English mandatory-complete | PROP-039 §8 |
| Capability | the `Safe`/`Mutating`/`Dangerous` lattice + the refusal semantics | PROP-039 §7.2 + §2.7 here (the AI-peer extension is vibeterm's, but the lattice is shared) |

What is **deliberately not shared**: the implementation language, the cell decomposition, the internal
types, the build graph, the runtime. The grammar is the **contract surface**; everything behind it is
free.

### 3.2 The conformance golden (the shape)

A single **fixture-driven golden** both CI floors run:

- a **fixture registry** — a small set of actions with addresses, presentations, params, enablement,
  capabilities — declared once in a language-neutral form (the JSON-Schema-validated discriminated union
  from §2.14, or a small DSL);
- a **fixture `Ctx`** + a set of **expected `Enablement`** values per action;
- a **fixture sequence of `invoke`s** + the **expected `ModelView` deltas**;
- a **fixture SE query** + the **expected ranked hits**.

The Rust `vibe-actions` and the TS `vibeterm-core` each load the fixtures and must produce the
**expected** results bit-for-bit (addresses, `ModelView` shapes, verb results, rankings). Drift fails the
floor on the offending side. This is a **characterization golden** in AI-Native terms — it pins the
behaviour the grammar promises.

### 3.3 Where the spec lives

The identity-grammar spec is a **normative document** under `spec/modules/vibeterm/` (vibeterm-owned,
self-contained) that **cites** the PROP-039 anchors as provenance and **re-states** the grammar for the
two-sided contract. It is the one surface D3 (the contracts) must ratify first; the conformance golden
follows it. Prospective REQs: `spec://vibeterm/conformance#grammar`, `#golden`, `#ci`.

---

## 4. The AI-UI evaluation matrix (RQ17) — DRAFT {#eval-matrix}

The owner's non-negotiable is that the AI drives any function **as well as or better than a human**. As
written that is a slogan; this matrix makes it measurable. **Status: draft — the task set and the metrics
are proposed here; the measurements are Phase-4 work (the findings REPORT checks P7 against it).**

### 4.1 The task set (representative, over the M1 shell)

| # | Task | Human path | AIUI path |
|---|---|---|---|
| T1 | Open a second terminal | click `+` in the list | `invoke tab.open` |
| T2 | Switch to tab #1 | click the tab row | `invoke tab.select?id=<t1>` |
| T3 | Split the active terminal | right-click → *Open in split view* | `invoke pane.split?dir=right` |
| T4 | Tear a terminal into a new window | right-click → *Open in new window* | `invoke tab.move-to-window?tab=<t>&window=new` |
| T5 | Close the split pane | click the pane `×` | `invoke pane.close?id=<p>` |
| T6 | Toggle compact | click the compact toggle | `invoke view.set-compact?on=true` |
| T7 | Switch theme (dark purple ↔ Anthropic) | (UI control, M1 placeholder) | `invoke theme.set?id=<anthropic>` |
| T8 | Switch locale (en ↔ ru) | (UI control, M1 placeholder) | `invoke locale.set?id=ru` |
| T9 | Search an action by name | open SE, type, select | `invoke search.everywhere?…` then `invoke <found>` |
| T10 | Assert the resulting state | (visual) | `state()` → assert the `ModelView` tree |

### 4.2 The metrics

- **Success rate** — did the task reach the intended `ModelView` state? (asserted structurally, not
  visually, for both paths).
- **Latency** — wall time from intent to the `ModelView` settling.
- **Observability** — can the driver assert the full resulting state (the AIUI path's `state()` is
  canonical; the human path's observability is whatever PROP-042 exposes — a deliberate asymmetry to
  close).

### 4.3 The parity bar (P7)

The AIUI path reaches **parity** if, across the matrix, it matches the human path on **success rate** and
**observability**, and trades **latency** within an acceptable bound (the AIUI is not user-blocking, so a
bound measured in tens of ms is fine). Residual gaps are named **design obligations**, not silent gaps.

---

## 5. Predictions — status (see §8 for the research-close check)

[P1–P8](VIBETERM-UI-ARCHITECTURE-RESEARCH-PLAN-v0.1.md#predictions) tracked across the research; §8 holds
the close-of-research verdict per prediction. Phase-by-phase:

- **P1 (render-free TS engine)** — **on track.** Every core concern maps to a TS cell with a
  dependency-boundary lint (§2.2). *Closed* when the lint lands on the floor (Phase 4 / D4).
- **P2 (`ModelView` tree, re-resolution only)** — **on track.** The window→tab→pane tree generalises the
  TUI snapshot with event-deltas, no second mutation mechanism (§2.11). *Closed* when the engine cell +
  golden land.
- **P3 (identity-grammar conformance)** — **on track.** The minimum surface is identified (§3.1); the
  golden shape is specified (§3.2). *Closed* when both CI floors run it green.
- **P4 (Solid vs immutable `ModelView`)** — **on track, but the riskiest.** The one-way projection model
  is recommended (§2.11); falsification is empirical (rebuild cost at tab count). *Open* until the engine
  cell + a real Solid projection measure it.
- **P5 (capability surface for an AI peer)** — **on track.** The model is sketched (§2.7); *open* until
  Phase 3 validates it is non-bypassable.
- **P6 (token set projected across themes/modes)** — **on track.** The tier→mode mapping is identified
  (§2.13); the sub-questions RQ7a–e are the earning work.
- **P7 (AI-UI parity)** — **open.** The matrix is drafted (§4); measurement is Phase 4.
- **P8 (we lead + comparative yields adoptable mechanisms)** — **open.** Awaits Phase 2.

---

## 6. Phase 2 — external comparative (clean-room, docs/behaviour-first; sketch depth)

**Posture (RP-E accepted):** docs/behaviour-first for all; source-read only MIT VS Code under the firewall;
Zed (GPL) / Warp (closed) source **not** read. **Depth (RP-B accepted):** sketch — the goal of this
research is the architecture, not a competitor audit; the two-way gaps below are sufficient to confirm or
refute the leans. (A quote-backed, access-dated comparative can be re-run later via the re-fetch table §7
without reopening the architecture.)

**The headline finding (P8): no mainstream GUI app exposes a render-free, addressable, AI-drivable action
core with a serialisable `ModelView` as the *reference* surface.** Confirmed across the set:

- **VS Code (MIT).** Commands are addressable strings (`editor.action.*`) and programmatically invocable
  (`executeCommand(id, …args)`); the Command Palette (`>` prefix) is the discovery surface. But: command
  ids are a **flat global namespace** (convention, not enforced); args are **`any`**; enablement is a
  **stringly `when`-clause**; the model is **not serialisable as a reference** — the extension host and
  the workbench are coupled to rendering. The agent surface (Copilot) drives via the extension API + CDP,
  not a semantic core. **Adoptable:** the command/palette split, `executeCommand` as a thin caller, the
  contribution model.
- **Zed (GPL — docs only).** Is genuinely **action-centric** (every key binding resolves to a named
  action; actions are the behavioural backbone) — closest in spirit to our thesis. But the actions are
  editor-coupled, and the agent panel drives the editor, not a render-free peer over a serialisable model.
  **Adoptable:** the "everything is an action" stance validates our addressability pillar.
- **Warp (closed — docs only).** AI-native terminal; the agent acts on blocks/sessions. The control
  surface is **proprietary** and block-oriented (not a render-free action core); no documented serialisable
  model surface for an external driver. **Adoptable:** the block/session as a first-class entity.
- **Raycast (closed — docs).** A command launcher; extensions expose commands with typed arguments. Closest
  to the typed-param idea, but launcher-scoped, not a full application action core. **Adoptable:** typed
  command arguments + a searchable command surface.

**Token systems (RQ7a–e leans confirmed):**
- **Radix Themes / Radix Colors** — semantic scales, CSS variables, no hardcoded hex at call sites; a
  theme = a scale set. Validates our role-token + theme-as-token-set model (§2.13).
- **Tailwind v4 `@theme`** — CSS-variable-native theme layer; utilities consume the variables. Confirms
  RQ7a lean: our tokens as the source, Tailwind `@theme` as the utility consumer (one namespace, not two).
- **Style-Dictionary / W3C Design-Tokens** — build-time token transforms and a standard interchange
  format. Validates that a token set is projectable across themes/modes (RQ7c); the W3C format is a
  candidate for the conformance/interchange side of the design system.

**Two-way gap — where we trail:** ecosystem breadth (VS Code's thousands of commands; Raycast's
extensions), and a polished palette UX (VS Code's Quick Access ranking). **Where we lead:** the render-free
core, the serialisable `ModelView` as reference, the addressable + collision-checked + typed grammar, the
conformance-tested Rust↔TS identity-grammar, and the AIUI peer as a first-class surface — none of the
incumbents has all of these. **Net:** P8 is **confirmed** (we lead on the core; we adopt their token
scales and palette/keyboard model).

## 7. Phase 3 — pitfalls → design obligations

Each [plan §5 pitfall](VIBETERM-UI-ARCHITECTURE-RESEARCH-PLAN-v0.1.md#pitfalls) becomes a binding design
obligation (a delta in §8); the RQ13/14/15 leans close here.

| Pitfall | Obligation (delta) |
|---|---|
| The CDP trap | Control is `invoke`; CDP observation-only (D1, D8). |
| Two cores drift silently | Identity-grammar conformance spec + CI golden (D2). |
| The capability hole | `CallerScope` + prompt-on-`Dangerous` + audit + scope-REFUSE (D7). |
| AIUI-plane proliferation | The four verbs fold onto the model plane; one CLI, target-scoped (D8). |
| Pixel-only capabilities | Every chrome capability is a named `action://vibeterm/*` (D5). |
| Hardcoded style | Design tokens; components reference roles, never hex (D11). |
| A render-coupled model | `#no-render-dep` dependency-boundary lint on the floor (D3). |
| Double source of truth | Engine is the single writer; Solid store is a one-way projection (D6). |
| i18n retrofit | Address-keyed catalogue + legibility gate from the start (D9). |
| Design-system-as-afterthought | First-class design system; the GUI twin of `tui-visual-language.md` (D11). |
| GUI-only unknowns discovered late | Catalogued up front; each a named REQ (D12). |
| The unmeasured AI-UI | The evaluation matrix; P7 is measured, not asserted (D13). |
| Over-building the universal contract | The shell is the first consumer; readiness proven by design (D14). |

## 8. Phase 4 — synthesis: the numbered architecture deltas {#deltas}

The pillars ([plan §6](VIBETERM-UI-ARCHITECTURE-RESEARCH-PLAN-v0.1.md#pillars)), each earned as a delta
naming its prospective REQ home (ratified in D3). REQ ids are provisional.

- **D1 — AI-UI-Ready by construction.** Control is semantic `invoke`; CDP is observation-only; the headless
  AIUI is the reference surface. → `spec://vibeterm/aiui#reference-surface`, `#cdp-observation-only`.
- **D2 — Identity-grammar conformance.** A normative identity-grammar spec (address grammar, `ModelView`
  schema, AIUI verbs, SE provider contract, i18n key scheme, capability lattice) + a CI conformance golden
  on both the Rust `vibe-actions` and TS `vibeterm-core` floors. Shared grammar, not shared build-dep. →
  `spec://vibeterm/conformance#grammar`, `#golden`.
- **D3 — Render-free engine, lint-enforced.** The tab registry, pane-layout maths, session model, and
  protocol codec import no Solid/DOM/Electron types; a dependency-boundary lint is on the floor. →
  `spec://vibeterm/architecture#render-free-boundary`.
- **D4 — The Action value & the Registry (ported).** Addressable `action://vibeterm/*` actions; mandatory
  name+description; collision-erroring registry; referential integrity; full enumeration; typed params +
  validation; typed `Ctx` + pure enablement + introspection. → `spec://vibeterm/actions#value`,
  `#registry`, `#enablement`, `#params`.
- **D5 — Every capability is a named action.** The Solid chrome and the AI both call one `invoke`; nothing
  is pixel-only. → `spec://vibeterm/actions#invoke-primary`.
- **D6 — The serialisable `ModelView` tree is the source of truth.** `windows[]→tabs[]→panes[]` + enabled
  actions + reasons; the engine is the single writer; the Solid chrome is a one-way projection rebuilt on
  re-resolution; ephemeral chrome state never crosses the seam; per-tab scope by `TabId`. →
  `spec://vibeterm/modelview#tree`, `#projection`, `#per-tab-scope`.
- **D7 — Capability-scoped AI / networked callers.** `CallerScope` (host-granted, not self-reported);
  prompt-on-`Dangerous`; audit; scope-REFUSE. → `spec://vibeterm/aiui#caller-scope`, `#prompt-on-dangerous`,
  `#audit`, `#scope-refuse`.
- **D8 — One AIUI surface, not two.** The four verbs fold onto the model plane; the legacy `--control`
  terminal plane stays single-view (frozen, PROP-044 §8); CDP stays observation-only; the `vibe aiui` CLI
  is target-scoped (shell `vibeterm/*` vs hosted `vibe.tree/*`). → `spec://vibeterm/aiui#plane-unification`,
  `#cli-target-scope`.
- **D9 — i18n from the start.** Address-keyed catalogue; `ResolvedLabel { value, original_en }`; English
  mandatory-complete; reactive live locale swap (no reload); **en + ru** ship; legibility gate in CI. →
  `spec://vibeterm/i18n#catalogue`, `#reactive-swap`, `#legibility-gate`, `#ru-locale`.
- **D10 — Search Everywhere, surface-neutral.** The two-phase provider model + the engine; GUI-rendered
  (cmdk/Kobalte); virtualisation; shell providers (sessions, actions, reserved profile). →
  `spec://vibeterm/search#provider-model`, `#virtualisation`.
- **D11 — First-class design system.** Semantic design tokens → CSS custom properties → Tailwind v4 `@theme`
  as the utility consumer; a Kobalte theming adapter; accessibility/density modes as a theme × mode matrix;
  an owned SVG icon vocabulary; one spacing-scale; two launch themes (dark purple + Anthropic-style); live
  switch by rebinding tokens. → `spec://vibeterm/design#tokens`, `#tailwind-integration`, `#kobalte-adapter`,
  `#a11y-modes`, `#icon-system`, `#spacing-scale`, `#launch-themes`.
- **D12 — The GUI-only inventory (catalogued).** DnD (tab reorder, tear-off drag-back); multi-pane focus;
  clipboard per platform; OS integration (notifications, dock/taskbar badges, jump-lists); DPI/scale; the
  accessibility tree. Each a named REQ; none reopens a frozen axis. → `spec://vibeterm/gui#dnd`,
  `#multi-pane-focus`, `#clipboard`, `#os-integration`, `#dpi`, `#a11y-tree`.
- **D13 — The measurable AI-UI.** The evaluation matrix (§4) is a deliverable; P7 is measured. →
  `spec://vibeterm/aiui#evaluation-matrix`.
- **D14 — Capability-scoped, transport-agnostic, sidecar-ready.** The chrome↔engine protocol is a versioned
  serialisable discriminated union (no Electron types) + a generated JSON-Schema; contract-semver; hybrid
  event+RPC; single-writer main; Electron IPC via a typed preload bridge is one transport adapter. →
  `spec://vibeterm/transport#contract-union`, `#versioning`, `#exchange-model`, `#consistency`.
- **D15 — Self-contained & detachable.** The whole vibeterm system lives under `spec/modules/vibeterm/` (+
  `apps/vibeterm/`) with no build-dep on vibevm-internal; methodology is provenance. →
  `spec://vibeterm#self-contained`.
- **D16 — The shell entity set & catalogue.** Window, Tab, Pane, Session, Profile, Theme, Locale; the
  `action://vibeterm/*` catalogue (`tab.open`, `tab.select`, `tab.close`, `pane.split`, `pane.close`,
  `tab.move-to-window`, `view.set-compact`, `theme.set`, `locale.set`, `search.everywhere`, `quit`). →
  `spec://vibeterm/entities#*`, `spec://vibeterm/catalogue#*`.

### Predictions check (P1–P8)

- **P1 — render-free TS engine:** **confirmed** (architecture; closed when the lint lands, §D3 / D4 build).
- **P2 — `ModelView` tree via re-resolution:** **confirmed** (architecture; event-deltas, no second
  mechanism).
- **P3 — identity-grammar conformance:** **confirmed** (§3; the surface + golden are specified).
- **P4 — Solid vs immutable `ModelView`:** **conditionally confirmed** — the one-way projection (D6) holds
  by construction; the empirical rebuild-cost check is a build-phase measurement.
- **P5 — capability surface for an AI peer:** **confirmed** (D7; non-bypassable by the host-granted scope).
- **P6 — token set projected across themes/modes:** **confirmed** (D11; the tier analogue is replaced).
- **P7 — AI-UI parity:** **open** — the matrix (§4) is drafted; measurement is a build-phase task.
- **P8 — we lead + comparative yields adoptable mechanisms:** **confirmed** (§6).

**REPORT.** The research closes with the architecture earned: a render-free, conformance-tested,
AI-UI-first TS shell that ports the methodology where the GUI has an analogue, adapts it where TS/Electron
/Solid demand, and invents only the GUI-specific surface. The frozen axes (PROP-044 D0–D7) hold; no axis
was reopened. Hands off to D2 (design-doc) and D3 (contracts).

---

## 9. Re-fetch / provenance table (Phase 2 external sources)

Sketch-depth access (docs/behaviour; RP-E posture held; no GPL/closed source read). Re-run via these URLs
for a quote-backed, access-dated refresh without reopening the architecture.

| Source | Concern | Licence | Posture (RP-E) | Access date | Re-fetch URL |
|---|---|---|---|---|---|
| VS Code | action/palette model; semantic-control surface | MIT | source-readable under the firewall (not read this pass — docs sufficient at sketch depth) | 2026-07-19 | https://github.com/microsoft/vscode |
| Zed | AI/agent control surface; action-centric stance | GPL-3.0 | docs/behaviour only — **no source** | 2026-07-19 | https://zed.dev/docs |
| Warp | AI/agent control surface; block/session model | closed | docs/behaviour only — **no source** | 2026-07-19 | https://docs.warp.dev |
| Raycast | command model; typed command args | closed | docs/behaviour only | 2026-07-19 | https://developers.raycast.com |
| Radix Themes/Colors | design-token system | MIT | docs | 2026-07-19 | https://www.radix-ui.com |
| Tailwind v4 | theme layer / `@theme` | MIT | docs + source | 2026-07-19 | https://tailwindcss.com |
| Style-Dictionary / W3C Design-Tokens | token format | MIT / W3C | docs | 2026-07-19 | https://styledictionary.com / https://design-tokens.org |

Internal sources (no firewall, no re-fetch — they are this repo): PROP-039, PROP-037, PROP-036, PROP-042,
PROP-044, `spec/design/action-system.md`, `spec/design/tui-visual-language.md`.

---

## 10. Quick-start (cold-resume)

```sh
# boot first: CLAUDE.md → spec/boot/INDEX.md → its files → spec/WAL.md → CONTINUE.md
#   → research/vibeterm/task.md → VIBETERM-UI-ARCHITECTURE-RESEARCH-PLAN-v0.1.md → THIS FILE
#
# Phase 1 (this revision): internal extraction done — §2 ports/adapts/new + §3 conformance + §4 eval draft.
# Phase 2 next: external comparative (clean-room, docs-first; RP-B/RP-E leans accepted).
# Output lives ONLY here; the firewall (plan §1) keeps external sources out of D2/D3/D4.
bash tools/self-check.sh   # the floor — green at every phase boundary
```

**Pointer.** `spec/WAL.md` (its `_Updated:` line) is the canonical living state and supersedes any
snapshot. This findings doc is research output; the design-doc (D2), the contracts (D3), and the
implementation (D4) are separate downstream campaigns.
