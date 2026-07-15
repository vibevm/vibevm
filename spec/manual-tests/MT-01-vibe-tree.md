# vibe tree — the interactive spec-tree browser

**Purpose.** `vibe tree` (PROP-036) is an interactive terminal UI: it renders
the resolved package tree, lets a human navigate/fold it, opens a detail modal,
and cycles ordering + display modes. The automated suite proves the *model*
(the engine + `--json` validate against the schema, and the flat renderer has
unit tests), but it cannot drive a real terminal and confirm that the tree
*renders and reads right* — the box-drawing aligns, the selection highlights,
the keys respond, the modal overlays cleanly, and colour works on this
terminal. That is what a human proves here. `vibe tree` is **read-only** (it
mutates nothing — no per-user state, no project files), so this test needs no
state isolation; it runs against the vibevm repo itself, which is a rich real
tree.

## Preconditions

- A real interactive terminal (a tty) — this is the whole point; the TUI does
  not launch when stdout is piped or redirected.
- The working-tree binary built from this repo: `cargo build -p vibe-cli`
  (invoke it as `./target/debug/vibe`, never a stale PATH `vibe`).
- Run from the vibevm repo root (its `vibe.lock` + `spec/boot/{STATIC,INDEX}.md`
  are the tree under test). No network, no credentials.

## Setup

```
cargo build -p vibe-cli
cd <vibevm repo root>          # the project whose tree is rendered
```

No scratch/state redirect is needed: `vibe tree` reads `vibe.lock`, the
manifests, and the committed boot artifacts, and writes nothing.

## Steps

1. The machine surface still works (non-tty fallbacks).

   ```
   ./target/debug/vibe tree --json | head -c 200
   ./target/debug/vibe tree --plain | head -20
   ```

   **Expected.** `--json` prints an object opening `{"ok":true,"command":"tree",`
   with `"schema_version":1`. `--plain` prints a static ASCII tree: a header
   line (`project: …`), a `STATIC.md: … bytes / … lines` line, a `columns: load
   T=… C=… S=…` legend, then rows drawn with `│ ├ └` and a `load` column
   (`static`/`dynamic`/`none`) plus three checkbox columns. `redbook` shows
   `static` with `S = x`; `rust-ai-native` (the umbrella) shows `none`. Neither
   command clears the screen or waits for input.

2. Launch the interactive TUI.

   ```
   ./target/debug/vibe tree
   ```

   **Expected.** The terminal switches to a full-screen view. A **status line**
   reads `ordering: topological   mode: all   STATIC.md: <N> bytes / <N> lines
   packages: <N>`. A **footer** shows the keymap hint (`↑/↓ move  ←→ pan  Space
   fold  F fold-all  n order  x mode  t swap  [ ] tabs  Enter detail  q quit`).
   The tree fills the body; the first row is highlighted (reverse/coloured).

3. Navigate with the arrow keys.

   **Expected.** `↓`/`↑` move the highlight one row and the view scrolls to keep
   the selection on screen when you reach the bottom/top. `←`/`→` pan the name
   column horizontally (deep/long ids that ran off the right edge come into
   view; the `load`/checkbox columns stay fixed).

4. Fold and unfold.

   **Expected.** With a node that has children selected, `Space` collapses its
   subtree and the node's indicator flips to `+`; `Space` again expands it back
   to `-`. `F` folds the whole tree to its roots (all `+`); `F` again unfolds
   everything. A package reached twice (a diamond) shows once expanded and once
   as a `(*)` leaf.

5. Open the detail modal.

   ```
   (press Enter on a selected package row)
   ```

   **Expected.** A bordered popup overlays the tree (the cells beneath are
   cleared, not bled through) showing the package's detail **vertically**:
   name, group, version, kind, load type, transitive (+ why), condition,
   in-STATIC.md, source, content hash, dependencies, boot file. `Esc` closes it
   and returns to the tree at the same selection. While the modal is open, other
   keys are swallowed (do not move the tree).

6. Toggle the ordering.

   ```
   (press n)
   ```

   **Expected.** The status line's `ordering:` flips to `alphabetical` and the
   siblings re-sort by `group/name` (the tree structure is preserved — a parent
   still precedes its children). `n` again returns to `topological`.

7. Cycle the display modes.

   ```
   (press x, then x, then x)
   ```

   **Expected.** `x` → **sub-tables**: a flat list under bold subheaders
   `static dependencies`, `dynamic dependencies`, `no-boot` (`mode: sub-tables`
   in the status line). `x` again → **tabs**: a tab bar `Static | Dynamic |
   No-boot` with one group's flat list below (`mode: tabs`). `x` again → back to
   `all` (the tree). In every mode the `load`/T/C/S columns stay meaningful.

8. Swap priority and switch tabs.

   ```
   (in sub-tables or tabs mode: press t; in tabs mode: press Tab, ], [)
   ```

   **Expected.** `t` swaps the section/tab order so `dynamic` comes before
   `static`; `t` again restores `static`-first. In tabs mode, `Tab` and `]`
   advance to the next tab (wrapping), `[` goes back; the shown flat list
   changes to the active group.

9. Quit.

   ```
   (press q)
   ```

   **Expected.** The TUI exits, the alternate screen is torn down, and the
   normal terminal (with your scrollback intact) is restored — no leftover
   raw-mode, no garbled prompt, exit code 0.

## Teardown

None — `vibe tree` wrote nothing. (If a crash ever leaves the terminal in raw
mode, `reset` restores it.)

## What to file if it fails

- The failing step number; what you saw beside its **Expected**.
- A screenshot or a copy of the mis-rendered frame (the exact glyphs/colours).
- `./target/debug/vibe tree --json` output (the model the TUI renders) and
  `./target/debug/vibe tree --plain` (the same tree, copy-pasteable).
- Platform, terminal emulator + `$TERM`, `./target/debug/vibe --version`, shell.
