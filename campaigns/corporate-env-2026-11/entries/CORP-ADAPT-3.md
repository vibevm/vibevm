# CORP-ADAPT-3

Planned at commit `9d8428d07`. Line numbers are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** vibe mcp install agent names come from the agent vocabulary

**Write.** `crates/vibe-agent-projection/src/agents.rs`, `crates/vibe-agent-projection/src/agents/vocabulary_tests.rs`, `specmap.json`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-agent-projection --lib agent_keys_come_from_vocabulary`

**Test.** crates/vibe-agent-projection/src/agents/vocabulary_tests.rs, fn agent_keys_come_from_vocabulary

**Read first.**
- crates/vibe-agent-projection/src/agents.rs (enum Agent, Agent::ALL, Agent::as_str, Agent::parse_filter, the child mod lines ambient_paths/home_paths)
- crates/vibe-core/src/agent_ids.rs

**Steps.**
1. In impl Agent add `pub fn id(self) -> vibe_core::agent_ids::AgentId` with this mapping: ClaudeCode->Claude, ClaudeCodeDesktop->ClaudeDesktop, Cursor->Cursor, OpenCode->OpenCode, Codex->Codex, QwenCode->Qwen, Pi->Pi, Hermes->Hermes. Give it a one-line /// doc.
2. Add `pub fn from_id(id: vibe_core::agent_ids::AgentId) -> Option<Agent>`, implemented as `Agent::ALL.iter().copied().find(|agent| agent.id() == id)`. Give it a one-line /// doc.
3. Leave Agent::as_str unchanged.
4. Rewrite parse_filter as follows. Keep the arm "all". Keep the three alias arms: "claude-code" -> ClaudeCode, "claude-code-desktop" -> ClaudeCodeDesktop, "qwen" -> QwenCode. Every other value goes through `vibe_core::agent_ids::AgentId::from_projection(other).and_then(Agent::from_id)`: Some(agent) returns Ok(vec![agent]), and None bails with exactly the current message text. Use no .unwrap() or .expect().
5. After the existing `mod home_paths;` line add:
```
#[cfg(test)]
mod vocabulary_tests;
```
6. Create agents/vocabulary_tests.rs with `#[test] fn agent_keys_come_from_vocabulary`. Assert:
- for every agent in Agent::ALL: Some(agent.as_str()) == agent.id().row().projection, and Agent::from_id(agent.id()) == Some(*agent);
- the BTreeSet of Agent::ALL ids equals the BTreeSet of AGENTS ids whose projection is Some;
- parse_filter returns exactly vec![agent] for ("qwen", QwenCode), ("qwen-code", QwenCode), ("claude-code", ClaudeCode), ("claude-code-desktop", ClaudeCodeDesktop) and ("hermes", Hermes);
- parse_filter("gemini") is Err.
7. Run cargo fmt --all and cargo clippy -p vibe-agent-projection --all-targets -- -D warnings. Then run cargo test -p vibe-agent-projection so the existing parse_filter and doctest assertions still pass. Then run cargo xtask specmap and commit its diff. agents.rs is at 468 lines and must stay under 600 (check with wc -l).

**Decisions already made.**
- The enum Agent and its variant names stay. AgentUserDirectories.native keys on Agent and must not change.
- as_str keeps its literal match. The test agent_keys_come_from_vocabulary pins it to the vocabulary. vibe-agent-projection is a gated crate, so non-test code may not use unwrap or expect.
- The CLI aliases (claude-code, claude-code-desktop, qwen) stay local to parse_filter because they are user input conveniences, not stored keys.
- The error text of an unknown --agent value is unchanged byte for byte.
- specmap.json is generated. On a merge conflict, take main's version and rerun cargo xtask specmap on top of it. Never hand-merge it.

**Not in this entry.** Not in this entry: config paths, formats, markers or any other per-agent behaviour in agents.rs. Only the identity lookup in parse_filter moves.

**Depends on.** CORP-ADAPT-2

Entries that write the same file run one after another, never in parallel.
