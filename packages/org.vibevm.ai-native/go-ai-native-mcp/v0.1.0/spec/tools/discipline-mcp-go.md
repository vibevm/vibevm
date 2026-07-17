# go-ai-native-mcp — the server brief {#root}

**What it is.** The AI-Native Go discipline served over MCP: one stdio
binary, seventeen tools, launched by an agent host straight from this
package's slot. Serving needs no vibe anywhere on the machine
(PROP-027 §2.6). Transport: the vendored `mcp-core` (line-delimited
JSON-RPC 2.0, protocol `2024-11-05` — MCP-CORE-v0.1). The Go oracle
stands on go/types — the reference implementation of the language
spec — one honest step short of the TS oracle (which IS tsc) and far
tighter than rust-analyzer↔rustc; the floor stays the truth
(TCG-ORACLE-GO §5).

**One engine, one truth.** The tools call the SAME lib fns the CLIs
call, and the `=0.1.0` pin on `stack:org.vibevm.ai-native/go-ai-native-lang`
holds this package's vendored copies and the consumer's installed
stack to one resolved version set. Tool-level failure — a red gate, an
absent-toolchain refusal WITH ITS RECIPE — is an `isError` RESULT;
protocol errors are reserved for the transport grammar.

**Reports are whole.** Every discipline tool runs inside the
`mcp-core::capture` guard, so the agent's report carries the run's
entire story — the runner's own words AND its child processes (go,
gofmt, staticcheck, exhaustive).

## The parity map {#parity-map}

Tool ↔ the CLI invocation it is parity-locked to (the enumeration test
pins the list; the descriptions in `tools/list` restate each row):

| Tool | CLI |
|---|---|
| `init` | `go-ai-native init [--namespace] [--force]` |
| `floor` | `go-ai-native floor [--keep-going]` |
| `conform_check` | `go-ai-native-conform check [--scope] [--baseline]` |
| `conform_freeze` | `go-ai-native-conform freeze [--baseline]` |
| `specmap_check` | `go-ai-native-specmap --check` |
| `specmap_write` | `go-ai-native-specmap` |
| `trace_explain` | `go-ai-native trace <target> [--json] [--prose]` |
| `test_gate` | `go-ai-native test-gate [--baseline]` |
| `tripwire` | `go-ai-native tripwire [--base] [--debt]` |
| `health` | `go-ai-native health [--out]` |
| `fast_loop` | `go-ai-native fast-loop [--cell] [--budget-secs] [--enforce-budget]` |
| `codemod_add_cell` | `go-ai-native codemod add-cell <cell> <spec-uri>` |
| `tcg_validate` | `go-ai-native-tcg validate <file> [--content-from]` |
| `tcg_scope` | `go-ai-native-tcg scope <file> [--position]` |
| `tcg_complete` | `go-ai-native-tcg complete <file> --position [--prefix] [--max]` |
| `tcg_type` | `go-ai-native-tcg type <file> --position` |
| `tcg_bench` | `go-ai-native-tcg bench --corpus --report` |

`go-ai-native-specmap --gate` (the package-self-trace form) stays
CLI-only, as on the sibling servers. The Go umbrella has no ledger
command — seventeen tools, the TS count.

## The discipline half {#discipline-tools}

Twelve thin adapters over `go_ai_native_cli` / `go_ai_native_conform` /
`go_ai_native_specmap` lib fns, each wrapped in the stderr-capture
guard — an agent's report carries the run's whole story (go, gofmt,
staticcheck, exhaustive children included). Every schema carries the
optional `language` property; a non-`go` value refuses with the recipe
naming that language's own server (PROP-026 §2 continuity). Heavy
tools say «expect minutes»; nothing prompts.

## The tcg half {#tcg-tools}

The four oracle ops + the bench harness over ONE persistent gopls
session shared by all five tools: lazy-spawned on first use, respawned
ONCE per op on a crashed session (the serve relay's posture,
server-local). Enrichment goes through
`go_ai_native_tcg::enrich_validate` — the gate's own rules over the
gate's own extractor (the `--stdin-file` overlay form) — with the
policy reloaded per call, so a mid-session freeze is honoured
immediately. `tcg_validate`'s `isError` mirrors the one-shot exit
contract: an error diagnostic OR a non-baselined finding; the FILLED
`markers` stream rides every validate (the Go relay's named delta
over the Rust one). The NDJSON serve relay (`go-ai-native-tcg serve`,
TCG-PROTOCOL-GO-v0.1) remains shipped in the stack as the non-MCP
embedding form.
