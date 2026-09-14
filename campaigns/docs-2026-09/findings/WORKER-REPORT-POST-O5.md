# WORKER-REPORT-POST-O5

2026-09-14
Дерево: `4d1138ab` (ветка `research-preview-1-docs`, ворктри
`C:\Users\olegc\git\v\vibevm-docs`); база — `0bbdce15`.

Пакет: POST-O5 — норма о быстром пути чтения по HTTPS и адаптеры хостов
в своём файле (B-165).

## Коммиты

```
633856c2 docs(spec): say that a single file is read over HTTPS before git is asked
         4 files changed, 21 insertions(+), 9 deletions(-)
4d1138ab refactor(registry): keep the raw-read hosts in their own file
         3 files changed, 348 insertions(+), 321 deletions(-)
```

Оба — владелец репозитория, без трейлеров и без упоминания модели или
агента; формой `git commit -m … -- <пути>`, новый файл добавлен
`git add -- <файл>`. `git push` не выполнялся. `specmap.json` не
регенерировался. В том же ворктри параллельно работает web-воркер
(W1-O5); его файлы в `vibevm/vibepacks/org.vibevm.doc/web/**` не тронуты
и в мои коммиты не попали — `git status` после каждого коммита показывает
их по-прежнему изменёнными и неcкоммиченными.

## Коммит U — норма

### Точный id и текст факта

Id: `RAW-READ-FAST-PATH`. Место: `PROP-002-decentralized-registry.xml`,
§2.12 `<perf>`, отдельным `<p>` сразу после блока `<facts>` (то есть
после `PERF-FETCH-FILE`) и **перед** `RAW-READ-BACKOFF`.

Атрибуты — как у соседа и как велит пакет:
`fact="true" status="impl/done" action="continue" actionstage="doc"`,
**без `audience`**.

Текст — дословно из пакета, без единой правки:

> Before a single file is asked of git, the backend reads it over HTTPS
> from the host's own raw endpoint when the host is one it knows —
> `github.com` through `raw.githubusercontent.com`, `gitverse.ru` through
> its contents API — with any credential sent only as a bearer header,
> never in the address. A hit is the file. A miss is authoritative only
> for a tag or a commit SHA, whose content is fixed; on a branch, or for
> a manifest, the read falls through to git, whose answer stands as it
> always did. A host the table does not name never enters this path.

Каждое предложение проверено по коду: таблица `HOSTS` (две строки,
`github.com` → `raw.githubusercontent.com`, `gitverse.ru` → `contents`),
`request.bearer_auth(token)` в `attempt_read` при том, что `address`
строит URL без userinfo, `Attempt::Settled(Some(Ok(bytes)))` на 2xx,
`absence_is_authoritative` = «не манифест И неподвижный реф», и
`RepoCoords::parse`, который возвращает `None` для хоста вне таблицы.

### Что стало с `PERF-FETCH-FILE`

Было:

> `get_dependencies(pkgref, version)` → backed by
> `ShellGit::fetch_file_at_ref(repo_url, tag, "vibe-package.toml")` — a
> new method using `git archive` to pull a single file from a tag
> **without a working tree**.

Стало:

> `get_dependencies(pkgref, version)` → backed by
> `ShellGit::fetch_file_at_ref(repo_url, tag, "vibe-package.toml")` — a
> method that pulls a single file from a tag **without a working tree**.
> The HTTPS fast path is its first step; `git archive` is its second, and
> serves every read the fast path declines.

`git archive` больше не единственный и не первый шаг; ссылка на быстрый
путь дана описательно («the HTTPS fast path»), а не позиционно
(«below»), чтобы порядок фактов в секции можно было менять, не ломая
предложение.

### Ревизия

Секция несёт `req rN`, и она получила предложение, которого не было —
значит `perf-req` поднят `req r2` → `req r3`.

Семь рёбер `#[verifies(…PROP-002#perf, r = 2)]` переведены на `r = 3`:

- `raw_http/tests.rs` — `github_https_repositories_map_to_the_raw_host`,
  `gitverse_https_repositories_map_to_the_contents_api`;
- `tests/https_file_read.rs` —
  `github_manifest_is_read_over_https_without_git`,
  `github_absent_marker_at_a_tag_is_answered_without_git`,
  `gitverse_manifest_is_read_over_https_without_git`,
  `gitverse_reports_an_absent_marker_but_only_for_its_own_miss_code`,
  `a_rate_limited_read_asks_again_instead_of_paying_for_git`.

Перечитав тесты, добавил **два** новых ребра `#perf, r = 3` — ровно на те
два теста, которые проверяют последнее предложение нового факта («A host
the table does not name never enters this path») и до сих пор не несли ни
одного ребра на `#perf`:

- `raw_http/tests.rs::ssh_http_and_unknown_hosts_are_not_fast_path` —
  ssh в обеих записях, `http://`, `git+https://`, `file://`, чужой хост и
  хост, лишь оканчивающийся на знакомый (`notgithub.com`), не
  адресуются;
- `tests/https_file_read.rs::hosts_and_shapes_outside_the_fast_path_never_reach_the_read`
  — то же сквозь весь бэкенд: мок не видит ни одного запроса.

Больше рёбер не добавлял. Это тот же приём, которым POST-O4 добавил одно
ребро на тест, читающий новое предложение напрямую; здесь предложений в
факте четыре, и непокрытым ребром оставалось ровно одно.

### Ребро `implements`

Атрибутом на `try_read` (прецедент крейта — `#[spec(implements = …)]` на
функции, `progress-core::seal`), без `r`:

```rust
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#RAW-READ-FAST-PATH"
)]
pub(super) fn try_read(
```

**Почему без `r`.** Якорь факта минтит *untyped* юнит: в `mdspec.rs`
сказано «no `kind:`/revision line applies to a fact», и committed
`specmap.json` это подтверждает — `PROP-002#RAW-READ-BACKOFF` и
`PROP-002#PERF-FETCH-FILE` лежат там с пустыми `kind`/`revision`, тогда
как `PROP-002#perf` — `kind=req, revision=2`. Пин `r` на безревизионный
юнит даёт предупреждение `pin-into-unmarked-unit` (в карте таких уже 7
штук, все про `PROP-030#project-local`). Поэтому ревизию несёт секция, а
ребро на факт — нет.

## Коммит V — адаптеры хостов в `raw_http/hosts.rs`

Переехали: `RawHost`, `HOSTS`, `RepoCoords` (+ `parse`), `AddressedRead`,
`address`, `github_request_url`, `gitverse_request_url`,
`GitverseContents`, `GitverseError`, `gitverse_read_body`,
`gitverse_is_miss`, `GITVERSE_FILE_NOT_FOUND`, `decode_base64`,
`token_from_userinfo`, `is_plain`.

Остались в `raw_http.rs`: сам `try_read`, `Attempt`, `attempt_read`,
`transport_failure`, `absence_is_authoritative`, `names_fixed_content`,
`build_client`, `READ_TIMEOUT_SECS` — то есть чтение, правило «во что
верить» и клиент. Ни одна из этих трёх вещей не зависит от хоста, и
ничто в `hosts.rs` не открывает сокет и не решает, что означает ответ.

Видимость — идиома соседнего `retry.rs`: `pub(super)` на том, что читает
родитель или тесты (`RawHost` + поля `host`/`read_body`/`is_miss`,
`HOSTS`, `AddressedRead` + все поля, `address`, `decode_base64`);
`default_base`, `request_url`, `RepoCoords`, `is_plain`,
`token_from_userinfo` и оба хостовых композера остались приватными внутри
`hosts`.

**Тесты не переезжали.** Пакет говорит «переезжают вместе с кодом, если
лежат рядом» — они не рядом: в этом модуле один общий файл тестов
(`raw_http/tests.rs`), и тесты `retry.rs` живут именно в нём, обращаясь
`retry::…`. Сделал то же самое: `hosts::address`, `hosts::HOSTS`,
`hosts::decode_base64`. Весь дифф тестового файла — шесть строк
квалификации, ни один тест не переименован, не переставлен и не изменён:

```
-    address(None, repo_url, refname, path).map(|r| r.url)
+    hosts::address(None, repo_url, refname, path).map(|r| r.url)
-    address(None, repo_url, "v1.0.0", "vibe.toml").and_then(|r| r.token)
+    hosts::address(None, repo_url, "v1.0.0", "vibe.toml").and_then(|r| r.token)
-    let read = address(
+    let read = hosts::address(
-    let github = &HOSTS[0];
+    let github = &hosts::HOSTS[0];
-    let gitverse = &HOSTS[1];
+    let gitverse = &hosts::HOSTS[1];
+    use hosts::decode_base64;
```

**Доказательство, что это чистый переезд.** Сравнил построчно код «до»
(`633856c2:…/raw_http.rs`) с конкатенацией «после»
(`raw_http.rs` + `hosts.rs`), сняв комментарии и нормализовав
`pub(super) `. Различий ровно три, и все три — обвязка переезда:

```
=> use hosts::{AddressedRead, address};
=> mod hosts;
=> specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#perf");
```

Ни одной удалённой строки. Докстринги перенесены дословно; поправлены
только две внутридокументные ссылки, которые пересекли границу модуля
(`[`try_read`]` → `[`super::try_read`]`, `[`absence_is_authoritative`]` →
`[`super::absence_is_authoritative`]`). Новых зависимостей нет,
`Cargo.toml` не тронут.

## Гейты — дословно

`CARGO_TARGET_DIR` не переопределялся (в окружении пуст, всё против
общего `target/`). Прогон — на дереве второго коммита, `4d1138ab`.

```
$ cargo fmt --all -- --check
FMT-EXIT=0
```

(вывода нет)

```
$ cargo clippy -p vibe-registry --all-targets -- -D warnings
warning: C:\Users\olegc\git\v\vibevm-docs\crates\vibe-cli\Cargo.toml: unused manifest key: build
help: build is a valid .cargo/config.toml key
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.41s

CLIPPY-EXIT=0
```

(`unused manifest key: build` — состояние репозитория до меня, к
`vibe-registry` отношения не имеет)

```
$ cargo test -p vibe-registry
     Running unittests src\lib.rs (target\debug\deps\vibe_registry-36f425a0582d867a.exe)
test result: ok. 239 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 88.24s
     Running tests\https_file_read.rs (target\debug\deps\https_file_read-1341f5bc7c21d064.exe)
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests\index_auth.rs (target\debug\deps\index_auth-907a4af5fd95e44b.exe)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests\index_fast_path.rs (target\debug\deps\index_fast_path-8725aa60beb186f0.exe)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests\index_handshake.rs (target\debug\deps\index_handshake-cfa7ffeaef89a8ff.exe)
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
     Running tests\index_search.rs (target\debug\deps\index_search-9124402639ad4afc.exe)
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
     Running tests\index_url_ladder.rs (target\debug\deps\index_url_ladder-fe3f214920c7e059.exe)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
     Running tests\registry_cells_oracle.rs (target\debug\deps\registry_cells_oracle-0c9dd87b30619f6c.exe)
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
     Running tests\store.rs (target\debug\deps\store-4f86c11b1e84acc1.exe)
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.87s
   Doc-tests vibe_registry
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.38s
TEST-EXIT=0
```

239 + 13 + 8 + 8 + 7 + 7 + 3 + 3 + 2 + 14 = **304**, столько же, сколько
до правок (снял базовый прогон на `0bbdce15` — те же 304, зелёные).
Приёмка требует «304 и больше»: ни один тест не пропал, новых не
добавлялось — работа была про норму и про переезд, а не про поведение.

```
$ wc -l crates/vibe-registry/src/git_backend/shell/raw_http.rs crates/vibe-registry/src/git_backend/shell/raw_http/hosts.rs
  332 crates/vibe-registry/src/git_backend/shell/raw_http.rs
  336 crates/vibe-registry/src/git_backend/shell/raw_http/hosts.rs
```

(для полноты: `retry.rs` — 231, `tests.rs` — 478; до правок `raw_http.rs`
был 640, а не 605 — пакет цитировал замер POST-O4, а бэкофф с тех пор
дорос)

```
$ cargo xtask specmap --check
Error: `C:\Users\olegc\git\v\vibevm-docs\specmap.json` is out of date relative to the tree.
  drift: revision bump: `spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#perf` r2 → r3
  drift: units added: 1
  drift: edges added: 4
Run `rust-ai-native-specmap` (or your project's wrapper), review the drift, and commit the result.

SPECMAP-EXIT=1
```

Красный — ровно по тому, что пакет запретил чинить, и ровно в тех трёх
числах, которые он предсказывал:

- **бамп r2 → r3** — секция `#perf`;
- **units added: 1** — новый факт `RAW-READ-FAST-PATH` (карта его минтит,
  значит якорь живой и ребро `implements` не повиснет);
- **edges added: 4** — `implements` на `try_read`, два новых `verifies`
  и `specmark::scope!` нового файла `hosts.rs` (`scope!` даёт ребро
  `implements` на `#perf`; в committed-карте таких у этого модуля было
  три — `raw_http.rs`, `retry.rs`, `tests.rs`).

Регенерация — за боссом, как и после POST-O4 (`54038d89
chore(specmap): regenerate the map after the backoff fact`).

**Важно:** гейты `ratchet` и `resolve` (B-076) в этом прогоне не
отработали — движок падает на дрейфе раньше них. Проверил вручную, что
разрешение не сломается: `PROP-002#RAW-READ-BACKOFF` и
`PROP-002#PERF-FETCH-FILE` лежат в committed `specmap.json` как юниты, то
есть XML-читатель минтит юнит из такого элемента; «units added: 1»
подтверждает, что мой минтится так же. После регенерации оба гейта должны
быть зелёными, но снять это своими глазами я не мог, не перегенерировав
карту.

## Отклонений от пакета нет

Два коммита с заданными подписями, периметр соблюдён (`raw_http.rs`,
`raw_http/**`, `tests/https_file_read.rs`, PROP-002; `shell.rs` не
понадобился — `use` там не менялся, вызов `raw_http::try_read` остался
буквально тем же), поведение не менялось, зависимостей не добавлял,
`specmap.json` не трогал, `git push` не делал, чужих процессов не
останавливал.

Единственное решение сверх буквы — два добавленных ребра `verifies`
(раздел «Ревизия» выше). Пакет велит «переведи рёбра… перечитав тесты»;
перечитав, я нашёл предложение факта без единого ребра и два теста,
которые именно его и проверяют. Если босс считает это превышением —
откат ровно два блока `#[verifies]`.

## Аномалии и находки

**1. Новое предупреждение rustdoc, того же рода, что уже есть три раза в
этом модуле.** `cargo doc -p vibe-registry --no-deps
--document-private-items` (не гейт пакета, exit 0) даёт по крейту 21
предупреждение; среди них с 2026-09-14 было три про этот модуль:

```
warning: unresolved link to `retry`
--> crates\vibe-registry\src\git_backend\shell.rs:479:1
warning: unresolved link to `absence_is_authoritative`
--> crates\vibe-registry\src\git_backend\shell.rs:479:1
warning: unresolved link to `MAX_ATTEMPTS`
--> crates\vibe-registry\src\git_backend\shell\raw_http.rs:325:1
warning: unresolved link to `PAUSE_BUDGET`
--> crates\vibe-registry\src\git_backend\shell\raw_http.rs:325:1
```

Мой файл добавил четвёртое такое же:

```
warning: unresolved link to `super::absence_is_authoritative`
--> crates\vibe-registry\src\git_backend\shell\raw_http.rs:320:1
    = note: no item named `absence_is_authoritative` in module `shell`
```

Причина общая и не моя: ссылки, написанные во **внутренней** докуменции
подмодуля (`//!`), rustdoc здесь разрешает в области родителя, а не
самого модуля — поэтому у `retry.rs` не разрешаются даже его собственные
`MAX_ATTEMPTS` и `PAUSE_BUDGET`. Я оставил ссылочную форму ради
единообразия с соседом, которого положил POST-O4. Починка всех четырёх —
одна и та же однострочная замена `[` … `]` на простые обратные кавычки,
и делать её стоит разом, отдельным атомом, а не половиной внутри этого.

**2. Пакет цитировал 605 строк, в дереве было 640.** Цифра 605 — замер
POST-O4 на его собственном коммите; после него бэкофф дорос. На существо
это не влияет: бюджет 600 был перейдён в обоих случаях, а после переезда
файл — 332.

**3. `shell.rs` — 767 строк, тоже за бюджетом.** Был за бюджетом и до
POST-O1 (709 строк на момент отчёта POST-O4), к моему периметру не
относится. Естественный шов, если босс захочет: `without_userinfo` /
`without_userinfo_in_arg` / `render_argv*` — целая половина файла про
вычистку учётных данных из argv и ошибок.

**4. `audience` у нового факта снят — покрытие не проверял.** Пакет
велит «без `audience`: прозу и аудиторию даёт центральная сессия», и
POST-O4 показал дословно, что с `audience="user"` гейт `vibe doc check
--coverage` краснеет, пока страницы нет. Я `vibe doc check` не запускал —
он не в списке самопроверки и требует собранного `vibe.exe`; факт лежит
ровно в том состоянии, в котором `RAW-READ-BACKOFF` ждал своей страницы,
и `audience="user"` ему предстоит получить тем же атомом центральной
сессии, что и прозу.

## Что не сделано и почему

1. **`specmap.json` не перегенерирован** — прямой запрет пакета; весь
   дрейф описан выше и целиком мой.
2. **Проза руководства и `audience="user"`** — не мои, атом центральной
   сессии (прецедент: POST-O3 `09f647d7` → `10e3eab4`, POST-O4).
3. **Ссылки rustdoc в `raw_http.rs` / `retry.rs`** — находка 1, вне
   периметра и вне двух заданных коммитов.
4. **Живой замер** — пакет его не требовал, а поведение не менялось:
   переезд доказан построчным сравнением, норма — тестами, которые уже
   были.
