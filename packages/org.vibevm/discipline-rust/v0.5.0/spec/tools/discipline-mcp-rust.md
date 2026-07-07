# discipline-mcp-rust — the server brief {#root}

**What it is.** The AI-Native Rust discipline served over MCP: one
stdio binary, eighteen tools, launched by an agent host straight from
this package's slot. Serving needs no vibe anywhere on the machine
(PROP-027 §2.6 in the consuming repo; the live chain scrubs PATH to
prove it). Transport: the vendored `mcp-core` (line-delimited JSON-RPC
2.0, protocol `2024-11-05` — MCP-CORE-v0.1).

**One engine, one truth.** The tools call the SAME lib fns the CLIs
call, and the `=X.Y.Z` pin on `stack:org.vibevm/rust-ai-native` holds
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
| `init` | `discipline-rust init [--namespace] [--force]` |
| `floor` | `discipline-rust floor [--keep-going] [--fast-loop]` |
| `conform_check` | `conform-rust check [--scope] [--baseline]` |
| `conform_freeze` | `conform-rust freeze [--baseline]` |
| `specmap_check` | `specmap-rust --check` |
| `specmap_write` | `specmap-rust` |
| `trace_explain` | `discipline-rust trace <target> [--json] [--prose]` |
| `test_gate` | `discipline-rust test-gate [--baseline]` |
| `tripwire` | `discipline-rust tripwire [--base] [--debt]` |
| `health` | `discipline-rust health [--out]` |
| `fast_loop` | `discipline-rust fast-loop [--cell] [--budget-secs] [--enforce-budget]` |
| `codemod_add_cell` | `discipline-rust codemod add-cell <crate> <cell> <seam> <variant> <spec-uri>` |
| `ledger_render` | `discipline-rust ledger render [--check]` |
| `tcg_validate` | `tcg-rust validate <file> [--content-from]` |
| `tcg_scope` | `tcg-rust scope <file> [--position]` |
| `tcg_complete` | `tcg-rust complete <file> --position [--prefix] [--max]` |
| `tcg_type` | `tcg-rust type <file> --position` |
| `tcg_bench` | `tcg-rust bench --corpus --report` |

`specmap-rust --gate` (the package-self-trace form) stays CLI-only: its
audience is package gates, not agents.

## The discipline half {#discipline-tools}

Thirteen thin adapters over `discipline_cli_rust` / `conform_cli_rust`
/ `specmap_cli_rust` lib fns. Every schema carries the optional
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
`tcg_cli_rust::enrich_validate` — the gate's own rules over the gate's
own frontend — and the policy (conform.toml + frozen ratchet) reloads
per call, so a mid-session freeze is honoured immediately.
`tcg_validate`'s `isError` mirrors the one-shot exit contract: an
error-grade diagnostic OR a non-baselined finding. The NDJSON serve
relay (`tcg-rust serve`, TCG-PROTOCOL-RUST-v0.1) remains shipped in the
stack as the non-MCP embedding form.
