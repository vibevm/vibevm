# OpenCode 2.3.2 native configuration

## Supported native sink

The existing public explicit native setup route admits `--agent opencode` for Chat Completions connections. The fixed first-slice destination is `HOME/.config/opencode/opencode.json` in existing protected Unix directories. Explicit config-directory/content/file overrides are refused instead of silently writing an unused default. Native configuration uses the proved built-in compatible package identity `aisdk:@ai-sdk/openai-compatible`, provider `vibevm`, `settings.apiKey`/`baseURL`, selected model and its native catalog entry. No direct credential SQL writes, authentication DB manipulation, broker, service, plugin or key-bearing arguments/environment are introduced.

The same bounded JSONC range editor preserves comments and unknown neighbors. An existing object-shaped `model` is updated only at `providerID` and `model`; its internal comments/unknown fields survive, and conflicting variant/route/auth overlays refuse. Other providers are not rewritten. An unchanged selected config is an exact-byte no-op. Native substitutions `{env:…}` and `{file:…}` in proposed literal values refuse before publication; Qwen's distinct dollar interpolation semantics are not applied by analogy.

## Actual evidence

Evidence root: `/data/git/vibevm-keyring-storage-20261120T0100Z/`.

- `opencode232-config-native-r1/receipt.json`: first direct unmodified official OpenCode 2.3.2 native config-only proof. Exit 0; two synthetic `/v1/chat/completions` requests match model and credential. No provider/key/model arguments, credential environment or manual SQL writes. This proves the fields are consumed, not merely accepted by an open JSON schema.
- `opencode232-public-native-r1/receipt.json`: public keyring create/connection add/native preview/apply/no-op followed by standalone native execution with only prompt arguments. Exit 0, matching model/credential on both requests. Comments, model-object comments and neighbor fields survive the Vibe write. An interpolation-bearing synthetic key refuses with unchanged native bytes. Architect independently accepted the behavior.
- The immutable public/native CLI snapshot at `opencode232-public-native-r1/vibe` has SHA-256 `31fd6e979f290108c0081c8ebf9b175db4145304cc6e13f3731f35bd6dd079d9`; receipt SHA-256 `596e53ef8925eec3ab61a20f99e48188b02cd33047ce8fd7311028faf3cd87ed`.
- `opencode-native-clean-build-r1/vibe_secrets-runtime.log`: complete storage library **24/24 PASS**, no ignored/filtered; existing native adapter regressions remain green.
- `opencode-native-clean-final-build-r2/runtime.log`: complete public AI connections target **18/18 PASS**, no ignored/filtered. New durable case checks model comments, no-op and native file-substitution refusal.
- `opencode-native-public-pty-r1/receipt.json`: Escape, default Keep and explicit Apply **3/3 PASS**, terminal state restored, no provider requests or secret output.

The clean build uses accepted owner projections with only this slice's exports, excluding the stopped preparation still present in the working tree. A reviewed draft defect that replaced the entire model object was corrected before these execution gates: range-level changes now preserve its comments and neighbors.

`opencode-native-clean-strict-r1` completed all targets of the three affected crates with warnings denied, **exit 0**. Exact final source/blob review remains pending. No OpenCode commit or aggregate CORP/NST completion is claimed.

## Remaining boundaries

Existing private Unix native directories only; alternate config roots, fresh-directory genesis, full multi-agent onboarding and native Windows/macOS security/runtime qualification remain separate. Protection/snapshot rechecks plus atomic replacement coordinate Vibe writers, not an independent native editor's final check-to-rename race. Actual runtime proof does not establish arbitrary credential DB coexistence or all selected model overlays. Hermes and Claude native adapters remain next, separate changes. Source support does not imply rebuilt delivery in the local corporate kit. No real account/provider or external publication/push was used.
