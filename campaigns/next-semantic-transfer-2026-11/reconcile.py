#!/usr/bin/env python3
"""Reconcile the changed-file inventory between two revisions against specs.

Usage: reconcile.py <repo> <base-rev> <head-rev> [> report.md]

Read-only: uses `git rev-parse`, `git diff`, `git ls-tree` and
`git cat-file --batch` only. Standard library only. The output is a pure
function of the two commits (no clock, no environment, sorted everywhere).
"""

import collections
import json
import posixpath
import re
import subprocess
import sys

UNMATCHED_CAP = 200
GIT_CFG = ["-c", "core.quotepath=off", "-c", "diff.renameLimit=20000"]

SPEC_URI = re.compile(r"spec://[A-Za-z0-9_.\-/]+(?:#[A-Za-z0-9_.\-]+)?")
SPEC_ID = re.compile(r"^(PROP|FEAT|MT)-\d+")
INLINE_TEST = re.compile(r"#\[cfg\(test\)\]|#\[(?:tokio::)?test\b|#\[test_case|proptest!")
SOURCE_EXT = (".rs",)

CAMPAIGN = "campaigns/next-semantic-transfer-2026-11/"
REPAIR = CAMPAIGN + "NST-AI-NATIVE-GLOBAL-REPAIR.md"

# Owner mandates of the campaign. Only NST-M9 is an identifier used by the
# campaign documents themselves; the R-NN identifiers are assigned by this
# report. `needle` is an exact substring checked against `source` at the head
# revision; `specs` and `paths` select the implementing areas mechanically.
MANDATES = [
    {
        "id": "R-01", "date": "2026-11-03", "source": CAMPAIGN + "LOG.md",
        "needle": "семантически перенести его возможности в основную VibeVM",
        "quote": "Владелец поручил критически исследовать причины слопа в NEXT и "
                 "семантически перенести его возможности в основную VibeVM, используя "
                 "прототип как материал для проверки, а не как готовый образец для bulk merge.",
        "specs": ["PROP-035", "PROP-066", "PROP-071", "PROP-072"],
        "paths": [CAMPAIGN + "features/", CAMPAIGN + "research/"],
    },
    {
        "id": "R-02", "date": "2026-11-03", "source": CAMPAIGN + "LOG.md",
        "needle": "недостроенную фичу следует честно достроить при переносе",
        "quote": "Нужную, но недостроенную фичу следует честно достроить при переносе.",
        "specs": [], "paths": [],
        "note": "Cross-cutting completion rule over the R-01 scope; it has no area of its own.",
    },
    {
        "id": "R-03", "date": "2026-11-03", "source": CAMPAIGN + "LOG.md",
        "needle": "Тщательно документируется весь ход кампании, не только изменения кода",
        "quote": "Тщательно документируется весь ход кампании, не только изменения кода.",
        "specs": [], "paths": [CAMPAIGN],
    },
    {
        "id": "R-04", "date": "2026-11-04", "source": CAMPAIGN + "LOG.md",
        "needle": "общего решения всех видов пакетов",
        "quote": "Затем поручил исследовать перенос/обобщение этой системы для "
                 "законченного общего решения всех видов пакетов.",
        "specs": ["PROP-065", "PROP-069", "PROP-077"], "paths": [],
    },
    {
        "id": "R-05", "date": "2026-11-04", "source": CAMPAIGN + "LOG.md",
        "needle": "отдельно фиксировать изменения vibe.toml, включая vibe install -g",
        "quote": "Владелец поручил отдельно фиксировать изменения vibe.toml, включая "
                 "vibe install -g, чтобы позднее объяснить переход между версиями.",
        "specs": ["PROP-044"], "paths": [CAMPAIGN + "FORMAT-CHANGES.md", "formats/"],
    },
    {
        "id": "NST-M9", "date": "2026-11-04", "source": CAMPAIGN + "LOG.md",
        "needle": "foundational tasks first, their consumers later",
        "quote": "Before any full copy, owner explicitly required architectural "
                 "classification: foundational tasks first, their consumers later.",
        "specs": [], "paths": [CAMPAIGN + "ARCHITECTURE.md", CAMPAIGN + "architecture/"],
    },
    {
        "id": "R-07", "date": "2026-11-07", "source": CAMPAIGN + "architecture/VIBE-NEEDS.md",
        "needle": "это дополнительный обязательный owner scope",
        "quote": "Требования владельца от 2026-11-07 встроены в те же L0–L5. "
                 "Исторические222задачи и113старых приёмок не перенумеровываются; "
                 "это дополнительный обязательный owner scope.",
        "specs": ["PROP-068", "PROP-073"],
        "paths": [CAMPAIGN + "architecture/VIBE-NEEDS.md", CAMPAIGN + "architecture/vibe-needs.json",
                  CAMPAIGN + "features/VN-"],
        "note": "Eight needs VN-01..VN-08 over nineteen VN-* nodes (architecture/vibe-needs.json).",
    },
    {
        "id": "R-08", "date": "2026-11", "source": CAMPAIGN + "LOG.md",
        "needle": "finish N3 and N4 after N2 to deliver useful native Git",
        "quote": "Owner explicitly expanded the current mandate: finish N3 and N4 after "
                 "N2 to deliver useful native Git.",
        "specs": ["PROP-001"],
        "paths": ["crates/vibe-registry/src/git_backend/", "crates/vibe-registry/src/git_package_registry/",
                  "crates/vibe-registry/src/git_registry/"],
    },
    {
        "id": "R-09", "date": "2026-11-18", "source": REPAIR,
        "needle": "Необходимо составить полный список измененных файлов",
        "quote": "Необходимо составить полный список измененных файлов, и сделать по ним "
                 "большой проход, который будет идти по коду и проставлять для функций "
                 "аннотации на соответствующие пункты спецификаций.",
        "specs": [], "paths": [REPAIR],
        "note": "Scope is every changed area of this inventory; this report is its child 3 (RECONCILE).",
    },
    {
        "id": "R-10", "date": "2026-11-18", "source": REPAIR,
        "needle": "расстановку specmark маркеров на всех важных функциях",
        "quote": "я хочу достаточно подробную расстановку specmark маркеров на всех "
                 "важных функциях, которые связаны со спецификациями",
        "specs": [], "paths": [],
        "note": "Measured by the per-crate table above (files with and without a spec reference).",
    },
    {
        "id": "R-11", "date": "2026-11-18", "source": REPAIR,
        "needle": "пройти по механизмам vibe install -g и удалить всё легаси",
        "quote": "я прошу пройти по механизмам vibe install -g и удалить всё легаси в "
                 "задачах касающихся NST (пока это все еще возможно, потому что у нас нет "
                 "у этих задач пользователей).",
        "specs": ["PROP-020", "PROP-059"], "paths": [],
    },
    {
        "id": "R-12", "date": "2026-11-19", "source": REPAIR,
        "needle": "будучи запущенными из внешнего красивого графического приложения",
        "quote": "одна из ключевых задач релиза 1.0.12 это то, чтобы все описанные нами "
                 "фичи работали, будучи запущенными из внешнего красивого графического приложения",
        "specs": ["PROP-060"], "paths": ["schemas/", "crates/vibe-wire/"],
        "note": "A review lens over existing machine-facing contracts, not a new scope.",
    },
]


def git(repo, *args):
    out = subprocess.run(["git", "-C", repo, *GIT_CFG, *args], check=True,
                         stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    return out.stdout


def read_blobs(repo, specs):
    """Return {spec: text or None} for `rev:path` specs through one batch process."""
    if not specs:
        return {}
    proc = subprocess.Popen(["git", "-C", repo, "cat-file", "--batch"],
                            stdin=subprocess.PIPE, stdout=subprocess.PIPE)
    data, _ = proc.communicate(("\n".join(specs) + "\n").encode())
    result, pos = {}, 0
    for spec in specs:
        end = data.index(b"\n", pos)
        header = data[pos:end].decode("utf-8", "replace")
        pos = end + 1
        parts = header.split(" ")
        if parts[-1] == "missing" or len(parts) < 3 or parts[1] != "blob":
            result[spec] = None
            if len(parts) >= 3 and parts[-1] != "missing":
                pos += int(parts[2]) + 1
            continue
        size = int(parts[2])
        result[spec] = data[pos:pos + size].decode("utf-8", "replace")
        pos += size + 1
    return result


def area_of(path):
    p = path.split("/")
    if p[0] == "crates" and len(p) > 2:
        return "crate:" + p[1]
    if p[0] == "xtask":
        return "crate:xtask"
    if p[0] == "campaigns" and len(p) > 2:
        return "campaigns/" + p[1]
    if p[0] == "vibevm" and len(p) > 2:
        return "vibevm/" + p[1]
    if len(p) == 1:
        return "(repository root)"
    return p[0]


def is_test_path(path):
    base = posixpath.basename(path)
    return ("/tests/" in path or "/fixtures/" in path or "/testdata/" in path
            or base in ("tests.rs", "test_support.rs", "support.rs") and "/tests" in path + "/"
            or base == "tests.rs" or base.endswith("_tests.rs") or base.endswith("_test.rs")
            or base == "test_support.rs" or base.startswith("test_")
            or path.startswith("conformance/") or path.startswith("formats/corpora/"))


def kind_of(path):
    base = posixpath.basename(path)
    if "/vibedeps/" in "/" + path or path.startswith("vibevm/vibedeps/"):
        return "generated"
    if (path.startswith("crates/vibe-wire/src/generated/") or base == "specmap.json"
            or base.endswith(".lock") or path.startswith("vibevm/vibespecs/boot/")):
        return "generated"
    if path.startswith("vibevm/vibespecs/"):
        return "spec"
    if is_test_path(path):
        return "test"
    if path.endswith(SOURCE_EXT) and (path.startswith("crates/") or path.startswith("xtask/")):
        return "source"
    if path.endswith((".md", ".txt")):
        return "doc"
    if path.endswith((".py", ".sh", ".ps1", ".cmd", ".mjs", ".cjs", ".ts", ".tsx", ".c", ".rs")):
        return "script"
    return "data"


def spec_id(uri):
    doc = uri.split("#", 1)[0].rstrip("/").rsplit("/", 1)[-1]
    m = SPEC_ID.match(doc)
    return m.group(0) if m else doc


def module_stem(path):
    base = posixpath.basename(path)[:-3]
    if base in ("mod", "lib", "main"):
        parent = posixpath.basename(posixpath.dirname(path))
        return None if parent in ("src", "bin") else parent
    return base


def table(headers, rows):
    out = ["| " + " | ".join(headers) + " |", "|" + "|".join("---" for _ in headers) + "|"]
    out += ["| " + " | ".join(str(c) for c in r) + " |" for r in rows]
    return out


def main():
    if len(sys.argv) != 4:
        sys.exit(__doc__)
    repo, base_arg, head_arg = sys.argv[1:4]
    base = git(repo, "rev-parse", base_arg + "^{commit}").decode().strip()
    head = git(repo, "rev-parse", head_arg + "^{commit}").decode().strip()
    tags = sorted(git(repo, "tag", "--points-at", base).decode().split())
    commits = int(git(repo, "rev-list", "--count", base + ".." + head).decode())

    # --- inventory -------------------------------------------------------
    files = {}  # path -> dict(status, old, add, del)
    raw = git(repo, "diff", "--name-status", "-M", "-z", base + ".." + head).decode("utf-8", "replace").split("\0")
    i = 0
    while i < len(raw) and raw[i]:
        st = raw[i][0]
        if st in "RC":
            files[raw[i + 2]] = {"status": st, "old": raw[i + 1], "add": 0, "del": 0}
            i += 3
        else:
            files[raw[i + 1]] = {"status": st, "old": None, "add": 0, "del": 0}
            i += 2
    raw = git(repo, "diff", "--numstat", "-M", "-z", base + ".." + head).decode("utf-8", "replace").split("\0")
    i = 0
    while i < len(raw) and raw[i]:
        a, d, name = raw[i].split("\t", 2)
        if name == "":
            name = raw[i + 2]
            i += 3
        else:
            i += 1
        if name in files and a != "-":
            files[name]["add"], files[name]["del"] = int(a), int(d)
        elif name in files:
            files[name]["binary"] = True

    head_tree = set(git(repo, "ls-tree", "-r", "--name-only", "-z", head).decode("utf-8", "replace").split("\0"))
    head_tree.discard("")

    # --- content ---------------------------------------------------------
    def blob_spec(path):
        f = files[path]
        return (base if f["status"] == "D" else head) + ":" + path

    wanted = [blob_spec(p) for p in sorted(files)]
    crate_tests = collections.defaultdict(list)  # crate area -> test files at head
    for p in sorted(head_tree):
        if p.endswith(".rs") and area_of(p).startswith("crate:") and is_test_path(p):
            crate_tests[area_of(p)].append(p)
    extra = [head + ":" + p for ps in crate_tests.values() for p in ps]
    mandate_sources = sorted({head + ":" + m["source"] for m in MANDATES})
    specmap_spec = head + ":specmap.json"
    blobs = read_blobs(repo, sorted(set(wanted + extra + mandate_sources + [specmap_spec])))

    edges_by_file = collections.defaultdict(set)
    specmap_note = "absent at head"
    if blobs.get(specmap_spec):
        try:
            sm = json.loads(blobs[specmap_spec])
            for e in sm.get("edges", []):
                if e.get("file") and e.get("uri"):
                    edges_by_file[e["file"]].add(e["uri"])
            specmap_note = "%d edges over %d files" % (len(sm.get("edges", [])), len(edges_by_file))
        except ValueError:
            specmap_note = "unparseable at head"

    spec_docs = collections.defaultdict(list)  # PROP-NNN -> spec files at head
    module_specs = collections.defaultdict(list)  # crate name -> spec ids by directory
    for p in sorted(head_tree):
        if p.startswith("vibevm/vibespecs/"):
            m = SPEC_ID.match(posixpath.basename(p))
            if m:
                spec_docs[m.group(0)].append(p)
                parts = p.split("/")
                if parts[2] == "modules" and len(parts) > 4:
                    module_specs[parts[3]].append(m.group(0))

    # A reference counts only when its document exists at head: this drops the
    # placeholder URIs (`spec://a`, `spec://x/entry`) that test fixtures carry.
    known_docs = set(spec_docs) | {"VIBEVM-SPEC"}
    for p in head_tree:
        if "/vibespecs/" in "/" + p and p.endswith((".xml", ".md")) and "/fixtures/" not in p:
            stem = posixpath.basename(p).rsplit(".", 1)[0]
            m = SPEC_ID.match(stem)
            known_docs.add(m.group(0) if m else stem)

    for p, f in files.items():
        text = blobs.get(blob_spec(p)) or ""
        ids = {spec_id(u) for u in set(SPEC_URI.findall(text)) | set(edges_by_file.get(p, ()))}
        f["specs"] = sorted(i for i in ids if i in known_docs)
        f["unresolved"] = sorted(ids - known_docs)
        f["area"], f["kind"] = area_of(p), kind_of(p)
        f["inline_test"] = bool(INLINE_TEST.search(text)) if p.endswith(".rs") else False

    test_text = {a: "\n".join(blobs.get(head + ":" + p) or "" for p in ps) for a, ps in crate_tests.items()}

    def test_reference(path):
        """Return the first criterion that ties a source file to a test, or None."""
        f = files[path]
        if f["inline_test"]:
            return "inline"
        d, stem = posixpath.dirname(path), posixpath.basename(path)[:-3]
        siblings = [d + "/" + stem + "/tests.rs", d + "/" + stem + "_tests.rs", d + "/tests/" + stem + ".rs"]
        if stem == "mod":
            siblings.append(d + "/tests.rs")
        if any(s in head_tree for s in siblings):
            return "sibling"
        mod = module_stem(path)
        if mod and re.search(r"(?:\b%s::|::%s\b)" % (re.escape(mod), re.escape(mod)), test_text.get(f["area"], "")):
            return "named"
        if d + "/tests.rs" in head_tree:
            return "directory"
        return None

    # --- aggregate -------------------------------------------------------
    out = []
    w = out.append
    tot = collections.Counter(f["status"] for f in files.values())
    add = sum(f["add"] for f in files.values())
    rem = sum(f["del"] for f in files.values())
    w("# NST reconciliation: 1.0.0 to candidate")
    w("")
    w("Generated by `reconcile.py <repo> %s %s`. Do not edit by hand; rerun the script." % (base_arg, head_arg))
    w("")
    w("- Base: `%s`%s" % (base, " (tags: %s)" % ", ".join(tags) if tags else " (no tag points here)"))
    w("- Head: `%s`" % head)
    w("- Commits in range: %d" % commits)
    w("- Changed paths: **%d** (added %d, modified %d, deleted %d, renamed %d), **+%d / -%d** lines"
      % (len(files), tot["A"], tot["M"], tot["D"], tot["R"] + tot["C"], add, rem))
    w("- `specmap.json`: %s" % specmap_note)
    w("")
    w("Method. The inventory is `git diff --name-status -M base..head`; line counts come from "
      "`--numstat`. A file *has a spec reference* when its content (head revision; base revision for "
      "deleted files) contains a `spec://` URI or `specmap.json` at head records an edge for it, and "
      "the referenced document exists in a `vibespecs` tree at head. "
      "This is a net diff: work added and later reverted inside the range is not visible here.")
    w("")

    w("## Totals by kind")
    w("")
    kinds = collections.defaultdict(lambda: collections.Counter())
    for f in files.values():
        k = kinds[f["kind"]]
        k[f["status"] if f["status"] in "AMD" else "R"] += 1
        k["add"] += f["add"]
        k["del"] += f["del"]
        k["ref"] += bool(f["specs"])
    w("Kinds: `source` = Rust under `crates/*/src` or `xtask`; `test` = tests, fixtures, conformance and "
      "format corpora; `spec` = `vibevm/vibespecs/**` (boot lane counted as generated); `generated` = "
      "`vibedeps` copies, `vibe-wire/src/generated`, lock files, `specmap.json`.")
    w("")
    out += table(["Kind", "Files", "Added", "Modified", "Deleted", "Renamed", "+lines", "-lines", "With spec ref"],
                 [[k, v["A"] + v["M"] + v["D"] + v["R"], v["A"], v["M"], v["D"], v["R"], v["add"], v["del"], v["ref"]]
                  for k, v in sorted(kinds.items())])
    w("")

    w("## Totals by area")
    w("")
    areas = collections.defaultdict(lambda: collections.Counter())
    for f in files.values():
        k = areas[f["area"]]
        k[f["status"] if f["status"] in "AMD" else "R"] += 1
        k["add"] += f["add"]
        k["del"] += f["del"]
    out += table(["Area", "Files", "Added", "Modified", "Deleted", "Renamed", "+lines", "-lines"],
                 [[a, v["A"] + v["M"] + v["D"] + v["R"], v["A"], v["M"], v["D"], v["R"], v["add"], v["del"]]
                  for a, v in sorted(areas.items(), key=lambda kv: (-(kv[1]["A"] + kv[1]["M"] + kv[1]["D"] + kv[1]["R"]), kv[0]))])
    w("")

    # --- per crate -------------------------------------------------------
    unmatched = []
    weak = collections.Counter()
    crate_rows = []
    crate_unmatched = collections.Counter()
    crates = sorted(a for a in areas if a.startswith("crate:"))
    for a in crates:
        fs = {p: f for p, f in files.items() if f["area"] == a}
        src = [p for p, f in fs.items() if f["kind"] == "source" and f["status"] != "D"]
        with_ref = sum(1 for f in fs.values() if f["specs"])
        src_ref = sum(1 for p in src if files[p]["specs"])
        tests = sum(1 for f in fs.values() if f["kind"] == "test")
        ids = collections.Counter()
        for f in fs.values():
            ids.update(f["specs"])
        for p in sorted(src):
            if files[p]["specs"]:
                continue
            why = test_reference(p)
            if why is None:
                unmatched.append(p)
                crate_unmatched[a] += 1
            else:
                weak[why] += 1
        name = a.split(":", 1)[1]
        owned = sorted(set(module_specs.get(name, [])))
        top = ", ".join("%s (%d)" % kv for kv in sorted(ids.items(), key=lambda kv: (-kv[1], kv[0]))[:5]) or "none"
        crate_rows.append([name, len(fs), with_ref, len(fs) - with_ref, tests, len(src), src_ref,
                           len(src) - src_ref, crate_unmatched[a], ", ".join(owned) or "-", top])
    w("## Per crate")
    w("")
    w("`Module specs` are the documents under `vibevm/vibespecs/modules/<crate>/` at head. `Referenced specs` "
      "are the five documents most often referenced by the crate's changed files (file counts). Source "
      "columns count surviving (not deleted) Rust source files only.")
    w("")
    out += table(["Crate", "Files changed", "With spec ref", "Without", "Test files", "Source files",
                  "Source with ref", "Source without", "Unmatched", "Module specs", "Referenced specs"], crate_rows)
    w("")

    # --- spec documents --------------------------------------------------
    w("## Governing specifications referenced by changed files")
    w("")
    by_spec = collections.defaultdict(lambda: collections.Counter())
    for f in files.values():
        if f["kind"] in ("generated",) or f["area"].startswith("campaigns/"):
            continue
        for s in f["specs"]:
            by_spec[s][f["area"].replace("crate:", "")] += 1
    rows = []
    for s, c in sorted(by_spec.items(), key=lambda kv: (-sum(kv[1].values()), kv[0])):
        if not SPEC_ID.match(s):
            continue
        doc = ", ".join("`%s`" % d.replace("vibevm/vibespecs/", "") for d in spec_docs.get(s, [])) or "not under `vibevm/vibespecs` at head"
        status = "; ".join(sorted({files[d]["status"] for d in spec_docs.get(s, []) if d in files})) or "unchanged"
        top = ", ".join("%s (%d)" % kv for kv in sorted(c.items(), key=lambda kv: (-kv[1], kv[0]))[:6])
        rows.append([s, doc, status, sum(c.values()), top])
    out += table(["Spec", "Document", "Doc status in range", "Changed files referencing", "Areas (files)"], rows)
    w("")
    silent = sorted(d for s, ds in spec_docs.items() for d in ds
                    if d in files and s not in by_spec and s.startswith(("PROP", "FEAT")))
    w("Specification documents changed in range that no changed non-campaign file references: %d." % len(silent))
    for d in silent:
        w("- `%s` (%s)" % (d, files[d]["status"]))
    w("")

    # --- mandates --------------------------------------------------------
    w("## Owner mandates and implementing areas")
    w("")
    w("`NST-M9` is the campaign's own identifier; `R-NN` identifiers are assigned by this report. Areas are "
      "selected mechanically: changed non-generated files that reference one of the mandate's specs, plus "
      "changed files under its named paths. *At head* states whether the quote's key phrase is present in "
      "the cited source at the head revision.")
    w("")
    for m in MANDATES:
        present = m["needle"] in (blobs.get(head + ":" + m["source"]) or "")
        w("### %s (%s)" % (m["id"], m["date"]))
        w("")
        w("> %s" % m["quote"])
        w("")
        w("- Source: `%s`; at head: %s" % (m["source"], "yes" if present else
                                             "no (amendment not committed at this revision)"))
        if m.get("note"):
            w("- Note: %s" % m["note"])
        hit = collections.Counter()
        for p, f in files.items():
            if f["kind"] == "generated":
                continue
            if (set(f["specs"]) & set(m["specs"]) and not f["area"].startswith("campaigns/")) \
                    or any(p.startswith(x) for x in m["paths"]):
                hit[f["area"].replace("crate:", "")] += 1
        if m["specs"]:
            w("- Specs: %s" % ", ".join(m["specs"]))
        if hit:
            w("- Areas (changed files): %s" % ", ".join(
                "%s (%d)" % kv for kv in sorted(hit.items(), key=lambda kv: (-kv[1], kv[0]))))
        else:
            w("- Areas: none selected mechanically")
        w("")

    # --- unmatched -------------------------------------------------------
    total_src = sum(1 for f in files.values() if f["kind"] == "source" and f["status"] != "D")
    w("## Unmatched")
    w("")
    w("Changed, surviving Rust source files with no spec reference and no test reference. A test reference "
      "is any of: `inline` (the file has `#[cfg(test)]` or a test attribute), `sibling` (`<stem>/tests.rs`, "
      "`<stem>_tests.rs` or `tests/<stem>.rs` exists), `named` (a test file of the same crate names the "
      "module as a path segment), `directory` (a `tests.rs` sits in the same directory). `named` and "
      "`directory` are heuristics. Rows are listed, not investigated.")
    w("")
    w("- Changed surviving source files: %d" % total_src)
    w("- Without a spec reference: %d; of those tied to a test by: inline %d, sibling %d, named %d, directory %d"
      % (len(unmatched) + sum(weak.values()), weak["inline"], weak["sibling"], weak["named"], weak["directory"]))
    only_unresolved = sum(1 for f in files.values() if f["kind"] == "source" and f["status"] != "D"
                          and not f["specs"] and f["unresolved"])
    w("- Of the files without a spec reference, %d carry only `spec://` URIs whose document does not "
      "exist at head" % only_unresolved)
    w("- **Unmatched total: %d**%s" % (len(unmatched), " (first %d listed)" % UNMATCHED_CAP
                                         if len(unmatched) > UNMATCHED_CAP else ""))
    w("")
    out += table(["Crate", "Unmatched"], [[a.split(":", 1)[1], n] for a, n in
                                           sorted(crate_unmatched.items(), key=lambda kv: (-kv[1], kv[0]))])
    w("")
    for p in unmatched[:UNMATCHED_CAP]:
        w("- `%s` (%s, +%d/-%d)" % (p, files[p]["status"], files[p]["add"], files[p]["del"]))
    w("")
    sys.stdout.write("\n".join(out))


if __name__ == "__main__":
    main()
