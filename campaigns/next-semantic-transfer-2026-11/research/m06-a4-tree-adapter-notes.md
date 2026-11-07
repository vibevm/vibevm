# A4 tree selected-native directive adapter — packet notes

Read-only exact preparation, 2026-11-07. No code/tests/Cargo/Git mutation. One next slice only; active level-zero writer is untouched. Full A4 and counts12/31/31/39 unchanged.

## Actual entry/candidate owner

`commands/tree/mod.rs::run` resolves root, loads TreeSettings UI preferences and calls `build::build_tree(&root)`. UI preferences are not install representation configuration. `build_tree` reads lock+manifest and committed selected STATIC/INDEX; it must keep deriving load type from those artifacts rather than regenerate a fresh graph.

`build.rs` creates boot_file_set from HOST_BOOT_FILES (00-core.md/90-user.md), STATIC contribution source paths and INDEX entry paths, then calls `in_place::collect_in_place(root, files, lock)`. This is the exact selected file universe. Do not replace it with capture_host_inventory or full capture_authored_boot: those would include unrelated/missing declared sources that this committed graph did not select.

## Minimal concrete route

Keep `build_tree(root)` as an internal/public-in-module compatibility wrapper calling a new private common implementation with no independent format. Add a bound sibling taking explicit selected SpecFormat for the actual CLI. Legacy inputs continue to work under the old wrapper; declared installed reads lacking format refuse, not guess. Host original reads require no materialized target.

At `tree::run`, after resolving root, select format through the existing `commands/reinstall::effective_rederive_format` (root manifest + UserConfig.install.spec_format via install::resolve_spec_format). Do not infer from STATIC basename or slot record and do not confuse TreeSettings preferences with operator install defaults. A second manifest read can be avoided with a small prepared manifest argument if convenient, but do not introduce workspace-wide session machinery for it. Committed artifact selection remains unchanged even when the current selected format mismatches installed input; that mismatch is a source-binding refusal.

Thread Option<SpecFormat> only to collect_in_place/shared read helper. For each distinct selected owner, prepare once and retain the observation until model completion:

### Installed selected owner

Build owner mapping from the full LockedPackage rows, respecting in-place spelling through existing slot path helpers rather than assuming every slot is versioned. Open the caller workspace once and descend selected slots no-follow. Construct InstalledSourceExpectation from lock kind/group/name/version/content_hash plus supplied format; call existing `vibedeps::prepare_declared_query_read`. Retain the complete selected package map for FileResolver and attach each returned binding. No slot-derived expectation, no record parser copy.

The boot_file_set path is a physical output. Resolve its exact row from PreparedInstalledSource.outputs by output_path; use row.source_path for `read_installed_source`, row.media/interpretation for projection and original identity. A converted suffix is not the original native address. Raw/unclassified/ordinary rows yield zero managed directives, never a parser call. A selected native VibeSpec row enters project_declared_native→Directives::parse; VibeDoc is not an instruction source. Selected binding/read ambiguity or drift must be visible error, not swallowed as empty output. Generated owner-relative STATIC/INDEX is skipped as today. Legacy unrecorded/read behavior remains explicitly separate; declared in-place retains precise unsupported binding until its actual owner exists.

### Original host owner, including ungrouped

Use only caller-selected host/registered workspace-node metadata, not an arbitrary nested vibe.toml found while walking a declared outer scope. For a member path, resolve the longest registered node-root prefix from actual Workspace nodes; nearest random manifest must not become new authority. No full source inventory is needed.

Observe that node's held policy, compare selected manifest where available, and select the EXACT relative file through `select_source_rows`/source_rows before reading. Existing public `read_source_observation(relative)` supplies bytes/identity with held/no-follow input semantics. Retain policy plus observation/path and recheck before returning the tree (recapture relevant exact bytes/identity; policy checkpoint recheck alone is not retained source content). Select only native VibeSpec for directive scanning; ordinary rows are skipped. This is a requested read of paths already named by committed boot artifacts, so native explicit may be read if explicitly present in that selected path set; it does not change automatic native+scan enrollment.

An ungrouped project still has path-based boot and may contain remote native directives. Do not fabricate a group, PackageMeta or spec:// self address just to select/read bytes. The selector's identity label can be an existing diagnostic root label, as accepted own-boot capture already documents; never export it as native address. If a narrow exact-file captured helper is needed, factor only this observation in the existing source-policy owner under root coordination. Do not alter the addressed host reader's real-coordinate contract.

## Output/compatibility

Preserve InPlaceSpec Carrier/address/file/line/resolved fields and existing schema. Keep file as actual selected boot path; directive line is the canonical projection line under existing XML diagnostic convention, not falsely original XML byte line. Generated tape decompilation, static/dynamic attribution, package order, conditions and root edges are unchanged. No new URI, manifest field or tree schema.

The existing `let Ok(load_spec_text) else continue` remains only the qualified legacy branch. Declared malformed/missing native or unavailable selection must not silently look like zero directives. Empty policy skips authored inputs. Known ordinary same-stem text does not create false pair collisions. Recheck lock/selected policy/binding or retained exact host observations before final model return; no lease or mutation.

## Bounded perimeter

- `crates/vibe-cli/src/commands/tree/mod.rs`: actual format composition/call.
- `.../tree/build.rs`: bound sibling/common implementation and collector threading, optional real registered-node owner metadata.
- `.../tree/build/in_place.rs` plus one bounded read helper child if needed: exact owner/binding/native capture.
- `.../tree/build/tests.rs`: deliberately update current explicit-policy refusal expectations to actual supported behavior, keep malformed/unknown owner controls.
- NEW focused CLI target for actual install→tree, plus existing `tests/tree_fixture.rs`, `tree_nested_attribution.rs`, `tree_json.rs` only as appropriate compatibility gates.

Do not edit docs/level-zero, progress, show, seal, graph-check algorithm or underlying selector rules. Any required lower accessor extension needs root coordination before edits.

## Nonzero discriminators

1. Actual install grouped and ungrouped host own boot with native XML arbitrary suffix carrying a real #use/#embed and ordinary same-stem containing forged directives. `vibe --json tree --path <root>` must report the real reference once with correct selected file, omit ordinary references and retain own Dynamic linkage/INDEX direct read-kind semantics.
2. Actual declared parent package installed as Mixed/Markdown/Xml, original arbitrary-suffix source mapped to converted physical output. Tree directive extraction uses selected media and original row, independently selected format; no suffix fallback. Snapshot source/project files/directories around query.
3. Mismatching selected format, lock hash, record/output or removed exact native input refuses visibly; raw entry gives zero directives without parsing; unrelated invalid declared file not in boot_file_set does not poison tree.
4. Registered member source owner uses its own policy; malformed nested manifest inside an outer declared scope is not a new owner. Existing parent/member attribution oracle remains green.
5. Legacy `tree_fixture` exact JSON model and current schema checks retain behavior. Old build_tree wrapper still handles no-declaration callers and refuses declared installed input without independent format.

Command spelling is existing `vibe --json tree --path`. Proposed target command: `cargo test -p vibe-cli --test cli_declared_tree` after that test file exists; bind exact unit names before compiling. No runtime evidence is claimed by these notes.
