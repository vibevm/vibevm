# WORKER-REPORT-P1-O6 — хвост панели: `cli_search`, литералы `7`, прогон по шагам

Пакет: `campaigns/docs-2026-09/findings/PACKET-P1-O6.md`.
HEAD на входе: `699e337bdc95fe7ed57d07dc1ec08d330075515c`.
HEAD на выходе: `aa8d196b` (два моих коммита поверх входного).

**Итог одной строкой.** Оба теста `cli_search` зелены по существу: причина
не в локе и не в конверте, а в том, что лестница поиска индекса отрастила
третью ступень, и «убрать переменную окружения» перестало значить «у этого
реестра нет индекса» — фикстуры ходили в живой интернет. Шестнадцать
фикстур с литералом `7` переведены на `CURRENT_SCHEMA_VERSION`. Панель
прогнана целиком, когда дерево на полчаса стало чистым, и **останавливается
на шаге 3 из 49**: рэтчет рукописных wire-derive красен с коммита
`29351a68`, который старше входного HEAD пакета. Это не периметр тестов и
clippy — записано и остановлено (§3.2). Остальные шаги прогнаны по
отдельности их же командами из скрипта; коды выхода и дословные хвосты — §3.

---

## 1. Атом 1 — `cli_search` ×2

Коммит `8df28576` `test(cli): the search fixtures name the no-index rung
instead of assuming it`, один файл `crates/vibe-cli/tests/cli_search.rs`
(+32 / −4).

### 1.1 Воспроизведение на входе

Прогон в приватном `CARGO_TARGET_DIR` (почему приватный — §5):

```
running 15 tests
test search_full_scan_finds_matching_packages_in_github_org ... FAILED
test search_without_full_scan_keeps_unconfigured_status ... FAILED

---- search_full_scan_finds_matching_packages_in_github_org stdout ----
thread 'search_full_scan_finds_matching_packages_in_github_org' (146548) panicked at crates\vibe-cli\tests\cli_search.rs:682:59:
called `Option::unwrap()` on a `None` value

---- search_without_full_scan_keeps_unconfigured_status stdout ----
thread 'search_without_full_scan_keeps_unconfigured_status' (77804) panicked at crates\vibe-cli\tests\cli_search.rs:760:5:
assertion `left == right` failed
  left: 0
 right: 1

test result: FAILED. 13 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 29.00s
TEST-EXIT=101
```

Строка 682 — `v["registries_full_scanned"].as_array().unwrap()`;
строка 760 — `assert_eq!(unconfigured.len(), 1)`.

### 1.2 Причина — не поля конверта, не реестр `local`, не лок. Лестница

Пакет предлагал три гипотезы (поля конверта `search`, реестр `local` в
фикстуре, сеть). Верна третья, и в неё же схлопываются первые две: поля
конверта не менялись, никакого `local` в фикстуре нет, а **`vibe search`
ходит в живой интернет**.

Дословное измерение, проект с той же `vibe.toml`, что пишет фикстура,
`VIBEVM_INDEX_URL_VIBESPECS` снят:

```
$ vibe --json search wal --path <tmp>
{
  "ok": true,
  "command": "search",
  "query": "wal",
  "registries_searched": [],
  "registries_unconfigured": [],
  "registries_unreachable": [
    {
      "name": "vibespecs",
      "reason": "index at `https://raw.githubusercontent.com/vibespecs/index/main/v1/packages` returned status 404 (violates spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#http; fix: check the index server health at that URL)"
    }
  ],
  "hit_count": 0,
  "hits": []
}
PROBE-EXIT=0
```

Механизм целиком:

1. `crates/vibe-registry/src/index_client/locate.rs:97` —
   `resolve_index_url_with`: лестница `env` → ключ `[[registry]].index_url`
   → **умолчание**. Контракт — `spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#form-factor`
   (B-083).
2. Ступень умолчания перестала быть безобидной. `github_raw_index_url`
   (`locate.rs:117`) переписывает канонический публичный
   `https://github.com/<org>` в **живой**
   `https://raw.githubusercontent.com/<org>/index/<ref>`. Пришло коммитом
   `764cf517 fix(index): resolve GitHub defaults to raw files` (11.09.2026)
   — на сутки раньше этого пакета.
3. Организация `vibespecs` такой репозиторий публикует. `IndexClient::probe`
   отвечает `Found`, `served_by_index` становится `true`
   (`crates/vibe-cli/src/commands/search.rs:189`), и дальше:
   - реестр **не попадает** в `registries_unconfigured` → тест 2 видит `0`
     вместо `1`;
   - `--full-scan` **не запускается** (`if !served_by_index`) →
     `registries_full_scanned` пуст → поле выбрасывается
     `skip_serializing_if = "Vec::is_empty"` → `.as_array()` даёт `None` →
     паника `unwrap` в тесте 1.

Оба падения — одно следствие. Поиск потом всё равно упирается в 404 на
`/v1/packages` (статическое зеркало не держит серверных маршрутов), но это
уже после того, как реестр посчитан «обслуженным индексом».

### 1.3 Чинить тест, а не продукт

Устарел **тест**. Он датируется временем, когда у лестницы было две
ступени и снятая переменная окружения падала насквозь. Сегодня ступеней
три, последняя **угадывает**, и «нет индекса» пишется не отсутствием
переменной, а явным `none` — контрактным выключателем, который
`explicit_step` (`locate.rs:179`) обрабатывает до всякого HTTP.

Вдобавок тест нарушал собственное обещание файла — шапка
`cli_search.rs:1` говорит «without any live internet». Три теста из
пятнадцати ходили в сеть: два падали, третий
(`search_text_hint_directs_agent_to_install_when_no_index_configured`)
был зелёным по совпадению — он проверяет только текст подсказки, который
печатается при пустом `registries_searched` в обеих ветках.

Продукт не тронут: ни `search.rs`, ни `locate.rs`, ни `index_client/**`.

### 1.4 Что сделано

Обе фикстуры-манифеста получили `index_url = "none"` через
именованную константу `NO_INDEX`, у которой в доккомменте записано, почему
она есть. `write_gitverse_only_manifest` — тоже: тот тест про отказ
`--full-scan` не-GitHub хосту, а не про лестницу, и он ради этого ходил на
`gitverse.ru`. Теперь ни один тест файла не делает сетевого запроса мимо
своих моков, и прогон стал короче (27.93 s против 29.00 s при том, что
раньше две цели падали рано).

```
running 15 tests
test search_purl_rejects_non_pkg_scheme ... ok
test search_full_scan_unsupported_for_non_github_host ... ok
test search_without_full_scan_keeps_unconfigured_status ... ok
test search_purl_and_query_are_mutually_exclusive ... ok
test search_full_scan_finds_matching_packages_in_github_org ... ok
test search_errors_when_registry_name_unknown ... ok
test search_filters_to_one_registry_via_flag ... ok
test search_text_hint_directs_agent_to_install_when_no_index_configured ... ok
test search_dedup_keeps_highest_score_across_registries ... ok
test search_reports_unreachable_registry_without_aborting ... ok
test search_purl_lookup_returns_binding_site_and_dedups_across_registries ... ok
test search_aggregates_hits_from_configured_registries ... ok
test search_kind_flag_is_propagated_to_index ... ok
test search_cache_ttl_zero_forces_refetch ... ok
test search_caches_results_and_serves_subsequent_runs_from_disk ... ok

test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 27.93s
TEST-EXIT=0
```

### 1.5 Что этот разбор оставляет владельцу

Две вещи, которые вне мандата пакета, но их стоит знать.

- **Найденный индекс без серверных маршрутов уходит в `unreachable`.**
  Статическое raw-зеркало отвечает на `hello.json`/`repomd.json`, но не на
  `/v1/packages`, и реестр получает статус «недостижим» вместо отката на
  индексless-путь. Тесты этого не утверждают ни в одну сторону; решать —
  владельцу. Предмет: `crates/vibe-cli/src/commands/search.rs:251-255`.
- **`vibe search` без явного `index_url` делает сетевой запрос к
  `raw.githubusercontent.com`** на всяком проекте с github-реестром. Это
  проектное решение `764cf517`, а не дефект, но для тестов оно означает:
  всякая новая фикстура с `https://github.com/<org>` обязана называть
  `index_url`, иначе её цвет решает чужой репозиторий.

---

## 2. Атом 2 — шестнадцать фикстур с литералом `7`

Коммит `aa8d196b` `test: take the lock schema version from the constant in
the remaining fixtures`, пять файлов (+90 / −51).

Список точек взят из `WORKER-REPORT-P1-O5.md` §7 и перепроверен заново:
на входе в дереве было ровно шестнадцать `schema_version = 7` в `.rs`.
Ни одна из них **не** тестирует чужую схему намеренно — все шестнадцать
пишут «текущую схему, записанную числом», и предмет каждого теста —
что-то другое (отсутствующий слот `vibedeps`, дрейф локального источника,
неизвестное поле пакета, проекция навыков). Поэтому переведены все.

| Файл | Точек | Форма правки |
|------|-------|--------------|
| `crates/vibe-core/src/manifest/lockfile/tests.rs` | 8 | `const FIXTURE: &str` → `fn fixture() -> String`; семь `let raw = r#"…"#` → `format!` |
| `crates/vibe-check/src/checks/lockfile_files.rs` | 4 | четыре литерала → `format!`, импорт константы в `mod tests` |
| `crates/vibe-check/src/checks/local_source_freshness.rs` | 2 | обе точки уже жили внутри `format!` — только подстановка + импорт |
| `crates/vibe-cli/src/commands/show/source_path.rs` | 1 | подстановка в существующий `format!` + импорт |
| `crates/vibe-agent-projection/src/pkgskill/projection/tests.rs` | 1 | литерал в `fs::write` → `format!` + импорт |

Идиома не новая: `cli_facts.rs`, `cli_mcp_path_parity.rs`, `cli_redirect.rs`,
`tree_fixture.rs` и `visibility/query/tests.rs` уже берут номер из
константы. Правки повторяют её.

**`FIXTURE` — три потребителя**, поэтому не `LazyLock`, а функция рядом с
соседним хелпером `fn org() -> Group` в том же файле: `&str`-константа не
умеет звать `format!`, а вводить `LazyLock` в `vibe-core`, где его сегодня
нет ни одного, — дороже, чем три `&fixture()`.

**Одна парная правка сверх шестнадцати.** `parses_fully`
(`lockfile/tests.rs`) утверждал версию своей же фикстуры литералом:
`assert_eq!(lf.meta.schema_version, 7)`. Оставить его при переведённой
фикстуре значило бы **сломать тест на следующем подъёме** — ровно то, что
пакет чинит. Утверждение переведено вместе с фикстурой, которую читает.

### 2.1 Что осталось литералом и почему

Три точки, ни одна не в списке шестнадцати:

| Точка | Почему оставлена |
|-------|------------------|
| `lockfile/tests.rs:233` `assert_eq!(CURRENT_SCHEMA_VERSION, 7)` | **Намеренный пин.** Он и делает подъём схемы решением, а не случайностью: после него всякий подъём обязан пройти через этот тест. |
| `lockfile/tests.rs:265` `schema_version = 6` в `read_rejects_non_current_version` | Намеренный негативный тест чужой схемы — оставлен решением P1-O5 §2, предмет теста отказ, а не приём. |
| `lockfile/tests.rs:250` `assert_eq!(lf.meta.schema_version, 7)` в `read_accepts_current_version` | Вне перечисленных пакетом шестнадцати: это утверждение, а не фикстура, и своей фикстуры у теста нет (`Lockfile::empty` пишет константу). Моя правка его не ухудшает. Кандидат следующей волны. |

### 2.2 Семнадцатая точка, которую пакет не назвал

```
crates/vibe-core/src/manifest/lockfile.rs:98:///     schema_version = 7
```

Это **доктест** `LockfileMeta`, и он сломается на следующем подъёме ровно
как шестнадцать фикстур: TOML говорит `7`, следующая строка утверждает
`assert_eq!(m.schema_version, CURRENT_SCHEMA_VERSION)`. Не тронут — файл
продуктовый и вне названных пакетом мест. Починка на одну правку:
обернуть тело в `toml::from_str(&format!(r#"…schema_version = {CURRENT_SCHEMA_VERSION}…"#))`.

### 2.3 Гейт правки

Приватный `CARGO_TARGET_DIR`, четыре затронутых пакета:

```
$ cargo test --no-fail-fast -p vibe-core -p vibe-check -p vibe-agent-projection --lib
running 71 tests
test result: ok. 71 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.47s
running 99 tests
test result: ok. 99 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
running 491 tests
test result: ok. 491 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.90s

$ cargo test --no-fail-fast -p vibe-cli --bins
running 726 tests
test commands::show::source_path::tests::cli_accepts_source_path_with_embedded_and_project_path ... ok
test commands::show::source_path::tests::embedded_root_uses_exact_lock_row_and_threads_offline ... ok
test commands::show::source_path::tests::package_root_is_the_canonical_normal_vibedeps_slot ... ok
test commands::show::source_path::tests::short_package_name_must_be_unambiguous ... ok
test result: ok. 726 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 22.50s
CLI-BINS-EXIT=0
```

Поимённо зелены и сами переведённые фикстуры:

```
test checks::lockfile_files::tests::lockfile_files_missing_slot_is_an_error ... ok
test checks::lockfile_files::tests::lockfile_files_orphan_vibedeps_slot_warns ... ok
test checks::lockfile_files::tests::embedded_source_kind_entry_warns_as_non_portable ... ok
test checks::lockfile_files::tests::local_source_kind_entry_is_portable_no_warn ... ok
test checks::local_source_freshness::tests::matching_local_source_emits_no_finding ... ok
test checks::local_source_freshness::tests::drifted_local_source_warns_once ... ok
test checks::local_source_freshness::tests::missing_local_source_dir_warns ... ok
test checks::local_source_freshness::tests::non_local_source_kind_is_skipped ... ok
test pkgskill::projection::tests::inventory_preserves_root_member_and_lock_order_while_skipping_malformed_slots ... ok
```

Проверка, что литералов больше не осталось:

```
$ grep -rn "schema_version = 7" --include=*.rs crates/ xtask/
crates/vibe-core/src/manifest/lockfile.rs:98:///     schema_version = 7
```

(единственное вхождение — доктест из §2.2).

---


## 3. Панель

### 3.1 Две среды за одну сессию

Предусловие пункта 3 — «`git status --short`: только твои файлы» — почти всю
сессию **не выполнялось**: рядом работал воркер P2-O7 (`crates/vibe-doc`,
`crates/vibe-wire`, `schemas/`, `formats/`). Дважды дерево под его
незавершённой правкой не собиралось вовсе:

```
error[E0599]: the method `as_dyn_error` exists for reference `&std::string::String`, but its trait bounds were not satisfied
   --> crates\vibe-doc\src\error.rs:172:19
    |
172 |     Translation { source: String, message: String },
    |                   ^^^^^^ method cannot be called on `&std::string::String` due to unsatisfied trait bounds
error: could not compile `vibe-doc` (lib) due to 1 previous error
STEP2-TEST-EXIT=101
```

Поэтому шаги шли по отдельности (§3.3). В `04:58Z` P2-O7 закоммитил четыре
атома (`55dfd646`, `27de558e`, `ff71092a`, `5ad8c7a8`), и дерево на
полчаса стало чистым — **только мой неотслеживаемый отчёт**. В это окно
панель прогнана целиком (§3.2) и повторены шаги, которые до того мерили
чужую незакоммиченную работу (§3.4). К моменту сдачи дерево снова занято:
идёт следующая работа (перенос `doc_media` в `vibe-doc/media`, правка
страниц документации, новый пакет `org.vibevm.doc/web`), HEAD `3e0efc08`.

### 3.2 Полная панель — останавливается на шаге 3 из 49

```
$ bash tools/self-check.sh

=== [1/49 08:03:07] the floor builds every live package workspace ===
self-check: the floor builds all 7 live package workspace(s) under vibevm/vibepacks/org.vibevm.ai-native/.
self-check: ✓ [1/49] the floor builds every live package workspace (2s)

=== [2/49 08:03:09] instruction files identical (CLAUDE.md = AGENTS.md = GEMINI.md) ===
self-check: ✓ [2/49] instruction files identical (CLAUDE.md = AGENTS.md = GEMINI.md) (1s)

=== [3/49 08:03:10] wire-derive ratchet (handwritten Serialize/Deserialize derives vs wire-derive-baseline.json) ===
self-check: `vibe-core` carries 35 handwritten Serialize/Deserialize derive file(s);
self-check: wire-derive-baseline.json froze 34. the rule — handwritten wire grows only through a
self-check: named decision, never silently (PROP-044 §2 law 5: a handwritten
self-check: parser or writer of our own format is the mechanism by which the
self-check: other four bans break unnoticed). fix: describe the format as a schema
self-check: and run `cargo xtask codegen`; if the new type is NOT our wire (a
self-check: config, a CLI-local struct, a foreign format), raise the `vibe-core` count
self-check: in wire-derive-baseline.json in the same commit and say in the commit body what the type is.
self-check: `wire-derive ratchet (handwritten Serialize/Deserialize derives vs wire-derive-baseline.json)` failed (exit 1, step 3/49, 17s)
PANEL-EXIT=1
```

**Причина названа точно.** Рэтчет считает файлы вне `**/generated/**`, в
которых есть рукописный `#[derive(… Serialize | Deserialize …)]`. Измерение
по ревизиям:

```
$ git grep -lE '^[[:space:]]*#\[derive\([^)]*(Serialize|Deserialize)' 29351a68^ -- 'crates/vibe-core/**/*.rs' | grep -v /generated/ | wc -l
34
$ git grep -lE '^[[:space:]]*#\[derive\([^)]*(Serialize|Deserialize)' 29351a68  -- 'crates/vibe-core/**/*.rs' | grep -v /generated/ | wc -l
35
```

Тридцать пятым файлом стал `crates/vibe-core/src/manifest/package/documentation.rs`,
добавленный коммитом

```
29351a68 2026-09-12 04:44:44 +0300 feat(core): carry documentation relations, locales and the card in the manifest
```

без поднятия `"vibe-core": 34` в `wire-derive-baseline.json`. Коммит
**старше входного HEAD пакета**:

```
$ git merge-base --is-ancestor 29351a68 699e337b && echo "YES - predates the packet input HEAD"
YES - predates the packet input HEAD
```

То есть панель мертва на шаге 3 с того момента и ни разу за мою сессию не
могла дойти до шага 4. Мои коммиты к этому отношения не имеют: в
`crates/vibe-core` они трогают ровно `manifest/lockfile/tests.rs`, файл,
которого в списке тридцати пяти нет.

**Чинить не стал.** Рэтчет — не тесты и не clippy, а именно та развилка,
ради которой он и существует: «опиши формат схемой и прогони
`cargo xtask codegen`» **или** «подними счётчик в том же коммите и скажи в
теле коммита, что это за тип». Это названное решение автора PROP-057, а не
работа воркера. По пакету — записано, остановился.

Цена остановки, которую шапка `self-check.sh` называет отдельно: шаги 4–49
**не выполнялись**. Всё, что ниже, — отдельные прогоны, а не панель.

### 3.3 Первый проход по шагам — грязное дерево

Нумерация — шапки `self-check.sh`.

| Шаг | Команда | Код | Мой? |
|---|---|---|---|
| 1 | `cargo fmt --all --check` | `0` | зелено |
| 2 | `cargo test --workspace --quiet` | `101` | единственная красная цель — `-p vibe-cli --test vvm`, артефакт приватного `CARGO_TARGET_DIR` (§5) |
| 3 | `cargo clippy --workspace --all-targets --quiet -- -D warnings` | `0` | зелено |
| 4 | `cargo run --quiet -p vibe-cli -- check --path . --quiet` | `0` | зелено (2 warning) |
| 5 | `cargo xtask conform check` | `1` | **нет** — §3.5 |
| 6a | `cargo xtask sync-engines --check` | `0` | зелено |
| 6b | `cargo xtask check-codegen` | `1` | **нет** — незакоммиченная кодогенерация P2-O7 |
| 6c | `cargo xtask specmap --check` | `1` | **нет** — 7 рёбер из новых файлов P2-O7 |
| 6d | `cargo xtask wire-diff` | `1` | **нет** — `index/e1`, §3.6 |

Дословно:

```
$ cargo fmt --all --check
STEP1-FMT-EXIT=0

$ cargo clippy --workspace --all-targets --quiet -- -D warnings
START 04:15:52Z
STEP3-CLIPPY-EXIT=0
END 04:17:59Z

$ cargo run --quiet -p vibe-cli -- check --path . --quiet
vibe check: 0 errors, 2 warnings, 0 info
STEP4-VIBECHECK-EXIT=0

$ cargo xtask sync-engines --check
sync-engines --check: every vendored crate matches its authored source (51 pair(s) across 9 sync set(s)); 6 vendored engine dir(s) under vibevm/vibepacks/org.vibevm.ai-native/ accounted for (6 sync targets, 0 recorded frozen slots).
STEP6a-EXIT=0
```

Оба warning шага 4 — давние и не мои:

```
vibe check: 2 findings in `C:\Users\olegc\git\v\vibevm-docs`
  [W]  [wal_wellformed] vibevm/vibespecs/WAL.xml — WAL is missing the canonical `## constraints` section
  [W]  [wal_wellformed] vibevm/vibespecs/WAL.xml — WAL is missing the canonical `## done` section
```

Шаг 2 первого прохода (приватный каталог, без `--no-fail-fast`) остановился
на первой же красной цели:

```
running 4 tests
ls_on_a_fresh_root_still_identifies_the_direct_source_execution --- FAILED
which_reports_the_direct_source_executable_without_an_active_version --- FAILED
..
command=`"C:\\Users\\olegc\\AppData\\Local\\Temp\\p1o6-target\\debug\\vibe.exe" "self" "ls"`
stdout="(no versions installed — run `vibe self install`)\n"

test result: FAILED. 2 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.74s
error: test failed, to rerun pass `-p vibe-cli --test vvm`
STEP2-TEST-EXIT=101
```

Ровно то, что P1-O5 §4.4 назвал ценой обхода: признак «прямой запуск из
исходников» выводится из того, что бинарь лежит в `target/` самого
репозитория. Прогон остановился здесь и про цели после `vibe-cli`,
включая `golden_corpus`, не говорит ничего.

### 3.4 Второй проход — чистое дерево, весь workspace

`cargo test --workspace --no-fail-fast --quiet` в **общем** `target/` (тот
самый, которым пользуется панель, поэтому `vvm` зелен):

```
START 05:10:44Z
STEP2-SHARED-EXIT=101
END 06:14:48Z
```

289 целей, 281 зелёная, **девять красных**:

```
error: 9 targets failed:
    `-p vibe-index --test golden_corpus`
    `-p vibe-install --test native_slot_lifecycle`
    `-p vibe-native-loader --test real_compile_fixture`
    `-p vibe-native-loader --test real_fixture`
    `-p vibe-native-loader --test real_mechanism_fixture`
    `-p vibe-orchestrator --lib`
    `-p vibe-specdoc --test docs_corpus`
    `-p vibe-check --doc`
    `-p vibe-doc --doc`
```

Разбор по причинам — их три, и ни одна не моя.

**(а) `index/e1`, одна цель.** Тот же перелом провода, что и в шаге 6d:

```
thread 'the_catalog_is_the_projection_of_its_journal' panicked at crates\vibe-index\tests\golden_corpus.rs:257:5:
the golden corpus no longer reproduces from its journal — either the journal or the projection changed:
  `primary.jsonl.gz` differs (binary): committed 1879 byte(s), projected 1891 byte(s)
  `repomd.json` differs first at line 90:
  committed:       "size": "1879",
  projected:       "size": "1891",
```

Корпус с момента P1-O4 вырос (было 1364 / 1368, стало 1879 / 1891), но
дельта та же природа: `flate2` собирается с объединением фич, `zip` тянет
`deflate-flate2-zlib-rs`, и `gzip_deterministic` отдаёт поток другого
deflate-движка. Разбор — `WORKER-REPORT-P1-O4.md` §3 (J-069).

**(б) «ровно одна копия фикстурной DLL», пять целей.**
`vibe-install --test native_slot_lifecycle`, три цели `vibe-native-loader`
и `vibe-orchestrator --lib` падают на одном и том же утверждении:

```
thread 'real_compiler_fixture_loads_raw_statuses_and_recovers_after_panic' panicked at crates\vibe-native-loader\tests\real_compile_fixture.rs:159:5:
assertion `left == right` failed: expected exactly one `vibe_native_loader_compiler_fixture.dll` compiler fixture artifact in `…\target\debug` and its deps directory; found ["…\\target\\debug\\deps\\vibe_native_loader_compiler_fixture.dll", "…\\target\\debug\\vibe_native_loader_compiler_fixture.dll"]
  left: 2
 right: 1
```

Найденные два пути — **одна и та же inode**: cargo поднимает cdylib из
`deps/` в `debug/` жёсткой ссылкой.

```
$ ls -la target/debug/vibe_native_loader_fixture.dll target/debug/deps/vibe_native_loader_fixture.dll
-rwxr-xr-x 2 olegc 197121 808960 Sep 12 08:05 target/debug/deps/vibe_native_loader_fixture.dll
-rwxr-xr-x 2 olegc 197121 808960 Sep 12 08:05 target/debug/vibe_native_loader_fixture.dll
```

(`2` в колонке — счётчик ссылок.) То есть эти пять целей утверждают что-то
про **содержимое каталога сборки**, а не про код, и краснеют на всяком
`target/`, где cargo успел сделать свой обычный подъём. В приватном
свежем каталоге P1-O5 та же `native_slot_lifecycle` была зелёной — значит
цвет решает история каталога, а не дерево. Кандидат в отдельный пакет:
либо тест должен схлопывать жёсткие ссылки, либо искать в одном месте.

**(в) Живой рефакторинг `vibe-doc`, три цели.** `vibe-check --doc` и
`vibe-doc --doc`:

```
error[E0433]: failed to resolve: use of unresolved module or unlinked crate `vibe_doc`
  --> crates\vibe-check\src\checks\doc_media.rs:26:5
   |
26 | use vibe_doc::media::{self, Severity};
error: doctest failed, to rerun pass `-p vibe-check --doc`
```

Это ровно тот перенос `doc_media` → `vibe-doc/src/media`, который в дереве
идёт прямо сейчас (§3.1). `vibe-specdoc --test docs_corpus` — из той же
работы:

```
assertion `left == right` failed: the documentation package carries 44 pages; a page added or removed is a deliberate edit, so update this count with it
  left: 45
 right: 44
```

Страница добавлена, счётчик в тесте не поднят — «намеренная правка»,
которую этот тест и обязан ловить.

Повтор шага 6c на чистом дереве:

```
$ cargo xtask specmap --check
Error: `C:\Users\olegc\git\v\vibevm-docs\specmap.json` is out of date relative to the tree.
  drift: edges added: 48
Run `rust-ai-native-specmap` (or your project's wrapper), review the drift, and commit the result.
STEP6c-CLEAN-EXIT=1
```

Сорок восемь рёбер вместо семи: `specmap.json` теперь закоммичен, но
устарел относительно закоммиченного дерева. Мои два коммита не добавляют и
не убирают ни одного ребра:

```
$ git diff 699e337b..aa8d196b -- . | grep -E "^[+-].*(specmark::scope!|#\[spec\(|#\[verifies\()"
(пусто)
```

Повтор шага 6b на чистом дереве **измерить не удалось** — §3.7.

### 3.5 Шаг 5 `conform check` — 27 новых находок, ни одна не моя

```
conform: policy conform.toml (loaded).
conform: extracted 16 file(s), 2158 cached (producer rust-syn-11).
  conform: NEW error-enum-cites-req crates/vibe-registry/src/embedded_source.rs:42 …
  conform: NEW error-message-cites-req crates/vibe-cli/src/commands/vvm/error.rs:94 …
  conform: NEW file-length crates/progress-core/src/model.rs:1 — 601 lines exceeds the 600-line file budget
  conform: NEW file-length crates/vibe-agent-projection/src/pkgskill.rs:1 — 958 lines …
  conform: NEW no-unwrap-in-domain crates/vibe-cli/src/commands/vvm/placer.rs:297 …
  conform: NEW pub-doctest crates/vibe-core/src/manifest/lockfile.rs:234 …
conform check: 85 finding(s) in scope <workspace> ({"ambient-env": 5, "error-enum-cites-req": 1, "error-message-cites-req": 2, "file-length": 17, "no-unwrap-in-domain": 43, "pub-doctest": 5, "unsafe-gate": 12}), 0 frozen in baseline, 27 new; SARIF at target\conform\report.sarif.
conform: 26 crate(s) gated, 7 exempt — see conform.toml for the why of each.
Error: conform: 27 new finding(s) against the baseline
STEP5-EXIT=1
```

Ни одного моего файла в отчёте — проверено по SARIF:

```
$ grep -c cli_search.rs target/conform/report.sarif              → 0
$ grep -c lockfile/tests.rs target/conform/report.sarif          → 0
$ grep -c lockfile_files.rs target/conform/report.sarif          → 0
$ grep -c local_source_freshness.rs target/conform/report.sarif  → 0
$ grep -c projection/tests.rs target/conform/report.sarif        → 0
$ grep -c show/source_path.rs target/conform/report.sarif        → 0
```

Находки раскиданы по `progress-core`, `vibe-registry`, `vibe-index`,
`vibe-orchestrator`, `vibe-specdoc`, `vibe-cli/commands/vvm` и `xtask` —
накопленный дрейф, а не «шаг сломался сегодня». Разбор — отдельный пакет:
часть требует разрезания файлов.

Оговорка: `cargo xtask conform check` пишет SARIF в **общий**
`target/conform/`, не в `CARGO_TARGET_DIR`. Каталог derived и в
`.gitignore`; отслеживаемое дерево не тронуто.

### 3.6 Шаг 6d `wire-diff`

```
rebuild: differs `primary.jsonl.gz`
rebuild: differs `repomd.json`
Error: rebuild --check: 2 drift item(s) against the journal under `…\formats/corpora/index/e1`. A catalog that differs from its journal's projection carries a fact the journal does not describe — a derived artifact holding truth (violates spec://org.vibevm.core/vibevm/common/PROP-044#truth; fix: regenerate the catalog FROM the journal …)
STEP6d-EXIT=1
```

Пакет вывел корпус за периметр; не тронут. Регенерация не просто запрещена
— **невозможна**: нет значения байтов, при котором зелены обе конфигурации
сборки, и санкционированной команды регенерации не существует
(`vibe-index rebuild` умеет только `--check`). Три варианта для владельца —
`WORKER-REPORT-P1-O4.md` §3.4.

### 3.7 Две операционные находки, которые стоили времени

**`cargo xtask check-codegen` — не проверка, а перегенерация с подменой
каталога.** Запущенный параллельно со сборкой, он на Windows падает на
файловой блокировке:

```
Error: publishing complete generated tree …\crates/vibe-wire/src/generated
Caused by:
    installing fresh generated tree …\crates/vibe-wire/src\generated.new-42192-… at …\crates/vibe-wire/src/generated failed: Access is denied. (os error 5); restored the complete old tree
STEP6b-CLEAN-EXIT=1
```

Отслеживаемое дерево он при этом восстановил корректно (`git status` без
единой `M`), но оставил два неотслеживаемых каталога
`generated.new-<pid>-<ns>/` — свои и соседний. Свои я удалил. Вывод для
следующих сессий: **этот шаг нельзя запускать одновременно со сборкой**, а
после его падения надо проверять дерево на `generated.new-*`.

**Два приватных `CARGO_TARGET_DIR` этого воркспейса стоят 84 ГБ и
переполняют диск.** Два прогона в общем `target/` умерли не на тестах:

```
LINK : fatal error LNK1180: insufficient disk space to complete link
LINK : fatal error LNK1318: Unexpected PDB error; LIMIT (12) …
```

На тот момент свободного места на `C:` оставалось 815 104 байта. После
удаления моих `p1o6-target` (62.9 ГБ) и `p1o6-clippy` (20.8 ГБ)
освободилось 84 ГБ, и прогон §3.4 прошёл. Общий `target/` этого
репозитория — **222.6 ГБ**; каждый приватный каталог обхода добавляет
десятки сверху. Обход «приватный target против гонки с соседом» не
бесплатный, и на этой машине он упирается в диск раньше, чем во время.

---

## 4. Свод: что осталось красным

| Где | Красное | Причина | Мой мандат? |
|-----|---------|---------|-------------|
| панель, шаг 3/49 | wire-derive ratchet, `vibe-core` 35 против 34 | `29351a68` добавил `manifest/package/documentation.rs`, не подняв `wire-derive-baseline.json`; коммит старше входного HEAD | нет — названное решение автора, не тесты и не clippy |
| шаг 2, 1 цель | `vibe-index --test golden_corpus` | перелом провода J-069 | выведен пакетом за периметр |
| шаг 2, 5 целей | `vibe-install`, ×3 `vibe-native-loader`, `vibe-orchestrator --lib` | утверждение «ровно одна копия фикстурной DLL» против жёсткой ссылки cargo | нет — свойство каталога сборки, кандидат в отдельный пакет |
| шаг 2, 3 цели | `vibe-specdoc --test docs_corpus`, `vibe-check --doc`, `vibe-doc --doc` | живой перенос `doc_media` → `vibe-doc/media` и новая страница документации | нет — чужая работа в полёте |
| шаг 5 | `conform check`, 27 новых | накопленный дрейф бюджета и банов; ни одного моего файла в SARIF | нет — отдельный пакет |
| шаг 6b | `check-codegen` | на грязном дереве — кодогенерация P2-O7; на чистом измерить не удалось (§3.7) | нет |
| шаг 6c | `specmap --check`, 48 рёбер | `specmap.json` устарел относительно закоммиченного дерева | нет — мои коммиты рёбер не добавляют |
| шаг 6d | `wire-diff`, `index/e1` | тот же J-069 | выведен пакетом за периметр |

Ожидание пакета — «красным остаётся только `index/e1`» — **не
подтвердилось**, и разрыв больше, чем «сосед не доделал»: панель
останавливается на шаге 3 ещё до всего перечисленного, и останавливалась
так уже на входном HEAD. Ни одно красное не наведено моими двумя коммитами:
на чистом дереве `cli_search` (15 из 15) и все цели атома 2 зелены, в SARIF
моих файлов нет, рёбер specmap мои правки не добавляют.

---

## 5. Замечания о среде

- **Приватный `CARGO_TARGET_DIR`.** Первый проход (§3.3) и оба атома шли в
  `…/Temp/p1o6-target` и `…/Temp/p1o6-clippy` — общий `target/` в тот
  момент перестраивал сосед, и измерение по нему ничего не доказывает
  (P1-O5 §3.1 поймал на этом тринадцать ложных доктест-падений). Известная
  цена: `-p vibe-cli --test vvm` краснеет **по построению** (P1-O5 §4.4).
  Вторая, не известная до сегодня цена — диск (§3.7).
- **`cargo fmt --all` не запускался** руками — он переформатировал бы
  незакоммиченные файлы соседа. Вместо него `rustfmt --edition 2024
  --check` по шести моим файлам: `FMT-CHECK-EXIT=0` в обоих атомах. Общий
  `cargo fmt --all --check` (шаг 1) зелёный на всём дереве.
- **Дерево двигалось под работой всю сессию.** У P2-O7 последовательно
  появились `manifest.rs`/`manifest/`, `llms.rs`/`llms/`, изменился
  `lib.rs`; дважды дерево не собиралось; в `04:58Z` он закоммитил четыре
  атома; к сдаче идёт уже следующая работа (§3.1). Мои коммиты при этом
  идут подряд и целы: `git merge-base --is-ancestor` подтверждает оба.
- **Коммиты — только явной формой** `git commit -m … -- <мои пути>`:
  индекс общий, и эта форма берёт из рабочего дерева ровно перечисленные
  файлы. `git add` чужих путей не делался, `git push` не делался.
- **После себя убрано:** два каталога `generated.new-*` от упавшего
  `check-codegen` удалены; два приватных target-каталога удалены (84 ГБ).

---

## 6. `git status --short` на сдаче

```
 M Cargo.lock
 M README.md
 M RUNTIME-GUIDE.md
 M crates/vibe-check/Cargo.toml
 M crates/vibe-check/src/checks/doc_media.rs
 M crates/vibe-check/src/checks/doc_media/tests.rs
 M crates/vibe-cli/src/cli/doc.rs
 M crates/vibe-cli/src/commands/doc.rs
 M crates/vibe-doc/src/lib.rs
RM crates/vibe-check/src/checks/doc_media/image.rs -> crates/vibe-doc/src/media/image.rs
R  crates/vibe-check/src/checks/doc_media/image/tests.rs -> crates/vibe-doc/src/media/image/tests.rs
 M vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0/… (22 страницы документации)
?? campaigns/docs-2026-09/findings/WORKER-REPORT-P1-O6.md
?? crates/vibe-doc/src/media.rs
?? crates/vibe-doc/src/media/placeholder.rs
?? crates/vibe-doc/src/media/placeholder/
?? crates/vibe-doc/src/media/png.rs
?? crates/vibe-doc/src/media/png/
?? crates/vibe-doc/src/media/tests.rs
?? vibevm/vibepacks/org.vibevm.doc/web/v0.1.0/design/base.css
?? vibevm/vibepacks/org.vibevm.doc/web/v0.1.0/design/src/
```

Всё, кроме этого отчёта, — чужая живая работа. Мои файлы в дереве не
остались: оба атома закоммичены (`8df28576`, `aa8d196b`).

---

## 7. Долг для следующей волны

1. **Рэтчет wire-derive, шаг 3/49** — панель мертва на нём с `29351a68`.
   Развилка названа самим шагом: описать
   `crates/vibe-core/src/manifest/package/documentation.rs` схемой и
   прогнать `cargo xtask codegen`, **или** поднять `"vibe-core"` до 35 в
   `wire-derive-baseline.json` и сказать в теле коммита, что это за тип.
   Пока это не сделано, никакой прогон панели ничего не измеряет.
2. **«Ровно одна копия фикстурной DLL»**, §3.4(б) — пять целей падают на
   жёсткой ссылке, которую cargo делает сам. Тест должен схлопывать
   ссылки или смотреть в одно место.
3. **27 новых находок `conform`**, §3.5 — семнадцать `file-length`, сорок
   три `no-unwrap-in-domain`, пять `pub-doctest` и остальное.
4. **`specmap.json` устарел на 48 рёбер**, §3.4 — регенерация и коммит.
5. **`index/e1`**, §3.6 — три варианта для владельца в
   `WORKER-REPORT-P1-O4.md` §3.4; ни один не в мандате воркера.
6. **Доктест `crates/vibe-core/src/manifest/lockfile.rs:98`** —
   семнадцатая точка литерала `7`, §2.2. Одна правка.
7. **`lockfile/tests.rs:250`** — `assert_eq!(lf.meta.schema_version, 7)` в
   `read_accepts_current_version`, §2.1. Одна правка.
8. **Сетевые фикстуры**, §1.5 — всякая новая тестовая `vibe.toml` с
   `https://github.com/<org>` обязана называть `index_url`, иначе её цвет
   решает чужой репозиторий. Стоит пройти тем же грепом по остальным
   тестовым бинарям `vibe-cli`.
9. **`check-codegen` и диск**, §3.7 — две операционные находки; вторая
   касается всякой сессии, которая заводит приватный `CARGO_TARGET_DIR`
   на этой машине.
