# ADV-CONTROL-MCP-PROGRESS-4

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** A tools/call carrying params._meta.progressToken receives notifications/progress lines before its response, each carrying the same vibe-progress-event/1 object the CLI streams. A call without a token gets only its response.

**Write.** `crates/vibe-mcp/src/lib.rs`, `crates/vibe-mcp/src/progress_notify.rs`, `crates/vibe-mcp/tests/lifecycle_run/transport.rs`, `vibevm/vibespecs/modules/vibe-mcp/PROP-015-mcp-integration.xml`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-mcp --test lifecycle_run progress_token`

**Test.** crates/vibe-mcp/tests/lifecycle_run/transport.rs. Fixture: project("") plus a row `[[extension]] id="first" point="phase:build" handler={kind="builtin",name="log"} config={message="FIRST-RAN"}`, with ctx = support::context(root). Request: {"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"lifecycle_run","arguments":{"phase":"build"},"_meta":{"progressToken":"tok-1"}}} sent through vibe_mcp::dispatch_one; split the output into lines. (1) fn progress_token_streams_notifications_before_the_response: at least 2 lines; every line except the last has method "notifications/progress", params.progressToken "tok-1", strictly increasing integer params.progress, a non-empty string params.message, and params._meta["org.vibevm/progress-event"].protocol == "vibe-progress-event/1"; at least one such event has kind "start" and label "Running contribution org.demo/demo#first"; the last line has id 7 and result.isError false. (2) fn no_progress_token_sends_only_the_response: same request without `_meta`; the output is exactly one line, with id 7 and result.isError false.

**Read first.**
- crates/vibe-mcp/src/lib.rs:290-352
- crates/vibe-mcp/src/tools.rs:40-70
- crates/vibe-wire/src/behaviour/progress_event.rs
- crates/vibe-mcp/tests/lifecycle_run/support.rs:1-80
- crates/vibe-mcp/tests/lifecycle_run/transport.rs:1-45

**Steps.**
1. Create crates/vibe-mcp/src/progress_notify.rs with `pub(crate) struct McpProgressObserver { token: Value, sink: Arc<dyn LineSink>, encoder: Mutex<vibe_wire::behaviour::progress_event::ProgressEventEncoder> }` and `pub(crate) fn new(token: Value, sink: Arc<dyn LineSink>) -> Self`. Its ProgressObserver::observe locks the encoder (returning silently on poison), encodes the event with at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true), and writes one line through the sink: `json!({"jsonrpc":"2.0","method":"notifications/progress","params":{"progressToken": token, "progress": event.seq, "message": message, "_meta": {"org.vibevm/progress-event": event}}})`, ignoring write errors.
2. message is "{label}: {message}" when both exist, otherwise the label, otherwise the message, otherwise the serde string of the event's kind (e.g. "finish").
3. Add `mod progress_notify;` to lib.rs. Add a field `sink: Option<Arc<dyn LineSink>>` to Dispatcher, set from Server::new. In Dispatcher::handle_tools_call read `params["_meta"]["progressToken"]`, keeping it only when it is a JSON string or integer.
4. Build the per-call control: progress = Progress::new(Arc::new(McpProgressObserver::new(token, sink))) when both the token and the sink exist, else Progress::default(). Use `CallControl::new(progress, CancellationToken::default())` and call `tool.run_controlled(&args, &self.context, &control)` instead of `tool.run`.
5. Apply the spec_edit, add the two tests, run the check, then `cargo test -p vibe-mcp --test lifecycle_run`, `cargo test -p vibe-mcp --test server_control` and `cargo test -p vibe-mcp --lib`.

**Decisions already made.**
- Notifications are sent only when the client supplies a string or integer progressToken (the MCP rule); the server never sends progress unasked.
- params.progress = the event's seq; total is omitted; the full wire event goes under params._meta key "org.vibevm/progress-event". This gives MCP and the CLI one shared vocabulary.
- PROTOCOL_VERSION stays "2024-11-05" and initialize advertises nothing new.
- The cancellation token here stays the inert default, so behaviour without a token is byte-identical to today. A controlled token arrives in ADV-CONTROL-MCP-PROGRESS-5.
- Notifications are written from the dispatching thread through the shared LineSink, so they always precede that call's response.

**Not in this entry.** notifications/cancelled handling, sending a `total`, and progress for tools other than lifecycle_run (they ignore the control by default).

**Depends on.** ADV-CONTROL-MCP-PROGRESS-1, ADV-CONTROL-MCP-PROGRESS-2, ADV-CONTROL-MCP-PROGRESS-3

**Spec edit (apply as a step of this entry).**

vibevm/vibespecs/modules/vibe-mcp/PROP-015-mcp-integration.xml: insert immediately after the `</SERVER-PING-WHILE-BUSY>` line: `        <SERVER-PROGRESS fact="true" status="impl/work">A `tools/call` whose `params._meta.progressToken` is a string or integer receives `notifications/progress` while it runs: `progress` is the event's sequence number, `message` a short text, and `_meta["org.vibevm/progress-event"]` the same `vibe-progress-event/1` object the CLI machine stream writes. Without a token no notification is sent.</SERVER-PROGRESS>`

Entries that write the same file run one after another, never in parallel.
