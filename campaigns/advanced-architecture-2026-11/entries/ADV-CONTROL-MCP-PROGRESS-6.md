# ADV-CONTROL-MCP-PROGRESS-6

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** When an offline-started MCP server's lifecycle_run fails because it needed the network, the response text starts "offline: " and tells the agent to restart `vibe mcp serve` with offline mode off.

**Write.** `crates/vibe-mcp/src/tools/lifecycle_run.rs`, `crates/vibe-mcp/tests/lifecycle_run/dependency.rs`, `vibevm/vibespecs/modules/vibe-mcp/PROP-015-mcp-integration.xml`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-mcp --test lifecycle_run offline_server_names_the_restart_remedy`

**Test.** crates/vibe-mcp/tests/lifecycle_run/dependency.rs, fn offline_server_names_the_restart_remedy: project("\n[requires]\npackages = { \"org.absent/tool\" = \"^0.1\" }\n") with no registry package written. Runs run(&context(root), "install") (support::context is offline) and unwraps Ok. Asserts output.is_error(), output.text().starts_with("offline: "), and output.text().ends_with("; this MCP server was started offline — restart `vibe mcp serve` without --offline, VIBE_OFFLINE or [net].offline").

**Read first.**
- crates/vibe-mcp/src/tools/lifecycle_run.rs:180-230
- crates/vibe-mcp/src/context.rs:60-75
- crates/vibe-mcp/tests/lifecycle_run/dependency.rs:1-30
- crates/vibe-mcp/tests/lifecycle_run/support.rs:30-70

**Steps.**
1. In lifecycle_run.rs add `fn offline_remedy(offline: bool, chain: String) -> String` that returns `format!("offline: {chain}; this MCP server was started offline — restart `vibe mcp serve` without --offline, VIBE_OFFLINE or [net].offline")` when `offline && chain.contains("offline")`, and returns chain unchanged otherwise.
2. In the executed-failure arm (`Some(original)`), when the cancelled branch from ADV-CONTROL-MCP-PROGRESS-2 does not apply, set the text to `offline_remedy(ctx.lifecycle_policy.offline, format!("{original:#}"))`.
3. Apply the spec_edit, add the test, run the check, then `cargo test -p vibe-mcp --test lifecycle_run`.

**Decisions already made.**
- Per-call offline or install overrides stay out: the policy is fixed at server start. Only the refusal's wording changes.
- The trigger is the server's offline posture plus the substring "offline" in the error chain, which covers both the `--offline: no local registry` bail and `not resolvable offline`.
- Exact suffix text: "; this MCP server was started offline — restart `vibe mcp serve` without --offline, VIBE_OFFLINE or [net].offline". It names all three sources because vibe-cli resolves the posture as flag OR VIBE_OFFLINE OR user-config [net].offline (crates/vibe-cli/src/output.rs resolve_offline).
- Only the executed-failure text changes; the structured report and the PreExecution path stay as they are.

**Not in this entry.** Adding an `offline` argument to lifecycle_run's schema, or changing the vibe-registry / vibe-package-source error messages.

**Depends on.** ADV-CONTROL-MCP-PROGRESS-2

**Spec edit (apply as a step of this entry).**

vibevm/vibespecs/modules/vibe-mcp/PROP-015-mcp-integration.xml: insert immediately after the line ending `</SERVER-MISSING-LOCK>`: `        <SERVER-OFFLINE-REMEDY fact="true" status="impl/work">Offline and install policy are fixed when the server starts. When an offline-started server's `lifecycle_run` fails because it needed the network, the text reads `offline: <error chain>; this MCP server was started offline — restart `vibe mcp serve` without --offline, VIBE_OFFLINE or [net].offline`.</SERVER-OFFLINE-REMEDY>`

Entries that write the same file run one after another, never in parallel.
