# go-ai-native-mcp — the server brief {#root}

<status stage="spec" state="done"/>

@fact:SERVED-OVER-MCP-AS-ONE-STDIO-BINARY **What it is.** The AI-Native Go discipline served over MCP: one stdio
binary, seventeen tools, launched by an agent host straight from this
package's slot. @status:impl/done

@fact:SERVING-NEEDS-NO-VIBE-ON-THE-MACHINE Serving needs no vibe anywhere on the machine
(PROP-027 §2.6). @status:impl/done

@fact:TRANSPORT-IS-THE-VENDORED-MCP-CORE Transport: the vendored `mcp-core` (line-delimited
JSON-RPC 2.0, protocol `2024-11-05` — MCP-CORE-v0.1). @status:impl/done

@fact:THE-GO-ORACLE-STANDS-ON-GO-TYPES The Go oracle
stands on go/types — the reference implementation of the language
spec — one honest step short of the TS oracle (which IS tsc) and far
tighter than rust-analyzer↔rustc; the floor stays the truth
(TCG-ORACLE-GO §5). @status:spec/done

@fact:THE-TOOLS-CALL-THE-SAME-LIB-FNS-THE-CLIS-CALL **One engine, one truth.** The tools call the SAME lib fns the CLIs
call, and the `=1.0.0` pin on `stack:org.vibevm.ai-native/go-ai-native-lang`
holds this package's vendored copies and the consumer's installed
stack to one resolved version set. @status:impl/done

@fact:TOOL-LEVEL-FAILURE-IS-AN-ISERROR-RESULT Tool-level failure — a red gate, an
absent-toolchain refusal WITH ITS RECIPE — is an `isError` RESULT;
protocol errors are reserved for the transport grammar. @status:impl/done

@fact:REPORTS-CARRY-THE-RUNS-ENTIRE-STORY **Reports are whole.** Every discipline tool runs inside the
`mcp-core::capture` guard, so the agent's report carries the run's
entire story — the runner's own words AND its child processes (go,
gofmt, staticcheck, exhaustive). @status:impl/done

## The parity map {#parity-map}

@fact:parity-map-lead Tool ↔ the CLI invocation it is parity-locked to (the enumeration test
pins the list; the descriptions in `tools/list` restate each row): @status:impl/done

| Tool | CLI |
|---|---|
| @fact:ROW-INIT `init` @status:impl/done | `go-ai-native init [--namespace] [--force]` @status:impl/done |
| @fact:ROW-FLOOR `floor` @status:impl/done | `go-ai-native floor [--keep-going]` @status:impl/done |
| @fact:ROW-CONFORM-CHECK `conform_check` @status:impl/done | `go-ai-native-conform check [--scope] [--baseline]` @status:impl/done |
| @fact:ROW-CONFORM-FREEZE `conform_freeze` @status:impl/done | `go-ai-native-conform freeze [--baseline]` @status:impl/done |
| @fact:ROW-SPECMAP-CHECK `specmap_check` @status:impl/done | `go-ai-native-specmap --check` @status:impl/done |
| @fact:ROW-SPECMAP-WRITE `specmap_write` @status:impl/done | `go-ai-native-specmap` @status:impl/done |
| @fact:ROW-TRACE-EXPLAIN `trace_explain` @status:impl/done | `go-ai-native trace <target> [--json] [--prose]` @status:impl/done |
| @fact:ROW-TEST-GATE `test_gate` @status:impl/done | `go-ai-native test-gate [--baseline]` @status:impl/done |
| @fact:ROW-TRIPWIRE `tripwire` @status:impl/done | `go-ai-native tripwire [--base] [--debt]` @status:impl/done |
| @fact:ROW-HEALTH `health` @status:impl/done | `go-ai-native health [--out]` @status:impl/done |
| @fact:ROW-FAST-LOOP `fast_loop` @status:impl/done | `go-ai-native fast-loop [--cell] [--budget-secs] [--enforce-budget]` @status:impl/done |
| @fact:ROW-CODEMOD-ADD-CELL `codemod_add_cell` @status:impl/done | `go-ai-native codemod add-cell <cell> <spec-uri>` @status:impl/done |
| @fact:ROW-TCG-VALIDATE `tcg_validate` @status:impl/done | `go-ai-native-tcg validate <file> [--content-from]` @status:impl/done |
| @fact:ROW-TCG-SCOPE `tcg_scope` @status:impl/done | `go-ai-native-tcg scope <file> [--position]` @status:impl/done |
| @fact:ROW-TCG-COMPLETE `tcg_complete` @status:impl/done | `go-ai-native-tcg complete <file> --position [--prefix] [--max]` @status:impl/done |
| @fact:ROW-TCG-TYPE `tcg_type` @status:impl/done | `go-ai-native-tcg type <file> --position` @status:impl/done |
| @fact:ROW-TCG-BENCH `tcg_bench` @status:impl/done | `go-ai-native-tcg bench --corpus --report` @status:impl/done |

@fact:THE-SPECMAP-GATE-FORM-STAYS-CLI-ONLY `go-ai-native-specmap --gate` (the package-self-trace form) stays
CLI-only, as on the sibling servers. @status:impl/done

@fact:THE-GO-UMBRELLA-HAS-NO-LEDGER-COMMAND The Go umbrella has no ledger
command — seventeen tools, the TS count. @status:impl/done

## The discipline half {#discipline-tools}

@fact:TWELVE-THIN-ADAPTERS-OVER-THE-LIB-FNS Twelve thin adapters over `go_ai_native_cli` / `go_ai_native_conform` /
`go_ai_native_specmap` lib fns, each wrapped in the stderr-capture
guard — an agent's report carries the run's whole story (go, gofmt,
staticcheck, exhaustive children included). @status:impl/done

@fact:EVERY-SCHEMA-CARRIES-THE-OPTIONAL-LANGUAGE-PROPERTY Every schema carries the
optional `language` property; a non-`go` value refuses with the recipe
naming that language's own server (PROP-026 §2 continuity). @status:impl/done

@fact:HEAVY-TOOLS-SAY-EXPECT-MINUTES-AND-NOTHING-PROMPTS Heavy
tools say «expect minutes»; nothing prompts. @status:impl/done

## The tcg half {#tcg-tools}

@fact:ONE-PERSISTENT-ORACLE-SESSION-SHARED-BY-FIVE-TOOLS The four oracle ops + the bench harness over ONE persistent gopls
session shared by all five tools: lazy-spawned on first use, respawned
ONCE per op on a crashed session (the serve relay's posture,
server-local). @status:impl/done

@fact:ENRICHMENT-GOES-THROUGH-ENRICH-VALIDATE Enrichment goes through
`go_ai_native_tcg::enrich_validate` — the gate's own rules over the
gate's own extractor (the `--stdin-file` overlay form) — with the
policy reloaded per call, so a mid-session freeze is honoured
immediately. @status:impl/done

@fact:TCG-VALIDATE-ISERROR-MIRRORS-THE-ONE-SHOT-EXIT-CONTRACT `tcg_validate`'s `isError` mirrors the one-shot exit
contract: an error diagnostic OR a non-baselined finding; the FILLED
`markers` stream rides every validate (the Go relay's named delta
over the Rust one). @status:impl/done

@fact:THE-NDJSON-SERVE-RELAY-REMAINS-SHIPPED The NDJSON serve relay (`go-ai-native-tcg serve`,
TCG-PROTOCOL-GO-v0.1) remains shipped in the stack as the non-MCP
embedding form. @status:impl/done
