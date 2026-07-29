# BACKLOG — what the mega-refactor found and did not do {#root}

_Created 2026-07-26 by owner directive. **Findings raised during the PROP-043
Progress-Control programme that are neither the campaign's own work nor an
emergency collect here, and the next wave of work drains from this file.**_

**Not `TASKS.md`.** That file is a live checklist for one work-slice — items
that are commits waiting to be made. This one is the opposite genre: findings
nobody is working on yet, kept so the decision to work on them can be taken
deliberately later. Two different questions, two files, by owner ruling.

---

## What this file is, against the three that resemble it {#boundaries}

| file | holds | drained by |
|---|---|---|
| ##REL-TASKS `TASKS.md` | the current slice's checklist — each item is a commit | itself, as work lands |
| ##REL-DEFERRALS `campaigns/<id>/deferrals.md` | **one campaign's** tails; dies with the zone | the next campaign's mandate (`campaign-plans` law) |
| ##REL-AUDIT `AUDIT.md` | the periodic health sweep; an append-only **trend** | re-judged at the next audit |
| ##REL-BACKLOG **this file** | product-shaped findings the programme surfaced and deliberately did not act on | the next wave of work, after the programme reaches its end |

- ##TASKS2-OUTLIVES-THE-ZONE **It lives at the repository root because a campaign zone is
  disposable.** `ZONE-LIFETIMES` says `run/` is throwaway after close-out and
  wave 1's already is. A finding about where the product should go outlives the
  campaign that noticed it.
- ##TASKS2-GENRE **Genre: forward-looking, non-binding, drained by a later mandate.** Not
  a contract, not a checkpoint, not a health record. `spec-genres`' map does not
  carry this genre — the row is owed, alongside the documentation row Phase G
  adds.

## The three severities {#severity}

The scale is **P1 / P2 / P3**, taken from the `health-audit` flow rather than
invented. One severity vocabulary in the project, not two.

| | meaning | routing |
|---|---|---|
| ##SEV-P1 **P1** | security, data loss, structural integrity — **or a gate reporting green while not looking** | **stops the wave, reaches the owner the same session.** It never enters this file as a plan; it appears only afterwards, as record |
| ##SEV-P2 **P2** | a real gap with no emergency in it: a missing surface, a feature the corpus assumes and the code lacks, a mechanism specified and unbuilt | **this file.** Drains into the next wave |
| ##SEV-P3 **P3** | noted, no action planned | recorded here as `accepted`, so it is not rediscovered as new |

- ##SEV-REVIEWER-IS-AN-AGENT **«Reviewer» here means the boss *agent*, not the owner.** That is
  fine for classifying, and **not** fine for two things: **severity moves up
  freely and down only through the owner** (an agent may escalate to P1, never
  downgrade from it), and **every P2/P3 filed during a wave is reported to the
  owner at the time**, not merely written here — otherwise the agent deciding
  «this is a finding, not work» is the agent that wants to move on.
- ##SEV-ASSIGNED-BY-REVIEWER **Severity is the reviewer's call, never a worker's.** A cheap model
  calling something critical is noise, and a scale anyone may set is not a
  scale. A worker **reports the observation**; the reviewer classifies it.
- ##SEV-WORKER-MAY-INTERRUPT **One exception, running the other way:** a worker that believes it has
  found something genuinely alarming — a credential in source, an auth bypass, a
  gate that is lying — **stops its own packet and says so immediately**. The
  classification stays the reviewer's; the *interruption* needs no permission.
- ##SEV-P1-IS-NEVER-FILED **A P1 is never «filed».** That is the whole point of the split: one
  class of finding is not allowed to become a line in a list. If it is here, it
  is here as history, with what was done.
- ##SEV-GATE-BLINDNESS-IS-P1 **A gate that reports green because it is not looking is P1**, not P2.
  This programme found that shape three times — a floor gating a frozen slot, a
  parser blind to units the grammar allows, a sync check covering four of seven
  workspaces. Each was green and each was wrong, and a green panel that says
  nothing about coverage is a structural-integrity failure, not a gap.

## What an entry carries {#entries}

An **id**, the **`spec://…#ANCHOR`** it came from where one exists, a one-line
**locator**, a **severity**, a **disposition** (`open` · `planned` · `done` ·
`accepted`), and the **campaign or session** that filed it.

- ##ENTRY-CITES-NEVER-RESTATES **Cite the anchor; never restate the fact.** The same law Phase G's
  documentation runs on, for the same reason: a restated fact is a second
  statement of one truth with its own writer, and this programme has found that
  shape seven times.
- ##ENTRY-PREFER-GENERATED **Prefer generated over hand-maintained.** Where a finding is already
  carried by a marker — `action="rework"`, `stage="idea"`, an `#[ignore]`d test
  bound by its `verifies` edge — **the marked corpus is the source and this file
  quotes a query, not a copy.** A hand-maintained backlog is a derived value
  with its own writer, which is the defect class this programme keeps paying for.
- ##ENTRY-NO-SILENT-DELETION An entry leaves only by changing disposition, never by deletion. A
  backlog that forgets is indistinguishable from one that was never right.

## P1 — handled; kept as record {#p1}

*(empty — an open P1 is not in a file, it is in the owner's hands)*

## P2 — the next wave drains from here {#p2}

*(Phase T's **T-unbuilt** bucket is still expected to be the bulk filler: a fact
whose surface does not exist is a P2 by construction, and the ignored test
already written from it is the specification of the work.)*

### B-001 — the §10 link tables, PROP-035's unbuilt half {#b-001}

| | |
|---|---|
| ##B001-ANCHOR **anchor** | [`spec://vibevm/modules/vibe-workspace/PROP-035#OPEN-LINK-TABLES`](spec/modules/vibe-workspace/PROP-035-spec-compiler.md) — see also `##link-tables-give-back`, both `@spec/work` |
| ##B001-LOCATOR **locator** | `crates/vibe-spec/src/link_table.rs` — the graph and a deterministic dump exist; the persisted on-disk format and the structural consumer do not |
| ##B001-SEVERITY **severity** | P2 |
| ##B001-DISPOSITION **disposition** | `open` |
| ##B001-FILED **filed by** | the packages-actualization campaign, Phase D, 2026-07-29, on an owner ruling |

- ##B001-WHY-NOT-NOW **Why it is filed and not built.** Phase D's boot-link repair reaches it
  and does not need it. `#embed spec://…` resolves and splices at compile time
  today — `render_static` calls `expand_embeds` (`crates/vibe-workspace/src/boot_artifacts.rs:268`),
  under two tests — and an `@spec://` pointer that costs a lookup is strictly
  better than the confidently wrong relative path it replaces. Building a new
  layer mid-refactor would create code the refactor then has to refactor, which
  is the owner's stated reason for deferring it.
- ##B001-WHAT-IT-IS **What it actually is.** The vtable of the structural / JIT executor of
  PROP-035 §13 — a prebuilt index so a late-bound reader dispatches instead of
  searching. We do not run that mode. It is an optimisation of navigation cost,
  not a precondition of correctness.
- ##B001-WHEN-IT-BECOMES-URGENT **The trigger that promotes it.** When `@spec://` pointers in the boot
  lane are measured to cost a reader more than the lane saves — or when the §13
  structural loader is opened, whichever comes first. Either makes the searching
  real rather than hypothetical.

### B-003 — the Go floor gates a directory named `dirty` {#b-003}

| | |
|---|---|
| ##B003-ANCHOR **anchor** | none — found in a captured run, not against a marked fact |
| ##B003-LOCATOR **locator** | `campaigns/packages-2026-09/harvest/go-ai-native-lang-floor.md:11,31-35`; the gate is `packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/crates/go-ai-native-cli/src/floor.rs` |
| ##B003-SEVERITY **severity** | P2 |
| ##B003-DISPOSITION **disposition** | `open` |
| ##B003-FILED **filed by** | the packages-actualization campaign, Phase D, 2026-07-29 |

- ##B003-WHAT **What it is.** `tools/go-extract/test/fixtures/dirty/` holds
  deliberately malformed Go — it is the extractor's negative-test input, and its
  directory is named `dirty`. The floor treats it as source: `gofmt` fails on
  `…/dirty/internal/cells/plan/plan.go`, and **all five** of the run's `conform`
  findings are inside that same tree. Two of the six failing steps are this one
  cause.
- ##B003-WHY-IT-IS-A-DEFECT **Why it is a defect and not taste.** The host already decided this
  question the other way for its own tooling: `DEFAULT_EXCLUDES` in
  `crates/progress-core/src/scope.rs` drops `fixtures` as *not a contract*,
  always on and not overridable by an explicit include. One project, two answers
  to «is a fixture source», and the Go floor has neither an exclude list nor the
  word `fixtures` anywhere in it.
- ##B003-NOT-P1 **Why P2 and not P1.** ##SEV-GATE-BLINDNESS-IS-P1 covers a gate that
  reports green because it is not looking. This one is the opposite: it looks at
  more than it should and reports red. That is noise, and noise in a gate is how
  a floor stops being read — but it is not a gate that lies.
- ##B003-DO-NOT-CONFUSE **What it is not.** The other four failures in that run — no Go module at
  the package root, no `conform.toml`, no `specmap.json`, two absent optional
  linters — are **not** defects. They are what a project-level floor prints when
  it is aimed at a package that is not a project, and Phase C's §2.2 decision
  captures that unmodified output on purpose. The missing-linter step failing
  rather than skipping is the discipline behaving correctly: it refuses to go
  green by omission.

### B-002 — the budget row still binds generated artifacts {#b-002}

| | |
|---|---|
| ##B002-ANCHOR **anchor** | `spec://org.vibevm.world/addressable-specs/authoring-rules#ROW-BUDGET-BOOT-FILE` |
| ##B002-LOCATOR **locator** | the row states one budget for «the boot file» and does not distinguish an authored document from a generated one |
| ##B002-SEVERITY **severity** | P2 |
| ##B002-DISPOSITION **disposition** | `open` |
| ##B002-FILED **filed by** | the packages-actualization campaign, Phase D, 2026-07-29 |

- ##B002-WHY-HERE **Why here rather than fixed.** The host side of this was ruled by the
  owner and is recorded in PROP-009 §2.3: a generated boot artifact carries no
  token budget. The package's own row is owed the same scope clarification, and
  changing it is a release event — a published version and a re-vendor — so it
  waits for the release batch rather than riding a document repair.

### B-004 — a fact inside a fenced block carries no anchor, so whether it is judged is luck {#b-004}

| | |
|---|---|
| ##B004-ANCHOR **anchor** | none — the finding is that the surface *has* no anchor. The nearest marked facts are the `##re-derive-prompt-lead` leads in 17 packages |
| ##B004-LOCATOR **locator** | `crates/vibe-spec/src/doctree.rs` `fence_mask`, applied by `directives.rs:13-14`; the corpus is the `Read spec/flows/<name>/ …` line opening the re-derive prompt in 17 `spec/flows/*/[A-Z]*-PROTOCOL.md` |
| ##B004-SEVERITY **severity** | P2 |
| ##B004-DISPOSITION **disposition** | `open` |
| ##B004-FILED **filed by** | the packages-actualization campaign, Phase D, wave 6, 2026-07-29 |

- ##B004-WHAT **What it is, measured.** Seventeen packages ship a re-derive prompt whose
  **first instruction** is `Read spec/flows/<name>/ …`. A consuming host has no
  `spec/flows/` — the flow arrives at `vibedeps/flow-<name>/…` — so the
  instruction cannot be followed where it is meant to be run. **Phase C recorded
  the defect in two of the seventeen** (`licensing`, `spec-genres`; obligation
  F-240). The other fifteen carry the identical line and their re-derive anchors
  are all judged `confirmed`.
- ##B004-WHY-THE-VERDICTS-ARE-NOT-WRONG **The eleven `confirmed` verdicts are not errors, and that is the
  point.** The same anchor supports several claims, and different workers took up
  different ones. `spec-genres` judged the *path* («its FIRST instruction cannot
  be followed where it is meant to be run»). `addressable-specs` judged the
  prompt's *shape* («a propose-then-approve shape — the host uses the same
  shape»). `source-mirrors` judged its *outcome* («the host's manifest is a
  derivation rather than a copy»). Each is defensible against a lead-in that says
  only «Paste this to your agent in a fresh session». Nothing was mis-judged;
  the fenced body simply is not addressable, so which of its claims gets tested
  depends on which one a worker happens to read.
- ##B004-WHY-P2-AND-NOT-P1 **Why P2 and not P1.** ##SEV-GATE-BLINDNESS-IS-P1 covers a gate that
  reports green because it is not looking. Phase C's gate reported **6 847 /
  6 847 anchors, zero owed**, and that claim is exactly true — it is scoped to
  *addressable anchors*, and excluding fenced content is deliberate (PROP-035 §7:
  directives inside fences «are ignored, exactly as headings are»). The gate
  measures what it says it measures. What is missing is reach of the **fact
  model**, not honesty in the gate.
- ##B004-WHY-IT-MATTERS-NOW **Why it is decision-relevant before the release batch.** F-240 asks
  the owner to publish a fix scoped to two packages. Fifteen more ship the same
  line. Publishing the narrow fix is precisely what §4.5 calls **not a closure** —
  «a fix landed in one consumer and not the others … is a new `duplication`
  obligation». The scope of that ask should be seventeen or the remainder should
  be recorded, and this file is the record until it is.
- ##B004-THE-GENERAL-SHAPE **The general shape, which outlives this corpus.** Copy-paste prompts,
  worked examples and quick-start blocks are exactly the content most likely to
  be *run* by a reader, and exactly the content the anchor model cannot see.
  Anywhere a fenced block carries an instruction rather than an illustration, it
  is unverified by construction.

## P3 — accepted, no action planned {#p3}

*(empty)*
