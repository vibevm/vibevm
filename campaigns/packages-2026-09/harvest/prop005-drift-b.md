# PROP-005 — замер расхождений с деревом, сегмент B (строки 464–1345)

## Метод

Сегмент 464–1345 `vibevm/vibespecs/modules/vibe-index/PROP-005-package-index.xml` прочитан целиком;
каждое утверждение о коде, файлах, числах и поведении сверялось с деревом чтением
(`crates/vibe-index/**`, `schemas/`, `formats/`, `tools/self-check.sh`, оба `Cargo.toml`,
выборочно `vibe-registry`/`vibe-publish`/`vibe-cli`) и грепом; маршруты посчитаны по
факту сборки роутера в `crates/vibe-index/src/server/mod.rs` (`build_app`), а не по
документации; фенсед-блоки `##CLI-SURFACE`, `##DATA-DIR-LAYOUT`, `##design-crate-layout`
сверены в обе стороны; каждый ноль, прочитанный как доказательство, снабжён контрольным
ненулевым случаем тем же инструментом (раздел «Контроли пустого вывода»); git и cargo не
вызывались ни разу. Остальные факты сегмента, не вошедшие в находки, искались и нашлись
подтверждёнными — они перечислены в «Счёте».

## Контрольная таблица (§3 пакета)

| якорь | вердикт | цитата дерева (file:line) |
|---|---|---|
| 1. `##HTTP-API` (~468–500) | false | `crates/vibe-index/src/server/mod.rs:54-107` — роутер регистрирует 16 путей / 19 пар (метод, путь), среди них нет `POST /v1/admin/reindex`; `routes/admin.rs:28` — единственный обработчик `status` |
| 2. `##HTTP-ERRORS` (~506) | stale | `crates/vibe-index/src/server/error.rs:98-107` — `Body { kind, title, status, detail, unavailable }`: поля `instance` нет, расширение `unavailable` есть |
| 3. `##CLI-SURFACE` (~516–544) | false | `crates/vibe-index/src/cli/get.rs:21-22` — `data_dir` позиционный; `cli/mod.rs:88-97` — глобальный флаг только `--log-level`; греп `VIBE_INDEX` по крейту → единственный хит `scanner/git_cli.rs:19` (`VIBE_INDEX_GIT`) |
| 4. `##RUST-TYPES` (~552) | false | `crates/vibe-index/src/types/mod.rs:5-13` — «the generated wire types of `vibe-wire`, re-exported here»; `schemas/index/e1/` — 5 JTD-схем (`entry`, `repomd`, `by_name`, `by_cap`, `by_purl`) |
| 5. `##DATA-DIR-LAYOUT` (~589–609) | stale | `crates/vibe-index/src/index/by_name.rs:9-11` — «`by-name/<kind>/<name>.json`. `kind` left package identity, so the directory level is gone»; `journal/store.rs:29-31` — `state/journal/`; `scanner/org_cache.rs:122` — `state/org-cache.json`; `index/mod.rs:52-57` — `hello.json` в `WRITER_FILES` |
| 6. `##NEVER-SILENT-SCHEMA` (~654) | confirmed | `crates/vibe-index/src/index/quarantine.rs:43-58` (`missing_capabilities`/`is_usable` — отказ поимённо) и `:118-130` (`recipe_for` — с рецептом); `index/memory.rs:346-382` — карантинные версии остаются в `by_pkgref`, остальной каталог грузится |
| 7. `##A-MUTATION-THAT-CHANGES-NOTHING-COMMITS-NOTHING` (~867) | confirmed | `crates/vibe-index/src/server/routes/packages.rs:447-449` — «nothing changed: no record, no write, no publish»; `:301-317` — upsert по целочленному равенству, ответ всё равно успех |
| 8. `##design-crate-layout` (~978–1054) | stale | листинг `crates/vibe-index/`: есть `src/journal/` (6 файлов), `src/index/quarantine.rs`, `src/index/inverted.rs`, `src/publish.rs`, `src/lock.rs`, `src/server/rate_limit.rs`, `src/cli/rescan_org.rs`, `src/types/entry/`-каталог; нет `LICENSE`, `fixtures/sample-org/`, `fixtures/golden-index/`, `tests/cli_e2e.rs`, `tests/persistence_atomic.rs`, `tests/scan_clones.rs`; `Cargo.toml:7` — `license-file.workspace = true` |
| 9. `##VIBE-CORE-DEP` (~1080) | stale | `crates/vibe-index/Cargo.toml:26` — `vibe-core` (зависимость подтверждена); `scanner/manifest.rs:51,230` — `Manifest::parse_str` / `SubskillManifest::read`; но `tests/content_hash_parity.rs:69-79` — фикстуры `fixtures/golden-flow-wal-0.1.0` и `fixtures/golden-order-trap-0.1.0`, а не копия `fixtures/registry/flow/wal/v0.1.0/` |
| 10. `##GATE-COVERS` (~1231) | stale | `tools/self-check.sh:433-445` — шаг 1 `cargo fmt --all --check`, шаг 2 `cargo test --workspace`, шаг 3 clippy: «steps 1–2» из спеки указывают не на те шаги; особого случая для второго workspace vibe-index действительно нет (`:687-725` — наоборот, специальный index clock gate по `crates/vibe-index/src/{index,types,journal}`) |

## Расхождения — по одному подразделу на каждое

### 1. `##HTTP-API` (PROP-005:495) — класс: `false`

**Спека говорит** (дословно):
> POST   /v1/admin/reindex                          # body: { mode, source, args }

**Дерево говорит** (`crates/vibe-index/src/server/mod.rs:104-105`, дословно):
> // Admin (read-only in slice 5; reindex POST lands in slice 6).
> .route("/v1/admin/status", get(routes::admin::status))

`routes/admin.rs` (43 строки) содержит единственный обработчик `status`; маршрута
`POST /v1/admin/reindex` в сборке роутера нет вообще. Итого роутер регистрирует
**16 путей / 19 пар (метод, путь)**; блок спеки перечисляет 19 пар, включая
несуществующий reindex и не включая существующий `hello.json` (см. №2).

**Что верно ТЕПЕРЬ** (факт): админ-поверхность сервера — единственный маршрут
`GET /v1/admin/status`; запуск переиндексации по HTTP невозможен.

**Как проверить заново** (одна команда):
```
grep -rn "admin" crates/vibe-index/src/server/mod.rs crates/vibe-index/src/server/routes/admin.rs
```

### 2. `##HTTP-API` (PROP-005:470-500) — класс: `silent`

**Спека говорит** (дословно, полный список статических файлов блока):
> GET    /v1/index/repomd.json
> GET    /v1/index/primary.jsonl
> GET    /v1/index/primary.jsonl.gz
> GET    /v1/index/by-name/{name}.json
> GET    /v1/index/by-cap/{slug}.jsonl
> GET    /v1/index/by-purl/{slug}.jsonl

**Дерево говорит** (`crates/vibe-index/src/server/mod.rs:61`, дословно):
> .route("/v1/index/hello.json", get(routes::index_files::hello_json))

и `crates/vibe-index/src/server/routes/index_files.rs:17-20`:
> /// The eternal handshake `hello.json` — the file a client reads FIRST
> /// (PROP-044 `##ONE-ETERNAL-FILE`, TZ Р41), before any world's
> /// `repomd.json`

Маршрут `GET /v1/index/hello.json` — седьмой статический файл поверхности — блоком не
назван. (Суффиксы `.json`/`.jsonl` у `by-name`/`by-cap`/`by-purl` handlers требуют
дословно — `index_files.rs:85,101,117` — так что обслуживаемое URL-пространство
остальных шести строк совпадает со спекой.)

**Что верно ТЕПЕРЬ**: статическая поверхность — семь файлов, первой читается
`hello.json`, и её нет ни в таблице маршрутов §2.10, ни в дереве каталога §2.13.

**Как проверить заново** (одна команда):
```
grep -n "route(" crates/vibe-index/src/server/mod.rs
```

### 3. `##HTTP-ERRORS` (PROP-005:506-510) — класс: `stale`

**Спека говорит** (дословно):
> { "type": "vibe-index/error/integrity-mismatch", "title": "content_hash mismatch", "status": 409, "detail": "…", "instance": "/v1/packages/flow/wal/0.1.0" }

**Дерево говорит** (`crates/vibe-index/src/server/error.rs:98-107`, дословно):
> struct Body<'a> {
>     #[serde(rename = "type")]
>     kind: &'a str,
>     title: &'a str,
>     status: u16,
>     detail: &'a str,
>     #[serde(skip_serializing_if = "Option::is_none")]
>     unavailable: Option<&'a Unavailable>,
> }

Члена `instance` тело ошибки не несёт вовсе; вместо него — расширение `unavailable`
(RFC 7807 extension member, строка 404 остаётся). Само утверждение «RFC 7807
lightweight subset» верно.

**Что верно ТЕПЕРЬ**: тело ошибки — `{type, title, status, detail[, unavailable]}`;
`instance` не эмитируется ни одним обработчиком.

**Как проверить заново** (одна команда):
```
grep -n "instance\|unavailable" crates/vibe-index/src/server/error.rs
```

### 4. `##CLI-SURFACE` (PROP-005:516) — класс: `false`

**Спека говорит** (дословно):
> All subcommands accept `--data-dir <path>` (or use `$VIBE_INDEX_DATA_DIR`, default `./vibe-index-data`).

**Дерево говорит** (`crates/vibe-index/src/cli/get.rs:21-22`, дословно; одинаково во всех 15 подкомандах):
> pub struct Args {
>     pub data_dir: PathBuf,

`data_dir` — обязательный позиционный аргумент, не флаг; греп `VIBE_INDEX` по всему
крейту даёт единственный хит — `scanner/git_cli.rs:19`:
> std::env::var("VIBE_INDEX_GIT").unwrap_or_else(|_| "git".to_string())

то есть `$VIBE_INDEX_DATA_DIR` и умолчания `./vibe-index-data` в дереве нет вовсе
(контроль непустоты — в разделе «Контроли пустого вывода», п.1–2).

**Что верно ТЕПЕРЬ**: у каждого глагола `<data-dir>` — позиционный параметр;
переменных окружения `VIBE_INDEX_DATA_DIR` нет; единственная `VIBE_INDEX_*`-переменная
дерева — `VIBE_INDEX_GIT` (переопределение git-бинарной), которую спека не называет.

**Как проверить заново** (одна команда):
```
grep -rn "VIBE_INDEX\|data_dir: PathBuf" crates/vibe-index/src/cli/mod.rs crates/vibe-index/src/cli/get.rs crates/vibe-index/src/scanner/git_cli.rs
```

### 5. `##CLI-SURFACE` (PROP-005:518-544) — класс: `stale`

**Спека говорит** (дословно, заголовок блока):
> vibe-index <subcommand> [args] … All emit human-readable text by default; `--json` for machine-readable shape

**Дерево говорит** (`crates/vibe-index/src/cli/mod.rs:100-147` — перечисление `Command`; `:93-94` — глобальный флаг; поэлементная сверка):
- глаголы блока (14/14: init, dump, verify, reindex, get, list, search, capabilities,
  purls, outdated, add, remove, serve, stop) — все в дереве есть;
- в дереве есть, в блоке не названы: **`rescan-org`** (`cli/rescan_org.rs:29-53`;
  текст сегмента сам ссылается на него — PROP-005:712 «the explicit `rescan-org` verb
  of §2.8») и **глобальный флаг `--log-level`** (`cli/mod.rs:93-94`, off/error/warn/
  info/debug/trace);
- флаги в дереве есть, в блоке не названы: `init --force` (`cli/init.rs:42-43`),
  `reindex --api-base` (умолчание `https://api.github.com`), `--clone-cache`,
  `--cache-org` / `--no-cache-org` (`cli/reindex.rs:63-100`),
  `serve --rate-limit-per-token` / `--rate-limit-per-ip` (`cli/serve.rs:52-59`);
- элементы блока, которых в дереве нет: значение **`--format toml`** у `dump`
  (`cli/dump.rs:18-23` — enum `DumpFormat { Jsonl, Json }`, toml отсутствует);
  `--json` существует на 9 из 15 глаголов (нет на add, remove, dump, init, serve,
  stop) — «All … `--json`» читается как универсальное и универсальным не является;
- скобки `[--registry NAME --registry-url URL …]` подают флаги как необязательные,
  в дереве `--registry` и `--registry-url` обязательны (`cli/init.rs:24-29` —
  `pub registry: String` без умолчания).

**Что верно ТЕПЕРЬ**: 15 глаголов + глобальный `--log-level`; переиндексационное
семейство — `reindex` + `rescan-org` с кэшевыми флагами; `dump` без toml.

**Как проверить заново** (одна команда):
```
grep -n "pub enum Command" -A 50 crates/vibe-index/src/cli/mod.rs
```

### 6. `##RUST-TYPES` (PROP-005:552) — класс: `false`

**Спека говорит** (дословно):
> Rust types live in `crates/vibe-index/src/types/`, **hand-written against §2.6 rather than generated** — there is no JTD schema for them (measured 2026-08-05), so the text is the contract and the compiler checks nothing between them

**Дерево говорит** (`crates/vibe-index/src/types/mod.rs:5-13`, дословно):
> The shapes are the generated wire types of `vibe-wire`, re-exported
> here so every `vibe_index::types::*` path keeps its meaning while
> the definition lives once, beside the schemas it is generated from
> (PROP-000 §16). … Still hand-written and staying that way: `repomd`

и `crates/vibe-index/src/types/entry/aggregate.rs:13`:
> pub use vibe_wire::generated::index::e1::by_name::{NameEntry, PackageEntry, Tombstone};

плюс `schemas/index/e1/` содержит пять JTD-схем: `entry.jtd.json`, `repomd.jtd.json`,
`by_name.jtd.json`, `by_cap.jtd.json`, `by_purl.jtd.json` (поиск не вернул ноль —
контроль §0.7 не потребовался; см. также «Контроли», п.13). Паритет типов и схем
держат шесть тестов `tests/wire_parity_*.rs`. Ручным остаётся только `repomd`
(задокументированное исключение, открытое решение B-056: `u64` против сгенерированного
`u32`). Заметно, что `##SLICE-2` (PROP-005:1122) при этом утверждает «JTD schemas in
`schemas/`» — внутренний конфликт двух секций спеки решается деревом в пользу SLICE-2.

**Что верно ТЕПЕРЬ**: типы записи — сгенерированные из JTD типы `vibe-wire`,
реэкспортированные через `vibe_index::types`; JTD-схемы существуют; между текстом §2.6
и типами стоят схемы и паритет-тесты.

**Как проверить заново** (одна команда):
```
ls schemas/index/e1 && grep -n "generated" crates/vibe-index/src/types/mod.rs crates/vibe-index/src/types/entry/aggregate.rs
```

### 7. `##RUST-TYPES`-блок + `##PKGKEY-SHAPE` (PROP-005:554-581) — класс: `stale`

**Спека говорит** (дословно, фрагмент блока):
>     pub by_pkgref: BTreeMap<PkgKey, PackageEntry>,
>     pub by_capability: BTreeMap<String, BTreeSet<VersionedPkgKey>>,
>     pub by_purl: BTreeMap<String, BTreeSet<VersionedPkgKey>>,
>     pub text_index: TextIndex,
> }
>
> pub struct PackageEntry {
>     pub kind: PackageKind,
>     pub name: String,

и PROP-005:580:
> `PkgKey = (PackageKind, String)` — interned for cheap clones.

**Дерево говорит** (`crates/vibe-index/src/index/memory.rs:28-30,74-94`, дословно):
> pub type PkgKey = (Group, String);
> …
> pub struct Index {
>     …
>     pub by_pkgref: BTreeMap<PkgKey, PackageEntry>,
>     pub quarantined: Vec<Quarantined>,
>     pub tombstones: BTreeMap<String, Tombstone>,
> }

полей `by_capability` / `by_purl` / `text_index` у `Index` нет — инвертированные виды
строятся на записи (`inverted::InvertedView::from_entries`, `memory.rs:280`) и лениво
на запрос (`index/search.rs:1-9` — «built lazily per query»); `PackageEntry` несёт
`group` (`types/entry/aggregate.rs:4-6` — «gathers every indexed version of one
`(group, name)` identity (PROP-008 §2.2)»), которого в блоке нет; интернирования
ключа в дереве нет — это plain tuple.

**Что верно ТЕПЕРЬ**: `PkgKey = (Group, String)`, `kind` не ключует ничего;
`Index` = `by_pkgref` + `quarantined` + `tombstones`; поиск и инвертированные виды —
вычисляемые, не хранимые поля.

**Как проверить заново** (одна команда):
```
grep -n "pub type PkgKey\|pub struct Index" -A 20 crates/vibe-index/src/index/memory.rs | head -40
```

### 8. `##DATA-DIR-LAYOUT` (PROP-005:591-609) — класс: `stale`

**Спека говорит** (дословно, фрагмент блока):
> ├── by-name/
> │   └── <kind>/
> │       └── <name>.json
> …
> └── state/                            # NOT mirrored (gitignored when data-dir is a git working tree)
>     ├── server.lock                   # PID file, present only when serve is running
>     ├── admin.tokens                  # bearer tokens (gitignored)
>     ├── checkpoint.json               # incremental-reindex bookkeeping (last commit/tag per repo)
>     └── stats.json                    # counters for /metrics endpoint

**Дерево говорит** (`crates/vibe-index/src/index/by_name.rs:9-11`, дословно):
> Before PROP-008 the layer keyed on the package's own `kind`
> (`by-name/<kind>/<name>.json`). `kind` left package identity, so the
> directory level is gone — `<name>` alone is the key.

`crates/vibe-index/src/journal/store.rs:29-31`:
> pub fn default_dir(data_dir: &Path) -> PathBuf {
>     data_dir.join("state").join("journal")
> }

`crates/vibe-index/src/scanner/org_cache.rs:122`:
> `<data-dir>/state/org-cache.json` — next to `checkpoint.json`.

`crates/vibe-index/src/index/mod.rs:52-57`:
> pub const WRITER_FILES: [&str; 4] = [
>     "hello.json",
>     "repomd.json",
>     "primary.jsonl",
>     "primary.jsonl.gz",
> ];

Поэлементная сверка в обе стороны: (а) уровень `<kind>/` в `by-name/` исчез — блок
показывает старую ключёвку (сам же HISTORY-GROUP-NATIVE, PROP-005:1344, говорит об
этом как о свершившемся); (б) `state/stats.json` в дереве не существует — счётчики
`/metrics` живут в памяти (`server/state.rs`, AtomicU64; греп `stats.json` по крейту —
ноль, контроль — п.3 раздела «Контроли»); (в) в дереве есть, в блоке не названы:
`hello.json` в корне (четвёртый файл писателя, `mod.rs:52-57`, подаётся маршрутом
№2), `state/journal/<YYYY>-<MM>.ndjson` (правдоносный слой: сервер без него не
стартует — `cli/serve.rs:153-176`), `state/org-cache.json`; (г) `server.lock`,
`admin.tokens`, `checkpoint.json`, `README.md` — подтверждены (`lock.rs:21`,
`auth.rs:52`, `checkpoint.rs:16`, `cli/init.rs:126-171`).

**Что верно ТЕПЕРЬ**: корень каталога — `hello.json` + 3 файла блока + `README.md` +
`.gitignore`; `by-name/<name>.json` без уровня kind; `state/` — `journal/`,
`org-cache.json`, `server.lock`, `admin.tokens`, `checkpoint.json`; `stats.json` нет.

**Как проверить заново** (одна команда):
```
grep -rn "join(\"state\")\|WRITER_FILES\|stats.json\|org-cache" crates/vibe-index/src/index/mod.rs crates/vibe-index/src/journal/store.rs crates/vibe-index/src/scanner/org_cache.rs
```

### 9. `##ATOMIC-WRITE-PROTOCOL`/AW-FSYNC-DIR (PROP-005:618) — класс: `false`

**Спека говорит** (дословно):
> 4. @fact:AW-FSYNC-DIR `fsync(parent_dir(F))` on POSIX. (No-op on Windows where the directory has no fsync semantics; rename itself is atomic.) @status:impl/done

**Дерево говорит** (`crates/vibe-index/src/index/persistence.rs:25-37`, дословно — весь протокол):
> pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
>     if let Some(parent) = path.parent() {
>         fs::create_dir_all(parent).map_err(|e| io_err(parent, e))?;
>     }
>     let tmp = tmp_sibling(path);
>     {
>         let mut f = fs::File::create(&tmp).map_err(|e| io_err(&tmp, e))?;
>         f.write_all(bytes).map_err(|e| io_err(&tmp, e))?;
>         f.sync_all().map_err(|e| io_err(&tmp, e))?;
>     }
>     fs::rename(&tmp, path).map_err(|e| io_err(path, e))?;

шага «fsync каталога на POSIX» нет ни здесь, ни где-либо в крейте: все четыре
call-site'а `sync_all` в `src/` — файловые (греп приведён в «Контролях», п.4-bis).
Шаги 1–3 (tmp → fsync → rename) — подтверждены; временный файл называется
`<F>.tmp.<pid>`, а не `F.tmp`, что сути контракта не меняет.

**Что верно ТЕПЕРЬ**: атомарная запись — tmp-sibling + fsync(файла) + rename;
fsync каталога не делается ни на POSIX, ни где-либо ещё.

**Как проверить заново** (одна команда):
```
grep -rn "sync_all\|sync_data" crates/vibe-index/src
```

### 10. `##REPOMD-LAST-LAW` (PROP-005:620) — класс: `stale`

**Спека говорит** (дословно):
> `repomd.json` is replaced **last** in any batch update, so a reader that fetches `repomd.json` first then chases hashes always sees consistent files.

**Дерево говорит** (`crates/vibe-index/src/index/memory.rs:319-326`, дословно):
> repomd::write(data_dir, &manifest)?;
>
> // The eternal handshake lands last, after the manifest, and
> // stays OUTSIDE its `files` map (Р39): `repomd.json` is the
> // manifest of one world, while the handshake stands above
> // worlds and dispatches to them.
> write_handshake(data_dir)

`repomd.json` пишется предпоследним; последним — `hello.json` (в `files`-карту
манифеста он не входит, так что погоня за хэшами из repomd по-прежнему консистентна —
но буквальное «replaced last in any batch update» перестало быть верным).

**Что верно ТЕПЕРЬ**: порядок записи — файлы данных → `repomd.json` → `hello.json`.

**Как проверить заново** (одна команда):
```
grep -n "repomd::write\|write_handshake" crates/vibe-index/src/index/memory.rs
```

### 11. `##THREAD-WRITER-TASK` (PROP-005:1100) — класс: `false`

**Спека говорит** (дословно):
> Disk writes serialised through a single dedicated tokio task: `index_writer`. The server posts mutations to it via an mpsc channel; the writer applies them in order.

**Дерево говорит** (`crates/vibe-index/src/server/routes/packages.rs:440-469`, дословно, вся модель записи):
> let mut idx = state.index.write().await;
> let journal_dir = default_dir(&state.data_dir);
> let mut records =
>     replay(&journal_dir).map_err(|e| journal_refused("could not read the journal", e))?;
> let mut probe = project(records.iter().cloned())
> …
> let persisted = fresh.write_to(&state.data_dir, &WriteCtx { at });
> *idx = fresh;

задачи `index_writer` и mpsc-канала в крейте нет (греп — ноль; контроль — п.5
«Контролей»): каждый мутирующий обработчик сам, под async write-lock
(`tokio::sync::RwLock`, `server/state.rs:18` — эта половина THREAD-SERVER-ASYNC
подтверждена), делает replay → project → append → write_to. Единственный
`spawn_blocking` — публикационная ветка (`packages.rs:543`). Смежное
THREAD-CLI-SYNC (PROP-005:1098) — подтверждено: `tokio::runtime` строится только в
`serve` (`cli/serve.rs:103` — единственный hit по крейту).

**Что верно ТЕПЕРЬ**: выделенной writer-задачи нет; запись идёт синхронно в обработчике
под write-lock; сериализация публикаций — отдельным `publish_lock` + `spawn_blocking`.

**Как проверить заново** (одна команда):
```
grep -rn "mpsc\|index_writer\|spawn_blocking\|tokio::runtime" crates/vibe-index/src
```

### 12. `##dep-prometheus` (PROP-005:1078) — класс: `false`

**Спека говорит** (дословно):
> - @fact:dep-prometheus `prometheus` — `/metrics` endpoint. @status:impl/done

**Дерево говорит** (`crates/vibe-index/src/server/metrics.rs:1-3`, дословно):
> //! Plain-text Prometheus metrics serialiser. We do not pull a
> //! prometheus crate for slice 5; the surface is small enough to roll
> //! by hand and keep the dep tree minimal.

`crates/vibe-index/Cargo.toml:24-50` перечисляет все зависимости — `prometheus` среди
них нет (таблица прочитана целиком, ноль — не слепота инструмента; см. «Контроли»,
п.6). Эндпоинт `/metrics` существует (`server/mod.rs:107`) и рендерит текстовый формат
руками (`metrics.rs:72-78`), включая `vibe_index_publish_failures_total` (`:57`).

**Что верно ТЕПЕРЬ**: `/metrics` — самописный текстовый сериализатор; зависимости
`prometheus` нет.

**Как проверить заново** (одна команда):
```
grep -n "prometheus" crates/vibe-index/Cargo.toml crates/vibe-index/src/server/metrics.rs
```

### 13. `##deps` (PROP-005:1060-1078) — класс: `stale`

**Спека говорит** (дословно, начало):
> @fact:deps-lead Minimal Rust crates to keep redistribution clean:

**Дерево говорит** (`crates/vibe-index/Cargo.toml:24-50`): из перечисленных в §3.2
семнадцати позиций шестнадцать подтверждены (clap, tokio, axum, tower/tower-http,
serde/serde_json, toml, semver, sha2, flate2, walkdir, tracing(+subscriber), chrono,
thiserror, shell-out вместо gix, reqwest, tempfile; `prometheus` — см. №12). В дереве
есть, в списке нет: **`specmark`** (`:25` — маркировка спек-фактов в коде) и
**`vibe-wire`** (`:50` — сгенерированные wire-типы, см. №6). `tokio` подключён не
«full», а `features = ["signal", "sync", "time", "fs"]` (`:41`) — @fact:dep-tokio
«tokio (full)» неточен.

**Что верно ТЕПЕРЬ**: 18 runtime-зависимостей; против списка §3.2 — минус prometheus,
плюс specmark и vibe-wire, tokio с узким набором features.

**Как проверить заново** (одна команда):
```
sed -n '24,63p' crates/vibe-index/Cargo.toml
```

### 14. `##CONFIG-PRECEDENCE` (PROP-005:1106) — класс: `false`

**Спека говорит** (дословно):
> For every flag with a default, precedence is: explicit CLI flag > env var (`VIBE_INDEX_*`) > on-disk config (`<data-dir>/state/config.toml`, optional) > built-in default.

**Дерево говорит**: греп `config.toml` по `crates/vibe-index/` — ноль (контроль —
п.2 «Контролей»); семейства переменных `VIBE_INDEX_*` нет (единственная переменная
этого префикса — `VIBE_INDEX_GIT`, `scanner/git_cli.rs:19`, о которой спека молчит);
файлового конфига и машины приоритетов в дереве не существует. Флаги со значениями по
умолчанию (`--bind`, `--naming`, `--api-base`, `--rate-limit-*`) получают их напрямую
из clap-деклараций, без env- и файл-уровней.

**Что верно ТЕПЕРЬ**: приоритетов четыре уровня нет; есть флаг → clap-default и один
env-оверрайд git-бинарной, спекой не названный.

**Как проверить заново** (одна команда):
```
grep -rn "config.toml\|VIBE_INDEX" crates/vibe-index/src
```

### 15. `##TEST-HERMETIC` (PROP-005:1217) — класс: `false`

**Спека говорит** (дословно):
> A separate `cli_live_e2e.rs` (`#[ignore]`-d, opt-in via `cargo test -- --ignored`) walks `--from-github vibespecs` against the real registry to confirm the API walk works against actual infrastructure.

**Дерево говорит**: файла `tests/cli_live_e2e.rs` нет (полный листинг `tests/` — 24
файла, среди них его нет; контроль — п.7 «Контролей»); `#[ignore]` в тестах крейта
встречается один раз и не про сеть — `tests/content_hash_parity.rs:276`:
> #[ignore = "constructing a non-UTF-8 path needs raw OsString bytes (std::os::unix::ffi); \

живой прогон по GitHub в дереве заменён герметичным
`tests/from_github_e2e.rs:1-5`:
> //! … a mock GitHub REST API runs in a background thread on a random
> //! port; the canned responses point at local bare repositories so
> //! `git clone` resolves entirely against the filesystem. No network
> //! access required.

**Что верно ТЕПЕРЬ**: opt-in живого теста против реального реестра не существует;
GitHub-прогон покрыт мок-сервером без сети.

**Как проверить заново** (одна команда):
```
ls crates/vibe-index/tests && grep -rn "ignore" crates/vibe-index/tests
```

### 16. `##TEST-INTEGRATION` (PROP-005:1213) — класс: `stale`

**Спека говорит** (дословно):
> full-reindex against `fixtures/sample-org/` produces a byte-identical `primary.jsonl` to `fixtures/golden-index/primary.jsonl`.

**Дерево говорит**: ни `fixtures/sample-org/`, ни `fixtures/golden-index/` в крейте
нет — листинг даёт `fixtures/golden-flow-wal-0.1.0/` и `fixtures/golden-order-trap-0.1.0/`
(контроль — п.8); золотой корпус форматов живёт в `formats/corpora/index/`
(`formats/: EPOCHS.toml, REGISTRY.toml, breaks/, corpora/, hash_recipes/,
vocabularies.json`), а роль эталонного прогона несут `tests/golden_corpus.rs` и
`cargo xtask rebuild --check`/`wire-diff` (последние — по `tools/self-check.sh:490-505`).

**Что верно ТЕПЕРЬ**: эталоны — `fixtures/golden-flow-wal-0.1.0/`,
`fixtures/golden-order-trap-0.1.0/` и корпус `formats/corpora/index/`; путей из §5 в
дереве нет.

**Как проверить заново** (одна команда):
```
ls crates/vibe-index/fixtures formats/corpora
```

### 17. `##INT-PUBLISH-HOOK` + `##SECRET-INDEX-TOKENS` + `##SLICE-9` (PROP-005:633, 1243, 1187) — класс: `stale`

**Спека говорит** (дословно, PROP-005:633):
> if the registry has an `index_url` configured AND a `[[registry]].index_token` (env: `VIBEVM_INDEX_TOKEN_<HOST>`), Publisher POSTs the new entry to `<index_url>/v1/packages`

и PROP-005:1187:
> New env var `VIBEVM_INDEX_TOKEN_<HOST>`. New `[[registry]].index_url` / `[[registry]].index_token` fields in the project manifest.

**Дерево говорит** (`crates/vibe-publish/src/post_hook.rs:5,49,57-62`, дословно):
> //! `VIBEVM_INDEX_URL_<REGISTRY>` and `VIBEVM_INDEX_TOKEN_<REGISTRY>`
> …
> pub fn index_url_for(registry: &str) -> Option<String> {
> …
>     std::env::var(format!("VIBEVM_INDEX_TOKEN_{suffix}")).ok()

POST-механика подтверждена (`post_hook.rs:295-301` — «POST `payload` to
`<config.index_url>/v1/packages` with bearer»), но суффикс переменных — `<REGISTRY>`,
не `<HOST>`; полей `[[registry]].index_url` / `index_token` в манифесте нет — греп
`index_url|index_token` по `crates/vibe-core/src` даёт ноль (контроль — п.9
«Контролей»); источник обеих величин — только окружение.

**Что верно ТЕПЕРЬ**: хук настраивается парой env-переменных
`VIBEVM_INDEX_URL_<REGISTRY>` / `VIBEVM_INDEX_TOKEN_<REGISTRY>`; манифестных полей нет.

**Как проверить заново** (одна команда):
```
grep -rn "VIBEVM_INDEX\|index_url\|index_token" crates/vibe-publish/src/post_hook.rs crates/vibe-core/src
```

### 18. `##wire-up-not-shipped` (PROP-005:1336) — класс: `stale`

**Спека говорит** (дословно):
> These live in `crates/vibe-index/docs/operator-handbook.md` rather than as shipped binaries — operators integrate at their own host…

(**These** = git-хук `post-receive` из `##WIRE-POST-RECEIVE` и cron-строка из
`##WIRE-CRON`.)

**Дерево говорит**: `crates/vibe-index/docs/operator-handbook.md:90` несёт cron-строку
(`*/5 * * * *  vibe-index reindex /home/owner/vibespecs-index \`) — а скрипта
`post-receive` в справочнике нет: греп `hook|post-receive|curl` по файлу даёт только
прозу про post-publish hookslice-9 (`:61`, `:111`); контроль — п.10 «Контролей».

**Что верно ТЕПЕРЬ**: из двух «wire-up» артефактов в справочнике живёт только
cron-строка; хук `post-receive` не документирован нигде в `docs/`.

**Как проверить заново** (одна команда):
```
grep -in "post-receive\|hook\|cron\|\*/5" crates/vibe-index/docs/operator-handbook.md
```

### 19. `##design-crate-layout` (PROP-005:978-1054) — класс: `stale`

**Спека говорит** (дословно, фрагменты блока):
> ├── LICENSE                                 # EULA (vibevm's proprietary license)
> …
> │   ├── index/
> │   │   ├── mod.rs                          # Arc<RwLock<Index>>
> …
> ├── fixtures/
> │   ├── sample-org/
> …
> │   └── golden-index/
> │       ├── repomd.json
> │       └── primary.jsonl
> ├── tests/
> │   ├── help_smoke.rs                       # clap renders help for every subcommand
> │   ├── cli_e2e.rs                          # init + reindex + get + search round-trips
> │   ├── server_e2e.rs                       # spawn server, drive HTTP API, shut down
> │   ├── persistence_atomic.rs               # crash-mid-write recovery
> │   ├── content_hash_parity.rs              # hash matches vibe-registry's exactly
> │   └── scan_clones.rs                      # walks fixtures/sample-org/

**Дерево говорит** (полный листинг `find crates/vibe-index -type f`; структурно, в обе
стороны):
- в дереве есть, в блоке нет (структурные поверхности): **модуль `src/journal/`**
  (`mod.rs`, `record.rs`, `store.rs`, `project.rs` + тесты) — правдоносный слой
  журнала; **`src/index/quarantine.rs`** и **`src/index/inverted.rs`**; **`src/publish.rs`**
  (авто-публикация §2.17); **`src/lock.rs`**, **`src/lockfile.rs`**, **`src/hash_recipe.rs`**,
  **`src/error.rs`**; в `scanner/` — **`git_cli.rs`**, **`manifest.rs`**,
  **`org_cache.rs`**, **`org_walk.rs`**; **`src/server/rate_limit.rs`**; в `cli/` —
  **`rescan_org.rs`**, **`kinds.rs`**; `types/entry.rs` стал каталогом
  **`src/types/entry/`** (`aggregate.rs`, `content.rs`, `relations.rs`); в `tests/` —
  `auto_publish.rs`, `cli_lifecycle.rs`, `cli_read.rs`(+`cli_read/unavailable.rs`),
  `cli_write.rs`, `from_github_e2e.rs`, `golden_corpus.rs`, `org_cache_e2e.rs`,
  `rate_limit_e2e.rs`, `round_trip_published.rs`, `scanner_e2e.rs`(+`journal_form.rs`),
  `seam_fakes.rs`, `server_writes.rs`, `server_e2e/unavailable.rs` и шесть
  `wire_parity_*.rs`;
- в блоке есть, в дереве нет: **`LICENSE`** («EULA») — файла в крейте нет,
  `Cargo.toml:7` — `license-file.workspace = true`, корневой `LICENSE.xml:3` — «The
  Universal Permissive License (UPL), Version 1.0»; **`fixtures/sample-org/`** и
  **`fixtures/golden-index/`** (см. №16); тесты **`cli_e2e.rs`**,
  **`persistence_atomic.rs`**, **`scan_clones.rs`** (их роль несут
  `cli_lifecycle.rs`/`cli_read.rs`/`cli_write.rs`, crash-теста с фейлом rename нет);
- подтверждены: `docs/{operator-handbook,consumer-protocol,format}.md`,
  `tests/{help_smoke,server_e2e,content_hash_parity}.rs`, `README.md`, отсутствие
  `[workspace]`-таблицы в `Cargo.toml` крейта, `src/content_hash.rs`,
  `src/types/kinds.rs`.

**Что верно ТЕПЕРЬ**: крейт вырос на четыре корневых модуля (journal, publish, lock,
hash_recipe) и ~15 файлов против блока; лицензия крейта — UPL-1.0 через workspace, без
собственного LICENSE-файла; тестовый набор переименован и дополнен.

**Как проверить заново** (одна команда):
```
find crates/vibe-index -type f | sort
```

### 20. `##VIBE-CORE-DEP` (PROP-005:1080) — класс: `stale`

**Спека говорит** (дословно):
> the `compute_content_hash` algorithm (`src/content_hash.rs`, gated by `tests/content_hash_parity.rs` against a byte-for-byte copy of `fixtures/registry/flow/wal/v0.1.0/`).

**Дерево говорит** (`crates/vibe-index/tests/content_hash_parity.rs:69-79`, дословно):
> fn golden_flow_wal() -> PathBuf {
>     PathBuf::from(env!("CARGO_MANIFEST_DIR"))
>         .join("fixtures")
>         .join("golden-flow-wal-0.1.0")
> }
>
> fn golden_order_trap() -> PathBuf {
>     PathBuf::from(env!("CARGO_MANIFEST_DIR"))
>         .join("fixtures")
>         .join("golden-order-trap-0.1.0")
> }

Голова факта подтверждена: `vibe-core` — зависимость (`Cargo.toml:26`), сканер парсит
через `vibe-core` (`scanner/manifest.rs:51` — `Manifest::parse_str`, `:230` —
`SubskillManifest::read`), `src/types/kinds.rs` и `src/content_hash.rs` существуют и
остаются дубликатами. Но паритет-гейт стоит на других фикстурах, а не на «копии
`fixtures/registry/flow/wal/v0.1.0/`»; более того, хэширование теперь рецептовое —
в крейте есть `src/hash_recipe.rs` и parity-тест гоняет ОБЕ реализации на ОБЕИХ
фикстурах в ДВУХ рецептах (`Legacy0`/`Tree1`, `content_hash_parity.rs:106-133`).

**Что верно ТЕПЕРЬ**: паритет-гейт — две фикстуры `golden-*` и два рецепта; путь
`fixtures/registry/flow/wal/v0.1.0/` в крейте не существует.

**Как проверить заново** (одна команда):
```
grep -n "fixtures\|RecipeId" crates/vibe-index/tests/content_hash_parity.rs | head -20
```

### 21. `##GATE-COVERS` (PROP-005:1231) — класс: `stale`

**Спека говорит** (дословно):
> `tools/self-check.sh` no longer special-cases a second workspace — steps 1–2 (`cargo test --workspace`, `cargo clippy --workspace`) cover `vibe-index` like any member.

**Дерево говорит** (`tools/self-check.sh:431-445`, дословно):
> # 1. Formatting. The cheapest invariant — no compilation — so it runs
> # first and fails fast, before the multi-minute test / clippy steps.
> run_step "cargo fmt --all --check" cargo fmt --all --check || OVERALL=$?
>
> # 2. Tests.
> run_step "cargo test --workspace" cargo test --workspace --quiet || OVERALL=$?
> …
> # 3. Clippy as errors.
> run_step "cargo clippy --workspace --all-targets -- -D warnings" \

существество утверждения верно — специального случая «второго workspace» для vibe-index
в панели нет, тест и clippy покрывают крейт как любого члена. Но «steps 1–2» в дереве —
это fmt и test; clippy — шаг 3. Кроме того, панель теперь содержит специальные
vibe-index-гейты, которых спека не знает: шаг 10d «index clock gate»
(`self-check.sh:687-725` — греп `Utc::now|SystemTime::now` по
`crates/vibe-index/src/{index,types,journal}`), 6b `check-codegen`, 6c `specmap
--check`, 0d wire-derive ratchet.

**Что верно ТЕПЕРЬ**: fmt=1, test=2, clippy=3; отдельного vibe-index-workspace-случая
нет; зато есть четыре панельных шага, специально глядящих в vibe-index.

**Как проверить заново** (одна команда):
```
grep -n "run_step \"cargo \(fmt\|test\|clippy\)\|index clock gate" tools/self-check.sh
```

### 22. `##INT-FAST-PATH` / `##INT-OUTDATED-FAST` / `##INT-SEARCH` (PROP-005:628, 638, 642) — класс: `stale`

**Спека говорит** (дословно, PROP-005:628):
> Before falling back to per-repo `git ls-remote`, it tries `HTTP GET <registry.index_url>/repomd.json`. On 200, it reads `by-name/<name>.json` for the pkgref

PROP-005:638:
> query `by-name/<kind>/<name>.json` for the latest version instead of `git ls-remote`

PROP-005:642:
> Walks every configured registry's `index_url`, fetches `primary.jsonl.gz`, scans for matches against the user's query.

**Дерево говорит** (`crates/vibe-registry/src/index_client/handshake.rs:44-52`, дословно):
> /// Probe `<candidate>/hello.json` with the shared probe client. A
> …
> let url = format!("{candidate}/hello.json");

`crates/vibe-registry/src/multi_registry_resolver/mod.rs:378-386`:
> if let Some(url) = crate::index_client::index_url_for(&reg.name) {
> …
>     match crate::index_client::IndexClient::probe(&url, auth) {

`crates/vibe-cli/src/commands/search.rs:33`:
> use vibe_registry::{IndexAuth, IndexClient, ProbeOutcome, SearchHit, index_url_for};

Интеграции существуют и подтверждены по существу (fast-path резолвера, поиск через
индекс-клиент, POST-хук), но механика не та, что в §2.14: обнаружение индекса — хэндшейк
`hello.json` с пробой кандидатов, а не `GET repomd.json`; `by-name` — без уровня
`<kind>` (см. №8); `vibe search` ходит через `IndexClient` (`hello.json`-хэндшейк +
запросы), а выборка `primary.jsonl.gz` для сканирования не встречается ни в
`vibe-registry`, ни в `vibe-cli` (греп — ноль; контроль — п.11 «Контролей»).

**Что верно ТЕПЕРЬ**: клиент индекса открывает сессию хэндшейком `hello.json`;
резолвер получает fast-path через probe; поиск — через индекс-клиент, без скачивания
`primary.jsonl.gz`.

**Как проверить заново** (одна команда):
```
grep -rn "hello.json\|repomd.json\|primary.jsonl.gz" crates/vibe-registry/src/index_client crates/vibe-registry/src/multi_registry_resolver crates/vibe-cli/src/commands | head -20
```

### 23. `##CHANNELS` (PROP-005:877-923) — класс: `stale`

**Спека говорит** (дословно, PROP-005:879-880 и 896-897):
> @fact:CHANNELS-ARE-AUTHOR-POINTERS **Decision (owner rulings, 2026-08-13; not
> built — this section is the contract the build will follow).**
> …
> the explicit
> acts `ChannelSet {group, name, channel, version}` / `ChannelUnset` retarget
> or clear a pointer.

**Дерево говорит** (`crates/vibe-index/src/journal/record.rs:84-94`, дословно):
> ChannelSet {
>     group: Group,
>     name: String,
>     channel: String,
>     version: Version,
> },
> ChannelUnset {
>     group: Group,
>     name: String,
>     channel: String,
> },

словарь журнала уже несёт оба канальных акта ровно в той форме, которую §2.18 объявляет
«контрактом, по которому будут строить», и проектор их обрабатывает (`journal/project.rs`
— 4 хита `channel`); сгенерированный `SubskillEntry` уже имеет поле `channels`
(`index/inverted.rs:408`). CLI/HTTP-глаголов `channel set/unset` в дереве ещё нет —
но секция объявляет себя совсем нестроенной, а правдоносная половина её контракта
уже в дереве. (Смежная §2.16 «Webhooks», напротив, честно не построена: греп
`webhook` по `crates/vibe-index/src` — ноль, контроль — п.12.)

**Что верно ТЕПЕРЬ**: журнал и проектор уже понимают `ChannelSet`/`ChannelUnset`;
отсутствуют только внешние поверхности (CLI/HTTP) и projection-детали из §2.18.

**Как проверить заново** (одна команда):
```
grep -rn "Channel" crates/vibe-index/src/journal crates/vibe-index/src/index/inverted.rs
```

## Часть 2 — описан ли `unavailable` в `spec/**`

Искомое (греп, без учёта регистра): `unavailable` → **15 вхождений в 9 файлах**:
`vibevm/vibespecs/WAL.xml` (2), `vibevm/vibespecs/common/PROP-044-change-native-formats.xml` (1),
`vibevm/vibespecs/modules/vibe-cli/PROP-037-tree-tui.xml` (1),
`vibevm/vibespecs/modules/vibe-registry/PROP-008-qualified-naming.xml` (1), пять файлов
`vibevm/vibespecs/research/schema-evolution-2026-08/**` (9 суммарно, включая внешние
первоисточники-кэши `cargo-index.txt`, `pep691.rst`).

Контроль непустоты (§0.7), тем же инструментом по тому же дереву:
`must_understand` → **7 вхождений в 2 файлах** (`PROP-005-package-index.xml` — 4,
`vibevm/vibespecs/WAL.xml` — 3); `quarantine` (без регистра) → **15 вхождений в 5 файлах**
(`PROP-044` — 3, `vibevm/vibespecs/design/change-native-formats-verdict.xml` — 5, `WAL.md` — 5,
`PROP-013` — 1, research — 1). Инструмент жив; нулём результат не был, но контроли
выполнены всё равно.

Разбор 15 хитов по смыслу: **один** описывает сам концепт отказа —
`vibevm/vibespecs/common/PROP-044-change-native-formats.xml:265` (факт `M-MUST-UNDERSTAND`,
`@status:spec/plan`):
> the refusal surfaces at the point of use with a generated recipe ("unavailable
> because X; run Y")

— план-статусная фраза с примерной формулировкой рецепта, без статуса, формы,
энвелопа или полей. `PROP-008:126` использует слово в другом смысле («short names are
unavailable» без индекса); PROP-037 — «действие недоступно в режиме TUI»; research-файлы
— английская проза и внешние кэши. `vibevm/vibespecs/WAL.xml:45` сам фиксирует прежний замер:
«Ответ `unavailable` не описан НИГДЕ в `spec/**`».

**Вывод**: реализованный ответ — CLI-энвелоп с полем `unavailable: [Unavailable]`
(`cli/get.rs:44-45`; строки отказа у read-глаголов) и HTTP 404 с kind
`vibe-index/error/unavailable`, title «version unavailable to this build» и
расширением-строкой отказа (`server/error.rs:83-91,98-107`; тесты
`tests/cli_read/unavailable.rs`, `tests/server_e2e/unavailable.rs`) — не описан как
контракт нигде в `spec/**`. Ближайший текст — план-статусное предложение в PROP-044
§4.5; PROP-005, в чьих секциях §2.10/§2.11 эта поверхность живёт, слово не произносит
ни разу (греп `unavailable` по `PROP-005-package-index.xml` — 0; контроль: `must_understand`
в том же файле — 4). Это молчание — класса `silent` применительно к §2.10/§2.11, и оно
согласуется с прежним замером в `vibevm/vibespecs/WAL.xml:45` (с той оговоркой, что «НИГДЕ» в
буквальном смысле слова уже нет: одно концептуальное упоминание в PROP-044 существует).

## Контроли пустого вывода

Каждый ноль ниже прочитан как доказательство только после того, как тот же инструмент
на том же дереве показал ненулевой счётчик на заведомо существующем случае:

1. `VIBE_INDEX_DATA_DIR` в `crates/vibe-index` → **0**. Контроль: `VIBE_INDEX` (тот же
   греп, шире на суффикс) → 1 хит (`scanner/git_cli.rs:19`); `VIBE_LOG` в
   `crates/vibe-index/src` → 13 вхождений в 3 файлах.
2. `config.toml` в `crates/vibe-index` → **0**. Контроль: `checkpoint.json` тем же
   грепом → хиты (`index/checkpoint.rs:1,16`, `scanner/org_cache.rs:7,21,122,149`,
   `scanner/org_walk.rs:44`).
3. `stats.json` в `crates/vibe-index/src` → **0** (в одном прогоне с поиском
   `server.lock|checkpoint.json|mpsc|index_writer`, см. п.5). Контроль: `server.lock` →
   `lock.rs:3,21`, `cli/stop.rs:3,35`; `checkpoint.json` → п.2.
4. fsync каталога → **0** (нет ни одного `fsync`/`sync_all` по каталогу). Контроль: греп
   `sync_all|sync_data` по `crates/vibe-index/src` → 4 файловых call-site'а
   (`persistence.rs:33`, `journal/store.rs:56`, `lockfile.rs:82`, `auth.rs:116`-тест);
   плюс протокол `atomic_write` прочитан целиком (№9).
5. `mpsc | index_writer` в `crates/vibe-index/src` → **0**. Контроль: `spawn_blocking`
   в том же прогоне → `server/routes/packages.rs:543`.
6. `prometheus` в `crates/vibe-index/Cargo.toml` → **0**. Контроль: `[dependencies]`
   прочитан целиком (строки 24–50, 16 runtime-зависимостей перечислены) — ноль не от
   слепого грепа, а от полного перечня; `axum` в том же файле → `:40`.
7. `tests/cli_live_e2e.rs` → отсутствует. Контроль: полный листинг `tests/` (24 файла)
   тем же `find`; греп `#[ignore]` по `tests/` → 2 хита
   (`content_hash_parity.rs:249,276`), оба не про сеть.
8. `fixtures/sample-org`, `fixtures/golden-index` → отсутствуют. Контроль: тот же
   листинг `fixtures/` показывает `golden-flow-wal-0.1.0/`, `golden-order-trap-0.1.0/`.
9. `index_url|index_token` в `crates/vibe-core/src` → **0**. Контроль: тот же паттерн
   по `crates/vibe-publish/src` → `post_hook.rs:5,49,57,62,121,131,132,295,301`.
10. `post-receive` в `crates/vibe-index/docs/operator-handbook.md` → **0**. Контроль:
    греп `hook|post-receive|curl|\*/5` по тому же файлу → `:61`, `:111` (проза про
    post-publish hook), `:90` (cron-строка).
11. `primary.jsonl.gz` в `crates/vibe-registry/src` и `crates/vibe-cli/src` → **0**.
    Контроль: тот же паттерн по `crates/vibe-index/src` → `server/mod.rs`,
    `index/mod.rs:56`, `server/routes/index_files.rs:42` и др.
12. `webhook` в `crates/vibe-index/src` → **0** (§2.16 честно не построена — находки
    нет). Контроль: `channel` (тот же крейт) → 22 вхождения в 6 файлах.
13. JTD-схемы для типов записи: поиск **не вернул ноль** — `schemas/index/e1/` содержит
    5 схем; контроль §0.7 для №4 не потребовался, положительный результат предъявлен
    (№6).
14. Часть 2: искомое слово дало 15 хитов; контроли `must_understand` (7/2) и
    `quarantine` (15/5) — ненулевые (см. раздел Части 2).

## Что измерить из этого сегмента нельзя

- **Всё, что требует прогона** (cargo запрещён пакетом): зелёность тестов, clippy,
  fmt (§5, §10, ACC-*), фактическая-success déterminisme `rebuild --check`,
  `wire-diff`; читались файлы тестов, не их результаты.
- **Рантайм-поведения, не видимые чтением**: реально ли «tokens never appear in logs»
  (PROP-005:502) — в коде заголовок Authorization нигде не логируется, но отсутствие
  логирования при чтении — не доказательство исполнения; то же для HTTP-AUTH
  «read endpoints accept missing/invalid tokens silently» и CONCURRENCY-цифр
  (~10 writes/min, ~1000 reads/min — целевые нагрузки, не код).
- **§8 Ops-прогон** и REDISTRIBUTION's `cargo install --path crates/vibe-index` —
  исполнение команд не выполнялось.
- **§12 Version history** (даты и порядок решений) и статусы `@status:` — исторические
  утверждения; чтением дерева не опровергаются.
- **§2.16 Webhooks** целиком — секция честно `spec/plan|done` и в дереве отсутствует
  (п.12 контролей); нечего измерять кроме этого отсутствия.
- **Внутренняя согласованность PROP-044 ↔ PROP-005** вне сегмента (например, §2.4
  `#layout` до строки 464) — вне порученного диапазона; зафиксировано только то, что
  входит в 464–1345.

## Счёт

Просмотрено: весь сегмент 464–1345 (все ~115 `@fact`-якорей; 10 якорей контрольной
таблицы §3 пакета закрыты поимённо). Опровергнуто **23** утверждения: **8 `false`**
(№1, 4, 6, 9, 11, 12, 14, 15), **14 `stale`** (№3, 5, 7, 8, 10, 13, 16, 17, 18, 19,
20, 21, 22, 23), **1 `silent`** (№2) — плюс молчание §2.10/§2.11 об ответе
`unavailable` (Часть 2), не вошедшее в нумерацию. Подтверждены дословно якоря №6 и
№7, а также факты: HTTP-AUTH, HTTP-LOCKDOWN, NEVER-REPLACE/MODIFY/ECHO/ASSUME,
DATA-DIR-IS-WORKTREE, ATOMIC-WRITE-PROTOCOL (шаги 1–3), §2.17 целиком (семь фактов
авто-публикации), HELP-SMOKE, SCOPE-DISCIPLINE, THREAD-CLI-SYNC, THREAD-SERVER-ASYNC
(RwLock), OPEN-RATE-LIMIT (Q10), SLICE-2 (JTD-схемы), WIRE-CRON (в справочнике),
SCANNER-SHELL-OUT, not-pulling-db, INT-PUBLISH-HOOK по существу (с оговоркой №17).
