# WORKER-REPORT-P2-O2 — словарь документации в пивоте (A2.8)

Пакет: `campaigns/docs-2026-09/findings/PACKET-P2-O2.md`
Дата: 2026-09-12. Ветка: `research-preview-1-docs`. Push не делался.

## Коммит

`a7bff252` — `feat(specdoc): give the documentation genre its seven elements`

Отчёт идёт отдельным коммитом `docs(campaign): …` — так же, как отчёты
предыдущих волн кампании: продуктовый атом остаётся одним коммитом и не
может содержать собственный хэш.

## Что сделано

Второй словарь диалекта в пивоте `vibe-specdoc`, включаемый параметром
читателя; спековый словарь не изменён ни на байт.

**Новые файлы**

- `crates/vibe-specdoc/src/xml_doc.rs` — чтение семи элементов жанра.
- `crates/vibe-specdoc/src/xml_doc_tests.rs` — 29 юнит-тестов жанра.
- `crates/vibe-specdoc/tests/docs_corpus.rs` — 6 корпусных тестов на живых
  44 страницах руководства.

**Изменённые файлы** (все — пивот и два внешних потребителя IR)

`crates/vibe-specdoc/src/{doc,lib,load,convert,md_in,md_out,xml_in,xml_blocks,xml_facts,xml_out,xml_support}.rs`,
`crates/vibe-specdoc/src/{convert_tests,md_in_tests,xml_in_tests}.rs`,
`crates/vibe-specdoc/tests/redbook_roundtrip.rs`,
`crates/vibe-facts/src/report.rs`,
`crates/vibe-workspace/src/vibedeps/derived.rs`.

`vibe-core`, `vibe-wire`, `formats/`, `vibe-install`, `xtask`,
`vibe-cli/src/commands/show` — не тронуты (пакеты параллельных воркеров).

## Решения

### 1. Флаг жанра — параметр читателя, аддитивные входы

`doc::Vocabulary::{Spec, Doc}`, `Spec` — `#[default]`. Четыре аддитивных
входа: `from_xml_with`, `load_spec_text_with`, `project_spec_text_with`,
`convert_with`; старые четыре — тонкие обёртки над ними с `Spec`, поэтому
ни один из 37 существующих вызовов не изменён.

Почему не «загрузчик берёт kind сам» (ожидание A0.4 подтвердилось):
`load.rs` чист и решает по одному расширению файла, а крейт по закону
отделимости не зависит ни от одной подсистемы vibevm, то есть `PackageKind`
в пивот не приходит **никогда**. Отображение «kind пакета → словарь» —
дело вызывающей стороны (`DOC-VOCAB-BY-KIND`). Документ о своём словаре не
заявляет ничего.

Вне режима — громкая ошибка, называющая жанр и цитирующая норму:

```
<note> belongs to the documentation vocabulary, which is open only in
packages of kind `doc` — see
spec://org.vibevm.core/vibevm/common/PROP-045#DOC-VOCAB-BY-KIND
```

Дискриминатор (`DOC-VOCAB-DISCRIMINATOR`) реализован буквально: блок жанра
**никогда** не несёт `title=`, именованная секция несёт всегда. Поэтому
восемь канонических файлов с `<example title=…>` / `<rule title=…>` читаются
как секции под **обоими** словарями, а чёрный список `anchor_is_elementable`
**не растёт** — писатель остаётся словарно-слепым, и один IR даёт одни байты
под любым словарём (тест `the_writer_is_vocabulary_blind`).

### 2. `when` живёт в слоте, а не в варианте блока

`BlockNode { when: Option<Cond>, block: Block }`; `SpecDoc.preamble` и
`Section.blocks` — `Vec<BlockNode>`; у `Section` своё поле `when`.

**Это отклонение от буквы пакета** (в цели пакета `when` перечислен полем
`Block::Example`), но точное исполнение нормы, которую пакет назвал
обязательной: `PROP-045 §7 ##DOC-VOCAB-WHEN-SLOT` — «`when` is a property of
the slot, not of a block kind: `Section.blocks` and `SpecDoc.preamble` hold
`BlockNode { when, block }`, so a condition applies to any block and to
sections», и `ROW-DOCVOCAB-WHEN` — «the attribute `when` on **any block** and
on sections». Слот-форма выполняет и требование пакета (`Example`, `Note`,
`Prompt` получают `when` через слот), и норму (его получают также `p`,
`list`, `table`, `fence`, `quote`, `facts`), и не создаёт нелегальных
состояний: условие не дублируется в семи вариантах и не бывает «наполовину»
на блоке. Цена — механическая правка ~50 мест в крейте и двух внешних
исчерпывающих `match` (компилятор перечислил их сам).

`Cond` — закрытый список: `os:windows|macos|linux` и
`agent:claude|claude-desktop|cursor|opencode|codex`, с подсказкой
`nearest` на ошибке. Оба перечня — санкционированное дублирование словарей,
живущих в `vibe_core::manifest::WhenCondition` и
`vibe_agent_projection::agents::Agent` (эджа к ним у пивота нет по закону
отделимости); единственный дом каждого назван в doc-комментарии.
`installed:` сознательно **не** включён: страница варьируется платформой и
агентом читателя, а не составом чьего-то проекта. Под словарём `Spec`
`when` остаётся чужим атрибутом и отвергается, как и раньше.

### 3. Формы вариантов

| вариант | поля | заметка |
| --- | --- | --- |
| `Example` | `id, fixture, lang, exit, run, expect, stderr` | `exit: Option<i32>`: `None` — атрибута не было (код 0 по умолчанию), `Some(0)` — написан явно; различимость нужна для байтовой стабильности, как у `Fence::lang` |
| `ExampleRef` | `id` | тела не несёт |
| `Rule` | `uri, rev: Option<u32>` | `uri` без `~rN`; `rev` записан, чтобы байты автора выжили, и **никогда** не используется цитатой (`DOC-VOCAB-RULE-ADDRESS`) |
| `Derived` | `kind: DerivedKind, reference` | текст не хранится |
| `Note` | `kind: NoteKind, body: Unit` | `Unit` ⇒ врезка адресуема якорем факта |
| `Figure` | `src, alt, caption: Unit` | `alt` обязателен |
| `Prompt` | `id, text, needs, outcome, asserts` | `needs`/`outcome` — по одному, проза (`Option<String>`); `asserts` пуст только при `assert="none"` |

Порядок детей фиксирован (`run`→`expect`→`stderr`; тело→`needs`→`outcome`→
`assert`+), нарушение — громкая ошибка: писатель пишет один порядок, и
принять другой значило бы сломать байтовую идемпотентность в тот день,
когда такой файл появится.

Тексты `run`/`expect`/`stderr`/`assert` — дословные, **не** триммятся:
в корпусе есть `<expect>` с ведущим `\n` и внутренней пустой строкой, и это
содержимое, а не отступ. Тело `prompt` — единственное исключение: оно
триммится по краям, потому что отступ между телом и первым ребёнком
принадлежит писателю (иначе байтовый round-trip невозможен в принципе).
CDATA разрешена ровно в этих пяти местах явным списком; сообщение об
ошибке в остальных местах этот список называет.

Пустой `<expect></expect>` пишется **парой тегов**, никогда `<expect/>`:
это утверждение «команда ничего не печатает», а не отсутствие; в корпусе
такая форма встречается 20 раз.

`example id` и `prompt id` чеканятся в общее пространство имён документа
(`mint_fact`), поэтому оба адресуемы как `spec://…#id` и не могут
столкнуться ни друг с другом, ни с якорем секции, ни с фактом.

### 4. `md_out` — по таблице PROP-045 §7

`example` → соседние fence `sh` (или `lang`) и `output` (+ `stderr`);
`example ref` → строка «Example \`id\` is copied from the source page at
projection time.» (в пивоте источника нет — копирование fence источника
делает конвейер); `rule` → цитата с автоссылкой `> <spec://…#ANCHOR>` без
ревизии; `derived` → fence `text` с пометкой «generated from kind: ref»;
`note` → цитата с меткой вида в первой строке; `figure` → изображение плюс
абзац подписи; `prompt` → fence `prompt`, список «needs», абзац «outcome»,
список ассертов; `when` на блоке → подзаголовок с условием, `when` на
секции → её собственный заголовок называет условие.

**Две сверки с текстом пакета.** Пакет в скобках пересказывает таблицу чуть
иначе: «`note` → абзац с меткой вида» (норма: «a quote with the kind label in
its first line») и «`when` → пометка блока» (норма: «a sub-heading named
after the platform»). Пакет сам называет нормой §7 («точный список… MD-проекций»),
поэтому реализована таблица; расхождение фиксирую здесь.

**Принятая потеря проекции:** `exit` и `fixture` в Markdown не выводятся —
таблица нормы перечисляет только два (три) fence. Проекция односторонняя по
закону, потеря записана.

## Гейты — вывод дословно

```
$ cargo fmt --all --check
(exit 0, вывода нет)

$ cargo build --workspace
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 28.92s

$ cargo test -p vibe-specdoc
test result: ok. 114 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.02s
(было до атома: 85 + 7 + 3 = 95; стало 114 + 6 + 7 + 5 = 132)

$ cargo test -p vibe-check -p vibe-spec -p vibe-facts
test result: ok. 72 passed; 0 failed; ...      (vibe-check lib)
test result: ok. 6 passed; 0 failed; ...       (check_cells_oracle)
test result: ok. 26 passed; 0 failed; ...      (vibe-facts lib)
test result: ok. 962 passed; 0 failed; ...     (vibe-spec lib)
test result: ok. 5 passed; 0 failed; ...       (vibe-spec compile)
test result: ok. 2 passed; 0 failed; ...       (vibe-spec embed)
test result: ok. 7 passed; 0 failed; ...       (vibe-spec resolve)
test result: ok. 9 / 8 / 4 passed              (doctests)

$ cargo test -p vibe-cli --bins
test result: ok. 712 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 28.05s

$ cargo test -p vibe-workspace
test result: FAILED. 539 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out; finished in 23.22s
    bins::tests::collect_walks_lockfile_slots_and_sorts
    bins::tests::explicit_build_authorization_preserves_direct_consent_semantics
    bins::tests::unknown_binary_names_the_known_set
  — НЕ мои: «unsupported vibe.lock schema version 6 — expected 7».
    Файл теста `crates/vibe-workspace/src/bins/tests.rs` в дереве не изменён
    (его фикстура пишет `schema_version = 6`), константа
    `CURRENT_SCHEMA_VERSION: u32 = 7` в `crates/vibe-core/src/manifest/lockfile.rs`
    тоже не изменена — противоречие целиком внутри закоммиченного кода,
    воспроизводится на HEAD. В тесте нет ни одного упоминания `Block` или
    `specdoc`.

$ cargo clippy -p vibe-specdoc -p vibe-facts -p vibe-workspace --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 48.99s

$ cargo xtask specmap
  drift: edges added: 1
specmap: wrote C:\Users\olegc\git\v\vibevm-docs\specmap.json (7930 spec units, 3423 tagged code items, 2969 edges, 0 suspects, 31 warnings).
specmap: ratchet gate — 0 gated orphan(s), 0 dispositioned (6 crate(s) exempt).
specmap: resolve gate — 0 unresolved host edge(s), 35 non-host edge(s) outside this map's jurisdiction.

$ target/debug/vibe.exe facts check --exhaustive
progress check: clean (325 files, 22 warning(s))
  — байт в байт как базовая линия, снятая до атома.
```

## Существующие документы и проекции

Не изменились. Свидетельства: голден `tests/golden/redbook-readme.xml`
сходится побайтово (`redbook_readme_golden_xml_is_pinned`); `facts check
--exhaustive` даёт тот же вердикт, то же число файлов и то же число
предупреждений, что и базовая линия; `git status` не показывает ни одного
изменённого `.md`/`.xml` в `vibevm/`. По коду: обе новые ветки `md_out`
(`(cond)` в заголовке секции и подзаголовок слота) недостижимы, пока
`when == None`, а под словарём `Spec` он `None` всегда.

## Дефекты страниц, найденные round-trip'ом (не чинил — пакет запрещает)

**Три страницы пивот не читает вовсе** (карантин в `docs_corpus.rs`,
константа `QUARANTINED`; тест `quarantined_pages_still_carry_exactly_the_recorded_defect`
упадёт в тот день, когда дефект починят, и потребует убрать запись):

1. `agent/how-agents-read-this-manual.xml:4` — `audience="agent"`, а словарь
   аудиторий — `user|author|dev` (PROP-043 §3.6, дом — `progress-core`).
   Расширять словарь ради одной страницы — решение, а не починка.
2. `glossary/index.xml:61` — статья глоссария про слово «fact» написана как
   `<fact title="fact">`, а `fact` — зарезервированное структурное имя
   (не elementable), то есть не секция ни под каким словарём. Рабочее
   написание — `<section id="fact" title="fact">`. Дефект **предшествует**
   жанру.
3. `reference/machine-formats.xml:43` — `<derived/>` стоит **внутри `<td>`**,
   причём дважды в одной ячейке. Члены жанра — блоки, а `td` держит ровно
   один юнит; ни позиция, ни повторение не выразимы. Нужна либо другая
   вёрстка таблицы, либо решение владельца о `derived` на позиции юнита.

**Пятнадцать страниц читаются, но записаны не в канонической форме**
(константа `NOT_CANONICAL`, тест `non_canonical_pages_are_exactly_the_recorded_ones`):
13 страниц пишут строку таблицы одной строкой `<tr><td>…</td><td>…</td></tr>`
(писатель даёт каждой ячейке свою строку);
`howto/read-documentation-locally.xml` держит литеральный `'` в атрибуте
`title` (писатель пишет `&apos;`); `lifecycle/build-package-deploy.xml:17`
имеет `<p>` с отступом глубже своей глубины. Это дефекты написания, не
смысла: `vibe refactor convert-source` нормализует их. Список прибит, чтобы
канонизация одной страницы была замечена.

Из-за карантина корпусные счётчики считают 41 страницу: 58 `example`,
319 `rule`, 54 `derived`, 19 `prompt`, 40 `assert`, 1 секция под `when`.
В корпусе **нет** `note`, `figure`, `example ref`, `stderr` и `when` на
блоке — они покрыты юнит-тестами (`xml_doc_tests.rs`), и корпусный тест
прибивает нули, чтобы появление первой такой страницы было замечено.

## Формулировка закона байтовой идемпотентности

Корпусный тест утверждает то же, что и `redbook_xml_to_ir_to_xml_is_byte_idempotent`:
**вывод писателя — неподвижная точка** (`IR → XML → IR` сохраняет IR, второй
`XML` совпадает с первым байт в байт). Сравнение с исходными байтами
страницы — другое (более сильное и неверное) утверждение: 15 страниц
написаны не канонически, а править их пакет запрещает. Поэтому
неканоничность вынесена в отдельный посчитанный тест, а не в падающий закон.

## Что не сделано и почему

1. **`specmap.json` не закоммичен.** Гейт выполнен (`cargo xtask specmap`,
   **0 suspects**), но регенерация в общем дереве неизбежно втягивает
   незакоммиченные элементы двух параллельных воркеров: сверка
   сгенерированного файла с HEAD показала изменения записей для
   `crates/vibe-core/src/error.rs`, `crates/vibe-core/src/package_ref/tests.rs`,
   `crates/vibe-install/src/error.rs`, `crates/vibe-install/src/plan/fetch.rs`
   — чужие пути. Файл возвращён к HEAD (`git checkout -- specmap.json`),
   чтобы не оставлять в общем артефакте чужое состояние. **Интегратору:**
   после того как все три пакета фазы приземлятся, один прогон
   `cargo xtask specmap` и один коммит `specmap.json` (панель гейтит его
   через `cargo xtask specmap --check`, шаг 6c `tools/self-check.sh`).
2. **Страницы руководства не правились** — по запрету пакета; три дефекта и
   пятнадцать неканоничных написаний описаны выше и прибиты тестами.
3. **Три красных теста `vibe-workspace`** — предсуществующие, к атому
   отношения не имеют (разбор выше в гейтах).
4. **`vibe doc check --examples` / `--prompts`, раннер, генераторы
   `derived`** — это A2.9 и A2.10, другие атомы; пивот их не открывает.
5. **`md_in` не расширялся** и расширен быть не может
   (`DOC-VOCAB-MD-ONE-WAY`): роды блоков Markdown закрыты в
   `progress-core::doc::BlockKind`. Односторонность проекции прибита тестом
   `doc_genre_is_not_round_trippable_through_markdown` — и на одной странице
   (юнит), и на всём корпусе (`Conversion::IrDivergent` для всех 41).

## `git status --short` на момент сдачи

```
 M crates/vibe-cli/resources/package-tree.schema.v1.json
 M crates/vibe-cli/src/cli.rs
 M crates/vibe-cli/src/cli/pkg.rs
 M crates/vibe-cli/src/commands/init/prompts.rs
 M crates/vibe-cli/src/commands/show/source_path.rs
 M crates/vibe-core/src/error.rs
 M crates/vibe-index/src/cli/list.rs
 M crates/vibe-index/src/cli/search.rs
 M crates/vibe-index/src/scanner/manifest.rs
 M crates/vibe-install/src/plan/fetch.rs
 M crates/vibe-mcp/src/skill_template.md
 M crates/vibe-wire/src/behaviour/vocabularies.rs
 M crates/vibe-wire/src/generated/index_cli/e1/list_report/mod.rs
 M crates/vibe-wire/src/generated/shared/mod.rs
 M crates/vibe-wire/tests/open_vocabulary.rs
 M formats/vocabularies.json
 M schemas/index_cli/e1/list_report.jtd.json
 M xtask/src/batch_review/refs.rs
 M xtask/src/bridge.rs
 M xtask/src/codegen/vocabulary/tests.rs
?? campaigns/docs-2026-09/findings/WORKER-REPORT-P2-O2.md

Все изменённые пути — двух параллельных воркеров (vibe-cli, vibe-core/error.rs,
vibe-index, vibe-install, vibe-mcp, vibe-wire, formats/, schemas/, xtask).
Моих несохранённых изменений нет; `specmap.json` возвращён к HEAD сознательно
(см. «Что не сделано»). Отчёт коммитится следующим шагом.
```
