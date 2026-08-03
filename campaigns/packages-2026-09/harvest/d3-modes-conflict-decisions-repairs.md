# D3 — operating-modes · conflict-protocol · decision-records, wave-3 repairs

_Twelve `prose-edit` obligations, prepared 2026-07-29 under
[`PHASE-D-BATCH-PLAN.md`](../PHASE-D-BATCH-PLAN.md) §1.2, §3.6 and §6.1.
Route check per obligation ran first:_

```
python campaigns/packages-2026-09/tasks/drift-registry.py --task F-NNN
```

_All twelve returned `route: prose-edit`, `release_event: false`. None was
OUT-OF-ROUTE._

---

## F-202 — «recorded as spoken» / «recorded verbatim» label an English rendering of Russian speech

**Outcome:** EDITED
**Anchors:** 2 of 3 edited — `##the-owners-framing-lead` and
`##THE-DESCRIPTION-IS-RECORDED-VERBATIM` edited;
`##THE-OWNERS-RECORDED-FRAMING` (the blockquote itself) left untouched.
**Files touched:**
`C:\Users\olegc\git\v\vibevm\packages\org.vibevm.world\operating-modes\v0.1.0\spec\flows\operating-modes\mfbt-mode.md`

**Re-verification:**

```
$ git log -1 --format='%s%n%b' a6df6050 | rg -n 'paraphrase'
… the owner's verbatim Russian authorisation (2026-05-06), which stays because the package's own copy
is an English paraphrase and a paraphrase of an authorisation is a different
authorisation.

$ git log -1 --format='%s%n%b' 4d5ccf83 | rg -n 'verbatim'
things» codeword with the owner's verbatim description (mfbt-mode), and how to

$ rg -uu -l --glob '!.git/**' "Целься сразу|Токены не экономь" .
.\campaigns\packages-2026-09\tasks\evidence\ev-W5c.json

$ f=packages/.../mfbt-mode.md; diff <(git show HEAD:$f | rg -o '##[A-Za-z][A-Za-z0-9-]*' | sort) \
      <(rg -o '##[A-Za-z][A-Za-z0-9-]*' $f | sort) && echo "ANCHOR SET IDENTICAL"
ANCHOR SET IDENTICAL

$ git diff --stat -- $f
 .../v0.1.0/spec/flows/operating-modes/mfbt-mode.md | 8 +++++---
 1 file changed, 5 insertions(+), 3 deletions(-)
```

**Who falsifies it:** the document itself — the two sentences make a claim
about the blockquote printed between them, in the same file, and the
extraction commit that produced that blockquote (`a6df6050`) calls it «an
English paraphrase» in its own words while the next sentence of this very
document says a paraphrase is a different authorisation.

**Perimeter searched:** for the spoken Russian original — `rg -uu` over the
whole working tree, ignored files included, everything except `.git/`
(so `spec/**`, `packages/**`, `vibedeps/**`, `refs/**`, `.vibe/cache/**`,
`legacy-spec/**`, `campaigns/**`, `crates/`, `xtask/`, `discipline/**` are all
inside the perimeter). Exactly one live hit, and it is a Phase C evidence
artefact quoting the loss, not a copy of the text:
`campaigns/packages-2026-09/tasks/evidence/ev-W5c.json`. The original itself
survives only in git history at `spec/common/PROP-006-operating-modes.md:24`
as of `a6df6050`, removed by `4d5ccf83` one day later.

**What changed and why:** the lead now says what the blockquote is — the
owner's framing of 2026-05-06, rendered in English rather than the Russian it
was spoken in — instead of «recorded as spoken», which the blockquote is not.
`##THE-DESCRIPTION-IS-RECORDED-VERBATIM` stops asserting that *this*
description is the verbatim record and states the norm it always carried,
strengthened rather than weakened: verbatim means the owner's own words **and
language**. The norm is untouched in force; what is removed is the false
claim that the artefact below satisfies it. The blockquote is left alone
deliberately: restoring the Russian would re-introduce owner-supplied content
into a published package, which is a publication decision, not a prose repair.

**New obligations noticed:**

1. **The MFBT authorisation exists in no live file.** The owner's Russian
   original was kept in `spec/common/PROP-006-operating-modes.md` by
   `a6df6050` for the stated reason that the package copy is a paraphrase, and
   deleted by `4d5ccf83` the next day on the opposite ground. Two commits one
   day apart cannot both be right, and the loss is real. Restoring it — to the
   host record or to this package — is an owner ruling. It also lost the four
   named depth switches the English renders as «Work at full reasoning depth».
2. **The package's flagship exemplar now visibly fails its own protocol part
   2** (`OPERATING-MODES-PROTOCOL.md#ROW-PART-AUTHORITATIVE-DESCRIPTION`), and
   `spec/common/PROP-006-operating-modes.md:19` still tells a host session that
   «the owner's verbatim description is recorded in the flow». That host
   sentence is now false on the package's own admission and needs a host-side
   repair (route (b)).

---

## F-177 — the rule is sound; its violation is the package's own exemplar, and one anchor is host-falsified

**Outcome:** RE-JUDGE: confirmed — 3 anchors; route (b) deferred — 1 anchor
**Anchors:** 0 of 4 edited —
`##CAPTURE-THE-OWNERS-OWN-FRAMING-IN-THEIR-WORDS-DATED`,
`##DO-NOT-TIDY-THE-DESCRIPTION-INTO-YOUR-OWN-PROSE`,
`##EACH-RULE-IS-TESTABLE-BY-THE-AGENT-ITSELF` re-judged confirmed;
`##AN-UNCATALOGUED-PHRASE-IS-A-PROPOSAL` is route (b), host obligation.
**Files touched:** none

**Re-verification:**

```
$ rg -n 'Each rule is testable' packages/org.vibevm.world/operating-modes/v0.1.0/spec/flows/operating-modes/writing-a-codeword.md
54:##EACH-RULE-IS-TESTABLE-BY-THE-AGENT-ITSELF Each rule is testable in
55:the sense that the agent can tell whether it is obeying it.

$ rg -n 'vague-rules' packages/org.vibevm.world/operating-modes/v0.1.0/spec/flows/operating-modes/writing-a-codeword.md
57:##vague-rules-are-useless-specific-ones-are-the-value Vague rules ("be thorough") are useless; specific ones ("each phase

$ rg -n 'ЗАВЕРШИ СЕССИЮ|ВОССТАНОВИ СЕССИЮ' CLAUDE.md
167:## Session-end checkpoint command — `ЗАВЕРШИ СЕССИЮ` / `END SESSION`
196:## Session-resume command — `ВОССТАНОВИ СЕССИЮ` / `RESUME SESSION`
```

**Who falsifies it:** for the first three anchors, nothing does — two are
imperatives (a rule is not made false by an instance failing it) and the third
was measured against a criterion it does not state. For the fourth, the host.

**Perimeter searched:** the codeword catalogue — `spec/boot/90-user.md`
(`##CODEWORD-MFBT`, `##operating-modes-intro`), `spec/common/PROP-006-operating-modes.md`,
`CLAUDE.md`. The two phrase families the verdict names sit in `CLAUDE.md`,
outside the catalogue the rule points at.

**What changed and why:** nothing changed, on purpose.
`CAPTURE-…-IN-THEIR-WORDS-DATED` and `DO-NOT-TIDY-…` are the rules
`mfbt-mode.md` breaks; the yield belongs at the exemplar, and it landed there
under F-202 (the lead now carries the date and names the rendering). Weakening
«in their words, dated» or «do not tidy it into your own prose» so the
exemplar passes is the reverted wave-2 shape and was not done.
`EACH-RULE-IS-TESTABLE-BY-THE-AGENT-ITSELF` defines testability in its own
clause — «in the sense that **the agent can tell whether it is obeying it**» —
and the verdict measured it against mechanical checkers in the host instead,
then noted that no host file states the meta-rule, which the fact never
requires. Its own sibling `##vague-rules-are-useless-…` fixes the sense as
vague-versus-specific, not machine-checkable; rules 1 and 7 of `mfbt-mode.md`
(«aim at the maximum version … if the planned scope is N slices, walk all N»,
«work at full reasoning depth») are both things an agent can tell it is
obeying. `AN-UNCATALOGUED-PHRASE-IS-A-PROPOSAL` is sound and the host acts on
two phrase families outside its own catalogue — route (b), the package does
not move.

**New obligations noticed:** the host's session-end and session-resume
commands in `CLAUDE.md` are recognised by intent from two-language trigger
lists and called «a hard contract, not a courtesy», but are not in the
codeword catalogue at `spec/boot/90-user.md` and carry no five-part spec. Host
obligation: catalogue them, or record the exception on the host side per
§3.6(c).

---

## F-201 — «every codeword … five parts» is host-falsified, not self-falsified

**Outcome:** RE-JUDGE: confirmed — route (b), host obligation, registry row to
`deferred` per §3.6(b)
**Anchors:** 0 of 3 edited —
`#every-codeword-is-specified-with-the-same-five-parts-lead`,
`##A-PHRASE-MISSING-A-PART-IS-A-PROPOSAL-NOT-A-CODEWORD`,
`##SUM-THE-FIVE-PART-SHAPE-OR-IT-IS-A-PROPOSAL`.
**Files touched:** none

**Re-verification:**

```
$ rg -n 'CODEWORD-MFBT|operating-modes-intro' spec/boot/90-user.md
64:##operating-modes-intro Trigger phrases that switch the session into an alternate working posture are catalogued in [PROP-006](../common/PROP-006-operating-modes.md). …
68:- ##CODEWORD-MFBT **«move fast and break things»** … Maximum scope, testable phases, no mid-work confirmations, full reasoning depth. …

$ git log -1 --format='%b' 251987b1 | rg -n 'codeword'
… the existing session-end codeword … same pattern, generalised …
```

**Who falsifies it:** the host. The five-part shape is stated correctly and
completely inside the package; what fails it is `CLAUDE.md`'s two
phrase-triggered contracts carrying two of the five parts.

**What changed and why:** nothing. This is the exact shape §6.1 and the wave-2
reverts warn about — broadening «every» so the host's two-part session
commands stop being counter-examples would rewrite a shipped normative rule to
describe a lax consumer. The host's own creating commit `251987b1` calls the
session-end contract a codeword, so the host has already treated it as one;
the compliance work is the host's, not the package's. Route (b).

**New obligations noticed:** same host obligation as F-177 — specify the two
`CLAUDE.md` phrase families in five parts and catalogue them, or record the
exception host-side. One obligation covers both; do not mint two.

---

## F-321 — the row that defines part 2 is a sound definition; its instance and the host pointer are what failed

**Outcome:** RE-JUDGE: confirmed
**Anchors:** 0 of 1 edited — `##ROW-PART-AUTHORITATIVE-DESCRIPTION`.
**Files touched:** none

**Re-verification:**

```
$ rg -n 'ROW-PART-AUTHORITATIVE-DESCRIPTION' packages/org.vibevm.world/operating-modes/v0.1.0/spec/flows/operating-modes/OPERATING-MODES-PROTOCOL.md
66:| ##ROW-PART-AUTHORITATIVE-DESCRIPTION **2. Authoritative description** @impl/done | the owner's own framing of what the mode is for, recorded verbatim @impl/done |

$ rg -n 'verbatim description is recorded in the flow' spec/common/PROP-006-operating-modes.md
19:##mfbt-pointer The pre-authorised heads-down execution codeword — … (the owner's verbatim description is recorded in the flow) — …
```

**Who falsifies it:** neither, once F-202 landed. The row is a definition and
was never false; the contradiction was `mfbt-mode.md` presenting a rendering
as satisfying it, and that claim is now withdrawn inside the same package.
What remains — `spec/common/PROP-006-operating-modes.md:19` telling a session
the verbatim description «is recorded in the flow» — is a host sentence:
route (b).

**What changed and why:** nothing. Softening «recorded verbatim» in the
definition so a translation qualifies is precisely the move that cost three
reverts in wave 2. The package's exemplar was repaired instead, at
`mfbt-mode.md` under F-202, which is where the false claim actually sat.

**New obligations noticed:** host obligation, already recorded under F-202
item 2 — repoint or correct `spec/common/PROP-006-operating-modes.md:19`. Do
not mint a second.

---

## F-174 — «three orderings, no two agree» rests on reading a file-role taxonomy as a priority order

**Outcome:** RE-JUDGE: confirmed
**Anchors:** 0 of 4 edited — `##ROW-TESTS-ARE-THE-SPEC-EXECUTABLE`,
`##ROW-CODE-BEATS-WAL`, `##TESTS-SIT-BESIDE-THE-SPEC-NOT-BELOW-IT`,
`##SUM-FIXED-PRIORITY-SETTLES-EVERYTHING`.
**Files touched:** none

**Re-verification:**

```
$ sed -n '36,45p' spec/boot/00-core.md
## Reading layers (per book, `refs/book/`)
##reading-layers-lead vibevm's instance of the **two-process-model** flow (…) — human and agent
as two processes sharing one repository; these are its reading layers, information flowing
top-down, the human winning conflicts: @impl/done
- ##LAYER-HEAD **Head** (human's memory) — … Human wins conflicts with the spec. @impl/done
- ##LAYER-WAL **WAL** (`spec/WAL.md`) — volatile, rewritten each session, describes *current* state.
- ##LAYER-SPEC **Spec** (other files under `spec/`) — stable decisions, addressable via spec:// URIs.
- ##LAYER-CODE **Code** (everything under `crates/`, including each crate's own `tests/`) — artefacts.
  Losing them is inconvenient; losing the spec is a catastrophe. @impl/done

$ sed -n '36,46p' spec/boot/00-core.md | rg -c '>'
0 matches

$ sed -n '183,196p' spec/boot/STATIC.md
## The hierarchy {#hierarchy}
Every disagreement between layers is settled by fixed priority:
    Human  >  Spec  >  Tests  >  Code  >  WAL
- The human may change the spec; nobody else may — silently.
- Code must conform to the spec, never the other way around.
- Tests are the spec in executable form: a test that contradicts the spec is a bug in exactly
  one of the two, never both.
- The volatile state file (WAL or equivalent) is a record, dead last: when it disagrees with
  anything above it, it is stale.

$ sed -n '166,180p' refs/book/chapter-1-two-process-model.md
… Можно выделить три уровня:
**Уровень 1: Управляющие файлы** (человек → AI) — Boot-файл … / WAL … / Спецификации (PROP, FEAT) …
**Уровень 2: Артефакты** (AI → человек, верифицируемые) — Код … / Тесты … / Обновления спек …

$ sed -n '205,213p' refs/book/chapter-1-two-process-model.md
Иерархия приоритетов:
1. Человек побеждает спеку (человек может изменить спеку)
2. Спека побеждает код (код должен соответствовать спеке)
3. Тесты — это спека в исполняемой форме (если тест противоречит спеке, это баг в тесте
   или в спеке, но не в обоих)

$ diff <(sed -n '160,215p' refs/book/chapter-1-two-process-model.md) \
       <(sed -n '160,215p' packages/org.vibevm.world/redbook/v0.2.0/spec/book/ru/chapter-1-two-process-model.md) \
  && echo "IDENTICAL 160-215"
IDENTICAL 160-215
```

**Who falsifies it:** neither — the verdict's premise does not hold. The
falsifier it names is the host, and the host sentence it reads as a competing
priority order is not one.

**Perimeter searched:** every place the host states an ordering — `spec/boot/`
in full (`00-core.md`, `STATIC.md`, `INDEX.md` and every snippet it names),
`spec/common/`, `spec/modules/**`, `CLAUDE.md`, and the reference the host
cites, `refs/book/` chapters 1–3 plus its packaged twin
`packages/org.vibevm.world/redbook/v0.2.0/spec/book/ru/`. Two structures
exist, on two axes; a third ordering does not.

**What changed and why:** nothing, because there are not three orderings.
`spec/boot/00-core.md:36-45` is titled **«Reading layers»**, calls itself
«its reading layers», belongs to a *different* flow (`two-process-model`),
contains **no `>` operator at all**, and makes exactly one conflict claim —
«the human winning conflicts». It is vibevm's instance of the book's
*file-role taxonomy* («Уровень 1: Управляющие файлы» — Boot, WAL, Specs;
«Уровень 2: Артефакты» — Code, Tests), which chapter 1 states forty lines
before it states «Иерархия приоритетов». The verdict converted list position
into rank and produced «WAL SECOND, above Spec and Code», which the host text
never says. The host's one stated conflict priority is
`spec/boot/STATIC.md:185-196` — this flow's own snippet, installed verbatim,
identical to the flow. On the Tests anchors the same conflation applies:
`00-core.md:43` classifies `crates/**/tests/` as a generated *artefact* (the
book's Level 2, which lists Код and Тесты together), while
`TESTS-SIT-BESIDE-THE-SPEC-NOT-BELOW-IT` is about *authority in a
contradiction*. The book holds both statements in one chapter, and so do the
flow and the host. Broadening `SUM-FIXED-PRIORITY-SETTLES-EVERYTHING` to admit
«WAL second» would rewrite the one-line law the host reads at boot so that a
taxonomy misread becomes the rule — the reverted wave-2 shape exactly.

**New obligations noticed:** `CONFLICT-PROTOCOL.md` does contradict itself, on
a point no verdict in this obligation names and which I have not touched.
`##THE-ORDER-IS-TOTAL-AND-ACYCLIC` («this order is total and acyclic») and
`##predictability-is-the-entire-point` («Every pairing has a predetermined
winner») cannot both hold with `##ROW-TESTS-ARE-THE-SPEC-EXECUTABLE`
(«Tests = Spec») and `##A-TEST-CONTRADICTING-THE-SPEC-IS-A-BUG-IN-EXACTLY-ONE`
(«a bug in exactly one of the two … either the test is stale or the spec prose
has decayed») — the Spec/Tests pairing has no predetermined winner by the
document's own §tests. The book writes relation 3 as an *identity*, not a
rung, and the flow renders it as a `>` link in a chain it then calls total.
Recorded, not fixed: the anchors are outside this obligation, and the chain is
installed verbatim in the host's boot lane, so a repair is a release event.

---

## F-196 — the boot snippet's hierarchy, same finding as F-174 at the compiled lane

**Outcome:** RE-JUDGE: confirmed
**Anchors:** 0 of 3 edited — `##EVERY-DISAGREEMENT-IS-SETTLED-BY-FIXED-PRIORITY`,
`##TESTS-ARE-THE-SPEC-IN-EXECUTABLE-FORM`,
`##THE-VOLATILE-STATE-FILE-IS-A-RECORD-DEAD-LAST`.
**Files touched:** none

**Re-verification:**

```
$ rg -n 'reading layers' spec/boot/00-core.md
38:##reading-layers-lead … these are its reading layers, information flowing top-down, the human
winning conflicts: @impl/done

$ sed -n '36,46p' spec/boot/00-core.md | rg -c '>'
0 matches

$ rg -n 'Every disagreement between layers is settled by fixed priority' spec/boot/STATIC.md
185:Every disagreement between layers is settled by fixed priority:
```

**Who falsifies it:** neither — the host's stated conflict priority at
`spec/boot/STATIC.md:185-196` **is** this snippet, installed verbatim. The
only competing text is the reading-layers taxonomy, which is not a priority
order (F-174).

**Perimeter searched:** as F-174 — `spec/boot/**` in full, `spec/common/`,
`spec/modules/**`, `CLAUDE.md`, `refs/book/` chapters 1–3 and the redbook
package twin.

**What changed and why:** nothing. The verdict's sharpest claim — that a
session reads two contradictory orders «within the same session-start read» —
is the one that fails on inspection: what it reads is one priority order
(`STATIC.md`) and one file-role taxonomy (`00-core.md`), and the taxonomy
carries no ordering operator. `THE-VOLATILE-STATE-FILE-IS-A-RECORD-DEAD-LAST`
is confirmed by the host's own mechanised practice, which the verdict itself
concedes: `spec/boot/00-core.md:64` rewrites the WAL wholesale at session end
precisely because it is a record and not a source of truth.

**New obligations noticed:** none beyond F-174's.

---

## F-223 — the README's one-line law, same finding as F-174 at the package's front door

**Outcome:** RE-JUDGE: confirmed
**Anchors:** 0 of 2 edited — `#the-core-law-fits-on-one-line`,
`##HUMAN-CHANGES-THE-SPEC-CODE-CONFORMS-AND-STATE-FILES-ARE-LAST`.
**Files touched:** none

**Re-verification:**

```
$ rg -n 'practice-extracted-from-the-book' -A3 packages/org.vibevm.world/conflict-protocol/v0.1.0/README.md
86:##practice-extracted-from-the-book The practice is extracted from *AI-native development*, chapters 1–2:
87:the priority hierarchy and the memory-fence framing come from
88:chapter 1 (two co-processors sharing files as their only IPC); the
89:REVIEW protocol, the silent-change data race, and the three failure modes come from chapter 2.

$ sed -n '205,213p' refs/book/chapter-1-two-process-model.md   # three relations, no WAL rung
Иерархия приоритетов: 1. Человек побеждает спеку … 2. Спека побеждает код … 3. Тесты — это
спека в исполняемой форме …
```

**Who falsifies it:** the host, on the verdict's own reading — and that
reading is the taxonomy misread ruled at F-174. Nothing inside the package
falsifies these two sentences.

**Perimeter searched:** as F-174, plus the provenance claim's own target —
`refs/book/chapter-1-two-process-model.md` and the packaged twin, checked
byte-for-byte over lines 160–215.

**What changed and why:** nothing. The README's only claim about a sibling
package is `##practice-extracted-from-the-book`, and it says the hierarchy
«comes from» chapter 1 — a provenance claim, which an extension at the tail
(the WAL rung) does not falsify; the book states no WAL rung either way. The
core law is the flow's own and is installed host-side verbatim. «State files
are last» is inverted only if the reading-layers list is a ranking, which it
is not.

**New obligations noticed:** none beyond F-174's.

---

## F-298 — the host recorded a different weighting, deliberately and in writing: §3.6 route (c)

**Outcome:** RE-JUDGE: confirmed — route (c), exception recorded host-side at
`spec://org.vibevm.core/vibevm/common/PROP-000#dep-weight`
**Anchors:** 0 of 1 edited — `##ROW-PREFER-NO-NEW-DEPENDENCY`.
**Files touched:** none

**Re-verification:**

```
$ rg -n 'dep-weight|DEP-WEIGHT-NOT-FACTOR|PICK-STRONGEST|TOO-HEAVY-NOT-REASON' spec/common/PROP-000.md
206:## 15. Dependency weight is not a decision factor {#dep-weight}
208:- ##DEP-WEIGHT-NOT-FACTOR **Decision:** Binary size, crate count, transitive dep weight are NOT
     decision factors when selecting third-party libraries. @spec/done
209:- ##PICK-STRONGEST Pick the strongest available library for the job … @spec/done
221:##TOO-HEAVY-NOT-REASON "Too heavy" alone is **not** a reason. @spec/done

$ rg -n 'ROW-PREFER-NO-NEW-DEPENDENCY' packages/org.vibevm.world/conflict-protocol/v0.1.0/spec/flows/conflict-protocol/uncertainty-protocol.md
85:| ##ROW-PREFER-NO-NEW-DEPENDENCY No new dependency @spec/done | Adding one @spec/done | A
     dependency is a permanent tax, and its removal is a migration. @spec/done |
```

**Who falsifies it:** the host — and it did so in the marked, recorded form
§3.6(c) exists for, which is why the fact is confirmed rather than deferred.

**Perimeter searched:** `spec/common/PROP-000.md` §15 in full (lines 206–226),
plus `spec/common/`, `spec/modules/**`, `CLAUDE.md` and `spec/boot/**` for any
other dependency-selection rule. The verdict's own NOTE FOR THE RECORD is
upheld: the delegated table's «no host rule discouraging new dependencies
exists» was an unchecked absence, and §15 exists and is the governing anchor.

**What changed and why:** nothing. Two points, either sufficient. First, the
two rules answer different questions on different criteria: §15 forbids
**weight** (binary size, crate count, transitive weight) as a criterion *when
selecting among libraries for a job that needs one*, and lists licence,
abandonment, security and API ergonomics as the legitimate rejections; this
row is a **tie-breaker for the spec-is-silent path**, ranked by *cost to
reverse*, about whether to introduce a dependency at all. Second, and decisive
for the route: to the extent they pull against each other, the host has
weighed the row's stated reason («a dependency is a permanent tax») and ruled
against it at the governing anchor, in decision-record form, with a why
(`##WHY-DEBT`). That is a **marked exception on the host side** — Phase C's
own ruling is that a marked exception is not drift. The package does not move
and the exception is named.

**New obligations noticed:** none.

---

## F-225 — «either fires unprompted» is refuted by its own package: «triggers do not fire themselves»

**Outcome:** EDITED
**Anchors:** 1 of 2 edited — `##either-condition-fires-unprompted` edited;
`##SUM-DATA-REASONS-AND-A-MEASURABLE-TRIGGER` re-judged confirmed, route (b).
**Files touched:**
`C:\Users\olegc\git\v\vibevm\packages\org.vibevm.world\decision-records\v0.1.0\spec\flows\decision-records\record-template.md`

**Re-verification:**

```
$ rg -n 'TRIGGERS-DO-NOT-FIRE-THEMSELVES|nothing-pages-anyone|RE-READING-IS-WHAT-FIRES-TRIGGERS' \
     packages/org.vibevm.world/decision-records/v0.1.0/spec/flows/decision-records/revisit-triggers.md
97:##TRIGGERS-DO-NOT-FIRE-THEMSELVES Triggers do not fire themselves. @spec/done
99:##nothing-pages-anyone-for-a-decision-grade-signal Nothing pages anyone when p99
103:##RE-READING-IS-WHAT-FIRES-TRIGGERS The mechanism that actually

$ sed -n '127,136p' packages/.../record-template.md
##note-the-trigger-shape Note the trigger's shape: two disjunct conditions, both observable —
one an unambiguous external event, one a threshold on a fact anyone
can check from the upstream repository today. @spec/done

##either-condition-fires-unprompted Either can be answered
yes-or-no by a stranger without the project instrumenting anything —
though neither fires itself: what fires a trigger is a re-read
([`revisit-triggers.md` §periodic-sweep](revisit-triggers.md#periodic-sweep)). @spec/done

$ f=packages/.../record-template.md; diff <(git show HEAD:$f | rg -o '##[A-Za-z][A-Za-z0-9-]*' | sort) \
      <(rg -o '##[A-Za-z][A-Za-z0-9-]*' $f | sort) && echo "ANCHOR SET IDENTICAL"
ANCHOR SET IDENTICAL

$ for pat in '\*\*Decision' '\*\*Why' '\*\*Considered and rejected' '\*\*(When to revisit|Revisit when)'; do
    rg -c --glob '*.md' "$pat" spec/common spec/modules | awk -F: -v p="$pat" '{s+=$NF} END {print p, "->", s+0}'; done
\*\*Decision -> 154
\*\*Why -> 33
\*\*Considered and rejected -> 4
\*\*(When to revisit|Revisit when) -> 7
```

**Who falsifies it:** for the edited anchor, the document's own package —
`revisit-triggers.md:97` says «Triggers do not fire themselves» and `:103`
names re-reading as the mechanism that does fire them, `@impl/done`, three
files away in the same install slot. For the summary anchor, the host.

**What changed and why:** `either-condition-fires-unprompted` claimed the two
worked-example conditions «fire without anyone having to remember to wonder»,
which is precisely what the package's trigger-design document denies in one
sentence. The property the example actually demonstrates — and which
`##note-the-trigger-shape`, the sentence immediately above it, already states
— is that both conditions are *observable without project instrumentation*: a
stranger can answer them on sight. The edit keeps that and drops the firing
claim, pointing at the sibling section that owns the mechanism. Nothing is
weakened: the stranger test and the three-part anatomy stand unchanged.
`SUM-DATA-REASONS-AND-A-MEASURABLE-TRIGGER` is a different case — its verdict
is the host census (0 of 22 whys cite a log path or sample size, 4 rejection
fields, 0 of 11 complete triggers), which is the host failing a sound summary,
not the summary being false. Route (b), no edit.

**New obligations noticed:** the host census above is itself the host
obligation, shared with F-197 / F-198 / F-224 — one obligation, not four.

---

## F-224 — the field definition is sound; the verdict counted the package's own sanctioned event triggers as failures

**Outcome:** RE-JUDGE: confirmed — residual non-compliance is route (b), host
obligation
**Anchors:** 0 of 2 edited —
`spec/boot/25-flow-decision-records.md#ROW-FIELD-WHEN-TO-REVISIT`,
`spec/flows/decision-records/record-template.md#ROW-FIELD-WHEN-TO-REVISIT`.
**Files touched:** none

**Re-verification:**

```
$ rg -n 'EVENT-TRIGGERS-ARE-THE-SANCTIONED-NON-NUMERIC-VARIANT|EVENT-TRIGGERS-TAKE-THE-SAME-TEST|event-trigger-examples' \
     packages/org.vibevm.world/decision-records/v0.1.0/spec/flows/decision-records/revisit-triggers.md
58:##EVENT-TRIGGERS-ARE-THE-SANCTIONED-NON-NUMERIC-VARIANT **Event triggers** are the sanctioned variant for non-numeric
61:##event-trigger-examples "A compliance requirement mandates a NIST-approved hash"; "upstream
64:##EVENT-TRIGGERS-TAKE-THE-SAME-TEST The test is the same — a stranger could answer yes-or-no today.

$ rg -n --glob '*.md' '\*\*(When to revisit|Revisit when)' spec/ | wc -l
12
   (11 host lines + spec/boot/STATIC.md:255, which is this package's own snippet installed verbatim)
```

**Who falsifies it:** the host — and only on the third of the definition's
three parts. The eight «event-shaped without a threshold» lines the verdict
counted as failures are the variant this package explicitly sanctions.

**Perimeter searched:** every revisit line the host has —
`rg '\*\*(When to revisit|Revisit when)' spec/` across `spec/boot/**`,
`spec/common/`, `spec/modules/**` and `spec/terraforms/`, 12 hits, one of
which is the installed snippet itself. Also `CLAUDE.md`, `campaigns/**` and
`legacy-spec/**` for records outside `spec/` — the campaign plans carry
`**Revisit when:**` lines, and the verdict already notes they are outside the
spec tree the summary governs.

**What changed and why:** nothing. The verdict measured all 11 host lines
against «metric + threshold + observation point» alone, and scored eight of
them failures for being event-shaped — but `revisit-triggers.md:58` states
that **event triggers are the sanctioned variant for non-numeric conditions**,
`@impl/done`, and `:64` gives them the same stranger test. Two more of the 11
are refusals to revisit, not malformed triggers. What genuinely fails is the
observation point (1 of 11 names one) and, far more, that 142 of ~154
Decision-bearing sections carry no revisit condition at all — host
non-compliance with a sound rule, route (b). Adding «or an unambiguous
external event» to the two rows was considered and **not** done: the evidence
this obligation carries is the host's practice, and an edit justified by host
practice is route (b) by §3.6 whatever its wording.

**New obligations noticed:** `record-template.md` contradicts itself on this
field, on evidence this obligation does not carry, so it is recorded and not
fixed. `##ROW-FIELD-WHEN-TO-REVISIT` (line 45) defines the field as «Metric +
threshold + observation point», and `{#example-library}` twelve lines below
(lines 122–124) ships a **pure event trigger** — «if a compliance requirement
mandates a NIST-approved hash, or blake3 upstream ships no release for 24
months» — with no metric and no threshold, presented as the correct model, and
`##note-the-trigger-shape` calls it «an unambiguous external event». The same
elision is in `DECISION-RECORDS-PROTOCOL.md#ROW-FIELD-WHEN-TO-REVISIT`
(line 78) and `spec/boot/25-flow-decision-records.md#ROW-FIELD-WHEN-TO-REVISIT`
(line 25) — three renderings of one field definition, all omitting the
sanctioned variant that `revisit-triggers.md#anatomy` states. One obligation,
three anchors, falsifier `self`.

---

## F-197 — the boot snippet's core rule is sound; 149 of the host's 153 Decision sections are the failure it names

**Outcome:** RE-JUDGE: confirmed — route (b), host obligation, registry row to
`deferred` per §3.6(b)
**Anchors:** 0 of 3 edited — `##ANY-REOPENABLE-CHOICE-GETS-A-RECORD`,
`##ASK-RATHER-THAN-INVENT-DATA`,
`##NEVER-RECORD-A-MISSING-REASON-OR-TRIGGER`.
**Files touched:** none

**Re-verification:**

```
$ rg -c --glob '*.md' '\*\*Decision' spec/common spec/modules | awk -F: '{s+=$NF} END {print "Decision labels:", s}'
Decision labels: 154

$ rg -ln --glob '*.md' '\*\*Considered and rejected' spec/common spec/modules
spec/modules\vibe-progress\PROP-043-progress-markup.md
spec/modules\vibe-cli\PROP-036-package-tree.md

$ rg -n '\*\*Considered and rejected' spec/modules/vibe-cli/PROP-036-package-tree.md \
     spec/modules/vibe-progress/PROP-043-progress-markup.md
PROP-036-package-tree.md:93:   - ##decision-artifacts-rejected **Considered and rejected:** …
PROP-043-progress-markup.md:96:  - ##element-name-rejected **Considered and rejected:** …
PROP-043-progress-markup.md:139: - ##freeze-rejected **Considered and rejected:** …
PROP-043-progress-markup.md:251: - ##registers-rejected **Considered and rejected:** …
```

**Who falsifies it:** the host. Three sound rules; a corpus that does not
follow them.

**Perimeter searched:** `spec/common/` and `spec/modules/**`, every markdown
file, for all four field labels — the census above is my own third
instrument and lands on the same four complete records the verdict names
(PROP-036 §2.3; PROP-043 §§3.1, 3.3, 3.8), in the same two files.

**What changed and why:** nothing. `ASK-RATHER-THAN-INVENT-DATA` and
`NEVER-RECORD-A-MISSING-REASON-OR-TRIGGER` name the host's failure exactly —
the two-field stub, the fact with decoration — and a rule is not made false by
a corpus made of what it forbids; that is the definition of non-compliance.
The verdict itself confirms the other half («no fabricated measurement was
found in any host why»), which is the part that would have been the package's
problem. Softening «any choice … gets a four-field record» so 127
Decision-only sections qualify is the reverted wave-2 shape.

**New obligations noticed:** one host obligation, shared with F-198, F-224 and
F-225's summary anchor and to be minted once, not four times: **149 of the
host's ~154 Decision-bearing sections are two-field stubs or bare Decision
lines.** Either complete them at the governing anchors, or record host-side
that vibevm's spec tree keeps Decision lines without the full record shape —
a §3.6(c) exception the owner rules on, since it is a policy choice rather
than a note. `spec/common/` contributes 0 of the 4 complete records.

---

## F-198 — «every record carries exactly four fields» measured against 4 of 153: the corpus fails, the rule does not

**Outcome:** RE-JUDGE: confirmed — route (b), host obligation, registry row to
`deferred` per §3.6(b)
**Anchors:** 0 of 3 edited — `##EVERY-RECORD-CARRIES-EXACTLY-FOUR-FIELDS`,
`##ROW-FIELD-WHEN-TO-REVISIT`, `##SUM-FOUR-FIELDS-ALWAYS`.
**Files touched:** none

**Re-verification:**

```
$ for pat in '\*\*Decision' '\*\*Why' '\*\*Considered and rejected' '\*\*(When to revisit|Revisit when)'; do
    rg -c --glob '*.md' "$pat" spec/common spec/modules | awk -F: -v p="$pat" '{s+=$NF} END {print p, "->", s+0}'; done
\*\*Decision -> 154
\*\*Why -> 33
\*\*Considered and rejected -> 4
\*\*(When to revisit|Revisit when) -> 7

$ rg -n 'SUM-THE-BEFORE-AFTER-TEST|NEVER-RECORD-A-MISSING-REASON-OR-TRIGGER' \
     packages/org.vibevm.world/decision-records/v0.1.0/spec/flows/decision-records/record-template.md \
     packages/org.vibevm.world/decision-records/v0.1.0/spec/boot/25-flow-decision-records.md
record-template.md:151:- ##SUM-THE-BEFORE-AFTER-TEST … if the record only says what the code already says, it is a
     fact with decoration — complete it or delete it. @impl/done
25-flow-decision-records.md:77:- ##NEVER-RECORD-A-MISSING-REASON-OR-TRIGGER Never record a decision with a missing reason or a
     missing revisit trigger — that is a fact with decoration, not a record. @impl/done
```

**Who falsifies it:** the host. Nothing inside the package contradicts «four
fields, always».

**Perimeter searched:** as F-197 — every markdown file under `spec/common/`
and `spec/modules/**`, all four field labels, counted independently of Phase C
and of the delegated worker; the `Considered and rejected` field lands in
exactly two files and four sections, which is the verdict's own «4 of 153».

**What changed and why:** nothing, and the package's own vocabulary is why.
By `record-template.md#SUM-THE-BEFORE-AFTER-TEST` and
`25-flow-decision-records.md#NEVER-RECORD-A-MISSING-REASON-OR-TRIGGER`, a
section carrying a Decision line alone **is not a record** — it is «a fact
with decoration». So the host does not have 153 records of which 4 are
complete; it has 4 records and ~150 undocumented decisions. «Every record
carries exactly four fields» is true of the 4 and says nothing false about the
other 150; what the census measures is how much of the host's reasoning was
never recorded. Broadening «every» or «exactly four» to admit two-field stubs
would rewrite the discipline to describe a lax consumer — the *профанация*
§3.6 exists to prevent, and the third instance of the reverted wave-2 shape in
this batch.

**New obligations noticed:** none beyond F-197's single host obligation.

---

## Batch summary

| id | outcome | anchors edited |
|---|---|---:|
| F-177 | RE-JUDGE: confirmed (3) · route (b) (1) | 0 of 4 |
| F-201 | RE-JUDGE: confirmed · route (b) | 0 of 3 |
| F-202 | **EDITED** | 2 of 3 |
| F-321 | RE-JUDGE: confirmed | 0 of 1 |
| F-174 | RE-JUDGE: confirmed | 0 of 4 |
| F-196 | RE-JUDGE: confirmed | 0 of 3 |
| F-223 | RE-JUDGE: confirmed | 0 of 2 |
| F-298 | RE-JUDGE: confirmed · route (c) | 0 of 1 |
| F-197 | RE-JUDGE: confirmed · route (b) | 0 of 3 |
| F-198 | RE-JUDGE: confirmed · route (b) | 0 of 3 |
| F-224 | RE-JUDGE: confirmed · route (b) | 0 of 2 |
| F-225 | **EDITED** | 1 of 2 |

**3 anchors edited of 31.** Two files touched, both inside the three named
package directories; no anchor added, removed or renamed in either; no
cross-package relative link added; no `git` write command run.

**Host obligations to mint (4, deduplicated from 9 sightings):**

1. The MFBT authorisation exists in no live file — restore the owner's Russian
   original or rule that the English rendering stands (F-202, F-177, F-321);
   `spec/common/PROP-006-operating-modes.md:19` states the opposite today.
2. `CLAUDE.md`'s session-end and session-resume phrase families are acted on as
   hard contracts but are not catalogued and carry two of five parts (F-177,
   F-201).
3. 149 of the host's ~154 Decision-bearing sections are stubs or bare Decision
   lines; 142 carry no revisit condition (F-197, F-198, F-224, F-225).
4. `spec/common/PROP-000.md#dep-weight` is the recorded §3.6(c) exception that
   confirms F-298 — no work, record the exception against the row.

**Package obligations noticed and deliberately not fixed (2):**

1. `CONFLICT-PROTOCOL.md#THE-ORDER-IS-TOTAL-AND-ACYCLIC` +
   `#predictability-is-the-entire-point` against `#ROW-TESTS-ARE-THE-SPEC-EXECUTABLE`
   + `#A-TEST-CONTRADICTING-THE-SPEC-IS-A-BUG-IN-EXACTLY-ONE` — the Spec/Tests
   pairing has no predetermined winner; the chain is installed verbatim
   host-side, so repairing it is a release event.
2. The `When to revisit` field definition omits the sanctioned event variant in
   all three of its renderings — `record-template.md:45`,
   `DECISION-RECORDS-PROTOCOL.md:78`, `25-flow-decision-records.md:25` — against
   `revisit-triggers.md#EVENT-TRIGGERS-ARE-THE-SANCTIONED-NON-NUMERIC-VARIANT`
   and `record-template.md`'s own `{#example-library}`. Falsifier `self`.
