# PACKET-W1-O4 — сайт: язык сайта и язык документации, три вкладки каталога, выбор версии (web, без Rust)

##subagent-quiet-clause

Ты — воркер-кодер (`opus5`, High). Читай ровно названные файлы; boot-лейн не читаешь.
Ветка `research-preview-1-docs`, ворктри `C:\Users\olegc\git\v\vibevm-docs`. Общий `git`-индекс с
центральной сессией и Rust-воркером W1-O3 (он в `crates/**`, `schemas/**`, спеках, манифестах doc-пакетов,
`docker/site.toml`, `site.example.toml` и генерирует `site/src/generated/doc-manifest.ts`; эти файлы ты не
трогаешь). Коммитишь **только** формой `git commit -m … -- <пути>`, новые файлы — `git add -- <файл>`;
никогда `git add -A`, никогда голый `git commit`, никаких трейлеров `Co-Authored-By` и никаких упоминаний
модели или агента в сообщениях коммитов — авторство репозитория человеческое (PROP-000 `#commits`).
`git push` не делаешь. `cargo` не запускаешь: всё нужное — `pnpm` в web-пакете и готовый
`target/debug/vibe.exe`. Никакие процессы, которых ты не запускал, не останавливать.

## Зачем

Владелец после первого просмотра (`findings/OWNER-REVIEW-2026-09-14.md`, пп. 4, 7, 15): язык интерфейса
сайта и язык документации — две разные вещи, сегодня перемешанные в одном переключателе; каталог `/doc`
должен делиться на featured / documents / projections; на каждой странице руководства всегда виден выбор
версии. W1-O1 уже сделал переключатель темы, поиск, карточки и панель чтения (`findings/WORKER-REPORT-W1-O1.md`,
прочитай разделы B, C и «Аномалии»: двойная шапка лендинга и её починка в `routes/layout.tsx`).

## Читать сначала, ровно эти файлы

1. `vibevm/vibepacks/org.vibevm.doc/web/v0.1.0/README.md`, `site/README.md`.
2. `site/src/lib/view.ts` (`headerLanguageChoices`, `catalogueChoices`, `languageChoices`), `lib/href.ts`
   (`docSegments`, `parseDocTarget`), `lib/library.ts`, `lib/search.ts` — данные и адреса.
3. `site/src/routes/layout.tsx`, `routes/layout-landing!.tsx`, `routes/doc/layout.tsx`, `routes/doc/index.tsx`,
   `routes/doc/[...path]/index.tsx` — каталог, страница документации, страница документа, страница пакета.
4. `site/src/landing/chrome.tsx`, `landing/chrome.css`, `landing/i18n.ts`, `landing/landing.tsx`.
5. `site/src/reader/**` (особенно `theme.ts`, `settings.ts`, `storage.ts`, `dom.ts`) — как страница
   получает поведение без инлайновых скриптов; `design/theme-init.js`.
6. `design/src/components/language-selector/**`, `version-switch/**`, `tab-pills/**`, `docs-header/**`,
   `theme-switch/**`, `search-box/**`, `badge/**`, `shelf/**`, `card/**`, `doc-card/**`.
7. `site/src/config.ts` (`ENV_NAMES`, `SITE`), `site/src/generated/doc-manifest.ts` (только читать: `DocPackage`,
   `DocPage`, `translation`, `status`, `subjects`).
8. `site/src/components/catalogue/**`, `site/src/seo/**` (hreflang, sitemap — чтобы не сломать),
   `tools/build.mjs`, `tools/lint-links.mjs`, `package.json`, тесты Playwright (`site/tests/**` или где лежат).

## Сделать — три атомарных коммита

**A. Язык сайта ≠ язык документации.** Правый верхний угол на **всех** страницах — язык интерфейса сайта
(`en` / `ru`): подписи шапки и подвала, названия полок, кнопки, метки («Documentation», «Pages», «Copy»,
«Ctrl K» и т. д.). На лендинге язык интерфейса — это уже адреса `/` и `/ru/`; переключатель ведёт на них.
На страницах документации (`/doc/**`) интерфейс тоже говорит на языке сайта: выбор хранится в
`localStorage` (ключ рядом с темой в `reader/storage.ts`), умолчание — из `navigator.language` (`ru*` → `ru`,
иначе `en`), применяется клиентским кодом к строкам интерфейса (одна небольшая таблица строк, en и ru;
`lang` на `<html>` не трогать — это язык документа). На страницах документации отдельно — **фильтр языка
документации** «English / Русский / Everything» (`tab-pills` или `language-selector`, что ближе):
на каталоге `/doc` и на странице документации он фильтрует карточки по языку издания (Everything — все);
на странице документа он предлагает языки, в которых документ есть (источник и адаптации, из `translation`
и рёбер библиотеки), и Everything, которое ничего не фильтрует; выбор языка ведёт на адрес того же документа
в этом языке (как сегодняшний `languageChoices`), а если перевода нет — на источник с существующей
пометкой (READER-LANGUAGE-SWITCH-KEEPS-PLACE). Фильтр запоминается (тот же `localStorage`), не влияет
на `hreflang`/sitemap. Сегодняшний смешанный переключатель убрать.
Коммит: `feat(web): tell the site's language from the documentation's`.

**B. Каталог `/doc`: три вкладки — Featured, Documents, Projections.** Featured — документации из
`SITE.featured`: новое имя окружения `ENV_NAMES.featured = "VITE_SITE_FEATURED"` (координаты через запятую;
W1-O3 научит `vibe doc build-site` его передавать; до этого — умолчание в `config.ts`:
`["org.vibevm.core/vibevm", "org.vibevm.core/vibevm-docs"]`); в featured входит и проекция самого
пакета `org.vibevm.core/vibevm` (уровень 0), и его официальная документация. Documents — все doc-пакеты
(featured входят). Projections — проекции уровня 0 (страницы, которые сайт печёт из байтов пакета без
doc-пакета): определи по данным библиотеки, как сайт отличает их сегодня (`lib/library.ts`,
`routes/doc/index.tsx`); когда W1-O3 добавит `DocPackage.projection`, это станет одним полем — оставь
одну функцию `isProjection(pkg)` с комментарием. На карточках проекций — бейдж `GENERATED`
(`design/src/components/badge`). Вкладки — `tab-pills`; активная вкладка в адресе (`/doc/?tab=projections`
или hash) и запоминается; по умолчанию Featured. Фильтр языка (A) действует внутри вкладки.
Коммит: `feat(web): shelve the catalogue as featured, documents and projections`.

**C. Выбор версии на каждой странице руководства.** `version-switch` показывается всегда, и при одной
версии (одна опция, не спрятана), рядом с фильтром языка документации в шапке документации; на странице
документа переключение ведёт на тот же документ другой версии, если он там есть, иначе на её первую
страницу с пометкой. `latest` остаётся первым и подписан как сегодня.
Коммит: `feat(web): always offer the version of the manual being read`.

Общее: никаких инлайновых скриптов (CSP считает хэши: число уникальных инлайновых скриптов должно
остаться 47 — гейт `build:static` это проверяет) и никаких внешних ресурсов (тест A4.15 «только
loopback»); тексты интерфейса — через i18n; стиль кода и докстрингов — как у соседей (короткие «почему»).
Скриншоты 1440 и 390, обе темы: каталог с тремя вкладками, шапка документации с двумя переключателями и
версией, лендинг с переключателем языка сайта — в `campaigns/docs-2026-09/findings/W1-O4-shots/`
(Playwright, как W1-O1).

## Периметр файлов

`vibevm/vibepacks/org.vibevm.doc/web/v0.1.0/site/**` и `design/**` (кроме `site/src/generated/**`,
`docker/**`, `site.example.toml`), `campaigns/docs-2026-09/findings/W1-O4-shots/**`. Ничего в `crates/**`,
`schemas/**`, руководстве и спеках.

## Самопроверка (обязательно, вывод в отчёт)

```
pnpm -C vibevm/vibepacks/org.vibevm.doc/web/v0.1.0 floor          # prettier, tsc, тесты, eslint, conform, specmap, APCA
pnpm -C vibevm/vibepacks/org.vibevm.doc/web/v0.1.0 build:static   # фикстурная библиотека; линтер ссылок; счётчик инлайновых скриптов
pnpm -C vibevm/vibepacks/org.vibevm.doc/web/v0.1.0 build:embedded # гейт PUBLIC_ONLY
pnpm -C vibevm/vibepacks/org.vibevm.doc/web/v0.1.0 test:e2e       # W1-O1 гонял: 38 passed, 2 skipped
```

## Приёмка боссом

Три коммита, каждый читается как PR; в шапке документации два разных переключателя и версия; каталог
показывает три вкладки с верным составом (featured: `org.vibevm.core/vibevm` и его документация;
projections с бейджем GENERATED); язык интерфейса переключается на лендинге и на `/doc/**` независимо от
языка документации; гейты зелёные; скриншоты.

## Отчёт

`campaigns/docs-2026-09/findings/WORKER-REPORT-W1-O4.md` (не коммитить; скриншоты — коммитить с B): хэши и
subject'ы, что сделано по каждому пункту, как сайт отличает проекции сегодня, вывод гейтов дословно,
аномалии, что не сделано и почему, решения, которые пришлось принять самому.
