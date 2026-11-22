# CORP-ADAPT-4

Planned at commit `9d8428d07`. Line numbers are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** App agent presets are pinned to the agent vocabulary

**Write.** `crates/vibe-app/src/agents/presets.rs`, `specmap.json`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-app --lib presets_match_agent_vocabulary`

**Test.** crates/vibe-app/src/agents/presets.rs, mod tests, fn presets_match_agent_vocabulary

**Read first.**
- crates/vibe-app/src/agents/presets.rs
- crates/vibe-core/src/agent_ids.rs

**Steps.**
1. In the existing #[cfg(test)] mod tests of presets.rs add `#[test] fn presets_match_agent_vocabulary`. It asserts that the BTreeSet of presets().iter().map(|p| p.name) equals the BTreeSet of every Some(preset) in vibe_core::agent_ids::AGENTS. It also asserts that for each preset, vibe_core::agent_ids::AgentId::from_preset(preset.name) is Some.
2. Run cargo fmt --all and cargo clippy -p vibe-app --all-targets -- -D warnings. Then run cargo xtask specmap and commit any diff it produces. Check presets.rs with wc -l and keep it under 600 lines.

**Decisions already made.**
- PRESETS keeps its literals and order. The doctest on presets() pins the order, and agents.toml stores the names.
- This is a test-only change. vibe-app already depends on vibe-core, so no Cargo change is needed.
- specmap.json is generated. On a merge conflict, take main's version and rerun cargo xtask specmap on top of it. Never hand-merge it.

**Not in this entry.** Not in this entry: deriving preset names from the vocabulary at compile time, or adding or removing presets.

**Depends on.** CORP-ADAPT-3

Entries that write the same file run one after another, never in parallel.
