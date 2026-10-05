# Partial package output across CLI frame, local API and MCP

This bounded M-16-C.4 atom starts from clean `d475943b2` and local plan
revision 429. The historic CLI-only damaged-output test and 36 accepted tasks
were not rerun.

A disposable fixed package owner first completes one copy under a CLI-issued
C1 key. The test removes only that fixture's copied output, leaving the
retained package journal and A2 artifact record intact. A trusted direct
`FixedPackageRoute::exchange_local` caller, a separate MCP stdio process and
an independently reattached CLI machine frame then inspect the same key.
All three publish the same generated `reply.kind=uncertain`, the known C1 run,
`output_file=absent` and `artifact_record=present`. CLI's outer frame does
not claim `NoEffects`. Same-key CLI submit preserves that uncertain reply and
does not recopy the missing output; the C1 index remains one namespace/key/run.

The first test attempt expected an Error arm and failed. The owner correctly
uses an Uncertain result with explicit partial observations. The final test
asserts that actual owner contract. This is a targeted retained-output fault
in a temporary fixture, not a mutation panel or product-code change.

## Exact checks

With the installed host nightly (`RUSTUP_HOME=/home/olegchir/.rustup`,
`CARGO_HOME=/home/olegchir/.cargo`, `PATH=/home/olegchir/.cargo/bin:/usr/bin:/bin`,
`CARGO_TARGET_DIR=/fast/git/v/vibevm-next/target`):

- `cargo +nightly test -q -p vibe-cli --test machine_session_package local_route::damaged_package_output_keeps_run_and_effect_truth_across_three_surfaces -- --exact`: 1/1 after correcting the expected owner arm, and 1/1 after scoped formatting; six filtered out.
- File-scoped `rustfmt +nightly --edition 2024 --config skip_children=true` and `git diff --check`: exit 0.

No PodBay/Zap file, real-user project, full gate panel, mutation panel or
public VibeVM 1.1.0 release was touched. M-16-C.4 remains active.
