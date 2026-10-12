# M-01-E.2 candidate — awaiting First Agent acceptance

## Source boundary

Current HEAD/base `8187ba0dfc13fff26eacad4dc84f70354556ab64` (accepted E1). Nine changed paths in SOURCE-SHA256.json: tests, required test registrations/helper visibility and PROP070 `SOURCE-QUERY-PURITY-EVIDENCE`. No production query algorithm, CLI command, dependency, wire/schema/ABI or original-proof owner changed. Root-owned FIXED-ISSUES delta is disjoint and was not edited here. All product source remains unstaged; no next atom, commit, release or public push.

## Deciding evidence

| Final bound target | Nonignored result | Actual measured Linux query scope |
|---|---|---|
| vibe-workspace --lib source_query::capture::continuity::service::m_01_e_2_cases:: | 4/4, exit0 | 8 complete marked intervals |
| vibe-mcp --lib tools::read_spec::m_01_e_2_cases:: | 2/2, exit0 | 5 complete marked dispatcher/service intervals |
| vibe-cli --test m_01_e_2_cases | 3/3, exit0 | 22 complete intervals plus 24 CLI/stdio-MCP exec-to-exit scopes |

Nine new distinct passing cases, zero ignored. Early actual MCP→service representative1/1 and real CLI query are separate. Corrected descriptor/memory syscall observers run the existing final binaries without Cargo rebuild: all35 query intervals and24 actual exec-to-exit surface scopes have **0 network syscall attempts and 0 filesystem mutation/sync attempts**. Surface scopes begin at the observed initial vibe exec and end at exit, including post-exec startup. Split clone3 attribution misses pre-exec ancestry; 72 pre-exec dup2 and24 pre-exec chdir calls are outside these scopes. All 24 vibe execs are independently recognized and these traces have no descendants. Each process_spawn=1 is that scope’s own initial exec, not a new query spawn. Library/proof scopes exclude explicit fixture setup and deliberate tampering; transport/stdout/stderr pipe writes are expected delivery, not filesystem publication. Process-spawn counts are retained separately (surface entry is not silently erased). Full raw traces and counter scopes are in `*-full-oracle.strace` / corresponding `*-complete-oracle-counters.json`. Clean PATH/LANG, owned HOME/VIBE_SETTINGS and explicit owned XDG config/cache/state/data/runtime homes; no inherited provider/XDG credentials/settings, remote provider or OS policy change. The early representative pathname/network trace and later explicit descriptor subset remain narrower historical evidence; neither is relabelled complete. The corrected filter is `%file,%network,%desc,%memory,%process` with `-yy`. File writes/transfers/locks, truncation/metadata/sync, shared writable file mmap, shared-file mprotect/msync/remap and failed attempts are counted. Decoded pipes/control descriptors and private/anonymous maps have explicit retained allowances; read-only shared maps are tracked for later write protection.

Owned positive control detects **10 attempts**, including successful and failed fd pwrite, ftruncate/fchmod, refused shared writable mmap, successful shared-file mprotect/msync and loopback socket/connect/write. Private file and anonymous maps are retained as allowed controls. Program and parser exit0. An earlier v2 label mismatch produced wrapper exit1 while correctly detecting attempts; it is retained in INCIDENTS. Corrected complete-oracle counters replay the fully traced bytes after parser refinement; replay is explicitly analysis-only, not fabricated new execution.

No provider port enters the source service or strict proof APIs; this is an inspected API boundary, **not a fabricated per-provider invocation counter**. Actual network and filesystem boundaries are measured.

12 independently parsed actual CLI/MCP/service success/refusal payload triples match in FINAL-INDEPENDENT-SURFACE-PARITY.json. 24 exact child stdout captures are retained in full-oracle-real-surfaces/, reconstructed from saved syscall writes without another process run. Validated outcomes cover repeat success, stale document expectation, unsupported strict-grade shape, missing installed capture, record-only/payload-only/coordinated tamper and coordinated pristine-document expectation. Workspace additionally binds source/document/resolution expectation roles. MCP direct dispatcher compares exact observed self content, stale expectation, closed-shape refusal and fresh absent-document refusal. Installed missing capture is unavailable_world; a fresh absent self document is unknown_document, both precise refusals without hydration.

27 before/after snapshot pairs retain **exact fixture bytes**, file/directory path/type inventories and the watched permissions where collected. FINAL-SNAPSHOT-PAIRS.json is independently parsed from final raw test output and each pair matches. Initial manifest/lock/config bytes and empty cache directories are preserved; a valid empty owned config.toml is seeded. Intentional poisoning is captured before query and preserved, never repaired. Snapshot digests are reporting metadata, not source authentication; equality includes actual byte vectors.

## Integrity strength / strict availability

Root decision question-group.f0a7ec42 revision2 binds the existing proof owners and preserves current ABI. Normal surfaces remain explicit observed or record-consistent; coordinated mutable record/payload agreement may remain record-consistent, **never source-proven**. Unsupported proof-strength input is invalid_request/request shape before preparation, not a genuine strict-grade success. Stronger surface delivery remains unavailable.

The actual install verifier is exercised separately in the executable integration target: an independently selected canonical Legacy0 hash and original capture mint the private verified seal; Mixed original output reaches SourceProven; XML builtin derivation compares exact retained materialization and distinct original/representation hashes. Missing seal refuses strict original source capture. Coordinated output+record poisoning refuses strict original membership or exact builtin replay. Replaying coherently changed original bytes against pristine selected H returns SourceHashMismatch. All these successful/refused owner calls have measured zero-effect intervals and exact state snapshots. No local editable digest is promoted; no new recipe/authentication framework or B229 availability expansion.

## Checks / retained failures

Final touched rustfmt, XML parse and git diff --check exit0. Max Rust owner 574 lines, all below600. Full workspace fmt exit1 on13 historical outside-owner diff entries; no green/full-panel claim or cleanup. No actual schema/format/generator change: conditional codegen/wire oracle is not triggered. Root reports freshly compiled specmap exit0 on the same nine-source subject:138 unchanged old orphans/0host unresolved. Acceptance remains pending with Root.

INCIDENTS.md preserves wrong schema filename, output truncations, missing-capture diagnostic fixture mismatch (3/4) and E0282 fixture inference compile failure. Initial inherited-environment and digest-only/in-memory state evidence remains historical; final evidence adds clean environment and retained exact bytes. COMMANDS.json records actual commands, cwd/env, exits/counts and hashes, including no-run as compile-only. Existing B227/B230/B231 and fullfmt debt stay deferred; no Windows/macOS/native race/hostile-ABA or Zap autonomy/release qualification. Managed review ZAP009 remains unconfigured and was not reprobed/replaced by private collaborator. External Aeneas/Zap/PodBay state untouched.

## Review handback

Source writes stopped. Root reviews nine-file source/hash/counter/snapshot evidence, runs its specmap and decides atomic acceptance. No further optional suites or next atom selected.

```text
 M FIXED-ISSUES.md
 M crates/vibe-mcp/src/tools/read_spec.rs
 M crates/vibe-workspace/src/source_query/capture/continuity/service.rs
 M crates/vibe-workspace/src/source_query/capture/continuity/service/m_01_d_2_cases.rs
 M vibevm/vibespecs/common/PROP-070-resource-resolution.xml
?? crates/vibe-cli/tests/m_01_e_2_cases.rs
?? crates/vibe-cli/tests/m_01_e_2_cases/
?? crates/vibe-mcp/src/tools/read_spec/
?? crates/vibe-workspace/src/source_query/capture/continuity/service/m_01_e_2_cases.rs
```

## Root-provided independent attribution audit

ASTRA-COUNTER-AUDIT.json is Root-supplied read-only evidence for its recorded trace epoch, not a claimed independent audit of the later mapping extension. It confirms zero recorded-class effects and explicitly limits child attribution to exec-to-exit. The already commissioned mapping/positive-control completion is in observe_queries.py, observer-positive-control-final-counters.json/run.log and the *-full-oracle.strace / *-complete-oracle-counters.json artifacts. Positive-control trace SHA256 ef2cf8f3c61ae3762b15cd6d5e44cccccf13570209a73526c6b4509b12bccdc4; corrected CLI raw trace SHA256 8a520afcc305b968751e97b8869c2de802fa59556b33dc08eb7a223d443fa0fa. No new behavior test, Cargo build or Rust edit followed this audit notice.

## Final independent audit and sealed frontier

ASTRA-FINAL-COUNTER-AUDIT.json confirms sufficient commissioned Linux evidence at these exact interval/exec boundaries. Its independent split-record review finds456 scoped CLI mappings:432 private/24anonymous, noMAP_SHARED; writes are response/marker pipes and ioctl probes failENOTTY. The parser allowance ledger records254 of these456 mappings because202 split private/anonymous records are omitted; it is not a universal or complete syscall allowance inventory. No measured effect is missed on this subject. Pre-exec72dup2/24chdir are outside attribution; no traced surface descendants.

The saved Astra audit covers its earlier control epoch (7mutation+2network). First Agent's finalization additionally matches the final control trace hash and final10 operations:4fd+3mapping+3network including socket write. These checks are distinguished, not silently merged into a claim Astra calibrated the later socket write. Root also independently compares all27 exact snapshot inventories equal and matches8/5/22 intervals plus24 vibe exec-to-exit scopes with zero diagnostics. No additional behavior/control execution, parser/ancestry refactor, Cargo build or product-source mutation was performed for finalization. Earlier narrow epochs, genuine original/derivation proof outcomes and failures remain retained. Raw traces and control binaries stay in research evidence; Root owns canonical packaging and acceptance.
