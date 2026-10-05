# Reconcile CLI frame negative option case with accepted trace support

This bounded M-16-C.4 maintenance atom starts at clean `c8a8d314e` and local
plan revision 425. Source audit found the CLI frame test
`frame_supplied_authority_outer_ids_and_unsupported_options_never_dispatch`
still expected `trace=true` to return `ImplementationGap`. The fixed Validate
owner now explicitly supports trace; the accepted real CLI frame plus MCP
process proof at `e66cc12fe` exercised `force=true, trace=true` and retained a
finalized trace. The old negative expectation no longer matched the product.

The test now retains its authority-forgery and unsupported
`deploy_profile="posix"` refusal checks, including `NoEffects` before provider
dispatch. It no longer labels supported trace as an implementation gap. This
changes only stale test data, not runtime behavior or Deploy authority.

## Exact checks

With the installed host nightly (`RUSTUP_HOME=/home/olegchir/.rustup`,
`CARGO_HOME=/home/olegchir/.cargo`, `PATH=/home/olegchir/.cargo/bin:/usr/bin:/bin`,
`CARGO_TARGET_DIR=/fast/git/v/vibevm-next/target`):

- `cargo +nightly test -q -p vibe-cli --test machine_session_fixed cases::frame_supplied_authority_outer_ids_and_unsupported_options_never_dispatch -- --exact`: 1/1, exit 0; 19 filtered out.
- File-scoped `rustfmt +nightly --edition 2024 --config skip_children=true` and `git diff --check`: exit 0.

The historical 36 accepted tasks were not rerun. M-16-C.4 remains active.
No PodBay/Zap file, full gate panel, routine mutation testing or public
VibeVM 1.1.0 release was involved.
