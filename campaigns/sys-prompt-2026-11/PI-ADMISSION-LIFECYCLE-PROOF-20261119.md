# Pi SDK admission and native reload — actual finite evidence

Date2026-11-19. Installed Pi1.5.2/Node24.18.0 is the proof subject. This extends accepted ordering evidence, not full CLI/native parity.

## Actual pre-submit boundary

Task `bhwb227ce` exit0 exercised actual native ModelRuntime, createAgentSession, session.prompt preflight/start-run and ExtensionRunner error callbacks in netless isolation. Synthetic runtime authentication existed only in the private ephemeral HOME; no operator credentials were read. An instance-local test stream function counted arrival at the submission boundary and threw before any real provider request.

Three actual arms:

| Arm | Carrier errors | Sentinel submissions | Named refusal |
|---|---:|---:|---|
| Valid prefix, error gate attached |0|1|no|
| Missing prefix, reporting listener only |1|1|no|
| Missing prefix, throwing owned error gate |1|0|yes|

This establishes a supported SDK error propagation seam with a non-vacuous positive control. It does not establish native CLI extension-only veto support. The proof listener matched event only; product integration must additionally match stable owned extension identity and exact session, preserve unrelated error reporting/cancellation and unsubscribe only owned registrations. No test sentinel is proposed as a product provider wrapper.

Receipt: `/data/git/vibevm-native-carrier-20261119/PI-SDK-ERROR-GATE-RESULT.json`, with exact proof/log SHA256.

## Actual reload and binding boundary

Task `bihgkonz0` exit0 exercised actual AgentSession.reload and bindExtensions:

1. A direct listener attached to the original ExtensionRunner observed the first carrier error.
2. Reload replaced the runner; the original direct listener was not transferred.
3. The native retained bindExtensions(onError) callback survived another actual reload.
4. A subsequent native bindExtensions(onError) replaced that single retained callback.

Actual InteractiveMode source uses its own later bindExtensions(onError) UI reporter. Therefore a creation factory attaching a listener once does not prove durable admission across both UI binding and reload. No UI callback was overwritten, no method/class/module patched, and no public hook invented.

Receipt: `/data/git/vibevm-native-carrier-20261119/PI-RELOAD-LIFECYCLE-RESULT.json`, with actual module/proof/log fingerprints. The SDK experiment is executed evidence; the UI call-site observation is source review, not a full interactive UI test.

## Architectural disposition and next decision

The architect accepted the three-arm SDK mechanism as feasibility only and selected exported native runtime/services/modes as the candidate integration seam. The finite reload proof returns a precise remaining gap: additive persistent admission supervision across UI rebind and runner reload has not been established through a supported hook. See `PI-RUNTIME-ADMISSION-03.md` and the builder proposal. Do not copy native main, replace native UI, poison payload/auth, force system prompts or patch constructors to conceal that gap.

Full CLI argv preservation, runtime-factory startup, native UI exit, new/resume/fork/import, resources/trust and Vibe adapter integration remain open. No real model/provider request, external publication or operator configuration change.
