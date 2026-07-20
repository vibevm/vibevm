# vibeframe

A minimal, real Electron terminal. It hosts an interactive PTY (via
[node-pty](https://github.com/microsoft/node-pty)) running a command, and
renders it with [xterm.js](https://xtermjs.org/). This is the visual terminal
an agent or human observes — a debug/test tool today, a terminal we grow.

It starts a shell (or the command you pass via `--exec`), streams its output
into the window, and sends your keystrokes back to it.

## Architecture

Two processes, one PTY, IPC between them:

- **node-pty runs in the Electron _main_ process — never the renderer.** The
  renderer has no `worker_threads`, so constructing a pty there throws
  `Failed to construct 'Worker'`. The renderer is xterm.js only.
- **PTY output** flows main → renderer as `win.webContents.send('pty', data)`
  and is written to the terminal with `term.write(data)`.
- **Keystrokes** flow renderer → main as `ipcRenderer.send('input', data)`
  and are written to the child with `pty.write(data)`.

Graceful teardown (otherwise Electron pops an error dialog):

1. Every `win.webContents.send(...)` is guarded by `!win.isDestroyed()`.
2. The `pty.onData` subscription is disposed **before** teardown, so no byte
   is ever sent to a destroyed window.
3. To stop the child we prefer letting it exit. `pty.kill()` is only called
   inside a `try/catch` — on Windows it throws ConPTY `AttachConsole failed`
   outside a real console, which is swallowed; `app.quit()` reaps the child
   regardless.

The pure argument/command logic lives in `lib/args.mjs`, which imports nothing
from Electron or node-pty, so it is unit-tested under a plain `node --test`
(no native build, no GUI).

## Setup

Requires Node ≥ 22.6 and npm 11.

```sh
cd apps/vibeframe

# 1. Install dependencies. npm 11 blocks native postinstall scripts by
#    default, so node-pty's prebuild and Electron's binary are NOT fetched
#    by `npm install` alone.
npm install

# 2. Build/verify node-pty's native addon (fetches the Windows prebuild +
#    the bundled ConPTY DLL; no C++ toolchain needed).
npm rebuild node-pty --foreground-scripts

# 3. Fetch Electron's own binary (its postinstall was blocked too).
node node_modules/electron/install.js
```

> node-pty is currently published on a `1.1.x` prerelease line. If
> `npm install` cannot resolve `^1.1.0`, pin the current beta explicitly
> (e.g. `npm install node-pty@1.1.0-beta`).
>
> node-pty is built on `node-addon-api` (N-API), so its shipped prebuilds are
> ABI-stable across Node/Electron versions: the same `.node` loads in system
> Node and in Electron 32 (verified — ConPTY spawns correctly under Electron).
> Do **not** run `@electron/rebuild` / `electron-rebuild` against node-pty here:
> it forces `node-gyp`, which trips a broken relative-path `cd shared &&
> GetCommitHash.bat` in `deps/winpty/src/winpty.gyp` and fails. The prebuild is
> correct as shipped; no rebuild is wanted.

## Run

```sh
npm start          # === electron .
```

By default vibeframe launches a plain per-platform shell: `%COMSPEC%` (falling
back to `cmd.exe`) on Windows, `$SHELL` (falling back to `/bin/sh`) elsewhere.

## Flags

Pass flags after `--` so Electron forwards them to the app:

```sh
electron . -- --exec "vibe tree -c" --cols 100 --rows 30
```

| Flag             | Meaning                                              | Default        |
| ---------------- | ---------------------------------------------------- | -------------- |
| `--exec "<cmd>"` | Shell command line to run instead of the default.    | (the shell)    |
| `--cols <n>`     | Terminal width in columns.                           | `84`           |
| `--rows <n>`     | Terminal height in rows.                             | `30`           |
| `--control`      | Start the loopback AIUI control server (see below).  | off            |
| `--headless`     | Run with no OS window (driven + observed over the control server). | off |

Both `--flag value` and `--flag=value` forms work. Malformed or missing values
fall back to the defaults. The first token of `--exec` may be quoted so an
executable path can contain spaces:
`--exec '"C:\Program Files\Git\bin\bash.exe" -l'`.

The smart pwsh-vs-powershell shell detection lives in the Rust `vibe term`
caller, which passes its chosen shell through `--exec`. vibeframe's own default
(`defaultShell`) is just the simple per-platform fallback above, for standalone
`electron .` use.

## Control mode (AIUI)

Passing `--control` starts a small **loopback HTTP+JSON control server** so an
agent can drive and observe a running vibeframe. Without the flag, none of this
is constructed (`@xterm/headless` is not even loaded) and vibeframe is a plain
terminal.

```sh
electron . -- --exec "vibe tree -c" --control
```

On start it:

- binds `127.0.0.1` on an **ephemeral port** (unreachable off-loopback);
- generates a random **Bearer token**;
- mirrors every PTY byte into a headless xterm (in the main process) so it can
  return a text snapshot of the exact grid the window shows;
- writes a **discovery file** at `~/.vibe/aiui/<pid>.json` and a
  `~/.vibe/aiui/latest.json` pointer, each holding
  `{ port, token, pid, startedAt }` (mode `0600`; removed on quit).

Every endpoint requires `Authorization: Bearer <token>` — a missing/wrong token
is `401 {"error":"unauthorized"}`.

| Method + path            | Body                              | Response                                            |
| ------------------------ | --------------------------------- | --------------------------------------------------- |
| `GET /state`             | —                                 | `{ alive, cols, rows, exitCode }`                   |
| `GET /snapshot?format=text` | —                              | `{ cols, rows, text }` (grid, one line/row, `\n`)   |
| `POST /input`            | `{ keys: ["F2","Down"], text }`   | `{ ok: true }`                                      |
| `POST /wait`             | `{ idleMs: 120, timeoutMs: 3000 }`| `{ stable, waitedMs }`                              |
| `POST /close`            | —                                 | `{ ok: true }` then tears down + quits              |

`POST /input` maps each symbolic key name (`Enter`, `Esc`, `Tab`, `BackTab`,
`Space`, `Backspace`, arrows, `Shift+`arrows, `F1`–`F12`; case-insensitive — see
`lib/keymap.mjs`) to the bytes the hosted program expects, writing them in order
and then `text` literally. The encoding is **platform-specific**: standard xterm
VT sequences on Unix, but **win32-input-mode** (`ESC [ Vk;Sc;Uc;Kd;Cs;Rc _`) on
Windows — a ConPTY synthesises a key record from the raw VT form for a cooked
reader (a shell) but not for a raw reader (a crossterm TUI), so the explicit
win32 form is required for keys to reach `vibe tree`. An unknown key is `400`.

`POST /wait` resolves once the terminal has settled: it waits for the program's
first render, and — if a key was sent since the last settle — for that key to be
answered by output, then for `idleMs` of quiet; `timeoutMs` bounds it (`stable`
distinguishes idle-reached from timed-out). This is what makes a `snapshot` after
an `input` deterministic rather than a race with the redraw. Only `format=text`
is supported by `/snapshot`; any other format is `400`.

When the hosted program exits under `--control`, the server (and the window, if
any) stays up — so the final grid and `exitCode` remain observable via `/snapshot`
and `/state` — until the agent calls `POST /close`; without `--control`, an
exiting child closes vibeframe as before.

## Test

```sh
npm test           # === node --test  (auto-discovers test/*.test.mjs)
```

The tests exercise the pure modules only — `lib/args.mjs` (`parseArgs`,
`defaultShell`, `splitCommand`) and `lib/keymap.mjs` (`keyToSeq`) — and need no
dependencies installed, so they run on a bare Node. (`node --test` with no path
uses Node's default test-file glob, which finds `test/*.test.mjs` and skips
`node_modules`. The bare form is the repo convention — `node --test <dir>` is
not a valid directory search on Node 24.)

## Layout

- `main.cjs` — Electron main: spawns node-pty, wires IPC both ways, handles
  graceful teardown, and (under `--control`) runs the AIUI control server and
  headless-xterm mirror. (`.cjs` = always CommonJS, regardless of `"type"`.)
- `renderer.js` — the xterm.js renderer, loaded by `index.html`.
- `index.html` — links xterm's stylesheet, hosts the `#term` grid.
- `lib/args.mjs` — pure, testable arg + command helpers (no Electron/node-pty).
- `lib/keymap.mjs` — pure key-name → PTY escape-sequence map (`keyToSeq`).
- `test/args.test.mjs` — `node --test` unit tests for `lib/args.mjs`.
- `test/keymap.test.mjs` — `node --test` unit tests for `lib/keymap.mjs`.
