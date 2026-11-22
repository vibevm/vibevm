# CORP-ADAPT-5

Planned at commit `9d8428d07`. Line numbers are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** Launch harness kinds and specdoc agent conditions are pinned to the agent vocabulary

**Write.** `crates/vibe-workspace/tests/agent_vocabulary.rs`, `specmap.json`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-workspace --test agent_vocabulary`

**Test.** crates/vibe-workspace/tests/agent_vocabulary.rs, fns harness_kinds_match_agent_vocabulary and specdoc_agents_match_agent_vocabulary

**Read first.**
- crates/vibe-workspace/src/system_prompt/harness.rs (HarnessKind::ALL, HarnessKind::as_str), re-exported as vibe_workspace::system_prompt::HarnessKind
- crates/vibe-specdoc/src/doc.rs (CondAgent::ALL, CondAgent::as_str), public as vibe_specdoc::doc::CondAgent
- crates/vibe-core/src/agent_ids.rs

**Steps.**
1. Create the integration test crates/vibe-workspace/tests/agent_vocabulary.rs. It uses std::collections::BTreeSet, vibe_core::agent_ids::AGENTS, vibe_specdoc::doc::CondAgent and vibe_workspace::system_prompt::HarnessKind. vibe-core and vibe-specdoc are already regular dependencies of vibe-workspace.
2. Write `#[test] fn harness_kinds_match_agent_vocabulary`. It asserts that the BTreeSet<&str> of HarnessKind::ALL.iter().map(|k| k.as_str()) equals the BTreeSet of every Some(harness) in AGENTS.
3. Write `#[test] fn specdoc_agents_match_agent_vocabulary`. It asserts that the BTreeSet<&str> of CondAgent::ALL.iter().map(|a| a.as_str()) equals the BTreeSet of every Some(specdoc) in AGENTS.
4. Run cargo fmt --all and cargo clippy -p vibe-workspace --all-targets -- -D warnings. Then run cargo xtask specmap and commit any diff it produces.

**Decisions already made.**
- No change to HarnessKind or the vibe-core manifest delivery. The phase acceptance forbids a SYS refactor now, so a parity test is the only form.
- CondAgent stays in vibe-specdoc, which does not depend on vibe-core. The parity test lives in vibe-workspace because it depends on both crates.
- Pi and Hermes are not specdoc conditions today. Their specdoc column is None, and that stays.
- specmap.json is generated. On a merge conflict, take main's version and rerun cargo xtask specmap on top of it. Never hand-merge it.

**Not in this entry.** Not in this entry: editing harness.rs, doc.rs or the manifest system_prompt harness code, or adding pi or hermes to CondAgent.

**Depends on.** CORP-ADAPT-4

Entries that write the same file run one after another, never in parallel.
