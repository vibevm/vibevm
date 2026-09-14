# WORKER-REPORT-W1-O1 — шесть замечаний владельца после первого просмотра

Пакет: `campaigns/docs-2026-09/findings/PACKET-W1-O1.md`.
Ветка `research-preview-1-docs`, ворктри `C:\Users\olegc\git\v\vibevm-docs`.
Шесть атомарных коммитов, без `push`. `cargo` не запускался; `tools/self-check.sh`
не запускался; чужие процессы не останавливались.

## Коммиты

| hash | subject |
| --- | --- |
| `25443af6` | `feat(web): copy an install line from the landing with one click` |
| `722d0c01` | `feat(web): put the theme switch on the landing and default to dark` |
| `9f88a9a7` | `fix(web): make the header search find pages` |
| `0ee2c5d5` | `fix(web): align the card glyph with its title on wide screens` |
| `c6737a92` | `fix(web): show a page card the abstract of that page` |
| `6b077771` | `fix(web): keep the reading controls clear of the header widgets` |

Периметр соблюдён: `git diff --name-only 818247cd..HEAD` не даёт ни одного файла
вне `vibevm/vibepacks/org.vibevm.doc/web/v0.1.0/**` и
`campaigns/docs-2026-09/findings/W1-O1-shots/**`. Рабочее дерево в периметре
чистое.

---

## A. Кнопка копирования у строк установки

Коммит `25443af6`.

Рядом с каждой командой — кнопка-пиктограмма (инлайновый SVG в
`design/src/components/hero/install-block.tsx`, никаких внешних ресурсов),
`aria-label` из `i18n.ts` («Copy» / «Скопировать»), обработчик — Qwik `onClick$`,
копируется ровно `entry.command` (без промпта `$` / `PS>`), подтверждение
«Copied» / «Скопировано» живёт 1,5 с.

Три вещи, которые пришлось решить по дороге.

- **Кнопки нет, если нет clipboard API.** На сервере это неизвестно, поэтому
  разметка рендерит её `hidden`, а первый клиентский кадр открывает. Стратегия
  `document-idle`, а не умолчание: умолчание (`intersection-observer`) ждёт, пока
  панель попадёт в область просмотра — на 390 px это происходит уже после того,
  как читатель дошёл до строки и кнопки там не нашёл. Проверено: до правки
  кнопка на `/ru/` при 390 px оставалась `hidden`.
- **Живая область — рядом с кнопкой, а не внутри.** `aria-label` кнопки
  перекрывает её содержимое как доступное имя, так что слово внутри кнопки
  диктор не прочитал бы. Подтверждение — отдельный `<span role="status">`.
- **Горизонтальное переполнение на телефоне.** Строка теперь скроллится в
  собственном `.install__line`, а кнопка стоит снаружи скроллера — иначе она
  уезжала бы вместе с текстом. Скроллер переехал на уровень ниже, и
  `.install__cmd` (grid item) потерял нулевой автоминимум, который получал от
  `overflow-x: auto` даром: панель стала шириной 506 px внутри 350-пиксельной
  колонки. Лечится одним `min-width: 0` на `.install__cmd`; измерено до и после
  (`scrollWidth` 526 → 390).

Скриншоты: `a-copy-1440-dark.png`, `a-copy-390-light-ru.png`.

---

## B. Переключатель темы на главной; умолчание — тёмная

Коммит `722d0c01`.

Отдельного компонента-переключателя не было: на страницах документации тему
меняли три кнопки, написанные прямо в `SettingsPanel`. По пакету («вынести в
`design/src/components`, чтобы был один») они вынесены в
`design/src/components/theme-switch/`. Панель чтения использует словесную форму
(«light / dark / system», выглядит как раньше), шапка лендинга — компактную:
три нарисованных знака в одной пилюле, имена в `aria-label`, три состояния, не
два.

Поведение тоже одно. `site/src/reader/theme.ts` — новый модуль: что такое тема,
где хранится выбор, что делает смена (штамп на `<html>`, отметка **каждого**
`[data-theme-choice]` на странице). `reader/settings.ts` стал его вызывающим, а
не вторым мнением; лендинг, у которого нет ни колонки, ни номеров блоков,
запускает `startThemeSwitch()` в одиночку.

Умолчание `dark`:

- `site/src/config.ts` — константа `DEFAULT_THEME` с причиной (ревью
  2026-09-14); незнакомое слово в `VITE_SITE_DEFAULT_THEME` теперь падает в неё,
  а не в `system`;
- `docker/site.toml` — `default_theme = "dark"`, выписано явно;
- `site.example.toml` — значение и абзац про то, что выбор читателя сильнее.

Обновлены пиннившие «system» тесты: `site/src/config.test.ts` (два теста плюс
докстринг, добавлена проверка, что `system` по-прежнему достижим явным
запросом) и `site/src/seo/deployment.test.ts` (новая проверка
`default_theme = "dark"` в конфигурации контейнера).

**`DEFAULT_SETTINGS.theme` тоже пришлось изменить** — иначе правка была бы
наполовину мёртвой: `startSettings` применяет умолчания при пустом хранилище, а
`apply("system")` **снимает** `data-theme`, то есть отменяет то, что
`theme-init.js` уже поставил. Теперь это `SITE.defaultTheme`.

`design/theme-init.js` не тронут: его собственный `var DEFAULT = "system"` — это
валидное значение файла, стоящего отдельно, которое сборка подменяет на
`SITE.defaultTheme`. Существующий тест `theme-init` остался зелёным без правок,
как пакет и предполагал.

Проверено в браузере: свежий лендинг — `data-theme=dark`, отмечен «dark»; клик
по «light» пишет хранилище и переносится на `/doc/` и `/ru/`; свежая страница
документации — `dark`, панель отмечает `dark`.

Скриншоты: `b-theme-landing-1440-dark.png`, `b-theme-landing-390-light.png`,
`b-theme-panel-1440-dark.png`.

---

## C. Поиск: причина и починка

Коммит `9f88a9a7`.

### Причина, как просил пакет

**Формы не было вообще.** `SearchBox` рендерил одинокий
`<input type="search" id="search-box-input">` с меткой и пилюлей `Ctrl K` — без
`<form>`, без `action`, без единого слушателя где-либо в `site/src` (проверено
поиском по всему дереву: слово «search» встречается только в самом компоненте,
в его подключении в `routes/layout.tsx` и в прозе комментариев). Маршрута
`/search` в дереве тоже нет. То есть поиск не «вёл на несуществующий маршрут» —
он никуда не вёл: ввод не вызывал ничего, Enter не отправлял ничего (отправлять
было нечему), а `Ctrl K` был обещанием, которое страница не умела исполнить.
Докстринг компонента это и говорил прямым текстом: «What it searches, and what
the shortcut opens, are wired over this shell» — просто провода так и не
появились.

CSP это подтверждает с другой стороны: политика несёт `form-action 'none'`, так
что вариант «сделать формой и отправлять» закрыт по построению.

### Что сделано

- **Индекс.** `tools/build.mjs` пишет `search.json` рядом с `manifest.json`,
  из тех же манифестов, из которых собраны каталог и sitemap: заголовок и
  аннотация документации, заголовок и первый факт страницы, координата. Только
  написание `latest`, без fallback-адресов. Отдельный документ, а не поле
  манифеста: манифест большой, потому что отвечает на другой вопрос, и читатель,
  набравший одну букву, не должен его ждать. Пишется компактно (его читает не
  человек).
- **Сопоставление** — чистая функция `site/src/lib/search.ts` с тестом рядом
  (`search.test.ts`, 11 проверок). Каждое слово запроса обязано куда-то попасть;
  вес: начало заголовка > заголовок > координата > контекст > аннотация; граница
  слова определяется через `\p{L}`, а не `\b`, иначе кириллица была бы особым
  случаем.
- **Ленивая загрузка.** `site/src/reader/search.ts` держит **промис** одного
  `fetch`, а не результат, — шесть нажатий за секунду дают один запрос. Ошибка
  сети = пустой ответ, а не исключение.
- **Виджет** — один. `SearchBox` знает, *когда* спрашивать и что делать с
  ответами, и ничего не знает о корпусе: он приходит пропом-вопросом `find$`.
  Стрелки ходят по списку, не уводя фокус из поля, Enter переходит, Esc
  закрывает, клик мимо закрывает, `Ctrl K` наконец делает то, что печатает
  пилюля. Результаты — настоящие ссылки (можно открыть в новой вкладке).

Проверено в браузере: индекс не запрашивается до ввода и запрашивается ровно
один раз после; «addr» → «Addresses» (слово из заголовка); «renderer» → «Fixture
Manual» (слово только из аннотации); стрелки/Esc/Enter работают; Enter приводит
на `/doc/com.example.docs/fixture-manual/latest/reference/addresses/`; на `/ru/`
плейсхолдер «Поиск» и находятся русские страницы.

### Аномалия, которую пришлось починить заодно

**На лендинге было ДВЕ шапки, два футера, два `<main>` и два элемента с
`id="search-box-input"`.** Это не моя правка: адрес лендинга — `index@landing.tsx`,
он называет `layout-landing!.tsx`, и `!` значит «это верхний layout, цепочка выше
не выполняется». Закреплённая бета этого не соблюдает и говорит об этом на каждой
сборке: «The "top" layout feature … has been deprecated». Именованный layout
разрешается, а корневой `routes/layout.tsx` рендерится вокруг него всё равно.
Один комплект рисовался ровно поверх другого (оба `z-index: 20`, побеждал
последний в DOM), поэтому дефект был невидим — ровно до момента, когда у поля
поиска появилось поведение и дубль `id` стал мешать: Playwright отказался
кликать по `#search-box-input` со словами «resolved to 2 elements».

Дефект существует с коммита `2780ba06` (порт лендинга). Проверено: файлы
маршрутов с тех пор не менялись, мой диф их не касается.

Починено минимально: корневой layout задаёт единственный вопрос, на который
может ответить, ничего не зная о лендинге, — «это адрес документации?»
(`docSegments(pathname) !== null`, функция из `lib/href.ts`, а не путь, вписанный
руками) — и уходит в сторону, если нет. Причина и путь отхода (route groups,
на которые указывает само предупреждение) описаны в докстринге файла. После
правки: по одной шапке, одному `<main>` и одному полю на каждой странице
(проверено подсчётом по всем шести построенным страницам).

### Решение, которое я принял сам

На 390 px четыре вещи в шапку лендинга не помещаются (замерено: `.landing-nav`
455 px при доступных 264). Ниже 520 px поле поиска уступает место входу в
документацию, языку и теме — то же поле стоит на один тап дальше, в шапке самой
документации, где при этой ширине больше ничего нет. Правило лежит рядом с уже
существующим, которое так же прячет две ссылки на зеркала.

Скриншоты: `c-search-doc-1440-dark.png`, `c-search-landing-1440-light-ru.png`.

---

## D. Иконка карточки на уровне названия

Коммит `0ee2c5d5`. Только CSS, разметка и данные карточек не тронуты.

От 768 px `.card__row` становится grid, `.card__body` — `display: contents`,
так что его трое детей становятся элементами сетки; размещается только знак
(колонка 1, ряд 1, по центру строки заголовка), остальное держит колонку 2 и
падает в ряды ниже в том же порядке. `gap: 0 0.6rem` — нулевой ряд, иначе
базовый `gap: 0.8rem` разогнал бы тело карточки по вертикали (это было видно на
первой пробе). Ниже 768 px всё как было: 48 px, сверху слева.

Скриншоты: `d-cards-1440-dark.png`, `d-cards-390-light.png`.

---

## E. Своя аннотация у карточки страницы

Коммит `c6737a92`.

Поле в манифесте — `DocPage.summary` («The page's leading fact — its first
paragraph… Taken, never composed»). Полка «Pages» передавала вместо него
`view.abstract`, то есть аннотацию всей документации, одну и ту же под каждым
именем на полке.

`Card.abstract` стал необязательным: карточка без своей аннотации не показывает
блок «what it covers» вовсе. В `lib/view.ts` добавлены тип `PageCard` и функция
`pageCards(library, at)`; `PackageView` получил поле `pages`. Это **не** тот же
список, что `nav`, и намеренно: навигация перечисляет страницы источника на
любом языке (читателю надо знать, какие страницы есть, прежде чем узнать, какие
адаптированы), а карточка отвечает на «про что эта» и потому берёт страницу
такой, какой её отдаёт выбранное издание — с текстом источника там, куда
адаптация не дошла. Правка применена к обеим полкам «Pages»: статической
(`PackagePage`) и локального читателя (`ServedHead`).

Проверено: карточка «Addresses» открывает «The shape of a documentation address,
which nothing in the product is allowed to invent» — свою. На русском издании
страница, которой у адаптации нет, показывает заголовок и аннотацию источника,
а переведённая — свои.

Других мест, где аннотация страницы подменялась чужой, нет: `PageView.summary`
используется в `seo/head.ts` (meta description, структурированные данные) и в
`seo/catalogue.ts` — везде правильно.

Скриншот: `e-page-abstracts-1440-dark.png`.

---

## F. Элементы управления чтением поверх шапки

Коммит `6b077771`.

**Воспроизведено** (`f-before-1440-dark.png`): при 1440 px долистать документ до
конца — появляется быстрый ряд, и он перекрывает правую часть шапки: пилюля
языка скрыта целиком, у поля поиска закрыт `Ctrl K`. Причина в двух строках:
`.settings` был `position: fixed; top: 0.7rem; right: 1rem; z-index: 32`, то есть
в том же углу и над `.docs-header` (`z-index: 20`), чей `__actions` прижат
`margin-left: auto` к правому краю. На 390 px было то же самое, только хуже
(`f-before-390-light.png`).

**Починка.** Шапка публикует свою высоту: `:root { --site-header-h: 64px }`
объявлено в стилях самой шапки — то есть переменная существует ровно тогда,
когда шапка на странице. Панель чтения стоит ниже полосы:
`top: calc(var(--site-header-h, 0px) + 0.7rem)`. В читателе, который `vibe`
отдаёт из локального хранилища, шапки нет, её стили не подключаются, fallback =
0 — и элементы стоят там же, где всегда.

Слои решены и в обратную сторону («и наоборот»): выпадашки шапки — результаты
поиска и список языков — открываются вниз, в ту самую полосу, где теперь стоит
панель чтения. Шапка поднята до `z-index: 40`, так что выигрывает та панель,
которую читатель только что открыл с полосы. Проверено скриншотами обоих случаев.

Скриншоты: `f-before-*.png` / `f-after-*.png`, 1440 и 390, обе темы.

---

## Гейты

```
$ pnpm -C vibevm/vibepacks/org.vibevm.doc/web/v0.1.0 floor        # exit 0
=== prettier --check (floor perimeter: design/src, site/src) ===
Checking formatting...
All matched files use Prettier code style!

=== tsc --noEmit ===

=== tests (node --test) ===
ℹ tests 88
ℹ pass 88
ℹ fail 0

=== eslint (floor perimeter: design/src, site/src) ===

=== typescript-ai-native-conform check ===
typescript-ai-native-conform: policy conform.toml (loaded).
typescript-ai-native-conform: extracted 0 file(s), 107 cached (producer ts-tsc-2).
typescript-ai-native-conform check: 0 finding(s) in scope <workspace> ({}), 0 frozen in baseline, 0 new; SARIF at target\conform\report-typescript.sarif.
typescript-ai-native-conform: 0 cell(s) gated, 0 exempt — see conform.toml for the why of each.

=== typescript-ai-native-specmap --check ===
typescript-ai-native-specmap --check: clean (0 spec units, 105 tagged code items, 105 edges, 0 suspects, 105 warnings).
typescript-ai-native-specmap: ratchet gate — 0 orphan(s) (0 root(s) exempt).

=== test-gate (xfail-strict) ===
floor: all green (7 step(s) run, 0 disabled by policy).

=== pairs: gated=40, reference=6; below threshold=0 ===
@media(prefers-color-scheme:dark) agrees with [data-theme="dark"]: OK (0 divergences)
tokens.css writes no colour of its own: OK (every value is a var() into palette.css)
```

```
$ pnpm -C vibevm/vibepacks/org.vibevm.doc/web/v0.1.0 build:static   # exit 0
build (static): generated 18 page(s), expected 18
build (static): removed dist/q-manifest.json from the output
build (static): 8 island(s) from the documentation trees, 0 from the package's own fixture page
build (static): 24 file(s) copied from 1 documentation tree(s) for 2 edition(s) of 1 library (2 page(s) in a language that does not carry them); 14 written — catalogue, manifests, 3 sitemap part(s) over 7 address(es), resolver, search index over 5 entries
build (static): root files robots.txt, llms.txt, llms-full.txt, sitemap.xml, feed.xml, og.png; 8 font file(s) at /fonts/, 3 page(s) repointed at the bundled faces, 17 crawler name(s) from 9 provider page(s)
build (static): csp.txt — 47 inline script hash(es) over 119 occurrence(s), no external source; .vibe-site/csp.conf written for the serving container
documentation links — every address against the files behind it
    ok        19 page(s), 249 file(s) in the output
    ok        651 link(s) followed, 18 hreflang pair(s) checked
    ok        9 page(s) name another page canonical and are read as it
    ok        12x github.com — the canonical source repository, linked by the landing
    ok        11x gitverse.ru — the source mirror, linked by the landing
    L-01      4x — [известное исключение, не моё]
links: green — 651 checked, 0 broken.
build (static): ok
```

```
$ pnpm -C vibevm/vibepacks/org.vibevm.doc/web/v0.1.0 build:embedded  # exit 0
build (embedded): generated 14 page(s), expected 14
build (embedded): removed dist-embedded/q-manifest.json from the output
build (embedded): 14 page(s) carry none of the 7 public-only tags
build (embedded): ok
```

Число инлайновых скриптов не выросло (47 хэшей, как и до правок) — ни одного
нового инлайнового скрипта не добавлено, как требовал пакет.

Сверх обязательного прогнал сквозной набор — он ходит по построенному `site/dist`
и покрывает панель чтения, тему на первом кадре, язык и переключатель платформ,
то есть ровно то, что правки B и F могли сломать:

```
$ pnpm -C vibevm/vibepacks/org.vibevm.doc/web/v0.1.0 test:e2e      # exit 0
  2 skipped
  38 passed (57.2s)
```

(Две пропущенные — те, что требуют запущенного `vibe doc serve` с настоящим
руководством; они пропускаются сами, я ничего не собирал.)

---

## Не сделано, и почему

1. **`vibe doc build-site` не запускался** — пакет прямо это разрешает
   («Полный рендер реестра не нужен»). Всё измерено на фикстурной библиотеке.
2. **`tools/parity.mjs` не запускался**: он требует собранного дерева Astro-сайта
   в отдельной копии чужого репозитория, которого здесь нет, и в самопроверку
   пакета не входит. Правило для новой видимой строки я всё же завёл — `D-16`,
   на метку поля поиска и пилюлю `Ctrl K`; переключатель темы видимого текста не
   добавляет (SVG-знаки плюс `aria-label`, а `visibleText` вырезает `<svg>`).
   **Владельцу стоит знать:** до правки C лендинг нёс вторую, «документационную»
   шапку целиком, а с ней — весь текст списка языков («English», «Русский»,
   «source», «official adaptation», …), под который правил в таблице нет. То
   есть сверка, если её прогнать, должна стать **чище**, а не грязнее; но
   утверждать это без прогона я не могу.
3. **Тему по-прежнему нельзя сменить на `/doc/` и на странице пакета.** Там есть
   шапка, но нет панели чтения (она живёт только на странице документа), а
   переключатель я, по пакету, поставил только на лендинг. Дефект
   предсуществующий; лечится одной строкой — тем же `<ThemeSwitch compact/>` в
   `routes/layout.tsx`, — но это добавит второй видимый переключатель на страницы
   документа, и это решение владельца, а не моё.
4. **На 390 px панель чтения теперь лежит поверх первой строки колонки** вместо
   шапки (видно на `f-after-390-light.png`). Требование пакета — «никогда не
   перекрывала шапку и её виджеты» — выполнено, но на телефоне любой фиксированный
   элемент перекрывает колонку: она во всю ширину. Честный ответ — увести ряд вниз
   экрана (левый нижний угол свободен: справа `Fab`, по центру `ReturnToPlace`) и
   раскрывать панель вверх. Это перекомпоновка мобильного чтения, которую пакет не
   просил; жду слова владельца.
5. **`design/theme-init.js` не тронут** — см. B.

## Аномалии

- **Двойная шапка лендинга** (см. C). Главная находка сверх списка: дефект с
  `2780ba06`, ломает `id`, `<label for>`, число `<main>` и удваивает байты шапки
  и футера на `/`, `/ru/`, `/en/` и `/404.html`. Починен, потому что без этого C
  не выполняется («один и тот же виджет»), но заслуживает отдельного взгляда:
  правильный долгий ответ — перестроить дерево маршрутов вокруг route groups, на
  которые указывает само предупреждение беты, и снять условие из
  `routes/layout.tsx`.
- **Гейт «никаких литеральных путей» ловит комментарии.** Тест
  `no source outside href.ts writes a documentation path as a literal` — это
  текстовый поиск `/["'`]\/doc\//`, поэтому обычная проза вида `` `/doc/ru/` ``
  в докстринге его роняет. Я переписал два своих комментария под домашнее
  написание (`vibevm.org/doc/ru/`). Гейт полезный, но сообщение об ошибке не
  говорит, что дело в комментарии.
- **`conform` ловит `as` даже в защитном парсере.** Первый вариант
  `readSearchIndex` использовал `value as {…}` и дал два `ts-unsafe-in-domain`.
  Переписан на предикат `isRecord`, как в `lib/manifest.ts`. Правильное поведение
  гейта, отмечаю как факт о том, как здесь пишут разбор `unknown`.
- **`useVisibleTask$` по умолчанию ждёт попадания в область просмотра.** Стоило
  одной итерации на A; в отчёте оставлено как факт об этой бете.

## Скриншоты

`campaigns/docs-2026-09/findings/W1-O1-shots/` (18 файлов, 742 КБ, закоммичены
вместе с F):

- `a-copy-*` — кнопка копирования и подтверждение, 1440/тёмная и 390/светлая (ru);
- `b-theme-*` — переключатель в шапке лендинга (1440 тёмная, 390 светлая) и
  словесная форма в панели чтения;
- `c-search-*` — выпадающий список на странице документации и на русском лендинге;
- `d-cards-*` — знак на строке заголовка (1440) и как было (390);
- `e-page-abstracts-*` — раскрытая «what it covers» у карточки страницы;
- `f-before-*` / `f-after-*` — по четыре: 1440 и 390, обе темы.
