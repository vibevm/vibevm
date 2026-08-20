# rust-ai-native-mcp — the server brief {#root}

**What it is.** The AI-Native Rust discipline served over MCP: one
stdio binary, eighteen tools, launched by an agent host straight from
this package's slot. Serving needs no vibe anywhere on the machine
(PROP-027 §2.6 in the consuming repo; the live chain scrubs PATH to
prove it). Transport: the vendored `mcp-core` (line-delimited JSON-RPC
2.0, protocol `2024-11-05` — MCP-CORE-v0.1).

**One engine, one truth.** The tools call the SAME lib fns the CLIs
call, and the `=X.Y.Z` pin on `stack:org.vibevm/rust-ai-native-lang` holds
this package's vendored copies and the consumer's installed stack to
one resolved version set. Tool-level failure (a red gate, a refusing
oracle) is an `isError` RESULT carrying the report; protocol errors are
reserved for the transport grammar.

**Reports are whole.** Every discipline tool runs inside the
`mcp-core::capture` guard, so the agent's report carries the run's
entire story — the runner's own words AND its child processes (cargo,
rustfmt, clippy, nextest).

## The parity map {#parity-map}

Tool ↔ the CLI invocation it is parity-locked to (the enumeration test
pins the list; the descriptions in `tools/list` restate each row):

| Tool | CLI |
|---|---|
| `init` | `rust-ai-native init [--namespace] [--force]` |
| `floor` | `rust-ai-native floor [--keep-going] [--fast-loop]` |
| `conform_check` | `rust-ai-native-conform check [--scope] [--baseline]` |
| `conform_freeze` | `rust-ai-native-conform freeze [--baseline]` |
| `specmap_check` | `rust-ai-native-specmap --check` |
| `specmap_write` | `rust-ai-native-specmap` |
| `trace_explain` | `rust-ai-native trace <target> [--json] [--prose]` |
| `test_gate` | `rust-ai-native test-gate [--baseline]` |
| `tripwire` | `rust-ai-native tripwire [--base] [--debt]` |
| `health` | `rust-ai-native health [--out]` |
| `fast_loop` | `rust-ai-native fast-loop [--cell] [--budget-secs] [--enforce-budget]` |
| `codemod_add_cell` | `rust-ai-native codemod add-cell <crate> <cell> <seam> <variant> <spec-uri>` |
| `ledger_render` | `rust-ai-native ledger render [--check]` |
| `tcg_validate` | `rust-ai-native-tcg validate <file> [--content-from]` |
| `tcg_scope` | `rust-ai-native-tcg scope <file> [--position]` |
| `tcg_complete` | `rust-ai-native-tcg complete <file> --position [--prefix] [--max]` |
| `tcg_type` | `rust-ai-native-tcg type <file> --position` |
| `tcg_bench` | `rust-ai-native-tcg bench --corpus --report` |

`rust-ai-native-specmap --gate` (the package-self-trace form) stays CLI-only: its
audience is package gates, not agents.

## The discipline half {#discipline-tools}

Thirteen thin adapters over `rust_ai_native_cli` / `rust_ai_native_conform`
/ `rust_ai_native_specmap` lib fns. Every schema carries the optional
`language` property; a non-`rust` value refuses with the recipe naming
that language's own server — never another language's fix surface
(PROP-026 §2 continuity). Heavy tools (`floor`, `test_gate`,
`fast_loop`, `tcg_bench`) say «expect minutes» in their descriptions;
nothing prompts — `force`-class decisions are explicit parameters.

## The tcg half {#tcg-tools}

The four oracle ops + the bench harness over ONE persistent
rust-analyzer session shared by all five tools: lazy-spawned on first
use, respawned ONCE per op on a crashed session (the serve relay's
posture, server-local now). Enrichment goes through
`rust_ai_native_tcg::enrich_validate` — the gate's own rules over the gate's
own frontend — and the policy (conform.toml + frozen ratchet) reloads
per call, so a mid-session freeze is honoured immediately.
`tcg_validate`'s `isError` mirrors the one-shot exit contract: an
error-grade diagnostic OR a non-baselined finding. The NDJSON serve
relay (`rust-ai-native-tcg serve`, TCG-PROTOCOL-RUST-v0.1) remains shipped in the
stack as the non-MCP embedding form.
