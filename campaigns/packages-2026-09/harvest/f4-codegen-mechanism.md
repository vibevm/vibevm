# F4-CODEGEN — механизм кодогенерации перед расширением его фазой Ф4

**Чем мерил:** только чтение дерева — Read/Grep/Glob плюс читающие shell-команды
(`wc -l`, `grep`, `ls`). Worktree `wt/F4-CODEGEN`, замер от корня рабочего дерева.

**Что НЕ запускалось:** ни одной команды `git` (запрещена пакетом), ни одной
команды `cargo` (запрещена пакетом), бинарь `jtd-codegen` не запускался — его в
worktree физически нет: `tools/jtd-codegen/` содержит один файл `README.md`
(перечень по Glob; двоичный файл не коммитится — `tools/jtd-codegen/README.md:3-5`).

**Дата:** 2026-08-15.

---

## 1. ВЕРДИКТ

**Расширяем существующий механизм — он для этого и построен; второй генератор не нужен.**
Механизм уже дважды пережил расширение без смены архитектуры: вторый дом схем
(specmap в `core-ai-native`, `xtask/src/codegen.rs:49`, `xtask/src/codegen.rs:115`)
и третья цель `format_id` из TOML-реестра (`xtask/src/codegen.rs:214-217`) въехали
в него как новые ветки, не как новый конвейер.

Самое узкое место для Ф4.2 — **не маршрутизация, а три шва**:

1. **Схема не видна как структура.** xtask передаёт бинарю *путь* файла
   (`xtask/src/codegen.rs:191-195`) и ни разу не разбирает JTD сам — а постпроцессор
   Ф4.2 обязан читать `metadata."x-empty"` из схемы
   (`campaigns/packages-2026-09/harvest/f0-gen-poc.md:541-546`). Шов дёшев:
   `serde_json` уже в зависимостях xtask (`xtask/Cargo.toml:17`).
2. **Бюджет длины исчерпан.** `codegen.rs` = 534 строки при бюджете 600
   (`conform.toml:16`; `xtask` в зоне действия бюджета — `conform.toml:21`).
   Запас 66 строк; Ф4.2 оценена в «несколько сотен строк»
   (`campaigns/packages-2026-09/harvest/f0-gen-poc.md:560`). Без раскола не
   переживёт; естественный шов раскола — модуль-каталог по прецеденту
   `xtask/src/batch_review/` и `xtask/src/mirror/`.
3. **Версия генератора не сведена с артефактами.** Пин — 0.4.1
   (`packages/org.vibevm.ai-native/jtd-codegen/v0.1.0/README.md:25`), все 8
   закоммиченных JTD-артефактов печатают `v0.2.1` в первой строке (см. §7.1), и
   ни один гейт версию не проверяет
   (`packages/org.vibevm.ai-native/jtd-codegen/v0.1.0/README.md:28-29`). Постпроцессор
   по плану привязан к форме эмиссии 0.4.1
   (`campaigns/packages-2026-09/harvest/f0-gen-poc.md:553-554`) — сводить надо до
   постпроцессора, не после.

## 2. Подтверждение-или-опровержение записанного (U1–U7)

| # | записанное утверждение | вердикт | цитата |
|---|---|---|---|
| U1 | `xtask/src/codegen.rs` имеет длину около 330 строк | **ОПРОВЕРГНУТО** — 534 строки (`wc -l xtask/src/codegen.rs` → `534 xtask/src/codegen.rs`). «Около 330» занижено на ~200 строк: блок `format_id` (294–534) один занимает ~240 | `wc -l`; границы блока `xtask/src/codegen.rs:294-534` |
| U2 | маршрутизация целей — «одна ветка `match`» в функции `generated_dir_for` | **ОПРОВЕРГНУТО в формулировке**. `match` в `generated_dir_for` существует, но веток в нём **две** — `"specmap"` и `_` (`xtask/src/codegen.rs:64-69`), и это НЕ весь маршрутизатор: цель `format_id` маршрутизируется отдельной веткой **по равенству каталога вывода**, а не по суффиксу (`xtask/src/codegen.rs:214-217`; пояснение, почему она сознательно не в `generated_dir_for`, — `xtask/src/codegen.rs:207-213`) | `xtask/src/codegen.rs:63-70`, `xtask/src/codegen.rs:214-217` |
| U3 | `check-codegen` — это `git diff --exit-code` по generated-каталогам | **ПОДТВЕРЖДЕНО**, с оговоркой: сперва выполняется полная регенерация (`run_codegen()` в первой строке), и только затем diff ровно по двум generated-каталогам | `xtask/src/codegen.rs:259` (реген), `xtask/src/codegen.rs:264-273` (argv: `git diff --exit-code -- <dir1> <dir2>`) |
| U4 | `check-codegen` НЕ видит untracked-файлов | **ПОДТВЕРЖДЕНО** по чтению реализации (разбор в §4) | `xtask/src/codegen.rs:269-273` (весь argv; в нём нет ни `add`, ни `status`, ни `--no-index`) |
| U5 | генератор НЕ трогает `crates/vibe-cli/resources/package-tree.schema.v1.json`, потому что маршрутизация идёт по суффиксу `*.jtd.json` | **ПОДТВЕРЖДЕНО**, причина двойная: (а) фильтр суффикса `.jtd.json` — у файла суффикс `.v1.json`; (б) каталог `crates/vibe-cli/resources/` вообще не сканируется — кодоген обходит только `schemas/` и engine-`schemas/`. Реестр фиксирует это решение дважды | суффикс: `xtask/src/codegen.rs:97-103`; каталоги сканирования: `xtask/src/codegen.rs:115`; решение в реестре: `formats/REGISTRY.toml:25-26`, `formats/REGISTRY.toml:89-90`; сам файл существует (Glob `crates/vibe-cli/resources/*` → `package-tree.schema.v1.json`) |
| U6 | в дереве есть ВТОРОЕ семейство JTD-схем — движка трассировки под `packages/org.vibevm.ai-native/**` — и `codegen.rs` его тоже маршрутизирует | **ПОДТВЕРЖДЕНО**. Второй дом схем — `packages/org.vibevm.ai-native/core-ai-native/v0.8.0/schemas/`, сканируется наравне с хостовым, маршрутизируется веткой `"specmap"` в engine-крейт. Оговорка: «семейство» сегодня — **одна** схема `specmap.jtd.json` (Glob каталога: `specmap.jtd.json`, `specmap.example.json`); копия в `vibedeps/…0.8.0/schemas/` не сканируется (правильно — это регенерируемое зеркало) | слот: `xtask/src/codegen.rs:49`; каталог: `xtask/src/codegen.rs:53-55`; сканирование: `xtask/src/codegen.rs:115`; маршрутизация: `xtask/src/codegen.rs:65-67` |
| U7 | jtd-codegen эмитит поля в camelCase с `#[serde(rename)]` и НЕ эмитит `deny_unknown_fields` | **ПОДТВЕРЖДЕНО**. Rust-поля — camelCase, `#[serde(rename)]` несёт wire-имя: `pub codeItems: …` под `#[serde(rename = "code_items")]`. `deny_unknown_fields` — 0 вхождений в обоих generated-деревьях (grep). Пояснение в исходниках называют это «v0.4.1-only quirk» | camelCase+rename: `packages/…/core-ai-native-specmap/src/generated/specmap/mod.rs:14-15`, `:23-24`, `:37-38`, `:44-45`; свидетельства исходников: `crates/vibe-wire/src/lib.rs:46-52`, `packages/…/core-ai-native-specmap/src/lib.rs:28-30`; отсутствие deny: grep `deny_unknown_fields` по `crates/vibe-wire/src/generated` и engine-`generated` — 0 совпадений |

Ценность для нарезки: опровергнуты U1 (сильно) и U2 (по существу — «одна ветка»
скрывала и вторую ветку match, и внесуффиксную ветку `format_id`); подтверждены
U3–U7, из них U4 и U6 — с важными нюансами (§4, §7).

## 3. Что делает `cargo xtask codegen`

Длина файла: **534 строки** (`wc -l xtask/src/codegen.rs`).
Диспетчеризация: `Cmd::Codegen => run_codegen()` — `xtask/src/main.rs:350`,
вариант описан в `xtask/src/main.rs:64-66`.

### 3.1. Цели генерации — полный перечень (9)

Входы-схемы собираются из **двух домов**: `schemas/` в корне и
`packages/org.vibevm.ai-native/core-ai-native/v0.8.0/schemas/` — цикл по массиву
из двух каталогов, отсутствие любого — ошибка «broken checkout»
(`xtask/src/codegen.rs:114-120`). Отбор — только файлы с суффиксом `*.jtd.json`,
лежащие **непосредственно** в каталоге (`xtask/src/codegen.rs:92-105`).

| # | вход | выход | код маршрутизации |
|---|---|---|---|
| 1 | `schemas/init_report.jtd.json` | `crates/vibe-wire/src/generated/init_report/mod.rs` | ветка `_` → `vibe_wire_generated_dir` (`xtask/src/codegen.rs:68`, `:77-79`); спавн бинаря в цикле `generate_into` (`:185-205`) |
| 2 | `schemas/install_plan.jtd.json` | `…/generated/install_plan/mod.rs` | то же |
| 3 | `schemas/install_report.jtd.json` | `…/generated/install_report/mod.rs` | то же |
| 4 | `schemas/list_report.jtd.json` | `…/generated/list_report/mod.rs` | то же |
| 5 | `schemas/registry_publish_report.jtd.json` | `…/generated/registry_publish_report/mod.rs` | то же |
| 6 | `schemas/registry_sync_report.jtd.json` | `…/generated/registry_sync_report/mod.rs` | то же |
| 7 | `schemas/uninstall_report.jtd.json` | `…/generated/uninstall_report/mod.rs` | то же |
| 8 | `packages/…/core-ai-native/v0.8.0/schemas/specmap.jtd.json` | `packages/…/core-ai-native-specmap/src/generated/specmap/mod.rs` | ветка `"specmap"` (`xtask/src/codegen.rs:65-67`) |
| 9 | `formats/REGISTRY.toml` | `crates/vibe-wire/src/generated/format_id/mod.rs` | ветка равенства каталога в `generate_into` → `emit_format_id` (`xtask/src/codegen.rs:214-217`, `:421-534`) |

Цели 1–7 подтверждаются фактическим деревом: `crates/vibe-wire/src/generated/mod.rs:7-14`
перечисляет ровно `format_id` + 7 стемов. Кроме целей, каждая порция вывода
завершается синтезом верхнего `mod.rs` для каталога (`xtask/src/codegen.rs:235-247`).

### 3.2. Маршрутизация

Функция — `generated_dir_for(stem, root)`, `xtask/src/codegen.rs:63-70`. Ветки:

1. `"specmap"` → `packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-specmap/src/generated`
   (`xtask/src/codegen.rs:65-67`; слот — константа `SPECMAP_ENGINE_SLOT`, `:49`);
2. `_` (всё прочее) → `crates/vibe-wire/src/generated`
   (`xtask/src/codegen.rs:68`, реализация `:77-79`).

**Ветки не по суффиксу `*.jtd.json` есть** — ровно одна: цель `format_id`. Она
маршрутизируется не в `generated_dir_for`, а условием
`if out_dir == vibe_wire_generated_dir(root)` внутри `generate_into`
(`xtask/src/codegen.rs:214-217`), потому что у TOML-реестра нет стема схемы —
это записано в комментарии `xtask/src/codegen.rs:207-213`.

### 3.3. Откуда берётся jtd-codegen

`find_jtd_codegen(root)`, `xtask/src/codegen.rs:19-44`:

1. проект-локальная копия `tools/jtd-codegen/jtd-codegen(.exe)` — предпочтительна
   (`xtask/src/codegen.rs:25-28`);
2. фолбэк — поиск на PATH с пробой `--version` (`xtask/src/codegen.rs:30-32`);
3. нет нигде — `bail!` с рецептом установки и ссылкой на PROP-000 §16
   (`xtask/src/codegen.rs:33-43`).

В этом worktree бинаря нет — в `tools/jtd-codegen/` только `README.md`
(перечень по Glob); сам README подтверждает: бинарь «never committed»
(`tools/jtd-codegen/README.md:3-5`).

**Ожидаемая версия — 0.4.1**, записана в четырёх местах: пин-дом
`packages/org.vibevm.ai-native/jtd-codegen/v0.1.0/README.md:25` («single home»,
`:26-28`), комментарий о поведении `jtd-codegen 0.4.1` (`xtask/src/codegen.rs:180`),
комментарий в `crates/vibe-wire/src/lib.rs:46-47`, указатель панели
`tools/self-check.sh:28-29`.

**Сверка с артефактом — РАСХОЖДЕНИЕ.** Все 8 закоммиченных JTD-артефактов печатают
в первой строке `// Code generated by jtd-codegen for Rust v0.2.1`:
`crates/vibe-wire/src/generated/{init_report,install_plan,install_report,list_report,registry_publish_report,registry_sync_report,uninstall_report}/mod.rs:1`
(7 файлов, grep) и
`packages/…/core-ai-native-specmap/src/generated/specmap/mod.rs:1`. Заявлено 0.4.1 —
в артефактах v0.2.1. При этом `check-codegen` байтовый: на машине с пин-бинарём 0.4.1,
если его эмиссия отличается (уже первой строкой), гейт должен уйти в красный до
перегенерации; и никакой гейт версию генератора не принуждает — прямо записано
«CI … does not enforce a particular generator build»
(`packages/org.vibevm.ai-native/jtd-codegen/v0.1.0/README.md:28-29`). Статически
установить, чем порождена разница (артефакты старше пина / другая семантика
шапки у версий), нельзя — бинарь не запускался (§ шапка); зафиксировано как
дыра №1 в §7.

### 3.4. Пост-обработка сегодня

xtask **не редактирует содержимое** выходов jtd-codegen — ни переименований, ни
`allow`-атрибутов, ни сортировки внутри файла. В цикле по схемам файл
передаётся бинарю (`xtask/src/codegen.rs:191-196`), проверяется статус
(`:197-203`), и `sub_out` больше не читается и не пишется. Что xtask делает
вокруг — оркестрация, не трансформация:

- **стирание каталога вывода перед регенерацией** (кроме `.gitkeep`), чтобы
  `check-codegen` был точным и не оставлял стейл-модулей —
  `xtask/src/codegen.rs:157-178` (пояснение — `:151-156`);
- **по-схемные подкаталоги**: jtd-codegen пишет один `mod.rs` на `--rust-out` и
  затирает прежний, поэтому каждая схема получает свой `<stem>/`, а верхний
  `mod.rs` синтезируется — `xtask/src/codegen.rs:180-189`;
- **синтез верхнего `mod.rs`** с шапкой «DO NOT EDIT» и списком подмодулей —
  `xtask/src/codegen.rs:235-247`;
- **эmission-ветка `format_id`** (TOML → Rust строковым билдингом) —
  `xtask/src/codegen.rs:214-217`, `:421-534`.

Итог для Ф4: **контентного постпроцессора сегодня нет** — Ф4.2 станет первым
преобразованием содержимого выходов jtd-codegen в этом механизме.

### 3.5. Байт-стабильность между платформами

Механизмы (все — в xtask или в git-конфиге дерева):

1. группировка каталогов в `BTreeMap` — детерминированный порядок обработки
   каталогов (`xtask/src/codegen.rs:127-130`, комментарий `:127-129`);
2. сортировка имён модулей перед синтезом `mod.rs` — «filesystem read order is
   not guaranteed» (`xtask/src/codegen.rs:219-221`);
3. сортировка + дедуп списка источников для шапки (`xtask/src/codegen.rs:232-233`);
4. нормализация разделителей путей в шапке: `replace('\\', "/")` — в комментарии
   всегда `/` (`xtask/src/codegen.rs:229`);
5. детерминизм реестра: таблица `toml::Value` — `BTreeMap`, итерация
   платформонезависима (`xtask/src/codegen.rs:317-319`);
6. `pascal_case` использует `to_ascii_uppercase` — вне локали
   (`xtask/src/codegen.rs:411`);
7. стирание-и-регенерация — на диске остаётся ровно то, что генерируется
   (`xtask/src/codegen.rs:151-156`);
8. переводы строк: канону LF держит `.gitattributes:7` (`* text=auto eol=lf`,
   назначение — стабильность content_hash, `.gitattributes:1-6`); фактические
   артефакты LF — `grep -c $'\r'` по четырём сгенерированным файлам дал 0
   в каждом (вывод в §8).

**Риски/пределы:** (а) `schemas_under` не сортирует выдачу `read_dir`
(`xtask/src/codegen.rs:92-105`) — сегодня безвредно (выходы схем дизъюнктны,
списки сортируются позже), но любой будущий шаг, эмитящий позиционный вывод по
порядку обхода, унаследует недетерминизм — шов Ф4.2 должен ключиться по стему;
(б) EOL и форму эмиссии самого бинаря xtask не контролирует — опора только на
пин версии (который не проверяется, §3.3) и на git-атрибуты.

## 4. Что покрывает `check-codegen`

Реализация — `run_check_codegen()`, `xtask/src/codegen.rs:258-292`; диспетчер
`xtask/src/main.rs:351`; документация `xtask/src/main.rs:68-71`.

1. Полная регенерация: `run_codegen()?` — `xtask/src/codegen.rs:259`.
2. Сравнение: `git diff --exit-code -- <vibe-wire generated> <engine generated>`
   — argv собирается в `xtask/src/codegen.rs:269-273` из массива `out_dirs`
   (`:264-268`), статус — `:274-277`, при отличии — `bail!` с рецептом
   «Run `cargo xtask codegen` and commit» (`:278-289`), иначе «clean» (`:290`).

### 4.1. Что он НЕ видит: untracked-файлы — НЕ видит

Ответ на вопрос пакета: **новый файл, созданный генератором и не добавленный в
git-индекс, `check-codegen` НЕ увидит — гейт молча пройдёт зелёным.**
Обоснование по реализации и по документации вызываемой команды:

- Код строит argv ровно `["diff", "--exit-code", "--", dir1, dir2]` и больше
  ничего с индексом не делает — ни `git add`, ни `git status`, ни `--no-index`
  (`xtask/src/codegen.rs:269-273`; весь охват функции `:258-292`).
- Форма `git diff [--] [<path>…]` по документации git сравнивает **рабочее
  дерево с индексом** по путям; untracked-файл не имеет записи в индексе, поэтому
  не даёт ни строки вывода, ни вклада в `--exit-code` — флаг лишь превращает
  «есть diff» в ненулевой код выхода.
- Дополнительная маскировка от регенерации: шаг 1 стирает каталог вывода
  (`xtask/src/codegen.rs:157-178`), поэтому untracked-файл, который генератор НЕ
  производит, удаляется ещё ДО diff; а файл, который генератор ПРОИЗВОДИТ (новая
  схема), остаётся untracked — и diff по-прежнему чист. Оба направления
  невидимы: и «забыли закоммитить новый выход», и «мусор в каталоге вывода».

Предел слепоты — только untracked: изменение/удаление **отслеживаемого** файла
каталога ловится, ведь после регенерации рабочая копия отличается от индекса.

### 4.2. Место в панели `tools/self-check.sh`

Шаг **6b**, строка `tools/self-check.sh:342`
(`run_step "cargo xtask check-codegen" cargo xtask check-codegen`); его
документация — `tools/self-check.sh:24-29`.

- **До него:** 0b (знаменатель живых пакетов, `:196`), 0c (триплет
  CLAUDE/AGENTS/GEMINI, `:219`), пробы gopls/node_modules (`:229-256`), шаг 0 —
  снапшот user-home (`:279-286`), 1 `cargo fmt --all --check` (`:295`), 2
  `cargo test --workspace` (`:298`), 2b tripwire (`:303`), 3 clippy `-D
  warnings` (`:306`), 4 `vibe check` (`:316`), 5 `xtask conform check` (`:325`),
  6 `xtask sync-engines --check` (`:332`).
- **После него:** 6c `xtask specmap --check` (`:352`), 7 — гейты core-ai-native
  fmt/test/clippy (`:378-385`), 8 — гейты трёх стеков (`:393-410`), 9 —
  self-trace гейты (`:425-430`), 10 — mcp-пакеты (`:437-458`), 10b — conform по
  каждому слоту (`:481-486`), 10c — знаменатель mcp (`:531`), 10d — clock-gate
  (`:571`), 11b lane-citation lint (`:603`), 11c лицензии (`:640`), 11b markup
  `--exhaustive` (`:663`), 12 — финальный tripwire (`:670`).

## 5. Цель `format_id` как образец расширения

- **Устройство.** Вход — `formats/REGISTRY.toml` (20 записей `[format.*]` —
  число исправлено боссом при перемере: `grep -c '^\[format\.'` даёт 20, и
  ровно 20 вариантов несёт сгенерированный `FormatId`, что и сторожит тест
  полноты ниже,
  `formats/REGISTRY.toml:33-199`). Выход —
  `crates/vibe-wire/src/generated/format_id/mod.rs`: еnum `FormatId` +
  `ForeignParsers` + impl (`ALL`, `id`, `epoch`, `recoverable`,
  `foreign_parsers`). Код: чтение и валидация — `load_format_registry`
  (`xtask/src/codegen.rs:320-357`) с редуцированной записью `FormatEntry`
  (`:306-315`) и валидаторами `require_*` (`:359-397`); именование —
  `pascal_case` (`:401-415`); эмиcсия — `emit_format_id` (`:421-534`),
  строковым билдингом.
- **Отличие от JTD-целей:** (1) вход разбирает сам xtask (`toml::Value`,
  `xtask/src/codegen.rs:324`), а не внешний бинарь; (2) маршрутизация — не по
  стему `.jtd.json`, а по равенству каталога вывода (`:214-217`) с явным
  обоснованием (`:207-213`); (3) выход обязан быть `cargo fmt`-чистым — панель
  гоняет fmt по нему (примечание `:417-420`; fmt — первый шаг панели,
  `tools/self-check.sh:295`); (4) это внутренний идентификатор без
  Serialize/Deserialize — сознательно вне запрета hand-written-wire
  (`:433-437`).
- **Тест полноты:** `format_id_completeness` —
  `crates/vibe-wire/tests/format_registry_completeness.rs:28-49`: двустороннее
  сравнение множеств (BTreeSet) id из TOML против `FormatId::ALL … .id()`
  (`:37-41`), падает в обе стороны — «добавили формат, забыли codegen» и
  «enum убежал от реестра» (`:6-9`); dev-зависимость toml у vibe-wire —
  `crates/vibe-wire/Cargo.toml:18`.
- **Что пришлось изменить в общей части:** факторизация
  `vibe_wire_generated_dir` из `generated_dir_for` — чтобы emission-ветка
  сравнивала с тем же путём, который даёт маршрутизация, а не с расходящимся
  литералом (`xtask/src/codegen.rs:72-79`); сама ветка и `module_names.push`
  до сортировки в `generate_into` (`:214-217`, сортировка `:221`).

Это и есть шаблон Ф4.2: новая цель = новая emission-ветка в `generate_into`
plus (при новом виде входа) собственный загрузчик входа в том же файле.

## 6. Что потребуется Ф4.2 — сшивка трёх входов

По §7 находки Ф0, конвейер — `(схема + реестр + сгенерированный Rust) → Rust`
(`campaigns/packages-2026-09/harvest/f0-gen-poc.md:541-546`, `:552-558`).

- **Схема как структура — НЕТ.** Сегодня xtask передаёт бинарю путь файла:
  `Command::new(binary).arg("--rust-out").arg(&sub_out).arg(schema)`
  (`xtask/src/codegen.rs:191-195`); JTD в коде не разбирается ни разу — в файле
  нет ни одного чтения схем кроме передачи путей (весь `run_codegen`/`
  generate_into`: `:107-256`).
- **Реестр как структура — ДА.** `load_format_registry` разбирает TOML в
  `toml::Value` и редуцирует в `Vec<FormatEntry>` — тип `FormatEntry { id,
  variant, epoch, recoverable, foreign_parsers }`
  (`xtask/src/codegen.rs:306-315`, парсинг `:320-357`).
- **Минимальный шов.** Место — цикл по схемам в `generate_into`
  (`xtask/src/codegen.rs:185-205`): сразу после спавна и проверки статуса
  (`:191-203`) и до `module_names.push(stem)` (`:204`) — вставить вызов вида
  `postprocess_schema(&sub_out, schema, &entries)?`, где `postprocess_schema`
  (а) читает `sub_out/mod.rs` (сгенерированный Rust уже на диске), (б) разбирает
  JTD-схему в `serde_json::Value` (зависимость уже есть —
  `xtask/Cargo.toml:17`), (в) получает записи реестра через существующий
  `load_format_registry(root)` (`xtask/src/codegen.rs:320`), загруженный один
  раз в `run_codegen` (`:107-149`) и продетый в сигнатуру `generate_into`
  (`:157`). Сшивка по путям — стем схемы ↔ подмодуль (`schema_stem`,
  `:82-89`) ↔ `schema`-поле записи реестра (`formats/REGISTRY.toml:35,43,…`).
  Для цели №3 (`x-empty`) этого достаточно; №4 (`deny_unknown_fields` по
  `foreign_parsers = "none"`) на тех же входах.
- **Бюджет длины.** 534 из 600 (`wc -l`; бюджет `conform.toml:16`, `xtask` в
  корнях сканирования `conform.toml:21`; `/generated/` исключён из conform —
  `conform.toml:22`, т.е. бюджет давит на сам `codegen.rs`, а не на его вывод).
  Остаток — **66 строк**; постпроцессор оценён планом в «несколько сотен строк»
  (`campaigns/packages-2026-09/harvest/f0-gen-poc.md:560`) — **не влезает,
  раскол обязателен**. Естественный шов раскола — по прецеденту соседей: в xtask
  подкоманды, переросшие ~500 строк, стали модулями-каталогами —
  `xtask/src/batch_review/` (8 файлов, `mod.rs` 244 строки) и `xtask/src/mirror/`
  (3 файла, `mod.rs` 475 строк); плоские файлы сегодня: `codegen.rs` 534,
  `sync_engines.rs` 517, `rebuild.rs` 416, `main.rs` 479 (все — `wc -l`, §8).
  Конкретно: `xtask/src/codegen/` с `mod.rs` (маршрутизация + `generate_into`,
  текущие `:1-256`), `format_id.rs` (`:294-534`, ~240 строк — уже готовая
  граница), `postproc.rs` (новое). Сам план Ф0 предлагает рождение в
  `xtask/src/wiregen/` (`campaigns/packages-2026-09/harvest/f0-gen-poc.md:560`)
  — совместимо: каталог либо так, либо внутри `codegen/`.

## 7. Дыры и неожиданности

1. **Версия генератора не сведена с артефактами.** Пин 0.4.1
   (`packages/org.vibevm.ai-native/jtd-codegen/v0.1.0/README.md:25`); все 8
   артефактов печатают v0.2.1 (`crates/vibe-wire/src/generated/*/mod.rs:1` — 7
   файлов; `packages/…/core-ai-native-specmap/src/generated/specmap/mod.rs:1`).
   Гейта на версию нет (`packages/org.vibevm.ai-native/jtd-codegen/v0.1.0/README.md:28-29`),
   `find_jtd_codegen` проверяет лишь успех `--version`, не строку версии
   (`xtask/src/codegen.rs:30-32`). Для Ф4.2, чей постпроцессор привязан к форме
   эмиссии, это первое, что надо свести.
2. **`schemas_under` нерекурсивен**, а реестр уже ссылается на схемы в
   подкаталогах: `schemas/index/e1/*.jtd.json`, `schemas/journal/e1/…`,
   `schemas/hello/e1/…` (`formats/REGISTRY.toml:104,112,120,128,136,144,187,195`);
   этих файлов ещё нет (Glob `schemas/**` — только 7 верхнеуровневых), но когда
   появятся — кодоген их **не увидит**: `read_dir` + `p.is_file()` без обхода в
   глубину (`xtask/src/codegen.rs:92-105`). Прямое отверстие для Ф4.
3. **Untracked-слепота `check-codegen` вдвойне**: diff не видит untracked
   (§4.1), а предшествующая регенерация стирает untracked-мусор до diff
   (`xtask/src/codegen.rs:157-178`) — «новый выход не закоммичен» и «мусор в
   каталоге вывода» невидимы обоими направлениями.
4. **`rustOptions.package` в схемах — мёртвые метаданные**: в
   `schemas/init_report.jtd.json:8` задан `"package": "vibe_wire::init_report"`,
   но вывод — голый `mod.rs` в подкаталоге без пути (факт:
   `crates/vibe-wire/src/generated/init_report/mod.rs`), и никто его не читает.
5. **Протухший путь в описании схемы**: `schemas/init_report.jtd.json:3`
   называет источником `…/generated/init_report.rs`, а фактический выход —
   `init_report/mod.rs` (подтверждается деревом); правка требует регенерации.
6. **Мёртвая запись в `.gitattributes`**: `crates/specmap-core/src/generated/**`
   (`:47`) — каталога `crates/specmap-core/` в дереве нет (Glob — 0 файлов);
   след релокации specmap в packages; для Ф4 это же место надо обновить под
   новые выходы, иначе linguist-разметка снова разойдётся с деревом.
7. **Дублирование литерала engine-пути**: `run_check_codegen` повторяет
   `root.join(SPECMAP_ENGINE_SLOT).join("crates/core-ai-native-specmap/src/generated")`
   (`xtask/src/codegen.rs:266-267`) вместо вызова
   `generated_dir_for("specmap", …)` (`:63-70`) — при смене слота рассинхрон
   возможен ровно в гейте.
8. **camelCase-поля как постоянный налог**: из-за эмиссии jtd-codegen оба
   потребителя несут `allow(non_snake_case)` — `crates/vibe-wire/src/lib.rs:52`
   и `packages/…/core-ai-native-specmap/src/lib.rs:30`; преобразование №2 Ф4.2
   (`campaigns/packages-2026-09/harvest/f0-gen-poc.md:537-540`) снимет их.
9. **Локального бинаря в свежем worktree нет** — `tools/jtd-codegen/` держит
   только README (`tools/jtd-codegen/README.md:3-5`): любой новый worktree/CI
   обязан ставить бинарь по рецепту пакета до первого `codegen`.

## 8. Как воспроизвести этот замер

Из корня рабочего дерева, только читающие команды:

```bash
wc -l xtask/src/codegen.rs
# 534 xtask/src/codegen.rs

wc -l xtask/src/*.rs xtask/src/batch_review/*.rs xtask/src/mirror/*.rs

grep -nE 'fn (generated_dir_for|vibe_wire_generated_dir|schemas_under|generate_into|run_codegen|run_check_codegen|find_jtd_codegen|emit_format_id|load_format_registry)' xtask/src/codegen.rs
grep -n 'SPECMAP_ENGINE_SLOT' xtask/src/codegen.rs
grep -n 'run_step' tools/self-check.sh            # позиция 6b: строка 342
grep -n 'jtd-codegen' xtask/src/codegen.rs        # комментарий о 0.4.1: строка 180
grep -n 'max_file_lines' conform.toml             # 600: строка 16
grep -n 'roots = ' conform.toml                   # xtask в корнях: строка 21

# цели: инвентарь схем и выходов
ls schemas
ls packages/org.vibevm.ai-native/core-ai-native/v0.8.0/schemas
ls crates/vibe-wire/src/generated
grep -rn --include='*.jtd.json' -l . schemas \
  packages/org.vibevm.ai-native/core-ai-native/v0.8.0/schemas

# версия генератора: пин против артефактов
grep -n '0.4.1' packages/org.vibevm.ai-native/jtd-codegen/v0.1.0/README.md
grep -rn '^// Code generated' crates/vibe-wire/src/generated \
  packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-specmap/src/generated

# U7: camelCase + rename, отсутствие deny_unknown_fields
grep -n 'serde(rename' packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-specmap/src/generated/specmap/mod.rs
grep -rn 'deny_unknown_fields' crates/vibe-wire/src/generated \
  packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-specmap/src/generated

# байт-стабильность: LF в артефактах (0 = чисто)
grep -c $'\r' crates/vibe-wire/src/generated/init_report/mod.rs \
  crates/vibe-wire/src/generated/mod.rs \
  crates/vibe-wire/src/generated/format_id/mod.rs \
  packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-specmap/src/generated/specmap/mod.rs

# тест полноты format_id
grep -rn 'format_id_completeness' crates/vibe-wire

# мёртвые ссылки
ls crates/specmap-core 2>&1          # каталога нет
ls tools/jtd-codegen                  # только README.md
```

Ни `git`, ни `cargo` в замере не участвовали (запрещены пакетом).
