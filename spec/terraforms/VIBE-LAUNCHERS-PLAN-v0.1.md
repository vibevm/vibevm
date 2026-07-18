# VIBE-LAUNCHERS — campaign plan v0.1

> Genre: **campaign plan** (non-binding execution recipe). The normative contract
> it produces is **PROP-0XX (launchers)**, authored in Phase 1. `spec/WAL.md` is the
> living state. Format: `spec://org.vibevm.world/campaign-plans/…/CAMPAIGN-PLAN-FORMAT`.

## 1. Mandate (owner, 2026-07-18, verbatim intent)

> «Я хочу сделать отдельный запускатор (VibeTree.exe, кроссплатформенный, Rust),
> который запускает `vibe tree -t`. И в дальнейшем положить его в инсталлятор. В
> дальнейшем там должно быть ещё несколько таких запускаторов, поэтому механизм
> лучше сделать достаточно универсальным.»

Decisions taken in the same conversation (owner-selected):

- **Artifact model:** *N thin binaries + a shared core* now, for **our own** curated
  launchers. A **uv-style trampoline** is a **separate, later system** for **third-party
  packages and prompts** (out of scope here — see Phase 5, deferred).
- **Platform scope now:** **Windows-first**, with a **cross-platform core**. macOS
  `.app` / Linux `.desktop` are later phases.
- **Process:** author this plan (and the PROP) **before** touching the tree — no
  spike-first.
- **Icon:** primary variant is a **dark-purple disc** (Discord-style) with the existing
  vibeterm node-graph figure centered, unchanged; a **black-disc** (Steam-style) variant
  is drawn for future use. Master format SVG; `.ico`/`.png` generated from it.

## 2. Baseline — current state (verified 2026-07-18)

- **No launcher exists.** Workspace `members` (root `Cargo.toml`) are `crates/vibe-*`
  + `xtask`; there is **no** `crates/vibe-launcher`.
- **The target command works:** `vibe tree -t` → `commands::term::launch_vibeterm`
  (shared with `vibe term`, PROP-042 §5) spawns the packaged vibeterm **detached** and
  returns fast with `vibeterm launched (pid …)`. Verified live this session (pid 50300).
- **The installer is the VVM pipeline:** `commands/vvm/{install.rs,placer.rs,store.rs}`
  builds a distribution, diff-copies it into `…/opt/vibevm/versions/<kind>/<id>/<inst>/`,
  and the stable shims live in `…/opt/bin/{vibe,vibe.cmd}` (on PATH). vibeterm is packaged
  **into each instance** (`vibeterm/`) — the same seam a launcher would ride.
- **Self-location is solved:** `commands/vvm/selfloc.rs::derive_self` derives the instance
  home from a binary's own path (`…/opt/vibevm/current` → instance dir). A launcher placed
  in `…/opt/bin` can reuse the same relative walk.
- **Icon source:** `apps/vibeterm/resources/icon.svg` — a terracotta (`#D97757`)
  node-graph on transparent bg (hub + 4 nodes + spokes). The launcher icons derive from it.
- **Tooling on this box:** ImageMagick (`magick`) is present (SVG→`.ico`/`.png` works);
  Rust 1.93, node 24. **Reference:** `refs/src/uv` ships the exact pattern to study —
  `uvw` (`#![windows_subsystem = "windows"]`) and `uv-trampoline` (console/gui stubs).
  Permissive-licensed analog; study, do not copy (clean-room default).

## 3. Design contract (→ PROP-0XX, ratified in Phase 1)

Recorded here as decisions with rejected options; Phase 1 lifts them into the PROP.

- **D1 — N thin GUI binaries + shared core + a declarative registry.**
  `crates/vibe-launcher` = a core lib (resolve/spawn/report) + one ~5-line `bin` per
  launcher, whose target `argv` is compiled in. A single `registry` table (`name → argv →
  icon → subsystem`) is the source of truth; adding a launcher is one entry + one thin bin.
  *Rejected:* multi-call `argv[0]` dispatch (one shared icon — GUI launchers want distinct
  icons); trampoline-now (append-metadata machinery is over-engineered for a small fixed
  set — it is the right tool only for the deferred third-party case).
  *Revisit when:* launchers must be **minted after install** (per-package/per-prompt) →
  build the trampoline system (Phase 5, separate).

- **D2 — GUI subsystem + windowless child.** Each launcher is
  `#![cfg_attr(windows, windows_subsystem = "windows")]` (no console allocation on
  double-click), and it spawns `vibe …` with `CREATE_NO_WINDOW`/`DETACHED_PROCESS` so the
  console-subsystem child never flashes a window either.
  *Rejected:* console subsystem (flash on double-click); a `.lnk`/`.desktop`-only approach
  (cannot suppress the child's console flash, cannot report errors).

- **D3 — resolve `vibe` selfloc-first.** The launcher, installed in `…/opt/bin`, reads
  `…/opt/vibevm/current` relative to itself and execs `<instance>/vibe.exe` (same logic as
  the `vibe` shim / `selfloc.rs`). *Rejected:* PATH lookup as primary (Explorer-inherited
  PATH is stale until re-login). PATH is the fallback.

- **D4 — fail loud, graphically.** A GUI-subsystem process has no visible stderr, so on any
  failure (no `vibe`, spawn error, non-zero exit) the launcher captures the child's output
  and shows a **native dialog** (`MessageBoxW` / `osascript display dialog` / `zenity`) and
  writes a log line. *Rejected:* silent exit (a dead double-click is the worst UX).

- **D5 — Windows-first, cross-platform core.** The core (resolve/spawn/report) is portable;
  the GUI-integration wrapper is per-OS: Windows `.exe` + embedded icon (`winres`/
  `embed-resource`) + installer shortcut; macOS `.app` bundle + Linux `.desktop` are later.

- **D6 — icons (DECIDED).** One visual system: the vibevm node-graph figure (from
  `apps/vibeterm/resources/icon.svg`, geometry unchanged) on a rounded-square gradient tile
  (`#2A2A32`→`#161619`); each app varies only the graph hue. **`default`** = coral `#D97757`
  (the default / library app); **`vibetree`** = muted emerald `#5FB584`. Masters + generated
  `.ico`/`.png` live in **`assets/icons/`** (shared across the app family, not per-crate); the
  rejected explorations + a colour map are archived in `ideas-icons/` (non-build). The `.ico`
  is rebuilt by downsampling a 1024px render (layers 256/128/64/48/32/16) so the **Start-menu
  tile is high quality** (256 is the `.ico` ceiling; a 512px PNG covers larger surfaces).
  *Rejected:* the bright-purple / black discs (too loud, off-system); a per-crate assets home
  (these icons are a shared family, not one launcher's).

- **D7 — home.** New workspace member `crates/vibe-launcher` (core lib + bins + registry);
  the icons are shared and live in `assets/icons/`. Installer changes ride the existing VVM
  pipeline (D-placement like vibeterm).

- **D8 — the window icon matches the launcher (owner requirement).** Launching `vibe tree -t`
  (directly or via `VibeTree.exe`) must open a vibeterm window whose icon is `vibetree`
  (emerald) — the same identity the launcher's own exe / Start-menu icon carries — so the whole
  path reads as one app. Mechanism: vibeterm bundles both family icons and gains a
  `--icon <name>` arg (`default` | `vibetree`) that sets its `BrowserWindow({ icon })`;
  `vibe tree -t` passes `--icon vibetree`; the plain `vibe term` stays `default`. The icons ride
  into the instance with the vibeterm package (Phase 3), so the installed `vibe` can name them.
  *Rejected:* swapping vibeterm's single default icon (would repaint every vibeterm window and
  erase the default/vibetree distinction); leaving the window on the default icon (owner wants
  the launcher and the window it opens to match).

## 4. Phases (each ends floor-green; any boundary is a safe stop)

- **Phase 0 — spikes, no commits.** Prove on Windows: a GUI-subsystem Rust bin double-clicked
  produces **no console window**; it selfloc-resolves `vibe`; it spawns `vibe tree -t`
  windowless and vibeterm opens; a forced failure raises a `MessageBox`. Findings rewrite
  D1–D4 if any spike is red.
- **Phase 1 — contract + core.** Author **PROP-0XX** (lift §3). Land `crates/vibe-launcher`:
  the portable core (`resolve_vibe`, `spawn_detached`, `report_error`), unit-tested; the
  declarative `registry`. Floor green.
- **Phase 2 — VibeTree on Windows.** The thin `vibetree` bin + `build.rs` embedding the
  high-quality `assets/icons/vibetree.ico`. vibeterm learns `--icon <name>`, bundles the family
  icons, and `vibe tree -t` passes `--icon vibetree` (D8). End-to-end: `VibeTree.exe`
  double-click → vibeterm opens `tree -t` with the **emerald** window icon, no console flash,
  a dialog on failure.
- **Phase 3 — installer integration.** `vibe self install`/`update` carries the launcher(s)
  into the instance (extend the vibeterm-packaging seam) and creates a discoverable entry
  point (Start-menu, optional desktop shortcut). `vibe self doctor` reports launcher health.
- **Phase 4 — later — macOS/Linux GUI wrappers.** `.app` bundle + `.icns`; `.desktop` +
  hicolor `.png`. Needs a mac/linux box to verify (deferred by name).
- **Phase 5 — later, separate — trampoline.** The append-metadata stub system for
  third-party packages/prompts. Its own PROP + plan; **not** part of this campaign.

## 5. Predictions (falsifiable — the REPORT checks each)

- **P1** Double-clicking `VibeTree.exe` opens vibeterm with **zero** console windows
  (neither the launcher's nor `vibe`'s).
- **P2** After a fresh `vibe self update`, the active instance carries the launcher and a
  working Start-menu entry (a **crisp 256px tile**) that opens the tree.
- **P3** Adding a second launcher (e.g. `VibeX` → `vibe x …`) is **one registry entry + one
  thin bin file** — no core changes.
- **P4** On a box without a resolvable `vibe`, the launcher shows a dialog naming the fix,
  never a silent no-op.
- **P5** The window opened by `vibe tree -t` (and by `VibeTree.exe`) shows the **vibetree**
  (emerald) icon — matching the launcher — while a plain `vibe term` window shows `default`.

## 6. Quick-start (cold reader)

```sh
git -C C:/Users/olegc/gits/vibevm status -sb
# Phase 0 spike lives in a scratch crate; Phase 1+ lands in:
#   crates/vibe-launcher/   core lib + thin bins + registry
#   assets/icons/           the shared family icons (default coral, vibetree emerald) — DONE
#   ideas-icons/            archived explorations + colour map (non-build) — DONE
# regenerate the high-quality .ico from a master (downsample a 1024px render):
magick -background none -density 768 assets/icons/vibetree.svg -resize 1024x1024 /tmp/vt.png
magick /tmp/vt.png -define icon:auto-resize=256,128,64,48,32,16 assets/icons/vibetree.ico
```

## 7. Acceptance (whole-campaign, run at REPORT)

1. `cargo build -p vibe-launcher` is green; `bash tools/self-check.sh` is green.
2. `VibeTree.exe` (from an installed instance) opens vibeterm on the project tree with no
   console flash; a broken `vibe` yields a dialog, not silence.
3. `crates/vibe-launcher/registry` has a documented one-entry recipe for the next launcher.
4. PROP-0XX exists and every `#[spec(implements=…)]` in `vibe-launcher` cites a live anchor.

## 8. Execution ledger (LOG) — append per phase

- _2026-07-18:_ plan drafted. **Icons DONE** (ahead of Phase 0, they are assets not code):
  the family pair `default` (coral) + `vibetree` (emerald) on the shared gradient tile
  shipped to `assets/icons/` (SVG master + high-quality `.ico` rebuilt from a 1024px render
  + 256/512 PNG); explorations + a colour map archived in `ideas-icons/`. Owner requirements
  folded in: **D8** (the window icon matches the launcher) and the high-quality Start-menu
  tile (**D6**). Awaiting owner go before Phase 0.
- _2026-07-18 (execution — owner: «запусти всё»):_ **Phases 0–2 + D8 landed, floor green,
  pushed.** Phase 0 spike proved the mechanism AND surfaced a real bug — `vibe tree -t`
  ignored an explicit `-t` in a non-tty (the GUI-launcher case), fixed in `5765da0`.
  Phase 1: PROP-043 (`905199d`). Phase 2: `crates/vibe-launcher` + the `vibetree` bin,
  conform-green, VibeTree.exe verified opening vibeterm, no console flash (`9742d4d`).
  **D8** (`965e479`): the tree window carries the `vibetree` icon end to end — verified the
  packaged vibeterm receives `--icon vibetree` and ships `icon-vibetree.ico`. Phase 3
  (install) done **manually**: the release `VibeTree.exe` placed in `~/opt/bin` (on PATH,
  selfloc-resolves), a Start-menu `VibeTree.lnk` created (its icon = the embedded hi-q
  green `.ico`). **Deferred by name:** folding the launcher build + placement + shortcut
  into the VVM `self install`/`update` pipeline (the automated Phase 3).
- _2026-07-18 (second launcher — owner: pick a vibeterm icon + a VibeTerm launcher):_
  **VibeTerm landed, floor green.** The vibeterm app icon was redesigned in a new
  `ideas-icons/vibeterm/` sub-batch (a terminal-prompt motif, two generators computing the
  trails/rain/sparkle geometry — see its README); the owner picked
  `vibeterm-c2-coralstars-sparkle`. Shipped as the **official** icon:
  `assets/icons/vibeterm.{svg,ico,png,-512.png}` + the vibeterm **window default**
  `apps/vibeterm/resources/icon.{svg,ico,png}` (so plain `vibe term` shows it — no `--icon`
  needed). Second launcher: the `vibeterm` bin (`vibe term`), and `build.rs` switched from
  crate-wide `winres` to **per-binary** embedding (`embed-resource::compile_for` →
  `rustc-link-arg-bin`) so `vibetree.exe`/`vibeterm.exe` carry different icons from one
  crate (the `LAUNCHERS` registry table). Verified: both release exes extract their distinct
  icons (green graph / coral prompt). PROP-043 #registry + #icon updated. Pushed to both
  mirrors (`02781dc`).
- _2026-07-19 (deploy — owner: install both + lowercase exe names):_ **Deployed on the
  owner's machine.** `vibe self update --force` rebuilt from the local tree to **instance 29**
  (active), whose packaged vibeterm now ships `resources/app/resources/icon.ico` == c2
  (byte-identical) — so the `vibe term` window shows c2 with no `--icon`. Launchers installed
  to `~/opt/bin` with **lowercase** names (`vibetree.exe`, `vibeterm.exe`) for easy terminal
  typing (the old capitalised `VibeTree.exe` removed; case-insensitive FS ⇒ delete-before-copy
  so the entry is truly lowercase); embedded icons verified on the installed exes (green /
  coral). Start-menu shortcuts recreated: display names **VibeTree** / **VibeTerm** (capitalised)
  → the lowercase exes, `IconLocation` = each exe (its embedded icon), cwd = repo (tree) / home
  (term). End-to-end confirmed by composition: the resolve→spawn path is the proven VibeTree
  core; every link (current→29, packaged icon==c2, exe icons, shortcut targets) checked.
- _2026-07-19 (fix — owner: VibeTree crashed when run outside a project):_ **VibeTree now
  works from anywhere.** Root cause: `vibe tree` needs a `vibe.toml` in cwd; a double-click
  from `~/opt/bin` (or any non-project shell) has none, so the child exited 1 and the
  launcher faithfully reported it (the dialog the owner saw). Fixed in `vibe tree`
  (PROP-036 §2.13 #project-resolution), **human surfaces only** (`--json` stays strict):
  resolve **cwd → remembered last-project → native folder picker (`-t` only)**; every
  successful open records `vibe.tree.last-project` (an L1 setting) so a later context-free
  launch reopens it; a cancelled picker is a clean no-op (no error dialog). New dep `rfd`
  (native chooser, no unsafe in our code). Floor-green (self-check all green, unittests 371);
  redeployed to instance 30 and verified live — from `~/opt/bin` the tree now renders the
  remembered repo instead of crashing.
- _2026-07-19 (feature — owner: run in place inside vibeterm + swap the icon):_ **VibeTree
  upgrades the current terminal instead of opening a second window.** vibeterm now exports
  `VIBETERM=1` in its PTY; `vibe tree` inside vibeterm (on a real tty) renders the console
  TUI **in place** — the plain shell becomes a "VibeTree terminal" for the session — instead
  of spawning another Electron window. While the tree is open, vibeterm's window + taskbar
  icon swaps to `vibetree` and reverts to its launch icon on exit, via an in-band `OSC 7773`
  the renderer forwards to `win.setIcon` (**Windows + Linux**; a no-op on macOS — the
  platform gap). A `user_attended` guard stops a no-tty `-t` (a GUI double-click, or a
  `| pipe`) from blocking on the TUI — it falls back to spawning the app. New: `tree/host.rs`
  + PROP-042 §5.1. Floor-green (self-check all green); verified the in-place path emits the
  `OSC 7773 ; vibetree` and spawns **no** second window, and the packaged vibeterm carries the
  env + OSC handler + IPC. **Owner's eyes** confirm the visual taskbar-icon swap in a live
  vibeterm (the OS titlebar/taskbar icon is not scriptable to screenshot here). Redeployed.
- _2026-07-19 (fix — owner: the `vibetree` launcher should upgrade in place too):_ **`vibetree`
  now upgrades the terminal instead of opening a window.** Root cause: `vibetree.exe` was
  GUI-subsystem (to avoid a console flash on double-click), but a GUI process **the shell does
  not wait for** — so its `vibe tree -t` child raced the shell prompt for the PTY. Switched
  `vibetree` to **console-subsystem** + a terminal-aware core
  (`vibe_launcher::run_terminal_aware`): from a terminal (`$VIBETERM`, or a console shared
  with a shell — via `GetConsoleProcessList`) it runs `vibe tree -t` **inheriting stdio** so it
  renders in place and the shell waits (then `vibe tree` does the TUI + icon swap); when
  **double-clicked** (it owns a fresh console alone) it hides that console (`ShowWindow
  SW_HIDE`) and spawns a window as before. `vibeterm` stays GUI-subsystem (window-only).
  PROP-043 #spawn updated. Verified: `vibetree.exe` is now CONSOLE-subsystem (`vibeterm.exe`
  stays GUI), green icon intact, floor-green. **Tradeoff:** a brief console blink on a raw
  double-click (the Start-menu shortcut launches minimised to soften it). This is a
  launcher-only change — rebuilt + reinstalled the release exe to `~/opt/bin`, no new `vibe`
  instance needed.

## 9. REPORT — (written at close; checks §5 P1–P5)

_pending._
