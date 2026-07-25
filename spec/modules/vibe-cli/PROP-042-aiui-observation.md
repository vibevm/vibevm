# PROP-042 — AIUI observation: the render plane & the `vibe aiui` surface {#root}

<status stage="spec" state="done" comment="B0 2026-07-24: ACTIVE v0.1, 2026-07-16; fact grain 2026-07-24"/>

##status-line **Status:** ACTIVE (v0.1, 2026-07-16). **Module:** `vibe-cli`.
**Related:** PROP-037 (the `vibe tree` TUI it observes), PROP-039 §11.3 (the model
plane / `vibe-actions::aiui`), PROP-036 (the tree model). The terminal products
(vibeterm, vibeframe) and their contracts now live in the `vibevm-term` products
repo — this PROP cites them as cross-repo contracts
(`spec://vibeterm/*`, `spec://term-common/*`); the `vibe aiui` / `vibe term`
CLI surface itself stays on the host. @spec/done

- ##PROP-SCOPE This contract governs the **render plane** — a terminal-free way to render the
  `vibe tree` TUI to a symbolic snapshot so an agent (or a golden test) can *see*
  the interface without a real terminal — and the `vibe aiui` CLI surface that
  exposes it. @spec/done
- ##other-planes The terminal plane (vibeterm) and the model plane are governed
  elsewhere (a vibeterm PROP in `vibevm-term` / PROP-039). @spec/done

---

## 1. The render plane {#render-plane}

- ##HEADLESS-RENDER REQ. The TUI renders **headlessly**: given a built `PackageTree`, a terminal size
  `cols×rows`, and an optional **key script** (§3), the surface drives the real
  input + render path — `input::handle` for each scripted key, then `render::draw`
  into an off-screen `ratatui::Buffer` — and returns that Buffer. @impl/done
- ##NO-TERMINAL No terminal, no alternate screen, no raw mode, no `rat-salsa` loop; the
  entrypoint is a pure function of `(tree, size, script)`. @impl/done

- ##RENDER-DETERMINISTIC REQ. The headless render is **deterministic**: it uses the built-in theme
  defaults (the canonical Rosé Pine palette, Tier 3 — §PROP-037 §2.2) and never
  loads user settings from disk, so the same `(tree, size, script)` always yields
  the same Buffer. @impl/done
- ##SNAPSHOT-PINNING Snapshot callers pin `tree` (a fixture), `size`, and `script`. @impl/done

- ##SIDE-EFFECT-KEYS-REFUSED REQ. A scripted key that would **escape the process or mutate the world** is
  refused, not executed: `F4` (spawns the settings subprocess) and `F6`/`Shift+F6`
  (write the clipboard) are rejected by the key-script parser (§3). @impl/done
- ##OBSERVES-NOT-ACTS The render plane observes; it does not act outside the model. @impl/done

## 2. The snapshot contract {#snapshot-contract}

##SNAPSHOT-FORMATS REQ. A rendered Buffer projects to one of two **snapshot formats**, the same
schema every observation plane emits: @impl/done

- ##FMT-TEXT **`text`** — the glyph grid: one line per row, each row the concatenation of
  the cells' symbols with trailing whitespace trimmed. The golden-file form
  (committed `.snap.txt`, re-rendered and diffed). @impl/done
- ##FMT-CELLS **`cells`** — JSON: `{cols, rows, rows:[[run,…],…]}` where each **run** is
  `{n, ch, fg?, bg?, mods?}` — `n` cells of glyph `ch` sharing a style, run-length
  encoded per row; `fg`/`bg` are `#rrggbb` (or an ANSI role name), `mods` the set
  of `bold`/`dim`/`italic`/`underlined`/`reversed` present. Enables style/colour
  assertions (e.g. "the active group's border run is the accent colour"). @impl/done

- ##FMT-LOSSLESS REQ. `text` is **lossless for layout** (every cell's glyph, in grid order) and
  `cells` is **lossless for style**; neither invents content. @impl/done
- ##BLANK-TRIM A blank cell is a
  space; the trim is per-row and right-only, so column alignment within a row is
  preserved. @impl/done

## 3. The key script {#key-script}

- ##KEY-SCRIPT-GRAMMAR REQ. A **key script** is a space-separated list of key names driving the TUI
  before the snapshot. The grammar: function keys `F1`–`F12`; navigation `Up`,
  `Down`, `Left`, `Right`; `Enter`, `Esc`, `Tab`, `BackTab`, `Space`, `Backspace`;
  a `Shift+` prefix on any of them (e.g. `Shift+Left`, `Shift+Tab` ≡ `BackTab`).
  Names are case-insensitive. @impl/done
- ##UNKNOWN-KEY-ERROR An unknown name, or a refused side-effecting key
  (`F4`, `F6`; §1), is a hard error naming the offending token — never a silent
  skip. @impl/done

- ##RENDER-PLANE-EVENTS REQ. The **render plane** (§1) turns each key name straight into a
  `crossterm::event::Event` — terminal-free, no escape bytes. @impl/done
- ##TERMINAL-PLANE-ENCODING The **terminal
  plane** (§4, `vibe aiui send`) must instead encode each name to the bytes the
  hosted program's platform expects, and the encoding is **platform-specific**: on
  Unix, the standard xterm VT sequences (SS3 `ESC O P`–`S` for F1–F4, CSI for the
  rest); on **Windows**, **win32-input-mode** (`ESC [ Vk;Sc;Uc;Kd;Cs;Rc _` — a
  key-down record then a key-up), the form a ConPTY translates into the console
  `INPUT_RECORD`s a raw reader expects. @impl/done
- ##VT-UNRELIABLE-WINDOWS The raw VT form is **not** reliable on
  Windows: conhost synthesises a key record from it for a cooked reader (a shell)
  but not for a raw reader (a crossterm TUI such as `vibe tree`), so the keys are
  silently dropped. @impl/done
- ##SCRIPT-PLANE-IDENTICAL A caller therefore drives the same key script identically on
  either plane; the encoding difference is the implementation's to hide. @impl/done

## 4. The `vibe aiui` surface {#aiui-cli}

##AIUI-FAMILY REQ. `vibe aiui` is the agent-facing command family. Its render-plane verb: @impl/done

```
vibe aiui render [--path <dir>] [--size <COLSxROWS>] [--send "<script>"] [--format text|cells]
```

- ##RENDER-VERB-SEMANTICS builds the `vibe tree` model at `--path` (the same resolver `vibe tree` uses),
  drives `--send` (§3) at `--size` (default `80x24`), and prints the `--format`
  snapshot (§2, default `text`) to stdout. @impl/done
- ##RENDER-READ-ONLY It is read-only and non-interactive:
  it never enters the TUI, spawns a terminal, or touches user state. @impl/done

##TERMINAL-VERBS REQ. The **terminal-plane** verbs drive a live vibeterm control session: @impl/done

```
vibe aiui open     [--exec <cmd>] [--size <COLSxROWS>] [--timeout-ms <n>]
vibe aiui send     <key>... [--text <literal>] [--session <pid>]
vibe aiui snapshot [--session <pid>]
vibe aiui wait     [--idle-ms <n>] [--timeout-ms <n>] [--session <pid>]
vibe aiui close    [--session <pid>]
vibe aiui inspect   <expr> [--session <pid>]
vibe aiui pty-stop  [--session <pid>]
vibe aiui pty-start [--session <pid>]
vibe aiui scrollbar <auto|on|off> [--session <pid>]
```

- ##VERB-OPEN `open` launches a **windowless** vibeterm running `--exec` (default: the console
  `vibe tree` over the current directory) with a control server, waits for its
  discovery file, and prints the session id (the vibeterm pid). @impl/done
- ##VERB-SEND `send` drives a key script (§3) and/or literal `--text`. @impl/done
- ##VERB-SNAPSHOT `snapshot` prints the live grid (§2). @impl/done
- ##VERB-WAIT `wait` blocks until the hosted program has answered the last input **and** the
  grid has settled (deterministic snapshots — never the pre-key screen). @impl/done
- ##VERB-CLOSE `close` tears the session down. @impl/done
- ##VERB-INSPECT `inspect` evaluates a JavaScript expression in the live renderer page over
  CDP and prints its return value as JSON — the agent reads the renderer's **real** runtime
  state (the xterm grid's cols and cell metrics, the scrollbar box) instead of inferring it
  from a snapshot. Requires a `--control` session. @impl/done
- ##VERB-PTY-STOP `pty-stop` stops the hosted program — the PTY child — **without** restarting
  Electron: the renderer, the CDP endpoint and the discovery file all stay live, so the
  program's binary is freed for a rebuild while the session survives. @impl/done
- ##VERB-PTY-START `pty-start` (re)spawns the hosted program at the current grid. Paired with
  `pty-stop` around a rebuild it is the fast TUI-preview loop: the agent sees the change
  without reconnecting CDP or relaunching Electron. @impl/done
- ##VERB-SCROLLBAR `scrollbar` sets the scrollbar policy live — `auto` (hidden for a
  full-screen TUI, shown for a shell), `on` (always), `off` (never). The renderer refits the
  grid; no Electron restart. Requires a `--control` session. @impl/done
- ##SESSION-DEFAULT A verb defaults to the most recent session; `--session <pid>`
  targets a specific one. @impl/done

##MODEL-VERB REQ. The **model-plane** verb projects the TUI state — no rendering at all: @impl/done

```
vibe aiui state [--path <dir>] [--send "<script>"]
```

- ##STATE-SEMANTICS builds the `vibe tree` model at `--path`, drives `--send` (§3), and prints a
  serialisable `ModelView` (PROP-039 §11.2/§11.3) — display mode, ordering, the
  active tab, the selection, the visible rows, and which modals are open. @impl/done
- ##STATE-READ-ONLY It is
  read-only and non-interactive; it observes structured state an agent asserts on
  (flow, focus, open menus), never pixels. @impl/done
- ##STATE-PURE The projection is a pure function of
  the built `App` and carries no rendering types. @impl/done

- ##CONTROL-LOOPBACK REQ. The control transport is **loopback-only and token-guarded**. A `--control`
  vibeterm serves JSON over `http://127.0.0.1:<ephemeral>`. @impl/done
- ##DISCOVERY-FILES It writes a discovery
  file `~/.vibe/aiui/<pid>.json` plus a `latest.json` pointer, each
  `{ port, token, pid, startedAt }` at mode `0600`. @impl/done
- ##BEARER-TOKEN Every request carries the
  bearer token; the socket binds `127.0.0.1` only. @impl/done
- ##STALE-SESSION-GUARD `open` accepts a discovered
  session only when its `startedAt` is at or after the spawn instant, so a stale
  `latest.json` is never mistaken for the freshly-spawned one. @impl/done
- ##state-governance The model-plane `state` verb is governed by PROP-039 §11.2/§11.3; its `vibe tree`
  projection is prototyped here per PROP-039 §13 (the TUI is the reference surface). @spec/done

## 5. The `vibe term` launcher {#vibe-term}

- ##TERM-LAUNCHER REQ. `vibe term` launches the **vibeterm** terminal app hosting an interactive
  shell, so the terminal can be used and eyeball-debugged standalone. @impl/done
- ##SHELL-DETECTION The shell is
  **detected**: on Windows, modern PowerShell 7+ (`pwsh`) is preferred over the
  built-in Windows PowerShell 5.1 — resolved via the standard install locations
  (`%ProgramFiles%\PowerShell\7\pwsh.exe`, `%LOCALAPPDATA%\…\WindowsApps\pwsh.exe`)
  then `PATH`, falling back to `…\WindowsPowerShell\v1.0\powershell.exe`; on other
  platforms `$SHELL`, falling back to `/bin/sh`. @impl/done
- ##EXEC-OVERRIDE An explicit `--exec <cmd>`
  overrides the detected shell. @impl/done

##APP-RESOLUTION REQ. The terminal app (`vibeterm` for `vibe term`, `vibeframe` for `vibe tree
-t`) is located in three tiers in order: @impl/done

1. ##TIER-ENV an explicit `$VIBEVM_<APP>`
   directory wins (`<APP>` is the uppercased app name — `VIBETERM` / `VIBEFRAME`,
   an override a developer or a launcher sets); @impl/done
2. ##TIER-PACKAGED-INSTANCE else an installed `vibe`
   checks the packaged `<app>/` shipped inside its own instance dir, next to its
   binary (the legacy, pre-extraction layout — kept for back-compat with instances
   that still carry it); @impl/done
3. ##TIER-PATH else a `PATH` lookup for the app-named packaged
   binary (`vibeterm` / `vibeframe`) — the **extracted-product path**, how the
   vibevm-term repo's `<app> self install` publishes the product. The directory the
   binary sits in is treated as the packaged root. @impl/done

- ##PACKAGED-VS-DEV The resolver distinguishes a
  **packaged** dir (electron binary at its root, `resources/app/` inside — invoked
  directly, no app-path arg) from a **dev** dir (Electron resolved via
  `node_modules/electron/path.txt`, the app dir passed as a positional arg — only
  reachable through `$VIBEVM_<APP>` now that the in-tree `apps/` source has moved
  to vibevm-term). @impl/done
- ##RESOLUTION-FAILURE Resolution failure returns a typed error; `vibe term` /
  `vibe frame` surface it to the user, while `vibe tree` falls back to running
  the console TUI in place (§5.1). @impl/done

### 5.1 In-place upgrade & the icon protocol {#in-place-upgrade}

- ##VIBETERM-ENV REQ. vibeterm sets `VIBETERM=1` in its PTY environment. @impl/done
- ##IN-PLACE-UPGRADE A `vibe tree` launched
  **inside** vibeterm (this env present) does not spawn a second window — it
  upgrades the current terminal in place: the `-t` / vibeterm launch resolves to
  the in-terminal console TUI here, so a plain shell becomes a "VibeTree terminal"
  for the session (PROP-036 §2.13). Outside vibeterm, `-t` still opens the desktop
  app. @impl/done

- ##ICON-SWAP REQ. While the tree is open in that upgrade, vibeterm's **window + taskbar icon**
  is swapped to `vibetree`, reverting to the window's launch icon on exit. The swap
  is in-band: `vibe tree` emits `OSC 7773 ; <icon-name> ST` (an empty name reverts);
  the vibeterm renderer forwards the name to the main process, which calls
  `win.setIcon`. @impl/done
- ##ICON-PLATFORM-GAP **Windows + Linux only** — `setIcon` is a no-op on macOS (the app
  owns its Dock icon there), the documented platform gap. @impl/done
- ##OSC-DISCARDED In any non-vibeterm
  terminal the OSC is an unknown sequence, harmlessly discarded. @impl/done

## 6. Never {#never}

- ##NEVER-USER-SETTINGS Never load user settings into a snapshot render — determinism dies and goldens
  churn. Defaults only. @impl/done
- ##NEVER-SIDE-EFFECT-KEYS Never execute a side-effecting key (`F4`/`F6`) in the render plane. @impl/done
- ##NEVER-INVENT-CONTENT Never let a snapshot format invent or drop content — `text` is every glyph in
  grid order; `cells` is every run with its true style. @impl/done
- ##NEVER-INTERACTIVE Never enter the interactive TUI from `vibe aiui` — it is headless by contract. @impl/done
