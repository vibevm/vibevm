# A4 documentation captured-source adapter decision

Read-only, 2026-11-07. No implementation/tests/Cargo/Git mutation. Root authorizes the next slice. Accepted host point reader and installed package preparation are reused, not reopened.

## Smallest useful first slice

Complete actual **point citation and agent page reads** through the existing SpecSources composition: `citations::resolve` and `agent::read_page`. Do not begin by removing page-census guards everywhere. These two public operations currently select an instance, get a path, reread it and guess the parser from suffix or force Doc XML. A bounded captured-document result can repair both and provide the prerequisite for page builds/borrowed examples.

SpecSources already owns the source precedence checkout → in-tree → lock → store. Preserve that configured chain rather than create another source world. Add an operation such as `capture_document(address)` returning selected source/lang, original logical relative path, physical diagnostic path, declared media/interpretation, raw bytes and a retained/recheckable existing source observation. Name is proposed; no existing method is claimed. Keep public Located/path-only behavior guarded for compatibility instead of silently enabling unchecked rereaders.

For an authored checkout/in-tree/store package root, use the accepted held policy + capture_addressed_native owner, actual manifest coordinate and source interpretation. This admits original source roots, not derived slots relocated into a registry-looking location. For a lock-selected materialized instance, retain the complete independently selected LockedPackage (kind/group/name/version/content_hash), selected materialized format and held workspace/slot; call existing prepare_declared_query_read and bound FileResolver read. The existing vibe-doc→vibe-workspace dependency already permits this. Do not build fake PackageMeta, parse SlotRecord again or derive expected hash/format from output metadata.

## Concrete missing selection inputs

`locked_instance` currently reads Lockfile then returns only Source::Lock/version/root, dropping the selected hash and materialization information. It maps load errors to None, permitting a later store fallback. Refactor only the new read selection through a fallible internal selected-instance result: genuine absence may advance the configured chain, but an actually selected lock source with malformed/missing binding cannot silently become another store version. Retain current explicit version/self-coordinate rules and precedence. Any stricter error behavior relative to the old optional helper must be recorded as compatibility, not hidden.

SpecSources currently stores no input SpecFormat. Add an optional trusted builder/context input for materialized reads; unbound declared lock sources refuse precisely while legacy APIs continue bounded compatibility. The actual CLI owner is `commands/doc/env.rs::spec_sources`, called by build/check/serve/site paths with DocEnv.cwd/settings. Resolve selected workspace root + existing effective_rederive_format/resolve_spec_format policy there; do not read user config inside vibe-doc. An infallible helper may need to become Result with real callers updated so bad configuration does not disappear as None. No new CLI/wire option. A server receives already-configured SpecSources in Reader::open, so retain that injection boundary rather than add server ambient configuration discovery. In-tree/store authored originals need no materialized format.

## Direct parsing and retained identity

`citations::resolve` should obtain captured bytes once, dispatch declared native media into the existing Spec/Doc pivot appropriate to the actual interpretation, then anchor_text; it must not call path-only anchor::read_spec for declared input. Read_spec may remain a clearly legacy explicit-path primitive. Recheck the same captured selection before returning citation text; metadata and content must describe one selected input.

`agent::fetch` should consume that capture directly as an actual Doc page. A plain project with a vibe-doc row does not gain Doc vocabulary; only a real Doc package grants it. VibeSpec remains a spec citation, not automatically an agent documentation page. Ordinary CommonMark/XML-data/opaque are not native pages/citations merely because their text has anchors/directives. Keep raw resources literal and do not introduce exports/resource URIs.

Agent currently constructs page.rel by appending `.xml` to parsed.doc_path. That is a concrete additional gap: an original `guide.payload` declared as Doc XML must retain its original relative page identity instead of becoming `guide.payload.xml`. Use captured original path plus the established spec-root mapping for Page.rel/borrowed-page addressing. Physical converted path is diagnostic/storage location, not logical identity. XML canonical projection positions stay distinct from original byte locations; direct Doc AST reading should not route through Markdown and lose note/figure/example metadata.

Existing language adaptation fallback tries another page then falls back; preserve legitimate missing-adaptation behavior. A failed bound selected source must not be mislabeled as simply absent and silently swapped for a different unvalidated instance. Language metadata for the new captured route should derive from the same validated manifest observation instead of canonical_language's separate raw-TOML reread.

## Subsequent full page/borrowed join (not first-slice scope)

pages::read_package currently returns empty for explicit-empty and refuses nonempty; documents/walk_package enumerate XML paths by suffix. After point capture works, add finite declared page inventory using actual Doc interpretation and original paths, retaining legacy layout behavior where absent. Full build/server routes need a bound variant when package_dir is an installed slot; do not treat a bare path as proof of installed selection. `build.rs` and doc-server Reader currently call read_package directly, so support must reach those actual edges before claiming builds work.

Self-citation validation and borrowed()/borrowed_by() must share the same captured source selection. Ordinary raw rows remain excluded, missing/ambiguous changed native sources refuse, and unrelated ordinary bytes do not become Doc parser input. Preserve source-language translation relationships and existing error/report shape. Do not simply remove Unsupported and reinstate fs::read_to_string.

## Exact first-slice perimeter and tests

Production: vibe-doc citations/sources.rs (bounded child allowed), citations.rs::resolve, citations/anchor.rs pure captured parse helper, agent.rs::fetch; CLI doc/env.rs and actual callers only for trusted input selection. Reuse public policy/installed APIs; request a narrow lower accessor only if captured original identity is genuinely unavailable. Server setup only if needed to supply existing trusted SpecSources. Page-census/translation adapters can remain named pending until the second slice.

Public tests: authored declared native XML under arbitrary suffix resolves actual rule anchor and fullslug; ordinary same-stem excluded; actual Doc package original page payload renders via agent::read_page with original identity and note/figure content. Actual materialise+lock selected Mixed/Xml source succeeds with supplied input format and refuses missing/wrong format/hash/record/output; Doc Markdown remains unsupported per existing one-way law. Wrong coordinate/version and changed captured source refuse; store does not mask a selected binding failure.

Real CLI discriminator: legacy-authored documentation package containing a `<rule ref=...>` to a declared arbitrary-suffix native source; invoke existing `vibe doc build --path <doc> --out <temp> --format md` from the selected checkout and assert rendered current rule text. Repeat with local-registry-installed declared source and selected format. This exercises the new citation path without falsely claiming declared page census already works. Agent page read is a real public library/server route; do not invent a nonexistent CLI `doc get` command. Later declared-doc build/serve tests close the census phase.

## Compatibility and remaining scope

New Rust capture/context APIs only; no manifest, URI, Doc IR or report schema change required. Preserve old path-only refusal rather than widening its safety contract. Record current-vs-original paths, stronger selected-source error propagation and trusted format requirement in FMT. No original publisher authenticity, whole-tree immutable snapshot or global CAS guarantee.

Full A4 still includes page/borrowed/self-citation census, host patterns, progress/tree/show/seal, publish staging, skills proof and tutorial corrections. This design supplies the first useful read seam and actual callers, not whole-A4 completion.

## Root verdict

Approved first slice: existing SpecSources captured native point reads into actual citations resolve and agent read_page, using direct Spec/Doc AST and original logical paths. Preserve checkout/in-tree/lock/store precedence, but selected lock failure cannot become store fallback; trusted selected format comes from actual CLI/server composition. Preserve guarded path-only APIs. Full page/borrowed census remains a subsequent explicit slice, not implicitly completed by point support. No source world, fake package metadata, URI/IR/schema or invented agent command.
