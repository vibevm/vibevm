# Boot/source transfer candidates and corrected publication finding

Read-only follow-on to `/tmp/nst-build-report.md`. Compared next `7fe6376ccef7f321302a8f18610fcb0c9cc2686f` against main `6d2f6e59be3bf55ac85fb2db67ad4e80b6925d79`; consulted `/tmp/nst-zap-plan-report.md`. No builds, tests, external operations or product/spec edits. The recovered Zap plan is an accepted research input, not a runtime proof or a mandate to copy its former task ordering.

## Direct answers

**Ordinary Markdown can be preserved and inspected without global classifier migration.** A pure CommonMark parser selected explicitly by a read-only caller neither needs `SourceDescriptor` nor changes the meaning of existing spec files. Existing raw skill/resource projection already has a byte-copy use case. However, a promise that an ordinary `.md` or `.xml` remains inert across install, facts, resolve, boot, show and conversion cannot be delivered by changing one converter alone. That broader feature needs a consistent opt-in classification boundary across those consumers, or a clearly narrower explicit resource API whose input never enters them.

**Boot explain can read current generated artifacts without the full source-query/machine framework.** Main already decompiles STATIC markers and INDEX entries for `vibe tree`. The useful first feature is a better explanation of those captured artifacts, not a compiler replay or authenticated source resolver. It can show ordered contributions, nested embed occurrences, emitted lines, exact artifact byte measurements and declared dynamic conditions. It must label absent historical provenance and source mapping as unknown; it cannot prove what a previous model actually read.

## Recommended independent slice 1: correct nested boot attribution in the existing tree

User scenario: an installed project's static boot contains A embedding B embedding C. `vibe tree --json` and the existing interactive tree should retain both B and C occurrences, including repeated same-address occurrences, and let a user understand their containment.

Main owner: `crates/vibe-cli/src/commands/tree/artifacts.rs::decompile_static/scan_embeds`; models `StaticContribution`, `EmbedSpan` in `tree/model.rs`. The current scanner keeps `open: Option<(String, usize)>`; an inner opening overwrites the outer opening. The matching end can also substitute an arbitrary closing address. This is a specific source-level limitation; no runtime reproduction was run here.

Next implementation evidence: `vibe-workspace/src/boot_debugger/static_summary.rs::scan_embeds` uses a stack and matches the top address; `boot_debugger/mod.rs::from_static_summary/why_included` retains occurrence identity and derives containment from strictly nested emitted spans. `vibe-cli/src/commands/tree/m_06_b_1_cases.rs::repeated_shared_sources_and_exact_nested_why_chain` and workspace graph tests specify the intended regression.

Transfer boundary: adapt the nested-marker algorithm into the existing main parser and test through its current consumer. No need to extract the entire parser into workspace, import the graph service or add an operation registry. Preserve one-based inclusive emitted line semantics and distinguish repeated occurrences from one logical address. Choose explicit malformed/unclosed/crossed-marker behavior; next's unmatched-stack omission is not a proof of perfect diagnostics and should not be copied unexamined.

Compatibility: retain `vibe tree --json` v1 shape. Existing embeds array can contain the corrected additional spans. A later explicit explain surface can expose parent occurrence IDs; do not silently add keys to the closed v1 boot schema. Main already has TUI tree navigation, so a second complete terminal application is not a prerequisite.

Missing user path: main currently shows package trees and embed spans, not an addressable per-occurrence “why included” query. Correct nested spans is independently useful now; a command/menu for selecting an occurrence is additional delivery, not achieved by adding a pure `why_included` helper.

Acceptance scenario to bind later: nested A/B/C, same-address siblings and nested repeats, mismatched close, unclosed open, XML c1 provenance comments, CRLF, canonical artifacts unchanged. These are proposed checks, not executed results.

## Recommended independent slice 2: artifact-backed boot explain and exact measurements

User scenario: “What contributes to the boot committed in this checkout, where is this text emitted, and which dynamic reads are only declarations?” It should work offline, before source-query preparation, with missing installed source files, and without rewriting the boot lane.

Main equivalents already exist:

- `tree/build.rs::build_tree` reads manifest/lock, resolves STATIC.md/XML and reads INDEX.md; it constructs `PackageTree.boot` and package load origins.
- `tree/artifacts.rs::decompile_static/read_index` parses existing marker and TOML contracts.
- `tree/model.rs` carries ordered contributions, embed spans, raw conditions and the static pointer.
- `vibe-workspace::boot_artifacts::resolve_static_path` owns exactly-one STATIC selection.
- Existing TUI/plain/JSON adapters provide user surfaces.

Useful next ingredients, separable from its machine architecture:

- `boot_debugger/static_summary.rs::body_ranges` measures original byte slices instead of reconstructed line lengths.
- `boot_debugger/dynamic_index.rs` preserves raw entry `kind`, order, duplicates and conditions, keeping the static pointer outside entry occurrences.
- `boot_debugger/measurement.rs::measure_reader` gives SHA-256, exact bytes and UTF-8 scalar count with chunk-boundary handling; invalid UTF-8 retains bytes/hash and marks scalars unavailable. No tokenizer is falsely inferred.
- `boot_debugger/mod.rs::why_included` and occurrence/source separation provide a useful presentation model.

Important main defects/limits to handle within this slice: `decompile_static` currently computes body bytes as each `text.lines()` line length plus one. This is not verbatim byte count for CRLF or a final line without newline. `tree/build.rs::read_opt` collapses every read error into absence. `artifacts.rs::read_index` maps every unknown kind string to Static. A trustworthy explain result must preserve unreadable/invalid/absent separately, retain unknown kinds as unknown, and use byte ranges for exact measurement. Keeping old summary values for compatibility while adding an explicitly named exact measure is possible; claiming old values were exact is not.

Minimal dependency set: existing boot-artifact selection, existing XML generated-comment codec in `vibe-specdoc`, TOML parsing, serde for a narrowly versioned report, and existing SHA-256 support if report identity is exposed. A single captured in-memory STATIC/INDEX pair with content identities is enough for one invocation. A retained source-query store, overlay/edit service, durable operation reservation, native process grants, derivative catalog and compiler IR DTOs are not prerequisites.

Do **not** transplant `vibe-spec::BootGraph` wholesale just to hold marker data: next's `compiler/boot_graph.rs` embeds `vibe_wire::generated::compiler_ir::e1::ir::Ir` and integrates all compiler carrier families. A small artifact-view structure can reuse main's `StaticContribution`/`EmbedSpan` vocabulary without importing a new compiler/machine wire world. If a second real API consumer needs the reader, move the shared artifact decoder to its existing workspace/boot owner then; the mere existence of next's abstraction does not make that move necessary now.

Captured artifact versus current sources: first delivery should measure the captured artifact exactly and leave source payload sizes unavailable unless explicitly requested and actually read. Next's `measurement::capture_texts` also opens paths named in marker/index rows via `raw_file`; that is a broader authority and observation window, not inherent in parsing a generated artifact. A displayed path is not permission to read arbitrary files. If current source reads are added, validate containment/no-follow, retain their own observed identity, and label them current, not the historical source of the committed artifact.

Compatibility: leave compiler generation bytes/order/status/void behavior untouched. Do not evaluate INDEX conditions or treat the INDEX pointer as another dynamic read. No boot-size quota. Exact bytes/scalars are not exact model tokens or billing. Unknown source positions, provider reasoning, conditions and actual runtime inclusion remain explicit unknowns.

Missing user path: the intended read-only explain selection and report need a real CLI/TUI consumer, not only a module. One coherent extension of the existing tree is sufficient; a separately versioned explain report avoids retrofitting new mandatory fields into the closed tree JSON v1. New CLI spelling is a design decision, not claimed to exist.

Not covered: preinstall install/remove/reconfigure cost. That needs both complete solver-selected worlds and real compiler semantics. It must not be approximated by summing independently cached package contributions, because shared/transitive dependencies and feature choices make deltas non-additive.

## Recommended independent slice 3: explicit ordinary-document inspection

User scenario: inspect a captured foreign SKILL.md/README.md, browse headings and retrieve a section while preserving literal `@fact`, `@requires`, `#use`, inline code, scripts and links as document content. No install, parser inference or network call occurs.

Next source: `vibe-specdoc/src/commonmark.rs::CommonMarkDoc::parse`, its tests, and `commonmark/export.rs::resolve_export`. The parser itself imports only `pulldown_cmark` and std, stores the original string and byte spans, and never calls Vibe SpecDoc/fact/directive readers. `resolve_export` is independent of manifest types and adds SHA-256 over the parsed bytes. The parser uses GFM tables to preserve inline structure; document that choice instead of asserting a different dialect. The next manifest calls require declarations, but the pure parser mechanically does not depend on the classifier.

Main equivalents: `vibe-specdoc` currently offers Vibe Markdown/XML conversion and legacy suffix dispatch; these should not be repurposed as ordinary CommonMark. Package skill projection already retains `DeclaredSkill.source_root`, provider identity and relative include information. Raw-copy preservation is an existing concept; adding a parser is useful for navigation, not required for byte copying.

Minimal transfer: additive pure ordinary-document API plus a concrete explicit resource/skill inspection consumer that reads one selected resource from an established trusted/captured root. Main currently lacks this ordinary structural view. `pulldown-cmark` is not in main's workspace manifest at the pin, so adding it and its lock resolution is a real dependency change to review. `sha2` already exists in the workspace; the specdoc crate would add it only if it owns a digest-bearing selection result. No `vibe-core` descriptor or machine-framework dependency is necessary.

Durable exports are optional for this first user scenario. Request-local exact heading-path selection can return a snapshot-local node/span. If authored `source_exports` are introduced, that is a separate manifest format change, with validation and migration/compatibility obligations; do not call local ordinal IDs durable facts. Duplicate sibling headings must refuse ambiguous selection, never take the first hit. Links remain strings, not fetched resources. Raw source must remain available when selection fails.

Boundary: an explicit inspect operation is not a global declaration that all consumers now treat the resource as inert. Do not send its bytes into `load_spec_text`, `from_markdown` or boot directive parsing. It should use the existing resource/skill selection context, not silently reinterpret arbitrary `.md` anywhere in a project.

Possible independent companion: `vibe-specdoc/src/xml_data.rs::inspect_xml_data/preserve_opaque`. Main already depends on `quick-xml`; a pure optional XML outline can retain exact raw bytes, refuse external entities/unsupported encodings for structure only, and never reserialize. This is useful only with an actual resource inspector; adding an unconsumed parser is not a completed user feature.

## Optional later slice: opt-in resource classification for materialization

User scenario: a selected package contains a normal prose README and an XML data fixture under a location currently eligible for spec conversion. Installing into an XML-target project must preserve those resources byte-for-byte while converting explicitly selected Vibe specs.

Main `vibedeps/derived.rs::is_spec_document/transform_file` infers conversion eligibility from `.md/.xml` plus specs-root/root-README location. Parsing success can normalize ordinary Markdown into a Vibe representation; raw copy outside those paths already exists. Therefore “ordinary resource preservation everywhere” is not delivered by the inspection slice.

Next provides usable pieces: `vibe-core::manifest::SourceDescriptor/select_source_rows`, declared reader selection, descriptor map and transformed-slot records, logical-resource collision checks, and deterministic authored source migration. However, carrying `commonmark` through materialization while leaving facts/resolver/boot to infer suffix would make one declaration mean different things to different consumers. A narrow opt-in can avoid global migration of every old package, but it must implement every semantic consumer that can encounter the opted-in package, or explicitly refuse unsupported consumer operations. It cannot silently fall back to Vibe semantics for a declared ordinary source.

Compatibility correction to earlier audit: next `PROP-072#SOURCE-HOST-OPT-IN` explicitly preserves legacy behavior when project.sources is absent/empty; `SOURCE-REPRESENTATION-COLLISION` explicitly permits a bounded undeclared-package compatibility route. Therefore legacy loaders remaining in next are **not by themselves a violation of the refined source contract**. The old task wording “no unclassified semantic path” was broader than these later qualifications. Target design can adopt per-package/project opt-in with unchanged old manifests. It still needs consistent behavior within the opted-in boundary and exact file-versus-directory precedence; introducing a converter-only exception is insufficient.

Recommend start with the first three slices before this broader change unless a concrete install-preservation user scenario is the next priority. This ordering follows independent semantics and present consumers, not lower effort as a design criterion.

## Publication finding corrected and bounded

The earlier term “legacy publication bypass” was too strong without identifying the governing contracts. Source evidence is unchanged, but the interpretation is corrected:

**Old live contract:** `PROP-019-version-manager.xml#release-production`, specifically `RELEASE-MUTABLE`, is `impl/done` in main and next. It explicitly says `dist prepare --version V` checks exact tag→commit, creates an absent tag, deletes an existing draft **or published release**, and creates a fresh draft. Builds opt into `--checks/--tests/--self-check`; `RELEASE-SECRETS` describes narrow credential-bearing CI prepare/upload/finalize steps. `xtask/src/dist/mod.rs::DistCommand::Prepare` even describes deletion in command help. This is an explicit operator-invoked legacy release route, not an undisclosed side effect of a read-only planner.

**New planned contract:** `PROP-073-publication-authority.xml` opens with “This document governs a future publication operation,” has stage spec/plan, and retains PROP-019 as input contract. Its `PUBLICATION-JOURNAL-BOUNDARY` constrains a protected machine submission and keeps it unavailable until response-loss recovery is supported. `publication_full_plan/calls/driver.rs::drive_one_fixed` is crate-private and says no live route invokes it; `calls/owner.rs` similarly has no public route. New `dist prepare-local` and `plan-local` are separate local operations, not aliases for legacy `prepare`; `run_plan_local_into` always emits apply_ready=false for operator-supplied remote observations.

**Observed gap:** the new prepared-plan/authority/journal semantics have not been connected to replace the old `dist prepare/upload/finalize` routes. That is a missing future integration, consistent with Zap still listing M-07-D/E unfinished. It is not proof that legacy commands evade an already enforced global runtime grant system. General operator authorization rules still apply to whoever invokes a mutating command, but that is separate from code-level grant enforcement.

**Actual user-visible risk under the old design:** deleting a published release removes its release/assets before a replacement is ready. If draft creation or later upload/finalization fails, the old release is not restored by this route. Exact tag/commit and digest verification do not restore deleted release assets, authorize a newly observed remote generation, or preserve latest-release availability. `prepare_with` has no new full-subject readiness/journal input. This is the established delete-first operational behavior, with a real failure window; it should not be represented as protected recoverable publication merely because new journal code exists elsewhere.

The new proposal itself acknowledges another concrete condition: `PUBLICATION-LATEST-BASELINE-LAW` makes deletion of the current latest release unavailable for the protected delete-first plan until a separately approved fallback strategy exists. That stronger protection is not supplied by legacy `prepare`.

Transfer implication: do not copy the whole private new publication framework solely to “fix a bypass.” First choose the intended user operation and its guarantees. If protected publication is required, connect one reviewed prepared-batch route and define old-command compatibility/deprecation deliberately. If preserving legacy explicit operator commands, keep their documented destructive semantics and do not advertise the stronger new guarantees for them. No external mutation or policy change was performed in this investigation.
