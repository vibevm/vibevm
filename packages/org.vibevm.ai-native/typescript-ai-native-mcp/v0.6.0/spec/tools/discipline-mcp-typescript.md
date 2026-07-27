# typescript-ai-native-mcp — the server brief {#root}

<status stage="spec" state="done"/>

##SERVED-OVER-MCP-AS-ONE-STDIO-BINARY **What it is.** The AI-Native TypeScript discipline served over MCP:
one stdio binary, seventeen tools, launched by an agent host straight
from this package's slot. @impl/done

##SERVING-NEEDS-NO-VIBE-ON-THE-MACHINE Serving needs no vibe anywhere on the machine
(PROP-027 §2.6; the live chain scrubs PATH to prove it). @impl/done

##TRANSPORT-IS-THE-VENDORED-MCP-CORE Transport: the
vendored `mcp-core` (line-delimited JSON-RPC 2.0, protocol
`2024-11-05` — MCP-CORE-v0.1). @impl/done

##THE-TS-ORACLE-IS-THE-COMPILER The TS oracle IS the compiler (the
LanguageService is tsc's own engine) — no approximation caveat rides
these answers. @spec/done

##THE-TOOLS-CALL-THE-SAME-LIB-FNS-THE-CLIS-CALL **One engine, one truth.** The tools call the SAME lib fns the CLIs
call, and the `=X.Y.Z` pin on `stack:org.vibevm.ai-native/typescript-ai-native-lang`
holds this package's vendored copies and the consumer's installed stack
to one resolved version set. @impl/done

##TOOL-LEVEL-FAILURE-IS-AN-ISERROR-RESULT Tool-level failure — a red gate, an
absent-toolchain refusal WITH ITS RECIPE — is an `isError` RESULT;
protocol errors are reserved for the transport grammar. @impl/done

## The parity map {#parity-map}

| Tool | CLI |
|---|---|
| ##ROW-INIT `init` @impl/done | `typescript-ai-native init [--namespace] [--force]` @impl/done |
| ##ROW-FLOOR `floor` @impl/done | `typescript-ai-native floor [--keep-going]` @impl/done |
| ##ROW-CONFORM-CHECK `conform_check` @impl/done | `typescript-ai-native-conform check [--scope] [--baseline]` @impl/done |
| ##ROW-CONFORM-FREEZE `conform_freeze` @impl/done | `typescript-ai-native-conform freeze [--baseline]` @impl/done |
| ##ROW-SPECMAP-CHECK `specmap_check` @impl/done | `typescript-ai-native-specmap --check` @impl/done |
| ##ROW-SPECMAP-WRITE `specmap_write` @impl/done | `typescript-ai-native-specmap` @impl/done |
| ##ROW-TRACE-EXPLAIN `trace_explain` @impl/done | `typescript-ai-native trace <target> [--json] [--prose]` @impl/done |
| ##ROW-TEST-GATE `test_gate` @impl/done | `typescript-ai-native test-gate [--baseline]` @impl/done |
| ##ROW-TRIPWIRE `tripwire` @impl/done | `typescript-ai-native tripwire [--base] [--debt]` @impl/done |
| ##ROW-HEALTH `health` @impl/done | `typescript-ai-native health [--out]` @impl/done |
| ##ROW-FAST-LOOP `fast_loop` @impl/done | `typescript-ai-native fast-loop [--cell] [--budget-secs] [--enforce-budget]` @impl/done |
| ##ROW-CODEMOD-ADD-CELL `codemod_add_cell` @impl/done | `typescript-ai-native codemod add-cell <cell> <spec-uri>` @impl/done |
| ##ROW-TCG-VALIDATE `tcg_validate` @impl/done | `typescript-ai-native-tcg validate <file> [--content-from]` @impl/done |
| ##ROW-TCG-SCOPE `tcg_scope` @impl/done | `typescript-ai-native-tcg scope <file> [--position]` @impl/done |
| ##ROW-TCG-COMPLETE `tcg_complete` @impl/done | `typescript-ai-native-tcg complete <file> --position [--prefix] [--max]` @impl/done |
| ##ROW-TCG-TYPE `tcg_type` @impl/done | `typescript-ai-native-tcg type <file> --position` @impl/done |
| ##ROW-TCG-BENCH `tcg_bench` @impl/done | `typescript-ai-native-tcg bench --corpus --report` @impl/done |

##THE-SPECMAP-GATE-FORM-STAYS-CLI-ONLY `typescript-ai-native-specmap --gate` (the package-self-trace form) stays
CLI-only, as on the rust side. @impl/done

##THE-TS-UMBRELLA-HAS-NO-LEDGER-COMMAND The TS umbrella has no ledger command —
seventeen tools, not eighteen. @impl/done

## The discipline half {#discipline-tools}

##TWELVE-THIN-ADAPTERS-OVER-THE-LIB-FNS Twelve thin adapters over `typescript_ai_native_cli` /
`typescript_ai_native_conform` / `typescript_ai_native_specmap` lib fns, each
wrapped in the stderr-capture guard — an agent's report carries the
run's whole story (node, tsc, prettier, eslint children included). @impl/done

##EVERY-SCHEMA-CARRIES-THE-OPTIONAL-LANGUAGE-PROPERTY Every schema carries the optional `language` property; a
non-`typescript` value refuses with the recipe naming that language's
own server (PROP-026 §2 continuity). @impl/done

##HEAVY-TOOLS-SAY-EXPECT-MINUTES-AND-NOTHING-PROMPTS Heavy tools say «expect minutes»;
nothing prompts. @impl/done

## The tcg half {#tcg-tools}

##ONE-PERSISTENT-ORACLE-SESSION-SHARED-BY-FIVE-TOOLS The four oracle ops + the bench harness over ONE persistent
LanguageService session shared by all five tools: lazy-spawned and
`init`-ed on first use (the policy's topology — cells dir, seam —
rides the init), respawned ONCE per op on a crashed session. @impl/done

##ENRICHMENT-GOES-THROUGH-ENRICH-VALIDATE Enrichment
goes through `typescript_ai_native_tcg::enrich_validate` — the gate's own
rules — with the policy reloaded per call, so a mid-session freeze is
honoured immediately. @impl/done

##TCG-VALIDATE-ISERROR-MIRRORS-THE-ONE-SHOT-EXIT-CONTRACT `tcg_validate`'s `isError` mirrors the one-shot
exit contract: an error diagnostic OR a non-baselined finding. @impl/done

##THE-NDJSON-SERVE-RELAY-REMAINS-SHIPPED The
NDJSON serve relay (`typescript-ai-native-tcg serve`, TCG-PROTOCOL-v0.1) remains
shipped in the stack as the non-MCP embedding form. @impl/done
