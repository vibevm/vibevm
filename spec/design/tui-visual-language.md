# Design doc: the `vibe tree` TUI visual language (lore for PROP-037 §2.2)

<status stage="doc" state="done" comment="B0 2026-07-24: FULL 2026-07-16; lore for PROP-037 S2.2, PROP-037 wins on conflict"/>

@fact:status-line _Status: **FULL** (2026-07-16, TREE-TUI-PLAN v0.2 Phase 1). This is **lore** — the
aesthetics, the why, the tables the eye reads. The **normative** surface (the REQs the code is
traceable to) lives in [PROP-037 §2.2](../modules/vibe-cli/PROP-037-tree-tui.md#theme); this document
expands it. When the two disagree, PROP-037 wins._ @status:doc/done

@fact:genre-line **Genre:** design doc (lore). **Contract:** [PROP-037 §2.2](../modules/vibe-cli/PROP-037-tree-tui.md#theme). @status:doc/done
@fact:related **Related:** [the action-system design](action-system.md); the settings meta-plan
(archived: `legacy-spec/terraforms/SETTINGS-SYSTEM-META-PLAN-v0.1.md`) — the active palette + tier are a user setting. @status:doc/done

@fact:OWNER-VISION **Owner vision (2026-07-16):** the TUI must look **deliberately beautiful**, not like ASCII
scaffolding. Unicode box-drawing with rounded corners (`╭╮╰╯`); truecolor with a **formal,
data-driven palette system** (five palettes; the current Rosé Pine "cosmic violet" preserved
exactly); braille/block glyphs for indicators; **symbols only** for the primary UI (SSH/tmux
portable — no Sixel/Kitty), but `ratatui-image`-ready; a graceful degradation strategy
truecolor→256→16 / rounded→unicode→ASCII; windows and modals that read as **windows**, not as a
"frame dropped on the worst terminal." @status:doc/done

---

## 1. Principles (the aesthetics)

- @fact:PRIN-SYMBOLIC-PRECISION **Symbolic precision.** Every glyph is chosen, not defaulted. Box-drawing, braille, and block
  elements carry the meaning; ASCII (`+ - * # x .`) is reserved for the **last** tier of degradation,
  never the primary look. @status:doc/done
- @fact:PRIN-TRUECOLOR-FIRST **Truecolor first.** 24-bit RGB is the primary tier; colour is expressed through **semantic role
  tokens** resolved by a swappable `Palette`, never a raw `Color` at a call site. @status:doc/done
- @fact:PRIN-PORTABILITY **Portability.** The primary UI is glyphs + colour roles. It works through SSH, `tmux`, a Linux
  VT, and a truecolor terminal — degraded but never broken. @status:doc/done
- @fact:PRIN-GRACEFUL-DEGRADATION **Graceful degradation.** A worse terminal gets a recognisably simpler look, not a broken one. One
  `Theme` value is the single source of truth, **projected** onto the detected tier. @status:doc/done
- @fact:PRIN-WINDOWS-ARE-WINDOWS **Windows are windows.** A modal reads as a solid panel floating over the terminal (filled
  background + rounded frame + padding + optional shadow) — never as "an error box on the worst
  terminal." @status:doc/done
- @fact:PRIN-ONE-CSS **One CSS.** A restyle touches only the theme; no component logic, no control flow. The `Theme`
  is the "CSS" of the TUI (PROP-037 §1.4, §2.2). @status:doc/done

---

## 2. Glyph vocabulary {#glyph-vocabulary}

@fact:glyph-lead Every glyph is a constant on the `Theme`, never a hardcoded string at a call site. The table below
is the **2026-07-16 capture**: the ASCII scaffolding of the day → the target vocabulary. The targets
shipped (`theme/glyphs.rs`, the TUI goldens, MT-02), so read the "Today" column as history, not as
current state: @status:doc/done

| Purpose | Today | Tier 3 target | Tier 0 (ASCII) |
|---|---|---|---|
| @fact:ROW-GLYPH-TREE Tree connectors @status:doc/done | `│ ├ └ ─` (✓ Unicode) @status:doc/done | `│ ├ └ ─` @status:doc/done | `| -` @status:doc/done |
| @fact:ROW-GLYPH-FOLD Fold indicator @status:doc/done | `+` / `-` (ASCII ✗) @status:doc/done | `▾` / `▸` (or `▼`/`▶`) @status:doc/done | `+` / `-` @status:doc/done |
| @fact:ROW-GLYPH-DAG DAG re-occurrence @status:doc/done | `(*)` (ASCII ✗) @status:doc/done | `↩` (or `⊙`/`◆`) @status:doc/done | `*` @status:doc/done |
| @fact:ROW-GLYPH-FLAGS Flags on / off @status:doc/done | `x` / `.` (ASCII ✗) @status:doc/done | `●` / `○` (or `✓`/`·`) @status:doc/done | `x` / `.` @status:doc/done |
| @fact:ROW-GLYPH-ROUNDED Rounded frame @status:doc/done | `╭ ╮ ╰ ╯` (✓ already) @status:doc/done | `╭ ╮ ╰ ╯` @status:doc/done | `+ - |` @status:doc/done |
| @fact:ROW-GLYPH-SQUARE Square frame @status:doc/done | — @status:doc/done | `┌ ┐ └ ┘` @status:doc/done | `+ - |` @status:doc/done |
| @fact:ROW-GLYPH-CLOSE Close affordance @status:doc/done | (none) @status:doc/done | `✕` (or `✖`) @status:doc/done | `x` @status:doc/done |
| @fact:ROW-GLYPH-SEPARATOR Separator / rule @status:doc/done | `—` @status:doc/done | `─` / `╌` @status:doc/done | `-` @status:doc/done |
| @fact:ROW-GLYPH-BAR Bar / indicator @status:doc/done | (none) @status:doc/done | `▁ ▂ ▃ ▄ ▅ ▆ ▇ █` / braille `⠁ … ⣿` @status:doc/done | `#` @status:doc/done |
| @fact:ROW-GLYPH-SCROLL Scroll marker @status:doc/done | (none) @status:doc/done | `▲` / `▼` @status:doc/done | `^` / `v` @status:doc/done |

@fact:NO-ASCII-PRIMARY **Rule (normative in PROP-037 §2.2 `#glyph-vocabulary`):** in the primary UI (Tier ≥ 1) there are no
`+`/`-`/`*`/`#`/`x`/`.` used as *semantic* glyphs. ASCII lives only behind the Tier 0 fallback. Menu
checked/unchecked already uses `◉`/`○` (kept); copy status uses `✓`/`✗` (kept). @status:doc/done

---

## 3. Palette system {#palette-tokens}

@fact:palette-definition A `Palette` is a data-driven mapping from **semantic role tokens** → `Color`. The roles (the full
set; everything a component can ask for): @status:doc/done

@fact:palette-roles-list `base` · `surface0` · `surface1` · `muted` · `subtext` · `text` · `accent` · `love` · `gold` ·
`foam` · `rose` · `selection` · `border` · `paper` · `button_on` · `button_off`. @status:doc/done

@fact:role-semantics-lead The role semantics: @status:doc/done
- @fact:ROLE-BASE `base` — the terminal/window ground. The main tree keeps the user's terminal background (so a
  themed terminal shows through); modals paint a solid `base` panel. @status:doc/done
- @fact:ROLE-SURFACES `surface0` / `surface1` — raised surfaces (status bar chrome / a subtle fill, an off-flag). @status:doc/done
- @fact:ROLE-MUTED `muted` — borders, disabled text, "why disabled" reasons. @status:doc/done
- @fact:ROLE-SUBTEXT `subtext` — secondary foreground (footer descriptions, header values). @status:doc/done
- @fact:ROLE-TEXT `text` — primary foreground. @status:doc/done
- @fact:ROLE-ACCENT `accent` — the brand: selection highlight, titles, the active key label, the cosmic violet. @status:doc/done
- @fact:ROLE-TONAL-ACCENTS `love` / `gold` / `foam` / `rose` — the four tonal accents (warnings / badges & highlights /
  static-load & links / secondary badges). @status:doc/done
- @fact:ROLE-SELECTION `selection` — the highlighted row (composed: `accent` ground + `base` text). @status:doc/done
- @fact:ROLE-BORDER `border` — the window/frame stroke (usually `muted`). @status:doc/done
- @fact:ROLE-PAPER `paper` — the detail-card panel, distinct from the tree beneath (a "paper card": light panel, dark
  text on dark themes; inverted on light themes). @status:doc/done
- @fact:ROLE-BUTTONS `button_on` / `button_off` — a focused vs unfocused button. @status:doc/done

@fact:five-palettes-lead **Five built-in palettes** (the full set — a worked example of mass theming): @status:doc/done

| Palette | Tone | `base` | `surface0` | `surface1` | `muted` | `subtext` | `text` | `accent` | `love` | `gold` | `foam` | `rose` |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| @fact:ROW-PAL-ROSE-PINE **Rosé Pine** @status:doc/done | dark (cosmic violet) @status:doc/done | `#191724` @status:doc/done | `#1f1d2e` @status:doc/done | `#26233a` @status:doc/done | `#6e6a86` @status:doc/done | `#908caa` @status:doc/done | `#e0def4` @status:doc/done | `#c4a7e7` @status:doc/done | `#eb6f92` @status:doc/done | `#f6c177` @status:doc/done | `#9ccfd8` @status:doc/done | `#ebbcba` @status:doc/done |
| @fact:ROW-PAL-MOCHA **Catppuccin Mocha** @status:doc/done | dark @status:doc/done | `#1e1e2e` @status:doc/done | `#313244` @status:doc/done | `#45475a` @status:doc/done | `#6c7086` @status:doc/done | `#a6adc8` @status:doc/done | `#cdd6f4` @status:doc/done | `#cba6f7` @status:doc/done | `#f38ba8` @status:doc/done | `#f9e2af` @status:doc/done | `#94e2d5` @status:doc/done | `#f5c2e7` @status:doc/done |
| @fact:ROW-PAL-MACCHIATO **Catppuccin Macchiato** @status:doc/done | dark @status:doc/done | `#24273a` @status:doc/done | `#363a4f` @status:doc/done | `#494d64` @status:doc/done | `#6e738d` @status:doc/done | `#a5adcb` @status:doc/done | `#cad3f5` @status:doc/done | `#c6a0f6` @status:doc/done | `#ed8796` @status:doc/done | `#eed49f` @status:doc/done | `#8bd5ca` @status:doc/done | `#f5bde6` @status:doc/done |
| @fact:ROW-PAL-FRAPPE **Catppuccin Frappé** @status:doc/done | dark @status:doc/done | `#303446` @status:doc/done | `#414559` @status:doc/done | `#51576d` @status:doc/done | `#737994` @status:doc/done | `#a5adce` @status:doc/done | `#c6d0f5` @status:doc/done | `#ca9ee6` @status:doc/done | `#e78284` @status:doc/done | `#e5c890` @status:doc/done | `#81c8be` @status:doc/done | `#f4b8e4` @status:doc/done |
| @fact:ROW-PAL-LATTE **Catppuccin Latte** @status:doc/done | **light** @status:doc/done | `#eff1f5` @status:doc/done | `#ccd0da` @status:doc/done | `#bcc0cc` @status:doc/done | `#9ca0b0` @status:doc/done | `#6c6f85` @status:doc/done | `#4c4f69` @status:doc/done | `#8839ef` @status:doc/done | `#d20f39` @status:doc/done | `#df8e1d` @status:doc/done | `#179299` @status:doc/done | `#ea76cb` @status:doc/done |

- @fact:ROSE-PINE-LOCKED **Rosé Pine is canonical-locked**: the eleven `Color::Rgb` values already in
  `crates/vibe-cli/src/commands/tree/tui/theme.rs` become the Rosé Pine `Palette` cell **unchanged**
  (R8). A snapshot test pins them. @status:doc/done
- @fact:CATPPUCCIN-MAPPING **Catppuccin** values are the canonical Catppuccin palette (the four flavours; Latte is the light
  one). Mapping: `accent`←mauve, `love`←red, `gold`←yellow, `foam`←teal, `rose`←pink, `muted`←overlay0,
  `subtext`←subtext0. @status:doc/done
- @fact:LIGHT-DARK-AWARENESS **Light/dark-awareness:** Latte is the light reference. A `Palette` carries an `is_light: bool`
  flag; the `paper` role and the `selection` composition invert against it (a light theme's "paper"
  card is a *dark* inset, a dark text on a light ground, etc.). The derived roles: @status:doc/done
  - @fact:DERIVED-SELECTION `selection` = `accent` ground + `base` text (bold) — high-contrast on every palette. @status:doc/done
  - @fact:DERIVED-BORDER `border` = `muted`. @status:doc/done
  - @fact:DERIVED-PAPER `paper` = `surface0` (raised, distinct from the tree ground); the detail-card text stays `text`. @status:doc/done
  - @fact:DERIVED-BUTTONS `button_on` = `accent` (focused), `button_off` = `surface1`. @status:doc/done

@fact:palette-is-setting The active palette is a `Model` field; through the settings system (Шаг 2) it persists across
L1/L2/L3 and is overridable at the CLI/env. @status:doc/done

---

## 4. Rendering tiers + degradation {#rendering-tiers}

| Tier | Condition | Palette | Frames | Indicators |
|---|---|---|---|---|
| @fact:ROW-TIER-3 **3** @status:doc/done | `$COLORTERM` ∈ {`truecolor`,`24bit`} @status:doc/done | full 24-bit RGB @status:doc/done | rounded `╭╮╰╯` @status:doc/done | braille / blocks @status:doc/done |
| @fact:ROW-TIER-2 **2** @status:doc/done | 256-colour (`$TERM` contains `256`) @status:doc/done | palette quantised to the 6×6×6 cube @status:doc/done | rounded @status:doc/done | blocks (8) @status:doc/done |
| @fact:ROW-TIER-1 **1** @status:doc/done | 16-colour ANSI @status:doc/done | ANSI role mapping @status:doc/done | rounded (if supported) / square `┌┐└┘` @status:doc/done | blocks (8) @status:doc/done |
| @fact:ROW-TIER-0 **0** @status:doc/done | explicitly dumb (`TERM=linux` / `dumb`) @status:doc/done | ANSI mono @status:doc/done | ASCII `+-\|` @status:doc/done | `#` @status:doc/done |

@fact:TIER-DETECTION **Detection (normative in PROP-037 §2.2 `#rendering-tiers`):** a **pure function** over the
environment — `detect_tier(colorterm: Option<&str>, term: Option<&str>) -> Tier`. `crossterm` exposes
no colour-count API, so detection is env-driven (`$COLORTERM` first, then `$TERM`); the TUI reads the
env once, at launch, in a sanctioned spot, and feeds the values in. The **default is Tier 3** — a
modern terminal is assumed truecolour even when it does not advertise the capability via env
(notably on Windows); the lower tiers are the **fallback** (the degradation path), reached only when
the environment explicitly advertises a lower capability (a 256-colour `TERM`, or an explicitly dumb
`TERM=linux`/`dumb`). The detected tier is overridable through the settings system. @status:doc/done

@fact:DEGRADATION-PROJECTION **Degradation = projection.** A `Theme` is built for Tier 3 (the full palette + rounded + braille),
then **projected** onto the detected tier: a 256-colour terminal gets each role quantised to the
nearest 6×6×6 cube colour; a 16-colour terminal gets the role mapped to one of the eight ANSI colours;
Tier 0 falls back to ASCII frames and `#` indicators. One source of truth (`Theme`), many projections. @status:doc/done

---

## 5. Window / modal aesthetics {#window-aesthetics}

@fact:window-composition-lead A window is not a fallback. The composition (the `ui::Window` component, PROP-037 §2.3): @status:doc/done

- @fact:WIN-SOLID-PANEL a **solid panel** background (filled `base`), floating over the terminal; @status:doc/done
- @fact:WIN-ROUNDED-FRAME a **rounded frame** (`╭╮╰╯` at Tier ≥ 2; the frame stroke is `border` = `muted`); @status:doc/done
- @fact:WIN-TITLE-CHIP a **title chip** — the window title rendered as a badge in the `accent` colour, not bare text; @status:doc/done
- @fact:WIN-PADDING-SHADOW **padding** inside the frame; an optional **shadow** (a low-alpha `▓`/`▒` band, or a 1-char offset)
  so the panel reads as raised; @status:doc/done
- @fact:WIN-CLOSE-AFFORDANCE a `[✕]` **close affordance** in the top-right. @status:doc/done

@fact:WINDOW-AT-TIER-1 Even at Tier 1 the window reads as a window — the rounded/square frame + filled panel + title chip
carry the "floating panel" reading without truecolor. The depth-2 modal stack (copy-settings →
file-path, PROP-037 §6, §10.5) draws each lower layer as a dimmed backdrop; the top window is the
only one that takes input. @status:doc/done

---

## 6. Spacing & rhythm {#spacing-rhythm}

@fact:spacing-vocabulary-lead A beautiful window is mostly **empty space used well**. The complaint that turns a "window" back
into "an error box on the worst terminal" is content jammed into a corner: a message flush against
the frame, a row of hints spilling off the left edge with no centre. The fix is a spacing
vocabulary held in three constants, so a re-space is one edit (the normative surface is
[PROP-037 §2.2.5](../modules/vibe-cli/PROP-037-tree-tui.md#spacing)): @status:doc/done

- @fact:SPACE-INTERIOR-PADDING **Interior padding (`PAD_X` = 2, `PAD_Y` = 1).** Every window frame holds its content off the
  border — two cells left and right, one row top and bottom. The body of a dialog floats: a blank
  row under the title chip, a blank row above the base, clear margins at the sides. This is what
  makes a `╭╮╰╯` frame read as a *raised panel* and not a box drawn around text. `ui::inner_pad` is
  the one helper; a dialog sizes its window to leave room for the padding, then lays its content
  into `inner_pad(inner)`. @status:doc/done
- @fact:SPACE-GROUP-GUTTER **Group gutter (`GUTTER` = 1).** A radio option inside a `Group` frame is inset one cell off the
  stroke, and its selection highlight bar is inset with it — so the `●`/`○` mark and the accent bar
  sit *inside* the group, never welded to its border. Nested frames (a window, then a group inside
  it) each keep their own breathing room, so the eye separates the levels. @status:doc/done
- @fact:SPACE-RHYTHM-CENTRING **Rhythm & centring.** Structure is read from separation: framed groups get a blank row between
  them; an inline run of hints gets a `•` separator with a space each side; and a row of controls
  is **centred in its area**. A left-jammed row reads as debug output — a website footer that slid
  to the left margin. The footer is the worked example: **two centred rows**, the F-keys above and
  the navigation below, each centred under the screen with `•`-separated rhythm. @status:doc/done

@fact:AIR-RULE The rule in one line: **content floats inside the frame with air on every side, and multi-element
rows are centred.** Air is not wasted space — it is the difference between a UI and a dump. @status:doc/done

---

## 7. ratatui-image readiness {#image-ready}

@fact:IMAGE-READINESS The primary UI is glyphs (portable). But the structure is **ready** for `ratatui-image` (future
package-preview images, a designed info-card image): placeholder slots and reserved image areas in
the layout, behind a capability flag. Sixel/Kitty are **not** for the primary UI — only an optional
image raster when the terminal advertises support. Reserved, not built (PROP-037 §12 non-goal). @status:doc/done

---

## 8. What becomes normative in PROP-037 §2.2

@fact:normative-pointer §2.2 carries the anchors — all five exist: `#palette-tokens` (the role set + the five canonical palettes),
`#glyph-vocabulary` (the replacement table + the "no ASCII in the primary UI" rule),
`#rendering-tiers` (the tier table + the pure `detect_tier` + the projection law), `#window-aesthetics`
(the window composition + "a window is not a fallback"), `#spacing` (interior padding, the group
gutter, and centred rhythm). This lore explains *why*; the contract carries the *values* the code
traces to. @status:doc/done
