# WORKER-REPORT-POST-O1

2026-09-14
Дерево: `818247cd` (ветка `research-preview-1-docs`, ворктри
`C:\Users\olegc\git\v\vibevm-docs`)

Пакет: POST-O1 — `vibe install` молчит минуты; чтение манифестов
зависимостей без клона. Плюс три уточнения координатора, полученные по
ходу работы: (1) `refname` не всегда тег — правило «404 = файла нет»
действует только для фиксированного рефа; (2) быстрый путь оформить
таблицей хостов с двумя строками, вторая — `gitverse.ru`; (3) вторым
коммитом — один HTTPS-клиент на все чтения и вычистка учётных данных из
`GitError`.

## Коммиты

```
b058098f perf(registry): read manifests from GitHub over HTTPS instead of cloning
         7 files changed, 1358 insertions(+), 24 deletions(-)
818247cd perf(registry): keep one HTTPS client for raw reads and scrub credentials from git errors
         5 files changed, 342 insertions(+), 26 deletions(-)
```

Оба — владелец репозитория, без трейлеров, формой
`git commit -m … -- <пути>`; новые файлы добавлены `git add -- <файл>`.
`git push` не выполнялся. В том же ворктри параллельно работают другие
воркеры; их файлы не тронуты и в мои коммиты не попали (проверено
`git status` после каждого коммита).

## Коммит 1 — чтение файла по HTTPS вместо клона

Семантика — как реализовано.

**Новое: `crates/vibe-registry/src/git_backend/shell/raw_http.rs`** —
чтение одного файла по HTTPS, которое пробуется *до* `git archive` и до
`preflight` (обслуженное этим путём чтение вообще не требует git на
машине). Оформлено таблицей `static HOSTS: [RawHost; 2]`; строка хоста
заявляет три факта — как адресовать чтение, как прочитать тело 200, что
считать промахом. Третий хост — третья строка.

- `github.com` → `https://raw.githubusercontent.com/<owner>/<repo>/<ref>/<path>`,
  тело 200 — сами байты, промах — HTTP 404.
- `gitverse.ru` → `https://gitverse.ru/api/repos/<owner>/<repo>/contents/<path>?ref=<ref>`
  (сегменты и значение `ref` экранируются через `reqwest::Url`, как в
  `IndexClient::lookup_purl`; заголовок `Accept` не нужен), тело 200 —
  JSON `{"encoding":"base64","content":…}`, промах — HTTP 400 с
  `{"code":4305,…}`. Массив вместо объекта (каталог) и любая другая
  `encoding` — не ошибка, а падение в git-путь.
- Разбор URL: только `https://[userinfo@]<host>/<owner>/<repo>[.git][/]`,
  хост сравнивается целиком и без учёта регистра. SSH в обеих записях,
  `http://`, `git+https://`, `file://`, чужой хост, лишние сегменты пути,
  query/fragment — всё это не быстрый путь.
- Токен берётся из userinfo (`x-access-token:<TOKEN>` или голый
  `<token>@host`) и уходит **только** заголовком
  `Authorization: Bearer <token>`. В URL запроса, в `tracing::debug!` и в
  тексте ошибки — плоский URL без userinfo.
- `is_plain` — узкий allow-list для рефа и для пути (буквы, цифры,
  `. - _ + ~`, `/` как разделитель; сегменты `.` и `..` отклоняются).
  Всё, что могло бы увести URL в другое место, уходит в git, а не
  экранируется в нечто, отличное от прочитанного бы git.
- `absence_is_authoritative(refname, path)` — два условия, оба
  обязательны: (а) это не манифест (сравнивается последний сегмент пути с
  `Manifest::FILENAME`) и (б) реф именует неподвижное содержимое —
  `v<semver>` или 40 hex. Иначе 404/4305 не вывод, а повод спросить git.
  Это и есть уточнение (1) координатора: `[requires.packages]` git-source
  и `vibe install --git … --branch <b>` передают сюда имя ветки, и
  промах по движущейся верхушке ничего не говорит о файле.
- `decode_base64` — свой, whitespace-tolerant. Причина в докстринге: одна
  точка вызова не оправдывает зависимость (та же причина у
  `search::full_scan::decode_base64`), а тянуть приватную функцию через
  шов `search` нельзя; периметр пакета разрешает менять `Cargo.toml`
  только ради уже имеющейся dev-зависимости.

**`shell.rs`**: поле `raw_base: Option<String>` (`None` = продакшн-базы
хостов), строящие методы `with_binary` / `with_raw_base`, вызов
`raw_http::try_read` первым в `fetch_file_at_ref` и перед `preflight`.

Выбор «конструктор против переменной окружения» (вопрос шага 5 пакета) —
**конструктор**, по прецеденту самого крейта: комментарий у
`GitPerPackageRegistry::open_with_explicit_token` — «so the test does not
have to mutate the process env (forbidden by `#![forbid(unsafe_code)]` on
Rust 2024+)». Плюс процессная переменная была бы общей для всех тестов,
идущих параллельно в одном бинарнике.

**Тесты.** `raw_http/tests.rs` — 11 чистых тестов (отображение URL обоих
хостов, ветки и SHA, SSH/`http`/чужие хосты, только `<owner>/<repo>`,
токен и его отсутствие в URL, обходы и инъекции, правило промаха, чтение
тел, base64). `tests/https_file_read.rs` — один axum-мок отдаёт обе формы
на одном корне; доказательство «git не запускается» точное: бэкенд
привязан к несуществующему бинарнику, поэтому любой ответ, кроме
`GitError::NotInstalled`, доказывает, что до git не дошли, а
`NotInstalled` — что в git-путь вошли намеренно.

**Докстринги.** `lookup.rs` («N archive round-trips») и
`redirect_follow.rs` («Two-path read shape») переписаны в лестницу из трёх
шагов, первый — HTTPS.

**`@scope` новых элементов** —
`spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#perf` (§2.12,
где живёт `PERF-FETCH-FILE`); `#[verifies]` на тестах — тот же якорь плюс
`…#failure-discriminator` и
`spec://org.vibevm.core/vibevm/common/PROP-000#token-secrecy`.

## Коммит 2 — один клиент и вычистка учётных данных

**Один клиент.** Поле `raw_client: OnceLock<Option<reqwest::blocking::Client>>`
на бэкенде, строится при первом чтении (`raw_http::build_client`), тот же
таймаут 10 с и тот же `User-Agent`. Выбран экземплярный `OnceLock`, а не
процессный статик: у структуры уже есть ровно такой ленивый кэш
(`preflight_cache`), а бэкенд на время обхода и так один — он живёт за
`Arc<dyn GitBackend>`. Анонимный вариант из `anonymized_for_public`
получает клонированный клиент, то есть **тот же пул**: клиент не несёт
никакой позиции — заголовок решает само чтение. `None` внутри замка
значит «клиент не построился вообще» (сломанный TLS-бэкенд — свойство
машины, не запроса), тогда быстрого пути нет и все чтения идут в git.

**Вычистка учётных данных.** Две функции в `shell.rs`, через которые
теперь проходит всё:

- `without_userinfo(url)` — URL без userinfo. Применён во всех шести
  конструкциях `GitError` в `fetch_file_at_ref` и в месте, где
  `classify_failure` достаёт URL из argv (а из него URL получают
  `RepoNotFound`, `AuthFailed`, `NetworkUnreachable`, `RefNotFound`).
- `without_userinfo_in_arg(arg)` — то же для одного элемента argv, где
  URL бывает и голым (`clone -- <url>`), и за флагом
  (`archive --remote=<url>`). Применён в `render_argv` и
  `render_argv_for_display`, то есть закрывает `Io.cmd`,
  `CommandFailed.cmd` и обе строки `tracing::debug!(argv = …)`.

Почему argv тоже: подпись коммита обещает «scrub credentials from git
errors», а `CommandFailed.cmd` — такой же отображаемый текст, как
`FileNotFoundInRef.url`; чинить одно и оставить другое значило бы
обещать больше сделанного. Это и приводит код в соответствие с уже
записанным в `credentialed_url` инвариантом «vibe never logs the
resulting URL outside the spawned-process boundary», который argv-строка
нарушала.

scp-форма `git@host:org/repo.git` остаётся нетронутой: там `git` — имя
ssh-аккаунта, а не секрет, и удаление сделало бы URL неверным, а не
безопасным. У URL со схемой userinfo снимается целиком — имя
пользователя там фиксированный литерал `x-access-token`.

**Тесты коммита 2** (5 новых, все зелёные):

- `without_userinfo_strips_the_credential_and_nothing_else` — таблица
  форм, включая ssh, scp и «нечего снимать».
- `without_userinfo_in_arg_reaches_a_url_behind_a_flag` — `--remote=…`
  и голый URL чистятся, `--format=tar`, `credential.helper=`, рефспеки
  проходят без изменений.
- `a_classified_failure_names_the_url_without_its_credential` — реальный
  `classify_failure` с credentialed-argv: и классифицированный вариант
  (`RepoNotFound`), и неклассифицированный запасной (`CommandFailed`,
  несущий весь argv) не содержат секрета, но плоский URL называют.
- `a_rendered_argv_is_safe_to_log` — обе функции рендера argv.
- `a_credentialed_url_never_reaches_an_error_from_a_real_git` — сквозной,
  с настоящим git и без сети: `list_tags` на
  `https://x-access-token:SECRET-TOKEN-VALUE@127.0.0.1:1/o/r.git`, порт 1
  на loopback отказывает сразу. Текст ошибки не содержит секрета и
  называет хост. (Сам git свой stderr уже редактирует; тест закрепляет ту
  половину, которая наша.)
- `a_credentialed_read_that_misses_carries_no_token_into_its_error`
  (интеграционный) — буквально запрошенный координатором случай:
  credentialed GitHub-URL, 404 на `vibe.toml`, падение в git-путь; в
  ошибке секрета нет, на мок он пришёл заголовком `Bearer` и не попал в
  URL запроса.

## Замеры

Бинарник «до всех правок» сохранён как `<scratch>/vibe-before.exe`, так
что все три состояния сравнивались одним и тем же скриптом.

Холодный кэш во всех прогонах задаётся `VIBE_REGISTRY_CACHE` в пустом
каталоге. **Почему не удалял `~/.vibe/registries/<bucket>`**: переменная
даёт ровно то же холодное состояние, ничего не разрушая в кэше
владельца, и это тот же способ, которым снята цифра 116 с, — иначе
сравнение было бы с разной методикой.

### Фаза разрешения, холодный кэш

`<scratch>/repro-timed.sh` — тот же install, `stdin < /dev/null`, каждая
строка проштампована секундами от старта.

| состояние | до плана |
|---|---|
| до правок (`vibe-before.exe`) | **не дошло за 302 с** (убито таймаутом) |
| коммит 1 (клиент на вызов) | **116 с** |
| коммит 2 (один клиент) | **90 с** |

Дословно, до правок (`<scratch>/repro-timed-BEFORE.txt`):

```
--- install (timeout 300s), lines stamped with elapsed seconds ---
[3s] Resolving 1 root package…
EXIT=124 after 302s (124 = killed by timeout)
```

Коммит 1 (`<scratch>/repro-timed-AFTER.txt`):

```
--- install (timeout 300s), lines stamped with elapsed seconds ---
[3s] Resolving 1 root package…
[116s]   → 1 root, 26 transitive — 27 packages total
```

Коммит 2 (`<scratch>/repro-timed-AFTER2.txt`):

```
--- install (timeout 300s), lines stamped with elapsed seconds ---
[3s] Resolving 1 root package…
[90s]   → 1 root, 26 transitive — 27 packages total
EXIT=124 after 301s (124 = killed by timeout)
```

`EXIT=124` — это фаза установки после плана, а не разрешение.

### Соединения: что дал один клиент

`VIBE_LOG=debug`, обе трассы обрезаны по строку плана
(`<scratch>/repro-timed-DEBUG.txt` — коммит 1;
`<scratch>/repro-debug-after2dbg.log` — коммит 2):

| | коммит 1 | коммит 2 |
|---|---|---|
| чтений по HTTPS | 108 (прогон обрезан) | **243** (полная фаза) |
| `reuse idle connection` | **1** | **243** |
| `starting new connection` | 222 | 196 |
| запросов к индексу | 109 | 190 |
| запусков git | 0 | 5 |

Читается так: **каждое из 243 чтений теперь переиспользует соединение из
пула** (243 чтения — 243 переиспользования), тогда как раньше
переиспользование случилось ровно один раз на сотню чтений. Оставшиеся
196 новых соединений — это почти ровно 190 запросов к индексу:
`index_client` по-прежнему строит клиент на вызов, а индекс этого
реестра — статическое зеркало **на том же** `raw.githubusercontent.com`.
То есть рукопожатия, которые ещё остались, — целиком его, и это уже
другой файл (`index_client/mod.rs` вне периметра пакета).

Счётчики коммита 1 в первой редакции этого отчёта (80 чтений / 116
соединений) сняты, пока файл ещё дописывался обёрткой; полные итоги той
же трассы — 108 и 222, они и приведены выше.

### Дословно командой пакета

`bash <scratch>/repro.sh target/debug/vibe.exe 300`, холодный кэш —
`<scratch>/repro-AFTER2-literal.txt`:

```
--- install (timeout 300s) ---
Resolving 1 root package…
  → 1 root, 26 transitive — 27 packages total
EXIT=124 after 304s (124 = killed by timeout)
```

Тот же скрипт до правок печатал только `Resolving 1 root package…` и
умирал по таймауту, не дойдя до плана (`<scratch>/repro-BEFORE.txt`).
`EXIT=124` здесь — фаза установки 27 пакетов после плана; сам скрипт
секунд до плана не печатает, поэтому длительность разрешения снята
штампующим прогоном выше.

## Вывод гейтов (на дереве коммита 2)

1. `cargo fmt --all -- --check` — **красный по чужим файлам**: диффы
   только в том, что прямо сейчас правят параллельные воркеры в этом же
   ворктри. `cargo fmt --all` после появления их правок я намеренно **не**
   запускал. Проверка по периметру — чистая:

   ```
   $ rustfmt --edition 2024 --check <восемь моих файлов>
   scoped-rustfmt-exit=0
   ```

2. `cargo clippy -p vibe-registry --all-targets -- -D warnings` — зелёный:

   ```
       Checking vibe-registry v1.0.0 (C:\Users\olegc\git\v\vibevm-docs\crates\vibe-registry)
       Finished `dev` profile [unoptimized + debuginfo] target(s) in 7.41s
   ```

3. `cargo test -p vibe-registry` — зелёный, 295 тестов, ничего не
   пропускалось (сетевых тестов, требующих `--skip`, в крейте нет):

   ```
        Running unittests src\lib.rs
   running 234 tests
   test result: ok. 234 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 115.22s
        Running tests\https_file_read.rs
   running 9 tests
   test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
        Running tests\index_auth.rs
   running 8 tests
   test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
        Running tests\index_fast_path.rs
   running 8 tests
   test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
        Running tests\index_handshake.rs
   running 7 tests
   test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
        Running tests\index_search.rs
   running 7 tests
   test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s
        Running tests\index_url_ladder.rs
   running 3 tests
   test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
        Running tests\registry_cells_oracle.rs
   running 3 tests
   test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
        Running tests\store.rs
   running 2 tests
   test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
      Doc-tests vibe_registry
   running 14 tests
   test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.37s
   ```

4. `cargo xtask specmap --check` — **красный, дрейф не только мой**:

   ```
     drift: unbumped-hash: `spec://org.vibevm.core/vibevm/common/PROP-019#surface` content changed while the revision stayed at r3 — editorial, or forgot to bump? (bump `r`, or mark the commit body `spec-editorial: surface`)
     drift: units added: 1
     drift: edges added: 20
   ```

   Двадцать рёбер — мои (`scope!` и `#[verifies]` новых элементов). Строка
   `unbumped-hash` и `units added: 1` приходят из `vibevm/vibespecs/**`,
   которого я не касался ни в одном коммите (оба трогают только
   `crates/vibe-registry/**`). `specmap.json` пакет трогать запрещает.
   Байт-сравнение падает первым, поэтому gate разрешения адресов до
   правок бы не дошёл — проверил его отдельно и обратимо (копия в scratch,
   регенерация, чтение гейтов, возврат из копии; MD5 до и после совпадают,
   `c2aafa556122791d591563ef03245d32`):

   ```
   specmap: wrote …\specmap.json (8113 spec units, 3620 tagged code items, 3132 edges, 0 suspects, 31 warnings).
   specmap: ratchet gate — 0 gated orphan(s), 0 dispositioned (6 crate(s) exempt).
   specmap: resolve gate — 0 unresolved host edge(s), 35 non-host edge(s) outside this map's jurisdiction.
   ```

   Мёртвых якорей новые ссылки не вносят.

5. `cargo build -p vibe-cli` — зелёный (`Finished dev profile … in 31.05s`),
   далее `repro.sh` — см. «Замеры».

## Аномалии и находки

1. **HTTP 429 от `raw.githubusercontent.com` стоит целого клона.** В
   debug-трассе коммита 2 ровно одно чтение получило 429:

   ```
   https read did not answer with the file; falling back to git
     url=https://raw.githubusercontent.com/vibespecs/org.vibevm.world.dev-runtime-docs/v1.0.0/vibe-redirect.toml status=429
   ```

   Дальше по проекту сработала штатная лестница: `git archive` (отказ) →
   `clone` → `fetch` → `checkout` → `submodule update`, то есть пять
   запусков git и ≈20 с из наблюдённых 90 с. Поведение **корректно** — это
   ровно то, что задумано для неожиданного статуса, — но цена промаха
   высока, и без ретрая один 429 съедает пятую часть фазы. Решать, вводить
   ли короткий backoff-ретрай на 429/5xx, — не моя вводная; выношу как
   находку с уликой.

2. **Оставшиеся рукопожатия — целиком `index_client`.** Цифры в разделе
   «Соединения». Один переиспользуемый клиент для `index_client` (и,
   отдельно, параллельный веер — `PERF-RAYON-FANOUT` в PROP-002 §2.12 всё
   ещё `spec/done`, не `impl/done`) — очевидные следующие рычаги, оба вне
   периметра этого пакета.

3. **Ворктри делится с параллельными воркерами.** За время работы в нём
   успели поменяться `crates/vibe-cli/src/commands/vvm/**` (пакет POST-O2,
   уже закоммичен как `02d586e3`) и `crates/vibe-doc/src/style/**`.
   Последствия: workspace-гейт `cargo fmt --all -- --check` красный не по
   моей вине, а `cargo build -p vibe-cli` собирает чужой незавершённый код.
   Оба коммита перечисляют пути явно, поэтому чужого в них нет.

4. **Штампы времени в debug-прогоне коммита 1 лгали.** Обёртка ставила
   штамп через `date` в подпроцессе на каждую строку и на объёме
   debug-вывода отставала от процесса в разы, а заодно притормаживала сам
   процесс. Для коммита 2 debug-прогон пишется прямо в файл
   (`<scratch>/repro-debug.sh`), а время берётся из чистого прогона.
   По той же причине счётчики коммита 1 в первой редакции отчёта были
   частичными — исправлены выше.

5. **Один испорченный хвост вывода.** Первый timed-прогон я запустил, а
   затем переписал сам скрипт, пока bash его ещё читал; в конце файла
   появилась строка `dbook: command not found`. Нужное измерение (116 с)
   было снято до этого. Артефакт только в scratch.

6. **Утечка токена из первого отчёта — закрыта** коммитом 2 (см. выше).

## Что не сделано и почему

1. **Необязательная память об `ArchiveUnsupported`-хостах** — не сделана,
   сознательно. Приёмка требует, чтобы поведение для не-GitHub хостов не
   изменилось, а запоминание отрицательного сетевого ответа меняет именно
   его; хост, включивший `upload-archive` посреди процесса, обслуживался
   бы неверно до конца прогона.
2. **Ретрай на 429/5xx** — см. «Аномалии», п. 1: цена измерена, решение за
   боссом.
3. **Пул соединений для `index_client` и параллельный веер** — вне
   периметра, см. «Аномалии», п. 2.
4. **`specmap.json` не перегенерирован** — запрет пакета.
5. **`fetch.rs` не тронут**, трейт `GitBackend` не менялся, семантика
   `ArchiveUnsupported` не менялась, `Cargo.toml` крейта не менялся.

## Отклонений от пакета нет

Все три уточнения координатора реализованы: правило промаха привязано к
форме рефа (тест на ветку), быстрый путь оформлен таблицей хостов с
записью `gitverse.ru`, один клиент и вычистка учётных данных — вторым
коммитом с указанной подписью.
