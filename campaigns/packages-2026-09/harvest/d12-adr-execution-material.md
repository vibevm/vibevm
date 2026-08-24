# D12 — execution material for the «B + A′» ruling: the criterion, the genre-table closure, the `spec/common` backfill, the forward rule

_Phase D, batch D12. **This file edits nothing.** It carries four blocks
**prepared for the boss to apply after review**: a publishable criterion, a
genre-table row plus the `spec/decisions/` closure line, the twelve
`spec/common` backfill records drafted in place, and the forward-only rule.
Every revisit trigger below is marked **OWNER CONFIRMS** — a trigger names a
signal this project actually watches, and no worker may decide that
(`#COLLECT-THE-SIGNAL-OR-REWRITE-THE-TRIGGER`)._

**No file was edited but this one.** No spec file, no package file, no campaign
state, no verdict JSON, no `vibe progress check`, no git write. `git` was run
read-only (`rev-parse`).

**Measured at `HEAD = 96b5b55f`**, working tree clean at batch start
(`git status --porcelain` → empty). Every count below names the command that
produced it, per
[`##THE-CAMPAIGN-IS-INSIDE-ITS-OWN-CORPUS`](../PHASE-D-BATCH-PLAN.md#delegation-lessons).

**HEAD advanced to `f1abad16` while this batch ran, and every figure holds.**
One commit — `docs(wal): the WAL pays its own addressability debt and catches up
to волна 9` — touching **one file, `spec/WAL.xml`**, which this batch does not
measure, quote, or edit:

```bash
git log --oneline 96b5b55f..HEAD          # 1 commit
git diff --name-only 96b5b55f..HEAD -- spec/   # spec/WAL.xml — and nothing else
grep -rn '\*\*Decision' spec/common --include='*.md' | wc -l   # 35, re-verified at f1abad16
```

The census re-measures **35**, and none of `spec/common/PROP-000.xml`,
`PROP-018-*.md`, `PROP-024-*.md`, `spec/design/README.md`, the six `spec/modules`
files quoted in [§1.3](#examples-verified), or the `decision-records` package
moved. **Every line number, quotation and count below is valid at both
`96b5b55f` and `f1abad16`.** Recorded rather than silently re-based, per
`##THE-CAMPAIGN-IS-INSIDE-ITS-OWN-CORPUS`: *«any figure over `git log` names the
HEAD it was taken at»*. *(The working tree also carries a sibling batch's
untracked `harvest/d12-health-audit-rulings.md`, which is not this batch's and
was not touched.)*

**The ruling being executed** (owner, 2026-08-01, «B + A′»): the four-field
decision record — Decision · Why · Considered and rejected · Revisit when —
is adopted **inside the owning section**, **forward-only** for new reopenable
decisions, with a **backfill limited to `spec/common/`**, the **criterion
published** so the negative classification of the rest is derivable, and
**`spec/decisions/` closed explicitly** in the host's genre table. The decision
material is [`d10-adr-genre-proposal.md`](d10-adr-genre-proposal.md); this file
is its execution half.

**What was read to write this**, in order: `d10-adr-genre-proposal.md` in full
(including its [§0](d10-adr-genre-proposal.md#zero) premise-withdrawal); the
installed flow in full — [`25-flow-decision-records.xml`](../../../packages/org.vibevm.world/decision-records/v0.1.0/spec/boot/25-flow-decision-records.xml),
[`DECISION-RECORDS-PROTOCOL.xml`](../../../packages/org.vibevm.world/decision-records/v0.1.0/spec/flows/decision-records/DECISION-RECORDS-PROTOCOL.xml),
[`revisit-triggers.xml`](../../../packages/org.vibevm.world/decision-records/v0.1.0/spec/flows/decision-records/revisit-triggers.xml),
[`record-template.xml`](../../../packages/org.vibevm.world/decision-records/v0.1.0/spec/flows/decision-records/record-template.xml);
the genre map, flow-side and host-side; and **all three `spec/common` files that
carry a `Decision` label, end to end** (PROP-000, PROP-018, PROP-024 — 1091
lines).

## Two rules that bind every line below

- @fact:NEVER-INVENT-A-WHY **A why is sourced or it is `TODO(owner)`.** Every *Why*
  and every *Considered and rejected* drafted in [§3](#backfill) is quoted or
  paraphrased from **the section's own prose, its PROP's history lines, or the
  frozen `VIBEVM-SPEC.md` it cites** — with the source anchor named on the same
  line. Where a field was not derivable it says **NOT DERIVABLE** and stops.
  `##ROW-ANTI-BACKFILLED-MEMORY` — *«Reconstructed reasoning is fiction with
  confidence; the data is gone»* — and d10's `##backfilled-reasoning-is-fiction`
  bind this batch directly, because a backfill **is** the anti-pattern unless
  every field is sourced.
- @fact:THE-CAMPAIGN-IS-INSIDE-ITS-OWN-CORPUS-D12 **This campaign's own plans are not
  host practice.** `campaigns/**` carries many complete four-field blocks —
  including in this file. None is counted as host-PROP adoption anywhere below;
  every census command in this file is scoped to `spec/common`.

---

## Contents

- [§0 — The census, and what it changes about «≈ 10»](#census)
- [§1 — Block 1 · the criterion, publishable text + where it lives](#criterion)
- [§2 — Block 2 · the genre-table row and the `spec/decisions/` closure](#genre-table)
- [§3 — Block 3 · the twelve `spec/common` backfill records](#backfill)
- [§4 — Block 4 · the forward-only rule](#forward-rule)
- [§5 — What this asks the owner to confirm](#ask)

---

## §0 — The census, and what it changes about «≈ 10» {#census}

```bash
# the population: every Decision-labelled line in spec/common
grep -rn '\*\*Decision' spec/common --include='*.md' | wc -l      # 35
grep -rc '\*\*Decision' spec/common --include='*.md' -r           # per file, below
```

| file | Decision labels |
|---|---:|
| `spec/common/PROP-000.xml` | 18 |
| `spec/common/PROP-018-agentic-standalone-modes.xml` | 11 |
| `spec/common/PROP-024-code-bearing-packages.xml` | 6 |
| the other nine files in `spec/common/` (PROP-006, -013, -016, -019, -028, -029, -031, -032, -033) | **0** |
| **total** | **35** |

**35 exactly — the figure d10 §4.3 reports, reproduced at `HEAD = 96b5b55f`.**
The population is unchanged since d10 was written. Note the shape the raw count
hides: **the whole `spec/common` decision debt lives in three files**, and nine
of the twelve files in the genre the map assigns records to carry no `Decision`
label at all.

### The count moved: ≈ 10 was extrapolated, 12 is measured {#twelve-not-ten}

d10 §5's option A′ estimates *«≈ 10 records»* from a **sample of n = 7**, and
says so: *«Sample n = 7 for this sub-perimeter; the rate is indicative only, the
population count of 35 is exact.»* This batch classified **all 35** by the
criterion, reading each section whole. The result is **12 reopenable, 23
settled** — inside the estimate's band, one above its point value.

**One of the twelve is marked `~` borderline** (`##MODE-INFERRED`, [§3.6](#r-mode-inferred)):
its *Why* is sourced from the record's own reasoning rather than from a
measurement, which is a weaker why than the other eleven carry. **Striking it
gives 11.** No other section is close to the line in either direction; the
owner has one lever here, not a dial.

*Reported rather than rounded to the ruling's «≈ 10», per
`##THE-CAMPAIGN-IS-INSIDE-ITS-OWN-CORPUS`: a figure re-measured at a new HEAD
names what it found.*

### The full classification {#classification}

Command-derived order — `grep -rn '\*\*Decision' spec/common --include='*.md'`,
file then line. **R** = reopenable (owed a record); **N** = settled (not owed
one), with the failing gate named.

| # | file:line | anchor · §anchor | verdict | one line |
|---|---|---|---|---|
| 1 | `PROP-000:16` | `##LANG-RUST` `#language` | **N1** | the host already wrote the verdict in the trigger field — *«Never, in the scope of v1»* |
| 2 | `PROP-000:29` | `##WORKSPACE-LAYOUT` `#build` | **N3** | a layout following the ecosystem default; no loser ever in contention (Q3) |
| 3 | `PROP-000:47` | `##LICENSE-EULA` `#license` | **R1/R3** | third-party licence terms are external and change without asking us; **3 of 4 fields already written** |
| 4 | `PROP-000:63` | `##MANIFEST-TOML` `#manifests` | **N2** | the live alternative (JTD/JSON) is already recorded at §16 `##JTD-OUT-OF-SCOPE` — a record here is a second writer |
| 5 | `PROP-000:77` | `##LAYOUT-PER-SPEC` `#layout` | **N3** | *«Per `VIBEVM-SPEC.md` §4.2.»* — the label doing a heading's work (d10 [§2](d10-adr-genre-proposal.md#ex-layout)) |
| 6 | `PROP-000:86` | `##IDENTITY-FORM` `#identity` | **N2** | restates PROP-008 §2.2 (d10 sample #6) |
| 7 | `PROP-000:100` | `##REG-M0` `#registry` | **N3** | a milestone-scope statement, superseded in place by `##REG-M1`; **but see the carve-out at [§3.13](#carve-out)** |
| 8 | `PROP-000:124` | `##GRAPH-BUILTIN-NODES` `#graph` | **R3** | *«in v1»* is a stated scope limit; the deferral and its target version are in the frozen spec §5.4 |
| 9 | `PROP-000:140` | `##OBS-TRACING` `#observability` | **N3 ⚑** | a library pick — the flow's first named occasion — but **nothing recorded was in contention** (Q3); top owner-overturn candidate |
| 10 | `PROP-000:148` | `##PLATFORMS-TRIO` `#platforms` | **N3** | a support-scope statement; no recorded contention |
| 11 | `PROP-000:170` | `##MIRROR-LAYOUT` `#package-layout` | **R3** | the declined `target =` field is named on the very next line (d10 sample #11) |
| 12 | `PROP-000:187` | `##MT-LOCATION` `#manual-tests` | **N3** | a directory + filename convention; the tier's rationale is the installed flow's |
| 13 | `PROP-000:208` | `##DEP-WEIGHT-NOT-FACTOR` `#dep-weight` | **R3** | a process rule whose predecessor policy is named **and its reversal recorded** (`##READMISSIBLE`) |
| 14 | `PROP-000:230` | `##JTD-SSOT` `#jtd` | **R1/R3** | RFC 8927 + `jtd-codegen` are upstream; the loser (JSON Schema / OpenAPI) is named with its reason |
| 15 | `PROP-000:255` | `##PROD-QUALITY-DAY-ONE` `#prod-arch` | **N3** | a stance, not a choice; the rival lens is *retained* (`##LENS-NOT-ONLY`), not rejected |
| 16 | `PROP-000:272` | `##RPM-CLASS-TARGET` `#complexity` | **N** (Q2) | nothing observes «RPM-class» (d10 sample #16); the resolver claims below it are N2, pinned in PROP-002 §2.8 |
| 17 | `PROP-000:286` | `##SETUP-DOCS` `#setup-docs` | **N2** | downstream of the installed `dev-runtime-docs` flow, which `##SETUP-DOCS-FLOW` cites |
| 18 | `PROP-000:299` | `##TOKEN-SURFACE-SECRET` `#token-secrecy` | **N2** | downstream of the installed `secrets-hygiene` flow; the section says so and lists its bindings |
| 19 | `PROP-018:99` | `##MODE-INFERRED` `#mode-is-backend` | **R3 ~** | the loser is in the decision sentence (*«not a global flag the user sets»*); **why sourced from prose, not data** |
| 20 | `PROP-018:117` | `##BACKEND-TRAIT` `#pluggable-backend` | **N3** | the only «loser» is an anticipated over-build objection `##NOT-OVERBUILT` answers — not an alternative in contention |
| 21 | `PROP-018:141` | `##AFFINITY-DECL` `#affinity` | **N1** | defines the affinity vocabulary (d10 sample #21) |
| 22 | `PROP-018:156` | `##SKILL-SECTION-NOT-KIND` `#skill-decl` | **R3** | **already half-reopened** — `##MCP-HALF-SUPERSEDED`, owner resolution 2026-07-07 |
| 23 | `PROP-018:191` | `##PROJECTION-DEF` `#projection` | **N3 ⚑** | five external agents' skill-path conventions make Q1 strong, but Q3 has only a *distinction*, not a declined alternative; second owner-overturn candidate |
| 24 | `PROP-018:207` | `##SKILL-CMD-FAMILY` `#vibe-skill` | **N3** | a command surface (cf. d10 sample #41) |
| 25 | `PROP-018:227` | `##RELAY-PARKS` `#relay` | **R3** | the declined alternative — *«just printing the intent from the producer»* — is named with two reasons |
| 26 | `PROP-018:261` | `##ONE-OP-TWO-TRANSPORTS` `#transports` | **N3** | structural shape; the loser (define it twice) was never in contention (d10 sample #26) |
| 27 | `PROP-018:284` | `##USAGE-SKILL-TEACHES` `#usage-skill` | **N3** | a content addition to an existing artefact |
| 28 | `PROP-018:301` | `##EXPLAIN-DEMONSTRATOR` `#explain` | **N3** | a first-demonstrator pick, already executed; nothing observes whether it was the right first op |
| 29 | `PROP-018:326` | `##RELAY-DIR` `#vibevm-dir` | **N1/N3** | the section says outright the path *«carries no external contract and can move freely»* (`##PATH-INTERNAL`) |
| 30 | `PROP-024:91` | `##PKG-PROJECT-LAW` `#package-is-project` | **N1** | «a package is a project» is the identity law the whole PROP is named for; changing it is a migration |
| 31 | `PROP-024:116` | `##SHIPPABLE-TREE-DEF` `#shippable-tree` | **R1** | a four-name build-output denylist that moves with every new toolchain (d10 sample #31) |
| 32 | `PROP-024:149` | `##MATERIALISE-AS-TODAY` `#build` | **N2** | *«as today»* — the behaviour is PROP-009 §2.1's and PROP-022's |
| 33 | `PROP-024:175` | `##OWN-WORKSPACE` `#consume` | **R1/R3** | Cargo forbids two workspaces over one directory; the loser is named, reasoned **and retained as a fallback** |
| 34 | `PROP-024:217` | `##SELF-HOST-VENDORED` `#bootstrap` | **R3** | crates.io publication is an **explicit deferral** (`##REJ-CRATES-IO`) — always R3 |
| 35 | `PROP-024:236` | `##PLACEMENT-LAW` `#placement` | **R3** | **its trigger already fired and was honoured in place** (`##CORE-STAYS-PROMPT-ONLY`) — it now needs a fresh one |

**12 R · 23 N.** Per file: PROP-000 **5 of 18**, PROP-018 **3 of 11**,
PROP-024 **4 of 6**. The concentration is worth the owner's eye:
**PROP-024 is two-thirds reopenable and PROP-000 is one-quarter** — the newest
document in `spec/common`, written against live external constraints (Cargo,
Windows path handling, a build-output denylist), owes far more records per
section than the foundational one, which is mostly definitional by design.

### The two settled verdicts most worth overturning {#overturn-candidates}

Marked ⚑ above. Both fail on **Q3 alone**, and only on the record *as written* —
if the owner remembers a real evaluation, the section becomes reopenable and a
record is owed. Neither can be promoted by a worker, because promoting them
means authoring a *Considered and rejected* field from memory, which is exactly
`##ROW-ANTI-BACKFILLED-MEMORY`.

- **`##OBS-TRACING`** (`PROP-000:140`) — *«Use `tracing` for structured logs.»*
  A library pick is `##ROW-OCCASION-LIBRARY-PICK`, the flow's first named
  occasion, and `log` / `slog` were available. Nothing in the section records a
  contest.
- **`##PROJECTION-DEF`** (`PROP-018:191`) — skill projection into
  `.claude/skills/`, `.opencode/skills/`, `.agents/skills/`. **Q1 is as strong
  as any R1 in the corpus** — five external agents' conventions, each free to
  change under us — and d10 sample #56 scored the parallel `PROP-015#agent-config`
  as R1 for exactly that. What is missing is a declined alternative:
  `##NO-SHARED-CODE` and `##DISTINCT-FROM-DELIVERY` draw a *distinction* from
  PROP-003 §2.5 subskill delivery, they do not record a rejection.

---

## §1 — Block 1 · the criterion {#criterion}

### 1.1 Where it lives, and why that is not a normativity dodge {#home}

**Recommended home: `spec/design/README.md`, a new top-level section placed
immediately after `## vibevm's spec/ genres` and before `## Linking rule`** —
i.e. directly beneath the genre table, sharing the file with Block 2's row and
closure line.

Four reasons, and the fourth is the one that answers the obvious objection.

1. **The ruling puts three of its four artefacts in this file.** The criterion,
   the genre-table row, the `spec/decisions/` closure and the forward rule are
   one ruling; splitting them across two files means a reader who lands on the
   table does not meet the criterion that qualifies its `common/` and
   `modules/` rows.
2. **The file already carries the only host-side sentence about the practice.**
   `spec/design/README.md` `##PROP-WINS-PRECEDENCE` already says load-bearing
   rationale — *«the decision itself and the alternatives weighed, in each
   PROP's `Decision` / `Rejected alternatives` sections (the
   **decision-records** genre…)»* — **stays inside the PROP**. The criterion
   extends that existing sentence rather than opening a second home for the same
   fact (`BACKLOG.md` `##ENTRY-CITES-NEVER-RESTATES`).
3. **It matches the directory's genre.** `spec/design/`'s charter is *«the why
   and the lore behind our PROPs»*; its README is the genre guide. A criterion
   deciding *which* PROP sections are owed a record is genre guidance — this
   document's own subject.
4. **It does not need to create normativity, because the rule is already
   binding.** `##ANY-REOPENABLE-CHOICE-GETS-A-RECORD` is compiled into the
   host's **static boot lane** — `spec/boot/STATIC.xml:311`, from
   `vibedeps/flow-decision-records/0.1.0/spec/boot/25-flow-decision-records.md`
   (the provenance comment is at `STATIC.md:294`) — so it is read at the start
   of every session in this repository. The host text does not *make* the
   four-field record required; it **localises** an already-required rule: it
   names the criterion by which «reopenable» is decided *in this corpus* and
   records the scope the owner ruled. That is the same kind of object as the
   genre table itself, which is the host's instance of an installed flow's map.

```bash
grep -n 'ANY-REOPENABLE-CHOICE-GETS-A-RECORD' spec/boot/STATIC.xml   # 311
grep -n 'vibe:static org.vibevm.world/decision-records' spec/boot/STATIC.xml  # 294
```

**The alternative home, if the owner rejects reason 4.** If the criterion should
bind *in its own right* rather than as a localisation, its home is a **new
`spec/common/PROP-000.xml` section** — `##ROW-COMMON` is the only
`Normative? yes` home for a rule crossing every module, and PROP-000 §12
`#commits` already houses a process-discipline pointer of exactly this shape
(the `git-practices` family). The cost is real and should be stated: PROP-000
`##AMEND-FIRST` — *«Changing any of them requires an explicit amendment here
first, then downstream updates»* — would then bind every future change to the
criterion.

**Recommended against: a new `spec/design/decision-records.md`.** Every entry in
the directory's `## Index` is *«rationale for PROP-NNN»*; a criterion is not
that shape, it would be the only non-PROP-anchored document there, and it splits
the criterion from the table it qualifies.

### 1.2 The publishable text {#criterion-text}

Ready to paste as a new section of
[`spec/design/README.md`](../../../spec/design/README.md), after
`##research-vs-design` and before `## Linking rule`. Anchor registers follow
PROP-043 `##DECISION-TWO-REGISTERS` (`##UPPER-SLUG` = normative fact,
`##kebab-case` = service unit); `@doc/…` markers match every other unit in the
file.

````markdown
## Which decisions are owed a record {#owed-a-record}

<status stage="doc" state="done"/>

##criterion-lead The installed `decision-records` flow requires a four-field
record — Decision · Why · Considered and rejected · When to revisit — for **any
choice a future reader could plausibly re-open**
(`spec://org.vibevm.world/decision-records/flows/decision-records/DECISION-RECORDS-PROTOCOL#when`,
read at every session boot from `spec/boot/STATIC.xml`). *Plausibly* is the word
two reviewers disagree on, so vibevm decides it with three questions. **A
section is owed a record iff all three can be answered; «I cannot name it» is a
No.** @doc/done

| | The question | What must be named | Fails when |
|---|---|---|---|
| ##Q1-CONDITION **Q1 · the condition** @doc/done | Complete *«this stands while X»*, where X is **outside this project's own definitions** @doc/done | an external dependency, a measured value, or a stated scope limit (`in v1`, `until`, `v2+`) @doc/done | X is a term we defined — changing it is a migration, not a re-decision @doc/done |
| ##Q2-OBSERVATION-POINT **Q2 · the observation point** @doc/done | Could a stranger check X **today**? @doc/done | a file, a command, an upstream repository, a version, or a measurement this project already takes @doc/done | nothing observes X — the record would carry the unobservable trigger `#UNOBSERVABLE-TRIGGERS-ARE-AS-BAD-AS-NONE` forbids @doc/done |
| ##Q3-LOSER **Q3 · the loser** @doc/done | Was an alternative **available at the time**, and could someone re-propose it? @doc/done | one named alternative and the reason it lost @doc/done | nothing was in contention — the section is a fact wearing a `Decision` label, and `#DO-NOT-RECORD-WHAT-HAS-NO-PLAUSIBLE-ALTERNATIVE` says do not record it @doc/done |

##CRITERION-IS-THE-RECORD-SHAPE The three questions are the flow's own record
with the *Decision* removed: Q1 + Q2 are `#A-TRIGGER-HAS-THREE-PARTS`, Q3 is
`#ROW-FIELD-CONSIDERED-AND-REJECTED`. A section is therefore owed a record
**precisely when a record could be written for it without inventing data** —
which `#ASK-RATHER-THAN-INVENT-DATA` demands anyway. @doc/done

##CRITERION-UNIT-IS-THE-CLAIM **The unit is the claim, not the section.** One
section may hold a definitional claim and a reopenable one: PROP-002 §2.1 fixes
the identity tuple (definitional) *and* fixes `content_hash` as `sha256:<hex>`
(a library-grade choice against an external algorithm). Classify each claim. @doc/done

##criterion-pass-lead A section **passes** in one of three ways: @doc/done

- ##R1-EXTERNAL **R1 · external dependency** — the choice is forced by a third
  party's behaviour, licence, format, limit or version. The third party can
  change without asking us. @doc/done
- ##R2-MEASURED-THRESHOLD **R2 · measured threshold** — the value is a number or
  cut-off chosen against an observation. New measurement moves it. @doc/done
- ##R3-LIVE-ALTERNATIVE **R3 · live alternative** — a named alternative was
  declined for a contingent reason (cost, effort, immaturity, «not needed
  yet»). **An explicit deferral is always R3**:
  `#MARK-HONEST-DEFERRALS-AS-DEFERRED-NOT-REJECTED` — a deferral has a built-in
  revisit. @doc/done

##criterion-fail-lead It **fails** in one of three ways, and a classifier cites
the one it used: @doc/done

- ##N1-DEFINITIONAL **N1 · definitional** — the section fixes a term, a grammar
  or an identity everything downstream is keyed on. Changing it is a migration,
  not a re-decision. *(Fails Q1: X is ours.)* @doc/done
- ##N2-DOWNSTREAM **N2 · downstream of a recorded choice** — the reopenable
  point is upstream; a record here creates a second writer for one fact.
  *(Fails Q3: the loser belongs to the upstream section.)* @doc/done
- ##N3-DESCRIPTIVE **N3 · descriptive** — the `Decision` label is doing a
  heading's work over a layout, a schema, a route table, a command surface.
  *(Fails Q3.)* @doc/done

##criterion-worked-lead Two worked examples, one each way: @doc/done

- ##ex-shippable-tree **R — `PROP-024` §2.2 `#shippable-tree`.** The shippable
  tree is the package directory minus `.git/ .vibe/ target/ node_modules/`.
  **Q1** — four literal build-output names from four external toolchains; a
  fifth toolchain adds a fifth. **Q2** — the denylist itself, and any package
  whose hash moves between clean and built. **Q3** — two, both already written
  in §4: hashing build output (`#REJ-HASH-BUILD-OUTPUT`) and a per-file
  allow-list (`#REJ-ALLOW-LIST`). Owed a record. @doc/done
- ##ex-layout **N — `PROP-000` §5 `#layout`.** *«##LAYOUT-PER-SPEC **Decision:**
  Per `VIBEVM-SPEC.md` §4.2.»* **Q3** — the line delegates to another document;
  nothing was in contention. **##N3-DESCRIPTIVE.** Not owed a record, and this
  is why a raw count of `Decision` labels overstates the debt. @doc/done

##COUNT-IS-DERIVED-NOT-MAINTAINED **The negative classification is derived, not
recorded.** Sections that fail the criterion carry **no** marker saying so —
writing `**Revisit when:** never` across them would be ceremony, and
`#ROW-TRIGGER-LATER` makes «never» read as a defect to every future reader.
Instead, any census of the decision debt applies this criterion and reports
«N reopenable, M complete»; a raw count of `Decision` labels is not that number
and must not be quoted as it. (`BACKLOG.md` `#ENTRY-PREFER-GENERATED` — prefer
generated over hand-maintained.) @doc/done

##HONEST-NEVER-IS-NOT-A-DEFECT **A section may state «never» in the trigger
field and be correct.** `PROP-000` `##LANG-REVISIT` — *«Never, in the scope of
v1. If Rust proves inadequate for a future milestone, open a new PROP
superseding this one»* — is an honest ##N1-DEFINITIONAL verdict written in the
trigger's slot, together with the escape route. Read against `#ROW-TRIGGER-LATER`
it looks like an anti-pattern; read against this criterion it is the correct
answer, and it is not scored as debt. @doc/done
````

### 1.3 The worked examples, verified at HEAD {#examples-verified}

d10 §2 works six examples and states they are quoted *«from `HEAD = 91ebf1fd`»*.
**All six were re-read at `HEAD = 96b5b55f` and all six still hold**, with one
class of discrepancy worth recording because the criterion's publishable text
quotes two of them.

| d10 §2 example | file:line at `96b5b55f` | holds? |
|---|---|---|
| `##FRESHNESS-TTL` / `##ttl-why` | `PROP-001-git-backend.xml:225,231` | ✔ — **but see the emphasis note** |
| `##RESOLVO-PRIMARY` / `##NOT-PUBGRUB` / `##LIBSOLV-FALLBACK-SLOT` | `PROP-002-decentralized-registry.xml:538,544,546` | ✔ verbatim |
| `##BARE-TAGS` / `##bare-tags-why` / `##cosmetic-cost` | `PROP-012-managed-redirect-block.xml:58,60,61` | ✔ — **emphasis note** |
| `##IDENTITY-TUPLE` / `##GROUP-CHANGE-NEW-PACKAGE` | `PROP-008-qualified-naming.xml:59,63` | ✔ verbatim |
| `##LAYOUT-PER-SPEC` | `PROP-000.xml:77` | ✔ verbatim |
| `##UNIFORM-MODEL` | `PROP-009-loading-model.xml:148` | ✔ verbatim |
| calibration: `##WORKSPACE-MEMBER` / `##fold-in-why` | `PROP-005-package-index.xml:820,822` | ✔ verbatim |
| house style: `##DECISION-ELEMENT-NAME` + three fields | `PROP-043-progress-markup.xml:91,93,96,98` | ✔ verbatim |

@fact:EMPHASIS-IS-ADDED-IN-D10 **Emphasis note — d10 bolds two phrases the source
does not.** d10 §2 renders `##ttl-why` as *«**Revisit once real usage
arrives.**»* and `##cosmetic-cost` as *«**addressed separately if it ever
matters**»*. At HEAD both phrases are **unbolded** in the source:
`PROP-001:233` reads `Revisit once real usage arrives.` and `PROP-012:61` reads
`addressed separately if it ever matters.` — plain text inside the unit. The
words are verbatim; the bold is d10's, added to make its point. **Anything
published in the host tree must quote the unbolded form**, and no census
instrument keyed on `**Revisit` matches either — which is itself the finding
d10 §4.2 makes about two live spellings and an undercount.

---

## §2 — Block 2 · the genre-table row and the `spec/decisions/` closure {#genre-table}

### 2.1 The table as it stands at HEAD {#table-now}

[`spec/design/README.md:13-22`](../../../spec/design/README.md), verbatim —
lead-in and all six rows, so the row drafted below can be checked against the
grammar it must match:

```markdown
##genre-table-lead vibevm's instance of the genre table — the general taxonomy (each genre's charter, mutability, reader, and authority-on-conflict) is the flow's `spec://org.vibevm.world/spec-genres/flows/spec-genres/SPEC-GENRES-PROTOCOL#genres`: @doc/done

| Directory | Holds | Normative? |
|---|---|---|
| ##ROW-BOOT [`boot/`](../boot/) @doc/done | Session-boot instructions read at the start of every session @doc/done | yes @doc/done |
| ##ROW-COMMON [`common/`](../common/) @doc/done | Foundation decisions crossing every crate (PROP-000, PROP-006) @doc/done | yes @doc/done |
| ##ROW-MODULES [`modules/`](../modules/) @doc/done | Per-crate PROP / FEAT — the implementation contract @doc/done | yes @doc/done |
| ##ROW-RESEARCH [`legacy-spec/research/`](../../legacy-spec/research/) — archived 2026-07-25 @doc/done | Backgrounders on **external** systems (Tessl, threat models, prior-art surveys) @doc/done | no @doc/done |
| ##ROW-DESIGN `design/` (this directory) @doc/done | Rationale for vibevm's **own** decisions — the why and the lore behind our PROPs @doc/done | no @doc/done |
| ##ROW-WAL [`WAL.md`](../WAL.md) @doc/done | Volatile current-state checkpoint, rewritten each session @doc/done | n/a @doc/done |
```

**The grammar the new row must match**, read off those six: `##ROW-<UPPER-SLUG>`
as the **first token of the first cell** (PROP-043 `##TABLE-ADDRESSING` — a
first-cell anchor addresses the row); the directory linked when it exists and
bare-backticked when it does not (`##ROW-DESIGN` is the precedent for the
unlinked form); **every cell closes with its own `@doc/done`**; `Normative?` is
`yes` / `no` / `n/a`.

### 2.2 The row, ready to paste {#row-draft}

**Placement: immediately after `##ROW-MODULES`.** The row exists to be *found*
by the search that produced the false verdict — obligation **F-233** in
[`run/state/routing.json`](../run/state/routing.json), where *«the prior verdict
searched for the artefact the flow forbids (a `spec/decisions/` directory,
against `##NO-SEPARATE-ADR-DIRECTORY`)»*. Sitting directly under the two rows
that **do** hold records makes the point in one glance: records live in those
two, not in a third directory.

```markdown
| ##ROW-NO-DECISIONS `decisions/` — **never created, and never will be** @doc/done | Nothing. A reopenable decision's four-field record lives **inside the `common/` or `modules/` section that governs the value** — the installed flow's `#NO-SEPARATE-ADR-DIRECTORY` and `#SUM-RECORDS-LIVE-AT-THE-ANCHOR`. Do not search for this directory; its absence is the design @doc/done | n/a @doc/done |
```

*(Alternative placement, if the owner prefers the live directories to run
uninterrupted: last row, after `##ROW-WAL`. Same text. The recommendation is
after `##ROW-MODULES`, for the reason above.)*

### 2.3 The closure line, ready to paste {#closure-draft}

A row states the absence; it does not explain the mutability, and **the host's
instance of the table has no `Mutability` column at all** — the flow's map
carries five columns (`Genre · Charter · Mutability · Reader · Authority`), the
host's three. So the mutability statement has to be prose. This is d10 §4.3's
own recommendation — *«the minimal honest change is a mutability statement, not
a row»* — kept alongside the row the owner ruled for, not instead of it.

**Placement: a new unit immediately after `##research-vs-design`
(`spec/design/README.md:24`), before Block 1's section.**

```markdown
##NO-DECISIONS-DIRECTORY **Decision records are a section form, not a directory.** A reopenable choice in [`common/`](../common/) or [`modules/`](../modules/) carries its four fields — Decision · Why · Considered and rejected · Revisit when — **inside the section that governs the value**, per `spec://org.vibevm.world/decision-records/flows/decision-records/DECISION-RECORDS-PROTOCOL#placement`. There is no `spec/decisions/` directory and there will not be one (`#NO-SEPARATE-ADR-DIRECTORY`); a silo *«preserves reasoning technically and loses it practically»* (`#SILOS-PRESERVE-TECHNICALLY-AND-LOSE-PRACTICALLY`), because nothing at the anchor points at it. The flow's map assigns the mutability **«amended by decision record»** to the *Foundational decisions* genre — `common/` in this instance; `modules/` keeps **«edit + changelog line»**. Which sections are owed a record is decided by [the criterion below](#owed-a-record), not by the presence of a `Decision` label. @doc/done
```

### 2.4 What this row does to d10's «inverted practice» finding {#inversion}

d10 §4.3 measured the inversion: the genre the map assigns records to
(`spec/common/`, mutability *«amended by decision record»*) held **35 Decision
labels and 0 complete records**, while `spec/modules/` — mutability *«edit +
changelog line»* — held the host's only four.

**The A′ backfill of [§3](#backfill) is exactly the correction.** Applying the
twelve records takes `spec/common/` from **0 complete to 12**, and the host's
practice stops contradicting the map it publishes. Worth saying in the ruling's
own commit message: the backfill's scope was argued *from this document*, and
this document is what it repairs.

### 2.5 NOTE — the flow's own map row is a package edit, not a host edit {#package-note}

**Not drafted here, deliberately.** The general taxonomy —
`packages/org.vibevm.world/spec-genres/v0.1.0/spec/flows/spec-genres/SPEC-GENRES-PROTOCOL.xml:43-47`,
whose `##ROW-GENRE-FOUNDATIONAL-DECISIONS` and `##ROW-GENRE-MODULE-CONTRACTS`
rows carry the `Mutability` column the host's instance drops — is **package
surface**. Any change to it is a package edit and rides the **release route**,
not the host route this batch works on: it needs a version bump, a re-publish,
and a re-materialise into every consumer, and it would put a vibevm-shaped rule
into a flow that other projects install.

Two consequences the boss should carry into the closure pass:

- **Nothing in Blocks 1–4 requires the package to change.** The host is the
  adopter; the flow already says everything the host needs, including the
  prohibition the new row publishes. The twelve anchors of d10 §5.0 close on
  route (b) — the host does the thing — with no package edit anywhere.
- **d10 §5.0's one package-side item stays open and is untouched by this
  ruling**: **F-224**'s self-falsifier, where `record-template.xml:45` defines
  the trigger as *«metric + threshold + observation point»* while its own
  `#example-library` twelve lines below ships a pure event trigger as the
  correct model. That is a `self` obligation on the package and is independent
  of everything in this file.

---

## §3 — Block 3 · the twelve `spec/common` backfill records {#backfill}

### 3.0 How these were drafted, and the four rules that constrained them {#drafting-rules}

- @fact:RULE-CITE-NEVER-RESTATE **A field cites the anchor that already carries the
  fact; it never restates it.** The corpus's most common defect — this
  campaign's single most-repeated finding — is a second writer for one fact.
  Nine of the twelve sections already carry their loser, their why, or both **in
  prose, unlabelled**; the insert adds the *label and the address*, pointing at
  the existing anchor. That is d10 §3.4's finding executed:
  **this is relabelling, not authoring.**
- @fact:RULE-SOURCED-OR-TODO **Sourced or `TODO(owner)`.** Every field below names
  where it came from. Three sources were admissible: the section's own prose,
  the PROP's `§7` history (`PROP-024` `##HIST-DRAFT-1` is the only one in
  `spec/common`), and the frozen [`VIBEVM-SPEC.md`](../../../VIBEVM-SPEC.md)
  where the section cites it. Nothing else.
- @fact:RULE-TRIGGERS-ARE-THE-OWNERS **Every trigger is marked OWNER CONFIRMS.**
  Eleven of the twelve need one written for the first time. A trigger names a
  signal *this project actually watches*
  (`#COLLECT-THE-SIGNAL-OR-REWRITE-THE-TRIGGER`), and a worker cannot decide
  that. Each proposal below names its **metric + threshold + observation point**
  so the owner is confirming a concrete thing, not approving a blank.
- @fact:RULE-NO-ANCHOR-IS-RENAMED **No existing anchor is renamed or reworded.** An
  anchor is an address; renaming one breaks every citation into it
  (`#NEVER-DELETE-A-RECORD-REWRITE-IT`). Every insert is **additive**. Where a
  section's existing units are already labelled `**Why:**` or
  `**When to revisit:**`, the draft says so and adds nothing.

**Anchor register — one open question, and it becomes visible in §3.1.** The
house style used 4-for-4 by the host's complete records
(`PROP-036#effective-load`, `PROP-043` ×3) is `##DECISION-<THING>` in UPPER plus
`##<thing>-{why,rejected,revisit}` in **kebab**. Every new unit below follows
it. But `PROP-000` §3 already carries `##LICENSE-OWNER-CALL` (a *Why*) and
`##LICENSE-REVISIT` (a *trigger*) in **UPPER** — so §3.1's insert places one
kebab unit beside two UPPER ones, in one section. That is d10 §4.2's open
sub-question — *«whether `##<thing>-revisit` should be `##REVISIT-<THING>`»* —
made concrete. **It is not resolved here**, and the drafts do not rename the two
UPPER units. **OWNER CONFIRMS** the register before these are applied.

**Progress markers.** Every new unit carries `@spec/done`, following the
`PROP-043` records' precedent for rationale units. If the owner prefers a
drafted-but-unconfirmed trigger to read as `@spec/plan` (*«intended, not
started»*, `##STATE-PLAN`) until it is confirmed, that is a one-token change per
unit — flagged for the boss, not decided here. **`vibe progress check` was not
run** (BACKLOG B-010: it writes zone state).

**Reading order.** The twelve run in census order. Each carries: the verdict and
the gate it passed, the section's **current text at `HEAD = 96b5b55f`**, which
of the four fields already exist, the **exact insert**, and the source of every
sourced claim.

---

### 3.1 `PROP-000` §3 `#license` — `##LICENSE-EULA` {#r-license}

**Verdict: R1 + R3.** Q1 — third-party licence terms are outside our
definitions and change without asking us. Q2 — a dependency's licence; a crate
to be published. Q3 — named below. **The best-equipped section in
`spec/common`: three of the four fields are already written and labelled.**

**Current text** (`spec/common/PROP-000.xml:47-57`, elided in the middle):

> - @fact:LICENSE-EULA **Decision:** vibevm ships under the **Universal Permissive
>   License 1.0** (UPL-1.0) — open source, relicensed 2026-07-12. … The
>   project's first phase shipped under a placeholder proprietary EULA; that
>   phase is over. @status:impl/done
> - @fact:NO-CRATES-IO Crates in this workspace set `license-file = "LICENSE.xml"`
>   and `publish = false` … @status:spec/done
>
> - @fact:LICENSE-OWNER-CALL **Why:** Owner's call, taken 2026-07-12 and executed
>   the same day: the whole shipped surface — the host tree and every
>   `packages/org.vibevm.*` package — carries UPL-1.0, so a consumer of any part
>   of vibevm gets one permissive licence and no per-package archaeology. @status:spec/done
> - @fact:LICENSE-SPEC-DEFERS `VIBEVM-SPEC.md` §1 explicitly defers the *produced*
>   software's license to the owner; the owner's choice is UPL-1.0. @status:spec/done
>
> …
>
> @fact:LICENSE-REVISIT **When to revisit:** the previous trigger — "when the owner
> decides to relicense (most likely UPL 1.0)" — **fired on 2026-07-12** and is
> spent. Re-open when either (a) a crate is to be published to crates.io: swap
> `license-file` for the SPDX string `license = "UPL-1.0"` and drop
> `publish = false`; or (b) a dependency or contribution arrives under terms
> UPL-1.0 cannot absorb. @status:spec/done

**Fields present: Decision ✔ · Why ✔ (`##LICENSE-OWNER-CALL`) · Revisit ✔
(`##LICENSE-REVISIT`). Missing: Considered and rejected — one unit.**

**Insert** — as the last bullet of the why group, after `##LICENSE-SPEC-DEFERS`
(`PROP-000.xml:51`):

```markdown
- ##license-rejected **Considered and rejected:** the placeholder **proprietary EULA** of the project's first phase — superseded 2026-07-12, *"that phase is over"* (`##LICENSE-EULA`); **per-package licensing** across the shipped surface — rejected because a consumer of any part of vibevm would then face per-package archaeology instead of one permissive licence (`##LICENSE-OWNER-CALL`); **any copyleft licence** (GPL / AGPL / LGPL) — never in contention, for the same reason it is forbidden in dependencies: it would force the whole product to relicense, *"which is exactly what UPL-1.0 exists to prevent"* (`##COPYLEFT-FORBIDDEN`, `##PROPRIETARY-TIGHTENS`). @spec/done
```

**Sources.** Loser 1 ← `##LICENSE-EULA`'s own last sentence. Loser 2 ←
`##LICENSE-OWNER-CALL`'s *«no per-package archaeology»*. Loser 3 ←
`##PROPRIETARY-TIGHTENS`, quoted.

**Revisit when — nothing to confirm.** `##LICENSE-REVISIT` already passes the
three-part test: two unambiguous external events, each checkable today
(`#EVENT-TRIGGERS-ARE-THE-SANCTIONED-NON-NUMERIC-VARIANT`), plus a **recorded
fired-and-spent prior trigger** — which is `#OUTCOME-CHANGED` and
`#CONSEQUENCE-EVOLUTION-IS-AN-EDIT` performed correctly by a project that was
not following the flow. **This is the only one of the twelve where the owner has
nothing to confirm.**

**One thing not derivable, recorded and left alone.** *Why UPL-1.0 rather than
MIT or Apache-2.0* is **NOT DERIVABLE** from this document.
`##LICENSE-SPEC-DEFERS` records only that `VIBEVM-SPEC.md` §1 defers the choice
to the owner, and `##LICENSE-OWNER-CALL` records that the owner made it. The
draft above therefore does **not** list MIT/Apache as considered-and-rejected,
because nothing on the record says they were. If the owner wants that comparison
on the record it is one sentence only the owner can write.

---

### 3.2 `PROP-000` §8 `#graph` — `##GRAPH-BUILTIN-NODES` {#r-graph}

**Verdict: R3.** Q1 — *«in v1»* is a stated scope limit, the exact form the
criterion asks for. Q2 — the frozen spec names the target release. Q3 — a
named alternative, explicitly deferred with its extension point documented ⇒
`##R3-LIVE-ALTERNATIVE`.

**Current text** (`spec/common/PROP-000.xml:124-128`):

> - @fact:GRAPH-BUILTIN-NODES **Decision:** Built-in nodes only in v1 (content-only
>   plugin contribution model per `VIBEVM-SPEC.md` §5.4). @status:spec/done
> - @fact:RUNNER-SEQUENTIAL Runner is sequential (no parallelism) in v1 per §5.2. @status:spec/done
> - @fact:TYPED-VALUES Typed value system per §5.3. @status:spec/done

**Fields present: Decision only. Missing: all three.**

**Insert** — three bullets after `##TYPED-VALUES` (`PROP-000.xml:126`), before
the `##WORKFLOWS-QUERIES` paragraph:

```markdown
- ##graph-nodes-why **Why:** the frozen `VIBEVM-SPEC.md` §5.4 states the constraint and its reason: v1's contribution model is *content-only* — *"a package materialises as a verbatim `vibedeps/` subtree and contributes a boot snippet, but does not contribute executable nodes. This keeps v1 small."* @spec/done
- ##graph-nodes-rejected **Considered and rejected:** **packages contributing executable / LLM nodes** (e.g. a flow adding a `wal:checkpoint` node bound after `build:compile`) — **deferred, not rejected**: `VIBEVM-SPEC.md` §5.4 targets v1.5 and directs *"document the extension point but do not implement it in v1."* Plugins influence the graph in v1 only by changing what content the built-in nodes operate on. @spec/done
- ##graph-nodes-revisit **Revisit when:** ⟨OWNER CONFIRMS — see the two candidates below⟩ @spec/done
```

**Sources.** Why and rejected are both quoted from
[`VIBEVM-SPEC.md` §5.4](../../../VIBEVM-SPEC.md) (`#plugin-contribution-model`),
which `##GRAPH-BUILTIN-NODES` already cites by name. Nothing was inferred.

**Revisit when — OWNER CONFIRMS.** Two candidates; the second is the more
honest and the weaker.

- **(a) Release-scoped, from the spec's own deferral.** *«The v1.5 milestone
  opens — `VIBEVM-SPEC.md` §5.4 names it as the target for the extension
  point.»* Metric: the milestone. Threshold: v1.5 declared. Observation point:
  the milestone list in `spec/WAL.xml`. **Fires on a schedule the project already
  keeps** — but it is a calendar, not a signal about the decision.
- **(b) Demand-scoped.** *«A published package needs a graph node the built-in
  set does not provide — observed as a `[hooks].post-install` doing work a node
  should do, or a `requires` no built-in node can satisfy.»* This is the real
  signal; its weakness is that no instrument watches for it today, which is
  precisely what `#COLLECT-THE-SIGNAL-OR-REWRITE-THE-TRIGGER` says to fix or
  rewrite.

*Recommendation to the owner: (a) as written, with (b) as its second clause —
whichever comes first. That keeps the trigger checkable in five minutes
(`#THE-FIVE-MINUTE-STRANGER-TEST`) without pretending to a signal nobody
collects.*

---

### 3.3 `PROP-000` §13 `#package-layout` — `##MIRROR-LAYOUT` {#r-mirror}

**Verdict: R3** (d10 sample #11). Q1 — the rule already carries one stated
exception, so its scope is bounded. Q2 — the manifest schema in `vibe-core`.
Q3 — the declined `target =` field is named on the very next line.

**Current text** (`spec/common/PROP-000.xml:170-181`, elided):

> - @fact:MIRROR-LAYOUT **Decision:** vibevm packages use a **mirror layout**. Every
>   entry in a package's `writes.files` is simultaneously (a) the path of the
>   file inside the package directory and (b) the path at which it will be
>   installed in the consumer's project. @status:spec/done
> - @fact:NO-TARGET-FIELD There is no separate `target = "…"` field per entry;
>   `writes.files` is the single source of truth for "where does this file go?" @status:spec/done
>
> …
>
> @fact:BOOT-SNIPPET-EXCEPTION **Boot snippets are the one exception.** The
> `[boot_snippet]` table carries an explicit `source` field naming the path
> inside the package (conventionally under `boot/`), while the target is always
> the fixed `spec/boot/<filename>`. @status:spec/done
>
> - @fact:MIRROR-WHY-DRIFT **Why:** a single source of truth for source-and-target
>   paths eliminates a whole class of authoring bug where the package layout
>   drifts from the declared writes. @status:spec/done
> - @fact:MIRROR-WHY-READABLE It also makes a package directory instantly
>   readable … @status:spec/done

**Fields present: Decision ✔ · Why ✔ (`##MIRROR-WHY-DRIFT`). Missing:
Considered and rejected (the loser is written at `##NO-TARGET-FIELD`, unlabelled)
and Revisit.**

**Insert** — two bullets after `##MIRROR-WHY-READABLE` (`PROP-000.xml:178`):

```markdown
- ##mirror-rejected **Considered and rejected:** a **per-entry `target = "…"` field** in `writes.files` — rejected because `writes.files` would stop being the single source of truth for *"where does this file go?"* (`##NO-TARGET-FIELD`), reviving the authoring-drift bug `##MIRROR-WHY-DRIFT` names and costing the package directory its at-a-glance readability (`##MIRROR-WHY-READABLE`). `[boot_snippet].source` is the **one retained exception**, not a rejection: its target is the fixed `spec/boot/<filename>` (`##BOOT-SNIPPET-EXCEPTION`). @spec/done
- ##mirror-revisit **Revisit when:** ⟨OWNER CONFIRMS⟩ a **second** source/target exception is proposed — i.e. any manifest table beyond `[boot_snippet]` needing an install path that differs from its in-package path. Observation point: the manifest schema in [`crates/vibe-core`](../../../crates/vibe-core/) — a per-entry target field appearing there *is* the fired state. One exception stands today; a second means the mirror rule is carrying less than `##MIRROR-LAYOUT` claims. @spec/done
```

**Sources.** Loser and its reason ← `##NO-TARGET-FIELD` + `##MIRROR-WHY-DRIFT`
+ `##MIRROR-WHY-READABLE`, all quoted. The retained exception ←
`##BOOT-SNIPPET-EXCEPTION`.

**Revisit when — OWNER CONFIRMS.** This is the cleanest three-part trigger in
the twelve and the only one that is fully mechanical: **metric** = the number of
source/target exceptions in the manifest schema, **threshold** = ≥ 2,
**observation point** = `crates/vibe-core`'s manifest types. A stranger answers
«has it fired?» by reading one schema.

---

### 3.4 `PROP-000` §15 `#dep-weight` — `##DEP-WEIGHT-NOT-FACTOR` {#r-dep-weight}

**Verdict: R3.** A process rule — `##ROW-OCCASION-PROCESS-RULE`, a sanctioned
occasion — whose **predecessor policy is named and whose reversal is already
recorded in the section**. Q1 — *«software of comparable surface area routinely
ships tens to hundreds of dependencies and remains fast and capable»* is an
external, checkable claim. Q2 — the release binary and the build clock. Q3 —
the footprint-rejection policy, with two recorded consequences.

**Current text** (`spec/common/PROP-000.xml:208-224`, elided):

> - @fact:DEP-WEIGHT-NOT-FACTOR **Decision:** Binary size, crate count, transitive
>   dep weight are NOT decision factors when selecting third-party libraries. @status:spec/done
> - @fact:PICK-STRONGEST Pick the strongest available library for the job … @status:spec/done
>
> - @fact:WHY-PRECEDENT **Why:** Software of comparable surface area (Chrome, modern
>   IDEs, production package managers) routinely ships tens to hundreds of
>   dependencies and remains fast and capable. @status:spec/done
> - @fact:WHY-DEBT Under-specifying a load-bearing component to save megabytes
>   creates ongoing architectural debt … @status:spec/done
>
> @fact:reject-reasons-lead **Legitimate reasons to reject a dep:** @status:spec/done
>
> - @fact:REJECT-LICENSE non-permissive license …, @fact:REJECT-ABANDONED abandoned
>   upstream, @fact:REJECT-SECURITY demonstrated security issues …,
>   @fact:REJECT-ERGONOMICS fundamentally bad API ergonomics …
>
> @fact:TOO-HEAVY-NOT-REASON "Too heavy" alone is **not** a reason. @status:spec/done
>
> - @fact:READMISSIBLE **Concrete consequences:** libraries previously rejected on
>   footprint grounds are re-admissible. Notable: `libsolv` …, `git2` … @status:spec/done
> - @fact:PROP-001-PRUNE The size-based argument in [PROP-001 §2.1] against `git2`
>   is to be pruned … @status:spec/done

**Fields present: Decision ✔ · Why ✔ (`##WHY-PRECEDENT`). Missing: Considered
and rejected (written unlabelled across `##READMISSIBLE` / `##PROP-001-PRUNE` /
the `##REJECT-*` list) and Revisit.**

**Insert** — two bullets after `##PROP-001-PRUNE` (`PROP-000.xml:224`), closing
the section:

```markdown
- ##dep-weight-rejected **Considered and rejected:** the **predecessor policy — reject a dependency on footprint** (binary size, crate count, transitive weight) — rejected, and its consequences already executed in this section: libraries previously refused on footprint grounds are re-admissible, `libsolv` and `git2` named (`##READMISSIBLE`), and PROP-001 §2.1's size-based argument against `git2` is marked for pruning (`##PROP-001-PRUNE`). Four grounds survive and are the *only* ones — licence, abandonment, demonstrated security issues, API ergonomics (`##REJECT-LICENSE` … `##REJECT-ERGONOMICS`); *"too heavy" alone is not a reason* (`##TOO-HEAVY-NOT-REASON`). @spec/done
- ##dep-weight-revisit **Revisit when:** ⟨OWNER CONFIRMS the two numbers⟩ the premise of `##WHY-PRECEDENT` — that weight does not cost us — stops holding, measured as either: the release `vibe` binary exceeding **⟨N⟩ MB**, or a clean `cargo build --release` exceeding **⟨M⟩ minutes** on the Windows dev box (`##DEV-ON-WINDOWS`). Observation point: the release artefact's size and `cargo build --timings`, both producible today. @spec/done
```

**Sources.** Every clause of the rejected field is quoted or cited from a unit
already in the section. Nothing added.

**Revisit when — OWNER CONFIRMS, and it needs two numbers.** The metric and the
observation point are real and already producible; **only the thresholds are
missing, and they are the owner's**. This is the one trigger in the twelve that
is genuinely numeric (`##R2-MEASURED-THRESHOLD` shape) rather than event-shaped.
If the owner declines to set numbers, the honest fallback is the event form:
*«a dependency is admitted whose weight measurably degrades a user-visible
surface — install time, first-run latency — recorded as a finding»*, which is
weaker but observable.

---

### 3.5 `PROP-000` §16 `#jtd` — `##JTD-SSOT` {#r-jtd}

**Verdict: R1 + R3.** Q1 — RFC 8927 and the pinned `jtd-codegen` binary are
upstream artefacts that change without asking us. Q2 — the upstream repository
and the pin in `tools/jtd-codegen/`. Q3 — named with its reason, one line
below the Decision.

**Current text** (`spec/common/PROP-000.xml:230-239`, elided):

> - @fact:JTD-SSOT **Decision:** JSON Type Definition (RFC 8927) schemas are the
>   single source of truth for every client/server and machine-to-machine
>   contract in this project. @status:spec/done
> - @fact:JTD-CODEGEN Rust types — and types in any future non-Rust clients — are
>   **generated** from JTD schemas via `jtd-codegen`, not hand-maintained. @status:spec/done
> - @fact:NO-DUPLICATION No client/server duplication is permitted on contracts. @status:spec/done
>
> - @fact:JTD-WHY-SKEW **Why:** duplication between a server contract and a
>   hand-written client is a classic source of version-skew bugs; schema-first
>   codegen eliminates that class of bug categorically. @status:spec/done
> - @fact:JTD-OVER-JSONSCHEMA JTD specifically (over JSON Schema / OpenAPI alone)
>   because JTD is deliberately narrower: its schema grammar is constructed so
>   every JTD schema maps to a clean static type in every target language, with
>   no language-specific escape hatches. @status:spec/done

**Fields present: Decision ✔ · Why ✔ (`##JTD-WHY-SKEW`). Missing: Considered
and rejected (written unlabelled at `##JTD-OVER-JSONSCHEMA`) and Revisit.**

**Insert** — two bullets after `##JTD-OVER-JSONSCHEMA` (`PROP-000.xml:235`):

```markdown
- ##jtd-rejected **Considered and rejected:** **JSON Schema / OpenAPI alone** — rejected: JTD is *"deliberately narrower: its schema grammar is constructed so every JTD schema maps to a clean static type in every target language, with no language-specific escape hatches"* (`##JTD-OVER-JSONSCHEMA`); **a hand-written client against each server contract** — rejected: that duplication is *"a classic source of version-skew bugs"* which codegen eliminates categorically (`##JTD-WHY-SKEW`, `##NO-DUPLICATION`). The boundary in the other direction is not a rejection: human-edited manifests stay TOML via `serde` — *"JTD is for wire, not for configs humans hand-edit"* (`##JTD-OUT-OF-SCOPE`). @spec/done
- ##jtd-revisit **Revisit when:** ⟨OWNER CONFIRMS⟩ either upstream fails us — **`jtd-codegen` ships no release for 24 months** (observation point: its upstream repository, version-pinned in `tools/jtd-codegen/` per `##TC-BINARY`) — **or** a contract listed in `##JTD-IN-SCOPE` proves inexpressible in JTD's grammar and would need an escape hatch, which is the property `##JTD-OVER-JSONSCHEMA` bought the narrowness for. Observation point for the second: the first schema in `schemas/` that cannot be written. @spec/done
```

**Sources.** Both losers quoted from `##JTD-OVER-JSONSCHEMA` and
`##JTD-WHY-SKEW`; the boundary clause from `##JTD-OUT-OF-SCOPE`.

**Revisit when — OWNER CONFIRMS.** The first clause is the flow's own worked
good trigger, `#ROW-TRIGGER-UPSTREAM-SILENT` — *«upstream ships no release for
24 months … Event; checkable from the repository today»* — reused with our
upstream in its slot. The second clause is the honest inverse of the why: the
narrowness was bought deliberately, so the trigger is the day it costs more than
it pays.

---

### 3.6 `PROP-018` §2.1 `#mode-is-backend` — `##MODE-INFERRED` **~ borderline** {#r-mode-inferred}

**Verdict: R3, marked `~`.** Q1 — *how vibevm was reached* is set by external
agents' invocation conventions, not by us. Q2 — the reach-detection path, and
`vibe-llm`'s arrival. Q3 — the loser is in the decision sentence.
**The `~` is on the Why**: it is sourced from adjacent units in the same PROP,
not from a measurement, which is a weaker why than the other eleven carry.
**This is the one section the owner can strike to reach eleven records.**

**Current text** (`spec/common/PROP-018-agentic-standalone-modes.xml:99-111`,
elided):

> - @fact:MODE-INFERRED **Decision.** Mode is not a global flag the user sets; it is
>   **inferred per operation from how vibevm was reached and what backend is
>   available.** @status:spec/done
> - @fact:OP-DECLARES An operation declares whether it needs inference; if it does,
>   the active backend decides the realisation: @status:spec/done
>
> - @fact:REACH-SUBPROCESS reached as a **subprocess of an agent** … → the **relay
>   backend** … @status:spec/done
> - @fact:REACH-STANDALONE-ENGINE reached **standalone** with a built-in engine
>   available (future) → the **built-in backend** … @status:spec/done
> - @fact:REACH-STANDALONE-NO-ENGINE reached **standalone** with no engine (today) →
>   a reasoning operation **fails loud** … @status:spec/done

**Fields present: Decision only. Missing: all three.**

**Insert** — three bullets after `##REACH-STANDALONE-NO-ENGINE`
(`PROP-018:111`):

```markdown
- ##mode-inferred-why **Why:** §1.2 `##UNIFYING-STATEMENT` fixes what a mode is — *"a mode is a choice of inference backend"* — and §2.3 `##AFFINITY-OF-WORK` fixes who chooses: *"Affinity is a property of the work, not a user choice."* Mode-by-inference is that same principle one level up: which backend can serve a call is a fact about the call's reach, not a preference. @spec/done
- ##mode-inferred-rejected **Considered and rejected:** **a global mode flag the user sets** (`--mode agentic|standalone` or a `vibe.toml` key) — rejected: a user could then name a backend the operation has no affinity for, which the dispatcher must refuse anyway (`##DISPATCHER-REFUSES`), or name one that does not exist on this machine, which today is every standalone reasoning call (`##REACH-STANDALONE-NO-ENGINE`). The flag would be a way to ask for a refusal. @spec/done
- ##mode-inferred-revisit **Revisit when:** ⟨OWNER CONFIRMS⟩ `BuiltinBackend` ships over `vibe-llm` (`##FB-BUILTIN-BACKEND`, `VIBEVM-SPEC.md` §10.4) — from that day two backends can both serve one standalone call, and *"what backend is available"* stops determining the answer on its own. Observation point: the far-backlog item closing, i.e. a `vibe-llm` inference path in the workspace. Second clause: a reach appears that the inference cannot classify — a persistent console (`##FB-CONSOLE`) or an invocation through a wrapper that hides the agent parentage — observed as a mis-chosen backend in a bug report. @spec/done
```

**Sources, stated plainly because this is the weak one.** The Why is
**adjacent-sourced**: neither `##UNIFYING-STATEMENT` (§1.2) nor
`##AFFINITY-OF-WORK` (§2.3) is *about* mode-inference; both are quoted verbatim
and the third sentence draws the parallel. The rejected field's *loser* is on
the record (the decision sentence names it); its *reason* is assembled from
`##DISPATCHER-REFUSES` and `##REACH-STANDALONE-NO-ENGINE`, both in this PROP.

**If the owner rejects the adjacency, the honest form of `##mode-inferred-why`
is `TODO(owner)`** — the flow's own re-derive step 3, *«Where you cannot source
a why, write TODO(owner) — never invent data»* — and the record then waits. It
should not be applied with a why the owner has not stood behind.

---

### 3.7 `PROP-018` §2.4 `#skill-decl` — `##SKILL-SECTION-NOT-KIND` {#r-skill-decl}

**Verdict: R3 — and demonstrably so.** This decision **has already been
reopened once, half of it went the other way, and the reversal is recorded in
place with a date and an owner resolution.** Nothing else in `spec/common`
carries that evidence.

**Current text** (`spec/common/PROP-018-agentic-standalone-modes.xml:156-170`,
elided), plus its loser from §5 (`:372`):

> - @fact:SKILL-SECTION-NOT-KIND **Decision.** A package declares which of its files
>   are **skills** for agents in a dedicated manifest section — **not** by
>   introducing a package kind of its own. The kind register (`package_ref.rs`,
>   `VIBEVM-SPEC.md` §4.1) stays closed to skills. @status:spec/done
> - @fact:ANY-KIND-RATIONALE Rationale: skills can live inside a package of *any*
>   kind and be structured any way. A `tool` package `vim` can ship the tool
>   itself **plus** a skill for driving vim — one self-contained package, two
>   artefact classes. Kind answers "what is this package"; the new section
>   answers "what does it project into an agent." @status:spec/done
> - @fact:MCP-HALF-SUPERSEDED This unit's original text sketched MCP servers as a
>   second any-kind section; that half is SUPERSEDED — MCP servers became their
>   own `mcp` kind with their own laws, owner resolution 2026-07-07:
>   [PROP-027](…). The skill law here is unchanged. @status:spec/done
>
> *(§5, line 372)* — @fact:OOS-FIFTH-KIND **A fifth package kind** — explicitly
> rejected (§2.4). @status:spec/done

**Fields present: Decision ✔ · a Why **under the wrong label** — `Rationale:` at
`##ANY-KIND-RATIONALE` · the loser at `##OOS-FIFTH-KIND`, in §5. Missing: a
labelled Why, a labelled Considered and rejected, and a trigger.**

**The label question, surfaced rather than decided.** `##ANY-KIND-RATIONALE`
*is* the why; only its label differs. Two ways to close it, and this is the
boss's call because one of them breaks this batch's additive-only posture
(`##RULE-NO-ANCHOR-IS-RENAMED`):

- **(b) recommended — a one-word in-place relabel**: `Rationale:` → `**Why:**`
  at `PROP-018:160`. The anchor `##ANY-KIND-RATIONALE` is untouched, so every
  citation stays live; only the field label changes, and the corpus stops
  carrying a third spelling for a field that already has two (d10 §4.2).
- **(a) strictly additive**: add a `##skill-decl-why` unit that cites
  `##ANY-KIND-RATIONALE` and adds nothing. Honest, but it makes a pointer unit
  whose only content is an address — a second writer for one fact in miniature.

**Insert** (assuming (b); with (a) the first bullet is added instead) — two
bullets after `##MCP-HALF-SUPERSEDED` (`PROP-018:170`):

```markdown
- ##skill-decl-rejected **Considered and rejected:** **a fifth package kind for skills** — explicitly rejected (`##OOS-FIFTH-KIND`): kind answers *"what is this package"* while the section answers *"what does it project into an agent"*, and skills can live inside a package of any kind (`##ANY-KIND-RATIONALE`). **MCP servers as a second any-kind section** — proposed in this unit's original text and **superseded**: they became their own `mcp` kind with their own laws, owner resolution 2026-07-07 ([PROP-027](../modules/vibe-mcp/PROP-027-mcp-packages.md), `##MCP-HALF-SUPERSEDED`). The skill law is unchanged; the MCP half is the counter-example that shows where the line falls. @spec/done
- ##skill-decl-revisit **Revisit when:** ⟨OWNER CONFIRMS⟩ an agent-installable artefact class arrives that needs **its own lifecycle laws** — install / uninstall semantics, resolution or conflict rules of its own — rather than only a projection path. That is exactly the state that fired for MCP servers on 2026-07-07 (`##MCP-HALF-SUPERSEDED`), so the trigger has a worked precedent. Observation point: the kind register in `crates/vibe-core` (`package_ref.rs`) and `VIBEVM-SPEC.md` §4.1 — whose `##INV-VOCABULARY` already anticipates `app`; the register growing is the fired state. @spec/done
```

**Sources.** Both losers and both reasons are quoted from
`##OOS-FIFTH-KIND` / `##ANY-KIND-RATIONALE` / `##MCP-HALF-SUPERSEDED`. The
trigger is **derived from the recorded firing**, not invented: the record itself
shows what condition moved an artefact class from «section» to «kind».

---

### 3.8 `PROP-018` §2.7 `#relay` — `##RELAY-PARKS` {#r-relay}

**Verdict: R3.** Q1 — *«MVP carries no write-back»* is a stated scope limit and
an explicit deferral. Q2 — the mailbox file itself. Q3 — the declined
alternative is named in a lead-in with **two** numbered reasons.

**Current text** (`spec/common/PROP-018-agentic-standalone-modes.xml:227-255`,
elided):

> - @fact:RELAY-PARKS **Decision.** When a reasoning operation runs under the relay
>   backend, it does not act. It writes an `Intent` … to a **single-slot
>   mailbox**, the project-local `.vibe/agentic/command.md` (§3), and returns a
>   pointer telling the caller to drain it. @status:spec/done
> - @fact:DRAIN-VERB The **consumer seam is one command**, `vibe command` … @status:spec/done
>
> @fact:two-step-lead Two properties make the two-step (produce → `vibe command`)
> worth its seam rather than just printing the intent from the producer: @status:spec/done
>
> 1. @fact:SEAM-UNIFORMITY **Uniformity.** *Any* vibevm command that discovers
>    mid-run it needs reasoning parks an intent the same way … The agent learns
>    one drain verb, not per-command stdout parsing. @status:spec/done
> 2. @fact:SEAM-DECOUPLING **Decoupling.** Producer and consumer need not be the
>    same invocation … @status:spec/done
>
> - @fact:NO-WRITE-BACK **MVP carries no write-back** (`req r4`): the relay is
>   fire-and-forget. @status:spec/done
> - @fact:AGENT-ORCHESTRATES The calling agent orchestrates the conversation … @status:spec/done
> - @fact:SKILL-STATES-NO-CHANNEL The installed skill (§2.9) states this contract
>   explicitly … (Full bidirectional conversations are §6.) @status:spec/done

**Fields present: Decision ✔ · Why and loser both written, both unlabelled
(`##two-step-lead` + `##SEAM-UNIFORMITY` + `##SEAM-DECOUPLING`). Missing: all
three labels.**

**Insert** — three bullets after `##SKILL-STATES-NO-CHANNEL` (`PROP-018:255`):

```markdown
- ##relay-why **Why:** the two-step seam buys two properties a direct print cannot, both stated at `##two-step-lead`: **uniformity** — any command that discovers mid-run it needs reasoning parks an intent the same way, so an agent learns one drain verb rather than per-command stdout parsing (`##SEAM-UNIFORMITY`); and **decoupling** — producer and consumer need not be the same invocation, which is what lets a future deterministic command park a reasoning step and exit (`##SEAM-DECOUPLING`). @spec/done
- ##relay-rejected **Considered and rejected:** **printing the intent directly from the producer**, with no `vibe command` seam — rejected for the two reasons above; it is cheaper by one command and forfeits both. **A return channel (write-back)** — **deferred, not rejected**: the MVP relay is fire-and-forget (`##NO-WRITE-BACK`), the calling agent orchestrates (`##AGENT-ORCHESTRATES`), and full bidirectional conversations are parked at §6 `##FB-CONVERSATIONS`. @spec/done
- ##relay-revisit **Revisit when:** ⟨OWNER CONFIRMS⟩ either the **single slot overflows** — a producer runs while `.vibe/agentic/command.md` already holds an undrained intent, which the mailbox's own shape makes mechanically observable (`##FILE-COMMAND-MD`) — **or** the fire-and-forget contract starts costing a round trip, observed as an agent issuing a follow-up `vibe …` command whose only purpose is to hand a result back (the case `##SKILL-STATES-NO-CHANNEL` tells agents not to expect). Either fires §6's `##FB-CONVERSATIONS`. @spec/done
```

**Sources.** Every clause quoted or cited from `##two-step-lead`,
`##SEAM-UNIFORMITY`, `##SEAM-DECOUPLING`, `##NO-WRITE-BACK`,
`##AGENT-ORCHESTRATES`, `##SKILL-STATES-NO-CHANNEL`. **Nothing was authored** —
this record is pure relabelling except the trigger.

---

### 3.9 `PROP-024` §2.2 `#shippable-tree` — `##SHIPPABLE-TREE-DEF` {#r-shippable}

**Verdict: R1** (d10 sample #31, and the criterion's own published worked
example — [§1.2](#criterion-text) `##ex-shippable-tree`). Q1 — four literal
build-output directory names from four external toolchains. Q2 — the project
**already takes the measurement**: `##ACC-HASH-EXCLUDES` asserts *«identical
source produces an identical hash regardless of build state»*. Q3 — two losers,
both already written with reasons, in §4.

**Current text** (`spec/common/PROP-024-code-bearing-packages.xml:116-143`,
elided):

> @fact:SHIPPABLE-TREE-DEF **Decision.** A package's **shippable tree** is its
> directory minus a build-output denylist: @status:spec/done
>
> ```
> .git/        .vibe/        target/        node_modules/
> ```
>
> - @fact:VIBEIGNORE-EXTENDS plus any glob listed in an optional `.vibeignore` … @status:spec/done
> - @fact:SHIPPABLE-CONSUMERS The `content_hash` …, the snapshot copy …, and the
>   verbatim materialised slot … all operate over the **shippable tree** … @status:spec/done
>
> - @fact:WHY-SOURCE-IDENTITY **Why.** Identity is the *source*, never build
>   artifacts: build output is non-deterministic (timestamps, host paths,
>   incremental state) and may be gigabytes — hashing or copying it would make
>   identity unstable and materialisation ruinous … @status:spec/done
> - @fact:SOURCE-IS-SHIPPED … @status:spec/done
> - @fact:DENYLIST-NOT-SELECTION The denylist formalises "what was never source", it
>   does not introduce selection. @status:spec/done
>
> *(§4, lines 279, 297)* — @fact:REJ-HASH-BUILD-OUTPUT **Hash and copy build output
> too** — rejected: non-deterministic and potentially gigabytes … ·
> @fact:REJ-ALLOW-LIST **A per-file `[ship]`/`[files]` allow-list in the manifest**
> — rejected: it resurrects the per-file write list PROP-009 §2.6 retired …

**Fields present: Decision ✔ · Why ✔ (`##WHY-SOURCE-IDENTITY`) · both losers
written with reasons, but in §4. Missing: a labelled Considered and rejected at
the governing anchor, and a trigger.**

**Insert** — two bullets after `##DENYLIST-NOT-SELECTION`
(`PROP-024:143`):

```markdown
- ##shippable-rejected **Considered and rejected:** **hashing and copying build output too** — rejected: non-deterministic and potentially gigabytes, the file-count/byte-count failure PROP-022 §1.1 exists to avoid (`##REJ-HASH-BUILD-OUTPUT`); **a per-file `[ship]` / `[files]` allow-list in the manifest** — rejected: it resurrects the per-file write list PROP-009 §2.6 retired, and a denylist keeps *"what ships" == "the source"*, preserving the verbatim guarantee (`##REJ-ALLOW-LIST`, `##VERBATIM-PRESERVED`). @spec/done
- ##shippable-revisit **Revisit when:** ⟨OWNER CONFIRMS⟩ a package ships in a language whose build output the four-name denylist does not cover and `.vibeignore` alone cannot carry — Python (`__pycache__/`, `.venv/`), JVM (`build/`), Go (`vendor/`) are the near candidates. **The fired state is mechanically observable and already asserted:** a package whose `content_hash` differs between a clean and a built checkout, which is exactly what `##ACC-HASH-EXCLUDES` tests. Observation point: that acceptance check, run over the published package set. @spec/done
```

**Sources.** Both losers quoted verbatim from §4. The trigger's observation
point is the project's **own existing acceptance criterion** — the strongest
form `#COLLECT-THE-SIGNAL-OR-REWRITE-THE-TRIGGER` allows, because the signal is
already collected.

---

### 3.10 `PROP-024` §2.4 `#consume` — `##OWN-WORKSPACE` {#r-own-workspace}

**Verdict: R1 + R3 — the best-equipped reopenable section in `spec/common`.**
The loser is named, reasoned, **retained as a live fallback**, and given an
evidentiary rule for its revival. Q1 — Cargo's own constraint plus Windows path
handling. Q2 — a spike this project already ran. Q3 — `##REJ-CONSUMER-MEMBERS`.

**Current text** (`spec/common/PROP-024-code-bearing-packages.xml:175-211`,
elided):

> - @fact:OWN-WORKSPACE **Decision.** A code-bearing package carries its **own**
>   workspace manifest (for Rust, a root `Cargo.toml` with `[workspace]`) — it
>   is a standalone, independently-buildable project. @status:spec/done
> - @fact:PATH-DEP-LAW A language-native consumer that needs a shipped crate …
>   references it **by path into the materialised slot** … @status:spec/done
>
> - @fact:WORKSPACE-EXCLUDE The consumer **excludes** `vibedeps/` … so the slot's
>   crates belong to the *package's* workspace, not the consumer's — Cargo
>   forbids a directory living in two workspaces, and this is the standard
>   resolution for a repo that contains a sub-project with its own workspace. @status:spec/done
> - @fact:PIN-ONCE The slot path is version-qualified; pinning it once … means a
>   package version bump touches a single line. @status:spec/done
>
> - @fact:SPIKE-FIRST **Spike before the irreversible move.** Cross-workspace
>   path-deps and the `exclude` topology are validated empirically on the target
>   host (Windows, where `canonicalize()` adds a `\\?\` prefix and Cargo path
>   handling has sharp edges) *before* any crate is physically relocated. @status:spec/done
> - @fact:SPIKE-FALLBACK The fallback, if cross-workspace path-deps prove unworkable
>   on a host, is §4's rejected-but-retained alternative (the consumer adds the
>   slot crates as its own workspace members) — chosen only on evidence. @status:spec/done
>
> *(§4, line 282)* — @fact:REJ-CONSUMER-MEMBERS **Consumer adds the slot crates as
> its own workspace members** … — *considered*; rejected as the primary model
> because it denies the package standalone-buildability (a package would not be
> a project) and couples the consumer's workspace membership to generated
> `vibedeps/` state. **Retained as the §2.4 fallback** …

**Fields present: Decision ✔ · Why written unlabelled (`##WORKSPACE-EXCLUDE`) ·
loser written with reason and retention (`##REJ-CONSUMER-MEMBERS`,
`##SPIKE-FALLBACK`). Missing: all three labels; the trigger is half-written.**

**Insert** — three bullets after `##SPIKE-FALLBACK` (`PROP-024:211`):

```markdown
- ##own-workspace-why **Why:** an external constraint, not a preference — *"Cargo forbids a directory living in two workspaces, and this is the standard resolution for a repo that contains a sub-project with its own workspace"* (`##WORKSPACE-EXCLUDE`). Giving the package its own workspace is what makes it *"a standalone, independently-buildable project"* (`##OWN-WORKSPACE`) and keeps a version bump to one pinned line (`##PIN-ONCE`). @spec/done
- ##own-workspace-rejected **Considered and rejected:** **the consumer adding the slot crates as its own workspace members** (no package workspace, no cross-workspace path-dep) — *considered*, and rejected as the **primary** model because it denies the package standalone-buildability and couples the consumer's workspace membership to generated `vibedeps/` state (`##REJ-CONSUMER-MEMBERS`). **Retained, not discarded**: it is the §2.4 fallback, *"chosen only on evidence"* (`##SPIKE-FALLBACK`). @spec/done
- ##own-workspace-revisit **Revisit when:** ⟨OWNER CONFIRMS⟩ cross-workspace path-deps prove unworkable on a supported host — the condition `##SPIKE-FALLBACK` already names, here given its observation point: a clean-checkout `cargo build` failing to resolve `vibedeps/<slot>/crates/<crate>` on any of the three platforms of PROP-000 §11 `##PLATFORMS-TRIO`, Windows first (`##SPIKE-FIRST`). The fired state has a landing place already specified, so reopening is a switch, not a redesign. @spec/done
```

**Sources.** Why quoted from `##WORKSPACE-EXCLUDE` and `##PIN-ONCE`; the
rejected field quoted from `##REJ-CONSUMER-MEMBERS` and `##SPIKE-FALLBACK`. The
trigger **restates the section's own stated condition and adds only the
observation point** — the one thing `##SPIKE-FALLBACK` leaves unsaid.

---

### 3.11 `PROP-024` §2.5 `#bootstrap` — `##SELF-HOST-VENDORED` {#r-bootstrap}

**Verdict: R3.** Q1 — crates.io publication is an **explicit deferral**, and
`##R3-LIVE-ALTERNATIVE` makes an explicit deferral always R3. Q2 — the
committed slot's own weight, measurable today. Q3 — one rejected and one
deferred alternative, both in §4.

**Current text** (`spec/common/PROP-024-code-bearing-packages.xml:217-230`,
elided):

> - @fact:SELF-HOST-VENDORED **Decision.** vibevm consumes its own discipline
>   toolchain from the **committed** `vibedeps/` slot. @status:spec/done
> - @fact:CLEAN-CLONE-BUILDS Because `vibedeps/` is committed (PROP-009 §2.1), a
>   fresh clone builds from a clean checkout **with no prior `vibe install`** …
>   already exists in the tree. @status:spec/done
> - @fact:NO-CHICKEN-EGG There is no chicken-and-egg: the toolchain a build needs is
>   vendored beside the code that needs it. @status:spec/done
>
> *(§4, lines 289, 293)* — @fact:REJ-IN-PLACE **`materialization = "in-place"` for
> tool packages** — rejected: `in-place` slots are `.gitignore`d and unversioned
> …; the discipline toolchain must be **vendored and versioned** so a clone is
> buildable offline (§2.5). · @fact:REJ-CRATES-IO **Publish the tool crates to
> crates.io and depend on the published versions** — deferred: the installed
> package *is* the distribution … crates.io publication is an optional later
> convenience for non-vibe Rust consumers, not a requirement of this model.

**Fields present: Decision ✔ · Why written unlabelled (`##CLEAN-CLONE-BUILDS` +
`##NO-CHICKEN-EGG`) · one rejection and one deferral in §4. Missing: all three
labels.**

**Insert** — three bullets after `##NO-CHICKEN-EGG` (`PROP-024:224`), before
the `##DEV-LOOP-MUTABLE` paragraph:

```markdown
- ##bootstrap-why **Why:** committing the slot is what makes a fresh clone build *"from a clean checkout with no prior `vibe install`"* — the path-dep target already exists in the tree (`##CLEAN-CLONE-BUILDS`), so there is no chicken-and-egg between the toolchain a build needs and the build that would fetch it (`##NO-CHICKEN-EGG`). Acceptance already asserts it: `##ACC-CLEAN-CLONE`. @spec/done
- ##bootstrap-rejected **Considered and rejected:** **`materialization = "in-place"` for the tool packages** — rejected: `in-place` slots are `.gitignore`d and unversioned (PROP-022 §2.4/§2.7), and the toolchain must be vendored and versioned so a clone is buildable offline (`##REJ-IN-PLACE`). **Publishing the tool crates to crates.io and depending on the published versions** — **deferred, not rejected**: *"the installed package is the distribution"*, and crates.io publication is *"an optional later convenience for non-vibe Rust consumers, not a requirement of this model"* (`##REJ-CRATES-IO`). @spec/done
- ##bootstrap-revisit **Revisit when:** ⟨OWNER CONFIRMS — one of two, see below⟩ @spec/done
```

**Sources.** Why quoted from `##CLEAN-CLONE-BUILDS` / `##NO-CHICKEN-EGG` /
`##ACC-CLEAN-CLONE`; both alternatives quoted from §4.

**Revisit when — OWNER CONFIRMS, two candidates that watch opposite costs.**

- **(a) the cost of the guarantee, measurable.** *«The committed `vibedeps/`
  slot exceeds ⟨N⟩ MB or ⟨M⟩ files, making a clean clone expensive enough to
  outweigh the offline-buildability it buys.»* Observation point:
  `git count-objects -vH` and the slot's size on disk — both producible today.
  This watches the **why's own price**, which is the honest place to watch.
- **(b) the deferral's demand.** *«A Rust consumer outside the vibe ecosystem
  needs `conform` / `specmap` without installing a vibe package»* — the exact
  case `##REJ-CRATES-IO` deferred. Weaker: **no instrument watches for it**, so
  by `#COLLECT-THE-SIGNAL-OR-REWRITE-THE-TRIGGER` it needs either a place such a
  request is recorded (the `F-NNN` finding space would do) or a rewrite.

*Recommendation: (a) as the trigger, with (b) named in its second clause and
pointed at the findings ledger so it has somewhere to be observed.*

---

### 3.12 `PROP-024` §2.6 `#placement` — `##PLACEMENT-LAW` {#r-placement}

**Verdict: R3 — and this section is a finding in its own right.** It carries a
revisit trigger that was **written, fired, and honoured in place, by a project
not following the flow.** d10 §2 `#ex-calibration` found the `spec/modules`
twin (`PROP-005` `##WORKSPACE-MEMBER`, revised 2026-05-22) and reported it as
the host's one correct reopening. **This is its `spec/common` counterpart, and
d10 did not name it.** Per `#OUTCOME-REAFFIRMED` — *«a fired trigger is
spent»* — what this record needs is not a first trigger but a **fresh** one.

**Current text** (`spec/common/PROP-024-code-bearing-packages.xml:236-259`,
elided), plus the §7 history line (`:338`):

> @fact:PLACEMENT-LAW **Decision.** The discipline's tools are code and obey the
> four-layer model: L4 (implemented checkers) ships in the package whose
> language they check. @status:spec/done
>
> - @fact:THIS-PASS-WHOLE-TOOLCHAIN For **this pass**, the **entire Rust discipline
>   toolchain** … ships in `stack:org.vibevm.ai-native/rust-ai-native-lang`. Its
>   centre of gravity is Rust, and shipping the toolchain whole avoids carving
>   language-neutral cores out under time pressure. @status:spec/done
> - @fact:CORE-STAYS-PROMPT-ONLY **The condition fired.** `flow:…/core-ai-native`
>   was to stay **prompt-only** … until a second language actually needed the
>   shared engine. The TypeScript pilot shipped, so core-ai-native now **authors
>   the neutral engines** …, which each `-lang` and `-mcp` package vendors
>   byte-identically. @status:impl/done
> - @fact:DEFERRED-ENGINE-SPLIT **Deferred (documented):** the language-neutral
>   conform engine (`conform-core`) is a genuine L1 artifact … taken when the
>   first non-Rust pilot needs it (YAGNI until then). … **Executed** — the
>   TypeScript pilot was that demand, and the neutral halves now live in
>   core-ai-native, vendored into each family by `cargo xtask sync-engines`. @status:impl/done
>
> *(§7, line 338)* — @fact:HIST-DRAFT-1 **2026-06-27 — draft 1.** … `conform-core`
> ships in the Rust stack now with the L1 engine-extraction deferred (§2.6). @status:spec/done

**Fields present: Decision ✔ · a deferral with its condition, its firing and its
outcome, all in place. Missing: a labelled Why, a labelled Considered and
rejected, and — because the old one is spent — a **new** trigger.**

**Insert** — three bullets after `##DEFERRED-ENGINE-SPLIT` (`PROP-024:259`):

```markdown
- ##placement-why **Why:** the toolchain's *"centre of gravity is Rust, and shipping the toolchain whole avoids carving language-neutral cores out under time pressure"* (`##THIS-PASS-WHOLE-TOOLCHAIN`); the layer model then decides placement rather than convenience — L4 ships with the language it checks. Recorded the same session the decision was taken (§7 `##HIST-DRAFT-1`, 2026-06-27). @spec/done
- ##placement-rejected **Considered and rejected:** **extracting the language-neutral cores (`conform-core`, the neutral half of `specmap-core`) up into core-ai-native in the same pass** — **deferred, not rejected**, with its condition stated: *"taken when the first non-Rust pilot needs it (YAGNI until then)"* (`##DEFERRED-ENGINE-SPLIT`). **The deferral has since been honoured**: the TypeScript pilot was that demand, the condition fired, and the neutral engines now live in core-ai-native, vendored by `cargo xtask sync-engines` (`##CORE-STAYS-PROMPT-ONLY`). @spec/done
- ##placement-revisit **Revisit when:** ⟨OWNER CONFIRMS — the previous trigger fired and is spent⟩ a **third** language family arrives and the neutral engines do not cover it — observed as `cargo xtask sync-engines` being unable to vendor a core byte-identically into the new `-lang` package, or a third family needing a core the two existing ones do not share. Observation point: the `sync-engines` task and the set of `*-ai-native-lang` / `*-ai-native-mcp` packages. @spec/done
```

**Sources.** Why quoted from `##THIS-PASS-WHOLE-TOOLCHAIN`, dated from §7
`##HIST-DRAFT-1` — **the only `spec/common` record whose why is corroborated by
a version-history line**, which is exactly the `#WRITE-IN-THE-SESSION-THAT-DECIDES`
evidence the other eleven lack. The deferral, its condition and its firing are
quoted from `##DEFERRED-ENGINE-SPLIT` and `##CORE-STAYS-PROMPT-ONLY`.

**The trigger is a replacement, not a first.** `#OUTCOME-REAFFIRMED` requires a
*new* trigger after a firing; the draft above sets one and says why the old one
is gone. **The owner is confirming a successor**, which is a different question
from the other ten and should be read as one.

---

### 3.13 Carve-out — a reopenable claim that is not a `Decision`-labelled section {#carve-out}

**Offered, not counted.** The ruling scopes the backfill to `spec/common`'s
**35 `Decision`-labelled sections**; this claim is not one of them, so it is
outside the twelve. It is raised because d10's criterion says
*«@fact:CRITERION-UNIT-IS-THE-CLAIM — the classification unit is the claim, not the
section»*, and this is the sharpest instance in `spec/common`.

**`PROP-000` §7 `#registry`.** The section's `Decision` label is `##REG-M0`
(*«M0: local-directory registry only»*), classified **N3** at row 7 — a
milestone-scope statement, superseded in place by `##REG-M1`. But the section
also carries this, unlabelled (`PROP-000.xml:108-113`):

> @fact:SPLIT-HOST-POSTURE **Source repositories — split-host posture.** The vibevm
> project and the package registry live on **separate hosts** by deliberate
> decision (2026-04-29). …
>
> - @fact:REG-MIGRATION-WHY The migration from `git@gitverse.ru:vibespecs/*`
>   happened on 2026-04-29 because GitVerse's public REST API does not expose
>   org-scoped repo creation (`POST /orgs/{org}/repos` returns 404 / WAF 403;
>   documented exhaustively in [PROP-002 §2.10] and
>   `crates/vibe-publish/src/gitverse.rs`). Without that endpoint
>   `vibe registry publish` cannot fully drive the publish loop end to end. @status:spec/done
> - @fact:REG-GITHUB-WORKS GitHub's equivalent endpoint works natively, so the
>   registry organization moved while the vibevm project repository stays put. @status:spec/done
> - @fact:REG-HASH-STABLE Identity is content-hashed (PROP-002 §2.1) — the lockfile's
>   `source_url` rotates but no `content_hash` value is invalidated by the host
>   change. @status:spec/done

**All three gates pass, and the why is the best-evidenced in `spec/common`:** a
named upstream endpoint, its two failure codes, the file that documents it, and
the consequence for a specific command. Q1 — a third party's API surface. Q2 —
`POST /orgs/{org}/repos` against GitVerse, checkable in one request. Q3 — the
prior host, with the reason it lost.

**What it needs is a trigger, and the trigger writes itself:** *«GitVerse's
public REST API gains org-scoped repo creation — `POST /orgs/{org}/repos`
returns 2xx»*. That is metric + threshold + observation point, answerable by a
stranger in five minutes, and it is the *only* condition under which the split
would be reconsidered.

**Left to the owner as an explicit choice**, not folded in: adopting it means
either (i) accepting that the backfill's unit is the claim rather than the
`Decision`-labelled section — which changes the scope the ruling set — or
(ii) promoting `##SPLIT-HOST-POSTURE` to a `Decision` label, which changes the
census population from 35 to 36 and every count derived from it. **Neither is a
worker's call.**

---

## §4 — Block 4 · the forward-only rule {#forward-rule}

**Home: the same section as the criterion** — the closing unit of
`## Which decisions are owed a record` in
[`spec/design/README.md`](../../../spec/design/README.md), after
`##HONEST-NEVER-IS-NOT-A-DEFECT`. The criterion answers *which*; this answers
*when*, and separating them would let a reader meet the test without meeting the
scope it is applied under.

```markdown
##FORWARD-ONLY-FROM-THE-RULING **The four fields are required forward-only, and existing sections are classified rather than edited.** From this ruling forward, any decision **newly taken or reopened** in [`common/`](../common/) or [`modules/`](../modules/) that passes the criterion above carries all four fields **at the moment it is minted** — same session, before it ends (`#WRITE-IN-THE-SESSION-THAT-DECIDES`), because the reasoning is in working memory then and nowhere afterwards. Sections written before the ruling are **classified by the criterion, not rewritten**: the reason is the flow's own, not a budget — *"a record backfilled a week later is fiction with confidence"* (`#backfilled-reasoning-is-fiction`), and a *why* or a trigger reconstructed from a document by someone who was not in the room is `#ROW-ANTI-BACKFILLED-MEMORY`, not a record. The one scheduled exception is [`common/`](../common/), the genre the installed map already declares *"amended by decision record"*: its reopenable sections were classified and backfilled in full, from their own recorded prose, in one pass. Everywhere else the debt drains opportunistically — `#RHYTHM-OPPORTUNISTIC`: a session that edits a `Decision`-bearing section completes its record **if the reasoning is already in the document**, and otherwise files the gap rather than inventing one. A record whose *why* cannot be sourced is not written; it is asked (`#ASK-RATHER-THAN-INVENT-DATA`). @doc/done
```

**What each clause is doing, so the boss can check it against the ruling.**

| clause | what it binds | its ground |
|---|---|---|
| four fields at minting, same session | new + **reopened** decisions | `#WRITE-IN-THE-SESSION-THAT-DECIDES` |
| existing sections classified, not rewritten | the ≈ 94 `spec/modules` non-reopenable and the 23 `spec/common` settled | `#backfilled-reasoning-is-fiction` — a *reason*, not a budget |
| `common/` is the one scheduled exception, done in one pass | the twelve of [§3](#backfill) | the installed genre map's declared mutability (d10 §4.3) |
| everywhere else drains opportunistically | `spec/modules`' ≈ 60 reopenable | `#RHYTHM-OPPORTUNISTIC`, and d10 §5's «cheap amendment worth naming» |
| an unsourceable why is asked, not written | every future record | `#ASK-RATHER-THAN-INVENT-DATA` |

**Two things the paragraph deliberately does not say.**

- **It does not say `spec/modules` is exempt.** The four-field requirement binds
  there from the ruling forward exactly as in `common/`; what is *not* scheduled
  is its backfill. d10 §5's option C — a marked exception excusing the module
  genre from the field rule — was **not** what the owner ruled, and no wording
  here should let a later reader read it that way. This matters because §3.6 of
  the batch plan warns that the easy drift direction is *«quietly rewriting the
  discipline to describe a lax consumer»*.
- **It does not add a "revisit: never" marker to settled sections.** That is
  `##COUNT-IS-DERIVED-NOT-MAINTAINED` in [Block 1](#criterion-text), and it is
  what makes the negative classification derivable instead of costing ≈ 94
  edits that would each read as a defect.

---

## §5 — What this asks, and what it does not touch {#ask}

### 5.1 The eleven owner confirmations, in one list {#confirmations}

| # | section | what is being confirmed | shape |
|---|---|---|---|
| — | `##LICENSE-EULA` | **nothing** — `##LICENSE-REVISIT` already passes | — |
| 1 | `##GRAPH-BUILTIN-NODES` | (a) the v1.5 milestone, or (a)+(b) with demand as the second clause | event |
| 2 | `##MIRROR-LAYOUT` | a second source/target exception appearing in the `vibe-core` manifest schema | count ≥ 2 |
| 3 | `##DEP-WEIGHT-NOT-FACTOR` | **two numbers** — binary-size MB and clean-build minutes | numeric |
| 4 | `##JTD-SSOT` | 24-month upstream silence, or the first inexpressible schema | event |
| 5 | `##MODE-INFERRED` ~ | the trigger **and** whether the adjacent-sourced why stands, or becomes `TODO(owner)` | event + a why call |
| 6 | `##SKILL-SECTION-NOT-KIND` | an artefact class needing its own lifecycle laws (precedent: 2026-07-07) | event |
| 7 | `##RELAY-PARKS` | mailbox overflow, or a round-trip forced by fire-and-forget | event |
| 8 | `##SHIPPABLE-TREE-DEF` | a language the four-name denylist misses, tested by `##ACC-HASH-EXCLUDES` | event |
| 9 | `##OWN-WORKSPACE` | cross-workspace path-deps failing on a supported host | event |
| 10 | `##SELF-HOST-VENDORED` | (a) slot size / file count, or (b) an outside-consumer request | numeric or event |
| 11 | `##PLACEMENT-LAW` | a **successor** trigger — the previous one fired and is spent | event |

Plus **four rulings that are not triggers**, each raised in place above:

1. **The criterion's home** — `spec/design/README.md` beside the genre table
   ([§1.1](#home)), or a normative `PROP-000` section if the owner wants it
   binding in its own right rather than as a localisation of an
   already-booted flow.
2. **The anchor register** ([§3.0](#drafting-rules)) — kebab
   `##<thing>-{why,rejected,revisit}` per the host's 4-for-4 house style, which
   places one kebab unit beside `PROP-000` §3's two UPPER ones. d10 §4.2's open
   sub-question, now concrete.
3. **`##ANY-KIND-RATIONALE`'s label** ([§3.7](#r-skill-decl)) — a one-word
   in-place relabel `Rationale:` → `**Why:**`, or a strictly additive pointer
   unit. The first breaks this batch's additive-only posture; the second creates
   a unit whose only content is an address.
4. **The carve-out** ([§3.13](#carve-out)) — whether `##SPLIT-HOST-POSTURE`
   joins the backfill, which would change the census population from 35 to 36.

### 5.2 What was not done, named so nothing is credited with it {#not-done}

- **Nothing was applied.** Every block above is a draft in this file. No file in
  `spec/`, `packages/`, or `campaigns/*/run/` was touched;
  `git status --porcelain` was empty at batch start and only this file is new.
- **`vibe progress check --campaign` was not run** — BACKLOG **B-010**: it
  writes zone state.
- **No verdict moved and no anchor was re-judged.** The twelve anchors of d10
  §5.0 (**F-197**, **F-198**, **F-224**, **F-225**, **F-299**, **F-233**) are
  untouched. Their closure is d10 §6.1 item 4's step and runs *after* the owner
  confirms: re-judge, then `merge-verdicts.py <slice.json> --force`, read the
  output, then `vibe progress seal` — **never chained**
  (`##NEVER-CHAIN-MERGE-AND-SEAL`).
- **F-224's package-side self-falsifier stays open**, untouched by any of this
  ([§2.5](#package-note)).
- **`BACKLOG.md` #b-007 was not updated.** It is `open` *«as a question to answer
  rather than work to schedule»*; the owner's ruling answers it, and recording
  that is the same closure pass, not this batch.

### 5.3 Two findings this batch produced that are not part of the four blocks {#findings}

- @fact:FINDING-PLACEMENT-LAW-IS-A-CALIBRATION-CASE **`PROP-024` `##PLACEMENT-LAW`
  is a second worked instance of a correctly-honoured revisit trigger, and d10
  missed it.** d10 §2 `#ex-calibration` reports one — `PROP-005`
  `##WORKSPACE-MEMBER`, in `spec/modules` — as evidence that *«the placement
  rule is already the host's practice; the field labels are not»*. `spec/common`
  has its own: `##DEFERRED-ENGINE-SPLIT` stated a condition, `##CORE-STAYS-PROMPT-ONLY`
  records it firing, and the outcome was executed and written back in place
  ([§3.12](#r-placement)). **Two instances is a practice, not an accident**, and
  it strengthens the ruling's case: the host already reopens records correctly
  when it has a condition — what it lacks is the condition, in 7 sections of 154.
- @fact:FINDING-DEBT-IS-CONCENTRATED **The `spec/common` debt is concentrated by
  file and inverted by age.** All 35 labels sit in three of twelve files, and
  the reopenable share runs **PROP-024 4/6 · PROP-000 5/18 · PROP-018 3/11** —
  the *newest* document, written against live external constraints, owes
  two-thirds of its sections a record, while the foundational one owes a
  quarter because most of it is definitional by design. If that pattern holds in
  `spec/modules`, a backfill there should be ordered by document age and
  external-dependency density, not swept front to back. **Not measured for
  `spec/modules` — stated as a hypothesis with its basis, per
  `BACKLOG.md` `##SEV-ASSIGNED-BY-REVIEWER`.**
