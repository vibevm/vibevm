#!/usr/bin/env python3
"""Build the Phase D obligation registry from the Phase C verdict cache.

The 601 drift verdicts in `run/cache.json` are not 601 pieces of work, and they
are also not clustered by the text of their reasons: measured, only 16 reason
texts repeat at all, over 54 rows. Phase C wrote a bespoke reason per anchor.
What groups them is the SUBJECT — one document's drifts of one kind close in one
edit — so the registry keys on `(subject document, obligation type)` and merges
across documents only on evidence a script can defend: a reason text shared
almost verbatim (the copied-prose case, which is also the release-event
signature) or a finding id the workers themselves cited.

Nothing here is a judgement. Every obligation records which rule assigned its
type and which signal merged it, so the boss can re-read the assignment against
the reason and overrule it. The type a row reaches when no rule claims it is
reported separately from the types that were positively matched.

    python campaigns/packages-2026-09/tasks/drift-registry.py            # report
    python campaigns/packages-2026-09/tasks/drift-registry.py --write    # + files

Writes, under --write:
    run/state/obligations.json   the machine registry
    OBLIGATIONS.md               the human view
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from collections import Counter, defaultdict
from pathlib import Path

ZONE = "campaigns/packages-2026-09"
CACHE = "run/cache.json"
FINDINGS = "run/state/findings.json"
ROUTING = "run/state/routing.json"
OUT_JSON = "run/state/obligations.json"
OUT_MD = "OBLIGATIONS.md"

# The campaign plan mints finding ids in LOG prose as well as in findings.json;
# both spaces are read so a cluster can adopt an id that already exists rather
# than minting a duplicate.
PLAN = "spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md"

REF = re.compile(r"^[^\s:]+:\d+\s")
FID = re.compile(r"\bF-(\d{3})\b")

# ---------------------------------------------------------------- extraction


ROOT_HINT: list = []
ROUTING_MAP: list = []


def repo_root() -> Path:
    here = Path(__file__).resolve()
    for p in here.parents:
        if (p / "CLAUDE.md").exists() and (p / ZONE).exists():
            return p
    sys.exit("cannot locate the repository root from %s" % here)


def zone_of(path: str) -> str:
    if path.startswith("vibevm/vibepacks/org.vibevm.ai-native/"):
        return "ai-native"
    if path.startswith("vibevm/vibepacks/org.vibevm.world/"):
        return "world"
    return "host"


def package_of(path: str) -> str:
    parts = path.split("/")
    if path.startswith("packages/") and len(parts) >= 3:
        return "/".join(parts[:3])
    return parts[0]


def load_drifts(root: Path) -> list[dict]:
    cache = json.loads((root / ZONE / CACHE).read_text(encoding="utf-8"))
    rows = []
    for path, entry in sorted(cache["files"].items()):
        verdicts = (entry.get("campaign") or {}).get("verdicts") or {}
        for anchor, v in verdicts.items():
            if v.get("v") != "drift":
                continue
            ev = v.get("ev") or []
            refs = [e for e in ev if REF.match(e)]
            prose = [e for e in ev if not REF.match(e)]
            reason = prose[-1] if prose else ""
            rows.append(
                {
                    "file": path,
                    "anchor": anchor,
                    "zone": zone_of(path),
                    "package": package_of(path),
                    "src": v.get("src"),
                    "reason": reason,
                    "extra_prose": prose[:-1],
                    "refs": refs,
                    "ref_packages": sorted(
                        {package_of(r.split(":", 1)[0]) for r in refs}
                    ),
                    "cites": sorted({"F-" + m for m in FID.findall(" ".join(prose))}),
                }
            )
    return rows


def installed_copies(root: Path, files: set[str]) -> dict[str, dict]:
    """Where each shipped file is INSTALLED, and whether the copy still matches.

    `packages/<ns>/<pkg>/v<ver>/<rest>` installs as `vibedeps/<kind>-<pkg>/<ver>/<rest>`
    (`flow-`, `stack-`, `mcp-`). An edit to a package file does not reach the
    consumer until the package is published and re-vendored, which is exactly
    §5-D's release event; a copy that already differs proves the delivery step
    is real rather than notional.
    """
    import hashlib

    out: dict[str, dict] = {}
    for f in sorted(files):
        parts = f.split("/")
        if len(parts) < 5 or parts[0] != "packages":
            continue
        _, _ns, pkg, ver, *rest = parts
        ver = ver[1:] if ver.startswith("v") else ver
        tail = "/".join(rest)
        found = []
        for cand in sorted((root / "vibedeps").glob("*-%s/%s/%s" % (pkg, ver, tail))):
            same = hashlib.sha256(cand.read_bytes()).hexdigest() == (
                hashlib.sha256((root / f).read_bytes()).hexdigest()
                if (root / f).exists()
                else ""
            )
            found.append(
                {"path": cand.relative_to(root).as_posix(), "identical": same}
            )
        if found:
            out[f] = {"installed": found}
    return out


def routing(root: Path) -> dict[str, dict]:
    """Anchors a wave examined and deliberately did NOT repair in the package.

    §3.6 routes most of this corpus away from the package: the rule is sound and
    the consumer does not keep it, so the package must not move. Without a
    machine record of that determination the registry cannot converge — the
    anchor stays `drift` for ever, the next wave re-derives the same answer, and
    the exit gate cannot tell «not worked» from «worked, and the work belongs to
    the host». This file is that record, written by the boss at review time,
    never by a worker.
    """
    p = root / ZONE / ROUTING
    if not p.exists():
        return {}
    doc = json.loads(p.read_text(encoding="utf-8"))
    return {e["anchor"]: e for e in doc.get("entries", [])}


def known_finding_ids(root: Path) -> tuple[dict[str, dict], set[str]]:
    """Ids already spent: the wave-2 ledger, plus every id the plan's LOG mints."""
    ledger = {}
    fpath = root / ZONE / FINDINGS
    if fpath.exists():
        data = json.loads(fpath.read_text(encoding="utf-8"))
        for f in data.get("findings", []):
            ledger[f["id"]] = f
    spent = set(ledger)
    plan = (root / PLAN).read_text(encoding="utf-8", errors="replace")
    spent |= {"F-" + m for m in FID.findall(plan)}
    # wave 1's ledger shares the id space; its ids are cited by wave-2 prose.
    w1 = root / "campaigns/progress-2026-08/run/state/findings.json"
    if w1.exists():
        data = json.loads(w1.read_text(encoding="utf-8"))
        spent |= {f["id"] for f in data.get("findings", [])}
    return ledger, spent


# ------------------------------------------------------------ normalisation

LEAD = re.compile(
    r"^\s*(drift(\s+(on|at|in|for|,|:|;)[^.:]{0,80})?[.:,-]\s*)+", re.IGNORECASE
)
PUNCT = re.compile(r"[^a-z0-9]+")


def normalise(reason: str) -> str:
    s = reason.lower()
    s = LEAD.sub("", s)
    s = PUNCT.sub(" ", s)
    return " ".join(s.split())


def shingles(norm: str, n: int = 5) -> set[str]:
    w = norm.split()
    if len(w) < n:
        return {" ".join(w)} if w else set()
    return {" ".join(w[i : i + n]) for i in range(len(w) - n + 1)}


def jaccard(a: set[str], b: set[str]) -> float:
    if not a or not b:
        return 0.0
    return len(a & b) / len(a | b)


# ------------------------------------------------------------ type rule table
#
# Ordered; first match wins. Each entry is (type, rule-name, regex). The regex
# runs against the NORMALISED reason, so it is punctuation- and case-free.
#
# The split that decides the cost of a row:
#   missing-support  the fact DEPENDS ON something that does not exist  (absence)
#   reality-mismatch the fact DESCRIBES something that exists, wrongly  (discrepancy)
# reality-mismatch is the expensive class — it closes through sync-from-code
# with owner approval on every spec diff — so it is the fallback, never a guess:
# a row reaches it only after every absence rule has declined it.

RULES: list[tuple[str, str, str]] = [
    # ---- relocation: the content is right and its address is not -----------
    (
        "relocation",
        "r-dangling-flows",
        r"\b(69[ -]?dangling|flows family)\b|\bno spec flows\b|\bhas no spec flows\b"
        r"|\bspec flows\b.{0,40}\bdoes not\b|\bthe consuming project does not have\b",
    ),
    (
        "relocation",
        "r-path-does-not-resolve",
        r"\bthe path is wrong\b|\bpath does not resolve\b|\bdoes not resolve from\b"
        r"|\bfront door pointer that does not resolve\b"
        r"|\bthe home the row designates does not\b"
        r"|\bnames a location the host does not have\b",
    ),
    (
        "relocation",
        "r-wrong-home",
        r"\bthe placement is right and the name is wrong\b"
        r"|\bthe content exists and the home\b|\bwrong place to look\b"
        r"|\blives in\b.{0,60}\bnot in\b.{0,60}\bwhere (this|the) (fact|rule|claim)\b"
        r"|\blives only under packages\b|\blands on nothing\b"
        r"|\bthe target it names lives only\b",
    ),
    # ---- duplication: one norm, two writers --------------------------------
    (
        "duplication",
        "r-duplication",
        r"\bbyte identical copies\b|\bone fact many homes\b"
        r"|\bduplicat\w+\b.{0,80}\b(live|two|three|copies|no reconciliation)\b"
        r"|\bstated in six host locations\b|\bduplicates this package s\b"
        r"|\btwo writers for one statement\b|\bduplicate anchor\b",
    ),
    # ---- terminology: the same thing under two names -----------------------
    (
        "terminology",
        "r-terminology",
        r"\bthe name is wrong\b|\bcalls it\b.{0,40}\bthe host calls\b"
        r"|\bvocabulary (differs|diverges|is different)\b"
        r"|\bnames? (it|them) \w+ (and|while) the (host|code|tree) names?\b"
        r"|\bthe term\b.{0,60}\bis used\b.{0,60}\bnot\b",
    ),
    # ---- contradiction: two written statements disagree --------------------
    (
        "contradiction",
        "r-contradiction",
        r"\bcontradict\w*\b|\bsays the opposite\b|\bthe opposite of the direction\b"
        r"|\bcollision of principle\b|\bfalsified by (the|its own)\b"
        r"|\bbroken by (an entry of )?this very\b|\bdisagree\w*\b"
        r"|\breverses (it|this|that) in writing\b|\bself falsifying\b"
        r"|\bthe document s own\b.{0,60}\b(says|treats|records)\b.{0,60}\bwhile\b",
    ),
    # ---- missing-support: the support the claim needs is absent ------------
    (
        "missing-support",
        "r-no-checker",
        r"\bno checker\b|\bwith no checker\b|\bnothing (enforces|checks|computes|"
        r"catches|schedules|selects|resolves|reconciles|expires|records)\b"
        r"|\bis a wish\b|\ba defect nothing can detect\b"
        r"|\bnothing that could catch\b|\bno selector\b|\bno gate\b"
        r"|\bhas fired zero times\b|\bnothing (auto|carries closed set)\b",
    ),
    (
        "missing-support",
        "r-nothing-exists",
        r"\bno \w+(\s+\w+){0,3}\s+exists?\b|\bdoes not exist\b|\bexists? nowhere\b"
        r"|\bis unbuilt\b|\bare unbuilt\b|\bnever built\b|\bno implementation\b"
        r"|\bnothing implements\b|\bhas no (producer|generator|machine|reader|"
        r"implementation|tool|checker)\b|\bno such (file|field|type|directory)\b"
        r"|\bare not authored anywhere\b|\bauthored nowhere\b|\bis absent\b"
        r"|\bare absent\b|\bno file named\b|\bnot implemented\b|\bunimplemented\b"
        r"|\bis not installed\b|\bare not installed\b|\bshipped in none\b"
        r"|\boccurs zero times\b|\bappears nowhere\b|\bexists anywhere\b",
    ),
    (
        "missing-support",
        "r-unimplemented-half",
        r"\bauthored on one side and unimplemented on the other\b"
        r"|\bhalf a mechanism\b|\bthe half that is true\b.{0,60}\bthe half\b"
        r"|\bspecified and never built\b|\bhas no machine\b"
        r"|\bspecified in two props and shipped in none\b",
    ),
    (
        "missing-support",
        "r-zero-instances",
        r"\bno live instance\b|\bzero live\b|\b0 live\b|\bnone to pin with\b"
        r"|\bhas never fired\b|\bsat unfired\b|\bnever fired\b"
        r"|\bhas none either\b|\bhas none\b|\bthere are none\b|\bnone of the\b"
        r"|\bnothing to pin\b",
    ),
]

RULES_C = [(t, n, re.compile(p)) for t, n, p in RULES]

CODE_EXT = (
    ".rs", ".ts", ".tsx", ".go", ".js", ".py", ".toml", ".json", ".sh",
    ".ps1", ".yml", ".yaml",
)


def fact_sentence(o: dict) -> str:
    """The subject fact's own line, which Phase C put first in every `ev`."""
    subjects = set(o["files"])
    for ref in o["evidence_refs"]:
        head = ref.split(":", 1)[0]
        if head in subjects and "##" in ref:
            return ref
    return o["evidence_refs"][0] if o["evidence_refs"] else ""


def falsifier(o: dict) -> str:
    """Which side of the package boundary the falsifying evidence sits on.

    This decides one thing and refuses to decide another. It decides that a
    `self` obligation — every falsifying ref inside the subject package or its
    own install slot — is the package being wrong ABOUT ITSELF, which is always
    a package edit and always the boss's. It does **not** decide `host` or
    `mixed`: there the package may be right and the consumer not complying, and
    weakening a shipped rule to match a lax consumer is the profanation the
    mandate names. That call is made per obligation at closure time, not by a
    regex over prose.

    `campaigns/` is neither side: a harvest file is this campaign's own capture
    of a run, not a fact about the consumer's behaviour.
    """
    subject_pkgs = tuple(o["packages"])
    host = pkg = False
    for ref in o["evidence_refs"]:
        head = ref.split(":", 1)[0]
        if head.startswith("campaigns/"):
            continue
        # package-side is the closed set; everything else is the consumer. A
        # prefix list of host directories would silently mis-file whatever it
        # forgot — `discipline/AUDIT.md` fell through it and read as `self`.
        if head.startswith(subject_pkgs) or head.startswith("vibedeps/"):
            pkg = True
        else:
            host = True
    if host and pkg:
        return "mixed"
    if host:
        return "host"
    return "self"


def closure_route(o: dict) -> str:
    """How this obligation can be closed — which decides who must approve it.

    The routes are ordered by cost, and the first that applies wins:

      release          spans a package boundary; a published version and a
                       re-vendor, never an edit in one consumer (§5-D).
                       **Owner approves before publication.**
      build-or-demote  the support is absent: build the mechanism (a Phase E
                       DRIFT task) or demote the claim. The choice is a
                       judgement, and demoting is itself a spec diff.
      sync-from-code   the evidence reaches code, so the repair is a spec diff
                       against what the code does. **Owner approves each diff.**
      prose-edit       the repair is entirely inside prose — a path, a roster,
                       a count, a name. Routine.
    """
    if o["cross_package"]:
        return "release"
    if o["type"] == "missing-support":
        return "build-or-demote"
    for ref in o["evidence_refs"]:
        head = ref.split(":", 1)[0]
        if head.endswith(CODE_EXT):
            return "sync-from-code"
    return "prose-edit"


def classify(norm: str) -> tuple[str, str]:
    for t, name, rx in RULES_C:
        if rx.search(norm):
            return t, name
    if norm:
        # NOT a match — the residue. A drift that is neither an absence nor a
        # written contradiction is a description of something that exists,
        # described wrongly, which is what `reality-mismatch` names. The rule
        # name says «default» so the honest count is visible in the report.
        return "reality-mismatch", "r-default-described-wrongly"
    return "unclassified", "r-no-reason-text"


# ------------------------------------------------------------------ clustering


class DSU:
    def __init__(self, n: int) -> None:
        self.p = list(range(n))

    def find(self, x: int) -> int:
        while self.p[x] != x:
            self.p[x] = self.p[self.p[x]]
            x = self.p[x]
        return x

    def union(self, a: int, b: int) -> None:
        ra, rb = self.find(a), self.find(b)
        if ra != rb:
            self.p[max(ra, rb)] = min(ra, rb)


def cluster(rows: list[dict], threshold: float) -> list[list[int]]:
    """`(subject document, type)` as the key, then two cross-document merges.

    The primary key is the pair a single edit pass closes: one document, one
    kind of defect. Two signals merge across documents, and both are things a
    script can prove rather than infer:

      * **shared reason text** — Jaccard >= threshold over 5-gram shingles.
        One reason falsifying anchors in two packages is prose copied between
        packages, which is precisely the release-event shape.
      * **a cited finding id** — a Phase C worker wrote `F-NNN` into the reason
        to say «this is that family». Two rows citing the same id are one
        family by the author's own statement.
    """
    norms = [normalise(r["reason"]) for r in rows]
    sh = [shingles(n) for n in norms]
    dsu = DSU(len(rows))

    # PASS 1 — the cross-document family. Run FIRST and on its own: if the
    # `(file, type)` key ran first, one shared anchor would chain two whole
    # documents' worth of unrelated defects into a single obligation through
    # it. A row that lands in a family of two or more belongs to the family;
    # `(file, type)` then groups only what no family claimed.

    # blocking: only compare rows sharing at least one shingle, so the O(n^2)
    # pass is over a small candidate set rather than the whole corpus.
    inverted: dict[str, list[int]] = defaultdict(list)
    for i, s in enumerate(sh):
        for g in s:
            inverted[g].append(i)
    candidates: set[tuple[int, int]] = set()
    for g, members in inverted.items():
        if len(members) > 60:  # boilerplate shingle, carries no signal
            continue
        for a_i, a in enumerate(members):
            for b in members[a_i + 1 :]:
                candidates.add((a, b))
    for a, b in sorted(candidates):
        if dsu.find(a) == dsu.find(b):
            continue
        j = jaccard(sh[a], sh[b])
        if j >= threshold:
            dsu.union(a, b)
            rows[b].setdefault("merged_by", "reason-text j=%.2f" % j)

    by_fid: dict[str, int] = {}
    for i, r in enumerate(rows):
        for fid in r["cites"]:
            if fid in by_fid:
                if dsu.find(by_fid[fid]) != dsu.find(i):
                    dsu.union(by_fid[fid], i)
                    rows[i].setdefault("merged_by", "cites %s" % fid)
            else:
                by_fid[fid] = i

    # the parallel corpus: one fact projected per language or per flow carries
    # the SAME anchor id in each package. Sixteen anchors drift in more than one
    # package; merging them is what keeps a repair from landing in one family
    # member and not its siblings.
    pkgs_of_anchor: dict[str, set[str]] = defaultdict(set)
    for r in rows:
        pkgs_of_anchor[r["anchor"]].add(r["package"])
    by_anchor: dict[str, int] = {}
    for i, r in enumerate(rows):
        if len(pkgs_of_anchor[r["anchor"]]) < 2:
            continue
        a = r["anchor"]
        if a in by_anchor:
            if dsu.find(by_anchor[a]) != dsu.find(i):
                dsu.union(by_anchor[a], i)
                rows[i].setdefault("merged_by", "shared anchor #%s" % a)
        else:
            by_anchor[a] = i

    # PASS 2 — everything no family claimed groups by `(file, type)`: one
    # document, one kind of defect, one edit pass.
    fam_size: Counter = Counter(dsu.find(i) for i in range(len(rows)))
    by_key: dict[tuple[str, str], int] = {}
    for i, r in enumerate(rows):
        if fam_size[dsu.find(i)] > 1:
            continue
        key = (r["file"], r["type"])
        if key in by_key:
            dsu.union(by_key[key], i)
            rows[i]["merged_by"] = "file+type"
        else:
            by_key[key] = i

    groups: dict[int, list[int]] = defaultdict(list)
    for i in range(len(rows)):
        groups[dsu.find(i)].append(i)
    return [sorted(v) for _, v in sorted(groups.items())]


# ------------------------------------------------------------------- assembly


def carry_ids(obligations: list[dict], prior_path: Path) -> tuple[list[dict], set[str]]:
    """Reuse the id a cluster carried last run, matched by its ANCHOR SET.

    Ids must survive regeneration or the ledger is worthless: a closure removes
    drift rows, the sort order changes, and a positional id would silently
    rename every obligation after it. Matching is greedy by Jaccard over the
    anchor sets — a cluster that loses anchors to a closure is still the same
    obligation — and a prior row with no match at all has been fully closed, so
    it is returned as history rather than dropped.
    """
    if not prior_path.exists():
        return [], set()
    prior_doc = json.loads(prior_path.read_text(encoding="utf-8"))
    prior = prior_doc.get("obligations", [])
    # History accumulates: an entry leaves the registry only by changing
    # disposition, never by deletion. Recomputing `resolved` from the prior
    # generation alone dropped every obligation closed two generations back.
    history = {h["id"]: h for h in prior_doc.get("resolved", [])}
    pairs = []
    for pi, p in enumerate(prior):
        pa = set(p.get("anchors") or [])
        for ci, c in enumerate(obligations):
            ca = set(c["anchors"])
            inter = len(pa & ca)
            if inter:
                # CONTAINMENT, not symmetric overlap. A partial closure shrinks
                # a cluster — two of F-205's three anchors were re-judged and
                # symmetric Jaccard scored the remainder at 0.33, so it was
                # minted a fresh id and the obligation was filed `resolved`
                # while one of its anchors still drifted. Scoring against the
                # SMALLER set makes a shrunk cluster still itself.
                pairs.append((inter / min(len(pa), len(ca)), inter, pi, ci))
    pairs.sort(reverse=True)
    taken_p, taken_c = set(), set()
    for j, _inter, pi, ci in pairs:
        if j < 0.5 or pi in taken_p or ci in taken_c:
            continue
        taken_p.add(pi)
        taken_c.add(ci)
        obligations[ci]["id"] = prior[pi]["id"]
        # provenance is kept as first assigned; `carried` is the separate fact
        # that this run recognised the cluster rather than re-minting it.
        obligations[ci]["id_source"] = prior[pi].get("id_source", "minted")
        obligations[ci]["carried"] = True
        obligations[ci]["status"] = prior[pi].get("status", "open")
        obligations[ci]["wave"] = prior[pi].get("wave", 1)
    live = {o["id"] for o in obligations if o.get("id")}
    for pi, p in enumerate(prior):
        if pi in taken_p:
            continue
        p = dict(p)
        p["status"] = "resolved"
        for k in ("reasons", "evidence_refs", "installed_copies"):
            p.pop(k, None)
        history[p["id"]] = p
    # an id that came BACK (a closure reverted) leaves history and is live again
    for i in list(history):
        if i in live:
            del history[i]
    closed = [history[i] for i in sorted(history)]
    return closed, {p["id"] for p in prior} | set(history)


def build(rows, groups, spent: set[str], ledger: dict, vendored: dict):
    obligations = []
    next_free = 130
    while "F-%03d" % next_free in spent:
        next_free += 1

    # the fingerprint that makes a finding a RELEASE EVENT: the same reason
    # text falsifying anchors in more than one package.
    for g in groups:
        members = [rows[i] for i in g]
        packages = sorted({m["package"] for m in members})
        files = sorted({m["file"] for m in members})
        cites = sorted({c for m in members for c in m["cites"]})
        # a cluster's evidence may reach outside the package it lives in
        ref_pkgs = sorted({p for m in members for p in m["ref_packages"]})
        typ = Counter(m["type"] for m in members).most_common(1)[0][0]
        rule = Counter(
            m["rule"] for m in members if m["type"] == typ
        ).most_common(1)[0][0]
        merged = sorted({m["merged_by"] for m in members if m.get("merged_by")})
        inst = [c for f in files for c in vendored.get(f, {}).get("installed", [])]
        obligations.append(
            {
                "type": typ,
                "rule": rule,
                "merged_by": merged or ["single row"],
                "packages": packages,
                "files": files,
                "anchors": ["%s#%s" % (m["file"], m["anchor"]) for m in members],
                "drift_count": len(members),
                "cross_package": len(packages) > 1,
                "installed": bool(inst),
                "installed_differs": any(not c["identical"] for c in inst),
                "release_event": len(packages) > 1,
                "evidence_spans": ref_pkgs,
                "evidence_refs": sorted({r for m in members for r in m["refs"]}),
                "cites": cites,
                "reason": members[0]["reason"],
                "reasons": [m["reason"] for m in members[1:]],
                "status": "open",
                "wave": 1,
            }
        )

    routed = ROUTING_MAP[0] if ROUTING_MAP else {}
    for o in obligations:
        o["closure_route"] = closure_route(o)
        o["fact"] = fact_sentence(o)
        o["falsifier"] = falsifier(o)
        out = [a for a in o["anchors"] if a in routed]
        o["routed_out"] = out
        o["owed"] = o["drift_count"] - len(out)
        if out:
            o["routed_to"] = sorted({routed[a]["route"] for a in out})

    # biggest first: a cluster's size is how many verdicts one closure clears.
    obligations.sort(key=lambda o: (-o["drift_count"], o["packages"], o["type"]))

    closed, prior_ids = carry_ids(obligations, ROOT_HINT[0] / ZONE / OUT_JSON)
    spent |= prior_ids

    adopted: set[str] = set()
    for o in obligations:
        if o.get("id"):  # carried from the prior registry — never re-minted
            adopted.add(o["id"])
            continue
        adopt = [c for c in o["cites"] if c in spent and c not in adopted]
        if len(adopt) == 1:
            o["id"] = adopt[0]
            o["id_source"] = (
                "adopted (wave-2 ledger)" if adopt[0] in ledger else "adopted (plan LOG)"
            )
            adopted.add(adopt[0])
        else:
            o["id"] = "F-%03d" % next_free
            o["id_source"] = "minted"
            spent.add(o["id"])
            next_free += 1
            while "F-%03d" % next_free in spent:
                next_free += 1
    return obligations, closed


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--threshold", type=float, default=0.65)
    ap.add_argument("--write", action="store_true")
    ap.add_argument("--show", default=None, help="print rows of one type")
    ap.add_argument("--task", default=None,
                    help="print the SPEC-task §2 table for one obligation id")
    args = ap.parse_args()

    root = repo_root()
    ROOT_HINT.append(root)
    rows = load_drifts(root)
    for r in rows:
        r["type"], r["rule"] = classify(normalise(r["reason"]))
    ROUTING_MAP.append(routing(root))
    ledger, spent = known_finding_ids(root)
    vendored = installed_copies(root, {r["file"] for r in rows})
    groups = cluster(rows, args.threshold)
    obligations, closed = build(rows, groups, spent, ledger, vendored)

    if args.task:
        return emit_task(obligations, args.task)

    print("drift verdicts read     : %d" % len(rows))
    print("clusters (obligations)  : %d" % len(obligations))
    print("reduction               : %.1f drifts per obligation" % (len(rows) / len(obligations)))
    print()
    by_type = Counter(o["type"] for o in obligations)
    rows_by_type = Counter()
    for o in obligations:
        rows_by_type[o["type"]] += o["drift_count"]
    print("%-18s %8s %8s" % ("type", "oblig", "drifts"))
    for t, _ in by_type.most_common():
        print("%-18s %8d %8d" % (t, by_type[t], rows_by_type[t]))
    print("%-18s %8d %8d" % ("TOTAL", len(obligations), sum(rows_by_type.values())))
    print()
    xp = [o for o in obligations if o["release_event"]]
    print("release events (span >1 package): %d obligations, %d drifts"
          % (len(xp), sum(o["drift_count"] for o in xp)))
    print("subject file is installed under vibedeps/ : %d obligations "
          "(of which %d already differ from the installed copy)"
          % (sum(1 for o in obligations if o["installed"]),
             sum(1 for o in obligations if o["installed_differs"])))
    print("ids: %d carried from the prior registry, %d newly assigned · "
          "%d obligation(s) RESOLVED and moved to history"
          % (sum(1 for o in obligations if o.get("carried")),
             sum(1 for o in obligations if not o.get("carried")),
             len(closed)))
    print()
    print("%-18s %8s %8s   %s" % ("closure route", "oblig", "drifts", "who approves"))
    who = {
        "release": "OWNER, before publication",
        "sync-from-code": "OWNER, on every spec diff",
        "build-or-demote": "owner only where the choice is to demote",
        "prose-edit": "boss",
    }
    for r in ("release", "sync-from-code", "build-or-demote", "prose-edit"):
        sel = [o for o in obligations if o["closure_route"] == r]
        print("%-18s %8d %8d   %s"
              % (r, len(sel), sum(o["drift_count"] for o in sel), who[r]))
    print()
    fully = [o for o in obligations if o["owed"] == 0]
    part = [o for o in obligations if o["routed_out"] and o["owed"]]
    owed_v = sum(o["owed"] for o in obligations)
    print("CONVERGENCE - what the exit gate measures")
    print("  obligations with nothing left owed to the package : %4d" % len(fully))
    print("  obligations partly routed out                     : %4d" % len(part))
    print("  drift verdicts still owed a package repair        : %4d of %d"
          % (owed_v, len(rows)))
    print("  routed out of the package (route b / owner)       : %4d"
          % (len(rows) - owed_v))
    print()
    print("WHERE THE FALSIFYING EVIDENCE SITS - the question 'which side is")
    print("wrong' is a judgement, and this settles only the half a script can:")
    for f, note in (
        ("self", "package wrong about ITSELF -> package edit, boss, unambiguous"),
        ("host", "the consumer falsifies it -> which side is wrong is a judgement"),
        ("mixed", "both cited -> which side is wrong is a judgement"),
    ):
        sel = [o for o in obligations if o["falsifier"] == f]
        print("  %-6s %4d obligations %4d drifts   %s"
              % (f, len(sel), sum(o["drift_count"] for o in sel), note))
    print()
    print("rule usage:")
    for r, c in Counter(o["rule"] for o in obligations).most_common():
        print("  %-34s %4d obligations, %4d drifts"
              % (r, c, sum(o["drift_count"] for o in obligations if o["rule"] == r)))

    if args.show:
        print()
        for o in obligations:
            if o["type"] == args.show:
                print("--- %s  %s  n=%d  %s" % (o["id"], o["rule"], o["drift_count"],
                                                ",".join(o["packages"])))
                print("    " + o["reason"][:400].replace("\n", " "))

    if args.write:
        out = {
            "schema": 1,
            "phase": "D",
            "generated_by": "campaigns/packages-2026-09/tasks/drift-registry.py",
            "generated_from": "%s/%s" % (ZONE, CACHE),
            "threshold": args.threshold,
            "drift_verdicts": len(rows),
            "obligations": obligations,
            "resolved": closed,
        }
        p = root / ZONE / OUT_JSON
        p.write_text(json.dumps(out, ensure_ascii=False, indent=1) + "\n",
                     encoding="utf-8")
        print("\nwrote %s" % p)
        md = render_md(obligations, len(rows), args.threshold)
        p2 = root / ZONE / OUT_MD
        p2.write_text(md, encoding="utf-8")
        print("wrote %s" % p2)
    return 0


def emit_task(obligations: list[dict], oid: str) -> int:
    """The §2 table of a SPEC task, from the registry rather than by hand.

    The template's row is `| id | from | to | type |`. For a wave-2 obligation
    **`from` is the package anchor that is false and `to` is what falsifies
    it** — the host artefact, the code, or the sibling document the evidence
    names. One row per drifting anchor, so a task's §2 is the list of verdicts
    its closure must clear.
    """
    match = [o for o in obligations if o["id"] == oid]
    if not match:
        print("no obligation %s in the registry" % oid)
        return 1
    o = match[0]
    print("**Wave:** %d · **type:** `%s` · **route:** `%s` · %d drift verdicts"
          % (o["wave"], o["type"], o["closure_route"], o["drift_count"]))
    if o["release_event"]:
        print("**RELEASE EVENT** — spans %d packages; closed by a published "
              "version and `cargo xtask sync-engines`, never by an edit in one "
              "consumer." % len(o["packages"]))
    print()
    print("| id | from | to | type |")
    print("|---|---|---|---|")
    tos = [r.split(":", 1)[0] for r in o["evidence_refs"]]
    subject_files = set(o["files"])
    tos = sorted({t for t in tos if t not in subject_files}) or sorted(subject_files)
    to = tos[0] if len(tos) == 1 else "%s (+%d more)" % (tos[0], len(tos) - 1)
    for a in o["anchors"]:
        f, anchor = a.rsplit("#", 1)
        print("| %s | `%s#%s` | `%s` | %s |" % (o["id"], f, anchor, to, o["type"]))
    print()
    print("Reason, verbatim from the verdict that opened it:\n")
    print("> " + o["reason"].replace("\n", "\n> "))
    if o["reasons"]:
        print("\nThe other %d verdicts in this obligation carry their own reasons; "
              "read them in `run/state/obligations.json`." % len(o["reasons"]))
    return 0


def render_md(obligations: list[dict], n_drifts: int, threshold: float) -> str:
    out = []
    out.append("# Phase D — the obligation registry {#root}\n")
    out.append(
        "_Generated by [`tasks/drift-registry.py`](tasks/drift-registry.py) from\n"
        "`run/cache.json`. Do not hand-edit: regenerate. The `status` of a row is\n"
        "carried in [`run/state/obligations.json`](run/state/obligations.json), which\n"
        "is what a closure updates._\n"
    )
    out.append("**%d drift verdicts → %d obligations** (%.1f drifts each), clustered at "
               "Jaccard ≥ %.2f over 5-gram shingles of the verdict reason.\n"
               % (n_drifts, len(obligations), n_drifts / len(obligations), threshold))
    by_type: dict[str, list[dict]] = defaultdict(list)
    for o in obligations:
        by_type[o["type"]].append(o)
    out.append("| type | obligations | drifts |")
    out.append("|---|---:|---:|")
    for t in sorted(by_type, key=lambda t: -sum(o["drift_count"] for o in by_type[t])):
        out.append("| `%s` | %d | %d |"
                   % (t, len(by_type[t]), sum(o["drift_count"] for o in by_type[t])))
    out.append("| **total** | **%d** | **%d** |\n" % (len(obligations), n_drifts))

    owed = sum(o["owed"] for o in obligations)
    fully = sum(1 for o in obligations if o["owed"] == 0)
    out.append("**Convergence.** %d drift verdicts still owe the PACKAGE a repair; "
               "**%d have been examined and routed out** — §3.6 route (b), the rule "
               "is sound and the consumer does not keep it, so the host owes the "
               "work. %d obligations have nothing left owed to the package and "
               "survive only as host obligations. The record is "
               "[`run/state/routing.json`](run/state/routing.json), written by the "
               "boss at review time.\n"
               % (owed, n_drifts - owed, fully))
    out.append("| closure route | obligations | drifts | who approves |")
    out.append("|---|---:|---:|---|")
    who = {
        "release": "**owner**, before publication",
        "sync-from-code": "**owner**, on every spec diff",
        "build-or-demote": "owner only where the choice is to demote",
        "prose-edit": "boss",
    }
    for r in ("release", "sync-from-code", "build-or-demote", "prose-edit"):
        sel = [o for o in obligations if o["closure_route"] == r]
        out.append("| `%s` | %d | %d | %s |"
                   % (r, len(sel), sum(o["drift_count"] for o in sel), who[r]))
    out.append("")

    for t in sorted(by_type, key=lambda t: -sum(o["drift_count"] for o in by_type[t])):
        out.append("## `%s` {#%s}\n" % (t, t))
        out.append("| id | n | route | packages | rule | reason (first line) |")
        out.append("|---|---:|---|---|---|---|")
        for o in by_type[t]:
            pk = ", ".join(p.split("/")[-1] for p in o["packages"])
            r = o["reason"].replace("|", "\\|").replace("\n", " ")
            r = (r[:150] + "…") if len(r) > 150 else r
            flag = " ⚑" if o["cross_package"] else ""
            out.append("| `%s`%s | %d | `%s` | %s | `%s` | %s |"
                       % (o["id"], flag, o["drift_count"], o["closure_route"],
                          pk, o["rule"], r))
        out.append("")
    out.append("⚑ = the same defect falsifies anchors in more than one package: "
               "a **release event** under §5-D of the campaign plan, closed by a "
               "published version and a re-vendor, never by an edit in one consumer.\n")
    return "\n".join(out)


if __name__ == "__main__":
    raise SystemExit(main())
