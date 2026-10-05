# Fixed Validate machine-frame and MCP process race

Zap Current revision 11, strategy `strategy.vibevm-next.coarse-c4.1`, names
`next.m16c4.parent`. This is a bounded M-16-C.4 transport proof on clean
source HEAD `8a8a713cf`; C.4 remains active.

An explicit fixed Validate grant is issued by a CLI machine-frame process.
A separate CLI-launched MCP stdio process reattaches and obtains its own typed
approval. A barrier releases both processes to submit the same scoped key.
The retained C1 index must contain one namespace, key and run; the captured
script records one effect. A surface that loses the active lease may report
unavailable effect truth, then both surfaces must inspect and replay the same
terminal run. A different approved control intent under that key refuses.

## Exact checks

- `cargo test -p vibe-cli --test machine_session_fixed cross_surface::real_cli_frame_and_mcp_stdio_race_one_fixed_validate_key -- --exact`: 1/1, exit 0 under the installed nightly toolchain, followed by three exact 1/1 repeats, all exit 0.
- `rustfmt --edition 2024` on the changed test file and `git diff --check`: exit 0.
- The Cargo command used the campaign two-slot wrapper and the integration
  target. Final logs: `/tmp/vibevm-next-validate-process-race-parent.log` and
  `/tmp/vibevm-next-validate-process-race-repeat-{1,2,3}.log`.

The worker's private-target attempt executed zero tests because it selected
the stable toolchain while this host's test profile requires nightly. Root
selected the module-qualified exact case and reran it. The first real run
exposed fixture-only assumptions: MCP reads user config on startup and CLI
inspection can temporarily lose the lease. The test now gives MCP valid
fixture config and retries typed unavailable inspection. This is not a product
code change. The barrier aligns submit calls, but does not instrument the
exact internal lease transition; one observed schedule is not a proof that
both winner orders occurred. Historical accepted tasks were not revalidated;
no full gate panel, routine mutation testing or public 1.1.0 release ran.
