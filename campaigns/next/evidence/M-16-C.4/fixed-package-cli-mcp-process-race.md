# Fixed Validate CLI frame / MCP process race

Baseline: `8a8a713cf` on `next`; current plan revision 416, node M-16-C.4. This is a focused test atom, not acceptance of C.4 or a full gate panel.

`real_cli_frame_and_mcp_stdio_race_one_fixed_validate_key` starts separate actual `vibe` machine-frame and MCP-stdio processes. Each obtains its own typed approval for the same fixed Validate root. A barrier releases same-key submissions together while the fixture holds the single counted effect. The test checks one retained run, both surfaces' honest busy/uncertain responses, terminal same-key replay, changed approved intent refusal, and one C1 namespace/key/run.

Verification: `rustfmt --edition 2024` on the changed test and `git diff --check` passed. The first coordinator attempts exited 101 before any test because `/fast/git/v/.cargo/config.toml` selects nightly Cranelift, `-Zthreads=8`, and an incompatible test backend on this host. A manifest-addressed stable Cargo invocation from `/tmp` avoids that parent config while reusing the hot XFS target:

`CARGO_TARGET_DIR=/fast/git/v/vibevm-next/target cargo +stable test --manifest-path /fast/git/v/vibevm-next/Cargo.toml -p vibe-cli --test machine_session_fixed real_cli_frame_and_mcp_stdio_race_one_fixed_validate_key -- --nocapture`

One initial exact run passed 1/1; five further exact runs passed 5/5. No production code, unrelated tests, mutation testing, full gates, public publication, or historical 36-task recheck occurred.
