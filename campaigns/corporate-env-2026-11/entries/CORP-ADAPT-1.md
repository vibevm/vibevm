# CORP-ADAPT-1

Planned at commit `9d8428d07`. Line numbers are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** vibe-core owns one agent vocabulary with a per-agent key row

**Write.** `crates/vibe-core/src/agent_ids.rs`, `crates/vibe-core/src/lib.rs`, `vibevm/vibespecs/modules/vibe-mcp/PROP-015-mcp-integration.xml`, `specmap.json`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-core --lib agent_ids::tests::every_surface_key_is_unique_and_round_trips`

**Test.** crates/vibe-core/src/agent_ids.rs, #[cfg(test)] mod tests, fn every_surface_key_is_unique_and_round_trips

**Read first.**
- crates/vibe-core/src/lib.rs
- crates/vibe-core/src/channel.rs (only to see how a module carries specmark::scope! below its //! doc comment)
- vibevm/vibespecs/modules/vibe-mcp/PROP-015-mcp-integration.xml (the agent-detection section: AGENT-SET, AGENT-PRESENCE, AGENT-DETECTION-DEFAULT)

**Steps.**
1. Create crates/vibe-core/src/agent_ids.rs. Start it with a //! doc comment saying it is the one owner of agent identity and of the key each surface stores. Follow that with the line `specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-mcp/PROP-015#AGENT-VOCABULARY");`.
2. Define `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)] pub enum AgentId { Claude, ClaudeDesktop, Cursor, OpenCode, Codex, Qwen, Gemini, Hermes, Pi }` in exactly this variant order. Give each variant a one-line /// doc. Give the enum a /// summary line followed by this compiled doctest (vibe-core is in gated_pub_doctest):
~~~
/// ```
/// use vibe_core::agent_ids::AgentId;
/// assert_eq!(AgentId::from_harness("qwen"), Some(AgentId::Qwen));
/// assert_eq!(AgentId::Qwen.row().projection, Some("qwen-code"));
/// ```
~~~
3. Define `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub struct AgentRow { pub id: AgentId, pub harness: Option<&'static str>, pub projection: Option<&'static str>, pub preset: Option<&'static str>, pub specdoc: Option<&'static str>, pub reference_binding: bool }`. Give each field a /// doc. harness is the launch and AI-reference key. projection is the MCP/skill key that `vibe mcp install --agent` uses. preset is the agents.toml preset name. specdoc is the specdoc `when` agent key. reference_binding means the agent may keep an AI reference binding. Give the struct a /// summary line followed by this compiled doctest:
~~~
/// ```
/// use vibe_core::agent_ids::{AgentId, AGENTS};
/// assert_eq!(AGENTS[AgentId::Pi as usize].id, AgentId::Pi);
/// assert!(AgentId::Pi.row().reference_binding);
/// ```
~~~
4. Define `pub const AGENTS: [AgentRow; 9]` in AgentId order, so that AGENTS[i].id as usize == i, with these values:
- Claude: harness Some("claude"), projection Some("claude"), preset Some("claude-code"), specdoc Some("claude"), reference_binding false.
- ClaudeDesktop: harness None, projection Some("claude-desktop"), preset None, specdoc Some("claude-desktop"), reference_binding false.
- Cursor: harness None, projection Some("cursor"), preset Some("cursor"), specdoc Some("cursor"), reference_binding false.
- OpenCode: harness, projection, preset and specdoc all Some("opencode"), reference_binding false.
- Codex: harness, projection, preset and specdoc all Some("codex"), reference_binding false.
- Qwen: harness Some("qwen"), projection Some("qwen-code"), preset Some("qwen-code"), specdoc Some("qwen-code"), reference_binding true.
- Gemini: harness None, projection None, preset Some("gemini"), specdoc None, reference_binding false.
- Hermes: harness Some("hermes"), projection Some("hermes"), preset Some("hermes"), specdoc None, reference_binding false.
- Pi: harness Some("pi"), projection Some("pi"), preset Some("pi"), specdoc None, reference_binding true.
5. Add `impl AgentId`:
- `pub const fn row(self) -> &'static AgentRow { &AGENTS[self as usize] }`
- `pub fn from_harness(key: &str) -> Option<AgentId>`, which returns the first row whose harness == Some(key).
- `pub fn from_projection(key: &str) -> Option<AgentId>`, the same for projection.
- `pub fn from_preset(name: &str) -> Option<AgentId>`, the same for preset.
Give each a one-line /// doc. Do not use .unwrap() or .expect() outside the test module.
6. Add `#[cfg(test)] mod tests` with `fn every_surface_key_is_unique_and_round_trips`. For each (i, row) in AGENTS: assert row.id as usize == i and row.id.row() == row. For each Some harness, projection and preset key: assert the matching from_* returns Some(row.id). Then assert:
- from_harness("qwen") == Some(AgentId::Qwen)
- from_projection("qwen-code") == Some(AgentId::Qwen)
- from_preset("claude-code") == Some(AgentId::Claude)
- from_harness("gemini") == None
- from_harness("qwen-code") == None
- the set of ids with reference_binding == true is exactly {Pi, Qwen}
7. In crates/vibe-core/src/lib.rs add `pub mod agent_ids;` as the first `pub mod` line, before `pub mod cancellation;`.
8. In PROP-015-mcp-integration.xml, inside the agent-detection <facts> element and directly after the line that ends with `</AGENT-DETECTION-DEFAULT>`, insert this line (8 spaces of indent):
```
        <AGENT-VOCABULARY fact="true" status="impl/done">`vibe_core::agent_ids` owns agent identity. One row per agent names the key each surface stores for it: the launch harness key (`qwen`), the MCP and skill projection key (`qwen-code`), the app preset name (`claude-code`), the specdoc `when` key, and whether the agent may keep an AI reference binding. Surfaces keep their stored keys and their own listing order.</AGENT-VOCABULARY>
```
9. Run cargo fmt --all, cargo clippy -p vibe-core --all-targets -- -D warnings and cargo test -p vibe-core --doc agent_ids (the two doctests must compile and pass). Then run `cargo xtask specmap` to regenerate specmap.json (it is slow on a cold target) and commit the diff it produces.

**Decisions already made.**
- The owner is the module vibe_core::agent_ids. vibe-core depends on no vibe-* crate (its in-repo dependencies are progress-core and the vendored specmark), so every consumer can depend on it without a cycle.
- Stored keys stay exactly as they are today. The vocabulary records a key per surface and does not rename anything.
- Parity is checked on sets, never on order. Each surface keeps its own listing order.
- No capability booleans beyond reference_binding. A surface's membership is shown by its key being Some.
- AgentId and AgentRow each carry one compiled doctest, because vibe-core is in conform.toml gated_pub_doctest and the conform baseline is empty. AGENTS and the fns are not types and need none.
- The spec anchor is a new fact, AGENT-VOCABULARY, in PROP-015's agent-detection facts.
- specmap.json is generated and records lines and fingerprints. Regenerate it with cargo xtask specmap. On a merge conflict, take main's version and rerun cargo xtask specmap on top of it. Never hand-merge it.
- This entry assumes the lead has already committed tools/cargo-test-ran.sh on main. The entry does not write that file.

**Not in this entry.** Not in this entry: changing any consumer crate (secrets, projection, presets, workspace, lifecycle, specdoc), the SYS HarnessKind and manifest delivery code, or tools/cargo-test-ran.sh.

**Spec edit (apply as a step of this entry).**

vibevm/vibespecs/modules/vibe-mcp/PROP-015-mcp-integration.xml: add the AGENT-VOCABULARY fact (exact line given in the steps) after the AGENT-DETECTION-DEFAULT fact.

Entries that write the same file run one after another, never in parallel.
