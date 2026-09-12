# WORKER-REPORT-P2-O11 — десять файлов за бюджетом, разложенных по ячейкам

Пакет: `campaigns/docs-2026-09/findings/PACKET-P2-O11.md`.
Ветка `research-preview-1-docs`, без push. Дата: 2026-09-12.

## Коротко для оркестратора

Все десять файлов разложены, пять коммитов легли, каждый явной формой
`git commit -m … -m … -- <свои пути>`. Незакоммиченных правок у меня в
дереве нет. Оба приватных каталога сборки удалены.

**Главное число.** `cargo xtask conform check` даёт **20 новых находок
вместо прежних 27**. Из них **ровно 17 — унаследованные с `main`**, то
есть точно список «не трогать» из пакета, файл в файл и правило в
правило. **Ни одной находки ни в одном из десяти файлов и ни в одной из
двадцати пяти новых ячеек**: все десять находок `file-length`, которые
кампания сама себе вырастила, погашены. Оставшиеся **три** новых находки
— чужие: их принёс параллельно работающий P4-O4
(`crates/vibe-doc-server/src/lib.rs`, `crates/vibe-doc-shell/src/lib.rs`
дважды), и они появились после того, как он добавил свой новый крейт в
`gated` список `conform.toml`.

**Число тестов не изменилось ни на одном крейте** — пары ниже сняты
честно: сначала на предсплитовой ревизии, потом на своей, тем же
набором команд и в том же каталоге сборки.

Три вещи, которые стоит прочитать: **`golden_corpus` в `vibe-index`
краснеет, пока в дереве идёт чужая перегенерация кодогена** — он читает
генерируемые wire-типы прямо из рабочего дерева, так что мерить им
что-либо во время параллельной работы нельзя (аномалия 1, с
доказательством, что к разрезу это отношения не имеет);
**`cargo xtask check-codegen` красен целиком из-за P4-O4** (аномалия 2);
и **полный `cargo test -p vibe-cli` в этой среде не проходит за разумное
время** — сетевые install-тесты, поэтому пара по `vibe-cli` снята на
`--bins` (аномалия 3).

## Хэши и subject'ы

| Крейт | Коммит | Subject |
|---|---|---|
| progress-core | `c349d2e7` | `refactor(progress): split the model and scope files into cells` |
| vibe-cli | `f8ee39ac` | `refactor(cli): split the progress tests by topic` |
| vibe-index | `140ca684` | `refactor(index): split the memory tests and the manifest scanner into cells` |
| vibe-specdoc | `e34fa5ae` | `refactor(specdoc): split the documentation genre files into cells` |
| xtask | `32c4a55c` | `refactor(xtask): split the optional-shape tests by topic` |

Тело каждого коммита отвечает на «почему»: бюджет файла дисциплины,
шов, по которому резали, сохранённые публичные пути, неизменность
поведения и равенство числа тестов. Трейлеров нет, упоминаний моделей и
инструментов нет.

## Что куда уехало

Ни одна строка кода не переписана: куски перенесены побайтно, к ним
добавлены только шапка модуля, нужные `use` и, где разрез того требовал,
`pub(crate)` / `pub(super)`. Наружу крейта не вышло ни одного нового
имени. `#[spec(…)]`-теги уехали вместе со своими элементами, а
`specmark::scope!` в новых ячейках кода повторяет якорь родителя (в
тестовых ячейках его нет — так же, как у уже существующих `cache/tests`
и `xml_in_tests`).

Конвенция взята у того крейта, который правился, а не заведена третья:
в `progress-core` и `vibe-index` это `foo.rs` + каталог `foo/`, в
`vibe-specdoc` — плоские соседи `<module>_tests.rs` рядом в `src/`, в
`xtask` — подмодули через `#[path = "tests/<тема>.rs"]`, как уже сделано
для `stitch` и `union`.

### progress-core

`crates/progress-core/src/model.rs` — **601 → 321**

| Ячейка | Строк | Что уехало |
|---|---|---|
| `model/artifacts.rs` | 160 | `ArtifactKind` с его `impl` и `Display`, `ArtifactRequirements` с `impl`, ручными `Serialize`/`Deserialize` — и тест на serde-круг |
| `model/rollup_order.rs` | 128 | `VOID_KEY`, `rollup_key` — и три теста о порядке: порядок §3.10, `void` выше любой пары при любой стадии, «худшее из» на трёх примерах DRIFT-028 |
| `model/typo_hints.rs` | 51 | `nearest`, `levenshtein` — и тест про знаменитую опечатку |

В `model.rs` остались сами словари (`Stage`, `State`, `Action`,
`Audience`, `MarkerForm`, `Granularity`, `Marker`), их `parse`/`as_str`/
`ALL` и `Display`, и три теста о словарях. Публичные пути сохранены
реэкспортом: `model::ArtifactKind`, `model::ArtifactRequirements`,
`model::rollup_key`, `model::nearest` резолвятся как раньше.

`crates/progress-core/src/scope.rs` — **612 → 189**

| Ячейка | Строк | Что уехало |
|---|---|---|
| `scope/enumerate.rs` | 163 | `ExcludeReport`, `observed_files`, `observed_files_reported`, `is_excluded`, `is_excluded_file`, `apply_config_excludes`, `rel_str` — и два теста о предикатах по умолчанию (они трогают приватные функции, поэтому уехали к ним, а не в общий тестовый модуль) |
| `scope/judging.rs` | 54 | `JudgingExemption` с `impl` |
| `scope/tests.rs` | 258 | остальные тесты модуля |

В `scope.rs` остались четыре таблицы по умолчанию, `ProgressSection`,
`JudgingSection`, `ScopeConfig`, `load_config` и реэкспорты. Разрез —
ровно тот, который шапка модуля уже описывала словами: перечисление
корпуса и «судимость» — разные оси, и вторая никогда не трогает первую.

### vibe-cli

`crates/vibe-cli/src/commands/progress/tests.rs` — **602 → 310**

| Ячейка | Строк | Что уехало |
|---|---|---|
| `tests/incremental.rs` | 313 | шесть тестов о том, что кэш не меняет **ответ** прогона: тёплый против холодного, на каждом рендеринге, после правки файла, под `--no-cache`, и вердикт, который обязан остаться в отслеживаемом `cache.json` — плюс `report_args`, который больше никому не нужен |

Фикстуры, `args`, `blank_stamps` и `read_state` остались в родителе:
через них сканируют то же дерево три другие ячейки (`writes`,
`baseline`, `xml_sources`).

### vibe-index

`crates/vibe-index/src/index/memory/tests.rs` — **705 → 375**

| Ячейка | Строк | Что уехало |
|---|---|---|
| `tests/idempotence.rs` | 188 | группа B-072: `later`, `snapshot`, четыре теста о нулевом изменении байтов и свежей метке, `assert_trees_byte_identical`, `walk` |
| `tests/catalog.rs` | 169 | группа PROP-057: `doc_entry` и два теста о карточке документации и о свёртке, отвечающей на обратные вопросы |

`crates/vibe-index/src/scanner/manifest.rs` — **680 → 87**

| Ячейка | Строк | Что уехало |
|---|---|---|
| `manifest/relations.rs` | 91 | `compatibility_from`, `provides_from`, `requires_from`, `requires_any_from`, `obsoletes_from`, `conflicts_from`, `features_from` |
| `manifest/documentation.rs` | 82 | `i18n_from`, `documents_from`, `documentation_from`, `translates_from`, `media_from` и `wire_path` (его единственный потребитель — эта ячейка) |
| `manifest/delivery.rs` | 78 | `boot_snippet_from`, `embedded_sources_from`, `workspace_origin_from`, `boot_category_str`, `delivery_from` |
| `manifest/subskills.rs` | 85 | `collect_subskills`, `declared_channels` |
| `manifest/tests.rs` | 326 | тесты модуля |

В `manifest.rs` остались `parse_manifest`, `require_package`,
`package_kind`, объявления подмодулей и реэкспорты — так что
`scanner::manifest::requires_from` и вся его родня резолвятся у обоих
внешних потребителей (`cli/add.rs`, `scanner/org_walk.rs`) без правок.
Единственное расширение видимости внутри семьи: `delivery_from` стал
`pub(super)`, потому что его зовёт `subskills`.

### vibe-specdoc

`crates/vibe-specdoc/src/xml_doc.rs` — **625 → 134**

| Ячейка | Строк | Что уехало |
|---|---|---|
| `xml_doc_verbatim.rs` | 303 | `example` и `prompt` — два читателя, чьи тела VERBATIM; именно это свойство их и объединяет |
| `xml_doc_citations.rs` | 140 | `rule` и `derived` — два, которые показывают наружу документа, и `split_revision`: пин — часть адреса |
| `xml_doc_callouts.rs` | 110 | `note` и `figure` — два выноса, чьи дети PROSE |

В `xml_doc.rs` остались диспетчер `doc_block` и три общих помощника
(`attr`, `verbatim_text`, `expect_end`).

`crates/vibe-specdoc/src/xml_doc_tests.rs` — **737 → 286**

| Ячейка | Строк | Что уехало |
|---|---|---|
| `xml_doc_element_tests.rs` | 385 | законы шести элементов, по элементу |
| `xml_doc_markdown_tests.rs` | 94 | односторонняя проекция в Markdown |

Родитель остался хозяином общих фикстур (`doc`, `doc_err`,
`only_block`, `assert_byte_stable`, `NS` — они стали `pub(crate)`),
закрытости словаря, `when` и CDATA.

`crates/vibe-specdoc/src/xml_in.rs` — **662 → 299**

| Ячейка | Строк | Что уехало |
|---|---|---|
| `xml_in_descent.rs` | 343 | `check_root_attrs`, `spec_children`, `section`, `block_node`, `validate_fence_bindings` |
| `xml_in_ids.rs` | 59 | `mint_heading`, `mint_fact`, `check_ids` — чеканка и проверка это один инвариант, поэтому одна ячейка |

Поля `vocab` и `ids` структуры `Parser` стали `pub(super)` — ровно то,
чего потребовал разрез; `evs`, `poss`, `i` такими были и раньше.

`crates/vibe-specdoc/src/xml_out.rs` — **654 → 424**

| Ячейка | Строк | Что уехало |
|---|---|---|
| `xml_out_emit.rs` | 100 | `bytes_start`, `esc_text`, `esc_attr`, `start`, `end`, `empty`, `inline`, `indent` — примитивы, на которых стоит закон байтовой идемпотентности |
| `xml_out_tests.rs` | 151 | тесты писателя |

### xtask

`xtask/src/codegen/optional_shapes/tests.rs` — **605 → 410**

| Ячейка | Строк | Что уехало |
|---|---|---|
| `tests/required_nullable.rs` | 134 | строка required-nullable: всё, что держит `null` и «отсутствует» разными ответами |
| `tests/refusals.rs` | 94 | отказы со стороны схемы: политика не выводится из Rust |

Образцы эмиссии и два помощника остались в родителе, чтобы все ячейки
цитировали одну закреплённую форму вывода jtd-codegen.

## Числа тестов парами

Пары сняты так: файлы этой задачи возвращены на предсплитовую ревизию
(`c349d2e7~1`), прогон; потом возвращены на `HEAD`, тот же прогон, тот
же приватный каталог сборки. Ничего другого в это время не собиралось.

По progress-core и vibe-index обе стороны сняты одной командой
`cargo test -p progress-core -p vibe-index`, и вывод совпал **цель в
цель**, все 29 целей зелёные (сравнение сделано `diff`-ом двух сводок, с
отброшенным временем прогона):

| Крейт / цель | До | После |
|---|---|---|
| progress-core `unittests src/lib.rs` | 195 passed | 195 passed |
| progress-core `tests/parser_law.rs` | 14 passed | 14 passed |
| progress-core Doc-tests | 10 passed | 10 passed |
| vibe-index `unittests src/lib.rs` | 215 passed | 215 passed |
| vibe-index `unittests src/main.rs` | 0 passed | 0 passed |
| vibe-index `tests/auto_publish.rs` | 5 passed | 5 passed |
| vibe-index `tests/cli_lifecycle.rs` | 12 passed | 12 passed |
| vibe-index `tests/cli_read.rs` | 16 passed | 16 passed |
| vibe-index `tests/cli_write.rs` | 10 passed | 10 passed |
| vibe-index `tests/config_cmd.rs` | 9 passed | 9 passed |
| vibe-index `tests/content_hash_parity.rs` | 7 passed, 1 ignored | 7 passed, 1 ignored |
| vibe-index `tests/from_github_e2e.rs` | 4 passed | 4 passed |
| vibe-index `tests/golden_corpus.rs` | 1 passed | 1 passed |
| vibe-index `tests/help_smoke.rs` | 8 passed | 8 passed |
| vibe-index `tests/org_cache_e2e.rs` | 7 passed | 7 passed |
| vibe-index `tests/rate_limit_e2e.rs` | 7 passed | 7 passed |
| vibe-index `tests/rebuild_cli.rs` | 3 passed | 3 passed |
| vibe-index `tests/round_trip_published.rs` | 5 passed | 5 passed |
| vibe-index `tests/scanner_e2e.rs` | 12 passed | 12 passed |
| vibe-index `tests/seam_fakes.rs` | 3 passed | 3 passed |
| vibe-index `tests/server_e2e.rs` | 26 passed | 26 passed |
| vibe-index `tests/server_writes.rs` | 16 passed | 16 passed |
| vibe-index `tests/wire_parity_by_name.rs` | 1 passed | 1 passed |
| vibe-index `tests/wire_parity_entry.rs` | 1 passed | 1 passed |
| vibe-index `tests/wire_parity_hello.rs` | 2 passed | 2 passed |
| vibe-index `tests/wire_parity_index_cli.rs` | 3 passed | 3 passed |
| vibe-index `tests/wire_parity_index_http.rs` | 5 passed | 5 passed |
| vibe-index `tests/wire_parity_inverted.rs` | 2 passed | 2 passed |
| vibe-index `tests/wire_parity_journal.rs` | 3 passed | 3 passed |
| vibe-index `tests/wire_parity_repomd.rs` | 2 passed | 2 passed |
| vibe-index Doc-tests | 5 passed | 5 passed |
| vibe-specdoc `unittests src/lib.rs` | 114 passed | 114 passed |
| vibe-specdoc `tests/docs_corpus.rs` | 6 passed | 6 passed |
| vibe-specdoc `tests/redbook_roundtrip.rs` | 7 passed | 7 passed |
| vibe-specdoc Doc-tests | 5 passed | 5 passed |
| xtask `unittests src/main.rs` | 293 passed | 293 passed |
| vibe-cli `--bins` | 744 passed | 744 passed |

Вторая, независимая проверка того же равенства — структурная, потому
что одна пара чисел могла бы совпасть и при перепутанных тестах. По
каждому крейту посчитано число атрибутов `#[test]` в наборе тронутых
файлов на `HEAD` предсплитовой ревизии и сейчас:

```
progress-core  HEAD=  23  now=  23  equal
vibe-cli       HEAD=   9  now=   9  equal
vibe-index     HEAD=  28  now=  28  equal
vibe-specdoc   HEAD=  35  now=  35  equal
xtask          HEAD=  16  now=  16  equal
```

И третья: множество объявленных имён (`fn` / `struct` / `enum` /
`const`) до и после по каждой группе разреза — чистый перенос обязан
оставить его тем же.

```
model.rs         HEAD=  48 now=  48  identical
scope.rs         HEAD=  37 now=  37  identical
cli tests        HEAD=  21 now=  21  identical
memory tests     HEAD=  28 now=  28  identical
manifest.rs      HEAD=  33 now=  33  identical
xml_doc.rs       HEAD=  11 now=  11  identical
xml_doc_tests    HEAD=  34 now=  34  identical
xml_in.rs        HEAD=  20 now=  20  identical
xml_out.rs       HEAD=  28 now=  28  identical
xtask tests      HEAD=  53 now=  53  identical
```

## Гейты

Все прогоны — в приватном каталоге сборки, заданном через
`CARGO_TARGET_DIR` (путь вне репозитория, поэтому здесь не приводится).
В выводе ниже абсолютные пути машины сокращены до путей от корня
репозитория; всё остальное дословно.

### Форматирование

`rustfmt --edition 2024` по тридцати шести файлам этой задачи — выход 0,
правки применены и вошли в коммиты.

`cargo fmt --all --check` целиком по рабочему пространству не
запускался как приёмка: в дереве живут незакоммиченные файлы P4-O4, и
любой его вывод говорил бы и о них тоже. Результат по своим файлам
проверен отдельной командой выше.

### Сборка и линт

`cargo check -p progress-core -p vibe-specdoc -p vibe-index -p xtask
--all-targets`:

```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 32.85s
```

Ноль ошибок и ноль предупреждений о неиспользуемых импортах.

`cargo clippy -p progress-core -p vibe-specdoc -p vibe-index -p xtask
--all-targets -- -D warnings`:

```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 21s
```

`cargo clippy -p vibe-cli --all-targets -- -D warnings`:

```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 37s
```

Единственное предупреждение в обоих clippy-прогонах —
`warning: crates/vibe-cli/Cargo.toml: unused manifest key: build`, и оно
про манифест P4-O4, а не про линт и не про мой периметр.

### conform

`cargo xtask conform check` — итоговые строки:

```
conform check: 78 finding(s) in scope <workspace> ({"ambient-env": 5,
"error-enum-cites-req": 1, "error-message-cites-req": 2, "file-length": 7,
"no-unwrap-in-domain": 43, "pub-doctest": 5, "seam-has-doctest": 3,
"unsafe-gate": 12}), 0 frozen in baseline, 20 new; SARIF at
target/conform/report.sarif.
conform: 28 crate(s) gated, 7 exempt — see conform.toml for the why of each.
Error: conform: 20 new finding(s) against the baseline
```

Полный список двадцати новых находок, дословно (в порядке вывода):

```
NEW error-enum-cites-req crates/vibe-registry/src/embedded_source.rs:42 — thiserror enum `EmbeddedSourceError` carries no #[spec] REQ edge
NEW error-message-cites-req crates/vibe-cli/src/commands/vvm/error.rs:94 — `VvmError::DoctorProblems` display text cites no spec:// REQ
NEW error-message-cites-req crates/vibe-cli/src/commands/vvm/error.rs:100 — `VvmError::CorruptInstance` display text cites no spec:// REQ
NEW file-length crates/vibe-agent-projection/src/pkgskill.rs:1 — 958 lines exceeds the 600-line file budget
NEW file-length crates/vibe-check/src/checks/bridge_provenance.rs:1 — 625 lines exceeds the 600-line file budget
NEW file-length crates/vibe-core/src/manifest/document/tests.rs:1 — 754 lines exceeds the 600-line file budget
NEW file-length crates/vibe-core/src/manifest/package.rs:1 — 639 lines exceeds the 600-line file budget
NEW file-length crates/vibe-orchestrator/src/dispatch/native_all_owner_tests/support.rs:1 — 606 lines exceeds the 600-line file budget
NEW file-length crates/vibe-orchestrator/src/world/mod.rs:1 — 604 lines exceeds the 600-line file budget
NEW file-length crates/vibe-registry/src/embedded_source.rs:1 — 611 lines exceeds the 600-line file budget
NEW no-unwrap-in-domain crates/vibe-cli/src/commands/vvm/placer.rs:297 — `.expect()` in domain logic
NEW no-unwrap-in-domain crates/vibe-cli/src/commands/vvm/placer.rs:344 — `.expect()` in domain logic
NEW pub-doctest crates/vibe-core/src/manifest/lockfile.rs:234 — public struct `LockedEmbeddedSource` has no compiled doctest
NEW pub-doctest crates/vibe-core/src/manifest/package/embedded_source.rs:15 — public enum `EmbeddedSourceKind` has no compiled doctest
NEW pub-doctest crates/vibe-core/src/manifest/package/embedded_source.rs:24 — public enum `EmbeddedSourceAuth` has no compiled doctest
NEW pub-doctest crates/vibe-core/src/manifest/package/embedded_source.rs:32 — public struct `EmbeddedSourceDecl` has no compiled doctest
NEW pub-doctest crates/vibe-core/src/manifest/package/skill.rs:100 — public struct `SkillResourceDecl` has no compiled doctest
NEW seam-has-doctest crates/vibe-doc-server/src/lib.rs:131 — public seam fn `content_policy` has no compiled doctest
NEW seam-has-doctest crates/vibe-doc-shell/src/lib.rs:89 — public seam enum `Provenance` has no compiled doctest
NEW seam-has-doctest crates/vibe-doc-shell/src/lib.rs:339 — public seam struct `Report` has no compiled doctest
```

(В каждой строке опущен неизменный хвост `violates REQ <uri>: …; fix
surface: …` — грамматика Class-F одна и та же и занимала бы страницу.)

Разбор: **первые семнадцать — ровно список «не трогать» из пакета**,
двенадцать файлов, все с `main`. **Последние три — чужие**: это
`vibe-doc-server` и новый крейт `vibe-doc-shell`, который P4-O4 в эту же
минуту добавил в `gated` список `conform.toml`. В моих десяти файлах и в
двадцати пяти новых ячейках находок нет ни одной; было десять
`file-length`, стало ноль.

### specmap

`cargo xtask specmap`:

```
specmap: wrote specmap.json (7930 spec units, 3574 tagged code items, 3092 edges, 0 suspects, 31 warnings).
specmap: ratchet gate — 0 gated orphan(s), 0 dispositioned (6 crate(s) exempt).
```

Ноль подозрительных, ноль сирот. Файл `specmap.json` возвращён к `HEAD`
(`git checkout HEAD -- specmap.json`), как велит пакет: карту
перегенерирует интегратор одним прогоном на волну.

### check-codegen

`cargo xtask check-codegen` — **красный, и целиком не мой** (аномалия 2):

```
Error: generated code under crates/vibe-wire/src/generated /
vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v1.0.0/crates/core-ai-native-specmap/src/generated /
vibevm/vibepacks/org.vibevm.doc/web/v0.1.0/site/src/generated/doc-manifest.ts
differs from what this machine's jtd-codegen emits for the current schemas
and `formats/REGISTRY.toml`.
```

Расходятся ровно два файла, и оба порождаются из схемы, которую правит
P4-O4:

```
diff --git a/crates/vibe-wire/src/generated/doc_manifest/mod.rs b/crates/vibe-wire/src/generated/doc_manifest/mod.rs
diff --git a/vibevm/vibepacks/org.vibevm.doc/web/v0.1.0/site/src/generated/doc-manifest.ts b/vibevm/vibepacks/org.vibevm.doc/web/v0.1.0/site/src/generated/doc-manifest.ts
```

Схем и генерируемых деревьев я не трогал вовсе; это был контроль, и он
показал чужой дрейф, а не мой.

## Аномалии

**1. `vibe-index` / `tests/golden_corpus.rs` дважды падал посреди работы
— и оба раза не из-за разреза.** Тест говорит, что каталог больше не
воспроизводится из своего журнала: `primary.jsonl.gz` — 1879 байт в
дереве против 1891 спроецированных, и `repomd.json` расходится на
строке 90 ровно этим числом. Что это не моё, показывают четыре
измерения:

* падение воспроизвелось **побайтово одинаково** (те же 1879/1891) и на
  моей ревизии, и на предсплитовой — то есть свойство не разреза;
* в одиночку (`cargo test -p vibe-index --test golden_corpus`) тест
  **проходит** на обеих ревизиях;
* в спокойных прогонах, снятых для таблицы выше, весь набор
  `vibe-index` **зелёный на обеих сторонах**, `golden_corpus`
  включительно;
* код, который он исполняет, моего разреза не касается вовсе: он держит
  `index::memory::WriteCtx`, `index::{WRITER_DIRS, WRITER_FILES}` и
  `journal`, а из `vibe-index` я правил только тестовый модуль `memory`
  (он под `#[cfg(test)]` и в сборку интеграционного теста не входит) и
  `scanner/manifest`, который этот тест не зовёт.

Оба падения пришлись на минуты, когда в том же дереве шли сборки
параллельной задачи. Отсюда вывод, который стоит записать волне: этот
тест читает генерируемые wire-типы **из рабочего дерева**, поэтому
чужая перегенерация кодогена в соседнем периметре делает его красным,
пока идёт. Мерить им что-либо во время параллельной работы нельзя.

**2. `cargo xtask check-codegen` красный из-за P4-O4.** Разбор выше:
дрейфуют `crates/vibe-wire/src/generated/doc_manifest/mod.rs` и
`…/site/src/generated/doc-manifest.ts`, оба — производные от
`schemas/doc_manifest.jtd.json`, который в периметре P4-O4. Я не трогал
ни одной схемы и ни одного генерируемого файла.

**3. Полный `cargo test -p vibe-cli` в этой среде не заканчивается за
разумное время.** Интеграционный тест
`cli_spec_format.rs::redbook_polygon_follows_each_static_target_and_switches_without_an_orphan`
запускает `vibe --json install` несколько раз подряд; каждый такой
запуск шёл здесь по 10–15 минут при почти нулевом CPU, то есть упирается
в сеть. Первый прогон я снял с него через час, не дождавшись. Поэтому
пара по `vibe-cli` снята на `cargo test -p vibe-cli --bins` — это
ровно тот таргет, в котором живёт мой разрез (модуль
`commands::progress::tests`), и обе стороны дали 744. Интеграционный
набор `vibe-cli` в этой задаче не измерен; это условие среды, а не
дефект пакета или кода.

**4. Первые снимки по `vibe-index` я снял при параллельных сборках** —
своих и чужих, — и именно они принесли красный `golden_corpus`. Это моя
ошибка в организации работы, а не находка. Все числа в таблице выше
сняты заново, по одному прогону за раз, и обе стороны совпали цель в
цель.

## Что не сделано и почему

* **`cargo fmt --all --check` как приёмка** — не запускался целиком:
  вывод говорил бы о незакоммиченных файлах P4-O4. Свои тридцать шесть
  файлов отформатированы `rustfmt --edition 2024` с выходом 0.
* **`cargo build` отдельной командой** — не запускался: его покрывают
  `cargo check --all-targets` и `cargo clippy --all-targets` по тем же
  пяти пакетам, а полные `cargo test` собирают те же цели с кодогенерацией.
* **`cargo test -p vibe-cli` целиком** — см. аномалию 3.
* **`conform freeze`** — не делался: это решение владельца, не моё.
* **`conform.toml`, `conform-baseline.json`, PROP-файлы, страницы
  руководства, web-пакет** — не тронуты. `conform.toml` в дереве
  изменён, но не мной: P4-O4 добавил туда `vibe-doc-shell`.
* **Периметр P4-O4** — не правился и не стейджился ни разу; после моих
  пяти коммитов его файлы остались ровно такими же незакоммиченными,
  какими были.

## Диск

`df -h .` до работы:

```
Filesystem      Size  Used Avail Use% Mounted on
C:              3.7T  3.5T  229G  94% /c
```

`df -h .` после удаления обоих приватных каталогов сборки:

```
Filesystem      Size  Used Avail Use% Mounted on
C:              3.7T  3.5T  194G  95% /c
```

Непосредственно перед удалением было 135G, после — 194G: два каталога
сборки занимали около 59 гигабайт. Каталогов было два, потому что пока
базовый прогон стоял в сетевом тесте, я вёл во втором проверку
компиляции; оба удалены.

Разница с исходными 229G не моя: её заняли сборки параллельной задачи,
живущие в том же дереве.
