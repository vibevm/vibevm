#!/usr/bin/env python3
"""Merge a batch's verdicts into the campaign cache — validating, load-and-merge only.

Usage:
    python tasks/merge-verdicts.py <batch.json> [--force]

The batch file is:

    {
      "batch":   "C1",
      "cluster": "ai-native" | "world" | "host",
      "files": {
        "<repo-relative path>": {
          "<anchor>": {"v": "confirmed", "ev": ["..."], "src": [1,2]}
        }
      }
    }

Why this exists as a program rather than as care. Three of this campaign's
rules are stated in prose and are trivially broken by hand: a verdict without
an evidence ref is rejected; a `world` verdict must record its §3.1 source
class; and the cache is mutated by load-and-merge only. A rule with no checker
is a WISH — the Discipline's own law — so they are checked here:

  * every anchor must be an ADDRESSABLE anchor of the file it is filed under,
    taken from the campaign's own mirror rather than from a regex of my own
    (`progress_core::seal::addressable` is the definition; the mirror is its
    output);
  * `v` must be one of confirmed / drift / unverifiable;
  * `ev` must be a non-empty list of non-trivial strings — "probably true" is
    not an evidence ref, and neither is "";
  * a `world` verdict must carry `src` ⊆ [1,2,3], non-empty (amendment A2);
  * an existing verdict is never silently overwritten.

It does NOT write `verified_at` or `processed_hash`. Those belong to
`vibe progress seal`, which is run afterwards, because a hand-written
timestamp fails UNSAFE: `moved_crate` calls a crate moved when its commits are
NEWER than the verdict, so a wrong stamp means nothing is ever newer and the
invalidation rule never fires.
"""

import json, os, sys, pathlib

ZONE = pathlib.Path(__file__).resolve().parent.parent
CACHE = ZONE / "run" / "cache.json"
MIRROR = ZONE / "run" / "mirror"
VOCAB = {"confirmed", "drift", "unverifiable"}


def addressable(path):
    """The anchors a verdict map may key, for one file, per the campaign's mirror."""
    slug = path.replace("/", "__") + ".json"
    f = MIRROR / slug
    if not f.exists():
        raise SystemExit(f"no mirror for {path} — run `vibe progress mirror --campaign {ZONE.name}`")
    d = json.loads(f.read_text(encoding="utf-8"))
    return {fact["id"] for b in d["blocks"] for fact in b.get("facts", [])
            if fact.get("marked") and fact.get("id")}


def main():
    if len(sys.argv) < 2:
        raise SystemExit(__doc__)
    batch = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
    force = "--force" in sys.argv
    cluster = batch["cluster"]
    bid = batch["batch"]

    raw = CACHE.read_text(encoding="utf-8")
    cache = json.loads(raw)

    errors, planned = [], []
    for path, verdicts in batch["files"].items():
        if path not in cache["files"]:
            errors.append(f"{path}: not observed by the campaign")
            continue
        allowed = addressable(path)
        existing = cache["files"][path].get("campaign", {}).get("verdicts", {})
        for anchor, rec in verdicts.items():
            where = f"{path}#{anchor}"
            if anchor not in allowed:
                errors.append(f"{where}: not an addressable anchor of this file")
            if rec.get("v") not in VOCAB:
                errors.append(f"{where}: verdict {rec.get('v')!r} is outside {sorted(VOCAB)}")
            ev = rec.get("ev")
            if not isinstance(ev, list) or not ev:
                errors.append(f"{where}: no evidence ref — a verdict without one is rejected")
            elif any(not isinstance(e, str) or len(e.strip()) < 8 for e in ev):
                errors.append(f"{where}: an evidence ref is empty or too short to locate anything")
            if cluster == "world":
                src = rec.get("src")
                if not isinstance(src, list) or not src or not set(src) <= {1, 2, 3}:
                    errors.append(f"{where}: a world verdict must carry src as a non-empty subset of [1,2,3] (A2)")
            elif "src" in rec:
                errors.append(f"{where}: src is defined for the world cluster only (§3.1)")
            if anchor in existing and not force:
                errors.append(f"{where}: already judged {existing[anchor].get('v')!r} — pass --force to restate")
            planned.append(where)

    if errors:
        print(f"REFUSED — {len(errors)} problem(s); nothing written:\n")
        for e in errors:
            print("  " + e)
        return 1

    for path, verdicts in batch["files"].items():
        camp = cache["files"][path].setdefault("campaign", {})
        vmap = camp.setdefault("verdicts", {})
        vmap.update(verdicts)
        prior = camp.get("verify_batch")
        camp["verify_batch"] = bid if not prior else (prior if bid in prior.split(" + ") else f"{prior} + {bid}")

    out = json.dumps(cache, indent=2, ensure_ascii=False)
    tmp = CACHE.with_suffix(".json.tmp")
    tmp.write_text(out, encoding="utf-8", newline="")
    os.replace(tmp, CACHE)

    tally = {}
    for verdicts in batch["files"].values():
        for rec in verdicts.values():
            tally[rec["v"]] = tally.get(rec["v"], 0) + 1
    print(f"{bid}: {len(planned)} verdict(s) merged over {len(batch['files'])} file(s)")
    for k in sorted(tally):
        print(f"  {k:14} {tally[k]}")
    if cluster == "world":
        selfref = sum(1 for vs in batch["files"].values() for r in vs.values() if r.get("src") == [1])
        print(f"  {'self-referential':14} {selfref}  (src == [1] alone, A2)")
    print("\nNow seal: vibe progress seal --campaign campaigns/" + ZONE.name + " <paths…>")
    return 0


if __name__ == "__main__":
    sys.exit(main())
