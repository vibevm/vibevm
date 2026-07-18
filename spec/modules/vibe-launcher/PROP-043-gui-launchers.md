# PROP-043 — GUI launchers {#root}

Status: accepted (VIBE-LAUNCHERS campaign, 2026-07-18).
Module: `vibe-launcher` (`crates/vibe-launcher`).

vibevm ships small **GUI launchers** — double-clickable desktop entry points that
open a specific `vibe` experience in its graphical form. The first two are
**VibeTree** (`vibe tree -t`) and **VibeTerm** (`vibe term`). A launcher is a
thin, GUI-subsystem binary over a shared core; the family grows by a one-line
registry entry, never by new machinery. The heavy execution stays in `vibe`; the launcher only resolves it,
starts it without a console flash, and reports failure where a user can see it.

Design lore + phases: `spec/terraforms/VIBE-LAUNCHERS-PLAN-v0.1.md`.

## Resolve `vibe` without a PATH search {#resolve}

REQ. A launcher resolves the active `vibe` binary **relative to its own
location** first: installed in `…/opt/bin`, it reads the VVM `current` pointer
(`…/opt/vibevm/current`) and runs that instance's `vibe` — the same derivation
the `vibe` shim and `selfloc` use. A `PATH` lookup is a fallback only (an
Explorer-inherited `PATH` is stale until re-login). Resolution failure is a
reported error (`#report`), never a silent exit.

## Start the child without a console flash {#spawn}

REQ. A **window-only** launcher (e.g. `vibeterm`) is compiled for the GUI
subsystem (`windows_subsystem = "windows"` on Windows) so a double-click
allocates **no console window**. It spawns `vibe` with the platform's no-window
creation flag (`CREATE_NO_WINDOW` on Windows) so the console-subsystem child does
not flash a window either. The launcher waits for `vibe` to start the graphical
app and exits; `vibe` itself detaches the desktop app.

REQ. A **terminal-aware** launcher (e.g. `vibetree`, whose command has an
in-terminal console mode) is instead **console-subsystem** — a GUI-subsystem
process the shell does not wait for, which would race a hosted TUI against the
shell prompt. Launched **from a terminal** ($VIBETERM, or a console shared with a
shell) it runs the sub-command **in place**, inheriting stdio so the child renders
in the current terminal and the shell waits (PROP-042 §5.1). **Double-clicked**
(it owns a fresh console alone), it hides that console and spawns the app
windowless as above — so this entry stays effectively window-only, at the cost of
a brief console blink the minimised-launch shortcut mitigates.

## Fail loud, graphically {#report}

REQ. A GUI-subsystem process has no visible stderr, so every failure — `vibe`
unresolved, spawn error, non-zero child exit — is surfaced in a **native
dialog** (`MessageBox` on Windows; the platform equivalent elsewhere), naming
the fix. A launcher that dies silently on a double-click is forbidden.

## One core, N thin binaries, a declarative registry {#registry}

REQ. The launcher core (resolve, spawn, report) is written **once**; each
launcher is a thin binary whose target `vibe` sub-command is compiled in. The
set of launchers is a single declarative registry (`name → argv → icon`); adding
one is a registry entry plus a thin binary, with no core change. Dynamic,
after-install minting of launchers (third-party packages/prompts) is a separate
future system, not this module.

| Launcher | Binary | `vibe` argv | Icon (`assets/icons/`) |
|----------|--------|-------------|------------------------|
| VibeTree | `vibetree` | `tree -t` | `vibetree` — emerald node-graph |
| VibeTerm | `vibeterm` | `term` | `vibeterm` — coral prompt `>_` |

The registry is materialised as the `LAUNCHERS` table in
`crates/vibe-launcher/build.rs` (binary → icon, for per-binary embedding) plus
each binary's compiled-in argv (`src/bin/<name>.rs`, a one-liner over
[`run`](#spawn)).

## Icon: family identity, window matches launcher {#icon}

REQ. A launcher embeds its family icon from `assets/icons/` (Windows: the
multi-resolution `.ico`, its 256 layer the Start-menu tile) — **per binary**, so
`vibetree.exe` and `vibeterm.exe` carry different icons from one crate. The
graphical window a launcher opens carries the **same** icon as the launcher
itself, so the whole path reads as one app:

- **VibeTree** — the window is forced to `vibetree` (`vibe tree -t` passes
  `--icon vibetree`), matching `VibeTree.exe`.
- **VibeTerm** — the vibeterm window's **default** icon *is* `vibeterm`
  (`apps/vibeterm/resources/icon.*`), so plain `vibe term` already matches
  `VibeTerm.exe` with no override.

`vibe`'s other neutral surfaces keep the `default` icon.

## Never {#never}

- Never search `PATH` before the self-relative `current` pointer (`#resolve`).
- Never spawn the child so a console window can appear (`#spawn`).
- Never let a launcher exit silently on failure — always the dialog (`#report`).
- Never grow the family by copying launcher logic; add a registry entry
  (`#registry`).
- Never let the opened window's icon diverge from the launcher's (`#icon`).
