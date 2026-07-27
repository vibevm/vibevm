# mcp:org.vibevm.ai-native/rust-ai-native-mcp {#root}

<status stage="doc" state="done" audience="user"/>

##SERVED-OVER-MCP-AS-ONE-STDIO-BINARY The AI-Native Rust discipline served over MCP: one stdio binary
(`rust-ai-native-mcp`), eighteen tools — the whole `rust-ai-native`
command surface (init, floor, the conform and specmap gates, trace,
test-gate, tripwire, health, fast-loop, codemod, ledger) plus the tcg
type oracle (validate / scope / complete / type / bench over a
persistent rust-analyzer session). @impl/done

- ##INSTALL-PULLS-THE-MATCHING-STACK-AT-THE-EXACT-PIN **Install:** `vibe install mcp:org.vibevm.ai-native/rust-ai-native-mcp` — the
  exact `=` pin pulls the matching `stack:org.vibevm.ai-native/rust-ai-native-lang`. @impl/done
- ##REGISTER-WRITES-THE-SERVER-INTO-THE-AGENT-CONFIG **Register:** `vibe mcp install` writes the server into your agent's
  config (a vibevm-managed entry; the command line is the slot's built
  artifact — `vibe bin build rust-ai-native-mcp` compiles it). @impl/done
- ##SERVE-NEEDS-NO-VIBE-IN-THE-RUNTIME-PATH **Serve:** the agent host launches the binary directly. No vibe in
  the runtime path — a consumer without vibe can build this package's
  workspace with cargo alone and wire the binary by hand. @impl/done

##server-brief-pointer The brief (tool inventory, the tool↔CLI parity map, the session and
capture semantics): [`spec/tools/discipline-mcp-rust.md`](spec/tools/discipline-mcp-rust.md). @impl/done

##PREREQUISITE-THE-RUST-ANALYZER-COMPONENT Prerequisite (inherited from the stack): `rustup component add
rust-analyzer`. @impl/done
