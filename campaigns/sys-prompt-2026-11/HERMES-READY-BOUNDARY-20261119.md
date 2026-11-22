# READY-owner contribution: structural boundary disposition

2026-11-19. Source-bound inspection of Hermes pin `4ae451d4126327c871fd084e60c045dec0c85911`; no native execution, provider call or installed-runtime patch in this investigation.

## Confirmed construction and information loss

- `agent/system_prompt.py::build_system_prompt_parts` (770–833) constructs ordered `stable`, `context`, `volatile` strings. The caller's `system_message` is the first context member (807–808), before context files/workspace material. `build_system_prompt` (836–846) retains the stable string on the agent and joins the tiers into one string. This is the structural boundary the existing owned carrier uses by passing Vibe bytes before its caller tail; it is not a textual marker convention.
- `agent/turn_api_request.py::build_request_payload` (roughly 145–178) invokes request middleware after native provider shaping/cache decoration/preflight. Middleware receives request kwargs and task/turn/session/model/provider/mode identity, but not the agent's witnessed stable/context tier lengths or original caller-tail offset.
- `agent/prompt_caching.py::_apply_system_cache_markers` (310–346) can produce one, two or three text blocks. A first marked block is NOT sufficient proof of a stable/context split: fallback marks the entire system message when a valid split is unavailable. `apply_anthropic_cache_control` (498–535) invokes that helper and also strips/rebuilds prior decoration. Consequently, recognizing a first text block or a cache marker in the final provider payload alone does not certify the desired insertion boundary.

## Disposition

The confirmed plain-text chat-completions system message has no exposed structural stable/context offset at the request-middleware boundary. The inspected decorated shapes cannot be admitted merely from their block count/first cache marker. No source-supported boundary witness has been established for a production READY plugin through the inspected public callback contract. No text search, synthetic marker trick, generic prepend or append is admitted as a substitute.

This is a separate cache-stable placement debt, not the same property as OPEN `HERMES-REQUIRED-PROMPT` strict-delivery debt. Best effort permits fail-open delivery; it does not permit moving a mutable native tail ahead of the large Vibe contribution or changing instruction priority. Required positive proof remains `native stable → Vibe project → Vibe personal → native mutable tail`, unchanged leading bytes when only that tail changes, plus missing/duplicate controls.

## Candidate state, not shipped behavior

Untracked `hermes_ready_plugin.py`, `hermes_ready_session.py`, `hermes_ready_session.rs` and `test_hermes_ready_plugin.py` are incomplete implementation candidates. They are NOT wired into `native.rs`, launcher dispatch or a shipped plugin package. Ten offline plugin-unit tests and two Python syntax checks pass; these tests establish only the candidate's narrower map/RPC behavior and currently assert an append layout which is **not accepted cache-stable behavior**. They are not native/provider/transport/parity proof. Independent review found native RPC routing params passed through unchanged; that handler defect was corrected with a transport-shaped test. The ordering finding remains unresolved; do not integrate the append candidate.

The non-owning transport draft retains exact native resume, native connected `GatewayChatView`, explicit conversation-scope consent and safe diagnostics. Its Rust module is not wired, compiled or tested; referenced READY module admission is not implemented. Native view commands changing the selected conversation are deliberately refused by the draft; ordinary history is not replaced. Completing/testing transport separately must not be represented as instruction-delivery acceptance.

Existing owned behavior is unchanged. No source release, SYS/NST closure, strict support, full worker/compaction coverage, cache hit or savings claim follows. Architect acceptance and a real structural witness are required before any READY cache-stable delivery wiring.
