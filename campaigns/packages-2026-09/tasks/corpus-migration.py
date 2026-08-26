#!/usr/bin/env python3
"""Move the judged corpus onto its live physical identities — PROP-052 roots and
the Markdown→XML conversion — without losing one verdict (B-107).

Usage:
    python tasks/corpus-migration.py --check
    python tasks/corpus-migration.py --apply --accept-moved-facts <N>

Why this exists. A campaign's knowledge is keyed by PHYSICAL IDENTITY: the cache
keys files by repo-relative path, the baseline keys units by `path#anchor`, and
both were written before two tree-wide moves — the K-wave Markdown→XML conversion
and the PROP-052 relayout. Every one of those keys named a path that no longer
exists, so every instrument reading them reported about a tree that is gone, and
`vibe progress` would have PRUNED the records on its next scan
(`Cache::retain_paths`) — one warning line per lost file. This program moves the
keys first.

What it is not: a rewriter of history. Evidence prose citing old paths is left
exactly as written (a verdict's evidence is a record of what was read, not a live
pointer), `run/journal.jsonl` is never touched, and no verdict value,
`verified_at` or batch id is altered.

The parser is never re-implemented. Both sides of every content comparison come
from `vibe progress mirror` — see `corpus_migration/mirror.py`.

This file is a launcher. The machinery lives in the `corpus_migration` package
beside it, which `text-stability.py` shares; see its `__init__` for the seams.

Exit codes: 0 success (or a no-op re-run), 1 refusal, 2 usage.
"""

import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

from corpus_migration.cli import main  # noqa: E402
from corpus_migration.refusal import Refusal, report_refusal  # noqa: E402

if __name__ == "__main__":
    try:
        sys.exit(main())
    except Refusal as refusal:
        report_refusal(refusal, sys.stderr)
        sys.exit(1)
