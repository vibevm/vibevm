# -*- coding: utf-8 -*-
"""
PP-C3-dispose.py

Adds a 7th "решение" (decision) column to every assertion row of
campaigns/docs-2026-09/LEGACY-INVENTORY.md, per
campaigns/docs-2026-09/findings/PACKET-PP-C3.md.

stdlib only. Run with cwd = repository root (vibevm-docs); needs
target/debug/vibe.exe built (used read-only, --help only).
"""
import io
import os
import re
import subprocess
import sys
import xml.etree.ElementTree as ET
from collections import namedtuple, OrderedDict, Counter

# Windows consoles are commonly cp1252/cp866, which cannot encode this
# script's Cyrillic log/status output; widen stdout so the final summary
# print never crashes after the (already UTF-8) files have been written.
try:
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
except Exception:
    pass

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------

INVENTORY_PATH = "campaigns/docs-2026-09/LEGACY-INVENTORY.md"
PAGES_ROOT = "vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0/vibevm/vibespecs"
VIBE_EXE = os.path.join("target", "debug", "vibe.exe")
REPORT_PATH = "campaigns/docs-2026-09/findings/WORKER-REPORT-PP-C3.md"

OUT_INVENTORY_PATH = INVENTORY_PATH
OUT_REPORT_PATH = REPORT_PATH

PAGE_DIRS_PREFERRED = {"model", "howto", "reference"}
PAGE_DIRS_DEPRIORITIZED = {"glossary", "faq"}

SPLIT_RE = re.compile(r'(?<!\\)\|')
NS = "{https://vibevm.org/spec/1}"
EXCLUDED_TAGS = {"rule", "derived", "status"}
WORD_SHAPE_RE = re.compile(r'^[a-z][a-z0-9-]*$')
TOPIC_STOPWORDS = {"vibe", "a", "the", "and", "or", "for", "to", "of", "in", "on"}
NON_ALNUM_RE = re.compile(r'[^a-zA-Z0-9]+')


def strip_ns(tag):
    return tag[len(NS):] if tag.startswith(NS) else tag


# ---------------------------------------------------------------------------
# 1. Load inventory table
# ---------------------------------------------------------------------------

Row = namedtuple("Row", "idx file section assertion typ label checked")


def load_inventory():
    with io.open(INVENTORY_PATH, encoding="utf-8", newline="") as f:
        text = f.read()
    lines = text.split("\n")
    header_idx = None
    for i, line in enumerate(lines):
        if line.startswith("| файл | заголовок раздела |"):
            header_idx = i
            break
    if header_idx is None:
        raise SystemExit("main table header not found")
    sep_idx = header_idx + 1
    if not lines[sep_idx].startswith("| ---"):
        raise SystemExit("separator row not found where expected")
    rows = []
    i = sep_idx + 1
    while i < len(lines) and lines[i].startswith("|"):
        line = lines[i]
        cells = [c.strip() for c in SPLIT_RE.split(line)]
        if len(cells) != 8:
            raise SystemExit("line %d: expected 8 cells, got %d: %r" % (i, len(cells), line))
        _, fajl, section, assertion, typ, label, checked, _ = cells
        rows.append(Row(i, fajl, section, assertion, typ, label, checked))
        i += 1
    end_idx = i
    return lines, header_idx, sep_idx, rows, end_idx


# ---------------------------------------------------------------------------
# 2. Load the 44 documentation pages
# ---------------------------------------------------------------------------

Section = namedtuple("Section", "page anchor dir title text prose order")


def extract_text(elem, skip_tables=False):
    """Recursively flatten an element's visible text (attribute values plus
    .text/.tail), skipping `rule`/`derived`/`status` wholesale -- a `rule`
    is a bare cross-reference pointer and a `derived` block is generated
    content the source XML never spells out (its own `ref=` attribute, e.g.
    "vibe registry --help", would otherwise look like a literal page
    mention it is not) -- and skipping every `ref=` attribute defensively
    wherever else it appears. ElementTree already decodes XML entities, so
    "vibe &lt;command&gt; --help" in the source reaches us as literal
    "vibe <command> --help", matching how the assertion text spells it."""
    tag = strip_ns(elem.tag)
    if tag in EXCLUDED_TAGS:
        return ""
    if skip_tables and tag == "table":
        return ""
    parts = []
    for k, v in elem.attrib.items():
        if k == "ref":
            continue
        parts.append(v)
    if elem.text:
        parts.append(elem.text)
    for child in elem:
        parts.append(extract_text(child, skip_tables=skip_tables))
        if child.tail:
            parts.append(child.tail)
    return " ".join(p for p in parts if p)


def load_pages():
    """One Section per page anchor, in document order, "root" first. A
    child of <spec> starts a new named anchor iff it carries a `title=`
    attribute (matching page-index.md's own anchors); everything before
    the first such child -- the page's own <title id="root">, <status/>,
    and any intro <p> -- is the "root" anchor, matching the packet's
    '#root, if the match is in the first paragraph' rule."""
    pages = OrderedDict()
    xml_files = []
    for dirpath, _, filenames in os.walk(PAGES_ROOT):
        for fn in filenames:
            if fn.endswith(".xml"):
                xml_files.append(os.path.join(dirpath, fn))
    xml_files.sort()
    for full in xml_files:
        rel = os.path.relpath(full, PAGES_ROOT).replace("\\", "/")
        page_id = rel[:-4]
        dir_cat = page_id.split("/")[0]
        with io.open(full, encoding="utf-8") as f:
            raw = f.read()
        root = ET.fromstring(raw)
        root_children = []
        sections = []
        started = False
        for child in list(root):
            tag = strip_ns(child.tag)
            has_title_attr = "title" in child.attrib
            if not started and (tag in ("title", "status") or not has_title_attr):
                root_children.append(child)
                continue
            started = True
            if has_title_attr:
                txt = extract_text(child)
                prose = extract_text(child, skip_tables=True)
                sections.append(Section(page_id, tag, dir_cat, child.attrib.get("title", ""), txt, prose, len(sections)))
            else:
                # stray non-section element after sections began: fold into
                # the previous section's text (defensive; not expected to
                # trigger on the current 44 pages).
                if sections:
                    txt = extract_text(child)
                    prose = extract_text(child, skip_tables=True)
                    prev = sections[-1]
                    sections[-1] = prev._replace(text=prev.text + " " + txt, prose=prev.prose + " " + prose)
                else:
                    root_children.append(child)
        root_text = " ".join(extract_text(c) for c in root_children)
        root_prose = " ".join(extract_text(c, skip_tables=True) for c in root_children)
        all_sections = [Section(page_id, "root", dir_cat, "root", root_text, root_prose, -1)] + sections
        pages[page_id] = all_sections
    return pages


# ---------------------------------------------------------------------------
# 3. reference/commands.xml -> the 53 derived cli-help entries
# ---------------------------------------------------------------------------

CmdEntry = namedtuple("CmdEntry", "words cmd_str section")


def load_reference_commands():
    path = os.path.join(PAGES_ROOT, "reference", "commands.xml")
    with io.open(path, encoding="utf-8") as f:
        raw = f.read()
    root = ET.fromstring(raw)
    entries = []
    for child in root:
        tag = strip_ns(child.tag)
        if "title" not in child.attrib:
            continue
        section_anchor = tag
        for derived in child.iter():
            dtag = strip_ns(derived.tag)
            if dtag == "derived" and derived.attrib.get("kind") == "cli-help":
                ref = derived.attrib.get("ref", "")
                if ref.endswith(" --help"):
                    cmd_str = ref[: -len(" --help")]
                elif ref == "--help":
                    cmd_str = "vibe"
                else:
                    cmd_str = ref
                words = tuple(cmd_str.split())
                entries.append(CmdEntry(words, cmd_str, section_anchor))
    return entries


# ---------------------------------------------------------------------------
# 4. Search helpers
# ---------------------------------------------------------------------------

def find_matches_plain(key, pages):
    """One hit per page: the first (document-order) section containing
    `key`, as (page, anchor, dir, in_table, offset). A table row is a bare
    enumeration -- "mentioned", not "explained" -- so a section is searched
    in its prose first (table content stripped out) and only falls back to
    its full text (in_table=True) when the key occurs nowhere outside a
    table on that page. Within whichever text it was found in, an earlier
    offset is a proxy for "this is what the section is about" over "one
    example among many"."""
    hits = []
    if not key:
        return hits
    for page_id, sections in pages.items():
        for sec in sections:
            off = sec.prose.find(key)
            if off != -1:
                hits.append((page_id, sec.anchor, sec.dir, False, off))
                break
            off = sec.text.find(key)
            if off != -1:
                hits.append((page_id, sec.anchor, sec.dir, True, off))
                break
    return hits


_FLAG_RX_CACHE = {}


def flag_regex(flag):
    """Exact `--name` token: the literal flag text, not immediately
    followed by another identifier/hyphen character, so a search for
    `--json` does not match inside `--json-lines`."""
    rx = _FLAG_RX_CACHE.get(flag)
    if rx is None:
        rx = re.compile(re.escape(flag) + r'(?![A-Za-z0-9_-])')
        _FLAG_RX_CACHE[flag] = rx
    return rx


def find_matches_flag(flag, pages):
    rx = flag_regex(flag)
    hits = []
    for page_id, sections in pages.items():
        for sec in sections:
            m = rx.search(sec.prose)
            if m:
                hits.append((page_id, sec.anchor, sec.dir, False, m.start()))
                break
            m = rx.search(sec.text)
            if m:
                hits.append((page_id, sec.anchor, sec.dir, True, m.start()))
                break
    return hits


def dir_tier(dir_cat):
    if dir_cat in PAGE_DIRS_PREFERRED:
        return 0
    if dir_cat in PAGE_DIRS_DEPRIORITIZED:
        return 2
    return 1


def pick_best(hits, topic=None):
    """Choose one (page, anchor, dir, in_table, offset) among hits (already
    deduped to the first matching anchor per page). Returns (chosen,
    rejected_all, is_genuine_tie, tier_rejected) where is_genuine_tie is
    True only when more than one *distinct page* shares the winning (best)
    dir-tier -- i.e. the dir-tier preference alone did not already settle
    the choice. tier_rejected holds only the other pages that shared the
    winning tier (the real contenders), not every lower-tier hit.

    Ranking within a tier: not `edge-cases` (a caveat/aside anchor) beats
    `edge-cases`; a page whose own slug names one of `topic`'s words (e.g.
    row "vibe update" vs page slug "update-packages") beats one that
    doesn't; a prose hit beats a table-row hit (a table row is a bare
    enumeration -- "mentioned", not "explained"); then the earlier the
    match sits in the section's text, the more likely it is the section's
    actual subject rather than one example among many; then page id,
    alphabetically, as the final deterministic tiebreak. All of this is a
    heuristic proxy for "explained, not just mentioned" -- genuine ties
    still get logged (see `ambiguous_log` in main()) for human review."""
    if not hits:
        return None, [], False, []
    topic = topic or set()

    def sort_key(h):
        page_id, anchor, dir_cat, in_table, off = h
        slug_words = set(NON_ALNUM_RE.split(page_id.split("/")[-1]))
        slug_words = {w.lower() for w in slug_words if w}
        no_topic_overlap = 0 if (topic and slug_words & topic) else 1
        return (dir_tier(dir_cat), 1 if anchor == "edge-cases" else 0,
                no_topic_overlap, 1 if in_table else 0, off, page_id, anchor)

    ordered = sorted(hits, key=sort_key)
    chosen = ordered[0]
    best_tier = dir_tier(chosen[2])
    tier_peers = [h for h in ordered[1:] if dir_tier(h[2]) == best_tier]
    is_genuine_tie = len(tier_peers) > 0
    return chosen, ordered[1:], is_genuine_tie, tier_peers


def tokenize(assertion):
    return assertion.split()


def extract_full_chain(tokens):
    """'vibe' plus every following token that still looks like a bare
    subcommand word (lowercase, digits, hyphens) -- stops at the first
    flag, placeholder (`<name>`), or argument-shaped token."""
    chain = ["vibe"]
    for tok in tokens[1:]:
        if WORD_SHAPE_RE.match(tok):
            chain.append(tok)
        else:
            break
    return chain


def topic_words(row):
    """A small set of lowercase words this row is 'about', used only to
    prefer -- among pages already tied on dir-tier -- one whose own slug
    names the same concept (e.g. row 'vibe update' vs page slug
    'update-packages') over one that merely happens to contain the phrase
    somewhere in its prose. Purely a tiebreak signal, never a filter."""
    if row.typ == "cmd":
        tokens = tokenize(row.assertion)
        chain = extract_full_chain(tokens) if tokens and tokens[0] == "vibe" else []
        words = chain[1:]
    elif row.typ == "flag":
        words = row.assertion.lstrip("-").split("-")
    elif row.typ == "path":
        base = row.assertion.rstrip("/").split("/")[-1]
        base = re.sub(r'\.[a-zA-Z0-9]+$', '', base)
        words = NON_ALNUM_RE.split(base)
    else:  # section
        words = NON_ALNUM_RE.split(row.assertion.strip("[]"))
    return {w.lower() for w in words if w and len(w) > 2 and w.lower() not in TOPIC_STOPWORDS}


def make_matchers(known_by_len_desc):
    def known_prefix_match(tokens):
        """Longest known reference/commands entry that is an exact word-prefix
        of tokens. Used (a) as the exact-eligibility anchor for flag-ownership
        hints, where the result must itself be a runnable, real command whose
        --help text we hold, and (b) as the broadest fallback search key."""
        for entry in known_by_len_desc:
            n = len(entry.words)
            if n == 0:
                continue
            if tuple(tokens[:n]) == entry.words:
                return entry.cmd_str
        return "vibe"

    def search_key_for_cmd(tokens):
        # kept for build_hints: the single best *known, runnable* command guess
        return known_prefix_match(tokens)

    def candidate_search_keys(tokens):
        """Step-1 page search tries these, most specific first: the full
        shape-extracted subcommand chain, then each shorter prefix of it,
        then the longest *known* command prefix -- but never the bare word
        'vibe' alone, which would match essentially every page and defeat
        the point of an exact-occurrence check. A row that reduces to
        nothing more specific than bare 'vibe' gets no Step-1 candidates
        and falls through to Step 2 (reference) instead."""
        chain = extract_full_chain(tokens)
        keys = []
        for n in range(len(chain), 1, -1):
            k = " ".join(chain[:n])
            if k not in keys:
                keys.append(k)
        kp = known_prefix_match(tokens)
        if kp != "vibe" and kp not in keys:
            keys.append(kp)
        return keys

    return search_key_for_cmd, candidate_search_keys


# ---------------------------------------------------------------------------
# 5. Live vibe.exe probing (read-only: --help only, per the packet)
# ---------------------------------------------------------------------------

def run_help(words):
    try:
        proc = subprocess.run([VIBE_EXE] + list(words) + ["--help"],
                               capture_output=True, text=True, timeout=30)
        return proc.returncode, (proc.stdout or "") + (proc.stderr or "")
    except Exception:
        return -1, ""


def probe_help_exists(token):
    """The packet's `неизвестно` re-check: one `vibe.exe <token> --help`
    call, nothing else. Exit 0 means the token is a real subcommand."""
    if not token:
        return False, -1, ""
    code, text = run_help([token])
    return code == 0, code, text


_SUBCOMMAND_LINE_RE = re.compile(r'^  ([a-z][a-z0-9-]*)(?:\s|$)')


def parse_subcommand_names(help_text):
    """Pull child subcommand names out of a clap --help 'Commands:' block."""
    names = []
    in_block = False
    for line in help_text.splitlines():
        if line.strip() == "Commands:":
            in_block = True
            continue
        if not in_block:
            continue
        if line.strip() == "" or not line.startswith(" "):
            break  # blank line or a new top-level heading (e.g. "Options:") ends it
        m = _SUBCOMMAND_LINE_RE.match(line)
        if m and m.group(1) != "help":
            names.append(m.group(1))
    return names


# ---------------------------------------------------------------------------
# 6. Flag -> owning-command hints
# ---------------------------------------------------------------------------

def build_hints(rows, known_exact, search_key_for_cmd, all_help):
    """For each row, a best-effort ordered list of 'vibe ...' command
    strings that might be its owning command -- used only to decide which
    real --help text to grep a flag against, and only as a preference
    order (a miss here just falls through to checking every command)."""
    hints = {}
    by_file_section = {}
    for r in rows:
        by_file_section.setdefault((r.file, r.section), []).append(r)
    slug_re = re.compile(r'^docs/commands/([a-zA-Z0-9-]+)\.md$')
    for r in rows:
        candidates = []
        m = slug_re.match(r.file)
        if m:
            words = m.group(1).split("-")
            for n in range(len(words), 0, -1):
                cand = "vibe " + " ".join(words[:n])
                # try the full slug-derived command first (e.g. "vibe
                # registry add"), even when it is a child not itself
                # listed on reference/commands -- all_help now holds real
                # --help text for those too. Fall back to shorter
                # prefixes, and finally to whatever is a known top-level
                # entry, so there is always at least one concrete guess
                # when the slug matches nothing exactly.
                if (cand in all_help or cand in known_exact) and cand not in candidates:
                    candidates.append(cand)
                    break
        for sib in by_file_section.get((r.file, r.section), []):
            if sib.typ == "cmd":
                toks = tokenize(sib.assertion)
                if toks and toks[0] == "vibe":
                    key = search_key_for_cmd(toks)
                    if key not in candidates:
                        candidates.append(key)
        hints[r.idx] = candidates
    return hints


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    log_lines = []

    def log(*a):
        log_lines.append(" ".join(str(x) for x in a))

    lines, header_idx, sep_idx, rows, end_idx = load_inventory()
    log("Loaded %d assertion rows (lines %d..%d)" % (len(rows), sep_idx + 1, end_idx - 1))

    pages = load_pages()
    log("Loaded %d pages" % len(pages))

    known_commands = load_reference_commands()
    log("Loaded %d reference/commands derived entries" % len(known_commands))
    known_by_len_desc = sorted(known_commands, key=lambda e: -len(e.words))
    known_exact = {" ".join(e.words): e for e in known_commands}
    search_key_for_cmd, candidate_search_keys = make_matchers(known_by_len_desc)

    # Precompute real --help text for every known top-level/second-level
    # (reference/commands-listed) command, for the "справочник" (reference)
    # eligibility path and for flag ownership.
    all_help = {}
    for e in known_commands:
        code, text = run_help(list(e.words[1:]))  # drop leading 'vibe'
        all_help[e.cmd_str] = text
        if code != 0:
            log("WARNING: real --help failed for %r (exit %d)" % (e.cmd_str, code))
    log("Collected real --help text for %d commands" % len(all_help))

    # A flag documented only on a *third-level* subcommand (e.g. `--naming`
    # on `vibe registry add`) never appears in its listed parent's own
    # --help text (a parent's --help only *names* "add" as a subcommand; it
    # does not repeat "add"'s own flags). reference/commands itself only
    # ever derives the 53 listed commands, so such a flag can never earn
    # "перенесён (справочник)" through the page -- but the ORIGINAL P0-C4
    # inventory verified these against the live binary one level deeper, so
    # we go one level deeper here too, purely so a flag genuinely live on
    # the binary is not wrongly sent to "нужен автор". Attribution still
    # points at the *parent's* reference/commands section, since that is
    # the only place the page actually names the child command at all.
    child_commands = []
    for e in known_commands:
        for child in parse_subcommand_names(all_help.get(e.cmd_str, "")):
            child_words = e.words + (child,)
            child_cmd_str = e.cmd_str + " " + child
            child_code, child_text = run_help(list(child_words[1:]))
            all_help[child_cmd_str] = child_text
            child_commands.append(CmdEntry(child_words, child_cmd_str, e.section))
    log("Collected real --help text for %d additional (unlisted) child commands" % len(child_commands))
    # Combined lookup for "which reference/commands section explains this
    # command" -- top-level entries map to their own section, child
    # commands (not themselves listed on the page) map to their parent's.
    all_entries_by_cmd_str = OrderedDict()
    for e in known_commands + child_commands:
        all_entries_by_cmd_str.setdefault(e.cmd_str, e)

    hints = build_hints(rows, known_exact, search_key_for_cmd, all_help)

    # Tier-B (second-level subcommand covered by a listed parent's real --help)
    # cache, keyed by (parent_cmd_str, subword) -> bool
    tier_b_cache = {}

    def tier_b_covers(parent_entry, subword):
        key = (parent_entry.cmd_str, subword)
        if key not in tier_b_cache:
            help_text = all_help.get(parent_entry.cmd_str, "")
            # subcommand names appear as their own token in the Commands: list
            rx = re.compile(r'(?m)^\s*' + re.escape(subword) + r'\b')
            tier_b_cache[key] = bool(rx.search(help_text))
        return tier_b_cache[key]

    reeval_log = []       # неизвестно re-evaluations
    ambiguous_log = []    # multi-page candidate rows
    needs_author = []     # (row, reason)
    decisions = [None] * len(rows)
    categories = Counter()

    def build_reason(row):
        if row.typ == "cmd":
            return "команда `%s` не встречается ни на одной странице руководства и не покрыта справочником команд" % row.assertion
        if row.typ == "flag":
            return "флаг `%s` не встречается ни на одной странице и не подтверждён `--help` ни одной известной команды" % row.assertion
        if row.typ == "path":
            return "путь `%s` не встречается ни на одной странице руководства" % row.assertion
        return "поле/секция `%s` не встречается ни на одной странице руководства" % row.assertion

    for pos, row in enumerate(rows):
        label = row.label

        # ---- неизвестно re-check (cmd only; see PACKET-PP-C3.md) ----------
        if label == "неизвестно" and row.typ == "cmd":
            tokens = tokenize(row.assertion)
            probe_token = tokens[1] if len(tokens) > 1 else ""
            exists, code, _help_text = probe_help_exists(probe_token)
            reeval_log.append({"row": row, "probe": probe_token, "exists": exists, "code": code})
            if exists:
                label = "живо"
            else:
                decisions[pos] = "снят: устарело"
                categories["снят: устарело"] += 1
                continue

        # ---- Step 1: exact occurrence on a page --------------------------
        hits = []
        if row.typ == "cmd":
            tokens = tokenize(row.assertion)
            if tokens and tokens[0] == "vibe":
                # try the most specific subcommand chain first, broadening
                # only if nothing at all is found under it -- so "vibe mcp
                # install" is matched precisely where that phrase actually
                # occurs, and only falls back to the broader "vibe mcp" if
                # no page spells out the specific subcommand.
                for cand_key in candidate_search_keys(tokens):
                    hits = find_matches_plain(cand_key, pages)
                    if hits:
                        break
            else:
                hits = find_matches_plain(row.assertion, pages)
        elif row.typ == "flag":
            hits = find_matches_flag(row.assertion, pages)
        elif row.typ == "path":
            hits = find_matches_plain(row.assertion, pages)
            if not hits:
                base = row.assertion.rstrip("/").split("/")[-1]
                if base and base != row.assertion and len(base) >= 4:
                    hits = find_matches_plain(base, pages)
        else:  # section
            hits = find_matches_plain(row.assertion, pages)

        if hits:
            chosen, rejected, is_tie, tier_peers = pick_best(hits, topic_words(row))
            if is_tie:
                ambiguous_log.append({"row": row, "chosen": chosen, "peers": tier_peers})
            page_id, anchor = chosen[0], chosen[1]
            decisions[pos] = "перенесён → %s#%s" % (page_id, anchor)
            categories["перенесён"] += 1
            continue

        # ---- Step 2: справочник (cmd/flag, метка живо) --------------------
        if label == "живо" and row.typ in ("cmd", "flag"):
            if row.typ == "cmd":
                tokens = tokenize(row.assertion)
                if tokens and tokens[0] == "vibe":
                    chain = extract_full_chain(tokens)
                    chain_key = " ".join(chain)
                    if chain_key in known_exact:
                        entry = known_exact[chain_key]
                        decisions[pos] = "перенесён → reference/commands#%s" % entry.section
                        categories["перенесён (справочник)"] += 1
                        continue
                    if len(chain) > 1:
                        parent_key = " ".join(chain[:-1])
                        if parent_key in known_exact and tier_b_covers(known_exact[parent_key], chain[-1]):
                            entry = known_exact[parent_key]
                            decisions[pos] = "перенесён → reference/commands#%s" % entry.section
                            categories["перенесён (справочник)"] += 1
                            continue
            else:  # flag
                found = None
                if flag_regex(row.assertion).search(all_help.get("vibe", "")):
                    found = all_entries_by_cmd_str["vibe"]
                if found is None:
                    for cand in hints.get(row.idx, []):
                        if cand in all_help and flag_regex(row.assertion).search(all_help[cand]):
                            found = all_entries_by_cmd_str.get(cand)
                            if found:
                                break
                if found is None:
                    for e in all_entries_by_cmd_str.values():
                        if flag_regex(row.assertion).search(all_help.get(e.cmd_str, "")):
                            found = e
                            break
                if found is not None:
                    decisions[pos] = "перенесён → reference/commands#%s" % found.section
                    categories["перенесён (справочник)"] += 1
                    continue

        # ---- Step 3: label устарело ---------------------------------------
        if label == "устарело":
            decisions[pos] = "снят: устарело"
            categories["снят: устарело"] += 1
            continue

        # ---- Step 4: needs author ------------------------------------------
        reason = build_reason(row)
        decisions[pos] = "нужен автор: " + reason
        categories["нужен автор"] += 1
        needs_author.append((row, reason))

    assert all(d is not None for d in decisions), "every row must get a decision"

    # -----------------------------------------------------------------
    # Write the rewritten inventory: only the 7th column is added; every
    # other line (including the file-map table and summary further down)
    # is carried over byte-for-byte.
    # -----------------------------------------------------------------
    new_lines = list(lines)
    new_lines[header_idx] = lines[header_idx][:-1] + "| решение |"
    new_lines[sep_idx] = lines[sep_idx][:-1] + "| --- |"
    for row, decision in zip(rows, decisions):
        orig = lines[row.idx]
        decision_escaped = decision.replace("|", "\\|")
        new_lines[row.idx] = orig[:-1] + "| " + decision_escaped + " |"

    out_text = "\n".join(new_lines)
    with io.open(OUT_INVENTORY_PATH, "w", encoding="utf-8", newline="\n") as f:
        f.write(out_text)

    # -----------------------------------------------------------------
    # Write the worker report
    # -----------------------------------------------------------------
    rep = []
    rep.append("# WORKER-REPORT-PP-C3 — решения по инвентарю legacy-документации\n")
    rep.append("Пакет: `campaigns/docs-2026-09/findings/PACKET-PP-C3.md`. "
                "Скрипт: `campaigns/docs-2026-09/findings/PP-C3-dispose.py`.\n")
    rep.append("## Счётчики по решениям\n")
    rep.append("| решение | количество |")
    rep.append("| --- | --- |")
    for key in ("перенесён", "перенесён (справочник)", "снят: устарело", "нужен автор"):
        rep.append("| %s | %d |" % (key, categories.get(key, 0)))
    rep.append("| **всего** | **%d** |\n" % sum(categories.values()))

    rep.append("## Переоценки `неизвестно`\n")
    rep.append("Правило: `target/debug/vibe.exe <проба> --help`, только `--help`. "
                "%d строк с меткой `неизвестно` относятся к типу `cmd` и прошли переоценку; "
                "остальные `неизвестно` (тип `path`) идут по общему порядку без переоценки.\n" % len(reeval_log))
    rep.append("| строка | файл | утверждение | проба | exit code | результат |")
    rep.append("| --- | --- | --- | --- | --- | --- |")
    for e in reeval_log:
        r = e["row"]
        result = "живо (существует)" if e["exists"] else "снят: устарело (не существует)"
        rep.append("| %d | `%s` | `%s` | `%s` | %d | %s |" % (
            r.idx + 1, r.file, r.assertion, e["probe"], e["code"], result))
    rep.append("")

    rep.append("## Неоднозначный выбор страницы (несколько кандидатов)\n")
    rep.append("Строка попадает сюда, только когда несколько РАЗНЫХ страниц делят "
                "лучший (выигравший) уровень предпочтения — `model`/`howto`/`reference` "
                "= 0, прочие разделы = 1, `glossary`/`faq` = 2 — то есть уровень сам "
                "не решил выбор и потребовался вторичный тай-брейк (не `edge-cases`, "
                "затем алфавит по странице). Страницы, совпавшие только на худшем "
                "уровне, здесь не перечисляются — уровень уже их отклонил.\n")
    rep.append("Выбор — чистая функция (`тип`, `утверждение`): одно и то же утверждение "
                "даёт одно и то же неоднозначное множество страниц независимо от того, "
                "в каком файле `docs/…` оно встретилось. Ниже — уникальные ситуации, "
                "с числом затронутых строк инвентаря и их номерами.\n")
    if ambiguous_log:
        groups = OrderedDict()
        for a in ambiguous_log:
            r = a["row"]
            key = (r.typ, r.assertion, a["chosen"], tuple(sorted(a["peers"])))
            groups.setdefault(key, []).append(r)
        # most-affected situations first, then stable by assertion text
        ordered_groups = sorted(groups.items(), key=lambda kv: (-len(kv[1]), kv[0][1]))
        rep.append("%d уникальных неоднозначных (тип, утверждение) из %d затронутых строк.\n" % (
            len(ordered_groups), len(ambiguous_log)))
        rep.append("| тип | утверждение | выбрано | отклонённые кандидаты того же уровня | строк | номера строк |")
        rep.append("| --- | --- | --- | --- | --- | --- |")
        for (typ, assertion, chosen, peers), rs in ordered_groups:
            chosen_s = "%s#%s" % (chosen[0], chosen[1])
            rejected = "; ".join("%s#%s" % (h[0], h[1]) for h in peers)
            line_nos = sorted(r.idx + 1 for r in rs)
            if len(line_nos) > 8:
                shown = ", ".join(str(n) for n in line_nos[:8]) + ", …"
            else:
                shown = ", ".join(str(n) for n in line_nos)
            rep.append("| %s | `%s` | %s | %s | %d | %s |" % (
                typ, assertion, chosen_s, rejected, len(rs), shown))
    else:
        rep.append("(нет)")
    rep.append("")

    rep.append("## Строки «нужен автор», по файлу `docs/...`\n")
    by_file = OrderedDict()
    for r, reason in needs_author:
        by_file.setdefault(r.file, []).append((r, reason))
    for fname, rs in by_file.items():
        rep.append("### `%s` (%d)\n" % (fname, len(rs)))
        for r, reason in rs:
            rep.append("- строка %d, раздел «%s», тип `%s`, метка `%s`: `%s` — %s" % (
                r.idx + 1, r.section, r.typ, r.label, r.assertion, reason))
        rep.append("")

    with io.open(OUT_REPORT_PATH, "w", encoding="utf-8", newline="\n") as f:
        f.write("\n".join(rep))

    log("")
    log("Categories: %r" % dict(categories))
    log("неизвестно re-evaluations: %d" % len(reeval_log))
    log("ambiguous rows: %d" % len(ambiguous_log))
    log("needs-author rows: %d" % len(needs_author))
    log("Wrote inventory to: %s" % OUT_INVENTORY_PATH)
    log("Wrote report to: %s" % OUT_REPORT_PATH)
    print("\n".join(log_lines))
    print("DONE")


if __name__ == "__main__":
    main()
