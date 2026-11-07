# A4 show-effective exact adapter packet notes

Read-only preparation, 2026-11-07. No edits/tests/Cargo/Git mutation. Active tree writer owns tree files; this next slice must not edit them concurrently. Full A4/counts12/31/31/39 unchanged.

## Existing operation universe — do not replace with a host census

Actual call: show/mod.rs::run → effective::run_effective → collect_sections. It displays (1) root boot-directory files, presently filtered by is_spec_source; (2) optional current_wal_md; (3) each lock package's files_written, in lock order with sorted paths, excluding already handled root boot prefix. It does NOT recursively follow INDEX entry paths. INDEX is currently shown as metadata text, not a directive to run another source discovery. Preserve this universe; installing an INDEX traversal engine is not required for this adapter.

Generated STATIC.md/XML is already recognized and shown verbatim; XML may contain multiple framed roots. resolve_static_path preserves the two-generated-format conflict check. Existing explicit-empty authored paths skip, nonempty authored reads refuse, and absent legacy files_written still produce an empty-body MISSING ON DISK section even when parents are absent. Legacy 00-core/90-user provenance headers and WAL origin have actual tests.

Concrete gap: suffix filtering loses declared native arbitrary-suffix own boot before a read helper could see it. Merely deleting Unsupported is insufficient and would restore raw XML native parsing.

## Proposed selected paths and reads

**Own boot:** for explicit host/package policy, use accepted capture_authored_boot for immediate native+scan candidates, retaining policy/media/bytes and original paths; this is the operation's own-boot scope, not capture_host_inventory. Generated paths are collected separately using existing ownership/format selection; show does not parse their content as native. Ordinary and explicit-only native boot files are not auto-enrolled just because they have .md/.xml suffix. Preserve incumbent exact category/order/origin formatting. Legacy absent policy keeps its current directory/file behavior. Explicit empty may still show generated dependency output.

**Generated tape and INDEX:** preserve actual bytes/text. STATIC tape must not enter single-document project_declared_native. INDEX remains displayed metadata; no invented provenance from authored comments or new parser. The tree artifact parser is private today; this plan does not require moving or duplicating it. A future explicit INDEX-following behavior would be a separate scope decision.

**WAL:** current command explicitly selects WAL.md only. Under declared policy classify that exact path before reading; native selected content uses declared media, ordinary UTF8 remains literal data if the named WAL view includes it. Explicit empty suppresses authored WAL as now. Do not broaden to every declared WAL-looking source or whole-tree census; XML WAL support would need separate deliberate selection change rather than a filename guess.

**files_written:** each named path is an explicit display request, but the lock package label alone is not proof of source interpretation. Determine whether it is a selected installed-slot output, a generated artifact or an actual project-written path. For installed output, retain full LockedPackage kind/group/name/version/content_hash, independent selected format, held slot/manifest/record and output row. Use existing prepare_declared_query_read, map exact physical output_path→source_path and read_installed_source. Native media drives projection; ordinary/unclassified recorded bytes are literal display data, never facts/directives. Original logical source identity belongs in the provenance decision; converted physical path remains the displayed path. Do not infer original identity by replacing suffixes or describe an output hash as original source hash.

For non-slot written paths, use the selected actual host/registered-node policy and exact held read, not the installed package's descriptor applied to an unrelated project file. Generated ownership keeps its separate lane. Unknown ownership inside dependency layout must refuse, not become a fresh caller-root grant. Missing legacy written files retain existing diagnostics; declared missing native/output binding must be a truthful unavailable/error outcome, not a fabricated empty success with native interpretation.

## Minimal retained reader and format ownership

A CLI-private selected read result needs exact original/physical path, optional declared media/interpretation, captured bytes and existing policy/binding recheck. Reuse accepted public APIs; do not route through facts Adoption::prepare because it adds registry mutation/destination concerns. No fake group needed for ungrouped path-based display. No spec address minting is necessary to read a raw file; keep current display-header grammar unless root explicitly approves a provenance compatibility correction.

Actual format owner is effective::run_effective after resolving root: load the real manifest and reuse commands/reinstall::effective_rederive_format (or the same existing trusted config selection used by the accepted tree adapter after it freezes). Do not infer materialized input format from selected STATIC filename, record.spec_format or user-interface preferences. Add optional/bound collect_sections sibling for tests/legacy wrapper if useful; current old helper calls must not silently guess format for declared installed sources.

Recheck retained observations after reading/projection and before emitting the assembled report. collect_sections already builds the entire result before printing, so a failed selected read can refuse without partial JSON. Output is a String body: ordinary UTF8 bytes can be represented exactly as literal text; nonUTF8 cannot. Preserve an honest unsupported/read error rather than lossy replacement, invented binary encoding or an unreviewed report member. Never invoke Directives or external providers while displaying literal ordinary content.

## Exact perimeter / source conflicts

Own commands/show/effective.rs and a bounded helper child if needed, plus focused new CLI test target. show/mod.rs needs changes only if entry context must carry configuration. Reuse core/spec/workspace installed and original read APIs; request any missing narrow accessor centrally. Do not edit active tree files, docs, progress or seal. Existing report fields remain ok/command/project/sections and section spec_uri/path/origin/body; no JTD/schema invention in this slice.

Current missing seams are command-level actual format threading, output→original selected binding lookup for named installed files, and retained own-boot/native/data read assembly. Existing lower primitives provide these ingredients; a new global capture service is not warranted.

## Actual tests and proposed nonzero gates

Existing tests, not newly executed here:
- embedded effective.rs tests: generated_static_tape_is_read_verbatim_in_both_formats; an_xml_boot_file_lands_as_its_projection.
- cli_registry_mgmt.rs: show_effective_emits_boot_files_with_provenance; show_effective_includes_wal_when_present.
- cli_spec_format.rs actual show-effective selected STATIC assertion.
- cli_host_sources.rs generated-only/empty and missing legacy written diagnostics. Guard-only refusal expectation must be deliberately updated when this supported path lands, not silently deleted.

New proposed actual target `cli_declared_effective`:
1. Actual local-registry install, grouped and ungrouped host native+scan XML arbitrary suffix +Markdown own files, raw same-stem directive-looking boot neighbor and explicit-only file. `vibe --json show effective --path <root>` includes correct selected own native projection, omits non-enrolled authored neighbors and keeps dependency STATIC tape verbatim. Source bytes unchanged.
2. Real files_written installed native source in Mixed/Markdown/Xml uses original logical row/media; explicitly named ordinary UTF8 XML/text is shown literally with #use/#embed inert. NonUTF8 named resource refuses clearly without partial report or source mutation.
3. Missing/wrong independent format/hash/record/output refuses instead of suffix fallback; unrelated invalid/missing declaration outside shown scope does not poison. Explicit empty still displays generated tape, legacy absence retains existing boot/WAL/MISSING diagnostics.
4. Snapshot files/directories around every read-only invocation. Exact header/schema/control assertions remain; no lifecycle lease/registry mutation.

Proposed command only after target exists: `cargo test -p vibe-cli --test cli_declared_effective --offline --locked`. Bind existing unit names against actual test registration before execution. No runtime success or completion is claimed by these notes.
