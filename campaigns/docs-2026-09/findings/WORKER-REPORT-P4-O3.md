# WORKER-REPORT-P4-O3 — статический адаптер, SEO документации, агентские файлы сайта

Пакет: `campaigns/docs-2026-09/findings/PACKET-P4-O3.md` (A4.4, A4.5)
плюс два хвостовых атома по указанию оркестратора (раздел «## Хвост»).
Ветка `research-preview-1-docs`, без push. Дата: 2026-09-12.

## Коротко для оркестратора

Два атома сделаны, два коммита в ветке. Страница документации получила
`<head>` того же контракта, что у лендинга: canonical на `latest`,
`hreflang` со взаимными парами, Open Graph и Twitter Card с превью
пакета, JSON-LD, тег Umami. Рядом со страницами легли агентские
поверхности — `.md`, `.xml`, четыре тира `llms`, `manifest.json`,
картинки карточки, — **скопированные** из выхода `vibe doc build`.
Написаны `/doc/sitemap.xml` (индекс по пакетам и языкам), `/doc/llms.txt`,
`/doc/manifest.json`, резолвер `/doc/resolve/` с таблицей, и
`dist/csp.txt`. Новый линтер `tools/lint-links.mjs` — шаг статической
сборки; красный останавливает сборку.

Пять результатов стоит прочитать отдельно.

1. **Адрес `latest` был ссылкой, за которой не было страницы.** Его
   называли переключатель версий, цитата без версии (D-06) и, по
   `##SITE-CANONICAL-LATEST`, каждый canonical — и ни одна из них не
   открывалась. Теперь генератор пишет оба написания версии; страниц
   стало 12 → **18**, встраиваемых 8 → **14**.
2. **CSP на хэшах не масштабируется, и это измерено.** Пакет ожидал два
   инлайн-скрипта; в собранном выходе их **47 уникальных на 119
   вхождений** при 19 страницах, потому что фреймворк пишет в документ
   свои `q:func`, preload-списки и загрузчики. Политика написана
   честно и проверена, но растёт линейно со страницами — разбор и что с
   этим делать в атоме развёртывания: «Аномалии», А-1.
3. **Аномалия P4-O5 А-1 живая и воспроизведена дважды.** В двух сборках
   подряд лендинг получил документационную шапку **поверх** своей —
   в том числе на дереве **до моих правок**, — а следующая сборка тех же
   файлов дала одну шапку. Это не мой дефект и не исправлено; метод
   проверки и вывод — «Аномалии», А-2.
4. **Проекции копируются из всех деревьев пакета, а не из первого.**
   `vibe doc build` пишет **одну** проекцию за прогон, то есть реальная
   выкладка даёт три каталога с одной координатой. Первая версия шага
   брала первый — и молча теряла две трети агентской поверхности;
   найдено прогоном по настоящему выходу руководства.
5. **Я тронул два чужих файла** — тест P4-O2 (`language.spec.ts`) и
   правила паритета P4-O5 (`parity.mjs`). Оба раза это следствие
   осознанного переноса sitemap'а, оба разобраны в «Чужие файлы».

## Хэши и subject'ы

| Атом | Коммит | Subject |
|---|---|---|
| A4.5 | `8482e33e` | `feat(web): publish the agent surfaces beside the pages` |
| A4.4 | `f8240086` | `feat(web): prerender the site with the SEO contract` |

**Порядок обратный пакету, и это вынужденно.** A4.4 вводит линтер ссылок
как шаг сборки, а страница ссылается на `.md`, `.xml` и `llms.txt`,
которых до A4.5 в выходе нет: коммит с линтером поверх пустых адресов
собирался бы красным. Поэтому агентские поверхности легли первыми, и оба
коммита собираются по отдельности — состояние первого проверено сборкой
(12 страниц из 12) и полом (7 шагов, 22 теста) до коммита.

Между моими коммитами в ветку лёг чужой (`907948a2`, P2-O9) — ожидаемо.

## A4.4 — что у страницы документации в `<head>`

| Что | Откуда значение | Форма |
|---|---|---|
| `<title>` | заголовок страницы из манифеста | как было |
| `meta description` | `summary` страницы; взят, не составлен | как было |
| `meta robots` | только у фоллбэка: `noindex` | как было (тест P4-O2) |
| `link canonical` | `latest` того же языка; у фоллбэка — источник того же написания версии | **относительный** адрес |
| `link alternate hreflang` | по одному на язык манифеста + `x-default` на язык источника, **в том же написании версии** | абсолютные |
| `link alternate` `.md` / `.xml` / `llms.txt` | адресная карта (`href.ts`) | относительные |
| `og:type/site_name/locale/url/title/description/image` | адрес, манифест, превью пакета | `og:url` и `og:image` абсолютные |
| `twitter:card/title/description/image` | то же; карточка `summary_large_image` | абсолютные |
| JSON-LD | `TechArticle` (`headline`, `abstract`, `author`, `inLanguage`, `keywords`, `datePublished`, `image`) + `BreadcrumbList` | один экземпляр (`dangerouslySetInnerHTML`) |
| тег Umami | `site/src/config.ts`, тем же кодом, что у лендинга | отсутствует при пустом website-id |

Пример — `/doc/com.example.docs/fixture-manual/latest/guide/every-block/`
(атрибуты Qwik `:="r5_*"` опущены):

```
<title>Every block once</title>
<meta name="description" content="This page uses every block …">
<meta property="og:type" content="article">
<meta property="og:locale" content="en">
<meta property="og:url" content="https://vibevm.org/doc/com.example.docs/fixture-manual/latest/guide/every-block/">
<meta property="og:image" content="https://vibevm.org/doc/com.example.docs/fixture-manual/0.1.0/media/ec9d1a1a18535a59.png">
<meta name="twitter:card" content="summary_large_image">
<link rel="canonical" href="/doc/com.example.docs/fixture-manual/latest/guide/every-block/">
<link rel="alternate" hreflang="en" href="https://vibevm.org/doc/com.example.docs/fixture-manual/latest/guide/every-block/">
<link rel="alternate" hreflang="ru" href="https://vibevm.org/doc/ru/com.example.docs/fixture-manual/latest/guide/every-block/">
<link rel="alternate" hreflang="x-default" href="https://vibevm.org/doc/com.example.docs/fixture-manual/latest/guide/every-block/">
<link rel="alternate" type="text/markdown" href="/doc/com.example.docs/fixture-manual/latest/guide/every-block.md">
<link rel="alternate" type="application/xml" href="/doc/com.example.docs/fixture-manual/latest/guide/every-block.xml">
<link rel="alternate" type="text/plain" href="/doc/com.example.docs/fixture-manual/latest/llms.txt">
<script type="application/ld+json">{"@context":"https://schema.org","@graph":[{"@type":"TechArticle", …
  "keywords":"user, dev, task","datePublished":"2026-09-12T09:00:00Z","image":"…/media/ec9d1a1a18535a59.png",
  "author":{"@type":"Organization","name":"com.example.docs"}, …},{"@type":"BreadcrumbList", …}]}</script>
```

И фоллбэк — `/doc/ru/com.example.docs/fixture-manual/0.1.0/reference/addresses/`:

```
<title>Addresses</title>
<meta name="description" content="The shape of a documentation address …">
<meta name="robots" content="noindex">
<link rel="canonical" href="/doc/com.example.docs/fixture-manual/0.1.0/reference/addresses/">
<link rel="alternate" hreflang="en" href="https://vibevm.org/doc/com.example.docs/fixture-manual/0.1.0/reference/addresses/">
<link rel="alternate" hreflang="ru" href="https://vibevm.org/doc/ru/com.example.docs/fixture-manual/0.1.0/reference/addresses/">
<link rel="alternate" hreflang="x-default" href="https://vibevm.org/doc/com.example.docs/fixture-manual/0.1.0/reference/addresses/">
<link rel="alternate" type="text/markdown" href="/doc/ru/com.example.docs/fixture-manual/0.1.0/reference/addresses.md">
```

### Решение 1 — `latest` материализуется, потому что на него уже ссылались

D-06 говорит, что цитата без версии резолвится в `latest`;
`##SITE-CANONICAL-LATEST` — что нумерованный адрес несёт canonical туда
же; переключатель версий P4-O2 предлагает `latest` как «второй адрес
того же содержимого». Ни одна из трёх ссылок не открывалась: генератор
писал только нумерованные адреса. Голден острова, к слову, тоже
ссылается на `/…/latest/…` — конвейер считает этот адрес существующим.

Поэтому `onStaticGenerate` теперь отдаёт оба написания, а гейт числа
страниц удвоил свою половину. Оба адреса показывают одно содержимое (это
и есть `##SITE-VERSION-SHOWS-CURRENT`), различаются только тем, какой из
них канонический: нумерованный указывает на `latest`, `latest` — на себя,
и в sitemap стоит только `latest`.

Цена — вдвое больше страниц в обоих выходах (18 статических, 14
встраиваемых). Для встраиваемой оболочки это вход P4-O4: шаблонов
маршрутов стало вдвое больше.

### Решение 2 — canonical относительный, `hreflang` и Open Graph абсолютные

Три тега, три разных потребителя, и правило у каждого своё.

`hreflang` по документации Google обязан быть полным адресом; Open Graph
читают скребки, которые взяли страницу с какого-то другого origin, и им
не от чего отсчитывать относительный путь. А `canonical` разрешён
относительным — и **должен** быть им здесь: те же маршруты собираются
дважды, второй раз для читателя, которого `vibe` отдаёт с машины
пользователя (`##SITE-CANONICAL-LATEST` прямо говорит «та же схема с
другим origin»). Абсолютный canonical, запечённый на сборке, заставил бы
каждую локальную страницу объявить себя копией страницы публичного сайта.

Побочно это сохранило зелёным тест P4-O2, который сверяет canonical
фоллбэка с относительным адресом источника.

### Решение 3 — фоллбэк меняет одну ось за раз

Фоллбэк несёт `noindex` и canonical на **источник того же написания
версии**: `/doc/ru/…/0.1.0/x/` → `/doc/…/0.1.0/x/`, а `/doc/ru/…/latest/x/`
→ `/doc/…/latest/x/`. Цепочка canonical возникает только у нумерованного
фоллбэка (→ нумерованный источник → `latest`), и она невидима: страница
`noindex`, краулер по ней не идёт.

### Решение 4 — превью приходит в страницу через окружение

X-042: манифест страниц не несёт адресов картинок карточки, а конвейер
пишет их под хэшем содержимого. Страница вычислить адрес не может.
Поэтому `tools/build.mjs` **до** рендера находит в дереве карточку
1200×630 (`##CARD-PREVIEW-COMPOSED` компонует её именно в этой
пропорции) и передаёт карту координат в обе сборки Vite переменной
`VITE_DOC_MEDIA` — тем же каналом, которым приходят origin и website-id.
Сборка без дерева называет собственный `og.png` сайта: четыре мета-тега
обещают картинку, и 404 — это обещание, нарушенное при каждом шаринге.

Значение `og:image` берёт **нумерованный** адрес картинки: `latest` —
адрес, а карточка принадлежит публикации.

### Решение 5 — sitemap документации отдельный, а корневой называет его

`/doc/sitemap.xml` — `<sitemapindex>` из трёх частей: каталоги языков и
по одному url-set на (пакет, язык). В частях стоит только `latest` и
никогда фоллбэк; `lastmod` — `published_at` издания, иначе `rendered_at`.
Часть названа по координате **источника** плюс язык
(`/doc/sitemap/com.example.docs/fixture-manual/ru.xml`), потому что
адреса внутри неё служатся под координатой источника — файл, названный
по координате адаптации, назвал бы путь, которого внутри нет.

Корневой `sitemap.xml` перестал перечислять страницы документации (они
дублировали бы `/doc/sitemap.xml` и требовали второго фильтра фоллбэков)
и вместо этого несёт **запись о самом файле**: `<url><loc>…/doc/sitemap.xml</loc>`.
Это осознанный компромисс, и он назван вслух: `<urlset>` по протоколу не
умеет ссылаться на другой sitemap, а пакет просит запись именно в
корневом индексе. Настоящее объявление для краулера — вторая строка
`Sitemap:` в `robots.txt`, которая стоит там с A4.16.

Фильтр `noindex` переехал из `tools/build.mjs` в `tools/root-files.mjs`,
как и просил пакет, и заодно стал общим: он проходит по **каждому**
`<urlset>` в выходе, а не по одному файлу, и идемпотентен.

### Решение 6 — линтер различает «ссылается» и «загружает»

`tools/lint-links.mjs` проверяет четыре вещи: каждая внутренняя ссылка
ведёт на существующий файл, каждая пара `hreflang` взаимна, каждый
canonical существует, и ни один **загружаемый** адрес не ведёт на чужой
хост (R-09).

Различие «загружает/называет» — это и есть R-09, и оно найдено на
настоящих данных: `llms`-файлы руководства цитируют `datatracker.ietf.org`
(ссылка на RFC в прозе). Гейт, который отказал бы прозе документации в
праве сослаться на стандарт, — не то, что просит R-09. Поэтому чужой
хост в `src`, в `<link rel=preload|stylesheet|icon|manifest>`, в
`og:image`, в canonical или в `<loc>` sitemap'а — красный, а в `<a href>`
или в ссылке внутри скопированного `llms.txt` — считается и печатается.

Взаимность `hreflang` проверяется **только между страницами, которые
сами себе канонические**. Страница, назвавшая каноническим другой адрес
(нумерованная рядом с `latest`, `/404.html` рядом с корнем), объявила,
что её аннотации принадлежат той странице; требовать от канонической
называть в ответ каждый дубликат — противоположность тому, что говорит
canonical.

Два промаха не красят гейт и оба названы: ссылка `/doc/` в координату,
которой сайт не несёт, — это цитата, уходящая из библиотеки (адресная
карта детерминированная, страница вправе цитировать документацию, которой
здесь нет), и таблица `EXCEPTIONS` с одной записью — `L-01`, разобрана
ниже.

## A4.5 — что копируется из выхода `vibe doc build`

| Файл в дереве | Куда в `dist` |
|---|---|
| `<группа>/<имя>/<версия>/<документ>.md` | `/doc/[<язык>/]<координата источника>/<версия\|latest>/<документ>.md` |
| `<документ>.xml` | то же с `.xml` |
| `llms.txt`, `llms-small.txt`, `llms-medium.txt`, `llms-full.txt` (корень дерева) | `/doc/…/<версия\|latest>/<имя файла>` |
| `media/**` (иконка, баннер, превью — хэшированные имена) | `/doc/…/<версия\|latest>/media/**` |
| `index.html` (остров) | **не копируется**: это вход оболочки, а не страница |
| `manifest.json` | не копируется; сайт пишет его из манифеста библиотеки, сериализуя то же, что прочитал |

Правила размещения:

- дерево языка кладётся под координату **источника** с языковым сегментом
  впереди (D-06), а не под собственную координату адаптации;
- страницу, которой у адаптации нет, берут поверхности **источника** —
  ровно как её текст (`##READER-LANGUAGE-SWITCH-KEEPS-PLACE`): иначе агент,
  которому страницу показали, получил бы 404 на её Markdown;
- пишутся **оба** написания версии, потому что страница по адресу
  `latest` называет свой `.md` рядом с собой;
- дерево пакета, которого нет в библиотеке страниц, копируется под своей
  координатой и об этом печатается строка. Ничего на него не ссылается —
  страниц там нет, — но это то, что выкладка попросила опубликовать, и
  молча выбросить отрендеренный пакет было бы худшим из трёх ответов.

Источник деревьев — `VIBE_DOC_OUT`, список каталогов через разделитель
платформы. **Все** деревья одной координаты читаются по очереди: `vibe
doc build` пишет одну проекцию за прогон, поэтому выкладка даёт три
каталога с одной координатой (см. результат 4 в шапке). По умолчанию
берётся фикстурное дерево `site/src/fixtures/doc-build` — шесть файлов,
из них две проекции побайтово скопированы с голденов конвейера, — и
благодаря ему обычная `pnpm build:static` даёт сайт, у которого
резолвится каждая ссылка.

### Форма `/doc/resolve.json`

Ключ — адрес `spec://` без фрагмента, в обоих написаниях версии; язык в
ключе не участвует вовсе (цитата называет документацию, а не чтение её).

```json
{
  "schema_version": 1,
  "documents": {
    "spec://com.example.docs/fixture-manual@0.1.0/guide/every-block": {
      "href": "/doc/com.example.docs/fixture-manual/0.1.0/guide/every-block/",
      "anchors": ["root", "LEAD", "prose", "ANCHORED-ITEM", "verifiable", "varying", "for-codex", "footnotes"]
    },
    "spec://com.example.docs/fixture-manual/guide/every-block": {
      "href": "/doc/com.example.docs/fixture-manual/latest/guide/every-block/",
      "anchors": ["root", "LEAD", "…"]
    }
  }
}
```

`dist/doc/resolve/index.html` — рукописная страница без фреймворка:
инлайн-скрипт в `<head>` читает `?uri=`, проверяет его форму, берёт
адрес **из таблицы** (никогда не строит из параметра) и переносит
фрагмент только если страница объявила такой якорь. Не нашёл — говорит
об этом словами, а не пустой страницей. `noindex`: это не страница, это
развилка.

### `/doc/llms.txt` и `/doc/manifest.json` — свои, а не копии

Это каталоги **сайта**, а не пакета: «заголовок, звёздочка, издатель,
язык, аудитории, аннотация, ссылка» в форме arXiv (`##SEO-LLMS-FILES`),
по записи на издание, плюс раздел машинных адресов. У адаптации в строке
страниц стоит «N adapted of M; the rest are served in the source
language» — адаптация в работе не уменьшенное руководство, и агенту
полезно знать, что он выбирает. `/doc/manifest.json` — то же для машины:
каждое издание, каждая страница, её адрес в обоих написаниях, `.md`,
`.xml`, признак фоллбэка, якоря, аудитории, жанр. Ни одной даты сверх
тех, что несёт карточка (D-27).

`/doc/llms-full.txt` — конкатенация `llms-full.txt` изданий с заголовком;
если ни одно дерево его не несёт, файл не пишется.

## Гейты, дословно

### Сборка над выходом `vibe doc build` руководства

Собрано хостовым `target/debug/vibe.exe` в scratch тремя прогонами
(`--format html|md|xml`) для руководства и для фикстурного doc-пакета
конвейера; пути scratch в отчёт не переносятся (R-25).

```
$ VIBE_DOC_OUT="<fixture×3>;site/src/fixtures/doc-build;<manual×3>" node tools/build.mjs static
- Generated: 18 pages

build (static): generated 18 page(s), expected 18
build (static): removed dist/q-manifest.json from the output
build (static): 164 file(s) copied from 7 documentation tree(s) for 2 edition(s) (2 page(s) in a language that does not carry them); 13 written — catalogue, manifests, 3 sitemap part(s) over 7 address(es), resolver
build (static): 1 tree(s) the page library does not carry — their surfaces are published at their own coordinate and nothing on the site links them: org.vibevm.core/vibevm-docs@0.1.0
build (static): root files robots.txt, llms.txt, llms-full.txt, sitemap.xml, feed.xml, og.png; 8 font file(s) at /fonts/, 3 page(s) repointed at the bundled faces, 17 crawler name(s) from 9 provider page(s)
build (static): csp.txt — 47 inline script hash(es) over 119 occurrence(s), no external source
documentation links — every address against the files behind it
    ok        19 page(s), 347 file(s) in the output
    ok        740 link(s) followed, 18 hreflang pair(s) checked
    ok        9 page(s) name another page canonical and are read as it
    ok        12x github.com — the canonical source repository, linked by the landing
    ok        11x gitverse.ru — the source mirror, linked by the landing
    ok        2 outbound link(s) to 1 host(s) the prose cites, fetched by nothing: datatracker.ietf.org (2)
    note      1 citation(s) into documentation this site does not carry:
              /doc/com.example/subject/latest/common/PROP-001/
    L-01      8x — The island golden cites `media/diagram.svg` relative to the page, and the fixture package carries no such file — the pipeline writes a package's media at the root of its tree, not beside a page, and SVG is not an allowed medium in this wave (D-20-6). Filed as an island-golden finding; the picture is the only broken one on the fixture page.
links: green — 740 checked, 0 broken.
build (static): ok
STATIC_EXIT=0
```

**18 = 12 адресов документации (по 6 на каждое из двух написаний версии:
страница пакета + две страницы) + 1 дверь `/doc/` + 1 каталог `ru` + 4
маршрута лендинга.** Девятнадцатая страница в выходе — резолвер, который
не маршрут приложения.

Та же сборка без `VIBE_DOC_OUT` (фикстурное дерево по умолчанию):
`generated 18, expected 18`, `24 file(s) copied from 1 tree`,
`links: green — 738 checked, 0 broken`.

### Встраиваемая сборка (гейт P4-O4 не сломан)

```
$ node tools/build.mjs embedded
build (embedded): generated 14 page(s), expected 14
build (embedded): removed dist-embedded/q-manifest.json from the output
build (embedded): ok
```

### Пол дисциплины

```
$ node tools/floor.mjs
=== prettier --check (floor perimeter: design/src, site/src) ===   OK
=== tsc --noEmit ===                                               OK
=== tests (node --test) ===   ℹ pass 26   ℹ fail 0
=== eslint (floor perimeter: design/src, site/src) ===             OK
=== typescript-ai-native-conform check ===
typescript-ai-native-conform check: 0 finding(s) in scope <workspace> ({}), 0 frozen in baseline, 0 new
=== typescript-ai-native-specmap --check ===
typescript-ai-native-specmap --check: clean (0 spec units, 88 tagged code items, 88 edges, 0 suspects, 88 warnings).
typescript-ai-native-specmap: ratchet gate — 0 orphan(s) (0 root(s) exempt).
=== test-gate (xfail-strict) ===
test-gate: 26 results parsed (0 failed, 0 skipped), baseline entries: 0
test-gate: green (xfail-strict).
floor: all green (7 step(s) run, 0 disabled by policy).
FLOOR_EXIT=0
```

22 теста стало 26: четыре новых в `site/src/seo/csp.test.ts`.
`specmap.json` перегенерирован дважды — по разу на коммит, каждый раз
только по своим файлам (81 → 88 юнитов), красный шаг из отчёта P4-O5
закрыт тем, что соседние воркеры сдали.

### Тест паритета лендинга

```
$ node tools/parity.mjs <reference-dist>
parity: green — 31 deliberate difference(s), 0 unexplained.
PARITY_EXIT=0
```

Эталон — **не пересобирался**: `npm ci` требует реестра, а `vibevm-org`
эта кампания только читает (R-28). Взят закоммиченный в том дереве
`dist/`, скопированный в scratch, и над копией выполнен его же
постбилд-скрипт `build-llms-full.mjs` (без него `/llms-full.txt`
отсутствует в эталоне и даёт ложное отличие). Тридцать первое осознанное
отличие — новое правило **D-15** для `/csp.txt`; см. «Чужие файлы».

### Playwright

```
$ pnpm test:e2e
Running 29 tests using 1 worker
…
  29 passed (22.9s)
```

Тест «the fallback is not offered to a crawler in the sitemap» переписан
под новый адрес sitemap'а — см. «Чужие файлы».

### Прочее

```
$ grep -rn "#[0-9a-fA-F]\{3,8\}" design/src site/src
GREP_EXIT=1 (1 = no match = clean)
```

```
$ du -sh site/dist ; find site/dist -type f | wc -l
6.8M    site/dist        347      (с поверхностями руководства)
2.8M    site/dist        223      (фикстурное дерево по умолчанию)
```

Шаг панели 8b, прогнанный из корня хоста ровно так, как его зовёт
`tools/self-check.sh` (всю панель не гонял):

```
$ <slot>/typescript-ai-native.exe floor --path vibevm/vibepacks/org.vibevm.doc/web/v0.1.0
floor: all green (7 step(s) run, 0 disabled by policy).
FLOOR_EXIT=0
$ node vibevm/vibepacks/org.vibevm.doc/web/v0.1.0/design/audit/contrast.mjs
=== pairs: gated=40, reference=6; below threshold=0 ===
@media(prefers-color-scheme:dark) agrees with [data-theme="dark"]: OK (0 divergences)
tokens.css writes no colour of its own: OK (every value is a var() into palette.css)
AUDIT_EXIT=0
$ grep -c '^run_step ' tools/self-check.sh
50
```

## Аномалии

**А-1. Политика на хэшах не масштабируется, и пакет исходил из числа,
которого нет.** Пакет ожидал два инлайн-скрипта (тема и отключение
восстановления прокрутки). Измерено на собранном выходе: **47 уникальных
хэшей на 119 вхождений при 19 страницах**. Разбор по видам:

| Вид инлайн-скрипта | Уникальных | Чей |
|---|---|---|
| `theme-init.js` | 1 | наш |
| отключение восстановления прокрутки | 1 | наш |
| `(window._qwikEv…)` | 2 | фреймворк |
| загрузчик графа бандла (`type="module"`) | 1 | фреймворк |
| `q:func="qwik/json"` — **своё на страницу** | 12 | фреймворк |
| preload-списки (`q:type="preload"`) — почти своё на страницу | 5 | фреймворк |
| пустые (`<script :="useOn" hidden>`) | 1 | фреймворк |
| прочие уникальные | 24 | фреймворк |

То есть политика растёт примерно на 2–3 хэша со **страницы**: на
руководстве в 48 страниц это ~150 хэшей и заголовок в килобайтах.
Блоков данных (`type="qwik/state"`, `qwik/vnode`, `application/ld+json`)
в политике нет намеренно: браузер их не исполняет и CSP их не блокирует,
хэш такого блока — хэш, который никто не ищет.

Ходовой ответ для Qwik — nonce, а статический хостинг nonce выдать не
может (значение обязано быть своим на ответ). Вход в атом развёртывания:
либо сервер подставляет nonce и отдаёт страницу не как файл, либо
`script-src` остаётся на хэшах и заголовок пишется большим, либо
`'unsafe-inline'` с записанной ценой. Решение владельца, не моё;
`dist/csp.txt` пишется честно и проверяется в обе стороны.

**А-2. Аномалия P4-O5 А-1 жива: лендинг получает документационную шапку
поверх своей, недетерминированно.** Первый прогон паритета после моих
правок дал 33 необъяснённых отличия, все — текст документационной шапки
(поиск, селектор языка, «English», «Русский», «★») на `/`, `/ru/` и
`/404.html`. В `dist/index.html` было **два** `<header class="docs-header">`.

Метод атрибуции, чтобы не приписать это себе: три моих файла из
`site/src` возвращены к HEAD (`git checkout --`), сборка повторена —
**«Search the documentation» на лендинге осталась**. То есть дефект уже
был в ветке до меня. Файлы восстановлены из scratch-копии, и следующая же
сборка того же дерева дала **одну** шапку; все последующие — тоже.

Вывод: `layout-landing!.tsx` в top-форме, как и починил P4-O5, но
резолвер Qwik 2.0.0-beta.43 всё ещё решает цепочку layout'ов
недетерминированно (кандидат: инкрементальный кэш конфигурации
маршрутов между клиентской и адаптерной сборками). Не чинил: периметр
P4-O5/P4-O2, а известное лечение уже применено. **Практическое
следствие для приёмки: паритет надо гонять на той же сборке, что
уходит в выкладку, а не на предыдущей.** Кандидат в `E-BUG` и в
пересмотр пина Qwik вместе с X-034.

**А-3. `vibe doc build` пишет одну проекцию за прогон, и шаг копирования
это чуть не проглядел.** Первая версия брала первое дерево координаты;
на фикстуре это работало (одно дерево), на настоящем выходе руководства
(три каталога — html, md, xml) она опубликовала бы только одну проекцию
из двух и промолчала. Лечение: все деревья координаты читаются по
очереди, файл ищется в каждом. Найдено только прогоном по настоящему
выходу — фикстура такого не ловит.

**А-4. `MSYS_NO_PATHCONV` нужен для `VIBE_DOC_OUT`.** Список путей через
`;` в bash на этой машине приезжает в Node разрезанным по `:` — MSYS
переписывает переменные, похожие на списки путей. Ровно то, о чём
предупреждает `##STACK-BUILD-HYGIENE`. Рабочая форма:
`MSYS_NO_PATHCONV=1 MSYS2_ARG_CONV_EXCL='*' VIBE_DOC_OUT="…" node tools/build.mjs static`.
Записано в отчёт, а не в скрипт: переменная окружения — дело того, кто
запускает.

**А-5. Инлайн-скрипт в `head.scripts` дублируется в атрибут — второй
случай.** P4-O2 нашёл это на JSON-LD и вылечил `dangerouslySetInnerHTML`;
у скрипта отключения прокрутки поле осталось `script`, и в выходе те же
236 байт стояли ещё и атрибутом `script="…"` на том же теге. Для CSP это
хуже, чем килобайт лишнего: исходник скрипта в атрибуте, который политика
никак не учитывает. Переведён на `dangerouslySetInnerHTML` (тег `<head>`
и мета — мой периметр); тест 21 Playwright («перезагрузка не скроллит»)
остался зелёным.

**А-6. `media/diagram.svg` острова не резолвится ни в одной раскладке.**
Голден ссылается на картинку **относительно страницы**
(`<page>/media/diagram.svg`), а конвейер кладёт медиа в **корень дерева**
(`media/<хэш>.<ext>`) — то есть адрес не сошёлся бы и у настоящего пакета
с картинкой, не только у фикстуры без неё. Расширение А-6 отчёта P4-O2:
там это «голден ссылается на файл, которого нет», здесь — «и не может
быть по такому адресу». В линтере это единственная запись `EXCEPTIONS`
(`L-01`, 8 попаданий = 4 адреса острова × 2 написания версии).

## Чужие файлы: что пришлось тронуть и почему

**`site/tests/language.spec.ts` (P4-O2), один тест.** «the fallback is
not offered to a crawler in the sitemap» читал корневой `/sitemap.xml` и
ждал там нумерованный адрес адаптации. После решения 5 адреса
документации живут в `/doc/sitemap.xml` и только в написании `latest`.
Тест переписан на индекс и его части, **утверждение усилено**: фоллбэка
нет, адаптированная страница есть, и ни в одной части не встречается
`/0.1.0/`. Имя теста и то, что он меряет, не изменились.

**`tools/parity.mjs` (P4-O5), одно правило.** `/csp.txt` — новый файл в
корне выхода, и без правила паритет краснел бы навсегда по причине,
которую никто бы не выводил заново. Добавлено **D-15** в той же форме, с
причиной. Ничего существующего не тронуто.

**`site/src/landing/head.ts` (P4-O5), функция `analytics()`.** Тег Umami
на страницах документации обязан быть «тем же способом, что у лендинга»
(D-24). Функция вынесена в `site/src/seo/analytics.ts` и импортируется
обеими половинами; в файле лендинга осталась строка импорта вместо тела.
Вывод байт в байт тот же — это и подтверждает зелёный паритет.

**`tools/root-files.mjs` (P4-O5)** правился в пределах, которые назвал
пакет: запись о `/doc/sitemap.xml` вместо перечисления страниц
документации, `Sitemap:` через общую константу, и переезд фильтра
`noindex` из `build.mjs`. Опечатку в строке отчёта CLI
(`result.preloaded`, которого нет: при прямом запуске
`node tools/root-files.mjs` печаталось `undefined`) в этих двух коммитах
**не трогал** — не мой периметр; исправлена отдельным хвостовым атомом
по указанию оркестратора, см. «## Хвост».

Чужого незакоммиченного не стейджил: `git status` перед каждым коммитом
проверялся, в обоих коммитах только пути
`vibevm/vibepacks/org.vibevm.doc/web/v0.1.0/**`. Крейты, `xtask`,
`vibevm-org`, `nginx.conf`, PROP-файлы и вижен не трогал; `cargo` не
запускал.

## Что не сделано и почему

- **IndexNow после выкладки** (`##SEO-INDEXNOW`, «send the list of
  changed URLs after every deploy») — не делал: это действие
  развёртывания, а не сборки, и ключ приходит из окружения выкладки.
  Сборка по-прежнему пишет ключ-файл, когда ключ ей назвали.
- **`FAQPage` в JSON-LD** («where apt») — не делал: у страниц фикстуры
  нет разметки вопросов, а угадывать по заголовкам значит писать
  структурированные данные, которых в тексте нет.
- **`theme-color` на страницах документации** не ставил. Значение
  лендинга — тёмная краска бренда на обе темы (P4-O5 записал это как
  вопрос к A4.14); одно значение на странице, которая слушается темы
  читателя, врало бы на половине показов. Пара тегов с `media` — правка
  содержания, вопрос ревью.
- ~~**SEO-теги во встраиваемой сборке не подавляются.**~~ — **закрыто
  хвостовым атомом**, см. «## Хвост».
- **Ни один `latest`-адрес не получает внутренних ссылок, кроме
  переключателя версий.** Навигация, полки и `llms.txt` конвейера ведут
  на нумерованные адреса, то есть канонический адрес страницы сегодня
  почти не слинкован изнутри. Это не ломает индексирование (canonical
  именно для этого), но кандидат в правку: либо навигация переходит на
  `latest`, либо решение записывается как осознанное.
- **`llms-small.txt`/`llms-medium.txt` фикстурного дерева** не написаны:
  их никто не линкует, настоящие деревья их несут, а фикстура бюджета
  токенов на двух коротких страницах была бы файлом, в котором нечему
  ошибаться.
- **Всю панель `tools/self-check.sh`** не гонял — только шаг 8b, как и
  просил пакет. **`cargo` не запускал**, выход `vibe doc build` собирал
  готовым `target/debug/vibe.exe`.

## Хвост

Два атома по указанию оркестратора, два коммита.

| Что | Коммит | Subject |
|---|---|---|
| публичная голова не попадает в локального читателя | `39c9c850` | `feat(web): keep the public head out of the local reader` |
| опечатка в строке отчёта `root-files` | `a3580a72` | `fix(web): print the real font count when run by hand` |

### Где живёт признак сборки

`site/src/seo/mode.ts`, одна строка:

```ts
export const IS_LOCAL_READER: boolean = ISLAND_HTML === ISLAND_PLACEHOLDER;
```

Это **тот самый `define`, которым уже различаются сборки**, а не новый:
`__VIBE_ISLAND_HTML__` подставляет каждая конфигурация Vite —
статическая кладёт отрендеренный остров, встраиваемая кладёт
`<!--vibe-doc-island-->`, который `vibe doc serve` заменяет на остров
запрошенной страницы (`lib/island-source.ts`, `lib/island-placeholder.ts`).
Эта подстановка **и есть** определение сборки локального читателя, так
что и узнавать её честнее по ней: второго флага, который пришлось бы
держать в согласии с первым, нет, переменной окружения, которую выкладка
могла бы выставить по ошибке, — тоже. Обе стороны сравнения к моменту
бандлера литералы, поэтому оно сворачивается в константу, и невыбранная
ветка выбрасывается вместе с ней.

Флаг **передаётся аргументом**, а не читается модулем головы:
`documentationHead(view, local)`, `catalogueHead(lang, local)`. Причина
прикладная — `seo/head.ts` должен оставаться импортируемым тестом, у
которого нет за спиной сборки, а `mode.ts` тянет за собой
`__VIBE_ISLAND_HTML__`, которого вне Vite не существует. Значение
подставляют два маршрута: `routes/doc/index.tsx` и
`routes/doc/[...path]/index.tsx`.

### Что несёт и чего не несёт страница во встраиваемой сборке

Проверено на `dist-embedded/com.example.docs/fixture-manual/latest/guide/every-block/index.html`:

```
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Every block once</title>
<meta name="description" content="This page uses every block of the documentation genre **once**, …">
<link rel="alternate" type="text/markdown"    href="/doc/com.example.docs/fixture-manual/latest/guide/every-block.md">
<link rel="alternate" type="application/xml"  href="/doc/com.example.docs/fixture-manual/latest/guide/every-block.xml">
<link rel="alternate" type="text/plain"       href="/doc/com.example.docs/fixture-manual/latest/llms.txt">
--- inline scripts: 3   (тема, отключение восстановления прокрутки, один загрузчик фреймворка)
```

Нет ни одного: `canonical`, `hreflang`, `og:*`, `twitter:*`,
`application/ld+json`, `meta robots`, тега аналитики. Последний —
**структурно**: `analytics()` во встраиваемой ветке не вызывается вовсе,
потому что короткую голову пишет `seo/local.ts`, а не фильтр поверх
длинной. Фильтр пришлось бы сначала построить публичную голову — позвать
тег аналитики, чтобы его выбросить, — и утверждение «локальный режим
этого не публикует» стало бы утверждением «публикует и стирает».

**Выбор теста назван: оба.** Node-тест `site/src/seo/local.test.ts` (три
теста, пол вырос 26 → 29) утверждает структурно, что короткая голова
несёт и чего не несёт, и что каждый из семи шаблонов гейта находит тот
тег, который называет (шаблон, тихо переставший совпадать, хуже
отсутствующего гейта). И шаг сборки `checkLocalHead` в `tools/build.mjs`
читает **байты** `dist-embedded` и краснеет на любом из семи тегов:
обещание — про то, что записано на диск, и спросить об этом можно только
выход. Список шаблонов живёт рядом с построителями в `seo/local.ts` —
одно утверждение, прочитанное с двух концов.

Число встраиваемых страниц не изменилось: **14 из 14**.

### Гейты хвоста, дословно

```
$ node tools/floor.mjs
=== prettier --check (floor perimeter: design/src, site/src) ===   OK
=== tsc --noEmit ===                                               OK
=== tests (node --test) ===   ℹ pass 29   ℹ fail 0
=== eslint (floor perimeter: design/src, site/src) ===             OK
=== typescript-ai-native-conform check ===
typescript-ai-native-conform check: 0 finding(s) in scope <workspace> ({}), 0 frozen in baseline, 0 new
=== typescript-ai-native-specmap --check ===
typescript-ai-native-specmap --check: clean (0 spec units, 91 tagged code items, 91 edges, 0 suspects, 91 warnings).
typescript-ai-native-specmap: ratchet gate — 0 orphan(s) (0 root(s) exempt).
=== test-gate (xfail-strict) ===
test-gate: 29 results parsed (0 failed, 0 skipped), baseline entries: 0
test-gate: green (xfail-strict).
floor: all green (7 step(s) run, 0 disabled by policy).
```

```
$ node tools/build.mjs embedded
build (embedded): generated 14 page(s), expected 14
build (embedded): removed dist-embedded/q-manifest.json from the output
build (embedded): 14 page(s) carry none of the 7 public-only tags
build (embedded): ok
```

```
$ node tools/build.mjs static
build (static): generated 18 page(s), expected 18
build (static): removed dist/q-manifest.json from the output
build (static): 24 file(s) copied from 1 documentation tree(s) for 2 edition(s) (2 page(s) in a language that does not carry them); 13 written — catalogue, manifests, 3 sitemap part(s) over 7 address(es), resolver
build (static): root files robots.txt, llms.txt, llms-full.txt, sitemap.xml, feed.xml, og.png; 8 font file(s) at /fonts/, 3 page(s) repointed at the bundled faces, 17 crawler name(s) from 9 provider page(s)
build (static): csp.txt — 47 inline script hash(es) over 119 occurrence(s), no external source
documentation links — every address against the files behind it
    ok        19 page(s), 223 file(s) in the output
    ok        738 link(s) followed, 18 hreflang pair(s) checked
    ok        9 page(s) name another page canonical and are read as it
    ok        12x github.com — the canonical source repository, linked by the landing
    ok        11x gitverse.ru — the source mirror, linked by the landing
    note      1 citation(s) into documentation this site does not carry:
              /doc/com.example/subject/latest/common/PROP-001/
    L-01      8x — The island golden cites `media/diagram.svg` relative to the page, …
links: green — 738 checked, 0 broken.
build (static): ok
STATIC_EXIT=0
```

```
$ node tools/parity.mjs <reference-dist>
parity: green — 31 deliberate difference(s), 0 unexplained.
```

Статическая страница документации после хвоста по-прежнему несёт
`rel="canonical"` (1) и `og:image` (1) — ветка `local` на неё не влияет;
Playwright — 29 из 29.

```
$ node tools/root-files.mjs dist
root-files: wrote robots.txt, llms.txt, llms-full.txt, sitemap.xml, feed.xml, og.png; 8 font file(s) published, 0 page(s) pointed at the bundled faces
```

(Ноль — правильный ответ: preload-ссылки в этом `dist` сборка уже
переписала, и второй прогон переписывать нечего. До правки эта строка
печатала `undefined preload(s)`.)

### А-2 наблюдалась ещё дважды, и у неё нашёлся дешёвый признак

Во время хвоста аномалия А-2 (двойная шапка на лендинге) сработала ещё
раз и снова ушла на следующей сборке **того же коммита**: 4 наблюдения
за сессию, примерно поровну. Полезное следствие: её видно по числу в
линтере ссылок — **738 проверенных ссылок при одной шапке и 750 при
двух** (лишняя документационная шапка добавляет по 4 ссылки на каждую из
трёх лендинговых страниц). То есть приёмке не нужен паритет, чтобы
заметить: расхождение в строке `link(s) followed` статической сборки —
уже сигнал, и пересборка его снимает.

## `git status --short` на момент сдачи (мои пути)

```
$ git status --porcelain -- vibevm/vibepacks/org.vibevm.doc/web/v0.1.0
(пусто)
```

Сам этот отчёт оставлен **незакоммиченным** — по заведённому в кампании
порядку его вносит центральная сессия вместе с записью в леджер.
