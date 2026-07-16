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

## 4. The `vibe aiui` surface {#aiui-cli}

REQ. `vibe aiui` is the agent-facing command family. Its render-plane verb:

```
vibe aiui render [--path <dir>] [--size <COLSxROWS>] [--send "<script>"] [--format text|cells]
```

builds the `vibe tree` model at `--path` (the same resolver `vibe tree` uses),
drives `--send` (§3) at `--size` (default `80x24`), and prints the `--format`
snapshot (§2, default `text`) to stdout. It is read-only and non-interactive:
it never enters the TUI, spawns a terminal, or touches user state. Additional
`vibe aiui` verbs (terminal-plane `open`/`send`/`snapshot`/`wait`/`close`,
model-plane `state`) land in later campaign phases and are governed here as they
arrive.

## 5. Never {#never}

- Never load user settings into a snapshot render — determinism dies and goldens
  churn. Defaults only.
- Never execute a side-effecting key (`F4`/`F6`) in the render plane.
- Never let a snapshot format invent or drop content — `text` is every glyph in
  grid order; `cells` is every run with its true style.
- Never enter the interactive TUI from `vibe aiui` — it is headless by contract.
