# AI-Native TypeScript (stack:org.vibevm.ai-native/typescript-ai-native)

The **family aggregator** for AI-Native TypeScript (PROP-028). This package
carries no code and no prompt content of its own — installing it pulls the
whole family at one exact-pinned version set:

- `stack:org.vibevm.ai-native/typescript-ai-native-lang` — the language stack: the
  TypeScript GUIDE, the cards, and the runnable toolchain
  (`typescript-ai-native`, `typescript-ai-native-conform`, `typescript-ai-native-specmap`,
  `typescript-ai-native-tcg`).
- `mcp:org.vibevm.ai-native/typescript-ai-native-mcp` — the same toolchain served over
  MCP (PROP-027), version-mirroring the stack.
- `flow:org.vibevm.ai-native/core-ai-native` — the language-neutral discipline core,
  arriving transitively through the stack.

Require `stack:org.vibevm.ai-native/typescript-ai-native` (`^0.5`) to get the whole
family; require `typescript-ai-native-lang` alone if you want the stack
without the MCP server. The consumer front door — wiring, floor, sweep — is
documented in the `-lang` package's README and
`spec/typescript/GUIDE-AI-NATIVE-TYPESCRIPT.md`.
