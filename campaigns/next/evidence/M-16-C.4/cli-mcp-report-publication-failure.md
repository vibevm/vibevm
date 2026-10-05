# Fixed Validate report publication failure across CLI frame and MCP

This bounded M-16-C.4 atom starts at clean `7bbf0e2ea` and local plan
revision 424. It is distinct from the accepted test that deletes an already
retained report: here the report path is occupied before the owner attempts
to publish it. The historical 36 accepted tasks were not rerun.

A real CLI machine-frame process issues a typed Validate approval and begins a
captured script. The script writes one effect and waits. The test obtains its
C1 run ID, places a directory at that run's report destination in its
**temporary fixture**, then releases the script. A separate CLI-launched MCP
stdio process has its own typed approval for the same key. Report publication
fails. Both surfaces retain the known run and Validate scope, project a
partial `RecoveryRequired` view, and refuse to call the effect `NoEffects`.
MCP same-key replay still resolves to the same known run without executing the
script again. The occupied path remains a directory and the counter contains
one effect.

This is a deterministic report-storage fault, not a claim about every disk
failure mode. It verifies the registered process boundary and actual owner
publication path; no product implementation changed.

## Exact checks

With the installed host nightly (`RUSTUP_HOME=/home/olegchir/.rustup`,
`CARGO_HOME=/home/olegchir/.cargo`, `PATH=/home/olegchir/.cargo/bin:/usr/bin:/bin`,
`CARGO_TARGET_DIR=/fast/git/v/vibevm-next/target`):

- `cargo +nightly test -q -p vibe-cli --test machine_session_fixed cross_surface::report_publication::cli_and_mcp_keep_run_uncertain_when_report_publication_fails_after_effect -- --exact`: 1/1, exit 0 before and after same-key replay and scoped formatting; 19 filtered out.
- File-scoped `rustfmt +nightly --edition 2024 --config skip_children=true` and `git diff --check`: exit 0.

No PodBay/Zap file or real-user project was changed. No full gate panel,
routine mutation testing or public VibeVM 1.1.0 release ran. Other domain
owners, deploy-profile decisions and complete adapter equivalence remain open.
