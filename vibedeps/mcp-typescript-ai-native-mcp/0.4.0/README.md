# mcp:org.vibevm/typescript-ai-native-mcp

The AI-Native TypeScript discipline served over MCP: one stdio binary
(`discipline-mcp-typescript`), seventeen tools — the whole
`discipline-typescript` command surface (init, the seven-step floor,
the ts-tsc conform and specmap gates, trace, test-gate, tripwire,
health, fast-loop, codemod) plus the tcg type oracle (validate / scope
/ complete / type / bench over a persistent LanguageService session —
tsc's own engine, agreement by construction).

- **Install:** `vibe install mcp:org.vibevm/typescript-ai-native-mcp` —
  the exact `=` pin pulls the matching
  `stack:org.vibevm/typescript-ai-native-lang`.
- **Register:** `vibe mcp install` writes the server into your agent's
  config (a vibevm-managed entry; `vibe bin build
  discipline-mcp-typescript` compiles the artifact).
- **Serve:** the agent host launches the binary directly. No vibe in
  the runtime path.

The brief (tool inventory, the tool↔CLI parity map, the session and
capture semantics):
[`spec/tools/discipline-mcp-typescript.md`](spec/tools/discipline-mcp-typescript.md).
Prerequisite (inherited from the stack): node ≥ 22.6 and the project's
own `typescript` devDependency — absent tools hard-fail with the
install recipe, through MCP as on the CLI.
