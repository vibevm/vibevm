# Fixed Validate MCP recovery with a missing retained report

This bounded M-16-C.4 process proof starts at clean `9b1f209d4` and plan
revision 421. The previous post-write script failure proof is accepted and
was not rerun. The historical 36 accepted tasks remain accepted; C.4 is active.

A CLI-launched MCP stdio fixed Validate process completes one explicitly
approved script effect and retains a C1 run. The test removes only that
run's final report from its temporary fixture, then starts a separate cold
MCP process. Cold `inspect` returns structured `inspection_unavailable` with
the known run, requested Validate phase, requested scope and
`effect_observation=unavailable`. Its typed view is partial
`RecoveryRequired`, bound to the same run. A fresh typed approval and same-key
submit may first return asynchronous `Starting`; subsequent inspection returns
the same known run and does not claim `NoEffects`. The script counter remains
one effect.

This proves the registered process surface preserves owner identity and
uncertainty across missing final evidence. It does not simulate a failure in
the report writer itself, and it does not infer the exact filesystem effects
from a missing report.

## Exact checks

With the installed host nightly (`RUSTUP_HOME=/home/olegchir/.rustup`,
`CARGO_HOME=/home/olegchir/.cargo`, `PATH=/home/olegchir/.cargo/bin:/usr/bin:/bin`,
`CARGO_TARGET_DIR=/fast/git/v/vibevm-next/target`):

- `cargo +nightly test -q -p vibe-cli --test cli_mcp_fixed_validate missing_report::real_stdio_missing_report_retains_run_and_unavailable_effect_without_redispatch -- --exact`: 1/1, exit 0 after the asynchronous replay assertion was corrected and again after final formatting; 10 filtered out.
- File-scoped `rustfmt +nightly --edition 2024 --config skip_children=true` and `git diff --check`: exit 0.

An intermediate test expected synchronous refusal from `submit` and failed:
MCP correctly returned `Starting` while its owner worked. The final test
waits for the authoritative inspected refusal. No product code, PodBay/Zap
file or real-user project was changed. No full gate panel, routine mutation
testing or public VibeVM 1.1.0 release ran.
