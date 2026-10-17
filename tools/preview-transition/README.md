# Preview transition checker

`python -B tools/preview-transition/check.py --transition
docs/releases/developer-preview-1-to-2/transition.json --mode capture --format json`
checks the registered transition and capture shapes, their semantic invariants,
the retained raw capture hash, and every recorded Git tree, package manifest,
and source-file identity. It reads the selected repository and writes only a
machine report to stdout.

The Python entrypoint only parses CLI arguments and sends a versioned request
to `preview-transition-check`, an `xtask` binary that uses generated JTD
readers from `vibe-wire`. Exit `0` means the selected stage passed; `1` means
a failed request or claim; `2` means the selected stage remains pending. A passing capture
still reports later documentation and final-product proof as pending.

Contribution mode selects an explicit `contribution_scope`: verified before/after
captures and sorted SHA-256 bindings for exactly the selected change records.
It reads records under the layout's specification root, checks retained record
and explicitly required evidence bytes through no-follow capabilities, and reconciles every
local commit and first-parent changed path in the selected range. Captured
source exclusions and exact selected-record-only maintenance commits are the
only exclusions. Matching local implementation subjects and sorted affected
paths must cover actual product paths; invented coverage refuses. Intermediate
captures and alternate supplied locations are verified on their exact bytes.
One immutable record may retain historical implementation commits inside its
own capture bounds. Their actual raw paths remain valid declared-path membership,
while only current-range commits participate in current product coverage.
Each record binding selects sorted nonempty `required_claim_ids`; absent selection
remains pending. Required claims resolve uniquely and bind current implementation
or selected-scope capture subjects. Historical attempts and future-platform
evidence stay recorded and are not automatically required or admitted as current
proof. Required failed, unavailable or unverified results keep their distinctions.
Missing prerequisites and unavailable evidence remain pending; external/content
subject authority remains unsupported and cannot hide local coverage failures.
Candidate records can establish this bounded proof without becoming accepted.
The report keeps central semantic review and complete interface/topic/native
linkage pending; byte hashes do not authenticate an evidence producer or infer
net delivered impact from commits or reverts.

An optional exact `affected_source_review` selects a retained finite review and
its independent source bytes. The checker reuses that same contribution traversal
to reconcile every touched product path, including transient paths absent at both
endpoints, then checks endpoint identities or explicit absence at the captured
commits. A present nonregular entry cannot prove absence or regular-file bytes.
Captured manifest authorities, scoped canonical document candidates and permanent
XML anchors use their existing owners; mutable worktree edits do not replace Git
blobs. Known collisions and local byte contradictions refuse even alongside
unsupported profiles, authorities or unrepresentable paths. Unknown paths prevent
claims of complete inventory or missing canonical documents.

Selected current effects join the reviewed finite interface, path, capability,
audience, platform, anchor and topic obligations. Comparable capture subjects also
check declared kind/before/after consistency; historical or intermediate subjects
retain explicit pending semantic comparison instead of becoming global net impact.
Independent no-impact path reasons do not force a mixed record's whole disposition.
Missing optional review preserves the bounded byte/delta result with a specific
pending finding. BASE.3 central review separately requires these finite subclaims
and actual semantic adequacy; declared rows do not prove a complete native,
capability, topic or documentation census, authenticate a reviewer, or grant stage
acceptance.

The temporary campaign `bridge-check` command validates only routing references
against supplied capture and record IDs, which remain unverified declarations.
It does not run this permanent checker or certify its results. Before central
acceptance, run the separate capture and contribution modes against the actual
retained subjects and required proof, then review semantic adequacy and independent
affected capability/topic/contract membership, including every new registration.
Neither bridge success nor byte/delta coverage grants central acceptance or the
later complete canonical native/capability/migration proof.

The request grammar also names `bindings` (`roles` or `linked`),
`documentation` (`intake`, `updated`, or `verified`), `release` (`reviewed` or
`verified`), and `retired`. The later stages emit deterministic prerequisite
diagnostics. They distinguish absent capture/map/base/guide selections from
selected declarations that this stage has not verified. Bindings and release
predecessors also distinguish missing permanent record/contribution-scope selection
from supplied but unverified bindings; an empty list or absent scope is not proof.
The declaration-only readiness branch reports unavailable owner adapters;
the bindings entrypoint runs native roles and the selected domain joins described below.
Each stage retains its full due predicate and keeps predecessor proof separate
from later obligations: reviewed guidance does not prove rehearsal, and candidate
eligibility does not establish later narrow final attestation. Producer flags,
checked/updated/reviewed/verified labels, equal declared hashes or an unrelated
successful claim cannot turn these prerequisite reports into a stage pass.
Unconnected domain/documentation/release predicates remain explicit gaps;
the checker adds no native parser, hash engine or generic evidence trust authority.

Documentation `--mode documentation --stage intake` checks explicitly retained
capture/raw/map bytes, an immutable local Git source tree and complete file census,
regular build-input bytes, and the exact `urn:vibevm:doc:xml` profile through the
existing documentation XML reader and named-anchor fold. Source pages and reasoned
exclusions use source-root-relative paths; build inputs use repository-relative
paths. Mutable worktree pages do not replace captured blobs. Symlinks, gitlinks,
foreign profiles and content recipes without an existing owner remain unsupported.

Repeat `--repository-root KEY=PATH` for explicit retrieval bindings, for example
`--repository-root documentation=/absolute/owned/repository`. The thin transport
preserves the text after the first `=` and refuses missing separators or duplicate
keys. Rust requires absolute roots and rejects the reserved `product` override;
missing keys have no directory, environment or parent-repository fallback.
The declaration of repository identity does not authenticate a repository.

Intake remains pending even with verified bytes, authored checked flags or no
topics: an independent complete product/topic denominator and typed documentation
anchor bindings are still missing. Product claim anchors are not interpreted as
documentation anchors. Declared page membership does not establish editorial,
three-way update, build, link, example or runtime proof. Missing product references
remain pending while DP2 is pending and fail once its catalogue is final.

`python3 -B tools/preview-transition/assemble.py --transition
docs/releases/developer-preview-1-to-2/transition.json --output-dir <owned-path>`
creates `MIGRATION-DRAFT.md` and `WORKLIST-DRAFT.md`. Python transports paths
to the `xtask` Cargo binary `preview-transition-assemble`; generated Rust
readers and the existing semantic validators own transition and release-impact parsing.
The enclosing `vibe.toml` identifies the project, and the project layout API
resolves its specification root. Sorted immutable IDs select
`changes/<id>/release-impact.json` beneath that root.

Assembly validates all selected records before creating outputs. It rejects
unknown epochs, duplicate or mismatched IDs, dangling relations and capture
mismatches. Manifest, transition and record inputs use no-follow capabilities
and stable bounded snapshots at their observed lengths; symlinks and shared
hard-linked files refuse. Shape diagnostics do not echo arbitrary source values.
DP1, present DP2 and the optional sorted capture catalogue form one lookup.
Contribution subjects may name intermediate captures instead of the global
preview endpoints. A repeated ID must retain the same digest/commit/tree;
alternate safe retrieval locations are retained and remain unverified by
assembly. Pending DP2 allows unknown before/after subjects only as visible
`UNRESOLVED` warnings. Once DP2 is captured, unknown record capture IDs refuse
before any output publication. Additional supplied references and exact
identities appear in both draft headers, explicitly unverified by assembly.
Drafts retain scoped effects, reasons, acceptance and evidence as recorded;
assembly never verifies these claims or infers net delivery from relations.
Migration recipes, documentation mappings and independent net reconciliation
remain explicit review obligations. Prose is escaped so embedded Markdown,
HTML, line breaks and display controls cannot introduce false document structure.

The owned output directory may be created when absent. Existing output files,
directories and symlinks are refused. `vibe-safefs` pins directory capabilities
through a no-follow walk and publishes whole staged files without replacing
an occupant. A failed second publication rolls back earlier files only while
their identities still match the proofs returned by publication. A replaced
file survives; a possibly-published file without a proof stays as named
residue. Refusal reports retained residue instead of deleting unproved data.
Successful assembly exits `0`; refused input or output exits `1`. Retrying
after completed rollback produces the same bytes. No input source is modified.

Both binaries are tooling consumers in `xtask`. Generated wire records and
pure record invariants remain in `vibe-wire`; inspection owners never depend
on preview tooling. Python entrypoints only transport arguments and results.

Run the focused tests with `cargo test -p xtask --bin preview-transition-check`,
`cargo test -p xtask --bin
preview-transition-assemble` and `python3 -B -m unittest discover -s
tools/preview-transition/tests -p 'test_*.py'`. The Python integration suite
builds its binary once, or reuses `PREVIEW_TRANSITION_ASSEMBLER_BIN` when supplied.

Bindings `linked` now consumes the optional registered `domain_bindings`
supplement in `transition`. Historical release-impact bytes are retained.
The supplement names exact contribution capture IDs, two retained capability
inventory source/SHA256 references (relative to the transition directory), and
per-effect surface joins. An effect is identified by change, interface,
capability, audience and platform. Each join names its canonical surface,
documentation topic, reason, action (`none`, `manual`, `automated`, `rebuild`,
`unsupported`), data class (`authored`, `derived`, `none`), captured break-note
references and optional captured machine-operation descriptor. Captured
references use capture ID, repository-relative path and SHA256, never a journal
or copied operation event stream. Unknown fields refuse through generated I/O.

The checker compares all surfaces in the selected before/after inventories,
requires an impact for every added, changed or removed row, and joins every
selected release effect back to a surface. Changed surfaces require an explicit
documentation obligation even without a break note. Every linked surface's
source rows are checked against regular immutable blobs at its selected endpoint;
unrelated historical source rows are not remeasured. A mixed-age inventory that
cannot supply matching relevant source bytes is stale, rather than implicitly
rebound to current sources. Select a separately retained, reviewed inventory
observation for that endpoint; do not rewrite historical proof or infer global
freshness from the inventory's observation label.

Descriptors use the existing registered machine descriptor reader and the existing
`vibe-orchestrator::machine_operation::validate_descriptor_metadata` owner.
The pure validator checks semantic metadata and recognized route identities;
it grants no invocation admission. Their exact
captured bytes, operation, capability, source and registered format IDs must join
the same canonical surface. This proves reference consistency only. Source/schema
metadata never counts as operational coverage. Automatic authored conversion
remains `unsupported` when the supplement selects no exact native migration
request/report, source/target profiles, recipe or application prerequisites; a planner descriptor, prose recipe or
producer `validated` label cannot establish a converter. Derived outputs require
rebuild classification rather than authored conversion. Manual, no-action and
unsupported conclusions retain their reasons. Final migration recipes, runtime
rehearsal and independently supplied documentation remain later obligations.

The original four-record bootstrap transition intentionally has no invented
canonical inventory selection: `linked` reports
`bindings-domain-selection-missing` and exits 2. The bounded
`domain_bindings_tests` Rust family exercises the format/descriptor/capability
join, additions without break notes, all five action classes, stale and dangling
links, missing new surfaces, schema-only exposure and journal/prose refusal.
Those fixtures are finite reference-consistency checks, not production migration
or callable-coverage evidence.

The capability `Surface` has no capability/interface/audience/platform fields.
Those keys belong to the explicitly selected supplemental mapping, checked
against the release effect; inventory membership does not authenticate that
mapping or a production registration. Break-note digests establish exact bytes,
not a semantic recipe or migration source/target relationship.

The owner-correction cases call `MachineOperationService::pure_text_planner`
with explicit controlled test admission, invoke its existing text-only planner,
and validate the actual response with `validate_report_for_request`. They
exercise a real snapshot-to-copy text plan, exact-request and recipe tampering
refusal, identity/no-op, unknown-recipe refusal and unavailable admission. The
actual response is retained as selected evidence on a controlled native saved
release-impact role and joined through the same descriptor metadata owner.
The supplemental inventory and capture mapping in this isolated test are
controlled declarations, not independently authenticated production records.
Manual and derived-rebuild classifications remain reasoned release conclusions,
not invented planner outcomes. No workspace migration is applied and no new
converter or journal is introduced. Automatic application in the controlled descriptor-only fixture remains unsupported
because planning alone supplies no application command or prerequisites.
The four historical bootstrap records and their proof selection are unchanged.


The production linked path also consumes `native_planning` references. Each
selection names `rust:vibe-workspace:migration_planning`, the registered
`migration-plan-request/e1` and `migration-plan-report/e1` formats, the after
capture, and two relative source/SHA256 references. Reads retain no-follow
snapshots with a 4 MiB ceiling per file. The existing presence-aware decoder,
`validate_report_for_request`, and `MachineOperationService` replay own the
request grammar, source text, profiles, recipe, identities and result. The
checker reports `planned`, `no-op`, `needs-input` and `refused` separately.
These are current pure text results on explicitly selected controlled inputs,
not a workspace migration or historical execution attestation.

`bind2-envelope-transition.json` is the Root-selected finite linked subject:

```sh
python3 -B tools/preview-transition/check.py --transition docs/releases/developer-preview-1-to-2/bind2-envelope-transition.json --mode bindings --stage linked --format json
```

It retains the wider machine-operation contribution accounting and narrows
`domain_bindings.selected_changes` to `pure-machine-service-and-envelope-tool`.
Both retained effects are due: `mcp:plan_manifest_migration_envelope` /
`pure-machine-planner-transport`, and `rust:vibe-orchestrator:machine_operation` /
`pure-machine-planner-service`. The incumbent `MachineCoverageRegistry` reviewed
adapter binds `operation.mcp.plan_manifest_migration_envelope` and its real
surface to `rust:vibe-workspace:migration_planning`, source
`pure-migration-transport-adapter`, capability `strict-text-only-migration-planning`.
The service relationship explicitly accounts for the underlying service using
that same owner, without inventing a second coverage surface. These permanent
identity axes stay distinct. Missing native selection is pending; contradictory
owner, format, capture, mapping, request or report refuses.

The two retained inventories are finite source-bound projections of the existing
canonical row identities at `machine-operation-baseline-77f6767f3` and
`machine-operation-b1-after-2292501f3`. They are newly selected projections,
not fresh observations of the historical mixed-age whole inventory. Every
projected source hash is independently checked against its exact immutable
endpoint blob. The existing direct MCP planner row and manifest plan model are
supporting context only; the direct route is not promoted to the envelope
adapter's Bound result. The other seven changes in the wider contribution
remain covered by existing native/contribution checks and are outside this
explicit domain-link selection. No whole-product capability census is claimed.

The retained native request and golden report come from the existing migration
owner corpus (`legacy-selection.json` and `planned-golden.json`). Production
exact replay verifies that the selected report equals the actual API result;
it is not a hand-authored DTO substitute for behavior. The release effects keep
`migration_action: none`: pure planning is checked while automatic file
application is explicitly unsupported. No unrelated format-break note is
attached. The controlled break/descriptor fixture continues to cover the
separate original break linkage case. All historical impact bytes, four-record
bootstrap transition and proof flags remain unchanged. Documentation topic
obligations are checked; documentation verification, final guidance, rehearsal,
release closure and platform qualification remain later work.
