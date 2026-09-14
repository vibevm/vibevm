# WORKER-REPORT-POST-O4

2026-09-14
Дерево: `87799488` (ветка `research-preview-1-docs`, ворктри
`C:\Users\olegc\git\v\vibevm-docs`)

Пакет: POST-O4 — короткий backoff на 429/5xx перед падением быстрого пути
в git (B-161). Прочитано ровно то, что пакет назвал: `raw_http.rs` +
`raw_http/tests.rs`, `shell.rs` + `tests/https_file_read.rs`, §2.12
PROP-002, разделы «Коммит 1» / «Замеры» / «Аномалии»
`WORKER-REPORT-POST-O1.md`.

## Коммиты

```
b709a02e perf(registry): retry a rate-limited raw read briefly before asking git
         5 files changed, 771 insertions(+), 50 deletions(-)
87799488 docs(spec): say that a rate-limited raw read is retried briefly
         3 files changed, 12 insertions(+), 7 deletions(-)
```

Оба — `Oleg Chirukhin <oleg@anarchic.pro>`, без трейлеров и без упоминаний
модели или агента, формой `git commit -m … -- <пути>`; новый файл добавлен
`git add -- <файл>`. `git push` не выполнялся, `specmap.json` не
регенерирован, чужих процессов не останавливал, `CARGO_TARGET_DIR` не
переопределял. Файлы веб-пакета не тронуты (за время работы воркер W1-O6
успел закоммитить `043da7de` и `1797723b`; после обоих моих коммитов
`git status` пуст).

## Коммит S — backoff

Семантика — как реализовано.

**Новое: `crates/vibe-registry/src/git_backend/shell/raw_http/retry.rs`**
(216 строк) — чистые решения без единого запроса и без единого сна:
какие ответы стоят повтора, сколько ждать, как читается `Retry-After`.

- `refused_for_now(status)` — **ровно** `429` и весь диапазон `5xx`. Это
  единственные ответы «не сейчас» (о хосте), всё прочее — ответ о файле.
- `worth_another_try(&reqwest::Error)` — транспортная ошибка повторяется,
  **кроме** двух случаев: имя не разрешилось и таймаут (см. «Отклонения»,
  п. 1).
- `pause_before_next(attempt, asked, already_waited)` — удвоение от 500 мс
  (0,5 с → 1 с → 2 с), `Retry-After` вместо расписания, когда он есть,
  всё вместе обрезано остатком общего бюджета.
- `retry_after(&HeaderMap)` — секунды или дата. Дата читается в
  предпочтительном RFC 9110 виде `IMF-fixdate`; два устаревших формата
  сознательно не читаются (это «нет значения», то есть обычное
  расписание). Разбор даты свой, по той же причине, что и `decode_base64`
  этажом выше: одна точка вызова не оправдывает зависимость, а в
  `Cargo.toml` пакет лезть запрещает. Алгоритм — `days_from_civil`
  Ховарда Хиннанта; проверен на примере самого RFC, на эпохе и на двух
  видах високосного дня (2016 и 2000).

**Выбранные константы:**

| что | значение | почему |
|---|---|---|
| `MAX_ATTEMPTS` | **3** (первая попытка + две) | приёмка пакета: «три 503 → падение в git» — значит три запроса всего |
| `FIRST_PAUSE_MS` | **500**, дальше удвоение | расписание пакета 0,5 с → 1 с → 2 с; при трёх попытках тратятся первые две паузы (0,5 + 1 = 1,5 с) |
| `PAUSE_BUDGET` | **5 с** на файл, суммарно по всем паузам | пакет даёт два ограничения одним числом — «`Retry-After` обрезается сверху пятью секундами» и «суммарно не дольше ~5 с»; общий бюджет покрывает оба, поэтому константа одна, а не две |
| `READ_TIMEOUT_SECS` | **10**, не менялся | существующее правило файла |

Худший случай на один файл: без `Retry-After` — 1,5 с ожидания и три
запроса; с `Retry-After: 60` — 5 с ожидания и два запроса (остаток
бюджета нулевой, третьей попытки нет); с таймаутом — как раньше, 10 с и
git.

**`raw_http.rs`.** `try_read` стал циклом; тело одной попытки вынесено в
`attempt_read`, возвращающий `Attempt::Settled(…)` (файл, поверенный
промах или «спросить git» — все три исхода прежние) либо
`Attempt::Again { status, after }`. Порядок разбора ответа не изменился:
2xx → промах хоста (+ `absence_is_authoritative`) → *и только затем*
проверка «отказ на сейчас» → падение в git. Поэтому ни `404` GitHub, ни
`400`+`4305` GitVerse повторов не получают, а `5xx` на GitVerse получает.

Логи. Строка на каждый повтор — статус, номер попытки, `wait_ms`:
`"https read was refused for now; waiting and asking again"`. Финальная
строка та же, что была, дословно: `"https read did not answer with the
file; falling back to git"` со статусом; для случая «ответа не было
вообще» — отдельная `"https read never reached the host; …"`. Поле
`status` — `Option<u16>`: `tracing-core` не печатает поле для `None`,
поэтому «статус 0» нигде не выдумывается.

**`shell.rs`.** Тип `Sleeper` (`Arc<dyn Fn(Duration) + Send + Sync>` с
ручным `Debug`, чтобы `ShellGit` сохранил производный `Debug`), поле
`raw_sleeper`, публичный строитель
`ShellGit::with_raw_sleeper(impl Fn(Duration) + Send + Sync + 'static)` —
прецедент и обоснование те же, что у `with_raw_base` (конструктор, а не
переменная окружения). `anonymized_for_public` отдаёт копии тот же
`Sleeper`, а не втихую возвращается к настоящему времени.

**Тесты.** Никаких задержек: весь файл `https_file_read.rs` — 13 тестов
за 0,03 с.

Чистые (в `raw_http/tests.rs`, 5 новых): `refused_for_now` по таблице
статусов; расписание и бюджет `pause_before_next` (включая «`Retry-After:
60` → остаток бюджета» и «бюджет исчерпан → повторов нет»); `retry_after`
на секундах, дате, пустоте и шести нечитаемых формах; `imf_fixdate_to_unix`
на примере RFC (`784111777`), эпохе, 2016-02-29 и 2000-02-29 плюс девять
не-дат; маркеры «хост неизвестен».

Мок-сервер (`tests/https_file_read.rs`, 4 новых): `429 → 200` со второй
попытки (2 запроса, паузы `[500ms]`); три `503` → `GitError::NotInstalled`
(3 запроса, паузы `[500ms, 1s]`); `Retry-After: 2` → паузы `[2s]`; `404`
не повторяется ни в виде поверенного промаха, ни в виде вопроса к git (по
1 запросу, паузы пусты). Мок получил скрипт ответов по адресу
(`Mock::script`), заголовок `Retry-After` в `Answer::after`, запись пауз
вместо сна (`Mock::pauses`).

**Один существующий тест изменён:**
`a_token_rides_the_header_and_appears_in_no_error` держал мок на `500` и
утверждал `seen.len() == 1`; теперь `5xx` повторяется, поэтому запросов
три — и тест стал строже: правило секретности проверяется **на каждом**
из трёх (`Bearer` в заголовке, токена нет ни в URL запроса, ни в тексте
ошибки).

## Коммит T — норма

`##RAW-READ-BACKOFF` добавлен в PROP-002 §2.12 сразу после
`PERF-FETCH-FILE`, текстом пакета дословно. Статус `impl/done`,
`action="continue" actionstage="doc"`, **без `audience`** — см. ниже.

Секция получила предложение, которого не было, поэтому `perf-req`
поднят `req r1` → `req r2`, а шесть `#[verifies(…PROP-002#perf, r = 1)]`
переведены на `r = 2` (два в `raw_http/tests.rs`, четыре в
`tests/https_file_read.rs`). Седьмым — новая метка `r = 2` на
`a_rate_limited_read_asks_again_instead_of_paying_for_git`: это тест,
который читает новое предложение напрямую. Больше рёбер не добавлял.

**`audience` снят, как пакет и предвидел.** С `audience="user"` гейт
покрытия краснеет:

```
$ target\debug\vibe.exe doc check --path vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0 --coverage --min 100
  UNCOVERED user — vibevm/vibespecs/modules/vibe-registry/PROP-002-decentralized-registry.xml#RAW-READ-BACKOFF
    vibevm/vibespecs/modules/vibe-registry/PROP-002-decentralized-registry.xml:699 — no page cites it
  user    311/312 (99%)
  author  220/220 (100%)
  dev     19/19 (100%)
  agent   40/40 (100%)
coverage: 535 of 536 obligation(s) told, 99% of 591 audience pair(s) (threshold 100%), 0 unreadable page(s)
error: the documentation does not tell everything the specifications promised: 99% of 591 audience pair(s) covered, 100 required, 0 page(s) unreadable (violates spec://org.vibevm.core/vibevm/common/PROP-057#OBS-COVERAGE-GATE; fix: write the page — the gate closes on a page for that audience citing the rule, never on a list of pages)
COV-EXIT=1
```

Со снятым атрибутом (`action`/`actionstage` оставлены — факт по-прежнему
должен документации) оба руководства зелёные:

```
$ target\debug\vibe.exe doc check --path vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0 --coverage --min 100
  user    311/311 (100%)
  author  220/220 (100%)
  dev     19/19 (100%)
  agent   40/40 (100%)
coverage: 535 of 535 obligation(s) told, 100% of 590 audience pair(s) (threshold 100%), 0 unreadable page(s)
COV-EN-EXIT=0

$ target\debug\vibe.exe doc check --path vibevm/vibepacks/org.vibevm.core/vibevm-docs-ru/v1.0.0 --coverage --min 100
  user    311/311 (100%)
  author  220/220 (100%)
  dev     19/19 (100%)
  agent   40/40 (100%)
coverage: 535 of 535 obligation(s) told, 100% of 590 audience pair(s) (threshold 100%), 0 unreadable page(s)
COV-RU-EXIT=0
```

Проза для руководства и возврат `audience="user"` — атом центральной
сессии, ровно как это было сделано в POST-O3 (`09f647d7` → `10e3eab4`).

## Гейты — дословно

`CARGO_TARGET_DIR` не переопределялся; всё против общего `target/`.
Прогон — на дереве второго коммита.

```
$ cargo fmt --all -- --check
FMT-EXIT=0
```

(Перед этим `cargo fmt --all` был запущен один раз: `--check` показывал
диффы **только** в трёх моих файлах, чужого форматирования в дереве не
было.)

```
$ cargo clippy -p vibe-registry --all-targets -- -D warnings
warning: C:\Users\olegc\git\v\vibevm-docs\crates\vibe-cli\Cargo.toml: unused manifest key: build
help: build is a valid .cargo/config.toml key
    Checking vibe-registry v1.0.0 (C:\Users\olegc\git\v\vibevm-docs\crates\vibe-registry)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.77s
CLIPPY-EXIT=0
```

```
$ cargo test -p vibe-registry
     Running unittests src\lib.rs
running 239 tests
test result: ok. 239 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 102.08s
     Running tests\https_file_read.rs
running 13 tests
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
     Running tests\index_auth.rs
running 8 tests
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
     Running tests\index_fast_path.rs
running 8 tests
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests\index_handshake.rs
running 7 tests
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests\index_search.rs
running 7 tests
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests\index_url_ladder.rs
running 3 tests
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests\registry_cells_oracle.rs
running 3 tests
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests\store.rs
running 2 tests
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
   Doc-tests vibe_registry
running 14 tests
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.46s
TEST-EXIT=0
```

Было 295 тестов, стало 304: +5 чистых, +4 интеграционных.

```
$ cargo xtask specmap --check
Error: `C:\Users\olegc\git\v\vibevm-docs\specmap.json` is out of date relative to the tree.
  drift: revision bump: `spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#perf` r1 → r2
  drift: units added: 1
  drift: edges added: 2
Run `rust-ai-native-specmap` (or your project's wrapper), review the drift, and commit the result.
SPECMAP-EXIT=1
```

Красный — **весь дрейф мой и ровно ожидаемый**: подъём ревизии §2.12,
одна новая единица (`##RAW-READ-BACKOFF`), два новых ребра (`scope!` в
`retry.rs` и новая метка `#[verifies]`). Чужого дрейфа нет — строка
`unbumped-hash` по `PROP-019#surface`, которую видел POST-O1, ушла с
регенерацией `75be0ac5`. Регенерацию карты пакет запрещает; она за
боссом.

Дополнительно (не гейт пакета, но затронут публичный тип):

```
$ cargo check --workspace --all-targets
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 18s
WS-CHECK-EXIT=0
```

## Отклонения от пакета

**1. Таймаут не повторяется — одно содержательное решение, не буквальное
следование.** Пакет говорит «на транспортную ошибку, если она не „хост
неизвестен“». Таймаут — тоже транспортная ошибка, но в самом файле уже
записано правило: `READ_TIMEOUT_SECS` — «чтение, не ответившее за десять
секунд, больше не быстрый путь». Повтор превратил бы один файл в 30 с
против обещанных пакетом «~5 с на файл» — то есть хуже той самой
лестницы, ради которой всё делается. Поэтому исключены три вида ошибок:
таймаут, «хост неизвестен» и ошибки построения/редиректа. Если босс хочет
буквальную формулировку, правка — одна строка в `worth_another_try`.

**2. Новый файл `raw_http/retry.rs` вне перечисленного периметра.**
Периметр пакета называл `raw_http.rs`, `raw_http/tests.rs`, `shell.rs`,
`tests/https_file_read.rs` и спеку. `raw_http.rs` уже стоял на 540
строках — на полу «опасной полосы» коллектора здоровья [540, 600) — и
разом принять ~215 строк не мог. Решения о числах и о тексте вынесены в
`raw_http/retry.rs` (тот же модуль, тот же каталог, что и у
`raw_http/tests.rs`; идиома репозитория — `shell/query.rs`,
`shell/tar.rs`). Ничего из запрещённого не тронуто: `Cargo.toml`,
`crates/vibe-cli/**`, веб-пакет, руководство, `specmap.json`.

Больше отклонений нет: паузы через инъекцию, тесты не спят, повторов на
404 нет, диагностика после последней попытки прежняя, новых зависимостей
нет.

## Аномалии и находки

**1. В PROP-002 вообще нет факта о быстром пути по HTTPS.** Пакет
предполагал, что «POST-O1 их добавил или уточнил» — не добавил: оба
коммита POST-O1 трогают только `crates/vibe-registry/**`. §2.12
по-прежнему описывает чтение файла как `git archive`
(`##PERF-FETCH-FILE`), а про `raw.githubusercontent.com` / `contents`-API,
про правило «промах поверен только для не-манифеста на неподвижном рефе»
и про один общий клиент спека молчит. `##RAW-READ-BACKOFF` положен рядом
с `##PERF-FETCH-FILE` — это ближайшее, что есть, — и получается, что
норма о повторе быстрого чтения стоит раньше нормы о самом быстром
чтении. **Кандидат в отдельный атом:** сказать в §2.12 сам быстрый путь
(и тогда же дать `audience` обоим фактам и написать страницу).

**2. `raw_http.rs` перешагнул бюджет длины: 605 строк при бюджете 600.**
Коллектор здоровья совещательный («never fails the build»), ни один гейт
пакета от этого не краснеет, поэтому рефакторинг сверх задания я не
делал. Естественный шов, если босс захочет его закрыть: вынести две
хостовые половины (`github_request_url`, `gitverse_*`, `decode_base64`)
в `raw_http/hosts.rs` — таблица `HOSTS` и `try_read` останутся
хост-слепыми, как и задумано. `shell.rs` (709 строк) был за бюджетом и
до меня.

**3. Формулировка факта про `404` чуть шире кода — сознательно оставлена
дословной.** «…on a tag or a commit it is authoritative» верно как довод
*не повторять* (на неподвижном рефе ответ не изменится), но как
утверждение о выводе не покрывает исключение для манифеста: `404` на
`vibe.toml` даже на теге уходит в git, потому что только git отличает
«нет файла» от «нет рефа / нет репозитория / не видно». Текст пакета
взят без правок; если босс читает предложение как утверждение о выводе —
нужна оговорка про манифест, и это тот же атом, что находка 1.

**4. Живой замер не снимался.** Пакет делает его необязательным, а 429 не
воспроизводится по желанию; ставить холодный `install` redbook (≥ 90 с
разрешения плюс фаза установки) ради ненаблюдаемого события в общем
ворктри я не стал. Корректность пути закрыта мок-сервером, где 429 и 503
воспроизводятся точно.

## Что не сделано и почему

1. **`specmap.json` не перегенерирован** — запрет пакета; дрейф описан
   выше и целиком мой.
2. **Проза руководства и `audience="user"`** — не мои; гейт покрытия
   доказывает, что атрибут ждёт страницы (вывод выше).
3. **Устаревшие формы даты `Retry-After`** (RFC 850 и asctime) не
   читаются — сказано в докстринге `retry_after`; они означают «нет
   значения», то есть обычное расписание.
4. **`Retry-After` больше бюджета не отменяет попытку, а обрезается** —
   как написано в пакете («уважается, но обрезается сверху пятью
   секундами»). Это стоит до 5 с на файл; альтернатива («минуту ждать не
   станем — сразу git») дешевле, но противоречила бы формулировке
   задания. Оставляю решение боссу.
5. **`index_client`, параллельный веер, память об `ArchiveUnsupported`** —
   вне периметра, как и в POST-O1.
