# PACKET-W1-O3 — манифест: авторство прозы, навигация, featured и происхождение бриджа (Rust + схемы)

##subagent-quiet-clause

Ты — воркер-кодер (`opus5`, High). Читай ровно названные файлы; boot-лейн не читаешь.
Ветка `research-preview-1-docs`, ворктри `C:\Users\olegc\git\v\vibevm-docs`. Общий `git`-индекс с
центральной сессией и web-воркером W1-O4 (он в `vibevm/vibepacks/org.vibevm.doc/web/v0.1.0/site/**` и
`design/**`; ты в `crates/**`, `schemas/**`, спеках и манифестах doc-пакетов). Коммитишь **только** формой
`git commit -m … -- <пути>`, новые файлы — `git add -- <файл>`; никогда `git add -A`, никогда голый `git commit`,
никаких трейлеров `Co-Authored-By` и никаких упоминаний модели или агента в сообщениях коммитов — авторство
репозитория человеческое (PROP-000 `#commits`). `git push` не делаешь. `specmap.json` не регенерируешь
(центральная сессия). Никакие процессы, которых ты не запускал, не останавливать. Общий `target/` свободен.

## Зачем

Владелец после первого просмотра сайта (`findings/OWNER-REVIEW-2026-09-14.md`, пп. 5, 6, 8, 12) хочет:
бейдж GENERATED у проекций, фильтр «all / human-authored / ai-generated» по авторству прозы, список
featured-документаций в конфигурации сайта, две страницы, припинённые к верху руководства, и у
бридж-пакетов — «Bridge Maintainer» отдельно от «Destination Author». Сайт всё это читает из
машинных манифестов, которые пишет Rust; этот пакет даёт данным места, а web-пакет (W1-O4/W1-O6) их
покажет.

## Читать сначала, ровно эти файлы

1. `schemas/doc_manifest.jtd.json`, `schemas/doc_site_config.jtd.json`, `formats/REGISTRY.toml` — формат
   манифеста документации и конфигурации сайта; правило: схема — единственный источник истины, типы
   генерируются (`cargo xtask check-codegen`), руками генерированное не правится.
2. `crates/vibe-wire/src/generated/doc_manifest/**`, `…/doc_site_config/**` — что генерируется сейчас.
3. `crates/vibe-doc/src/manifest.rs` (+ `manifest/tests.rs`) — проекция манифеста doc-пакета;
   `crates/vibe-doc/src/site/level0.rs` — уровень 0 (проекции пакетов без документации);
   `crates/vibe-doc/src/site/config.rs` (+ `config/tests.rs`) — разбор `site.toml`;
   `crates/vibe-cli/src/commands/doc/site/web.rs` — как значения `site.toml` едут в web-сборку окружением
   (`VITE_SITE_*`, константа `DEFAULT_THEME` и соседи).
4. `crates/vibe-core/src/manifest/**` — `[package]` (поле `bridge`, `authors`), `[[embedded_source]]`
   (`upstream_authors`, `upstream_license`); `crates/vibe-check/**` там, где проверяются поля манифеста
   doc-пакета (найди по `KIND-DOC-MUST-DOCUMENT` / `documents`).
5. `vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml` — разделы `CARD-*`, `KIND-DOC-*`,
   `REL-*`; факты добавляются рядом, тем же стилем (`fact="true"`, статус `impl/done`, `action="continue"
   actionstage="doc" audience="author"`).
6. Манифесты двух doc-пакетов: `vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0/vibe.toml`,
   `vibevm/vibepacks/org.vibevm.core/vibevm-docs-ru/v1.0.0/vibe.toml`; фикстура перевода
   `crates/vibe-doc/tests/fixture/translations/adaptation/**` (если поля обязательны — не делать их
   обязательными).
7. `vibevm/vibepacks/org.vibevm.doc/web/v0.1.0/docker/site.toml`, `site.example.toml`,
   `site/src/generated/doc-manifest.ts` (генерируется — не править руками, перегенерировать).

## Сделать — четыре атомарных коммита

**A. Авторство прозы.** В `[package]` doc-пакета необязательное поле `authorship` со значениями `human`,
`ai`, `mixed`. Смысл (текст факта, положи в PROP-057 рядом с `CARD-FIELDS` под id `CARD-AUTHORSHIP`):

> A `doc` package MAY declare `authorship` in `[package]`: `human`, `ai` or `mixed` — who wrote the prose
> the package carries. It is metadata of the document, kept for the reader who filters a shelf by it; it
> is never an attribution of the commits or of the repository, whose authorship law is
> `spec://org.vibevm.core/vibevm/common/PROP-000#commits`. Absent means unknown: the site shows no badge
> and a filter by authorship leaves the package out of both named groups.

Поле едет в проекцию: `DocPackage.authorship?` в схеме (enum), в Rust и TS. Оба doc-пакета получают
`authorship = "ai"` (слово владельца 2026-09-14: все существующие документы — ai-generated). Значение вне
трёх — ошибка `vibe check` с именем поля. Не-doc пакет с полем — ошибка (поле doc-пакетов).
Коммит: `feat(doc): let a documentation package say who wrote its prose`.

**B. Навигация: припинённые страницы и разделы.** Таблица `[navigation]` в манифесте doc-пакета:
`pinned = ["start/what-vibevm-is", "start/index"]` — пути документов без расширения, каждый должен
существовать (`vibe check` — ошибка с путём, если нет); `[[navigation.section]]` с `id` (первый сегмент пути
страниц, например `start`) и `title` (на языке пакета). Проекция: `DocManifest.navigation?: { pinned:
string[], sections: [{ id, title }] }`. Порядок остальных страниц не меняется (закон слоёв, порядок
манифеста). Оба doc-пакета объявляют: pinned — те два пути; разделы — все, что есть в дереве страниц
(`start`, `model`, `howto`, `agent`, `lifecycle`, `authoring`, `reference`, `architecture`, `diagnostics`,
`faq`, `glossary`), заголовки для источника — как в `start/index.xml` источника (Start, Model, How to, …;
если там нет прямых слов — коротко и по-английски), для адаптации — по-русски (Старт, Модель,
Как сделать, Агент, Жизненный цикл, Авторам, Справочник, Архитектура, Диагностика, Вопросы, Глоссарий).
Факт в PROP-057 (`NAV-PINNED`, рядом с `READER-*`):

> A documentation package MAY declare `[navigation]`: `pinned`, the document paths the site and the
> local reader list first, in the order given; and `[[navigation.section]]`, one row per top-level folder
> of the page tree with the title the navigation shows for it. Pinning changes only where the named pages
> stand; every other page keeps the manifest's order. A pinned path that names no page is an error of
> `vibe check`.

Коммит: `feat(doc): let a documentation package pin pages and name its sections`.

**C. Featured в конфигурации сайта.** `[site] featured = ["org.vibevm.core/vibevm",
"org.vibevm.core/vibevm-docs"]` в `site.toml` (координаты без версии; неизвестная координата — не
ошибка сборки, а предупреждение в отчёте сборки, потому что реестр меняется отдельно от конфигурации).
Схема `doc_site_config` и разбор в `site/config.rs`; в web-сборку — окружением `VITE_SITE_FEATURED`
(через запятую, без пробелов) из `web.rs`, тем же путём, что `DEFAULT_THEME`. `docker/site.toml` и
`site.example.toml` — значение и абзац. Коммит: `feat(doc): name the featured documentations in the
site configuration`.

**D. Проекции и происхождение бриджа в проекции пакета.** В `DocPackage` два дополнения: `projection:
bool` — истина для уровня 0 (страницы, которые сайт печёт из байтов пакета без doc-пакета, `level0.rs`),
ложь для настоящих doc-пакетов; и `bridge?: { maintainers: string[], upstream_authors: string[],
upstream_license?: string }` — только когда `[package].bridge = true`: `maintainers` — это
`[package].authors` (кто написал мост), `upstream_authors` — объединение `upstream_authors` всех
`[[embedded_source]]` без повторов, `upstream_license` — если у всех источников она одна. Обе вещи —
данные, которые уже есть в манифестах; проекция их только доносит. Факт-уточнение к PROP-023
`AUTHORSHIP-SEPARATION` не нужен — он уже говорит, что списки держат порознь; в PROP-057 рядом с
`LEVEL-ZERO` факт `LEVEL-ZERO-MARKED`:

> The manifest of a level-zero rendering says so: `projection = true`, so a shelf can tell a page the
> site derived from a package's own bytes from a page an author wrote, and mark the first as
> generated. A bridge's rendering carries the two authorships the bridge keeps apart.

Коммит: `feat(doc): mark level-zero renderings and carry a bridge's two authorships`.

Общее: каждое новое поле необязательно и не ломает существующие манифесты и фикстуры; схемы меняются
первыми, типы перегенерируются `cargo xtask check-codegen` (ровно так, как делает репозиторий; если
генератор пишет и `site/src/generated/doc-manifest.ts` — коммить его в том же атоме, это генерированный
файл web-пакета); `formats/REGISTRY.toml` — если формат требует записи об изменении, сделай её по
образцу соседей. Стиль кода и докстрингов — как у соседей: короткие «почему».

## Периметр файлов

`crates/vibe-doc/**`, `crates/vibe-wire/src/generated/**`, `crates/vibe-cli/src/commands/doc/site/web.rs`,
`crates/vibe-check/**` (только проверки новых полей), `crates/vibe-core/src/manifest/**` (только если поле
`authorship`/`[navigation]` разбирается там), `schemas/doc_manifest.jtd.json`,
`schemas/doc_site_config.jtd.json`, `formats/REGISTRY.toml`,
`vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml`, манифесты двух doc-пакетов,
`vibevm/vibepacks/org.vibevm.doc/web/v0.1.0/docker/site.toml`, `…/site.example.toml`,
`…/site/src/generated/doc-manifest.ts` (только генератором). Ничего другого в web-пакете, ничего в
руководстве (страницы), `specmap.json` не трогать.

## Самопроверка (обязательно, вывод в отчёт)

```
cargo fmt --all -- --check
cargo xtask check-codegen
cargo clippy -p vibe-doc -p vibe-cli -p vibe-check -p vibe-core --all-targets -- -D warnings
cargo test -p vibe-doc
cargo test -p vibe-core manifest
cargo test -p vibe-check
cargo build -p vibe-cli
target/debug/vibe.exe doc check --path vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0 --citations --derived --coverage --media --style --min 100
target/debug/vibe.exe doc check --path vibevm/vibepacks/org.vibevm.core/vibevm-docs-ru/v1.0.0 --citations --derived --translations --style --min 100
target/debug/vibe.exe check --path vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0
target/debug/vibe.exe doc manifest --path vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0 --json | head -60   # authorship, navigation видны
cargo xtask specmap --check   # красный только по новым фактам — напиши в отчёт, регенерирует центральная сессия
```

Если `--coverage` требует, чтобы новые факты с `audience` цитировала страница, — не пиши страницу: сними
`audience`/`action` с факта (оставь `fact="true"` и статус) и скажи в отчёте; прозу пишет центральная сессия.

## Приёмка боссом

Четыре коммита, каждый читается как PR; схемы первичны, генерированное совпадает; оба doc-пакета несут
`authorship = "ai"`, `[navigation]` с двумя pinned и разделами; `vibe doc manifest --json` показывает
новые поля; уровень 0 помечен `projection = true`; `site.toml` с `featured` разбирается и уезжает в
`VITE_SITE_FEATURED`; гейты зелёные, кроме `specmap --check` по новым фактам.

## Отчёт

`campaigns/docs-2026-09/findings/WORKER-REPORT-W1-O3.md` (не коммитить): хэши и subject'ы, что сделано
по каждому пункту, точные имена новых полей и значений в схемах, вывод гейтов дословно, аномалии, что
не сделано и почему.
