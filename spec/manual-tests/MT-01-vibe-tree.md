# vibe tree — the interactive spec-tree browser

##purpose **Purpose.** `vibe tree` (PROP-036) is an interactive terminal UI: it renders
the resolved package tree, lets a human navigate/fold it, opens a detail modal,
and cycles ordering + display modes. The automated suite proves the *model*
(the engine + `--json` validate against the schema, and the flat renderer has
unit tests), but it cannot drive a real terminal and confirm that the tree
*renders and reads right* — the box-drawing aligns, the selection highlights,
the keys respond, the modal overlays cleanly, and colour works on this
terminal. That is what a human proves here. `vibe tree` is **read-only** (it
mutates nothing — no per-user state, no project files), so this test needs no
state isolation; it runs against the vibevm repo itself, which is a rich real
tree. @impl/done

## Preconditions

- ##PRE-TTY A real interactive terminal (a tty) — this is the whole point; the TUI does
  not launch when stdout is piped or redirected. @impl/done
- ##PRE-BINARY The working-tree binary built from this repo: `cargo build -p vibe-cli`
  (invoke it as `./target/debug/vibe`, never a stale PATH `vibe`). @impl/done
- ##PRE-REPO-ROOT Run from the vibevm repo root (its `vibe.lock` + `spec/boot/{STATIC,INDEX}.md`
  are the tree under test). No network, no credentials. @impl/done

## Setup

```
cargo build -p vibe-cli
cd <vibevm repo root>          # the project whose tree is rendered
```

##setup-no-isolation No scratch/state redirect is needed: `vibe tree` reads `vibe.lock`, the
manifests, and the committed boot artifacts, and writes nothing. @impl/done

## Steps

1. ##STEP-1-MACHINE-SURFACE The machine surface still works (non-tty fallbacks). @impl/done

   ```
   ./target/debug/vibe tree --json | head -c 200
   ./target/debug/vibe tree --plain | head -20
   ```

   ##EXP-1-MACHINE-SURFACE **Expected.** `--json` prints an object opening `{"ok":true,"command":"tree",`
   with `"schema_version":1`. `--plain` prints a static ASCII tree: a header
   line (`project: …`), a `STATIC.md: … bytes / … lines` line, a `columns: load
   T=… C=… S=…` legend, then rows drawn with `│ ├ └` and a `load` column
   (`static`/`dynamic`/`none`) plus three checkbox columns. `redbook` shows
   `static` with `S = x`; `rust-ai-native` (the umbrella) shows `none`. Neither
   command clears the screen or waits for input. @impl/done

2. ##STEP-2-LAUNCH-TUI Launch the interactive TUI. @impl/done

   ```
   ./target/debug/vibe tree
   ```

   ##EXP-2-LAUNCH-TUI **Expected.** The terminal switches to a full-screen view. A **status line**
   reads `ordering: topological   mode: all   STATIC.md: <N> bytes / <N> lines
   packages: <N>`. A **footer** shows the keymap hint (`↑/↓ move  ←→ pan  Space
   fold  F fold-all  n order  x mode  t swap  [ ] tabs  Enter detail  q quit`).
   The tree fills the body; the first row is highlighted (reverse/coloured). @impl/done

3. ##STEP-3-NAVIGATE Navigate with the arrow keys. @impl/done

   ##EXP-3-NAVIGATE **Expected.** `↓`/`↑` move the highlight one row and the view scrolls to keep
   the selection on screen when you reach the bottom/top. `←`/`→` pan the name
   column horizontally (deep/long ids that ran off the right edge come into
   view; the `load`/checkbox columns stay fixed). @impl/done

4. ##STEP-4-FOLD Fold and unfold. @impl/done

   ##EXP-4-FOLD **Expected.** With a node that has children selected, `Space` collapses its
   subtree and the node's indicator flips to `+`; `Space` again expands it back
   to `-`. `F` folds the whole tree to its roots (all `+`); `F` again unfolds
   everything. A package reached twice (a diamond) shows once expanded and once
   as a `(*)` leaf. @impl/done

5. ##STEP-5-DETAIL-MODAL Open the detail modal. @impl/done

   ```
   (press Enter on a selected package row)
   ```

   ##EXP-5-DETAIL-MODAL **Expected.** A bordered popup overlays the tree (the cells beneath are
   cleared, not bled through) showing the package's detail **vertically**:
   name, group, version, kind, load type, transitive (+ why), condition,
   in-STATIC.md, source, content hash, dependencies, boot file. `Esc` closes it
   and returns to the tree at the same selection. While the modal is open, other
   keys are swallowed (do not move the tree). @impl/done

6. ##STEP-6-ORDERING Toggle the ordering. @impl/done

   ```
   (press n)
   ```

   ##EXP-6-ORDERING **Expected.** The status line's `ordering:` flips to `alphabetical` and the
   siblings re-sort by `group/name` (the tree structure is preserved — a parent
   still precedes its children). `n` again returns to `topological`. @impl/done

7. ##STEP-7-MODES Cycle the display modes. @impl/done

   ```
   (press x, then x, then x)
   ```

   ##EXP-7-MODES **Expected.** `x` → **sub-tables**: a flat list under bold subheaders
   `static dependencies`, `dynamic dependencies`, `no-boot` (`mode: sub-tables`
   in the status line). `x` again → **tabs**: a tab bar `Static | Dynamic |
   No-boot` with one group's flat list below (`mode: tabs`). `x` again → back to
   `all` (the tree). In every mode the `load`/T/C/S columns stay meaningful. @impl/done

8. ##STEP-8-SWAP-TABS Swap priority and switch tabs. @impl/done

   ```
   (in sub-tables or tabs mode: press t; in tabs mode: press Tab, ], [)
   ```

   ##EXP-8-SWAP-TABS **Expected.** `t` swaps the section/tab order so `dynamic` comes before
   `static`; `t` again restores `static`-first. In tabs mode, `Tab` and `]`
   advance to the next tab (wrapping), `[` goes back; the shown flat list
   changes to the active group. @impl/done

9. ##STEP-9-QUIT Quit. @impl/done

   ```
   (press q)
   ```

   ##EXP-9-QUIT **Expected.** The TUI exits, the alternate screen is torn down, and the
   normal terminal (with your scrollback intact) is restored — no leftover
   raw-mode, no garbled prompt, exit code 0. @impl/done

## Teardown

##teardown-none None — `vibe tree` wrote nothing. (If a crash ever leaves the terminal in raw
mode, `reset` restores it.) @impl/done

## What to file if it fails

- ##FAIL-STEP-NUMBER The failing step number; what you saw beside its **Expected**. @impl/done
- ##FAIL-SCREENSHOT A screenshot or a copy of the mis-rendered frame (the exact glyphs/colours). @impl/done
- ##FAIL-MODEL-OUTPUT `./target/debug/vibe tree --json` output (the model the TUI renders) and
  `./target/debug/vibe tree --plain` (the same tree, copy-pasteable). @impl/done
- ##FAIL-PLATFORM Platform, terminal emulator + `$TERM`, `./target/debug/vibe --version`, shell. @impl/done
