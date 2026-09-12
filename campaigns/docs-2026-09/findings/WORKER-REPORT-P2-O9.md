# WORKER-REPORT-P2-O9 — линтер стиля и прогон промптов

Пакет: `campaigns/docs-2026-09/findings/PACKET-P2-O9.md` (A2.25, A2.29).
Ветка `research-preview-1-docs`, без push. Дата: 2026-09-12.

## Коротко для оркестратора

Оба атома сделаны, двумя коммитами, каждый по своим путям
(`git commit … -- <пути>`). Незакоммиченных правок у меня в дереве нет,
кроме этого отчёта.

**Главные числа линтера по руководству: 48 страниц, 15 чистых (31 %),
85 ошибок, 124 предупреждения, 0 нечитаемых.** Правил, давших ноль, пять
из десяти — и это не осечка линтера, а результат прозаической работы
кампании: первый абзац ни одной из 48 страниц не несёт термина глоссария
(у PP-C1 на `b1291b06` таких было 69), отсылок «see the specification»
нет ни одной, запрещённых заголовков нет, `!`/эмодзи/жирного в прозе нет,
промптов без ассертов нет. Полный список находок — очередь для F1 — в
конце отчёта.

`--prompts` прогнан вживую на настоящем руководстве с фальшивым агентом:
`start/first-project#first-project` — **раннер 0, все четыре ассерта
страницы 0, exit 0**; объявленный пропуск и отказ раннера тоже показаны
живьём. Настоящего агента не вызывал.

## Хэши и subject'ы

| Атом | Коммит | Subject |
|---|---|---|
| A2.25 | `7675efc9` | `feat(doc): lint documentation prose against the style law` |
| A2.29 | `316d2570` | `feat(doc): run documented prompts through an agent and assert the outcome` |

Этот отчёт идёт отдельным коммитом `docs(campaign): …` — продуктовый атом
остаётся одним коммитом и не может содержать собственный хэш.

## A2.25 — линтер стиля (`7675efc9`)

**Файлы.** `crates/vibe-doc/src/style.rs` и восемь модулей под
`style/` (`inline`, `prose`, `sentence`, `banned`, `glossary`, `terms`,
`rules`, `report`) с тестами; `crates/vibe-doc/src/{lib,error,manifest}.rs`;
`crates/vibe-cli/src/{cli/doc.rs,commands/doc.rs}` и новый
`crates/vibe-cli/src/commands/doc/tests.rs`.

### Инлайн читается один раз, и это несущее решение

Пивот держит инлайн одной строкой на узел (`##INLINE-STAYS-MARKDOWN`), так
что правило, читающее сырой текст, читает сразу три языка: прозу, кодовые
спаны и адреса ссылок. Поэтому текст разбирается **однажды** в `Inline`:
кодовый спан схлопывается в один символ-маску (U+FFFC — одно слово, ни
одной буквы), ссылка отдаёт свой адрес и оставляет текст, маркеры
выделения снимаются с запоминанием диапазонов.

Цена отказа от этого видна на корпусе: PP-C1 без маскирования нашёл 11
запрещённых слов, из них **пять** — слово `capabilities` внутри
`` `[requires] capabilities` ``, то есть имя поля манифеста в кодовом
начертании. Линтер, который это репортит, требует переименовать продукт.
С маскированием осталось **2** находки, обе настоящие (разбор ниже).

### Пороги X-028 — буквально

| Вид блока | Лимит | Уровень |
|---|---|---|
| элемент списка (шаг) | 20 слов | ошибка |
| `note kind="warning"` | 20 слов | ошибка |
| `p` внутри процедуры | 25 слов | ошибка |
| `p`, `quote`, `note`, `caption` (коридоры) | 35 слов | предупреждение |
| `prompt`, `needs`, `outcome` | нет лимита | — |
| `title`, ячейка таблицы | нет лимита | — |
| абзац длиннее 6 фраз | — | предупреждение |

**«Процедура» читается по форме, а не по заголовку.** Секция — процедура,
когда среди её блоков есть упорядоченный `list` или `p`, начинающийся с
`1.`/`2)`. Правило по имени секции (`by-hand`) перестало бы работать в тот
день, когда автор напишет «The steps»; в корпусе уже три разных написания.

**Фразы и слова.** Разбиение на фразы не считает точкой номер шага
(`1. Build the package. 2. Check it.` — две фразы, а не четыре) и знает
закрытый список сокращений (`e.g`, `i.e`, `etc`, …); версия `1.0.0` фразу
не рвёт, потому что после точки идёт цифра, а не пробел. Слово — токен
после маскирования, так что команда `vibe install org.vibevm.world/wal`
— одно слово, а не восемь: иначе каждая фраза с командой вылетала бы за
лимит по причине, с которой автор ничего не может сделать.

### Термины: три правила и одно честное сужение

Термины — секции `glossary/index.xml` самого пакета, читаются на каждом
прогоне (50 записей; терм `index (of a registry)` даёт слово `index`).
Множественное число — три регулярные формы (`registry` → `registries`);
длинный термин выигрывает у короткого (`freshness fingerprint` не
разваливается на `fingerprint`).

**Оговорка PROP-057 от 2026-09-12 применена ко всем трём правилам**:
шесть обычных слов (`package`, `project`, `kind`, `feature`, `workspace`,
`translation`) термами не считаются вовсе. Они названы в коде, а не в
данных пакета, потому что их называет сама норма; это её список, и седьмое
слово — правка правила, а не файла.

Введением термина считается: ссылка на его якорь глоссария; курсив (§8 —
знак термина в момент определения); глосса **в той же фразе сразу после
термина** — запятая, двоеточие, тире или скобка, за которыми идёт не
меньше четырёх слов. Последнее — сознательное сужение метода PP-C1,
который искал пояснительный оборот по всей фразе и сам записал, что
поэтому занижает число нарушений: «есть ли где-нибудь в этой фразе
запятая» отвечает «да» по неверной причине.

### Правила, которые в отчёт вошли нулём, и почему это правда

- **Термин в первом абзаце — 0.** Проверено руками на трёх страницах:
  `model/lock-and-store` пишет «One file in your project records exactly
  which package versions it got», избегая слова *lock file*; `model/registries`
  — «a place vibe knows how to read» вместо *registry*. Первые абзацы
  переписаны кампанией после прогона PP-C1, и правило это подтверждает.
- **Отсылки — 0.** Три фразы (`see the specification`, `refer to the spec`,
  `as described in`) стоят и в `banned.en.txt`; линтер отдаёт вхождение
  **один раз**, под правилом `deferral`, чтобы страница не получала одно
  замечание дважды. Прощаются они ровно в одном месте: когда следующий
  блок — `rule`. «The rule, in the specification's own words:» и затем
  сами слова — это цитата-**добавление**, которого §2 и требует; та же
  фраза без ничего — отсылка вместо объяснения.
- **Заголовки — 0.** Запрещены пять имён (Overview, Summary, Conclusion,
  Next steps, Key takeaways). Правила «заголовок-вопрос» я **не завёл**:
  пакет прямо говорит, что страница вопросов с вопросительными
  заголовками — не ошибка, а двенадцать таких заголовков `faq/index.xml`
  — это её жанр. Кандидат в BACKLOG, если центральная сессия захочет
  ловить вопросительный заголовок вне страницы вопросов.
- **Знаки — 0.** `!`, эмодзи, больше одного тире на абзац, жирный в прозе.
  Ячейки таблиц из этого правила выведены: жирный там размечает шапку, а
  стрелка — колонку состояний; это контейнер, а не проза.
- **Промпт без ассерта — 0.** Страница-сценарий определяется по форме
  скелета: `prompt` стоит **до первой секции** (`##STYLE-PAGE-SKELETON`:
  «then the prompt»). На такой странице каждый промпт обязан нести
  ассерты; иллюстративный `assert="none"` ниже по странице-объяснению
  правило не трогает. В корпусе все 20 промптов несут ассерты, и пивот это
  и так проверяет на разборе.

### Гейт по страницам и `--min`

«`--style` входит в `--min 100`» реализовано так: проверка считает **долю
страниц без ошибок** и сравнивает с `--min` — тем же флагом, которым
пользуется `--coverage`. 100 % (умолчание) — стоячая планка; меньшая — для
промежуточных прогонов кампании, которая ещё пишет страницы. Предупреждения
не гейтят никогда. Непрочитанная пивотом страница не проходит: её проза
неизвестна, а неизвестное — не чисто.

### Индекс читаемости

ARI (automated readability index), не Flesch: Flesch считает слоги, а
счётчик слогов — это словарь на язык, который проект был бы обязан вести
для каждой адаптации. ARI считает символы, слова и фразы — это есть в
любом языке. По руководству: **медиана 9,3; тяжелее всех
`authoring/bridge-a-repository.xml` — 15,2**. В отчёт, не в гейт.

### Фикстуры

Тест-фикстуры — сами пары «до/после» §9 (ловушка плотности, тики, отсылка,
процедура прозой) на двух страницах, «плохой» и «хорошей», плюс русская
страница со списком §10. Взяты именно они, потому что это собственное
утверждение закона о том, чего он хочет: линтер, который не отличает
«до» от «после», этот закон не реализует. «Хорошая» страница цитируется со
ссылками на глоссарий — §9 печатает абзацы как выдержки, а ссылка и есть
то, что делает выдержку страницей.

## A2.29 — прогон промптов (`316d2570`)

**Файлы.** `crates/vibe-doc/src/prompts.rs` + `prompts/{config,assert,report}.rs`
с тестами; `crates/vibe-cli/src/{cli/doc.rs,commands/doc.rs,commands/doc/tests.rs}`;
`vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0/prompts.toml` (новый
файл данных пакета, страниц не трогал).

### Где живёт `[doc.prompts]` и почему не в `vibe.toml`

Таблица лежит в `<пакет>/prompts.toml`, рядом с `examples/deferred.toml`,
и **не** в манифесте. Причина проверяемая: `ManifestWire`
(`crates/vibe-core/src/manifest/package/visibility.rs:300`) объявлен
`#[serde(deny_unknown_fields)]`, так что `[doc.prompts]` в `vibe.toml`
уронил бы чтение манифеста у всех; завести поле — это правка ядра, wire,
кодогенерации и схемы, то есть смена нормы, которую пакет запрещает.
Имя таблицы норма даёт — оно и сохранено.

### Почему фикстура промпта объявлена в конфигурации, а не на странице

PROP-045 §7 (`##ROW-DOCVOCAB-PROMPT`) закрывает `<prompt>` **одним**
атрибутом — `id`. Атрибут `fixture` был бы шестым элементом закрытого
словаря, то есть правкой нормы и пивота. Поэтому отображение «промпт →
состояние, из которого он стартует» лежит рядом с раннером, ключом
`page` + `id` — в той же форме строк, что и `deferred.toml`.

### Раннер — конфигурация, и он намеренно не назван

`runner` в `prompts.toml` оставлен закомментированным: один и тот же
корпус задуман к прогону через **двух** разных агентов, и промпт, который
работает только у одного, — это ровно то, что проверка должна ловить.
Назвать одного в файле значило бы тихо сделать его единственным, кого
кто-либо гоняет. Команда приходит флагом `--runner "<команда>"`.

Раннер стартует в рабочем каталоге песочницы; текст промпта идёт ему **на
стандартный вход** и лежит файлом; в окружении — `VIBE_DOC_PROMPT_FILE`,
`VIBE_DOC_DIR`, `VIBE_DOC_BINARY`, `VIBE_DOC_NEEDS`, плюс та же изоляция,
что у раннера примеров (`VIBE_SETTINGS` в песочницу, `NO_COLOR`, снятые
поведенческие переменные). Командная строка раннера разбирается как
программа и аргументы — не как строка оболочки.

### Ассерты — свой закрытый набор из трёх программ

Корпус пишет ровно `vibe`, `test` и `grep` (36 ассертов на 20 промптов).
Они диспетчеризуются здесь, а не отдаются оболочке, и причин две. Первая —
та же, что у раннера примеров. Вторая принадлежит этой проверке: она идёт
на Windows, где нет ни `test`, ни `grep`, и `test -f`, падающий из-за
отсутствия `test`, сообщает о работе агента как о сломанной — худший вид
красного. Это **свой** набор, а не расширение набора примеров: пример
показывает команду читателю и может показывать только `vibe`, ассерт —
проверка страницы на результат и вправе спрашивать, есть ли файл.
Реализованы `test [!] -f|-d|-e|-s`, `grep [-q] <regex> <path>` и `vibe` в
песочнице. Отсутствующий файл у `grep` — его собственный код 2, а не тихое
несовпадение: «агент ничего не написал» и «агент написал не то» — разные
отчёты.

### Вердикты и выборка

Четыре состояния: `held` (раннер 0 и все ассерты 0), `broke` (раннер 0,
ассерт не 0), `failed` (раннер не стартовал, упал или убит по таймауту),
`skipped` (иллюстративный `assert="none"`, объявленный `skip`, или вне
выборки). Хвост вывода агента печатается **для человека и никогда для
сравнения** — 12 последних строк, нормализованных правилами A2.9, чтобы в
отчёт не попал абсолютный путь машины.

`--sample N` — FNV-1a от адреса `page#id` как зерно, затем первые N по
этому порядку. Выборка не позиционная (тот же корпус в обратном порядке
даёт те же промпты — это проверяет тест) и повторяемая (два прогона через
месяц сравнимы, красный воспроизводим). Зерно выписано константой, чтобы
выборка не поехала от смены хешера в стандартной библиотеке.

**Проверка не в панели** — `tools/self-check.sh` шага не получил, по
прямому указанию нормы (`##STYLE-PROMPT-FIRST`): она зовёт настоящего
агента, идёт минутами и стоит денег.

### Трипвайр

`--prompts` снимает и сверяет то же состояние, что раннер примеров:
настоящий `~/.vibe` целиком и наблюдаемые места дерева-исходника. Здесь он
важнее, чем у примеров: пример запускает команду, которую показывает
страница, а тут в каталоге распущен **агент**. За все прогоны не сработал.

### Про «установленный скилл по `needs`»

Реализовано настройкой `[doc.prompts] skill`, которая проецирует скилл
пакета в проект песочницы перед вызовом раннера. В `prompts.toml`
руководства она **не выставлена**, и это решение, а не пропуск: у
настоящего агента скилл `vibevm` — часть **его** окружения, поставленная
один раз на машине, которая его запускает, и проверке нечего делать в
этой машине; `vibe mcp install` пакет разрешает только с `--dry-run`.
Текст `needs` со страницы уходит раннеру в `VIBE_DOC_NEEDS` дословно и
печатается в отчёте. Настройка существует для раннера-скрипта.

## Прогон промптов вживую, с фальшивым раннером

Фальшивый агент — питоновский скрипт в scratch (в репозиторий не кладу:
он делает ровно то, что нужно одному промпту, и это не документация).
Читает промпт со стандартного входа, сверяет его с файлом, и для
`start/first-project` делает буквально то, что промпт просит: `vibe init
hello-vibe`, затем `vibe install org.vibevm.world/wal --path hello-vibe
--assume-yes`. Любой другой промпт — громкий отказ с кодом 3.

```
$ vibe doc check --prompts --only "first-project" \
    --runner "python <scratch>/fake-agent.py" \
    --sandbox C:/vd9 --path <руководство>
  ok   start/first-project.xml#first-project [empty] runner exit 0
       assert 1 exit 0 — `test -f hello-vibe/vibe.toml`
       assert 2 exit 0 — `test -f hello-vibe/vibe.lock`
       assert 3 exit 0 — `grep -q "org.vibevm.world/wal" hello-vibe/vibe.lock`
       assert 4 exit 0 — `vibe check --path hello-vibe --quiet`

prompts: 1 held, 0 broke, 0 failed, 0 skipped, 0 unreadable page(s)
EXIT=0

$ vibe doc check --prompts --only "start/" …
  ok   start/first-project.xml#first-project [empty] runner exit 0
       (те же четыре ассерта)
  skip start/install-vibe.xml#install-vibe [empty]
       declared: installing vibe itself changes the machine, which is the one
       thing the sandbox forbids

prompts: 1 held, 0 broke, 0 failed, 1 skipped, 0 unreadable page(s)
EXIT=0

$ vibe doc check --prompts --only "install-a-package" …
  FAIL howto/install-a-package.xml#install-a-package [hello-vibe-empty] runner exit 3

howto/install-a-package.xml#install-a-package — the runner exited 3

prompts: 0 held, 0 broke, 1 failed, 0 skipped, 0 unreadable page(s)
error: a documented prompt no longer gets the result the page promises: 0 broke,
1 could not run, 0 page(s) unreadable (violates
spec://org.vibevm.core/vibevm/common/PROP-057#STYLE-PROMPT-FIRST;
fix: repair the product, or the prompt — never the assert)
EXIT=1
```

Три состояния показаны на настоящем корпусе: зелёное с четырьмя настоящими
ассертами страницы, объявленный пропуск, отказ раннера с ненулевым кодом
выхода. Механика — фикстура, песочница, стандартный вход, окружение,
ассерты, нормализация, трипвайр — прогнана целиком. Настоящего агента не
вызывал: это отдельный запуск центральной сессии.

## Вывод гейтов, дословно

Сборка и тесты гонялись с приватным каталогом сборки
(`CARGO_TARGET_DIR=<scratch>/target-p2o9`): дерево делят несколько
воркеров, и общий `target/debug/vibe.exe` они перелинковывают.

```
$ cargo fmt --all --check
FMT_EXIT=0

$ cargo build -p vibe-doc -p vibe-cli
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 37.02s

$ cargo test -p vibe-doc
test result: ok. 367 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 45 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out  (doctests)
  — из них 99 новых: 67 по `style`, 32 по `prompts`

$ cargo test -p vibe-cli --bins commands::doc
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 718 filtered out

$ cargo clippy -p vibe-doc -p vibe-cli --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s)
CLIPPY_EXIT=0

$ cargo xtask conform check        # фильтр по моим путям
(ноль строк: ни одной находки в crates/vibe-doc/**,
 crates/vibe-cli/src/cli/doc.rs, crates/vibe-cli/src/commands/doc{.rs,/**})
conform check: 85 finding(s) in scope <workspace>, 0 frozen in baseline, 27 new
conform: 27 crate(s) gated, 7 exempt
Error: conform: 27 new finding(s) against the baseline
  — все 27 предсуществующие и чужие; на первом прогоне их было 29, и обе
    лишние были мои: `file-length` (мои правки довели
    crates/vibe-cli/src/commands/doc.rs до 633 строк) и `no-unwrap-in-domain`
    (`.expect()` в `run_build`). Обе закрыты — разбор в «Отступлениях».

$ cargo xtask specmap
  drift: edges added: 76
specmap: wrote …\specmap.json (7930 spec units, 3507 tagged code items,
         3044 edges, 0 suspects, 31 warnings).
specmap: ratchet gate — 0 gated orphan(s), 0 dispositioned (6 crate(s) exempt).
specmap: resolve gate — 0 unresolved host edge(s), 35 non-host edge(s) …
SPECMAP_EXIT=0

$ vibe check --path vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0
vibe check: clean — every check passed
CHECK_EXIT=0
  — новый `prompts.toml` в корне пакета проверку не ломает

$ vibe doc check --style --path vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0
  (209 строк находок — полный список ниже)
  banned-word               2 error(s), 0 warning(s)
  sentence-length           7 error(s), 66 warning(s)
  paragraph-length          0 error(s), 4 warning(s)
  term-before-introduction  76 error(s), 0 warning(s)
  terms-per-sentence        0 error(s), 54 warning(s)
  readability (ARI)         median 9.3, hardest authoring/bridge-a-repository.xml at 15.2
style: 15 of 48 page(s) clean, 31% (threshold 100%), 85 error(s), 124 warning(s),
       0 unreadable page(s) [en]
error: the prose does not answer to the style law: 15 of 48 page(s) clean,
100 required, 85 error(s) (violates
spec://org.vibevm.core/vibevm/common/PROP-057#STYLE-LINT;
fix: rewrite the sentence — a false positive is fixed in the linter's rule,
with a BACKLOG entry, never worked around in the text)
DOCSTYLE_EXIT=1
```

Правила, давших ноль находок и потому не попавших в таблицу правил:
`term-in-first-paragraph`, `deferral`, `heading`, `sign`,
`prompt-without-assert`.

## Отступления от текста пакета

1. **`[doc.prompts]` лежит в `<пакет>/prompts.toml`, а не в `vibe.toml`.**
   Пакет говорит «конфигурация `[doc.prompts]` в пакете»; манифест
   строгий (`deny_unknown_fields`), и таблица там уронила бы чтение
   манифеста у всего проекта. Имя таблицы сохранено, файл лежит в пакете.
2. **Фикстура промпта — в конфигурации, а не атрибутом `<prompt>`.**
   PROP-045 §7 закрывает промпт одним атрибутом; шестой элемент закрытого
   словаря — правка нормы, которую пакет запрещает.
3. **Добавлен флаг `--runner`** (пакет перечислял только `--prompts` и
   `--sample N`). Без него единственный способ прогнать корпус вторым
   агентом — править файл в пакете, а нейтральность второго раннера —
   прямое требование плана A2.29.
4. **`skill` в `prompts.toml` руководства не выставлен** — разбор выше:
   скилл настоящего агента живёт на его машине, а `vibe mcp install`
   пакет разрешает только с `--dry-run`.
5. **Правила «заголовок-вопрос» нет.** Пакет выводит страницу вопросов
   из-под правила; самый честный способ не красить её жанр — не заводить
   правило. Кандидат в BACKLOG, если правило понадобится вне FAQ.
6. **Два чужих конформ-нарушения в `crates/vibe-cli/src/commands/doc.rs`
   закрыты мной.** `file-length` был вызван моими же правками (633 > 600):
   тестовый модуль уехал в `crates/vibe-cli/src/commands/doc/tests.rs` по
   образцу соседних команд, файл стал 492 строки. `no-unwrap-in-domain` —
   `.expect()` в `run_build`, написанный не мной (атом A2.18), но
   всплывший как новый после моей правки файла; заменён на `let … else` с
   отказом, цитирующим `spec://`. Обе правки — в файле, который я и так
   правлю; чужой работы не трогал.

## Аномалии и находки по пути (правок не делал)

1. **Два `not only` — настоящие находки, но не те тики.**
   `model/boot-lane.xml` p14 и `model/dependency-visibility.xml` p35 пишут
   «not only» в обычном смысле («lawful in any manifest, not only at the
   root»), а список §3 запрещает конструкцию «not only … but also». Список
   данных я не менял (запрещено пакетом). Решение — центральной сессии:
   либо переписать две фразы, либо завести в списке именно пару
   `not only … but also` как одну запись. Это ровно тот случай, о котором
   `##STYLE-LINT` говорит «ложное срабатывание чинится в правиле, с
   записью в BACKLOG, и никогда обходом в тексте».
2. **`term-before-introduction` — 76 ошибок, и это самая большая очередь
   фазы.** Разброс маленький: 33 страницы из 48, медиана 2 находки на
   страницу, максимум 7 (`model/packages-and-kinds.xml`). Чаще всего не
   введены `specification` (9 страниц), `manifest` (8), `registry` (6),
   `store` (5), `family` (4). Лечится либо ссылкой на глоссарий, либо
   полуфразой-глоссой — то и другое проза, воркеру запрещена.
3. **`terms-per-sentence` — 54 предупреждения.** Это не ошибки, и планку
   они не двигают; но это и есть та «неуместная плотность», ради которой
   §2 написан, и её стоит прочитать глазами на F1.
4. **`sentence-length` — 7 ошибок все внутри процедур** (лимит 25) и 66
   предупреждений в коридорах (лимит 35). Ни одного нарушения лимита 20 у
   шагов списка: корпус почти не пользуется `<list ordered="true">`,
   нумерованные процедуры записаны как `1. …` внутри `<p>` — именно то,
   что PP-C1 отметил в §3.2 как «не разросшаяся проза, а процедура в одном
   абзаце». Три таких абзаца дают четыре предупреждения
   `paragraph-length`; правило видит семь-шестнадцать «фраз» там, где
   читатель видит шаги. Кандидат в BACKLOG: либо разметить эти процедуры
   списками, либо не считать шаги фразами абзаца.
5. **`faq/index.xml` даёт 3 ошибки** — все `term-before-introduction`, ни
   одной за вопросительные заголовки: правило заголовков ловит только пять
   запрещённых имён.
6. **Самая тяжёлая страница по ARI — `authoring/bridge-a-repository.xml`
   (15,2 против медианы 9,3)**, и она же несёт 5 ошибок. Кандидат номер
   один на чтение вслух.

## Что не сделано и почему

1. **Страницы руководства не правлены** — прямой запрет пакета; все
   находки ушли в очередь ниже.
2. **Списки запрещённых слов и PROP-файлы не менялись.**
3. **Настоящий агентский прогон `--prompts` не делался** — пакет выводит
   его в отдельный запуск центральной сессии. Прогнана вся механика с
   фальшивым раннером.
4. **`vibe registry publish`, `vibe mcp install`, `vibe self` не
   запускались**; промпты против настоящего `~/.vibe` не гонялись —
   песочница `C:/vd9`, трипвайр не срабатывал.
5. **`bash tools/self-check.sh` целиком не гонялся** — по постоянному
   указанию оркестратора (панель гонит `cargo run -p vibe-cli` и
   перелинковывает общий `target/`). Шага для `--prompts` в панель я не
   добавлял: норма прямо выводит эту проверку из панели. Шага для
   `--style` тоже нет — файл панели трогают другие воркеры фазы, а команда
   стоит в этом отчёте; решение о шаге за центральной сессией.
6. **`specmap.json` не закоммичен** — гейт выполнен (0 suspects, 0 сирот,
   0 нерешённых хостовых рёбер), но регенерация втягивает дрейф
   параллельных воркеров; файл возвращён к HEAD, как это сделали P2-O2,
   P2-O4, P2-O6, P2-O7 и P2-O8. **Интегратору:** один прогон
   `cargo xtask specmap` и один коммит после посадки волны.
7. **Приватный каталог сборки удалён** в конце работы; `df -h` до и после
   — ниже.

## Диск

```
$ df -h /c    # в начале работы
C:              3.7T  3.5T  238G  94% /c

$ du -sh <scratch>/target-p2o9
21G

$ df -h /c    # перед удалением
C:              3.7T  3.5T  216G  95% /c

$ df -h /c    # после удаления каталога сборки и песочницы C:/vd9
C:              3.7T  3.5T  237G  94% /c
```

Приватный каталог сборки `<scratch>/target-p2o9` (21 ГБ) и песочница
промптов `C:/vd9` удалены. Ошибок линковки за всю работу не было.

## `git status --short` на момент сдачи

Моих в списке ровно один: этот отчёт — он уходит следующим коммитом.
Всё остальное чужое и не тронуто: `vibevm/vibepacks/org.vibevm.doc/web/**`
(воркеры P4-O2 и P4-O5) и их каталоги снимков
`campaigns/docs-2026-09/findings/P4-O2-shots/`, `…/P4-O5-shots/`.

Оба продуктовых коммита сделаны формой `git commit … -- <пути>` (новые
файлы предварительно добавлены `git add` по тем же путям); проверено
`git show --name-only` по каждому: чужих путей нет.

## Очередь для F1 — полный список находок линтера

Страница, блок `pNN`, правило, уровень и текст, который сработал. 209
строк: 85 ошибок и 124 предупреждения. Порядок — документный, как его
печатает сама проверка.

| страница | блок | вид | правило | уровень | что сработало | текст |
|---|---|---|---|---|---|---|
| `agent/ask-your-agent.xml` | `p02` | p | sentence-length | warning | sentence 2 is 37 words, and a p allows 35 | Under it, needs lists what the agent must have (the skill, network access, a token in the … |
| `agent/ask-your-agent.xml` | `p16` | p | sentence-length | warning | sentence 1 is 42 words, and a p allows 35 | The same handshake governs the build steps a project declares as agent work: when vibe run… |
| `agent/give-your-agent-the-skill.xml` | `p05` | p | sentence-length | warning | sentence 2 is 36 words, and a p allows 35 | `…` is a Model Context Protocol server that exposes the project's lock-derived state to an… |
| `agent/give-your-agent-the-skill.xml` | `p05` | p | terms-per-sentence | warning | 3 glossary terms in one sentence (family, skill, manifest) — three means the sentence is a container in disguise | `…` is a Model Context Protocol server that exposes the project's lock-derived state to an… |
| `agent/give-your-agent-the-skill.xml` | `p05` | p | term-before-introduction | error | `family` is used before it is introduced — link it to `glossary/index.xml#family` or gloss it in the same sentence | `…` is a Model Context Protocol server that exposes the project's lock-derived state to an… |
| `agent/give-your-agent-the-skill.xml` | `p05` | p | term-before-introduction | error | `manifest` is used before it is introduced — link it to `glossary/index.xml#manifest` or gloss it in the same sentence | `…` is a Model Context Protocol server that exposes the project's lock-derived state to an… |
| `agent/give-your-agent-the-skill.xml` | `p19` | p | terms-per-sentence | warning | 4 glossary terms in one sentence (skill, skill, skill, skill) — three means the sentence is a container in disguise | Packages can declare skills of their own, for any kind of package: a flow that ships a che… |
| `agent/how-agents-read-this-manual.xml` | `p15` | p | sentence-length | error | sentence 1 is 33 words, and a p allows 25 | 3. On a task, take the page's request block as your task, substitute the user's names and … |
| `agent/how-agents-read-this-manual.xml` | `p19` | p | sentence-length | warning | sentence 3 is 45 words, and a p allows 35 | `…` takes up to seven predicates, `…`, `…`, `…`, `…`, `…`, `…` and `…`, whitespace separat… |
| `agent/how-agents-read-this-manual.xml` | `p27` | p | sentence-length | warning | sentence 1 is 60 words, and a p allows 35 | The language packages of the `…` kind bring servers of their own with four tools, `…`, `…`… |
| `agent/how-agents-read-this-manual.xml` | `p37` | p | sentence-length | warning | sentence 1 is 37 words, and a p allows 35 | A page that exists in the source language but not in the language you asked for is served … |
| `agent/how-agents-read-this-manual.xml` | `p05` | p | term-before-introduction | error | `manifest` is used before it is introduced — link it to `glossary/index.xml#manifest` or gloss it in the same sentence | The site also serves the page manifest as JSON at `…`, with statuses, languages, audiences… |
| `agent/how-agents-read-this-manual.xml` | `p09` | p | terms-per-sentence | warning | 3 glossary terms in one sentence (anchor, anchor, block number) — three means the sentence is a container in disguise | Headings keep their named anchors as well, and a named anchor never changes once published… |
| `agent/how-agents-read-this-manual.xml` | `p11` | p | term-before-introduction | error | `specification` is used before it is introduced — link it to `glossary/index.xml#specification` or gloss it in the same sentence | Rules on a page are quoted from the specification by address and shown in the specificatio… |
| `architecture/how-vibe-is-built.xml` | `p01` | p | sentence-length | warning | sentence 1 is 37 words, and a p allows 35 | vibe is one binary built from a set of Rust libraries, each owning one concern: reading th… |
| `architecture/how-vibe-is-built.xml` | `p15` | p | sentence-length | error | sentence 1 is 26 words, and a p allows 25 | 3. Otherwise qualify every requested coordinate, build the solver's view of available vers… |
| `architecture/how-vibe-is-built.xml` | `p24` | p | sentence-length | warning | sentence 2 is 36 words, and a p allows 35 | Machine formats, the JSON reports, the lock file's records, the release manifests, are des… |
| `architecture/how-vibe-is-built.xml` | `p29` | p | sentence-length | warning | sentence 1 is 36 words, and a p allows 35 | The specifications are the authority: `…` for the foundational decisions, `…` for the load… |
| `architecture/how-vibe-is-built.xml` | `p02` | p | terms-per-sentence | warning | 4 glossary terms in one sentence (registry, mirror, override, index) — three means the sentence is a container in disguise | Registry: ordered package sources with mirrors, overrides and an optional index. |
| `architecture/how-vibe-is-built.xml` | `p02` | p | term-before-introduction | error | `override` is used before it is introduced — link it to `glossary/index.xml#override` or gloss it in the same sentence | Registry: ordered package sources with mirrors, overrides and an optional index. |
| `architecture/how-vibe-is-built.xml` | `p08` | p | term-before-introduction | error | `capability` is used before it is introduced — link it to `glossary/index.xml#capability` or gloss it in the same sentence | A capability lives in a library, and the command line, the terminal interface and the MCP … |
| `architecture/how-vibe-is-built.xml` | `p13` | p | terms-per-sentence | warning | 5 glossary terms in one sentence (manifest, lock file, registry, mirror, override) — three means the sentence is a container in disguise | 1. Discover the workspace root and read the manifests, the lock file, the user configurati… |
| `architecture/how-vibe-is-built.xml` | `p29` | p | terms-per-sentence | warning | 4 glossary terms in one sentence (specification, registry, store, lifecycle) — three means the sentence is a container in disguise | The specifications are the authority: `…` for the foundational decisions, `…` for the load… |
| `architecture/traceability.xml` | `p08` | p | sentence-length | warning | sentence 1 is 37 words, and a p allows 35 | `…` walks the crates for marks and the specification tree for units, and writes `…`: nodes… |
| `architecture/traceability.xml` | `p08` | p | term-before-introduction | error | `specification` is used before it is introduced — link it to `glossary/index.xml#specification` or gloss it in the same sentence | `…` walks the crates for marks and the specification tree for units, and writes `…`: nodes… |
| `architecture/what-the-lifecycle-epic-delivered.xml` | `p15` | p | sentence-length | warning | sentence 1 is 43 words, and a p allows 35 | The campaign also ran execution lanes that are no longer current: a subscription-backed ex… |
| `architecture/what-the-lifecycle-epic-delivered.xml` | `p06` | p | terms-per-sentence | warning | 3 glossary terms in one sentence (contribution, provider, provider) — three means the sentence is a container in disguise | One extension plane: scheduled contributions answer when, sibling mechanism providers answ… |
| `architecture/what-the-lifecycle-epic-delivered.xml` | `p06` | p | term-before-introduction | error | `phase` is used before it is introduced — link it to `glossary/index.xml#phase` or gloss it in the same sentence | One nine-phase line: dependency materialisation is `…`, placement outside the project is `… |
| `architecture/what-the-lifecycle-epic-delivered.xml` | `p06` | p | term-before-introduction | error | `provider` is used before it is introduced — link it to `glossary/index.xml#provider` or gloss it in the same sentence | One extension plane: scheduled contributions answer when, sibling mechanism providers answ… |
| `architecture/what-the-lifecycle-epic-delivered.xml` | `p12` | p | terms-per-sentence | warning | 3 glossary terms in one sentence (override, phase, lifecycle) — three means the sentence is a container in disguise | Deploy targets beyond the first genres, a WebAssembly extension tier, per-language and per… |
| `architecture/what-the-lifecycle-epic-delivered.xml` | `p12` | p | term-before-introduction | error | `specification` is used before it is introduced — link it to `glossary/index.xml#specification` or gloss it in the same sentence | Each is named in its specification with a compatibility law, so the door stays open withou… |
| `authoring/bridge-a-repository.xml` | `p03` | p | sentence-length | warning | sentence 1 is 40 words, and a p allows 35 | The agent scaffolds the package with `…`, marks it as a bridge in the manifest, and declar… |
| `authoring/bridge-a-repository.xml` | `p12` | p | sentence-length | warning | sentence 1 is 45 words, and a p allows 35 | A declared source is authenticated twice when it arrives: the checkout must be at the decl… |
| `authoring/bridge-a-repository.xml` | `p15` | p | sentence-length | warning | sentence 1 is 39 words, and a p allows 35 | `…` in the manifest names the people who wrote the bridge, its metadata and adapters, neve… |
| `authoring/bridge-a-repository.xml` | `p19` | p | sentence-length | warning | sentence 1 is 36 words, and a p allows 35 | vibe fetches a package's submodules when it fetches the package, updates them with it, and… |
| `authoring/bridge-a-repository.xml` | `p19` | p | sentence-length | warning | sentence 2 is 37 words, and a p allows 35 | Publication is the boundary: a registry repository is a self-contained snapshot, so the pu… |
| `authoring/bridge-a-repository.xml` | `p03` | p | term-before-introduction | error | `store` is used before it is introduced — link it to `glossary/index.xml#store` or gloss it in the same sentence | Installing the package fetches that exact source into the consumer's machine store beside … |
| `authoring/bridge-a-repository.xml` | `p07` | p | term-before-introduction | error | `hook` is used before it is introduced — link it to `glossary/index.xml#hook` or gloss it in the same sentence | This is the cheapest bridge, plain files and the flag, with hooks if the layout needs shap… |
| `authoring/bridge-a-repository.xml` | `p15` | p | term-before-introduction | error | `index` is used before it is introduced — link it to `glossary/index.xml#index-registry` or gloss it in the same sentence | `…` in the manifest names the people who wrote the bridge, its metadata and adapters, neve… |
| `authoring/bridge-a-repository.xml` | `p15` | p | term-before-introduction | error | `coordinate` is used before it is introduced — link it to `glossary/index.xml#coordinate` or gloss it in the same sentence | The coordinate names the product a consumer installs, not the codebase the packaging was w… |
| `authoring/bridge-a-repository.xml` | `p19` | p | term-before-introduction | error | `registry` is used before it is introduced — link it to `glossary/index.xml#registry` or gloss it in the same sentence | Publication is the boundary: a registry repository is a self-contained snapshot, so the pu… |
| `authoring/facts-and-status-markers.xml` | `p14` | p | paragraph-length | warning | 7 sentences in one paragraph, and the limit is 6 | There are four places. A document marker stands in the preamble, or right after the first … |
| `authoring/ship-tools-and-mcp-servers.xml` | `p03` | p | sentence-length | warning | sentence 1 is 36 words, and a p allows 35 | The agent scaffolds the package slot with `…`, puts the crate inside it, adds a `…` table … |
| `authoring/ship-tools-and-mcp-servers.xml` | `p14` | p | term-before-introduction | error | `family` is used before it is introduced — link it to `glossary/index.xml#family` or gloss it in the same sentence | The `…` is unique within the package and should be safe from collisions across packages, w… |
| `authoring/ship-tools-and-mcp-servers.xml` | `p27` | p | term-before-introduction | error | `phase` is used before it is introduced — link it to `glossary/index.xml#phase` or gloss it in the same sentence | The working directory is the package's own folder in the dependency tree, and the environm… |
| `authoring/ship-tools-and-mcp-servers.xml` | `p27` | p | term-before-introduction | error | `hook` is used before it is introduced — link it to `glossary/index.xml#hook` or gloss it in the same sentence | A hook's edits to files vibe owns are ephemeral: a reinstall or an update restores those b… |
| `authoring/specs-agents-can-cite.xml` | `p21` | p | sentence-length | warning | sentence 3 is 52 words, and a p allows 35 | Before writing, it converts the result back and compares: a byte-identical round trip conv… |
| `authoring/specs-agents-can-cite.xml` | `p42` | p | sentence-length | warning | sentence 1 is 37 words, and a p allows 35 | Code may cite specifications too, with an attribute on the item that implements a rule, an… |
| `authoring/specs-agents-can-cite.xml` | `p02` | p | term-before-introduction | error | `specification` is used before it is introduced — link it to `glossary/index.xml#specification` or gloss it in the same sentence | The specification tree is the only channel between them, and a channel works when a messag… |
| `authoring/specs-agents-can-cite.xml` | `p26` | p | term-before-introduction | error | `manifest` is used before it is introduced — link it to `glossary/index.xml#manifest` or gloss it in the same sentence | Every file a directive names must be declared in the package's manifest. |
| `authoring/translate-documentation.xml` | `p12` | p | sentence-length | warning | sentence 1 is 38 words, and a p allows 35 | A translation mirrors blocks, not sentences: within a block the translator writes what a n… |
| `authoring/translate-documentation.xml` | `p17` | p | sentence-length | warning | sentence 2 is 42 words, and a p allows 35 | When the source changes, the structural check still passes as long as the shape did, and w… |
| `authoring/translate-documentation.xml` | `p03` | p | term-before-introduction | error | `subject` is used before it is introduced — link it to `glossary/index.xml#subject` or gloss it in the same sentence | It sets the package language, names the source in `…` and the same subject in `…`. |
| `authoring/translate-documentation.xml` | `p12` | p | term-before-introduction | error | `mirror` is used before it is introduced — link it to `glossary/index.xml#mirror` or gloss it in the same sentence | A translation mirrors blocks, not sentences: within a block the translator writes what a n… |
| `authoring/translate-documentation.xml` | `p15` | p | term-before-introduction | error | `specification` is used before it is introduced — link it to `glossary/index.xml#specification` or gloss it in the same sentence | Rules quoted from a specification stay in the specification's language, marked as such; tr… |
| `authoring/write-a-feat-or-stack.xml` | `p08` | p | sentence-length | warning | sentence 1 is 36 words, and a p allows 35 | A stack is a technology context: it says how the abstract abilities a feat asks for are re… |
| `authoring/write-a-feat-or-stack.xml` | `p03` | p | terms-per-sentence | warning | 4 glossary terms in one sentence (manifest, specification, capability, capability) — three means the sentence is a container in disguise | The agent creates both slots with `…`, sets their kinds in the manifests, writes the feat'… |
| `authoring/write-a-feat-or-stack.xml` | `p08` | p | terms-per-sentence | warning | 3 glossary terms in one sentence (manifest, specification, capability) — three means the sentence is a container in disguise | Its manifest declares what it provides, `…`, and its specification documents describe each… |
| `authoring/write-a-feat-or-stack.xml` | `p08` | p | term-before-introduction | error | `phase` is used before it is introduced — link it to `glossary/index.xml#phase` or gloss it in the same sentence | A stack is a technology context: it says how the abstract abilities a feat asks for are re… |
| `authoring/write-a-feat-or-stack.xml` | `p10` | p | terms-per-sentence | warning | 3 glossary terms in one sentence (lifecycle, contribution, manifest) — three means the sentence is a container in disguise | A stack may also bind lifecycle contributions in its manifest, so that `…` and `…` in a co… |
| `authoring/write-a-feat-or-stack.xml` | `p14` | p | term-before-introduction | error | `family` is used before it is introduced — link it to `glossary/index.xml#family` or gloss it in the same sentence | The word `…` also names a family bundle: a package of kind `…` with nothing but exact pins… |
| `authoring/write-a-flow.xml` | `p03` | p | paragraph-length | warning | 7 sentences in one paragraph, and the limit is 6 | The agent runs `…`, which adds a package slot to the project at `…`: a manifest with a `…`… |
| `authoring/write-a-flow.xml` | `p28` | p | sentence-length | warning | sentence 2 is 41 words, and a p allows 35 | `…` opts into a split between `…`, small and cheap to load like a header, and `…`, the hea… |
| `authoring/write-a-lang-package.xml` | `p08` | p | term-before-introduction | error | `family` is used before it is introduced — link it to `glossary/index.xml#family` or gloss it in the same sentence | A language guide that also ships tools, a checker, a formatter, a type oracle, becomes a f… |
| `authoring/write-documentation.xml` | `p20` | p | sentence-length | warning | sentence 1 is 46 words, and a p allows 35 | A concept page has a noun as its title, opens with a paragraph that uses no term of the gl… |
| `authoring/write-documentation.xml` | `p20` | p | sentence-length | warning | sentence 2 is 40 words, and a p allows 35 | A task page has an imperative title, opens the same way, and then gives the request for an… |
| `authoring/write-documentation.xml` | `p36` | p | sentence-length | warning | sentence 1 is 43 words, and a p allows 35 | A package with no documentation at all is still shown: the site renders any published vers… |
| `authoring/write-documentation.xml` | `p03` | p | terms-per-sentence | warning | 3 glossary terms in one sentence (subject, subject, official documentation) — three means the sentence is a container in disguise | The manual is published like any package; the site finds it through the `…` edge and, beca… |
| `authoring/write-documentation.xml` | `p03` | p | term-before-introduction | error | `specification` is used before it is introduced — link it to `glossary/index.xml#specification` or gloss it in the same sentence | `…` resolves every `…` address against the subject's specification and fails on any that d… |
| `authoring/write-documentation.xml` | `p05` | p | terms-per-sentence | warning | 3 glossary terms in one sentence (subject, skill, mirror) — three means the sentence is a container in disguise | A `…` package must name at least one subject and carry a `…` and an `…`; it may declare a … |
| `authoring/write-documentation.xml` | `p05` | p | term-before-introduction | error | `mirror` is used before it is introduced — link it to `glossary/index.xml#mirror` or gloss it in the same sentence | A `…` package must name at least one subject and carry a `…` and an `…`; it may declare a … |
| `authoring/write-documentation.xml` | `p27` | p | term-before-introduction | error | `fact` is used before it is introduced — link it to `glossary/index.xml#fact` or gloss it in the same sentence | Every `…` is a live citation without a pin: the page shows the fact's current text at ever… |
| `authoring/write-documentation.xml` | `p27` | p | term-before-introduction | error | `anchor` is used before it is introduced — link it to `glossary/index.xml#anchor` or gloss it in the same sentence | Every `…` is a live citation without a pin: the page shows the fact's current text at ever… |
| `authoring/write-documentation.xml` | `p36` | p | terms-per-sentence | warning | 4 glossary terms in one sentence (manifest, boot snippet, anchor, skill) — three means the sentence is a container in disguise | A package with no documentation at all is still shown: the site renders any published vers… |
| `authoring/write-documentation.xml` | `p36` | p | term-before-introduction | error | `manifest` is used before it is introduced — link it to `glossary/index.xml#manifest` or gloss it in the same sentence | A package with no documentation at all is still shown: the site renders any published vers… |
| `faq/index.xml` | `p04` | p | paragraph-length | warning | 7 sentences in one paragraph, and the limit is 6 | First read the resolver's explanation: it names the two constraints that disagree, and ove… |
| `faq/index.xml` | `p08` | p | sentence-length | warning | sentence 3 is 36 words, and a p allows 35 | When a step needs reasoning, vibe parks an instruction for the agent that hosts it, or, at… |
| `faq/index.xml` | `p18` | p | sentence-length | warning | sentence 1 is 41 words, and a p allows 35 | Because a tool that knows what a package is for before opening it can refuse the wrong thi… |
| `faq/index.xml` | `p08` | p | term-before-introduction | error | `provider` is used before it is introduced — link it to `glossary/index.xml#provider` or gloss it in the same sentence | When a step needs reasoning, vibe parks an instruction for the agent that hosts it, or, at… |
| `faq/index.xml` | `p12` | p | terms-per-sentence | warning | 4 glossary terms in one sentence (store, registry, lock file, store) — three means the sentence is a container in disguise | Not while the machine store holds the packages: a stored version is usable even when no re… |
| `faq/index.xml` | `p12` | p | terms-per-sentence | warning | 3 glossary terms in one sentence (mirror, lock file, mirror) — three means the sentence is a container in disguise | For the long run, `…` writes a mirror folder of everything the lock file references, which… |
| `faq/index.xml` | `p12` | p | term-before-introduction | error | `mirror` is used before it is introduced — link it to `glossary/index.xml#mirror` or gloss it in the same sentence | For the long run, `…` writes a mirror folder of everything the lock file references, which… |
| `faq/index.xml` | `p16` | p | terms-per-sentence | warning | 4 glossary terms in one sentence (fingerprint, lock file, mirror, override) — three means the sentence is a container in disguise | No. A fingerprint mismatch means the bytes served are not the bytes the lock file remember… |
| `faq/index.xml` | `p16` | p | term-before-introduction | error | `override` is used before it is introduced — link it to `glossary/index.xml#override` or gloss it in the same sentence | No. A fingerprint mismatch means the bytes served are not the bytes the lock file remember… |
| `howto/install-a-package.xml` | `p03` | p | paragraph-length | warning | 7 sentences in one paragraph, and the limit is 6 | The agent runs `…`. vibe walks the project's registries in order and asks the first one th… |
| `howto/install-a-package.xml` | `p23` | p | sentence-length | warning | sentence 1 is 47 words, and a p allows 35 | A package that delivers tools is recorded at install and built on demand: `…` compiles the… |
| `howto/publish-a-package.xml` | `p12` | p | sentence-length | error | sentence 1 is 33 words, and a p allows 25 | vibe looks for the token in a fixed order and takes the first it finds: `…` for the regist… |
| `howto/publish-a-package.xml` | `p12` | p | sentence-length | error | sentence 2 is 34 words, and a p allows 25 | The token is a surface secret: it is never printed, never logged, never written to a file … |
| `howto/publish-a-package.xml` | `p22` | p | sentence-length | warning | sentence 3 is 40 words, and a p allows 35 | A consumer who already resolved that version keeps the exact bytes the lock file recorded,… |
| `howto/publish-a-package.xml` | `p26` | p | sentence-length | warning | sentence 1 is 49 words, and a p allows 35 | The publisher never rewrites the registry's history: every publish commit is a child of th… |
| `howto/read-documentation-locally.xml` | `p13` | p | sentence-length | warning | sentence 2 is 36 words, and a p allows 35 | A `…` you built from source carries a plain fallback; on the first `…` it offers to downlo… |
| `howto/read-documentation-locally.xml` | `p15` | p | term-before-introduction | error | `subject` is used before it is introduced — link it to `glossary/index.xml#subject` or gloss it in the same sentence | Warming a documentation package warms its subjects too, so the rules a page quotes resolve… |
| `howto/remove-a-package.xml` | `p09` | p | terms-per-sentence | warning | 3 glossary terms in one sentence (manifest, lock file, store) — three means the sentence is a container in disguise | `…` deletes the dependency tree and the generated boot files and keeps the manifest, the l… |
| `howto/remove-a-package.xml` | `p16` | p | term-before-introduction | error | `hook` is used before it is introduced — link it to `glossary/index.xml#hook` or gloss it in the same sentence | Removing a package does not undo what its install script did, if it had one: hook effects … |
| `howto/set-up-a-workspace.xml` | `p03` | p | sentence-length | warning | sentence 1 is 45 words, and a p allows 35 | The agent adds a `…` table to the root manifest naming the member paths, and writes each m… |
| `howto/set-up-a-workspace.xml` | `p19` | p | sentence-length | warning | sentence 2 is 40 words, and a p allows 35 | A member requires another by path during development and by version once published, in one… |
| `howto/set-up-a-workspace.xml` | `p23` | p | sentence-length | warning | sentence 2 is 38 words, and a p allows 35 | `…` walks the members dependency-first, skips `…`, and stops at the first failure with a r… |
| `howto/update-packages.xml` | `p15` | p | terms-per-sentence | warning | 3 glossary terms in one sentence (registry, manifest, index) — three means the sentence is a container in disguise | `…` reads the registries declared in the project's manifest, and for the fastest answer th… |
| `howto/use-a-private-registry.xml` | `p07` | p | sentence-length | error | sentence 2 is 29 words, and a p allows 25 | The values are `…` for public read, `…` for keys, `…` for a token in an environment variab… |
| `howto/use-a-private-registry.xml` | `p12` | p | sentence-length | warning | sentence 2 is 45 words, and a p allows 35 | Instead of the package, the repository named after it carries one file, `…`, with a `…` ta… |
| `howto/use-a-private-registry.xml` | `p26` | p | term-before-introduction | error | `mirror` is used before it is introduced — link it to `glossary/index.xml#mirror` or gloss it in the same sentence | A mirror is not a second registry. |
| `howto/work-offline.xml` | `p16` | p | sentence-length | warning | sentence 1 is 44 words, and a p allows 35 | If an offline install refuses a package the store already holds, the project's registries … |
| `howto/work-offline.xml` | `p14` | p | terms-per-sentence | warning | 3 glossary terms in one sentence (registry, lock file, mirror) — three means the sentence is a container in disguise | For a machine that never sees the registry, `…` writes a folder holding every package the … |
| `howto/work-offline.xml` | `p14` | p | term-before-introduction | error | `mirror` is used before it is introduced — link it to `glossary/index.xml#mirror` or gloss it in the same sentence | For a machine that never sees the registry, `…` writes a folder holding every package the … |
| `howto/work-offline.xml` | `p16` | p | terms-per-sentence | warning | 3 glossary terms in one sentence (store, registry, mirror) — three means the sentence is a container in disguise | If an offline install refuses a package the store already holds, the project's registries … |
| `howto/work-offline.xml` | `p19` | p | terms-per-sentence | warning | 4 glossary terms in one sentence (store, registry, store, coordinate) — three means the sentence is a container in disguise | A package in the store is usable even when its registry no longer lists it; a package that… |
| `lifecycle/build-package-deploy.xml` | `p07` | p | sentence-length | error | sentence 1 is 31 words, and a p allows 25 | The mechanism `…` places the executable as a launcher in vibe's own `…` folder on this mac… |
| `lifecycle/build-package-deploy.xml` | `p03` | p | term-before-introduction | error | `phase` is used before it is introduced — link it to `glossary/index.xml#phase` or gloss it in the same sentence | `…` runs the whole default lifecycle in planning mode and prints what each phase would do,… |
| `lifecycle/build-package-deploy.xml` | `p07` | p | term-before-introduction | error | `specification` is used before it is introduced — link it to `glossary/index.xml#specification` or gloss it in the same sentence | The mechanism `…` places the executable as a launcher in vibe's own `…` folder on this mac… |
| `lifecycle/build-package-deploy.xml` | `p18` | p | terms-per-sentence | warning | 3 glossary terms in one sentence (deploy profile, provider, specification) — three means the sentence is a container in disguise | A deploy profile names its targets in order and the provider that applies each: a folder o… |
| `lifecycle/build-package-deploy.xml` | `p18` | p | term-before-introduction | error | `provider` is used before it is introduced — link it to `glossary/index.xml#provider` or gloss it in the same sentence | A deploy profile names its targets in order and the provider that applies each: a folder o… |
| `lifecycle/extensions-and-providers.xml` | `p10` | p | sentence-length | warning | sentence 1 is 38 words, and a p allows 35 | Every handler receives one context envelope, a versioned JSON document that names the proj… |
| `lifecycle/extensions-and-providers.xml` | `p12` | p | sentence-length | warning | sentence 1 is 45 words, and a p allows 35 | A native handler is a dynamic library with exactly four C symbols; the request and the rep… |
| `lifecycle/extensions-and-providers.xml` | `p28` | p | sentence-length | warning | sentence 3 is 36 words, and a p allows 35 | At a terminal, vibe can call a configured model provider, the first one being any OpenAI-c… |
| `lifecycle/extensions-and-providers.xml` | `p03` | p | terms-per-sentence | warning | 3 glossary terms in one sentence (contribution, handler, manifest) — three means the sentence is a container in disguise | A contribution binds a handler to a point and is declared in a manifest as an `…` table, i… |
| `lifecycle/extensions-and-providers.xml` | `p03` | p | term-before-introduction | error | `family` is used before it is introduced — link it to `glossary/index.xml#family` or gloss it in the same sentence | The `…` family is the nine phases. |
| `lifecycle/extensions-and-providers.xml` | `p03` | p | term-before-introduction | error | `phase` is used before it is introduced — link it to `glossary/index.xml#phase` or gloss it in the same sentence | The `…` family is the nine phases. |
| `lifecycle/extensions-and-providers.xml` | `p03` | p | term-before-introduction | error | `handler` is used before it is introduced — link it to `glossary/index.xml#handler` or gloss it in the same sentence | A contribution binds a handler to a point and is declared in a manifest as an `…` table, i… |
| `lifecycle/extensions-and-providers.xml` | `p06` | p | terms-per-sentence | warning | 3 glossary terms in one sentence (contribution, override, handler) — three means the sentence is a container in disguise | A contribution carries an id, which is also the key a project uses to disable or override … |
| `lifecycle/extensions-and-providers.xml` | `p06` | p | term-before-introduction | error | `override` is used before it is introduced — link it to `glossary/index.xml#override` or gloss it in the same sentence | A contribution carries an id, which is also the key a project uses to disable or override … |
| `lifecycle/extensions-and-providers.xml` | `p10` | p | terms-per-sentence | warning | 3 glossary terms in one sentence (handler, phase, contribution) — three means the sentence is a container in disguise | Every handler receives one context envelope, a versioned JSON document that names the proj… |
| `lifecycle/extensions-and-providers.xml` | `p18` | p | terms-per-sentence | warning | 3 glossary terms in one sentence (contribution, phase, contribution) — three means the sentence is a container in disguise | Installing a package is the consent to run its contributions: a dependency's phase and slo… |
| `lifecycle/extensions-and-providers.xml` | `p18` | p | terms-per-sentence | warning | 3 glossary terms in one sentence (manifest, contribution, override) — three means the sentence is a container in disguise | The consuming manifest can activate a contribution that is not automatic, disable one by i… |
| `lifecycle/phases.xml` | `p13` | p | sentence-length | warning | sentence 1 is 38 words, and a p allows 35 | `…` reports what a phase run would do and changes nothing; every real run prints, before e… |
| `lifecycle/phases.xml` | `p03` | p | term-before-introduction | error | `phase` is used before it is introduced — link it to `glossary/index.xml#phase` or gloss it in the same sentence | `…` has one phase and removes derived state. |
| `lifecycle/phases.xml` | `p10` | p | terms-per-sentence | warning | 3 glossary terms in one sentence (phase, fingerprint, phase) — three means the sentence is a container in disguise | Every phase run records a fingerprint of the inputs it declared; the next run skips a phas… |
| `lifecycle/phases.xml` | `p13` | p | terms-per-sentence | warning | 3 glossary terms in one sentence (phase, contribution, handler) — three means the sentence is a container in disguise | `…` reports what a phase run would do and changes nothing; every real run prints, before e… |
| `lifecycle/phases.xml` | `p17` | p | terms-per-sentence | warning | 4 glossary terms in one sentence (contribution, manifest, contribution, override) — three means the sentence is a container in disguise | Installing a package is the consent for its contributions to run, and the manifest of the … |
| `lifecycle/phases.xml` | `p17` | p | term-before-introduction | error | `override` is used before it is introduced — link it to `glossary/index.xml#override` or gloss it in the same sentence | Installing a package is the consent for its contributions to run, and the manifest of the … |
| `lifecycle/scrape.xml` | `p03` | p | sentence-length | warning | sentence 2 is 37 words, and a p allows 35 | It runs `…`, which reads the contract, classifies every file the tool ever wrote or marked… |
| `lifecycle/scrape.xml` | `p07` | p | sentence-length | error | sentence 2 is 29 words, and a p allows 25 | A fresh project has no modification baseline for vibe's files, so the two `…` rules refuse… |
| `lifecycle/scrape.xml` | `p22` | p | sentence-length | warning | sentence 1 is 46 words, and a p allows 35 | A scrape never deletes a path blindly: a build script that calls a vibe tool, generated pr… |
| `lifecycle/scrape.xml` | `p03` | p | terms-per-sentence | warning | 4 glossary terms in one sentence (manifest, lock file, managed block, specification) — three means the sentence is a container in disguise | The copy is a plain project of its language: no manifest, no lock file, no dependency tree… |
| `lifecycle/scrape.xml` | `p03` | p | term-before-introduction | error | `specification` is used before it is introduced — link it to `glossary/index.xml#specification` or gloss it in the same sentence | The copy is a plain project of its language: no manifest, no lock file, no dependency tree… |
| `model/boot-lane.xml` | `p14` | p | banned-word | error | `not only` is on this language's list | Every package in the dependency tree carries its own boot files, what was compiled into it… |
| `model/boot-lane.xml` | `p20` | p | sentence-length | warning | sentence 1 is 36 words, and a p allows 35 | vibe computes the sequence for each project from the resolved dependency graph: the founda… |
| `model/boot-lane.xml` | `p26` | p | sentence-length | warning | sentence 1 is 36 words, and a p allows 35 | A snippet that declares a condition is always a dynamic entry, whatever link type the proj… |
| `model/boot-lane.xml` | `p11` | p | term-before-introduction | error | `contribution` is used before it is introduced — link it to `glossary/index.xml#contribution` or gloss it in the same sentence | `…` is the default: the contribution becomes a path in `…`, read on demand, and gated by a… |
| `model/boot-lane.xml` | `p14` | p | term-before-introduction | error | `link type` is used before it is introduced — link it to `glossary/index.xml#link-type` or gloss it in the same sentence | The link type belongs to the edge, declared by the consumer, never baked into the package. |
| `model/dependency-visibility.xml` | `p35` | p | banned-word | error | `not only` is on this language's list | The `…` table rewrites foreign edges, and it is lawful in any manifest, not only at the ro… |
| `model/dependency-visibility.xml` | `p46` | p | sentence-length | warning | sentence 2 is 37 words, and a p allows 35 | That is how one `…` is both your development set and your contract, split edge by edge rat… |
| `model/dependency-visibility.xml` | `p48` | p | sentence-length | warning | sentence 1 is 36 words, and a p allows 35 | Because a mark in the middle of the graph can widen what reaches you, `…` prints the chang… |
| `model/dependency-visibility.xml` | `p03` | p | term-before-introduction | error | `provider` is used before it is introduced — link it to `glossary/index.xml#provider` or gloss it in the same sentence | The mark is the provider's word about its own dependency: how far it may seep upward, to t… |
| `model/dependency-visibility.xml` | `p38` | p | term-before-introduction | error | `override` is used before it is introduced — link it to `glossary/index.xml#override` or gloss it in the same sentence | Overrides apply along the chains that pass through the manifest declaring them. |
| `model/lock-and-store.xml` | `p05` | p | sentence-length | warning | sentence 2 is 37 words, and a p allows 35 | If a source serves different bytes under a known version, because a tag was force-pushed o… |
| `model/lock-and-store.xml` | `p08` | p | sentence-length | warning | sentence 1 is 40 words, and a p allows 35 | The lock file is kept even when derived state is removed: `…` deletes the dependency tree … |
| `model/lock-and-store.xml` | `p28` | p | terms-per-sentence | warning | 3 glossary terms in one sentence (store, registry, store) — three means the sentence is a container in disguise | The store and the registry clone cache are two different folders: the store holds extracte… |
| `model/packages-and-kinds.xml` | `p01` | p | sentence-length | warning | sentence 2 is 39 words, and a p allows 35 | Packages come in eight kinds, and the kind tells you what a package is for before you open… |
| `model/packages-and-kinds.xml` | `p25` | p | sentence-length | warning | sentence 1 is 37 words, and a p allows 35 | An `…` differs from a `…` mechanically: a tool lives in a project and runs through `…` by … |
| `model/packages-and-kinds.xml` | `p07` | p | term-before-introduction | error | `manifest` is used before it is introduced — link it to `glossary/index.xml#manifest` or gloss it in the same sentence | On the command line the kind prefix and the group are both optional; in a manifest the coo… |
| `model/packages-and-kinds.xml` | `p07` | p | term-before-introduction | error | `registry` is used before it is introduced — link it to `glossary/index.xml#registry` or gloss it in the same sentence | In a registry the repository is named by joining group and name with a dot, `…`, which is … |
| `model/packages-and-kinds.xml` | `p21` | p | term-before-introduction | error | `specification` is used before it is introduced — link it to `glossary/index.xml#specification` or gloss it in the same sentence | The set is closed and grows only by an amendment to the specification; a manifest with an … |
| `model/packages-and-kinds.xml` | `p29` | p | term-before-introduction | error | `boot snippet` is used before it is introduced — link it to `glossary/index.xml#boot-snippet` or gloss it in the same sentence | The bundle itself is the smallest package there is: a manifest and a README, no code and n… |
| `model/packages-and-kinds.xml` | `p31` | p | term-before-introduction | error | `subject` is used before it is introduced — link it to `glossary/index.xml#subject` or gloss it in the same sentence | The manual says which versions of its subject it describes, and the site picks the newest … |
| `model/packages-and-kinds.xml` | `p33` | p | term-before-introduction | error | `store` is used before it is introduced — link it to `glossary/index.xml#store` or gloss it in the same sentence | `…` is the default and all an ordinary package needs; `…` is the same content sharing byte… |
| `model/packages-and-kinds.xml` | `p33` | p | term-before-introduction | error | `git source` is used before it is introduced — link it to `glossary/index.xml#git-source` or gloss it in the same sentence | `…` keeps a live git checkout with its own `…`, ignored by git and restored by a fresh clo… |
| `model/packages-and-kinds.xml` | `p44` | p | terms-per-sentence | warning | 4 glossary terms in one sentence (registry, index, manifest, coordinate) — three means the sentence is a container in disguise | A short name without a group, such as `…`, is accepted on the command line and resolved th… |
| `model/packages-and-kinds.xml` | `p47` | p | terms-per-sentence | warning | 4 glossary terms in one sentence (index, registry, registry, coordinate) — three means the sentence is a container in disguise | Resolving a short name needs the index, one lookup per registry; a registry without one of… |
| `model/registries.xml` | `p39` | p | sentence-length | warning | sentence 1 is 44 words, and a p allows 35 | An override replaces one package with a copy from elsewhere, for a hotfix or a patch waiti… |
| `model/registries.xml` | `p46` | p | sentence-length | warning | sentence 1 is 47 words, and a p allows 35 | A git source is written as an inline table on the requirement, with a `…` address and exac… |
| `model/registries.xml` | `p72` | p | sentence-length | warning | sentence 1 is 40 words, and a p allows 35 | The source repository of vibe itself is mirrored on two hosts, but that is a different thi… |
| `model/registries.xml` | `p07` | p | terms-per-sentence | warning | 4 glossary terms in one sentence (mirror, registry, override, coordinate) — three means the sentence is a container in disguise | The list is an array in priority order; beside it a mirror is a second address for the sam… |
| `model/registries.xml` | `p07` | p | term-before-introduction | error | `mirror` is used before it is introduced — link it to `glossary/index.xml#mirror` or gloss it in the same sentence | The list is an array in priority order; beside it a mirror is a second address for the sam… |
| `model/registries.xml` | `p07` | p | term-before-introduction | error | `override` is used before it is introduced — link it to `glossary/index.xml#override` or gloss it in the same sentence | The list is an array in priority order; beside it a mirror is a second address for the sam… |
| `model/registries.xml` | `p24` | p | terms-per-sentence | warning | 4 glossary terms in one sentence (registry, index, manifest, fingerprint) — three means the sentence is a container in disguise | So a registry may keep an index: a separate repository beside the packages that records, f… |
| `model/registries.xml` | `p26` | p | terms-per-sentence | warning | 3 glossary terms in one sentence (registry, index, index) — three means the sentence is a container in disguise | A registry without an index works exactly as before, only slower; a missing index is not a… |
| `model/registries.xml` | `p29` | p | terms-per-sentence | warning | 6 glossary terms in one sentence (index, registry, registry, registry, index, registry) — three means the sentence is a container in disguise | The index location is derived from the registry's address and can be overridden per regist… |
| `model/registries.xml` | `p37` | p | terms-per-sentence | warning | 4 glossary terms in one sentence (mirror, registry, fingerprint, mirror) — three means the sentence is a container in disguise | A mirror is another address for the same registry, tried first for availability and verifi… |
| `model/registries.xml` | `p37` | p | terms-per-sentence | warning | 3 glossary terms in one sentence (mirror, lock file, mirror) — three means the sentence is a container in disguise | Mirrors never appear in the lock file: the canonical address is what gets recorded, so swi… |
| `model/registries.xml` | `p39` | p | terms-per-sentence | warning | 4 glossary terms in one sentence (override, registry, coordinate, lock file) — three means the sentence is a container in disguise | An override replaces one package with a copy from elsewhere, for a hotfix or a patch waiti… |
| `model/registries.xml` | `p41` | p | terms-per-sentence | warning | 3 glossary terms in one sentence (override, fingerprint, lock file) — three means the sentence is a container in disguise | An override relaxes nothing: the copy's fingerprint is still pinned in the lock file and v… |
| `model/registries.xml` | `p46` | p | term-before-introduction | error | `git source` is used before it is introduced — link it to `glossary/index.xml#git-source` or gloss it in the same sentence | A git source is written as an inline table on the requirement, with a `…` address and exac… |
| `model/registries.xml` | `p51` | p | terms-per-sentence | warning | 3 glossary terms in one sentence (override, git source, registry) — three means the sentence is a container in disguise | The source of a requirement is decided in a fixed order: an override first, then a git sou… |
| `model/registries.xml` | `p56` | p | terms-per-sentence | warning | 4 glossary terms in one sentence (registry, override, git source, embedded registry) — three means the sentence is a container in disguise | Version enumeration still unions the embedded and the declared registries, so a newer publ… |
| `model/registries.xml` | `p72` | p | terms-per-sentence | warning | 3 glossary terms in one sentence (registry, mirror, registry) — three means the sentence is a container in disguise | The source repository of vibe itself is mirrored on two hosts, but that is a different thi… |
| `model/two-trees.xml` | `p08` | p | sentence-length | warning | sentence 1 is 36 words, and a p allows 35 | Three things in a project are derived from the manifest and the lock file, and vibe rebuil… |
| `model/two-trees.xml` | `p13` | p | sentence-length | warning | sentence 1 is 37 words, and a p allows 35 | A package's folder in the dependency tree is named by its group, its name and its version,… |
| `model/two-trees.xml` | `p17` | p | sentence-length | warning | sentence 1 is 36 words, and a p allows 35 | The managed block in an instruction file stays where you put it: vibe rewrites the text be… |
| `model/two-trees.xml` | `p03` | p | term-before-introduction | error | `specification` is used before it is introduced — link it to `glossary/index.xml#specification` or gloss it in the same sentence | Your specifications live in `…`; the packages you depend on are copied, whole and unchange… |
| `model/two-trees.xml` | `p08` | p | terms-per-sentence | warning | 3 glossary terms in one sentence (manifest, lock file, managed block) — three means the sentence is a container in disguise | Three things in a project are derived from the manifest and the lock file, and vibe rebuil… |
| `model/two-trees.xml` | `p08` | p | terms-per-sentence | warning | 3 glossary terms in one sentence (lock file, store, registry) — three means the sentence is a container in disguise | `…` rebuilds all three from the lock file and the machine store, without asking any regist… |
| `model/versions.xml` | `p16` | p | sentence-length | warning | sentence 1 is 37 words, and a p allows 35 | The program manages its own versions with `…`: `…` builds a version from source or install… |
| `model/versions.xml` | `p06` | p | term-before-introduction | error | `family` is used before it is introduced — link it to `glossary/index.xml#family` or gloss it in the same sentence | A family of packages that must move together pins its members exactly, so that a language … |
| `model/versions.xml` | `p21` | p | term-before-introduction | error | `override` is used before it is introduced — link it to `glossary/index.xml#override` or gloss it in the same sentence | `…` repoints the active version; `…` prints a shell line for a one-terminal override inste… |
| `reference/commands.xml` | `p02` | p | sentence-length | warning | sentence 1 is 37 words, and a p allows 35 | Every command takes `…` for machine-readable output, `…` for a one-line summary, `…` to st… |
| `reference/lock-file.xml` | `p08` | p | terms-per-sentence | warning | 4 glossary terms in one sentence (mirror, manifest, lock file, manifest) — three means the sentence is a container in disguise | `…` mirrors the manifest's `…`, so the lock file is a self-contained snapshot that needs n… |
| `reference/lock-file.xml` | `p08` | p | term-before-introduction | error | `mirror` is used before it is introduced — link it to `glossary/index.xml#mirror` or gloss it in the same sentence | `…` mirrors the manifest's `…`, so the lock file is a self-contained snapshot that needs n… |
| `reference/lock-file.xml` | `p08` | p | term-before-introduction | error | `lock file` is used before it is introduced — link it to `glossary/index.xml#lock-file` or gloss it in the same sentence | `…` mirrors the manifest's `…`, so the lock file is a self-contained snapshot that needs n… |
| `reference/lock-file.xml` | `p19` | p | terms-per-sentence | warning | 6 glossary terms in one sentence (lock file, registry, index, mirror, coordinate, fingerprint) — three means the sentence is a container in disguise | The lock file records no registry index and no mirror: reproducing it needs only the coord… |
| `reference/machine-formats.xml` | `p30` | p | sentence-length | warning | sentence 1 is 41 words, and a p allows 35 | A document that no schema describes is a defect, not a feature; the campaign that wrote th… |
| `reference/machine-formats.xml` | `p29` | p | terms-per-sentence | warning | 3 glossary terms in one sentence (index, scrape, registry) — three means the sentence is a container in disguise | The documents of the index server, the scrape contract and plan, the compiler trace and th… |
| `reference/manifest.xml` | `p02` | p | sentence-length | warning | sentence 2 is 37 words, and a p allows 35 | The tables present decide the role: `…` marks a consumer that is never published, `…` a pu… |
| `reference/manifest.xml` | `p30` | p | term-before-introduction | error | `override` is used before it is introduced — link it to `glossary/index.xml#override` or gloss it in the same sentence | `…` overrides the preference for one run. |
| `reference/manifest.xml` | `p37` | p | term-before-introduction | error | `skill` is used before it is introduced — link it to `glossary/index.xml#skill` or gloss it in the same sentence | A skill is a manifest section, never a kind of its own. |
| `reference/manifest.xml` | `p52` | p | terms-per-sentence | warning | 3 glossary terms in one sentence (manifest, subject, skill) — three means the sentence is a container in disguise | The manifest of this manual, generated from the package itself, shows a `…` package with a… |
| `reference/manifest.xml` | `p52` | p | term-before-introduction | error | `subject` is used before it is introduced — link it to `glossary/index.xml#subject` or gloss it in the same sentence | The manifest of this manual, generated from the package itself, shows a `…` package with a… |
| `reference/settings-and-environment.xml` | `p22` | p | term-before-introduction | error | `override` is used before it is introduced — link it to `glossary/index.xml#override` or gloss it in the same sentence | A field shows its provenance on demand, the winning layer and the shadowed ones, and lets … |
| `reference/tree.xml` | `p19` | p | sentence-length | warning | sentence 1 is 42 words, and a p allows 35 | The screen has three display modes, each a configuration of one tree widget and none a fla… |
| `reference/tree.xml` | `p40` | p | sentence-length | warning | sentence 2 is 51 words, and a p allows 35 | The render verb builds the same model `…` builds at `…`, drives a key script at a given si… |
| `reference/tree.xml` | `p49` | p | sentence-length | warning | sentence 1 is 38 words, and a p allows 35 | Underneath, every action of the interface has an address, `…`, and the observable state is… |
| `reference/tree.xml` | `p03` | p | terms-per-sentence | warning | 3 glossary terms in one sentence (lock file, manifest, manifest) — three means the sentence is a container in disguise | The command reads the committed lock file, the manifests and the generated boot files of t… |
| `reference/tree.xml` | `p03` | p | term-before-introduction | error | `manifest` is used before it is introduced — link it to `glossary/index.xml#manifest` or gloss it in the same sentence | The command reads the committed lock file, the manifests and the generated boot files of t… |
| `reference/tree.xml` | `p40` | p | term-before-introduction | error | `family` is used before it is introduced — link it to `glossary/index.xml#family` or gloss it in the same sentence | An agent has no terminal, so the `…` family renders the screen for it. |
| `reference/tree.xml` | `p53` | p | term-before-introduction | error | `override` is used before it is introduced — link it to `glossary/index.xml#override` or gloss it in the same sentence | A package may ship its own locale file, and a language pack may override one. |
| `start/first-project.xml` | `p03` | p | terms-per-sentence | warning | 3 glossary terms in one sentence (manifest, registry, store) — three means the sentence is a container in disguise | vibe reads the manifest's registries, finds the package, resolves a version, fetches it in… |
| `start/install-vibe.xml` | `p05` | p | sentence-length | warning | sentence 2 is 41 words, and a p allows 35 | The one-line cold start is a script at a stable address, `…` for Bash and `…` for PowerShe… |
| `start/install-vibe.xml` | `p24` | p | sentence-length | warning | sentence 1 is 37 words, and a p allows 35 | vibe keeps everything it owns under one folder in your home directory, `…`: the installed … |
| `start/what-a-project-contains.xml` | `p08` | p | term-before-introduction | error | `specification` is used before it is introduced — link it to `glossary/index.xml#specification` or gloss it in the same sentence | `…` is your tree: the specifications and rules this project itself writes, in Markdown or … |
| `start/what-a-project-contains.xml` | `p19` | p | term-before-introduction | error | `anchor` is used before it is introduced — link it to `glossary/index.xml#anchor` or gloss it in the same sentence | Removing a package asks whether to keep or clean its adoption file; `…` removes the files … |
