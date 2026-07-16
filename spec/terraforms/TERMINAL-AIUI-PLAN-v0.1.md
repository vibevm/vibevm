# TERMINAL-AIUI-PLAN v0.1 — the vibe terminal + agent eyes on the TUI

**Genre:** campaign / meta-plan (executable, phase-gated). **Status:** DRAFT — a
proposal awaiting owner ratification of §9 decisions; Phase 0 spikes fill the
baselines/predictions. **Contract targets:** new PROP(s) authored in Phase 1
(§8); extends `spec://vibevm/modules/vibe-actions/PROP-039#aiui`.

> **Owner mandate (2026-07-16, verbatim):** «Я хочу разработать систему, которая
> позволит тебе открывать интерфейс в виртуальном терминале и смотреть, что
> получается. И показывать мне для отладки. … 1) сделать собственное приложение
> терминала … electron + xterm.js … пусть открывается через `vibe tree -t` … мы
> не бросим этот терминал, будем развивать … вначале для тестов/отладки, в
> будущем — для обычных пользователей. 2) Реализовать слой AIUI чтобы ты мог
> подключаться к терминалу и смотреть, что там происходит. 3) Управление нашим
> терминалом тоже часть AIUI, в том числе сохранение скриншотов — визуальных PNG
> и символьных снимков экрана. 4) Написать инструкции как делать тестирование и
> несколько основных тестовых сценариев.»

---

## 1. TL;DR

Give the agent **eyes on the `vibe tree` TUI** so it does its own first-pass
visual verification, and build the **vibe terminal** (Electron + xterm.js) as
the real-pixels ground truth and a future user-facing product. Recon
(2026-07-16) reframed the task: observation is **three planes**, not one, and
the cheapest — a terminal-free symbolic render — already half-exists and should
land first as the foundation for a golden-test tier.

- **Render plane (symbolic, terminal-free) — the quick win.** `render::draw`
  already paints a full frame into an off-screen `ratatui::Buffer` with zero TTY
  (`crates/vibe-cli/src/commands/tree/tui/render.rs:23`). Wrap it +
  `input::handle` (`…/input.rs:38`) in a one-shot `vibe tree --snapshot` that
  drives a scripted input sequence and prints the cell grid. Deterministic,
  instant, CI-gateable. This is the substrate for golden tests and my everyday
  "did my layout change look right" check.
- **Terminal plane (visual, real pixels) — the product.** An Electron +
  xterm.js + node-pty desktop app (`vibeterm`) hosts a real PTY, renders it with
  true fonts/colors, and exposes a loopback control API for snapshot (text +
  cells + **PNG**) and input. **`vibe term`** opens vibeterm standalone with a
  shell (`pwsh` on Windows when present) so the owner debugs it directly;
  **`vibe tree -t`** runs the tree in it, `-c` forces the console TUI, and a bare
  `vibe tree` follows the `vibe.tree.launch_mode` setting (**clean-install
  default: console**). The PNG is what I `Read` to actually *see*
  color/glyph/aesthetics; it is also the seed of a real terminal we grow for
  users.
- **Model plane (semantic) — extends PROP-039.** `vibe-actions::aiui` already
  ships `list_actions` + `invoke`; add the missing `state() → ModelView` and a
  `search()` wrapper (PROP-039 §11.3, "designed-for, not built now"). Drive by
  `action://` address, read a serialisable model snapshot — for flow/state
  assertions with no rendering at all.

All three planes speak **one snapshot contract** (§5). The `vibe aiui` CLI is
the agent's entry (I run it via Bash; I `Read` the PNG). MCP tools follow.

---

## 2. The problem & why it matters

Today the agent is blind to the TUI. It writes render code and verifies via unit
tests + hand-rolled buffer dumps (the scratch test used to catch this session's
menu-truncation bug); the human owns all visual/semantic sign-off. That is the
two-process model working as designed — but it makes the human a bottleneck on
exactly the class of bug (spacing, centring, truncation, colour) the agent
causes most and could catch cheapest.

This system **moves first-pass visual verification onto the agent** and leaves
the human the *taste* sign-off. It is a force-multiplier, not a replacement:
the render/terminal planes catch gross defects (jammed frames, clipped hints,
wrong colours, broken glyphs); the owner still rules on whether it is
*beautiful* (the `deliberately beautiful` bar, `spec/design/tui-visual-language.md`).

Cross-reference: `spec://vibevm/modules/vibe-actions/PROP-039#aiui` already
frames the agent-facing surface; this plan builds its **render + terminal**
half, which PROP-039 does not cover (it is the model plane only).

---

## 3. Architecture — three planes, one contract

```
                    ┌───────────────────────────── AIUI ─────────────────────────────┐
                    │                                                                 │
  agent (me) ──►  vibe aiui  ──►  (A) RENDER PLANE   render::draw → ratatui Buffer     │  terminal-free,
     │  (Bash)     CLI / MCP      one-shot, scripted input, cell-grid snapshot         │  deterministic, fast
     │                                                                                 │
     │             ──────────►  (B) TERMINAL PLANE   ┌─ Electron main (node) ────────┐ │  real pixels,
     │  Read(png)                loopback control ──►│  node-pty runs `vibe tree`     │ │  the product
     │                           API (§4)            │  ├─► renderer xterm.js (visual)│ │
     │                                               │  │        └► capturePage → PNG │ │
     │                                               │  └─► @xterm/headless (cells)   │ │
     │                                               └───────────────────────────────┘ │
     │                                                                                 │
     │             ──────────►  (C) MODEL PLANE   vibe-actions::aiui                    │  semantic,
     │                           list_actions / invoke / state()→ModelView / search()   │  no rendering
                    │                                                                 │
                    └─────────────────────────────────────────────────────────────────┘
```

- **(A) and (B) must agree** on the symbolic grid: the ratatui `Buffer` (A) is
  the bytes written to the PTY that xterm.js (B) parses back. Their cell grids
  should be identical modulo terminal quirks — a free cross-check that the PTY
  round-trip is faithful.
- **(A) is the golden substrate** (deterministic, no async, no fonts). **(B) is
  the visual ground truth** (fonts, colour, real emulator) and the user product.
  **(C) is for state/flow**, orthogonal to rendering.

---

## 4. The control protocol (terminal plane)

The Electron **main process** owns everything Node: it spawns the PTY, serves
the control API, and holds a headless xterm parser; the **renderer** owns the
visual xterm.js and yields the PNG.

**Transport:** loopback HTTP+JSON on an ephemeral port, bound to `127.0.0.1`,
guarded by a per-session token. Port+token+pid+size written to a **discovery
file** `~/.vibevm/aiui/<session-id>.json` so `vibe aiui` finds the running app
without a fixed port. (Rejected: OS-native pipes — more portable-code cost;
stdio to Electron — Electron's stdio is unreliable. See §9-D3.)

**Verbs (request/response):**

| Verb | Request | Response |
|---|---|---|
| `POST /input` | `{keys:["F2","Down"], text?:"…"}` | `{ok}` — named keys → PTY escape seqs, `text` typed literally |
| `POST /wait` | `{idle_ms:120, timeout_ms:3000}` | `{stable:true, waited_ms}` — block until PTY output is quiet |
| `GET /snapshot` | `?format=text\|cells\|png &path=…` | text/cells inline (JSON); png written to `path`, returns `{path,w,h}` |
| `GET /state` | — | `{alive, exit_code?, cols, rows, cursor:{x,y}, title}` |
| `POST /resize` | `{cols,rows}` | `{ok}` |
| `POST /close` | — | `{ok}` |

**Snapshot sourcing inside the app:** text/cells come from the **`@xterm/headless`**
`Terminal.buffer.active` in the main process (fed the same PTY bytes as the
renderer — no IPC needed for symbolic capture); the PNG comes from
`renderer.webContents.capturePage()` cropped to the terminal rect. One PTY, two
parsers, two snapshot modalities.

---

## 5. The snapshot contract (shared by all planes)

One schema, three producers (ratatui Buffer, xterm headless, — model plane emits
its own `ModelView`). Golden-friendly + assertion-friendly:

- **`text`** — the glyph grid, one row per line, trailing space trimmed. Exactly
  the dumps in this session's previews. The **golden** form: commit a `.snap.txt`,
  re-render, diff. Human- and diff-readable.
- **`cells`** — JSON, run-length-encoded per row: `[{n, ch, fg, bg, bold,
  underline, inverse}]`. Colours as role tokens where derivable (render plane
  knows the `Theme` role that painted each cell; the terminal plane reports the
  resolved RGB). Enables assertions like "the active group's border cell is
  `accent`". (Rejected: per-cell objects — 10× the bytes for no gain.)
- **`png`** — terminal plane only. Real pixels; the aesthetic ground truth I
  `Read` to *see* the UI.

**Determinism knobs (mandatory for goldens):**
1. **Fixed size** — snapshots pin `cols×rows` (default `80×24`; scenarios may
   set others). The render plane renders that `Rect`; the terminal plane resizes
   the PTY.
2. **Pinned theme/tier** — force a known palette + Tier 3 (via a `--theme` /
   env override) so colours are stable. The render plane's `App::new` already
   defaults to Rosé Pine / Tier 3 with no settings load
   (`…/tui/state.rs:144`).
3. **Hermetic fixture data** — drive against a fixed fixture project
   (`fixtures/…`), never the live tree, so the package list is reproducible.
   The render plane can build an `App` from a fixture `PackageTree` directly
   (the `test_support::app()` pattern).
4. **Quiescence** — the terminal plane snapshots only after `/wait` reports
   stable; the render plane is synchronous and needs no wait.
5. **Pinned font** — the terminal app ships/loads one known font with full
   box-drawing + braille coverage, so PNGs are reproducible across machines.

---

## 6. Launch surfaces — `vibe term`, `vibe tree`, `vibe aiui`

### 6.1 `vibe term` — vibeterm standalone (a real terminal)

`vibe term` opens **vibeterm hosting an interactive shell**, so the owner can use
and eyeball-debug it directly and vibeterm is a general terminal from day one —
not just a `vibe tree` viewer, matching the "eventually for regular users" goal.
vibeterm takes an `--exec <cmd>` (default: the detected shell); `vibe term`
launches it with no `--exec` (⇒ the shell), and `vibe tree -t` (§6.2) is just
vibeterm `--exec "<this-exe> tree -c"`.

**Shell detection (verified on this box, 2026-07-16):**
- **Windows — prefer modern PowerShell 7+ (`pwsh`) over the built-in 5.1.**
  Resolve `pwsh` via PATH → `%ProgramFiles%\PowerShell\7\pwsh.exe` →
  `%LOCALAPPDATA%\Microsoft\WindowsApps\pwsh.exe`; if none, fall back to
  `%SystemRoot%\System32\WindowsPowerShell\v1.0\powershell.exe` (5.1). (This box:
  `pwsh` 7.6.3 present → used; 5.1.26100 is the fallback.) Preferring `pwsh` also
  sidesteps the PS-5.1 UTF-8 round-trip corruption this repo already fights.
- **Linux/macOS — `$SHELL`, falling back to `/bin/sh`.**

### 6.2 `vibe tree` launch modes

`vibe tree` gains a **launch mode**, resolved by precedence:

1. **explicit flag wins** — `-c`/`--console` (the in-terminal console TUI, today's
   behaviour) or `-t`/`--terminal` (open in vibeterm). Mutually exclusive (clap
   `conflicts_with`); added to `TreeArgs` (`crates/vibe-cli/src/cli/inspect.rs:79`;
   `-c`/`-t` are free).
2. **the `vibe.tree.launch_mode` setting** — a new enum field (`console` |
   `vibeterm`) in the `vibe.tree.*` schema (PROP-040/041, edited via `vibe prefs`),
   consulted by a bare `vibe tree`.
3. **clean-install default = `console`.** With nothing configured, `vibe tree`
   runs the console TUI — **never** vibeterm. Don't push a desktop app (Electron,
   node-pty) on a fresh user; the console TUI is the safe, dependency-free default.

`--plain` stays orthogonal (the non-interactive text dump). Wiring: branch at the
launch gate (`…/tree/mod.rs:56`) — `console` → `tui::run` (today); `vibeterm` →
resolve vibeterm (§6.4) + spawn it `--exec "<this-exe> tree --path <path> -c"`
(the `open_prefs` subprocess pattern, `…/tui/input.rs:504`; resolve `<this-exe>`
via `current_exe`). The child carries `-c`, so it renders the console TUI inside
vibeterm — no recursion.

### 6.3 `vibe aiui …` (agent entry)

New `crates/vibe-cli/src/cli/aiui.rs` + `Command::Aiui(AiuiArgs)` +
`commands/aiui.rs` (the `cli/mcp.rs` pattern, verified at `cli.rs:114` /
`main.rs:106`). Sub-verbs:

- `vibe aiui render --path <fixture> --size 80x24 --send "F2 Down Enter" --format text|cells`
  — the **render plane**, one-shot, prints the grid to stdout. No terminal, no
  Electron. *This is the everyday agent tool.*
- `vibe aiui open --exec "vibe tree" [--size …]` — launch the terminal plane,
  wait for the control server, print the session id.
- `vibe aiui send <keys…> [--text …]`, `vibe aiui snapshot --text|--cells|--png <path>`,
  `vibe aiui wait [--idle 120ms]`, `vibe aiui resize WxH`, `vibe aiui close` —
  the terminal-plane control verbs (§4), keyed by session id (or the sole running
  session).
- `vibe aiui state [--path <fixture>]` — the **model plane** `ModelView` dump.

**MCP (later):** the same verbs as `McpTool`s (`crates/vibe-mcp/src/tools.rs:35`
trait, one struct + one line in `default_tools()`), so I can call them natively
instead of via Bash. `McpTool::run` is sync and `aiui::invoke` is async → bridge
with a current-thread `block_on` (§9-D5).

### 6.4 vibeterm resolution

Never PATH (PROP-025 §8): mirror the vvm precedence — `current_exe`→`derive_self`
sibling (`…/vvm/selfloc.rs:22`) → `$VIBEVM_*` override (new `vibe vars` row) →
default under `~/opt`. In dev, resolve to `electron <repo>/packages/…/vibeterm` /
an `npm start`. On Windows a Node/electron launcher spawns via `cmd /c` (the
PROP-025 §4 shim lesson); node-pty (main-process) needs no rebuild (Phase-0 (g)).

---

## 7. Phases & gates

Phase 0 is spikes (no commits); every later phase ends with the floor green
(`tools/self-check.sh`) and a ledger entry. A phase boundary is a safe stop.

| Ph | Title | Deliverable | Gate |
|---|---|---|---|
| **0** | Spikes (no commits) | Prove: (a) `render::draw` into an off-screen Buffer from a new `pub` entrypoint; (b) `input::handle` accepts synthetic `Event`s and drives state; (c) node-pty runs `vibe tree` under Electron and xterm.js renders it; (d) `capturePage()` yields a faithful PNG; (e) `@xterm/headless` cell grid == render-plane grid on a simple frame; (f) a Rust↔Node loopback JSON round-trip. | findings recorded; red spike rewrites the affected §9 decision |
| **1** | Render plane + snapshot contract | `vibe tree --snapshot` / `vibe aiui render`: headless drive-loop (build `App` → apply `--send` script → `render::draw` → emit `text`/`cells`). Pin size/theme/fixture. The `text`/`cells` schema (§5). A `pub fn` headless entrypoint in `tui/mod.rs`. | self-check green; a golden `.snap.txt` for the base frame committed + a test that re-renders and diffs it |
| **2** | vibeterm MVP + `vibe term` | `vibeterm` Electron app: **node-pty in the main process** + xterm.js renderer over IPC, a pinned font, graceful teardown (Phase-0 (g)); hosts an arbitrary `--exec` (default: the detected shell — pwsh-preferred on Windows, §6.1). **`vibe term`** launches it standalone with the shell. Own npm project + a new `self-check.sh` gate step (`npm ci && npm test && npm run build`). Registered as a specspace. | `vibe term` opens a working shell; `vibeterm --exec "vibe tree -c"` renders the TUI; the gate step green |
| **3** | `vibe tree` launch modes + control API | `-c`/`-t` flags + the `vibe.tree.launch_mode` setting (default `console`, §6.2); `-t` (or a bare `vibe tree` when the setting is `vibeterm`) launches vibeterm running `vibe tree -c`. The loopback control server (§4) + discovery file. `vibe aiui open/send/snapshot(text,cells)/wait/state/close`. | `vibe tree -t` opens in vibeterm; the setting flips the bare default; the agent drives it: open → send F2 → wait → snapshot text, asserted |
| **4** | PNG snapshots | `capturePage()` cropped to the terminal rect → `vibe aiui snapshot --png`. Determinism (font, size, theme). | agent `Read`s a PNG of the F2 menu and the quit dialog and confirms the fix visually |
| **5** | Model plane completion | `ModelView` + `state()` + `search()` wrapper in `vibe-actions::aiui` (PROP-039 §11.3); give the tree catalogue real `invoke` bodies or wire `dispatch_by_addr` behind `aiui::invoke`. `vibe aiui state`. | `vibe aiui state` returns a serialised `ModelView`; a flow test drives by `action://` and asserts state |
| **6** | Testing doc + scenarios (goldens) | The test-tier doc (§8) + the §8 scenarios as committed goldens/tests. | scenarios run green in self-check (render plane) + a manual-test index entry for the PNG/visual pass |
| **7** | (future) MCP + user-facing terminal | AIUI verbs as `McpTool`s; the terminal grows real user features (tabs, config, its own settings via the same `vibe-actions`). | out of this campaign's scope; deferred by name |

**Recommended sequencing note:** the owner staged the Electron terminal first;
recon says the **render plane (Phase 1) is the cheaper, higher-leverage first
increment** and de-risks the snapshot contract the terminal plane reuses. This
plan runs render-plane-first; §9-D1 is the owner's to confirm.

---

## 8. Test tier & scenarios (stage 4)

**New tier — "render goldens" + "visual pass".** It sits between unit tests
(logic) and manual tests (human eyes):

- **Render goldens (automated, in self-check):** a scenario is a `(fixture,
  size, input-script, theme)` → committed `text`/`cells` snapshot. The test
  re-renders via the render plane and diffs. Catches spacing/centring/truncation/
  colour regressions with no terminal. Updating a golden is a reviewed diff.
- **Visual pass (agent pre-run, human sign-off):** the same scenarios rendered
  through the terminal plane to PNG; the agent `Read`s them and flags divergence;
  the owner signs off aesthetics. This is the `manual-tests` flow's model
  (agent pre-runs, human signs off) with PNG evidence.

**Doc:** `spec/manual-tests/` gets an "AIUI visual testing" guide: how to run a
render golden, how to add one, how to do a visual pass, the determinism rules.

**Seed scenarios (each = a golden + a visual-pass PNG):**
1. **base frame** — `vibe tree` over the fixture; assert the **two-row centred
   footer** (F-keys row, nav row), the status line, the table.
2. **F2 sort menu** — send `F2`; assert group frames inset from the window,
   options inset from group frames, the **hint row not truncated**, centred.
3. **F2 Tab focus** — `F2 Tab`; assert the **active focus-group is
   accent-framed** and the other dim (cells snapshot on the border colour).
4. **quit dialog** — `Esc`; assert `OK`/`Cancel` centred with air, the body
   centred, `Esc`-cancels semantics via a follow-up `Esc` + `state`.
5. **F3 mode menu** — `F3`; assert the single-group flat list + inert `Tab`.
6. **tabs mode + Shift+arrows** — switch to tabs, `Shift+Right`; assert the tab
   advanced (model plane) and the tab chrome (render plane).
7. **narrow width** — size `56×20`; assert graceful degradation (documents the
   footer-clip edge this session flagged).
8. **`vibe term` shell** — `vibe term` (vibeterm hosting the detected shell);
   assert vibeterm opens and the shell prompt renders — vibeterm as a general
   terminal, not just a tree viewer (Windows uses `pwsh` when present).

---

## 9. Decisions

**Ratified refinements (owner, 2026-07-16):**
- **R1 — the name is `vibeterm`** (one word, no dash) — the app, the package, the
  launcher, the `--exec` host. Applied throughout this plan.
- **R2 — `vibe term` launches vibeterm standalone with a shell** (§6.1), so the
  owner can eyeball-debug it and vibeterm is a general terminal from day one, not
  just a `vibe tree` viewer. Windows prefers `pwsh` (7+) over the built-in 5.1;
  unix uses `$SHELL`.
- **R3 — `vibe tree` launch modes** (§6.2): `-c` console, `-t` vibeterm, bare →
  the `vibe.tree.launch_mode` setting; **clean-install default = `console`** —
  never force the desktop app (Electron/node-pty) on a fresh user.

**Open (proposals — ratify or override):**
- **D1 — Sequencing.** *Proposed:* render plane (Phase 1) before the Electron
  terminal (Phase 2). *Why:* terminal-free, deterministic, days not weeks, and
  it fixes the snapshot contract the terminal reuses; I get symbolic eyes
  immediately. *Rejected:* terminal-first (the owner's staging) — front-loads
  the heaviest, least-deterministic piece before the cheap win. **Owner call.**
- **D2 — Where the terminal app lives.** *Proposed:* start in `research/vibeterm/`
  (the sanctioned scratch home, like `research/ts-demo`), **promote** to a
  first-class specspace `packages/org.vibevm.terminal/vibeterm/v0.1.0/` once it
  stabilises (the fractality precedent). *Rejected:* a top-level `apps/` dir —
  breaks the "first-party code is a versioned package or a `research/` scratch"
  rule.
- **D3 — Control transport.** *Proposed:* loopback HTTP+JSON + discovery file +
  token. *Rejected:* OS-native pipe (portable-code cost); WebSocket-only
  (streaming is a later want, not needed for request/response); Electron stdio
  (unreliable).
- **D4 — Spec home.** *Proposed:* a **new PROP** for the AIUI render+terminal
  observation surface (in `vibe-cli` or a new `vibe-aiui` module) + a PROP for
  the terminal app in its package; **extend** PROP-039 §11.3 only for the model
  plane (`ModelView`/`state`/`search`). *Rejected:* cram it all into PROP-039
  (that doc is the model plane / vibe-actions, not rendering or a desktop app).
- **D5 — Agent interface first.** *Proposed:* `vibe aiui` **CLI first** (I use it
  via Bash + `Read` the PNG), MCP tools in Phase 7. *Rejected:* MCP-first — the
  sync/async bridge + `ServerContext` extension is friction better paid once the
  CLI shape is proven.
- **D6 — Launching a non-Rust app.** *Proposed:* dev-mode `electron <path>` /
  `npm start` first; formalise an `[[app]]` manifest kind (or a shipped prebuilt
  asset resolved directly) later — the `[[binary]]` model is cargo-only
  (`…/vibe-workspace/src/bins.rs`), the `[[mcp_server]]` model (PROP-027) is the
  precedent for declaring + spawning an external process. *Rejected:* forcing
  the Electron app through `[[binary]]` (it has no `crate`, no `cargo build`).

---

## 10. Risks & mitigations

- **node-pty native addon** needs a C++ toolchain / prebuilds — a build/runtime
  concern on fresh machines. *Mitigate:* pin a version with prebuilt binaries;
  document the toolchain in the terminal package's setup doc (dev-runtime-docs
  flow); the render plane (Phase 1) has no native dep, so agent eyes don't block
  on it.
- **PNG non-determinism** (font, DPI, GPU) — pixel-diffing PNGs is brittle.
  *Mitigate:* PNGs are for *human/agent visual judgment*, not byte-golden asserts;
  the byte-goldens live in the render plane (`text`/`cells`).
- **Electron won't fit the pure-cargo gate.** *Mitigate:* its own npm gate step
  in `self-check.sh` (the org.vibevm.ai-native package-gate precedent, steps
  7–10); excluded from the root workspace.
- **Render-plane ≠ terminal-plane divergence** (a ratatui quirk xterm renders
  differently). *Mitigate:* Phase 0(e) proves parity on a simple frame; the two
  cell grids are cross-checked as an invariant.
- **Snapshot drift churn** (goldens change on every intentional restyle).
  *Mitigate:* goldens are small `text` files, updated as a reviewed diff — the
  same discipline as characterization goldens already in the repo.
- **Scope creep into a real terminal product** before the debug tool works.
  *Mitigate:* Phase 7 is explicitly deferred; Phases 1–6 ship the debug/test
  capability first, exactly the owner's "вначале для тестов/отладки" staging.

---

## 11. Repo integration

- **Render/terminal-plane specs:** new PROP(s) (§9-D4), design-doc lore for the
  three-plane model + testing pyramid, cross-linked to PROP-037 (the TUI) and
  PROP-039 (the model plane).
- **Terminal app:** `research/vibeterm/` → `packages/org.vibevm.terminal/…`;
  own `spec/`, `LICENSE.md` (permissive third-party banner — Electron/xterm.js/
  node-pty all MIT), `SPECSPACES.md` registration, boot contract + WAL + CONTINUE.
- **Gating:** a new `self-check.sh` step for the terminal npm project; the render
  plane + model plane are host-crate Rust, gated by the existing steps.
- **Skills/docs:** the AIUI visual-testing guide under `spec/manual-tests/`; a
  `verify`-style flow entry so "does my TUI change look right" routes here.

---

## 12. Acceptance (whole-campaign, sketch — Phase 0 finalises)

```
# render plane (deterministic, in self-check)
vibe tree --snapshot --path fixtures/<fx> --size 80x24 --send "F2" --format text \
  | diff - spec/.../goldens/f2-menu.snap.txt        # exit 0

# terminal plane (agent-driven visual pass)
vibe aiui open --exec "vibe tree --path fixtures/<fx>"
vibe aiui send Esc ; vibe aiui wait --idle 120ms
vibe aiui snapshot --png /tmp/quit.png               # agent Reads /tmp/quit.png, confirms centring/air
vibe aiui close

# model plane
vibe aiui state --path fixtures/<fx> | jq .display_mode   # "all"
```

Campaign done when: the seed scenarios (§8) are committed goldens green in
self-check; the terminal renders `vibe tree` and yields a PNG the agent reads;
`vibe aiui state` returns a `ModelView`; the testing guide is written; the owner
has signed off one visual pass.

---

## 13. Ledger

### Phase 0 — spikes (2026-07-16, no code committed; findings below)

**Verdict: every plane's hardest uncertainty was probed and PASSED — GREEN to
Phase 1.**

- **(A) Render plane — PASS.** A throwaway test drove the real handlers with no
  terminal: synthetic `Event::Key(F2)` → `input::handle` returned `Changed` and
  opened the sort menu → `render::draw` painted into `Buffer::empty(58×18)` → the
  grid dumped clean (menu over base + the two-row footer). Confirms spikes a+b
  and the render-plane cell grid. Phase-1 seam: a `pub` headless entrypoint in
  `tui/mod.rs` (`App`/`render` are module-private).
- **(B) Terminal plane, symbolic — PASS.** `node-pty` (ConPTY) spawned the real
  `target/debug/vibe.exe tree` in an 80×24 PTY; `@xterm/headless` parsed the ANSI
  into a faithful cell grid — the tree (`▾├─└│`, `●/○`), the tabs chrome, and the
  **new two-row centred footer** all rendered correctly in a real emulator.
  node-pty ships **prebuilt binaries + a bundled ConPTY DLL — no C++ toolchain**.
- **(d) Terminal plane, visual — PASS.** Electron (`offscreen: true` +
  `disableHardwareAcceleration()`) `capturePage()` wrote an **842×483 PNG** of a
  Rosé-Pine F2 menu; the agent `Read` it and saw the colours/glyphs/layout —
  proving the "agent produces + reads a screenshot" loop headlessly.
- **(f) Control transport — PASS.** Loopback HTTP+JSON round-trip on
  `127.0.0.1:<ephemeral>` echoed `{keys:[…]}`; the §4 protocol shape holds.
- **(g) Full vertical (owner-requested) — PASS.** The real `vibe tree` PTY output
  rendered through **xterm.js in an Electron renderer** to a **deterministic
  178 KB PNG** (three identical captures) — the actual tree (`├─│└─▾`, `●/○`, the
  tabs chrome, the selected row, the two-row footer), which the agent `Read`.
  **Architecture correction:** node-pty MUST run in the Electron **main** process
  — the renderer has no `worker_threads` ("Failed to construct 'Worker'"); the
  renderer is xterm.js only, fed PTY bytes over IPC (§4 gains a main↔renderer
  hop). **Clean teardown:** guard `webContents.send` with `!win.isDestroyed()`
  **and graceful-quit** (write `Esc`+`Enter` so `vibe` exits itself; never
  `p.kill()` — which throws the ConPTY `AttachConsole failed` in a non-console
  context and pops an Electron error dialog). Both errors eliminated once these
  two rules were applied.
- **(C) Model plane — not spiked** (existing `vibe-actions::aiui` seams confirmed
  by recon; Phase 5).
- **(e) strict render↔terminal parity — deferred to Phase 1** (needs a shared
  fixture; both pipelines proven independently).

**Setup findings (→ the terminal package's dev-runtime doc):**
- Node **v24.18** (≥22.6 ✓), npm 11.16.
- **npm 11 `allow-scripts` blocks native/postinstall by default.** node-pty needs
  `npm rebuild node-pty --foreground-scripts`; electron needs its binary fetched
  via `node node_modules/electron/install.js` (or `npm approve-scripts`). Bake
  into the setup steps.
- node-pty ConPTY throws `AttachConsole failed` on `kill()` in a non-console
  context — cosmetic teardown quirk; mitigate by sending a quit key so `vibe`
  exits before the PTY closes.
- Electron offscreen capture needs `disableHardwareAcceleration()` for reliable
  headless software rendering.

### Phase 1 — render plane + snapshot contract (LANDED 2026-07-16, self-check green)

`vibe aiui render` ships. `tui::snapshot_headless` builds a fresh `App` over a
`PackageTree` (theme defaults, no settings load → deterministic), drives a
`--send` key script through the real `input::handle`, paints one frame into an
off-screen `Buffer`, and projects it to `text` or `cells` (`snapshot`); the
`keyscript` parser refuses the side-effecting `F4`/`F6`. PROP-042 is the
contract. **Golden tier:** a byte-stable base-frame `.snap.txt` diffed by a test
(`UPDATE_GOLDENS=1` refreshes) + a cells-shape test — a layout regression now
fails in self-check with no terminal. Proven live:
`vibe aiui render --send "F2"` renders the sort menu + the two-row footer.

Commit-map: `a3d1341` docs(spec) PROP-042 · `a5e3068` feat(vibe-cli) render
plane · `8cb3bcc` chore(specmap).

### Phase 2 — vibeterm MVP + `vibe term` (LANDED 2026-07-16, self-check green)

vibeterm ships (`research/vibeterm/`): an Electron terminal — node-pty in the
main process, xterm.js renderer over IPC, a **ready-handshake** that spawns the
PTY at the renderer's FitAddon-fitted grid size (aligned, no resize race),
graceful teardown (guarded sends, dispose-before-kill). `--exec` hosts any
command (default: a platform shell); the pure arg/shell logic is `node --test`'d
(17 cases) and gated by a new self-check step. **`vibe term`** (Rust) detects the
shell (pwsh 7+ preferred on Windows, present 7.6.3), locates vibeterm without a
PATH search, resolves its Electron binary via the app's own `path.txt`, and
launches it detached. **Proven live:** vibeterm rendered the real `vibe tree` TUI
perfectly aligned (handshake reported 131×35), the agent `Read` the PNG.
`term.rs` joined conform's `env_roots` (a launcher reading env for the spawn).

Commit-map: `9e94394` docs(spec) §5 · `94f24b4` feat(vibeterm) MVP · `bee50cf`
feat(vibe-cli) vibe term · `676ab53` chore(specmap).

### Decisions status
D1 (sequencing) — render-plane-first **executed** (Phase 1 landed) ahead of the
terminal (Phase 2 next), per the recommendation; owner ratified "whole plan".
D2–D6 proposals stand.

### Commit-map
- `d6a85be` docs(plan): TERMINAL-AIUI-PLAN v0.1 (this plan).
- Phase 0 left **no code commits** (spikes reverted; node probes ran in the
  session scratchpad, outside the repo), per the campaign-plans discipline —
  findings recorded above.
