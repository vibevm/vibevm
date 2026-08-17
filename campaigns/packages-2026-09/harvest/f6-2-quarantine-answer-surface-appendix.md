# Ф6.2 — карантин и ответ по имени: приложение к замеру

Продолжение находки `f6-2-quarantine-answer-surface.md` (разделы 7–11; раскол по шву разделов разрешён §2 пакета — основной файл превысил 600 строк). Нумерация разделов продолжается.

## 7. Один рычаг: --log-level и VIBE_LOG

**Установка подписчика целиком** (`crates/vibe-index/src/main.rs:6-31`):
```rust
fn main() -> ExitCode {
    init_tracing();
    match cli::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Install the tracing subscriber unconditionally — a binary's job, not
/// the library's. One lever, `VIBE_LOG` (default `warn`); there is no
/// `RUST_LOG` fallback and no second lever. WARN-level observability
/// (quarantine refusals on load, auto-commit-push outcomes) must be on
/// for every subcommand, not only the flag-gated ones.
fn init_tracing() {
    use tracing_subscriber::{EnvFilter, fmt};

    let filter = EnvFilter::try_from_env("VIBE_LOG").unwrap_or_else(|_| EnvFilter::new("warn"));
    let _ = fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_writer(std::io::stderr)
        .try_init();
}
```
Вызов — `main.rs:7`, первым действием, безусловно.

**Рычаг:** только `VIBE_LOG` (`main.rs:25`). `RUST_LOG` не читается нигде
(doc-строка `main.rs:18-19` прямо это обещает; в крейте других `EnvFilter`
нет — §0.2).

**Домашний образец** — `crates/vibe-cli/src/main.rs:408-417`, байт-в-байт та
же форма (`vibe-cli/main.rs:411`: `EnvFilter::try_from_env("VIBE_LOG")
.unwrap_or_else(|_| EnvFilter::new("warn"))`, writer stderr, `try_init`),
вызов безусловно (`vibe-cli/main.rs:52`). У `vibe` рычаг, кроме того,
Описан поверхностью: `vibe vars`/`show config` показывают `VIBE_LOG`
(`vibe-cli/src/commands/vars.rs:88`, `show/config.rs:119-120`).

**Где физически встанет `--log-level`** (У7: глобальных аргументов нет,
`cli/mod.rs:55-58`):
- **глобальным аргументом на `Cli`** (`#[arg(long, global = true)] level:
  Option<Level>` перед `#[command(subcommand)]`): доступен всем 15 подкомандам
  и после подкоманды (`vibe-index serve … --log-level debug`); парсится до
  диспетчеризации; трогает 1 файл (`cli/mod.rs`) + `main.rs` (уровень
  складывается с `VIBE_LOG` в `init_tracing`, которому придётся принять
  аргумент). Существующие тесты CLI не ломаются: `help_smoke.rs` проверяет
  `--help` каждой подкоманды и список в root-help (новый глобальный флаг в
  root-help добавляет строку, перечисление подкоманд не меняется);
  `assert_cmd`-прогоны флага не передают.
- **аргументом `serve`** (`serve.rs:19-60`): меньше поверхность (1 файл), но
  рычаг исчезает у остальных 14 подкоманд — противоречит D11 («подписчик
  ставится ВСЕГДА… уровень по умолчанию warn, `--log-level` для операторов»)
  и комментарию `serve.rs:66-67` («the binary installs the subscriber for
  every subcommand (one lever, `VIBE_LOG`)»).

**Закон одного рычага — применён, не переоткрыт.** Вопрос не «добавлять ли
флаг», а как флаг и переменная складываются в ОДИН рычаг. Композиции (ВЕЗДЕ
`init_tracing` читает ровно один источник в момент старта):
1. **Флаг перекрывает переменную** (`Some(v)` → фильтр из флага; `None` →
   `VIBE_LOG` → `warn`). Наблюдаемо: оператор управляет уровнем из командной
   строки, env — умолчание второго ранга. Прячет: установленная `VIBE_LOG`
   перестаёт объяснять наблюдаемое поведение (по выводу `ps`/скрипта не
   видно, что флаг перекрыл).
2. **Переменная перекрывает флаг** (`VIBE_LOG` set → она выигрывает).
   Наблюдаемо: окружение — всегда полная истина (стиль systemd). Прячет:
   флаг «не работает» при установленной env — читается как баг, поддержки
   будет стоить больше всего.
3. **Отсутствие флага = не трогать переменную** (`Option<Level>`: `None` →
   только `VIBE_LOG`; `Some(v)` → флаг). Наблюдаемо: дефолт остаётся одним
   рычагом, флаг — явным вмешательством; формально это та же композиция 1,
   но с явно проговорённым «None ≠ warn». Прячет: два способа задать один
   уровень — тест обязан покрыть обе ветки, иначе пара дрейфует.
4. **Флаг как запись в env** (`set_var("VIBE_LOG", v)` до `init_tracing`,
   затем единый `try_from_env`). Наблюдаемо: механически один рычаг —
   `EnvFilter` читает всегда одно место. Прячет: мутация env видна дочерним
   процессам (`serve` → `--auto-commit-push` порождает git; сегодня
   `vibe-cli/main.rs:391-403` уже применяет точно такой трюк с
   `SAFETY`-комментарием — прецедент есть).
Выбор — боссов (§11.2).

**Тесты про логирование `vibe-index`: их НЕТ** — grep по
`crates/vibe-index` (`VIBE_LOG|init_tracing|tracing_subscriber`) даёт хиты
только в `src/main.rs` (6 строк) и ни одного в `tests/**` (проверка
инструмента — §0.2). Подписчик — свойство бинаря, `fn main`, ни один
тестовый таргет его не трогает и не утверждает.

## 8. Базовая линия и что покраснеет

**Базовая линия (дерево зелено ДО работы):**
```
$ cargo check -p vibe-index -p vibe-registry --all-targets
    Checking vibe-registry v0.1.0-dev (…/crates/vibe-registry)
    Checking vibe-index v0.1.0-dev (…/crates/vibe-index)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 16.35s
EXIT=0
```

**`cargo test -p vibe-index quarantine`** — фильтр ловит 3 теста (lib-таргет),
EXIT=0. ВСЕ строки `test result` (22 таргета — каждый интеграционный файл
плюс lib и bin):
```
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 135 filtered out; finished in 0.02s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 10 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 8 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 7 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 7 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 20 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 16 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s
```
Пойманные имена (`-- --list`):
```
index::memory::tests::load_quarantines_unknown_capability_and_keeps_the_rest: test
index::quarantine::tests::empty_must_understand_needs_nothing: test
index::quarantine::tests::unknown_capability_is_reported_missing: test
```
(первый — `memory/tests.rs:175`, два — `quarantine.rs:46,51`).

**Тесты, утверждающие СЕГОДНЯШНИЙ ответ по имени** (кандидаты на покраснение
при появлении `unavailable`; уровень претензии указан у каждого):
- `tests/cli_read.rs:192` `get_unknown_json_form_returns_found_false` —
  держатель формы молчания (`found:false, versions:[]`, EXIT=0); краснеет,
  если not-found-конверт меняет форму или код;
- `tests/cli_read.rs:176` `get_unknown_package_text_form_errors` — текстовый
  отказ + EXIT=1; краснеет при смене кода/текста;
- `tests/cli_read.rs:268` `search_no_match_returns_zero_hits` и
  `:376` `outdated_unknown_packages_marked_unknown` — держатели «пустой
  ответ = законный ответ»;
- `tests/cli_read.rs:132,153` (`get_returns_versions_for_known_package`,
  `get_specific_version_filters_correctly`) — зелёные, пока их индексы без
  `must_understand` (свои манифесты), но это же делает их НЕ покрытием
  карантинного случая;
- `tests/server_e2e.rs:281` `by_name_route_serves_candidate_set_file`,
  `:299` `by_name_route_404s_for_missing`, `:314`
  `packages_list_returns_sorted_envelope`, `:328`
  `packages_search_via_query_param`, `:343`
  `package_versions_returns_full_entries`, `:357`
  `single_version_returns_entry`, `:371`
  `single_version_404_for_missing_version`, `:438`
  `unknown_group_in_url_yields_not_found` — серверная форма ответа по имени;
- `index/memory/tests.rs:175`
  `load_quarantines_unknown_capability_and_keeps_the_rest` — утверждает
  сегодняшнюю семантику отбрасывания (длина версий, содержимое карантина);
  краснеет при любом изменении носителя или семантики.

**Тесты, утверждающие форму `--json`:** `tests/cli_read.rs` весь файл
(`--json` в каждом прогоне: строки 136-150, 157-172, 196-209, 217-233,
241-248, 256-264, 272-278, 286-304, 312-325, 354-370, 391-404);
`tests/server_e2e.rs:141` `body_to_json` + перечисленные выше;
байтовые: `tests/wire_parity_by_name.rs` (константы полноты
`NAME_ENTRY_KEY_COUNT=4`, `PACKAGE_ENTRY_KEY_COUNT=5`,
`VERSION_ENTRY_KEY_COUNT=33`, строки 47-49 — ЛЮБОЕ изменение схемы записи
меняет счёт и краснит), `tests/wire_parity_entry.rs`,
`tests/golden_corpus.rs:189` `the_catalog_is_the_projection_of_its_journal`
(байтовое сравнение проекции — краснеет, если `write_to` начнёт писать
что-либо новое в файлы каталога).

## 9. Периметр строящего шага

**Создаст:**
- тест приёмки ТЗ Ф6.2 («запись в карантине → поиск/по имени отвечает
  `unavailable` с причиной»; «лог несёт warn без флага автопубликации») —
  расширение `tests/cli_read.rs` / `tests/server_e2e.rs` или новый файл;
- каталог рецептов, ЕСЛИ босс выбирает вариант (A) или (B) §5 — новый файл
  данных (+ запись в `formats/REGISTRY.toml`, + при (A) схема и включение в
  codegen; при `public=false, break_window_open=true` записка опциональна по
  D13);
- (при композиции 4 §7 — ничего нового, только правка `main.rs`).

**Изменит** (каждый — с одной фразой «что меняется»):
- `crates/vibe-index/src/index/quarantine.rs` — носитель `Quarantined`
  обога­щается (recipe/ключ) и, вероятно, получает lookup «карантинные версии
  имени»;
- `crates/vibe-index/src/index/memory.rs` — наполнение карантина (340) и/или
  доступ к нему по имени для поверхностей;
- `crates/vibe-index/src/cli/get.rs` — конверт `GetEnvelope` + текстовая
  ветка несут `unavailable` для карантинных версий;
- `crates/vibe-index/src/cli/search.rs` — `HitRow`/текст: пометка или
  исключение с явной причиной;
- `crates/vibe-index/src/cli/outdated.rs` — статусная строка для имени, чья
  единственная версия в карантине (сегодня `unknown`);
- `crates/vibe-index/src/cli/{list,capabilities,purls}.rs` — если периметр
  решает, что перечень/лукапы тоже обязаны говорить;
- `crates/vibe-index/src/cli/mod.rs` — глобальный `--log-level` (если
  выбрана эта форма);
- `crates/vibe-index/src/main.rs` — `init_tracing` принимает уровень,
  сложенный с `VIBE_LOG` по выбранной композиции;
- `crates/vibe-index/src/server/routes/packages.rs` — серверные ответы по
  имени (ФОРМА ЗАВИСИТ ОТ РАЗВИЛКИ §11.1 — сегодня серверное
  `Index.quarantined` всегда пусто);
- `crates/vibe-registry/src/index_client/{mod.rs,wire.rs}` — если
  `unavailable` появляется в by-name/структурированных ответах, клиентские
  view-типы и `list_versions` (сегодня `must_understand` игнорируется:
  `mod.rs:272`);
- `schemas/index/e1/*.jtd.json` + `formats/REGISTRY.toml` + регенерация
  `cargo xtask codegen` — только если `unavailable` уходит в ФАЙЛОВУЮ форму
  (`by-name`); тогда же — записка `formats/breaks/NNN.md` по режиму D13;
- `crates/vibe-index/tests/{cli_read,server_e2e,wire_parity_*}.rs` —
  ассерты формы (перечень держателей — §8).

**Сломает** (перестанет компилироваться / покраснеет — важнейший список):
- `crates/vibe-index/src/index/memory.rs:340` — литерал `Quarantined {…}`:
  добавление поля в структуру = ошибка компиляции в том же крейте;
- `tests/wire_parity_by_name.rs:47-49` и `tests/wire_parity_entry.rs` —
  счётчики ключей (4/5/33) при ЛЮБОМ изменении JTD-схемы записи;
- `tests/golden_corpus.rs:189` — при любом изменении того, что `write_to`
  кладёт в файлы каталога;
- `index/memory/tests.rs:175` — при смене семантики отбрасывания/носителя;
- `tests/cli_read.rs:176,192,268,376` — при смене формы/кода молчаливых
  ответов (см. §8);
- `journal/project_tests.rs:90` — краснеет, только если Ж7 нарушена (проекция
  начнёт карантинить) — тест-страж, трогать не нужно;
- `index_client`-тесты (`crates/vibe-registry/src/index_client/tests.rs` +
  `tests/wire_parity_*`) — при смене wire-формы by-name/ответов.

**Команда босса для замера разлёта компилятором:**
`cargo check -p vibe-index -p vibe-registry --all-targets` — та же пара
крейтов и тот же флаг, что и базовая линия §8 (сравнимо строка-в-строку);
`--all-targets` обязателен: 22 тестовых таргета `vibe-index` краснеют раньше
бинаря (константы wire-parity, ассерты конвертов), а `vibe-registry` тащит
клиентскую сторону (`index_client`), которую Ф6.2 задевает впервые за фазу.
После зелёного чека — `cargo test -p vibe-index -p vibe-registry` и
`cargo xtask check-codegen` (если тронуты схемы).

## 10. Расхождения с пакетом

1. **У10/§3: «около 14 вызовов `.route(`» — фактически 15**
   (`grep -c "\.route(" crates/vibe-index/src/server/mod.rs` → `15`;
   таблица §3 покрывает все 15). «Около» — не ошибка, но число для нарезки
   теперь точное.
2. **У3: «около строк 319–361» — фактический диапазон цикла 315–364**
   (`load_from` объявлен на 315, warn на 333, push на 340, сборка структуры
   353–363). Суть утверждения (ровно одно место, с warn) подтвердилась без
   оговорок.
3. **§1 пакета: «судя по всёму, больше нигде не всплывает» — подтверждено
   как факт, не предположение** (У5-перечень: чтения только в двух тестах).
4. **В1, число «шесть» из WAL** — совпало при правиле «имя или запрос,
   разрешаемый в имена; перечисления и мутации не в счёт» (§3). Пограничные
   случаи, меняющие число при другом правиле, перечислены там же (raw-файл
   `by-name` → 5; `?q=` → 7).
5. **Первичный листинг копии корпуса** казался лишённым журнала (`find
   -maxdepth 2`); проверка глубже показала `state/journal/*.ndjson` на месте.
   Расхождения с деревом нет — расхождение было между глубиной листинга и
   глубиной каталога.

## 11. Открытые развилки для босса

1. **Где возникает серверный `unavailable`?** Вопрос: ТЗ §9 Ф6.2 требует
   «vibe И СЕРВЕР отвечают unavailable», но серверный индекс — проекция
   журнала, карантин которой всегда пуст (Ж7; `project.rs:95-97`, тест
   `project_tests.rs:90`), и носитель `Index.quarantined` на сервере не
   наполняется никогда.
   - Вариант A — проверка `must_understand` В МОМЕНТЕ ОТВЕТА на версиях,
     стоящих в проекции: сервер и CLI отвечают `unavailable` по одной и той
     же причине; проекция и Ж7 не тронуты; цена — ответ-слой обязан
     пересчитывать `missing_capabilities` по каждой версии (или кэшировать),
     меняются `packages.rs:163-217` + конверты; сырой `by-name`-файл
     по-прежнему отдаёт запись дословно (клиентскую сторону это развязывает
     отдельно).
   - Вариант B — карантин в проекции: `project()` начинает отбрасывать
     версии с незнакомым `must_understand` в носитель; цена — нарушение Ж7
     (переоткрытие решённого), красный `project_tests.rs:90`, рассинхрон
     «журнал → каталог» в golden corpus (файл писал 0.1.0, проекция его
     теряет ⇒ `golden_corpus.rs:189` красный), потеря «трёх версий» у
     сервера целиком.
2. **Композиция `--log-level` × `VIBE_LOG` (закон одного рычага).** Вопрос:
   какая из четырёх композиций §7 принята (флаг>env · env>флаг ·
   None=не-трогать · флаг=запись-в-env) и где флаг живёт (глобально на
   `Cli` против `serve`). Цены — §7; выбор не мой.
3. **Каталог рецептов A/B/C** — вопрос и цены в §5: (A) новый файл данных
   (+формат, +первый рантайм-читатель `formats/**`, +гейт), (B) поле в
   `formats/REGISTRY.toml` (+разделение строгого лоадера с xtask, ключ
   слабее — на формат, не на причину), (C) литерал по месту (0 файлов, но
   прямое нарушение А.7 и N копий текста).
4. **Как померить серверную сторону прогона без порта** (для приёмки Ф6.2):
   in-process тест по образцу `tests/server_e2e.rs:77` `populated_state()`
   поднимает `AppState` без листенера — но чтобы сервер увидел карантинную
   запись, тестовая data-dir должна нести `must_understand` через журнал;
   сегодня `server_e2e` таких записей не строит. Строящему пакету —
   заложить.
5. **Незарегистрированные поверхности ответа** (§4): конверты CLI `--json` и
   HTTP `/v1/**` читаются чужим клиентом (`index_client/wire.rs`), но не
   входят в `formats/REGISTRY.toml` и не имеют схем. Вопрос отдельным шагом
   (Ф6.2 может стать поводом): регистрировать их или сознательно оставить
   внутренними.
