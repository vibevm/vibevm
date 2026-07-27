# go-ai-native-mcp — the server brief {#root}

<status stage="spec" state="done"/>

##SERVED-OVER-MCP-AS-ONE-STDIO-BINARY **What it is.** The AI-Native Go discipline served over MCP: one stdio
binary, seventeen tools, launched by an agent host straight from this
package's slot. @impl/done

##SERVING-NEEDS-NO-VIBE-ON-THE-MACHINE Serving needs no vibe anywhere on the machine
(PROP-027 §2.6). @impl/done

##TRANSPORT-IS-THE-VENDORED-MCP-CORE Transport: the vendored `mcp-core` (line-delimited
JSON-RPC 2.0, protocol `2024-11-05` — MCP-CORE-v0.1). @impl/done

##THE-GO-ORACLE-STANDS-ON-GO-TYPES The Go oracle
stands on go/types — the reference implementation of the language
spec — one honest step short of the TS oracle (which IS tsc) and far
tighter than rust-analyzer↔rustc; the floor stays the truth
(TCG-ORACLE-GO §5). @spec/done

##THE-TOOLS-CALL-THE-SAME-LIB-FNS-THE-CLIS-CALL **One engine, one truth.** The tools call the SAME lib fns the CLIs
call, and the `=0.1.0` pin on `stack:org.vibevm.ai-native/go-ai-native-lang`
holds this package's vendored copies and the consumer's installed
stack to one resolved version set. @impl/done

##TOOL-LEVEL-FAILURE-IS-AN-ISERROR-RESULT Tool-level failure — a red gate, an
absent-toolchain refusal WITH ITS RECIPE — is an `isError` RESULT;
protocol errors are reserved for the transport grammar. @impl/done

##REPORTS-CARRY-THE-RUNS-ENTIRE-STORY **Reports are whole.** Every discipline tool runs inside the
`mcp-core::capture` guard, so the agent's report carries the run's
entire story — the runner's own words AND its child processes (go,
gofmt, staticcheck, exhaustive). @impl/done

## The parity map {#parity-map}

##parity-map-lead Tool ↔ the CLI invocation it is parity-locked to (the enumeration test
pins the list; the descriptions in `tools/list` restate each row): @impl/done

| Tool | CLI |
|---|---|
| ##ROW-INIT `init` @impl/done | `go-ai-native init [--namespace] [--force]` @impl/done |
| ##ROW-FLOOR `floor` @impl/done | `go-ai-native floor [--keep-going]` @impl/done |
| ##ROW-CONFORM-CHECK `conform_check` @impl/done | `go-ai-native-conform check [--scope] [--baseline]` @impl/done |
| ##ROW-CONFORM-FREEZE `conform_freeze` @impl/done | `go-ai-native-conform freeze [--baseline]` @impl/done |
| ##ROW-SPECMAP-CHECK `specmap_check` @impl/done | `go-ai-native-specmap --check` @impl/done |
| ##ROW-SPECMAP-WRITE `specmap_write` @impl/done | `go-ai-native-specmap` @impl/done |
| ##ROW-TRACE-EXPLAIN `trace_explain` @impl/done | `go-ai-native trace <target> [--json] [--prose]` @impl/done |
| ##ROW-TEST-GATE `test_gate` @impl/done | `go-ai-native test-gate [--baseline]` @impl/done |
| ##ROW-TRIPWIRE `tripwire` @impl/done | `go-ai-native tripwire [--base] [--debt]` @impl/done |
| ##ROW-HEALTH `health` @impl/done | `go-ai-native health [--out]` @impl/done |
| ##ROW-FAST-LOOP `fast_loop` @impl/done | `go-ai-native fast-loop [--cell] [--budget-secs] [--enforce-budget]` @impl/done |
| ##ROW-CODEMOD-ADD-CELL `codemod_add_cell` @impl/done | `go-ai-native codemod add-cell <cell> <spec-uri>` @impl/done |
| ##ROW-TCG-VALIDATE `tcg_validate` @impl/done | `go-ai-native-tcg validate <file> [--content-from]` @impl/done |
| ##ROW-TCG-SCOPE `tcg_scope` @impl/done | `go-ai-native-tcg scope <file> [--position]` @impl/done |
| ##ROW-TCG-COMPLETE `tcg_complete` @impl/done | `go-ai-native-tcg complete <file> --position [--prefix] [--max]` @impl/done |
| ##ROW-TCG-TYPE `tcg_type` @impl/done | `go-ai-native-tcg type <file> --position` @impl/done |
| ##ROW-TCG-BENCH `tcg_bench` @impl/done | `go-ai-native-tcg bench --corpus --report` @impl/done |

##THE-SPECMAP-GATE-FORM-STAYS-CLI-ONLY `go-ai-native-specmap --gate` (the package-self-trace form) stays
CLI-only, as on the sibling servers. @impl/done

##THE-GO-UMBRELLA-HAS-NO-LEDGER-COMMAND The Go umbrella has no ledger
command — seventeen tools, the TS count. @impl/done

## The discipline half {#discipline-tools}

##TWELVE-THIN-ADAPTERS-OVER-THE-LIB-FNS Twelve thin adapters over `go_ai_native_cli` / `go_ai_native_conform` /
`go_ai_native_specmap` lib fns, each wrapped in the stderr-capture
guard — an agent's report carries the run's whole story (go, gofmt,
staticcheck, exhaustive children included). @impl/done

##EVERY-SCHEMA-CARRIES-THE-OPTIONAL-LANGUAGE-PROPERTY Every schema carries the
optional `language` property; a non-`go` value refuses with the recipe
naming that language's own server (PROP-026 §2 continuity). @impl/done

##HEAVY-TOOLS-SAY-EXPECT-MINUTES-AND-NOTHING-PROMPTS Heavy
tools say «expect minutes»; nothing prompts. @impl/done

## The tcg half {#tcg-tools}

##ONE-PERSISTENT-ORACLE-SESSION-SHARED-BY-FIVE-TOOLS The four oracle ops + the bench harness over ONE persistent gopls
session shared by all five tools: lazy-spawned on first use, respawned
ONCE per op on a crashed session (the serve relay's posture,
server-local). @impl/done

##ENRICHMENT-GOES-THROUGH-ENRICH-VALIDATE Enrichment goes through
`go_ai_native_tcg::enrich_validate` — the gate's own rules over the
gate's own extractor (the `--stdin-file` overlay form) — with the
policy reloaded per call, so a mid-session freeze is honoured
immediately. @impl/done

##TCG-VALIDATE-ISERROR-MIRRORS-THE-ONE-SHOT-EXIT-CONTRACT `tcg_validate`'s `isError` mirrors the one-shot exit
contract: an error diagnostic OR a non-baselined finding; the FILLED
`markers` stream rides every validate (the Go relay's named delta
over the Rust one). @impl/done

##THE-NDJSON-SERVE-RELAY-REMAINS-SHIPPED The NDJSON serve relay (`go-ai-native-tcg serve`,
TCG-PROTOCOL-GO-v0.1) remains shipped in the stack as the non-MCP
embedding form. @impl/done
