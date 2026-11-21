# Codex 0.171.0 native Responses settings

## Explicit native protocol, not translation

The connection metadata owner admits the explicit API kind `openai-responses` alongside the existing `openai-chat-completions`. This does not add Responses to Vibe's ChatTransport. Codex's fixed native sink requires Responses; Pi/Qwen require Chat Completions. An incompatible selected connection refuses before native publication. Native `vibe ai setup --native-settings --agent codex --connection ID` uses the incumbent preview/confirmation/result route, safe default Keep, explicit operation token and fixed `HOME/.codex/config.toml` destination. An explicit `CODEX_HOME` override is refused rather than ignored.

Pinned native Codex 0.171.0 source explicitly rejects the historical `wire_api = "chat"`. The concrete sink uses native custom provider `vibevm`, `wire_api = "responses"`, native persistent `experimental_bearer_token` and `requires_openai_auth = false`. `toml_edit` is the already-pinned workspace dependency, preserving comments and unrelated providers. Existing selected-provider command/env/gateway/AWS/header/query authentication sources, including `env_http_headers`, refuse instead of creating ambiguous authorization. Native `auth.json`/OAuth is never overwritten. No key in launcher arguments/environment, broker, service, plugin, fork or protocol translator is introduced.

## Existing managed metadata remains protocol-safe

Default-scope, explicit binding and metadata setup validate the actual proposed connection API. Pi inheriting a default and Qwen with an explicit override cannot silently become attached to an incompatible Responses connection. The existing reference storage owner rechecks compatibility under its inventory/store locks before default, single binding, bulk binding and atomic default-scope publication. Missing connection IDs retain their original NotFound behavior; malformed metadata for an existing selected ID still refuses. An inherited binding with no selected default remains valid but unresolved, not falsely configured.

## Actual gates

Evidence root: `/data/git/vibevm-keyring-storage-20261120T0100Z/`.

- `codex0171-public-native-r1/receipt.json`: first public preview/apply/no-op followed by direct unmodified official Codex 0.171.0 without Vibe or credential environment. One actual synthetic `POST /v1/responses` matched model and credential; chat refusal preserved native settings, comments, neighbor provider and OAuth file. Architect independently confirmed it. Later source changes are not retroactively covered by r1.
- `codex-native-clean-gates-r4/`: offline clean-owner compilation and actual isolated complete secrets library **23/23 PASS**, connection native-settings tests **2/2 PASS**. Includes locked default/inherit/explicit incompatibility controls and selected-provider env-header refusal.
- `codex-native-clean-public-final-build-r4/runtime.log`: complete public AI connections target **17/17 PASS, no ignored/filtered**, including original inheritance behavior and added Codex preview/apply/no-op/chat-refusal regression.
- `codex-protocol-public-r1/receipt.json`: eight actual public incompatible API refusals leave both metadata indexes unchanged; inherited Pi and explicit Qwen selection survive.
- `codex0171-public-native-r2/receipt.json`: final concrete native request plus public `env_http_headers` refusal; exact path/model/credential verified, no credential output/environment, adjacent native settings/OAuth preserved.
- `codex-native-public-pty-r1/receipt.json`: Escape, default Keep and explicit Apply, **3/3**, terminal state restored, no provider calls or secret output.

The latter public/native/PTY controls use the immutable CLI retained at `codex-public-controls-r3/vibe`, SHA-256 `b694767bdf1d53f5c782b6c3eccd7ad2782397eda5369442c03e913e9c0de5a6`.

Retained REDs: an old library fixture used non-TOML `b"metadata"` and refused before its intended publication-failure injection; corrected the fixture to valid synthetic chat metadata without weakening Publication/index-preservation assertions. The next run exposed validation ordering: missing ID returned RefusedUnknown instead of NotFound; existence validation now precedes compatibility under the same lock, original expectation retained. Full public target then exposed an accidental refusal of inherit-default before default selection; restored valid unresolved behavior without changing the original regression. Final reruns above pass.

`codex-native-clean-strict-r1` completed strict all-target lint for all three affected crates with warnings denied, **exit 0**. Independent final source/blob acceptance and commit remain pending.

## Remaining and limits

Existing protected Unix native directories only. Fresh-directory genesis, full multi-agent onboarding and inspect/capability UI, Windows ACL/native qualification and remaining agent adapters are separate unfinished criteria. Atomic replacement is not a compare-and-swap against an independent native editor; Vibe's lock coordinates Vibe writers only. Source support is not delivery in the local corporate kit until native Vibe bundles are rebuilt. No real provider, private corporate inputs, external push or publication occurred.
