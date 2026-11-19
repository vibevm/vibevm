# Pi native append ordering — bounded prototype acceptance

Date2026-11-19. Actual proof runtime is published Pi1.5.2 under Node24.18.0; equivalence to the separate pinned upstream repository is not claimed. Existing native SDK/extension attachment is reused, not replaced by a custom agent loop.

## Implemented candidate

Private proof candidate `pi-order-exact-retry/carrier.mjs` preserves the exact native SessionManager ID guard before file I/O. It validates event/options and append type before mutation, admits nonempty prefix bytes on the first selected session event and freezes them in that carrier closure. A private WeakMap records each options object's original tail and last written value. Output is prefix alone for an empty tail, otherwise prefix plus two newlines plus the exact original tail. Repeated owned options remain unchanged; foreign mutation is reported without overwrite. New options capture fresh personal text. No text-includes heuristic, replacing system prompt, module/class patch or alternate prompt pipeline remains.

This candidate was authored directly by the builder under the owner's explicit outage instruction, not by the timed-out coding worker. Prior candidates and RED evidence remain retained. This is a proof candidate, NOT shipped Vibe adapter integration.

## Actual checks

Finite tests cover empty/undefined/null, nonempty and leading/trailing-newline tails; caller text identical to the project prefix; same-object repeat; fresh tail; foreign edit; invalid options/type; missing prefix with no mutation; unrelated-session return before prefix file access; and unchanged captured prefix after a disk edit.

Actual native ModelRuntime/DefaultResourceLoader/createAgentSession/ExtensionRunner execution confirms prefix-before-nonempty-tail for a selected one-tool session and exclusion of an unrelated actual full-tool session. The other-session oracle was strengthened after architectural review: actual returned options must exist and carry the exact unchanged value, with no fallback to an expected default.

Actual native `onError` observation also confirms that an invalid append reports a `before_agent_start` error and returns the unchanged invalid value17. **Native ExtensionRunner catches handler exceptions and continues. This is NOT production fail-closed delivery.** A production pre-submit gate with zero model submissions on carrier failure remains mandatory. No provider request, `session.prompt`, CLI/worker/fork/resume or cache-hit proof follows from this assembly check.

The architect reviewed the actual candidate architecture, fingerprints and retained logs, accepted the bounded ordering prototype, and explicitly kept the production failure gate and full integration open.

## Evidence and remaining obligations

Private evidence root `/data/git/vibevm-native-carrier-20261119/`: `PI-ORDERING-RESULT.json`, `control/pi-order-proof.mjs`, accepted revised candidate and passing logs `bamf0b6lq`, `b67csmn3l`, `bei98xvvp`, `b23vy3azv`. The receipt fingerprints actual files and records the proof revisions; earlier reviewed hashes do not silently stand for later revisions.

Next: integrate through existing Vibe/native owners, prove production failure blocks submission, then actual CLI, fresh personal layers, quiet workers, fork/resume and native resources. No global permissions/config/auth changes, upstream module edits or external publication were performed.
