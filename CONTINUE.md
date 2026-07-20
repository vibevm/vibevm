# CONTINUE.md — cold-resume checkpoint (2026-07-20, VIBETERM UI-ARCHITECTURE: research → execution + M1 split/tear-off + ProjectX redesign)

> `spec/WAL.md` is the canonical living state; if this snapshot and the WAL diverge, the WAL wins.

## TL;DR

The **vibeterm UI-architecture campaign ran end to end** this session-arc and is **on `main`, floor-green,
pushed to both mirrors** (GitVerse + GitHub), working tree clean. Cadence: **research → design → contracts
→ execution**, then the **M1 build-out** (split + tear-off) and a **chrome redesign to match the ProjectX
reference** (4-column layout). The pre-MVP shell creates/switches/splits/tears-off terminals over a
render-free TS engine, AI-UI-ready by construction, verifiable offscreen via CDP.

**No blocker.** The one open item is **pixel-polish against the reference captures** (exact spacing,
avatar art, copy) and **wiring the placeholder controls** (rename / background-tab / close-others /
search). The architecture and the working surface are done.

## Where work stands

- Branch `main`, **synced with `origin/main`** (both mirrors at `0976b03`). Working tree clean.
- `apps/vibeterm`: TypeScript + Solid + Vite + Tailwind v4 + Kobalte + vitest. `npm run build` (engine
  esbuild bundle + chrome vite bundle) green; `npm test` green (41 `node --test` lib + 17 `vitest`
  engine); `bash tools/self-check.sh` green (Rust gate + vibe check + conform + both app-test steps).
- The shell **runs offscreen** (`electron . --headless --cdp-port N`, spawn via `Stdio::null` — see
  non-obvious findings) and is **driven + screenshot via CDP** (`scripts/cdp-smoke.mjs`,
  `scripts/cdp-split-smoke.mjs`). `analyze_image` on the screenshots confirms the 4-column layout and
  that the sidebar drops in split.

## What landed this session-arc (vibeterm UI-architecture campaign)

research → design → contracts → execution → M1 build-out → redesign:

- `6ff6a2a` **sharpen the research plan** (frozen-vs-open framing, identity-grammar conformance, 6 new
  RQs, AI-Native-ready output).
- `3bd277e` Phase 1 findings — ports/adapts/new + conformance surface + AI-UI eval matrix.
- `a6e22fc` research close — Phase 2/3/4 (comparative + pitfalls→obligations + **16 deltas D1–D16**).
- `2932349` D2 design-doc — `spec/modules/vibeterm/architecture.md` + `design-system.md`.
- `cb15828` D3 contracts — **PROP-046** (action/AIUI core + conformance) + **PROP-047** (ModelView/MVC +
  transport) + PROP-044 §12 family cross-note.
- `43f6716` D4 pre-MVP shell — render-free TS engine + Solid chrome + per-tab `WebContentsView`
  terminals + typed preload bridge.
- `7c297e1` offscreen shell + CDP for headless screenshot/drive.
- `0721ad2` fix initial-openTab + IPC-handler race offscreen.
- `a0cea60` **split view + tear-off (M1 P2/P3)**.
- `0976b03` **chrome visual fidelity to the ProjectX reference** (4-column layout, full 8-item context
  menu, tear-off header).

## The vibeterm pre-MVP shell (apps/vibeterm)

Architecture (PROP-044/046/047 in force):

- **engine** (`src/engine/`, render-free TS, `#no-render-dep` lint/boundary): `address` · `action` ·
  `registry` · `context` · `i18n` · `modelview` (window→tab→pane tree) · `protocol` (versioned
  discriminated union) · `tabs` (pure single-writer cell) · `aiui` (4-verb reference surface). 17 vitest
  cases.
- **main** (`main.cjs`): `shellWindows` Map (multi-window) + `shellTabs` (windowId) + `shellPanes`
  (slot). Layout constants match the chrome grid (PROFILES_RAIL 56 / LIST_WIDTH 240 / SIDEBAR_WIDTH 280
  / CONTENT_HEADER 40 / TEAROFF_HEADER 36); the terminal `WebContentsView`s overlay the content region
  only; the sidebar drops in split; tear-off reparents a view into a chromeless `BrowserWindow` (D0 —
  no reload, xterm buffer intact). `--control`/`--headless` single-view path frozen (PROP-042).
- **chrome** (`src/chrome/`, Solid): `ProfilesRail` · `TabList` (rich rows + user card) · `TabItem` (8-
  item context menu) · `ContentHeader` (theme/locale toggles working) · `RightSidebar` · `App` (4-col
  grid). `bridge.ts` is the one-way ModelView projection + command sender. Tailwind v4 + design tokens
  (Rosé Pine dark-purple + Anthropic-style).
- **terminal-view** (`terminal-view/`): lean vanilla xterm page (per-tab, reused by single-view);
  `tearoff.html` = chromeless header host.

Working surface: **open / select / close / pane.split / pane.close / tab.move-to-window / theme /
locale**. Placeholders: rename, background-tab, close-others/all, sidebar search + items, profiles-rail
icons, content-header utility icons.

## Non-obvious findings (do not re-learn)

- **pty spawn needs `Stdio::null`.** A bare `electron .` launched from Git Bash trips node-pty's ConPTY
  agent (`AttachConsole failed`) because electron inherits the shell's console. `vibe-cli`'s
  `spawn_vibeterm` uses `Stdio::null()` and pty works; for a direct smoke, spawn electron with
  `</dev/null >/dev/null 2>&1 &` (the cdp-smoke scripts do this). Visible `vibe term` is unaffected.
- **CDP screenshot is the GUI verify.** `--headless` + `--cdp-port` render the chrome offscreen;
  `Page.captureScreenshot` returns a faithful PNG; `Runtime.evaluate` drives `window.vibeterm.*`. The
  terminal `WebContentsView`s are native surfaces over the chrome renderer, so a chrome screenshot
  shows the rail/list/sidebar (not the terminals) — drive + assert through the ModelView (`state()`) for
  terminal/pane truth, screenshot for chrome layout.
- **IPC handlers register BEFORE `loadFile`.** `await loadFile` resolves once HTML is parsed; the chrome
  module can then fire `state()` before a handler registered after the await exists (`No handler
  registered`). And **initial openTab starts immediately after loadFile** — a `did-finish-load`/`isLoading`
  gate races and never settles offscreen.
- **ESM/CJS**: the package is `type: module`; `main.cjs`/`preload.cjs` are CommonJS; the engine is an
  **esbuild ESM bundle** (tsc emits extension-less imports Node ESM rejects) imported by main via
  dynamic `import('./dist/engine/index.js')`; chrome is a vite browser bundle; types come from source
  via the `@vibeterm/engine` path alias.
- **Engine build is no-DOM** (`tsconfig.engine.json` lib ES2022, no DOM); the typecheck-all `tsconfig.json`
  adds DOM for the chrome. The `#no-render-dep` invariant holds on the engine build.
- **Kobalte** `ContextMenu` is itself the Root (no `.Root`); `ContextMenu.Item`/`.Separator`/`.Portal`.
- **Layout constants are duplicated** between `main.cjs` (terminal view bounds) and `theme.css` (chrome
  grid) — keep them in sync (PROFILES_RAIL 56 / LIST_WIDTH 240 / SIDEBAR_WIDTH 280 / CONTENT_HEADER 40
  / TEAROFF_HEADER 36).

## Discipline constraints (hard, from CLAUDE.md + user memory)

- **NEVER write the reference app's real name** in repo / git / chat. **"ProjectX"** is a substitution
  token; the equivalence lives only in out-of-git user memory. Our feature is the **VibeTerm shell**.
- Commits **only** via heredoc `git commit -F -`; **no AI attribution** (Rule 1). Conventional Commits,
  atomic (Rule 3).
- Edits **only** via Edit/Write (PS5.1 UTF-8-no-BOM round-trip corrupts non-ASCII); self-check via **Git
  Bash** (`bash tools/self-check.sh`), real exit code.
- Push via **`cargo xtask mirror`** (GitVerse + GitHub, fast-forward-only). Never blanket
  `Stop-Process` by name. Never suggest `/code-review`.
- Rule-4 red lines (history rewrite of pushed, force-push, large blobs, CI/secrets) stop for the owner.

## Candidate next steps (a RESUME is report-then-wait — do NOT auto-start)

1. **Pixel-polish vs the reference captures** (`refs/screens/projectx/`, out of git): exact spacing,
   avatar art, the sidebar/row copy, the user-card. Layout shape is confirmed; this is visual tuning.
2. **Wire the placeholder controls**: rename (an `action://vibeterm/tab.rename` + prompt), background-tab
  open, close-others/close-all, sidebar search (a Search Everywhere provider), profiles-rail selection.
3. **Identity-grammar conformance golden** (PROP-046 §9 / findings §3): the Rust `vibe-actions` ↔ TS
  `vibeterm-core` CI golden — the load-bearing anti-drift mechanism, currently architected-not-built.
4. **M1 close (P4)**: manual-test walkthrough (`spec/manual-tests/MT-*` for the shell), README/runtime-docs,
  health-audit, then the "something more serious" backlog (profiles restore per PROP-040/041, per-tab
  AIUI over the wire).

## Repository map (vibeterm-focused)

- `apps/vibeterm/` — the shell. `src/engine/` (render-free TS cells), `src/chrome/` (Solid), `main.cjs`
  (Electron main + shell state), `preload.cjs` (typed bridge), `terminal-view/` (lean xterm + tearoff),
  `lib/` (args/keymap, `node --test`), `scripts/` (cdp-smoke), `tsconfig*.json`, `vite.config.ts`,
  `vitest.config.ts`, `package.json`. `dist/` gitignored.
- `spec/modules/vibeterm/` — **PROP-044** (shell regions/tabs/panes/windows) · **PROP-046** (action/AIUI
  core + conformance) · **PROP-047** (ModelView/MVC + transport) · `architecture.md` + `design-system.md`
  (lore).
- `research/vibeterm/` — `task.md` (cold-start index) · `VIBETERM-UI-ARCHITECTURE-RESEARCH-PLAN-v0.1.md`
  · `vibeterm-ui-architecture-findings-v0.1.md` (the closed research) · `VIBETERM-SHELL-PLAN-v0.1.md`
  (build campaign, P1 done / P2-P3 done / P4 close pending) · `VIBEFRAME-SPLIT-PLAN-v0.1.md`.
- `crates/vibe-cli/` — the `vibe` CLI; `commands/term.rs` (`spawn_vibeterm` — the `Stdio::null` pty
  fix), `commands/aiui/` (the three-plane `vibe aiui`).
- `refs/screens/projectx/` — reference captures (out of git). `assets/icons/vibeterm.*` — launcher icons.

## Architectural decisions in force

- **vibeframe = simple / vibeterm = complex** (PROP-044/045); vibeframe hosts `vibe tree`, vibeterm is
  the standalone evolving workspace.
- **AI-UI-Ready by construction** (owner): control is semantic `invoke`; CDP is observation-only.
- **Self-contained & detachable** under `spec/modules/vibeterm/` + `apps/vibeterm/`, no build-dep on
  vibevm-internal; **identity-grammar conformance** (shared grammar + CI golden) is the one tie.
- **Stack**: Solid + Vite + Tailwind v4 + Kobalte + strict TS; terminal views lean vanilla xterm.
  Tabs = per-tab `WebContentsView` + main-owned pty (reparent preserves state, D0 verified).
- **Engine is the single writer of the ModelView**; the Solid chrome is a one-way projection rebuilt on
  re-resolution; ephemeral chrome state never crosses the seam.
- **Transport-agnostic chrome↔engine protocol** (versioned discriminated union, no Electron types,
  sidecar-ready); Electron IPC via a typed preload bridge is one adapter.
- **i18n from the start** (en + ru, address-keyed, reactive live swap, legibility gate); **live theming**
  via design tokens (two launch themes).

## Recent commit chain (last 25, newest first)

```
0976b03 feat(vibeterm): chrome visual fidelity to the ProjectX reference
a0cea60 feat(vibeterm): split view + tear-off (M1 P2/P3)
0721ad2 fix(vibeterm): initial openTab + IPC-handler race in the offscreen shell
7c297e1 feat(vibeterm): offscreen shell + CDP for headless screenshot/drive
4c7fb4c docs(continue): cold-resume checkpoint -- vibeterm UI-arch campaign
cb32901 docs(wal): session checkpoint -- vibeterm UI-arch campaign (research -> execution) done
43f6716 feat(vibeterm): D4 pre-MVP shell -- render-free engine + Solid chrome + per-tab terminals
cb15828 docs(vibeterm): D3 contracts -- the vibeterm PROP family
2932349 docs(vibeterm): D2 design-doc -- architecture + design system lore
a6e22fc docs(research): vibeterm research close -- Phase 2/3/4 (deltas)
3bd277e docs(research): vibeterm Phase 1 -- internal methodology extraction
6ff6a2a docs(research): sharpen the vibeterm UI-architecture plan before Phase 1
731e7f1 docs(wal): session checkpoint — vibeframe split + self-install launchers closed
6dc9f10 docs(continue): cold-resume checkpoint — vibeframe split + self-install launchers
2c1588b docs(vibeframe): mark the self-installing-launchers enhancement done
f3df5dd feat(vvm): self-install the GUI launchers on every install/update
58f8b96 docs(vibeframe): add the simple-terminal-frame contract PROP-045
b17328f feat: accept VIBEFRAME in the in-place-upgrade detection
2acd358 docs(research): note the self-installing-launchers enhancement
25ca6fc feat(vibe-cli): package vibeframe into installed instances
c572c3c feat(vibeframe): give VibeFrame its own no-dots icon
908debe feat(vibe-launcher): add the VibeFrame.exe launcher
6e5aa9e feat(vibe-cli): add the vibe frame command
278d6ef fix(vibe-cli): fall back to vibeterm when the target app is unpackaged
0723a3c feat(vibe-cli): route vibe tree -t and vibe aiui to vibeframe
```

## Quick-start

```bash
# boot: CLAUDE.md → spec/boot/ → spec/WAL.md → CONTINUE.md (this file)
#       → research/vibeterm/task.md + the findings doc + PROP-044/046/047
# floor gate (real exit code — always via Git Bash)
bash tools/self-check.sh
# rebuild + reinstall vibe from the working tree (run TWICE for a pipeline-code change)
vibe self update
# the terminals
vibe frame          # simple frame (vibeframe)   — VibeTree's host
vibe term           # the complex shell (vibeterm) — 4-column, split/tear-off
vibe tree -t        # tree TUI, hosted in vibeframe
# run the shell from the working tree (dev)
cd apps/vibeterm && npm install && npm run build && npm start
# offscreen smoke (CDP screenshot + drive)
cd apps/vibeterm && MSYS_NO_PATHCONV=1 node_modules/.bin/electron . --headless --cdp-port 9222 --exec "cmd.exe" </dev/null >/dev/null 2>&1 &
node scripts/cdp-smoke.mjs 9222 /tmp/vibe          # open/select + screenshots
node scripts/cdp-split-smoke.mjs 9222 /tmp/vibe    # split/tear-off + screenshots
# push both mirrors
cargo xtask mirror
```

> The WAL (`spec/WAL.md`) is the canonical living state and supersedes this snapshot if they diverge.
