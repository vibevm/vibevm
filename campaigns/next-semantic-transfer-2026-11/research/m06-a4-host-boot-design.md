# A4 declared own-boot integration decision

Read-only design, 2026-11-07. No source/spec edits, tests, Cargo or Git mutation. Implementation waits for root authorization after host-query acceptance. Full A4 remains open.

## Decisive actual behavior

`boot::compute_effective_boot` sets BOTH inherited foundation and own AuthoredBoot entries to `LinkType::Dynamic`, `PackageFormat::Simple`, `BootProvenance::Node` (boot.rs:310–343). Nearby comments saying static are stale. `prepare_index_body` emits dynamic-link entries in INDEX, with `kind="static"` when unconditional and `kind="dynamic"` when conditional; INDEX read timing is not compiler static linkage. Consequently ordinary successful own-boot enrollment writes INDEX references and preserves original files. It does not compile these entries into STATIC. Do not change linkage or switch to Normal merely to produce a static test result.

Current blockers: source_admission rejects every nonempty project.sources before effects; node_own_boot returns the same unsupported error rather than classifying immediate boot files. Broad host inventory now exists but is the wrong input here: it requests all declared source scopes and requires a coordinate. Calling it then filtering would let unrelated malformed/missing explicit files poison boot.

## Smallest reusable source seam

Add a narrowly scoped original-resource capture operation to the existing source_policy owner: caller supplies the selected root/held manifest and immediate authored boot scope. It selects through existing source_rows/select_source_rows, observes original file bytes/media/interpretation/loading and relevant parent/membership identities, and can recheck those observations. It does not require a spec coordinate because no native address is being minted. Retain actual manifest identity and compare it with workspace's supplied Manifest; never invent a group, PackageMeta, installed record/hash or global source world.

This may factor internal helpers from inventory/host capture (checked child traversal, selected resource observation, policy/root/materialization cue checks), but must NOT run whole-host capture_host_inventory. Do not weaken coordinate checks in addressed capture_cited_native/capture_addressed_native; add the non-addressed boot operation as its own caller request. Shared regular inodes remain allowed. Caller-root alias rules and installed-root refusal cues remain the accepted owners.

Finite candidate set:

1. Immediate children of layout::current_boot_dir(), plus exact declared file rows targeting an immediate child (so an explicitly promised native+scan missing file cannot silently disappear).
2. Exclude generated STATIC.md/STATIC.xml/INDEX and transaction-owned controls using existing ownership law before authored enrollment. Directory defaults may apply to these children, but no recursive boot subtree scan.
3. Apply shared descriptor precedence. Auto-enroll only VibeSpec + loading=scan; ordinary/unclassified and native explicit rows are not auto-enrolled. Raw same-stem neighbor is not a native collision. A requested scoped native candidate with ambiguity, unsupported media/vocabulary, missing/unsafe file refuses. Exact native addresses outside boot are unaffected and remain explicitly readable.
4. Compare actual selected native original canonical claims for ambiguity; preserve path identity law without inventing alternate output filenames. Capture only selected native bytes, not every declared resource. Parse/project only that selected native set if needed by the actual consumer/preflight; never parse outside scope merely because its declaration exists.

Do not impose a blanket group requirement. An ungrouped host can select and carry original native Simple boot bytes; address-bearing references still need the incumbent real coordinate. `DocumentProvider::HostUngrouped(name)` already expresses that host, and HostVirtualWorkspace remains distinct. Grouped node identity comes from its actual workspace manifest. Existing compiler B-031 self-coordinate remains workspace-root-owned; do not silently retarget it for member nodes in this change.

## Retained workspace carrier

Introduce a private prepared node-boot result beside node_own_boot: ordinary `Vec<AuthoredBoot>` entries plus the scoped policy/captured input observations needed for recheck. Keep public AuthoredBoot and serialized INDEX shape unchanged. Store original workspace-relative paths/category/origin; retain captured media/bytes in the private carrier, not a second independently rediscovered path map.

Use the same preparation helper for early source_admission preflight and authoritative regeneration preparation. Early preflight evaluates all known selected own-boot inputs before metadata/package hooks. At regeneration, prepare root/member boot once before emit_package_units or any node output, retain root foundation entries from that preparation, compose node graphs through the existing pure function, and recheck relevant selected observations immediately before each node INDEX/redirect publication. The current implementation discovers node sources after package-unit emission; merely removing the unsupported guard would allow predictable late source failure after effects.

Early preflight and later authoritative prepare are distinct finite observations, not a promise of one immutable lifecycle-wide snapshot. A source change between them must be revalidated/refused; do not silently consume fresh suffix semantics. Actual compile input uses the captured original media/bytes when a static consumer really reaches a node input; ordinary own boot remains dynamic and is not falsely counted as a compiler-input test. Zero-static lanes still need recheck before INDEX publication: placing a guard only in compile_static_artifact_with_plans would miss them because it returns early when entries are empty.

Ordinary and managed-native writers share the publication requirement. Replay stores node candidate/index state, so any retained own-boot reference carried into replay must keep/revalidate its scoped selection before publication; do not invent overlay bytes for dynamic own inputs. Analysis/backend paths use the same prepared discovery to select honest identities; they must not regain ambient suffix lookup. Existing public path-only/unbound readers keep their precise guards until an actual bound operation supplies the capture.

Categories/order: preserve exact incumbent `00-core.md` Foundation and `90-user.md` UserOverride mapping and lexicographic original path order. Arbitrary-suffix native can be ordinary NodeOwn without changing filename category conventions. Root/member inherited foundation behavior and existing INDEX output target remain unchanged.

## Implementation perimeter

- Narrow source_policy scoped original-resource capture/recheck helper and focused tests, reusing held source APIs without whole declared inventory.
- workspace install/bootgen private prepared-node helper; source_admission validation; ordinary/native regenerate and their existing analysis/backend/replay callers as needed to retain the actual input.
- boot_artifacts input carrier only where a node contribution is actually compiled; ordinary/native publication boundary rechecks also for no-static lanes.
- Existing AuthoredBoot/compute_effective_boot remain pure data/order/link owners; no public format/schema change. Correct misleading nearby static comments if touched.
- Actual CLI install/regenerate and bounded library tests; PROP-072/FMT/log root-owned.

Do not remove the early refusal independently of the supported preparation route. This slice concerns automatic own-boot enrollment; remaining check/docs/show/skills/adoption/mixed matrix still have their own A4 closure.

## Decisive tests

1. Actual grouped project install/regenerate with immediate native+scan XML under arbitrary suffix, Markdown native, ordinary same-stem resource containing literal #use/#embed, native explicit-only file, unrelated malformed/missing explicit source outside boot. INDEX references exactly the scanned native boot files; original bytes remain exact; raw/explicit-only/outside files are absent. Dependency STATIC still follows selected SpecFormat.
2. A native explicit source outside boot remains readable through accepted exact host SectionSource but is not auto-enrolled. This proves loading selection without inventing a new resource command.
3. Ungrouped project with native+scan own boot succeeds and emits path-based INDEX content without synthesized group or spec address. Grouped addressable control remains separate; own format stays Simple and link stays Dynamic.
4. Root/member workspace preserves exact Foundation/UserOverride categories, ordering and inherited references. Empty declaration produces no own entries but dependency boot continues; absent policy keeps existing legacy output.
5. Selected later-node malformed/ambiguous/missing native+scan input fails early admission before earlier package hooks/output; unrelated outside-scope malformed native does not poison this boot operation. Add selected membership/media/manifest drift discriminator before actual INDEX publication, including zero-static case.
6. If the retained media path is consumed by a real static compiler call, test that existing composition route with captured node input without altering automatic own linkage. Do not claim normal INDEX enrollment as evidence of #embed expansion or static attribution.

No new output authentication, global census/CAS transaction or all-platform certification is requested.
