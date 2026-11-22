# Hermes/Pi product adapter boundary — source-bound proposal

Date2026-11-19. Builder integration proposal after actual CLI/SDK/UI proofs. Not shipped functionality or full parity acceptance. No repeat PlanMode cycle; architect selects the explicit native-language and CLI bootstrap contracts below.

## Existing Vibe owners

- Typed declaration/defaults: `crates/vibe-core/src/manifest/package/system_prompt/harness.rs`.
- Typed native family, file delivery, conflicts and composed-prefix preparation: `crates/vibe-workspace/src/system_prompt/{harness,constraints,flags}.rs`; preserve existing personal-layer composer and source-only lane distinction.
- Executable discovery, launch and teardown: `crates/vibe-cli/src/commands/agent/{launchers,launch,program}.rs`. Add narrowly owned native bootstrap dispatch below this owner, not another agent framework.
- Ordinary interactive/one-shot presets: `crates/vibe-app/src/agents/presets.rs`.
- Native owned skill/MCP capability projection: `crates/vibe-agent-projection/src/agents.rs` and existing per-agent home/ambient path/receipt owners. Do not invent configuration schemas; native unsupported capability is explicit.

## Delivery and ownership

The Rust launcher prepares selected-node stable prefix and fresh personal/override continuation through existing owners. Prefix bytes travel in a private admitted UTF-8 file, never argv, shared user config, SOUL or persistent native identity files. Native bootstrap receives only invocation-owned file path and binding information. It must preserve normal native project instruction discovery for the universal dependency ownership rule or deliver an equivalent owned policy where it actually substitutes the router. No second layer solver.

Runtime discovery is launcher-bound: resolve the user-selected native executable/package environment, verify supported version/module identity before attaching; fail softly to existing classic read delivery with an actionable warning when unsupported, without claiming reduced fallback completes parity. Never switch provider/auth/endpoint to make readiness pass. Native worker creation is not the main session: no ambient prefix environment inherited into arbitrary child agents.

## Hermes selected technical seam and unresolved packaging choice

Actual pinned Hermes is Python and its CLI uses a private native gateway. The proven route is native `hermes_cli.main.main` plus native owned GatewayRunner/authority/listener and the upstream explicit gateway endpoint input; credentials stay in process/child environment, never logs/receipts. A thin native Python bootstrap is technically necessary to invoke those Python APIs without reimplementing them in Rust/TypeScript. Propose a small reviewed resource `crates/vibe-cli/src/commands/agent/native/hermes_bootstrap.py`, embedded/published by the Rust launch owner and executed only by the discovered Hermes environment's interpreter. Rust remains the single-binary Vibe CLI and owns preparation, process lifecycle and asset publication; ordinary Vibe gains no Python runtime requirement. This helper executes only inside the already installed Hermes Python runtime, as agent-native carrier glue analogous to Pi's ESM extension, not a supporting Vibe crate or replacement subsystem. This is an EXPLICIT native-language boundary for architectural review, not permission to ship the proof or silently supersede LANG-RUST/LANG-REVISIT. If this boundary cannot be maintained, a language-contract change requires the owner.

The bootstrap must import native APIs, never copy upstream prompt assembly/UI/session code. It owns one gateway process, preserves native operator profile/resource locations and authentication/admission, and forwards supported native CLI arguments unchanged. Native endpoint credentials must not be serialized into durable launch artifacts. Version/hash admission is explicit; package updates outside the supported seam produce a warning/refusal before attaching, not unreviewed monkeypatch compatibility.

Attach only instance-owned runner turn-context and prompt methods with captured originals. Every turn validates exact authority source identity, platform/route/session/invocation; consult native cache under its lock. Same native key/different session requires re-admission or a named refusal before prompt assembly. Reject existing foreign instance shadows and double-attach. Preserve upstream signature/message-count/dead-session invalidation. Do not add a cache or patch module/class globals. Detach only exact owned attributes after the admitted invocation/session ends; native teardown owns clients/resources.

Remaining product gap: the no-provider proof directly calls native construction after actual CLI admission; it is not an actual CLI-submitted model turn. Product bootstrap must bind the admitted session via a supported native authority admission event/API, not poll for the first session. Exact public admission hook must be established before wiring this draft into launch. If unavailable, return the precise native integration contract gap rather than intercept private state globally.

## Pi selected technical seam

Propose a small native ESM bootstrap `crates/vibe-cli/src/commands/agent/native/pi_bootstrap.mjs`, embedded/published by the same Rust launch owner. It imports the discovered supported Pi package exports; no vendored upstream modules or alternate provider loop. Native `main(args,{extensionFactories})` alone lacks the session supervisor hook. Use exported `createAgentSessionRuntime`, native services and InteractiveMode/print/RPC owners, as the approved executable brief selects.

Each native runtime factory creates its actual SessionManager/services, validates the selected invocation binding and admits prefix availability BEFORE prompt/startup/possible compaction, registers a named inline extension, then returns the native session/runtime. A closure-bound actual session reference attaches an additive native error listener identified by loaded extension metadata, `before_agent_start` and exact session. `session_start` reattaches to the current public ExtensionRunner after native UI rebind/reload; `session_shutdown` removes only the owned listener. Other extension errors retain UI reporting. Do not replace the UI-owned single retained callback. This mechanism has actual SDK/reload proof and real InteractiveMode init/reload/stop proof, not full command-loop coverage.

Prefix/options ownership uses the accepted factory: once-admitted project bytes before exact fresh personal tail; WeakMap original/lastWritten; foreign mutation refusal, no text includes heuristic or force prompt. New/resume/fork/import must use native runtime replacement and new admission; quiet/unrelated sessions are excluded. Native print/RPC mode dispatch and complete argv preservation remain required.

## Native argument and resource contract before product completion

Map every recognized native parser family to the native owner: cwd/session selection, new/resume/fork, model/provider/reasoning, prompt/files/stdin, tools/resources/extensions, trust/settings, UI/print/RPC and informational/subcommands. Preserve it or refuse an unsupported option before session effects. Do not silently drop options, copy hundreds of lines of native main or label the proof-only offline/read-tool subset first-class parity. Native HOME/profile/auth/config, memory/skills/MCP and workers remain native.

## Finite integrated gates

1. Typed manifests and default/read fallback; executable/runtime discovery and argument collisions, unsupported-version/actionable warning before attachment.
2. Final launched native input: stable full prefix, fresh personal/override tail, universal boot policy, no cross-project/worker leakage.
3. Hermes exact source/route/cache/foreign-owner admission and teardown; native same cached agent, different-ID re-admission/refusal, actual native process/CLI ingress without shared-gateway changes.
4. Pi native UI command lifecycle, runtime replacement and reload, exact owned errors yield zero model submissions while unrelated errors retain reporting; include existing-history precompaction admission, not only fresh-session proof.
5. Actual CLI argv/mode preservation and native resource/worker/resume/fork paths. No real provider request inferred from installation/proof authority; live model/adoption remains separately authorized and reported.

## Decisions needed now

- Hermes: permit minimal necessary native Python glue as a reviewed exception consistent with the project language contract, and identify the supported authority admission hook for exact session binding. No proof file ships as-is.
- Pi: approve the concrete ESM bootstrap owner and require the finite full-parser-to-native-owner mapping before launch wiring; reject copying main or silently narrowing final parity.

The accepted standalone owner report `~/temp/HERMES.md` is written by the architect only after actual Hermes implementation/review, in plain language organized by user features and measured-versus-potential savings, not by internal spec IDs. No report is fabricated now.
