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

## 0.2 Execution ledger (running) {#ledger}

_A running record so a cold resume continues without loss; `git log` is the authoritative history._

- **2026-07-19 — PLANNED.** This plan authored from a full read of `vibe-cli` + `vibe-actions` + the
  design lore + the house template. Awaiting owner review of §11 before Phase 1. No study commits yet.

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

Each RQ carries a preliminary hypothesis to confirm or refute.

- **RQ1 — Universality of the action core.** Which PROP-039 concerns are surface/language-neutral, and
  how does each port to TS? *Hypothesis: address grammar, Action value, Registry laws, typed
  context + pure enablement, `invoke`, capabilities, the serialisable `ModelView`, the Surface seam, the
  Search-Everywhere provider model, and address-keyed i18n are ALL expressible in TS with zero rendering
  deps — the `#no-render-dep` invariant holds in TypeScript.*
- **RQ2 — Entities & domain model.** The shell's entity set (Window, Tab, Pane, Session/Terminal,
  Profile, Theme, Locale, Action, Provider) — identity/addressing, hierarchy, and the `action://vibeterm/*`
  catalogue. *Hypothesis: stable ids (branded `TabId`, etc.) + the `action://` grammar cover addressing
  with no new scheme.*
- **RQ3 — MVC/state in TS/Solid.** How does "the Model + Registry are the interface; the View is one
  projection" realise in SolidJS? Render-free engine (no Solid/DOM/Electron types) + a serialisable
  `ModelView` + the chrome as its projection + the AI-UI surface as a peer client. *Hypothesis: Solid's
  fine-grained store is an excellent `ModelView` host; MVVM and our MVC coincide; a dependency-boundary
  lint keeps the engine render-free.*
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
- **RQ7 — Visual language & design system (the GUI twin of the Theme).** The GUI analogue of semantic
  palette role-tokens + one-source-projection: **design tokens** (colour/space/radius/typography roles),
  the **two launch themes** (dark purple + Anthropic-style), **live** theme switching, the component
  library (Solid + Kobalte + Tailwind v4), the visual grammar (windows/panels/spacing/rhythm — the GUI
  §5/§6), icon vocabulary, and accessibility axes (contrast / reduced-motion / density). *Hypothesis: the
  TUI's "one Theme projected across tiers" becomes "one token set projected across themes/modes"; the
  tier-degradation concept has no GUI analogue and is replaced by theme/accessibility modes.*
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
  provenance (citations), never build/spec dependencies.*

---

## 4. Part (a) — the independent design-space map, written BEFORE external sources {#design-space}

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
11. **Transport** — the sidecar-ready contract; Electron IPC now; capability scope.
12. **Testability & evolvability** — the headless AIUI as the golden; how the stack grows without churn.

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

## 7. Deliverables {#deliverables}

**Only D1 belongs to THIS campaign.** D2–D4 are named for sequencing; each is its own downstream
campaign.

- **D1 — the findings document** (this campaign's output). Home `research/vibeterm/` (comparative-research
  genre for the external part): the §4 design-space map; the internal methodology extraction (what
  ports / adapts / is new); the §5 pitfalls validated; the external comparative (two-way gaps); the §6
  pillars as **numbered architecture deltas** each naming a prospective contract REQ; the §9 predictions
  checked; a re-fetch/provenance table for external sources.
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

- **Phase 0 — framing (NO commits).** Resolve the §11 review points with the owner; lock RQ1–RQ10; write
  the §4 design-space map and turn its claims into §9 predictions; confirm which external sources are
  in-bounds (RP-E). **Gate: the RQs + review points + design-space map reviewed with the owner.**
- **Phase 1 — internal methodology extraction.** From our own specs, write the ports/adapts/new table
  for the full vertical (entities, MVC/state, actions/AIUI, SE, i18n, visual language). Commit `docs(research): …`.
- **Phase 2 — external comparative (clean-room, docs-first).** Targeted study of §2.2; two-way gaps.
  Commit.
- **Phase 3 — pitfalls validated (§5) → design obligations.** Commit.
- **Phase 4 — synthesis.** The §6 pillars become numbered deltas each naming a contract REQ; the §9
  predictions checked; the findings REPORT + re-fetch table. **Gate: the findings document is complete
  and self-contained.** Commit. Hands off to the design-doc session.

## 9. Predictions — falsifiable, checked at close {#predictions}

- **P1** — Every PROP-039 core concern is expressible in TS with **zero rendering deps**. *Falsified if
  some concern needs a DOM/Solid/Electron type.*
- **P2** — The serialisable `ModelView` generalises from a single-screen TUI to a **window→tab→pane
  tree** with no new mechanism. *Falsified if the multi-surface shell needs a fundamentally different
  model shape.*
- **P3** — A **language-neutral action/AIUI contract** shared by Rust and TS is feasible and unifies the
  two AIUIs. *Falsified if they cannot share address grammar + `ModelView` + verbs without loss.*
- **P4** — "One Theme projected across tiers" maps to **"one token set projected across themes/modes"**;
  the tier-degradation concept has **no** GUI analogue. *Falsified if tiers have a real GUI analogue we
  need.*
- **P5** — No mainstream GUI app (VS Code / Zed / Warp) exposes a **render-free, addressable,
  AI-drivable** action core with a serialisable `ModelView` as the *reference* surface — our approach
  **leads**. *Falsified if one already does exactly this.*

## 10. Risks {#risks}

- **R1 — scope creep** (a whole GUI stack + a design system + external apps). Mitigation: port-first;
  the shell is the first-and-only current consumer; essential-first.
- **R2 — anchoring** on the external apps. Mitigation: §4 is written before §2.2; predictions up front.
- **R3 — firewall / licence leak** (Zed GPL, Warp closed). Mitigation: docs/behaviour-first; source only
  for MIT VS Code; the findings doc is the only interface; the Rust/Java/TS boundary forces re-expression.
- **R4 — over-building the universal contract** before one surface ships. Mitigation: target the shell;
  readiness proven by design, not by building adapters.
- **R5 — two-core drift** (Rust `vibe-actions` vs the TS shell core). Mitigation: RP-A — settle the one
  shared contract in this research, before either grows.

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
- **RP-B — research scope & external source set.** *Lean:* internal port is primary; external comparative
  is targeted + docs-first (VS Code, Zed, Warp, Raycast; Radix/Tailwind/Style-Dictionary). Owner confirms
  the set (and whether to include any).
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
