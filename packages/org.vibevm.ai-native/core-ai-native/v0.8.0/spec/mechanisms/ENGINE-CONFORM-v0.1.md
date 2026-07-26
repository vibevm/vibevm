# ENGINE — conform: the cross-language conformance engine, v0.1 {#root}

<status stage="spec" state="done"/>

##status-line **Status.** Design, beta. @impl/done

##IMPLEMENTS-T3-OWN-ENGINE-BORROWED-FRONTENDS Implements the Charter's T3 decision: **our own engine, borrowed frontends.** @impl/done

##rationale-compiler-is-a-page-reimplementation-is-weeks Rationale (owner-decided): asking a compiler about its own language is one page of code; rebuilding name resolution over tree-sitter is weeks. @spec/done

##BALANCE-IS-EXPLICIT-THROUGH-ESCALATION-TIERS The balance is made explicit through escalation tiers, not ad-hoc judgment. @impl/done

##derives-from-lead **Derives from.** @impl/done

- ##DERIVES-A1-EXPLANATION-CHAINS Charter A1 (findings carry explanation chains), @impl/done
- ##DERIVES-A2-CACHED-INCREMENTAL A2 (extraction is cached, incremental), @impl/done
- ##DERIVES-A3-BELOW-THE-ALGORITHMIC-FLOOR A3 (conformance is below the algorithmic floor — no LLM in the checking path), @impl/done
- ##DERIVES-A5-RULES-SHIP-WITH-CHECKERS A5 (rules ship with checkers). @impl/done

---

## 1. Escalation tiers {#tiers}

##RULE-RECORD-DECLARES-ITS-TIER Every rule record declares `tier` — the minimum analysis depth its check requires. @impl/done

##ENGINE-RUNS-THE-CHEAPEST-ADEQUATE-FRONTEND The engine runs the cheapest adequate frontend; escalation is declared, never improvised. @impl/done

| Tier | Capability | Backend | Cost |
|---|---|---|---|
| ##ROW-TIER-T-LEX **T-lex** @impl/done | textual invariants (forbidden tokens, file layout, license headers) @impl/done | ripgrep-class scan (MIT/Unlicense) @impl/done | ~free @impl/done |
| ##ROW-TIER-T-SYN **T-syn** @impl/done | structure: items, attributes, imports, spans, hashes @impl/done | tree-sitter (MIT) universal; `syn` (MIT/Apache-2.0) for Rust precision @impl/done | cheap, incremental @impl/done |
| ##ROW-TIER-T-SEM **T-sem** @impl/done | types, name resolution, macro/template expansion, real import graphs @impl/done | per-language compiler frontend (§2) @impl/done | expensive; cached hard @impl/done |

##rule-examples-lead Rule examples: @impl/done

- ##EXAMPLE-R-021-FORBIDDEN-IDIOM R-021 forbidden-idiom scan → T-lex/T-syn; @impl/done
- ##EXAMPLE-R-002-IMPORT-GRAPH-ISOLATION R-002 import-graph isolation → T-syn (Rust) / T-sem (C++ where headers lie); @impl/done
- ##EXAMPLE-R-020-NAMING-VS-MANIFEST R-020 naming-vs-manifest → T-syn + specmap index; @impl/done
- ##EXAMPLE-TYPE-FLOW-RULES type-flow rules (future) → T-sem. @spec/done

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
| ##ROW-FRONTEND-RUST Rust @impl/done | `syn` in-process @impl/done | rust-analyzer crates or `rustc_driver` (nightly caveat) @impl/done | MIT/Apache-2.0 — clean @impl/done |
| ##ROW-FRONTEND-CPP C++ @impl/done | tree-sitter-cpp @impl/done | **libclang** via `clang-sys` — the one-page-AST path @impl/done | Apache-2.0 w/ LLVM exception — clean @impl/done |
| ##ROW-FRONTEND-TS-JS TS/JS @impl/done | tree-sitter / SWC (Apache-2.0) @impl/done | TypeScript compiler API via a Node **sidecar process** @impl/done | Apache-2.0 — clean @impl/done |
| ##ROW-FRONTEND-GO Go @impl/done | `go/parser`+`go/ast` via a stdlib-only **`go run` sidecar** (go-extract) @impl/done | gopls / `go vet` as evidence providers @impl/done | BSD-3 — clean @impl/done |
| ##ROW-FRONTEND-PYTHON Python @impl/done | RustPython parser (MIT) in-process @impl/done | CPython `ast`/`symtable` via sidecar @impl/done | PSF / MIT — clean @impl/done |

##SIDECAR-PROTOCOL-IS-NDJSON-OVER-STDIO Sidecar protocol: newline-delimited JSON over stdio, versioned; sidecars emit Facts, nothing else. @impl/done

##FRONTEND-CRASH-DEGRADES-VISIBLY-NEVER-SILENTLY A frontend crash degrades that language's T-sem rules to `skipped (frontend unavailable)` — visible in the report, never silent. @impl/done

##FOREIGN-LINTERS-ARE-EVIDENCE-PROVIDERS **Foreign linters as evidence providers.** clippy, eslint, ruff, clang-tidy run as-is; their output is ingested as facts via **SARIF** (the OASIS static-analysis interchange format). @impl/done

##LINTERS-ARE-CITED-NOT-REIMPLEMENTED We neither reimplement their checks nor fork them — we *cite* them: a Discipline rule may be `check: { tool: "clippy", id: "...", status: enforced }`, and conform's job is orchestration + the checks no generic linter can know (manifest-vs-name, specmap coverage, cell isolation). @impl/done

## 3. The fact store {#facts}

##fact-store-is-the-ledgers-facts-class-lead Language-neutral normalized facts; this is the ledger's "facts class" (LEDGER §3) instantiated: @impl/done

```json
{ "fact": "item",   "lang": "rust", "path": "crates/vibe-resolver/src/naive.rs",
  "kind": "struct", "symbol": "vibe_resolver::naive::NaiveDepSolver",
  "span": [29, 41], "hash": "sha256:…", "attrs": ["spec(implements=…, r=2)"] }
{ "fact": "import", "from": "vibe_resolver::naive", "to": "vibe_core::manifest" }
{ "fact": "flag_read", "symbol": "…", "site": "crates/vibe-cli/src/registry.rs:88" }
```

- ##KEY-IS-FILE-HASH-PLUS-FRONTEND-VERSION **Key:** `(file content-hash, frontend id+version)`. Facts never rot semantically — re-extraction happens only when the file or the frontend changes. This is what makes conformance incremental: a 1-file diff re-extracts 1 file. @impl/done
- ##STORE-IS-CONTENT-ADDRESSED-NEVER-COMMITTED Store: content-addressed, local + CI-cache; never committed (derived data with a deterministic producer). @impl/done

## 4. Rules as queries {#rules}

##RULES-ARE-RUST-TRAIT-IMPLS-COMPILED-IN v0.1: rules are Rust implementations of one trait — `fn check(&self, facts: &FactStore, specmap: &Index) -> Vec<Finding>` — compiled into the engine. @impl/done

##QUERY-DSL-IS-DELIBERATELY-DEFERRED A declarative query DSL (datalog-flavored) is deliberately deferred: we will know its right shape after ~30 real rules exist, not before (Open Question 2). @spec/done

##FINDINGS-CARRY-THE-A1-CHAIN **Findings** carry the A1 chain: rule id → why (axiom trace) → span → involved facts → deviation status (a matching `deviates` record downgrades the finding to `deviation-acknowledged`). @impl/done

##OUTPUT-IS-SARIF-PLUS-THE-RATCHET-BASELINE Output: SARIF (so IDEs and CI render findings for free) + the ratchet baseline file (`conform-baseline.json`): pre-existing findings are frozen per scope; new ones fail the gate; the baseline only shrinks. @impl/done

## 5. Determinism and gates {#determinism}

##SAME-INPUTS-BYTE-IDENTICAL-SARIF Same inputs → byte-identical SARIF (stable ordering, no wall-clock). @impl/done

##DETERMINISM-TESTED-BY-RUN-TWICE-DIFF Tested the way vibevm tests its resolver and codegen: run twice, diff. @impl/done

##GATE-EXIT-CODE-IS-THE-ACCEPTANCE-CRITERION Gate command: `conform check --baseline conform-baseline.json --scope crates/vibe-resolver` — exit code is the acceptance criterion the Playbook relies on; no human judgment in the loop (A3). @impl/done

## 6. Open questions {#open}

1. ##OPEN-RUST-T-SEM-BACKEND rust-analyzer crates vs `rustc_driver` for Rust T-sem (stability vs fidelity) — decide when the first T-sem Rust rule actually lands; none of the Phase ≤4 checks need it. @spec/done
2. ##OPEN-QUERY-DSL-SHAPE Query DSL: shape and whether rules become data (loadable rule-packs) — after 30 in-tree rules. @spec/done
3. ##OPEN-FACT-SCHEMA-VERSIONING Fact schema versioning across frontend upgrades — proposal: schema carries `v`, store segregates by version, mixed reads forbidden. @spec/done
4. ##OPEN-PERFORMANCE-ENVELOPE Performance envelope targets (full-workspace cold scan budget; warm incremental budget) — set from Phase 4 measurements. @spec/done

---

##UNEXERCISED-FRONTEND-OR-TIER-IS-REMOVED *Any frontend or tier specified here that is not exercised by Playbook Phase 4 is removed from this document rather than carried as aspiration.* @impl/done
