# M-06-A.4 consumer architecture audit

Read-only source audit, 2026-11-07. No Cargo/tests/Git mutation, no runtime proof, no A4 promotion. A3 accepted at 3fcf514a5; its reviewed reader/materializer/generated lanes are the base, not candidates to replace. Exact original contract including saved notes is preserved in the JSON companion from immutable prototype pin 7fe6376ccef7f321302a8f18610fcb0c9cc2686f, campaigns/next/tasks/M-06-A.json.

## Decision for the next implementation slice

Start with the selected installed requirements-query/facts observation join. A3 already owns validated slot mapping; query already owns independent lock identity. Keep that identity through source selection and call the A3 prepared-reader seam before interpreting facts. The old crosswalk predates A1–A3 and incorrectly says all facts/materialization are still suffix-only. The remaining gap is binding actual consumers, not creating descriptors again.

SourceCoord currently retains kind-of-report, package coordinate, root, locked flag and content_hash string; enumerate discards detailed selected package kind/version/materialization when constructing it. QueryContext carries selected_root, observed_at and lifecycle_run_id, no materialized SpecFormat. A transformed reader needs independently selected input format from the existing command/operator configuration owner, distinct from output lane target; never infer it from the record. Both CLI and MCP are callers of query, so keep one library semantics and document any added trusted context parameter. A bounded Mixed-first join is useful only as a named intermediate slice with explicit transformed refusal; whole A4 still requires the final matrix.

Original saved note8 unambiguously assigns validated [[project.sources]] and exact-citation host sync to A4. It is not indefinitely deferred host work. Reuse the selector law; absence stays legacy and explicit empty must remain distinct. Do not force package metadata or pretend a project is an installed package.

## Existing dependency and surface owners

`vibe-requirements` already depends on `vibe-workspace`; use that existing edge for installed preparation. `vibe-facts` already depends on `vibe-spec`, so lower prepared data can flow there without a reverse workspace edge. Actual surface constructors are `vibe-cli/src/commands/requirements.rs` and `vibe-mcp/src/tools/requirements_query.rs`. Preserve strict query grammar validation before filesystem discovery, the single query report constructor and read-only/no-lifecycle-lease execution. The MCP server already retains `lifecycle_policy.spec_format_default`; combine trusted operator default with selected project configuration using the incumbent selection law, not a new wire argument or slot-record guess. CLI uses its existing operator/config selection owner. Root verified this proposed next implementation perimeter; the source audit does not implement it.

## Current consumer inventory

| Consumer | State | Actual evidence / remaining behavior |
|---|---|---|
| package facts observation | supported exact originals; installed authority incomplete | observe_authored_source selects declared policy before parsing, exact paths held-read; raw/CommonMark do not mint facts. Directory census explicitly refuses. It does not receive installed selected hash/record/output map; inappropriate as an unbound installed reader. Sources: `crates/vibe-facts/src/scan.rs; scan/declared_capture.rs; scan/declared/walk.rs`. |
| facts adopt | supported Mixed exact; honest transformed refusal | Independent lock kind/group/name/version/content_hash and record bytes checked; non-Mixed and in-place explicitly refuse; pre-rederive checks before registry writes. Reuse this admission law and A3 map, do not add a second record validator. Sources: `crates/vibe-cli/src/commands/facts/selected.rs`. |
| requirements query/report | missing selected installed binding | enumerate retains content_hash in SourceCoord but observe passes root/coordinate/kind only to observe_authored_source. Slot record not validated here; transformed output names/media are not mapped. Hash remains report authority without establishing read identity. This is source evidence, not an executed exploit. Sources: `crates/vibe-requirements/src/sources.rs; query.rs`. |
| host facts sync | missing host policy | ProjectSection deny_unknown_fields has no sources. Sync enumerates md/xml, filters cited canonical doc paths, load_spec_text then parse_document, and later mutates registry. Original note8 requires exact cited declared paths, shared selector, nofollow capture and pre-write refusals. This belongs A4. Sources: `crates/vibe-facts/src/sync.rs; crates/vibe-core/src/manifest/project.rs`. |
| lookup / compiler / boot | accepted A3 bound installed support; legacy guarded | A3 supplies selected expectations/map/held reads for Simple/Normal/analysis/native/backend/replay; raw has no native address. Ordinary unbound resolver rejects explicit package policy. Preserve generated-only initial role, reference admission before cache, current attribution and explicit input format. Sources: `crates/vibe-spec/src/resolver/declared.rs; resolver/observation.rs; crates/vibe-workspace/src/boot_artifacts/declared_sources.rs`. |
| materialization | accepted A3 | Declared media conversion and finite inventory/no-clobber accepted. Do not rerun or redesign this owner to implement A4. Sources: `crates/vibe-workspace/src/vibedeps/derived/declared.rs; source_policy/inventory.rs`. |
| boot graph verification CLI | honest library refusal suppressed by surface | verify_boot_graph prepares declared sources with no input format, thus refuses; check append_boot_graph_findings drops any error. Bound format-selected verifier should use build_unit_table_bound and plan_digest_frames_for, and CLI should expose relevant refusal. Small dependent consumer fix, not whole A4. Sources: `crates/vibe-workspace/src/install/bootgen.rs; crates/vibe-cli/src/commands/check.rs`. |
| skill resource projection | explicit raw copy already; descriptor consistency unjoined | hydrate_declared_skill snapshots explicit skill/resource roots, merges bytes with collision checks; lower_one_binding validates selected frontmatter and portability, retains selected_files and snapshot digest. No Vibe parser/directive execution here, so not an unclassified semantic interpreter merely for copying. It does not use source descriptor/loading policy; need mixed fixture proving explicit resource selection remains exact and cannot enter managed compiler input. Avoid importing a second source world. Sources: `crates/vibe-agent-projection/src/pkgskill/projection.rs; projection/binding.rs`. |
| documentation citations/pages/agent | honest explicit refusal; read-result join missing | SpecSources uses unbound FileResolver and returns a path; installed declared lookup refuses. resolve_self, page parsing and borrowed examples explicitly reject policy. Agent fetch then fs::read_to_string + Doc parser and anchor reader reread file. A4 support should carry captured selected bytes/media from existing reader, not enable path-only fallback by removing guards. Sources: `crates/vibe-doc/src/citations/sources.rs; citations.rs; pages.rs; agent.rs; citations/anchor.rs`. |
| progress/tree/check host adapters | honest explicit package refusals | Progress/in-place directives and WAL/boot authored checks guard package source policy; project activation must integrate these before allowing unsupported host policy to fall through. ExplicitEmpty suppresses authored paths in boot checker; generated lane remains separate. Sources: `crates/vibe-cli/src/commands/progress/grounding.rs; tree/build/in_place.rs; crates/vibe-check/src/checks/boot_directory.rs; wal_wellformed.rs`. |
| show effective | unclassified representation projection | Generated STATIC shown verbatim correctly. Authored/lock files_written paths call load_spec_text by suffix without descriptor-selected representation: XML-data .xml may be parsed as Vibe XML. This is display behavior, not compiler execution; classify and retain ordinary bytes or report unavailable encoding explicitly. Sources: `crates/vibe-cli/src/commands/show/effective.rs`. |
| init package/defaults/docs | code already supports; docs stale | args.link threads to scaffold; help says normal; existing static/dynamic/default test checks Normal. Root comments still packages/. Current EN/RU tutorials at v1.0.0 still claim --link ignored; original v1.1.0 paths do not exist on main. Update actual canonical authored tutorial slots after behavior tests, do not create a phantom version. Sources: `crates/vibe-cli/src/commands/init/package.rs; cli/init.rs; tests/cli_init_kinds.rs; vibe.toml`. |

## Loading and literal-data boundary

SourceLoading is parsed and retained. In inspected production sources, loading() is used by the migration source recipe but not by these semantic consumers. Interpretation checks already keep raw/CommonMark out of native lookup/boot; that is real protection. Do not turn the lack of a loading accessor call into a claim of current directive execution. A4 must define and test when an explicit user/skill/resource request authorizes reading ordinary data, retain raw bytes and provenance, and ensure text such as #use/#embed/@fact remains literal rather than entering compiler inputs. Metadata-only assertion is insufficient.

## Foundation-first slices and discriminators

1. Selected package query/read bridge: preserve lock-selected identity plus independently selected input format, use A3 preparation and recorded output mapping, feed mapped native bytes to the existing fact parser without re-inferring suffix. Test actual install→requirements query in Mixed/Markdown/Xml, arbitrary suffix native source, raw same-stem XML/CommonMark/opaque, absent/mismatched lock hash, changed manifest/record/output, and truthful unavailable/invalid states. Keep observed output digest distinct from original source SHA.
2. Host sources grammar and exact cited sync: shared descriptor selector, presence semantics and policy checkpoint first; host observer/resolver then sync. Test exact native citation status and void equivalence, raw/ambiguous/missing/directory citation refusals before any registry mutation, nofollow path swaps and explicit-empty. Unrelated invalid source must not poison cited-only sync. Adding grammar while other host consumers silently ignore it is incomplete.
3. Bound check adapter can run independently once source preparation contract is fixed: install declared package, check clean graph, corrupt INDEX and get meaningful finding/refusal; explicit input-format mismatch must not become silent clean. Reuse A3 and existing fingerprint owner.
4. Documentation/agent and progress/tree/show adapters: replace explicit-refusal guards only with actual bound read/projection; preserve captured bytes for citations and avoid path-only reread. Distinguish authored native, ordinary raw and generated tape. Test one mixed package across each selected entry point; same-stem ordinary data must not false-collide or mint fact/boot meaning.
5. Skill resource join: retain existing explicit raw projection and exact-byte snapshots, validate descriptor consistency at the existing selected-source owner without replacing foreign content with parsed Vibe text. Test explicit adapter + embedded resources, directive-looking literals, target collision and missing resource; absence of any boot effect is part of the discriminator.
6. End-to-end matrix and documentation: canonical native projection/status/void must equal equivalent legacy fixture while raw resources stay exact across read/query/display/skill/boot boundaries. Re-run focused init static/dynamic/default/help fixture; correct root comment and actual v1.0.0 EN/RU tutorial source. Do not port the prototype entire module tree or its historical version paths.

## Original obligations (verbatim)

### steps

1. Pass one descriptor view through facts/sync, lookup, materialization, compiler/boot and skill resource preparation.
2. Require explicit loading for ordinary resources; literal embedded data cannot become managed compiler directives.
3. Build mixed-source fixtures that pass each consumer and record classification/provenance.
4. Honor vibe init package --link static/dynamic rather than hard-coding dynamic; reconcile the documented --format default and scaffold output.
5. Correct the root vibe.toml in-tree package comment to vibevm/vibepacks and refresh affected tutorial/reference examples after behavior is proven.

### positive_cases

1. Declared Vibe source retains existing facts and canonical projection.
2. Foreign bridge resources remain source-exact across all consumers.
3. vibe init package --kind flow --link static writes link = "static"; default/help and generated format agree.

### negative_cases

1. Any suffix-only fallback or ordinary same-stem false collision fails the matrix.
2. A resource under spec root does not enter boot automatically.

### checks

1. SELECT BEFORE DISPATCH: vibe-facts target/cases are TO BIND from source/metadata to affected positive/negative obligations; choose --lib, --test <name>, --bin <name>, or affected --doc before compiling.
2. RUN BOUND CHECKS: after binding, use cargo test -p vibe-facts <target> <case> -- --exact for lib/test/bin, or a reviewed small family without --exact; bind supported rustdoc selection separately. Verify intended nonzero cases in output; use target-specific --list only when needed to confirm selection, and reuse its valid inventory.
3. SELECT BEFORE DISPATCH: vibe-spec target/cases are TO BIND from source/metadata to affected positive/negative obligations; choose --lib, --test <name>, --bin <name>, or affected --doc before compiling.
4. RUN BOUND CHECKS: after binding, use cargo test -p vibe-spec <target> <case> -- --exact for lib/test/bin, or a reviewed small family without --exact; bind supported rustdoc selection separately. Verify intended nonzero cases in output; use target-specific --list only when needed to confirm selection, and reuse its valid inventory.
5. SELECT BEFORE DISPATCH: vibe-workspace target/cases are TO BIND from source/metadata to affected positive/negative obligations; choose --lib, --test <name>, --bin <name>, or affected --doc before compiling.
6. RUN BOUND CHECKS: after binding, use cargo test -p vibe-workspace <target> <case> -- --exact for lib/test/bin, or a reviewed small family without --exact; bind supported rustdoc selection separately. Verify intended nonzero cases in output; use target-specific --list only when needed to confirm selection, and reuse its valid inventory.
7. SELECT BEFORE DISPATCH: vibe-agent-projection target/cases are TO BIND from source/metadata to affected positive/negative obligations; choose --lib, --test <name>, --bin <name>, or affected --doc before compiling.
8. RUN BOUND CHECKS: after binding, use cargo test -p vibe-agent-projection <target> <case> -- --exact for lib/test/bin, or a reviewed small family without --exact; bind supported rustdoc selection separately. Verify intended nonzero cases in output; use target-specific --list only when needed to confirm selection, and reuse its valid inventory.
9. cargo fmt --all -- --check
10. git diff --check

### acceptance

1. Consumer inventory has no unclassified semantic path.
2. Existing equivalent Vibe fixtures preserve canonical output and status/void meaning.

### notes

1. Internal children execute sequentially; cross-parent prerequisites remain exactly the root campaign graph.
2. Permanent destination: vibevm/vibespecs/common/PROP-072-boot-observability.xml (PROPOSED file until its source-contract task creates it). Create/update the governing permanent contract before implementation assertions cite its real anchors; campaign retirement/promotion is root-owned.
3. TO CREATE: m_06_a_4_cases test family for the cases below. This is not an existing test. Bind the owning target and intended regression cases before dispatch; confirm created names and nonzero exact selection before running. Do not compile unrelated targets for discovery.
4. Only listed file/subtree perimeters may change; new implementation/test/schema paths within them are PROPOSED, not claims of existing files. Reuse prerequisite-owned modules where present and stop for a packet correction if an essential edit falls outside the perimeter.
5. Registration perimeter includes existing crate manifests and module owners only for required exports/dependencies; do not change unrelated declarations.
6. For installed descriptor packages, facts adopt/report must receive the independently selected lock content hash and compare it with the validated slot record before consuming declared Mixed originals. Never derive expected identity from the slot; transformed slots lacking an original-to-output recipe refuse. Legacy host and undeclared sources remain explicit until their own consumer slice.
7. The requirements query has an independent lock content hash but still calls the suffix scanner; route selected Mixed package bytes through the same declared classifier with explicit incomplete/refusal. The authored 1.1.0 English/Russian Markdown/XML tutorial must describe actual --link and scaffold-vs-manifest format defaults before B-211 closes.
8. For opt-in host classification, add validated [[project.sources]] using the shared package selector law. Host sync must capture only exact cited declared files through nofollow reads and refuse raw, ambiguous, directory or unavailable citations before registry writes; legacy [project] without rows remains an explicit compatibility route.

Safe stop: Do not open M-01 consumers until the classification matrix is complete.

## Limits

This audit does not claim tests were run or that a suspicious source path is exploitable. Facts original observation and skill raw copy already provide useful behavior; an honest unsupported guard is not a suffix fallback. Main query lacks installed read binding by source inspection; host sync remains legacy because opt-in grammar is absent. Accepted A3 scope is not reopened. Root must select concrete file perimeters per slice; likely existing requirements sources/query/context and tests, facts observation adapters, core project/selector plus policy, workspace public installed-read seam, actual CLI/MCP call edges, then the named consumers. No new global world, schema, arbitrary quotas or historical-output authentication is required.
