"""The PROP-052 path law, in both directions.

Forward (`relayout` → `map_path` → `build_map`) is what the migration moves the
campaign's keys along. Backward (`unrelayout` → `historical_candidates`) is what
`text-stability.py` walks to find the identity a live document had at the
revision its verdicts were formed against. One table, one slot-depth rule, two
directions — because two copies of this law would drift, and the symptom would
be a verdict quietly attached to the wrong file.
"""

import collections
import pathlib
import re

from .refusal import Refusal

# Verdict keys that are document-level bundles rather than facts; they have no
# addressable anchor by construction (the set `judging-debt.py` also knows).
DOCUMENT_KEYS = {"_elements"}

LINE_FRAG = re.compile(r"^L(\d+)$")

# The PROP-052 root rewrites. `spec/` and `packages/` are the two legacy roots
# the campaign's keys were written against; `crates/vibe-core/src/layout.rs` is
# the product-side home of these names. This table is a one-shot migration's own
# copy of the same law and it is frozen — it describes ONE move that already
# happened, so it must not follow the product if the product moves again.
ROOTS = (("spec/", "vibevm/vibespecs/"), ("packages/", "vibevm/vibepacks/"))
# L4 — a package root mirrors the same layout: `<ns>/<pkg>/<ver>/spec/…` becomes
# `<ns>/<pkg>/<ver>/vibevm/vibespecs/…`. The depth is fixed by the slot shape, so
# this is a POSITION rule and never "replace the first `/spec/` you see" — a
# document whose path carries a `spec` segment deeper down keeps it.
PACKAGE_SLOT_DEPTH = 3


def relayout(old):
    """The PROP-052 rewrite of one repo-relative path, extension untouched.

    `None` for a path naming neither legacy root — which is also how a re-run
    recognises a cache that has already been migrated.
    """
    for legacy, live in ROOTS:
        if old.startswith(legacy):
            rest = old[len(legacy):]
            if legacy == "packages/":
                seg = rest.split("/")
                if len(seg) > PACKAGE_SLOT_DEPTH and seg[PACKAGE_SLOT_DEPTH] == "spec":
                    rest = "/".join(seg[:PACKAGE_SLOT_DEPTH] + ["vibevm", "vibespecs"]
                                    + seg[PACKAGE_SLOT_DEPTH + 1:])
            return live + rest
    return None


def unrelayout(live):
    """The exact inverse of [`relayout`]: a live path back to its legacy spelling.

    `relayout(unrelayout(p)) == p` for every path this corpus holds, which is the
    property the historical search rests on — and the reason the inner-root case
    is spelled here rather than approximated by a string replace: the live inner
    root is two segments (`vibevm/vibespecs`) where the legacy one was one
    (`spec`), so the position arithmetic differs going back.
    """
    for legacy, forward in ROOTS:
        if live.startswith(forward):
            rest = live[len(forward):]
            if legacy == "packages/":
                seg = rest.split("/")
                inner = slice(PACKAGE_SLOT_DEPTH, PACKAGE_SLOT_DEPTH + 2)
                if len(seg) > PACKAGE_SLOT_DEPTH + 2 and seg[inner] == ["vibevm", "vibespecs"]:
                    rest = "/".join(seg[:PACKAGE_SLOT_DEPTH] + ["spec"]
                                    + seg[PACKAGE_SLOT_DEPTH + 2:])
            return legacy + rest
    return None


def markdown_twin(path):
    """The `.md` spelling of an `.xml` document, or `None` for anything else."""
    return path[:-len(".xml")] + ".md" if path.endswith(".xml") else None


def historical_candidates(live):
    """Every identity `live` could have carried before the two moves, in the order
    a search must try them — most recent shape first.

    1. the live path itself (a revision after the relayout, or a document that
       never moved);
    2. the same document before the relayout, same serialisation (a revision
       between the K-wave conversion and the relayout);
    3. the same document before both moves (`spec/…/X.md`);
    4. the live path with the `.md` spelling — the shape a conversion without a
       relayout would have left, kept so the search does not depend on the two
       moves having landed in the order they happened to land in.

    Deduplicated, order preserved. This function invents nothing: every entry is
    a mechanical rewrite of the live path, and the caller still has to PROVE the
    blob is there before using one.
    """
    out = [live]
    legacy = unrelayout(live)
    if legacy:
        out.append(legacy)
        twin = markdown_twin(legacy)
        if twin:
            out.append(twin)
    twin = markdown_twin(live)
    if twin:
        out.append(twin)
    return list(dict.fromkeys(out))


def map_path(old, exists):
    """`old` → `(live path, kind)`, or `(None, reason)` when it does not land.

    Two steps, in this order and no other: relayout the roots, then — only when
    the relaid path is absent — accept the exact `.xml` twin of a `.md` document.
    The order matters. Trying the twin first would silently prefer XML for a
    document that still ships as Markdown, and this corpus holds 98 of those
    (package `README.md` / `SKILL.md`, two host `README.md`).

    `exists` is injected so the rule can be tested without a tree.
    """
    live = relayout(old)
    if live is None:
        return None, f"names no legacy root (`spec/` or `packages/`): {old}"
    if exists(live):
        return live, "kept"
    if live.endswith(".md"):
        twin = live[:-len(".md")] + ".xml"
        if exists(twin):
            return twin, "xml"
        return None, f"neither `{live}` nor its `.xml` twin exists"
    return None, f"`{live}` does not exist"


def build_map(old_paths, exists):
    """The whole mapping, or a `Refusal` naming every path that did not land.

    Refuses on absence and on collision. The collision is the dangerous case and
    the reason it is checked rather than assumed: two old records landing on one
    live file would merge two verdict maps into whichever the iteration order
    wrote last, and the loss would look exactly like success.
    """
    mapping, problems = {}, []
    for old in old_paths:
        live, why = map_path(old, exists)
        if live is None:
            problems.append(why)
        else:
            mapping[old] = live
    hits = collections.defaultdict(list)
    for old, live in mapping.items():
        hits[live].append(old)
    for live, olds in sorted(hits.items()):
        if len(olds) > 1:
            problems.append(f"collision — {len(olds)} old paths map onto `{live}`: "
                            + ", ".join(sorted(olds)))
    if problems:
        raise Refusal(f"{len(problems)} path(s) do not map one-to-one onto the live tree",
                      problems)
    return mapping


def split_kinds(mapping):
    """`{kind: count}` — how many kept their serialisation and how many became XML."""
    kinds = collections.Counter()
    for old, live in mapping.items():
        kinds["kept" if live.endswith(pathlib.PurePosixPath(old).suffix) else "xml"] += 1
    return kinds
