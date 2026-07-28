# W3 — addressable-specs, decision-records, conflict-protocol: the three sources

_Captured 2026-07-28 at the W3 opening. Every number below is the output of the
command printed above it._

W3 is the batch where §3.1's source 2 is at its most quantitative anywhere in
`world`: these three flows specify a citation grammar, a record format and a
conflict hierarchy, and the host uses all three at scale in files that can be
counted.

## Source 1 — the package agreeing with itself {#source-1}

```console
$ python campaigns/packages-2026-09/tasks/source1-join.py \
    packages/org.vibevm.world/addressable-specs \
    packages/org.vibevm.world/decision-records \
    packages/org.vibevm.world/conflict-protocol
source-1 join over 18 file(s) under packages/org.vibevm.world/addressable-specs, packages/org.vibevm.world/decision-records, packages/org.vibevm.world/conflict-protocol
  relative .md citations resolved: 24
  broken: 0
```

**Twenty-four relative citations, none broken** — the largest clean count of any
world batch so far (W1: 11, W2: 23). The mechanical half of source 1 is clean.

## Source 3 — the installed reality {#source-3}

```console
$ python campaigns/packages-2026-09/tasks/source23-boot-join.py
  (…none of the three W3 slots appears on the join's problem list…)
$ grep -n 'vibe:static org.vibevm.world/\(addressable-specs\|decision-records\|conflict-protocol\)' spec/boot/STATIC.md
5:<!-- vibe:static org.vibevm.world/addressable-specs — vibedeps/flow-addressable-specs/0.1.0/spec/boot/15-flow-addressable-specs.md -->
174:<!-- vibe:static org.vibevm.world/conflict-protocol — vibedeps/flow-conflict-protocol/0.1.0/spec/boot/35-flow-conflict-protocol.md -->
235:<!-- vibe:static org.vibevm.world/decision-records — vibedeps/flow-decision-records/0.1.0/spec/boot/25-flow-decision-records.md -->
```

**All three are INSTALLED, SOURCED and word-identical.** None appears on the
join's problem list — unlike W2, where `two-process-model` was WORDS-DIFFER (three
`{#…}` heading anchors lost to a stale install) and `sync-from-code` NO-SOURCE
(installed at the pre-DRIFT-039 `boot/` path). W3's three slots are the clean
case, and `addressable-specs` is the **first** contribution in the compiled lane,
at line 5.

**Expect the sibling-pointer family anyway.** Each of the three boot snippets ends
with `../flows/<name>/<file>.md` links, and the host has no `spec/flows/` at all —
that is W1's 69-dangling finding and it has been drift in every batch so far. It
is a fact about the pointer, not about the rule the pointer sits under.

## Source 2 — the host's observed conformance {#source-2}

The consuming project uses all three at scale.

```console
$ grep -c 'spec://' spec/common/*.md spec/modules/**/*.md   # summed
117
$ grep -rn 'REVIEW:' CLAUDE.md spec/boot/00-core.md spec/WAL.md | wc -l
1
```

**`spec://` URIs occur 117 times in the host's own contract tree**, so
`addressable-specs`' citation grammar is in daily use — but note the counter-fact
already measured in W2a: **`spec://` occurs 0 times in `spec/WAL.md`**, the one
file whose own flow requires spec anchors on every constraint. Both numbers matter
and they point opposite ways.

**The REVIEW-marker contract has almost no host instances.** One hit across
`CLAUDE.md`, `spec/boot/00-core.md` and `spec/WAL.md`, and it is inside the
uncertainty ladder that *prescribes* the marker (`00-core.md:60`) rather than a
marker in use. `conflict-protocol`'s REVIEW contract is therefore installed,
restated by the host, and — on this evidence — never exercised. Widen before
concluding: the marker may live in `spec/common/`, `spec/modules/` or the crates.

**Three host surfaces bear on `decision-records` and each must be searched, not
assumed.** The flow demands a four-field record — Decision / Why / Considered and
rejected / When to revisit — at the anchor that governs the value, and forbids a
record with a missing reason or a missing revisit trigger. Candidate sites: the
`## 2. Three decisions taken at the opening` block of
`campaigns/packages-2026-09/PHASE-C-BATCH-PLAN.md` (which does carry all four
fields), the `Decision / Why / Considered and rejected / Revisit when` blocks in
`spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md` §3, and the PROP
documents under `spec/common/`. **A already-recorded counter-instance:** W2c found
that `two-process-model`'s own `RE-DERIVE-THE-SPLIT-WHEN-CAPABILITIES-MOVE`
delegates its revisit to this flow and supplies no measurable trigger.

**The conflict hierarchy is installed verbatim and restated by the host in its own
vocabulary** — `spec/boot/00-core.md:38-45` calls the reading layers «vibevm's
instance» of the two-process model and orders Head > WAL > Spec > Code, which is a
*different order* from `Human > Spec > Tests > Code > WAL`. That divergence is real
and belongs to whichever fact asserts the ordering; do not smooth it over.

**Scope:** §3.1 sources 1, 2 and 3 for the three flows of batch W3.
