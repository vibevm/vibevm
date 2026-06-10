# ENGINE — conform: the cross-language conformance engine, v0.1

**Status.** Design, beta. Implements the Charter's T3 decision: **our own engine, borrowed frontends.** Rationale (owner-decided): asking a compiler about its own language is one page of code; rebuilding name resolution over tree-sitter is weeks. The balance is made explicit through escalation tiers, not ad-hoc judgment.

**Derives from.** Charter A1 (findings carry explanation chains), A2 (extraction is cached, incremental), A3 (conformance is below the algorithmic floor — no LLM in the checking path), A5 (rules ship with checkers).

---

## 1. Escalation tiers {#tiers}

Every rule record declares `tier` — the minimum analysis depth its check requires. The engine runs the cheapest adequate frontend; escalation is declared, never improvised.

| Tier | Capability | Backend | Cost |
|---|---|---|---|
| **T-lex** | textual invariants (forbidden tokens, file layout, license headers) | ripgrep-class scan (MIT/Unlicense) | ~free |
| **T-syn** | structure: items, attributes, imports, spans, hashes | tree-sitter (MIT) universal; `syn` (MIT/Apache-2.0) for Rust precision | cheap, incremental |
| **T-sem** | types, name resolution, macro/template expansion, real import graphs | per-language compiler frontend (§2) | expensive; cached hard |

Rule examples: R-021 forbidden-idiom scan → T-lex/T-syn; R-002 import-graph isolation → T-syn (Rust) / T-sem (C++ where headers lie); R-020 naming-vs-manifest → T-syn + specmap index; type-flow rules (future) → T-sem.

## 2. Frontends — borrowed, behind one trait {#frontends}

```rust
trait Frontend {
    fn lang(&self) -> Lang;
    fn tier(&self) -> Tier;
    fn extract(&self, files: &[SourceFile]) -> Result<Vec<Fact>, FrontendError>;
}
```

| Lang | T-syn | T-sem | License posture |
|---|---|---|---|
| Rust | `syn` in-process | rust-analyzer crates or `rustc_driver` (nightly caveat) | MIT/Apache-2.0 — clean |
| C++ | tree-sitter-cpp | **libclang** via `clang-sys` — the one-page-AST path | Apache-2.0 w/ LLVM exception — clean |
| TS/JS | tree-sitter / SWC (Apache-2.0) | TypeScript compiler API via a Node **sidecar process** | Apache-2.0 — clean |
| Python | RustPython parser (MIT) in-process | CPython `ast`/`symtable` via sidecar | PSF / MIT — clean |

Sidecar protocol: newline-delimited JSON over stdio, versioned; sidecars emit Facts, nothing else. A frontend crash degrades that language's T-sem rules to `skipped (frontend unavailable)` — visible in the report, never silent.

**Foreign linters as evidence providers.** clippy, eslint, ruff, clang-tidy run as-is; their output is ingested as facts via **SARIF** (the OASIS static-analysis interchange format). We neither reimplement their checks nor fork them — we *cite* them: a Discipline rule may be `check: { tool: "clippy", id: "...", status: enforced }`, and conform's job is orchestration + the checks no generic linter can know (manifest-vs-name, specmap coverage, cell isolation).

## 3. The fact store {#facts}

Language-neutral normalized facts; this is the ledger's "facts class" (LEDGER §3) instantiated:

```json
{ "fact": "item",   "lang": "rust", "path": "crates/vibe-resolver/src/naive.rs",
  "kind": "struct", "symbol": "vibe_resolver::naive::NaiveDepSolver",
  "span": [29, 41], "hash": "sha256:…", "attrs": ["spec(implements=…, r=2)"] }
{ "fact": "import", "from": "vibe_resolver::naive", "to": "vibe_core::manifest" }
{ "fact": "flag_read", "symbol": "…", "site": "crates/vibe-cli/src/registry.rs:88" }
```

- **Key:** `(file content-hash, frontend id+version)`. Facts never rot semantically — re-extraction happens only when the file or the frontend changes. This is what makes conformance incremental: a 1-file diff re-extracts 1 file.
- Store: content-addressed, local + CI-cache; never committed (derived data with a deterministic producer).

## 4. Rules as queries {#rules}

v0.1: rules are Rust implementations of one trait — `fn check(&self, facts: &FactStore, specmap: &Index) -> Vec<Finding>` — compiled into the engine. A declarative query DSL (datalog-flavored) is deliberately deferred: we will know its right shape after ~30 real rules exist, not before (Open Question 2).

**Findings** carry the A1 chain: rule id → why (axiom trace) → span → involved facts → deviation status (a matching `deviates` record downgrades the finding to `deviation-acknowledged`). Output: SARIF (so IDEs and CI render findings for free) + the ratchet baseline file (`conform-baseline.json`): pre-existing findings are frozen per scope; new ones fail the gate; the baseline only shrinks.

## 5. Determinism and gates {#determinism}

Same inputs → byte-identical SARIF (stable ordering, no wall-clock). Tested the way vibevm tests its resolver and codegen: run twice, diff. Gate command: `conform check --baseline conform-baseline.json --scope crates/vibe-resolver` — exit code is the acceptance criterion the Playbook relies on; no human judgment in the loop (A3).

## 6. Open questions {#open}

1. rust-analyzer crates vs `rustc_driver` for Rust T-sem (stability vs fidelity) — decide when the first T-sem Rust rule actually lands; none of the Phase ≤4 checks need it.
2. Query DSL: shape and whether rules become data (loadable rule-packs) — after 30 in-tree rules.
3. Fact schema versioning across frontend upgrades — proposal: schema carries `v`, store segregates by version, mixed reads forbidden.
4. Performance envelope targets (full-workspace cold scan budget; warm incremental budget) — set from Phase 4 measurements.

---

*Any frontend or tier specified here that is not exercised by Playbook Phase 4 is removed from this document rather than carried as aspiration.*
