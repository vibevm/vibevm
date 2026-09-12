# WORKER-REPORT-P1-O5 — фикстуры lock-файлов берут номер схемы из константы

Пакет: `campaigns/docs-2026-09/findings/PACKET-P1-O5.md`.
HEAD на входе: `dccaeac68e719ea1f0f89017c15a22260e1455cc`.
HEAD перед коммитом: `7c829c0d832d716cf13e6b7442184bd301946caf` (дерево общее
с воркером P2-O1, HEAD сдвинулся под работой четыре раза — §5).

**Итог одной строкой.** Все десять живых фикстур переведены на
`CURRENT_SCHEMA_VERSION`; одиннадцатая — намеренный негативный тест старой
схемы — оставлена литералом по указанию пакета. Каскад, который назвал
P1-O4 §4.1, закрыт целиком: ни одна из названных им целей больше не
краснеет. В воркспейсе остаётся четыре красных цели, ни одна не имеет
отношения к схеме лока (§3).

---

## 1. Точка за точкой — все одиннадцать

Список точек взят из `WORKER-REPORT-P1-O4.md` §4.1 и перепроверен заново:
в дереве на входе было ровно эти одиннадцать `schema_version = 6` в `.rs`.

Идиома выбрана не новая: в дереве уже жили три фикстуры, берущие номер из
константы — `crates/vibe-cli/tests/cli_list_bridge_authors.rs:15`,
`crates/vibe-core/src/visibility/query/tests.rs:16` и
`crates/vibe-check/src/checks/visibility_hygiene.rs:134`. Правки ниже
повторяют её, а не изобретают вторую.

| # | Точка | Решение |
|---|-------|---------|
| 1 | `crates/vibe-cli/tests/cli_facts.rs:129` | строковый литерал → `format!` с `{CURRENT_SCHEMA_VERSION}` |
| 2 | `crates/vibe-cli/tests/cli_mcp_path_parity.rs:16` | `const WINDOWS_STYLE_LOCKFILE` → `fn windows_style_lockfile() -> String` |
| 3 | `crates/vibe-cli/tests/cli_redirect.rs:213` | raw-строка внутри `fs::write` → `format!` |
| 4 | `crates/vibe-core/src/manifest/lockfile/tests.rs:249` | **литерал оставлен** — см. §2 |
| 5 | `crates/vibe-install/tests/incremental_in_place.rs:136` | `&format!` (хелпер `write` берёт `&str`) |
| 6 | `crates/vibe-mcp/tests/tools_oracle.rs:24` | `const LOCKFILE_FIXTURE: &str` → `static LOCKFILE_FIXTURE: LazyLock<String>` |
| 7 | `crates/vibe-requirements/src/tests_followup.rs:421` | именованная подстановка в существующий `format!` |
| 8 | `crates/vibe-requirements/src/tests_provider.rs:43` | то же |
| 9 | `crates/vibe-requirements/src/tests_query.rs:63` | то же |
| 10 | `crates/vibe-workspace/src/bins/tests.rs:8` | `const LOCK` → `fn lock_fixture() -> String` |
| 11 | `crates/vibe-workspace/src/freshness.rs:307` | именованная подстановка в существующий `format!` |

Пояснения там, где решение не сводится к одной подстановке.

**Точка 2 и точка 10 — константы с единственным потребителем.** `&str`-конста
не умеет звать `format!`, поэтому обе стали функциями. У каждой ровно один
вызов, так что цена — один `()` на месте использования.

**Точка 6 — константа с семнадцатью потребителями.** `LOCKFILE_FIXTURE` читают
пятнадцать тестов в `tools_oracle.rs` и два в вынесённом
`tools_oracle/dispatch_compat.rs` (`use super::{LOCKFILE_FIXTURE, …}`).
Переписывать её в функцию значило бы тронуть семнадцать мест и сломать
импорт соседнего файла, поэтому взят `LazyLock<String>`: имя сохранено,
импорт цел, строка считается один раз, а на местах использования добавлен
единственный символ `&` (`project_with_locked(&LOCKFILE_FIXTURE)` —
разыменование `LazyLock<String> → String → str` цепочкой). `LazyLock` в
дереве не новичок: `crates/vibe-spec/src/compiler/pass_tier/frontend_pipeline_tests.rs`
уже пользуется им тем же способом.

**Точки 5, 7, 11 несли комментарий, называвший схему по номеру** («schema
v6», «Schema v6 carries qualified naming…»). Комментарий, который врёт
после следующего подъёма, — тот же дефект, что и литерал, поэтому в этих
трёх местах он переписан на «та схема, которую читатель принимает
сегодня».

`Cargo.toml` не тронут ни у одного пакета: `vibe-core` уже был
зависимостью всех пяти (`vibe-cli` держит его и в `[dependencies]`, и в
`[dev-dependencies]`), а интеграционные тесты видят обычные зависимости
пакета наравне с dev-зависимостями.

---

## 2. Единственный литерал, который остался — и почему

`crates/vibe-core/src/manifest/lockfile/tests.rs:249`, тест
`read_rejects_non_current_version`:

```rust
    // A pre-v7 lockfile is rejected outright — no legacy reader, no
    // migration. The fix is to regenerate with `vibe install`.
    std::fs::write(
        &path,
        "[meta]\ngenerated_by = \"old\"\ngenerated_at = \"x\"\nschema_version = 6\n",
    )
    .unwrap();
    let err = Lockfile::read(&path).unwrap_err();
    assert!(
        matches!(
            err,
            crate::error::Error::UnsupportedLockfile {
                found: 6,
                expected: 7
            }
        ),
```

Это ровно та фикстура, про которую пакет сказал «намеренно тестирует
старую схему — оставить литерал и назвать в отчёте». Её предмет — отказ
читателя, а не приём: `6` здесь не «текущая схема, записанная числом», а
«какая-нибудь не-текущая». Подстановка константы превратила бы тест в
тавтологию (`CURRENT` отвергает `CURRENT`), а `CURRENT - 1` связала бы
утверждение с арифметикой, которой у схемы нет: версии не обязаны идти
подряд, и отвергается не предыдущая, а всякая иная. Файл вдобавок лежит
в периметре соседнего воркера (§5).

Проверка, что больше литералов не осталось:

```
$ grep -rn "schema_version = 6" --include=*.rs crates/ xtask/
crates/vibe-core/src/manifest/lockfile/tests.rs:249:        "[meta]\ngenerated_by = \"old\"\ngenerated_at = \"x\"\nschema_version = 6\n",
```

**Вне `.rs` литерал `6` встречается ещё в пяти местах, и ни одно не является
тестовой фикстурой** — поэтому не тронуты:
`manual-tests/M1.6-mirror-vendor-smoke.md:245` (сценарий ручного теста,
`cargo test` его не видит); `research/rust-demo/vibe.lock:4` и
`research/ts-demo/vibe.lock:4` (настоящие локи демо-проектов);
`vibevm/vibepacks/org.vibevm.fractality/delegation-rules/v1.0.0/vibe.lock:4`
и `…/fractality/v1.0.0/vibe.lock:4` (локи опубликованных пакетов —
исторические артефакты, а не входы теста).

---

## 3. Дословный вывод гейта

### 3.1 Почему гейт прогнан в приватном `CARGO_TARGET_DIR`

Первый прогон `cargo test --workspace --no-fail-fast` в общем
`target/` дал **23** упавших цели, из которых тринадцать — доктесты с
`can't find crate for vibe_workspace` и `found possibly newer version of
crate vibe_specdoc`. Это не красное дерева, а гонка: соседний воркер
пересобирал `vibe-core`, `vibe-wire` и `vibe-specdoc` в тот же
`target/` во время прогона. Измерение по такому выводу не доказывает
ничего, поэтому гейт перезапущен с
`CARGO_TARGET_DIR=<scratch>/target-p1o5` — по указанию оркестратора.

Приватный прогон охватывает все пять пакетов, которых касается правка,
плюс `vibe-index` (ради корпуса `index/e1`):

```
$ CARGO_TARGET_DIR=<scratch>/target-p1o5 cargo test --no-fail-fast \
    -p vibe-cli -p vibe-install -p vibe-mcp -p vibe-requirements \
    -p vibe-workspace -p vibe-index
```

### 3.2 Цели, которые чинила правка — все зелёные

```
     Running tests\cli_facts.rs (…\target-p1o5\debug\deps\cli_facts-4455dd462a7f83de.exe)
running 6 tests
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.65s

     Running tests\cli_mcp_path_parity.rs (…\deps\cli_mcp_path_parity-e5f337e4684dc94b.exe)
running 1 test
test list_json_and_query_package_return_identical_files_written_paths ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.88s

     Running tests\cli_redirect.rs (…\deps\cli_redirect-0677554a6ed7ce2c.exe)
running 9 tests
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.77s

     Running tests\incremental_in_place.rs (…\deps\incremental_in_place-b91069573c14bcdc.exe)
running 1 test
test general_install_defers_in_place_instead_of_recloning ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.41s

     Running tests\tools_oracle.rs (…\deps\tools_oracle-9a8db54c24df7e45.exe)
running 24 tests
test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

     Running unittests src\lib.rs (…\deps\vibe_requirements-630e49e962de19cc.exe)
running 40 tests
test result: ok. 40 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s

     Running unittests src\lib.rs (…\deps\vibe_workspace-a4258f1af9464203.exe)
running 542 tests
test result: ok. 542 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 20.42s
```

Внутри последних двух зелены и сами фикстуры:
`bins::tests::collect_walks_lockfile_slots_and_sorts`,
`freshness::tests::*`, `tests_followup::*`, `tests_provider::*`,
`tests_query::*`.

### 3.3 Хвост прогона

```
error: 4 targets failed:
    `-p vibe-cli --test cli_search`
    `-p vibe-cli --test vvm`
    `-p vibe-index --test golden_corpus`
    `-p vibe-index --doc`
PRIVATE-TEST-EXIT=101
```

---

## 4. Что осталось красным и почему — ни одно не про схему лока

### 4.1 `-p vibe-index --test golden_corpus` — перелом провода (J-069)

Дословно тот же дрейф, который разобрал P1-O4 §3: зависимость байтов
gzip-эмиттера от конфигурации сборки.

```
thread 'the_catalog_is_the_projection_of_its_journal' panicked at crates\vibe-index\tests\golden_corpus.rs:250:5:
the golden corpus no longer reproduces from its journal — either the journal or the projection changed:
  `primary.jsonl.gz` differs (binary): committed 1364 byte(s), projected 1368 byte(s)
  `repomd.json` differs first at line 75:
  committed:       "size": "1364",
  projected:       "size": "1368",
```

Пакет прямо вывел корпус `index/e1` за периметр; не тронут.

### 4.2 `-p vibe-cli --test cli_search` — чужая живая работа по проводу

Два теста, и оба про поля JSON-конверта `search`, а не про лок:

```
thread 'search_full_scan_finds_matching_packages_in_github_org' panicked at crates\vibe-cli\tests\cli_search.rs:682:59:
called `Option::unwrap()` on a `None` value
```

(строка 682 — `v["registries_full_scanned"].as_array().unwrap()`)

```
thread 'search_without_full_scan_keeps_unconfigured_status' panicked at crates\vibe-cli\tests\cli_search.rs:760:5:
assertion `left == right` failed
  left: 0
 right: 1
```

(строка 760 — `assert_eq!(unconfigured.len(), 1)`)

`cli_search.rs` вообще не содержит `schema_version = 6`: единственное
вхождение схемы в нём — `"schema_version": 1` в JSON-фикстуре индекса
(строка 58), к локу отношения не имеющее. Источник — незавершённая правка
соседа в `crates/vibe-wire/src/generated/**`, `crates/vibe-index/src/cli/search.rs`
и `formats/vocabularies.json` (§5).

### 4.3 `-p vibe-index --doc` — чужая живая работа, половина типов ещё не сгенерирована

```
error[E0432]: unresolved imports `vibe_wire::generated::shared::DocumentationEntry`, `vibe_wire::generated::shared::DocumentsEntry`, `vibe_wire::generated::shared::MediaEntry`, `vibe_wire::generated::shared::TranslatesEntry`
  --> crates\vibe-index\src\types\entry\documentation.rs:21:5
```

`crates/vibe-index/src/types/entry/documentation.rs` на момент прогона —
**неотслеживаемый** файл (`??` в `git status`), которого в дереве на входе
сессии не было. Это работа P2-O1 по `doc`-пакетам, не моя.

### 4.4 `-p vibe-cli --test vvm` — артефакт самого приватного каталога

Два теста утверждают, что запущенный бинарь опознаётся как «прямой запуск
из исходников»:

```
command=`"…\scratchpad\target-p1o5\debug\vibe.exe" "self" "which"`
stderr="error: no active version (…; fix: select one with `vibe self use <selector>`, or pass an explicit selector)"
```

Признак «direct source execution» выводится из того, что бинарь лежит в
`target/` самого репозитория. Приватный `CARGO_TARGET_DIR` его ломает по
построению. **В общем `target/` эта цель зелёная** — её нет среди 23
упавших целей первого прогона. То есть это цена обхода, а не красное
дерева; при обычном прогоне панели она не появится.

### 4.5 Свод

| Цель | Причина | Мой мандат? |
|------|---------|-------------|
| `vibe-index --test golden_corpus` | перелом провода J-069 (§4.1) | выведена пакетом за периметр |
| `vibe-cli --test cli_search` | живая правка провода соседом (§4.2) | нет |
| `vibe-index --doc` | неотслеживаемый файл соседа (§4.3) | нет |
| `vibe-cli --test vvm` | артефакт приватного `CARGO_TARGET_DIR` (§4.4) | нет; в общем `target/` зелена |

Ни одна цель из каскада, который P1-O4 §4.1 приписал `schema_version = 6`
(`cli_facts`, `cli_mcp_path_parity`, `cli_redirect`, `tools_oracle`,
`incremental_in_place`, `vibe-requirements`, `vibe-workspace`), больше не
краснеет.

---

## 5. Замечание о среде — дерево общее и очень живое

Параллельно работает воркер P2-O1 по `doc`-пакетам. За время сессии:

- HEAD сдвинулся четырежды: `dccaeac6` → `ba93b48a` → … → `7c829c0d`;
- дважды дерево не собиралось по чужой незавершённой правке —
  `crates\vibe-check\src\checks\boot_directory.rs:45: cannot find function
  is_doc_package`, и отдельно `failed to load manifest for workspace member
  crates\vibe-doc … dependency.regex was not found in workspace.dependencies`.
  Оба раза повтор через минуту проходил;
- первый прогон панели в общем `target/` дал тринадцать доктест-целей с
  `can't find crate` / `found possibly newer version of crate` — отсюда и
  переход на приватный каталог (§3.1).

`cargo fmt --all` **не запускался** намеренно: он переформатировал бы
незакоммиченные файлы соседа. Вместо него `rustfmt --edition 2024`
прогнан ровно по десяти моим файлам, и проверка чиста:

```
$ rustfmt --edition 2024 --check <мои десять файлов>
FMT-CHECK-EXIT=0
```

Бюджет в 600 строк соблюдён: самый близкий к нему файл —
`crates/vibe-workspace/src/freshness.rs`, 596 строк после правки (+1).

---

## 6. `git status --short` и состав коммита

Мои файлы — одиннадцать продуктовых плюс этот отчёт:

```
crates/vibe-cli/tests/cli_facts.rs
crates/vibe-cli/tests/cli_mcp_path_parity.rs
crates/vibe-cli/tests/cli_redirect.rs
crates/vibe-install/tests/incremental_in_place.rs
crates/vibe-mcp/tests/tools_oracle.rs
crates/vibe-mcp/tests/tools_oracle/dispatch_compat.rs
crates/vibe-requirements/src/tests_followup.rs
crates/vibe-requirements/src/tests_provider.rs
crates/vibe-requirements/src/tests_query.rs
crates/vibe-workspace/src/bins/tests.rs
crates/vibe-workspace/src/freshness.rs
campaigns/docs-2026-09/findings/WORKER-REPORT-P1-O5.md
```

```
 crates/vibe-cli/tests/cli_facts.rs                 |  5 ++-
 crates/vibe-cli/tests/cli_mcp_path_parity.rs       | 16 ++++++--
 crates/vibe-cli/tests/cli_redirect.rs              |  9 +++--
 crates/vibe-install/tests/incremental_in_place.rs  | 17 +++++----
 crates/vibe-mcp/tests/tools_oracle.rs              | 43 ++++++++++++++--------
 .../vibe-mcp/tests/tools_oracle/dispatch_compat.rs |  4 +-
 crates/vibe-requirements/src/tests_followup.rs     |  3 +-
 crates/vibe-requirements/src/tests_provider.rs     |  3 +-
 crates/vibe-requirements/src/tests_query.rs        |  3 +-
 crates/vibe-workspace/src/bins/tests.rs            | 15 ++++++--
 crates/vibe-workspace/src/freshness.rs             |  7 ++--
```

Индекс делится со всеми воркерами, поэтому коммит сделан явной формой
`git commit … -- <мои пути>`: он берёт из рабочего дерева только
перечисленные файлы и не может увести чужое застейдженное. `git add`
чужих путей не делался, `git push` не делался.

---

## 7. Долг, который правка не покрывает (для следующей волны)

Каскад, починенный здесь, воспроизведётся при следующем подъёме схемы:
в дереве остаётся **пятнадцать** фикстур, пишущих литерал `7`. Пакет
ограничил мандат литералами `6`, и почти все эти точки лежат в периметре
соседнего воркера (`crates/vibe-core`, `crates/vibe-cli/src`), поэтому они
не тронуты. Перепись на момент сдачи:

```
crates/vibe-agent-projection/src/pkgskill/projection/tests.rs:254
crates/vibe-check/src/checks/local_source_freshness.rs:163
crates/vibe-check/src/checks/local_source_freshness.rs:265
crates/vibe-check/src/checks/lockfile_files.rs:153
crates/vibe-check/src/checks/lockfile_files.rs:186
crates/vibe-check/src/checks/lockfile_files.rs:216
crates/vibe-check/src/checks/lockfile_files.rs:255
crates/vibe-cli/src/commands/show/source_path.rs:274
crates/vibe-core/src/manifest/lockfile/tests.rs:22
crates/vibe-core/src/manifest/lockfile/tests.rs:106
crates/vibe-core/src/manifest/lockfile/tests.rs:161
crates/vibe-core/src/manifest/lockfile/tests.rs:195
crates/vibe-core/src/manifest/lockfile/tests.rs:277
crates/vibe-core/src/manifest/lockfile/tests.rs:321
crates/vibe-core/src/manifest/lockfile/tests.rs:358
crates/vibe-core/src/manifest/lockfile/tests.rs:394
```

Часть из них — оракулы самого читателя лока в `vibe-core`, и там литерал
может быть предметом теста ровно так же, как в §2; разбирать их
поштучно — работа отдельного пакета, а не догадка на выходе этого.
