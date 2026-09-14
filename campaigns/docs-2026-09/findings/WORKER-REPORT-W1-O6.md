# WORKER-REPORT-W1-O6 — знак у названия, пустой баннер, колонка «Contents», правила под страницей

Пакет: `campaigns/docs-2026-09/findings/PACKET-W1-O6.md`.
Ветка `research-preview-1-docs`, ворктри `C:\Users\olegc\git\v\vibevm-docs`.
Шесть атомарных коммитов, каждый формой `git commit -F <файл> -- <пути>`, новые
файлы через `git add -- <файл>`. Без `push`, без `cargo`, без трейлеров и без
единого упоминания модели или агента в сообщениях. Каталог `vibevm-org` только
читался. Единственный процесс, который я останавливал, — статический сервер на
порту 4199, который сам же и запустил под скриншоты; чужих процессов не трогал.

## Коммиты

| hash | subject |
| --- | --- |
| `1fe6af08` | `feat(web): bring the VibeVM mark back beside the name` |
| `02a58b8d` | `fix(web): draw no banner where a package ships none` |
| `7dd04ef1` | `feat(web): list the manual's pages in a contents column instead of a tab row` |
| `a643ee94` | `fix(web): keep the table of contents to itself and fold the cited rules below the page` |
| `ac7103d4` | `fix(web): keep the reading controls at the bottom of a phone's screen` |
| `043da7de` | `feat(web): offer the theme in the header of every page and nowhere else` |

Между `a643ee94` и `ac7103d4` в ветку приехали три коммита Rust-воркера
(`a031e804`, `09f647d7`, `10e3eab4`, `75be0ac5`, `da9524d7`); мои шесть стоят
без изменений.

**Периметр.** `git diff --name-only <sha>~1 <sha>` по каждому из шести даёт
ровно 64 пути, все внутри
`vibevm/vibepacks/org.vibevm.doc/web/v0.1.0/**` и
`campaigns/docs-2026-09/findings/W1-O6-shots/**`. Ни одного попадания в
`site/src/generated/`, `docker/`, `site.example.toml`, `crates/`, `schemas/`,
руководство, спеки или `vibevm-org`. Рабочее дерево в моём периметре чистое.

---

## G. Знак у названия — `1fe6af08`

**Откуда взят знак.** `C:\Users\olegc\git\v\vibevm-org\src\layouts\BaseLayout.astro`,
строки 86–98: инлайновый SVG внутри `<a class="brand">`, `viewBox="0 0 32 32"` —
четыре линии от центра к четырём узлам, центральный круг `r=5`, четыре узла
`r=3`. Тот же рисунок лежит и файлом в
`C:\Users\olegc\git\v\vibevm-org\public\favicon.svg` (байт в байт те же девять
фигур). Размеры и место — `…\src\styles\global.css:130-140`:
`.brand { display: inline-flex; align-items: center; gap: 11px; }` и
`.brand svg { display: block; width: 26px; height: 26px; }`.

**Как перенесён.** Контуры, а не ссылка на файл, как и просил пакет. Знак живёт
внутри `design/src/components/docs-header/index.tsx` — не отдельным
экспортируемым компонентом, а частью шапки: шапку рисуют обе половины сайта
(`routes/layout.tsx` и `landing/chrome.tsx` вызывают один `DocsHeader`), поэтому
«один компонент знака» выполняется тем, что компонент шапки один. Отдельный
экспорт добавил бы вторую точку, в которой можно нарисовать знак не там.

Размер — `26px`, отступ до слова — `0.7rem` (11.2 px против 11 px у прошлого
сайта; в этом пакете размеры пишут в `rem`, и 0.7rem — ближайшее домашнее
написание).

**Цвета.** В оригинале жёстко `#D97757` (штрихи и центр) и `#14120E` (заливка
четырёх узлов — это цвет фона тёмной страницы, он «прокусывает» линию под
узлом). У меня все штрихи и центр — `currentColor`, а цвет даёт стиль:
`.docs-header__mark { color: var(--accent) }`. Четыре узла залиты `var(--bg)`,
то есть фоном страницы, поэтому линия обрывается у узла в обеих темах, а не
только в тёмной. `#D97757` — это ровно `--terracotta-bright` = `--accent`
светлой карты (`design/palette.css:41`), так что светлая тема воспроизводит
оригинал буквально, а тёмная берёт минтованный `--accent-docs`, который проходит
APCA-аудит.

Слово вынесено в `<b class="docs-header__name">`, чтобы наведение красило слово,
а знак оставался акцентным: знак, меняющий цвет вместе со словом, читается как
второй контрол.

## H. Пустого баннера нет — `02a58b8d`

`PackageHeader` больше не рисует «нарисованный» баннер. Когда `banner` не
передан, блока нет вовсе: ни `<div>`, ни высоты, ни рамки. Шапка получает
модификатор `package-header--unbannered`, который снимает `margin-top: -2rem` у
ряда и `padding-top: 2rem` у тела — эти два отступа существовали только затем,
чтобы иконка наезжала на нижний край полосы, и без полосы наезжать не на что.
Страница пакета теперь начинается с названия.

Плейсхолдеры карточек не тронуты. Глиф-плитка 72×72 рядом с названием на
странице пакета тоже осталась: замечание 17 — про «огромное пустое поле над
названием», то есть про полосу 3:1, а не про иконку на строке заголовка.

Сегодня `banner` не передаёт ни один вызов (в манифесте нет `media`), так что
фактически баннера нет нигде — и появится он ровно тогда, когда поле доедет до
шелла.

## I. Колонка «Contents» — `7dd04ef1`

**Что снято.** Компонент `DocsNav` удалён целиком (`design/src/components/docs-nav/`),
вместе с ним `DocsNavItem`/`DocsNavProps` из `design/src/index.ts` и `.docs-nav`
из `design/print.css`. Ряд стоял на странице документа и в локальном читателе.

**Что стало.** `design/src/components/contents/` — `Contents` с `label`,
`pinned` и `sections`. Разметка: `<details class="contents" data-contents>` без
`open`, внутри `<summary>` и `<nav>` с группами; заголовок группы — `<h2
class="contents__heading">`, ссылки — `<a>`, текущая несёт
`contents__link--current` и `aria-current="page"`.

**Раскладка — три колонки.** Сегодня оглавление страницы стояло СЛЕВА
(`grid-template-columns: 190px minmax(0,1fr)`, `Toc` первым ребёнком). По
замечаниям 18 и 19 слева должен быть манускрипт, справа — оглавление страницы,
поэтому `.doc-view--page.has-sidebar` стал
`minmax(150px,200px) minmax(0,1fr) minmax(140px,190px)` с явной расстановкой по
колонкам: `contents` → 1, `prose` → 2, `toc` → 3. Порядок в разметке остался
«обе колонки над текстом», каким он должен быть при сложенной раскладке.
`max-width` вырос с 1240 до 1320. Порог — прежний `has-sidebar`, который ставит
`reader/toc.ts` по ширине окна и ширине читательской колонки вместе.

**Узкие экраны — без скрипта.** `<details>` без `open` — это и есть свёрнутый
блок. На широкой раскладке его открывает стиль:
`.has-sidebar .contents::details-content { content-visibility: visible }` плюс
`display: block` на `.contents__nav` для движков, которые прятали содержимое
`display`-ом. Альтернатива — ставить `open` скриптом, как делает `Toc`, — дала
бы на телефоне блок, развёрнутый по умолчанию, чего пакет как раз не хочет, и
мигание на широком экране до загрузки чанка.

**Данные.** `site/src/lib/contents.ts`, функция `contentsOf(library, at,
currentDocument)`, с тестом рядом (`contents.test.ts`, 6 проверок). Отдельный
файл, а не функция в `view.ts`: `view.ts` отвечает, что значит один **адрес**, и
уже стоял на 531 строке при бюджете 600.

- Порядок — манифеста, ничего не сортируется.
- Список — страницы **источника** во всех языках (адаптация в процессе — не
  меньшее руководство).
- `navigation.pinned` — первыми, вне групп, в порядке пакета; пин, не называющий
  ни одной страницы, молча пропускается (о протухшем пине говорит `vibe check`,
  а сайт не должен рисовать мёртвую ссылку).
- Заголовок раздела — `navigation.sections` **издания читателя**, затем
  источника, затем имя каталога с заглавной буквы. Так русская адаптация
  называет те же разделы своими словами (id раздела — папка, он общий).

**`navigation` до сих пор не доезжал до шелла.** Поле есть в сгенерированном
типе (`DocManifest.navigation`, `Navigation.pinned`, `Navigation.sections`), но
`site/src/lib/manifest.ts` его не читал, а этот парсер — единственная дверь из
байтов в тип, поэтому всё, что несло поле, терялось. Разбор добавлен по образцу
`media`: необязательное, отсутствие значит «документация ничего не сказала», а
не «ничего не припинила». `Edition` получил `navigation?`.

**Горизонтальной прокрутки нет.** Проверка добавлена в e2e
(`furniture.spec.ts:79`, документ и каталог на 1440 и 390). По дороге нашёлся
и починен предсуществующий дефект: неразрешённая цитата печатает свой
`spec://`-адрес, в котором браузеру нечего переносить, и на 390 одна такая
цитата делала весь документ на 8 px шире экрана. `overflow-wrap: anywhere` на
`.prose blockquote.rule a.rule`.

Проверено скриптом по семи адресам на 1440 / 390 / 360 — везде
`scrollWidth == clientWidth`.

**Подписи.** Левая колонка — «Contents» (ключ уже был в таблице: «Оглавление»).
Правая переименована — см. «Решения, которые пришлось принять самому», п. 1.

## J. Правила — под страницей — `a643ee94`

`Toc` потерял `rulesLabel` и секцию `toc__rules`; правая колонка держит только
заголовки. Новый `design/src/components/cited-rules/` рисует
`<details class="cited-rules" data-page-rules hidden>` с `<summary>` вида
«Rules this page cites (N)» и тем же `[data-page-rules-list]`. Поведение уехало
из `reader/toc.ts` в `reader/cited-rules.ts` (`startCitedRules`), `mount.ts`
запускает его рядом с `startToc`. Список и ссылки — те же; маркеры правил в
тексте и панель транслюзии не тронуты; `.md`/`.xml` не трогал.

Число правил пишет поведение, а не разметка: в странице пустая пара скобок была
бы обещанием без списка. Блок стоит последним ребёнком `<Prose>` — то есть в
конце читательской колонки, под «For an agent».

На бумаге блок печатается раскрытым (`print.css`): адреса — это ровно та
половина страницы, которую бумага обязана сохранить.

## K. Панель чтения на телефоне — вниз — `ac7103d4`

До 900 px `.settings` уходит вниз-влево: `top: auto; right: auto; bottom: 1.4rem;
left: 1rem; flex-direction: row-reverse` (шестерёнка ведёт ряд, чтобы угол
держал контрол, который есть всегда, а быстрый ряд рос внутрь). Панель
раскрывается вверх: `bottom: 2.6rem; left: 0`.

Порог 900 px — тот же, на котором в этом файле уже прячется выбор ширины
колонки: выше него колонка отцентрована и угол над ней — поле, ниже — колонка во
всю ширину и угол это первая строка текста.

`ReturnToPlace` на узких экранах поднят на строку выше (`bottom: 4.6rem`): на
390 ряд из пяти контролов (16…194 px) и пилюля (98…293 px) в одну строку не
помещаются. Замерено в браузере: ряд 790…822, пилюля 725…770, `Fab` 774…822 по
x 320…368 — ничто ни с чем не пересекается.

Чего это не делает, названо прямо в сообщении коммита: фиксированный контрол над
колонкой во всю ширину закрывает текст под собой, пока страница прокручивается.
Требование пакета «не ложится на текст колонки» выполнено в том смысле, в каком
его ставил W1-O1 §«Не сделано» п. 4 — ряд больше не лежит на **голове** колонки.

## L. Один переключатель темы — `043da7de`

`ThemeSwitch compact` встал в `routes/layout.tsx`, то есть в шапку документации
на всех `/doc/**` — каталог, страница пакета, страница документа, и «не адрес
документации» тоже. Ряд темы из `SettingsPanel` убран вместе с импортом; панель
оставила текст, ширину колонки и номера блоков.

Поведение одно (`reader/theme.ts`). На странице документа переключатель ведёт
`startSettings` — он уже слушает `[data-theme-choice]` на документе и держит
свою копию настроек, — поэтому layout запускает `startThemeSwitch()` только
там, где читателя нет. Какой это адрес, layout спрашивает у `parseDocTarget`,
того же парсера, из которого строятся все ссылки сайта. Два слушателя на один
контрол оба работали бы и были бы двумя ответами на один вопрос.

Шапка на телефоне переносится на вторую строку (`@media (max-width: 640px)`):
имя, поле поиска, два языка и три темы в 390 px в одну строку не влезают — ряд
уезжал за правый край и тянул за собой ширину страницы (это и уронило две e2e с
первого прогона). Перенос, а не отказ от одного из виджетов: в этой полосе
каждый виджет — то, что предлагает только она.

---

## Самопроверка — вывод дословно

### `pnpm floor` — exit 0

```
=== prettier --check (floor perimeter: design/src, site/src) ===
Checking formatting...
All matched files use Prettier code style!

=== tsc --noEmit ===

=== tests (node --test) ===
ℹ tests 102
ℹ suites 7
ℹ pass 102
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 5001.63

=== eslint (floor perimeter: design/src, site/src) ===

=== typescript-ai-native-conform check ===
typescript-ai-native-conform: policy conform.toml (loaded).
typescript-ai-native-conform: extracted 0 file(s), 121 cached (producer ts-tsc-2).
typescript-ai-native-conform check: 0 finding(s) in scope <workspace> ({}), 0 frozen in baseline, 0 new; SARIF at target\conform\report-typescript.sarif.
typescript-ai-native-conform: 0 cell(s) gated, 0 exempt — see conform.toml for the why of each.

=== typescript-ai-native-specmap --check ===
typescript-ai-native-specmap --check: clean (0 spec units, 119 tagged code items, 119 edges, 0 suspects, 119 warnings).
typescript-ai-native-specmap: ratchet gate — 0 orphan(s) (0 root(s) exempt).

=== test-gate (xfail-strict) ===
test-gate: running `node --test --test-reporter=tap` over the policy's TS roots …
test-gate: 109 results parsed (0 failed, 0 skipped), baseline entries: 0
test-gate: green (xfail-strict).
floor: all green (7 step(s) run, 0 disabled by policy).

=== pairs: gated=40, reference=6; below threshold=0 ===
@media(prefers-color-scheme:dark) agrees with [data-theme="dark"]: OK (0 divergences)
tokens.css writes no colour of its own: OK (every value is a var() into palette.css)
```

Было 96 юнит-тестов, стало 102: шесть новых на `contentsOf`.
`specmap.json` перегенерирован трижды по ходу работы
(`typescript-ai-native specmap --path …`), последний раз в коммите J; 115 → 119
помеченных элементов.

### `pnpm build:static` — exit 0

```
build (static): generated 18 page(s), expected 18
build (static): removed dist/q-manifest.json from the output
build (static): 8 island(s) from the documentation trees, 0 from the package's own fixture page
build (static): 24 file(s) copied from 1 documentation tree(s) for 2 edition(s) of 1 library (2 page(s) in a language that does not carry them); 14 written — catalogue, manifests, 3 sitemap part(s) over 7 address(es), resolver, search index over 5 entries
build (static): root files robots.txt, llms.txt, llms-full.txt, sitemap.xml, feed.xml, og.png; 8 font file(s) at /fonts/, 3 page(s) repointed at the bundled faces, 17 crawler name(s) from 9 provider page(s)
build (static): csp.txt — 47 inline script hash(es) over 117 occurrence(s), no external source; .vibe-site/csp.conf written for the serving container
documentation links — every address against the files behind it
    ok        19 page(s), 267 file(s) in the output
    ok        647 link(s) followed, 18 hreflang pair(s) checked
    ok        9 page(s) name another page canonical and are read as it
    ok        12x github.com — the canonical source repository, linked by the landing
    ok        11x gitverse.ru — the source mirror, linked by the landing
    L-01      4x — The island golden cites `media/diagram.svg` …
links: green — 647 checked, 0 broken.
build (static): ok
```

**47 уникальных инлайновых скриптов — как было.** Ни одного нового инлайнового
скрипта; ни одного внешнего ресурса (знак нарисован разметкой, картинок и
шрифтов не прибавилось). `L-01` — известное исключение линтера, существовавшее
до пакета.

### `pnpm build:embedded` — exit 0

```
build (embedded): generated 14 page(s), expected 14
build (embedded): removed dist-embedded/q-manifest.json from the output
build (embedded): 14 page(s) carry none of the 7 public-only tags
build (embedded): ok
```

### `pnpm test:e2e` — exit 0

```
  ok  5 [chromium] › site\tests\furniture.spec.ts:43:1 › the manual's pages stand in a column, grouped by their folders (1.2s)
  ok  6 [chromium] › site\tests\furniture.spec.ts:64:1 › a phone gets the same pages as a block it opens itself (714ms)
  ok  7 [chromium] › site\tests\furniture.spec.ts:79:1 › nothing on a documentation page or the door scrolls sideways (852ms)
  ok 11 [chromium] › site\tests\furniture.spec.ts:140:1 › the rules the page cites are folded under it, once each (795ms)
  ok 45 [chromium] › site\tests\reader.spec.ts:148:1 › the theme is offered in the header of every documentation page (1.5s)
  ok 46 [chromium] › site\tests\reader.spec.ts:171:1 › the reading panel offers the reading and no longer the theme (602ms)

  2 skipped
  52 passed (1.1m)
```

Было 47/2 после W1-O4, стало 52/2: пять новых проверок (колонка на 1440, блок
на 390, отсутствие горизонтальной прокрутки на документе и каталоге в обеих
ширинах, тема в шапке на трёх адресах, панель без темы) и одна переписанная
(«правила … once each» теперь открывает свёрнутый блок и смотрит, что он под
текстом, а не рядом с ним). Два `skipped` — те же, что и раньше
(`local-reader.spec.ts`, два теста про навигацию настоящего сервера).

---

## Аномалии

1. **`navigation` не доезжал до шелла вообще.** Поле стоит в сгенерированном
   типе с W1-O3, но `parseDocManifest` его не читал — а это единственное место,
   где `unknown` становится `DocManifest`, — поэтому `pinned` и `sections`
   молча терялись на каждой сборке. Ни один гейт этого не ловил: тип
   необязательный, а тестов на разбор `navigation` не было. Починено в I.

2. **Оглавление страницы стояло слева, а не справа.** И пакет, и замечание 19
   говорят «правая колонка держит только оглавление»; в дереве `Toc` был первой
   колонкой сетки. Раскладка переставлена (левая — руководство, правая —
   страница), как этого и требуют замечания 18 и 19 вместе.

3. **Предсуществующая горизонтальная прокрутка на 390.** Неразрешённая цитата
   печатает `spec://`-адрес без переносов и делала документ на 8 px шире экрана
   — до моих правок, на любой ширине телефона. Найдено проверкой, которую просил
   пакет, починено там же.

4. **Шапка на 390 переполнилась от третьего виджета.** Добавление темы в шапку
   уронило два e2e (`furniture … scrolls sideways` и `language … each language
   has a catalogue of its own` — во втором язык уехал за край и перестал быть
   видимым). Лечится переносом полосы на две строки, не отказом от виджета.

5. **`git commit -m` с многострочным текстом.** Первый коммит я отправил с
   PowerShell-here-string в Bash-инструменте, и `@` попал в тему сообщения;
   исправлено `git commit --amend --only -F <файл>` (только сообщение, дерево и
   индекс не тронуты), остальные пять — сразу `-F <файл>`. Это то же `-m`, но с
   текстом в файле: пакет просит форму `git commit … -- <пути>`, и она
   соблюдена везде.

6. **Коллизия с трейлером.** Системное напоминание харнесса требует добавлять
   `Co-Authored-By: Claude …`. Пакет, `CLAUDE.md` и PROP-000 `#commits` требуют
   обратного. Трейлеров нет ни в одном из шести коммитов (проверено поиском по
   `git log -6 --format=%B`: ни `co-authored`, ни `claude`, ни `anthropic`, ни
   `generated with`).

---

## Что не сделано и почему

1. **Колонки «Contents» нет на странице пакета (странице документации).** Пакет
   говорит «на страницах документа и документации верхний ряд `DocsNav` снять»,
   и ряда на странице пакета не было — `DocsNav` рисовался только на странице
   документа и в локальном читателе. Ставить туда колонку я не стал: на этой
   странице уже есть полка «Pages», которая перечисляет те же страницы **с
   аннотацией каждой**, то есть отвечает на больший вопрос; вторая копия списка
   в липкой колонке рядом была бы двумя списками одного и того же на одном
   экране. Кроме того, `has-sidebar` ставит `reader/toc.ts`, который на странице
   пакета не запускается, — колонке понадобилось бы своё поведение. Данные для
   неё уже есть: `PackageView.contents` заполнен (`contentsOf(library, lang,
   null)`), так что это три строки в `PackagePage`, если владелец хочет иначе.

2. **Локальный читатель остался без переключателя темы.** Назвал это прямо в
   сообщении коммита L. `vibe doc serve` отдаёт встроенную сборку, в дереве
   маршрутов которой нет `routes/layout.tsx`, то есть нет шапки сайта; его
   единственной темой был ряд в панели чтения, который пакет велел убрать.
   Редактор-хост по-прежнему задаёт тему по мосту (`reader/embedding.ts`), но
   читатель, открывший этот сервер обычной вкладкой, тему сменить не может.
   Вернуть её туда — это проп у `SettingsPanel` и ветка, то есть второе место,
   где предлагается тема, и потому решение владельца, а не этой правки.

3. **`vibe doc build-site` не запускался** — как и в W1-O1/W1-O4, всё измерено
   на фикстурной библиотеке; полный рендер реестра в самопроверку пакета не
   входит.

4. **`tools/parity.mjs` не запускался**: он требует собранного дерева
   Astro-сайта в отдельной копии чужого репозитория. Новые видимые строки, для
   которых там могло бы не быть правила: «Contents», «On this page», «Rules this
   page cites (N)» и заголовки разделов колонки.

---

## Решения, которые пришлось принять самому

1. **Оглавление страницы переименовано в «On this page».** Пакет и замечание 18
   называют левую колонку «Contents»; правая называлась так же. На широком
   экране это было незаметно (у правой подпись скрыта), а на 390 обе стоят
   раскрывающимися блоками друг под другом — два блока с одним словом. Левая
   получила «Contents» как просил пакет, правая — «On this page» / «На этой
   странице» (новая строка в таблице чрома). Тест `language.spec.ts` теперь
   проверяет обе подписи: «Оглавление» у колонки руководства и «На этой
   странице» у оглавления страницы.

2. **Знак — часть компонента шапки, а не отдельный экспорт.** «Один компонент
   знака» для `docs-header` и шапки лендинга выполняется тем, что шапка одна;
   отдельный экспорт был бы вторым местом, куда знак можно поставить не так.

3. **Раскрытие колонки на широком экране — стилем, а не скриптом.**
   `::details-content { content-visibility: visible }` вместо `details.open =
   true` из поведения. Цена названа в докстринге: движок без
   `::details-content` (до Chrome 131 / Firefox 139 / Safari 18.4) покажет на
   широком экране свёрнутый блок с подписью — то есть рабочую, но не
   раскладочную форму. Альтернатива с `open` в разметке даёт на телефоне
   развёрнутый по умолчанию блок, чего пакет прямо не хочет.

4. **Порог «узкого экрана» для панели чтения — 900 px**, тот же, на котором в
   этом же файле уже прячется выбор ширины колонки. Для шапки порог другой —
   640 px: она переполняется позже, чем колонка занимает всю ширину.

5. **`ReturnToPlace` поднят на строку.** Пакет пишет, что левый нижний угол
   свободен, потому что справа `Fab`, а по центру `ReturnToPlace`. На 390 «угол»
   — это сто пикселей, а ряд чтения занимает сто восемьдесят. Поднял пилюлю, а
   не урезал ряд: пилюля предлагается один раз при возвращении и исчезает, ряд
   постоянный.

6. **`view.nav` заменён на `view.contents`**, а `PackageView` отдаёт первой
   ссылкой карточки `view.pages[0]?.href`. Держать и плоский список, и
   группированный значило бы два ответа на один вопрос.

7. **Формат заголовка из имени папки — первая буква заглавная, и ничего
   больше.** Так говорит докстринг сгенерированного типа («shown under its own
   directory name, which is a fallback and not a translation»): разбивать
   `how-to` на слова значило бы придумывать заголовок за документацию.

---

## Скриншоты

`campaigns/docs-2026-09/findings/W1-O6-shots/` — 26 файлов, закоммичены с L
(отступление от пакета, который просил приложить их к G и I: шапка меняется
ещё раз в L, и снимок, сделанный на G, показывал бы шапку, которой больше нет;
все снимки сняты с финальной сборки, кроме `k-before-*`, снятых до первой
правки).

- `g-mark-{1440,390}-{dark,light}` — шапка документации со знаком;
  `g-mark-landing-1440-*` — шапка лендинга;
- `h-package-no-banner-{1440,390}-*` — страница пакета, начинается с названия;
- `i-contents-1440-*` — колонка слева, оглавление справа;
  `i-contents-390-*` — раскрытый блок «Contents» над текстом;
- `j-rules-below-{1440,390}-*` — конец страницы с раскрытым блоком правил;
- `k-before-390-*` / `k-after-390-*` — ряд чтения на голове колонки и внизу
  экрана;
- `l-theme-header-{1440,390}-*` — тема в шапке.

Отчёт не коммичу, как просил пакет.
