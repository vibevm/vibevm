M-10-B.1 same-task R1 generator correction — BLOCKED / UNREVIEWED

This is not acceptance. Root alone reviews/accepts. Baseline remains 72 accepted task contracts plus separate M-03-A.2 plan safe-stop; no acceptance events replayed. Both previous stores/candidates remain untouched.

Work performed: verified HEAD f7de5182ace73b18e0a6f9214a211a48a54d7338 and all 31 source-baseline.json current_files hashes before editing. Read the independent R1 review, prior correction report/findings/candidate and codegen failure, bounded standing ownership/user/compiler sources, READ-ROSTER and owning optional_shapes/emit.rs, postproc.rs and mod.rs. Initial attempted PROJECT-BOUNDARIES path lacked common/; the correct file was subsequently read. No root AGENTS/fullboot/stewardship, child agents or solver calls.

Implemented schema-side support for optional nullable string with explicit metadata.x-default:null, reusing Decision::OptionValue and the existing emission/stitch. Both direct strings and ref-resolved strings use the same decision. Required-nullable behavior remains unchanged. The optional nullable guard now runs before collection delegation, so unsupported optional nullable collection shapes also refuse in this pass. Other scalars, vocabulary, structure and union shapes refuse; absent/non-null defaults refuse. No emission implementation or global generator cleanup. Three added unit cases cover direct/alias stitch and missing-skip refusal, serde null/absence normalization and non-string rejection, and unsupported-shape/default matrices. Updated the existing missing-policy test to reflect the bounded new rule. These Rust cases have NOT been executed.

The prior nullable selector schema, corpus and wire regression are retained unchanged. Exactly-one effective-selector validation is unchanged. Four authorized generator source/test files changed; 27/27 other baseline files are byte-identical, including all generated/core/workspace/schema/corpus/reader files. No generated output was hand-edited or regenerated in this run.

BLOCKER: required warm-target authorization was not supplied. WORKER-PACKET.md and the user instruction require reuse ONLY AFTER Root retires the prior run and authorizes idle handover. WARM-TARGET.md, PREPARATION.md and preparation-evidence.json all still record pending Root handover. Asked asynchronously for that authorization; no response was received before this blocked report. Did not infer authorization from launch, elapsed time or the presence of a target. Did not run preflight-idle.py, stop processes, access target locks, copy/delete/seed targets, create a cold target, or use /tmp for builds. No dependency/toolchain/lockfile/configuration changes.

Required build commands NOT ATTEMPTED (no exits/counts claimed):
- RUSTUP_AUTO_INSTALL=0 CARGO_TARGET_DIR=/fast/zap-managed/2026-10-14-m10b1-correction/cargo-target TMPDIR=/fast/zap-managed/2026-10-14-m10b1-codegen/tmp cargo run --offline --locked --quiet -p xtask -- codegen
- Same environment: cargo test --offline --locked -p vibe-wire --lib m_10_b_1_cases
- Same environment: cargo test --offline --locked -p xtask --bin xtask codegen::optional_shapes::tests
No unsupported emission shape was observed in this run: the actual generator has not been invoked. A successful build or regeneration is not implied by static inspection or formatting.

Actual commands/checks:
1. Python SHA256/HEAD baseline assertion: exit 0, all 31 current_files matched (baseline-verification.log).
2. RUSTUP_AUTO_INSTALL=0 rustfmt --edition 2024 --config skip_children=true xtask/src/codegen/optional_shapes.rs xtask/src/codegen/optional_shapes/tests/required_nullable.rs xtask/src/codegen/optional_shapes/tests/refusals.rs xtask/src/codegen/optional_shapes/tests/stitch.rs: exit 0, scoped formatting operation.
3. Same rustfmt command with --check: exit 0 (fmt-final.log).
4. git diff --check: exit 0 (diff-check.log).
5. git apply --check --reverse /fast/zap-managed/2026-10-14-m10b1-codegen/same-task.patch: exit 0; read-only validation, no patch application.
6. git apply --check --reverse /fast/zap-managed/2026-10-14-m10b1-codegen/correction-only.patch: exit 0; read-only validation.
7. Python final baseline perimeter assertion: exit 0, 27 unchanged files retain exact hashes (perimeter-verification.log).
No tests executed in this run. findings.checks_attempted=true refers only to the actual formatting/diff/patch/hash checks above. It does NOT claim either captured cargo verification was attempted or passed; candidate check references are emitted by finish-candidate and must be read with this explicit limitation.

Reusable prior evidence: original core-test-2.log 6 passed / 0 failed / 0 ignored and workspace-test-2.log 5 passed / 0 failed / 0 ignored under /tmp/vibevm-managed-m10b1. These unchanged core/workspace sources preserve complete navigable base/local/result chain evidence. Previous correction wire-test.log recorded 5 passed / 0 failed / 0 ignored against the unchanged reader; its prior codegen.log failure is retained, not replaced with a success claim. No core/workspace/all15/full-suite rerun. Prior global formatting and committed-baseline check-codegen limitations remain. No durable publication, live authentication, activation or general five-family transformation qualification is claimed.

Dispositions in capture order: UNSATISFIED strict schema/carrier criterion because successful incumbent codegen and required focused checks remain pending; SATISFIED complete navigable chain from unchanged retained implementation and prior evidence. Original task acceptance remains unchanged. This incomplete candidate requires Root's target handover and subsequent actual generation/wire5/optional_shapes checks before review can establish R1 closure.

Artifacts: correction-only.patch compares current bytes to this packet's before/ snapshot (four generator files). same-task.patch contains retained original implementation, prior nullable correction and this generator correction against recorded HEAD, restricted to M10B1 source/schema/corpus/spec/generated files and authorized generator files. No Git staging/mutation/commit/deployment occurred.

correction-only.patch: 8502 bytes; SHA256 fa7d4e02f2aaa59f73f4e824373fc0ef32eecfff9f75096be4f49460d76a0acf

same-task.patch: 66505 bytes; SHA256 8bff3224610d64dd1a2932e93d3effda719357e5c78c1c7340f037858f09c958
