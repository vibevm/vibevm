# MCP-CORE v0.1 — the neutral MCP server transport {#root}

<status stage="spec" state="done"/>

@fact:status-line **Status:** SHIPPED with discipline-core 0.6.0 (MCP-SOVEREIGNTY-PLAN
Wave 2); the flow foundation is named core-ai-native since the
package-family rename (PROP-028), and the crate ships as
core-ai-native-mcp. @status:impl/done

@fact:TRANSPORT-EVERY-MCP-KIND-SERVER-BUILDS-ON The transport every `mcp`-kind package's server builds on:
this package owns what no single language owns, and a JSON-RPC loop is
exactly that. @status:impl/done

@fact:CONSUMERS-VENDOR-THIS-CRATE-BYTE-IDENTICALLY Consumers: the per-language discipline servers
(`rust-ai-native-mcp`, `typescript-ai-native-mcp`) vendor this crate
byte-identically (sync-engines) and mount their tool sets on it. @status:impl/done

@fact:RELATED-LAWS-LIVE-IN-THE-CONSUMING-REPO **Related:** the consumer-side kind and delivery laws live in the
consuming repo's PROP-027 (`spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-027`);
the tcg tool grammar in TCG-PROTOCOL-RUST-v0.1 / TCG-PROTOCOL-v0.1 (the
stacks) is transport-independent and unchanged by this mechanism. @status:impl/done

## 1. The wire {#wire}

@fact:kind-line-wire `req r1` @status:impl/done

@fact:WIRE-IS-LINE-DELIMITED-JSON-RPC-OVER-STDIO Line-delimited JSON-RPC 2.0 over stdio — one JSON object per line, no
Content-Length framing (that is the LSP convention; MCP hosts speak
lines, and the production vibe-mcp server has spoken exactly this shape
against every supported agent host since PROP-015). @status:impl/done

@fact:ANSWERED-PROTOCOL-REVISION The answered protocol revision is `2024-11-05`. @status:impl/done

@fact:inbound-frames-classify-by-id-lead Inbound frames classify by `id`: @status:impl/done

- @fact:FRAME-WITH-ID-IS-A-REQUEST present-and-non-null = a REQUEST that
  MUST be answered; @status:impl/done
- @fact:FRAME-WITHOUT-ID-IS-A-NOTIFICATION absent-or-null = a NOTIFICATION that MUST be absorbed
  without a response (hosts send `notifications/initialized` and
  cancellations). @status:impl/done

@fact:MALFORMED-LINE-MUST-NOT-KILL-THE-LOOP A malformed line — non-JSON, or a frame without
`method` — is answered with JSON-RPC `parse error` (-32700) and MUST
NOT kill the loop: the loop ends only at end-of-input or a dead
channel. @status:impl/done

@fact:TRANSPORT-FAILURE-IS-ONE-THISERROR-ENUM Transport-level failure is the layer's one `thiserror` enum,
each variant citing this mechanism and naming a fix surface. @status:impl/done

## 2. The loop {#server}

@fact:kind-line-server `req r1` @status:impl/done

@fact:SERVER-IS-NAME-VERSION-TOOLSET-OVER-A-TRANSPORT-SEAM The server is `(name, version, ToolSet)` driven over a `Transport` seam
(`read_line` / `write_line`; production = locked stdio, tests = a
scripted replay double — the whole loop tests with no agent host near
the suite). @status:impl/done

@fact:methods-lead Methods: @status:impl/done

- @fact:METHOD-INITIALIZE `initialize` → `{protocolVersion, serverInfo{name,version},
  capabilities:{tools:{listChanged:false}}}`. @status:impl/done
- @fact:METHOD-TOOLS-LIST `tools/list` → the registry's descriptors, stable (sorted) order. @status:impl/done
- @fact:METHOD-TOOLS-CALL `tools/call` → dispatch by `name` with `arguments`; a missing `name`
  is `invalid params` (-32602); an unknown tool is
  `method not found` (-32601). @status:impl/done
- @fact:METHOD-PING `ping` → `{}`. @status:impl/done
- @fact:METHOD-ANYTHING-ELSE Anything else → `method not found`. @status:impl/done

## 3. Tools {#toolset}

@fact:kind-line-toolset `req r1` @status:impl/done

@fact:TOOL-IS-DESCRIPTOR-PLUS-RUN A tool is `descriptor()` (name, description, `inputSchema` JSON schema)
plus `run(args) → ToolOutput{report, is_error}`. @status:impl/done

@fact:REGISTRY-LAST-REGISTRATION-OF-A-NAME-WINS The registry maps
name → tool; the last registration of a name wins. @status:impl/done

@fact:TOOL-FAILURE-IS-A-RESULT-NEVER-A-PROTOCOL-ERROR **Tool-level failure is a RESULT, never a protocol error**: a gate that
found findings, an oracle that refused — these answer
`{content:[{type:"text",text:report}], isError:true}` so the agent
reads the report; protocol errors are reserved for the transport
grammar itself (unknown tool, malformed params). @status:impl/done

@fact:REPORTS-SPEAK-THE-CLASS-F-REQ-CITING-GRAMMAR Reports speak the
Class-F REQ-citing grammar of the runners they wrap. @status:impl/done

## 4. The capture guard {#capture}

@fact:kind-line-capture `req r1` @status:impl/done

@fact:CAPTURE-IS-A-PROCESS-LEVEL-REDIRECT Everything a tool run says — including CHILD processes (a floor's
cargo, prettier, node) — goes to the process's stderr channel, so the
only capture that sees a whole run is a process-level redirect around
the call: `dup2` over fd 2 on unix, `SetStdHandle(STD_ERROR_HANDLE, …)`
on Windows, into a temp FILE (a file, not a pipe — a filling pipe
blocks the writer and deadlocks a chatty floor). @status:impl/done

@fact:threaded-write-was-rejected-at-the-spike A threaded
`&mut dyn Write` was rejected at the plan's Wave-0 spike: children
inherit the process handle and bypass it entirely. @status:spec/done

@fact:LAWS-CAPTURES-MUST-NOT-NEST-OR-RUN-CONCURRENTLY Laws: the redirect is process-global, so captures MUST NOT nest or run
concurrently (the loop dispatches tools sequentially — that is the
licence); a second simultaneous capture refuses with this unit cited. @status:impl/done

@fact:RESTORATION-RIDES-DROP Restoration rides `Drop`, so a panicking tool cannot leave the process
mute. @status:impl/done

@fact:CAPTURE-CELL-IS-THE-AUDITED-HOME-OF-UNSAFETY This cell is the crate's audited home of raw-descriptor
unsafety — nothing else touches fds or std handles. @status:impl/done

## 5. No prompts, no vibe {#no-prompts}

@fact:kind-line-no-prompts `req r1` @status:impl/done

@fact:NO-INTERACTIVE-CHANNEL-TOOLS-MUST-NOT-PROMPT A server has no interactive channel: tools MUST NOT prompt — anything
that would ask becomes an explicit tool parameter (`force`-class flags
included). @status:impl/done

@fact:CRATE-KNOWS-NEITHER-VIBE-NOR-A-LANGUAGE And nothing in this crate knows vibe, a language, or a
discipline rule: a server built on it serves with `vibe` absent from
`PATH` — the consuming repo's PROP-027 §2.6 turns that property into an
acceptance test. @status:impl/done
