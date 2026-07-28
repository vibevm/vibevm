#!/usr/bin/env python3
"""Re-locate an evidence ref whose line number went stale, and refuse to guess.

Usage:
    python tasks/repair-refs.py <ev-*.json> [--apply]

`verify-evidence.py` reports a ref as TEXT when the quote is not at the cited
line. That verdict has two very different causes and the checker cannot tell
them apart:

  * the quote is REAL and the line moved — the file was edited after the worker
    read it. Nothing is wrong with the evidence; the coordinate is stale.
  * the quote is NOT in the file at all — the ref is a fiction, and no verdict
    may rest on it.

This separates them. For every failing ref it searches the whole file for the
quoted text; if the quote occurs at exactly ONE place, the ref is repairable and
the new line number is printed (and written with `--apply`). If it occurs at
several places the ref is ambiguous and is left alone — a repair that picks one
of three candidates is a guess wearing a line number. If it occurs nowhere the
ref is a fiction and is reported as such.

Written after this session edited a harvest file while five workers were citing
it: eleven lines went in, and every ref into that file below the insertion
point pointed one paragraph too high. **Do not edit a file a running worker
cites.** The workers here caught it themselves and re-verified before returning
— one said so in its report — so nothing needed repairing in the end. The next
set may not, and discarding a table over a coordinate the boss moved would
throw away good evidence for a defect the boss introduced.

Matching uses `verify-evidence.py`'s own `norm`, imported rather than copied, so
a repair here is a pass there and the two cannot drift apart.
"""

import json, re, sys, pathlib, collections, importlib.util

ROOT = pathlib.Path(__file__).resolve().parents[3]
REF = re.compile(r"^\s*([^\s:]+(?::[^\s:]*)??[^\s:]*\.[A-Za-z0-9]+):(\d+)\s*(.*)$")

# One normalisation, defined once. Two copies of this rule would drift, and a
# repair that passes here and fails the checker is worse than no repair.
_spec = importlib.util.spec_from_file_location(
    "verify_evidence", pathlib.Path(__file__).with_name("verify-evidence.py"))
_ve = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_ve)
norm = _ve.norm


def locate(path, snippet):
    """Every 1-based line whose text (or its 4-line window) contains the quote."""
    f = ROOT / path
    if not f.is_file():
        return None
    lines = f.read_text(encoding="utf-8", errors="replace").splitlines()
    want = norm(snippet)
    if len(want) < 4:
        return []
    hits = [i + 1 for i, l in enumerate(lines) if want in norm(l)]
    if hits:
        return hits
    # A quote may span a few source lines, anchored at the first.
    return [i + 1 for i in range(len(lines))
            if want in norm(" ".join(lines[i:i + 4]))]


def main():
    if len(sys.argv) < 2:
        raise SystemExit(__doc__)
    p = pathlib.Path(sys.argv[1])
    rows = json.loads(p.read_text(encoding="utf-8"))
    apply = "--apply" in sys.argv

    tally = collections.Counter()
    changed = 0
    for r in rows:
        evs = r.get("evidence") or []
        for i, ref in enumerate(evs):
            m = REF.match(ref)
            if not m:
                tally["unparsed"] += 1
                continue
            path, line, snippet = m.group(1), int(m.group(2)), m.group(3)
            # The checker owns pass/fail. Anything it accepts — OK, OFF-BY, or an
            # honestly elided quote — is not this tool's business, and treating
            # an ELIDED ref as a fiction because this file reimplements a
            # narrower rule is precisely the wolf-crying the campaign has now
            # caught in four separate instruments.
            kind, _ = _ve.check_ref(ref)
            if kind in ("OK", "OFF-BY", "ELIDED"):
                tally["already-right"] += 1
                continue
            f = ROOT / path
            if not f.is_file():
                tally["missing-file"] += 1
                print(f"  FICTION  {r['anchor']}: {path} does not exist")
                continue
            hits = locate(path, snippet)
            if hits is None or not snippet.strip() or len(snippet.strip()) < 4:
                tally["no-quote"] += 1
                continue
            if len(hits) == 1:
                tally["repairable"] += 1
                print(f"  MOVED    {r['anchor']}: {path}:{line} -> :{hits[0]}  ({hits[0] - line:+d})")
                if apply:
                    evs[i] = re.sub(rf"^(\s*{re.escape(path)}):{line}\b", rf"\1:{hits[0]}", ref)
                    changed += 1
            elif hits:
                tally["ambiguous"] += 1
                print(f"  AMBIG    {r['anchor']}: {path}:{line} — quote occurs at {hits[:6]}; left alone")
            else:
                tally["fiction"] += 1
                print(f"  FICTION  {r['anchor']}: {path}:{line} — the quote is nowhere in the file")

    print(f"\n{p.name}: " + ", ".join(f"{k}={v}" for k, v in sorted(tally.items())))
    if apply and changed:
        p.write_text(json.dumps(rows, indent=2, ensure_ascii=False), encoding="utf-8", newline="")
        print(f"applied: {changed} ref(s) re-pointed")
    elif apply:
        print("applied: nothing to change")
    else:
        print("(dry run — pass --apply to write)")
    return 1 if tally["fiction"] or tally["missing-file"] else 0


if __name__ == "__main__":
    sys.exit(main())
