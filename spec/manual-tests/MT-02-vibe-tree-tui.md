# vibe tree — the TUI application (PROP-037) visual sign-off

##purpose **Purpose.** `vibe tree` is now a full TUI application (PROP-037, TREE-TUI-PLAN
v0.2): a formal visual language (five palettes, glyph vocabulary, rendering
tiers), a reusable `ui::` component library, the tree filter/shape pipeline,
trees in every mode, a keymap-driven action dispatch, a detail card, settings
persistence, and a copy system. The automated suite (241 vibe-cli tests,
`self-check` all green) proves the *model + the rendering fns*; it cannot drive
a real terminal and confirm the TUI *looks and reads right* — that the Unicode
box-drawing aligns, the palette is beautiful and switchable, the windows float,
the card wraps, the modals cascade at depth 2. That is what a human signs off
here. @impl/done

##settings-mutation-note Unlike MT-01, this TUI **writes user settings** (`~/.vibe/` via the vibe-settings
system, Шаг 2) — palette/tier/mode/sort/shape choices persist. The test mutates
only machine-global user prefs (never project files); back them up or accept the
delta. @impl/done

## Preconditions

- ##PRE-TTY-TRUECOLOR A real interactive terminal (a tty), ideally truecolor (`echo $COLORTERM` →
  `truecolor`/`24bit`) so Tier 3 shows; a 256-colour or 16-colour terminal is the
  degradation path and is worth a second pass. @impl/done
- ##PRE-BINARY `cargo build -p vibe-cli` (invoke `./target/debug/vibe`). @impl/done
- ##PRE-REPO-ROOT Run from the vibevm repo root. @impl/done

## Steps

1. ##STEP-1-DEFAULT-LOOK **Launch + the default look (Rosé Pine, Tier 3).** @impl/done
   ```
   ./target/debug/vibe tree
   ```
   - ##S1-GLYPHS Tree connectors `│├└─` align; fold indicator is `▾`/`▸` (not `+`/`-`); DAG
     re-occurrence is `↩` (not `(*)`); flags are `●`/`○` (not `x`/`.`). @impl/done
   - ##S1-FOOTER The footer lists `F1 search · F2 sort · F3 mode · F4 settings · F6 copy
     · ↑↓ move · ←→ pan · Space fold · Enter details · Esc quit` (Tabs mode adds
     `Shift+←/→ tab`). @impl/done
   - ##S1-STATUS-LINE The status line shows ordering · mode · STATIC.md size · package count. @impl/done

2. ##STEP-2-NAVIGATION **Navigation + fold (every mode is a tree).** `↑`/`↓` move, `←`/`→` pan,
   `Space` folds a node (▾↔▸), `Enter` opens the detail card. @impl/done

3. ##STEP-3-DETAIL-CARD **The detail card (§8).** `Enter` on a package → a paper panel, **bold field
   headers**, wrapped long values (a 64-char hash wraps, never truncates), a `✕`
   close affordance top-right; `Esc`/`✕` closes. @impl/done

4. ##STEP-4-MODES **Modes (§4) — all trees.** `F3` → the mode menu → SubTables (stacked trees per
   load partition, each under a subheader), Tabs (`Shift+←`/`Shift+→` switch
   tabs, each tab a tree). Fold a package in SubTables — it folds in every block. @impl/done

5. ##STEP-5-SORT-SHAPE **Sort & shape (§7.2).** `F2` → a multi-group dialog: Sort by (alphabetical/
   topological) + Shape (members-as-roots / load-type-forest / pruned-tree) +
   Block order (sub-tables only). Pick a shape — the tree re-forms. The menu
   stays open (sticky); `Esc` closes. @impl/done

6. ##STEP-6-SEARCH **Search Everywhere (§7.3).** `F1` → the hybrid "All" + per-category tabs;
   type a query — packages, card fields, and `vibe.tree` actions match; `Enter`
   on an action runs it in place. @impl/done

7. ##STEP-7-COPY **Copy (§10).** `F6` → copies the current screen (tree or card) as Markdown
   to the clipboard (footer flash `✓ copied`). `Shift+F6` → copy-settings
   (format Markdown/PNG + dest clipboard/file). PNG → ComingSoon. dest=file →
   the FileDest modal (TextField + Save/Cancel) **over** copy-settings (depth-2);
   `Esc` returns to copy-settings, not the base. @impl/done

8. ##STEP-8-QUIT-CONFIRM **Quit-confirm (§7.4).** At the base, `Esc` → "Really quit?" dialog (not an
   instant quit); `Enter` quits, `Esc`/`No` cancels. @impl/done

9. ##STEP-9-PALETTE **Switchable palette (§2.2.1, the owner vision).** Quit, set the palette: @impl/done
   ```
   # edit directly, or pick it in the F4 settings menu    ~/.vibe/settings.toml
   [vibe.tree]
   palette = "catppuccin-mocha"   # or -macchiato / -frappe / -latte (light) / rose-pine
   ```
   ##S9-RELAUNCH Relaunch — the whole UI (tree, windows, card, menus, search) is now in the
   chosen palette; Latte is light. Tier override: `tier = 1` (16-colour) or
   `0` (ASCII fallback) to see the degradation. @impl/done

10. ##STEP-10-PERSISTENCE **Persistence.** Change mode/sort/shape via the menus; quit; relaunch — the
    choices are restored. @impl/done

## Pass

##pass-criteria Every step reads as described: the Unicode aligns, the palettes are beautiful
and distinct, the windows float, the card wraps, the depth-2 cascade works, and
the five palettes all render. The owner signs the date below. @impl/done

## Sign-off

- ##SIGNOFF-OWNER [ ] Owner visual sign-off (date / initials): ______ @impl/work
- ##SIGNOFF-TIER-3 Tier 3 (truecolor) checked on terminal: ______ @impl/work
- ##SIGNOFF-DEGRADATION A degradation tier (256 / 16 / ASCII) also checked: ______ @impl/work
