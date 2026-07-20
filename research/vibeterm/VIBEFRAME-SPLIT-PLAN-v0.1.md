# vibeframe split — copy the simple terminal out of vibeterm (plan v0.1)

**Status: PLANNED (2026-07-19)** — owner-commissioned. Self-contained; a fresh session can execute
it end to end. Scoped while the parent session's context was low, so it is deliberately explicit.

## Concept {#concept}

- **vibeframe** = a **COPY** of today's *simple, single-window* vibeterm implementation
  (`apps/vibeterm/` as it stands now). Role: the stable **terminal FRAME that VibeTree runs in** as a
  standalone app, and the AIUI-observation host for `vibe tree`. It stays **simple** and stable.
- **vibeterm** = the **COMPLEX** multi-tab / multi-window AI-UI workspace shell (PROP-044, the big
  campaign). **LEFT IN PLACE, untouched**; keeps advancing per research → design → execution.
- So: two terminals — **vibeframe** (simple, VibeTree's host) and **vibeterm** (complex, the
  standalone product / everything else). **COPY, not move.**

## Routing after the split {#routing}

| Entry | What it hosts | App |
|---|---|---|
| `vibe tree -t` (interactive tree in a window) | the `vibe tree` TUI | **vibeframe** |
| `VibeTree.exe` / Start-menu VibeTree (the spawn-desktop path) | `vibe tree` | **vibeframe** |
| `vibe aiui open` (observe `vibe tree`) | `vibe tree` | **vibeframe** |
| in-place upgrade (`vibe tree` run *inside* a vibe terminal) | detect + upgrade in place | vibeframe marker |
| `vibe term` (standalone terminal) | shell / the workspace | **vibeterm** (unchanged, PROP-044 D2) |
| `vibeterm.exe` | vibeterm | **vibeterm** (unchanged) |

## Steps {#steps}

1. **Copy** `apps/vibeterm/` → `apps/vibeframe/`. Only source commits (node_modules is gitignored);
   the new dir needs its own `npm install` (or copy node_modules for immediate runnability).
2. **Rename identifiers** in the copy: `package.json` name → `@org.vibevm/vibeframe` + description
   ("simple terminal frame — VibeTree's host"); `[vibeterm]` → `[vibeframe]` log tags; `<title>` +
   `BrowserWindow` title; the in-terminal **env marker** (see Decisions); icons (reuse vibeterm's for
   now — own icon later). Keep the shared bits: `OSC 7773` icon protocol, the `~/.vibe/aiui`
   discovery dir, the control-server/CDP surface (vibeframe still supports `--control` for aiui).
3. **vibe-cli resolver** — the terminal-app locator (PROP-042 §5 `#vibe-term`: `$VIBEVM_VIBETERM` →
   packaged `vibeterm/` → dev `apps/vibeterm`). Route the **tree/aiui** paths to **vibeframe** (add
   `$VIBEVM_VIBEFRAME` override → packaged `vibeframe/` → dev `apps/vibeframe`); keep `vibe term` →
   vibeterm. Find the locator fn in `crates/vibe-cli` (the `vibe term` / `vibe aiui open` /
   `vibe tree -t` spawn path).
4. **vibe-launcher** (`crates/vibe-launcher`) — `VibeTree.exe`'s spawn-desktop path (when NOT already
   in a terminal) currently launches vibeterm; make it launch **vibeframe**. `vibeterm.exe` unchanged.
5. **Packaging** — `vibe self update` / `scripts/package.mjs` + the vibe-cli packaging list must
   include `apps/vibeframe` in the instance (next to or instead-of vibeterm for the tree path).
6. **Specs** — reflect "simple terminal **vibeframe** + complex terminal **vibeterm**":
   - New `spec/modules/vibeframe/PROP-045-*` (confirm 045 is free) — the simple-frame contract:
     single-window host for a console program; VibeTree's frame; the AIUI-observation terminal plane.
   - PROP-042 §4 `#aiui-cli` / §5 `#vibe-term` / §5.1 `#in-place-upgrade` — the tree/aiui terminal
     plane is **vibeframe**; update the resolver + the env marker.
   - PROP-043 (launchers) — `VibeTree.exe` spawns **vibeframe**.
   - PROP-036 §2.13 (`#project-resolution`) — update if it names the terminal.
   - PROP-044 — cross-note: vibeterm = the complex terminal; vibeframe = the simple host.
7. **Verify** — `bash tools/self-check.sh` green; `cd apps/vibeframe && npm start` runs; `vibe tree -t`
   opens **vibeframe**; `vibe term` opens **vibeterm**; `vibe aiui open` observation works; vibeterm is
   byte-for-byte unchanged.

## Decisions (confirm with owner) {#decisions}

- **In-terminal env marker.** *Lean:* vibeframe sets `VIBEFRAME=1`; the vibe-cli/launcher
  in-terminal detection accepts **either** `VIBEFRAME` or `VIBETERM` (both are vibe terminals). This
  keeps in-place upgrade working from either host.
- **vibeframe icon.** *Lean:* reuse vibeterm's icon initially; give vibeframe its own later.
- **No own `.exe` for vibeframe.** *Lean:* vibeframe is only ever spawned by `VibeTree.exe` and by
  `vibe` — it needs no standalone launcher of its own.
- **`vibe aiui open` host.** *Lean:* vibeframe (it observes `vibe tree`).

## Risks / guardrails {#risks}

- **Cross-cutting** (apps + vibe-cli resolver + vibe-launcher + packaging + specs) — land the
  behaviour-changing **redirects atomically**; a half-done redirect breaks `vibe tree -t`.
- **Do NOT touch vibeterm** — leave it fully in place (the whole point is a copy).
- **node_modules** is per-app and gitignored — `npm install` (or an `@electron/rebuild`) in vibeframe.
- Discipline: heredoc commits, **no AI attribution**, atomic commits, **never write the reference
  app's real name / any forbidden token in the repo**, edits via Edit/Write only.

## Note {#note}

Big and cross-cutting; safe to interrupt only at atomic boundaries. **Recommend executing in a
session with full context.** The copy (step 1) is safe/reversible on its own; the redirects (steps
3–5) change behaviour and should land together.

## Self-installing launchers — DONE (Windows) {#deferred-launchers}

Owner point (2026-07-19): **`vibe self update` / the install pipeline should itself place the GUI
launchers (VibeTree / VibeTerm / VibeFrame) into the install bin dir (`~/opt/bin`) and create their
Start-menu shortcuts** — instead of a manual `cargo build --release -p vibe-launcher` + `cp` + a
hand-made shortcut. A self-contained install: updating vibe installs/refreshes its launchers +
shortcuts too.

**Built 2026-07-19 (Windows path).** `crates/vibe-cli/src/commands/vvm/launchers.rs` — the
`LauncherInstaller` seam (native impl live, no-op for the gate), invoked at the tail of
`perform_install` on **both** the new-instance and dedup-skip paths (idempotent, self-bootstrapping).
It `cargo build -p vibe-launcher` into the managed target-dir, **rename-aside**-places each exe into
the shim dir (a running launcher is renamed to a `.old-<n>` sidecar, swept next update), and creates
per-user Start-menu `.lnk`s (`Programs\vibevm\<Label>.lnk`) via PowerShell `WScript.Shell`. Best-effort
throughout — a locked exe / missing rc.exe / shortcut failure is a note, never an install failure.
Contract: **PROP-043 #self-install**. Cross-platform exe placement works; **Windows shortcuts only**
for now — Linux `.desktop` / macOS `.app` are tracked separately (owner: tested apart from this task).
