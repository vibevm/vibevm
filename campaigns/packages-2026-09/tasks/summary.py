#!/usr/bin/env python3
"""The phase's own X/Y/Z summary, and the self-referential count nothing ships.

Usage:
    python tasks/summary.py                 # the whole campaign, by namespace
    python tasks/summary.py --by-file       # per file, within each namespace
    python tasks/summary.py --batch W1      # one batch, from PHASE-C-BATCHES.json
    python tasks/summary.py --cache <path>  # read a fixture instead of run/cache.json

Phase C's exit gate asks for two counts that no shipped command produces.

Clause (ii) wants the X/Y/Z summary — confirmed / drift / unverifiable, per
cluster — recorded in the LOG. `vibe progress report` counts markers and
coverage, not verdict outcomes, so the phase has been arriving at these
figures by hand each time.

Clause (iv) wants something the tooling cannot know at all: **how many `world`
verdicts rest on source 1 alone.** Amendment A2 calls those self-referential —
the package agreeing with itself — because F-063 survived wave 1 by
cross-checking one spec document against another spec document carrying the
identical error. A2 made `src` a field precisely so the number could be
counted rather than asserted; this is the counter it named.

It reads `run/cache.json` and writes nothing.
"""

import json, sys, pathlib, collections

ZONE = pathlib.Path(__file__).resolve().parent.parent
CACHE = ZONE / "run" / "cache.json"
BATCHES = ZONE / "tasks" / "PHASE-C-BATCHES.json"
ORDER = ["confirmed", "drift", "unverifiable"]


def namespace(path):
    if "vibevm/vibepacks/org.vibevm.ai-native/" in path:
        return "ai-native"
    if "vibevm/vibepacks/org.vibevm.world/" in path:
        return "world"
    return "host"


def arg(name):
    return sys.argv[sys.argv.index(name) + 1] if name in sys.argv else None


def evidence_key(v):
    """The identity of a verdict's evidence, for the shared-blob split.

    Two verdicts stamped with the same evidence paragraph are, between them,
    one act of reading — so the pair is evidence about the DOCUMENT, not about
    either statement. The key is the evidence itself; nothing needs to be
    re-judged to compute the split, which is why the ruling could start today.
    """
    ev = v.get("ev")
    if not ev:
        return None
    return json.dumps(ev, sort_keys=True, ensure_ascii=False)


def line(label, tally, selfref=None, width=22, grains=None):
    total = sum(tally.get(k, 0) for k in ORDER)
    if not total:
        return
    pct = 100.0 * tally.get("confirmed", 0) / total
    cells = "  ".join(f"{k[:5]} {tally.get(k, 0):>5}" for k in ORDER)
    tail = ""
    if selfref is not None:
        tail = f"   src=[1] {selfref:>4} ({100.0 * selfref / total:.1f} % self-referential)"
    print(f"  {label:<{width}} {cells}   total {total:>5}   {pct:5.1f} %{tail}")
    if grains:
        per_fact, doc_level = grains
        both = per_fact + doc_level
        if both:
            share = 100.0 * per_fact / both
            print(
                f"  {'':<{width}} of which confirmed: per-fact {per_fact:>5}"
                f" · document-level {doc_level:>5}   ({share:5.1f} % per-fact)"
            )


def main():
    # `--cache` exists so the src arithmetic below can be exercised against a
    # fixture before four thousand world verdicts ride on it. The campaign's own
    # rule: a tool is trusted after its behaviour has been made to fire, not
    # before.
    cache = json.loads(pathlib.Path(arg("--cache") or CACHE).read_text(encoding="utf-8"))
    want = None
    if arg("--batch"):
        d = json.loads(BATCHES.read_text(encoding="utf-8"))
        rows = [b for b in d["batches"] if b["id"] == arg("--batch")]
        if not rows:
            raise SystemExit(f"no batch {arg('--batch')!r} in {BATCHES.name}")
        want = set(rows[0]["files"])

    # First pass: how many verdicts each evidence blob carries. A blob used
    # once is that fact's own evidence; a blob used twice or more is one
    # reading stamped on several statements.
    blob_uses = collections.Counter()
    for path, rec in cache["files"].items():
        if want is not None and path not in want:
            continue
        for v in rec.get("campaign", {}).get("verdicts", {}).values():
            if isinstance(v, dict):
                k = evidence_key(v)
                if k is not None:
                    blob_uses[k] += 1

    per_ns = collections.defaultdict(collections.Counter)
    grain = collections.defaultdict(collections.Counter)
    selfref = collections.Counter()
    nosrc = collections.Counter()
    per_file = collections.defaultdict(collections.Counter)
    files_seen = collections.defaultdict(set)

    for path, rec in cache["files"].items():
        if want is not None and path not in want:
            continue
        verdicts = rec.get("campaign", {}).get("verdicts", {})
        if not verdicts:
            continue
        ns = namespace(path)
        for anchor, v in verdicts.items():
            if not isinstance(v, dict) or "v" not in v:
                continue
            per_ns[ns][v["v"]] += 1
            per_file[path][v["v"]] += 1
            if v["v"] == "confirmed":
                k = evidence_key(v)
                shared = k is None or blob_uses[k] > 1
                grain[ns]["document" if shared else "fact"] += 1
            files_seen[ns].add(path)
            if ns == "world":
                src = v.get("src")
                if not src:
                    nosrc[ns] += 1
                elif list(src) == [1]:
                    selfref[ns] += 1

    scope = f"batch {arg('--batch')}" if want is not None else "the whole campaign"
    print(f"verdict summary — {scope}\n")
    grand = collections.Counter()
    grand_grain = collections.Counter()
    for ns in ("host", "ai-native", "world"):
        if not per_ns[ns]:
            continue
        line(f"{ns} ({len(files_seen[ns])} files)", per_ns[ns],
             selfref[ns] if ns == "world" else None,
             grains=(grain[ns]["fact"], grain[ns]["document"]))
        grand.update(per_ns[ns])
        grand_grain.update(grain[ns])
        if nosrc[ns]:
            print(f"  {'':<22} !! {nosrc[ns]} world verdict(s) carry no src — A2 requires one")
    print()
    line("ALL", grand, grains=(grand_grain["fact"], grand_grain["document"]))
    print()
    print("  per-fact         — this statement has its own evidence record naming a")
    print("                     concrete place in code or another document. If the")
    print("                     statement is false the evidence collapses with it.")
    print("  document-level   — one evidence paragraph is stamped on several")
    print("                     statements at once: somebody read the document whole")
    print("                     and concluded it is implemented. If one of them is")
    print("                     false, the paragraph about the rest still looks right")
    print("                     and the lie does not surface.")
    print("  A verdict stays document-level until its fact's text moves or somebody")
    print("  re-judges it deliberately.")

    if "--by-file" in sys.argv:
        print()
        for path in sorted(per_file, key=lambda p: (namespace(p), p)):
            t = per_file[path]
            if want is None and namespace(path) == "host":
                continue
            line(path.split("/", 2)[-1][:70], t, width=72)
    return 0


if __name__ == "__main__":
    sys.exit(main())
