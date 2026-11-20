# Per-agent connection metadata — missing-route qualification

Baseline accepted selection `afa25ba2a`; finite qualification contract `d2bdda5c2`. Test-only, no native launch, credential access, provider request or product change.

## Real identity and capability owners

`vibe-workspace/src/system_prompt/harness.rs::HarnessKind::{ALL,parse,as_str}` is the canonical six-identity set: claude, codex, opencode, qwen, hermes, pi. `vibe-cli/src/commands/agent/launchers.rs::{resolve_all,resolve}` maps a user-selected launcher/alias to that kind and preserves its executable/arguments; a launcher name is not a new protocol identity. The core manifest harness table is closed delivery configuration, not provider-protocol admission.

There is no existing production connection-to-harness protocol capability owner in the inspected connection metadata/agent launch seams. `vibe-connections::Metadata` currently admits only the string openai-chat-completions as connection metadata; this does not demonstrate any harness uses it. Accepted proxy receipts remain narrowly different:

- Pi1.5.2 and Qwen0.27.2 use their native deterministic local OpenAI-compatible SSE paths in the recorded Linux proxy fixtures.
- Hermes old4ae query uses synthetic OpenAI chat/SSE under its admitted owned route; this is not the newer packaged Hermes's complete auth adapter.
- Codex0.170.0 fixture is native Responses API, not proof of Chat Completions support.
- Claude2.1.328 fixture is Anthropic Messages; its receipt explicitly excludes OpenAI compatibility/setup.
- OpenCode2.2.7 fixture uses its O2-configured synthetic provider and native transport, not a generic per-agent metadata binding or proof for every supplied package/version/model.

These records qualify their exact native/proxy subjects, not a new adapter declaration. No live requests were repeated here and no account/profile files were read.

## First missing step

The existing AI reference index has schema/items and optional explicit default selection; no per-agent mode map exists. `cli/ai.rs::ConnectionCommand` exposes list/details/add/remove/default only. `vibe-connections` owns connection/default metadata but no harness binding methods, and `commands/agent/launchers.rs` does not consume it. Therefore there is no public setter/reader for native/unmanaged, inherit-default or explicit-connection modes to qualify.

The focused public fixture creates/selects a synthetic default, asserts the index has no agents/bindings field, then attempts the proposed-looking bindings show pi route. Actual observation is usage2/unrecognized subcommand with both indexes unchanged and no agents.toml creation. `harness-binding-qualification-test-r1.log` records one PASS; scoped strict Clippy, format and diff checks passed. The final manifest binds exact source/binary. This tests the first missing public route, not runtime behavior of a launched harness.

Default-switch inheritance, explicit binding stability, absent-default refusal, unavailable explicit target, alias-to-binding lookup and bound-connection removal are NOT REACHED. A default alone is not relabeled automatic harness opt-in. Old strict AI index readers/writers currently reject unknown fields and unsupported epochs; adding modes needs a deliberate versioned index/capability decision before writer activation, not a sidecar or ignored field. Metadata-binding qualification cannot authorize native credential delivery.

Next implementation needs one trusted adapter capability registry keyed by canonical harness identity and actual admitted protocol/version scope, a strict persisted three-mode shape and exact revision/connection-reference rules in the existing atomic owner. This receipt requests that finite architecture, not permission to infer protocol support, mutate native credentials or add a generic plaintext resolver. AR and IDE remain untouched.
