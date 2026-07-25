# The vibevm Action System — design & architecture {#root}

##genre-line **Genre:** design (lore) — non-binding rationale and architecture. The normative contract is
**Spec 1 = PROP-039** (`spec://vibevm/modules/vibe-actions/PROP-039`, forthcoming); this document
explains *why* the system is shaped as it is and *how* the pieces fit. It is derived, behind the
clean-room firewall, from the study
[`action-systems-vscode-idea.md`](../../legacy-spec/research/action-systems-vscode-idea.md) (the design
obligations DO1–DO18 and roadmap deltas Δ1–Δ16 cited throughout) and governed by the mandate in
[`ACTION-SYSTEM-RESEARCH-PLAN`](../../legacy-spec/research/ACTION-SYSTEM-RESEARCH-PLAN-v0.1.md#mandate). When this
lore and the contract disagree, **the contract wins** and this file is corrected (spec-genres). @doc/done

## 0. Thesis — in one paragraph {#thesis}

##THESIS The **vibevm action system** is a **frontend-agnostic, addressable, programmatically-drivable
behaviour layer** — the behaviour-layer twin of `spec://`. Every thing a UI can *do* is an
**Action** with a stable **address** (`action://<group>/<name>`), a **typed parameter schema**, a
**typed context-enablement** predicate, a **mandatory human-readable name + description**, and a
pure **`invoke`**. Actions live in a collision-erroring **registry**, are bound to keys by a pure
**keymap resolver**, are localized through an **address-keyed message catalogue**, and are
discovered through a **provider-model Search Everywhere** that searches actions, packages, and every
package-card field today and any structured universe (AI-Native specmap nodes) tomorrow. The core is
**pure Rust with zero rendering dependencies**; a visual **Surface** (the TUI now; web/IDE later) is
one optional projection, and the **headless AIUI surface is the reference** — an AI drives and
observes the interface by address and by serialisable state, never by pixels. @doc/done

## 1. Founding principles {#principles}

##principles-lead Six principles, each carrying its study derivation (see the findings doc for the full argument): @doc/done

1. ##P-ADDRESSABILITY **Addressability of behaviour** (Δ1). Actions are addressed by URI, never by paraphrase — the
   same move `addressable-specs` makes for facts, applied to behaviour. @doc/done
2. ##P-PROGRAMMATIC-PRIMARY **Programmatic invocation is primary; the AIUI is the reference surface** (Δ8, Δ15, DO18).
   `invoke(address, args, ctx)` is *the* interface; key presses and menu clicks are thin callers.
   Because of this, an AI can operate the UI headless. @doc/done
3. ##P-FRONTEND-AGNOSTIC **Frontend-agnostic core** (Δ10, DO12). Zero rendering deps; visual surfaces are adapters. @doc/done
4. ##P-HUMAN-LEGIBILITY **Human-legibility is a discipline, not decoration** (Δ6, DO5). Mandatory name + description,
   searchable, enforced by a floor gate. @doc/done
5. ##P-DISCOVERY-UNIVERSE **Discovery over any structured universe** (Δ7, Δ14, DO16). One provider seam — actions,
   packages, card-fields now; specmap/AI-Native structure later. @doc/done
6. ##P-TYPED-EVERYTHING **Typed everything** (Δ4, Δ5, DO3, DO4). Typed addresses, params, context, results — the gap
   both incumbents leave stringly. @doc/done

## 2. Crate & module architecture {#crate}

##CRATE-INVARIANT A new crate **`vibe-actions`** (home `spec/modules/vibe-actions/`, contract PROP-039). Pure Rust,
**no `ratatui`/`crossterm`/DOM/any rendering dependency** — this is the invariant that makes the
AIUI and every other surface possible (DO12). Modules: @doc/done

| Module | Owns | Key deltas |
|---|---|---|
| ##ROW-MOD-ADDRESS `address` @doc/done | The `action://<group>/<name>[?params]` address type, its grammar, parse/format, uniqueness, tombstone/alias @doc/done | Δ1 @doc/done |
| ##ROW-MOD-ACTION `action` @doc/done | The **Action** value (address · presentation · param-schema · enablement · invoke) and its immutable resolved snapshot @doc/done | Δ2 @doc/done |
| ##ROW-MOD-REGISTRY `registry` @doc/done | The collision-erroring registry: register/lookup, referential-integrity checks, full enumeration @doc/done | Δ3, Δ12 @doc/done |
| ##ROW-MOD-PARAMS `params` @doc/done | The typed, serialisable named-parameter **schema** + **values** + validation @doc/done | Δ4 @doc/done |
| ##ROW-MOD-CONTEXT `context` @doc/done | The typed **context snapshot** (a `TypeId`-keyed typemap), context keys, and the pure **enablement** predicate → `{visible, enabled, reason}` @doc/done | Δ5 @doc/done |
| ##ROW-MOD-INVOKE `invoke` @doc/done | Invocation: sync/async, typed **result/error**, cancellation, the **capability** check @doc/done | Δ8, Δ11 @doc/done |
| ##ROW-MOD-I18N `i18n` @doc/done | The address-keyed message **catalogue** (Fluent-backed), `MessageKey`, `ResolvedLabel {value, original_en}`, locale swap @doc/done | Δ13 @doc/done |
| ##ROW-MOD-KEYMAP `keymap` @doc/done | Key → (address, args) binding, the pure **3-state resolver**, chord model (timers live in the adapter) @doc/done | Δ9 @doc/done |
| ##ROW-MOD-SEARCH `search` @doc/done | The **Search Everywhere** engine: the provider trait (two-phase), the match/rank pipeline, tabs, dedup, recency, freeze-on-more @doc/done | Δ7, Δ14, Δ16 @doc/done |
| ##ROW-MOD-SURFACE `surface` @doc/done | The **Surface** adapter trait (the seam) + the headless **AIUI** surface + the serialisable model-state view @doc/done | Δ10, Δ15 @doc/done |

##crate-consumers Consumers: **`vibe-cli`** hosts the TUI Surface (Spec 2 revises PROP-037 to sit on this crate);
future web / VSCode / JetBrains / Zed surfaces are additional adapters. Nothing in `vibe-actions`
depends on any consumer. @doc/done

## 3. The core types {#types}

##types-lead Illustrative Rust shapes (the contract PROP-039 fixes the normative form; these convey intent): @doc/done

```rust
// address — Δ1
struct ActionAddr { group: Group, name: Name }          // (group, name) globally unique; Display = "action://<group>/<name>"
// e.g. action://vibe.tree/copy.markdown , action://core/search.everywhere

// action — Δ2
struct Action {
    addr: ActionAddr,
    presentation: Presentation,          // Δ6 — name + description MANDATORY, localizable
    params: ParamSchema,                 // Δ4 — may be empty
    enablement: Box<dyn Fn(&Ctx) -> Enablement>,  // Δ5 — pure, fast, no rendering, no UI thread
    invoke: Box<dyn Fn(&Ctx, ParamValues) -> BoxFuture<InvokeResult>>,  // Δ8 — primary interface
    capability: Capability,              // Δ11
    search_meta: SearchMeta,             // Δ16 — synonyms/aliases/abbreviations, keywords
}
struct Presentation { name: Msg, description: Msg, icon: Option<Glyph>, category: Option<Msg> }
struct Msg { key: MessageKey, default_en: &'static str }   // Δ13 — key = "action.<addr>.name"
struct Enablement { visible: bool, enabled: bool, reason: Option<Localized> }  // "why disabled"

// search provider seam — Δ7, Δ14 — the two-phase "searchable structured universe" contract
trait SearchProvider {
    fn id(&self) -> ProviderId;
    fn group_name(&self) -> Localized;       // tab label + group separator
    fn sort_weight(&self) -> i32;            // orders TABS/groups, NOT elements
    fn separate_tab(&self) -> bool;
    fn enumerate(&self, q: &Query, sink: &mut dyn KeySink);          // cheap keys, streamed, scope-aware
    fn resolve(&self, key: &ItemKey) -> Vec<Hit>;                    // heavy items, only for matched keys
    fn accessor(&self) -> &dyn ItemAccessor;                        // {label, description, key} → one ranker
    fn on_selected(&self, hit: &Hit, mods: Modifiers) -> Selected;  // Selected::Close | Selected::Stay
    fn render_row(&self, hit: &Hit) -> RowDescriptor;              // normalized: {icon, primary, secondary, group, enabled, kind}
}

// surface seam — Δ10, Δ15
trait Surface {                        // a visual adapter (TUI) OR the headless AIUI
    fn present(&mut self, view: &ModelView);   // no-op for a headless surface
    fn next_event(&mut self) -> Event;         // key event, or a programmatic Invoke/Query for AIUI
}
struct ModelView { /* serialisable snapshot: focus, modals, visible rows, enabled actions … */ }  // Δ15
```

## 4. Data flow — MVC, with the model as the real interface {#flow}

```
        ┌──────────── Surface (adapter) ───────────┐
event → │ TUI: key/mouse   |   AIUI: invoke/query  │
        └───────────────┬──────────────────────────┘
                        ▼
              Controller (keymap resolve → address + args)     Δ9
                        ▼
              invoke(address, args, ctx)  ──►  Action           Δ8
                        ▼
              Action mutates Model (typed, SERIALISABLE)
                        ▼
        ┌───────────────┴───────────────┐
        ▼                               ▼
  View renders ModelView          AIUI reads ModelView + enumerates enabled actions   Δ15
  (TUI, optional)                 (headless, the reference)
```

##MODEL-IS-THE-INTERFACE The load-bearing property: **the Model + the action registry are the interface**; the View is one
optional projection. An AIUI needs only three capabilities the core already provides — *enumerate
enabled actions with their addresses/params/reasons*, *invoke by address with typed args*, and *read
the serialisable `ModelView`* — none of which touch rendering. This is why AIUI is "not built now"
yet costs nothing later: it is a `Surface` whose `present` is a no-op and whose events are
programmatic (DO18). @doc/done

## 5. Key design decisions {#decisions}

##decisions-lead Recorded in the four-field form (Decision · Why · Considered-and-rejected · Revisit-when); these
become decision records at their governing PROP-039 anchors. @doc/done

- ##D1-URI-ADDRESS **D1 — Address = `action://<group>/<name>[?params]` (URI).** *Why:* the behaviour-layer twin of
  `spec://<module>/<doc>#<anchor>`; owner-ratified; typed params ride the query; `(group, name)`
  globally unique (ties to `qualified-naming`). *Rejected:* IntelliJ-style dotted FQDN
  (`org.vibevm.tree.copy.markdown`) — parameters cannot live in the address and it reads less like
  the project brand; a bare opaque string (both incumbents) — no structure, no enforced uniqueness.
  *Revisit:* if URI parse cost ever shows on a profile (it will not at these volumes). @doc/done
- ##D2-COLLISION-ERROR **D2 — The registry errors on collision.** *Why:* both incumbents are inconsistent or silent
  (VSCode: three policies; IntelliJ: log-and-drop) → surprise + lost actions; `qualified-naming`
  says a collision is a hard, distinct failure. *Rejected:* the permissive override-stack
  (VSCode `CommandsRegistry`) as the *default* — override must be an explicit, uniform semantics,
  not an accident of which door you use. *Revisit:* if a real layered-override use case appears →
  add an explicit `override_of(addr)` op, still collision-checked. @doc/done
- ##D3-TYPED-CONTEXT **D3 — Typed context + pure enablement.** *Why:* IntelliJ's `update()` EDT/BGT threading is its
  single biggest documented pain; VSCode's stringly `when` evaluates false forever on a typo. A pure
  function over a `TypeId`-keyed snapshot has neither failure mode and is introspectable ("why
  disabled") and enumerable ("what keys does this context carry"). *Rejected:* a stringly `when`
  DSL; a nullable `DataContext`-style map. *Revisit:* never — this is the core differentiator. @doc/done
- ##D4-PROGRAMMATIC-PRIMARY **D4 — Programmatic invocation is primary; AIUI is the reference surface.** *Why:* the owner's
  AIUI mandate + vibevm's two-process model; both incumbents retrofitted programmatic invocation and
  it shows (VSCode `unknown[]`; IntelliJ result recovered out-of-band). *Rejected:* UI-event-primary
  with a bolted-on programmatic path. *Revisit:* never. @doc/done
- ##D5-PROVIDER-MODEL **D5 — Search Everywhere is a provider model with the two-phase enumerate→resolve contract.**
  *Why:* IntelliJ's proven design; it generalises to any structured universe (packages now, specmap
  later) through one seam and keeps per-keystroke latency by resolving only matched keys. *Rejected:*
  a hardwired god-provider (VSCode's `anythingQuickAccess` — the study's explicit cautionary tale).
  *Revisit:* if a provider needs a fundamentally different fetch shape → it drops to the raw
  `enumerate`+`resolve` escape hatch (already supported). @doc/done
- ##D6-I18N-KEYS **D6 — i18n: address-derived keys + inline English default + `{value, original_en}` + Fluent.**
  *Why:* IntelliJ's key-from-id (no second namespace) + VSCode's inline-English (self-documenting,
  always-present fallback) + `localize2`'s original-kept-beside-value (so Search Everywhere matches
  the English label under any locale); Fluent is the Rust-idiomatic catalogue with named args +
  plurals. *Rejected:* VSCode's build-time numeric-index indirection (opaque at runtime, needs a
  build step); IntelliJ's JDK-`ResourceBundle` reflection (accidental complexity). *Revisit:* if a
  non-Fluent format is mandated downstream. @doc/done
- ##D7-ENGLISH-GATE **D7 — The human-legibility gate targets English only.** *Why:* English is the source of truth and
  the always-present fallback in both incumbents; other locales may lag. *Rejected:* gating every
  locale (blocks shipping on incomplete translations). *Revisit:* if a locale is declared
  ship-blocking. @doc/done
- ##D8-ONE-RENDERER **D8 — One normalized row renderer per surface.** *Why:* the study's ADAPT note — IntelliJ's
  per-provider Swing renderers give heterogeneous rows and inconsistent look; a TUI wants one
  renderer over a `RowDescriptor {icon, primary, secondary, group, enabled, kind}` so every category
  looks uniform. *Rejected:* per-provider renderers. *Revisit:* if a provider needs a bespoke row
  the descriptor cannot express → extend the descriptor, not the renderer count. @doc/done
- ##D9-PURE-RESOLVER **D9 — The keymap resolver is pure and returns a 3-state result** (`NoMatch | NeedMoreChords |
  Found`); chord timers, IME, focus walking live in the **adapter**. *Why:* VSCode's clean
  `ResultKind` + IntelliJ's ambiguity-as-list-resolved-by-enablement, minus the UI coupling.
  *Rejected:* a resolver that owns timers/focus (both incumbents entangle these). *Revisit:* n/a. @doc/done
- ##D10-ONE-SCORER **D10 — Ranking: one commensurable scorer emitting score + highlight ranges, recency-weighted,
  with an exact-match floor.** *Why:* VSCode's two-engine highlight/rank mismatch (DO7) + "recency
  beats score" (DO8) + IntelliJ's exact-match floor. *Rejected:* separate match/highlight engines.
  *Revisit:* if an ML reranker is added → it slots above the floor, like IntelliJ's. @doc/done

## 6. Search Everywhere — the architecture (the acceptance) {#search}

##SEARCH-ACCEPTANCE The feature the whole system must deliver: **F1 opens a window that searches packages by name, every
field of the package detail cards, and all actions — with a hybrid "All" tab and per-category tabs —
and invokes a found action in place.** @doc/done

##providers-lead **Providers at ship** (all against the one `SearchProvider` seam, §3): @doc/done
- ##PROVIDER-PACKAGE `PackageProvider` — keys = package FQNs streamed from the `PackageTree`; resolve = the
  `PackageNode`; navigate = reveal/select it in the tree. @doc/done
- ##PROVIDER-PACKAGE-FIELD `PackageFieldProvider` — keys = every field of every package detail card (name, version, kind,
  license, load-type, origin, path, deps, diagnostics…); resolve = the field's `(package, field)`;
  navigate = open the card focused on that field. *(This is the owner's "search inside all card
  fields.")* @doc/done
- ##PROVIDER-ACTION `ActionProvider` — keys = action addresses + names + descriptions + synonyms/aliases; resolve =
  the `Action`; `on_selected` = **invoke it** (perform → close; a toggle stays open). Disabled
  actions render greyed with their "why disabled" reason; shortcuts render right-aligned. @doc/done
- ##PROVIDER-STRUCTURE-RESERVED **Reserved (same trait, no engine change):** `StructureProvider` — AI-Native specmap spec/code
  nodes, added when the AI-Native language structure lands. @doc/done

##engine-lead **The engine** (single-threaded TUI-friendly, the study's ADAPT of IntelliJ's threaded design): @doc/done
- ##ENG-DEBOUNCE Per keystroke: **debounce** (~90–120 ms) + **cancel** the prior run. @doc/done
- ##ENG-PROVIDER-SET Active provider set = one (a category tab) or all filter-enabled (the "All" tab); each gets a cap
  (single 30 / All 15). @doc/done
- ##ENG-MATCH-LADDER Each provider **enumerates cheap keys** (streamed, cancellable), the keys are **matched** by one
  scorer (the match-tier ladder: exact → prefix → CamelCase/subsequence → substring → **name/
  description word — the fallback lane**, Δ6/DO5), survivors are **resolved** to hits. @doc/done
- ##ENG-COMMENSURABLE-SCALE Hits are wrapped `{hit, score, provider}` on **one commensurable scale**, **recency-weighted** with
  an **exact-match floor**, **deduped keeping the higher score** across providers, drained
  **round-robin** from per-provider bounded queues into a single flat list. @doc/done
- ##ENG-ORDERING Ordering: score DESC, tie → provider `sort_weight` DESC. In "All", a **group header** precedes each
  provider's rows; single tabs have none. The "All" tab carries a **category checkbox filter**. @doc/done
- ##ENG-FREEZE-ON-MORE **Freeze-on-"more":** a per-provider "more…" row re-queries that provider and freezes the rows
  above so async results don't reshuffle under the cursor. @doc/done
- ##ENG-NORMALIZED-RENDERER One **normalized renderer** (`RowDescriptor`) draws every category uniformly. @doc/done

##TABS-MODEL **Tabs** are built from the providers: sort by `sort_weight`; prepend "All" when >1; one tab per
`separate_tab` provider. `Tab`/`Shift-Tab` cycle. Selecting a row calls `provider.on_selected` →
`Close` dismisses, `Stay` keeps the window (in-place toggles). @doc/done

## 7. The AIUI surface — the reference {#aiui}

##aiui-lead Not built now; designed-for. The headless surface exposes, over the same core, a small programmatic
protocol (in-process API first; a JSON-RPC / MCP binding later, aligning with vibevm's MCP surface): @doc/done
- ##AIUI-LIST-ACTIONS **`list_actions(filter?) -> [{address, name, description, params, enabled, reason}]`** — enumerate
  the registry with live enablement (the enumeration + pure enablement the core already provides). @doc/done
- ##AIUI-INVOKE **`invoke(address, args) -> Result`** — the same `invoke` the keymap calls. @doc/done
- ##AIUI-STATE **`state() -> ModelView`** — the serialisable model snapshot (focus, modals, visible rows, current
  tree/selection) so the AI observes structured state, not pixels. @doc/done
- ##AIUI-SEARCH **`search(query, tab?) -> [Hit]`** — drive Search Everywhere programmatically. @doc/done

##AIUI-THIN-ADAPTER Because enablement is pure + introspectable, the model is serialisable, and invocation is
address-based, this surface is a thin adapter with a no-op `present`. Prototyping on the TUI proves
the core; the AIUI then "just works" because the core owes rendering nothing (DO18). @doc/done

## 8. i18n — the architecture {#i18n}

##I18N-ARCHITECTURE Per D6 / §3.8 of the findings doc: presentation is two `Msg { key = "action.<addr>.name" |
".description", default_en }`. A `Catalogue { locale, entries, parent }` chain resolves a key,
terminating in an `en` catalogue **seeded from the inline defaults** (release lookups never miss).
Every resolved label is `ResolvedLabel { value, original_en }` so Search Everywhere indexes both.
Locale swap is `ArcSwap<Catalogue>`. Packages ship `locales/<lang>.ftl`; a language-pack package may
override. The legibility gate asserts the **English** surface is complete; a `pseudo` locale QA build
surfaces un-externalised strings. `MessageKey`/`Localized` are newtypes; a CI check asserts every
registry key resolves in `en`. @doc/done

## 9. Mapping to `vibe tree` (Spec 2 preview) {#vibe-tree}

##VIBE-TREE-MAPPING Spec 2 revises [PROP-037](../modules/vibe-cli/PROP-037-tree-tui.md) so the TUI sits on this crate:
every TUI command becomes an **Action** with an address in group `vibe.tree` (e.g.
`action://vibe.tree/copy.markdown`, `action://vibe.tree/sort`, `action://vibe.tree/mode.set`), a
name + description (feeding both the footer/menus and Search Everywhere), a typed param schema, and a
typed enablement over a `TreeCtx` snapshot. The F-key map (PROP-037 §5) becomes a `keymap` binding
key → address. F1 opens Search Everywhere with the three providers (§6). The four-layer MVC of
PROP-037 is preserved and *sharpened*: the Model becomes the serialisable `ModelView` source (AIUI-
ready), the Controller becomes the keymap + `invoke`, the View becomes the one normalized renderer +
Theme. `ComingSoon` stays for genuinely-unbuilt features; Search Everywhere is **promoted** from its
reserved stub to a shipped feature. @doc/done

## 10. What Spec 1 ratifies {#contract-pointer}

##CONTRACT-POINTER PROP-039 will carry one granular addressable REQ per Δ (Δ1–Δ16), organised by the §2 modules, each
cited by the code via `specmark`. The human-legibility gate (D7) and the enumerable-registry golden
(Δ12) are floor gates. This design-doc is the lore that explains those REQs; the two-way links are
kept per spec-genres. No open owner-decisions remain (RP1–RP5 resolved,
`legacy-spec/research/ACTION-SYSTEM-RESEARCH-PLAN-v0.1.md#review-points`); the address form is `action://`
and the crate is `vibe-actions`. @doc/done
