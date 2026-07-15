# Action-System Research Plan v0.1 — a clean-room study feeding the addressable-action system

**status: EXECUTING (2026-07-15) · autonomous full-arc mandate — STUDY → design-doc → Spec 1 → Spec 2 → implementation, run to completion, MAXIMAL (no simplification, owner directive §0.1) · acceptance = a working F1 Search Everywhere in the `vibe tree` TUI (§0.1) · the §1 firewall is held as phase discipline within one continuous run (sources are read only during STUDY, and by read-only subagents; the specs and code are authored from the findings document)**

> **Read-first / boot.** Executed **cold, in a fresh session the owner launches for it**. Boot the normal way (`CLAUDE.md` → `spec/boot/INDEX.md` → its files → `spec/WAL.md` → `CONTINUE.md`), then read this whole file. It is self-contained: the strategic thesis, the **clean-room firewall (non-negotiable)**, the verified sources and where they live, the question-driven agenda, the deliverables, the phases, the predictions, the risks, and the open owner-decisions are all here.
>
> **Its output is a findings document written in OUR words** — never VSCode's or IntelliJ's code — that feeds a *separate* Spec-1 session, which feeds a *separate* Spec-2 session, which feeds *separate* implementation sessions. The studied sources never cross those boundaries (§1). This plan is modelled on its sibling [`OPENREWRITE-RESEARCH-PLAN-v0.1`](OPENREWRITE-RESEARCH-PLAN-v0.1.md), the house genre for exactly this kind of study.

---

## 0. Why this exists — the strategic thesis {#why}

The owner wants a **Search Everywhere** window for `vibe tree` — "по типу shift-shift в IntelliJ IDEA или F1 в VSCode" — that searches **not only packages by name but inside ALL TUI functions**. The enabling requirement, in the owner's words, is that **every action performed through the TUI is addressable**: it has a name, an identifier ("вероятно fdqn подобный адрес"), and **can be invoked programmatically, not only by a key press**.

**The deeper thesis — this is vibevm's own core idea applied to behaviour.** vibevm's entire identity is *addressability*: every normative fact lives under a stable anchor and is cited by `spec://` URI, never by paraphrase (the `addressable-specs` flow, boot slot 1). This campaign extends addressability **from specs to actions**: actions get addresses, a registry, and a discovery surface. Search Everywhere then becomes *"grep for what the product can do."* The design is not "copy an IDE feature"; it is "carry the project's founding move — address everything — into the behaviour layer, and compile the newest and best of how the two reference IDEs did it."

**The system is the product first.** The owner's explicit sequencing: *"вначале нам нужно построить систему, система и будет продуктом вначале … эта система должна быть … self-contained."* We build a **frontend-agnostic, portable action core** — a Rust library that encapsulates vibevm's invocable surface — and only then bind it to `vibe tree`. The same core is meant to later back a **web UI for `vibe tree`**, and to stand as a **guideline** other frontends adopt: a VSCode plugin, an IntelliJ/Zed plugin, a standalone IDE, or a browser IDE. None of those integrations are built now; the abstraction must merely be *ready* for them.

**Leverage.** One action core → N frontends (the "million implementations of the same control" problem, one level up). And because every action is programmatically invocable **by address**, the surface is drivable by scripts, by an RPC/web client, and by fractality workers — a natural fit for the two-process model and the delegation-first posture (a worker can invoke a vibevm action by name instead of simulating keystrokes).

**Guard-rails** keep an open-ended study tractable:

1. **Essential-first.** We do not rebuild VSCode's or IntelliJ's action stack. We define the **smallest useful core** that makes `vibe tree`'s actions addressable + searchable, and grow. Part of the research's job is to tell us what that essential slice *is*.
2. **Clean-room (§1).** We take ideas, never code.

---

## 0.1 Overarching mandate & acceptance — LIVING (owner, 2026-07-15) {#mandate}

The owner ratified this plan and set a standing mandate that governs **every** campaign it seeds — the STUDY here, then the design-doc, Spec 1, Spec 2, and the implementation. Verbatim essentials:

> «не нужно ничего упрощать — делай максимальные нормальные варианты дизайн-доков и прочего» · «Продолжай пока не закончишь реализацию» · «напиши ВСЕ СПЕЦИФИКАЦИИ ХОРОШО И ПОЛНО и РЕАЛИЗУЙ Search Everywhere в TUI в котором можно и искать пакеты, и искать actions, найденные экшены можно вызывать» · «я хочу чтобы у экшенов были человекочитаемые названия и описания и … можно было искать … по названиям и описаниям … заполнение этих полей должно быть частью дисциплины ИИ разработки UI (любого UI). Интерфейс должен быть понимаемым и хорошо навигируемым, и это одна из вещей которые лежат в основе».

**No simplification.** Every deliverable is the maximal, proper version — a full standalone design-doc (RP5 → authored, not folded), a full Spec 1, a full Spec 2. "Essential-first" (§0) means the *sequence* is smallest-useful-first, never a *scope-cut*.

**Run to completion, autonomously.** Execute the whole arc without stopping at each gate for permission (report status, not requests — `operating-modes`); surface only Rule-4 red lines (history rewrite, force-push, large blobs, CI/secrets) and true blockers. Checkpoint via commits + this plan's ledger so a cold resume continues without loss.

**Acceptance — the definition of done.** In the `vibe tree` TUI, **F1** opens a Search-Everywhere window in the **IntelliJ IDEA visual idiom** — a hybrid **"All"** tab that searches everything, plus separate **per-category tabs** that narrow the search — which searches, and lets the user act on:

1. **packages** by name;
2. **inside every field of the package detail cards** — a real Search Everywhere, all card fields indexed;
3. **all actions** — and a found action **can be invoked** directly from the results.

**Human-legibility is a founding discipline, not decoration.** Every action carries a **human-readable name and a description**; both are **first-class searchable fields** (the name/description lane is the fallback match when id and other fields do not). Filling them is **part of the AI-native discipline for building any UI** on this system: an action with an empty or meaningless name/description **fails the floor gate** (`conform`), exactly as untested domain logic does. "The interface is understandable and well-navigable" is a **stated design goal of the system itself**, inherited by every frontend built on it — the reusable guideline the owner wants for other UIs (§0). This is normatively owned by Spec 1 and enforced by the implementation.

**AIUI — the final abstraction: an interface with no visual part (owner, 2026-07-15).** The TUI is
only *one* surface. The canonical interface this system targets is **visual-free** — a **native way
for an AI to operate the UI**: invoke actions by address with typed parameters, read the structured
model/state and the set of available (enabled) actions, and observe the typed result — with
rendering **entirely optional** (shown to a human or for debugging, or not shown at all). This is a
full **"AIUI"**, and it is exactly what the rest of this mandate already builds toward: actions *and
the model* **fully decoupled from the visual**, addressable, typed, programmatically **invocable and
observable**. It is therefore promoted to a **founding design goal**: the headless AIUI surface is
the **reference** surface, and every visual surface (TUI now; web / IDE later) is a **projection** of
the same core — *nothing* in the core / model / controller may depend on rendering, and the model
must be **serialisable and queryable** so an AI reads structured state, never pixels. **Not built
now** — we prototype on the ordinary TUI — but the architecture is designed so the AIUI is a thin
headless adapter that already works because the core owes it nothing. This crowns the
frontend-agnostic and programmatic-invocation-primary pillars, and aligns the UI with vibevm's
two-process model and its MCP surface. (New obligation **DO18**; Spec 1 owns it normatively.)

---

## 0.2 — Execution ledger + scope addenda (running) {#ledger}

_A running record so a cold resume continues without loss; `git log` is the authoritative history.
This plan runs continuously (§0.1) — phases land and commit as they complete._

**Progress**
- **2026-07-15 — STUDY (action systems) COMPLETE.** Six read-only subagents studied the VSCode +
  IntelliJ action systems (clean-room, sources outside the repo). Findings doc
  [`action-systems-vscode-idea.md`](action-systems-vscode-idea.md) landed — the part-(a)
  design-space map, both systems quote-first, 14 design obligations (DO1–DO14), the two-way gaps,
  12 roadmap deltas (Δ1–Δ12 → PROP-039 REQs), and the predictions check (P1–P5 CONFIRMED, P6
  SUPPORTED). Commits `ba2fe1f` + `3351168` + `386ac19`.
- **2026-07-15 — STUDY fully COMPLETE** (9 subagents): the follow-up project-wide/structural SE +
  i18n addendum landed (`8623e45`); the findings doc now carries DO1–DO18 and Δ1–Δ16.
- **2026-07-15 — DESIGN-DOC landed** (`4a9a8f9`): [`spec/design/action-system.md`](../design/action-system.md)
  — the maximal architecture + the ten decisions (D1–D10).
- **2026-07-15 — SPEC 1 landed:** [`PROP-039`](../modules/vibe-actions/PROP-039-action-system.md) —
  the `vibe-actions` contract, granular addressable REQs (§1–§14) mapping Δ1–Δ16.
- **2026-07-15 — SPEC 2 landed:** [PROP-037](../modules/vibe-cli/PROP-037-tree-tui.md) revised onto
  `vibe-actions` (new §13 built-on-the-action-system + §7.3 Search Everywhere promoted from a stub +
  the `vibe.tree` action catalogue §13.5 + non-goals refreshed); TREE-TUI-PLAN carries the
  superseding note. **The specifications are complete** (research → design-doc → Spec 1 → Spec 2).
- **Next — IMPLEMENTATION:** build the `vibe-actions` crate (PROP-039 §§2–11), then wire the F1
  Search Everywhere window into the `vibe tree` TUI (the acceptance, §0.1). Gated phases, floor
  green, committed at each boundary.

**Scope addenda (owner, 2026-07-15) — fold into the findings doc + Spec 1**
- **RQ13 — project-wide / STRUCTURAL Search Everywhere.** Study how VSCode + IDEA search the whole
  project, not only actions — files/symbols/text, and IDEA's language-structural index (PSI/stubs).
  The point: the SE **provider abstraction must be open enough** that we implement a **PackageTree
  provider now** (packages + every package-card field) and a **future AI-Native structural provider**
  (spec/code nodes — the specmap) against the *same* seam.
- **RQ14 — i18n.** The TUI is English-only; we add a **real** message-catalogue i18n (IDEA
  `ActionsBundle` `action.<id>.text/.description`; VSCode `nls.localize`). → **DO15**: every action's
  name/description is a localizable catalogue entry keyed by the stable **address**, English the
  default/fallback, the legibility gate checks the default locale.
- **DO16** — structural / project-wide **provider extensibility** (from RQ13): one provider seam,
  packages now, AI-Native structure later. **DO17** — systematically adopt the incumbents' proven
  mechanisms we still lack: synonyms/aliases + abbreviations as searchable metadata; usage-stats /
  recency ranking; precise key-scoped reactivity; progressive fast/slow async. (The owner's "add the
  missing mechanisms that VSCode/IDEA have and we don't.")

**Next (the arc):** finish the follow-up study → the maximal **design-doc**
(`spec/design/action-system.md`, RP5) → **Spec 1** (PROP-039, the `vibe-actions` contract) →
**Spec 2** (revise PROP-037 + the vibe-tree action catalogue) → **implementation** (the `vibe-actions`
crate + the SE engine + the F1 window). Acceptance in §0.1.

---

## 1. Clean-room discipline — GATING, non-negotiable {#clean-room}

This section governs the whole campaign. Violating it is worse than not doing the research at all.

**Posture.** VSCode (MIT) and IntelliJ Community (Apache-2.0) are both permissive, and their licences would *technically* allow dependency use. **We do not rely on that.** Both are treated as **inspiration-only, never a code source** — the same posture the sibling plan takes toward OpenRewrite. The method: **READ to understand the approach; then write STRUCTURALLY DIFFERENT code, from scratch or on our own permissive Rust dependencies.** No copying, no line-by-line porting, no adaptation of their expression. Identical *behaviour* is fine; borrowed *expression* is not. Rule 1 (the human-authored surface) governs throughout; nothing is attributed to any tool.

**A clean-room advantage by construction.** VSCode is TypeScript; IntelliJ is Java/Kotlin; our engine is Rust. You *cannot* paste TypeScript or Java into Rust — the language boundary forces re-expression. Study the **concepts** (the command/action split, the context model, the discovery/matcher pipeline); implement them idiomatically in Rust on our own permissive deps.

**The firewall — separated sessions, the findings document the only interface.** The sources and our product artifacts must never share a context:

- **(a) STUDY session — *this campaign.*** Reads the sources (§2), produces the **findings document** (§7 D1) in our words. Reads their code; writes **no** spec and **no** product code.
- **(b) SPEC-1 session** — authors the self-contained action-system contract (Spec 1) from the **findings document only**. Does **not** open the studied sources.
- **(c) SPEC-2 session** — adapts `vibe tree` onto the system (revises PROP-037, authors the vibe-tree action catalogue, updates TREE-TUI-PLAN) — again from our own specs, not the sources.
- **(d) IMPLEMENT session(s)** — build from the redesigned specs. Do **not** open the studied sources.

The findings document is the *only* thing that crosses from (a) into (b)/(c)/(d). This is the owner's *"почти Clean Room"* made concrete: we may **read the sources freely to understand** (that is the "почти"), but we **extract nothing** across the firewall except distilled ideas in our own prose. No legal rationale is recorded in any deliverable — the posture is clean-room, full stop.

---

## 2. What we study & where it lives {#sources}

**The sources are the owner's local snapshots, OUTSIDE this repository — deliberately.** They are huge; keeping them out of git honours Rule 4 (no large blobs) and reinforces the firewall (they cannot be accidentally committed or extracted). Verified present at authoring (2026-07-15):

| Snapshot | Upstream | Root (verified) |
|---|---|---|
| **VSCode** | `microsoft/vscode` (TypeScript, MIT) | `C:\Users\olegc\git\snapshot\vscode` |
| **IDEA** | `intellij-community` (Java/Kotlin, Apache-2.0) | `C:\Users\olegc\git\snapshot\idea` |

**Do NOT index the whole trees.** The action systems live in known subsystems; read only these. Every path below was **verified to exist** at authoring (the model's map of where these live proved correct against the snapshots — Phase 0 opens each once to confirm before deep reading).

### 2.1 VSCode source-map {#sources-vscode}

| Concern | File(s) (relative to the VSCode root) |
|---|---|
| **Command identity + invocation** | `src/vs/platform/commands/common/commands.ts` — `CommandsRegistry`, `ICommandService`, `ICommandHandler` |
| **Action = command + menu + kbd + title** | `src/vs/platform/actions/common/actions.ts` — `MenuRegistry`, `MenuId`, `registerAction2`, `Action2`, `MenuItemAction` |
| **Menu resolution vs context** | `src/vs/platform/actions/common/menuService.ts` |
| **The `when` context language** | `src/vs/platform/contextkey/common/contextkey.ts` — `ContextKeyExpr`, `IContextKeyService` |
| **Keybindings + conflict resolution** | `src/vs/platform/keybinding/common/keybindingsRegistry.ts`, `keybindingResolver.ts`, `abstractKeybindingService.ts` |
| **Command Palette / Quick Open (the `>` prefix)** | `src/vs/platform/quickinput/common/quickInput.ts`, `src/vs/platform/quickinput/browser/quickAccess.ts`, the commands provider under `src/vs/workbench/contrib/quickaccess/` + `commandsQuickAccess` |
| **Fuzzy matching / ranking** | `src/vs/base/common/fuzzyScorer.ts` |
| **Cross-process command invocation (RPC surface)** | `src/vs/workbench/api/common/extHostCommands.ts`, `.../mainThreadCommands.ts` |
| **Declarative contribution (extensibility)** | `contributes.commands` / `menus` / `keybindings` via `src/vs/workbench/services/extensions/common/extensionsRegistry.ts` |

### 2.2 IntelliJ source-map {#sources-idea}

| Concern | File(s) (relative to the IDEA root) |
|---|---|
| **The action model + presentation + grouping** | `platform/editor-ui-api/src/com/intellij/openapi/actionSystem/` — `AnAction.java`, `AnActionEvent.java`, `Presentation.java`, `ActionGroup.java`, `DefaultActionGroup.java`, `ActionUpdateThread.java` |
| **Registration + lookup by id** | `ActionManager.java` (+ `platform/platform-impl/src/com/intellij/openapi/actionSystem/impl/ActionManagerImpl.java` — the `<action>`/`<group>` XML parsing) |
| **The context model** | `DataContext`, `DataKey`, `CommonDataKeys` (note: `DataKey.java` is *not* under `editor-ui-api` — Phase 0 pins its real path, likely `platform/core-api/.../actionSystem/`) |
| **Keymap + shortcuts + conflicts** | `platform/platform-impl/src/com/intellij/openapi/keymap/impl/KeymapManagerImpl.java`, `KeymapImpl.java`; `Shortcut` / `KeyboardShortcut` / `MouseShortcut` |
| **Search Everywhere engine** (the headline) | `platform/lang-impl/src/com/intellij/ide/actions/searcheverywhere/` — `SearchEverywhereManagerImpl`, `SearchEverywhereContributor`, `ActionSearchEverywhereContributor`, `SearchEverywhereUI` (dir verified present) |
| **The action-search model + matcher** | `platform/lang-impl/src/com/intellij/ide/util/gotoByName/GotoActionModel.java`, `GotoActionItemProvider.java` |
| **Registration DTD + i18n** | `<actions>`/`<action>`/`<group>` in `plugin.xml`; `ActionsBundle.properties` (localizable labels decoupled from stable ids) |

**Reading discipline.** Quote-first: every claim about a source is backed by a fenced verbatim snippet with its **file path + access date** (comparative-research genre law). Targeted reads only — signatures, registries, the palette/SE pipelines, the context model. We are mapping *approaches*, not auditing implementations.

---

## 3. Study questions — grounded in OUR gaps {#questions}

The study is **question-driven**, not aimless reading. Each RQ carries our preliminary hypothesis, to be confirmed or refuted against the sources.

- **RQ1 — Identity & addressing.** How is an action *named*? (VSCode: an unstructured global string id, e.g. `editor.action.formatDocument`, namespaced by convention only. IntelliJ: a string id registered in XML, e.g. `$Copy`, `EditorSelectWord`.) How is uniqueness / collision handled? *Hypothesis: neither enforces a namespaced, collision-checked, versioned address — our `spec://`-style discipline is a genuine improvement, not a reinvention.*
- **RQ2 — Registration & contribution.** Static manifest vs runtime code vs plugin extension point? (VSCode `registerAction2` + `contributes.*`; IntelliJ `<action>` XML + `AnAction` class.) What is the split's cost?
- **RQ3 — Invocation.** By UI event, by keybinding, and **programmatically** (VSCode `commands.executeCommand(id, …args)`; IntelliJ `ActionManager.getAction(id)` + `ActionUtil`). Return values? Async / cancellation? *Hypothesis: programmatic invocation was retrofitted, not primary — we make it the primary interface.*
- **RQ4 — Parameters & typing.** How are arguments passed and typed? (VSCode: variadic `any`. IntelliJ: `AnActionEvent` + `DataContext`.) *Hypothesis: args are effectively untyped in both — a typed, serializable parameter schema is the gap that unlocks RPC/web/script invocation.*
- **RQ5 — Context & enablement.** How is enabled/visible computed? (VSCode: `when`-clause DSL over context keys. IntelliJ: `update()` + `Presentation.setEnabledAndVisible` reading `DataContext`, on a chosen `ActionUpdateThread`.) What are the correctness/performance hazards?
- **RQ6 — Presentation & i18n.** Label, description, icon, category, tooltip, and how the *localizable label* is decoupled from the *stable id* (IntelliJ `ActionsBundle`; VSCode manifest `title`/`category`).
- **RQ7 — Discovery / Search Everywhere (the headline).** The palette/SE engine: indexing, the **contributor/provider** abstraction, fuzzy matching, ranking (recency/frequency/ML), grouping, preview. (IntelliJ `SearchEverywhereContributor` + `GotoActionModel`; VSCode Quick Open `>` + `fuzzyScorer`.) *Hypothesis: IntelliJ's contributor model already generalises past actions (files, classes, symbols) — it validates a provider-model index for us.*
- **RQ8 — Keybindings & conflicts.** Binding model, multiple keymaps, chords, and how conflicts resolve. What makes conflict resolution debuggable or not.
- **RQ9 — Grouping & menus.** How action groups / menus are modelled and placed (VSCode `MenuRegistry`+`MenuId`; IntelliJ `ActionGroup`+`<group>`).
- **RQ10 — Extensibility.** How a third party adds an action without touching core, and how the core protects itself (overrides, collisions, ordering).
- **RQ11 — Cross-cutting.** Cancellation, async, typed results/errors, undo/transaction wrapping, telemetry, **testing & headless invocation**, and any **capability/permission** model (who may invoke what — load-bearing for a future networked web UI).
- **RQ12 — Failure modes.** What do programmers and users actually complain about (§5)? Each complaint becomes a design obligation for Spec 1.

---

## 4. Part (a) — the independent design-space map, written BEFORE the sources {#design-space}

The owner asked us first to **think independently** ("подумать самостоятельно, использовать знания Claude"). We do this **before** the source study, on purpose: to enumerate what an action system *needs* from first principles, so the research confirms or refutes a stated hypothesis rather than merely paraphrasing two implementations (anchoring guard). Phase 1 writes this map into the findings document as its opening section, and its claims become the predictions (§9).

The concept inventory an action system must cover — to be filled from memory first, then checked against the sources:

1. **Action identity** — the address form; uniqueness; versioning; rename/tombstone/alias.
2. **Action value** — what an action *is* (identity + presentation + enablement + parameters + the invoke function + a typed result).
3. **Registry** — how actions register, are looked up, and how collisions/conflicts are reported.
4. **Invocation** — event, keybinding, programmatic, RPC; sync/async; cancellation; result & error.
5. **Parameters** — a typed, serializable schema; validation; defaults; how a palette/RPC supplies them.
6. **Context & enablement** — the typed context store; a checkable predicate; pure/fast evaluation; "why disabled" introspection.
7. **Presentation** — stable id vs localizable label/description/icon/category; grouping.
8. **Discovery** — the Search-Everywhere index; providers/contributors; matcher; ranking; recency/frequency; grouping; preview.
9. **Keybindings & menus** — bindings, keymaps, chords, conflict reporting; menu/group placement.
10. **Extensibility** — third-party contribution; capability/permission scoping; ordering & override rules.
11. **Frontend binding** — the surface-adapter seam that lets a TUI, a web client, or an IDE plugin drive the same core.
12. **Testability & introspection** — enumerating the registry for golden coverage; headless invocation; telemetry hooks.

---

## 5. Part (c) — the complaint / failure-mode catalogue {#pain}

The owner explicitly asked us to analyse **what programmers and users usually complain about** in action systems. These are our starting hypotheses (from knowledge), to be **validated against the sources and against public discussion** during the study; each validated complaint becomes a **design obligation** the new system must answer point-by-point.

**VSCode.**
- Command ids are **unstructured global strings**; namespacing is convention, not enforced → collisions, no ownership, no versioning; renaming an id silently breaks users' `keybindings.json` and macros.
- The **`when`-clause** is a stringly-typed DSL over a flat global context-key namespace; no compile-time checking; "why is my command greyed out / not firing" is hard to debug.
- Command **arguments are `any`** — untyped, unvalidated, undocumented; the return is `any`/`Promise<any>` → poor for programmatic composition.
- **Three registries** (command / menu / keybinding) must be kept consistent; easy to ship a command with no palette entry (invisible) or a menu item pointing at a missing command. `registerAction2` unified some of this but the split persists underneath.
- **Activation events** couple availability to extension activation → "command 'X' not found" until the contributor activates.
- Keybinding **conflict resolution** (order + `when` + chords) is opaque — the whole "Keyboard Shortcuts" editor exists because it is hard.

**IntelliJ.**
- **`AnAction` boilerplate**; registration split between `plugin.xml` and code; verbose stringly-referenced `<action>`/`<group>` XML.
- **`update()` threading** — must be fast *and* correct about EDT vs BGT; the `ActionUpdateThread` model is a frequent source of UI freezes; the platform-wide migration caused years of churn and deprecation warnings.
- **`DataContext`/`DataKey`** is stringly-keyed and nullable → NPEs and "action mysteriously disabled" debugging; you must know which keys exist in which context.
- Action ids are a **flat global namespace**; plugin collisions; overriding a platform action is fragile.
- **Search Everywhere** contributor API is heavy to implement; ranking (now partly ML) is opaque; "too many results / wrong order / I know it exists but can't find it" is a recurring user complaint; the Find-Action vs Search-Everywhere duplication historically confused users.
- **Shortcut conflicts** across multiple keymaps and OSes.

**General / both (the deep lessons).**
- **No semantic versioning of action ids** → renames are breaking changes with no migration path (keymaps, macros, and muscle memory break silently).
- **Presentation entangled with identity** — getting "stable id, localizable label" right is subtle.
- **Testing & headless invocation** were retrofitted, not designed in.
- **No capability/permission model** — any code can invoke any action; tolerable for a trusted desktop IDE, a liability for a networked/multi-tenant surface (our future web UI).
- **Weak composition** — an action that invokes another action, or a typed macro, is awkward.
- **Discoverability vs. namespace hygiene tension** — the palette/SE is *how* users find actions, so anything not surfaced there is "hidden," which pressures everything into one flat searchable pile.

---

## 6. Part (d) preview — candidate design pillars (to be EARNED, not yet normative) {#pillars}

These are **hypotheses for Spec 1**, listed here so the study knows what it is trying to justify or overturn. **They are candidate, not binding** — the contract (Spec 1) is the only place they become normative (spec-genres: a plan proposes; a contract ratifies). Each answers a §5 obligation and is frontend-agnostic.

1. **Addressable identity.** An action is named by a **stable, namespaced, versioned address** — the owner's "fdqn-подобный адрес", the behaviour-layer twin of `spec://`. Uniqueness is enforced; a rename is a new identity with a tombstone/alias, never a silent break. *(Answers: flat namespace, collisions, id versioning; ties to the `qualified-naming` flow.)*
2. **Action as a first-class typed value.** `{ address, presentation, params-schema, enablement, invoke(ctx, args) -> Result }`. The value is the whole action; the UI, keybinding, and palette are just *invokers* of it.
3. **Programmatic invocation is primary.** `invoke(address, args, ctx)` is *the* interface; key presses and menu clicks are thin callers. *(Answers: retrofitted programmatic paths; unlocks RPC/web/script/worker drivers.)*
4. **Typed, serializable parameters.** A parameter schema so an action can be invoked and validated from a palette, a script, or an RPC client. *(Answers: `any` args.)*
5. **Typed, pure, fast enablement.** Enablement is a pure function over a typed context snapshot, with explicit "why disabled" introspection — no UI-thread hazard, no stringly `when`. *(Answers: `when` debugging, `update()` freezes, `DataContext` NPEs.)*
6. **Collision-erroring registry.** Registration is declarative data with a typed code binding; a collision is a hard, deterministic error (collision ≠ conflict, per `qualified-naming`); the registry is enumerable for golden coverage. *(Answers: XML/code split, silent overrides, testability.)*
7. **Provider-model discovery.** One Search-Everywhere index fed by pluggable **providers** (actions, tree nodes/packages now; files/symbols/settings later), with a fuzzy matcher + transparent ranking (recency/frequency). Frontend-agnostic — the core returns ranked hits; the frontend renders them. *(Answers: SE heaviness, opaque ranking, discoverability.)*
8. **Frontend-agnostic core.** Pure Rust, zero rendering deps; a thin surface-adapter trait binds a frontend (TUI now; web / VSCode / JetBrains / Zed / standalone later). *(The owner's portability + guideline goal.)*
9. **Invocation surfaces are adapters.** Keymap, menu/group tree, palette/SE, programmatic API, and a future JSON-RPC for out-of-process frontends — all funnel through the one `invoke`.
10. **Cross-cutting by design, not bolted on.** Cancellation, typed results/errors, an optional telemetry hook, an optional undo/transaction wrapper, a capability/visibility scope, and localizable presentation decoupled from the stable address. *(Answers: capability model, i18n, composition, testing.)*

11. **Human-legibility as a founding discipline (owner directive, §0.1).** Every action MUST carry a **human-readable name and a description** — non-empty and meaningful — and both are **first-class searchable fields** (the fallback match lane, per the acceptance). This is enforced, not encouraged: an action missing either **fails the floor gate** (`conform`), the same way untested domain logic does. Navigability and understandability are a **stated design goal** of the system, inherited by every frontend built on it — the reusable guideline the owner wants for other UIs (§0). *(Answers: presentation-vs-identity entanglement; discoverability; "I can't find the thing I know exists".)*

There is a thematic through-line to state plainly in the findings doc: **this is addressability extended from specs to behaviour.** It fits the AI-Native Rust discipline the owner already mandated for `vibe tree` (every action an addressable REQ the code cites via specmark; a characterization golden over the whole registry).

---

## 7. Deliverables {#deliverables}

**Only D1 belongs to THIS campaign.** D2–D4 are named here so the firewall and the sequencing are visible, but each is its own downstream campaign with its own plan.

- **D1 — the findings document** (this campaign's output). A single, evergreen, ADR-like **comparative study**: `spec/research/action-systems-vscode-idea.md`, in the **comparative-research genre** (quote-first with access dates; two-way gap analysis — where a fresh design would trail the incumbents and where it would lead; numbered roadmap deltas each naming a target Spec-1 REQ; a re-fetch/provenance table). It carries: the part-(a) design-space map (§4), the per-system answers to RQ1–RQ12 (§3), the part-(c) complaint catalogue (§5) validated, and the distilled candidate design (§6) as *deltas*, not decrees.
- **D2 — Spec 1: the action-system contract** (downstream, SPEC-1 session). A binding PROP with **granular addressable REQs** (PROP-037's house style). Tentative number **PROP-039**; home is an owner decision (RP3) — likely a new module `spec/modules/vibe-actions/` backing a new crate, since the core is a self-contained library.
- **D3 — Spec 2: the vibe-tree adaptation** (downstream, SPEC-2 session). Revise [PROP-037](../modules/vibe-cli/PROP-037-tree-tui.md) so every TUI action is an addressable action of the Spec-1 system; author a **vibe-tree action catalogue** (each F-key/command an addressed REQ); promote Search Everywhere from its reserved `ComingSoon` stub (TREE-TUI-PLAN D3) to a first-class feature; update [TREE-TUI-PLAN](../terraforms/TREE-TUI-PLAN-v0.1.md).
- **D4 — implementation** (downstream, IMPLEMENT sessions). Build the core crate, the providers, the Search-Everywhere engine, and the TUI binding; the web UI and other adapters follow when commissioned.

---

## 8. Phases (gated) {#phases}

Every phase ends with `bash tools/self-check.sh` green (docs-only phases are trivially green, but the floor is still run) + an execution-ledger entry (§2 of this file, prepended at close) + a refreshed status line. Any phase boundary is a safe stop.

- **Phase 0 — framing (NO commits).** Confirm the deliverable homes (RP3); lock RQ1–RQ12; write the part-(a) design-space map (§4) and turn its claims into predictions (§9); open each source signature file in §2 **once** to confirm the map before deep reading; resolve the `DataKey.java` real path. Gate: RQ list + source-map + design-space map reviewed with the owner.
- **Phase 1 — VSCode study (part b).** Targeted reads of §2.1; quote-first extraction answering RQ1–RQ12 for VSCode; draft the VSCode half of the findings doc. Commit `docs(research): …`.
- **Phase 2 — IntelliJ study (part b).** Targeted reads of §2.2; the same for IntelliJ. **Search Everywhere gets extra depth** — it is the owner's headline; map the contributor model, the matcher, and the ranking. Commit.
- **Phase 3 — complaint catalogue (part c).** Validate §5 against the sources and public discussion; turn each surviving complaint into a numbered **design obligation**. Commit.
- **Phase 4 — synthesis: two-way gaps + roadmap deltas (comparative-research close).** Where a fresh design trails the incumbents, where it leads; the candidate pillars (§6) become **numbered deltas**, each naming a prospective Spec-1 REQ; check every prediction (§9); write the findings-doc REPORT + the re-fetch table. Commit. **Gate: the findings document is complete and self-contained — the firewall can close.**

At close, the campaign hands **only the findings document** to the SPEC-1 session.

---

## 9. Predictions — falsifiable, checked at close {#predictions}

- **P1** — Both systems keep **command/action identity separate from menu placement and from keybinding** (distinct registries). A naïve single-object model is *not* what they do. *Falsified if either unifies them into one object.*
- **P2** — Neither enforces a **namespaced, collision-checked, versioned** action id at the type level (both use flat global strings). *Falsified if either has enforced structured ids.* (If true → pillar 1 is a real improvement.)
- **P3** — IntelliJ's Search Everywhere uses a **contributor/provider** abstraction that already generalises beyond actions. *Falsified if SE is actions-only or hard-wired.* (If true → pillar 7 is validated prior art.)
- **P4** — VSCode command **arguments are untyped** (`any`); there is no parameter schema. *Falsified if a typed arg schema exists.* (If true → pillar 4 fills a real gap.)
- **P5** — The dominant complaint class is **`update()`/threading** for IntelliJ and **`when`-clause/context-key debugging + discoverability** for VSCode. *Falsified if the evidence points elsewhere.*
- **P6** — A **frontend-agnostic** action core is feasible with **zero rendering deps** — registry + invocation + typed context + SE-index have no UI dependency; rendering is purely an adapter concern. *Falsified if some core concern cannot be expressed without a UI type.* (A design spike in the SPEC-1 session settles this; the study gathers the evidence.)

---

## 10. Risks {#risks}

- **R1 — scope creep.** These are two of the largest codebases in existence. Mitigation: the §2 source-map is a hard boundary; we read *approaches*, not implementations; essential-first (§0).
- **R2 — anchoring on the incumbents.** Reading first would make us re-describe VSCode+IntelliJ instead of designing. Mitigation: part (a) (§4) is written *before* the sources; predictions are registered up front.
- **R3 — firewall leak.** A studied idea's *expression* bleeding into Spec 1. Mitigation: the §1 firewall — the findings doc is the only interface; the SPEC sessions never open the sources; the Rust/TS-Java language boundary forces re-expression.
- **R4 — over-building the core.** Designing for six hypothetical frontends before one ships. Mitigation: Spec 1 targets the `vibe tree` TUI as its *first and only current* consumer, with the seams *ready* for more — readiness proven by design, not by building adapters.
- **R5 — naming the address scheme too early or too late.** Mitigation: RP1 is surfaced now; the study informs it; Spec 1 ratifies it.

---

## 11. Open review points — owner decisions {#review-points}

**All resolved by the owner (2026-07-15)** — «я со всем согласен, кроме упрощения» + the §0.1 follow-ups. Kept below for the record: RP1 → the URI `action://` form; RP2 → the maximal self-contained core, no scope-cut; RP3 → homes accepted (findings doc in `spec/research/`; Spec 1 = a new module `spec/modules/vibe-actions/` + PROP-039 + a new crate `vibe-actions`); RP4 → the boss reads the sources, source-reading fanned to read-only subagents to spare context (announced); RP5 → author the **full standalone design-doc** `spec/design/action-system.md` (do not fold).

- **RP1 — the product name + the addressing grammar.** **Owner lean (2026-07-15): the URI form `action://<group>/<name>[?params]`** — the behaviour-layer twin of `spec://<module>/<doc>#<anchor>`, with typed parameters carried as the query (e.g. `action://vibe.tree/sort?by=name&dir=asc`). This lean **directs the study** — sharpen RQ1 (identity/addressing) and RQ7 (Search Everywhere) against it — and is **ratified in Spec 1**, not here. Rejected for now: an IntelliJ-style **dotted FQDN** (`org.vibevm.tree.copy.markdown`) — parameters cannot live in the address, and it reads less like the project's `spec://` brand. Working concept name **"Addressable Actions"**, also ratified in Spec 1.
- **RP2 — Spec-1 scope boundary.** What is in the self-contained core (identity, registry, invocation, typed params, typed context, discovery/SE, adapter seam) vs. what stays vibe-tree-specific (the concrete action catalogue, the F-key map, the tree providers)? A first cut is in §6/§7; the owner confirms the line.
- **RP3 — homes.** Findings doc `spec/research/action-systems-vscode-idea.md` (recommended); Spec-1 home — a new module `spec/modules/vibe-actions/PROP-039-*` backing a new crate `vibe-actions` (recommended), vs. `spec/common/` as a foundational decision.
- **RP4 — delegation posture for the study legwork** (§12).
- **RP5 — a standalone design-doc (lore) or folded?** Default: fold the "why" into the findings doc + decision records at the Spec-1 anchors; author a separate `spec/design/action-system.md` only if the rationale outgrows that.

---

## 12. Delegation posture {#delegation}

Per the delegation-first directive, stated out loud: **the synthesis, the design, and the contract are never-delegate** (architecture / spec / judgment). The **source-reading legwork** (targeted reads of a §2 subsystem → a structured extraction: signatures + verbatim quotes + a plain-language summary) *is* in principle delegable — verification is cheaper than generation, since the boss re-checks quotes against the files. **But** the sources sit **outside the repo** (a fractality `worktree` worker sees only the vibevm tree) and quote-fidelity is load-bearing for a clean-room study. **Default: the boss reads the sources**; delegation is opt-in per-subsystem via a `dir`-mode packet pointed read-only at a snapshot root, the boss verifying every returned quote — and any such run is announced. This is a judgment-heavy study; running it mostly solo is the delegation-rules verdict, not an oversight.

---

## 13. Quick-start (cold-resume) {#quick-start}

```sh
# boot first: CLAUDE.md → spec/boot/INDEX.md → its files → spec/WAL.md → CONTINUE.md → this file

# confirm the sources are present (verified 2026-07-15):
ls "C:/Users/olegc/git/snapshot/vscode/src/vs/platform/actions/common/actions.ts"
ls "C:/Users/olegc/git/snapshot/idea/platform/lang-impl/src/com/intellij/ide/actions/searcheverywhere"

# Phase 0: write the part-(a) design-space map (§4) BEFORE reading the sources.
# Phase 1/2: read only the §2 source-map files; quote-first; answer RQ1–RQ12.
# Output goes ONLY to the findings doc; the firewall (§1) keeps sources out of Spec 1/2 + impl:
#   spec/research/action-systems-vscode-idea.md   (comparative-research genre)

bash tools/self-check.sh   # the floor — green at every phase boundary
```

**Pointer.** `spec/WAL.md` (its `_Updated:` line) is the canonical living state and supersedes any snapshot. This plan is STUDY-only; it produces the findings document and nothing else — Spec 1, Spec 2, and the implementation are separate campaigns behind the §1 firewall.
