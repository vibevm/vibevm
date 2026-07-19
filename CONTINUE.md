# CONTINUE.md — cold-resume checkpoint (2026-07-19, VIBETERM UI-ARCHITECTURE: research → execution done)

> `spec/WAL.md` is the canonical living state; if this snapshot and the WAL diverge, the WAL wins.

## TL;DR

The **whole vibeterm UI-architecture campaign** ran end to end under a goal-hook and landed on `main`,
floor-green, **ahead of `origin/main` by 6 commits (mirror pending)**:

- **research** — the plan was **sharpened** first (frozen-vs-open framing, identity-grammar conformance,
  6 new RQs, AI-Native-ready output), then the **findings doc** closed: Phase 1 ports/adapts/new + the
  conformance surface + the AI-UI eval matrix; Phase 2/3/4 comparative + pitfalls→obligations + 16 numbered
  architecture deltas D1–D16. `research/vibeterm/vibeterm-ui-architecture-findings-v0.1.md`.
- **design** — the vibeterm-owned design-doc: `spec/modules/vibeterm/architecture.md` (entities, MVC, AI-UI,
  transport, conformance) + `design-system.md` (the GUI twin of `tui-visual-language.md`).
- **contracts** — the vibeterm PROP family: **PROP-046** (action/AIUI core + identity-grammar conformance)
  + **PROP-047** (ModelView/MVC + transport + entities) + PROP-044 §12 family cross-note.
- **execution** — a **pre-MVP architectural sketch** of the shell (`apps/vibeterm`): a **render-free TS
  engine** (`#no-render-dep`; address/action/registry/context/i18n/modelview/protocol/tabs/aiui cells, 15
  vitest cases), an **Electron main shell path** (`Map<TabId,{pty,WebContentsView}>` + Solid chrome window
  + typed preload bridge, `contextIsolation:true`), a **Solid chrome** (contacts-style TabList + design
  tokens, two launch themes, reactive en/ru i18n), and a **lean vanilla xterm terminal-view**. **Create +
  switch tabs** works over the typed command/event protocol; the engine is the single writer of the
  `ModelView`, the chrome is a one-way projection. `--control`/`--headless` single-view frozen.

**No blocker.** Code complete, build green (engine esbuild bundle + chrome vite bundle), tests green
(41 node-test + 15 vitest + the Rust gate + vibe check). The one thing not verified here is the **GUI
visual pass**: pty spawn in this sandbox hits node-pty's known "AttachConsole failed" without a real
Windows console (environment, not code — vibeterm runs on the owner's desktop, instance 38). The owner
smokes it on a real box: `cd apps/vibeterm && npm run build && npm start`.

## Where work stands

- Branch `main`, **ahead of `origin/main` by 6** (the vibeterm campaign; mirror pending). Working tree: only
  the docs edits for this checkpoint uncommitted (WAL + this file).
- Installed env: `~/opt/vibevm/current` → instance **38** (pre-refactor vibeterm; the new shell needs a
  `vibe self update` cycle to land in an instance). `apps/vibeterm` runs standalone via
  `cd apps/vibeterm && npm install && npm run build && npm start`.
- `bash tools/self-check.sh` — all green (fmt / clippy `--all-targets -D warnings` / vibe check / conform /
  Rust tests / npm / node --test (vibeterm args) / **vitest (vibeterm engine cells)** — the new step).

## What landed this session (self-install launchers)

- `f3df5dd` **feat(vvm): self-install the GUI launchers on every install/update**
- `2c1588b` **docs(vibeframe): mark the self-installing-launchers enhancement done**

New module **`crates/vibe-cli/src/commands/vvm/launchers.rs`** — the `LauncherInstaller`
seam (native impl live; `SkipLauncherInstaller` `#[cfg(test)]` no-op for the gate), mirroring
the `Builder` / `VibetermPackager` / `EnvPersister` seams. Wired into `perform_install`
(`install.rs`) as a **tail on BOTH the new-instance and dedup-skip paths** → idempotent,
self-bootstrapping without `--force`. `mod.rs` declares `mod launchers;` and passes
`&NativeLauncherInstaller` in `run_install_cmd`.

Native impl does, best-effort throughout (a locked exe / missing rc.exe / shortcut failure
is a *note*, never an install failure):
1. `cargo build -p vibe-launcher --target-dir <managed build dir>` (same dir + profile as
   the `vibe` build; default profile is **Debug**);
2. **rename-aside** placement into `store.shim_dir()` = `~/opt/bin`: a running exe can't be
   overwritten on Windows but CAN be renamed → move to `.old-<nanos>`, copy the new one,
   then **drop the sidecar immediately if unlocked** (else it waits for the next update's
   sweep, which runs at the start of the next placement);
3. Windows Start-menu `.lnk` via PowerShell `WScript.Shell` into
   `[Environment]::GetFolderPath('Programs')\vibevm\<Label>.lnk` (target = shim-dir exe,
   icon = exe,0, workingdir = shim dir). Non-Windows = no-op (exe still placed).

Spec: **PROP-043 #self-install** (new section + VibeFrame added to the registry table +
two `#never` bullets); tests tagged `#[verifies(...#self-install)]`. PROP-045 §4 and
`research/vibeterm/VIBEFRAME-SPLIT-PLAN-v0.1.md#deferred-launchers` flipped from *deferred*
to *done (Windows)*.

## What landed earlier in the arc (vibeframe split) — commits `85a5420`…`58f8b96`

- `daedb6e` copy `apps/vibeterm/` → `apps/vibeframe/` (sed-renamed identifiers, `VIBEFRAME=1`).
- `0723a3c` + `278d6ef` route `vibe tree -t` / `vibe aiui` → vibeframe, with a **fallback to
  vibeterm when the target app is unpackaged** (so an installed launcher never hard-fails).
- `6e5aa9e` `vibe frame`; `908debe` `VibeFrame.exe` launcher; `c572c3c` no-dots icon
  (`assets/icons/vibeframe.*` from `ideas-icons/vibeterm/vibeterm-1-nodots`).
- `25ca6fc` package **both** `apps/vibeterm` → `vibeterm/` and `apps/vibeframe` → `vibeframe/`
  into each instance (packager + dist-walker parameterised by app name).
- `b17328f` accept **`VIBEFRAME`** (alongside `VIBETERM`) in the in-place-upgrade detection.
- `85a5420` PROP-044 (vibeterm = complex shell contract); `58f8b96` PROP-045 (vibeframe = simple
  frame contract).

The **terminal-app resolver** is `crates/vibe-cli/src/commands/term.rs`, parameterised by app
name: `resolve_app(app)` (env override `VIBEVM_<APP>` → dev `apps/<app>` → packaged `<home>/<app>`),
with `resolve_app` falling back to `"vibeterm"` when `app != "vibeterm"` and unresolved.

## Candidate next steps (a RESUME is report-then-wait — do NOT auto-start)

1. **The VibeTerm complex shell, milestone-1** — the Slack-like multi-column layout the owner
   asked for (codename **ProjectX** — a substitution token; see the discipline note below).
   **GATED**: research → design → contracts first, per `research/vibeterm/`:
   `VIBETERM-UI-ARCHITECTURE-RESEARCH-PLAN-v0.1.md` (AI-UI-ready: port the vibe-actions
   Search-Everywhere/action-system methodology + the visual language; headless AIUI is the
   reference surface) → then `VIBETERM-SHELL-PLAN-v0.1.md`. Self-contained handoff:
   `research/vibeterm/task.md`. Stack decided: **Solid + Vite + Tailwind v4 + Kobalte + strict
   TS**, Electron IPC transport, i18n + two themes from day one.
2. **Mac/Linux launcher shortcuts** (`.desktop` / `.app`) — the only deferred part of
   self-install; exe placement is already cross-platform. Owner will test Mac/Linux separately.
3. Minor: the last PROP-042 §5 routing cross-note is implied by PROP-045 §2 — trim if a reader
   wants it explicit.

## Non-obvious findings (do not re-learn)

- **`vibe self update` bootstrap**: it runs the *currently-installed* binary's pipeline code, so
  a change to the install pipeline (packaging, launcher-refresh) takes effect only on the
  **SECOND** update — the first builds the new binary into a new instance; the second runs it.
  Verified 35→36 (feature active) and 37→38 (the sidecar-cleanup polish active).
- **Launcher refresh runs on BOTH `perform_install` paths** (new-instance + dedup-skip) — that
  is what makes it idempotent and self-bootstrap without `--force`. A no-op update still
  re-ensures the launchers.
- **Windows `std::fs::copy` propagates the SOURCE mtime** (CopyFileExW) → a placed launcher's
  timestamp is its *build* time, not the copy time. Cosmetic; bytes are correct.
- **Rename-aside** is the Windows-safe replace: overwrite/delete of a running exe fails, but
  *rename* succeeds. Sidecar dropped immediately when unlocked; swept next update when locked.
- **`SkipLauncherInstaller` must be `#[cfg(test)]`** (struct + impl) or clippy `-D dead-code`
  fails the non-test build — exactly how `SkipPackager` is gated.
- **Install does NOT write the `vibe`/`vibe.cmd` shims** — only `vibe self use` + `self doctor`
  do. Launchers, like the shim, resolve the active instance via `~/opt/vibevm/current`, so they
  self-maintain across updates once placed.
- `store.shim_dir()` = `<root>/bin` = `~/opt/bin`; `store.build_dir()` = `~/opt/vibevm/build`
  (the managed `--target-dir`, never the source `target/`).
- `grep -c` exits 1 on zero matches → it breaks `&&` chains (bit me checking sidecar count).

## Repository map (top level)

- `crates/` — Rust workspace. `vibe-cli` (the `vibe` CLI: `commands/vvm/` = the version manager
  / install pipeline; `commands/term.rs` = terminal-app resolver; `commands/tree/` = the tree
  TUI + host integration; `commands/aiui/` = AIUI control). `vibe-launcher` (GUI launcher bins
  vibetree/vibeterm/vibeframe + shared core + per-bin icon embed in `build.rs`). Plus
  `specmark`/specmap tooling, `vibe-actions`, etc.
- `apps/` — Electron apps: `vibeterm/` (the complex terminal, evolving) and `vibeframe/` (the
  simple frame, VibeTree's host — a copy of vibeterm, kept minimal).
- `spec/` — PROP/FEAT contracts + `WAL.md` + `boot/`. Relevant here: `modules/vibe-launcher/PROP-043`
  (launchers, now incl. `#self-install`), `modules/vibeterm/PROP-044` (complex shell),
  `modules/vibeframe/PROP-045` (simple frame), `common/PROP-019` (install pipeline).
- `research/vibeterm/` — the VibeTerm UI-architecture research + shell-campaign plans + the
  split plan + `task.md` handoff.
- `assets/icons/` — launcher icons (`vibetree/vibeterm/vibeframe.{svg,png,ico}`).
- `packages/org.vibevm.*/` — the AI-Native discipline stacks (Rust/TS/Go), fractality,
  delegation-rules, wal-specspaces. `refs/` — third-party references (gitignored screenshots).

## Architectural decisions in force

- **vibeframe = simple / vibeterm = complex**, siblings from one copy; they never share renderer
  code beyond the initial copy (PROP-044/045). vibeframe hosts `vibe tree`; vibeterm is the
  standalone evolving workspace.
- **AI-UI-ready from the start**: any terminal function must be drivable by an AI over a
  *semantic* `invoke` API (not CDP), exactly as by a human — every architecture decision accounts
  for the future headless AIUI surface (port the Vibe Tree action-system + visual-language
  methodology; it was designed universal).
- **Research → design → execution** as separate phases for the big shell architecture; don't
  one-shot. Read prior art in full.
- **Install pipeline is self-contained**: `vibe self update` builds vibe, packages both terminal
  apps into the instance, and now self-installs the launchers + shortcuts (PROP-043 #self-install
  × PROP-019).

## Discipline constraints (hard, from CLAUDE.md + user memory)

- **NEVER write "Slack" anywhere** in repo / git history / chat. "ProjectX" is a substitution
  token for the reference app; the equivalence lives ONLY in out-of-git user memory. Our feature
  is the **VibeTerm shell**, not "ProjectX".
- Commits **only** via heredoc `git commit -F - <<'MSG'` (never `-m` with backticks — command
  substitution has corrupted messages twice). **No AI attribution** anywhere (Rule 1).
  Conventional Commits, atomic (Rule 3).
- Edits **only** via Edit/Write (PS5.1 UTF-8-no-BOM round-trip corrupts non-ASCII); revert via
  `git restore`. Self-check via **Git Bash** (`bash tools/self-check.sh`) for the real exit code;
  commit only when green (fmt is its fail-fast first gate — run `cargo fmt --all` after edits).
- Push via **`cargo xtask mirror`** (fans out to both GitVerse + GitHub). Never blanket
  `Stop-Process` by name. Never suggest `/code-review`.
- Rule-4 non-routine ops (history rewrite of *pushed* commits, force-push, large blobs,
  CI/secrets) stop for the owner. Delegation (fractality/GLM) is the default for cheap execution,
  but high-error-cost / high-context work like the install pipeline stays with the boss.

## Recent commit chain (last 26, newest first)

```
43f6716 feat(vibeterm): D4 pre-MVP shell -- render-free engine + Solid chrome + per-tab terminals
cb15828 docs(vibeterm): D3 contracts -- the vibeterm PROP family (PROP-046/047 + PROP-044 §12)
2932349 docs(vibeterm): D2 design-doc -- architecture + design system lore
a6e22fc docs(research): vibeterm research close -- Phase 2/3/4 (deltas)
3bd277e docs(research): vibeterm Phase 1 -- internal methodology extraction
6ff6a2a docs(research): sharpen the vibeterm UI-architecture plan before Phase 1
731e7f1 docs(wal): session checkpoint — vibeframe split + self-install launchers closed
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
daedb6e feat(vibeframe): copy vibeterm as the simple terminal frame
2a19e63 docs(research): plan the vibeframe split — VibeTree's simple terminal host
146b007 docs(research): add the self-contained VibeTerm task handoff (task.md)
edf7bc7 docs(research): relocate vibeterm plans into a self-contained research/vibeterm/
d6d61af docs(research): resolve RP-A/RP-D — vibeterm as a self-contained system
4cdb2f7 docs(campaign): gate the VibeTerm shell build behind the UI-architecture research
d65d086 docs(research): open the VibeTerm UI-architecture research plan
8dd21bf docs(campaign): open the VibeTerm shell campaign plan
85a5420 docs(vibeterm): add terminal-shell contract PROP-044
f263b22 chore(gitignore): never track ProjectX reference captures
c3b7b09 feat(icons): vibeterm back to the maximized (enlarged) glyph
09e2391 docs(ideas-icons): archive the enlarged icon attempts
5bc7332 feat(icons): original glyphs, uniformly scaled to full-bleed
90f5a2b feat(icons): full-bleed tile — kill the edge margin, fill like neighbours
129e01b feat(icons): enlarge vibeterm + vibetree to fill the taskbar tile
```

## Quick-start

```bash
# floor gate (real exit code — always via Git Bash)
bash tools/self-check.sh

# rebuild + reinstall vibe from the working tree (run TWICE for a pipeline-code change)
vibe self update

# the terminals
vibe frame          # simple frame (vibeframe)   — VibeTree's host
vibe term           # complex terminal (vibeterm)
vibe tree -t        # tree TUI, hosted in vibeframe

# push both mirrors
cargo xtask mirror

# inspect the installed env
cat ~/opt/vibevm/current
ls ~/opt/bin/ | grep -Ei 'vibe(tree|term|frame)'
ls "$APPDATA/Microsoft/Windows/Start Menu/Programs/vibevm/"
```

> The WAL (`spec/WAL.md`) is the canonical living state and supersedes this snapshot if they diverge.
