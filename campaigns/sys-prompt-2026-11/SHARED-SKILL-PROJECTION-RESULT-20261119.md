# Pi projection and shared physical skill ownership — coherent candidate

2026-11-19. Base `6f33c5c9c` (reviewed shared-target contract integrated). Implementation candidate under SHARED-SKILL-TARGETS-01.md; not complete Hermes/Pi/SYS acceptance.

## Delivered owners

Pi is registered once in existing projection Agent inventory, using native JSON mcpServers (`.pi/agent/mcp.json`, project `.pi/mcp.json`) and generic `.agents/skills`. Existing stdio/HTTP payload and MCP receipt register/repeat/remove preserve foreign entries. CLI composition root captures PI_CODING_AGENT_DIR into an explicit agent-keyed directory map; pure path resolution never reads environment and refuses nonabsolute/parent-traversal override instead of fallback. Existing safefs publication/identity guards remain. Injected caller literals updated for the additive Rust context field; no manifest/wire grammar change for agent paths.

Package-skill receipt epoch3/schema4 has one physical target, canonical nonempty logical `agents` aliases and one file inventory. Frozen generated singleton epoch2 types admit schemas2/3 and convert to singleton groups; current publication uses one SCHEMA4 constant. No hand-edited generated reader, parallel transaction engine or new journal. Canonical JTD, registry, corpus and normal codegen outputs updated; SP-FMT-118 and PROP-073 shared-target fact record the semantics.

Logical selected agents/artifact IDs remain visible. Receipt writer groups equal same-binding physical paths; different binding collisions still refuse. Existing transaction/recovery matches physical path, publishes/removes each tree once and emits logical reports per alias. Group activation preflights complete alias intents including retracted groups; contradictory single-agent changes refuse before retaining impossible state. Complete selected-group changes retain provenance for each alias. Unselected aliases also prevent a subset update from silently changing shared payload bytes. Removing one association retains the physical tree; status `shared-unbound` accurately reports same-binding physical exposure rather than falsely calling it foreign or independently disabled. Last association removal uses existing owned-file guards.

## Actual verification

Private evidence root `/data/git/vibevm-native-carrier-20261119/control/`:

- Final `shared-skills-final-runtime.log`:89 library tests PASS,18 doctests PASS. Includes defaultALL physical grouping/both aliases/bytes+mtime repeat, old singleton3→current4 publication, single-off refusal/no receipt mutation, complete group off/on, one-agent re-enable after group-off refusal, one-association retention/shared-unbound, final removal, command-facing grouped activation/uninstall, existing concurrency/crash/recovery/portability/source drift controls.
- `shared-skills-coherent-gates.log`: wire corpus2PASS, MCP95PASS, CLI registration31PASS; then conform2NEW failures. Its shell group continued and returned0: overall0 is NOT accepted as green. Original results preserved.
- Fixed env read at existing CLI composition root and split alias admission from activation mutation. `shared-skills-final-conform.log`:0 new,141 existing findings.
- Initial unstaged check-codegen compared worktree to Git index and reported intentional new generated emission as drift. Exact50 generated output files staged (no product commit yet), then `shared-skills-staged-codegen.log`: actual check-codegen clean/exit0. It regenerates normally and checks byte equality against staged emission; no generated file hand edits or toolchain replacement.
- `shared-skills-final-wire.log`: reporting green under public=false; precise new schema/corpus bound separately by actual e3 wire roundtrip test. Existing predecessor corpora unchanged. Final staged wire-diff `buuypopow` exit0 reports exactly1 schema,1 corpus and1 registry change; no unchanged-wire claim. Exact accepted81-file candidate committed `89d3e188b`; independent review bound every staged blob to manifest SHA256 `500f3611b7767be8fc040c57ec46df2e86ed8c4c7401be9a85fc389fd036e0fe`. Hermes YAML follow-up is excluded.
- Pi native resource loader fixture passed global/trusted-project MCP, untrusted-project exclusion and generic skill discovery, no servers/provider launched. Pi receipt test1PASS and injected default/override/path refusal tests PASS; exact earlier logs in NATIVE-PROJECTION-PREFLIGHT report.

## Retained failures / review disposition

Initial full projection82PASS2FAIL exposed actual Codex/Pi shared target collision and old diagnostic golden. First grouped runtime82PASS3FAIL exposed expected Pi ALL inventory and two fixture schema3 literals, corrected to current schema owner. Later full suites pass without weakening foreign/collision/activation guards. Initial codegen refused missing explicit x-empty policy; fixed canonical schema. Prior conform and unstaged codegen failures remain recorded, not erased or counted PASS.

No Windows/macOS native projection execution, actual delegated worker, Hermes MCP final registration or full cross-agent parity claim. Hermes native YAML helper stage render/remove is separately tested and remains uncommitted/unintegrated, excluded from this shared projection candidate. Root instructions, working rules, old dependency dirt and unrelated proposal drafts remain excluded. Future central per-agent adapter consolidation is explicitly deferred by owner, not a current prerequisite.
