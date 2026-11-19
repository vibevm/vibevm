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

## Finite proof and review

- Positive actual runtime-factory/native event delivers full stable prefix before exact fresh personal tail; test stream sentinel remains reachable.
- Missing carrier and foreign-mutation errors from the owned extension produce zero sentinel submissions and a named refusal; unrelated extension error does not become a carrier refusal.
- Actual session replacement and runner reload re-admit the right binding or refuse before prompt; old listeners/contexts do not remain active, unrelated sessions remain unmodified.
- Native UI/resources/trust/session persistence use native owners; bounded startup/exit restores terminal and disposes the owned runtime. No real provider/cache-savings claim.
- Final entrypoint arguments are tested at that entrypoint, not inferred from the SDK result. Report unsupported surface precisely and keep full integration open until resolved.
- Builder inspects ownership, dependency direction, lifecycle and complexity before handover. Architect reviews actual candidate/diff, runtime hashes, proof oracles and exact results. Green tests alone are not design acceptance.

Stop after this finite native seam proof and exact product-wiring proposal. Do not grow an unbounded API survey, custom CLI framework, second provider loop, payload-poisoning workaround, forced system prompt or global patch. The existing Hermes report deliverable remains separate and is written only after actual Hermes implementation/review.
