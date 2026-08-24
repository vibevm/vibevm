# mcp:org.vibevm.ai-native/rust-ai-native-mcp {#root}

<status stage="doc" state="done" audience="user"/>

@fact:SERVED-OVER-MCP-AS-ONE-STDIO-BINARY The AI-Native Rust discipline served over MCP: one stdio binary
(`rust-ai-native-mcp`), eighteen tools — the whole `rust-ai-native`
command surface (init, floor, the conform and specmap gates, trace,
test-gate, tripwire, health, fast-loop, codemod, ledger) plus the tcg
type oracle (validate / scope / complete / type / bench over a
persistent rust-analyzer session). @status:impl/done

- @fact:INSTALL-PULLS-THE-MATCHING-STACK-AT-THE-EXACT-PIN **Install:** `vibe install mcp:org.vibevm.ai-native/rust-ai-native-mcp` — the
  exact `=` pin pulls the matching `stack:org.vibevm.ai-native/rust-ai-native-lang`. @status:impl/done
- @fact:REGISTER-WRITES-THE-SERVER-INTO-THE-AGENT-CONFIG **Register:** `vibe mcp install` writes the server into your agent's
  config (a vibevm-managed entry; the command line is the slot's built
  artifact — `vibe bin build rust-ai-native-mcp` compiles it). @status:impl/done
- @fact:SERVE-NEEDS-NO-VIBE-IN-THE-RUNTIME-PATH **Serve:** the agent host launches the binary directly. No vibe in
  the runtime path — a consumer without vibe can build this package's
  workspace with cargo alone and wire the binary by hand. @status:impl/done

@fact:server-brief-pointer The brief (tool inventory, the tool↔CLI parity map, the session and
capture semantics): [`spec/tools/discipline-mcp-rust.xml`](spec/tools/discipline-mcp-rust.xml). @status:impl/done

@fact:PREREQUISITE-THE-RUST-ANALYZER-COMPONENT Prerequisite (inherited from the stack): `rustup component add
rust-analyzer`. @status:impl/done

