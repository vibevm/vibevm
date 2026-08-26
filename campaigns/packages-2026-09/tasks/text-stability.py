#!/usr/bin/env python3
"""Which judged facts actually moved since their verdicts were formed.

Usage:
    python tasks/text-stability.py               # the full report
    python tasks/text-stability.py --sealable    # only the paths safe to seal
    python tasks/text-stability.py --verify-live # + prove the mirror matches the tree

Staleness (`processed_hash` != `content_hash`) is a DOCUMENT-level signal and it
over-reports badly: a file goes stale when any byte moves, including a `<status>`
comment or prose no marked fact owns. Measured 2026-08-05 over the 20 stale files
of `packages-2026-09`: 1214 judged verdicts, of which 13 facts had actually
changed. Re-reading 1214 to find 13 is not diligence, it is theatre — this
program finds the 13, per fact, over every judged file rather than only the stale
ones.

**How, and why this file is now four lines over a package.** The fact's text is
needed twice — at the revision its verdict was formed against, and now — and both
readings come from `vibe progress mirror`, the shipped parser. The historical
side is reached by walking the PROP-052 path law BACKWARDS to the shape the
document had at that revision, proving the blob is there, materialising each
revision's blobs into an isolated tree and mirroring it. See
`corpus_migration/stability.py` for the four rules that follow, and
`corpus_migration/__init__.py` for the package seams.

**What a clean result licenses, and what it does not.** Byte-identical text means
each verdict was formed against exactly the text on disk today, so the
re-derivation `vibe progress seal` asks for has been performed — mechanically,
over every judged fact. It does NOT re-assert the verdict against the TREE: a
claim whose wording never moved can still have gone stale because the code under
it moved, and that is `drift`, caught by ongoing judging and never by a seal.
Seal what this reports clean; do not read it as freshness.

Exit code 0 when the report was produced, 1 on a refusal that stopped it. A
refusal about ONE file is data, printed in its own bucket: that file is simply
never sealable.
"""

import argparse
import json
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

from corpus_migration.mirror import Mirror, have_vibe, vibe_command  # noqa: E402
from corpus_migration.refusal import Refusal, report_refusal  # noqa: E402
from corpus_migration.stability import analyse, buckets, render  # noqa: E402

ZONE = pathlib.Path(__file__).resolve().parent.parent
ROOT = ZONE.parent.parent


def parse_args(argv):
    ap = argparse.ArgumentParser(prog="text-stability.py",
                                 description=__doc__.splitlines()[0])
    ap.add_argument("--sealable", action="store_true",
                    help="print only the paths every judged fact of which compared equal")
    ap.add_argument("--verify-live", action="store_true",
                    help="re-parse the working tree and prove the campaign mirror is current "
                         "with it, not merely with the cache")
    ap.add_argument("--only", action="append", metavar="PATH",
                    help="restrict to these live paths (repeatable) — for a quick re-check")
    ap.add_argument("--reuse-mirrors", action="store_true",
                    help="reuse historical mirrors built by an earlier run")
    ap.add_argument("--root", help="repository root")
    ap.add_argument("--zone", help="campaign zone")
    ap.add_argument("--vibe", help="the shipped CLI (default: target/debug, else cargo run)")
    ap.add_argument("--max-report", type=int, default=25, help="rows per listing (default 25)")
    return ap.parse_args(argv)


def main(argv=None, out=sys.stdout):
    args = parse_args(argv)
    root = pathlib.Path(args.root).resolve() if args.root else ROOT
    zone = pathlib.Path(args.zone).resolve() if args.zone else ZONE
    cache_path = zone / "run" / "cache.json"
    if not cache_path.exists():
        raise Refusal(f"no cache at {cache_path} — run `vibe progress scan` first")
    cache = json.loads(cache_path.read_text(encoding="utf-8"))
    live_mirror = Mirror(zone / "run" / "mirror")
    if not live_mirror.dir.is_dir():
        raise Refusal(f"no mirror at {live_mirror.dir} — run `vibe progress mirror` first")
    if not have_vibe(root):
        raise Refusal("no shipped CLI reachable (build it, or set VIBE_BIN) — this program "
                      "parses nothing itself and cannot proceed without the engine")

    outcomes, stats = analyse(vibe_command(root, args.vibe), root, zone, cache, live_mirror,
                              only=set(args.only) if args.only else None,
                              reuse=args.reuse_mirrors, verify_live=args.verify_live)
    if args.sealable:
        for outcome in sorted(buckets(outcomes)[0], key=lambda o: o.path):
            print(outcome.path, file=out)
        return 0
    render(outcomes, stats, args.max_report, out)
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Refusal as refusal:
        report_refusal(refusal, sys.stderr)
        sys.exit(1)
