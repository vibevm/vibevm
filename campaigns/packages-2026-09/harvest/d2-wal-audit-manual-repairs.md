# D2 — `wal` · `health-audit` · `manual-tests` repairs

_Written 2026-07-29. Eighteen obligations, all confirmed `prose-edit` /
`release_event=False` before any file was opened:_

```
python campaigns/packages-2026-09/tasks/drift-registry.py --task F-137   (…and 17 more)
```

```
F-137: route=prose-edit release_event=False   F-141: route=prose-edit release_event=False
F-165: route=prose-edit release_event=False   F-164: route=prose-edit release_event=False
F-205: route=prose-edit release_event=False   F-235: route=prose-edit release_event=False
F-256: route=prose-edit release_event=False   F-310: route=prose-edit release_event=False
F-257: route=prose-edit release_event=False   F-311: route=prose-edit release_event=False
F-349: route=prose-edit release_event=False   F-312: route=prose-edit release_event=False
F-352: route=prose-edit release_event=False   F-149: route=prose-edit release_event=False
                                              F-317: route=prose-edit release_event=False
                                              F-318: route=prose-edit release_event=False
                                              F-319: route=prose-edit release_event=False
                                              F-320: route=prose-edit release_event=False
```

**The single fact that shapes this batch.** These three packages are flows whose
subject is the host's own practice, so almost every verdict in them reads *«the
package prescribes X and the host does not do X»*. Under
[§3.6](../PHASE-D-BATCH-PLAN.md#which-side) that is route **(b)** — the rule is
sound, the package does not move, a host obligation is recorded — and **not** an
invitation to soften the prescription until the measurement passes. The
route-(a) cases here are the minority and they are all the same shape: a
sentence that states a **fact about the world** (another package's shipped
content, a chapter number, the package's own model) and states it wrongly.

Where a verdict re-verified but lands on route (b), the entry is
**OUT-OF-ROUTE** and no byte moved.

---

## F-137 — «the only persistent memory» contradicts the package's own two-file model

**Outcome:** EDITED (1 of 11 anchors) · OUT-OF-ROUTE (10 of 11 anchors)
**Files touched:** `packages/org.vibevm.world/wal/v0.2.0/spec/flows/wal/WAL-PROTOCOL.xml`

**Re-verification.** Every measurement in the eleven reasons re-verified today,
and the file has grown since the verdict was written:

```
$ wc -lwc spec/WAL.xml
  344  3296 22006 spec/WAL.xml          (verdict measured 299 / 2 914 / 18 972)

$ sed -n '1,4p' spec/WAL.xml
# WAL — Project Continuation State

_Updated: 2026-07-29 (**PHASE C IS CLOSED — 6 847 / 6 847 anchors, zero owed, all
seven world batches complete and the exit gate discharged**)_

$ for c in $(git log -n 14 --format=%h -- spec/WAL.xml); do git show "$c:spec/WAL.xml" | sed -n '3p'; done
  → 14 of 14 carry a bare calendar date (`_Updated: 2026-07-29 (…`), 0 carry an
    ISO-8601 timestamp. The protocol's own worked example at WAL-PROTOCOL.xml:179
    is `_Updated: 2026-04-16T18:23:00Z_`.

$ grep -n '^## ' spec/WAL.xml
14:## Current phase        →  lines 14-79 = 66 lines, against «one or two lines»
80:## Constraints — do not violate
183:## Done (collapsed — see `git log`)
203:## In progress
231:## Next                →  six numbered items, none marked default
255:## Known issues
307:## Session context

$ sed -n '80,182p' spec/WAL.xml | grep -cE '^- '                      → 28
$ sed -n '80,182p' spec/WAL.xml | grep -E '^- ' | grep -cE 'spec://|#[0-9]+'  → 0
$ grep -n 'spec://' spec/WAL.xml
341:cannot be cited. `spec://` occurs zero times in this file, in every revision
  → the file's single `spec://` occurrence is the WAL *narrating this very
    finding*, not a citation. Zero constraints cite a spec anchor or an issue.
```

So the eleven reasons hold, and on the size and Next-section legs they hold
harder than when they were written.

**What changed and why.** Exactly one anchor moved, and it is the only one of
the eleven whose falsifier sits **inside the package**.
`FOR-THE-AGENT-IT-IS-THE-ONLY-PERSISTENT-MEMORY` claimed the WAL is *«the only
persistent memory»* — which the same document contradicts two sections earlier:
`FILE-CONTINUE-IS-THE-SUBORDINATE-COLD-RESUME-SNAPSHOT` (WAL-PROTOCOL.xml:20-23)
specifies `CONTINUE.md` as a second session-durable repository file, and
`RESUMPTION-STATE-LIVES-IN-THE-REPOSITORY-NEVER-IN-A-SESSION` (:25) makes the
repository the medium. The sentence is a survival of the one-file model that
0.2.0 replaced. One word changed — *only* → *canonical* — which is precisely
what `FILE-WAL-IS-THE-CANONICAL-LIVING-CHECKPOINT` (:17-19) already establishes:
*«where it and any snapshot disagree, the WAL wins»*. The second half, *«the WAL
is read first»*, was **left untouched**: it restates the flow's own boot rule
(the one F-352 covers), and softening it here to match the host's boot order
would be weakening a rule to make a measurement pass.

**Why the other ten anchors did not move — route (b).** Every one of them is a
**prescription** about how a WAL is to be written, and every verdict against
them is a report that this host writes its WAL differently:

| anchor | the rule | what the host does |
|---|---|---|
| `NOT-A-TO-DO-LIST` | Next names *one* action | six numbered items |
| `SECTION-UPDATED-LINE` | ISO-8601 UTC, «always and without exception» | bare date, 14/14 |
| `SECTION-CURRENT-PHASE` | one or two lines | 66 lines |
| `SECTION-CONSTRAINTS` | each *why* cites a spec anchor or issue | 0 of 28 |
| `SECTION-IN-PROGRESS` | cite `spec://` anchors | 0 occurrences |
| `SECTION-NEXT` | single next action, mark the default | six, none default |
| `SECTION-SESSION-CONTEXT` | one-paragraph orientation | multi-section retrospective |
| `MORE-AUTONOMY-DEMANDS-A-MORE-PRECISE-WAL` | constraints explicit **and spec-anchored** | not anchored |
| `BUDGET-TARGET` | ≤ 3 000 tokens | ~5 500 at 4 bytes/token |
| `SUM-SIZE-AND-CONSTRAINTS` | «keep it under a page» | 344 lines |

Rewriting any of these to describe what the host does is the *профанация* §3.6
exists to prevent: the timestamp rule exists so the 24-hour freshness rule can be
evaluated (`WAL-OLDER-THAN-24-HOURS-IS-PRESUMED-STALE`, :139), the budget exists
because `EVERY-TOKEN-IS-A-TAX-PAID-PER-SESSION` (:151), and both are the
package's product. **The obligation therefore cannot close on this edit** — ten
anchors need a host-side obligation (or a §3.6(c) written exception), not a
package edit.

**New obligations noticed:** the host's `spec/WAL.xml:341` now contains prose
*about* the campaign's own finding that `spec://` never appears in it. A durable
artefact narrating its own audit verdict is a new kind of self-reference; worth
a look but outside these eighteen. Not touched.

---

## F-165 — the wind-down hook's five rules, all of which the host keeps loosely

**Outcome:** OUT-OF-ROUTE (5 of 5 anchors)
**Files touched:** «none»

**Re-verification.** Both load-bearing measurements re-verified, one of them
with a different method than the verdict used, and I record my number rather
than the verdict's:

```
$ git log --since=2026-06-01 --date=short --format=%ad | sort -u | wc -l          → 38
$ git log --since=2026-06-01 --date=short --format=%ad -- spec/WAL.xml | sort -u | wc -l → 29
$ comm -23 <(all days) <(WAL days)
2026-06-18  2026-06-19  2026-06-20  2026-06-26  2026-07-06
2026-07-10  2026-07-11  2026-07-18  2026-07-21
  → nine active days (23.7 %) carried commits and no WAL commit.
    The verdict said 37/28 and nine days; one more day has elapsed since.
```

```
$ for the 18 most recent commits touching spec/WAL.xml, compare line 3 pairwise
consecutive pairs compared: 17 ; pairs with byte-identical _Updated: line: 6
  (fac57627, bdc52412, 0c586c44, cf6c7927, 6a026de1, 95937de5)
```

The verdict claimed 10 of 17 (58 %). **My method measures a different thing** —
it compares consecutive revisions of line 3, where the verdict counted commits
that edited the body while leaving the line untouched — and gives 6 of 17
(35 %). The direction is identical and the rule is plainly broken repeatedly;
the magnitude in the verdict is not reproducible by my command and I do not
adopt it. Either number settles the same question, and neither changes the
route.

Confirmed against the compiled host lane:

```
$ grep -n 'Read `spec/WAL.xml` \*\*before\*\*' spec/boot/STATIC.xml   → 1382
$ wc -l spec/boot/STATIC.xml                                          → 1584
```

**What changed and why — nothing, and this is the §3.6(b) case in its purest
form.** All five anchors are prescriptions, and each verdict is a report that
the host keeps the rule loosely:

- `HOOK-FIRES-IMPLICITLY` — «every session that touched project state ends with
  at least steps 1–3». Nine active days ended without one.
- `REWRITE-THE-FILE-DO-NOT-PATCH-OR-APPEND` — the host's own wind-down verb is
  «Update … bump … refresh» (`CLAUDE.md:189`), a patch instruction.
- `NEVER-LEAVE-THE-UPDATED-LINE-UNTOUCHED` — a `## Never` entry, broken above.
- `SUM-WHEN-THE-HOOK-RUNS` / `SUM-THE-STEPS-IN-ORDER` — summaries of the same.

Editing any of these means writing down a laxer hook because the consumer runs
a laxer hook. `REWRITE-THE-FILE-DO-NOT-PATCH-OR-APPEND` is load-bearing for the
whole flow (`WAL-IS-A-WHITEBOARD-NOT-A-JOURNAL`, `NOT-A-LOG`,
`NEVER-APPEND-TO-THE-WAL`); demoting it to «update» would gut the checkpoint
model the package exists to ship. The package does not move. A host obligation
is owed on all five.

**One caveat the boss should carry into the host obligation.**
`SUM-THE-STEPS-IN-ORDER`'s verdict is only *half* a host finding — it also says
*«All three divergences trace to the package's own cold-resume.xml §wind-down»*.
That half is an internal package contradiction and it is the whole of F-349;
see that entry. Closing F-165 host-side without closing F-349 package-side would
leave the host obeying one half of the package and failing the other.

**New obligations noticed:** none beyond F-349, already an obligation.

---

## F-205 — a deference that never shipped, and a chapter range that excludes its own first idea

**Outcome:** EDITED (2 of 3 anchors) · OUT-OF-ROUTE (1 of 3 anchors)
**Files touched:** `packages/org.vibevm.world/wal/v0.2.0/README.md`

**Re-verification.** Both edited claims are statements about **other packages'
shipped bytes**, so both are settled by reading those bytes:

```
$ ls packages/org.vibevm.ai-native/core-ai-native/
v0.7.0
v0.8.0

$ ls -la packages/org.vibevm.ai-native/core-ai-native/v0.8.0/spec/06-WAL-CONVENTION.xml
-rw-r--r-- 1 olegc 197121 5350 Jul 26 15:56 …/v0.8.0/spec/06-WAL-CONVENTION.md

$ grep -nE 'flow:wal|org\.vibevm\.world/wal|defer' \
    packages/org.vibevm.ai-native/core-ai-native/v0.8.0/spec/06-WAL-CONVENTION.xml
NO MATCH — zero occurrences

$ sed -n '5p' …/v0.8.0/spec/06-WAL-CONVENTION.md
##status-line **Discipline v0.2 · status: BETA · T1 · language-neutral · OPTIONAL but preferred**

$ git log --diff-filter=A -1 --date=short --format='%h %ad %s' -- \
    packages/org.vibevm.ai-native/core-ai-native/v0.8.0/spec/06-WAL-CONVENTION.xml
bfb72da7 2026-07-17 feat(ai-native): core-ai-native 0.8.0 — Go support in the neutral engine
```

`v0.8.0` **is** «its next release» after the `v0.7.0` this README was written
against, it landed 2026-07-17, and it ships the full rival convention with zero
occurrences of *defer*, *flow:wal* or *org.vibevm.world/wal*. The claim is false
by inspection.

```
$ ls packages/org.vibevm.world/redbook/v0.2.0/spec/book/ru/
chapter-1-two-process-model.xml
chapter-2-shared-state-and-files.xml
chapter-3-memory-individual.xml

$ head -4 of each → # Глава первая.<br>Два процесса, одна задача
                    # Глава вторая.<br>Shared state: файлы как IPC
                    # Глава третья.<br>Архитектура памяти
```

The fact lists four ideas and the **first of them** — «two-process cooperation»
— is chapter *one*, not 2–3. The range excluded the chapter it names first.

**Perimeter searched** (for «nothing in the repository cites this package as its
WAL authority», the third anchor's basis). Rooted at the repository root, over
the host tree and not only the package:

```
$ grep -rnE 'org\.vibevm\.world/wal|flow:wal|flow:org\.vibevm\.world/wal' \
    CLAUDE.md AGENTS.md GEMINI.md MEMORY.md TASKS.md BACKLOG.md AUDIT.md \
    ROADMAP.md SPECSPACES.md vibe.lock vibe.toml \
    spec/ crates/ xtask/ tools/ discipline/ research/ campaigns/ legacy-spec/
```

37 hits, and **not one of them cites this package as the host's WAL authority.**
They are: `ROADMAP.md` and `spec/modules/**` using `flow:wal` as the worked
package-manager *example* / e2e fixture (`spec/common/PROP-000.xml:92-94,105,173`,
`spec/modules/vibe-registry/PROP-002…:69,199,219`,
`spec/modules/vibe-index/PROP-005…:19,308`,
`spec/design/workspace-and-qualified-naming.xml:62,76,81,82`); `vibe.lock:62`
and `spec/boot/STATIC.xml:1369` recording that it is *installed and compiled in*;
and `CLAUDE.md:141` / `SPECSPACES.md:5-6` naming the **different** package
`flow:org.vibevm.world/wal-specspaces`. Installation is not citation of
authority, and no host document names a WAL authority at all.

**What changed and why.** Two sentences, each the smallest edit that makes it
true.

1. `DISCIPLINE-DEFERS-TO-THIS-PACKAGE-FROM-ITS-NEXT-RELEASE` now says the
   Discipline **still ships its own** convention document, marked OPTIONAL but
   preferred, and that the deference has not landed in a release yet. This is a
   pure reality repair: the sentence asserted a fact about `core-ai-native`'s
   shipped content and the shipped content refutes it.
2. `ideas-are-chapters-two-and-three` — «chapters 2–3» → «chapters 1–3». One
   token. The `0.1.0` README said 1–3; `0.2.0` narrowed the range and kept
   chapter 1's subject in the list, which is the defect exactly.

**Two things the boss must see.**

- **The anchor ids now read against their own text.**
  `DISCIPLINE-DEFERS-TO-THIS-PACKAGE-FROM-ITS-NEXT-RELEASE` names a deference
  the repaired sentence says has not happened, and
  `ideas-are-chapters-two-and-three` names a range that is now 1–3. Both were
  left **exactly as they were** per `RULE-ANCHORS-IMMUTABLE` and the brief's
  «do not add, remove or rename any `##ANCHOR` fact id». Flagging it rather
  than acting on it.
- **`PACKAGE-IS-THE-CANONICAL-HOME-OF-THE-WAL-CONVENTION` did not move, and is
  OUT-OF-ROUTE.** Its falsifier is *another package refusing to cede* —
  `core-ai-native` stating the same two-file model, the same canonicity rule and
  the same supersession while naming this package nowhere, with **both installed
  and booted** (`vibe.lock:62`; `spec/boot/STATIC.xml:1369`). Making that
  sentence true means editing `core-ai-native` and re-vendoring it, which is a
  cross-package fix and therefore [§4.5](../PHASE-D-BATCH-PLAN.md#release)'s
  release event — owner, before publication. The alternative, deleting this
  package's canonicity claim, decides an ecosystem question in the owner's place
  and is not a prose repair. With the deference sentence now honest, a reader
  gets the claim and its unsettled status one sentence apart, which is the most
  a prose edit can do here.

**New obligations noticed:** the host boots **two** complete WAL conventions —
`spec/boot/STATIC.xml:1369` compiles this package's snippet, and `core-ai-native`
lists `06-WAL-CONVENTION.xml` among its playbooks. That is a live `duplication`
against the host's own boot lane, distinct from this obligation's README claim.
Recorded, not fixed.

---

## F-256 — cold start reads `CONTINUE.md` first; the host's resume command reads it second

**Outcome:** OUT-OF-ROUTE (2 of 2 anchors)
**Files touched:** «none»

**Re-verification.** The two host citations re-verified verbatim:

```
$ sed -n '3p' CLAUDE.md
Every session in this repository begins by reading this file, then every file in
`spec/boot/` in filename order, then `spec/WAL.xml`, then any relevant PROP/FEAT
documents under `spec/common/` and `spec/modules/` for the task at hand. Only
after that, start work.

$ grep -n 'Run the full boot sequence' CLAUDE.md
205:1. Run the full boot sequence (this file → `spec/boot/INDEX.md` and its files →
    `spec/WAL.xml`), read `CONTINUE.md`, and verify repository state empirically…
```

**The reason holds, but for a narrower cause than it states — recorded per the
brief's third outcome.** The verdict indicts `CLAUDE.md:3` *and* `:205`. Only
`:205` actually bears: `READ-CONTINUE-FIRST-THEN-THE-WAL` is scoped to **cold
starts** — the section head is *«Coming back after a machine switch or a long
gap, or arriving at a repository that is not yours?»* (morning-routine.xml:70-73)
— and `CLAUDE.md:3` is the *ordinary* boot order, not a cold start, so its
silence about `CONTINUE.md` is out of the rule's scope rather than contrary to
it. `CLAUDE.md:205`, the `RESUME SESSION` command, **is** the host's cold-start
path, and it does reverse the order: boot sequence ending at `spec/WAL.xml`,
`CONTINUE.md` after. One host document contradicts the flow, not two.

**Perimeter searched.** The claim above is about scope, not an absence, but I
checked that the host has no *other* cold-start contract that does honour the
order:

```
$ grep -rn 'CONTINUE' CLAUDE.md spec/boot/*.md | grep -v STATIC.md
CLAUDE.md:141, :144   specspace boot scoping (a specspace's own CONTINUE.md)
CLAUDE.md:178, :190   the wind-down (writes it)
CLAUDE.md:194         the wind-down's rationale
CLAUDE.md:205, :206, :207   the resume command
$ grep -c 'CONTINUE' spec/boot/STATIC.xml   → 4   (this package's own snippet)
```

The only host cold-start entry point is `CLAUDE.md`'s resume command at :205,
mirrored in `AGENTS.md` / `GEMINI.md`, which `CLAUDE.md:150` states are kept
identical. Nothing else in the host boot contract orders the two files, so the
verdict's «two host documents» is really one, twice-mirrored.

**What changed and why — nothing.** `READ-CONTINUE-FIRST-THEN-THE-WAL` is a
prescription with a stated reason one section above it:
`THE-COLD-READER-NEEDS-THE-TOUR` — *«where things are, what commands run, what
was decided and why — before the one-page checkpoint means anything»*
(cold-resume.xml:21-23). Reversing it in the package so the host's order passes
would sell the cold reader the checkpoint before the map, which is the exact
failure the rule exists to prevent. Route (b): the rule is sound and the host
should keep it. `SUM-COLD-START-ORDER` carries the same verdict and likewise
does not move.

**The package is internally consistent here**, which is worth stating because
F-349 shows it is not everywhere: `cold-resume.xml`'s
`RESUME-STEP-RUN-THE-BOOT-SEQUENCE` (:118-119) also orders *«then read
`CONTINUE.md` and `spec/WAL.xml`»* — same order, no rival list. The disagreement
is host-side only.

**New obligations noticed:** a host obligation is owed on `CLAUDE.md:205` — the
resume command should read `CONTINUE.md` before the WAL, or record a §3.6(c)
written exception saying why this host deliberately does otherwise. Recorded,
not fixed (host file, outside my edit perimeter).

---

## F-257 — «never appended to, never patched», and the host does both

**Outcome:** OUT-OF-ROUTE (2 of 2 anchors)
**Files touched:** «none»

**Re-verification.** The size half re-verified with the same command as F-137
(344 lines / 3 296 words / 22 006 bytes → ~5 500 tokens at 4 bytes/token, ~4 280
at words × 1.3; over 3 000 either way). The wholesale half I re-measured
directly, and it comes out **worse** than the verdict recorded:

```
$ for c in $(git log -n 14 --format=%h -- CONTINUE.md); do
    git show --numstat --format= "$c" -- CONTINUE.md; git show "$c^:CONTINUE.md" | wc -l; done

14e33e46  +224  -199   (before: 332)   ← wholesale
fac57627  +40   -21    (before: 313)
42de8b5a  +15   -0     (before: 298)   ← pure APPEND, zero deletions
4e81921c  +254  -397   (before: 441)   ← wholesale
0c586c44  +12   -6     (before: 435)
6ba4d6f8  +98   -30    (before: 367)
2edd6e54  +34   -0     (before: 333)   ← pure APPEND, zero deletions
b012460a  +3    -3     (before: 333)
5402cfe9  +202  -250   (before: 381)   ← wholesale
f0dc7b04  +26   -11    (before: 366)
ba959fdb  +209  -265   (before: 422)   ← wholesale
48025285  +32   -18    (before: 408)
8406eb2a  +29   -8     (before: 387)
100617b3  +4    -3     (before: 386)
```

**Four of fourteen are wholesale rewrites; ten are patches.** And the verdict
understated the defect: it said the rule's core (*«overwritten wholesale»*) holds
because every *wind-down* write is wholesale, and only *«never patched»* fails.
But `42de8b5a` (+15/-0) and `2edd6e54` (+34/-0) delete **nothing at all** —
those are appends, so the clause the verdict said the host keeps,
*«never appended to»*, is broken twice in the same fourteen commits. Recorded as
the real cause; it does not change the route.

**What changed and why — nothing, both anchors are prescriptions.**

- `CONTINUE-IS-OVERWRITTEN-WHOLESALE` — *«never appended to, never patched;
  staleness compounds otherwise»* is a rule with its reason inline, and it is
  the one the host is failing. Rewriting it to permit patching because the host
  patches is §3.6's forbidden direction exactly.
- `WAL-IS-WRITTEN-FOR-THE-SAME-PROJECT-RHYTHM` — *«terse, current, one page»*
  restates `BUDGET-TARGET`'s ≤ 3 000 tokens as a property of the file. Same
  ruling as F-137's `BUDGET-TARGET` and `SUM-SIZE-AND-CONSTRAINTS`: the budget
  is the package's product, and the host being over it is a host fact.

**New obligations noticed:** none new — the host obligation here is the same
wind-down-discipline obligation F-165 already owes, extended to `CONTINUE.md`.

---

## F-349 — the package ships two rival wind-down orderings, and the repair lands outside these eighteen

**Outcome:** OUT-OF-ROUTE (1 of 1 anchor)
**Files touched:** «none»

**Re-verification.** This one is settled entirely inside the package, by reading
the two lists against each other. I read both files in full:

`session-end-hook.xml` — six numbered sections, and the anchor under judgement
(`RUN-THE-FULL-HOOK-AS-A-HARD-CONTRACT`, :33-35) orders *«the full hook, steps
1–6»*:

```
:37  ## 1. Confirm the work is in a good stopping state
:47  ## 2. Rewrite `spec/WAL.xml`
:85  ## 3. Collapse aggressively
:92  ## 4. Overwrite `CONTINUE.md`
:107 ## 5. Commit — propose by default
:124 ## 6. Report
```

`cold-resume.xml` §wind-down — five numbered steps under
`required-behaviour-lead` (:87), *«Required behaviour, in order»*:

```
:89  1. STEP-OVERWRITE-CONTINUE            Overwrite `CONTINUE.md`
:90  2. STEP-REWRITE-THE-WAL               Rewrite `spec/WAL.xml`
:93  3. STEP-COMMIT-IN-TOPIC-GROUPED-COMMITS
:96  4. STEP-PUSH-ONLY-IF-AUTONOMY-SANCTIONS-IT
:99  5. STEP-EMIT-A-CHAT-TLDR
```

The verdict is confirmed and is more than a count mismatch: **the WAL/`CONTINUE`
order is literally reversed between the two**, the stopping-state confirmation
and the collapse step have no counterpart in the five, and
`WIND-DOWN-IS-THE-EXPLICIT-FORM-OF-THE-HOOK` (cold-resume.xml:76) asserts the two
are the same procedure. A consumer reading the package end to end is told two
different things.

**Which side is wrong, decided from the package's own text.** The hook states
WAL-before-`CONTINUE` in **three** places — `scope-of-this-document` (:5-7,
*«confirm a good stopping state, rewrite `spec/WAL.xml`, overwrite
`CONTINUE.md`, report»*), the numbered sections above, and
`SUM-THE-STEPS-IN-ORDER` (:164-165). `cold-resume.xml` states
`CONTINUE`-before-WAL in **two** — the §wind-down list and `SUM-WIND-DOWN-SHAPE`
(:179-180). The hook is also the document whose declared subject *is* the
procedure, where `cold-resume.xml`'s declared subject is *«the second file of the
discipline: `CONTINUE.md`»* (:5-9). **So the hook is authoritative and
`cold-resume.xml` §wind-down is the deviating restatement.**

**Why nothing moved.** The repair that closes this obligation lands on
`cold-resume.xml`'s §wind-down step list — anchors `STEP-OVERWRITE-CONTINUE`,
`STEP-REWRITE-THE-WAL` and the `required-behaviour-lead` around them. **None of
those five anchors is in my eighteen** (F-257, the only other obligation on that
file, holds `WAL-IS-WRITTEN-FOR-THE-SAME-PROJECT-RHYTHM` and
`CONTINUE-IS-OVERWRITTEN-WHOLESALE` and nothing else). Editing them is fixing
something outside my assignment, which the brief forbids — record, do not fix.

The anchor I *was* assigned, `RUN-THE-FULL-HOOK-AS-A-HARD-CONTRACT`, is on the
**correct** side of the contradiction: nothing in it is false, and softening it
to «steps 1–5» or dropping the stopping-state and collapse beats would delete two
rules (`NEVER-PAPER-OVER-A-BROKEN-STOPPING-STATE`,
`COLLAPSE-DONE-TO-ONE-LINE-EACH`) to make a rival list agree. That is the §3.6
forbidden direction in the sharpest form it takes in this batch.

**The recommendation, for the boss to act on or reject.** Bring
`cold-resume.xml` §wind-down into the hook's order and completeness — reordering
the five existing `STEP-*` anchors, not renaming them, and adding the two beats
the hook has — or, cheaper and equally honest, have `required-behaviour-lead`
stop presenting itself as the complete ordering and defer to the hook for the
procedure, enumerating only what is specific to the *explicit* form. Either is a
prose edit on the `prose-edit` route; both need a decision I was not given the
anchors to make.

**New obligations noticed:** `SUM-WIND-DOWN-SHAPE` (cold-resume.xml:179-180)
carries the same reversed order and is not in any of my eighteen. Whichever way
the above is settled, that summary moves with it or it becomes a third writer for
the same ordering. Recorded, not fixed.

---

## F-352 — «read the WAL before doing anything else», in a snippet the host reads third

**Outcome:** OUT-OF-ROUTE (1 of 1 anchor)
**Files touched:** «none»

**Re-verification.** Every leg of the reason re-verified:

```
$ sed -n '3p' CLAUDE.md
Every session in this repository begins by reading this file, then every file in
`spec/boot/` in filename order, then `spec/WAL.xml`, …

$ sed -n '9,13p' spec/boot/00-core.xml
##boot-sequence-lead Every session starts here. In order:
1. ##BOOT-STEP-BOOT-DIR Read this file and the rest of `spec/boot/` end to end …
2. ##BOOT-STEP-WAL Read `spec/WAL.xml` — current project state (checkpoint, not log).
3. ##BOOT-STEP-PROPS Read the relevant PROP/FEAT …
4. ##BOOT-STEP-START-WORK Only then start work.

$ grep -n 'Read `spec/WAL.xml` \*\*before\*\*' spec/boot/STATIC.xml   → 1382
$ wc -l spec/boot/STATIC.xml                                          → 1584
```

Confirmed in both independent host files, and the compiled position is confirmed
too: this package's instruction sits at line 1382 of a 1584-line static lane the
host reads *before* opening the WAL.

**What changed and why — nothing.** `READ-THE-WAL-BEFORE-DOING-ANYTHING-ELSE` is
the flow's first prescription, and the host's boot contract orders the WAL third.
Rewriting it to «read the WAL third» because this consumer reads it third writes
one consumer's layout into a package shipped to all of them. Route (b): the rule
is sound, the package does not move, a host obligation is owed.

**A second reading the boss should weigh before opening that host obligation.**
The verdict reads *«before doing anything else»* strictly — as *before reading
anything else* — and on that reading the instruction is unsatisfiable **by
construction**, because a boot snippet is only ever delivered as part of a boot
lane; no compiled snippet can be read before the lane that carries it. On the
looser reading — *before doing any work*, which is how `spec/boot/00-core.xml`'s
own step 4 phrases the same idea (*«Only then start work»*) — the host complies
and there is no drift at all. The fact sits under the heading `## At the start of
every session`, which supports the looser reading. **I did not edit on that
reading**, because disambiguating a shipped normative sentence is a rule change
dressed as a clarification, and because either way the sentence is not repaired
by a measurement of one consumer. If the boss takes the looser reading, this is a
`RE-JUDGE: confirmed`, not an edit; if the strict one, it is a host obligation
plus a package-side wording question that belongs to the owner.

**New obligations noticed:** none.

---

## F-141 — two overstatements the host's own record falsifies; seven prescriptions the host does not keep

**Outcome:** EDITED (2 of 9 anchors) · OUT-OF-ROUTE (7 of 9 anchors)
**Files touched:**
`packages/org.vibevm.world/health-audit/v0.1.0/spec/flows/health-audit/HEALTH-AUDIT-PROTOCOL.xml`

**Re-verification.** The artefact's shape first, since seven of the nine verdicts
rest on it:

```
$ grep -n '^## ' AUDIT.md
20:## Audit run — 2026-05-23 (seed)
154:## Audit run — 2026-06-10 (terraform close-out, instrumented category C)
191:## Audit run — 2026-06-12 (discipline depth — the full AI-Native sweep)

$ sed -n '22,25p' AUDIT.md
… it is **not** a fresh full sweep of the PROP-013 §2.2 checklist. The first
full sweep is the next invocation; this seed gives that run a populated
inventory to carry forward …

$ sed -n '156,160p' AUDIT.md
… scoped to what the new machinery can feed the audit automatically — category
**C (drift)** plus the gate panel. It is **not** the full §2.2 breadth sweep
(INT-0001 stays rescoped to the next audit window) …

$ git log -1 --date=short --format='%h %ad %s' -- AUDIT.md
3656f362 2026-06-12 docs(audit): AUD-0016 dispositioned fixed - the posture is live
```

Three runs, two disclaiming the breadth sweep in their own text, the third
scoped to the category it had just added, and nothing since 2026-06-12.

The two **edited** anchors were verified separately, and both are settled by the
host's own durable record rather than by any absence:

```
$ sed -n '20,22p' spec/common/PROP-013-periodic-health-audit.xml
- ##M119-TEST-ASSERTED-BROKEN … a `cli_init` test even *asserted the broken
  value as correct*, staying green the whole time.
- ##M119-CAUGHT-BY-SMOKE It was caught only by a live smoke run during the
  registry migration.
```

**What changed and why.**

1. `ONLY-A-READER-CATCHES-A-TEST-THAT-ENCODES-THE-WRONG-ANSWER` claimed that
   *«only a reader judging the assertion against the intent»* can catch a test
   that encodes the wrong answer. The flow's **own canonical instance** — the
   broken initializer, narrated three facts earlier at
   `the-canonical-failure-is-the-broken-initializer` (:50-53) — was caught by
   neither the gate nor a reader: `PROP-013 ##M119-CAUGHT-BY-SMOKE` records *«It
   was caught only by a live smoke run»*. The sentence excluded the mechanism
   that actually worked. Repaired to name it, and the thesis the sentence serves
   is untouched and if anything sharper: what caught it was still **outside the
   gate**, which is the section's whole argument
   (`not-more-gate-but-a-different-activity`, :59). This is a factual
   overstatement corrected, **not** a rule softened — the fact carries
   `@spec/done` and is rationale prose, not a prescription.
2. `TWO-FORCES-RESHAPE-THE-CHECKLIST-EVERY-RUN` claimed both forces fire *«every
   run»*. The two forces it introduces are both **conditional in their own
   text** — `FORCE-A-NEW-DEFECT-CLASS-BECOMES-A-PERMANENT-CATEGORY` says *«When
   a run discovers a kind of rot the checklist did not name…»* and
   `FORCE-A-MECHANISABLE-CATEGORY-MIGRATES-INTO-THE-GATE` says *«When a category
   can be checked by a script, it should»*. Read as a rule, «every run» would
   oblige a run that discovered no new defect class to invent one, which neither
   force asks for; read as a description, it is false here (one of three runs
   applied both, two applied neither). Changed to *«over time, each when its own
   condition holds»* — which removes a frequency claim the package's own
   conditionals never made and **weakens neither force**.

**Why the other seven did not move — route (b).** All seven are prescriptions,
and every verdict against them reports that this host's three runs did not meet
them:

| anchor | the rule | what the host did |
|---|---|---|
| `THE-AUDIT-IS-A-DIFFERENT-KIND-OF-CHECK` | a breadth-first sweep | two runs disclaim the sweep |
| `AN-AUDIT-RUN-WALKS-ONE-CATEGORY-GROUP-PER-BLIND-SPOT` | walk A, B, C, D | nothing ever filed under D2/D3/D4 |
| `ROW-FIELD-ID` | `<run-date>-NN` | one run used repo-wide `AUD-00NN` |
| `ROW-DISP-FILED` | the finding records where it was filed | 3 of 9 record it |
| `AUDIT-IS-OWNER-TRIGGERED-WITH-A-ONCE-PER-MILESTONE-FLOOR` | floor once per milestone | two milestones shipped un-audited |
| `A-RUN-MUST-FINISH-THE-INVENTORY-NOT-EVERY-FIX` | finish the inventory | two runs did not |
| `SUM-OWNER-TRIGGERED-FLOOR-ONCE-PER-MILESTONE` | summary of the floor | same |

The floor is the package's product — `AUDIT-IS-OWNER-TRIGGERED-…` exists so that
*«a milestone is never declared done on an un-audited base»*, and demoting it
because two milestones were is §3.6's forbidden direction. Same for the
breadth-first walk, which is the whole difference between the audit and the gate.

**One route-(a) candidate I deliberately did not take.** `ROW-DISP-FILED`
enumerates destinations — *«a checkpoint "known issues" entry, a `TASKS.md`
line, or a design note»* — and the host files to a destination the flow names
nowhere:

```
$ grep -cin 'audit' TASKS.md                 → 0
$ ls -la discipline/registry/debt.json       → exists, 26 026 bytes
$ grep -rn 'debt.json\|DEBT.md' packages/org.vibevm.world/health-audit/v0.1.0/
  → zero occurrences
```

So no finding has ever been filed as a `TASKS.md` line, and the real destination
is a debt registry the flow does not mention. I read the enumeration as
**illustrative** — the binding clause is *«the finding records where it was
filed»*, and a debt registry is plainly «tracked work» — so a consumer using one
is complying, not diverging. Widening the list would be changing something the
evidence does not falsify. The part that *is* falsified is the recording
requirement (3 of 9), and that is route (b). Flagged so the boss can overrule me
if the list was meant to be closed.

**New obligations noticed:** `AUDIT.md`'s header (`:12`) defines `filed` as
*«became tracked work — WAL / `TASKS.md` / a PROP»*, which is the host adopting
the flow's illustrative list verbatim and then using none of the three. That is a
host-side defect on a host file, outside these eighteen. Recorded, not fixed.

---

## F-164 — the run procedure's five obligations, and a host that has never completed one

**Outcome:** OUT-OF-ROUTE (5 of 5 anchors)
**Files touched:** «none»

**Re-verification.** Two of the five verdicts make sharp, checkable claims; both
reproduce exactly.

```
$ for c in $(git log --format=%h -- AUDIT.md); do
    echo "$c files=$(git show --numstat --format= "$c" | wc -l) $(git log -1 --format=%s $c)"; done
3656f362  files=1  docs(audit): AUD-0016 dispositioned fixed - the posture is live
f11ed38a  files=1  docs(audit): same-day dispositions - the depth program closed its P2s
21d47694  files=1  docs(audit): 2026-06-12 run - the discipline-depth inventory
9283132f  files=5  docs(terraform): Phase 6 close-out — reconciliation, audit, REPORT
b98227ce  files=1  docs(audit): 2026-05-23 seed inventory
```

Four of five are one-file `docs(audit)` commits; the 2026-06-10 run's section
rides inside a five-file `docs(terraform)` commit, so it is neither its own
commit nor under an audit subject —
`STEP-COMMIT-THE-SECTION-AND-EACH-FIX-SEPARATELY` broken in exactly one run of
three, as the verdict said.

```
$ grep -cE '^### [0-9]{4}-[0-9]{2}-[0-9]{2}-[0-9]+ ·|^### AUD-[0-9]+ ·' AUDIT.md   → 25
$ grep -cE '^\| (AUD-[0-9]+|[0-9]{4}-[0-9]{2}-[0-9]{2}-[0-9]+) \|' AUDIT.md        → 4
$ grep -n '^| id | cat | sev' AUDIT.md                                             → 182
```

25 findings as per-finding `###` headings against 4 as table rows, and the single
prescribed table is the 2026-06-10 run's. Exact match to the verdict.

The inventory-completion verdicts rest on the same two self-disclaimers quoted
under F-141 (`AUDIT.md:23-24`, `:158`), re-read there.

**What changed and why — nothing. Five anchors, five prescriptions.**

- `A-RUN-MUST-FINISH-THE-INVENTORY` — *«A run must finish the inventory.»*
- `STEP-WALK-THE-CHECKLIST-BREADTH-FIRST` — *«touch every category once before
  going deep on any one.»*
- `STEP-COMMIT-THE-SECTION-AND-EACH-FIX-SEPARATELY` — *«The audit section and the
  fixes are different ideas; they are different commits.»*
- `EVERY-FINDING-IS-ONE-TABLE-ROW-CARRYING-THE-FIVE-FIELDS` — the record's form.
- `OWES-A-COMPLETE-INVENTORY` — the four sub-obligations of a run.

Every verdict against them is *«this host's three runs did not do it»*. Demoting
`A-RUN-MUST-FINISH-THE-INVENTORY` to something two disclaiming runs satisfy would
delete the flow's central distinction — the audit owes breadth where the gate
owes depth — and demoting `STEP-WALK-THE-CHECKLIST-BREADTH-FIRST` would delete
the step the whole document is built around
(`SUM-THE-SEVEN-STEPS`, running-an-audit.xml:149-151). Route (b) throughout.

**The one worth the boss's second look is
`EVERY-FINDING-IS-ONE-TABLE-ROW-CARRYING-THE-FIVE-FIELDS`,** because the
verdict itself concedes the fields survive in both forms and that *«nothing in
PROP-013 prescribes either shape»*. It still does not move: the table form is not
decoration, it is what
`A-READER-CAN-DIFF-TWO-RUNS-AND-SEE-THE-TREND` (HEALTH-AUDIT-PROTOCOL.xml:89-92)
depends on — 25 multi-paragraph headings do not diff into a trend the way 25 rows
do. If the boss judges the form incidental, this is the one anchor in F-164 that
could take a route-(a) reading; I did not take it, because relaxing a form that
another fact in the same package leans on is a design decision, not a prose
repair.

**New obligations noticed:** none outside these eighteen. Note for the host
obligation: `INT-0001`, the full-sweep window both disclaiming runs name, has
never closed — it is the single host action that would clear four of these five
verdicts at once.

---

## F-235 — «every run applies both», said of two rules that are conditional in their own text

**Outcome:** EDITED (1 of 2 anchors) · OUT-OF-ROUTE (1 of 2 anchors)
**Files touched:**
`packages/org.vibevm.world/health-audit/v0.1.0/spec/flows/health-audit/audit-checklist.xml`

**Re-verification.** Which of the three runs applied which growth rule:

```
$ grep -n 'HIST-CATEGORY-E' spec/common/PROP-013-periodic-health-audit.xml
77: **2026-06-12 — category E (discipline depth) added** by that day's
    owner-requested full sweep — the first post-adoption depth audit.

$ grep -nE 'xtask conform' AUDIT.md
177: **Conform panel** (`cargo xtask conform check`): 6 findings
425: … the new `cargo xtask conform freeze`: `error-message-cites-req` …

$ sed -n '20,30p' AUDIT.md      (the 2026-05-23 seed)
… it is **not** a fresh full sweep … Findings came from the M1.19 work itself
and from the WAL's standing Known-issues list.
```

Rule 1 (a new defect class becomes a permanent row) fired **once**, on
2026-06-12. Rule 2 (a mechanisable row migrates into the gate) fired in the same
run's fixes. The 2026-05-23 seed added no category and migrated nothing; the
2026-06-10 run, scoped to category C plus the gate panel, did neither. **One run
of three applied both; two applied neither.** The verdict reproduces.

**What changed and why.** `TWO-RULES-GOVERN-THE-LISTS-GROWTH` said *«Two rules
govern its growth, and every run applies both»*. The two rules it introduces are
**conditional in their own text** — `GROWTH-RULE-A-NEW-DEFECT-CLASS-BECOMES-A-PERMANENT-ROW`
begins *«When a run finds a kind of rot no category named…»* and
`GROWTH-RULE-A-MECHANISABLE-ROW-MIGRATES-INTO-THE-GATE` begins *«When a category
can be fully checked by a script…»*. Read as a rule, «every run applies both»
would oblige a run that found no new defect class to invent a category, which
neither rule asks for; read as a description, three runs falsify it. Changed to
*«each applying when its own condition holds»* — the frequency claim goes, both
rules stay exactly as strong as they were. This is the identical repair, and the
identical reasoning, as F-141's `TWO-FORCES-RESHAPE-THE-CHECKLIST-EVERY-RUN`,
and the verdict itself asked for them to be ruled identically; the two now use
matching wording so they cannot drift apart again.

**Why `SUM-FOUR-GROUPS-WALKED-BREADTH-FIRST` did not move — route (b).** *«Walk
all four breadth-first»* is the checklist's central instruction, not an
observation, and the verdict against it is that no run's text records touching A,
B, C and D once each before going deep. That is the same host non-compliance
ruled at four other anchors in this package (F-141's
`THE-AUDIT-IS-A-DIFFERENT-KIND-OF-CHECK` and
`AN-AUDIT-RUN-WALKS-ONE-CATEGORY-GROUP-PER-BLIND-SPOT`, F-164's
`STEP-WALK-THE-CHECKLIST-BREADTH-FIRST`, F-312's
`WALK-THE-CHECKLIST-BREADTH-FIRST-AND-RUN-EACH-AID`). The half of the fact that
*is* about the host — the four groups themselves — is confirmed: `AUDIT.md:14-15`
adopts A/B/C/D/E verbatim.

**Anchor-id note.** `TWO-RULES-GOVERN-THE-LISTS-GROWTH` names no frequency, so
unlike F-141's `…-EVERY-RUN` this id still reads true against its repaired text.

**New obligations noticed:** none.

---

## F-310 — a P1 that outlived two milestones, and two host records that disagree about its severity

**Outcome:** OUT-OF-ROUTE (1 of 1 anchor)
**Files touched:** «none»

**Re-verification.** Every leg reproduces, and the commit count has grown:

```
$ sed -n '9p' AUDIT.md
**Severity** — `P1` blocker (resolve before the next milestone ships) ·
   → the flow's definition, adopted verbatim in the host's own header.

$ sed -n '32p' AUDIT.md
### 2026-05-23-01 · A1 · P1 · filed

$ sed -n '365,369p' AUDIT.md
- **2026-05-23-01** (A1, git-registry path under-tested) — **reduced**: … the
  `vibe init` default-path e2e remains unverified this run. Re-judged P2 → P3, open.

$ grep -n 'DBT-0001' discipline/DEBT.md
29:- **DBT-0001** `coverage-gap` [P1] — Production git-registry + naming path is
   under-tested _(filed as INT-0002 (the test-hardening work))_

$ grep -noE '(SHIPPED|COMPLETE)[^)]*\(20[0-9]{2}-[0-9]{2}-[0-9]{2}\)' ROADMAP.md | tail -3
661:SHIPPED (2026-07-07)     ← M1.26
707:SHIPPED (2026-07-07)     ← M1.24
938:SHIPPED (2026-05-22)

$ git log --oneline --since=2026-06-12 | wc -l   → 1632   (verdict recorded 1 546)
```

So: the finding was raised P1 on 2026-05-23, never fixed, quietly re-judged down
to P3 in `AUDIT.md`, and **still carries `[P1]` in `discipline/DEBT.md:29`
today**, while two milestones shipped on 2026-07-07. Two durable host records
carry different severities for the same finding right now.

**What changed and why — nothing.** `SEVERITY-P1-BLOCKER` reads *«Must be
resolved before the next milestone is declared shipped»*. That is a
prescription, and both halves of the verdict are host facts: the host shipped two
milestones over an unresolved P1, and the host's two registries disagree about
what severity it carries. Rewriting the definition — «P1: resolve before the next
milestone, or re-judge it down» — would convert the flow's hardest severity into
a formality and legalise exactly the manoeuvre that happened here. Route (b): the
package does not move.

**The verdict's own framing supports this.** It types the row `contradiction`
and locates the contradiction *«the record disagrees with itself about the
finding that proves it»* — `AUDIT.md` versus `discipline/DEBT.md`, both host
files. Neither is in my edit perimeter and neither should be repaired by moving
the package.

**New obligations noticed:** the `AUDIT.md` ↔ `discipline/DEBT.md` severity
split on DBT-0001 / 2026-05-23-01 is a live host `contradiction` between two
durable registries, and it is not itself one of these eighteen (F-310's anchor is
the flow's definition, not the host records). It needs a host obligation of its
own. Recorded, not fixed.

---

## F-311 — «never declared done on an un-audited base», broken twice on the artefact the flow specifies

**Outcome:** OUT-OF-ROUTE (1 of 1 anchor)
**Files touched:** «none»

**Re-verification.** The verdict's own commands, re-run:

```
$ git log -1 --date=short --format='%h %ad %s' -- AUDIT.md
3656f362 2026-06-12 docs(audit): AUD-0016 dispositioned fixed - the posture is live

$ grep -n '^## ' AUDIT.md          → three run sections, latest 2026-06-12 (line 191)
$ grep -noE '(SHIPPED|COMPLETE)[^)]*\(20[0-9]{2}-[0-9]{2}-[0-9]{2}\)' ROADMAP.md | tail
  661:SHIPPED (2026-07-07)   ← M1.26 MCP sovereignty
  707:SHIPPED (2026-07-07)   ← M1.24 the agentic tcg line
$ git log --oneline --since=2026-06-12 | wc -l   → 1632
```

Two milestones declared SHIPPED on 2026-07-07, the last audit run dated
2026-06-12, no audit section between them, and 1 632 commits in the gap.
Confirmed, unchanged in substance from the verdict.

**What changed and why — nothing.** `A-MILESTONE-IS-NEVER-DECLARED-DONE-ON-AN-UN-AUDITED-BASE`
is the package's headline `never`, stated in the README (`:17-18`), in the
protocol's `AUDIT-IS-OWNER-TRIGGERED-WITH-A-ONCE-PER-MILESTONE-FLOOR`, in its
summary, and adopted by the host itself at
`spec/common/PROP-013-periodic-health-audit.xml:63` (`##CADENCE-FLOOR`, *«a vibevm
milestone is never declared done on an un-audited base»*, `@spec/done`). **The
host wrote the rule down, kept it in its own spec, and then shipped two
milestones against it.** That is the cleanest §3.6(b) case in the batch: the rule
is sound, the host adopted it in writing, and the compliance failure is entirely
host-side. Softening a `never` because the consumer broke it twice is the exact
inversion the mandate exists to prevent.

Note also that this is not a package/consumer disagreement at all — package and
host *agree* on the rule verbatim. There is nothing here for a prose edit to
repair.

**New obligations noticed:** a host obligation is owed — either run the audit
that M1.24/M1.26 shipped without, or record a §3.6(c) written exception on the
host side. Not mine to open.

---

## F-312 — the skill's four named aids: two run, one has no host equivalent, one has never been invoked

**Outcome:** OUT-OF-ROUTE (1 of 1 anchor)
**Files touched:** «none»

**Re-verification, aid by aid.** The fact
(`WALK-THE-CHECKLIST-BREADTH-FIRST-AND-RUN-EACH-AID`, SKILL.md:28-33) names four
mechanical aids. Each checked separately:

1. **`grep` for skip markers / `TODO` / suppressions — RUN.**
   ```
   $ grep -n '2026-06-12-11' AUDIT.md → 346
   ### 2026-06-12-11 · D · P3 · open
   Hygiene census for the record: `#[ignore]` 5 …; `#[allow]` 28 src-side …;
   TODO-family ≈ 17 raw …
   ```
2. **CI-config-versus-tree diff — EXISTS, but not as the verdict describes it.**
   ```
   $ find . -name 'self-check.sh' -not -path './target/*' -not -path './.opencode/*'
   ./tools/self-check.sh                     (not discipline/, as the verdict implied)
   $ grep -n '0b' tools/self-check.sh
   38:  # Before any of it, step 0b asserts the floor's own DENOMINATOR: every LIVE …
   148: # 0b. The floor's denominator. …
   $ ls -la .github/workflows/
   ls: cannot access '.github/workflows/': No such file or directory
   ```
   Step 0b is a **declared-workspaces-versus-live-tree** guard, which is the
   aid's spirit, but the host has **no CI config at all** — `tools/self-check.sh`
   *is* the gate. Recorded as the real state; it strengthens the aid's presence
   rather than weakening it.
3. **The coverage tool — NO HOST EQUIVALENT.**
4. **The dependency audit — NEVER INVOKED.**

**Perimeter searched** (both absences, and this is where §6.1's rule bites):
rooted at the repository root over the whole host tree, excluding only vendored
and regenerated copies (`vibedeps/`, `.vibe/`, `target/`, `refs/`,
`node_modules/`) and the packages under judgement (`packages/org.vibevm.*`);
globs `*.toml *.rs *.sh *.ps1 *.yml *.yaml *.md *.json`, so `Cargo.toml`,
`tools/`, `xtask/`, `discipline/`, `crates/`, `spec/`, `campaigns/`,
`legacy-spec/`, `research/` and every dotfile-config directory were all in scope:

```
$ grep -rniE 'llvm-cov|tarpaulin|grcov|cargo[- ]cov|coverage' …
  → only .opencode/node_modules/** (JS dependency manifests), agent SKILL prose,
    and prose uses of the word ("coverage gap", "coverage matrix"). No coverage
    tool is configured, installed or invoked anywhere in the host.

$ grep -rniE 'cargo audit|cargo-audit|cargo outdated|cargo-outdated|cargo deny|cargo-deny|RUSTSEC|advisory' …
  → zero tool hits. Every "advisory" match is an unrelated word use
    (`vvm doctor` advisories, conform cards "advisory until a checker exists",
    AGENTS.md on fractality acceptance). No dependency-audit command exists.
```

Both absences confirmed against the host tree, not against the package.

**What changed and why — nothing.** The fact is an **instruction to the agent
running the skill** — *«Walk the checklist breadth-first … Run each mechanical
aid»* — and both failing legs are host facts: no host run walked breadth-first
(the same measurement ruled at four other anchors in this package), and two of
the four aids have no host implementation. Crucially, **the flow already handles
a consumer that lacks an aid**: `audit-checklist.xml`'s
`TRANSLATE-EVERY-AID-INTO-YOUR-STACKS-EQUIVALENT` (:18-23) says *«"the coverage
tool" is whatever your language ships»*, and `D4-AID` names the generic
`cargo outdated` / `cargo audit` family. A consumer that has installed neither is
a consumer with a gap, not a flow that overpromises. Route (b).

**New obligations noticed:** two, both host-side and both outside these eighteen.
(a) The host has **no coverage tool and no dependency-audit command**, while
`PROP-013` §2 carries `A1` (coverage gaps) and `D4` (dependency staleness) as
standing checklist rows — two adopted rows with no means of being checked here,
which is why nothing has ever been filed under D4. (b) `health-audit`'s own
`README.md:81-83`, `NO-AMOUNT-OF-GATE-CATCHES-A-TEST-THAT-GUARDS-A-BUG` —
*«only a periodic judgment sweep does»* — carries the **identical overstatement**
I repaired under F-141 in `HEALTH-AUDIT-PROTOCOL.xml`, and is falsified by the same
evidence (`PROP-013 ##M119-CAUGHT-BY-SMOKE`: it was caught by a live smoke run).
It is in this package but in **no** obligation of mine, so per the brief it is
recorded and not touched. If the boss wants the pair consistent, that README line
needs the same repair.

---

## F-149 — the tier's definition and directory convention, against a host that keeps three homes and an incomplete index

**Outcome:** OUT-OF-ROUTE (7 of 7 anchors)
**Files touched:** «none»

**Re-verification.** Every number reproduces, several of them exactly:

```
$ ls manual-tests/
M1.1-git-registry-smoke.md   M1.5-gate-multi-package-smoke.md   M1.6-mirror-vendor-smoke.md
M1.15-git-source-smoke.md    M1.5-gate-v2-per-package-smoke.md  M2.10-index-smoke.md
M1.16-redirect-smoke.md      M1.17-workspace-publish-smoke.md   README.md
   → 8 test files.

$ sed -n '77,92p' manual-tests/README.md      → 7 data rows (+ header + separator)
$ grep -n 'M2.10' manual-tests/README.md      → NO MATCH
$ sed -n '89p' manual-tests/README.md
Add a row to this table when you add a test. Keep the table sorted by milestone.
   → the README instructs the rule it is itself breaking. Confirmed.

$ wc -l manual-tests/*.md | sort -rn | head -3
  348 manual-tests/M1.5-gate-v2-per-package-smoke.md      ← over the host's own ~300
  296 manual-tests/M1.1-git-registry-smoke.md
  288 manual-tests/M1.16-redirect-smoke.md
$ sed -n '74p' manual-tests/README.md
Keep each file under ~300 lines. If a test is larger than that, it is …

$ grep -n '^## Scenario' manual-tests/*.md
M1.6-mirror-vendor-smoke.md:59   ## Scenario A — vendor → file:// mirror → offline install
M1.6-mirror-vendor-smoke.md:171  ## Scenario B — multi-`[[registry]]` priority walk
M2.10-index-smoke.md:11          ## Scenario A — bootstrap + serve + consume
M2.10-index-smoke.md:106         ## Scenario B — vibe-registry consumer fast path
M2.10-index-smoke.md:122         ## Scenario C — vibe-publish post-publish hook
```

The ambient-state leg (MT-02 / MT-03 writing `~/.vibe/` with no Teardown) is
re-verified under F-317 below; the `Expected` leg under F-318.

**Perimeter searched** — for «the host has three homes», which is the one
finding here that rests on an enumeration:

```
$ find . -type d -name 'manual-tests' -not -path './target/*' -not -path './.opencode/*'
```

19 directories, of which **three are authored homes** —
`./manual-tests`, `./spec/manual-tests`, and
`./packages/org.vibevm.fractality/fractality/v0.1.0/spec/manual-tests`. The other
sixteen are the package's own flow directory, its install slots under
`vibedeps/**`, and regenerated copies under `.vibe/cache/**` — not homes. The
verdict's count is right.

**One correction to the verdict's framing, recorded per the brief's third
outcome.** The third home belongs to the **fractality specspace**, which
`CLAUDE.md:141-144` establishes as a sub-project with *«their own boot contract,
WAL, and `CONTINUE.md`, worked on as independent projects»*. A separate project
keeping its own `manual-tests/` is that project applying the convention, not this
project violating it. So the honest count of homes **in this project** is
**two** — `manual-tests/` and `spec/manual-tests/` — and only the second is the
divergence. That halves the finding without changing its direction or its route.

**What changed and why — nothing. Seven anchors, seven prescriptions.** This
package is a *convention*, and every verdict against it says the host keeps the
convention partially:

| anchor | the rule | what the host does |
|---|---|---|
| `A-MANUAL-TEST-IS-A-SELF-CONTAINED-MARKDOWN-WALKTHROUGH` | «no ambient state left behind» | MT-02 / MT-03 persist `~/.vibe/` |
| `OPEN-IT-FOLLOW-IT-AND-KNOW-PRECISELY-WHICH-STEP-DIVERGED` | a divergence localises to a step | 3 tests have no per-step Expected |
| `MANUAL-TESTS-LIVE-IN-A-DEDICATED-DIRECTORY-AT-THE-REPOSITORY-ROOT` | one directory, at the root | a second home at `spec/manual-tests/` |
| `AN-INDEX-README-IN-THE-DIRECTORY` | «New test, new row» | 7 rows against 8 files |
| `KEEP-EACH-FILE-TO-ONE-SCENARIO` | split a grown walkthrough | M1.6 has 2, M2.10 has 3 |
| `SUM-WHAT-A-MANUAL-TEST-IS` | summary of row 1 | same |
| `SUM-ONE-FILE-PER-SCENARIO-INDEXED-BY-A-README` | summary of rows 4-5 | same |

Not one of these is a claim about the world that turned out false; each is a rule
the consumer keeps loosely. And the host **adopted every one of them in writing**
— `spec/common/PROP-000.xml:187` (`##MT-LOCATION`, the root directory and the
index), `:188` (`##MT-FLOW-POINTER`, citing this flow by `spec://` URI as the
tier's definition), and `manual-tests/README.md:74` sharpening
`KEEP-EACH-FILE-TO-ONE-SCENARIO` into a numeric ~300-line limit it then exceeds
by 48 lines. A package cannot yield to a consumer that wrote the same rule down
and drifted from it; that is §3.6(b) with the host's signature on it.

**New obligations noticed:** two host-side, both outside these eighteen.
(a) `manual-tests/README.md` is missing the `M2.10-index-smoke.md` row while
line 89 of the same file instructs *«Add a row to this table when you add a
test»* — a one-row host fix. (b) `spec/common/PROP-000.xml` `##MT-LOCATION`
records only the root home, so the eleven tests' second home at
`spec/manual-tests/` sits outside the decision that governs the tier. Both
recorded, neither fixed.

---

## F-317 — the flow's hardest `never`, broken by two tests that say so in a marked fact

**Outcome:** OUT-OF-ROUTE (1 of 1 anchor)
**Files touched:** «none»

**Re-verification.** I read both notes and both files' headings directly:

```
$ grep -n 'settings-mutation-note' spec/manual-tests/MT-0*.md
MT-02-vibe-tree-tui.xml:14  ##settings-mutation-note Unlike MT-01, this TUI **writes
  user settings** (`~/.vibe/` via the vibe-settings system, Шаг 2) …
MT-03-vibe-prefs-tui.xml:12  ##settings-mutation-note This TUI writes user settings
  (`~/.vibe/`, the vibe-settings system) — palette/ …

$ grep -nE '^#{2,3} ' spec/manual-tests/MT-02-vibe-tree-tui.xml spec/manual-tests/MT-03-vibe-prefs-tui.xml
MT-02: 19 ## Preconditions · 27 ## Steps · 82 ## Pass · 88 ## Sign-off
MT-03: 15 ## Preconditions · 21 ## Steps · 65 ## Pass · 72 ## Sign-off
   → neither has a Setup or a Teardown section.

$ grep -lE 'VIBE_REGISTRY_CACHE|VIBE_HOME|SCRATCH' manual-tests/M*.md | wc -l   → 7 of 8
```

My redirect count is **7 of 8** where the verdict said 6 of 8, because my pattern
also matches `SCRATCH`-only isolation; I record my number and my pattern rather
than adopting the verdict's. Either way the point stands and is the important
one: **the rule is demonstrably achievable in this very repository** — seven of
the eight root-home tests isolate, and `manual-tests/M1.1-git-registry-smoke.md:273`
asserts the real directory stayed absent (`ls ~/.vibe/registries … || echo "no
~/.vibe/registries — good"`).

**The host contradicts itself, and the contradiction is host-side.**

```
$ grep -n 'MT-ISOLATION' spec/common/PROP-000.xml
193:- ##MT-ISOLATION **vibevm's bindings.** Every test isolates state with `mktemp -d`
    … the user's real `~/.vibe/` is never touched by a run. @spec/done
```

So `PROP-000` — the host's own spec, marked `@spec/done` — asserts the rule
holds universally here, while two of the host's eleven tests declare in their own
marked `@impl/done` facts that they break it. That is one host document against
two host artefacts. **Nothing in it falsifies the package.**

**What changed and why — nothing.**
`NEVER-LET-A-MANUAL-TEST-TOUCH-REAL-USER-STATE` is the flow's hardest `never`,
and `authoring-rules.xml` Rule 1 carries its reason:
`A-TEST-THAT-MUTATES-REAL-USER-STATE-IS-A-BUG-IN-THE-TEST` — *«even if every step
passes — because the next contributor's run inherits that mutation and the
walkthrough is no longer reproducible»* — with the remedy stated one fact later:
`A-STEP-THAT-SEEMS-TO-NEED-REAL-STATE-MEANS-FIX-THE-REDIRECT`. Adding «unless the
test documents the mutation» would convert a `never` into a disclosure
requirement and legalise precisely what MT-02 and MT-03 do. Route (b) at its
sharpest: seven sibling tests prove the rule is keepable, so the two that do not
keep it are the defect.

**This is the batch's strongest §3.6(c) candidate, and I did not take it.** The
two tests *do* carry a written, marked exception — which is the shape (c)
describes. But (c) requires the exception to be *recorded on the host side as an
exception to the rule*, and here the host's spec says the opposite (`MT-ISOLATION`
claims universal isolation). A note inside the deviating artefact, contradicted by
the governing spec, is not a recorded choice — it is the unmarked gap (c) exists
to convert. And the plan reserves (c) for the owner *«wherever the exception is a
policy choice rather than a note»*, which «may a TUI test write real user prefs»
plainly is. Flagged for the owner rather than decided here.

**New obligations noticed:** a host `contradiction` between
`spec/common/PROP-000.xml` `##MT-ISOLATION` and
`spec/manual-tests/MT-02`/`MT-03`'s `##settings-mutation-note` — two host records
asserting opposite things about the same eleven tests. Not one of these eighteen.
Recorded, not fixed.

---

## F-318 — «never write a step without an Expected paragraph», scored per file

**Outcome:** OUT-OF-ROUTE (1 of 1 anchor)
**Files touched:** «none»

**Re-verification.** The per-file counts, which are the whole verdict:

```
$ for f in manual-tests/*.md spec/manual-tests/*.md; do echo "$f $(grep -c 'Expected' "$f")"; done
manual-tests/M1.1-git-registry-smoke.md          10
manual-tests/M1.5-gate-multi-package-smoke.md    10
manual-tests/M1.5-gate-v2-per-package-smoke.md    9
manual-tests/M1.6-mirror-vendor-smoke.md          8
manual-tests/M1.16-redirect-smoke.md              8
manual-tests/M1.17-workspace-publish-smoke.md     7
manual-tests/M1.15-git-source-smoke.md            5
manual-tests/M2.10-index-smoke.md                 0
spec/manual-tests/MT-01-vibe-tree.xml             10
spec/manual-tests/MT-02-vibe-tree-tui.xml          0
spec/manual-tests/MT-03-vibe-prefs-tui.xml         0
```

**Every one of the eleven counts matches the verdict's list exactly.** Three of
eleven tests score zero, and all three substitute an aggregate section —
`MT-02:82 ## Pass`, `MT-03:65 ## Pass`, and `M2.10:38 Pass when:` lines.

**What changed and why — nothing.**
`NEVER-WRITE-A-STEP-WITHOUT-AN-EXPECTED-PARAGRAPH` is a `Never` carrying its own
reason inline — *«A command with no stated outcome cannot pass or fail; it is not
a test step»* — restated in `authoring-rules.xml` at
`A-COMMAND-WITH-NO-EXPECTED-IS-NOT-A-TEST-STEP` and
`if-you-cannot-articulate-the-outcome-you-do-not-know-what-it-proves`, and adopted
by the host at `manual-tests/README.md:62` (*«and an "Expected" subsection
describing the observable outcome»*). Eight of eleven tests keep it, so it is
keepable. Relaxing it to permit an aggregate `## Pass` would delete the property
F-149's `OPEN-IT-FOLLOW-IT-AND-KNOW-PRECISELY-WHICH-STEP-DIVERGED` promises —
that a divergence localises to a step — which is the reason the tier is
step-structured at all. Route (b).

**New obligations noticed:** none beyond the host obligation already implied —
MT-02, MT-03 and M2.10 need per-step Expected paragraphs, or a recorded exception.

---

## F-319 — «a walkthrough that breaks any of these is a bug in the test», and nothing is filed as one

**Outcome:** OUT-OF-ROUTE (1 of 1 anchor)
**Files touched:** «none»

**Re-verification.** Which tests break which rules is settled by F-317 (Rule 1),
F-318 (Rule 2) and F-149 (Rule 4 — MT-02 and MT-03 have no Teardown and no
what-to-file list; their headings are Preconditions / Steps / Pass / Sign-off).
The remaining claim is an absence, so it gets a perimeter.

**Perimeter searched.** For «nothing anywhere classifies these as bugs in the
test», rooted at the repository root over every durable host record that could
carry a defect — the inventories, the trackers, the discipline registries, the
specs, the campaign documents and the legacy tree:

```
$ grep -rniE 'manual[- ]test|MT-0[123]|M2\.10' \
    AUDIT.md BACKLOG.md TASKS.md ROADMAP.md CLAUDE.md \
    discipline/ spec/common/ spec/modules/ spec/design/ \
    campaigns/packages-2026-09/*.md legacy-spec/
$ grep -niE 'manual|MT-0' discipline/DEBT.md discipline/registry/*.json
```

Every hit is one of four things, and **none is a walkthrough filed as defective**:

1. **`fixtures/manual-test-packages/` schema rot** — `AUDIT.md:78`, `:372`,
   `discipline/DEBT.md:10` (DBT-0003), `discipline/registry/debt.json:49-59`,
   `discipline/registry/DEBT.md:42`. This is about *fixture packages the tests
   consume*, not about any walkthrough's authoring. Exactly the «unrelated
   fixture finding» the verdict named.
2. **Tests being written or run** — `TASKS.md:57`, `ROADMAP.md:119, 275, 358`.
3. **The convention being specified** — `spec/common/PROP-000.xml:187-202`.
4. **A pointer in `CLAUDE.md:137`** to fractality's own MT-05.

And one finding the wider perimeter turned up that sharpens the verdict:
`spec/common/PROP-000.xml:202` `##MT-WAL-NAMES` records that *«MT-02 and MT-03
have been awaiting owner sign-off since the TUI work landed»* — so the host tracks
these two tests as **pending**, never as **defective**. The absence is confirmed
on a much wider perimeter than the verdict used, and its cause is now named.

**What changed and why — nothing.**
`A-WALKTHROUGH-THAT-BREAKS-ANY-OF-THESE-IS-A-BUG-IN-THE-TEST` is the
classification rule the whole authoring-rules document rests on — it is what
makes the four rules binding rather than advisory. The verdict is that the host
has three such walkthroughs and files none as a defect. Adding an escape («…
unless the walkthrough documents why») would hand MT-02 and MT-03 their own
justification as a legal exemption, and would do it in the shipped flow, for
every consumer. Route (b): the rule stands; the host owes either three defect
records or a written exception.

**New obligations noticed:** the `fixtures/manual-test-packages/` rot (DBT-0003)
has been `open` since the 2026-05-23 seed and is carried in three separate host
registries — `discipline/DEBT.md:10`, `discipline/registry/debt.json:49` and
`discipline/registry/DEBT.md:42`. Three writers for one debt row is a
`duplication` in the host's own discipline tree, outside these eighteen.
Recorded, not fixed.

---

## F-320 — a summary that lists four section functions and delivers three

**Outcome:** OUT-OF-ROUTE (1 of 1 anchor)
**Files touched:** «none»

**Re-verification.** The fact, in full (`test-template.xml:181-182`):

> `##SUM-WHAT-EACH-SECTION-DOES` Purpose justifies the tier; Preconditions gate
> the run; Setup isolates it; every Step carries an Expected.

Three of the four hold on the host, as the verdict says. The fourth is the
`Expected` measurement, re-run in full under F-318 above: **0 in MT-02 (10
steps), 0 in MT-03 (8 steps), 0 in M2.10**, against 5–10 in the other eight.

**What changed and why — nothing.** This is a summary restatement, and it carries
its clause's verdict — the same shape ruled at F-137's `SUM-SIZE-AND-CONSTRAINTS`,
F-165's two `SUM-*` anchors, F-141's `SUM-OWNER-TRIGGERED-FLOOR-ONCE-PER-MILESTONE`
and F-149's two. Its fourth clause is `NEVER-WRITE-A-STEP-WITHOUT-AN-EXPECTED-PARAGRAPH`
compressed into five words, so it moves if and only if that `Never` moves, and
F-318 rules that it does not. Dropping «every Step carries an Expected» from the
summary would leave the template describing a document shape its own
`authoring-rules.xml` Rule 2 forbids.

**A note on scope, since the fact is a summary of the template's sections.** The
template's own `SUM-FIXED-SECTION-ORDER` (`:179-180`) prescribes *«Title, Purpose,
Preconditions, Setup, Steps, Teardown, What to file if it fails»*, and MT-02 /
MT-03 use Preconditions / Steps / Pass / Sign-off — so the host diverges on the
**section order** too, not only on the Expected clause. That anchor is not in any
of my eighteen and I did not touch it, but the boss should know the same three
tests falsify it, so a host obligation opened for F-320 will want to cover both.

**New obligations noticed:** `test-template.xml#SUM-FIXED-SECTION-ORDER` is
falsified by the same three tests and carries no obligation in the registry.
Recorded, not fixed.

---

# Close-out {#close}

## The eighteen, at a glance {#table}

| id | package | outcome | anchors moved |
|---|---|---|---:|
| F-137 | `wal` | EDITED · OUT-OF-ROUTE | 1 of 11 |
| F-165 | `wal` | OUT-OF-ROUTE | 0 of 5 |
| F-205 | `wal` | EDITED · OUT-OF-ROUTE | 2 of 3 |
| F-256 | `wal` | OUT-OF-ROUTE | 0 of 2 |
| F-257 | `wal` | OUT-OF-ROUTE | 0 of 2 |
| F-349 | `wal` | OUT-OF-ROUTE | 0 of 1 |
| F-352 | `wal` | OUT-OF-ROUTE | 0 of 1 |
| F-141 | `health-audit` | EDITED · OUT-OF-ROUTE | 2 of 9 |
| F-164 | `health-audit` | OUT-OF-ROUTE | 0 of 5 |
| F-235 | `health-audit` | EDITED · OUT-OF-ROUTE | 1 of 2 |
| F-310 | `health-audit` | OUT-OF-ROUTE | 0 of 1 |
| F-311 | `health-audit` | OUT-OF-ROUTE | 0 of 1 |
| F-312 | `health-audit` | OUT-OF-ROUTE | 0 of 1 |
| F-149 | `manual-tests` | OUT-OF-ROUTE | 0 of 7 |
| F-317 | `manual-tests` | OUT-OF-ROUTE | 0 of 1 |
| F-318 | `manual-tests` | OUT-OF-ROUTE | 0 of 1 |
| F-319 | `manual-tests` | OUT-OF-ROUTE | 0 of 1 |
| F-320 | `manual-tests` | OUT-OF-ROUTE | 0 of 1 |
| **total** | | **4 edited · 14 out-of-route** | **6 of 55** |

**Six anchors of 55 moved, and every one of them was a statement of fact that
turned out false** — a sibling package's shipped content, a chapter number, the
package's own two-file model, a detection mechanism the host's own record names,
and two frequency claims the packages' own conditionals never made. **The other
49 are prescriptions this host does not keep**, which §3.6 routes to (b): the
package does not move.

## What I edited {#diff}

Four files, all inside the three assigned package directories:

```
packages/org.vibevm.world/wal/v0.2.0/README.md                                   +4 -3
packages/org.vibevm.world/wal/v0.2.0/spec/flows/wal/WAL-PROTOCOL.xml              +2 -2
packages/org.vibevm.world/health-audit/v0.1.0/spec/flows/health-audit/HEALTH-AUDIT-PROTOCOL.xml  +5 -3
packages/org.vibevm.world/health-audit/v0.1.0/spec/flows/health-audit/audit-checklist.xml        +1 -1
```

No file under `manual-tests/` was touched. Verified before hand-off:

```
$ for f in <the four>; do compare `git show HEAD:$f` anchor set to working tree; done
IDENTICAL anchor set (27 ids)  wal/v0.2.0/README.md
IDENTICAL anchor set (60 ids)  wal/v0.2.0/spec/flows/wal/WAL-PROTOCOL.md
IDENTICAL anchor set (71 ids)  health-audit/…/HEALTH-AUDIT-PROTOCOL.xml
IDENTICAL anchor set (65 ids)  health-audit/…/audit-checklist.xml

$ git diff -- <the two packages> | grep '^+' | grep -E '\]\(\.\./'
none — clean          (no relative cross-package link added)
```

**Not mine.** `git diff --stat` also shows four files under
`packages/org.vibevm.world/campaign-plans/` and
`…/comparative-research/` modified. Those were already in the working tree and
belong to a parallel worker; I neither made nor reverted them.

## Two anchor ids that now read against their own text {#id-drift}

Both are forced by `RULE-ANCHORS-IMMUTABLE` and the brief's «do not add, remove
or rename any `##ANCHOR` fact id», and both are flagged rather than acted on:

- `DISCIPLINE-DEFERS-TO-THIS-PACKAGE-FROM-ITS-NEXT-RELEASE` — the repaired
  sentence says the deference has **not** landed.
- `TWO-FORCES-RESHAPE-THE-CHECKLIST-EVERY-RUN` — the repaired sentence no longer
  claims «every run».

## What the boss has to decide {#decisions}

1. **F-349 is a real package defect I was not given the anchors to fix.** The
   `wal` package ships two rival wind-down orderings — `session-end-hook.xml`'s six
   steps (WAL before `CONTINUE.md`, three times over) against `cold-resume.xml`
   §wind-down's five (`CONTINUE.md` first), while
   `WIND-DOWN-IS-THE-EXPLICIT-FORM-OF-THE-HOOK` asserts they are the same
   procedure. The repair lands on `cold-resume.xml`'s `STEP-*` anchors, none of
   which is in these eighteen. This is the single highest-value item in the batch:
   it is route (a), it is `prose-edit`, and F-165's host obligation is unsound
   until it is settled.
2. **F-205's canonicity anchor needs a release event.** Making
   `PACKAGE-IS-THE-CANONICAL-HOME-OF-THE-WAL-CONVENTION` true means editing
   `core-ai-native` and re-vendoring — §4.5, owner before publication. The host
   currently boots **both** conventions (`vibe.lock:62`,
   `spec/boot/STATIC.xml:1369`).
3. **F-317 is the batch's §3.6(c) candidate.** MT-02 and MT-03 carry written,
   marked exceptions to the flow's hardest `never` — but
   `spec/common/PROP-000.xml` `##MT-ISOLATION` asserts the opposite, so the
   «exception» is contradicted by the governing spec. The plan reserves (c) for
   the owner where the exception is a policy choice; «may a TUI test write real
   user prefs» is one.
4. **F-352 turns on a reading, not a measurement.** Strictly, *«read the WAL
   before doing anything else»* is unsatisfiable for any compiled boot snippet;
   loosely — the reading `spec/boot/00-core.xml`'s own step 4 uses — the host
   complies and the anchor is a `RE-JUDGE: confirmed`.

## Host obligations this batch owes {#host}

Recorded, none opened by me: the WAL's format and budget rules (F-137, ten
anchors); the wind-down hook's implicit trigger, rewrite-not-patch and
`_Updated:` rules (F-165, F-257); the cold-start read order (F-256); the boot
order (F-352); the breadth-first sweep and the once-per-milestone floor (F-141,
F-164, F-235, F-310, F-311, F-312); and the manual tier's isolation, `Expected`,
index and one-scenario rules (F-149, F-317, F-318, F-319, F-320). Plus four
defects found outside the eighteen: the `AUDIT.md` ↔ `discipline/DEBT.md`
severity split on DBT-0001; the host booting two WAL conventions;
`health-audit/README.md:81-83` carrying the same overstatement repaired under
F-141; and `manual-tests/README.md` missing its `M2.10` row.
