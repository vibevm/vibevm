# ENGINE — conform: the cross-language conformance engine, v0.1 {#root}

<status stage="spec" state="done"/>

##status-line **Status.** Design, beta. @impl/done

##IMPLEMENTS-T3-OWN-ENGINE-BORROWED-FRONTENDS Implements the Charter's T3 decision: **our own engine, borrowed frontends.** @impl/done

##rationale-compiler-is-a-page-reimplementation-is-weeks Rationale (owner-decided): asking a compiler about its own language is one page of code; rebuilding name resolution over tree-sitter is weeks. @spec/done

##BALANCE-IS-EXPLICIT-THROUGH-ESCALATION-TIERS The balance is made explicit through escalation tiers, not ad-hoc judgment. *Specified, not built: the whole tier vocabulary of §1 is unimplemented. There is no `Tier` type, no `tier` field and no escalation path in `core-ai-native-conform` or in any frontend crate, stack CLI or host driver; `T-lex` / `T-sem` appear nowhere in the tree and `T-syn` only inside three doc comments that quote this document. What is actually explicit is the **frontend** choice, made by the caller at the call site — which is a different mechanism, and an ad-hoc one.* @spec/done

##derives-from-lead **Derives from.** @impl/done

- ##DERIVES-A1-EXPLANATION-CHAINS Charter A1 (findings carry explanation chains), @impl/done
- ##DERIVES-A2-CACHED-INCREMENTAL A2 (extraction is cached, incremental), @impl/done
- ##DERIVES-A3-BELOW-THE-ALGORITHMIC-FLOOR A3 (conformance is below the algorithmic floor — no LLM in the checking path), @impl/done
- ##DERIVES-A5-RULES-SHIP-WITH-CHECKERS A5 (rules ship with checkers). @impl/done

---

## 1. Escalation tiers {#tiers}

##RULE-RECORD-DECLARES-ITS-TIER Every rule record declares `tier` — the minimum analysis depth its check requires. *Specified, not built: no rule record declares a tier, because a rule record has no such field. The shipped contract is `pub trait Rule { fn id(&self) -> &'static str; fn why(&self) -> &'static str; fn check(&self, facts: &[SourceFacts]) -> Vec<Finding>; }` (`conform/src/finding.rs:53-57`) — three methods, none of them `tier`. All fifteen shipped rules take whatever facts they are handed.* @spec/done

##ENGINE-RUNS-THE-CHEAPEST-ADEQUATE-FRONTEND The engine runs the cheapest adequate frontend; escalation is declared, never improvised. *Specified, not built: the engine performs no selection, so it can neither pick the cheapest nor escalate. `store.rs:95-118` exposes one entry point per language, each taking a caller-supplied `&dyn Frontend` — the choice is made outside the engine, by whoever calls it, which is precisely the improvisation this sentence rules out.* @spec/done

| Tier | Capability | Backend | Cost |
|---|---|---|---|
| ##ROW-TIER-T-LEX **T-lex** @spec/done | textual invariants (forbidden tokens, file layout, license headers) @spec/done | ripgrep-class scan (MIT/Unlicense) — *Specified, not built: this tier has no implementation and no backend. No ripgrep-class scanner is wired into the engine, and no forbidden-token, file-layout or license-header rule exists in the roster (`rules/mod.rs:21-25`).* @spec/done | ~free @spec/done |
| ##ROW-TIER-T-SYN **T-syn** @spec/done | structure: items, attributes, imports, spans, hashes @impl/done | tree-sitter (MIT) universal; `syn` (MIT/Apache-2.0) for Rust precision — *Half built: the `syn` half is real and running (`rust-ai-native-conform-frontend`, whose own module doc calls itself "the Rust T-syn frontend"). The **universal tree-sitter backend does not exist** — `tree-sitter` / `tree_sitter` return no hit in any crate or manifest in the repository, so there is no universal path and each language got a bespoke frontend instead. The capability column is accurate; the tier that would name it is not.* @spec/done | cheap, incremental @impl/done |
| ##ROW-TIER-T-SEM **T-sem** @spec/done | types, name resolution, macro/template expansion, real import graphs @spec/done | per-language compiler frontend (§2) — *Partly built, but not as a tier: one real compiler frontend ships. `typescript-ai-native-conform-frontend` reaches the TypeScript Compiler API through the packaged `tools/ts-extract` sidecar, and the Go stack drives `gopls` as an LSP oracle in `go-ai-native-tcg`. Neither is reachable as a **tier**: both are plain `Frontend` implementations a caller names directly, there is nothing to escalate from, and no rule declares it needs them.* @spec/done | expensive; cached hard @spec/done |

##rule-examples-lead Rule examples: @impl/done

- ##EXAMPLE-R-021-FORBIDDEN-IDIOM R-021 forbidden-idiom scan → T-lex/T-syn; *Specified, not built: R-021 is not a rule in this engine. `R-021` returns zero hits across every crate in the repository, and no forbidden-idiom scan of any kind ships. It is cited as a ban in the language guides and authored nowhere.* @spec/done
- ##EXAMPLE-R-002-IMPORT-GRAPH-ISOLATION R-002 import-graph isolation → T-syn (Rust) / T-sem (C++ where headers lie); *The rule is real; the mapping is not. `CellIsolation` carries id `"R-002"` (`conform/src/rules/structure.rs:77-91`) and is projected per language as `TsCellIsolation` and `GoCellIsolation`. What does not exist is either side of the arrow — no tier assigns it a depth, and there is no C++ frontend to escalate to.* @spec/done
- ##EXAMPLE-R-020-NAMING-VS-MANIFEST R-020 naming-vs-manifest → T-syn + specmap index; *Specified, not built: neither the rule nor the join. `R-020` returns zero hits across every crate, and the conform engine does not depend on the specmap crate at all — no manifest lists it — so a rule combining structural facts with the specmap index cannot be written today without a new dependency edge.* @spec/done
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
| ##ROW-FRONTEND-RUST Rust @impl/done | `syn` in-process @impl/done | rust-analyzer crates or `rustc_driver` (nightly caveat) — *Specified, not built: the T-sem column names software that is absent. `rust-analyzer`, `rustc_driver`, `ra_ap` and `hir` return no hit in the engine, in `rust-ai-native-conform-frontend`, or in any manifest. The T-syn column is exact and running.* @spec/done | MIT/Apache-2.0 — clean @impl/done |
| ##ROW-FRONTEND-CPP C++ — *Specified, not built, in full: there is no C++ frontend at either depth. `tree-sitter`, `tree-sitter-cpp`, `libclang` and `clang-sys` return no hit anywhere in the repository, no C++ crate exists in any workspace, and the `Fact` model (`conform/src/facts.rs:25`) carries no C++ variant. This row is a design intention; the three language stacks that ship are Rust, TypeScript and Go.* @spec/done | tree-sitter-cpp @spec/done | **libclang** via `clang-sys` — the one-page-AST path @spec/done | Apache-2.0 w/ LLVM exception — clean @spec/done |
| ##ROW-FRONTEND-TS-JS TS/JS @impl/done | tree-sitter / SWC (Apache-2.0) @impl/done | TypeScript compiler API via a Node **sidecar process** @impl/done | Apache-2.0 — clean @impl/done |
| ##ROW-FRONTEND-GO Go @impl/done | `go/parser`+`go/ast` via a stdlib-only **`go run` sidecar** (go-extract) @impl/done | gopls / `go vet` as evidence providers — *Built, but at another layer, and not as evidence providers. Both tools run: `go vet ./...` is step 2 of the Go floor (`go-ai-native-cli/src/floor.rs:115-120`) and `gopls` is driven as a long-lived LSP oracle by `go-ai-native-tcg`. Neither reaches conform — conform ingests no output from either, so nothing they know becomes a fact. Read this cell as naming where semantic depth lives in the Go stack, not as a conform frontend.* @spec/done | BSD-3 — clean @impl/done |
| ##ROW-FRONTEND-PYTHON Python @impl/done | RustPython parser (MIT) in-process @impl/done | CPython `ast`/`symtable` via sidecar @impl/done | PSF / MIT — clean @impl/done |

##SIDECAR-PROTOCOL-IS-NDJSON-OVER-STDIO Sidecar protocol: newline-delimited JSON over stdio, versioned; sidecars emit Facts, nothing else. @impl/done

##FRONTEND-CRASH-DEGRADES-VISIBLY-NEVER-SILENTLY A frontend whose toolchain is broken is a **hard error**: each stack's driver probes its frontend before extraction and fails the run (`typescript-ai-native-conform/src/lib.rs:66-70`), so the gate can never report green over zero facts. A per-file extraction failure surfaces on stderr and yields an empty fact set for that file. *Specified, not built: there is no `skipped (frontend unavailable)` report status — `Finding` carries no status field.* @spec/done

##FOREIGN-LINTERS-ARE-EVIDENCE-PROVIDERS **Foreign linters as evidence providers.** clippy, eslint, ruff, clang-tidy run as-is; their output is ingested as facts via **SARIF** (the OASIS static-analysis interchange format). @impl/done

##LINTERS-ARE-CITED-NOT-REIMPLEMENTED We neither reimplement their checks nor fork them — we *cite* them: a Discipline rule may be `check: { tool: "clippy", id: "...", status: enforced }`, and conform's job is orchestration + the checks no generic linter can know (manifest-vs-name, specmap coverage, cell isolation). *The posture holds; the record shape and two of the three examples do not. Foreign linters really are run as-is and never reforked — the floor shells out to `cargo clippy`, `go vet`, `staticcheck`, `exhaustive`, `prettier`, `tsc` and `eslint`. But the citation is a **floor step**, not a rule field: the `check: { tool, id, status }` shape returns zero hits across the engine, so no rule can cite a linter finding. Of the three checks named as conform's own, **cell isolation ships** (R-002); manifest-vs-name does not (R-020 is unauthored) and specmap coverage cannot, since conform does not depend on the specmap crate. Orchestration, too, lives one layer up in each stack's `floor`, not in the engine.* @spec/done

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

##UNEXERCISED-FRONTEND-OR-TIER-IS-REMOVED *Any frontend or tier specified here that is not exercised by Playbook Phase 4 is either removed from this document or annotated in place as **specified, not built** — never carried as unmarked aspiration.* @impl/done
