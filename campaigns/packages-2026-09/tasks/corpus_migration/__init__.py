"""Corpus-migration machinery: the PROP-052 path law, the shipped parser's
mirrors, and the campaign state that is keyed by physical identity.

The seams, and why they are where they are:

* `refusal`  — the one way anything here declines to act.
* `mapping`  — the PROP-052 path law in BOTH directions. Forward the migration
               moves keys along it; backward `text-stability.py` walks it to find
               the identity a live document had at its verdict revision. One
               table, one slot-depth rule: two copies would drift, and the
               symptom would be a verdict silently attached to the wrong file.
* `mirror`   — the ONLY reader of a document in this package. Every parse comes
               from `vibe progress mirror`, so the projection dispatch and the
               content-hash recipe have exactly one implementation, the engine's.
* `compare`  — two mirrors in, "what survived / what moved" out.
* `state`    — the tracked artifacts: pure transforms plus one all-or-nothing write.
* `report`   — the migration's printed form.
* `cli`      — argument surface and the decision procedure; the launcher
               `corpus-migration.py` is a four-line shim over `cli.main`.

Both programs that use this package — `corpus-migration.py` and
`text-stability.py` — are launchers. The code is here so it can be imported,
tested and kept under the repository's 600-line file budget (`conform.toml`
`max_file_lines`).
"""

from .compare import Comparison, compare_corpus, remap_baseline, remap_baseline_addr
from .mapping import (DOCUMENT_KEYS, LINE_FRAG, PACKAGE_SLOT_DEPTH, ROOTS, build_map,
                      historical_candidates, map_path, markdown_twin, relayout, split_kinds,
                      unrelayout)
from .mirror import (Mirror, batch_header, blob_exists, git, have_vibe, materialise, mirror_tree,
                     run, unit_frag, vibe_command)
from .refusal import Refusal, report_refusal
from .state import (byte_sorted, dumps, migrate_cache, migrate_corpus_state, publish,
                    zone_paths)

__all__ = [
    "Comparison", "DOCUMENT_KEYS", "LINE_FRAG", "Mirror", "PACKAGE_SLOT_DEPTH", "ROOTS",
    "Refusal", "batch_header", "blob_exists", "build_map", "byte_sorted", "compare_corpus",
    "dumps", "git", "have_vibe", "historical_candidates", "map_path", "markdown_twin",
    "materialise", "migrate_cache", "migrate_corpus_state", "mirror_tree", "publish", "relayout",
    "remap_baseline", "remap_baseline_addr", "report_refusal", "run", "split_kinds",
    "unit_frag", "unrelayout", "vibe_command", "zone_paths",
]
