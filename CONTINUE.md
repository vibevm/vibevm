# CONTINUE.md — cold-resume checkpoint (2026-07-16, мета-план CLOSED: settings system ✓ → TUI ✓ → settings UI ✓)

> `spec/WAL.md` is the canonical living state; if this snapshot and the WAL diverge, the WAL wins.

## TL;DR

**Весь текущий мета-план ВЫПОЛНЕН.** Три шага на `main`, floor-green throughout
(`self-check` all green, 347 vibe-cli tests, conform 0, specmap clean):

- **Шаг 2 — settings system (PROP-040)** — EXECUTED ранее (трёхуровневая модель,
  resolver, inspect-API, persist, `vibe prefs` CLI).
- **Шаг 3 — `vibe tree` TUI (PROP-037)** — EXECUTED, все 11 фаз P0–P10
  (`TREE-TUI-PLAN-v0.2.md`): визуальный язык (5 switchable палитр, glyph-vocab,
  4-tier rendering, `&Theme` threaded), `ui::` library, TreeShape×3, trees
  everywhere, `vibe_actions::keymap` + invoke-by-addr, detail Card, settings
  persistence, copy system с depth-2.
- **Шаг 4 — `vibe prefs` settings UI (PROP-041)** — EXECUTED, все 7 фаз S1–S7
  (`SETTINGS-UI-PLAN-v0.1.md`): page registry + settings tree, edit form
  (per-type + Configurable lifecycle + write-layer), provenance view, validation
  + lint, search (Search Everywhere over registry), vibe.prefs action surface.

**Единственный открытый item — owner visual sign-off** двух manual-тестов:
`spec/manual-tests/MT-02-vibe-tree-tui.md` (TUI) + `MT-03-vibe-prefs-tui.md`
(prefs UI). **AIUI — «потом»** (settings + actions AIUI-ready по дизайну, surface
не built — следующий крупный milestone, когда владелец решит).

## Where work stands

- Ветка **`main`**, дерево **чистое**, всё запушено в origin/gitverse.
- `bash tools/self-check.sh` **GREEN** (fmt / clippy `--all-targets` `-D warnings`
  / vibe check / conform / specmap-advisory / workspace tests / package self-traces).
- **Нет блокера.** 429 usage-limit (прерывал S2) — reset прошёл, работа завершена.

## The candidate next step (a RESUME is report-then-wait)

Владелец решает. Варианты по убыванию готовности:

1. **Owner visual sign-off** — прогнать MT-02 (`./target/debug/vibe tree`) и
   MT-03 (`./target/debug/vibe prefs ui`) глазами; отметить дату в файлах.
2. **AIUI surface** (PROP-039 §11.3) — следующий крупный milestone: headless
   driver над теми же `vibe-actions` (settings + tree actions AIUI-ready). Это
   «потом» по мета-плану; отдельная кампания.
3. **Pre-existing debt** (не блокирует): 34 specmap orphans в `vibe_spec`/
   `vibe_resolver` (PROP-035 provisional) — spec-tag когда формат стабилизируется.

## Non-obvious findings (do not re-learn)

- **Parallel delegates в общем worktree ГОНЯЮТСЯ** (P5+P7 конфликтовали через
  `git checkout`/`rm`). Пересекающиеся файлы — только последовательно; разные —
  параллельно ок (P6+P8, P3+P4, S3+S4, S5+S6).
- **Phase-verify должен использовать `clippy --all-targets`** (не только lib) —
  self-check использует `--all-targets` и ловит unused imports в test-модулях,
  которые lib-pass пропускает (один такой fix: `2592112`).
- **Conform-gated vibe-cli delegates ~20–40 мин** (conform-итерации); compat
  baseline EMPTY держится если каждый pub item имеет scope!/edge.
- **`schema/types.rs` у `vibe-settings` на 600** — не добавлять; при growth
  split в `types/` cells (один delegate превысил и обошёл — теперь约束: split
  нормально, не workaround).
- **PROP-041 anchors `REQ {#anchor}`** (без точки) — specmap resolver их НЕ
  подбирает (advisory +warnings, не блокирует). Heading-style работает.

## Repository map (this session)

- **`crates/vibe-cli/src/commands/tree/tui/`** — PROP-037 TUI (Шаг 3): `theme/`,
  `ui/` (9 компонентов), `shape.rs`, `flatten.rs`, `state.rs`, `render.rs`,
  `input.rs`, `dispatch.rs`, `keymap_bridge.rs`, `settings/`, `copy/`,
  `modal.rs`, `menu/`, `modes.rs`, `search/`.
- **`crates/vibe-cli/src/commands/prefs/tui/`** — PROP-041 settings UI (Шаг 4):
  `{mod,registry,page_tree,state,render,settings,input,lint,catalogue,dispatch,
  ui.rs}` + `form/` (`{mod,control,lifecycle,render,provenance,provenance_edit,
  validation}`) + `search/` (`{mod,providers,render}`).
- **`crates/vibe-actions/src/keymap.rs`** — pure resolver seam.
- **Specs/plans:** PROP-037 (§2.2 visual-language), PROP-040 (settings system),
  PROP-041 (settings UI); `TREE-TUI-PLAN-v0.2.md` (EXECUTED),
  `SETTINGS-UI-PLAN-v0.1.md` (EXECUTED); `spec/design/tui-visual-language.md`;
  `spec/manual-tests/MT-02-*` (TUI sign-off) + `MT-03-*` (prefs sign-off).

## Decisions in force

- **Мета-план CLOSED:** settings system (✓) → TUI PROP-037 (✓) → settings UI
  PROP-041 (✓). AIUI потом.
- **TUI visual language:** Unicode/truecolor; 5 палитр data-driven; rendering
  tiers; символы-only; `ratatui-image` readiness.
- **Boss rulings:** state/ split + formal ModalStack (D3/D7) — depth-2 покрыто
  pragmatically через captive fields (formal refactor deferred); palette/tier
  picker — через prefs UI (Шаг 4) или settings file/env.
- SDD + AI-Native Rust throughout; Rule 1–4 на месте.

## Recent commits (oneline — Шаг 4 tail)

```
2592112 fix(vibe-cli): drop an unused FieldControl import in provenance_edit tests
e95d8d5 chore(specmap): regenerate the snapshot (prefs search + action surface, S5+S6)
623eab8 feat(vibe-cli): prefs search + the vibe.prefs action surface (PROP-041 §7, §8; S5+S6)
d82b5b7 chore(specmap): regenerate the snapshot (prefs provenance + validation, S3+S4)
9fb48bc feat(vibe-cli): prefs provenance view + validation feedback (PROP-041 §5, §6; S3+S4)
ffbf853 chore(specmap): regenerate the snapshot (the prefs edit form, PROP-041 S2)
4438aee feat(vibe-cli): the prefs edit form — per-type fields + Configurable lifecycle (PROP-041 §4; S2)
21dfc0d chore(specmap): regenerate the snapshot (prefs TUI foundation, PROP-041 S1)
0128360 feat(vibe-cli): the vibe prefs TUI foundation — page registry + settings tree (PROP-041 §1–§3; S1)
… (Шаг 3 P0–P10 earlier: f875413 → 514f3b3)
```

## Quick-start

```sh
git status -sb && git log --oneline -5          # main, clean, pushed
bash tools/self-check.sh                         # floor GREEN
cargo test -p vibe-cli                           # 347 tests green
./target/debug/vibe tree                         # the TUI (PROP-037)
./target/debug/vibe prefs ui                     # the settings TUI (PROP-041)
```

## Pointer

- **Канонический живой state:** `spec/WAL.md`.
- **Plans:** `TREE-TUI-PLAN-v0.2.md` + `SETTINGS-UI-PLAN-v0.1.md` (оба EXECUTED).
