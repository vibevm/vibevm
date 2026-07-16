# vibe prefs — the settings TUI (PROP-041) visual sign-off

**Purpose.** `vibe prefs ui` is the TUI surface over `vibe-settings` (PROP-040):
a page tree, per-type edit forms, provenance, validation feedback, lint, and
search — built on the `vibe tree` TUI's component library + theme (Шаг 3) and
driven by `vibe.prefs` actions. The automated suite (347 vibe-cli tests,
`self-check` all green) proves the model + the rendering fns; it cannot drive a
real terminal and confirm the surface *reads right* — the page tree aligns, the
form fields edit, the provenance shows the winning layer, the validation
warnings land inline. That is what a human signs off here.

This TUI writes user settings (`~/.vibe/`, the vibe-settings system) — palette/
tier/mode/sort/shape/static-first persist. Back them up or accept the delta.

## Preconditions

- A real interactive terminal (tty), ideally truecolor.
- `cargo build -p vibe-cli` (invoke `./target/debug/vibe`).
- Run from the vibevm repo root (a project context → L1+L2+L3; elsewhere L1 only).

## Steps

1. **Launch + the page tree.**
   ```
   ./target/debug/vibe prefs ui
   ```
   Left pane: the page hierarchy (Appearance / Tree groups → the palette/tier/
   mode/sort/shape/static-first pages) through the same visual language as the
   tree TUI (`│├└─`, `▾`/`▸` fold, theme colours). `↑`/`↓` move, `←`/`→` fold.

2. **Open a page + the edit form (§4).** `Enter` on a page → the right pane
   shows the form: per-type fields (bool→toggle, enum/closed-set→selection,
   int/string→text), each with an `applies` badge + a write-layer selector at
   the bottom. `↑`/`↓` field focus, `Space`/`Enter` toggle/select, typing edits a
   text field, `Tab` cycles the write-layer (L3 project / L1 no-project).

3. **Apply / reset (§4 Configurable lifecycle).** Edit a field → the form is
   modified; `a` applies (writes through `vibe-settings` to the chosen layer;
   a scope-forbidden layer is refused with the reason), `r` resets. Quit +
   relaunch — the change persisted.

4. **Provenance (§5).** `?` on a focused field → an inline block: the resolved
   value + each layer's contribution (default/L1/L2/L3/cli/env), the winning
   layer marked, shadowed layers dimmed. `x` clears the focused write-layer for
   that key → the value falls back to the layer beneath.

5. **Validation + lint (§6).** Type a wrong-type value → an inline warning line
   under the field (gold, rule cited); `apply` is blocked. `c` opens the lint
   modal — `schema::validate` across L1/L2/L3 as a flat warning list; selecting
   a warning jumps to the owning page focused on that field.

6. **Search (§7).** `/` or `F1` → the Search Everywhere window (the same engine
   the tree TUI uses): match by key / display name / description / synonyms;
   selecting opens the owning page focused on the field. Deprecated keys surface
   their `replaced_by`.

7. **Actions + footer (§8).** The footer lists the enabled `vibe.prefs` actions
   for the current context (open/apply/reset/search/lint/layer/provenance/quit),
   keymap-bound. `Esc` pops the modal stack (provenance → page).

8. **Theme switch (the live Theme, §1 `#built-on-tree-tui`).** Change
   `vibe.tree.palette` in the form (or `~/.vibe/settings.toml`) → the whole
   prefs UI re-skins to match the tree TUI.

## Pass

Every step reads as described: the page tree aligns, the form edits + applies +
persists, the provenance shows the winning layer, validation lands inline, the
lint + search work, and the footer lists the live actions. The owner signs the
date below.

## Sign-off

- [ ] Owner visual sign-off (date / initials): ______
