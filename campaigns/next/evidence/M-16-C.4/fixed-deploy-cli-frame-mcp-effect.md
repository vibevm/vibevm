# M-16-C.4 fixed Deploy CLI frame to MCP effect

A Linux process fixture seeded one verified artifact in an isolated project,
issued a fixed Deploy host through `vibe machine session`, negotiated the five
registered schemas and submitted `old-key` through the generated local frame.
The owner returned a completed Deploy run and published the launcher. After
the CLI process ended, a separate `vibe mcp serve --fixed-deploy reattach`
process submitted the same key and returned the identical completed reply.
The C1 index had one namespace and one key with the same run ID; the grant
pointer and launcher bytes did not change on replay.

Exact check on 2026-10-05: `cargo +stable test -p vibe-cli --test
cli_mcp_fixed_deploy` passed 4/4. The new case passed 1/1 before the final
assertions; the full file passed after them. `rustfmt --check` on the new Rust
file and `git diff --check` exited 0. Cargo ran from `/tmp` against the
repository manifest with stable Rust, explicit target directory and mold
linker flag to avoid inherited nightly-only parent Cargo settings. Fixture
`HOME` and `VIBE_SETTINGS` were temporary directories.
