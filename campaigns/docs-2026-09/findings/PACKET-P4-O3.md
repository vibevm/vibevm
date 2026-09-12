# PACKET-P4-O3 — статический адаптер и SEO документации, агентские файлы сайта (A4.4, A4.5)

##subagent-quiet-clause

Ты — воркер кампании документации, исполнитель уровня `opus5`. Boot-лейн
репозитория не читать. Твоя инструкция — этот файл и ровно те файлы,
которые он называет. Предусловия: коммиты P4-O1 (`8ed08eca`), P4-O5
(`2780ba06`, `131bdd1d`) и P4-O2 (оболочка ридера, фоллбэк перевода) в
ветке — проверь `git log --oneline -80` и прочитай отчёты
`WORKER-REPORT-P4-O1.md` (решения 3–4, А-1, А-2), `WORKER-REPORT-P4-O5.md`
(решения 2–6: конфигурация из окружения, язык документа, `og.png`,
шрифты, корневые файлы; таблица краулеров; список осознанных отличий
паритета), `WORKER-REPORT-P4-O2.md` (фоллбэк-страницы, что оболочка ждёт
от статики, тесты Playwright), `WORKER-REPORT-P2-O8.md` §A2.18 (что
кладёт `vibe doc build`: адресная карта, проекции рядом, машинные файлы в
корне выхода).

## Читать сначала

1. Этот пакет целиком; четыре отчёта выше.
2. `campaigns/docs-2026-09/PLAN.md` — атомы **A4.4**, **A4.5**; правила
   **R-09**, **R-10**, **R-27**; развилки **F-15** (резолвер на статике —
   статическая карта редиректов), **F-38** (краулеры — уже проверены P4-O5,
   не перепроверяй, возьми его список), **F-75**; `DEFERRALS.md` X-035
   (хэш инлайн-скрипта темы для CSP), X-040 (пути ассетов и правило кеша
   nginx — не твоё, но выход сборки должен их называть).
3. Решения: `vibevm/vibespecs/design/documentation-vision.xml` D-06
   (адреса, `latest`, canonical в пределах языка), D-18/D-19 (языки,
   официальность), D-20 (превью 1200×630 из плейсхолдера), D-24 (тег
   Umami — из конфигурации P4-O5, на страницах документации тоже), D-27
   (никаких ревизий и «устарело» в мете).
4. Норма: PROP-057 `seo` целиком (SEO-LEAD, SEO-SSR,
   SEO-CANONICAL-HREFLANG, SEO-SITEMAP, SEO-ROBOTS, SEO-CHARSET-AND-REDIRECTS,
   SEO-INDEXNOW, SEO-STRUCTURED-DATA, SEO-LLMS-FILES, SEO-RAW-PROJECTIONS,
   SEO-MANIFEST-AND-RESOLVER, SEO-LINK-GRAPH, SEO-PAGE-TEMPLATE,
   SEO-LOCAL-EXEMPT), `site` (SITE-MOUNT, SITE-TRAILING-SLASH,
   SITE-CANONICAL-LATEST, SITE-VERSION-SHOWS-CURRENT, SITE-ONE-SITE,
   SITE-ANALYTICS), `card` (CARD-PREVIEW-COMPOSED, CARD-SITE-COPIES),
   `stack` (STACK-PAGE-COUNT-GATE, STACK-BUILD-HYGIENE).
5. Код пакета: `vibevm/vibepacks/org.vibevm.doc/web/v0.1.0/` — `site/src/routes/doc/**`,
   `site/src/lib/{href,manifest,pages,view}.ts`, `site/src/landing/head.ts`
   (образец мета-тегов и JSON-LD у лендинга — та же форма для
   документации), `site/src/config.ts`, `tools/build.mjs`,
   `tools/root-files.mjs`, `tools/parity.mjs`, `site/adapters/static/**`,
   `site/src/fixtures/manifest.json`; выход `vibe doc build --format html
   --out <каталог>` руководства (собери его хостовым `target/debug/vibe.exe`
   в scratch — это твой источник островов, проекций, манифеста и `llms`).
6. Правила репозитория: Conventional Commits с телом «почему»; атом —
   коммит; **коммитить только явной формой `git commit -m … -- <свои пути>`**;
   **никаких трейлеров и упоминаний моделей**; `git push` не делать; чужие
   незакоммиченные файлы не стейджить (в дереве могут работать P2-O10 в
   `crates/**` и P4-O4 в `crates/**` и `xtask/**` — web-пакет они не
   правят, только собирают `pnpm build:embedded`); `cargo` не запускать;
   сеть — только `pnpm install` из настроенного npm-реестра;
   `node_modules/`, `dist*/`, `server*/`, `tmp/` — в `.gitignore`; секреты и
   `infra/` не читать; Node и pnpm на машине есть.

## Твой периметр файлов

`site/src/routes/doc/**` (только `<head>` и мета, согласованно с тем, что
P4-O2 оставил), `site/src/seo/**` (новое), `site/src/lib/{head,sitemap,resolve}*.ts`
(новые), `tools/build.mjs` (шаг копирования выхода `vibe doc build` и
линтер), `tools/root-files.mjs` (только добавление `/doc/sitemap.xml` в
индекс sitemap и `/doc/llms.txt` — если P4-O5 не сделал), `tools/lint-links.mjs`
(новое), `site/adapters/static/**`, `site/README.md` (раздел SEO и
агентские файлы), `site/src/fixtures/**` (расширение фикстуры, не
пересоздание).

## Цель — два атома, два коммита

**A4.4 Статический адаптер и SEO.** Пререндер маршрутов документации
на всех языках манифеста (источник — `vibe doc build`: страницы всех
языков и фоллбэки P4-O2); `rel=canonical` на `latest` в пределах языка
(D-06: адрес с версией — canonical на `latest` того же языка; фоллбэк —
canonical на источник); `hreflang` для каждого языка и `x-default`;
`sitemap.xml` документации по адресу `/doc/sitemap.xml`, индексный по
пакетам и языкам, без фоллбэков и без `noindex`-страниц, и запись о нём
в корневой индекс sitemap лендинга; JSON-LD на странице документации
(`headline`, `abstract`, `author` — издатель из манифеста, `inLanguage`,
`keywords`, `datePublished` — дата рендера из манифеста, `image` —
превью); Open Graph и Twitter Card `summary_large_image` с `preview`
пакета (сгенерированное превью из выхода `vibe doc build`, скопированное
под своим хэшированным именем); тег Umami из `site/src/config.ts` на
страницах документации тем же способом, что у лендинга; `<meta charset>`
и завершающий слэш везде; CSP: сборка пишет `dist/csp.txt` — строку
политики без внешних источников с `sha256` **каждого** инлайн-скрипта в
собранных страницах (после P4-O2 их два: инициализация темы и отключение
восстановления прокрутки роутером), вычисленными из байтов собранного
HTML, а не захардкоженными (X-035), для атома развёртывания; тест: набор
хэшей в `csp.txt` равен набору инлайн-скриптов в `dist/`. Фильтр
фоллбэк-страниц (`noindex`) из sitemap, который P4-O2 положил в
`tools/build.mjs`, переезжает в `tools/root-files.mjs`, где живут
остальные корневые файлы. Проверка:
`tools/lint-links.mjs` над `dist/` — каждая внутренняя ссылка ведёт на
существующий файл, каждая пара `hreflang` взаимна, каждый canonical
существует, ни одного `https://` вне `vibevm.org` и списка эталона
(тот же список, что у `parity.mjs`); линтер — шаг `tools/build.mjs`
статической сборки, красный красит сборку. Коммит:
`feat(web): prerender the site with the SEO contract`.

**A4.5 Агентские файлы сайта.** `llms.txt`, `llms-small.txt`,
`llms-medium.txt`, `llms-full.txt` на язык и на пакет по адресам
`/doc/<lang>/<группа>/<имя>/<версия>/…` и общий `/doc/llms.txt`;
`manifest.json` по `/doc/manifest.json` (и на пакет); проекции
`<страница>.md` и `.xml` рядом с каждой страницей — всё копируется из
выхода `vibe doc build` в `dist/` статической сборки одним шагом
`tools/build.mjs` (сборка сайта не пересчитывает то, что считает Rust);
резолвер `/doc/resolve?uri=spec://…` — на статике карта редиректов
(F-15): файл `dist/doc/resolve/index.html` с клиентским разбором `uri`
и таблица адрес → страница#якорь в `dist/doc/resolve.json`,
сгенерированная из манифеста и якорей; `robots.txt` лендинга остаётся
один на домен (SITE-ONE-SITE) — проверь, что он не запрещает `/doc/`
и что `Sitemap:` называет `/doc/sitemap.xml`. Коммит:
`feat(web): publish the agent surfaces beside the pages`.

## Гейты (вывод дословно в отчёт)

`pnpm floor` — зелёный; `pnpm build:static` над выходом `vibe doc build`
руководства — число страниц сходится (все языки, фоллбэки, лендинг),
`tools/lint-links.mjs` зелёный, вывод в отчёт; `node tools/parity.mjs`
P4-O5 остаётся зелёным (лендинг не изменился); `pnpm test:e2e` P4-O2
остаётся зелёным; `grep -rn "#[0-9a-fA-F]\{3,8\}" design/src site/src` —
пусто; `du -sh dist` и число файлов — в отчёт; `bash tools/self-check.sh`
— только шаг web-пакета.

## Что не делать

Лендинг и его корневые файлы не переписывать (только записи sitemap и
`llms.txt`, если их нет); ридер и остров не трогать; крейты — нет;
`vibevm-org` не править; Tailwind, внешние CDN, шрифты по сети — нет;
`nginx.conf` и любой серверный файл — нет (R-24; правило кеша ассетов
— X-040, атом развёртывания); значения website-id и ключа IndexNow —
только из окружения сборки; PROP-файлы и вижен не менять.

## Результат

Два коммита (без push) и `campaigns/docs-2026-09/findings/WORKER-REPORT-P4-O3.md`
по форме отчёта P4-O5: таблица «что у страницы документации в `<head>`»
с примером для одной страницы и одного фоллбэка, список файлов, которые
копируются из `vibe doc build`, форма `resolve.json`, вывод линтера и
гейтов дословно, что не сделано и почему. Никаких путей вне репозитория,
IP и секретов в отчёте (R-25).
