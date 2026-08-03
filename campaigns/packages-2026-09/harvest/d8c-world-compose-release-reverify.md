# D8c — re-verifying the `release`-route composition claims and the tail of the release queue

_Phase D, wave 8, batch D8c. Five obligations over ten anchors in
`org.vibevm.world` packages — the three composition claims of
[release queue §C](../PHASE-D-RELEASE-QUEUE.md#composes), the arithmetic row of
[§D](../PHASE-D-RELEASE-QUEUE.md#arithmetic), and the root-relative address
variant `F-240` of [§A](../PHASE-D-RELEASE-QUEUE.md#addresses). Every one closes
through [`release`](../PHASE-D-BATCH-PLAN.md#routes), where **the owner approves
before publication** — so a re-verdict that edits nothing produces no spec diff
and needs no approval, while a correction does. This file is evidence and a
recommendation; **no package file was edited, no campaign state was written, and
no verdict JSON was touched.** The verdict itself is the boss's._

**Measured at** `HEAD = f2b11b0a` (`fix(campaign): the registry snapshot on disk
was two waves stale, and it read as open work`, 2026-07-31), taken with
`git rev-parse --short HEAD`. Every count below names the command that produced
it, per [§6.1's](../PHASE-D-BATCH-PLAN.md#delegation-lessons) rule that a figure
over `git log` names its HEAD and a figure quoted from an earlier wave is
re-measured rather than carried — this batch's `F-219` carries a count already
restated once from 515 to 716, and it is re-measured here from scratch.

**Route check, run first per [`##ROUTE-BEFORE-FALSIFIER`](../PHASE-D-BATCH-PLAN.md#delegation-lessons).**
All five report `closure_route: release`, read from `run/state/obligations.json`
as an instrument:

```
F-219 | reality-mismatch | release | mixed | open | 2 anchors
F-220 | reality-mismatch | release | mixed | open | 2 anchors
F-233 | reality-mismatch | release | mixed | open | 2 anchors
F-240 | relocation       | release | mixed | open | 2 anchors
F-251 | reality-mismatch | release | mixed | open | 2 anchors
```

**The standing perimeter.** Unless an entry narrows it, every search was run from
the repository root over: `packages/**` **including
`packages/org.vibevm.fractality/**`** (a second complete adopter project with its
own `vibe.toml`, `vibedeps/` and plans —
[§3.7's wave-6 extension](../PHASE-D-BATCH-PLAN.md#compliance-blindness)),
`vibedeps/**`, `crates/**`, `xtask/**`, `tools/**`, `spec/**`, `discipline/**`,
`terraform/**`, `research/**`, `campaigns/**` minus `campaigns/*/run/**`,
`fixtures/**`, `schemas/**`, `docs/**`, `manual-tests/**`, and the repository
root's own `*.md` / `*.toml` / `*.json` / `*.sh` / `*.ps1`. **Excluded:**
`legacy-spec/**` (owner ruling 2026-07-31 — not evidence of practice in either
direction), `**/target/**`, `.git/**`, `**/node_modules/**`,
`campaigns/*/run/**`. `refs/**` is third-party and is searched but reported
separately.

**Evidence source classes**, tagged on every evidence line per
[§3.1](../../spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md#world-verdicts):
**[1]** the package's own shipped artifacts · **[2]** the host's observed
conformance · **[3]** the installed reality (`vibe.lock`, `vibedeps/`,
`files_written`).

**The inside-own-corpus trap, applied throughout.** Per
[`##THE-CAMPAIGN-IS-INSIDE-ITS-OWN-CORPUS`](../PHASE-D-BATCH-PLAN.md#delegation-lessons),
every count over `campaigns/**` or `spec/terraforms/**` was opened and each hit
confirmed to be an **instance** of the form rather than this campaign's prose
**about** the finding. Where campaign prose matched, it is counted separately and
named.

---

## F-219 — the misattribution is real in one package and the other anchor is a different sentence entirely

**Outcome:** SPLIT — 1 **STANDS-RESTATED** (addressable-specs; the reason's
central count is wrong by method and the family precedent is already closed) ·
1 **FALLS** (campaign-plans; the anchor makes no attribution claim at all, and
all three of its clauses have live instances the verdict's perimeter could not
see).

**Anchors:** 2 of 2. **They are not the same sentence**, and the registry's
`reason` field carries only the addressable-specs verdict — the two rows were
merged by [§2.2's third signal](../PHASE-D-BATCH-PLAN.md#signals) («one anchor
drifting in two or more packages»), which keys on the anchor **name**. The
campaign-plans row carries its own, entirely different reason, read from the
cache as an instrument.

### The count first: re-measured at HEAD, and the restated figure is a different quantity

The registry reason says «515 commits»; the release queue
[§C](../PHASE-D-RELEASE-QUEUE.md#composes) restates it as «**716** commit bodies
cite a `spec://` URI at HEAD». **Both are commit counts by their own wording, and
716 is not one.** Measured at `HEAD = f2b11b0a` [2]:

```console
$ git rev-parse --short HEAD
f2b11b0a

$ git log --oneline | wc -l
2212

$ git log --grep="spec://" --oneline | wc -l          # COMMITS whose message cites a spec:// URI
579

$ git log --grep="spec://" -F --oneline | wc -l       # same, fixed-string, to rule out regex effects
579

$ git log --format=%B | grep -c "spec://"             # LINES across all commit messages
732

$ git log --format=%B | grep -o "spec://" | wc -l     # OCCURRENCES
749
```

A commit count cannot fall, so 716 > 579 is by itself proof the two figures
measure different things. Reproduced at the commit that recorded 716 —
`596588fb` (`docs(campaign): two more queue figures restated…`, 2026-07-31,
found with `git log --oneline -S"716" -- campaigns/packages-2026-09/PHASE-D-RELEASE-QUEUE.md`):

```console
$ git log --grep="spec://" --oneline 596588fb | wc -l
565
$ git log --format=%B 596588fb | grep -c "spec://"
718
```

**565 commits, 718 lines.** The recorded «716 commit bodies» is the *line* count,
within two of the value that command returns, and the *commit* figure at that
same HEAD was 565. The honest sentence at `f2b11b0a` is **579 of 2 212 commits
(26.2 %) cite a `spec://` URI in their message; the practice produces 732 such
lines**. The finding does not rest on the count either way — but the queue's
restatement replaced a stale number with a wrong-unit one, and it should not be
carried a third time.

### Anchor 1 — `addressable-specs/v0.1.0/README.md#COMPOSES-ATOMIC-COMMITS` → **STANDS-RESTATED**

**Current text at HEAD** (`README.md:64-65`, inside `## Composition {#composition}`) [1]:

```
- ##COMPOSES-ATOMIC-COMMITS `flow:git-atomic-commits` — commit bodies cite `spec://` URIs; this
  package defines what those URIs resolve to. @impl/done
```

**Capability / practice / rule ([§6.1](../PHASE-D-BATCH-PLAN.md#delegation-lessons)):**
neither a capability nor a consumer-practice claim. It is an **attribution** — a
composition row that names `flow:git-atomic-commits` as the flow under which
commit bodies cite `spec://` URIs. So §3.6(b) does not apply: nothing here
depends on what a consumer does, and the sentence is falsifiable entirely inside
`packages/` (`falsifier: self` in substance, whatever the row records).

**The rule lives in the sibling, and the sibling's snippet is addressable proof** [1]:

```console
$ grep -rn "spec://" packages/org.vibevm.world/git-conventional-commits/
…/spec/boot/31-flow-conventional-commits.md:24:##CITE-SPEC-URIS-WHERE-RELEVANT Cite `spec://…` URIs where relevant. @impl/done
…/spec/flows/conventional-commits/conventional-commits.md:75:  measurement, or conversation that drove it. Use `spec://…` URIs
…/spec/flows/conventional-commits/conventional-commits.md:142:Cited by spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-001#freshness.
```

The verdict's `31-flow-conventional-commits.md:24` citation is **exact**. The
full rule is at `conventional-commits.md:74-77` [1]:

```
- ##INCLUDE-WHY-THIS-CHANGE-WAS-MADE **Why this change was made.** Link to the spec section, issue,
  measurement, or conversation that drove it. Use `spec://…` URIs
  so future sessions can follow the reference without having to
  guess where it is documented. @impl/done
```

**`git-atomic-commits` does not carry it, and explicitly disclaims the class** [1]:

```console
$ grep -rn "spec://" packages/org.vibevm.world/git-atomic-commits/
…/spec/boot/30-flow-atomic-commits.md:24:`spec://org.vibevm.world/git-conventional-commits/flows/conventional-commits/conventional-commits#root`. @impl/done
…/spec/flows/atomic-commits/ATOMIC-COMMITS-PROTOCOL.md:78:(`spec://org.vibevm.world/decision-records/flows/decision-records/DECISION-RECORDS-PROTOCOL#root`)
…/spec/flows/atomic-commits/splitting-large-changes.md:96:`git-conventional-commits` flow: `spec://org.vibevm.world/git-conventional-commits/flows/conventional-commits/conventional-commits#root`). @impl/done
```

**All three are pointers to siblings; none is a citation rule.** And the snippet
draws the line the composition row erases:

```
21: ## Message format {#message-format}
23: ##COMMIT-MESSAGES-FOLLOW-THE-CONVENTIONAL-COMMITS-FLOW Commit messages follow the **git-conventional-commits** flow — a sibling package:
26: ##CONVENTIONAL-COMMITS-IS-THE-FORMAT-THIS-FLOW-IS-THE-ATOMICITY Conventional Commits is the *format*; this flow is the *atomicity* (one commit, one idea). @impl/done
```

**Correction owed to the reason, (a):** the verdict cites this at
`30-flow-atomic-commits.md:22`. At HEAD, `:22` is a **blank line**; the section
head is `:21` and the line-drawing sentence is `##CONVENTIONAL-COMMITS-IS-THE-FORMAT-THIS-FLOW-IS-THE-ATOMICITY`
at **`:26`**. (In the installed copy [3] the same sentence sits at
`vibedeps/flow-git-atomic-commits/0.1.0/boot/30-flow-atomic-commits.md:19` — the
package/install line offset is the known Phase B markup difference,
[§3.5](../PHASE-D-BATCH-PLAN.md#vendored), and carries no drift signal.)

**«Both flows are installed» — verified** [3]:

```console
$ grep -n "git-atomic-commits\|git-conventional-commits" vibe.lock
246:name = "git-atomic-commits"
276:name = "git-conventional-commits"
293:    "flow:org.vibevm.world/git-atomic-commits@=0.1.0",
296:    "flow:org.vibevm.world/git-conventional-commits@=0.1.0",
```

**Read further before searching wider
([`##READ-FURTHER-BEFORE-SEARCHING-WIDER`](../PHASE-D-BATCH-PLAN.md#delegation-lessons)):**
`addressable-specs`' own `## Composition` section (`README.md:61-72`) lists
`two-process-model`, `git-atomic-commits`, `conflict-protocol`, `wal`,
`decision-records` — **`git-conventional-commits` is not in it.** So the README
does not merely name a second true sibling; it routes this exact fact to the one
sibling that disclaims it and omits the one that owns it. Nothing in the ±15
lines rescues the reading.

**Correction owed to the reason, (b) — and it is the strongest evidence here.**
The reason ends «*Same defect W2d found in sync-from-code's
BOUNDARY-FLOW-ATOMIC-COMMITS*». That sentence is **`confirmed` at HEAD**,
because it was already repaired — cache, read as an instrument: `v: confirmed`,
«*CLOSED F-253 (Phase D wave 3, route a … The boundary note credited Conventional
Commits to `flow:git-atomic-commits`, which disclaims exactly that in its own
boot snippet … The same misattribution W2 found from the other side at
addressable-specs' COMPOSES-ATOMIC-COMMITS*)». Its current text [1]:

```
144: - ##BOUNDARY-FLOW-ATOMIC-COMMITS **`flow:git-atomic-commits`** handles commit discipline: one sync,
145:   one commit, one logical idea. The message *format* — Conventional
146:   Commits, with `docs(spec)` as the type a sync commit carries — is
147:   defined by the sibling `flow:git-conventional-commits`, not by the
148:   atomicity flow and not here. @impl/done
```

So the cross-reference is **stale as written** (it points at a sentence that no
longer carries the defect) and **decisive as evidence**: the identical
misattribution in a sibling world package was routed **(a) — the package's
statement is wrong**, repaired by prose edit, and closed. The precedent for this
anchor is set, and the repaired sentence is the model for the fix.

**Restated reason (for the record; the verdict is the boss's):**

> DRIFT — misattribution, and the family precedent is already closed. The row
> credits `flow:git-atomic-commits` with the rule that commit bodies cite
> `spec://` URIs. That rule is authored in the sibling
> `flow:git-conventional-commits` — `spec/boot/31-flow-conventional-commits.md:24`
> `##CITE-SPEC-URIS-WHERE-RELEVANT`, elaborated at
> `spec/flows/conventional-commits/conventional-commits.md:74-77` — and
> `git-atomic-commits` carries no citation rule at all: all three `spec://`
> occurrences in that package are pointers to siblings, and its own snippet
> disclaims the class at `spec/boot/30-flow-atomic-commits.md:26`
> («Conventional Commits is the *format*; this flow is the *atomicity*»).
> `addressable-specs`' `## Composition` section does not list
> `git-conventional-commits` at all, so the fact is routed to the one sibling
> that disclaims it. Both flows are installed and pinned (`vibe.lock:293,296`),
> so the misattribution is checkable against the shipped payload. The behaviour
> itself is real: 579 of 2 212 commits cite a `spec://` URI in their message at
> `HEAD = f2b11b0a` (`git log --grep="spec://" --oneline | wc -l`) — **not** the
> 515 or 716 previously recorded; 716 was a line count (732 at this HEAD), not a
> commit count. The identical misattribution at `sync-from-code`'s
> `##BOUNDARY-FLOW-ATOMIC-COMMITS` was closed as F-253 by route (a) in wave 3 and
> now reads `confirmed`; its repaired wording is the model.

**RECOMMENDATION: STANDS-RESTATED.** Route (a) — the package's statement is
wrong, measurable entirely inside `packages/`, precedent already set by F-253.
The repair is one line and needs no ruling; the owner gate is publication only,
exactly as the queue says.

### Anchor 2 — `campaign-plans/v0.1.0/README.md#COMPOSES-ATOMIC-COMMITS` → **FALLS**

**Current text at HEAD** (`README.md:70-72`) [1]:

```
- ##COMPOSES-ATOMIC-COMMITS `flow:git-atomic-commits` — each phase's commit set follows it:
  subjects are spelled in the plan, one idea per commit, and the
  ledger binds hashes to the planned subjects. @impl/done
```

**This sentence makes no attribution claim.** It does not mention `spec://`
URIs, message format, or Conventional Commits. Every clause it does carry —
*subjects spelled in advance*, *one idea per commit*, *the ledger binds hashes to
planned subjects* — is either atomicity (which `git-atomic-commits` genuinely
owns) or campaign-plan ledger structure. **The registry's F-219 reason is
therefore not this anchor's reason**, and applying it here would be
[`##A-REAL-DEFECT-CONVICTING-THE-WRONG-SENTENCE`](../PHASE-D-BATCH-PLAN.md#delegation-lessons)
at obligation scale. The row's own reason, read from the cache, is a *practice
absence* argument, and that is what is re-verified below.

**The cache's own evidence list for this row** — four refs, and **two are in
`legacy-spec/`**:

```
packages/org.vibevm.world/campaign-plans/v0.1.0/spec/flows/campaign-plans/phase-gates.md:67
legacy-spec/terraforms/SELF-SUFFICIENCY-PLAN-v0.1.md:374        ← EXCLUDED perimeter
legacy-spec/terraforms/GO-AI-NATIVE-PLAN-v0.1.md:298            ← EXCLUDED perimeter
vibe.lock:293
```

Under the owner's 2026-07-31 ruling
([§3.7](../PHASE-D-BATCH-PLAN.md#compliance-blindness)), `legacy-spec/**` «is not
evidence of practice in either direction», and the ruling names this exact
failure mode: *«Wave 6 and wave 7 both leaned on archived-versus-live ratios
drawn from it.»* **The reason is built entirely on such a ratio** («4 archived
plans … 0 live … 3 in the archive»). Strip the archive column and the argument
has no numerator. Of the two admissible refs, `vibe.lock:293` **supports** the
sentence (the flow is installed and pinned), and `phase-gates.md:67` is the
package's own spec **also supporting** it [1]:

```
| ##ROW-ELEMENT-COMMIT-SET **Commit set** @impl/done | the planned commits, subjects spelled in advance @impl/done |
```

**The verdict's own commands, re-run at HEAD** — they reproduce, and that is the
point: the numbers are right and the perimeter is wrong.

```console
$ grep -rlE '\*Commits' legacy-spec/terraforms/*.md | wc -l        # EXCLUDED; run only to reproduce
4
$ grep -rlE '\*Commits' spec/terraforms/*.md | wc -l
0
$ grep -rlE '\*Commits' campaigns/packages-2026-09/*.md | wc -l
0
$ for p in spec/terraforms/*.md; do grep -ciE 'commits:' "$p"; done
0
0
```

**The perimeter miss, and it is [§3.7's wave-6 extension](../PHASE-D-BATCH-PLAN.md#compliance-blindness)
exactly.** The verdict searched this host's two live campaign zones
(`spec/terraforms/`, `campaigns/packages-2026-09/`) and the archive. It did not
search **the second complete project in this repository that installs
`flow:campaign-plans` and boots it** — the `fractality` specspace inside
`packages/`. That project is a consumer of this very flow [3]:

```console
$ grep -n "campaign-plans" packages/org.vibevm.fractality/fractality/v0.1.0/vibe.lock
25:    "flow:org.vibevm.world/campaign-plans@=0.1.0",
135:name = "campaign-plans"

$ grep -n "campaign-plans" packages/org.vibevm.fractality/fractality/v0.1.0/spec/boot/INDEX.md
37:path = "vibedeps/flow-campaign-plans/0.1.0/spec/boot/40-flow-campaign-plans.md"
```

**Searched for the thing, over the standing perimeter, `legacy-spec/` and
`campaigns/*/run/` excluded:**

```console
$ grep -rln '\*Commits:\*' packages spec campaigns terraform discipline research \
    crates xtask tools docs manual-tests fixtures schemas *.md \
    --exclude-dir=target --exclude-dir=node_modules --exclude-dir=run \
    --exclude-dir=.vibe --exclude-dir=vibedeps | sort -u
campaigns/packages-2026-09/baseline.json                                   ← campaign record
campaigns/packages-2026-09/harvest/d2-campaign-plans-research-repairs.md   ← campaign prose ABOUT the finding
campaigns/packages-2026-09/harvest/d6a-plans-research-absences.md          ← campaign prose ABOUT the finding
campaigns/packages-2026-09/tasks/evidence/…  (6 files)                     ← campaign records
packages/org.vibevm.fractality/fractality/v0.1.0/spec/plans/FRACTALITY-IGNITION-PLAN-v0.1.md   ← INSTANCE
packages/org.vibevm.world/campaign-plans/v0.1.0/spec/flows/campaign-plans/phase-gates.md       ← the spec itself
packages/org.vibevm.world/git-attribution-policy/v0.1.0/spec/flows/attribution-policy/disclosure-alternative.md
```

**Nine of the twelve hits are this campaign's own footprint**
([`##THE-CAMPAIGN-IS-INSIDE-ITS-OWN-CORPUS`](../PHASE-D-BATCH-PLAN.md#delegation-lessons)) —
opened and confirmed to be prose about the finding, not instances. **One is a
live instance**, and it settles all three clauses.

*Clause 1 — «subjects are spelled in the plan».* `FRACTALITY-IGNITION-PLAN-v0.1.md:894-896` [2]:

```
894: *Commits:* `feat(fractality): cargo workspace + core model` ·
895: `feat(fractality): mission-control — journal, registry, lifecycle` ·
896: `feat(fractality): pod — child supervision skeleton`.
```

`grep -c '\*Commits:\*'` on that file returns **8** — eight phases with subjects
spelled in advance, in a live plan of a project that boots this flow at slot 40.

*Clause 2 — «one idea per commit».* Conceded by the verdict itself («that half
stands»). Not re-litigated.

*Clause 3 — «the ledger binds hashes to the planned subjects».* Three live
ledgers [2]:

```console
$ grep -c "^### Phase .* — EXECUTED .*; commit map" …/FRACTALITY-IGNITION-PLAN-v0.1.md
8
$ grep -c "^- \*\*Ф[0-9]* — EXECUTED .*Commit map:\*\*" …/FRACTALITY-INITIATIVE-PLAN-v0.1.md
5
$ sed -n '225,227p' …/FRACTALITY-RLM-PLAN-v0.1.md
## 9. Ledger {#ledger}

Commit map (Stage B execution, Campaign 3):
```

And the binding is checkable **in both directions**, which no archived ratio
could establish. Planned at `:894`, ledgered at `:1163`, and the commit exists
in this repository's history:

```console
$ sed -n '1163p' …/FRACTALITY-IGNITION-PLAN-v0.1.md
- `bd1e65d` feat(fractality): cargo workspace + core model.

$ git log -1 --format="%h %s" bd1e65d
bd1e65d7 feat(fractality): cargo workspace + core model
```

Eight of eight sampled hashes resolve to real commits whose subjects match the
planned strings (`bd1e65d`, `bd4d78c`, `04cb292`, `e7eb608`, `6f5788a`,
`35a378c`, `d91780d`, `12b9824`). Counted with a Python scan for
`` `[0-9a-f]{7,10}` ``: **58 unique short hashes bound across the three plans**
(IGNITION 22, INITIATIVE 8, RLM 28) — against the verdict's «0 live plans».

**RECOMMENDATION: FALLS — re-judge `confirmed`.** Two independent grounds, either
sufficient: *(i)* the sentence carries no attribution claim, so the obligation's
own reason cannot convict it, and its own reason is an archived-versus-live ratio
the owner's 2026-07-31 ruling voids; *(ii)* on the standing perimeter including
the `fractality` adopter, all three clauses have live instances — 8 phases with
subjects spelled in advance, 3 ledgers, 58 hashes bound, sampled bindings
verified against `git log`. This is
[§3.7](../PHASE-D-BATCH-PLAN.md#compliance-blindness) in its wave-6 form: a
search scoped to the host read a second adopter's compliance as an absence.

**Consequence for the release queue.** F-219 is described in
[§C](../PHASE-D-RELEASE-QUEUE.md#composes) as «a pure attribution fix» over two
packages. On this evidence it is a **one-package** fix; `campaign-plans` needs no
edit and should not be published as part of it. Publishing a two-package fix
where one package was never wrong is not
[§4.5](../PHASE-D-BATCH-PLAN.md#release)'s duplication risk but its mirror — an
edit with no defect under it.

---

## F-220 — one anchor is a textbook §3.6(b); the other is a different sentence convicted on a host-internal contradiction

**Outcome:** SPLIT — 1 **ROUTE-OUT-CANDIDATE** (addressable-specs; the rule is
specified on both sides and the host does not keep it) · 1
**STANDS-RESTATED** (source-mirrors; a different sentence, one of whose two
halves survives and on completely different evidence than the record carries).

**Anchors:** 2 of 2, and — as with F-219 — **the two sentences share only their
anchor name.** The registry `reason` is the addressable-specs verdict. The
source-mirrors row carries its own reason, about mirror fan-out, which mentions
neither Constraints, nor anchors, nor resumed sessions.

### The WAL measurements, re-run at HEAD [2]

`spec/WAL.md` is measured here as a **host artefact** (source 2 — the host's
observed conformance), never quoted for campaign numbers.

```console
$ grep -c "spec://" spec/WAL.md
2

$ grep -n "spec://" spec/WAL.md
65:links take `@spec://` where they are pointers and `#embed` where the target
337:cannot be cited. `spec://` occurs zero times in this file, in every revision
```

**Both hits are the inside-own-corpus trap and both were opened.** `:64-68` is
this campaign's own record of the Phase D owner rulings («the 69 dangling
`../flows/…` links take `@spec://` …»); `:333-338` is this campaign's own note
about the absence («*`spec://` occurs zero times in this file, in every revision
measured, while the flow that requires it is installed and read at every boot*»).
**Neither is a citation.** So the substantive count is **0 `spec://` citations in
the host WAL**, exactly as the verdict says, and the verdict's own «2 today, both
campaign prose ABOUT the absence» parenthetical reproduces exactly.

**The constraint-entry count has moved and is now worse, not better.** The
verdict recorded «4 of its 26 constraint entries». At HEAD, counted with a Python
scan that splits `## Constraints — do not violate` (`:76`) at every `^- ` and runs
to the next `## ` heading (`:179`):

```
constraint entries: 28
entries citing a spec location in ANY notation: 2   (:168, :172)
entries citing an ANCHOR (##NAME or spec://): 0
```

And the two that cite anything cite **section numbers, not anchors** [2]:

```
168: - **mtime unit in the vvm manifest.** TS port stores `mtime_ms`; the Rust twin stores
169:   `mtime_nanos` (PROP-019 §2.15).
172: - **CI-off gate split.** `CI` / `VIBE_NO_DEFAULT_REGISTRY` suppresses vibe-embedded
173:   but NOT project-local (PROP-030 §5 + §3.3).
```

`§2.15` and `§5 + §3.3` are precisely the notation `addressable-specs` exists to
replace. **So the honest figure is 2 of 28 by any notation and 0 of 28 by
anchor** — the direction of the verdict is right and its arithmetic is stale, the
WAL having been rewritten at three wind-downs since (`git log --oneline -5 --
spec/WAL.md`).

### Anchor 1 — `addressable-specs/v0.1.0/README.md#COMPOSES-WAL` → **ROUTE-OUT-CANDIDATE (§3.6(b))**

**Current text at HEAD** (`README.md:68-69`) [1]:

```
- ##COMPOSES-WAL `flow:wal` — WAL Constraints and next-step pointers cite anchors,
  so a resumed session lands on the exact unit. @impl/done
```

**Capability / practice / rule:** a **rule restatement plus its consequence.**
The antecedent («WAL Constraints and next-step pointers cite anchors») is not
`addressable-specs`' own invention — it restates the composed flow's own
requirement, which is specified and addressable [1]:

```
83: 3. ##SECTION-CONSTRAINTS **Constraints** — the "do not touch" list; each entry carries a
84:    brief *why*, citing a spec anchor or issue: `match_by_hash(): DO NOT
85:    TOUCH — fragile reconnection logic, issue #12`. …  @impl/done

91: 5. ##SECTION-IN-PROGRESS **In progress** — what is open, with enough detail that the next
92:    session can pick it up. Cite spec anchors (`spec://…`). @impl/done
```

The consequent («so a resumed session lands on the exact unit») is
`addressable-specs`' own contribution and is true by construction — the anchors
`flow:wal` asks for are the addresses this package defines. **So the composition
is soundly specified on both sides, exactly as the verdict's own first sentence
concedes**, and both flows are installed [3] (`vibe.lock:62`
`"flow:org.vibevm.world/wal@=0.2.0"`; the `addressable-specs` slot at
`vibedeps/flow-addressable-specs/`).

**Read further before searching wider — and it corrects one of the verdict's two
measurements.** The reason ends «*The Next section cites repository paths, not
anchors*». But `flow:wal`'s `##SECTION-NEXT` **imposes no citation requirement at
all** [1]:

```
93: 6. ##SECTION-NEXT **Next** — the single next action. Several candidates? List them
94:    briefly and mark the default. @impl/done
```

Measuring the `Next` section for anchor citations tests a rule that does not
exist. The section the rule actually binds is `##SECTION-IN-PROGRESS` (`:92`,
«Cite spec anchors (`spec://…`)»), and the host fails **that** one too — its
`## In progress` (`spec/WAL.md:199-226`) names campaign files and paths
(`harvest/world-w6-project-practice-ii.md`, `tasks/batch-progress.py`,
`ev-W5a…d`) and cites no anchor [2]. So the finding survives on the correct
section; the reason cites the wrong one.

**Why this is (b) and not (a).** [§6.1's](../PHASE-D-BATCH-PLAN.md#delegation-lessons)
second cheap check settles it: *«a rule the consumer breaks is §3.6(b), not a
wrong sentence.»* Nothing in the sentence is a claim about this repository.
`flow:wal` requires the citation, `addressable-specs` supplies the address space,
and the host — the one consumer — writes 28 constraint entries with 0 anchors
among them. Under [§3.6(b)](../PHASE-D-BATCH-PLAN.md#which-side) the package does
not move; a host obligation is recorded and the row goes `status: deferred`
naming it. Editing `addressable-specs` here would be the *профанация* §3.6 names:
rewriting the discipline to describe a lax consumer.

**Which consumer, and what it owes.** The consumer is **this host repository**,
and the obligation is on `spec/WAL.md`, not on the package:

1. `## Constraints — do not violate` — bring the 28 entries under
   `flow:wal` `##SECTION-CONSTRAINTS`: each carries a *why* citing a spec anchor
   or issue. 26 cite neither; the 2 that cite anything use `PROP-NNN §N` section
   numbers.
2. `## In progress` — bring it under `##SECTION-IN-PROGRESS`: «Cite spec anchors
   (`spec://…`)». It currently cites campaign file paths only.
3. **The prerequisite the host already recorded against itself**
   (`spec/WAL.md:335-337`, this campaign's own note): *«all 8 [headings] here
   carry no anchor, so the Constraints section above cannot be cited»* — the WAL
   is not addressable, so a constraint entry cannot even be pointed at. That is
   the same defect one level up and belongs in the same host task.

**RECOMMENDATION: ROUTE-OUT-CANDIDATE (§3.6(b)).** Rule sound on both sides, one
consumer does not keep it, package does not move. If the boss accepts, the row
belongs in [`run/state/routing.json`](../run/state/routing.json) with the host
obligation above — **written by the boss at review time, never by a worker**
([§7's](../PHASE-D-BATCH-PLAN.md#gate) exit-gate rule). This removes one anchor
from the release ask entirely: no publication is needed for a package that does
not move.

### Anchor 2 — `source-mirrors/v0.1.0/README.md#COMPOSES-WAL` → **STANDS-RESTATED**

**Current text at HEAD** (`README.md:74-75`) [1]:

```
- ##COMPOSES-WAL `flow:wal` — the fan-out is a natural session wind-down step; the WAL
  entry notes "fanned out at <checkpoint>". @spec/done
```

Note the marker: **`@spec/done`, not `@impl/done`** — the sibling anchor's is
`@impl/done`. This one asserts a *specified* composition, which is the weaker
claim, and that bears on how a practice absence lands on it.

**Half (i) — «the fan-out is a natural session wind-down step» → the verdict's
ground is a HOST-INTERNAL contradiction, not a falsification of this sentence.**

The recorded reason argues: *«`CLAUDE.md`'s END SESSION command, step 4,
prescribes «Push to `origin/main`» — the bare named-remote push that
`spec/boot/90-user.md:35` and `PROP-016:59` BOTH name as not the standard
rollout.»* Every fact in that sentence is true, and it acquits the package. Read
the two documents it names [2]:

```
spec/boot/90-user.md:35
- ##CMD-MIRROR Roll a change out to ALL source mirrors (GitVerse + GitHub), verified 2026-06-14:
  `cargo xtask mirror` … This is the standard rollout, preferred over a bare `git push origin`.

spec/common/PROP-016-source-mirrors.md:59
- ##CMD-MIRROR `cargo xtask mirror` — push mainline (`main` + tags) to every `push` target,
  fast-forward-only, never `--force` … This — not `git push origin` — is the standard rollout.
```

**Both host documents agree with the package.** The only document out of step is
`CLAUDE.md:191`. So the disagreement is *`CLAUDE.md` against `spec/boot/90-user.md`
and `PROP-016`* — a host-internal contradiction between three host documents —
and the package's sentence sits on the side that two of the three take. This is
[`##A-REAL-DEFECT-CONVICTING-THE-WRONG-SENTENCE`](../PHASE-D-BATCH-PLAN.md#delegation-lessons)
in its clearest form: the defect is real and its owner is `CLAUDE.md`'s END
SESSION step 4, not `source-mirrors`' README. The package's own spec states the
same rule [1]:

```
daily-loop.md:42
| ##ROW-MOMENT-END-OF-A-WORK-SESSION End of a work session @impl/done | Fan out as the wind-down step @impl/done |
```

**Half (i) therefore falls as a ground for drift**, and — recorded rather than
used — a **host** finding falls out of it: `CLAUDE.md:191` prescribes at
wind-down the exact push two other host documents mark as not the standard
rollout. That is a host obligation for the boss, on a file
[Rule 4](../../CLAUDE.md) treats as sensitive; it is **not** actioned here.

**Half (ii) — «the WAL entry notes "fanned out at <checkpoint>"» → survives, and
this is the only surviving ground.**

```console
$ grep -niE "fanned out|fan out|fan-out|xtask mirror" spec/WAL.md
(no output)
```

Zero hits in the host WAL at HEAD [2]. Widened to the standing perimeter for the
*thing*:

```console
$ grep -rln "fanned out" packages spec campaigns terraform discipline research \
    crates xtask tools docs manual-tests fixtures schemas *.md \
    --exclude-dir=target --exclude-dir=node_modules --exclude-dir=run --exclude-dir=.vibe
```

Twelve files, **and every one was opened**. Five are campaign records
(`baseline.json`, four `tasks/evidence/*W6b*.json`). Two are vendored copies of
this very sentence
(`packages/org.vibevm.fractality/{delegation-rules,fractality}/…/vibedeps/flow-source-mirrors/0.1.0/README.md`)
[3]. One is the sentence itself. **The remaining four are a different sense of
the word entirely** — the delegation sense, not the mirror sense:

```
terraform/adopt-v0.3/LOG.md:542            doctest/REQ work fanned out to four parallel authoring agents
…/fractality/spec/PROP-001-foundation.md:70  **swarm** — N runs fanned out over a task decomposition
…/fractality/spec/manual-tests/MT-C3-01-rlm-gated-trial.md:124  fanned out hardest. 9 gate calls
…/fractality/reports/2026-12-07-06-44-campaign3-f6-trial.md:34  boss fanned out to 8 workers
xtask/src/mirror.rs:8                      … then fanned out from here). This
```

**No WAL entry anywhere in the perimeter — host or fractality — carries the
form.** Perimeter named, per
[`##ABSENCE-NAMES-ITS-PERIMETER`](../PHASE-D-BATCH-PLAN.md#delegation-lessons).

**And the composition is specified on ONE side only, which is what distinguishes
this anchor from its sibling** [1]:

```console
$ grep -rni "mirror\|fan.out" packages/org.vibevm.world/wal/v0.2.0/
(no output)
```

`flow:wal` — the flow that owns `spec/WAL.md`'s section grammar (`:76-98`,
eight numbered sections) — **never mentions mirrors or fan-out at all**. So
`source-mirrors` prescribes content for a document whose own protocol has no slot
for it, and no consumer has ever produced it. That is *not* the addressable-specs
shape (rule on both sides, consumer lax); it is a one-sided claim with no
instance.

**Restated reason (for the record):**

> DRIFT on the WAL-entry half only, and on different evidence than first
> recorded. The «fan-out is a natural session wind-down step» half is **sound**:
> the package's own `daily-loop.md:42` states it, and the host's
> `spec/boot/90-user.md:35` and `spec/common/PROP-016-source-mirrors.md:59` both
> name `cargo xtask mirror` the standard rollout, «preferred over a bare
> `git push origin`». The document that disagrees is `CLAUDE.md:191`, which
> contradicts those two host documents — a host defect, not this sentence's. What
> does not hold is the second half: no WAL entry notes «fanned out at
> <checkpoint>». `grep -niE "fanned out|fan out|fan-out|xtask mirror"
> spec/WAL.md` → 0 at `HEAD = f2b11b0a`, and over the standing perimeter every
> «fanned out» hit outside this README and its two vendored copies is the
> delegation sense of the word (`terraform/adopt-v0.3/LOG.md:542`,
> `fractality/spec/PROP-001-foundation.md:70`, `MT-C3-01:124`,
> `reports/…f6-trial.md:34`, `xtask/src/mirror.rs:8`). And `flow:wal` specifies
> no such entry: `grep -rni "mirror\|fan.out"` over `wal/v0.2.0/` returns nothing,
> so its eight-section grammar (`WAL-PROTOCOL.md:76-98`) has no slot for it. The
> claim is prescribed on one side, unsupported on the other, and unexercised.

**The which-side call, stated rather than taken.** The queue says F-220 «needs
the which-side ruling before an edit exists to approve», and for this half the
ruling turns on one reading:

- **(a)** if «the WAL entry notes …» is *a described practice stated as fact*,
  it is route (a) — the package's own statement, over-claiming a convention no
  protocol carries. The repair is one clause: drop the WAL-entry half, or soften
  it to what `daily-loop.md` actually supports.
- **(b)** if it is read as a *prescription* — which the `@spec/done` marker
  supports, and which §6.1 protects («an unexercised capability is not a false
  capability») — it is a sound-but-unexercised rule and the package does not
  move; what is owed is `flow:wal` growing the slot, or the host writing the note.

**RECOMMENDATION: STANDS-RESTATED**, with the (a)/(b) reading left to the
which-side ruling the queue already schedules. The verdict's **current** reason
must be replaced before any diff is shown to the owner — as written it asks for
the demotion of a sentence that two host documents support, on the strength of a
third host document that contradicts them.

---

## F-233 — the composed flow settles §6.1's question outright, and both anchors are §3.6(b)

**Outcome:** BOTH **ROUTE-OUT-CANDIDATE (§3.6(b))** — the rule is sound, it is
the *composed flow's own* four-field contract, and **two** consumers in this
repository install both flows and neither keeps it.

**Anchors:** 2 of 2. Unlike F-219 and F-220, these two sentences genuinely make
**the same claim** about different subject matter, so the obligation's merge is
sound and they are judged as a family.

### The question §6.1 asks is answered by the composed flow, not by a reading [3]

The brief's framing — *does the composition sentence require a decision record
in the decision-records FORM (the four-field block), or merely that the choice IS
RECORDED as a decision?* — does not need a judgement call. The installed flow
defines what «a recorded decision» means, in its own boot snippet
(`vibedeps/flow-decision-records/0.1.0/spec/boot/25-flow-decision-records.md`):

```
Any choice a future reader could plausibly re-open … gets a **four-field record
at the spec anchor that governs the value**:

| **Decision**                | The chosen value or approach. One line. |
| **Why**                     | Concrete and cited: a measurement, a constraint, an incident — with data. |
| **Considered and rejected** | One line per alternative, each carrying its rejection reason. |
| **When to revisit**         | A measurable trigger: metric + threshold + where it is observed. |

There is no separate ADR directory and no immutable numbered log.
The spec section that governs the value IS the record …
```

and again in its `## Never` list:

```
- Never record a decision with a missing reason or a missing revisit
  trigger — that is a fact with decoration, not a record.
```

**So a three-field record is not a record**, by the composed flow's own words.
And critically: **both composition sentences name the revisit trigger
explicitly**, so whichever way the form question is read, the field they assert
is the field being measured. This is *not*
[`##A-REAL-DEFECT-CONVICTING-THE-WRONG-SENTENCE`](../PHASE-D-BATCH-PLAN.md#delegation-lessons) —
the defect belongs to this clause of this sentence.

**Two corrections owed to the searches, and the first one matters most.** The
`git-attribution-policy` reason's leading command is
`git ls-files | grep -iE 'decision|adr'` «returns only the flow package and
vendored copies», plus «there is no `spec/decisions/`». **The composed flow
forbids exactly that artefact** — «There is no separate ADR directory and no
immutable numbered log». So the search looked for the thing the rule says must
not exist, and would have read every genuine record as an absence. The host in
fact **practises the four-field form**, and the search missed all of it [2]:

```console
$ grep -rl -i "when to revisit" spec/common/*.md spec/modules/*/*.md
spec/common/PROP-000.md                            (##LANG-REVISIT:23, ##LICENSE-REVISIT:57)
spec/modules/vibe-cli/PROP-036-package-tree.md     (##decision-artifacts-revisit:95)
spec/modules/vibe-registry/PROP-001-git-backend.md (##revisit-lead:113)
```

The clause of that reason that **is** load-bearing and **is** correct is the last
one: «PROP-000 §12 records the posture as a pointer with no trigger». Verified —
`awk 'NR>=157 && NR<=168' spec/common/PROP-000.md | grep -ciE "revisit|considered and rejected"` → **0**.

### Anchor 1 — `git-attribution-policy/v0.1.0/README.md#COMPOSES-DECISION-RECORDS` → **ROUTE-OUT-CANDIDATE (§3.6(b))**

**Current text at HEAD** (`README.md:83-84`) [1]:

```
- ##COMPOSES-DECISION-RECORDS `flow:decision-records` (`25-…`): the posture choice is a recorded
  decision with a revisit trigger tied to regulation changes. @impl/done
```

**The sentence restates the package's own instruction to its adopter**, which is
what makes it a rule and not a report [1]:

```
ATTRIBUTION-POLICY.md:160-161
  choice. Record the choice as a decision with a revisit trigger tied
  to regulation changes. Show me the draft; do not apply until I approve.

disclosure-alternative.md:82-84
  1. ##SWITCH-EDIT-THE-SINGLE-POLICY-PLACE The owner edits the single policy place (the boot snippet) to the
     new posture, with a dated decision record and a revisit trigger
     (see `flow:decision-records`). @impl/done
```

**Worth flagging for B-004:** `ATTRIBUTION-POLICY.md:160` sits **inside the
fenced re-derive block** (`:151-163`) and therefore carries no anchor — the same
unaddressable-instruction surface [`B-004`](../../BACKLOG.md#b-004) files. The
only *addressable* statement of the rule is `##SWITCH-EDIT-THE-SINGLE-POLICY-PLACE`,
and it binds a posture **change**, not the standing choice. So the composition
row at `README.md:83` is, at HEAD, the most addressable statement of the very
rule it is being convicted of failing.

**The absence, with its perimeter named
([`##ABSENCE-NAMES-ITS-PERIMETER`](../PHASE-D-BATCH-PLAN.md#delegation-lessons)).**
Searched for the *thing* — a regulation-tied trigger governing the attribution
posture — over the standing perimeter:

```console
$ grep -rln -i "regulation" packages spec campaigns terraform discipline research \
    crates xtask tools docs manual-tests fixtures schemas *.md \
    --exclude-dir=target --exclude-dir=node_modules --exclude-dir=run \
    --exclude-dir=.vibe --exclude-dir=vibedeps
campaigns/packages-2026-09/baseline.json                      ← campaign record
campaigns/packages-2026-09/tasks/evidence/batch-W1c.json      ← campaign record
campaigns/packages-2026-09/tasks/evidence/ev-W1c.json         ← campaign record
packages/org.vibevm.world/git-attribution-policy/…  (4 files) ← the package itself
spec/boot/STATIC.md                                           ← the COMPILED copy of that package's snippet
spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md       ← campaign prose ABOUT the finding
```

**Every hit was opened.** `spec/boot/STATIC.md:445-446` and `:639-640` are the
package's own boot snippet compiled into the host's lane [3] («*such regulation
and violates no present law; the owner chooses the posture proactively…*») — the
package speaking, not a host record. Four are this campaign's own footprint. **No
host document anywhere carries a regulation-tied revisit trigger**, and neither
does the second adopter:

```console
$ grep -rn -i "regulation" packages/org.vibevm.fractality/ --include=*.md | grep -v vibedeps | grep -v "\.vibe"
(no output)
```

**What the host does have** [2] — and it is exactly the pointer-without-a-record
shape:

```
spec/boot/00-core.md:21
1. ##RULE-ATTRIBUTION **Attribution — keep this repository human-authored.** Never mark commits,
   branches, comments, or any artefact as machine-authored. The rule itself (and its copy in
   PROP-000 §12.1) is the only place in the project where that topic is discussed. @impl/done

spec/common/PROP-000.md:161
- ##GP-ATTRIBUTION human-authored **attribution** —
  `spec://org.vibevm.world/git-attribution-policy/flows/attribution-policy/ATTRIBUTION-POLICY#root`; @spec/done
```

A rule and a pointer. No *Why* with data, no *Considered and rejected*, no
*When to revisit* — three of the four fields absent, and the composed flow's
`## Never` list calls that «a fact with decoration, not a record».

**Why (b) and not (a).** The rule is the package's own, it is sound, and it is
the composed flow's contract verbatim. Nothing in the sentence is a measurement
of this repository. Editing `git-attribution-policy` to stop claiming a trigger
would delete a correct instruction because a consumer skipped it — the exact
direction [§3.6](../PHASE-D-BATCH-PLAN.md#which-side) forbids.

**Which consumer, and what it owes. Two, not one** — the reason says «in the one
consumer that installs both flows», and that is wrong at HEAD [3]:

```console
$ grep -n "attribution-policy@\|decision-records@" vibe.lock
46:    "flow:org.vibevm.world/decision-records@=0.1.0",
294:    "flow:org.vibevm.world/git-attribution-policy@=0.1.0",

$ grep -n "attribution-policy@\|decision-records@" packages/org.vibevm.fractality/fractality/v0.1.0/vibe.lock
24:    "flow:org.vibevm.world/attribution-policy@=0.1.0",
28:    "flow:org.vibevm.world/decision-records@=0.1.0",
```

Both this host and the `fractality` specspace install both flows; neither records
the posture. **The host owes** a four-field record at the anchor that governs the
posture — `spec/boot/00-core.md` `##RULE-ATTRIBUTION` or `PROP-000` §12
`##GP-ATTRIBUTION` — carrying the *why* (present law imposes none; the owner
chooses proactively so future regulation finds no hook), the rejected alternative
(the disclosure posture, which the package already documents in full), and a
measurable revisit trigger tied to regulation changes.

**RECOMMENDATION: ROUTE-OUT-CANDIDATE (§3.6(b)).** Package does not move; host
obligation recorded; row to
[`run/state/routing.json`](../run/state/routing.json) by the boss.

### Anchor 2 — `source-mirrors/v0.1.0/README.md#COMPOSES-DECISION-RECORDS` → **ROUTE-OUT-CANDIDATE (§3.6(b))**

**Current text at HEAD** (`README.md:76-78`) [1]:

```
- ##COMPOSES-DECISION-RECORDS `flow:decision-records` — the host set and the single-writer choice
  are recorded decisions, each with a revisit trigger (e.g. "revisit
  when parallel full-time integrators exceed one"). @impl/done
```

**The brief's hypothesis, tested and rejected.** The suggestion was that if
`PROP-016`'s `##HIST-AUTHORED` **is** a recorded decision, this may be a real
defect convicting the wrong sentence. It is not, and the reason is the sentence's
own wording. Read `PROP-016:78` against the four fields [2]:

```
78: - ##HIST-AUTHORED **2026-06-14 — authored, in force.** Owner decision: the source becomes
    multi-homed (GitVerse + GitHub `vibevm/vibevm`, both public, canonical for reading;
    US↔GitHub, RU↔GitVerse), kept in sync by the maintainer's fan-out. … Supersedes the
    interim multi-push-remote and the abandoned bidirectional-multi-master sketch. @spec/done
```

| field | `##HIST-AUTHORED` |
|---|---|
| **Decision** | ✔ «the source becomes multi-homed …» |
| **Why** | ~ «owner-requested» (`##status-line:5`); the substantive why is delegated to the flow (`##model-pointer:8`), not carried here |
| **Considered and rejected** | ~ two alternatives **named** («the interim multi-push-remote», «the abandoned bidirectional-multi-master sketch») but **without rejection reasons**, which `record-template.md` requires |
| **When to revisit** | ✘ **absent** |

**So it is three-quarters of a record, and the quarter it is missing is the
quarter the sentence explicitly claims.** The sentence does not merely say «are
recorded decisions» — it says «**each with a revisit trigger**» and then quotes
one. The verdict's own command reproduces exactly at HEAD [2]:

```console
$ grep -n -i 'revisit\|parallel\|integrator' spec/common/PROP-016-source-mirrors.md
(no output; rc=1)

$ wc -l < spec/common/PROP-016-source-mirrors.md
80
```

Not one of the three words occurs in the whole document — including the two
distinctive words of the trigger the README itself quotes. The **single-writer
choice** is likewise stated without a trigger (`##HOSTS-MODEL:13`, its why
pointed off to the flow at `##model-pointer:8`).

**Read further before searching wider:** `PROP-016` §5 `## Open questions`
(`:68-74`) is the section a revisit trigger would most plausibly hide in. It
carries three open questions, each `@spec/work` — server-side mirroring,
`self-pull` adoption, a `vibe`-level mirror surface — and **none is a revisit
trigger for the host set or the single-writer choice**. The section's own
`<status stage="spec" state="work">` confirms it is a what-is-unresolved list,
not a when-to-re-open list.

**Why (b) and not (a).** Identical to anchor 1: the four-field requirement is the
**composed flow's** contract, not `source-mirrors`' invention, and the sentence
restates it. A consumer with 3 of 4 fields is a consumer that broke a sound rule.

**Which consumer, and what it owes.** Again **two** — the host and the
`fractality` specspace both install `source-mirrors@=0.1.0` **and**
`decision-records@=0.1.0` [3] (`vibe.lock:46,57`;
`fractality/…/vibe.lock:28,37`). **The host owes** a `When to revisit` field on
`PROP-016`'s two governed choices — the host set (`##HOSTS-MODEL:13`, recorded at
`##HIST-AUTHORED:78`) and the single-writer model — plus rejection *reasons* on
the two alternatives `##HIST-AUTHORED` already names.

**The scale datum, which is the argument for routing out rather than editing** [2]:

```console
$ ls spec/common/*.md spec/modules/*/*.md | wc -l
43
$ grep -rl -i "when to revisit" spec/common/*.md spec/modules/*/*.md | wc -l
3
```

**3 of 43 host spec documents carry the field at all.** This is not one missing
trigger in one PROP — it is systemic host non-conformance with an installed flow,
which is precisely the class §3.6(b) exists to route to a host task rather than
absorb into the packages. It also settles the question the record says was left
open in `spec-genres`: the revisit trigger is the **exception** in this host, not
the rule, and no package should be edited to describe that.

**RECOMMENDATION: ROUTE-OUT-CANDIDATE (§3.6(b))** for both anchors. F-233 needs
no publication and no edit; it needs the which-side ruling the queue
[§C](../PHASE-D-RELEASE-QUEUE.md#composes) already schedules, and the evidence
above points one way.

---

## F-240 — the lead asserts nothing about a path, and the anchor one line above it forbids the reading that convicts it

**Outcome:** BOTH **FALL** — the defect the verdict describes is real, lives in
the fenced body, and is **not this anchor's**. Worse than a neighbour
misattribution: in both packages an **addressable anchor sitting one to two lines
ABOVE the lead explicitly instructs the reader not to copy the fenced block
verbatim**, which is precisely the reading the verdict performs.

**Anchors:** 2 of 2.

### The corpus, re-measured at HEAD — seventeen exactly, and the roster matches [1]

```console
$ grep -rn "^Read spec/flows/" packages/org.vibevm.world/*/v*/spec/flows/*/*.md | wc -l
17
```

addressable-specs `:260` · comparative-research `:188` · conflict-protocol `:227`
· decision-records `:208` · discovery-prompt `:180` · git-attribution-policy
`:152` · health-audit `:233` · **licensing `:150`** · managed-blocks `:272` ·
manual-tests `:124` · operating-modes `:144` · qualified-naming `:180` ·
secrets-hygiene `:210` · source-mirrors `:165` · **spec-genres `:180`** ·
two-process-model `:168` · wal `:240`. Identical to the queue's
[`##A1-F240-IS-SCOPED-AT-TWO-AND-THE-DEFECT-IS-IN-SEVENTEEN`](../PHASE-D-RELEASE-QUEUE.md#addresses-scope)
list, seventeen for seventeen.

**And the address genuinely does not resolve for a consumer — verified on both
consumers in this repository** [2][3]:

```console
$ ls spec/
WAL.md  boot  common  design  manual-tests  modules  terraforms

$ ls packages/org.vibevm.fractality/fractality/v0.1.0/spec/
PROP-001-foundation.md  VISION-RECURSIVE-FABRIC.md  boot  examples
manual-tests  plans  refs  skills
```

Neither the host nor the `fractality` adopter has a `spec/flows/`. It resolves
inside the package [1] (`ls …/licensing/v0.1.0/spec/flows/licensing/` →
`LICENSING-PROTOCOL.md dependency-licenses.md eula-template.md`) and inside the
install slot [3] (`vibedeps/flow-licensing/0.1.0/spec/flows/licensing/`), and
nowhere a consuming session stands. **The verdict's underlying observation is
correct.** So is its aside that the campaign's `\.\./flows/` family scan cannot
see it: all seventeen are root-relative, no `../` anywhere.

**The defect is real. The question is whose sentence it is.**

### Anchor 1 — `licensing/v0.1.0/spec/flows/licensing/LICENSING-PROTOCOL.md#re-derive-prompt-lead` → **FALLS**

**Current text at HEAD** (`:146-147`) — the lead's **entire** content [1]:

```
146: ##re-derive-prompt-lead Paste this to
147: your agent in a fresh session: @impl/done
```

**What THAT claim is:** *the block below is a prompt to paste to an agent in a
fresh session.* It is an **instruction to the reader**, and it names no path, no
directory, no `spec/flows/`, and no property of the prompt's contents. Applying
[§6.1's](../PHASE-D-BATCH-PLAN.md#delegation-lessons) capability/practice/rule
triage: it is a **rule for the reader**, and the rule is followable — you can
paste the block. Whether the pasted block's first line then resolves is the
fenced body's property, not this sentence's.

**And the anchor two lines above it rules out the verbatim reading explicitly** [1]:

```
144: ##COPY-THE-PROMPT-TASK-NOT-THE-PROMPT-IMPLEMENTATION Copy the prompt-task, not the prompt-implementation. @impl/done
```

A literal `spec/flows/licensing/` inside the fence **is** prompt-implementation.
The document instructs the reader to take the task and not the implementation,
two lines before the anchor being convicted for the implementation. This is
[`##READ-FURTHER-BEFORE-SEARCHING-WIDER`](../PHASE-D-BATCH-PLAN.md#delegation-lessons)
in its literal form — *«the cheapest disproof is usually twelve lines down»*;
here it is **two lines up**, in the same section, and it is addressable.

**RECOMMENDATION: FALLS — re-judge `confirmed`.** The lead's own claim is true and
its own section carves out the reading that falsifies it. The path defect is
recorded below and belongs to the fenced body.

### Anchor 2 — `spec-genres/v0.1.0/spec/flows/spec-genres/SPEC-GENRES-PROTOCOL.md#re-derive-prompt-lead` → **FALLS**

**Current text at HEAD** (`:176-177`) [1]:

```
176: ##re-derive-prompt-lead Have the agent surface them and map yours onto this
177: frame: @impl/done
```

**What THAT claim is:** *have the agent surface your project's own genres and map
them onto this frame.* Again an instruction to the reader; again no path, no
directory, no `spec/flows/`. Its own antecedent («them») is the project's
existing genres, named in the anchor above [1]:

```
173: ##DO-NOT-ADOPT-THIS-TABLE-VERBATIM Do not adopt this table verbatim — your project already has genres,
174: named or not. @impl/done
```

**Honest note on the asymmetry:** `spec-genres`' carve-out is narrower than
`licensing`' — it forbids adopting *the genre table* verbatim, not the prompt.
So the licensing case is the stronger of the two. But the lead itself is equally
silent about paths in both, and that alone is sufficient: the sentence cannot be
false about a path it does not mention.

**RECOMMENDATION: FALLS — re-judge `confirmed`.**

### What a consistent treatment of all seventeen is — the measurement, then the two independent questions

**The house form of this family, measured over all 17** [1]. Scanning the twelve
lines above each fence for a do-not-copy-verbatim instruction:

```
packages whose pre-fence window carries a do-not-copy-verbatim carve-out: 16 of 17
```

The one exception is `discovery-prompt`, whose adjacent anchor
`##ARTIFACT-IS-READ-ONLY-BUT-BUILT-TO-BE-ADAPTED` says something close in
different words. **Eight of the sixteen name the carve-out in nearly identical
anchors**, which is what makes this a house form rather than a coincidence:

| package | the addressable carve-out immediately above the lead |
|---|---|
| addressable-specs | `##COPY-THE-PROMPT-TASK-NOT-THE-IMPLEMENTATION` |
| git-attribution-policy · licensing · operating-modes · secrets-hygiene · two-process-model | `##COPY-THE-PROMPT-TASK-NOT-THE-PROMPT-IMPLEMENTATION` |
| wal | `##COPY-THE-TASK-NOT-THE-IMPLEMENTATION` |
| decision-records | `##COPY-THE-TASK-NOT-THE-EXAMPLES` |
| health-audit | `##COPY-THE-TASK-NOT-THE-CATEGORY-LETTERS` |
| conflict-protocol · managed-blocks | `##ADAPT-IT-BY-HANDING-YOUR-AGENT-THE-TASK` («the task, not a copied template») |
| spec-genres | `##DO-NOT-ADOPT-THIS-TABLE-VERBATIM` |

**The current judgement spread, read from the cache as an instrument.** Of the 17,
**14 carry a lead verdict** (`re-derive-prompt-lead` or `re-derive-lead`); the
other 3 — conflict-protocol, decision-records, health-audit, managed-blocks —
name their pre-fence anchor differently and carry no `re-derive*` verdict:

```
confirmed: 12    drift: 2 (licensing, spec-genres)
```

**This corrects B-004's own arithmetic**, which says «eleven `confirmed`» in one
paragraph and «the other fifteen … are all judged `confirmed`» in another; at
HEAD it is **12 confirmed, 2 drift, 3 with no lead verdict**. The two drifts are
the outliers, not the pattern — and this batch finds the twelve to be right.

**The two questions are independent, and separating them is the whole answer to
B-004's scope question.**

1. **The verdict question — settled by the leads' own text.** Both drift verdicts
   convict a sentence of its neighbour's defect, against a carve-out the same
   section states. Consistent treatment: **all 17 leads read `confirmed`.** Two
   re-judgements, no edits, no publication. This costs nothing and removes F-240
   from the release queue.
2. **The repair question — and it is NOT a verdict question, because the fence
   carries no anchor.** Per [`B-004`](../../BACKLOG.md#b-004) and PROP-035 §7,
   fenced content is deliberately outside the fact model, so **repairing the
   fenced first line in all 17 packages changes no verdict at all** — nothing
   would be re-judged, before or after. That is the sharpest statement of B-004's
   finding: *the campaign has no instrument that can register this repair.*

   So the owner's decision is a **product** decision, not a closure: should the
   seventeen fenced prompts open with an address a consuming session can follow
   (`@spec://org.vibevm.world/<name>/flows/<name>/…`, the form
   [`##A1-ALL-POINTERS-NO-EMBEDS`](../PHASE-D-RELEASE-QUEUE.md#addresses-scope)
   already settled for the `../flows/` family), or does the house carve-out
   («copy the task, not the implementation») already discharge it? **Whichever
   way it goes, it goes for all seventeen** — [§4.5](../PHASE-D-BATCH-PLAN.md#release)
   makes two-of-seventeen «not a closure», and after this batch it would also be
   two-of-seventeen with **no defective verdict under it**.

**Recorded, not used — the «prompt was never run here» half.** The registry
reason concedes it («expected rather than damning — this host is the extraction
SOURCE»), and the concession is right: the host's licensing posture predates the
package (`spec/common/PROP-000.md` §3 `##LICENSE-REVISIT:57` records the
relicensing trigger as fired and spent on 2026-07-12). Ask (2) of the fenced
prompt — «list any current dependencies that violate it» — is the one output no
host artefact could show, and its absence is not evidence about either anchor.
Not load-bearing anywhere above.

---

## F-251 — pure package-own arithmetic; both anchors stand, and the sibling row is stronger than recorded

**Outcome:** BOTH **STAND**. This is the one obligation in the batch that needs no
consumer, no host observable and no ruling: the sentence contradicts the four
bullets under it, and the tree under those bullets. Entirely `[1]`.

**Anchors:** 2 of 2, and here the two sentences really are **word-identical**, so
the merge is sound and the repair is one word in each.

### Anchor 1 — `spec-genres/v0.1.0/README.md#package-contents-lead` → **STANDS**

**Current text at HEAD** (`README.md:32`) with its four bullets (`:34`, `:38`,
`:42`, `:45`) [1]:

```
32: ##package-contents-lead This package ships four pieces of content plus a boot snippet: @impl/done
34: - ##CONTENT-THE-PROTOCOL `spec/flows/spec-genres/SPEC-GENRES-PROTOCOL.md` — the taxonomy: …
38: - ##CONTENT-THE-DESIGN-DOCS-GUIDE `spec/flows/spec-genres/design-docs.md` — the contract/lore split …
42: - ##CONTENT-THE-ROUTING-GUIDE `spec/flows/spec-genres/when-to-write-what.md` — the routing table …
45: - ##CONTENT-THE-BOOT-SNIPPET `spec/boot/17-flow-spec-genres.md` — boot snippet: the genre map, …
```

**The self-contradiction, in one reading.** «four pieces of content **plus** a
boot snippet» promises five things. Four bullets follow — `grep -c '^- ##CONTENT-'`
→ **4** — and the fourth **is** the boot snippet. So the snippet is counted twice
and the content documents are over-counted by one.

**And the tree agrees** [1]:

```console
$ ls packages/org.vibevm.world/spec-genres/v0.1.0/spec/flows/spec-genres/*.md
SPEC-GENRES-PROTOCOL.md   design-docs.md   when-to-write-what.md          (3)

$ find packages/org.vibevm.world/spec-genres -type f | wc -l
7
```

Seven files: `LICENSE.md`, `README.md`, `vibe.toml`, `spec/boot/17-flow-spec-genres.md`,
and **three** documents under `spec/flows/spec-genres/`. Three pieces of content
plus a boot snippet.

### Anchor 2 — `tool-design-lessons/v0.1.0/README.md#package-contents-lead` → **STANDS**

**Current text at HEAD** (`README.md:22`) with its four bullets (`:24`, `:29`,
`:33`, `:38`) [1]:

```
22: ##package-contents-lead This package ships four pieces of content plus a boot snippet: @impl/done
24: - ##CONTENT-THE-CATALOG-INDEX `spec/flows/tool-design-lessons/TOOL-DESIGN-LESSONS.md` — the catalog: …
29: - ##CONTENT-THE-SELF-UPDATING-LESSONS `spec/flows/tool-design-lessons/self-updating-tools.md` — lessons S1–S7 …
33: - ##CONTENT-THE-PACKAGING-LESSONS `spec/flows/tool-design-lessons/packaging-lessons.md` — lessons P1–P7 …
38: - ##CONTENT-THE-BOOT-SNIPPET `spec/boot/70-flow-tool-design-lessons.md` — boot snippet loaded at session start …
```

Identical shape, identical defect [1]:

```console
$ ls packages/org.vibevm.world/tool-design-lessons/v0.1.0/spec/flows/tool-design-lessons/*.md
TOOL-DESIGN-LESSONS.md   packaging-lessons.md   self-updating-tools.md     (3)

$ find packages/org.vibevm.world/tool-design-lessons -type f | wc -l
7
```

### The sibling row, re-measured — and it lands stronger than either figure on record [1]

Both the original verdict («14 of 16») and the queue's
[§D](../PHASE-D-RELEASE-QUEUE.md#arithmetic) restatement («14 of 17, 1 says
«five»») are superseded by a stricter comparison. Matching on the **house
sentence shape** — `Th(is|e) package ships <N> pieces of content plus a boot
snippet` — and then checking each package's number against `ls` of its own
`spec/flows/<name>/*.md`:

```
world READMEs carrying ##package-contents-lead:            25   (of 28 world READMEs)
using the STRICT house shape:                              16
distribution:                                              three 14 · four 2
```

| says | packages | flow docs shipped | verdict |
|---|---|---:|---|
| «three» | addressable-specs, campaign-plans, comparative-research, conflict-protocol, decision-records, git-attribution-policy, managed-blocks, manual-tests, operating-modes, qualified-naming, secrets-hygiene, source-mirrors, sync-from-code, two-process-model | **3 each** (7 files each) | **14 / 14 correct** |
| «four» | **spec-genres, tool-design-lessons** | **3 each** (7 files each) | **2 / 2 wrong, both by one, same direction** |

**So the row is 14 of 16, every one of the 14 verified against its own tree
rather than counted as an opinion.** The nine READMEs excluded from the strict set
use a different sentence and are **not** outliers — each is correct under its own
shape, which is why removing them strengthens rather than weakens the comparison:

```
dev-runtime-docs   "This package ships:"                            1 flow doc  · 5 files
discovery-prompt   "ships the artifact plus two pieces of guidance"  2 flow docs · 6 files
git-atomic-commits "ships the **atomicity** discipline …"            2 flow docs · 6 files
git-autonomy       "This package ships:"                             1 flow doc  · 5 files
git-conventional-commits "This package ships:"                       1 flow doc  · 5 files
health-audit       "three flow documents, a skill, and a boot snippet"  3 flow docs · 8 files  ✓
licensing          "three pieces of content, a skill, and a boot snippet" 3 flow docs · 8 files ✓
wal                "The package ships five pieces of content plus a skill" 4 flow docs · 9 files
wal-specspaces     "What ships:"                                     1 flow doc  · 5 files
```

**Correction owed to §D:** `wal` is **not** a third outlier saying «five». Its
sentence ends «plus a **skill**», not «plus a boot snippet», and it ships four
flow documents plus a boot snippet plus a skill — a different shape with a
different denominator. Counting it as a dissenting «five» inflated the
denominator to 17 and manufactured a disagreement that is not there. The
comparable set is 16, and it splits 14–2.

**The convention the 14 keep, stated so the repair is mechanical.** `addressable-specs`
is the exemplar and its structure is byte-for-byte the same as the two outliers'
[1]:

```
22: ##package-contents-lead This package ships three pieces of content plus a boot snippet: @impl/done
24: - ##CONTENT-THE-PROTOCOL …
30: - ##CONTENT-THE-AUTHORING-RULES …
35: - ##CONTENT-THE-SPEC-TREE-LAYOUT …
38: - ##CONTENT-THE-BOOT-SNIPPET …
```

**The number names the flow documents; the snippet is the «plus a boot snippet»
and is listed as a fourth bullet for completeness.** Both outliers counted the
bullets instead of the pieces.

**Proposed correction (NOT APPLIED)** — one word in each, no other change:

```
spec-genres/v0.1.0/README.md:32
- ##package-contents-lead This package ships four pieces of content plus a boot snippet: @impl/done
+ ##package-contents-lead This package ships three pieces of content plus a boot snippet: @impl/done

tool-design-lessons/v0.1.0/README.md:22
- ##package-contents-lead This package ships four pieces of content plus a boot snippet: @impl/done
+ ##package-contents-lead This package ships three pieces of content plus a boot snippet: @impl/done
```

**RECOMMENDATION: STANDS** on both anchors. Route (a) — the package's own
statement is wrong, `falsifier: self` in substance, decidable inside each package
without any consumer. The only owner gate is publication, exactly as
[§D](../PHASE-D-RELEASE-QUEUE.md#arithmetic) says. Both packages take a version
bump and a re-vendor; the edit needs no ruling.

---

## Summary

| obligation | anchor (short) | recommendation | src classes used |
|---|---|---|---|
| **F-219** | `addressable-specs/README#COMPOSES-ATOMIC-COMMITS` | **STANDS-RESTATED** — misattribution real; count re-measured 579 commits / 732 lines at `f2b11b0a` (the recorded «716 commits» is a line count); `sync-from-code`'s identical defect already closed as F-253, route (a) | [1] [2] [3] |
| **F-219** | `campaign-plans/README#COMPOSES-ATOMIC-COMMITS` | **FALLS** — different sentence, no attribution claim; its own reason is a legacy-spec archived-vs-live ratio the owner voided; all three clauses live in the `fractality` adopter (8 phases with subjects spelled in advance, 3 ledgers, 58 hashes bound, sampled bindings verified against `git log`) | [1] [2] [3] |
| **F-220** | `addressable-specs/README#COMPOSES-WAL` | **ROUTE-OUT-CANDIDATE (§3.6(b))** — rule specified on both sides (`WAL-PROTOCOL.md:83`, `:92`); host WAL has 0 anchors in 28 constraint entries and 0 `spec://` citations; verdict measured `##SECTION-NEXT`, which carries no citation rule | [1] [2] [3] |
| **F-220** | `source-mirrors/README#COMPOSES-WAL` | **STANDS-RESTATED** — half (i) acquitted: `90-user.md:35` and `PROP-016:59` both back the package, `CLAUDE.md:191` is the odd one out (host-internal contradiction); half (ii) survives — no WAL entry anywhere notes «fanned out at <checkpoint>» and `flow:wal` never mentions mirrors | [1] [2] [3] |
| **F-233** | `git-attribution-policy/README#COMPOSES-DECISION-RECORDS` | **ROUTE-OUT-CANDIDATE (§3.6(b))** — the four-field form is the composed flow's own contract; the verdict searched for an ADR directory the flow forbids and missed the host's live four-field records; the load-bearing clause (PROP-000 §12 is a pointer, 0 fields) holds; **two** consumers install both flows, not one | [1] [2] [3] |
| **F-233** | `source-mirrors/README#COMPOSES-DECISION-RECORDS` | **ROUTE-OUT-CANDIDATE (§3.6(b))** — `PROP-016 ##HIST-AUTHORED` is 3 of 4 fields and the missing one is exactly what the sentence claims; `revisit\|parallel\|integrator` → 0 over 80 lines; 3 of 43 host spec docs carry the field at all | [1] [2] [3] |
| **F-240** | `licensing/…/LICENSING-PROTOCOL.md#re-derive-prompt-lead` | **FALLS** — the lead says only «Paste this to your agent in a fresh session» and names no path; `##COPY-THE-PROMPT-TASK-NOT-THE-PROMPT-IMPLEMENTATION` two lines above forbids the verbatim reading that convicts it | [1] [2] [3] |
| **F-240** | `spec-genres/…/SPEC-GENRES-PROTOCOL.md#re-derive-prompt-lead` | **FALLS** — the lead says only «Have the agent surface them and map yours onto this frame» and names no path; the defect is the fenced body's, which carries no anchor (B-004) | [1] [2] [3] |
| **F-251** | `spec-genres/README#package-contents-lead` | **STANDS** — «four … plus a boot snippet» is five; 4 bullets follow and the 4th IS the snippet; 3 flow docs, 7 files; 14 of 16 strict siblings say «three» and all 14 are correct against their own trees | [1] |
| **F-251** | `tool-design-lessons/README#package-contents-lead` | **STANDS** — identical sentence, identical defect; 3 flow docs, 7 files | [1] |

**Tally: 2 STANDS · 2 STANDS-RESTATED · 3 FALLS · 3 ROUTE-OUT-CANDIDATE.**
**Four of ten verdicts do not survive as written** (3 FALLS + 1 whose stated
ground is acquitted), and **three more leave the package in place**. If the boss
accepts, the `release` route's ask shrinks from five obligations to **two
publications** — F-219's `addressable-specs` half and F-251's two READMEs — plus
F-240's owner scope question, which after this batch is a product decision about
seventeen fenced prompts and **not** a closure any verdict can register.

**Route-tail check.** No package file was edited, no campaign state was written,
no verdict JSON was touched, and no `git` write command was run. The
re-verdict-edits-nothing basis this batch runs on is intact.

---

## Postscript — HEAD moved during the batch, and every figure was re-checked against the move

[`##THE-CAMPAIGN-IS-INSIDE-ITS-OWN-CORPUS`](../PHASE-D-BATCH-PLAN.md#delegation-lessons)
warns that this campaign *«moves its own measurements, inside a single
session»*. It did, here, and the check is recorded rather than the file quietly
re-stated.

**Between the first and last measurement, HEAD advanced three commits** —
`f2b11b0a` → **`12640d7c`**:

```console
$ git log --oneline f2b11b0a..HEAD
12640d7c fix(campaign): five boss obligations close, two by building and none by softening a package
a49a74c1 fix(world): the also-test stops calling itself mechanical, as its own third statement required
5b8c9cb6 fix(workspace): the malformed-block report names its marker lines, in both reporters

$ git diff --name-only f2b11b0a..HEAD
campaigns/packages-2026-09/run/cache.json
crates/vibe-check/src/checks/redirect_block.rs
crates/vibe-workspace/src/boot_artifacts.rs
crates/vibe-workspace/src/boot_artifacts/tests.rs
crates/vibe-workspace/src/lib.rs
packages/org.vibevm.world/git-atomic-commits/v0.1.0/spec/flows/atomic-commits/ATOMIC-COMMITS-PROTOCOL.md
```

**One of the six files is a file this batch cites** —
`ATOMIC-COMMITS-PROTOCOL.md`, under F-219. The diff touches
`##THE-TEST-IS-MECHANICAL-THE-WORD-ALSO` (`:93-98`) and `##SUM-THE-NO-ALSO-TEST`
(`:219`); **neither line carries a `spec://` URI**, and F-219's citation of that
file (`:78`, the decision-records pointer, one of the package's three `spec://`
occurrences) is untouched. Re-run at `12640d7c` [1] — identical to the figures in
the F-219 entry:

```console
$ grep -rn "spec://" packages/org.vibevm.world/git-atomic-commits/
  …/spec/boot/30-flow-atomic-commits.md:24                       (pointer to the sibling)
  …/spec/flows/atomic-commits/ATOMIC-COMMITS-PROTOCOL.md:78      (pointer to decision-records)
  …/spec/flows/atomic-commits/splitting-large-changes.md:96      (pointer to the sibling)

$ awk 'NR==26' …/git-atomic-commits/v0.1.0/spec/boot/30-flow-atomic-commits.md
##CONVENTIONAL-COMMITS-IS-THE-FORMAT-THIS-FLOW-IS-THE-ATOMICITY …

$ awk 'NR==24' …/git-conventional-commits/v0.1.0/spec/boot/31-flow-conventional-commits.md
##CITE-SPEC-URIS-WHERE-RELEVANT Cite `spec://…` URIs where relevant. @impl/done
```

**None of the other evidence files moved:**

```console
$ git diff --name-only f2b11b0a..HEAD -- \
    packages/org.vibevm.world/{addressable-specs,campaign-plans,source-mirrors,git-attribution-policy,licensing,spec-genres,tool-design-lessons,wal,decision-records} \
    spec/ vibe.lock vibedeps/ CLAUDE.md
(no output)
```

**The commit figure moved, exactly as §6.1 predicts, and it moves the
conclusion nowhere** [2]:

| figure | at `f2b11b0a` | at `12640d7c` |
|---|---:|---:|
| total commits | 2 212 | 2 215 |
| commits citing `spec://` (`git log --grep="spec://" --oneline \| wc -l`) | **579** | **581** |
| lines citing `spec://` (`git log --format=%B \| grep -c "spec://"`) | 732 | 735 |

Three commits of this campaign's own bookkeeping moved the commit count by 2 and
the line count by 3. **581 is still far below the recorded «716 commit bodies»**,
so the unit finding — that 716 is a line count — holds at both HEADs. The F-219
entry's figures are stated at `f2b11b0a` and are correct as stated; a reader
taking them forward should re-run the command rather than carry either number.

**And the ten verdicts are all still open.** The `12640d7c` closure («five boss
obligations close») touched none of this batch's anchors — re-read from
`run/cache.json` at the new HEAD, all ten still report `drift`. Nothing in this
file was overtaken.
