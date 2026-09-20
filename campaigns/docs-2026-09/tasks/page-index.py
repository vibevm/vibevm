"""Regenerate PAGE-INDEX.md of the campaign zone from the pages of the core manual.

Mechanical: one row per page and per named section, with the first words of
the section's opening paragraph. Run from the repository root:

    python campaigns/docs-2026-09/tasks/page-index.py

The index is the citation target list for the legacy inventory (P.5) and the
page list of the wave B handover packet (P.9). Regenerate after any page edit.
"""
import glob
import os
import xml.etree.ElementTree as ET

ROOT = "vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0/vibevm/vibespecs"
NS = "{https://vibevm.org/spec/1}"
OUT = "campaigns/docs-2026-09/PAGE-INDEX.md"

rows = []
pages = 0
counts = {"example": 0, "prompt": 0, "rule": 0, "derived": 0, "figure": 0}
for f in sorted(glob.glob(ROOT + "/**/*.xml", recursive=True)):
    page = os.path.relpath(f, ROOT).replace(os.sep, "/")[:-4]
    if page.startswith("skills"):
        continue
    pages += 1
    t = ET.parse(f).getroot()
    rows.append(f"| `{page}` | `root` | {t.find(NS + 'title').text} | |")
    for el in t:
        if el.get("title") is None:
            continue
        p = el.find(NS + "p")
        first = " ".join((p.text or "").split()[:12]) if p is not None else ""
        rows.append(f"| `{page}` | `{el.tag.replace(NS, '')}` | {el.get('title')} | {first} |")
    for name in counts:
        counts[name] += sum(1 for _ in t.iter(NS + name))

head = [
    "# Индекс страниц и якорей руководства ядра {#root}",
    "",
    '<status stage="doc" state="work" comment="генерируется скриптом campaigns/docs-2026-09/tasks/page-index.py из страниц пакета vibevm-docs; руками не править"/>',
    "",
    f"Страниц: {pages}; секций с якорями: {len(rows) - pages}; примеров: {counts['example']}; "
    f"промптов: {counts['prompt']}; ссылок `rule`: {counts['rule']}; блоков `derived`: {counts['derived']}; "
    f"фигур: {counts['figure']}.",
    "",
    "| Страница | Якорь | Заголовок | Первые слова |",
    "|---|---|---|---|",
]
open(OUT, "w", encoding="utf-8", newline="\n").write("\n".join(head + rows) + "\n")
print("pages", pages, "rows", len(rows), counts)
