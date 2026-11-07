# A4 progress captured-corpus adapter — exact next-slice notes

Read-only preparation, 2026-11-07. No code/tests/Cargo/Git mutation. Root chooses dispatch. Active show writer remains untouched; seal mutation is excluded. Full A4/counts12/31/31/39 unchanged.

## Operation scope is progress.toml, not source declarations

`progress::grounding::ground` loads ScopeConfig, campaign cache and sidecar, then `progress_core::scope::observed_files_reported`. The latter expands include globs, applies fixed component/file exclusions and configured exclude globs, sorts/deduplicates and reports excluded/stale patterns. It has NO inherent md/xml filter. Only DEFAULT_INCLUDES name those suffixes. Therefore an explicit observe/include pattern can already select arbitrary native .payload; don't enroll every declared source or change standalone progress-core defaults to make the test pass.

The declared reader must classify the files this operation selected. Ordinary selected files do not enter progress parsing; native explicit remains valid in a requested observation. Explicit-empty contributes zero. Unrelated missing/malformed declarations outside progress scope cannot poison it. Preserve actual exclude reports and user scope; do not replace these with capture_host_inventory followed by filtering.

## Current guarded path and minimal reader replacement

Ground currently rejects explicit root policy; VocabularyByPackage walks nearest manifests and rejects explicit packages before pair checks/cache use. The parse loop then load_spec_text_with(path,vocab), hashes the projected text, loads a cached ParsedDoc or parses the same projection, and records XmlProjected paths. This is a safe refusal but not supported declared progress.

Replace only the selected-native read stage with a retained operation observation per actual owner:
- original host/registered authored package: shared descriptor selection over held root/source policy, original path/media and captured bytes. Inside an explicitly selected outer source scope, a random nested manifest is data, not automatic new package identity. Registered separate roots retain their actual owner where the operation selects them;
- installed slot: independently retained full lock kind/group/name/version/hash and trusted selected format, existing prepare_declared_query_read + read_installed_source. Map physical output to original resource through the binding, never infer from output suffix;
- explicit raw/ordinary/unclassified: skip native parse regardless of @fact/#use text, including previously cached native-looking content;
- declared unsafe/ambiguous/missing selected native: explicit error, not silently missing corpus; legacy stays its separately qualified compatibility route.

No fake group needed for path-based parsing/counts, but real native address semantics retain their coordinate requirements. No fake PackageMeta/record or new source world. Use existing source policy and installed read owners; any small shared named-file adapter from accepted tree/show should be coordinated after its writer freezes rather than copied into another framework.

## Projection/cache identity and later reads

Retain the exact canonical text passed to progress_core::parse_document along with its original logical/report path and selected physical diagnostics. Cache key must remain the existing projection content hash; raw XML witness hash and projection hash are different quantities and must not be substituted. Classification precedes any cache hit, so an ordinary row cannot regain facts from an old cache entry. Preserve native-only collision checks, status/void and existing projected-XML diagnostic notice. For transformed paths, do not accidentally relabel native logical identity as converted physical identity; pin this in an original/converted twin test and preserve reported locations explicitly.

Ground should carry a bounded map of the selected projected texts or equivalent retained read results already consumed. `progress.rs::weave_cmd` currently rereads `g.root.join(d.path)` through load_spec_text; replace that with the same captured projection. Otherwise arbitrary-suffix XML would be treated as Markdown and the weave text could differ from the parsed/cached text. Do not compute a fresh representation solely for weaving.

Retain source/policy/binding rechecks through model production. Existing `Ground.source_policies` only guards manifest observations; declared selected bytes/membership need their actual retained source owner, not just a renamed boolean. Missing/untrusted selected input must not cause a successful cache prune. No cache schema or digest-recipe change is implied if canonical text and parser semantics stay unchanged.

## Shared ground is NOT read-only — exact effect map

| Caller | Actual behavior / required barrier |
|---|---|
| progress report | ground→report_body→stdout; read-only. |
| progress rescan | ground + baseline comparison/output; inspect chosen output options, preserve existing read-only result path. |
| progress check / facts check | same facts_check::run; write-state defaults false. `--write-state` calls refresh_state. |
| progress scan | immediately calls refresh_state, which writes campaign cache/payload/state when a campaign exists. Not categorically read-only. |
| progress mirror | writes per-file mirror JSON BEFORE calling refresh_state_with_evidence. A guard only in refresh is too late. |
| progress resume | uses ground/journal/evidence and writes refreshed campaign state; inspect its direct outputs before enabling declared mutation. |
| progress baseline | separate baseline.rs store after ground; refresh guard does not cover it. |
| progress weave | stdout/digest view is read-only; --out writes shards in emit_weave, outside refresh. |
| progress gate / seal | do not rely on ground. Seal remains separately guarded and outside this packet. Gate records human-supplied verdicts and must not be accidentally pulled into a source-read rewrite. |

Two safe implementation choices, root to pin before dispatch: (A) complete common captured read support and add recheck immediately before EVERY affected existing write boundary, including mirror/baseline/weave-output, testing each activated mutation; or (B) bounded first read-only slice with explicit operation mode passed to grounding/effect admission, retaining precise Unsupported for declared mutation before any write. Do not silently enable mutating callers by simply removing ground's Unsupported. Choice B is smaller and honest intermediate A4 work; it must remain visibly unfinished rather than counted as completed progress support. Read-only check/report/weave-stdout are useful first positives.

Legacy mutation behavior remains unchanged. No global transaction/rollback is required: a late per-file I/O failure may keep its current partial-output semantics, but known source admission failure must precede relevant output.

## Trusted configuration and perimeter

CLI owns installed input format using existing effective_rederive_format/root manifest/operator defaults. progress-core must stay free of vibe-workspace dependencies. Pass selected format/read context through the CLI grounding owner; never infer from STATIC or slot record. ScopeConfig/user include/exclude semantics stay with progress-core. Bound source selection should not reinterpret external sidecar/evidence formats as Vibe documents.

Suggested first packet: progress/grounding.rs plus bounded source-read child, Ground captured projection storage; progress.rs read-only report/weave consumption and explicit effect guard placement; facts_check.rs no-write path; relevant existing tests. Keep scan/mirror/resume/baseline/--write-state/--out guarded if not fully integrated in the same packet. Any changed signatures at those callers are only to preserve safe admission, not a broad campaign-state redesign.

## Actual tests and nonzero discriminators

Existing owners: `commands/progress/tests/xml_sources.rs` covers XML/Markdown unit equivalence, stable named XML facts, XML projection header, native pair collision and write-state default; `progress/grounding/tests.rs` covers policy admission; `progress/evidence_boundaries.rs` guards evidence before state writes; `tests/cli_progress_sidecar.rs` covers cache behavior. Bind actual test names before executing; this audit runs none.

New real CLI fixture should use progress.toml include explicitly selecting native XML .payload, Markdown and ordinary same-stem literals, with excluded malformed native input. Run `vibe facts check --path <root>` (or existing progress check alias), progress report and weave-to-stdout with exact existing args. Assert native status/void/unit equivalence, no ordinary markers/directives, correct XML-projection diagnostics and stable second-cache read. Weave body must equal the projection actually parsed. Snapshot project files/directories for read-only calls, including missing/ambiguous/drift negatives.

A separate command test exercises each mutation option retained Unsupported and asserts refusal before cache/state/baseline/mirror/shard creation. If mutation is activated instead, require retained-input drift before its FIRST write, not merely before refresh. Test legacy mutation controls. Installed declared input requires actual local-registry install in Mixed/Markdown/Xml, original path mapping and missing/wrong lock/format refusal. Do not claim default include patterns promised .payload unless configured.

This plan adds native corpus reads while preserving scope/cache meaning and keeping unimplemented effectful consumers explicit. It is not full progress/A4 completion by itself.

## Root dispatch decision

Choose A for the shared ground consumers: complete captured reads and the existing derived-output boundaries together (scan/check write-state, mirror JSON, resume, baseline, weave --out). Inspection confirms these routes write derived artifacts, not authored native documents. Add source/evidence rechecks before their actual first write and retain existing partial-I/O semantics; do not introduce a transaction framework or claim only refresh_state is sufficient. Report/rescan/check default/weave stdout remain read-only. Gate's human verdict input stays unchanged; seal's separate source reader remains the next explicit adapter. Preserve progress.toml scope/defaults, projection-based cache identity and source-kind diagnostics. No blanket Unsupported wrapper is the final result for shared-ground commands.
