# CONTINUE.md — cold-resume checkpoint (2026-07-15, end of day)

> `spec/WAL.md` is the canonical living state; if this snapshot and the WAL diverge, the WAL wins.

## ⏭️ FIRST THING NEXT SESSION (owner directive, 2026-07-15)

**Significantly improve the `vibe tree` TUI spec + plan BEFORE writing any code.**
The owner said, verbatim, on winding down: *"там нужно сильно улучшить
спецификацию и план, напомни мне про это при восстановлении сессии."* So at
resume, surface this first: **[PROP-037](spec/modules/vibe-cli/PROP-037-tree-tui.md)**
(the TUI application contract) and **[TREE-TUI-PLAN-v0.1](spec/terraforms/TREE-TUI-PLAN-v0.1.md)**
(the campaign recipe) are a **first draft** — they need a hard revision pass with
the owner (tighten the architecture, the granular REQs, the phasing) before
Phase 0 starts. Do **not** jump into Phase 0 / coding; open with the report and
the improve-the-spec agenda, then take the owner's steer (this is a RESUME →
report-then-wait boundary anyway).

## TL;DR

Two things happened this session. **(1) The PACKAGE-TREE campaign is DONE and
shipped** — `vibe tree`, the algorithmic spec-tree analyzer with an interactive
ratatui TUI, landed in vibevm core (all 5 phases + close-out, floor green,
mirrored). A follow-up **resize bug-fix** shipped too (the TUI now repaints on
terminal resize + paints the status line on the first frame). **(2) A NEW, MUCH
BIGGER campaign — TREE-TUI — was kicked off**: the owner commissioned turning the
TUI into a real application (MVC layers, a reusable component library, F-key
menus, a modal stack, settings persistence, a copy system, trees in all modes).
The **spec (PROP-037) + plan (TREE-TUI-PLAN)** are written and committed but are a
first draft to be improved next session (see the reminder above). **No TREE-TUI
code exists yet** — Phase 0 has not started.

## Where work stands

- Branch **`main`**, tree **clean**, **2 commits ahead of `origin`** (`6473ecb`
  PROP-037 + `1f30037` the plan) — this wind-down mirrors them.
- `bash tools/self-check.sh` **GREEN**. `vibe tree` (analyzer + TUI) fully works:
  `./target/debug/vibe tree` (TUI), `--json` (schema-valid), `--plain`.
- The PACKAGE-TREE campaign: `spec/terraforms/PACKAGE-TREE-PLAN-v0.1.md` (status
  **EXECUTED**). The TREE-TUI campaign: `spec/terraforms/TREE-TUI-PLAN-v0.1.md`
  (status **PLANNED**, Phase 0 not begun) + `spec/modules/vibe-cli/PROP-037-tree-tui.md`.

## The active next step

1. **Improve PROP-037 + TREE-TUI-PLAN with the owner** (the reminder above) — the
   agenda: sharpen the four-layer MVC boundaries (RP1), the `ui::` component API
   (RP2), the granularity/addressability of the REQs, and the phasing.
2. Then Phase 0 spikes (rat-widget component coverage, the Tree-widget + filter
   pipeline, `arboard` clipboard, `~/.vibe/tree` JSON, the modal stack).
3. Then Phase 1 (the four-layer foundation refactor of the existing TUI).

## Non-obvious findings (do not re-learn)

- **The TREE-TUI architecture (owner-approved direction):** four layers — vibevm
  boundary (`PackageTree` only) / Model (data + UI state) / View (a
  rat-widget-idiomatic component library + a separate `Theme`) / Controller (a
  mode-aware keymap registry + a modal stack). Styling must not leak into logic;
  vibevm logic must not leak into the app (MVC).
- **The load-bearing abstraction:** the **Tree is a widget fed by a configurable
  filter/shape pipeline** — the three tree shapes and the three modes are
  pipeline configs, not bespoke renderers (PROP-037 §3). Default shape =
  members-as-roots + full subtrees; shape/sort are F2 settings, persisted.
- **Components:** wrap `rat-widget` behind our `ui::` API + `Theme`; extend in its
  idiom where it lacks; `ratatui-core` only as a last resort.
- **Standard `ComingSoon` modal:** one reusable placeholder for every unbuilt
  feature (PNG export, F1 Search Everywhere, …) — lets all F-keys be wired early.
- **Keymap:** F-keys for commands (F1 search, F2 sort, F3 mode, F6 copy / ↑F6
  copy-settings), `Esc` = quit-with-confirm; the footer writes `Shift` as `↑`.
  (The earlier "don't touch keys" is retired by this redesign.)
- **The resize fix (shipped):** rat-salsa repaints only on `Control::Changed`; the
  handler must return `Changed` on `Event::Resize` (the startup alt-screen resize
  was the missing-first-frame cause too). Fixed in `tui/input.rs`.
- **Delegation state (owner rulings this session):** the "native sub-agent tool ≠
  the cheap GLM slot" loophole is named in the directive (`#route`,
  `#worker-choice`) + the fractality ledger; **opencode < fractality**; and **no
  fractality this session** (transient z.ai 529s) → **Opus[1m] subagents** for
  delegation. That last ruling is session-scoped — a fresh session may re-evaluate
  fractality if z.ai is healthy.
- **Machine quirks:** edit `.md` via Edit/Write only (PS5.1 corrupts UTF-8);
  heredoc commits; `self-check.sh` via Git Bash; **no AI-authorship trailers**
  (Rule 1); the WAL is too big to Read whole — `Read limit=2` gets the giant
  `_Updated:` summary line; a `vibe install` reinstall produces CRLF noise across
  vibedeps — stage the meaningful files then `git -c core.autocrlf=false checkout
  -- .`.

## Repository map (vibe tree)

- `crates/vibe-cli/src/commands/tree/` — the shipped analyzer + TUI. `build.rs`
  (the `PackageTree` engine — the vibevm boundary the app renders), `model.rs`,
  `artifacts.rs`, `diagnostics.rs`, `plain.rs`, and `tui/` (`mod.rs`/`state.rs`/
  `render.rs`/`input.rs`/`modal.rs`/`modes.rs` — the current rat-salsa TUI, to be
  refactored onto MVC in TREE-TUI Phase 1; `modes.rs`'s flat lists become trees).
- `crates/vibe-cli/resources/package-tree.schema.v1.json` — the `--json` schema.
- `spec/modules/vibe-cli/PROP-036-package-tree.md` — the analyzer contract.
- `spec/modules/vibe-cli/PROP-037-tree-tui.md` — the TUI application contract (the
  granular addressable REQs; **improve next session**).
- `spec/manual-tests/MT-01-vibe-tree.md` — the analyzer/TUI manual test.
- `spec/terraforms/PACKAGE-TREE-PLAN-v0.1.md` (EXECUTED) · `TREE-TUI-PLAN-v0.1.md`
  (PLANNED).

## Decisions in force

- `vibe tree` = vibevm core (a `vibe-cli` subcommand, canonical parsers); the
  future `tool:org.vibevm.core/package-tree` is for a runtime skill + GUI, not this.
- Load type = effective, read from the committed artifacts.
- TREE-TUI: four-layer MVC; Tree-widget + filter pipeline; wrap-rat-widget
  components; `ComingSoon` for stubs; F-keys; English-only (i18n indirection only);
  AI-Native Rust + granular addressable REQs.

## Recent commits (last 14)

```
1f30037 docs(plan): TREE-TUI campaign — the vibe tree TUI application
6473ecb docs(spec): PROP-037 vibe tree TUI application contract
80944ee fix(vibe-cli): vibe tree — repaint on terminal resize (fixes the stale first frame)
ee92ad6 docs(continue): cold-resume checkpoint — vibe tree shipped
d8822f9 docs(wal): session-end checkpoint — vibe tree shipped
98ad6d6 docs(plan): PACKAGE-TREE campaign EXECUTED — close report + scorecard
007c030 build(host): materialize the delegation directive edits into vibedeps + lock
f724798 test(vibe-cli): MT-01 manual test for the vibe tree TUI
a0e0b15 feat(vibe-cli): vibe tree — @spec widening + root-drift diagnostic (PROP-036 §2.9-§2.10)
5b59f82 docs(delegation): record the opencode-vs-fractality owner ruling
32f4d49 docs(plan): Phase 3 landed — ordering + display modes in the ledger
4e3d269 feat(vibe-cli): vibe tree — ordering + display modes (PROP-036 §2.11)
e732ac0 docs(plan): Phase 2 landed — ledger + the reverted vibe.toml anomaly
cee039d feat(vibe-cli): vibe tree — the interactive TUI (PROP-036 §2.11)
```

## Quick-start

```sh
cargo build -p vibe-cli                 # ./target/debug/vibe
./target/debug/vibe tree                # the interactive TUI (resize now repaints)
bash tools/self-check.sh                # the floor — expect all green
# next session, read + improve these before any code:
sed -n '1,60p' spec/modules/vibe-cli/PROP-037-tree-tui.md
sed -n '1,40p' spec/terraforms/TREE-TUI-PLAN-v0.1.md
```

## Pointer

`spec/WAL.md` (the `_Updated:` line at the top) is the canonical living state and supersedes this snapshot on any divergence.
