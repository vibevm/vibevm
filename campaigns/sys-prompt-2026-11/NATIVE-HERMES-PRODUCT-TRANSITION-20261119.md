# Hermes executable product transition — Linux native owner

2026-11-19; base `b17056def`. Intermediate executable transition under approved NATIVE-PRODUCT-IMPLEMENTATION-04, not full SYS-HERMES-PI acceptance.

## Product ownership

Rust discovers the selected launcher through existing `program::locate`, not Unix `resolve` (which intentionally leaves bare names for exec). Runtime admission requires an absolute installed Python shebang and exact measured module hashes for native turn construction, session execution, prompt assembly, API, authority and CLI. Placeholder package version0.0.0 is not treated as compatibility proof. Shell wrappers/Windows launchers remain unadmitted, explicitly not platform parity.

Two Python assets are embedded/published using existing immutable no-follow native asset owner. Supervisor runs with `-I`, inserts only the selected admitted installation, and sets cwd inside its own process to the selected project. No ordinary Vibe Python requirement, new resolver, project-authored script execution, native module copying/editing or global class patch.

Supervisor uses native parser, creation policy, ProfileOwnership reservation, GatewayRunner, session authority and native listener. It preallocates an exact CLI session before attachment and launches normal native CLI with that ID. Existing owned profile/external endpoint refuses; never stop/replace another gateway. Endpoint credential is transient child environment only; no receipt/log contains it. Existing native worker environment policy remains the owner of stripping it.

Carrier shadows only owned runner and agent instances; original native prompt assembler remains authoritative. Every owned turn validates exact authority/session/source/route and cache access uses native lock. Same cache key with another session refuses. Foreign/double shadows refuse. Teardown checks owned shadow identity without requiring that an ended native session still exists.

Child termination is bounded terminate/wait then kill/reap. Signals are forwarded to the owned child; negative signal status is mapped to shell status. Cleanup attempts carrier, listener and native `runner.stop()` independently through nested finally blocks, with bounds; profile release is last. Cleanup timeout remains failure, not a clean-settlement claim. Error diagnostics expose fixed categories only, not arbitrary native exception strings/transport secrets.

## Actual proof

- `cargo check -p vibe-cli` and production `cargo build -p vibe-cli --bin vibe` pass. Final gate `brzf0ihaw` exit0: fmt, native asset tests3PASS, cli_agent_launch42PASS/2ignored, conform0new (141 existing). Private `control/hermes-product-final-gates.log`. Exact5 product-source and log hashes recorded in `HERMES-PRODUCT-TRANSITION-RESULT.json`.
- Normal-exit/cancellation-stage binary SHA256 was observed as `75c8bc81e821313d82dd3b4c7d9576f2d21f0379b92af73373a5b88ac9fafb08`. After the subsequent embedding/launcher gate the current binary and private receipt SHA256 are `d648255cb2c903ef173f6ea62c590acfe8b02fe6f8f2865ff8a973e5edab8453`. These are not claimed to be the same executable; the earlier normal/cancel logs did not independently freeze binary hashes within their drivers. Exact unchanged product-source identities and separately recorded build/gate results remain the boundary, not an invented single-binary replay claim.
- Actual rebuilt `/vibe agent hermes` in synthetic project/install/HOME enters real native CLI and exits after `/exit`, code0/no timeout/no native refusal. Task `booukd4st`, private `control/hermes-vibe-dispatch-locate.log`.
- Actual same Vibe dispatch SIGTERM arm `bzbmahiws` exits0/no timeout; native CLI handles signal gracefully. This is no-active-turn cancellation, not forced-kill/active-provider settlement proof. Private `control/hermes-vibe-dispatch-cancel.log`.
- Current product module against native runner/authority/real agent constructor and original assembler: `bki4iwdw6` exit0, full310KB prefix before caller tail, native same-agent cache reuse, unrelated session excluded, same-key/different-ID refusal, class unchanged. Private `control/hermes-product-module-proof-final.log`. This is direct native turn construction, NOT a CLI-submitted model turn.
- Runtime Hermes source pin4ae451d4126327c871fd084e60c045dec0c85911. All execution network-isolated unshare-all bwrap, fresh synthetic HOME, readonly source/private runtime; native loopback listener only. No real provider request/key or host auth copied.

Assets: hermes_bootstrap.py SHA256 `dd8ea227cc48534e16a5c17d454eab24930c92fe3d6566cc1fe98eec2c3da9e2`; hermes_entry.py SHA256 `56d7a96b335bf01aa296e87e4d1721cd0946824689c7114a547a27cb8e9cac34`; hermes_runtime.rs SHA256 `9ad2d06409bdc8b481fa39de7cc9f2795b7d7decb1c2d003597d303b56ac91fe`.

## Rework / failures retained

Prior preallocation proof timed out; not reused as normal-exit proof. First product module assertion expected old diagnostic wording despite correct refusal; retained RED. Actual Vibe fixture initially lacked AGENTS.md, then had an ENOENT because runtime discovery used Unix bare-name resolve; fixed via existing locate owner and rerun. Isolated interpreter fixture initially hid required system dependency directory; corrected sandbox-only site layout, no host installation/config modification. No failed run retroactively counted PASS.

Native runtime warns about missing truststore/snowballstemmer and SQLite journal fallback; full resources/TLS/tool-search are not proven by this minimal runtime.

## Subsequent submitted-turn evidence (not a completed turn gate)

Actual Vibe CLI user input reached the isolated synthetic HTTP provider with both prepared prefix and user message. First fixture replied JSON to streaming requests, causing native empty-stream retries and a timeout; retained RED. Corrected SSE fixture `bjg4l0vml` captured3 requests with prefix/user turn and observed the synthetic response, but the driver still timed out after sending `/exit`; early exit-input timing is a hypothesis, not a verified cause. Therefore the combined turn/normal-exit gate is NOT PASS. Logs `control/hermes-vibe-submitted-turn.log` and `control/hermes-vibe-submitted-turn-sse.log` preserve those outcomes; the latter has no raw transport credential dump. The previously passed no-turn normal-exit/cancellation gates remain separate. No real provider was contacted.

## Completed submitted-turn follow-up

Task `btn16vj95` exit0: actual Vibe → Hermes CLI user input → isolated synthetic SSE provider response → ready native prompt → `/exit` code0/no timeout. Driver freezes binary SHA256 `d648255cb2c903ef173f6ea62c590acfe8b02fe6f8f2865ff8a973e5edab8453` internally. Main streaming request has exactly one system message and one prepared-prefix occurrence, with user turn present and native system content retained. Two non-streaming requests have a different shape and are not counted as main turns or independent successes. Scope is the small synthetic project canary; earlier direct native constructor proof supplies the separate310KB placement evidence, not a full-size actual CLI request claim.

Private `control/hermes-vibe-main-request-payload.log` records per-request nonsecret cardinalities. Prior payload oracle mistakenly required the prefix on every request containing user text, including a non-streaming auxiliary request; its assertion failure and timing failure remain RED in `hermes-vibe-submitted-turn-payload.log`. Readiness-specific earlier `bq3nwphx9` also completed normally; the corrected oracle's terminal success does not erase previous failures. No product source changed for these driver corrections.

## Full-size actual request follow-up

Task `bm69tueaw` exit0 repeats the same native request oracle with exact310024-byte source core (canary plus310000 filler bytes). The complete source text is present unchanged in the main streaming system message; one prefix occurrence, native system content retained, user turn present, synthetic SSE response and `/exit` code0/no timeout. Binary identity is frozen inside the driver as `d648255cb2c903ef173f6ea62c590acfe8b02fe6f8f2865ff8a973e5edab8453`. Private `control/hermes-vibe-fullsize-request.log`; no product source changes. This closes full-size actual CLI delivery for this synthetic subject, not personal-tail freshness or cache-hit measurements.

## Remaining finite obligations

Fresh personal ordering; workers and unrelated project/quiet exclusions; existing-history/compaction admission; new/resume/fork/source-policy families and full native argv; shared-gateway admission path; Windows/macOS/native wrapper layouts; active-turn cleanup. Fresh interactive chat is the implemented admitted entrypoint; explicit existing selectors/worktree/oneshot refuse pending native policy mapping, not final parity. Pi remaining resume/fork/lifecycle/worker requirements remain open. Replacing Pi system-prompt is a carrier-conflict candidate disposition, not an obligation to replace native identity; read delivery retains native flags.

No cache-hit/cost savings measured, no final Hermes owner blog yet, no SYS/CORP/NST completion or external push/release.
