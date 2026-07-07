# rust-demo

The committed Rust consumer testbed (AGENTIC-TCG-RUST-PLAN v0.1,
Phase 2): a real vibe project — its own `vibe.toml` resolved from the
in-repo registry, the rust-ai-native 0.6.0 family (the
rust-ai-native-lang stack + its mcp server, PROP-028) materialised
into `vibedeps/`, the full discipline floor green — mirroring
`research/ts-demo` cell for cell so the two languages' oracles answer
the same shaped questions.

- Cells `greeting` / `farewell` over `core::text`; the `GuestName`
  NEWTYPE has a private inner and `parse_guest_name` as its only
  constructor — the compiler (E0603) enforces what ts-demo's brand
  enforces type-side. That privacy is also the differential corpus's
  documented-gap exhibit (case 06): rust-analyzer's native diagnostics
  do not report it at 1.93.1, cargo check does.
- Floor: `vibe bin exec discipline-rust -- floor` — fmt → test →
  clippy → conform → specmap → test-gate, one exit code. The conform
  baseline is frozen EMPTY (Rust needs no irreducible cast where TS
  froze one).
- The oracle: `vibe bin exec tcg-rust -- validate src/cells/greeting.rs
  --root .` (or the `tcg_*` MCP tools with `language: "rust"`).
  Prerequisite: `rustup component add rust-analyzer`.

Setup from a fresh clone:

```sh
cd research/rust-demo
vibe install --registry ../../packages --assume-yes
cargo test
vibe bin exec discipline-rust -- floor
```
