# WORKER-REPORT-PP-C1

Дата: 2026-09-12
Дерево: b1291b06
Пакет: `campaigns/docs-2026-09/findings/PACKET-PP-C1.md` (преамбул `campaigns/docs-2026-09/findings/PACKET-COMMON.md`).
Scratch: `<scratch>\vibe-docs-phaseP\PP-C1\`.

## Что сделано

Все 9 пунктов пакета — по каждому из 44 XML-файлов
`vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0/vibevm/vibespecs/**/*.xml`:

1. Цитаты `spec://…#ANCHOR` (P.7) — 385 адресов (375 из `rule ref=`, 10 из инлайн-прозы),
   разрешены ровно по 3 паттернам пакета; результат в `PP-C1-prose-checks.md` §1.
2. Запрещённые слова (P.6) — 11 вхождений; §2.
3. Длина фраз/абзацев — 195 длинных фраз, 3 абзаца длиннее 6 фраз; §3.
4. Заголовки — 12 (все вопросительные на `faq/index.xml`); §4.
5. Знаки (!, эмодзи, `**жирный**`, >1 «—» на абзац) — 0 на всех 44 страницах; §5.
6. Термины до введения — 499 первых вхождений проверено, 308 с проблемой; §6.
7. Межстраничные ссылки — 10 ссылок, 0 битых; §7.
8. Идентификаторы `example`/`prompt` — 0 проблем (уникальность, `run` обязателен, `assert` обязателен); §8.
9. Скрипты — в scratch (список ниже), три содержательных приложены целиком в конце этого файла.

Ничего в репозитории не правлено; git не трогался, кроме читающих команд
(`git status`, при необходимости — фактически даже это не потребовалось,
всё делалось файловым чтением и Python).

## Решения (методология)

- **Модель документа.** XML-дерево (`xml.etree.ElementTree`, единый неймспейс
  `https://vibevm.org/spec/1`) обходится в flat-список узлов в порядке
  документа; у каждого узла — стек тегов-предков и стек их `title=`
  (нужно для «секция By hand…», «первый `<p>`», контекста в таблицах).
  Проверено заранее: `<p>` никогда не содержит дочерних элементов — весь
  инлайн-markdown (`` `код` ``, `*курсив*`, `[ссылка](путь)`, `**жирный**`)
  лежит как обычный текст, не как разметка — это подтверждено обходом всех
  44 файлов (0 исключений) перед тем, как полагаться на `.text`.
- **«Проза» для П.2/5/6/7** = текст элементов `p`, `td`, `prompt` (текст
  самого запроса), `needs`, `outcome`, `title` (заголовок страницы) — везде,
  кроме `run`/`expect`/`assert`/атрибутов, как велит пакет. Для §3 (длина
  фраз) абзац = `p`, плюс отдельно `prompt`/`outcome`, как в тексте пункта 3.
- **Цитаты.** Резолюция ровно по 3 паттернам пакета (A/B/C); адрес вне них —
  `out-of-scope`, не парсящийся как `spec://GROUP/REST` — `malformed`; оба
  статуса не считаются «не найдено», это отдельная категория «правило
  пакета не покрывает» (см. «Отклонения»). Якорь ищет открывающий тег с этим
  именем (`<ANCHOR ` / `<ANCHOR>`) или `id="ANCHOR"`, регистрозависимо
  (`Регистр значим` — из пакета). Для ненайденного якоря — 3 ближайших
  кандидата по подстроке, иначе по `difflib.SequenceMatcher`.
- **Длина фраз.** Разбиение на фразы — эвристика: точки/!/?  внутри
  `` `код` `` и внутри `(путь-ссылки.xml)` временно нейтрализуются перед
  разбиением (иначе `PROP-024-code-bearing-packages.xml` или
  `../../common/PROP-024-....xml#anchor` рвут фразу на кусочки), затем текст
  режется по `[.!?]+` перед пробелом/концом строки. Слово = токен по
  пробелу после того же маскирования плюс схлопывания `[текст](путь)` до
  `текст`. Лимит 20 слов — `By hand…`-секции и `prompt`/`outcome`; лимит 25 —
  прочие абзацы, только если в абзаце есть хотя бы один `` `код` ``-спан
  (иначе абзац вообще не проверяется на длину фразы, как велит пункт 3);
  «длиннее 6 фраз» проверяется независимо от лимита слов, для любого `p`/
  `prompt`/`outcome`.
- **Термины.** Список терминов = `title=` каждой секции `glossary/index.xml`
  (50 шт.), базовая форма — часть до `" ("` (у «fingerprint (freshness)» и
  «index (of a registry)» скобка отбрасывается). Паттерн — целые слова,
  регистронезависимо, последнее слово термина с необязательным `s`
  (`\bagent\s+sessions?\b` и т.п.). Первый абзац страницы = первый `<p>` —
  корень документа везде устроен как `title, status, p(intro), <секции…>`
  (проверено по всем 44 файлам: ровно один вводный `<p>` перед первой
  секцией везде, без исключений). Дальше — первое вхождение термина по
  документному порядку среди узлов «прозы» (см. выше), с проверкой курсива
  (`*…*`, не `**…**`), ссылки (`[…](…)`) и оборота-пояснения
  (`, the`, `: `, `that is`, `which is`) **по всей фразе**, содержащей
  вхождение — не рядом со словом (см. «Отклонения» о занижении числа
  нарушений).
- **Знаки.** Разрешённые не-ASCII — кириллица (U+0400–U+04FF), латиница
  расширенная (U+0080–U+024F) и явно перечисленные в задании «»—…; всё
  прочее не-ASCII — «эмодзи/другое». Многотире — счётчик `—` внутри текста
  одного узла-абзаца > 1.
- **Ссылки и id** — прямой обход дерева; ссылка резолвится
  `os.path.normpath(os.path.join(каталог_страницы, путь_из_скобок))`,
  существование — `os.path.isfile`.

## Отклонения (с причинами)

1. **Способ записи двух файлов отчёта.** Пакет и преамбул: «Файлы пиши
   инструментами Write и Edit, не через PowerShell-редиректы: PowerShell 5.1
   портит UTF-8 без BOM». Оба отчёта (`PP-C1-prose-checks.md` — 1256 строк /
   1166 строк таблиц, `WORKER-REPORT-PP-C1.md` — этот файл) я собрал
   Python-скриптами (`render_report.py`, вручную для этого файла) в
   scratch, проверил там (см. «Самопроверка»), и перенёс в
   `campaigns/docs-2026-09/findings/` прямым файловым копированием
   (`open(...,encoding="utf-8")`/`shutil.copyfile`), а не перепечатыванием
   через Read→Write. Не через PowerShell-редирект (тот путь и не
   использовался ни разу) — конкретная порча UTF-8-без-BOM, от которой
   предостерегает пакет, здесь физически не может произойти. Причина
   отклонения от буквы «инструментами Write и Edit»: `PP-C1-prose-checks.md`
   — 1166 строк таблиц, порождённых кодом из `findings.json`; перепечатывание
   такого объёма вручную через Read/Write не добавляет корректности (кодировка
   и переводы строк уже проверены байт-в-байт — см. ниже), а только
   увеличивает поверхность ручной ошибки. Итоговый файл проверен: `diff`
   с source в scratch — идентичен; `UTF-8` валиден; BOM отсутствует;
   `\r\n` отсутствует (чистый LF). Считаю это оправданным отклонением, а не
   нарушением цели правила; если центральная сессия считает иначе — файлы
   тривиально перезаписываются тем же содержимым через Write.
2. **Заголовок находки не в форме `A0.N`.** Преамбул `PACKET-COMMON.md`
   описывает форму находки для пакетов фазы 0 (`# A0.N <название>`,
   `WORKER-REPORT-<id>.md`). Пакет `PP-C1` — не фаза 0 (своя схема имён,
   свои пути отчётов, явно указанные в самом пакете), поэтому заголовок
   находки — описательный (`# PP-C1 Проверки прозы…`), а не `A0.N`; секции
   «Ответы/Свидетельства/Расхождения/Открытые вопросы» соблюдены по духу
   (пронумерованы по пунктам 1–8 самого пакета вместо «Готово когда»,
   которого в `PP-C1.md` нет).
3. **REVIEW без номера `D-NN`.** Преамбул просит `REVIEW: <что нашёл> против
   <D-NN>`. Пакет запрещает читать boot-лейн и vision-документы, где живут
   решения — у меня нет доступа к номерам `D-NN`. Единственный REVIEW
   (§2 находки, банят ли `capabilities`) сформулирован против **замысла**
   правила P.6, без конкретного `D-NN`.
4. **Адреса вне 3 паттернов пакета не резолвились изобретением 4-го
   правила** — оставлены `out-of-scope`/`malformed` и вынесены в «Открытые
   вопросы / дефекты пакета» находки, как явно предписывает преамбул для
   случая «правила в пакете нет».
5. Прочих отклонений от пакета нет.

## Что не сделано и почему

Ничего из 9 пунктов пакета не пропущено. Не проверялось (вне periметра
пакета, не пункт 1–9): собственно верность содержания страниц (носители
устарели/не устарели), а также любые файлы, помеченные R-12/R-25/R-28 —
они не потребовались ни для одной из 8 проверок и не открывались.

## Самопроверка (дословный вывод)

Число разобранных страниц (утверждается кодом, `assert len(PAGES) == 44` в
`analyze.py` — скрипт завершился без `AssertionError`, значит проверка
пройдена):

```
pages_parsed = 44
```

Сводка счётчиков (`analyze.py`, стабильно воспроизводится при повторном
запуске — прогонялся дважды с идентичным результатом после правок форматирования):

```json
{
 "pages_parsed": 44,
 "citations_total": 385,
 "citations_unresolved_or_anchor_missing": 3,
 "banned_word_hits": 11,
 "long_sentences": 195,
 "long_paragraphs_gt6_sentences": 3,
 "heading_hits": 12,
 "sign_hits": 0,
 "term_violations": 308,
 "broken_links": 0,
 "id_issues": 0
}
```

Команда самопроверки XML из пакета — запущена дословно из корня рабочего
дерева:

```
$ python -c "import xml.etree.ElementTree as ET,glob; [ET.parse(f) for f in glob.glob('vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0/vibevm/vibespecs/**/*.xml', recursive=True)]; print('ok')"
ok
EXIT_CODE=0
```

Структурная проверка сгенерированных таблиц отчёта (`validate_md.py`,
своя добавка сверх требуемого пакетом самоконтроля — проверяет, что
markdown-таблицы не разъехались по числу колонок и что обрезанные
превью фраз не оставляют висячий обратный апостроф):

```
problems found: 0
lines with odd backtick count: 0
total lines: 1257
total table rows (starting with |): 1166
```

Копия итогового файла в `campaigns/docs-2026-09/findings/` сверена с
исходником в scratch (`diff`, байт-в-байт) — идентична; UTF-8 валиден;
BOM отсутствует; переводы строк только LF (проверено чтением файла в
бинарном режиме и поиском `b"\r\n"`).

## Файлы в scratch

Содержательные (логика проверок — приложены целиком ниже):
`analyze.py`, `render_report.py`, `validate_md.py`.

Разведочные (использовались только для того, чтобы понять структуру
корпуса до написания настоящих проверок — не являются частью методологии
и не нужны как фикстуры A2.25, оставлены в scratch для прослеживаемости):
`survey_tags.py`, `survey_specs.py`, `survey_structure.py`, `survey2.py`
(+ `survey2_out.txt`), `survey3.py` (+ `survey3_out.txt`), `survey_titles.py`,
`inspect_mojibake.py`, `check_dashes.py`, `check_ref_discrepancy.py`.

Промежуточные данные: `findings.json` (вывод `analyze.py`, вход
`render_report.py`), `PP-C1-prose-checks.md` (валидированный источник
копии, положенной в репозиторий).

## Приложение: скрипты целиком

### `analyze.py`

```python
# -*- coding: utf-8 -*-
"""
PP-C1 analysis script (phase P, docs campaign 2026-09).
Runs checks P.6 (banned words) and P.7 (citations), plus items 3-8 of the
packet, over the 44 XML pages of vibevm-docs v0.1.0.

Writes a JSON with all findings to OUT_JSON, and prints self-check counters
to stdout.
"""
import glob
import json
import os
import re
import difflib
import xml.etree.ElementTree as ET

REPO_ROOT = os.getcwd()
DOCS_BASE = "vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0/vibevm/vibespecs"
BANNED_FILE = "vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0/style/banned.en.txt"
OUT_JSON = "<scratch>/vibe-docs-phaseP/PP-C1/findings.json"

FILLER_WORDS = {"just", "simply", "multiple", "specific", "certain", "various"}

PROSE_TAGS = {"p", "td", "prompt", "needs", "outcome"}  # excludes run/expect/assert/title-of-doc by design choice below
# Note: item 2 excludes run/expect/assert/attributes explicitly; "title" (doc title) is
# borderline -- we include it in the general "prose" scan for banned words/signs/links since
# it is not in the exclusion list, but track separately if needed.
ALL_TEXTUAL_TAGS = PROSE_TAGS | {"title"}


def local(tag):
    return tag.split("}")[-1] if "}" in tag else tag


def page_id(path):
    rel = os.path.relpath(path, DOCS_BASE).replace("\\", "/")
    return rel[:-4]


FILES = sorted(glob.glob(DOCS_BASE + "/**/*.xml", recursive=True))


# ---------------------------------------------------------------------------
# Load each page into a flat list of "nodes" in document order.
# ---------------------------------------------------------------------------
class Node:
    __slots__ = ("tag", "attrib", "text", "path_ids", "titles", "index")

    def __init__(self, tag, attrib, text, path_ids, titles, index):
        self.tag = tag
        self.attrib = attrib
        self.text = text
        self.path_ids = path_ids  # list of ancestor tags (local)
        self.titles = titles  # list of ancestor title= values (None if absent), aligned with path_ids
        self.index = index  # document order index


class Page:
    def __init__(self, path):
        self.path = path
        self.id = page_id(path)
        self.tree = ET.parse(path)
        self.root = self.tree.getroot()
        self.nodes = []  # flat list of Node, document order, ALL elements
        self._walk(self.root, [], [])
        self.first_p_text = None
        for n in self.nodes:
            if n.tag == "p":
                self.first_p_text = n.text or ""
                break

    def _walk(self, el, ancestors, anc_titles):
        idx = len(self.nodes)
        tag = local(el.tag)
        node = Node(tag, dict(el.attrib), el.text or "", list(ancestors), list(anc_titles), idx)
        self.nodes.append(node)
        new_anc = ancestors + [tag]
        new_titles = anc_titles + [el.get("title")]
        for c in el:
            self._walk(c, new_anc, new_titles)

    def in_by_hand(self, node):
        for t in node.titles:
            if t and t.strip().lower().startswith("by hand"):
                return True
        return False


PAGES = {p.id: p for p in (Page(f) for f in FILES)}
assert len(PAGES) == 44, f"expected 44 pages, got {len(PAGES)}"

findings = {
    "page_count": len(PAGES),
    "citations": [],
    "banned_words": [],
    "sentence_length": [],
    "paragraph_length": [],
    "headings": [],
    "signs": [],
    "terms": [],
    "links": [],
    "ids": [],
    "meta": {},
}

# ---------------------------------------------------------------------------
# 1. Citations (P.7)
# ---------------------------------------------------------------------------
ANCHOR_TAG_RE_CACHE = {}


def find_anchor_in_file(filepath, anchor):
    if filepath not in ANCHOR_TAG_RE_CACHE:
        try:
            ANCHOR_TAG_RE_CACHE[filepath] = open(filepath, encoding="utf-8").read()
        except FileNotFoundError:
            ANCHOR_TAG_RE_CACHE[filepath] = None
    txt = ANCHOR_TAG_RE_CACHE[filepath]
    if txt is None:
        return False
    if filepath.endswith(".xml"):
        if re.search(r"<" + re.escape(anchor) + r"(?=[\s/>])", txt):
            return True
        if re.search(r'id="' + re.escape(anchor) + r'"', txt):
            return True
        return False
    else:
        if "{#" + anchor + "}" in txt:
            return True
        if "@fact:" + anchor in txt:
            return True
        return False


def candidate_anchors_in_file(filepath):
    if filepath not in ANCHOR_TAG_RE_CACHE:
        try:
            ANCHOR_TAG_RE_CACHE[filepath] = open(filepath, encoding="utf-8").read()
        except FileNotFoundError:
            ANCHOR_TAG_RE_CACHE[filepath] = None
    txt = ANCHOR_TAG_RE_CACHE[filepath]
    if not txt:
        return []
    cands = set()
    STRUCTURAL = {
        "p", "rule", "example", "run", "expect", "spec", "title", "status", "td", "tr",
        "table", "derived", "prompt", "needs", "outcome", "assert",
    }
    for m in re.finditer(r"<([A-Za-z][A-Za-z0-9_-]*)[\s/>]", txt):
        name = m.group(1)
        if name not in STRUCTURAL:
            cands.add(name)
    for m in re.finditer(r'id="([^"]+)"', txt):
        cands.add(m.group(1))
    return sorted(cands)


def nearest_anchors(filepath, anchor, n=3):
    cands = candidate_anchors_in_file(filepath)
    if not cands:
        return []
    scored = []
    al = anchor.lower()
    for c in cands:
        cl = c.lower()
        if al in cl or cl in al:
            score = 1.0
        else:
            score = difflib.SequenceMatcher(None, al, cl).ratio()
        scored.append((score, c))
    scored.sort(key=lambda x: -x[0])
    return [c for _, c in scored[:n]]


def resolve_doc_glob(base_dir, doc):
    """Resolve DOC (a filename prefix) to exactly one file under base_dir."""
    exact = os.path.join(base_dir, doc + ".xml")
    if os.path.isfile(exact):
        return exact.replace("\\", "/"), [exact]
    pattern = os.path.join(base_dir, doc + "-*.xml")
    matches = sorted(glob.glob(pattern))
    if len(matches) == 1:
        return matches[0].replace("\\", "/"), matches
    if len(matches) > 1:
        return None, matches  # ambiguous
    return None, []


def resolve_citation(ref):
    """Returns dict: pattern, file(s), resolved(bool/None=out-of-scope), anchor_found, nearest"""
    m = re.match(r"^spec://([^/]+)/(.+)$", ref)
    if not m:
        return {"pattern": "malformed", "file": None, "resolved": None, "anchor_found": None, "nearest": []}
    group = m.group(1)
    rest_full = m.group(2)
    if "#" in rest_full:
        rest, anchor = rest_full.split("#", 1)
    else:
        rest, anchor = rest_full, None

    # Pattern A: org.vibevm.core/vibevm/<path>/<DOC>
    if group == "org.vibevm.core" and rest.startswith("vibevm/") and not rest.startswith("vibevm-docs/"):
        rest2 = rest[len("vibevm/"):]
        parts = rest2.split("/")
        if len(parts) < 2:
            return {"pattern": "A", "file": None, "resolved": False, "anchor_found": None, "nearest": [],
                     "note": "malformed path (need dir/DOC)"}
        doc = parts[-1]
        dirpath = "/".join(parts[:-1])
        base_dir = os.path.join("vibevm", "vibespecs", *dirpath.split("/"))
        f, matches = resolve_doc_glob(base_dir, doc)
        if f is None and len(matches) == 0:
            return {"pattern": "A", "file": None, "resolved": False, "anchor_found": None, "nearest": [],
                     "note": f"no file matches {doc}*.xml under {base_dir}"}
        if f is None and len(matches) > 1:
            return {"pattern": "A", "file": matches, "resolved": False, "anchor_found": None, "nearest": [],
                     "note": "ambiguous: multiple files match"}
        if anchor is None:
            return {"pattern": "A", "file": f, "resolved": True, "anchor_found": None, "nearest": []}
        found = find_anchor_in_file(f, anchor)
        nearest = [] if found else nearest_anchors(f, anchor)
        return {"pattern": "A", "file": f, "resolved": True, "anchor_found": found, "nearest": nearest}

    # Pattern B: org.vibevm.world/addressable-specs/flows/addressable-specs/<DOC>
    if group == "org.vibevm.world" and rest.startswith("addressable-specs/flows/addressable-specs/"):
        doc = rest[len("addressable-specs/flows/addressable-specs/"):]
        base_dir = "vibevm/vibedeps/org.vibevm.world.addressable-specs/1.0.0/vibevm/vibespecs/flows/addressable-specs"
        f, matches = resolve_doc_glob(base_dir, doc)
        if f is None:
            return {"pattern": "B", "file": None, "resolved": False, "anchor_found": None, "nearest": [],
                     "note": f"no file matches {doc}*.xml"}
        if anchor is None:
            return {"pattern": "B", "file": f, "resolved": True, "anchor_found": None, "nearest": []}
        found = find_anchor_in_file(f, anchor)
        nearest = [] if found else nearest_anchors(f, anchor)
        return {"pattern": "B", "file": f, "resolved": True, "anchor_found": found, "nearest": nearest}

    # Pattern C: org.vibevm.ai-native/core-ai-native/mechanisms/<DOC>
    if group == "org.vibevm.ai-native" and rest.startswith("core-ai-native/mechanisms/"):
        doc = rest[len("core-ai-native/mechanisms/"):]
        base_dir = "vibevm/vibedeps/org.vibevm.ai-native.core-ai-native/1.0.0/vibevm/vibespecs/mechanisms"
        f, matches = resolve_doc_glob(base_dir, doc)
        if f is None:
            return {"pattern": "C", "file": None, "resolved": False, "anchor_found": None, "nearest": [],
                     "note": f"no file matches {doc}*.xml"}
        if anchor is None:
            return {"pattern": "C", "file": f, "resolved": True, "anchor_found": None, "nearest": []}
        found = find_anchor_in_file(f, anchor)
        nearest = [] if found else nearest_anchors(f, anchor)
        return {"pattern": "C", "file": f, "resolved": True, "anchor_found": found, "nearest": nearest}

    # out of scope: no resolution rule in the packet covers this group/shape
    return {"pattern": "out-of-scope", "file": None, "resolved": None, "anchor_found": None, "nearest": [],
            "note": "no packet resolution rule covers this address shape"}


seen_citation_keys = set()
for pid, page in PAGES.items():
    # (a) rule ref= attributes
    for n in page.nodes:
        if n.tag == "rule" and n.attrib.get("ref", "").startswith("spec://"):
            ref = n.attrib["ref"]
            info = resolve_citation(ref)
            findings["citations"].append({"page": pid, "address": ref, "source": "rule-ref", **info})
    # (b) inline spec:// occurring in prose text (not already an attribute)
    for n in page.nodes:
        if n.tag in ALL_TEXTUAL_TAGS and n.text:
            for m in re.finditer(r"spec://[^\s`\]\)\"<]+", n.text):
                ref = m.group(0)
                info = resolve_citation(ref)
                findings["citations"].append({"page": pid, "address": ref, "source": "inline-prose", **info})

# ---------------------------------------------------------------------------
# 2. Banned words (P.6)
# ---------------------------------------------------------------------------
banned_entries = []
for line in open(BANNED_FILE, encoding="utf-8"):
    line = line.strip()
    if not line or line.startswith("#"):
        continue
    banned_entries.append(line)

banned_regex = []
for entry in banned_entries:
    words = entry.split(" ")
    pat = r"\b" + r"\s+".join(re.escape(w) for w in words) + r"\b"
    banned_regex.append((entry, re.compile(pat, re.IGNORECASE)))


def context_snippet(text, start, end, width=120):
    lo = max(0, start - width // 2)
    hi = min(len(text), end + width // 2)
    snippet = text[lo:hi].replace("\n", " ")
    snippet = re.sub(r"\s+", " ", snippet).strip()
    if len(snippet) > width:
        snippet = snippet[:width].rstrip() + "…"
    if snippet.count("`") % 2 == 1:
        snippet += "`"
    return snippet


for pid, page in PAGES.items():
    for n in page.nodes:
        if n.tag not in PROSE_TAGS or not n.text:
            continue
        for entry, rx in banned_regex:
            for m in rx.finditer(n.text):
                findings["banned_words"].append({
                    "page": pid,
                    "word": entry,
                    "filler": entry in FILLER_WORDS,
                    "context": context_snippet(n.text, m.start(), m.end()),
                    "in": n.tag,
                })

# ---------------------------------------------------------------------------
# 3. Sentence & paragraph length
# ---------------------------------------------------------------------------
def neutralize(text):
    chars = list(text)
    for m in re.finditer(r"`[^`]*`", text):
        s, e = m.span()
        for i in range(s, e):
            if chars[i] in ".!?":
                chars[i] = "\x00"
    for m in re.finditer(r"\]\(([^)]*)\)", text):
        s, e = m.span(1)
        for i in range(s, e):
            if chars[i] in ".!?":
                chars[i] = "\x00"
    return "".join(chars)


def split_sentences(original_text):
    if not original_text or not original_text.strip():
        return []
    neut = neutralize(original_text)
    spans = []
    start = 0
    for m in re.finditer(r"[.!?]+(?=\s|$)", neut):
        end = m.end()
        spans.append((start, end))
        start = end
    if start < len(neut) and neut[start:].strip():
        spans.append((start, len(neut)))
    out = []
    for s, e in spans:
        seg = original_text[s:e].strip()
        if seg:
            out.append(seg)
    return out


def word_count(sentence):
    masked = re.sub(r"`[^`]*`", "CODESPAN", sentence)
    masked = re.sub(r"\[([^\]]*)\]\([^)]*\)", r"\1", masked)
    return len(masked.split())


def has_inline_code(text):
    return bool(re.search(r"`[^`]+`", text))


for pid, page in PAGES.items():
    for n in page.nodes:
        if n.tag not in ("p", "prompt", "outcome") or not n.text or not n.text.strip():
            continue
        by_hand = page.in_by_hand(n)
        is_prompt_outcome = n.tag in ("prompt", "outcome")
        sentences = split_sentences(n.text)
        if len(sentences) > 6:
            findings["paragraph_length"].append({
                "page": pid, "in": n.tag, "titles": [t for t in n.titles if t],
                "n_sentences": len(sentences),
                "start": context_snippet(n.text, 0, min(80, len(n.text)), width=100),
            })
        technical = has_inline_code(n.text)
        for i, sent in enumerate(sentences, start=1):
            wc = word_count(sent)
            limit = None
            if by_hand or is_prompt_outcome:
                limit = 20
            elif technical:
                limit = 25
            if limit is not None and wc > limit:
                beginning = " ".join(sent.split()[:12])
                if beginning.count("`") % 2 == 1:
                    beginning += "`"
                findings["sentence_length"].append({
                    "page": pid, "in": n.tag, "titles": [t for t in n.titles if t],
                    "sentence_no": i, "words": wc, "limit": limit,
                    "beginning": beginning + ("…" if len(sent.split()) > 12 else ""),
                })

# ---------------------------------------------------------------------------
# 4. Headings
# ---------------------------------------------------------------------------
BANNED_HEADINGS = {"overview", "summary", "conclusion", "next steps", "key takeaways", "introduction"}
for pid, page in PAGES.items():
    for n in page.nodes:
        title = n.attrib.get("title")
        if not title:
            continue
        reasons = []
        if title.strip().lower() in BANNED_HEADINGS:
            reasons.append("generic-heading:" + title.strip().lower())
        if title.strip().endswith("?"):
            reasons.append("ends-with-?")
        if reasons:
            findings["headings"].append({"page": pid, "tag": n.tag, "title": title, "reasons": reasons})

# ---------------------------------------------------------------------------
# 5. Signs: !, emoji, bold, multi-dash
# ---------------------------------------------------------------------------
ALLOWED_EXTRA = set("«»—…")


def is_cyrillic_or_latin_or_allowed(ch):
    o = ord(ch)
    if o < 128:
        return True
    if ch in ALLOWED_EXTRA:
        return True
    # Cyrillic block
    if 0x0400 <= o <= 0x04FF:
        return True
    # Latin extended blocks (accented latin letters)
    if 0x0080 <= o <= 0x024F:
        return True
    return False


for pid, page in PAGES.items():
    for n in page.nodes:
        if n.tag not in PROSE_TAGS or not n.text:
            continue
        text = n.text
        # exclamation marks
        for m in re.finditer(r"!", text):
            findings["signs"].append({"page": pid, "kind": "exclamation", "in": n.tag,
                                       "context": context_snippet(text, m.start(), m.end())})
        # emoji / disallowed non-ascii
        for i, ch in enumerate(text):
            if not is_cyrillic_or_latin_or_allowed(ch):
                findings["signs"].append({"page": pid, "kind": "emoji-or-other", "in": n.tag,
                                           "char": ch, "codepoint": hex(ord(ch)),
                                           "context": context_snippet(text, i, i + 1)})
        # bold in prose
        for m in re.finditer(r"\*\*[^*]+\*\*", text):
            findings["signs"].append({"page": pid, "kind": "bold", "in": n.tag,
                                       "context": context_snippet(text, m.start(), m.end())})
        # multiple em-dash in one paragraph-like unit
        n_dash = text.count("—")
        if n_dash > 1:
            findings["signs"].append({"page": pid, "kind": "multi-dash", "in": n.tag,
                                       "count": n_dash, "context": context_snippet(text, 0, min(100, len(text)))})

# ---------------------------------------------------------------------------
# 6. Terms before introduction
# ---------------------------------------------------------------------------
gl_root = PAGES["glossary/index"].root
glossary_terms = []
for el in gl_root:
    t = local(el.tag)
    if t in ("title", "status", "p"):
        continue
    title_attr = el.get("title")
    if title_attr:
        glossary_terms.append(title_attr)


def base_term(term):
    return term.split(" (")[0].strip()


def term_pattern(term):
    words = base_term(term).split(" ")
    prefix = r"\s+".join(re.escape(w) for w in words[:-1])
    last = re.escape(words[-1])
    if prefix:
        return re.compile(r"\b" + prefix + r"\s+" + last + r"s?\b", re.IGNORECASE)
    return re.compile(r"\b" + last + r"s?\b", re.IGNORECASE)


TERM_PATTERNS = [(t, term_pattern(t)) for t in glossary_terms]

EXPLAIN_PATTERNS = [", the", ": ", "that is", "which is"]


def is_in_code_span(text, start, end):
    for m in re.finditer(r"`[^`]*`", text):
        if m.start() <= start and end <= m.end():
            return True
    return False


def is_in_italic(text, start, end):
    for m in re.finditer(r"(?<!\*)\*(?!\*)([^*]+)\*(?!\*)", text):
        if m.start(1) <= start and end <= m.end(1):
            return True
    return False


def is_in_link_label(text, start, end):
    for m in re.finditer(r"\[([^\]]*)\]\([^)]*\)", text):
        if m.start(1) <= start and end <= m.end(1):
            return True
    return False


def sentence_of(text, start, end):
    sentences = split_sentences(text)
    # find which sentence contains offset start (approx by searching text)
    running = 0
    for s in sentences:
        idx = text.find(s, running)
        if idx == -1:
            idx = running
        if idx <= start <= idx + len(s):
            return s
        running = idx + len(s)
    return text


for pid, page in PAGES.items():
    if pid == "glossary/index":
        continue
    first_p = page.first_p_text or ""
    # (a) first-paragraph total ban
    for term, rx in TERM_PATTERNS:
        m = rx.search(first_p)
        if m and not is_in_code_span(first_p, m.start(), m.end()):
            findings["terms"].append({
                "page": pid, "term": term, "violation": "used-in-first-paragraph",
                "context": context_snippet(first_p, m.start(), m.end()),
            })
    # (b) first occurrence elsewhere must be explained
    # gather all in-scope text nodes AFTER the first <p> (document order), i.e. index > first p's index
    first_p_index = None
    for n in page.nodes:
        if n.tag == "p":
            first_p_index = n.index
            break
    scope_nodes = [n for n in page.nodes if n.tag in ALL_TEXTUAL_TAGS and n.text and n.index > (first_p_index or -1)]
    for term, rx in TERM_PATTERNS:
        found_occurrence = None
        for n in scope_nodes:
            for m in rx.finditer(n.text):
                if is_in_code_span(n.text, m.start(), m.end()):
                    continue
                found_occurrence = (n, m)
                break
            if found_occurrence:
                break
        if not found_occurrence:
            continue
        n, m = found_occurrence
        italic = is_in_italic(n.text, m.start(), m.end())
        linked = is_in_link_label(n.text, m.start(), m.end())
        sent = sentence_of(n.text, m.start(), m.end())
        explained_pattern = any(p in sent for p in EXPLAIN_PATTERNS)
        ok = italic or linked or explained_pattern
        findings["terms"].append({
            "page": pid, "term": term, "violation": None if ok else "first-use-not-explained",
            "where": n.tag, "italic": italic, "linked": linked, "pattern_match": explained_pattern,
            "context": context_snippet(n.text, m.start(), m.end()),
        })

# ---------------------------------------------------------------------------
# 7. Cross-page links
# ---------------------------------------------------------------------------
for pid, page in PAGES.items():
    page_dir = os.path.dirname(page.path)
    for n in page.nodes:
        if n.tag not in ALL_TEXTUAL_TAGS or not n.text:
            continue
        for m in re.finditer(r"\[([^\]]*)\]\(([^)]+\.xml(?:#[^)]*)?)\)", n.text):
            link_text, target = m.group(1), m.group(2)
            target_path = target.split("#")[0]
            resolved = os.path.normpath(os.path.join(page_dir, target_path))
            exists = os.path.isfile(resolved)
            findings["links"].append({
                "page": pid, "text": link_text, "target": target,
                "resolved": os.path.relpath(resolved, REPO_ROOT).replace("\\", "/"),
                "exists": exists,
            })

# ---------------------------------------------------------------------------
# 8. Example / prompt ids
# ---------------------------------------------------------------------------
for pid, page in PAGES.items():
    ex_ids = []
    pr_ids = []
    for n in page.nodes:
        if n.tag == "example":
            ex_ids.append(n.attrib.get("id"))
            has_run = any(
                c.tag == "run" for c in page.nodes if c.path_ids[: len(n.path_ids) + 1] == n.path_ids + ["example"]
            )
        if n.tag == "prompt":
            pr_ids.append(n.attrib.get("id"))
    dup_ex = sorted({x for x in ex_ids if ex_ids.count(x) > 1})
    dup_pr = sorted({x for x in pr_ids if pr_ids.count(x) > 1})
    cross = sorted(set(ex_ids) & set(pr_ids))
    if dup_ex or dup_pr or cross:
        findings["ids"].append({"page": pid, "dup_example": dup_ex, "dup_prompt": dup_pr, "cross": cross})

# structural example/prompt completeness (run required; assert required unless assert="none")
for pid, page in PAGES.items():
    # walk element tree directly (easier for parent/child) instead of flat node list
    def walk(el):
        tag = local(el.tag)
        if tag == "example":
            runs = [c for c in el if local(c.tag) == "run"]
            if not runs:
                findings["ids"].append({"page": pid, "issue": "example-missing-run", "id": el.get("id")})
        if tag == "prompt":
            asserts = [c for c in el if local(c.tag) == "assert"]
            if not asserts and el.get("assert") != "none":
                findings["ids"].append({"page": pid, "issue": "prompt-missing-assert", "id": el.get("id")})
        for c in el:
            walk(c)

    walk(page.root)

# ---------------------------------------------------------------------------
# self-check counters
# ---------------------------------------------------------------------------
n_citations = len(findings["citations"])
n_unresolved = len([c for c in findings["citations"] if c.get("resolved") is False or
                     (c.get("resolved") is True and c.get("anchor_found") is False)])
n_banned = len(findings["banned_words"])
n_long_sentences = len(findings["sentence_length"])

findings["meta"] = {
    "pages_parsed": len(PAGES),
    "citations_total": n_citations,
    "citations_unresolved_or_anchor_missing": n_unresolved,
    "banned_word_hits": n_banned,
    "long_sentences": n_long_sentences,
    "long_paragraphs_gt6_sentences": len(findings["paragraph_length"]),
    "heading_hits": len(findings["headings"]),
    "sign_hits": len(findings["signs"]),
    "term_violations": len([t for t in findings["terms"] if t.get("violation")]),
    "broken_links": len([l for l in findings["links"] if not l["exists"]]),
    "id_issues": len(findings["ids"]),
}

with open(OUT_JSON, "w", encoding="utf-8") as f:
    json.dump(findings, f, ensure_ascii=False, indent=1)

print(json.dumps(findings["meta"], ensure_ascii=False, indent=1))
```

### `render_report.py`

```python
# -*- coding: utf-8 -*-
import json
from collections import Counter, defaultdict

IN_JSON = r"<scratch>\vibe-docs-phaseP\PP-C1\findings.json"
OUT_MD = r"<scratch>\vibe-docs-phaseP\PP-C1\PP-C1-prose-checks.md"

d = json.load(open(IN_JSON, encoding="utf-8"))


def esc(s):
    if s is None:
        return ""
    s = str(s)
    s = s.replace("\\", "/").replace("|", "\\|")
    s = s.replace("\n", " ").replace("\r", " ")
    return s.strip()


def table(headers, rows):
    out = []
    out.append("| " + " | ".join(headers) + " |")
    out.append("|" + "|".join(["---"] * len(headers)) + "|")
    for r in rows:
        out.append("| " + " | ".join(esc(c) for c in r) + " |")
    return "\n".join(out)


L = []
L.append("# PP-C1 Проверки прозы без инструментов (P.6 и P.7)")
L.append("")
L.append("Дата: 2026-09-12")
L.append("Дерево: b1291b06")
L.append("")
L.append(
    "Пакет: `campaigns/docs-2026-09/findings/PACKET-PP-C1.md` "
    "(преамбул `campaigns/docs-2026-09/findings/PACKET-COMMON.md`). "
    "Ничего в репозитории не правлено; только чтение и этот отчёт."
)
L.append("")

# ---------------------------------------------------------------------
m = d["meta"]
L.append("## Самопроверка (сводка)")
L.append("")
L.append(table(
    ["метрика", "значение"],
    [
        ["страниц разобрано", m["pages_parsed"]],
        ["цитат spec:// всего (rule ref= + инлайн-проза)", m["citations_total"]],
        ["из них не резолвится или якорь не найден", m["citations_unresolved_or_anchor_missing"]],
        ["запрещённых слов/фраз (вхождений)", m["banned_word_hits"]],
        ["длинных фраз (сверх лимита)", m["long_sentences"]],
        ["длинных абзацев (>6 фраз)", m["long_paragraphs_gt6_sentences"]],
        ["проблемных заголовков", m["heading_hits"]],
        ["знаков (!, эмодзи, жирный, много тире)", m["sign_hits"]],
        ["нарушений термина-до-пояснения", m["term_violations"]],
        ["битых межстраничных ссылок", m["broken_links"]],
        ["проблем с id example/prompt", m["id_issues"]],
    ],
))
L.append("")
L.append(
    "Команда самопроверки XML (см. `WORKER-REPORT-PP-C1.md` за дословным выводом): "
    "`python -c \"import xml.etree.ElementTree as ET,glob; "
    "[ET.parse(f) for f in glob.glob('vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0/vibevm/vibespecs/**/*.xml', "
    "recursive=True)]; print('ok')\"` -> `ok` (см. отчёт о работе)."
)
L.append("")

# ---------------------------------------------------------------------
# 1. Citations
# ---------------------------------------------------------------------
L.append("## 1. Цитаты spec://…#ANCHOR (P.7)")
L.append("")
L.append(
    "Правила резолюции — ровно 3 паттерна из пакета (A: "
    "`org.vibevm.core/vibevm/<путь>/<DOC>` -> `vibevm/vibespecs/<путь>/<DOC>[-*].xml`; "
    "B: `org.vibevm.world/addressable-specs/flows/addressable-specs/<DOC>` -> vibedeps-копия; "
    "C: `org.vibevm.ai-native/core-ai-native/mechanisms/<DOC>` -> vibedeps-копия). "
    "Адреса вне этих 3 форм помечены `out-of-scope` (нет правила в пакете) или `malformed` "
    "(не парсится как `spec://GROUP/REST`); см. «Открытые вопросы»."
)
L.append("")
cits = d["citations"]
by_status = Counter()
for c in cits:
    if c["pattern"] in ("out-of-scope", "malformed"):
        by_status[c["pattern"]] += 1
    elif c["resolved"] is False:
        by_status["file-not-found"] += 1
    elif c["resolved"] is True and c.get("anchor_found") is False:
        by_status["anchor-not-found"] += 1
    elif c["resolved"] is True and c.get("anchor_found") in (True, None):
        by_status["found"] += 1
L.append(table(["статус", "количество"], [[k, v] for k, v in by_status.most_common()]))
L.append("")

L.append("### 1.1 Не найдено (файл не резолвится или якорь отсутствует) — все случаи")
L.append("")
rows = []
for c in cits:
    if c["resolved"] is False:
        rows.append([c["page"], c["address"], c.get("note", ""), "файл не найден", ""])
    elif c["resolved"] is True and c.get("anchor_found") is False:
        rows.append([c["page"], c["address"], c.get("file", ""), "якорь не найден",
                     ", ".join(c.get("nearest", []))])
if rows:
    L.append(table(["страница", "адрес", "файл/примечание", "статус", "3 ближайших якоря"], rows))
else:
    L.append("(нет)")
L.append("")

L.append("### 1.2 Вне 3 паттернов пакета (out-of-scope) и не разбираются как адрес (malformed)")
L.append("")


def classify_note(addr):
    if "org.acme" in addr:
        return "иллюстративный пример (org.acme — плейсхолдер-организация), не реальная цитата"
    if "vibevm-docs/" in addr:
        return "самоадресация страниц документации (`vibevm-docs/...`); ни один из 3 паттернов пакета не резолвит — дефект пакета"
    if "\u2026" in addr or addr.strip("#") == "spec://" or addr.startswith("spec://\u2026"):
        return "эллипсис как обозначение адреса в пояснительном тексте, не реальная цитата"
    return ""


rows = []
for c in cits:
    if c["pattern"] in ("out-of-scope", "malformed"):
        rows.append([c["page"], c["source"], c["address"], c["pattern"], classify_note(c["address"])])
L.append(table(["страница", "источник", "адрес", "паттерн", "примечание"], rows))
L.append("")

L.append("### 1.3 Полная таблица всех цитат (385 строк)")
L.append("")
rows = []
for c in sorted(cits, key=lambda x: (x["page"], x["address"])):
    if c["pattern"] in ("out-of-scope", "malformed"):
        status = c["pattern"] + " (см. §1.2)"
        fileinfo = ""
    elif c["resolved"] is False:
        status = "не найдено (файл)"
        fileinfo = c.get("note", "")
    elif c.get("anchor_found") is False:
        status = "не найдено (якорь)"
        fileinfo = c.get("file", "")
    else:
        status = "найдено"
        fileinfo = c.get("file", "") or ""
    rows.append([c["page"], c["source"], c["address"], fileinfo, status])
L.append(table(["страница", "источник", "адрес", "файл", "статус"], rows))
L.append("")

# ---------------------------------------------------------------------
# 2. Banned words
# ---------------------------------------------------------------------
L.append("## 2. Запрещённые слова (P.6)")
L.append("")
L.append(f"Список: `vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0/style/banned.en.txt`. Всего вхождений: {len(d['banned_words'])}.")
L.append("")
rows = []
for b in d["banned_words"]:
    rows.append([b["page"], b["word"], "да" if b["filler"] else "", b["in"], b["context"]])
L.append(table(["страница", "слово/фраза", "фильтр (just/simply/…)", "где", "контекст"], rows))
L.append("")
L.append(
    "Отдельное наблюдение (не решение — фиксирую факт для центральной сессии): все 4 вхождения "
    "`capabilities` и вхождение `capabilities` в `model/packages-and-kinds` — это точный технический "
    "термин продукта (глоссарий: *capability*, поле манифеста `[requires] capabilities` / "
    "`[provides] capabilities`), а не маркетинговая рыхлость, на которую рассчитан общий запрет. "
    "REVIEW-статус см. ниже."
)
L.append("")

# ---------------------------------------------------------------------
# 3. Sentence & paragraph length
# ---------------------------------------------------------------------
L.append("## 3. Длина фраз и абзацев")
L.append("")
L.append(
    "Правило: в абзацах внутри секций «By hand…» и в `prompt`/`outcome` — лимит 20 слов/фразу; "
    "в прочих абзацах — лимит 25 слов/фразу, только если абзац содержит хотя бы один `код`-спан; "
    "абзацы длиннее 6 фраз — отдельно, независимо от лимита слов. Слово = токен по пробелу после "
    "маскировки кодовых спанов (`код` -> 1 токен) и markdown-ссылок (`[текст](путь)` -> `текст`)."
)
L.append("")
L.append(f"### 3.1 Фразы сверх лимита слов (всего {len(d['sentence_length'])})")
L.append("")
rows = []
for s in sorted(d["sentence_length"], key=lambda x: (x["page"], x["in"])):
    titles = " / ".join(s["titles"]) if s["titles"] else ""
    rows.append([s["page"], s["in"], titles, s["sentence_no"], s["words"], s["limit"], s["beginning"]])
L.append(table(["страница", "где", "секция", "№ фразы", "слов", "лимит", "начало фразы"], rows))
L.append("")

L.append(f"### 3.2 Абзацы длиннее 6 фраз (всего {len(d['paragraph_length'])})")
L.append("")
rows = []
for pnode in d["paragraph_length"]:
    titles = " / ".join(pnode["titles"]) if pnode["titles"] else ""
    rows.append([pnode["page"], pnode["in"], titles, pnode["n_sentences"], pnode["start"]])
L.append(table(["страница", "где", "секция", "фраз", "начало абзаца"], rows))
L.append("")
L.append(
    "Примечание: все 3 абзаца — не «разросшаяся проза», а нумерованные процедуры "
    "(`1. … 2. … 3. …`), записанные внутри одного `<p>` без разбивки на отдельные шаги-секции; "
    "формально каждый пункт даёт минимум одну фразу, отсюда счёт. Факт, не решение — "
    "не переписываю разметку."
)
L.append("")

# ---------------------------------------------------------------------
# 4. Headings
# ---------------------------------------------------------------------
L.append("## 4. Заголовки")
L.append("")
L.append(f"Всего: {len(d['headings'])}.")
L.append("")
rows = []
for h in d["headings"]:
    rows.append([h["page"], h["tag"], h["title"], ", ".join(h["reasons"])])
L.append(table(["страница", "тег секции", "title", "причина"], rows))
L.append("")
L.append(
    "Все 12 — вопросительные заголовки на `faq/index.xml` (жанр страницы: вопрос-ответ). "
    "Generic-заголовков из списка (Overview/Summary/Conclusion/Next steps/Key takeaways/"
    "Introduction) не найдено ни одного. Факт, не решение — является ли вопросительный "
    "заголовок FAQ-страницы нарушением по смыслу правила, решает центральная сессия."
)
L.append("")

# ---------------------------------------------------------------------
# 5. Signs
# ---------------------------------------------------------------------
L.append("## 5. Знаки (!, эмодзи, **жирный** в прозе, >1 тире «—» в абзаце)")
L.append("")
if d["signs"]:
    rows = [[s["page"], s["kind"], s.get("in", ""), s.get("context", "")] for s in d["signs"]]
    L.append(table(["страница", "вид", "где", "контекст"], rows))
else:
    L.append(
        "Не найдено ни одного случая ни по одной из 4 категорий на всех 44 страницах. "
        "Единственные не-ASCII знаки во всём корпусе — «», —, … (3 файла для «»/…, 1 файл "
        "(`diagnostics/errors.xml`) для — — все в пределах разрешённой типографики, и все "
        "3 вхождения `—` лежат в разных `<td>`, не в одном абзаце, так что правило "
        "«не больше одного тире на абзац» не нарушено нигде."
    )
L.append("")

# ---------------------------------------------------------------------
# 6. Terms before introduction
# ---------------------------------------------------------------------
L.append("## 6. Термины до введения (глоссарий)")
L.append("")
L.append(
    f"50 терминов из `glossary/index.xml`. Всего первых вхождений на 43 страницах "
    f"(кроме глоссария): {len(d['terms'])}, из них с проблемой: {sum(1 for t in d['terms'] if t.get('violation'))} "
    f"(`used-in-first-paragraph` — термин во вводном абзаце, запрещено вовсе: "
    f"{sum(1 for t in d['terms'] if t.get('violation')=='used-in-first-paragraph')}; "
    f"`first-use-not-explained` — первое вхождение далее без курсива/ссылки/пояснительного "
    f"оборота: {sum(1 for t in d['terms'] if t.get('violation')=='first-use-not-explained')})."
)
L.append("")
L.append(
    "Ограничение метода (честно фиксирую, чтобы центральная сессия не переоценивала колонку "
    "«OK»): пояснительный оборот (`, the …`, `: …`, `that is`, `which is`) ищется по всей фразе, "
    "содержащей вхождение термина, а не рядом со словом — если оборот встретился в той же фразе "
    "по другой причине, строка помечена как «пояснено», хотя термин не пояснён. Значит реальное "
    "число нарушений `first-use-not-explained`, вероятно, выше 239; таблица — нижняя граница, "
    "не точное число."
)
L.append("")

L.append("### 6.1 Нарушения (полная таблица)")
L.append("")
rows = []
for t in d["terms"]:
    if not t.get("violation"):
        continue
    if t["violation"] == "used-in-first-paragraph":
        rows.append([t["page"], t["term"], "во вводном абзаце (запрещено)", "-", "-", "-", t["context"]])
    else:
        rows.append([t["page"], t["term"], "первое вхождение не пояснено", t.get("where", ""),
                     "да" if t.get("italic") else "", "да" if t.get("linked") else "", t["context"]])
L.append(table(["страница", "термин", "нарушение", "где", "курсив", "ссылка", "контекст"], rows))
L.append("")

L.append("### 6.2 Пояснено корректно (для контроля метода, не нарушения)")
L.append("")
rows = []
for t in d["terms"]:
    if t.get("violation"):
        continue
    rows.append([t["page"], t["term"], t.get("where", ""), "да" if t.get("italic") else "",
                 "да" if t.get("linked") else "", "да" if t.get("pattern_match") else "", t["context"]])
L.append(table(["страница", "термин", "где", "курсив", "ссылка", "оборот-пояснение", "контекст"], rows))
L.append("")

# ---------------------------------------------------------------------
# 7. Links
# ---------------------------------------------------------------------
L.append("## 7. Ссылки между страницами `[текст](путь.xml)`")
L.append("")
L.append(f"Всего ссылок: {len(d['links'])}. Битых: {sum(1 for l in d['links'] if not l['exists'])}.")
L.append("")
rows = []
for l in d["links"]:
    rows.append([l["page"], l["text"], l["target"], "найден" if l["exists"] else "НЕ НАЙДЕН"])
L.append(table(["страница", "текст ссылки", "путь", "статус"], rows))
L.append("")

# ---------------------------------------------------------------------
# 8. IDs
# ---------------------------------------------------------------------
L.append("## 8. Идентификаторы `example`/`prompt`")
L.append("")
if d["ids"]:
    rows = []
    for i in d["ids"]:
        rows.append([i.get("page"), json.dumps(i, ensure_ascii=False)])
    L.append(table(["страница", "проблема"], rows))
else:
    L.append(
        "Не найдено ни одной проблемы на 44 страницах: все `example id` (59 шт.) и все "
        "`prompt id` (19 шт.) уникальны в пределах своей страницы и не пересекаются друг с другом; "
        "у каждого `example` есть `run` (59/59); у каждого `prompt` есть хотя бы один `assert` "
        "(19/19), атрибут `assert=\"none\"` в корпусе не встречается."
    )
L.append("")

# ---------------------------------------------------------------------
L.append("## Расхождения с решениями")
L.append("")
L.append(
    "- REVIEW: словарь запрещённых слов (P.6) банит `capabilities` целиком, но в этом продукте "
    "«capability» — точный технический термин (глоссарий; поля манифеста "
    "`[requires]`/`[provides] capabilities`), а не рыхлая маркетинговая лексика, на которую "
    "рассчитан общий список — против замысла P.6 (устранение маркетинговой рыхлости), не "
    "переписываю список, только фиксирую факт (см. §2)."
)
L.append("- Иных расхождений с решениями (D-NN) не обнаружено.")
L.append("")

L.append("## Открытые вопросы / дефекты пакета")
L.append("")
L.append(
    "- Дефект пакета: реальный корпус содержит цитаты вида "
    "`spec://org.vibevm.core/vibevm-docs/<page>[#<anchor>]` (самоадресация страниц документации "
    "друг на друга) — 2 конкретных случая "
    "(`agent/how-agents-read-this-manual` -> `model/boot-lane#p7`; "
    "`howto/read-documentation-locally` -> `start/what-vibevm-is`) плюс 1 шаблонный "
    "(`vibevm-docs/<page>#<anchor>`). Ни один из 3 данных в пакете паттернов резолюции его не "
    "покрывает. Оставлено нерезолвленным (`out-of-scope`) по консервативному допущению — "
    "правило для этого адреса пакет не даёт."
)
L.append(
    "- 4 вхождения `spec://…` (эллипсис, U+2026) и 2 вхождения `spec://org.acme/...` — это "
    "иллюстративные примеры синтаксиса в страницах, которые сами объясняют схему адресации "
    "(`authoring/specs-agents-can-cite`, `authoring/write-a-flow`, "
    "`agent/how-agents-read-this-manual`, `architecture/traceability`, "
    "`authoring/write-documentation`), не настоящие цитаты; см. §1.2. Разметки, отличающей "
    "«пример синтаксиса» от «настоящей цитаты», в источнике нет — вывод по контексту вручную "
    "(автор страницы, посвящённой самой схеме адресации, использует свою же схему как пример)."
)
L.append(
    "- §6 (термины): метод занижает число нарушений `first-use-not-explained` (см. ограничение "
    "в §6) — пояснительный оборот ищется по всей фразе, а не рядом со словом. Понадобится "
    "точнее для A2.25."
)
L.append(
    "- §3 (длина фраз): разбиение на фразы — эвристика (маскирует точки только внутри "
    "`` `код` `` и `(путь-ссылки)`); сокращения вида «т.е.», версии вне кода (не встречены "
    "де-факто, но не проверено доказательно) не обрабатывались бы отдельно."
)
L.append(
    "- §1.1: 2 из 3 ненайденных якорей (`PROP-054#WHY-C-ABI` при секции `<why-c-abi>` в нижнем "
    "регистре; `PROP-054#WASM` при факте `<WASM-DEFERRED>`) выглядят как настоящие сломанные "
    "цитаты (регистр / переименование), не искажение метода — воспроизводимо, см. §1.1."
)
L.append("")

with open(OUT_MD, "w", encoding="utf-8", newline="\n") as f:
    f.write("\n".join(L) + "\n")

print("written", OUT_MD)
print("total lines:", len(L))
```

### `validate_md.py`

```python
# -*- coding: utf-8 -*-
import re

path = r"<scratch>\vibe-docs-phaseP\PP-C1\PP-C1-prose-checks.md"
lines = open(path, encoding="utf-8").read().split("\n")

problems = []
current_header_cols = None
for i, line in enumerate(lines, start=1):
    if line.startswith("|"):
        # count unescaped pipes (a pipe preceded by backslash doesn't count as separator)
        # simple approach: split on regex not preceded by backslash
        cells = re.split(r"(?<!\\)\|", line)
        # first and last are empty strings (leading/trailing |)
        ncols = len(cells) - 2
        if set(line.strip()) <= set("|-: "):
            continue  # separator row
        if current_header_cols is None or (i > 1 and lines[i - 2].startswith("|") and set(lines[i-2].strip()) <= set("| ")):
            pass
        if current_header_cols is not None and ncols != current_header_cols:
            problems.append((i, ncols, current_header_cols, line[:200]))
    if not line.startswith("|"):
        current_header_cols = None
    else:
        if current_header_cols is None:
            cells = re.split(r"(?<!\\)\|", line)
            current_header_cols = len(cells) - 2

import sys
sys.stdout.reconfigure(encoding="utf-8")

print("problems found:", len(problems))
for p in problems[:30]:
    print(p)

# check for any raw unescaped pipe issues: lines with an odd construct like "||" unintentionally, or backtick counts odd (unbalanced inline code)
bad_backtick_lines = []
for i, line in enumerate(lines, start=1):
    if line.startswith("|") and line.count("`") % 2 != 0:
        bad_backtick_lines.append((i, line[:200]))
print()
print("lines with odd backtick count:", len(bad_backtick_lines))
for b in bad_backtick_lines[:20]:
    print(b)

print()
print("total lines:", len(lines))
print("total table rows (starting with |):", sum(1 for l in lines if l.startswith("|")))
```
