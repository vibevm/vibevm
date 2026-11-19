# Pi native runtime admission — bounded integration proposal

Date2026-11-19. Builder proposal for architect selection, not implemented support or permission expansion. Reuses accepted ordering prototype and actual three-arm SDK failure-gate proof.

## Established and missing boundary

Native exported `main(args, {extensionFactories})` preserves native argument parsing, interactive/print/RPC dispatch and resources, but MainOptions exposes no session/runtime-factory observer for installing an owned error supervisor. Extension handlers throwing alone do not stop submission. Actual SDK `ExtensionRunner.onError` throwing propagates through native `_prepareRun`/`_startRun` and blocks submissions, proved with successful1/unguarded-error1/guarded-error0 synthetic stream sentinel arms.

Exports `createAgentSessionRuntime`, its full cwd-bound factory, `createAgentSessionServices`, `createAgentSessionFromServices` and native `InteractiveMode` provide the next supported lifecycle boundary. Native runtime reuses the factory for new/resume/fork/import and owns replacement/disposal. UI owns its single rebind/invalidation callbacks; the adapter must not overwrite them.

## Proposed implementation boundary

1. Keep existing Vibe selected-node composer/preparation/launch owners. A small version-bound native Pi bootstrap consumes a launcher-trusted prefix path/session policy, never literal prefix bytes on argv or user-selected executable from a repository.
2. Before coding a replacement entrypoint, establish whether exported bootstrap services suffice to preserve native argument selection without copying `main.js`. Prefer native `main` if an actual supported admission hook can be supplied. Current MainOptions does not expose one; do not invent `onSession`/runtime callbacks or patch global constructors.
3. If architect selects exported runtime/mode bootstrap, define a finite admitted native argument surface explicitly. Preserve selected cwd, config/model/auth/trust/resource/extension resolution through native owners. Reject unsupported arguments before session effects rather than silently dropping them. Full native capability obligations remain; a limited bootstrap is not final first-class parity.
4. Wrap only the exported runtime factory: after native session/services creation and before returning each session, admit that exact session ID and project/personal scope, register its carrier through the existing resource-loader path, and attach a native error listener scoped to the stable owned extension identity, `before_agent_start`, and exact session. Other extension errors retain native reporting and do not trigger this gate.
5. Error identity must come from actual loaded extension metadata, not string heuristics over error prose. Preserve original carrier error/cancellation; no payload poisoning, invalid credentials, forced system prompt, provider wrapper or second model loop.
6. Attach before the first prompt. Use existing session extension binding/reload lifecycle to reinstall only owned admission on the replacement runner. Preserve native UI error callbacks, project-trust context and runtime invalidation. Unsubscribe only owned listeners on final owned teardown; do not override native single callbacks or reuse stale contexts.

## Exact implementation owners to bind after decision

- Existing `crates/vibe-core/src/manifest/package/system_prompt/harness.rs`, workspace harness/constraints and CLI agent launcher/program owners for typed native support and admission.
- A small Rust/TypeScript-owned native Pi bootstrap under the existing carrier ownership selected by the architect; no shipped Python or installed upstream module edits.
- Existing tests for launch args/owned projection and a finite isolated native runtime fixture. No second user-layer solver or generic agent middleware.

## Finite acceptance

- Positive admitted prefix preserves stable native base plus exact current personal tail; actual native stream sentinel reachable.
- Missing/foreign-mutated carrier fails before stream invocation, with positive control and exact owned extension error identity; unrelated extension errors still delivered normally.
- New/resume/fork replacement uses the native runtime factory, re-admits exact session binding before prompt and releases only obsolete owned listeners. Unrelated session receives no prefix.
- Native InteractiveMode lifecycle and UI/resources/trust/auth remain native; no operator config mutation or provider request in proof. Native CLI argv preservation/refusals tested at final entrypoint, not inferred from SDK tests.

## Decision requested

Choose the supported native bootstrap/admission seam and exact bounded initial argument contract. The known gap is specifically `main` accepting extension factories but not exposing the session error-supervisor hook. Do not treat this as permission to copy the CLI implementation, silently degrade capabilities or declare extension-only fail-closed support.
