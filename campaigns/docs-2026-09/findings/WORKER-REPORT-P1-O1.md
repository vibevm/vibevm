# WORKER-REPORT-P1-O1 — импорт вижена как design-документа в XML-диалекте (атом A1.9)

Дата: 2026-09-11. Дерево: `C:\Users\olegc\git\v\vibevm-docs`, HEAD `b1291b06`.

Файлы периметра — оба тронуты, других правок в репозитории нет:

- создан `C:\Users\olegc\git\v\vibevm-docs\vibevm\vibespecs\design\documentation-vision.xml`
  (2698 строк, 303 835 байт, UTF-8 без BOM, LF, 778 фактов);
- правлен `C:\Users\olegc\git\v\vibevm-docs\vibevm\vibespecs\design\README.md`
  — одна строка индекса добавлена в конец списка.

Scratch (промежуточные файлы и скрипты):
`C:\Users\olegc\AppData\Local\Temp\claude\vibe-docs-phase1\P1-O1\`.

Откат к Markdown не понадобился: конвертер принял документ, файл создан как
`.xml`.

## 1. Шаг 1 — подготовка источника

`C:\Users\olegc\git\v\vibe-docs-vision\VISION.md` (2406 строк) скопирован в
scratch как `VISION-orig.md`. Абзац провенанса удалён скриптом
`strip-provenance.py` (ищет строку, начинающуюся с `**Провенанс.**`, и
удаляет её вместе с продолжением до пустой строки включительно).

```
removed lines: 8
lines in: 2407 lines out: 2399
```

Удалены строки 5–12 источника: абзац провенанса (7 строк) и закрывающая
пустая строка. Больше в тексте не изменено ничего — ни одного слова, ни
одной цитаты владельца, ни одного раздела; это подтверждено обратной
проекцией (§6).

## 2. Шаг 2 — конвертация через пивот продукта

Использован готовый бинарник `target/debug/vibe.exe` (не `cargo run`).

**Сухой прогон:**

```
$ ./target/debug/vibe.exe refactor convert-source --from md --to xml --dry-run \
    "C:/Users/olegc/AppData/Local/Temp/claude/vibe-docs-phase1/P1-O1/documentation-vision.md"
dry-run ir-stable-loss C:/Users/olegc/AppData/Local/Temp/claude/vibe-docs-phase1/P1-O1/documentation-vision.md
…
summary converted=0 already=0 lossy-confirmed=0 refused=0 skipped-generated=0 skipped-foreign=0 skipped-harness=0 dry-run=1
EXIT=0
```

**Классификация конвертера — `ir-stable-loss`.** Это не `IrDivergent`:
конвертер называет потери IR-стабильными, то есть IR источника и IR обратной
проекции совпадают, расходятся только байты. Пакет разрешает `--force` ровно в
этом случае, поэтому реальный прогон сделан с `--force`:

```
$ ./target/debug/vibe.exe refactor convert-source --from md --to xml --force \
    "C:/Users/olegc/AppData/Local/Temp/claude/vibe-docs-phase1/P1-O1/documentation-vision.md"
loss C:/Users/olegc/AppData/Local/Temp/claude/vibe-docs-phase1/P1-O1/documentation-vision.md
…
lossy-confirmed C:/Users/olegc/AppData/Local/Temp/claude/vibe-docs-phase1/P1-O1/documentation-vision.md
summary converted=0 already=0 lossy-confirmed=1 refused=0 skipped-generated=0 skipped-foreign=0 skipped-harness=0 dry-run=0
EXIT=0
```

**Полный перечень потерь** (по диффу конвертера, четыре класса, больше ничего):

1. **Разделительная строка таблицы нормализуется:** `|---|---|` →
   `| --- | --- |`. Девять таблиц, чисто косметика.
2. **Перед списком, идущим сразу за абзацем, вставляется пустая строка.**
   Одиннадцать мест (`**Отвергнуто.**` и `Правила:` перед их списками в
   D-01, D-03, D-04, D-12, D-17, D-18 ×2, D-19, D-20 ×2). Markdown-эквивалент,
   смысл не меняется.
3. **Вложенный нумерованный список в D-25 разворачивается в плоский.**
   Единственная потеря структуры. В источнике пункт «**Норма стиля —
   `STYLE.md`** … Её суть в четырёх пунктах.» нёс вложенный список `1.`–`4.`;
   в XML эти четыре пункта стали соседними пунктами того же списка. Порядок
   чтения и весь текст сохранены, введение «Её суть в четырёх пунктах» стоит
   прямо перед ними; утрачены только отступ и нумерация. IR пивота вложенность
   списков не несёт — поэтому конвертер и назвал потерю IR-стабильной. Вручную
   не чинилось: восстановление вложенности было бы правкой текста за пределами
   пакета и сломало бы обратную проекцию.
4. **Лишняя пустая строка в конце файла** при обратной проекции.

## 3. Шаг 3 — разметка фактов

Разметка сделана скриптом `mark-facts.py` (текст скрипта — §10; лежит в
scratch). Скрипт детерминирован: один проход в порядке документа, никакой
ручной правки идентификаторов после него не было.

### Схема идентификаторов

- Префикс — тег **ближайшей** секции (это её якорь `{#id}` из источника);
  вложенность в идентификатор не кодируется.
- У каждой секции **один сквозной счётчик по всем её юнитам** (абзац, пункт
  списка, ячейка тела таблицы, цитата), поэтому `<секция>-<n>` всегда значит
  «n-й юнит этой секции».
- В секции `d-NN` юнит, чей текст открывается меткой записи решения, берёт
  имя метки вместо порядкового номера:

  | Метка в тексте | Идентификатор |
  |---|---|
  | `**Решение.**` | `D-NN-DECISION` |
  | `**Почему.**` | `d-nn-why` |
  | `**Отвергнуто.**` | `d-nn-rejected` |
  | `**Пересмотреть когда.**` | `d-nn-revisit` |
  | `**Известное ограничение.**` | `d-nn-limit` |

- В секции `principles` пункт, открывающийся `**P-NN `, берёт имя `P-NN`.
- Все прочие юниты — `<якорь секции>-<порядковый номер>`.
- Идентификатор, который совпал бы с именем секции или со словом
  зарезервированного словаря, уходит в общую форму `<fact id="…">`. **Таких
  не случилось: 0 обращений к общей форме.**
- Статус у всех фактов — `spec/done`.

### Что получилось

| Вид юнита | Сколько |
|---|---|
| абзацы `<p>` | 154 |
| цитаты `<quote>` | 26 |
| пункты списков | 286 |
| ячейки тела таблиц | 311 |
| **итого юнитов** | **777** |
| `companion-line` (добавленный абзац) | 1 |
| **фактов в файле** | **778** |

Все 778 идентификаторов уникальны. Именованных: 30 × `D-NN-DECISION`,
30 × `-why`, 29 × `-rejected`, 29 × `-revisit`, 2 × `-limit`, 16 × `P-NN`
(P-01…P-16). Значения 29 — это D-17: у него по источнику нет полей
«Отвергнуто» и «Пересмотреть когда». Секции решений — ровно 30, `d-01`…`d-30`.
Цитаты мандата §3 получили `mandate-2`…`mandate-28` (28 юнитов секции — два
абзаца и 26 цитат, один сквозной счётчик).

### Структурные преобразования

- Все 36 списков стали `<facts ordered="…">` (30 неупорядоченных,
  6 упорядоченных), а `<item>` заменён самим фактом-элементом — форма образца
  `host-as-package.xml`; `<list>`/`<item>` в файле не осталось.
- `<p>`, `<td>`, `<quote>` сохранили свой блочный тег и получили факт внутрь.
- Заголовочная строка каждой таблицы (30 ячеек в девяти таблицах) оставлена
  без фактов — она не юнит.
- `<fence>` (4 блока) скопированы дословно, без разметки.

### Шапка документа

`<status>` заменён на предписанный пакетом:

```xml
<status stage="spec" state="done" comment="design rationale behind PROP-057 — the campaign vision, twelve editions 2026-09-09..11; imported 2026-09-11; non-normative, PROP-057 wins where they disagree"/>
```

Сразу за ним — предписанный абзац `companion-line` одной строкой; адрес
`../common/PROP-057-documentation-packages-and-site.xml` проверен, файл
существует.

## 4. Шаг 4 — гейты

Оба зелёные по новому файлу. Чужих красных строк нет вовсе: оба гейта зелёные
по всему дереву.

- `facts check --exhaustive`: единственная строка, называющая новый файл, —
  служебная «(XML source; line numbers are projection-relative)»; ни одной
  ошибки и ни одного предупреждения по нему. Предупреждений в дереве 22 — это
  **те же 22**, что и до импорта (замерены на HEAD до создания файла:
  `progress check: clean (324 files, 22 warning(s))`); новых не появилось.
- `vibe check --path .`: 0 ошибок, 2 предупреждения — оба существовали до
  импорта и к нему не относятся:

```
  [W]  [wal_wellformed] vibevm/vibespecs/WAL.xml — WAL is missing the canonical `## constraints` section
  [W]  [wal_wellformed] vibevm/vibespecs/WAL.xml — WAL is missing the canonical `## done` section
```

## 5. Шаг 5 — строка индекса

В `vibevm/vibespecs/design/README.md` добавлена одна строка — в конец
списка-индекса, после `idx-command-nodes`, по образцу соседей (одна длинная
строка, пустая строка-разделитель сверху, текст пакета дословно):

```
- @fact:idx-documentation-vision [The documentation vision](documentation-vision.xml) — the rationale behind PROP-057: thirty decision records (D-01…D-30) on documentation as packages, the site, the local reader, localization, style and maintenance; written in Russian for the owner; imported 2026-09-11 from the docs-2026-09 campaign; PROP-057 wins where they disagree. @status:spec/done
```

Место: пакет говорит «в конце файла, рядом с `idx-tooling-map`». Строка
`idx-tooling-map` стоит в середине того же списка, а сам список кончается
файлом; выбран конец списка — так индекс живёт («living index — every new
design doc adds a row») и так лежат все записи, добавленные после него.

## 6. Проверка целостности текста

Сверх пакета сделана обратная проверка: готовый размеченный XML спроецирован
обратно в Markdown тем же пивотом (`convert-source --from xml --to md`,
классификация `converted` — без потерь), из проекции сняты маркеры
`@fact:`/`@status:` (`normalize-projection.py`) и результат сдиффен с
`VISION-stripped.md`.

**Весь дифф — ровно четыре класса потерь конвертера из §2 плюс две
предписанные пакетом правки шапки.** Ни одного изменённого слова, ни одной
тронутой цитаты, ни одного пропавшего раздела:

```
44 diff-строк: замена <status/>, добавленный абзац **Explains:**,
9 разделительных строк таблиц, 11 пустых строк перед списками,
4 строки плоского списка D-25, 1 пустая строка в конце файла.
```

## 7. Грепп-гейт по импорту (§8.2 вижена, строка «Детали инфраструктуры утекают»)

Вижен сам предписывает грепп-гейт при импорте в репозиторий. Прогнан:

- **IPv4:** найдены `127.0.0.1` (×3) и `0.0.0.0` (×1) — это решение D-09
  («только на 127.0.0.1») и отвергнутый им вариант, а не адреса сервера.
- **VPN:** 9 вхождений, **все — само стоп-правило** («содержимое не
  переносится: адреса, порты, схема трафика, VPN», «атом не касается … VPN»).
  Ни одного имени VPN-компонента.
- **Порты:** одно вхождение, `:80` в §2.5 — «TLS снаружи контейнера, контейнер
  слушает `:80`». Это стандартный порт внутри контейнера, а не порт сервера и
  не схема трафика; текст владельца, шаг 1 запрещает его менять. **Отдаётся на
  решение боссу** — если по R-25 это всё же считается портом, строка правится
  отдельным решением, а не воркером.
- **Путей на сервере нет**; `C:\Users\olegc\git\infra\main\main.md` упоминается
  как ссылка на приватный документ — ровно та форма, которую предписывает сам
  D-23. Файл `main.md` не открывался (R-25 соблюдён). Секретов не встречено.

**«Ни одного упоминания инструмента-соавтора»** — критерий приёмки выполнен:
утверждение о соавторстве жило только в удалённом абзаце провенанса. Шесть
оставшихся вхождений слова «Claude» перечислены полностью, ни одно не говорит
об авторстве этого документа:

1. §2.1 — «`vibe skill install` проецирует скиллы в Claude Code, OpenCode и
   Codex» (факт о продукте);
2. §2.5 — имя файла `CLAUDE.md` в перечне файлов репозитория oleg.guru;
3–5. §3 — дословная цитата владельца о клаудизмах и балансе сложности
   (три вхождения в одной цитате);
6. D-13 — «что уже случалось с фетчером Claude на oleg.guru» (наблюдение о
   веб-фетчере).

Имена моделей `Fable` (14), `Opus` (9), `Astra` (2), `Sol` (2), `Codex` (2)
стоят в цитатах владельца §3 и в решениях D-25 и D-29 — это его распоряжения о
разделении труда в кампании, а не атрибуция документа; шаг 1 пакета прямо
запрещает их трогать.

## 8. Самопроверка (дословно)

```
$ ./target/debug/vibe.exe facts check --exhaustive; echo "EXIT=$?"
… (строки, называющие новый файл — одна, служебная)
vibevm/vibespecs/design/documentation-vision.xml: (XML source; line numbers are projection-relative)
… (итоговая строка)
progress check: clean (325 files, 22 warning(s))
EXIT=0
```

```
$ ./target/debug/vibe.exe check --path . --quiet; echo "EXIT=$?"
vibe check: 0 errors, 2 warnings, 0 info
EXIT=0
```

```
$ python -c "import xml.etree.ElementTree as ET; ET.parse('vibevm/vibespecs/design/documentation-vision.xml'); print('well-formed')"; echo "EXIT=$?"
well-formed
EXIT=0
```

```
$ grep -c 'fact="true"' vibevm/vibespecs/design/documentation-vision.xml
778
$ ./target/debug/vibe.exe facts check --exhaustive 2>&1 | grep -c "documentation-vision.xml:[0-9]*: Error \[unmarked\]"
0
```

Число фактов в файле — **778**. Число юнитов без факта — **0** (grep возвращает
код 1 именно потому, что ни одной строки `Error [unmarked]` по этому файлу нет;
это и есть искомый ноль). Для сравнения: тот же гейт по неразмеченному
черновику давал 777 ошибок — 311 `Cell`, 286 `Item`, 180 `Para`; 180 `Para`
= 154 абзаца + 26 цитат, то есть цитата считается юнитом и размечена.

## 9. Решения, допущения, отклонения

1. **`--force` применён**, потому что конвертер назвал потери IR-стабильными;
   все четыре класса перечислены в §2. Отклонений от пакета здесь нет — это
   ровно предусмотренная им ветка.
2. **Счётчик порядковых номеров — сквозной по всем юнитам секции**, а не
   только по безымянным. Пакет допускает оба чтения («остальные юниты секции →
   `d-nn-<порядковый номер>`»). Выбрано сквозное, потому что тогда
   `<секция>-<n>` значит одно и то же во всех секциях документа — «n-й юнит
   секции» — и правило одно, а не два. Побочное следствие: в секциях решений
   номера идут с пропусками (именованные юниты тоже расходуют номер).
3. **Списки переведены в `<facts>`**, а не размечены внутри `<item>`: это
   форма образца `host-as-package.xml`, на который ссылается пакет, и
   нормализованная форма «все пункты — факты» по PROP-045.
4. **Вложенность D-25 не восстанавливалась вручную** (см. §2, потеря 3).
5. **Разметка сделана скриптом**, как требует пакет; после скрипта файл не
   правился руками ни в одном месте.
6. **Текст `<status>` и `companion-line` взяты из пакета дословно**, с
   переносами, склеенными в одну строку (в XML это один абзац; так же лежит
   `companion-line` в `host-as-package.xml`).
7. Файлы вне периметра, изменённые параллельной центральной сессией
   (`campaigns/docs-2026-09/JOURNAL.md`, `specmap.json`,
   `campaigns/docs-2026-09/findings/PACKET-P1-O2.md`), не трогались.
   Отдельно проверено, что их меняю не я: `specmap.json` не переписывается ни
   `vibe check`, ни `facts check` — mtime после обоих прогонов не двигается.
   Git не трогался вовсе, кроме читающих `git status` / `git diff` / `git
   rev-parse`.

## 10. Дефекты пакета

1. **Находка не определена.** Преамбула пакета говорит «отчёт и находка
   пишутся в `campaigns/docs-2026-09/findings/`», но разделы «Сделать» и
   «Отчёт» не называют ни имени файла находки, ни её содержания, а общий
   преамбул описывает форму находки только для спайков фазы 0. Взято
   консервативное допущение: у этого пакета один артефакт-отчёт, находка не
   пишется.
2. **Критерий приёмки «ни одного упоминания инструмента-соавтора» и шаг 1
   «цитаты владельца остаются» пересекаются.** Разведено в §7: удалено
   утверждение о соавторстве; имена моделей внутри цитат владельца и решений
   D-25/D-29 оставлены, полный перечень приведён, чтобы боссу не пришлось
   грепать самому.
3. **Место строки индекса задано двусмысленно** («в конце файла, рядом с
   `idx-tooling-map`» — это разные места). Выбран конец списка, обоснование в
   §5.

## 11. Что не сделано

Ничего из предписанного не пропущено. Коммитов нет — по пакету коммитит
центральная сессия; рабочее дерево оставлено с двумя изменёнными файлами
периметра.

## 12. Приложение: скрипты

Все три лежат в `C:\Users\olegc\AppData\Local\Temp\claude\vibe-docs-phase1\P1-O1\`.
`strip-provenance.py` (шаг 1) и `normalize-projection.py` (проверка §6) —
по десятку строк каждый и описаны выше. Ниже — `mark-facts.py`, скрипт
разметки, целиком.

```python
"""Mark every unit of the converted vision XML with a fact.

Input : documentation-vision.xml (the pivot's XML output, unmarked)
Output: documentation-vision.marked.xml

Identifier scheme (deterministic, one pass in document order):

  * the nearest enclosing section's tag is the prefix; nesting is not encoded;
  * every section carries one running counter over ALL its units (paragraph,
    list item, table body cell, quote), so `<section>-<n>` always means
    "the n-th unit of that section";
  * inside a `d-NN` section a unit whose text opens with a decision-record
    label takes the label's name instead of the ordinal:
        `**Решение.**`              -> D-NN-DECISION
        `**Почему.**`               -> d-nn-why
        `**Отвергнуто.**`           -> d-nn-rejected
        `**Пересмотреть когда.**`   -> d-nn-revisit
        `**Известное ограничение.**`-> d-nn-limit
  * inside `principles` an item opening with `**P-NN ` takes the name P-NN;
  * every other unit takes `<section>-<ordinal>`;
  * an identifier that would collide with a section name or with the reserved
    block vocabulary falls back to the generic `<fact id="...">` form.

Structural transforms: `<list>` becomes `<facts>` (all children are facts),
`<item>` becomes the fact element itself, `<p>` / `<td>` / `<quote>` keep their
block tag and gain a fact child. Fences are copied verbatim.
"""
import io
import re
import sys

SRC = r"C:\Users\olegc\AppData\Local\Temp\claude\vibe-docs-phase1\P1-O1\documentation-vision.xml"
DST = r"C:\Users\olegc\AppData\Local\Temp\claude\vibe-docs-phase1\P1-O1\documentation-vision.marked.xml"

STATUS = "spec/done"

DOC_STATUS = (
    '<status stage="spec" state="done" comment="design rationale behind '
    "PROP-057 \u2014 the campaign vision, twelve editions 2026-09-09..11; "
    "imported 2026-09-11; non-normative, PROP-057 wins where they disagree"
    '"/>'
)

COMPANION = (
    '<p><companion-line fact="true" status="spec/done">**Explains:** '
    "[PROP-057](../common/PROP-057-documentation-packages-and-site.xml) \u2014 "
    "the documentation packages and site contract; the thirty decision records "
    "D-01\u2026D-30 below are its rationale, and PROP-057 wins where they "
    "disagree (the spec-genres precedence law). Written in Russian for the "
    "owner; the norm it explains is English.</companion-line></p>"
)

RESERVED = {
    "spec",
    "title",
    "status",
    "section",
    "p",
    "fact",
    "list",
    "item",
    "table",
    "tr",
    "td",
    "fence",
    "quote",
    "facts",
}

LABELS = [
    ("**\u0420\u0435\u0448\u0435\u043d\u0438\u0435.**", "DECISION"),
    ("**\u041f\u043e\u0447\u0435\u043c\u0443.**", "why"),
    ("**\u041e\u0442\u0432\u0435\u0440\u0433\u043d\u0443\u0442\u043e.**", "rejected"),
    (
        "**\u041f\u0435\u0440\u0435\u0441\u043c\u043e\u0442\u0440\u0435\u0442\u044c "
        "\u043a\u043e\u0433\u0434\u0430.**",
        "revisit",
    ),
    (
        "**\u0418\u0437\u0432\u0435\u0441\u0442\u043d\u043e\u0435 "
        "\u043e\u0433\u0440\u0430\u043d\u0438\u0447\u0435\u043d\u0438\u0435.**",
        "limit",
    ),
]

SECTION_OPEN = re.compile(r'^(\s*)<([A-Za-z][\w.\-]*)\s+title="')
SECTION_CLOSE = re.compile(r"^\s*</([A-Za-z][\w.\-]*)>\s*$")
D_SECTION = re.compile(r"^d-\d{2}$")
PRINCIPLE = re.compile(r"^\*\*P-(\d{2})\b")


def read_lines(path):
    with io.open(path, "r", encoding="utf-8", newline="") as fh:
        text = fh.read()
    if "\r" in text:
        sys.exit("source carries CR")
    return text.split("\n")


def collect_section_names(lines):
    names = set()
    for line in lines:
        m = SECTION_OPEN.match(line)
        if m:
            names.add(m.group(2))
    return names


class Marker:
    def __init__(self, section_names):
        self.section_names = section_names
        self.counters = {}
        self.used = set()
        self.stats = {"p": 0, "item": 0, "td": 0, "quote": 0}

    def mint(self, section, text):
        if section is None:
            sys.exit("unit outside any section: " + text[:60])
        self.counters[section] = self.counters.get(section, 0) + 1
        ordinal = self.counters[section]
        name = None
        if D_SECTION.match(section):
            for label, suffix in LABELS:
                if text.startswith(label):
                    name = (
                        "%s-DECISION" % section.upper()
                        if suffix == "DECISION"
                        else "%s-%s" % (section, suffix)
                    )
                    break
        elif section == "principles":
            m = PRINCIPLE.match(text)
            if m:
                name = "P-%s" % m.group(1)
        if name is None:
            name = "%s-%d" % (section, ordinal)
        if name in self.used:
            sys.exit("duplicate identifier: " + name)
        self.used.add(name)
        generic = name in RESERVED or name in self.section_names
        return name, generic

    def open_tag(self, name, generic):
        if generic:
            return '<fact id="%s" fact="true" status="%s">' % (name, STATUS)
        return '<%s fact="true" status="%s">' % (name, STATUS)

    def close_tag(self, name, generic):
        return "</fact>" if generic else "</%s>" % name


def transform(lines):
    section_names = collect_section_names(lines)
    marker = Marker(section_names)
    out = []
    stack = []
    i = 0
    total = len(lines)
    status_seen = False
    while i < total:
        line = lines[i]
        stripped = line.strip()
        indent = line[: len(line) - len(line.lstrip())]

        # document status + companion line
        if not status_seen and stripped.startswith("<status "):
            out.append(indent + DOC_STATUS)
            out.append(indent + COMPANION)
            status_seen = True
            i += 1
            continue

        # fences are verbatim
        if stripped.startswith("<fence"):
            out.append(line)
            if not stripped.endswith("</fence>"):
                i += 1
                while i < total:
                    out.append(lines[i])
                    if lines[i].rstrip().endswith("</fence>"):
                        break
                    i += 1
            i += 1
            continue

        # lists become fact groups
        if stripped.startswith("<list"):
            out.append(line.replace("<list", "<facts", 1))
            i += 1
            continue
        if stripped == "</list>":
            out.append(line.replace("</list>", "</facts>", 1))
            i += 1
            continue

        # table rows: the first row of each table is the header
        if stripped == "<table>":
            out.append(line)
            header_pending = True
            i += 1
            while i < total:
                inner = lines[i]
                inner_stripped = inner.strip()
                if inner_stripped == "</table>":
                    out.append(inner)
                    break
                if inner_stripped == "<tr>":
                    out.append(inner)
                    i += 1
                    while i < total and lines[i].strip() != "</tr>":
                        cell = lines[i]
                        cell_stripped = cell.strip()
                        if header_pending:
                            out.append(cell)
                        else:
                            if not (
                                cell_stripped.startswith("<td>")
                                and cell_stripped.endswith("</td>")
                            ):
                                sys.exit("unexpected cell shape: " + cell_stripped[:60])
                            body = cell_stripped[len("<td>") : -len("</td>")]
                            name, generic = marker.mint(stack[-1] if stack else None, body)
                            marker.stats["td"] += 1
                            cell_indent = cell[: len(cell) - len(cell.lstrip())]
                            out.append(
                                "%s<td>%s%s%s</td>"
                                % (
                                    cell_indent,
                                    marker.open_tag(name, generic),
                                    body,
                                    marker.close_tag(name, generic),
                                )
                            )
                        i += 1
                    out.append(lines[i])  # </tr>
                    header_pending = False
                    i += 1
                    continue
                sys.exit("unexpected line in table: " + inner_stripped[:60])
            i += 1
            continue

        # wrapped blocks: paragraph, quote (tag kept, fact child added)
        for tag in ("p", "quote"):
            opening = "<%s>" % tag
            closing = "</%s>" % tag
            if stripped.startswith(opening):
                body_first = stripped[len(opening) :]
                single = body_first.endswith(closing)
                if single:
                    body_first = body_first[: -len(closing)]
                name, generic = marker.mint(stack[-1] if stack else None, body_first)
                marker.stats[tag] += 1
                head = "%s%s%s%s" % (
                    indent,
                    opening,
                    marker.open_tag(name, generic),
                    body_first,
                )
                if single:
                    out.append(head + marker.close_tag(name, generic) + closing)
                    i += 1
                else:
                    out.append(head)
                    i += 1
                    while i < total:
                        cur = lines[i]
                        if cur.rstrip().endswith(closing):
                            trimmed = cur.rstrip()
                            out.append(
                                trimmed[: -len(closing)]
                                + marker.close_tag(name, generic)
                                + closing
                            )
                            break
                        out.append(cur)
                        i += 1
                    i += 1
                break
        else:
            # list items: the fact element replaces <item>
            if stripped.startswith("<item>"):
                body_first = stripped[len("<item>") :]
                single = body_first.endswith("</item>")
                if single:
                    body_first = body_first[: -len("</item>")]
                name, generic = marker.mint(stack[-1] if stack else None, body_first)
                marker.stats["item"] += 1
                head = indent + marker.open_tag(name, generic) + body_first
                if single:
                    out.append(head + marker.close_tag(name, generic))
                    i += 1
                else:
                    out.append(head)
                    i += 1
                    while i < total:
                        cur = lines[i]
                        if cur.rstrip().endswith("</item>"):
                            trimmed = cur.rstrip()
                            out.append(
                                trimmed[: -len("</item>")]
                                + marker.close_tag(name, generic)
                            )
                            break
                        out.append(cur)
                        i += 1
                    i += 1
                continue

            m = SECTION_OPEN.match(line)
            if m:
                stack.append(m.group(2))
                out.append(line)
                i += 1
                continue
            m = SECTION_CLOSE.match(line)
            if m and stack and m.group(1) == stack[-1]:
                stack.pop()
                out.append(line)
                i += 1
                continue
            out.append(line)
            i += 1
            continue
    if stack:
        sys.exit("unclosed sections: " + ",".join(stack))
    if not status_seen:
        sys.exit("document status line not found")
    return out, marker


def main():
    lines = read_lines(SRC)
    out, marker = transform(lines)
    with io.open(DST, "w", encoding="utf-8", newline="") as fh:
        fh.write("\n".join(out))
    print("units marked:", marker.stats)
    print("total facts:", sum(marker.stats.values()) + 1)  # + companion-line
    print("sections:", len(marker.counters))


if __name__ == "__main__":
    main()
```

Его вывод на этом документе:

```
units marked: {'p': 154, 'item': 286, 'td': 311, 'quote': 26}
total facts: 778
sections: 51
```
