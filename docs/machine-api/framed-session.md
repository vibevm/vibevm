# On-demand framed machine session

The current machine session is a library adapter over a supplied reader and
writer. It runs only when an owning caller constructs
`MachineSessionRunner` with a `MachineSession` built from trusted context
sources. It does not install a daemon or replace the MCP server. There is no
public `vibe` command that starts this framed session yet. The only admitted
domain exchange is the pure text migration planner; action dispatch and a
general IDE protocol are later work.

## Frame and handshake

Each frame starts with a four-byte unsigned big-endian byte count, followed by
exactly that many UTF-8 JSON bytes. The count must be 1–8,388,608. The
generated `machine-session-frame` epoch-one reader rejects unknown shape and
fields; the codec rejects zero length, truncation, over-limit length before
body allocation, invalid UTF-8/JSON and an unsupported frame epoch. EOF before
the first prefix byte closes the session. A malformed frame closes it before
domain dispatch. Output is flushed once per frame.

The first accepted message is an `initialize_offer` with a request ID, API
versions, schema offers, position encodings and requested capability flags.
The current implementation selects API version `1`, `raw-byte-offsets`, and
epoch one of all five registered formats: `machine-session-frame`,
`machine-operation-envelope`, `migration-plan-request`,
`migration-plan-report` and `migration-plan-error`. An offer missing any
required intersection receives `initialization_refused` with its specific
reason and the actual supported sets. The reply advertises `streaming = false`,
`replay = false`, `control = false`; the presence of an `event` frame shape
does not mean streaming is available. Requests before initialization, with a
foreign session ID, repeated request ID or unbound context/target ID refuse.

The frame schema has seven bodies: `initialize_offer`, `initialized`,
`initialization_refused`, `request`, `event`, `complete` and `refusal`. The
request's `envelope_json` is a string carrying a registered operation envelope,
not arbitrary executable JSON. Outer frame validation only establishes
session/context identity. The runner separately decodes and validates the
domain envelope before the pure planner exchange. Completed and refused
replies carry an explicit outcome; this path reports `no_effects`.

## Trusted contexts and current route

The owning surface supplies already-composed `ServerContext` values and any
target roots to `MachineSession::new`. The session gives them opaque,
connection-local context and target IDs. The client sees IDs, kind and
availability, never the private root paths. Workspace and member contexts can
be available when supplied by the owner. HOME and container contexts are
reported unavailable with reason `owner-not-bound`. A client request or
manifest cannot register another root. IDs from another connection do not
authorize this one, and EOF removes the bindings.

For now, the runner accepts a request only with no target ID and routes its
validated `machine-operation-envelope` through the orchestrator's neutral
pure-planner exchange. The MCP machine-operation tool uses that same exchange.
Unavailable or invalid domain requests produce a refusal; a diagnostic that
cannot safely be published closes the runner with its original evidence in the
error channel. The current source capture is finite but does not authenticate
every source byte. Persistent cross-request cursors and effectful operation
control have not been implemented.

## Coexistence and source of truth

The legacy MCP server still uses JSON-RPC 2.0 as newline-delimited stdio and
its observed fixed initialize protocol version `2024-11-05`. That fixed MCP
response is not the machine-session negotiation. Existing MCP tools retain
their line transport. A library or one-shot CLI domain call can use the pure
planner without starting either server; using the framed adapter requires an
explicit caller-owned reader/writer.

The governing law is [PROP-080](../../vibevm/vibespecs/common/PROP-080-machine-operations-and-observability.xml).
The executable boundaries are the [frame codec](../../crates/vibe-mcp/src/machine_session/framing.rs),
[session state](../../crates/vibe-mcp/src/machine_session/state.rs),
[runner](../../crates/vibe-mcp/src/machine_session/runner.rs),
[registered schema](../../schemas/machine/e1/session_frame.jtd.json) and
[neutral domain exchange](../../crates/vibe-orchestrator/src/machine_operation/exchange.rs).
