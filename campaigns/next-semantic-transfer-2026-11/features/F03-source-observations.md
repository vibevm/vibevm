# Safe-source byte observations — implementation report

Worktree `/fast/git/wt/source-observations`, branch `fix/source-observations`, base `8ed16f49e`. Permanent root-owned contract read: `PROP-035#URI-OBSERVED-SOURCE-READ` and `source-read-decision`; research input read from `campaigns/next-semantic-transfer-2026-11/research/safe-read-design-input.md`. No full boot/central plan read, no prototype writes, no commits, no maps/spec edits.

Status: bounded implementation complete and ready for root review. The implementation and targeted tests pass. Root owns conform/specmap, acceptance, integration and worktree retirement.

## Implemented behavior

`vibe-safefs` now has three specifically named read-only methods:

- `Project::read_source_bytes(relative) -> Result<Option<Vec<u8>>>`.
- `Project::read_source_bytes_in(&Pinned, relative)` retains the locator's exact parent through the read. Root explicitly approved this normal `_in` companion.
- `Project::source_file_present_in(&Pinned, name) -> Result<bool>` is a finite, content-free metadata probe needed before frontend admission. Root explicitly approved it: shared regular file=true, absent/directory=false, symlink/special/I/O failure=error. It grants no later byte guarantee.

The new byte method opens no-follow, accepts shared regular inodes, observes initial length from the held handle, converts that length to usize with checking, reserves memory using `try_reserve_exact`, reads exactly that extent and probes for growth. Its second pass compares fixed-size chunks against the single retained byte buffer. It checks closing length/mode and reopens the final name through the same pinned parent to compare object identity. On Unix the source open is nonblocking so a special-file substitution cannot turn the open into a FIFO wait before its regular-file check. No fixed source quota, COW, mutation, retry, cache or proof carrier was added.

Existing `StableFileSnapshot`, bounded/state/mutation read methods and their single-link rules were not changed. Existing `cap_options`, held directory traversal, file identity, Unix mode, read-chunk constant and test-only race hooks were reused; the new policy lives in its own module. The byte API returns only bytes because that is what current consumers need.

`FileResolver` point-source location now admits only an explicit caller root via canonicalization, once for each location/read epoch. From that held root, layout, derived dependency/version slots, document parents and candidate metadata are traversed capability-relatively without following descendant links. Selected package roots remain explicit caller authority, including unversioned in-place roots. Logical PathBuf values are preserved for overlay keys and diagnostics. Exact document lookup does not enumerate unrelated siblings; ID-prefix lookup uses the held directory enumeration. Legacy wildcard expansion itself remains unchanged and is deferred to the next atom.

The old authority/slot selection logic moved from `resolver.rs` into the focused `resolver/observation.rs` helper. A private `SourceLocation` groups the existing Project capability, selected parent/name and resolved locator for this one operation; it is not a public source-world authority/proof object. Existing candidate-choice/collision logic is shared by the new held locator and legacy glob lookup through `lookup::select_doc`; the wildcard language and lexical-first duplicate behavior were not revised here.

`FsSectionSource` builtin and admitted custom reads consume one selected buffer. Builtins use the existing pure `project_spec_text` and section parser; custom text remains raw UTF-8. Exact overlays borrow their held Arc bytes without a filesystem content read or fallback after malformed overlay bytes. Custom frontend admission still occurs in the unchanged compiler `AdmittedSource::load` before content reading.

Public supplied metadata is revalidated against named authority/path/format/stem. FsSectionSource-produced metadata also privately retains its original active suffix set; this preserves compound suffixes and detects a newly introduced alternate active form. Constructors/signatures remain unchanged. I searched all crate consumers of ResolvedSourceMetadata/source_metadata and relevant Eq/Debug patterns: the production compiler uses accessors and passes the locator onward; test doubles use constructors/accessors; no existing Eq/Debug consumer assumption was found. Derived Eq/Debug now includes the private active_formats context, which is an observable trait-level detail for external callers, not a new public constructor field.

Root explicitly confirmed the per-read interpretation: old metadata does **not** freeze a root alias across calls. If an explicitly caller-owned alias is retargeted between metadata lookup and the read, the new read admits its current root. Inside that read, root is pinned once and descendants stay no-follow. This is documented on ResolvedSourceMetadata and pinned by a dedicated native Unix compatibility test. No optional root-identity precondition or query-world witness was added.

## Files and perimeter

Modified tracked files:

- `crates/vibe-safefs/src/file.rs` — register new module only.
- `crates/vibe-spec/Cargo.toml` — existing workspace SafeFS production dependency and test-only inject-failures feature.
- `Cargo.lock` — Cargo's single derived dependency edge `vibe-spec -> vibe-safefs`, explicitly authorized by root; no package/version updates.
- `crates/vibe-spec/src/resolver.rs` — point locator delegation and requirement-citing SourceObservation error; old authority/slot code moved to held helper.
- `crates/vibe-spec/src/resolver/lookup.rs` — share candidate selection with held locator; retain legacy pattern lookup path.
- `crates/vibe-spec/src/embed.rs` — same-buffer read delegation, private active suffix context and locator guarantee comment.
- `crates/vibe-lifecycle/src/agent/tests/resolver.rs` — actual prompt same-byte replacement regression.
- `crates/vibe-spec/src/compiler/transform/dependency_dag_fence_tests.rs` — root-authorized exact dependency allowlist consequence, including strict SafeFS feature shapes and no reverse edge.

New files:

- `crates/vibe-safefs/src/file/source_read.rs`
- `crates/vibe-safefs/src/file/source_read_tests.rs`
- `crates/vibe-spec/src/resolver/observation.rs`
- `crates/vibe-spec/src/embed/read.rs`
- `crates/vibe-spec/src/embed/source_reads.rs`
- `crates/vibe-workspace/tests/source_reads.rs`

No other manifests, source owners, specs, maps or shared logs were edited. Largest changed production file is embed.rs, 572 lines, below current 600-line policy.

## Observed RED and corrections

The initial actual-consumer regressions ran before integration:

`/usr/bin/time -f 'ELAPSED_SECONDS=%e EXIT=%x' cargo test -p vibe-spec --lib embed::source_reads::`

Result: exit 101, 4 selected tests: 1 valid explicit-root-alias/shared-file control passed, 3 failed, 67.66 seconds. Failures demonstrated actual `OUTSIDE_SENTINEL` content returned through redirected document/parent/layout paths and through a supplied foreign metadata locator. These were behavioral failures, not expected-parser mocks.

The same four-case family passed after integration: exit 0, 4/4, 14.21 seconds.

Two mistakes in later test development were corrected explicitly:

- I called `ResolvedSource::text()` as a method although this code has a field. This compilation failed with E0599, exit 101, 2.68 seconds; no tests ran. The test now inspects its accessible field.
- I assumed `tempfile::tempdir_in(".")` returns a relative PathBuf. It returned an absolute path. The expanded family ran 9 cases, 8 passed and that fixture assertion failed, exit 101, 6.87 seconds. The corrected fixture derives an explicit `./<owned-temp-name>` spelling without changing cwd or environment.

The subsequent expanded family passed 10/10, exit 0, 4.88 seconds. Optional-version membership and root-alias epoch controls were added afterward and tested in the final selections below.

## Final scoped verification

All commands used this worktree's own target and normal environment; no external dependency/toolchain installs or global configuration changes. Cargo emits the pre-existing `vibe-cli/Cargo.toml: unused manifest key: build` warning. Durations are `/usr/bin/time` wall seconds.

Every cargo command below was prefixed by `/usr/bin/time -f 'ELAPSED_SECONDS=%e EXIT=%x'`.

| Command | Observed result | Seconds |
| --- | --- | ---: |
| `cargo test -p vibe-spec --lib resolver::` | exit 0; 52 passed, 0 failed; includes all F01 and existing pattern/custom-form compatibility | 5.70 |
| `cargo test -p vibe-spec --lib embed::` | exit 0; 37 passed, 0 failed; includes 11 new source-read consumer cases at that point | 0.21 |
| `cargo test -p vibe-spec --lib compiler::pass_tier::frontend_` | exit 0; 8 passed, 0 failed; admission-before-read, inactive formats and real pipeline ordering preserved | 0.21 |
| `cargo test -p vibe-safefs --lib file::bounded_tests::` | exit 0; 17 passed, 0 failed; old caps, single-link/hardlink refusals and identity tests preserved | 1.06 |
| `cargo test -p vibe-safefs --lib file::source_read::tests::` | exit 0; 8 passed, 0 failed | 0.20 |
| `cargo test -p vibe-safefs --doc file::source_read` | exit 0; 3 doctests passed, 0 failed, 41 filtered out | 2.72 |
| `cargo test -p vibe-workspace --test source_reads` | exit 0; 2 passed, 0 failed | 35.70 |
| `cargo test -p vibe-lifecycle --lib agent::tests::resolver::` | exit 0; 12 passed, 0 failed | 52.53 |
| `cargo clippy -p vibe-safefs -p vibe-spec --lib -- -D warnings` | exit 0 | 33.69 |
| `cargo test -p vibe-spec --lib embed::source_reads::metadata_locator_does_not_freeze_explicit_root_alias_across_reads -- --exact` | exit 0; 1 passed, 0 failed, 987 filtered out; final requested compatibility addition | 4.53 |

The final alias test and associated doc comment were added after the 37-case embed selection; only that new exact test needed execution. There are 140 passing selected test/doctest cases across the disjoint final selections above, not a claim of a full package/workspace run. New behavioral cases total 23 (12 spec consumers, 8 SafeFS, 2 workspace, 1 lifecycle), plus 3 API doctests. The earlier first SafeFS family run also passed 8/8, exit 0, 13.40 seconds.

Final formatting command:

`/usr/bin/time -f 'ELAPSED_SECONDS=%e EXIT=%x' rustfmt --check --edition 2024 --config skip_children=true crates/vibe-safefs/src/file.rs crates/vibe-safefs/src/file/source_read.rs crates/vibe-safefs/src/file/source_read_tests.rs crates/vibe-spec/src/resolver.rs crates/vibe-spec/src/resolver/lookup.rs crates/vibe-spec/src/resolver/observation.rs crates/vibe-spec/src/embed.rs crates/vibe-spec/src/embed/read.rs crates/vibe-spec/src/embed/source_reads.rs crates/vibe-workspace/tests/source_reads.rs crates/vibe-lifecycle/src/agent/tests/resolver.rs`

Result: exit 0, 0.07 seconds. The same file set was formatted using rustfmt before checks.

`/usr/bin/time -f 'ELAPSED_SECONDS=%e EXIT=%x' git diff --check` returned exit 0, 0.01 seconds.

## Important evidence and limits

- The workspace test invokes actual `materialise_with(..., CopyMode::Hardlink, ...)`, then asserts equal device/inode and link count >=2 before reading MD/XML through FsSectionSource. Copy fallback cannot satisfy this test. After read it checks the same inode/link count, unchanged original bytes, slot record and fixture path inventory: no COW or hidden state creation.
- The second workspace case uses actual `materialise_in_place` to place an unversioned checkout and reads it through the selected map. It does not claim a live Git/provider operation; the fixture owns a minimal checkout directory.
- Actual source and prompt cases exercise same-byte rename replacement through existing SafeFS test-only hooks; the native cases ran on Linux. The separate forced identity test is a simulated comparison, not native Windows evidence.
- Shared-inode same-length rewriting between passes, growth/truncation, mode changes, failed second reads/comparisons and impossible allocation sizes are covered. Allocation failure uses a deterministic impossible size, not an attempted OOM.
- The retained-parent test deliberately renames/replaces the textual parent path during reading, and the read returns the originally held source, never redirected outside bytes. This proves capability anchoring. It does not prove all original absolute ancestor names remain unchanged.
- Exact overlay tests arm the content-read hook and prove it does not run; malformed overlay bytes do not fall back. Compound custom suffix and newly introduced alternate active-form collision have explicit tests.
- Public metadata is a locator, not a content or cross-call root-identity witness. Same logical locator can observe current content in a later admitted epoch. No whole-closure atomic snapshot, hostile ABA immunity, exclusive source ownership, source authentication or portable case/NFC-alias proof is claimed.
- Legacy glob enumeration/error/duplicate semantics remain for the next atom. No claim is made that every wildcard expansion metadata probe is now capability-pinned.
- Ordinary regular files occupying current-layout components preserve the legacy layout fallback; missing components do likewise. Symlink/reparse/special entries still refuse through the no-follow metadata boundary.
- No native Windows/macOS execution, full package/workspace suites, conform or specmap work was performed here. Root owns those gates and acceptance. No commits or cleanup were attempted.

## Root review correction — ordinary files must preserve legacy fallback

Root identified a compatibility error in my initial layout traversal: it treated an ordinary file named `vibevm` (or `vibevm/vibespecs`) as a malformed directory even when valid legacy `spec/DOC.md` existed. The old selector deliberately chose current layout only when it was a directory; there was no contract reason to reject these valid package files.

Correction is limited to `resolver/observation.rs` and its consumer tests. Current-layout components now use the already approved content-free `source_file_present_in`: a regular file means this layout is unavailable, so legacy fallback applies. Directory traversal remains no-follow; symlink/reparse/special or I/O error still refuses. No new API, canonicalization, wildcard or other source behavior was added.

New test `regular_files_in_current_layout_preserve_legacy_spec_fallback` covers both blocker positions and asserts their ordinary bytes remain unchanged. Before the correction, its exact run failed with `Not a directory`, exit 101, 1 selected / 1 failed, 3.93 seconds:

`/usr/bin/time -f 'ELAPSED_SECONDS=%e EXIT=%x' cargo test -p vibe-spec --lib embed::source_reads::regular_files_in_current_layout_preserve_legacy_spec_fallback -- --exact`

After the correction:

- `/usr/bin/time -f 'ELAPSED_SECONDS=%e EXIT=%x' cargo test -p vibe-spec --lib embed::source_reads::` — exit 0, 13 passed / 0 failed, 4.27 seconds. Includes existing layout-symlink refusal and the new ordinary-file controls.
- `/usr/bin/time -f 'ELAPSED_SECONDS=%e EXIT=%x' cargo test -p vibe-spec --lib resolver::` — exit 0, 52 passed / 0 failed, 0.21 seconds.
- `/usr/bin/time -f 'ELAPSED_SECONDS=%e EXIT=%x' rustfmt --check --edition 2024 --config skip_children=true crates/vibe-spec/src/resolver/observation.rs crates/vibe-spec/src/embed/source_reads.rs` — exit 0, 0.04 seconds.
- `/usr/bin/time -f 'ELAPSED_SECONDS=%e EXIT=%x' git diff --check` — exit 0, 0.01 seconds.

No whole-suite repetition was performed. Initial verification above remains historical evidence; the correction adds one distinct passing test, bringing cumulative distinct selected tests/doctests to 141 (24 new behavioral cases plus 3 new API doctests). Final hashes below supersede the initial review hashes. No source process remains running.

## Root review correction — exact dependency DAG fence

Root identified the existing architectural fence that freezes vibe-spec runtime/dev dependency sets. This test was outside my initial selected test list. Root explicitly extended the write perimeter to `crates/vibe-spec/src/compiler/transform/dependency_dag_fence_tests.rs` only; no other reviewed code changed in this correction.

Recorded RED on the old exact test:

`/usr/bin/time -f 'ELAPSED_SECONDS=%e EXIT=%x' cargo test -p vibe-spec --lib compiler::transform::dependency_dag_fence_tests::the_dependency_dag_gains_exactly_the_two_intended_lower_edges -- --exact`

Result: exit 101, 1 selected / 1 failed, 0.26 seconds. The mismatch was exactly the approved runtime `vibe-safefs` edge missing from the frozen set.

Updated test name is `the_dependency_dag_keeps_only_approved_runtime_and_test_edges`. Runtime/dev package-name sets remain exact; SafeFS entry shapes are compared as parsed TOML against exactly `workspace = true` for production and `workspace = true, features = ['inject-failures']` for tests. This is not a general feature/dependency allowance. Existing core/registry boundaries remain, SafeFS is added to the no-reverse-edge loop, and that loop also examines platform-specific dependency sections. The rationale and verification edge cite permanent `PROP-035#URI-OBSERVED-SOURCE-READ`.

Recorded GREEN:

`/usr/bin/time -f 'ELAPSED_SECONDS=%e EXIT=%x' cargo test -p vibe-spec --lib compiler::transform::dependency_dag_fence_tests::the_dependency_dag_keeps_only_approved_runtime_and_test_edges -- --exact`

Result: exit 0, 1 selected / 1 passed, 6.70 seconds.

`/usr/bin/time -f 'ELAPSED_SECONDS=%e EXIT=%x' rustfmt --check --edition 2024 --config skip_children=true crates/vibe-spec/src/compiler/transform/dependency_dag_fence_tests.rs` returned exit 0, 0.04 seconds. `git diff --check` returned exit 0, 0.01 seconds. No blanket exemption, whole-suite run or other source edit was made.

This adds one distinct passing architecture fence to the previous 141 selected cases: cumulative distinct selected tests/doctests now 142. All 14 current changed-file hashes below supersede the previous manifests. No test/build process remains running.

## Final source SHA-256

```text
17c6e9e04aec207e0a1925652eda3c1a98bf7f6057dd198244190a32eeeb1f89  Cargo.lock
4af80697b1e381935e0b2a68adaee7cfbbc813c9574eeed9e788d4a10f7eee67  crates/vibe-lifecycle/src/agent/tests/resolver.rs
9746804e3753076315033a4d7f6d87172a9dc586ce67bc5ab9ee3bedcec18c38  crates/vibe-safefs/src/file.rs
ea7ebdc001ff0adf1306921a5f486a7eec8deec5e9e2025ad07af6a7580c507b  crates/vibe-safefs/src/file/source_read.rs
6691b90b8026497523925c19e2698d9a32679d8ecf6d400f66cbce9483074a1b  crates/vibe-safefs/src/file/source_read_tests.rs
4f9839937a8eb5824eb7081d432249e98119556051c52af2f8a41c43ddd0da37  crates/vibe-spec/Cargo.toml
e7bd6a995db5fda3241e86fc0f169cf080ca3959bd3249b77fa14f357f61933c  crates/vibe-spec/src/compiler/transform/dependency_dag_fence_tests.rs
f76f3ebe0e18ade1c9eb1cf6abc60e344c301ad9034832f1c6142f161836546c  crates/vibe-spec/src/embed.rs
8e9a4bb733be25cb79a179f0827a9059104b4c50437e4c0afd3ebfa2cdcf2e97  crates/vibe-spec/src/embed/read.rs
46d34fd5132d3b3d7b3bf435f655338f77bff76aeb5b46f88e6e634621e21ebd  crates/vibe-spec/src/embed/source_reads.rs
31dee9485dfcaec928937914d8a6178b62a74468778cfa4fc9c5d4ec2deab9ed  crates/vibe-spec/src/resolver.rs
bd3a04af889fd2da74a6fb78c7b7203374ed4c032b429d894a86736f92cb9fc9  crates/vibe-spec/src/resolver/lookup.rs
9290488cd53c2fcd397aeded3f4c4f57fb8fab6aa378211a2ffb8e542e9bd293  crates/vibe-spec/src/resolver/observation.rs
ccef6ed223b9176c948664e8f213019b447582e572c962a0f2bebb30ac71131e  crates/vibe-workspace/tests/source_reads.rs
```

## Central acceptance review

The complete production patch and relevant adversarial/consumer fixtures were reviewed. Exact final source hashes were checked before integration. Root scoped conform passed for safefs/spec/workspace/lifecycle with zero new findings. Root reran the final embed family:39passed. Two review corrections preserve ordinary-file legacy layout fallback and the exact dependency DAG fence; neither weakens the existing single-link mutation/state contract. No native Windows/macOS or whole-product pass is inferred. Final map/integration is recorded in LOG.md.
