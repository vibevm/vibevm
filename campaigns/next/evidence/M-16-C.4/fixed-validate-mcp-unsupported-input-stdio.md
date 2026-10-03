# Unsupported fixed Validate inputs through CLI MCP stdio

Zap Current revision 11, strategy `strategy.vibevm-next.coarse-c4.1`, names
`next.m16c4.parent`. This bounded M-16-C.4 process proof starts after
`b63e4d149`; it does not advance C.4 acceptance.

A CLI-launched `vibe mcp serve --fixed-validate issue` process receives two
explicit requests: an unsupported `build` phase and a Validate request with
`deploy_profile="posix"`. Each returns structured `implementation_gap`.
Neither request creates a script effect or a lifecycle outcome. The requested
profile is therefore refused visibly instead of being discarded or silently
replaced by a startup default.

## Exact checks

- `cargo test -p vibe-cli --test cli_mcp_fixed_validate real_stdio_unsupported_phase_and_profile_refuse_before_effect -- --exact`: 1/1, exit 0.
- `cargo test -p vibe-cli --test cli_mcp_fixed_validate`: 7/7, exit 0.
- Both ran through the campaign two-slot wrapper with a temporary `/tmp` lock root and `RUSTC_WRAPPER=` because this sandbox cannot write the shared slot lock or start sccache. Logs: `/tmp/vibevm-next-c4-unsupported-options-exact.log` and `/tmp/vibevm-next-c4-unsupported-options-family.log`.
- `rustfmt --edition 2024` on the changed test file and `git diff --check`: exit 0.

This is a refusal proof for the fixed Validate process surface. It does not
implement Deploy profiles, another phase/domain or complete cross-surface
equivalence. The historical 36 accepted tasks were not revalidated. No full
gate panel, routine mutation testing or public 1.1.0 release ran.
