"""What survived the move, and what moved under it — over two parsed mirrors."""

import collections

from .mapping import DOCUMENT_KEYS, LINE_FRAG
from .mirror import unit_frag
from .refusal import Refusal


class Comparison:
    def __init__(self):
        self.anchors = 0
        self.elements = 0
        self.absent = []      # (old, live, anchor)
        self.duplicated = []  # (old, live, anchor, n)
        self.moved = []       # (old, live, anchor)
        self.historical_gap = []  # (old, live, anchor, why)
        self.structure = []   # (old, live, what, before, after)
        self.doc_hash_changed = 0

    @property
    def structural_failures(self):
        return self.absent + self.duplicated + self.structure

    @property
    def compared(self):
        """Anchors for which BOTH sides yielded exactly one hash — the only ones
        `moved` can speak about. `anchors` counts every judged anchor asked
        about; the difference is the structural failures plus the gaps."""
        return self.anchors - len(self.absent) - len(self.duplicated) - len(self.historical_gap)


def compare_corpus(mapping, cache, old_mirror, new_mirror):
    """Per-file, per-anchor: does every verdict still have somewhere to live?

    Three separate questions, deliberately not merged into one verdict:

    * does the anchor exist at the mapped path, exactly once — structural. A
      failure here means the migration would orphan a verdict, and nothing below
      it matters;
    * does the unit scaffolding still line up — structural, and the precondition
      for remapping the baseline's line addresses by ordinal;
    * is there anything on the PRE-MIGRATION side to compare against — a
      HISTORICAL GAP, and its own class. The anchor is present and unique at the
      target, so no verdict is orphaned and the migration may still proceed; what
      is missing is the ability to say whether the text moved. Folding it into
      "moved" (the shape this code had until B-107 repair 3, via a
      `.get(anchor, [None])[0]` default) reported a fact as content drift on the
      strength of a `None`, and hid the one question an operator would actually
      want asked: why does the judged corpus carry an anchor its own historical
      parse never minted;
    * has the fact's own text moved — NOT structural either. A moved fact keeps
      its verdict and comes due for re-judgement, which is the campaign's
      ordinary business under PROP-043 §10.1. The caller reports moves and gaps
      separately and declines to move state until an operator acknowledges each
      exact count; nothing here re-judges and nothing seals.
    """
    cmp_ = Comparison()
    for old, record in sorted(cache["files"].items()):
        live = mapping[old]
        old_doc, new_doc = old_mirror.doc(old), new_mirror.doc(live)
        old_facts, new_facts = old_mirror.marked_facts(old), new_mirror.marked_facts(live)
        verdicts = (record.get("campaign") or {}).get("verdicts") or {}
        for anchor in sorted(verdicts):
            if anchor in DOCUMENT_KEYS:
                cmp_.elements += 1
                continue
            cmp_.anchors += 1
            got = new_facts.get(anchor, [])
            if not got:
                cmp_.absent.append((old, live, anchor))
                continue
            if len(got) > 1:
                cmp_.duplicated.append((old, live, anchor, len(got)))
                continue
            was = old_facts.get(anchor, [])
            if not was:
                cmp_.historical_gap.append((old, live, anchor,
                                            "absent from the pre-migration parse"))
                continue
            if len(was) > 1:
                cmp_.historical_gap.append((old, live, anchor,
                                            f"minted {len(was)} times in the pre-migration parse"))
                continue
            if was[0] != got[0]:
                cmp_.moved.append((old, live, anchor))
        # Unit alignment is what the baseline's line addresses are remapped
        # through, so it is proven here rather than trusted there.
        if len(old_doc["units"]) != len(new_doc["units"]):
            cmp_.structure.append((old, live, "unit count",
                                   len(old_doc["units"]), len(new_doc["units"])))
        else:
            for i, (a, b) in enumerate(zip(old_doc["units"], new_doc["units"])):
                if a.get("anchor") != b.get("anchor"):
                    cmp_.structure.append((old, live, f"unit[{i}] anchor",
                                           a.get("anchor"), b.get("anchor")))
        if old_doc["content_hash"] != new_doc["content_hash"]:
            cmp_.doc_hash_changed += 1
    return cmp_


# --------------------------------------------------------------------------
# The baseline — `path#anchor` and `path#L<line>` both carry physical identity
# --------------------------------------------------------------------------


def remap_baseline_addr(addr, unit_hash, mapping, old_mirror, new_mirror):
    """One baseline address → its live spelling, and how it was resolved.

    An anchor fragment survives the move verbatim when the live document still
    mints it. A LINE fragment survives nothing: `L38` names a physical line, and
    both the conversion and any edit move it. Three resolutions are tried, in
    falling order of directness, each a fact about the pair of parses rather than
    a guess:

      1. the pre-migration parse has exactly one unit starting at that line →
         its ORDINAL is the identity, and the live unit at the same ordinal is
         the answer (safe only because `compare_corpus` has already proven the
         two unit sequences align one-for-one, anchor by anchor);
      2. the live parse has exactly one unit whose text hashes to the baseline's
         own `unit_hash` — the unit is recognised by its content;
      3. the pre-migration parse has exactly one such unit → its ordinal.

    When none of them fires the address is carried with its path remapped and its
    fragment untouched, and the caller reports it by name. That is the
    "explicitly handled, never dropped" case: the verdict stays in the baseline,
    the address still names the right FILE, and the worst that can happen is the
    unit is re-verified next campaign — which `progress_core` already names as
    the failure mode of every coarse baseline rule.
    """
    path, _, frag = addr.rpartition("#")
    live = mapping[path]
    new_units = new_mirror.units(live)
    line_match = LINE_FRAG.match(frag)
    if not line_match:
        hits = [u for u in new_units if u.get("anchor") == frag]
        return f"{live}#{frag}", ("anchor" if len(hits) == 1 else "unresolved-anchor")
    line = int(line_match.group(1))
    old_units = old_mirror.units(path)
    at_line = [i for i, u in enumerate(old_units) if u["line_start"] == line]
    if len(at_line) == 1:
        return f"{live}#{unit_frag(new_units[at_line[0]])}", "line-by-ordinal"
    by_live = [u for u in new_units if u["content_hash"] == unit_hash]
    if len(by_live) == 1:
        return f"{live}#{unit_frag(by_live[0])}", "line-by-live-hash"
    by_old = [i for i, u in enumerate(old_units) if u["content_hash"] == unit_hash]
    if len(by_old) == 1:
        return f"{live}#{unit_frag(new_units[by_old[0]])}", "line-by-old-hash"
    return f"{live}#{frag}", "unresolved-line"


def resolved_before(addr, old_mirror):
    """Did this address name a live unit in the PRE-migration parse?

    Asked only about addresses that resolve to nothing now, and it is the whole
    difference between "the migration broke this" and "this was already rot": a
    baseline is written weeks before the cache it outlives, so a file
    restructured in between carries dead addresses on its own.
    """
    path, _, frag = addr.rpartition("#")
    units = old_mirror.units(path)
    line_match = LINE_FRAG.match(frag)
    if line_match:
        line = int(line_match.group(1))
        return sum(1 for u in units if u["line_start"] == line) == 1
    return sum(1 for u in units if u.get("anchor") == frag) == 1


def remap_baseline(baseline, mapping, old_mirror, new_mirror):
    """The whole `units` map, re-keyed. Refuses on a key collision."""
    out, how, unresolved = {}, collections.Counter(), []
    for addr, unit in baseline["units"].items():
        new_addr, kind = remap_baseline_addr(addr, unit["unit_hash"], mapping,
                                             old_mirror, new_mirror)
        how[kind] += 1
        if kind.startswith("unresolved"):
            note = "" if resolved_before(addr, old_mirror) else "; already dead pre-migration"
            unresolved.append((addr, new_addr, kind + note))
        if new_addr in out:
            raise Refusal("baseline remap collides",
                          [f"`{addr}` and an earlier address both land on `{new_addr}`"])
        out[new_addr] = dict(unit, addr=new_addr)
    return out, how, unresolved
