M-06-G.4 continuation report — native proof pending

Assignment and scope
Read the full campaigns/next/tasks/M-06-G.json contract, including M-06-G.4, and the governing PROP-007/009/035 materialization/loading/compiler laws. Preserved clean-start retained implementation and fixtures; no Git mutations, delegation, installs, external source changes, configuration/toolchain/dependency changes, CI trigger or publication of packages. Scope extension used only for SafeFS project.rs + new project/path_limits.rs and one narrow workflow, as explicitly authorized. Logical workspace discovery, relative wire coordinates, package identity, ownership and no-follow checks remain independent laws. The PROP-007 addition specifies native component measurement; it does not replace those laws or the retained total-path diagnostic.

Changed files and behavior
- crates/vibe-safefs/src/project.rs: register a cfg-Windows module.
- crates/vibe-safefs/src/project/path_limits.rs (new): safe Project::windows_maximum_component_length method queries the volume of the retained directory handle with GetVolumeInformationByHandleW. One function-local unsafe allowance, documented ABI/lifetimes/NULL outputs, no exported raw handle and no new dependency. Zero or failed query returns unavailable rather than a guessed bound.
- crates/vibe-workspace/src/vibedeps/slot_diff/path_limits.rs: before the existing metadata probe and reconciliation effects, locate the nearest existing no-follow pinned parent and check its prospective suffix components using the queried WCHAR limit. Missing parents cannot mask an oversized package slot component. Each slot, record, record staging, payload and payload staging path remains in the preflight. A component refusal names exact destination, component, stage, UTF-16 length, bound, deficit, query parent/basis, and a practical remedy. It explicitly states that root shortening cannot fix that component and never fabricates an OS error code. Native OS length probes remain as fallback with query availability detail; later I/O errors still admit possible partial state. A component bound does not establish a total pathname ceiling. POSIX behavior is unchanged.
- crates/vibe-workspace/src/vibedeps/m_06_g_4_cases.rs: retain positive long-root, short-root/owned cleanup/retry, and Linux OS-length refusal cases. Measure Windows paths in UTF-16 rather than encoded byte length. Add native volume L/L+1 ASCII creation control (actual OS code retained; census proves no created L+1 entry), and parser-valid overlong group.name with a short physical source. Keep manifest, requires and ResolvedDep identity consistent. Real public install composition must refuse the new slot before dependency parent/slot/record/payload creation; source manifest/payload and foreign marker must survive. An explicitly selected shorter coordinate then installs the selected payload and preserves the neighbor. Optional VIBE_G4_EVIDENCE_DIR writes compact JSON only on native execution; no native JSON was fabricated here.
- vibevm/vibespecs/modules/vibe-workspace/PROP-007-workspace.xml: additive MATERIALIZATION-COMPONENT-LIMIT law, retaining prior MATERIALIZATION-PATH-LENGTH and all other facts.
- .github/workflows/windows-materialization-paths.yml (new): push to actual branch 1.1.0, narrow owner paths, one windows-2025 native job, contents:read only, pinned checkout/upload actions, persist-credentials:false. Preinstalled toolchain only, RUSTUP_AUTO_INSTALL=0, source SHA/runner image/tool versions recorded. One selected cargo test command, actual exit preserved without pipeline masking; always upload combined log and JSON as g4-native-windows-${github.sha}. No release campaign or manual dispatch introduced. Cold CI uses --locked with normal dependency resolution; local run uses --offline too. Workflow was not triggered.

Production route and proof limits
Real non-test caller: crates/vibe-install/src/apply.rs calls vibe_workspace::install::apply_resolution_with_configuration_recovery for the standard installation path. The fixture constructs Workspace via Workspace::load from authored vibe.toml and prepares a real source directory, manifest and ResolvedDep, then invokes this same public composition with SlotIntegrity::Verify, SpecFormat::Mixed and SlotLifecycleMode::None. This reaches install/materialise.rs -> vibedeps::materialise_with_spec_format_report (or prepared publication) -> reconcile_slot_with_policy -> path_limits::preflight before inspect_existing_record/create/migrate/place/remove/write-record. The change is wired into ordinary reconciliation, not an opt-in production helper. The local fixtures exercise the public composition, not CLI resolution/registry/network or the full CLI command. The new Windows-specific branch is wired but neither compiled nor executed on this Linux host. Windows fixture source and workflow presence are proposed native evidence routes only, not completed production proof.

Primary native API source inspected
https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getvolumeinformationbyhandlew documents the handle query, WCHAR component units, optional outputs, zero failure return and Kernel32 linkage. This is a component bound only; no assumed 255/260/32767 threshold was added.

Bound verification actually executed
Only cargo test --offline --locked -j2 -p vibe-workspace --lib m_06_g_4_cases -- --nocapture, using absolute /home/olegchir/.cargo/bin/cargo, RUSTUP_AUTO_INSTALL=0, exclusive granted CARGO_TARGET_DIR=/fast/zap-managed/2026-10-14-m10b1-correction/cargo-target and private0700 TMPDIR=/tmp/vibevm-g4-native-zwdox0uw. Two executions: initial edited tree and final fixture measurement edit; both selected exactly 3 cases, 3 passed, 0 failed, 775 filtered out. These are Linux passes only. The two cfg-Windows-only tests were absent, not passed; on Windows the common retry plus native long-root and native component tests form the three-case family. Existing parent dev Cranelift configuration retained; no LLVM override used. Target scan used sudo -n /proc inspection: no permission gaps and no target cwd/exe/fd/maps references. Environment-only references belonged to current codex/controller/shell and, in the final scan, the sccache server started by the first selected build. No concurrent Vibe writer/build target holder was found; persistent sccache was left running.

Scoped rustfmt --check initially exited 1; literal output retained below. Applied rustfmt only to touched Rust owners with skip_children=true. Final same scoped check exited 0. git diff --check exited 0. PROP-007 XML parsed. Workflow YAML parsed with BaseLoader and structure asserted: 1.1.0 push, one Windows job, read-only contents, disabled checkout credentials, always upload, 40-hex pinned actions. These are static checks, not GitHub execution. No schema/generator/source-index change, so conditional codegen/specmap oracle not applicable; no whole-workspace, mutation, crossbuild, Wine or release gate was run.

Open obligations / Need for Root
1. Root reviews this exact patch, commits and performs its ordinary authorized mirror push if suitable. Worker performed no commit/push/CI trigger.
2. Root inspects actual source-SHA-bound windows-2025 run/log/JSON for successful long root, actual queried L and L/L+1 control OS code/census, exact new-slot refusal and source/neighbor preservation/retry. Build failure, unsupported query or unrelated install error stays failed/open, never waived.
3. No observed numeric Windows component limit or native total-path ceiling can be reported yet. Total-path ceiling/minimum root shortening remains unknown where the OS only returns a length error. macOS was not executed. No universal transaction/rollback guarantee was added; unexpected later I/O can still leave owned residue.
4. Host verification verify.m06.g4.01 and independent final review/acceptance remain open. Artifact publication and progress acknowledgment retain content only.

Criteria: slot0 satisfied (full contract read); slot1 unresolved (native usability/exact-limit proof not run); slot2 unresolved (native positive/negative review remains open). Local source effect is complete useful WIP; overall effect_state pending native proof. No earlier report/candidate was treated as acceptance.

--- retained target-scan.json ---
{"time_ns": 1792391195102594497, "refs": [["2202445", "environ", "target reference"], ["2203751", "environ", "target reference"], ["2222667", "environ", "target reference"], ["2222668", "environ", "target reference"]], "gaps": []}

--- retained target-scan-final.json ---
{"time_ns": 1792391251497933045, "refs": [["2202445", "environ", "target reference"], ["2203751", "environ", "target reference"], ["2223345", "environ", "target reference"], ["2232292", "environ", "target reference"], ["2232293", "environ", "target reference"]], "gaps": []}

--- retained tests.log ---
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
   Compiling vibe-safefs v1.1.0 (/fast/git/v/vibevm-next/crates/vibe-safefs)
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
   = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: function `during` is never used
   --> crates/vibe-safefs/src/transaction.rs:252:12
    |
252 |     pub fn during(parent: &crate::Pinned, name: &str) {
    |            ^^^^^^

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

   Compiling vibe-spec v1.1.0 (/fast/git/v/vibevm-next/crates/vibe-spec)
warning: `vibe-safefs` (lib) generated 4 warnings
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
    Finished `test` profile [unoptimized + debuginfo] target(s) in 7.72s
     Running unittests src/lib.rs (/fast/zap-managed/2026-10-14-m10b1-correction/cargo-target/debug/build/vibe-workspace/a8265efdd2c697f3/out/vibe_workspace-a8265efdd2c697f3)

running 3 tests
G4 materialization destination: /tmp/vibevm-g4-native-zwdox0uw/.tmppKt49X/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/vibevm/vibedeps/org.example.path-fixture/1.1.0/vibevm/vibespecs/long-spec-name-long-spec-name-long-spec-name-long-spec-name-long-spec-name-long-spec-name-long-spec-name-long-spec-name-long-spec-name-long-spec-name-long-spec-name-long-spec-name-.xml
test m_06_g_4_cases::long_root_spec_path_installs_on_this_host ... ok
G4 native probe: /tmp/vibevm-g4-native-zwdox0uw/.tmpD6v0yg/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/vibevm/vibedeps/org.example.path-fixture/1.1.0/vibevm/vibespecs/spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-.xml: File name too long (os error 36)
G4 install refusal: cannot materialise spec documents at `/tmp/vibevm-g4-native-zwdox0uw/.tmpD6v0yg/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/vibevm/vibedeps/org.example.path-fixture/1.1.0/vibevm/vibespecs/spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-.xml`: path-length refusal during payload destination: `/tmp/vibevm-g4-native-zwdox0uw/.tmpD6v0yg/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/vibevm/vibedeps/org.example.path-fixture/1.1.0/vibevm/vibespecs/spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-.xml` (4211 bytes excluding NUL); native OS code Some(36): File name too long (os error 36). The host did not report its exact effective path/component limit. Shorten the project root and retry the same package; if a single filename is too long, root shortening cannot fix that component. No filename is silently shortened. (violates spec://org.vibevm.core/vibevm/common/PROP-045#materialisation; fix: repair the colliding path or unreadable source, then re-run install)
test m_06_g_4_cases::unsupported_path_diagnostic_is_not_nesting_or_false_success ... ok
test m_06_g_4_cases::ordinary_short_root_and_owned_retry_keep_neighbors ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 775 filtered out; finished in 0.01s


--- retained tests.exit ---
0

--- retained tests-final.log ---
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
   = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

warning: function `during` is never used
   --> crates/vibe-safefs/src/transaction.rs:252:12
    |
252 |     pub fn during(parent: &crate::Pinned, name: &str) {
    |            ^^^^^^

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
    Finished `test` profile [unoptimized + debuginfo] target(s) in 3.49s
     Running unittests src/lib.rs (/fast/zap-managed/2026-10-14-m10b1-correction/cargo-target/debug/build/vibe-workspace/a8265efdd2c697f3/out/vibe_workspace-a8265efdd2c697f3)

running 3 tests
G4 materialization destination: /tmp/vibevm-g4-native-zwdox0uw/.tmpOr30di/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/vibevm/vibedeps/org.example.path-fixture/1.1.0/vibevm/vibespecs/long-spec-name-long-spec-name-long-spec-name-long-spec-name-long-spec-name-long-spec-name-long-spec-name-long-spec-name-long-spec-name-long-spec-name-long-spec-name-long-spec-name-.xml
G4 native probe: /tmp/vibevm-g4-native-zwdox0uw/.tmpoC8w4H/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/vibevm/vibedeps/org.example.path-fixture/1.1.0/vibevm/vibespecs/spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-.xml: File name too long (os error 36)
test m_06_g_4_cases::long_root_spec_path_installs_on_this_host ... ok
G4 install refusal: cannot materialise spec documents at `/tmp/vibevm-g4-native-zwdox0uw/.tmpoC8w4H/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/vibevm/vibedeps/org.example.path-fixture/1.1.0/vibevm/vibespecs/spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-.xml`: path-length refusal during payload destination: `/tmp/vibevm-g4-native-zwdox0uw/.tmpoC8w4H/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/root-segment-root-segment-root-segment-root-segment-root-segment-root-segment-/vibevm/vibedeps/org.example.path-fixture/1.1.0/vibevm/vibespecs/spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-spec-name-.xml` (4211 bytes excluding NUL); native OS code Some(36): File name too long (os error 36). The host did not report its exact effective path/component limit. Shorten the project root and retry the same package; if a single filename is too long, root shortening cannot fix that component. No filename is silently shortened. (violates spec://org.vibevm.core/vibevm/common/PROP-045#materialisation; fix: repair the colliding path or unreadable source, then re-run install)
test m_06_g_4_cases::unsupported_path_diagnostic_is_not_nesting_or_false_success ... ok
test m_06_g_4_cases::ordinary_short_root_and_owned_retry_keep_neighbors ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 775 filtered out; finished in 0.01s


--- retained tests-final.exit ---
0

--- retained fmt-initial.log ---
Diff in /fast/git/v/vibevm-next/crates/vibe-workspace/src/vibedeps/slot_diff/path_limits.rs:41:
                 other => other,
             };
             Err(error)
-        },
+        }
         _ => Ok(()),
     }
 }
Diff in /fast/git/v/vibevm-next/crates/vibe-workspace/src/vibedeps/slot_diff/path_limits.rs:114:
             Ok(project) => project,
             Err(error) => {
                 if error.chain().any(|cause| {
-                    cause.downcast_ref::<io::Error>()
-                        .is_some_and(|error| error.kind() == io::ErrorKind::NotFound || is_length_error(error))
+                    cause.downcast_ref::<io::Error>().is_some_and(|error| {
+                        error.kind() == io::ErrorKind::NotFound || is_length_error(error)
+                    })
                 }) {
                     continue;
                 }
Diff in /fast/git/v/vibevm-next/crates/vibe-workspace/src/vibedeps/slot_diff/path_limits.rs:126:
             Ok(limit) => limit as usize,
             Err(error) => return Ok(format!("unavailable at `{}`: {error}", parent.display())),
         };
-        for component in path.strip_prefix(parent).expect("ancestor prefix").components() {
+        for component in path
+            .strip_prefix(parent)
+            .expect("ancestor prefix")
+            .components()
+        {
             let measured = component.as_os_str().encode_wide().count();
             if measured > limit {
                 return Err(WorkspaceError::SpecMaterialization {
Diff in /fast/git/v/vibevm-next/crates/vibe-workspace/src/vibedeps/slot_diff/path_limits.rs:140:
                          target volume and retry. No filename is silently shortened. \
                          Pre-effect metadata refusal; no native failure code was generated. \
                          This component bound does not establish a total path limit.",
-                        path.display(), component.as_os_str().to_string_lossy(),
-                        parent.display(), measured - limit,
+                        path.display(),
+                        component.as_os_str().to_string_lossy(),
+                        parent.display(),
+                        measured - limit,
                     ),
                 });
             }
Diff in /fast/git/v/vibevm-next/crates/vibe-workspace/src/vibedeps/slot_diff/path_limits.rs:148:
         }
-        return Ok(format!("component limit {limit} WCHARs at `{}`; total path limit unknown", parent.display()));
+        return Ok(format!(
+            "component limit {limit} WCHARs at `{}`; total path limit unknown",
+            parent.display()
+        ));
     }
     Ok("unavailable: no existing parent capability".into())
 }
Diff in /fast/git/v/vibevm-next/crates/vibe-workspace/src/vibedeps/m_06_g_4_cases.rs:113:
             .any(|row| row.path == relative.to_string_lossy().replace('\\', "/"))
     );
     #[cfg(windows)]
-    evidence("long-root", serde_json::json!({
-        "destination": output, "outcome": "installed", "bytes_verified": true,
-        "record_verified": true,
-    }));
+    evidence(
+        "long-root",
+        serde_json::json!({
+            "destination": output, "outcome": "installed", "bytes_verified": true,
+            "record_verified": true,
+        }),
+    );
     assert!(
         fs::read_dir(output.parent().unwrap())
             .unwrap()
Diff in /fast/git/v/vibevm-next/crates/vibe-workspace/src/vibedeps/m_06_g_4_cases.rs:175:
         b"incoming"
     );
     #[cfg(windows)]
-    evidence("owned-retry", serde_json::json!({
-        "short_root": temp.path(), "foreign_preserved": true,
-        "collision_refused": true, "owned_cleanup_and_retry": true,
-    }));
+    evidence(
+        "owned-retry",
+        serde_json::json!({
+            "short_root": temp.path(), "foreign_preserved": true,
+            "collision_refused": true, "owned_cleanup_and_retry": true,
+        }),
+    );
 }

 #[cfg(not(windows))]
Diff in /fast/git/v/vibevm-next/crates/vibe-workspace/src/vibedeps/m_06_g_4_cases.rs:268:
     eprintln!("G4 {name}: {value}");
     if let Some(directory) = std::env::var_os("VIBE_G4_EVIDENCE_DIR") {
         fs::create_dir_all(&directory).unwrap();
-        fs::write(PathBuf::from(directory).join(format!("{name}.json")),
-            serde_json::to_vec_pretty(&value).unwrap()).unwrap();
+        fs::write(
+            PathBuf::from(directory).join(format!("{name}.json")),
+            serde_json::to_vec_pretty(&value).unwrap(),
+        )
+        .unwrap();
     }
 }

Diff in /fast/git/v/vibevm-next/crates/vibe-workspace/src/vibedeps/m_06_g_4_cases.rs:278:
 fn native_windows_component_limit_refuses_new_slot_before_effect() {
     let temp = TempDir::new().unwrap();
     let project = vibe_safefs::Project::open(temp.path()).unwrap();
-    let limit = project.windows_maximum_component_length()
-        .expect("native volume component limit unavailable; proof remains open") as usize;
+    let limit = project
+        .windows_maximum_component_length()
+        .expect("native volume component limit unavailable; proof remains open")
+        as usize;
     // Independent on-volume control: legal ASCII at L succeeds; L+1 fails
     // with an actual OS result, without a newly created directory entry.
     let scratch = temp.path().join("control");
Diff in /fast/git/v/vibevm-next/crates/vibe-workspace/src/vibedeps/m_06_g_4_cases.rs:290:
     let native = fs::create_dir(&over_limit).unwrap_err();
     assert!(native.raw_os_error().is_some(), "{native}");
     assert_eq!(fs::read_dir(&scratch).unwrap().count(), 1);
-    assert_eq!(fs::read_dir(&scratch).unwrap().next().unwrap().unwrap().path(), at_limit);
+    assert_eq!(
+        fs::read_dir(&scratch)
+            .unwrap()
+            .next()
+            .unwrap()
+            .unwrap()
+            .path(),
+        at_limit
+    );
     fs::remove_dir(&at_limit).unwrap();

     let name = vibe_core::PackageName::parse("path-fixture").unwrap();
Diff in /fast/git/v/vibevm-next/crates/vibe-workspace/src/vibedeps/m_06_g_4_cases.rs:304:
     let (source, mut dep) = package();
     dep.group = group;
     dep.name = name.into();
-    write(source.path(), "vibe.toml", format!(
-        "[package]\ngroup = \"{}\"\nname = \"{}\"\nkind = \"flow\"\nversion = \"1.1.0\"\n", dep.group, dep.name
-    ).as_bytes());
+    write(
+        source.path(),
+        "vibe.toml",
+        format!(
+            "[package]\ngroup = \"{}\"\nname = \"{}\"\nkind = \"flow\"\nversion = \"1.1.0\"\n",
+            dep.group, dep.name
+        )
+        .as_bytes(),
+    );
     dep.manifest = Manifest::read(source.path().join("vibe.toml")).unwrap();
     write(source.path(), "payload", b"selected bytes");
     let source_manifest = fs::read(source.path().join("vibe.toml")).unwrap();
Diff in /fast/git/v/vibevm-next/crates/vibe-workspace/src/vibedeps/m_06_g_4_cases.rs:326:
     ] {
         assert!(error.contains(&expected), "missing {expected}: {error}");
     }
-    assert!(!dependency_parent.exists(), "new slot parent must remain absent");
+    assert!(
+        !dependency_parent.exists(),
+        "new slot parent must remain absent"
+    );
     assert!(!destination.exists());
     assert!(!destination.join(vibedeps::SLOT_RECORD_FILENAME).exists());
     assert!(!destination.join("payload").exists());
Diff in /fast/git/v/vibevm-next/crates/vibe-workspace/src/vibedeps/m_06_g_4_cases.rs:333:
-    assert_eq!(fs::read(temp.path().join("foreign-marker")).unwrap(), b"foreign");
-    assert_eq!(fs::read(source.path().join("vibe.toml")).unwrap(), source_manifest);
-    assert_eq!(fs::read(source.path().join("payload")).unwrap(), b"selected bytes");
+    assert_eq!(
+        fs::read(temp.path().join("foreign-marker")).unwrap(),
+        b"foreign"
+    );
+    assert_eq!(
+        fs::read(source.path().join("vibe.toml")).unwrap(),
+        source_manifest
+    );
+    assert_eq!(
+        fs::read(source.path().join("payload")).unwrap(),
+        b"selected bytes"
+    );
     assert_eq!(fs::read_dir(source.path()).unwrap().count(), 2);
     // A separately chosen, parser-valid shorter coordinate can use the same
     // physical source; neither shortening the root nor silently renaming is a fix.
Diff in /fast/git/v/vibevm-next/crates/vibe-workspace/src/vibedeps/m_06_g_4_cases.rs:340:
     write(retry_source.path(), "payload", b"selected bytes");
     let retry_ws = workspace(temp.path());
     apply(&retry_ws, &retry_dep).unwrap();
-    assert_eq!(fs::read(slot(&retry_ws, &retry_dep).join("payload")).unwrap(), b"selected bytes");
-    assert_eq!(fs::read(temp.path().join("foreign-marker")).unwrap(), b"foreign");
-    evidence("component-limit", serde_json::json!({
-        "limit": limit, "unit": "WCHAR/UTF-16 code units excluding NUL",
-        "query_basis": "GetVolumeInformationByHandleW on held Project directory",
-        "control_at_limit_created": true, "control_over_limit_os_code": native.raw_os_error(),
-        "control_over_limit_error": native.to_string(), "control_no_created_entry": true,
-        "destination": destination, "refusal": error, "new_slot_parent_absent": true,
-        "record_and_payload_absent": true, "source_and_foreign_preserved": true,
-        "explicit_shorter_coordinate_retry": true, "total_path_limit": null,
-    }));
+    assert_eq!(
+        fs::read(slot(&retry_ws, &retry_dep).join("payload")).unwrap(),
+        b"selected bytes"
+    );
+    assert_eq!(
+        fs::read(temp.path().join("foreign-marker")).unwrap(),
+        b"foreign"
+    );
+    evidence(
+        "component-limit",
+        serde_json::json!({
+            "limit": limit, "unit": "WCHAR/UTF-16 code units excluding NUL",
+            "query_basis": "GetVolumeInformationByHandleW on held Project directory",
+            "control_at_limit_created": true, "control_over_limit_os_code": native.raw_os_error(),
+            "control_over_limit_error": native.to_string(), "control_no_created_entry": true,
+            "destination": destination, "refusal": error, "new_slot_parent_absent": true,
+            "record_and_payload_absent": true, "source_and_foreign_preserved": true,
+            "explicit_shorter_coordinate_retry": true, "total_path_limit": null,
+        }),
+    );
 }


--- retained fmt-final.log ---

--- retained fmt-final.exit ---
0

Observed local source HEAD: 07af7fecd07869436df90e5b8466ad2f8f946874. Existing configured effective toolchain: rustc 1.101.0-nightly (c36f14571 2026-10-01), host x86_64-unknown-linux-gnu, cargo 1.101.0-nightly (f3865b2a4 2026-09-29), LLVM 23.1.1. No toolchain selection/version change was performed.
