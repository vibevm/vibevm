# MT-05 — AIUI visual testing: render goldens + the visual pass

The agent-facing **visual-verification loop** for the `vibe tree` TUI. It sits
between unit tests (logic, no pixels) and pure human eyeballing, so the agent
catches the class of bug it causes most — spacing, centring, truncation, colour,
box-drawing — and the human is left the *taste* sign-off, not the gross-defect
hunt.

Two tiers, one contract. Both consume the snapshot contract
([PROP-042 §2](../../modules/vibe-cli/PROP-042-aiui-observation.md#snapshot-contract)):

| Tier | Plane | What it catches | Who runs it | Gate |
|---|---|---|---|---|
| **Render goldens** | render (terminal-free) | layout / glyph / truncation regressions, byte-exact | `self-check` (`cargo test`) | must be green |
| **Visual pass** | terminal (real pixels) | colour, font, aesthetic fidelity — what a glyph grid cannot show | agent pre-runs, **human signs off** | one owner OK per change |

Pair this with [MT-04](MT-04-vibeterm-tui-dev.md) (the live-preview loop inside
vibeterm) and [PROP-042](../../modules/vibe-cli/PROP-042-aiui-observation.md)
(the contract).

## The render goldens (automated)

A golden is a **scenario** — `(fixture, cols×rows, key-script)` — re-rendered
headlessly through the real input + render path and diffed against a committed
`.snap.txt`. No terminal, no Electron, no fonts: deterministic and instant. The
seed scenarios live in
[`crates/vibe-cli/src/commands/tree/tui/snapshot.rs`](../../crates/vibe-cli/src/commands/tree/tui/snapshot.rs)
and their committed outputs under
[`…/tui/goldens/`](../../crates/vibe-cli/src/commands/tree/tui/goldens/):

- `base` — the two-row centred footer, the status line, the table.
- `f2-sort-menu` — `F2`: group frames inset, options inset, hint row intact.
- `f3-mode-menu` — `F3`: the flat list, the inert `Tab`.
- `quit-dialog` — `Esc`: `OK`/`Cancel` centred with air.
- `narrow-width` — `56×20`: graceful degradation (documents the footer-clip edge).

### Running the tier

```sh
cargo test -p vibe-cli matches_golden     # all *_matches_golden scenarios
```

A drift fails with the refresh recipe in the message — the fix is mechanical.

### Adding a scenario

1. In `snapshot.rs`, add a test fn over the shared `assert_text_golden` helper:
   ```rust
   #[test]
   fn f5_thing_menu_matches_golden() {
       let got = snapshot_headless(fixture_tree(), 74, 22, "F5", false).expect("render");
       assert_text_golden("f5-thing-menu", &got);
   }
   ```
2. Seed the file: `UPDATE_GOLDENS=1 cargo test -p vibe-cli f5_thing_menu_matches_golden`.
3. **Read the new `goldens/f5-thing-menu.snap.txt`** — a golden is a reviewed
   diff, not a blind capture. Confirm it shows what the scenario claims.
4. Commit the `.snap.txt` alongside the test.

`UPDATE_GOLDENS=1` refreshes **every** golden in one run, so re-seed only the
ones you meant to change and review the `git diff` before committing — an
accidental restyle shows up there.

## The determinism rules (mandatory for goldens)

A golden is only as stable as its inputs. Five knobs, all already enforced by
the render plane ([PROP-042 §1](../../modules/vibe-cli/PROP-042-aiui-observation.md#render-plane), §5):

1. **Fixed size** — the scenario pins `cols×rows`; the render paints exactly that `Rect`.
2. **Pinned theme/tier** — `App::new` defaults to Rosé Pine / Tier 3 and loads
   **no** user settings, so colours never depend on the machine.
3. **Hermetic fixture** — the in-memory `fixture_tree()` (one `g/a` package),
   never the live tree, so the package list is reproducible.
4. **No quiescence needed** — the render plane is synchronous: drive the script,
   paint one frame. (The terminal plane's `/wait` quiescence rule applies only to
   the visual pass below.)
5. **Glyph-stable** — box-drawing comes from the theme's glyph set, not the
   terminal, so `├─└│▾●○` render identically everywhere.

A golden that drifts without a code change is a **bug** (determinism broke), not
a refresh candidate.

## The visual pass (PNG, real pixels)

The render plane cannot show colour fidelity, font, or how a frame *feels*. For
that, render the same scenario through the **terminal plane** to a PNG and look
at it. The PNG is for human/agent judgment — never a byte-golden assert (font /
DPI / GPU make pixel-diffing brittle).

```sh
vibe aiui open --visible --exec "vibe tree --path <fixture-or-dir> -c"
vibe aiui wait --idle-ms 120          # let the first frame settle
vibe aiui snapshot --png /tmp/frame.png
vibe aiui send F2 ; vibe aiui wait    # drive, then settle
vibe aiui snapshot --png /tmp/f2.png
vibe aiui close
```

Then `Read` the PNG (or open it) and check the thing the golden cannot: is the
accent colour right, is the box-drawing crisp, is there air around the dialog.
Flag divergence; the owner signs off aesthetics. This is the `manual-tests`
model (agent pre-runs, human signs off) with PNG evidence — see
[MT-04 §"PNG snapshot (visual pass)"](MT-04-vibeterm-tui-dev.md).

## When to use which

- **Did my layout/code/colour change look right?** → render golden first (fast,
  deterministic). If the golden is unchanged, the layout is unchanged.
- **Is it beautiful / does the font render / is the colour faithful?** → visual
  pass PNG + human eyes. The golden is blind to taste.
- **Did I break the live terminal round-trip?** → the terminal-plane `snapshot`
  (text) vs the render-plane `render` (text) over the same fixture must agree
  modulo terminal quirks — a free cross-check (PROP-042, TERMINAL-AIUI-PLAN §3).
