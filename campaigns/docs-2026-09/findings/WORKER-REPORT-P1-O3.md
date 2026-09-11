# WORKER-REPORT-P1-O3 — манифест дистрибуции как JTD-схема

Дата: 2026-09-12. Ветка `research-preview-1-docs`.
Дерево на старте работы: `0df597b3` (пакет называл `b1291b06`; центральная
сессия за время фазы 0 добавила `ab7731b6`, `69053551`, `42457142`, `0df597b3`
— в том числе коммит P1-O2, поднявший `wire-derive-baseline.json` до
`vibe-publish = 7`). Именно на этой базовой линии измерен результат.

**Итог:** шаг 3 панели (`wire-derive ratchet`) зелёный —
`wire-derive ratchet holds: 178 handwritten derive file(s) match
wire-derive-baseline.json`. `wire-derive-baseline.json` не тронут; в
`crates/vibe-publish/src/release_manifest.rs` не осталось ни одной рукописной
derive; байтовая форма `DISTRIBUTION.json`, `DISTRIBUTIONS.json` и фрагмента
платформы не сдвинулась ни на байт (голдены до/после, md5 совпадают).

---

## 1. Что сделано

### 1.1 Схема

Создана **одна** схема:
`schemas/distribution/e1/aggregate_distribution_manifest.jtd.json`.

Корень — `AggregateDistributionManifest` (`DISTRIBUTIONS.json`); все
остальные шесть типов семейства — её `definitions`:

| definition | сгенерированный тип |
| --- | --- |
| `platform_distribution_fragment` | `PlatformDistributionFragment` |
| `bundle_distribution_manifest` | `BundleDistributionManifest` |
| `distribution_component` | `DistributionComponent` |
| `distribution_component_name` | `DistributionComponentName` |
| `distribution_source_archive` | `DistributionSourceArchive` |
| `distribution_asset` | `DistributionAsset` |
| `byte_count` | `pub type ByteCount = u64;` |

**Почему одна схема, а не две или три.** Документов-корней три
(`DISTRIBUTION.json`, фрагмент платформы, `DISTRIBUTIONS.json`), но это одно
дерево: фрагмент — элемент `platforms[]` агрегата, а бандл-манифест — поле
`bundle` фрагмента. Две схемы отчеканили бы **два разных Rust-типа**
`BundleDistributionManifest` (у JTD нет межфайловых ссылок), и
`PlatformDistributionFragment::new(asset, bootstrap, bundle)` перестал бы
принимать то, что возвращает `BundleDistributionManifest::from_json_slice`.
Единственная альтернатива — вынести общие фрагменты в
`formats/vocabularies.json`; она отвергнута: это дом **словарей**, общих
между независимыми форматами, а здесь речь об одном семействе, и семь его
объектов засорили бы общее пространство `generated::shared`.

### 1.2 Решения по форме

- **Порядок полей.** `jtd-codegen` печатает поля алфавитно, а
  `serde_json::to_vec_pretty` пишет их в порядке объявления — то есть без
  вмешательства байты бы поехали. Поэтому каждый объект (корень и пять
  объектных definitions) несёт `metadata."x-wire-order"` — полную
  перестановку своих членов в том порядке, в котором их печатал рукописный
  код. Это тот же приём, которым пользуется `schemas/scrape/e2/*.jtd.json`.
- **`size: u64`.** В JTD нет формы `uint64` (RFC 8927 доходит до `uint32`).
  Взят ровно приём соседей (`scrape/e2/retirement_checkpoint.jtd.json`,
  `prepared_health_snapshot`, `deploy_lock_resources`): definition
  `byte_count` объявлен `"type": "float64"` с `"x-rust-type": "u64"`, и
  проход `domain_types` переписывает правую сторону алиаса. На проводе —
  то же JSON-число, что и раньше; в Rust — тот же `u64`.
  **Замечание для босса (не расхождение с пакетом, но с общим правилом):**
  `formats/breaks/003.md` записывает стоячее правило владельца (B-091,
  2026-08-20) «целые шире 32 бит едут по проводу десятичной строкой». Здесь
  оно **не** применено — пакет прямо запрещает менять байтовую форму
  `DISTRIBUTION.json`/`DISTRIBUTIONS.json`, а `size` там всегда ехал числом;
  ровно так же поступили все эпоха-2 схемы `scrape/**`, посаженные уже
  после этого правила. Если владелец захочет исполнить B-091 и на этом
  семействе — это отдельный перелом с запиской и правкой обоих инсталлеров.
- **`deny_unknown_fields`.** Реестр несёт `unknown_fields = "deny"` при
  честном `foreign_parsers = "many"` — тот же расклад, что у семейства
  `scrape-*`. `many` правдиво: `DISTRIBUTIONS.json` скачивают и разбирают
  `distribution/install/install.sh:281` (POSIX sh) и `install.ps1:25`
  (PowerShell) **до** того, как на машине появится хоть один бинарник vibe;
  бандл-манифест едет внутри агрегата (`platforms[].bundle`), то есть те же
  чужие парсеры ходят по его форме. `deny` тоже правдиво: сегодняшние
  читатели отвергают неизвестное поле (тест
  `strict_json_rejects_unknown_fields`), и пакет запрещает это менять.
- **Словарь.** `distribution_component_name` помечен
  `"x-vocabulary": "closed"`: код сопоставляет имя компонента с
  фиксированным путём назначения, у незнакомого имени нет безопасного
  поведения. Генератор печатает `Vibe` / `VibeIndex` с
  `#[serde(rename = …)]` — ровно прежние имена вариантов.
- **`x-empty: "emit"`** на обеих коллекциях (`platforms`, `components`) —
  единственная законная политика для обязательного члена (правило R21).

### 1.3 Реестр

`formats/REGISTRY.toml` — две записи на одну схему (прецедент
`index-entry` / `index-primary`), обе `epoch = 1`, `recoverable = false`,
`foreign_parsers = "many"`, `unknown_fields = "deny"`, `corpus = "none"`:

- `[format.distribution-bundle-manifest]` — `DISTRIBUTION.json`;
- `[format.distribution-aggregate-manifest]` — `DISTRIBUTIONS.json`.

Роли обязаны совпадать: `Strictness::read` (`xtask/src/codegen/strictness.rs`)
громко отказывается, если две записи на одну схему расходятся по
`foreign_parsers`, а `schema_denies_unknown_fields` — если по
`unknown_fields`.

`recoverable = false` — суждение, называю его вслух: документ фиксирует
точный размер и дайджест байтов **одной** сборки, и после факта их неоткуда
перепроецировать (класс `artifact-record`). Фрагмент платформы отдельной
записи не получил: это промежуточный артефакт CI между job'ами, у него нет
чужого парсера, а как форма он инвентаризован внутри агрегата.

**Записки перелома нет.** Ни одна из четырёх `formats/breaks/00N.md` не
пишется под регистрацию нового формата — все четыре про сдвиг уже
существующего провода; десятки записей реестра (`scrape/**`,
`index_http/**`, `deploy/**`) заведены без записок. Здесь провод не
сдвинулся вовсе (голдены), так что записывать нечего.

### 1.4 Код

`cargo xtask codegen` положил типы в
`crates/vibe-wire/src/generated/distribution/e1/aggregate_distribution_manifest/mod.rs`
(107 подмодулей вместо 106).

Поведение — константы, независимые лимиты парсера, реляционная валидация,
детерминированные сериализаторы и тип ошибки — переехало в
**`crates/vibe-wire/src/behaviour/release_manifest.rs`** (+ `error.rs`,
`validation.rs` рядом). `crates/vibe-publish/src/release_manifest.rs` стал
фасадом из одного `pub use` (плюс прежний `specmark::scope!` и прежний
`mod tests`).

**Почему поведение переехало — и почему иначе было нельзя.** Пакет
предлагал «`pub use` и `impl` блоки». `impl` блока не существует: как только
`BundleDistributionManifest` определён в `vibe-wire`, `impl
BundleDistributionManifest { … }` в `vibe-publish` — это E0116 (inherent impl
для чужого типа). Это касается не только `as_str`, а **всех** методов
семейства: `new`, `validate`, `to_json_bytes`, `from_json_slice` на трёх
типах. Запасной вариант пакета — «тонкая обёртка без derive» — здесь тоже не
работает: поля этих типов читаются и конструируются литералами по всему
дереву (`xtask/src/dist/build.rs:413,428,461,467`,
`crates/vibe-cli/src/commands/vvm/bundle/tests.rs:91…`), а newtype ломает и
литерал, и доступ к полю. Дом у поведения ровно один, и он назван в законе
самого крейта — `crates/vibe-wire/src/lib.rs:23-30`: «a type this crate
defines can carry its behaviour … **ONLY here — the orphan rule bars every
other home**». Слой `behaviour/` уже несёт 3200 строк такого рукописного
поведения (`native_build.rs`, `native_mechanism.rs`, `vocabularies.rs` …).

**Публичный API `vibe-publish` сохранён полностью.** Те же имена типов, те
же константы, те же методы, тот же модуль `vibe_publish::release_manifest` и
тот же список `pub use` в `lib.rs` (файл не тронут). Единственное изменение
сигнатуры — `DistributionComponentName::as_str(self)` →
`as_str(&self)`: сгенерированный enum несёт только пол derive'ов
(`Debug, Clone, PartialEq, Eq, Serialize, Deserialize` —
`xtask/src/codegen/derive_floor.rs:65`), `Copy` там нет и добавить его
схемой нельзя. Все места вызова (`component.name.as_str()`) работают без
правки; форма `as_str(&self) -> &'static str` — буквально форма соседа
`NamingConvention::as_str` (`crates/vibe-wire/src/behaviour/vocabularies.rs:79`).

**Потеря `Ord`/`Copy` и как сохранён порядок байтов.** Рукописный enum нёс
`Copy, PartialOrd, Ord`; три `sort_by_key(|c| c.name)` опирались на них.
Заменено на `sort_by_key(|c| component_sort_key(&c.name))`, где
`component_sort_key` возвращает `name.as_str()`: `"vibe" < "vibe-index"`
лексикографически — тот же порядок, что давал производный `Ord` по порядку
объявления. `validate_components` по той же причине строит `BTreeSet` из
`&str`, а не из значений enum'а; семантика (ровно два компонента, ровно эти
два имени) не изменилась — это доказывают неизменённые тесты
`bundle_requires_exactly_the_two_runtime_components` и
`serialization_is_deterministic_and_canonicalizes_vector_order`.

**Тесты не тронуты вовсе.** `crates/vibe-publish/src/release_manifest/tests.rs`
остался байт-в-байт прежним (`git diff --stat` по нему пуст) и продолжает
проверять всё семейство через публичный фасад — включая
`strict_json_rejects_unknown_fields` и `source_archive_is_required_and_strict`.

### 1.5 Файлы

Создано:

- `schemas/distribution/e1/aggregate_distribution_manifest.jtd.json`
- `crates/vibe-wire/src/behaviour/release_manifest.rs`
- `crates/vibe-wire/src/behaviour/release_manifest/error.rs` (переезд, тело без изменений)
- `crates/vibe-wire/src/behaviour/release_manifest/validation.rs` (переезд; изменены только два места — см. §1.4)
- `crates/vibe-wire/src/generated/distribution/**` (только `cargo xtask codegen`)

Изменено:

- `formats/REGISTRY.toml` — две записи формата + комментарий-обоснование
- `crates/vibe-publish/src/release_manifest.rs` — фасад
- `crates/vibe-publish/Cargo.toml` — `vibe-wire.workspace = true`
- `crates/vibe-wire/Cargo.toml` — `specmark.workspace`, `thiserror.workspace`
- `crates/vibe-wire/src/behaviour/mod.rs` — `pub mod release_manifest;`
- `crates/vibe-cli/src/commands/vvm/bundle/archive.rs:260` — одна правка:
  `match (component.name, windows)` → `match (&component.name, windows)`
- `crates/vibe-wire/src/generated/**/mod.rs` (28 файлов) — только строка
  заголовка со списком домов схем + новая `pub mod distribution;`; всё
  генерируется
- `specmap.json` — `cargo xtask specmap`
- `Cargo.lock` — ровно три новых ребра зависимостей, ни одного апгрейда

Удалено (переехало):

- `crates/vibe-publish/src/release_manifest/error.rs`
- `crates/vibe-publish/src/release_manifest/validation.rs`

**Не тронуто:** `wire-derive-baseline.json`, `crates/vibe-publish/src/lib.rs`,
`distribution/install/*`, `formats/EPOCHS.toml`, `formats/vocabularies.json`,
`xtask/src/dist/**`, `xtask/src/codegen/**`, тесты.

---

## 2. Голдены до и после

Способ: временный `#[test] golden_dump_for_review` в
`crates/vibe-publish/src/release_manifest/tests.rs` (использует уже
существующие фикстуры `bundle()` / `fragment()` / `aggregate()` того же
файла), пишет три документа в каталог из `VIBE_GOLDEN_OUT`. Прогнан до
правки и после неё; **после снятия голденов тест удалён**, файл тестов
вернулся к исходным байтам.

```
$ diff -r before after; echo "DIFF-EXIT=$?"
DIFF-EXIT=0
$ md5sum before/* after/*
a20c9c85eac8a58ba600c88949761b63 *before/DISTRIBUTION.json
62d1926f6c4c95d3cff51c1169c8d215 *before/DISTRIBUTIONS.json
8e9ef1faa2002c1ab97113d23492d003 *before/FRAGMENT.json
a20c9c85eac8a58ba600c88949761b63 *after/DISTRIBUTION.json
62d1926f6c4c95d3cff51c1169c8d215 *after/DISTRIBUTIONS.json
8e9ef1faa2002c1ab97113d23492d003 *after/FRAGMENT.json
```

Размеры совпадают тоже (812 / 6815 / 1461 байта). Файлы лежат в
`<scratch>\vibe-docs-phase1\P1-O3\{before,after}\`.

---

## 3. Самопроверка (вывод дословный)

### 3.1 `cargo xtask check-codegen; echo EXIT=$?`

```
EXIT=1
```

Последняя строка вывода:

```
Error: generated code under C:\Users\olegc\git\v\vibevm-docs\crates/vibe-wire/src/generated / C:\Users\olegc\git\v\vibevm-docs\vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v1.0.0\crates/core-ai-native-specmap/src/generated differs from what this machine's jtd-codegen emits for the current schemas and `formats/REGISTRY.toml`.
```

**Это не дрейф схем, а свойство гейта в рабочем дереве воркера.**
`run_check_codegen` сначала прогоняет `run_codegen()`, потом делает
`git diff --exit-code -- <оба дерева>` — то есть сравнивает рабочее дерево с
**индексом**. Любая незакоммиченная перегенерация делает этот шаг красным по
построению; коммитит центральная сессия, и после `git add` он станет
зелёным. Ни один из 28 файлов в этом diff не отличается от свежей эмиссии —
что проверено прямо:

```
$ cp -r crates/vibe-wire/src/generated <scratch>/gen1
$ cargo xtask codegen
xtask codegen: ...crates/vibe-wire/src/generated (107 submodules).
$ diff -r <scratch>/gen1 crates/vibe-wire/src/generated; echo "CODEGEN-IDEMPOTENT-EXIT=$?"
CODEGEN-IDEMPOTENT-EXIT=0
```

То есть ровно то утверждение, которое `check-codegen` проверяет («дерево
байт-в-байт равно эмиссии этой машины для текущих схем и
`formats/REGISTRY.toml`»), выполняется; `git diff` его не видит только
потому, что изменения ещё не в индексе. Версия генератора — пин:
`jtd-codegen 0.4.1`.

### 3.2 `cargo xtask wire-diff 2>&1 | tail -20`

```
rebuild: differs `primary.jsonl.gz`
rebuild: differs `repomd.json`
Error: rebuild --check: 2 drift item(s) against the journal under `C:\Users\olegc\git\v\vibevm-docs\formats/corpora/index/e1`. A catalog that differs from its journal's projection carries a fact the journal does not describe — a derived artifact holding truth (violates spec://org.vibevm.core/vibevm/common/PROP-044#truth; fix: regenerate the catalog FROM the journal — every vibe-index mutation reprojects it wholesale — and never edit the journal to match the catalog — that would launder the secret truth into the truth layer.)
```

**Красное унаследовано, не моё.** Падает `rebuild --check` над корпусом
`formats/corpora/index/e1` — это корпус форматов `index-*` / `journal`.
Мои две записи несут `corpus = "none"`, то есть `wire-diff` над ними ничего
не перестраивает. `xtask/src/rebuild.rs` — тонкая обёртка над
`vibe_index::cli::rebuild`, который перепроецирует каталог индекса из
журнала и не читает ни `schemas/`, ни `formats/REGISTRY.toml`, ни
`vibe-publish`. Ни один файл под `formats/corpora/` в моём дереве не
изменён (`git status --short`), а сгенерированный модуль
`generated/index/e1/repomd/` вообще не тронут (изменились только строки
заголовка в 28 директорных `mod.rs`). Чинить не стал — пакет велит записать
и не чинить.

### 3.3 `cargo test -p vibe-publish -p vibe-cli --quiet 2>&1 | tail -5`

```
    commands::show::source_path::tests::embedded_root_uses_exact_lock_row_and_threads_offline

test result: FAILED. 709 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.76s

error: test failed, to rerun pass `-p vibe-cli --bin vibe`
```

Полная причина падения:

```
failed to parse `…\vibe.lock`: missing field `upstream_authors`
   in `package.embedded_source` at line 17, column 1 (violates spec://org.vibevm.core/vibevm/VIBEVM-SPEC#manifest-schema; fix: add the missing field to vibe.toml)
```

**Красное унаследовано, не моё.** Поле `upstream_authors` обязательно в
`crates/vibe-core/src/manifest/package/embedded_source.rs:51`, а фикстура
лок-файла в самом тесте
(`crates/vibe-cli/src/commands/show/source_path.rs:287`) его не пишет —
строка `[[package.embedded_source]]` там обрывается на
`license_file_sha256`. Это хвост bridge-работы центральной сессии
(`7b465809 feat(bridge): separate package and upstream identity`). Мой
периметр туда не заходит. Чинить не стал.

Остальные тесты зелёные: `cargo test -p vibe-publish --quiet` — 82 + 12 + 3,
все `ok`; `cargo test -p vibe-cli --bin vibe commands::vvm` — 109 passed,
0 failed.

Остальные команды сборки/линта:

```
$ cargo build -p vibe-publish -p vibe-cli -p xtask      # чисто, 0 ошибок
$ cargo fmt --all --check; echo FMT-EXIT=$?
FMT-EXIT=0
$ cargo clippy -p vibe-publish -p vibe-wire --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 17.49s
$ cargo xtask specmap --check
specmap --check: clean (7930 spec units, 3422 tagged code items, 2968 edges, 0 suspects, 31 warnings).
specmap: ratchet gate — 0 gated orphan(s), 0 dispositioned (6 crate(s) exempt).
specmap: resolve gate — 0 unresolved host edge(s), 35 non-host edge(s) outside this map's jurisdiction.
```

`cargo clippy -p vibe-publish -p vibe-cli --all-targets -- -D warnings`
(команда из пакета) падает **до** того, как дойдёт до `vibe-cli`:

```
error: this function has too many arguments (8/7)
   --> crates\vibe-install\src\plan\fetch.rs:286:1
error: could not compile `vibe-install` (lib) due to 1 previous error
```

`crates/vibe-install/` в мой периметр не входит и мной не тронут; тот же
лимит нарушает и `xtask/src/bridge.rs:78`. Оба — унаследованные красные.
Проверено, что мой код чист: `cargo clippy -p vibe-cli --all-targets` (без
`-D warnings`) выдаёт ровно одно предупреждение, и оно указывает в
`vibe-install`, ни одно — в файлы, которые я правил.

### 3.4 `bash tools/self-check.sh 2>&1 | grep -E "ratchet|failed|passed" | head`

```
=== [3/47 01:24:18] wire-derive ratchet (handwritten Serialize/Deserialize derives vs wire-derive-baseline.json) ===
self-check: wire-derive ratchet holds: 178 handwritten derive file(s) match wire-derive-baseline.json.
self-check: ✓ [3/47] wire-derive ratchet (handwritten Serialize/Deserialize derives vs wire-derive-baseline.json) (4s)
test result: ok. 188 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.27s
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 106 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 71 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.85s
test result: ok. 72 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
    0: failed to parse `…\vibe.lock`: missing field `upstream_authors`
```

Полный прогон до первого падения (шаги 1–5):

```
self-check: ✓ [1/47] the floor builds every live package workspace (1s)
self-check: ✓ [2/47] instruction files identical (CLAUDE.md = AGENTS.md = GEMINI.md) (0s)
self-check: wire-derive ratchet holds: 178 handwritten derive file(s) match wire-derive-baseline.json.
self-check: ✓ [3/47] wire-derive ratchet (…) (4s)
self-check: ✓ [4/47] cargo fmt --all --check (7s)
=== [5/47] cargo test --workspace ===
commands::show::source_path::tests::embedded_root_uses_exact_lock_row_and_threads_offline --- FAILED
test result: FAILED. 709 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.56s
self-check: `cargo test --workspace` failed (exit 101, step 5/47, 14s)
```

**Шаг 3 — цель пакета — зелёный.** Первое падение — шаг 5, унаследованный
тест `source_path` (§3.3). Дальше скрипт не идёт (без `--keep-going` он
останавливается на первом падении), поэтому шаги 6b/6c/6d панели этим
прогоном не измерены; они прогнаны отдельно и приведены выше.

Наблюдение о среде: два из четырёх прогонов `self-check.sh` упали на шаге 5
не на тесте, а на `error: failed to remove file
C:\…\target\debug\vibe.exe / Access is denied. (os error 5)` — соседний
воркер (в дереве появились `findings/WORKER-REPORT-PP-C2.md` и
`findings/PP-C2-expects/`) держит тот же `target/`. Это флейк общей
директории сборки, не свойство изменений; повторный прогон проходит.

### 3.5 `git status --short`

Мои файлы:

```
 M Cargo.lock
 M crates/vibe-cli/src/commands/vvm/bundle/archive.rs
 M crates/vibe-publish/Cargo.toml
 M crates/vibe-publish/src/release_manifest.rs
 D crates/vibe-publish/src/release_manifest/error.rs
 D crates/vibe-publish/src/release_manifest/validation.rs
 M crates/vibe-wire/Cargo.toml
 M crates/vibe-wire/src/behaviour/mod.rs
 M crates/vibe-wire/src/generated/…/mod.rs        (28 директорных mod.rs, генерация)
 M formats/REGISTRY.toml
 M specmap.json
?? crates/vibe-wire/src/behaviour/release_manifest.rs
?? crates/vibe-wire/src/behaviour/release_manifest/
?? crates/vibe-wire/src/generated/distribution/
?? schemas/distribution/
?? campaigns/docs-2026-09/findings/WORKER-REPORT-P1-O3.md   (этот отчёт)
```

Чужое в дереве, не трогал и не трогаю:
`campaigns/docs-2026-09/findings/PACKET-P1-O3.md`,
`campaigns/docs-2026-09/findings/WORKER-REPORT-PP-C2.md`,
`campaigns/docs-2026-09/findings/PP-C2-expects/`.

---

## 4. Отклонения от пакета

### 4.1 Периметр расширен на слой поведения `vibe-wire` — вынужденно

Пакет перечислял к правке `release_manifest.rs`, `generated/**`,
«потребителей типов» и не называл `crates/vibe-wire/src/behaviour/**`,
`crates/vibe-wire/Cargo.toml` и `crates/vibe-publish/Cargo.toml`.

**Дефект пакета.** Инструкция «сохранить публичный API… через `pub use` и
`impl` блоки» невыполнима: `impl` для типа, определённого в другом крейте,
запрещён правилом сирот (E0116). Это касается не одного метода, а всех
одиннадцати (`new`/`validate`/`to_json_bytes`/`from_json_slice` на трёх
типах плюс `as_str`). Запасной вариант «обёртка без derive» тоже
невыполним: поля этих типов конструируются литералами и читаются по всему
дереву, newtype ломает и то и другое. Дом у поведения назван законом самого
`vibe-wire` (`lib.rs:23-30`) и ровно один.

Взято самое консервативное из выполнимого: тела `error.rs` и `validation.rs`
перенесены без изменений (кроме двух мест §1.4), публичные пути
`vibe_publish::release_manifest::*` и `vibe_publish::*` сохранены полностью,
`lib.rs` не тронут, тесты не тронуты.

`specmark` и `thiserror` в `vibe-wire/Cargo.toml` — следствие того же
переезда. `thiserror` позволил перенести тип ошибки **дословно** (21 вариант
с точными текстами сообщений) вместо рукописной транскрипции `Display`
(стиль соседей по `behaviour/`), то есть исключил риск изменить
пользовательский текст ошибки. `specmark` сохранил якорь
`#[spec(implements = "…PROP-019#instances")]` на типе ошибки — без него
карта потеряла бы ребро; `cargo xtask specmap` подтверждает: `0 unresolved
host edge(s)`.

### 4.2 `specmap.json` перегенерирован

Не назван в периметре, но `cargo xtask specmap --check` — гейт панели
(шаг 6c), а переезд файлов сдвигает карту. Дельта ровно ожидаемая:
+8 `<schema>`-элементов новой схемы, перенос элемента
`ReleaseManifestError` из `crates/vibe-publish/src/release_manifest/error.rs`
в `crates/vibe-wire/src/behaviour/release_manifest/error.rs`, +1 файловая
метка `vibe_wire::behaviour::release_manifest`. Три ребра добавлено, одно
убрано.

### 4.3 Установлен локальный `tools/jtd-codegen/jtd-codegen.exe`

В этом worktree генератора не было (`tools/jtd-codegen/` содержал только
`README.md`), а без него `cargo xtask codegen` не запускается. Бинарник
**скопирован** из соседнего чекаута `C:\Users\olegc\git\v\vibevm\tools\jtd-codegen\`
— без сети и без глобальной установки. Версия совпадает с пином
(`jtd-codegen 0.4.1`, `vibevm/vibepacks/org.vibevm.ai-native/jtd-codegen/v1.0.0/README.md`).
Каталог `tools/` игнорируется гитом (`tools/.gitignore`), так что в дифф это
не попадает — но следующая сессия в этом worktree найдёт генератор на месте.

---

## 5. Что не сделано и почему

- **Два унаследованных красных не починены** (пакет: «записать шаг и вывод,
  не чинить»): `cargo test --workspace` → `source_path` /
  `upstream_authors` (§3.3), и `cargo xtask wire-diff` → дрейф корпуса
  `formats/corpora/index/e1` (§3.2). К ним же примыкает унаследованное
  красное clippy в `crates/vibe-install/src/plan/fetch.rs:286` и
  `xtask/src/bridge.rs:78` (§3.3).
- **`cargo xtask check-codegen` остаётся `EXIT=1`** до `git add`: гейт
  сравнивает дерево с индексом (§3.1). Идемпотентность генерации доказана
  отдельно.
- **Корпус голденов (`formats/corpora/distribution/**`) не заведён** —
  записи несут `corpus = "none"`. Пакет корпуса не просил, а `wire-diff`
  над `corpus = "none"` ничего не требует. Если босс захочет, чтобы байты
  этих манифестов сторожил корпус, это отдельный маленький шаг: три
  авторских документа + смена `corpus` в двух записях.
- **Записка перелома не написана** — обоснование в §1.3.
- **Правило B-091 (широкие целые строкой) не применено** — обоснование в
  §1.2; это осознанный вопрос владельцу, а не упущение.
- **Ничего не закоммичено** — коммитит центральная сессия.
