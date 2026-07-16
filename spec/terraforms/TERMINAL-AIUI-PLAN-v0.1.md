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
  xterm.js + node-pty desktop app (`vibe-term`) hosts the real `vibe tree` in a
  PTY, renders it with true fonts/colors, and exposes a loopback control API for
  snapshot (text + cells + **PNG**) and input. `vibe tree -t` launches it. The
  PNG is what I `Read` to actually *see* color/glyph/aesthetics; it is also the
  seed of a real terminal we grow for users.
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

## 6. `vibe tree -t` and the `vibe aiui` surface

**`vibe tree -t` (UX sugar):** add `#[arg(short='t', long)] terminal: bool` to
`TreeArgs` (`crates/vibe-cli/src/cli/inspect.rs:79`; `-t` is free). Branch at the
launch gate (`…/tree/mod.rs:56`, before `tui::run`): resolve the terminal app
(§ resolution below), spawn it with `--exec "<this-exe> tree --path <path>"`
(the `open_prefs` subprocess pattern, `…/tui/input.rs:504` — resolve `vibe` via
`current_exe`), and hand over. No recursion: the child runs `vibe tree` **without**
`-t`.

**`vibe aiui …` (agent entry):** new `crates/vibe-cli/src/cli/aiui.rs` +
`Command::Aiui(AiuiArgs)` + `commands/aiui.rs` (the `cli/mcp.rs` pattern,
verified at `cli.rs:114` / `main.rs:106`). Sub-verbs:

- `vibe aiui render --path <fixture> --size 80x24 --send "F2 Down Enter" --format text|cells`
  — the **render plane**, one-shot, prints the grid to stdout. No terminal, no
  Electron. *This is the everyday agent tool.*
- `vibe aiui open --exec "vibe tree" [--size …]` — launch the terminal plane,
  wait for the control server, print the session id.
- `vibe aiui send <keys…> [--text …]`, `vibe aiui snapshot --text|--cells|--png <path>`,
  `vibe aiui wait [--idle 120ms]`, `vibe aiui resize WxH`, `vibe aiui close` —
  the terminal-plane control verbs (§4), keyed by session id (or the sole
  running session).
- `vibe aiui state [--path <fixture>]` — the **model plane** `ModelView` dump.

**MCP (later):** the same verbs as `McpTool`s (`crates/vibe-mcp/src/tools.rs:35`
trait, one struct + one line in `default_tools()`), so I can call them natively
instead of via Bash. Note `McpTool::run` is sync and `aiui::invoke` is async →
bridge with a current-thread `block_on` (§9-D5).

**App resolution** (never PATH — PROP-025 §8): mirror the vvm precedence —
`current_exe`→`derive_self` sibling (`…/vvm/selfloc.rs:22`) → `$VIBEVM_*`
override (new `vibe vars` row) → default under `~/opt`. In dev, resolve to
`electron <repo>/packages/…/vibe-term` / an `npm start`.

---

## 7. Phases & gates

Phase 0 is spikes (no commits); every later phase ends with the floor green
(`tools/self-check.sh`) and a ledger entry. A phase boundary is a safe stop.

| Ph | Title | Deliverable | Gate |
|---|---|---|---|
| **0** | Spikes (no commits) | Prove: (a) `render::draw` into an off-screen Buffer from a new `pub` entrypoint; (b) `input::handle` accepts synthetic `Event`s and drives state; (c) node-pty runs `vibe tree` under Electron and xterm.js renders it; (d) `capturePage()` yields a faithful PNG; (e) `@xterm/headless` cell grid == render-plane grid on a simple frame; (f) a Rust↔Node loopback JSON round-trip. | findings recorded; red spike rewrites the affected §9 decision |
| **1** | Render plane + snapshot contract | `vibe tree --snapshot` / `vibe aiui render`: headless drive-loop (build `App` → apply `--send` script → `render::draw` → emit `text`/`cells`). Pin size/theme/fixture. The `text`/`cells` schema (§5). A `pub fn` headless entrypoint in `tui/mod.rs`. | self-check green; a golden `.snap.txt` for the base frame committed + a test that re-renders and diffs it |
| **2** | The vibe terminal (MVP) | `vibe-term` Electron app: node-pty + renderer xterm.js + a pinned font, runs an arbitrary `--exec`. Its own npm project + a new `self-check.sh` gate step (`npm ci && npm test && npm run build`). Registered as a specspace. | app launches `vibe tree`, renders it; its gate step green |
| **3** | `vibe tree -t` + terminal control API | `-t` launches `vibe-term` running `vibe tree`. The loopback control server (§4) + discovery file. `vibe aiui open/send/snapshot(text,cells)/wait/state/close`. | agent drives the running terminal: open → send F2 → wait → snapshot text, asserted |
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

---

## 9. Decisions for the owner (proposals — ratify or override)

- **D1 — Sequencing.** *Proposed:* render plane (Phase 1) before the Electron
  terminal (Phase 2). *Why:* terminal-free, deterministic, days not weeks, and
  it fixes the snapshot contract the terminal reuses; I get symbolic eyes
  immediately. *Rejected:* terminal-first (the owner's staging) — front-loads
  the heaviest, least-deterministic piece before the cheap win. **Owner call.**
- **D2 — Where the terminal app lives.** *Proposed:* start in `research/vibe-term/`
  (the sanctioned scratch home, like `research/ts-demo`), **promote** to a
  first-class specspace `packages/org.vibevm.terminal/vibe-term/v0.1.0/` once it
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
- **Terminal app:** `research/vibe-term/` → `packages/org.vibevm.terminal/…`;
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

## 13. Ledger (filled during execution)

_Phase 0 → …: hashes, subjects, what each confirmed/falsified. Empty until
execution begins._
