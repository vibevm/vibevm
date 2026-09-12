# WORKER-REPORT-P2-O5 — аудитория `agent` и документация вне судейства (A2.12, A2.21)

Пакет: `campaigns/docs-2026-09/findings/PACKET-P2-O5.md`
Дата: 2026-09-12. Ветка: `research-preview-1-docs`. Push не делался.

## Коммиты

- `d608a797` — `feat(progress): admit the agent audience` (A2.12)
- `b9be9f3b` — `chore(facts): keep documentation observed but unjudged` (A2.21)

Между ними лёг `f8220dd1` (`feat(doc): open the vibe-doc crate`) —
коммит параллельного воркера; дерево общее.

Отчёт идёт отдельным коммитом `docs(campaign): …`, как у P2-O2: продуктовый
атом остаётся одним коммитом и не может содержать собственный хэш.

## A2.12 — аудитория `agent`

Спековая половина уже стояла в дереве (`390c7b1d`, атом A1.6): оба факта
PROP-043 несли `status="spec/done" action="continue" actionstage="impl"` —
«решение принято, реализация за фазой 2». Этот коммит — реализация.

**Файлы**

- `crates/progress-core/src/model.rs` — вариант `Audience::Agent`, `ALL` из
  четырёх, ветка `as_str`; `parse` был generic по `ALL` и не тронут.
- `crates/progress-core/src/report.rs` — только тесты; фильтр
  `audience_matches` generic и не тронут.
- `crates/vibe-specdoc/tests/docs_corpus.rs` — снятие карантина и счётчики.

**Что получилось само.** Парсер CSV-атрибута (`element.rs:185`), подсказка
«did you mean», `Audience::parse`, фильтр отчёта и тест
`vocabularies_round_trip` — все generic по `Audience::ALL`, поэтому один
вариант enum'а включил их разом. Счёт по таблице A0.5: из девяти названных
мест кода правку потребовали три, и все три — в одном файле рядом
(`enum`, `ALL`, `as_str`); четыре не потребовали ничего благодаря
генеричности; два остались за границей надела (см. «Что не сделано»,
пункт 1). Прогноз находки подтвердился.

**Дефолт не сдвинулся.** `audience_matches` при пустом списке отвечает
`Dev` (§3.6) — `agent` этой ветки не касается, и ни одна существующая
строка не переехала в отчёт агента. Это отдельный тест
(`the_agent_audience_filters_without_widening_the_default`), потому что
именно здесь тихое расширение дефолта было бы незаметно.

**Словарь закреплён поимённо.** `the_audience_vocabulary_is_exactly_the_four_amended_names`
проверяет не круговой ход `parse ∘ as_str` (он тавтологичен по `ALL`), а
буквальный список имён в порядке §3.6 — значение, появившееся в enum'е без
строки в спеке, ломает этот тест. `VOCAB-AMENDMENT-ONLY` получил
исполнителя.

**Карантин снят.** `agent/how-agents-read-this-manual.xml` читается пивотом;
запись `QUARANTINED` удалена, список стал пустым. Механизм оставлен — с
тестом `quarantined_pages_still_carry_exactly_the_recorded_defect`, который
теперь ходит по пустому списку, — чтобы следующая страница, написанная
вперёд своего читателя, стоила одной строки, а не конструкции. Doc-комментарий
списка переписан в историю: три записи, которые он держал, и как каждая ушла.
(Две из трёх — `glossary/index.xml` и `reference/machine-formats.xml` — были
починены раньше, а буллеты про них остались висеть над записью-одиночкой;
после опустошения const они врали бы втройне.)

**Счётчики корпуса.** `44 - QUARANTINED.len()` стало 44 — выражение
оставлено, оно само себя правит. `rules` 373 → 381: страница несёт восемь
`rule` и ноль примеров, ноль `derived`, ноль промптов, ровно как обещал
пакет; остальные шесть утверждений `docs_corpus_shape_is_counted` —
включая кортеж из пяти конструкций, которых в корпусе ещё нет, — не
двинулись.

**Новая запись в `NOT_CANONICAL`.** Страница парсится, но не
канонична — `compact <tr> rows`, тот же дефект, что у четырнадцати других.
Проверено не на глаз: временный тест печатал первое расхождение
`to_xml(ir)` с исходником — строка 8, `<tr><td>…</td>…</tr>` против
`<tr>` + строка на ячейку, и весь дальнейший сдвиг из той же причины.

## A2.21 — документация наблюдаема, но не судится

**Файлы**

- `crates/progress-core/src/scope.rs` — таблица `[judging]`, тип
  `JudgingExemption`, пять юнит-тестов.
- `facts.toml` — ключ `[judging] exempt` с обоснованием.
- `campaigns/packages-2026-09/tasks/judging-debt.py` — чтение ключа,
  фильтр, строка в сводке; попутно — правка устаревших цитат.
- `crates/vibe-facts/tests/judging_exempt.rs` (новый) — три теста-сторожа.

### Механизм — вариант (а) из A0.19, в форме, которую называет PROP-057

`OBS-NOT-JUDGED` называет ключ буквально: `[judging] exempt = ["<glob>", …]`
в `facts.toml`, «читается скриптом долга сейчас и отгруженным глаголом
потом». Реализовано так:

- `JudgingSection { exempt: Vec<String> }` в `ScopeConfig`, `#[serde(default)]` —
  отсутствие ключа равно поведению конфигурации, которая его не знала.
- `JudgingExemption` — отдельный тип, а не поле в перечислении файлов.
  Освобождение обязано быть неспособно сузить корпус, и тип делает это
  видимым на месте вызова: `observed_files` о нём не знает вовсе,
  спрашивает его только потребитель, который считает долг.
- Юнит-тест `a_judging_exemption_leaves_the_corpus_untouched` — это и есть
  главный закон ключа: файл под `exempt` остаётся в перечислении,
  `ExcludeReport::dropped` равен нулю.

Почему не (б) и не (в): как и писала A0.19, (б) — отдельный спек-раунд с
закрытым словарём и проход разметки по всему корпусу, (в) смешивает ось
«что пакет устанавливает» с осью «сколько в нём прозы». (а) — единственный,
который конструктивно разводит «наблюдается» и «считается в долг», и именно
его назвала норма.

### Диалект глобов — найдено при проверке, не выбрано

`glob::Pattern::matches` вызывается с дефолтными `MatchOptions`, а они **не**
требуют литерального разделителя: `d/*/f` в этом дереве совпадает с
`d/e/x/f`. Так ключ `exclude` читается с DRIFT-024, и `exempt` намеренно
читается так же — один файл конфигурации не должен держать два прочтения
`*`. Зафиксировано тестом
`the_exempt_globs_speak_the_dialect_the_exclude_key_speaks`; те же семь
случаев проверены на питоновском переводе глобов в скрипте, потому что долг
считают сейчас два разных читателя и в день замены стопгапа глаголом
паттерн не должен сменить смысл.

### Сторож против устаревания списка

`OBS-NOT-JUDGED` требует, чтобы перечисление не протухало: «ячейка
`vibe check` проверяет, что путь каждого in-tree пакета вида `doc` покрыт
`exempt`». Ячейка `vibe check` не сделана (см. «Что не сделано»); проверка
живёт тестом `crates/vibe-facts/tests/judging_exempt.rs`, который делает
ровно то же и держит три утверждения:

1. `every_in_tree_doc_package_is_exempt_from_judging` — обход
   `vibevm/vibepacks/**` за манифестами с `kind = "doc"` (сравнение с
   `PackageKind::Doc`, не со строкой), каждый найденный слот обязан быть
   покрыт. Проверено, что сторож кусается: с удалённой строкой `exempt`
   тест падает и называет пакет —
   `these packages declare kind = "doc" and would enter the judging debt:
   ["vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0"]`.
2. `the_exemption_reaches_the_pages_and_not_just_the_slot` — долг считается
   по файлам, поэтому глоб, называющий только каталог, не освобождает
   ничего; каждая `.md`/`.xml` под слотом обязана быть покрыта.
3. `no_documentation_package_is_excluded_as_well_as_exempt` — `exempt` и
   `exclude` не должны называть один путь: пакет, названный обоими, был бы
   невидим, выглядя ухоженным.

Утверждение — по **слотам пакетов**, а не по наблюдаемому корпусу,
намеренно: освобождение обязано быть объявлено **до** include-глоба, который
делает пакет наблюдаемым, чтобы пакет не провёл ни одного коммита
одновременно наблюдаемым и судимым. Сегодня дерево именно в этом состоянии
(см. ниже), и тест уже зелёный.

## X-024 — освобождает ли `[judging] exempt` и от `--exhaustive`

**Решение: да.** Рекомендация центральной сессии подтверждается, но по
дороге выяснилось, что вопрос сегодня не встаёт — и почему он обязан быть
решён так, когда встанет.

**Вопрос сегодня не достигает `--exhaustive`.** Я проверил эмпирически:
добавил `vibevm/vibepacks/org.vibevm.core/**/*.{md,xml}` в `include` и
запустил гейт. `vibe facts check --exhaustive` не доходит до подсчёта
маркеров — он падает на **чтении первой же страницы**:

```
error: reading …/vibevm-docs/v0.1.0/vibevm/vibespecs/agent/ask-your-agent.xml:
line 8: <rule> belongs to the documentation vocabulary, which is open only in
packages of kind `doc` — see
spec://org.vibevm.core/vibevm/common/PROP-045#DOC-VOCAB-BY-KIND
```

Загрузчик корпуса зовёт `vibe_specdoc::load_spec_text`, то есть словарь
`Vocabulary::Spec`. Аддитивный вход `load_spec_text_with(path, Vocabulary::Doc)`
уже существует (его сделал P2-O2 атомом A2.8), но выбор словаря по `kind`
владеющего пакета — дело вызывающей стороны (`DOC-VOCAB-BY-KIND`), и эта
сторона одна: `crates/vibe-cli/src/commands/progress/grounding.rs:122`.
Файл в наделе воркера P2-O1, поэтому не тронут (см. «Что не сделано»).

**Почему всё же «да», по существу.**

1. Это одна ошибка категории, а не две. `--exhaustive` требует, чтобы
   каждая единица несла маркер; маркер на абзаце руководства утверждает,
   что этот абзац — обещание на такой-то стадии. Вердикт на нём не
   утверждает ничего — и маркер на нём тоже.
2. Так говорит собственная история `facts.toml`. Книга (F-091) и
   одиннадцать legacy-гидов (F-080) были **исключены** ровно за
   неимением способа сказать «размечено, но не верифицируется», и файл
   прямо пишет, что `--exhaustive` этого не выражает. `exempt` — тот самый
   способ; если он не покрывает `--exhaustive`, он не решает задачи, ради
   которой заведён, и книгу по-прежнему пришлось бы прятать.
3. Гейт от этого не слабеет. `facts check` без `--exhaustive` на
   освобождённой странице продолжает всё: ловит малформленные маркеры,
   отказывает на нарушениях словаря, проверяет якоря. Снимается ровно одно
   требование — «маркер на каждую единицу».
4. Границу стоит записать словами, потому что её будут проверять:
   `exempt` — не способ утихомирить нормативный пакет, отставший по
   вердиктам. Это долг, и он платится, а не освобождается. Ключ перечислим
   и обозреваем именно поэтому.

**Что из этого следует для плана.** Половина «наблюдаемости» и половина
«`--exhaustive`» — одна работа и один владелец: выбор словаря по `kind` в
`grounding.rs` и пропуск `unmarked_facts` для освобождённых документов в
`facts_check.rs` бесполезны поодиночке (первая без второй завалит гейт
тысячами `[unmarked]`, вторая без первой нечего освобождать). Предлагаю
отдать их одним атомом тому, кто держит `crates/vibe-cli/src`.

## Гейты

**О `cargo fmt`.** Форматировал только свои крейты — `cargo fmt -p …`, не
`cargo fmt --all`. Причина не вкусовая: в середине работы
`cargo fmt --all --check` в этом дереве был нечист чужим файлом
(`crates/vibe-core/src/manifest/package/documentation.rs`, воркер P2-O1,
тогда ещё untracked), и `cargo fmt --all` переписал бы его. К моменту
отчёта воркер отформатировал его сам, и названный пакетом гейт зелёный:

```
$ cargo fmt -p progress-core -p vibe-facts -p vibe-specdoc --check
FMT_OK

$ cargo fmt --all --check
FMT_ALL_EXIT=0
```

```
$ cargo test -p progress-core -p vibe-facts -p vibe-specdoc
test result: ok. 195 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 26 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 114 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

```
$ cargo clippy -p progress-core -p vibe-facts -p vibe-specdoc --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 36.85s
CLIPPY_EXIT=0
```

```
$ cargo check --workspace --all-targets
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 45.05s
CHECK_EXIT=0

$ cargo build --workspace --exclude vibe-cli
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 10.79s
EXIT=0
```

```
$ cargo build --workspace
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 11.93s
BUILD_OK
```

Этот гейт пришлось ждать: пока параллельный воркер гонял свой CLI-набор
(до пяти живых `vibe.exe` одновременно), линковка падала на
`error: failed to remove file target/debug/vibe.exe … Access is denied
(os error 5)` — состояние машины, не код. Ретрай-цикл поймал окно и прошёл
в общем `target/`; до того тот же гейт был прогнан в чистом
`CARGO_TARGET_DIR` (`Finished … in 3m 05s`, `ISO_EXIT=0`) — оба на тех же
исходниках.

```
$ target/debug/vibe.exe facts check --exhaustive
progress check: clean (325 files, 22 warning(s))
EXIT=0
```

Прогнан на **пересобранном** бинарнике, то есть на том, чей `ScopeConfig`
уже знает поле `judging`: живой `facts.toml` с новой таблицей читается, и
корпус от неё не изменился — те же 325 файлов и те же 22 предупреждения,
что до правки. Это и есть проверка главного закона ключа на живом дереве,
а не только на временных каталогах юнит-тестов.

Флаг проверен на собранном бинарнике, а не только в юнит-тестах:

```
$ target/debug/vibe.exe progress report --audience agent --view doc --md
| source | stage | state | action | comment |
|---|---|---|---|---|
| **…/core-ai-native/v1.0.0/README.md** (27 markers, 0/26 unmarked) | doc | done |  |  |
EXIT=0

$ target/debug/vibe.exe progress report --audience agents
error: unknown --audience `agents` (expected user|author|dev)
EXIT=1
```

Второй запуск — та самая устаревшая строка из пункта 1 «Что не сделано»:
значение отвергнуто правильно, перечисление в тексте ошибки отстало.

```
$ python campaigns/packages-2026-09/tasks/judging-debt.py
  UNJUDGED  facts with no verdict at all          0   in 0 file(s)
  ORPHANED  verdicts whose anchor is gone         0   in 0 file(s)
  STALE     files whose bytes moved since judging      0
```

Долг не вырос: он был нулевым до правок и остался нулевым. Честная оговорка
о том, что это значит: сегодня скрипт печатает `! 508 file(s) have no mirror`
— зеркала кампании в дереве нет, поэтому три счётчика равны нулю по причине
отсутствия данных, а не по причине уплаченного долга. Это состояние до
моих правок, я его не менял и не могу подтвердить им работу фильтра
освобождения. Фильтр проверен отдельно и напрямую: на живом `facts.toml`
питоновские глобы дают `EXEMPT` для страниц и `README.md` руководства и
`judged` для `redbook/README.md` и `PROP-057`.

## Что не сделано и почему

1. **Две устаревшие строки в `crates/vibe-cli/src`** после A2.12:
   `cli/progress.rs:133` (doc-комментарий флага `--audience`, уходит в
   `--help`) и `commands/progress.rs:146` (текст `bail!`) по-прежнему
   перечисляют `user | author | dev`. Функционально фильтр `--audience agent`
   работает (`Audience::parse` generic по `ALL`), устарели только два
   человекочитаемых перечисления. `crates/vibe-cli/src` — надел воркера
   P2-O1 по моему пакету, поэтому не тронуты. Правка на две строки.
2. **Наблюдаемость doc-пакетов** (вторая половина A2.21): требует выбора
   словаря по `kind` в `crates/vibe-cli/src/commands/progress/grounding.rs`
   — тот же надел. Разбор и рекомендация — в X-024 выше. Освобождение
   объявлено заранее и сторож зелёный, так что include-глоб можно добавить
   в тот же коммит, что и выбор словаря, без окна «наблюдаем и судим».
3. **Ячейка `vibe check`** из `OBS-NOT-JUDGED` не заведена. Новая ячейка —
   это правки `crates/vibe-check/src/lib.rs` и `checks/mod.rs`, общих
   файлов, а P2-O1 в этот момент правит `checks/boot_directory.rs`. Пакет
   ячейки не требовал; требование нормы закрыто тестом-сторожем той же
   силы (обход дерева за `kind = "doc"`, отказ при непокрытом пакете).
   Перенос теста в ячейку — механический.
4. **Статусы фактов PROP-043 не переведены в `impl/done`.** Оба факта
   аудитории несут `action="continue" actionstage="impl"` — «реализация
   ожидается», — и после A2.12 это уже неправда. Не трогал: правка
   PROP-файлов пакетом не разрешена, а смена байтов наблюдаемого документа
   — это ещё и `STALE` в долге судейства. Оставляю решение центральной
   сессии.

## Аномалии продукта, найденные по пути (без правок)

1. **`toml` 0.9: `str::parse::<toml::Value>()` не читает документ.**
   Целый манифест, отданный `FromStr`, отказывает с
   `unexpected content, expected nothing` (span `0..0`); правильный вход —
   `toml::from_str`. Опасно тем, что при `.ok()?` отказ молчит: мой сторож
   в первой редакции «не нашёл ни одного doc-пакета» и прошёл бы мимо
   дерева, если бы не утверждение «список не может быть пустым».
   `grep` по `crates/` и `xtask/` вхождений `parse::<toml::Value>` не дал —
   сегодня в дереве этой ловушки больше нигде нет.
2. **`glob::Pattern` с дефолтными `MatchOptions`: `*` пересекает `/`.**
   Это касается не только нового ключа, но и живого `exclude` в
   `facts.toml`: любой его паттерн шире, чем выглядит. Ни одно из пяти
   текущих исключений от этого сегодня не страдает — ни в одном нет `*`
   внутри сегмента, все называют файл или подкаталог целиком, — но при
   следующем добавлении паттерна это стоит помнить. Зафиксировано тестом.
3. **Цитаты в `judging-debt.py` указывали на переехавшие анкеры** —
   `PROP-043 §10.1`, `##DEBT-IS-A-LIST-NOT-A-RATIO`, `##DEBT-MUST-BE-ASKABLE`
   живут в `PROP-047` §6.2 после раздвоения 2026-08-22 (находка A0.19,
   раздел «Расхождения с решениями»). Исправил в том же коммите: файл всё
   равно правился, а мой новый текст цитирует PROP-047, и две цитаты одного
   файла не могут расходиться. Это единственная правка за пределами
   заявленного механизма.
4. **Doc-комментарий `QUARANTINED` описывал три записи при одной живой** —
   `glossary/index.xml` и `reference/machine-formats.xml` были починены
   раньше, а буллеты остались. Переписан вместе со снятием карантина.

## `git status --short`

Своих незакоммиченных файлов нет — оба атома закоммичены, отчёт уходит
третьим коммитом. Всё остальное в списке принадлежит параллельным воркерам
и лежало в дереве до меня или появилось во время работы; ничего из этого я
не трогал и не стейджил.

```
 M Cargo.lock
 M crates/vibe-check/src/checks/boot_directory.rs
 M crates/vibe-cli/Cargo.toml
 M crates/vibe-cli/src/cli.rs
 M crates/vibe-cli/src/commands/mod.rs
 M crates/vibe-cli/src/main.rs
 M crates/vibe-cli/tests/cli_facts.rs
 M crates/vibe-cli/tests/cli_mcp_path_parity.rs
 M crates/vibe-cli/tests/cli_redirect.rs
 M crates/vibe-core/src/manifest/document.rs
 M crates/vibe-core/src/manifest/document/validation.rs
 M crates/vibe-core/src/manifest/mod.rs
 M crates/vibe-core/src/manifest/package.rs
 M crates/vibe-core/src/manifest/package/tests.rs
 M crates/vibe-core/src/manifest/package/visibility.rs
 M crates/vibe-doc/src/lib.rs
 M crates/vibe-install/tests/doc_kind_refusal.rs
 M crates/vibe-install/tests/incremental_in_place.rs
 M crates/vibe-mcp/tests/tools_oracle.rs
 M crates/vibe-mcp/tests/tools_oracle/dispatch_compat.rs
 M crates/vibe-requirements/src/tests_followup.rs
 M crates/vibe-requirements/src/tests_provider.rs
 M crates/vibe-requirements/src/tests_query.rs
 M crates/vibe-workspace/src/bins/tests.rs
 M crates/vibe-workspace/src/freshness.rs
?? campaigns/docs-2026-09/findings/WORKER-REPORT-P2-O5.md
?? crates/vibe-cli/src/cli/doc.rs
?? crates/vibe-cli/src/commands/doc.rs
?? crates/vibe-core/src/manifest/document/tests_documentation.rs
?? crates/vibe-core/src/manifest/package/documentation.rs
?? crates/vibe-doc/src/examples.rs
?? crates/vibe-doc/src/examples/
?? vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0/examples/
```

Одно наблюдение для центральной сессии, а не претензия: `crates/vibe-cli/src`
уже правится воркером (`cli/doc.rs`, `commands/doc.rs`, `cli.rs`,
`commands/mod.rs`, `main.rs`) — значит, две устаревшие строки из пункта 1
«Что не сделано» дешевле всего закрыть именно тем же воркером, пока файлы у
него открыты.
