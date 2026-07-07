# discipline-mcp-typescript — the server brief {#root}

**What it is.** The AI-Native TypeScript discipline served over MCP:
one stdio binary, seventeen tools, launched by an agent host straight
from this package's slot. Serving needs no vibe anywhere on the machine
(PROP-027 §2.6; the live chain scrubs PATH to prove it). Transport: the
vendored `mcp-core` (line-delimited JSON-RPC 2.0, protocol
`2024-11-05` — MCP-CORE-v0.1). The TS oracle IS the compiler (the
LanguageService is tsc's own engine) — no approximation caveat rides
these answers.

**One engine, one truth.** The tools call the SAME lib fns the CLIs
call, and the `=X.Y.Z` pin on `stack:org.vibevm/typescript-ai-native`
holds this package's vendored copies and the consumer's installed stack
to one resolved version set. Tool-level failure — a red gate, an
absent-toolchain refusal WITH ITS RECIPE — is an `isError` RESULT;
protocol errors are reserved for the transport grammar.

## The parity map {#parity-map}

| Tool | CLI |
|---|---|
| `init` | `discipline-typescript init [--namespace] [--force]` |
| `floor` | `discipline-typescript floor [--keep-going]` |
| `conform_check` | `conform-typescript check [--scope] [--baseline]` |
| `conform_freeze` | `conform-typescript freeze [--baseline]` |
| `specmap_check` | `specmap-typescript --check` |
| `specmap_write` | `specmap-typescript` |
| `trace_explain` | `discipline-typescript trace <target> [--json] [--prose]` |
| `test_gate` | `discipline-typescript test-gate [--baseline]` |
| `tripwire` | `discipline-typescript tripwire [--base] [--debt]` |
| `health` | `discipline-typescript health [--out]` |
| `fast_loop` | `discipline-typescript fast-loop [--cell] [--budget-secs] [--enforce-budget]` |
| `codemod_add_cell` | `discipline-typescript codemod add-cell <cell> <spec-uri>` |
| `tcg_validate` | `tcg-typescript validate <file> [--content-from]` |
| `tcg_scope` | `tcg-typescript scope <file> [--position]` |
| `tcg_complete` | `tcg-typescript complete <file> --position [--prefix] [--max]` |
| `tcg_type` | `tcg-typescript type <file> --position` |
| `tcg_bench` | `tcg-typescript bench --corpus --report` |

`specmap-typescript --gate` (the package-self-trace form) stays
CLI-only, as on the rust side. The TS umbrella has no ledger command —
seventeen tools, not eighteen.

## The discipline half {#discipline-tools}

Twelve thin adapters over `discipline_cli_typescript` /
`conform_cli_typescript` / `specmap_cli_typescript` lib fns, each
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
goes through `tcg_cli_typescript::enrich_validate` — the gate's own
rules — with the policy reloaded per call, so a mid-session freeze is
honoured immediately. `tcg_validate`'s `isError` mirrors the one-shot
exit contract: an error diagnostic OR a non-baselined finding. The
NDJSON serve relay (`tcg-typescript serve`, TCG-PROTOCOL-v0.1) remains
shipped in the stack as the non-MCP embedding form.
