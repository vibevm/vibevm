# Native projection preflight — Pi implementation and shared target defect

2026-11-19; product base `f0b763d21`. Work in progress, not accepted projection closure.

## Pi owner integration

Agent registry adds Pi; MCP uses existing JSON `mcpServers` and command/args or HTTP url payloads, user default `.pi/agent/mcp.json`, project `.pi/mcp.json`. Native `PI_CODING_AGENT_DIR` is captured at ambient boundary into explicit agent-keyed native directory map; pure path resolver uses only injected inputs and refuses relative/parent-traversal overrides rather than falling back to default home. Publication still uses existing no-follow safefs transaction/receipts. Existing caller struct literals updated for the additive Rust field, not a new wire/manifest grammar.

Skills map to native generic `.agents/skills` through existing injected-home and project paths, not an invented Pi directory. Actual resource loader recognizes these generic skills; low-level loadSkills alone does not discover generic package-manager paths.

## Executed evidence

Private root `/data/git/vibevm-native-carrier-20261119/control/`:

- `pi-projection-gates.log`: full projection82PASS/2FAIL; MCP95PASS; downstream CLI check passed. Shell command group returned0 because later commands ran after the failed first test; actual full projection result is FAILED, not aggregate PASS.
- One failure is old unknown-agent diagnostic golden; updated for Pi.
- Real remaining failure is shared Codex/Pi physical `.agents/skills/<name>` collision for the SAME package binding. No dummy path, silent dedup or weakened receipt guard applied.
- `pi-native-override-check.log`: injected default/override/relative-refusal test1PASS and downstream CLI check PASS.
- `pi-projection-focused.log`: Pi injected paths + explicit Pi-only receipt projection2PASS; neighbor skill preserved and repeat unchanged.
- `pi-mcp-receipt-test.log`: actual register/repeat/remove receipt test1PASS, foreign server/operator data preserved.
- `pi-native-projection-resource-loader.log`: native published Pi1.5.2 global MCP accepted; trusted project MCP accepted; untrusted project excluded; generic `.agents/skills` discovered. No MCP servers launched or provider calls. Earlier incorrect direct low-level skill-loader fixtures retained as failed logs.

All native execution used unshare-all sandbox with synthetic HOME/runtime, not operator config/auth.

## Shared physical target blocker

`pkgskill/projection/binding.rs` rejects duplicate physical targets; receipt/state.rs independently enforces exactly one physical target and transaction/activation code keys target rows by logical agent. Codex and Pi legitimately share `.agents/skills`. Simply allowing the planner collision creates invalid receipts/duplicate writes. Simply discarding an agent loses activation/artifact ownership. Different packages targeting the same skill must still refuse.

Architect selected bounded design: one physical owner with logical agent aliases; identical same-binding payload coalesces, conflicting per-agent activation intents refuse rather than silently disabling another agent; removing one logical alias retains physical files while another is active. Canonical receipt/activation contract amendment and exact implementation brief pending. Current dirty candidate is NOT integrated while ordinary default package skill install remains broken.

## Hermes next owner

Pinned Hermes native MCP is YAML `mcp_servers` in profile config.yaml, not JSON. Its native `mcp_config._save_mcp_server` validates then uses native load/save; config.save_config uses existing atomic/comment-preserving writer and unreadable guard. Prefer that installed-runtime owner with exact selected profile home, owned-key projection only, before-fingerprint/lock/recheck and no full config dumps. Never use a lossy whole-document Rust YAML roundtrip or overwrite native auth/persona/foreign keys. A bounded transaction proposal is still required before implementing native mutation.

Full projection, actual worker, native profile/platform and remaining parity obligations stay open; no SYS/NST completion or source push implied.
