# WORKER-REPORT-P4-O8 — сайт из многих библиотек: по одной на исходную документацию

Пакет: `campaigns/docs-2026-09/findings/PACKET-P4-O8.md` (атом + хвост X-055)
плюс дополнение оркестратора о фикстурных островах. Ветка
`research-preview-1-docs`, без push. Дата: 2026-09-12.

## Коротко для оркестратора

Атом и хвост сделаны, три коммита. Web-пакет больше не строит **одну**
библиотеку: манифесты группируются по координате источника, которую
адаптация называет в `translates`, и каждая группа становится своей
библиотекой со своими адресами, своим `latest` и своими языками.
`editionsOf` больше не отказывает двум источникам — отказывать нечему,
двух источников в одной библиотеке теперь не бывает по построению.

**Главные цифры.** Фикстурная сборка (без `VIBE_DOC_OUT`) — **18 страниц
из 18, 8 островов, 646 ссылок, 0 сломанных**, байт в байт как у P4-O6.
Две библиотеки в одной сборке (три фикстурных дерева) — **24 страницы из
24, 12 островов, 2 библиотеки, 3 издания, 4 части sitemap над 11
адресами, 796 ссылок, 0 сломанных**. Пол — зелёный, 48 node-тестов
(было 38); Playwright — **38 passed, 2 skipped** из 40 (шесть новых);
паритет лендинга — **зелёный, 31 осознанное отличие, 0 необъяснённых**.

**Живой прогон над всем реестром сдан целиком, вместе с web-половиной:
48 библиотек, 1237 страниц из 1237 ожидаемых, 1136 островов из деревьев
конвейера и 0 из фикстуры, 127 568 ссылок проверено, 0 сломанных,
49 частей sitemap, 212 МБ выдачи.** А-1 отчёта P5-O1 — «сборка сайта над
всем реестром сегодня невозможна» — закрыта.

Три вещи стоит прочитать отдельно.

1. **Группировка — одна, и живёт она в `lib/library.ts`.** Вопрос «какие
   манифесты составляют одну библиотеку» обязан иметь ровно один ответ,
   иначе страницы и машинные файлы описывают разные сайты. `seo/editions.ts`
   зовёт ту же `groupBySource`, а не повторяет её.
2. **Адаптация без источника в этой сборке становится библиотекой из
   самой себя, под собственной координатой.** Альтернатива — отказ — и
   она отвергнута по той же причине, по которой A5.3 не роняет рендер
   из-за одного сломанного пакета: реестр, публикующий чей-то перевод
   чужой документации, иначе не выложил бы ничего.
3. **Каталог — адрес САЙТА, а не документации.** `/doc/ru/` — русская
   полка сайта; три документации, переведённые на русский, это три
   карточки на ней, а не три адреса. Это одно правило, и оно проведено
   через список адресов, sitemap-индекс и гейт числа страниц.

## Хэши и subject'ы

| Что | Коммит | Subject |
|---|---|---|
| дополнение оркестратора | `c1917fd3` | `test(web): refresh the fixture islands from the pipeline` |
| атом | `b1e07385` | `feat(web): build one library per source documentation` |
| хвост X-055 | `4206e9ff` | `feat(web): take the link card from the manifest that names it` |

Между моими коммитами и концом сессии в ветку легли чужие (`985e2a56`,
`ce987890`, `3c96b78c`, `2355b20f` — P4-O7 и центральная сессия);
web-пакета после моего хвоста не касался никто (`git log --name-only
4206e9ff..HEAD -- <web-пакет>` пуст).

**Честно о том, что чем измерено.** Гейты сняты с вершины (все три
коммита вместе) и повторены после хвоста: пол, обе сборки, линтер,
паритет и Playwright — на состоянии `4206e9ff`. Коммит `c1917fd3` в
отдельности не измерялся: это байтовая копия двух фикстурных файлов и
одна строка причины в таблице исключений линтера, и формы сборки он не
меняет.

Этот отчёт **не закоммичен** — его вносит центральная сессия.

## Решения

### Решение 1 — группировка по `translates`, и один ответ на неё

`groupBySource(manifests)` в `site/src/lib/library.ts` возвращает список
групп «источник плюс его адаптации». Правила три, и все три проверены
тестами:

- адаптация принадлежит той документации, которую называет в
  `[translates]`, и никакой другой — не первому попавшемуся источнику и
  не тому, с кем делит язык;
- источник, которого никто не адаптировал, — библиотека из одного
  издания (это каждый пакет реестрового рендера);
- порядок — порядок, в котором названы манифесты, взятый по первому
  упоминанию каждой библиотеки: его задаёт `VIBE_DOC_OUT` выкладки, и два
  прогона над одними деревьями дают один порядок.

`seo/editions.ts` (машинные поверхности) **зовёт** эту функцию, а не
повторяет её. Два типа `Edition` и два типа `Library` в двух модулях
остались — это осознанная двойственность P4-O6 (один для оболочки,
другой для файлов, которые пишет `tools/build.mjs` на голом Node), — но
ответ на вопрос «кто с кем в одной библиотеке» ровно один.

### Решение 2 — адаптация без источника: библиотека, а не отказ

Манифест с `translates`, чьего источника в этой сборке нет, становится
источником собственной библиотеки под собственной координатой. D-06
запрещает служить адаптацию под её именем **потому, что рядом стоит
источник**, — а здесь его нет ни под каким адресом, и столкнуться не с
чем. Отказ отвергнут: реестр из сотен всегда будет держать перевод
чего-то, опубликованного в другом месте, и остановленный из-за него
рендер не выложил бы ничего (та же логика, что у «упавший пакет
становится страницей», A5.3, решение 4).

Локальный ридер получил это бесплатно: `vibe doc serve` над пакетом-
переводом раньше не показывал ни навигации, ни мета-строки (`parseLibrary`
падал с «no source manifest»), а теперь показывает его собственные.

### Решение 3 — каталог это язык сайта

`/doc/<язык>/` — полка САЙТА в этом языке. Из этого следует всё
остальное: в списке адресов язык стоит один раз, сколько бы библиотек в
него ни перевели (`siteAddressesOf` снимает дубли); в sitemap-индексе
каталоги — одна часть на весь сайт, и её `lastmod` — новейший из тех,
что несут стоящие на ней издания; гейт числа страниц считает
`1 + языков + Σ по библиотекам`.

Дверь `/doc/` перечисляет **по карточке на издание** всех библиотек, в
порядке библиотек, а внутри каждой — источник, потом издания со
звёздочкой, потом community. Звёздочка и слово — те, что посчитал
конвейер и несёт манифест (R-23: сайт ничего не пересчитывает; статус
источника — его `status` из манифеста, статус адаптации — `translation.status`).

Список карточек вынесен из компонента в `catalogueEntries(libraries)`
(`lib/view.ts`): JSX перестал решать, а node-тест получил возможность
спросить, что на полке, без браузера.

### Решение 4 — селектор языков в шапке: издание там, где издание есть

Селектор `LanguageSelector` построен, чтобы называть **издание** —
звёздочку, слово и издателя. На странице он и называет издание: языки
той документации, на которой стоит читатель (`libraryAt` по координате
адреса). Вне страницы — на лендинге, в двери, в каталоге языка — он ведёт
в каталог языка, у которого издателя нет вовсе.

Поэтому: язык, за которым стоит **одна** документация, показывается её
изданием — звёздочкой, словом и издателем, ровно как до меня; язык, за
которым стоит несколько, отвечает на «кем опубликовано» числом
(`3 documentations`) и звёздочки не несёт. Названный предел: первое слово
строки («source» / «community adaptation») в этом случае описывает
адресный уровень, а не издателя; правдивый ответ на «чья это полка» —
сама полка, куда ведёт пункт, и там у каждой карточки свой издатель и
своя звёздочка.

**Второй названный предел, и он не мой, а D-06.** «Язык без сегмента» —
это не язык сайта, а язык КАЖДОЙ документации по отдельности: источник
не несёт префикса, поэтому руководство на английском и документация,
написанная сразу по-русски, стоят под одним и тем же безъязыким адресом
`/doc/…`, и пункт селектора для него показывает тег первой библиотеки.
Сегодня на живом реестре все 48 источников `en`, так что расхождения
нет; в день, когда в реестр попадёт документация, написанная не
по-английски, честный ответ на «что показывать в этом пункте» придётся
выбрать вслух. Кандидат в наблюдения кампании, не правка этого атома.

### Решение 5 — где библиотеку находит `<head>`

`documentationHead` больше не читает одну библиотеку из модуля: он
находит её по координате адреса (`libraryAt`) и передаёт в `pageHead` /
`packageHead`. Это важнее, чем выглядит: `hreflang`, canonical, `og:*` и
JSON-LD страницы обязаны быть про **её** документацию, а не про первую в
сборке. Голову каталога считает `siteLanguages(BUILT)` — языки сайта.

В `PageView`/`PackageView` библиотека **не** кладётся, и это
принципиально: view едет пропсом в компонент, а пропсы каркас
сериализует в документ вторым экземпляром (аномалия А-4 отчёта P4-O4) —
библиотека реестра в каждой странице весила бы больше самой страницы.

### Решение 6 — куда пишет сборка (`VIBE_SITE_DIST`)

Одна сборка рендерит один набор библиотек — тот, что назван в
`VIBE_DOC_OUT`. Чтобы измерить второй набор, его надо построить **рядом**,
а не поверх: `tools/out-dir.mjs` читает `VIBE_SITE_DIST` (по умолчанию
`dist`), и его читают и драйвер, и `site/vite.config.ts`. Переменная
доезжает до Vite через окружение, которое он и так наследует.

Это цена одного нового рычага в контракте сборки за воспроизводимый
Playwright-прогон над двумя библиотеками, не трогающий `site/dist`, из
которого читают остальные 34 теста. У встраиваемой сборки такого рычага
нет: она производит один шаблон маршрута, а не библиотеку.

### Решение 7 — фикстуры второй библиотеки: выход конвейера, не голден

`site/src/fixtures/doc-build-pair/` и `doc-build-pair-ru/` — это то, что
`vibe doc build --format html|md|xml` написал над
`crates/vibe-doc/tests/fixture/translations/source` и `…/adaptation`
(источники названы в `site/src/fixtures/README.md`), слитое по одному
дереву на пакет. Ничего не редактировалось: карточные плейсхолдеры
приехали как есть, поэтому адрес в `og:image` — настоящий файл, за
которым линтер ссылок может сходить.

Выбор пакета не случаен: `com.example.docs/pair` ничего не адаптирует у
`fixture-manual`, а `pair-ru` объявляет `translates = com.example.docs/pair`
и обязан попасть именно к нему. Одна сборка над тремя деревьями даёт
сразу и «две библиотеки», и «адаптация приписана своему источнику», и
«источник без адаптаций».

## Дополнение оркестратора — фикстурные острова (`c1917fd3`)

Остров, который пакет держит фикстурой, — байтовая копия голдена
конвейера, и конвейер за это время сдвинул в нём две вещи (P4-O7,
`9da13611`): картинка цитируется **рядом с документом**, а не рядом со
страницей (`media/diagram.svg` → `../media/diagram.svg`), и неразрешённое
правило пишется в резолвер (`/doc/resolve/?uri=…`), а не в адрес, который
никому не отвечает.

Перекопировано байт в байт из
`crates/vibe-doc/tests/golden/guide-every-block.numbered.html`
(sha256 `c229304b…`) в оба места: `site/src/fixtures/island.html` и
`…/doc-build/com.example.docs/fixture-manual/0.1.0/guide/every-block/index.html`.
`.md` и `.xml` той же фикстуры с голденами **не расходились** — проверено
`diff`, изменений нет.

**Откуда голден, а не живая сборка.** Живой прогон
`vibe doc build --format html` над тем же фикстурным пакетом отличается
от голдена ещё двумя местами: блок `derived` он заполняет настоящим
выводом `vibe list --help` этой машины (голден несёт заглушку теста), а
правило разрешает иначе, потому что у прогона нет прогретого store.
Класть в фикстуру вывод CLI этой машины значило бы пинить фикстуру к
машине; `README.md` фикстур и так говорит, что при расхождении прав
голден. Источник назван, разбор здесь.

**Что после этого ушло и что осталось.**

- Ушла запись `note 1 citation(s) into documentation this site does not
  carry: /doc/com.example/subject/latest/common/PROP-001/` — правило
  теперь называет `/doc/resolve/`, который сборка пишет.
- **`L-01` осталась и срабатывает 4×.** Адрес картинки только переехал:
  `../media/diagram.svg` со страницы `…/0.1.0/guide/every-block/`
  разрешается в `…/0.1.0/guide/media/diagram.svg`, а конвейер публикует
  медиа пакета в корне дерева под контентным именем — файла нет ни там,
  ни там, и у фикстурного пакета его нет вовсе. Запись оставлена, текст
  причины переписан под новую форму; матчер (`endsWith("/media/diagram.svg")`)
  ловит обе. Снять её было бы правкой гейта под ожидание, а не под
  измерение.

## Тесты

### Node (`site/src/lib/library.test.ts`, 16 тестов; было 9)

Новые случаи: две документации — две библиотеки; адаптация приписана
источнику, которого называет, а не первому; каждая библиотека под своей
координатой, адаптация за координатой своего источника; один каталог на
язык, когда две библиотеки переведены в один; адаптация без источника —
библиотека из самой себя; остров одной библиотеки никогда не приезжает по
адресу другой; `viewOf` над многими отвечает из той библиотеки, которую
называет адрес, и `null` для координаты, которой в сборке нет; дверь
перечисляет оба издания в порядке «источник, потом адаптация».

Тест «refuses a set that is two documentations rather than one in two
languages» **снят**: он держал отказ, который был дефектом. На его месте
— «gives two documentations a library each».

### Node (`site/src/seo/media.test.ts`, 3 теста — хвост X-055)

Карточка берётся из `media.preview` манифеста (адрес рядом с пакетом, при
номере версии и никогда при `latest`); манифест старше поля проваливается
в поиск сборки; разбор карты окружения защитный.

### Playwright (`site/tests/libraries.spec.ts`, 6 тестов)

Спека строит **свой** сайт: те же три фикстурных дерева в `VIBE_DOC_OUT`,
выход в `dist-libraries` (не поверх `dist`, из которого читают остальные),
и тот же маленький статический сервер на своём порту. Меряет: дверь
перечисляет три издания в правильном порядке и ведёт каждую карточку на
её пакет; каждая документация отдаётся из своих байтов (включая русский
остров на русском адресе); навигация страницы — страницы её же
документации; адрес `…/pair-ru/…` отвечает 404; sitemap-индекс несёт по
части на (пакет, язык) и ровно одну часть каталогов; `/doc/llms.txt`
называет все три издания со стандингами.

## Вывод гейтов, дословно

### Сборка без окружения (фикстурная библиотека, паритет P4-O6)

```
$ node tools/build.mjs static
build (static): generated 18 page(s), expected 18
build (static): removed dist/q-manifest.json from the output
build (static): 8 island(s) from the documentation trees, 0 from the package's own fixture page
build (static): 24 file(s) copied from 1 documentation tree(s) for 2 edition(s) of 1 library (2 page(s) in a language that does not carry them); 13 written — catalogue, manifests, 3 sitemap part(s) over 7 address(es), resolver
build (static): root files robots.txt, llms.txt, llms-full.txt, sitemap.xml, feed.xml, og.png; 8 font file(s) at /fonts/, 3 page(s) repointed at the bundled faces, 17 crawler name(s) from 9 provider page(s)
build (static): csp.txt — 47 inline script hash(es) over 119 occurrence(s), no external source
documentation links — every address against the files behind it
    ok        19 page(s), 234 file(s) in the output
    ok        646 link(s) followed, 18 hreflang pair(s) checked
    ok        9 page(s) name another page canonical and are read as it
    ok        12x github.com — the canonical source repository, linked by the landing
    ok        11x gitverse.ru — the source mirror, linked by the landing
    L-01      4x — The island golden cites `media/diagram.svg` beside the DOCUMENT, …
links: green — 646 checked, 0 broken.
build (static): ok
```

646 ссылок и 8 островов — числа P4-O6 без изменения. Файлов в выходе
**234 против 233**: один новый чанк — компонент двери теперь импортирует
`lib/view.ts` (список карточек уехал туда из JSX). Проверено чистой
пересборкой после `rm -rf site/dist site/server`.

### Две библиотеки в одной сборке

```
$ VIBE_SITE_DIST=dist-libraries VIBE_DOC_OUT="<fixtures>/doc-build;<fixtures>/doc-build-pair;<fixtures>/doc-build-pair-ru" node tools/build.mjs static
build (static): generated 24 page(s), expected 24
build (static): 12 island(s) from the documentation trees, 0 from the package's own fixture page
build (static): 56 file(s) copied from 3 documentation tree(s) for 3 edition(s) of 2 libraries (0 page(s) in a language that does not carry them); 16 written — catalogue, manifests, 4 sitemap part(s) over 11 address(es), resolver
build (static): csp.txt — 60 inline script hash(es) over 159 occurrence(s), no external source
    ok        25 page(s), 275 file(s) in the output
    ok        796 link(s) followed, 23 hreflang pair(s) checked
    ok        11 page(s) name another page canonical and are read as it
    L-01      2x — …
links: green — 796 checked, 0 broken.
build (static): ok
```

**24 = 1 дверь + 1 каталог `ru` + 2×(1+2) руководства + 2×2×(1+2) пары +
4 маршрута лендинга.** 12 островов = 2 страницы × 2 написания версии × 3
издания.

### Встраиваемая сборка

```
$ node tools/build.mjs embedded
build (embedded): generated 14 page(s), expected 14
build (embedded): removed dist-embedded/q-manifest.json from the output
build (embedded): 14 page(s) carry none of the 7 public-only tags
build (embedded): ok
EMBEDDED_EXIT=0
```

### Пол дисциплины и аудит контраста

```
$ node tools/floor.mjs
=== prettier --check (floor perimeter: design/src, site/src) ===
=== tsc --noEmit ===
=== tests (node --test) ===
ℹ tests 48
ℹ pass 48
ℹ fail 0
=== eslint (floor perimeter: design/src, site/src) ===
=== typescript-ai-native-conform check ===
typescript-ai-native-conform check: 0 finding(s) in scope <workspace> ({}), 0 frozen in baseline, 0 new
=== typescript-ai-native-specmap --check ===
typescript-ai-native-specmap --check: clean (0 spec units, 94 tagged code items, 94 edges, 0 suspects, 94 warnings).
typescript-ai-native-specmap: ratchet gate — 0 orphan(s) (0 root(s) exempt).
=== test-gate (xfail-strict) ===
test-gate: 54 results parsed (0 failed, 0 skipped), baseline entries: 0
test-gate: green (xfail-strict).
floor: all green (7 step(s) run, 0 disabled by policy).

$ node design/audit/contrast.mjs
=== pairs: gated=40, reference=6; below threshold=0 ===
@media(prefers-color-scheme:dark) agrees with [data-theme="dark"]: OK (0 divergences)
tokens.css writes no colour of its own: OK (every value is a var() into palette.css)
```

Тестов было 38, стало 48: семь новых в `library.test.ts` и три в
`media.test.ts`. `specmap.json` перегенерирован один раз (93 → 94 юнита),
в коммите хвоста.

### Playwright

```
$ pnpm test:e2e
Running 40 tests using 1 worker
…
  2 skipped
  38 passed (46.6s)
```

Было 34 из 34; стало 40, из них шесть новых (`libraries.spec.ts`) и два
пропуска — те самые два теста локального ридера, которые пропускаются,
когда в `vibe` запасная оболочка. Бинарник дерева сейчас собран **без**
фичи `embedded-shell`, как и должно быть, поэтому пропуск ожидаемый и
печатает свою причину.

### Паритет лендинга

```
$ node tools/parity.mjs <reference-dist>
parity: green — 31 deliberate difference(s), 0 unexplained.
```

Эталон не пересобирался (R-28): взят закоммиченный `dist/` соседнего
дерева, скопирован в scratch вместе с его `scripts/`, и над копией
выполнен её же постбилд `build-llms-full.mjs` — без него `/llms-full.txt`
в эталоне отсутствует и даёт ровно одно ложное отличие.

## Живой прогон — весь реестр и хост

Конфигурация — scratch-копия `site.example.toml`, у которой изменено одно
значение: `checkout` указывает на этот worktree (отступление 6). Store
билдера уведён из `~/.vibe` через `$VIBE_SETTINGS` в scratch; сеть — только
чтение индекса реестра. Прогон целиком, **вместе с web-половиной** (без
`--no-web`) — то, чего P5-O1 сдать не мог.

```
$ vibe doc build-site --config <scratch>/site.toml --out <scratch>/site-out
site: <scratch>/site.toml
  registry vibespecs https://github.com/vibespecs (index <url>/index, naming fqdn)
  host https://github.com/vibevm/vibevm @main from <checkout> (debounce 60 min)
  mounted at /doc/ on https://vibevm.org, theme system
  analytics none — no website id, so no tag is rendered
  registry vibespecs at https://raw.githubusercontent.com/vibespecs/index/main
            — 46 package(s), 46 version(s), generated 2026-09-11 12:35:53 UTC
  host https://github.com/vibevm/vibevm @main from <checkout>
            — the project and 1 documentation package(s) in tree
queue: 0 to render, 48 unchanged, 0 gone
render: 0 version(s) written, 0 refused, 48 standing
addresses: 48 alias(es), 48 package(s), 1 language(s), 1232 page address(es)
  site   144 tree(s) handed to <checkout>/vibevm/vibepacks/org.vibevm.doc/web/v0.1.0

SSG results
- Generated: 1237 pages
- Duration: 2.3 s

build (static): generated 1237 page(s), expected 1237
build (static): removed dist/q-manifest.json from the output
build (static): 1136 island(s) from the documentation trees, 0 from the package's own fixture page
build (static): 2944 file(s) copied from 144 documentation tree(s) for 48 edition(s) of 48 libraries (0 page(s) in a language that does not carry them); 151 written — catalogue, manifests, 49 sitemap part(s) over 617 address(es), resolver
build (static): root files robots.txt, llms.txt, llms-full.txt, sitemap.xml, feed.xml, og.png; 8 font file(s) at /fonts/, 3 page(s) repointed at the bundled faces, 17 crawler name(s) from 9 provider page(s)
build (static): csp.txt — 2485 inline script hash(es) over 8560 occurrence(s), no external source
documentation links — every address against the files behind it
    ok        1238 page(s), 4511 file(s) in the output
    ok        127568 link(s) followed, 621 hreflang pair(s) checked
    ok        618 page(s) name another page canonical and are read as it
    ok        330x github.com — the canonical source repository, linked by the landing
    ok        11x gitverse.ru — the source mirror, linked by the landing
    ok        954 outbound link(s) to 83 host(s) the prose cites, fetched by nothing: arxiv.org (78), www.conventionalcommits.org (54), doc.rust-lang.org (40), raw.githubusercontent.com (40), maven.apache.org (39)
    note      490 link(s) the pipeline wrote inside an island in a form this site does not carry: descriptor.md (44), VIBEVM-SPEC.md (34), layer.md (28), manifest.md (26), media-types.md (26)
    note      2 citation(s) into documentation this site does not carry:
              /doc/org.vibevm.fractality/plans/postponed/PP-002-def-c2-2b-worker-credibility/
              /doc/org.vibevm.fractality/reports/
links: green — 127568 checked, 0 broken.
build (static): ok
  output <scratch>/site-out
EXIT=0
```

**Числа, которые просил пакет:**

| Что | Значение |
|---|---|
| библиотек | **48** |
| изданий | 48 (адаптаций в реестре нет ни одной) |
| страниц | **1237 из 1237 ожидаемых** |
| островов из деревьев конвейера | **1136**, из фикстуры — **0** |
| ссылок проверено | **127 568**, сломанных — **0** |
| «in a form this site does not carry» | **490** (`descriptor.md` 44, `VIBEVM-SPEC.md` 34, `layer.md` 28, `manifest.md` 26, `media-types.md` 26) |
| частей sitemap | 49 (48 по (пакет, язык) + одна каталогов) над 617 адресами |
| `du -sh` выдачи | **212 МБ**, 4511 файлов; вместе с деревьями билдера в `.vibe-site` — 315 МБ, 7988 файлов |

Второй прогон (ничего не менялось) дал те же байты и те же числа:
`queue: 0 to render, 48 unchanged`, `generated 1237 page(s), expected 1237`,
`links: green — 127568 checked, 0 broken`.

**1237 = 1 дверь + 0 каталогов языков + 1232 адреса страниц и пакетов + 4
маршрута лендинга**, и 1232 — это то самое число, которое билдер печатает
своей строкой `addresses:`. Две половины, посчитавшие одно и то же
независимо, сошлись на живом реестре.

### Паритет с одной библиотекой (руководство)

Пакет просил назвать числа над руководством. Сборка над тремя его
деревьями из выхода билдера:

```
$ VIBE_DOC_OUT="<trees>/org.vibevm.core.vibevm-docs@0.1.0/{html;md;xml}" node tools/build.mjs static
build (static): generated 107 page(s), expected 107
build (static): 100 island(s) from the documentation trees, 0 from the package's own fixture page
build (static): 214 file(s) copied from 3 documentation tree(s) for 1 edition(s) of 1 library (0 page(s) in a language that does not carry them); 10 written — catalogue, manifests, 2 sitemap part(s) over 52 address(es), resolver
    ok        108 page(s), 510 file(s) in the output
    ok        13725 link(s) followed, 56 hreflang pair(s) checked
    ok        53 page(s) name another page canonical and are read as it
    note      2 link(s) the pipeline wrote inside an island in a form this site does not carry: AUTHORING.md (2)
links: green — 13725 checked, 0 broken.
```

**107, а не 103, и 13 725, а не 13 187 — и ни то ни другое не моя
правка.** Оба числа — ровно те, что измерил P5-O1 в конце своей сессии:
руководство в деревьях билдера несёт **50** страниц (48 авторских плюс
манифест и README уровня 0), а не 48, так что
`2 × (50 + 1) + 1 дверь + 4 маршрута лендинга = 107`. После P4-O7 число
ссылок не сдвинулось: 13 725 и две записи `AUTHORING.md`, как и было.

## Аномалии

**А-1. Гейт числа страниц краснел от ЦВЕТА, а не от числа.** Первый
прогон новой Playwright-спеки дал `the generator reported : 0` при
двадцати четырёх реально сгенерированных страницах. Причина: Playwright
ставит детям `FORCE_COLOR`, Vite начинает писать escape-коды, и строка
приезжает как `- Generated: \e[2m24 pages\e[22m` — регулярное выражение
гейта (`/- Generated:\s+(\d+)\s+page/`) на ней не сходится. Лечение в
`tools/build.mjs`: цвет снимается перед разбором. Это дефект гейта, а не
спеки: любой CI, выставляющий `FORCE_COLOR`, получал бы красную сборку с
причиной, которой нет рядом.

**А-2. Двойная шапка лендинга (P4-O3, А-2) жива и ловится числом.**
Наблюдалась **пять раз** за сессию: в `dist/index.html` два
`<header class="docs-header">`, и линтер печатает **658** проверенных
ссылок вместо 646 (ровно +12 = 4 ссылки × 3 лендинговых страницы, как и
записал P4-O3). Не мой периметр, не чинил; все числа в этом отчёте сняты
с прогонов, где шапка одна, и это проверено `grep` по выходу.

Что добавилось к разбору P4-O3 — распределение. Каждое срабатывание
приходилось на **первую сборку после того, как менялся вход**: правка
исходника, обновление фикстурного острова, хвост, и переключение набора
библиотек с реестровых 48 обратно на фикстуру. Следующая сборка того же
входа давала одну шапку — кроме одного случая (после реестрового
прогона удвоение повторилось дважды подряд и снялось только после
`rm -rf site/dist site/server`). Проверенная гипотеза, которая **не**
подтвердилась: «сборка поверх существующего `dist`» — чистая пересборка
и сборка поверх дали по одной шапке подряд. Практический вывод для
приёмки прежний и теперь с числом: расхождение `link(s) followed`
646 → 658 — датчик, пересборка его снимает.

**А-3. Playwright оставляет свой `webServer` жить.** Дважды после
прогона на 4173 оставался `node serve.mjs 4173`, и следующий прогон падал
с «already used». Убивал только процесс, чью командную строку проверил
(`serve.mjs 4173`, старт внутри моего прогона) — чужих не трогал.
Наблюдение для панели: если шаг e2e когда-нибудь встанет в
`self-check.sh`, ему нужен `reuseExistingServer` или уборка порта.

**А-4. Наблюдение: 490 ссылок уровня 0 ведут в форму, которой сайт не
несёт.** На всём реестре линтер печатает
`note 490 link(s) the pipeline wrote inside an island in a form this site
does not carry: descriptor.md (44), VIBEVM-SPEC.md (34), layer.md (28),
manifest.md (26), media-types.md (26)`. Это не ссылки сайта и не мой
периметр: страницы уровня 0 композируются из README и спек пакета, а те
ссылаются друг на друга относительными именами файлов (`descriptor.md`),
которые сайт отдаёт как каталоги. Ровно тот датчик, который P5-O1 назвал
в А-8, только теперь над 48 пакетами, а не над одним. Кандидат в задачу
конвейера: перевод README уровня 0 мог бы разрешать соседние `*.md` в
адреса сайта так же, как P4-O7 научил это делать островам руководства.
Рядом стоит `note 2 citation(s) into documentation this site does not
carry` — две цитаты в страницы `org.vibevm.fractality`, которых в рендере
нет.

## Отступления от текста пакета

1. **Хвост сдан вторым коммитом**, как пакет и разрешал («тем же
   коммитом или вторым, назови»).
2. **Дополнение оркестратора сдано отдельным коммитом** `c1917fd3`
   (`test(web): refresh the fixture islands from the pipeline`) — тем
   subject'ом, который он назвал.
3. **`L-01` не снята** — разбор выше: она срабатывает, адрес картинки
   только переехал.
4. **Острова взяты из голдена, а не из живой сборки** — разбор выше
   (живая сборка несёт вывод CLI этой машины в блоке `derived`).
5. **Введена переменная `VIBE_SITE_DIST`** — решение 6; её не просили, но
   без неё Playwright-прогон над двумя библиотеками пришлось бы строить
   поверх `dist`, из которого в тот же момент читают остальные тесты.
6. **Живой прогон выполнен с scratch-копией `site.example.toml`**, у
   которой изменено одно значение — `checkout` указывает на этот
   worktree. У примера `checkout` относительный и называет каталог,
   который заводит выкладка; с ним билдер отказывается до сети.
7. **Селектор языка вне страницы** отвечает числом документаций вместо
   издателя, когда за языком их несколько — решение 4, названный предел.

## Что не сделано и почему

1. **Картинки карточек на полках по-прежнему не показываются** —
   показывается нарисованный плейсхолдер. Хвост X-055 переводит на
   манифест `og:image` (и вместе с ним `twitter:image` и `image` в
   JSON-LD), то есть карточку ССЫЛКИ; иконки и баннеры на полках — это
   решение 10 отчёта P4-O6, и оно упирается в маршрут локального ридера,
   а не в манифест.
2. **`tools/parity.mjs` и лендинг не трогались** — паритет зелёный без
   новых правил.
3. **Rust не трогался вовсе**: `crates/**`, `xtask/**`, `schemas/**` не
   менялись, `cargo` не запускался, `target/debug/vibe.exe` взят как есть
   (собран в дереве в 20:07, уже с правками P4-O7).
4. **Всю панель `tools/self-check.sh` не гонял** — по постоянному
   указанию оркестратора; шагов в неё не добавлял.
5. **Ничего на сервере, никакого деплоя, никакой публикации** (R-24).
   PROP-файлы, вижен и страницы руководства не менялись. `git push` не
   делался.

## R-25, диск, процессы

Путей вне репозитория в отчёте нет: scratch обозначен `<scratch>`,
чекаут — `<checkout>`, эталон паритета — `<reference-dist>`, деревья
живого прогона — `<trees>`, фикстурные деревья — относительными именами.
Секретов, токенов, `secrets.local.md` и `infra/` не читал и не касался.
Сеть — только чтение индекса реестра для живого прогона; состояние
билдера уведено из `~/.vibe` через `$VIBE_SETTINGS` в scratch.

Диск: `site/dist` оставлен в состоянии сборки по умолчанию (2,7 МБ, 234
файла, одна шапка на лендинге, 646 ссылок); `site/dist-libraries` и его
серверный каталог удалены. След в scratch — выход живого прогона (315 МБ
вместе с деревьями билдера), store билдера (30 МБ), шесть фикстурных
деревьев и копия эталона паритета — снят по завершении, осталась только
конфигурация прогона (8 КБ). Свободно на диске: **213 ГБ из 3,7 ТБ** в
начале и в конце.

Процессы: свои остановлены. Про два случая с `serve.mjs` — А-3; чужих не
трогал и не останавливал. Rust не пересобирал; `target/debug/vibe.exe`
взят как есть.

## `git status --short` на момент сдачи (мои пути)

```
$ git status --porcelain -- vibevm/vibepacks/org.vibevm.doc/web/v0.1.0
(пусто)
```

Каждый коммит сделан формой `git commit -m … -- <пути>` (новые файлы
предварительно добавлены `git add` по тем же путям); `git show
--name-status` проверен по каждому — путей вне
`vibevm/vibepacks/org.vibevm.doc/web/v0.1.0/**` нет. `git push` не
делался.
