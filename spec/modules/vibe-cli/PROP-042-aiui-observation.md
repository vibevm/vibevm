# PROP-042 — AIUI observation: the render plane & the `vibe aiui` surface

**Status:** ACTIVE (v0.1, 2026-07-16). **Module:** `vibe-cli`. **Campaign:**
[`spec/terraforms/TERMINAL-AIUI-PLAN-v0.1.md`](../../terraforms/TERMINAL-AIUI-PLAN-v0.1.md).
**Related:** PROP-037 (the `vibe tree` TUI it observes), PROP-039 §11.3 (the model
plane / `vibe-actions::aiui`), PROP-036 (the tree model).

This contract governs the **render plane** — a terminal-free way to render the
`vibe tree` TUI to a symbolic snapshot so an agent (or a golden test) can *see*
the interface without a real terminal — and the `vibe aiui` CLI surface that
exposes it. The terminal plane (vibeterm) and the model plane are governed
elsewhere (the campaign plan / a vibeterm PROP / PROP-039).

---

## 1. The render plane {#render-plane}

REQ. The TUI renders **headlessly**: given a built `PackageTree`, a terminal size
`cols×rows`, and an optional **key script** (§3), the surface drives the real
input + render path — `input::handle` for each scripted key, then `render::draw`
into an off-screen `ratatui::Buffer` — and returns that Buffer. No terminal, no
alternate screen, no raw mode, no `rat-salsa` loop; the entrypoint is a pure
function of `(tree, size, script)`.

REQ. The headless render is **deterministic**: it uses the built-in theme
defaults (the canonical Rosé Pine palette, Tier 3 — §PROP-037 §2.2) and never
loads user settings from disk, so the same `(tree, size, script)` always yields
the same Buffer. Snapshot callers pin `tree` (a fixture), `size`, and `script`.

REQ. A scripted key that would **escape the process or mutate the world** is
refused, not executed: `F4` (spawns the settings subprocess) and `F6`/`Shift+F6`
(write the clipboard) are rejected by the key-script parser (§3). The render
plane observes; it does not act outside the model.

## 2. The snapshot contract {#snapshot-contract}

REQ. A rendered Buffer projects to one of two **snapshot formats**, the same
schema every observation plane emits:

- **`text`** — the glyph grid: one line per row, each row the concatenation of
  the cells' symbols with trailing whitespace trimmed. The golden-file form
  (committed `.snap.txt`, re-rendered and diffed).
- **`cells`** — JSON: `{cols, rows, rows:[[run,…],…]}` where each **run** is
  `{n, ch, fg?, bg?, mods?}` — `n` cells of glyph `ch` sharing a style, run-length
  encoded per row; `fg`/`bg` are `#rrggbb` (or an ANSI role name), `mods` the set
  of `bold`/`dim`/`italic`/`underlined`/`reversed` present. Enables style/colour
  assertions (e.g. "the active group's border run is the accent colour").

REQ. `text` is **lossless for layout** (every cell's glyph, in grid order) and
`cells` is **lossless for style**; neither invents content. A blank cell is a
space; the trim is per-row and right-only, so column alignment within a row is
preserved.

## 3. The key script {#key-script}

REQ. A **key script** is a space-separated list of key names driving the TUI
before the snapshot. The grammar: function keys `F1`–`F12`; navigation `Up`,
`Down`, `Left`, `Right`; `Enter`, `Esc`, `Tab`, `BackTab`, `Space`, `Backspace`;
a `Shift+` prefix on any of them (e.g. `Shift+Left`, `Shift+Tab` ≡ `BackTab`).
Names are case-insensitive. An unknown name, or a refused side-effecting key
(`F4`, `F6`; §1), is a hard error naming the offending token — never a silent
skip.

REQ. The **render plane** (§1) turns each key name straight into a
`crossterm::event::Event` — terminal-free, no escape bytes. The **terminal
plane** (§4, `vibe aiui send`) must instead encode each name to the bytes the
hosted program's platform expects, and the encoding is **platform-specific**: on
Unix, the standard xterm VT sequences (SS3 `ESC O P`–`S` for F1–F4, CSI for the
rest); on **Windows**, **win32-input-mode** (`ESC [ Vk;Sc;Uc;Kd;Cs;Rc _` — a
key-down record then a key-up), the form a ConPTY translates into the console
`INPUT_RECORD`s a raw reader expects. The raw VT form is **not** reliable on
Windows: conhost synthesises a key record from it for a cooked reader (a shell)
but not for a raw reader (a crossterm TUI such as `vibe tree`), so the keys are
silently dropped. A caller therefore drives the same key script identically on
either plane; the encoding difference is the implementation's to hide.

## 4. The `vibe aiui` surface {#aiui-cli}

REQ. `vibe aiui` is the agent-facing command family. Its render-plane verb:

```
vibe aiui render [--path <dir>] [--size <COLSxROWS>] [--send "<script>"] [--format text|cells]
```

builds the `vibe tree` model at `--path` (the same resolver `vibe tree` uses),
drives `--send` (§3) at `--size` (default `80x24`), and prints the `--format`
snapshot (§2, default `text`) to stdout. It is read-only and non-interactive:
it never enters the TUI, spawns a terminal, or touches user state.

REQ. The **terminal-plane** verbs drive a live vibeterm control session:

```
vibe aiui open     [--exec <cmd>] [--size <COLSxROWS>] [--timeout-ms <n>]
vibe aiui send     <key>... [--text <literal>] [--session <pid>]
vibe aiui snapshot [--session <pid>]
vibe aiui wait     [--idle-ms <n>] [--timeout-ms <n>] [--session <pid>]
vibe aiui close    [--session <pid>]
```

`open` launches a **windowless** vibeterm running `--exec` (default: the console
`vibe tree` over the current directory) with a control server, waits for its
discovery file, and prints the session id (the vibeterm pid). `send` drives a key
script (§3) and/or literal `--text`; `snapshot` prints the live grid (§2); `wait`
blocks until the hosted program has answered the last input **and** the grid has
settled (deterministic snapshots — never the pre-key screen); `close` tears the
session down. A verb defaults to the most recent session; `--session <pid>`
targets a specific one.

REQ. The control transport is **loopback-only and token-guarded**. A `--control`
vibeterm serves JSON over `http://127.0.0.1:<ephemeral>` and writes a discovery
file `~/.vibevm/aiui/<pid>.json` plus a `latest.json` pointer, each
`{ port, token, pid, startedAt }` at mode `0600`. Every request carries the
bearer token; the socket binds `127.0.0.1` only. `open` accepts a discovered
session only when its `startedAt` is at or after the spawn instant, so a stale
`latest.json` is never mistaken for the freshly-spawned one. The model-plane
`state` verb lands in a later phase and is governed here as it arrives.

## 5. The `vibe term` launcher {#vibe-term}

REQ. `vibe term` launches the **vibeterm** terminal app hosting an interactive
shell, so the terminal can be used and eyeball-debugged standalone. The shell is
**detected**: on Windows, modern PowerShell 7+ (`pwsh`) is preferred over the
built-in Windows PowerShell 5.1 — resolved via the standard install locations
(`%ProgramFiles%\PowerShell\7\pwsh.exe`, `%LOCALAPPDATA%\…\WindowsApps\pwsh.exe`)
then `PATH`, falling back to `…\WindowsPowerShell\v1.0\powershell.exe`; on other
platforms `$SHELL`, falling back to `/bin/sh`. An explicit `--exec <cmd>`
overrides the detected shell.

REQ. vibeterm is located **without a `PATH` search**: an explicit
`$VIBEVM_VIBETERM` directory wins, else a development fallback walks up from the
running binary for `research/vibeterm`. Its Electron binary is resolved through
the app's own `node_modules/electron/path.txt`. A missing install fails with a
message naming the setup step, never a silent hang.

## 6. Never {#never}

- Never load user settings into a snapshot render — determinism dies and goldens
  churn. Defaults only.
- Never execute a side-effecting key (`F4`/`F6`) in the render plane.
- Never let a snapshot format invent or drop content — `text` is every glyph in
  grid order; `cells` is every run with its true style.
- Never enter the interactive TUI from `vibe aiui` — it is headless by contract.
