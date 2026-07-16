# Design doc: the `vibe tree` TUI visual language (lore for PROP-037 §2.2)

_Status: **FULL** (2026-07-16, TREE-TUI-PLAN v0.2 Phase 1). This is **lore** — the
aesthetics, the why, the tables the eye reads. The **normative** surface (the REQs the code is
traceable to) lives in [PROP-037 §2.2](../modules/vibe-cli/PROP-037-tree-tui.md#theme); this document
expands it. When the two disagree, PROP-037 wins._

**Genre:** design doc (lore). **Contract:** [PROP-037 §2.2](../modules/vibe-cli/PROP-037-tree-tui.md#theme).
**Related:** [the action-system design](action-system.md); the settings meta-plan
(`../terraforms/SETTINGS-SYSTEM-META-PLAN-v0.1.md`) — the active palette + tier are a user setting.

> **Owner vision (2026-07-16):** the TUI must look **deliberately beautiful**, not like ASCII
> scaffolding. Unicode box-drawing with rounded corners (`╭╮╰╯`); truecolor with a **formal,
> data-driven palette system** (five palettes; the current Rosé Pine "cosmic violet" preserved
> exactly); braille/block glyphs for indicators; **symbols only** for the primary UI (SSH/tmux
> portable — no Sixel/Kitty), but `ratatui-image`-ready; a graceful degradation strategy
> truecolor→256→16 / rounded→unicode→ASCII; windows and modals that read as **windows**, not as a
> "frame dropped on the worst terminal."

---

## 1. Principles (the aesthetics)

- **Symbolic precision.** Every glyph is chosen, not defaulted. Box-drawing, braille, and block
  elements carry the meaning; ASCII (`+ - * # x .`) is reserved for the **last** tier of degradation,
  never the primary look.
- **Truecolor first.** 24-bit RGB is the primary tier; colour is expressed through **semantic role
  tokens** resolved by a swappable `Palette`, never a raw `Color` at a call site.
- **Portability.** The primary UI is glyphs + colour roles. It works through SSH, `tmux`, a Linux
  VT, and a truecolor terminal — degraded but never broken.
- **Graceful degradation.** A worse terminal gets a recognisably simpler look, not a broken one. One
  `Theme` value is the single source of truth, **projected** onto the detected tier.
- **Windows are windows.** A modal reads as a solid panel floating over the terminal (filled
  background + rounded frame + padding + optional shadow) — never as "an error box on the worst
  terminal."
- **One CSS.** A restyle touches only the theme; no component logic, no control flow. The `Theme`
  is the "CSS" of the TUI (PROP-037 §1.4, §2.2).

---

## 2. Glyph vocabulary {#glyph-vocabulary}

Every glyph is a constant on the `Theme`, never a hardcoded string at a call site. The current
ASCII scaffolding → the target vocabulary:

| Purpose | Today | Tier 3 target | Tier 0 (ASCII) |
|---|---|---|---|
| Tree connectors | `│ ├ └ ─` (✓ Unicode) | `│ ├ └ ─` | `| -` |
| Fold indicator | `+` / `-` (ASCII ✗) | `▾` / `▸` (or `▼`/`▶`) | `+` / `-` |
| DAG re-occurrence | `(*)` (ASCII ✗) | `↩` (or `⊙`/`◆`) | `*` |
| Flags on / off | `x` / `.` (ASCII ✗) | `●` / `○` (or `✓`/`·`) | `x` / `.` |
| Rounded frame | `╭ ╮ ╰ ╯` (✓ already) | `╭ ╮ ╰ ╯` | `+ - |` |
| Square frame | — | `┌ ┐ └ ┘` | `+ - |` |
| Close affordance | (none) | `✕` (or `✖`) | `x` |
| Separator / rule | `—` | `─` / `╌` | `-` |
| Bar / indicator | (none) | `▁ ▂ ▃ ▄ ▅ ▆ ▇ █` / braille `⠁ … ⣿` | `#` |
| Scroll marker | (none) | `▲` / `▼` | `^` / `v` |

**Rule (normative in PROP-037 §2.2 `#glyph-vocabulary`):** in the primary UI (Tier ≥ 1) there are no
`+`/`-`/`*`/`#`/`x`/`.` used as *semantic* glyphs. ASCII lives only behind the Tier 0 fallback. Menu
checked/unchecked already uses `◉`/`○` (kept); copy status uses `✓`/`✗` (kept).

---

## 3. Palette system {#palette-tokens}

A `Palette` is a data-driven mapping from **semantic role tokens** → `Color`. The roles (the full
set; everything a component can ask for):

`base` · `surface0` · `surface1` · `muted` · `subtext` · `text` · `accent` · `love` · `gold` ·
`foam` · `rose` · `selection` · `border` · `paper` · `button_on` · `button_off`.

The role semantics:
- `base` — the terminal/window ground. The main tree keeps the user's terminal background (so a
  themed terminal shows through); modals paint a solid `base` panel.
- `surface0` / `surface1` — raised surfaces (status bar chrome / a subtle fill, an off-flag).
- `muted` — borders, disabled text, "why disabled" reasons.
- `subtext` — secondary foreground (footer descriptions, header values).
- `text` — primary foreground.
- `accent` — the brand: selection highlight, titles, the active key label, the cosmic violet.
- `love` / `gold` / `foam` / `rose` — the four tonal accents (warnings / badges & highlights /
  static-load & links / secondary badges).
- `selection` — the highlighted row (composed: `accent` ground + `base` text).
- `border` — the window/frame stroke (usually `muted`).
- `paper` — the detail-card panel, distinct from the tree beneath (a "paper card": light panel, dark
  text on dark themes; inverted on light themes).
- `button_on` / `button_off` — a focused vs unfocused button.

**Five built-in palettes** (the full set — a worked example of mass theming):

| Palette | Tone | `base` | `surface0` | `surface1` | `muted` | `subtext` | `text` | `accent` | `love` | `gold` | `foam` | `rose` |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| **Rosé Pine** | dark (cosmic violet) | `#191724` | `#1f1d2e` | `#26233a` | `#6e6a86` | `#908caa` | `#e0def4` | `#c4a7e7` | `#eb6f92` | `#f6c177` | `#9ccfd8` | `#ebbcba` |
| **Catppuccin Mocha** | dark | `#1e1e2e` | `#313244` | `#45475a` | `#6c7086` | `#a6adc8` | `#cdd6f4` | `#cba6f7` | `#f38ba8` | `#f9e2af` | `#94e2d5` | `#f5c2e7` |
| **Catppuccin Macchiato** | dark | `#24273a` | `#363a4f` | `#494d64` | `#6e738d` | `#a5adcb` | `#cad3f5` | `#c6a0f6` | `#ed8796` | `#eed49f` | `#8bd5ca` | `#f5bde6` |
| **Catppuccin Frappé** | dark | `#303446` | `#414559` | `#51576d` | `#737994` | `#a5adce` | `#c6d0f5` | `#ca9ee6` | `#e78284` | `#e5c890` | `#81c8be` | `#f4b8e4` |
| **Catppuccin Latte** | **light** | `#eff1f5` | `#ccd0da` | `#bcc0cc` | `#9ca0b0` | `#6c6f85` | `#4c4f69` | `#8839ef` | `#d20f39` | `#df8e1d` | `#179299` | `#ea76cb` |

- **Rosé Pine is canonical-locked**: the eleven `Color::Rgb` values already in
  `crates/vibe-cli/src/commands/tree/tui/theme.rs` become the Rosé Pine `Palette` cell **unchanged**
  (R8). A snapshot test pins them.
- **Catppuccin** values are the canonical Catppuccin palette (the four flavours; Latte is the light
  one). Mapping: `accent`←mauve, `love`←red, `gold`←yellow, `foam`←teal, `rose`←pink, `muted`←overlay0,
  `subtext`←subtext0.
- **Light/dark-awareness:** Latte is the light reference. A `Palette` carries an `is_light: bool`
  flag; the `paper` role and the `selection` composition invert against it (a light theme's "paper"
  card is a *dark* inset, a dark text on a light ground, etc.). The derived roles:
  - `selection` = `accent` ground + `base` text (bold) — high-contrast on every palette.
  - `border` = `muted`.
  - `paper` = `surface0` (raised, distinct from the tree ground); the detail-card text stays `text`.
  - `button_on` = `accent` (focused), `button_off` = `surface1`.

The active palette is a `Model` field; through the settings system (Шаг 2) it persists across
L1/L2/L3 and is overridable at the CLI/env.

---

## 4. Rendering tiers + degradation {#rendering-tiers}

| Tier | Condition | Palette | Frames | Indicators |
|---|---|---|---|---|
| **3** | `$COLORTERM` ∈ {`truecolor`,`24bit`} | full 24-bit RGB | rounded `╭╮╰╯` | braille / blocks |
| **2** | 256-colour (`$TERM` contains `256`) | palette quantised to the 6×6×6 cube | rounded | blocks (8) |
| **1** | 16-colour ANSI | ANSI role mapping | rounded (if supported) / square `┌┐└┘` | blocks (8) |
| **0** | dumb / `TERM=linux` / no Unicode | ANSI mono | ASCII `+-\|` | `#` |

**Detection (normative in PROP-037 §2.2 `#rendering-tiers`):** a **pure function** over the
environment — `detect_tier(colorterm: Option<&str>, term: Option<&str>) -> Tier`. `crossterm` exposes
no colour-count API, so detection is env-driven (`$COLORTERM` first, then `$TERM`); the TUI reads the
env once, at launch, in a sanctioned spot, and feeds the values in. The detected tier is overridable
through the settings system (a user on a misreported terminal can force Tier 3).

**Degradation = projection.** A `Theme` is built for Tier 3 (the full palette + rounded + braille),
then **projected** onto the detected tier: a 256-colour terminal gets each role quantised to the
nearest 6×6×6 cube colour; a 16-colour terminal gets the role mapped to one of the eight ANSI colours;
Tier 0 falls back to ASCII frames and `#` indicators. One source of truth (`Theme`), many projections.

---

## 5. Window / modal aesthetics {#window-aesthetics}

A window is not a fallback. The composition (the `ui::Window` component, PROP-037 §2.3):

- a **solid panel** background (filled `base`), floating over the terminal;
- a **rounded frame** (`╭╮╰╯` at Tier ≥ 2; the frame stroke is `border` = `muted`);
- a **title chip** — the window title rendered as a badge in the `accent` colour, not bare text;
- **padding** inside the frame; an optional **shadow** (a low-alpha `▓`/`▒` band, or a 1-char offset)
  so the panel reads as raised;
- a `[✕]` **close affordance** in the top-right.

Even at Tier 1 the window reads as a window — the rounded/square frame + filled panel + title chip
carry the "floating panel" reading without truecolor. The depth-2 modal stack (copy-settings →
file-path, PROP-037 §6, §10.5) draws each lower layer as a dimmed backdrop; the top window is the
only one that takes input.

---

## 6. ratatui-image readiness {#image-ready}

The primary UI is glyphs (portable). But the structure is **ready** for `ratatui-image` (future
package-preview images, a designed info-card image): placeholder slots and reserved image areas in
the layout, behind a capability flag. Sixel/Kitty are **not** for the primary UI — only an optional
image raster when the terminal advertises support. Reserved, not built (PROP-037 §12 non-goal).

---

## 7. What becomes normative in PROP-037 §2.2

When §2.2 carries the anchors: `#palette-tokens` (the role set + the five canonical palettes),
`#glyph-vocabulary` (the replacement table + the "no ASCII in the primary UI" rule),
`#rendering-tiers` (the tier table + the pure `detect_tier` + the projection law), `#window-aesthetics`
(the window composition + "a window is not a fallback"). This lore explains *why*; the contract
carries the *values* the code traces to.
