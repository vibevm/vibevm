# PACKET-W1-O3B — сайт: авторство прозы, бейдж GENERATED по полю, две подписи бриджа (web, после W1-O3 и W1-O4)

##subagent-quiet-clause

Ты — воркер-кодер (`opus5`, High). Читай ровно названные файлы; boot-лейн не читаешь.
Ветка `research-preview-1-docs`, ворктри `C:\Users\olegc\git\v\vibevm-docs`. Общий `git`-индекс с
центральной сессией. Коммитишь **только** формой `git commit -m … -- <пути>`, новые файлы — `git add --
<файл>`; никогда `git add -A`, никогда голый `git commit`, никаких трейлеров `Co-Authored-By` и никаких
упоминаний модели или агента в сообщениях коммитов — авторство репозитория человеческое (PROP-000
`#commits`). `git push` не делаешь. `cargo` не запускаешь. Никакие процессы, которых ты не запускал, не
останавливать.

## Зачем

W1-O3 (Rust) дал манифестам поля: `DocPackage.authorship` (`human` | `ai` | `mixed`, необязательное),
`DocManifest.navigation` (`pinned`, `sections`), `DocPackage.projection` (истина для уровня 0),
`DocPackage.bridge` (`maintainers`, `upstream_authors`, `upstream_license?`), а `site.toml` — `featured`
через `VITE_SITE_FEATURED`. W1-O4 сделал вкладки каталога, фильтр языка и бейдж GENERATED по эвристике.
Этот пакет доводит сайт до данных: замечания владельца 5, 6, 12 (`findings/OWNER-REVIEW-2026-09-14.md`).
Прочитай `findings/WORKER-REPORT-W1-O3.md` (точные имена полей и значений) и `findings/WORKER-REPORT-W1-O4.md`
(как сделаны вкладки и `isProjection`).

## Читать сначала, ровно эти файлы

1. `vibevm/vibepacks/org.vibevm.doc/web/v0.1.0/site/src/generated/doc-manifest.ts` — только читать; поля
   выше должны быть в типе (если нет — дефект пакета в отчёт, не додумывать).
2. `site/src/lib/library.ts`, `lib/view.ts`, `site/src/routes/doc/index.tsx`, `routes/doc/[...path]/index.tsx`,
   `site/src/components/catalogue/**`.
3. `design/src/components/card/**`, `doc-card/**`, `badge/**`, `package-header/**`, `tab-pills/**`,
   `language-selector/**` (или что W1-O4 использовал для фильтров), `docs-nav/**` (или колонка «Contents»
   из W1-O6, если он уже приземлился — `findings/WORKER-REPORT-W1-O6.md`).
4. `site/src/config.ts` (`ENV_NAMES.featured`), `README.md` web-пакета, `site/README.md`, `package.json`,
   тесты.

## Сделать — три атомарных коммита

**N. Проекции и авторство по данным.** `isProjection(pkg)` читает `pkg.projection` (эвристика W1-O4 —
только запасной путь, если поле не пришло, с комментарием). На каталоге, рядом с вкладками, выпадашка
«all / human-authored / ai-generated» (i18n сайта) по `authorship`: `human` — в human-authored, `ai` — в
ai-generated, `mixed` — в обоих, отсутствующее — только в all (PROP-057 `CARD-AUTHORSHIP`: неизвестное не
попадает ни в одну названную группу и не носит бейджа). На карточке — маленькая метка авторства
(`badge`, «AI» / «Human» / «Mixed»), когда поле есть; у проекций — бейдж GENERATED как сделал W1-O4.
Выбор запоминается (`localStorage`), действует внутри вкладки и вместе с фильтром языка.
Коммит: `feat(web): filter the catalogue by who wrote the prose`.

**O. Две подписи бриджа (12).** На карточке и в шапке страницы пакета, у которого есть `bridge`: «Bridge
maintainer» — `bridge.maintainers`, «Destination author» — `bridge.upstream_authors` (подписи через i18n:
«Сопровождает мост» / «Автор оригинала»), лицензия оригинала рядом, если пришла; у пакета без `bridge` —
как сейчас (`publisher`). Ничего не выводить из имён групп: только поля.
Коммит: `feat(web): show a bridge's maintainer apart from the upstream's author`.

**P. Припинённые страницы и заголовки разделов из манифеста.** Там, где сайт перечисляет страницы
руководства (колонка «Contents» W1-O6 или, если его ещё нет, `view.nav`), `navigation.pinned` идут
первыми в заданном порядке, заголовки разделов — `navigation.sections` (id → title), запасной путь W1-O6
(первый сегмент пути) — только когда поля нет. Порядок остальных страниц — порядок манифеста.
Коммит: `feat(web): list pinned pages first and name sections as the manual does`.

Общее: никаких инлайновых скриптов (счётчик 47) и внешних ресурсов; стиль как у соседей. Скриншоты
каталога с выпадашкой и карточки бриджа (фикстурная библиотека — проверь, есть ли в ней бридж; если нет,
добавь к фикстуре один пакет с `bridge = true` и `[[embedded_source]]`, минимально) —
`campaigns/docs-2026-09/findings/W1-O3B-shots/`.

## Периметр файлов

`vibevm/vibepacks/org.vibevm.doc/web/v0.1.0/site/**` и `design/**` (кроме `site/src/generated/**`,
`docker/**`, `site.example.toml`), фикстурная библиотека сайта (там, где она лежит в web-пакете),
`campaigns/docs-2026-09/findings/W1-O3B-shots/**`. Ничего в `crates/**`, `schemas/**`, руководстве, спеках.
(Если web-пакет уже переехал в `v1.0.0` — W1-O2 — работай там; центральная сессия скажет при запуске.)

## Самопроверка (обязательно, вывод в отчёт)

```
pnpm -C vibevm/vibepacks/org.vibevm.doc/web/v0.1.0 floor
pnpm -C vibevm/vibepacks/org.vibevm.doc/web/v0.1.0 build:static
pnpm -C vibevm/vibepacks/org.vibevm.doc/web/v0.1.0 build:embedded
pnpm -C vibevm/vibepacks/org.vibevm.doc/web/v0.1.0 test:e2e
```

## Приёмка боссом

Три коммита, каждый читается как PR; выпадашка делит каталог по `authorship`, неизвестное — только в all;
проекции помечены по полю; бридж показывает две подписи из полей; pinned первыми, заголовки разделов из
манифеста; гейты зелёные; скриншоты.

## Отчёт

`campaigns/docs-2026-09/findings/WORKER-REPORT-W1-O3B.md` (не коммитить; скриншоты — коммитить с N и O):
хэши и subject'ы, что сделано по каждому пункту, вывод гейтов дословно, аномалии, что не сделано и почему.
