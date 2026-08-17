# Ф6.1 — поверхность хэндшейка: замер перед нарезкой

## 0. Как это мерилось

Чтение файлов — построчное, с координатами, снятыми в этом чекауте (ветка
рабочего каталога, состояние на момент замера). Инструменты и — для каждого,
чей пустой вывод читается ответом, — случай, которым он проверен (§0.8 пакета):

- **`ls schemas/` + `ls schemas/hello`** (У1): вторая команда вернула ошибку
  `No such file or directory` с кодом 2 — не пустоту, а явный отказ; пустой
  вывод здесь не участвовал.
- **`find formats/corpora -type f | sort`** (В3/В5, состав корпуса): нашёл 14
  файлов, включая `repomd.json` в корне корпуса. Контроль: тот же `find`
  перечисляет файлы в корне корпуса (значит, `hello.json` в корне появился бы
  в этом же выводе) и рекурсивно уходит в `state/journal/` (значит, обход
  полный, а не только верхний уровень).
- **Grep `TcpListener::bind|Router::new|spawn_mock|base_url`** по
  `crates/**/tests/**` (В3, полнота перечня mock-серверов): нашёл все семь
  mock-семейств в пяти файлах. Контроль: тот же шаблон находит каждый
  mock-сервер, который дерево строит этим идиомом (axum + TcpListener +
  mpsc), и положительно срабатывает на всех пяти; идиомы вне этого семейства
  искались отдельным Grep по `Mock|Fake|serve` — нашёл `seam_fakes.rs`
  (in-process фейки, не HTTP) и `MockSource` в vibe-install.
- **Grep `IndexClient|repomd` по `registry_cells_oracle.rs`** (В3): пусто.
  Контроль: тот же Grep по соседним `index_*.rs` даёт десятки попаданий —
  инструмент различает, пустота — факт про файл, не про шаблон.
- **Grep `atomic_write` + Grep `data_dir.join("`** (В1, полнота писателей
  data-dir): два независимых шаблона сошлись на одном и том же наборе
  писателей. Контроль: каждый шаблон положительно находит всех, кого видит
  второй; `journal/store.rs:6` прямо документирует, что НЕ использует
  `atomic_write` — и найден вторым шаблоном.
- **`wc -l` по четырём файлам `index_client`**: 533/223/300/91 — сверка чисел
  пакета.
- **`awk`-разметка `VIBEVM_INDEX_URL_*` по тестовым функциям** в
  `cli_search.rs`: каждая строка окружения привязана к имени функции.

Запрещённые действия пакета соблюдены: ни одной команды `git` не выполнено,
ни одна строка продуктового кода не изменена (периметр — `find` в отчёте
работника).

## 1. Утверждения пакета: подтверждено / опровергнуто

| # | утверждение | вердикт + цитата |
|---|---|---|
| У1 | `schemas/hello/` в дереве НЕ существует | **ПОДТВЕРЖДЕНО** — `ls schemas/hello` → `ls: cannot access 'schemas/hello': No such file or directory` (код 2). Содержимое `schemas/`: `index`, `journal`, 7 CLI-схем; `hello` нет. |
| У2 | `formats/REGISTRY.toml` несёт `[format.handshake]` со всеми шестью полями | **ПОДТВЕРЖДЕНО** — `formats/REGISTRY.toml:197-203`: `[format.handshake]` / `epoch = 1` / `schema = "schemas/hello/e1/hello.jtd.json"` / `recoverable = true` / `foreign_parsers = "many"` / `corpus = "none"` / `sunset = "none"` — все шесть дословно. |
| У3 | Пасс строгости пропускает запись по имени с явной строкой в выводе | **ПОДТВЕРЖДЕНО** — `xtask/src/codegen/strictness.rs:106-112`: `if !root.join(&entry.schema).is_file() { skips.push(format!("  - strictness: [format.{}] names schema `{}`, which no phase has built yet — no reader-strictness policy applied", …)); continue; }`; строки печатаются: `strictness.rs:80-82` `for skip in &skips { eprintln!("{skip}"); }`. Тест на этом литерале: `strictness/tests.rs:181` `record("handshake", "schemas/hello/e1/hello.jtd.json", "many")`. Нюанс: пропуск существует, пока файла схемы нет — как только Ф6.1 положит схему, строка пропуска исчезнет и запись начнёт ruled как `many` (пермиссивный читатель, штамп `deny_unknown_fields` не ставится — `strictness.rs:183-186`). |
| У4 | `Index::write_to(&self, data_dir: &Path, ctx: &WriteCtx)` в `memory.rs` около строки 197 | **ПОДТВЕРЖДЕНО** — `crates/vibe-index/src/index/memory.rs:197`: `pub fn write_to(&self, data_dir: &Path, ctx: &WriteCtx) -> Result<()> {`. Ровно строка 197. |
| У5 | `write_to` штампует `repomd.json` последним, «чтобы частичные виды всегда были согласованы» | **ПОДТВЕРЖДЕНО** — док-блок `memory.rs:190-193`: «Writes `primary.jsonl` and every `by-name/<name>.json` candidate set, then stamps `repomd.json` last **so partial views are always consistent** against an older manifest until the new one lands»; последняя операция тела — `memory.rs:302`: `repomd::write(data_dir, &manifest)`. |
| У6 | `repomd.json` — манифест с sha256 КАЖДОГО ДРУГОГО файла индекса | **ПОДТВЕРЖДЕНО** — карта собирается из каждого записанного файла: `memory.rs:218-285` (`files.insert(primary::FILENAME…)`, по одному `files.insert(written.relative_path…)` на каждый `by-name`/`by-cap`/`by-purl` файл + три директорные записи); поле — `types/repomd.rs:34`: `pub files: BTreeMap<String, RepomdFileEntry>` с доком «Path-keyed map of file or directory entries beneath the data directory (excluding `state/`)». Нюанс: «каждого другого **файла индекса**» — верно; вне карты остаются `repomd.json` (сам), `.gitignore`/`README.md` (пишет `init`, не `write_to`) и `state/**` (док в скобках). |
| У7 | Клиент собирает URL как `file_base` + `repomd.json`/`by-name/<name>.json`; `server_base` — для `/v1/...` | **ПОДТВЕРЖДЕНО** — `index_client/mod.rs:145`: `let url = format!("{candidate}/repomd.json");`; `:239`: `let url = format!("{}/by-name/{}.json", self.file_base, name);`; `:338`: `let base_url = format!("{}/v1/purls/", self.server_base);`; `:401`: `let url = format!("{}/v1/packages", self.server_base);`. Док полей — `mod.rs:46-56`. |
| У8 | `IndexClient::probe` сам определяет, живой сервер это или статический корень | **ПОДТВЕРЖДЕНО** — `mod.rs:144`: `for candidate in [format!("{trimmed}/v1/index"), trimmed.to_string()]` — проба `<base>/v1/index/repomd.json`, затем `<base>/repomd.json`; победивший кандидат становится `file_base` (`:149-153`), `server_base` — всегда голый `<base>` (`:151`). |
| У9 | Сервер отдаёт файлы индекса маршрутами из `routes/index_files.rs`, `repomd.json` — один из них | **ПОДТВЕРЖДЕНО** — `server/mod.rs:59-62`: `.route("/v1/index/repomd.json", get(routes::index_files::repomd_json))`; хендлер `index_files.rs:17-20`: `serve_file(&state.data_dir.join("repomd.json"), "application/json")`. Маршрута `hello.json` в таблице нет; произвольных путей сервер не раздаёт — axum на незнакомый путь отвечает 404. |
| У10 | `publish.rs` делает `git add -A` из корня data-dir; новый файл в корне уезжает тем же коммитом | **ПОДТВЕРЖДЕНО** — `publish.rs:78`: `run_git(data_dir, &["add", "-A"])?;` (запуск с `current_dir(data_dir)`, `:124-128`); док `:9-18`: «the data directory is itself the index's git working copy… `state/` is gitignored… and the rest is tracked and published». |
| У11 | `cli/init.rs` содержит прозаический перечень файлов индекса (около строк 115 и 139) | **ПОДТВЕРЖДЕНО** — `init.rs:114-119` (тело `.gitignore`): `# Index files (repomd.json, primary.jsonl[.gz],` / `# by-name/, by-cap/, by-purl/) are tracked;…` (строка 115); `init.rs:139-145` (тело `README.md`): `- \`repomd.json\` — manifest with sha256 of every other file.` (строка 139) и далее весь список до `state/`. Оба перечня устареют с появлением `hello.json` — это проза, не машинная проверка. |
| У12 | `cli/verify.rs` пересчитывает хэши и проверяет целостность `repomd.json` | **ПОДТВЕРЖДЕНО** — `verify.rs:89`: `let actual_sha = persistence::sha256_of_bytes(&bytes);`, `:91`: `if actual_size != *size || &actual_sha != sha256 { mismatches.push(…) }`; док `:1-2`: «recompute file hashes and check `repomd.json` integrity». |

Все двенадцать — ПОДТВЕРЖДЕНО; опровержений нет. Числа пакета по координатам
(`memory.rs:197`, строки 115/139 в `init.rs`, длины файлов `index_client`)
совпали с деревом точно.

## 2. Куда пишет write_to

**Полный перечень действий `write_to` в `data_dir`, в порядке выполнения**
(всё — `memory.rs:197-303`):

1. `memory.rs:198` — `std::fs::create_dir_all(data_dir)` — создать корень.
2. `memory.rs:210` — `clear_by_name(data_dir)?` — **снести каталог**
   `by-name/` целиком (`remove_dir_all`, `memory.rs:367-376`).
3. `memory.rs:211` — `inverted::clear_dir(&inverted::by_cap_dir(data_dir))?`
   — **снести** `by-cap/`.
4. `memory.rs:212` — то же для `by-purl/`.
5. `memory.rs:216` — `primary::write` — записать `primary.jsonl`
   (`primary.rs:44-59`), затем `primary.jsonl.gz` (детерминированный gzip,
   `primary.rs:66-74`); обе записи через `atomic_write` (tmp+fsync+rename,
   `persistence.rs:25-37`).
6. `memory.rs:218-226` — в карту `files` вставлены `primary.jsonl` и
   `primary.jsonl.gz` (размер + sha256).
7. `memory.rs:234-255` — для каждого имени — `by_name::write`
   (`by-name/<name>.json`, `by_name.rs:46-55`), каждая запись — в `files`;
   имя только с надгробием тоже получает файл (`:242-247`, закон «не молчать»).
8. `memory.rs:256-259` — в `files` вставлена директорная запись `by-name`
   (`RepomdFileEntry::directory(entry_count)`).
9. `memory.rs:263-277` — инвертированные виды: `by-cap/<slug>.jsonl`,
   `by-purl/<slug>.jsonl` (`inverted.rs:226,237`), каждая — в `files`.
10. `memory.rs:278-285` — директорные записи `by-cap` и `by-purl`.
11. `memory.rs:291-301` — собран `Repomd { schema_version, registry, …, files }`.
12. `memory.rs:302` — `repomd::write(data_dir, &manifest)` — **последняя**
    операция (`repomd.rs:27-30`, atomic_write в корень).

**Естественное место нового файла в КОРНЕ data-dir.** Если `hello.json`
попадает в манифест (развилка Р1-А ниже) — между шагами 10 и 11: после
последней вставки в `files` (`memory.rs:285`) и до сборки `Repomd`
(`memory.rs:291`); соседи — директорные записи `by-cap`/`by-purl` сверху и
сборка манифеста снизу, то есть та же «пишу файл → вставляю в карту»
последовательность, что у `primary`. Если вне манифеста (Р1-Б) — после
`memory.rs:302`, последней строкой тела; тогда инвариант «`repomd.json`
последним среди манифестных файлов» не нарушается вовсе.

**Как формируется карта `repomd.files`.** Перечислением известных писателей,
а НЕ обходом каталога: каждый писатель сам вставляет свою запись
(`memory.rs:219-226, 250-254, 266-276`). Обхода `data_dir` в `write_to` нет.
Следствие для `cli/verify.rs`: `verify.rs:78` — `for (rel_path, entry) in &manifest.files`
— verify ходит ТОЛЬКО по ключам карты и каталог не сканирует. Развилка для
босса, обе стороны с ценой в коде:
- **файл в корне ВНЕ карты** — verify его не видит: ни ошибки, ни проверки;
  статус остаётся `OK` (прецедент: `.gitignore`, `README.md`, `state/**`).
- **файл в карте** — verify пересчитает его sha256 (`verify.rs:89-99`) и
  сломается о любое расхождение; файл становится защищённым манифестом.

**Прецедент «файл в корне вне манифеста» — есть, но не у `write_to`.** Сам
`write_to` сегодня не пишет ничего вне карты (кроме `repomd.json`, который не
может хэшировать сам себя). Прецедент — `cli/init.rs`: `init.rs:110`
`.gitignore` и `init.rs:127` `README.md`, оба в корне data-dir, оба пишутся
`std::fs::write` только если отсутствуют (`:111-113`, `:128-130`), оба вне
`repomd.files`. Это и есть готовый образец для `hello.json` по варианту Р1-Б.

**Все, кто пишет в data-dir (не только `write_to`):**
- `Index::write_to` — каталожные файлы (выше).
- `cli/init.rs:109-124,126-168` — `.gitignore`, `README.md` (корень),
  `:84-95` — первая запись журнала (`state/journal/`).
- `journal/store.rs:43-45` — шарды `state/journal/<YYYY-MM>.ndjson`
  (`store.rs:30`: `data_dir.join("state").join("journal")`); вызывается из
  `cli/add.rs:151`, `cli/remove.rs:98`, `cli/reindex.rs:286`,
  `server/routes/packages.rs:425`.
- `index/checkpoint.rs:52,73-81` — `state/<checkpoint>` (инкрементальный
  реиндекс).
- `lock.rs:30-35` — `state/<lock>`.
- `scanner/org_cache.rs:126,152-161` — `state/<org-cache>`.
- `server/auth.rs:52` — `state/admin.tokens`.
- Серверные мутации: `server/routes/packages.rs:406-434` — replay журнала →
  fold → append записи → re-fold → `write_to`; то есть любой каталог после
  мутации переписывается целиком тем же `write_to`, и `hello.json`, будь он
  в `write_to`, уезжал бы каждой мутацией автоматически.

Итог: в КОРНЕ data-dir машинных файлов сегодня ровно пять: `repomd.json`,
`primary.jsonl`, `primary.jsonl.gz` (от `write_to`) + `.gitignore`,
`README.md` (от `init`, человеко-ориентированные). `hello.json` станет
шестым — первым с схемой, но не первым вне манифеста.

## 3. Что делает index_client до repomd.json

**Продуктовые точки входа в клиент — три, все через `probe`:**
1. `crates/vibe-registry/src/multi_registry_resolver/mod.rs:385` —
   `match crate::index_client::IndexClient::probe(&url, auth)` (путь
   установки/резолва; `Found` → `entry.with_index_client(client)`, `:386-388`).
2. `crates/vibe-cli/src/commands/search.rs:203` — `match IndexClient::probe(&base, auth)`.
3. `crates/vibe-cli/src/commands/search/purl.rs:69` — то же для PURL-поиска.

**Точная последовательность до первого чтения `repomd.json`** (на примере
резолвера): `index_url_for(reg.name)` (`mod.rs:510-518`, env
`VIBEVM_INDEX_URL_<REGISTRY>`) → `IndexAuth::for_registry(reg, &url)`
(`auth.rs:95-102` — решает план аутентификации) → `probe(base, auth)`
(`mod.rs:131`): строит reqwest-клиент с таймаутом 5 с (`:133-137`,
`build_client` `:475-489`), затем цикл по кандидатам `mod.rs:144-146`:

```rust
for candidate in [format!("{trimmed}/v1/index"), trimmed.to_string()] {
    let url = format!("{candidate}/repomd.json");
    match client.get(&url).send() {
```

Тело ответа **не парсится вовсе** — решает только статус (`:147`,
`resp.status().is_success()`); `repomd.json` сегодня для клиента —
доступность, не данные. Дальше: 200 → `Found` (`:149-153`); 401/403 →
`Refused { reason }` (`:157-163`); прочее → следующий кандидат; все
исчерпаны → `Absent` (`:177-178`).

**404 по `repomd.json` и варианты ошибок.** Внутри probe 404 — не ошибка, а
шаг к `Absent`. `IndexError` (enum — `mod.rs:69-102`) несёт РОВНО четыре
варианта:
- `Http { url, message }` — `mod.rs:72-77` (транспортный сбой);
- `Status { url, status }` — `mod.rs:78-83` (не-2xx на рабочем запросе);
- `AuthIncapable { url, regime, status }` — `mod.rs:84-95` (401/403 при
  режиме без HTTP-реквизита);
- `Malformed { url, message }` — `mod.rs:96-101` (битый JSON тела).

«Здесь индекса нет» — это НЕ `IndexError`: это `ProbeOutcome::Absent`
(`mod.rs:116`) у probe или `Ok(None)` у `list_versions` на 404
(`mod.rs:254-256`: `if status.as_u16() == 404 { return Ok(None); }`).
`IndexError` = «индекс есть, но сломан/недоступен». Обработчики `Absent`:
резолвер молча падает на `git ls-remote` (`multi_registry_resolver/mod.rs:400`:
`ProbeOutcome::Absent => {}`); CLI-search вносит в `registries_unreachable` с
текстом «probe of `{base}/repomd.json` failed (server down or wrong URL)»
(`search.rs:212-219`).

**Куда встанет чтение `hello.json` ПЕРЕД `repomd.json`.** Единственное место,
где клиент сегодня читает `repomd.json`, — цикл probe: `mod.rs:144` (сама
строка цикла) / между `:143` (клиент построен) и `:146` (GET repomd).
Вставка туда покрывает ВСЕ ТРИ продуктовых вызывателя разом — единая точка
входа есть, и это `probe`. Счёт мест: **одна функция** (плюс поле мира на
структуре `IndexClient`, `mod.rs:62-67`). Оговорка: конструкторы `at()` /
`at_with_auth()` (`mod.rs:189,201`) строят клиент БЕЗ probe — сегодня их
используют только тесты; если босс потребует хэндшейк на каждом пути, это
второй фронт, но требование D8 («читать перед repomd») точечно закрывается в
probe.

**Понятие СВОЕЙ эпохи у клиента — НЕТ.** Grep `schema_version|epoch` по
`crates/vibe-registry/src` даёт только UNIX-секунды (`git_registry.rs:302,310`,
`search/cache.rs:62`). Wire-виды клиента (`wire.rs:17-33`:
`NameEntryView`/`PackageEntryView`/`VersionEntryView`) читают только
`version`/`group`, никакой версии формата не сверяют. Значит Ф6.1 заводит
понятие «эпоха клиента» ВПЕРВЫЕ — это отдельная работа, и периметр строящего
шага обязан её назвать (константа + решение, что с ней сравнивать:
`hello.vibe`, `worlds[].epoch`, обе). Серверная `SCHEMA_VERSION`
(`memory.rs:29`) — это версия схемы каталога в `repomd.json`, другой
семейства число; клиент её сегодня тоже не читает.

**Статический корень против живого сервера, и где `hello.json` в каждом
режиме.** Probe пробует `<base>/v1/index/repomd.json` (живой сервер —
маршруты `/v1/index/*` из `server/mod.rs:59-82`), затем `<base>/repomd.json`
(статический корень сырых файлов). `file_base` после победы — либо
`<base>/v1/index`, либо `<base>`. Поэтому для `hello.json` справедлив ТОТ ЖЕ
джойн, что для repomd сегодня: `file_base + "/hello.json"`; на живом сервере
это `<base>/v1/index/hello.json` (маршрут придётся добавить — его в таблице
нет), на статическом зеркале — `<base>/hello.json` (файл туда положит
`write_to` + `publish.rs:78` `git add -A`).

**Цена лишнего запроса.** Константы: `PROBE_TIMEOUT_SECS = 5`
(`mod.rs:41`), `FETCH_TIMEOUT_SECS = 10` (`mod.rs:42`); клиент строится на
каждый вызов (`mod.rs:461-463`: «A fresh client per call preserves the
per-call timeout»), повторов нет — один `.send()` на URL. Худший случай
добавленного чтения hello: +1 GET с таймаутом 5 с на мёртвом URL (но там
probe и так падает); на живом индексе — +1 круговой запрос. Если hello
спрашивать на каждом кандидате цикла, probe станет до 4 запросов вместо 2 —
избежимо чтением hello только на базе-победителе.

## 4. Клиентские фикстуры и их судьба

Полнота получена Grep-обходом `crates/**/tests/**` по
`TcpListener::bind|Router::new|spawn_mock|base_url` (контроль — §0) плюс
просмотр всех тестовых файлов трёх крейтов. **HTTP-моки, изображающие
индекс (видимые клиенту):**

| # | фикстура | маршруты | что отвечает на неизвестный путь | класс |
|---|---|---|---|---|
| 1 | `crates/vibe-registry/tests/index_fast_path.rs` — `spawn_mock`, Router `:100-103` | `/repomd.json`, `/by-name/{name}` | 404 (явных маршрутов два, fallback нет) | **переживёт** |
| 2 | `crates/vibe-registry/tests/index_auth.rs` — `spawn_mock`, Router `:125-129` | `/repomd.json`, `/v1/packages`, `/by-name/{name}` | 404 | **переживёт** |
| 3 | `crates/vibe-registry/tests/index_search.rs` — `spawn_mock`, Router `:102-105` | `/v1/packages`, `/v1/purls/{purl}` (repomd-маршрута НЕТ) | 404; probe сюда не заходит — клиенты строятся `IndexClient::at()` (`:151,189,208,230,265,301,316`) | **переживёт** |
| 4 | `crates/vibe-cli/tests/cli_search.rs` — `spawn_mock`, Router `:278-282` | `/repomd.json`, `/v1/packages`, `/v1/purls/{purl}` | 404 | **переживёт** |
| 5 | `cli_search.rs` — `spawn_github_mock`, Router `:215-221` | `/orgs/{org}/repos`, `/repos/…/contents/vibe.toml` | 404; это GitHub-API, клиент индекса его не спрашивает | **не участвует** |
| 6 | `crates/vibe-index/tests/org_cache_e2e.rs` — `spawn_mock`, Router `:187` | GitHub-API (реиндекс) | 404 | **не участвует** |
| 7 | `crates/vibe-index/tests/from_github_e2e.rs` — `spawn_mock`, Router `:103` | GitHub-API (реиндекс) | 404 | **не участвует** |
| 8 | `crates/vibe-publish/tests/post_hook.rs` — `spawn_mock`, Router `:77-79` | POST `/v1/packages` (издательский хук) | 404; клиент индекса не читает | **не участвует** |
| 9 | `crates/vibe-index/tests/seam_fakes.rs` — in-process фейки `TokenStore`/`RateLimiter` (`:24-36`, `:38-61`) | не HTTP вовсе | — | **не участвует** |

Ни одна HTTP-фикстура не отвечает «200-мусором на любой путь»: все Router-ы
перечисляют маршруты явно, axum на прочее даёт 404 — фолбэк «нет хэндшейка →
сегодняшний путь» сработает против каждой из четырёх участвующих. Для
приёмки Ф6.1 («e2e против фикстур С hello») как минимум одна из №1–4 должна
научиться отдавать валидный `hello.json` — это правка фикстуры, не поломка.

**Дисковые фикстуры индекса.** Единственный закоммиченный образец каталога —
золотой корпус `formats/corpora/index/e1/` (14 файлов: `repomd.json`,
`primary.jsonl(.gz)`, `by-name/`×2, `by-cap/`×3, `by-purl/`×4,
`state/journal/`×2; `hello.json` НЕТ). Как только `write_to` начнёт писать
хэндшейк, корпус обязан нести его байты — тест корпуса байт-сравнивает
проекцию с коммитом (см. §6). Прочие «каталоги» в тестах создаются в
tempdir-ах через `init`/`write_to` на лету — класть в них hello руками не
придётся, он появится сам.

**Счёт тестов, упражняющих `index_client`:**
- юнит-тесты модуля: `index_client/tests.rs` — 5; `wire.rs` — 5; `auth.rs` —
  12 (итого 22);
- интеграционные `vibe-registry`: `index_fast_path.rs` — 5; `index_auth.rs` —
  8; `index_search.rs` — 7 (итого 20);
- `vibe-cli`: `cli_search.rs` — 15 `#[test]`, из них 8 направляют
  `VIBEVM_INDEX_URL_*` на индекс-мок: `search_aggregates_hits…` (`:320`),
  `search_dedup_keeps…` (`:390`), `search_reports_unreachable…` (`:461`),
  `search_filters_to_one_registry…` (`:522`), `search_kind_flag…` (`:578`),
  `search_caches_results…` (`:819`), `search_cache_ttl_zero…` (`:946`),
  `search_purl_lookup…` (`:1039`); ещё четыре трогают те же env-переменные
  для «ненастроенного» пути.
- `registry_cells_oracle.rs` — клиента не касается (Grep пуст, контроль §0).

## 5. Требования к схеме hello

**Обязательные аннотации, которых требует генератор** (правило «нет
аннотации — ошибка генерации» живёт в каждом пассе отдельно):

1. `metadata."x-empty": "omit"|"emit"` — на КАЖДОМ коллекционном поле
   (`elements`/`values` как член `properties`/`optionalProperties`);
   отсутствие — отказ: `empty_policy.rs:272-286` («carries no
   `metadata.\"x-empty\"`… is not derivable from the generated Rust»).
   Обязательное поле + `omit` — отдельный отказ R21 (`:303-315`): у
   required-коллекции единственный законный полис — `"emit"`. Пример
   употребления: `schemas/index/e1/by_name.jtd.json:18-20` —
   `"x-empty": "emit"` на `packages`.
2. `metadata."x-default"` — на каждом ОПЦИОНАЛЬНОМ СКАЛЯРНОМ поле: `null`
   (оставить `Option`) или булев литерал (схлопнуть в `bool`); отсутствие —
   отказ: `optional_shapes.rs:20-21` («a missing key is a generation
   error»). Опциональная СТРУКТУРА аннотации не требует (`:26-29`). Пример:
   `by_name.jtd.json:57-60` — `latest_stable` с `"x-default": null`.
3. `metadata."x-vocabulary": "open"|"closed"` — на каждом enum-узле;
   отсутствие — отказ: `open_vocabulary.rs:14-16`. У hello enum-ов нет —
   неприменимо, но правило названо.
4. `metadata."x-rust-type"` — необязательна (определение без неё легально,
   `domain_types.rs:32-33`); если стоит — форма должна классифицироваться
   (`:28-36`), путь обязан резолвиться без локальных импортов (P23, `:38-45`).
   Пример: `by_name.jtd.json:4` — корень `"x-rust-type": "NameEntry"`.
5. `metadata."x-vocabularies"` — только если схема тянет общие фрагменты из
   `formats/vocabularies.json` (реестр фрагментов: `package_kind`,
   `delivery_mode`, `naming_convention`, `group`, `version`, `timestamp` и
   12 записных); схема без подтягиваний живёт без ключа (`vocabulary.rs` —
   «the schema's own path when it declares no vocabularies»).
6. **Реестровая запись — обязательна**: схема, которую не называет ни один
   `[format.*]`, роняет генерацию: `strictness.rs:170-182` («no `[format.*]`
   record… names it»). Для hello запись уже есть (У2), и после появления
   файла строка пропуска исчезнет (У3); роль `many` → пермиссивный читатель
   без `deny_unknown_fields` (`strictness.rs:183-186`) — буквальное
   исполнение D8 «читается максимально терпимо».

**Таблица «узел → обязательная аннотация → почему» для формы А.6**
(`{"vibe":"hello/1","worlds":[{"epoch":1,"path":".","sunset"?}],"min_client"?,
"notice"?,"successor"?}`):

| узел | обязательная аннотация | почему |
|---|---|---|
| `vibe` (required string) | — (скаляр required) | аннотации требуют только коллекции/опциональные скаляры/enum-ы |
| `worlds` (required `elements`) | `x-empty: "emit"` | required-коллекция: R21 запрещает `omit`, отсутствие — отказ генерации |
| `worlds[].epoch` (required uint32) | — | required-скаляр |
| `worlds[].path` (required string) | — | required-скаляр |
| `worlds[].sunset?` (optional scalar) | `x-default: null` | опциональный скаляр без ключа — отказ генерации |
| `min_client?` (optional scalar) | `x-default: null` | то же; тип/доменный тип — развилка Р5 |
| `notice?` (optional scalar) | `x-default: null` | то же |
| `successor?` (optional scalar) | `x-default: null` | то же |
| корень | — | `x-vocabularies` не нужна, если ничего не подтягивается; `x-rust-type` по желанию |
| `definitions` для элемента `worlds` | — (описательная description) | конвенция всех схем (описания в metadata есть у каждого узла — стиль, не требование генератора) |

**Массив объектов — НЕ первый случай.** Образец формы:
`schemas/index/e1/by_name.jtd.json:14-17`:

```json
"packages": {
  "elements": { "ref": "package_entry" },
```

— массив объектов с локальным `definitions.package_entry` (`:30-62`);
`worlds[]` ляжет по той же колоде (плюс массивы объектов в CLI-отчётах:
`install_plan.jtd.json:19`, `list_report.jtd.json:29`). Факт для босса:
нового класса формы нет.

**Куда попадёт тип и имя модуля.** Правило раскладки: путь схемы зеркалится
от дома схем в generated-дерево — `layout.rs:92-98` («`<root>/<rel_dir>/<stem>.jtd.json`
→ `<out_dir>/<rel_dir>/<stem>/`»); сегменты имени проверяет сторож
`check_module_ident` (`layout.rs:144-174`): ASCII-буквы/цифры/`_`, начало —
буква или `_`, и не ключевое слово Rust (список `:132-138`). Проверка
законности: сегмент `hello` и стем `hello` — строчные буквы, проходят форму;
в списке ключевых слов `hello` отсутствует → **законны**; сегмент `e1`
законен (прецедент — `index/e1`, `journal/e1`). Ожидаемый путь:
`crates/vibe-wire/src/generated/hello/e1/hello/` (точный прецедент:
`schemas/journal/e1/journal.jtd.json` → `generated/journal/e1/journal/`),
плюс три синтезируемых `mod.rs` (`generated/mod.rs` gains `pub mod hello;`,
`generated/hello/mod.rs` — `pub mod e1;`, `generated/hello/e1/mod.rs` —
`pub mod hello;`). Вдобавок `FormatId::Handshake` уже сгенерирован из
реестра (`crates/vibe-wire/src/generated/format_id/mod.rs:31,93,119,145,171`)
— записи реестра трогать не нужно вовсе.

## 6. Что покраснеет

**Покраснеют (полный список, поимённо):**
- `crates/vibe-index/tests/golden_corpus.rs::the_catalog_is_the_projection_of_its_journal`
  (`:189`) — сравнивает проекцию с коммитом по объединению множеств
  (`:196-244`); как только `write_to` пишет `hello.json`, проекция несёт
  файл, которого в корпусе нет → вывод «`hello.json`: the projection writes
  this file, the committed corpus lacks it» (`:204-207`). Если hello входит
  в `repomd.files` — второе красное: байты `repomd.json` разъедутся с
  коммитом (`:213-241`). Дважды красный до обновления `WRITER_FILES`
  (`:58`) и байтов корпуса.
- `xtask/src/rebuild.rs` — тесты модуля: `a_projected_catalog_passes`
  (`:313`, `assert!(outcome.drift.is_clean())` `:318`),
  `an_extra_by_name_file_is_named` (`:330`, `:342`
  `assert!(outcome.drift.missing.is_empty())`), `a_flipped_byte_in_primary_is_named`
  (`:350`, `:366` то же), `state_dir_files_do_not_count` (`:375`, `:381`
  `is_clean`) — все четыре падают с `missing: ["hello.json"]`, пока
  `WRITER_FILES` (`rebuild.rs:35`) не вырастет до четырёх имён: сторона
  каталога — белый список (`:135-141`), сторона проекции — полный обход
  scratch (`:103-106`).
- `cargo xtask rebuild --check` на КАЖДОМ data-dir, включая корпус: прогон
  по корпусам переиспользует этот же движок (`wire_diff.rs:218`:
  `rebuild::run_rebuild(true, &root.join(dir))?`).

**Проверено и НЕ краснеет (проверял, а не предположил):**
- `cli_lifecycle.rs:31` `assert_disk_has_files(dir.path(), &["repomd.json", "primary.jsonl"])`
  — хелпер (`:259-264`) проверяет существование подмножества, не равенство
  множества;
- `cli_lifecycle.rs:146-152` и `memory/tests.rs:146-154` — `contains_key`/
  `matches!` на присутствие, не на полноту карты;
- `memory/tests.rs:361-377` `assert_trees_byte_identical` — обе стороны
  пишет один и тот же `write_to`, новый файл появляется в обоих деревьях;
- `wire_parity_repomd.rs:93` `files.len()` — против собственной константы
  `FILES_MAP_LEN = 2` (`:41`) на самодельной фикстуре, `write_to` не зовётся;
- `server_e2e.rs` / `rate_limit_e2e.rs` — про существующие маршруты, hello
  не спрашивают;
- `round_trip_published.rs`, `wire_parity_{entry,by_name,inverted,journal}.rs`
  — пишут/читают конкретные поверхности напрямую.

**Про золотой корпус:** сегодня 14 файлов, `hello.json` НЕ несёт (перечень —
§0). Для теста корпуса это обязательная новая строка контента: байты hello
должны быть детерминированной проекцией (файл — функция констант, не
`ctx.at`, либо с часом из того же `ctx.at` — развилка Р2).

**`rebuild --check`: hello — от писателя или от журнала?** ОТ ПИСАТЕЛЯ.
Журнал (`state/journal/*.ndjson`, события `Initialised`/`Published`/… —
`journal/store.rs`, `packages.rs:420-424`) фактов о мирах/эпохах не несёт
вообще; `hello.json` будет функцией состояния, которого нет в записях
(константа эпохи +, возможно, identity реестра — как сегодняшнее
`schema_version`, которое `write_to` берёт из state, а не из журнала:
`memory.rs:287-290`). Код сравнения устроен асимметрично: сторона проекции —
«scratch содержит только вывод write_to, поэтому обход целком и есть
поверхность писателя» (`rebuild.rs:103-106`), сторона каталога — белый
список `WRITER_FILES`+`WRITER_DIRS` (`:35-39`, `:135-141`), и док прямо
запрещает читать его как чёрный список (`:126-134`). Вывод: хэндшейк
попадёт в проекцию автоматически (достаточно, чтобы его писал `write_to`),
но в сравнение — только после расширения белого списка; без этого любая
проверка красна «missing hello.json» даже при наличии файла на диске.

## 7. Периметр строящего шага

**СОЗДАСТ:**
- `schemas/hello/e1/hello.jtd.json` — схема по таблице §5.
- `crates/vibe-wire/src/generated/hello/e1/hello/mod.rs` + два
  промежуточных `mod.rs` (`generated/hello/`, корневой обновится) —
  генерацией (`cargo xtask codegen`), не руками.
- `formats/corpora/index/e1/hello.json` — закоммиченные байты корпуса.
- e2e-тест приёмки ТЗ («клиент против фикстур с/без hello; отказной текст
  несёт рецепт и перечень миров») — новый тест-файл в
  `crates/vibe-registry/tests/` или расширение `index_fast_path.rs` (выбор
  строящего шага); фикстура «с hello» в нём.

**ИЗМЕНИТ (что именно):**
- `crates/vibe-index/src/index/memory.rs` — `write_to` пишет `hello.json`
  (место и членство в манифесте — развилки Р1/Р2 §9).
- `crates/vibe-index/src/server/routes/index_files.rs` +
  `crates/vibe-index/src/server/mod.rs` — хендлер + маршрут
  `/v1/index/hello.json` (живой сервер; статическим зеркалам хватает файла).
- `crates/vibe-registry/src/index_client/mod.rs` — чтение hello в `probe`
  перед GET repomd (`:144-146`), выбор мира своей эпохи, отказ с рецептом,
  первая в клиенте константа своей эпохи.
- `crates/vibe-index/src/cli/init.rs` — проза `.gitignore` (`:115`) и
  `README.md` (`:139-145`) пополняется `hello.json`.
- `crates/vibe-index/tests/golden_corpus.rs` — `WRITER_FILES` (`:58`).
- `xtask/src/rebuild.rs` — `WRITER_FILES` (`:35`).
- mock-фикстуры №1–4 из §4 — маршрут `/hello.json` для «с хэндшейком»-сценария.
- `formats/REGISTRY.toml` — НЕ меняется (запись уже есть, У2; пропуск
  строгости исчезнет сам, У3).

**СЛОМАЕТ (покраснеет без правки, список важнее двух первых):**
- `golden_corpus.rs::the_catalog_is_the_projection_of_its_journal`.
- `xtask` тесты: `a_projected_catalog_passes`, `an_extra_by_name_file_is_named`,
  `a_flipped_byte_in_primary_is_named`, `state_dir_files_do_not_count`.
- `cargo xtask rebuild --check` на всех data-dir и корпусах (включая
  прогон wire-diff по корпусам, `wire_diff.rs:218`).
- Никакой lib-код НЕ перестаёт компилироваться: все правки аддитивны;
  единственный риск компиляции — сигнатура `probe`, которую трогать не нужно.

**Команда замера разлёта компилятором:**
`cargo check --workspace --all-targets`. Почему ею: радиус шага лежит в
пяти крейтах (`vibe-index`, `vibe-registry`, `vibe-cli`, `vibe-wire`,
`xtask`), но главные поломки — в ТЕСТАХ и в xtask-целях, а не в lib-коде;
`--all-targets` компилирует тесты/бенчи вместе с lib, `--workspace`
гарантирует, что ни один косвенный потребитель `IndexClient`/`write_to` не
ускользнёт. Дополнить `cargo xtask check-codegen` — дифф генерации после
добавления схемы.

## 8. Расхождения с пакетом

Расхождений нет. Сверки, которые могли разойтись и сошлись: `write_to` —
ровно строка 197 (`memory.rs`); прозы `init.rs` — ровно строки 115 и 139;
длины четырёх файлов `index_client` — 533/223/300/91 (`wc -l`); форма А.6
совпадает с ТЗ дословно. Нюансы, не являющиеся расхождениями: у У3 строка
пропуска исчезает после появления схемы (свойство механизма, утверждение
пакета верно на сегодня); у У6 «каждого другого файла» верно в чтении
«каждого другого файла ИНДЕКСА» — `.gitignore`/`README.md`/`state/**` вне
карты и сегодня (документировано в самом типе, `types/repomd.rs:28-33`).

## 9. Открытые развилки для босса

1. **Членство `hello.json` в `repomd.files` (Р1).** Вариант A (в карте):
   verify проверяет целостность хэндшейка (`verify.rs:89-99`), строка README
   «sha256 of every other file» остаётся точной; цена — байты `repomd.json`
   меняются (корпус перекоммитить), место записи — до сборки манифеста
   (`memory.rs:285→291`). Вариант B (вне карты, как `.gitignore`/`README.md`):
   `repomd.json` не меняется вовсе, запись — последней строкой после
   `:302`; цена — verify слеп к хэндшейку, README-формула становится
   неточной, и «вечный файл» живёт без машинной проверки целостности.
2. **Кто и когда пишет `hello.json` (Р2).** Вариант A: `write_to` (как
   решено ТЗ §9 Ф6.1) — мутации/реиндекс/корпус получают файл бесплатно,
   `rebuild --check` сходится после расширения белого списка; цена — файл
   становится «проекцией» факта, которого в журнале НЕТ (прецедент уже есть:
   `schema_version` из state, `memory.rs:287-290`, но это одно число, а не
   файл). Вариант B: `init`-only, как README — цена: каждая мутация оставит
   старый hello, `rebuild --check` увидит missing, публикация расползётся с
   каталогом; по факту механизма дерева B не сходится с Ф3.2d, и это надо
   назвать вслух при выборе.
3. **Что такое «своя эпоха клиента» и где константа (Р3).** Вариант A:
   константа в `vibe-registry` рядом с клиентом + сравнение с
   `worlds[].epoch`; вариант B: сверка строки `vibe: "hello/1"` (версия
   самого хэндшейка) и отдельное поле мира. Цена разная радиусом: A — одна
   константа и одно сравнение; B — два семейства версий и два отказа.
   Сегодня у клиента нет НИ ТОГО НИ ДРУГОГО (§3) — шаг заводит понятие
   впервые.
4. **`sunset` в `worlds[]`: тип.** Вариант A: строка-дата (`type: string`) —
   дёшево, никакой аннотации кроме `x-default: null`; вариант B: фрагмент
   `timestamp` из `formats/vocabularies.json` (как `generated_at` в repomd,
   `repomd.jtd.json:22-24`) — типизовано, но тянет `x-vocabularies` и
   общий модуль `shared`.
5. **`min_client`: строка или доменный тип (Р5).** Вариант A: `type: string`
   — ничего сверх `x-default: null`; вариант B: `x-rust-type:
   "semver::Version"` по прецеденту А.5а (`domain_types.rs:16-20`,
   `chrono::DateTime`/`semver::Version` уже в употреблении) — тип на проводе,
   цена — зависимость генерации от rulings.
6. **Текст отказа «своей эпохи нет».** Вариант A: литерал по месту в
   клиенте; вариант B: рецепт из каталога рецептов (принцип ТЗ А.7 для
   карантина: «recipe — из каталога рецептов, не литерал по месту»).
   Стоимость A — нулевая сейчас и копеечная при смене; B — заводит каталог
   ради одного текста.
7. **Нужно ли чтение hello в `at()`/`at_with_auth()`-путях.** Сегодня они
   тестовые (§3); если босс хочет хэндшейк на КАЖДОМ использовании клиента,
   это второй фронт правки (два конструктора + кэш мира на структуре), если
   только probe — один. Внутри probe: читать hello на каждом кандидате
   (до 4 запросов) или только на победителе (2+1) — тоже выбор босса.
