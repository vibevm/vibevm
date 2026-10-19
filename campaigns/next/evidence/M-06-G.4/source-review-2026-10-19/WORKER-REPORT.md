M-06-G.4 — implementation WIP and focused Linux evidence; native Windows OPEN

Scope and contract
Read the complete campaigns/next/tasks/M-06-G.json including M-06-G.4, the supplied full TaskContract, PROPOSED-CHECKS.json, and the three governing PROP-007/009/035 specifications. Existing working tree was clean. No Git mutations, delegation, installs, configuration/toolchain/dependency changes, CI actions, or writes outside the permitted source owners were performed. Temporary evidence is task-owned. No acceptance or verified-check attribution is granted by this report.

Changes
Reuse the existing prepared payload and slot reconciliation owners. Before reconciliation inspects/migrates/creates a slot or removes stale files, perform read-only symlink_metadata probes for the slot, selected destination paths, .vibe-slot.toml, and representative payload/record staging paths. Set both tempfile builders to eight random bytes so staging probes match the actual basename lengths. Retain the existing copy/hardlink/hash/sync/persist and ownership machinery.
Classify only native length error codes (Windows 206, Linux 36, macOS 63). Report exact failed path, stage, native error/code, and measured length excluding NUL (Windows UTF-16 code units; POSIX bytes). Preserve unrelated error handling. Later reconciliation length failures explicitly say state may be partial. The host error does not give its exact filesystem/API/component limit: diagnostics state that gap rather than inventing a numeric threshold or shortening filenames.
PROP-007 gains an additive spec/work law separating logical nesting from physical host pathname limits. Existing nesting, root discovery, one lockfile, separate authored/dependency trees and compiler routing laws remain intact; no independent law was replaced. No custom verbatim-path prefix or new no-follow mechanism was introduced without a native reproduction showing it necessary.

Production route and fixture boundary
Real non-test caller: crates/vibe-install/src/apply.rs calls vibe_workspace::install::apply_resolution_with_configuration_recovery for its non-native-provider branch (the native-provider branch shares materialise_resolution_with_spec_format). Standard input: Workspace::load plus ResolvedDep with parsed source Manifest, source content directory, mutable source flag and explicit fixture source identity. The new fixtures invoke that exact public configuration/recovery entrypoint, SpecFormat::Mixed, SlotIntegrity::Verify, no verifier, no lifecycle, trace/configuration/recovery None. This runs real slot materialization and boot regeneration, not just argument parsing or a direct diagnostic helper. The fixed fixture source_hash is an explicit synthetic resolved identity, not a registry-fetch integrity proof. Full CLI resolution/fetch, native-provider carriage and transformed representations were not driven here; their shared reconciliation exposure is a source trace, not additional executed proof.

Evidence
Before build, a privileged read-only active Cargo/rustc target scan found 0 target references and 0 permission gaps for the granted exclusive CARGO_TARGET_DIR. The preceding unprivileged process scan had permission gaps and inherited environment references and was not treated as sufficient. Used only the supplied existing toolchain/target, RUSTUP_AUTO_INSTALL=0, private TMPDIR=/tmp/vibevm-m06g4-yqr139oe, offline/locked Cargo, -j2, existing dev Cranelift backend. No LLVM override needed.
Selected BEFORE compilation: vibe-workspace --lib m_06_g_4_cases, implemented in the permitted vibedeps subtree and registered through lib.rs.
Executed command (exit 0):
env RUSTUP_AUTO_INSTALL=0 TMPDIR=/tmp/vibevm-m06g4-yqr139oe CARGO_TARGET_DIR=/fast/zap-managed/2026-10-14-m10b1-correction/cargo-target /home/olegchir/.cargo/bin/cargo test --offline --locked -j2 -p vibe-workspace --lib m_06_g_4_cases -- --nocapture
Observed: 3 passed, 0 failed, 0 ignored, 775 filtered out. Native Windows-only case is cfg-excluded on this Linux host, not passed or waived. Baseline dependency/fixture warnings retained below.
1. ordinary_short_root_and_owned_retry_keep_neighbors: successful short-root bytes/record; real collision error preserves prior record and foreign occupant; owner removes only its blocker; retry succeeds; foreign neighbor survives; ownership validator refuses whole-slot cleanup until fixture owner releases the foreign file, then owned cleanup and retry succeed.
2. long_root_spec_path_installs_on_this_host: actual combined output beyond legacy 260, including vibevm/vibedeps/org.example.path-fixture/1.1.0/vibevm/vibespecs and a long legal XML basename; exact output bytes, identity/footprint and temporary payload cleanup checked. This is Linux evidence only.
3. unsupported_path_diagnostic_is_not_nesting_or_false_success: native symlink_metadata on the actual 4,207-byte output returns OS error 36. The real install route refuses during payload destination preflight; existing stale payload, record bytes and foreign neighbor remain unchanged. Shortened-root retry installs the identical selected bytes and fixture identity. The raw output path and exact stage/error are retained below. The old unmodified tree was not separately rebuilt/tested; no baseline whole-install failure is claimed.
Final focused rustfmt check across six touched Rust files (skip_children=true) exit 0; git diff --check exit 0; PROP-007 XML parse succeeded. Initial new-file formatting check and subsequent tracked-file formatting check each failed with literal diffs, retained below; corrected formatting only, no failures suppressed. No broad suite or conditional codegen/specmap was run: no schema, generator or source index changed and no required dependent generated artifact was identified.

Open obligations and limits
Native Windows fixture has not executed; Windows/macOS target branches have not been cross-compiled or executed. Legal Windows long-root installation is not proven, and no observed Windows failure justified adding a path-spelling workaround. Native case currently requires successful installation; any unrelated failure fails the test instead of becoming a portability pass. A genuinely unsupported Windows host still needs a native negative fixture with exact effective limit, pre-effect evidence and practical numeric root-shortening guidance.
The diagnostics do not establish exact effective path/component limits or minimum root-shortening amounts. This is an explicit remaining part of acceptance criterion 1, not satisfied by the Linux path measurement. Read-only probes can miss limits behind an earlier nonexistent component on some hosts, and cannot guarantee later I/O success or transactionality. Unexpected later record-writer errors still use its existing detailed string diagnostics. No claim of complete pre-effect refusal on every filesystem, universal rollback or all-slot atomicity is made. The new negative case exercises an existing slot; a separate native Windows new-slot refusal remains open.
Criterion 0: satisfied by full task-contract read. Criteria 1 and 2: unresolved pending native Windows functional evidence, exact-limit behavior and host review (verify.m06.g4.01). All observed Linux positive/negative evidence reviewed locally; no automatic acceptance is inferred.
Follow-up: on an authorized disposable native Windows host run the selected family, record exact paths and stages, establish host/API/component limits and root-shortening amounts for unsupported cases, add/refine the native negative oracle and only any observed necessary platform correction. Retain Linux checks and ownership semantics. Root owns final acceptance and platform follow-up.

Raw selected test output (exit 0)
warning: unused field `description` in `workspace.package`
  --> Cargo.toml:91:1
   |
91 | description = "The disciplined runtime for spec-driven vibecoding"
   | ^^^^^^^^^^^
   |
   = note: `cargo::unused_workspace_package_fields` is set to `warn` by default
help: consider removing the field `workspace.package.description`
warning: unused field `keywords` in `workspace.package`
  --> Cargo.toml:92:1
   |
92 | keywords = ["spec-driven", "ai", "cli", "package-manager"]
   | ^^^^^^^^
   |
help: consider removing the field `workspace.package.keywords`
warning: unused field `categories` in `workspace.package`
  --> Cargo.toml:93:1
   |
93 | categories = ["command-line-utilities", "development-tools"]
   | ^^^^^^^^^^
   |
help: consider removing the field `workspace.package.categories`
warning: workspace (manifest) generated 3 warnings
warning: `package.homepage` is redundant with `package.repository`
  --> crates/vibe-test-support/Cargo.toml:10:1
   |
 9 | repository.workspace = true
   | ----------
10 | homepage.workspace = true
   | ^^^^^^^^
   |
   = note: `cargo::redundant_homepage` is set to `warn` by default
help: consider removing `package.homepage`
warning: `vibe-test-support` (manifest) generated 1 warning
warning: `package.homepage` is redundant with `package.repository`
  --> crates/progress-core/Cargo.toml:10:1
   |
 9 | repository.workspace = true
   | ----------
10 | homepage.workspace = true
   | ^^^^^^^^
   |
   = note: `cargo::redundant_homepage` is set to `warn` by default
help: consider removing `package.homepage`
warning: `progress-core` (manifest) generated 1 warning
warning: `package.homepage` is redundant with `package.repository`
  --> crates/vibe-package-source/Cargo.toml:10:1
   |
 9 | repository.workspace = true
   | ----------
10 | homepage.workspace = true
   | ^^^^^^^^
   |
   = note: `cargo::redundant_homepage` is set to `warn` by default
help: consider removing `package.homepage`
warning: `vibe-package-source` (manifest) generated 1 warning
warning: `package.homepage` is redundant with `package.repository`
  --> xtask/Cargo.toml:11:1
   |
10 | repository.workspace = true
   | ----------
11 | homepage.workspace = true
   | ^^^^^^^^
   |
   = note: `cargo::redundant_homepage` is set to `warn` by default
help: consider removing `package.homepage`
warning: `xtask` (manifest) generated 1 warning
warning: `package.homepage` is redundant with `package.repository`
  --> crates/vibe-spec/Cargo.toml:10:1
   |
 9 | repository.workspace = true
   | ----------
10 | homepage.workspace = true
   | ^^^^^^^^
   |
   = note: `cargo::redundant_homepage` is set to `warn` by default
help: consider removing `package.homepage`
warning: `vibe-spec` (manifest) generated 1 warning
warning: `package.homepage` is redundant with `package.repository`
  --> crates/vibe-lifecycle/Cargo.toml:10:1
   |
 9 | repository.workspace = true
   | ----------
10 | homepage.workspace = true
   | ^^^^^^^^
   |
   = note: `cargo::redundant_homepage` is set to `warn` by default
help: consider removing `package.homepage`
warning: `vibe-lifecycle` (manifest) generated 1 warning
warning: `package.homepage` is redundant with `package.repository`
  --> crates/vibe-actions/Cargo.toml:10:1
   |
 9 | repository.workspace = true
   | ----------
10 | homepage.workspace = true
   | ^^^^^^^^
   |
   = note: `cargo::redundant_homepage` is set to `warn` by default
help: consider removing `package.homepage`
warning: `vibe-actions` (manifest) generated 1 warning
warning: `package.homepage` is redundant with `package.repository`
  --> crates/vibe-orchestrator/Cargo.toml:10:1
   |
 9 | repository.workspace = true
   | ----------
10 | homepage.workspace = true
   | ^^^^^^^^
   |
   = note: `cargo::redundant_homepage` is set to `warn` by default
help: consider removing `package.homepage`
warning: `vibe-orchestrator` (manifest) generated 1 warning
warning: crates/vibe-cli/Cargo.toml: unused manifest key: build
help: build is a valid .cargo/config.toml key
warning: `package.homepage` is redundant with `package.repository`
  --> crates/vibe-cli/Cargo.toml:10:1
   |
 9 | repository.workspace = true
   | ----------
10 | homepage.workspace = true
   | ^^^^^^^^
   |
   = note: `cargo::redundant_homepage` is set to `warn` by default
help: consider removing `package.homepage`
warning: `vibe-cli` (manifest) generated 2 warnings
warning: `package.homepage` is redundant with `package.repository`
  --> crates/vibe-safefs/Cargo.toml:10:1
   |
 9 | repository.workspace = true
   | ----------
10 | homepage.workspace = true
   | ^^^^^^^^
   |
   = note: `cargo::redundant_homepage` is set to `warn` by default
help: consider removing `package.homepage`
warning: `vibe-safefs` (manifest) generated 1 warning
warning: `package.homepage` is redundant with `package.repository`
  --> crates/vibe-facts/Cargo.toml:10:1
   |
 9 | repository.workspace = true
   | ----------
10 | homepage.workspace = true
   | ^^^^^^^^
   |
   = note: `cargo::redundant_homepage` is set to `warn` by default
help: consider removing `package.homepage`
warning: `vibe-facts` (manifest) generated 1 warning
warning: `package.homepage` is redundant with `package.repository`
  --> crates/vibe-mcp/Cargo.toml:10:1
   |
 9 | repository.workspace = true
   | ----------
10 | homepage.workspace = true
   | ^^^^^^^^
   |
   = note: `cargo::redundant_homepage` is set to `warn` by default
help: consider removing `package.homepage`
warning: `vibe-mcp` (manifest) generated 1 warning
warning: `package.homepage` is redundant with `package.repository`
  --> crates/vibe-registry/Cargo.toml:10:1
   |
 9 | repository.workspace = true
   | ----------
10 | homepage.workspace = true
   | ^^^^^^^^
   |
   = note: `cargo::redundant_homepage` is set to `warn` by default
help: consider removing `package.homepage`
warning: `vibe-registry` (manifest) generated 1 warning
warning: `package.homepage` is redundant with `package.repository`
  --> crates/vibe-check/Cargo.toml:10:1
   |
 9 | repository.workspace = true
   | ----------
10 | homepage.workspace = true
   | ^^^^^^^^
   |
   = note: `cargo::redundant_homepage` is set to `warn` by default
help: consider removing `package.homepage`
warning: `vibe-check` (manifest) generated 1 warning
warning: crates/vibe-install/Cargo.toml: file `/fast/git/v/vibevm-next/crates/vibe-install/src/lib.rs` found to be present in multiple build targets:
  * `lib` target `vibe_install`
  * `integration-test` target `m_05_e_2_cases`
  * `integration-test` target `m_05_f_2_cases`
  * `integration-test` target `m_06_e_1_cases`
warning: `package.homepage` is redundant with `package.repository`
  --> crates/vibe-install/Cargo.toml:10:1
   |
 9 | repository.workspace = true
   | ----------
10 | homepage.workspace = true
   | ^^^^^^^^
   |
   = note: `cargo::redundant_homepage` is set to `warn` by default
help: consider removing `package.homepage`
warning: `vibe-install` (manifest) generated 2 warnings
warning: `package.homepage` is redundant with `package.repository`
  --> crates/vibe-trace/Cargo.toml:10:1
   |
 9 | repository.workspace = true
   | ----------
10 | homepage.workspace = true
   | ^^^^^^^^
   |
   = note: `cargo::redundant_homepage` is set to `warn` by default
help: consider removing `package.homepage`
warning: `vibe-trace` (manifest) generated 1 warning
warning: `package.homepage` is redundant with `package.repository`
  --> crates/vibe-publish/Cargo.toml:10:1
   |
 9 | repository.workspace = true
   | ----------
10 | homepage.workspace = true
   | ^^^^^^^^
   |
   = note: `cargo::redundant_homepage` is set to `warn` by default
help: consider removing `package.homepage`
warning: `vibe-publish` (manifest) generated 1 warning
warning: `package.homepage` is redundant with `package.repository`
  --> crates/vibe-core/Cargo.toml:10:1
   |
 9 | repository.workspace = true
   | ----------
10 | homepage.workspace = true
   | ^^^^^^^^
   |
   = note: `cargo::redundant_homepage` is set to `warn` by default
help: consider removing `package.homepage`
warning: `vibe-core` (manifest) generated 1 warning
warning: `package.homepage` is redundant with `package.repository`
  --> crates/vibe-ext/Cargo.toml:10:1
   |
 9 | repository.workspace = true
   | ----------
10 | homepage.workspace = true
   | ^^^^^^^^
   |
   = note: `cargo::redundant_homepage` is set to `warn` by default
help: consider removing `package.homepage`
warning: `vibe-ext` (manifest) generated 1 warning
warning: `package.homepage` is redundant with `package.repository`
  --> crates/vibe-specdoc/Cargo.toml:10:1
   |
 9 | repository.workspace = true
   | ----------
10 | homepage.workspace = true
   | ^^^^^^^^
   |
   = note: `cargo::redundant_homepage` is set to `warn` by default
help: consider removing `package.homepage`
warning: `vibe-specdoc` (manifest) generated 1 warning
warning: `package.homepage` is redundant with `package.repository`
  --> crates/vibe-workspace/Cargo.toml:10:1
   |
 9 | repository.workspace = true
   | ----------
10 | homepage.workspace = true
   | ^^^^^^^^
   |
   = note: `cargo::redundant_homepage` is set to `warn` by default
help: consider removing `package.homepage`
warning: `vibe-workspace` (manifest) generated 1 warning
warning: `package.homepage` is redundant with `package.repository`
  --> crates/vibe-extension-registry/Cargo.toml:10:1
   |
 9 | repository.workspace = true
   | ----------
10 | homepage.workspace = true
   | ^^^^^^^^
   |
   = note: `cargo::redundant_homepage` is set to `warn` by default
help: consider removing `package.homepage`
warning: `vibe-extension-registry` (manifest) generated 1 warning
warning: `package.homepage` is redundant with `package.repository`
  --> crates/vibe-native-loader/Cargo.toml:10:1
   |
 9 | repository.workspace = true
   | ----------
10 | homepage.workspace = true
   | ^^^^^^^^
   |
   = note: `cargo::redundant_homepage` is set to `warn` by default
help: consider removing `package.homepage`
warning: `vibe-native-loader` (manifest) generated 1 warning
warning: `package.homepage` is redundant with `package.repository`
  --> crates/vibe-requirements/Cargo.toml:10:1
   |
 9 | repository.workspace = true
   | ----------
10 | homepage.workspace = true
   | ^^^^^^^^
   |
   = note: `cargo::redundant_homepage` is set to `warn` by default
help: consider removing `package.homepage`
warning: `vibe-requirements` (manifest) generated 1 warning
warning: `package.homepage` is redundant with `package.repository`
  --> crates/vibe-wire/Cargo.toml:10:1
   |
 9 | repository.workspace = true
   | ----------
10 | homepage.workspace = true
   | ^^^^^^^^
   |
   = note: `cargo::redundant_homepage` is set to `warn` by default
help: consider removing `package.homepage`
warning: `vibe-wire` (manifest) generated 1 warning
warning: `package.homepage` is redundant with `package.repository`
  --> crates/vibe-resolver/Cargo.toml:10:1
   |
 9 | repository.workspace = true
   | ----------
10 | homepage.workspace = true
   | ^^^^^^^^
   |
   = note: `cargo::redundant_homepage` is set to `warn` by default
help: consider removing `package.homepage`
warning: `vibe-resolver` (manifest) generated 1 warning
warning: `package.homepage` is redundant with `package.repository`
  --> crates/vibe-agent-projection/Cargo.toml:10:1
   |
 9 | repository.workspace = true
   | ----------
10 | homepage.workspace = true
   | ^^^^^^^^
   |
   = note: `cargo::redundant_homepage` is set to `warn` by default
help: consider removing `package.homepage`
warning: `vibe-agent-projection` (manifest) generated 1 warning
warning: `package.homepage` is redundant with `package.repository`
  --> crates/vibe-doc-server/Cargo.toml:10:1
   |
 9 | repository.workspace = true
   | ----------
10 | homepage.workspace = true
   | ^^^^^^^^
   |
   = note: `cargo::redundant_homepage` is set to `warn` by default
help: consider removing `package.homepage`
warning: `vibe-doc-server` (manifest) generated 1 warning
warning: `package.homepage` is redundant with `package.repository`
  --> crates/vibe-doc-shell/Cargo.toml:10:1
   |
 9 | repository.workspace = true
   | ----------
10 | homepage.workspace = true
   | ^^^^^^^^
   |
   = note: `cargo::redundant_homepage` is set to `warn` by default
help: consider removing `package.homepage`
warning: `vibe-doc-shell` (manifest) generated 1 warning
warning: `package.homepage` is redundant with `package.repository`
  --> crates/vibe-doc/Cargo.toml:10:1
   |
 9 | repository.workspace = true
   | ----------
10 | homepage.workspace = true
   | ^^^^^^^^
   |
   = note: `cargo::redundant_homepage` is set to `warn` by default
help: consider removing `package.homepage`
warning: `vibe-doc` (manifest) generated 1 warning
warning: `package.homepage` is redundant with `package.repository`
  --> crates/vibe-graph/Cargo.toml:10:1
   |
 9 | repository.workspace = true
   | ----------
10 | homepage.workspace = true
   | ^^^^^^^^
   |
   = note: `cargo::redundant_homepage` is set to `warn` by default
help: consider removing `package.homepage`
warning: `vibe-graph` (manifest) generated 1 warning
warning: `package.homepage` is redundant with `package.repository`
  --> crates/vibe-scrape/Cargo.toml:10:1
   |
 9 | repository.workspace = true
   | ----------
10 | homepage.workspace = true
   | ^^^^^^^^
   |
   = note: `cargo::redundant_homepage` is set to `warn` by default
help: consider removing `package.homepage`
warning: `vibe-scrape` (manifest) generated 1 warning
warning: `package.homepage` is redundant with `package.repository`
  --> crates/vibe-llm/Cargo.toml:10:1
   |
 9 | repository.workspace = true
   | ----------
10 | homepage.workspace = true
   | ^^^^^^^^
   |
   = note: `cargo::redundant_homepage` is set to `warn` by default
help: consider removing `package.homepage`
warning: `vibe-llm` (manifest) generated 1 warning
warning: `package.homepage` is redundant with `package.repository`
  --> crates/vibe-index/Cargo.toml:10:12
   |
10 | homepage = "https://gitverse.ru/vibevm/vibevm"
   |            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
11 | repository = "https://gitverse.ru/vibevm/vibevm"
   |              -----------------------------------
   |
   = note: `cargo::redundant_homepage` is set to `warn` by default
help: consider removing `package.homepage`
warning: `vibe-index` (manifest) generated 1 warning
warning: `package.homepage` is redundant with `package.repository`
  --> crates/vibe-settings/Cargo.toml:10:1
   |
 9 | repository.workspace = true
   | ----------
10 | homepage.workspace = true
   | ^^^^^^^^
   |
   = note: `cargo::redundant_homepage` is set to `warn` by default
help: consider removing `package.homepage`
warning: `vibe-settings` (manifest) generated 1 warning
warning: function `during` is never used
   --> crates/vibe-safefs/src/transaction.rs:252:12
    |
252 |     pub fn during(parent: &crate::Pinned, name: &str) {
    |            ^^^^^^
    |
    = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: variants `Occupied`, `SourceChanged`, `SourceReappeared`, `CrossFilesystem`, and `Io` are never constructed
  --> crates/vibe-safefs/src/transaction/platform.rs:10:5
   |
 9 | pub(super) enum NoReplaceError {
   |                 -------------- variants in this enum
10 |     Occupied,
   |     ^^^^^^^^
11 |     SourceChanged,
   |     ^^^^^^^^^^^^^
12 |     SourceReappeared,
   |     ^^^^^^^^^^^^^^^^
13 |     CrossFilesystem,
   |     ^^^^^^^^^^^^^^^
14 |     Unsupported,
15 |     Io(std::io::Error),
   |     ^^
   |
   = note: `NoReplaceError` has a derived impl for the trait `Debug`, but this is intentionally ignored during dead code analysis

warning: variants `NotCreated` and `CreatedButUnsealed` are never constructed
  --> crates/vibe-safefs/src/transaction/platform.rs:19:5
   |
18 | pub(super) enum NativeCreateError {
   |                 ----------------- variants in this enum
19 |     NotCreated(std::io::Error),
   |     ^^^^^^^^^^
20 |     CreatedButUnsealed(std::io::Error),
   |     ^^^^^^^^^^^^^^^^^^

warning: variants `Changed` and `Io` are never constructed
  --> crates/vibe-safefs/src/transaction/platform.rs:26:5
   |
25 | pub(super) enum NativeRemoveError {
   |                 ----------------- variants in this enum
26 |     Changed(String),
   |     ^^^^^^^
27 |     Io(std::io::Error),
   |     ^^

warning: `vibe-safefs` (lib) generated 4 warnings
   Compiling vibe-spec v1.1.0 (/fast/git/v/vibevm-next/crates/vibe-spec)
   Compiling vibe-facts v1.1.0 (/fast/git/v/vibevm-next/crates/vibe-facts)
   Compiling vibe-workspace v1.1.0 (/fast/git/v/vibevm-next/crates/vibe-workspace)
warning: fields `output` and `expected` are never read
  --> /fast/git/v/vibevm-next/crates/vibe-workspace/../../fixtures/next/derivatives/service_fixture.rs:13:5
   |
 9 | struct Fixture {
   |        ------- fields in this struct
...
13 |     output: Project,
   |     ^^^^^^
...
16 |     expected: Vec<RetainedFile>,
   |     ^^^^^^^^
   |
   = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: field `output` is never read
  --> /fast/git/v/vibevm-next/crates/vibe-workspace/../../fixtures/next/derivatives/service_fixture.rs:13:5
   |
 9 | struct Fixture {
   |        ------- field in this struct
...
13 |     output: Project,
   |     ^^^^^^

warning: `vibe-workspace` (lib test) generated 3 warnings (1 duplicate)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 9.86s
     Running unittests src/lib.rs (/fast/zap-managed/2026-10-14-m10b1-correction/cargo-target/debug/build/vibe-workspace/a8265efdd2c697f3/out/vibe_workspace-a8265efdd2c697f3)

running 3 tests
G4 materialization destination: /tmp/vibevm-m06g4-yqr139oe/.tmpnfx2f2/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/vibevm/vibedeps/org.example.path-fixture/1.1.0/vibevm/vibespecs/long-spec-name-long-spec-name-long-spec-name-long-spec-name-long-spec-name-long-spec-name-long-spec-name-long-spec-name-long-spec-name-long-spec-name-long-spec-name-long-spec-name-.xml
test m_06_g_4_cases::long_root_spec_path_installs_on_this_host ... ok
G4 native probe: /tmp/vibevm-m06g4-yqr139oe/.tmpI0Vjhz/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/vibevm/vibedeps/org.example.path-fixture/1.1.0/vibevm/vibespecs/spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-.xml: File name too long (os error 36)
G4 install refusal: cannot materialise spec documents at `/tmp/vibevm-m06g4-yqr139oe/.tmpI0Vjhz/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/vibevm/vibedeps/org.example.path-fixture/1.1.0/vibevm/vibespecs/spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-.xml`: path-length refusal during payload destination: `/tmp/vibevm-m06g4-yqr139oe/.tmpI0Vjhz/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/vibevm/vibedeps/org.example.path-fixture/1.1.0/vibevm/vibespecs/spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-.xml` (4207 bytes excluding NUL); native OS code Some(36): File name too long (os error 36). The host did not report its exact effective path/component limit. Shorten the project root and retry the same package; if a single filename is too long, root shortening cannot fix that component. No filename is silently shortened. (violates spec://org.vibevm.core/vibevm/common/PROP-045#materialisation; fix: repair the colliding path or unreadable source, then re-run install)
test m_06_g_4_cases::unsupported_path_diagnostic_is_not_nesting_or_false_success ... ok
test m_06_g_4_cases::ordinary_short_root_and_owned_retry_keep_neighbors ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 775 filtered out; finished in 0.01s


Initial new-file rustfmt failure (exit 1; corrected)
Diff in /fast/git/v/vibevm-next/crates/vibe-workspace/src/vibedeps/slot_diff/path_limits.rs:33:

 pub(super) fn io_error(path: &Path, error: io::Error) -> WorkspaceError {
     if is_length_error(&error) {
-        length_error(path, "slot reconciliation I/O (state may be partial)", error)
+        length_error(
+            path,
+            "slot reconciliation I/O (state may be partial)",
+            error,
+        )
     } else {
         super::super::io_err(path, error)
     }
Diff in /fast/git/v/vibevm-next/crates/vibe-workspace/src/vibedeps/slot_diff/path_limits.rs:51:
     #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
     return false;
     #[cfg(any(windows, target_os = "linux", target_os = "macos"))]
-    { error.raw_os_error() == Some(code) }
+    {
+        error.raw_os_error() == Some(code)
+    }
 }

 fn length_error(path: &Path, stage: &str, error: io::Error) -> WorkspaceError {
Diff in /fast/git/v/vibevm-next/crates/vibe-workspace/src/vibedeps/slot_diff/path_limits.rs:58:
     #[cfg(windows)]
     let measured = {
         use std::os::windows::ffi::OsStrExt;
-        format!("{} UTF-16 code units excluding NUL", path.as_os_str().encode_wide().count())
+        format!(
+            "{} UTF-16 code units excluding NUL",
+            path.as_os_str().encode_wide().count()
+        )
     };
     #[cfg(not(windows))]
-    let measured = format!("{} bytes excluding NUL", path.as_os_str().as_encoded_bytes().len());
+    let measured = format!(
+        "{} bytes excluding NUL",
+        path.as_os_str().as_encoded_bytes().len()
+    );
     WorkspaceError::SpecMaterialization {
         path: path.to_path_buf(),
         reason: format!(
Diff in /fast/git/v/vibevm-next/crates/vibe-workspace/src/vibedeps/slot_diff/path_limits.rs:69:
              The host did not report its exact effective path/component limit. \
              Shorten the project root and retry the same package; if a single filename is too long, \
              root shortening cannot fix that component. No filename is silently shortened.",
-            path.display(), error.raw_os_error()
+            path.display(),
+            error.raw_os_error()
         ),
     }
 }
Diff in /fast/git/v/vibevm-next/crates/vibe-workspace/src/vibedeps/m_06_g_4_cases.rs:1:
 //! Disposable real install-composition fixtures. Native Windows is a separate
 //! obligation: execution of the POSIX cases is never Windows evidence.

-use std::{fs, path::{Path, PathBuf}};
+use crate::{
+    Workspace, WorkspaceError,
+    install::{self, ResolvedDep, SlotLifecycleMode},
+    vibedeps,
+};
+use std::{
+    fs,
+    path::{Path, PathBuf},
+};
 use tempfile::TempDir;
-use vibe_core::{ContentHash, Group, PackageKind, manifest::{Manifest, SpecFormat}, user_config::SlotIntegrity};
-use crate::{Workspace, WorkspaceError, install::{self, ResolvedDep, SlotLifecycleMode}, vibedeps};
+use vibe_core::{
+    ContentHash, Group, PackageKind,
+    manifest::{Manifest, SpecFormat},
+    user_config::SlotIntegrity,
+};

 fn write(root: &Path, relative: impl AsRef<Path>, bytes: &[u8]) {
     let path = root.join(relative);
Diff in /fast/git/v/vibevm-next/crates/vibe-workspace/src/vibedeps/m_06_g_4_cases.rs:29:
         source_hash: Some(ContentHash::parse(&format!("sha256:{}", "1".repeat(64))).unwrap()),
         embedded_sources: vec![],
         manifest: Manifest::read(source.path().join("vibe.toml")).unwrap(),
-        requires: vec![], admitted_by: None, via_override: None,
-        source_mutable: true, in_place_changed: None,
+        requires: vec![],
+        admitted_by: None,
+        via_override: None,
+        source_mutable: true,
+        in_place_changed: None,
     };
     (source, dep)
 }
Diff in /fast/git/v/vibevm-next/crates/vibe-workspace/src/vibedeps/m_06_g_4_cases.rs:39:
     // The same public composition entry used by vibe-install/src/apply.rs.
     // Verify + no verifier deliberately reconciles rather than trusting presence.
     install::apply_resolution_with_configuration_recovery(
-        ws, std::slice::from_ref(dep), SlotIntegrity::Verify, SpecFormat::Mixed,
-        None, SlotLifecycleMode::None, None, None, None,
+        ws,
+        std::slice::from_ref(dep),
+        SlotIntegrity::Verify,
+        SpecFormat::Mixed,
+        None,
+        SlotLifecycleMode::None,
+        None,
+        None,
+        None,
     )
 }

Diff in /fast/git/v/vibevm-next/crates/vibe-workspace/src/vibedeps/m_06_g_4_cases.rs:62:
     let root = nested_root(temp.path(), minimum);
     let ws = workspace(&root);
     let (source, dep) = package();
-    let relative = PathBuf::from(format!("vibevm/vibespecs/{}.xml", "long-spec-name-".repeat(12)));
-    let bytes = b"<spec xmlns=\"https://vibevm.org/spec/1\"><title id=\"root\">Path fixture</title></spec>";
+    let relative = PathBuf::from(format!(
+        "vibevm/vibespecs/{}.xml",
+        "long-spec-name-".repeat(12)
+    ));
+    let bytes =
+        b"<spec xmlns=\"https://vibevm.org/spec/1\"><title id=\"root\">Path fixture</title></spec>";
     write(source.path(), &relative, bytes);
     let output = slot(&ws, &dep).join(&relative);
     assert!(output.as_os_str().as_encoded_bytes().len() > 260);
Diff in /fast/git/v/vibevm-next/crates/vibe-workspace/src/vibedeps/m_06_g_4_cases.rs:70:
     eprintln!("G4 materialization destination: {}", output.display());
-    let result = apply(&ws, &dep).unwrap_or_else(|error| panic!("install composition failed at {}: {error}", output.display()));
-    assert_eq!(result.materialised, vec![vibedeps::slot_rel_path(&dep.group, &dep.name, &dep.version)]);
+    let result = apply(&ws, &dep).unwrap_or_else(|error| {
+        panic!(
+            "install composition failed at {}: {error}",
+            output.display()
+        )
+    });
+    assert_eq!(
+        result.materialised,
+        vec![vibedeps::slot_rel_path(&dep.group, &dep.name, &dep.version)]
+    );
     assert_eq!(fs::read(&output).unwrap(), bytes);
     let record = vibedeps::read_slot_record(&slot(&ws, &dep)).unwrap();
     assert_eq!(Some(&record.source_hash), dep.source_hash.as_ref());
Diff in /fast/git/v/vibevm-next/crates/vibe-workspace/src/vibedeps/m_06_g_4_cases.rs:76:
-    assert!(record.files.iter().any(|row| row.path == relative.to_string_lossy().replace('\\', "/")));
-    assert!(fs::read_dir(output.parent().unwrap()).unwrap().all(|entry| !entry.unwrap().file_name().to_string_lossy().starts_with(".vibe-place-")));
+    assert!(
+        record
+            .files
+            .iter()
+            .any(|row| row.path == relative.to_string_lossy().replace('\\', "/"))
+    );
+    assert!(
+        fs::read_dir(output.parent().unwrap())
+            .unwrap()
+            .all(|entry| !entry
+                .unwrap()
+                .file_name()
+                .to_string_lossy()
+                .starts_with(".vibe-place-"))
+    );
 }

 #[test]
Diff in /fast/git/v/vibevm-next/crates/vibe-workspace/src/vibedeps/m_06_g_4_cases.rs:82:
     let temp = TempDir::new().unwrap();
     let ws = workspace(temp.path());
     let (source, dep) = package();
-    write(source.path(), "vibevm/vibespecs/guide.xml", b"authored payload");
+    write(
+        source.path(),
+        "vibevm/vibespecs/guide.xml",
+        b"authored payload",
+    );
     apply(&ws, &dep).unwrap();
     let destination = slot(&ws, &dep);
     write(&destination, "foreign-output", b"foreign");
Diff in /fast/git/v/vibevm-next/crates/vibe-workspace/src/vibedeps/m_06_g_4_cases.rs:95:
     let error = apply(&ws, &dep).unwrap_err().to_string();
     assert!(error.contains("collides"), "{error}");
     assert_eq!(fs::read(&record_path).unwrap(), record);
-    assert_eq!(fs::read(destination.join("new-output")).unwrap(), b"foreign occupant");
+    assert_eq!(
+        fs::read(destination.join("new-output")).unwrap(),
+        b"foreign occupant"
+    );
     // The fixture owner releases only its blocker; normal reconciliation retries.
     fs::remove_file(destination.join("new-output")).unwrap();
     apply(&ws, &dep).unwrap();
Diff in /fast/git/v/vibevm-next/crates/vibe-workspace/src/vibedeps/m_06_g_4_cases.rs:102:
-    assert_eq!(fs::read(destination.join("new-output")).unwrap(), b"incoming");
-    assert_eq!(fs::read(destination.join("foreign-output")).unwrap(), b"foreign");
+    assert_eq!(
+        fs::read(destination.join("new-output")).unwrap(),
+        b"incoming"
+    );
+    assert_eq!(
+        fs::read(destination.join("foreign-output")).unwrap(),
+        b"foreign"
+    );
     assert!(vibedeps::validate_owned_slot_removal(&destination).is_err());
     fs::remove_file(destination.join("foreign-output")).unwrap();
     vibedeps::validate_owned_slot_removal(&destination).unwrap();
Diff in /fast/git/v/vibevm-next/crates/vibe-workspace/src/vibedeps/m_06_g_4_cases.rs:107:
     assert!(vibedeps::remove_slot(&ws.root, &dep.group, &dep.name, &dep.version).unwrap());
     apply(&ws, &dep).unwrap();
-    assert_eq!(fs::read(destination.join("new-output")).unwrap(), b"incoming");
+    assert_eq!(
+        fs::read(destination.join("new-output")).unwrap(),
+        b"incoming"
+    );
 }

 #[test]
Diff in /fast/git/v/vibevm-next/crates/vibe-workspace/src/vibedeps/m_06_g_4_cases.rs:129:
     let root = nested_root(temp.path(), 3850);
     let ws = workspace(&root);
     let (source, dep) = package();
-    write(source.path(), "old-output", b"keep until successful reconciliation");
+    write(
+        source.path(),
+        "old-output",
+        b"keep until successful reconciliation",
+    );
     apply(&ws, &dep).unwrap();
     let destination = slot(&ws, &dep);
     write(&destination, "foreign-output", b"foreign");
Diff in /fast/git/v/vibevm-next/crates/vibe-workspace/src/vibedeps/m_06_g_4_cases.rs:136:
     let old_record = fs::read(destination.join(vibedeps::SLOT_RECORD_FILENAME)).unwrap();
     fs::remove_file(source.path().join("old-output")).unwrap();
     let relative = format!("vibevm/vibespecs/{}.xml", "spec-name-".repeat(23));
-    write(source.path(), &relative, b"same selected bytes after root shortening");
+    write(
+        source.path(),
+        &relative,
+        b"same selected bytes after root shortening",
+    );
     let output = destination.join(&relative);
     // Observed native failure, not an injected classifier-only oracle.
     let native = fs::symlink_metadata(&output).unwrap_err();
Diff in /fast/git/v/vibevm-next/crates/vibe-workspace/src/vibedeps/m_06_g_4_cases.rs:143:
-    assert_eq!(native.raw_os_error(), Some(36), "{}: {native}", output.display());
+    assert_eq!(
+        native.raw_os_error(),
+        Some(36),
+        "{}: {native}",
+        output.display()
+    );
     eprintln!("G4 native probe: {}: {native}", output.display());
     let error = apply(&ws, &dep).unwrap_err().to_string();
     eprintln!("G4 install refusal: {error}");
Diff in /fast/git/v/vibevm-next/crates/vibe-workspace/src/vibedeps/m_06_g_4_cases.rs:147:
-    for expected in ["path-length refusal", "payload destination", "bytes excluding NUL", "Shorten the project root", "exact effective path/component limit", &output.display().to_string()] {
+    for expected in [
+        "path-length refusal",
+        "payload destination",
+        "bytes excluding NUL",
+        "Shorten the project root",
+        "exact effective path/component limit",
+        &output.display().to_string(),
+    ] {
         assert!(error.contains(expected), "missing {expected}: {error}");
     }
-    assert_eq!(fs::read(destination.join(vibedeps::SLOT_RECORD_FILENAME)).unwrap(), old_record);
-    assert_eq!(fs::read(destination.join("old-output")).unwrap(), b"keep until successful reconciliation");
-    assert_eq!(fs::read(destination.join("foreign-output")).unwrap(), b"foreign");
+    assert_eq!(
+        fs::read(destination.join(vibedeps::SLOT_RECORD_FILENAME)).unwrap(),
+        old_record
+    );
+    assert_eq!(
+        fs::read(destination.join("old-output")).unwrap(),
+        b"keep until successful reconciliation"
+    );
+    assert_eq!(
+        fs::read(destination.join("foreign-output")).unwrap(),
+        b"foreign"
+    );
     let shorter = TempDir::new().unwrap();
     let shorter_ws = workspace(shorter.path());
     apply(&shorter_ws, &dep).unwrap();
Diff in /fast/git/v/vibevm-next/crates/vibe-workspace/src/vibedeps/m_06_g_4_cases.rs:156:
-    assert_eq!(fs::read(slot(&shorter_ws, &dep).join(&relative)).unwrap(), b"same selected bytes after root shortening");
+    assert_eq!(
+        fs::read(slot(&shorter_ws, &dep).join(&relative)).unwrap(),
+        b"same selected bytes after root shortening"
+    );
     let record = vibedeps::read_slot_record(&slot(&shorter_ws, &dep)).unwrap();
     assert_eq!(Some(&record.source_hash), dep.source_hash.as_ref());
 }

Tracked-file rustfmt failure (exit 1; corrected)
Diff in /fast/git/v/vibevm-next/crates/vibe-workspace/src/vibedeps/slot_diff.rs:11:
 use vibe_core::ContentHash;

 use super::{
-    CopyMode, SLOT_RECORD_FILENAME, SlotRecord, read_slot_record, sha256_file,
-    write_slot_record,
+    CopyMode, SLOT_RECORD_FILENAME, SlotRecord, read_slot_record, sha256_file, write_slot_record,
 };
 use crate::{WorkspaceError, path_to_slash};


Final focused rustfmt output (exit 0)
