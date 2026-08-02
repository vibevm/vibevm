# Design-rationale specs

<status stage="doc" state="done" comment="pilot markup 2026-07-24: living genre guide + index; grows with each captured design session"/>

##genre-definition This directory holds vibevm's **design-rationale** documents: the *why* and the *lore* behind vibevm's own architectural decisions — the path of a design discussion, the forks weighed and rejected, the precedents studied, the owner's mental model, and the ideas parked for later. It is the **design-doc genre** of the `spec-genres` flow this project follows: `spec://org.vibevm.world/spec-genres/flows/spec-genres/SPEC-GENRES-PROTOCOL#root`. @doc/done

##PROP-WINS-PRECEDENCE These documents are **non-normative**. The contract — *what* the system does — lives in the PROP / FEAT documents under [`spec/modules/`](../modules/) and [`spec/common/`](../common/); a `spec/design/` document explains *why a PROP is shaped the way it is*. When a design document and its PROP disagree, **the PROP wins** and the design document is corrected — the flow's precedence law (`spec://org.vibevm.world/spec-genres/flows/spec-genres/SPEC-GENRES-PROTOCOL#precedence`): load-bearing rationale — the decision itself and the alternatives weighed, in each PROP's `Decision` / `Rejected alternatives` sections (the **decision-records** genre: `spec://org.vibevm.world/decision-records/flows/decision-records/DECISION-RECORDS-PROTOCOL#root`) — stays inside the PROP; the narrative lore moves out to here (`spec://org.vibevm.world/spec-genres/flows/spec-genres/SPEC-GENRES-PROTOCOL#contract-vs-lore`). @doc/done

## vibevm's spec/ genres

<status stage="doc" state="done"/>

##genre-table-lead vibevm's instance of the genre table — the general taxonomy (each genre's charter, mutability, reader, and authority-on-conflict) is the flow's `spec://org.vibevm.world/spec-genres/flows/spec-genres/SPEC-GENRES-PROTOCOL#genres`: @doc/done

| Directory | Holds | Normative? |
|---|---|---|
| ##ROW-BOOT [`boot/`](../boot/) @doc/done | Session-boot instructions read at the start of every session @doc/done | yes @doc/done |
| ##ROW-COMMON [`common/`](../common/) @doc/done | Foundation decisions crossing every crate (PROP-000, PROP-006) @doc/done | yes @doc/done |
| ##ROW-MODULES [`modules/`](../modules/) @doc/done | Per-crate PROP / FEAT — the implementation contract @doc/done | yes @doc/done |
| ##ROW-NO-DECISIONS `decisions/` — **never created, and never will be** @doc/done | Nothing. A reopenable decision's four-field record lives **inside the `common/` or `modules/` section that governs the value** — the installed flow's `#NO-SEPARATE-ADR-DIRECTORY` and `#SUM-RECORDS-LIVE-AT-THE-ANCHOR`. Do not search for this directory; its absence is the design @doc/done | n/a @doc/done |
| ##ROW-RESEARCH [`legacy-spec/research/`](../../legacy-spec/research/) — archived 2026-07-25 @doc/done | Backgrounders on **external** systems (Tessl, threat models, prior-art surveys) @doc/done | no @doc/done |
| ##ROW-DESIGN `design/` (this directory) @doc/done | Rationale for vibevm's **own** decisions — the why and the lore behind our PROPs @doc/done | no @doc/done |
| ##ROW-WAL [`WAL.md`](../WAL.md) @doc/done | Volatile current-state checkpoint, rewritten each session @doc/done | n/a @doc/done |

##research-vs-design `legacy-spec/research/` (archived) and `design/` are both non-normative, but they look in opposite directions: the archived research studies what *other* projects did; `design/` records why *we* chose what we chose. @doc/done

##NO-DECISIONS-DIRECTORY **Decision records are a section form, not a directory.** A reopenable choice in [`common/`](../common/) or [`modules/`](../modules/) carries its four fields — Decision · Why · Considered and rejected · Revisit when — **inside the section that governs the value**, per `spec://org.vibevm.world/decision-records/flows/decision-records/DECISION-RECORDS-PROTOCOL#placement`. There is no `spec/decisions/` directory and there will not be one (`#NO-SEPARATE-ADR-DIRECTORY`); a silo *«preserves reasoning technically and loses it practically»* (`#SILOS-PRESERVE-TECHNICALLY-AND-LOSE-PRACTICALLY`), because nothing at the anchor points at it. The flow's map assigns the mutability **«amended by decision record»** to the *Foundational decisions* genre — `common/` in this instance; `modules/` keeps **«edit + changelog line»**. Which sections are owed a record is decided by [the criterion below](#owed-a-record), not by the presence of a `Decision` label. @doc/done

## Which decisions are owed a record {#owed-a-record}

<status stage="doc" state="done"/>

##criterion-lead The installed `decision-records` flow requires a four-field
record — Decision · Why · Considered and rejected · When to revisit — for **any
choice a future reader could plausibly re-open**
(`spec://org.vibevm.world/decision-records/flows/decision-records/DECISION-RECORDS-PROTOCOL#when`,
read at every session boot from `spec/boot/STATIC.md`). *Plausibly* is the word
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

##FORWARD-ONLY-FROM-THE-RULING **The four fields are required forward-only, and existing sections are classified rather than edited.** From this ruling forward, any decision **newly taken or reopened** in [`common/`](../common/) or [`modules/`](../modules/) that passes the criterion above carries all four fields **at the moment it is minted** — same session, before it ends (`#WRITE-IN-THE-SESSION-THAT-DECIDES`), because the reasoning is in working memory then and nowhere afterwards. Sections written before the ruling are **classified by the criterion, not rewritten**: the reason is the flow's own, not a budget — *"a record backfilled a week later is fiction with confidence"* (`#backfilled-reasoning-is-fiction`), and a *why* or a trigger reconstructed from a document by someone who was not in the room is `#ROW-ANTI-BACKFILLED-MEMORY`, not a record. The one scheduled exception is [`common/`](../common/), the genre the installed map already declares *"amended by decision record"*: its reopenable sections were classified and backfilled in full, from their own recorded prose, in one pass. Everywhere else the debt drains opportunistically — `#RHYTHM-OPPORTUNISTIC`: a session that edits a `Decision`-bearing section completes its record **if the reasoning is already in the document**, and otherwise files the gap rather than inventing one. A record whose *why* cannot be sourced is not written; it is asked (`#ASK-RATHER-THAN-INVENT-DATA`). @doc/done

## Linking rule

<status stage="doc" state="done"/>

##TWO-WAY-LINKING Every `spec/design/` document names the PROP(s) it explains; every PROP it explains links back to it from its `Related` header — the flow's two-way linking law (`spec://org.vibevm.world/spec-genres/flows/spec-genres/SPEC-GENRES-PROTOCOL#linking`), so a session that reads a PROP during the boot sequence finds the rationale without being told it exists. A one-directional link is a latent break. @doc/done

## When to write a document here

<status stage="doc" state="done"/>

##when-to-write When a design discussion produces more reasoning than a PROP can absorb without losing its contract readability — a multi-fork design session, a large refactor weighed against several alternatives, a decision whose context would otherwise live only in one conversation and be lost at the next session boundary. (The general decision table is the flow's `when-to-write-what` document.) @doc/done

## Index

<status stage="doc" state="work" comment="living index — every new design doc adds a row; checked complete against the directory 2026-07-24"/>

- ##idx-workspace-naming [Workspace & qualified naming](workspace-and-qualified-naming.md) — rationale for [PROP-007](../modules/vibe-workspace/PROP-007-workspace.md) (workspace) and [PROP-008](../modules/vibe-registry/PROP-008-qualified-naming.md) (qualified naming): the owner's Maven-submodules + cargo mental model, the four-axis decomposition, the fork-by-fork decision record, the Cargo-vs-Maven precedent lore, the physical-publication model, and ideas parked for later. Captured 2026-05-20. @doc/done

- ##idx-loading-boot [Loading & boot composition model](loading-and-boot-model.md) — rationale for PROP-009 (loading model): why the flat boot model fails under a workspace, the static/dynamic linking spine, the two-trees + computed-index design, the three inclusion types (`inline` / `static` / `dynamic`) and the `STATIC.md` priority lane, and the fork-by-fork record. Captured 2026-05-21. @doc/done

- ##idx-action-system [The action system](action-system.md) — rationale + architecture for [PROP-039](../modules/vibe-actions/PROP-039-action-system.md) (the `vibe-actions` contract): the addressable, frontend-agnostic, programmatically-drivable behaviour layer (`action://`) — the behaviour-layer twin of `spec://`. The crate/module architecture, the core types, the MVC-plus data flow (the model is the real interface), the ten design decisions (URI address grammar, collision-erroring registry, typed pure enablement, primary programmatic invocation + the **headless AIUI reference surface**, the two-phase Search Everywhere provider seam, address-keyed i18n, …), the Search Everywhere architecture (packages + every card-field + actions now, structural/AI-Native later through one seam), and the AIUI surface. Derived clean-room from the [VSCode/IntelliJ study](../../legacy-spec/research/action-systems-vscode-idea.md). Captured 2026-07-15. @doc/done

- ##idx-tui-visual [TUI visual language](tui-visual-language.md) — the shared visual conventions of the `vibe` TUIs. @doc/done

- ##idx-structural-loader [Structural loader](structural-loader.md) — provisional loader instructions held for PROP-035; not yet wired into any live boot. @spec/hold

- ##idx-tooling-map [The tooling development map](../../TOOLING-MAP.md) — the B-041 synthesis: the four planes of the discipline tooling (loading/addressing, the conform gate, specmap, the agent runtime, plus the parked security overlay), each mechanism's measured state against its backlog build entry, the dependency spine, the proposed waves А–Г, the ten owner forks, and the five observable «system is good» milestones. Commissioned by the owner 2026-08-02 («Мне нужно понимание, как развивать вообще наш инструментарий»); approved and **moved to the repository root beside the backlog it arranges** the same day, by his integration direction — the one design-genre document living outside this directory. Framed by the running campaign: waves execute through campaign phases, nothing starts from the map. Captured 2026-08-02. @doc/work

- ##idx-deterministic-loading [Deterministic loading — qualified anchors and aliases](deterministic-loading-aliasing.md) — the B-011 design proposal (wave А's opener, the owner's highest priority): qualified rename-on-splice killing the compiled lane's 59 duplicate-anchor collisions by construction, `#use … as X` / `@!X` aliases bound to source-of-truth addresses (strip-proof and splice-proof), the two-scope lookup rule (the ADL analogy narrowed to a loading contract), and the append-only dynamic-`STATIC.md` composition case. Names the exact PROP-035 / PROP-009 §2.3 edits that become contract on approval, the four owner forks, and the four-slice worker cut. Captured 2026-08-03; **awaiting the owner's ruling.** @spec/work
