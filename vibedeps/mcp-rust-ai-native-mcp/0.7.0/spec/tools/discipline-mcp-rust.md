# rust-ai-native-mcp — the server brief {#root}

<status stage="spec" state="done"/>

##SERVED-OVER-MCP-AS-ONE-STDIO-BINARY **What it is.** The AI-Native Rust discipline served over MCP: one
stdio binary, eighteen tools, launched by an agent host straight from
this package's slot. @impl/done

##SERVING-NEEDS-NO-VIBE-ON-THE-MACHINE Serving needs no vibe anywhere on the machine
(PROP-027 §2.6 in the consuming repo; the live chain scrubs PATH to
prove it). @impl/done

##TRANSPORT-IS-THE-VENDORED-MCP-CORE Transport: the vendored `mcp-core` (line-delimited JSON-RPC
2.0, protocol `2024-11-05` — MCP-CORE-v0.1). @impl/done

##THE-TOOLS-CALL-THE-SAME-LIB-FNS-THE-CLIS-CALL **One engine, one truth.** The tools call the SAME lib fns the CLIs
call, and the `=X.Y.Z` pin on `stack:org.vibevm.ai-native/rust-ai-native-lang` holds
this package's vendored copies and the consumer's installed stack to
one resolved version set. @impl/done

##TOOL-LEVEL-FAILURE-IS-AN-ISERROR-RESULT Tool-level failure (a red gate, a refusing
oracle) is an `isError` RESULT carrying the report; protocol errors are
reserved for the transport grammar. @impl/done

##REPORTS-CARRY-THE-RUNS-ENTIRE-STORY **Reports are whole.** Every discipline tool runs inside the
`mcp-core::capture` guard, so the agent's report carries the run's
entire story — the runner's own words AND its child processes (cargo,
rustfmt, clippy, nextest). @impl/done

## The parity map {#parity-map}

##parity-map-lead Tool ↔ the CLI invocation it is parity-locked to (the enumeration test
pins the list; the descriptions in `tools/list` restate each row): @impl/done

| Tool | CLI |
|---|---|
| ##ROW-INIT `init` @impl/done | `rust-ai-native init [--namespace] [--force]` @impl/done |
| ##ROW-FLOOR `floor` @impl/done | `rust-ai-native floor [--keep-going] [--fast-loop]` @impl/done |
| ##ROW-CONFORM-CHECK `conform_check` @impl/done | `rust-ai-native-conform check [--scope] [--baseline]` @impl/done |
| ##ROW-CONFORM-FREEZE `conform_freeze` @impl/done | `rust-ai-native-conform freeze [--baseline]` @impl/done |
| ##ROW-SPECMAP-CHECK `specmap_check` @impl/done | `rust-ai-native-specmap --check` @impl/done |
| ##ROW-SPECMAP-WRITE `specmap_write` @impl/done | `rust-ai-native-specmap` @impl/done |
| ##ROW-TRACE-EXPLAIN `trace_explain` @impl/done | `rust-ai-native trace <target> [--json] [--prose]` @impl/done |
| ##ROW-TEST-GATE `test_gate` @impl/done | `rust-ai-native test-gate [--baseline]` @impl/done |
| ##ROW-TRIPWIRE `tripwire` @impl/done | `rust-ai-native tripwire [--base] [--debt]` @impl/done |
| ##ROW-HEALTH `health` @impl/done | `rust-ai-native health [--out]` @impl/done |
| ##ROW-FAST-LOOP `fast_loop` @impl/done | `rust-ai-native fast-loop [--cell] [--budget-secs] [--enforce-budget]` @impl/done |
| ##ROW-CODEMOD-ADD-CELL `codemod_add_cell` @impl/done | `rust-ai-native codemod add-cell <crate> <cell> <seam> <variant> <spec-uri>` @impl/done |
| ##ROW-LEDGER-RENDER `ledger_render` @impl/done | `rust-ai-native ledger render [--check]` @impl/done |
| ##ROW-TCG-VALIDATE `tcg_validate` @impl/done | `rust-ai-native-tcg validate <file> [--content-from]` @impl/done |
| ##ROW-TCG-SCOPE `tcg_scope` @impl/done | `rust-ai-native-tcg scope <file> [--position]` @impl/done |
| ##ROW-TCG-COMPLETE `tcg_complete` @impl/done | `rust-ai-native-tcg complete <file> --position [--prefix] [--max]` @impl/done |
| ##ROW-TCG-TYPE `tcg_type` @impl/done | `rust-ai-native-tcg type <file> --position` @impl/done |
| ##ROW-TCG-BENCH `tcg_bench` @impl/done | `rust-ai-native-tcg bench --corpus --report` @impl/done |

##THE-SPECMAP-GATE-FORM-STAYS-CLI-ONLY `rust-ai-native-specmap --gate` (the package-self-trace form) stays CLI-only: its
audience is package gates, not agents. @impl/done

## The discipline half {#discipline-tools}

##THIRTEEN-THIN-ADAPTERS-OVER-THE-LIB-FNS Thirteen thin adapters over `rust_ai_native_cli` / `rust_ai_native_conform`
/ `rust_ai_native_specmap` lib fns. @impl/done

##EVERY-SCHEMA-CARRIES-THE-OPTIONAL-LANGUAGE-PROPERTY Every schema carries the optional
`language` property; a non-`rust` value refuses with the recipe naming
that language's own server — never another language's fix surface
(PROP-026 §2 continuity). @impl/done

##HEAVY-TOOLS-SAY-EXPECT-MINUTES-AND-NOTHING-PROMPTS Heavy tools (`floor`, `test_gate`,
`fast_loop`, `tcg_bench`) say «expect minutes» in their descriptions;
nothing prompts — `force`-class decisions are explicit parameters. @impl/done

## The tcg half {#tcg-tools}

##ONE-PERSISTENT-ORACLE-SESSION-SHARED-BY-FIVE-TOOLS The four oracle ops + the bench harness over ONE persistent
rust-analyzer session shared by all five tools: lazy-spawned on first
use, respawned ONCE per op on a crashed session (the serve relay's
posture, server-local now). @impl/done

##ENRICHMENT-GOES-THROUGH-ENRICH-VALIDATE Enrichment goes through
`rust_ai_native_tcg::enrich_validate` — the gate's own rules over the gate's
own frontend — and the policy (conform.toml + frozen ratchet) reloads
per call, so a mid-session freeze is honoured immediately. @impl/done

##TCG-VALIDATE-ISERROR-MIRRORS-THE-ONE-SHOT-EXIT-CONTRACT `tcg_validate`'s `isError` mirrors the one-shot exit contract: an
error-grade diagnostic OR a non-baselined finding. @impl/done

##THE-NDJSON-SERVE-RELAY-REMAINS-SHIPPED The NDJSON serve
relay (`rust-ai-native-tcg serve`, TCG-PROTOCOL-RUST-v0.1) remains shipped in the
stack as the non-MCP embedding form. @impl/done
