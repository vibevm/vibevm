# MT-04 — Debugging the `vibe tree` TUI inside vibeterm (Electron)

A **live-preview loop** for the `vibe tree` TUI hosted in vibeterm (the
Electron terminal), so a Rust change is seen without relaunching Electron or
reconnecting the CDP/AIUI plane every time.

## The rule (read this first)

| What changed | What to restart |
|---|---|
| **Rust** — `vibe tree` / `vibe-cli` (render, theme, layout, actions…) | **Only the PTY child**: `pty-stop` → rebuild → `pty-start`. Electron, the renderer, the CDP endpoint, and the discovery file all stay live. |
| **Electron / xterm.js** — `apps/vibeterm/main.cjs`, `renderer.js`, `index.html`, `lib/*` | **Full restart**: close the session (`vibe aiui close`) and reopen (`vibe aiui open`). These load once at window start. |

A build of `vibe.exe` fails with `Access denied` while the PTY child holds the
binary open — that is exactly why the PTY must be stopped first, then rebuilt,
then started. `pty-restart`-as-one-action is deliberately not provided: there
has to be room for the rebuild between the stop and the start.

## One-time bring-up

```sh
vibe aiui open --visible        # vibeterm + control plane + CDP endpoint
```

This opens a visible window running `vibe tree` over the current directory, with
the loopback control server and the Chrome DevTools Protocol endpoint. The
session writes `~/.vibevm/aiui/latest.json` (`port`, `token`, `cdpPort`, `pid`).
Leave it open for the whole dev session.

## Live preview — Rust change

```sh
# 1. edit the TUI, e.g.
$EDITOR crates/vibe-cli/src/commands/tree/tui/render.rs

# 2. free the binary (kills the vibe-tree PTY child only — Electron stays)
vibe aiui pty-stop

# 3. rebuild
cargo build -p vibe-cli

# 4. respawn the fresh binary in the SAME window (same grid size)
vibe aiui pty-start

# 5. look at the result — no reconnect, no relaunch
vibe aiui snapshot            # symbolic text grid
vibe aiui inspect "JSON.stringify({ cols: term.cols, rows: term.rows })"
```

The whole loop is seconds; the CDP `inspect` and the discovery file never
budge.

## Diagnosing through CDP (no screenshot OCR)

`vibe aiui inspect "<js>"` evaluates a JavaScript expression in the live
renderer page over the DevTools Protocol — read the **real** runtime state
(xterm grid `cols`, cell metrics, per-cell foreground colour/mode, DOM layout)
straight from the source, never from a PNG. This is how every layout/colour bug
in this plane was localised.

Useful probes:

```sh
# grid + cell metrics + scrollbar box
vibe aiui inspect "JSON.stringify({cols:term.cols,rows:term.rows,cellW:term._core._renderService.dimensions.css.cell.width,vpW:Math.round(document.querySelector('.xterm-viewport').getBoundingClientRect().width)})"

# per-cell colour: which flag cells are RGB (truecolor) vs Indexed, and their fg
vibe aiui inspect "(()=>{const b=term.buffer.active;let f={};for(let r=0;r<b.length;r++){const l=b.getLine(r);if(!l)continue;for(let i=0;i<l.length;i++){const c=l.getCell(i);const ch=c.getChars();if(ch==='●'||ch==='○'){const k=ch+' m='+c.getFgColorMode()+' fg=0x'+c.getFgColor().toString(16);f[k]=(f[k]||0)+1;}}}return JSON.stringify(f);})()"
```

`getFgColorMode()` returns `50331648` (`0x03____`) for RGB truecolour — the
signal that `vibe tree` is on its Tier-3 path and emitting exact `Rosé Pine`
RGB (gated on the PTY env `COLORTERM=truecolor` that vibeterm sets).

## Full restart — Electron / xterm.js change

```sh
vibe aiui close                 # tears down the window + control server
# edit apps/vibeterm/{main.cjs,renderer.js,index.html,lib/*}
vibe aiui open --visible        # fresh window picks up the JS change
```

## PNG snapshot (visual pass)

```sh
vibe aiui snapshot --png out.png   # (when wired) or POST /capture directly
```

The PNG is for **visual judgement** (aesthetics, glyph shape); byte-exact
assertions live in the render-plane goldens (`crates/vibe-cli/src/commands/tree/tui/goldens/`), not in pixel diffs.

## Scrollbar policy — one knob, three positions

Everything learned about rendering with and without the scrollbar lives behind a
single switch, flippable live (no Electron restart):

```sh
vibe aiui scrollbar auto    # default: hidden for a full-screen TUI (alt-screen), shown for a shell
vibe aiui scrollbar on      # always show — grid reserves the bar's width so it never overlaps
vibe aiui scrollbar off     # never show — grid owns the full content width
```

Mechanism (in `renderer.js`): `window.setScrollbarMode(m)` sets the policy;
`scrollbarShown()` resolves it to a boolean; `proposeDimensions` is wrapped so
the grid reserves the scrollbar width **only when the bar is shown** (otherwise
the grid reclaims the full content width — no dead gutter); the body carries
`.scrollbar-hidden` so CSS suppresses xterm's scrollback viewport. The refit on
a policy/buffer flip is delayed 400 ms so it does not race the program's first
alt-screen frame.

Verify it the way real apps flip (all without restarting Electron):

```sh
vibe aiui open --visible                 # window up; runs `vibe tree` (alt-screen)
vibe aiui scrollbar auto && vibe aiui inspect 'JSON.stringify({mode:window.getScrollbarMode(),cols:term.cols,hidden:document.body.classList.contains("scrollbar-hidden")})'
# → {"mode":"auto","cols":114,"hidden":true}        # bar hidden, full width
vibe aiui scrollbar on && vibe aiui inspect '...'
# → {"mode":"on","cols":112,"hidden":false}         # bar shown, grid reserves width — no overlap
vibe aiui scrollbar off && vibe aiui inspect '...'
# → {"mode":"off","cols":114,"hidden":true}         # bar hidden, full width
```

The `cols` difference between `on` and `off` is exactly the scrollbar width —
the proof that the grid neither overlaps the bar nor leaves a gutter.

