# F32-SURFACE — поверхности, которых коснётся проектор (замер под Ф3.2)

Чем мерил: только чтение — Read/Grep/Glob по `crates/**`, `xtask/**`, `tools/**`
и корневому `Cargo.toml` (vibe-index разобран до строки). Что НЕ запускалось:
ни одной команды git, ни одной команды cargo, никакой сборки; `vibedeps/**` и
`packages/**` не открывались (периметр пакета), из-за чего реализация
`specmap --check` внутри пакетного крейта осталась за кадром — см. §8. Дата:
2026-08-14. Рабочее дерево: `.wt/F32-SURF2`, ветка `wt/F32-SURF2`.

## 1. ВЕРДИКТ

**ДА С ОГОВОРКАМИ.** По ФОРМЕ сегодняшней оснастки хватает: проверочный глагол
`--check` — отработанная идиома xtask (пять штук, §3), побайтовый компаратор
деревьев существует в продакшн-качестве прямо внутри xtask
(`xtask/src/sync_engines.rs:159-217`), форма сообщений «правило-причина-починка»
канонизирована и в xtask, и в панели (`tools/self-check.sh:550-570`), шаг панели
— копия строки `run_step "cargo xtask … --check" …` (`tools/self-check.sh:332`).
По ЗВЕНУ — не хватает одного соединения: xtask сегодня не зависит ни от одного
продуктового крейта, поэтому `rebuild --check` тянет за собой не «новый слой»,
а новую ЗАВИСИМОСТЬ плюс два-три решения, которых ни один существующий
проверочный глагол не требовал. Что именно мешает написать его «ещё одним
проверочным глаголом» без новых слоёв:

1. В `xtask/Cargo.toml:13-25` нет ни `vibe-index`, ни какого-либо продуктового
   крейта вообще; при этом `vibe-index` отсутствует и в
   `[workspace.dependencies]` корня (`Cargo.toml:66-88` — есть `vibe-registry`
   на строке 73, index нет). Подключение = либо прямой
   `vibe-index = { path = "../crates/vibe-index" }` в xtask, либо новая строка в
   workspace-таблице: правка двух манифестов там, где все прочие глаголы
   обходятся нулём.
2. `tempfile` в xtask — только dev-зависимость (`xtask/Cargo.toml:27-28`), весь
   tempdir-опыт живёт в `#[cfg(test)]`-модулях (`xtask/src/sync_engines.rs:371`,
   `xtask/src/batch_review/checks.rs:374`). `rebuild --check` нужен
   scratch-каталог под проекцию → либо поднять tempfile в `[dependencies]`,
   либо проектить в сиблинг-каталог руками.
3. Компараторов два, и оба не-в-форме как есть: `assert_trees_byte_identical`
   — `#[cfg(test)]`-приват в vibe-index (`crates/vibe-index/src/index/memory/tests.rs:360`);
   `file_set`/`diff_crate` — приватные fn в xtask (`xtask/src/sync_engines.rs:159`,
   `:195`) с deny-листом под vendor-мирроры. Дешевле всего поднять
   sync-engines-механику в общий модуль xtask — оба потребителя в одном крейте.
4. Прецедент «как сравнивать» разошёлся: `check-codegen` делегирует сравнение
   `git diff --exit-code` (`xtask/src/codegen.rs:269-277`), `sync-engines
   --check` сравнивает сам в процессе (`xtask/src/sync_engines.rs:258-273`). Для
   rebuild git-diff не годится — каталог данных не есть git-трекаемое дерево
   (журнал под gitignored `state/`, `crates/vibe-index/src/journal/store.rs:29-31`)
   — значит форма за sync-engines, но это надо зафиксировать решением.
5. Журнал (Ф3.1) уже приземлён, но `init` его ещё не пишет: `Event::Initialised`
   документирован как «written by `init` as the journal's first record»
   (`crates/vibe-index/src/journal/record.rs:36-41`), а вызова `journal::append`
   в `crates/vibe-index/src/cli/init.rs:42-65` нет. Каталог, рождённый сегодняшним
   `init`, не несёт правды → `rebuild --check` на нём не пройдёт. Проводка
   init→journal — предусловие приёмки Ф3.2 (разобрано в §4).

## 2. Сверка опорных координат (B1..B5)

- **B1 — ДА.** xtask — отдельный крейт, член workspace (`Cargo.toml:28`;
  `xtask/Cargo.toml:1-11`), с подкомандами через clap (`xtask/src/main.rs:51-229`).
  Проверочных глаголов БОЛЬШЕ заявленных двух: `check-codegen`
  (`xtask/src/main.rs:65`, реализация `xtask/src/codegen.rs:258`), `specmap
  --check` (`xtask/src/main.rs:69-74`, `xtask/src/specmap.rs:11-13`), а также
  `sync-engines --check` (`xtask/src/main.rs:207-211`,
  `xtask/src/sync_engines.rs:293`), `mirror --check` (`xtask/src/main.rs:219-228`)
  и ratchet-режим `conform check` (`xtask/src/main.rs:270-282`).
- **B2 — ДА.** `crates/vibe-index/src/cli/init.rs:42-65`: флаги `--registry`
  (:22-24), `--registry-url` (:26-28), `--naming` (:30-34) → единственный замер
  часов на краю `at = Utc::now()` (:46) → `Index::new` (:53) → первый
  `write_to` (:54) → `.gitignore` только если файла нет (:55, :67-82: check
  exists на :69-71) → `README.md` только если файла нет (:56, :84-126: check
  exists на :86-88). Уточнение к слову «единственный»: это единственное
  рождение идентичности ИЗ ВХОДОВ (флагов). `reindex` тоже зовёт `Index::new`
  в продакшне (`crates/vibe-index/src/cli/reindex.rs:252`), но пересоздаёт
  идентичность из ПРОЧИТАННОГО каталога (`existing.registry`), а `add.rs:168` и
  `server/metrics.rs:96` — вызовы внутри `#[cfg(test)]`-модулей
  (`crates/vibe-index/src/cli/add.rs:156`, `crates/vibe-index/src/server/metrics.rs:85`).
- **B3 — ДА.** `tools/self-check.sh` несёт гейты как shell-функции с говорящими
  именами; гейт часов `check_index_clock_gate` (`tools/self-check.sh:550-570`)
  — образец: многострочное сообщение называет правило, причину (PROP-044 §4.3,
  F2-1) и команду починки. Полный перечень — §5.
- **B4 — ДА.** `crates/vibe-index/src/index/memory/tests.rs:360-377` —
  `assert_trees_byte_identical` (+ рекурсивный `walk` :379-394): полный обход
  двух каталогов, сравнение множества путей (:362-366) и байтов каждого файла
  (:367-376). НО: `#[cfg(test)]`-приват, не pub, из xtask недоступен (§6).
- **B5 — НЕТ.** `xtask/Cargo.toml:13-25` не содержит ни одного продуктового
  крейта (только anyhow/clap/serde/serde_json/toml/walkdir + пакетные
  движки specmap/conform/cli). Сегодня xtask не может построить `Index`,
  вызвать `write_to` или прочитать `repomd.json` типами крейта. Это главный
  сюрприз сверки и пункт 1 вердикта.

## 3. Подкоманды `xtask` и форма проверочного глагола

Полный перечень (объявление = вариант enum `Cmd` в `xtask/src/main.rs`;
диспетчер = `main()` там же; реализация — файл:строка входа):

| Подкоманда | Что делает | Объявлена | Реализация |
|---|---|---|---|
| `codegen` | регенерация JTD-типов под `src/generated/` | main.rs:60 | codegen.rs:107 `run_codegen` |
| `check-codegen` | codegen + `git diff --exit-code` по сгенерённым деревьям | main.rs:65 | codegen.rs:258 `run_check_codegen` |
| `specmap [--check]` | регенерация/байт-diff `specmap.json` | main.rs:69-74 | specmap.rs:11 `run_specmap` ( shim → пакетный крейт) |
| `test-gate` | workspace-тесты vs xfail-база | main.rs:79-83 | main.rs:327 → `rust_ai_native_cli::run_test_gate` |
| `tripwire` | debt-записи с сработавшими touch; warn-only, exit 0 | main.rs:88-97 | main.rs:328-330 |
| `trace explain` | подграф трассируемости вокруг цели | main.rs:101-104, :296-318 | main.rs:337-345 |
| `conform check/freeze` | гейт дисциплины vs ratchet-база / перезапись базы | main.rs:108-111, :270-293 | main.rs:331-336 → conform.rs |
| `codemod add-cell` | шаблонная правка «новая ячейка», all-or-nothing | main.rs:119-122, :232-267 | main.rs:356-372 |
| `fast-loop` | каждая ячейка собирается и тестируется в бюджете | main.rs:129-142 | main.rs:346-355 |
| `health` | advisory-сборщик фактов (+ `--mirrors` проба) | main.rs:151-161 | main.rs:373-383 + :418-438 |
| `batch-review` | механическая половина ревью markup-батча | main.rs:169-199 | main.rs:384-410 → batch_review/mod.rs |
| `sync-engines [--check]` | миррор/байт-diff vendored-движков | main.rs:207-211 | sync_engines.rs:293 `run_sync_engines` |
| `mirror [--check] [--from]` | фан-аут main+tags по зеркалам, ff-only | main.rs:219-228 | mirror/mod.rs `run_mirror` (main.rs:411) |

Дословный разбор проверочных глаголов (два обязательных + один по замене):

**(а) `check-codegen`** — `xtask/src/codegen.rs:258-292`. Успех/провал: сначала
НАСТОЯЩЕЙ регенерацией на месте (`run_codegen()` :259 — `generate_into` сносит
каталог, сохранив `.gitignore`-подобный `.gitkeep`, codegen.rs:157-178), затем
сравнение ДЕЛЕГИРОВАНО git: `Command::new("git").arg("diff").arg("--exit-code")`
по обоим сгенерённым деревьям (:269-277). Провал — `bail!` с рецептом:
«generated code under … is out of date … Run `cargo xtask codegen` and commit
the result» (:279-288); через `anyhow` из `main() -> Result<()>` это выход 1.
Успех — `eprintln!("xtask check-codegen: clean.")` (:290): слово «clean» —
устойчивая формула. Корень репозитория — `repo_root()` (main.rs:440-451):
родитель `CARGO_MANIFEST_DIR` ( defensive-подъём, работает и не из-под cargo).

**(б) `specmap --check`** — xtask-сторона это ШИМ в одну строку:
`rust_ai_native_specmap::run_specmap(&repo_root()?, check)`
(xtask/src/specmap.rs:11-13); контракт флага — «Regenerate and byte-diff against
the committed index instead of writing; non-zero exit on drift»
(main.rs:70-72). Сам байт-diff живёт в пакетном крейте
`packages/…/rust-ai-native-specmap` — ВНЕ периметра чтения этого пакета,
дословно разобрать нельзя (дыра №2 в §8). Поэтому дословно вторым взят:

**(в) `sync-engines --check`** (ближайший к `rebuild --check` по задаче —
сравнение КАТАЛОГА байтами, без git) — `xtask/src/sync_engines.rs:293-345`.
Механика: `file_set(dir)` (:159-179) — `BTreeSet` относительных путей через
`walkdir` с deny-листом `target/.git/node_modules/.vibe`; `diff_crate(src, dst)`
(:195-217) — union множеств, классификация `missing/extra/changed`, байты —
`fs::read` + `!=` (:204-211); check-режим `sync_all` (:258-273) собирает
drift-строки вида «{target}/{crate}: missing|extra|differs `{rel}`». Выход:
успех — `println!("sync-engines --check: every vendored crate matches its
authored source ({pairs} pair(s) across {sets} sync set(s))…")` (:320-325);
провал — все drift-строки в stderr (:327-329) + `bail!` с числом и рецептом
«Edit the AUTHORED copy … then run `cargo xtask sync-engines`» (:330-336).
Существенно: сравнение В ПРОЦЕССЕ, без порчи рабочих деревьев — та же форма,
что нужна rebuild.

Переиспользуемо для `rebuild --check` (конкретно, с цитатами):

- `repo_root()` — main.rs:440-451, как есть;
- идиома флага — `#[arg(long)] check: bool` + доккомент контракта
  (main.rs:69-74; то же для sync-engines main.rs:207-211);
- `file_set` + `diff_crate` + классификация drift + «список, а не первый diff»
  — sync_engines.rs:159-217, :258-273 (приватные; поднять в общий модуль xtask);
- прецедент «снести и перегенерить на месте» — `generate_into`
  (codegen.rs:157-178);
- форма выхода — `bail!` с рецептом + «clean»-println (codegen.rs:279-290,
  sync_engines.rs:306-336).

Временные каталоги и фикстуры в xtask: в продакшн-пути — НЕТ ни одного
(синк-движки пишут на место, codegen на место); tempdir — только в тестах:
`#[cfg(test)]` у sync_engines.rs:347 (tempdir на :371, :408, :442, :489, :507) и
в batch_review/checks.rs:374+ ; tempfile при этом лишь dev-dep
(xtask/Cargo.toml:27-28).

Зависимости `xtask` — дословно (`xtask/Cargo.toml:13-29`):

```toml
[dependencies]
anyhow.workspace = true
clap.workspace = true
serde.workspace = true
serde_json.workspace = true
toml.workspace = true
specmap-core.workspace = true
rust-ai-native-specmap.workspace = true
walkdir.workspace = true
rust-ai-native-conform.workspace = true
conform-core.workspace = true
rust-ai-native-conform-frontend.workspace = true
rust-ai-native-cli.workspace = true

[dev-dependencies]
tempfile.workspace = true
```

Ответ на вопрос §3.1: сегодня xtask НЕ может построить `Index`, НЕ может
вызвать `write_to`, НЕ может прочитать `repomd.json` типами крейта (B5=НЕТ).
После добавления path-dep на vibe-index — может всё: `lib.rs` открывает
`pub mod index`, `pub mod journal`, `pub mod types`
(crates/vibe-index/src/lib.rs), `index/mod.rs` ре-экспортирует
`Index` и `atomic_write` (:16-19), `journal` — `append/default_dir/replay`
(journal/mod.rs:17-18). Прямой dep на chrono не нужен: у rebuild нет своего
часа — `at` приезжает из записей журнала.

## 4. Путь рождения каталога и цена второй записи

Пошагово `init::run` (`crates/vibe-index/src/cli/init.rs:42-65`):

1. `at = Utc::now()` (:46) — единственный замер часов на команду; коммент
   :43-45 фиксирует закон F2-1 («the clock enters here, once per command, at
   the edge»).
2. Guard (:47-52): `repomd::exists(data_dir) && !force` → `InvalidInput`
   «data directory … already carries an index (use --force to overwrite)».
3. `Index::new(&registry, &registry_url, naming, at)` (:53).
4. `index.write_to(&data_dir, &WriteCtx { at })` (:54) — оркестрация в
   `crates/vibe-index/src/index/memory.rs:197-303`: `create_dir_all` (:198),
   выжженная земля by-name/by-cap/by-purl (:210-212), `primary.jsonl` + `.gz`
   (:216), by-name-файлы (:250), инвертированные виды (:265-277), `repomd.json`
   ПОСЛЕДНИМ (:302) — доккомент :190-196: «partial views are always consistent
   against an older manifest until the new one lands».
5. `.gitignore` — только если файла ещё нет (:55; функция :67-82, exists-check
   на :69-71).
6. `README.md` — только если файла ещё нет (:56; функция :84-126, exists-check
   на :86-88).
7. `println!("Initialised empty index for …")` (:57-63).

`--force` делает РОВНО одну вещь — пропускает guard (:47). Чего он НЕ делает:
не сносит каталог (всё, что вне зон `write_to`, живёт — прежде всего `state/`
целиком: серверный lock, checkpoint, а после Ф3.1 — журнал); не трогает
`.gitignore`/`README.md` (тест `init_preserves_existing_readme_on_force`,
`crates/vibe-index/tests/cli_lifecycle.rs:169-191`); не сбрасывает журнальную
правду (журнал append-only, сброса не существует — store.rs:43-58). Побочный
эффект сегодня: `--force` с ДРУГИМ `--registry` перепишет идентичность в
repomd (:53-54), а README останется со строками старой идентичности
(формат-строки :89-93) — расхождение repomd-vs-README никто не ловит.

Цена второй записи (первый журнал-факт от init):

- Куда писать: `journal::default_dir(data_dir) = data_dir/state/journal`
  (`crates/vibe-index/src/journal/store.rs:29-31`); `append` = `create_dir_all`
  + append + `sync_all` (:43-58); append-only, atomic_write НАМЕРЕННО не
  используется — «a journal that rewrites its own past is not a journal»
  (store.rs:4-10).
- Затронутые строки: новый вызов между init.rs:53 и :54 (правда-первым) либо
  после :54; плюс импорт journal-типов к :13-14. Событие уже описано дизайном:
  `Event::Initialised { registry, registry_url, naming }` с доккоментом «written
  by `init` as the journal's first record» (record.rs:36-41).
- `actor`: нужен источник строки. Тестовый образец — «vibe-index 0.1.0-dev»
  (journal/tests.rs:32); продакшн-аналог `default_generator() =
  format!("vibe-index {}", env!("CARGO_PKG_VERSION"))` — ПРИВАТНАЯ fn
  (memory.rs:382-384), init.rs её не видит: либо делать pub, либо дублировать
  формат-строку.
- Частичный отказ, журнал-первым: append успел, `write_to` упал → каталог без
  repomd; guard следующего init смотрит ТОЛЬКО `repomd::exists` (init.rs:47) →
  повторный init без `--force` пройдёт и аппендит ВТОРОЙ `Initialised`.
  Дубликат идентичности в правде — проектор обязан иметь правило
  (последний-побеждает либо watershed в духе `EntrySetReplaced`, record.rs:103-109).
- Частичный отказ, каталог-первым: `write_to` успел, append упал (`?` вернёт
  Err после успешной проекции) → каталог несёт индекс, правда пуста; `replay`
  молчит — «A missing journal directory is an empty history, not an error»
  (store.rs:62-63). `rebuild --check` на таком каталоге провалится «проекция
  пуста ≠ каталог» — и это правильный сигнал: правда потеряна.
- Прецеденты «две записи обязаны лечь вместе» в дереве: (1) доминирующий —
  `write_to` сам пишет десятки файлов не-атомарно, консистентность даётся
  ПОРЯДКОМ: манифест последним (memory.rs:190-196, :302) — не транзакция, а
  упорядочение+штамп; (2) `codemod add-cell` — «All-or-nothing: files are
  written together and rolled back if the post-check fails»
  (xtask/src/main.rs:236-238) — rollback-прецедент, но реализация в пакетном
  крейте (вне периметра); (3) журнал нарочно отказался от atomic_write
  (store.rs:4-10). Вывод: сквозной транзакции «журнал+каталог» в дереве нет;
  ближайший законный паттерн — правда-первым (правда переживает отказ
  проекции), а дубликат от повторного init — правилом проектора.

Тесты, утверждающие поведение init (все — `crates/vibe-index/tests/cli_lifecycle.rs`,
через бинарник: `cmd() = vibe_test_support::cargo_bin("vibe-index")` :9-11):

- `init_creates_repomd_and_empty_primary` :14-37 — stdout, наличие
  repomd/primary, содержимое repomd;
- `init_refuses_existing_index_without_force` :40-57 — второй init failure +
  stderr-строка, третий с `--force` success;
- `init_writes_primary_jsonl_gz_alongside_plain` :134-139;
- `init_seeds_empty_repomd_with_inverted_dirs` :142-153;
- `init_writes_gitignore_and_readme` :156-166;
- `init_preserves_existing_readme_on_force` :169-191;
- смежные: dump :60-87, verify :90-131 — на фоне `init_at` :193-205.

Кто из них заметит журнальную запись: НИКТО. Все ассерты — «содержит/существует»;
ни один не обходит дерево целиком и не утверждает отсутствие лишних путей;
`verify` читает только `manifest.files` (verify.rs:78) и в `state/` не глядит.
Единственный полный обходчик — `walk` в memory/tests.rs:379-394 — юнит-уровень
`write_to`, init не касается. Новая запись пройдёт все тесты незаметно → вместе
с проводкой нужен НОВЫЙ тест: `replay(default_dir(data))` после init ==
`[Initialised]`.

## 5. Форма архитектурного гейта под G4

Полный перечень grep-подобных гейтов `tools/self-check.sh`:

- `check_index_clock_gate` (:550-570) — запрещает `Utc::now(|SystemTime::now(`
  в `crates/vibe-index/src/{index,types,journal}`; фильтр строк-комментариев по
  форме строки (:557); сообщение: правило → причина (PROP-044 §4.3, F2-1) →
  починка.
- `check_lane_citations` (:588-602) — запрещает `@spec://…/boot/STATIC#` и
  директивы `#use/#embed/#source` на STATIC-лейн в `*.md` под spec/, packages/,
  crates/; исключения :594-595.
- `check_member_licence_keys` (:614-639) — каждому члену workspace нужны
  `license(-file)` и `publish` ключи (список членов выводится из корневого
  Cargo.toml, :616-617).
- `check_core_slot_is_authored` (:368-377) — `grep -qF 'source_root = …'` по
  sync-engines.toml; при провале печатает и РЕЦЕПТ-команду `grep -n …` (:375).
- `check_floor_denominator` (:163-195) — сверка множеств: производные live-слоты
  (:145-157) vs `GATED_SLOTS`; сообщение называет провал в обе стороны + мораль
  (:188-189).
- `check_mcp_authored_denominator` (:513-530) — awk/comm: authored-крейты vs
  `conform.toml roots`; многострочное «classify the newcomer…».
- `check_instruction_triple` (:208-218) — `cmp -s` побайтово
  CLAUDE=AGENTS=GEMINI.
- Обвязка: `run_step` (:97-110) — заголовок шага, «self-check: \`label\` failed
  (exit rc)», exit на первом провале (или `--keep-going`), `OVERALL` в конце
  (:672-680).

Образец формы — целиком, `tools/self-check.sh:550-570`:

```sh
check_index_clock_gate() {
  local hits
  hits=$(grep -rnE 'Utc::now\(|SystemTime::now\(' \
      crates/vibe-index/src/index \
      crates/vibe-index/src/types \
      crates/vibe-index/src/journal \
      2>/dev/null \
    | grep -vE ':[0-9]+:[[:space:]]*//')
  if [ -n "$hits" ]; then
    printf '%s\n' "$hits" >&2
    printf 'self-check: the index writer modules call the clock directly.\n' >&2
    printf 'self-check: the rule — time enters at the edge (CLI command or\n' >&2
    printf 'self-check: server mutation event) and never inside index/, types/ or journal/:\n' >&2
    printf 'self-check: one state must produce one byte sequence, or "rebuild and\n' >&2
    printf 'self-check: compare" measures nothing (PROP-044 §4.3, F2-1).\n' >&2
    printf 'self-check: fix: pass the time as an argument — a WriteCtx for\n' >&2
    printf 'self-check: write_to, an `at` for Index::new / VersionEntry::minimal.\n' >&2
    return 1
  fi
  return 0
}
```

Почему выбран именно он: (1) периметр назван МОДУЛЬНЫМИ каталогами
(`crates/vibe-index/src/index|types|journal`), а не маской по репозиторию —
новый файл под ними покрыт в день посадки (:539-543); (2) есть фильтр ПО ФОРМЕ
СТРОКИ — доккомменты легальны, код нет (:545-549, :557); (3) сообщение —
полный канон правило→причина→починка, который G4 хочет повторить; (4) он уже
охраняет закон Ф3.2 (детерминизм = измерительный прибор). G4 — тот же жанр:
сигнатурное правило над писателями vibe-index, проверяемое grep-шагом панели.

Свип кандидатов под БУКВАЛЬНОЕ G4 («функция, пишущая каталог, принимает тип,
который читается из каталога») по `crates/vibe-index/src/index/` и
`crates/vibe-index/src/cli/`:

- `Index::write_to(&self, data_dir, ctx)` — memory.rs:197; `self: Index` —
  ровно то, что возвращает `load_from` (memory.rs:315); на этом стоит весь
  RMW-цикл (add/remove/reindex — территория находки f3, здесь только фиксация).
- `by_name::write(data_dir, entry: &NameEntry)` — by_name.rs:46; `NameEntry` =
  выход `read/read_all/parse` (by_name.rs:57/:72/:66).
- `primary::write(dir, entries: &mut [VersionEntry])` — primary.rs:43;
  `VersionEntry` = выход `read/parse` (primary.rs:75/:84).
- `repomd::write(data_dir, r: &Repomd)` — repomd.rs:27; `Repomd` = выход
  `read` (repomd.rs:32).
- `checkpoint::save(data_dir, &Checkpoint)` — checkpoint.rs:72; `Checkpoint` =
  выход `load` (checkpoint.rs:55) — но это `state/`, не каталог-каталог.
- Чистый контраст: `inverted::write_capability/write_purl`
  (inverted.rs:200-226) принимают `CapabilityRow/PurlRow`, которые с диска НЕ
  читаются (в inverted.rs нет их read/parse; строятся
  `InvertedView::from_entries`, inverted.rs:105-133) — по буквальному правилу
  НЕ нарушение.
- `cli/`: сигнатуры несут только Args (`pub fn run(args: …)` по всем
  подкомандам; диспетчер `dispatch(command)` — cli/mod.rs:117); каталогных
  типов в СИГНАТУРАХ cli нет — RMW-поток живёт внутри тел функций.

Прямо о безобидном: **правило в буквальном прочтении ловит ВСЮ поверхность
записи** — читатели и писатели делят типы ПО ПОСТРОЕНИЮ (пары
serialise/parse: by_name.rs:35/:66, primary.rs:27/:84, repomd.rs:16/:32; сам
`write_to`). Тип — не происхождение: тот же `VersionEntry` рождается и из
журнала, и из сканера, и из `load_from`. Греп по именам типов в сигнатурах
зажжёт все пять кандидатов разом и не отличит журнал-проекцию от
read-modify-write. Это важнее списка: G4-шаг панели обязан целить не типы, а
поток (например, запрет связки `load_from(…)` → `write_to(…)` в одном пути —
но это уже не сигнатурный грей), либо формулировать правило только для НОВОЙ
поверхности — сигнатуры проектора (`project(events) -> Index`, где события —
типы `journal::record`), не задним числом для существующих писателей.

## 6. Оснастка побайтового сравнения деревьев

Помощник — целиком, `crates/vibe-index/src/index/memory/tests.rs:357-394`:

```rust
/// Walk both trees and compare every file path-for-path,
/// byte-for-byte. File *sets* must match too — a stray or missing
/// file is as much a difference as a changed byte.
#[cfg(test)]
fn assert_trees_byte_identical(a: &std::path::Path, b: &std::path::Path) {
    let mut la = walk(a);
    let mut lb = walk(b);
    la.sort();
    lb.sort();
    assert_eq!(la, lb, "the two trees hold different file sets");
    for rel in &la {
        let ca = std::fs::read(a.join(rel)).unwrap();
        let cb = std::fs::read(b.join(rel)).unwrap();
        assert_eq!(
            ca,
            cb,
            "{} differs byte-for-byte between the two writes",
            rel.display()
        );
    }
}

#[cfg(test)]
fn walk(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(&dir).unwrap() {
            let e = e.unwrap();
            if e.file_type().unwrap().is_dir() {
                stack.push(e.path());
            } else {
                out.push(e.path().strip_prefix(root).unwrap().to_path_buf());
            }
        }
    }
    out
}
```

- pub или тестовый: ТЕСТОВЫЙ — `#[cfg(test)]`-приват внутри тестового модуля
  `index::memory` (подключён как `#[cfg(test)] #[path = "tests.rs"] mod tests`,
  memory.rs:386-388); из xtask недоступен ни при каком import.
- что сравнивает: и то и другое — множество путей (sorted Vec, :362-366) И байты
  каждого файла (:367-376); обход рекурсивный, БЕЗ deny-листа (walk :379-394
  берёт всё, включая `state/`, окажись он там).
- как сообщает: паника assert_eq с именем пути; ПЕРВЫЙ diff останавливает
  (не список) — для теста нормально, для xtask-гейта слабо.

Другие места, сравнивающие деревья/каталоги побайтово (свип):

- `xtask/src/sync_engines.rs:159-217` — `file_set`+`diff_crate`: те же
  путь-множество+байты, но в ПРОДАКШН-коде xtask, с классификацией
  missing/extra/changed и полным СПИСКОМ drift (форма сообщения — §3(в)).
- `tools/self-check.sh:208-218` — `cmp -s` двух файлов (instruction triple).
- `tools/user-home-tripwire.sh` — снапшот/сравнение пер-юзерного дома
  hash-на-путь (подключён self-check.sh:279-291; «Hash-and-path only — never
  contents», self-check.sh:57-64).
- `check-codegen` — `git diff --exit-code` (codegen.rs:269-277): сравнение на
  стороне git.
- `specmap --check` — байт-diff, реализация в packages/ (вне периметра).
- `verify` (crates/vibe-index/src/cli/verify.rs:73-131) — НЕ сравнение двух
  деревьев: сверка sha256/size файлов ПРОТИВ МАНИФЕСТА; итерация по
  `manifest.files` (:78) — ЛИШНИЕ файлы не ловит.

Переиспользуемо для `rebuild --check` vs писать заново:

- Как есть: `repo_root()`; механика `file_set`/`diff_crate` — она уже В XTASK,
  оба потребителя в одном крейте, поднять видимость внутри xtask дешевле, чем
  что-либо экспортировать из vibe-index.
- Заново (решение, которого нет ни в одном готовом компараторе): судьба
  нетранзакционных частей каталога. `state/` — это ВХОД проекции (журнал) и
  рантайм (lock, checkpoint), НЕ её выход: сносить нельзя, сравнивать нельзя.
  `README.md`/`.gitignore` — init-обвязка, НЕ проекция `write_to`
  (memory.rs:197-303 их не пишет) — rebuild их не создаст. Значит «снести
  каталог, спроецировать, сравнить байты» в лоб неформулируемо над живым
  каталогом: либо сравнивать только проекционное подмножество путей
  (repomd.json, primary*, by-name/, by-cap/, by-purl/), либо rebuild в чистый
  temp + сравнение подмножества. Это и есть «новое» в оснастке — остальное
  готово.
- `assert_trees_byte_identical` переносить не стоит: 38 строк под `#[cfg(test)]`
  с паникой-на-первом; правильный донор — sync-engines-механика (продакшн,
  список drift, exit-код через anyhow).

## 7. Фикстуры и герметичность

Приёмы построения герметичного data-dir в тестах vibe-index:

- Интеграционные (через бинарник): `cmd() = vibe_test_support::cargo_bin("vibe-index")`
  (cli_lifecycle.rs:9-11; реализация crates/vibe-test-support/src/lib.rs:45);
  каталог — `tempfile::tempdir()` (:15 и далее); посев — РЕАЛЬНЫЙ прогон CLI:
  хелпер `init_at(dir)` (cli_lifecycle.rs:193-205). Герметичность по дому —
  vibe-test-support как dev-dep ИЗОЛИРУЕТ настройки до первого `#[test]`
  (vibe-index/Cargo.toml dev-dependencies + коммент «isolates the test
  process's per-user settings home»; механизм описан в корневом Cargo.toml:80-82,
  DRIFT-020).
- Юнит-тесты памяти: tempdir + `fresh_index()` = `Index::new` с ФИКСИРОВАННОЙ
  меткой `now() = 2026-05-06T12:00:00Z` (memory/tests.rs:16-35) + `write_to` с
  тем же `WriteCtx { at }` (:38-40) — герметичность и по файлам, и по часам
  (F2-1).
- Журнал: tempdir + `append`/`replay`, метки — RFC3339-константы через `at()`
  (journal/tests.rs:15-19); round-trip на все 11 вариантов событий (:197-296);
  байт-детерминизм одной записи в двух каталогах (:180-194).
- Комmitted-фикстуры: `crates/vibe-index/fixtures/` — золотые ПАКЕТЫ
  (`golden-flow-wal-0.1.0/`, `golden-order-trap-0.1.0/`) для content-hash
  (crates/vibe-index/tests/content_hash_parity.rs:1-60). Это входы ХЭШЕРА,
  НЕ каталоги данных.

Готовая фикстура каталога данных под вход `rebuild --check`: НЕТ. Самый дешёвый
способ породить её из существующего: интеграционный паттерн cli_lifecycle —
tempdir → `init_at(dir)` реальной командой (уже обёрнут) → досыпать правду
прямыми `journal::append` в `default_dir(dir)` (Initialised уже будет, если
проводка §4 сделана; Published — из `VersionEntry::minimal`, как в
journal/tests.rs:39-45). Тогда проверка = `replay` → `project` → `write_to` в
другой temp → сравнение проекционного подмножества (§6). Ни одной новой
committed-фикстуры не нужно; всё порождается в tempdir на лету.

## 8. Дыры и неожиданности

1. **B5 опровергнута**: xtask не зависит ни от одного продуктового крейта
   (xtask/Cargo.toml:13-25), и `vibe-index` отсутствует в
   `[workspace.dependencies]` корня (Cargo.toml:66-88 — там есть vibe-registry
   на :73, но не index). Подключение тронет два манифеста.
2. **specmap --check не разобран дословно** — реализация в пакетном крейте
   `packages/…/rust-ai-native-specmap`, вне периметра чтения; вместо него
   дословно взят `sync-engines --check` как ближайший по задаче. Осознанное
   отклонение, зафиксировано здесь.
3. **Журнал уже есть, проводки init→journal нет**: `Event::Initialised`
   документирован как первая запись init (record.rs:36-41), но init.rs его не
   пишет. Разрыв ровно между Ф3.1 и Ф3.2.
4. **Журнал живёт под gitignored `state/`** (store.rs:29-31): правда каталога
   по умолчанию НЕ git-трекаема; rebuild обязан трактовать `state/journal` как
   вход, а не как проекцию.
5. **verify не ловит лишние файлы** (итерация по manifest.files,
   verify.rs:78-118) — как компаратор для rebuild непригоден.
6. **Буквальное G4 ловит всю поверхность записи** (типы делятся
   читателем/писателем по построению) — гейт обязан целить поток/происхождение
   или только новую поверхность; детали в §5.
7. **Дублирование нумерации шагов в self-check.sh**: два разных шага
   подписаны «11b» (lane-citations :580 и markup-валидация :643) — косметика,
   но комментарий-якорь «# 11b» двоится.
8. **`--force` при живом журнале создаст вторую идентичность в правде**:
   guard смотрит только repomd (init.rs:47), сброса/терминирования журнала нет
   (append-only, store.rs:43-58).
9. **README/.gitignore переживают `--force` с чужой идентичностью** —
   расхождение repomd-vs-README никто не ловит (init.rs:84-126;
   cli_lifecycle.rs:169-191).
10. **Проверочных глаголов больше, чем B1**: пять (check-codegen, specmap
    --check, sync-engines --check, mirror --check, conform check) — идиома
    отработана, для Ф3.2 это плюс.

## 9. Как воспроизвести этот замер

По одной команде на глагол (замер чтением; сборка/git не участвовали):

- `cat xtask/Cargo.toml`
- `cat xtask/src/main.rs`
- `cat xtask/src/codegen.rs`
- `cat xtask/src/specmap.rs`
- `cat xtask/src/sync_engines.rs`
- `cat tools/self-check.sh`
- `cat crates/vibe-index/src/cli/init.rs`
- `cat crates/vibe-index/src/cli/verify.rs`
- `cat crates/vibe-index/tests/cli_lifecycle.rs`
- `cat crates/vibe-index/src/index/memory.rs`
- `cat crates/vibe-index/src/index/memory/tests.rs`
- `cat crates/vibe-index/src/index/persistence.rs`
- `cat crates/vibe-index/src/journal/mod.rs`
- `cat crates/vibe-index/src/journal/record.rs`
- `cat crates/vibe-index/src/journal/store.rs`
- `cat crates/vibe-index/src/journal/tests.rs`
- `cat crates/vibe-index/src/lib.rs`
- `grep -n "pub fn \|pub(crate) fn " crates/vibe-index/src/index/{by_name,primary,repomd,inverted,checkpoint}.rs`
- `grep -rn "Index::new" crates/ xtask/ --include="*.rs"`
- `grep -rn "fn project" crates/ xtask/ --include="*.rs"`
- `grep -rn "tempfile\\|tempdir" xtask/src/`
- `find crates/vibe-index/fixtures -type f`
