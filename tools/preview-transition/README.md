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
Bindings explicitly report unsupported authoritative native role/identity and
domain-join adapters.
Each stage retains its full due predicate and keeps predecessor proof separate
from later obligations: reviewed guidance does not prove rehearsal, and candidate
eligibility does not establish later narrow final attestation. Producer flags,
checked/updated/reviewed/verified labels, equal declared hashes or an unrelated
successful claim cannot turn these prerequisite reports into a stage pass.
Actual owner adapters remain with the later domain/documentation/release work;
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
