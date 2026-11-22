# ADV-CONTROL-MCP-PROGRESS-3

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** The MCP server answers ping while a tools/call is still executing, and tool calls still run one at a time in arrival order.

**Write.** `crates/vibe-mcp/src/transport.rs`, `crates/vibe-mcp/src/lib.rs`, `crates/vibe-mcp/tests/server_control.rs`, `vibevm/vibespecs/modules/vibe-mcp/PROP-015-mcp-integration.xml`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-mcp --test server_control`

**Test.** New file crates/vibe-mcp/tests/server_control.rs, starting with `use vibe_test_support as _;`. Helper ScriptedTransport implements vibe_mcp::Transport: input from std::sync::mpsc::Receiver<String>, where read_line returns Some(line + "\n") and returns None once the sender is dropped. Output goes through `Arc<ScriptedSink>`, which implements vibe_mcp::LineSink by sending each line on a Mutex<mpsc::Sender<String>>; write_line delegates to the sink and sink() returns Some(the Arc). Helper GateTool holds fields active: Arc<AtomicUsize>, max_active: Arc<AtomicUsize>, entered: Mutex<mpsc::Sender<()>>, release: Mutex<mpsc::Receiver<()>>. descriptor() returns ToolDescriptor { name: "gate", description: "test gate", input_schema: json!({"type":"object"}) }. run increments active, records the maximum in max_active, sends () on entered, then polls release every 5 ms for up to 10 s, decrements active, and returns ToolOutput::executed(json!({"released":true}), "released") or ToolOutput::executed(json!({}), "timed out"). The server is built and run inside std::thread::spawn: `let mut server = Server::new(transport, ServerContext::new(tempdir.path().to_path_buf())); server.register_tool(Box::new(GateTool { .. })); server.run()`. (1) fn ping_is_answered_while_a_tool_call_runs: sends tools/call id 1 for gate, waits (recv_timeout 10 s) on entered, sends {"jsonrpc":"2.0","id":2,"method":"ping"}, asserts the next output line within 10 s has id 2 and result {}; then releases the gate and asserts the next line has id 1 with result.content[0].text "released"; drops the input sender and asserts the server thread's run() returned Ok. (2) fn tool_calls_run_one_at_a_time_in_order: sends two gate calls (ids 1, 2), releases twice, collects two responses, asserts the ids arrive in order 1, 2 and max_active == 1.

**Read first.**
- crates/vibe-mcp/src/transport.rs:1-163
- crates/vibe-mcp/src/lib.rs:185-260
- crates/vibe-mcp/src/lib.rs:290-411
- crates/vibe-mcp/src/jsonrpc.rs:20-70
- crates/vibe-mcp/src/tools/output.rs:160-245

**Steps.**
1. In transport.rs add `pub trait LineSink: Send + Sync { fn write_line(&self, line: &str) -> io::Result<()>; }` and a provided method on Transport: `fn sink(&self) -> Option<std::sync::Arc<dyn LineSink>> { None }`. Add a private `struct StdoutSink;` whose write_line locks io::stdout(), writes the line plus "\n" and flushes. Remove the `writer: io::Stdout` field from StdioTransport and from StdioTransport::new (it would otherwise be dead code under clippy -D warnings). StdioTransport::sink returns Some(Arc::new(StdoutSink)) and StdioTransport::write_line delegates to StdoutSink.
2. Change MemoryTransport.output to `Arc<Mutex<Vec<u8>>>`. Add a private `struct MemorySink(Arc<Mutex<Vec<u8>>>)` implementing LineSink with today's append logic. MemoryTransport::sink returns Some(Arc::new(MemorySink(self.output.clone()))), write_line delegates to it, and take_output keeps its signature and result. Re-export LineSink from lib.rs next to Transport.
3. In lib.rs move `context` and `tools` into a private `struct Dispatcher { context: ServerContext, tools: BTreeMap<String, Box<dyn McpTool>> }` and move dispatch, dispatch_request and handle_* onto it unchanged. Server becomes `{ transport: T, sink: Option<Arc<dyn LineSink>>, core: Dispatcher }`. Server::new sets `sink = transport.sink()`; register_tool inserts into core.tools.
4. Split Server::run (add `where T: Send`). If self.sink is None, run today's serial loop (rename the existing body to run_serial). Otherwise use std::thread::scope with a reader thread that owns `&mut self.transport`, a clone of the sink and an mpsc::Sender<JsonRpcMessage>. The reader skips blank lines, writes a parse error with `JsonRpcResponse::error(Value::Null, JsonRpcError::parse_error(..))` through the sink, answers a Request whose method is "ping" at once with `JsonRpcResponse::ok(id, json!({}))` through the sink, drops every Notification, and sends every other Request to the channel. On EOF it drops the sender and returns Ok; it returns read errors.
5. The main thread iterates the receiver, calls `self.core.dispatch(msg)` and writes each response through the sink in order. After the loop it joins the reader and propagates its io error as ServerError::Transport.
6. Apply the spec_edit, add the tests, run the check, then `cargo test -p vibe-mcp --lib`, `cargo test -p vibe-mcp --doc`, `cargo test -p vibe-cli --test cli_mcp_lifecycle_run` and `cargo clippy -p vibe-mcp --all-targets -- -D warnings`.

**Decisions already made.**
- Only the reader thread is new. Tools and ServerContext stay on the calling thread, so McpTool needs no Send/Sync bound.
- EOF is not a cancellation: queued and in-flight calls finish and their responses are written before run() returns. This keeps piped batches working (crates/vibe-cli/tests/cli_mcp_lifecycle_run.rs writes the request and closes stdin at once).
- Every write goes through the one LineSink, which writes whole lines under a lock, so frames never interleave.
- A custom Transport whose sink() returns None keeps the exact serial behaviour.
- PROTOCOL_VERSION and the initialize capabilities stay unchanged.

**Not in this entry.** Progress notifications, notifications/cancelled handling, per-call CallControl in the dispatcher, and any change to the vibe-cli `vibe mcp serve` command.

**Spec edit (apply as a step of this entry).**

vibevm/vibespecs/modules/vibe-mcp/PROP-015-mcp-integration.xml: insert immediately after the line ending `</SERVER-MISSING-LOCK>` (inside <server>'s <facts>): `        <SERVER-PING-WHILE-BUSY fact="true" status="impl/work">A reader thread answers `ping` at once, even while a `tools/call` executes; tool calls run strictly one at a time in arrival order. End of input lets queued and in-flight calls finish and write their responses, then the server exits — end of input is not a cancellation.</SERVER-PING-WHILE-BUSY>`

Entries that write the same file run one after another, never in parallel.
