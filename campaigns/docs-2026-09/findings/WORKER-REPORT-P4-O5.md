# WORKER-REPORT-P4-O5 — лендинг на Qwik и тест паритета

Пакет: `campaigns/docs-2026-09/findings/PACKET-P4-O5.md` (A4.16, A4.17).
Ветка `research-preview-1-docs`, без push. Дата: 2026-09-12.

## Коротко для оркестратора

Два атома сделаны, два коммита в ветке. Лендинг `/` и `/ru/` (плюс `/404.html`
и `/en/`) собирается тем же адаптером, что и документация, из компонентов
`design/`; корневые машинные файлы домена генерирует сборка. Тест паритета —
**зелёный: 35 осознанных отличий, 0 необъяснённых**.

Четыре результата, которые стоит прочитать отдельно.

1. **Тест паритета нашёл настоящий дефект с первого прогона, и ровно того
   рода, что предсказывает предсказание 13:** страница `/ru/` рендерилась под
   `lang="en"`. `<html lang>` — контейнерный атрибут, его пишет серверный
   вход до того, как исполнится хоть один компонент, и ни один маршрут его не
   задаёт. Видимого следа нет; нашлось только сравнением.
2. **Qwik 2.0.0-beta.43: вложенный именованный layout резолвится
   недетерминированно.** Две сборки одних и тех же файлов маршрутов дали
   разные цепочки layout'ов — во второй лендинг получил документационную
   шапку **поверх** лендинговой. Лечение — top-форма имени
   (`layout-landing!.tsx`), после которой резолвер обрывает цепочку сразу.
   Подробности и метод — «Аномалии», А-1.
3. **`pnpm floor`: пять шагов зелёные плюс APCA-аудит (40 гейтуемых пар, 0
   ниже порога); шестой шаг, `specmap`, красный** — и это общий
   генерируемый индекс, а не мой код. Перегенерация покрывает и мои файлы, и
   шесть незакоммиченных модулей соседнего воркера; см. «Гейты» и «Что не
   сделано».
4. **Два общих файла пришлось коммитить со смешанным содержимым**
   (`design/src/index.ts`, `tools/build.mjs`) — иначе атом не собирается.
   Разбор — в конце, раздел «Общие файлы».

## Хэши и subject'ы

| Атом | Коммит | Subject |
|---|---|---|
| A4.16 | `2780ba06` | `feat(web): port the landing onto the shared design system` |
| A4.17 | `131bdd1d` | `test(web): pin landing parity against the Astro build` |

Между атомами и вокруг них в ветку ложатся коммиты соседнего воркера
(`8fe694b0`, `68497e29` и далее) — ожидаемо; чужого я не добавлял, кроме двух
общих файлов, о которых сказано отдельно.

## A4.16 — лендинг

### Таблица переноса: было в Astro → стало в Qwik

По разделам инвентаря A0.26.

| A0.26 | Было (`vibevm-org`) | Стало (`org.vibevm.doc/web`) |
|---|---|---|
| §1.1 константы | `src/i18n.ts:5,8,9` — `LOCALES`, `GITHUB_URL`, `GITVERSE_URL` | `site/src/landing/i18n.ts` — те же три, значения дословно |
| §1.2 строки обеих локалей | `STRINGS.en` (34–71), `STRINGS.ru` (72–109) | `site/src/landing/i18n.ts`, `STRINGS` того же типа; **все значения байт в байт**, включая асимметрии (регистр eyebrow, точка в конце EN-заголовка, непереводимые `installBash`/`installPowerShell`) |
| §1.2 хардкод разметки | команды установки `Landing.astro:46,54,60` | `site/src/landing/i18n.ts`, объект `INSTALL` — перенесены в таблицу строк, чтобы перенос был проверяем |
| §1.3 `DependencyGraph` | `src/components/DependencyGraph.astro` | `design/src/components/dep-graph/` — тот же `viewBox`, те же координаты, те же `--d`-задержки на элементах; `draw`/`pop`/`pulse` в `styles.css`, свой блок `prefers-reduced-motion` |
| §1.3 `Landing` | `src/components/Landing.astro` | `design/src/components/hero/` (`Hero` + `InstallBlock`), `design/src/components/capability-card/` (`CapabilityCard` + `CapabilityRow`), композиция — `site/src/landing/landing.tsx` |
| §1.3 `.release`/`.release-dot` | `global.css:250–277` | `hero/styles.css`, `@keyframes hero-release-breathe`; полупрозрачные обводки — `color-mix()` на `--accent`, не литералы |
| §1.3 hover/transition кнопок и ссылок | `global.css:220–236, 150, 445` | `hero/styles.css`, `site/src/landing/chrome.css` |
| §1.3 `prefers-reduced-motion` | `global.css:467–473` | `design/base.css` (глобально) + собственный блок в `dep-graph/styles.css` и `hero/styles.css` |
| §1.4 мета-теги | `src/layouts/BaseLayout.astro:41–82` | `site/src/landing/head.ts` → `DocumentHead` каждого маршрута; рендерит `<DocumentHeadTags />` в `site/src/root.tsx` |
| §1.4 `canonical`, `hreflang`×3 | :45–48 | тот же набор, абсолютные адреса из `site/src/config.ts` |
| §1.4 `og:*`, `twitter:*` | :54–64 | один к одному, включая `og:image` на `/og.png` |
| §1.4 JSON-LD | :15–36, :79 | `structuredData()` в `head.ts`; **английский `description` на обеих локалях сохранён как был** — факт источника, не решение порта |
| §1.4 `theme-color` | :51 | `site/src/landing/theme-color.ts`: значение читается из `design/palette.css` (`--ink`), а не пишется литералом (R-27); отличие только в регистре hex |
| §1.4 preload шрифтов | :70–75 | `fontPreloads()` в `head.ts`: 2 тега на `/`, 4 на `/ru/`; адреса `/fonts/*.woff2` переписываются сборкой на хешированные (см. «Решение 5») |
| §1.4 Umami | :82 | `analytics()` в `head.ts`, website-id из окружения сборки, тег отсутствует при пустом (D-24) |
| §1.4 favicon | :50 | `site/public/favicon.svg` — файл скопирован дословно |
| §1.5 `robots.txt` | `public/robots.txt`, вручную | генерирует `tools/root-files.mjs`; ASCII-only проверяется в самом генераторе |
| §1.5 `sitemap.xml`, `feed.xml` | скриптами `build-content.mjs`/`build-sitemap.mjs` | `tools/root-files.mjs`, из того, что реально лежит в `dist` |
| §1.5 `llms.txt` | вручную | `tools/root-files.mjs`; абзац дизамбигуации дословно, плюс одна строка на `/doc/llms.txt` |
| §1.5 `llms-full.txt` | постбилд-скрипт `build-llms-full.mjs` в `dist` | `tools/root-files.mjs`, по образцу того скрипта (тот же заголовок, тот же формат `## URL:` / `## Title:`), только страницы лендинга |
| §1.5 ключ-файл IndexNow | статический файл в `public/` | генерируется из окружения сборки; при пустом ключе файла нет (см. «Решение 2») |
| §1.5 `og.png` | **отсутствовал вообще** | рисует `tools/og-card.mjs` (см. «Решение 4») |
| §1.5 `public/fonts/*` | 8 статических файлов | `design/fonts/` — байты уже в пакете (A4.10); сборка кладёт копии в `dist/fonts/` по прежним адресам |
| §1.6 `/en/` → `/` | `nginx.conf:36–37`, `return 301` | маршрут `site/src/routes/en/index@landing.tsx`: meta-refresh, `canonical` на корень, `noindex` |
| §1.7 адреса sitemap/feed | 2 адреса, `priority` 1 и 0.9 | те же два с теми же приоритетами, плюс адреса документации с 0.8 |
| §2 эталон | `dist/` из `npm ci && npm run build` | пересобран в scratch-копии (R-28), 19 файлов; `astro 5.18.2`, `node v24.18.0`, `npm 11.16.0` |
| §3.1 Tailwind | объявлен и не используется | не заведён (F-75); ни одного класса Tailwind в пакете |
| §3.2 i18n-роутинг Astro | `astro.config.mjs:10–17` | файловая раскладка Qwik: `index@landing.tsx` в корне, `ru/`, `en/`, `404@landing.tsx`; `prefixDefaultLocale: false` воспроизведён тем, что английский лежит в корне |
| §3.3 `_astro/` в `nginx.conf` | годовой immutable-кеш на `/_astro/` | выход другой: `/assets/` и `/build/`; правило nginx придётся обновить — записано в «Что не сделано» как вход в атом развёртывания |

### Решение 1 — у лендинга своя шапка, и она запрошена по имени

`site/src/routes/layout.tsx` — рама документации (бренд, поиск, селектор
языка, футер), и она принадлежит соседнему пакету работ. Лендингу нужна
другая начинка тех же двух компонентов: вход в документацию, два зеркала
исходников и переключатель языка вместо поиска, плюс локализованный футер
(«в пакетах», «© 2026 Олег Чирухин»). Ставить в общую раму условие «если это
лендинг» — значит завести флаг, который придётся читать обеим половинам
сайта.

Qwik Router даёт ровно этот механизм: маршрут `index@landing.tsx` просит
layout по имени, `layout-landing!.tsx` этим именем отзывается, `!` делает его
**top**-layout и цепочка на нём обрывается. Так `/`, `/ru/`, `/en/` и
`/404.html` живут в одном дереве маршрутов с документацией и носят свою раму,
без флага внутри любой из двух.

`!` — не украшение: без него резолвер повёл себя недетерминированно (А-1).

Побочный эффект решения: **правка `site/src/routes/layout.tsx` не
потребовалась**, и я её не делал. Пакет разрешал одну согласованную правку
навигации в чужом файле в конце работы; она оказалась не нужна, а значит и
риск застейджить чужое незакоммиченное в этом файле — тоже.

### Решение 2 — website-id и ключ IndexNow приходят из окружения, и пустое значение это инструкция

`site/src/config.ts` — одно место, где сайт узнаёт, для какого домена его
собирают: `VITE_SITE_ORIGIN` (по умолчанию `https://vibevm.org`),
`VITE_UMAMI_WEBSITE_ID`, `VITE_INDEXNOW_KEY`, `VITE_SITE_LASTMOD`. Три
последних по умолчанию **пустые**, и это не дырка «заполнит тот, кто заметит»:

- нет website-id — тега аналитики нет вообще, а не тег с пустым атрибутом
  (такой загрузил бы скрипт и не сообщал никому);
- нет ключа — ключ-файла нет, а не файл с пустой строкой (такой провалил бы
  проверку самого IndexNow и выглядел бы как сломанная настройка).

Значения из `BaseLayout.astro` и из `public/` **я не читал в конфигурацию и
нигде не воспроизвожу** — пакет это прямо запрещает, владелец переносит их в
A5.1.

Модуль читают две стадии сборки. В бандле `import.meta.env` подставляет Vite;
в Node (`tools/root-files.mjs`) `import.meta.env` не существует и то же
окружение называется `process.env`. Одно определение имён, две стадии, и
страница с ключ-файлом не могут разойтись в том, для какого они домена.

**Проверено обеими ветками.** С пустым окружением: тега нет, ключ-файла нет.
С выдуманными тестовыми значениями (UUID из нулей и 32 hex-символа —
одноразовые, никуда не записанные): тег отрендерился полностью
(`defer src="/u/s.js" data-website-id=… data-host-url="https://vibevm.org"`),
ключ-файл появился, и его имя совпало с содержимым, как требует протокол.

### Решение 3 — язык документа решает серверный вход, из адреса

`<html lang>` — контейнерный атрибут: он пишется до того, как исполнится
первый компонент, поэтому его не может задать ни маршрут, ни layout. Раньше
`site/src/entry.ssr.tsx` ставил `lang="en"` всегда — для лендинга это ошибка,
невидимая глазу и видимая только краулеру.

Теперь язык берётся из адреса, который на этом сайте и есть место языка
(D-06): `/ru/` — русский, всё остальное — английский. Адрес документации
отвечает здесь «английский» и получает свой язык на самой статье, как и было
записано в этом файле — поведение страниц документации не изменилось.

### Решение 4 — `og.png` рисует сборка

A0.26 установил факт: четыре мета-тега на обеих страницах год ссылались на
`${SITE}/og.png`, а файла не было нигде в дереве. D-28 относит `og.png` к
машинным файлам корня, которые «генерирует та же сборка», — так и сделано:
`tools/og-card.mjs` рисует карточку 1200×630 и сам кодирует PNG (zlib из
Node, CRC32, один IDAT; сглаживание — рендер в двойном размере и усреднение).

Что на карточке: тот самый граф зависимостей из hero на брендовом фоне, под
тем же смещённым терракотовым свечением, что и у страницы. **Ни одного
слова** — и это решение, а не нехватка: слова карточки живут в `og:title` и
`og:description`, которые всякая площадка рисует рядом с картинкой шрифтом
читателя, а текст, запечённый в пиксели, нельзя ни выделить, ни перевести, ни
прочесть вслух.

Все цвета читаются из `design/palette.css` — ни одного литерала в генераторе
(R-27). Размер выхода — 25 КБ; в репозиторий не коммитится (R-10).

Оговорка для дизайн-ревью A4.14: это не «карточка, нарисованная дизайнером»,
а честная брендовая заглушка, которая наконец отвечает на четыре
существующих тега. Если владелец захочет карточку с текстом — это отдельное
решение после ревью (F-74).

### Решение 5 — шрифты по двум адресам, и preload указывает на тот, который реально скачивается

`/fonts/*.woff2` — адреса, которые домен отдаёт с самого начала: на них
ссылается годовое immutable-правило nginx и всё, что держит старую ссылку.
Сборка кладёт туда копии из `design/fonts/` (476 КБ, байты в репозитории не
дублируются).

Но таблица стилей просит **не** их: Vite эмитит шрифты как
`/assets/<hash>-<имя>.woff2`, и именно этот адрес стоит в `@font-face`.
`<link rel="preload">` на публичный путь рядом с `@font-face` на хешированный
— это не preload, это вторая загрузка того же начертания, и браузер не может
об этом узнать. Поэтому страница объявляет публичный путь (он и есть смысл),
а сборка после рендера переписывает `href` preload-ссылок на хешированный
файл. Ничего больше в HTML не трогается.

Проверено: на `/` — два preload, на `/ru/` — четыре, все указывают в
`/assets/`; `dist/fonts/` содержит все восемь файлов.

### Решение 6 — корневые файлы выводятся из того, что собралось

`tools/root-files.mjs` обходит `dist`, а не список. Sitemap строится из
найденных страниц (лендинг с приоритетами эталона, документация — 0.8), feed
— из заголовков и описаний тех же страниц, `llms-full.txt` — из их HTML по
образцу `build-llms-full.mjs` (комментарии, `script`/`style`/`svg` срезаются
первыми — туда же уходит сериализованное состояние Qwik). Файл, который ведут
руками, устаревает в тот день, когда переезжает первая страница.

`pubDate` в feed оставлен прежним (`Thu, 16 Jul 2026 12:00:00 GMT`): это дата
публикации двух записей, и перевыставлять её на каждой сборке значит просить
каждый ридер объявить их заново. `lastBuildDate` — время сборки, потому что
это оно и значит.

### Имена краулеров: источники и дата проверки (F-38)

Проверено **2026-09-12**, по документации провайдеров, ни одного имени по
памяти. Список в `tools/root-files.mjs`, там же дата; она печатается и в самом
`robots.txt` отдельной строкой комментария.

| Провайдер | Страница документации | Что она называет сегодня |
|---|---|---|
| OpenAI | `https://developers.openai.com/api/docs/bots` | `GPTBot`, `ChatGPT-User`, `OAI-SearchBot`, `OAI-AdsBot` |
| Anthropic | `https://support.claude.com/en/articles/8896518-does-anthropic-crawl-data-from-the-web-and-how-can-site-owners-block-the-crawler` | `ClaudeBot`, `Claude-User`, `Claude-SearchBot`; `anthropic-ai` и `Claude-Web` **не упоминаются** |
| Google | `https://developers.google.com/search/docs/crawling-indexing/google-common-crawlers` | `Google-Extended` (и весь набор Googlebot) |
| Perplexity | `https://docs.perplexity.ai/guides/bots` | `PerplexityBot`, `Perplexity-User` |
| Common Crawl | `https://commoncrawl.org/ccbot` | `CCBot` |
| Apple | `https://support.apple.com/en-us/119829` | `Applebot`, `Applebot-Extended` |
| Amazon | `https://developer.amazon.com/amazonbot` | `Amazonbot`, `Amzn-SearchBot`, `Amzn-User` |
| Meta | `https://developers.facebook.com/docs/sharing/webmasters/web-crawlers/` | `meta-externalagent`, `meta-externalfetcher` |
| Yandex | `https://yandex.com/support/webmaster/robot-workings/check-yandex-robots.html` | `YandexBot` (главный индексирующий) |

Что изменилось в списке относительно эталона и почему:

- **сняты** `Claude-Web` и `anthropic-ai` — сегодняшняя документация Anthropic
  их не называет; писать имя, которого нет в документации, — это и есть «по
  памяти». Функционально ничего не теряется: такой краулер попадает под
  `User-agent: *` с `Allow: /`;
- **добавлены** `Claude-User`, `Claude-SearchBot`, `Perplexity-User`,
  `Amzn-SearchBot`, `Amzn-User`, `Meta-ExternalFetcher` — документированные
  собратья уже перечисленных имён (провайдеры развели «обучение», «поиск» и
  «переход по запросу пользователя» в отдельные токены, и у каждого своя
  директива);
- **не добавлен** `OAI-AdsBot`: он документирован, но проверяет страницы,
  поданные как реклама, — на этом сайте рекламы нет. Это выбор политики, а не
  имени; если владелец решит иначе, это одна строка.
- Регистр `Meta-ExternalAgent` оставлен прежним: сопоставление `User-agent` по
  RFC 9309 регистронезависимо, а меньший диff полезнее.

Оговорка честности: страницу Apple прямой выборкой открыть не удалось (сервер
трижды закрыл соединение); имена подтверждены по её же содержимому,
процитированному в результатах поиска по `support.apple.com`. Остальные восемь
прочитаны напрямую.

## A4.17 — тест паритета

`tools/parity.mjs <astro-dist> [qwik-dist]`. Эталон собирается в scratch-копии
`vibevm-org` (`npm ci && npm run build && node scripts/build-llms-full.mjs`),
сам каталог не тронут (R-28). Без зависимостей: сравнения — работа со
строками над собранным HTML, а парсер в гейте — ещё один пин.

Что сравнивается:

- **множества адресов** обеих сборок;
- для `/`, `/ru/`, `/404.html`: `lang`, `<title>`, `meta description`,
  `theme-color`, все `og:*`, все `twitter:*`, `canonical`, весь набор
  `hreflang`, число preload-ссылок на шрифты, JSON-LD (нормализованный —
  ключи отсортированы на всех уровнях), тег Umami, видимый текст;
- `robots.txt` и ключ-файл — байт в байт, а при расхождении построчно;
- `llms.txt` — абзац дизамбигуации дословно и каждая ссылка эталона;
- `sitemap.xml` — множество адресов и `lastmod`.

**Видимый текст сравнивается как множество фрагментов, а не как одна строка**,
и это решение с причиной. Общий футер печатает копирайт после своих колонок, а
не внутри них; сравнение страниц как текста по порядку объявило бы отличием
каждый фрагмент после этого места, и глушить его пришлось бы правилом
достаточно широким, чтобы спрятать настоящую потерю. Множество отвечает на два
вопроса, которые и важны: не пропала ли авторская строка и не появилась ли
строка, которой не было.

**Про секреты в выводе (R-25).** Имя ключ-файла IndexNow — это и есть ключ,
поэтому скрипт печатает его как `/<indexnow-key>.txt`. Тег Umami сравнивается
по форме (`defer`, тот же `data-host-url`, непустой id), а значение id не
читается и не печатается ни разу.

### Осознанные отличия

Список живёт в самом скрипте (`DIFFERENCES`), каждое правило — форма плюс
причина; отличие, которое ни под одно правило не попало, красит прогон. На
сегодняшнем выходе — 14 правил, сработали все.

| Id | Где | Отличие и почему |
|---|---|---|
| D-01 | адреса | `/en/` — новый. Раньше `return 301` в nginx; статическая сборка не умеет отдавать код, поэтому это страница, которая уходит сама (`noindex`, `canonical` на корень) |
| D-02 | адреса | `/doc/…` — документация теперь на том же домене, в той же сборке (D-28) |
| D-03 | адреса | `/og.png` — файл, которого не было (A0.26); четыре мета-тега наконец разрешаются |
| D-04 | адреса | выход бандлера другой: `/_astro/index.<hash>.css` → `/assets/…` и `/build/…`; CSS инлайнится в страницу, шрифты хешируются, публичные `/fonts/*` остаются |
| D-05 | адреса | `/manifest.json` — web-app-манифест оболочки |
| D-06 | адреса | ключ-файла IndexNow нет, пока окружение сборки не назовёт ключ |
| D-07 | robots.txt | вторая строка `Sitemap:` — на `/doc/sitemap.xml` |
| D-08 | robots.txt | имена краулеров по документации провайдеров на день сборки (таблица выше) |
| D-09 | robots.txt | одна строка комментария с датой проверки имён |
| D-10 | текст | шапка получает пункт «Documentation» / «Документация» на `/doc/` |
| D-11 | мета | `theme-color` — тот же цвет в нижнем регистре: значение читается из `palette.css`, где prettier нормализует hex |
| D-12 | umami | тега нет, пока окружение сборки не назовёт website-id |
| D-13 | sitemap | `lastmod` — дата сборки, а не замороженная дата рукописного файла; адреса документации добавлены с приоритетом 0.8 |
| D-14 | llms.txt | одна ссылка добавлена — на `/doc/llms.txt` |

Итог прогона: **35 отличий, все осознанные; необъяснённых 0**.

### Что тест поймал

`/ru/` рендерился с `lang="en"`. Ни на одном скриншоте этого не видно, ни один
тип-чекер этого не поймает, а для Яндекса и для скринридера это ровно та
ошибка, ради которой русское дерево существует. Починено решением 3.

Второй раз тест сработал на регрессии, которую внесла не правка, а сборка:
после перехода с вложенного именованного layout на top-форму и обратно (А-1)
он показал 22 «text added» — всю документационную шапку на лендинге.

## Гейты, дословно

### Сборка

```
$ node tools/build.mjs static
- Generated: 10 pages

build (static): generated 10 page(s), expected 10
build (static): removed dist/q-manifest.json from the output
build (static): root files robots.txt, llms.txt, llms-full.txt, sitemap.xml, feed.xml, og.png; 8 font file(s) at /fonts/, 3 page(s) repointed at the bundled faces, 17 crawler name(s) from 9 provider page(s)
build (static): ok
STATIC_EXIT=0
```

10 = 6 адресов документации (два манифеста фикстур плюс каталог `/doc/`,
счёт соседнего воркера) + 4 лендинговых маршрута (`/`, `/ru/`, `/en/`,
`/404.html`).

### Тест паритета

```
landing parity — the Astro build against this one
  reference : <scratch>/vibevm-org-copy/dist
  this build: <scratch>/dist-snapshot

--- addresses
    D-06      1 address(es) only in the Astro build: /<indexnow-key>.txt
    D-04      1 address(es) only in the Astro build: /_astro/index.RlFBae0X.css
    D-04      142 address(es) only in this build: /assets/B9CIFXIH-JetBrainsMono-latin.woff2, /assets/Bx9Tn3WZ-Spectral-cyrillic-400.woff2, /assets/CB0VLJ91-Inter-cyrillic.woff2 and 139 more
    D-02      6 address(es) only in this build: /doc/, /doc/com.example.docs/fixture-manual/0.1.0/, /doc/com.example.docs/fixture-manual/0.1.0/guide/every-block/ and 3 more
    D-01      1 address(es) only in this build: /en/
    D-05      1 address(es) only in this build: /manifest.json
    D-03      1 address(es) only in this build: /og.png
    ok        17 address(es) served by both
--- page /
    ok        lang: en
    ok        title: VibeVM — a package manager for Spec-Driven Development
    ok        meta description
    D-11      meta theme-color: «#14120E» became «#14120e»
    ok        meta og:type
    ok        meta og:site_name
    ok        meta og:locale
    ok        meta og:url
    ok        meta og:title
    ok        meta og:description
    ok        meta og:image
    ok        meta twitter:card
    ok        meta twitter:title
    ok        meta twitter:description
    ok        meta twitter:image
    ok        canonical: https://vibevm.org/
    ok        hreflang: en -> https://vibevm.org/ | ru -> https://vibevm.org/ru/ | x-defaul…
    ok        font preloads: 2
    ok        structured data (normalised) identical
    D-12      the analytics tag is not on this page
    D-10      text added: «Documentation»
    ok        37 visible fragment(s) compared
--- page /ru/
    ok        lang: ru
    ok        title: VibeVM — пакетный менеджер для Spec-Driven Development
    ok        meta description
    D-11      meta theme-color: «#14120E» became «#14120e»
    ok        meta og:type
    ok        meta og:site_name
    ok        meta og:locale
    ok        meta og:url
    ok        meta og:title
    ok        meta og:description
    ok        meta og:image
    ok        meta twitter:card
    ok        meta twitter:title
    ok        meta twitter:description
    ok        meta twitter:image
    ok        canonical: https://vibevm.org/ru/
    ok        hreflang: en -> https://vibevm.org/ | ru -> https://vibevm.org/ru/ | x-defaul…
    ok        font preloads: 4
    ok        structured data (normalised) identical
    D-12      the analytics tag is not on this page
    D-10      text added: «Документация»
    ok        37 visible fragment(s) compared
--- page /404.html
    ok        lang: en
    ok        title: VibeVM — a package manager for Spec-Driven Development
    ok        meta description
    D-11      meta theme-color: «#14120E» became «#14120e»
    ok        meta og:type
    ok        meta og:site_name
    ok        meta og:locale
    ok        meta og:url
    ok        meta og:title
    ok        meta og:description
    ok        meta og:image
    ok        meta twitter:card
    ok        meta twitter:title
    ok        meta twitter:description
    ok        meta twitter:image
    ok        canonical: https://vibevm.org/
    ok        hreflang: en -> https://vibevm.org/ | ru -> https://vibevm.org/ru/ | x-defaul…
    ok        font preloads: 2
    ok        structured data (normalised) identical
    D-12      the analytics tag is not on this page
    D-10      text added: «Documentation»
    ok        14 visible fragment(s) compared
--- robots.txt
    D-08      dropped: User-agent: Claude-Web
    D-08      dropped: User-agent: anthropic-ai
    D-09      added: # Agent names verified against the providers' own documentation on 2026-09-12.
    D-08      added: User-agent: Claude-User
    D-08      added: User-agent: Claude-SearchBot
    D-08      added: User-agent: Perplexity-User
    D-08      added: User-agent: Amzn-SearchBot
    D-08      added: User-agent: Amzn-User
    D-08      added: User-agent: Meta-ExternalFetcher
    D-07      added: Sitemap: https://vibevm.org/doc/sitemap.xml
    note      not byte for byte: 719 bytes became 997
--- IndexNow key file
    D-06      the key file is not in this build (no key in the build environment)
--- sitemap.xml
    D-13      address added to the sitemap: https://vibevm.org/doc/
    D-13      address added to the sitemap: https://vibevm.org/doc/com.example.docs/fixture-manual/0.1.0/
    D-13      address added to the sitemap: https://vibevm.org/doc/com.example.docs/fixture-manual/0.1.0/guide/every-block/
    D-13      address added to the sitemap: https://vibevm.org/doc/com.example.docs/fixture-manual/0.1.0/reference/addresses/
    D-13      address added to the sitemap: https://vibevm.org/doc/ru/com.example.docs/fixture-manual/0.1.0/
    D-13      address added to the sitemap: https://vibevm.org/doc/ru/com.example.docs/fixture-manual/0.1.0/guide/every-block/
    ok        2 address(es) in the Astro sitemap, 8 in this one
    D-13      lastmod: 2026-07-17 became 2026-09-12
--- llms.txt
    ok        the disambiguation paragraph is there, word for word
    D-14      link added to llms.txt: https://vibevm.org/doc/llms.txt
    ok        6 reference link(s) checked

--- deliberate differences cited
  D-01 (1×) … D-14 (1×)   [полные формулировки — в таблице выше и в самом скрипте]

parity: green — 35 deliberate difference(s), 0 unexplained.
PARITY_EXIT=0
```

(Строки `reference`/`this build` отредактированы: путь scratch-каталога вне
репозитория в отчёт не переносится, R-25. Блок «deliberate differences cited»
печатается полностью — здесь он сжат, потому что те же четырнадцать причин
дословно приведены таблицей выше.)

Сборку и снимок для сравнения я делаю в одном шаге и сравниваю **копию**
`dist` в scratch: в том же каталоге пакета параллельно собирает соседний
воркер, и один прогон паритета уже успел прочитать наполовину переписанный
`dist` (см. А-2).

### Пол дисциплины

```
$ typescript-ai-native floor --path vibevm/vibepacks/org.vibevm.doc/web/v0.1.0
=== prettier --check (floor perimeter: design/src, site/src) ===   OK
=== tsc --noEmit ===                                               OK
=== tests (node --test) ===   ℹ pass 22   ℹ fail 0
=== eslint (floor perimeter: design/src, site/src) ===             OK
=== typescript-ai-native-conform check ===
typescript-ai-native-conform check: 0 finding(s) in scope <workspace> ({}), 0 frozen in baseline, 0 new
=== typescript-ai-native-specmap --check ===
`…/web/v0.1.0\specmap.json` is out of date relative to the tree.
  drift: edges added: 46
  drift: edges removed: 2
floor: `specmap` FAILED
FLOOR_EXIT=1
```

```
$ node .../design/audit/contrast.mjs
=== pairs: gated=40, reference=6; below threshold=0 ===
@media(prefers-color-scheme:dark) agrees with [data-theme="dark"]: OK (0 divergences)
tokens.css writes no colour of its own: OK (every value is a var() into palette.css)
AUDIT_EXIT=0
```

Пять шагов пола зелёные, седьмой (`test-gate`) не запускался — пол
останавливается на первом красном. Шестой, `specmap`, красный, и **красный не
мой**: `specmap.json` — общий генерируемый индекс пакета, и перегенерация
покрывает не только мои файлы. Проверено явно: после
`typescript-ai-native specmap --path <пакет>` индекс называет `landing/`,
`config.ts`, `hero/`, `dep-graph`, `capability-card` — мои — **и**
`code-block`, `lightbox`, `components/toc`, `reader/code`, `reader/overlay`,
`reader/toc` — шесть модулей соседнего воркера, которых в дереве коммитов ещё
нет. Индекс, называющий несуществующие в коммите файлы, ломает `--check` на
чистом чекауте сильнее, чем индекс, который просто отстал. Поэтому
`specmap.json` я вернул в закоммиченное состояние (`git checkout --`) и не
трогал. Ratchet при перегенерации, к слову, чистый: `0 orphan(s)`,
`0 suspects`, 73 tagged code items.

### Прочее

```
$ grep -rn "#[0-9a-fA-F]\{3,8\}" design/src site/src
GREP_EXIT=1 (1 = no match = clean)
```

```
$ grep -rhoE 'https?://[A-Za-z0-9.-]+' site/dist … | sort | uniq -c | sort -rn
     92 https://vibevm.org
     23 https://github.com
     21 https://gitverse.ru
      7 https://schema.org
      6 http://www.w3.org
      1 https://qwikdev-build-v2.qwik-8nx.pages.dev
      1 https://llmstxt.org
      1 http://www.sitemaps.org
      1 http://localhost
```

Разбор четырёх «чужих» хостов: `schema.org` — `@context` в JSON-LD (адрес
словаря, не ресурс); `w3.org` и `sitemaps.org` — пространства имён XML;
`llmstxt.org` — ссылка в тексте `llms-full.txt`, как в эталоне;
`qwikdev-…pages.dev` — **строка внутри шаблона сообщения об ошибке** в чанке
фреймворка (`Code(Q${e}) https://…/docs/errors/#q${e}`), никто её не
запрашивает; `http://localhost/` — запасной origin в роутере, когда URL не
передан. Ни одной загрузки с чужого CDN нет (R-09).

```
$ du -sh site/public design/fonts
5.0K    site/public
476K    design/fonts
```

```
$ du -sh campaigns/docs-2026-09/findings/P4-O5-shots
700K
```

Шаг панели (`tools/self-check.sh`, шаг 8b), прогнанный из корня хоста ровно
так, как его зовёт панель, — это и есть два блока выше: `floor --path` и
`contrast.mjs`. Всю панель не гонял.

## Скриншоты

`campaigns/docs-2026-09/findings/P4-O5-shots/` — четыре PNG, суммарно 700 КБ:

| Файл | Что |
|---|---|
| `landing-1440-light.png` | `/`, 1440 px, светлая тема |
| `landing-1440-dark.png` | `/`, 1440 px, тёмная тема |
| `landing-390-light.png` | `/`, 390 px, светлая тема |
| `landing-390-dark.png` | `/ru/`, 390 px, тёмная тема |

Сняты Playwright'ом (он уже стоит в пакете после соседнего атома) поверх
**собранного** `site/dist`, через тот же локальный статический сервер, что
используют e2e-тесты; `colorScheme` эмулируется, тема выбирается
`prefers-color-scheme`, как и задумано. Скрипт съёмки одноразовый и живёт в
scratch — в пакет он не входит.

Одно наблюдение для дизайн-ревью A4.14 видно прямо на светлом скриншоте: граф
зависимостей в светлой теме почти не читается — узлы залиты `--bg-raise`
(почти цвет фона), рёбра терракотовые при `opacity .32`. Лендинг родился
тёмным, и в светлой теме его подпись нужно перебрать. Перенос один к одному
это не чинит (F-74).

## Аномалии

**А-1. Qwik 2.0.0-beta.43: вложенный именованный layout резолвится
недетерминированно.** Симптом: страницы лендинга приходят с **двумя** шапками
— документационная (`layout.tsx`: поиск, селектор языка, английский
копирайт) снаружи, лендинговая внутри. Ни ошибки, ни предупреждения; счёт
страниц сходится; глазами на скриншоте видно сразу, а тестом паритета — как 22
«text added» на каждой из трёх страниц.

Что важно: **это не воспроизводится стабильно**. Сборка в 10:52 с
`layout-landing.tsx` (вложенная форма) дала одну шапку — правильную; сборка
через час с теми же файлами маршрутов дала две; после переименования в
`layout-landing!.tsx` (top) и обратно очередная сборка с вложенной формой
снова дала одну. То есть вложенная форма даёт то один результат, то другой на
одном и том же входе.

Разбор кода резолвера (`@qwik.dev/router/lib/vite/index.mjs`, `resolveRoute`)
объясняет, почему top-форма надёжна, и не объясняет недетерминированности:
цикл, найдя именованный layout, обрывается на `layout.layoutType === "top"` —
это единственный безусловный выход; без него выход зависит от сравнения
`currentDir === routesDir`, то есть от нормализации двух путей. Точную причину
разброса я не установил (кандидаты: инкрементальный пересчёт конфигурации
маршрутов между клиентской и адаптерной сборками; параллельная сборка
соседнего воркера в том же каталоге, см. А-2). Установленный факт — тот, что
выше.

Лечение и правило на будущее: **именованный layout, который должен заменять, а
не вкладываться, пишется top-формой** — `layout-<имя>!.tsx`. Так и сделано.
Апстримного тикета не заводил (сеть разрешена только для установки и
документации краулеров) — кандидат в `E-BUG`.

**А-2. Параллельные сборки двух воркеров в один `site/dist` тихо портят
измерение.** Первый «финальный» прогон паритета показал 22 необъяснённых
отличия вида «страница есть в эталоне и нет здесь» и `q-manifest.json` в
выходе — то есть читал `dist` в тот момент, когда клиентская сборка соседа его
уже переписала, а SSG ещё не отработал. Гейт при этом честно покраснел, но по
ложной причине. Обход: собирать и **снимать копию** `dist` в scratch одним
шагом, и сравнивать копию. Записываю как факт эксплуатации: в общем чекауте
`dist` — это разделяемый ресурс без блокировки.

**А-3. `DocumentScript` в Qwik не принимает `data-*`.** В JSX TypeScript не
проверяет атрибуты с дефисом, и `data-island` в компоненте работает; объектный
литерал в `head.scripts` такой поблажки не получает, и тег Umami не
типизируется. Обошёл композицией (`Object.assign` двух половин), а не
утверждением типа: `as` здесь был бы ровно тем непроверенным приведением,
которое запрещает R-16.

**А-4. `head.scripts` с полем `script` печатает содержимое дважды.** Qwik
рендерит JSON-LD и внутрь элемента, и как атрибут `script="…"` того же тега —
килобайт дублированных структурированных данных на каждой странице плюс
нестандартный атрибут. Лечится вторым вариантом того же типа:
`dangerouslySetInnerHTML`. Проверено: в выходе остаётся один экземпляр.

**А-5. `<link rel="preload">` на шрифт и `@font-face` на тот же шрифт легко
расходятся.** Это не баг фреймворка, а ловушка переноса: публичный путь
`/fonts/*` жив, а таблица стилей просит хешированный файл. Никакой гейт, кроме
взгляда в сеть браузера, этого бы не заметил. Решение 5.

## Состояние дерева после сдачи: сборка сейчас красная, и красная не здесь

Гейты выше сняты на дереве моих двух коммитов: `node tools/build.mjs static`
— `STATIC_EXIT=0`, паритет — зелёный. Повторный прогон уже **после** того, как
в ветку легли `c8a91c9d` и `7675efc9` соседнего воркера (плюс его
незакоммиченные правки в `site/src/routes/doc/`), падает:

```
Error during request handling /doc/ru/ TypeError: Cannot read properties of undefined (reading 'lang')
    at headerLanguageChoices (…/build/q-D2cQ1SY6.js)
!!! /doc/ru/: Error during SSG
SSG completed with 1 error(s)
```

`headerLanguageChoices` — функция `site/src/lib/view.ts`, зовёт её
`site/src/routes/layout.tsx` на новом маршруте `/doc/ru/`; и файл, и маршрут —
периметр P4-O2, в работе прямо сейчас. Ни один из моих файлов в стеке не
участвует. Записываю, чтобы красная сборка в момент приёмки не была прочитана
как дефект лендинга: лендинговые маршруты в этом же прогоне отрендерились, а
остановил сборку адаптер на одной странице документации.

## Общие файлы: что пришлось закоммитить не только своё

Соседний воркер (P4-O2) работает в том же пакете и коммитит часто. Два файла
на момент моего коммита несли правки обоих, и обе мои правки без его правок не
работают:

- **`design/src/index.ts`** — шов дизайн-системы. Мои четыре блока экспортов
  (`Hero`, `InstallBlock`, `DepGraph`, `CapabilityCard`/`CapabilityRow`) и его
  тринадцать (`LanguageSelector`, `VersionSwitch`, `Card`, `Shelf`,
  `PackageHeader`, `PageMeta`, `RulePanel`, `ForAgent`, `SettingsPanel`,
  `ReturnToPlace`). Его компоненты **уже закоммичены** отдельными коммитами, а
  строки экспорта — нет; то есть без них дерево на HEAD не собиралось бы вовсе.
- **`tools/build.mjs`** — драйвер сборки. Моё: импорт `writeRootFiles`,
  регулярное выражение `PAGE_MODULE` (лендинговые маршруты теперь называются
  `index@landing.tsx` и `404@landing.tsx`) и шаг 5 — вызов генератора корневых
  файлов. Его: `docAddressCount()` вместо `manifestPageCount()`, которого
  требуют **уже закоммиченные** им фикстуры (`manifest-ru.json`).

Вариант «закоммитить только свои строки» я проверил и отверг: и в том, и в
другом файле он даёт коммит, который не собирается. Вариант «не коммитить
вовсе» даёт атом без проводки. Поэтому оба файла закоммичены целиком, чужие
строки в них — законченная работа соседа, а не черновик, и его собственный
`git commit -- tools/build.mjs` после этого станет либо пустым, либо маленькой
разницей. Ничего чужого я не правил и не откатывал.

Все остальные мои коммиты — только мои файлы; `git add` вызывался
поимённо, вывод проверен перед коммитом.

## Что не сделано и почему

- **`site/src/routes/layout.tsx` не тронут.** Пакет разрешал одну правку
  навигации; после решения 1 она не понадобилась (у лендинга своя рама из тех
  же компонентов). Заодно снят риск застейджить чужое незакоммиченное.
- **`specmap.json` не перегенерирован** — см. «Гейты». Красный шаг пола
  чинится первой же перегенерацией после того, как соседний воркер
  закоммитит свои модули.
- **`nginx.conf` нового развёртывания не трогал** (R-24). Но записываю вход
  для атома развёртывания: правило годового immutable-кеша сейчас написано на
  `/_astro/`, а новая сборка кладёт ассеты в `/assets/` и `/build/`. Если
  правило не обновить, длинное кеширование молча перестанет применяться —
  регрессия в раздаче, не в содержании (A0.26 §3.3).
- **`og.png` — брендовая заглушка без текста**, а не карточка от дизайнера.
  См. решение 4; текст на карточке — вопрос дизайн-ревью.
- **Логотип-марка в шапке лендинга не перенесена.** В эталоне рядом со
  словом «VibeVM» стоит инлайновый SVG-граф; общий `DocsHeader` принимает
  словесный знак и слот, а слот — это правая часть бара. Добавить марку —
  значит добавить проп компоненту, который принадлежит соседнему пакету
  работ. Текстового паритета это не нарушает (SVG — не текст), favicon марку
  сохраняет; кандидат в правку после A4.11/A4.12.
- **`aria-label` ряда карточек — по-прежнему английский на обеих локалях**
  («What VibeVM is»), как и в эталоне: строка была захардкожена в разметке
  Astro, а не в таблице строк. Перенёс как есть (F-74); добавить её в
  `i18n.ts` — работа адаптации, не переноса.
- **`theme-color` один и тот же на обе темы**, как в эталоне (тёмный `--ink`).
  Для сайта с двумя темами честнее пара тегов с `media`, но это правка
  содержания, а не перенос; вопрос для A4.14.
- **Локальный ридер, оглавление, страницы документации** — периметр P4-O2, не
  трогал. **Крейт оболочки, сервер, развёртывание, `cargo`** — не трогал.
  **`vibevm-org`** — только чтение; эталон собран в scratch-копии (R-28).

## `git status --short` на момент сдачи (мои пути)

```
$ git status --porcelain -- vibevm/vibepacks/org.vibevm.doc/web/v0.1.0/site/src/landing \
    vibevm/vibepacks/org.vibevm.doc/web/v0.1.0/site/src/config.ts \
    vibevm/vibepacks/org.vibevm.doc/web/v0.1.0/design/src/components/hero \
    vibevm/vibepacks/org.vibevm.doc/web/v0.1.0/design/src/components/dep-graph \
    vibevm/vibepacks/org.vibevm.doc/web/v0.1.0/design/src/components/capability-card \
    vibevm/vibepacks/org.vibevm.doc/web/v0.1.0/tools/root-files.mjs \
    vibevm/vibepacks/org.vibevm.doc/web/v0.1.0/tools/og-card.mjs \
    vibevm/vibepacks/org.vibevm.doc/web/v0.1.0/tools/parity.mjs
(пусто)
```

Сам этот отчёт и каталог `P4-O5-shots/` оставлены **незакоммиченными** — по
заведённому в кампании порядку их вносит центральная сессия вместе с записью
в леджер.
