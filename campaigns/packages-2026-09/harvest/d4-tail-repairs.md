# D4 — the tail repairs (18 `prose-edit` obligations)

_Wave 4 of Phase D. Every obligation below was routed first with
`python campaigns/packages-2026-09/tasks/drift-registry.py --task F-NNN`;
all eighteen returned **`route: prose-edit`**, `release_event = False`,
`cross_package = False`, so none is OUT-OF-ROUTE on the §6.1
`##ROUTE-BEFORE-FALSIFIER` check. The route was confirmed a second time from
`run/state/obligations.json` before the first edit._

```
F-175 prose-edit  F-209 prose-edit  F-226 prose-edit  F-228 prose-edit
F-231 prose-edit  F-122 prose-edit  F-264 prose-edit  F-265 prose-edit
F-268 prose-edit  F-269 prose-edit  F-274 prose-edit  F-276 prose-edit
F-304 prose-edit  F-314 prose-edit  F-325 prose-edit  F-326 prose-edit
F-329 prose-edit  F-331 prose-edit
```

---

## F-268 — the demotion's basis was an absence, and the absence is false: the golden transcripts exist

**Outcome:** RE-JUDGE: confirmed
**Anchors:** 0 edited of 1 — `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/vibevm/vibespecs/mechanisms/BROWNFIELD-PROTOCOL-v0.1.xml#PHASE-GATES-NOW-MEAN-SNAPSHOTS-UNCHANGED`
**Files touched:** none

**Re-verification:**

```
$ ls -la discipline/golden/
-rwxr-xr-x 5209 Jul 12 22:54 capture.sh
-rw-r--r-- 1305 Jul 12 22:54 check-installed.transcript.md
-rw-r--r-- 2569 Jul 12 22:54 init.transcript.md
-rw-r--r-- 3383 Jul 12 22:54 install-qualified.transcript.md
-rw-r--r-- 1818 Jul 12 22:54 install-short-name.transcript.md
-rw-r--r-- 2303 Jul 12 22:54 uninstall.transcript.md

$ head -3 discipline/golden/capture.sh
#!/usr/bin/env bash
# discipline/golden/capture.sh — Phase −1 characterization capture
# (PLAYBOOK-TERRAFORM-VIBEVM v0.2 Phase −1; BROWNFIELD-PROTOCOL §6).

$ grep -n "golden" terraform/BASELINE.md
91:[`golden/`](golden/): **5 hermetic flows, 12 CLI steps, all exit 0**,
92:captured by the re-runnable [`golden/capture.sh`](golden/capture.sh)
95:`<TIMESTAMP>`, fixed `golden-proj` basename).
111:DBT-0001 records — the golden set should grow a `GitPackageRegistry`

$ grep -n "BROWNFIELD" terraform/BASELINE.md
119:debt/intent reference in the PR (BROWNFIELD §6).

$ grep -rn "discipline/golden" vibevm/vibepacks/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/vibevm/vibespecs/skills/rust-ai-native-sweep/SKILL.md
96:- ##DRIFT-GOLDEN-TRANSCRIPTS Golden transcripts (`discipline/golden/`): must fail loudly, re-captured
97:  deliberately (`capture.sh`), never auto-updated. @impl/done

$ sed -n '107p' vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/vibevm/vibespecs/mechanisms/BROWNFIELD-PROTOCOL-v0.1.xml
##PHASE-GATES-NOW-MEAN-SNAPSHOTS-UNCHANGED Phase gates that previously said "behavior unchanged" now mean "characterization snapshots unchanged, except where a debt/intent record says we changed it deliberately." @impl/done
```

**Who falsifies it:** nobody — the verdict's own premise is false, and the
supporting artefact is in the host.

**Perimeter searched:** `discipline/**` (the golden directory itself),
`terraform/**`, `tools/`, `xtask/`, `crates/`, `spec/**`, `legacy-spec/**`,
`.claude/skills/**`, `.agents/skills/**`, and the whole package tree — via
`grep -rn "capture.sh\|discipline/golden" --include=*.md --include=*.sh
--include=*.rs --include=*.toml --include=*.yml . --exclude-dir=vibedeps
--exclude-dir=refs --exclude-dir=.vibe --exclude-dir=target`. The earlier
demotion's perimeter was `rust-ai-native-cli/src/` alone.

**What changed and why:** nothing, and that is the finding. The verdict
demoted `PHASE-GATES-NOW-MEAN-SNAPSHOTS-UNCHANGED` on the claim that its
sibling `CAPTURE-GOLDEN-TRANSCRIPTS-AT-INVENTORY-TIME` promises snapshots
nothing captures. Five golden transcripts and a re-runnable `capture.sh` sit
at `discipline/golden/`, and `capture.sh`'s own header line 3 names
**BROWNFIELD-PROTOCOL §6** — the very section under judgement — as its
contract. `terraform/BASELINE.md:91` records the capture at inventory time
(«5 hermetic flows, 12 CLI steps, all exit 0») and its line 119 applies this
exact anchor by name: changing a pinned hint «must come with a debt/intent
reference in the PR (BROWNFIELD §6)». The shipped `rust-ai-native-sweep`
skill carries the same rule as a recurring sweep item. Both anchors are
implemented in the host, so `@impl/done` stands on both and the redefinition
rests on something that exists.

**New obligations noticed:** the host's phase-close **gate panel** as recorded
in `terraform/adopt-v0.3/LOG.md` (specmap `--check`, `conform check`,
`test-gate`, `fast-loop`) does not name a golden-transcript re-run among its
green lines, even though `capture.sh`'s header states the determinism check
(«Run it twice; `git diff discipline/golden` must be empty»). That is the
consumer doing less than the rule asks — §3.6 route (b), a **host** obligation
to add the golden diff to the gate panel or to record the exception. The
package does not move for it. Recorded, not fixed.

---

## F-209 — two dead addresses in the package map: a guide path that resolves in no package, and a `vibevm-terraform/` that exists nowhere

**Outcome:** EDITED
**Anchors:** 2 edited of 2 — `spec/00-MANIFESTO.md#MAP-RUST-GUIDE`,
`spec/00-MANIFESTO.md#ADOPTION-PLAN-LIVES-OUTSIDE`
(both `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/`)
**Files touched:** `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/vibevm/vibespecs/00-MANIFESTO.xml`

**Re-verification:**

```
$ grep -n "MAP-RUST-GUIDE\|ADOPTION-PLAN-LIVES-OUTSIDE" vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/vibevm/vibespecs/00-MANIFESTO.xml
172:- ##MAP-RUST-GUIDE `spec/rust/GUIDE-AI-NATIVE-RUST.md` in `stack:org.vibevm.ai-native/rust-ai-native-lang` — the law and scaffolds projected onto Rust; supersedes GUIDE-RUST-v0.1. (Pilot language.) @impl/done
187:##ADOPTION-PLAN-LIVES-OUTSIDE The vibevm-specific adoption plan lives OUTSIDE this package, in the host's `terraform/`, because the Discipline is the product and vibevm is its pilot. @impl/done

$ ls vibevm/vibepacks/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/vibevm/vibespecs/rust/GUIDE-AI-NATIVE-RUST.xml
vibevm/vibepacks/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/vibevm/vibespecs/rust/GUIDE-AI-NATIVE-RUST.xml

$ ls vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/vibevm/vibespecs/
00-MANIFESTO.xml  01-PATTERN-CARD-FORMAT.xml  02-EXECUTABLE-SCAFFOLDS.xml
03-RAID-PLAYBOOK.xml  04-SWEEP-PLAYBOOK.xml  05-CAMPAIGN-FORM.xml
06-WAL-CONVENTION.xml  appendix  boot  legacy-projections  mechanisms
   (no `rust/` — the old address resolved in no package at all)

$ ls terraform/
BASELINE.md  LOG.md  PHASE1-PILOT.md  REPORT.md  adopt-v0.3
discipline-sweep  specmap-proposals.json
$ ls terraform/adopt-v0.3/
LOG.md  PREDICTIONS.md  REPORT.md
```

**Who falsifies it:** a sibling package for `MAP-RUST-GUIDE`
(`rust-ai-native-lang` holds the guide), and the host's directory layout for
`ADOPTION-PLAN-LIVES-OUTSIDE` — both are **paths stated as fact**, §3.6 route
(a)'s first-named exemplar and the one route that «edits a package because of a
disagreement with the host». Neither is a rule, so no rule was broadened.

**Perimeter searched:** for `vibevm-terraform` — the whole working tree via
`Grep` (repo-wide, all file types). Fourteen hits, every one of them either this
same sentence in another slot (`core-ai-native/v0.7.0`, four `vibedeps/`
re-vendors), a campaign record quoting it, or the campaign spec noting the
defect. **Zero are a directory.** Confirmed with `ls -d vibevm-terraform` →
`No such file or directory`, and `ls terraform/` → the adoption artefacts.

**What changed and why:** `MAP-RUST-GUIDE` named `rust/GUIDE-AI-NATIVE-RUST.md`,
which resolves in neither package — core has no `spec/rust/`, and the rust stack
roots the guide at `spec/rust/`. It now names the file and the owning stack, in
the same `stack:org.vibevm.ai-native/rust-ai-native-lang` form
`SPEC-UNIT-URI-FORM` two lines above already uses, so no cross-package relative
link was introduced. `ADOPTION-PLAN-LIVES-OUTSIDE` claimed the plan lives in
`vibevm-terraform/`; the claim's substance («OUTSIDE this package») is true and
only the name was wrong, so the name was corrected to the host's `terraform/`
and nothing else in the sentence moved. Both markers stay `@impl/done`: the
content was never in question, only its address — which is what `relocation`
means.

**New obligations noticed:** the four sibling entries in the same list —
`MAP-RUST-TCG`, `MAP-RUST-TCG-AGENTIC`, `MAP-TYPESCRIPT-GUIDE`,
`MAP-TYPESCRIPT-TCG` — carry the identical defect (`rust/tools/…`,
`typescript/…`, all rooted in no package) and carry **no drift verdict**: a
query over `run/state/obligations.json` for those anchor names returns nothing.
Either the Phase C judge saw the same string and ruled differently four times,
or those anchors were never reached. Recorded, not fixed — they are outside this
obligation's `anchors` list. Also: `MAP-CARDS-SCAFFOLDS` (line 181) still names
the stacks by the superseded short form `stack-rust-ai-native` /
`stack-typescript-ai-native` while `SPEC-UNIT-URI-FORM` uses the qualified
`stack:org.vibevm.ai-native/…`; one package, two naming conventions.

---

## F-269 — the README's front-door pointer to the stack guide is one path segment short

**Outcome:** EDITED
**Anchors:** 1 edited of 1 — `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/README.md#READ-STACK-GUIDE`
**Files touched:** `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/README.md`

**Re-verification:**

```
$ grep -n "READ-STACK-GUIDE" vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/README.md
31:4. ##READ-STACK-GUIDE The active language stack's GUIDE (e.g. `spec/rust/GUIDE-AI-NATIVE-RUST.md` in the Rust stack). @impl/done

$ ls vibevm/vibepacks/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/vibevm/vibespecs/rust/GUIDE-AI-NATIVE-RUST.xml
vibevm/vibepacks/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/vibevm/vibespecs/rust/GUIDE-AI-NATIVE-RUST.xml
```

**Who falsifies it:** a sibling package — `rust-ai-native-lang` roots the guide
at `spec/rust/`, and the README sent the reader to `rust/`.

**What changed and why:** one path, `rust/GUIDE-AI-NATIVE-RUST.md` →
`spec/rust/GUIDE-AI-NATIVE-RUST.md`. The sentence already said «in the Rust
stack», so the owning package was never wrong and no package name needed adding
— which also means no cross-package relative link. Every neighbouring step in
the same numbered list (`spec/00-MANIFESTO.md`, `spec/mechanisms/`,
`spec/appendix/`) is already package-root-relative, so the repaired form matches
the list's own convention rather than inventing one. Marker unchanged: the guide
exists and is shipped.

**New obligations noticed:** none beyond F-209's list.

---

## F-264 — «Most cards aim here» is false in all three shipped registries: inline holds 2 of 9, gate holds 5

**Outcome:** EDITED
**Anchors:** 1 edited of 1 — `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/vibevm/vibespecs/00-MANIFESTO.xml#TRIGGER-INLINE`
**Files touched:** `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/vibevm/vibespecs/00-MANIFESTO.xml`

**Re-verification:**

```
$ grep -n "TRIGGER-INLINE" vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/vibevm/vibespecs/00-MANIFESTO.xml
129:1. ##TRIGGER-INLINE **Inline (edit-time)** — lint-detectable, fires in the per-cell loop; the cheapest mode and the one that fires most often. Each stack's `cards/INDEX.md` is the roster: of the nine scaffold cards, 2 sit here (C, F) against 5 at gate. @impl/done

$ grep -A4 "Trigger-mode delivery summary" vibevm/vibepacks/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/vibevm/vibespecs/cards/INDEX.xml
- **inline** (per-edit, lint-detectable): C, F. Most frequent; cheapest.
- **gate** (per-merge): B, D, E, G, H.
- **raid** (scheduled/on-adoption): A, I.
- **review** (human/strong-agent): none yet; reserved for judgment-heavy cards.

  … typescript-ai-native-lang/v0.6.0 — identical split (C, F | B, D, E, G, H | A, I | none)
  … go-ai-native-lang/v0.1.0        — identical split (C, F | B, D, E, G, H | A, I | none)
```

**Who falsifies it:** three shipped sibling packages — every `evidence_ref` on
this row is a file under `rust-ai-native-lang/`, and the typescript and go
stacks carry the same table. Not the host.

**What changed and why:** the sentence conflated two different claims that the
registries keep apart. «Fires most often» is true of the *mode* — the registries
say so in their own words («Most frequent; cheapest») — but «most cards aim
here» is a claim about the *roster*, and the roster puts 2 of the nine scaffold
cards at inline (C, F) and 5 at gate (B, D, E, G, H) in all three stacks
identically. The fact now states the frequency claim, which holds, and the count
from the registries, which is the thing that was wrong. The count is scoped to
«the nine scaffold cards» so that authoring one of the seven pending
rule/anti-pattern cards does not re-falsify it. The registry file is named as
`cards/INDEX.md`, the way `MAP-CARDS-INDEX` already names it, so no
cross-package relative link was introduced. Marker unchanged.

**New obligations noticed:** the three `cards/INDEX.md` files each carry
«Generated/maintained as a derived index (A2/R-030); hand edits are a defect»,
and no generator for them was found under `xtask/`, `tools/` or the stacks'
`crates/`. That is a `missing-support` shape on a sibling package's anchor, not
on any anchor of this obligation. Recorded, not fixed.

---

## F-265 — the format names a GoF section it never adopted: no card has *Related Patterns*, and the format's own field list has none either

**Outcome:** EDITED
**Anchors:** 1 edited of 1 — `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/vibevm/vibespecs/01-PATTERN-CARD-FORMAT.xml#PARENT-GOF-RECOGNITION`
**Files touched:** `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/vibevm/vibespecs/01-PATTERN-CARD-FORMAT.xml`

**Re-verification:**

```
$ grep -n "PARENT-GOF-RECOGNITION" vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/vibevm/vibespecs/01-PATTERN-CARD-FORMAT.xml
13:- ##PARENT-GOF-RECOGNITION **GoF → recognition & navigation.** Its load-bearing sections for us are *Applicability* (how to RECOGNIZE the situation from the code) and the neighbour-pattern pointer GoF calls *Related Patterns*, which this format carries under the name *Collaborations* (§1 Band 2 — where to go if this feels wrong). @impl/done

$ grep -c "Related Patterns" \
    vibevm/vibepacks/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/vibevm/vibespecs/cards/*.md \
    vibevm/vibepacks/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/vibevm/vibespecs/cards/*.md \
    vibevm/vibepacks/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/vibevm/vibespecs/cards/*.md | grep -v ":0"
   (no output — 0 in every one of the 30 shipped cards)

$ grep -n "FIELD-COLLABORATIONS\|FIELD-APPLICABILITY" vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/vibevm/vibespecs/01-PATTERN-CARD-FORMAT.xml
29:- ##FIELD-APPLICABILITY-RECOGNITION **Applicability / Recognition** — THE central section: …
33:- ##FIELD-COLLABORATIONS **Collaborations** — interaction with other cards and the toolchain at runtime. @impl/done
   (the §4 copy-paste authoring stub likewise has `Collaborations:` and no `Related Patterns:`)
```

**Who falsifies it:** the document itself, first and decisively — §1's Band-1
and Band-2 field lists and §4's authoring stub define nineteen `FIELD-*` units
and none of them is *Related Patterns*. The thirty shipped cards in three
sibling packages agree.

**Perimeter searched:** all three `spec/cards/` directories (30 card files) plus
the format document itself and the `core-ai-native/v0.7.0` predecessor slot, for
the literal string `Related Patterns` — via `grep -rn "Related Patterns"
vibevm/vibepacks/org.vibevm.ai-native/ --include=*.md`. The only hits are this sentence
and its v0.7.0 twin.

**What changed and why:** the fact claimed GoF's *Related Patterns* is
load-bearing «for us» while the format two sections later declines to carry it,
which made the document contradict itself — the exact shape §3.6 route (a)
covers. The GoF *role* really is load-bearing and really is shipped; only the
section name was borrowed unchanged. The sentence now says which GoF section the
role comes from and which field of *this* format carries it, so the parenthetical
«where to go if this feels wrong» keeps its referent. Nothing was added to or
removed from the field list. Marker unchanged.

**New obligations noticed:** `FIELD-COLLABORATIONS` describes itself as
«interaction with other cards and the toolchain at runtime» — the toolchain half
has no GoF parent and is not mentioned in `PARENT-GOF-RECOGNITION` or
`PARENT-OPERATIONAL-EXECUTION`. If the three-parents accounting is meant to be
exhaustive, that field is a fourth source. Recorded, not fixed.

---

## F-175 — the protocol's absolute «never amend / never rebase» is contradicted by its own boot snippet and its own summary; «only place the why is recorded» is contradicted by a sibling flow; the «mechanical» reading is the verdict's, not the package's

**Outcome:** EDITED (3 anchors) + RE-JUDGE: confirmed (1 anchor)
**Anchors:** 3 edited of 4 —
`#THE-MESSAGE-IS-THE-ONLY-PLACE-THE-WHY-IS-RECORDED-DURABLY` (edited),
`#FROZEN-NEVER-AMEND` (edited),
`#FROZEN-NEVER-REBASE-THE-PUSHED-RANGE` (edited),
`#THE-TEST-IS-MECHANICAL-THE-WORD-ALSO` (**RE-JUDGE: confirmed**, not edited).
All in `vibevm/vibepacks/org.vibevm.world/git-atomic-commits/v0.1.0/vibevm/vibespecs/flows/atomic-commits/ATOMIC-COMMITS-PROTOCOL.xml`
**Files touched:** `vibevm/vibepacks/org.vibevm.world/git-atomic-commits/v0.1.0/vibevm/vibespecs/flows/atomic-commits/ATOMIC-COMMITS-PROTOCOL.xml`

**Re-verification:**

```
$ sed -n '155,166p' vibevm/vibepacks/org.vibevm.world/git-atomic-commits/v0.1.0/vibevm/vibespecs/flows/atomic-commits/ATOMIC-COMMITS-PROTOCOL.xml
## Pushed history is frozen {#pushed}

##once-a-commit-has-been-pushed-lead Once a commit has been pushed: @impl/done

- ##FROZEN-NEVER-AMEND **Never** `git commit --amend` without explicit human approval. @impl/done
- ##FROZEN-NEVER-REBASE-THE-PUSHED-RANGE **Never** `git rebase -i` the pushed range without explicit
  human approval. @impl/done
- ##FROZEN-NEVER-FORCE-PUSH-WITHOUT-APPROVAL **Never** `git push --force` or `--force-with-lease` without
  explicit human approval. @impl/done

$ sed -n '216,217p' .../ATOMIC-COMMITS-PROTOCOL.xml      # the document's own summary, unchanged
- ##SUM-PUSHED-HISTORY-IS-FROZEN Pushed history is frozen. Amend/force-push only with human
  approval. @impl/done

$ sed -n '40,42p;63,64p' vibevm/vibepacks/org.vibevm.world/git-atomic-commits/v0.1.0/vibevm/vibespecs/boot/30-flow-atomic-commits.xml
4. ##STEP-DO-NOT-AMEND-OR-FORCE-PUSH-PUSHED-COMMITS Do not amend or force-push commits that are already pushed; create a
   new commit instead, unless the human explicitly approves history
   rewriting. @impl/done
- ##NEVER-AMEND-A-PUSHED-COMMIT-WITHOUT-HUMAN-APPROVAL Never `git commit --amend` on a pushed commit without explicit human
  approval. Same for `git push --force`. @impl/done

$ sed -n '73,79p' .../ATOMIC-COMMITS-PROTOCOL.xml
##THE-MESSAGE-IS-THE-ONLY-PLACE-THE-WHY-IS-RECORDED-DURABLY The commit message is where the *why* of a **change** is recorded at
per-change granularity, bound to its diff and surviving spec prose
decay and WAL overwrites. The *why* of a **decision** has its own
durable home — the sibling `decision-records` flow puts it at the
governing spec anchor
(`spec://org.vibevm.world/decision-records/flows/decision-records/DECISION-RECORDS-PROTOCOL#root`)
— and neither substitutes for the other. @impl/done

$ sed -n '5,10p' vibevm/vibepacks/org.vibevm.world/decision-records/v0.1.0/vibevm/vibespecs/flows/decision-records/DECISION-RECORDS-PROTOCOL.xml
##scope-of-this-document ... *where* records live (at the governing spec anchor, not in an ADR silo) ...

$ grep -rn "spec://org.vibevm.world/decision-records" --include=*.md packages/ spec/
vibevm/vibespecs/design/README.md:7: ... (the **decision-records** genre: `spec://org.vibevm.world/decision-records/flows/decision-records/DECISION-RECORDS-PROTOCOL#root`) ...
   (the URI form written into the fact is the one already in use, not invented here)

--- the anchor that was NOT edited, and why ---
$ grep -rni "mechanical" vibevm/vibepacks/org.vibevm.world/git-atomic-commits/v0.1.0 --include=*.md
README.md:27:  ... `splitting-large-changes.xml` — mechanical procedure for turning a messy working tree into a sequence of atomic commits using `git add -p` ...
ATOMIC-COMMITS-PROTOCOL.xml:91: ##THE-TEST-IS-MECHANICAL-THE-WORD-ALSO The test is mechanical: ...
ATOMIC-COMMITS-PROTOCOL.xml:112: ##sibling-document-pointers Mechanical procedure for producing the split:
ATOMIC-COMMITS-PROTOCOL.xml:213: - ##SUM-THE-NO-ALSO-TEST The "no 'also' in the body" test catches violations mechanically.
splitting-large-changes.xml:169: ##the-agent-is-better-at-this-than-most-humans-under-pressure This is a highly mechanical task and the agent is genuinely better
splitting-large-changes.xml:216: - ##SUM-DELEGATE-AND-VERIFY-THE-PLAN Delegate the mechanical split to the agent; verify the plan,

$ sed -n '88,89p' vibevm/vibepacks/org.vibevm.world/decision-records/v0.1.0/vibevm/vibespecs/flows/decision-records/revisit-triggers.xml
##THE-MECHANICAL-TEST The mechanical test: *"when it breaks" is not a trigger; a trigger
is a measurable signal.*
```

**Who falsifies it:** the document itself and its own package for all three
edited anchors — the `#pushed` list is contradicted by this same document's
`SUM-PUSHED-HISTORY-IS-FROZEN` and by two facts in this same package's boot
snippet; and `THE-MESSAGE-IS-THE-ONLY-PLACE...` is contradicted by a **shipped
sibling package** in the same corpus. The host's practice decided nothing here
and is not cited.

**Perimeter searched:** for the meaning of «mechanical» — the whole
`git-atomic-commits` package (`README.md`, both protocol files, the boot
snippet) and all 27 `vibevm/vibepacks/org.vibevm.world/*` packages, via
`grep -rni "mechanical\|mechanically" vibevm/vibepacks/org.vibevm.world/ --include=*.md`.

**What changed and why:**

- **`FROZEN-NEVER-AMEND` / `FROZEN-NEVER-REBASE-THE-PUSHED-RANGE`** — the
  three-item list forbade amend and rebase absolutely while its third bullet,
  the document's own summary, and **the same package's boot snippet** all carry
  the approval clause. The snippet is the lane a session actually reads and it
  says «unless the human explicitly approves history rewriting», which is what
  `git rebase -i` on a pushed range *is*. Both bullets now carry the clause the
  other three carriers already had, so the list is uniform and the document
  stops contradicting itself. **Nothing was relaxed:** the rule the package
  publishes to its consumers through the boot lane is unchanged; only the
  protocol's outlier restatement was brought back to it.
- **`THE-MESSAGE-IS-THE-ONLY-PLACE-THE-WHY-IS-RECORDED-DURABLY`** — «the only
  place» is an exclusivity claim, and the same corpus ships `decision-records`
  whose declared scope is «*where* records live (at the governing spec anchor,
  not in an ADR silo)» and whose thesis is that a decision «cannot be recovered
  at all» unless written. Two installed flows claimed one exclusive role. The
  fact now keeps everything it was actually defending — per-change granularity,
  bound to the diff, surviving prose decay and WAL overwrites — and names the
  other home for the other granularity. The sibling is named by `spec://` URI,
  the form already used at `vibevm/vibespecs/design/README.md:7`, so no cross-package
  relative link was added.

**RE-JUDGE: confirmed — `THE-TEST-IS-MECHANICAL-THE-WORD-ALSO`.** The verdict
reads «mechanical» as «machine-checkable» and then falsifies it with «no
commit-message check of any kind exists in this repository» — a fact about the
**host**, route (b), plus a reading the corpus does not support. This package
uses «mechanical» four other times and every one means *rote, delegable to an
agent*, never *machine-executable*: the README calls a `git add -p` procedure
«mechanical»; `splitting-large-changes.xml:169` calls it «a highly mechanical
task and the agent is genuinely better» at it; :216 says «delegate the mechanical
split to the agent». The sibling `decision-records` flow uses the identical
construction for an identical judgement-shaped discriminator
(`##THE-MECHANICAL-TEST`, `revisit-triggers.xml:88`). Under the corpus's own
vocabulary the sentence is true, and a demotion here would be the §6.1
`##ABSENCE-NAMES-ITS-PERIMETER` mistake wearing a different hat.

**New obligations noticed:**

1. `spec/boot/30-flow-atomic-commits.md#WHY-COMMIT-LOG-AS-DECISION-RECORD`
   carries the *same* «the message is the only place where *why* survives»
   sentence that was just repaired in the protocol, and it is compiled into the
   host at `vibevm/vibespecs/boot/STATIC.xml:602-603`. It is **not** in this obligation's
   anchor list and carries no verdict of its own — so the package now states the
   repaired form in the protocol and the old exclusive form in the boot lane.
   This one needs an obligation of its own; recorded, not fixed.
2. `#SUM-THE-NO-ALSO-TEST` («catches violations mechanically», `@impl/done`)
   is the summary twin of the anchor re-judged confirmed above; it stands or
   falls with it and carries no verdict either.

---

## F-304 — the README claims a role its own first paragraph and its own boot snippet both hand to a sibling package

**Outcome:** EDITED
**Anchors:** 1 edited of 1 — `vibevm/vibepacks/org.vibevm.world/git-atomic-commits/v0.1.0/README.md#COMPOSES-SYNC-FROM-CODE-FOR-THE-COMMIT-MESSAGE`
**Files touched:** `vibevm/vibepacks/org.vibevm.world/git-atomic-commits/v0.1.0/README.md`

**Re-verification:**

```
$ sed -n '53,62p' vibevm/vibepacks/org.vibevm.world/git-atomic-commits/v0.1.0/README.md
## Composition {#composition}

- ##COMPOSES-WAL-AND-SYNC-FROM-CODE-BY-DISTINCT-PREFIXES Works with `flow:wal` (`10-…`) and `flow:sync-from-code` (`20-…`):
  numeric boot-snippet prefixes are distinct by design. @impl/done
- ##COMPOSES-SYNC-FROM-CODE-FOR-THE-COMMIT-MESSAGE `flow:sync-from-code`'s final step is a `docs(spec)` commit; this
  flow is why the sync lands as its own commit and not folded into the
  code change. The *format* of that message is pinned by the sibling
  `flow:git-conventional-commits`, not here. @impl/done
- ##COMPOSES-WAL-FOR-THE-SESSION-END-COMMIT End-of-session WAL rewrite (from `flow:wal`) ends in a commit;
  git-atomic-commits is how that commit is shaped. @impl/done

$ sed -n '20p' vibevm/vibepacks/org.vibevm.world/git-atomic-commits/v0.1.0/README.md
##package-contents-lead This package ships the **atomicity** discipline (the message **format** is the separate
`flow:git-conventional-commits` package):

$ sed -n '23,30p' vibevm/vibepacks/org.vibevm.world/git-atomic-commits/v0.1.0/vibevm/vibespecs/boot/30-flow-atomic-commits.xml
##COMMIT-MESSAGES-FOLLOW-THE-CONVENTIONAL-COMMITS-FLOW Commit messages follow the **git-conventional-commits** flow — a sibling package:
`spec://org.vibevm.world/git-conventional-commits/flows/conventional-commits/conventional-commits#root`. @impl/done

##CONVENTIONAL-COMMITS-IS-THE-FORMAT-THIS-FLOW-IS-THE-ATOMICITY Conventional Commits is the *format*; this flow is the *atomicity* (one commit, one idea). @impl/done
```

**Who falsifies it:** the document itself — line 20 of the same README says the
message **format** ships in a separate package, and the same package's boot
snippet says it twice more. Not the host; the host's 82 `docs(spec): sync …`
commits in 400 are what makes the *first* half of the sentence true.

**Perimeter searched:** every file of the package, for anything that pins a
message format — `README.md`, `spec/boot/30-flow-atomic-commits.md`,
`spec/flows/atomic-commits/ATOMIC-COMMITS-PROTOCOL.md`,
`spec/flows/atomic-commits/splitting-large-changes.md`. The only format
statements found are the two that **delegate** the format to
`git-conventional-commits`.

**What changed and why:** the fact's first half is true and untouched —
`sync-from-code`'s final step really is a `docs(spec)` commit. The second half
attributed the pinning of that message's *format* to this flow, which this
package disclaims in two other places. The sentence now says what this flow
actually contributes to that composition (the sync is its own commit — the
atomicity) and names the sibling that owns the format, matching the boot
snippet's own division of labour word for word (*format* vs *atomicity*).
`flow:` short names were used because the surrounding bullets already use them;
no relative link was added. Marker unchanged.

**New obligations noticed:** none.

---

## F-226 — the same-commit doc rule is sound and the host keeps it 3 times in 36; the measurement reproduces exactly and it is about the host, not the package

**Outcome:** OUT-OF-ROUTE — §3.6 **route (b)**: the rule is sound, the consumer
does not keep it, the package does not move. A **host** obligation is recorded
below; the registry row belongs at `status: deferred` naming it.
**Anchors:** 0 edited of 2 —
`vibevm/vibepacks/org.vibevm.world/dev-runtime-docs/v0.1.0/vibevm/vibespecs/boot/58-flow-dev-runtime-docs.xml#EVERY-SETUP-TOUCHING-CHANGE-UPDATES-THE-DOC-IN-THE-SAME-COMMIT`,
`vibevm/vibepacks/org.vibevm.world/dev-runtime-docs/v0.1.0/vibevm/vibespecs/flows/dev-runtime-docs/DEV-RUNTIME-DOCS-PROTOCOL.xml#EVERY-SETUP-TOUCHING-CHANGE-UPDATES-THE-DOC-IN-THE-SAME-COMMIT`
**Files touched:** none

**Re-verification:** the verdict's Selector A was re-run from scratch and
reproduces to the commit.

```
$ git log --format="%H" be5e0600..HEAD -- rust-toolchain.toml tools/self-check.sh \
      crates/vibe-core/src/settings.rs crates/vibe-publish/src/token.rs | wc -l
36
$ …and of those, the number that also touched DEV-GUIDE.md or RUNTIME-GUIDE.md:
of which also touched a guide: 3

$ git show --stat --format="%h %ad %s" --date=short f0e89db5 | head -8
f0e89db5 2026-07-20 feat(settings): consolidate the settings home behind one chokepoint
 crates/vibe-core/src/settings.rs   | 190 ++++++++++++++++++++++
 …  (10 files; NEITHER guide)

$ git log --format="%h %ad %s" --date=format:'%Y-%m-%d %H:%M' -6 -- DEV-GUIDE.md RUNTIME-GUIDE.md
cb14fe5c 2026-07-26 00:58 fix(vibe-core): there is one per-user home, and one place a credential lives
de373ba9 2026-07-20 19:21 docs(dev-guide): rust-analyzer LSP setup section
e19efec6 2026-07-20 16:28 docs(registry): clarify ~/.vibe/registry.toml accepts any registry, not just local
14e11747 2026-07-20 16:17 docs: point tokens, config, and aiui discovery at the canonical ~/.vibe

$ git log --format="%h %ad %s" --date=format:'%Y-%m-%d %H:%M' -1 f0e89db5
f0e89db5 2026-07-20 15:28 feat(settings): consolidate the settings home behind one chokepoint
$ git log --format="%h %ad %s" --date=format:'%Y-%m-%d %H:%M' -1 8aec7cc9
8aec7cc9 2026-07-20 15:38 feat(registry): machine-global registry config merged project-first
   → the doc updates landed at 16:17 and 16:28: 49 and 50 minutes later, in
     separate commits. The verdict's numbers are exact.
```

**Who falsifies it:** **the host.** Every falsifying artefact is a host commit.
Nothing inside the package's own tree contradicts either anchor: the boot
snippet and the protocol state the same rule in the same words, no third
statement disagrees with them, and neither claims a checker, a compliance rate,
or any mechanism that could be missing. There is nothing here for the package to
be wrong *about*.

**Perimeter searched:** the package's whole tree
(`vibevm/vibepacks/org.vibevm.world/dev-runtime-docs/v0.1.0/` — README, boot snippet,
protocol) for any self-contradiction or unmet promise, and the host history
`be5e0600..HEAD` (2 086 commits) with both of the verdict's selectors plus the
guides' own file history. Also checked for a §3.6 route **(c)** marked
exception on the host side: `DEV-GUIDE.md:7` and `RUNTIME-GUIDE.md:5` both
*restate* the obligation as MUST, and `vibevm/vibespecs/common/PROP-000.xml:291`
(`##SETUP-DOCS-FLOW`) pins it to this flow. **No exception is recorded
anywhere**, so this is (b) and not (c).

**What changed and why:** nothing. This is the exact shape the phase keeps
mis-routing — «the consumer does less than the rule asks». Any edit that made
this fact true of the host would have to weaken «in the SAME commit», which is
the whole content of the flow and the reason `PROP-000` installs it. The
package's own `HABIT-A-PERIODIC-READ-THROUGH` already anticipates the failure
mode («the drift that slipped in … through a change nobody recognised as
setup-touching») and prescribes the remedy, which is further evidence the rule
is written knowing compliance is imperfect — not evidence the rule is wrong.

**New obligations noticed — HOST:** vibevm keeps its own installed
`dev-runtime-docs` obligation at 3/36 by the over-counting selector, i.e. **≤8 %**.
Two remedies exist and neither is in place: no mechanical check ties a
setup-touching path to a guide edit, and no periodic read-through is scheduled
even though the flow names one. This belongs to Phase E or a host task. Recorded,
not fixed, and **not** charged to the package.

---

## F-228 — «never defer the doc update» is the same rule and the same host record; the 49/50-minute pair is the deferral it names

**Outcome:** OUT-OF-ROUTE — §3.6 **route (b)**, same determination as F-226.
**Anchors:** 0 edited of 2 —
`vibevm/vibepacks/org.vibevm.world/dev-runtime-docs/v0.1.0/vibevm/vibespecs/boot/58-flow-dev-runtime-docs.xml#NEVER-DEFER-THE-DOC-UPDATE-TO-A-LATER-COMMIT`,
`vibevm/vibepacks/org.vibevm.world/dev-runtime-docs/v0.1.0/vibevm/vibespecs/flows/dev-runtime-docs/DEV-RUNTIME-DOCS-PROTOCOL.xml#NEVER-DEFER-THE-DOC-UPDATE-TO-LATER`
**Files touched:** none

**Re-verification:**

```
$ git show --stat --format="%h %ad %s" --date=format:'%Y-%m-%d %H:%M' 4dbe1987 | head -10
4dbe1987 2026-07-26 14:09 fix(vibe-core): one config home, the way there is one credential home
 VIBEVM-SPEC.md                                |   2 +-
 campaigns/packages-2026-09/tasks/DRIFT-027.md | 124 +++++++++++
 crates/vibe-cli/src/commands/show/config.rs   |   4 +-
 crates/vibe-core/src/settings.rs              |  33 ++-
 crates/vibe-core/src/user_config.rs           | 256 ++++++++++------------
 crates/vibe-core/src/user_config/tests.rs     | 292 ++++++++++++++++++++++++++
 docs/commands/show.md                         |   4 +-
 7 files changed, 557 insertions(+), 158 deletions(-)
   → changed the config home; updated VIBEVM-SPEC.md and a command doc;
     touched NEITHER DEV-GUIDE.md nor RUNTIME-GUIDE.md. Verdict confirmed.

   (the 49/50-minute deferral pair is reproduced in F-226 above — same evidence)
```

**Who falsifies it:** **the host**, on host commits only. The two anchors are
one rule stated in two lanes of the same package and they agree with each other
verbatim in substance; nothing in the package's tree is false.

**Perimeter searched:** as F-226 — the package's whole tree, plus
`4dbe1987`'s stat and the guides' file history. No marked exception exists on the
host side, so route (c) does not apply.

**What changed and why:** nothing. Editing «never defer» so that a 49-minute
deferral counts as compliance is precisely the reverted-edit shape §6.1 warns
about: a rule broadened until the host's practice fits it.

**New obligations noticed — HOST:** folded into F-226's host obligation; it is
one compliance gap with one remedy, not two.

---

## F-231 — the single-place law is sound; the host restates the attribution rule in four places outside the single place, including inside the sentence that claims it does not

**Outcome:** OUT-OF-ROUTE — §3.6 **route (b)**: the rule is sound, the consumer
does not keep it, the package does not move. A **host** obligation is recorded
below.
**Anchors:** 0 edited of 2 —
`vibevm/vibepacks/org.vibevm.world/git-attribution-policy/v0.1.0/vibevm/vibespecs/flows/attribution-policy/ATTRIBUTION-POLICY.xml#NO-RESTATEMENT-ANYWHERE-ELSE`,
`…/ATTRIBUTION-POLICY.xml#SUM-RUN-IT-HONESTLY`
**Files touched:** none

**Re-verification:**

```
$ sed -n '99p;158p' README.md
Read [`CLAUDE.md`](CLAUDE.md) before your first commit — the four non-negotiable rules (attribution, Conventional Commits, group by meaning, autonomy on routine changes only) apply to every contribution. …
| [`CLAUDE.md`](CLAUDE.md) (and identical `AGENTS.md` / `GEMINI.md`) | AI agents working in the repo | The four non-negotiable rules + memory discipline + boot read-order. |

$ sed -n '15p' .claude/agents/opus5.md
… Never mark any artifact as machine-authored.
   (a standing per-agent instruction; the rule with no reason attached)

$ sed -n '5p' CLAUDE.md
The repository's commit-and-push discipline — human-authored **attribution** (never mark any part of this repository as AI-authored), **Conventional Commits**, **atomicity**, and commit **autonomy** … The rules live in that inline lane, not restated here.
   → the parenthetical IS the restatement, inside the clause asserting there is none.

$ sed -n '101,102p' vibevm/vibepacks/org.vibevm.world/git-attribution-policy/v0.1.0/vibevm/vibespecs/flows/attribution-policy/ATTRIBUTION-POLICY.xml
##NO-RESTATEMENT-ANYWHERE-ELSE No repeating it in the README, no per-directory
reminders, no comments citing it. @impl/done
```

**Who falsifies it:** **the host** — four host files, no package file. Nothing
inside `git-attribution-policy/` contradicts either anchor; the flow's own
«Instructing an agent» section already contemplates an agent-facing instruction
block and requires it «kept inside the single place», which is exactly the
condition `.claude/agents/opus5.md` fails.

**Perimeter searched:** the package's whole tree
(`ATTRIBUTION-POLICY.xml`, `enforcement-checklist.xml`, `spec/boot/55-…`,
`README.md`) for an internal contradiction — none; and the host's `README.md`,
`CLAUDE.md`, `.claude/agents/**` for restatements — four found. Checked for a
route **(c)** marked exception: `CLAUDE.md:5` does the opposite of recording one,
asserting compliance («not restated here») while restating.

**What changed and why:** nothing. Editing `NO-RESTATEMENT-ANYWHERE-ELSE` to
permit «a README mention and a per-agent reminder» is the textbook §6.1 revert
shape — a rule broadened until the host's practice fits it — and it would
destroy the flow's stated structural reason (every restatement is itself a
mention of AI tooling in the repository). `SUM-RUN-IT-HONESTLY` falls the same
way: «recorded openly in exactly one place» is falsified only by the host having
more than one place.

**New obligations noticed — HOST:** four restatements of the attribution rule
outside the single always-loaded place: `README.md:99`, `README.md:158`,
`.claude/agents/opus5.md:15`, and `CLAUDE.md:5`'s own parenthetical. Either
remove them and point at the inline lane, or record a deliberate §3.6 route (c)
exception naming each. Note `CLAUDE.md:5` is the interesting one: it already
*claims* the single-place posture, so the fix there is to delete a parenthetical,
not to change policy. Recorded, not fixed — and `CLAUDE.md` is owner-sensitive,
so it stops for the owner either way.

---

## F-122 — the length and case rules are sound and unchecked; the host breaks both routinely, and the «never omit the type» half is kept

**Outcome:** OUT-OF-ROUTE — §3.6 **route (b)**: the rule is sound, the consumer
does not keep it, and nothing in this tree checks it. A **host** obligation is
recorded below.
**Anchors:** 0 edited of 2 —
`vibevm/vibepacks/org.vibevm.world/git-conventional-commits/v0.1.0/vibevm/vibespecs/boot/31-flow-conventional-commits.xml#HEADER-SUBJECT-LENGTH-MOOD-AND-CASE`,
`…/31-flow-conventional-commits.xml#NEVER-CAPITALISE-OR-OMIT-THE-TYPE`
**Files touched:** none

**Re-verification:** re-measured today over the last 400 commits (the wave-1
harvest measured its own 400-commit window and got 82 / 42; the window has moved,
the direction has not).

```
$ git log -400 --format="%s" | awk '{ if (length($0) > 72) n++ } END { print n+0 }'
123                       # subjects over the hard limit of 72 — 30.8 %

$ git log -400 --format="%s" | grep -cE '^[a-z]+(\([^)]*\))?!?: [A-Z]'
62                        # capitalised first word after the prefix — 15.5 %

$ git log -400 --format="%s" | grep -oE '^[a-z]+(\([^)]*\))?!?: [A-Z][a-zA-Z]*' | sed -E 's/^.*: //' | sort | uniq -c | sort -rn
     24 W        13 C        7 Phase        6 F        5 B        4 DRIFT        1 PHASE        1 NOTOUCH
   → every one is a campaign identifier (wave / phase / finding id), not a
     sentence-capitalised word. The rule as written forbids them all.

$ git log --format="%h %p %s" | grep -vE '^[0-9a-f]+ [0-9a-f ]* [a-zA-Z]+(\([^)]*\))?!?: '
8628f1cb 724a3368 5466215d Merge branch 'new': the Discipline terraform — complete
8ed8e222 e4ff1f12 074a66e2 Merge branch 'm1.17-workspace' — M1.18 Loading model
   → 2 typeless subjects in the whole history, both two-parent merges.
     The «never omit the type» half is KEPT.
```

**Who falsifies it:** **the host**, entirely — its commit subjects and the
absence of any checker. The package's own tree is internally consistent: the
boot snippet, the protocol and the README state the same numbers, and no file in
the package claims a checker exists.

**Perimeter searched:** for an enforcement point, the wave-1 perimeter was
re-checked and holds — no `.github/` directory at all, no non-sample hook in
`.git/hooks`, no commitlint / husky / commit-msg config outside vendored
third-party clones, no hook key in `.claude/settings*.json`, and none of
`tools/self-check.sh`'s ten invariants reads a commit message. For the package
side: its whole tree, for any statement that a checker exists — none.

**What changed and why:** nothing. Both edits the evidence invites are the
forbidden shape. Raising or dropping the 60/72 limit makes the rule admit 123
subjects the host happens to have written; carving campaign identifiers out of
«never capitalise» makes it admit the other 62. The verdict itself notes the
distinction a real checker would have to make — «the check that would enforce
this rule has to know» an identifier from a capitalised word — which is an
argument for building the checker (Phase E), not for weakening the rule.

**New obligations noticed — HOST:** two, and they are separable.
(1) **No commit-message checker exists in this tree** for any rule of this flow;
every rule is unchecked, which is why the drift is invisible. Phase E.
(2) **The campaign-identifier case.** 62 of the last 400 subjects open with a
wave/phase/finding id (`W3`, `C`, `Phase`, `F-…`, `DRIFT-…`). Under the rule as
written these are violations. This is the one place a §3.6 route **(c)** ruling
would be legitimate — a *host-side* recorded exception saying campaign
identifiers keep their case — and it is an owner call, not a boss one. Recorded,
not fixed, and explicitly **not** written into the package.

---

## F-274 — the campaign note points at a `vibevm/vibespecs/terraforms/` this package does not have; its `spec/` holds `tools/` alone

**Outcome:** EDITED
**Anchors:** 1 edited of 1 — `vibevm/vibepacks/org.vibevm.ai-native/go-ai-native-mcp/v0.1.0/README.md#campaign-in-flight-note`
**Files touched:** `vibevm/vibepacks/org.vibevm.ai-native/go-ai-native-mcp/v0.1.0/README.md`

**Re-verification:**

```
$ sed -n '12,15p' vibevm/vibepacks/org.vibevm.ai-native/go-ai-native-mcp/v0.1.0/README.md
> ##campaign-in-flight-note Campaign in flight: built end-to-end per
> GO-AI-NATIVE-PLAN v0.1, the campaign plan, which is authored outside
> this package; the server brief is `spec/tools/discipline-mcp-go.md`.
> This README is finalized at campaign close. @spec/done

$ ls -R vibevm/vibepacks/org.vibevm.ai-native/go-ai-native-mcp/v0.1.0/vibevm/vibespecs/
spec/:        tools
spec/tools:   discipline-mcp-go.xml
   → no `terraforms/`; the second half of the note (the server brief) resolves.

$ find . -name "GO-AI-NATIVE-PLAN-v0.1.md" -not -path "./target/*"
./legacy-spec/terraforms/GO-AI-NATIVE-PLAN-v0.1.md
   → one copy in the tree, in the host, outside every package.

$ grep -rn "GO-AI-NATIVE-PLAN" vibevm/vibepacks/org.vibevm.ai-native/ --include=*.md
go-ai-native-lang/v0.1.0/spec/cards/INDEX.md:56:            GO-AI-NATIVE-PLAN campaign)
go-ai-native-lang/v0.1.0/spec/go/mechanisms/TCG-ORACLE-GO-v0.1.md:5:  GO-AI-NATIVE-PLAN v0.1 (Phase 3)
go-ai-native-lang/v0.1.0/spec/go/mechanisms/TCG-PROTOCOL-GO-v0.1.md:5:  GO-AI-NATIVE-PLAN v0.1 (Phase 3)
go-ai-native-lang/v0.1.0/spec/go/tools/conform-frontend-go.md:5:      GO-AI-NATIVE-PLAN Phases 4–5
go-ai-native-lang/v0.1.0/spec/go/tools/vibe-agentic-tcg-go.md:9:      the GO-AI-NATIVE-PLAN mandate
   → five sibling citations, all BY NAME, none by path. The repair adopts the
     family's existing convention rather than inventing one.
```

**Who falsifies it:** the package's own tree — `spec/` contains exactly one
directory, `tools/`, so the address is false about the package itself. §3.6
route (a), `falsifier` would read `self` on the strict reading even though the
row says `mixed`.

**Perimeter searched:** the package's own tree
(`ls -R vibevm/vibepacks/org.vibevm.ai-native/go-ai-native-mcp/v0.1.0/vibevm/vibespecs/`) and the
whole repository for the plan file (`find . -name "GO-AI-NATIVE-PLAN-v0.1.md"
-not -path "./target/*"` → one hit, in the host).

**What changed and why:** the note claimed a package-relative path for a
document the package does not ship. Writing the host path instead
(`legacy-spec/terraforms/…`) was considered and rejected: it resolves in this dev
tree and in nothing a consumer installs, which is the same defect in a different
direction. Every one of the five sibling citations of this plan across
`go-ai-native-lang` names it **by name only** — so the note now does too, and
says plainly that the plan is authored outside the package. The half that was
true (`spec/tools/discipline-mcp-go.md`) is unchanged and still resolves. Marker
stays `@spec/done`.

**New obligations noticed:** none.

---

## F-276 — 74.8 % is not the figure the corpus's own findings register holds; the register says 75.3 % / 70.2 %

**Outcome:** EDITED
**Anchors:** 1 edited of 1 — `vibevm/vibepacks/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/vibevm/vibespecs/rust/tools/rust-ai-native-tcg.xml#RISK-TRANSFER-UNPROVEN`
**Files touched:** `vibevm/vibepacks/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/vibevm/vibespecs/rust/tools/rust-ai-native-tcg.xml`

**Re-verification:**

```
$ sed -n '78p' vibevm/vibepacks/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/vibevm/vibespecs/rust/tools/rust-ai-native-tcg.xml
- ##RISK-TRANSFER-UNPROVEN **Transfer unproven:** the PLDI'25 reduction — 75.3% on synthesis and 70.2% on translation (DR2-012) — is TypeScript. Rust's richer types may yield smaller gains, or the per-completion analyzer latency may make Stage 3 impractical for interactive generation. Measure at Stage 2 before committing to Stage 3. @spec/done

$ grep -n "DR2-012" vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/vibevm/vibespecs/appendix/ATLAS.xml
##FINDING-DR2-012 **DR2-012** — Type-constrained decoding cuts compile errors ~75%, lifts correctness
  … reduces compilation errors by 75.3% (synthesis) and 70.2% (translation) …

$ grep -n "DR2-012" vibevm/vibepacks/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/vibevm/vibespecs/rust/tools/rust-ai-native-tcg.xml
11:  … DR2-012/DR1-014 (the 74.8% compile-error reduction in TypeScript) …   ← F-216, sync-from-code, NOT this obligation
78:  … (DR2-012) …                                                          ← this edit
81:  … (The CITYWALK false-positive trap, DR2-012 caveat.)
   (the finding id is already this file's own vocabulary, so nothing new was introduced)
```

**Who falsifies it:** a shipped sibling package — `core-ai-native`'s
`spec/appendix/ATLAS.md`, the family's findings register, which is the document
this brief's own §1 cites as its evidence base.

**Perimeter searched:** every `74.8` in the ai-native tree —
`grep -n "74.8\|74,8" -r vibevm/vibepacks/org.vibevm.ai-native/ --include=*.md` → **12
sites in 5 packages**, and **not one of them is in the findings register**. The
register carries 75.3 / 70.2 and its own title says «~75 %».

**What changed and why:** only the figure and its attribution. The risk the
sentence states — that a TypeScript result may not transfer to Rust, and that
Stage 3 must be measured before it is committed to — is untouched, because
nothing falsified it. The number now matches `FINDING-DR2-012` verbatim and
names the finding, so the next reader can check it in one grep. **Only this one
anchor moved**, deliberately: the same 74.8 % sits on three different closure
routes, and §6.1 `##ROUTE-BEFORE-FALSIFIER` is precisely the rule that says a
batch is cut by route first.

**New obligations noticed:** the 74.8 % family, and it is bigger than this row.

1. **The two appendices of one package disagree.** `ATLAS.xml` says 75.3 / 70.2;
   `CONTRADICTION-MAP.xml`'s C-4 publishes «74.8%» in the **section title**
   (`#c-4-tcg-without-rust`) and again in `##C-4-RESOLUTION`. Neither C-4 anchor
   carries any obligation — a query over `run/state/obligations.json` for
   `CONTRADICTION-MAP` anchors returns only `#ENTRY-CARRIES-FOUR-PARTS` (F-121)
   and `#open-items-are-why-the-package-is-beta` (F-262). This is very likely
   where every downstream 74.8 % was copied from, and it is unclaimed.
2. **Nine more sites carry the figure and sit on other routes or none:**
   `rust-ai-native-tcg.xml:11` and `:65` (`:11` is **F-216, sync-from-code —
   owner route**; `:65` `##STAGE-3-TYPE-VALID-CONTINUATIONS` has no obligation),
   `go-ai-native-tcg.xml:31` (none), `typescript-ai-native-tcg.xml:38` and `:104`
   (none), `GUIDE-AI-NATIVE-TYPESCRIPT.xml:39` (F-168, build-or-demote), `:258`
   (F-161, sync-from-code), `:270` (none), and the `core-ai-native/v0.7.0`
   predecessor slot. A figure this widely copied with one authoritative source
   is the `duplication` shape the registry mints on the next run.

---

## F-314 — the same-commit licence rule is sound; the host's one relicense touched one file and README.md still contradicts LICENSE.xml seventeen days on

**Outcome:** OUT-OF-ROUTE — §3.6 **route (b)**: the rule is sound, the consumer
did not keep it. A **host** obligation is recorded below.
**Anchors:** 0 edited of 1 —
`vibevm/vibepacks/org.vibevm.world/licensing/v0.1.0/vibevm/vibespecs/boot/60-flow-licensing.xml#WHEN-A-CHANGE-TOUCHES-ANY-LICENCE-STATEMENT`
**Files touched:** none

**Re-verification:**

```
$ sed -n '38,40p' vibevm/vibepacks/org.vibevm.world/licensing/v0.1.0/vibevm/vibespecs/boot/60-flow-licensing.xml
- ##WHEN-A-CHANGE-TOUCHES-ANY-LICENCE-STATEMENT A change touching the licence file, the manifest `license` field,
  or the third-party carve-out updates all of them together, in one
  commit. @impl/done

$ git show --stat --format="%h %ad %s" --date=short 5086c5b5 | head -5
5086c5b5 2026-07-12 chore(license): relicense vibevm to UPL-1.0
 LICENSE.xml | 65 ++++++++++++++++++++++++++++++++++++++++++--------------------
 1 file changed, 44 insertions(+), 21 deletions(-)

$ git log -S'UPL-1.0' --format="%h %ad %s" --date=short -- vibevm/vibespecs/common/PROP-000.xml | head -2
71d8383b 2026-07-25 docs(spec): Phase D d1b — the foundation catches up with reality
   → thirteen days later, in a separate commit.

$ sed -n '3p' LICENSE.xml
The Universal Permissive License (UPL), Version 1.0
$ grep -n "proprietary EULA placeholder" README.md
164:vibevm itself ships under the proprietary EULA placeholder in [`LICENSE.xml`](LICENSE.xml) for the moment; the eventual target is UPL 1.0. …
   → live today, 2026-07-29: a sentence contradicting the file it links to,
     seventeen days after that file changed.
```

**Who falsifies it:** **the host**, on host files and host commits. Nothing
inside `licensing/v0.1.0/` disagrees with the rule; the flow's own
`WHEN-CHANGING-THE-PRODUCTS-LICENCE` (an owner decision, never autonomous) was
kept — the relicense was an owner act.

**Perimeter searched:** the package tree (boot snippet + `LICENSING-PROTOCOL.xml`)
for an internal contradiction — none. Host side: `LICENSE.xml`, `README.md`,
`VIBEVM-SPEC.md`, `vibevm/vibespecs/common/PROP-000.xml`, and the CLAUDE.md carve-out ledger.
The ledger at `CLAUDE.md:132-137` enumerates the deliberately-stale `"EULA"`
strings — `refs/**`, `vibedeps/**`, `.vibe/cache/**`, `fixtures/**`, `crates/**`
test data, the licensing package, `VIBEVM-SPEC.md` and specs. **`README.md` is
on none of them**, while `VIBEVM-SPEC.md`, which says the same stale thing, IS.
So this is not a §3.6 route (c) marked exception; the ledger draws the line
correctly and the README was simply missed.

**What changed and why:** nothing. Weakening «updates all of them together, in
one commit» so that a thirteen-day lag and a seventeen-day-and-counting
omission count as compliance is the reverted-edit shape exactly.

**New obligations noticed — HOST:** `README.md:164` is stale and provably so —
it names a proprietary EULA placeholder and links to a file whose third line
reads «The Universal Permissive License (UPL), Version 1.0». One-line fix, and
the carve-out ledger already says it is not exempt. Recorded, not fixed.

---

## F-325 — the verdict's «the fix was available» is false: PROP-009 v1 has exactly one `when` probe (`os:`), and the static lane is the host's own choice

**Outcome:** RE-JUDGE: confirmed
**Anchors:** 0 edited of 1 —
`vibevm/vibepacks/org.vibevm.world/qualified-naming/v0.1.0/README.md#IT-IS-A-DESIGN-TIME-DISCIPLINE-READ-ONCE`
**Files touched:** none

**Re-verification:**

```
$ sed -n '99p;106p;122p' vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml
- ##LINK-DYNAMIC `link = "dynamic"` — **the default.** … An optional `when` condition gates the read … **v1 implements the `os:` probe end-to-end** — `when = "os:windows"` matches the session's operating system (`windows` / `macos` / `linux`); the remaining probes are reserved until PROP-003's activation engine is built. @impl/done
##WHEN-FORCES-DYNAMIC A `[boot_snippet]` that declares a `when` condition (§2.6) stays a conditional `dynamic` entry … OS-specific content must never reach a session on the wrong OS. @impl/done
- ##SCHEMA-BOOT-SNIPPET … For v1 the only `when` is an operating-system match, the wire string `"os:<name>"` with `<name>` one of `windows` / `macos` / `linux` …

$ cat vibevm/vibepacks/org.vibevm.world/qualified-naming/v0.1.0/vibe.toml   # the [boot_snippet] table, in full
[boot_snippet]
source = "spec/boot/67-flow-qualified-naming.md"
category = "flow"
   → no `link` either, so PROP-009's default for this package is `dynamic`.

$ grep -n "qualified-naming" vibe.toml        # the HOST manifest
   (exit 1 — not a direct dependency at all)
$ sed -n '28p' vibe.toml
"flow:org.vibevm.world/redbook" = { version = "^0.2.0", link = "static-transitive" }
   → the snippet reaches STATIC.md transitively through the redbook collection,
     under a link mode the HOST chose.

$ sed -n '13,20p' vibevm/vibepacks/org.vibevm.world/qualified-naming/v0.1.0/vibevm/vibespecs/boot/67-flow-qualified-naming.xml
## When this applies {#when}

##READ-THE-PROTOCOL-BEFORE-THE-FIRST-NAME-IS-MINTED When you design any user-facing namespace — a package registry, a
plugin id scheme, an artifact coordinate, an extension marketplace —
read [`QUALIFIED-NAMING-PROTOCOL.xml`](../flows/qualified-naming/QUALIFIED-NAMING-PROTOCOL.md)
**before the first name is minted**. @impl/done
   → the snippet is a ~20-line TRIGGER. The discipline is the protocol it
     points at, and that is what «read once» is about.
```

**Who falsifies it:** nobody. Three independent grounds, none of which needs the
host to change and none of which the package could have avoided.

**Perimeter searched:** for a mechanism that could have made the snippet
conditional — `vibevm/vibespecs/modules/vibe-workspace/PROP-009-loading-model.xml` (§2.3,
§2.4, §2.6 and the §history entry) and `vibevm/vibespecs/modules/vibe-resolver/PROP-003-dep-evolution.xml`
(`##CH-IF-OS`, the shared probe vocabulary). For who chose the lane — the
package's `vibe.toml` and the host's `vibe.toml`.

**What changed and why:** nothing, on three grounds.

1. **The subject of the sentence is the protocol, not the snippet.** «This is a
   design-time discipline, read once while shaping identifiers» is about the
   *discipline* — `QUALIFIED-NAMING-PROTOCOL.xml`, which no session reads unless
   it is minting names. What every session reads is a twenty-line trigger whose
   own §«When this applies» says to go read the protocol *before the first name
   is minted*. Reading «the assertion is itself read every session» as
   falsifying the assertion is the same category error as reading a section
   titled «Reading layers» as a priority chain.
2. **The fix the verdict says was available does not exist.** PROP-009 v1
   implements exactly one `when` probe — `os:<name>`, windows/macos/linux — and
   says in three places that «the remaining probes are reserved until PROP-003's
   activation engine is built». No probe can express «this session is shaping
   identifiers». A package cannot decline a mechanism that has not shipped.
3. **The static lane is the consumer's choice, not the package's.** The
   package's `[boot_snippet]` declares no `link`, whose PROP-009 default is
   `dynamic`; it lands in `STATIC.md` because the *host* wrote
   `link = "static-transitive"` on the redbook collection at `vibe.toml:28`.
   And even `dynamic` would not have helped: PROP-009 says a dynamic entry
   without a `when` «is read unconditionally».

**New obligations noticed:** the reserved-probe gap is worth a record of its
own — several packages would benefit from a task-shaped `when` (this one, and
any other design-time flow), and PROP-009 §2.6's own history entry already flags
«the wider probe set follows when PROP-003's activation engine is built». That is
a **host** roadmap item, not a defect in this package. Recorded, not fixed.

---

## F-326 — «the Discipline's internal copies defer to them from their next release»: the next release came, and deferred in neither

**Outcome:** EDITED
**Anchors:** 1 edited of 1 — `vibevm/vibepacks/org.vibevm.world/redbook/v0.2.0/README.md#THE-REDBOOK-PACKAGE-IS-CANONICAL-WHERE-THE-TWO-OVERLAP`
**Files touched:** `vibevm/vibepacks/org.vibevm.world/redbook/v0.2.0/README.md`

**Re-verification:**

```
$ sed -n '99,105p' vibevm/vibepacks/org.vibevm.world/redbook/v0.2.0/README.md
##THE-REDBOOK-PACKAGE-IS-CANONICAL-WHERE-THE-TWO-OVERLAP Where the two describe the same practice, **the redbook package is
canonical**: `flow:wal` is the canonical home of the WAL convention
and `flow:campaign-plans` of the campaign-plan format. That is this
package's position, and the Discipline has not recorded it: as of
core-ai-native 0.8.0 its `05-CAMPAIGN-FORM.xml` and
`06-WAL-CONVENTION.xml` carry no deferral, so the two remain parallel
copies until one lands. @spec/done

$ grep -c "campaign-plans\|redbook\|defer\|superseded" \
      vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/vibevm/vibespecs/05-CAMPAIGN-FORM.xml \
      vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v0.8.0/vibevm/vibespecs/06-WAL-CONVENTION.xml
…/05-CAMPAIGN-FORM.xml:0
…/06-WAL-CONVENTION.xml:0                 # 117 and 104 lines respectively

$ Grep "redbook" over vibevm/vibepacks/org.vibevm.ai-native/
No files found                            # zero across the entire ai-native tree

$ git log --format="%h %ad %s" --date=short -1 bfb72da7
bfb72da7 2026-07-17 feat(ai-native): core-ai-native 0.8.0 — Go support in the neutral engine
$ git log --format="%h %ad %s" --date=short -1 69708287
69708287 2026-07-12 refactor(packages): move the remaining packages to org.vibevm.world
   → the «next release» landed FIVE DAYS after wal 0.2.0 and campaign-plans
     reached org.vibevm.world, and deferred to neither.

$ sed -n '108,110p' vibe.lock
name = "core-ai-native"
group = "org.vibevm.ai-native"
version = "0.8.0"                         # v0.8.0 is the slot the host installs
```

**Who falsifies it:** a **shipped sibling package** — `core-ai-native` v0.8.0,
whose two overlapping documents record no deferral and whose whole tree does not
contain the word `redbook`. Not the host.

**Perimeter searched:** both overlapping documents in the installed
`core-ai-native/v0.8.0` slot for `campaign-plans` / `redbook` / `defer` /
`superseded` (0 hits in 221 lines), and the entire
`vibevm/vibepacks/org.vibevm.ai-native/` tree for `redbook` (no files). `vibe.lock`
confirms v0.8.0 is the installed slot, so the check is against what a consumer
actually reads.

**What changed and why:** the claim had two parts and only one was false. The
canonicity **position** — redbook is canonical where the two overlap — is this
package's to state and is untouched. The **prediction** — «the Discipline's
internal copies defer to them from their next release» — was falsifiable and has
been falsified: the release landed 2026-07-17, five days later, deferring in
neither document. The prediction is replaced by the measurement, and the
sentence now says plainly that the other side has not recorded the position, so
a reader of either package learns the same thing. Marker stays `@spec/done`: the
statement is a position, not an implementation.

**New obligations noticed:** the mirror of this is a **`core-ai-native`**
obligation — if the canonicity ruling stands, `05-CAMPAIGN-FORM.xml` and
`06-WAL-CONVENTION.xml` should carry the deferral, and that is a cross-package
change in a package four language families copy, i.e. a §4.5 **release event**,
not a prose edit. It is not this obligation's to make. Recorded, not fixed.

---

## F-329 — «enforced by mechanical scans» over-claims for both halves, and both packages say so themselves

**Outcome:** EDITED
**Anchors:** 1 edited of 1 — `vibevm/vibepacks/org.vibevm.world/secrets-hygiene/v0.1.0/README.md#COMPOSES-ATTRIBUTION-POLICY`
**Files touched:** `vibevm/vibepacks/org.vibevm.world/secrets-hygiene/v0.1.0/README.md`

**Re-verification:**

```
$ sed -n '65,70p' vibevm/vibepacks/org.vibevm.world/secrets-hygiene/v0.1.0/README.md
- ##COMPOSES-ATTRIBUTION-POLICY `flow:git-attribution-policy` (`55-…`) is the sibling policy package:
  both are one-place policies, mechanised where mechanisation reaches
  and reviewed everywhere else — the sibling scans two of its eight
  surfaces before push, this one backs Law 4's redaction with a unit
  test. One keeps authorship marks off every surface, this one keeps
  secret values off every surface. @impl/done

$ grep -n "SUM-EIGHT-SURFACES-TWO-OF-THEM-MECHANICAL" -A2 \
      vibevm/vibepacks/org.vibevm.world/git-attribution-policy/v0.1.0/vibevm/vibespecs/flows/attribution-policy/enforcement-checklist.xml
- ##SUM-EIGHT-SURFACES-TWO-OF-THEM-MECHANICAL Eight surfaces; two of them (messages, trailers) get a mechanical
  pre-push scan, the rest ride templates, tool configuration, and
  the periodic audit. @impl/done
   (the checklist's §surfaces table lists all eight; the pre-push scan is a
    concrete `git log … | grep -inE …` block, so the sibling DOES ship a scan
    — for two surfaces, and it says so.)

$ grep -n "^### Law 4\|THE-REDACTION-IS-BACKED-BY-A-UNIT-TEST\|EVERY-NEW-CODE-PATH-IS-REVIEWED" \
      vibevm/vibepacks/org.vibevm.world/secrets-hygiene/v0.1.0/vibevm/vibespecs/flows/secrets-hygiene/SECRETS-HYGIENE-PROTOCOL.xml
107:### Law 4 — redaction is tested, not promised {#law-tested}
113:##THE-REDACTION-IS-BACKED-BY-A-UNIT-TEST That redaction is **backed by a unit test** …
120:##EVERY-NEW-CODE-PATH-IS-REVIEWED-AGAINST-THE-FOUR-LAWS Every new code path that touches a secret is reviewed against these …

$ grep -rn "scan\|mechanical\|checker" vibevm/vibepacks/org.vibevm.world/secrets-hygiene/v0.1.0/vibevm/vibespecs/ --include=*.md
SECRETS-HYGIENE-PROTOCOL.xml:147: … the response is mechanical:      ← the accidental-read drill
   → exactly one hit, and it is about incident response. This package ships
     NO scan of its own; its enforcement is a unit test plus review.
```

**Who falsifies it:** both packages, from inside their own trees — the sibling's
`enforcement-checklist.xml` summary («two of them … the rest ride templates, tool
configuration, and the periodic audit») and this package's own protocol, whose
Law 4 is a unit test and whose closing rule is review. The host's 11 `redact`
functions were not needed to decide it.

**Perimeter searched:** the whole `secrets-hygiene` package `spec/` tree for any
scan/checker/mechanical statement (one hit, unrelated), and the sibling's
`enforcement-checklist.xml` in full — the §surfaces table, the §pre-push scan, and
the §summary.

**What changed and why:** «enforced by mechanical scans» asserted symmetric
mechanisation for two policies that are each honest about being partly
mechanised. The composition's real point — two one-place policies of the same
shape, one for authorship marks and one for secret values — is preserved word for
word; what was added is the accurate description of how far mechanisation
reaches on each side, taken verbatim from each package's own summary. Marker
unchanged.

**New obligations noticed:** the sibling's boot snippet is named
`55-flow-attribution-policy.xml` while its package directory is
`git-attribution-policy` — a filename/package-name mismatch, recorded by the
Phase C verdict as a separate fact and still true. It is a fact about the
sibling's filename, not about this rule; not fixed here.

---

## F-331 — the fan-out rule is sound; the host instructs both the rule and its violation, in one file, and has pushed directly at least 130 times

**Outcome:** OUT-OF-ROUTE — §3.6 **route (b)**, and it needs an **owner**: the
contradiction is in the host's own boot contract, so the fix is a host policy
decision, not a boss edit.
**Anchors:** 0 edited of 1 —
`vibevm/vibepacks/org.vibevm.world/source-mirrors/v0.1.0/vibevm/vibespecs/boot/62-flow-source-mirrors.xml#NEVER-PUSH-DIRECTLY-TO-A-REPLICA-HOST`
**Files touched:** none

**Re-verification:**

```
$ git reflog show refs/remotes/origin/main --format='%gs' | sort | uniq -c | sort -rn | head -2
    258
     69 update by push
$ git reflog show refs/remotes/github/main --format='%gs' | sort | uniq -c | sort -rn | head -2
    307
     61 update by push
   → 130 direct pushes in the visible reflog window, a floor (older entries
     expired). The method holds: `git push <named-remote>` writes
     «update by push»; the fan-out's raw-URL push updates no tracking ref and
     its `refresh_tracking` moves it with `git update-ref` and no `-m`,
     leaving an empty message. The two message kinds separate the two paths.

--- the host instructing the violation ---
$ sed -n '191p' CLAUDE.md
4. **Push to `origin/main`** — routine per Rule 4, since the user invoked the wind-down explicitly. …
$ sed -n '34p' vibevm/vibespecs/boot/90-user.xml
- ##CMD-ROUTINE-PUSH Routine push to GitVerse: `git push origin main`. …

--- the host instructing the rule, in the same file ---
$ sed -n '13p;35p' vibevm/vibespecs/boot/90-user.xml
- ##SRC-MULTI-HOMED … Roll a change out to both with `cargo xtask mirror` … NOT `git push origin` (which only hits GitVerse). …
- ##CMD-MIRROR … `cargo xtask mirror` … This is the standard rollout, preferred over a bare `git push origin`. …
$ sed -n '15p' vibevm/vibespecs/common/PROP-016-source-mirrors.xml
- ##HOST-GITVERSE **GitVerse** … `origin` on the maintainer's machine points here — a single-host convenience remote; fan-out is the manifest, not `git push origin`. @spec/done
```

**Who falsifies it:** **the host**, twice over — in its reflogs and in its own
written contract. `vibevm/vibespecs/boot/90-user.xml` says both things: line 13 forbids
`git push origin`, line 34 prescribes it as routine. Nothing in
`source-mirrors/v0.1.0/` disagrees with itself; its `#never` list is internally
consistent (`NEVER-PUSH-DIRECTLY-TO-A-REPLICA-HOST`,
`NEVER-FORCE-ANY-TARGET`, `NEVER-RESOLVE-A-DIVERGENCE-BY-CLOBBERING`).

**Perimeter searched:** the package tree for a self-contradiction — none; the
host for both sides of the instruction — `CLAUDE.md:191`, `vibevm/vibespecs/boot/90-user.xml`
lines 13 / 34 / 35, `vibevm/vibespecs/common/PROP-016-source-mirrors.xml:15`; and both
tracking reflogs for the push-path signature.

**What changed and why:** nothing. This is the clearest route (b) in the batch
and the only one whose resolution is not a package matter at all: the host must
decide whether `git push origin main` is routine (and then `origin` is not a
replica host, and PROP-016 and `90-user.xml:13` need correcting) or whether
`cargo xtask mirror` is the only rollout (and then `CLAUDE.md:191` and
`90-user.xml:34` need correcting). Either way the decision changes `CLAUDE.md`,
which is owner-sensitive, and the flow's rule is unaffected by whichever way it
goes.

**New obligations noticed — HOST, for the owner:** `vibevm/vibespecs/boot/90-user.xml`
contradicts itself at lines 13 and 34 about whether `git push origin main` is
permitted, and `CLAUDE.md:191` makes the forbidden form step 4 of the END
SESSION contract. This is a genuine policy fork, not an editorial slip; it is
stated here for the owner and **not** decided. Recorded, not fixed.
