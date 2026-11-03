# Rust + Lean traceability: source evidence inventory

Source pin: `7fe6376ccef7f321302a8f18610fcb0c9cc2686f` in `/fast/git/v/vibevm-next`. Main comparison: `6d2f6e59be3bf55ac85fb2db67ad4e80b6925d79`. This is a supplemental capability outside the original 222-task denominator. No build, checker, toolchain installation, embedded command, campaign flow or product/spec/plan mutation was run. All conclusions below are source inspection; historical test receipts are labelled separately. Compact inventory: `/tmp/nst-traceability-capabilities.json`.

## Observed result

There is an actual selected-subject Rust+Lean capability, not just a proposed diagram or `verifies` annotation. It composes declaration maps, runs named Rust tests and a real Lean model checker, saves an exact joined witness and freshly rechecks that witness through CLI/MCP. The supported result is **G1 model evidence plus Rust example-test evidence**. It deliberately returns `correspondence = "not_established"`; it does not prove that Rust implements the Lean theorem or that the theorem correctly formalizes prose.

The first commissioned consumer is `fixtures/lean-composition`. Its exact subject is `spec://org.example/same-subject/Protocol#identity`. Rust `rust::identity(value: u64) -> u64` returns `value`; two named libtests check zero and `u64::MAX`. Lean `Example.identity (n : Nat) : n = n := rfl` proves reflexive equality. They share a subject URI. There is no encoded conversion/refinement theorem connecting Rust `u64` semantics to Lean `Nat`, and no automatic semantic validation of the prose-to-theorem association. The code and fixture README state this limitation.

## Exact source and user entrypoints

Canonical family roots discovered from root Cargo dependency and package manifests:

- `vibevm/vibepacks/org.vibevm.ai-native/lean-ai-native-lang/v0.1.0/`: `lean-ai-native-check` library and `lean-ai-native` CLI.
- `vibevm/vibepacks/org.vibevm.ai-native/lean-ai-native-mcp/v0.1.0/`: Lean MCP projection and packaged checker source.
- Host `crates/vibe-trace/src/composition.rs`, `proof.rs`, `proof/{wire,files,rustcheck}.rs`.

The Lean-only command surface is `lean-ai-native --path PROJECT [--toolchain-dir ROOT] scan --carrier ...`, `check --carrier ... --subject URI`, `verify-witness --carrier ... --subject URI --witness ...`. The package MCP tool names are `lean_ai_native_scan`, `lean_ai_native_check`, `lean_ai_native_verify_witness` (`lean-ai-native-mcp/.../src/descriptor.rs`). Scan reports declarations only. Startup supplies project root/toolchain; tool arguments cannot override them.

Host CLI registers `ProofCheck`/`ProofVerify` in `crates/vibe-cli/src/cli.rs:408,411`, dispatches in `main.rs:298,301`, and projects the shared engine in `commands/proof.rs::run`. Commands are `vibe proof-check --path ... --subject ... --witness ...` and `vibe proof-verify ...`; optional toolchain directories are trusted startup inputs. `vibe-mcp/src/tools.rs` registers `ProofCheckMcpTool` and `ProofVerifyMcpTool`. Their wire names are `proof_check` and `proof_verify`; only subject and root-relative witness are permitted. Both call `vibe_trace::proof::execute`, expose its complete typed Reply and report failure as failure rather than omitting its body.

## Declaration mapping: what changed relative to main

Main already has the neutral `CodeScanner`, `CompositeScanner`, `DefaultScanner` (Rust+JTD), `build_with_scanner` and multi-edge graph. These live in main's `core-ai-native/v1.0.0/crates/core-ai-native-specmap/src/scanner.rs` and index owner. Main already allows several code symbols/relations to address one spec unit. A new graph engine is not part of the Lean contribution.

Main also already has `vibe-requirements::RelationProvider` and read-only requirement/fact query with derived current/carried provenance. `crates/vibe-requirements/src/query.rs` is byte-identical at the two pins. The existing provider builds a fresh host map and verifies carried package-map bytes against the lock-selected slot; it does not execute tests or proofs, and its relation projection omits `pinned_r`, suspect details and code bodies.

Next adds a closed `[composition]` table with `kind="rust_lean"`, one exact subject, a Rust symbol, Lean carrier and optional proof policy. `composition::load` removes that table and delegates remaining config to the existing core parser. `composition::build` composes DefaultScanner and Lean carrier scanner through the existing seam, validates namespace, selected Rust `implements` edge, Lean `verifies` declarations and exact spec hash/revision, and refuses warnings/suspects rather than publishing a partial-current composition. `write_map` produces one canonical map through atomic replacement. The fresh requirements relation provider now calls this same composition in memory without writing.

Legacy family parsers remain closed and reject the new table instead of overwriting a combined map with a Rust-only or TypeScript-only result. This solves a concrete consumer problem: sequential independent map writers previously could replace rather than combine declaration coverage. It does not make the `verifies` edge a theorem result. The unchanged requirements projection still carries relation metadata, not a `ProofWitness` or requirement-acceptance decision.

Scope is explicitly one consumer's own namespace. Foreign inbound proof coverage and a generic dynamically discovered multilingual scanner/checker framework are unsupported. Lean code locations point to the authored carrier declaration, not a falsely inferred theorem source span. The fixture uses an authored `@fact` with absent numeric revision; absent is preserved and bound by hashes, not silently promoted to r1.

## What runs when checking

### Rust contribution

`proof/rustcheck.rs::capture` validates one default library in a subdirectory. The manifest accepts only package and limited workspace data; dependency tables, features, custom targets, build scripts and workspace inheritance are refused. Cargo.lock must contain exactly the selected dependency-free package. Explicit source list must equal all local `.rs` files outside `.git`/target. Parsing is real `syn` parsing. A conservative token scan refuses include/include_str/include_bytes/env/option_env/path even in macro bodies or cfg attributes, so undeclared compile-time inputs cannot enter this supported contract.

`rustcheck::check` supports native Linux and checks selected rustc release, commit and host. It builds captured bytes in a private temporary tree with offline/frozen Cargo, private Cargo home/target, cleared environment and explicit compiler paths. Cargo `--message-format=json` must identify exactly one libtest executable inside the private target. It lists tests, invokes each requested name with `--exact` and JSON output, requires exactly one successful test event and a suite with one passed, zero failed and zero ignored. Compilation alone and zero selected tests cannot pass. It records executable hash, input hashes, test outcomes, toolchain bin/lib hash and `/usr/bin/cc`, `ld`, `as` identities, and rechecks them.

The supplied Rust symbol is verified as an `implements` edge in the map and copied into the RustWitness. The checker does **not** establish that a selected test calls that symbol, covers all inputs or proves its contract. Test relevance remains authored/reviewed evidence; a URI association is not semantic correspondence. The current fixture does call the function, as read directly in its source.

### Lean contribution

`lean-ai-native-check/src/carrier.rs::capture` validates the carrier, exact subject/module/theorem, spec file/hash/revision, explicit dependency-first local sources and bounded policy. Sources/proofs have explicit 1..128 limits; selectors currently support ASCII dotted names. `proof.rs::check` runs the configured external Lean distribution's `lean --version` and `--githash`, checks Lean 4.34.1 and its pinned release commit, hashes the bin/lib closure, materializes captured source into a temporary directory and compiles each declared module to `.olean`.

The runner-owned `inspector.lean` imports compiled modules and requires the selected declaration to be `.thmInfo` from the declared originating module. It captures the elaborated type representation and `collectAxioms` transitive closure. This is compiled-environment inspection, not parsing user stdout for a claim of proof. The inspector is a trusted unsafe IO program; theorem validity is checked in a separate required kernel replay.

The runner invokes bundled `leanchecker -v` over **every declared local module**, then checks compiled object identities and toolchain bytes again. This is real kernel replay but not an independently implemented second prover: it trusts the same pinned distribution/imported standard library. The witness includes authored sources, produced `.olean`/server/private object bytes where present, statement and hash, spec identity, allowed/observed axioms, runner/checker-source/inspector identities and toolchain identity.

Only a unique subset of `propext`, `Quot.sound`, `Classical.choice` may be configured as allowed logical axioms. Any observed axiom outside that selected subset refuses. Custom axioms, `sorryAx` and native proof axioms cannot be added to permission policy. The simple fixture permits none and observes none.

### Joined result and freshness

`proof.rs::check` requires two distinct qualified execution IDs, validates G1 policy, captures consumer/specmap/policy/Rust/Lean/spec inputs and requires the private captured closure to reproduce the current complete composed map. It runs Rust first, then Lean, checks exact subject/spec unit hash/revision/outcome, rechecks original and snapshot files plus Rust closure/toolchain, and rebuilds the **whole map** after checking to detect added/removed scan-root membership.

Only then does `proof-check` publish `vibevm/selected-proof-join/1`. The JoinedWitness contains separate RustWitness and LeanWitness, consumer/selection/policy/map identities, checker source identity and explicit assumptions. Grade is hard-coded `g1_model`, correspondence is hard-coded `not_established`.

`proof-verify` snapshots the existing witness bytes, decodes a strict shape, reexecutes Rust and calls Lean `VerifyWitness` (which itself checks freshly), compares the complete resulting join and rereads the saved bytes. It reports the exact witnessed file digest. It does not rewrite a stale witness into success, authenticate a historical run by execution-ID spelling, or lock paths against future/intervening external writers. The implemented guarantee is two matching observations around the fresh run, not hostile-writer atomic snapshot isolation.

## Trust and claim boundaries

| Claim | Source-supported meaning | Stronger claim not established |
|---|---|---|
| Rust `implements` edge | Author attached exact source symbol to spec URI; scanner found it | Source function satisfies prose |
| Lean `verifies` edge | Carrier declares exact theorem/subject binding | Theorem checked or prose correctly formalized |
| Rust passed contribution | Exact named libtests ran successfully on captured narrow source closure | All inputs/paths proved; test relevance automatically checked |
| Lean passed contribution | Selected model theorem kernel-checked under recorded permitted assumptions | Rust refinement, runtime behavior, correctness of model intent |
| Joined witness matched | Fresh Rust and Lean results and exact current bindings equal retained witness | G2/G3/G4 correspondence/system proof or automatic task acceptance |
| Exact toolchain hashes | Identifies configured compiler/checker bytes and detects observed drift | Authenticity signature or verified compiler/OS |
| Private scratch/offline Cargo | Controls declared build inputs for supported subset | Sandbox or no arbitrary runtime I/O |
| Installed family present | Sources/binaries can be materialized and used separately | Host dynamically selects installed checker version |

The source is unusually explicit here: assurance text says G1 only and no semantic equivalence to prose; G3 request is refused. Canonical Lean guide marks G2 conformance and G3/G4 refinement unsupported. Full init/floor/conform/health/sweep, automatic cards, general generators, differential harnesses and codemods remain WISH in Lean 0.1.0. Ordinary Lean definitions remain usable; the carrier selector subset is not a new language dialect.

Rust tests and Lean elaboration are trusted executable project code. The runners are not sandboxes. They bound subprocess time/output and clean process groups on timeout, but do not prove absence of network/file effects. They trust native OS/runtime, compiler/linker and Lean distribution. The recorded bin/lib identity is not release-authentication proof. This is an explicit admitted trust boundary, not an unreported security guarantee.

## Tests and historical runtime evidence

Real source test cases include:

- Composition: `combined_write_and_fresh_relations_preserve_both_same_subject_edges`, `legacy_closed_config_refuses_before_partial_write`, `malformed_duplicate_foreign_and_missing_sources_never_replace_map`, `rust_only_map_remains_the_core_map`, `fresh_read_refuses_missing_scanner_inputs_and_foreign_proof_coverage` in `vibe-trace/src/composition/tests.rs`.
- Join/freshness: `saved_join_replacement_during_checker_is_refused`, `added_scan_root_member_at_checker_boundary_refuses_publication`, `removed_spec_root_member_at_checker_boundary_refuses_publication`, `ambient_compile_inputs_hidden_in_macros_and_attributes_are_refused` in `proof/tests.rs`.
- Real joined check/recheck and negative matrix: `real_selected_checks_and_saved_join_recheck`, `real_selected_join_negative_matrix`; both are **ignored by default**, require explicit real toolchain selection.
- Front-door behavior: `cli_and_mcp_keep_the_same_engine_refusal_body`, `quiet_proof_refusal_still_returns_one_complete_json_value`, `selected_fact_query_preserves_rust_and_lean_relations_without_writing` in CLI integration tests. Real CLI/MCP positive parity is also ignored by default.
- Lean family `tests/contracts.rs`: actual pinned Lean success/reproducibility/forged-statement/wrong-subject/stale-source/spec changes, invalid proof, custom axiom, sorry, local imported dependency changes and omitted dependency. These real integration tests require `LEAN_AI_NATIVE_TEST_TOOLCHAIN` and fail visibly when it is missing; they do not silently skip.
- Lean MCP tests verify exact shared-engine replies, strict inputs, unknown-member startup override refusal, inventory-only scan and absent-checker failure. They do not replace the real checker tests.

Historical records under `campaigns/next/evidence/NEXT-LEAN-A0/` date the supplemental owner-directed work to 2026-10-05. Source commits: `55b23c986` map composition, `48bcda3b3` proof join, `0bf036ab9` final complete-map recheck, `50ee2cbf6` installed-consumer commissioning. The later `installed-gate-acceptance.md` reports seven offline installed slots, four built binaries, 21 bounded commands and source checkouts hidden during execution. It reports agreement among installed Lean CLI/MCP and host joined CLI/MCP and a requirements query with both relations.

Those are historical claimed test results, not rerun results of this audit. The installed-gate report supersedes the earlier composition report's then-open installed-slot proof. The source is consistent with the narrower conclusion: host checking links `lean-ai-native-check` via root Cargo source path at **build time**. Runtime host execution need not read its source checkout, but updating an installed Lean family does not change the already compiled host checker. Installed family checking and host compiled checking are two distinct consumers that were compared, not dynamic plugin dispatch.

## Main overlap and uncompleted user scope

Verified at pinned trees:

- main has no `vibe-trace/src/proof.rs`, `composition.rs`, CLI proof command or host MCP proof tools, and no canonical Lean family.
- main already has CodeScanner/CompositeScanner, Rust/JTD default scanning, requirement relation provider, fresh host map and carried package-map provenance.
- `vibe-requirements/src/query.rs` is identical. `vibe-trace/src/lib.rs` and CLI specmap command differ because the composition path was added.

Thus the supplemental capability is easy to miss if transfer inventory uses only task IDs: it contributes new user-visible checks and exact evidence, while reusing substantial current-main traceability substrate.

Uncompleted scope to keep explicit in the inventory, without prescribing architecture: real dependency-bearing/workspace Rust projects; other native platforms/targets; cross-package inbound subject coverage; broader Lean dependency and selector support; model-to-prose validation; test relevance/coverage; actual Rust↔Lean representation/refinement proof; G2/G3/G4 grades; installed checker discovery/version negotiation; full Lean discipline parity. None follows automatically from successful identity fixture checks. Root should decide which of these belong to the intended user feature before calling the broader capability complete.
