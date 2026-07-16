# CONTINUE.md — cold-resume checkpoint (2026-07-16, Шаг 3 TUI EXECUTED → Шаг 4 settings UI S1 done, S2–S7 pending)

> `spec/WAL.md` is the canonical living state; if this snapshot and the WAL diverge, the WAL wins.

## TL;DR

**Шаг 3 (the `vibe tree` TUI, PROP-037) ВЫПОЛНЕН ПОЛНОСТЬЮ** — все 11 фаз P0–P10
`TREE-TUI-PLAN-v0.2.md` на `main`, floor-green throughout (`self-check` all green,
241 vibe-cli tests, conform 0). **Шаг 4 (settings UI, PROP-041)** в процессе по
`SETTINGS-UI-PLAN-v0.1.md`: **S1 (page registry + settings tree) готов**, S2
(edit form) **прерван 429-лимитом** — partial откатан, дерево чистое.

**Активный блокер:** API usage limit (429, «Usage limit reached for 5 hour»,
**reset 2026-07-16 15:36:25**). До reset делегат-работа (native subagents) снова
упадёт; main loop тоже может быть близко к лимиту. **Нет кодового блокера** —
после reset продолжить S2.

## Where work stands

- Ветка **`main`**, дерево **чистое**, ~50+ коммитов этой сессии (Шаг 2 settings
  + Шаг 3 TUI + Шаг 4 S1). **Не запушено** — push при закрытии сессии (Rule 4).
- `bash tools/self-check.sh` **GREEN** (проверено после Шага 3; Шаг 4 S1 его не
  сломал — vibe-cli 273 tests, conform 0, specmap clean).
- **Шаг 3 TUI (PROP-037) — EXECUTED:** visual language (5 switchable палитр,
  glyph vocab ▾▸↩●○, 4-tier rendering, `&Theme` threaded, compat shim retired),
  `ui::` library (Window/Button/RadioGroup/TextField/Group/Card/MsgDialog/
  ComingSoon), TreeShape pipeline × 3, trees everywhere, `vibe_actions::keymap`
  resolver + invoke-by-addr dispatch, Esc quit-confirm, detail Card (wrapped),
  settings persistence (palette/tier/mode/sort/shape via `vibe-settings`),
  copy system (F6/Shift+F6, depth-2 copy-settings→FileDest). Sign-off:
  `spec/manual-tests/MT-02-vibe-tree-tui.md` (owner visual — единственный open
  item Шага 3).
- **Шаг 4 settings UI (PROP-041) — S1 done:** `commands/prefs/tui/` (page
  registry Configurable-EP, settings tree через visual language, origin hint,
  `vibe prefs ui` launch). S2 partial (form/) **откатан** к чистому S1.

## The active next step (candidate — a RESUME is report-then-wait)

**Шаг 4 — S2 (the edit form, PROP-041 §4).** Подробный план —
`spec/terraforms/SETTINGS-UI-PLAN-v0.1.md` (фазы S1–S7).

1. **S2 edit form** — right pane: per-type fields (bool→toggle, enum→RadioGroup,
   int/string→TextField, array→list, table→Group); Configurable lifecycle
   (`is_modified`/`apply`/`reset`); write-layer choice (L3 project / L1
   no-project, scope-forbidden refused); `applies` badge. Пометка: S2-делегат
   успел создать `prefs/tui/form/{control,lifecycle,mod}.rs` + раздуть
   `vibe-settings/src/schema/types.rs` до 616 (conform RED) до обрыва 429 —
   **откатано**; запустить заново после reset.
2. **S3 provenance** — «where does this value come from?» + layer-aware override.
3. **S4 validation** — inline schema-violation feedback + «check all layers».
4. **S5 search** — reuse `vibe-actions` Search Everywhere over the registry.
5. **S6 actions** — every command a `vibe.prefs` action, keymap-bound; modal
   stack popups; AIUI-clean.
6. **S7 sign-off** — MT-03 + owner visual.

После S7 — **мета-план завершён** (settings system ✓ → TUI ✓ → settings UI ✓).
AIUI — «потом».

## Non-obvious findings (do not re-learn)

### Delegation patterns (this session)
- **Параллельные delegates в общем worktree ГОНЯТСЯ** (P5+P7 конфликтовали —
  каждый откатывал другого через `git checkout`/`rm`). Запускать delegates с
  пересекающимися файлами **только последовательно**. Разные файлы — параллельно
  ок (P6+P8, P3+P4 прошли).
- **Conform-gated vibe-cli delegates работают ~20–40 мин** (conform-итерации).
  floor-gate verification: `cargo check` + `clippy -D warnings` + `cargo test` +
  `cargo xtask conform check` (baseline EMPTY) + `cargo xtask specmap --check`.
- **delegate оставляет diff для boss** (не коммитит) — boss verify + commit.
  specmap.json delegate уже регенерит — boss коммитит как `chore(specmap)`.
- **AI-Native**: `#[allow(dead_code)]` для Phase-staged API — принятая конвенция
  (theme/, P4 TreeShape variants, P9a `set_shape`). conform baseline EMPTY
  держится если каждый pub item имеет scope!/edge.

### TUI architecture (Шаг 3)
- **rat-salsa 4.x** — не trait-based: `run_tui(init, render, event, error, ...)`
  с fn pointers. `Control::Changed` на resize ( repaint). AppEvent wraps
  crossterm Event.
- **vibe-actions no-render-dep** — `keymap` модуль (Key/KeyCode/KeyModifiers +
  pure resolver) cross-crate seam; TUI владеет crossterm→Key bridge
  (`keymap_bridge.rs`). `KeyModifiers` (не `Modifiers` — коллизия с
  `search::Modifiers`).
- **state.rs 600/600** (P9b) — на грани бюджета. Новый prefs TUI — отдельный
  `PrefsApp` (не shared), свои cell-файлы.
- **PROP-041 anchors `REQ {#anchor}`** (без точки) — specmap resolver их НЕ
  подбирает (advisory, +1 warning; не блокирует). Heading-style `## … {#anchor}`
  работает.

### 429 limit
- «Usage limit reached for 5 hour» — reset **2026-07-16 15:36:25**. Delegate
  spawns падают; main loop работал до последнего. После reset — продолжить.

## Repository map (новое этой сессии)

- **`crates/vibe-cli/src/commands/tree/tui/`** — PROP-037 TUI: `theme/`
  (`{mod,palette,palettes/{rose_pine,catppuccin,mod},glyphs,tier}`),
  `ui/` (`{mod,window,button,radio_group,text_field,group,card,msg_dialog,
  coming_soon}`), `shape.rs` (TreeShape), `flatten.rs`, `state.rs`,
  `render.rs`, `input.rs`, `dispatch.rs`, `keymap_bridge.rs`, `settings/`
  (vibe-settings bridge), `copy/` (`{mod,settings,file_dest}`), `modal.rs`,
  `menu/` (`{mod,sort,mode}`), `modes.rs`, `search/`.
- **`crates/vibe-cli/src/commands/prefs/tui/`** — PROP-041 settings UI (S1):
  `{mod,registry,page_tree,state,render,settings,input}` + `../ui.rs` launch.
- **`crates/vibe-actions/src/keymap.rs`** — pure resolver seam.
- **Specs/plans:** `PROP-037` (TUI, §2.2 visual-language anchors), `PROP-041`
  (settings UI), `PROP-040` (settings system); `TREE-TUI-PLAN-v0.2.md` (EXECUTED),
  `SETTINGS-UI-PLAN-v0.1.md` (S1 done, S2–S7 pending), `SETTINGS-SYSTEM-*`;
  `spec/design/tui-visual-language.md` (lore); `spec/manual-tests/MT-02-*`
  (TUI sign-off).

## Decisions in force

- **Мета-план:** settings system (✓) → TUI PROP-037 (✓ EXECUTED) → settings UI
  PROP-041 (S1 ✓, S2–S7 pending) → DONE. AIUI потом.
- **TUI visual language:** Unicode/truecolor primary; 5 палитр data-driven;
  rendering tiers; символы-only; ratatui-image readiness.
- **Boss rulings recorded:** state/ split + formal ModalStack (D3/D7) deferred
  (depth-2 covered pragmatically via captive fields); palette/tier picker UI
  (S2/S6 territory) pending.
- SDD + AI-Native Rust throughout; Rule 1–4 на месте.

## Recent commits (oneline — Шаг 3 + Шаг 4)

```
40201dc docs(wal): checkpoint — tree TUI (Шаг 3) EXECUTED, settings UI (Шаг 4) in progress
21dfc0d chore(specmap): regenerate the snapshot (prefs TUI foundation, PROP-041 S1)
0128360 feat(vibe-cli): the vibe prefs TUI foundation — page registry + settings tree (PROP-041 §1–§3; S1)
f875413 docs(terraform): SETTINGS-UI-PLAN v0.1 — the vibe prefs TUI (PROP-041, Шаг 4)
514f3b3 docs(tree-tui): MT-02 visual sign-off + mark TREE-TUI-PLAN v0.2 EXECUTED
4be7adf chore(specmap): regenerate the snapshot (the copy system, P9b)
bdc88fd feat(vibe-cli): the copy system + depth-2 copy-settings → file-dest (PROP-037 §10; P9b)
8b57b13 chore(specmap): regenerate the snapshot (settings cell + theme threading, P9a)
b019e88 feat(vibe-cli): settings persistence + the live, switchable theme (PROP-037 §9, §2.2; P9a)
b7c1374 chore(specmap): regenerate the snapshot (keymap dispatch + detail card, P6+P8)
3641aff feat(vibe-cli): the detail card as a wrapped form (PROP-037 §8, §2.9; P8)
43a3c60 feat(vibe-cli): keymap resolver + invoke-by-addr dispatch + Esc quit-confirm (PROP-037 §5, §13; P6)
db81e03 chore(specmap): regenerate the snapshot (ui components + F2 menu, P7)
f9de0e8 feat(vibe-cli): ui components + the F2 sort/shape menu (PROP-037 §2.6–2.10, §7.2; P7)
f77957a chore(specmap): regenerate the snapshot (modes scope → PROP-037#modes, P5)
a503339 feat(vibe-cli): trees in every mode + Shift-arrow tab nav (PROP-037 §4, §5.3; P5)
c7ad9a0 docs(terraform): TREE-TUI-PLAN ledger — P3 foundation + P4 pipeline DONE
f02112f chore(specmap): regenerate the snapshot (ui foundation + tree-shape pipeline)
5d808a2 feat(vibe-cli): ui foundation + the tree filter/shape pipeline (PROP-037 §2.3–2.10, §3.2–3.3; P3+P4)
3c828d2 docs(terraform): TREE-TUI-PLAN ledger — P2 visual-language system DONE
… (P2 → P0 earlier)
```

## Quick-start

```sh
git status -sb && git log --oneline -5          # main, clean; ~50 commits this session
sed -n '1,60p' spec/terraforms/SETTINGS-UI-PLAN-v0.1.md   # Шаг 4 plan (S1 done, S2 next)
bash tools/self-check.sh                         # floor GREEN (verified post-Шаг 3)
cargo test -p vibe-cli                           # 273 tests green
./target/debug/vibe tree                         # the TUI (PROP-037, Шаг 3)
./target/debug/vibe prefs ui                     # the settings TUI (PROP-041 S1 — tree only; form is S2)
# resume S2 after the 429 reset (2026-07-16 15:36:25):
#   re-launch the S2 edit-form delegate (spec in SETTINGS-UI-PLAN v0.1 §S2)
```

## Pointer

- **Канонический живой state:** `spec/WAL.md` (верхняя `_Updated:` строка).
- **Шаг 4 plan:** `spec/terraforms/SETTINGS-UI-PLAN-v0.1.md`.
- **Шаг 3 plan (EXECUTED):** `spec/terraforms/TREE-TUI-PLAN-v0.2.md`.
