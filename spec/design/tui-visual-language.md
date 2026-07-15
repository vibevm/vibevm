# Design doc: TUI visual language (lore for PROP-037 §2.2)

_Status: **STUB** — vision зафиксирован (plan-сессия 2026-07-16); полная проработка — Phase 1
(`TREE-TUI-PLAN-v0.2`) boss-authored. Это lore (дизайн/эстетика/why); normative anchors — в
[PROP-037 §2.2](../modules/vibe-cli/PROP-037-tree-tui.md#theme), которые расширяются этим документом._

**Genre:** design doc (lore). **Контракт:** [PROP-037 §2.2](../modules/vibe-cli/PROP-037-tree-tui.md#theme)
(normative). **Связано:** `spec/terraforms/SETTINGS-SYSTEM-META-PLAN-v0.1.md` (палитра = настройка).

> **Owner vision (2026-07-16):** текущий TUI выглядит как ASCII-уродство (`+-|`, fold `+/-`, DAG
> `(*)`, flags `x`/`.`). Нужен ДЕЙСТВИТЕЛЬНО КРАСИВЫЙ TUI: Unicode box-drawing со скруглениями
> (`╭╮╰╯`), truecolor + формальная палитра-система (5 палитр, data-driven), Брайль/блоки для
> индикаторов, только символы для основного UI (переносимость SSH/tmux — **без Sixel/Kitty**), но
> `ratatui-image` readiness; стратегия деградации truecolor→256→16 / rounded→unicode→ASCII; красивые
> окна/модалки (не fallback-вид).

---

## 1. Принципы (эстетика)

_(раскрывается в Phase 1; кратко)_
- **Символьная точность.** Каждый glyph выбран осознанно — Unicode box-drawing/braille/blocks, не
  ASCII-заглушки. Скруглённые рамки по умолчанию; ASCII — лишь финальный уровень деградации.
- **Truecolor first.** 24-bit RGB как primary tier; палитра — data-driven semantic role tokens.
- **Переносимость.** Основной UI — только символы (работает через SSH/tmux/любой терминал).
- **Деградация graceful.** Худший терминал получает узнаваемый, но упрощённый вид (не сломанный).
- **Окна — окна.** Модалка читается как окно над терминалом (solid panel + frame + padding), не как
  «error на худшем терминале».

## 2. Glyph vocabulary {#glyph-vocabulary}

Все glyph'ы — константы в `Theme`, не захардкожены в строках. Текущие «уродства» → замены:

| Назначение | Сейчас | Целевой glyph (Tier 3) | Fallback (Tier 0 ASCII) |
|---|---|---|---|
| Tree-коннекторы `│├└─` | `│├└─` (✓ уже Unicode) | `│├└─` | `|-` |
| Fold indicator | `+` / `-` (ASCII ✗) | `▾` / `▸` (или `▼`/`▶`) | `+`/`-` |
| DAG-dedup marker | `(*)` (ASCII ✗) | `↩` (или `⊙`/`◆`) | `*` |
| Flags (on/off) | `x` / `.` (ASCII ✗) | `●` / `○` (или `✓`/`·`) | `x`/`.` |
| Rounded frame | `╭╮╰╯` (✓ уже) | `╭╮╰╯` | `+-|` |
| Close affordance | (нет) | `✕` (или `✖`) | `x` |
| Separator | `—` | `─` / `╌` | `-` |
| Bar indicator | (нет) | `▁▂▃▄▅▆▇█` / braille `⠁⠃…⣿` (2×4) | `#` |

**Правило:** в основном UI (Tier ≥ 1) нет `+`/`-`/`*`/`#`/`x`/`.` как смысловых glyph'ов.

## 3. Palette system {#palette-tokens}

`Palette` trait/value (data-driven): semantic **role tokens** → `Color`. Роли:
`base / surface0 / surface1 / muted / subtext / text / accent / love / gold / foam / rose`
(+ `selection / border / paper / button-on / button-off`).

**5 встроенных палитр** (полный набор = «пример массовой работы с темами»):
- **Rosé Pine** (cosmic violet) — **сохранить ТОЧНО**: текущие 11 `Color::Rgb` из
  `crates/vibe-cli/src/commands/tree/tui/theme.rs` (`BASE #191724` … `ROSE #ebbcba`) + gold highlights.
- **Catppuccin Mocha** (тёмный, canonical).
- **Catppuccin Macchiato** (тёмный, средний).
- **Catppuccin Frappé** (тёмный, светлый из тёмных).
- **Catppuccin Latte** (light → light/dark-aware theme).

Активная палитра — поле Model; через настройки (`SETTINGS-SYSTEM-META-PLAN` Шаг 2): L1/L2/L3.

## 4. Rendering tiers + degradation {#rendering-tiers}

| Tier | Условие | Палитра | Рамки | Индикаторы |
|---|---|---|---|---|
| **3** | `$COLORTERM`∈{truecolor,24bit} или crossterm 16.7M | full RGB | rounded `╭╮╰╯` | braille/blocks |
| **2** | 256-color | palette quantized → 256-cube | rounded | blocks |
| **1** | 16 ANSI | ANSI role-mapping | rounded (если поддерживается) / unicode box `┌┐└┘` | blocks (8) |
| **0** | dumb / `TERM=linux` / нет Unicode | ANSI mono | ASCII `+-|` | `#` |

**Detection:** crossterm color-count + `$COLORTERM`/`$TERM`; override в настройках. Degradation =
`Theme` проецируется на detected tier (один source of truth).

## 5. Window/modal aesthetics {#window-aesthetics}

Окно — не fallback. Composition:
- solid panel background (фон `base`), «парящий» над терминалом;
- rounded frame (`╭╮╰╯`), title как chip/badge (accent-цвет);
- padding внутри; опциональная тень (shadow glyph `▓`/`▒` low-alpha или 1-char offset);
- `[✕]` close affordance top-right.

Цель: даже на Tier 1 окно читается как окно, не как «упавшая рамка».

## 6. ratatui-image readiness {#image-ready}

Основной UI — символьный (переносимость). Но структура готова к `ratatui-image` для будущих картинок
(превью пакетов, info-card images): placeholder slots, image-area reservation в layout. Sixel/Kitty —
**не** для основного UI; только опциональный image-raster когда терминал поддерживает. Зарезервировано,
не built.

## 7. Что нормативно → PROP-037 §2.2 (Phase 1)

При расширении §2.2 anchors: `#palette-tokens` (роли + 5 палитр canonical-значения), `#glyph-vocabulary`
(таблица замены + правило «no ASCII in primary UI»), `#rendering-tiers` (tier-table + detection),
`#window-aesthetics` (composition + «window not fallback»). Lore (этот doc) объясняет why; контракт
несёт normative values.
