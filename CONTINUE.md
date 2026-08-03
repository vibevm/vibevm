# CONTINUE — cold-resume snapshot (2026-08-04, wind-down №7: батч 2 волны Б ЗАКРЫТ)

**Не цитируй числа отсюда — меряй:**
`python campaigns/packages-2026-09/tasks/summary.py` ·
`python campaigns/packages-2026-09/tasks/drift-registry.py`.
`spec/WAL.md` переписан этой же сессией и **суперсидит** этот снапшот.

## TL;DR

Один длинный автономный прогон **построил И закрыл батч 2 волны Б** (паритет
гейтов) — 27 коммитов, панель зелёная на каждой посадке, реестр двинулся
**88/179 → 87/176 (98.1 %)**. Движок получил три новых правила
(`go-seam-error-cites-req` обе половины, `ts-seam-error-cites-req`,
`go-conformance-assertion` gated) + `[rust] floor_disable`; экстракторы их
эмитят; все смонтированы и показаны на фикстурах; принцип паритета поднят в
манифест и цитируется тремя гайдами; луп B-035 №2 перекроил паритет-таблицу;
семья F-185 пересужена `confirmed`; backlog-строки помечены. **Остаток батча 2
— только гигиена реестра** (ре-seal S4-правленных гайдов). **Дальше — батч 3**
(развилка №1 карты приходит с B-038 — настоящий стоп владельцу). Мандат Б/В/Г
стоит, паузы нет. Зеркала выкачаны этим wind-down'ом.

## Где стоит работа

- Ветка `main`; после `cargo xtask mirror` (этот wind-down) — синхронна с
  зеркалами. Дерево чистое. `.wt/E12-S4-DOCS` — handle-locked leftover
  (gitignored, git-prune чист, снести позже).
- Панель зелёная — «self-check: all green» прочитан хвостом (bare-форма).
- Реестр: **87 obligations / 176 drift verdicts, 98.1 % confirmed** — меряй
  командами. F-185 закрыт пересудом.
- Открытые аудит-строки: `AUDIT.md` §2026-08-03 (cargo-outdated, dead_code
  shadow; DBT-0023).
- Активного блокера НЕТ. Ближайшее настоящее решение владельца — **развилка №1
  карты (computed-names)**, приходит в батче 3 с B-038.

## ПЕРВОЕ ДЕЛО НОВОЙ СЕССИИ — батч 3 (мандат стоит, паузы нет)

**Внимание:** промт продолжения НЕ обязан быть слово-в-слово; мандат Б/В/Г
(§7 LOG «Хочу все остальные волны сделать») живой. Порядок:

0. **(Гигиена, оппортунистически, не гейтит)** ре-seal S4-правленных гайдов +
   манифеста: verdict-батч (правленые/новые якоря → `confirmed`; правленые:
   go-GUIDE `##CONFORMANCE-IS-MADE-LOUD`/`##SWEEP-CENSUS-REGRESSIONS`,
   rust-GUIDE `##SCAFFOLD-F-STRUCTURED-DIAGNOSTICS`; новые: ts-GUIDE
   `##TS-SEAM-ERROR-CITES-REQ-IS-BUILT`, манифест 4 `##PARITY-*` клаузы) →
   `merge-verdicts.py <batch> --force` → `vibe progress seal <пути>`. **Образец
   батча — `campaigns/packages-2026-09/tasks/evidence/batch-E12-F185-rejudge.json`.**
   Присоединяется к пяти незапечатанным observed-файлам (PROP-035, PROP-029,
   три дизайн-дока — теперь четыре с `seam-error-and-assertion-parity.md`).
1. **Батч 3** (карта §4, `TOOLING-MAP.md#waves`): каденс тот же — **цензус**
   (read-only claudez-воркеры, образец E8/E11-цензусов в `harvest/`) →
   **босс-дизайн** (`spec/design/`, образец `gate-parity-config.md` /
   `seam-error-and-assertion-parity.md`) → **claudez-стройки** (закон
   транспорта — `SUBAGENT-LAUNCHERS.md` ЦЕЛИКОМ + `SUBAGENT-MODE.toml` перед
   КАЖДЫМ fan-out). Состав: **B-036** (conform-правило «инварианты не тонут в
   середине файла» — `BACKLOG.md {#b-036}`), **B-037** (кастомные REQ-линты:
   dylint-класс + typescript-eslint — `{#b-037}`), **B-038** (pending-карты
   R-060 + closed-vocabulary-naming — `{#b-038}`). **Развилка №1 карты
   (computed-names, `TOOLING-MAP.md#forks` №1) приходит С B-038 — НАСТОЯЩИЙ
   стоп владельцу** (дерево-вопрос, AskUserQuestion с рекомендацией — форма
   работает отлично).
2. **Батч 4** (B-025 mark-don't-suppress → последний якорь F-146; B-026
   SARIF-ингест → F-206) → выход **M-PARITY** (паритет-таблица без языковой
   ячейки слабее Rust без записанной причины; после батча 2 остаются строки 6
   и 8/12 — Go flag-rule и Go-floor `./...`-остаток).
3. **Волна В** (карта): B-013 done → один формат-чейндж (B-019а+B-016.1+B-017,
   B-024 рядом) → B-018.1/.2 → B-018.4+B-016.2 → B-020+B-021 (B-014 там; B-020
   разблокирует четыре interim'а LEDGER-INTENT) → выход M-ASK+M-DRIFT. **Волна
   Г** оппортунистически: B-040 (цензус снят — `harvest/g1-b040-seams-census.md`),
   B-005, F-132-схемы, B-010-check.

## Что построено батчем 2 (карта посадки)

- **Движок (canonical `core-ai-native/v0.8.0`, вендорится ×6):**
  `Fact::GoConformance {seam, impl_type, line, in_test}` +
  `Fact::TsSeamError {symbol, cites_req, line, in_test}` + новый `GoUnsafe`
  kind `seam_error_message_no_req`; правила `rules/go_parity.rs`
  (`GoSeamErrorCitesReq` — обе половины, per-half отпечатки; `GoConformanceAssertion`
  — **gated**-предикат `new(cells_dir, gated)`) и `rules/typescript_parity.rs`
  (`TsSeamErrorCitesReq`); `RustConfig.floor_disable: Vec<FloorDisable>`.
- **Экстракторы/мосты:** go-extract читает тела `Error()` (маркер `spec://`/
  `violates REQ`; якорь message — строка МЕТОДА, не типа) + `var _ Seam =
  (*Impl)(nil)`; ts-extract ловит Form-1 union-ошибки; мосты — новые `RawFact`
  арма. `tools/go-extract/go.mod` создан (для `go test`; materialise независим).
- **Драйверы:** go монтирует seam-error (всегда) + conformance (условно
  cells_dir, gated); ts монтирует seam-error; rust-floor чтит `floor_disable`.
- **Фикстуры:** clean go-greet получил seam `Greeting` + ассерцию (комплаент);
  dirty plan красный по обеим half + conformance (gate = 12); голден
  `specmap.json` регенерирован.
- **Дисциплина:** манифест §4 несёт `##PARITY-ACROSS-PROJECTIONS` (+3 клаузы),
  три гайда цитируют.

## Не-очевидные находки прогона (durable-уроки, в WAL #constraints)

- **Завершение воркера — по НОТИФИКАЦИИ фоновой задачи, НЕ по маркеру
  `TASK-DONE`** (echo'нулся рано, продолжил в main-repo — мисджаджил как failed).
- **Doc-воркер с незаписываемым worktree пишет в main-repo** — ревьюй `git diff`
  хоста как обычно.
- **Вынос вида-находки из умбреллы в своё правило** ломает КАЖДЫЙ by-rule count
  тест (gate-count, TCG-parity) — монтаж + правка счётчиков той же посадкой.
- **Новое gate-правило, требующее комплаентных образцов**, каскадит в фикстуры
  + init-шаблоны + голдены + тесты. Специмап-голден: `run_specmap_go(root, false)`
  пишет (CLI нет — одноразовый bless-тест).
- **Предикат conformance = gated-ячейки** (не «каждая» — бесшовные/exempt вне).
- **Message-маркер = `spec://` ИЛИ `violates REQ`** (Go рендерит URI из поля).
- **Кэш экстракции** протухает по (контент, версия фронтенда); правка ЛОГИКИ
  внутри версии не инвалидирует → чисти `fixtures/*/target`.
- **`Fact`-ВАРИАНТ = кросс-пакетная рябь** (Rust FE сорт + 3 health-цензуса +
  мосты); `Fact`-KIND — нет.
- **`merge-verdicts.py` берёт verdict-БАТЧ JSON** (`{batch, cluster, files:
  {path:{ANCHOR:{v,ev}}}}`), НЕ голый флаг; рефьюзит restate без `--force`;
  `vibe progress seal` рефьюзит файл с несуждёнными маркерами.
- **Воркеры хорошо эскалируют суждение** — приняты 2 коррекции, бывшие
  правильными эскалациями воркера (`spec://`-vs-`REQ`, `Vec<String>`).

## Карта репозитория (верхний уровень)

- `spec/` — PROP/FEAT-контракты (`common/`, `modules/`), `boot/` (загрузка
  PROP-009), `design/` (рационали — non-normative), `terraforms/`
  (кампанийные планы), `WAL.md`.
- `campaigns/packages-2026-09/` — активная кампания: `harvest/` (цензусы +
  паритет-таблицы), `tasks/` (`summary.py`, `drift-registry.py`,
  `merge-verdicts.py`, `evidence/`), `run/` (реестр-состояние — генерится),
  `SUBAGENT-LAUNCHERS.md` + `SUBAGENT-MODE.toml` (закон транспорта воркеров),
  `OBLIGATIONS.md`, `BATCH-PLAN.md`.
- `packages/org.vibevm.ai-native/` — дисциплина: `core-ai-native/v0.8.0/`
  (нейтральный движок conform+specmap + манифест; вендорится ×6),
  `{rust,go,typescript}-ai-native-lang/` (стек-гайды, экстракторы, драйверы,
  CLI, фикстуры), `*-mcp/` (MCP-твины, синкаются). `fractality/` — специспейс.
- `packages/org.vibevm.world/` — флоу (git-practices, delegation-*, spec-genres…).
- `crates/`, `xtask/` — хост-инструментарий (`vibe`, `cargo xtask
  sync-engines`/`mirror`/`conform`). `vibedeps/` — рематериализованные копии
  installed-пакетов. `.wt/` — worktree'ы воркеров (gitignored).
- Корень: `BACKLOG.md` (P1/P2/P3), `TOOLING-MAP.md` (волны+развилки),
  `AUDIT.md`, `TASKS.md`, `ROADMAP.md`, `CLAUDE.md`/`AGENTS.md`/`GEMINI.md`.

## Действующие архитектурные/полит-решения (в силе)

- **Принцип паритета — закон дисциплины** (манифест `##PARITY-ACROSS-PROJECTIONS`):
  ни одна проекция не слабее без записанной причины.
- **Конфиг пер-язык** (v2): корень = `max_file_lines` + `[rust]`/`[go]`/
  `[typescript]` одной формы, нейтральный `gated`, `[[<lang>.exempt]]` /
  `[[<lang>.floor_disable]] {step,reason}`; единицы — crate/package/cell;
  старые плоские ключи — надгробия. fractality-конфиг СОЗНАТЕЛЬНО flat (0.7.0).
- **Хост — пакет** `org.vibevm.core/vibevm` (B-031); `spec://vibevm/…` парсится
  и НЕ резолвится; старую форму не возвращать.
- **BUILD-FIRST** (не ослаблять правило за неиспользованность); **T/F/G вне
  добра**; публикация — после рефакторинга; версии не бампать до
  пред-публикации; замеров нет — стоячий ответ.
- **Делегация:** claudez-воркеры (GLM-5.2), закон транспорта
  `SUBAGENT-LAUNCHERS.md`; ревью/вердикты/коммиты — босс; Rules 1–4 биндят
  делегированное как прямое (человеческая атрибуция — без AI-трейлеров).
- **Роллаут — ТОЛЬКО `cargo xtask mirror`** (fast-forward, никогда `--force`).

## Недавняя цепочка коммитов (последние 25, сверху — свежие)

```
fd562b7c docs(continue): batch 2 closed — next is batch 3 (fork №1 with B-038)
ad3c1d7e docs(wal): batch 2 CLOSED — F-185 re-judged confirmed, registry 87/176
d0c0fddd docs(backlog): batch 2's builds land — B-033/B-030/B-049 done, the parity loop's pass №2
3fae12d7 docs(campaign): F-185's family re-judged confirmed — the go seam-error rule is built (registry 87/176)
7e946e77 docs(continue): correct — S4 and the loop done, the F-185 close recipe
aeb6c703 docs(wal): correct the checkpoint — S4 and the B-035 loop landed, F-185 re-judge remains
a4e40c4a docs(campaign): the §7 LOG takes the batch-2-built entry
6a7e40f7 docs(campaign): the B-035 parity loop re-cuts after batch 2 — rows 1/7/13 closed
0bdffc21 chore(packages): rematerialise the guide docs
ee2df2ce docs(packages): the guides describe the built seam-error and conformance rules and cite the parity law
94e6db0e chore(packages): vendor and rematerialise the batch-2 rule mounts
32aba0ab feat(rust-ai-native): the floor honours [rust] floor_disable
f63c1d32 feat(typescript-ai-native): the ts gate mounts the seam-error rule
d09e2a19 feat(go-ai-native): the go gate mounts the conformance-assertion rule
bd4291d5 feat(core-ai-native): the conformance-assertion rule scopes to the gate list
0393ce91 chore(packages): sync the extractor twins and rematerialise
a5ba2b0b feat(typescript-ai-native): ts-extract detects the seam-error union and its REQ citation
8f1fc914 feat(go-ai-native): go-extract emits the seam-error message half and the conformance assertion
c0f99902 chore(packages): vendor the parity engine into the copies and rematerialise
549677c8 fix(rust-ai-native): the fact sort and health census cover the new variants
736cdcf5 feat(core-ai-native): the Rust config gains a floor_disable twin
c5f14183 feat(go-ai-native): the go gate mounts the dedicated seam-error rule
ae927800 feat(core-ai-native): the seam-error and conformance-assertion rules join the engine
8e03348a docs(core-ai-native): the parity principle joins the manifesto
3c5f51e5 docs(design): the seam-error and assertion parity sketch on the E11 census pair
```

## Quick-start

```sh
python campaigns/packages-2026-09/tasks/summary.py
python campaigns/packages-2026-09/tasks/drift-registry.py
bash tools/self-check.sh   # exit — настоящий, хвостом; фоном — bare-форма
cargo xtask sync-engines   # вендор ×6 после движковых правок
cargo run -q -p vibe-cli --bin vibe -- install --assume-yes   # рематериализация vibedeps
```

_WAL — канон живого состояния; при расхождении верить ему, не этому файлу._
