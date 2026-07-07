# typescript-ai-native-mcp — the server brief {#root}

**What it is.** The AI-Native TypeScript discipline served over MCP:
one stdio binary, seventeen tools, launched by an agent host straight
from this package's slot. Serving needs no vibe anywhere on the machine
(PROP-027 §2.6; the live chain scrubs PATH to prove it). Transport: the
vendored `mcp-core` (line-delimited JSON-RPC 2.0, protocol
`2024-11-05` — MCP-CORE-v0.1). The TS oracle IS the compiler (the
LanguageService is tsc's own engine) — no approximation caveat rides
these answers.

**One engine, one truth.** The tools call the SAME lib fns the CLIs
call, and the `=X.Y.Z` pin on `stack:org.vibevm/typescript-ai-native-lang`
holds this package's vendored copies and the consumer's installed stack
to one resolved version set. Tool-level failure — a red gate, an
absent-toolchain refusal WITH ITS RECIPE — is an `isError` RESULT;
protocol errors are reserved for the transport grammar.

## The parity map {#parity-map}

| Tool | CLI |
|---|---|
| `init` | `typescript-ai-native init [--namespace] [--force]` |
| `floor` | `typescript-ai-native floor [--keep-going]` |
| `conform_check` | `typescript-ai-native-conform check [--scope] [--baseline]` |
| `conform_freeze` | `typescript-ai-native-conform freeze [--baseline]` |
| `specmap_check` | `typescript-ai-native-specmap --check` |
| `specmap_write` | `typescript-ai-native-specmap` |
| `trace_explain` | `typescript-ai-native trace <target> [--json] [--prose]` |
| `test_gate` | `typescript-ai-native test-gate [--baseline]` |
| `tripwire` | `typescript-ai-native tripwire [--base] [--debt]` |
| `health` | `typescript-ai-native health [--out]` |
| `fast_loop` | `typescript-ai-native fast-loop [--cell] [--budget-secs] [--enforce-budget]` |
| `codemod_add_cell` | `typescript-ai-native codemod add-cell <cell> <spec-uri>` |
| `tcg_validate` | `typescript-ai-native-tcg validate <file> [--content-from]` |
| `tcg_scope` | `typescript-ai-native-tcg scope <file> [--position]` |
| `tcg_complete` | `typescript-ai-native-tcg complete <file> --position [--prefix] [--max]` |
| `tcg_type` | `typescript-ai-native-tcg type <file> --position` |
| `tcg_bench` | `typescript-ai-native-tcg bench --corpus --report` |

`typescript-ai-native-specmap --gate` (the package-self-trace form) stays
CLI-only, as on the rust side. The TS umbrella has no ledger command —
seventeen tools, not eighteen.

## The discipline half {#discipline-tools}

Twelve thin adapters over `typescript_ai_native_cli` /
`typescript_ai_native_conform` / `typescript_ai_native_specmap` lib fns, each
wrapped in the stderr-capture guard — an agent's report carries the
run's whole story (node, tsc, prettier, eslint children included).
Every schema carries the optional `language` property; a
non-`typescript` value refuses with the recipe naming that language's
own server (PROP-026 §2 continuity). Heavy tools say «expect minutes»;
nothing prompts.

## The tcg half {#tcg-tools}

The four oracle ops + the bench harness over ONE persistent
LanguageService session shared by all five tools: lazy-spawned and
`init`-ed on first use (the policy's topology — cells dir, seam —
rides the init), respawned ONCE per op on a crashed session. Enrichment
goes through `typescript_ai_native_tcg::enrich_validate` — the gate's own
rules — with the policy reloaded per call, so a mid-session freeze is
honoured immediately. `tcg_validate`'s `isError` mirrors the one-shot
exit contract: an error diagnostic OR a non-baselined finding. The
NDJSON serve relay (`typescript-ai-native-tcg serve`, TCG-PROTOCOL-v0.1) remains
shipped in the stack as the non-MCP embedding form.
