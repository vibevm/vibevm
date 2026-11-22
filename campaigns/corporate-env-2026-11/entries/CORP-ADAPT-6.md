# CORP-ADAPT-6

Planned at commit `9d8428d07`. Line numbers are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** Deploy and projection client words are pinned to the agent vocabulary

**Write.** `crates/vibe-lifecycle/src/mechanism.rs`, `crates/vibe-lifecycle/src/mechanism/client_vocabulary_tests.rs`, `specmap.json`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-lifecycle --lib deploy_clients_match_agent_vocabulary`

**Test.** crates/vibe-lifecycle/src/mechanism/client_vocabulary_tests.rs, fn deploy_clients_match_agent_vocabulary

**Read first.**
- crates/vibe-lifecycle/src/mechanism.rs (its pub(crate) mod list)
- crates/vibe-lifecycle/src/mechanism/deploy/skill/client.rs (SkillClient, cfg(test) ALL, as_str)
- crates/vibe-lifecycle/src/mechanism/deploy/plugin/client.rs (PluginClient, cfg(test) ALL, as_str); it is re-exported as deploy::plugin::PluginClient
- crates/vibe-lifecycle/src/mechanism/client_projection/client.rs (ProjectionClient, as_str)
- crates/vibe-core/src/agent_ids.rs

**Steps.**
1. In crates/vibe-lifecycle/src/mechanism.rs, directly after the `pub(crate) mod zip;` line, add:
```
#[cfg(test)]
mod client_vocabulary_tests;
```
mechanism.rs is at 551 lines and must stay under 600.
2. Create mechanism/client_vocabulary_tests.rs with these imports: super::deploy::skill::client::SkillClient, super::deploy::plugin::PluginClient, super::client_projection::client::ProjectionClient and vibe_core::agent_ids::AgentId. If a path is private at that level, use the nearest pub(crate) re-export and do not change any visibility outside the new test file.
3. Write `#[test] fn deploy_clients_match_agent_vocabulary`. Collect the as_str of SkillClient::ALL, PluginClient::ALL and [ProjectionClient::Claude, ProjectionClient::Codex, ProjectionClient::OpenCode]. For each word, assert that AgentId::from_harness(word) is Some(id) (panic message names the word) and that id.row().projection == Some(word).
4. Run cargo fmt --all and cargo clippy -p vibe-lifecycle --all-targets -- -D warnings. Then run cargo xtask specmap and commit any diff it produces.

**Decisions already made.**
- The three enums and their as_str literals stay. They are const fns, and the strings are fingerprint domain separators and receipt words, so they are stored identities.
- The test lives in a new child file because deploy/plugin.rs (597 lines) and deploy.rs (580 lines) are close to the 600-line budget.
- Package-declared deploy clients are not part of this phase.
- specmap.json is generated. On a merge conflict, take main's version and rerun cargo xtask specmap on top of it. Never hand-merge it.

**Not in this entry.** Not in this entry: replacing the closed client enums with vocabulary-driven records, adding a Qwen deploy client, or changing any module's visibility.

**Depends on.** CORP-ADAPT-5

Entries that write the same file run one after another, never in parallel.
