# Old 113 task semantic-transfer crosswalk

Cumulative reconciliation. Initial whole-crosswalk pin `a344c6389fdb60a8db075f60b8a7c89712572916`; later scoped reviews carry their own pins. G123 review uses accepted main `5d44c5104`; the later app scope review uses `c5662ee42`, and G2 is accepted in `bc49f1220`. Unaccepted candidates are excluded; source-only reviews do not imply runtime acceptance.

**Current counting result:** 113 old accepted IDs verified: **12 full semantic transfers, 31 partial correspondences, 31 scopes with no accepted campaign delivery, 39 not yet fully reconciled**. Root accepted M-01-A.3 in3be4a0590 after checking its exact caller/fixture matrix; see [acceptance](m01-a3-transfer-acceptance.md). M-06-G.2 is accepted in bc49f1220; see [its clause matrix](m06-g2-transfer-acceptance.md). **The exact global completed total remains unestablished; at least12are proved.** The original zero lower bound is superseded, not a claim that no capabilities had been delivered.

**Comparison rule:** semantic transfer, not reproduction of old mechanism. Approved literal-percent compatibility, shared source inodes, bytes-only APIs and simpler rollback owners do not themselves keep a task partial. Only remaining useful behavior or acceptance evidence counts. No full already-main task equivalence was established.

## Concrete partial correspondence

### M-01-A.1 — Freeze address grammar and compatibility corpus

Accepted mapping: F01, F05; commits 504b42f76, df040becb.

**Implemented:** Portable lexical checks, unique scoped anchors and truthful selected/pinned patterns.

**Remaining behavior/evidence:** Verify all supported old/new address meanings and ambiguity/refusal corpus against the accepted target compatibility contract. Resource-export and unsupported-revision handling from explicit old steps have no criterion-complete accepted mapping here; absence of ResourceAddress type and literal percent behavior are not failures.

Exact old clause: “Define permanent exact scheme, authority, version, Vibe path, resource-export and unsupported-revision rules.” (explicit_task_steps).

**Superseded, not remaining:** Existing literal percent spellings deliberately remain literal; no new public parser grammar is imposed merely to match prototype. Reference: features/F01-source-paths.md Change and rationale / Proven scope; LOG.md F01 acceptance (504b42f76).

### M-01-A.2 — Unify portable component containment

Accepted mapping: F01, F03, F05; commits 504b42f76, 6dafbac72, df040becb.

**Implemented:** Existing shared declarant-path authority plus held/no-follow source traversal.

**Remaining behavior/evidence:** Exact old steps include NFC/case alias distinctions and native/portable aliases. F05 explicitly leaves portable sibling aliases open; no full old contract equivalence.

Exact old clause: “Drive/UNC/ADS/device roots, trailing dots, control bytes and NFC/case aliases cannot escape or silently alias.” (explicit_task_negative_case_and_steps).

### M-01-A.3 — Preserve failures across compiler and prompt callers — accepted full

**Root disposition:** complete at its original internal-caller boundary, with compositional test limits explicitly recorded in [M-01-A.3 acceptance](m01-a3-transfer-acceptance.md).

Accepted mapping: F03, F05; commits 6dafbac72, df040becb.

**Implemented:** Real filesystem/embed/compiler and prompt consumers tested with typed refusals.

**Accepted boundary:** the earlier missing caller/fixture matrix was resolved by the linked acceptance. Full resource-query grammar/service is not a criterion of this task and is not a reason to reopen its completed internal-caller boundary. The matrix and its compositional limits remain the evidence; there is no outstanding provisional-partial instruction for this row.

Exact old clause: “Every inventoried caller has a result/refusal fixture.” (unverified_acceptance_matrix_not_demonstrated_behavior_gap).

### M-01-C.1 — Retain file observation for digest and parsing

Accepted mapping: F03; commits 6dafbac72.

**Implemented:** Actual read_source_bytes_in holds parent/file identity, compares two passes and closing/reopened observations, returns the observed bytes; FsSectionSource read_location parses/projects exactly that buffer or exact held overlay. Real Linux read/link/replacement and pure-source tests are accepted in F03. No reread is required by the parser.

**Remaining behavior/evidence:** Specific useful old promise still unestablished independently of the rejected laws: task step says compute raw digest and parsing input from the same buffer; positive case says exact bytes and matching digest. At pinned main, the accepted source reader returns Vec<u8>; SourceLocation::read returns Vec<u8>; actual FsSectionSource::read_location feeds those bytes into projection/section parsing or UTF-8 and returns text/metadata, without computing/exposing an associated raw digest on this route. No accepted mapping demonstrates a separate real consumer receiving/verifying that same-observation digest. This is a narrowly identified observable-digest/evidence gap, NOT a requirement to recreate a proof wrapper. Root must decide whether that digest behavior remains useful required scope or is explicitly superseded; no blanket global source framework is required. If superseded, this row needs reassessment, not automatic partial status.

Exact old clause: “Compute raw digest and parsing input from the same buffer or retained stream.” (explicit_task_step_and_positive_case).

**Superseded, not remaining:** Read-only source bytes intentionally allow shared regular inodes; strict single-link state/mutation readers remain separate. Reference: LOG.md:115-119 URI-OBSERVED-SOURCE-READ decision; F03 Implemented behavior; accepted6dafbac72.

**Superseded, not remaining:** Existing source consumers need consistent bytes; root explicitly rejected a new proof/authority object. Documented observation window plus measured refusal checks is a valid replacement, not missing delivery. Reference: LOG.md:115-119 says new source consumer gets consistent bytes and needs no proof/authority object; F03 API returns bytes by design.

### M-01-C.3 — Bind builtin converter and overlay provenance

Accepted mapping: F03; commits 6dafbac72.

**Implemented:** Builtin conversion and exact overlay consume held bytes without a fallback read.

**Remaining behavior/evidence:** No complete inspectable transformation chain or separate source/raw/representation hash result delivered by this accepted slice.

Exact old clause: “Every transformation between source and returned representation is inspectable.” (explicit_acceptance).

### M-01-E.1 — Exercise native and portable filesystem adversaries

Accepted mapping: F01, F03, F05; commits 504b42f76, 6dafbac72, df040becb.

**Implemented:** Linux real filesystem adversaries, replacement/links, duplicate anchors and portable lexical controls recorded.

**Remaining behavior/evidence:** Old cases additionally include Unicode aliases and cancellation of large authorized responses; full inventory not closed. Native Windows/macOS explicitly unavailable, which is honestly recorded rather than waived.

Exact old clause: “Unique nested source resolves; large authorized response supports cancellation.” (explicit_task_cases).

### M-05-A.1 — Characterize root union and Fresh request loss

Accepted mapping: F02; commits 075d38cb2.

**Implemented:** RED/GREEN multi-consumer constraints, held roots, selected-node scope, retries and Fresh kind guards.

**Remaining behavior/evidence:** Old task also characterizes bare-install Fresh when only requested features change. F02 explicitly limited to registry/workspace constraints; feature/request identity portion has no complete mapped gate here.

Exact old clause: “Record root deduplication and bare-install Fresh behavior when only requested features change.” (explicit_task_step).

### M-05-A.2 — Characterize feature syntax and solver capabilities

Accepted mapping: F04; commits c20b6bc89, 033a4236d, 5af2eadc8, d0c961277, da741ee16, 237f7bf2b, eefe840b8, 8f84aefb9.

**Implemented:** Legacy app/package feature characterization, all-features/exclusive and differential cases.

**Remaining behavior/evidence:** No accepted full naive/SAT/resolvo capability matrix plus wrong-version and negated conditional characterization mapped to this old task.

Exact old clause: “Inventory actual supported naive/SAT/resolvo entry routes from their existing dispatch; report capability gaps explicitly.” (explicit_task_step).

### M-05-B.1 — Implement one typed feature-effect algebra

Accepted mapping: F04; commits c20b6bc89, 033a4236d, 5af2eadc8, d0c961277, da741ee16, 237f7bf2b, eefe840b8, 8f84aefb9.

**Implemented:** One common closure kernel, package/app legacy lowering, typed authored effects admission.

**Remaining behavior/evidence:** Stable dependency activation/strong-weak forwarding and actual induced effects not connected end to end; accepted cards explicitly limit foreign binding and effect consumers.

Exact old clause: “One effect set activates an optional dependency and forwards requested features.” (explicit_goal_steps_positive_case).

### M-05-B.2 — Add constrained groups and consumer resolution domains

Accepted mapping: F04; commits c20b6bc89, 033a4236d, 5af2eadc8, d0c961277, da741ee16, 237f7bf2b, eefe840b8, 8f84aefb9.

**Implemented:** Choice cardinality/guards, original/dormant/exact/additive/default inputs and real requires-edge source admission.

**Remaining behavior/evidence:** Generated requests carrying original owner edge/domain/target and candidate-bound choice provenance are not delivered across ordinary configured solver routes; app candidate excluded.

Exact old clause: “Generated requests retain owner edge/domain/target and choice provenance.” (explicit_acceptance).

### M-05-C.3 — Use normalized requests across every solve route

Accepted mapping: F02, F04; commits 075d38cb2, c20b6bc89, 033a4236d, 5af2eadc8, d0c961277, da741ee16, 237f7bf2b, eefe840b8, 8f84aefb9.

**Implemented:** All real workspace consumer constraints retained through retry; normalized input data admitted.

**Remaining behavior/evidence:** Same immutable request/catalog across every solve/metadata/conditional route and selected declaration/provider provenance on every graph edge not integrated by these slices.

Exact old clause: “Every graph edge has a selected declaration/provider provenance.” (explicit_acceptance).

### M-05-D.1 — Persist requested intent separately from resolved configuration

Current accepted app scope is expanded in the [four-row runtime review](m05-app-transfer-review.md); the following initial mapping predates c5662ee42. Generic configured lock/provider-edge replay remains open.

Accepted mapping: F04; commits c20b6bc89, 033a4236d, 5af2eadc8, d0c961277, da741ee16, 237f7bf2b, eefe840b8, 8f84aefb9.

**Implemented:** Original ConfigurationIntentDecl survives requires-edge authored round-trip and target-aware normalization.

**Remaining behavior/evidence:** Supporting input boundary only: no complete requested-vs-resolved configuration lock round-trip, capture/domain/target or induced-edge explanations accepted. Do not count this as persistence delivery.

Exact old clause: “Lock round-trip reconstructs requested versus resolved semantics distinctly.” (explicit_acceptance).

### M-05-D.2 — Connect feature and subskill activation to real consumers

Accepted mapping: F04; commits c20b6bc89, 033a4236d, 5af2eadc8, d0c961277, da741ee16, 237f7bf2b, eefe840b8, 8f84aefb9.

**Implemented:** Common local evaluation connects existing legacy app/package consumers; dormant input projection available.

**Remaining behavior/evidence:** Full selected feature/subskill contribution activation into visibility/loading/projection remains explicitly required; populated effect fields are not actual consumer proof.

Exact old clause: “End-to-end fixture observes contribution activation, not only a populated field.” (explicit_acceptance).

### M-05-E.3 — Make configuration apply and removal recoverable

Accepted mapping: F06; commits 50a4630aa.

**Implemented:** Bounded safe manifest/lock/provenance publication and truthful rollback conflicts through actual apply/hosted caller.

**Remaining behavior/evidence:** Task explicitly still requires failure coverage across manifest/lock/slot/boot with retained recovery outcomes and a removal plan explaining affected dependents before deletion. F06 proves bounded metadata rollback and real caller refusal, not removal or all these outcomes. No old journal mechanism is mandatory.

Exact old clause: “Removal plan explains affected dependents before any deletion.” (explicit_acceptance).

**Superseded, not remaining:** F06 uses incumbent transaction plus three real file owners; no broad recovery-image store required. Reference: F06 Change/design and Proven boundary, accepted50a4630aa.

### M-06-B.1 — Capture the existing boot graph and inclusion provenance

Accepted mapping: F08; commits 7ed7e7256.

**Implemented:** Real committed-boot tree retains nested/repeated occurrences and typed malformed attribution failures.

**Remaining behavior/evidence:** Exact old behavior beyond F08: source positions and exact capture/snapshot identities plus authorized graph traversal and retained-versus-live inspection. F08 handles committed nested/repeated marker spans, not that captured graph. Full interactive debugger is NOT an added criterion for this task.

Exact old clause: “Retain repeated occurrences, inclusion chains, source positions and exact capture/snapshot identities.” (explicit_task_step).

## Other scopes and unresolved rows

- M-01-B.1: Opaque prepared visible-source authority/capture service not delivered by F01/F03/F05 (private point locator expressly is not that object).
- M-01-B.2: Full source-world/membership witness service remains outside accepted source slices.
- M-01-C.2: Independent source membership/authentication proof not delivered; cards expressly exclude authentication/proof.
- M-01-C.4: No accepted warning/strict install-conversion outcome/closure-diff slice mapped in F01-F08.
- M-01-D.1: Complete classified source query result with descriptor/capture/representation/location not delivered.
- M-01-D.2: Generated source-query CLI/MCP/service contract and full parity remain open.
- M-01-E.2: Whole source-query CLI/MCP/service purity and integrity parity remains open.
- M-05-B.3: No accepted executable syntax-preserving idempotent migration; authored admission is not migration.
- M-05-C.1: Prototype transaction catalog binding remains transfer work; no accepted F04 configured solver integration.
- M-05-C.2: Candidate-conditioned configured solving/backtracking not delivered by common closure/choice kernel.
- M-05-E.1: now partial local app inspection/review/no-execution semantics; generic Fresh/cache-only discovery remains open.
- M-05-E.2: now partial accepted app original-intent maintenance; generic exact-capture replay and induced-graph update remain open.
- M-05-E.4: now partial common app CLI/PTY/machine questions; MCP/service/all-kind parity remains open.
- M-05-F.1: No accepted Redbook configuration/continuity dogfood slice in F01-F08.
- M-05-F.2: No accepted Redbook consumer migration slice in F01-F08.
- M-05-G.1: Full configured solver/backend/workspace parity not accepted by kernel unit/differential gates.
- M-05-G.2: Whole M05 activation/recovery/replay and failed-prefix coexistence exit not accepted.

The 71 still-unreconciled IDs are individually `unknown` in JSON. These are not assumed absent, not automatically already-main and not silently dropped. The bounded read does not cover every whole changes/migration/deployment/environment/machine/release contract.

F07 inline anchor/cache correction is accepted (`d6147a94d`) but is not a complete ordinary-CommonMark task. F04 secret/env improvements are accepted real behavior without automatically completing one old task each.

## Evidence and tracking debt

The denominator is inventory222 minus Zap remaining109, cross-checked against all113 historical accepted statuses. Private Zap work records were parsed without copying their raw personal data. Exact old task steps, cases and criteria for candidate rows are retained in JSON from pinned prototype contracts. F01–F08/LOG acceptance records and15 implementation commit ancestors were checked at main; existing test receipts are evidence, not fresh executions.

Root tracking debt remains: the accepted feature ledger has no complete reviewed task-ID→remaining semantic criterion→LAND matrix. This report is a bounded first crosswalk, not a campaign acceptance certificate. Approved alternative designs must be carried into that matrix as superseding decisions, never reintroduced as obligations merely to increase a legacy count.

Current JSON includes `implemented_coverage`, `remaining_behavior`, `superseded_obligations` and exact counter-clauses. For C1, direct source inspection separates a genuinely absent associated raw-digest result on the current consumer path from deliberately rejected proof/single-link machinery; usefulness/disposition of that residual digest promise remains root judgment.

Preserved versions: `/tmp/nst-old113-transfer-crosswalk-before-root-correction.{md,json}` and `/tmp/nst-old113-transfer-crosswalk-after-criterion-review.{md,json}`. Current inputs/hashes and all113 rows: `old113-transfer-crosswalk.json`.


## Root reconciliation boundary

Root verified the exact113ID set against the recovered remaining109 complement, all19pinned input/contract hashes and15implementation commit ancestry links. Intentional target alternatives (shared source inodes, literal percent spelling, avoiding an unconsumed proof wrapper) are accepted design decisions, not missing work merely because they differ from prototype types. Useful observable semantics still require their own decision/evidence. The A3 caller matrix and later G2 preservation matrix were accepted separately; those two proven tasks form the current lower bound. Source runtime flags and the1056history census are unaffected.


## M06 bounded update

Seven formerly unknown rows were reconciled in [the M06 review](m06-transfer-crosswalk-review.md): five partial and two untransferred requested scopes. Root verified37pinned source/contract hashes and5baseline symbol/blob checks. Existing init link handling, nested-item conversion, raw-copy paths and standalone VibeTree/headless presentation are actual already-main subclauses. F07/F08 add accepted opaque quotations and occurrence attribution; no full source-classification/measurement/debugger task follows from those pieces. That review reduced the then-unknown partition to74 without claiming any extra complete task or fresh runtime execution; the later G1/G2/G3 update below further refines it.

## G1/G2/G3 scoped follow-through

[The three-row review](m06-g123-transfer-review.md) maps30 exact clauses against actual source and fixture definitions. Root verified57 pinned Git-object hashes and9 byte-identical pre-campaign comparisons. All three rows become partial because useful pieces already existed in main; this review transfers no new code and closes no whole task. At that review boundary the partition was1full/22partial/19named untransferred/71unknown; later updates below supersede it.

- G1 still needs same-source section/preamble once coverage across normal contributions, preserving uncovered fragments and equal prose from different documents; ordinary simple/embed distinctions already work.
- G2 ordinary init still hardcodes Mixed instead of reading the host consumer dialect, potentially replacing XML and disturbing derived state. The next narrow atom is the existing dialect adapter plus real XML/Markdown preservation cases.
- G3 needs generated-payload exclusion in both source walks, authored static-zone projection and selected source-member publication protection. Existing multi-fragment retention and compiled-STATIC citation refusal must be preserved.

These are source-derived gaps, not fresh runtime failures. G2 is independent of the current app runtime, while these foundations must precede their consuming package/init enhancements. Native Windows qualification remains explicitly unperformed.

## Accepted runtime and G2 update

[Four M05 rows](m05-app-transfer-review.md) now reflect the accepted c5662ee42 runtime. D1 remains partial with actual app index/work original-input persistence; E1/E2/E4 move from not-transferred to partial for local inspection/review, real app maintenance and common CLI/PTY/question handling. Root verified38source files,7standing files,2old contract containers and8evidence artifacts. No generic solver/provider-edge lock replay, cache-only discovery purity or MCP/all-kind parity is inferred. No whole M05 task is closed.

[G2 is fully accepted at its original ordinary-init boundary](m06-g2-transfer-acceptance.md) in bc49f1220, with actual XML RED/GREEN, three host variants,37distinct CLI tests and the strengthened existing Tool control. Native Windows qualification stays explicitly open as its original acceptance requires. G1/G3 remain partial. At that review boundary the total was2full/24partial/16named untransferred/71unknown; the later M02 audit below supersedes that partition.


## M02 source/criterion reconciliation

[Four foundation rows](m02-foundation-audit.md) are now reconciled against97hash-bound source observations. A.1/A.2 become partial because current requirements/adoption/source-coordinate owners cover part of the characterization contract; the exact two-project fixtures and descriptor seam still need current evidence. B.1/B.2 are not transferred in the inspected current manifest/requirements surface: their actual pure change parser, token migration proposal, structural work graph and intent/contribution identities have no corresponding exposed owner. No implementation or runtime completion was added by this audit.

Current partition:2full/26partial/18not-transferred/67unknown. Full completion remains a lower bound of2. The migration subpart needs only M13 pure token replay; full apply/recovery and editor barriers are downstream, not prerequisites of the current pure parser/proposal. The architecture map retains those downstream relations separately.


## G1 accepted

[M-06-G.1](m06-g1-transfer-acceptance.md) is accepted in `2a9da5c8b2e9ba6b73d2192df1f6b6a541a7f373` at its original compiler/boot boundary. Native Windows qualification remains explicitly open; it is not a passed platform claim. Current partition:3full/25partial/18not-transferred/67unknown. This closes one old task; F18 internal policy reuse and F19 adjacent typed-provenance completion are not extra historical credits.


### 2026-11-06 — G3 exact original boundary accepted

[M-06-G.3](m06-g3-transfer-acceptance.md) accepted in `c266a5b62db5c6221c90734a3ddc4119eede3542`: generated payload/citation exclusion, authored nested static fragments, selected source-member byte preservation through install/check/repeat and durable-lock version drift. Root reviewed26committed hashes and final specification links. Native Windows stays explicitly open. Current partition:4full/24partial/18not-transferred/67unknown.


### 2026-11-06 — original neutral M13C1 accepted

[M13C1](m13-c1-transfer-acceptance.md) accepted in `f56a4cba60e502416d194f6fc3677e51a1ad2e45`. The exact task explicitly separates later SourceDescriptor/feature converters. Genuine snapshot-to-copy and current identity proposals preserve exact bytes; typed ambiguity/refusal and independent hash/replay evidence pass. This does not claim CLI, saved apply or broad historical corpus. Current partition:5full/24partial/18not-transferred/66unknown.


### 2026-11-06 — M02B1/B2 accepted

[Exact two-task matrix](m02-b12-transfer-acceptance.md) accepted in `940f93db4115c83484916849580f085d47c4b8ae`. Shared immutable change grammar/pure migration/safe saved reads and independent structural identities pass meaningful counterexamples and format gates. Corrected inherited address # bug and omitted work-format inventory; no writer/scheduler/authentication claim. Current partition:7full/24partial/16not-transferred/66unknown.


### 2026-11-06 — neutral C2 accepted, D1 scope reconciled

[C2](m13-c2-transfer-acceptance.md) accepted in `21d7683e027d81f14b2528734dd77bb7da236821` with actual CLI/MCP parity, strict input presence and recipe-consistent results without state effects. [D1 audit](m13-d1-next.md) independently reconciles42source bindings: needed migration journal/apply/recovery domain is not delivered, despite available shared primitives. It moves unknown→not-transferred, not to complete. Current partition:8full/24partial/17not-transferred/64unknown.


### 2026-11-06 — original neutral D1 accepted

[D1 acceptance](m13-d1-transfer-acceptance.md) in `a4b322150c494e5d7245252ace109b7be9e62c1a` binds real apply/inspect/complete/restore, process-cut recovery, independent grant surfaces and exact effect/limitation reporting. Original ruling explicitly separates Linux measurements from later Mac/Windows native proof. Current partition:9full/24partial/16not-transferred/64unknown. Source/feature recipes and A1 are not counted by this foundation.

## Five remaining M02 tasks — source reconciliation, 2026-11-06

Root verified all26 pinned source hashes and all five original contracts in [the clause review](m02-remaining-transfer-review.md). B3/C1/C2 are partial; D1/D2 are not transferred. This removes five unknowns without completing any task. Existing saved readers and one-file migration recovery do not provide editor snapshots, retained-origin rebasing, complete fact selection or no-overwrite change scaffolding. D2 read-only inspection can precede D1 creation; C2 consumes accepted source interpretation/capture semantics. No runtime gates were executed by this audit.

## Seven M14 tasks — environment audit, 2026-11-06

Root verified49 pinned source hashes and7 original contracts in [the environment review](m14-transfer-review.md). A2/B2/C2 are partial; A1/B1/C1/C3 are not transferred. No new full task. Existing global/project installer choices, layout, cache and ownership must be extended, not replaced by another package manager. Root substitution alone does not redirect host commands or native launchers. The original stale C1 cross-reference is interpreted against explicit C2/C3 contracts: first actual bound effect belongs C3.

## Eight M10 tasks — derivative audit, 2026-11-06

Root verified31 pinned source hashes and8 original contracts in [the derivative review](m10-transfer-review.md). D1 is partial; seven others are not transferred. No full task promoted. Existing compiler provenance and materializer records are reusable foundations, but adoption statuses are not arbitrary local patches, and preparation that hydrates skills/cache is not a passive read. Preserve one source/compiler/ownership system when adding independent base/local/result lineage.

## Five M16 foundation tasks — source audit, 2026-11-06

Root verified32 pinned hashes and5 original contracts in [the foundation review](m16-foundation-transfer-review.md). B1/B2 partial, A1/A2/B3 not-transferred; no new complete task. Existing strict migration services and source/occurrence identity owners should be reused. Source review identified a concrete MCP query input-schema discrepancy; minimal requests and exact paths are retained for runtime reproduction and repair under B1. Whole framed transport is not a prerequisite for pure domain functions, and snapshot data alone does not grant mutation authority.

## M06A1 accepted — 2026-11-06

Original A1 is full in264b07466: [clause acceptance](m06-a1-transfer-acceptance.md). Root verified122committed files and real source migration/adoption/guard/materializer evidence. Counter rises from9to10full; A1 came from not-transferred, not partial. A2/A3/A4 and the complete source feature remain open.

## M06A2 accepted — 2026-11-06

Original A2 is full in626fcbd8d: [acceptance](m06-a2-transfer-acceptance.md). Pure ordinary structure and strict declarations are joined to a real captured Mixed source reader, preserving zero facts and native inline opacity. Library resolution is delivered; new CLI/resource transport and A3/A4 are not inferred. Counts11full/32partial/31not-transferred/39unknown.

## M-06-A.3 — accepted full

Declared raw preservation, logical collision/conversion and actual selected installed boot/native/backend/replay reading are accepted in `3fcf514a5cf9a60262fe233f6480ea63f14a58f5`. See [original-clause acceptance](m06-a3-transfer-acceptance.md). A4 broader consumer integration remains open.

## M-06-A.4 — package-query slice accepted; task remains partial

Installed requirements query is accepted in `12ee51b0ebcb185ede2ac94f08ded0de7694caa4`; see [bounded acceptance](m06-a4-package-query-acceptance.md). Host declarations/cited sync and the remaining consumer matrix are still required. No old113count increment follows from this slice.

A4 host policy/cited sync accepted in `c24dfc6c64a2d900a05b5464f0bc510d4efda925`; see [bounded host acceptance](m06-a4-host-acceptance.md). Full host read/boot and remaining consumer support are still required; counts unchanged.

A4 checked host point-reader accepted in `d2c283e6970432f563a9c44129daf1ada750725a`; [bounded acceptance](m06-a4-host-read-acceptance.md). Broad host inventory/query and own boot remain; no original-task count increment.

A4 host facts/query accepted in `432761746296c7913297cb7daad1302d30fc3b4b`; [bounded acceptance](m06-a4-host-query-acceptance.md). Own boot and remaining consumers still keep the original task partial.

A4 own/inherited boot accepted in `33a680809c903a0fa0e40fbdb64640126b19296d`; [bounded acceptance](m06-a4-own-boot-acceptance.md). Remaining consumers and full matrix keep A4 partial.

A4 bound boot-check accepted in `04e7ccad8ead2002fd49d936e6917b8631e7006a`; [adapter acceptance](m06-a4-boot-check-acceptance.md). Remaining consumer matrix keeps A4 partial.

A4 facts report/adopt accepted in `728d41319b04b2e527058ed29e77e4d88209c566`; [bounded acceptance](m06-a4-facts-command-acceptance.md). Other consumers and full mixed matrix remain.

A4 tutorial correction accepted `839fdd0c3`: EN/RU --link and Simple/Normal defaults plus root layout comment; exact init1PASS, no config/default change, no old113count increment.

A4 captured documentation point reads accepted `7859d3e2d`; actual citations/agent/CLI support, full census and generated-render/baseline repair remain. No old113count increment.

A4 generated documentation repair accepted `8e61e9c75`; all62prior library failures closed, full634/0. Full declared page census remains. Old113counts unchanged.

A4 Doc census/build/server/self/borrowed accepted `3cafec8b7`. Actual Doc CLI uses cache/store. Non-Doc/host level0 projection and other A4 consumers remain; counts unchanged.
