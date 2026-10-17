M-05-F.1 continuation: Redbook variants in existing mutable 1.1.0

Human-attributed source implementation and advisory verification report; no acceptance event.
Observed HEAD efc6da9a04034dc280b4d5a0eabe8f8b4c723f97. Retained and adapted the existing 20-path WIP. Final patch contains 22 paths; the two added owner paths are resolvo_engine/request.rs and package-source/cells.rs, directly necessary for ordinary source-route safety after removing the inventory-wide refusal. No Git mutation, source push, PR, mirror, package/registry/release publication, nested delegation, models, installs, network fetch, global configuration change or Root32 API mutation occurred. Required artifact publication is content-only.

Criteria (advisory claims)
0 SATISFIED: actual Redbook source definition passes the production reader, B1/B2 and C2; explicit MUP and WAL each produce exactly one continuity closure, independent additions and positively guarded branch additions preserve declared effects and dormant intent. Both continuity selections conflict locally, independent WAL with Redbook MUP remains valid, and absent current authority/unauthorized defaults refuse. Normal production LocalSource construction, bind_request, FrozenSource, selected graph/effects, planner, original authored requested_record and selected dependency provenance were exercised with nonzero test counts.
1 SATISFIED: current Redbook identity remains 1.1.0. Root vibe.toml has no diff. No sibling package conflicts or unrelated source edits. No version increment or source/package publication.
These are producer claims for Root review, not accepted or verified evidence records.

Current authority and actual build environment
The continuation packet explicitly corrected stale prospective lease/config text. Used its exclusive /fast/zap-managed/2026-10-14-m10b1-correction/cargo-target and incumbent normal Linux GNU host configuration. Did not cross-compile VibeVM to musl or change/install toolchains. Invoked absolute /home/olegchir/.cargo/bin/cargo with --offline --locked -j2, RUSTUP_AUTO_INSTALL=0, CARGO_NET_OFFLINE=true, CARGO_BUILD_JOBS=2. Created /tmp/m05f1-continuation.6E5fKu under umask 0077; observed private mode 0700 and /tmp mode 1777. Split/musl helper runtime details are packet attribution, not independently queried or mutated runtime facts.

Review fixes and preserved laws
- The constrained reader distinguishes array-valued legacy feature profile from the explicit string marker. profile=[] follows the ordinary legacy decoder; unknown string profiles and malformed opt-in definitions still refuse. One arm extends the existing core compatibility case.
- Source admission keeps checked definitions in immutable C1 independently of demand, binds caller instances where actually present, and skips unbound inventory rows without fabricating domains, consumer identities or consent. Exact source receipt and full normalized-definition equality checks remain in place.
- Replaced the inventory-wide missing-definition rejection with C2 candidate-local NeedsSelection rows. Only known required declarations participate in an unbound rich candidate's SAT requirements; optional declarations never become required-all. Configured candidates with real inputs continue through the incumbent B2/C2 evaluator and exact shown-default authorization. A selected unbound rich candidate returns NeedsSelection before a result/effects graph.
- Ordinary source routes project the same known required dependencies, then explicitly refuse a selected rich candidate without current consumer/domain input before returned effects. Unused rich alternatives do not block ordinary routes. No rich profile is interpreted through legacy required-all semantics.
- Added one bounded technical-OR regression arm, shared by the existing resolver/install stories: an ordinary root excludes the Redbook alternative and selects f1-plain. C1 inventory demonstrably contains actual Redbook without a Redbook consumer instance. The configured route has a real ordinary consumer/definition; the legacy route has ordinary roots. Both resolve only f1-alternative + f1-plain. Selecting the unbound Redbook still refuses. This fixture authors no replacement Redbook definition; its ordinary empty-feature definition is lowered from its own real metadata.
- Corrected the shared fixture import for the resolver's own lib-test target using a parent crate alias. No new dependency/toolchain is needed.
- Updated only the existing permanent F1 profile/admission anchors to spell out the two preservation laws. Independent B1/B2/B3/C1/C2/C3/D1/D2/E laws remain intact. The original replacement is only Redbook's fixed continuity roster; later F2 consumer migration, global WAL/MUP conflicts and later publication horizons remain outside this task.

Final source-bound checks
All commands use the environment above. Test families remain 3/1/1, with bounded arms inside existing cases.
slot 0: cargo test --offline --locked -j2 -p vibe-core --lib manifest::package::features::m_05_f_1_cases
  exit 0; 3 passed, 0 failed, 651 filtered out (core-final.log).
slot 1: cargo test --offline --locked -j2 -p vibe-resolver --lib resolvo_engine::m_05_f_1_cases
  exit 0; 1 passed, 0 failed, 133 filtered out (resolver-corrected.log).
slot 2: cargo test --offline --locked -j2 -p vibe-install --test m_05_e_2_cases m_05_f_1_cases
  exit 0; 1 passed, 0 failed, 35 filtered out (install-corrected.log).
  Existing safe integration target src/lib.rs respects lib.test=false; it calls the actual external production InstallSource/plan crate. Compilation of other harness modules is inherent in this bound target; no unrelated test family was selected.
Additional focused preservation check, justified by changes to the candidate-conditioned solver:
  cargo test --offline --locked -j2 -p vibe-resolver --lib resolvo_engine::m_05_c_2_cases
  exit 0; 7 passed, 0 failed, 127 filtered out (c2-preservation.log).
slot 3: cargo fmt --all -- --check: exit 1, exactly the retained seven baseline files. All seven byte-identical to HEAD; no touched source appears. Scoped rustfmt --edition 2024 --config skip_children=true --check: exit 0 on 16 modified/new Rust files. Scoped success does not replace the failed global check.
slot 4: git diff --check exit 0; each new file also checked with git diff --no-index --check /dev/null PATH, expected exit 1 for differences and empty diagnostics. Full 22-path patch read-only git apply --reverse --check exit 0. Changed TOML, Cargo.lock and XML parse successfully.

Failure ledger (retained, not erased by later success)
- Initial core family exit 0, 3 passed. Initial resolver family exit 101 before tests (0 executed): shared fixture import could not resolve its own crate as vibe_resolver, E0433. Corrected the source import. Initial install family exit 0, 1 passed; this was a transitional build during continuation edits, not final-source evidence.
- First expanded resolver and install runs each executed 1 test and failed (0 passed, 1 failed, exit 101): fixture incorrectly used an empty consumer request, refused as UnsupportedRoute("empty consumer request"). Preserved that existing codec law and replaced the fixture input with a real ordinary consumer. Final corrected runs above passed.
- Global fmt remains failed on seven unchanged files. Original candidate's global fmt failures, no-index aggregation assertion error and all-platform offline metadata failure for android_system_properties v0.1.5 are retained in the historical report below. Actual host test runs now succeed; that prior all-platform metadata failure was not used as a host-build gate, and no full metadata/fetch/install was repeated.
- Exploratory rg with a nonexistent frozen* path returned a missing-path error; a patch application with rustfmt-stale context refused without mutation and was reapplied to current context. Neither was a functional test result.

Effects and limits
Source implementation and focused functional work are completed; no product install/apply was executed. Install evidence is the real prepared planner and source route, not durable application or universal CLI/service/MCP parity. Negative cases compare project/settings snapshots, preserve the authored manifest, and create no lockfile. No source change followed the final focused passing runs except none; only read-only checks and report/patch generation followed. Global formatting debt remains unresolved outside scope. Root retains semantic judgment, Git and acceptance; this report and artifact handles do not supply acceptance. No further build resource gap remains for these host checks.


Full retained report SHA256 d02e7469a2a085338c026cb356712dee05127014915f02a4b7f0681caa63f239. Complete original report/log bytes remain in the immutable campaign artifact repository.
