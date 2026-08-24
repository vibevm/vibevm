# mcp:org.vibevm.ai-native/typescript-ai-native-mcp {#root}

<status stage="doc" state="done" audience="user"/>

@fact:SERVED-OVER-MCP-AS-ONE-STDIO-BINARY The AI-Native TypeScript discipline served over MCP: one stdio binary
(`typescript-ai-native-mcp`), seventeen tools — the whole
`typescript-ai-native` command surface (init, the seven-step floor,
the ts-tsc conform and specmap gates, trace, test-gate, tripwire,
health, fast-loop, codemod) plus the tcg type oracle (validate / scope
/ complete / type / bench over a persistent LanguageService session —
tsc's own engine, agreement by construction). @status:impl/done

- @fact:INSTALL-PULLS-THE-MATCHING-STACK-AT-THE-EXACT-PIN **Install:** `vibe install mcp:org.vibevm.ai-native/typescript-ai-native-mcp` —
  the exact `=` pin pulls the matching
  `stack:org.vibevm.ai-native/typescript-ai-native-lang`. @status:impl/done
- @fact:REGISTER-WRITES-THE-SERVER-INTO-THE-AGENT-CONFIG **Register:** `vibe mcp install` writes the server into your agent's
  config (a vibevm-managed entry; `vibe bin build
  typescript-ai-native-mcp` compiles the artifact). @status:impl/done
- @fact:SERVE-NEEDS-NO-VIBE-IN-THE-RUNTIME-PATH **Serve:** the agent host launches the binary directly. No vibe in
  the runtime path. @status:impl/done

@fact:server-brief-pointer The brief (tool inventory, the tool↔CLI parity map, the session and
capture semantics):
[`spec/tools/discipline-mcp-typescript.xml`](spec/tools/discipline-mcp-typescript.xml). @status:impl/done

@fact:PREREQUISITE-NODE-AND-THE-PROJECTS-OWN-TYPESCRIPT Prerequisite (inherited from the stack): node ≥ 22.6 and the project's
own `typescript` devDependency — absent tools hard-fail with the
install recipe, through MCP as on the CLI. @status:impl/done

