"""The migration's argument surface and its one decision procedure."""

import argparse
import json
import pathlib
import sys

from .compare import compare_corpus, remap_baseline
from .mapping import build_map, relayout
from .mirror import Mirror, git, materialise, mirror_tree, vibe_command
from .refusal import Refusal
from .report import report
from .state import byte_sorted, dumps, migrate_cache, migrate_corpus_state, publish, zone_paths

DEFAULT_ZONE = pathlib.Path(__file__).resolve().parent.parent.parent
DEFAULT_ROOT = DEFAULT_ZONE.parent.parent


def parse_args(argv):
    ap = argparse.ArgumentParser(
        prog="corpus-migration.py",
        description="Move the judged corpus onto its live physical identities.")
    mode = ap.add_mutually_exclusive_group(required=True)
    mode.add_argument("--check", action="store_true",
                      help="report the mapping and the comparison; write no tracked state")
    mode.add_argument("--apply", action="store_true",
                      help="rewrite cache.json, baseline.json and state/corpus.json")
    ap.add_argument("--accept-moved-facts", type=int, metavar="N",
                    help="acknowledge that exactly N judged facts' text moved across the "
                         "migration and come due for re-judgement. The count is pinned: a "
                         "corpus that moved further since --check refuses again")
    ap.add_argument("--accept-historical-gaps", type=int, metavar="N",
                    help="acknowledge exactly N judged anchors that exist at the mapped path "
                         "but have nothing to compare against in the pre-migration parse. A "
                         "separate pin from --accept-moved-facts on purpose: 'the text moved' "
                         "and 'nobody can say whether it moved' are different admissions")
    ap.add_argument("--old-rev", help="revision holding the pre-migration paths "
                                      "(default: the last commit at the cache's `updated_at`)")
    ap.add_argument("--root", help="repository root (default: the tree this script lives in)")
    ap.add_argument("--zone", help="campaign zone (default: this script's own campaign)")
    ap.add_argument("--scratch", help="scratch root for the two mirrors "
                                      "(default: <zone>/run/mirror/.migration-scratch)")
    ap.add_argument("--old-mirror", help="a pre-migration mirror to use instead of building one")
    ap.add_argument("--new-mirror", help="a live mirror to use instead of building one")
    ap.add_argument("--vibe", help="the shipped CLI (default: target/debug, else cargo run)")
    ap.add_argument("--reuse-mirrors", action="store_true",
                    help="reuse scratch mirrors from an earlier run instead of rebuilding")
    ap.add_argument("--max-report", type=int, default=25, help="rows per listing (default 25)")
    return ap.parse_args(argv)


def resolve_paths(args):
    root = pathlib.Path(args.root).resolve() if args.root else DEFAULT_ROOT
    zone = pathlib.Path(args.zone).resolve() if args.zone else DEFAULT_ZONE
    return zone_paths(root, zone)


def resolve_old_rev(root, cache, explicit):
    """The revision the campaign cache describes.

    Default: the last commit at or before the cache's own `updated_at`. That
    stamp is when the records were last written, so that tree is the one every
    `content_hash` in them was taken over — which makes the reference a fact
    about the artifact being migrated rather than a date somebody remembered.
    """
    if explicit:
        return explicit
    stamp = cache.get("updated_at")
    if not stamp:
        raise Refusal("the cache carries no `updated_at`; pass --old-rev explicitly")
    rev = git(root, "rev-list", "-1", f"--before={stamp}", "HEAD").strip()
    if not rev:
        raise Refusal(f"no commit at or before the cache stamp {stamp}")
    return rev


def build_mirrors(args, paths, cache):
    """The two mirrors, built by the shipped parser unless handed in.

    `--old-mirror` / `--new-mirror` skip construction entirely: a caller that
    already has both — a re-run, or a test with no history to read — should not
    pay for a second parse of the corpus, and the comparison cannot tell the
    difference, because the mirrors are the same artifact either way.
    """
    if args.old_mirror and args.new_mirror:
        return "(supplied)", Mirror(args.old_mirror), Mirror(args.new_mirror), None
    root, zone = paths["root"], paths["zone"]
    # Default scratch home: inside `run/mirror/`, which `.gitignore` already
    # declares unstageable ("a campaign's per-file mirror … `run/` is
    # disposable"). The scratch is the same genre — regenerable parse output,
    # hundreds of megabytes — and anywhere else it would sit in `git status` for
    # the length of the migration, which is what that rule exists to stop.
    scratch = (pathlib.Path(args.scratch) if args.scratch
               else zone / "run" / "mirror" / ".migration-scratch")
    scratch = scratch if scratch.is_absolute() else root / scratch
    old_tree = scratch / "old-tree"
    old_mirror = old_tree / "campaigns" / "pre-migration" / "run" / "mirror"
    new_mirror = scratch / "zone-live" / "run" / "mirror"
    vibe = vibe_command(root, args.vibe)
    rev = resolve_old_rev(root, cache, args.old_rev)

    if not (args.reuse_mirrors and old_mirror.is_dir() and new_mirror.is_dir()):
        materialise(root, rev, sorted(cache["files"]), old_tree)
        mirror_tree(vibe, root, old_tree, "campaigns/pre-migration")
        # The live side is parsed with the project's own `facts.toml`, so the
        # scope this migration lands on is the scope the next `vibe progress`
        # run will see — never a private one this program made up.
        (scratch / "zone-live").mkdir(parents=True, exist_ok=True)
        mirror_tree(vibe, root, root, str(scratch / "zone-live"))
    return rev, Mirror(old_mirror), Mirror(new_mirror), scratch


def require_mirrored(cache, mapping, old_mirror, new_mirror):
    missing = [f"{p} (pre-migration)" for p in cache["files"] if not old_mirror.has(p)]
    missing += [f"{mapping[p]} (live)" for p in cache["files"]
                if not new_mirror.has(mapping[p])]
    if missing:
        raise Refusal(f"{len(missing)} file(s) are absent from a mirror — the live scope "
                      "does not observe them", missing[:20])


def main(argv=None, out=sys.stdout):
    args = parse_args(argv)
    paths = resolve_paths(args)
    cache = json.loads(paths["cache"].read_text(encoding="utf-8"))
    baseline = json.loads(paths["baseline"].read_text(encoding="utf-8"))
    corpus = json.loads(paths["corpus"].read_text(encoding="utf-8"))

    unmapped = [p for p in cache["files"] if relayout(p) is None]
    if cache["files"] and len(unmapped) == len(cache["files"]):
        print(f"no-op: all {len(unmapped)} cache records already name live-layout paths; "
              "nothing to migrate.", file=out)
        return 0
    if unmapped:
        raise Refusal(f"the cache is half-migrated — {len(unmapped)} of {len(cache['files'])} "
                      "records already name live-layout paths", sorted(unmapped)[:20])

    mapping = build_map(sorted(cache["files"]), lambda p: (paths["root"] / p).is_file())
    overlap = sorted(set(mapping.values()) & set(cache["files"]))
    if overlap:
        raise Refusal("old and new key sets overlap — a mapped path is already a cache key",
                      overlap[:20])

    rev, old_mirror, new_mirror, scratch = build_mirrors(args, paths, cache)
    require_mirrored(cache, mapping, old_mirror, new_mirror)
    cmp_ = compare_corpus(mapping, cache, old_mirror, new_mirror)
    new_units, how, unresolved = remap_baseline(baseline, mapping, old_mirror, new_mirror)
    report(args.max_report, rev, mapping, cmp_, how, unresolved, out=out)
    if scratch:
        print(f"\n  scratch mirrors: {scratch}", file=out)

    if cmp_.structural_failures:
        raise Refusal(f"{len(cmp_.structural_failures)} structural failure(s) — a verdict would "
                      "be orphaned; nothing was written")
    if args.check:
        print("\n--check: nothing written.", file=out)
        return 0
    if cmp_.moved and args.accept_moved_facts != len(cmp_.moved):
        raise Refusal(
            f"{len(cmp_.moved)} judged fact(s) do not hash to the text they were judged "
            f"against. Their verdicts are carried, not re-formed — so an operator, not this "
            f"program, decides that is acceptable: re-run with "
            f"--accept-moved-facts {len(cmp_.moved)}")
    if cmp_.historical_gap and args.accept_historical_gaps != len(cmp_.historical_gap):
        raise Refusal(
            f"{len(cmp_.historical_gap)} judged anchor(s) have nothing to compare against in "
            f"the pre-migration parse — the verdict exists, the historical text does not, so "
            f"whether it moved is UNKNOWN rather than false: re-run with "
            f"--accept-historical-gaps {len(cmp_.historical_gap)}",
            [f"{old}#{anchor} -> {live} ({why})"
             for old, live, anchor, why in cmp_.historical_gap[:20]])
    if len(new_units) != len(baseline["units"]):
        raise Refusal(f"baseline unit count changed: {len(baseline['units'])} -> {len(new_units)}")

    migrated_baseline = dict(baseline, units={k: new_units[k] for k in byte_sorted(new_units)})
    writes = {
        paths["cache"]: dumps(migrate_cache(cache, mapping)),
        paths["baseline"]: dumps(migrated_baseline),
        paths["corpus"]: dumps(migrate_corpus_state(corpus, mapping)),
    }
    journal = paths["journal"]
    journal_before = journal.read_bytes() if journal.exists() else None
    publish(writes)
    if journal_before is not None and journal.read_bytes() != journal_before:
        raise Refusal("the journal changed during the migration — it must never be written")
    print(f"\n--apply: rewrote {', '.join(sorted(p.name for p in writes))}. "
          "The journal was not touched.", file=out)
    print(f"Next: `vibe progress mirror --campaign {paths['zone'].name} --path .` to regenerate "
          "run/mirror/ and refresh the derived projections.", file=out)
    return 0
