# ADV-CONTROL-MCP-PROGRESS-5

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** A notifications/cancelled naming the in-flight tools/call stops it cooperatively. For lifecycle_run that means the running handler process is killed and the response is the report with isError true and text starting "cancelled: ".

**Write.** `crates/vibe-mcp/src/lib.rs`, `crates/vibe-mcp/src/tools.rs`, `crates/vibe-mcp/src/tools/lifecycle_run.rs`, `crates/vibe-mcp/tests/server_control.rs`, `crates/vibe-mcp/src/skill_template.md`, `vibevm/vibespecs/modules/vibe-mcp/PROP-015-mcp-integration.xml`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-mcp --test server_control cancel`

**Test.** crates/vibe-mcp/tests/server_control.rs (reuses ScriptedTransport). Add PollTool, with the same entered: Mutex<mpsc::Sender<()>> and release: Mutex<mpsc::Receiver<()>> fields as GateTool. descriptor() names it "poll" with input_schema {"type":"object"}. Its `run` returns `self.run_controlled(args, ctx, &CallControl::inert())`. It overrides run_controlled: sends () on entered, then loops every 5 ms for up to 10 s, returning ToolOutput::executed(json!({}), "stopped") as soon as control.cancellation().is_cancelled(), or "released" when release receives. PollTool does NOT override controls_cancellation, so it also proves a default token still observes cancel(). (1) fn cancelled_notification_stops_the_in_flight_call: send tools/call id 1 "poll", wait for entered, send {"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":1,"reason":"test"}}, assert the response id 1 has content[0].text "stopped" within 10 s. (2) fn cancel_for_another_request_id_is_ignored: same, but cancel requestId 99, then release; assert the text is "released". (3) fn lifecycle_run_cancelled_mid_script_returns_a_cancelled_report: tempdir project with vibe.toml = `[package]\ngroup = "org.demo"\nname = "demo"\nkind = "flow"\nversion = "0.1.0"\n` plus `[[extension]] id="slow" point="phase:build" handler={kind="script", base="scripts/run"} inputs=["scripts/**"]`, scripts/run.sh = "sleep 30\n" and scripts/run.ps1 = "Start-Sleep -Seconds 30\n". Server built with ScriptedTransport, ServerContext::new(root).with_lifecycle_execution(InstallPolicy{offline:true,..Default::default()}, None, false), and register_tool(Box::new(vibe_mcp::tools::LifecycleRunMcpTool)) (or the builtin registration the server already performs). Send lifecycle_run phase build with _meta.progressToken "tok". Read lines until a notifications/progress whose _meta event has kind "start" and label "Running contribution org.demo/demo#slow" (timeout 30 s), then send notifications/cancelled with requestId = the call id. Assert the response arrives within 25 s with result.isError true, content[0].text starting "cancelled: ", and structuredContent.ok false.

**Read first.**
- crates/vibe-mcp/src/lib.rs
- crates/vibe-mcp/tests/server_control.rs
- crates/vibe-mcp/src/tools.rs:40-110
- crates/vibe-core/src/cancellation.rs:140-183
- crates/vibe-lifecycle/src/process/controlled.rs:40-130
- crates/vibe-mcp/src/skill_template.md:105-115

**Steps.**
1. In tools.rs add a provided McpTool method `fn controls_cancellation(&self) -> bool { false }` with a doc comment: true means the tool is run with a controlled CancellationToken::new(), which turns on per-row process kill, async HTTP and the native-helper requirement downstream. In lifecycle_run.rs override it to return true on LifecycleRunMcpTool.
2. In lib.rs add to Server a shared `in_flight: Arc<Mutex<Option<(Value, CancellationToken)>>>` and pass a clone to Dispatcher.
3. In Dispatcher::handle_tools_call create the token as `if tool.controls_cancellation() { CancellationToken::new() } else { CancellationToken::default() }` for every call, store `Some((req.id.clone(), token.clone()))` in in_flight before run_controlled, and reset it to None after the call returns, on both arms. Pass this token into CallControl::new instead of the ADV-CONTROL-MCP-PROGRESS-4 default.
4. In the reader thread, handle a Notification whose method is "notifications/cancelled": read params.requestId, and if in_flight holds Some((id, token)) with id == requestId, call token.cancel(). Ignore every other id and every other notification.
5. In skill_template.md, after the sentence ending "`lifecycle_run` again with the same phase.", add: "Send `_meta.progressToken` with the call to receive `notifications/progress` (each carries the `vibe-progress-event/1` event under `_meta[\"org.vibevm/progress-event\"]`). Send `notifications/cancelled` with the call's id to stop it at the next safe point; the response then starts with `cancelled:` and still carries the report."
6. Apply the spec_edit, add the three tests, run the check, then as regression gates `cargo test -p vibe-mcp --lib`, `cargo test -p vibe-mcp --test lifecycle_run`, `cargo test -p vibe-mcp --test lifecycle_tasks` and `cargo test -p vibe-cli --test cli_mcp_lifecycle_run`.

**Decisions already made.**
- Only tools that return true from controls_cancellation get a controlled CancellationToken::new(); today that is lifecycle_run alone. Every other tool gets CancellationToken::default(), which still records cancel() (clones share state, so is_cancelled() turns true) but leaves downstream behaviour unchanged.
- Accepted consequence for lifecycle_run: a controlled token makes vibe-registry use its async HTTP client and admits native rows only when a native helper is available. `vibe mcp serve` runs through vibe-cli main, which calls composition::native_helper::install() before any command, so the served process has the helper. The vibe-mcp test process has none, so the tests here use builtin and script rows only. The named lifecycle_run, lifecycle_tasks and cli_mcp_lifecycle_run suites are the regression gate; if any of them fails because of the controlled token, stop and report instead of widening the entry.
- Cancellation is cooperative: cancel() only requests a stop, and the call still returns its normal response stating the actual outcome. Per the MCP spec a cancelled request may still be answered, and this server always answers.
- Because calls are serial, only the one in-flight request can be cancelled. A cancel for a queued, finished or unknown id is ignored, as MCP allows.
- EOF still does not cancel (see ADV-CONTROL-MCP-PROGRESS-3).
- The cancelled-text wording comes from ADV-CONTROL-MCP-PROGRESS-2 and is not repeated or changed here.

**Not in this entry.** Cancelling queued requests, a server-wide stop on EOF, opting any tool other than lifecycle_run into controls_cancellation, binding the vibe-net session to the token, and any new structured "cancelled" member in the lifecycle report schema.

**Depends on.** ADV-CONTROL-MCP-PROGRESS-4

**Spec edit (apply as a step of this entry).**

vibevm/vibespecs/modules/vibe-mcp/PROP-015-mcp-integration.xml: insert immediately after the `</SERVER-PROGRESS>` line: `        <SERVER-CANCEL fact="true" status="impl/work">`notifications/cancelled` naming the in-flight request id requests cooperative cancellation of that call; any other id is ignored. The call still returns its response; a cancelled `lifecycle_run` returns its lifecycle report with `isError: true` and text starting `cancelled: `.</SERVER-CANCEL>`

Entries that write the same file run one after another, never in parallel.
