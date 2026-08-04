# BACKLOG — what the mega-refactor found and did not do {#root}

_Created 2026-07-26 by owner directive. **Findings raised during the PROP-043
Progress-Control programme that are neither the campaign's own work nor an
emergency collect here, and the next wave of work drains from this file.**_

**Not `TASKS.md`.** That file is a live checklist for one work-slice — items
that are commits waiting to be made. This one is the opposite genre: findings
nobody is working on yet, kept so the decision to work on them can be taken
deliberately later. Two different questions, two files, by owner ruling.

---

## What this file is, against the three that resemble it {#boundaries}

| file | holds | drained by |
|---|---|---|
| ##REL-TASKS `TASKS.md` | the current slice's checklist — each item is a commit | itself, as work lands |
| ##REL-DEFERRALS `campaigns/<id>/deferrals.md` | **one campaign's** tails; dies with the zone | the next campaign's mandate (`campaign-plans` law) |
| ##REL-AUDIT `AUDIT.md` | the periodic health sweep; an append-only **trend** | re-judged at the next audit |
| ##REL-BACKLOG **this file** | product-shaped findings the programme surfaced and deliberately did not act on | the next wave of work, after the programme reaches its end |

- ##TASKS2-OUTLIVES-THE-ZONE **It lives at the repository root because a campaign zone is
  disposable.** `ZONE-LIFETIMES` says `run/` is throwaway after close-out and
  wave 1's already is. A finding about where the product should go outlives the
  campaign that noticed it.
- ##TASKS2-GENRE **Genre: forward-looking, non-binding, drained by a later mandate.** Not
  a contract, not a checkpoint, not a health record. `spec-genres`' map does not
  carry this genre — the row is owed, alongside the documentation row Phase G
  adds.

## Карта развития — порядок осушения этого файла {#map}

##MAP-POINTER **Как записи этого файла складываются в развитие системы — [`TOOLING-MAP.md`](TOOLING-MAP.md)** (рядом, корень репозитория): четыре плоскости инструментария с измеренным состоянием каждой, хребет зависимостей, предложение волн, десять развилок владельца, пять наблюдаемых вех. Карта — производное: записи здесь и рулинги владельца побеждают её везде, где разойдутся. Одобрена владельцем 2026-08-02 («мне нравится этот документ») с рамкой: **действуем внутри идущего рефакторинга (PROP-043, волна 2 — кампания packages-2026-09; фаза D → E/T/F/G), чего не хватает — откладывается на потом**; карта — форма осушения, не параллельный процесс. @doc/done

##MAP-WAVES-DIGEST Волны одной строкой (полные составы — в карте): **А** — детерминированная загрузка (B-011 самым высоким приоритетом → B-006/B-031/B-028); **Б** — паритет гейтов и новые классы правил (B-029/033/034/039/030 под циклом B-035 → B-036/037/038 → B-025/026); **В** — карта и её потребители (B-013 done 2026-08-03 → одна смена формата B-019а+B-016.1+B-017 → B-018 → B-020/021); **Г** — хост догоняет дисциплину (B-040, B-005, spec-метки схем) — оппортунистически. Вне волн: B-042 (далёкое будущее), B-015 (запаркована), B-032, B-043. @doc/done

## The three severities {#severity}

The scale is **P1 / P2 / P3**, taken from the `health-audit` flow rather than
invented. One severity vocabulary in the project, not two.

| | meaning | routing |
|---|---|---|
| ##SEV-P1 **P1** | security, data loss, structural integrity — **or a gate reporting green while not looking** | **stops the wave, reaches the owner the same session.** It never enters this file as a plan; it appears only afterwards, as record |
| ##SEV-P2 **P2** | a real gap with no emergency in it: a missing surface, a feature the corpus assumes and the code lacks, a mechanism specified and unbuilt | **this file.** Drains into the next wave |
| ##SEV-P3 **P3** | noted, no action planned | recorded here as `accepted`, so it is not rediscovered as new |

- ##SEV-REVIEWER-IS-AN-AGENT **«Reviewer» here means the boss *agent*, not the owner.** That is
  fine for classifying, and **not** fine for two things: **severity moves up
  freely and down only through the owner** (an agent may escalate to P1, never
  downgrade from it), and **every P2/P3 filed during a wave is reported to the
  owner at the time**, not merely written here — otherwise the agent deciding
  «this is a finding, not work» is the agent that wants to move on.
- ##SEV-ASSIGNED-BY-REVIEWER **Severity is the reviewer's call, never a worker's.** A cheap model
  calling something critical is noise, and a scale anyone may set is not a
  scale. A worker **reports the observation**; the reviewer classifies it.
- ##SEV-WORKER-MAY-INTERRUPT **One exception, running the other way:** a worker that believes it has
  found something genuinely alarming — a credential in source, an auth bypass, a
  gate that is lying — **stops its own packet and says so immediately**. The
  classification stays the reviewer's; the *interruption* needs no permission.
- ##SEV-P1-IS-NEVER-FILED **A P1 is never «filed».** That is the whole point of the split: one
  class of finding is not allowed to become a line in a list. If it is here, it
  is here as history, with what was done.
- ##SEV-GATE-BLINDNESS-IS-P1 **A gate that reports green because it is not looking is P1**, not P2.
  This programme found that shape three times — a floor gating a frozen slot, a
  parser blind to units the grammar allows, a sync check covering four of seven
  workspaces. Each was green and each was wrong, and a green panel that says
  nothing about coverage is a structural-integrity failure, not a gap.

## What an entry carries {#entries}

An **id**, the **`spec://…#ANCHOR`** it came from where one exists, a one-line
**locator**, a **severity**, a **disposition** (`open` · `planned` · `done` ·
`accepted`), and the **campaign or session** that filed it.

- ##ENTRY-CITES-NEVER-RESTATES **Cite the anchor; never restate the fact.** The same law Phase G's
  documentation runs on, for the same reason: a restated fact is a second
  statement of one truth with its own writer, and this programme has found that
  shape seven times.
- ##ENTRY-PREFER-GENERATED **Prefer generated over hand-maintained.** Where a finding is already
  carried by a marker — `action="rework"`, `stage="idea"`, an `#[ignore]`d test
  bound by its `verifies` edge — **the marked corpus is the source and this file
  quotes a query, not a copy.** A hand-maintained backlog is a derived value
  with its own writer, which is the defect class this programme keeps paying for.
- ##ENTRY-NO-SILENT-DELETION An entry leaves only by changing disposition, never by deletion. A
  backlog that forgets is indistinguishable from one that was never right.

## P1 — handled; kept as record {#p1}

*(empty — an open P1 is not in a file, it is in the owner's hands)*

## P2 — the next wave drains from here {#p2}

### B-059 — исключения конформа сопоставляются не с тем путём, который конформ печатает {#b-059}

| | |
|---|---|
| ##B059-ANCHOR **anchor** | нет — измерено при закрытии [B-057](#b-057); ближайший закон — `##SEV-GATE-BLINDNESS-IS-P1` в мягкой форме: ключ конфига, молча не делающий ничего, это не лгущий гейт, но соседняя болезнь |
| ##B059-LOCATOR **locator** | `packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-conform/src/store.rs:271-275` — исключение сопоставляется с `rel_in_crate`, путём **внутри крейта** (`src/lib.rs`); строками ниже (`:277-281`) в находку кладётся `file` — путь **от корня репозитория** (`crates/foo/src/lib.rs`). Одно поле конфига, два разных строковых пространства |
| ##B059-SEVERITY **severity** | P2 |
| ##B059-DISPOSITION **disposition** | `open` |
| ##B059-FILED **filed by** | замер долга дисциплины по пакетам, волна Г, 2026-08-05 — воркер опроверг боссов дизайн политики измеренным числом |

- ##B059-SUT **Суть, по-простому.** Человек читает находку и видит адрес `crates/foo/src/lib.rs`. Он хочет убрать этот крейт из скана и пишет `exclude_substrings = ["crates/foo/"]`. Не происходит ничего: сравнение идёт со строкой `src/lib.rs`, в которой имени крейта нет вовсе, поэтому совпадение невозможно в принципе. Ключ не отказывает и не предупреждает — он молча остаётся нулём. @doc/done
- ##B059-HOW-IT-SURFACED **Как всплыло — и почему это не теория.** Политика mcp-пакетов проектировалась так, чтобы вендор-копии не ловили находок: авторский дом у них в другом пакете, и править копию запрещено. Исключения были выписаны по именам копий. Замер показал `extracted` = «всё, кроме `crates/vendor/`» — то есть копии сканировались, — и правило длины файла выдало находку прямо на вендор-копии `rust-ai-native-conform-frontend/src/lib.rs` (601 строка при бюджете 600), по адресу, где чинить запрещено. Периметр был обойдён литеральным корнем (`##B057-LANDED`), сам дефект — здесь. @impl/done
- ##B059-WHY-NOT-P1 **Почему не P1.** Гейт не лжёт: он честно печатает, что просканировал, и находка настоящая. Ложным было ожидание автора конфига, а не вердикт машины. Но цена высока именно из-за бесшумности: конфиг выглядит рабочим, и обнаруживается это только сравнением `extracted` с ручным подсчётом файлов. @spec/done
- ##B059-FIX-SHAPE **Форма починки — три варианта, выбор за постройкой.** *(1)* Сопоставлять исключение с тем же репо-относительным путём, который печатается в находке — ключ начнёт работать так, как его читают, но у существующих конфигов меняется смысл (наш образец `"/generated/"` совпадёт в обоих прочтениях, у чужих потребителей может быть иначе — мерить до правки). *(2)* Сопоставлять с обоими путями — совместимо, но два пространства у одного ключа остаются. *(3)* Оставить как есть и добавить предупреждение: строка исключения, содержащая `/` и не отсеявшая ни одного файла, объявляет себя вслух. Дешевле всех и лечит именно бесшумность. @spec/plan
- ##B059-RELATED **Смежность.** [B-057](#b-057) — дыра, при закрытии которой это нашлось. Правило «пробел несёт причину И маршрут» здесь работает против нас: у молчаливого исключения нет ни причины, ни маршрута. @doc/done

### B-058 — производные сущности без гейта свежести: `vibedeps/` и `specmap.toml` {#b-058}

| | |
|---|---|
| ##B058-ANCHOR **anchor** | класс — «out-of-gate drift», предмет периодического аудита здоровья; родня — [B-014](#b-014), та же болезнь у коммитнутого индекса карты |
| ##B058-LOCATOR **locator** | *(i)* `vibedeps/` материализуются `vibe install` и **ничем не сверяются** с `packages/`: `tools/self-check.sh` install не запускает и свежесть копий не проверяет (единственное упоминание — комментарий на строке 201 о том, что инструкционные файлы пишет install). *(ii)* `specmap.toml` пишется генератором `init` **через `write_once`** (`…/go-ai-native-cli/src/init.rs:218`) — то есть один раз за жизнь проекта; обнаружение внешних корней (`[[external_specs]]`) при этом в генераторе есть и работает, но повторно не запускается никогда |
| ##B058-SEVERITY **severity** | P2 |
| ##B058-DISPOSITION **disposition** | `closed` — оба экземпляра получили свой сигнал 2026-08-05; см. `##B058-LANDED`. Третий экземпляр класса, [B-014](#b-014), закрыт волной В отдельно |
| ##B058-FILED **filed by** | вопрос владельца о цене переустановки при большом рефакторинге, 2026-08-04 (волна В) |

- ##B058-WHAT-IT-IS-NOT **Чем проблема НЕ является — и это надо сказать первым.** Не «переустановок слишком много». Измерено: правка КОДА пакета не требует переустановки вообще — хостовые крейты path-зависят от `packages/…` (8 из 8 зависимостей корневого `Cargo.toml`), а на `vibedeps/` не смотрят **ни разу**, и панель install не гоняет. Сто агентов, пишущих сто тестов в пакет, стоят ноль переустановок. Переустановка нужна только когда пакет меняет то, что публикует в поверхность потребителя (boot-снипет, тексты спек, скиллы), потому что скомпилированная boot-полоса собирается из установленных копий. @impl/done
- ##B058-WHAT-IT-IS **Чем является: нет сигнала о том, что переустановка нужна.** Никто не сверяет `vibedeps/` с `packages/`, поэтому протухание копий обнаруживается случайно, а не механизмом. Следствие поведенческое: агент, не имеющий сигнала, начинает переустанавливать **защитно** — и вот это уже дорого при большом рефакторинге. Защитный прогон есть симптом отсутствия сигнала, а не необходимость. @spec/done
- ##B058-BOTH-INSTANCES-PAID **Оба экземпляра класса заплачены в один день.** 2026-08-04: внешний spec-корень в хостовом `specmap.toml` указывал на `flow-core-ai-native/0.7.0` при установленном `0.8.0` — двенадцать цитат резолвились в никуда (починено коммитом `25628598`); и `vibedeps/` пришлось рематериализовать вручную после смены формата карты (`550f26d3`). Оба раза починка случилась не потому, что механизм сказал, а потому что босс случайно посмотрел. @impl/done
- ##B058-FIX-SHAPE **Форма починки — дешёвая проверка, а не новая машинерия.** *(i)* Шаг «согласованы ли `vibedeps/` с `packages/`» — по образцу того, как `cargo xtask sync-engines --check` уже сверяет вендор-копии с их источниками (механизм существует и работает, нужен его аналог этажом выше). *(ii)* Для `specmap.toml` — режим перегенерации/сверки обнаруживаемой части (`[[external_specs]]`), поскольку код обнаружения уже написан и просто не вызывается повторно; ручная правка агентом остаётся законной только там, где алгоритм не справился (формулировка владельца). @spec/plan
- ##B058-LANDED **Закрыто 2026-08-05, и обе половины вышли дешевле, чем ожидала запись — по одной причине в каждой.** *(i)* Свежесть `vibedeps/` **не потребовала ни нового шага панели, ни новой машинерии**: панель уже гоняет `vibe check`, а замо́к уже хранит на каждый пакет и адрес источника, и его хэш на момент установки. Проверка встала клеткой `local-source-freshness` в тот же ряд: пересчитать хэш источника у каждой записи `source_kind = "local"` и сравнить. Уровень — **предупреждение, а не ошибка**, и это решение, а не слабость: правка кода пакета переустановки не требует, так что ошибка красила бы панель на ровном месте и приучала бы её игнорировать — нужен сигнал, а не стена. Живой замер на посадке: **0 предупреждений** (36 записей, все локальные), и ноль подтверждён пересчётом хэша одного настоящего пакета вручную — то есть это не молчаливый пропуск. *(ii)* Для `specmap.toml` пере-запуск обнаружения оказался НЕВЕРНЫМ инструментом: генератор пишет полное имя (`org.vibevm.ai-native/core-ai-native`), а коммитнутый файл несёт короткое (`core-ai-native`), и сверка «пере-обнаружил и сравнил» краснела бы на расхождении имён вместо протухшей версии. Вместо неё — проверка в самом движке карты: объявленный корень `[[external_specs]]`, которого нет на диске, объявляет себя **громким предупреждением** при загрузке политики. Строгость снята с ошибки до предупреждения по предъявленному факту: слой разрешения НАМЕРЕННО терпит отсутствующий корень как состояние «ещё не установлено», и четыре такие записи живут в дереве (демо под `research/`). Болезнью было молчание, а не отсутствие. Одна правка в нейтральном движке достаётся всем трём языкам вендорингом. @impl/done
- ##B058-WHY-ONE-ENTRY **Почему одной записью, а не двумя.** Это один класс: производная сущность, у которой есть производитель и нет гейта свежести. B-014 — третий его экземпляр (коммитнутый индекс карты), и его решение внутри волны В пришло к тому же выводу — проверять содержание, а не байты. Чинить их порознь значит трижды изобретать один и тот же шаг панели. @doc/done


### B-057 — движок дисциплины не наведён сам на себя: конформ не гоняется по исходникам пакетов {#b-057}

| | |
|---|---|
| ##B057-ANCHOR **anchor** | нет — измерено попутно при постройке B-021; ближайший закон — принцип паритета (`##PARITY-ACROSS-PROJECTIONS`) в его самом неудобном прочтении: проекция, в которой дисциплина не применяется, — это тоже проекция |
| ##B057-LOCATOR **locator** | `tools/self-check.sh:325` — единственный прогон `cargo xtask conform check`, и он хостовый (13→14 гейтируемых крейтов, 6 исключённых — все из `crates/`). По воркспейсам пакетов панель гоняет `fmt`/`test`/`clippy` и специмаповые self-trace'ы (сиротство), но **конформ — нет**. Следствие: правила Class-F/G, бюджет длины файла и прочие правила движка не применяются к исходникам самого движка и стеков |
| ##B057-SEVERITY **severity** | P2 |
| ##B057-DISPOSITION **disposition** | `closed` — конформ гоняется по всем семи живым пакетным воркспейсам с 2026-08-05; см. `##B057-LANDED` |
| ##B057-FILED **filed by** | постройка B-021, волна В, 2026-08-04 — воркер сам назвал переполнение бюджета в файле, который никакой гейт не проверял |

- ##B057-WHY-NOT-P1 **Почему не P1.** `##SEV-GATE-BLINDNESS-IS-P1` — про гейт, который зелен **потому что не смотрит, утверждая, что смотрит**. Панель про пакетные исходники ничего не утверждает: её шаги названы честно (`cargo fmt --all --check (core-ai-native pkg)` и т.п.), конформа среди них нет и он не обещан. Это непокрытая площадь, а не лгущая панель.
- ##B057-THE-DEBT-IS-MEASURED **Долг за дырой измерен, и он почти нулевой.** По всем авторским исходникам пакетов (без `vendor/`, `generated/`, `target/`) бюджет 600 строк превышают **три файла**: `core-ai-native/v0.8.0/…/index.rs` (847) и `…/mdspec.rs` (615) — оба приведены в бюджет той же посадкой, что нашла проблему; и `core-ai-native/v0.7.0/…/index.rs` (639) — **замороженный слот предыдущей версии**, который по политике вне периметра наблюдения. Прочих превышений нет.
- ##B057-WHY-NOW **Почему закрывать сейчас.** Цена закрытия известна и мала именно потому, что долга нет: включение конформа по воркспейсам пакетов сегодня стоит одного шага панели и нуля правок кода. Через год, когда файлов станет больше, та же дыра будет стоить раскопок. Дыры закрывают, когда за ними пусто. @spec/done
- ##B057-FIX-SHAPE **Форма починки.** Шаг панели, гоняющий конформ по каждому живому воркспейсу пакета — по образцу того, как панель уже гоняет по ним `fmt`/`test`/`clippy`. Открытый вопрос к постройке: какой конфиг конформа брать для пакета (свой `conform.toml` у каждого или хостовый), и что делать с замороженными слотами предыдущих версий — их политика «размечено, но не верифицируется» уже существует и, вероятно, распространяется сюда без изменений. @spec/done
- ##B057-LANDED **Закрыто 2026-08-05 — и главное здесь то, что запись выше ошибалась в цене.** «Долг почти нулевой, три файла» было верно ровно для ОДНОГО правила (длина файла): остальные правила движка по исходникам пакетов не гонялись никогда, и первый же прогон всеми правилами дал **134 находки** — 9 у ядра, 31 у Rust-стека, 53 у TypeScript, 41 у Go, из них **102 одним правилом** (`seam-has-doctest`). Открытый вопрос про конфиг решён так: у каждого живого слота **свой** `conform.toml` (политика остаётся у потребителя, как и у `conform.toml` хоста), замороженные слоты прошлых версий в периметр не входят — панель выводит живой набор сама. Ничего из 134 **не заморожено**: политика гейтит крейты, у которых долга нет, и называет остальные `exempt` С ЧИСЛОМ находок — та же поза «расширяем по мере готовности», что у хоста. Итог калибровки: **8 крейтов под гейтом, 20 освобождённых с причиной, 134 → 6 находок**, из которых четыре `unsafe` в `core-ai-native-mcp/src/capture.rs` оформлены признанным отступлением (закон «помечать, а не гасить»), а две про длину файла лежат в реечных baseline как настоящий и дренируемый долг. Побочная находка стройки — [B-059](#b-059). @impl/done


### B-056 — множественное наследование контрактных документов и плагинная форма `#source` {#b-056}

| | |
|---|---|
| ##B056-ANCHOR **anchor** | `spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#source` (механика связывания) + `#contract-source` (раскол); родня — [B-055](#b-055), который про сегодняшнее молчание на второй директиве |
| ##B056-LOCATOR **locator** | сегодня свёртка одноисточниковая и одноуровневая: `crates/vibe-spec/src/pipeline.rs` берёт первую `#source` и сворачивает её текст, **не читая собственную `#source` источника** (рекурсии нет); резолвер отображает координату пакета в один установленный слот — перечисления по образцу не умеет |
| ##B056-SEVERITY **severity** | P2 |
| ##B056-DISPOSITION **disposition** | `planned`, **высокий приоритет** — **предложение владельца 2026-08-04:** «можем ли мы сделать полноценное множественное наследование для контрактных документов… и даже систему плагинов: `#source spec://org.vibevm.plugins/plugin-*`, дальше подхватываются все подходящие спеки и тела соответствующих секций склеиваются»; приоритет — его же словом («вероятно это нужно добавить всё куда-то в BACKLOG с высоким приоритетом»). **Три вопроса из четырёх им же и закрыты в тот же день — см. `##B056-RULED`.** |
| ##B056-FILED **filed by** | разговор о публикации контрактов, 2026-08-04 (волна В) |

- ##B056-WHY-IT-COMPOSES **Почему `:add`-половина достаётся почти даром.** Режим по умолчанию — сумма, а сумма ассоциативна: контракт + s1 + s2 + … складывается без единой новой сущности, секция-только-в-источнике просто добавляется. И коллизии между источниками **уже** ловятся громко: после слияния компилятор пере-проверяет уникальность якорей по объединённому виду и падает на выжившем дубле, то есть два плагина с одним `##ID` дают ошибку сборки, а не тихое «победил последний». @impl/done
- ##B056-RULED **Четыре рулинга владельца, 2026-08-04 — форма механизма закрыта.** *(1)* **`:replace` от любого источника выбрасывает только КОНТРАКТНЫЙ текст; источники между собой всё равно складываются по порядку.** *(2)* **Глоб — обязательна сортировка.** *(3)* **Обе формы:** несколько директив `#source` подряд И звёздочка для поиска. *(4)* **Рекурсия — по образцу C++/Java:** контракты включаются рекурсивно сколько угодно (граф от этого не растёт — дедупликация есть), реализации рекурсивно включать не следует; реализация может включать реализацию, но только до возникновения циклов. @spec/done
- ##B056-REPLACE-BECOMES-A-FLAG **Почему рулинг (1) лучше исходного предложения — и почему он ничего не ломает.** Босс предлагал считать два `:replace` на один якорь ошибкой сборки; при формулировке владельца **конфликта нет вовсе**: `:replace` перестаёт быть режимом «чей текст канонический» и становится флагом «контрактную сторону выбросить», после чего источники складываются по порядку независимо от того, сколько из них несли флаг. Проверено на вырожденном случае: при ОДНОМ источнике результат совпадает с сегодняшним поведением, то есть обобщение обратно совместимо. @spec/done
- ##B056-RECURSION-LAW-ALREADY-EXISTS **Рулинг (4) — не новое правило, а действующий закон этого проекта, не дотянутый до свёртки.** PROP-035 §9 несёт его под именем `##NO-DEADLOCK-INVARIANT`: «слой контрактов — там, где циклы законны; слой источников — там, где топологический порядок обязателен», и там же разобрана C++-механика на два разных приёма (стражи включения делают повторное включение no-op; предварительное объявление закрывает цикл без тела). **Реализовано:** `crates/vibe-spec/src/use_graph.rs` — трёхцветный DFS с дедупликацией («a node reached by several paths appears once»), предикат `is_contract` по сегменту пути, петля допускается только если ВСЕ её узлы контрактные, петля с источником — жёсткая ошибка; `#embed`-цикл запрещён отдельно. **Но всё это про `#use`:** тот же модуль явно говорит, что `#embed` и `#source` рёбрами зависимости не считаются и в обходе игнорируются. Значит работа — распространить существующий закон на свёртку `#source`, а не изобретать его. @impl/done
- ##B056-ODR-PARALLEL **Почему реализация не включает реализацию — то же правило одного определения.** В C++ `.cpp`, включивший `.cpp`, даёт дубль символа на линковке. У нас это уже воспроизведено: после слияния компилятор пере-проверяет уникальность якорей по объединённому виду, и выживший дубль — ошибка сборки. Java приходит к тому же с другой стороны: включения текста нет, интерфейсы наследуются множественно свободно, а неоднозначность default-метода — ошибка компиляции, снимаемая автором явно. Под всем этим одна фраза: **объявление идемпотентно, определение — нет.** @doc/done
- ##B056-DEDUP-ASYMMETRY **Дедупликация: статическая сторона подтверждена кодом, динамическая — нет.** Обход `#use` дедуплицирует по построению (см. выше). Структурный (динамический) режим исполняется LLM по первым инструкциям, и сами инструкции стоят на удержании до посадки B-011 — то есть там дедупликация есть свойство промпта, а не машины. Симметрию не утверждать, пока не измерено. @impl/work
- ##B056-DESIGN **Боссов дизайн стройки написан 2026-08-05:** [`spec/design/multiple-sources-and-plugins.md`](spec/design/multiple-sources-and-plugins.md). Он измеряет сегодняшнее состояние (свёртка берёт ПЕРВУЮ `#source` и имеет ровно два входа; рекурсии нет, потому что `use_graph` не считает `#source` ребром), выводит правило секции для последовательности источников, показывает, что рулинг (4) — уже действующий закон `##NO-DEADLOCK-INVARIANT`, которому не хватает досягаемости, и режет стройку на четыре посадки, каждая самостоятельная. Шаг 2 из четырёх закрывает заодно [B-055](#b-055). @spec/done
- ##B056-ORDER-AND-RECURSION **Что осталось на стройку после рулингов.** Порядок явного списка — порядок объявления (глоб — сортировка, рулинг (2)). Свёртке нужен собственный страж циклов по образцу `use_graph` и собственная дедупликация. Резолверу нужно перечисление по образцу. И `:replace`-флаг надо провести через `fold_source`, у которого сегодня ровно два входа вместо N. @spec/plan
- ##B056-PLUGIN-FORM **Плагинная форма: новое — только перечисление, а воспроизводимость спасает lockfile.** Резолвер сегодня точечный; глобу нужно перебрать установленное. «Что установлено» при этом не произвол среды, а зафиксированный lockfile'ом набор, поэтому при сортированном раскрытии одно дерево плюс один lockfile дают один результат. @spec/plan
- ##B056-GLOBS-DEGRADE-NATURALLY **Побочная выгода, закрывающая соседний спор.** Глоб, не совпавший ни с чем, — законный пустой набор, а не отсутствующий источник. Плагинная форма поэтому **не имеет** проблемы «объявлено-не-поставляется против потеряно», вокруг которой ходит тир приватности: глобы деградируют естественно, точечные адреса — нет. @spec/done
- ##B056-COST **Честная цена.** Дёшева только `:add`-половина. Резолвер с перечислением, семантика `:replace`, порядок и рекурсия — отдельная стройка со своим дизайном. Плюс следствие для чтения: секция, собранная из пяти плагинов, длинная, и пороговое предупреждение про длинные секции ([B-021](#b-021)) на ней сработает — скорее правильная обратная связь, чем помеха, но знать стоит заранее. @spec/done


*(Phase T's **T-unbuilt** bucket is still expected to be the bulk filler: a fact
whose surface does not exist is a P2 by construction, and the ignored test
already written from it is the specification of the work.)*

### B-001 — the §10 link tables, PROP-035's unbuilt half {#b-001}

| | |
|---|---|
| ##B001-ANCHOR **anchor** | [`spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#OPEN-LINK-TABLES`](spec/modules/vibe-workspace/PROP-035-spec-compiler.md) — see also `##link-tables-give-back`, both `@spec/work` |
| ##B001-LOCATOR **locator** | `crates/vibe-spec/src/link_table.rs` — the graph and a deterministic dump exist; the persisted on-disk format and the structural consumer do not |
| ##B001-SEVERITY **severity** | P2 |
| ##B001-DISPOSITION **disposition** | `open` |
| ##B001-FILED **filed by** | the packages-actualization campaign, Phase D, 2026-07-29, on an owner ruling |

- ##B001-WHY-NOT-NOW **Why it is filed and not built.** Phase D's boot-link repair reaches it
  and does not need it. `#embed spec://…` resolves and splices at compile time
  today — `render_static` calls `expand_embeds` (`crates/vibe-workspace/src/boot_artifacts.rs:268`),
  under two tests — and an `@spec://` pointer that costs a lookup is strictly
  better than the confidently wrong relative path it replaces. Building a new
  layer mid-refactor would create code the refactor then has to refactor, which
  is the owner's stated reason for deferring it.
- ##B001-WHAT-IT-IS **What it actually is.** The vtable of the structural / JIT executor of
  PROP-035 §13 — a prebuilt index so a late-bound reader dispatches instead of
  searching. We do not run that mode. It is an optimisation of navigation cost,
  not a precondition of correctness.
- ##B001-WHEN-IT-BECOMES-URGENT **The trigger that promotes it.** When `@spec://` pointers in the boot
  lane are measured to cost a reader more than the lane saves — or when the §13
  structural loader is opened, whichever comes first. Either makes the searching
  real rather than hypothetical.

### B-003 — the Go floor gates a directory named `dirty` {#b-003}

| | |
|---|---|
| ##B003-ANCHOR **anchor** | none — found in a captured run, not against a marked fact |
| ##B003-LOCATOR **locator** | `campaigns/packages-2026-09/harvest/go-ai-native-lang-floor.md:11,31-35`; the gate is `packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/crates/go-ai-native-cli/src/floor.rs` |
| ##B003-SEVERITY **severity** | P2 |
| ##B003-DISPOSITION **disposition** | `done` — **landed 2026-08-04 (`082e205b`), волна Б попутной стройкой:** the `[go].exclude_substrings` default (and the init template) gains `"/fixtures/"` (parity with the TS default), and the floor's gofmt step post-filters its listing through the same key with the engine's exact match semantics (`\`→`/` then substring; pure function, floor.rs's first unit tests). Measured on the package root: gofmt red-on-fixtures + 5 conform findings → gofmt green + 0 findings; the four remaining red steps are this row's recorded not-defects. Engine edit fanned ×6 via sync-engines; the TS twin hole is B-048 |
| ##B003-FILED **filed by** | the packages-actualization campaign, Phase D, 2026-07-29 |

- ##B003-WHAT **What it is.** `tools/go-extract/test/fixtures/dirty/` holds
  deliberately malformed Go — it is the extractor's negative-test input, and its
  directory is named `dirty`. The floor treats it as source: `gofmt` fails on
  `…/dirty/internal/cells/plan/plan.go`, and **all five** of the run's `conform`
  findings are inside that same tree. Two of the six failing steps are this one
  cause.
- ##B003-WHY-IT-IS-A-DEFECT **Why it is a defect and not taste.** The host already decided this
  question the other way for its own tooling: `DEFAULT_EXCLUDES` in
  `crates/progress-core/src/scope.rs` drops `fixtures` as *not a contract*,
  always on and not overridable by an explicit include. One project, two answers
  to «is a fixture source», and the Go floor has neither an exclude list nor the
  word `fixtures` anywhere in it.
- ##B003-NOT-P1 **Why P2 and not P1.** ##SEV-GATE-BLINDNESS-IS-P1 covers a gate that
  reports green because it is not looking. This one is the opposite: it looks at
  more than it should and reports red. That is noise, and noise in a gate is how
  a floor stops being read — but it is not a gate that lies.
- ##B003-DO-NOT-CONFUSE **What it is not.** The other four failures in that run — no Go module at
  the package root, no `conform.toml`, no `specmap.json`, two absent optional
  linters — are **not** defects. They are what a project-level floor prints when
  it is aimed at a package that is not a project, and Phase C's §2.2 decision
  captures that unmodified output on purpose. The missing-linter step failing
  rather than skipping is the discipline behaving correctly: it refuses to go
  green by omission.

### B-002 — the budget row still binds generated artifacts {#b-002}

| | |
|---|---|
| ##B002-ANCHOR **anchor** | `spec://org.vibevm.world/addressable-specs/authoring-rules#ROW-BUDGET-BOOT-FILE` |
| ##B002-LOCATOR **locator** | the row states one budget for «the boot file» and does not distinguish an authored document from a generated one |
| ##B002-SEVERITY **severity** | P2 |
| ##B002-DISPOSITION **disposition** | `open` |
| ##B002-FILED **filed by** | the packages-actualization campaign, Phase D, 2026-07-29 |

- ##B002-WHY-HERE **Why here rather than fixed.** The host side of this was ruled by the
  owner and is recorded in PROP-009 §2.3: a generated boot artifact carries no
  token budget. The package's own row is owed the same scope clarification, and
  changing it is a release event — a published version and a re-vendor — so it
  waits for the release batch rather than riding a document repair.

### B-004 — a fact inside a fenced block carries no anchor, so whether it is judged is luck {#b-004}

| | |
|---|---|
| ##B004-ANCHOR **anchor** | none — the finding is that the surface *has* no anchor. The nearest marked facts are the `##re-derive-prompt-lead` leads in 17 packages |
| ##B004-LOCATOR **locator** | `crates/vibe-spec/src/doctree.rs` `fence_mask`, applied by `directives.rs:13-14`; the corpus is the `Read spec/flows/<name>/ …` line opening the re-derive prompt in 17 `spec/flows/*/[A-Z]*-PROTOCOL.md` |
| ##B004-SEVERITY **severity** | P2 |
| ##B004-DISPOSITION **disposition** | `open` |
| ##B004-FILED **filed by** | the packages-actualization campaign, Phase D, wave 6, 2026-07-29 |

- ##B004-WHAT **What it is, measured.** Seventeen packages ship a re-derive prompt whose
  **first instruction** is `Read spec/flows/<name>/ …`. A consuming host has no
  `spec/flows/` — the flow arrives at `vibedeps/flow-<name>/…` — so the
  instruction cannot be followed where it is meant to be run. **Phase C recorded
  the defect in two of the seventeen** (`licensing`, `spec-genres`; obligation
  F-240). The other fifteen carry the identical line and their re-derive anchors
  are all judged `confirmed`.
- ##B004-WHY-THE-VERDICTS-ARE-NOT-WRONG **The eleven `confirmed` verdicts are not errors, and that is the
  point.** The same anchor supports several claims, and different workers took up
  different ones. `spec-genres` judged the *path* («its FIRST instruction cannot
  be followed where it is meant to be run»). `addressable-specs` judged the
  prompt's *shape* («a propose-then-approve shape — the host uses the same
  shape»). `source-mirrors` judged its *outcome* («the host's manifest is a
  derivation rather than a copy»). Each is defensible against a lead-in that says
  only «Paste this to your agent in a fresh session». Nothing was mis-judged;
  the fenced body simply is not addressable, so which of its claims gets tested
  depends on which one a worker happens to read.
- ##B004-WHY-P2-AND-NOT-P1 **Why P2 and not P1.** ##SEV-GATE-BLINDNESS-IS-P1 covers a gate that
  reports green because it is not looking. Phase C's gate reported **6 847 /
  6 847 anchors, zero owed**, and that claim is exactly true — it is scoped to
  *addressable anchors*, and excluding fenced content is deliberate (PROP-035 §7:
  directives inside fences «are ignored, exactly as headings are»). The gate
  measures what it says it measures. What is missing is reach of the **fact
  model**, not honesty in the gate.
- ##B004-WHY-IT-MATTERS-NOW **Why it is decision-relevant before the release batch.** F-240 asks
  the owner to publish a fix scoped to two packages. Fifteen more ship the same
  line. Publishing the narrow fix is precisely what §4.5 calls **not a closure** —
  «a fix landed in one consumer and not the others … is a new `duplication`
  obligation». The scope of that ask should be seventeen or the remainder should
  be recorded, and this file is the record until it is.
- ##B004-THE-GENERAL-SHAPE **The general shape, which outlives this corpus.** Copy-paste prompts,
  worked examples and quick-start blocks are exactly the content most likely to
  be *run* by a reader, and exactly the content the anchor model cannot see.
  Anywhere a fenced block carries an instruction rather than an illustration, it
  is unverified by construction.
- ##B004-WAVE8-CORRECTION **Corrected by wave 8's re-measurement (2026-07-31), three ways.**
  *(i)* The counts above were wrong in both directions: measured at HEAD, the
  fenced `Read spec/flows/<name>/ …` first line ships in **17 packages
  exactly**, and after wave 8 re-judged F-240's two leads `confirmed` (each was
  convicted of the fence's defect while its own do-not-copy-verbatim carve-out
  sits two lines above — 16 of 17 carry one), the lead anchors read
  **14 confirmed / 0 drift / 3 unjudged** — not «eleven confirmed», not
  «fifteen unrecorded». *(ii)* The scope question this entry poses to the owner
  is cleaner than either option it listed: **repairing all seventeen fences
  changes no verdict on any scope**, because a fence carries no anchor and no
  instrument can register the fix — the verdict question is closed and the
  repair question is a pure product decision. *(iii)* Wave 8 found the shape's
  second instance in the wild: the three `##three-processes-lead` ASCII
  diagrams in the `-lang` tools docs draw the retired `vibe-tcg` topology
  inside fences no anchor covers (`harvest/d8b-stacks-audience-release-reverify.md`).

### B-005 — `mirror --check` tests equality where the flow specifies ancestry {#b-005}

| | |
|---|---|
| ##B005-ANCHOR **anchor** | `spec://org.vibevm.world/source-mirrors/flows/source-mirrors/fanout-mechanics#INVARIANT-THE-ANCESTRY-GATE` — the rule; the defect is in the host's port of it |
| ##B005-LOCATOR **locator** | `xtask/src/mirror.rs:327-342` (`probe`), against the flow's own reference script at `fanout-mechanics.md:190-195` |
| ##B005-SEVERITY **severity** | P2 |
| ##B005-DISPOSITION **disposition** | `done` — построено 2026-08-04 (волна Г, коммит `39ad7b1d`). `probe` различает три состояния вместо двух: равенство ⇒ синхрон, предок ⇒ `Behind` (здоровое отставание, `--check` от него не падает), всё остальное ⇒ дрейф. Ловушка, из-за которой это не однострочник: sha цели приходит из `ls-remote` и может отсутствовать в локальном хранилище объектов — git тогда завершается кодом 128, а не 1; неизвестный объект трактуется как «не предок» ⇒ дрейф и никогда не роняет проверку. Решение вынесено в чистую функцию `classify(head, remote, ancestry)` и покрыто офлайн-таблицей из пяти случаев. JSON отчёта здоровья получил четвёртое имя состояния `behind`; набор имён нигде в дереве не закреплён. Путь push'а не тронут. Хвост F-204: строка реестра, названная `deferred` по этой записи, пере-судится ближайшим проходом якорей |
| ##B005-FILED **filed by** | the packages-actualization campaign, Phase D, wave 6, 2026-07-29 — found in passing while re-verifying F-204, outside its anchor list |

- ##B005-WHAT **What it is.** The flow specifies an **ancestry** gate: the target's main
  must be an ancestor of local mainline. Its own fifteen-line reference script
  implements exactly that — `git ls-remote` for the target's tip, then
  `git merge-base --is-ancestor`. The host's port does not: `probe` matches
  `Some(sha) if sha == head => SyncState::InSync` and sends everything else to
  `SyncState::Drift`. That is **equality**, and a target legitimately *behind*
  mainline — the ordinary state of every target between two fan-outs — is
  reported as drifted.
- ##B005-WHY-P2 **Why P2 and not P1.** It cannot produce a false green. `sha == head`
  implies in-sync under either test, so the error is strictly in the
  conservative direction: it reports red where the truth is «behind, which is
  fine». That is noise, and noise in a check is how a check stops being read —
  but it is not a gate that lies. Same reasoning as [B-003](#b-003), same
  direction.
- ##B005-NOT-THE-PUSH-PATH **What it is not.** The *push* path is sound and stays sound: it is
  fast-forward-only by construction, and `push_args_never_force`
  (`mirror.rs:426-440`) pins the never-`--force` invariant across four ref
  shapes. This is the read-only `--check` probe only.
- ##B005-THE-GENERAL-SHAPE **The shape worth remembering.** The package shipped a correct
  reference implementation *in shell*, and the consumer's re-implementation in
  Rust lost a property of it. Wave 6 nearly demoted the rule for the consumer's
  omission — the perimeter check caught that the package itself implements it.
  Where a flow ships a reference script, that script is a witness, and the port
  is the thing to audit against it.
- ##B005-NAMED-AS-F204-DEBT **Named as F-204's host debt** (owner ruling
  2026-08-01, the build-or-demote tail): the registry row is `deferred` naming
  this entry; the fix drains both together.

### B-006 — the highest-priority boot lane carries four normative snippets twice {#b-006}

| | |
|---|---|
| ##B006-ANCHOR **anchor** | falsifies `spec://org.vibevm.world/git-attribution-policy/flows/attribution-policy/ATTRIBUTION-POLICY#THE-POLICY-IS-STATED-IN-EXACTLY-ONE-ALWAYS-LOADED-PLACE` — from the host side, not the package's |
| ##B006-LOCATOR **locator** | `spec/boot/STATIC.md:421` and `:615` carry the identical `vibe:static org.vibevm.world/git-attribution-policy` provenance marker and source path; the emitter is `crates/vibe-workspace/src/boot_artifacts.rs` / the `bootgen` static lane |
| ##B006-SEVERITY **severity** | P2 |
| ##B006-DISPOSITION **disposition** | `done` — designed → **owner-approved 2026-08-04** («согласен с твоими рекомендациями a1 b1 c1», plus two hardening probes answered in the rule: mixed static/static-transitive consumers, de-substitution for a snippet-bearing aggregator) → contract landed (PROP-009 §2.3 `##STATIC-EMITS-ONCE-EACH`, PROP-038 §2.1, PROP-035 §8 per-node) → **built the same day by two claudez slices** (W-A de-substitution + Т1–Т7; W-B per-node qualify + Q1–Q7) → **acceptance measured on the live lane: git-family markers 9 → 5, double-qualified labels 164 → 0, −404 lines, the anchor claim true from the host side**. Record: [`spec/design/lane-composition-dedup.md`](spec/design/lane-composition-dedup.md). Named residual, accepted and routed: partial-coverage duplication belongs to hoisting (DRIFT-030's counter is the recorded trigger); the nested boot-bearing-umbrella fixpoint case is parked in the pass's doc comment |
| ##B006-FILED **filed by** | the packages-actualization campaign, Phase D, wave 6, 2026-07-29 |

- ##B006-WHAT **What it is, measured.** `spec/boot/STATIC.md` carries **31 static
  contributions resolving to 27 distinct sources**. The four duplicates are the
  whole `git-*` family — `git-atomic-commits`, `git-attribution-policy`,
  `git-autonomy`, `git-conventional-commits` — each emitted twice from the same
  `vibedeps/` path. They are reached both directly and through the
  `git-practices` umbrella the boot contract loads first, and the compiler
  concatenates both arrivals instead of emitting the contribution once.
- ##B006-WHY-IT-MATTERS **Why it is worth fixing rather than tolerating.** This is the lane
  `CLAUDE.md` tells every session to read «first and in full», so the cost is
  paid on every session by the most expensive reader in the project. And the
  content duplicated is **normative** — the commit rules — which is the
  `duplication` defect class this whole campaign exists to remove: one norm
  authored in two places with nothing forcing them to agree. Here they agree
  because they are byte-identical copies of one source, so nothing is *wrong*
  today; what is wrong is the shape.
- ##B006-IT-FALSIFIES-A-SHIPPED-CLAIM **It falsifies a shipped package's claim, and the package is not at
  fault.** `git-attribution-policy` states the policy «in exactly one
  always-loaded place (the boot snippet this package installs)». It installs
  exactly one. The consumer's compiler emits two. Wave 6 routed that obligation
  to the host on this evidence rather than softening the package's sentence.
- ##B006-WHY-P2 **Why P2 and not P1.** Nothing lies and nothing is lost: both copies are
  byte-identical and the rule they carry is the one in force. It is waste and a
  broken invariant, not a gate reporting green while not looking.

### B-007 — do the specs owe ADRs, and in what form? {#b-007}

| | |
|---|---|
| ##B007-ANCHOR **anchor** | the question is about `spec/common/**` and `spec/modules/**` as a genre, not about one anchor. The rule it would satisfy is `spec://org.vibevm.world/decision-records/flows/decision-records/DECISION-RECORDS-PROTOCOL#root` |
| ##B007-LOCATOR **locator** | 153 sections in `spec/common/` + `spec/modules/**` carry a bolded **Decision** label; 4 carry all four fields |
| ##B007-SEVERITY **severity** | P2 |
| ##B007-DISPOSITION **disposition** | `open` — **filed at owner request, 2026-07-31**, as a question to answer rather than work to schedule |
| ##B007-FILED **filed by** | the packages-actualization campaign, Phase D, wave 7 |

- ##B007-THE-QUESTION **The question, in the owner's framing.** Should the specifications
  carry Architecture Decision Records — and if so, **how**: as a section inside
  the PROP/FEAT that owns the decision, as a separate `spec/decisions/` genre, or
  as the four-field block the `decision-records` flow already prescribes? This is
  a **spec-genre design question**, and answering it decides how much work the
  `decision-records` host obligation actually is.
- ##B007-WHAT-IS-MEASURED **What is measured, so the question starts from facts.** Sections
  carrying a bolded `Decision` against those carrying all four fields
  (`Decision` · `Why` · `Considered and rejected` · `Revisit when` /
  `When to revisit`): `spec/common` + `spec/modules` **153 → 4**; all of `spec/`
  **157 → 7**; the `fractality` specspace **34 → 14**; this campaign's own
  records **15 → 8**. The practice is adopted at roughly **41 %** in the sibling
  project and **4.6 %** in the host's PROP/FEAT tree. Counted 2026-07-31.
- ##B007-CENSUS-CORRECTION **The sibling-adoption premise is withdrawn — corrected the same day
  by the D10 proposal pass.** The fractality «14 complete records» are, by file,
  **8 files carrying all four fields, all 8 vendored copies of the
  `decision-records` flow's own template, protocol, boot snippet and worked
  examples** (under `*/vibedeps/flow-decision-records/` and
  `flow-comparative-research/`, ×2 vendoring packages) — **0 authored**; the
  specspace's own authored decision blocks are 9, in a three-label dialect,
  none complete. So the honest comparison is «nobody authors the four-field
  form anywhere except this campaign's own plans», and the question is again
  *whether to adopt*, not «why is the PROP tree the outlier». Full measurement
  and the four costed options:
  `campaigns/packages-2026-09/harvest/d10-adr-genre-proposal.md`; the
  campaign's recommendation there is **B + A′** (four-field inside the owning
  section, forward-only, backfill only `spec/common/`, close `spec/decisions/`
  explicitly in the genre table).
- ##B007-WHY-IT-IS-A-QUESTION-NOT-A-TASK **Why it is a question and not a task.** «Add the missing fields to
  153 sections» is the wrong shape twice over. Most of those decisions are not
  reopenable, so a revisit condition on them would be ceremony; and the four-field
  block is not obviously the right ADR form for a specification, which already
  states rationale in prose. **What is owed first is the genre decision**, and
  `spec-genres`' own map does not carry an ADR row today.
- ##B007-WHAT-IT-UNBLOCKS **What it unblocks.** The largest single host obligation this phase
  surfaced ([`PHASE-D-HOST-OBLIGATIONS.md`](campaigns/packages-2026-09/PHASE-D-HOST-OBLIGATIONS.md)).
  It cannot be sized, let alone scheduled, until this is answered.

### B-008 — one workspace crate declares no licence, and the live ledger says otherwise {#b-008}

| | |
|---|---|
| ##B008-ANCHOR **anchor** | `CLAUDE.md`'s operating-facts ledger, «License state»: *«our shipped surface is fully UPL-1.0 … host crates inherit via `license-file.workspace`»* |
| ##B008-LOCATOR **locator** | `crates/vibe-index/Cargo.toml` — no `license` or `license-file` key of any kind. Every other workspace member carries `license-file.workspace = true` on line 7; the workspace declares `license-file = "LICENSE.md"` at `Cargo.toml:55` |
| ##B008-SEVERITY **severity** | P2 |
| ##B008-DISPOSITION **disposition** | `open` |
| ##B008-FILED **filed by** | the packages-actualization campaign, Phase D, wave 7, 2026-07-31 — surfaced while re-verifying F-236, outside its anchor list |

- ##B008-WHAT **What it is.** `vibe-index` carries full package metadata —
  `authors`, `description`, `homepage`, `repository`, `keywords`, `categories` —
  and omits the licence line alone. It is the **only** crate in the workspace
  that does, checked by iterating every `crates/*/Cargo.toml`. So the ledger
  sentence «host crates inherit via `license-file.workspace`» is true of every
  crate but one, and the relicensing run that made the surface UPL-1.0 did not
  reach it.
- ##B008-WHY-P2-AND-NOT-HIGHER **Why P2.** `publish = false`, so nothing reaches a registry
  undeclared and no third party receives an unlicensed artifact. The defect is
  that a **live, owner-maintained ledger asserts something that is false for one
  member** — which is precisely the class this campaign exists to remove, and
  the campaign found it in its own host rather than in a package.
- ##B008-WHY-FILED-NOT-FIXED **Why filed and not fixed.** It is a one-line change and it is a
  change to the legal surface. `RULE-NO-SILENT-REPAIRS` binds the phase, and
  `CLAUDE.md`'s licence ledger is owner-maintained — an agent editing a licence
  declaration on its own initiative is the wrong default even when the edit is
  obviously right. **The fix is `license-file.workspace = true` on line 7, to
  match its twenty-odd siblings.**

### B-009 — the wind-down's push step contradicts the rollout two host documents standardise {#b-009}

| | |
|---|---|
| ##B009-ANCHOR **anchor** | falsifies nothing in a package — the contradiction is host-internal. The rule side is `spec/boot/90-user.md` `##CMD-MIRROR` and `spec/common/PROP-016-source-mirrors.md` `##CMD-MIRROR`; the breach side is `CLAUDE.md`'s END SESSION step 4 |
| ##B009-LOCATOR **locator** | `CLAUDE.md:191` — «Push to `origin/main` — routine per Rule 4» as the wind-down's step 4, where `90-user.md:35` says `cargo xtask mirror` «is the standard rollout, preferred over a bare `git push origin`» and `PROP-016:59` says «This — not `git push origin` — is the standard rollout» |
| ##B009-SEVERITY **severity** | P2 |
| ##B009-DISPOSITION **disposition** | `done` — owner ruling 2026-07-31 («сделай»): step 4 of the wind-down in all three instruction files now names `cargo xtask mirror` as the standard rollout, with the bare push demoted to fallback and the escape hatch preserved |
| ##B009-FILED **filed by** | the packages-actualization campaign, Phase D, wave 8, 2026-07-31 — surfaced re-verifying F-220's source-mirrors half, where the recorded verdict used `CLAUDE.md:191` to demote a package sentence that the other two host documents support |

- ##B009-WHAT **What it is.** Three host documents state the wind-down rollout and one
  disagrees with the other two. The wind-down contract in `CLAUDE.md` prescribes
  the bare named-remote push; the user-owned boot snippet and PROP-016 both name
  the mirror fan-out the standard rollout and explicitly deprecate the bare push
  for it. A session following `CLAUDE.md` to the letter rolls out to one host
  and leaves every other mirror behind — the exact state `PROP-016`'s fan-out
  exists to prevent.
- ##B009-WHY-FILED-NOT-FIXED **Why filed and not fixed.** `CLAUDE.md` is the owner-maintained
  boot contract; its END SESSION section is an owner-authored command
  specification, and `RULE-NO-SILENT-REPAIRS` binds the phase. The fix is one
  line — step 4 saying `cargo xtask mirror` (or «push, then fan out») — but
  which wording the owner wants is the owner's call.
- ##B009-COST-TODAY **What it costs today.** Every session that ends by the book pushes
  `origin` only; the mirrors drift until someone runs the fan-out by hand, and
  `mirror --check`'s equality probe (B-005) then reports the *targets* as
  drifted — two filed defects compounding into one confusing red panel.

### B-010 — a check verb that writes, and a `--campaign` flag that selects state rather than scope {#b-010}

| | |
|---|---|
| ##B010-ANCHOR **anchor** | none — found by a delegated run, not against a marked fact; the nearest law is `tool-design-lessons`' read-verbs-do-not-mutate genre |
| ##B010-LOCATOR **locator** | `vibe progress check --exhaustive --campaign <zone>` — rewrites the named zone's `run/cache.json` / `state/campaign.json` / `state/corpus.json` (observed: +4 962 lines in the closed wave-1 zone's cache, plus a re-scope of the live zone's corpus), because `--campaign` selects the **state zone to write**, not the perimeter to read |
| ##B010-SEVERITY **severity** | P2 |
| ##B010-DISPOSITION **disposition** | `open` |
| ##B010-FILED **filed by** | the packages-actualization campaign, D10 pass, 2026-07-31 — a drafting worker pointed the check at the closed `progress-2026-08` zone expecting a read; the boss restored all six files from HEAD, loss-free |

- ##B010-WHY-IT-BITES **Why it bites.** A verb named `check` reads as read-only, and the flag
  named `--campaign` reads as «over this campaign's perimeter»; together they
  silently rewrite a **closed** campaign's frozen state. `ZONE-LIFETIMES` calls
  a closed zone's `run/` throwaway, so nothing broke here — but the same
  combination pointed at a **live** zone during another session's merge window
  would race its cache.
- ##B010-THE-FIX-SHAPE **The fix shape, for Phase E.** Either `check` becomes read-only
  (scan state moves behind an explicit `--write-state`), or its help says in the
  first line that it warms the zone's cache; and `--campaign`'s help says it
  selects the state zone. One of the two — a check that quietly writes is how a
  frozen zone stops being frozen.

### B-011 — marker stripping in the boot compiler needs an aliasing design first {#b-011}

| | |
|---|---|
| ##B011-ANCHOR **anchor** | the compile path: `crates/vibe-workspace/src/boot_artifacts.rs` (static lane), `boot_artifacts/normal.rs` (PROP-035 §8 compile); no marker handling exists anywhere in it today |
| ##B011-LOCATOR **locator** | measured 2026-07-31: the 22 canonically-mapped static contributions carry 838 `##ANCHOR` / `@stage/state` tokens over 1 446 source lines, all of which compile verbatim into `spec/boot/STATIC.md` after a `--force` re-vendor |
| ##B011-SEVERITY **severity** | P2 |
| ##B011-DISPOSITION **disposition** | `planned`, **самый высокий приоритет — решение владельца 2026-08-02**: «Я бы поставил это в бэклог план с Самым Высоким Приоритетом. От этой вещи зависит как вообще работает загрузка, насколько детерминированно и хорошо» (прежняя запись 2026-07-31 — «это не сейчас, это в бэклог» — поглощена этим решением) |
| ##B011-FILED **filed by** | the packages-actualization campaign, the publication runbook's marker fork, 2026-07-31; расширена рулингом семьи дупликаций F-217/F-218, 2026-08-02 |

- ##B011-WHY-NAIVE-STRIPPING-IS-WRONG **Why naive stripping is wrong, in the owner's own framing.** Strip
  the markup from the compiled lane and a **dynamic module can reference an
  anchor that existed in the source markup and vanished after cleaning** — the
  reference resolves at authoring time and dangles at read time. Stripping is
  not a filter; it changes what is addressable from where, so it needs a
  resolution design, not a regex.
- ##B011-THE-DESIGN-DIRECTION **The design direction (owner, 2026-07-31).** Short names of the shape
  `#use spec://… as SOMETHING`: a lane consumer imports an anchor under an
  alias, and **when SOMETHING's carrier was cleaned, the compiler loads the
  source markup and learns where the anchor lives** — resolution survives
  stripping because the alias binds to the source-of-truth address, not to the
  compiled text. Stripping then becomes safe to build on top.
- ##B011-INTERIM **The interim, ruled the same day:** publish as is — the lane carries
  the authoring tokens (the house grammar every agent here reads), and the
  strip waits for the aliasing design rather than shipping half-safe.
- ##B011-COLLISION-REQUIREMENT **Расширение 2026-08-02 — коллизии меток при склейке (из F-217/F-218).**
  Склейка `spec/boot/STATIC.md` из boot-сниппетов (31 вклад от 27 пакетов)
  сталкивает заголовочные метки: измерено 59 предупреждений `duplicate-anchor`
  по 11 именам — `{#root}` определён 26 раз, `{#never}` 17, `{#when}` 9; индекс
  уже чеканит живой адрес `spec://org.vibevm.core/vibevm/boot/STATIC#root` на 26 «корней» сразу
  (ссылок из кода пока ноль — неоднозначность латентна). Требование владельца
  (2026-08-02, дословно по смыслу): **склейщик переименовывает метки внутри
  STATIC.md так, чтобы все ссылки на эти метки сохраняли валидность по всему
  документу**; отдельно рассмотреть сложные случаи **динамической дозагрузки
  новых библиотек, в которых есть собственные STATIC.md**. Реестровые строки
  F-218 (обе — «один факт — один якорь» и его summary) отложены с именем этой
  записи; половина F-217 — тоже (вторая половина — тройка CLAUDE/AGENTS/GEMINI —
  ждёт отдельного решения владельца).
- ##B011-QUALIFIED-REWRITE **Идея владельца (2026-08-02): qualified-переписывание при материализации.**
  «Возможно, при материализации нам нужно сразу переписывать все ссылки на
  qualified варианты? То есть, если кто-то обращается к #root внутри документа,
  мы понимаем, что это какой-то из "spec://.....#root"». Ориентир дизайна —
  **ADL в C++ (Argument Dependent Lookup)**: как контекстно-зависимо сокращать
  разные ссылки. Синтаксис из прежнего направления дополняется короткой формой
  обращения: `#use spec://… as X`, затем в тексте спецификации — `@!X`. Алиасы
  привязываются к source-of-truth адресу, поэтому переживают и склейку, и
  стриппинг (исходная мотивация записи).
- ##B011-DESIGN-APPROVED **Дизайн одобрен владельцем 2026-08-04** («Принимаю
  дизайн B-011») — [`spec/design/deterministic-loading-aliasing.md`](spec/design/deterministic-loading-aliasing.md),
  со всеми рекомендованными развилками (полный слаг · обе разновидности меток ·
  `@!X` компилируется в полный адрес · жёсткая ошибка с кандидатами + флаг
  мягкости · install-валидация ссылок отдельной стройкой) и добавлением
  владельца тем же сообщением: правила резолвинга приоритизируются для
  агента-исполнителя — преамбула первыми строками склейки + first-instructions
  контракт §13 (дизайн §5.1). Нормативная посадка — правки PROP-035/PROP-009
  (§7 дизайна); имплементация — фаза E, срезы W1–W4 (§10). @impl/work

### B-012 — PROP-014's specified-not-built mechanism set: research feasibility {#b-012}

| | |
|---|---|
| ##B012-ANCHOR **anchor** | the ten annotated facts of `spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014-specmap-bidirectional-traceability` — each now carries its «Specified, not built» clause naming exactly what is absent |
| ##B012-LOCATOR **locator** | the mechanisms, in one list: package-shipped `specmap.json` index + fetch-by-content-hash; the per-item edge-multiplicity lint in `vibe check`; `CodeItem.content_hash` + derived `Command`/`ErrorVariant` node views; error-rendering **index lookup** with revision + `run: vibe explain` hint (the compile-time-constant doorway ships); the LLM prose producer behind `vibe explain --prose` (deterministic template ships); `[metamodel] profile` runtime profiles; the spec-unit length warning (≤ 120); rustdoc composition in `explain`; `specmap_query` / `specmap_source` MCP tools |
| ##B012-SEVERITY **severity** | P2 |
| ##B012-DISPOSITION **disposition** | `done` — исследование выполнено 2026-08-01 (`campaigns/packages-2026-09/harvest/d14-b012-prop014-feasibility.md` + части A/B), решения владельца того же дня разлиты в записи [B-015](#b-015)…[B-021](#b-021): всё из десятки строится (диспозиция `planned`), безопасность — протоколируется и паркуется до уведомления владельца |
| ##B012-FILED **filed by** | the packages-actualization campaign, партия 1a, 2026-08-01 |

- ##B012-WHY-RESEARCH-FIRST **Why research-first.** Wave 8/D9 established the corpus-side truth
  (the annotations); the product-side question — which of the ten are worth
  building, in what order, and which are better retired from the spec — is a
  design pass over PROP-014's §13-era ambitions against today's shipped
  surface. B-001 (the link tables) is the same family and the same trigger
  logic; the two studies should run together.

### B-013 — the specmap schema-bump path is broken before anyone needs it {#b-013}

| | |
|---|---|
| ##B013-ANCHOR **anchor** | none — found by the B-012 evidence pass, not against a marked fact; the nearest law is `dev-runtime-docs`' never-describe-an-abandoned-toolchain |
| ##B013-LOCATOR **locator** | `xtask/src/codegen.rs:50-52` routes the `specmap` schema's codegen to `packages/org.vibevm.ai-native/rust-ai-native-lang/v0.5.0/crates/specmap-core/src/generated` — a slot that does not exist (only `v0.7.0` does); repeated for the drift check at `codegen.rs:215`. Two more coordinates of the same stale relocation: `schemas/specmap.jtd.json` metadata still names `crates/specmap-core/...` / `specmap_core::specmap`, and `core-ai-native-specmap/src/lib.rs:24-27` names a package-local `schemas/specmap.jtd.json` that is absent from the repository |
| ##B013-SEVERITY **severity** | P2 |
| ##B013-DISPOSITION **disposition** | `done` — закрыт исполнением рулинга F-279 (вариант (а), дан 2026-08-02, исполнен 2026-08-03, коммиты `a6bb261e`/`b4c48aa0`): все четыре координаты fix shape — `generated_dir_for` и drift-check целят в живой движок `core-ai-native/v0.8.0/crates/core-ai-native-specmap/src/generated`, метаданные схемы называют `core-ai-native-specmap`, и сама схема переехала в пакет (`core-ai-native/v0.8.0/schemas/specmap.jtd.json`, рядом канонический `specmap.example.json`), так что заголовок движка стал правдой; `cargo xtask check-codegen` чист; генератор упакован как `tool:org.vibevm.ai-native/jtd-codegen` (рецепт, не бинарь) |
| ##B013-FILED **filed by** | the B-012 feasibility study (`campaigns/packages-2026-09/harvest/d14-b012-part-B.md` §B2, part A cross-cutting note), 2026-08-01 |

- ##B013-WHY-IT-BITES **Why it bites.** Every serialised-index evolution in the B-012 set —
  `CodeItem.content_hash` (M7a), a serialised `doc` field (M10), signatures for
  the `contract` profile (M3) — is a `SCHEMA` 2→3 bump that must go through
  jtd-codegen, and the route 404s on first use. The engine relocated into
  `core-ai-native/v0.8.0` and the codegen plumbing did not move with it.
- ##B013-WHY-P2 **Why P2.** Nothing lies: the checked-in generated module is current and
  the gate byte-compares real artefacts. The defect is a dev-op that fails on
  first invocation — noise at the exact moment someone attempts a planned
  evolution, plus two documentation surfaces describing a pre-relocation world.
- ##B013-FIX-SHAPE **The fix shape.** Point `generated_dir_for` at the authored engine
  (`packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-specmap/src/generated`),
  fix the drift-check twin, refresh the schema file's metadata, and either add
  the package-local schema copy the engine header promises or reword the header.

### B-014 — the committed host specmap.json drifts with no freshness gate {#b-014}

| | |
|---|---|
| ##B014-ANCHOR **anchor** | none — measured by the B-012 evidence pass; the class is the health-audit's out-of-gate drift, `spec://org.vibevm.world/health-audit/flows/health-audit/HEALTH-AUDIT-PROTOCOL#root`'s own subject |
| ##B014-LOCATOR **locator** | root `specmap.json`: **599 of 5266** spec units' recorded `line` no longer lands on that unit's anchor at HEAD (concentrated: PROP-000 ×137, PROP-043 ×112, PROP-018 ×92, PROP-009 ×91); the code side holds (898/912 edges land on a marker line). No gate covers it: `tools/self-check.sh:366-375`'s specmap steps are the packages' own `--gate` self-traces, and no host-index regeneration or byte-compare runs anywhere in the panel |
| ##B014-SEVERITY **severity** | P2 |
| ##B014-DISPOSITION **disposition** | `open` |
| ##B014-FILED **filed by** | the B-012 feasibility study (`campaigns/packages-2026-09/harvest/d14-b012-part-B.md` §B4 freshness caveat), 2026-08-01 |

- ##B014-WHY-NOT-P1 **Why P2 and not P1.** `##SEV-GATE-BLINDNESS-IS-P1` covers a gate that
  reports green *because it is not looking while claiming to look*. No gate
  claims to check the host index — self-check's specmap steps name the package
  slots they trace, truthfully. This is a committed derived artefact whose
  producer is never re-run: out-of-gate drift, the exact class the periodic
  audit exists for, not a lying panel.
- ##B014-COST **What it costs today.** Any consumer of the committed index inherits
  stale spec-side coordinates — including the M2 доorway slice the B-012 study
  shortlists (its 81/81 URI-resolution measurement holds, but a printed
  `file:line` would be wrong for ~11 % of units) — and every index-derived
  distribution must carry a freshness caveat, as the study's own tables do.
- ##B014-FIX-SHAPE **The fix shape, two independent halves.** (i) Regenerate the index and
  commit it (one command, one churny diff). (ii) Decide whether the host wants
  a freshness gate at all — a `--check`-style byte-compare in self-check, a
  WalFreshness-style staleness warning in `vibe check`, or a deliberate
  «regenerated on demand only» posture recorded as a decision. The A–D
  health-audit inventory scheduled at the Phase D exit gate should meet this
  entry there.

### B-015 — программа безопасности runtime-канала: запротоколирована и запаркована до уведомления владельца {#b-015}

| | |
|---|---|
| ##B015-ANCHOR **anchor** | тема §2.8.4 PROP-014 (specmap); полное досье — `campaigns/packages-2026-09/harvest/d14-b012-part-A.md`, раздел A5 |
| ##B015-LOCATOR **locator** | подписи нет нигде в дереве (единственная crypto-зависимость — sha2 для контент-хэшей); две уже шипящиеся дороги «текст пакета → контекст агента» перечислены ниже |
| ##B015-SEVERITY **severity** | P2 |
| ##B015-DISPOSITION **disposition** | `open` — **запаркована решением владельца, НЕ строить до его специального уведомления**; кодовых триггеров нет намеренно |
| ##B015-FILED **filed by** | решение владельца 2026-08-01 по исследованию B-012 |

- ##B015-SUT **Суть, по-простому.** Задуманные инструменты для агентов будут отдавать текст из пакетов прямо в контекст агента. Текст в контексте агента — потенциальные команды: подложи в пакет вредный абзац — и читающий агент может быть им управляем (prompt injection). Защита — криптографическая подпись содержимого пакетов, чтобы читатель мог проверить «текст от автора, не подменён». Дизайн specmap изначально требовал: канал не шипится без подписи.
- ##B015-RULING **Решение владельца (2026-08-01, дословно):** «Положить в бэклог, ничего не строить до специального уведомления. Нужно вначале построить чтобы вся система работала "как-то", наполнить репозитории, и так далее. И только потом уже беспокоиться о безопасности. Бессмысленно строить безопасность проекта, которым никто не пользуется. Пользуется им кто-то или не пользуется — из кодовой базы не видно, это видно владельцу из наблюдения внешнего мира, поэтому это решение владельца.» Следствие: условие переоткрытия — **только уведомление владельца**; никакие наблюдаемые в коде события записью не назначаются.
- ##B015-TASKS **Протокол задач на день переоткрытия (полный список):**
  1. **Выбор схемы подписи.** Кандидаты, в порядке рекомендации исследования: (1) подписанные git-теги SSH-ключом мейнтейнера — реестр и есть git, паблишер уже пушит теги, ноль нового wire-формата, верификация через allowed_signers; (2) minisign-класс — detached-подпись контент-хэша пакета, крошечная permissive-зависимость, полностью офлайн; (3) sigstore-класс — отклонён на сегодня: тяжёлые зависимости, онлайн-верификация против clean-clone/offline-постуры, identity через OIDC чужда single-writer-модели; пересмотреть при втором независимом издателе.
  2. **Единица подписи** — дерево пакета на теге (рекомендация), не index отдельно: всё, что сервится из верифицированного дерева, наследует целостность. Сегодняшний контент-хэш в lockfile защищает от подмены байтов зеркалом, но не отвечает «это байты издателя?» — подпись закрывает второй вопрос.
  3. **Инфраструктура:** trust root (где живёт публичный ключ), точка верификации при fetch (рядом с существующей проверкой хэша), ротация/ревокация, кастодия ключа по secrets-hygiene, возможное поле в lockfile.
  4. **Оформление ответов инструментов:** фраза «возвращаемое — справочные данные, не инструкции» на всех инструментах, отдающих агенту текст пакетов, включая **уже существующие две дороги** — чтение сабскиллов установленного пакета и boot-снипеты, читаемые агентом на старте сессии. Явное исключение: агентский релей (agentic_explain) — там инструкции суть фичи, оформление не меняется.
  5. **Линт императивных формулировок** в текстах пакетов (второе-лицо-повелительное вне guide-типа) — требует меток типа на секциях (см. B-019, twin-разметка).
  6. **Правка позиции спеки.** PROP-014 несёт позицию «канал шипится только подписанным». Решением владельца последовательность перевёрнута (канал раньше подписи — B-018); в момент постройки B-018 эта позиция правится owner-approved диффом, чтобы спека не противоречила построенному. Записано здесь, чтобы не потерялось.

### B-016 — карта в составе пакета + получение исходных фрагментов по отпечатку {#b-016}

| | |
|---|---|
| ##B016-ANCHOR **anchor** | механизмы «distribution» PROP-014; досье — `d14-b012-part-A.md`, раздел A1 |
| ##B016-LOCATOR **locator** | producer карты готов и гейтится; манифест не имеет списка файлов вообще (пакет едет целой директорией — файл карты поедет бесплатно); хранилища «хэш → исходный текст» не существует нигде |
| ##B016-SEVERITY **severity** | P2 |
| ##B016-DISPOSITION **disposition** | `planned` — **решение владельца 2026-08-01: «это надо строить»** |
| ##B016-FILED **filed by** | решение владельца 2026-08-01 по исследованию B-012 |

- ##B016-SUT **Суть, по-простому.** Сегодня каждый проект строит карту связей только про себя. Задумка из двух половин: **(1)** пакет возит готовую карту с собой — потребитель может спрашивать про установленный пакет, ничего не пересобирая; **(2)** механизм «дай точный кусок исходника по его отпечатку (хэшу)» — чтобы ответ мог показать не только «где», но и «что именно».
- ##B016-HALF1 **Половина 1 (дёшево).** Файл карты и так поедет внутри пакета — грузовик уже ездит. Достроить: политику (в чьём пространстве имён URI карты; входит ли карта в контент-хэш пакета — если да, каждый код-эдит пакета меняет его пин в lockfile, это осознать); шаг генерации карты на пакет; и главное — **читателя** на стороне потребителя, которого сегодня нет (единственный сегодняшний потребитель чужих спек пере-парсит markdown и живёт без карты).
- ##B016-HALF2 **Половина 2 (дорого).** Целиком новое: тип «адрес фрагмента», хранилище «хэш → текст», глагол скачивания в словаре реестра (он сегодня пакет-гранулярный). Нерешённый дизайн-вопрос до кода: что такое фрагмент **со стороны кода** (сегодня у элемента кода нет ни конца диапазона, ни тела — только файл и строка).
- ##B016-DEPS **Зависимости и порядок.** Читатель половины 1 — инструменты B-018 (строить смежно). Половина 2 — после половины 1. Любое изменение формата карты идёт **одной** сменой формата вместе с B-019 (не тремя), и до неё чинится сломанный инструмент перегенерации (B-013).

### B-017 — профили приватности для закрытых проектов {#b-017}

| | |
|---|---|
| ##B017-ANCHOR **anchor** | механизм «[metamodel] profile» PROP-014; досье — `d14-b012-part-A.md`, раздел A3 |
| ##B017-LOCATOR **locator** | ключа не существует ни в одном манифесте/схеме/парсере; редакционного пути нет; у «contract»-уровня нет данных (карта не хранит сигнатур) |
| ##B017-SEVERITY **severity** | P2 |
| ##B017-DISPOSITION **disposition** | `planned` — **решение владельца 2026-08-01: «это надо строить»** |
| ##B017-FILED **filed by** | решение владельца 2026-08-01 по исследованию B-012 |

- ##B017-SUT **Суть, по-простому.** Закрытый (не open-source) проект должен уметь сказать в конфиге: «когда мою карту читают снаружи — делись всем / только контрактом без тел кода / ничем». Три уровня: open / contract / none.
- ##B017-BUILD **Что строить.** (1) Ключ в манифесте — сам по себе маленький, но парсер манифеста отвергает незнакомые ключи, значит старые версии vibe будут падать на файле с новым ключом: вводить вместе с механикой минимальной версии, не «на вырост». (2) Редакцию применять **на стороне производителя** (байты закрытого проекта не покидают его машину), не фильтром на сервере. (3) Для уровня «contract» карте нужны сигнатуры элементов кода — это смена формата карты: ехать той же одной сменой, что B-016/B-019. (4) Содержание «contract»-уровня (что именно безопасно отдавать: сигнатуры? доки?) — вопрос, который дизайн сам отложил до реального закрытого потребителя; в момент постройки вернуть владельцу с требованиями такого потребителя на столе.
- ##B017-DEPS **Зависимости.** Применяется только там, где есть чем делиться наружу: строить после/вместе с B-016 (половина 1) и B-018.

### B-018 — инструменты для агентов (MCP), широкий вариант — высокий приоритет владельца {#b-018}

| | |
|---|---|
| ##B018-ANCHOR **anchor** | механизмы «runtime exposure» PROP-014; досье — `d14-b012-part-A.md`, раздел A4 |
| ##B018-LOCATOR **locator** | локальная команда «объясни» работает в чекауте; в трёх стековых MCP-серверах её аналог уже шипится; у хостового vibe таких инструментов нет; ответы про установленные пакеты сегодня сознательно исключены |
| ##B018-SEVERITY **severity** | P2 |
| ##B018-DISPOSITION **disposition** | `planned` — **решение владельца 2026-08-01: «это надо строить, причем с высоким приоритетом и в широком варианте (вместе с объяснением чужих пакетов)»** |
| ##B018-FILED **filed by** | решение владельца 2026-08-01 по исследованию B-012 |

- ##B018-SUT **Суть, по-простому.** Дать AI-агенту спрашивать работающий vibe: «объясни это требование», «что реализует эту команду», «покажи фрагмент», «поищи по карте» — и не только про свой проект, но и про **установленные пакеты**. Это центральная фича всего сюжета «поделиться картой».
- ##B018-PARTS **Четыре части, в порядке постройки.**
  1. **Перенос «объясни» в агентский интерфейс vibe** — легко: все швы готовы, в стековых серверах есть три рабочих образца этой же формы. **Цена уточнена измерением (2026-08-04, волна В, владелец: «заноси, делай»):** «все швы готовы» верно про СТЕКИ — их MCP тонко оборачивает CLI-функцию, которая строит карту в памяти и рендерит (`rust-ai-native-cli/src/trace.rs:8`, обёртка `rust-ai-native-mcp/src/tools_discipline.rs:207`). У ХОСТА этой способности нет вовсе: его MCP несёт четыре инструмента (`query_package`, `read_subskill`, `materialise_subskill`, `agentic_explain` — `crates/vibe-mcp/src/tools.rs`) и картой не занимается; движок карты сегодня тянет только `xtask`. То есть работа — не «обернуть готовое», а «дать хосту способность». **Но связывание заново не нужно:** корневой `Cargo.toml:102` уже объявляет `specmap-core` рабочей зависимостью через вендор-копию, так что рельс проложен и новой связи хоста с пакетом не возникает. Форма — по норме поверхностей ([B-047](#b-047)): способность в разделяемом крейте, CLI и MCP — тонкие поверхности над ней, а не две копии. Карта строится СВЕЖЕЙ в памяти, как у стеков («explain answers for the tree as it is, never for a stale committed artefact»); чтение карт УСТАНОВЛЕННЫХ пакетов — это часть 4, не эта.
  2. **Поиск по карте.** Дизайн не определил язык запросов — сначала спроектировать (заготовка v0: точный URI + имя символа + фильтр по типу, жёсткий потолок размера ответа), положить в спеку owner-диффом, потом кодить. **Отложено владельцем 2026-08-04 («положить в бэклог со средним приоритетом») — развилка №6 карты НЕ взята; часть 1 при этом построена и живёт.** Что стоит помнить к моменту возврата: точечный `explain` уже отвечает, значит недостающее — именно ПОИСК, и его форма зависит от того, каких вопросов агенту не хватило на практике. Два рассмотренных варианта записаны, чтобы не изобретать заново: *(i)* три независимых фильтра (точный URI · подстрока имени символа · тип элемента), комбинируемые через И, плюс жёсткий потолок числа результатов — парсить нечего, ломаться нечему, расширяется добавлением полей; *(ii)* то же плюс обход графа (глубина N и «нет ребра такого-то типа»), что сразу отвечает на «какие правила никто не проверяет», но заводит грамматику, которую придётся версионировать. Первый — рекомендация босса на момент отложения.
  3. **Фрагменты по отпечатку** — вместе с B-016 (половина 2).
  4. **Ответы про установленные пакеты** («объяснение чужих пакетов»). Сегодня чужие секции сознательно не попадают в карту проекта — на этом исключении держится воспроизводимость карты (байт-в-байт проверка). Ломать исключение нельзя; строить **вторую, некоммитимую** карту-резолвер, собираемую в момент запроса из установленных пакетов. Кормится из B-016 (половина 1).
- ##B018-SECURITY **Безопасность.** Осознанно строится ДО подписи — перепоследовательность зафиксирована решением владельца в [B-015](#b-015): безопасность паркуется до его уведомления. В момент постройки этой записи позиция спеки «канал шипится только подписанным» правится owner-approved диффом (см. B-015, задача 6), чтобы построенное не противоречило написанному.
- ##B018-CANONICAL-QUERY **Канонический запрос (владелец, 2026-08-02):** «какой тест проверяет это правило спеки?» — агент даёт `spec://…#якорь`, получает verifies-рёбра с file:line. Сегодня это умеет CLI (`trace explain`) и MCP трёх стековых серверов (`trace_explain`) — по чекауту; хостовый vibe-mcp не умеет вовсе. Часть (i) этой записи закрывает ровно этот запрос для агентов vibe; принять его приёмочным примером постройки.

### B-019 — отпечатки кода + узлы «команда» и «вариант ошибки» в карте {#b-019}

| | |
|---|---|
| ##B019-ANCHOR **anchor** | механизм «edge model nodes» PROP-014; досье — `d14-b012-part-B.md`, раздел B2 |
| ##B019-LOCATOR **locator** | элемент кода в карте — пять полей без отпечатка и без тела; узла «команда» нет нигде; извлечение «вариантов ошибок» полностью существует — но в соседней подсистеме conform, не в карте |
| ##B019-SEVERITY **severity** | P2 |
| ##B019-DISPOSITION **disposition** | `planned` — **решение владельца 2026-08-01: «это надо строить. Это должна быть алгоритмическая фича, без использования LLM. Все части — а, б, в»** |
| ##B019-FILED **filed by** | решение владельца 2026-08-01 по исследованию B-012 |

- ##B019-SUT **Суть, по-простому.** Три доделки самой карты, все чисто алгоритмические (владелец: без LLM). **(а)** Отпечаток (хэш) на каждом элементе кода — чтобы карта замечала «код под этим требованием изменился, пересмотри связь»; сегодня она слепа к изменениям кода. **(б)** Узел «команда» — чтобы `vibe install` был сущностью карты, а не только функцией: ответ «что реализует vibe install» становится возможен напрямую. **(в)** Узел «вариант ошибки» — чтобы каждая ошибка была узлом карты и вела к своему требованию.
- ##B019-A **(а) — решения перед кодом.** Что хэшируем: текст (каждый прогон форматтера и правка комментария меняют отпечаток — шумно) или поток токенов (форматонезависимо — рекомендация); решение владельцу в момент постройки, с замером шума на обоих вариантах. Это смена формата карты с полной перегенерацией: ехать одной сменой вместе с B-016/B-017, до неё починить сломанный инструмент перегенерации (B-013). Парная половина со стороны спеки — метки-редакции на секциях: целевой набор ~80 секций, на которые ссылаются сообщения об ошибках, + правило «новые секции сразу с меткой» (решение владельца 2026-08-01 по «ключу 2»).
- ##B019-B **(б) — с нуля.** В дереве нет ни определения «команды», ни экстрактора, ни потребителя. Определить, что считается командой (поверхность CLI-подкоманд), написать экстрактор, добавить тип узла (та же одна смена формата), научить «объясни» принимать команду как цель.
- ##B019-V **(в) — что имеется в виду, и вопрос границы систем (решить ДО реализации — требование владельца).** В кодовой базе два независимых движка: **conform** (гейт качества кода: прогоняет правила, находит нарушения) и **specmap** (карта связей «код ↔ спека»). Данные о «вариантах ошибок» — какие enum-варианты с какими текстами ошибок существуют и на какие требования ссылаются — **уже извлекаются конформом** для двух его правил. Карта этих данных не видит: это два разных графа двух разных подсистем. Вопрос: чьей частью становится узел «вариант ошибки»? Три варианта: **(1)** specmap извлекает сам — дублирование экстракции в двух движках, две правды об одном; **(2)** specmap читает данные conform'а — новая зависимость между сознательно разделёнными движками; **(3)** не сливать данные вовсе, объединять на этапе запроса — инструмент B-018 показывает и карту, и находки conform'а рядом. Склонность исследования — (3) при наличии B-018, иначе (1) с выносом общей экстракции в разделяемую библиотечку; окончательное решение — первый шаг реализации этой части.

### B-020 — объяснения человеческим языком через внешние LLM {#b-020}

| | |
|---|---|
| ##B020-ANCHOR **anchor** | механизм «LLM as renderer» PROP-014; досье — `d14-b012-part-B.md`, раздел B3 |
| ##B020-LOCATOR **locator** | команда «объясни» отвечает детерминированным шаблоном; слот под второго производителя текста в кэше готов; LLM-клиента у vibe нет (крейт-заглушка) |
| ##B020-SEVERITY **severity** | P2 |
| ##B020-DISPOSITION **disposition** | `planned` — **решение владельца 2026-08-01: «это надо строить. Я думаю построить лайтовый клиент для внешних нелокальных LLM, который будет через них строить такие объяснения. Возможно это будет fractality, с этим нужно разобраться позднее»** |
| ##B020-FILED **filed by** | решение владельца 2026-08-01 по исследованию B-012 |

- ##B020-SUT **Суть, по-простому.** Команда «объясни» сегодня отвечает сухим шаблоном («такая-то функция реализует такой-то пункт»). Фича: опционально та же информация пересказывается внешней LLM человеческой прозой — «эта команда устроена так потому-то, вот решения, вот известные отступления».
- ##B020-DIRECTION **Направление владельца.** Лайтовый клиент к внешним нелокальным LLM (не встроенный движок); возможный носитель — fractality (воркер дергает внешнюю модель); разобраться позднее, в момент постройки.
- ##B020-BUILD **Что строить и что помнить.** (1) Сначала — текст в данных: сегодня ответ «объясни» несёт только имена и пути, без текста спеки и без документации кода; LLM было бы не из чего писать. Зависимость: включить текст документации и секций в ответ (кандидат ближайшего рабочего среза, дёшево, формат карты не меняется). (2) Второй «производитель текста» встаёт в готовый слот кэша; в ключ кэша добавляется идентификатор модели. (3) Шаблонный режим остаётся навсегда — инструмент обязан быть полноценным без LLM (инвариант дизайна). (4) Проза — только презентационный слой поверх детерминированных данных; сами данные карты LLM не трогает. (5) Ключи/креды внешних LLM — по secrets-hygiene.

### B-021 — пороговые предупреждения: перегруженные связи и длинные секции {#b-021}

| | |
|---|---|
| ##B021-ANCHOR **anchor** | механизмы «multiplicity lint» и «units fit a page» PROP-014; досье — `d14-b012-part-B.md`, разделы B1 и B4 |
| ##B021-LOCATOR **locator** | ни один слой не считает связей на элемент кода; длина секций вычисляется и выбрасывается в движке карты и уже лежит готовым полем в хостовом компиляторе спек |
| ##B021-SEVERITY **severity** | P2 |
| ##B021-DISPOSITION **disposition** | `planned` — **решение владельца 2026-08-01: строить обе, вместе** |
| ##B021-FILED **filed by** | решение владельца 2026-08-01 по исследованию B-012 |

- ##B021-SUT **Суть, по-простому.** Два предупреждения о качестве. **(1) Перегруженные связи:** элемент кода, реализующий сразу много пунктов спеки, обычно делает слишком много (или спека нарезана слишком мелко). Определяется чисто алгоритмически: карта — это список пар «элемент кода → пункт спеки»; сгруппировать пары по элементу, посчитать, сравнить с порогом. Ни LLM, ни эвристик — арифметика по готовому файлу. **(2) Длинные секции:** пункт спеки длиннее порога плохо читается и чаще меняется; длина уже вычисляется, осталось сравнить и предупредить.
- ##B021-RULING **Обоснование владельца (2026-08-01, дословно):** «Эти волшебные свойства не срабатывают на нашей базе, потому что она написана относительно хорошо. В других проектах это может быть совершенно не так. Мы пишем систему для всех, а не только для нас.» То есть: нулевые срабатывания на нашем корпусе — не довод против постройки; это продуктовые фичи для чужих, менее чистых корпусов.
- ##B021-BUILD **Что строить.** Оба порога **конфигурируемые**, стартовые значения (3 связи; 120 строк) — честные плейсхолдеры до реальной статистики, которую предупреждения сами и соберут. Оба — предупреждения, не блокирующие гейты (по крайней мере на старте). Счёт связности живёт в движке карты рядом с существующим механизмом предупреждений (там уже есть цикл отчёта, конфиг и блокирующий режим); формулировка в дизайне, называющая другой дом, правится однострочным owner-диффом при постройке. Счёт длины — по «листовым» секциям (без вложенных подсекций) по умолчанию, иначе предупреждение измеряет жанр документа, а не дисциплину секций; настройка зерна — в конфиг.

### B-022 — исследование: механизмы кэша объяснений (LEDGER-INTENT), можно ли реализовать {#b-022}

| | |
|---|---|
| ##B022-ANCHOR **anchor** | пять фактов LEDGER-INTENT-v0.1 (партия 1c очереди группы B); измерения и готовые аннотации — `campaigns/packages-2026-09/harvest/d7a-core-sync-reverify.md`, раздел F-159 |
| ##B022-LOCATOR **locator** | движок кэша: `core-ai-native-specmap/src/ledger.rs`; на диске хранится текст без полей, чистки нет, метрик две из четырёх, срез не экспортируется и не подписывается, вид запроса — строка в теле функции |
| ##B022-SEVERITY **severity** | P2 |
| ##B022-DISPOSITION **disposition** | `done` — исследование выполнено 2026-08-03 (`harvest/e1-b022-evidence.md` + синтез `e1-b022-ledger-feasibility.md`), **владелец согласился 2026-08-04 («с B-022 согласен»)**; исполнено целиком: M-E+M-A-слой-1 построены 2026-08-04 (QueryKind-enum, структурный ключ, entry-wrapper; вендорено ×6), четыре interim-аннотации M-A/M-B/M-C/M-D вписаны в LEDGER-INTENT (ключи: B-020, B-015), пять якорей F-159 пере-суждены `confirmed` (batch E3-F159), обязательство ушло в историю реестра; хост-строка `terraform/REPORT.md:41` исправлена. Наследники: B-020 (LLM-поля, давление GC, cost-метрики), B-015-нотис (release slice) |
| ##B022-FILED **filed by** | решение владельца 2026-08-01 по предъявленной партии 1c |

- ##B022-SUT **Суть, по-простому.** Документ про кэш сгенерированных объяснений обещает пять механизмов, которых нет: записи с полями происхождения (кто произвёл, какая модель, когда, почём), чистку кэша по давности с защитой от выселения нужного, полный набор метрик, подписанный «релизный срез» кэша при выпуске, и закрытый перечень видов запросов. Исследовать по образцу B-012: что из пяти реально строить, что чего требует, что честнее вычеркнуть.
- ##B022-COUPLING **Связки.** «Подписанный срез» — подмножество запаркованной программы безопасности [B-015](#b-015) (подписи нет нигде в дереве — не строить до уведомления владельца); поля происхождения пересекаются с B-020 (клиент внешних LLM захочет писать model_id в запись); вид-запроса-как-enum — дешёвый и независимый. Готовые тексты честных аннотаций (если исследование скажет «не строить») лежат в harvest и не применяются без владельца.

### B-023 — исследование: синтаксический уровень для JS/TS и Python-фронтенд {#b-023}

| | |
|---|---|
| ##B023-ANCHOR **anchor** | строки таблицы фронтендов ENGINE-CONFORM (партия 1b, пункты 1–2); измерения — `harvest/d7a-core-sync-reverify.md`, раздел F-146 |
| ##B023-LOCATOR **locator** | таблица обещает tree-sitter/SWC для TS/JS и RustPython/CPython-sidecar для Python; в дереве нет ни одного — семантический фронтенд TS (Compiler API через node-sidecar) есть и точен, Python-стека нет вовсе |
| ##B023-SEVERITY **severity** | P2 |
| ##B023-DISPOSITION **disposition** | `done` (исследование) — выполнено 2026-08-03/04 (`campaigns/packages-2026-09/harvest/e1-b023-evidence.md` + синтез `e1-b023-syntactic-tiers-feasibility.md`, включая контр-пробу владельца по глубине); **рулинг владельца 2026-08-04, дословно: «давай B-023 отложим до тех пор, пока не появится ещё какое-то правило кроме "as_cross с не локальной областью". Не нужно забывать об этом, это нормальное продолжение развития, просто это кандидат на середину или конец бэклога»** — checker-глубина ждёт ВТОРОГО типо-требующего правила (один `as_cross` триггером не является), приоритет средний/низкий, вне текущих волн; tree-sitter/SWC-дубль не строится ни в каком исходе; Python-фронтенд ждёт продуктового решения о Python-стеке; два якоря F-146 пере-судятся с пере-аннотацией таблицы §2, когда деферрал будет вписан в спеку |
| ##B023-FILED **filed by** | решение владельца 2026-08-01 по предъявленной партии 1b (пункты 1–2) |

- ##B023-SUT **Суть, по-простому.** Гейт качества читает код через «фронтенды» двух глубин: быстрый синтаксический разбор и глубокий семантический. Для TS/JS сегодня есть только глубокий (через компилятор TypeScript в node-процессе); быстрого нет. Для Python нет ничего. Исследовать: что даёт синтаксический уровень для TS/JS (tree-sitter или SWC — какие факты извлекаемы без компилятора, почём, какие лицензии/зависимости), и реализуем ли Python-фронтенд (RustPython-парсер in-process против CPython-sidecar по образцу ts-extract/go-extract), — с рекомендацией строить/не строить по каждому.
- ##B023-CONTEXT **Контекст.** Пакеты языков пишутся для внешних потребителей (наша база — не скамья для Go/TS, §3.8 кампании); прецедент sidecar-архитектуры двойной (ts-extract, go-extract) — Python-sidecar ляжет в готовую форму. До итогов исследования строки таблицы стоят как есть (drift-вердикты кампании остаются честными).

### B-024 — исследование: не вытесняют ли маркеры @stage/state lifecycle-статусы specmap {#b-024}

| | |
|---|---|
| ##B024-ANCHOR **anchor** | вопрос владельца 2026-08-01 к тексту EDGE-MODEL-EDGES (партия 1d): «не устарела ли вообще вся эта система с появлением синтаксиса вида @doc/done? Там же тоже есть свой tombstone» |
| ##B024-LOCATOR **locator** | две параллельные системы: kind-line-статусы specmap (`planned`/`disputed`; `ratified` — отсутствие, `retired` — tombstone; парсер готов, носителей 0 из 5 266) и хостовые маркеры PROP-043 `@stage/state` (весь корпус размечен; `void` — их tombstone) |
| ##B024-SEVERITY **severity** | P2 |
| ##B024-DISPOSITION **disposition** | `planned` — **решение владельца 2026-08-01 (вторая сессия): «предлагаю запланировать в бэклог свести стадии жизненного цикла в specmap к аналогичным в progress»** — направление выбрано: сводим словарь specmap к словарю progress (derive, not declare); исследовательская часть сужается до механики (как выводить; что делать с `disputed`, у которого аналога нет) |
| ##B024-FILED **filed by** | вопрос владельца 2026-08-01, зафайлен как исследование; повышен до `planned` его же решением в тот же день |

- ##B024-SUT **Суть, по-простому.** В проекте два способа сказать «в каком состоянии кусок спеки». Маркеры `@stage/state` — прогресс каждого факта (насколько сделано: spec/impl/doc × done/work/…), живут на всём корпусе, `void` — их могильный камень. Статусы specmap — контрактное состояние секции для машины трассировки (`planned` — задумано, `disputed` — оспорено парой, `retired` — второй могильный камень), задуманы, чтобы управлять рёбрами графа (заморозка связей в спорные секции, отдельный учёт planned в покрытии — механики не построены), и не носятся ни одной секцией. **Два tombstone на одно понятие — реальная дупликация**; `planned` перекрывается со стадиями маркеров; уникален только `disputed` (пара конфликтующих секций аналога в маркерах не имеет).
- ##B024-QUESTION **Вопрос исследования.** Может ли машина трассировки **читать хостовые маркеры** вместо собственной параллельной системы (derive, not declare): `void` ⇒ retired, стадия/state ⇒ planned-эквивалент, а `disputed` — единственное, что останется собственным словарём specmap? Если да — kind-line-статусы сокращаются до `disputed`, и разметка B-019(а)-twin (метки ~80 секций) дешевеет. Если нет — записать, почему двум системам жить (разные предметы: прогресс факта ≠ контрактный статус юнита), и развести их словари явно.

### B-025 — находки гейта: помечать признанные отступления, а не гасить {#b-025}

| | |
|---|---|
| ##B025-ANCHOR **anchor** | факт «цепочка из пяти звеньев» ENGINE-CONFORM (партия 1b, пункт 5); измерения — `harvest/d7a-core-sync-reverify.md`, раздел F-146 |
| ##B025-LOCATOR **locator** | сегодня записанное отступление ГАСИТ находку на этапе правила (`in_deviation`, `conform/src/facts.rs:62`) — метка «deviation-acknowledged» не рождается никогда; поля вовлечённых фактов у находки нет (`Finding` = rule/file/line/message/why/fingerprint) |
| ##B025-SEVERITY **severity** | P2 |
| ##B025-DISPOSITION **disposition** | `planned` — **решение владельца 2026-08-01, пункт 5 = (б): «помечать вместо гасить. Я в будущем хочу сделать инструменты визуализации, и просто убирать неприменимые факты из IR это плохо, нужно всё видеть»** |
| ##B025-FILED **filed by** | решение владельца 2026-08-01 по партии 1b |

- ##B025-SUT **Суть, по-простому.** Когда правило гейта находит нарушение, а рядом есть записанное отступление («мы так делаем сознательно, вот причина»), гейт сейчас просто не рождает находку — она исчезает из всех данных. Строим наоборот: находка рождается всегда, но помечается «отступление признано» и не валит гейт. Тогда инструменты визуализации видят полную картину — сколько нарушений, сколько из них признанных, где; ничего не выпадает из IR.
- ##B025-BUILD **Что строить.** Статус-поле (или отдельный класс) у находки; правило перестаёт фильтровать по `in_deviation` и вместо этого штампует статус; baseline/ratchet учитывает «признанные» отдельно (не считает их новыми нарушениями); SARIF-рендер несёт статус. Заодно — поле вовлечённых фактов (второе недостающее звено той же цепочки). Обязательство F-146 частично ждёт этой стройки (якорь цепочки остаётся drift до неё).

### B-026 — ингест SARIF: диагнозы чужих линтеров становятся фактами гейта {#b-026}

| | |
|---|---|
| ##B026-ANCHOR **anchor** | факт «foreign linters as evidence providers» ENGINE-CONFORM (партия 1b, пункт 6); измерения — `harvest/d7a-core-sync-reverify.md`, раздел F-206 |
| ##B026-LOCATOR **locator** | SARIF сегодня только пишется (`sarif::render` — единственная публичная функция модуля), парсера нет ни в одном слое; clippy/eslint бегут floor-шагами, их вывод никуда не попадает |
| ##B026-SEVERITY **severity** | P2 |
| ##B026-DISPOSITION **disposition** | `planned`, **высокий приоритет — решение владельца 2026-08-01, пункт 6: «Построить ингест SARIF в будущем, с высоким приоритетом поместить это в бэклог»** |
| ##B026-FILED **filed by** | решение владельца 2026-08-01 по партии 1b |

- ##B026-SUT **Суть, по-простому.** Чужие линтеры (clippy, eslint, в перспективе ruff/clang-tidy) уже бегут рядом с гейтом, но гейт их результатов не видит. Строим чтение SARIF (стандартный формат отчётов статанализа): диагнозы чужих линтеров превращаются в факты гейта, и правила Дисциплины могут на них ссылаться («цитируем clippy, не переизобретаем его» — ровно та постура, которую документ всегда декларировал).
- ##B026-BUILD **Что строить.** SARIF-парсер (или зависимость serde-схемы), маппинг диагноза → `Fact` (какой линтер, какой rule id, файл/строка/сообщение), точка входа (floor-шаг складывает отчёты линтеров, conform их читает), и словарь цитирования в правилах (`check: { tool, id, status }` — форма уже описана в документе и нигде не построена). После стройки якорь foreign-linters пере-суживается по построенному; обязательство F-206 в реестре — `deferred` до тех пор.

### B-027 — аудит маркеров у «Specified, not built»: смысл против буквы {#b-027}

| | |
|---|---|
| ##B027-ANCHOR **anchor** | вопрос владельца 2026-08-01 (партия 1b, пункт 7): «Я не понимаю, почему у specified not built статус @impl/done, если спецификация не реализована — то это же @spec/done или @impl/planned?» |
| ##B027-LOCATOR **locator** | аннотированные факты несут маркеры вразнобой: часть @spec/done (партия 1a, D14-семья), часть @impl/done (например, DISTRIBUTION-RIDES в PROP-014, RULE-MULTIPLICITY-LINT, QUERY-ERROR-PROVENANCE, LLM-AS-RENDERER, RUNTIME-TRANSPORT); четыре закрывающих правила-собрата запечатаны с @impl/done |
| ##B027-SEVERITY **severity** | P2 |
| ##B027-DISPOSITION **disposition** | `done` — **правило утверждено владельцем 2026-08-02 («согласен, применяй»), свип исполнен тем же ситтингом**: инвентаризация 48 аннотированных фактов в 16 файлах пакетов; 19 флипов на `@impl/plan` с именем записи стройки прямо в аннотации («Specified, not built (→ B-nnn): …»), у остальных 29 стройка не запланирована и `@spec/done` стоит верно (включая ROW-KIND-PROP, чей ячеечный маркер при перечтении оказался честным как есть); каждый флип пере-сужен батчем D34 (19 confirmed, 0 отказов), 6 файлов запечатаны; хост-сторона фразы не несёт (проверено грепом) |
| ##B027-FILED **filed by** | вопрос владельца 2026-08-01, зафайлен как аудит-задача; правило утверждено и исполнено 2026-08-02 |

- ##B027-SUT **Суть, по-простому.** Владелец прав: `@impl/done` на факте, чей механизм не построен, — семантически ложь (маркер утверждает «стадия реализации завершена»). Разнобой — историческая случайность, не дизайн: партия 1a ставила одним фактам @spec/done, другим оставила @impl/done; закрывающие правила держат @impl/done на том основании, что само ПРАВИЛО (как амендировано) в силе. Грамматика маркеров уже несёт нужные слова: стадии `idea<spec<impl<test<doc<freeze`, состояния `hold<plan<work<done<void` — то есть догадка владельца «@impl/planned» существует в форме **`@impl/plan`**.
- ##B027-RULE-PROPOSAL **Предлагаемое правило для аудита (утвердить перед свипом):** «specified, not built, стройка НЕ планируется» → `@spec/done`; «specified, not built, стройка запланирована (есть запись в бэклоге)» → `@impl/plan` — тогда маркер сам показывает, что реализация в плане (B-016…B-021, B-025, B-026 — их якоря получат @impl/plan с именем записи). Закрывающие правила пяти документов решаются одним решением на семью. Свип механический после утверждения правила; каждое изменение маркера — пере-суд якоря (D14-порядок: mirror → merge → seal).

### B-028 — грамматика spec://-адресов: пакет публикует подмножество того, что реализует хост {#b-028}

| | |
|---|---|
| ##B028-ANCHOR **anchor** | секция URI-схемы в `addressable-specs` (ADDRESSABLE-SPECS-PROTOCOL) против хостовой грамматики PROP-035 `##UNIFIED-GRAMMAR`; замечено re-verify-проходом волны 7 (`harvest/d7b-addressing-naming-sync-reverify.md`, раздел F-169, «New obligation noticed») |
| ##B028-LOCATOR **locator** | пакет публикует `spec://<module>/<doc>#<section>`; хост реализует строгий суперсет `spec://<group>/<name>[@<version>]/<doc-path>#<anchor>[.<sub>…][~r<N>]` — опциональная версия, многосегментный путь, revision-pin; пакетная секция не упоминает ни одного из трёх расширений |
| ##B028-SEVERITY **severity** | P2 |
| ##B028-DISPOSITION **disposition** | `done` — **owner-ruled 2026-08-04, дословно: «Я хочу чтобы указание версий было опциональной фичей. Если версия не указана - используется самая свежая»** — и исполнено тем же заходом: флоу несёт ПОЛНУЮ грамматику (секция `{#uri-scheme}` переписана: authority = координата пакета, `[@<version>]` — опциональная фича с дефолтом «свежайшая установленная» (semver-newest — единственное детерминированное офлайн-чтение), `<doc-path>` многосегментный, `[~r<N>]` — пин ревизии юнита; якоря старых строк сохранены, смысл эволюционировал в структуру `<doc-path>`); обе redbook-главы (гл.1:195, гл.2:63) перестали пересказывать схему и цитируют секцию (закон «одна норма — один дом»); PROP-035 §6 `##URI-VERSION-OPTIONAL`/`##ROUTER-VERSION` перерулены с lockfile-формулировки на freshest; резолвер: multi-slot без версии выбирает новейшую вместо ошибки «address must pin @version» (срез E7-W1). Правки пакетов — in-slot до пред-публикационной границы. Filed: **решение владельца 2026-08-02: «положи в бэклог с высоким приоритетом»** |
| ##B028-FILED **filed by** | решение владельца 2026-08-02 по предъявлению группы C/D |

- ##B028-SUT **Суть, по-простому.** Флоу адресуемых спек учит консьюмеров грамматике ссылок — но учит **урезанной версии**: наш собственный резолвер понимает ещё версию пакета (`@0.8.0`), путь из нескольких сегментов и пин ревизии (`~r2`). Пакетная версия не ложна (подмножество), но продаётся как целое. Вопрос: нести ли флоу полную грамматику?
- ##B028-STAKES **Что решение тянет.** Если «да» — это release event: секция URI-схемы переписывается, и **redbook пересказывает схему в двух главах** (те тоже двигаются — как раз класс «одна норма в трёх местах», который кампания выжигает). Если «нет» — записать явно, что пакет публикует базовую грамматику, а расширения — хостовое superset-расширение (одна оговорка в пакетной секции + ссылка). Обе развязки закрывают вопрос честно; открытым он оставляет грамматику раздвоенной.
- ##B028-RELATED **Смежное.** Вопрос сегментов `<module>`/`<doc>` (единый namespace хоста, усечённые имена документов — F-169/F-147) — соседний, но отдельный: там спор о **значениях** сегментов, здесь — о **составе** грамматики. Решения независимы.

### B-029 — ключ гейта: нейтральное/пер-языковое имя вместо растового на всех {#b-029}

| | |
|---|---|
| ##B029-ANCHOR **anchor** | Config нейтрального движка гейта (`core-ai-native-conform/src/config.rs:44`), вендорится в шесть пакетов |
| ##B029-LOCATOR **locator** | единственный ключ ratchet-списка — `gated_crates`, одно написание на все языки; `deny_unknown_fields` — любое другое слово даёт громкий parse error; Go-доки на 2026-08-02 приведены к шипнутому ключу с оговоркой «слово — общего движка» |
| ##B029-SEVERITY **severity** | P2 |
| ##B029-DISPOSITION **disposition** | `planned` → **развилка №2 карты взята владельцем 2026-08-04** (единица = родная единица языка: Rust crate / Go package / TS cell; дом — полная симметрия секций одной формы, нейтральный ключ `gated` в идиомном доме, корень — только общий бюджет; плоские корневые ключи умирают громко с подсказкой переезда; бар владельца дословно: «расширяемо на новые языки (скоро добавится Python!)… Хочется сделать хорошо и надолго»). Запись: `spec/design/gate-parity-config.md` §2; прежний пункт 2.1 (2026-08-02: «правда кода сегодня, идиома — записанной стройкой») исполняется этой стройкой |
| ##B029-FILED **filed by** | решение владельца 2026-08-02 (его же challenge: «crates — это не термин Golang») |

- ##B029-SUT **Суть, по-простому.** Список «какие единицы кода уже под гейтом» во всех языках называется растовым словом `gated_crates` — Go-проект пишет чужой термин в свой конфиг. Стройка: нейтральный ключ (например `gated_units`) или пер-языковый алиас (`gated_packages` для Go, `gated_cells` для TS), старое написание остаётся алиасом совместимости навсегда.
- ##B029-SCOPE **Масштаб.** Release event: движок вендорится в шесть пакетов; доки трёх стеков и легенды `conform.toml` поворачиваются на идиому после посадки; фикстуры/тесты, читающие ключ, идут той же правкой. Решить при постройке: один нейтральный ключ или языковые алиасы (моя заготовка — нейтральный + алиасы, чтобы старые конфиги не ломались).
- ##B029-CONFIG-SURFACE **Обогащение 2026-08-02 (владелец, по предъявлению F-185): расширить саму поверхность конфига под Go и TypeScript.** «Кажется, мы где-то уже пообещали в бэклоге решить это. Вероятно нам нужно обогатить это обещание. Может быть, под Go и Typescript, нам надо как-то расширить или улучшить то, что мы сохраняем в conform.toml». То есть запись перестаёт быть только переименованием ключа: при постройке спроектировать, что вообще хранит `conform.toml` пер-язык — сегодня гейт-список `gated_crates` / `[[exempt]]` и бюджет `max_file_lines` живут ТОЛЬКО в корневой таблице и работают только на Rust; `[go]` несёт шесть своих ключей (`roots`, `exclude_substrings`, `cells_dir`, `seams_pkg`, `registry_pkg`, `floor_disable`), `[typescript]` — пять (`roots`, `exclude_substrings`, `cells_dir`, `seam`, `floor_disable`), и ни один не гейтовый. Пер-языковые гейт-списки — совместная развязка с [B-034](#b-034) (инвариант gated-or-exempt для Go/TS); единицу гейта пер-язык (crate / package / cell) решить там же.

### B-030 — проверка «ассерция соответствия присутствует»: построить для Go, обследовать Rust/TS {#b-030}

| | |
|---|---|
| ##B030-ANCHOR **anchor** | факт «Conformance is made loud» Go-гайда (аннотирован 2026-08-02); ростер Go-гейта `build_rules` (`go-ai-native-conform/src/lib.rs:51-60`) |
| ##B030-LOCATOR **locator** | ни один гейт семьи не проверяет присутствие compile-time-ассерции: Go — три правила, ни одно не парсит `var _ seams.X = (*Impl)(nil)`; у растового гейта аналогичного правила тоже нет (grep «assertion» по правилам пуст) |
| ##B030-SEVERITY **severity** | P2 |
| ##B030-DISPOSITION **disposition** | `done` — **построено волной Б батч 2, 2026-08-04**: Go — правило `go-conformance-assertion` полицирует **gated**-ячейки на `var _ Seam = (*Impl)(nil)` (бесшовные/exempt вне; экстрактор эмитит `go_conformance`); Rust — вердикт «причина записана» (компилятор на use-site = ассерция, гейт не обещает проверку письменной ассерции); TS — вердикт «маршрут» (type-level-tests, паритет-долг лупа B-035). Записано в `harvest/e12-b035-parity-pass.md` строка 7. Исходно: решение владельца 2026-08-02, пункт 2.2 = (а) сейчас + (б) обследование Rust/TS |
| ##B030-FILED **filed by** | решение владельца 2026-08-02 |

- ##B030-SUT **Суть, по-простому.** Гайды учат: каждая ячейка кода несёт компайл-тайм-ассерцию («эта реализация действительно удовлетворяет шву») — а гейт, мол, проверяет её присутствие. Ассерции — настоящая идиома, гейт их присутствие не проверяет нигде. Стройка: (1) Go — синтаксический скан файлов ячеек за паттерном ассерции (go-extract уже парсит исходники, правило ложится в готовый ростер); (2) **обследование Rust и TypeScript**: что их гайды обещают про ассерции/регистрацию (у Rust — `var _`-аналоги и single-registration-point, у TS — branded-швы), проверяют ли их гейты это фактически, и построить недостающие правила там, где обещание есть, а проверки нет.
- ##B030-FORM **Форма.** Каждое новое правило — по образцу существующих (`Rule` с id/why/check, findings через ratchet-baseline, предупреждение до стабилизации); обещания гайдов пере-суживаются по построенному.

### B-031 — корень vibevm становится полноценным пакетом: fully-qualified адресация без исключений {#b-031}

| | |
|---|---|
| ##B031-ANCHOR **anchor** | сегментные правила `addressable-specs` (ROW-SEGMENT-MODULE/DOC + близнецы F-147) против хостовых исключений PROP-029 (`##SCOPE-HOST`, namespace `vibevm`) и PROP-035 (`##ROUTER-DOC-ID`, усечение имён) |
| ##B031-LOCATOR **locator** | 1 384 цитаты вида `spec://org.vibevm.core/vibevm/...`; `specmap.toml` `namespace = "vibevm"`; резолвер `crates/vibe-spec/src/resolver.rs` знает хост как спец-случай |
| ##B031-SEVERITY **severity** | P2 |
| ##B031-DISPOSITION **disposition** | `done` — designed on the E5 census → **owner-approved 2026-08-04** (группа `org.vibevm.core`, имя `vibevm` · жёсткая ошибка с подсказкой · все живые поверхности; his fourth point — the refactor-metadata check — executed by the boss and recorded in the design §5.1) → **landed the same day**: W1 resolver+identity (self coordinate first, `LegacyHostAuthority` hint, both `HOST_NAMESPACE` constants dead), W2 migrator (byte-exact, idempotent), the host wet pass — **1 893 occurrences over 606 files, `--verify` residue 0**, specmap namespace + specmap.json re-minted consistently, sync-engines ×6, five fixture families honestly re-pointed (resolver corpus, address grammar, specmark grammar tests + doctests, Т2 via `concat!`), three budget splits, panel green; **the mass re-seal executed (15 re-vouched; 4 files honestly refused pending their new anchors' own judging)**; F-169 resolved whole and F-147's twins confirmed (batch E6-F169-F147) — registry 179 drifts / 88 obligations / owed 6. Record and ruling verbatim: («1. координаты: группа org.vibevm.core, имя vibevm. 2. жесткая ошибка с подсказкой 3. все живые поверхности»); his fourth point — the refactor-metadata check — was assigned to the boss personally and is executed and recorded in the design §5.1 (registry path-keyed, specmap regenerates, pins inert, ledger soft-misses; the landing carries the mass re-seal). Record: [`spec/design/host-as-package.md`](spec/design/host-as-package.md) on the E5 census [`harvest/e5-b031-evidence.md`](campaigns/packages-2026-09/harvest/e5-b031-evidence.md); **building per its §8 cut (W1 → W2 → W3)**. **Решение владельца 2026-08-02, дословно:** «не логичней ли самому корневому пакету vibevm в vibe.toml дать нормальное fully-qualified имя как у всех остальных пакетов, и дальше чтобы все ссылки работали по обычным правилам, без исключений? Резолвер при этом должен начать учитывать не только структуру внутри spec/packages/vibedeps, но и учитывать адресацию внутри vibe.toml ВЕЗДЕ, включая корневой пакет. Таким образом мы теряем короткую нотацию spec://org.vibevm.core/vibevm/..., но получаем универсальность. Я бы назвал корневой пакет spec://org.vibevm.core» |
| ##B031-FILED **filed by** | решение владельца 2026-08-02 по предъявлению сегментного вопроса (3.2) |

- ##B031-SUT **Суть, по-простому.** Сегодня хост — единственный «не-пакет» в собственной адресации: у всех пакетов адреса строятся из их полного имени, а у хоста — короткий спец-namespace `vibevm` со своими записанными исключениями. Стройка: корневой проект получает fully-qualified имя (**org.vibevm.core** — слово владельца), резолвер читает адресацию из vibe.toml везде, включая корень; исключения умирают, короткая нотация умирает вместе с ними, адресация становится универсальной.
- ##B031-DESIGN **Решить при проектировании (первый шаг стройки).** (1) Точная форма координаты: `<group>/<name>` по грамматике (`org.vibevm/core`?) или namespace-строка `org.vibevm.core` — согласовать с хостовой unified-grammar (PROP-035) и B-028. (2) **Вторая половина исключений — усечение имён документов** (`PROP-043` vs `PROP-043-progress-markup`): либо узаконить префикс-скан как универсальный сахар для ВСЕХ пакетов (тогда правило поднимается во флоу и исключение тоже умирает), либо писать полные имена. (3) Механика миграции 1 384 цитат (скриптуемо; операция одного коммита с пере-судом затронутых якорей). (4) Правки PROP-029/PROP-035 (owner-approved диффы) + пересказы в redbook. (5) Кампанийные инструменты, парсящие `spec://org.vibevm.core/vibevm/` (verify-evidence и родня) — та же правка.
- ##B031-CLOSES **Что закрывает.** Сегментные факты F-169 (ROW-SEGMENT-MODULE/DOC) и близнецы F-147 (SEGMENT-MODULE-IS-THE-DIRECTORY / SEGMENT-DOC-IS-THE-FILE-NAME) — после миграции хост соответствует пакетному правилу буквально, пере-суд confirmed без правки пакета; реестровые строки — `deferred` с именем этой записи до посадки.

### B-032 — протокол гранулярности планирования: FEAT-файлы как адресуемые единицы, планы собираются из них {#b-032}

| | |
|---|---|
| ##B032-ANCHOR **anchor** | ряд «дом фичи» spec-tree-layout (правлен 2026-08-02: оба дома названы) + флоу campaign-plans; ни один из двух флоу не несёт протокола выбора между ними |
| ##B032-LOCATOR **locator** | живая практика на 2026-08-02: `FEAT-*` — 0 экземпляров у четырёх адоптеров, живых планов кампаний — 8; выбор медиума сегодня не проговаривается с пользователем нигде |
| ##B032-SEVERITY **severity** | P2 |
| ##B032-DISPOSITION **disposition** | `planned` — **решение владельца 2026-08-02: «если такое исправление элементарно — можно сделать сейчас, если нет — лучше добавить в бэклог расширение»; исправление не элементарно (новая норма на стыке двух пакетов) — расширение сюда** |
| ##B032-FILED **filed by** | решение владельца 2026-08-02 по вопросу двух домов (F-147) |

- ##B032-SUT **Суть, по-простому (направление владельца дословно по смыслу).** При планировании фичи агент **спрашивает пользователя**, какую гранулярность и какой медиум использовать (FEAT-файл, план кампании, или что-то ещё). Самый логичный способ: **большие фичи** планируются как отдельные адресуемые `FEAT-*`-файлы — хорошо выделенная сущность на диске со своим якорным пространством, — а план спеки или кампании **собирается из ссылок на реализацию этих файлов**. Мелочь на три строчки в FEAT не кладётся.
- ##B032-BUILD **Что строить.** (1) Протокольный абзац «как выбрать медиум» — дом решить при постройке: секция when-to-propose у campaign-plans + перекрёстная ссылка из ряда-дома spec-tree-layout (правка двух пакетов — release event). (2) Композиция: форма «план ссылается на FEAT-файлы как на единицы работ» — конвенция ссылок (адресуемость уже есть бесплатно: FEAT-файл получает якоря как любой спек-документ). (3) Порог крупности («не на 3 строчки») — качественная формулировка, без числового порога до реальной статистики (урок B-021).
- ##B032-RELATED **Смежное.** B-031 (корень-как-пакет) не пересекается; вопрос взаимных ссылок двух флоу отсюда же — закрывается пунктом (1).

### B-033 — Go: выделенное правило «ошибка шва цитирует REQ» по образцу растовых {#b-033}

| | |
|---|---|
| ##B033-ANCHOR **anchor** | `##RULE-SEAM-ERROR-CONTRACT` в `conform-frontend-go.md` (F-185); растовые образцы — правила `error-enum-cites-req` / `error-message-cites-req` (`rules/diagnostics.rs:314`, `:235`) |
| ##B033-LOCATOR **locator** | детекция уже есть: вид находки `seam_error_missing_req` внутри правила `go-unsafe-in-domain` (`rules/go.rs:146-149`), эмитится экстрактором (`go-extract/extract.go:534`); выделенного правила с собственным id — нет; проверки, что `Error()` реально рендерит REQ (message-половина), — нет ни в каком виде |
| ##B033-SEVERITY **severity** | P2 |
| ##B033-DISPOSITION **disposition** | `done` — **построено волной Б батч 2, 2026-08-04**: `go-seam-error-cites-req` (одно Go-правило, обе половины, per-half отпечатки; message-половина читает тела `Error()`, маркер `spec://`/`violates REQ`, якорь на строке метода) + TS-близнец `ts-seam-error-cites-req` (Form-1 union, пределы записаны честно) — оба смонтированы и показаны на фикстурах; F-185 пересужен `confirmed`. Исходно: решение владельца 2026-08-02 по предъявлению F-185; рамка семьи B-033/B-034/B-035: «По сути мы не можем писать на Typescript и Go пока не поправим вот это» |
| ##B033-FILED **filed by** | рулинг предъявления F-185, 2026-08-02 |

- ##B033-SUT **Суть, по-простому.** Документация Go-гейта обещает именованное правило «тип ошибки на границе модуля цитирует требование спецификации» — правила с таким именем нет; сама проверка живёт видом находки внутри другого правила и ловит только половину контракта (наличие поля `Spec` в типе ошибки), а вторую половину — что метод `Error()` реально печатает `violates REQ …` — не проверяет никто. Стройка: продвинуть вид находки в отдельное правило со своим id + достроить message-половину.
- ##B033-ANSWER **Ответ на вопрос владельца («имеет ли смысл, или особенность языка?»).** Имеет смысл; особенность языка — только в детекторе, не в архитектуре. Rust проверяет замкнутые enum'ы ошибок (два правила), Go-идиома — struct с полями `Code`/`Spec`/`Err` на шов (обе фикстуры пакета это уже доказывают: dirty-фикстура нарочно без `Spec`, clean-фикстура рендерит `violates REQ %s`). Ростер правил Go-фронтенда — те же нейтральные `Rule` с id/why/check, что у Rust; правило ложится в готовую форму. Что даёт отдельный id: собственная строка в SARIF, отдельная гранулярность baseline-«трещотки», per-rule включение/выключение, и — главное — документация перестаёт врать.
- ##B033-TS-TWIN **TS-близнец — решить при постройке.** У TypeScript проверки нет вовсе (ни правилом, ни видом находки — см. B-035, первый проход), при том что TS-гайд предписывает «the E union cites spec:// REQs». Строить близнеца сразу или после B-035 — первый шаг реализации.

### B-034 — инвариант «каждая единица кода под гейтом или исключена» для Go и TypeScript {#b-034}

| | |
|---|---|
| ##B034-ANCHOR **anchor** | `##EVERY-PACKAGE-GATED-OR-EXEMPT` в `conform-frontend-go.md` (F-185); реализация-образец — `Config::validate_against_tree` (`core-ai-native-conform/src/config.rs:259-266`) |
| ##B034-LOCATOR **locator** | инвариант реален и оттестирован, но crate-овый: читает `gated_crates`/`[[exempt]]`/`roots` корневой таблицы; вызывают его только `rust-ai-native-conform` (`lib.rs:119`, `:188`) и его MCP-близнец; Go- и TS-фронтенды не вызывают никогда |
| ##B034-SEVERITY **severity** | P2 |
| ##B034-DISPOSITION **disposition** | `planned` — **решение владельца 2026-08-02: «Похоже на задачу — нужно реализовать эту функциональность в Typescript и Go»**; рамка семьи — как у B-033; **единица гейта и дом списков решены развилкой №2 2026-08-04** (Go = package, TS = cell, симметричные секции — см. `##B029-DISPOSITION` и `spec/design/gate-parity-config.md` §2) |
| ##B034-FILED **filed by** | рулинг предъявления F-185, 2026-08-02 |

- ##B034-SUT **Суть, по-простому.** Проверка «каждый модуль либо под контролем гейта, либо явно исключён с причиной — ничего не забыто молча» существует и работает только для Rust-крейтов. Go-документация обещала её «на каждом прогоне» — по факту для Go и TS она не бежит никогда: их код попадает под контроль только через baseline-«трещотку», а классификацию дерева никто не сверяет. Стройка: пер-языковый инвариант — Go-единица (package), TS-единица (cell) — плюс пер-языковые гейт-списки в конфиге.
- ##B034-DESIGN **Решить в первый шаг стройки (вместе с B-029).** Единица гейта пер-язык (crate / package / cell); где живут списки — корневая таблица с пер-языковыми алиасами или секции `[go]`/`[typescript]`; и как инвариант включается у языка, где сегодня `roots = []` по умолчанию (пустое дерево — нечего классифицировать: инвариант должен не давать ложную зелень на пустом скоупе). Развязка конфиг-поверхности — `##B029-CONFIG-SURFACE`.

### B-035 — паритет-аудит языковых стеков: TS и Go не слабее Rust, или причина записана {#b-035}

| | |
|---|---|
| ##B035-ANCHOR **anchor** | принцип владельца, 2026-08-02 (дословно): «Мы по условию идеи не должны делать поддержку других языков (в особенности Typescript) хуже, чем это сделано для Rust. А по факту мы почему-то начинаем ослаблять правила без видимой причины» |
| ##B035-LOCATOR **locator** | ростеры трёх фронтендов (`build_rules` в `rust-`/`typescript-`/`go-ai-native-conform`), их census-виды, инварианты конфига, флор-шаги, и обещания гайдов против фактических проверок гейтов |
| ##B035-SEVERITY **severity** | P2 |
| ##B035-DISPOSITION **disposition** | `planned` — **решение владельца 2026-08-02: «надо проверить, нет ли подобных же проблем в реализации Typescript»**; рамка семьи — как у B-033 |
| ##B035-FILED **filed by** | рулинг предъявления F-185, 2026-08-02 |

- ##B035-SUT **Суть, по-простому.** Систематически сравнить, что проверяет гейт у каждого языка, против того, что проверяет Rust и что обещают гайды, — и каждую найденную слабину либо достроить, либо записать причиной («язык не имеет аналога идиомы»), но не оставлять молчаливой. Метод — по образцу исследования B-012: таблица «механизм → Rust / TS / Go → есть/нет/частично → вердикт строить/записать».
- ##B035-FIRST-PASS **Первый проход (2026-08-02, эта сессия), чтобы аудит не начинался с нуля.** (1) **REQ-цитирование ошибок:** Rust — два правила; Go — structure-половина видом находки (B-033); **TS — ничего**: пять census-видов `TsUnsafeInDomain` (`any_type`, `as_cross`, `non_null`, `ts_ignore`, `ts_expect_error`) — все про unsafe-набор, про ошибки швов ни одного, при обещании гайда «the E union cites spec:// REQs». (2) **Инвариант gated-or-exempt:** только Rust (B-034). (3) **Документация:** Go-доки обещали несуществующее (F-185); TS-доки честны — `conform-frontend-typescript.md` не обещает ни гейт-списков, ни seam-error-правила (проверено грепом), то есть слабина TS — в движке, не в доке. Ростеры симметричны по форме (по 3 правила: unsafe-in-domain, cell-isolation, file-length) — расхождение в начинке видов.
- ##B035-NORM **Шаг стройки: поднять принцип в спеку.** «Не слабее Rust без записанной причины» сегодня живёт только этой записью; при первой стройке семьи поднять его в дисциплину owner-approved диффом, чтобы правило пережило бэклог. **Дом решён владельцем 2026-08-04 (развилка №9 карты): «Ядро дисциплины» — языко-нейтральный guiding-слой core-ai-native (манифест-уровень), один дом, стеки цитируют.** Подъём — босс-авторский контрактный дифф, едет батчем 2. Смежность: B-030 — тот же вопрос для одной конкретной идиомы (ассерции соответствия); этот аудит — его обобщение; B-023 — синтаксический tier, отдельная ось.
- ##B035-LOOP-1 **Луп-проход №1 (после батча 1 волны Б, 2026-08-04):** [`harvest/e10-b035-parity-pass.md`](campaigns/packages-2026-09/harvest/e10-b035-parity-pass.md) — 13 строк; инфраструктурные асимметрии закрыты (инвариант/vacuous/scope/счётчики — у всех трёх); контентный долг — REQ-цитирование швов (B-033) и Go-правило флагов; две свежие находки — Rust-floor без floor_disable (рулинг: строить, B-049) и нефильтрованный остаток Go-floor'а (vet/tests/staticcheck — проверка рутиной батча 2).
- ##B035-LOOP-2 **Луп-проход №2 (после батча 2 волны Б, 2026-08-04):** [`harvest/e12-b035-parity-pass.md`](campaigns/packages-2026-09/harvest/e12-b035-parity-pass.md) — строки 1 (REQ-цитирование швов, обе половины ×3) и 13 (floor-disable ×3, B-049 закрыл инверсию) **паритет достигнут**; строка 7 (conformance — Go построен gated, Rust причина-компилятор, TS маршрут type-level-tests). Принцип паритета поднят в манифест (`##PARITY-ACROSS-PROJECTIONS`), три гайда цитируют. Остаётся открытым (записано, маршрутизировано, не молча): строка 6 (Go flag/registry-правило) и строки 8/12 (Go-floor `./...`-остаток, B-048-близнец) — оба прямые стройки поздних батчей. **M-PARITY: recorded-honest, но не build-complete** (нужны строка 6 + 8/12).

### B-036 — conform-правило «инварианты не тонут в середине файла» {#b-036}

| | |
|---|---|
| ##B036-ANCHOR **anchor** | `GUIDE-AI-NATIVE-RUST.md` `##POSITION-IS-A-RESOURCE` (:59) и его TS-близнец (:128) — оба обещают проверку, оба теперь честно drift (F-154 + верди́кт-сначала D30) |
| ##B036-LOCATOR **locator** | нигде не построено: единственные «middle third» в движке — док-комментарий и текст сообщения правила длины (`core-ai-native-conform/src/rules/budget.rs:119`, `:147`); позицию комментариев не смотрит ни один фронтенд |
| ##B036-SEVERITY **severity** | P2 |
| ##B036-DISPOSITION **disposition** | `planned` — **решение владельца 2026-08-02: механизмы дисциплины строятся, не аннотируются в отказ** («Если для их работы нужно что-то построить — нужно это спроектировать и потом построить, а не отказываться просто потому что так проще») |
| ##B036-FILED **filed by** | рулинг предъявлений F-154/F-161, 2026-08-02 |

- ##B036-SUT **Суть, по-простому.** Гайды учат: критичные инварианты — в начале или конце файла, не в «разбавленной середине» (модели внимания хуже читают середину контекста), и обещают проверку: предупреждать, когда комментарий-инвариант оказался в средней трети файла. Проверка длины файла есть во всех языках; проверки позиции нет ни в одном. Стройка: правило движка (алгоритмическое: извлечь комментарии с маркерами инвариантности, вычислить положение в файле, сравнить с границами третей), конфигурируемые пороги, предупреждение — не блокирующий гейт на старте (урок B-021).
- ##B036-DESIGN **Решить перед кодом.** Что считается «комментарием-инвариантом» (эвристика по маркерам: `SAFETY:`, `INVARIANT:`, `##ANCHOR`-строки, doc-комменты с MUST/NEVER?) — словарь в конфиг; и общий движок против пер-языковых экстракторов (комментарии уже извлекаются фронтендами — проверить, доносят ли позиции).

### B-037 — слой кастомных REQ-цитирующих линтов: dylint для Rust, typescript-eslint для TS {#b-037}

| | |
|---|---|
| ##B037-ANCHOR **anchor** | `GUIDE-AI-NATIVE-RUST.md` `##SCAFFOLD-F-STRUCTURED-DIAGNOSTICS` (:72, клауза «custom clippy lints name the rule and the remedy») и TS-близнец (:141, «Custom `@typescript-eslint` rules …») — F-154 + D30 |
| ##B037-LOCATOR **locator** | ни `dylint`/`declare_lint`, ни `createRule`/`ESLintUtils` не встречаются нигде в дереве; clippy-шаг панели стоковый; eslint-конфиг демо — стоковый recommended |
| ##B037-SEVERITY **severity** | P2 |
| ##B037-DISPOSITION **disposition** | `planned` — решение владельца 2026-08-02 (та же формула, что B-036; «Там могут быть правки компилятора, изобретение новых инструментов, да что угодно») |
| ##B037-FILED **filed by** | рулинг предъявлений F-154/F-161, 2026-08-02 |

- ##B037-SUT **Суть, по-простому.** Два из трёх обещанных каналов структурной диагностики построены (сообщения ошибок цитируют требования; отчёты в SARIF). Третий — собственные линты, чьи сообщения называют правило и лекарство, — не построен ни для одного языка. Стройка: Rust — dylint-класс библиотека линтов (свой крейт, свои правила поверх clippy-инфраструктуры); TS — пакет правил `@typescript-eslint` с REQ-контекстом. Оба выводят находки в той же грамматике «violates REQ …; fix surface: …».
- ##B037-RELATED **Смежность.** B-026 (SARIF-ингест) — встречная половина: чужие линтеры читаем, свои — пишем; вместе они замыкают контур. Пер-языковый паритет — B-035.

### B-038 — pending-карточки правил обретают карточки и чекеры: R-060 и closed-vocabulary-naming {#b-038}

| | |
|---|---|
| ##B038-ANCHOR **anchor** | `##DECLARED-TEST-MATRICES-NEVER-EXPONENTIAL` (:127, «R-060, retained» — id без карточки и чекера) и `##NAMES-ARE-TOKEN-PROGRAMS` (:57, `{Variant}{Seam}` — computed-имя без линта; карточка `rule-closed-vocabulary-naming`/R3-004 «candidate future card» во всех стеках) — F-154 |
| ##B038-LOCATOR **locator** | R-060: две цитаты в гайдах + «Phase 4+ runtime consumer» в `crates/vibe-cli/src/registry.rs:63`; карточек нет, ATLAS-записей нет, чекеров нет; семь pending-карточек перечислены в `core-ai-native/…/01-PATTERN-CARD-FORMAT.md:7` |
| ##B038-SEVERITY **severity** | P2 |
| ##B038-DISPOSITION **disposition** | `planned` — решение владельца 2026-08-02 (формула B-036) |
| ##B038-FILED **filed by** | рулинг предъявлений F-154, 2026-08-02 |

- ##B038-SUT **Суть, по-простому.** Гайды цитируют правила по id, за которыми не существует ни карточки (описания правила в реестре карточек), ни проверки. Стройка, двумя первыми: **R-060** — «тест-матрицы объявляются данными, никогда полный перебор 2^n»: карточка + чекер (алгоритмический: найти комбинаторные тест-циклы/макросы, сверить с объявленной матрицей); **rule-closed-vocabulary-naming (R3-004)** — карточка + линт именования из закрытого словаря токенов. Остальные пять pending-карточек — тем же конвейером после первых двух.
- ##B038-DESIGN **Дизайн-вопрос внутри — РЕШЁН ВЛАДЕЛЬЦЕМ 2026-08-04 (развилка №1 карты): computed-имена.** Вопрос стоял так: принимает ли Rust вычисляемые имена ячеек `{Variant}{Seam}` (как Go, где это живая практика), или растовые ячейки именуются свободно и линт проверяет только словарь и уникальность. Решение — **вычисляемые**: имя = `Pascal(variant)` + шов как написан, одно правило движка обслуживает Rust И Go (Go практикует конвенцию без единой машинной проверки — стройка закрывает и его пробел), TS записывает причину (манифеста ячейки нет, вычислять не из чего). Текст гайда `:57` остаётся и перестаёт быть целью. **Замер цены по ВСЕМУ дереву (цензус `harvest/e13-r3-pending-cards-census.md` мерил только пакеты дисциплины и `rust-demo` — хост был вне его периметра, и цифра «~10» оказалась занижена):** 40 ячеек с манифестом, **14 уже соответствуют** (всё семейство `vibe-check`), **13 переименований в продакшн-коде хоста** (`vibe-resolver` ×5, `vibe-mcp` ×4, `vibe-registry` ×2, `vibe-index` ×2), остальное — тестовые фикстуры и регенерируемые `.vibe/cache/**`. Имена нигде не уходят в протокол (MCP-имена — отдельные строковые литералы), так что каждое переименование проверяется компилятором. Правило приземляется с замороженным ratchet-baseline; переименования — отдельным коммитом.

### B-039 — смонтировать R-001 (FlagSites) на TypeScript-гейт; обследовать Go {#b-039}

| | |
|---|---|
| ##B039-ANCHOR **anchor** | `GUIDE-AI-NATIVE-TYPESCRIPT.md` `##NO-IF-FLAG-IN-DOMAIN-CELLS` (:167) + `##RULE-FLAGS-READ-AT-THE-ROOT-AND-DISPATCHED` (:177) — F-161 |
| ##B039-LOCATOR **locator** | правило `FlagSites` (id `R-001`) живёт в общем движке (`rules/structure.rs:33-35`), Rust монтирует его через ветку конфига (`rust-ai-native-conform/src/lib.rs:55-63`, `registry_file` + `registry_gated_crate`); TS-`build_rules` ветки не имеет вовсе — правило непримонтируемо |
| ##B039-SEVERITY **severity** | P2 |
| ##B039-DISPOSITION **disposition** | `planned` — **решение владельца 2026-08-02: «Я против чтобы ты выключал правила только потому, что они нигде пока не используются»** — правило не ослабляется, гейт дорастает |
| ##B039-FILED **filed by** | рулинг предъявлений F-161, 2026-08-02 |

- ##B039-SUT **Суть, по-простому.** Учебник TS называет силой гейта правило «никаких `if (flag)` в доменных ячейках» — а TS-гейт его не запускает и не может запустить: нет ветки конфигурации, которая есть у Rust. Стройка: добавить TS-гейту конфиг-ветку (аналог `registry_file`/`registry_gated_crate` — где у TS живёт реестр флагов, решить при проектировании), смонтировать `FlagSites`, и обследовать Go на ту же дыру. Клауза «deviates + reason» из :177 оживает вместе с правилом.
- ##B039-RELATED **Смежность.** Прямой близнец семьи паритета: B-034 (инвариант gated-or-exempt), B-029 (поверхность конфига), B-035 (аудит). Вести совместно.

### B-040 — рефакторинг-обзор собственных швов: полный scaffold-B на нашем коде {#b-040}

| | |
|---|---|
| ##B040-ANCHOR **anchor** | `GUIDE-AI-NATIVE-RUST.md` `##SCAFFOLD-B-TYPED-BUILDERS` (:68) — F-154 |
| ##B040-LOCATOR **locator** | практикуются newtypes, typestate-швы и `#[must_use]` (11 употреблений в одном `vibe-actions/src/action.rs`); sealed traits и `PhantomData`-строители — ноль употреблений во всех Rust-проектах дерева на HEAD |
| ##B040-SEVERITY **severity** | P2 |
| ##B040-DISPOSITION **disposition** | `planned` — **решение владельца 2026-08-02: «Если у нас на нашем же коде не выполняется пять каких-то важных дисциплин — это похоже на причину по которой нужно всё отрефакторить и начать их применять»** |
| ##B040-FILED **filed by** | рулинг предъявлений F-154, 2026-08-02 |

- ##B040-SUT **Суть, по-простому.** Учебник учит делать неправильный вызов непредставимым: typestate, newtypes, строители с обязательными полями в типах, запечатанные трейты (sealed traits — чтобы чужой код не мог подложить свою реализацию контрактного трейта), `PhantomData`-параметры состояния. Половина идиом у нас в ходу, вторая — нет вообще. Работа: обзор швов vibevm (публичные трейты крейтов, строители, протокольные поверхности) на «где sealed trait / typestate-строитель окупается», применить где окупается, задокументировать где сознательно нет. Не механическая рассыпка идиом по коду — точечный рефакторинг по месту.
- ##B040-VERIFY **Верификация.** Дифференциальный оракул на затронутые ячейки (правило замены нетривиальной ячейки — гайд §D); панель зелёная после каждого шва; итог — пере-суд `:68` по факту практики.
- ##B040-CENSUS **Обзор-цензус снят (2026-08-04, волна Г попутно):** [`harvest/g1-b040-seams-census.md`](campaigns/packages-2026-09/harvest/g1-b040-seams-census.md) — 24 pub-трейта (ни один не sealed; `Watcher` вовсе без прод-реализации), один рантайм-валидирующий строитель (`ActionBuilder`), typestate/PhantomData — 0 (локатор подтверждён точно), `#[must_use]` 146 (82 % — TUI-виджеты vibe-cli), зрелые newtypes на identity-шве vibe-core с измеренной асимметрией (только `Group` валидирует на load; `content_hash` в progress-core — голый String при валидированном близнеце в vibe-core; URL — везде голый String). Точечный рефакторинг — босс-дизайном по этому цензусу, отдельным заходом.

### B-041 — карта развития инструментария: от реестра дыр к системе {#b-041}

| | |
|---|---|
| ##B041-ANCHOR **anchor** | директива владельца 2026-08-02 (дословно): «Мне нужно понимание, как развивать вообще наш инструментарий, чтобы оно стало хорошей системой. Система не заморожена, она должна развиваться» + «Построение ai-native дисциплин сложная штука, ее нужно делать, а не отказываться. Там могут быть правки компилятора, изобретение новых инструментов, да что угодно» |
| ##B041-LOCATOR **locator** | сырьё уже собрано кампанией: аннотации «Specified, not built» по корпусу, исследование B-012 (+ его #rulings), стройки B-016…B-021, B-025/B-026, B-033…B-040, паритет-аудит B-035, реестр обязательств |
| ##B041-SEVERITY **severity** | P2 |
| ##B041-DISPOSITION **disposition** | `planned`, высокий приоритет — прямой запрос владельца |
| ##B041-FILED **filed by** | рулинг предъявлений F-154/F-161, 2026-08-02 |

- ##B041-SUT **Суть, по-простому.** Собрать из накопленного не список дыр, а карту развития: какие механизмы дисциплины существуют / обещаны / строятся, в каком порядке их строить (зависимости: конфиг → правила → линты → карта → агентские инструменты → подпись), что из этого меняет движок, что — стеки, что — хост, и где вехи «система стала хорошей». Жанр — design-документ (lore, не контракт), дом — `spec/design/`; нормативные следствия — отдельными owner-диффами.
- ##B041-METHOD **Метод.** По образцу B-012: свод таблицей «механизм → слой (SPEC/ENGINE/DRIVER/DEPLOYMENT) → состояние → запись стройки → зависимости», сверху — порядок волн и развилки, требующие слова владельца. Черновик — боссом (архитектурная работа, не делегируется).
- ##B041-DRAFT **Черновик написан, одобрен и интегрирован** (2026-08-02, один ситтинг): карта живёт в корне рядом с бэклогом — [`TOOLING-MAP.md`](TOOLING-MAP.md) (переезд из `spec/design/` по слову владельца «интегрировать в бэклог — или положить рядом…»; корневой `ROADMAP.md` — продуктовые вехи, другой документ, не пересекаются) + раздел-указатель [`#map`](#map) в шапке этого файла. Владелец: «мне нравится этот документ», с рамкой — **действовать внутри идущего рефакторинга, недостающее откладывать**; волны исполняются фазами кампании (E и далее), не параллельным процессом. Развилки §5 карты ждут его слова по мере подхода волн.

### B-044 — no-zombie тест: процесс-таблица подтверждает смерть ребёнка оракула, во всех трёх стеках {#b-044}

| | |
|---|---|
| ##B044-ANCHOR **anchor** | шесть копий клятвы «no-zombie property is test-asserted» по трём стекам (семья F-281): rust `TCG-ORACLE-RUST-v0.1.md#GRACEFUL-EXIT…` (аннотирован D5/F-192) + `vibe-agentic-tcg-rust.md#RISK-WINDOWS…`; go `TCG-ORACLE-GO-v0.1.md#GRACEFUL-EXIT-IS-THE-LSP-DANCE` + `vibe-agentic-tcg-go.md#RISK-WINDOWS…`; ts `TCG-ORACLE-v0.1.md#RUST-SIDE-OWNS-TERMINATION` + `vibe-agentic-tcg-ts.md#RISK-WINDOWS…` |
| ##B044-LOCATOR **locator** | механика есть во всех трёх (shutdown-танец + kill-on-drop; ts-транспорт даже реапит через `try_wait`), **тест-ассерция — нигде**: единственная настоящая process-table-ассерция в дереве — у fractality (`fractality-pod/tests/loopback.rs:288-299`, sysinfo-проба с дедлайном) — и она про чужого ребёнка (pod/worker), не про оракул |
| ##B044-SEVERITY **severity** | P2 |
| ##B044-DISPOSITION **disposition** | `planned` — **решение владельца 2026-08-02: «тест на зомби лучше написать»** |
| ##B044-FILED **filed by** | рулинг семьи F-281, 2026-08-02 |

- ##B044-SUT **Суть, по-простому.** Все три оракула обещают: убили обёртку — дочерний языковой сервер (rust-analyzer / gopls / node) не выживает. Механика убийства построена, а теста, который бы **спросил у операционной системы**, что ребёнок действительно умер, нет ни у одного стека — клятва «проверено тестом» держится на споуке Phase-0. Стройка: по тесту на стек — поднять оракул, узнать PID ребёнка, убить/уронить обёртку, опросить процесс-таблицу с дедлайном (паттерн уже доказан в дереве — fractality-проба на `sysinfo`, копировать её форму).
- ##B044-DESIGN **Решить при постройке.** (1) Доступ к PID ребёнка из теста — сегодня он инкапсулирован в транспорте; либо тестовый акцессор, либо перенос ассерции в существующие live-тесты (`live_oracle.rs` всех трёх стеков — им уже доступен спавн). (2) Windows-специфика опроса (sysinfo уже кроссплатформен у fractality-пробы). (3) Гейтинг: live-тесты капабилити-гейтятся отсутствием тулчейна (gopls/node) — ассерция едет в те же тесты и наследует их гейт. Транспорт исполнения — фазы кампании (T — тестовая фаза — естественный дом; см. рамку карты).
- ##B044-CLOSES **Что закрывает.** Шесть якорей семьи: пять drift (два исходных + три пере-суженных вердикт-сначала батчем D32) отложены на эту запись; аннотированный rust-близнец (F-192) пере-судится по факту постройки. Тексты стоят целями — по build-first ни один не смягчается.

### B-045 — qualified-naming: хост доводит грамматику — kind-валидация, короткие имена у четырёх глаголов, четыре мис-цитаты {#b-045}

| | |
|---|---|
| ##B045-ANCHOR **anchor** | `ref-grammar.md` `##THE-RESOLVER-CHECKS-THE-TYPE-AND-ERRORS-ON-A-MISMATCH` + `##ROW-FORM-KIND-AND-NAME` (валидационная колонка) + `##RULE-THE-CLI-ACCEPTS-ALL-FORMS` (F-178); PROP-008 `##KIND-VALIDATION` |
| ##B045-LOCATOR **locator** | kind парсится и переносится (`package_ref/tests.rs:117-124`, `short_name.rs:135` копирует), сверки с манифестом нет нигде; типа `KindMismatch` не существует (три хита — док-коммент `package_ref.rs:428`, design-запись, спека), exit-код `TYPE_MISMATCH = 4` зарезервирован и мёртв; `require_group` отбивает короткие имена у `uninstall` / `update` / трёх `registry redirect`, хотя lockfile-first резолвер (`short_name.rs:76-79`) уже умеет отвечать без сети; четыре call-site'а цитируют PROP-008 §2.4, который короткого запрета не несёт (его таблица шлёт short в §2.6) |
| ##B045-SEVERITY **severity** | P2 |
| ##B045-DISPOSITION **disposition** | `planned` — **решение владельца 2026-08-02 по предъявлению F-178: «(1) записать B-045 + применить однострочный фикс»** |
| ##B045-FILED **filed by** | рулинг предъявления F-178, 2026-08-02 |

- ##B045-SUT **Суть, по-простому.** Три доводки хостовой стороны грамматики имён. **(а) Kind-валидация:** префикс вида `flow:` в ссылке на пакет сегодня парсится и едет дальше, но никогда не сверяется с тем, чем пакет оказался на самом деле — обещанная ошибка `KindMismatch` не существует как тип, а док-комментарий кода утверждает, что сверка происходит. Построить: сверка после резолюции, тип ошибки, посадка на зарезервированный exit-код 4. **(б) Короткие имена у четырёх глаголов:** `vibe uninstall wal` и `vibe update wal` сегодня отбиваются «пиши `<group>/<name>`», хотя обе команды работают по уже установленному и lockfile-first резолвер отвечает из lockfile без индекса и сети — не хватает нескольких строк подключения (redirect-глаголы — решить при постройке, им short редко нужен). **(в) Четыре мис-цитаты:** `uninstall.rs:152`, `update.rs:444`, `redirect/mod.rs:124` + комментарий `uninstall.rs:36` оправдываются разделом §2.4, который этого не говорит; честная опора — §2.6 `##INDEX-DEPENDENCY` (и она перестанет быть нужна после (б)).
- ##B045-RELATED **Смежность.** B-031 (корень-как-пакет) — не пересекается; транспорт — фаза E по рамке карты. Слои: SPEC в пакете (правило здорово), ENGINE/DRIVER — `crates/vibe-core` + `crates/vibe-cli`, DEPLOYMENT — `vibe.lock` (правило хранения там держится без исключений).

### B-046 — мультиязычная композиция: агент собирает несколько AI-Native языков в одном проекте {#b-046}

| | |
|---|---|
| ##B046-ANCHOR **anchor** | директива владельца 2026-08-02 (по истории OracleRegistry, дословно): «должен быть понятный способ которым AI агент должен собрать процесс использования нескольких языков в одном проекте одновременно. Может это отдельный MCP+CLI, может еще что-то… Сделать общий реестр может быть стоит, но этот реестр должен работать на основе autodiscovery подключенных AI-Native языков, не нарушая их автономность когда они установлены по-отдельности» |
| ##B046-LOCATOR **locator** | сегодня агент мультиязычного проекта подключает N серверов руками (по одному на язык); суверенитет держится (MCP-SOVEREIGNTY, PROP-027), композиционного слоя нет; рельсы autodiscovery уже существуют — lockfile знает установленные пакеты, `[[mcp_server]]`-таблицы объявляют серверы (PROP-027), `[[binary]]` — CLI-бинари (PROP-025), `vibe mcp` их уже читает |
| ##B046-SEVERITY **severity** | P2 |
| ##B046-DISPOSITION **disposition** | `planned` — **«Такое стоит сразу класть в бэклог»** (владелец, 2026-08-02) |
| ##B046-FILED **filed by** | рулинг истории OracleRegistry/F-210, 2026-08-02 |

- ##B046-SUT **Суть, по-простому.** Суверенитет языков сохраняется (каждый стек автономен, установлен по-отдельности — работает сам по себе), но над ним появляется понятный способ собрать мультиязычный проект: агент одним жестом узнаёт, какие AI-Native языки подключены, и получает их поверхности. Это НЕ возврат удалённого OracleRegistry (тот был прибит к одной топологии в хосте) — это композиция поверх суверенных.
- ##B046-OPTIONS **Варианты (решить при проектировании, владельцу):** **(1)** отдельный тонкий MCP+CLI-агрегатор («ai-native-workspace»): autodiscovery по lockfile → релей к пер-языковым серверам/бинарям; одна точка подключения для агента, ноль собственной логики; **(2)** конвенция discovery-манифеста без нового сервера: `vibe` отдаёт агенту ростер подключённых стеков и их поверхностей (расширение `vibe mcp` / B-018-инструментов), агент подключает языки сам; **(3)** гибрид: (2) как основа + (1) как опциональная обёртка для хостов, где число серверов ограничено. Заготовка рекомендации: начать с (2) — рельсы готовы (lockfile + `[[mcp_server]]` + `[[binary]]`), автономность не тронута по построению; (1) добавлять по реальной боли одного-подключения.
- ##B046-AUTONOMY **Закон автономности (владелец, дословно в anchor):** реестр/агрегатор работает ТОЛЬКО autodiscovery-путём по установленному; стек, поставленный в одиночку, не знает об агрегаторе и не зависит от него; отсутствие агрегатора ничего не ломает.
- ##B046-RELATED **Смежность.** B-018 (агентские инструменты vibe — ростер-половина варианта (2) ложится туда естественно); B-047 (норма поверхностей — агрегатор обязан быть тонкой поверхностью над разделяемой логикой); PROP-026-грамматика (единая грамматика инструментов — то, что делает композицию дешёвой).

### B-047 — норма поверхностей: логика в разделяемом крейте, CLI и MCP — тонкие поверхности над ней {#b-047}

| | |
|---|---|
| ##B047-ANCHOR **anchor** | критика владельца 2026-08-02 (дословно): «Нужен какой-то код, доступный из разных поверхностей. MCP — одна поверхность, инструменты командной строки — другая. У нас постоянно в коде недостаточный уровень абстракции, всё прибивается гвоздями к конкретной реализации… логика, общая между MCP и CLI должна быть сформулирована абстрактно в какой-то библиотеке или крейте, чтобы ее переиспользовали разные поверхности» |
| ##B047-LOCATOR **locator** | стеки норму уже держат: логика в bridge/engine-крейтах (`rust-ai-native-tcg-bridge` и близнецы, conform/specmap-движки), CLI-бинари — первая поверхность, MCP-серверы — вторая (описания инструментов буквально «= `rust-ai-native init`»); проверить и довести хостовую сторону: vibe-mcp (четыре продуктовых тула) против CLI-паритета, B-018-инструменты — с рождения двумя поверхностями |
| ##B047-SEVERITY **severity** | P2 |
| ##B047-DISPOSITION **disposition** | `planned` — решение владельца 2026-08-02 (та же директива, что B-046) |
| ##B047-FILED **filed by** | рулинг истории OracleRegistry/F-210, 2026-08-02 |

- ##B047-SUT **Суть, по-простому.** Стоячая норма: пользовательская способность живёт в разделяемой библиотеке; CLI и MCP — тонкие поверхности над ней, ни одна не является «основой»; новая способность рождается минимум с двумя поверхностями или с записанной причиной, почему одной хватит. Работа записи: (1) аудит «где прибито гвоздями» — обход поверхностей хоста и стеков с таблицей «способность → логика-крейт → CLI → MCP → дыра»; (2) доводка найденных дыр (первый известный кандидат: карт-инструменты vibe — CLI-половина есть, MCP-половина — B-018); (3) поднять норму в спеку дисциплины owner-approved диффом (дом — решить при аудите; кандидат — ENGINE-CONFORM/GUIDE-семья рядом с четырёхслойной моделью SPEC/ENGINE/DRIVER/DEPLOYMENT, чьим уточнением норма и является: DRIVER — это не один бинарь, а набор тонких поверхностей над ENGINE).
- ##B047-RELATED **Смежность.** B-018 (первый потребитель нормы), B-046 (агрегатор обязан ей подчиняться), B-035 (паритет-аудит — та же таблично-обходная механика).

### B-048 — TS-floor: prettier/eslint-шаги обходят fixtures пакета (двойник B-003) {#b-048}

| | |
|---|---|
| ##B048-ANCHOR **anchor** | none — найдено цензусом E8-R3 (`campaigns/packages-2026-09/harvest/e8-r3-go-floor-census.md`, Q5), не против размеченного факта |
| ##B048-LOCATOR **locator** | `typescript-ai-native-cli/src/floor.rs:78-97` — `prettier --check .`; `:141-160` — `eslint .`; оба обходят `.` без скоупа на policy-roots; в TS-пакете живут настоящие деревья фикстур (`tools/ts-extract/test/fixtures/{clean,dirty}`, `tools/ts-oracle/test/fixtures/proj`), `.prettierignore` в пакете отсутствует |
| ##B048-SEVERITY **severity** | P2 |
| ##B048-DISPOSITION **disposition** | `open` |
| ##B048-FILED **filed by** | цензус E8-R3 волны Б, 2026-08-04 |

- ##B048-SUT **Суть, по-простому.** Тот же класс дыры, что B-003 у Go, на двух шагах TS-floor'а: prettier и eslint ходят по всему дереву пакета и в чужом checkout'е упрутся в нарочно сломанные фикстуры экстрактора/оракула. Conform-половины дыры у TS НЕТ (default `exclude_substrings = ["/fixtures/"]` уже стоит); tests-шаг уже заскоуплен на roots (`floor.rs:121-138` — урок demo-walk). Стройка: заскоупить/отфильтровать prettier- и eslint-шаги тем же приёмом, что B-003 у gofmt (пост-фильтр вывода конфигом либо скоуп на roots — решить при постройке единообразно с посаженным B-003).
- ##B048-RELATED **Смежность.** B-003 (прямой образец фикса, Go); B-035 (паритет-строка: floor-шаги «не слабее Rust» — у Rust дыры нет, fmt cargo-скоуплен); попадает в первый же луп B-035 после батча 1.
- ##B048-GO-RESIDUAL **Go-остаток той же семьи (проверен боссом 2026-08-04, луп-строка 12):** после B-003 нефильтрованными у Go-floor'а остались `vet`/`tests`/`staticcheck` (`./...`) — остаток **латентный**: на пакетном корне без `go.mod` эти шаги падают на резолюции модуля раньше, чем дойдут до фикстур (записанные «не-дефекты» строки B-003); укусить может только module-rooted потребитель с фикстурными деревьями. Единообразное лечение — тем же приёмом, каким ляжет фикс этой строки; вести вместе.

### B-049 — Rust-floor обретает floor_disable (близнец Go/TS-механизма) {#b-049}

| | |
|---|---|
| ##B049-ANCHOR **anchor** | паритет-таблица лупа B-035, строка 13 (`harvest/e10-b035-parity-pass.md`) — единственная асимметрия «не в ту сторону» |
| ##B049-LOCATOR **locator** | `rust-ai-native-cli/src/floor.rs:46-141` бежит все шаги безусловно; у `RustConfig` нет поля `floor_disable`; у Go/TS механизм есть и работает (`GoConfig`/`TsConfig.floor_disable` + enforcement в их floor'ах: печать каждого отключения, hard-fail неизвестного шага) |
| ##B049-SEVERITY **severity** | P2 |
| ##B049-DISPOSITION **disposition** | `done` — **построено волной Б батч 2, 2026-08-04**: `RustConfig.floor_disable: Vec<FloorDisable>` + enforcement в `rust-ai-native-cli/src/floor.rs` (`STEPS`-словарь реальных шагов, печать каждого отключения с причиной, hard-fail неизвестного шага) — текстовое зеркало Go/TS-близнецов; паритет-инверсия лупа закрыта (строка 13). Исходно: решение владельца 2026-08-04 «Строить близнеца» |
| ##B049-FILED **filed by** | луп-проход №1 B-035, 2026-08-04 |

- ##B049-SUT **Суть, по-простому.** Потребитель Go/TS-стека может отключить шаг floor'а с записанной причиной (и floor громко печатает каждое отключение); потребитель Rust-стека — не может никак. Принцип паритета симметричен: и Rust не слабее прочих. Стройка: `[rust] floor_disable` в конфиге движка (форма секций уже единая) + enforcement в Rust-floor'е той же механикой, что у близнецов.

### B-053 — текст причины отступления не доходит до находки у Rust (у Go и TS доходит) {#b-053}

| | |
|---|---|
| ##B053-ANCHOR **anchor** | паритет-таблица прохода №4 (`harvest/e14-b035-parity-pass.md`), под-асимметрия внутри B-025: SARIF-`justification` у признанной находки несёт человеческий текст причины в Go и TS и фиксированный маркер в Rust |
| ##B053-LOCATOR **locator** | растовые факты `UnsafeUse` / `UnwrapUse` / `EnvRead` несут только `in_deviation: bool` (`core-ai-native-conform/src/facts.rs`), тогда как `TsUnsafe` / `GoUnsafe` несут `reason: Option<String>`; рендер отмечен в `finding.rs` |
| ##B053-SEVERITY **severity** | P3 |
| ##B053-DISPOSITION **disposition** | `planned` — цена измерена при стройке B-025: ~33 места (3 поля факта + ~25 литералов в conform-крейте + 5 сайтов растового фронтенда) плюс ~20 строк логики извлечения и бамп версии фронтенда; **решение «не сейчас» принято сознательно**, чтобы одно поле не двигало весь фронт вместе с инвалидацией кэша |
| ##B053-FILED **filed by** | луп-проход №4 B-035, 2026-08-04 — он назвал этот пункт единственным открытым долгом волны Б без маршрута в бэклоге |

- ##B053-SUT **Суть, по-простому.** Когда гейт помечает находку «отступление признано», в SARIF полагается текст причины — тот самый, который автор написал в `#[spec(deviates = …, reason = …)]`. У Go и TypeScript этот текст доезжает до отчёта, у Rust — нет: растовые факты несут только булев флаг, и `justification` заполняется фиксированным маркером. Для будущей визуализации это разница между «здесь признанное отступление, вот почему» и «здесь признанное отступление». Стройка: довести `reason` до трёх растовых вариантов факта, вытащить его во фронтенде из атрибута, поднять версию фронтенда.
- ##B053-RELATED **Смежность.** B-025 построил сам механизм статуса (эта строка — только текст причины). Паритет-ось — B-035: пробел записан причиной и теперь маршрутом, а не молчанием.

### B-052 — три непостроенные половины R3-004: закрытый словарь токенов, один референт, отсутствие синонимов {#b-052}

| | |
|---|---|
| ##B052-ANCHOR **anchor** | `##NAMES-ARE-TOKEN-PROGRAMS` в трёх гайдах и карточка `rule-closed-vocabulary-naming` (Rust-стек): R3-004 состоит из четырёх утверждений, чекер построен ровно для ОДНОГО — композиции имени (`cell-name-is-computed`, батч 3 волны Б) |
| ##B052-LOCATOR **locator** | цензус `harvest/e13-r3-pending-cards-census.md` §Q4: закрытого словаря структурных токенов в дереве нет вообще (0 — ни данными, ни константой); реестра имён контрактной поверхности нет (частично покрывается `cell_types()` + `Fact::Item`); детектора синонимов/затенения нет (0) |
| ##B052-SEVERITY **severity** | P3 |
| ##B052-DISPOSITION **disposition** | `planned` — заведено, чтобы у трёх непостроенных половин была названная запись; **сегодня они честно помечены `@spec/done` («specified, not built»), а не `@impl/plan`**, потому что по правилу B-027 `@impl/plan` требует существующей записи бэклога. С появлением ЭТОЙ записи маркеры можно поднять до `@impl/plan` — но только сознательным решением, а не автоматически |
| ##B052-FILED **filed by** | док-срез E13-W10, батч 3 волны Б, 2026-08-04 |

- ##B052-SUT **Суть, по-простому.** «Имена — программы из токенов» (R3-004) утверждает четыре вещи: имя ячейки вычисляется из манифеста; структурные токены берутся из ЗАКРЫТОГО словаря; одно имя = один референт на контрактной поверхности; синонимов и затенения нет. Батч 3 построил первую — правило `cell-name-is-computed` на Rust и Go. Остальные три не построены, и владелец на развилке №1 сознательно выбрал именно вариант «вычисляемые имена», а не «свободные имена + линт словаря и уникальности» — то есть словарь не был отложен по недосмотру, он был не выбран. Эта запись существует, чтобы непостроенные половины имели дом, а не жили одной фразой в гайде. Стройка, если до неё дойдут руки: словарь токенов ДАННЫМИ (сегодня 0), реестр имён контрактной поверхности с предикатом «что считается контрактной поверхностью» (сегодня есть только `in_src`/`is_lib_root`), детектор синонимов и затенения.
- ##B052-RELATED **Смежность.** Развилка №1 карты (взята 2026-08-04: вычисляемые имена) — прямая причина, по которой эти половины остались вне стройки. Карточка `rule-closed-vocabulary-naming` несёт границу построенного явным разделом. Слаг карточки сознательно НЕ переименован: он месяцами цитируется в pending-списках трёх стеков, и от неверного прочтения защищает раздел границ внутри самой карточки.

### B-051 — у пилотного языка нет документа поверхности конформа, который есть у обеих его проекций {#b-051}

| | |
|---|---|
| ##B051-ANCHOR **anchor** | `packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/spec/go/tools/conform-frontend-go.md` и `…/typescript-ai-native-lang/v0.6.0/spec/typescript/tools/conform-frontend-typescript.md` существуют и описывают поверхность конформа своего стека; растового близнеца `…/rust-ai-native-lang/v0.7.0/spec/rust/tools/conform-frontend-rust.md` **нет** — в `spec/rust/tools/` лежат только два TCG-документа |
| ##B051-LOCATOR **locator** | найдено док-срезом батча 3 волны Б (2026-08-04), когда в три спеки поверхности надо было внести два новых корневых ключа конфига и оказалось, что вносить некуда; попутный замер: `max_file_lines` не описан НИ В ОДНОЙ спеке — он существует только в коде (`core-ai-native-conform/src/config.rs`) |
| ##B051-SEVERITY **severity** | P3 |
| ##B051-DISPOSITION **disposition** | `planned` |
| ##B051-FILED **filed by** | E13-W7, док-срез B-036, 2026-08-04 |

- ##B051-SUT **Суть, по-простому.** Дисциплина держит закон паритета для ПРОВЕРОК, но здесь асимметрия в документации: Go и TS описывают, что их фронтенд конформа читает и какой конфиг понимает, а Rust — пилотный язык, с которого проекции срисованы — такого документа не имеет. Практическое следствие уже наступило: два новых корневых ключа конфига (`invariant-comment-position`) удалось задокументировать в Go- и TS-спеках поверхности, а в Rust пришлось положить их в клаузу гайда, потому что дома для них нет. Стройка: авторить `conform-frontend-rust.md` по форме двух существующих близнецов, и заодно решить, где вообще живёт описание КОРНЕВЫХ (языко-нейтральных) ключей конфига — сегодня они не описаны нигде, включая `max_file_lines`, который старше всех.
- ##B051-RELATED **Смежность.** Паритет-ось — B-035 (таблица меряет проверки, а не документы; эта строка предлагает расширить взгляд). Конфиг-поверхность — B-029/B-034 (построили пер-языковые секции; корневой уровень остался неописанным).

### B-050 — типо-аварный вехикл кастомных линтов для Rust: dylint-библиотека и её toolchain {#b-050}

| | |
|---|---|
| ##B050-ANCHOR **anchor** | `GUIDE-AI-NATIVE-RUST.md` `##SCAFFOLD-F-STRUCTURED-DIAGNOSTICS` (:72) — клауза «custom clippy lints name the rule and the remedy»; та половина третьего канала, которую батч 3 не построил |
| ##B050-LOCATOR **locator** | цензус `harvest/e13-r2-custom-lints-census.md` §Q3/Q8: `dylint`, `declare_lint!`, `LateLintPass`, `rustc_private` — 0 в исходниках; `rust-toolchain.toml:2` пинит `channel = "stable"` и это ЕДИНСТВЕННЫЙ toolchain-файл дерева; на машине владельца (замер 2026-08-04) нет ни nightly, ни `cargo-dylint` |
| ##B050-SEVERITY **severity** | P3 |
| ##B050-DISPOSITION **disposition** | `planned` — **решение владельца 2026-08-04: «добавить в BACKLOG.md с низким приоритетом»**, то есть dylint сейчас НЕ строим и обещание гайда НЕ снимаем; вопрос про nightly возвращается вместе с этой строкой |
| ##B050-FILED **filed by** | босс-дизайн батча 3 волны Б, 2026-08-04 (`spec/design/new-rule-classes.md` §3) |

- ##B050-SUT **Суть, по-простому.** Гайд обещает три канала структурной диагностики; два построены (ошибки цитируют REQ; отчёты в SARIF), третий — свои линты — построен батчем 3 только для TypeScript (плагин `@typescript-eslint`, вехикл был уже в дереве). У Rust вехикл ровно один — `dylint`, чья библиотека линкуется с потрохами компилятора через `#![feature(rustc_private)]` и не собирается на stable, а stable мы пиним сознательно. Что при этом НЕ является пробелом: грамматика `violates REQ …; fix surface: …` у Rust соблюдается уже сегодня — единственный рендерер `req_message` и 19 мест его вызова в conform-движке, то есть слой «своих проверок с правильной формой сообщения» существует. Пробел ровно один и он назван: у Rust нет вехикла, ВИДЯЩЕГО ТИПЫ (conform читает синтаксис). Стройка, когда до неё дойдут руки: крейт линт-библиотеки с СОБСТВЕННЫМ nightly-пином внутри (рабочее пространство остаётся на stable) + шаг флора `cargo dylint` с рецептом установки при отсутствии инструмента — та же форма, в какой у Go живёт `staticcheck`, а у TS `eslint`.
- ##B050-RELATED **Смежность.** Go-половина того же канала: гайд Go не называет вехикла вообще («custom checks emit the same grammar»), а естественный носитель — свой `analysis.Analyzer` по образцу уже вызываемых флором `staticcheck`/`exhaustive`; едет этой же строкой. Встречная половина контура — B-026 (SARIF-ингест чужих линтеров). Паритет-ось — B-035: пробел записан причиной и маршрутом, не молчанием (`##PARITY-GAP-IS-NEVER-SILENT`).

### B-043 — генератор реестра может выдать один id двум кластерам {#b-043}

| | |
|---|---|
| ##B043-ANCHOR **anchor** | закон единого пространства находок — план кампании `#ids` («Ids continue the campaign's one finding space») |
| ##B043-LOCATOR **locator** | `campaigns/packages-2026-09/tasks/drift-registry.py` — сопоставление новых кластеров с прежним реестром не несёт ограничения уникальности: два кластера могут унаследовать один прежний id |
| ##B043-SEVERITY **severity** | P2 |
| ##B043-DISPOSITION **disposition** | `open` |
| ##B043-FILED **filed by** | ситтинг 2026-08-02, четвёртый обмен — воспроизведено вживую |
| ##B043-REPRO **воспроизведение** | пере-кластеризация после D30 (двойка TS-близнецов вошла в кэш) разнесла прежний F-154 на два кластера — и оба унаследовали id `F-154`; счётчик «newly assigned» при этом показал 1, дубль тихий. Разведено вручную тем же днём (кросс-файловый кластер → `F-355`), следующая регенерация несёт оба id стабильно; дефект в инструменте остался |
| ##B043-FIX **fix shape** | уникальное сопоставление (жадное по пересечению якорей с пометкой занятости прежнего id, либо честное паросочетание) и громкий отказ при коллизии вместо тихого дубля — рефьюз лучше двойника, как у merge-verdicts |

## P3 — accepted, no action planned {#p3}

### B-055 — вторая директива `#source` в документе проглатывается молча {#b-055}

| | |
|---|---|
| ##B055-ANCHOR **anchor** | `spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#source` — механика связывания контракта с реализацией |
| ##B055-LOCATOR **locator** | `crates/vibe-spec/src/pipeline.rs` — `first_source_directive` берёт **первую** директиву `#source` в тексте документа (`.find(…)`) и возвращает её; вызывающий код сворачивает ровно один сорс. Вторая и последующие директивы не читаются, предупреждения нет |
| ##B055-SEVERITY **severity** | P2 |
| ##B055-DISPOSITION **disposition** | `open` |
| ##B055-FILED **filed by** | вопрос владельца о числе сорсов у контракта, 2026-08-04 (волна В) |

- ##B055-SUT **Суть.** Автор, написавший в одном контрактном документе две директивы `#source`, получит слияние только с первой. Вторая не сработает и об этом никто не скажет — ни ошибки, ни предупреждения. Молчание здесь хуже отказа: автор уверен, что связал два источника, а связан один.
- ##B055-WHAT-IS-NOT-THE-DEFECT **Чего дефект НЕ означает.** Ограничение «один сорс на документ» само по себе может быть правильным (контракт — единица, у неё одна реализация). Пара — отношение документ↔документ, а не пакет↔пакет: контрактный **пакет** волен иметь много сорсов, каждый документ свой, хоть в разных пакетах. Дефект — не в числе, а в тишине.
- ##B055-FIX-SHAPE **Форма починки.** Либо отказ на второй директиве (одна на документ — контракт, нарушение громкое), либо честная поддержка нескольких с определённым порядком слияния. Первое дешевле и, вероятно, честнее; выбор — при постройке. @spec/done


### B-054 — файл тестов прогресс-команды стоит в тринадцати строках от бюджета {#b-054}

| | |
|---|---|
| ##B054-ANCHOR **anchor** | нет — найдено попутно при посадке B-010; ближайший закон — правило гейта `file-length` (`discipline://rust-ai-native-lang/guide#surface-form`, бюджет 600 строк) |
| ##B054-LOCATOR **locator** | `crates/vibe-cli/src/commands/progress/tests.rs` — **587 строк** после `cargo fmt` на HEAD этой записи (было 574 до B-010, который добавил тест разбора нового флага) |
| ##B054-SEVERITY **severity** | P3 |
| ##B054-DISPOSITION **disposition** | `accepted` — не нарушение и не долг: гейт зелёный. Записано, чтобы следующая правка этого файла не превратилась в внезапно-красную панель у того, кто её сделает |
| ##B054-FILED **filed by** | волна Г, посадка B-010, 2026-08-04 |

- ##B054-SUT **Суть.** Файл в тринадцати строках от блокирующего бюджета. Любая следующая правка — добавление одного теста — уронит панель у автора правки, и он потратит время на выяснение, при чём тут длина файла. Разрез по швам ответственности стоит дёшево сегодня и дорого в момент срабатывания.
- ##B054-WHY-NOT-NOW **Почему не сейчас.** Разрез файла тестов — не задача той работы, которая его обнаружила; делать его попутно значит смешать в одной посадке фикс поведения и рефакторинг чужого файла. Ждёт первой же работы, которая тронет этот файл по существу. @spec/done
- ##B054-THE-CLASS **Класс, а не случай.** Это второй раз за одну сессию: `xtask/src/mirror.rs` и `go-ai-native-cli/src/floor.rs` оба перевалили бюджет ПОСЛЕ форматирования, хотя воркеры мерили до него. Мерить длину надо после `cargo fmt`, и мерит её босс — у воркера этого шага нет. @impl/done


### B-042 — тестовая кодовая база для TCG-замеров: далёкое будущее, сейчас не строим {#b-042}

| | |
|---|---|
| ##B042-ANCHOR **anchor** | `TCG-ORACLE-GO-v0.1.md` `##QUANTITIES-ARE-CAMPAIGN-MEASURED` — аннотация 2026-08-02 несёт это решение прямо в тексте; тот же вопрос ждёт Rust/TS-замеров семьи F-215 |
| ##B042-LOCATOR **locator** | bench-станки всех трёх стеков готовы и параметризованы на корпус потребителя; Go-корпуса нет; растовый и TS-корпуса в `research/tcg-bench/` малы (9 и 7 кейсов) |
| ##B042-SEVERITY **severity** | P3 |
| ##B042-DISPOSITION **disposition** | `accepted` — **решение владельца 2026-08-02 (дословно): «создание тестовой кодовой базы, на которой мы будем делать замеры — это какая-то работа на далекое будущее. Например, тестовый код можно было бы сгенерировать через LLM или фаззером. Прямо сейчас мы такую базу делать не будем»** |
| ##B042-FILED **filed by** | рулинг предъявления F-167, 2026-08-02 |

- ##B042-SUT **Суть.** Замеры производительности TCG-оракулов требуют представительной кодовой базы-корпуса. Решено: не строить сейчас; направление на будущее — генерация корпуса LLM'ом или фаззером. Запись существует, чтобы отсутствие Go-замеров не переоткрывалось как новая находка.
- ##B042-STANDING-ANSWER **Стоячий ответ (владелец, 2026-08-02, второй раз):** «замеров нет и нескоро будет, нужно положить куда-нибудь в роадмап и больше не кошмарить меня вопросами "почему нет замеров"». Исполнено: все три complete-цели стеков (rust/go/ts) несут аннотацию «posted, not yet measured» с именем своего bench-станка и ссылкой на эту запись (батч D33); карта развития несёт тот же стоячий ответ. **Вопросы вида «почему нет замеров» владельцу больше не задаются — ответ здесь.** @doc/done
