# TREE-TUI-PLAN v0.2 — полная реализация PROP-037 (`vibe tree` TUI), визуальный язык как primary axis

_Status: **ACTIVE** (2026-07-16). Supersedes `TREE-TUI-PLAN-v0.1.md` (wave 1 of the owner's pivot
onto the action system). v0.2 reframes the campaign with the **visual language as the primary axis**
and folds the now-landed settings system (Шаг 2) into §9. Each phase ends floor-green; the owner
visually signs off once at the end (P10)._

> **Verbatim mandate (owner, 2026-07-16):** «полностью реализовать PROP-037, кроме AIUI, с разворотом
> на визуальное качество» — ДЕЙСТВИТЕЛЬНО КРАСИВЫЙ TUI: Unicode box-drawing со скруглениями (`╭╮╰╯`),
> truecolor + формальная палитра-система (5 палитр, data-driven; Rosé Pine cosmic-violet сохранить
> ТОЧНО), Брайль/блоки для индикаторов, только символы для основного UI (переносимость SSH/tmux — без
> Sixel/Kitty), но `ratatui-image` readiness; стратегия деградации truecolor→256→16 / rounded→unicode→ASCII;
> красивые окна/модалки (не fallback-вид). AI-Native Rust для всего кода; спеки обновляются на ходу,
> недостающее исследуется и описывается, новое связывается с Rust-кодом через `specmark`.

**Контракт:** [PROP-037](../modules/vibe-cli/PROP-037-tree-tui.md) (каждая фича — гранулярный
addressable REQ, cite via `specmark`). **AIUI исключён** (§13.1 / PROP-039 §11.3) — «потом».
**Режим:** native Claude subagents реализуют ячейки по boss-спекам (REQ-anchor + сигнатуры +
acceptance-test + target); boss читает diff как PR + finish-tail + `self-check`. **Каждая фаза =
безопасная остановка** (floor-green commit).

---

## Контекст (почему это изменение)

`vibe tree` уже работает (PROP-036) и уже частично красив: tree-коннекторы `│├└─`, rounded borders
`╭╮╰╯` в modal/menu/search, `↑↓←→⇆` в footer, truecolor Rosé Pine (11 `Color::Rgb` в `theme.rs`).
Но есть ASCII-уродства (fold `+`/`-`, DAG `(*)`, flags `x`/`.`) и структурные PROP-037 gaps:
формальный визуальный язык отсутствует (одна hardcoded палитра, нет glyph-констант, нет `Theme`
value-типа, нет rendering tiers/degradation); four-layer MVC (§1) слит в `state.rs` 458/600; `ui::`
facade не существует (Window дублирован 3×); Tree widget + pipeline + 3 shapes (§3) не обобщены
(`flatten()` shape (a) захардкочен); trees in all modes (§4) FLAT; keymap resolver (§5/§13.3)
отсутствует в `vibe-actions`; modal stack (§6) = 3 поля; F2/F3/Esc (§7) частичны; detail card (§8) =
text blob; settings (§9) теперь = экземпляр системы; copy (§10) ~15%; discipline (§11/§13) частична.

## Ключевые архитектурные решения (D1–D7)

- **D1 — `Palette`/`Theme` value-система.** `Palette` trait/value (semantic role tokens → `Color`);
  `Theme { palette, glyphs, spacing, borders, tiers }` value-тип; style-fn остаются тонкими обёртками
  над значениями (call sites не меняются массово). Degradation = `Theme` проецируется на detected tier.
- **D2 — `ui::` facade + `ui::Window`.** extract из 3 дубликатов (modal/menu/search); рисует красивое
  окно (rounded + title-chip + padding + shadow) поверх `Theme`. Call sites никогда не трогают
  `rat_widget::` напрямую (§2.1).
- **D3 — ModalStack.** 3 поля → `Vec<ModalKind>` (push/pop/top-dispatch/backdrop). Двухэтап: P3 Vec≤1
  (behavior identical), P9b depth-2.
- **D4 — TreeShape pipeline.** `flatten(tree,folded,ordering)` → `+ shape: TreeShape, filter:
  &FilterSet`; root-set/visit-predicate/orphan-pass в `TreeShape` strategy; core walk reusable.
- **D5 — SubTables = одна таблица** (stacked trees через `RowNode::Subheader`, не новый рендер);
  fold-state глобально по package-id.
- **D6 — keymap resolver как cross-crate seam.** Новый `vibe_actions::keymap`: pure фн над абстрактным
  `vibe_actions::Key` → `NoMatch|NeedMoreChords|Found` (PROP-039 §9.2). **vibe-actions не зависит от
  crossterm** (`lib.rs` `#no-render-dep`); TUI владеет конверсией.
- **D7 — `state.rs` → `state/` каталог (превентивно).** `mod.rs`(App+rebuild)+`folds.rs`+
  `modal_stack.rs`+`settings.rs`+`theme_state.rs`.

## Фазировка (cold-executable; каждая ends floor-green + manual `cargo xtask specmap --check`)

| Фаза | REQ § | Deliverable | Делегат/Boss | Риск |
|---|---|---|---|---|
| **P0** prep | — | спайки (rat-widget wrap, TreeShape, arboard, modal-stack draw+input, **capability detection**, **Key-seam**, **Palette trait shape**, **tier-mapping**); **specmap regen + `chore(specmap)` commit** | **Boss** | R3 Key-seam; R4 specmap diff |
| **P1** visual language — **spec** | §2.2 | design-doc `spec/design/tui-visual-language.md` (lore); PROP-037 §2.2 → normative anchors `#palette-tokens`/`#glyph-vocabulary`/`#rendering-tiers`/`#window-aesthetics` | **Boss** | — |
| **P2** visual language — **system** | §2.2 | `Palette` trait + **5 палитр** (Rosé Pine сохранён точно + Catppuccin Mocha/Macchiato/Frappé/Latte); glyph vocabulary (заменить ASCII); `Theme` value-тип; rendering tiers + degradation + capability detection | **Boss** (trait/tier policy) + delegate (palette cells, glyph consts) | R1 budget; R3 detection; R8 fidelity |
| **P3** ui foundation | §1, §2.1–2.3, §6, §11 | `ui::Window` (extract 3×); `state/` split; `ModalStack` (Vec≤1); scope!→PROP-037; `Button`+`MsgDialog` wrappers | **Boss** (modal-stack, state split) + delegate (Window extract) | R1 budget; R2 regression |
| **P4** tree+pipeline | §3.1–3.3 | обобщённый flatten + `TreeShape` enum + 3 unit-теста shapes | **Delegate** (чистая фн) | R7 |
| **P5** trees везде | §4.1–4.4, §5.3 | `modes.rs` → обобщённый flatten (filter=LoadGroup); Shift+←/→ tabs; `TreeShape` в App | **Delegate** | R7 stacked trees |
| **P6** keymap+actions+quit | §5.1–5.2, §13.2–13.3, §7.4, §13.5 | `vibe_actions::keymap` (pure resolver); footer action-names (enabled only); **unify-dispatch** (invoke-by-addr, убрать string-match); полный каталог §13.5; Esc quit-confirm | **Boss** (resolver seam, dispatch) | R3 cross-crate; R5 attrs |
| **P7** components+F2+Soon | §2.4–2.10, §7.2 | `ui::RadioGroup/TextField/Group/ComingSoon/Card-skeleton`; F2 full; PNG→ComingSoon; split `menu.rs` | **Delegate** + boss (F2 spec) | R1 budget |
| **P8** detail card | §8 | `ui::Card` над Window: paper/bold/`✕`/wrap/per-line copy | **Delegate** | — |
| **P9a** settings | §9 | settings-system instance: palette choice, tier override, mode, per-context sort/shape/block-order; restore-on-launch (consumes `vibe-settings`, Шаг 2 готов) | **Delegate** | R1 |
| **P9b** copy system | §10.1–10.5 | per-screen providers; ↑F6 copy-settings modal; file-dest modal (depth-2); card markdown; PNG ComingSoon | **Delegate** + boss (depth-2 stack) | R2 depth-2; R6 popup nesting |
| **P10** discipline+sign-off | §11, §13.4, §12.2 | `#[spec(implements)]` повсеместно (с P4 как acceptance); i18n `Msg::resolve` подключён; specmap в self-check; **MT-02 manual test + owner visual sign-off** | **Boss** | R4/R5 polish |

**Dependency notes (no cycles):** визуальный язык (P1–P2) ВПЕРЕДИ компонентов. P5 до P6. `Button`+
`MsgDialog` в P3 (нужны quit-confirm в P6 и ComingSoon в P7). `#[spec(implements)]` — acceptance с P4.

## Риски + fallback (R1–R8)

- **R1 state.rs budget (458/600, zero-slack).** → P3 превентивный split в `state/`; P2/P9a новые поля в подcells.
- **R2 ModalStack migration.** → двухэтап: P3 Vec≤1 (identical), P9b depth-2.
- **R3 cross-crate keymap seam + capability detection.** → P0 спайки Key-токена (vibe-actions no-render-dep) и tier-detection (crossterm); resolver/detection — pure фн.
- **R4 specmap stale.** → P0 regen+`chore(specmap)` commit; manual `--check` в каждой фазе; P10 gate в self-check.
- **R5 zero-slack conform + `#[spec(implements)]`.** → атрибут как acceptance с P4; `anyhow` только на command edge.
- **R6 rat-widget popup nesting depth-2.** → P0 spike; fallback manual draw (Clear+Block, z-order снизу вверх).
- **R7 SubTables stacked trees.** → одна таблица + `RowNode::Subheader`; fold-state по package-id.
- **R8 palette fidelity** (сохранить Rosé Pine cosmic-violet ТОЧНО).** → P2: текущие 11 `Color::Rgb` из `theme.rs` = canonical Rosé Pine cell; snapshot-тест палитры.

## Verification (per-phase + финал)

```sh
# per-phase floor:
cargo fmt --all --check && cargo clippy --workspace -- -D warnings
cargo test -p vibe-cli -p vibe-actions
cargo xtask conform check                    # zero-slack (baseline EMPTY)
cargo xtask specmap --check                  # manual
bash tools/self-check.sh                     # all green

# visual language acceptance (P2):
cargo build -p vibe-cli && ./target/debug/vibe tree   # glyph-замены видны; palette-switchable

# финал (P10):
./target/debug/vibe tree                      # 5 палитр × окна × деревья × card × copy × Esc
```

## Critical files

- `crates/vibe-cli/src/commands/tree/tui/theme.rs` (P2 → `Palette`/`Theme` system; сохранить 11 Rosé Pine `Color::Rgb`)
- `crates/vibe-cli/src/commands/tree/tui/flatten.rs` (P2 glyph; P4 generalize `flatten()`/`walk()`)
- `crates/vibe-cli/src/commands/tree/tui/render.rs` (P2 flags; P3 draw-order; P6 footer)
- `crates/vibe-cli/src/commands/tree/tui/state.rs` (P3 split → `state/`; `rebuild` dispatch)
- `crates/vibe-cli/src/commands/tree/tui/{modal,menu,search/render}.rs` (P3 `ui::Window` extract)
- `crates/vibe-cli/src/commands/tree/tui/modes.rs` (P5)
- `crates/vibe-cli/src/commands/tree/tui/input.rs` (P3 modal-cascade; P6 resolver)
- `crates/vibe-actions/src/lib.rs` (P6 add `keymap` module)
- `spec/design/tui-visual-language.md` (P1) + `spec/modules/vibe-cli/PROP-037-tree-tui.md` §2.2 (P1)
- `specmap.toml` + `conform.toml` (root; namespace `vibevm`, gated vibe-cli, baseline EMPTY)

---

## Running ledger (заполняется по фазам)

_(пусто — кампания начинается с P0)_
