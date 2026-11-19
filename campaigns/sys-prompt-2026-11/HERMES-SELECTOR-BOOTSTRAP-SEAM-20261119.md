# Hermes selectors — native unattended bootstrap seam

2026-11-19; working candidate after accepted skills `cda62ffbbaeea51f1402622d9d2c332a79b87005`. Not acceptance or completion. Native source pin `4ae451d4126327c871fd084e60c045dec0c85911`.

## Observed product result

Private `control/hermes-selectors-oneshot.log`: actual rebuilt Vibe oneshot reached selected project preparation, then native frontend refused `invalid_params`; no positive provider-turn result. Binary `1f4e6cb0443fd6a783ccd86916b4dfa05ae6900efef099603d0c1d0506794474`. First Rust check failed module visibility; retry compiled. Dirty selector code remains an unaccepted candidate. No raw credential, private URL or authority identity is printed.

## Exact native owners

- `hermes_cli/main.py::_run_oneshot_from_args(args)` delegates to `gateway_chat_startup.launch_gateway_chat(args)`, then `gateway_chat.launch_from_args(args)` and `run_gateway_chat(args, emitter=None)`.
- `gateway_chat_view.py::GatewayChatView.submit(text)` sends `finite=True, unattended=True` for native `--oneshot`.
- `gateway/session_finite.py::admit_finite(params, *, submitter=None)` / `_may_run_unattended(connection, ref)` require transport-verified `native_owner`, same creator principal and CLI platform. Existing `_build_gateway_ws_url()` process/internal credential is not the native bootstrap ticket and fails this guard. Do not forge `native_bootstrap`, weaken finite guards or change the requested approval mode.
- `gateway/run_runtime.py::initialize_gateway_runtime(runner)` requires `runtime_ownership.process_ownership`, creates SessionAuthorities, runtime descriptor and TicketStore.
- `gateway/run_bootstrap.py::_start_gateway_start_control_socket(runner)` starts the owner-only peer-checked control endpoint, including native `session-ticket` and descriptor/resolve handlers.
- `gateway/run_runtime.py::start_gateway_runtime_api(runner)` starts the API and records api_origin; `publish_gateway_runtime_ready(runner)` requires a live API and `runner._running`, set by native `GatewayRunner.start()`.
- Native `gateway_client.connect_gateway(endpoint=None, subject=None, *, max_size=...)` on its ordinary discovery path requests a private control ticket and calls `gateway_ws_target(endpoint, ticket)` with ticket subprotocol. Explicit `HERMES_TUI_GATEWAY_URL` does not carry that native subprotocol. Preserve the ordinary owner path rather than introducing an auth transport.

## Lifecycle gap and minimal owner seam

The current carrier owns a minimal `initialize_session_authority` + API runner, not full gateway startup. Native `GatewayRunner.start()` is explicitly all-configured-platform startup (`run_startup.py`): adapter connections, plugin/relay registration, recovery and resume scheduling, service wiring, notifications and background watchers. No-enabled-platform configuration keeps a gateway running, but does not expose a terminal-only startup parameter and still runs recovery/services/watchers. Changing installed operator config, clearing global platform config, fake `_running` or starting unrelated configured adapters is not permitted here. Search found no production embedded-terminal initializer that performs native ready publication independently; upstream tests use the full start sequence or fabricated fixture state (not a production remedy).

A sanctioned upstream owner seam would separate terminal runtime readiness from messaging startup: retain exact profile reservation, initialize owned authorities/store references/TicketStore, start peer-checked control and API, validate ready without claiming configured adapters started, expose the normal descriptor/ticket discovery to the native frontend, and withdraw tickets/control/API before authority/store/reservation teardown. No second runtime, lock gap, unrelated platform/recovery submissions or fabricated readiness. This requires an upstream-supported change or a separately reviewed admissible existing entry point; current Vibe glue must not copy run_startup or patch native state to simulate it.

Without that seam, resume/continue/worktree finite query can be exercised separately using native non-unattended query behavior, but the failed unattended oneshot subcase remains open. Native `session_worktree.create()/restore(session_id)/finish(info, session_id, *, ask)` is the supported worktree lifecycle for external chat launchers and supersedes the earlier classic-only handoff hypothesis. Actual worktree and historical resume proofs have not yet passed in this candidate. Shared external gateway, remaining parser/frontend parity and platforms remain separate.
