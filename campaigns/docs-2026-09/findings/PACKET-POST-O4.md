# PACKET-POST-O4 — короткий backoff на 429/5xx перед падением быстрого пути в git (B-161; Rust)

##subagent-quiet-clause

Ты — воркер-кодер (`opus5`, High). Читай ровно названные файлы; boot-лейн не читаешь.
Ветка `research-preview-1-docs`, ворктри `C:\Users\olegc\git\v\vibevm-docs`. Общий `git`-индекс с
центральной сессией и другими воркерами (web-пакет и, возможно, `crates/vibe-cli/**`; ты — только в
`crates/vibe-registry/**` и одной спеке). Коммитишь **только** формой `git commit -m … -- <пути>`, новые
файлы — `git add -- <файл>`; никогда `git add -A`, никогда голый `git commit`, никаких трейлеров
`Co-Authored-By` и никаких упоминаний модели или агента в сообщениях коммитов — авторство репозитория
человеческое (PROP-000 `#commits`). `git push` не делаешь. `specmap.json` не регенерируешь. Никакие
процессы, которых ты не запускал, не останавливать. Общий `target/` — общий: если `cargo` ждёт замок,
жди, не переопределяй `CARGO_TARGET_DIR`.

## Зачем

POST-O1 сделал быстрый путь: один файл читается по HTTPS вместо клона. На неожиданный статус путь
честно падает в лестницу git (`archive` → clone → fetch → checkout → submodule), и один HTTP 429 от
`raw.githubusercontent.com` стоил около 20 с из 90 с фазы разрешения на графе redbook (B-161, отчёт
`findings/WORKER-REPORT-POST-O1.md`, «Аномалии» п. 1). Слово владельца 2026-09-14: сделать короткий backoff.

## Читать сначала, ровно эти файлы

1. `crates/vibe-registry/src/git_backend/shell/raw_http.rs` (+ `raw_http/tests.rs`) — `try_read`,
   `build_client`, таблица хостов, что считается «неожиданным статусом».
2. `crates/vibe-registry/src/git_backend/shell.rs` — где вызывается `try_read` и что делает промах;
   `crates/vibe-registry/tests/https_file_read.rs` — сквозной тест с локальным mock-сервером (образец).
3. `vibevm/vibespecs/modules/vibe-registry/PROP-002-*.xml` — факты про быстрый путь (найди по
   `raw`/`HTTPS`/`archive`, POST-O1 их добавил или уточнил) — новый факт кладётся рядом, тем же стилем.
4. `findings/WORKER-REPORT-POST-O1.md`, разделы «Коммит 1», «Замеры», «Аномалии».

## Сделать — два атомарных коммита

**S. Backoff.** На `429` и `5xx` (и на транспортную ошибку, если она не «хост неизвестен») `try_read`
повторяет запрос: не больше трёх попыток, паузы 0,5 с → 1 с → 2 с, `Retry-After` (секунды или дата)
уважается, но обрезается сверху пятью секундами; суммарно быстрый путь не ждёт дольше ~5 с на один файл.
После последней неудачи — ровно то, что сейчас: падение в git с той же диагностикой. На `404` (и другие
«ответ есть, файла нет») — без повторов, как сейчас. Паузы — через инъекцию (`Sleeper`/функция), чтобы
тесты не спали; тесты на mock-сервере: 429 → 200 со второй попытки; три 503 → падение в git; `Retry-After: 2`
уважается; 404 не повторяется. Строка debug-лога на каждый повтор (какой статус, какая попытка, сколько
ждём). Никаких новых зависимостей.
Коммит: `perf(registry): retry a rate-limited raw read briefly before asking git`.

**T. Норма.** Факт в PROP-002 рядом с фактом быстрого пути (id `RAW-READ-BACKOFF`):

> A raw read that the host refuses with `429` or a `5xx` is retried a small, bounded number of times
> with a short pause, honouring `Retry-After` within that bound, before the read falls through to git as
> any other unexpected answer does. A `404` is never retried: on a tag or a commit it is authoritative,
> and on a branch it is what git will be asked about next.

Статус — как у соседей (`impl/done`, `action="continue" actionstage="doc" audience="user"`; если
`--coverage` потребует цитаты со страницы — сними `audience` и скажи в отчёте).
Коммит: `docs(spec): say that a rate-limited raw read is retried briefly`.

## Периметр файлов

`crates/vibe-registry/src/git_backend/shell/raw_http.rs`, `…/raw_http/tests.rs`, `…/shell.rs` (только если
нужно), `crates/vibe-registry/tests/https_file_read.rs`, `vibevm/vibespecs/modules/vibe-registry/PROP-002-*.xml`.
Ничего в `Cargo.toml`, в `crates/vibe-cli/**`, в web-пакете, в руководстве, в `specmap.json`.

## Самопроверка (обязательно, вывод в отчёт)

```
cargo fmt --all -- --check
cargo clippy -p vibe-registry --all-targets -- -D warnings
cargo test -p vibe-registry
cargo xtask specmap --check   # красный по новому факту — в отчёт
```

Живой замер не обязателен (429 не воспроизводится по желанию); если `scratch`-скрипт POST-O1
(`repro.sh` — путь в его отчёте) под рукой и сеть есть, один холодный прогон с временем — приятно, но не
условие приёмки.

## Приёмка боссом

Два коммита; тесты с mock-сервером покрывают 429→200, 3×503→git, `Retry-After`, 404 без повторов; никаких
задержек в тестах; факт на месте.

## Отчёт

`campaigns/docs-2026-09/findings/WORKER-REPORT-POST-O4.md` (не коммитить): хэши и subject'ы, выбранные
константы, вывод гейтов дословно, аномалии.
