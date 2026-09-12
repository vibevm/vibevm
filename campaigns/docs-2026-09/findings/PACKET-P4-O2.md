# PACKET-P4-O2 — оболочка ридера: поведение, оглавление, таблицы, фоллбэк (A4.3, A4.11, A4.12, A4.13)

##subagent-quiet-clause

Ты — воркер кампании документации, исполнитель уровня `opus5`. Boot-лейн
репозитория не читать. Твоя инструкция — этот файл и ровно те файлы,
которые он называет. Предусловие: коммиты P4-O1 (`4ccc9438`, `3e0efc08`,
`90f21883`, `8ed08eca`) в ветке — проверь `git log --oneline -60` и
прочитай `campaigns/docs-2026-09/findings/WORKER-REPORT-P4-O1.md` целиком
(решения 1–4, аномалии А-1 и А-2 обязательны для тебя: `<Slot />` в layout,
порядок сборок в `tools/build.mjs`).

## Читать сначала

1. Этот пакет целиком; отчёт P4-O1; `WORKER-REPORT-P2-O6.md` §A2.13
   (словарь HTML-острова: `.p-anchor`, `blockquote.rule`, `pre.derived`,
   `.example`, `aside.note`, `figure`, `.prompt`, `[data-when]`,
   `[data-unresolved]`, таблицы) и §A2.22; `WORKER-REPORT-P2-O7.md` §A2.14
   (что несёт манифест: статусы, языки, порядок слоёв, четыре тира).
2. `campaigns/docs-2026-09/PLAN.md` — атомы **A4.3**, **A4.11**, **A4.12**,
   **A4.13**; правила **R-09**, **R-26** (номера блоков — только из
   острова; клиент не считает), **R-27**.
3. Находки: `findings/A0.22-reader-inventory.md` (таблица фич и константы
   ридера, внешние зависимости скриптов); `findings/A0.10-qwik-probe.md` §3
   (базовый путь) и §6.
4. Решения: `vibevm/vibespecs/design/documentation-vision.xml` **D-22**
   целиком (десять фич ридера — это твой список приёмки), D-18 и D-19
   (селектор языка: официальные со звёздочкой первыми, community после,
   издатель у каждого), D-20 (полки и карточки: звёздочка, подпись,
   издатель, плейсхолдер, аннотация по клику), D-21 п. 6–8 (типографика,
   компоненты, образ страниц), D-06 (адреса).
5. Норма: PROP-057 `reader` (READER-NUMBERED-BLOCKS,
   READER-LANGUAGE-SWITCH-KEEPS-PLACE, READER-SETTINGS, READER-NO-AUTOSCROLL,
   READER-FOR-AGENT, READER-META-AND-PRINT), `site` (SITE-MOUNT,
   SITE-CANONICAL-LATEST, SITE-VERSION-SHOWS-CURRENT), `localization`
   (LOC-* в части фоллбэка), `stack` (STACK-DESIGN-FLOOR, STACK-FLOOR).
6. Образцы для переноса, **только чтение** (R-28):
   `C:\Users\olegc\git\oleg-guru\public\js\theme.js`, `lang-fallback.js`,
   `lightbox.js`, `scripts\templates\article.html.tpl` (инлайн-скрипты
   «Paragraph anchors», «reading position»);
   `C:\Users\olegc\git\talks\2026.08.08-agents-talk\design-system\public\assets\js\ui.js`
   (подсветка `.toc` через IntersectionObserver, `rootMargin '-15% 0px -70% 0px'`).
7. Код пакета: `vibevm/vibepacks/org.vibevm.doc/web/v0.1.0/` — `design/`
   (токены, компоненты, `theme-init.js`), `site/src/` (`components/island`,
   `lib/href.ts`, `lib/island-target.ts`, `lib/manifest.ts`, `lib/pages.ts`,
   `routes/doc/**`, `routes/layout.tsx`, `root.tsx`), `tools/build.mjs`,
   `site/README.md`, фикстуры `site/src/fixtures/`.
8. Правила репозитория: Conventional Commits с телом «почему»; атом —
   коммит; **коммитить только явной формой `git commit -m … -- <свои пути>`**;
   **никаких трейлеров и упоминаний моделей**; `git push` не делать; чужие
   незакоммиченные файлы не стейджить (в дереве работают другие воркеры:
   P2-O8 в `crates/**` и `DEV-GUIDE.md`; **P4-O5 в том же web-пакете** —
   его периметр `site/src/routes/index.tsx`, `site/src/routes/ru/**`,
   `site/src/routes/404*`, `site/src/landing/**`, `site/public/**`,
   `design/src/components/{hero,dep-graph,capability-card}/**`,
   `tools/parity.mjs`; ты туда не пишешь, он не пишет в твой периметр ниже);
   `cargo` не запускать; сеть — только `pnpm install` из настроенного
   npm-реестра и загрузка браузера Playwright (`pnpm exec playwright
   install chromium`); `node_modules/`, `dist*/`, `server*/`, `tmp/`,
   `test-results/` — в `.gitignore`, никогда в коммите; секреты и `infra/`
   не читать.

## Твой периметр файлов

`site/src/routes/doc/**`, `site/src/routes/layout.tsx` (только селектор
языка и мета-блок в шапке, согласованно с O5 — правки минимальные),
`site/src/reader/**` (новое: поведение ридера), `site/src/lib/**` (новые
файлы и правки своих), `site/src/components/**` кроме `island` (правки
своих), `design/src/components/**` кроме перечисленных за O5 (правки и
новые: `toc`, `lightbox`, `code-block`, `settings-panel`, `for-agent`,
`return-to-place`, `language-selector`, `shelf`, `card`), `design/base.css`,
`design/print.css` (новое), `site/tests/**` (Playwright), `site/package.json`
и корневой `package.json` (только добавление Playwright и скриптов
`test:e2e`), `pnpm-lock.yaml`, `site/README.md`.

## Цель — четыре атома, коммит на атом (или на компонент, где сказано)

**A4.3 Оболочка.** Layout страницы документации из компонентов P4-O1:
навигация из манифеста (порядок слоёв — сигнал манифеста, не список
секций), переключатель версий, переключатель платформ по `data-when`
(`tab-pills`: показывает блоки одной платформы, скрывает остальные, выбор
сохраняется), всплывающая транслюзия правила (клик по `blockquote.rule` —
текст факта уже внутри острова, показать его рядом, не грузить), подсветка
якоря, ссылки «.md» и «.xml» (`projectionHref()`), секция «для агентов»;
**селектор языка** по D-18/D-19: официальные со звёздочкой первыми,
community после, издатель у каждого, ведёт на ту же страницу с тем же
фрагментом; **полки и карточки** по D-19/D-20 для страницы пакета:
звёздочка, подпись, порядок, издатель, иконка или плейсхолдер, аннотация по
клику; шапка страницы пакета с баннером или плейсхолдером. Коммит на
компонент: `feat(web): …`.

**A4.11 Ридер.** Обработчики над островом делегированием (остров не
реактивен; `islandTarget()` уже разводит клики): клик по `.p-anchor` →
`history.replaceState('#pNN')` + полный URL в буфер + класс `copied` на
1.5 с; открытие с `#pNN` → `scrollIntoView({block:'center'})`;
переключатель якорей с сохранением; панель настроек — тема
(тёмная/светлая/системная через `theme-init.js`), `A−`/`A+` по шагам
80…150, `W−`/`W+` 740…1400 на десктопе, «Сброс», значения в `localStorage`
(каждое чтение и запись в `try`); режим чтения по порогу 40 % высоты окна;
сохранение ближайшего якоря раз в секунду в `reading-pos:<путь>`, кнопка
«вернуться» вместо авто-скролла, `history.scrollRestoration = 'manual'`,
сброс при клике по пункту оглавления; кнопка «для агента» (`fab`) с
`spec://…@<версия>/…#pNN`, ссылками `.md`, `.xml`, `llms.txt` пакета и
копированием. В embedded-режиме те же настройки через `postMessage`
`{ "settings": … }` и `{ "theme": … }`, `{"open": "spec://…"}` внутрь,
`{"openFile": "…"}` наружу (контракт LOCAL-EMBEDDING-CONTRACT). Тесты
Playwright (`site/tests/*.spec.ts`, статическая сборка + `vite preview`
или статический сервер на `127.0.0.1`): открыть `#p07` → блок в центре;
перезагрузка показывает «вернуться» и не скроллит сама; смена языка
сохраняет фрагмент; тема переживает перезагрузку без вспышки (проверка
`data-theme` до первого кадра). Коммит:
`feat(web): port the reader behaviours`.

**A4.12 Оглавление и остальное.** Липкое оглавление 190px с подсветкой
активного пункта (IntersectionObserver, `rootMargin '-15% 0px -70% 0px'`);
`<details>` на узких экранах и при ширине колонки выше 1100px; блок
«правила страницы» из элементов `blockquote.rule`; сноски с обратными
ссылками; таблицы в `table-scroll` с кнопкой «развернуть» в overlay,
зебра, первый столбец жирный; лайтбокс картинок (overlay
`position:fixed; inset:0`, без `backdrop-filter`); кнопка копирования и
подпись языка у fence; вывод `expect` под `example` (уже в острове —
оформить); пометка `derived` с адресом источника; `@media print` без
панелей, с номерами и адресами ссылок. Коммит на компонент.

**A4.13 Фоллбэк перевода и cookie `lang`.** Статический адаптер
материализует страницу-фоллбэк для каждой страницы, отсутствующей в
переводе: `<html lang>` источника, `rel=canonical` на источник, `<meta
name="robots" content="noindex">`, вне sitemap (список страниц — из
манифеста: страница есть у языка или нет); клиент — плашка один раз за
сессию (`sessionStorage`), переписывание внутренних ссылок под язык
адреса; cookie `lang` (365 дней, `SameSite=Lax`, `path=/`) при заходе на
страницу с языковым сегментом; каталог `/doc/` без языка читает cookie,
потом `Accept-Language`, потом исходный язык (на статике — клиентский
редирект, документируй). Фикстура: `site/src/fixtures/manifest.json`
получает вторую страницу и один перевод `ru` с пропуском, чтобы фоллбэк
было на чём собрать и проверить. Коммит:
`feat(web): fall back to the source language without losing the reader`.

## Гейты (вывод дословно в отчёт)

`pnpm floor` (семь шагов + аудит контраста) — зелёный; `pnpm build:static`
и `pnpm build:embedded` — число страниц из манифеста фикстуры (после A4.13
— с фоллбэками) сходится; `pnpm test:e2e` (Playwright, chromium, headless)
— все тесты зелёные, вывод в отчёт; `grep -rn "#[0-9a-fA-F]\{3,8\}"
design/src site/src` — пусто; ни одного внешнего адреса в бандле, кроме
`vibevm.org` (grep по `dist/` на `https://` — в отчёт дословно, X-020 про
адрес превью Qwik в тексте ошибок беты — известен, не считать);
`bash tools/self-check.sh` — только шаг web-пакета.

## Что не делать

Остров не делать реактивным и не перерисовывать; номера блоков не
считать на клиенте (R-26); Tailwind — нет (F-75); литералы цвета — нет
(R-27); внешние CDN, шрифты по сети, аналитика — нет (R-09; тег Umami —
атом A4.4/A5.1); лендинг, SEO, корневые машинные файлы, крейт оболочки —
не начинать; периметр P4-O5 не трогать; PROP-файлы и вижен не менять;
`cargo` не запускать; браузер Playwright ставить в кэш пользователя, не в
дерево.

## Результат

Коммиты (без push) и `campaigns/docs-2026-09/findings/WORKER-REPORT-P4-O2.md`
по форме отчёта P4-O1: решения, что из D-22 сделано и что нет с причиной,
вывод гейтов дословно, аномалии беты Qwik, три скриншота 1440 и 390 px
обеих тем страницы с примером, таблицей и правилом — в
`campaigns/docs-2026-09/findings/P4-O2-shots/` (PNG, для дизайн-ревью
A4.14; суммарно не больше 3 МБ). Никаких путей вне репозитория, IP и
секретов в отчёте (R-25).
