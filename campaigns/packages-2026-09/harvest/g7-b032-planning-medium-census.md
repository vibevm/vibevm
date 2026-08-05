# MEASUREMENT-M-B032 — протокол гранулярности планирования (BACKLOG B-032)

Замер того, что из строки B-032 уже реализовано, до её постройки. Все числа —
измерены в этом проходе по рабочему дереву `wt/M-B032` (коммит `34dfb52f`),
ни одно не унаследовано из задания. Корневая цитата задачи —
`BACKLOG.md:788` (`##B032-BUILD`): три стройки (when-to-propose у campaign-plans
+ кросс-ссылка из ряда-дома; конвенция «план ссылается на FEAT-файлы»; порог
крупности) и один замер от 2026-08-02 («`FEAT-*` — 0 у четырёх адоптеров, живых
планов кампаний — 8»).

Все пути ниже — относительно корня рабочего дерева, если не сказано иное.

---

## Q1 — где физически живут два названных флоу

**Важное уточнение формы:** `campaign-plans` — это **пакет**, а
`spec-tree-layout` — **не отдельный пакет, а один из документов флоу** внутри
пакета `addressable-specs`. То есть «два флоу» разнесены по **двум разным
пакетам** группы `org.vibevm.world`.

### Флоу `campaign-plans` — авторский пакет

- Каталог пакета: `packages/org.vibevm.world/campaign-plans/v0.1.0/`
- Главные спек-документы флоу (каталог `spec/flows/campaign-plans/`):
  - `CAMPAIGN-PLAN-FORMAT.md` — **283 строки**
  - `phase-gates.md` — **208 строк**
  - `execution-ledger.md` — **205 строк**
- Boot-сниппет флоу: `spec/boot/40-flow-campaign-plans.md` — **85 строк**
- Фасад пакета: `README.md` — **99 строк**

### Флоу `addressable-specs` (в нём живёт документ `spec-tree-layout`) — авторский пакет

- Каталог пакета: `packages/org.vibevm.world/addressable-specs/v0.1.0/`
- Документ `spec-tree-layout` (искомый «ряд-дом»): `spec/flows/addressable-specs/spec-tree-layout.md` — **181 строка**
- Соседние документы того же флоу: `ADDRESSABLE-SPECS-PROTOCOL.md` (296),
  `authoring-rules.md` (263); boot-сниппет `spec/boot/15-flow-addressable-specs.md` (69);
  фасад `README.md` (99).

**Почему это — АВТОРСКИЕ копии, а не что-то ещё.** Только пути под
`packages/org.vibevm.world/` несут пакетный фасад (`LICENSE.md`, `README.md`,
`vibe.toml`) и объявляют координату пакета — это авторский дом. Те же два флоу
лежат ещё **копиями** под `vibedeps/` внутри `packages/org.vibevm.fractality/**`:

- `packages/org.vibevm.fractality/fractality/v0.1.0/vibedeps/flow-campaign-plans/0.1.0/...`
- `packages/org.vibevm.fractality/fractality/v0.1.0/vibedeps/flow-addressable-specs/0.1.0/...`
- (и дубль в `…/delegation-rules/v0.1.0/vibedeps/…`)

Это регенерируемые потребительские копии (фрактальность как адоптер стянула
флоу в свой `vibedeps/`) — находка там ничего не доказывает. Отдельного пакета
`spec-tree-layout` в дереве нет: `glob packages/**/spec-tree-layout/**` → пусто;
`packages/**/*spec-tree-layout*` → ровно три файла (2 копии в `vibedeps/` +
1 авторский документ внутри `addressable-specs`).

Воспроизводящие команды:

```
wc -l packages/org.vibevm.world/campaign-plans/v0.1.0/spec/flows/campaign-plans/*.md \
      packages/org.vibevm.world/campaign-plans/v0.1.0/spec/boot/40-flow-campaign-plans.md \
      packages/org.vibevm.world/addressable-specs/v0.1.0/spec/flows/addressable-specs/spec-tree-layout.md
```

---

## Q2 — есть ли уже секция «when to propose» у campaign-plans

**ДА, секция существует** — но в **boot-сниппете** флоу, а не в форматном
документе. Boot-сниппет `spec/boot/40-flow-campaign-plans.md` несёт явный
заголовок:

`packages/org.vibevm.world/campaign-plans/v0.1.0/spec/boot/40-flow-campaign-plans.md:9`
```
## When to propose a campaign {#when}
```
и тут же правило под ним, дословно:
`packages/org.vibevm.world/campaign-plans/v0.1.0/spec/boot/40-flow-campaign-plans.md:11-13`
```
##PROPOSE-A-CAMPAIGN-BEFORE-TOUCHING-THE-TREE When the owner commissions work that spans **more than one session or
more than a handful of commits**, propose a campaign plan before
touching the tree. @impl/done
```

Форматный документ добавляет порог «когда платить за формат» дословно:
`packages/org.vibevm.world/campaign-plans/v0.1.0/spec/flows/campaign-plans/CAMPAIGN-PLAN-FORMAT.md:29-31`
```
##PAY-THE-FORMATS-COST-ONLY-FOR-WORK-THAT-SPANS-SESSIONS The
format's cost is real: pay it only when the work spans sessions or
more than a handful of commits. @impl/done
```
и определение «что такое кампания» — `CAMPAIGN-PLAN-FORMAT.md:16-18`.

**Оглавление кампании-плана (форматный документ, `CAMPAIGN-PLAN-FORMAT.md`),
заголовки с номерами строк:**

- L1 `# The Campaign Plan Format {#root}`
- L14 `## What a campaign is {#what}`
- L33 `## The five artifact roles {#artifacts}`
- L61 `## The section skeleton {#skeleton}`
- L68 `### 1 — Title and status line {#s1-status}`
- L87 `### 2 — Execution record (prepended at close) {#s2-execution-record}`
- L99 `### 3 — The mandate {#s3-mandate}`
- L113 `### 4 — Target arithmetic {#s4-arithmetic}`
- L127 `### 5 — Current-state facts (verified; do not re-discover) {#s5-facts}`
- L143 `### 6 — Decisions D1–DN {#s6-decisions}`
- L163 `### 7 — Predictions {#s7-predictions}`
- L171 `### 8 — Phases {#s8-phases}`
- L180 `### 9 — Risks and fallbacks {#s9-risks}`
- L188 `### 10 — Non-goals {#s10-non-goals}`
- L198 `### 11 — Quick-start for the executing session {#s11-quick-start}`
- L212 `### 12 — Whole-campaign acceptance {#s12-acceptance}`
- L223 `### 13 — Review points {#s13-review-points}`
- L230 `### 14 — Execution ledger {#s14-ledger}`
- L236 `### 15 — Deferrals ledger {#s15-deferrals}`
- L245 `## The lineage law {#lineage}`
- L254 `## Re-derive for your project {#re-derive}`
- L274 `## Summary {#summary}`

Соседние документы (оглавления заголовками): `phase-gates.md` — Phase 0 (L14),
Anatomy (L57), safe-stop (L101), Resumability (L122), Review points (L145),
Discovered-necessary work (L174), Summary (L195). `execution-ledger.md` — Why
(L14), status-flip (L30), execution-record (L47), commit-maps (L71), Honesty
(L100), report (L122), deferrals (L149), lineage (L171), Summary (L192).

**Итог Q2:** секция «когда заводить план кампании» есть (boot L9–13 +
FORMAT L29–31). Замечу для вердикта: эта секция говорит, *когда* платить за
кампанию-план, но **не** упоминает FEAT-файлы и **не** формулирует выбор медиума
— это перекрёстно с Q4.

---

## Q3 — что говорит «ряд-дом» у spec-tree-layout

Ряд, называющий дом для фичи, стоит в секции
`## What goes where {#what-goes-where}` (заголовок на
`packages/org.vibevm.world/addressable-specs/v0.1.0/spec/flows/addressable-specs/spec-tree-layout.md:78`)
и после правки 2026-08-02 действительно **называет оба дома** — и FEAT-файл, и
план кампании. Дословно:

`packages/org.vibevm.world/addressable-specs/v0.1.0/spec/flows/addressable-specs/spec-tree-layout.md:84`
```
| ##ROW-HOME-FEATURE-SCOPE A feature's scope and acceptance criteria @impl/done | `spec/modules/<m>/FEAT-*` — or a campaign plan where the project runs slices as plans (`flow:campaign-plans`) @impl/done |
```

- **Якорь ряда:** `##ROW-HOME-FEATURE-SCOPE`.
- **Якорь секции-родителя:** `## What goes where {#what-goes-where}` (`spec-tree-layout.md:78`).

Утверждение строки B-032 (якорь «правлен 2026-08-02: оба дома названы») —
**подтверждено дословно**: оба дома присутствуют в одной ячейке. Замечу для
вердикта: ряд *перечисляет* оба дома, но **не даёт критерия выбора** между ними
(см. Q4).

---

## Q4 — говорит ли хоть один из двух флоу, КАК выбирать между FEAT-файлом и планом кампании

**НЕТ.** Ни один из двух флоу не содержит протокола выбора медиума
(FEAT-файл vs план кампании). Доказательство — перечисление прогнанных шаблонов
(по смыслу, не по слову «FEAT»), с отрицательным результатом в обоих пакетах.

### Где оба медиума упомянуты вместе — ровно одно место

Греп `FEAT|campaign` по авторскому пакету `addressable-specs` даёт совместное
упоминание обоих медиумов **только в одной строке** — том самом ряде-доме из Q3,
и там критерия выбора нет:
`packages/org.vibevm.world/addressable-specs/v0.1.0/spec/flows/addressable-specs/spec-tree-layout.md:84`
```
... `spec/modules/<m>/FEAT-*` — or a campaign plan where the project runs slices as plans (`flow:campaign-plans`) ...
```
Это локатор («или план кампании там, где проект гоняет срезы как планы»), а не
правило выбора.

### campaign-plans вообще не упоминает FEAT как вид документа

Греп `FEAT` по авторскому пакету `campaign-plans` — ни одного попадания в смысле
«вид спек-документа FEAT-NNN»: только `##FEATURE-…` (имена якорей README) и
`feat(packages):` (пример conventional-commit в `execution-ledger.md:82`). Слово
«FEAT-файл» в campaign-plans отсутствует.

### Шаблоны, прогнанные по смыслу (отрицательный результат)

Регэксп `(?i)granular|medium|choose|choosing|choice|separate file|own file|own document|size of|three line|3 line|trivia|small feature|big feature|when to|when not` по **обоим** авторским пакетам. Совпадения и почему ни одно не есть протокол выбора:

**В `campaign-plans` (`…/campaign-plans/v0.1.0/`):**
- `README.md:36` — «when to propose a campaign» (описание boot-сниппета; не выбор медиума).
- `CAMPAIGN-PLAN-FORMAT.md:152` — «Rejections are as load-bearing as the **choice**» (`choice` = выбор опции внутри решения D1–DN кампании, не медиума).
- `spec/boot/40-flow-campaign-plans.md:9` — `## When to propose a campaign` (когда платить за формат; медиум не противопоставляется).

**В `addressable-specs` (`…/addressable-specs/v0.1.0/`):**
- `spec-tree-layout.md:158` — «Inventory every Markdown file» (промпт миграции, не выбор).
- `authoring-rules.md:8` — scope: «the size budgets, when to split a document».
- `authoring-rules.md:68` — таблица normativity, «the choice binds» (вид решения).
- `authoring-rules.md:182` — `## When to split a document {#splitting}` — **ближайший по смыслу**, но это про разбиение *одного* документа на под-документы по токен-бюджету (`authoring-rules.md:182-195`: over-budget / «and also» / two audiences / cited-section); FEAT и план кампании тут не упомянуты.
- `authoring-rules.md:190` — «promote it to its own document» (триггер сплита процитированной секции).

**Итог Q4 (обязательная форма «нет»):** НЕТ — искал по смыслу (granular/medium/
choose/choice/separate file/own file/own document/size of/three line/trivia/
when to propose/when to split), прогнал эти шаблоны по обоим авторским пакетам
`campaign-plans` и `addressable-specs` целиком; единственное совместное
упоминание обоих медиумов — `spec-tree-layout.md:84`, и оно критерия выбора не
даёт. Протокола «как выбрать медиум» в дереве флоу нет.

---

## Q5 — сколько сегодня `FEAT-*`-файлов в дереве и где

**0 (ноль) файлов** с именем, начинающимся на `FEAT-`, во всём дереве — включая
авторские места и любые копии. Замер 2026-08-02 («0 у четырёх адоптеров»)
подтверждается и расширен: ноль по всему дереву, не только у адоптеров.

Разбивка авторские/копии поэтому вырождается в **0 / 0**. Бонусный факт:
`.vibe/` в этом дереве отсутствует (нет каталога `.vibe` и, значит, нет
`.vibe/cache/`).

Воспроизводящие команды (обе дают пустой вывод):

```
find . -type f -iname 'feat-*' | sort        # файлы — 0
find . -type d -iname 'feat-*' | sort        # каталоги — 0
```

Санити-проверка, что `find` реально обходит дерево (а «0» — не слепое пятно):
`find . -type f -name '*.md' | wc -l` → **1710**; `vibedeps/`-каталоги под
`packages/` видны. Имена, лишь *содержащие* «feat» (`find . -type f -iname
'*feat*'`) — это исходники Rust (`features.rs`, `features_graph.rs`,
`crates/vibe-core/src/manifest/package/features.rs` и т. п.) и один
`docs/authoring-feat.md` (руководство по авторингу FEAT, не сам FEAT-файл); ни
один не начинается с `FEAT-`.

---

## Q6 — сколько живых планов кампаний и где они лежат

**Что флоу предписывает как «план кампании».** Форматный документ определяет
план как **один документ** с пятью ролями, а re-derive-промпт фиксирует имя и
расположение дословно:
`packages/org.vibevm.world/campaign-plans/v0.1.0/spec/flows/campaign-plans/CAMPAIGN-PLAN-FORMAT.md:264`
```
2. Name where campaign plans live (a version-controlled directory)
   and the filename convention (<NAME>-PLAN-v<N>.md).
```
То есть предписанный шаблон имени — `<NAME>-PLAN-v<N>.md`, один файл на кампанию.

**Замер по этому шаблону через всё дерево:** `find . -type f -iname
'*-PLAN-v*.md'` → **31 файл**, но почти все не «живые хостовые»:
- `legacy-spec/research/` — 3 файла, `legacy-spec/terraforms/` — 22 файла
  (**25 legacy**, замороженная история, не живые контракты — см. `progress.toml:13-22`);
- `packages/org.vibevm.fractality/fractality/v0.1.0/spec/plans/` — **6 файлов**
  (это specspace фрактальности, отдельный проект со своим boot/WAL).

**Хостовый нюанс (важно для вердикта):** хост для своих *живых* кампаний
использует **не** предписанный `-PLAN-v*`, а инфикс `-CAMPAIGN-v*`. Живых хостовых
планов кампаний — **2**, оба в `spec/terraforms/`, оба в активном исполнении:
- `spec/terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.md` — header: «**status:
  AUTHORED 2026-07-24 · IN FLIGHT** … Phase D OPEN …» (согласно
  `progress.toml:18-20` — это тот самый carve-out «active campaign plan»).
- `spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md` — header:
  «**status: RATIFIED 2026-07-26 · … PHASE E AUTHORIZED 2026-08-03**».

Отдельно: под `campaigns/` лежит **операционная** структура исполнения
(`campaigns/packages-2026-09/`, `campaigns/progress-2026-08/` — `run/state/
campaign.json`, `baseline.json`, `deferrals.md`, `tasks/`, `harvest/`). Это
run-state «Progress Control» (PROP-043), а не документы-планы; `campaign_id` там
(`packages-2026-09`, `progress-2026-08`) по имени не совпадает с планами в
`spec/terraforms/`.

**Воспроизведение «8» из B-032-LOCATOR.** Цифра **воспроизводится точно** как
**2 живых хостовых плана + 6 планов specspace фрактальности = 8** документов
формы «план кампании», не лежащих в `legacy-spec/` и не являющихся копиями в
`vibedeps/`. Воспроизводящие команды:

```
find spec -type f -iname '*-CAMPAIGN-v*.md' | sort                                   # хост: 2
find packages/org.vibevm.fractality -type f -iname '*-PLAN-v*.md' -not -path '*/vibedeps/*' | wc -l   # specspace: 6
```

Замечу для вердикта о границе: разложение 2+6 **пересекает границу
хост/specspace** (строка B-032 в остальном держит границу — `progress.toml:35-37`
явно выводит `packages/org.vibevm.fractality/**` из хостового периметра). Если
считать **только хост**, живых планов кампаний — **2**, не 8. Замер 2026-08-02
«8» корректен ровно для чтения «всё дерево минус legacy минус копии».

---

## Q7 — есть ли у спек-документов якорное пространство «бесплатно»

**ДА — утверждение B-032-BUILD («адресуемость уже есть бесплатно: FEAT-файл
получает якоря как любой спек-документ») подтверждено.** Адресация в этом
проекте — **выводится из пути**, а не регистрируется конфигом; плюс есть два
явных периметра, и оба накрывают канонический дом FEAT.

### Адрес как таковой — выводится из пути, нулевой конфиг

`packages/org.vibevm.world/addressable-specs/v0.1.0/spec/flows/addressable-specs/spec-tree-layout.md:138-140`
```
##A-URI-RESOLVES-WITH-ZERO-INDEX `spec://com.example.shop/PROP-001#verification.timeout` resolves with
zero index: `spec/modules/com.example.shop/PROP-001.md`, then find
`{#verification.timeout}`. @impl/done
```
и правило «имя = сегмент URI» — `spec-tree-layout.md:130-136` (каталог = модуль,
имя файла = документ, `{#anchor}` = фрагмент). Якорь — это `{#id}` на любом
заголовке любого спек-markdown: `ADDRESSABLE-SPECS-PROTOCOL.md:101-102`
(`##ANCHORS-ARE-EXPLICIT-HEADING-IDS`). То есть FEAT-файлу ничто регистрировать
не надо.

### Периметр 1 — `specmap.toml` (минтит `spec://`-адреса), корень репозитория

`specmap.toml:20-21`
```
# Markdown trees walked for anchored spec units (<root>/**/*.md).
spec_roots = ["spec"]
```
со исключениями `specmap.toml:65` → `spec_exclude = ["spec/WAL.md",
"spec/boot/STATIC.md"]`. Файл вида `spec/…/FEAT-что-нибудь.md` попадает под
`spec/**/*.md` и **не** в исключения → подлежит обходу и минтингу адресов.

### Периметр 2 — `progress.toml` (наблюдаемый периметр, include-глобы), корень репозитория

`progress.toml:83-91` (include-массив, дословно):
```
include = [
    "spec/boot/[0-9]*.md",
    "spec/common/**/*.md",
    "spec/design/**/*.md",
    "spec/manual-tests/**/*.md",
    "spec/modules/**/*.md",
    "packages/org.vibevm.world/**/*.md",
    "packages/org.vibevm.ai-native/**/*.md",
]
```

**Попал бы новый `spec/…/FEAT-что-нибудь.md` под эти глобы?** — **ДА, если он в
каноническом доме.** Канонический дом FEAT, по тому же ряду-дому
(`spec-tree-layout.md:84`), — `spec/modules/<m>/FEAT-*`; его накрывает глоб
`progress.toml:88`
```
    "spec/modules/**/*.md",
```
Также накрыты `spec/common/`, `spec/design/`, `spec/manual-tests/`. Не накрыт
только FEAT, положенный *прямо в корень* `spec/` (не под одним из перечисленных
подкаталогов) — но это не канонический дом.

**Итог Q7:** новый `spec/modules/<m>/FEAT-что-нибудь.md` (а) адресуется
`spec://<координата>/modules/<m>/FEAT-…` выводом из пути без конфига и (б)
попадает в наблюдаемый периметр через `progress.toml:88` `spec/modules/**/*.md`
и в адресный периметр через `specmap.toml:21` `spec_roots = ["spec"]`. Утверждение
«якоря бесплатно» — подтверждено.

---

## Сводка для вердикта босса (по трём стройкам B-032-BUILD)

- **(1) Протокольный абзац «как выбрать медиум» — НЕ построен.** Секция
  *where* (when-to-propose) у campaign-plans есть (`40-flow-campaign-plans.md:9`),
  но *what-to-choose* в ней нет (Q4); ряд-дом `spec-tree-layout.md:84` оба дома
  *называет*, но выбора *не предписывает* (Q3). Кросс-ссылка из ряда на
  campaign-plans уже есть (`spec-tree-layout.md:84` → `flow:campaign-plans`),
  обратной — нет.
- **(2) Конвенция «план ссылается на FEAT-файлы как на единицы работ» — НЕ
  построена.** В дереве **0** `FEAT-*`-файлов (Q5); ссылаться пока не на что.
- **(3) Порог крупности («не на 3 строчки») — НЕ построен** (нет ни числового,
  ни качественного порога в флоу; ближайшее — токен-бюджеты `authoring-rules.md`,
  не про это).
- **Замер-локатор от 2026-08-02 — оба числа подтверждены:** `FEAT-*` = 0 (Q5);
  живых планов кампаний = 8 по чтению «всё дерево минус legacy минус копии» (Q6,
  разложение 2 хост + 6 specspace; по чтению «только хост» — 2).
