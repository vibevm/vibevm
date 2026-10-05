# Fixed Cargo CLI and MCP process race

This M-16-C.4 atom tests the existing fixed offline Cargo owner from two
separate processes. A machine-frame CLI connection and a registered MCP stdio
server reattach the same issued host, then submit one scoped key concurrently.
The test accepts an incumbent lease refusal during contention only when a
subsequent cold reattach returns the winning retained result. It checks one
C1 namespace, one reserved key and one retained Cargo result for the same run;
the host pointer bytes remain unchanged. The fixture is temporary and invokes
only its fixed reviewed Cargo build.

## Exact checks

- `cargo test -p vibe-cli --test machine_session_cargo process_mcp::machine_frame_and_stdio_mcp_process_race_one_cargo_key -- --exact --nocapture`: exit 0, 1 passed, 0 failed, 7 filtered out.
- The same exact case with `cargo test -q` passed in three further independent runs, each exit 0.
- File-scoped `rustfmt --edition 2024 --check` for the two touched test files: exit 0.
- `git diff --check`: exit 0.

The test does not claim a general build or hermetic toolchain capability.
M-16-C.4 remains active. The historical 36 accepted tasks were not rerun;
no full gate, routine mutation panel or public VibeVM 1.1.0 release ran.
