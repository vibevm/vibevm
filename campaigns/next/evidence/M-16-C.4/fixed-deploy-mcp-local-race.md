# Fixed Deploy MCP/local route race

Zap Current revision 11, strategy `strategy.vibevm-next.coarse-c4.1`; verified clean baseline `035cbe01f` on `next`. This is one focused M-16-C.4 test atom, not C.4 acceptance or a full gate panel.

`fixed_deploy_mcp_and_local_route_race_one_key` releases a fixed Deploy MCP submit and a local `FixedDeployRoute::exchange_local` submit against the same C1 key together. It requires a completed cold retry, one retained bound Deploy result under the run ID, a verified receipt with the expected resources, and identical cold MCP/local replies. A raced surface must observe that same run; any run ID returned by either surface must agree.

The worker's private-target nightly run passed 1/1 (`182 filtered`). The coordinator reviewed the diff and independently repeated the focused `vibe-mcp --lib` test with the installed nightly and the same private target: 1/1 passed (`182 filtered`). `rustfmt --edition 2024` and `git diff --check` passed. No full gate, production change, public release, or historical task recheck was performed.

This in-process cross-adapter test does not grant a durable Deploy pointer or decide B-223 recovery-only reattach.
