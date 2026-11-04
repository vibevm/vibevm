# F04 — Common configuration from the working application wizard

Status: active. The common user journey is not accepted merely because a pure kernel is extracted. Canonical contract: PROP-069-common-configuration.xml.

Architecture-ordered foundation update: F09 adds the independently checked core semantic fingerprint/direct-input delta and its versioned recipe. It does not activate new app/wire/state consumers or close F04. The whole222architecture map now governs subsequent integration order, including standalone proxy compatibility before index4 writer activation.

## Why this feature is being generalized

The owner identified the existing app console/GUI choice system as a stronger starting point than the prototype's manual choice-host route. Main has describe → choose → plan → install → record → modify/update consumers. Next contributes important semantics: cardinalities, positive guards, original omitted/empty/additive/exact intent, multiple consumers and dormant choices. Neither a set of booleans nor a parallel host service completes the combined requirement.

The target is one kind-neutral local configuration capability, adapters to existing effects and state owners, and the same answers through console and serialized external input. Read-only doc and policy-consent remap boundaries remain distinct. All nine package kinds must have an honest supported-operation matrix before final acceptance.

## First slice and explicit limits

The first slice extracts local feature closure into vibe-core::configuration and returns application behavior through a compatibility adapter. It performs no I/O, registry operation, installation or persistence. Rich consumer choices, parameters, effect integration, retained intent and common machine surfaces are following work. The existing app wire documents are not changed by this extraction.

A closure constraint is not a consumer-owned choice. The former constrains the computed feature set; the latter requires and retains an answer and may not be selected accidentally by ordinary implication. This distinction belongs to the normalized rule, not the package kind. Closure-group activity derives from its members' parents; arbitrary positive choice guards are deliberately excluded from this first primitive.

## Problems found while characterizing main

1. Ordinary requires closure and group fallback interleaved. Reordering root/relay/chosen declarations could change success to conflict; renaming groups could activate a later fallback prematurely. Fix: close ordinary implications, collect eligible fallbacks from one snapshot, apply as a batch, repeat. Conflicting defaults refuse without searching for a different answer.
2. Profile/prompt selections were classified as direct additions. A valid gtk profile plus --feature qt conflicted; --no-feature gui conflicted with a profile child instead of removing it. PROP-065 already says direct flags apply on top. Normalize profile/prompt as the base, flags as additions/disables; direct contradictions remain errors.
3. Group replacement could silently remove a directly enabled descendant of the replaced branch and report success. The requested fix is a named conflict, while base-only descendants remain overridable. The standalone repair is integrated in main and its regression passes.

The first two standalone patches are separate from kernel extraction. This separates actual behavior repairs from the claim of a compatible refactor. A malformed first profile fixture mixed installation sugar and steps; correcting the fixture is not behavioral RED evidence.

## Verification so far

Original evaluator against intended regressions: 3 passed, 5 failed, exit 101. Repaired standalone evaluator in main: 8 passed, 0 failed. Public app wizard after the repairs: 1 passed, 0 failed (describe/plan/install/events/record/cancel/resume). Scoped app conform: 0 new findings; one pre-existing ambient-env observation is nonblocking under current policy. The existing public wizard passed before extraction. Shared-kernel and maintenance evidence will be appended after review.

## Remaining acceptance

Shared kernel review and integration; rich intent/guard/default/consumer semantics; typed parameters and secret handling; actual feature-effect and solver integration; authored/install-owner persistence; ordinary console and versioned external views; complete install/update/modify and all-kind scenarios. Prototype receipts and pure tests are evidence for bounded claims, not proof of these outcomes.

## Kernel candidate review

The shared kernel compiles local references into a private validated model. It supports all four closure cardinalities, parent/required/requires implications, structured refusals and declaration-ordered selection/reasons. Positive requires cycles terminate; parent cycles and invalid references refuse. Group activity derives from member parents. Overlapping constraint groups are independent; rich choice member ownership is a separate rule. App normalization, public types and error wording remain in the app adapter. The old production Tree closure is removed; the exact old implementation exists only as a test oracle.

Root review corrected two unnecessary restrictions in the first candidate: arbitrary activation guards were removed from closure constraints, and the single-group membership ban was removed after comparison with main package exclusive sets. The app adapter also deduplicates redundant requires entries in first-occurrence order, preserving validated legacy input and implication reasons. These are candidate review corrections, not claims that all old declarations were broken.

Final worker checks: 13 core unit tests, 14 app feature tests (including 18 named unaffected differential rows), 1 planner precedence test, 15 core doctests, 1 app doctest, and one each public wizard, modify and update/rollback test: 47 distinct test/doc functions. Scoped core/app clippy, formatting and diff checks returned 0. Main integration and scoped traceability/conform checks remain pending at this entry.

The revised rich-choice semantics are frozen separately in PROP-069#CHOICE-CORE: positive group/member guards; independent omission and merge strength; dormant projection; no executable partial result; final composition through the existing closure kernel with an exact check that no unchosen choice member appeared and no chosen member disappeared. Candidate/default binding and persistence still need their real owners. Research found no downstream main consumers for several existing FeatureExpansion fields; their pure algebra is not end-to-end delivery. See research/rich-choice-design-input.md for exact paths and proposed distinguishing cases.

Integrated main full application library: 107 passed, 0 failed, including feature/planner cases and existing child/cancellation/state tests. Candidate and main source bytes match all 12 recorded SHA256 values. This expands app consumer coverage; it does not claim native-platform execution or complete package configuration.

Kernel accepted after main integration: full app library 107/107, scoped core/app conform 0 new findings, fmt and specmap --check exit 0. Traceability has 0 suspects, 0 gated orphans and 0 unresolved host edges; 43 pre-existing warnings remain. All 12 candidate source hashes match integrated bytes. Only PROP-069#FEATURE-KERNEL moves to impl/done; F04 and the whole common configuration remain active.

## Rich-choice candidate, not yet accepted

The next candidate owns a validated ChoiceModel over FeatureModel, positive guard DAG, full original consumer answers, active/dormant projections, default-use facts and explainable controls. Only ChoiceOutcome::Resolved contains final features. Omitted exact without an applicable default is unanswered, not an exact-empty bound. Choice members are added as base seeds, and the one feature closure must neither activate an unchosen choice member nor lose a chosen one.

Review found ordinary invalid input hidden behind pending questions and overflow diagnostics blaming unanswered consumers. Both were reproduced (2 failing cases) and corrected through shared input preparation and contributor filtering. Current configuration tests: 22 choice + 13 kernel, all pass. App consumer/docs/clippy and final main gates remain pending; candidate/default binding and persistence are outside this component.

Main rich-choice integration tests passed 35/35. The first scoped conform check failed on a test helper because its nested out-of-line test module lacked a directly visible cfg(test) marker. Root added explicit markers to both nested test module declarations, preserving compile/runtime meaning; exact validation family 4/4 and conform with 0 findings passed. Original worker hashes remain archived; the one-file root amendment has a separate accepted hash manifest. No baseline suppression or production unwrap was introduced.

Rich-choice component accepted: 87 distinct candidate tests/docs; integrated main configuration 35/35; after explicit nested test markers, validation 4/4 and conform 0 findings; fmt/specmap --check exit 0. Traceability remains 0 suspects/orphans/unresolved host edges, with 43 known warnings. The accepted 12-file hashes include the one test-context amendment. Only CHOICE-CORE becomes impl/done. Cross-process intent/default receipts, effects and public all-kind user journeys are still required before F04 can close.

## Existing package consumer integration

The package FeaturesTable now lowers into the same FeatureModel as app, with a small parser/policy/effect-projection adapter. Default is a real named feature; explicit/default seeds union; all-features starts from public names and retains its precedence over explicit input. Local implications may reach private features. Exclusive groups are independent at-most-one sets, including overlap; duplicate members are deduplicated and empty sets are no-ops. No app replacement priority is imposed on package inputs.

Five old flaws were reproduced as failing correction cases: the all-alternatives probe falsely rejected valid definitions; repeated exclusive members falsely conflicted; private dormant unknown references escaped validation; unknown exclusive members were ignored; and unselected invalid branches escaped expansion. Full local definition admission now catches them. Foreign dependency and export binding remain outside a table-only API. The shared lexical admission was relaxed to preserve demonstrated literal-space IDs without normalization; app's authored portable-token rule stays intact.

Candidate verification: resolver 28, shared configuration 38, docs 37, app feature consumer 14, checker 2, public CLI installation 1 — 120 distinct tests/docs. The 21-row differential matrix is included, not counted again. Clippy/fmt/diff passed. Root verified exact 12-file hashes and integrated them; main checks are in progress. The old evaluator remains test-only. Checker prose is corrected to match actual local admission and avoid claiming export existence from a selector.

Package adapter accepted after main resolver 28/28 and configuration 38/38, fmt and scoped conform (0 new), specmap --check (0 blocking findings, 43 known warnings). All 12 source hashes match. Existing package CLI install remains the tested consumer; richer authored choices, dependency effects, selected loading and durable original intent still require connection.

## Classified secret diagnostics

Six reviewed files correct existing secret-value error paths and Debug, and choose hidden input before public boolean/choice widgets. Invalid booleans, integers, choices, canonical range values, resolved paths and opaque prompt callback errors no longer disclose marked answers. Public diagnostics retain useful details; typed canonicalization, explicit/recorded/prompt/default precedence and existing recipe/shell input delivery remain. Authored default/choice definitions are public data.

The complete real CLI RED has 5 failures and 2 passing controls; corrected human+JSON output passes all 7, retaining exit 2 and parameter_invalid. Core/app RED is retained as observed excerpts, not fabricated complete logs. Final candidate coverage is 23 distinct tests/docs (6 core, 7 params, 7 CLI, 1 existing wizard, 2 docs), plus scoped clippy/fmt/diff. Password branch selection is source-reviewed and compiled; no real PTY interaction is claimed. Root integrated all six SHA256-verified files and is running public CLI tests in main.

This accepts only classified errors/Debug/prompt handling. Template interpolation/persistence, raw argv in hosted replay, batch/root/current scopes, optional-secret resupply and other continuation gaps remain required work. There is no assertion of arbitrary child sandboxing or redaction of malformed CLI input before its declaration is known.

Classified secret diagnostics accepted after main public CLI 7/7 and fmt, scoped conform (0 new) and specmap --check (0 blocking findings; 43 known warnings). All six integrated hashes match the reviewed candidate. SECRET-DIAGNOSTICS is implemented; the broader SECRET-BOUNDARY and complete F04 remain unfinished.

## Ordinary authored definitions

A single optional ConfigurationDecl now passes through ordinary ManifestWire/read/parse/validate/serialize at package scope and selected installer scope. It retains ordered features/groups/parameters/profiles and typed effect data, uses existing FeatureModel/ChoiceModel and parameter validation, rejects mixed same-scope legacy declarations and validates real requires targets. All nine kinds admit data definitions; installer/doc/remap scopes cannot use package dependency/forward/subskill effects.

Root review corrected missing default-profile metadata, an invented shorthand restriction, duplicate ID/intent vocabulary, wrong profile preset FieldPath, misleading empty legacy describe and lost global resolution error classification. Three behavior defects were reproduced separately from compiler/fixture/style mistakes. The actual public -g and describe/steps routes now refuse unsupported common definitions explicitly; valid legacy wizard documents still pass. This is source admission with honest guard boundaries, not completed runtime configuration. Exact field forms, before/after examples and command/version compatibility live in FORMAT-CHANGES.md (FMT-004).

Candidate coverage: 597 distinct successful test/doc names after deduplicating filters; root integrated all 31 source hashes. Main authored family11/11 and public CLI4/4 passed. Integrated conform/specmap remains in progress. Profiles/default receipts, parameter answer merging, actual solver/loading effects, persistence and complete common surfaces remain required.

Authored admission accepted after reader11/11 + public CLI4/4 in main, reviewed structural extraction (enum/algorithm bodies unchanged; 15 existing app cases), fmt and four scoped conform gates with 0 new findings. Final 33 source hashes match. Specmap:9051units/4724items/3964edges,0blocking findings,43known warnings. AUTHORED-DEFINITIONS/SCOPES are implemented at their narrowed admission boundary; PROFILE-INPUTS explicitly remains unimplemented runtime normalization. The full common configuration remains active.

## Original requires-edge intent

Ordinary requires inline rows retain ConfigurationIntentDecl across registry/git/path/version.var and actual Workspace finalization. Checked serialization refuses dangling/ambiguous metadata. Target-aware normalization applies an explicitly named profile, overlays exact/additive omitted/empty answers and typed public parameters, returns existing kernel inputs, and preserves original source. Explicit common public/internal visibility governs direct additions without changing opaque-ID closure or legacy underscore policy. Foreign forwarding visibility remains pending.

Root reviewed 22 amended source hashes and the source/normalization/guard paths. Candidate529distinct tests/docs include real workspace separation and two diagnostic RED→GREEN corrections; main source6/6, normalization6/6, prepared9/9+F02constraints22/22 pass. Four conform scopes have0new findings, fmt/specmap --check exit0,0blocking traceability findings and43known warnings. AUTHORED-INTENT/INPUT-NORMALIZATION are accepted at their exact input boundary. No default receipts, scalar merge, configured solver or final persistence/UI support is inferred from this result. Exact formats and -g impact: FORMAT-CHANGES FMT-006.

## Secret templates and prompt continuity

The actual app planner now refuses secret references in framework template fields, prepares only running agent prompt files through the existing SafeFS source observer, and retains exact checked literal text for preview/execution. Broad raw/empty fallback is removed; only legitimate finish root/bin placeholders remain. Recipe/shell ephemeral environment and direct parameter conditions remain, agent/native transport is unchanged. Shared hardlinks and caller-root aliases are tested; descendant symlinks refuse.

Real RED proves earlier work/outbox publication with secret prompt expansion and actual prompt replacement after planning. Candidate75distinct tests/docs passed, including10existing agent and1wizard scenarios. Root reviewed all14source hashes and the only Cargo.lock change (app→existing safefs), then main joint publicCLI4+2 and fmt/conform/specmap passed. SECRET-TEMPLATES is accepted; raw argv replay and broader continuation remain open. Exact validation/migration/-g/Rust field details are FMT-007.

## Application environment identity accepted

The existing app transport collapsed distinct IDs a-b/a.b despite correct plan/index data. One core application projection owner now checks portable, injective names for every declared feature/parameter in options with expanded recipe/shell consumers; runtime protects programmatic plans before work publication. Literal IDs elsewhere and all noncolliding exported names remain unchanged. This is a main-app consumer defect exposed while generalizing inputs, not evidence that the shared evaluator lost data.

Candidate89distinct tests/docs plus clippy/fmt/diff passed. Root reviewed all13source hashes, production consumers and tests; main publicCLI2/2+app2/2+core16/16, fmt, scoped conform0new and specmap --check0blocking/43known warnings passed. APP-ENV-IDENTITY is implemented. FORMAT-CHANGES FMT-008 states the exact -g/migration/API boundary. F04 remains active; full selected-installer evaluation/persistence/UI and the broader package consumers are not established by this fix.
