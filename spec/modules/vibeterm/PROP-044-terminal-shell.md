# PROP-044 — VibeTerm terminal shell: tabs, panes, windows

**Status:** DRAFT (v0.1, 2026-07-19) — milestone-1 target, not yet implemented.
**Module:** `vibeterm` (new). **Campaign:**
[`research/vibeterm/VIBETERM-SHELL-PLAN-v0.1.md`](../../../research/vibeterm/VIBETERM-SHELL-PLAN-v0.1.md).
**Related:** PROP-042 (AIUI observation — the headless single-view path this
contract must NOT disturb), PROP-043 (GUI launchers — `vibeterm.exe` opens this
shell), PROP-036 / PROP-037 (the `vibe tree` model + TUI, hostable as a tab),
PROP-040 / PROP-041 (settings — the future home of profile restore).

This contract governs the **VibeTerm terminal shell** — the visible, multi-tab /
multi-window workspace VibeTerm presents. It generalises today's single-pty,
single-window VibeTerm into N terminals, each an **isolated web surface**,
switchable, splittable, and detachable into its own OS window **without
restarting or losing terminal state**. The layout draws on the **ProjectX
reference** (design captures at `refs/screens/projectx/`, out of git,
non-normative).

---

## 1. Shell regions {#regions}

REQ. The visible shell window composes five **named regions**, left to right: a
**profiles rail** (workspace/session profiles), a **quick-access rail** (pinned
shortcuts), a **terminal list** (the sessions in this window), a **content area**
(one or two panes hosting the active terminal view(s), §4), and a **top bar**
(window controls plus the compact toggle, §2). Visual style — palette, gradients,
the content field — is theme-driven (§11) and non-normative; it draws on the
ProjectX reference.

REQ. Milestone 1 **renders all five regions** but wires only the terminal list
and the content area (§7). The profiles rail and quick-access rail render as
static placeholders.

## 2. Compact toggle {#compact}

REQ. A control in the top bar, beside the menu button, **hides and shows the
profiles rail**, collapsing the shell to a more compact layout. The toggle is
functional in milestone 1; it is pure view state and touches no terminal.

## 3. The tab model {#tab-model}

REQ. A **tab** is exactly one terminal session: one node-pty child owned by the
**main process** bound to exactly one Electron **`WebContentsView`** (its own
renderer process, its own xterm.js). The main process holds the authoritative
registry `tabs: Map<TabId, { pty, view, ownerWindow, … }>`. `TabId` is stable
for the tab's lifetime and **independent of which window currently hosts it**.

REQ. pty ownership stays in **main** (as today — PROP-042, `apps/vibeterm/main.cjs`).
Per tab: bytes flow main → that tab's view, keystrokes flow view → main, resize
flows view → main. Every message is **addressed to a `TabId`** (or routed by the
sending `WebContents`), never broadcast — one window's keystroke never reaches
another tab's pty.

REQ. **The preservation invariant.** Switching (§4), splitting (§4), or moving a
tab between windows (§5) **never re-creates the tab's pty and never reloads its
`WebContentsView`**. xterm.js scrollback, cursor, viewport, and alt-screen state
survive every such operation by construction. This is the load-bearing property
of the whole design; see Decision [#d0-reparent](#d0-reparent) for the empirical
proof.

## 4. Panes: switching and split view {#panes}

REQ. A window's content area shows **one or two panes** (milestone-1 ceiling =
two — Decision [#d3-split-two](#d3-split-two)). Each pane hosts one tab's view,
positioned by the main process (`view.setBounds`); the non-visible tabs' views
are hidden (`setVisible(false)`), not destroyed.

REQ. **Switching** selects which tab occupies the (single) pane: the target view
is shown, the previous hidden. No reload (§3).

REQ. **Open in split view** adds the chosen terminal as a **second pane beside
the first**. Each pane carries a **close affordance** (the ×, top-right of the
pane). Closing a split pane returns the window to a single pane; the tab it held
(pty + view) **survives** and remains in the list unless explicitly closed.

REQ. On any pane geometry change (split, unsplit, window resize, compact toggle),
each affected tab's pty is **resized** to its pane's fitted grid — a reflow
(SIGWINCH), never a restart (§3).

## 5. Tear-off: open in new window {#tear-off}

REQ. **Open in new window** moves the tab's `WebContentsView` out of its current
window and into a **new `BrowserWindow`**, by reparenting the view
(`sourceWindow.contentView.removeChildView(view)` →
`newWindow.contentView.addChildView(view)`). The view's `WebContents` is **not
reloaded**; the pty (in main) is untouched (§3). The tab's `ownerWindow` is
reassigned and its pty resized to the new pane (§4).

REQ. When a window loses its last tab it closes; when it closes, its remaining
tabs' ptys are disposed and their views destroyed (a real session close, not a
move). Moving the *only* tab of a window into a new window therefore closes the
now-empty source window.

## 6. Context menu {#context-menu}

REQ. Right-clicking a terminal in the list opens a **context menu**. In milestone
1 the acting items are **Open in new window** (§5) and **Open in split view**
(§4). Additional items (rename, close-others, notification prefs, …) may render
as inert placeholders (§7) and are specified in a later revision.

## 7. Milestone-1 scope — placeholder vs functional {#m1-scope}

REQ. Milestone 1 is a **placeholder shell**. **Functional:** create / select /
switch terminals (list + content area), split view (§4), tear-off (§5), the
compact toggle (§2), the terminal context menu's two acting items (§6), **and the
foundations of §10 (i18n) and §11 (theming)** — those two systems are laid in
from the start, not bolted on later. **Placeholder (renders, inert):** the
profiles rail, the quick-access rail (showing at least the `vibetree` shortcut
with a wireframe icon), and all other top-bar / rail controls. Profile restore
and per-tab AIUI are **out of milestone 1** and named in the plan's deferrals.

## 8. The headless / control path is unchanged {#headless-unchanged}

REQ. `--headless` and `--control` (PROP-042 — the offscreen observation surface
an agent drives over HTTP and reads from the headless mirror) keep their
**current single-view, single-pty, offscreen behaviour**: no chrome, no tabs, one
`WebContents`. The shell is the **visible** `vibe term` experience only (Decision
[#d2-shell-default](#d2-shell-default)). No control-plane endpoint, discovery
file, or CDP behaviour changes in milestone 1.

## 9. The chrome ↔ engine seam {#seam}

REQ. The **chrome** (rails / list / tab-bar / pane frames UI) and the **terminal
views** are separate web surfaces. The chrome drives the **engine** (the
main-process tab registry, §3) through one **command channel** —
`open` · `select` · `split` · `close` · `move-to-window` · `set-compact` — and the
engine reports **tab-lifecycle events** back (`opened` · `closed` ·
`active-changed` · `moved`). The chrome never owns a pty and never addresses
another tab's view directly.

REQ. The command/event protocol is a **versioned, serialisable message contract**
— a discriminated union carrying **no Electron-specific types** — so its
**transport is swappable**. Milestone 1 transports it over **Electron IPC through
a typed preload bridge** (`contextIsolation: true`); the contract is shaped so a
later move to an **external state process** (a sidecar over HTTP / WebSocket /
stdio) needs a new transport adapter, not a redesign (Decision
[#d5-sidecar-ready](#d5-sidecar-ready)).

REQ. Per Decision [#d4-ui-stack](#d4-ui-stack) the chrome is authored in
**SolidJS** (Vite, Tailwind v4, Kobalte headless primitives) with strict TS; the
engine's pure logic — the tab registry, pane-layout maths, session model, and the
protocol codec — is typed TypeScript **cells** (`tsc --noEmit` + `vitest`), with
`TabId` a branded type, the command/event union exhaustively `tsc`-checked, and
seam failures typed values rather than untyped throws. **Terminal-view pages stay
lean vanilla TS + xterm.js** (no chrome framework), so N tabs are N lightweight
renderers. The thin Electron main entry (`main.cjs`) stays CommonJS interop.

## 10. Internationalisation {#i18n}

REQ. **Every user-facing string** in the chrome resolves through a **message
catalog** keyed by a stable message id; **no literal UI copy is hardcoded in a
component**. The active **locale is switchable at runtime**, no reload. Initial
locales are **English and Russian** (the owner works in Russian); the catalog
structure is the binding part and the locale set grows freely. (Decision
[#d6-i18n](#d6-i18n).)

## 11. Theming {#theming}

REQ. Visual style is expressed as **design tokens** — CSS custom properties for
colour / gradient / spacing / radius roles — and **components reference tokens,
never hardcoded hex**. The active **theme switches at runtime, live** (no reload,
no restart): switching rebinds the token values. (Decision
[#d7-theming](#d7-theming).)

REQ. The **first release ships two themes**: a **dark purple** theme (visually
after the ProjectX reference) and an **Anthropic-style** theme. Milestone 1 stands
up the token system and **both theme token sets**, even where the chrome that
consumes them is still placeholder. Theme ids are provisional; the token contract
is the binding part.

---

## Decisions {#decisions}

### D0 — `WebContentsView` reparent preserves live state {#d0-reparent}

- **Decision.** Each tab is its own `WebContentsView`; switching/splitting/moving
  a tab hides or reparents that view and never reloads it or its pty (§3, §5).
- **Why.** Verified by a Phase-0 spike on VibeTerm's own Electron **32.3.3**
  (Chrome 128): view moved A→B via `removeChildView`/`addChildView` with
  `reparentErr=null`, `webContents.id` identical before/after, **zero reloads**
  (`did-finish-load` fired once), the JS heap intact (session id stable, a timer
  kept counting 38→75), and a **real xterm.js** buffer identical before/after the
  move.
- **Considered and rejected.** (a) One renderer + hidden divs — no cross-window
  move without reload. (b) Serialize + rebuild via `@xterm/addon-serialize` in a
  fresh window — that *restarts* xterm.js (only the visible buffer is restored),
  violating the owner's explicit "without restarting xterm.js". (c) Legacy
  `BrowserView` — deprecated since Electron 30; `WebContentsView` is the successor.
- **When to revisit.** If a target Electron upgrade breaks `WebContentsView`
  cross-window reparent (the spike, re-run, regresses), or per-tab renderer memory
  becomes the dominant cost at the tab counts users actually reach.

### D1 — TypeScript core now, full gate later {#d1-ts-core}

- **Decision.** Author the engine's pure logic as typed TS cells with
  `tsc`+`vitest` from the start; defer wiring `eslint`/`conform`/`specmap` into
  the shell's floor until the shell stabilises.
- **Why.** Owner ruling, 2026-07-19: "TS-ядро, гейт позже." Balances the standing
  "production-grade, not a sketch" directive against front-loading full toolchain
  scaffolding onto an app that is today plain JS/CJS.
- **Considered and rejected.** Full AI-Native TS floor immediately (heavier than
  the milestone warrants); plain JS placeholder (contradicts enabling the TS
  discipline).
- **When to revisit.** When milestone 1 lands floor-green and the cell boundaries
  have settled — then wire the remaining gate steps.

### D2 — Shell is the default visible `vibe term` {#d2-shell-default}

- **Decision.** The multi-tab shell becomes the visible experience of `vibe term`
  / `vibeterm.exe`; the headless/`--control` path stays bare single-view (§8).
- **Why.** Owner ruling, 2026-07-19: "Шелл = дефолт vibe term." It is the product
  direction; a separate opt-in mode would fork the visible surface for no gain.
  The observation apparatus (PROP-042) is protected by keeping its path bare.
- **Considered and rejected.** Opt-in shell flag with today's single-window
  `vibe term` as default — forks the surface and delays the intended experience.
- **When to revisit.** If the shell cannot reach parity (startup latency, resource
  use) with the bare terminal for the plain single-terminal case.

### D3 — Split ceiling of two panes in milestone 1 {#d3-split-two}

- **Decision.** A window's content area holds at most two panes in milestone 1 (§4).
- **Why.** The reference "open in split view" shows two panes; two proves the
  pane-layout + close-affordance machinery without an N-pane tiling model.
- **Considered and rejected.** Arbitrary N-pane tiling now — layout / focus /
  resize complexity beyond what milestone 1 needs to validate.
- **When to revisit.** When users ask for 3+ panes, or a tiling model becomes a
  named goal of a later milestone.

### D4 — UI stack: Solid + Vite + Tailwind v4 + Kobalte + strict TS {#d4-ui-stack}

- **Decision.** The chrome is authored in **SolidJS** with **Vite**, **Tailwind
  CSS v4**, **Kobalte** (headless accessible primitives), and **strict TypeScript**
  ratcheting toward the AI-Native TS floor. Terminal-view pages stay lean vanilla
  TS + xterm.js (§9).
- **Why.** Proven on the owner's near-identical UI in the `foton` project
  (owner-recommended, 2026-07-19); Solid's fine-grained reactivity suits an
  always-on tab/list chrome; small runtime; Kobalte covers exactly what the shell
  needs (accessible context menu, dialogs, popovers); framework-agnostic w.r.t. the
  `WebContentsView`/xterm engine; satisfies the AI-Native TS discipline (strict
  tsconfig, branded types, Result errors, cells, `vitest` + solid-testing-library,
  `eslint-plugin-solid`).
- **Considered and rejected.** React (largest Electron ecosystem, but no `foton`
  components are reused and xterm is vanilla, so the ecosystem edge is marginal
  here, at a heavier runtime and a from-scratch rewrite); vanilla TS / Lit (the
  growing chrome — dnd tab reorder, command palette, menus, profiles — would become
  a hand-rolled mini-framework). Legacy from `foton` is **not** carried over — only
  the base stack; its Tauri-faked multi-view is replaced by real `WebContentsView`.
- **When to revisit.** If the chrome outgrows Solid's ecosystem, or a larger phase
  makes React's ubiquity (hiring, libraries) decisive.

### D5 — Chrome↔engine protocol: transport-agnostic, sidecar-ready {#d5-sidecar-ready}

- **Decision.** The chrome↔engine command/event protocol (§9) is a versioned,
  serialisable message contract with no Electron types; Electron IPC is one
  transport adapter, not the contract. Milestone 1 uses Electron IPC via a typed
  preload bridge.
- **Why.** Owner intent, 2026-07-19: someday all state may move to an **external
  application**, so the protocol must be designed carefully and ready for
  larger-scale use. Coupling the contract to Electron IPC specifics now would force
  a redesign then.
- **Considered and rejected.** HTTP+SSE to a sidecar now (the `foton` approach) —
  premature; our state (pty) lives in main, there is no external process yet.
  Ad-hoc, untyped Electron IPC with no defined contract — would not port.
- **When to revisit.** When an external state process becomes a real requirement —
  then add a transport adapter (HTTP/WS/stdio) behind the same contract.

### D6 — Internationalisation from the start {#d6-i18n}

- **Decision.** All user-facing chrome strings go through a runtime-switchable
  message catalog; no hardcoded UI copy (§10).
- **Why.** Owner requirement, 2026-07-19: bake i18n in from the start. Retrofitting
  i18n across a grown UI is expensive and error-prone.
- **Considered and rejected.** Defer i18n to a later pass — the retrofit cost and
  the risk of missed strings.
- **When to revisit.** Foundational; revisit only the catalog mechanism if the
  chosen i18n surface proves inadequate.

### D7 — Live theming; two launch themes {#d7-theming}

- **Decision.** Theming via design tokens (CSS custom properties), switchable at
  runtime with no reload; components reference tokens, never hardcoded hex. First
  release ships a dark-purple theme (after the ProjectX reference) and an
  Anthropic-style theme (§11).
- **Why.** Owner requirement, 2026-07-19: users switch visual themes on the fly,
  and two named themes go into the first release. (The `foton` recon showed
  hardcoded hex throughout with no theme system — the anti-pattern this decision
  avoids.)
- **Considered and rejected.** A single hardcoded palette (no live switch, no
  second theme); a theme system deferred past first release (contradicts the
  requirement).
- **When to revisit.** When user-authored themes or a theme editor become a goal.

---

## 12. Sibling contracts — the vibeterm PROP family {#family}

This contract governs the **shell regions, tab model, panes, and tear-off** (the visible surface). The
**AI-UI-ready architecture** the shell is built on lives in the self-contained vibeterm PROP family
alongside it (RP-A/RP-D — [`architecture.md`](architecture.md) is the lore; the findings doc is the
research):

- **[PROP-046](PROP-046-action-aiui-core.md)** — the action/AIUI core: the render-free engine, the
  addressable `action://vibeterm/*` actions, the registry, typed context + pure enablement, `invoke` +
  the **capability/permission surface for an AI peer** (caller-scope, prompt-on-`Dangerous`, audit,
  scope-REFUSE), i18n (en + ru), the four AIUI verbs, and the **identity-grammar conformance** with the
  Rust `vibe-actions`.
- **[PROP-047](PROP-047-modelview-transport.md)** — the serialisable `ModelView` tree (window→tab→pane),
  the MVC reconciliation (engine single-writer, Solid one-way projection), the chrome↔engine transport
  contract (versioned discriminated union, no Electron types, sidecar-ready), and the shell entity set.

The shell decisions D0–D7 above are **frozen axes** (constraints) for that architecture; PROP-046/047
*realise* them and earn the open questions, they do not reopen them. Lore:
[`architecture.md`](architecture.md) + [`design-system.md`](design-system.md).
