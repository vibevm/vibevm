# Full M-08-C.1 independent review

**C1 source/spec/compatibility: bounded clear; no reproducible P1/P2 implementation finding. Acceptance remains pending required V5.** Its actual exits are codegen0, check-codegen1, wire-diff1. Root clarified that the captured acceptance contract requires observed passes: neither historical debt nor this semantic review can substitute for them. No source change, test/build, process control, Git mutation or acceptance action was performed here.

Root supplied managed runtime succeeded/effects completed and candidate revision717. That is not WorkAccepted. All104 candidate hashes match the manifest and stayed stable across review; four main evidence files also stayed stable. sourcehashes.json binds the exact subject. Read full captured C1 task, packet/preflight/addendum, approved question/answer, actual changed owners/schema/corpus, requested ledgers and failed/final logs. Historical preflight prerequisite warnings are superseded by the captured committed prerequisite binding, not independently replayed here.

## Full C1 behavior

The optional typed client_request is integrated into existing MechanismDecl parsing/validation/serialization, including omission compatibility. Qualified identity/version applicability, bounded scope/shape/path/key/operation fields and duplicate binding rejection are implemented as inert data. Selected registry admission validates the entire map, including disabled rows, before publication. Canonical open ClientCapabilities is consumed by the actual upper service; this is not a test-only parallel catalogue. Explicit legacy profile and legacy3/native adapters refuse new identities they cannot represent.

One private CapturedHost extraction preserves old AdapterPlanHost/Route interfaces and shares incumbent document/source context capture. New descriptor discovery uses existing selected registry observation, not prepare_adapter_sources/hydration. Captured provider coordinate/root/version/self and declaration equality are joined with manifest/request digests; source revalidation surrounds capture and query. No raw-world getter, new resolver/context table, native Plan or grant issuer was introduced. Observed/lock-bound strength, not-evaluated implementation, not-probed availability, not-granted authorization and not-admitted principal stay separate. M14 context/root/principal are discovery metadata here; C2 admission and C3 receipts remain subsequent work.

New client-capability request/result/error epoch1 and operation identity are distinct from old adapter-plan/e1. CLI Clients branches before legacy target/route/serve; no MCP or top-level/progress rewrite. New generated schemas are closed while qualified map keys remain open; semantic reader rejects duplicate JSON map keys, invalid request matrix and digest mismatch. Old route/readers reject the new family. Unknown descriptor epoch/operation and duplicate selected binding produce generated NoEffects refusal, not partial map or implicit preparation. Marker presence never changes authority.

## Actual proof and compatibility

Final V1–V4 logs contain13 distinct nonignored passing cases:3 core/registry,2 map/legacy,5 upper/CLI/retained B1-B2,3 wire/old grammar. Repeats and failed attempts are not added. All122 supporting artifact hashes match. Failed compilation/bootstrap/fixture/format attempts are retained and actual_log paths distinguish them from final logs.

The499,522,232-byte CLI independently hashes to `95ca9a0b6b085a62cff4d27d5260bd97287ebbbd68156a1912602d2e6e56696d`. Six same-binary phases record that same hash: before, unselected directory, selected package, duplicate, unknown operation, unknown epoch. First two expose an empty map; selected exposes org.example/new-client, .new-client/settings.json and key segments [mcpServers,new.key]; the final three refuse. Twelve measured requests have unchanged owned censuses and zero observed selected process/network/mutation/lock attempts. Grant issuance absence is supported by source capability/call-graph inspection, not a syscall counter. Setup/preparation effects are separate. Actual build/dep-info bindings disclose two post-build test-only source differences (registry cfg(test) and fixture data), not hidden production drift; the fixture is not a production dep-info input. No CLI rebuild occurs between proof phases.

Old e1 preservation is exactly **11 raw-equal files +2 initial-comment-header-only files**, per matching Root authorization; both Rust bodies are exact. Independent comparison finds51 existing generated module changes comment-only; the other existing semantic generated changes register the three new FormatIds/module. New-family generated payloads match inspected schemas. All36 recorded native-e1 semantic schema/generated/corpus files and44 read-only subjects match. The nine narrowly authorized initializer paths differ only by client_request:None. Five incidental generator outputs outside wire are unchanged. Removing the additive CLIENT-CAPABILITY-REQUESTS paragraph reconstructs prior PROP054 bytes, including A2/B1; no old rule deletion.

## Required V5: exact cause and minimum handback

**check-codegen is not green.** `xtask/src/codegen/mod.rs::run_check_codegen` calls run_codegen then `git diff --exit-code -- <generated paths>` with no HEAD argument. It compares worktree against index. Its red output contains intended generated additions/comment changes. Root must first establish an ordinary reviewed generated snapshot/index state through its normal integration discipline, then obtain the actual selected check pass; a commit is not technically the only way the command compares, and no staging-only bypass or fabricated pass is recommended here.

**wire-diff is not green and its epoch tail has not run.** It invokes the existing journal projection check before wire-change/epoch decisions and stops on only `formats/corpora/index/e1/repomd.json`. All18 index corpus files, Cargo.toml and relevant projector implementation are byte-identical at both f676286eff01ee058eeb6d23b5dd9ec1848d57ab and51c7472e3e435dfe4c0bf601c5f99c61447e05be. Thus the mismatch predates C1.

The concrete old stamp is repomd.generator=`vibe-index 1.0.0`; workspace/package version is1.1.0. Existing journal::project creates Index::new, whose generator is default_generator() = `format!("vibe-index {}", env!("CARGO_PKG_VERSION"))`; write_to places that generator in repomd. The current deterministic projection therefore owes the1.1.0 stamp. Actual V5 reports only this file; generating into scratch and comparing must confirm the complete exact delta before copying anything. `golden_corpus.rs` expressly says version-stamp changes regenerate the corpus rather than normalize the test.

There is **no existing repair CLI**: `vibe-index rebuild` and its xtask wrapper accept only --check and reject repair; reindex scans/appends journal and is wrong for this job. The existing public deterministic API is sufficient for Root's narrowly authorized generated-artifact operation:

- `use vibe_index::journal;`
- `use vibe_index::index::memory::WriteCtx;`
- `journal::default_dir(data_dir: &Path) -> PathBuf`
- `journal::replay(journal_dir: &Path) -> vibe_index::error::Result<Vec<JournalRecord>>`
- `journal::project(events: impl IntoIterator<Item=JournalRecord>) -> vibe_index::error::Result<Index>`
- `Index::write_to(&self, data_dir: &Path, ctx: &WriteCtx) -> vibe_index::error::Result<()>`
- `WriteCtx { at: index.generated_at }` uses the last applied journal timestamp; do not use wall clock or the existing catalog as input.

Root's managed repair can replay only this corpus journal, project through these incumbent APIs into owned scratch, compare the complete writer surface, and copy only generated repomd.json if it is the sole difference. Preserve journal/other catalog hashes and saved diff. No writer, script or artifact was created/executed by this reviewer. No new product repair command, migration, source rule or journal edit is required.

Then the existing focused proof is `cargo xtask rebuild --check formats/corpora/index/e1`, followed by the selected V5 checks (`cargo xtask check-codegen`, `cargo xtask wire-diff`) on the reviewed snapshot. codegen already has an actual0 receipt; repeat only where source/output identity requires it. Current EPOCHS public=false/break_window_open=true would use the reporting branch after successful corpus proof, but no pass is predicted as evidence. C1 stays unaccepted until Root has the required actual outcomes.

## Limits

This review establishes C1 readable/validated/no-inferred-grants semantics and compatible separate wire transition, not C2/C3 execution, vendor/platform certification or hostile atomicity. No optional UI, broad suite or foundational audit is needed. V5 restoration is a narrow baseline generated-artifact/gate issue, not a newly found C1 architecture defect. Additional exact diagnostic reads were the existing xtask rebuild/wire/codegen owners, index CLI rebuild/reindex, journal projection, index memory writer and golden_corpus test; no neighboring project was changed. Root alone accepts/commits/counts.
