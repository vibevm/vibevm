# Fixed Validate MCP failure after a script effect

This bounded M-16-C.4 process proof starts from clean `bc43a6b83`; the
historical 36 tasks remain accepted and M-16-C.4 remains active.

An explicitly granted CLI-launched MCP stdio fixed Validate process runs a
captured script that writes `effect\n` and exits 17. Its terminal typed
inspection reports the same C1 run as its submit, `Failed` with `Uncertain`
effect state, partial result, `validate/fail` last phase and a failed script
work row. A separate cold MCP process inspects the retained report and
preserves the run, terminal status and work row. The counted file still
contains exactly one effect. This verifies that the registered MCP surface
does not serialize this post-write handler failure as `NoEffects`.

The first test version wrongly compared the entire live callback view with the
cold retained-report view. Live callbacks include intermediate phase rows and
counts that are not part of the retained summary. The final assertion compares
owner facts that both views promise, while checking the cold evidence basis.
That correction did not require a product-code change.

## Exact checks

With the installed host nightly (`RUSTUP_HOME=/home/olegchir/.rustup`,
`CARGO_HOME=/home/olegchir/.cargo`, `PATH=/home/olegchir/.cargo/bin:/usr/bin:/bin`,
`CARGO_TARGET_DIR=/fast/git/v/vibevm-next/target`):

- `cargo +nightly test -q -p vibe-cli --test cli_mcp_fixed_validate partial_effect::real_stdio_failed_validate_retains_prior_effect_and_phase_cold -- --exact`: 1/1, exit 0 after the assertion correction and again after final format/assertions; 9 filtered out.
- File-scoped `rustfmt +nightly --edition 2024 --config skip_children=true` and `git diff --check`: exit 0.

The run did produce a final retained report. Missing-report recovery, other
operations and complete CLI/MCP/local API equivalence are still open. No
full gate panel, routine mutation testing, PodBay/Zap edit or public VibeVM
1.1.0 release ran.
