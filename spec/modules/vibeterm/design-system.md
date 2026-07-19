# VibeTerm Design System (lore — the GUI twin of `tui-visual-language.md`)

**Genre:** design doc (lore) — non-binding aesthetics + rationale. Normative REQs live in the vibeterm
PROP family (D3); when the two disagree, the contract wins. **Source:**
[`research/vibeterm/vibeterm-ui-architecture-findings-v0.1.md` §2.13](../../../research/vibeterm/vibeterm-ui-architecture-findings-v0.1.md#ports-adapts-new).
**Sibling:** [`tui-visual-language.md`](../../design/tui-visual-language.md) (the TUI original); this is
its GUI twin.

## 1. Principles (ported from the TUI)

- **Role-tokens, never raw colour.** A component names a **semantic role**; a token resolves it to a value.
  The TUI's `Palette` (role → `Color::Rgb`) becomes the GUI's **design tokens** (role → CSS custom
  property). A restyle touches only the token set — the token set is the "CSS."
- **One source, projected.** One token set is **projected** across themes × modes (the GUI analogue of the
  TUI's "one Theme projected across tiers"). The TUI's tier-degradation (truecolor → dumb) has **no GUI
  analogue** — it is replaced by theme + accessibility/density modes.
- **Live switch.** Themes and locales switch at runtime by rebinding token values — no reload, no restart.
- **Windows are windows.** A panel reads as a solid surface with a frame, a title, interior padding —
  never "a box dropped on the worst desktop."
- **Air, not jam.** Content floats inside the frame; rows are centred; spacing is a scale, not magic
  numbers.

## 2. The token architecture

| Layer | What | Example |
|---|---|---|
| **Role-tokens** | the semantic surface a component asks for (the contract) | `--vibe-surface`, `--vibe-accent`, `--vibe-text`, `--vibe-muted`, `--vibe-border`, `--vibe-selection`, … |
| **Theme** | a role → value mapping (a token set) | the **dark-purple** theme (after the ProjectX reference); the **Anthropic-style** theme |
| **Mode** | a variant axis layered on a theme | `prefers-reduced-motion`, `prefers-contrast`, density (`compact`/`comfortable`) |

A **computed token** at render time = the active theme's value, possibly transformed by the active mode.
Components read `var(--vibe-<role>)`; they never read a hex literal.

### 2.1 Tailwind v4 `@theme` integration (RQ7a)

Our role-tokens are the **source**; Tailwind v4's `@theme` **consumes** them — Tailwind generates its
utilities from our CSS variables. One namespace, not two; Tailwind is the utility layer, our tokens are the
truth.

### 2.2 Kobalte theming (RQ7b)

Kobalte primitives are styled through a thin adapter that maps our roles onto Kobalte's expected CSS
variables. Components never reach past our tokens.

### 2.3 Accessibility / density modes (RQ7c)

A **theme × mode matrix**. Modes are driven by both CSS media queries (`prefers-reduced-motion`,
`prefers-contrast: more`) and explicit user choice (the `view.set-compact` action; a future density
control). A mode is a token-variant layer, not a second theme.

## 3. The two launch themes

- **dark-purple** — after the ProjectX reference layout: a deep purple-violet ground, an iris/violet
  accent, soft surface layers, a warm secondary. The default.
- **Anthropic-style** — a warm paper/cream-tinted light surface, a clay/coral accent, the calm editorial
  feel.

Both ship from M1's first vertical slice (PROP-044 §11); the token contract is the binding part, the
visual tuning is iterative.

## 4. The icon vocabulary (RQ7d — genuinely new)

The TUI glyph table (`▾▸●○╭╮`) does **not** carry over. A GUI needs an **SVG icon system**: a small owned
set consumed through one `<Icon name role>` primitive; icons inherit colour from a role token (never a raw
fill); every icon carries an a11y label. Reserved slots for not-yet-drawn icons mirror the TUI's
`ComingSoon`.

## 5. Spacing & rhythm (RQ7e)

One **spacing-scale** exposed as tokens (`--vibe-space-1 … --vibe-space-8`); Tailwind utilities reference
it. The TUI's `PAD_X`/`PAD_Y`/`GUTTER` constants become layout primitives: interior padding inside every
panel, a gutter inside every group, centred multi-element rows. A left-jammed row reads as debug output —
air is the difference between a UI and a dump.

## 6. The contacts-style list (the M1 visible surface)

The terminal list is a **vertical list of items in a left rail**, each item a row (icon + title + a
secondary line + an active indicator) — the contacts/channels-list idiom. Selecting an item selects that
tab's view in the content area; the active item carries the `selection` role. This is the M1 "contacts in
ProjectX" look the owner named: a calm, item-per-row rail, not a tab strip.
