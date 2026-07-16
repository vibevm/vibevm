# PROP-037: `vibe tree` — the interactive TUI application {#root}

**Status:** DRAFT — requirements, 2026-07-15 (owner-commissioned); **revised
2026-07-15 (Spec 2)** onto the action system: the TUI is a `Surface` on
`vibe-actions` (PROP-039), every command is an addressed action (§13), Search
Everywhere (§7.3) is promoted from a stub to a shipped feature, and the i18n
mechanism (§1.6) is now real. Extends
[PROP-036 §2.11](PROP-036-package-tree.md#tui) (the analyzer's TUI sketch) into
a full application contract.
**Related:** [PROP-036](PROP-036-package-tree.md) (the `vibe tree` analyzer +
the `PackageTree` model this app renders); **[PROP-039](../vibe-actions/PROP-039-action-system.md)
(the `vibe-actions` action system this TUI is built on — §13)** + its design-doc
[`spec/design/action-system.md`](../../design/action-system.md); [core-ai-native
discipline](`spec://org.vibevm.ai-native/core-ai-native`) (§11). Plan:
`spec/terraforms/TREE-TUI-PLAN-v0.1.md`.
**Language:** the shipped UI is **English** (§1.6); the **i18n mechanism ships**
(PROP-039 §8, §13.4) with English the only mandatory-complete locale — no
non-English catalogues are shipped now (§12).

This contract is deliberately **granular and addressable** (owner directive):
every feature is its own `{#anchor}` REQ, cited by the code via `specmark`, so a
large feature set stays a map rather than a tangle. A REQ is the unit of work.

---

## 1. Architecture — layers {#architecture}

The application separates, as fixed layers, (a) the vibevm data it renders, (b)
its own model + UI state, (c) the reusable view components + their styling, and
(d) the event/control logic. The one-line law: **styling never leaks into
control or app logic, and vibevm logic never leaks into the app.**

### 1.1 The four layers {#layers}

REQ. The code is organized as four layers with one-directional dependencies
(each may depend only on those above it):

1. **vibevm backend** — `vibe-core` / `vibe-workspace` / `vibe-spec` (the
   `PackageTree` builder, PROP-036). Unowned by this app.
2. **Model** — the app's data: the borrowed/owned `PackageTree` plus all UI
   state (§1.3). Pure data + logic-on-data; **no rendering, no event types**.
3. **View** — the component library (§2) and the theme (§2.2). Renders Model +
   component state into a ratatui buffer; **no app control flow**.
4. **Controller** — event routing, the keymap registry (§5.1), the modal stack
   (§6), and the actions that mutate the Model (§1.5).

The MVC cycle: event → Controller (routes to the focused component / top modal,
produces an Action) → Action mutates Model → View renders Model + component
state through the Theme.

### 1.2 The vibevm boundary {#backend-boundary}

REQ. The app consumes vibevm through exactly one seam — the `PackageTree` model
(PROP-036) and a small read interface over it. No view or controller code
reaches into `vibe-core` / `vibe-workspace` / `vibe-spec` types directly; if the
app needs a vibevm fact, it is exposed as a method/field on the app Model's
projection of `PackageTree`. This keeps the analyzer free to evolve without
dragging the UI with it.

### 1.3 Model {#model}

REQ. The Model owns: the analyzed `PackageTree` (read-only vibevm data); and the
**UI state** — the active display mode (§4), the sort/shape configuration *per
context* (§3.2, §7.2), the fold set + selection + horizontal pan, the active tab,
the sub-table block order, and the loaded settings (§9). The Model exposes
queries and mutating methods; it never renders and never names a key or a
crossterm event. Model logic (filtering, ordering, flattening) is unit-tested
without a terminal.

### 1.4 View — components + theme {#view}

REQ. All on-screen output is produced by the component library (§2). A component
renders itself from its own small state plus the Model and the `Theme`; a
component never reads keys or mutates the Model. The **Theme** (§2.2) is the sole
source of colors, styles, spacing, and glyphs — the "CSS" — so a restyle touches
only the theme, never component logic.

### 1.5 Controller — events, keymap, modals, actions {#controller}

REQ. The Controller receives each terminal event and routes it: to the **top
modal** if the modal stack (§6) is non-empty, else to the focused base-screen
component, else to the global keymap. Routing is driven by the **mode-aware
keymap registry** (§5.1), never by an ad-hoc `match` scattered across the app.
An event resolves to an **Action** (a typed intent — e.g. `OpenModeMenu`,
`ToggleFold`, `Quit`); Actions mutate the Model. A terminal resize always
requests a repaint (already fixed; the repaint law: rat-salsa repaints only on
`Control::Changed`).

### 1.6 English-only, i18n-ready {#i18n}

REQ. All user-facing strings are English and live behind a single string
accessor (a `strings` module / a `label(key)` indirection), so a future
localization layer swaps the source without touching call sites. No localization
is shipped now (§12); the indirection is the only present obligation.

---

## 2. The component library {#components}

A reusable, vibevm-owned widget library — the abstraction that stops "a million
implementations of the same control" from accumulating.

### 2.1 Component strategy — wrap, extend, or invent {#component-strategy}

REQ. Components are built in this order of preference: **(1) wrap** a `rat-widget`
widget behind our `ui::` API + Theme (Menu, Button, TextInput, scrolling,
Tabbed, popup, MsgDialog — reuse, do not reinvent); **(2) extend** in
rat-widget's idiom when a needed widget is not in rat-widget — author it as
though continuing the rat-widget family, but on the vibevm side; **(3) invent**
on bare `ratatui-core` only as a last resort, when the need is unthinkable within
rat-widget's ideology. Every component has exactly one implementation behind one
`ui::` facade; call sites never touch `rat_widget::` directly.

### 2.2 The theme {#theme}

REQ. A single `Theme` value carries every color, text style, border style,
spacing, and glyph the components use (selection highlight, window chrome, group
frames, the `[x]` close glyph, subheaders, disabled/enabled button, etc.).
Components take `&Theme`; no component hard-codes a `Color`/`Style`. The theme is
terminal light/dark aware where it matters. The theme is the TUI's "CSS"
(§1.4): a restyle touches only the theme.

The visual language is **a first-class part of this contract**, not an
afterthought. The lore (aesthetics, rationale, the full tables the eye reads)
lives in the design doc
[`spec/design/tui-visual-language.md`](../../design/tui-visual-language.md);
the normative REQs the code is traceable to are the four anchors below.

#### 2.2.1 Palette tokens {#palette-tokens}

REQ. Colour reaches a component only through a **`Palette`** — a data-driven
mapping from **semantic role tokens** to `Color`. The role set is exactly:
`base`, `surface0`, `surface1`, `muted`, `subtext`, `text`, `accent`, `love`,
`gold`, `foam`, `rose`, `selection`, `border`, `paper`, `button_on`,
`button_off`. No component names a `Color` literal; it names a role.

REQ. Five palettes ship, each a complete role→`Color` mapping. **Rosé Pine**
(the cosmic-violet look already in `theme.rs`) is **canonical-locked**: its
eleven `Color::Rgb` values are preserved exactly (a snapshot test pins them).
**Catppuccin Mocha, Macchiato, Frappé** (dark) and **Catppuccin Latte** (light)
are the canonical Catppuccin values (`accent`←mauve, `love`←red, `gold`←yellow,
`foam`←teal, `rose`←pink, `muted`←overlay0, `subtext`←subtext0). The canonical
hex values for all five are normative in the design doc §3.

REQ. A palette carries an `is_light` flag; `selection` is composed (`accent`
ground + `base` text, bold), `border` = `muted`, `paper` = `surface0`, and the
`paper`/`selection` rendering inverts against `is_light` so a light theme's
detail card reads correctly. The active palette is a `Model` field, persisted
through the settings system (§9) and overridable at the CLI/env.

#### 2.2.2 Glyph vocabulary {#glyph-vocabulary}

REQ. Every glyph is a constant on the `Theme`, never a hardcoded string at a
call site. The fold indicator is `▾`/`▸` (not `+`/`-`); the DAG re-occurrence
marker is `↩` (not `(*)`); the on/off flags are `●`/`○` (not `x`/`.`). Tree
connectors stay `│├└─`; the frame stays rounded `╭╮╰╯`; the close affordance is
`✕`; the bar indicator uses block elements `▁▂▃▄▅▆▇█` (or braille). The full
replacement table is normative in the design doc §2.

REQ. In the primary UI (Tier ≥ 1) there are no `+`/`-`/`*`/`#`/`x`/`.` used as
**semantic** glyphs. Those ASCII characters appear only behind the Tier 0
fallback (§2.2.3).

#### 2.2.3 Rendering tiers {#rendering-tiers}

REQ. Rendering degrades through four tiers — **3** (truecolor: full RGB, rounded
frames, braille/blocks), **2** (256-colour: palette quantised to the 6×6×6 cube,
rounded, blocks), **1** (16 ANSI: role→ANSI mapping, rounded-or-square frames,
blocks), **0** (dumb / `TERM=linux` / no Unicode: ANSI mono, ASCII `+-|` frames,
`#` indicators).

REQ. Tier detection is a **pure function** over the environment —
`detect_tier(colorterm: Option<&str>, term: Option<&str>) -> Tier` (`$COLORTERM`
first, then `$TERM`; `crossterm` exposes no colour-count API). The TUI reads the
env once at launch in a sanctioned spot and feeds the values in; the detected
tier is overridable through the settings system (§9).

REQ. Degradation is a **projection**: one `Theme` is built for Tier 3 and
projected onto the detected tier (roles quantised / ANSI-mapped / ASCII-fallback).
One source of truth, many projections — never bespoke per-tier rendering in a
component.

#### 2.2.4 Window aesthetics {#window-aesthetics}

REQ. A window (§2.3) is not a fallback: it composes a solid `base` panel, a
rounded frame stroked in `border`, a title rendered as an `accent`-coloured chip,
interior padding, an optional shadow (so the panel reads as raised), and a
top-right `[✕]` close affordance. Even at Tier 1 the frame + filled panel +
title chip carry the "floating panel" reading without truecolor.

### 2.3 Window / Panel {#window}

REQ. A `Window` component: a bordered, titled region drawn over a cleared rect,
with an optional title and an optional top-right `esc [x]` close affordance
(§8). It is the base of every modal and the card. It composes child components
and lays them out; it owns no app logic.

### 2.4 Menu — the dropdown list {#menu}

REQ. A `Menu` component: a centered, window-framed list of options; `↑`/`↓` move
the highlight (the highlighted item is styled), `Enter` confirms the highlighted
option, `Esc` cancels. This is the canonical "dropdown menu" design referenced
throughout this contract (F3 mode menu §7.1; sort options §7.2).

### 2.5 Button {#button}

REQ. A `Button` component: a labelled, focusable control; the focused button is
highlighted; `Enter` activates it. Buttons participate in a window's focus order
(arrow / Tab navigation). Used by dialogs (OK, Save, Cancel).

### 2.6 Group / Fieldset {#group}

REQ. A `Group` component: a visually delimited cluster of child components
(color / padding / a border frame), with an optional group name rendered at the
frame's top-right corner. Groups give a multi-setting dialog its visual
structure (§7.2).

### 2.7 RadioGroup {#radio-group}

REQ. A `RadioGroup` component: a `Group` whose children are mutually-exclusive
options; exactly one is selected; `↑`/`↓` move, `Enter`/`Space` selects. Backs
single-choice settings (the sub-table block order, §7.2; the copy format/dest,
§10.2).

### 2.8 TextField {#text-field}

REQ. A `TextField` component: a single-line editable text input (the file-path
entry, §10.5). Simple for now (a later REQ enriches it); wraps rat-widget's text
input where possible (§2.1).

### 2.9 Card / Form {#card}

REQ. A `Card` component: a `Window` laid out as a labelled vertical form — see
§8 for its full design contract (background/foreground, bold headers, line
spacing, wrapping, per-line copy).

### 2.10 The standard "Coming Soon" modal {#coming-soon}

REQ. A single reusable `ComingSoon` modal — a `Window` titled with the feature
name, a "coming soon" body, and an `OK` button (`Enter`/`Esc` closes). It is the
standard placeholder for **every** not-yet-implemented feature (F1 Search
Everywhere §7.3; PNG export §10.4; any future stub). Wiring a feature's entry
point to `ComingSoon` is how a feature is "reserved" before it is built.

---

## 3. The Tree widget + filter pipeline {#tree-widget}

The core reusable abstraction: one tree widget, fed by a configurable pipeline.

### 3.1 The Tree widget {#tree}

REQ. A single `Tree` component renders a forest of `PackageTree` nodes — the
`│├└` glyphs, the `+`/`-` expand indicator, the load/`T`/`C`/`S` columns, the
`(*)` DAG dedup + cycle-guard (PROP-036 §2.12), selection highlight, vertical
scroll, and horizontal pan. **Every mode (§4) renders through this one widget** —
there are no bespoke flat-list renderers. Fold state and selection are the
widget's (per-instance) state.

### 3.2 The filter / shape pipeline {#tree-filters}

REQ. Data reaches the `Tree` widget only after passing a configurable
**pipeline**: `PackageTree` → (filter: which packages) → (shape: how they form a
forest) → (order: sibling ordering) → the widget's row list. The pipeline is
data-driven and testable in isolation; a mode (§4) is a pipeline configuration,
not new rendering code. The three tree *shapes* (§3.3) and the orderings (§7.2)
are pipeline settings, selectable by the user and persisted (§9).

### 3.3 The three tree shapes {#tree-shapes}

REQ. The pipeline offers three shapes, selectable per context on the F2 sort menu
(§7.2), **default = (a)**:

- **(a) members-as-roots + full subtrees** — each package in the filter set is a
  forest root shown with its entire dependency subtree (cross-type deps
  included); DAG dedup via `(*)`.
- **(b) load-type forest** — a package is a root only if no other member of the
  same filter set depends on it; children are its same-set dependencies
  (cross-set deps omitted).
- **(c) pruned tree** — the tree from the declared roots, keeping only branches
  that reach a member of the filter set.

All three are pipeline configurations over §3.2 — implementing them validates
that the filter/shape abstraction is right.

---

## 4. Modes {#modes}

REQ. `vibe tree` has three display modes, each a Tree-widget (§3.1) configuration
(§3.2) — none is a flat list:

### 4.1 Tree mode {#mode-tree}
REQ. One tree over the whole package set (the current default). Filter = all;
shape + order per §3.2/§7.2.

### 4.2 Sub-tables mode {#mode-subtables}
REQ. **Several trees stacked vertically** — one Tree instance per effective-load
partition (`static` / `dynamic` / `no-boot`), each under a subheader, in the
user-chosen block order (§7.2). Each block is a full tree (per §3.3), not a flat
list.

### 4.3 Tabs mode {#mode-tabs}
REQ. **One tree per tab** — a tab bar (`Static` / `Dynamic` / `No-boot`); the
active tab shows that partition's tree (per §3.3). `Shift`+`←`/`→` switches tabs
(§5.3); plain arrows navigate the active tab's tree.

### 4.4 Mode selection {#mode-select}
REQ. The active mode is chosen from the F3 menu (§7.1) — not a bare cycle key.
The active mode is shown in the status line and persisted (§9).

---

## 5. The keymap {#keymap}

### 5.1 The mode-aware registry {#keymap-registry}

REQ. A single **keymap registry** maps each context (base mode, and each modal)
to the set of actions available there, each with its key(s) and its footer
label. The footer (§5.2) renders **only** the actions valid in the current
context — an action unavailable in a mode (e.g. sub-table block reorder in tree
mode) is absent from both the keymap and the footer. Routing (§1.5) consults the
registry; no context re-implements dispatch.

### 5.2 The F-key scheme + footer {#keys}

REQ. Primary actions are on function keys: `F1` Search Everywhere (§7.3), `F2`
sort menu (§7.2), `F3` mode menu (§7.1), `F6` copy / `Shift+F6` copy-settings
(§10.2). The footer lists the active keys with short labels; **`Shift` is written
as `↑`** (e.g. `Shift+F6` → `↑F6`) to keep hints short.

### 5.3 Tree interaction keys {#tree-keys}

REQ. Within a tree: `↑`/`↓` move selection (+ scroll), `←`/`→` pan horizontally,
`Space` folds/unfolds the selected node, `Enter` opens the detail card (§8). In
tabs mode, `Shift`+`←`/`→` switches tabs (plain `←`/`→` stay tree-pan, which is
why tab-switch takes Shift). These direct keys are exempt from the F-key scheme
(they are navigation, not commands).

---

## 6. The modal stack {#modal-stack}

REQ. Modals form a **stack**: opening a modal pushes it; it draws over
everything below (each lower layer still visible as backdrop); input goes only to
the top modal; `Esc` pops the top modal (returning to the one beneath). A feature
may push a modal over a modal — e.g. copy-settings (§10.2) → file-path (§10.5) is
a depth-2 stack. The base screen is the bottom of the stack; `Esc` at the bottom
triggers quit-confirm (§7.4), it does not pop.

---

## 7. Menus & dialogs {#dialogs}

### 7.1 F3 — the mode menu {#f3-mode-menu}
REQ. `F3` opens a `Menu` (§2.4) of the display modes (§4). Selecting one switches
the mode. This replaces any bare mode-cycle key.

### 7.2 F2 — the sort menu {#f2-sort-menu}
REQ. `F2` opens a dialog whose content depends on the active mode:
- **tree & tabs modes** — one `RadioGroup` (§2.7): `alphabetical` / `topological`
  (the sibling order, §3.2). One group; no group chrome needed.
- **sub-tables mode** — two `Group`s (§2.6): a "sort" group (`alphabetical` /
  `topological`) and a "block order" group (a `RadioGroup` over the static /
  dynamic / no-boot block orderings). Group names sit at each frame's top-right.
The tree **shape** (§3.3) is also chosen here (a further group), per context.
Choices persist (§9).

### 7.3 F1 — Search Everywhere {#f1-search}
REQ. `F1` opens the **Search Everywhere** window — the `vibe-actions` Search
Everywhere engine (PROP-039 §10), in the IntelliJ IDEA idiom: a hybrid **"All"**
tab that searches everything, plus **per-category tabs** that narrow it
(`Tab`/`Shift+Tab` cycle; the "All" tab carries a category checkbox filter). Three
providers ship (PROP-039 §10.4):
- **Packages** — by name (`PackageProvider` over the `PackageTree`); selecting
  reveals the package in the tree.
- **Card fields** — inside **every field** of the package detail cards (§8)
  (`PackageFieldProvider`: name, version, kind, license, load-type, origin, path,
  deps, diagnostics…); selecting opens the card focused on that field.
- **Actions** — all `vibe.tree` actions (§13.5) by address, **name, and
  description** (`ActionProvider`); selecting **invokes** the action in place (a
  command performs and closes; a toggle stays open). Disabled actions render greyed
  with their "why disabled" reason and their keybinding.

Matching falls back to the **name/description** lane when nothing matches by id or
another field (PROP-039 §10.3). Results render through one normalized row (icon ·
primary · a right-aligned keybinding · group). This **supersedes** the reserved
`ComingSoon` stub. A future `StructureProvider` (AI-Native specmap nodes) plugs into
the same engine with no TUI change.

### 7.4 Escape — quit with confirmation {#quit-confirm}
REQ. At the base screen, `Esc` opens a confirm dialog ("Really quit?") with
`Yes`/`No` buttons; `Yes`/`Enter`-on-Yes quits, `No`/`Esc` returns. The
confirmation exists because loading may be slow and `Esc` is used pervasively in
modals — an accidental single `Esc` must not discard the session.

---

## 8. The detail card {#detail-card}

REQ. `Enter` on a package opens the detail **Card** (§2.9) — a real form, not a
glued text blob:
- a light panel background with dark text (a "paper" card), distinct from the
  tree beneath;
- **bold** field headers; blank-line / padded spacing between fields;
- an `esc [x]` close affordance drawn as a pseudographic cross in the top-right
  corner;
- long values **wrap** (never truncate) and each wrapped value line can be
  **copied to the clipboard** (a per-line copy affordance);
- `Esc` / the `[x]` closes it.
The card's content is the package detail from PROP-036 §2.11 (name, group,
version, kind, load, transitive + why, condition, STATIC.md, source, hash,
dependencies, boot path).

---

## 9. Settings persistence {#settings}

REQ. UI state persists under `~/.vibe/tree/` as one or more JSON files. Saved:
the active mode (§4.4), the per-context sort + shape + block-order choices (§7.2,
§3.3). On TUI launch the settings are loaded and the UI restored to them; a
missing/corrupt file falls back to defaults (never a hard error). Writing is
atomic; the directory is created if absent. (This is the machine-global
`~/.vibe/` config root, distinct from the project's `.vibe/`.)

---

## 10. The copy system {#copy}

### 10.1 Per-screen copy providers {#copy-providers}
REQ. Each screen (tree/sub-tables/tabs; the card) supplies a **copy provider** —
an object that knows how to serialize *what is currently shown* into a copy
format. A tree provider serializes the tree with its current display options; the
card provider serializes the card's fields. Copy is "what I see is what I copy":
in tabs mode only the active tab is serialized.

### 10.2 F6 copy / Shift+F6 copy-settings {#copy-flow}
REQ. `F6` copies the current screen straight to the clipboard as text (the
provider's default format). `Shift+F6` opens the **copy-settings** modal: a
`RadioGroup` for **format** (Markdown / PNG — and later PlantUML / Mermaid) and a
`RadioGroup` for **destination** (clipboard / file). Confirming with destination
= file pushes the file-path modal (§10.5) over it (§6 stack).

### 10.3 Markdown export {#copy-markdown}
REQ. Markdown is a real serialization, not a screen-buffer scrape: the tree
renders as a pseudographic tree inside a fenced block (or a nested list); the
card renders as well-formatted Markdown (headers + fields), not a raw text dump.

### 10.4 PNG export {#copy-png}
REQ. PNG export (a rasterized tree image; a designed package info-card image) is
**reserved** — selecting PNG opens the `ComingSoon` modal (§2.10) until the
rasterization (font + image crates) is built. Named non-goal-for-now (§12).

### 10.5 Destination — clipboard vs file {#copy-dest}
REQ. Clipboard writes via the platform clipboard. File destination pushes a
modal with a `TextField` (§2.8) path entry plus `Save` and `Cancel` `Button`s
(§2.5): arrow/Tab focus, `Enter` on a button acts, `Esc` cancels back to
copy-settings. The path field is plain text for now (a later REQ enriches it).

---

## 11. AI-Native Rust discipline {#discipline}

REQ. All code implementing this contract follows the AI-Native Rust discipline
(`spec://org.vibevm.ai-native/core-ai-native`): cells with single registration
points and no sibling-cell coupling; `specmark::scope!` on every file citing the
governing PROP-037 anchor; per-fn `#[spec(implements = "spec://…#…")]` where a fn
implements a specific REQ; `anyhow` at the command edge, typed errors citing REQs
in library layers, no `unwrap`/`expect` in domain logic; the `≤600`-line file
budget; and green `conform` + `specmap` at every commit. Each REQ anchor here is
an addressable target the code traces to — that traceability is what keeps the
feature set navigable.

---

## 12. Non-goals {#non-goals}

- **Non-English localization content** — the i18n *mechanism* ships (PROP-039 §8,
  §13.4); no non-English catalogues are shipped now, and English is the only
  mandatory-complete locale.
- **PNG export** — reserved behind `ComingSoon` (§10.4) until the rasterization
  spike lands.
- **The AIUI surface itself** — not built now; this TUI is its prototype (§13.1,
  PROP-039 §11.3). The Search-Everywhere `StructureProvider` (AI-Native specmap
  nodes) is likewise reserved (§7.3) — the same engine, added later.
- **PlantUML / Mermaid copy formats** — later additions to §10.2.
- **A settings UI** — settings are edited via the menus (§7) and persisted (§9);
  no dedicated settings editor screen.
- **Non-tty operation** — `--json` / `--plain` (PROP-036) remain the machine and
  fallback surfaces; this contract governs the interactive TUI only.

---

## 13. Built on the action system (`vibe-actions`, PROP-039) {#action-system}

This TUI is the **first consumer** and the **prototype surface** of the action
system (PROP-039). This section adapts the architecture above onto it and is
**authoritative** where it upgrades an earlier section.

### 13.1 The TUI is a Surface; the Model is the serialisable view {#as-surface}
REQ. The TUI is a `Surface` (PROP-039 §11.1) over `vibe-actions`; it owns rendering
and event capture and nothing of the action core. The Model (§1.3) is the source of
the **serialisable `ModelView`** (PROP-039 §11.2) — focus, the modal stack, the
visible rows, the active tab/mode, the selection, and the set of enabled actions
with their reasons. No rendering type leaks into the Model. This makes the TUI the
prototype that proves the **AIUI** (PROP-039 §11.3): the same tree can later be
driven headless with no change to this crate's model/controller.

### 13.2 Commands are addressed actions {#as-actions}
REQ. Every TUI command is a `vibe-actions` **Action** (PROP-039 §3) in the group
**`vibe.tree`**, addressed `action://vibe.tree/<name>`, carrying a mandatory
human-readable **name + description** (§13.4), a typed param schema, and a typed
enablement over a `TreeCtx` snapshot (the mode, the selection, the active tab). This
**upgrades §1.5's "typed intent enum"**: the Controller resolves an event to an
`ActionAddr` and calls `invoke` (PROP-039 §7.1); it no longer switch-matches a local
enum.

### 13.3 The keymap binds keys to addresses {#as-keymap}
REQ. The mode-aware keymap (§5.1) binds each key/chord, per context, to an
`(action://vibe.tree/<name>, params)` (PROP-039 §9). The footer label (§5.2) is the
action's **name**; the footer lists exactly the actions **enabled** in the current
context (their enablement predicate, PROP-039 §6.2). Tree navigation keys (§5.3)
remain direct (navigation, not commands).

### 13.4 i18n is real {#as-i18n}
REQ. This **upgrades §1.6** from "i18n-ready" to the shipped `vibe-actions` i18n
(PROP-039 §8): every action and UI string is an address-keyed catalogue entry
(`action.vibe.tree.<name>.name` / `.description`) with an inline English default;
the resolved label keeps `{value, original_en}` so Search Everywhere (§7.3) matches
the English text under any locale. English is the only mandatory-complete locale and
is checked by the legibility gate (PROP-039 §8.4).

### 13.5 The action catalogue {#action-catalogue}
REQ. The `vibe.tree` actions at ship — each an addressed Action with a name +
description; the **key is its default binding** (the map, not the identity):

| Address (`action://vibe.tree/…`) | Key | Name | Description |
|---|---|---|---|
| `search.everywhere` | `F1` | Search Everywhere | Search packages, card fields, and actions; run a found action. |
| `sort` | `F2` | Sort & shape… | Choose the ordering and tree shape for the current view. |
| `mode.set` | `F3` | Switch mode… | Switch between tree, sub-tables, and tabs display. |
| `copy` | `F6` | Copy | Copy the current screen (Markdown) to the clipboard or a file. |
| `copy.settings` | `↑F6` | Copy settings… | Choose the copy format and destination. |
| `fold.toggle` | `Space` | Fold / unfold | Fold or unfold the selected node. |
| `card.open` | `Enter` | Open details | Open the detail card for the selected package. |
| `tab.next` / `tab.prev` | `↑→` / `↑←` | Next / previous tab | Move between tabs in tabs mode. |
| `quit` | `Esc` (base) | Quit | Leave `vibe tree` (with confirmation). |

REQ. This catalogue is the **enumerable** source for both the footer and the Search
Everywhere `ActionProvider` (§7.3): adding a command means registering an Action
(address + name + description + enablement), which then appears in the footer, the
keymap, and Search Everywhere with no further wiring (PROP-039 §4.3, §12.2).

### 13.6 What this leaves to PROP-039 {#as-boundary}
REQ. The action core — address, registry, params, context, invoke, i18n, the keymap
resolver, the Search Everywhere engine + provider trait, and the Surface seam — is
owned by `vibe-actions` (PROP-039); this contract owns only the **vibe.tree
specifics**: the concrete actions (§13.5), the three providers' key/resolve/navigate
(§7.3), the `TreeCtx` shape, the F-key map, the theme, and the TUI Surface. The
layering law of §1 is extended: **the action core never leaks into the TUI, and TUI
rendering never leaks into the action core.**
