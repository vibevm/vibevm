# A4 read-only views — sequence by actual operation scope

Source-only design, 2026-11-07. No edits/tests/Cargo/Git mutation. Counts remain12 full /31 partial /31 no accepted transfer /39 unresolved; A4 remains partial. Active docs writer owns docs point reads and its narrow declared-manifest accessor; do not edit that perimeter concurrently. Seal mutation and publish staging are excluded here.

## Recommendation: tree first, then show, then progress

These operations share source observation/classification primitives, not one inventory or semantics. A generic whole-host census would be wrong for tree/show, and replacing progress scope with package declarations would change the user's selected corpus. Implement one narrow consumer at a time with operation-specific positives; reusable read code should remain a small CLI adapter over existing owners, not a new source world.

| Operation | Already working | Remaining actual gap |
|---|---|---|
| tree | Computes loaded boot_file_set from INDEX entries, STATIC provenance and host boot entries; generated outputs are not parsed as authored directives. | in_place.rs refuses nonempty declared policy, then legacy load_spec_text/Directives::parse. It needs native selected media/bytes for the already named files, not a new scan. |
| show effective | Generated STATIC tape is returned verbatim; explicit-empty authored input is skipped; missing legacy written-file diagnostic survives absent parent dirs. | Nonempty authored input refuses. Boot candidate list filters is_spec_source before selection, so native .payload is absent even if read guard is removed. WAL and files_written have distinct scope; none should become a global source census. |
| progress views | Own progress.toml observed/excluded scope, cache/payload format, XML projection diagnostics and legacy equivalence tests exist; explicit policies currently refuse before parse/cache reuse. | Needs classified captured selected sources and retained projected text. Ground is shared with mutating verbs; read-only activation must not accidentally claim those complete. weave later rereads paths by suffix, so ground-only change is insufficient for complete read-view support. |

## Narrow read seam using existing owners

For each operation-selected file, resolve its actual owner and source intent: caller-selected host root versus exact lock-selected installed package. Host original resources use held observe_package_policy/source_rows/shared selector and original source reads; automatic own boot can reuse capture_authored_boot for the exact enrolled set. Explicitly named native files use exact selection, not automatic scan-only filtering. Ungrouped host read/display can retain original path/media without minting a fake spec coordinate. Address-based point requests keep the accepted grouped reader rule.

Installed reads preserve complete LockedPackage kind/group/name/version/hash and trusted selected materialized format, call existing prepare_declared_query_read and read_installed_source. The record's physical output→original path/media mapping must identify the actual listed file; no guessing the original identity by stripping a converted suffix. Caller-supplied format comes from existing effective_rederive_format/resolve_spec_format at the CLI boundary; no slot-record expectation or STATIC suffix inference. Do not reuse Adoption::prepare merely as a generic reader: it includes registry mutation/destination concerns these views do not own.

Retain captured original bytes, selected interpretation/media, diagnostic path/original logical path and recheck obligation through parsing/rendering. Native projection uses existing project_declared_native; ordinary content never passes through Directives/progress fact grammar. Equal supplied overlays and current capture checks follow accepted owners. Existing legacy branch remains explicit. If a narrow helper is truly missing, root should choose one existing source owner extension; do not create parallel docs and views capture frameworks.

## Slice1: tree in-place directive view

Perimeter: commands/tree/build/in_place.rs, small build.rs signature/format threading and tree entry caller, existing tree build tests plus a new bounded real CLI fixture. Keep graph/load attribution algorithms and generated tape parser unchanged.

Input universe is already boot_file_set assembled in build.rs; map each named source to held host or selected slot binding. Read only selected native VibeSpec inputs; ordinary listed paths produce no managed in-place directive. Preserve genuine parse/read failures as existing documented diagnostic/error semantics rather than silently treating an unsafe declared source as zero directives. Existing legacy tolerant parse path can remain separately qualified. Never prefilter policy rows before selector precedence.

Actual discriminator: install the accepted own-boot fixture with XML native arbitrary suffix containing a real native #use, ordinary same-stem containing forged #use/#embed, plus a declared installed package snippet. `vibe tree --json --path <root>` must show real in-place reference once, no ordinary reference, correct file/projection location and existing load origin. Compare legacy twin. Corrupt selected record/hash/input-format and require visible failure; snapshot read-only state. Reuse `tree_fixture.rs`, `tree_nested_attribution.rs`, `tree_json.rs` semantics without running the real-repo fixture as sole evidence.

## Slice2: show effective selected authored content

Perimeter: commands/show/effective.rs and its embedded tests plus actual CLI fixture (existing cli_host_sources.rs contains generated/empty/missing controls). No docs writer overlap.

Select declared authored boot from actual effective INDEX/accepted authored-boot capture so arbitrary native suffixes are included; generated INDEX/STATIC stay their existing direct metadata/tape branch. Do not enroll ordinary/explicit-only host files merely because they sit in boot. Preserve ordering/origin conventions and dependency STATIC content. For explicit WAL and files_written paths, use their existing operation request rather than a broad scan. Named ordinary UTF8 data may be shown literally if that surface already includes the path, without native projection; nonUTF8 needs an honest unsupported-content diagnostic rather than lossy decoding. Root should pin that presentation choice before adding output fields. No invented source URI or report schema is required.

Actual `vibe show effective --json --path <root>` after install must include arbitrary-suffix selected native projection, preserve generated repeated-root XML tape verbatim, omit non-enrolled ordinary boot, and retain explicit-empty/dependency-only behavior. Add a concrete named raw files_written case if claimed supported, checking literal directive text is not executed or reinterpreted. Unrelated missing explicit sources outside selected display scope must not fail this view. Keep the legacy missing-written-file diagnostic and no-write snapshot tests.

## Slice3: progress corpus views

Perimeter: progress/grounding.rs (+ bounded child), existing progress/tests/xml_sources.rs, grounding/tests.rs, any read-view caller that currently rereads text (notably weave in progress.rs), and a focused CLI target. Do not broaden to seal in this packet.

The selected corpus comes from progress.toml's observe/exclude rules, not all declared host inventory. Establish candidate selection and native interpretation together: declared raw candidates are not progress documents; native arbitrary suffix under an explicitly observed scope must be selectable. Check scope/enumerate's actual filename rules before changing lower progress-core; do not assume defaults automatically promise every extension. Directory declarations do not erase user excludes; a directory default only classifies files the progress operation actually selects. Nested manifests must not silently replace the selected outer source policy inside that operation's declared scope.

Capture only operation-selected files, classify before pair checks, hash/parse cached projections from those same bytes and preserve XML-projected diagnostics. Pair/native canonical collision law applies to native candidates only. Keep a retained projection/read result for weave instead of a second suffix-based load_spec_text. Cache remains acceleration, not source authority. Extend current Ground recheck mechanism through existing observation objects where needed; don't advertise mutating progress subcommands as finished merely because they share ground.

Actual commands: existing progress tests exercise `progress scan` and `progress check --path`; use those real argument fixtures and `cli_progress_sidecar.rs` for cache behavior. Equivalent declared XML/arbitrarysuffix and Markdown native corpus should yield matching status/void/unit counts, ordinary FAKE markers excluded, raw same-stem legal, repeated scan stable, and unrelated excluded malformed input untouched. Ensure the tested command uses read-only options (`check` defaults write-state off); snapshot source/project state and keep no-cache positive. Weave must use captured projected body and preserve ordinary/code literal rules for native documents.

## Shared prerequisites and closure

Tree/show can use accepted own-boot paths and point/installed read owners immediately; they do not require host pattern expansion or docs census. Progress's scope policy and projection retention are additional prerequisites, so it should follow those simpler named-file consumers. Trusted format selection may be a shared small CLI helper, but no generic registry/world discovery layer is warranted. Current docs writer should be consulted before touching any lower accessor it owns.

Exact initial validation targets: `cargo test -p vibe-cli --test tree_fixture` (legacy oracle), selected tree build unit family; `cargo test -p vibe-cli --test cli_host_sources` (existing guard expectations must be deliberately updated to positive support); `cargo test -p vibe-cli --lib commands::progress::tests::xml_sources` (confirm actual registration/name before execution), and new focused real CLI positive target for each implemented slice. These are proposed commands, not executed receipts. Root must bind nonzero case names at implementation time.

This route closes three read-view adapters without redefining full A4. Remaining seal, publish, docs page census/borrowed joins, host patterns, skill proof and tutorial corrections stay separately visible.
