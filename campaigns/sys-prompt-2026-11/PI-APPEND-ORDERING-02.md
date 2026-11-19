# Pi append ordering — executable bounded correction

Date: 2026-11-19. Existing SYS-HERMES-PI atom, not a new campaign.
Builder owns execution in its assigned tree under the current owner-authorized outage exception. Architect owns design/review. No extra architecture approval cycle is required. This brief grants no provider calls or external publication.

## Actual defect and admitted inputs

The actual native extension/SDK attachment works, but PI-NONEMPTY-APPEND-RED.json shows existing personal append before the project prefix. Current prototype: `/data/git/vibevm-native-carrier-20261119/pi-factory-empty-fix/carrier.mjs`, reviewed source SHA256761f18c295f43d221df52be2abd4ad5b4786b7de25d36be7e552d6b70d94fdb3. Builder verifies the actual base before edits and preserves prior RED evidence. Actual installed Pi runtime, not assumed repository equivalence, remains the proof subject.

## Exact design

- Retain native before_agent_start extension registration and exact SessionManager session-id binding. Unrelated sessions return without reading the prefix file or mutating options. Do not classify main sessions by tool count, prompt length or first invocation.
- Validate the admitted prefix/options only for the selected session. A missing/invalid options object or non-string/non-null append value is an actionable error, never implicit coercion.
- Preserve the original caller append as an exact string. undefined/null/empty mean no nonempty tail; do not turn them into literal text.
- Native append output is `prefix` for an empty tail and `prefix + "\n\n" + originalTail` for a nonempty tail. Do not trim/rewrite the caller tail. Preserve native base/system prompt construction; do not set a replacing system prompt.
- Use a private WeakMap keyed by the actual mutable options object, owned by this carrier closure. Its state records original tail and last value written, plus admitted binding/prefix identity if required. This is bounded ownership bookkeeping, not another prompt pipeline.
- First encounter records the original append and writes the composition. Repeated invocation on the same object must first verify that its current append is exactly the last owned value. If so, leave unchanged for the same binding; never concatenate again or recapture the already-carried prefix as a caller tail.
- If another extension changes that owned value, refuse with an actionable diagnostic. Do not strip guessed prefix substrings, overwrite foreign changes, or silently assert ordering. A new native options object captures its own fresh original tail.
- Admit prefix bytes for the launch/session boundary. Do not silently reread changed prefix contents into an ongoing conversation and claim cache stability. Reject a stale/inconsistent binding or require the existing new-session remedy. Changed personal tails on a fresh native event/object must remain fresh.
- Remove `existing.includes(prefix)`/text-equality heuristics as ownership/deduplication tests. Caller text may legitimately contain identical bytes: preserve that caller text, counting the owned injection separately rather than deleting authored content.

## File/write scope and execution order

1. Record exact candidate base and prototype/runtime hashes. Preserve the previous prototype and RED receipt; write a distinct revised candidate in the assigned proof tree or modify the explicitly bound candidate with retained before bytes. No peer checkout writes, operator configs or installed upstream module edits.
2. Change only the carrier factory and its bounded proof/tests. No Hermes changes or generic cross-agent middleware in this atom.
3. Run focused pure ownership cases, then actual native ExtensionRunner/SDK assembly using the existing offline isolated harness. No session.prompt/model submission is necessary for this correction.
4. Independently inspect the diff for the design above, then report candidate hashes and actual results. Vibe adapter integration and full CLI/worker/resume parity remain subsequent obligations, not a reason to enlarge this patch.

## Required finite cases

1. Empty/undefined/null tail: full prefix inserted exactly once; native base retained.
2. Nonempty tail, including leading/trailing newlines: exact tail bytes after prefix and separator; previous RED becomes a retained positive result.
3. Repeat same options: identical bytes and no additional prefix.
4. Fresh options/new event with changed personal tail: new tail present, old tail absent, project prefix unchanged.
5. Foreign mutation of previously owned options: explicit refusal, no overwrite.
6. Unrelated actual native session, including a full ordinary toolset: options untouched and no prefix file read; selected session is not chosen by tools.
7. Caller text that contains prefix-like or even identical text: authored text not stripped; distinguish original caller text from carrier-owned injection in the test oracle.
8. Invalid append type/options and unavailable prefix: named failure before carrier mutation.

Native proof must exercise the installed extension loader/runner and construction, not only call the handler directly. Actual installed `dist/core/extensions/runner.js::emitBeforeAgentStart` catches handler exceptions, emits an extension error and continues returning options. Consequently, an error in this ordering prototype proves the extension diagnostic and absence of mutation, NOT abortion of a native model turn. Report that distinction explicitly. Production adapter integration must establish admission/error handling at a supported pre-submission owner so a missing carrier or foreign mutation cannot silently proceed without boot. This is a separate integrated fail-closed claim; do not manufacture it with a replacing forceSystemPrompt, global upstream patch or custom agent loop. Reuse previously valid source/runtime/isolation proofs within their exact scope. Do not claim CLI launch, worker spawning, resume or provider cache hits from this assembly test.

## Review and stopping condition

Accept the ordering atom only after the actual native nonempty case, ownership/idempotence/isolation negatives and architectural inspection pass. Return defects for a bounded fix even if narrow tests are green. Handover includes exact files/hashes, failed and passing receipts, design deviations, warnings and remaining adapter integration claims. Finish this atom once these criteria hold; no unrelated cleanup or speculative universal extension framework.
