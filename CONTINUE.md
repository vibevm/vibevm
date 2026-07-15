# CONTINUE.md — cold-resume checkpoint (2026-07-16, the action system + F1 Search Everywhere shipped)

> `spec/WAL.md` is the canonical living state; if this snapshot and the WAL diverge, the WAL wins.

## TL;DR

This session built the **`vibe-actions` action system** and the **F1 Search Everywhere**
feature in the `vibe tree` TUI — the whole arc, autonomously, floor-green throughout:
clean-room research (VSCode + IntelliJ) → a design-doc → **Spec 1 (PROP-039)** → **Spec 2
(PROP-037 §13)** → the `vibe-actions` crate → **F1 Search Everywhere** in the TUI → **five
follow-up increments** → a full **visual redesign** (a Rosé Pine "cosmic violet" theme, a
mode-aware footer, F6 Markdown copy). **~24 commits** (`ba2fe1f` → `2ff4308`),
`bash tools/self-check.sh` GREEN at every phase boundary, `main` ahead of origin (mirrored
this wind-down). **No blocker.** The one open thing is **the owner's own visual sign-off**:
the session had no tty, so the TUI look (F1/F2/F3/F6 + the theme) was verified by tests +
self-check, not by eye — the owner should run it and steer the palette if the violet isn't
right.

## Where work stands

- Branch **`main`**, tree **clean**. This wind-down **mirrors to origin + github**
  (`cargo xtask mirror`, fast-forward-only, per `source-mirrors`).
- `bash tools/self-check.sh` **GREEN** (fmt / clippy `-D warnings` / `vibe check` /
  conform / specmap / whole-workspace tests).
- `vibe tree` TUI now has: **F1 Search Everywhere** (packages by name + every card field +
  all `vibe.tree` actions, a found action runs in place; hybrid "All" tab + per-category
  tabs), **F2 sort menu**, **F3 mode menu**, **F6 Markdown copy** (→ clipboard, footer
  flash), a **Rosé Pine** theme, and a **mode-aware footer**.
- New crate **`vibe-actions`** (gated in `conform.toml`): address / action / registry /
  params / context / invoke / i18n / search / gate / aiui.

## The active next step (candidate — a RESUME is report-then-wait)

1. **Owner visually verifies the TUI** — `./target/debug/vibe tree`, exercise F1/F2/F3/F6,
   judge the Rosé Pine look. The palette is a single file, `tui/theme.rs` (13 colour consts
   + style helpers) — a re-palette (more/less violet, darker base, different semantic
   colours) is a one-file change; the owner said the violet should read "космическое".
2. **Deferred, reserved, non-blocking** (all noted in PROP-037 / the ledger):
   - PNG copy export + the copy-format menu (`↑F6`) — reserved behind the idea of a
     `ComingSoon` modal (§10.4, §12).
   - The F2 sort menu covers **ordering only**; the shape / block-order sub-groups
     (PROP-037 §7.2, §3.3) are not built.
   - `Esc` = quit-with-confirm (§7.4) — today `q` quits, `Esc` is unbound at the base.
   - **Trees in all modes** (sub-tables / tabs are still flat lists — PROP-037 §4, the
     Tree-widget + filter-pipeline abstraction §3) — the big remaining TUI feature.
   - Settings persistence (`~/.vibe/tree`, §9).
   - The **full AIUI frontend**: the `vibe_actions::aiui` core module exists + is tested;
     a real headless drive-and-observe *surface* (in-process API / JSON-RPC / MCP) is
     future (PROP-039 §11.3 — designed-for, prototyped on the TUI, not built).
   - More SE providers: a `StructureProvider` over AI-Native specmap spec/code nodes — the
     provider seam is reserved for it (PROP-039 §10.4, DO16), no engine change needed.

## Non-obvious findings (do not re-learn)

### The action-system architecture
- **`vibe-actions` is frontend-agnostic** — zero rendering deps (the PROP-039 §1 invariant;
  no `ratatui`/`crossterm`/UI toolkit, no consumer-crate dep). This is what makes the AIUI
  (and any future surface) possible.
- **`action://<group>/<name>[?params]`** URI addresses — the behaviour-layer twin of
  `spec://`. Owner-chosen (the URI form, not a dotted FQDN).
- **The SE engine's provider trait is the two-phase `enumerate → match → resolve`** contract
  distilled from IntelliJ; **all providers must emit scores on ONE shared scale** (the
  make-or-break hybrid-list rule) — the engine owns the matcher.
- **The vibe.tree `ActionProvider` enumerates a LIVE `vibe_actions::Registry`**
  (`catalogue.rs::build_registry` — real `Action`s with presentation + capability + a typed
  `Ctx` enablement). The **App dispatches effects by address** (`search/mod.rs::run_action`
  matches the addr string); the action's `invoke` is a **no-op marker** — the Surface (the
  App) applies the effect, because only the App may mutate the model. `gate::legibility` /
  `gate::reachable` / `aiui::list_actions` run over that real Registry (tests in
  `catalogue.rs`).

### Delegation used this session (announced, per the directive)
- **9 read-only subagents** did the clean-room source study (VSCode + IntelliJ action
  systems, project-wide/structural search, i18n) — sources at
  **`C:\Users\olegc\git\snapshot\vscode`** and **`…\idea`** (outside the repo,
  deliberately). Then subagents built the **`vibe-actions` core crate**, the **SE engine**,
  and the **gate + aiui** modules — each to a precise boss-authored spec, self-verified with
  `cargo test -p vibe-actions` + `cargo xtask conform check`.
- The **boss kept**: all architecture + specs, the **TUI integration** (providers, the F1
  modal, the theme/menu/render redesign, F6), review, and the conform/fmt fixes.
- **z.ai / fractality NOT used**: native Claude subagents offload the boss's *context* (the
  scarce resource here), and the crate must pass the repo's *real* `self-check` gates — a
  cold-worktree GLM worker can't run those. This was the delegation-rules verdict, stated
  out loud.

### API / build gotchas (Rust, rat_widget, conform)
- **`BorderType` lives at `ratatui_widgets::borders::BorderType`** (NOT `::block::` — that
  re-export is private). Rounded modal borders use it.
- **rat_widget/rat-ftable styling:** `Cell::style(Option<Style>)`, `Row::style(Option<Style>)`,
  `Table::style(Style)` + `.select_row_style(Some(Style))`, `Tabbed::style(Style)` /
  `.select_style(Style)`.
- **The theme is truecolor** (`Color::Rgb`), **Rosé Pine** — the owner wants a violet/cosmic
  look (iris `#c4a7e7`, purple-tinted base `#191724`). Single source: `tui/theme.rs`.
- **conform discipline:** a **new crate must be classified** `gated`/`exempt` in
  `conform.toml` (vibe-actions is gated); a **gated crate's `thiserror` enums need
  `#[specmark::spec(implements = "spec://…")]`** right after the derive (the full-path
  attribute form is accepted); the **≤600-line file budget** forced extracting the
  fold-aware flatten (`flatten` + `walk`) out of `state.rs` into a new `flatten.rs` cell.
- **F6 copy uses `arboard`** (a new permissive Apache/MIT dep). `copy::markdown()` is
  unit-tested; `copy::copy()` needs a display so it is not exercised headlessly.

### Machine quirks (unchanged, still true)
- Edit `.md`/`.rs` via **Edit/Write only** (PS5.1 corrupts UTF-8); commit via **heredoc**
  (`git commit -F - <<'MSG'`); **no AI-authorship trailers** (Rule 1); the **WAL is too big
  to Read whole** (`Read limit=2` gets the giant `_Updated:` line). **Subagents don't
  `cargo fmt`** → run `cargo fmt --all` after applying their work (fmt is self-check's
  fail-fast first gate). **Never commit on a red floor** — tail `self-check` for "all green".

## Repository map (this session's surface)

- **`crates/vibe-actions/`** — NEW. The frontend-agnostic action system: `address.rs`,
  `action.rs`, `registry.rs`, `params.rs`, `context.rs`, `invoke.rs`, `i18n.rs`,
  `search/` (the SE engine + the `SearchProvider` trait + the matcher), `gate.rs` (the
  legibility + reachable gates), `aiui.rs` (the headless `list_actions`/`invoke`). Gated in
  `conform.toml`.
- **`crates/vibe-cli/src/commands/tree/tui/`** — the TUI:
  - `state.rs` (the `App` — now with `search`/`menu`/`flash` + `set_display_mode`/`set_ordering`),
    `flatten.rs` (**NEW** — the DAG→rows flatten cell), `input.rs` (the keymap: F1/F2/F3/F6,
    mode-aware, the letter keys removed), `render.rs` (status / coloured table / mode-aware
    footer, all themed), `theme.rs` (**NEW** — Rosé Pine), `menu.rs` (**NEW** — the F2/F3
    menus), `modal.rs` (the detail card, themed), `copy.rs` (**NEW** — F6 Markdown export).
  - `search/`: `mod.rs` (`SearchState` + the effect dispatch), `providers.rs` (the three
    providers), `catalogue.rs` (**NEW** — the `vibe.tree` action Registry + `build_registry`),
    `render.rs` (the SE window, themed with coloured provider badges).
- **Specs:** `spec/modules/vibe-actions/PROP-039-action-system.md` (Spec 1, the contract);
  `spec/modules/vibe-cli/PROP-037-tree-tui.md` §13 (Spec 2, revised onto vibe-actions);
  `spec/design/action-system.md` (the design-doc / lore);
  `spec/research/action-systems-vscode-idea.md` (the clean-room study);
  `spec/research/ACTION-SYSTEM-RESEARCH-PLAN-v0.1.md` (the plan + the **running ledger** —
  the live record of this whole arc).

## Decisions in force

- **`action://` URI addresses** (behaviour-layer twin of `spec://`); a **collision-erroring,
  enumerable registry**; **typed context + a pure enablement predicate** (no stringly `when`,
  no UI-thread hazard); **programmatic-invocation-primary + a headless AIUI reference surface**
  (designed-for; prototyped on the TUI; not a separate app yet); the **two-phase provider
  Search Everywhere**; **address-keyed i18n** (English inline default + `{value, original_en}`
  so SE matches the English label under any locale); the **human-legibility floor gate** (every
  action has a non-empty name + description).
- The **vibe.tree action catalogue is a live `vibe_actions::Registry`**; effects dispatched by
  address; the **F-key menus (F2/F3) + F1 action search supersede** the `n`/`x`/`t`/`[ ]`/`F`
  letter shortcuts (removed from the keymap + footer).
- The **TUI theme is Rosé Pine** ("cosmic violet"), single source `tui/theme.rs`.

## Recent commits (last 26, oneline)

```
2ff4308 docs(research): ledger — the design revision (theme, footer, F6)
e388912 feat(vibe-cli): F6 copies the current tree view as Markdown (PROP-037 §10)
cd12031 feat(vibe-cli): a Rosé Pine theme, mode-aware footer, cleaner keymap
78ae34a docs(research): ledger — the five deferred increments all done
171239d feat(vibe-cli): F2 sort / F3 mode selection menus (PROP-037 §7.1/§7.2)
a4e6d2c test(vibe-cli): the vibe.tree catalogue passes the gates + is AIUI-drivable
94f10d8 feat(vibe-cli): highlight matched ranges in Search Everywhere results
551532e feat(vibe-cli): back the vibe.tree action catalogue with a live Registry
dd26677 feat(vibe-actions): the gate + headless AIUI modules (PROP-039 §8.4/§11.3/§12.2)
0c79587 docs(research): ledger — acceptance landed (F1 Search Everywhere)
a69963b test(vibe-cli): headless integration test for F1 Search Everywhere
78b156a feat(vibe-cli): F1 Search Everywhere in the tree TUI (PROP-037 §7.3)
d8ddc7b feat(vibe-actions): the Search Everywhere engine (PROP-039 §10)
bb87abd docs(research): ledger — implementation started, core crate landed
a7f00c7 feat(vibe-actions): the action-system core crate (PROP-039 §§2–8)
d8c0257 docs(vibe-cli): PROP-037 revised onto vibe-actions (Spec 2)
6321607 docs(vibe-actions): PROP-039 — the action-system contract (Spec 1)
4a9a8f9 docs(design): the action-system architecture (lore for PROP-039)
8623e45 docs(research): structural-SE + i18n study addendum (STUDY complete)
14f9b3c docs(research): AIUI headless surface as a founding principle
fe3274a docs(research): running ledger + structural-SE/i18n scope addenda
386ac19 docs(research): VSCode/IntelliJ action-system study
3351168 docs(research): lock action-system mandate + acceptance
ba2fe1f docs(research): action-system study plan (clean-room)
6df6abf docs(wal): session-end checkpoint — AINATIVE-ANALYSIS raid EXECUTED
5f24d40 docs(continue): cold-resume checkpoint — AINATIVE-ANALYSIS raid EXECUTED
```

## Quick-start

```sh
cargo build -p vibe-cli && ./target/debug/vibe tree   # F1 search · F2 sort · F3 mode · F6 copy
bash tools/self-check.sh                               # the floor — expect all green
cargo test -p vibe-actions                             # the action-system core + engine + gate + aiui
# the palette (if the violet needs tuning) is one file:
sed -n '1,40p' crates/vibe-cli/src/commands/tree/tui/theme.rs
```

## Pointer

`spec/WAL.md` (the `_Updated:` line at the top) is the canonical living state and supersedes
this snapshot on any divergence.
