# Full M-08-B.2 root-selection characterization — unreviewed candidate

Counterexample verdict: **no current product source-root defect reproduced**. Proposed subject: `test(adapters): characterize captured bridge roots through prepared Plan`. No fix is claimed and no production behavior is changed. The full characterization safe stop has been reached locally; Root alone owns independent review, integration and acceptance.

The patch changes four authorized authored Rust paths: lower test registration, four lower tests, three upper test entrypoints, and their upper fixture helper. It includes new files without staging. No capture.rs/binding.rs correction, upper production, schema/wire/generated/spec, metadata, acceptance archive, Git mutation, vendor/client/model execution, native Plan or delegated worker was introduced.

## Baseline and B1 continuity

HEAD is `8ff5e173506bbd28342043ef296b88777360a441`, following source/archive `4f55717b78d9bb38ea8899e1e02b874088ec8b02`. Initial READ-ROSTER validation matched all 185 hashes. The exact 186-input sourceBoundPreflight InputSha map recomputes to `f073b1ddd4b24508c4bdb9d3a1a048071f3f5de934f52c1ae503f16da409505b`; comparing the verified initial baseline leaves exactly the intentional B2 task metadata change. `worker-baseline-rebind.json` records both values, current commit and the separate metadata rebind. `worker-preflight-rebind.json` is the earlier post-authoring diagnostic and therefore also lists the two changed registration files; it is not an initial-baseline claim.

The standing boundary/user rules, full canonical B2 and B1 interface notes, named PROP054 preparation/capture/effects and executable/client-state continuity, PROP070 selected-visible/source membership, PROP080 route/wire/currentness, accepted B1 report and independent review were consulted. Accepted B1 executable/client-state semantics remain unchanged. Exact old projection bytes are reconstructed by removing only the new cfg(test) module registration; upper tests retain their exact old prefix. Remaining roster sources are hash-identical. See `worker-b1-preservation.json` and V5 subject bindings. No full boot, AGENTS or steward work was performed.

## What the actual cases establish

Lower V1 exercises `hydrate_declared_skill_using → binding::lower_one_binding` for direct embedded source and original installed identity, and `capture::prepare_with_ports` for retained closure/composition. Provider and selected-root same-relative-path decoys have different bytes. The admitted source root remains distinct from provider root, source paths are exact, includes select complete nested membership, excluded paths are omitted, expected ordered snapshot digest is recomputed, and targets remain selected-project Claude skills. The resource fixture uses the admitted local adapter plus embedded resource form. Lower injection counters observe one resource resolution/one composition publication during preparation and no additional calls across queries; direct hydration performs no composition. Missing, changed, member and composed-output drift refuse as SourceCaptureStale with unchanged inventories. Installed group/name/version/content hash and original provider root remain separate from the hydrated root.

Upper V2 actually constructs `AdapterPlanHost::new(explicit selected root, Skills/Project)`, invokes `prepare(true)`, builds `AdapterPlanRoute::new`, and requests Plan. It traverses the incumbent SourceQueryService::prepare_host, world::prepared_presets, selected self/dependency provider join, and PreparedProjectSkills::plan. It never constructs a synthetic prepared session or bypasses upper preparation with loose bindings. Test inspection of the real prepared carrier checks original provider role/root/version/hash separately from embedded/composed source_root.

Each positive fixture has four rows: selected-direct, selected-composed, dependency-direct, dependency-composed. The installed dependency has a real copied materialization/slot record created by the existing workspace owner, exact selected lock row, and a portable matching LockedEmbeddedSource. Cache input uses the incumbent Tree1 computation and exact source.toml/complete/tree layout; the existing cache verifier checks it before host preparation. The seed writer is only fixture data, not a new validator. Stable embedded Tree1 hash is `sha256-tree/1:754fe606cb3d22d9b92a22d42fcf8e21af15c254693816c588ac5cd6376b6ec1`; the portable URL is example.invalid and no fetch is performed. Exact package hash, cache roots and composed roots for every preparation are saved in `worker-fixture-effects.json`.

Every successful public row checks provider coordinate, declaration, agent, scope, target, ownership=Unavailable, proof=Observed, complete ordered paths, byte lengths, per-file SHA256 and source_digest. No hash or pin is promoted to source-proven strength. Repeated Plan replies are exactly equal. Decoy mutations in provider, selected and unrelated cwd roots cannot redirect resolution; excluded upstream bytes may change without altering selected membership/digest. Explicit host root is checked against unrelated child cwd; wrong-root comparison refuses.

The upper negative case uses seven fresh preparation scenarios: selected direct member missing, selected resource bytes changed, new selected resource member, composed-output bytes changed, selected lock changed, original dependency manifest changed, and entire embedded cache tree absent. Each is queried twice and returns NeedsPreparation/SourceCaptureStale, never partial rows. None refreshes, repairs or falls back. All original positive/negative/acceptance/safe-stop rows are reproduced below in full.

## Actual effects and query measurements

The parent launches only the same inert regression binary with one exact test and a 45-second bound. Each child has owned HOME, TMPDIR, cwd, cache/config env and an empty executable PATH. No process-global env/chdir mutation occurs in concurrent test bodies. The existing vibe-test-support load-time constructor further relocates VIBE_SETTINGS before test threads; the fixture consumes the existing settings API and asserts the actual settings path remains inside the bounded child's owned tree. This constructor behavior was discovered in failed fixture attempt 3, not bypassed with a new authority.

Local effects did occur and are **completed**: authored source/evidence files, compilation/test artifacts in the handed-over warm target, temporary fixture directories/files, workspace dependency materialization/record/lock, cache seeds, intentional tampering, and two composition publications per preparation. Nine successful upper preparations record settings inventory growth from 20 to 40 entries, with exact before/after byte-inventory hashes. Between independent negative scenarios the fixture removes its owned prior composition cache and reseeds its owned embedded entry; this is explicit test setup outside query, never query repair. Parent TempDir cleanup removes each owned child fixture after exit. These setup effects are not labelled NoEffects.

There are 34 measured upper Plan intervals: 20 positive/repeated queries and 14 typed refusals. Four-root exact byte/directory inventories (scenario including provider/selected/materialization, actual settings/cache, HOME, unrelated cwd) are equal before/after every interval; their hashes and actual paths are retained in `worker-fixture-effects.json`. `worker-query-census.json` is derived from the actual strace V2 run and reports zero filesystem mutation/write-open, process, network or lock attempts in those intervals. Actual query syscall counts are 1,408 openat, 10,486 openat2, 20,576 statx and 32,392 fcntl; 68 writes are the marker label/newline writes to stderr, explicitly excluded as instrumentation. Setup/compiler/child-launch/teardown effects are outside the markers. This is finite Linux evidence for selected syscall families, not an atomic filesystem, hostile-ABA or other-platform guarantee. Lower cases additionally retain resolver/composer counters and complete owned-root inventories.

## Selected verification and all attempts

All seven actual new names and registrations were bound in worker-check-bindings.json before the first compilation. V1/V2 were TO CREATE at dispatch, not inherited passes. Compiler is repository override `rustc 1.101.0-nightly (c36f14571 2026-10-01)`. Cargo uses `--offline --locked`, handed-over target `/fast/zap-managed/2026-10-14-m10b1-correction/cargo-target`, TMPDIR `/fast/zap-managed/2026-10-15-m08b2/tmp`. ROOT-TARGET-HANDOVER authorized use after idle; no cold target, copy or clean was used.

- V1 final: exit 0, 4 passed, 0 failed, 0 ignored. Original attempt1 also passed 4 but had an overpermissive lower composition fixture; final V1 follows correction to the admitted local-adapter form.
- V2 attempt1: exit 101, no tests ran (two private-module references and one SourceUrl type inference error in new test code). Corrected to public RecipeId re-export and direct string argument.
- V2 attempt2: exit 101, 0 passed/3 failed. Manifest correctly refused the invalid direct-source-plus-resource fixture. Corrected fixture, not product.
- V2 attempt3: exit 101, 0 passed/3 failed. Existing test-support constructor relocated settings, so the seed was at the wrong fixture path; offline owner correctly refused. Corrected seed location through existing settings_dir with containment assertion.
- V2 attempt4: exit 0, 3 unique passed, 0 failed/ignored; 30 query intervals, before adding whole-cache absence and durable query inventory hashes.
- V2 final: exit 0, 3 unique passed, 0 failed/ignored; 34 query intervals. Wrapper raw counts contain three child 1-pass summaries plus the parent 3-pass summary; these are **three cases, not six distinct passes**.
- V3: two separate commands, each exit 0, 1 passed/0 failed/0 ignored: prepared_embedded_closure_uses_injected_provider_once_and_exact_offline; prepared_resources_preserve_includes_and_only_prepare_publishes_owned_cache.
- V4: two separate commands, each exit 0, 1 passed/0 failed/0 ignored: unprepared_and_foreign_bindings_have_closed_refusals; unknown_capture_and_unknown_fields_refuse_without_refresh.
- V5: wrapper exit 0 validating **reuse**, executed=false. The exact accepted B1 CLI/MCP parity cases, source hashes, old logs and independent review are bound in V5-SELECTION.json. No new pass is claimed. B2 has no upper production/public contract change. New test registration is accounted for separately by V1–V4 and exact B1 prefix equivalence.
- V6: exit 0; scoped rustfmt edition2024/skip_children check of exactly the four authored Rust paths in V6-PATHS.json.
- V7: exit 0; exact supplied git diff --check command. New files are also scoped-rustfmt checked and included in worker.patch via read-only git diff --no-index; nothing staged.

Every underlying command has its own log/exit/count in worker-command-results.json and the separate V*-result.json files. Failed and superseded attempts are retained as attemptN-*; they are not product reds or extra unique green cases. Source hashing/fixture census/trace analysis and patch creation are evidence operations, not additional verification plans. No broad suite/test-list or codegen oracle was run. Accepted B1's prior global oracle limitations remain historical and are not relabelled green.

No missing outside owner was encountered: no ROOT-QUESTION or outside edit was needed. No recursive worker, provider launcher, resolver, scheduler, context authority or acceptance action was added. Query captures grant no write authority. All selected commands have physically exited; the warm target remains in place for Root handback.

## Exact final source/fixture and artifact bindings

`worker-patch-manifest.json` binds all four final authored files; `worker-expected-membership.json` binds every projected fixture byte, length, per-file hash and ordered source snapshot. `worker-fixture-effects.json` binds runtime roots/settings inventories. `worker-artifacts.json` binds patch, reports' supporting JSON, individual logs, selected decisions and syscall trace. Runtime fixtures are generated by the hashed helper source and are cleaned up; no static fixture file or private archive was overwritten.

```json
{
  "crates/vibe-agent-projection/src/pkgskill/projection.rs": {
    "sha256": "ab85011cd156fe4ed65b0f62b227bcf4742536837262df05b3cc7ce8f07121bb",
    "bytes": 21597
  },
  "crates/vibe-agent-projection/src/pkgskill/projection/m_08_b_2_cases.rs": {
    "sha256": "70470810c02fda69264b7a1fa0a4c2baa3fd4fa617edef51c72d3c3dc673db18",
    "bytes": 11805
  },
  "crates/vibe-orchestrator/src/machine_operation/adapter_plan/tests.rs": {
    "sha256": "8b034c83344f37254dd0dfeec00bc088580f1a1a6766cd79c0b1a9da743b2765",
    "bytes": 8122
  },
  "crates/vibe-orchestrator/src/machine_operation/adapter_plan/m_08_b_2_fixture.rs": {
    "sha256": "0cd918a6c6dd6d6e10929c342a7cd4c304910ceb04c9fab5fbddbf54c9831871",
    "bytes": 19049
  }
}
```

Exact projected membership:

```json
{
  "direct": {
    "source_snapshot": "sha256:400278bcdf96f442dffea96a9934a09621549cac6a5099bdf3163871347b0d04",
    "files": {
      "SKILL.md": {
        "bytes": 4,
        "sha256": "230d8358dc8e8890b4c58deeb62912ee2f20357ae92a5cc861b98e68fe31acb5",
        "utf8": "body"
      },
      "references/deep/a.md": {
        "bytes": 13,
        "sha256": "4332eb1427ba11715d5faa046304070e4286c5a2a22f19486a0c522c59a57de6",
        "utf8": "nested source"
      }
    }
  },
  "composed": {
    "source_snapshot": "sha256:3c41b364e8e1eced61f5164848ec528a8538b9cf3b8dcc0a6cbe3c5114a67f5c",
    "files": {
      "SKILL.md": {
        "bytes": 4,
        "sha256": "230d8358dc8e8890b4c58deeb62912ee2f20357ae92a5cc861b98e68fe31acb5",
        "utf8": "body"
      },
      "references/deep/a.md": {
        "bytes": 13,
        "sha256": "4332eb1427ba11715d5faa046304070e4286c5a2a22f19486a0c522c59a57de6",
        "utf8": "nested source"
      },
      "references/vendor/deep/resource.md": {
        "bytes": 15,
        "sha256": "559bef277536042572778c4f5b7d8ca586a6fc074da08c4aed852a9538eb9fff",
        "utf8": "nested resource"
      }
    }
  }
}
```

## Preserved original B2 obligations

```json
{
  "id": "M-08-B.2",
  "title": "Resolve bridge sources against their captured root",
  "goal": "Fix any demonstrated hydrated source-root mismatch without inventing an unobserved bug.",
  "steps": [
    "Create a fixture where provider.root differs from the hydrated captured source root and resources include nested files.",
    "Trace actual resource resolution through automatic binding; record whether the suspected mismatch reproduces.",
    "If reproduced, carry the admitted source root through resource selection and include closure/digest checks; if not, retain the characterization proving the correct current path."
  ],
  "positive_cases": [
    "Captured nested resources project from the exact hydrated capture.",
    "An already-correct route remains byte-compatible."
  ],
  "negative_cases": [
    "Provider-relative fallback cannot read a different resource with the same name.",
    "Missing/changed captured resource gives an explicit refusal rather than a partial plugin."
  ],
  "acceptance": [
    "Evidence identifies the actual source root and verifies complete projected membership.",
    "No product fix is claimed unless the counterexample demonstrated the defect."
  ],
  "safe_stop": "Root-selection characterization passes; a demonstrated correction is isolated."
}
```
