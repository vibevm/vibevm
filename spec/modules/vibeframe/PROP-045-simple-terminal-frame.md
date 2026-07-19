# PROP-045 — vibeframe: the simple terminal frame

**Status:** DRAFT (v0.1, 2026-07-19). **Module:** `vibeframe` (new). **Plan:**
[`research/vibeterm/VIBEFRAME-SPLIT-PLAN-v0.1.md`](../../../research/vibeterm/VIBEFRAME-SPLIT-PLAN-v0.1.md).
**Related:** PROP-042 (AIUI observation — vibeframe is the terminal plane's host
for `vibe tree`), PROP-043 (GUI launchers — `VibeFrame.exe`), PROP-036 §2.13 (the
`vibe tree` project resolution VibeFrame hosts), PROP-044 (vibeterm — the *complex*
terminal, the sibling this frame is deliberately NOT).

This contract governs **vibeframe** — the **simple, single-window Electron
terminal frame**. It is a **copy** of the minimal vibeterm implementation, split
out so **VibeTree has a stable, simple app to run in** as a standalone tool, while
**vibeterm** evolves into the complex multi-tab workspace (PROP-044). The two are
siblings: vibeframe stays simple and stable; vibeterm grows.

---

## 1. Role & scope {#role}

REQ. vibeframe hosts **exactly one** node-pty child rendered by **one** xterm.js in
**one** `BrowserWindow` — the minimal terminal (main-process pty, IPC `pty` /
`input` / `resize` / `ready`, the `--control` AIUI surface + headless mirror + CDP,
the `OSC 7773` icon protocol). It carries **no** multi-tab / multi-window / split
machinery; that is vibeterm's domain (PROP-044). vibeframe is what today's
`apps/vibeterm` already is, re-homed as `apps/vibeframe`.

REQ. vibeframe is the **host for `vibe tree`**: the interactive tree window
(`vibe tree -t`, `VibeTree.exe`) and the AIUI observation session (`vibe aiui open`)
run their console `vibe tree` inside vibeframe. It is also launchable standalone
via `vibe frame` / `VibeFrame.exe`.

## 2. Routing {#routing}

REQ. The terminal-app resolver (`vibe-cli`, PROP-042 §5) routes by target app:
`vibe frame`, `vibe tree -t`, `VibeTree.exe`, and `vibe aiui open` resolve
**vibeframe**; `vibe term` / `VibeTerm.exe` resolve **vibeterm**. Resolution tiers
per app `X`: `$VIBEVM_<X>` override → the instance's packaged `X/` → the dev
`apps/X` walk-up. An app that is not yet packaged **falls back to vibeterm**, so an
installed launcher never fails (it degrades to the complex terminal until the
simple one is packaged).

## 3. Identity & the in-terminal marker {#identity}

REQ. vibeframe's package identity is `@org.vibevm/vibeframe`; its packaged
executable is `vibeframe(.exe)`; its window/launcher icon is a **no-dots** coral
glyph (`assets/icons/vibeframe.*`, hinting at simplicity), distinct from
vibeterm's. It sets **`VIBEFRAME=1`** in its PTY environment.

REQ. A `vibe tree` launched **inside** a vibe desktop terminal upgrades in place
(no second window) when **either** `VIBETERM` **or** `VIBEFRAME` is set — both
markers are honoured by the in-place-upgrade detection (PROP-042
§5.1 `#in-place-upgrade`). Inside vibeframe the tree still swaps the window icon to
`vibetree` via `OSC 7773`, reverting on exit.

## 4. Packaging & install {#packaging}

REQ. The VVM install pipeline (PROP-019 §2.7) packages **both** `apps/vibeterm` →
the instance's `vibeterm/` and `apps/vibeframe` → the instance's `vibeframe/` (the
packager and dist-walker are parameterised by app name). A missing/unpackaged
vibeframe is a graceful skip; `vibe tree -t` then uses the §2 fallback.

REQ. `VibeFrame.exe` is a GUI-subsystem launcher running `vibe frame`, carrying
vibeframe's no-dots icon via the per-binary embed table (PROP-043 #icon); it needs
no standalone app of its own. **Deferred** (VIBEFRAME-SPLIT-PLAN
`#deferred-launchers`): the install pipeline placing the launcher exes + creating
their Start-menu / desktop shortcuts itself, rather than by hand.

## 5. Relationship to vibeterm {#vs-vibeterm}

REQ. vibeframe is **self-contained and stable by intent**: it is not extended with
vibeterm's evolving workspace features. When vibeterm's complex shell (PROP-044)
ships, vibeframe remains the *simple* option — VibeTree's host and a plain
standalone terminal. The two never share renderer code beyond the initial copy;
each evolves on its own line (vibeframe: minimal; vibeterm: workspace).

## Non-goals {#non-goals}

- No multi-tab / split / multi-window (that is vibeterm, PROP-044).
- No new renderer architecture — vibeframe is the minimal terminal verbatim.
- macOS launcher icon parity is bounded by the platform gap (`setIcon` no-op).
