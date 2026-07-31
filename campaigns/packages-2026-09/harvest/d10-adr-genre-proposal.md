# D10 — do the specs owe ADRs, and in what form? A proposal, not a ruling

_Phase D, batch D10. **This file answers nothing.** [`BACKLOG.md` #b-007](../../../BACKLOG.md#b-007)
is filed «as a question to answer rather than work to schedule», and
##B007-WHY-IT-IS-A-QUESTION-NOT-A-TASK puts the genre decision on the owner's
desk. This document produces the decision material — a decidable criterion, a
sized estimate, a recommended form, three costed options — and marks its own
preference as the campaign's recommendation. **The ruling is the owner's.**_

**No file was edited but this one.** No spec file, no package file, no campaign
state, no verdict JSON, no `merge-verdicts.py`, no `vibe progress seal`, no git
write. `git` was run read-only (`rev-parse`, `log`, `status`).

**Measured at** `HEAD = 91ebf1fd` (`docs(campaign): the rulings session in the
LOG — four forms ruled, six obligations closed, the verdict-first rule's first
live test`, 2026-07-31). The working tree at batch start carried eleven
modifications that are **not** this batch's and were not touched
(`AGENTS.md`, `BACKLOG.md`, `CLAUDE.md`, `GEMINI.md`,
`campaigns/packages-2026-09/PHASE-D-SYNC-QUEUE.md`, `run/cache.json`,
`run/state/routing.json`, and four `core-ai-native` spec files).

**HEAD advanced to `fffcb494` while this batch ran** (three commits: the
2026-07-31 rulings, the wind-down mirror rollout, and an `ai-native` closing-rule
fix). **Every figure below was re-verified at `fffcb494` and every one holds** —
`154 → 4`, `158 → 7`, the fractality `33 → 14`, and the twelve routed anchors
are unchanged, because the two files that moved (`BACKLOG.md` ##B009-DISPOSITION
`open → done`, and one new `F-220` routing entry for `#COMPOSES-WAL`) touch
nothing this file measures. Recorded rather than silently re-based, per
##THE-CAMPAIGN-IS-INSIDE-ITS-OWN-CORPUS: *«any figure over `git log` names the
HEAD it was taken at»*.

**One figure did move, and it moved because of this file.** Re-measured at
`fffcb494`, `campaigns/**` reads **31 → 12** where §3.1 reports 17 → 10. The
difference is **this document and its sibling `d10-campaign-plans-forms.md`**:
this file alone contributes **14 Decision labels and 2 complete records**, so
writing a proposal about decision records nearly doubled the campaign's own
measured adoption of them. §3.1's table therefore reports `campaigns/**`
**excluding `harvest/d10-*`**, and says so in the row. The trap named at
##THE-CAMPAIGN-IS-INSIDE-ITS-OWN-CORPUS has no sharper instance than a document
that changes the metric it is arguing about by existing.

**Every count below names the command that produced it**, per
[`##THE-CAMPAIGN-IS-INSIDE-ITS-OWN-CORPUS`](../PHASE-D-BATCH-PLAN.md#delegation-lessons).
Two perimeter rules bind every figure in this file:

- **`campaigns/**` is broken out as its own row, never folded into a host
  total** — §6.1's rule. `campaigns/*/run/**` contributes nothing to any
  practice count here (it happens to contribute nothing at all: measured, `0`
  of the `campaigns/` hits are under `run/`).
- **`vibedeps/**` and `.vibe/cache/**` are broken out as their own row.** This
  rule is not in §6.1 and it should be — [§0](#zero) is the whole reason.

**What was read to write this**, in order:

1. [`BACKLOG.md` #b-007](../../../BACKLOG.md#b-007) in full — the question, the
   measurements, «what it unblocks».
2. `packages/org.vibevm.world/decision-records/v0.1.0/spec/` in full: the boot
   snippet [`25-flow-decision-records.md`](../../../packages/org.vibevm.world/decision-records/v0.1.0/spec/boot/25-flow-decision-records.md),
   [`DECISION-RECORDS-PROTOCOL.md`](../../../packages/org.vibevm.world/decision-records/v0.1.0/spec/flows/decision-records/DECISION-RECORDS-PROTOCOL.md),
   [`record-template.md`](../../../packages/org.vibevm.world/decision-records/v0.1.0/spec/flows/decision-records/record-template.md),
   [`revisit-triggers.md`](../../../packages/org.vibevm.world/decision-records/v0.1.0/spec/flows/decision-records/revisit-triggers.md).
3. The genre map: `spec-genres`'
   [`SPEC-GENRES-PROTOCOL.md#genres`](../../../packages/org.vibevm.world/spec-genres/v0.1.0/spec/flows/spec-genres/SPEC-GENRES-PROTOCOL.md)
   and the host's instance, [`spec/design/README.md`](../../../spec/design/README.md)
   `##genre-table-lead`.
4. The living practice: the `fractality` specspace's own decision blocks; this
   campaign's [`PHASE-D-BATCH-PLAN.md` §3](../PHASE-D-BATCH-PLAN.md#decisions).
5. [`PHASE-D-HOST-OBLIGATIONS.md` §census](../PHASE-D-HOST-OBLIGATIONS.md#census)
   — the re-measure that reframed the question, and
   [`run/state/routing.json`](../run/state/routing.json) for what rides on it.

**The instrument.** Every census figure comes from one script, written for this
batch and named in each command below. It splits a Markdown file into heading
sections (fence-aware), finds every section carrying a bolded `Decision` label,
and tests the same section for the other three fields — `**Why…**`,
`**Considered and rejected…**` / `**Rejected alternatives…**`, and
`**When to revisit…**` / `**Revisit when…**` / `**Revisit…**`. It lives in this
session's scratchpad, not in the repository, because a one-batch measuring stick
is not a project artefact; the commands below reproduce every number from it,
and §3 states its two known biases.

---

## Contents

- [§0 — The correction that changes the question](#zero)
- [§1 — The criterion: what makes a Decision «genuinely reopenable»](#criterion)
- [§2 — Six worked examples, three each way](#examples)
- [§3 — The sizing](#sizing)
- [§4 — The form](#form)
- [§5 — Three options and one variant, costed](#options)
- [§6 — The campaign's recommendation](#recommendation)

---

## §0 — The correction that changes the question {#zero}

**The census's most load-bearing figure is an artefact of the perimeter, and
removing it removes the premise the current framing rests on.**

[`PHASE-D-HOST-OBLIGATIONS.md` §census](../PHASE-D-HOST-OBLIGATIONS.md#census)
re-measured on 2026-07-31 and concluded: *«The practice is adopted, and adopted
well, in the sibling project: 14 of 34, about 41 %»* — and on that basis
reframed B-007 from «whether to adopt» to *«why the PROP tree is the outlier»*.
`BACKLOG.md` ##B007-WHAT-IS-MEASURED carries the same figure.

The figure reproduces. Its content does not survive being opened.

```bash
# every Decision-labelled section in the fractality specspace, and how many
# carry all four fields — excluding .vibe/cache/** as the census did
python <scratchpad>/adr-census.py packages/org.vibevm.fractality \
  | grep -v '/\.vibe/'                                     # 33 labels
python <scratchpad>/adr-census.py packages/org.vibevm.fractality \
  | grep -v '/\.vibe/' | awk -F'\t' '$5==1&&$6==1&&$7==1'  # 14 complete
```

**33 labels, 14 complete** — the census's `34 → 14` to within the ±1 my
instrument carries against every one of its figures (§3). And **all fourteen of
the complete records are vendored copies of the flow packages' own documents**:

```bash
python <scratchpad>/adr-census.py packages/org.vibevm.fractality \
  | grep -v '/\.vibe/' | awk -F'\t' '$5==1&&$6==1&&$7==1 {print $1" #"$4}'
```

| the 14 «complete records» | what the document actually is |
|---|---|
| `…/vibedeps/flow-decision-records/0.1.0/…/record-template.md` `#template` | the copy-ready template — the *shape*, with angle-bracket placeholders |
| the same file `#fields` | the field table: «passes when / fails when» |
| the same file `#example-timeout` | **the flow's own worked example** — a fictional 600 s VPN timeout |
| the same file `#example-library` | **the flow's other worked example** — blake3 over SHA-256 |
| `…/flow-decision-records/…/DECISION-RECORDS-PROTOCOL.md` `#four-fields` | the four-field definition table |
| `…/flow-decision-records/0.1.0/spec/boot/25-flow-decision-records.md` `#core-rule` | the boot snippet's copy of the same table |
| `…/vibedeps/flow-comparative-research/0.1.0/…/from-research-to-roadmap.md` `#accepted` | a different flow's worked example |

Seven documents — **counted twice**, because two packages under
`packages/org.vibevm.fractality/` each vendor the same flows
(`fractality/v0.1.0/vibedeps/` and `delegation-rules/v0.1.0/vibedeps/`).
7 × 2 = 14, exactly.

**Verified as vendored copies, not independent writing.** `diff` against the
canonical package reports the vendored copies differ *only* by the Phase B
progress markup (`<status>` elements and `##anchor` labels) that has never been
published — the prose is the flow's. The vendored `record-template.md` still
carries `**Decision:** 600 seconds.` at line 51 and `**Decision:** blake3 for
every content hash.` at line 98.

```bash
diff packages/org.vibevm.world/decision-records/v0.1.0/spec/flows/decision-records/record-template.md \
     packages/org.vibevm.fractality/fractality/v0.1.0/vibedeps/flow-decision-records/0.1.0/spec/flows/decision-records/record-template.md
```

**What the fractality project actually authored is nine blocks in two files, in
a different, three-label dialect.**

```bash
python <scratchpad>/adr-census.py packages/org.vibevm.fractality \
  | grep -v '/\.vibe/\|/vibedeps/'                          # 9 labels
grep -rn "When to revisit\|Revisit when" packages/org.vibevm.fractality \
  --include='*.md' | grep -v '/\.vibe/\|/vibedeps/' | wc -l # 0
```

Seven are `D-R1`…`D-R7` in
`packages/org.vibevm.fractality/fractality/v0.1.0/spec/plans/FRACTALITY-RLM-RESEARCH-PLAN-v0.1.md`
§4; two are in `spec/refs/notes/rlm-runners-up-t3.md`. The dialect, verbatim
(`D-R1`, lines 111–120):

> **Decision:** the research is its own plan (this document, Campaign 3 · Stage
> A). … **Rejected:** embedding the research as Stage B's Phase 0 — the
> Phase-0-no-commits law conflicts with committed study notes, and
> implementation decisions written before the study would violate the
> clean-room order (decisions flow FROM notes). **Revisit:** if Ф5 finds the
> field too thin to justify a full campaign, Stage B may shrink to a slice
> inside another campaign — owner call at RP-R3.

and `D-R2`, lines 125–134:

> **Decision:** Wave 1 = the deep-research harness; Wave 2 = plain web search
> executed directly, **without reading Wave 1's output first**; Wave 3 = merge.
> … **Rejected:** a single deep-research pass (cheaper, but self-confirming — no
> second modality to catch what the harness misses); seeding W2 queries from W1
> results (kills independence). **Revisit:** never — this is the owner's
> protocol verbatim.

Three labels, not four: **`Decision` / `Rejected` / `Revisit`**, with the *why*
folded into the Decision sentence rather than given its own field. Zero of the
nine carry a `**Why**` label; zero carry `Considered and rejected` or
`When to revisit` verbatim; one states its trigger as `**Revisit:** never`,
which the flow's own table types as the `##ROW-TRIGGER-LATER` failure in
costume — though here it is honest, because the decision genuinely is the
owner's protocol.

**The same shape contaminates one more published figure.** Of the census's
«all of `spec/` **157 → 7**», one of the seven is
`spec/boot/STATIC.md#core-rule` — the *same* boot-snippet table, compiled into
the host's static lane. Two more are in
`spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md`, this campaign's own
plan. **Four of the seven are host PROP/FEAT records; three are the
specification of the practice, or the campaign writing about it.**

```bash
python <scratchpad>/adr-census.py spec | awk -F'\t' '$5==1&&$6==1&&$7==1 {print $1":"$2"  #"$4}'
# spec/boot/STATIC.md:252  #core-rule                    <- the flow's own table
# spec/modules/vibe-cli/PROP-036-package-tree.md:88  #effective-load
# spec/modules/vibe-progress/PROP-043-progress-markup.md:91  #element
# spec/modules/vibe-progress/PROP-043-progress-markup.md:134  #stages
# spec/modules/vibe-progress/PROP-043-progress-markup.md:243  #placement
# spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md:93  #world-verdicts   <- this campaign
# spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md:121  #ai-native-verdicts
```

### What this does to the question {#zero-consequence}

- **The «41 % in the sibling project» premise is withdrawn.** The sibling
  project's own adoption rate of the four-field form is **0 of 9**. It writes a
  three-label variant in one plan document and nowhere else.
- **«Why is the PROP tree the outlier?» has no referent.** Measured over
  authored content only, there is **no** live four-field practice anywhere in
  this repository except (i) four host PROP sections and (ii) this campaign's
  own plans. The PROP tree is not an outlier against a thriving sibling; it is
  the *majority* case, and the campaign's plans are the outlier.
- **The question therefore reverts to the one §census thought it had escaped**
  — but better posed than «adopt a practice», because §1 below makes
  «reopenable» decidable and §3 sizes it. The right question is: **which
  decisions are owed a record, and is the four-field block the right form for a
  document genre that already argues in prose?**
- **This is §6.1's trap in a new dress, and the rule it needs is one line
  wider.** `##THE-CAMPAIGN-IS-INSIDE-ITS-OWN-CORPUS` says a search over this
  repository finds the campaign's own records. The generalisation the D10
  measurement forces: **a search over this repository also finds the
  *specification* of the practice being measured, vendored once per consumer,
  and counts it as the practice.** `vibedeps/**` and `.vibe/cache/**` hold N
  copies of every flow the host installs; any adoption metric that does not
  exclude them measures the installer, not the adopter. Recommended as an
  addition to §6.1 — filed as a proposal, not applied, because §6.1 is the
  batch plan and this file edits nothing.

*Reported as an observation, per `BACKLOG.md` ##SEV-ASSIGNED-BY-REVIEWER: the
severity of «a published campaign figure measures the wrong thing» is the
reviewer's call, not this worker's. It is not filed as a defect here.*

---

## §1 — The criterion: what makes a Decision «genuinely reopenable» {#criterion}

`PHASE-D-HOST-OBLIGATIONS.md` §census asks *«which PROP/FEAT decisions are
genuinely reopenable — almost certainly far fewer than 153»*. The flow already
ships a test — `##THE-COMPETENT-NEWCOMER-TEST`, *«would a competent newcomer,
reading the code cold, plausibly propose changing this?»* — and it is the right
test but not a decidable one: «plausibly» is exactly the word two reviewers
disagree on, and a criterion two reviewers disagree on cannot size 154 sections.

**Proposed sharpening: the same test, restated as three artefacts that must be
nameable. A section is reopenable iff all three can be named. «I cannot name
it» is a No.**

| | the question | what must be produced | fails when |
|---|---|---|---|
| **Q1 · name the condition** | Can you complete the sentence *«this stands while X»*, where X is **outside the project's own definitions**? | one clause naming an external dependency, a measured value, or a stated scope limit (`in v1`, `until`, `v2+`) | X is a term this project defined — then changing X is not a re-decision, it is a different system |
| **Q2 · name the observation point** | Could a stranger check X **today**? | a file, a command, an upstream repository, a version, or a measurement the project already takes | nothing observes X — the record would carry the unobservable trigger `##UNOBSERVABLE-TRIGGERS-ARE-AS-BAD-AS-NONE` forbids |
| **Q3 · name the loser** | Was an alternative **actually available at the time**, and could someone re-propose it? | one named alternative and the reason it lost | nothing was ever in contention — the section is a fact wearing a `Decision` label, and `##DO-NOT-RECORD-WHAT-HAS-NO-PLAUSIBLE-ALTERNATIVE` says do not record it |

**Why three artefacts rather than one judgement.** They are the flow's own three
fields with the *Decision* removed: Q1 and Q2 together are exactly
`##A-TRIGGER-HAS-THREE-PARTS` (metric/event + threshold + observation point),
and Q3 is `##ROW-FIELD-CONSIDERED-AND-REJECTED`. So the criterion is not a new
rule bolted beside the flow — **it is the flow's own record shape used as an
admissions test**: a section is owed a record precisely when a record could be
written for it without inventing data, which is what
`##ASK-RATHER-THAN-INVENT-DATA` demands anyway. A section that fails Q1–Q3 could
only receive a record by fabricating a why or a trigger, and the flow forbids
both.

**The three ways a section fails, named so a classifier can cite one.**

- ##N1-DEFINITIONAL **N1 · definitional.** The section fixes a term, a grammar,
  or an identity that everything downstream is keyed on. Changing it is a
  migration, not a re-decision. *(Fails Q1: X is ours.)*
- ##N2-DOWNSTREAM **N2 · downstream of a recorded choice.** The section states a
  consequence of a decision taken elsewhere. The reopenable point is upstream,
  and putting a record here creates a second writer for one fact — this
  campaign's single most-repeated finding. *(Fails Q3: the loser belongs to the
  upstream section.)*
- ##N3-DESCRIPTIVE **N3 · descriptive.** The `Decision` label is doing the work
  of a heading over a layout, a schema, a route table, a command surface.
  Nothing was in contention; the section describes what the module *is*.
  *(Fails Q3.)*

**And the three ways it passes.**

- ##R1-EXTERNAL **R1 · external dependency.** The choice is forced by a third
  party's behaviour, licence, format, limit, or version — a crate, `git`, an
  agent's config schema, an OS path limit. The third party can change without
  asking us.
- ##R2-MEASURED-THRESHOLD **R2 · measured threshold.** The value is a number or
  a cut-off chosen against an observation. New measurement moves it.
- ##R3-LIVE-ALTERNATIVE **R3 · live alternative.** A named alternative was
  declined for a reason that is itself contingent — cost, effort, immaturity,
  «not needed yet». New evidence revives it. **An explicit deferral is always
  R3**: `##MARK-HONEST-DEFERRALS-AS-DEFERRED-NOT-REJECTED` says a deferral has a
  built-in revisit.

**Two calibration notes, both learned from §6.1.**

- ##CRITERION-CAPABILITY-VS-PRACTICE **Ask what the sentence asserts before
  classifying it.** §6.1's `##A-REAL-DEFECT-CONVICTING-THE-WRONG-SENTENCE`
  distinguishes a *capability*, a *practice*, and a *rule*. The same split binds
  here: a section describing a **capability the system offers** is N3 unless the
  capability's shape was contested; a section stating a **rule the project keeps**
  is reopenable only if the rule has a stated scope limit.
- ##CRITERION-READ-THE-WHOLE-SECTION **Read the whole section before answering
  Q3.** §6.1's `##READ-FURTHER-BEFORE-SEARCHING-WIDER` applies verbatim: in this
  corpus the loser is very often present, twelve lines below the Decision line,
  unlabelled. §2 shows two such sections and §3 measures how common it is.

---

## §2 — Six worked examples, three each way {#examples}

All six are quoted from `HEAD = 91ebf1fd`, with file and line, and all six sit
inside the `spec/common` + `spec/modules` perimeter the census measures.

### R · `PROP-001` §2.5 `#freshness` — a measured threshold {#ex-ttl}

`spec/modules/vibe-registry/PROP-001-git-backend.md:225,231`:

> ##FRESHNESS-TTL **Decision:** the default freshness TTL is **1 hour**, checked
> against `meta.toml.last_pulled_at`. …
>
> ##ttl-why **Why 1 hour:** short enough to pick up new package versions within
> one working session, long enough to amortise network round-trips over a burst
> of installs. **Revisit once real usage arrives.**

- **Q1 · condition** — «stands while a working session is the unit of freshness
  and round-trips are worth amortising». ✔ **R2**, a threshold.
- **Q2 · observation point** — `meta.toml.last_pulled_at` against install
  frequency; the value is one constant in `vibe-registry`. ✔
- **Q3 · loser** — a shorter TTL (correctness over cost) and no TTL at all
  (always pull). Both are named implicitly by the *why*'s two-sided argument. ✔
- **Verdict: reopenable.** Note what the host already did here: it wrote
  *«Revisit once real usage arrives»* — the flow's `##ROW-ANTI-REVISIT-LATER`
  anti-pattern **in its own words**. The section wants to be a record and fails
  on the trigger alone. Note also `##NO-OFFLINE-YET` at line 235, labelled
  `**Superseded — --offline shipped.**`: this section has *already been
  reopened once*, in place, with a dated supersession line — which is
  `##CONSEQUENCE-EVOLUTION-IS-AN-EDIT` performed correctly and unlabelled.

### R · `PROP-002` §2.8 `#solver` — a library pick with the record already written in prose {#ex-solver}

`spec/modules/vibe-registry/PROP-002-decentralized-registry.md:538,544,546`:

> ##RESOLVO-PRIMARY **Decision.** The primary depsolver is the
> [`resolvo`](https://crates.io/crates/resolvo) crate (pure Rust,
> BSD-3-Clause-or-Apache-2.0, used by Pixi and Rattler at conda scale). …
>
> ##NOT-PUBGRUB **Not** `pubgrub` — the algorithm does not handle virtual
> packages or disjunctions, undershoot relative to PROP-000 §18.
>
> ##LIBSOLV-FALLBACK-SLOT **libsolv as explicit fallback.** … a future
> `LibsolvSolver` (FFI to C libsolv, BSD-3-Clause) drops in as a feature-gated
> alternative **if resolvo ever hits a ceiling we can't raise**. …

- **Q1 · condition** — an upstream crate's feature set and maintenance. ✔ **R1**
  and **R3**.
- **Q2 · observation point** — the resolvo repository; the constraint classes
  PROP-000 §18 enumerates; a solve that fails. ✔
- **Q3 · loser** — two of them, each with its reason: `pubgrub` («does not handle
  virtual packages or disjunctions») and libsolv (kept as a slot, not rejected).
  ✔
- **Verdict: reopenable — and this is the shape that decides §4.** All four
  fields are *present*, in prose, unlabelled: a decision, a why with three cited
  properties, two losers with reasons, and an event trigger («if resolvo ever
  hits a ceiling we can't raise») that would pass
  `##EVENT-TRIGGERS-TAKE-THE-SAME-TEST` almost as written. **Making this a
  four-field record is relabelling, not authoring.**

### R · `PROP-012` §2.2 `#markers` — a protocol shape with a named, declined alternative {#ex-markers}

`spec/modules/vibe-workspace/PROP-012-managed-redirect-block.md:58,60,61`:

> ##BARE-TAGS **Decision.** The block is delimited by the literal **bare tags**
> `<vibevm>` and `</vibevm>` …
>
> - ##bare-tags-why Bare tags are chosen over HTML-comment delimiters
>   (`<!-- vibevm:begin -->` …): they read unambiguously to an LLM, the file's
>   primary consumer.
> - ##cosmetic-cost A markdown renderer may display a bare non-standard tag
>   oddly — an accepted cosmetic cost, **addressed separately if it ever
>   matters**.

- **Q1 · condition** — «stands while an LLM is the file's primary consumer and
  the renderer cost is tolerable». ✔ **R3**; `##ROW-OCCASION-PROTOCOL-SHAPE`.
- **Q2 · observation point** — how the block renders on GitHub; who actually
  reads `CLAUDE.md`. ✔
- **Q3 · loser** — HTML-comment delimiters, with the reason it lost. ✔
- **Verdict: reopenable.** Again the loser and the accepted cost are already
  written; only the trigger is missing, and `##cosmetic-cost`'s *«if it ever
  matters»* is the unobservable form of one.

### N · `PROP-008` §2.2 `#identity` — definitional {#ex-identity}

`spec/modules/vibe-registry/PROP-008-qualified-naming.md:59,63`:

> ##IDENTITY-TUPLE **Decision.** Package identity becomes
> `(group, name, version, content_hash)`. `kind` **leaves the identity tuple**.
> …
>
> - ##GROUP-CHANGE-NEW-PACKAGE Changing a package's `group` is a new package,
>   not a rename — same discipline as changing `name`.

- **Q1 · condition** — none external. Every term in the tuple is one this
  project defines; the lockfile, the cache, the index and the resolver are all
  keyed on it. ✘
- **Q3 · loser** — the previous tuple, which this PROP *replaced*; there is no
  third candidate anyone could re-propose.
- **Verdict: not reopenable — ##N1-DEFINITIONAL.** Changing it is a migration of
  every artefact in the system, which is a programme, not a re-decision. A
  revisit trigger here would be ceremony: nothing anyone could observe would
  make it fire.
- **One carve-out worth the owner's eye.** The *neighbouring* `PROP-002` §2.1
  `##IDENTITY-TUPLE` fixes `content_hash` as **`sha256:<hex>`**. The hash
  *algorithm* is R1/R3 — it is verbatim the flow's own worked example
  (`#example-library`, blake3 over SHA-256), and the host has taken the opposite
  choice with no recorded why. The tuple is N1; the algorithm inside it is not.
  **The classification unit is the claim, not the section.**

### N · `PROP-000` §5 `#layout` — a pointer, not a decision {#ex-layout}

`spec/common/PROP-000.md:77`:

> - ##LAYOUT-PER-SPEC **Decision:** Per `VIBEVM-SPEC.md` §4.2.

- **Q3 · loser** — nothing. The line delegates to another document. ✘
- **Verdict: not reopenable — ##N3-DESCRIPTIVE.** This is the clearest instance
  of the label doing a heading's work, and it is the reason a raw count of 154
  overstates the debt: some of the 154 are not decisions in any sense the flow
  recognises.

### N · `PROP-009` §2.9 `#uniform` — downstream of PROP-007 {#ex-uniform}

`spec/modules/vibe-workspace/PROP-009-loading-model.md:148`:

> ##UNIFORM-MODEL **Decision.** The loading model is uniform: a single-package
> project is a degenerate (zero-member) workspace. `Workspace::discover` already
> degenerates cleanly (PROP-007 §2.3).

- **Q1 · condition** — none external. **Q3 · loser** — «two code paths», which
  PROP-007 already declined; this section inherits that.
- **Verdict: not reopenable — ##N2-DOWNSTREAM.** A record here would be a second
  writer for PROP-007's fact, which is `##ENTRY-CITES-NEVER-RESTATES` in
  `BACKLOG.md` and the defect class this whole campaign exists to remove.

### Two calibration cases the corpus already settled for us {#ex-calibration}

- **The host has already written a «not reopenable» verdict, in the flow's own
  field.** `spec/common/PROP-000.md:23` — `##LANG-REVISIT **When to revisit:**
  Never, in the scope of v1. If Rust proves inadequate for a future milestone,
  open a new PROP superseding this one.` By `##ROW-TRIGGER-LATER` that is a bad
  trigger; by §1 it is an **honest N1 classification wearing the trigger
  field**. Any option the owner picks should let a section say this without it
  reading as a defect.
- **The host has already reopened a record correctly, unprompted.**
  `spec/modules/vibe-index/PROP-005-package-index.md:820,822` —
  `##WORKSPACE-MEMBER **Decision (revised 2026-05-22).**` followed by
  `##fold-in-why **Why this reverses the original standalone-workspace
  decision.** … It rotted silently against the M1.17 / M1.18 manifest-schema
  churn`. Dated, in place, with the superseded option and its measured failure.
  That is `##OUTCOME-CHANGED` and `##CONSEQUENCE-EVOLUTION-IS-AN-EDIT` executed
  by a project that was not following the flow. **The placement rule is already
  the host's practice; the field labels are not.**

---

## §3 — The sizing {#sizing}

### 3.1 The population, re-measured {#population}

```bash
python <scratchpad>/adr-census.py spec/common spec/modules            # 154 labels
python <scratchpad>/adr-census.py spec/common spec/modules \
  | awk -F'\t' '{print $5,$6,$7}' | sort | uniq -c | sort -rn         # field histogram
```

| perimeter | Decision labels | Why | Considered-and-rejected | Revisit | all four |
|---|---:|---:|---:|---:|---:|
| `spec/common` + `spec/modules` — the census's perimeter | **154** | 27 | 4 | 7 | **4** |
| all of `spec/` | 158 | — | — | — | 7 *(3 of them not host records — [§0](#zero))* |
| `campaigns/**` — **this campaign's own, broken out**, excluding `harvest/d10-*` | 17 | — | — | — | 10 |
| the same, **including this file and its sibling** | 31 | — | — | — | 12 |
| `packages/org.vibevm.fractality/**` **excluding** `.vibe/` | 33 | — | — | — | 14 *(all 14 vendored — [§0](#zero))* |
| the same, **authored by the specspace itself** | **9** | 0 | 0 | 0 | **0** |

**Two reconciliations, stated rather than smoothed.**

- **My instrument reads within ±1 of every recorded census figure** — 154 vs
  153, 158 vs 157, **33 vs 34** — and the **complete-record counts agree
  exactly** at 4, 7 and 14. The label offset is one row in each direction, so it
  is an instrument difference, not a disagreement: the most likely cause on the
  host side is the single `**Decision (revised 2026-05-22).**` variant at
  `PROP-005-package-index.md:820` or a section carrying two Decision lines that
  the earlier pass counted once. **It changes no conclusion at this resolution**
  — the figures that carry the argument are the complete-record counts, and
  those are identical. Recorded so the next reader does not re-derive it.
- **`campaigns/**` has moved twice since the census, inside the same day:**
  recorded `15 → 8`, measured at batch start `17 → 10`, measured at batch end
  `31 → 12`. Nothing was mis-counted. The campaign wrote two more records
  between the census and this batch — and then **this batch wrote fourteen
  more**, because a document arguing about decision records is full of the words
  `Decision`, `Why`, `Considered and rejected` and `Revisit when`. That is
  `##THE-CAMPAIGN-IS-INSIDE-ITS-OWN-CORPUS`'s second clause — *«it moves its own
  measurements, inside a single session»* — firing twice on the exact figure
  that reframed B-007, the second time on the file you are reading. `0` of the
  17 are under `campaigns/*/run/**`.

**82.5 % of the population carries none of the three supporting fields**
(127 of 154). The debt, stated plainly: the host writes the Decision line and
stops, four times out of five.

### 3.2 The sample {#sample}

**Frame.** The census output ordered by file, then line; every 5th row taken
(indices 1, 6, 11 … 151). **n = 31, 20 % of the population, and it touches all
17 files that carry a Decision label** — so no file dominates and none is
missed. Each was classified by §1's Q1/Q2/Q3, reading the whole section.

| # | anchor | verdict | why |
|---|---|---|---|
| 1 | `PROP-000#language` `##LANG-RUST` | **N1** | the host itself wrote «revisit: Never, in the scope of v1» |
| 6 | `PROP-000#identity` `##IDENTITY-FORM` | **N2** | restates PROP-008 §2.2 |
| 11 | `PROP-000#package-layout` `##MIRROR-LAYOUT` | **R3** | the declined `target` field is named on the next line |
| 16 | `PROP-000#complexity` `##RPM-CLASS-TARGET` | **N** | fails Q2 — no observation point for «RPM-class» |
| 21 | `PROP-018#affinity` `##AFFINITY-DECL` | **N1** | defines the affinity vocabulary |
| 26 | `PROP-018#transports` `##ONE-OP-TWO-TRANSPORTS` | **N3** | structural shape; the loser (define it twice) was never in contention |
| 31 | `PROP-024#shippable-tree` `##SHIPPABLE-TREE-DEF` | **R1** | a build-output **denylist** moves with every new toolchain |
| 36 | `PROP-036#effective-load` | **R** | already a complete four-field record |
| 41 | `PROP-005#modes` `##ONE-BINARY-TWO-MODES` | **N3** | describes the binary's surface |
| 46 | `PROP-005#http` `##HTTP-API` | **N** | fails Q2 — nothing observes «REST is still right» |
| 51 | `PROP-005#distribution` `##WORKSPACE-MEMBER` | **R3** | proven reopenable: it *was* reopened, 2026-05-22 |
| 56 | `PROP-015#agent-config` `##CONFIG-SHAPE-DECL` | **R1** | five external agents' config schemas, each free to change |
| 61 | `PROP-043#stages` | **R** | already a complete four-field record |
| 66 | `PROP-001#cache-layout` `##CACHE-LAYOUT` | **R** ~ | superseded in practice by PROP-010's identity-keyed cache |
| 71 | `PROP-002#identity` `##IDENTITY-TUPLE` | **R** | the tuple is N1; **`sha256` inside it is R1/R3** ([§2](#ex-identity)) |
| 76 | `PROP-002#enabled` `##ENABLED-FLAG` | **N3** | a flag and its default |
| 81 | `PROP-002#layout` `##FLAT-LAYOUT` | **R3** ~ | one-package-per-repo, against PROP-007's workspace publishing |
| 86 | `PROP-002#publish` `##PUBLISH-UTILITY` | **R3** | explicit deferral — «semantic review remains v2+» |
| 91 | `PROP-008#kind` `##KIND-METADATA` | **N1** | defines what `kind` is |
| 96 | `PROP-008#index-ext` `##INDEX-FIELDS` | **N2** | consequence of §2.2 |
| 101 | `PROP-010#user-registries` `##USER-LEVEL-REGISTRIES` | **R3** ~ | a rival home for the same config is specified at `PROP-002#global-config` |
| 106 | `PROP-010#layering` `##LAYERS-EXPLICIT` | **N3** | says outright it «changes none of the three existing layers» |
| 111 | `PROP-003#interface-tags` | **N** | fails Q2 — a specified-not-built extension; the open question is *whether to build*, which is a plan, not a record |
| 116 | `PROP-007#nesting` `##NESTED-WORKSPACES` | **R3** ~ | «arbitrary depth» against the re-proposable «one level» |
| 121 | `PROP-007#published-repos` `##ONE-SOURCE-TREE` | **N1** | defines the development/publication split |
| 126 | `PROP-009#inclusion-types` `##INCLUSION-TYPES` | **N1** | defines `static` / `dynamic` |
| 131 | `PROP-009#uniform` `##UNIFORM-MODEL` | **N2** | downstream of PROP-007 ([§2](#ex-uniform)) |
| 136 | `PROP-011#materialise-diff` `##SLOT-SKIP` | **R1** | rests on a premise §2.6 says does not hold for path-sources |
| 141 | `PROP-012#markers` `##BARE-TAGS` | **R3** | loser named ([§2](#ex-markers)) |
| 146 | `PROP-038#units` `##UNIT-PER-PACKAGE` | **N1** | defines the per-unit artefact model |
| 151 | `PROP-038#single-version` `##SINGLE-VERSION-INVARIANT` | **R1** | correctness depends on resolvo's single-version guarantee |

**Result: 15 reopenable, 16 not — 48 %.** Four are marked `~` **borderline**;
striking all four gives the strict reading **11 / 31 = 35 %**.

### 3.3 The extrapolation, with its error stated {#extrapolation}

| reading | rate | over 154 |
|---|---:|---:|
| hand classification, as scored | 48 % | **≈ 75** |
| strict — the four `~` struck | 35 % | **≈ 55** |
| ±1 standard error on the point estimate (n = 31) | ±9 pp | ±14 |
| independent mechanical proxy, whole population (below) | 38 % | 58 |

**The estimate: between about a third and about a half — ≈ 55 to 75 sections.
Plan against ≈ 60.** The number that matters for the owner's decision is the
order of magnitude, and it is unambiguous: **tens, not 153.**

**The mechanical proxy, and why it corroborates only weakly.** A second
instrument flags any Decision-bearing section whose body names a declined
alternative *or* a contingent condition:

```bash
python <scratchpad>/reopenable-probe.py <scratchpad>/host.tsv
# Decision labels in scope: 154
#   section names a declined alternative (Q3 proxy): 39  (25.3%)
#   section names a contingent condition (Q1 proxy): 33  (21.4%)
#   BOTH proxies fire: 14  (9.1%)
# union (loser OR condition): 58  (37.7%)
```

Scored against the hand classification on the same 31 sections it agrees on
**20 of 31 = 65 %** — sensitivity 8/15, specificity 12/16. It misses seven
reopenable sections because their contingency is in the argument rather than in
a keyword, and flags four that are not. **So its 38 % corroborates the order of
magnitude and nothing finer**; the hand sample is the measurement.

### 3.4 The figure that decides the cost, and it is not the count {#already-written}

The estimated ~60 is *not* ~60 records to author. §2 found the same shape three
times in three different PROPs: **the losers and the conditions are already
written, in prose, unlabelled.** `##RESOLVO-PRIMARY` carries two named losers
with reasons and an event trigger; `##BARE-TAGS` carries its loser and its
accepted cost; `##FRESHNESS-TTL` carries a two-sided why and a trigger in the
anti-pattern form; `##WORKSPACE-MEMBER` carries a measured why for a reversal it
already performed.

**27 of 154 already carry a labelled `Why`** and the union proxy puts ~58 in the
class that argues in prose. On the sample, **8 of the 15 reopenable sections
fired the proxy** — i.e. roughly half the reopenable population is a
**relabelling** job, and the rest need one field written, not four. What is
almost universally absent is the **trigger**: 7 of 154, and three of those seven
are `Revisit: later` / `Revisit: never` in some form.

**So the honest cost line is: ~60 sections, of which about half need labels
around reasoning that already exists, and nearly all need a revisit trigger
written for the first time.** The trigger is the expensive field — and it is the
one `##TRIGGER-FIELD-KEEPS-THE-IMMUNITY-HONEST` says the whole practice depends
on.

---

## §4 — The form {#form}

### 4.1 One of B-007's three candidates is forbidden by the installed flow {#no-silo}

`BACKLOG.md` ##B007-THE-QUESTION offers three shapes: *«as a section inside the
PROP/FEAT that owns the decision, as a separate `spec/decisions/` genre, or as
the four-field block the `decision-records` flow already prescribes»*.

**The middle one is not open.** The flow is installed in this project (its boot
snippet compiles into `spec/boot/STATIC.md`), and it forbids the silo in four
places:

- `25-flow-decision-records.md:27` ##NO-SEPARATE-ADR-DIRECTORY — *«There is no
  separate ADR directory and no immutable numbered log.»* Line 29,
  ##GOVERNING-SPEC-SECTION-IS-THE-RECORD — *«The spec section that governs the
  value IS the record.»*
- `DECISION-RECORDS-PROTOCOL.md` ##CONSEQUENCE-NO-ADR-DIRECTORY — *«No `adr/`
  directory. The spec tree is the only home.»*
- the same file's `#placement` table, ##ROW-ADR-SILO, which types
  `adr/0007-use-blake3.md` as the **classic** practice this protocol departs
  from — with the reason at ##a-record-in-a-silo-is-never-looked-up and
  ##SILOS-PRESERVE-TECHNICALLY-AND-LOSE-PRACTICALLY.
- `##SUM-RECORDS-LIVE-AT-THE-ANCHOR` in the summary.

**And it has already cost a false verdict in this very campaign.** From
[`run/state/routing.json`](../run/state/routing.json), obligation **F-233**, on
`git-attribution-policy/v0.1.0/README.md#COMPOSES-DECISION-RECORDS`:

> The prior verdict searched for the artefact the flow forbids (a
> `spec/decisions/` directory, against `##NO-SEPARATE-ADR-DIRECTORY`).

So the silo option is not merely disfavoured — **a worker has already looked for
it, not found it, and written a wrong verdict on the strength of its absence.**
Any proposal that creates `spec/decisions/` contradicts an installed flow and
would have to be recorded as a §3.6(c) deliberate exception *against the
package*, which is the one direction §3.6 exists to make hard.

The first and third candidates are the **same** shape: the flow's four-field
block **is** «a section inside the PROP/FEAT that owns the decision». There are
therefore two live options, not three.

### 4.2 The recommended shape is already the host's house style {#house-style}

**All four complete host records use one convention, and it is consistent
4 for 4.** From `PROP-043-progress-markup.md:91-99` verbatim:

> ##DECISION-ELEMENT-NAME **Decision — element name `status`, not `progress`.**
>
> - ##element-name-why **Why:** `<progress>` is an HTML5 element: GitHub-class
>   sanitizers strip it, and `html:true` renderers (VS Code preview) draw a
>   literal progress-bar widget mid-spec. `status` is not an HTML element and
>   renders inert.
> - ##element-name-rejected **Considered and rejected:** `progress` (HTML
>   collision), `vp`/`prg` (unreadable), HTML comments (invisible in raw
>   reading, defeating the point).
> - ##element-name-revisit **Revisit when:** the XML storage frontend lands and
>   element naming is re-grounded in a schema.

The convention, stated so it can be followed without re-deriving it:

| part | anchor register | shape |
|---|---|---|
| the decision | `##DECISION-<THING>` — **UPPER**, the normative register | `**Decision — <one line>.**` |
| the why | `##<thing>-why` — kebab, the service register | `**Why:** <observation, cited>` |
| the losers | `##<thing>-rejected` | `**Considered and rejected:** <loser> (<reason>), …` |
| the trigger | `##<thing>-revisit` | `**Revisit when:** <condition + where observed>` — but see the spelling note below |

`PROP-036#effective-load` uses `##DECISION-ARTIFACTS-CANONICAL` +
`##decision-artifacts-{why,rejected,revisit}`; `PROP-043#stages` uses
`##DECISION-FREEZE-STAGE` + `##freeze-{why,rejected,revisit}`;
`PROP-043#placement` uses `##DECISION-TWO-REGISTERS` +
`##registers-{why,rejected,revisit}`. **Nothing needs designing here — the host
has a house style, used four times, and it satisfies the flow's
`##CONSEQUENCE-RECORDS-ARE-CITABLE` because every field carries its own
anchor.**

**Two spellings are live and one should be pinned.** `PROP-043` writes
`**Revisit when:**` three times; `PROP-036-package-tree.md:95` writes
`**When to revisit:**`, which is the flow's own label
(`##ROW-FIELD-WHEN-TO-REVISIT`). Both are already accepted by the census
instrument, so nothing is mis-measured — but a corpus with two spellings for one
field is a grep away from an undercount, and pinning one is a free line in
whichever ruling lands.

**And a calibration point that argues for tolerance, not rigour.** One of the
four complete records reads `##freeze-revisit **Revisit when:** never expected;
the cycle-of-improvement premise is core.` (`PROP-043:141`) — a **«never»
trigger inside a record that otherwise passes every field test.** Read against
`##ROW-TRIGGER-LATER` that is a defect; read against §1 it is an honest
##N1-DEFINITIONAL classification written in the trigger's slot, exactly as
`##LANG-REVISIT` does. **Whichever option is chosen should sanction that
sentence explicitly**, or the host will be scored down for its most honest
records.

**Two consequences worth the owner's attention, both of them arguments in
favour of this shape over the sibling project's three-label dialect
([§0](#zero)):**

- **A four-field record is four addressable facts, so adoption becomes
  measurable by the instruments the project already runs.** Each field takes its
  own `##anchor` and therefore its own `vibe progress` verdict; a record with a
  stub trigger fails visibly rather than passing as prose. The three-label
  dialect folds the why into the decision sentence, and an unlabelled why cannot
  be judged separately from the claim it supports — which is exactly the defect
  ##B004-THE-GENERAL-SHAPE describes for fenced content.
- **One open sub-question, not decided here.** The house style puts three of the
  four fields in the **kebab / service** register, which
  `PROP-043` ##DECISION-TWO-REGISTERS defines as *«status lines, lead-ins,
  connective prose»* — non-binding. A *why* is evidence and belongs there; a
  **trigger** arguably does not, since ##TRIGGER-SAYS-THIS-DECISION-STANDS-UNTIL-X
  makes it a rule that *«both forbids re-litigation before X and mandates it
  after»*. Whether `##<thing>-revisit` should be `##REVISIT-<THING>` is a
  one-line ruling the owner may fold into whichever option is chosen.

### 4.3 What the genre map's new row would read — and why it should probably not be a row {#genre-row}

`BACKLOG.md` ##B007-WHY-IT-IS-A-QUESTION-NOT-A-TASK notes that *«`spec-genres`'
own map does not carry an ADR row today»*. **On reading the map, that is
correct and deliberate: the map already places decision records as a
*mutability*, not a genre.**

`SPEC-GENRES-PROTOCOL.md#genres`, verbatim:

| Genre | Charter | Mutability | Reader | Authority |
|---|---|---|---|---|
| **Foundational decisions** | Choices that cross every module (identity, versioning, licensing) | **Amended by decision record** | Everyone | Binding |
| **Module contracts** | What one module does — its behaviour, constraints, invariants (this convention: PROP / FEAT) | **Edit + changelog line** | Implementers | Binding |

Two things follow, and the second is the more surprising.

- **A record is not a genre, so it does not take a row.** The map's unit is a
  *document kind* with a charter, a reader and an authority; a decision record
  is a **section-level form** that lives inside two of the existing binding
  genres. Adding a row would require naming a home — and the host's own instance
  of the table, `spec/design/README.md` ##genre-table-lead, is keyed on
  `| Directory | Holds | Normative? |`. **A row needs a directory, and the flow
  forbids the directory.** The two facts meet exactly here.
- **The map assigns records to the genre that has none of them.**
  «Amended by decision record» is the mutability of **Foundational decisions** —
  `spec/common/` in the host's instance. Measured:

```bash
python <scratchpad>/adr-census.py spec/common spec/modules \
  | awk -F'\t' '$5==1&&$6==1&&$7==1 {print $1}' | sort | uniq -c
```

| genre | host directory | Decision labels | complete records |
|---|---|---:|---:|
| Foundational decisions — *«amended by decision record»* | `spec/common/` | 35 | **0** |
| Module contracts — *«edit + changelog line»* | `spec/modules/` | 119 | **4** |

**The practice is inverted against the map.** Every complete record the host has
written sits in the genre the map does *not* ask for one, and the genre the map
*does* ask for one has none. That is a cleaner, smaller finding than «the PROP
tree is an outlier», and it makes a genre-scoped option available that nobody
has proposed yet ([§5](#options), option A′).

**If the owner does want the map amended, the minimal honest change is a
mutability statement, not a row** — for the host's instance,
`spec/design/README.md`, a fourth column or a sentence under
##genre-table-lead reading approximately:

> **Decision records are a section form, not a directory.** A reopenable choice
> in `common/` or `modules/` carries its four fields — Decision · Why ·
> Considered and rejected · Revisit when — **inside the section that governs the
> value**, per `spec://org.vibevm.world/decision-records/flows/decision-records/DECISION-RECORDS-PROTOCOL#placement`.
> There is no `spec/decisions/` directory and there will not be one
> (`##NO-SEPARATE-ADR-DIRECTORY`).

*(Drafted for the owner to accept, amend or discard — **not applied**. This
batch edits no file but this one.)*

---

## §5 — Three options and one variant, costed {#options}

### 5.0 What every option has to close {#what-rides}

```bash
python -c "import json;d=json.load(open('campaigns/packages-2026-09/run/state/routing.json'));\
print(len([e for e in d['entries'] if '/decision-records/' in e.get('anchor','')]))"   # 10
```

**Twelve anchors ride on this one ruling, across three packages, and every one
is `route: host`:**

| obligation | anchors | what the host is measured against |
|---|---:|---|
| **F-197** | 3 | `25-flow-decision-records.md` — `#ANY-REOPENABLE-CHOICE-GETS-A-RECORD`, `#ASK-RATHER-THAN-INVENT-DATA`, `#NEVER-RECORD-A-MISSING-REASON-OR-TRIGGER` |
| **F-198** | 3 | `DECISION-RECORDS-PROTOCOL.md` — `#EVERY-RECORD-CARRIES-EXACTLY-FOUR-FIELDS`, `#ROW-FIELD-WHEN-TO-REVISIT`, `#SUM-FOUR-FIELDS-ALWAYS` |
| **F-224** | 2 | the `When to revisit` field definition, in the snippet and the template |
| **F-225** | 1 | `record-template.md#SUM-DATA-REASONS-AND-A-MEASURABLE-TRIGGER` |
| **F-299** | 1 | `decision-records/v0.1.0/README.md#FOUR-FIELD-RECORD-AT-THE-GOVERNING-ANCHOR` |
| **F-233** | 2 | `#COMPOSES-DECISION-RECORDS` in **`git-attribution-policy`** and **`source-mirrors`** — two *other* packages whose composition claim the host also fails |

**One thing no option closes, named so nothing is credited with it.** F-224 also
carries a **package-side self-falsifier**, recorded in `routing.json` and not yet
worked: `record-template.md:45` defines the trigger as *«metric + threshold +
observation point»* while its own `#example-library`, twelve lines below, ships a
pure event trigger as the correct model. `PHASE-D-HOST-OBLIGATIONS.md#weight`
lists it among the three that are *«a one-line host fix rather than a ruling»* —
except that one is `self`, i.e. **the package's own obligation**. It is
independent of everything below.

**The closure mechanic is the same for all three options** — §3.1 `#closure`: the
host does the thing (or records the exception), the anchors are **re-judged**,
`merge-verdicts.py <slice.json> --force` then, separately,
`vibe progress seal` (never chained — ##NEVER-CHAIN-MERGE-AND-SEAL). What
differs is *what the host does* and *how many anchors close on it*.

---

### Option A — the four-field block is owed to reopenable decisions only, backfilled {#option-a}

**The shape.** Adopt §1's Q1/Q2/Q3 as the host's stated criterion, publish it,
and backfill the four-field block — in the §4.2 house style, inside the owning
section — over the sections it selects.

**What it costs.**

| item | estimate | who pays |
|---|---:|---|
| sections in scope | **≈ 60** (band 55–75, [§3.3](#extrapolation)) | — |
| of those, reasoning already in prose → relabelling | ≈ 30 | delegable; cheap review against the section's own text |
| of those, a why must be **sourced** | ≈ 30 | **owner** — `##ASK-RATHER-THAN-INVENT-DATA` and the flow's re-derive step 3 forbid inventing it; a worker's honest output is `TODO(owner)` |
| revisit triggers to write | **≈ 55** | **owner**, nearly all of them — 7 of 154 exist today, and `##COLLECT-THE-SIGNAL-OR-REWRITE-THE-TRIGGER` requires the signal to be one this project actually observes |
| new addressable anchors | ≈ 180 | each takes a `vibe progress` verdict |

**The real cost is the trigger field, and it is owner-bound.** A worker can
relabel a why; a worker cannot decide what measurement this project will watch.
Roughly **55 owner rulings** is the honest headline, and
`##backfilled-reasoning-is-fiction` plus `##ROW-ANTI-BACKFILLED-MEMORY` say a
worker filling those from the document alone produces *«fiction with
confidence»*.

**What it does to the obligation.** Closes **all 12** on route (b) — the
worked precedent is `flow:campaign-plans`' `##COLD-A-LITERAL-QUICK-START-BLOCK`
(`PHASE-D-HOST-OBLIGATIONS.md#answers`, answer 1): the owner ruled the rule
sound, the host complied, the fact re-judged `confirmed` with no package edit.

**The trap it must not fall into.** With ≈ 60 in and ≈ 94 out, *nothing records
the negative*. Next year's census re-reads 154 raw `Decision` labels and reports
the same debt, because «classified not-reopenable» and «not done» look
identical. Two ways out, and the cheap one is right:

- ✗ write `**Revisit when:** never — <reason>` on all ≈ 94 — 94 more edits, and
  `##ROW-TRIGGER-LATER` makes «never» read as a defect to every future reader.
- ✓ **publish the criterion, and derive the count from it** rather than from the
  raw label — `BACKLOG.md` ##ENTRY-PREFER-GENERATED, *«prefer generated over
  hand-maintained»*. The instrument then reports «N reopenable, M complete», and
  the 94 stop being debt because they were never in scope.

### Option A′ — the same, scoped to the genre the map already names {#option-a-prime}

**The shape.** Option A, but the obligation binds **`spec/common/` only** — the
*Foundational decisions* genre, whose declared mutability in the installed map
is already *«amended by decision record»* ([§4.3](#genre-row)). `spec/modules/`
keeps its declared mutability, *«edit + changelog line»*, unchanged.

**What it costs.** 35 Decision labels in scope, of which the sample puts ≈ 2 in 7
reopenable — **≈ 10 records, ≈ 10 triggers**. *(Sample n = 7 for this
sub-perimeter; the rate is indicative only, the population count of 35 is
exact.)* One to two sessions, not a programme.

**What it does to the obligation.** Closes the 12 **as a (b) + (c) pair**: the
host adopts for foundational decisions, and records a **marked exception** for
module contracts naming the genre map as its ground. Phase C's ruling —
*«a marked exception is not drift»* — makes that a real closure.

**Why it is worth putting on the table.** It is the only option whose scope is
argued from a document rather than from a budget, and it inverts the finding
that `spec/common/` — the genre the map assigns records to — has **zero** of
them.

---

### Option B — the four-field block is required forward-only; no backfill {#option-b}

**The shape.** From the ruling forward, any *new or reopened* decision in
`spec/common` or `spec/modules` carries the four fields. Existing sections are
untouched. `##WRITE-IN-THE-SESSION-THAT-DECIDES` becomes the operative rule and
`##backfilled-reasoning-is-fiction` becomes the stated reason for not
backfilling.

**What it costs.** One paragraph in `spec/design/README.md` (§4.3's draft), plus
per-decision discipline. **Zero backfill.** The marginal cost of a record
written in the deciding session is minutes, because the reasoning is in working
memory — which is the flow's entire argument.

**What it does to the obligation.** Closes the 12 as **(b) adopted + (3)
deferred backfill with the reason on record**. The exit gate accepts that
(`#gate` check 1: *«every survivor carries an owner ruling»*), and the reason is
strong rather than budgetary: **the flow itself says a backfilled why is
fiction.**

**What it forfeits.** The ≈ 60 existing reopenable decisions stay unrecorded, so
the two failure modes `##FAILURE-RE-DERIVATION` and `##FAILURE-RE-LITIGATION`
keep running on exactly the choices most likely to be re-proposed — including
`sha256` ([§2](#ex-identity)), the resolvo/libsolv slot, and the 1-hour TTL whose
own text says *«Revisit once real usage arrives»*. It also does nothing for the
**already-reopened** cases, where the reasoning is *not* lost and a record is
therefore *not* fiction: `PROP-005#distribution` and `PROP-001#freshness` both
carry their supersession in the document today.

**Cheap amendment worth naming.** Forward-only **plus a bounded, opportunistic
backfill**: `##RHYTHM-OPPORTUNISTIC` — *«whenever a session touches a document,
glance at the records in it»*. Any session that edits a Decision-labelled
section completes it if the reasoning is in the document, and files the rest.
Zero scheduled work, and the ≈ 30 relabelling cases drain by attrition.

---

### Option C — the status quo, recorded as a deliberate §3.6(c) exception {#option-c}

**The shape.** The host states, on the host side, that its PROP/FEAT genre argues
rationale **in prose** and does not carry the four labelled fields; the flow's
*placement* rule is kept (records live at the governing anchor — which the host
already does), the *field* rule is not.

**What it costs.** One paragraph. It is the cheapest option by an order of
magnitude and it is a **legitimate** answer, not a loophole:
`PHASE-D-HOST-OBLIGATIONS.md#answers` lists it second, and Phase C ruled that a
marked exception is not drift while an unmarked one is.

**What it does to the obligation.** Closes the 12 on route (c), *provided the
exception is written and the anchors re-judged with it named*. An exception
declared in chat and not in a file closes nothing.

**What it forfeits, and it is more than it looks.** The four fields are not four
equal conveniences. Dropping *Considered and rejected* costs re-litigation of
settled evaluations; dropping **When to revisit** costs the mechanism the flow
says the practice lives or dies by —
`##NO-CONDITION-MEANS-A-SACRED-COW`, and `##TRIGGER-FIXES-BOTH-AT-ONCE`
(a recorded decision without a trigger *«survives on the authority of being
recorded»*). The host has 7 triggers in 154 sections today, so option C mostly
ratifies what exists — but it ratifies it as **policy**, which is a different
thing from an accident, and the next reader is entitled to read it as «this
project has decided not to know when its decisions expire».

**And one honest risk.** §3.6 warns that the easy direction is *«quietly
rewriting the discipline to describe a lax consumer»* — the *профанация* §0 of
the campaign mandate exists to prevent. Option (c) is explicitly sanctioned and
is therefore **not** that. But it is the option most easily mistaken for it, and
if it is chosen the exception's wording should say what the host does *instead*
(prose rationale at the governing anchor, kept on reopening), not merely what it
declines.

### 5.5 The options side by side {#options-table}

| | **A** — backfill the reopenable | **A′** — backfill `spec/common` only | **B** — forward-only | **C** — status quo as a marked exception |
|---|---|---|---|---|
| **sections in scope** | ≈ 60 of 154 | ≈ 10 of 35 | 0 existing | 0 |
| **records to write** | ≈ 60 | ≈ 10 | new decisions only | none |
| **owner rulings** (triggers + unsourced whys) | **≈ 55 + ≈ 30** | ≈ 10 + ≈ 5 | per decision, in the deciding session | 1 (the exception's wording) |
| **delegable share** | ≈ half (relabelling) | ≈ half | n/a | n/a |
| **closes the 12 anchors** | ✔ all, route (b) | ✔ all, (b) for `common` + (c) for `modules` | ✔ all, (b) adopted + (3) backfill deferred | ✔ all, route (c) |
| **argued from** | the census's raw count | **the installed genre map** | the flow's own `##backfilled-reasoning-is-fiction` | cost |
| **keeps the trigger mechanism** | ✔ | ✔ for foundational choices | ✔ going forward | ✘ |
| **main risk** | ≈ 55 triggers invented under deadline = `##ROW-ANTI-BACKFILLED-MEMORY` at scale | leaves 119 module-contract labels unrecorded | the ≈ 60 existing reopenable choices stay re-litigable | reads as «we have decided not to know when our decisions expire» |

*(`spec/decisions/` is absent from this table on purpose: [§4.1](#no-silo) — the
installed flow forbids it in four places and it has already produced one false
verdict.)*

---

## §6 — The campaign's recommendation {#recommendation}

> **This is the campaign's recommendation, not a ruling.** B-007 is
> ##B007-DISPOSITION `open` *«as a question to answer rather than work to
> schedule»*, and the genre decision is the owner's. Nothing below has been
> applied and no verdict has moved.

**Recommended: B + A′ — forward-only as the standing rule, plus the small
document-argued backfill of `spec/common/`. Written as the record it proposes,
so the form can be judged on its own example.**

##DECISION-ADR-GENRE **Decision — the four-field block, inside the owning
section, required forward-only; backfilled only over `spec/common/`; no
`spec/decisions/`, now or later.**

- ##adr-genre-why **Why.** Four measurements, each with its command in this
  file. *(i)* The premise for a large backfill has been withdrawn: the «41 % in
  the sibling project» figure is **14 vendored copies of the flow's own template
  and worked examples**, counted twice; the specspace's own adoption is **0 of
  9** ([§0](#zero)). There is no thriving practice for the PROP tree to be an
  outlier against. *(ii)* The scope is **≈ 60 of 154**, not 153 — but the cost
  is not the 60, it is the **≈ 55 revisit triggers**, and a trigger names a
  signal *this project actually observes*, which no worker can decide
  (`##COLLECT-THE-SIGNAL-OR-REWRITE-THE-TRIGGER`). *(iii)* The flow argues
  against the backfill in its own voice: `##backfilled-reasoning-is-fiction`,
  *«a record backfilled a week later is fiction with confidence»*, and
  `##ROW-ANTI-BACKFILLED-MEMORY`. Sixty backfilled records under a phase
  deadline is that anti-pattern executed at scale, by the campaign that exists
  to remove exactly this defect class. *(iv)* `spec/common/` is the one scope
  argued from a document rather than a budget: the installed genre map already
  declares its mutability *«amended by decision record»*, and it holds **35
  labels and 0 complete records** — the practice is inverted against the map
  ([§4.3](#genre-row)).

- ##adr-genre-rejected **Considered and rejected.**
  - **A — backfill all ≈ 60** — rejected: its expensive half is ≈ 55 owner
    rulings that cannot be delegated, and producing them on a schedule is the
    `##ROW-ANTI-BACKFILLED-MEMORY` failure. Not rejected *in principle*: A′ is
    its first instalment, and B's opportunistic clause drains the ≈ 30
    relabelling cases without scheduling them.
  - **C — status quo as a marked exception** — rejected: it is legitimate and
    cheap, and it forfeits the trigger, which
    `##NO-CONDITION-MEANS-A-SACRED-COW` makes the load-bearing field. A project
    that keeps `sha256`, resolvo-over-libsolv and a 1-hour TTL with no stated
    condition for reopening any of them is one whose next agent re-derives all
    three — which is `##FAILURE-RE-DERIVATION`, measured three times in this
    file's §2.
  - **A separate `spec/decisions/` genre** — rejected: forbidden by the
    installed flow in four places ([§4.1](#no-silo)), and it has **already**
    caused a false verdict in this campaign (F-233, `routing.json`). It should
    be closed explicitly in the host's genre table so the next worker does not
    search for it a second time.
  - **The sibling project's three-label dialect** (`Decision` / `Rejected` /
    `Revisit`, why folded into the decision sentence) — rejected: an unlabelled
    why cannot carry its own `##anchor`, so it cannot be judged, and adoption
    stops being measurable by the instruments the project already runs
    ([§4.2](#house-style)).
  - **Adding an ADR row to the genre map** — rejected as the wrong unit: the
    map's row is a document kind with a directory, and the flow forbids the
    directory. What is owed is a **mutability statement**, drafted at the end of
    [§4.3](#genre-row).

- ##adr-genre-revisit **Revisit when.** Re-run the census
  (`adr-census.py spec/common spec/modules`, excluding `vibedeps/**` and
  `.vibe/**`) **at the close of the next campaign**. Two triggers, either one
  reopening this:
  - **the forward rule is not being kept** — fewer than **3 in 4** Decision
    sections *added or edited since the ruling* carry all four fields; or
  - **the deferred backfill has started to cost** — a session re-litigates a
    decision that a record would have answered, twice in one campaign, recorded
    in the campaign's findings ledger.

  *(Both are observable from artefacts this project already keeps: the census
  script over `git log --since`, and the `F-NNN` finding space.)*

### 6.1 What the recommendation asks the owner to do, concretely {#ask}

Four items, in dependency order. **None is started; this batch edits no file but
this one.**

1. **Rule on the genre question** — which of A / A′ / B / C, or a combination.
   Everything below is scoped by that.
2. **Ratify or amend the criterion** of [§1](#criterion) (Q1 condition · Q2
   observation point · Q3 loser), and decide **where it is published** — the
   natural home is `spec/design/README.md`, beside ##genre-table-lead, so the
   count of «sections owed a record» becomes **derivable** rather than
   hand-maintained (`BACKLOG.md` ##ENTRY-PREFER-GENERATED). Without this, the
   negative classification is unrecorded and the same census re-fires next year.
3. **Rule on the two small sub-questions** raised in passing: whether
   `##<thing>-revisit` should move to the UPPER register ([§4.2](#house-style)),
   and whether §6.1 of the batch plan gains the one-line perimeter rule
   `vibedeps/**` + `.vibe/cache/**` are excluded from any adoption count
   ([§0](#zero-consequence)).
4. **Then, and only then, the closure** — the twelve anchors of
   [§5.0](#what-rides) are re-judged against whatever was ruled, per §3.1
   `#closure`: `merge-verdicts.py --force`, read the output, then
   `vibe progress seal` — **never chained** (##NEVER-CHAIN-MERGE-AND-SEAL).
   `PHASE-D-HOST-OBLIGATIONS.md` and `BACKLOG.md` #b-007 are updated to the
   ruling in the same pass, and F-224's package-side self-falsifier stays open
   as a `self` obligation, untouched by any of this.
