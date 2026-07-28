# W4 — campaign-plans, comparative-research, discovery-prompt, redbook: the three sources

_Captured 2026-07-28 at the W4 opening. Every number below is the output of the
command printed above it._

W4 is the batch where §3.1's three sources pull in three different directions, and
the split is worth knowing before the first row is written:

- **`campaign-plans` has the richest source 2 in the cluster after the git family.**
  The host runs campaigns as its primary working form — two ratified plans in
  `spec/terraforms/`, two zones under `campaigns/`, 314 commits between them — so
  every prescription about roles, phase gates, ledgers and reports can be checked
  against a living instance rather than argued about.
- **`comparative-research` has almost no *living* source 2.** The genre is adopted,
  but every instance is either in `legacy-spec/` (an archive) or in the fractality
  specspace under a different document shape. This is a **non-adoption** shape, and
  the batch-plan line applies: non-adoption is not drift.
- **`discovery-prompt` deliberately has none.** Its own boot snippet forbids loading
  the artefact outside a deployment request, so "is it in use here?" is the wrong
  question. Its facts are about what the package ships and what the snippet says.
- **`redbook` is an umbrella, and its source-1 half is arithmetic** — the member
  list against the pin list against `vibedeps/`. Two of those three disagree, and
  the disagreement is measured below.

## Source 1 — the package agreeing with itself {#source-1}

```console
$ python campaigns/packages-2026-09/tasks/source1-join.py \
    packages/org.vibevm.world/campaign-plans \
    packages/org.vibevm.world/comparative-research \
    packages/org.vibevm.world/discovery-prompt \
    packages/org.vibevm.world/redbook
source-1 join over 31 file(s) under packages/org.vibevm.world/campaign-plans, packages/org.vibevm.world/comparative-research, packages/org.vibevm.world/discovery-prompt, packages/org.vibevm.world/redbook
  relative .md citations resolved: 37
  broken: 2

  MISSING FILE: 2

  MISSING FILE    packages/org.vibevm.world/redbook/v0.1.0/spec/book/ru/chapter-1-two-process-model.md
                    -> safeharbor.md
  MISSING FILE    packages/org.vibevm.world/redbook/v0.2.0/spec/book/ru/chapter-1-two-process-model.md
                    -> safeharbor.md
```

**Thirty-seven relative citations, two broken, and both are the same already-filed
finding.** F-119: the book's chapter 1 cites `safeharbor.md`, which exists nowhere,
in both redbook slots. It is invisible to the campaign's own gate because the
`exclude` globs drop `redbook/*/spec/book/ru/` — which is also why neither chapter
file appears in W4's fifteen. Prior batches: W1 11 clean, W2 23 clean, W3 24 clean.
**W4 is the first world batch whose source-1 join is not clean.**

### redbook's member list against its own pin list {#redbook-arithmetic}

```console
$ grep -cE '^"flow:org\.vibevm\.world/' packages/org.vibevm.world/redbook/v0.2.0/vibe.toml
22
$ grep -cE '^- ##MEMBER-' packages/org.vibevm.world/redbook/v0.2.0/spec/boot/03-flow-redbook.md
23
```

The boot snippet's member list and the manifest's pin list are **not the same set**:

```console
$ grep -oE '^- ##MEMBER-[A-Z-]+ `[a-z-]+`' packages/org.vibevm.world/redbook/v0.2.0/spec/boot/03-flow-redbook.md \
    | grep -oE '`[a-z-]+`$' | tr -d '\140' | sort > /tmp/named.txt
$ grep -oE '^"flow:org\.vibevm\.world/[a-z-]+' packages/org.vibevm.world/redbook/v0.2.0/vibe.toml \
    | sed 's|.*/||' | sort > /tmp/pinned.txt
$ comm -23 /tmp/named.txt /tmp/pinned.txt          # named in the snippet, not pinned
git-atomic-commits
git-attribution-policy
$ comm -13 /tmp/named.txt /tmp/pinned.txt          # pinned in the manifest, not named
git-practices
```

The manifest carries the reconciliation as a comment —
`# attribution-policy now arrives via git-practices (§12.1 human-authored)` — and
pins the umbrella; the snippet names the two members the umbrella pulls and never
names the umbrella. Both are defensible individually; **they are not the same
list**, and any fact asserting "the umbrella pins every member exactly" is judged
against that.

All 22 pins are installed:

```console
$ for m in $(pins); do test -d "vibedeps/flow-$m" || echo "NOT INSTALLED: flow-$m"; done
    missing: 0 of 22
```

## Source 3 — the installed reality {#source-3}

```console
$ python campaigns/packages-2026-09/tasks/source23-boot-join.py
boot-lane join over 31 contribution(s) in spec/boot/STATIC.md
  installed, sourced, same word stream: 17
  problems: 14

  org.vibevm.world/campaign-plans  [INSTALLED SOURCED WORDS-DIFFER]
    installed: vibedeps/flow-campaign-plans/0.1.0/spec/boot/40-flow-campaign-plans.md
    source   : packages/org.vibevm.world/campaign-plans/v0.1.0/spec/boot/40-flow-campaign-plans.md
    package 441 words, host 435 — 6 differ
    only in the package: cold facts verified at writing time

  org.vibevm.world/comparative-research  [INSTALLED SOURCED WORDS-DIFFER]
    installed: vibedeps/flow-comparative-research/0.1.0/spec/boot/52-flow-comparative-research.md
    source   : packages/org.vibevm.world/comparative-research/v0.1.0/spec/boot/52-flow-comparative-research.md
    package 314 words, host 311 — 3 differ
    only in the package: sibling document pointers

  org.vibevm.world/redbook  [INSTALLED SOURCED WORDS-DIFFER]
    installed: vibedeps/flow-redbook/0.2.0/spec/boot/03-flow-redbook.md
    source   : packages/org.vibevm.world/redbook/v0.2.0/spec/boot/03-flow-redbook.md
    package 519 words, host 513 — 6 differ
    only in the package: spirit source member list git git
```

**Three of W4's four slots are on the join's problem list; `discovery-prompt` is the
clean one** — INSTALLED, SOURCED, and word-identical at 251 = 251.

### Two of those three differences are the instrument, not the corpus {#instrument}

The join's `strip_markup` removes a fact anchor only when a space or tab follows it
(`##[A-Za-z][A-Za-z0-9_-]*[ \t]+`). **An anchor alone on its line survives the
strip and is counted as prose.** Both of these are that:

```console
$ grep -n 'COLD-FACTS-VERIFIED-AT-WRITING-TIME' packages/org.vibevm.world/campaign-plans/v0.1.0/spec/boot/40-flow-campaign-plans.md
35:- ##COLD-FACTS-VERIFIED-AT-WRITING-TIME
$ grep -n '##sibling-document-pointers' packages/org.vibevm.world/comparative-research/v0.1.0/spec/boot/52-flow-comparative-research.md
13:##sibling-document-pointers
```

`campaign-plans`' six words are `cold facts verified at writing time` — the anchor
above, plus `{#cold}`, both heading anchors being identical on the two sides.
`comparative-research`' three are `sibling document pointers`. **Neither is a prose
difference. Do not write a drift row on either.**

### redbook's difference IS a prose difference, and it is measured {#redbook-drift}

```console
$ python - <<'PY'  # host boot lane vs package source, member names only
host boot lane names 23 members; package source names 23
  in package, NOT in host lane: ['git-atomic-commits', 'git-attribution-policy']
  in host lane, NOT in package: ['atomic-commits', 'attribution-policy']
```

The installed 0.2.0 copy (mtime 2026-07-15) names two members by short, unqualified
names; the package source (mtime 2026-07-28) names them qualified. The **qualified
names are the real ones** — `spec/boot/INDEX.md` and the provenance markers in
`spec/boot/STATIC.md` both read `org.vibevm.world/git-atomic-commits` and
`org.vibevm.world/git-attribution-policy`, and `vibedeps/flow-atomic-commits/` does
not exist. **So the host's compiled boot lane names two members by names that no
package in the registry carries.** One `name@version`, two contents — F-122's
family, with a semantic difference rather than a markup one.

### What the four slots actually ship {#payload}

```console
$ find vibedeps/flow-discovery-prompt -type f
  vibedeps/flow-discovery-prompt/0.1.0/LICENSE.md
  vibedeps/flow-discovery-prompt/0.1.0/README.md
  vibedeps/flow-discovery-prompt/0.1.0/spec/boot/50-flow-discovery-prompt.md
  vibedeps/flow-discovery-prompt/0.1.0/spec/flows/discovery-prompt/DISCOVERY-PROMPT.md
  vibedeps/flow-discovery-prompt/0.1.0/spec/flows/discovery-prompt/usage.md
  vibedeps/flow-discovery-prompt/0.1.0/vibe.toml
```

**The artefact ships.** A consumer receives `DISCOVERY-PROMPT.md` itself, not just
prose about it.

### The sibling-pointer family, per file {#dangling}

```console
$ for f in <the four boot snippets>; do grep -oE '\.\./flows/[^)]*' "$f" | wc -l; done
  campaign-plans/…/40-flow-campaign-plans.md              3
  comparative-research/…/52-flow-comparative-research.md  3
  discovery-prompt/…/50-flow-discovery-prompt.md          3
  redbook/…/03-flow-redbook.md                            0
$ ls spec/          # does the host have spec/flows/ at all?
WAL.md  boot  common  design  manual-tests  modules  terraforms
```

**The host has no `spec/flows/` directory.** Nine `../flows/…` pointers across three
of W4's boot snippets resolve nowhere in the consuming project — W1's 69-dangling
finding, now in its eighth package. **`redbook` is the exception with zero**, which
is the first W4 fact to check rather than assume.

## Source 2 — the host's observed conformance {#source-2}

### campaign-plans — the host is a heavy, measurable consumer {#s2-campaigns}

```console
$ ls spec/terraforms/
PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md
SPEC-ACTUALIZATION-CAMPAIGN-v0.1.md
$ ls campaigns/
packages-2026-09
progress-2026-08
```

**Both plans carry all five roles the flow names**, as top-level sections:

| role | wave 2 (`PACKAGES-…`) | wave 1 (`SPEC-…`) |
|---|---|---|
| mandate, verbatim + dated | `## 0. Mandate … 2026-07-25 … recorded verbatim` | `## 0. Mandate … 2026-07-24 …` |
| BASELINE | `## 1. Baseline (verified at authoring time, 2026-07-25)` | `## 1. Baseline (verified at authoring time, 2026-07-24)` |
| PLAN (phases) | `## 5. Phases` | `## 5. Phases` |
| PREDICTIONS | `## 6. Predictions (falsifiable, campaign-wide)` | `## 8. Predictions (falsifiable, campaign-wide)` |
| LOG | `## 7. LOG` | `## 9. LOG (execution ledger — append per batch/wave/phase)` |
| deferrals | `## 8. Deferrals` | `## 10. Deferrals` |
| REPORT | `## 9. REPORT` *(empty — campaign live)* | `## 11. REPORT` *(filled 2026-07-26)* |

**Wave 1's REPORT scores every prediction individually**, and scores one of them
against the process rather than the number:

> `spec/terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.md:1314` — *A prediction that no
> step of the plan forces you to run is one you will reach the end of the campaign
> without having tested.*

**Wave 2 adopted A5 in response**, and §6's six predictions each name their testing
step (`**Tested by:** Phase C's exit gate (ii)`, `**Tested by:** Phase A step 3's
pilot`, …). That is the flow's "falsifiable expectations" role in its strongest
observed form anywhere in the repository.

**The commit-map is where the ledger thins out.** The flow's `#boundary` step 2 asks
for the phase's commit-map entry — hashes, subjects, what each commit confirmed or
falsified:

```console
$ git log --oneline -- campaigns/packages-2026-09 | wc -l
189
$ grep -oE '\b[0-9a-f]{8}\b' spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md | sort -u | wc -l
17
$ git log --oneline -- campaigns/progress-2026-08 | wc -l
125
$ grep -oE '\b[0-9a-f]{8}\b' spec/terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.md | sort -u | wc -l
20
```

**Wave 2: 17 unique hashes cited against 189 commits in the zone (9 %). Wave 1: 20
against 125 (16 %).** The LOG is dense in prose and thin in hashes; the entries that
do carry them carry subjects and outcomes too. Note the counts are not
apples-to-apples — a phase's commit map need not name every commit that touched the
zone — but the flow asks for the map at every boundary, and the boundaries are many.

**The status line is stale by three phases.** The flow's `#boundary` step 3 asks for
"Phase N landed, floor green, next: Phase N+1":

```console
$ sed -n 5p spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md
**status: RATIFIED 2026-07-26 · PHASE A OPEN · all six [§4.5](#amendments) amendments adopted · …**
$ python campaigns/packages-2026-09/tasks/summary.py | tail -2
  ALL                    confi  8523  drift   347  unver    40   total  8910    95.7 %
```

The plan says **PHASE A OPEN**; Phases A and B are closed and C is at 64.4 %.

**Review points: the two waves diverge sharply.**

```console
$ grep -ciE 'review point' spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md
0
$ grep -ciE 'review point' spec/terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.md
8
$ grep -nE '\*\*(OPEN|RESOLVED)' spec/terraforms/SPEC-ACTUALIZATION-CAMPAIGN-v0.1.md | wc -l
6
```

Wave 1 carries six `**RESOLVED …**` entries with the owner's ruling quoted
verbatim, dated, several marked "in session". **Wave 2 carries zero of either
form** — its owner rulings are recorded in the LOG's prose and in the batch plan's
four-field decisions instead. Whether the flow's OPEN→RESOLVED shape is required or
merely offered is a per-fact reading; the counts are here either way.

**Phase 0 committed.** The flow's `#never` says spikes leave findings, not tree
changes:

```console
$ git log --oneline --all --grep='C0' -i | head -1
a90cc387 chore(campaign): C0 closes the host debt, and the merge law holds
```

Read the boundary carefully before writing a row: `C0` is a **batch inside Phase C**
that the batch plan titles "spike" (`### C0 — spike: the harvest, and a control on
the mechanism`), not the campaign's Phase 0. The flow's rule names Phase 0. Both
readings are on the table and the reviewer settles it.

**Gate panel at boundaries** — mentioned 5 times in wave 2's plan and 11 in wave 1's;
the wave-2 instances read `floor green on the committed tree`, `floor green on 25
steps`, `tools/self-check.sh steps 7 and 9 gated`.

**Deferrals ledgers exist in both zones** (`campaigns/progress-2026-08/deferrals.md`,
131 lines; `campaigns/packages-2026-09/deferrals.md`), and both plans' in-document
`## Deferrals` sections are empty and say why — wave 1's reads
`*(empty — drained into campaigns/<id>/deferrals.md at close-out)*`.

### comparative-research — adopted, then archived {#s2-comparative}

The genre is named and used by name in the host, and **every instance is outside the
living spec tree**:

```console
$ grep -rlniE 'comparative.research genre|two-way gap' spec/ docs/
spec/boot/STATIC.md
```

The only hit in `spec/` is the compiled boot lane — the flow's own snippet. The
instances are here:

| document | lines | genre markers |
|---|---:|---|
| `legacy-spec/research/PROP-004-tessl-comparative-research.md` | 963 | gap analysis, roadmap deltas, "Captured 2026-05-04 against Tessl CLI 0.78.0" |
| `legacy-spec/research/action-systems-vscode-idea.md` | 782 | `## 5. Two-way gap analysis {#gaps}`, DO1–DO18, 16 deltas |
| `legacy-spec/research/settings-system-vscode-idea.md` | 382 | `## 6. Two-way gaps (где vibevm отстаёт / где может вести)` |

`legacy-spec/research/ACTION-SYSTEM-RESEARCH-PLAN-v0.1.md:307` describes the genre
in the flow's own vocabulary — *"quote-first with access dates; two-way gap analysis
… numbered roadmap deltas each naming a target Spec-1 REQ; a re-fetch/provenance
table"* — so the practice was consciously adopted, and then the whole tree became
an archive. **`legacy-spec/` is an archive: nothing living may rest on it as a
normative source, and that is itself the finding, not evidence of conformance.**

**A second, living body of external-system study exists and is a different genre.**
The `fractality` specspace carries 15 study notes:

```console
$ ls packages/org.vibevm.fractality/fractality/v0.1.0/spec/refs/notes/*-study.md | wc -l
15
$ grep -nE '^#{1,3} ' packages/org.vibevm.fractality/fractality/v0.1.0/spec/refs/notes/rlm-study.md
1:# Study note — Recursive Language Models (the anchor project) {#root}
10:## What it is {#what}
33:## Verified facts that matter to us {#facts}
91:## Decisions we take (→ synthesis deltas) {#decisions}
137:## Open questions carried to synthesis {#open}
$ grep -c 'accessed\|Accessed' packages/org.vibevm.fractality/fractality/v0.1.0/spec/refs/notes/*.md | grep -c ':0$'
21          # of 22 .md files in that directory; only rlm-source-selection.md carries any
```

No two-way gap section, no per-quote access dates inside the notes. The access dates
live once, out of band:

```console
$ head -2 refs/articles/ACCESS-DATES.txt
accessed: 2026-07-10
fugu pages accessed: 2026-07-11
```

**Widen to `packages/org.vibevm.fractality/**` for this package** — it is not on the
brief's default perimeter and it is where the living practice is. Exclude the nested
`packages/org.vibevm.fractality/*/*/vibedeps/**` and `.vibe/cache/**` copies: those
are the specspace's own installed dependencies, i.e. copies of the very flows under
verification, and counting them counts the subject as its own consumer.

### discovery-prompt — the artefact is present and its non-use is by design {#s2-discovery}

```console
$ grep -rl 'PrimaryHypothesis' --include='*.md' . | grep -vE '^\./vibedeps/|\.vibe/cache'
./legacy-spec/research/DISCOVERY_PROMPT.md
./packages/org.vibevm.fractality/…/vibedeps/flow-discovery-prompt/0.1.0/…   (4 files, nested install)
./packages/org.vibevm.world/discovery-prompt/v0.1.0/…                       (4 files, the package)
./spec/boot/STATIC.md
$ diff legacy-spec/research/DISCOVERY_PROMPT.md packages/org.vibevm.world/discovery-prompt/v0.1.0/spec/flows/discovery-prompt/DISCOVERY-PROMPT.md
  IDENTICAL   (23 479 bytes, modulo CRLF)
```

**The host carries one unedited copy of the artefact, in the archive**, byte-identical
to the shipped one. That is consistent with the flow's `#never` ("never edit the
artifact in place; adapt a copy") and consistent with the snippet's own instruction
not to load it. **The absence of deployment traces is the flow working as specified,
not the flow being ignored** — judge these facts on what they prescribe, per §judged.

### redbook — the umbrella, the book, and what the gate cannot see {#s2-redbook}

```console
$ ls packages/org.vibevm.world/redbook/
v0.1.0
v0.2.0
$ find packages/org.vibevm.world/redbook/v0.2.0 -type f
  …/LICENSE.md  …/README.md  …/vibe.toml
  …/spec/boot/03-flow-redbook.md
  …/spec/book/README.md
  …/spec/book/ru/chapter-1-two-process-model.md
  …/spec/book/ru/chapter-2-shared-state-and-files.md
  …/spec/book/ru/chapter-3-memory-individual.md
```

**The book ships: three Russian chapters plus an edition-plan README**, exactly as
the snippet claims. `v0.1.0` is the superseded slot — §3.3 marks it, never verifies
it, so it is out of W4's fifteen. The three chapter files are also out of the
fifteen: the campaign's `exclude` globs drop `redbook/*/spec/book/ru/`, which is why
F-119's dangling `safeharbor.md` had to be found by the source-1 join.

**W4's two files for this package are `README.md` (44 anchors) and the boot snippet
(39).** Its facts are mostly about the collection's own composition, so source 1 and
the manifest carry more of the weight than in any other W4 package — and the
composition arithmetic is §source-1 above.

## The fifteen files and their anchor counts {#files}

Measured from `campaigns/packages-2026-09/run/mirror/`; the batch total agrees with
`tasks/PHASE-C-BATCHES.json` (`W4 … 15 files, 651 markers, 564 anchors`).

```
campaign-plans (218)
  23  packages/org.vibevm.world/campaign-plans/v0.1.0/README.md
  29  …/campaign-plans/v0.1.0/spec/boot/40-flow-campaign-plans.md
  64  …/spec/flows/campaign-plans/CAMPAIGN-PLAN-FORMAT.md
  48  …/spec/flows/campaign-plans/execution-ledger.md
  54  …/spec/flows/campaign-plans/phase-gates.md
comparative-research (180)
  21  packages/org.vibevm.world/comparative-research/v0.1.0/README.md
  12  …/spec/boot/52-flow-comparative-research.md
  60  …/spec/flows/comparative-research/COMPARATIVE-RESEARCH-PROTOCOL.md
  61  …/spec/flows/comparative-research/from-research-to-roadmap.md
  26  …/spec/flows/comparative-research/research-template.md
discovery-prompt (83)
  19  packages/org.vibevm.world/discovery-prompt/v0.1.0/README.md
  10  …/spec/boot/50-flow-discovery-prompt.md
  54  …/spec/flows/discovery-prompt/usage.md
redbook (83)
  44  packages/org.vibevm.world/redbook/v0.2.0/README.md
  39  …/spec/boot/03-flow-redbook.md
```

`discovery-prompt/…/DISCOVERY-PROMPT.md` ships but carries no addressable anchors —
it is the payload, not a contract, and it has no mirror file.

**Scope:** §3.1 sources 1, 2 and 3 for the four flows of batch W4.
