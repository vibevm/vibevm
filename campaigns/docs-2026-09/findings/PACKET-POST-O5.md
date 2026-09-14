# PACKET-POST-O5 — норма о быстром пути чтения по HTTPS и адаптеры хостов в своём файле (B-165; Rust + спека)

##subagent-quiet-clause

Ты — воркер-кодер (`opus5`, High). Читай ровно названные файлы; boot-лейн не читаешь.
Ветка `research-preview-1-docs`, ворктри `C:\Users\olegc\git\v\vibevm-docs`. Общий `git`-индекс с
центральной сессией и web-воркером (он в `vibevm/vibepacks/org.vibevm.doc/web/**`; ты — только в
`crates/vibe-registry/**` и одной спеке). Коммитишь **только** формой `git commit -m … -- <пути>`, новые файлы —
`git add -- <файл>`; никогда `git add -A`, никогда голый `git commit`, никаких трейлеров `Co-Authored-By` и
никаких упоминаний модели или агента в сообщениях коммитов — авторство репозитория человеческое (PROP-000
`#commits`). `git push` не делаешь. `specmap.json` не регенерируешь. Никакие процессы, которых ты не
запускал, не останавливать; `CARGO_TARGET_DIR` не переопределять.

## Зачем

B-165 (`BACKLOG.md`): POST-O1 научил бэкенд читать один файл по HTTPS с GitHub и GitVerse до `git archive`,
а PROP-002 §2.12 (`PERF-FETCH-FILE`) всё ещё описывает чтение одного файла как `git archive`; POST-O4 положил
рядом факт о повторах (`RAW-READ-BACKOFF`), и норма о повторе стоит раньше нормы о самом пути. Заодно
`raw_http.rs` перерос бюджет строк (605 против 600).

## Читать сначала, ровно эти файлы

1. `crates/vibe-registry/src/git_backend/shell/raw_http.rs` (докстринги модуля и `try_read`, таблица хостов,
   `absence_is_authoritative`, `names_fixed_content`, токен заголовком), `raw_http/retry.rs`, `raw_http/tests.rs`,
   `crates/vibe-registry/tests/https_file_read.rs`, `git_backend/shell.rs` (место вызова `try_read`).
2. `vibevm/vibespecs/modules/vibe-registry/PROP-002-*.xml` §2.12 (`PERF-FETCH-FILE`, `RAW-READ-BACKOFF`,
   соседние `PERF-*`) — как формулируются факты и ревизии (`req rN`, если секция их носит).
3. `findings/WORKER-REPORT-POST-O1.md` (разделы «Коммит 1», «Отклонений от пакета нет» — три уточнения:
   промах авторитетен по форме рефа, таблица хостов с `gitverse.ru`, токен) и `findings/WORKER-REPORT-POST-O4.md`
   («Находки»).

## Сделать — два атомарных коммита

**U. Норма.** Факт `RAW-READ-FAST-PATH` в PROP-002 §2.12 **перед** `RAW-READ-BACKOFF`, по докстрингам кода:

> Before a single file is asked of git, the backend reads it over HTTPS from the host's own raw endpoint
> when the host is one it knows — `github.com` through `raw.githubusercontent.com`, `gitverse.ru` through
> its contents API — with any credential sent only as a bearer header, never in the address. A hit is the
> file. A miss is authoritative only for a tag or a commit SHA, whose content is fixed; on a branch, or for
> a manifest, the read falls through to git, whose answer stands as it always did. A host the table does
> not name never enters this path.

`PERF-FETCH-FILE` поправить так, чтобы `git archive` описывался как второй шаг после быстрого пути (не
первым и единственным); если секция носит `req rN` — подними ревизию и переведи рёбра `verifies` на неё в
том же коммите, перечитав тесты. Атрибуты факта — как у соседей (`impl/done`, `action="continue"
actionstage="doc"`; **без** `audience`: прозу и аудиторию даёт центральная сессия). Ребро `implements` на
`RAW-READ-FAST-PATH` — атрибутом на `try_read` (или scope модуля `raw_http.rs`, как принято в крейте).
Коммит: `docs(spec): say that a single file is read over HTTPS before git is asked`.

**V. Адаптеры хостов — в `raw_http/hosts.rs`.** Таблица хостов и их адресация (`RawHost`, `address`,
github/gitverse) переезжают в `raw_http/hosts.rs`; `raw_http.rs` остаётся ниже 600 строк; поведение и тесты
не меняются (тесты переезжают вместе с кодом, если лежат рядом). Никаких новых зависимостей.
Коммит: `refactor(registry): keep the raw-read hosts in their own file`.

## Периметр файлов

`crates/vibe-registry/src/git_backend/shell/raw_http.rs`, `raw_http/**`, `shell.rs` (только `use`),
`crates/vibe-registry/tests/https_file_read.rs`, `vibevm/vibespecs/modules/vibe-registry/PROP-002-*.xml`.
Ничего в `Cargo.toml`, web-пакете, руководстве, `specmap.json`.

## Самопроверка (обязательно, вывод в отчёт)

```
cargo fmt --all -- --check
cargo clippy -p vibe-registry --all-targets -- -D warnings
cargo test -p vibe-registry
wc -l crates/vibe-registry/src/git_backend/shell/raw_http.rs crates/vibe-registry/src/git_backend/shell/raw_http/hosts.rs
cargo xtask specmap --check   # красный по новому факту / ребру / бампу — в отчёт
```

## Приёмка боссом

Два коммита; факт стоит перед `RAW-READ-BACKOFF` и говорит то, что делает код; `PERF-FETCH-FILE` не
противоречит ему; `raw_http.rs` < 600 строк; тесты зелёные, 304 и больше.

## Отчёт

`campaigns/docs-2026-09/findings/WORKER-REPORT-POST-O5.md` (не коммитить): хэши и subject'ы, точный id и
текст факта, что стало с `PERF-FETCH-FILE` и ревизией, вывод гейтов дословно, аномалии.
