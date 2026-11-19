# Pi runtime admission — selected native bootstrap seam

Date: 2026-11-19. Executable architecture decision for existing SYS-HERMES-PI, following builder PI-RUNTIME-ADMISSION-PROPOSAL-20261119.md. Builder owns implementation, tests and integration under current owner authority; architect reviews. No new PlanMode selection cycle is needed. This is not full parity acceptance.

## Decision and boundary

Select the exported `createAgentSessionRuntime`/native services/native modes seam with invocation-owned factory admission. Do not invent callbacks on `main(args, options)`: its admitted options expose extension factories but not the needed session supervisor. Do not patch native module/class constructors to capture a session. Preserve native InteractiveMode, print and RPC owners, not a custom UI or agent loop.

The actual SDK three-arm result demonstrates the error-gate mechanism, not a production adapter: successful submission1, missing-prefix report-only1, owned throwing error gate0. Retain that baseline and its failure evidence.

## Bounded implementation order

1. Bind exact installed Pi1.5.2 modules/runtime hashes, approved source candidate and own task tree. Inspect actual exported signatures before calls; no guessed runtime options. Keep trusted bootstrap/proof scripts outside downloaded package directories. No operator HOME/auth configuration or upstream module edits.
2. Create a thin proof bootstrap using existing exported native services, session runtime factory and modes. Initial interactive proof uses the native parser's offline mode, explicit selected cwd, and no initial prompt. This is a finite proof input, NOT an approved reduction of the final product's CLI surface. Use isolated synthetic config/auth and the existing enforced offline boundary.
3. In the native creation factory, bind the carrier to the actual created SessionManager/session before any prompt. Carrier registration must occur through the native resource loader. Admit project/personal input through existing Vibe preparation; no parallel layer solver or ambient worker prefix.
4. Attach the fail-closed listener to the actual runner before returning the session. Match stable owned extension metadata plus `before_agent_start` and admitted session, not error text or a guessed path. Propagate a named refusal preserving the cause; unrelated extension errors retain native reporting. A broad event-only listener is proof scaffolding, not accepted product code.
5. Use native runtime replacement for new/resume/fork/import. Do not overwrite UI-owned single rebind/invalidation callbacks. Track and unsubscribe only owned listeners at the correct teardown. Prove what happens when the same session reloads its ExtensionRunner: a factory used only on session replacement does not automatically cover reload. Use the existing native extension-binding lifecycle if available; return the precise missing hook otherwise. No instance-method patch disguised as a lifecycle API.
6. Exercise actual native InteractiveMode startup/exit and runtime disposal without model submission. Exercise native print/RPC mode dispatch separately with finite no-provider inputs where their signatures allow it. Do not claim all modes passed from an interactive constructor.
7. Before product launch wiring, map the actual native parser's recognized argument families to their native implementation owners: selection/cwd, new/resume/fork, model/provider/thinking, prompt/files/stdin, tools/resources/extensions, trust/settings, interactive/print/RPC and informational/subcommands. Preserve behavior through native owners. Unsupported options must refuse before session effects, never silently disappear. A proof-only small argv set cannot close first-class parity. If preserving the full surface requires duplicating native main, report that exact bootstrap gap for a decision rather than copying it.
8. Integrate only through existing Vibe typed harness/composer/launcher/preset/projection owners after the native seam passes. Bind any new bootstrap file owner explicitly in the candidate report; do not create a generic cross-agent framework. Keep Hermes changes out of this atom.

## Observed native lifecycle constraint

Source inspection of the pinned installed runtime shows `AgentSession.bindExtensions(onError)` stores one retained listener, while native `InteractiveMode.bindCurrentSessionExtensions` supplies its own UI reporting callback. `_applyExtensionBindings` unsubscribes the retained old callback and installs the current one on the runner. An additional direct `runner.onError` listener can coexist with UI initially, but `_buildRuntime` creates a new runner during reload. A creation-factory listener alone therefore does not establish reload-safe admission. The caller-owned `reload({beforeSessionStart})` hook is not automatically the hook used by native UI reload commands.

One bounded actual reload proof is required to confirm this gap. Do not accept the factory as production-complete, overwrite the UI callback, wrap private instance methods or invent a public lifecycle hook. Return the precise missing integration capability for an architectural decision if no admitted lifecycle owner can preserve both UI reporting and carrier admission.

## Bounded public-abort alternative after the observed gap

One source-bound alternative avoids the single UI error binding entirely: an invocation-owned extension closure can be bound to the actual created AgentSession, then request `void session.abort()` only on its own carrier error and let native reporting preserve that error. This means the public session method, not `ctx.abort()` (which native UI can redirect). In the inspected runtime, public abort sets the active-run cancellation flag synchronously before awaiting idle; the native agent-start handler observes that flag before the first request. Do not await abort inside the before-start handler, because idle waits for that handler/run to finish.

This is a hypothesis, not an accepted gate. A finite actual three-arm stream-sentinel proof must show successful1 / unguarded-error1 / guarded-error0 submissions, plus visible error/cancellation. If it passes, verify the same owned binding across actual reload with native UI-style error reporting preserved. Bind each replacement session through the runtime factory; never inspect or set private cancellation fields. No global callback replacement, method patch or provider wrapper is permitted. If the public abort does not prevent submission, retain the failure and return the precise gap rather than adding timing guesses or retries.

## Public-abort hypothesis rejected by actual proof

`PI-PUBLIC-ABORT-RESULT.json` records actual task `buzfhwf08`, exit1: the owned before-start handler called public `session.abort()` without awaiting it, then threw, but the test stream boundary was still invoked once. The zero-invocation criterion failed in the first guarded arm; reload was NOT RUN after that assertion. Do not adopt this alternative as production admission or retry unchanged. This witnesses stream invocation, not an actual network request on an aborted signal. Retain the narrower working SDK throwing-listener result and its unresolved native UI/reload conflict; full Pi adapter acceptance remains open.

## Selected additive lifecycle proof after abort rejection

Test a public event-based reattachment instead of replacing the UI's retained callback. An invocation-owned extension factory closes over a holder bound to the actual AgentSession after creation and before first prompt. Its `session_start` handler reads the public `session.extensionRunner` and attaches the owned filtered additive `onError` gate; its `session_shutdown` handler unsubscribes only that gate. Initial bootstrap attaches before any prompt. Each runtime replacement gets its own admitted holder/session.

The inspected native reload rebuilds resources and the runner, performs the UI's before-session-start binding, then emits `session_start` with reason reload when native bindings exist. Thus the additive listener can be reattached after native UI binding without overriding its single callback. Actual proof must confirm this ordering, missing-carrier zero submissions after reload, native handling of unrelated errors, and no duplicate/stale owned listeners. No private field access or native method patch is permitted. If events or binding are absent, refuse before prompt and report the gap; do not claim the factory has solved native main's lack of session access. This candidate replaces the rejected public-abort hypothesis, not its failure evidence.

## Additive mechanism accepted within SDK/reload scope

Actual task `bd7qp0yy2` exit0, PI-ADDITIVE-LIFECYCLE-GATE-RESULT.json, implements the named inline extension with an actual session holder and public session-start/shutdown handlers. Architect independently read the proof, verified proof SHA256 `d9c01f178dcadbc7be9bdf6f0cfcdc2e21f66e55af73f0160a81cdc070416f1a` and log SHA256 `9c7ba0a2d7890a4190202ffc04ee85ef0649d3aee858481b276fbee124b07b52`. Valid delivery reaches the test stream once; missing prefix before and after actual reload reaches it zero times with one owned refusal. Actual loaded extension metadata/event/session identity filters the listener; unrelated errors retain UI-shaped reporting. Reload installs the owned listener twice in total without duplicate refusal. This resolves the observed additive SDK lifecycle gap without UI callback replacement.

Remaining next atom: assemble the selected exported runtime factory and actual native InteractiveMode around this proven mechanism, with no initial prompt, then normal startup/exit/disposal and one session replacement/reload. Verify native UI callbacks, resource discovery, tools and trust remain intact. UI-shaped callback proof is not actual TUI proof; no provider call is required for startup/exit. Preserve native argument parsing and report exact unsupported bootstrap gaps rather than copying main. Product argument/mode coverage and pre-compaction admission still require explicit integration evidence. Do not repeat the earlier rejected abort experiment or re-prove the same helper.

## Finite proof and review

- Positive actual runtime-factory/native event delivers full stable prefix before exact fresh personal tail; test stream sentinel remains reachable.
- Missing carrier and foreign-mutation errors from the owned extension produce zero sentinel submissions and a named refusal; unrelated extension error does not become a carrier refusal.
- Actual session replacement and runner reload re-admit the right binding or refuse before prompt; old listeners/contexts do not remain active, unrelated sessions remain unmodified.
- Native UI/resources/trust/session persistence use native owners; bounded startup/exit restores terminal and disposes the owned runtime. No real provider/cache-savings claim.
- Final entrypoint arguments are tested at that entrypoint, not inferred from the SDK result. Report unsupported surface precisely and keep full integration open until resolved.
- Builder inspects ownership, dependency direction, lifecycle and complexity before handover. Architect reviews actual candidate/diff, runtime hashes, proof oracles and exact results. Green tests alone are not design acceptance.

Stop after this finite native seam proof and exact product-wiring proposal. Do not grow an unbounded API survey, custom CLI framework, second provider loop, payload-poisoning workaround, forced system prompt or global patch. The existing Hermes report deliverable remains separate and is written only after actual Hermes implementation/review.
