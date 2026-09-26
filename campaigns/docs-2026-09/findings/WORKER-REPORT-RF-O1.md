# Отчёт RF-O1 — свёрнутые цитаты правил: остров и читалка — 2026-09-26

Пакет: `campaigns/docs-2026-09/findings/PACKET-RF-O1.md`. Дерево
`C:\Users\olegc\git\v\vibevm`, ветка `main`, HEAD `c500dcba0`. Git — только
читающий: ничего не добавлено в индекс, не закоммичено, не спрятано, ничего
не отправлено.

Сделано всё, что перечислено в пакете. Отклонения — одна строка вне
объявленного периметра (§9.1) и одно расширение тестовой фикстуры (§9.2);
оба описаны ниже и оба нужны, чтобы требования пакета вообще были
выполнимы.

## 1. Файлы

### Новые

| Файл | Что |
|---|---|
| `crates/vibe-doc/src/html/rule.rs` | блок `rule` целиком: раскрывающийся остров + чистая функция описания |
| `crates/vibe-doc/src/html/rule/tests.rs` | 12 юнит-тестов описания, по ветке на тест |
| `vibevm/vibepacks/org.vibevm.doc/web/v1.0.0/site/src/reader/print-rules.ts` | печать: раскрыть закрытые цитаты на `beforeprint`, вернуть на `afterprint` |
| `vibevm/vibepacks/org.vibevm.doc/web/v1.0.0/site/tests/folded-rules.spec.ts` | 11 e2e-тестов сворачивания |

### Изменены

| Файл | Что |
|---|---|
| `crates/vibe-doc/src/html.rs` | `mod rule;`, вызов `rule::fold`, старая `fn rule` удалена (−51 строка → файл 548 из 600 по бюджету) |
| `crates/vibe-doc/src/content.rs` | поле `Content.lang`, `with_lang`, `edition_lang()` с доктестом |
| `crates/vibe-doc/src/build.rs` | `build::content` читает язык издания из манифеста один раз за сборку |
| `crates/vibe-doc/src/agent.rs` | язык издания = язык найденного экземпляра (`located.lang`) |
| `crates/vibe-doc/src/html/tests.rs` | 3 новых теста острова, 2 существующих усилены, хелпер `one_rule` |
| `crates/vibe-doc/tests/island.rs`, `tests/projections.rs` | текст цитаты в тестовом наборе длиннее 10 слов (§9.2) |
| `crates/vibe-doc/tests/golden/guide-every-block{,.numbered}.html` | перевыпущены (`VIBE_DOC_BLESS=1`) |
| `crates/vibe-doc/tests/golden/guide-every-block.md` | перевыпущен — одна строка, текст цитаты из §9.2 |
| `crates/vibe-doc-server/src/routes.rs` | одна строка: язык издания для локальной читалки (§9.1) |
| `…/web/v1.0.0/design/src/components/prose/styles.css` | стили свёрнутой строки; селектор якоря `[data-p] > .p-anchor` → по потомку |
| `…/web/v1.0.0/site/src/reader/mount.ts` | подключение `startPrintedRules()` |
| `…/web/v1.0.0/site/tests/reader.spec.ts` | тест панели правила сначала раскрывает цитату |
| `…/web/v1.0.0/site/src/fixtures/island.html` | байт-копия перевыпущенного golden |
| `…/fixtures/doc-build/…/guide/every-block/index.html` | то же (по таблице `fixtures/README.md`) |
| `…/fixtures/doc-build/…/guide/every-block.md` | байт-копия `guide-every-block.md` |
| `…/fixtures/doc-build/llms-full.txt` | одна строка `> [p11] …` — фикстура «в форме, которую пишет сборка» |
| `…/web/v1.0.0/specmap.json` | перегенерирован `typescript-ai-native specmap --path .` |

Не тронуто: `md.rs`, `xml.rs`, `llms.rs`, спеки, страницы руководств,
`BACKLOG.md`, корневой `specmap.json`, базовые линии храповиков,
`design/print.css` (вне периметра — см. §8, пункт 3).

Две страницы руководств (`howto/read-documentation-locally.xml` в
`vibevm-docs` и `vibevm-docs-ru`) в рабочем дереве изменены **не мной** —
это раздел `quotations`, добавленный центральной сессией; см. §7.1.

## 2. Разметка острова

`fn rule::fold`. Атрибуты блока (`data-when`, `data-p`, `data-unresolved`)
переехали с `blockquote` на `details`; `blockquote` сохранил единственный
класс `rule`, а ссылка внутри — прежние `class`, `href`, `data-uri`, `lang`
и прежнее тело.

С описанием (из golden, `data-p` появляется в нумерованной проекции):

```html
<details data-p="11" class="rule-fold">
  <summary class="rule-fold__line">
    <a class="p-anchor" id="p11" href="#p11">11</a>
    <span class="rule-fold__mark" aria-hidden="true">▶</span>
    <span class="rule-fold__kind">spec:</span>
    <span class="rule-fold__gist" lang="en">A package MUST declare its kind…</span>
  </summary>
  <blockquote class="rule">
    <a class="rule" href="/doc/resolve/?uri=…%23A-RULE" data-uri="…#A-RULE" lang="en">A package <strong>MUST</strong> declare its <code>kind</code>, …</a>
  </blockquote>
</details>
```

Без описания — строка без `spec:`, общая подпись на языке **издания**:

```html
<details data-p="12" class="rule-fold" data-unresolved="true">
  <summary class="rule-fold__line">
    <a class="p-anchor" id="p12" href="#p12">12</a>
    <span class="rule-fold__mark" aria-hidden="true">▶</span>
    <span class="rule-fold__gist rule-fold__gist--generic" lang="en">quote from the specification</span>
  </summary>
  <blockquote class="rule">
    <a class="rule" href="…" data-uri="…#UNRESOLVED">spec://…#UNRESOLVED</a>
  </blockquote>
</details>
```

Инварианты, которые я проверял отдельно:

- `open` не выставляется никогда — страница приходит свёрнутой до любого
  скрипта и остаётся свёрнутой у читателя без JavaScript;
- `data-unresolved="true"` выводится ровно один раз на цитату, поэтому
  перепись пропусков (`build::gaps`, тест `the_gap_census_counts_the_blocks_the_island_marks`)
  считает столько же, сколько раньше;
- `lang` на `rule-fold__gist` — язык спецификации (тот же, что на `a.rule`),
  у общей подписи — язык издания;
- полный текст правила остаётся в острове (`.md`/`.xml`/`llms`/MCP не
  затронуты — см. §7.1).

## 3. Функция описания и её ветки

`crates/vibe-doc/src/html/rule.rs`, `fn gist(text: &str) -> Option<String>`.
Чистая функция от текста правила; ничего не читает и не кэширует.

Разметка снимается **через тот же грамматический разбор, что и остров**:
`inline::render` даёт HTML, `plain()` снимает теги и возвращает три
экранирования (`&amp;`, `&lt;`, `&gt;`), которые пишет `inline::escape`.
Второй разбор Markdown разошёлся бы с цитатой двумя строками ниже на той же
странице — первое же правило, цитирующее обратную кавычку, это показало бы.
Побочная выгода: «жирный лид» — это буквально ведущий `<strong>` в том
рендере, который увидит читатель.

| Ветка | Правило | Константа |
|---|---|---|
| правило ≤ 10 слов | описания нет → общая подпись | `SHORT_RULE = 10` |
| текст не разрешён | описания нет → общая подпись | — |
| жирный лид 2–8 слов | лид как есть, без `…`; хвостовые `.`/`:` сняты | `LEAD_WORDS = 8` |
| лид 1 слово («Decision», «Why») | `Лид: ` + первые слова остатка | `AFTER_LEAD = 5` |
| лид > 8 слов | первые 6 слов лида + `…` | `LEAD_CUT = 6` |
| жирный ран только из пунктуации | лидом не считается → первые слова остатка | — |
| нет лида | первые 6 слов текста | `FIRST_WORDS = 6` |

Обрезка: берутся первые N слов; пока последнее слово служебное — оно
отбрасывается (список из 25 слов пакета, сравнение без регистра и без
обрамляющей пунктуации, поэтому «the,» тоже служебное); если обрезали —
добавляется `…`. Если после отбрасывания не осталось ни слова, описания
нет. Примеры из тестов: `Offline resolution is therefore computed against…`
(6 слов), `A build writes every artefact…` («of» отброшено),
`The block number rides…` («at the» отброшены двумя подряд).

Общая подпись: `ru` → `цитата из спецификации`, иначе
`quote from the specification`.

Юнит-тесты (`html/rule/tests.rs`, 12): жирный лид; пунктуация лида не
переезжает; однословный лид + остаток; длинный лид; лид из пунктуации;
без лида; служебные слова (один и два подряд); запятая перед `…` снимается;
ровно 10 слов → нет описания, 11 → есть; короткое правило с лидом → нет
описания; inline-разметка становится текстом; `<script type="…">` из
код-спана доходит до строки как символы; общая подпись на трёх языках.

Тесты острова (`html/tests.rs`, 5): раскрывающийся блок со всей разметкой
и «закрыт, но текст в странице»; данные блока и номер на `details`/в
`summary`; общая подпись на русском издании при английском правиле;
неразрешённая цитата (метка на `details`, ровно одна, общая подпись);
плюс прежние два теста цитаты, усиленные.

### Одно решение, которого пакет не называл

При обрезке с конца снимаются `,`, `;`, `:` перед `…` — иначе выходит
«A package MUST declare its kind,…». Это косметика и чистая функция;
норма («первые слова … с многоточием») ей не противоречит. Если ревью
сочтёт иначе — снимается тремя строками в `first_words` и одним тестом.

## 4. Как проведён язык издания

Поле `Content.lang` рядом с `Content.base`: язык — свойство **рендера**,
как и база, а не документа (один пакет — один язык, `##LOC-LANGUAGE-FIELD`,
поэтому на `Page` он не поехал: `manifest::language` прямо говорит, что это
свойство пакета и никогда страницы). Пустая строка = проектная
умолчальность; `Content::edition_lang()` — единственное место, где она
решается, и у него доктест.

Заполняется в трёх композиционных корнях:

- `build::content` — `manifest::language(package_dir).unwrap_or_default()`,
  один раз за сборку. Нечитаемый манифест не валит сборку: тот же файл
  читается и отвергается по имени там, где манифест — предмет
  (`manifest::build`), а рендерер, упавший здесь, сообщил бы о том дефекте
  дважды, а о дефектах страниц — ни разу (это уже записанная позиция
  `content.rs`);
- `agent::fetch` — `located.lang`, язык того экземпляра, который ответил
  на адрес: второе чтение манифеста могло бы только разойтись с ним;
- `vibe-doc-server` (локальная читалка) — из пакета, на котором открыт
  reader (§9.1).

`.md`, `.xml` и `llms` о языке издания не узнали: поле читает только
остров, и только для одной строки, которую он пишет своим голосом, а не
цитируя.

## 5. Стили и печать

`design/src/components/prose/styles.css`, только токены, ни одного
литерала цвета:

- `.rule-fold__line` — `display:flex`, `gap`, одна строка, кегль `0.86em`,
  цвет `--text-3`, левая граница `3px solid var(--accent)` как у
  `blockquote.rule`, фон `--bg-raise`, радиус `0 var(--radius-md) var(--radius-md) 0`,
  `cursor:pointer`, `list-style:none` + `::-webkit-details-marker{display:none}`
  (родной маркер убран — ▶ уже в острове);
- `.rule-fold__kind` — `--font-mono`, цвет `--accent`;
- `.rule-fold__gist` — `min-width:0; overflow:hidden; white-space:nowrap;
  text-overflow:ellipsis` (обрезка браузером в одну строку);
- `.rule-fold__mark` — `transition: transform var(--speed) ease`, в
  `[open]` — `rotate(90deg)`; под `prefers-reduced-motion: reduce` —
  `transition: none` (базовый лист отнимает только длительность, то есть
  превращает движение в прыжок, поэтому свойство отказывается здесь);
- `:hover` (строка и маркер) и `:focus-visible` — кольцо `2px solid
  var(--accent)` с отступом, на самой строке: её открывает клавиатура;
- раскрытая цитата — прежний `blockquote.rule` под строкой: `margin:0`,
  радиус `0 0 var(--radius-md) 0`, а у открытой строки радиус
  `0 var(--radius-md) 0 0`, так что рамка одна и без выреза;
- `[data-unresolved="true"] > .rule-fold__line` и `> blockquote.rule` —
  пунктирная левая граница и `--text-3` (раньше это ловил общий селектор
  по блоку, у которого теперь нет границы);
- `@media (max-width: 900px)`: `.rule-fold__line .p-anchor{flex:none}` —
  на телефоне номер встаёт в начало строки (у остальных блоков он там же
  переезжает над блоком, но здесь строка обязана остаться одной строкой).

Селектор якоря: `.prose [data-p]:has(> .p-anchor:target)` →
`.prose [data-p]:has(.p-anchor:target)`. Якорь свёрнутого правила лежит
внутри `summary`, как у списка внутри первого `li`, а у таблицы — внутри
`caption`; блоки не вкладываются, поэтому «блок, держащий этот якорь» имеет
один ответ и по потомку.

Печать — `site/src/reader/print-rules.ts`, идиома соседей (start → вернуть
teardown, слушатели снимаются): на `beforeprint` собираются
`details.rule-fold:not([open])`, им ставится `open`; на `afterprint` `open`
снимается **только у тех, которые раскрыл модуль** — цитата, раскрытая
читателем руками, остаётся раскрытой. Список живёт в замыкании, не рядом
с модулем. Подключён в `mount.ts` после `startRuleTransclusion()`. Тег
`@scope spec://org.vibevm.core/vibevm/common/PROP-057#READER-RULE-FOLDED`;
`specmap.json` пакета перегенерирован, ратчет чистый (0 сирот).

Панель правила (`rules.ts`) и «Rules this page cites» (`cited-rules.ts`) на
новой структуре работают без правок: `quote.closest("blockquote")` находит
`blockquote` внутри `details`, `quote.closest("[data-unresolved]")` —
`details`, а `a.rule[data-uri]` в списке цитат остаётся тем же селектором
(e2e на оба — в §8). Клик по номеру внутри `summary` не переключает
раскрытие, потому что `anchors.ts` вызывает `preventDefault()`, а
переключение `details` — это действие по умолчанию того же клика; на это
есть отдельный e2e.

Глазами посмотрено (Playwright-скриншоты по рецепту из памяти, светлая и
тёмная темы, 1200 и 390): свёрнутая строка — одна строка с номером на
полях, `spec:` акцентом и обрезанным описанием; неразрешённая — пунктирная
граница и общая подпись; раскрытая — та же цитата под строкой, одна рамка,
маркер повёрнут; на 390 номер в начале строки, сдвига по горизонтали нет.

## 6. Числа по руководству

Обе редакции собраны в HTML текущим бинарём
(`vibe doc build --format html`), 49 страниц каждая.

| Редакция | Свёрнутых цитат (`class="rule-fold"`) | С описанием (`rule-fold__kind`) | С общей подписью (`--generic`) |
|---|---|---|---|
| EN (`vibevm-docs`) | 803 | 751 | 52 |
| RU (`vibevm-docs-ru`) | 803 | 751 | 52 |

Общая подпись: в EN 52× `lang="en">quote from the specification`, в RU 52×
`lang="ru">цитата из спецификации` — то есть язык издания проведён до
страницы, а не взят из спецификации.

803, а не 802 из нормы: центральная сессия добавила в руководство раздел
`quotations` с ещё одной цитатой (`PROP-057#READER-RULE-FOLDED`) —
см. §7.1. Описания в обеих редакциях английские, потому что описание — это
слова самой спецификации, а спецификации написаны по-английски; по-русски
в RU-издании говорит только общая подпись. Это ровно то, что предписывает
норма (`lang` описания = язык спецификации).

640 различных описаний на 751 цитату с описанием: одно и то же правило
цитируется с разных страниц.

15 случайных описаний (EN, `random.seed(57)` по отсортированному списку
различных):

```
A documentation package MAY declare…
Resolved — shipped as proposed
vibe [clean] <phase> [<pkgref>…] [flags]
vibe mcp status reports each declared…
6.2 vibe.toml is the most expensive…
Freshness is judged per contribution, not…
[package].authors names only the people…
REQ {#provenance-edit}. From the provenance view…
Tree-shaking default
One trust model, two verbs: registration…
The owner's hard constraint: installing…
Short or bare names survive only…
8.1 The ABI is C +…
Decision: Registry settings may also live…
a compromised mirror cannot silently substitute…
```

Что тут видно честно: описание — это слова правила, поэтому правило,
которое начинается с командной строки, описывается командной строкой
(`vibe [clean] <phase> …`), правило с номером раздела в лиде — номером
(`6.2 …`, `8.1 …`), а правило без лида — своими первыми словами, иногда со
строчной буквы (`a compromised mirror …`) или с якорем в тексте
(`REQ {#provenance-edit}. …`). Все три случая — «слова самой
спецификации», как решено в `##RULE-FOLDED-DECISION`; если владелец
захочет их причесать, норма уже называет ход: необязательный атрибут
`summary` на `rule` (`rule-folded-revisit`).

Пример новой страницы руководства (та, которую добавила центральная
сессия) в острове: `▶ spec: Cited rules are folded` — то есть прозе,
которая это обещает, остров отвечает.

## 7. Самопроверка — дословно

### 7.1 Снимок `.md` «до»

Снят **после** старта, но **до** первой моей правки, и тем бинарём, который
собран из неизменённого HEAD (`cargo build -p vibe-cli`, потом
`vibe doc build --format md`), поэтому раздел `quotations`, добавленный
центральной сессией, уже внутри обоих снимков, и сравнение чистое:
разницы по страницам нет вовсе. Дополнительно снята RU-редакция.

```
$ diff -r <scratch>/md-before <scratch>/md-after
diff -r <scratch>/md-before/manifest.json <scratch>/md-after/manifest.json
33c33
<     "rendered_at": "2026-09-26T05:51:19.762097Z"
---
>     "rendered_at": "2026-09-26T06:12:04.492550800Z"
MD-DIFF EXIT=1

$ diff -r <scratch>/md-before-ru <scratch>/md-after-ru
diff -r <scratch>/md-before-ru/manifest.json <scratch>/md-after-ru/manifest.json
38c38
<     "rendered_at": "2026-09-26T05:51:30.124514700Z"
---
>     "rendered_at": "2026-09-26T06:12:22.317267400Z"
MD-DIFF-RU EXIT=1

$ diff -x manifest.json -r <scratch>/md-before <scratch>/md-after
MD-DIFF-NO-MANIFEST EXIT=0
$ diff -x manifest.json -r <scratch>/md-before-ru <scratch>/md-after-ru
MD-DIFF-RU-NO-MANIFEST EXIT=0
```

Единственная разница на все 57 файлов каждой редакции — `rendered_at` в
`manifest.json`, то есть показания часов: сборка вызывает часы, а не
рендерер. 49 страниц `.md`, `llms.txt`, `llms-full.txt`, `llms-small.txt`,
`llms-medium.txt` и три картинки-заглушки — байт в байт. `.xml` не
затронут ни кодом, ни golden-файлом (`guide-every-block.xml` не изменился
даже при изменившемся тексте правила — проекция держит адрес, а не текст).

### 7.2 Rust

```
$ cargo fmt --all -- --check
EXIT=0

$ cargo test -p vibe-doc
running 605 tests
test result: ok. 605 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.08s
running 5 tests
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
running 4 tests
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
running 5 tests
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
running 78 tests
test result: ok. 78 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s
EXIT=0

$ cargo clippy -p vibe-doc --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.29s
EXIT=0

$ cargo build -p vibe-cli
   Compiling vibe-doc-shell v1.0.0 (C:\Users\olegc\git\v\vibevm\crates\vibe-doc-shell)
   Compiling vibe-doc-server v1.0.0 (C:\Users\olegc\git\v\vibevm\crates\vibe-doc-server)
   Compiling vibe-cli v1.0.0 (C:\Users\olegc\git\v\vibevm\crates\vibe-cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 7.64s
EXIT=0
```

(605 = юнит-тесты крейта, 5 + 4 + 5 = `projections`, `island`,
`doc_manifest_wire`, 78 = доктесты. Итого 697, упавших 0.)

### 7.3 TypeScript: пол, контраст, сборки, e2e

Пол запускался дважды, и первый прогон **красный по причине, которая
старше этой задачи** — держу оба.

```
$ TYPESCRIPT_AI_NATIVE=… node tools/floor.mjs --keep-going    # прогон 1
=== prettier --check (floor perimeter: design/src, site/src) ===
All matched files use Prettier code style!
=== tsc --noEmit ===
=== tests (node --test) ===
ℹ tests 141
ℹ pass 140
ℹ fail 1
✖ the generator accepts the policy this package's build wrote
  AssertionError: assert.ok(conf.includes(hash))   (site/src/seo/csp.test.ts:160)
floor: `tests` FAILED
=== eslint (floor perimeter: design/src, site/src) ===
=== typescript-ai-native-conform check ===
typescript-ai-native-conform check: 0 finding(s) in scope <workspace> ({}), 0 frozen in baseline, 0 new
=== typescript-ai-native-specmap --check ===
typescript-ai-native-specmap --check: clean (0 spec units, 162 tagged code items, 162 edges, 0 suspects, 162 warnings)
typescript-ai-native-specmap: ratchet gate — 0 orphan(s) (0 root(s) exempt)
=== test-gate (xfail-strict) ===
test-gate: 153 results parsed (1 failed, 0 skipped), baseline entries: 0
  NEWLY FAILING: the generator accepts the policy this package's build wrote
floor: `test-gate` FAILED
Error: floor: 2 step(s) failed: tests, test-gate
EXIT=1
```

Причина: тест читает `site/dist/csp.txt` от **предыдущей** сборки в этом
дереве, а тот файл — от 25 сентября 23:11 (до начала этой сессии), 94 922
байта против потолка `CSP_CONF_LIMIT = 4000`, то есть политика в нём — от
сборки над настоящими деревьями руководства (сотни страниц → сотни хешей).
Генератор такую политику не сериализует и пишет «NO POLICY IS SERVED»,
отчего `conf.includes(hash)` ложно. Файл в `.gitignore` и моими правками
записан быть не мог. После `node tools/build.mjs static` (шаг из этого же
списка, фикстурная библиотека, `csp.txt` = 3 608 байт) пол зелёный:

```
$ TYPESCRIPT_AI_NATIVE=… node tools/floor.mjs --keep-going    # прогон 2, после build static
=== prettier --check === … === tsc --noEmit === … === tests (node --test) ===
ℹ tests 141
ℹ pass 141
ℹ fail 0
=== eslint === … === conform === … === specmap === …
=== test-gate (xfail-strict) ===
test-gate: 153 results parsed (0 failed, 0 skipped), baseline entries: 0
test-gate: green (xfail-strict).
floor: all green (7 step(s) run, 0 disabled by policy).
EXIT=0
```

```
$ node design/audit/contrast.mjs
=== pairs: gated=46, reference=6; below threshold=0 ===
@media(prefers-color-scheme:dark) agrees with [data-theme="dark"]: OK (0 divergences)
tokens.css writes no colour of its own: OK (every value is a var() into palette.css)
EXIT=0
```

(Новых пар не добавилось: строка использует уже проверяемые `--text-3` на
`--bg-raise` и `--accent`.)

```
$ node tools/build.mjs static
build (static): generated 26 page(s), expected 26
build (static): 8 island(s) from the documentation trees, 0 from the package's own fixture page
build (static): csp.txt — 64 inline script hash(es) over 177 occurrence(s), no external source
links: green — 1029 checked, 0 broken.
build (static): ok
EXIT=0

$ node tools/build.mjs embedded
build (embedded): generated 14 page(s), expected 14
build (embedded): 14 page(s) carry none of the 7 public-only tags
build (embedded): ok
EXIT=0
```

`build (static)` печатает ещё одну известную находку `L-01` (4×): золотой
остров ссылается на `media/diagram.svg`, которого в фикстурном пакете нет.
Она старше задачи и не связана с цитатами.

```
$ node node_modules/@playwright/test/cli.js test -c site/tests/playwright.config.ts
  2 skipped
  206 passed (56.3s)
EXIT=0
```

Все e2e, не выборка: **206 прошло, 0 упало, 2 пропущено**. Пропуски
предсказаны самими тестами и к задаче не относятся: `local-reader.spec.ts`
пропускает два теста навигации, потому что локальный `vibe.exe` собран с
базовой (bare) оболочкой (`test.skip(bare !== "", …)`; рецепт в сообщении —
`--features vibe-doc-shell/embedded-shell`).

Мои 11 тестов из прогона:

```
ok  15 folded-rules.spec.ts:74  › a cited rule arrives folded to one line (303ms)
ok  16 folded-rules.spec.ts:93  › a rule with nothing to describe shows the edition's own words (277ms)
ok  17 folded-rules.spec.ts:107 › clicking the line opens the rule, and the mark turns (714ms)
ok  18 folded-rules.spec.ts:129 › Enter on the line opens the rule and closes it again (301ms)
ok  19 folded-rules.spec.ts:150 › the block's number stands in the margin while the rule is closed (329ms)
ok  20 folded-rules.spec.ts:174 › a print opens every closed rule and gives the page back after it (294ms)
ok  21 folded-rules.spec.ts:189 › a rule the reader opened by hand survives the print (313ms)
ok  22 folded-rules.spec.ts:203 › the rule panel opens from the text the fold reveals (345ms)
ok  23 folded-rules.spec.ts:219 › the page lists the rules it cites, folded or opened (294ms)
ok  24 folded-rules.spec.ts:230 › nothing the fold adds scrolls sideways at any width (600ms)
ok  25 folded-rules.spec.ts:253 › the mark turns on opening, and does not animate under reduced motion (473ms)
ok  86 reader.spec.ts:245       › a quoted rule opens beside itself with the text already in the page (335ms)
```

Покрытие списка пакета: свёрнута по умолчанию (▶ + `spec:` видны, текст
скрыт) — 15; клик раскрывает — 17; Enter раскрывает и сворачивает — 18;
номер виден в свёрнутом состоянии (и клик по нему не раскрывает) — 19;
`beforeprint` раскрывает всё, `afterprint` возвращает — 20 и 21; панель
правила по клику на раскрытом тексте — 22 (и 86 в `reader.spec.ts`); нет
горизонтального скролла на 390/834/1440, свёрнутом и раскрытом — 24;
при reduced motion перехода нет, но состояние приходит — 25. Плюс 16 (общая
подпись и отсутствие `spec:`) и 23 (список цитируемых правил жив).

## 8. Что не сделано и что стоит сделать дальше

1. **Корневой `specmap.json` устарел** — появился новый Rust-файл
   `crates/vibe-doc/src/html/rule.rs` с `specmark::scope!`, а корневая карта
   перечисляет файлы. Периметр прямо запрещает её трогать, поэтому она не
   перегенерирована: центральной сессии нужен один проход
   `cargo xtask specmap` (или обычный шаг пола Rust) перед коммитом.
2. **Базовая (bare) оболочка локальной читалки** не знает о
   `details.rule-fold`: `crates/vibe-doc-shell/src/fallback.rs` несёт
   минимальный inline-CSS для `.prose blockquote` и не прячет родной
   маркер `summary`. Сворачивание там работает (это родной `details`), но
   рядом с ▶ будет виден и браузерный треугольник. Крейт вне периметра —
   не тронут; правка на три строки, если владелец захочет.
3. **Печать в `design/print.css`** не тронута (вне периметра: разрешён
   `design/src/components/**`). Раскрытие на печати делает модуль читалки,
   как и предписано; при этом на бумаге у раскрытого правила остаётся и
   строка описания, и полный текст. Если это лишнее — одно правило
   `@media print` в `prose/styles.css` или в `print.css`.
4. `site/src/fixtures/README.md` не менялся: происхождение фикстур
   осталось прежним (копии тех же golden-файлов). Мелочь на будущее: в
   README записано «стоит на 3607 из 4000 байт» для потолка CSP, а сейчас
   `csp.txt` — 3608 байт; хешей столько же, число страниц и адресов я не
   менял, так что это старый дрейф записи, а не следствие этой задачи.

## 9. Отклонения

### 9.1 Одна строка в `crates/vibe-doc-server/src/routes.rs` (вне периметра)

Периметр разрешал `crates/vibe-cli/**` «только если без этого не провести
язык издания». На деле `vibe-cli` не строит `Content` вовсе, а
`vibe-doc-server` строит — исчерпывающим литералом структуры
(`routes.rs:339`). Любое новое поле в `Content` ломает такой литерал, а
`cargo build -p vibe-cli` собирает и `vibe-doc-server`, то есть шаг
самопроверки из пакета без этой правки не проходит физически.

Варианты, которые я отверг: положить язык на `Page` (прямо противоречит
записанной позиции `manifest::language` — «свойство пакета и никогда
страницы»); добавить вторую точку входа рендера и оставить локальную
читалку с английской подписью на русском пакете (тихая неправда в
продукте); провезти язык в `derived`-карте под служебным ключом (хак).

Сделано минимально и по духу разрешения: добавлено одно поле и одна
строка, заполняющая его из пакета, на котором открыт reader, так что
локальная читалка говорит о пакете то же, что сайт. Сообщаю как
отклонение — решение о периметре за центральной сессией.

### 9.2 Текст цитаты в тестовом наборе крейта удлинён

Пакет требует e2e «видна строка с ▶ и `spec:`». Единственное разрешаемое
правило фикстуры было длиной 6 слов, то есть по норме показывало бы общую
подпись, и слова `spec:` на фикстурной странице не было бы нигде —
требование стало бы непроверяемым, а golden пинал бы запасную ветку дважды
и выведенное описание ни разу.

Поэтому в наборе `Content` двух тестов (`tests/island.rs`,
`tests/projections.rs`) текст правила стал
`A package **MUST** declare its `kind`, and the kind decides which shape a
build reads it as.` (17 слов). Это **тестовые данные**, не продукт: код
`md.rs`/`xml.rs` не менялся, `.xml`-golden не двинулся, а `.md`-golden
изменился на одну строку — ту самую подставленную цитату. За ним по
таблице `fixtures/README.md` обновлены три копии в веб-пакете и одна
строка в рукописном `llms-full.txt` фикстуры.

Байт-в-байт требование пакета («`.md`, `.xml`, `llms*` как были») касается
руководства, и оно выполнено буквально (§7.1).

### 9.3 Прочее

- Новый модуль назван `html/rule.rs`, а не «функция описания рядом с
  `html.rs`»: `html.rs` стоял на 589 строках при бюджете 600
  (`conform.toml: max_file_lines = 600`), а в `html/example.rs` уже есть
  точный образец «блок жанра — свой модуль». Описание живёт там же, где
  единственный её потребитель; `html.rs` стал короче на 51 строку.
- `cargo fmt --all` переформатировал мой новый тестовый файл (один
  `assert_eq!` в три строки) — оставил как есть.
- Дважды правил концы строк: `python` на Windows перевёл
  `tests/island.rs` и `tests/projections.rs` в CRLF, вернул LF (проверено
  `tr -cd '\r'` = 0 у всех затронутых файлов).
