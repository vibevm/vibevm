# CONTINUE — холодный резюм (чекпойнт 2026-08-24, перед компактом; СЕРЕДИНА ПЕРЕЕЗДА)

> WAL (`vibevm/vibespecs/WAL.xml`) — канонический живой статус.
> **ВНИМАНИЕ: рабочее дерево несёт НЕЗАКОММИЧЕННЫЙ физический переезд
> (~6.5k renames/правок). Это НАМЕРЕННО: панель ещё не зелёная, красное
> не коммитим. Диск переживает компакт — ничего не потеряно.**

## TL;DR

1. **XML-переход (goal-половина 1) — ЗАВЕРШЁН И ПОСАЖЕН.** PROP-051 BUILT,
   план CONVERT-SOURCE ЗАВЕРШЁН, K1…K7+K6.5 (2170 ячеечных имён, оба
   линта anchored-when-marked-на-ячейки), корпус 871 файл в XML, панели
   были all green, зеркала синхронны. Посажено вплоть до `373683b7` /
   `265af3dc` / `011e6614` (R2) / `12b53193` (PROP-052+план).
2. **Раскладка vibevm/ (goal-половина 2) — ФИЗИЧЕСКИ ВЫПОЛНЕНА, НЕ
   ПОСАЖЕНА.** Сделано на диске: 4 корня хоста → `vibevm/{vibespecs,
   vibepacks,vibedeps,vibefacts}`; spec→vibevm/vibespecs внутри всех 86
   пакетов + вложенные vibedeps 4 пакетов; `USE_NEW_LAYOUT=true`
   (crates/vibe-core/src/layout.rs); свип путей 497 файлов/3304 + 26
   (sync-engines.toml) + 8 (Cargo.toml, exclude=["vibevm"]); манифесты
   пакетов: boot_snippet source → `vibevm/vibespecs/boot/*.xml` (70
   файлов); фикстурные пакеты fixtures/registry — мигрированы (L4).
3. **Сейчас:** R6-панель выгребает слои полусвипнутых скаффолдов —
   фоновая задача blwzg7ywa (`…\tasks\blwzg7ywa.output`). Метод слоя:
   красный тест → фикстуру/скаффолд на `vibe_core::layout::current_*`
   (или литерал новой формы с комментом-ссылкой в крейтах без vibe-core)
   → targeted test → снова панель.

## Уже ДОКАЗАНО на новой раскладке (не перепроверять зря)

- `cargo xtask specmap`: **0 unresolved** — адреса пережили переезд (L1).
- Идемпотентность: двойная установка, дифф lock == только `generated_at`.
- `vibe why org.vibevm.world/wal` — живой root-edge chain.
- `vibe check`: 0 errors (2 softened-concept warnings — штатные F7).
- Слоты зеркалят новую раскладку (`…/wal/0.2.0/vibevm/vibespecs/boot/10-flow-wal.xml`).

## Выгребенные слои фоллаута (НЕ переделывать)

Cargo.toml (path-депсы+exclude) · sync-engines.toml (26) · движок:
Config::default→vibevm/vibespecs + двухпрефиксный canonical_doc_path
(R2E-воркер, трансплантирован в vibevm/vibepacks-путь, 143/143) ·
vibe-core query.rs slot_manifest_path · vibe-trace целиком (vibedeps_dir()
многокомп., все скаффолды) · vibe-check oracle + wal-чеки двухформенные ·
cli-фикстуры: facts, mcp_path_parity, progress_sidecar, spec_format,
workspace_publish, explain_foreign, refactor e2e (+сообщение spec-src из
layout) · vibe-install: incremental_in_place, slot_integrity_verify
(fixture_pkg, vibefacts-оверлей, **slot_verify.rs::live_overlay_hash —
многокомпонентный дерайвер корня**), store_materialisation seed ·
vibe-spec compile.rs — литералы новой формы (нет vibe-core) · панель
нарратит 48 шагов (счётчик/таймстампы/heartbeat 30s) — запрос владельца.

## Незакрытые хвосты (порядок работы после компакта)

1. **Дожать панель**: tail blwzg7ywa; красный слой → тем же методом.
   Кандидаты-остатки: vibe-mcp tools_oracle (патчен, не гонялся отдельно),
   vibe-publish/workspace tests, cli_spec_format e2e глубже, engine-гейты
   пакетных воркспейсов (шаги 7-10 панели ещё не достигались!).
2. **По зелёной панели — ПОСАДКА** (3+ коммитов): (а) физический переезд+свип
   (renames spec→vibevm/... + текстовые правки + манифесты + фикстуры), (б)
   продуктовые починки (layout flip, query/trace/install/engine, panel-нарация,
   Cargo.toml), (в) chore(install) перегенерированный мир+lock; затем статусы
   плана (R4/R5→done), зеркала.
3. **R6-доказательства** (PROP-052 ##RELAYOUT-PROOF, частично сделаны):
   остались — clean-room смоук (vibe init→install→check в tempdir), ноль-греп
   старых корней вне layout-модуля/санкционированных, скрипт-резолюция
   INDEX-целей, агентская проба маршрутизации (cold-читатель по CLAUDE.md
   находит vibevm/vibespecs/boot). Потом R7: статусы, BACKLOG-остатки
   (research/rust-demo path-dep на свой vibedeps — вне периметра, записать),
   WAL/CONTINUE, зеркала.
4. **Обещано владельцу**: прогресс-нарация `cargo xtask sync-engines`
   (по-сетно `[k/9] … crates→targets`) — сделать при первой свободной панели.
5. Уроки в `campaigns/packages-2026-09/SUBAGENT-LAUNCHERS.md` уже дописаны
   (PID-точные убийства+MSIX, потолок 2 холодных cargo, Monitor-не-sleep,
   touch-пакета при провизии); страж MSIX в codexrunner живой.

## Механика воркеров (живое)

Лейны: codexrunner (nvm4w codex-cli 0.148.0, страж WindowsApps) + claudez.
Возможных живых нет — все посажены: K1/K1b/K25/K26/K65/R1/R2A-E (архивы+
меты в `C:\Users\olegc\git\v\cache\agents\sorted\<ID>/`). Worktrees
`.wt/{K1-CONVERT,R1-LAYOUT,R2C-CHECKS,R2D-TAILS}` — на старой базе, их
диффы ПРИМЕНЕНЫ; после посадки переезда — снести без сожаления.

## Ловушки этой стройки (не наступать повторно)

`cmd | tail` маскирует код (использовать `>file; echo EXIT=$?`) ·
polusweep-жанр: скаффолд наполовину новый → os error 3 (лечить
КОГЕРЕНТНОСТЬЮ через layout) · `create_dir` не умеет вложенный
`vibevm/vibepacks` → `create_dir_all` · Windows user-mapped lock (1224)
на STATIC/INDEX → ретрай · generated-свап локи → снести `generated.new-*`
· питон-replace по бэкслэшам мимо → Edit-инструментом · `spec://` в
r4_sweep неуязвим (`spec/` требует слэш сразу после spec).

## Быстрый старт после компакта

```sh
tail -30 "C:\Users\olegc\AppData\Local\Temp\claude\C--Users-olegc-git-v-vibevm\65ac15d7-9831-4db5-8a5b-736924494335\tasks\blwzg7ywa.output"
git status --short | head    # незакоммиченный переезд — это норма
bash tools/self-check.sh     # нарратит шаги [N/48 HH:MM:SS]
```
Свип-скрипт: `$CLAUDE_JOB_DIR/tmp/r4_sweep.py` (условный по цели).
