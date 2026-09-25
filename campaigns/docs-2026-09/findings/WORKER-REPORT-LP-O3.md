# Отчёт LP-O3 — `vibe doc check --chapters --json` стал зарегистрированным форматом

Дата: 2026-09-25. Дерево `C:\Users\olegc\git\v\vibevm`, ветка `main`, HEAD
`b168c9a91`. Git только читающий: ничего не добавлено в индекс, ничего не
закоммичено.

## Файлы

| файл | что с ним |
|---|---|
| `C:\Users\olegc\git\v\vibevm\schemas\doc_chapters.jtd.json` | новый — JTD-схема документа замера |
| `C:\Users\olegc\git\v\vibevm\formats\REGISTRY.toml` | запись `[format.doc-chapters]` + комментарий группы |
| `C:\Users\olegc\git\v\vibevm\crates\vibe-wire\src\generated\doc_chapters\mod.rs` | новый, **сгенерирован** `cargo xtask codegen` |
| `C:\Users\olegc\git\v\vibevm\crates\vibe-wire\src\generated\mod.rs` | сгенерирован — `pub mod doc_chapters;` |
| `C:\Users\olegc\git\v\vibevm\crates\vibe-wire\src\generated\format_id\mod.rs` | сгенерирован — вариант `DocChapters` |
| `C:\Users\olegc\git\v\vibevm\crates\vibe-wire\src\generated\format_id\properties.rs` | сгенерирован — шесть таблиц записи |
| `C:\Users\olegc\git\v\vibevm\crates\vibe-doc\src\chapters.rs` | строит и печатает сгенерированный тип; рукописные derive убраны |
| `C:\Users\olegc\git\v\vibevm\crates\vibe-doc\src\chapters\tests.rs` | тесты на сгенерированном типе |
| `C:\Users\olegc\git\v\vibevm\crates\vibe-cli\src\commands\doc.rs` | одна строка: `report.render()` → `chapters::render(&report)` |

Сгенерированное руками не правилось. TypeScript не появился: таблица целей
второго языка (`xtask/src/codegen/typescript.rs`, `const TARGETS`) содержит
ровно один элемент — `schemas/doc_manifest.jtd.json`. Сайту этот документ не
нужен, и реестр его туда не отправляет.

## Форма схемы

Корень `doc_chapters` → `DocChapters`; одно определение `forward_link` →
`ForwardLink`. Все семь членов корня обязательные (`properties`), как у
соседей.

| член | форма | примечание |
|---|---|---|
| `schema_version` | `uint32` | **добавлен**, см. ниже |
| `chapters` | `uint32` | было `usize` |
| `pages` | `uint32` | было `usize` |
| `forward_links` | `elements` → `forward_link`, `x-empty: "emit"` | обязательный список пишется и пустым |
| `forward_link_count` | `uint32` | было `usize` |
| `unreadable` | `elements` строк, `x-empty: "emit"` | то же |
| `measured` | `boolean` | |

`forward_link`: `page`, `target` — строки.

- **`x-empty: "emit"`** обязателен, а не выбран: проход
  `xtask/src/codegen/empty_policy.rs` отказывается генерировать коллекцию без
  аннотации, а для обязательного члена единственная законная политика — `emit`
  (правило R21). Это ровно поведение рукописного `Vec`: пустой список
  печатался и будет печататься.
- **`x-wire-order`** объявлен и на корне, и на `forward_link`. Без него
  `jtd-codegen` ставит поля по алфавиту, и печать превратилась бы в
  `chapters, forward_link_count, forward_links, measured, pages,
  schema_version, unreadable`. Из соседей его несёт `doc_manifest`
  (корень + три определения); `doc_todo` и `doc_surface_diff` его не несут и
  печатаются по алфавиту. Здесь порядок — часть уже выпущенного вывода, поэтому
  он объявлен: это единственный способ сохранить байты.
- Описания цитируют `##NAV-CHAPTERS-CHECKED` (и `##NAV-CHAPTERS` в корне) и
  повторяют довод нормы «замер, а не гейт» и «ноль не потому, что никто не
  смотрел». Проза полей ушла из `chapters.rs` в схему и вернулась в крейт
  доккоментами сгенерированного типа — одно описание на обоих читателей.

### `schema_version` — единственная разница в байтах

Каждый соседний машинный документ `vibe doc` несёт `schema_version: uint32`
первым членом и никакого конверта: `doc_todo`, `doc_surface`,
`doc_surface_diff`, `doc_manifest`. (`doc_reviews` несёт `schema` — это
авторский TOML, не печать команды.) Поэтому конверта нет и здесь, а
`schema_version` добавлен и стоит первым; в крейте он `pub const
chapters::SCHEMA_VERSION: u32 = 1`, по образцу `todo::SCHEMA_VERSION`.

Это **единственное** расхождение байтов с выводом до правки — одна строка,
дословный `diff` ниже. Человеческая форма (`render`) не изменилась ни на байт;
`ok()` по-прежнему нет, и довод, почему его не должно быть, перенесён на
свободную функцию.

## Запись реестра

Группа «Documentation maintenance» в
`C:\Users\olegc\git\v\vibevm\formats\REGISTRY.toml`: заголовок стал
`(5 records, 5 JTD)`, «all four» → «all five», к цитатам добавлен
`##NAV-CHAPTERS-CHECKED`; абзац про `doc-chapters` вставлен между
`doc-surface-diff` и `doc-todo`, запись — там же по порядку.

```toml
[format.doc-chapters]
epoch = 1
schema = "schemas/doc_chapters.jtd.json"
recoverable = true
foreign_parsers = "ours"
corpus = "none"
sunset = "none"
```

`recoverable = true` — это чтение дерева: повторный запуск проверки выдаёт
документ заново (тот же довод, что у `doc-todo`). `foreign_parsers = "ours"` —
документ печатается разработчику и читается только нашим кодом, поэтому
`unknown_fields` остаётся `allow`, как у всей группы, и проход `strictness` не
ставит `deny_unknown_fields`. `corpus = "none"` — золотых байтов у формата нет,
`wire-diff` его не судит.

Запись обязательна, а не «для порядка»: `xtask/src/codegen/strictness.rs`
отказывается генерировать схему, которую сканер видит, а реестр не называет.

## Счёт храповика рукописного wire

**Дефект пакета (пункт 5).** Шага wire-derive в `tools/self-check.sh` больше
нет. Коммит `558982a4a` «refactor(gates): narrow self-check to the host release
floor» (2026-09-20) сократил панель с 53 шагов до 7 и удалил вместе со шагом
`0d` файл базовой линии `wire-derive-baseline.json`:

```
$ grep -n 'run_step "' tools/self-check.sh
72:run_step "cargo fmt --all --check" \
74:run_step "cargo test --workspace" \
76:run_step "cargo clippy --workspace --all-targets -- -D warnings" \
78:run_step "vibe check --path . --quiet" \
80:run_step "cargo xtask conform check" \
82:run_step "cargo xtask check-codegen" \
84:run_step "cargo xtask wire-diff" \

$ grep -c "wire-derive" tools/self-check.sh
0
$ ls -la wire-derive-baseline.json
ls: cannot access 'wire-derive-baseline.json': No such file or directory
```

Останов не потребовался: и рецепт, и базовая линия восстановимы из истории
только чтением. Я воспроизвёл проверку дословно из `558982a4a^` (тот же
шаблон `^[[:space:]]*#\[derive\([^)]*(Serialize|Deserialize)`, тот же фильтр
`grep -v '/generated/'`, та же единица — файл) с той же базовой линией; ничего
в дереве не создавалось, скрипт и копия линии лежат в scratchpad. **Базовые
линии не поднимались — поднимать нечего, файла в дереве нет.**

Счёт `vibe-doc` (базовая линия `558982a4a^`: **2**):

| момент | файлы | счёт |
|---|---|---|
| до правки | `chapters.rs`, `derived.rs`, `examples/fixture.rs`, `prompts/config.rs` | **4** |
| после правки | `derived.rs`, `examples/fixture.rs`, `prompts/config.rs` | **3** |

```
$ git show HEAD:crates/vibe-doc/src/chapters.rs | grep -cE '^[[:space:]]*#\[derive\([^)]*(Serialize|Deserialize)'
2
$ grep -cE '^[[:space:]]*#\[derive\([^)]*(Serialize|Deserialize)' crates/vibe-doc/src/chapters.rs
0
```

`chapters.rs` больше не считается вовсе: счёт из-за него не вырос, а **упал на
единицу**. Остаток `3 > 2` — это дрейф, который B-173 и фиксирует
(`vibe-doc` 3/2 на 2026-09-17, до появления `chapters.rs`); три оставшихся
файла лежат вне периметра пакета, и я их не трогал. Полное чтение храповика
сегодня: `vibe-core` 36/35, `vibe-doc` 3/2, `vibe-registry` 9/8, `xtask` 16/14,
`vibe-doc-shell` 2/отсутствует, остальные тринадцать крейтов совпадают
(122 файла).

## Самопроверка — дословно

```
$ cargo fmt --all -- --check
EXIT=0

$ cargo xtask codegen
📦     Definition "translation_status" converted into type: TranslationStatus
📦     Definition "version" converted into type: Version
  - schemas/doc_manifest.jtd.json → C:\Users\olegc\git\v\vibevm\vibevm/vibepacks/org.vibevm.doc/web/v1.0.0/site/src/generated\doc-manifest.ts
EXIT=0

$ cargo test -p vibe-doc chapters
running 6 tests
test chapters::tests::a_fixture_package_that_declares_no_path_is_read_and_left_unmeasured ... ok
test chapters::tests::a_package_without_a_path_is_not_measured_and_says_so ... ok
test chapters::tests::an_anchor_a_citation_and_a_page_off_the_path_are_not_counted ... ok
test chapters::tests::a_link_ahead_is_counted_and_a_link_back_is_not ... ok
test chapters::tests::a_link_into_an_appendix_chapter_is_not_a_link_ahead ... ok
test chapters::tests::the_json_carries_the_pairs_and_their_count ... ok
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 570 filtered out; finished in 0.00s
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 0.00s
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 0.00s
EXIT=0

$ cargo test -p vibe-cli --bin vibe commands::doc::
running 34 tests
test result: ok. 34 passed; 0 failed; 0 ignored; 0 measured; 783 filtered out; finished in 0.36s
EXIT=0

$ cargo clippy -p vibe-doc -p vibe-wire --all-targets -- -D warnings
    Checking vibe-doc v1.0.0 (C:\Users\olegc\git\v\vibevm\crates\vibe-doc)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 10.03s
EXIT=0

$ cargo build -p vibe-cli
   Compiling vibe-cli v1.0.0 (C:\Users\olegc\git\v\vibevm\crates\vibe-cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 18.13s
EXIT=0

$ target/debug/vibe.exe doc check --chapters --json --path vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0
{
  "schema_version": 1,
  "chapters": 10,
  "pages": 49,
...
  "forward_link_count": 18,
  "unreadable": [],
  "measured": true
}
EXIT=0

$ diff before.json after.json          # вывод до правки против вывода после
1a2
>   "schema_version": 1,
DIFF_EXIT=1

$ bash wire-derive-ratchet.sh wire-derive-baseline.json   # шаг 0d из 558982a4a^
wire-derive: `progress-core` holds at 11
wire-derive: `vibe-actions` holds at 7
wire-derive: `vibe-agent-projection` holds at 1
wire-derive: `vibe-cli` holds at 44
wire-derive: `vibe-core` GREW: 36 file(s), baseline froze 35
wire-derive: `vibe-doc-shell` carries 2 file(s) and is ABSENT from the baseline
wire-derive: `vibe-doc` GREW: 3 file(s), baseline froze 2
wire-derive: `vibe-facts` holds at 2
wire-derive: `vibe-index` holds at 27
wire-derive: `vibe-lifecycle` holds at 2
wire-derive: `vibe-mcp` holds at 7
wire-derive: `vibe-publish` holds at 7
wire-derive: `vibe-registry` GREW: 9 file(s), baseline froze 8
wire-derive: `vibe-scrape` holds at 5
wire-derive: `vibe-trace` holds at 3
wire-derive: `vibe-wire` holds at 2
wire-derive: `vibe-workspace` holds at 4
wire-derive: `xtask` GREW: 16 file(s), baseline froze 14
wire-derive: 122 file(s) in the crates that match the baseline exactly
EXIT=1
```

### `check-codegen` — идемпотентность вместо зелёного кода

`cargo xtask check-codegen` красный, и ровно по названной в пакете причине:
`xtask/src/codegen/mod.rs:437` сначала выполняет `run_codegen()`, а затем
`git diff --exit-code` по деревьям вывода, то есть сравнивает с
**закоммиченным** состоянием. Регенерацию коммитит босс, поэтому шаг красный до
коммита по построению. Названы ровно три изменённых файла (`generated/mod.rs`,
`format_id/mod.rs`, `format_id/properties.rs`) — все сгенерированные.

Доказательство идемпотентности (как в отчёте LP-O1): хеш всего дерева
`crates/vibe-wire/src/generated` совпадает после трёх последовательных
`cargo xtask codegen`:

```
$ find crates/vibe-wire/src/generated -type f -name '*.rs' | sort | xargs sha256sum | sha256sum
dbcaaabd9b687752afef3b19117d7ee1ddfc7360c632ff0b136ccf07e47e93cb *-
$ cargo xtask codegen; # EXIT=0
$ find crates/vibe-wire/src/generated -type f -name '*.rs' | sort | xargs sha256sum | sha256sum
dbcaaabd9b687752afef3b19117d7ee1ddfc7360c632ff0b136ccf07e47e93cb *-
```

Второй и третий прогон не добавили ни одного изменённого файла: `git status`
после каждого — те же три `M` плюс новый каталог `doc_chapters/`.

## Отклонения

1. **Дефект пакета, обойдён чтением истории.** Пункт 5 велит найти шаг
   wire-derive в `tools/self-check.sh`; шага и его базовой линии в дереве нет с
   `558982a4a` (2026-09-20). Проверка воспроизведена дословно из истории,
   изложено выше. Ничего не создано в дереве, базовые линии не тронуты.
2. **`schema_version` меняет байты `--json` на одну строку.** Разрешено
   пунктом 1 пакета и обосновано выше: все соседние машинные документы
   `vibe doc` его несут первым членом.
3. **`Report::render()` стал свободной функцией `chapters::render(&report)`.**
   Иначе нельзя: inherent-метод на чужом типе не объявить. Это идиома соседа
   (`todo::report::render_md(&DocTodo)`). Затронута одна строка
   `crates/vibe-cli/src/commands/doc.rs` — периметр это разрешает; печатаемый
   текст человеческой формы не изменился.
4. **`usize` → `u32`** в четырёх числах — следствие схемы (проект держит
   схемы без float и с `uint32`). На JSON не влияет; тесты сравнивают с
   целочисленными литералами и не менялись.
5. **Найдено, не исправлено (вне периметра).** `cargo test -p vibe-wire` даёт
   одно падение, существовавшее до этой работы:
   `crates/vibe-wire/tests/requirements_report_wire.rs:102`
   `the_format_is_inventoried_under_a_surface_neutral_id` требует ровно 13
   записей `cli-*`, а реестр несёт 14 — столько же на `HEAD`
   (`git show HEAD:formats/REGISTRY.toml | grep -c '^\[format\.cli-'` → `14`),
   и мой диф не касается ни одной записи `cli-*`. Кандидат в BACKLOG; трогать
   `crates/vibe-wire/tests/**` пакет не разрешает.

Ничего не заиндексировано и не закоммичено; сайт, спеки, пакеты руководства,
`specmap.json` и базовые линии храповиков не тронуты.
