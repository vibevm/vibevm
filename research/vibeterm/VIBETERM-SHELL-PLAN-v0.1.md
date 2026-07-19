# Campaign — VibeTerm terminal shell (tabs · panes · windows) v0.1

**Status:** GATED (2026-07-19) — implementation (P1 onward) waits on the
UI-architecture research
[`VIBETERM-UI-ARCHITECTURE-RESEARCH-PLAN-v0.1`](VIBETERM-UI-ARCHITECTURE-RESEARCH-PLAN-v0.1.md)
→ a design-doc → the contracts, per the owner's research → design → execution
cadence. Phase 0 (the reparent spike) stands; P1+ resume once the architecture is
settled. **Opened:** 2026-07-19. **Contract:**
[`spec/modules/vibeterm/PROP-044-terminal-shell.md`](../../spec/modules/vibeterm/PROP-044-terminal-shell.md).
**Floor:** `apps/vibeterm` — `node --test` (existing) + `tsc --noEmit` + `vitest`
(new Solid chrome + engine cells). The Rust floor (`bash tools/self-check.sh`)
applies to any `crates/**` change (e.g. a new `vibe term` flag).

---

## Mandate {#mandate}

Owner, 2026-07-18/19 (paraphrased from the commissioning turn; reference captures
in `refs/screens/projectx/`, out of git — "ProjectX" is the reference prototype,
not a name for our work): a workspace-style VibeTerm — a profiles rail, a
quick-access rail (with `vibetree` under a white wireframe icon), a terminal list
where the sessions live, and the terminals themselves in the content area. A
compact toggle (beside the menu button) hides the profiles rail. **Milestone 1 is
a placeholder shell** — most controls are inert images; only terminal appearance +
switching, **split view**, and **open-in-new-window** work. Right-click a terminal
→ context menu with *Open in new window* and *Open in split view*. "Когда отладим
появление и переключение терминалов, появление их в split view и в новом окне,
можно будет переходить к чему-то более серьёзному."

Owner rulings, 2026-07-19 (recorded as PROP-044 Decisions): **TS-core now, gate
later** (D1); **shell = default visible `vibe term`**, headless untouched (D2);
**UI stack Solid + Vite + Tailwind v4 + Kobalte + strict TS** (D4); **the
chrome↔engine protocol is transport-agnostic and sidecar-ready** — Electron IPC
now, but designed so state can later move to an external app (D5); **i18n from the
start** (D6); **live-switchable theming, two launch themes** — a dark purple
(after the ProjectX reference) and an Anthropic-style (D7).

## Baseline — current state, verified 2026-07-19 {#baseline}

- `apps/vibeterm` is **plain JS/CJS**, no TypeScript toolchain. Entry `main.cjs`
  (Electron main, node-pty here), `renderer.js` (xterm.js), one `index.html`
  (`#term`), pure ESM libs `lib/args.mjs` + `lib/keymap.mjs` (unit-tested via
  `node --test`), `scripts/package.mjs`.
- **One `BrowserWindow`, one pty, one xterm.js.** IPC channels: `pty`
  (main→renderer), `input`, `resize`, `ready` (renderer→main),
  `vibeterm:set-icon`. `nodeIntegration:true, contextIsolation:false` today; the
  new chrome moves to `contextIsolation:true` + a typed preload bridge (PROP-044
  §9).
- `--control` stands up a per-process loopback HTTP server + `@xterm/headless`
  mirror + discovery files + optional CDP (PROP-042). **Frozen** by this campaign
  (PROP-044 §8).
- Deps: `electron ^32.0.0` (runs 32.3.3 here), `@xterm/xterm ^5.5.0`,
  `@xterm/addon-fit`, `@xterm/headless`, `node-pty ^1.1.0`.
- **Chosen UI stack (D4), to be added:** `solid-js`, `vite` + `vite-plugin-solid`,
  `tailwindcss` v4 + `@tailwindcss/vite`, `@kobalte/core`, `typescript`, `vitest`
  + solid-testing-library. Modelled on the owner's `foton` desktop package; only
  the base stack is reused, none of its Tauri-specific multi-view legacy.

## Phase 0 — spike (done, no commits) {#phase-0}

**Question:** can a live xterm.js tab be moved between windows without reload /
state loss? **Result: PASS**, on Electron 32.3.3 / Chrome 128:

```
reparentErr=null   loadCount=1   webContents.id 3→3   sessionStable=true
tick 38→75 (timers alive)   xterm buffer "projectx-spike line-one" identical
corePASS=true   xtermPASS=true
```

→ `WebContentsView` reparent (`removeChildView`→`addChildView`) preserves the same
`WebContents` (no reload) and its live xterm.js. Foundation of PROP-044 §3/§5 and
Decision [#d0](../../spec/modules/vibeterm/PROP-044-terminal-shell.md#d0-reparent). Spike
kept in the session scratchpad (throwaway).

## Decisions {#decisions}

The binding records live at the contract:
[PROP-044 #decisions](../../spec/modules/vibeterm/PROP-044-terminal-shell.md#decisions) —
D0 reparent-preserves-state (verified), D1 TS-core-now, D2 shell-default, D3
split-ceiling-2, D4 UI-stack (Solid/Vite/Tailwind v4/Kobalte/strict TS), D5
transport-agnostic-sidecar-ready-protocol, D6 i18n-from-the-start, D7
live-theming-two-launch-themes. This plan executes them; it does not restate them.

## Phases {#phases}

Each phase ends **floor-green** (`node --test` + `tsc --noEmit` + `vitest` under
`apps/vibeterm`) and writes its ledger row (§execution-ledger). A phase boundary
is a safe cold-stop.

### P1 — stack + engine + chrome skeleton + switching {#p1}
- **Stack bootstrap:** add the D4 toolchain to `apps/vibeterm` (Vite + Solid +
  Tailwind v4 + Kobalte + strict `tsconfig` + `vitest`), building the chrome as a
  Vite bundle Electron loads; keep the existing `node --test` libs green.
- **Foundations laid in from the start (D6/D7):** the **theming token system**
  (CSS custom properties, runtime live switch) with **both launch theme token
  sets** (dark-purple + Anthropic-style), and the **i18n message-catalog** surface
  (runtime locale switch, en + ru) — even though most chrome is placeholder.
- **Engine cells (TS):** `TabId` (branded), the tab registry
  (`open/select/close`), the transport-agnostic command/event protocol + its codec
  (D5), pane-layout maths — typed cells with `vitest`. No Electron in these cells.
- **Main wiring:** generalise `main.cjs` to own `tabs: Map<TabId,…>`, one pty +
  one `WebContentsView` per tab, per-`TabId` IPC over the typed preload bridge;
  preserve the existing single-pty visible path as "one tab".
- **Chrome:** the five regions render (placeholder rails), a terminal list, a
  content area with one pane; **create + select + switch** terminals work; the
  compact toggle works; theme + locale switch demonstrably flip live.
- **Gate / prediction:** two terminals switch with **zero reload** and independent
  scrollback; theme + locale switch live with no reload; the headless/`--control`
  path is byte-for-byte unchanged (PROP-042 tests still green).

### P2 — split view {#p2}
- Second pane beside the first; per-pane `setBounds`; the × close affordance;
  unsplit returns to one pane with both tabs surviving; ptys reflow to pane grid.
- **Gate / prediction:** splitting/unsplitting never reloads either view; each
  pane's pty grid matches its pane (no skew).

### P3 — tear-off + context menu {#p3}
- Right-click list → context menu (Kobalte); **Open in new window** reparents the
  tab's view into a fresh `BrowserWindow` (P0 mechanism); **Open in split view**
  routes to P2; window lifecycle (empty window closes; last-tab move closes source).
- **Gate / prediction:** a moved terminal keeps its scrollback + running program
  live across the move (the P0 property, now in-product).

### P4 — milestone close {#p4}
- Manual-test walkthrough (`spec/manual-tests/MT-*` for the shell), README /
  runtime-doc updates, health-audit note, WAL + CONTINUE refresh. Then the
  "something more serious" backlog (profiles restore, per-tab AIUI, theme/locale
  polish for first release) opens as the next campaign.

## Quick-start {#quick-start}

```bash
# run the visible shell (once P1 lands):
cd apps/vibeterm && npm start
# chrome + engine cell checks:
cd apps/vibeterm && npx tsc --noEmit && npx vitest run
# existing lib tests (must stay green every phase):
cd apps/vibeterm && node --test
# re-run the P0 reparent spike (scratchpad):
node apps/vibeterm/node_modules/electron/cli.js <scratch>/projectx-spike/main.js
```

## Whole-campaign acceptance {#acceptance}

Milestone 1 is done when, in one visible shell window: (1) ≥2 terminals appear in
the list and switch with no reload and independent state; (2) *Open in split view*
tiles a second pane with a working × close; (3) *Open in new window* tears a
terminal into its own window with its running program + scrollback intact; (4) the
compact toggle hides/shows the profiles rail; (5) the theme switches live between
the two launch themes and the locale switches live (en/ru), both with no reload;
(6) `node --test` + `tsc --noEmit` + `vitest` are green under `apps/vibeterm`;
(7) the PROP-042 headless/`--control` suite is unchanged and green.

## Deferrals {#deferrals}

- Profiles rail behaviour — restore terminals from saved settings (ties to
  PROP-040/041). Placeholder only in M1.
- Per-tab AIUI: one control surface / headless mirror **per tab** (today's is
  per-process). M1 leaves `--control` single-view (PROP-044 §8).
- The external-state **sidecar transport** — the protocol is designed for it (D5),
  but no sidecar is built in M1.
- Full theme / locale coverage for first release (M1 lays the systems + the two
  theme token sets + en/ru scaffolding; exhaustive coverage is later).
- N-pane tiling (>2), drag-to-reorder tabs, drag-a-torn-tab back into a window.
- Context-menu items beyond the two acting ones (§6).

## Execution ledger {#execution-ledger}

_Phase → commits (hash · subject · confirmed/falsified). Filled as phases land._

- **P0 (spike):** no commits — finding recorded in §phase-0 (PASS).
- **Setup:** the `.gitignore` guard, the PROP-044 contract, and this plan (this
  session's kickoff commits).
