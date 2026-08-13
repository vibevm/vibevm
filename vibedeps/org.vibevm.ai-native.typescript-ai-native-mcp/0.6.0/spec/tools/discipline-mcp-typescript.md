# typescript-ai-native-mcp — the server brief {#root}

<status stage="spec" state="done"/>

@fact:SERVED-OVER-MCP-AS-ONE-STDIO-BINARY **What it is.** The AI-Native TypeScript discipline served over MCP:
one stdio binary, seventeen tools, launched by an agent host straight
from this package's slot. @status:impl/done

@fact:SERVING-NEEDS-NO-VIBE-ON-THE-MACHINE Serving needs no vibe anywhere on the machine
(PROP-027 §2.6; the live chain scrubs PATH to prove it). @status:impl/done

@fact:TRANSPORT-IS-THE-VENDORED-MCP-CORE Transport: the
vendored `mcp-core` (line-delimited JSON-RPC 2.0, protocol
`2024-11-05` — MCP-CORE-v0.1). @status:impl/done

@fact:THE-TS-ORACLE-IS-THE-COMPILER The TS oracle IS the compiler (the
LanguageService is tsc's own engine) — no approximation caveat rides
these answers. @status:spec/done

@fact:THE-TOOLS-CALL-THE-SAME-LIB-FNS-THE-CLIS-CALL **One engine, one truth.** The tools call the SAME lib fns the CLIs
call, and the `=X.Y.Z` pin on `stack:org.vibevm.ai-native/typescript-ai-native-lang`
holds this package's vendored copies and the consumer's installed stack
to one resolved version set. @status:impl/done

@fact:TOOL-LEVEL-FAILURE-IS-AN-ISERROR-RESULT Tool-level failure — a red gate, an
absent-toolchain refusal WITH ITS RECIPE — is an `isError` RESULT;
protocol errors are reserved for the transport grammar. @status:impl/done

## The parity map {#parity-map}

| Tool | CLI |
|---|---|
| @fact:ROW-INIT `init` @status:impl/done | `typescript-ai-native init [--namespace] [--force]` @status:impl/done |
| @fact:ROW-FLOOR `floor` @status:impl/done | `typescript-ai-native floor [--keep-going]` @status:impl/done |
| @fact:ROW-CONFORM-CHECK `conform_check` @status:impl/done | `typescript-ai-native-conform check [--scope] [--baseline]` @status:impl/done |
| @fact:ROW-CONFORM-FREEZE `conform_freeze` @status:impl/done | `typescript-ai-native-conform freeze [--baseline]` @status:impl/done |
| @fact:ROW-SPECMAP-CHECK `specmap_check` @status:impl/done | `typescript-ai-native-specmap --check` @status:impl/done |
| @fact:ROW-SPECMAP-WRITE `specmap_write` @status:impl/done | `typescript-ai-native-specmap` @status:impl/done |
| @fact:ROW-TRACE-EXPLAIN `trace_explain` @status:impl/done | `typescript-ai-native trace <target> [--json] [--prose]` @status:impl/done |
| @fact:ROW-TEST-GATE `test_gate` @status:impl/done | `typescript-ai-native test-gate [--baseline]` @status:impl/done |
| @fact:ROW-TRIPWIRE `tripwire` @status:impl/done | `typescript-ai-native tripwire [--base] [--debt]` @status:impl/done |
| @fact:ROW-HEALTH `health` @status:impl/done | `typescript-ai-native health [--out]` @status:impl/done |
| @fact:ROW-FAST-LOOP `fast_loop` @status:impl/done | `typescript-ai-native fast-loop [--cell] [--budget-secs] [--enforce-budget]` @status:impl/done |
| @fact:ROW-CODEMOD-ADD-CELL `codemod_add_cell` @status:impl/done | `typescript-ai-native codemod add-cell <cell> <spec-uri>` @status:impl/done |
| @fact:ROW-TCG-VALIDATE `tcg_validate` @status:impl/done | `typescript-ai-native-tcg validate <file> [--content-from]` @status:impl/done |
| @fact:ROW-TCG-SCOPE `tcg_scope` @status:impl/done | `typescript-ai-native-tcg scope <file> [--position]` @status:impl/done |
| @fact:ROW-TCG-COMPLETE `tcg_complete` @status:impl/done | `typescript-ai-native-tcg complete <file> --position [--prefix] [--max]` @status:impl/done |
| @fact:ROW-TCG-TYPE `tcg_type` @status:impl/done | `typescript-ai-native-tcg type <file> --position` @status:impl/done |
| @fact:ROW-TCG-BENCH `tcg_bench` @status:impl/done | `typescript-ai-native-tcg bench --corpus --report` @status:impl/done |

@fact:THE-SPECMAP-GATE-FORM-STAYS-CLI-ONLY `typescript-ai-native-specmap --gate` (the package-self-trace form) stays
CLI-only, as on the rust side. @status:impl/done

@fact:THE-TS-UMBRELLA-HAS-NO-LEDGER-COMMAND The TS umbrella has no ledger command —
seventeen tools, not eighteen. @status:impl/done

## The discipline half {#discipline-tools}

@fact:TWELVE-THIN-ADAPTERS-OVER-THE-LIB-FNS Twelve thin adapters over `typescript_ai_native_cli` /
`typescript_ai_native_conform` / `typescript_ai_native_specmap` lib fns, each
wrapped in the stderr-capture guard — an agent's report carries the
run's whole story (node, tsc, prettier, eslint children included). @status:impl/done

@fact:EVERY-SCHEMA-CARRIES-THE-OPTIONAL-LANGUAGE-PROPERTY Every schema carries the optional `language` property; a
non-`typescript` value refuses with the recipe naming that language's
own server (PROP-026 §2 continuity). @status:impl/done

@fact:HEAVY-TOOLS-SAY-EXPECT-MINUTES-AND-NOTHING-PROMPTS Heavy tools say «expect minutes»;
nothing prompts. @status:impl/done

## The tcg half {#tcg-tools}

@fact:ONE-PERSISTENT-ORACLE-SESSION-SHARED-BY-FIVE-TOOLS The four oracle ops + the bench harness over ONE persistent
LanguageService session shared by all five tools: lazy-spawned and
`init`-ed on first use (the policy's topology — cells dir, seam —
rides the init), respawned ONCE per op on a crashed session. @status:impl/done

@fact:ENRICHMENT-GOES-THROUGH-ENRICH-VALIDATE Enrichment
goes through `typescript_ai_native_tcg::enrich_validate` — the gate's own
rules — with the policy reloaded per call, so a mid-session freeze is
honoured immediately. @status:impl/done

@fact:TCG-VALIDATE-ISERROR-MIRRORS-THE-ONE-SHOT-EXIT-CONTRACT `tcg_validate`'s `isError` mirrors the one-shot
exit contract: an error diagnostic OR a non-baselined finding. @status:impl/done

@fact:THE-NDJSON-SERVE-RELAY-REMAINS-SHIPPED The
NDJSON serve relay (`typescript-ai-native-tcg serve`, TCG-PROTOCOL-v0.1) remains
shipped in the stack as the non-MCP embedding form. @status:impl/done
