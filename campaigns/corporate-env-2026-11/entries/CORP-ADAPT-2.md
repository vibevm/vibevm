# CORP-ADAPT-2

Planned at commit `9d8428d07`. Line numbers are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** vibe-secrets decides reference-binding admission from the agent vocabulary

**Write.** `crates/vibe-secrets/Cargo.toml`, `crates/vibe-secrets/src/references.rs`, `crates/vibe-secrets/src/references/vocabulary_tests.rs`, `Cargo.lock`, `specmap.json`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-secrets --lib reference_harnesses_match_agent_vocabulary`

**Test.** crates/vibe-secrets/src/references/vocabulary_tests.rs, fn reference_harnesses_match_agent_vocabulary

**Read first.**
- crates/vibe-secrets/src/references.rs (the items REFERENCE_HARNESSES, known_reference_harness, reference_binding_supported, and the `mod native_settings;` line)
- crates/vibe-secrets/Cargo.toml
- crates/vibe-core/src/agent_ids.rs

**Steps.**
1. In crates/vibe-secrets/Cargo.toml [dependencies], add `vibe-core.workspace = true` directly after `vibe-safefs.workspace = true`.
2. In references.rs, replace the body of reference_binding_supported with `vibe_core::agent_ids::AgentId::from_harness(harness).is_some_and(|agent| agent.row().reference_binding)`. Keep its signature and doc comment.
3. In references.rs, after the existing `mod native_settings;` line, add:
```
#[cfg(test)]
mod vocabulary_tests;
```
4. Create references/vocabulary_tests.rs with `#[test] fn reference_harnesses_match_agent_vocabulary`. Assert:
- the BTreeSet of REFERENCE_HARNESSES equals the BTreeSet of every Some(harness) in vibe_core::agent_ids::AGENTS;
- for each row with Some(h), known_reference_harness(h) is true and reference_binding_supported(h) == row.reference_binding;
- reference_binding_supported("qwen-code") is false.
5. Run cargo fmt --all and cargo clippy -p vibe-secrets --all-targets -- -D warnings. Then run cargo xtask specmap and commit its diff together with the Cargo.lock change.

**Decisions already made.**
- REFERENCE_HARNESSES stays a const with its current order, because vibe-connections iterates it for listings. The test pins it to the vocabulary.
- known_reference_harness keeps its body.
- references/compatibility.rs `matches!(harness, "pi" | "qwen")` is a provider API rule, not identity, and stays.
- Adding vibe-core is cycle-free: vibe-secrets already reaches it through vibe-safefs.
- specmap.json is generated. On a merge conflict, take main's version and rerun cargo xtask specmap on top of it. Never hand-merge it.

**Not in this entry.** Not in this entry: the private native_settings Kind enum, vibe-connections, and any change to the binding storage format.

**Depends on.** CORP-ADAPT-1

Entries that write the same file run one after another, never in parallel.
