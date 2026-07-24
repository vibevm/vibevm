# Design doc: the `vibe tree` TUI visual language (lore for PROP-037 §2.2)

<status stage="doc" state="done" comment="B0 2026-07-24: FULL 2026-07-16; lore for PROP-037 S2.2, PROP-037 wins on conflict"/>

##status-line _Status: **FULL** (2026-07-16, TREE-TUI-PLAN v0.2 Phase 1). This is **lore** — the
aesthetics, the why, the tables the eye reads. The **normative** surface (the REQs the code is
traceable to) lives in [PROP-037 §2.2](../modules/vibe-cli/PROP-037-tree-tui.md#theme); this document
expands it. When the two disagree, PROP-037 wins._ @doc/done

##genre-line **Genre:** design doc (lore). **Contract:** [PROP-037 §2.2](../modules/vibe-cli/PROP-037-tree-tui.md#theme). @doc/done
##related **Related:** [the action-system design](action-system.md); the settings meta-plan
(`../terraforms/SETTINGS-SYSTEM-META-PLAN-v0.1.md`) — the active palette + tier are a user setting. @doc/done

##OWNER-VISION **Owner vision (2026-07-16):** the TUI must look **deliberately beautiful**, not like ASCII
scaffolding. Unicode box-drawing with rounded corners (`╭╮╰╯`); truecolor with a **formal,
data-driven palette system** (five palettes; the current Rosé Pine "cosmic violet" preserved
exactly); braille/block glyphs for indicators; **symbols only** for the primary UI (SSH/tmux
portable — no Sixel/Kitty), but `ratatui-image`-ready; a graceful degradation strategy
truecolor→256→16 / rounded→unicode→ASCII; windows and modals that read as **windows**, not as a
"frame dropped on the worst terminal." @doc/done

---

## 1. Principles (the aesthetics)

- ##PRIN-SYMBOLIC-PRECISION **Symbolic precision.** Every glyph is chosen, not defaulted. Box-drawing, braille, and block
  elements carry the meaning; ASCII (`+ - * # x .`) is reserved for the **last** tier of degradation,
  never the primary look. @doc/done
- ##PRIN-TRUECOLOR-FIRST **Truecolor first.** 24-bit RGB is the primary tier; colour is expressed through **semantic role
  tokens** resolved by a swappable `Palette`, never a raw `Color` at a call site. @doc/done
- ##PRIN-PORTABILITY **Portability.** The primary UI is glyphs + colour roles. It works through SSH, `tmux`, a Linux
  VT, and a truecolor terminal — degraded but never broken. @doc/done
- ##PRIN-GRACEFUL-DEGRADATION **Graceful degradation.** A worse terminal gets a recognisably simpler look, not a broken one. One
  `Theme` value is the single source of truth, **projected** onto the detected tier. @doc/done
- ##PRIN-WINDOWS-ARE-WINDOWS **Windows are windows.** A modal reads as a solid panel floating over the terminal (filled
  background + rounded frame + padding + optional shadow) — never as "an error box on the worst
  terminal." @doc/done
- ##PRIN-ONE-CSS **One CSS.** A restyle touches only the theme; no component logic, no control flow. The `Theme`
  is the "CSS" of the TUI (PROP-037 §1.4, §2.2). @doc/done

---

## 2. Glyph vocabulary {#glyph-vocabulary}

##glyph-lead Every glyph is a constant on the `Theme`, never a hardcoded string at a call site. The current
ASCII scaffolding → the target vocabulary: @doc/done

| Purpose | Today | Tier 3 target | Tier 0 (ASCII) |
|---|---|---|---|
| ##ROW-GLYPH-TREE Tree connectors @doc/done | `│ ├ └ ─` (✓ Unicode) @doc/done | `│ ├ └ ─` @doc/done | `| -` @doc/done |
| ##ROW-GLYPH-FOLD Fold indicator @doc/done | `+` / `-` (ASCII ✗) @doc/done | `▾` / `▸` (or `▼`/`▶`) @doc/done | `+` / `-` @doc/done |
| ##ROW-GLYPH-DAG DAG re-occurrence @doc/done | `(*)` (ASCII ✗) @doc/done | `↩` (or `⊙`/`◆`) @doc/done | `*` @doc/done |
| ##ROW-GLYPH-FLAGS Flags on / off @doc/done | `x` / `.` (ASCII ✗) @doc/done | `●` / `○` (or `✓`/`·`) @doc/done | `x` / `.` @doc/done |
| ##ROW-GLYPH-ROUNDED Rounded frame @doc/done | `╭ ╮ ╰ ╯` (✓ already) @doc/done | `╭ ╮ ╰ ╯` @doc/done | `+ - |` @doc/done |
| ##ROW-GLYPH-SQUARE Square frame @doc/done | — @doc/done | `┌ ┐ └ ┘` @doc/done | `+ - |` @doc/done |
| ##ROW-GLYPH-CLOSE Close affordance @doc/done | (none) @doc/done | `✕` (or `✖`) @doc/done | `x` @doc/done |
| ##ROW-GLYPH-SEPARATOR Separator / rule @doc/done | `—` @doc/done | `─` / `╌` @doc/done | `-` @doc/done |
| ##ROW-GLYPH-BAR Bar / indicator @doc/done | (none) @doc/done | `▁ ▂ ▃ ▄ ▅ ▆ ▇ █` / braille `⠁ … ⣿` @doc/done | `#` @doc/done |
| ##ROW-GLYPH-SCROLL Scroll marker @doc/done | (none) @doc/done | `▲` / `▼` @doc/done | `^` / `v` @doc/done |

##NO-ASCII-PRIMARY **Rule (normative in PROP-037 §2.2 `#glyph-vocabulary`):** in the primary UI (Tier ≥ 1) there are no
`+`/`-`/`*`/`#`/`x`/`.` used as *semantic* glyphs. ASCII lives only behind the Tier 0 fallback. Menu
checked/unchecked already uses `◉`/`○` (kept); copy status uses `✓`/`✗` (kept). @doc/done

---

## 3. Palette system {#palette-tokens}

##palette-definition A `Palette` is a data-driven mapping from **semantic role tokens** → `Color`. The roles (the full
set; everything a component can ask for): @doc/done

##palette-roles-list `base` · `surface0` · `surface1` · `muted` · `subtext` · `text` · `accent` · `love` · `gold` ·
`foam` · `rose` · `selection` · `border` · `paper` · `button_on` · `button_off`. @doc/done

##role-semantics-lead The role semantics: @doc/done
- ##ROLE-BASE `base` — the terminal/window ground. The main tree keeps the user's terminal background (so a
  themed terminal shows through); modals paint a solid `base` panel. @doc/done
- ##ROLE-SURFACES `surface0` / `surface1` — raised surfaces (status bar chrome / a subtle fill, an off-flag). @doc/done
- ##ROLE-MUTED `muted` — borders, disabled text, "why disabled" reasons. @doc/done
- ##ROLE-SUBTEXT `subtext` — secondary foreground (footer descriptions, header values). @doc/done
- ##ROLE-TEXT `text` — primary foreground. @doc/done
- ##ROLE-ACCENT `accent` — the brand: selection highlight, titles, the active key label, the cosmic violet. @doc/done
- ##ROLE-TONAL-ACCENTS `love` / `gold` / `foam` / `rose` — the four tonal accents (warnings / badges & highlights /
  static-load & links / secondary badges). @doc/done
- ##ROLE-SELECTION `selection` — the highlighted row (composed: `accent` ground + `base` text). @doc/done
- ##ROLE-BORDER `border` — the window/frame stroke (usually `muted`). @doc/done
- ##ROLE-PAPER `paper` — the detail-card panel, distinct from the tree beneath (a "paper card": light panel, dark
  text on dark themes; inverted on light themes). @doc/done
- ##ROLE-BUTTONS `button_on` / `button_off` — a focused vs unfocused button. @doc/done

##five-palettes-lead **Five built-in palettes** (the full set — a worked example of mass theming): @doc/done

| Palette | Tone | `base` | `surface0` | `surface1` | `muted` | `subtext` | `text` | `accent` | `love` | `gold` | `foam` | `rose` |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| ##ROW-PAL-ROSE-PINE **Rosé Pine** @doc/done | dark (cosmic violet) @doc/done | `#191724` @doc/done | `#1f1d2e` @doc/done | `#26233a` @doc/done | `#6e6a86` @doc/done | `#908caa` @doc/done | `#e0def4` @doc/done | `#c4a7e7` @doc/done | `#eb6f92` @doc/done | `#f6c177` @doc/done | `#9ccfd8` @doc/done | `#ebbcba` @doc/done |
| ##ROW-PAL-MOCHA **Catppuccin Mocha** @doc/done | dark @doc/done | `#1e1e2e` @doc/done | `#313244` @doc/done | `#45475a` @doc/done | `#6c7086` @doc/done | `#a6adc8` @doc/done | `#cdd6f4` @doc/done | `#cba6f7` @doc/done | `#f38ba8` @doc/done | `#f9e2af` @doc/done | `#94e2d5` @doc/done | `#f5c2e7` @doc/done |
| ##ROW-PAL-MACCHIATO **Catppuccin Macchiato** @doc/done | dark @doc/done | `#24273a` @doc/done | `#363a4f` @doc/done | `#494d64` @doc/done | `#6e738d` @doc/done | `#a5adcb` @doc/done | `#cad3f5` @doc/done | `#c6a0f6` @doc/done | `#ed8796` @doc/done | `#eed49f` @doc/done | `#8bd5ca` @doc/done | `#f5bde6` @doc/done |
| ##ROW-PAL-FRAPPE **Catppuccin Frappé** @doc/done | dark @doc/done | `#303446` @doc/done | `#414559` @doc/done | `#51576d` @doc/done | `#737994` @doc/done | `#a5adce` @doc/done | `#c6d0f5` @doc/done | `#ca9ee6` @doc/done | `#e78284` @doc/done | `#e5c890` @doc/done | `#81c8be` @doc/done | `#f4b8e4` @doc/done |
| ##ROW-PAL-LATTE **Catppuccin Latte** @doc/done | **light** @doc/done | `#eff1f5` @doc/done | `#ccd0da` @doc/done | `#bcc0cc` @doc/done | `#9ca0b0` @doc/done | `#6c6f85` @doc/done | `#4c4f69` @doc/done | `#8839ef` @doc/done | `#d20f39` @doc/done | `#df8e1d` @doc/done | `#179299` @doc/done | `#ea76cb` @doc/done |

- ##ROSE-PINE-LOCKED **Rosé Pine is canonical-locked**: the eleven `Color::Rgb` values already in
  `crates/vibe-cli/src/commands/tree/tui/theme.rs` become the Rosé Pine `Palette` cell **unchanged**
  (R8). A snapshot test pins them. @doc/done
- ##CATPPUCCIN-MAPPING **Catppuccin** values are the canonical Catppuccin palette (the four flavours; Latte is the light
  one). Mapping: `accent`←mauve, `love`←red, `gold`←yellow, `foam`←teal, `rose`←pink, `muted`←overlay0,
  `subtext`←subtext0. @doc/done
- ##LIGHT-DARK-AWARENESS **Light/dark-awareness:** Latte is the light reference. A `Palette` carries an `is_light: bool`
  flag; the `paper` role and the `selection` composition invert against it (a light theme's "paper"
  card is a *dark* inset, a dark text on a light ground, etc.). The derived roles: @doc/done
  - ##DERIVED-SELECTION `selection` = `accent` ground + `base` text (bold) — high-contrast on every palette. @doc/done
  - ##DERIVED-BORDER `border` = `muted`. @doc/done
  - ##DERIVED-PAPER `paper` = `surface0` (raised, distinct from the tree ground); the detail-card text stays `text`. @doc/done
  - ##DERIVED-BUTTONS `button_on` = `accent` (focused), `button_off` = `surface1`. @doc/done

##palette-is-setting The active palette is a `Model` field; through the settings system (Шаг 2) it persists across
L1/L2/L3 and is overridable at the CLI/env. @doc/done

---

## 4. Rendering tiers + degradation {#rendering-tiers}

| Tier | Condition | Palette | Frames | Indicators |
|---|---|---|---|---|
| ##ROW-TIER-3 **3** @doc/done | `$COLORTERM` ∈ {`truecolor`,`24bit`} @doc/done | full 24-bit RGB @doc/done | rounded `╭╮╰╯` @doc/done | braille / blocks @doc/done |
| ##ROW-TIER-2 **2** @doc/done | 256-colour (`$TERM` contains `256`) @doc/done | palette quantised to the 6×6×6 cube @doc/done | rounded @doc/done | blocks (8) @doc/done |
| ##ROW-TIER-1 **1** @doc/done | 16-colour ANSI @doc/done | ANSI role mapping @doc/done | rounded (if supported) / square `┌┐└┘` @doc/done | blocks (8) @doc/done |
| ##ROW-TIER-0 **0** @doc/done | explicitly dumb (`TERM=linux` / `dumb`) @doc/done | ANSI mono @doc/done | ASCII `+-\|` @doc/done | `#` @doc/done |

##TIER-DETECTION **Detection (normative in PROP-037 §2.2 `#rendering-tiers`):** a **pure function** over the
environment — `detect_tier(colorterm: Option<&str>, term: Option<&str>) -> Tier`. `crossterm` exposes
no colour-count API, so detection is env-driven (`$COLORTERM` first, then `$TERM`); the TUI reads the
env once, at launch, in a sanctioned spot, and feeds the values in. The **default is Tier 3** — a
modern terminal is assumed truecolour even when it does not advertise the capability via env
(notably on Windows); the lower tiers are the **fallback** (the degradation path), reached only when
the environment explicitly advertises a lower capability (a 256-colour `TERM`, or an explicitly dumb
`TERM=linux`/`dumb`). The detected tier is overridable through the settings system. @doc/done

##DEGRADATION-PROJECTION **Degradation = projection.** A `Theme` is built for Tier 3 (the full palette + rounded + braille),
then **projected** onto the detected tier: a 256-colour terminal gets each role quantised to the
nearest 6×6×6 cube colour; a 16-colour terminal gets the role mapped to one of the eight ANSI colours;
Tier 0 falls back to ASCII frames and `#` indicators. One source of truth (`Theme`), many projections. @doc/done

---

## 5. Window / modal aesthetics {#window-aesthetics}

##window-composition-lead A window is not a fallback. The composition (the `ui::Window` component, PROP-037 §2.3): @doc/done

- ##WIN-SOLID-PANEL a **solid panel** background (filled `base`), floating over the terminal; @doc/done
- ##WIN-ROUNDED-FRAME a **rounded frame** (`╭╮╰╯` at Tier ≥ 2; the frame stroke is `border` = `muted`); @doc/done
- ##WIN-TITLE-CHIP a **title chip** — the window title rendered as a badge in the `accent` colour, not bare text; @doc/done
- ##WIN-PADDING-SHADOW **padding** inside the frame; an optional **shadow** (a low-alpha `▓`/`▒` band, or a 1-char offset)
  so the panel reads as raised; @doc/done
- ##WIN-CLOSE-AFFORDANCE a `[✕]` **close affordance** in the top-right. @doc/done

##WINDOW-AT-TIER-1 Even at Tier 1 the window reads as a window — the rounded/square frame + filled panel + title chip
carry the "floating panel" reading without truecolor. The depth-2 modal stack (copy-settings →
file-path, PROP-037 §6, §10.5) draws each lower layer as a dimmed backdrop; the top window is the
only one that takes input. @doc/done

---

## 6. Spacing & rhythm {#spacing-rhythm}

##spacing-vocabulary-lead A beautiful window is mostly **empty space used well**. The complaint that turns a "window" back
into "an error box on the worst terminal" is content jammed into a corner: a message flush against
the frame, a row of hints spilling off the left edge with no centre. The fix is a spacing
vocabulary held in three constants, so a re-space is one edit (the normative surface is
[PROP-037 §2.2.5](../modules/vibe-cli/PROP-037-tree-tui.md#spacing)): @doc/done

- ##SPACE-INTERIOR-PADDING **Interior padding (`PAD_X` = 2, `PAD_Y` = 1).** Every window frame holds its content off the
  border — two cells left and right, one row top and bottom. The body of a dialog floats: a blank
  row under the title chip, a blank row above the base, clear margins at the sides. This is what
  makes a `╭╮╰╯` frame read as a *raised panel* and not a box drawn around text. `ui::inner_pad` is
  the one helper; a dialog sizes its window to leave room for the padding, then lays its content
  into `inner_pad(inner)`. @doc/done
- ##SPACE-GROUP-GUTTER **Group gutter (`GUTTER` = 1).** A radio option inside a `Group` frame is inset one cell off the
  stroke, and its selection highlight bar is inset with it — so the `●`/`○` mark and the accent bar
  sit *inside* the group, never welded to its border. Nested frames (a window, then a group inside
  it) each keep their own breathing room, so the eye separates the levels. @doc/done
- ##SPACE-RHYTHM-CENTRING **Rhythm & centring.** Structure is read from separation: framed groups get a blank row between
  them; an inline run of hints gets a `•` separator with a space each side; and a row of controls
  is **centred in its area**. A left-jammed row reads as debug output — a website footer that slid
  to the left margin. The footer is the worked example: **two centred rows**, the F-keys above and
  the navigation below, each centred under the screen with `•`-separated rhythm. @doc/done

##AIR-RULE The rule in one line: **content floats inside the frame with air on every side, and multi-element
rows are centred.** Air is not wasted space — it is the difference between a UI and a dump. @doc/done

---

## 7. ratatui-image readiness {#image-ready}

##IMAGE-READINESS The primary UI is glyphs (portable). But the structure is **ready** for `ratatui-image` (future
package-preview images, a designed info-card image): placeholder slots and reserved image areas in
the layout, behind a capability flag. Sixel/Kitty are **not** for the primary UI — only an optional
image raster when the terminal advertises support. Reserved, not built (PROP-037 §12 non-goal). @doc/done

---

## 8. What becomes normative in PROP-037 §2.2

##normative-pointer When §2.2 carries the anchors: `#palette-tokens` (the role set + the five canonical palettes),
`#glyph-vocabulary` (the replacement table + the "no ASCII in the primary UI" rule),
`#rendering-tiers` (the tier table + the pure `detect_tier` + the projection law), `#window-aesthetics`
(the window composition + "a window is not a fallback"), `#spacing` (interior padding, the group
gutter, and centred rhythm). This lore explains *why*; the contract carries the *values* the code
traces to. @doc/done
