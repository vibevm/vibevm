# AI-Native Rust (stack:org.vibevm.ai-native/rust-ai-native)

The **family aggregator** for AI-Native Rust (PROP-028). This package
carries no code and no prompt content of its own — installing it pulls the
whole family at one exact-pinned version set:

- `stack:org.vibevm.ai-native/rust-ai-native-lang` — the language stack: the Rust
  GUIDE, the cards, and the runnable toolchain (`rust-ai-native`,
  `rust-ai-native-conform`, `rust-ai-native-specmap`, `rust-ai-native-tcg`).
- `mcp:org.vibevm.ai-native/rust-ai-native-mcp` — the same toolchain served over MCP
  (PROP-027), version-mirroring the stack.
- `flow:org.vibevm.ai-native/core-ai-native` — the language-neutral discipline core,
  arriving transitively through the stack.

Require `stack:org.vibevm.ai-native/rust-ai-native` (`^0.6`) to get the whole family;
require `rust-ai-native-lang` alone if you want the stack without the MCP
server. The consumer front door — wiring, floor, sweep — is documented in
the `-lang` package's README and `spec/rust/GUIDE-AI-NATIVE-RUST.md`.
