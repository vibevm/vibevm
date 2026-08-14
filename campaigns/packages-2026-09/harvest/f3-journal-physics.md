# F3-PHYSICS — физика журнала: где он может лежать, не уехав клиентам

Чем мерил: чтение `crates/vibe-index/**` (все CLI-команды, `index/`, `server/`,
`scanner/`, `publish.rs`, `lock.rs`, тесты), `crates/vibe-publish/src/git_publish.rs`,
`crates/vibe-registry/src/git_backend/shell.rs`, `tools/self-check.sh`, корневые
`.gitignore` и `Cargo.toml` (больше ничего вне `crates/**`+`tools/**` не открывалось;
`vibedeps/**` и `packages/**` не открывались). Что НЕ запускалось: `cargo` (запрещён),
`git` (запрещён) — ни одной команды; замер чисто чтением, git-семантика (`add -A`
при данном `.gitignore`) выведена из текстов правил, а не из запуска. Дата:
2026-08-14. Дерево: worktree `wt/F3-PHYSICS`.

## 1. ВЕРДИКТ

**ДА С ОГОВОРКАМИ** — на вопрос «можно ли положить журнал под сегодняшний
data-dir, не нарушив рулинг „журнал не входит в отдаваемую клиентам
поверхность"?» ответ двоится, и обе половицы измеримы:

- **Под `state/` — да, формально можно**: `state/` исключён из всех трёх
  клиентских поверхностей — из git (единственное правило data-dir-`.gitignore` —
  `/state/`, `crates/vibe-index/src/cli/init.rs:77`; preflight активно требует
  это, `crates/vibe-index/src/publish.rs:46-53`), из карты `files`
  (`repomd.json::files` строится только из того, что записали сами писатели, и
  доктрина типа прямо говорит «excluding `state/`», 
  `crates/vibe-index/src/types/repomd.rs:29-35`, карта — 
  `crates/vibe-index/src/index/memory.rs:218-285`), и из HTTP (маршрутизатор
  перечисляет ровно шесть статических маршрутов `/v1/index/*`, никакого
  catch-all — `crates/vibe-index/src/server/mod.rs:54-104`).
- **Рядом с проекцией (например `<data-dir>/journal/`) — нет**: `commit_and_push`
  выполняет `git add -A` из `current_dir = data_dir`
  (`crates/vibe-index/src/publish.rs:78`, `:124-127`), а `.gitignore`, который
  пишет `init`, покрывает ТОЛЬКО `/state/` (`crates/vibe-index/src/cli/init.rs:72-77`)
  — любой новый каталог под data-dir вне `state/` попадает в тот же коммит и на
  тот же remote, то есть прямо к клиентам. Документ модуля закрепляет это как
  дизайн: «`state/` is gitignored … and the rest is tracked and published»
  (`crates/vibe-index/src/publish.rs:9-14`).

Что мешает положить журнал «просто под data-dir» (нумерованно, с цитатами):

1. `git add -A` из корня data-dir (`crates/vibe-index/src/publish.rs:78`) —
   бездумно заметает любой новый каталог; исключение одно — правила
   `.gitignore`, а в нём одна строка `/state/` (`crates/vibe-index/src/cli/init.rs:77`).
2. `preflight` проверяет gitignore-покрытие ТОЛЬКО для `state/admin.tokens`
   (`crates/vibe-index/src/publish.rs:46`) — появление нового «должно быть
   непубличным» каталога не проверяется никем: механизм расширения
   секрет-периметра отсутствует.
3. `init` не переписывает существующий `.gitignore`
   (`crates/vibe-index/src/cli/init.rs:69-71`) — даже если Ф3 научит `init`
   писать правило для журнала, уже развёрнутые data-dir-ы его не получат
   (обновление `.gitignore` — отдельная миграционная задача, сегодня её некому
   нести: `init` с `--force` переписывает индексные файлы, но щадит
   `.gitignore`/`README.md`, `crates/vibe-index/src/cli/init.rs:47-52`, `:69-71`, `:86-88`).
4. Для публичного деплоя рулинг 2026-08-13 требует для журнала отдельный
   репозиторий `<registry>/index-journal`; вариант `state/journal/` публикует
   журнал В НИКУДА (см. §6б) — истина (PROP-044 §3, durable) остаётся на одном
   диске. Это не нарушение буквы «не входит в поверхность», но обесценивает
   журнал для публичного случая.
5. Мульти-воркеры через git-CAS из рулинга Ф3.1 («push только fast-forward;
   проигравший re-fetch → повтор append → push») сегодня не имеют ни одной
   детали: в `vibe-index` нет ни одного `git fetch`/`git pull`, push —
   single-shot без refspec (`crates/vibe-index/src/publish.rs:85`), отказ push —
   warn+счётчик без повторa (`crates/vibe-index/src/server/routes/packages.rs:418-425`).

## 2. Сверка опорных координат (B1..B8)

| # | утверждение | вердикт | цитата file:line |
|---|---|---|---|
| B1 | док модуля `publish.rs` объявляет data-dir git-рабочей копией со ссылкой на факт `DATA-DIR-IS-WORKTREE` | ПОДТВЕРЖДЕНО | `crates/vibe-index/src/publish.rs:9-10` — «Design, after PROP-005 fact `DATA-DIR-IS-WORKTREE`: the data directory is itself the index's git working copy» |
| B2 | `commit_and_push` (`publish.rs:75`) — ровно `add -A` → `commit --quiet -m` → `push --quiet` из `current_dir = data_dir` | ПОДТВЕРЖДЕНО С УТОЧНЕНИЕМ | последовательность есть: `publish.rs:78` (`add -A`), `:83` (`commit --quiet -m <msg>`), `:85` (`push --quiet`), `:124-127` (`current_dir(data_dir)`). Уточнение: между add и commit есть короткое замыкание `nothing_staged` (`git diff --cached --quiet`, `publish.rs:79-82`, `:144-147`) — пустой индекс ⇒ `NothingToCommit`, push не выполняется |
| B3 | `preflight` (`publish.rs:34`) отказывается стартовать, если data-dir не git-копия ИЛИ `state/admin.tokens` не покрыт gitignore | ПОДТВЕРЖДЕНО | `publish.rs:36-44` (не git-копия ⇒ ошибка «not a git working copy»), `publish.rs:46-59` (`check-ignore state/admin.tokens`: не игнорируется ⇒ ошибка про утечку токенов; ошибка проверки ⇒ отдельная ошибка) |
| B4 | `init` пишет `.gitignore` ровно с одной строкой-правилом `/state/` (`init.rs:67-82`) | ПОДТВЕРЖДЕНО С УТОЧНЕНИЕМ | `init.rs:72-77` — тело файла: 5 строк-комментариев + РОВНО одна строка-правило `/state/`. Уточнения: (1) «комментарии + одно правило», не «одна строка» в буквальном смысле; (2) если `.gitignore` уже существует — не трогается (`init.rs:69-71`) |
| B5 | чекпойнт лежит в `<data-dir>/state/checkpoint.json` (`checkpoint.rs:52`) | ПОДТВЕРДЕНО | `crates/vibe-index/src/index/checkpoint.rs:51-53` — `data_dir.join("state").join("checkpoint.json")`; `state/` покрыт `/state/` (init.rs:77), т.е. внутри gitignored-каталога |
| B6 | `Index::write_to` строит карту `files` только из записанного самим (`memory.rs:218-285`), не обходя data-dir | ПОДТВЕРЖДЕНО | `memory.rs:218-226` (primary.jsonl/.gz), `:251-259` (by-name файлы + каталог), `:264-285` (by-cap/by-purl файлы + каталоги) — вставки только из результатов собственных записей; обхода data-dir (`read_dir`/walk) в `write_to` нет. Док типа подтверждает намерение: «excluding `state/`» (`types/repomd.rs:29-35`) |
| B7 | HTTP отдаёт статику через `index_files.rs`, и набор путей ограничен | ПОДТВЕРЖДЕНО | маршруты: `server/mod.rs:59-82` — ровно `/v1/index/repomd.json`, `/v1/index/primary.jsonl`, `/v1/index/primary.jsonl.gz`, `/v1/index/by-name/{name}`, `/v1/index/by-cap/{slug}`, `/v1/index/by-purl/{slug}`; обработчики читают конкретные пути (`index_files.rs:19`, `:25`, `:33`, `:83`, `:99`, `:115`); catch-all/static-mount в роутере отсутствует (`server/mod.rs:54-104`) |
| B8 | подписчик трассировки ставится безусловно в `main.rs`, не под флагом | ПОДТВЕРЖДЕНО | `crates/vibe-index/src/main.rs:6-7` (`init_tracing()` до разбора CLI), `:17-21` («Install the tracing subscriber unconditionally — a binary's job»); `serve.rs:62-66` подтверждает: под флагом остаётся только preflight («only the preflight gate stays under the flag») |

## 3. Инвентарь data-dir

Таблица «путь → кто пишет → под git? → в `repomd.json`? → отдаётся по HTTP?».
Правило git-покрытия одно: `/state/` (`init.rs:72-77`); всё остальное «tracked
and published» (`publish.rs:9-14`).

| путь | кто пишет | под git? | в `repomd.json::files`? | по HTTP? |
|---|---|---|---|---|
| `repomd.json` | `repomd::write` из `write_to` (`index/repomd.rs:27-30`; вызов `memory.rs:302`) | да (не покрыто ничем) | нет — манифест не включает сам себя (карта строится из писателей, `memory.rs:218-285`) | да: `GET /v1/index/repomd.json` (`server/mod.rs:59-62`, `index_files.rs:17-20`) |
| `primary.jsonl` | `primary::write` (`index/primary.rs:43-46`) | да | да: `memory.rs:219-222` | да: `server/mod.rs:63-66`, `index_files.rs:22-29` |
| `primary.jsonl.gz` | `primary::write` (`index/primary.rs:51-53`) | да | да: `memory.rs:223-226` | да: `server/mod.rs:67-70`, `index_files.rs:31-69` (gzip, `Content-Encoding`) |
| `by-name/<name>.json` | `by_name::write` из `write_to` (`memory.rs:248-255`) | да | да: пофайлово `memory.rs:251-255` + запись-каталог `by-name` `memory.rs:256-259` | да: `/v1/index/by-name/{name}` (`server/mod.rs:71-74`, `index_files.rs:103-117`) |
| `by-cap/<slug>.jsonl` | `inverted::write_capability` из `write_to` (`memory.rs:264-270`) | да | да: `memory.rs:266-270` + каталог `memory.rs:278-281` | да: `/v1/index/by-cap/{slug}` (`server/mod.rs:75-78`, `index_files.rs:71-85`) |
| `by-purl/<slug>.jsonl` | `inverted::write_purl` из `write_to` (`memory.rs:271-277`) | да | да: `memory.rs:272-277` + каталог `memory.rs:282-285` | да: `/v1/index/by-purl/{slug}` (`server/mod.rs:79-82`, `index_files.rs:87-101`) |
| `.gitignore` | `init` → `write_gitignore`, только если файла нет (`init.rs:55`, `:67-82`, `:69-71`) | да (сам файл трекается и уезжает клиентам) | нет | нет |
| `README.md` | `init` → `write_readme`, только если файла нет (`init.rs:56`, `:84-126`, `:86-88`) | да | нет | нет |
| `state/` (каталог) | создаётся писателями: `ServerLock::try_acquire` (`lock.rs:30-34`), `checkpoint::save` (`checkpoint.rs:73-77`) | НЕТ — покрыт `/state/` (`init.rs:77`; предполагается preflight'ом `publish.rs:46-53`) | нет — док карты: «excluding `state/`» (`types/repomd.rs:29-35`) | нет — маршрута к `state/` нет (`server/mod.rs:54-104`) |
| `state/server.lock` | PID-файл: `lock.rs:35-58` (`create_new`), удаляется в `Drop` (`lock.rs:68-72`); читают `stop.rs:33`, `add.rs:147`, `remove.rs:76` | нет (внутри `/state/`) | нет | нет |
| `state/checkpoint.json` | `checkpoint::save` (`checkpoint.rs:72-82`); вызыватель — `reindex` (`cli/reindex.rs:301`), читает `reindex.rs:241-245` | нет | нет | нет |
| `state/org-cache.json` | `org_cache::save` (`scanner/org_cache.rs:151`); путь — `org_cache::path` (`org_cache.rs:125`), вызыватель `reindex.rs:121-125`; документация пути `reindex.rs:76-77` | нет | нет | нет |
| `state/admin.tokens` | КОД НЕ ПИШЕТ: `FileTokenStore` только читает (`server/auth.rs:50-53`; вызов `serve.rs:82-85`). Файл создаёт оператор; README это декларирует (`init.rs:102-103` — «state/ — gitignored runtime data (server PID, admin tokens …)») | нет | нет | нет |
| `<файл>.tmp.<pid>` (транзит) | `atomic_write`: временный сиблинг до rename (`index/persistence.rs:29-48`, имя `:39-48`); в покое не существует | формально да (ничем не покрыт), но транзиентен | нет | нет |
| `.git/` | сам data-dir — рабочая копия (`publish.rs:9-10`; тест-постановка `tests/auto_publish.rs:125`) | git сам не добавляет `.git` | нет | нет |

Иные писатели в data-dir отсутствуют: свип `remove_dir_all`/`clear_dir` и обход
`src` показал, что всё, что пишется под data-dir, перечислено выше (сканеры
пишут во внешние каталоги — см. §5). Каталога `journal/` в дереве нет:
`lib.rs:20-30` перечисляет модули без `journal`, `grep state/journal` по
`crates/vibe-index` пуст.

## 4. Что случилось бы с `journal/` под data-dir

По шагам, форма `<data-dir>/journal/2026-08.ndjson` при сегодняшнем коде:

1. **Попал бы в `git add -A`? ДА.** `commit_and_push` выполняет `add -A` из
   `data_dir` (`publish.rs:78`, `:124-127`), а `.gitignore` data-dir содержит
   единственное правило `/state/` (`init.rs:72-77`) — `journal/` ничем не
   покрыт. Дизайн-док модуля говорит это явно: «`state/` is gitignored … the
   rest is tracked and published» (`publish.rs:9-14`). Живое подтверждение —
   тест-постановка `auto_publish.rs:128-129`: `add -A` подхватывает всё, что не
   игнорируется, и коммитит.
2. **Попал бы в карту `files` внутри `repomd.json`? НЕТ.** Карта строится
   ТОЛЬКО вставками из результатов собственных записей писателей
   (`memory.rs:218-226`, `:251-259`, `:264-285`); обхода data-dir в `write_to`
   нет. Файл, который `write_to` не писал, в манифест не попадает, кто бы его
   ни положил рядом. (Докарта: ключи — «POSIX-style relative paths … beneath
   the data directory (excluding `state/`)», `types/repomd.rs:29-35` — но это
   описание множества, а не обход: механизм — перечисление писателей.)
3. **Отдался бы по HTTP? НЕТ.** Сервер не монтирует статику каталогом: роутер
   перечисляет маршруты явно (`server/mod.rs:54-104`), статика — ровно шесть
   путей: `/v1/index/repomd.json`, `/v1/index/primary.jsonl`,
   `/v1/index/primary.jsonl.gz`, `/v1/index/by-name/{name}`,
   `/v1/index/by-cap/{slug}`, `/v1/index/by-purl/{slug}`
   (`server/mod.rs:59-82`); каждый обработчик собирает конкретный путь
   (`index_files.rs:19`, `:25`, `:33`, `:83`, `:99`, `:115`) через
   `data_dir.join(...)`; прочие маршруты — structured query/admin/metrics
   (`server/mod.rs:84-104`), к файловой системе data-dir не обращаются.
   `/v1/index/journal/...` не существует, catch-all отсутствует — запрос
   получит 404 от axum.
4. **Заметил бы его `vibe-index verify`? НЕТ.** `verify` идёт по карте
   `manifest.files` (`cli/verify.rs:78` — `for (rel_path, entry) in
   &manifest.files`), проверяя перечисленные файлы на размер/sha256
   (`verify.rs:82-99`) и каталог `by-name` на количество (`verify.rs:103-114`).
   Обратного направления нет: файлы, отсутствующие в `repomd.json`, `verify`
   не видит и не ругает — журнал под data-dir для него прозрачен. (Бонус-факт:
   из Directory-записей проверяется только `by-name` — гейт
   `if rel_path == by_name::DIRNAME`, `verify.rs:103`; записи `by-cap`/`by-purl`
   не проверяются вовсе.)
5. **Пережил бы он `write_to`? ДА.** `write_to` сносит ровно три каталога:
   `by-name/` (`memory.rs:210`, реализация `clear_by_name` — `remove_dir_all`,
   `memory.rs:367-376`), `by-cap/` и `by-purl/` (`memory.rs:211-212`, реализация
   `inverted::clear_dir` — `remove_dir_all`, `inverted.rs:264-272`; сама
   процедура описана `memory.rs:203-209`). Полный свип
   `remove_dir_all|clear_dir` по `crates/vibe-index/src` даёт пять точек:
   `inverted.rs:264` (объявление `clear_dir`), `inverted.rs:266` (внутри неё),
   `memory.rs:211-212` (вызовы для by-cap/by-purl), `memory.rs:370` (by-name),
   `git_cli.rs:105` — четвёртая, ВНЕ data-dir: это удаление `.git` из
  клона-назначения сканера `materialise_at_ref` (`git_cli.rs:79-117`), не
   каталога data-dir. `journal/` под data-dir не сносится ничем и никем.

Итого физика: сегодня `journal/` под data-dir (вне `state/`) = уезжает клиентам
через git (шаг 1), невидим клиентам через HTTP и манифест (шаги 2-3), невидим
`verify` (шаг 4) и переживает перезаписи проекции (шаг 5). Единственная из
пяти дверей, которая для него открыта, — та самая, которую рулинг велит держать
закрытой.

## 5. Существующие механизмы второго каталога

Все path-аргументы всех команд (свип `#[arg(...)]` + `PathBuf` по
`crates/vibe-index/src/cli/*.rs`):

| команда | флаг/позиционный | тип | назначение | время жизни |
|---|---|---|---|---|
| все 16 команд | `data_dir` (позиционный) | `PathBuf` | сам индекс (`init.rs:20`, `reindex.rs:32`, `serve.rs:19`, `add.rs:27`, `remove.rs:21`, `verify.rs:18`, `get.rs:19`, `list.rs:19`, `search.rs:18`, `dump.rs:25`, `outdated.rs:21`, `capabilities.rs:18`, `purls.rs:18`, `stop.rs:22`, `rescan_org.rs:30`) | долгоживущий |
| `add` | `--manifest <PATH>` | `PathBuf` (`add.rs:31-32`) | чужой `vibe.toml`; каталог манифеста хэшируется в `content_hash` | читаемый вход, вне data-dir |
| `reindex` | `--from-clones <ORG-DIR>` | `Option<PathBuf>` (`reindex.rs:36-37`) | локальный каталог орг-клонов (вход сканера) | долгоживущий, вне data-dir |
| `reindex` | `--token-file <FILE>` | `Option<PathBuf>` (`reindex.rs:49-50`) | файл host-API токена | читаемый вход |
| `reindex` | `--clone-cache <DIR>` | `Option<PathBuf>` (`reindex.rs:60-61`) | куда сканер `--from-github` клонирует репо; БЕЗ флага — свежий tempdir, удаляемый в конце прогона (`reindex.rs:174-183`) | по флагу — долгоживущий «тёплый кэш»; без — транзиентный |
| `reindex` | `--cache-org` / `--no-cache-org` | `bool` (`reindex.rs:81-91`) | управление org-image кэшем; сам путь НЕ флаг — производный: `org_cache::path(&data_dir)` = `<data-dir>/state/org-cache.json` (`reindex.rs:121-125`, `org_cache.rs:125`, док `reindex.rs:76-77`, `org_cache.rs:1-7`) | долгоживущий, внутри gitignored `state/` |
| `rescan-org` | `--token-file <FILE>` | `Option<PathBuf>` (`rescan_org.rs:37-38`) | то же | читаемый вход |
| `rescan-org` | `--clone-cache <DIR>` | `Option<PathBuf>` (`rescan_org.rs:48-49`) | то же (`rescan_org.rs:45-47` — «Pass an explicit path to keep a warm cache») | как у reindex |
| `serve` | `--auth-tokens-file <FILE>` | `Option<PathBuf>` (`serve.rs:27-28`) | файл bearer-токенов ВМЕСТО `state/admin.tokens` (`serve.rs:82-85`) | читаемый вход, законно вне data-dir |
| `outdated` | `--lockfile <PATH>` | `PathBuf`, default `vibe.lock` (`outdated.rs:23-24`) | чужой lockfile; default относителен к CWD, не к data-dir | читаемый вход |

Ответы на вопросы §3.3:

- **Есть ли каталог ВНЕ data-dir?** Да, но только как ЧИТАЕМЫЕ входы или
  scratch: `--from-clones`, `--token-file`, `--auth-tokens-file`, `--manifest`,
  `--lockfile` (все `Option<PathBuf>`/`PathBuf` выше) и `--clone-cache`
  (scratch/warm-cache сканера, дефолт — tempdir: `reindex.rs:174-183`).
  Кандидат `--org-cache-path` флагом НЕ существует — путь org-кэша всегда
  производный от `data_dir` (`reindex.rs:121-125`).
- **Временный или долгоживущий?** `--clone-cache` без флага — транзиентный
  tempdir (`reindex.rs:176-182`); с флагом — долгоживущий тёплый кэш
  (`reindex.rs:57-59`, `rescan_org.rs:45-47`). Org-кэш и чекпойнт —
  долгоживущие, но ВНУТРИ `state/`. Всё остальное — read-only входы.
- **Прецедент «два каталога, один под git, второй нет»?** Частичный, не
  полный: (1) внутри одного data-dir — трекаемая проекция против
  gitignored-`state/` (правило `init.rs:77`, гейт `publish.rs:46-53`); (2)
  data-dir против внешнего `--clone-cache` (второй вообще не под git от имени
  `vibe-index`). Прецедента «второй ГИТ-репозиторий, которым управляет код»
  нет: единственный git-working-copy, который `vibe-index` коммитит и
  пушит, — сам data-dir (`publish.rs:9-14`, `:75-87`). `vibe-publish` —
  другой крейт — умеет staging-tempdir + push в чужой remote
  (`crates/vibe-publish/src/git_publish.rs:48-72`, `:235-285`), но это не
  `vibe-index`.

## 6. Три формы размещения журнала и цена каждой

Не выбираю — измеряю цену. Общие для всех трёх: новый модуль
`crates/vibe-index/src/journal/` (в `lib.rs:20-30` его нет), точки аппенда —
места мутаций: серверные хендлеры `packages.rs:251-272` (upsert),
`:315-324` (delete_version), `:345-354` (delete_package) и CLI-пути
`cli/add.rs`, `cli/remove.rs`, `cli/reindex.rs` (у всех часовня на краю:
`add.rs:50`, `remove.rs:35`, `reindex.rs:217` — паттерн F2-1 «clock at the
edge» делает аппенд с меткой времени законным именно на краю).

### (а) `<data-dir>/journal/` — рядом с проекцией, попадает в git-коммит

- **Что тронуть:** `init.rs:72-77` — НЕТ, правило для журнала сюда писать
  бессмысленно (игнор отрежет его от git, превратив в форму (б)); напротив,
  чтобы журнал не уехал, пришлось бы добавить правило — а это и есть признание,
  что форма выбрана неправильно. Тронуть: `publish.rs:9-14` (док «the rest is
  tracked and published» станет ложью — журнал трекается и публикуется);
  `publish.rs:34-61` (preflight не проверяет ничего про журнал); док README
  (`init.rs:95-104` — список Files придётся расширить); точки аппенда выше.
- **Что ломается:** тест
  `auto_publish.rs:279-309` (`identical_repeat_upsert_publishes_exactly_one_commit`,
  ассерт `rev-list --count HEAD == 2` на `:299-303`) — если журнал пишет строку
  на каждый запрос мутации (включая идемпотентный повтор), повтор создаёт
  непустой diff ⇒ второй коммит ⇒ счётчик 3 ⇒ красный тест; вместе с ним
  ломается инвариант F2-3 «a mutation that changes nothing … must not write,
  and must not publish, anything» (`memory.rs:113-119`). Тесты
  `auto_publish.rs:220-226`, `:257-271` (ассерты на тему HEAD-коммита) ломаются,
  если журнал публикуется ОТДЕЛЬНЫМ коммитом после каталожного. Гейт determinism
  `memory/tests.rs:352-354` + `:361-394` (два полных обхода дерева data-dir,
  побайтовое сравнение) покраснеет, если аппенд журнала встрою́т в `write_to`.
- **Против рулинга 2026-08-13:** ДА, прямое противоречие — «журнал не входит в
  отдаваемую клиентам поверхность», а форма (а) кладёт его в тот же коммит и
  на тот же remote, что и проекцию (механика — §4 шаг 1). Рулинг для
  публичного деплоя требует отдельный репозиторий; форма (а) — это ровно
  «журнал внутри репозитория проекции».
- **Цена ошибки:** журнал утекает клиентам (публикация фактов внутреннего
  журнала, включая, например, отказы push с stderr в сообщениях
  `publish.rs:136-139`); откат невозможен без history-rewrite удалённого
  репозитория проекции (force-push запрещён и рулингом Ф3.1, и общей
  дисциплиной ветки) — односторонняя дверь.

### (б) `<data-dir>/state/journal/` — внутри gitignored `state/`

- **Что тронуть:** точки аппенда (выше); больше по сути ничего — `state/` уже
  исключён из всех трёх поверхностей (правило `init.rs:77`, гейт
  `publish.rs:46-53`, карта `types/repomd.rs:29-35` + `memory.rs:218-285`,
  маршруты `server/mod.rs:54-104`). Опционально README (`init.rs:102-103` уже
  называет `state/` «runtime data» — строку можно уточнить).
- **Что ломается:** ничего из найденных тестов — тесты на `state/`-содержимое
  точечные (`scanner_e2e.rs:358` — checkpoint существует; `org_cache_e2e.rs:323`,
  `:445`, `:511-526` — org-cache существует/отсутствует), полного перечисления
  `state/` никто не утверждает.
- **Против рулинга 2026-08-13:** НЕ против буквы «не входит в отдаваемую
  клиентам поверхность» — все три двери закрыты измеренно (§1). Но рулинг же
  говорит: «для публичного деплоя журнал живёт в отдельном репозитории …
  история вечная, append-only», а «для локальных/герметичных инсталляций журнал
  в data-dir законен». Форма (б) — это законный локальный случай, оставленный
  на публичном деплое: цепочка «gitignored-журнал = истина, которую никуда не
  отгружают», по коду ДЕРЖИТСЯ: git не видит `state/` (правило + preflight),
  `repomd` не видит, HTTP не видит, а ДРУГОГО канала отгрузки у `vibe-index`
  нет (единственный push — `publish.rs:85`, из data-dir; fetch отсутствует
  вовсе, §7). То есть в публичном деплое форма (б) даёт durable-на-одном-диске:
  PROP-044 §3 (истина обязана быть durable) выполняется лишь постольку,
  поскольку живёт один диск; реплики, истории «вечной», как у git-репо, нет.
  Это не противоречие рулингу, это его неприменение: рулинг сам же для этого
  случая предписывает форму (в).
- **Цена ошибки:** тихая недолговечность — потеря диска/хоста = потеря всей
  истории фактов без внешних копий; multi-writer git-CAS (§7) невозможен в
  принципе (нет второго git-репозитория); ошибка не видна ни одному гейту —
  ни один тест/панель не заметит, что журнал не отгружается.

### (в) отдельный каталог/репозиторий, задаваемый новым аргументом

- **Что тронуть:** `serve.rs` Args — новый флаг (по образцу `--auth-tokens-file`,
  `serve.rs:27-28`) + preflight для журнального ворктри (по образцу
  `publish.rs:34-61`: is-git-dir + нужные gitignore-гарантии); `publish.rs` —
  второй `commit_and_push`-путь (существующий принимает `data_dir: &Path`
  параметром — `publish.rs:75` — и реентерабелен для другого каталога;
  единственное, что придётся добавить — fast-forward-восстановление после
  отклонённого push: fetch + повторный аппенд, см. §7); `state.rs` — поле
  `journal_dir` рядом с `data_dir` (`state.rs:26`, `:36-41`); `packages.rs:396-431`
  — журнальный publish рядом с каталожным; CLI-пути `add.rs`/`remove.rs`/
  `reindex.rs` — тот же аргумент; `tests/help_smoke.rs:11-27` — список
  подкоманд не изменится (флаг, не подкоманда), но при подкоманде — расширить
  (инвариант «every later slice's CLI addition must keep green»,
  `help_smoke.rs:1-5`); `init.rs` — опционально создание/инициализация
  журнального каталога; Ф3.1-цикл «push fast-forward; проигравший re-fetch →
  повтор append в хвост → push» — НОВЫЙ код: re-fetch сегодня отсутствует
  во всём `vibe-index` (§7).
- **Что ломается:** из существующего — ничего найденного: новые аргументы
  дефолтно-выключаемы (образец — `with_auto_commit_push`, `state.rs:126-133`,
  «the flag defaults to off, and only `serve` opts in»); тест
  `flag_off_runs_no_git` (`auto_publish.rs:349-370`) продолжит держать
  «flag-off server never runs git». Риск новый: двойной push на мутацию —
  частичный отказ (каталог уехал, журнал нет) даёт рассинхрон проекции и
  истины; сегодня warn+счётчик (`packages.rs:418-425`) лечит это «следующей
  мутацией», для журнала так же.
- **Против рулинга 2026-08-13:** НЕТ — это и есть предписанная форма для
  публичного деплоя («конвенция `<registry>/index-journal`; история вечная,
  append-only, branch-protection без force-push»).
- **Цена ошибки:** операционная сложность — оператор обязан провижинить второй
  репозиторий и его branch-protection (код принципиально не задаёт refspec/
  remote — Р1, `publish.rs:13-15`); double-push latency; окно частичного
  отказа между двумя push. Ошибка сборки/пуша журнала не роняет клиентский
  запрос (паттерн Р4 уже есть — `packages.rs:70-74`, `:418-425`).

## 7. Конкуренция писателей и git-CAS

- **Кто сериализует что сегодня.** Три уровня, все ОДНО-хостовые:
  1. Один сервер на data-dir: `state/server.lock`, PID-файл, атомарный
     `create_new(true)` (`lock.rs:29-59`); повторный старт получает отказ
     «another vibe-index server already holds» (`lock.rs:41-46`); файл удаляется
     в `Drop` (`lock.rs:68-72`).
  2. CLI против сервера: `add`/`remove` ОТКАЗЫВАЮТСЯ работать, если PID-файл
     существует (`add.rs:146-154`, `remove.rs:75-83` — «Use the HTTP API or
     stop the server first»). CLI-против-CLI ничем не сериализуется (замок
     CLI-команды не берут — только читают PID, `add.rs:147`, `remove.rs:76`) —
     мягкое окно TOCTOU, сегодня невидимое (одиночные операторские запуски).
  3. Внутри сервера: индекс под async `RwLock` (`state.rs:30`) — мутация
     эксклюзивна; publish отделён от индексного замка и сериализован собственным
     `publish_lock: Mutex<()>` (`state.rs:37-42`), хендлер держит его на время
     блокирующего git-вызова (`packages.rs:408-412` — `spawn_blocking` +
     `commit_and_push`).
- **git push с явным refspec / с force — свип `crates/**`.** Продакшн-вызовы
  push (не тесты), все:
  - `crates/vibe-index/src/publish.rs:85` — `push --quiet`, БЕЗ refspec, без
    force (Р1: «no refspec, no hard-coded remote», `publish.rs:13-15`);
  - `crates/vibe-publish/src/git_publish.rs:71` — `push -u origin main`;
  - `crates/vibe-publish/src/git_publish.rs:72` — `push origin <tag>`;
  - `crates/vibe-publish/src/git_publish.rs:105` — `push -u origin main`;
  - `crates/vibe-publish/src/git_publish.rs:177` — `push origin <tag>`;
  - `crates/vibe-publish/src/git_publish.rs:211` — `push origin main`.
  `--force`/`--force-with-lease` НЕТ нигде в `crates/**` (grep пуст; наоборот,
  `git_publish.rs:184` закрепляет: «The push is a fast-forward (no `--force`)»).
  Остальные git-глаголы продакшн-кода: `publish.rs:78` `add -A`, `:83` commit,
  `:147` `diff --cached --quiet`, `:169` `check-ignore`; сканер —
  `clone --depth 1 --branch <tag>` (`git_cli.rs:86-94`), `clone --quiet`
  (`from_github.rs:191`), `tag -l`/`rev-list`/`rev-parse` (`git_cli.rs:30-72`);
  `vibe-registry` — `clone --recurse-submodules` (`shell.rs:143-152`) и
  `fetch --prune --tags origin` (`shell.rs:164`) — но это кэш-зеркало ЧТЕНИЯ,
  не push-путь.
- **Re-fetch/retry вокруг push? НЕТ.** В `vibe-index` нет ни одного
  `git fetch`/`git pull` (grep по `crates/vibe-index/src` — только clone/tag/
  rev-list/rev-parse/check-ignore/add/commit/push/diff). Отказ push
  обрабатывается по Р4: warn-лог + счётчик `publish_failures_total`
  (`packages.rs:418-425`, счётчик `state.rs:44-51`, метрика видна в
  `auto_publish.rs:340-344`), запрос не роняется; комментарий «the index
  retries on the next mutation» (`packages.rs:421-422`) — не retry и не
  re-fetch: следующая мутация коммитит ПОВЕРХ расходящейся локальной ветки и
  снова толкает — при ушедшем вперёд remote это будет падать до ручной
  сверки. Единственный «конкурентный» случай, который код реально решает, —
  intra-copy: `NothingToCommit` после пустого `diff --cached`
  (`publish.rs:79-82`, док `:63-68`) — «конкурентная публикация уже увезла
  это изменение» в ОДНОЙ рабочей копии. Итого: цикл Ф3.1
  «fast-forward push → проигравший re-fetch → повторный append в хвост →
  push» сегодня не реализован ни одной строкой; строительные блоки —
  append-only NDJSON-файлы (форма хранения Ф3), `publish_lock` (внутри-process
  сериализация), fast-forward-только push — есть; re-fetch и повторный append
  после отклонения — нет.

## 8. Тесты, которые заметят новый каталог

Свип `read_dir|entry_count|exists()` по `crates/vibe-index/tests/**` и
`src/**/tests*.rs`:

- **Тестов, утверждающих ТОЧНЫЙ набор верхнего уровня data-dir, НЕТ.** Два
  `read_dir`-места: `persistence.rs:111` (свой tempdir с одним файлом — про
  отсутствие `.tmp.`-мусора после `atomic_write`, к data-dir отношения не
  имеет) и `memory/tests.rs:384` (обход внутри помощника `walk`).
- **Самый сильный enumerating-тест:** `memory/tests.rs:361-394`
  (`assert_trees_byte_identical` + `walk`) — ПОЛНЫЙ рекурсивный обход двух
  data-dir-ов и побайтовое сравнение множеств файлов; используется на
  `memory/tests.rs:352-354` (двойной `write_to` одного индекса в два каталога).
  Он не заметит `journal/`, созданный вне `write_to`; но ПОКРАСНЕЕТ, если
  журнальный аппенд встроят в `write_to` (множества/байты разойдутся) — это
  фактический гейт детерминизма F2-1/F2-2 на уровне дерева.
- **Тесты существования (не точного набора):** `cli_lifecycle.rs:137-138`
  (primary.jsonl/.gz), `:207-212` (`assert_disk_has_files` — наличие по
  списку), `cli_write.rs:72`, `:227`, `scanner_e2e.rs:211-213`, `:358`,
  `org_cache_e2e.rs:323`, `:445`, `:511-529`, `from_github_e2e.rs:291`,
  `memory/tests.rs:129`, `:167-168`, `:217`, `:245`. Появление `journal/`
  под data-dir их НЕ ломает (наличие, не отсутствие/исчерпание).
- **Что реально сломает `journal/` под data-dir (вне `state/`):**
  1. `auto_publish.rs:279-309` — если журнал пишет факт на идемпотентный
     повтор мутации и живёт в трекаемом дереве: второй коммит, ассерт
     `rev-list --count HEAD == "2"` (`:299-303`) падает; корень — F2-3
     (`memory.rs:113-119`).
  2. `auto_publish.rs:220-226`, `:257-271` — если журнал публикуется отдельным
     коммитом: ассерты «последний коммит называется `index: …`» падают.
  3. Любой тест с `assert_trees_byte_identical` — если аппенд в `write_to`.
- **Тесты на точное содержимое `repomd.json::files`:** точного равенства НЕТ;
  единственное место, смотрящее в карту, — `cli_lifecycle.rs:142-153`
  (`init_seeds_empty_repomd_with_inverted_dirs`): asserts `contains_key` для
  `primary.jsonl`, `primary.jsonl.gz`, `by-name`, `by-cap`, `by-purl`
  (`:147-152`). Добавление журнала в карту его не сломает; НЕ-добавление —
  тоже. `help_smoke.rs:33-43` проверяет лишь вхождение подкоманд в `--help`
  (не исчерпывающее отсутствие лишних) — новые флаги/подкоманды не роняют.

## 9. Панель и гейты

Шаги `tools/self-check.sh` по порядку (метки run_step, с номерами строк):

1. `:196-197` — знаменатель этажа: floor строит все live-пакеты (fn `:163-195`).
2. `:219-220` — идентичность CLAUDE.md = AGENTS.md = GEMINI.md (fn `:208-218`).
3. `:274-286` — snapshot user-home tripwire (шаг 0).
4. `:295` — `cargo fmt --all --check`.
5. `:298` — `cargo test --workspace`.
6. `:303` — tripwire после workspace-тестов.
7. `:306-307` — `cargo clippy --workspace --all-targets -- -D warnings`.
8. `:316-317` — `cargo run -p vibe-cli -- check --path . --quiet` (спек-линтер).
9. `:325` — `cargo xtask conform check` (Class-F/G доктесты, file-length, unwrap).
10. `:332` — `cargo xtask sync-engines --check`.
11. `:342` — `cargo xtask check-codegen`.
12. `:352` — `cargo xtask specmap --check` (host traceability + orphan-рачет).
13. `:378-385` — core-ai-native: authored-slot guard + fmt/test/clippy.
14. `:393-410` — три языковых стека: fmt/test/clippy.
15. `:425-430` — specmap self-traces стеков.
16. `:437-458` — mcp-пакеты: fmt/test/clippy/self-trace.
17. `:481-486` — conform check на каждый gated slot (цикл).
18. `:531-532` — знаменатель authored-крейтов mcp.
19. `:569-570` — **index clock gate (F2.1)**, fn `:549-568`.
20. `:601` — lane-citation lint (PROP-035 §11), fn `:586-600`.
21. `:638-639` — каждый member workspace декларирует licence (PROP-000 §3).
22. `:661-662` — `vibe progress check --exhaustive` (markup-валидация).
23. `:668` — tripwire за весь прогон.

Что из этого ходит по дереву и среагирует на новый каталог/модуль Ф3:
шаг 4 (fmt — каждый новый файл), 5 (тесты — компиляция+прогоны), 7 (clippy
`-D warnings`), 8 (`vibe check` по bootstrap-манифесту), 9 (conform: budget
длины файлов, unwrap-запрет, доктест-правила — новые файлы обязаны пройти),
12 (specmap orphan-гейт: публичные items gated-крейтов без spec-тега —
красный), 22 (markup по прогресс-корпусу), 19 — см. ниже.

**Grep-гейт часов из Ф2.1 — дословно** (`tools/self-check.sh:549-570`, комментарий-обоснование `:534-548`):

```sh
check_index_clock_gate() {
  local hits
  hits=$(grep -rnE 'Utc::now\(|SystemTime::now\(' \
      crates/vibe-index/src/index \
      crates/vibe-index/src/types \
      2>/dev/null \
    | grep -vE ':[0-9]+:[[:space:]]*//')
  if [ -n "$hits" ]; then
    printf '%s\n' "$hits" >&2
    printf 'self-check: the index writer modules call the clock directly.\n' >&2
    printf 'self-check: the rule — time enters at the edge (CLI command or\n' >&2
    printf 'self-check: server mutation event) and never inside index/ or types/:\n' >&2
    printf 'self-check: one state must produce one byte sequence, or "rebuild and\n' >&2
    printf 'self-check: compare" measures nothing (PROP-044 §4.3, F2-1).\n' >&2
    printf 'self-check: fix: pass the time as an argument — a WriteCtx for\n' >&2
    printf 'self-check: write_to, an `at` for Index::new / VersionEntry::minimal.\n' >&2
    return 1
  fi
  return 0
}
run_step "index clock gate (no Utc::now/SystemTime::now in index/ or types/)" \
  check_index_clock_gate || OVERALL=$?
```

Ответ на вопрос пакета: **попадает ли журнал под запрет `Utc::now` — решает
этот гейт, и сегодня ответ НЕТ для плана Ф3.** Периметр гейта назван по
каталогам: `crates/vibe-index/src/index` и `crates/vibe-index/src/types`,
рекурсивно (`self-check.sh:540-542` — «The perimeter is named by module
directory … a new file under either is covered the day it lands»). План Ф3
кладёт журнал в `crates/vibe-index/src/journal/` — вне обоих каталогов:
`Utc::now()` в `src/journal/**` гейт НЕ увидит. Если журнал ляжет под
`src/index/journal/` — гейт накроет его автоматически. Фильтр комментариев —
только строки, начинающиеся с `//` (`:555`) — док-комментарии с примерами
проходят законно (это отмечено и в комментарии `:544-548`).

## 10. Дыры и неожиданности

1. **`verify` проверяет Directory-записи только для `by-name`** — гейт
   `if rel_path == by_name::DIRNAME` (`cli/verify.rs:103`): записи `by-cap` и
   `by-purl` в `files` парсятся, но не проверяются вовсе (ни счётчик, ни
   хэши). Пакет об этом не спрашивал; для Ф3 значит: «журнал не в манифесте —
   verify молчит» — и вообще verify слабее, чем манифест обещает.
2. **Три разные «клиентские поверхности», и они не совпадают.** git-дерево
   репозитория проекции (публикуемое) ⊃ HTTP-поверхность (6 путей,
   `server/mod.rs:59-82`) ⊃ `repomd::files`. `.gitignore` и `README.md`
   трекаются и уезжают клиентам через git (`init.rs:67-82`, `:84-126` — ничем
   не игнорируются), но не отдаются по HTTP и не в манифесте. Рулинг говорит
   «клиенты читают только проекцию» — при git-канале доставки «проекция»
   физически шире HTTP-протокола. Для Ф3 это довод: границу «что видит
   клиент» надо определять по git-дереву, не только по маршрутам.
3. **`B2`-уточнение:** между `add -A` и `commit` стоит `nothing_staged`
   (`git diff --cached --quiet`, `publish.rs:79-82`, `:144-147`) — пустой diff
   это УСПЕХ (`NothingToCommit`), push не вызывается. Существенно для Ф3:
   «мутация без изменений» не создаёт коммит (F2-3, `memory.rs:113-119`) —
   журнальный факт «повторной мутации» в трекаемом дереве сломал бы это.
4. **`preflight` проверяет ровно ОДИН путь** (`state/admin.tokens`,
   `publish.rs:46`) — обобщённого механизма «этот путь обязан быть
   игнорируемым» нет; появление новых непубличных каталогов под data-dir
   некому гейтить.
5. **`init` не обновляет существующие `.gitignore`/`README.md`**
   (`init.rs:69-71`, `:86-88`) даже с `--force` — миграцию правил на уже
   развёрнутых data-dir-ах Ф3 должен нести отдельно.
6. **CLI-писатели не сериализованы между собой**: `add`/`remove` лишь читают
   PID (`add.rs:147`, `remove.rs:76`) и отказывают только при живом сервере;
   два параллельных CLI-мутатора одного data-dir сегодня возможны (TOCTOU).
   Для Ф3 (мульти-воркеры) это готовая дыра независимо от формы размещения.
7. **Push-отказ без восстановления**: при ушедшем вперёд remote
   (`--auto-commit-push`, два сервера на одном remote) push будет падать
   вечно, warn+`publish_failures_total` — единственный сигнал; никакого
   fetch/rebase/retry (`packages.rs:418-425`). Ровно этот механизм —
   «проигравший re-fetch → повтор append» — рулинг Ф3.1 требует, и его нет
   (§7).
8. **`--lockfile` по умолчанию относителен к CWD**, а не к data-dir
   (`outdated.rs:23-24`, default `vibe.lock`) — мелкая непоследовательность
   path-семантики, для Ф3 прецедент «внешний путь по умолчанию» скорее
   анти-паттерн.
9. **`state/admin.tokens` кодом не пишется** — только читается
   (`auth.rs:50-53`); README декларирует, но создать его должен оператор
   (`init.rs:102-103`). Для Ф3: «оператор создаёт файл» — уже существующий
   прецедент ручного провижинина под `state/`.
10. **Root `.gitignore` хост-репозитория**: `/campaigns/*/run/mirror/` и
    `/.wt/` игнорируются (`.gitignore:58`, `:70`) — файлы находки под
    `campaigns/packages-2026-09/harvest/` трекаются (не подпадают под
    игнор-правила), а сам ворктаут-каталог — нет; `git status` босса покажет
    ровно два новых файла (находка + отчёт).
11. **Гейт часов не накрывает `src/journal/`** (§9): при запланированном пути
    `crates/vibe-index/src/journal/` правило Ф2.1 «часы на краю» для журнала
    станет необеспеченным гейтом — либо периметр гейта расширить, либо модуль
    класть под `src/index/`, либо принять, что для append-only журнала
    детерминизм байтов неприменим (у журнала другое назначение), и зафиксировать
    это решением, а не умолчанием.
12. **Ни один тест не утверждает точный верхнеуровневый набор data-dir**
    (§8) — появление `journal/` под data-dir само по себе ничего не роняет;
    единственные реальные детекторы — F2-3/F2-1-ассерты в `auto_publish.rs` и
    побайтовое сравнение деревьев в `memory/tests.rs`.

## 11. Как воспроизвести этот замер

Все команды — из корня рабочего дерева, Git Bash; `cargo`/`git` не
запускались и не нужны (замер чтением):

```sh
# 1. Сверка B1-B3: док/код публикации
sed -n '1,90p' crates/vibe-index/src/publish.rs

# 2. Сверка B4-B6: init/gitignore, checkpoint, карта files
sed -n '67,82p' crates/vibe-index/src/cli/init.rs
sed -n '51,53p' crates/vibe-index/src/index/checkpoint.rs
sed -n '197,303p' crates/vibe-index/src/index/memory.rs

# 3. Сверка B7: маршруты и статика
sed -n '54,104p' crates/vibe-index/src/server/mod.rs
sed -n '1,120p' crates/vibe-index/src/server/routes/index_files.rs

# 4. Сверка B8: безусловный tracing
sed -n '1,31p' crates/vibe-index/src/main.rs

# 5. Свип сносимых каталогов (§4 шаг 5)
grep -rn "remove_dir_all\|clear_dir" crates/vibe-index/src

# 6. Свип git-глаголов продакшн-кода (§7)
grep -rn '"push"\|"add"\|"commit"' crates/vibe-index/src
grep -rn -- '--force\|force-with-lease' crates --include='*.rs'

# 7. Механизмы второго каталога (§5)
grep -rn ': PathBuf\|Option<PathBuf>' crates/vibe-index/src/cli

# 8. Что проверяет verify (§4 шаг 4)
sed -n '73,131p' crates/vibe-index/src/cli/verify.rs

# 9. Тесты-детекторы (§8)
grep -rn 'read_dir\|entry_count\|exists()' crates/vibe-index/tests crates/vibe-index/src/index/memory/tests.rs
sed -n '279,309p' crates/vibe-index/tests/auto_publish.rs

# 10. Панель и гейт часов (§9)
grep -n 'run_step\|check_index_clock_gate' tools/self-check.sh
sed -n '534,570p' tools/self-check.sh

# 11. Самопроверка пакета (§5 задания)
wc -l campaigns/packages-2026-09/harvest/f3-journal-physics.md
grep -c "^## " campaigns/packages-2026-09/harvest/f3-journal-physics.md
cat .gitignore
```
