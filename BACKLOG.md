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
- ##B004-WAVE8-CORRECTION **Corrected by wave 8's re-measurement (2026-07-31), three ways.**
  *(i)* The counts above were wrong in both directions: measured at HEAD, the
  fenced `Read spec/flows/<name>/ …` first line ships in **17 packages
  exactly**, and after wave 8 re-judged F-240's two leads `confirmed` (each was
  convicted of the fence's defect while its own do-not-copy-verbatim carve-out
  sits two lines above — 16 of 17 carry one), the lead anchors read
  **14 confirmed / 0 drift / 3 unjudged** — not «eleven confirmed», not
  «fifteen unrecorded». *(ii)* The scope question this entry poses to the owner
  is cleaner than either option it listed: **repairing all seventeen fences
  changes no verdict on any scope**, because a fence carries no anchor and no
  instrument can register the fix — the verdict question is closed and the
  repair question is a pure product decision. *(iii)* Wave 8 found the shape's
  second instance in the wild: the three `##three-processes-lead` ASCII
  diagrams in the `-lang` tools docs draw the retired `vibe-tcg` topology
  inside fences no anchor covers (`harvest/d8b-stacks-audience-release-reverify.md`).

### B-005 — `mirror --check` tests equality where the flow specifies ancestry {#b-005}

| | |
|---|---|
| ##B005-ANCHOR **anchor** | `spec://org.vibevm.world/source-mirrors/flows/source-mirrors/fanout-mechanics#INVARIANT-THE-ANCESTRY-GATE` — the rule; the defect is in the host's port of it |
| ##B005-LOCATOR **locator** | `xtask/src/mirror.rs:327-342` (`probe`), against the flow's own reference script at `fanout-mechanics.md:190-195` |
| ##B005-SEVERITY **severity** | P2 |
| ##B005-DISPOSITION **disposition** | `open` |
| ##B005-FILED **filed by** | the packages-actualization campaign, Phase D, wave 6, 2026-07-29 — found in passing while re-verifying F-204, outside its anchor list |

- ##B005-WHAT **What it is.** The flow specifies an **ancestry** gate: the target's main
  must be an ancestor of local mainline. Its own fifteen-line reference script
  implements exactly that — `git ls-remote` for the target's tip, then
  `git merge-base --is-ancestor`. The host's port does not: `probe` matches
  `Some(sha) if sha == head => SyncState::InSync` and sends everything else to
  `SyncState::Drift`. That is **equality**, and a target legitimately *behind*
  mainline — the ordinary state of every target between two fan-outs — is
  reported as drifted.
- ##B005-WHY-P2 **Why P2 and not P1.** It cannot produce a false green. `sha == head`
  implies in-sync under either test, so the error is strictly in the
  conservative direction: it reports red where the truth is «behind, which is
  fine». That is noise, and noise in a check is how a check stops being read —
  but it is not a gate that lies. Same reasoning as [B-003](#b-003), same
  direction.
- ##B005-NOT-THE-PUSH-PATH **What it is not.** The *push* path is sound and stays sound: it is
  fast-forward-only by construction, and `push_args_never_force`
  (`mirror.rs:426-440`) pins the never-`--force` invariant across four ref
  shapes. This is the read-only `--check` probe only.
- ##B005-THE-GENERAL-SHAPE **The shape worth remembering.** The package shipped a correct
  reference implementation *in shell*, and the consumer's re-implementation in
  Rust lost a property of it. Wave 6 nearly demoted the rule for the consumer's
  omission — the perimeter check caught that the package itself implements it.
  Where a flow ships a reference script, that script is a witness, and the port
  is the thing to audit against it.
- ##B005-NAMED-AS-F204-DEBT **Named as F-204's host debt** (owner ruling
  2026-08-01, the build-or-demote tail): the registry row is `deferred` naming
  this entry; the fix drains both together.

### B-006 — the highest-priority boot lane carries four normative snippets twice {#b-006}

| | |
|---|---|
| ##B006-ANCHOR **anchor** | falsifies `spec://org.vibevm.world/git-attribution-policy/flows/attribution-policy/ATTRIBUTION-POLICY#THE-POLICY-IS-STATED-IN-EXACTLY-ONE-ALWAYS-LOADED-PLACE` — from the host side, not the package's |
| ##B006-LOCATOR **locator** | `spec/boot/STATIC.md:421` and `:615` carry the identical `vibe:static org.vibevm.world/git-attribution-policy` provenance marker and source path; the emitter is `crates/vibe-workspace/src/boot_artifacts.rs` / the `bootgen` static lane |
| ##B006-SEVERITY **severity** | P2 |
| ##B006-DISPOSITION **disposition** | `open` |
| ##B006-FILED **filed by** | the packages-actualization campaign, Phase D, wave 6, 2026-07-29 |

- ##B006-WHAT **What it is, measured.** `spec/boot/STATIC.md` carries **31 static
  contributions resolving to 27 distinct sources**. The four duplicates are the
  whole `git-*` family — `git-atomic-commits`, `git-attribution-policy`,
  `git-autonomy`, `git-conventional-commits` — each emitted twice from the same
  `vibedeps/` path. They are reached both directly and through the
  `git-practices` umbrella the boot contract loads first, and the compiler
  concatenates both arrivals instead of emitting the contribution once.
- ##B006-WHY-IT-MATTERS **Why it is worth fixing rather than tolerating.** This is the lane
  `CLAUDE.md` tells every session to read «first and in full», so the cost is
  paid on every session by the most expensive reader in the project. And the
  content duplicated is **normative** — the commit rules — which is the
  `duplication` defect class this whole campaign exists to remove: one norm
  authored in two places with nothing forcing them to agree. Here they agree
  because they are byte-identical copies of one source, so nothing is *wrong*
  today; what is wrong is the shape.
- ##B006-IT-FALSIFIES-A-SHIPPED-CLAIM **It falsifies a shipped package's claim, and the package is not at
  fault.** `git-attribution-policy` states the policy «in exactly one
  always-loaded place (the boot snippet this package installs)». It installs
  exactly one. The consumer's compiler emits two. Wave 6 routed that obligation
  to the host on this evidence rather than softening the package's sentence.
- ##B006-WHY-P2 **Why P2 and not P1.** Nothing lies and nothing is lost: both copies are
  byte-identical and the rule they carry is the one in force. It is waste and a
  broken invariant, not a gate reporting green while not looking.

### B-007 — do the specs owe ADRs, and in what form? {#b-007}

| | |
|---|---|
| ##B007-ANCHOR **anchor** | the question is about `spec/common/**` and `spec/modules/**` as a genre, not about one anchor. The rule it would satisfy is `spec://org.vibevm.world/decision-records/flows/decision-records/DECISION-RECORDS-PROTOCOL#root` |
| ##B007-LOCATOR **locator** | 153 sections in `spec/common/` + `spec/modules/**` carry a bolded **Decision** label; 4 carry all four fields |
| ##B007-SEVERITY **severity** | P2 |
| ##B007-DISPOSITION **disposition** | `open` — **filed at owner request, 2026-07-31**, as a question to answer rather than work to schedule |
| ##B007-FILED **filed by** | the packages-actualization campaign, Phase D, wave 7 |

- ##B007-THE-QUESTION **The question, in the owner's framing.** Should the specifications
  carry Architecture Decision Records — and if so, **how**: as a section inside
  the PROP/FEAT that owns the decision, as a separate `spec/decisions/` genre, or
  as the four-field block the `decision-records` flow already prescribes? This is
  a **spec-genre design question**, and answering it decides how much work the
  `decision-records` host obligation actually is.
- ##B007-WHAT-IS-MEASURED **What is measured, so the question starts from facts.** Sections
  carrying a bolded `Decision` against those carrying all four fields
  (`Decision` · `Why` · `Considered and rejected` · `Revisit when` /
  `When to revisit`): `spec/common` + `spec/modules` **153 → 4**; all of `spec/`
  **157 → 7**; the `fractality` specspace **34 → 14**; this campaign's own
  records **15 → 8**. The practice is adopted at roughly **41 %** in the sibling
  project and **4.6 %** in the host's PROP/FEAT tree. Counted 2026-07-31.
- ##B007-CENSUS-CORRECTION **The sibling-adoption premise is withdrawn — corrected the same day
  by the D10 proposal pass.** The fractality «14 complete records» are, by file,
  **8 files carrying all four fields, all 8 vendored copies of the
  `decision-records` flow's own template, protocol, boot snippet and worked
  examples** (under `*/vibedeps/flow-decision-records/` and
  `flow-comparative-research/`, ×2 vendoring packages) — **0 authored**; the
  specspace's own authored decision blocks are 9, in a three-label dialect,
  none complete. So the honest comparison is «nobody authors the four-field
  form anywhere except this campaign's own plans», and the question is again
  *whether to adopt*, not «why is the PROP tree the outlier». Full measurement
  and the four costed options:
  `campaigns/packages-2026-09/harvest/d10-adr-genre-proposal.md`; the
  campaign's recommendation there is **B + A′** (four-field inside the owning
  section, forward-only, backfill only `spec/common/`, close `spec/decisions/`
  explicitly in the genre table).
- ##B007-WHY-IT-IS-A-QUESTION-NOT-A-TASK **Why it is a question and not a task.** «Add the missing fields to
  153 sections» is the wrong shape twice over. Most of those decisions are not
  reopenable, so a revisit condition on them would be ceremony; and the four-field
  block is not obviously the right ADR form for a specification, which already
  states rationale in prose. **What is owed first is the genre decision**, and
  `spec-genres`' own map does not carry an ADR row today.
- ##B007-WHAT-IT-UNBLOCKS **What it unblocks.** The largest single host obligation this phase
  surfaced ([`PHASE-D-HOST-OBLIGATIONS.md`](campaigns/packages-2026-09/PHASE-D-HOST-OBLIGATIONS.md)).
  It cannot be sized, let alone scheduled, until this is answered.

### B-008 — one workspace crate declares no licence, and the live ledger says otherwise {#b-008}

| | |
|---|---|
| ##B008-ANCHOR **anchor** | `CLAUDE.md`'s operating-facts ledger, «License state»: *«our shipped surface is fully UPL-1.0 … host crates inherit via `license-file.workspace`»* |
| ##B008-LOCATOR **locator** | `crates/vibe-index/Cargo.toml` — no `license` or `license-file` key of any kind. Every other workspace member carries `license-file.workspace = true` on line 7; the workspace declares `license-file = "LICENSE.md"` at `Cargo.toml:55` |
| ##B008-SEVERITY **severity** | P2 |
| ##B008-DISPOSITION **disposition** | `open` |
| ##B008-FILED **filed by** | the packages-actualization campaign, Phase D, wave 7, 2026-07-31 — surfaced while re-verifying F-236, outside its anchor list |

- ##B008-WHAT **What it is.** `vibe-index` carries full package metadata —
  `authors`, `description`, `homepage`, `repository`, `keywords`, `categories` —
  and omits the licence line alone. It is the **only** crate in the workspace
  that does, checked by iterating every `crates/*/Cargo.toml`. So the ledger
  sentence «host crates inherit via `license-file.workspace`» is true of every
  crate but one, and the relicensing run that made the surface UPL-1.0 did not
  reach it.
- ##B008-WHY-P2-AND-NOT-HIGHER **Why P2.** `publish = false`, so nothing reaches a registry
  undeclared and no third party receives an unlicensed artifact. The defect is
  that a **live, owner-maintained ledger asserts something that is false for one
  member** — which is precisely the class this campaign exists to remove, and
  the campaign found it in its own host rather than in a package.
- ##B008-WHY-FILED-NOT-FIXED **Why filed and not fixed.** It is a one-line change and it is a
  change to the legal surface. `RULE-NO-SILENT-REPAIRS` binds the phase, and
  `CLAUDE.md`'s licence ledger is owner-maintained — an agent editing a licence
  declaration on its own initiative is the wrong default even when the edit is
  obviously right. **The fix is `license-file.workspace = true` on line 7, to
  match its twenty-odd siblings.**

### B-009 — the wind-down's push step contradicts the rollout two host documents standardise {#b-009}

| | |
|---|---|
| ##B009-ANCHOR **anchor** | falsifies nothing in a package — the contradiction is host-internal. The rule side is `spec/boot/90-user.md` `##CMD-MIRROR` and `spec/common/PROP-016-source-mirrors.md` `##CMD-MIRROR`; the breach side is `CLAUDE.md`'s END SESSION step 4 |
| ##B009-LOCATOR **locator** | `CLAUDE.md:191` — «Push to `origin/main` — routine per Rule 4» as the wind-down's step 4, where `90-user.md:35` says `cargo xtask mirror` «is the standard rollout, preferred over a bare `git push origin`» and `PROP-016:59` says «This — not `git push origin` — is the standard rollout» |
| ##B009-SEVERITY **severity** | P2 |
| ##B009-DISPOSITION **disposition** | `done` — owner ruling 2026-07-31 («сделай»): step 4 of the wind-down in all three instruction files now names `cargo xtask mirror` as the standard rollout, with the bare push demoted to fallback and the escape hatch preserved |
| ##B009-FILED **filed by** | the packages-actualization campaign, Phase D, wave 8, 2026-07-31 — surfaced re-verifying F-220's source-mirrors half, where the recorded verdict used `CLAUDE.md:191` to demote a package sentence that the other two host documents support |

- ##B009-WHAT **What it is.** Three host documents state the wind-down rollout and one
  disagrees with the other two. The wind-down contract in `CLAUDE.md` prescribes
  the bare named-remote push; the user-owned boot snippet and PROP-016 both name
  the mirror fan-out the standard rollout and explicitly deprecate the bare push
  for it. A session following `CLAUDE.md` to the letter rolls out to one host
  and leaves every other mirror behind — the exact state `PROP-016`'s fan-out
  exists to prevent.
- ##B009-WHY-FILED-NOT-FIXED **Why filed and not fixed.** `CLAUDE.md` is the owner-maintained
  boot contract; its END SESSION section is an owner-authored command
  specification, and `RULE-NO-SILENT-REPAIRS` binds the phase. The fix is one
  line — step 4 saying `cargo xtask mirror` (or «push, then fan out») — but
  which wording the owner wants is the owner's call.
- ##B009-COST-TODAY **What it costs today.** Every session that ends by the book pushes
  `origin` only; the mirrors drift until someone runs the fan-out by hand, and
  `mirror --check`'s equality probe (B-005) then reports the *targets* as
  drifted — two filed defects compounding into one confusing red panel.

### B-010 — a check verb that writes, and a `--campaign` flag that selects state rather than scope {#b-010}

| | |
|---|---|
| ##B010-ANCHOR **anchor** | none — found by a delegated run, not against a marked fact; the nearest law is `tool-design-lessons`' read-verbs-do-not-mutate genre |
| ##B010-LOCATOR **locator** | `vibe progress check --exhaustive --campaign <zone>` — rewrites the named zone's `run/cache.json` / `state/campaign.json` / `state/corpus.json` (observed: +4 962 lines in the closed wave-1 zone's cache, plus a re-scope of the live zone's corpus), because `--campaign` selects the **state zone to write**, not the perimeter to read |
| ##B010-SEVERITY **severity** | P2 |
| ##B010-DISPOSITION **disposition** | `open` |
| ##B010-FILED **filed by** | the packages-actualization campaign, D10 pass, 2026-07-31 — a drafting worker pointed the check at the closed `progress-2026-08` zone expecting a read; the boss restored all six files from HEAD, loss-free |

- ##B010-WHY-IT-BITES **Why it bites.** A verb named `check` reads as read-only, and the flag
  named `--campaign` reads as «over this campaign's perimeter»; together they
  silently rewrite a **closed** campaign's frozen state. `ZONE-LIFETIMES` calls
  a closed zone's `run/` throwaway, so nothing broke here — but the same
  combination pointed at a **live** zone during another session's merge window
  would race its cache.
- ##B010-THE-FIX-SHAPE **The fix shape, for Phase E.** Either `check` becomes read-only
  (scan state moves behind an explicit `--write-state`), or its help says in the
  first line that it warms the zone's cache; and `--campaign`'s help says it
  selects the state zone. One of the two — a check that quietly writes is how a
  frozen zone stops being frozen.

### B-011 — marker stripping in the boot compiler needs an aliasing design first {#b-011}

| | |
|---|---|
| ##B011-ANCHOR **anchor** | the compile path: `crates/vibe-workspace/src/boot_artifacts.rs` (static lane), `boot_artifacts/normal.rs` (PROP-035 §8 compile); no marker handling exists anywhere in it today |
| ##B011-LOCATOR **locator** | measured 2026-07-31: the 22 canonically-mapped static contributions carry 838 `##ANCHOR` / `@stage/state` tokens over 1 446 source lines, all of which compile verbatim into `spec/boot/STATIC.md` after a `--force` re-vendor |
| ##B011-SEVERITY **severity** | P2 |
| ##B011-DISPOSITION **disposition** | `open` — **owner design direction recorded 2026-07-31**, deliberately deferred («это не сейчас, это в бэклог») |
| ##B011-FILED **filed by** | the packages-actualization campaign, the publication runbook's marker fork, 2026-07-31 |

- ##B011-WHY-NAIVE-STRIPPING-IS-WRONG **Why naive stripping is wrong, in the owner's own framing.** Strip
  the markup from the compiled lane and a **dynamic module can reference an
  anchor that existed in the source markup and vanished after cleaning** — the
  reference resolves at authoring time and dangles at read time. Stripping is
  not a filter; it changes what is addressable from where, so it needs a
  resolution design, not a regex.
- ##B011-THE-DESIGN-DIRECTION **The design direction (owner, 2026-07-31).** Short names of the shape
  `#use spec://… as SOMETHING`: a lane consumer imports an anchor under an
  alias, and **when SOMETHING's carrier was cleaned, the compiler loads the
  source markup and learns where the anchor lives** — resolution survives
  stripping because the alias binds to the source-of-truth address, not to the
  compiled text. Stripping then becomes safe to build on top.
- ##B011-INTERIM **The interim, ruled the same day:** publish as is — the lane carries
  the authoring tokens (the house grammar every agent here reads), and the
  strip waits for the aliasing design rather than shipping half-safe.

### B-012 — PROP-014's specified-not-built mechanism set: research feasibility {#b-012}

| | |
|---|---|
| ##B012-ANCHOR **anchor** | the ten annotated facts of `spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014-specmap-bidirectional-traceability` — each now carries its «Specified, not built» clause naming exactly what is absent |
| ##B012-LOCATOR **locator** | the mechanisms, in one list: package-shipped `specmap.json` index + fetch-by-content-hash; the per-item edge-multiplicity lint in `vibe check`; `CodeItem.content_hash` + derived `Command`/`ErrorVariant` node views; error-rendering **index lookup** with revision + `run: vibe explain` hint (the compile-time-constant doorway ships); the LLM prose producer behind `vibe explain --prose` (deterministic template ships); `[metamodel] profile` runtime profiles; the spec-unit length warning (≤ 120); rustdoc composition in `explain`; `specmap_query` / `specmap_source` MCP tools |
| ##B012-SEVERITY **severity** | P2 |
| ##B012-DISPOSITION **disposition** | `done` — исследование выполнено 2026-08-01 (`campaigns/packages-2026-09/harvest/d14-b012-prop014-feasibility.md` + части A/B), решения владельца того же дня разлиты в записи [B-015](#b-015)…[B-021](#b-021): всё из десятки строится (диспозиция `planned`), безопасность — протоколируется и паркуется до уведомления владельца |
| ##B012-FILED **filed by** | the packages-actualization campaign, партия 1a, 2026-08-01 |

- ##B012-WHY-RESEARCH-FIRST **Why research-first.** Wave 8/D9 established the corpus-side truth
  (the annotations); the product-side question — which of the ten are worth
  building, in what order, and which are better retired from the spec — is a
  design pass over PROP-014's §13-era ambitions against today's shipped
  surface. B-001 (the link tables) is the same family and the same trigger
  logic; the two studies should run together.

### B-013 — the specmap schema-bump path is broken before anyone needs it {#b-013}

| | |
|---|---|
| ##B013-ANCHOR **anchor** | none — found by the B-012 evidence pass, not against a marked fact; the nearest law is `dev-runtime-docs`' never-describe-an-abandoned-toolchain |
| ##B013-LOCATOR **locator** | `xtask/src/codegen.rs:50-52` routes the `specmap` schema's codegen to `packages/org.vibevm.ai-native/rust-ai-native-lang/v0.5.0/crates/specmap-core/src/generated` — a slot that does not exist (only `v0.7.0` does); repeated for the drift check at `codegen.rs:215`. Two more coordinates of the same stale relocation: `schemas/specmap.jtd.json` metadata still names `crates/specmap-core/...` / `specmap_core::specmap`, and `core-ai-native-specmap/src/lib.rs:24-27` names a package-local `schemas/specmap.jtd.json` that is absent from the repository |
| ##B013-SEVERITY **severity** | P2 |
| ##B013-DISPOSITION **disposition** | `open` |
| ##B013-FILED **filed by** | the B-012 feasibility study (`campaigns/packages-2026-09/harvest/d14-b012-part-B.md` §B2, part A cross-cutting note), 2026-08-01 |

- ##B013-WHY-IT-BITES **Why it bites.** Every serialised-index evolution in the B-012 set —
  `CodeItem.content_hash` (M7a), a serialised `doc` field (M10), signatures for
  the `contract` profile (M3) — is a `SCHEMA` 2→3 bump that must go through
  jtd-codegen, and the route 404s on first use. The engine relocated into
  `core-ai-native/v0.8.0` and the codegen plumbing did not move with it.
- ##B013-WHY-P2 **Why P2.** Nothing lies: the checked-in generated module is current and
  the gate byte-compares real artefacts. The defect is a dev-op that fails on
  first invocation — noise at the exact moment someone attempts a planned
  evolution, plus two documentation surfaces describing a pre-relocation world.
- ##B013-FIX-SHAPE **The fix shape.** Point `generated_dir_for` at the authored engine
  (`packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-specmap/src/generated`),
  fix the drift-check twin, refresh the schema file's metadata, and either add
  the package-local schema copy the engine header promises or reword the header.

### B-014 — the committed host specmap.json drifts with no freshness gate {#b-014}

| | |
|---|---|
| ##B014-ANCHOR **anchor** | none — measured by the B-012 evidence pass; the class is the health-audit's out-of-gate drift, `spec://org.vibevm.world/health-audit/flows/health-audit/HEALTH-AUDIT-PROTOCOL#root`'s own subject |
| ##B014-LOCATOR **locator** | root `specmap.json`: **599 of 5266** spec units' recorded `line` no longer lands on that unit's anchor at HEAD (concentrated: PROP-000 ×137, PROP-043 ×112, PROP-018 ×92, PROP-009 ×91); the code side holds (898/912 edges land on a marker line). No gate covers it: `tools/self-check.sh:366-375`'s specmap steps are the packages' own `--gate` self-traces, and no host-index regeneration or byte-compare runs anywhere in the panel |
| ##B014-SEVERITY **severity** | P2 |
| ##B014-DISPOSITION **disposition** | `open` |
| ##B014-FILED **filed by** | the B-012 feasibility study (`campaigns/packages-2026-09/harvest/d14-b012-part-B.md` §B4 freshness caveat), 2026-08-01 |

- ##B014-WHY-NOT-P1 **Why P2 and not P1.** `##SEV-GATE-BLINDNESS-IS-P1` covers a gate that
  reports green *because it is not looking while claiming to look*. No gate
  claims to check the host index — self-check's specmap steps name the package
  slots they trace, truthfully. This is a committed derived artefact whose
  producer is never re-run: out-of-gate drift, the exact class the periodic
  audit exists for, not a lying panel.
- ##B014-COST **What it costs today.** Any consumer of the committed index inherits
  stale spec-side coordinates — including the M2 доorway slice the B-012 study
  shortlists (its 81/81 URI-resolution measurement holds, but a printed
  `file:line` would be wrong for ~11 % of units) — and every index-derived
  distribution must carry a freshness caveat, as the study's own tables do.
- ##B014-FIX-SHAPE **The fix shape, two independent halves.** (i) Regenerate the index and
  commit it (one command, one churny diff). (ii) Decide whether the host wants
  a freshness gate at all — a `--check`-style byte-compare in self-check, a
  WalFreshness-style staleness warning in `vibe check`, or a deliberate
  «regenerated on demand only» posture recorded as a decision. The A–D
  health-audit inventory scheduled at the Phase D exit gate should meet this
  entry there.

### B-015 — программа безопасности runtime-канала: запротоколирована и запаркована до уведомления владельца {#b-015}

| | |
|---|---|
| ##B015-ANCHOR **anchor** | тема §2.8.4 PROP-014 (specmap); полное досье — `campaigns/packages-2026-09/harvest/d14-b012-part-A.md`, раздел A5 |
| ##B015-LOCATOR **locator** | подписи нет нигде в дереве (единственная crypto-зависимость — sha2 для контент-хэшей); две уже шипящиеся дороги «текст пакета → контекст агента» перечислены ниже |
| ##B015-SEVERITY **severity** | P2 |
| ##B015-DISPOSITION **disposition** | `open` — **запаркована решением владельца, НЕ строить до его специального уведомления**; кодовых триггеров нет намеренно |
| ##B015-FILED **filed by** | решение владельца 2026-08-01 по исследованию B-012 |

- ##B015-SUT **Суть, по-простому.** Задуманные инструменты для агентов будут отдавать текст из пакетов прямо в контекст агента. Текст в контексте агента — потенциальные команды: подложи в пакет вредный абзац — и читающий агент может быть им управляем (prompt injection). Защита — криптографическая подпись содержимого пакетов, чтобы читатель мог проверить «текст от автора, не подменён». Дизайн specmap изначально требовал: канал не шипится без подписи.
- ##B015-RULING **Решение владельца (2026-08-01, дословно):** «Положить в бэклог, ничего не строить до специального уведомления. Нужно вначале построить чтобы вся система работала "как-то", наполнить репозитории, и так далее. И только потом уже беспокоиться о безопасности. Бессмысленно строить безопасность проекта, которым никто не пользуется. Пользуется им кто-то или не пользуется — из кодовой базы не видно, это видно владельцу из наблюдения внешнего мира, поэтому это решение владельца.» Следствие: условие переоткрытия — **только уведомление владельца**; никакие наблюдаемые в коде события записью не назначаются.
- ##B015-TASKS **Протокол задач на день переоткрытия (полный список):**
  1. **Выбор схемы подписи.** Кандидаты, в порядке рекомендации исследования: (1) подписанные git-теги SSH-ключом мейнтейнера — реестр и есть git, паблишер уже пушит теги, ноль нового wire-формата, верификация через allowed_signers; (2) minisign-класс — detached-подпись контент-хэша пакета, крошечная permissive-зависимость, полностью офлайн; (3) sigstore-класс — отклонён на сегодня: тяжёлые зависимости, онлайн-верификация против clean-clone/offline-постуры, identity через OIDC чужда single-writer-модели; пересмотреть при втором независимом издателе.
  2. **Единица подписи** — дерево пакета на теге (рекомендация), не index отдельно: всё, что сервится из верифицированного дерева, наследует целостность. Сегодняшний контент-хэш в lockfile защищает от подмены байтов зеркалом, но не отвечает «это байты издателя?» — подпись закрывает второй вопрос.
  3. **Инфраструктура:** trust root (где живёт публичный ключ), точка верификации при fetch (рядом с существующей проверкой хэша), ротация/ревокация, кастодия ключа по secrets-hygiene, возможное поле в lockfile.
  4. **Оформление ответов инструментов:** фраза «возвращаемое — справочные данные, не инструкции» на всех инструментах, отдающих агенту текст пакетов, включая **уже существующие две дороги** — чтение сабскиллов установленного пакета и boot-снипеты, читаемые агентом на старте сессии. Явное исключение: агентский релей (agentic_explain) — там инструкции суть фичи, оформление не меняется.
  5. **Линт императивных формулировок** в текстах пакетов (второе-лицо-повелительное вне guide-типа) — требует меток типа на секциях (см. B-019, twin-разметка).
  6. **Правка позиции спеки.** PROP-014 несёт позицию «канал шипится только подписанным». Решением владельца последовательность перевёрнута (канал раньше подписи — B-018); в момент постройки B-018 эта позиция правится owner-approved диффом, чтобы спека не противоречила построенному. Записано здесь, чтобы не потерялось.

### B-016 — карта в составе пакета + получение исходных фрагментов по отпечатку {#b-016}

| | |
|---|---|
| ##B016-ANCHOR **anchor** | механизмы «distribution» PROP-014; досье — `d14-b012-part-A.md`, раздел A1 |
| ##B016-LOCATOR **locator** | producer карты готов и гейтится; манифест не имеет списка файлов вообще (пакет едет целой директорией — файл карты поедет бесплатно); хранилища «хэш → исходный текст» не существует нигде |
| ##B016-SEVERITY **severity** | P2 |
| ##B016-DISPOSITION **disposition** | `planned` — **решение владельца 2026-08-01: «это надо строить»** |
| ##B016-FILED **filed by** | решение владельца 2026-08-01 по исследованию B-012 |

- ##B016-SUT **Суть, по-простому.** Сегодня каждый проект строит карту связей только про себя. Задумка из двух половин: **(1)** пакет возит готовую карту с собой — потребитель может спрашивать про установленный пакет, ничего не пересобирая; **(2)** механизм «дай точный кусок исходника по его отпечатку (хэшу)» — чтобы ответ мог показать не только «где», но и «что именно».
- ##B016-HALF1 **Половина 1 (дёшево).** Файл карты и так поедет внутри пакета — грузовик уже ездит. Достроить: политику (в чьём пространстве имён URI карты; входит ли карта в контент-хэш пакета — если да, каждый код-эдит пакета меняет его пин в lockfile, это осознать); шаг генерации карты на пакет; и главное — **читателя** на стороне потребителя, которого сегодня нет (единственный сегодняшний потребитель чужих спек пере-парсит markdown и живёт без карты).
- ##B016-HALF2 **Половина 2 (дорого).** Целиком новое: тип «адрес фрагмента», хранилище «хэш → текст», глагол скачивания в словаре реестра (он сегодня пакет-гранулярный). Нерешённый дизайн-вопрос до кода: что такое фрагмент **со стороны кода** (сегодня у элемента кода нет ни конца диапазона, ни тела — только файл и строка).
- ##B016-DEPS **Зависимости и порядок.** Читатель половины 1 — инструменты B-018 (строить смежно). Половина 2 — после половины 1. Любое изменение формата карты идёт **одной** сменой формата вместе с B-019 (не тремя), и до неё чинится сломанный инструмент перегенерации (B-013).

### B-017 — профили приватности для закрытых проектов {#b-017}

| | |
|---|---|
| ##B017-ANCHOR **anchor** | механизм «[metamodel] profile» PROP-014; досье — `d14-b012-part-A.md`, раздел A3 |
| ##B017-LOCATOR **locator** | ключа не существует ни в одном манифесте/схеме/парсере; редакционного пути нет; у «contract»-уровня нет данных (карта не хранит сигнатур) |
| ##B017-SEVERITY **severity** | P2 |
| ##B017-DISPOSITION **disposition** | `planned` — **решение владельца 2026-08-01: «это надо строить»** |
| ##B017-FILED **filed by** | решение владельца 2026-08-01 по исследованию B-012 |

- ##B017-SUT **Суть, по-простому.** Закрытый (не open-source) проект должен уметь сказать в конфиге: «когда мою карту читают снаружи — делись всем / только контрактом без тел кода / ничем». Три уровня: open / contract / none.
- ##B017-BUILD **Что строить.** (1) Ключ в манифесте — сам по себе маленький, но парсер манифеста отвергает незнакомые ключи, значит старые версии vibe будут падать на файле с новым ключом: вводить вместе с механикой минимальной версии, не «на вырост». (2) Редакцию применять **на стороне производителя** (байты закрытого проекта не покидают его машину), не фильтром на сервере. (3) Для уровня «contract» карте нужны сигнатуры элементов кода — это смена формата карты: ехать той же одной сменой, что B-016/B-019. (4) Содержание «contract»-уровня (что именно безопасно отдавать: сигнатуры? доки?) — вопрос, который дизайн сам отложил до реального закрытого потребителя; в момент постройки вернуть владельцу с требованиями такого потребителя на столе.
- ##B017-DEPS **Зависимости.** Применяется только там, где есть чем делиться наружу: строить после/вместе с B-016 (половина 1) и B-018.

### B-018 — инструменты для агентов (MCP), широкий вариант — высокий приоритет владельца {#b-018}

| | |
|---|---|
| ##B018-ANCHOR **anchor** | механизмы «runtime exposure» PROP-014; досье — `d14-b012-part-A.md`, раздел A4 |
| ##B018-LOCATOR **locator** | локальная команда «объясни» работает в чекауте; в трёх стековых MCP-серверах её аналог уже шипится; у хостового vibe таких инструментов нет; ответы про установленные пакеты сегодня сознательно исключены |
| ##B018-SEVERITY **severity** | P2 |
| ##B018-DISPOSITION **disposition** | `planned` — **решение владельца 2026-08-01: «это надо строить, причем с высоким приоритетом и в широком варианте (вместе с объяснением чужих пакетов)»** |
| ##B018-FILED **filed by** | решение владельца 2026-08-01 по исследованию B-012 |

- ##B018-SUT **Суть, по-простому.** Дать AI-агенту спрашивать работающий vibe: «объясни это требование», «что реализует эту команду», «покажи фрагмент», «поищи по карте» — и не только про свой проект, но и про **установленные пакеты**. Это центральная фича всего сюжета «поделиться картой».
- ##B018-PARTS **Четыре части, в порядке постройки.**
  1. **Перенос «объясни» в агентский интерфейс vibe** — легко: все швы готовы, в стековых серверах есть три рабочих образца этой же формы.
  2. **Поиск по карте.** Дизайн не определил язык запросов — сначала спроектировать (заготовка v0: точный URI + имя символа + фильтр по типу, жёсткий потолок размера ответа), положить в спеку owner-диффом, потом кодить.
  3. **Фрагменты по отпечатку** — вместе с B-016 (половина 2).
  4. **Ответы про установленные пакеты** («объяснение чужих пакетов»). Сегодня чужие секции сознательно не попадают в карту проекта — на этом исключении держится воспроизводимость карты (байт-в-байт проверка). Ломать исключение нельзя; строить **вторую, некоммитимую** карту-резолвер, собираемую в момент запроса из установленных пакетов. Кормится из B-016 (половина 1).
- ##B018-SECURITY **Безопасность.** Осознанно строится ДО подписи — перепоследовательность зафиксирована решением владельца в [B-015](#b-015): безопасность паркуется до его уведомления. В момент постройки этой записи позиция спеки «канал шипится только подписанным» правится owner-approved диффом (см. B-015, задача 6), чтобы построенное не противоречило написанному.

### B-019 — отпечатки кода + узлы «команда» и «вариант ошибки» в карте {#b-019}

| | |
|---|---|
| ##B019-ANCHOR **anchor** | механизм «edge model nodes» PROP-014; досье — `d14-b012-part-B.md`, раздел B2 |
| ##B019-LOCATOR **locator** | элемент кода в карте — пять полей без отпечатка и без тела; узла «команда» нет нигде; извлечение «вариантов ошибок» полностью существует — но в соседней подсистеме conform, не в карте |
| ##B019-SEVERITY **severity** | P2 |
| ##B019-DISPOSITION **disposition** | `planned` — **решение владельца 2026-08-01: «это надо строить. Это должна быть алгоритмическая фича, без использования LLM. Все части — а, б, в»** |
| ##B019-FILED **filed by** | решение владельца 2026-08-01 по исследованию B-012 |

- ##B019-SUT **Суть, по-простому.** Три доделки самой карты, все чисто алгоритмические (владелец: без LLM). **(а)** Отпечаток (хэш) на каждом элементе кода — чтобы карта замечала «код под этим требованием изменился, пересмотри связь»; сегодня она слепа к изменениям кода. **(б)** Узел «команда» — чтобы `vibe install` был сущностью карты, а не только функцией: ответ «что реализует vibe install» становится возможен напрямую. **(в)** Узел «вариант ошибки» — чтобы каждая ошибка была узлом карты и вела к своему требованию.
- ##B019-A **(а) — решения перед кодом.** Что хэшируем: текст (каждый прогон форматтера и правка комментария меняют отпечаток — шумно) или поток токенов (форматонезависимо — рекомендация); решение владельцу в момент постройки, с замером шума на обоих вариантах. Это смена формата карты с полной перегенерацией: ехать одной сменой вместе с B-016/B-017, до неё починить сломанный инструмент перегенерации (B-013). Парная половина со стороны спеки — метки-редакции на секциях: целевой набор ~80 секций, на которые ссылаются сообщения об ошибках, + правило «новые секции сразу с меткой» (решение владельца 2026-08-01 по «ключу 2»).
- ##B019-B **(б) — с нуля.** В дереве нет ни определения «команды», ни экстрактора, ни потребителя. Определить, что считается командой (поверхность CLI-подкоманд), написать экстрактор, добавить тип узла (та же одна смена формата), научить «объясни» принимать команду как цель.
- ##B019-V **(в) — что имеется в виду, и вопрос границы систем (решить ДО реализации — требование владельца).** В кодовой базе два независимых движка: **conform** (гейт качества кода: прогоняет правила, находит нарушения) и **specmap** (карта связей «код ↔ спека»). Данные о «вариантах ошибок» — какие enum-варианты с какими текстами ошибок существуют и на какие требования ссылаются — **уже извлекаются конформом** для двух его правил. Карта этих данных не видит: это два разных графа двух разных подсистем. Вопрос: чьей частью становится узел «вариант ошибки»? Три варианта: **(1)** specmap извлекает сам — дублирование экстракции в двух движках, две правды об одном; **(2)** specmap читает данные conform'а — новая зависимость между сознательно разделёнными движками; **(3)** не сливать данные вовсе, объединять на этапе запроса — инструмент B-018 показывает и карту, и находки conform'а рядом. Склонность исследования — (3) при наличии B-018, иначе (1) с выносом общей экстракции в разделяемую библиотечку; окончательное решение — первый шаг реализации этой части.

### B-020 — объяснения человеческим языком через внешние LLM {#b-020}

| | |
|---|---|
| ##B020-ANCHOR **anchor** | механизм «LLM as renderer» PROP-014; досье — `d14-b012-part-B.md`, раздел B3 |
| ##B020-LOCATOR **locator** | команда «объясни» отвечает детерминированным шаблоном; слот под второго производителя текста в кэше готов; LLM-клиента у vibe нет (крейт-заглушка) |
| ##B020-SEVERITY **severity** | P2 |
| ##B020-DISPOSITION **disposition** | `planned` — **решение владельца 2026-08-01: «это надо строить. Я думаю построить лайтовый клиент для внешних нелокальных LLM, который будет через них строить такие объяснения. Возможно это будет fractality, с этим нужно разобраться позднее»** |
| ##B020-FILED **filed by** | решение владельца 2026-08-01 по исследованию B-012 |

- ##B020-SUT **Суть, по-простому.** Команда «объясни» сегодня отвечает сухим шаблоном («такая-то функция реализует такой-то пункт»). Фича: опционально та же информация пересказывается внешней LLM человеческой прозой — «эта команда устроена так потому-то, вот решения, вот известные отступления».
- ##B020-DIRECTION **Направление владельца.** Лайтовый клиент к внешним нелокальным LLM (не встроенный движок); возможный носитель — fractality (воркер дергает внешнюю модель); разобраться позднее, в момент постройки.
- ##B020-BUILD **Что строить и что помнить.** (1) Сначала — текст в данных: сегодня ответ «объясни» несёт только имена и пути, без текста спеки и без документации кода; LLM было бы не из чего писать. Зависимость: включить текст документации и секций в ответ (кандидат ближайшего рабочего среза, дёшево, формат карты не меняется). (2) Второй «производитель текста» встаёт в готовый слот кэша; в ключ кэша добавляется идентификатор модели. (3) Шаблонный режим остаётся навсегда — инструмент обязан быть полноценным без LLM (инвариант дизайна). (4) Проза — только презентационный слой поверх детерминированных данных; сами данные карты LLM не трогает. (5) Ключи/креды внешних LLM — по secrets-hygiene.

### B-021 — пороговые предупреждения: перегруженные связи и длинные секции {#b-021}

| | |
|---|---|
| ##B021-ANCHOR **anchor** | механизмы «multiplicity lint» и «units fit a page» PROP-014; досье — `d14-b012-part-B.md`, разделы B1 и B4 |
| ##B021-LOCATOR **locator** | ни один слой не считает связей на элемент кода; длина секций вычисляется и выбрасывается в движке карты и уже лежит готовым полем в хостовом компиляторе спек |
| ##B021-SEVERITY **severity** | P2 |
| ##B021-DISPOSITION **disposition** | `planned` — **решение владельца 2026-08-01: строить обе, вместе** |
| ##B021-FILED **filed by** | решение владельца 2026-08-01 по исследованию B-012 |

- ##B021-SUT **Суть, по-простому.** Два предупреждения о качестве. **(1) Перегруженные связи:** элемент кода, реализующий сразу много пунктов спеки, обычно делает слишком много (или спека нарезана слишком мелко). Определяется чисто алгоритмически: карта — это список пар «элемент кода → пункт спеки»; сгруппировать пары по элементу, посчитать, сравнить с порогом. Ни LLM, ни эвристик — арифметика по готовому файлу. **(2) Длинные секции:** пункт спеки длиннее порога плохо читается и чаще меняется; длина уже вычисляется, осталось сравнить и предупредить.
- ##B021-RULING **Обоснование владельца (2026-08-01, дословно):** «Эти волшебные свойства не срабатывают на нашей базе, потому что она написана относительно хорошо. В других проектах это может быть совершенно не так. Мы пишем систему для всех, а не только для нас.» То есть: нулевые срабатывания на нашем корпусе — не довод против постройки; это продуктовые фичи для чужих, менее чистых корпусов.
- ##B021-BUILD **Что строить.** Оба порога **конфигурируемые**, стартовые значения (3 связи; 120 строк) — честные плейсхолдеры до реальной статистики, которую предупреждения сами и соберут. Оба — предупреждения, не блокирующие гейты (по крайней мере на старте). Счёт связности живёт в движке карты рядом с существующим механизмом предупреждений (там уже есть цикл отчёта, конфиг и блокирующий режим); формулировка в дизайне, называющая другой дом, правится однострочным owner-диффом при постройке. Счёт длины — по «листовым» секциям (без вложенных подсекций) по умолчанию, иначе предупреждение измеряет жанр документа, а не дисциплину секций; настройка зерна — в конфиг.

### B-022 — исследование: механизмы кэша объяснений (LEDGER-INTENT), можно ли реализовать {#b-022}

| | |
|---|---|
| ##B022-ANCHOR **anchor** | пять фактов LEDGER-INTENT-v0.1 (партия 1c очереди группы B); измерения и готовые аннотации — `campaigns/packages-2026-09/harvest/d7a-core-sync-reverify.md`, раздел F-159 |
| ##B022-LOCATOR **locator** | движок кэша: `core-ai-native-specmap/src/ledger.rs`; на диске хранится текст без полей, чистки нет, метрик две из четырёх, срез не экспортируется и не подписывается, вид запроса — строка в теле функции |
| ##B022-SEVERITY **severity** | P2 |
| ##B022-DISPOSITION **disposition** | `open` — **решение владельца 2026-08-01: «давай положим в бэклог исследование»** (вместо немедленных аннотаций); обязательство F-159 в реестре — `deferred` до итогов |
| ##B022-FILED **filed by** | решение владельца 2026-08-01 по предъявленной партии 1c |

- ##B022-SUT **Суть, по-простому.** Документ про кэш сгенерированных объяснений обещает пять механизмов, которых нет: записи с полями происхождения (кто произвёл, какая модель, когда, почём), чистку кэша по давности с защитой от выселения нужного, полный набор метрик, подписанный «релизный срез» кэша при выпуске, и закрытый перечень видов запросов. Исследовать по образцу B-012: что из пяти реально строить, что чего требует, что честнее вычеркнуть.
- ##B022-COUPLING **Связки.** «Подписанный срез» — подмножество запаркованной программы безопасности [B-015](#b-015) (подписи нет нигде в дереве — не строить до уведомления владельца); поля происхождения пересекаются с B-020 (клиент внешних LLM захочет писать model_id в запись); вид-запроса-как-enum — дешёвый и независимый. Готовые тексты честных аннотаций (если исследование скажет «не строить») лежат в harvest и не применяются без владельца.

### B-023 — исследование: синтаксический уровень для JS/TS и Python-фронтенд {#b-023}

| | |
|---|---|
| ##B023-ANCHOR **anchor** | строки таблицы фронтендов ENGINE-CONFORM (партия 1b, пункты 1–2); измерения — `harvest/d7a-core-sync-reverify.md`, раздел F-146 |
| ##B023-LOCATOR **locator** | таблица обещает tree-sitter/SWC для TS/JS и RustPython/CPython-sidecar для Python; в дереве нет ни одного — семантический фронтенд TS (Compiler API через node-sidecar) есть и точен, Python-стека нет вовсе |
| ##B023-SEVERITY **severity** | P2 |
| ##B023-DISPOSITION **disposition** | `open` — **решение владельца 2026-08-01: «давай в бэклог положим исследование, что мы можем реализовать синтаксически для JS/TS и PYTHON»**; два якоря F-146 ждут итогов |
| ##B023-FILED **filed by** | решение владельца 2026-08-01 по предъявленной партии 1b (пункты 1–2) |

- ##B023-SUT **Суть, по-простому.** Гейт качества читает код через «фронтенды» двух глубин: быстрый синтаксический разбор и глубокий семантический. Для TS/JS сегодня есть только глубокий (через компилятор TypeScript в node-процессе); быстрого нет. Для Python нет ничего. Исследовать: что даёт синтаксический уровень для TS/JS (tree-sitter или SWC — какие факты извлекаемы без компилятора, почём, какие лицензии/зависимости), и реализуем ли Python-фронтенд (RustPython-парсер in-process против CPython-sidecar по образцу ts-extract/go-extract), — с рекомендацией строить/не строить по каждому.
- ##B023-CONTEXT **Контекст.** Пакеты языков пишутся для внешних потребителей (наша база — не скамья для Go/TS, §3.8 кампании); прецедент sidecar-архитектуры двойной (ts-extract, go-extract) — Python-sidecar ляжет в готовую форму. До итогов исследования строки таблицы стоят как есть (drift-вердикты кампании остаются честными).

### B-024 — исследование: не вытесняют ли маркеры @stage/state lifecycle-статусы specmap {#b-024}

| | |
|---|---|
| ##B024-ANCHOR **anchor** | вопрос владельца 2026-08-01 к тексту EDGE-MODEL-EDGES (партия 1d): «не устарела ли вообще вся эта система с появлением синтаксиса вида @doc/done? Там же тоже есть свой tombstone» |
| ##B024-LOCATOR **locator** | две параллельные системы: kind-line-статусы specmap (`planned`/`disputed`; `ratified` — отсутствие, `retired` — tombstone; парсер готов, носителей 0 из 5 266) и хостовые маркеры PROP-043 `@stage/state` (весь корпус размечен; `void` — их tombstone) |
| ##B024-SEVERITY **severity** | P2 |
| ##B024-DISPOSITION **disposition** | `planned` — **решение владельца 2026-08-01 (вторая сессия): «предлагаю запланировать в бэклог свести стадии жизненного цикла в specmap к аналогичным в progress»** — направление выбрано: сводим словарь specmap к словарю progress (derive, not declare); исследовательская часть сужается до механики (как выводить; что делать с `disputed`, у которого аналога нет) |
| ##B024-FILED **filed by** | вопрос владельца 2026-08-01, зафайлен как исследование; повышен до `planned` его же решением в тот же день |

- ##B024-SUT **Суть, по-простому.** В проекте два способа сказать «в каком состоянии кусок спеки». Маркеры `@stage/state` — прогресс каждого факта (насколько сделано: spec/impl/doc × done/work/…), живут на всём корпусе, `void` — их могильный камень. Статусы specmap — контрактное состояние секции для машины трассировки (`planned` — задумано, `disputed` — оспорено парой, `retired` — второй могильный камень), задуманы, чтобы управлять рёбрами графа (заморозка связей в спорные секции, отдельный учёт planned в покрытии — механики не построены), и не носятся ни одной секцией. **Два tombstone на одно понятие — реальная дупликация**; `planned` перекрывается со стадиями маркеров; уникален только `disputed` (пара конфликтующих секций аналога в маркерах не имеет).
- ##B024-QUESTION **Вопрос исследования.** Может ли машина трассировки **читать хостовые маркеры** вместо собственной параллельной системы (derive, not declare): `void` ⇒ retired, стадия/state ⇒ planned-эквивалент, а `disputed` — единственное, что останется собственным словарём specmap? Если да — kind-line-статусы сокращаются до `disputed`, и разметка B-019(а)-twin (метки ~80 секций) дешевеет. Если нет — записать, почему двум системам жить (разные предметы: прогресс факта ≠ контрактный статус юнита), и развести их словари явно.

### B-025 — находки гейта: помечать признанные отступления, а не гасить {#b-025}

| | |
|---|---|
| ##B025-ANCHOR **anchor** | факт «цепочка из пяти звеньев» ENGINE-CONFORM (партия 1b, пункт 5); измерения — `harvest/d7a-core-sync-reverify.md`, раздел F-146 |
| ##B025-LOCATOR **locator** | сегодня записанное отступление ГАСИТ находку на этапе правила (`in_deviation`, `conform/src/facts.rs:62`) — метка «deviation-acknowledged» не рождается никогда; поля вовлечённых фактов у находки нет (`Finding` = rule/file/line/message/why/fingerprint) |
| ##B025-SEVERITY **severity** | P2 |
| ##B025-DISPOSITION **disposition** | `planned` — **решение владельца 2026-08-01, пункт 5 = (б): «помечать вместо гасить. Я в будущем хочу сделать инструменты визуализации, и просто убирать неприменимые факты из IR это плохо, нужно всё видеть»** |
| ##B025-FILED **filed by** | решение владельца 2026-08-01 по партии 1b |

- ##B025-SUT **Суть, по-простому.** Когда правило гейта находит нарушение, а рядом есть записанное отступление («мы так делаем сознательно, вот причина»), гейт сейчас просто не рождает находку — она исчезает из всех данных. Строим наоборот: находка рождается всегда, но помечается «отступление признано» и не валит гейт. Тогда инструменты визуализации видят полную картину — сколько нарушений, сколько из них признанных, где; ничего не выпадает из IR.
- ##B025-BUILD **Что строить.** Статус-поле (или отдельный класс) у находки; правило перестаёт фильтровать по `in_deviation` и вместо этого штампует статус; baseline/ratchet учитывает «признанные» отдельно (не считает их новыми нарушениями); SARIF-рендер несёт статус. Заодно — поле вовлечённых фактов (второе недостающее звено той же цепочки). Обязательство F-146 частично ждёт этой стройки (якорь цепочки остаётся drift до неё).

### B-026 — ингест SARIF: диагнозы чужих линтеров становятся фактами гейта {#b-026}

| | |
|---|---|
| ##B026-ANCHOR **anchor** | факт «foreign linters as evidence providers» ENGINE-CONFORM (партия 1b, пункт 6); измерения — `harvest/d7a-core-sync-reverify.md`, раздел F-206 |
| ##B026-LOCATOR **locator** | SARIF сегодня только пишется (`sarif::render` — единственная публичная функция модуля), парсера нет ни в одном слое; clippy/eslint бегут floor-шагами, их вывод никуда не попадает |
| ##B026-SEVERITY **severity** | P2 |
| ##B026-DISPOSITION **disposition** | `planned`, **высокий приоритет — решение владельца 2026-08-01, пункт 6: «Построить ингест SARIF в будущем, с высоким приоритетом поместить это в бэклог»** |
| ##B026-FILED **filed by** | решение владельца 2026-08-01 по партии 1b |

- ##B026-SUT **Суть, по-простому.** Чужие линтеры (clippy, eslint, в перспективе ruff/clang-tidy) уже бегут рядом с гейтом, но гейт их результатов не видит. Строим чтение SARIF (стандартный формат отчётов статанализа): диагнозы чужих линтеров превращаются в факты гейта, и правила Дисциплины могут на них ссылаться («цитируем clippy, не переизобретаем его» — ровно та постура, которую документ всегда декларировал).
- ##B026-BUILD **Что строить.** SARIF-парсер (или зависимость serde-схемы), маппинг диагноза → `Fact` (какой линтер, какой rule id, файл/строка/сообщение), точка входа (floor-шаг складывает отчёты линтеров, conform их читает), и словарь цитирования в правилах (`check: { tool, id, status }` — форма уже описана в документе и нигде не построена). После стройки якорь foreign-linters пере-суживается по построенному; обязательство F-206 в реестре — `deferred` до тех пор.

### B-027 — аудит маркеров у «Specified, not built»: смысл против буквы {#b-027}

| | |
|---|---|
| ##B027-ANCHOR **anchor** | вопрос владельца 2026-08-01 (партия 1b, пункт 7): «Я не понимаю, почему у specified not built статус @impl/done, если спецификация не реализована — то это же @spec/done или @impl/planned?» |
| ##B027-LOCATOR **locator** | аннотированные факты несут маркеры вразнобой: часть @spec/done (партия 1a, D14-семья), часть @impl/done (например, DISTRIBUTION-RIDES в PROP-014, RULE-MULTIPLICITY-LINT, QUERY-ERROR-PROVENANCE, LLM-AS-RENDERER, RUNTIME-TRANSPORT); четыре закрывающих правила-собрата запечатаны с @impl/done |
| ##B027-SEVERITY **severity** | P2 |
| ##B027-DISPOSITION **disposition** | `open` — **владелец 2026-08-01: «положить в бэклог … Возможно нам нужно поменять у них статусы чтобы было правильно по смыслу»** |
| ##B027-FILED **filed by** | вопрос владельца 2026-08-01, зафайлен как аудит-задача |

- ##B027-SUT **Суть, по-простому.** Владелец прав: `@impl/done` на факте, чей механизм не построен, — семантически ложь (маркер утверждает «стадия реализации завершена»). Разнобой — историческая случайность, не дизайн: партия 1a ставила одним фактам @spec/done, другим оставила @impl/done; закрывающие правила держат @impl/done на том основании, что само ПРАВИЛО (как амендировано) в силе. Грамматика маркеров уже несёт нужные слова: стадии `idea<spec<impl<test<doc<freeze`, состояния `hold<plan<work<done<void` — то есть догадка владельца «@impl/planned» существует в форме **`@impl/plan`**.
- ##B027-RULE-PROPOSAL **Предлагаемое правило для аудита (утвердить перед свипом):** «specified, not built, стройка НЕ планируется» → `@spec/done`; «specified, not built, стройка запланирована (есть запись в бэклоге)» → `@impl/plan` — тогда маркер сам показывает, что реализация в плане (B-016…B-021, B-025, B-026 — их якоря получат @impl/plan с именем записи). Закрывающие правила пяти документов решаются одним решением на семью. Свип механический после утверждения правила; каждое изменение маркера — пере-суд якоря (D14-порядок: mirror → merge → seal).

### B-028 — грамматика spec://-адресов: пакет публикует подмножество того, что реализует хост {#b-028}

| | |
|---|---|
| ##B028-ANCHOR **anchor** | секция URI-схемы в `addressable-specs` (ADDRESSABLE-SPECS-PROTOCOL) против хостовой грамматики PROP-035 `##UNIFIED-GRAMMAR`; замечено re-verify-проходом волны 7 (`harvest/d7b-addressing-naming-sync-reverify.md`, раздел F-169, «New obligation noticed») |
| ##B028-LOCATOR **locator** | пакет публикует `spec://<module>/<doc>#<section>`; хост реализует строгий суперсет `spec://<group>/<name>[@<version>]/<doc-path>#<anchor>[.<sub>…][~r<N>]` — опциональная версия, многосегментный путь, revision-pin; пакетная секция не упоминает ни одного из трёх расширений |
| ##B028-SEVERITY **severity** | P2 |
| ##B028-DISPOSITION **disposition** | `open`, **высокий приоритет — решение владельца 2026-08-02: «положи в бэклог с высоким приоритетом»** |
| ##B028-FILED **filed by** | решение владельца 2026-08-02 по предъявлению группы C/D |

- ##B028-SUT **Суть, по-простому.** Флоу адресуемых спек учит консьюмеров грамматике ссылок — но учит **урезанной версии**: наш собственный резолвер понимает ещё версию пакета (`@0.8.0`), путь из нескольких сегментов и пин ревизии (`~r2`). Пакетная версия не ложна (подмножество), но продаётся как целое. Вопрос: нести ли флоу полную грамматику?
- ##B028-STAKES **Что решение тянет.** Если «да» — это release event: секция URI-схемы переписывается, и **redbook пересказывает схему в двух главах** (те тоже двигаются — как раз класс «одна норма в трёх местах», который кампания выжигает). Если «нет» — записать явно, что пакет публикует базовую грамматику, а расширения — хостовое superset-расширение (одна оговорка в пакетной секции + ссылка). Обе развязки закрывают вопрос честно; открытым он оставляет грамматику раздвоенной.
- ##B028-RELATED **Смежное.** Вопрос сегментов `<module>`/`<doc>` (единый namespace хоста, усечённые имена документов — F-169/F-147) — соседний, но отдельный: там спор о **значениях** сегментов, здесь — о **составе** грамматики. Решения независимы.

## P3 — accepted, no action planned {#p3}

*(empty)*
