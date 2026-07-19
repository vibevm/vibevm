# VibeTerm — self-contained task handoff (`research/vibeterm/task.md`)

**Purpose.** A cold-start-complete brief so ANY fresh session can drive the VibeTerm
UI-architecture work to completion with no prior context. Read this file, then the
files in §2, then resume at §5. Last updated 2026-07-19.

---

## 0. TL;DR — what this task is

Build **VibeTerm**: a multi-tab / multi-window **terminal workspace** (Electron +
SolidJS), **AI-UI-ready by construction**, whose whole architecture is a **full,
self-contained, vibeterm-adapted port** of vibevm's proven `vibe-actions` +
`vibe tree` methodology. Goal state: vibeterm can live as a **standalone project and
even spin out of vibevm** (self-contained, specspace-ready).

The owner's non-negotiables:
1. **AI-UI-Ready.** An AI must drive **any** UI function through a **semantic API**
   (`invoke` / `state` / `list_actions` / `search`), **NOT CDP / screenshots**, as
   well as or better than a human. **Every** architectural decision accounts for the
   future AI-UI surface.
2. **Port the universal methodology.** Same **Action System, Search Everywhere,
   `ModelView`, AIUI-as-reference, i18n, design tokens** as `vibe tree` — it was
   designed universal (render-free core; every surface is a projection).
3. **Whole vertical, done as a product:** entities → MVC/state → actions/AIUI →
   Search Everywhere → i18n → **visual language → design system**.
4. **Cadence: research → design → execution.** Do NOT one-shot. Read prior art fully.
   Currently in the **RESEARCH** phase (at the Phase-0 gate).

**Immediate next action:** resolve the two open review points **RP-B** and **RP-E**
(§4) with the owner, then **execute Phase 1** of the research plan (§5) — the
internal methodology-extraction into the findings doc.

> **Plan sharpened 2026-07-19.** A critical re-read found the original plan declared "open research" while
> D0–D7 were already frozen binding — so research would have ratified decisions instead of earning
> answers. The plan now carries an explicit **frozen-axes-vs-open-questions split** (research-plan §0.3),
> a sharpened **RP-A = identity-grammar conformance** (a normative spec + a CI golden; build-dep stays
> none — the reconciliation of "self-contained" with "no silent two-core drift"), six **new RQs**
> (conformance, capability/security for an AI peer, AIUI-plane unification, transport-contract form,
> GUI-only inventory, the AI-UI evaluation criterion), **falsifiable+measured predictions (P1–P8)**, and
> **AI-Native-ready output** framing (deltas land as REQs under AI-Native Rust for `vibe-actions` and
> AI-Native TS for `vibeterm-core`). This revision is itself a docs commit; Phase 1 runs after.

---

## 1. Where everything lives (the map)

- **`research/vibeterm/VIBETERM-UI-ARCHITECTURE-RESEARCH-PLAN-v0.1.md`** — the
  **research plan** (the current driver; house research-plan form; **RQ1–RQ17**,
  phases, predictions **P1–P8**, review points). Carries the **frozen-vs-open split (§0.3)** and the
  **AI-Native-ready** framing. RP-A + RP-D are RESOLVED (RP-A sharpened to **identity-grammar
  conformance** — §0.1); RP-B/C/E summarized in §4.
- **`research/vibeterm/VIBETERM-SHELL-PLAN-v0.1.md`** — the **build/campaign plan**
  (milestone-1 shell: placeholder chrome + working switch/split/new-window). **GATED**
  behind the research → design → contracts.
- **`research/vibeterm/task.md`** — this file.
- **`spec/modules/vibeterm/PROP-044-terminal-shell.md`** — the **shell contract**
  (Decisions D0–D7; §1–§11 REQs). This is where the vibeterm PROP FAMILY will grow
  (the self-contained system).
- **`apps/vibeterm/`** — the **existing Electron app** being generalized: `main.cjs`
  (Electron main, node-pty here, `--control` AIUI server + headless mirror + CDP),
  `renderer.js` (xterm.js), `index.html`, `lib/args.mjs` + `lib/keymap.mjs`
  (node --test), `scripts/package.mjs`, `resources/` (icons), `package.json`
  (electron ^32, @xterm/xterm ^5.5, node-pty ^1.1).
- **`refs/screens/projectx/`** — reference UI captures (OUT OF GIT — gitignored).

---

## 2. Prerequisite reading — READ THESE FULLY (+ what each carries)

The essence is captured below so this file stands alone; still open the files when
designing. **The action system + the visual language are the two pillars — full notes:**

### 2a. `spec/modules/vibe-actions/PROP-039-action-system.md` — THE action system
The render-free, addressable, programmatically-drivable behaviour layer. Key REQs
(cite by anchor `spec://vibevm/modules/vibe-actions/PROP-039#…`):
- **`#no-render-dep`** — the crate/core has **ZERO rendering deps** (no ratatui/DOM/
  Electron/terminal types). *This invariant makes every surface + the AIUI possible.*
  (For vibeterm: the TS engine imports no Solid/DOM/Electron types; lint it.)
- **`#address-grammar`** — an action is `action://<group>/<name>[?params]`;
  `(group,name)` globally unique; params ride the query. (vibeterm → `action://vibeterm/*`.)
- **`#action-fields`** — an Action = address · **Presentation** (mandatory non-empty
  **name + description**, localizable, searchable — `#presentation`, gated by
  `#legibility-gate`) · **ParamSchema** (typed, named — `#param-schema`) · **enablement**
  (pure `Fn(&Ctx)->{visible,enabled,reason}` — `#enablement`) · **invoke** · **Capability** ·
  **SearchMeta**. Resolved snapshot is immutable; change = re-resolution (`#action-snapshot`).
- **Registry** — `#registry-collision` (registering a dup `(group,name)` is a HARD
  error), `#registry-integrity` (any address reference validated at build), `#registry-enumeration`
  (fully enumerable — backs the legibility golden + Search Everywhere + `list_actions`).
- **`#context-snapshot`** — a typed `TypeId`-keyed typemap `Ctx`; `#context-introspection`
  (enumerate keys; a disabled action exposes "why disabled").
- **`#invoke`** — `invoke(addr, params, ctx) -> Future<InvokeResult>` is **THE** interface;
  key/menu/palette/AIUI are thin callers; async, typed result, cancellable. `#capabilities`
  (Safe/Mutating/Dangerous, checked before run — the seam a networked/AI caller is refused by).
- **i18n** — `#i18n-catalogue` (`Msg{key = "action.<addr>.name|.description", default_en}`),
  `#i18n-resolved` (`ResolvedLabel{value, original_en}` — SE matches English under any locale),
  `#i18n-fallback` (English mandatory-complete, `ArcSwap<Catalogue>` live swap, Fluent format),
  `#legibility-gate` (CI floor: every action's name+desc present + resolves in `en`).
- **Keymap** — `#keymap-bindings` (key→`(addr, params)`), `#keymap-resolver` (pure 3-state
  `NoMatch | NeedMoreChords | Found`; chord timers/IME/focus in the adapter), `#keymap-conflicts`.
- **Search Everywhere** — `#se-provider` (a `SearchProvider`: two-phase
  `enumerate(query)->cheap keys` + `resolve(key)->hits`, `id/group_name/sort_weight/separate_tab`,
  `ItemAccessor`, `on_selected->Close|Stay`, `render_row->RowDescriptor`), `#se-engine`
  (debounce ~90-120ms + cancel; per-provider caps; one commensurable scorer; recency-weighted;
  exact-match floor; dedup keeping higher; round-robin drain; freeze-on-"more"), `#se-ranking`
  (match ladder exact→prefix→CamelCase/subsequence→substring→**name/description word** = the
  fallback lane), `#se-renderer` (one normalized `RowDescriptor{icon,primary,secondary,group,
  enabled,kind}`), `#se-providers` (ship: Package / PackageField / Action; reserved: Structure),
  `#se-tabs` (hybrid "All" + per-category; Tab/Shift-Tab cycle).
- **Surfaces & AIUI** — `#surface-trait` (a `Surface` adapter: `present(ModelView)` +
  yields `Event`s; headless surface's present is a no-op), `#model-view` (a **serialisable
  `ModelView`**: focus, open modals, visible rows, selection, active tab, enabled actions +
  addresses + reasons — NO rendering types; AI reads structured state, never pixels), `#aiui`
  (the **headless AIUI = the REFERENCE surface**: `list_actions(filter?)` / `invoke(addr,args)` /
  `state()->ModelView` / `search(query,tab?)`; a thin adapter because the core owes rendering nothing).

### 2b. `spec/design/action-system.md` — the lore (why + the MVC data-flow)
- **Thesis:** the **Model + the Action Registry ARE the interface; the View is one
  optional projection; the headless AIUI is the reference.** Six founding principles
  (addressability of behaviour; programmatic-invocation-primary + AIUI-reference;
  frontend-agnostic core; human-legibility a discipline; discovery over any structured
  universe; typed everything).
- **Data-flow (§`#flow`):** `event → Surface(TUI key | AIUI invoke/query) → Controller
  (keymap resolve → addr+args) → invoke(addr,args,ctx) → Action mutates Model (serialisable)
  → View renders ModelView (optional) / AIUI reads ModelView + list_actions (reference).`
- Decisions D1–D10 (URI address; collision-erroring registry; typed ctx + pure enablement;
  programmatic-primary + AIUI-reference; two-phase provider SE; address-keyed i18n; English
  legibility gate; one normalized renderer; pure 3-state keymap; one recency-weighted ranker).

### 2c. `spec/modules/vibe-cli/PROP-037-tree-tui.md` — the surface-side MVC + components
- **Four layers (§`#layers`):** backend → **Model** (data + UI state; NO rendering/event
  types) → **View** (component library + **Theme**; NO control flow) → **Controller** (events,
  keymap registry, modal stack, actions). Law: *styling never leaks into control; domain never
  leaks into the app.*
- **Component library (§`#components`):** wrap→extend→invent (one `ui::` facade): Window, Menu,
  Button, Group, RadioGroup, TextField, Card, **ComingSoon** (the reserve-a-feature placeholder).
- **§13 built-on-vibe-actions:** the TUI is a **Surface**; the Model IS the serialisable
  **ModelView**; **every command is an addressed Action** (`action://vibe.tree/*`, §`#action-catalogue`
  §13.5); the keymap **binds keys to addresses**; i18n is real; adding a command = registering an
  Action → it appears in footer + keymap + Search Everywhere with no extra wiring.

### 2d. `spec/design/tui-visual-language.md` — THE visual-language / design-system reference
The GUI design system is the analogue of this. Key content:
- **Semantic palette role-tokens** (the roles a component may ask for; NEVER a raw colour):
  `base · surface0 · surface1 · muted · subtext · text · accent · love · gold · foam · rose ·
  selection · border · paper · button_on · button_off`. Components name a **role**, never a hex.
- **Five palettes** (Rosé Pine "cosmic violet" canonical-locked + Catppuccin Mocha/Macchiato/
  Frappé/Latte); `is_light` flag; derived roles (`selection = accent ground + base text`,
  `border = muted`, `paper = surface0`).
- **Rendering tiers** 3(truecolor)→2(256)→1(16 ANSI)→0(dumb ASCII); **degradation = projection**
  — ONE `Theme` built for Tier 3, **projected** onto the detected tier. *(GUI analogue: ONE token
  set **projected** across themes/modes; the tier-degradation concept has NO GUI analogue → replace
  with theme + accessibility/density modes.)*
- **"Windows are windows"** (solid panel + rounded frame + title chip + padding + shadow + close),
  **spacing/rhythm** (`PAD_X=2, PAD_Y=1, GUTTER=1`; content floats with air; rows centred).
- One-line law: **"the Theme is the CSS"** — a restyle touches only the theme.

### 2e. `spec/modules/vibe-cli/PROP-042-aiui-observation.md` — the three planes
- **Render plane** (headless snapshot; observation-only — "observes, does not act"), **terminal
  plane** (live vibeterm + `--control` server + CDP), **model plane** (`vibe aiui state` →
  serialisable `ModelView`). **CDP is OBSERVATION ONLY** (`#render-plane`); CONTROL is `invoke`.
- Snapshot formats `text`/`cells`; the key-script grammar; the `vibe aiui` CLI family; the
  `vibe term` launcher + in-place upgrade (`VIBETERM=1`) + `OSC 7773` icon protocol (§5.1).

### 2f. `spec/modules/vibe-cli/PROP-036-package-tree.md` — entities/domain-model exemplar
- The `PackageTree` analyzer; **`--json` = the same serialisable model the TUI renders**; a GUI
  client explicitly deferred. Least central to the UI arch, but the entity/serialisable-model
  pattern is the template.

### 2g. `spec/research/ACTION-SYSTEM-RESEARCH-PLAN-v0.1.md` — the house research-plan TEMPLATE
- How the owner runs research: **independent design-space map BEFORE sources** (anti-anchoring);
  **clean-room firewall** (STUDY → design-doc → Spec1 → Spec2 → impl, separated; findings doc the
  only interface); RQ-driven with hypotheses; gated phases; falsifiable predictions; open review
  points; delegation posture. **Our research plan follows this form.**

### 2h. The current app + the base stack
- `apps/vibeterm/main.cjs` + `renderer.js` + `index.html` + `package.json` — read to see the
  CURRENT single-window architecture (node-pty in main; IPC `pty`/`input`/`resize`/`ready`/
  `vibeterm:set-icon`; `--control` server + `@xterm/headless` mirror + discovery + CDP;
  `nodeIntegration:true, contextIsolation:false`).
- `C:\Users\olegc\git\foton\packages\desktop` — the base-stack reference (**Solid + Vite +
  Tailwind v4 + Kobalte**; strict-ish TS). BASE STACK ONLY — its multi-view is a Tauri CSS-hide
  hack (no WebContentsView); DO NOT port that legacy. Its area/widget config schema + tiered
  contexts are the reusable *ideas*.

---

## 3. Resolved decisions — these are the FROZEN AXES (constraints; research-plan §0.3)

These are **constraints the research works within**, not hypotheses it earns (the frozen-vs-open split is
load-bearing — see research-plan §0.3). Do not re-litigate without naming the trigger.

- **AI-UI-Ready by construction** (owner). Control is semantic `invoke`; CDP observation-only.
- **Self-contained & detachable** (RP-A + RP-D RESOLVED, owner 2026-07-19): the FULL adapted
  system (action/AIUI core, MVC/state + ModelView, Search Everywhere, i18n, visual language +
  design system) lives under **`spec/modules/vibeterm/`** (+ `apps/vibeterm/`), a PROP family,
  with **NO hard dep on vibevm-internal crates/specs**, so vibeterm can spin out. It **ports** the
  methodology (patterns, `action://` grammar, `ModelView` shape, AIUI verbs, design tokens) as
  provenance, keeping a methodologically-compatible grammar for coherence — never a build dep.
  Research + plans live in **`research/vibeterm/`**.
- **Identity-grammar conformance (RP-A sharpened, owner 2026-07-19; research-plan §0.1 + RQ12).** "No
  build-dep" does **not** mean "no shared grammar" — the address grammar, the `ModelView` schema, the
  AIUI verbs, the SE provider contract, and the i18n key scheme are an **identity-grammar spec**, a
  normative document both the Rust `vibe-actions` and the TS `vibeterm-core` validate via a
  **conformance golden in CI**. This is the reconciliation of "self-contained" with "no silent two-core
  drift" — shared grammar, not shared build-dep. The minimum surface and the golden's shape are an open
  question (RQ12); the direction is settled.
- **AI-Native-ready output (owner 2026-07-19; research-plan §0.1).** The deltas land as REQ-ready
  anchors under **AI-Native Rust** (`vibe-actions` side) and **AI-Native TypeScript** (`vibeterm-core`
  side): granular addressable REQs, `scope!`/specmark traceability, cells, strict `tsconfig`, branded
  types, `Result` errors, `vitest`, and a **`#no-render-dep` dependency-boundary lint on the floor**.
- **Stack (PROP-044 D4):** Solid + Vite + Tailwind v4 + Kobalte + strict TS. xterm terminal-views
  stay **lean vanilla TS** (N tabs = N light renderers). **TS-core now, full gate later** (D1).
- **Shell = default visible `vibe term`** (D2); headless/`--control` stays **bare single-view,
  unchanged**.
- **Tabs engine** (D0, VERIFIED by a Phase-0 spike on Electron 32.3.3): each tab = its own
  `WebContentsView` + a **main-owned pty keyed by `TabId`**; switching/splitting/**tear-off to a
  new window** = show/hide or **reparent the view** (`removeChildView`→`addChildView`) with
  **zero reload, no state loss** (spike proved: same webContents.id, no reload, xterm buffer
  intact). Split ceiling = 2 in M1 (D3).
- **Chrome↔engine protocol (D5):** transport-agnostic, **sidecar-ready** — Electron IPC now via a
  **typed preload bridge** (`contextIsolation:true`), designed so state can later move to an
  external process without redesign. The **form** of the contract (codec, versioning, stream/RPC,
  consistency) is an open question (RQ15), not the transport-shape decision.
- **i18n from the start (D6):** address-keyed catalogue, `{value, original_en}`, legibility gate,
  **en + ru**, live locale switch, no hardcoded UI copy. The TS **mechanism** (Fluent runtime,
  reactive catalogue) is an open question (RQ9).
- **Live theming (D7):** **design tokens** (CSS custom properties), **live** switch (no reload);
  components reference roles, **never hardcoded hex**; **two launch themes** — a **dark purple**
  (after the reference layout) + an **Anthropic-style**. The **token architecture** (Tailwind `@theme`
  integration, Kobalte, a11y modes, SVG icons, spacing-scale) is an open question (RQ7a–e).

---

## 4. Open decisions (resolve at the Phase-0 gate, before Phase 1)

- **RP-B — external comparative scope.** Lean: internal port primary; external **targeted +
  docs-first** (VS Code, Zed, Warp, Raycast; token systems Radix/Tailwind/Style-Dictionary). **Per-source
  depth:** VS Code (MIT) gets action/palette depth + the semantic-control-API question (RQ8/RQ14);
  Zed/Warp/Raycast get docs/behaviour reads focused on the AI/agent control surface; Radix/Tailwind/Style
  -Dictionary get design-token depth only (RQ7a). Acceptance = the two-way gap table, not an exhaustive
  audit.
- **RP-E — clean-room posture.** Lean: docs/behaviour-first; read source only for **MIT VS Code**
  under the firewall; **do NOT read** Zed (GPL) / Warp (closed) source.
- **RP-C — design-system depth.** Effectively CONFIRMED by the self-contained decision (a
  standalone product owns its design system) — treat as first-class unless the owner defers it.
  Now split into the **RQ7a–e** sub-questions (Tailwind `@theme`, Kobalte theming, a11y modes, SVG
  icons, spacing-scale) for the findings.

---

## 5. What to DO next — the phases (from the research plan)

Follow `VIBETERM-UI-ARCHITECTURE-RESEARCH-PLAN-v0.1.md` (§8). **Phase 0 first — the plan is sharpened,
read its §0.3 (frozen/open) before any extraction, or Phase 1 will ratify instead of earn.**
- **Phase 0 — framing (no commits):** resolve RP-B/RP-E with the owner; lock **RQ1–RQ17**; write the §4
  design-space map **under the frozen/open framing**; turn its claims into predictions **P1–P8**; sketch
  the AI-UI evaluation matrix (RQ17) so the parity predictions are measurable from the start.
- **Phase 1 — internal methodology extraction:** from our own specs (§2 here), write the
  **"ports / adapts / new" table** for the full vertical (entities, MVC/state, actions/AIUI,
  Search Everywhere, i18n, visual language + design system) into the **D1 findings doc** at
  `research/vibeterm/vibeterm-ui-architecture-findings-v0.1.md`. Include the **identity-grammar
  conformance surface** (RQ12) — the minimum the Rust and TS cores share + the golden's shape. Commit.
- **Phase 2 — external comparative** (clean-room, docs-first; RP-B per-source depth). Commit.
- **Phase 3 — pitfalls → design obligations** (incl. the new §5 pitfalls: two-core drift, capability
  hole, AIUI-plane proliferation, double-truth, GUI-only unknowns, the unmeasured AI-UI). Commit.
- **Phase 4 — synthesis → numbered architecture deltas**, each naming a prospective vibeterm
  contract REQ; predictions P1–P8 checked against the evaluation matrix; the AI-UI evaluation-matrix
  results; findings REPORT. Gate: findings doc complete.

THEN (downstream, separate sessions): **D2 design-doc** (vibeterm-owned, under
`spec/modules/vibeterm/`) → **D3 contracts** (the vibeterm PROP family + revised PROP-044 with
AI-UI-readiness REQs) → **D4 build** (VIBETERM-SHELL-PLAN, milestone 1).

---

## 6. My thinking on the research (working design-space, to be earned in Phase 1)

- **Render-free engine in TS.** The tab registry, pane-layout maths, session model, and the
  protocol codec form a **cell** that imports **no** Solid/DOM/Electron types; add a
  dependency-boundary lint to the floor. This is the invariant everything rests on (PROP-039
  `#no-render-dep`, ported).
- **The `ModelView` is a window→tab→pane TREE** (richer than the TUI's single-screen snapshot):
  windows[] → tabs[] (id, title, kind, active) → panes[] (which tab, bounds), plus `compact`,
  `activeWindow`, enabled actions + reasons. Events (`opened/closed/active-changed/moved`) are its
  deltas; change by re-resolution. The Solid chrome **renders** it; the AI **reads** it.
- **Every chrome command is a named `action://vibeterm/*`** (`tab.open`, `tab.select?id`,
  `pane.split?target&dir`, `tab.close?id`, `tab.move-to-window?tab&window`, `view.set-compact?on`,
  `theme.set?id`, `locale.set?id`). The Solid chrome and the AI both call ONE `invoke`. Nothing
  pixel-only. Enablement + "why disabled" per action (chrome greys menu items; AI reads it).
- **AI-UI surface = a peer client** exposing the four verbs against the same engine; the reference
  surface. CDP stays observation-only. The per-tab AIUI = a `Surface` whose ModelView scopes to a
  `TabId` — falls out for free.
- **Search Everywhere** = the same provider model, surface-neutral; providers: sessions/terminals,
  actions, profiles(later); GUI-rendered (cmdk/Kobalte); only the row renderer differs.
- **i18n** ports directly (Fluent has a JS runtime); address-keyed; legibility gate in CI; en/ru.
- **Design system** = the GUI twin of the Theme: **semantic design tokens** (colour/space/radius/
  typography roles) → **themes are token sets** → **one source projected** (dark-purple +
  Anthropic); live switch by rebinding CSS custom properties; components reference roles, never
  hex (the foton anti-pattern: hardcoded hex everywhere — avoid). Accessibility/density modes are
  the GUI analogue of the TUI's tiers.
- **Predictions to check (P1–P8):** every PROP-039 concern is zero-render-dep in TS; the ModelView
  generalises to a window/tab/pane tree with no new mechanism; the self-contained TS re-expression
  keeps a methodologically-compatible grammar; "one theme projected across tiers" → "one token set
  projected across themes/modes" (no tier analogue); no mainstream GUI app has a render-free
  AI-drivable action core with a serialisable ModelView as the reference — we lead.

**Working pillars added in this sharpening (to be earned, not assumed):**
- **Conformance, not coupling (RQ12).** The Rust `vibe-actions` and the TS `vibeterm-core` share an
  identity-grammar spec + a CI golden — no build-dep, but no silent drift either. This is the
  reconciliation of RP-A's "self-contained" with the two-core-drift risk; the minimum shared surface is
  itself a research output.
- **Capability surface for an AI peer (RQ13).** Once the AI has the same `invoke` as a human, `Dangerous`
  actions + caller identity + granted scope + scope-REFUSE become load-bearing — inert in the TUI, a real
  security surface here. The engine never trusts a self-reported scope.
- **One AIUI, not two (RQ14).** The new semantic-invoke and the legacy `vibe aiui` three planes must be
  declared one surface (extension / fourth plane / replacement); a single CLI addresses both the shell
  (`vibeterm/*`) and the hosted `vibe tree` (`vibe.tree/*`) without forking.
- **A measurable AI-UI (RQ17).** "As well as a human" is an evaluation matrix — a task set driven both
  ways and compared on success / latency / observability — not a slogan.

---

## 7. Constraints & discipline (MUST follow — repo rules)

- **NEVER write the reference app's real name anywhere in the repo, git history, or agent chat.** The reference app is
  codenamed **"ProjectX"** (its real name lives only in out-of-git user-memory). OUR feature is the
  **VibeTerm shell** (a VibeTerm capability), never "ProjectX". "ProjectX" appears in-repo only as
  the *reference* (e.g. `refs/screens/projectx/`).
- **Commits:** heredoc only (`git commit -F - <<'MSG' … MSG`); **NO AI attribution** (Rule 1,
  human-authored surface); **Conventional Commits**; **atomic** (one idea per commit). Routine
  proceeds; non-routine (history rewrite, force-push, large blobs, CI/secrets) stops for the owner.
- **Mirror push** is `cargo xtask mirror` (GitVerse + GitHub, fast-forward-only) — deliberate, at
  a checkpoint. `git push origin` hits only GitVerse.
- **Edits via Edit/Write ONLY** (PowerShell 5.1 corrupts UTF-8-no-BOM round-trips); self-check via
  Git Bash (`bash tools/self-check.sh`), check the REAL exit code.
- **Cadence:** research → design → execution; do NOT one-shot; read prior art fully. Architecture
  is the owner's zone — surface decisions.
- **Delegation-first:** delegate mechanical execution (fractality GLM workers); keep Claude for
  architecture/judgment/review; the architecture understanding stays boss-held.
- **Boot each session:** `CLAUDE.md` → `spec/boot/` (INDEX/STATIC + the AI-Native Rust/TS stacks +
  redbook flows) → `spec/WAL.md` → `CONTINUE.md` → then this `task.md`.

---

## 8. Git state (update on each checkpoint)

As of 2026-07-19, on `main`, ahead of `origin/main` (unpushed unless a `cargo xtask mirror` ran).
Recent relevant commits (newest first): the research plan + its RP-A/RP-D self-contained
resolution + the relocation of both plans into `research/vibeterm/`; the shell campaign plan
(gated); PROP-044 contract; the `.gitignore` guard for ProjectX captures. `git log --oneline -12`
shows the chain. **The Phase-0 reparent spike is throwaway (session scratchpad), NOT committed.**

## 9. Quick-start to resume

```sh
# boot: CLAUDE.md → spec/boot/ → spec/WAL.md → CONTINUE.md → THIS FILE
# read §2 prerequisite files (esp. PROP-039, action-system.md, PROP-037, tui-visual-language.md)
# then resume at §5 Phase 0: get RP-B/RP-E from the owner, write the design-space map, start Phase 1.
# floor (any crates change): bash tools/self-check.sh
# TS floor (apps/vibeterm, once bootstrapped): npx tsc --noEmit && npx vitest run ; node --test
```

**Pointer.** `spec/WAL.md` (its `_Updated:` line) is the canonical living state; the research plan
and shell plan in `research/vibeterm/` are the task drivers; this `task.md` is the cold-start index.
