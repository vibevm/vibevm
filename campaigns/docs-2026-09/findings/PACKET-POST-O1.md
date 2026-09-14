# PACKET-POST-O1 — `vibe install` молчит минуты: чтение манифестов зависимостей без клона

##subagent-quiet-clause

Ты — воркер-кодер (`opus5`, High). Читай ровно названные файлы; boot-лейн не читаешь.
Ветка `research-preview-1-docs`, ворктри `C:\Users\olegc\git\v\vibevm-docs`. Общий `git`-индекс с
центральной сессией: коммитишь **только** формой `git commit -m … -- <пути>`, новые файлы —
`git add -- <файл>` перед этим; никогда `git add -A`, никогда голый `git commit`. `git push` не делаешь.
Один Rust-воркер на общем тёплом `target/` (он твой, приватных каталогов не заводить).
Правила, которые тебя связывают: R-12 — не читать `~/.vibe/*.token`; никакие процессы, которых ты не
запускал, не останавливать; `bash tools/self-check.sh` не гонять (точечные гейты ниже).

## Читать сначала, ровно эти файлы

1. `crates/vibe-registry/src/git_backend/shell.rs` — `ShellGitBackend::fetch_file_at_ref` (строки ~224–310),
   `run_raw`, `apply_common_env`, `preflight`, разбор ошибок.
2. `crates/vibe-registry/src/git_backend/mod.rs` — трейт `GitBackend`, `GitError` (варианты
   `FileNotFoundInRef`, `ArchiveUnsupported`, `RefNotFound`, `RepoNotFound`, `AuthFailed`).
3. `crates/vibe-registry/src/git_package_registry/lookup.rs` — `fetch_dep_manifest`, `fetch_manifest_at_ref`
   («archive-first, clone fallback»): обещание в докстринге «N `git archive` round-trips, not N clones».
4. `crates/vibe-registry/src/multi_registry_resolver/redirect_follow.rs` — `try_fetch_redirect_for_url`
   (та же пара «архив → клон» для `vibe-redirect.toml`).
5. `crates/vibe-registry/src/git_package_registry/urls.rs` и `auth.rs` — как токен попадает в URL
   (`credentialed_url` / `inject_token`), какая форма userinfo.
6. `crates/vibe-registry/src/index_client/mod.rs` — как здесь строится `reqwest::blocking::Client`
   с таймаутом на вызов (строки ~540–570); тот же приём для нового пути.
7. `crates/vibe-registry/tests/index_fast_path.rs` — mock-сервер на axum в тестах крейта: образец.
8. `crates/vibe-registry/src/git_backend/shell/tests_pure.rs` — как тесты бэкенда проверяют, что
   именно выполняется, без сети.

## Диагноз (измерено центральной сессией 2026-09-14, `VIBE_LOG=debug`)

`vibe install org.vibevm.world/redbook --path <свежий проект>` печатает «Resolving 1 root package…» и
молчит: 108 с до плана в одном прогоне, 240 с и убит таймаутом в другом. Трасса на каждый пакет
зависимости (у redbook их 26, все `=1.0.0` на `github.com/vibespecs/*`):

```
02:09:58.980 running git (raw) argv="git archive --remote=https://github.com/vibespecs/org.vibevm.world.discovery-prompt.git --format=tar v1.0.0 -- vibe-redirect.toml"
02:10:00.593 running git argv="git clone --recurse-submodules --no-checkout --no-tags -- https://github.com/… C:\Users\…\.vibe\registries\…\clone"
02:10:03.440 running git argv="git fetch --no-tags -- origin +refs/tags/v1.0.0:refs/tags/v1.0.0"
02:10:05.139 running git argv="git checkout --detach --force refs/tags/v1.0.0^{commit}"
02:10:05.597 running git argv="git submodule update --init --recursive"
02:10:07.701 running git (raw) argv="git archive --remote=… --format=tar v1.0.0 -- vibe.toml"
```

GitHub не поддерживает `upload-archive`: каждый `git archive --remote` заканчивается
`ArchiveUnsupported` через ~1,6 с сетевого раунда, и код честно падает в клон (~5,5 с на пакет).
Пробa `vibe-redirect.toml` и чтение `vibe.toml` — две такие пары на пакет. Итог ≈ 9 с × 26 ≈ 4 мин,
без единой строки вывода. Версии при этом берутся из индекса (`list_versions served from index`),
дорог только манифест.

## Сделать

**Быстрый путь чтения файла с GitHub по HTTPS в `ShellGitBackend::fetch_file_at_ref`** — до попытки
`git archive`, так, чтобы выиграли все вызывающие (`fetch_dep_manifest`, `fetch_manifest_at_ref`,
проба редиректа):

1. Если `url` — HTTPS-адрес репозитория на хосте ровно `github.com`
   (`https://[userinfo@]github.com/<owner>/<repo>[.git]`), прочитать
   `https://raw.githubusercontent.com/<owner>/<repo>/<refname>/<path>` блокирующим `reqwest` с таймаутом
   10 с (тот же приём «клиент на вызов», что в `index_client`), с `User-Agent` как у индекса.
   `<refname>` — как передан (тег `v1.0.0`, ветка, SHA); `<path>` — нормализованный (posix).
2. Учётные данные: если userinfo URL несёт токен (форма — см. `urls.rs`/`auth.rs`), передать его
   заголовком `Authorization: Bearer <token>` (raw.githubusercontent.com принимает его для приватных
   репозиториев); в URL запроса, в логах и в текстах ошибок — только plain URL без userinfo.
   Никакого чтения токенов из файлов: токен уже в URL, как и для `git`.
3. Ответы: `200` → байты. `404` → для `vibe.toml` (имя `Manifest::FILENAME`) **падать в существующий
   git-путь**, чтобы диагноз остался честным (`RefNotFound` / `RepoNotFound` / `FileNotFoundInRef`
   различает только git); для любого другого файла (`vibe-redirect.toml` — обычный случай
   «маркера нет») → `GitError::FileNotFoundInRef` без единого запуска git. Любой другой статус,
   таймаут или ошибка транспорта → `tracing::debug!` и падение в существующий путь (архив → клон)
   без изменения его поведения.
4. Хосты не `github.com` — без изменений. Не менять `GitBackend`-трейт; не менять семантику
   `ArchiveUnsupported`; не трогать `fetch.rs` (клон при установке остаётся как есть).
5. Тесты (без сети): mock-сервер на axum, база raw-адреса подменяется в тесте (например,
   конструктор/поле бэкенда `raw_base` с умолчанием `https://raw.githubusercontent.com`, или
   переменная окружения только для тестов — выбери то, что чище в этом крейте, и скажи почему):
   - `200` на `vibe.toml` → байты, и `git` **не запускается** (проверить тем же способом, что
     `tests_pure.rs`, или счётчиком запросов + отсутствием клона);
   - `404` на `vibe-redirect.toml` → `FileNotFoundInRef`, git не запускается;
   - `404` на `vibe.toml` → происходит переход в git-путь (достаточно показать, что путь
     вызван — например, ошибка классифицирована как git-ошибка, а не как HTTP);
   - токен из userinfo приходит на mock заголовком `Authorization: Bearer …`, а в тексте ошибки
     при `500` его нет;
   - маппинг URL: `https://github.com/o/r.git`, `https://github.com/o/r`, `https://x@github.com/o/r.git`,
     не-GitHub и SSH-формы (`git@github.com:o/r.git` — не быстрый путь).
6. Докстринги: `lookup.rs` («N archive round-trips») и `redirect_follow.rs` (шаги 1–2) — дописать
   третий, первый по порядку, шаг: HTTPS-чтение с GitHub; `@scope` у новых элементов —
   `spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#…` (тот же якорь, что у соседей;
   `PERF-FETCH-FILE` — см. `PROP-002-decentralized-registry.xml:720`).
7. Замер до/после тем же скриптом:
   `bash <scratch>/repro.sh target/debug/vibe.exe 300` (`repro.sh` лежит в scratch центральной сессии; полный путь воркеру сообщён)
   (свежий проект, `vibe install org.vibevm.world/redbook`, печатает `EXIT=… after N s`; до плана
   спрашивает подтверждение без TTY — это и есть конец фазы разрешения). «До» — 108 с / 240 с
   (см. выше), «после» — ожидание порядка 10–20 с. Скрипт пишет в scratch центральной сессии,
   в `~/.vibe/registries/…` пишет сам `vibe` — это нормально.

Необязательно, если дёшево и с тестом: запоминать в экземпляре бэкенда хосты, ответившие
`ArchiveUnsupported`, и не пробовать `git archive` на них повторно в том же процессе.

## Периметр файлов

`crates/vibe-registry/src/git_backend/**`, `crates/vibe-registry/src/git_package_registry/lookup.rs`,
`crates/vibe-registry/src/multi_registry_resolver/redirect_follow.rs`, `crates/vibe-registry/tests/**`,
`crates/vibe-registry/Cargo.toml` (только если нужна dev-зависимость, которая уже есть в workspace),
`specmap.json` не трогать (перегенерирует центральная сессия). Ничего вне периметра.

## Самопроверка (обязательно, вывод в отчёт)

```
cargo fmt --all -- --check
cargo clippy -p vibe-registry --all-targets -- -D warnings
cargo test -p vibe-registry
cargo xtask specmap --check
cargo build -p vibe-cli && bash <repro.sh выше> target/debug/vibe.exe 300
```

`cargo test -p vibe-registry` может содержать сетевые тесты — если такие есть и они не скипаются
сами, назови их и не жди их часами: `--skip <имя>` с причиной в отчёте.

## Приёмка боссом

Диф читается как PR; git не запускается для `200`/`404`-случаев (тест); токен нигде не печатается;
`repro.sh` показывает фазу разрешения в разы короче; поведение для не-GitHub хостов не изменилось.

## Коммит

Один коммит: `perf(registry): read manifests from GitHub over HTTPS instead of cloning` с телом «почему»
(GitHub не даёт `upload-archive`, и обещание «архив, не клон» на нём не держалось: 26 клонов и 52
провальных архива ради 26 файлов; чтение по HTTPS возвращает обещание). Авторство человеческое,
без трейлеров.

## Отчёт

`campaigns/docs-2026-09/findings/WORKER-REPORT-POST-O1.md` (не коммитить): хэш, что сделано, замеры
до/после дословно, вывод гейтов, аномалии, что не сделано и почему.
