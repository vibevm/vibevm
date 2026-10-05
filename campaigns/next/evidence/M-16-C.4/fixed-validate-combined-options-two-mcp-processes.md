# Fixed Validate combined options across two MCP processes

M-16-C.4 bounded slice, starting at clean `b0769c5ab`. Historical 36
accepted tasks remain accepted; M-16-C.4 remains active.

Two independent CLI-launched MCP stdio processes attach to the same project.
Each obtains a typed selection and approval for `force=true, trace=true`.
A barrier releases their same-key submits together. One process may report
`inspection_unavailable` while it loses the lifecycle lease; the other must
publish a terminal retained C1 run. Cold inspection and same-key reapproval
must resolve to that run. A changed `trace=false` approval under the key
must be refused. The script records exactly one effect and the retained trace
report is finalized for the run.

The first version of the exact test intermittently found both in-process
owners stopped before an effect. `submit_one_script_validate_internal`
preflighted under a lifecycle lease that it released before
`submit_protected_lifecycle_inner` acquired a second lease. Two processes could
cross in that gap. The adapter now retains one lease from source preflight
through protected reservation and dispatch; other protected entry points keep
their existing acquisition path. The process test inspects both participants
because a losing process can retain its own unavailable owner observation.
It then verifies replay from a fresh process.

## Exact checks

With `RUSTUP_HOME=/home/olegchir/.rustup`,
`CARGO_HOME=/home/olegchir/.cargo`,
`PATH=/home/olegchir/.cargo/bin:/usr/bin:/bin`, and
`CARGO_TARGET_DIR=/fast/git/v/vibevm-next/target`:

- `cargo +nightly test -q -p vibe-cli --test cli_mcp_fixed_validate combined_options::two_mcp_processes_racing_combined_options_replay_one_run_and_effect -- --exact`: 1/1, exit 0, followed by 12 independent exact 1/1 passes.
- `cargo +nightly test -q -p vibe-cli --test cli_mcp_fixed_validate combined_options::real_stdio_combined_force_trace_retains_both_options_across_cold_inspect -- --exact`: 1/1, exit 0.
- Targeted `rustfmt +nightly --edition 2024` and `git diff --check`: exit 0. An incidental recursive formatting change in another module was restored.

The private PodBay rustup lacked the required nightly Cranelift backend, and
the checkout's parent Cargo config supplies `-Zthreads=8`, which stable
rejects. The installed host rustup paths above let the exact checks run.
No PodBay/Zap files were changed. No full gate panel, routine mutation test,
or public VibeVM 1.1.0 release ran. The barrier proves a process-level race
attempt and retained result; it does not prove every possible winner order.
