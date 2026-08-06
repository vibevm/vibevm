# Design-rationale specs

<status stage="doc" state="done" comment="pilot markup 2026-07-24: living genre guide + index; grows with each captured design session"/>

@fact:genre-definition This directory holds vibevm's **design-rationale** documents: the *why* and the *lore* behind vibevm's own architectural decisions — the path of a design discussion, the forks weighed and rejected, the precedents studied, the owner's mental model, and the ideas parked for later. It is the **design-doc genre** of the `spec-genres` flow this project follows: `spec://org.vibevm.world/spec-genres/flows/spec-genres/SPEC-GENRES-PROTOCOL#root`. @status:doc/done

@fact:PROP-WINS-PRECEDENCE These documents are **non-normative**. The contract — *what* the system does — lives in the PROP / FEAT documents under [`spec/modules/`](../modules/) and [`spec/common/`](../common/); a `spec/design/` document explains *why a PROP is shaped the way it is*. When a design document and its PROP disagree, **the PROP wins** and the design document is corrected — the flow's precedence law (`spec://org.vibevm.world/spec-genres/flows/spec-genres/SPEC-GENRES-PROTOCOL#precedence`): load-bearing rationale — the decision itself and the alternatives weighed, in each PROP's `Decision` / `Rejected alternatives` sections (the **decision-records** genre: `spec://org.vibevm.world/decision-records/flows/decision-records/DECISION-RECORDS-PROTOCOL#root`) — stays inside the PROP; the narrative lore moves out to here (`spec://org.vibevm.world/spec-genres/flows/spec-genres/SPEC-GENRES-PROTOCOL#contract-vs-lore`). @status:doc/done

## vibevm's spec/ genres

<status stage="doc" state="done"/>

@fact:genre-table-lead vibevm's instance of the genre table — the general taxonomy (each genre's charter, mutability, reader, and authority-on-conflict) is the flow's `spec://org.vibevm.world/spec-genres/flows/spec-genres/SPEC-GENRES-PROTOCOL#genres`: @status:doc/done

| Directory | Holds | Normative? |
|---|---|---|
| @fact:ROW-BOOT [`boot/`](../boot/) @status:doc/done | Session-boot instructions read at the start of every session @status:doc/done | yes @status:doc/done |
| @fact:ROW-COMMON [`common/`](../common/) @status:doc/done | Foundation decisions crossing every crate (PROP-000, PROP-006) @status:doc/done | yes @status:doc/done |
| @fact:ROW-MODULES [`modules/`](../modules/) @status:doc/done | Per-crate PROP / FEAT — the implementation contract @status:doc/done | yes @status:doc/done |
| @fact:ROW-NO-DECISIONS `decisions/` — **never created, and never will be** @status:doc/done | Nothing. A reopenable decision's four-field record lives **inside the `common/` or `modules/` section that governs the value** — the installed flow's `#NO-SEPARATE-ADR-DIRECTORY` and `#SUM-RECORDS-LIVE-AT-THE-ANCHOR`. Do not search for this directory; its absence is the design @status:doc/done | n/a @status:doc/done |
| @fact:ROW-RESEARCH [`legacy-spec/research/`](../../legacy-spec/research/) — archived 2026-07-25 @status:doc/done | Backgrounders on **external** systems (Tessl, threat models, prior-art surveys) @status:doc/done | no @status:doc/done |
| @fact:ROW-DESIGN `design/` (this directory) @status:doc/done | Rationale for vibevm's **own** decisions — the why and the lore behind our PROPs @status:doc/done | no @status:doc/done |
| @fact:ROW-WAL [`WAL.md`](../WAL.md) @status:doc/done | Volatile current-state checkpoint, rewritten each session @status:doc/done | n/a @status:doc/done |

@fact:research-vs-design `legacy-spec/research/` (archived) and `design/` are both non-normative, but they look in opposite directions: the archived research studies what *other* projects did; `design/` records why *we* chose what we chose. @status:doc/done

@fact:NO-DECISIONS-DIRECTORY **Decision records are a section form, not a directory.** A reopenable choice in [`common/`](../common/) or [`modules/`](../modules/) carries its four fields — Decision · Why · Considered and rejected · Revisit when — **inside the section that governs the value**, per `spec://org.vibevm.world/decision-records/flows/decision-records/DECISION-RECORDS-PROTOCOL#placement`. There is no `spec/decisions/` directory and there will not be one (`#NO-SEPARATE-ADR-DIRECTORY`); a silo *«preserves reasoning technically and loses it practically»* (`#SILOS-PRESERVE-TECHNICALLY-AND-LOSE-PRACTICALLY`), because nothing at the anchor points at it. The flow's map assigns the mutability **«amended by decision record»** to the *Foundational decisions* genre — `common/` in this instance; `modules/` keeps **«edit + changelog line»**. Which sections are owed a record is decided by [the criterion below](#owed-a-record), not by the presence of a `Decision` label. @status:doc/done

## Which decisions are owed a record {#owed-a-record}

<status stage="doc" state="done"/>

@fact:criterion-lead The installed `decision-records` flow requires a four-field
record — Decision · Why · Considered and rejected · When to revisit — for **any
choice a future reader could plausibly re-open**
(`spec://org.vibevm.world/decision-records/flows/decision-records/DECISION-RECORDS-PROTOCOL#when`,
read at every session boot from `spec/boot/STATIC.md`). *Plausibly* is the word
two reviewers disagree on, so vibevm decides it with three questions. **A
section is owed a record iff all three can be answered; «I cannot name it» is a
No.** @status:doc/done

| | The question | What must be named | Fails when |
|---|---|---|---|
| @fact:Q1-CONDITION **Q1 · the condition** @status:doc/done | Complete *«this stands while X»*, where X is **outside this project's own definitions** @status:doc/done | an external dependency, a measured value, or a stated scope limit (`in v1`, `until`, `v2+`) @status:doc/done | X is a term we defined — changing it is a migration, not a re-decision @status:doc/done |
| @fact:Q2-OBSERVATION-POINT **Q2 · the observation point** @status:doc/done | Could a stranger check X **today**? @status:doc/done | a file, a command, an upstream repository, a version, or a measurement this project already takes @status:doc/done | nothing observes X — the record would carry the unobservable trigger `#UNOBSERVABLE-TRIGGERS-ARE-AS-BAD-AS-NONE` forbids @status:doc/done |
| @fact:Q3-LOSER **Q3 · the loser** @status:doc/done | Was an alternative **available at the time**, and could someone re-propose it? @status:doc/done | one named alternative and the reason it lost @status:doc/done | nothing was in contention — the section is a fact wearing a `Decision` label, and `#DO-NOT-RECORD-WHAT-HAS-NO-PLAUSIBLE-ALTERNATIVE` says do not record it @status:doc/done |

@fact:CRITERION-IS-THE-RECORD-SHAPE The three questions are the flow's own record
with the *Decision* removed: Q1 + Q2 are `#A-TRIGGER-HAS-THREE-PARTS`, Q3 is
`#ROW-FIELD-CONSIDERED-AND-REJECTED`. A section is therefore owed a record
**precisely when a record could be written for it without inventing data** —
which `#ASK-RATHER-THAN-INVENT-DATA` demands anyway. @status:doc/done

@fact:CRITERION-UNIT-IS-THE-CLAIM **The unit is the claim, not the section.** One
section may hold a definitional claim and a reopenable one: PROP-002 §2.1 fixes
the identity tuple (definitional) *and* fixes `content_hash` as `sha256:<hex>`
(a library-grade choice against an external algorithm). Classify each claim. @status:doc/done

@fact:criterion-pass-lead A section **passes** in one of three ways: @status:doc/done

- @fact:R1-EXTERNAL **R1 · external dependency** — the choice is forced by a third
  party's behaviour, licence, format, limit or version. The third party can
  change without asking us. @status:doc/done
- @fact:R2-MEASURED-THRESHOLD **R2 · measured threshold** — the value is a number or
  cut-off chosen against an observation. New measurement moves it. @status:doc/done
- @fact:R3-LIVE-ALTERNATIVE **R3 · live alternative** — a named alternative was
  declined for a contingent reason (cost, effort, immaturity, «not needed
  yet»). **An explicit deferral is always R3**:
  `#MARK-HONEST-DEFERRALS-AS-DEFERRED-NOT-REJECTED` — a deferral has a built-in
  revisit. @status:doc/done

@fact:criterion-fail-lead It **fails** in one of three ways, and a classifier cites
the one it used: @status:doc/done

- @fact:N1-DEFINITIONAL **N1 · definitional** — the section fixes a term, a grammar
  or an identity everything downstream is keyed on. Changing it is a migration,
  not a re-decision. *(Fails Q1: X is ours.)* @status:doc/done
- @fact:N2-DOWNSTREAM **N2 · downstream of a recorded choice** — the reopenable
  point is upstream; a record here creates a second writer for one fact.
  *(Fails Q3: the loser belongs to the upstream section.)* @status:doc/done
- @fact:N3-DESCRIPTIVE **N3 · descriptive** — the `Decision` label is doing a
  heading's work over a layout, a schema, a route table, a command surface.
  *(Fails Q3.)* @status:doc/done

@fact:criterion-worked-lead Two worked examples, one each way: @status:doc/done

- @fact:ex-shippable-tree **R — `PROP-024` §2.2 `#shippable-tree`.** The shippable
  tree is the package directory minus `.git/ .vibe/ target/ node_modules/`.
  **Q1** — four literal build-output names from four external toolchains; a
  fifth toolchain adds a fifth. **Q2** — the denylist itself, and any package
  whose hash moves between clean and built. **Q3** — two, both already written
  in §4: hashing build output (`#REJ-HASH-BUILD-OUTPUT`) and a per-file
  allow-list (`#REJ-ALLOW-LIST`). Owed a record. @status:doc/done
- @fact:ex-layout **N — `PROP-000` §5 `#layout`.** *«@fact:LAYOUT-PER-SPEC **Decision:**
  Per `VIBEVM-SPEC.md` §4.2.»* **Q3** — the line delegates to another document;
  nothing was in contention. **##N3-DESCRIPTIVE.** Not owed a record, and this
  is why a raw count of `Decision` labels overstates the debt. @status:doc/done

@fact:COUNT-IS-DERIVED-NOT-MAINTAINED **The negative classification is derived, not
recorded.** Sections that fail the criterion carry **no** marker saying so —
writing `**Revisit when:** never` across them would be ceremony, and
`#ROW-TRIGGER-LATER` makes «never» read as a defect to every future reader.
Instead, any census of the decision debt applies this criterion and reports
«N reopenable, M complete»; a raw count of `Decision` labels is not that number
and must not be quoted as it. (`BACKLOG.md` `#ENTRY-PREFER-GENERATED` — prefer
generated over hand-maintained.) @status:doc/done

@fact:HONEST-NEVER-IS-NOT-A-DEFECT **A section may state «never» in the trigger
field and be correct.** `PROP-000` `##LANG-REVISIT` — *«Never, in the scope of
v1. If Rust proves inadequate for a future milestone, open a new PROP
superseding this one»* — is an honest @fact:N1-DEFINITIONAL verdict written in the
trigger's slot, together with the escape route. Read against `#ROW-TRIGGER-LATER`
it looks like an anti-pattern; read against this criterion it is the correct
answer, and it is not scored as debt. @status:doc/done

@fact:FORWARD-ONLY-FROM-THE-RULING **The four fields are required forward-only, and existing sections are classified rather than edited.** From this ruling forward, any decision **newly taken or reopened** in [`common/`](../common/) or [`modules/`](../modules/) that passes the criterion above carries all four fields **at the moment it is minted** — same session, before it ends (`#WRITE-IN-THE-SESSION-THAT-DECIDES`), because the reasoning is in working memory then and nowhere afterwards. Sections written before the ruling are **classified by the criterion, not rewritten**: the reason is the flow's own, not a budget — *"a record backfilled a week later is fiction with confidence"* (`#backfilled-reasoning-is-fiction`), and a *why* or a trigger reconstructed from a document by someone who was not in the room is `#ROW-ANTI-BACKFILLED-MEMORY`, not a record. The one scheduled exception is [`common/`](../common/), the genre the installed map already declares *"amended by decision record"*: its reopenable sections were classified and backfilled in full, from their own recorded prose, in one pass. Everywhere else the debt drains opportunistically — `#RHYTHM-OPPORTUNISTIC`: a session that edits a `Decision`-bearing section completes its record **if the reasoning is already in the document**, and otherwise files the gap rather than inventing one. A record whose *why* cannot be sourced is not written; it is asked (`#ASK-RATHER-THAN-INVENT-DATA`). @status:doc/done

## Linking rule

<status stage="doc" state="done"/>

@fact:TWO-WAY-LINKING Every `spec/design/` document names the PROP(s) it explains; every PROP it explains links back to it from its `Related` header — the flow's two-way linking law (`spec://org.vibevm.world/spec-genres/flows/spec-genres/SPEC-GENRES-PROTOCOL#linking`), so a session that reads a PROP during the boot sequence finds the rationale without being told it exists. A one-directional link is a latent break. @status:doc/done

## When to write a document here

<status stage="doc" state="done"/>

@fact:when-to-write When a design discussion produces more reasoning than a PROP can absorb without losing its contract readability — a multi-fork design session, a large refactor weighed against several alternatives, a decision whose context would otherwise live only in one conversation and be lost at the next session boundary. (The general decision table is the flow's `when-to-write-what` document.) @status:doc/done

## Index

<status stage="doc" state="work" comment="living index — every new design doc adds a row; checked complete against the directory 2026-07-24"/>

- @fact:idx-workspace-naming [Workspace & qualified naming](workspace-and-qualified-naming.md) — rationale for [PROP-007](../modules/vibe-workspace/PROP-007-workspace.md) (workspace) and [PROP-008](../modules/vibe-registry/PROP-008-qualified-naming.md) (qualified naming): the owner's Maven-submodules + cargo mental model, the four-axis decomposition, the fork-by-fork decision record, the Cargo-vs-Maven precedent lore, the physical-publication model, and ideas parked for later. Captured 2026-05-20. @status:doc/done

- @fact:idx-loading-boot [Loading & boot composition model](loading-and-boot-model.md) — rationale for PROP-009 (loading model): why the flat boot model fails under a workspace, the static/dynamic linking spine, the two-trees + computed-index design, the three inclusion types (`inline` / `static` / `dynamic`) and the `STATIC.md` priority lane, and the fork-by-fork record. Captured 2026-05-21. @status:doc/done

- @fact:idx-action-system [The action system](action-system.md) — rationale + architecture for [PROP-039](../modules/vibe-actions/PROP-039-action-system.md) (the `vibe-actions` contract): the addressable, frontend-agnostic, programmatically-drivable behaviour layer (`action://`) — the behaviour-layer twin of `spec://`. The crate/module architecture, the core types, the MVC-plus data flow (the model is the real interface), the ten design decisions (URI address grammar, collision-erroring registry, typed pure enablement, primary programmatic invocation + the **headless AIUI reference surface**, the two-phase Search Everywhere provider seam, address-keyed i18n, …), the Search Everywhere architecture (packages + every card-field + actions now, structural/AI-Native later through one seam), and the AIUI surface. Derived clean-room from the [VSCode/IntelliJ study](../../legacy-spec/research/action-systems-vscode-idea.md). Captured 2026-07-15. @status:doc/done

- @fact:idx-tui-visual [TUI visual language](tui-visual-language.md) — the shared visual conventions of the `vibe` TUIs. @status:doc/done

- @fact:idx-structural-loader [Structural loader](structural-loader.md) — provisional loader instructions held for PROP-035; not yet wired into any live boot. @status:spec/hold

- @fact:idx-tooling-map [The tooling development map](../../TOOLING-MAP.md) — the B-041 synthesis: the four planes of the discipline tooling (loading/addressing, the conform gate, specmap, the agent runtime, plus the parked security overlay), each mechanism's measured state against its backlog build entry, the dependency spine, the proposed waves А–Г, the ten owner forks, and the five observable «system is good» milestones. Commissioned by the owner 2026-08-02 («Мне нужно понимание, как развивать вообще наш инструментарий»); approved and **moved to the repository root beside the backlog it arranges** the same day, by his integration direction — the one design-genre document living outside this directory. Framed by the running campaign: waves execute through campaign phases, nothing starts from the map. Captured 2026-08-02. @status:doc/work

- @fact:idx-deterministic-loading [Deterministic loading — qualified anchors and aliases](deterministic-loading-aliasing.md) — the B-011 design (wave А's opener, the owner's highest priority): qualified rename-on-splice killing the compiled lane's 59 duplicate-anchor collisions by construction, `#use … as X` / `@!X` aliases bound to source-of-truth addresses (strip-proof and splice-proof), the two-scope lookup rule (the ADL analogy narrowed to a loading contract), and the append-only dynamic-`STATIC.md` composition case. Captured 2026-08-03; **APPROVED by the owner 2026-08-04** («Принимаю дизайн B-011») with every recommended fork plus the priority-placement addition — the §7 PROP edits are contract, the five worker slices are landed, and the host lane compiles anchor-qualified (59 → 0). @status:spec/done

- @fact:idx-lane-dedup [Once-each lane composition — the aggregator double-emission](lane-composition-dedup.md) — the B-006 design proposal: the git commit-rules family enters the priority lane twice (four direct member contributions + the same members nested inside the `git-practices` unit artifact the emitter substitutes for the snippetless umbrella — 323 lines, 164 double-qualified labels, nine provenance markers for a five-package family). Names the exact mechanism (the closure walk and the unit-artifact substitution both firing on one static entry), why the two prior tasks (DRIFT-029/030) stopped for this ruling, the recommended compose-time elision rule (once-each, with the dynamic-member guard that never drops coverage), the three owner forks, the per-node-qualify rider (W3's named follow-up), and the two-packet worker cut. Captured 2026-08-04; **APPROVED by the owner the same day** («согласен с твоими рекомендациями a1 b1 c1») — the §6 contract edits are landed (PROP-009 §2.3 `##STATIC-EMITS-ONCE-EACH`, PROP-038 §2.1, PROP-035 §8's per-node refinement), the build landed the same day and the acceptance measured on the live lane (git-family markers 9 → 5, double-qualified labels 164 → 0, −404 lines). @status:spec/done

- @fact:idx-host-as-package [The host becomes a package — `org.vibevm.core/vibevm`](host-as-package.md) — the B-031 design proposal, standing on the E5 census (`campaigns/packages-2026-09/harvest/e5-b031-evidence.md`): the owner's fully-qualified-root direction mapped through the unified grammar, the one-line contract carve-out (PROP-029 `##SCOPE-HOST`) retired with a tombstone, the resolver's self-coordinate mapping replacing the host token, undotted authorities failing loudly with the rename hint, a scripted one-commit migration of ≈1 851 living-surface occurrences (historical evidence JSON untouched), and the F-169/F-147 re-judgements riding the landing. Names why truncation is NOT part of this build (already authority-agnostic; flow legalization = B-028), the three owner forks (coordinate form, legacy behaviour, migration perimeter), and the sequenced W1–W3 cut. Captured 2026-08-04; **APPROVED by the owner the same day** («1. координаты: группа org.vibevm.core, имя vibevm. 2. жесткая ошибка с подсказкой 3. все живые поверхности» + the personally-assigned §5.1 metadata check) — landed the same day: the 1 893-occurrence migration at residue 0, five fixture families re-pointed, the mass re-seal executed. @status:spec/done

- @fact:idx-gate-parity [Per-language gate units — config surface, coverage invariant, flag rule](gate-parity-config.md) — the B-029 + B-034 + B-039 design proposal (волна Б батч 1), standing on the E8 census pair (`harvest/e8-r1-config-census.md`, `harvest/e8-r2-gate-units-census.md`): only Rust has a gate unit and a coverage invariant; Go/TS scan flat file lists and pass silently on an empty scope; the literal FlagSites mount on TS is impossible (Rust-only facts). Proposes Go = package / TS = cell, per-language sections with idiomatic keys (`gated_packages` / `gated_cells` + their own exempt), the root table staying the flat Rust layer by recorded decision; a generic coverage validator speaking each language's noun; vacuous-green guards for both drivers; the TS flag rule built TS-shaped on a new env-read census kind; the Go sole-importer twin routed to batch 2. Carries the map's fork №2 — **RULED by the owner 2026-08-04** (units = each language's native one: crate/package/cell; homes = full symmetry under his quality bar, «расширяемо на новые языки (скоро добавится Python!)… Хочется сделать хорошо и надолго») — **and LANDED whole the same day** (engine v2 + three frontends + host migration + doc sweep + the TS flag rule; panel green; the B-035 loop pass measures the acceptance). @status:spec/done

- @fact:idx-seam-error-parity [Seam-error and conformance-assertion parity](seam-error-and-assertion-parity.md) — the B-033 + B-030 + B-049 design (волна Б батч 2), standing on the E11 census pair (`harvest/e11-r1-seam-errors-census.md`, `harvest/e11-r2-assertions-census.md`): the seam-error paradigm is one Rust fact / two rules / two halves, Go has only the structure half (a finding-kind with no id of its own) and TS has neither, while the conformance assertion `var _ Seam = (*Impl)(nil)` is a live Go idiom the gate is blind to. Proposes ONE Go rule `go-seam-error-cites-req` on both halves (per-half fingerprints; the message half needs `go-extract` to read `Error()` bodies), the TS twin built now conservatively with its limit recorded, a Go conformance-assertion presence rule with the Rust (compiler-is-the-assertion) and TS (routed type-level-test) survey verdicts recorded, and `[rust] floor_disable` (B-049). Carries the parity-principle LIFT — map fork №9 (taken 2026-08-04, «Ядро дисциплины») — as a boss contract diff into `00-MANIFESTO.md` §4, the stacks citing. Captured 2026-08-04; no owner map-fork inside the batch. @status:spec/work

- @fact:idx-map-format-change [The one map-format change — span and fingerprint, the map in the package, the privacy tier](map-format-change.md) — the B-019а + B-016.1 + B-017 design (волна В), standing on the E-V2/E-V3 census pair (`harvest/e17-map-format-census.md`, `harvest/e16-b024-lifecycle-vocab-census.md`) plus the fingerprint noise measurement. Six measured facts reshaped it: the manifest refuses unknown keys (so a key is a schema bump) while `min_vibe_version` already exists and is wired through publish and the registry, discharging B-017's stated precondition by measurement; a map inside a package feeds that package's identity hash with no exclusion path, and the fractality package already ships one; a package's spec namespace is **consumer-assigned**, so a self-shipped map has no authoritative address until a second manifest key adds one; and the code item has neither fingerprint nor end-of-range. Its load-bearing decision is that the fingerprint string is **self-describing** (`<scheme>:<hex>`), which makes a later change of substance a regeneration rather than another bump. Decides B-014's second half against a byte-compare gate (the index's heading line is documented volatile decoration; such a gate would fire on nearly every documentation commit) and for a content-only freshness check riding this change. Carries the map's forks №3 (fingerprint substance, measured first), №4 (fragment identity, explicitly not built here) and №5 (the `contract` tier's content, recommendation: define it over data the map already carries). @status:spec/work

- @fact:idx-new-rule-classes [The three new rule classes — comment position, custom lints, pending cards](new-rule-classes.md) — the B-036 + B-037 + B-038 design (волна Б батч 3), standing on the E13 census trio (`harvest/e13-r1-comment-position-census.md`, `e13-r2-custom-lints-census.md`, `e13-r3-pending-cards-census.md`). Its spine: each build is a card the Discipline names and never gave a checker — three of the seven «pending cards» plus Scaffold F's third channel. Records four measurements that reshaped the plan: the engine has **no severity class** and needs none (the ratchet freeze already is «warn, don't block»); `FileMetrics` already ships the position denominator while two of three extractors already walk every comment; the invariant-marker vocabulary is near-absent from our own tree, so the rule must be exhibited on fixtures; and `rust-toolchain.toml` pins `stable`, which blocks the `dylint` vehicle and routes B-037's Rust half to an owner toolchain decision while TS builds fully (the vehicle is already a demo devDependency). Carries the map's **fork №1 — TAKEN by the owner 2026-08-04: computed cell names** (`Pascal(variant)` + the seam as written), with the boss's whole-tree cost measured against the census's narrower one: 40 manifest-bearing cells, 14 already compliant, 13 production renames in the host. @status:spec/work

- @fact:idx-command-nodes [Command nodes in the map](command-nodes.md) — the B-019(б) design, standing on the M-B019B measurement (archived under `cache/agents/sorted/M-B019B/`). Two measurements decide most of it and both cut the price down: `item_kind` is an **open string** in the wire schema while its four neighbours carry `enum`s, and no production code matches or filters on it — so a new kind is a new value, not the schema bump B-019(а) was; and `explain`'s target grammar is one prefix test with no closed set of kinds behind it, so a node whose `symbol` is the invocation path (`vibe install`) is answerable through the existing symbol path with no change to `explain` at all. Recognition is by clap's `Subcommand` derive rather than by an author's marker, because a marker that a new subcommand can be added without is a norm with no checker, and both derive spellings in this tree must be read. Corrects the measurement's one wrong conclusion — the enum a command lives in **is** a `syn::Item` the walker already visits, so no pass parallel to `jtd.rs` is needed; the real obstacle is that `tag_item` records nothing untagged, and command nodes take the unconditional `record_item` path instead. Names the crate-wide join (root binary name ↔ group enums, across files) as the design's only structural cost, and cuts the build into three slices with the surfaces census's own numbers as acceptance (29 top-level, 68 nested). Captured 2026-08-06; part (в) explicitly not in it — its systems boundary is the owner's, before implementation. @status:spec/work
