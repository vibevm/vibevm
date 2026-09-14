# PACKET-W1-O2 — версия 1.0.0 у руководства и сайта; «перезаписывать, не бампать» (Rust + web + docker)

##subagent-quiet-clause

Ты — воркер-кодер (`opus5`, High). Читай ровно названные файлы; boot-лейн не читаешь.
Ветка `research-preview-1-docs`, ворктри `C:\Users\olegc\git\v\vibevm-docs`. Этот пакет двигает каталоги,
поэтому запускается, когда в web-пакете нет другого воркера (центральная сессия это гарантирует); общий
`git`-индекс с центральной сессией. Коммитишь **только** формой `git commit -m … -- <пути>`, переносы —
`git mv`, новые файлы — `git add -- <файл>`; никогда `git add -A`, никогда голый `git commit`, никаких
трейлеров `Co-Authored-By` и никаких упоминаний модели или агента в сообщениях коммитов — авторство
репозитория человеческое (PROP-000 `#commits`). `git push` не делаешь. `specmap.json` не регенерируешь.
Никакие процессы, которых ты не запускал, не останавливать.

## Зачем

Слово владельца 2026-09-14 (`findings/OWNER-REVIEW-2026-09-14.md`, п. 1): версия всех пакетов — 1.0.0;
опубликованный номер не бампается, а перезаписывается, до отдельного решения. Сегодня руководство лежит в
`vibevm-docs/v0.1.0`, сайт — в `org.vibevm.doc/web/v0.1.0`, а PROP-058 обещает минорную версию раз в месяц.

## Читать сначала, ровно эти файлы

1. `vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0/vibe.toml`, `README.md`, `CHANGELOG.md`,
   `maintenance/**` (снимок поверхности уже `1.0.0.json`); `vibevm/vibepacks/org.vibevm.core/vibevm-docs-ru/v1.0.0/vibe.toml`
   (`[translates] version`).
2. `vibevm/vibepacks/org.vibevm.doc/web/v0.1.0/vibe.toml`, `package.json`, `README.md`, `site/README.md`,
   `docker/Dockerfile*`, `docker/compose.yaml`, `docker/nginx.conf`, `docker/site.toml`, `site.example.toml`,
   `Dockerfile.dockerignore` в корне репозитория (allow-list путей).
3. Всё, что называет любой из двух путей `v0.1.0`: `grep -rn "vibevm-docs/v0.1.0\|web/v0.1.0" --include=*.rs --include=*.ts --include=*.tsx --include=*.mjs --include=*.md --include=*.toml --include=*.yaml --include=*.yml --include=*.sh --include=*.json --include=*.ps1 . --exclude-dir=target --exclude-dir=node_modules --exclude-dir=.git --exclude-dir=vibedeps` — каждое вхождение либо переименовать, либо назвать в отчёте, почему нет (исторические записи кампании — `campaigns/docs-2026-09/LEDGER.md`, `JOURNAL.md`, отчёты воркеров — **не** править: это история).
4. `vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml` (`SITE-VERSION-SHOWS-CURRENT`,
   `REL-VERSION-SELECTION`), `PROP-058-documentation-maintenance.xml` (`LOOP-VERSION`, `LOOP-MONTHLY`),
   `PROP-000.xml` (`#commits` — как выглядят факты).
5. `DEV-GUIDE.md` §8 (сайт), `tools/self-check.sh` (что из этого он гоняет), `campaigns/docs-2026-09/PLAN.md`
   §9 (строки с путями пакетов — обновить, это живой план, не история).

## Сделать — четыре атомарных коммита

**N2. Вид пакета — на провод (первым коммитом, до переезда).** Проекция манифеста документации не несёт
`[package].kind`: `crates/vibe-doc/src/manifest.rs` тратит его на `projection`, и сайт видит только «doc или
не doc» (отчёт `findings/WORKER-REPORT-W1-O5.md`). Добавь `doc_package.kind` в `schemas/doc_manifest.jtd.json`
(enum восьми видов PROP-000 `KIND-SET`), перегенерируй типы (`cargo xtask check-codegen`), заполни в
`manifest.rs` (и в `site/level0.rs`, если проекция уровня 0 строится там), прочитай в парсере оболочки
`site/src/lib/manifest.ts` (четыре двери: схема, типы, проекция, парсер — J-126) и дай `kindOf(card)` в
`site/src/lib/library.ts` читать поле вместо вывода из `projection`; `maintenance/derived.json` обоих
doc-пакетов сдвинется (`--derived --accept`). Тест на разбор поля. Коммит: `feat(doc): carry a package's kind
in the documentation manifest`.

**K. Переименование.** `git mv vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0 …/v1.0.0`, в манифесте
`version = "1.0.0"`; `git mv vibevm/vibepacks/org.vibevm.doc/web/v0.1.0 …/v1.0.0`, в манифесте и
`package.json` версия `1.0.0`. Все ссылки на пути — из п. 3 списка чтения — переписаны; `[translates]
version` адаптации — `"^1.0"`; `[[documents]] version` руководства не меняется (это версия предмета).
`CHANGELOG.md` руководства: запись `1.0.0 — 2026-09-14` с тем, что изменилось для читателя с первой
выкладки (переключатель темы и тёмное умолчание, работающий поиск, аннотации карточек, `self update`
вперёд и `self reinstall`, русская адаптация как отдельный пакет; коротко, без чисел).
Коммит: `refactor(doc): publish the manual and the site as 1.0.0`.

**L. Политика номера.** Факт в PROP-057 рядом с `SITE-VERSION-SHOWS-CURRENT`, id `VERSION-OVERWRITE-POLICY`:

> Until the owner decides otherwise (ruling 2026-09-14), every package this repository publishes carries
> the number `1.0.0`, and a change is published under the same number in place: the registry keeps no
> past publication, the site shows the current content of the number, and the changelog records what
> changed by date rather than by a new number. A consumer who pinned the number keeps the bytes the lock
> file recorded, and a fresh install gets the current ones (`##SITE-VERSION-SHOWS-CURRENT`,
> PROP-002 `##PUBLISH-MUTABLE-VERSIONS`).

И поправка `LOOP-VERSION` / `LOOP-MONTHLY` в PROP-058: где они обещают «минорную версию пакета» — заменить
на «публикацию под тем же номером с датированной записью в changelog, пока действует
`VERSION-OVERWRITE-POLICY`»; снимок поверхности (`surface --record 1.0.0`) перезаписывается под тем же
номером. Статусы фактов — как у соседей (`impl/done`, `action="continue" actionstage="doc"
audience="author"`; если `--coverage` требует цитаты со страницы — сними `audience` и скажи в отчёте,
прозу пишет центральная сессия). Коммит: `docs(spec): keep every published number at 1.0.0 and republish
in place`.

**M. Сборка сайта под новым путём.** `docker/Dockerfile*`, `compose.yaml`, `Dockerfile.dockerignore`,
`DEV-GUIDE.md` §8, `tools/self-check.sh` — новые пути; `build:static`/`build:embedded` зелёные из нового
каталога; `docker build` сам не гоняешь (серверная выкладка — центральная сессия), но `docker compose
config -q` над `docker/compose.yaml` с плейсхолдерами — да, если compose установлен локально (иначе скажи).
Коммит: `build(web): follow the site package to 1.0.0`. (Если весь M умещается в K без потери
читаемости — два коммита вместо трёх, скажи в отчёте.)

## Периметр файлов

Два переименованных каталога целиком, `vibevm/vibespecs/common/PROP-057-*.xml`, `PROP-058-*.xml`,
`Dockerfile.dockerignore`, `DEV-GUIDE.md`, `tools/self-check.sh`, `campaigns/docs-2026-09/PLAN.md`,
`crates/**` только там, где путь `v0.1.0` вписан в тест или фикстуру. Ничего в `specmap.json`, в истории
кампании (леджер, журнал, отчёты, пакеты), в `vibedeps/**`.

## Самопроверка (обязательно, вывод в отчёт)

```
grep -rn "vibevm-docs/v0.1.0\|web/v0.1.0" . --exclude-dir=target --exclude-dir=node_modules --exclude-dir=.git --exclude-dir=vibedeps --exclude-dir=campaigns   # пусто
cargo build -p vibe-cli
target/debug/vibe.exe doc check --path vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0 --examples --citations --derived --coverage --media --style --min 100
target/debug/vibe.exe doc check --path vibevm/vibepacks/org.vibevm.core/vibevm-docs-ru/v1.0.0 --citations --derived --translations --style --min 100
target/debug/vibe.exe check --path vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0
cargo test -p vibe-doc
pnpm -C vibevm/vibepacks/org.vibevm.doc/web/v1.0.0 floor
pnpm -C vibevm/vibepacks/org.vibevm.doc/web/v1.0.0 build:static
pnpm -C vibevm/vibepacks/org.vibevm.doc/web/v1.0.0 build:embedded
cargo xtask specmap --check   # красный только по новым фактам и перемещённым doc-страницам — в отчёт
```

## Приёмка боссом

Два-три коммита, каждый читается как PR; в дереве нет ни одной ссылки на `v0.1.0` двух пакетов вне
истории кампании; оба doc-пакета и сайт проходят свои гейты из новых путей; факты политики на месте;
changelog с датой.

## Отчёт

`campaigns/docs-2026-09/findings/WORKER-REPORT-W1-O2.md` (не коммитить): хэши и subject'ы, список всех
переписанных вхождений путей и всех оставленных (с причиной), вывод гейтов дословно, аномалии, что не
сделано и почему.
