# VibeTerm UI-Architecture Research Plan v0.1 — the AI-UI-ready, universal GUI stack

**Status: PLANNED (2026-07-19)** — awaiting owner review of the §11 review points before Phase 1
executes. A **research campaign** (research → design → execution): its output is a **findings
document** that feeds a *separate* design-doc session, which feeds *separate* contract sessions
(PROP-044 + siblings), which feed *separate* implementation sessions. This plan is modelled on its
sibling [`ACTION-SYSTEM-RESEARCH-PLAN-v0.1`](../../spec/research/ACTION-SYSTEM-RESEARCH-PLAN-v0.1.md), the house genre for
exactly this kind of study.

> **Read-first / boot.** Executed cold. Boot the normal way (`CLAUDE.md` → `spec/boot/INDEX.md` → its
> files → `spec/WAL.md` → `CONTINUE.md`), then read this whole file. It is self-contained: the thesis,
> the clean-room firewall, the sources, the question-driven agenda, the deliverables, the phases, the
> predictions, the risks, and the open owner-decisions are all here. **Prerequisite reading (our own
> specs, no firewall):** PROP-039 (`vibe-actions`) + [`spec/design/action-system.md`](../../spec/design/action-system.md);
> PROP-037 (the `vibe tree` TUI surface) + [`spec/design/tui-visual-language.md`](../../spec/design/tui-visual-language.md);
> PROP-042 (the AIUI observation planes); PROP-044 (the VibeTerm shell contract, the first consumer).

---

## 0. Why this exists — the strategic thesis {#why}

We are building the **VibeTerm shell** (PROP-044): a multi-tab / multi-window terminal workspace in
Electron + SolidJS. Before we build it we must settle its **architecture as a product in its own
right** — one that is (a) **AI-UI-ready by construction**: an AI drives *any* function through a
**semantic API, never CDP or screen-scraping**, as well as or better than a human; (b) **universal**:
the same methodology `vibe tree` already proved, ported to the GUI; and (c) **evolvable**: a stack we
can grow, worked out deliberately rather than grabbed first-come.

**The deeper thesis — vibevm already designed this methodology to be universal.** The action system
(PROP-039) is a **render-free semantic core**: every capability is an addressable `action://<group>/<name>`
Action; the **Model + the Action Registry *are* the interface**; a visual **Surface** is one optional
projection; and the **headless AIUI surface is the *reference*** — an AI invokes by address and reads a
serialisable **`ModelView`**, never pixels (`spec://vibevm/research/ACTION-SYSTEM-RESEARCH-PLAN#mandate`,
PROP-039 §11.3 `#aiui`). PROP-037 shows the surface side: a four-layer MVC, a component library, and a
**Theme-as-CSS** visual language (semantic role-tokens, one source **projected**). None of this is
TUI-specific by accident — it was built so the TUI, a web UI, an IDE plugin, and the AIUI are all
projections of one core. **This research works out how that universal methodology becomes the VibeTerm
GUI stack** — the whole vertical the owner named: *entities → MVC/state → actions/AIUI → Search
Everywhere → i18n → visual language → design system.*

**The crux question — RESOLVED by the owner (2026-07-19).** `vibe-actions` is Rust and render-free; the
shell is TypeScript / SolidJS / Electron. Rather than couple them through a shared contract, **vibeterm
carries its own full, self-contained adaptation** of the methodology under `spec/modules/vibeterm/` —
able to live as a standalone project and even spin out of vibevm (RP-A, RP-D). It **ports** the
methodology (addressable `action://` actions, the Registry, typed context + pure enablement, `invoke`,
the serialisable `ModelView`, the AIUI-as-reference, Search Everywhere, i18n, design-tokens) and
re-expresses it in TS, keeping a methodologically-compatible grammar for coherence — but depending on
nothing vibevm-internal. Making that self-contained re-expression *right* across the whole vertical is
the heart of the study.

**Guard-rails** (keep an open-ended study tractable):
1. **Port-first, invent-second.** The methodology exists and is universal by design; the default is to
   **port and adapt** it, and to *invent* only where the GUI/multi-surface world has no analogue (the
   TUI's colour *tiers*; multi-window/pane layout; live theming). Part of the research's job is to name
   exactly what ports verbatim, what adapts, and what is genuinely new.
2. **Clean-room (§1)** for any external source. We take ideas, never code or expression.
3. **The design-system is first-class, not decoration** — the owner named "визуальный язык и
   дизайн-система" as the top of the stack; it is a full deliverable (RP-C), the GUI twin of
   `tui-visual-language.md`.

## 0.1 Mandate & acceptance — LIVING (owner, 2026-07-19) {#mandate}

Verbatim essentials the owner set for this work:

> «терминал должен быть изначально **AI UI Ready**. AI должен в любой момент мочь управлять любой
> функцией терминала не хуже чем человек — **не по CDP, а используя именно семантический API**» ·
> «посмотри на методологию, которую мы выбирали для Vibe Tree и аккуратно перенеси туда всё, что
> связано с виджетами, уровнями абстракции» · «**ЛЮБАЯ АРХИТЕКТУРНАЯ ВЕЩЬ КОТОРУЮ ТЫ ДЕЛАЕШЬ должна
> учитывать будущее наличие AI UI поверхности**» · «У нас в терминале будет точно такой же **Search
> Everywhere, экшены**, и так далее. Перенеси методологию, она тоже разрабатывалась универсально» ·
> «проработать и правильно записывать и улучшать **весь стек — начиная с сущностей, всевозможных
> паттернов MVC и состояния, и заканчивая визуальным языком и дизайн-системой**» · «сделать **отдельное
> исследование** про то, как это вообще должно делаться … написать отдельный план … выполнить его, и
> на основании этого понимания потом построить всю архитектуру» · «Нейросеть лучше работает когда
> делает **исследование, проектирование и исполнение**, а не ваншотает всё за раз» · «Правильная
> архитектура, которую можно развивать — это тоже своего рода **продукт**, который надо выработать».

**No simplification; deliberate phases.** Each deliverable is the maximal, proper version. The cadence
is explicit: **research (this plan) → design (a design-doc) → contracts → execution** — never one-shot.
Each phase is a safe cold-stop.

**Acceptance — the definition of done for the RESEARCH.** A single, self-contained **findings document**
that (1) extracts the universal methodology from our own specs and states precisely what ports / adapts /
is new for the GUI; (2) settles the crux (RP-A) with a recommended Rust↔TS contract; (3) covers the full
vertical — entities, MVC/state, the AI-UI surface (semantic, not CDP), Search Everywhere, i18n, and the
**visual language + design system** (the GUI twin of the palette-token/projection model); (4) closes with
numbered **architecture deltas**, each naming a prospective contract REQ (PROP-044 or a sibling), so the
design-doc and contracts can be authored from the findings alone. The AI-UI surface itself is **not built**
by this research — but the architecture it recommends must make that surface a thin peer-client adapter
that "already works because the core owes rendering nothing."

**Conformance direction — RESOLVED (owner, 2026-07-19, sharpens RP-A).** "Self-contained, no
**build**-dependency on vibevm-internal" (RP-A) does **not** mean "no shared grammar." A Rust `vibe-actions`
and a TS `vibeterm-core` that each re-derive the address grammar, the `ModelView` shape, and the AIUI verbs
independently **will diverge** — different addresses, different `ModelView`, two incompatible AIUIs (the
R5 two-core-drift failure mode). The resolution is to separate two notions the original RP-A conflated:
**(a) build-dependency** — which stays none, correctly — from **(b) identity-grammar conformance** — which
is now first-class. The findings must recommend an **identity-grammar spec** (the address grammar, the
`ModelView` schema, the AIUI verb set, the Search-Everywhere provider contract) carried as a **normative
document**, against which **both** implementations validate via a **conformance golden in CI**. No crate
imports the other; the grammar is provenance, verified by machine. This is the load-bearing refinement
that makes "methodologically-compatible grammar" (RP-A's own words) **testable** rather than aspirational.

**AI-Native-ready output (owner, 2026-07-19).** Every deliverable this research seeds — the findings'
numbered deltas, and downstream the design-doc and the vibeterm PROP family — is authored to land
**directly** under the two AI-Native disciplines the host project already runs, with no retrofit:

- **AI-Native Rust** (`spec://org.vibevm.ai-native/core-ai-native`) governs the **`vibe-actions` side** of
  the conformance contract: each REQ is a granular addressable anchor cited by code via `specmark::scope!`;
  the conformance golden is a **characterization golden** over the registry/`ModelView`/AIUI surface.
- **AI-Native TypeScript** (the `typescript-ai-native` stack) governs the **`vibeterm-core` side**: the
  engine's pure cells (tab registry, pane-layout maths, session model, protocol codec) carry single
  registration points, no sibling-cell coupling, strict `tsconfig`, branded types, `Result` errors,
  `vitest` cell tests, and a `tsc`/`specmap` gate; the **`#no-render-dep` invariant is a dependency-boundary
  lint on the floor from day one** (the TS twin of the Rust gate).
- The **identity-grammar spec** itself is the seam where the two disciplines meet: it is the one document
  both sides conform to and the one golden both CI floors run. Authoring the deltas as REQ-ready (each
  carrying a prospective `spec://vibeterm/…#…` anchor and a falsifiable acceptance) is therefore not a
  late styling pass — it is how the research output is shaped from the first delta.

## 0.2 Execution ledger (running) {#ledger}

_A running record so a cold resume continues without loss; `git log` is the authoritative history._

- **2026-07-19 — PLANNED.** This plan authored from a full read of `vibe-cli` + `vibe-actions` + the
  design lore + the house template. Awaiting owner review of §11 before Phase 1. No study commits yet.
- **2026-07-19 — PLAN SHARPENED.** A critical re-read surfaced a structural conflict: the plan declared
  "open research → design → contracts" while PROP-044 + `task.md` §3 had already frozen D0–D7 as **binding**
  (stack, transport shape, i18n/theming presence, tab model, tear-off). Left as-is, research would ratify
  frozen decisions rather than earn its answers. Resolution landed in this revision: §0.3 separates the
  **frozen axes** (constraints research works within) from the **open questions** (what research actually
  earns); §3 RQs are re-posed in the mode "how exactly to realise X within the frozen stack, and what new
  questions does that raise," not "which stack." RP-A is sharpened to **conformance** (identity-grammar
  spec + CI golden — §0.1). New RQs added: capability/security surface for an AI peer, AIUI-plane
  unification, the transport-contract form. Predictions sharpened to falsifiable + measured, with an
  AI-UI evaluation-matrix deliverable. Output framed AI-Native-ready (Rust + TS). No study commits yet;
  this is a plan revision, awaiting owner review of §11 before Phase 1.

---

## 0.3 Frozen axes vs open questions — the scope contract {#frozen-vs-open}

This research is **constrained**, not open-ended. The owner has already decided a set of load-bearing axes
(recorded as PROP-044 D0–D7 + RP-A/D + the AI-UI mandate); they are **constraints this research works
within**, not hypotheses it earns. What the research **does** earn is the realisation detail of each frozen
axis, plus the genuinely open questions the frozen axes raise. Naming both explicitly is what keeps the
study from collapsing into a post-hoc ratification of decisions already made.

### Frozen axes — constraints (do not re-litigate without naming the trigger) {#frozen}

- **AI-UI-Ready by construction** (owner). Control is semantic `invoke`; CDP is observation-only (PROP-042).
- **Self-contained & detachable** (RP-A + RP-D): the full adapted system lives under `spec/modules/vibeterm/`
  (+ `apps/vibeterm/`), a PROP family, no build-dep on vibevm-internal. **Conformance to a shared
  identity-grammar is in-scope** (§0.1); a build-dep is not.
- **Stack — Solid + Vite + Tailwind v4 + Kobalte + strict TS** (PROP-044 D4). Terminal-view pages stay lean
  vanilla TS + xterm.js (N tabs = N light renderers).
- **Shell = default visible `vibe term`** (D2); headless/`--control` stays bare single-view, unchanged (§8).
- **Tabs engine — each tab = own `WebContentsView` + main-owned pty keyed by `TabId`**; reparent preserves
  live state (D0, empirically verified). Split ceiling = 2 in M1 (D3).
- **Chrome↔engine protocol — transport-agnostic, sidecar-ready** (D5): Electron IPC via a typed preload
  bridge now; the **form** of the contract (codec, versioning, stream/RPC, consistency) is an open question.
- **i18n from the start** (D6): address-keyed catalogue, `{value, original_en}`, legibility gate, en + ru,
  live locale switch. The TS **mechanism** (Fluent runtime, reactive catalogue) is an open question.
- **Live theming via design tokens** (D7): CSS custom properties, live switch, components reference roles
  never hex, two launch themes (dark purple + Anthropic). The **token architecture** (Tailwind @theme
  integration, Kobalte, a11y modes, icons, spacing-scale) is an open question.
- **TS-core now, full AI-Native gate later** (D1): typed TS cells + `tsc` + `vitest` from the start;
  `eslint`/`conform`/`specmap` wiring deferred until the shell stabilises.
- **AI-Native-ready output** (§0.1): the deltas land as REQ-ready anchors under AI-Native Rust
  (`vibe-actions` side) and AI-Native TS (`vibeterm-core` side), with a conformance golden both floors run.

### Open questions — what this research EARNS {#open}

- The **shape of the action/AIUI core in TS** (registry, typed context, pure enablement, `invoke`,
  `ModelView`, the Surface seam) — its concrete cell decomposition within the frozen stack and the
  `#no-render-dep` boundary lint.
- The **`ModelView` schema for a window→tab→pane tree** (fields, deltas vs re-resolution, per-pane focus,
  accessibility projection) — richer than the TUI's single-screen snapshot.
- The **identity-grammar conformance contract** between Rust `vibe-actions` and TS `vibeterm-core` (§0.1).
- The **capability/permission surface** for an AI / networked caller (Dangerous actions, caller provenance,
  scope-REFUSE, audit) — inert in the TUI, load-bearing once AI is a peer.
- How the new **semantic AIUI relates to PROP-042's three planes** (render / terminal / model) — extension,
  fourth plane, or peer; per-tab vs per-process.
- The **transport-contract form** within "transport-agnostic" — codec, versioning, stream vs RPC,
  backpressure, ordering, consistency between main-engine and Solid-store.
- The **state-container model** reconciling Solid fine-grained reactivity with the immutable-`ModelView` /
  re-resolution discipline, without a double source of truth.
- The **design-system specifics** within "design tokens + two themes" (Tailwind v4 `@theme`, Kobalte
  theming, accessibility modes, SVG icon vocabulary, spacing-scale).
- The **AI-UI evaluation criterion** — how "not worse than a human" is measured (a task matrix driven both
  ways: `invoke` vs human, compared on success / latency / observability).
- The **GUI-only inventory** with no TUI analogue (drag-and-drop, multi-pane focus, clipboard, OS
  integration, DPI/scale, the accessibility tree) — each a candidate architecture delta.
- The **external comparative** (clean-room, docs-first): how VS Code / Zed / Warp / Raycast expose control
  (semantic vs CDP vs proprietary) and how Radix / Tailwind / Style-Dictionary model design tokens.

The research **earns** the open questions and **details** the realisation of the frozen axes. It does not
reopen the axes. A finding that a frozen axis is wrong stops the study and surfaces a named trigger to the
owner (per the decision-record discipline) — it is not silently re-decided inside the findings.

---

## 1. Clean-room discipline — GATING for external sources {#clean-room}

The **internal extraction** (our own PROP-039/037/042/044 + design docs) needs no firewall — it is our
own material. The **external comparative** (§2.2) does: modern GUI apps' action systems, command
palettes, design-token systems, and AI surfaces are **inspiration-only, never a code source** — the
standing repo posture (the `eth-sri` directive; the action-system clean-room). Method: **READ or
observe to understand the approach; then design STRUCTURALLY DIFFERENT, in our own expression.**
Identical *behaviour* is fine; borrowed *expression* is not. Rule 1 governs; nothing is attributed to a
tool.

**Licence caution, per source.** VS Code is MIT (permissive); its source may be read under the firewall.
**Zed is GPL-3.0** and **Warp is closed-source** — for these, prefer **public documentation, blog posts,
and observed product behaviour** over source reading; do not read GPL/closed source for this study unless
the owner explicitly clears it (RP-E). The findings document records **no** legal rationale — the posture
is clean-room, full stop.

**The firewall — separated sessions.** (a) this STUDY reads sources and writes only the findings doc;
(b) the DESIGN-DOC session authors from the findings only; (c) the CONTRACT sessions author from our own
specs; (d) IMPLEMENT sessions build from the contracts. The findings document is the only thing that
crosses (a)→(b/c/d).

---

## 2. What we study & where it lives {#sources}

### 2.1 Internal — our own methodology (primary, no firewall) {#sources-internal}

| Concern | Where |
|---|---|
| The render-free action core, Registry, typed Ctx + pure enablement, `invoke`, capabilities, i18n, Search Everywhere provider model, Surface + serialisable `ModelView`, the headless AIUI | **PROP-039** (`spec://vibevm/modules/vibe-actions/PROP-039`) + design-doc [`action-system.md`](../../spec/design/action-system.md) |
| The four-layer MVC surface; component library; the Theme; the action catalogue; keymap-binds-addresses; i18n-is-real | **PROP-037** §1, §2, §13 |
| The visual language: semantic palette role-tokens, glyph vocabulary, tiers, **degradation-as-projection**, window aesthetics, spacing/rhythm | **[`tui-visual-language.md`](../../spec/design/tui-visual-language.md)** + PROP-037 §2.2 |
| The three observation planes (render / terminal / model), the `ModelView` projection verb, CDP-is-observation-only | **PROP-042** §1–§5 |
| The shell we are architecting: tab model, panes, windows, the chrome↔engine seam (D5), i18n (D6), theming (D7) | **PROP-044** (VibeTerm shell) |
| The foton base stack (Solid + Vite + Tailwind v4 + Kobalte), reused for the chrome | `C:\Users\olegc\git\foton\packages\desktop` (owner's other project; base stack only) |

### 2.2 External — comparative (secondary, clean-room; docs/behaviour-first) {#sources-external}

Study **approaches**, not implementations. Prefer public docs + observed behaviour; read source only
for permissive VS Code, under the firewall (§1).

| Concern | Subjects |
|---|---|
| GUI command palette + action/command system (the AI-UI-relevant part: is behaviour addressable and programmatically invocable?) | VS Code (MIT, source OK); Zed (GPL — docs/behaviour only); Warp terminal (closed — docs/behaviour only); Raycast (command model — docs/behaviour) |
| Design-token / theming systems (the GUI twin of our palette-tokens + projection) | Radix Themes / Radix Colors; Tailwind v4 theme layer; Style-Dictionary / W3C Design-Tokens format; VS Code's theme-token model |
| Multi-window / multi-view desktop UIs + AI/agent surfaces | Warp's agent/AI surface (behaviour); Electron `WebContentsView` docs; how any of these expose (or fail to expose) a semantic control API vs pixels |
| SolidJS state / MVVM patterns for a render-free model | Solid `createStore`/signals patterns; the foton tiered-context design (already reconned) |

---

## 3. Study questions — grounded in OUR port {#questions}

Each RQ carries a preliminary hypothesis to confirm or refute. **Scope (§0.3):** RQ1–RQ11 are the
original study questions (several sharpened in this revision); RQ12–RQ17 are added where the §0.1
conformance refinement, the §0.3 frozen/open split, and a critical re-read exposed open questions the
original plan had not posed. Every RQ is posed within the frozen axes — it asks *how to realise X under
the frozen stack*, or names a genuinely open question — it does not reopen an axis.

- **RQ1 — Universality of the action core.** Which PROP-039 concerns are surface/language-neutral, and
  how does each port to TS? *Hypothesis: address grammar, Action value, Registry laws, typed
  context + pure enablement, `invoke`, capabilities, the serialisable `ModelView`, the Surface seam, the
  Search-Everywhere provider model, and address-keyed i18n are ALL expressible in TS with zero rendering
  deps — the `#no-render-dep` invariant holds in TypeScript.*
- **RQ2 — Entities & domain model.** The shell's entity set (Window, Tab, Pane, Session/Terminal,
  Profile, Theme, Locale, Action, Provider) — identity/addressing, hierarchy, and the `action://vibeterm/*`
  catalogue. *Hypothesis: stable ids (branded `TabId`, etc.) + the `action://` grammar cover addressing
  with no new scheme.*
- **RQ3 — MVC/state in TS/Solid: the state-container reconciliation (sharpened).** The naive framing
  ("Solid's store is an excellent `ModelView` host; MVVM and our MVC coincide") hides a real tension.
  PROP-039 §3.2 requires the resolved snapshot to be **immutable** and change to come by **re-resolution**
  — but Solid's `createStore` is a **fine-grained mutable** store optimised for in-place path updates. The
  question this research earns: **what is the source of truth, and how do the immutable-re-resolution
  discipline and Solid's mutable reactivity coexist without a double source of truth** and without a
  re-render storm on a large window→tab→pane `ModelView`? Candidate shape to confirm or refute: the
  render-free engine owns the authoritative immutable `ModelView`; a Solid store is a **one-way projection**
  (engine → chrome) rebuilt on re-resolution; chrome-local **ephemeral** state only (hover, drag-ghost,
  in-flight keystrokes) lives outside the engine and never crosses the seam. *Hypothesis: the projection
  model holds; the chrome never mutates the `ModelView` directly — it dispatches actions (the AI-UI verb)
  and re-resolves. Falsified if a class of UI state cannot be expressed without chrome-side mutation of the
  model, or if the projection rebuild is too costly at the tab counts users reach.*
- **RQ4 — The AI-UI surface (semantic, NOT CDP).** The verbs (`invoke` / `state` / `list_actions` /
  `search`), the `ModelView` **schema for a multi-window/tab/pane shell**, how it attaches as a peer of
  the Solid chrome, and how it relates to PROP-042's planes and the Rust `vibe aiui`. *Hypothesis: the
  shell's `ModelView` is a window→tab→pane tree; the AI-UI surface is the same four verbs; CDP stays
  observation-only (PROP-042 `#render-plane`), control is `invoke`.*
- **RQ5 — Search Everywhere for the GUI.** The same provider model (two-phase enumerate→resolve, one
  ranker, normalized rows); providers at ship (sessions/terminals, actions, profiles…); GUI rendering
  (cmdk/Kobalte). *Hypothesis: the PROP-039 §10 engine is surface-neutral; only the row renderer changes.*
- **RQ6 — i18n for TS/GUI.** Address-keyed catalogue in TS, `{value, original_en}`, the legibility gate,
  en + ru, live locale switch. *Hypothesis: PROP-039 §8 ports directly; Fluent has a JS runtime.*
- **RQ7 — Visual language & design system (the GUI twin of the Theme; split into sub-questions).** The
  frozen axis (D7) fixes "design tokens + two themes + live switch + role-not-hex"; what this research
  earns is the **token architecture**. The TUI's glyph vocabulary (`▾▸●○╭╮`) does **not** port to a GUI, so
  the design system has genuinely new surface beyond a port. Sub-questions, each its own prospective REQ:
  - **RQ7a — Tailwind v4 `@theme` vs our design tokens.** Tailwind v4 already ships a theme layer (`@theme`
    with CSS-variable generation); does our token system **sit above** it, **replace** it, or **layer
    through** it? *Hypothesis: our semantic role-tokens resolve to CSS custom properties that Tailwind's
    `@theme` consumes — one source, Tailwind as the utility consumer, not a competing token namespace.*
  - **RQ7b — Kobalte theming unification.** Kobalte exposes its primitives via `data-*` attributes and its
    own CSS-variable convention; how do its parts adopt **our** role-tokens so a Kobalte dialog and our
    `ui::Window` read as one system? *Hypothesis: a thin Kobalte-theme adapter maps our roles onto
    Kobalte's expected variables; components never reach past our tokens.*
  - **RQ7c — Accessibility modes (contrast / reduced-motion / density).** The TUI's tier-degradation has no
    GUI analogue; the GUI analogue is **a11y/density modes**. Representation: CSS media queries
    (`prefers-reduced-motion`, `prefers-contrast`), explicit `data-` attributes, or token-variants? *Hypothesis:
    a token-variant layer (a theme × a mode matrix) driven by both media queries and explicit user choice.*
  - **RQ7d — The GUI icon vocabulary.** The TUI glyph table does not carry over; a GUI needs an **SVG icon
    system** (one source, role-coloured, theme-aware, a11y-labelled). *Hypothesis: a small owned SVG icon
    set consumed through a single `<Icon name=role>` primitive; icons reference roles, never raw colour.*
  - **RQ7e — Spacing & rhythm scale.** The TUI's `PAD_X/PAD_Y/GUTTER` constants become a GUI **spacing
    scale**; is it Tailwind's spacing scale, a custom scale, or both? *Hypothesis: one owned spacing scale
    exposed as tokens; Tailwind utilities reference it; the §6 rhythm rules (centred rows, interior padding,
    group gutter) port as layout primitives, not magic numbers.*
  - *Umbrella hypothesis: the TUI's "one Theme projected across tiers" becomes "one token set projected
    across themes/modes"; the tier-degradation concept has no GUI analogue and is replaced by theme × a11y
    modes; the design system is a first-class deliverable (RP-C), not decoration.*
- **RQ8 — External comparative (clean-room, docs-first).** How do VS Code / Zed / Warp / Raycast model
  the palette + actions; how do Radix/Tailwind/Style-Dictionary model design tokens; do any expose a
  render-free, addressable, **AI-drivable** action core with a serialisable model as the reference
  surface? *Hypothesis: none does exactly this — our approach leads; we still adopt their proven
  mechanisms (token scales, palette UX, keyboard model).*
- **RQ9 — Portability adaptation.** Where the Rust/TUI methodology needs GUI/multi-surface adaptation:
  no colour tiers; a window/tab/pane `ModelView` tree (richer than a single-screen snapshot); live
  theming vs env-detected; the Electron process model (chrome renderer vs per-tab `WebContentsView` vs
  main-process engine). *Hypothesis: adaptations are additive; the core methodology is unchanged.*
- **RQ10 — Cross-cutting.** Capabilities/permissions (AI + future networked callers), the
  transport-agnostic **sidecar-ready** protocol (PROP-044 D5), testability (the headless AIUI as the
  golden reference), and how the whole stack stays evolvable. *Hypothesis: PROP-039 §7.2 capabilities +
  PROP-044 D5 already frame this.*
- **RQ11 — Self-containment & detachability.** What must vibeterm **own** vs. borrow so it builds and
  specs standalone and can spin out of vibevm? Which vibevm concepts are ported as owned re-expressions
  vs. depended upon? *Hypothesis: everything the shell needs (actions/AIUI/`ModelView`/SE/i18n/design
  system) is re-expressed under `spec/modules/vibeterm/`; the only ties to vibevm are methodological
  provenance (citations), never build/spec dependencies — with the identity-grammar (RQ12) the **one**
  shared normative surface, conformance-tested not build-depended.*

- **RQ12 — Conformance: the identity-grammar contract (new; sharpens RP-A).** What is the **minimum**
  shared surface the Rust `vibe-actions` and the TS `vibeterm-core` must agree on so their two AIUIs are
  one surface, while neither imports the other? Candidate: an **identity-grammar spec** — the `action://`
  grammar, the `ModelView` schema (incl. the window→tab→pane tree), the AIUI verb set
  (`invoke`/`state`/`list_actions`/`search`), the Search-Everywhere provider contract, the i18n key scheme
  — carried as a normative document both implementations validate via a **conformance golden in CI**.
  *Hypothesis: the grammar is small enough to specify without coupling, rich enough that conformant
  implementations interoperate at the AIUI surface; the conformance golden is a characterization golden on
  both floors. Falsified if a load-bearing behaviour cannot be expressed without a build-dep, or if the
  two implementations cannot be kept conformant by a CI check.*

- **RQ13 — Capability / permission surface for an AI / networked caller (new).** PROP-039 §7.2's
  `Capability` (`Safe`/`Mutating`/`Dangerous`) is **inert in the trusted local TUI**. Once the AI is a peer
  client with the same `invoke` — and later a networked sidecar — "the AI may do anything a human may"
  meets "a `Dangerous` action must not fire without consent." This research earns the model: how is a
  `Dangerous` action gated for a non-human caller (prompt-on-dangerous? allowlist? a confirmation action
  the AI must invoke first?), how is the **caller identified and scoped** (local AI vs sidecar vs remote),
  and where does the scope-REFUSE live (it must refuse scope escalation as an error, never a warning — the
  `secrets-hygiene` posture)? *Hypothesis: PROP-039 §7.2 capabilities + an explicit caller-identity +
  granted-scope context + a prompt-on-`Dangerous` flow for non-trusted callers; the engine never trusts
  the caller's self-reported scope. Falsified if a safe, non-bypassable model cannot be sketched.*

- **RQ14 — AIUI-plane unification (new).** Today `vibe aiui` (PROP-042) drives **three planes**: the render
  plane (CDP, observation-only), the terminal plane (`--control` HTTP over a live single view), and the
  model plane (`vibe aiui state` → `ModelView`). The new semantic AIUI (`invoke` over the shell's action
  core) is a fourth control surface. The research earns the relationship: is semantic-`invoke` an
  **extension of the model plane**, a **fourth peer plane**, or a **replacement** of the terminal-plane
  control? How do the per-tab AIUI scope (`TabId`) and the per-process `--control` discovery coexist? And
  how does a single `vibe aiui` CLI address both the shell (invoke `vibeterm/*`) and the hosted `vibe tree`
  (invoke `vibe.tree/*`) without forking the surface? *Hypothesis: the four verbs fold onto the model
  plane; the terminal plane stays as the legacy single-view observation path (frozen, PROP-044 §8); CDP
  stays observation-only. Falsified if the unification loses a capability the three-plane model has.*

- **RQ15 — Transport-contract form (new; details the frozen D5).** "Transport-agnostic, sidecar-ready,
  no Electron types" (D5) is frozen; its **form** is open. The research earns: the **codec** (a TS
  discriminated union + hand-rolled codec? a JSON-Schema? an IDL — protobuf/cap'n'proto — generating both
  sides?), the **versioning** (contract-semver vs per-message), the **exchange model** (event-stream for
  `opened`/`closed`/`moved`/`active-changed` vs request-response for commands), **backpressure** and
  **ordering** across windows, and the **consistency model** between the authoritative main engine and the
  Solid-store projection (RQ3). *Hypothesis: a TS discriminated union as the contract source, a generated
  JSON-Schema for cross-language/conformance use, contract-semver, a hybrid event+RPC exchange, and a
  single-writer (main) consistency model with the chrome as a one-way projection. Falsified if a
  transport requirement forces Electron types back into the contract.*

- **RQ16 — The GUI-only inventory (new).** Each item below has **no TUI analogue** and is therefore a
  candidate *invent*, not a port; the research catalogues them and assigns each a prospective REQ so none
  is discovered late: drag-and-drop (tab reorder, tear-off drag-back into a window); keyboard focus
  management across multiple panes (a Tab-order model richer than PROP-037 §5.4's single-screen order);
  clipboard semantics per platform; OS integration (notifications, dock/taskbar badges, jump-lists,
  window-state restore); DPI/scale and font rendering under xterm.js; the **accessibility tree** (ARIA for
  the chrome; xterm.js's own a11y for the terminal views). *Hypothesis: every item resolves as an additive
  GUI delta under the frozen axes; none reopens an axis. Falsified if any item forces a change to a frozen
  axis (e.g. a11y forces a non-token colour path).*

- **RQ17 — The AI-UI evaluation criterion (new; makes "not worse than a human" measurable).** The owner's
  non-negotiable is that the AI drives any function **as well as or better than a human**. As written that
  is a slogan. The research earns an **evaluation matrix**: a representative task set over the shell (open
  / select / switch / split / tear-off / search / theme-switch / locale-switch), each driven **both** ways
  — by a human (mouse + keyboard) and by the AIUI verbs — and compared on **success rate**, **latency**,
  and **observability** (can the AI assert the resulting state?). This matrix is a findings-deliverable and
  the substrate of the "we lead" prediction (§9). *Hypothesis: on the matrix the AI path reaches parity on
  success and observability and trades latency within an acceptable bound; cases where it cannot are named
  design obligations, not silent gaps.*

Written first, on purpose (anti-anchoring): enumerate what an **AI-UI-ready, universal GUI architecture**
needs from first principles + our own methodology, so the external study confirms or refutes a stated
hypothesis rather than paraphrasing other apps. Phase 1 writes this into the findings doc; its claims
become the predictions (§9). The concept inventory to fill from our specs + first principles first:

1. **Entities & addressing** — the domain objects; stable ids; the `action://` behaviour address.
2. **The render-free core** — Model + Registry as the interface; the `#no-render-dep` invariant in TS.
3. **Actions** — value shape, presentation (mandatory name+description), typed params, pure enablement,
   `invoke`, capability.
4. **State / `ModelView`** — the serialisable projection; the multi-window/tab/pane tree; deltas vs
   re-resolution; the chrome as a projection; the AI-UI surface as a peer client.
5. **The AI-UI surface** — the four verbs; the semantic-not-CDP law; the peer-client attach point.
6. **Search Everywhere** — the provider model for the GUI; providers at ship.
7. **i18n** — address-keyed catalogue; `{value, original_en}`; legibility gate; en/ru; live switch.
8. **Visual language & design system** — semantic design tokens; themes as token sets; one-source
   projection; live switching; the component library; visual grammar; accessibility modes.
9. **Keymap / input** — keys bind to addresses; a pure resolver; chords in the adapter.
10. **Surfaces as adapters** — the chrome (Solid), the terminal views (vanilla xterm), the AI-UI, a
    future sidecar — all peers of one engine.
11. **Transport** — the sidecar-ready contract; Electron IPC now; the contract **form** (RQ15).
12. **Testability & evolvability** — the headless AIUI as the golden; how the stack grows without churn.
13. **Identity-grammar conformance (RQ12)** — the one normative surface the Rust `vibe-actions` and the TS
    `vibeterm-core` share; the conformance golden both CI floors run; provenance, not a build-dep.
14. **Capability / permission surface (RQ13)** — `Dangerous`-action gating, caller identity + granted scope,
    scope-REFUSE, audit — the security model an AI/networked peer makes load-bearing.
15. **The GUI-only inventory (RQ16)** — drag-and-drop, multi-pane focus, clipboard, OS integration,
    DPI/scale, the accessibility tree; each a candidate *invent*, catalogued up front.
16. **The AI-UI evaluation criterion (RQ17)** — the task matrix that makes "as well as a human" measurable.

## 5. Part (c) — the pitfall / failure-mode catalogue {#pitfalls}

Starting hypotheses (from knowledge + our specs), validated during the study; each survivor becomes a
design obligation:

- **The CDP trap.** Driving a UI by CDP/DOM/screenshot is brittle, at the wrong abstraction, and never
  "as well as a human." (Our answer: control is `invoke`; CDP is observation-only.)
- **Two cores drift.** A Rust `vibe-actions` and a TS shell core that share no contract diverge —
  different addresses, different `ModelView`, two incompatible AIUIs. (Answer: RP-A — one contract.)
- **Pixel-only capabilities.** A chrome command reachable only from a DOM handler is invisible to the AI.
  (Answer: every capability is a named action; the chrome has no capability the AI lacks.)
- **Hardcoded style.** Hex literals in components (the foton anti-pattern reconned) — no live switch, no
  second theme. (Answer: design tokens; components reference roles.)
- **A render-coupled model.** Solid/DOM/Electron types leaking into the engine kill the AIUI and the
  headless golden. (Answer: the `#no-render-dep` invariant, lint-enforced in TS.)
- **i18n retrofit.** Externalising strings after the UI grows is expensive and misses strings. (Answer:
  address-keyed catalogue + legibility gate from the start.)
- **Design-system-as-afterthought.** A component zoo with no token discipline. (Answer: the design
  system is a first-class deliverable, the GUI twin of `tui-visual-language.md`.)
- **Over-building the universal contract** before one surface ships. (Answer: target the shell as the
  first consumer; the contract is *ready* for more, proven by design not by building adapters.)
- **Two cores drift silently (the no-conformance failure).** A Rust `vibe-actions` and a TS `vibeterm-core`
  that share no grammar diverge — addresses, `ModelView`, AIUI verbs — and the two AIUIs become
  incompatible with no test to catch it. (Answer: RQ12 — an identity-grammar spec + a conformance golden
  in CI; build-dep stays none, conformance is machine-checked.)
- **The capability hole.** An AI peer with the same `invoke` as a human, but no gating, fires `Dangerous`
  actions without consent; a networked caller escalates scope. (Answer: RQ13 — caller identity + granted
  scope + prompt-on-`Dangerous` + scope-REFUSE; the engine never trusts the caller's self-reported scope.)
- **AIUI-plane proliferation.** The legacy three-plane `vibe aiui` (render/terminal/model) plus a new
  semantic-invoke surface fork into two unrelated AIUIs to one app. (Answer: RQ14 — fold the verbs onto one
  plane, keep CDP observation-only, declare the relationship explicitly in the contracts.)
- **Double source of truth.** A Solid mutable store and an immutable-`ModelView` engine both mutated → the
  chrome and the AI disagree about state; optimistic chrome edits race authoritative engine events. (Answer:
  RQ3 — the engine is the single writer; the Solid store is a one-way projection; ephemeral chrome state
  never crosses the seam.)
- **The GUI-only unknowns discovered late.** DnD, multi-pane focus, clipboard, OS integration, DPI, a11y —
  each has no TUI analogue and surfaces mid-build as rework. (Answer: RQ16 — catalogue them up front, each
  a named prospective REQ.)
- **"AI-UI-ready" as an unmeasured slogan.** Without a criterion, the AI surface ships "designed-for" but
  never proven "as well as a human." (Answer: RQ17 — the evaluation matrix; cases the AI cannot do are
  design obligations, not silent gaps.)

## 6. Part (d) — candidate architecture pillars (to be EARNED, not yet normative) {#pillars}

Hypotheses for the design-doc + contracts, listed so the study knows what it is justifying. Each is
AI-UI-first and answers a §5 pitfall.

1. **One language-neutral action/AIUI contract; a TS core conforming to it** (RP-A). Shared address
   grammar, `ModelView` schema, AIUI verbs, Search-Everywhere provider model, i18n key scheme — Rust and
   TS impls both conform, so the two AIUIs are one surface.
2. **Render-free engine, lint-enforced.** The shell's tab registry, pane-layout, session model, and
   protocol codec import no Solid/DOM/Electron types; a dependency-boundary check is on the floor from
   day one.
3. **Every capability is a named `action://vibeterm/*`.** The Solid chrome and the AI both call one
   `invoke`; nothing is pixel-only.
4. **A serialisable `ModelView` (window→tab→pane tree) is the source of truth.** The chrome renders it;
   the AI reads it; events are its deltas; capabilities carry enablement + reason.
5. **The AI-UI surface is a peer client**, the reference; control is semantic (`invoke`), CDP is
   observation-only.
6. **Search Everywhere is the same provider model**, surface-neutral, GUI-rendered.
7. **i18n from the start**, address-keyed, `{value, original_en}`, legibility-gated, en/ru, live.
8. **A first-class design system**: semantic design tokens, themes as token sets, one-source projection,
   live switching, a disciplined Solid/Kobalte component library, a documented visual grammar and
   accessibility modes — the GUI twin of the palette-token/projection model.
9. **Capability-scoped, transport-agnostic, sidecar-ready** (PROP-044 D5): the protocol is the contract;
   Electron IPC is one adapter; a future external state process is another.
10. **Self-contained & detachable** (owner, 2026-07-19). The whole vibeterm system — specs, contracts,
    design-docs, code — lives under `spec/modules/vibeterm/` (+ `apps/vibeterm/`) with **no hard
    dependency on vibevm-internal crates or specs**, so vibeterm can stand alone and spin out
    (specspace-ready). vibevm's methodology is ported as provenance, never a build dependency.
11. **Identity-grammar conformance (RQ12).** A normative identity-grammar spec (address grammar,
    `ModelView` schema, AIUI verbs, SE provider contract, i18n key scheme) that both the Rust
    `vibe-actions` and the TS `vibeterm-core` validate against a CI conformance golden — provenance, not a
    build-dep; the load-bearing answer to two-core drift (R5).
12. **Capability-scoped AI / networked callers (RQ13).** A non-bypassable capability + caller-identity +
    granted-scope model with prompt-on-`Dangerous` and scope-REFUSE — the security surface the AIUI peer
    makes load-bearing; the engine never trusts the caller's self-reported scope.
13. **One AIUI surface, not two (RQ14).** The semantic verbs fold onto a single plane; the legacy
    `vibe aiui` terminal/model/render planes are declared (extension / fourth / replacement) explicitly;
    CDP stays observation-only.
14. **A measurable AI-UI (RQ17).** The evaluation matrix — a task set driven both by human and by the AIUI
    verbs and compared on success / latency / observability — is a findings deliverable; "as well as a
    human" is demonstrated, not asserted.

> **All pillars are posed within the frozen axes (§0.3).** A pillar does not reopen the stack, the tab
> model, the transport-shape decision, or the presence of i18n/theming — it earns the *realisation* of
> each and the genuinely open questions above. A pillar falsified by the study becomes a named trigger to
> the owner, not a silent redesign.

## 7. Deliverables {#deliverables}

**Only D1 belongs to THIS campaign.** D2–D4 are named for sequencing; each is its own downstream
campaign.

- **D1 — the findings document** (this campaign's output). Home `research/vibeterm/` (comparative-research
  genre for the external part): the §4 design-space map (incl. the §0.3 frozen/open split as its framing);
  the internal methodology extraction (what ports / adapts / is new); the §5 pitfalls validated; the
  external comparative (two-way gaps); the §6 pillars as **numbered architecture deltas** each naming a
  prospective contract REQ; the §9 predictions checked; the **AI-UI evaluation matrix** (RQ17); the
  **recommended identity-grammar conformance surface** (RQ12) — the minimum the Rust and TS cores share and
  the shape of the conformance golden; a re-fetch/provenance table for external sources. Every delta is
  **REQ-ready** — a prospective `spec://vibeterm/…#…` anchor, a one-line acceptance, and the AI-Native
  discipline (Rust for `vibe-actions`, TS for `vibeterm-core`) it lands under — so D2/D3 author from the
  findings alone with no re-derivation.
- **D2 — the design-doc** (downstream): **vibeterm-owned** (under `spec/modules/vibeterm/`, not the
  shared `spec/design/`) — the VibeTerm UI architecture (entities, MVC/state, the AI-UI surface) and the
  **design system** (the GUI twin of `tui-visual-language.md`), likely split into an architecture-lore
  doc and a design-system doc. Self-contained, so vibeterm can spin out.
- **D3 — the contracts** (downstream): the **vibeterm PROP family** under `spec/modules/vibeterm/` — the
  action/AIUI system, the design system, MVC/state, Search Everywhere, i18n — plus a revised **PROP-044**
  carrying the AI-UI-readiness REQs. All self-contained, with no hard dependency on vibevm-internal crates
  or specs (RP-A/RP-D).
- **D4 — implementation** (downstream): the VibeTerm-shell campaign (VIBETERM-SHELL-PLAN), now **gated
  behind** D1–D3.

## 8. Phases (gated) {#phases}

Every phase ends `bash tools/self-check.sh` green (docs-only phases are trivially green; the floor still
runs) + an execution-ledger entry (§0.2) + a refreshed status line. Any boundary is a safe stop.

- **Phase 0 — framing (NO commits).** Resolve the §11 review points with the owner; lock RQ1–RQ17; write
  the §4 design-space map **under the §0.3 frozen/open framing** (frozen axes are constraints, not
  hypotheses) and turn its claims into §9 predictions; **sketch the AI-UI evaluation matrix (RQ17)** so the
  "we lead / parity" predictions are measurable from the start; confirm which external sources are in-bounds
  (RP-E) and the per-source depth (RP-B). **Gate: the RQs + review points + design-space map reviewed with
  the owner.**
- **Phase 1 — internal methodology extraction.** From our own specs, write the ports/adapts/new table
  for the full vertical (entities, MVC/state, actions/AIUI, SE, i18n, visual language). Commit `docs(research): …`.
- **Phase 2 — external comparative (clean-room, docs-first).** Targeted study of §2.2; two-way gaps.
  Commit.
- **Phase 3 — pitfalls validated (§5) → design obligations.** Commit.
- **Phase 4 — synthesis.** The §6 pillars become numbered deltas each naming a contract REQ; the §9
  predictions checked; the findings REPORT + re-fetch table. **Gate: the findings document is complete
  and self-contained.** Commit. Hands off to the design-doc session.

## 9. Predictions — falsifiable, checked at close {#predictions}

- **P1** — Every PROP-039 core concern (address, registry, typed context, pure enablement, `invoke`,
  capabilities, `ModelView`, Surface, SE provider model, address-keyed i18n) is expressible in TS with
  **zero rendering deps**, enforced by a dependency-boundary lint on the floor. *Falsified if a core
  concern cannot be expressed without importing a Solid/DOM/Electron type into an engine cell, or if the
  boundary lint cannot be made to fail on such an import.*
- **P2** — The serialisable `ModelView` generalises from a single-screen TUI to a **window→tab→pane tree**
  using only re-resolution + event-deltas (no stateful diffing, no second mutation mechanism). *Falsified
  if the multi-surface shell needs a fundamentally different model shape or a beyond-re-resolution
  diffing mechanism.*
- **P3** — A **language-neutral identity-grammar** (address grammar + `ModelView` schema + AIUI verbs + SE
  provider contract + i18n key scheme) is small enough to specify without coupling, and a **conformance
  golden** keeps the Rust `vibe-actions` and the TS `vibeterm-core` conformant in CI. *Falsified if a
  load-bearing behaviour cannot be expressed without a build-dep, or if the two implementations cannot be
  kept conformant by a CI check.*
- **P4** — The immutable-`ModelView`/re-resolution discipline and Solid's fine-grained reactivity
  reconcile via a **one-way projection** with no double source of truth (RQ3). *Falsified if a class of UI
  state forces chrome-side mutation of the model, or if the projection rebuild is too costly at real tab
  counts.*
- **P5** — A safe, non-bypassable **capability + caller-scope** model for an AI/networked peer is
  feasible (prompt-on-`Dangerous`, scope-REFUSE, engine never trusts self-reported scope). *Falsified if
  the model cannot be sketched without a trusted-caller assumption that breaks the AIUI-peer goal.*
- **P6** — "One Theme projected across tiers" maps to **"one token set projected across themes/modes"**;
  the tier-degradation concept has **no** GUI analogue (replaced by theme × a11y/density modes). *Falsified
  if a real GUI tier-analogue is needed.*
- **P7** — On the **AI-UI evaluation matrix (RQ17)**, the AIUI-verb path reaches **parity** with the
  human path on success rate and observability, and trades latency within an acceptable bound; residual
  gaps are named design obligations. *Falsified if the AI path is measurably worse than human on success or
  observability on the matrix, with no design-obligation remedy.*
- **P8** — No mainstream GUI app (VS Code / Zed / Warp / Raycast) exposes a **render-free, addressable,
  AI-drivable** action core with a serialisable `ModelView` as the *reference* surface — our approach
  **leads** — **and** the comparative confirms at least one proven mechanism per open area (token systems,
  palette UX, keyboard model) worth adopting. *Falsified if a competitor already does exactly this, or if
  the comparative yields nothing adoptable.*

## 10. Risks {#risks}

- **R1 — scope creep** (a whole GUI stack + a design system + external apps). Mitigation: port-first;
  the shell is the first-and-only current consumer; essential-first.
- **R2 — anchoring** on the external apps. Mitigation: §4 is written before §2.2; predictions up front.
- **R3 — firewall / licence leak** (Zed GPL, Warp closed). Mitigation: docs/behaviour-first; source only
  for MIT VS Code; the findings doc is the only interface; the Rust/Java/TS boundary forces re-expression.
- **R4 — over-building the universal contract** before one surface ships. Mitigation: target the shell;
  readiness proven by design, not by building adapters.
- **R5 — two-core drift** (Rust `vibe-actions` vs the TS shell core). Mitigation: **RQ12 / §0.1 — an
  identity-grammar conformance spec + a CI conformance golden**; build-dep stays none (RP-A), conformance
  is machine-checked from the first delta. (The original "settle the one shared contract" mitigation
  contradicted RP-A's "no shared contract" — conformance is the reconciliation: shared **grammar**, not
  shared **build-dep**.)
- **R6 — the frozen-axis trap (new).** With D0–D7 frozen, research risks ratifying decisions rather than
  earning answers. Mitigation: §0.3 names frozen vs open explicitly; every RQ is posed within the frozen
  axes; a finding that a frozen axis is wrong surfaces a named trigger to the owner, it is not silently
  re-decided.
- **R7 — the unmeasured AI-UI (new).** "AI-UI-ready by construction" can ship as "designed-for" without
  ever being proven "as well as a human." Mitigation: RQ17 — the evaluation matrix is a Phase-0 sketch and
  a findings deliverable; P7 is the measured prediction.

## 11. Open review points — owner decisions {#review-points}

Resolve these in Phase 0, before Phase 1.

- **RP-A — the action-core relationship (the crux). RESOLVED (owner, 2026-07-19).** vibeterm carries its
  **own full, self-contained adaptation** of the methodology under `spec/modules/vibeterm/` (RP-D) — it
  does **not** depend on the Rust `vibe-actions` crate or a shared cross-language contract module. It
  **ports** the methodology (addressable `action://` actions, the Registry laws, typed context + pure
  enablement, `invoke`, the serialisable `ModelView`, the AIUI-as-reference, the Search-Everywhere provider
  model, address-keyed i18n, design-tokens) and re-expresses it in TS, keeping a methodologically-compatible
  grammar (address form, `ModelView` shape, AIUI verbs) as a deliberate choice for coherence and possible
  future cross-surface bridging — but depending on nothing vibevm-internal. *Why:* the owner wants vibeterm
  self-sufficient, able to live standalone and even spin out of vibevm; a shared-contract dependency would
  tie it back in. *Rejected:* a shared language-neutral contract module (couples vibeterm to vibevm); a
  bridge to the Rust crate (process/language mismatch + coupling). *Revisit:* if a live shared contract with
  the Rust TUI-AIUI is ever needed, add an interop adapter — without making vibeterm depend on vibevm to
  build.
  **Conformance refinement — RESOLVED (owner, 2026-07-19; sharpens RP-A, see §0.1 + RQ12).** The
  "methodologically-compatible grammar" line above is now load-bearing and testable: the shared surface
  is an **identity-grammar spec** (address grammar + `ModelView` schema + AIUI verbs + SE provider
  contract + i18n key scheme) carried as a **normative document**, validated by a **conformance golden in
  CI** on both the Rust `vibe-actions` and the TS `vibeterm-core` sides. This is **not** the rejected
  "shared language-neutral contract module" (that would be a build-dep coupling vibeterm to vibevm) —
  there is no crate import either way; the grammar is provenance the CI keeps honest. The minimum shared
  surface and the golden's shape are themselves an open question this research earns (RQ12); the
  *direction* (identity-grammar + CI golden) is settled.
- **RP-B — research scope & external source set.** *Lean:* internal port is primary; external comparative
  is targeted + docs-first (VS Code, Zed, Warp, Raycast; Radix/Tailwind/Style-Dictionary). Owner confirms
  the set (and whether to include any). **Per-source depth (added):** the comparative is bounded by the
  open questions — VS Code (MIT, source-readable) gets depth on the action/palette model **and on how it
  exposes (or fails to expose) a semantic control API** (RQ8/RQ14); Zed/Warp/Raycast get docs/behaviour
  reads focused on the **AI/agent control surface** (RQ8, the "we lead" prediction); Radix/Tailwind/Style
  -Dictionary get design-token depth only (RQ7a). The acceptance is the two-way gap table, not an
  exhaustive audit.
- **RP-C — design-system depth.** *Lean:* the visual language + design system is a **first-class
  deliverable** of this research (its own design-doc, the GUI twin of `tui-visual-language.md`), covering
  the two launch themes, tokens, live switching, the component library, the visual grammar, and
  accessibility modes. Owner confirms (vs deferring it to a follow-on).
- **RP-D — homes. RESOLVED (owner, 2026-07-19).** `spec/modules/vibeterm/` carries the **full
  self-contained vibeterm system** — a PROP family adapting the whole vertical (the action system,
  MVC/state + `ModelView`, the AIUI surface, Search Everywhere, i18n, and the visual language + design
  system) for vibeterm, alongside PROP-044 (the shell). vibeterm's design-docs (lore, incl. the design
  system) live in its own vibeterm-owned space, not the shared `spec/design/`. The findings doc for THIS
  research lives in `research/vibeterm/`. *Why:* vibeterm must be able to live as a standalone project and
  theoretically detach from vibevm — a self-contained module is what makes that possible (RP-A).
  *Rejected:* a language-neutral extension of `vibe-actions` / folding into a shared module (both couple
  vibeterm to vibevm). *Revisit:* if vibeterm is promoted to a full specspace (own boot/WAL/CONTINUE), its
  home migrates accordingly.
- **RP-E — clean-room posture for external GUI sources.** *Lean:* docs/behaviour-first for all; read
  source only for MIT VS Code under the firewall; **do not read** Zed (GPL) or Warp (closed) source.
  Owner confirms (and names any local snapshots if source reading is cleared).

## 12. Delegation posture {#delegation}

Per delegation-first, stated out loud: **the synthesis, the architecture, and the contracts are
never-delegate** (judgment). The **legwork** — targeted internal spec extraction and docs-first external
reads into structured, quote-backed summaries — *is* delegable (verification cheaper than generation),
via read-only Explore agents or `dir`-mode fractality packets, the boss verifying every returned claim,
each run announced. The internal reading for THIS plan was done by the boss directly (owner directive:
read the specs fully). This is a judgment-heavy study; it runs mostly boss-led.

## 13. Quick-start (cold-resume) {#quick-start}

```sh
# boot first: CLAUDE.md → spec/boot/INDEX.md → its files → spec/WAL.md → CONTINUE.md → this file
# prerequisite reading (our own specs; no firewall):
#   PROP-039 + spec/design/action-system.md ; PROP-037 + spec/design/tui-visual-language.md ;
#   PROP-042 ; PROP-044 (the shell contract, first consumer)
#
# Phase 0: resolve §11 with the owner; write the §4 design-space map BEFORE external sources.
# Phase 1: internal methodology extraction → ports/adapts/new table.
# Phase 2: external comparative (docs/behaviour-first; MIT source only, under the §1 firewall).
# Output goes ONLY to the findings doc (comparative-research genre):
#   research/vibeterm/vibeterm-ui-architecture-*.md   (D1)
bash tools/self-check.sh   # the floor — green at every phase boundary
```

**Pointer.** `spec/WAL.md` (its `_Updated:` line) is the canonical living state and supersedes any
snapshot. This plan is RESEARCH-only; it produces the findings document. The design-doc, the contracts
(PROP-044 + siblings), and the implementation (VIBETERM-SHELL-PLAN, gated behind this) are separate
campaigns.
