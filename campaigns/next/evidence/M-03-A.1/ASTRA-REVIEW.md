# M-03-A.1 independent candidate review

Final bounded verdict: no remaining observed P1/P2 blocker in the inspected A.1 boundary. The specification-dialect blocker below is now resolved by the verified container-only correction and fresh Root gate. This is review evidence for Root, not acceptance and not a whole-product clean verdict.

Root's deciding gate found that appended top-level `<p>` clauses in PROP-054 line 617 and PROP-071 line 38 violate the actual spec dialect after top-level sections. Normal specmap exits 0 for generation despite losing all PROP-054 units and reporting 1716 unresolved host edges (HEAD: 0). Required correction is structural section wrapping of the same facts, followed by Root's actual dialect/specmap check and resealed document/source identities. This candidate cannot receive a final no-blocker verdict while that defect remains. Unchanged code observations below survive the document-only correction; the original source manifest remains the exact subject this report inspected.

### Final dialect addendum

The preceding paragraph records the historical pending cause, now closed. Compared `PRE-DIALECT-SOURCE-CANDIDATE.json` with current SOURCE-CANDIDATE.json: identical 162-path set; only PROP-054 and PROP-071 hashes changed. All 162 current filesystem hashes match the new manifest, SHA256 `0c2dd2e9d2a07fee55aa7cc3aabfe06702807e522d7c5021d45c08d9ff302338`. SOURCE-STOP records source writes stopped.

Independently removed exactly the two added container lines from each current document in memory. The resulting full bytes hash to the respective pre-correction document hashes (`be4f906058ba15dab39845f8589fc38491ac1a41beaf80370b4dd9299f51ee38` and `f0f8b94930b016a0f7cb12d62ff24f2d6212b4bd69550eeb305f001a1dd08edc`). Therefore all original document bytes, six PROP-054 fact blocks, one PROP-071 fact block and their anchors are unchanged. The sole additions are titled `invocation-effects` opening/closing containers.

Fresh `ROOT-M03-A1-DECIDING-SPECMAP.json` reports exit 0 and log SHA256 `c4fca73769cfc7f6552b18644ad85a0d6a327b03ae49341203677ded930f8022`; independently verified that log hash. The actual log reports 10481 spec units, 0 suspects, 138 gated orphans and 0 unresolved host edges, with 35 non-host edges outside jurisdiction. No dialect warning or other warning naming either corrected document appears. Root identifies the 138 orphans as inherited. The historical 1716-edge result remains pre-correction evidence, not the current verdict. No semantic code review or tests were repeated for this addendum; unchanged-code observations and coverage limits remain exactly as below. Root retains acceptance authority.

## Subject

Repository `/fast/git/v/vibevm-next`, HEAD `c9e5eac5803fe00e80c2fae4fbe3e23509bcd455`. Read SOURCE-STOP.json and SOURCE-CANDIDATE.json first. Manifest SHA256 `e3a1178a26c4cad8589591355142c5a8756e891cb36fe2829f6ce631501d0a90`; all 162 listed source hashes matched both at entry and immediately before this report. Root-owned FIXED/specmap changes are outside that producer manifest. All 164 EVIDENCE-MANIFEST producer file hashes also matched.

Standing scope: supplied quiet packet, AGENTS.md, boot/90-user.xml, parallel-execution.xml and verification-selection.xml; original M-03-A.1 task obligations and actual PROP-054 invocation/effect/completion/order anchors plus PROP-071 CHANGE-INVOCATION-EFFECTS. No boot/stewardship, delegation, build, test, provider, network, state mutation or repository write was performed.

## Source observations

- Authored effects are optional only for legacy syntax; present sets are closed, nonempty, sorted and reject duplicates. Mixed product-source retains production classification.
- Public preparation derives role order, and phase admission runs before prerequisite install. The admitted-current path skips install and consumes the retained registry. Phase rows, precise target-qualified slot rows including legacy hooks, compile declarations and selected build mechanisms are checked. Native source fallback compiler selection is checked for explicit create/full-pass. Compile selector scope and unavailable artifact-target verify proof return explicit need rather than dispatch by guessing.
- The shared runner refuses unknown effects before envelope/agent preparation. A disallowed producer cannot escape through force/Always or a fresh header: reuse needs original basis, selected node, completion digest, declaration/input witness equality and original attributed output witness equality. Changed input/output/effect/selected negatives exercise that branch.
- Completion digest is fed from the same prepared input walk as execution identity; it excludes requested/chain while retaining execution/root and incumbent subject material. Successful tracked execution records it; reuse retains original basis and output attribution. Current input observation may receive the current run id without replacing the producer output baseline.
- Explicit host full-pass uses validate/install/generate/create/build/test/verify. The evidence validator accepts logical full-pass only with that exact causal chain. Ordinary prepare and reserved-identity prepare both use the role chain; I found no hidden ordinary inclusive-verify branch in those owners.
- Frozen e1 schema and flat generated lifecycle_state/mod.rs are byte-identical to HEAD. The e2 schema differs structurally only by optional completion_basis and its closed definition; metadata names the separate home. Decode selects strict frozen e1 or current e2 before validation; e1 projection cannot supply completion basis and does not upgrade the epoch. Store upgrades only when retaining actual basis.
- Verify injects the no-build binary backend. Actual native invoke resolves current/prebuilt artifact and does not compile it. Declared effects remain a cooperative contract; no adversarial script analysis was used as a criterion.

## Executed evidence inspected (not executed by reviewer)

Read the actual test bodies, final CLI driver, per-process structured records, and selected raw/gzip-decoded logs. Every inspected selected log matched its RUNS.jsonl SHA256 and had the reported nonzero case selection:

- final public family `native-builder-classified-before-preparation`: 10 passed;
- current codec: 4 passed; connected runner family: 3 passed; independent framing: 1 passed;
- effect parser, no-lazy-tool, CLI parser and full-pass wire: 1 passed each;
- legacy declaration/execution and two affected command compatibility cases: 1 passed each; chain compatibility: 6 passed.

The final public log is the 10-case current native guard epoch. Earlier unchanged helper/compatibility logs remain historical compatible evidence as identified by producer/Root; this review did not rerun them or independently reconstruct every historical build input closure.

The final CLI driver calls the production binary, writes source via a script producer, invokes installed rustc to compile a real test binary, then runs that binary as the observer. It asserts exact `produce/compile/observe` order, actual `1 passed`, six successful child exits, twelve plan/result documents, four equal source hash pairs and four equal control inventories. Both default cwd and relative member selection are represented; force is supplied for build/verify. Records use binary `dbae2643e9fdaeff0e1d7a6dc9b459cb98053c72f483383ba0a1d7ae88f3313e`. Cold-create hermetic registry startup effects are disclosed, not counted as zero effects.

Read BOUND-PLAN, FINDINGS, INCIDENTS, CANDIDATE-REPORT, CLI-SMOKE-SUMMARY and CLI-SMOKE-DECIDING-NATIVE. Root specmap record has exit 0 and matching log SHA; its log still reports existing unresolved edges, so exit 0 is not a zero-debt assertion. Root owns that deciding gate.

## Exact inspection limit

All 162 manifest files were read for hash identity. Semantic inspection was full new admission/completion/effects/slot-plan/codec owners and new test bodies, changed hunks in connected owners, and bounded directly called helpers. It was not a line-by-line read of all 1.68 MB of unchanged generated/test bodies. In particular, this report does not claim full-file semantic coverage of every mechanical e1-to-e2 import/fixture or generated header file in the manifest.

Principal fully inspected owners: core manifest extension/effects.rs; lifecycle runner/invocation.rs and invocation/m_03_a_1_execution.rs; state/io.rs and validate.rs; orchestrator command/invocation.rs and command/m_03_a_1_cases.rs; phase/invocation.rs; world/invocation.rs; dispatch/runtime.rs and backends.rs; install/lifecycle/plan.rs. Inspected connected hunks/body regions: lifecycle chain.rs, runner.rs, runner/hosted.rs, runner/verify.rs and inputs.rs, fingerprint.rs, fingerprint/inputs.rs and declaration.rs, state/store.rs; orchestrator phase.rs, command.rs, dispatch/mod.rs and untracked.rs, world/mod.rs and dependencies.rs; core extension/mechanism wire; builtin mechanism descriptors; CLI lifecycle parser; verification-evidence run gate; e2 schema/registry and frozen schema/DTO identity. Direct helpers inspected include source-query prepare_host/retain_source_inventory forwarding and retained continuity, native candidate projection/source-group projection and native invoke.

Limits retained: artifact-target service verification freshness may be unavailable; hosted handoff satisfaction retains no invented original completion basis; later change-apply carrier work remains owed; inherited formatting and broader campaign debt remain separate. None is presented as a universal freshness or production-platform qualification. No actionable literal A.1 counterexample was found in the inspected paths.

Reviewer command incidents: `python` was unavailable (127), corrected to python3; a guessed frozen DTO filename ending in `.rs` did not exist in HEAD (128), corrected to actual `lifecycle_state/mod.rs`, which matched. No dependent mutation or execution followed either failed read.
