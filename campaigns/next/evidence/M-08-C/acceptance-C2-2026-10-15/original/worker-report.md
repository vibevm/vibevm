# Full M-08-C.2 implementation candidate — Unreviewed

Baseline HEAD ec310e2ed755e2f55140efb5172ea4cd77ca205a. C1 remains canonically accepted at revision731/work revision5, source57e6dd64e+043e5d11e with archive parent58886e9c5. Root count82/222 plus separate M03A2 safe stop is retained provenance, not edited/recounted here. Historical preflight pending/count81 statements are superseded. Retained B2/M14/M16 prerequisites and separate aggregate platform/release debt are unchanged.

## Implementation and host boundary

`vibe_lifecycle::generic` composes the entire injected-root C2 lifecycle. `GenericHost` owns retained SafeFS project/user/settings capabilities, a trusted scoped `GenericAuthority` provider and the explicitly populated complete protected-control categories. It obtains current selection/source and current principal/policy observations from that provider; neither comes from a submitted grant or descriptor. `capture` compares the actual selected enabled provider against the canonical C1 map and pins provider/version/digest, declaration/digest, source/digest and descriptor/digest. `admit_payload` is a separate explicit host action over owned File/Tree/JSON/TOML data. Captured and admitted fields are private, with no deserializer.

`plan` is read-only. It enumerates full tree members, rejects traversal/reserved components, physical/case aliases, duplicates, file/tree ancestors, whole-document/member conflicts and member-prefix overlaps. Protected grant/policy/producer-trust/publication-mode resources and their aliases/ancestors refuse. There is no universal hidden policy filename or persistent grant store. Project/user/settings roots must be disjoint. Descriptor requests cannot select settings state. Literal dots/slashes/case in logical member segments stay literal; only physical paths use SafeFS path identity. The plan exposes admitted payload and actual resulting document bytes including active launch content, not merely a digest. Duplicate JSON keys/scalar ancestors/invalid existing JSON refuse, preserving foreign values rather than dropping ambiguous data. TOML digest framing retains typed value distinctions; nonfinite TOML floats are explicitly outside admitted closed payload semantics.

`approve` separately issues a private immutable non-deserializable ExactGrant for the full typed plan. Plan identity includes complete payload, operations/resources/locks derivation, before snapshots, source identities, root identities, protected controls, principal/policy, incumbent receipts and state-home identity. A grant cannot authorize a different host, altered plan, root, operation, payload or expanded set. Missing/wrong/stale authority refuses before lock/state/receipt infrastructure. There is no public raw-path grant issuer, JSON marker authority or side table.

`apply` rechecks the grant/current provider, takes the incumbent deployment lock and deduplicated whole-document locks, and rereads destination snapshots and ownership under lock. It never refreshes or partially merges a stale plan. Generic logical ownership extends the existing receipt census in `ownership.rs`, preserving literal segmented keys. Existing `Transaction::apply` owns pending sidecar, intent, checkpoint, independent verification, receipt, sidecar promotion and intent retirement. `ClosedProvider` implements that existing protocol; no parallel filesystem, transaction, resolver, context or process engine was introduced.

New absent files use incumbent SafeFS create-new publication, so a racing foreign occupant is preserved. Existing documents use incumbent snapshot-guarded replacement preserving mode under the cooperating document lock. Complete member merges publish each physical document once. Any infrastructure, directory-creation or possibly-published error retains the actual effect boundary; an injected landed failure leaves the incumbent intent and pending sidecar visible for inspection. Arbitrary hostile-content atomic CAS/ABA is not claimed. C3 recovery/inverse/resource-set migration/revocation remains unstarted and explicitly refused. Existing exact owned resource sets can reconcile; changed sets require separate C3 authority. No upper CLI/MCP, Native Plan, vendor execution or wire/schema change was added. Existing vibe-bin admission restrictions are byte-preserved.

## Scope and questions

All new source is in the five authorized generic files, with lifecycle exports, additive state/ownership helpers and one PROP054 addition. Original C1/A2/B1 bytes remain exact after removing that addition. SafeFS production, C1 request/map, OpenCode, upper discovery, manifests/dependencies, formats/schemas/wire and task/campaign records are unchanged.

Matching Root approval `m08c2-generic-provider-error-1` authorizes only `MechanismError::GenericHost { reason }` and its scoped formatting. Existing error variants/behavior are untouched. This lets the actual DeployProvider protocol transport generic failures without pretending they are a legacy provider error; the public generic error retains the effect boundary. The local question channel is the explicitly selected workaround, not canonical question-protocol proof.

Matching Root approval `m08c2-v4-native-path-fixture-2` authorized only the selected SafeFS component test's native-path fixture correction and one same-prefix sibling assertion. The original actual V4 failure (exit101, 0 passed/1 failed) remains archived. No SafeFS production primitive changed. The corrected two selected V4 cases passed. Root explicitly said the additional non-UTF8 case was unnecessary for this test-only amendment; it was not run. The local verify-group plumbing accepts these two exact approved source additions for V6 without editing canonical task/check/acceptance data.

## Original C2 obligations, preserved literally

Steps:
1. Implement contained file/tree projection and typed JSON/TOML member reconciliation as closed host semantics.
2. Expand symbolic roots and member paths to exact resources; check duplicate/ancestor overlap, case aliases and physical containment before grant issuance.
3. Bind grants to plan/descriptor/operation/resources/root/principal/policy generation; prohibit adapter writes to grant/policy/producer-trust/publication-mode control stores.

Positive cases:
- A granted new relative destination and new config member reconcile successfully.
- Foreign neighboring values and files remain unchanged.

Negative cases:
- Traversal, alias overlap, self-grant policy edits or broadened resources refuse.
- Active MCP launch content is visible in the prepared plan and does not inherit execution approval from a hash.

Criteria in capture order:
1. Positive fixture requires a real issued exact grant; removing it prevents writes.
2. Physical whole-document locks coexist with logical member ownership.

Safe stop: Generic host operations and grant refusal matrix pass in injected roots.

## Observed evidence versus limits

The actual V1 family contains twelve nonignored Rust cases, bound before its first compilation in CASE-BINDINGS.json. Table/subfixture rows are not counted as tests. The four-shape fixture issues a real grant, first removes it and observes an unchanged census, then applies it and checks all projected files and foreign JSON/TOML/tree neighbors. It inspects the actual finalized receipt, committed sidecar and absent intent. The contention case holds the actual shared physical lock in one thread, observes the apply thread blocked, releases it, then verifies literal case-distinct members and a separately owned sibling member coexist. Ancestor ownership refuses.

Other actual cases cover map-only non-authority, separate payload admission, absent/wrong/expanded grants, plan/descriptor/operation/root/state-home/principal/policy/provider/source/selected/disabled drift, traversal/reserved/physical aliases/ancestors, every supplied protected category, visible active launch content, source/destination/ownership drift under lock, equal foreign bytes, invalid/scalar/duplicate-key documents, a late foreign occupant at create-new publication, and truthful pre/post-publication failure boundaries. Fixture failure injection uses existing SafeFS test support; production SafeFS primitives are unchanged.

Saved census pairs bind actual before/after path membership and file hashes for pure planning/approval and pre-effect refusals. They are observed finite directory/byte censuses, not syscall/ABA proofs. The launch marker remains absent after writing explicitly approved command content. Absence of a process capability/call path is structural source evidence; no fabricated syscall process/network counter is reported. All fixture roots and payloads are owned/inert and remain under the private TMPDIR. Fixture creation and locked-race injection are explicit local test effects, outside the no-effect assertions. Deliberately retained failure intents exist only in disposable fixtures, subsequently cleaned by TempDir.

Local effect_state is completed: source/spec edits, builds/test artifacts, fixture setup/mutations/cleanup and evidence writes actually happened. There was no real HOME/vendor/client/network execution, no task acceptance/count change, Git mutation, public action, recursive worker, or next-task work. This is finite Linux injected-root evidence; native Windows/macOS execution and aggregate release/platform clearance are not claimed. Root alone reviews, accepts and commits.

## Actual verification and command history

Effective rustc1.101.0-nightly c36f14571 / cargo1.101.0-nightly f3865b2a4; edition2024. Only the authorized private warm target and TMPDIR were used, offline/locked. No cold target, target copy/clean, toolchain install/network, broad suite, --list, Lean, CLI rebuild, codegen or global gate was run. Initial target inspection found no cargo/rustc/rustdoc writer.

Final V1: 12/12; V3: 1/1; V4: 1/1 + 1/1; V5: 1/1. All final test commands exit0 and ignored0: 16 distinct Rust cases total. V2 is not applicable because no projection module was introduced, not an executed pass. V6 rustfmt and git diff check separately exit0. V7 XML parse/preservation passes, with an additional exact byte-reconstruction proof. Four V1 attempts each passed12; repeats are not added to the distinct count. V4 originally had first case1/1 exit0 and second0/1 exit101; that failure is retained, not silently replaced or relabeled reuse. Verification summaries below bind each actual log and exact argv.

V1 was rerun after actual implementation changes. Its final source is unchanged since that pass. The subsequent SafeFS test-only fixture amendment is compiled only in the SafeFS lib-test target, not in the SafeFS dependency used by lifecycle; SafeFS production bytes remain exact. V3/V5 reuse their actual recorded passes on unchanged affected bodies; no new execution is claimed beyond the command ledger. V6 is repeated on final authored paths, including the two exact Root-approved additions.

Final pure/pre-effect assertions saved 57 before/after census pairs, all equal. Historical attempt censuses remain separate. Final source/fixture/spec bytes are in worker-patch-manifest.json and source-observations.json; fixtures are the owned Rust data in generic/tests.rs, with no optional committed fixture directory.

```json
[
  {
    "argv": [
      "cargo",
      "test",
      "--offline",
      "--locked",
      "-p",
      "vibe-lifecycle",
      "--lib",
      "mechanism::deploy::generic::tests"
    ],
    "exit": 0,
    "counts": [
      [
        "12",
        "0",
        "0"
      ]
    ],
    "count_matches": true,
    "log": "/fast/zap-managed/2026-10-15-m08c2/V1-1.log",
    "seconds": 20.110810601036064,
    "passed": true,
    "group": "V1",
    "result_file": "attempt-1-V1-result.json",
    "saved_log": "attempt-1-V1-1.log",
    "log_sha256": "712563c102f74ac1da1907b4bc285f8a1dc57eebacd08007b806aa3bd0b2a454",
    "actual_test_summaries": [
      [
        "12",
        "0",
        "0"
      ]
    ]
  },
  {
    "argv": [
      "cargo",
      "test",
      "--offline",
      "--locked",
      "-p",
      "vibe-safefs",
      "--lib",
      "component::tests::complete_sets_share_the_identity_and_literal_path_laws",
      "--",
      "--exact"
    ],
    "exit": 0,
    "counts": [
      [
        "1",
        "0",
        "0"
      ]
    ],
    "count_matches": true,
    "log": "/fast/zap-managed/2026-10-15-m08c2/V4-1.log",
    "seconds": 1.4761207650299184,
    "passed": true,
    "group": "V4",
    "result_file": "attempt-1-V4-result.json",
    "saved_log": "attempt-1-V4-1.log",
    "log_sha256": "a1c3b1cf14bd87deb86ac15cfc2e08d440ec514cb2a01369d15ad6369c675129",
    "actual_test_summaries": [
      [
        "1",
        "0",
        "0"
      ]
    ]
  },
  {
    "argv": [
      "cargo",
      "test",
      "--offline",
      "--locked",
      "-p",
      "vibe-safefs",
      "--lib",
      "component::tests::overlap_and_containment_use_the_same_component_boundaries",
      "--",
      "--exact"
    ],
    "exit": 101,
    "counts": [],
    "count_matches": false,
    "log": "/fast/zap-managed/2026-10-15-m08c2/V4-2.log",
    "seconds": 0.15019343799212947,
    "passed": false,
    "group": "V4",
    "result_file": "attempt-1-V4-result.json",
    "saved_log": "attempt-1-V4-2.log",
    "log_sha256": "61b61e09d6569b4eeeb8d10cbcac9cb1f0e8b147850b68d0ea92b3c33f0fdbb6",
    "actual_test_summaries": [
      [
        "0",
        "1",
        "0"
      ]
    ]
  },
  {
    "argv": [
      "rustfmt",
      "--edition",
      "2024",
      "--config",
      "skip_children=true",
      "--check",
      "crates/vibe-lifecycle/src/lib.rs",
      "crates/vibe-lifecycle/src/mechanism.rs",
      "crates/vibe-lifecycle/src/mechanism/deploy.rs",
      "crates/vibe-lifecycle/src/mechanism/deploy/generic.rs",
      "crates/vibe-lifecycle/src/mechanism/deploy/generic/apply.rs",
      "crates/vibe-lifecycle/src/mechanism/deploy/generic/grant.rs",
      "crates/vibe-lifecycle/src/mechanism/deploy/generic/plan.rs",
      "crates/vibe-lifecycle/src/mechanism/deploy/generic/tests.rs",
      "crates/vibe-lifecycle/src/mechanism/deploy/ownership.rs",
      "crates/vibe-lifecycle/src/mechanism/deploy/state.rs",
      "crates/vibe-lifecycle/src/mechanism/error.rs"
    ],
    "exit": 0,
    "counts": [],
    "count_matches": null,
    "log": "/fast/zap-managed/2026-10-15-m08c2/V6-1.log",
    "seconds": 0.06009893404552713,
    "passed": true,
    "group": "V6",
    "result_file": "attempt-1-V6-result.json",
    "saved_log": "attempt-1-V6-1.log",
    "log_sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "actual_test_summaries": []
  },
  {
    "argv": [
      "git",
      "diff",
      "--check"
    ],
    "exit": 0,
    "counts": [],
    "count_matches": null,
    "log": "/fast/zap-managed/2026-10-15-m08c2/V6-2.log",
    "seconds": 0.016286358004435897,
    "passed": true,
    "group": "V6",
    "result_file": "attempt-1-V6-result.json",
    "saved_log": "attempt-1-V6-2.log",
    "log_sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "actual_test_summaries": []
  },
  {
    "argv": [
      "cargo",
      "test",
      "--offline",
      "--locked",
      "-p",
      "vibe-lifecycle",
      "--lib",
      "mechanism::deploy::generic::tests"
    ],
    "exit": 0,
    "counts": [
      [
        "12",
        "0",
        "0"
      ]
    ],
    "count_matches": true,
    "log": "/fast/zap-managed/2026-10-15-m08c2/V1-1.log",
    "seconds": 7.806657552020624,
    "passed": true,
    "group": "V1",
    "result_file": "attempt-2-V1-result.json",
    "saved_log": "attempt-2-V1-1.log",
    "log_sha256": "2f8225b4be4bdf49f553f21d970f57c37866c17cd1c61ebf61a788706e510383",
    "actual_test_summaries": [
      [
        "12",
        "0",
        "0"
      ]
    ]
  },
  {
    "argv": [
      "rustfmt",
      "--edition",
      "2024",
      "--config",
      "skip_children=true",
      "--check",
      "crates/vibe-lifecycle/src/lib.rs",
      "crates/vibe-lifecycle/src/mechanism.rs",
      "crates/vibe-lifecycle/src/mechanism/deploy.rs",
      "crates/vibe-lifecycle/src/mechanism/deploy/generic.rs",
      "crates/vibe-lifecycle/src/mechanism/deploy/generic/apply.rs",
      "crates/vibe-lifecycle/src/mechanism/deploy/generic/grant.rs",
      "crates/vibe-lifecycle/src/mechanism/deploy/generic/plan.rs",
      "crates/vibe-lifecycle/src/mechanism/deploy/generic/tests.rs",
      "crates/vibe-lifecycle/src/mechanism/deploy/ownership.rs",
      "crates/vibe-lifecycle/src/mechanism/deploy/state.rs",
      "crates/vibe-lifecycle/src/mechanism/error.rs"
    ],
    "exit": 0,
    "counts": [],
    "count_matches": null,
    "log": "/fast/zap-managed/2026-10-15-m08c2/V6-1.log",
    "seconds": 0.06115254800533876,
    "passed": true,
    "group": "V6",
    "result_file": "attempt-2-V6-result.json",
    "saved_log": "attempt-2-V6-1.log",
    "log_sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "actual_test_summaries": []
  },
  {
    "argv": [
      "git",
      "diff",
      "--check"
    ],
    "exit": 0,
    "counts": [],
    "count_matches": null,
    "log": "/fast/zap-managed/2026-10-15-m08c2/V6-2.log",
    "seconds": 0.015877127007115632,
    "passed": true,
    "group": "V6",
    "result_file": "attempt-2-V6-result.json",
    "saved_log": "attempt-2-V6-2.log",
    "log_sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "actual_test_summaries": []
  },
  {
    "argv": [
      "cargo",
      "test",
      "--offline",
      "--locked",
      "-p",
      "vibe-lifecycle",
      "--lib",
      "mechanism::deploy::generic::tests"
    ],
    "exit": 0,
    "counts": [
      [
        "12",
        "0",
        "0"
      ]
    ],
    "count_matches": true,
    "log": "/fast/zap-managed/2026-10-15-m08c2/V1-1.log",
    "seconds": 7.862363560998347,
    "passed": true,
    "group": "V1",
    "result_file": "attempt-3-V1-result.json",
    "saved_log": "attempt-3-V1-1.log",
    "log_sha256": "f6b30cc08736d5651d7363b98a23f99bf75308c8a4fce336afcfdb56ad5f7ffb",
    "actual_test_summaries": [
      [
        "12",
        "0",
        "0"
      ]
    ]
  },
  {
    "argv": [
      "cargo",
      "test",
      "--offline",
      "--locked",
      "-p",
      "vibe-lifecycle",
      "--lib",
      "mechanism::deploy::generic::tests"
    ],
    "exit": 0,
    "counts": [
      [
        "12",
        "0",
        "0"
      ]
    ],
    "count_matches": true,
    "log": "/fast/zap-managed/2026-10-15-m08c2/V1-1.log",
    "seconds": 5.817872229963541,
    "passed": true,
    "group": "V1",
    "result_file": "V1-result.json",
    "saved_log": "V1-1.log",
    "log_sha256": "f3cf74f09155e009a27cfc4989875a1235a21f268f50d89bd96258b732eecd9b",
    "actual_test_summaries": [
      [
        "12",
        "0",
        "0"
      ]
    ]
  },
  {
    "argv": [
      "cargo",
      "test",
      "--offline",
      "--locked",
      "-p",
      "vibe-lifecycle",
      "--lib",
      "mechanism::deploy::preplan_tests::a_shared_physical_lock_needs_reference_ownership_from_every_participant",
      "--",
      "--exact"
    ],
    "exit": 0,
    "counts": [
      [
        "1",
        "0",
        "0"
      ]
    ],
    "count_matches": true,
    "log": "/fast/zap-managed/2026-10-15-m08c2/V3-1.log",
    "seconds": 0.2980105889728293,
    "passed": true,
    "group": "V3",
    "result_file": "V3-result.json",
    "saved_log": "V3-1.log",
    "log_sha256": "19a475de025de5145ec753ef43aac1eef52983308a6e45b933ceb28006e860e6",
    "actual_test_summaries": [
      [
        "1",
        "0",
        "0"
      ]
    ]
  },
  {
    "argv": [
      "cargo",
      "test",
      "--offline",
      "--locked",
      "-p",
      "vibe-safefs",
      "--lib",
      "component::tests::complete_sets_share_the_identity_and_literal_path_laws",
      "--",
      "--exact"
    ],
    "exit": 0,
    "counts": [
      [
        "1",
        "0",
        "0"
      ]
    ],
    "count_matches": true,
    "log": "/fast/zap-managed/2026-10-15-m08c2/V4-1.log",
    "seconds": 0.9165939380181953,
    "passed": true,
    "group": "V4",
    "result_file": "V4-result.json",
    "saved_log": "V4-1.log",
    "log_sha256": "aab4594ac64211f7de8058773aa8cad83e257d2fa187c7c328ed265fcbe09e6e",
    "actual_test_summaries": [
      [
        "1",
        "0",
        "0"
      ]
    ]
  },
  {
    "argv": [
      "cargo",
      "test",
      "--offline",
      "--locked",
      "-p",
      "vibe-safefs",
      "--lib",
      "component::tests::overlap_and_containment_use_the_same_component_boundaries",
      "--",
      "--exact"
    ],
    "exit": 0,
    "counts": [
      [
        "1",
        "0",
        "0"
      ]
    ],
    "count_matches": true,
    "log": "/fast/zap-managed/2026-10-15-m08c2/V4-2.log",
    "seconds": 0.1639998439932242,
    "passed": true,
    "group": "V4",
    "result_file": "V4-result.json",
    "saved_log": "V4-2.log",
    "log_sha256": "635405e878958e7f29fe6bbd160a70b26255c60ff50bb0d7e433758766f1e037",
    "actual_test_summaries": [
      [
        "1",
        "0",
        "0"
      ]
    ]
  },
  {
    "argv": [
      "cargo",
      "test",
      "--offline",
      "--locked",
      "-p",
      "vibe-agent-projection",
      "--lib",
      "client_capabilities::m_08_c_1_cases::new_id_destination_and_key_are_map_data",
      "--",
      "--exact"
    ],
    "exit": 0,
    "counts": [
      [
        "1",
        "0",
        "0"
      ]
    ],
    "count_matches": true,
    "log": "/fast/zap-managed/2026-10-15-m08c2/V5-1.log",
    "seconds": 3.465395829989575,
    "passed": true,
    "group": "V5",
    "result_file": "V5-result.json",
    "saved_log": "V5-1.log",
    "log_sha256": "070dbc3648c7481f273fd3c57ef982172cc8d38e041beda828ef18966f96c89d",
    "actual_test_summaries": [
      [
        "1",
        "0",
        "0"
      ]
    ]
  },
  {
    "argv": [
      "rustfmt",
      "--edition",
      "2024",
      "--config",
      "skip_children=true",
      "--check",
      "crates/vibe-lifecycle/src/lib.rs",
      "crates/vibe-lifecycle/src/mechanism.rs",
      "crates/vibe-lifecycle/src/mechanism/deploy.rs",
      "crates/vibe-lifecycle/src/mechanism/deploy/generic.rs",
      "crates/vibe-lifecycle/src/mechanism/deploy/generic/apply.rs",
      "crates/vibe-lifecycle/src/mechanism/deploy/generic/grant.rs",
      "crates/vibe-lifecycle/src/mechanism/deploy/generic/plan.rs",
      "crates/vibe-lifecycle/src/mechanism/deploy/generic/tests.rs",
      "crates/vibe-lifecycle/src/mechanism/deploy/ownership.rs",
      "crates/vibe-lifecycle/src/mechanism/deploy/state.rs",
      "crates/vibe-lifecycle/src/mechanism/error.rs",
      "crates/vibe-safefs/src/component/tests.rs"
    ],
    "exit": 0,
    "counts": [],
    "count_matches": null,
    "log": "/fast/zap-managed/2026-10-15-m08c2/V6-1.log",
    "seconds": 0.061061357031576335,
    "passed": true,
    "group": "V6",
    "result_file": "V6-result.json",
    "saved_log": "V6-1.log",
    "log_sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "actual_test_summaries": []
  },
  {
    "argv": [
      "git",
      "diff",
      "--check"
    ],
    "exit": 0,
    "counts": [],
    "count_matches": null,
    "log": "/fast/zap-managed/2026-10-15-m08c2/V6-2.log",
    "seconds": 0.01565787399886176,
    "passed": true,
    "group": "V6",
    "result_file": "V6-result.json",
    "saved_log": "V6-2.log",
    "log_sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "actual_test_summaries": []
  },
  {
    "argv": [
      "rustfmt",
      "--edition",
      "2024",
      "--config",
      "skip_children=true",
      "crates/vibe-lifecycle/src/lib.rs",
      "crates/vibe-lifecycle/src/mechanism.rs",
      "crates/vibe-lifecycle/src/mechanism/deploy.rs",
      "crates/vibe-lifecycle/src/mechanism/deploy/generic.rs",
      "crates/vibe-lifecycle/src/mechanism/deploy/generic/apply.rs",
      "crates/vibe-lifecycle/src/mechanism/deploy/generic/grant.rs",
      "crates/vibe-lifecycle/src/mechanism/deploy/generic/plan.rs",
      "crates/vibe-lifecycle/src/mechanism/deploy/generic/tests.rs",
      "crates/vibe-lifecycle/src/mechanism/deploy/ownership.rs",
      "crates/vibe-lifecycle/src/mechanism/deploy/state.rs",
      "crates/vibe-lifecycle/src/mechanism/error.rs"
    ],
    "exit": 0,
    "result_file": "format-apply-result.json"
  },
  {
    "argv": [
      "rustfmt",
      "--edition",
      "2024",
      "--config",
      "skip_children=true",
      "crates/vibe-lifecycle/src/lib.rs",
      "crates/vibe-lifecycle/src/mechanism.rs",
      "crates/vibe-lifecycle/src/mechanism/deploy.rs",
      "crates/vibe-lifecycle/src/mechanism/deploy/generic.rs",
      "crates/vibe-lifecycle/src/mechanism/deploy/generic/apply.rs",
      "crates/vibe-lifecycle/src/mechanism/deploy/generic/grant.rs",
      "crates/vibe-lifecycle/src/mechanism/deploy/generic/plan.rs",
      "crates/vibe-lifecycle/src/mechanism/deploy/generic/tests.rs",
      "crates/vibe-lifecycle/src/mechanism/deploy/ownership.rs",
      "crates/vibe-lifecycle/src/mechanism/deploy/state.rs",
      "crates/vibe-lifecycle/src/mechanism/error.rs"
    ],
    "exit": 0,
    "result_file": "format-final-result.json"
  },
  {
    "argv": [
      "rustfmt",
      "--edition",
      "2024",
      "--config",
      "skip_children=true",
      "crates/vibe-lifecycle/src/lib.rs",
      "crates/vibe-lifecycle/src/mechanism.rs",
      "crates/vibe-lifecycle/src/mechanism/deploy.rs",
      "crates/vibe-lifecycle/src/mechanism/deploy/generic.rs",
      "crates/vibe-lifecycle/src/mechanism/deploy/generic/apply.rs",
      "crates/vibe-lifecycle/src/mechanism/deploy/generic/grant.rs",
      "crates/vibe-lifecycle/src/mechanism/deploy/generic/plan.rs",
      "crates/vibe-lifecycle/src/mechanism/deploy/generic/tests.rs",
      "crates/vibe-lifecycle/src/mechanism/deploy/ownership.rs",
      "crates/vibe-lifecycle/src/mechanism/deploy/state.rs",
      "crates/vibe-lifecycle/src/mechanism/error.rs"
    ],
    "exit": 0,
    "result_file": "format-publication-result.json"
  },
  {
    "argv": [
      "rustfmt",
      "--edition",
      "2024",
      "--config",
      "skip_children=true",
      "crates/vibe-safefs/src/component/tests.rs"
    ],
    "exit": 0,
    "result_file": "format-v4-result.json"
  }
]
```

## Exact source and evidence hashes

The scoped patch includes all new files without staging.

```json
{
  "patch": {
    "sha256": "91c48de21ccb406130ef7cd97c05c7b387b74f80ba8db0ec729c97e11d8779a3",
    "bytes": 93213
  },
  "changed_sources": {
    "crates/vibe-lifecycle/src/lib.rs": {
      "sha256": "4dc723a03b5bcd8a2806b88b638e5efa18f5d4e60696c2917208b9f06ba4b7b1",
      "bytes": 8258,
      "new": false
    },
    "crates/vibe-lifecycle/src/mechanism.rs": {
      "sha256": "713b2b4e7da5de1c7ad4b0656f17cf94efe37c9f63b72086e1a0d4dc19e02675",
      "bytes": 23651,
      "new": false
    },
    "crates/vibe-lifecycle/src/mechanism/deploy.rs": {
      "sha256": "13761900f7feb429887786dff6f12b24e0a47c24a31cc832d166c7d7b75e245d",
      "bytes": 30211,
      "new": false
    },
    "crates/vibe-lifecycle/src/mechanism/deploy/generic.rs": {
      "sha256": "7218bece02df41af201b1dd9b0fb0fb0d3042ecf5e81d2d83eff1f3fffdfc293",
      "bytes": 9756,
      "new": true
    },
    "crates/vibe-lifecycle/src/mechanism/deploy/generic/apply.rs": {
      "sha256": "a2c59b266c5b8093f76faede08e94911a17ad456d44308d6fa6cd1e6d652fc14",
      "bytes": 12426,
      "new": true
    },
    "crates/vibe-lifecycle/src/mechanism/deploy/generic/grant.rs": {
      "sha256": "8ebe972ad045aa9b790d4c8ea1a0bbaa059b56174e05b09e8ad5f2359356b062",
      "bytes": 2459,
      "new": true
    },
    "crates/vibe-lifecycle/src/mechanism/deploy/generic/plan.rs": {
      "sha256": "94ec04ed7c717954fa7a574fae482004d088b62844027370ef20f774ac259ff2",
      "bytes": 21124,
      "new": true
    },
    "crates/vibe-lifecycle/src/mechanism/deploy/generic/tests.rs": {
      "sha256": "947d6c9bd73989e930ab3773d6fde653a567a97ff586c09e042355685d7a5ee6",
      "bytes": 31339,
      "new": true
    },
    "crates/vibe-lifecycle/src/mechanism/deploy/ownership.rs": {
      "sha256": "ad9d8ca8c26465686968238df25b3e48dfeab78606c0e6d8db7648f98ce5fbbd",
      "bytes": 9164,
      "new": false
    },
    "crates/vibe-lifecycle/src/mechanism/deploy/state.rs": {
      "sha256": "2201530f8b05ecd835bccd5798c09910cf8799499feed350930d76666413f5ec",
      "bytes": 36180,
      "new": false
    },
    "crates/vibe-lifecycle/src/mechanism/error.rs": {
      "sha256": "f33532ebf551362c8e98d97c713c2d42076b195996bdc080371c05da88621377",
      "bytes": 23820,
      "new": false
    },
    "crates/vibe-safefs/src/component/tests.rs": {
      "sha256": "11dd896177117bcdea0d49b653af830eb2651e6138e65ba1663e71ebf75fd482",
      "bytes": 12234,
      "new": false
    },
    "vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml": {
      "sha256": "f250c99d77352c640e8335b275f4175e73a657faeb911d5ccfed3106ab26ca54",
      "bytes": 183397,
      "new": false
    }
  },
  "read_roster": {
    ".cargo/config.toml": {
      "sha256": "8cbe9a7044c66eab456264077660597b42efadf6da6f904d73d861764b3cf326",
      "bytes": 371,
      "baseline_sha256": "8cbe9a7044c66eab456264077660597b42efadf6da6f904d73d861764b3cf326",
      "unchanged": true
    },
    "campaigns/next/evidence/M-08-C/acceptance-C1-2026-10-15/MANIFEST.json": {
      "sha256": "e05df5660c52f46cd7aabb70556603ee59562bf031042da9ca3a52dbbeca087e",
      "bytes": 148414,
      "baseline_sha256": "e05df5660c52f46cd7aabb70556603ee59562bf031042da9ca3a52dbbeca087e",
      "unchanged": true
    },
    "campaigns/next/evidence/M-08-C/acceptance-C1-2026-10-15/V5-supplement/REPORT.md": {
      "sha256": "a3917a64cbe8cac5336ae120d749217f4d639e77e4afb22c65e82031db629e73",
      "bytes": 2482,
      "baseline_sha256": "a3917a64cbe8cac5336ae120d749217f4d639e77e4afb22c65e82031db629e73",
      "unchanged": true
    },
    "campaigns/next/evidence/M-08-C/acceptance-C1-2026-10-15/acceptance/accept-work-receipt.json": {
      "sha256": "74ccdc7e36032bc95e78549d3f3bba00586a335bc9dfb896c6130970f5c769d5",
      "bytes": 870,
      "baseline_sha256": "74ccdc7e36032bc95e78549d3f3bba00586a335bc9dfb896c6130970f5c769d5",
      "unchanged": true
    },
    "campaigns/next/evidence/M-08-C/acceptance-C1-2026-10-15/implementation/worker-report.md": {
      "sha256": "3b2c45a3a51f959480fa1056c622a92d4fc3f688c94ceb84f4e691d83cd7cee4",
      "bytes": 424175,
      "baseline_sha256": "3b2c45a3a51f959480fa1056c622a92d4fc3f688c94ceb84f4e691d83cd7cee4",
      "unchanged": true
    },
    "campaigns/next/evidence/M-08-C/acceptance-C1-2026-10-15/review/REPORT.md": {
      "sha256": "a618915e4f4d7435bcb79a841b6d9a715c09a5ecaf71833c1d89f045d45e023d",
      "bytes": 9147,
      "baseline_sha256": "a618915e4f4d7435bcb79a841b6d9a715c09a5ecaf71833c1d89f045d45e023d",
      "unchanged": true
    },
    "campaigns/next/evidence/M-08-C/prerequisites-2026-10-15.json": {
      "sha256": "3a9f5adfbcb681b50f7ba3cefc6df9cccc10405a2062d5cc0036c9df331ab1f4",
      "bytes": 7467,
      "baseline_sha256": "3a9f5adfbcb681b50f7ba3cefc6df9cccc10405a2062d5cc0036c9df331ab1f4",
      "unchanged": true
    },
    "campaigns/next/tasks/M-08-C.json": {
      "sha256": "b1d94cefddddca2e1e9ff9c4b70db420eb078c20c86e696562570283b856703b",
      "bytes": 37911,
      "baseline_sha256": "b1d94cefddddca2e1e9ff9c4b70db420eb078c20c86e696562570283b856703b",
      "unchanged": true
    },
    "campaigns/next/verification-policy.json": {
      "sha256": "0a3339b333551d5975a2cb384693886bb1432dd7bea63020f3d3eaded1bb5089",
      "bytes": 4411,
      "baseline_sha256": "0a3339b333551d5975a2cb384693886bb1432dd7bea63020f3d3eaded1bb5089",
      "unchanged": true
    },
    "crates/vibe-agent-projection/Cargo.toml": {
      "sha256": "e02b2762832f5bd6f17a773d59777bf7088c52a8e1161411dd54ce9c97e90152",
      "bytes": 1131,
      "baseline_sha256": "e02b2762832f5bd6f17a773d59777bf7088c52a8e1161411dd54ce9c97e90152",
      "unchanged": true
    },
    "crates/vibe-agent-projection/src/client_capabilities.rs": {
      "sha256": "89ae44f464ee0a89ddf1274dcc0a224919faecf7bd1b58dc59d50f3da9fca2b2",
      "bytes": 3941,
      "baseline_sha256": "89ae44f464ee0a89ddf1274dcc0a224919faecf7bd1b58dc59d50f3da9fca2b2",
      "unchanged": true
    },
    "crates/vibe-agent-projection/src/lib.rs": {
      "sha256": "c93970a3b66cb14d69a5f0d7cb48ad3cf3ea25ede507b65fa57846bbf416c0b9",
      "bytes": 2421,
      "baseline_sha256": "c93970a3b66cb14d69a5f0d7cb48ad3cf3ea25ede507b65fa57846bbf416c0b9",
      "unchanged": true
    },
    "crates/vibe-core/src/manifest/mechanism/client_request.rs": {
      "sha256": "001f40c3e91b08b4beb4dc496bddfcdcbb75fbbd54d4c970c4d4af047ab1ed91",
      "bytes": 5241,
      "baseline_sha256": "001f40c3e91b08b4beb4dc496bddfcdcbb75fbbd54d4c970c4d4af047ab1ed91",
      "unchanged": true
    },
    "crates/vibe-lifecycle/Cargo.toml": {
      "sha256": "714eba1c48ca1ab18930715070b0cef06ae528958bffe8f13dbb34a3ffbd10c4",
      "bytes": 2291,
      "baseline_sha256": "714eba1c48ca1ab18930715070b0cef06ae528958bffe8f13dbb34a3ffbd10c4",
      "unchanged": true
    },
    "crates/vibe-lifecycle/src/lib.rs": {
      "sha256": "4dc723a03b5bcd8a2806b88b638e5efa18f5d4e60696c2917208b9f06ba4b7b1",
      "bytes": 8258,
      "baseline_sha256": "9d2cdbfd74f8dfaaa4776d383e67730428e90c510ee74df5add7f3d46203f8c4",
      "unchanged": false
    },
    "crates/vibe-lifecycle/src/mechanism.rs": {
      "sha256": "713b2b4e7da5de1c7ad4b0656f17cf94efe37c9f63b72086e1a0d4dc19e02675",
      "bytes": 23651,
      "baseline_sha256": "26ea9f9143c9dc67461ab44130d164e36a9c314cc6a185fa2f7e8a9362bc83f5",
      "unchanged": false
    },
    "crates/vibe-lifecycle/src/mechanism/deploy.rs": {
      "sha256": "13761900f7feb429887786dff6f12b24e0a47c24a31cc832d166c7d7b75e245d",
      "bytes": 30211,
      "baseline_sha256": "7590e3faf444c217cd17d533500f249d214beffa8ab33e0419b7c0e9f9473596",
      "unchanged": false
    },
    "crates/vibe-lifecycle/src/mechanism/deploy/binding.rs": {
      "sha256": "a6d5beab875beffc06190a960ff4ae52144f42153a21622ae66150f508dc87f9",
      "bytes": 20990,
      "baseline_sha256": "a6d5beab875beffc06190a960ff4ae52144f42153a21622ae66150f508dc87f9",
      "unchanged": true
    },
    "crates/vibe-lifecycle/src/mechanism/deploy/binding_tests.rs": {
      "sha256": "3f15d0428c6302290e98471e11f186daac064af782c37ce31a102fe524aaebcb",
      "bytes": 12808,
      "baseline_sha256": "3f15d0428c6302290e98471e11f186daac064af782c37ce31a102fe524aaebcb",
      "unchanged": true
    },
    "crates/vibe-lifecycle/src/mechanism/deploy/bound_effect_tests.rs": {
      "sha256": "6040ab7aa238349dba361522c28956de5b4d591b837228fcc864e41c00e5bd3e",
      "bytes": 17459,
      "baseline_sha256": "6040ab7aa238349dba361522c28956de5b4d591b837228fcc864e41c00e5bd3e",
      "unchanged": true
    },
    "crates/vibe-lifecycle/src/mechanism/deploy/bridge.rs": {
      "sha256": "3e74500277d14f875cdab1697f686739c01a1918e8cf98ef45dab6d4e51ea80d",
      "bytes": 21160,
      "baseline_sha256": "3e74500277d14f875cdab1697f686739c01a1918e8cf98ef45dab6d4e51ea80d",
      "unchanged": true
    },
    "crates/vibe-lifecycle/src/mechanism/deploy/client_capabilities.rs": {
      "sha256": "a5ba340bdc827b6d7354f5c7b57394d0b680722bd2963798bb6448ec0db301ba",
      "bytes": 4545,
      "baseline_sha256": "a5ba340bdc827b6d7354f5c7b57394d0b680722bd2963798bb6448ec0db301ba",
      "unchanged": true
    },
    "crates/vibe-lifecycle/src/mechanism/deploy/error.rs": {
      "sha256": "16564f3ba82e30108a79c0ee900d799722819b0828c8476ea2a7eaf906e3dd2a",
      "bytes": 24755,
      "baseline_sha256": "16564f3ba82e30108a79c0ee900d799722819b0828c8476ea2a7eaf906e3dd2a",
      "unchanged": true
    },
    "crates/vibe-lifecycle/src/mechanism/deploy/model.rs": {
      "sha256": "6a944b4890337bb4bd2a620ab2095e4eb58fd8fb39efb423df125c5690d90e52",
      "bytes": 17989,
      "baseline_sha256": "6a944b4890337bb4bd2a620ab2095e4eb58fd8fb39efb423df125c5690d90e52",
      "unchanged": true
    },
    "crates/vibe-lifecycle/src/mechanism/deploy/ownership.rs": {
      "sha256": "ad9d8ca8c26465686968238df25b3e48dfeab78606c0e6d8db7648f98ce5fbbd",
      "bytes": 9164,
      "baseline_sha256": "7a8552fa8baab5f789aa85ae7406dad7d7e45ef9dc484e0f4aef1848426e0076",
      "unchanged": false
    },
    "crates/vibe-lifecycle/src/mechanism/deploy/plugin/opencode.rs": {
      "sha256": "be92ac78f6ef19f0404d67264a5aeb375f10d78bcf8cd67ab6ba35f0dd5d9eb5",
      "bytes": 19668,
      "baseline_sha256": "be92ac78f6ef19f0404d67264a5aeb375f10d78bcf8cd67ab6ba35f0dd5d9eb5",
      "unchanged": true
    },
    "crates/vibe-lifecycle/src/mechanism/deploy/preplan.rs": {
      "sha256": "0a7382ddcaa030fd54cb7e655f28d4a62ea642d94a95d987513e4c6adb1d4e6d",
      "bytes": 11905,
      "baseline_sha256": "0a7382ddcaa030fd54cb7e655f28d4a62ea642d94a95d987513e4c6adb1d4e6d",
      "unchanged": true
    },
    "crates/vibe-lifecycle/src/mechanism/deploy/preplan_tests.rs": {
      "sha256": "6926dcd1bb62b3817c2139a739d58c87dae73c55910eed56f7c499aed595a809",
      "bytes": 21542,
      "baseline_sha256": "6926dcd1bb62b3817c2139a739d58c87dae73c55910eed56f7c499aed595a809",
      "unchanged": true
    },
    "crates/vibe-lifecycle/src/mechanism/deploy/protocol.rs": {
      "sha256": "d0b4b847baac6ef0e61566c9d71664e7afa4e81c5dad5037e53e20171b9c5b15",
      "bytes": 28601,
      "baseline_sha256": "d0b4b847baac6ef0e61566c9d71664e7afa4e81c5dad5037e53e20171b9c5b15",
      "unchanged": true
    },
    "crates/vibe-lifecycle/src/mechanism/deploy/sidecar.rs": {
      "sha256": "ddc427d6ea76e33cd8aa1150b5bbe48f9fabef74eb57f0333c5acef2e144984e",
      "bytes": 23915,
      "baseline_sha256": "ddc427d6ea76e33cd8aa1150b5bbe48f9fabef74eb57f0333c5acef2e144984e",
      "unchanged": true
    },
    "crates/vibe-lifecycle/src/mechanism/deploy/sidecar_tests.rs": {
      "sha256": "5d3fa05b6a7a1026bde1c009e020563f35c2014eed7bc81206da0a0ea1bf4e92",
      "bytes": 22633,
      "baseline_sha256": "5d3fa05b6a7a1026bde1c009e020563f35c2014eed7bc81206da0a0ea1bf4e92",
      "unchanged": true
    },
    "crates/vibe-lifecycle/src/mechanism/deploy/state.rs": {
      "sha256": "2201530f8b05ecd835bccd5798c09910cf8799499feed350930d76666413f5ec",
      "bytes": 36180,
      "baseline_sha256": "64b8ec6c3f40b56ec304cb916e29cfd9856eef1a0e532fdf785b7944127b523a",
      "unchanged": false
    },
    "crates/vibe-lifecycle/src/mechanism/deploy/transaction.rs": {
      "sha256": "af143675841a5199639b4c8297f3762929529e7eeed090912728f6d3eb44f67d",
      "bytes": 37418,
      "baseline_sha256": "af143675841a5199639b4c8297f3762929529e7eeed090912728f6d3eb44f67d",
      "unchanged": true
    },
    "crates/vibe-orchestrator/src/machine_operation/adapter_plan/client_capabilities.rs": {
      "sha256": "6c9a125052702977c02e2e9781550a4102f52efd1c00e40ba03285dbe52eb8a2",
      "bytes": 11316,
      "baseline_sha256": "6c9a125052702977c02e2e9781550a4102f52efd1c00e40ba03285dbe52eb8a2",
      "unchanged": true
    },
    "crates/vibe-safefs/Cargo.toml": {
      "sha256": "619ede8f9c2fca016723f9136f7fc39755d7feecb3e87fa5c0164757b4026a4c",
      "bytes": 1227,
      "baseline_sha256": "619ede8f9c2fca016723f9136f7fc39755d7feecb3e87fa5c0164757b4026a4c",
      "unchanged": true
    },
    "crates/vibe-safefs/src/component.rs": {
      "sha256": "7a61118f9e2af8629e3868d6c016f74903a2c803659b71c3852459ff767e97f1",
      "bytes": 15779,
      "baseline_sha256": "7a61118f9e2af8629e3868d6c016f74903a2c803659b71c3852459ff767e97f1",
      "unchanged": true
    },
    "crates/vibe-safefs/src/component/tests.rs": {
      "sha256": "11dd896177117bcdea0d49b653af830eb2651e6138e65ba1663e71ebf75fd482",
      "bytes": 12234,
      "baseline_sha256": "79361baba91af4a82aff3e37c975d325fea68d1c6c189a0f6d4e5c3b84aa7aea",
      "unchanged": false
    },
    "crates/vibe-safefs/src/file.rs": {
      "sha256": "0e1dbc161f03d9f6a1930642dd0da1755f881e4992ae287db4810854d5933729",
      "bytes": 8487,
      "baseline_sha256": "0e1dbc161f03d9f6a1930642dd0da1755f881e4992ae287db4810854d5933729",
      "unchanged": true
    },
    "crates/vibe-safefs/src/file/project_methods.rs": {
      "sha256": "34a08356cf1fe1d4bcfee9a8e1fe0e9571d148fd6c0dbda9b88dcbb74477fd78",
      "bytes": 24840,
      "baseline_sha256": "34a08356cf1fe1d4bcfee9a8e1fe0e9571d148fd6c0dbda9b88dcbb74477fd78",
      "unchanged": true
    },
    "crates/vibe-safefs/src/lib.rs": {
      "sha256": "d067966d4092321b4f3381b233672be6ba027e267300842a3b9ee9a08b0816a7",
      "bytes": 3982,
      "baseline_sha256": "d067966d4092321b4f3381b233672be6ba027e267300842a3b9ee9a08b0816a7",
      "unchanged": true
    },
    "crates/vibe-safefs/src/project.rs": {
      "sha256": "1adb308a571c857d10b3f446bb9810577c9b2531a3d7f4ae98f1838a07022f1d",
      "bytes": 20469,
      "baseline_sha256": "1adb308a571c857d10b3f446bb9810577c9b2531a3d7f4ae98f1838a07022f1d",
      "unchanged": true
    },
    "vibevm/vibedeps/org.vibevm.world.fast-rust-compiler/1.1.0/vibevm/vibespecs/flows/fast-rust-compiler/FAST-RUST-COMPILER.xml": {
      "sha256": "3139b3a3b60a607e14868656c359c7d86bc3b77f43a139cf4184eefd62be556d",
      "bytes": 6718,
      "baseline_sha256": "3139b3a3b60a607e14868656c359c7d86bc3b77f43a139cf4184eefd62be556d",
      "unchanged": true
    },
    "vibevm/vibespecs/boot/90-user.xml": {
      "sha256": "377191548d742bb86f1fdd612814631e48a8de056e16781192da786e1440429d",
      "bytes": 24871,
      "baseline_sha256": "377191548d742bb86f1fdd612814631e48a8de056e16781192da786e1440429d",
      "unchanged": true
    },
    "vibevm/vibespecs/common/PROJECT-BOUNDARIES.xml": {
      "sha256": "3ee091653f61cba4c8d39e3482821c9ad3cfe304143b3888fd9b2e843e356bdb",
      "bytes": 6299,
      "baseline_sha256": "3ee091653f61cba4c8d39e3482821c9ad3cfe304143b3888fd9b2e843e356bdb",
      "unchanged": true
    },
    "vibevm/vibespecs/common/PROP-054-lifecycle-and-extensions.xml": {
      "sha256": "f250c99d77352c640e8335b275f4175e73a657faeb911d5ccfed3106ab26ca54",
      "bytes": 183397,
      "baseline_sha256": "cd594468e7b3cad5a5edf92855e01fcabe613426182f6fe7a60ecc03f47db2e5",
      "unchanged": false
    },
    "xtask/src/main.rs": {
      "sha256": "fba5174892210ae0720cb8e1a07f0cc178e88cee1ff1aa3a0600e131318ee60c",
      "bytes": 22699,
      "baseline_sha256": "fba5174892210ae0720cb8e1a07f0cc178e88cee1ff1aa3a0600e131318ee60c",
      "unchanged": true
    }
  },
  "spec_proof": {
    "exactly_one_added_line": true,
    "all_prior_bytes_exact": true,
    "old_sha256": "cd594468e7b3cad5a5edf92855e01fcabe613426182f6fe7a60ecc03f47db2e5",
    "new_sha256": "f250c99d77352c640e8335b275f4175e73a657faeb911d5ccfed3106ab26ca54",
    "addition_sha256": "d304d2939358d9c554d0848fbad0c6cf1e96d9b9cd0bef75f6ae597c2b38bc9d"
  },
  "artifacts": {
    "CASE-BINDINGS.json": {
      "sha256": "5387e70e5c5b6ad140b733475d7527c247c14f8aa253c7559c15c15d87c1665f",
      "bytes": 1503
    },
    "CENSUS-MANIFEST.json": {
      "sha256": "41004c6f6f7918faec93211280fc61630a02b7e80f2e6ee7a391ce12e1602d32",
      "bytes": 11680
    },
    "CHECK-SELECTION.json": {
      "sha256": "a3e8250b56d51c5b46a54ae10d08c5a9bf7a450b072b3ba50640e52ca0f16f94",
      "bytes": 537
    },
    "COMMAND-LEDGER.json": {
      "sha256": "fd4b5333728d831a409f149461a1c4eb031cceb8d60ebc57b72bc500d1be0989",
      "bytes": 15146
    },
    "ROOT-ANSWER.json": {
      "sha256": "f5652c92a530429d07d1c7c9c28f5d8d133082c1c9419a7af0d02277374d4e63",
      "bytes": 688
    },
    "ROOT-QUESTION.json": {
      "sha256": "7e0ad3f8c74b5a1ad7d9c5450e4dafd446e6a9afb528cf75e18b49088aabe53b",
      "bytes": 1467
    },
    "V1-1.log": {
      "sha256": "f3cf74f09155e009a27cfc4989875a1235a21f268f50d89bd96258b732eecd9b",
      "bytes": 20008
    },
    "V1-result.json": {
      "sha256": "abdb5549d825643c0a80d8b2212414372fe5155d5c8c27ac23070eb90ff50345",
      "bytes": 2056
    },
    "V2-result.json": {
      "sha256": "d1c56c40d0a2983bc8df2554f6bd451be2c3d241c26032e6a91a60294095d002",
      "bytes": 269
    },
    "V3-1.log": {
      "sha256": "19a475de025de5145ec753ef43aac1eef52983308a6e45b933ceb28006e860e6",
      "bytes": 18745
    },
    "V3-result.json": {
      "sha256": "0314fb5920f9d63652a3c1017f4796a4cbc85cdb5b889637c4803145b736191c",
      "bytes": 675
    },
    "V4-1.log": {
      "sha256": "aab4594ac64211f7de8058773aa8cad83e257d2fa187c7c328ed265fcbe09e6e",
      "bytes": 17948
    },
    "V4-2.log": {
      "sha256": "635405e878958e7f29fe6bbd160a70b26255c60ff50bb0d7e433758766f1e037",
      "bytes": 17874
    },
    "V4-result.json": {
      "sha256": "eedb9f4650d4c624d268f81804eb5e1b40659b6df095b4deb61107e1159864b7",
      "bytes": 1179
    },
    "V5-1.log": {
      "sha256": "070dbc3648c7481f273fd3c57ef982172cc8d38e041beda828ef18966f96c89d",
      "bytes": 18220
    },
    "V5-result.json": {
      "sha256": "7dcd2d2c2581acee00589734cd275dd3a2f3935e96eaaf4a5af705e1f423a8fe",
      "bytes": 652
    },
    "V6-1.log": {
      "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
      "bytes": 0
    },
    "V6-2.log": {
      "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
      "bytes": 0
    },
    "V6-PATHS.json": {
      "sha256": "1b64337aad155f2e32628d54235087e94b90c174cc70547c249c6eead6024b84",
      "bytes": 669
    },
    "V6-result.json": {
      "sha256": "a7934587e1ee50e155065d55b64a362eac2d4975d88a37b32b68281b4da6b7f9",
      "bytes": 1460
    },
    "V7-result.json": {
      "sha256": "e98e22229dd3122ea2d46bd9918e1d7475e9b6ee2350879ac94095423d8b2fa8",
      "bytes": 298
    },
    "additional-read-sources.json": {
      "sha256": "08a26488303d04e470c9ae88bcd1030b7a68e9117eb845a8bcb1808c2d72bbbc",
      "bytes": 1493
    },
    "approved-error-ROOT-ANSWER.json": {
      "sha256": "c4e6fab8445bc697ae94ddac61b6a0cc4f08390d847125714dfbb4158ebaca70",
      "bytes": 921
    },
    "approved-error-ROOT-QUESTION.json": {
      "sha256": "a076dea2e2747b76605615373e4a30a763d3024b52caa3dd8b68423e1b60318f",
      "bytes": 762
    },
    "attempt-1-V1-1.log": {
      "sha256": "712563c102f74ac1da1907b4bc285f8a1dc57eebacd08007b806aa3bd0b2a454",
      "bytes": 21060
    },
    "attempt-1-V1-result.json": {
      "sha256": "da59ab9ba27f6171530ba26461a24d72c06f7c683438e43169d9c63ad3da718f",
      "bytes": 2057
    },
    "attempt-1-V4-1.log": {
      "sha256": "a1c3b1cf14bd87deb86ac15cfc2e08d440ec514cb2a01369d15ad6369c675129",
      "bytes": 17955
    },
    "attempt-1-V4-2.log": {
      "sha256": "61b61e09d6569b4eeeb8d10cbcac9cb1f0e8b147850b68d0ea92b3c33f0fdbb6",
      "bytes": 18424
    },
    "attempt-1-V4-result.json": {
      "sha256": "82ed2a58fe06c772e4b978ac0e95f7d8eed2a61eed3944950423992998efb197",
      "bytes": 1114
    },
    "attempt-1-V6-1.log": {
      "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
      "bytes": 0
    },
    "attempt-1-V6-2.log": {
      "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
      "bytes": 0
    },
    "attempt-1-V6-result.json": {
      "sha256": "4ee23d5faddeaefb67bc8203b92ad1dacbce1abf683fdf48f61604328c73010e",
      "bytes": 1407
    },
    "attempt-2-V1-1.log": {
      "sha256": "2f8225b4be4bdf49f553f21d970f57c37866c17cd1c61ebf61a788706e510383",
      "bytes": 20008
    },
    "attempt-2-V1-result.json": {
      "sha256": "c9197fac1b306fb90c9beb0fc17e0d9d76b76fc4a4fa8b930e562973f7ce2b61",
      "bytes": 2056
    },
    "attempt-2-V6-1.log": {
      "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
      "bytes": 0
    },
    "attempt-2-V6-2.log": {
      "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
      "bytes": 0
    },
    "attempt-2-V6-result.json": {
      "sha256": "0ea7898f1be4f583bb381d3a2af88ed38dffdb290b0b6944d268e4880b91ee92",
      "bytes": 1407
    },
    "attempt-3-V1-1.log": {
      "sha256": "f6b30cc08736d5651d7363b98a23f99bf75308c8a4fce336afcfdb56ad5f7ffb",
      "bytes": 20008
    },
    "attempt-3-V1-result.json": {
      "sha256": "f98bae8d01a69b05dbb59b5e922557a98e0f556e585585158ded820d63898d04",
      "bytes": 2056
    },
    "effective-environment.json": {
      "sha256": "ad85467d6d6ff94f7232f0e4d86651f3ec5a6a1b6c71f0b6651702f164dcfcc1",
      "bytes": 1046
    },
    "format-apply-result.json": {
      "sha256": "e4f8816234a309ee8a26ed5693bbd38da41e5735057505f7dc1447445ce83594",
      "bytes": 759
    },
    "format-final-result.json": {
      "sha256": "e4f8816234a309ee8a26ed5693bbd38da41e5735057505f7dc1447445ce83594",
      "bytes": 759
    },
    "format-publication-result.json": {
      "sha256": "e4f8816234a309ee8a26ed5693bbd38da41e5735057505f7dc1447445ce83594",
      "bytes": 759
    },
    "format-v4-result.json": {
      "sha256": "5117ef3fcfc2fa62794d64f7358e286df5907a0b587f38ee797ccafe80547b15",
      "bytes": 167
    },
    "source-observations.json": {
      "sha256": "9682aeb0c45bd25fe2b09cc6143c305cba3301a60cffda69b424b8196ace7e1d",
      "bytes": 12865
    },
    "spec-byte-preservation.json": {
      "sha256": "154b88782f71bccb8c0a82c64c0e24bb07c0ddd4a515a5b254f555dff2b23ea3",
      "bytes": 327
    },
    "v4-fixture-proposed.patch": {
      "sha256": "f052609bdfe5d868c1ef6a72b6c6283f28a748c36493ce4a3a0511ad7d3902e2",
      "bytes": 1026
    },
    "verify-group-original.py": {
      "sha256": "af57f2039ee3dad03f68a27d8d4712952854aeb502dd050e3a437c33a466166c",
      "bytes": 4407
    },
    "verify-group.py": {
      "sha256": "1adbe98ac811548ffac8e1581d6cc2ece647cdd79973c5489d8cade51f60063d",
      "bytes": 4778
    },
    "worker-patch-manifest.json": {
      "sha256": "5f1d3e666e6193f6f3bd2b655fc7333d7e7bb98672ecdffbefaebd2c51690421",
      "bytes": 2366
    },
    "worker.patch": {
      "sha256": "91c48de21ccb406130ef7cd97c05c7b387b74f80ba8db0ec729c97e11d8779a3",
      "bytes": 93213
    }
  }
}
```

Both captured criteria are satisfied by the real issued-grant refusal/positive fixture and actual physical-lock/logical-ownership contention/coexistence proof. All eight finite obligations are covered. This candidate remains Unreviewed; no acceptance or next task is claimed.

Final read-only audit: all 13 final changed source hashes still match worker-patch-manifest.json; five files are new; the Git index delta is empty. No cargo/rustc/rustdoc process remained after checks. final-target-observation.json SHA256 d9f1e78855966eba0e3a20f08d23982cd706fe9254b2145fb14b7e455d8b9e73. No Git mutation was performed.
