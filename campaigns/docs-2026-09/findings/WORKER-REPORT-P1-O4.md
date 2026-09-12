# WORKER-REPORT-P1-O4 — три унаследованных красных шага панели

Пакет: `campaigns/docs-2026-09/findings/PACKET-P1-O4.md`.
HEAD на входе: `bb6320b10e240a1cc06fa641b1a88bd72317d189`.
HEAD на выходе: `b18453f2d12636bb671fb6ddc63c30084676f8f9` (см. §5 —
дерево общее с другими живыми воркерами, HEAD сдвинулся под работой).

**Итог одной строкой.** Пункты 1 и 2 пакета закрыты и проверены. Пункт 3
— **перелом провода, остановился по инструкции пакета**: дрейф корпуса
`formats/corpora/index/e1` не «протухший голден», а *зависимость байтов
эмиттера от конфигурации сборки* (unification фич `flate2`), доказанная
контролируемым экспериментом. Панель остаётся красной на шаге 2, причём
по **двум** причинам, ни одна из которых — не та, что назвал пакет:
унаследованная фикстура `schema_version = 6` в `cli_facts.rs` (файл вне
названного периметра) и тот же самый перелом провода.

---

## 1. Что изменено — файл за файлом

Трогал ровно три файла продукта плюс этот отчёт.

### 1.1 `crates/vibe-cli/src/commands/show/source_path.rs`

Одна строка в фикстуре `bridge_package`: в TOML-строку
`[[package.embedded_source]]` добавлено
`upstream_authors = ["Example Upstream Authors"]` между `content_hash` и
`upstream_license`.

*Почему:* поле обязательно в `LockedEmbeddedSource`
(`crates/vibe-core/src/manifest/lockfile.rs:250`) с коммита `7b465809`, а
фикстура — **лок-файл**, не манифест, поэтому правится по схеме лока, а не
по `EmbeddedSourceDecl`. Порядок полей и формулировка значения взяты
один-в-один из канонической фикстуры лока
`crates/vibe-core/src/manifest/lockfile/tests.rs:124`. Ни тест, ни схема
не тронуты — как и требовал пакет.

### 1.2 `crates/vibe-install/src/plan/fetch.rs`

`sync_fetched_to_graph` был 8-арной функцией (`too_many_arguments`, 8/7),
из которых шесть параметров она **транзитом** передавала в
`fetch_or_defer`. Приобретение узла вынесено в параметр-замыкание:

```rust
fn sync_fetched_to_graph(
    graph: &vibe_resolver::ResolvedGraph,
    fetched: &mut Vec<Fetched>,
    mut acquire: impl FnMut(&ResolvedNode) -> Result<Fetched>,
) -> Result<()>
```

Единственный вызов (в `expand_conditional_deps`) держит контекст у себя:

```rust
sync_fetched_to_graph(&effective.graph, fetched, |node| {
    fetch_or_defer(
        source, node, lockfile, store_root, root_features, workspace_root, offline,
    )
})?;
```

*Почему именно так, а не структурой-контекстом:* соседняя функция в этом
же файле несёт `#[expect(clippy::too_many_arguments)]` с обоснованием
«bundling the borrows into a struct would only rename the arity»
(`fetch.rs:172`). Замыкание арность не переименовывает, а **снимает**:
пересадка вектора на граф обязана знать, *какие* узлы новые, и не обязана
знать, *как* узел добывается. Заодно ушёл генерик-параметр `S`. `#[allow]`
не добавлен — пакет это прямо запретил. Поведение не менялось: тот же
порядок обхода, тот же `fetch_or_defer` с теми же аргументами.

### 1.3 `xtask/src/bridge.rs`

`pin` был 8-арной функцией (`too_many_arguments`, 8/7) из восьми полей
одной и той же записи `[[embedded_source]]`. Поля варианта
`BridgeCommand::Pin { … }` вынесены в `#[derive(clap::Args)] pub struct
PinArgs`, вариант стал `Pin(PinArgs)`, сигнатура — `fn pin(args:
&PinArgs) -> Result<()>`.

*Почему:* восемь позиционных заимствований одной записи — это и есть
запись; clap для tuple-варианта с `Args`-структурой разворачивает те же
флаги, поэтому **поверхность CLI не изменилась**: те же `--name`,
`--url`, `--commit`, `--ref-hint`, `--upstream-author` (повторяемый,
`required = true`), `--upstream-license`, `--license-path`,
`--license-url`, тот же about-текст (doc-комментарий остался на варианте).
`#[allow]` не добавлен.

### 1.4 `campaigns/docs-2026-09/findings/WORKER-REPORT-P1-O4.md`

Этот отчёт.

**Голдены не регенерированы** — обоснование в §3.

`cargo fmt --all` прогнан после правок.

---

## 2. Дословный вывод

### 2.1 Пункт 1 пакета — тест `source_path` зелёный

```
$ cargo test -p vibe-cli --bin vibe --quiet commands::show::source_path
running 4 tests
....
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

### 2.2 Пункт 2 пакета — обе названные точки clippy зелёные

До правки (обе воспроизведены дословно):

```
error: this function has too many arguments (8/7)
   --> crates\vibe-install\src\plan\fetch.rs:286:1
    |
286 | / fn sync_fetched_to_graph<S: InstallSource + ?Sized>(
...
error: could not compile `vibe-install` (lib) due to 1 previous error
```

```
error: this function has too many arguments (8/7)
  --> xtask\src\bridge.rs:78:1
   |
78 | / fn pin(
...
error: could not compile `xtask` (bin "xtask") due to 1 previous error
```

После правки:

```
$ cargo clippy -p vibe-install -p xtask --all-targets --quiet -- -D warnings
CLIPPY-TWO-CRATES-EXIT=0
```

### 2.3 Шаг 3 панели целиком — `cargo clippy --workspace`

```
$ cargo clippy --workspace --all-targets --keep-going --quiet -- -D warnings
error: unneeded unit return type
error: unneeded unit return type
error: could not compile `vibe-specdoc` (test "docs_corpus") due to 2 previous errors
KEEPGOING-EXIT=101
```

Полный текст единственного оставшегося красного:

```
error: unneeded unit return type
   --> crates\vibe-specdoc\tests\docs_corpus.rs:428:49
    |
428 |     fn unwrap_err_or_else(self, f: impl FnOnce() -> ()) -> E;
    |                                                 ^^^^^^ help: remove the `-> ()`
...
error: unneeded unit return type
   --> crates\vibe-specdoc\tests\docs_corpus.rs:432:49
```

`--keep-going` означает, что проверен **весь** воркспейс: больше нигде ни
одного предупреждения. Подтверждение:

```
$ cargo clippy --workspace --exclude vibe-specdoc --all-targets --keep-going --quiet -- -D warnings
STEP3-MINUS-SPECDOC-EXIT=0
```

**Это не унаследованный красный и не мой.** `crates/vibe-specdoc/tests/docs_corpus.rs`
в `git status` имеет статус `??` — **нового, неотслеживаемого файла**,
которого в дереве не было на входе сессии; его создал живой соседний
воркер уже во время этого прогона (§5). Чинится удалением `-> ()` в двух
строках, но файл принадлежит чужой незавершённой работе, поэтому не тронут.

**Вывод по шагу 3: обе названные пакетом точки закрыты; воркспейсный
шаг зелен везде, кроме чужого неотслеживаемого файла.**

### 2.4 Шаг 6d панели — `cargo xtask wire-diff`

```
$ cargo xtask wire-diff
rebuild: differs `primary.jsonl.gz`
rebuild: differs `repomd.json`
Error: rebuild --check: 2 drift item(s) against the journal under `C:\Users\olegc\git\v\vibevm-docs\formats/corpora/index/e1`. A catalog that differs from its journal's projection carries a fact the journal does not describe — a derived artifact holding truth (violates spec://org.vibevm.core/vibevm/common/PROP-044#truth; fix: regenerate the catalog FROM the journal — every vibe-index mutation reprojects it wholesale — and never edit the journal to match the catalog — that would launder the secret truth into the truth layer.)
STEP6D-EXIT=1
```

Красный. Разбор — §3.

### 2.5 Шаг 2 панели — `cargo test --workspace`

```
$ cargo test --workspace --quiet
...
running 6 tests
. 1/6
facts_clean_names_orphans_preserves_spec_and_honours_dry_run --- FAILED
....
failures:

---- facts_clean_names_orphans_preserves_spec_and_honours_dry_run stdout ----

thread 'facts_clean_names_orphans_preserves_spec_and_honours_dry_run' (141840) panicked at crates\vibe-cli\tests\cli_facts.rs:153:5:
assertion failed: report.status.success()
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    facts_clean_names_orphans_preserves_spec_and_honours_dry_run

test result: FAILED. 5 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.62s

error: test failed, to rerun pass `-p vibe-cli --test cli_facts`
STEP2-EXIT=101
```

Тест, который назвал пакет, больше не падает — красное теперь другое
(§4.1). Прогон с `--no-fail-fast` показывает, что это не одиночный
случай, а каскад (§4.1).

### 2.6 Панель целиком — хвост 30 строк и код выхода

`bash tools/self-check.sh`:

```
running 3 tests
...
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.56s


running 3 tests
...
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.48s


running 6 tests
. 1/6
facts_clean_names_orphans_preserves_spec_and_honours_dry_run --- FAILED
....
failures:

---- facts_clean_names_orphans_preserves_spec_and_honours_dry_run stdout ----

thread 'facts_clean_names_orphans_preserves_spec_and_honours_dry_run' (64696) panicked at crates\vibe-cli\tests\cli_facts.rs:153:5:
assertion failed: report.status.success()
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    facts_clean_names_orphans_preserves_spec_and_honours_dry_run

test result: FAILED. 5 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.88s

error: test failed, to rerun pass `-p vibe-cli --test cli_facts`
self-check: `cargo test --workspace` failed (exit 101, step 5/47, 287s)
```

```
PANEL-EXIT=101
```

Шаги до падения:

```
self-check: ✓ [1/47] the floor builds every live package workspace (2s)
self-check: ✓ [2/47] instruction files identical (CLAUDE.md = AGENTS.md = GEMINI.md) (0s)
self-check: wire-derive ratchet holds: 178 handwritten derive file(s) match wire-derive-baseline.json.
self-check: ✓ [3/47] wire-derive ratchet (…) (28s)
self-check: ✓ [4/47] cargo fmt --all --check (27s)
=== [5/47] cargo test --workspace ===   ← падение
```

Нумерация: шаг панели `[5/47]` — это инвариант **№2** из шапки
`self-check.sh` (`cargo test --workspace`), то есть один из трёх, а не
посторонний. `cargo fmt --all --check` — зелёный (шаг `[4/47]`).

*Замечание о среде:* первый прогон панели умер не на тесте, а на
`error: failed to remove file target\debug\vibe-index.exe / Access is
denied. (os error 5)` — та же флака общего `target/`, что описал P1-O3:
соседний воркер строит в этом же дереве (§5). Повторный прогон дошёл до
настоящего падения; выше приведён именно он.

---

## 3. Пункт 3 пакета — это перелом провода, а не протухший голден

Пакет: «если это регенерация голденов командой xtask — выполнить её; если
это изменение провода — **остановиться**». Остановился. Ниже — почему
это второе, и почему регенерация не просто не разрешена, а **невозможна**.

### 3.1 Что именно дрейфует

Из двух дрейфующих файлов дрейфует, по существу, **один**:

- `primary.jsonl` (несжатый) в списке **отсутствует** → проекция
  «журнал → каталог» не менялась ни на байт;
- `primary.jsonl.gz` дрейфует → отличается **только сжатое представление
  тех же самых байтов**;
- `repomd.json` дрейфует **следствием**: он записывает `size` и `sha256`
  этого `.gz` (`memory.rs:294`).

Зафиксированный голден внутренне согласован — это не полу-регенерация:

```
gz size on disk   : 1364
gz sha256 on disk : sha256:addd3779a7ee190a8340339c36a32eb5873f0c00154ca586d75113f4b04f4908
gz decompresses to plain identical: True
gz header bytes   : 1f8b08000000000000ff        (mtime=0, без имени файла, OS=ff)
```

и ровно эти `1364` / `addd3779…` лежат в `repomd.json`.

### 3.2 Контролируемый эксперимент — причина

Один и тот же журнал, один и тот же корпус, один и тот же код проектора,
**две разные конфигурации сборки**:

```
$ cargo run -q -p vibe-index --bin vibe-index -- rebuild formats/corpora/index/e1 --check
rebuild --check: the catalog at `formats/corpora/index/e1` is byte-identical to its journal's projection (13 file(s)); no fact lives in the derived artifact (PROP-044 ##FORBID-SECRET-TRUTH).
VIBE-INDEX-REBUILD-EXIT=0
```

```
$ cargo test -p vibe-index --test golden_corpus --quiet
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
SOLO-EXIT=0
```

```
$ cargo test -p vibe-index -p xtask --test golden_corpus --quiet
thread 'the_catalog_is_the_projection_of_its_journal' panicked at crates\vibe-index\tests\golden_corpus.rs:250:5:
the golden corpus no longer reproduces from its journal — either the journal or the projection changed:
  `primary.jsonl.gz` differs (binary): committed 1364 byte(s), projected 1368 byte(s)
  `repomd.json` differs first at line 75:
  committed:       "size": "1364",
  projected:       "size": "1368",
PROBE-EXIT=101
```

Механизм — unification фич Cargo:

```
$ cargo tree -p xtask -e features -i flate2
flate2 v1.1.9
└── zip v4.6.1
    ├── zip feature "_deflate-any"
    │   └── zip feature "deflate-flate2"
    │       └── zip feature "deflate-flate2-zlib-rs"
    │           └── xtask v1.0.0
├── flate2 feature "any_impl"
│   ├── flate2 feature "any_zlib"
│   │   └── flate2 feature "zlib-rs"
│   ├── flate2 feature "miniz_oxide"
│   │   └── flate2 feature "rust_backend"
│   │       └── flate2 feature "default"
│   │           └── vibe-index v1.0.0

$ cargo tree -p vibe-index -e features -i flate2
flate2 v1.1.9
├── flate2 feature "any_impl"
│   ├── flate2 feature "miniz_oxide"
│   │   └── flate2 feature "rust_backend"
│   │           └── vibe-index v1.0.0
```

`crates/vibe-index/Cargo.toml` просит `flate2 = "1"` (бэкенд
`miniz_oxide`). Корневой `Cargo.toml:220` объявляет
`zip = { …, features = ["deflate-flate2-zlib-rs"] }`; `zip` — обычная
зависимость `vibe-cli` и `xtask`. В любой сборке, где `zip` попадает в
граф, `flate2` компилируется **один раз** с объединением фич, включая
`zlib-rs` → `any_zlib`, и `GzEncoder::new(…, Compression::new(6))` внутри
`gzip_deterministic` (`crates/vibe-index/src/index/primary.rs:66`) выдаёт
поток другого deflate-движка: **1368 байт вместо 1364**.

Фича пришла коммитом `40590b5c feat(release): ship verified native
distributions` (`git log -S 'deflate-flate2-zlib-rs' -- Cargo.toml`).
Голден минтился до него, на `miniz_oxide`.

### 3.3 Почему это перелом провода, а не устаревший голден

1. `primary.jsonl.gz` — **публикуемый артефакт провода**: он в
   `WRITER_FILES`, он записан в `repomd.json` размером и хешем, и
   `primary.rs:62` обещает контракт «header `mtime=0`, no filename, level
   6 … Same input, same bytes». Сегодня «same bytes» держится только
   *в пределах одного бэкенда*.
2. **Регенерация голдена математически не решает задачу.** Не существует
   значения байтов корпуса, при котором зелены обе конфигурации: положишь
   1368 — покраснеет `cargo test -p vibe-index` и `vibe-index rebuild
   --check`; оставишь 1364 — краснеет всё, что собирается вместе с `zip`.
   Это не «голден отстал от кода», это «у эмиттера нет одного ответа».
3. **Санкционированной команды регенерации нет.**
   `vibe-index rebuild` умеет только `--check` и отказывается явно:
   «only `--check` exists — repairing the catalog from its journal in
   place is a separate decision; this verb ships the proof only»
   (`crates/vibe-index/src/cli/rebuild.rs:27`). `cargo xtask rebuild` —
   тонкая обёртка над ним. То есть ветка пакета «если это регенерация
   голденов командой xtask — выполнить её» не существует физически.
4. Сам `wire-diff` относит дрейф проекции к безусловно красному классу,
   вне режима эпох: «Drift here is red under every regime — a derived
   artifact holding truth (`##FORBID-SECRET-TRUTH`) is not an epoch
   matter» (`xtask/src/wire_diff.rs`, шапка, вопрос 1). Записка о
   переломе тут ничего не покупает: `formats/EPOCHS.toml` несёт
   `public = false`, и сам сдвиг байтов под `formats/` был бы «зелёным с
   докладом», но шаг падает **раньше** — на вопросе 1, до вердикта.

Поэтому корпус, журнал, `Cargo.toml`, `Cargo.lock` и
`crates/vibe-index/**` мной **не тронуты** (проверено:
`git status --short -- formats/corpora Cargo.toml Cargo.lock
crates/vibe-index/Cargo.toml` пуст).

### 3.4 Варианты для владельца (ни один не в моём мандате)

- **A.** Сменить фичу в корневом `Cargo.toml` на `deflate-flate2`
  (miniz_oxide) вместо `deflate-flate2-zlib-rs` — голден снова
  воспроизводится везде. Цена: решение о зависимости релизной
  дистрибуции из `40590b5c`, плюс `Cargo.lock`.
- **B.** Перемонтировать голден на zlib-rs. **Не работает** — см. §3.3.2:
  покраснеет изолированная сборка `vibe-index`.
- **C.** Сделать `gzip_deterministic` независимым от бэкенда (прибить
  конкретную реализацию deflate внутри `vibe-index`, а не полагаться на
  фичу, которую может включить любой сосед по графу). Это и есть
  настоящая починка контракта «same input, same bytes», и это работа по
  проводу.

Побочно: `cargo xtask check-codegen`/`wire-diff` — не единственные
пострадавшие; тот же корень роняет `crates/vibe-index/tests/golden_corpus.rs`
внутри шага 2 (§4.1).

---

## 4. Что не сделано и почему

### 4.1 Шаг 2 не зелёный — и не мог стать зелёным в периметре пакета

Пакет предполагал в шаге 2 ровно одно красное (`source_path` /
`upstream_authors`). Оно починено. Но под ним лежат ещё два класса, оба
вне названных файлов:

**(а) Протухшая фикстура `schema_version = 6`.** Причина падения
`cli_facts` установлена прямым прогоном CLI на воспроизведённой фикстуре
(изолированный `VIBE_SETTINGS`, реальный `~/.vibe` не тронут):

```
error: reading `…\proj\vibe.lock` as installedness source: unsupported vibe.lock schema version 6 — expected 7 (violates spec://org.vibevm.core/vibevm/VIBEVM-SPEC#lockfile-schema; fix: regenerate with `vibe install`)
```

`CURRENT_SCHEMA_VERSION = 7` (`crates/vibe-core/src/manifest/lockfile.rs:52`)
поднят коммитом `72d866d2 feat(bridge): add reference-backed package
loading`; `crates/vibe-cli/tests/cli_facts.rs:129` до сих пор пишет `6`.
Это **ровно тот же класс дефекта, что пункт 1 пакета** — bridge-кампания
подняла схему и не обновила фикстуры. Правка — один символ. Файл в
пакете не назван, а «Готово, когда …» прямо требует «в дереве нет
изменений вне названных файлов», поэтому не тронут.

Всего в дереве **11** точек `schema_version = 6`:

```
crates/vibe-cli/tests/cli_facts.rs:129
crates/vibe-cli/tests/cli_mcp_path_parity.rs:16
crates/vibe-cli/tests/cli_redirect.rs:213
crates/vibe-core/src/manifest/lockfile/tests.rs:249     ← негативный тест, трогать нельзя
crates/vibe-install/tests/incremental_in_place.rs:136
crates/vibe-mcp/tests/tools_oracle.rs:24
crates/vibe-requirements/src/tests_followup.rs:421
crates/vibe-requirements/src/tests_provider.rs:43
crates/vibe-requirements/src/tests_query.rs:63
crates/vibe-workspace/src/bins/tests.rs:8
crates/vibe-workspace/src/freshness.rs:307
```

`cargo test --workspace --no-fail-fast` подтверждает каскад — падают
цели (перечень оборван на 60 строках захвата, поэтому он **неполный**):

```
-p vibe-cli   --test cli_facts             1 failed
-p vibe-cli   --test cli_mcp_path_parity   1 failed
-p vibe-cli   --test cli_redirect          1 failed
-p vibe-cli   --test cli_search            2 failed
-p vibe-index --test golden_corpus         1 failed
-p vibe-install --test incremental_in_place 1 failed
-p vibe-mcp   --test tools_oracle         10 failed
-p vibe-requirements (tests_followup / tests_provider)  ≥9 failed
…                                          (захват оборван)
```

Полную перепись довести не удалось: повторный прогон `--no-fail-fast`
завис за пределами таймаута (соседний воркер занимал общий `target/`),
и я его снял, чтобы не блокировать дерево.

**(б) Перелом провода** — `golden_corpus` в списке выше, §3. Даже если
починить все `schema_version`, шаг 2 останется красным, пока живёт §3.

Вывод: **шаг 2 внутри периметра пакета недостижим.** Мой мандат по нему
исчерпан пунктом 1.

### 4.2 Голдены не регенерированы

Команды регенерации не существует (§3.3.3), а ручная правка корпуса —
именно то «laundering», которое запрещает сообщение самого гейта.

### 4.3 Чужие файлы не тронуты

`crates/vibe-specdoc/tests/docs_corpus.rs` (§2.3),
`crates/vibe-cli/tests/cli_facts.rs` (§4.1а) и всё остальное из §5 —
не мои; правок не вносил.

### 4.4 Ничего не закоммичено

Коммитит центральная сессия. `git` использовался только как
`status --short`, `diff`, `rev-parse`, `log`.

---

## 5. Замечание о среде — дерево общее и живое

Дерево делится с другими активными воркерами; это влияет на чтение любых
измерений выше.

- HEAD сдвинулся во время прогона: `bb6320b1` → `d3f578ae` → `b18453f2`.
- На входе `git status --short` показывал только документы кампании; на
  выходе — три десятка файлов в `vibe-specdoc`, `vibe-facts`,
  `vibe-cli`, `vibe-core`, `vibe-index`, `vibe-install`, `schemas/`,
  часть уже **в индексе** (`M `), часть новая (`??`).
- Отсюда флака `Access is denied (os error 5)` в первом прогоне панели и
  повисший `--no-fail-fast`.

Мои выводы к этому устойчивы: ключевые для них файлы —
`crates/vibe-cli/tests/cli_facts.rs`, `crates/vibe-index/tests/golden_corpus.rs`,
`formats/corpora/**`, `crates/vibe-index/Cargo.toml`, корневой `Cargo.toml`,
`Cargo.lock` — **никем не изменены** (`git status --short` по ним пуст).
А вот красное clippy в §2.3 к унаследованным не относится: его источник —
неотслеживаемый файл соседа.

---

## 6. `git status --short` в конце

Мои файлы (ровно четыре):

```
 M crates/vibe-cli/src/commands/show/source_path.rs
 M crates/vibe-install/src/plan/fetch.rs
 M xtask/src/bridge.rs
?? campaigns/docs-2026-09/findings/WORKER-REPORT-P1-O4.md
```

```
 crates/vibe-cli/src/commands/show/source_path.rs |  2 +-
 crates/vibe-install/src/plan/fetch.rs            | 49 ++++++------
 xtask/src/bridge.rs                              | 96 ++++++++++--------------
 3 files changed, 63 insertions(+), 84 deletions(-)
```

`cargo fmt --all --check` на момент сдачи:

```
FMT-EXIT=0
```

Полный вывод `git status --short` на момент сдачи (всё, кроме четырёх
строк выше, — чужая живая работа, §5; набор чужих файлов успел смениться
между §5 и этим снимком — сосед закоммитил `vibe-specdoc` и перешёл к
словарям и `index_cli`):

```
 M crates/vibe-cli/resources/package-tree.schema.v1.json
 M crates/vibe-cli/src/cli.rs
 M crates/vibe-cli/src/cli/pkg.rs
 M crates/vibe-cli/src/commands/init/prompts.rs
 M crates/vibe-cli/src/commands/show/source_path.rs          ← мой
 M crates/vibe-core/src/error.rs
 M crates/vibe-index/src/cli/list.rs
 M crates/vibe-index/src/cli/search.rs
 M crates/vibe-index/src/scanner/manifest.rs
 M crates/vibe-install/src/plan/fetch.rs                     ← мой
 M crates/vibe-mcp/src/skill_template.md
 M crates/vibe-wire/src/behaviour/vocabularies.rs
 M crates/vibe-wire/src/generated/index_cli/e1/list_report/mod.rs
 M crates/vibe-wire/src/generated/shared/mod.rs
 M crates/vibe-wire/tests/open_vocabulary.rs
 M formats/vocabularies.json
 M schemas/index_cli/e1/list_report.jtd.json
 M xtask/src/batch_review/refs.rs
 M xtask/src/bridge.rs                                       ← мой
 M xtask/src/codegen/vocabulary/tests.rs
?? campaigns/docs-2026-09/findings/WORKER-REPORT-P1-O4.md    ← мой
```

Соседние правки под `formats/` и `schemas/` появились уже после моих
измерений §2.4; на разбор §3 они не влияют (дрейф `wire-diff` падает на
вопросе 1 — проекция журнала, — до вопроса о сдвиге байтов), но повторный
прогон `wire-diff` теперь будет ещё и докладывать их сдвиг.
