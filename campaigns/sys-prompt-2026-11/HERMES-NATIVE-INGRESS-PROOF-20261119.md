# Hermes native gateway ingress — provider-free proof

Date2026-11-19. Pinned upstream `4ae451d4126327c871fd084e60c045dec0c85911`. This is verification of upstream runtime seams, not a shipped Vibe implementation or full native parity acceptance.

## Actual successful path

Each run used a new bwrap process/network namespace, ephemeral synthetic HOME, read-only upstream source and private runtime dependency sites. No operator HOME, auth store, profile, sockets or shared gateway were mounted. No model turn was submitted; external network was unavailable during execution.

| Task | Actual result |
|---|---|
| `ba99hqbeb` | Native `GatewayRunner(config=GatewayConfig())` constructed |
| `b0qcik6a6` | Native authority initialized; native loopback API started and stopped |
| `bz8h4o1em` | Native own-process attach URL builder → `connect_gateway()` → successful `session.list` RPC |
| `bia4z5tid` | Native client `session.create` → `session.info`; source `cli`, `lazy=true`, stored ID matches and session exists in native authority |
| `bwu5pcfvq` | Actual synchronous `GatewayConnection.start/call`, actual `GatewayAgent` bound to created CLI session, native info RPC and connection close |
| `b7nwcna6m` | Complete native runtime refused missing credential and synthetic invalid token with HTTP403; authority session set unchanged |

The native supported input is `HERMES_TUI_GATEWAY_URL` in `hermes_cli/gateway_client.py:330–393`. The upstream `_build_gateway_ws_url()` supplied the private synthetic process's own credential, retained only in process memory/environment. No attach URL or ticket was printed or persisted. Authentication was not disabled or patched. These are native client/adapter calls, not a complete interactive CLI invocation.

## Retained failed attempts and environment corrections

- `bexstb2x1`: native listener import lacked `uvicorn`.
- `bmr0siap5`: native client import lacked `websockets`.
- `b6zekosq4`: lazy dashboard import lacked `fastapi`, causing HTTP403. This was NOT an authentication-negative proof.
- `b1m11499d`: a browser ticket was the wrong credential shape for the private loopback listener. Its native refusal was retained; the corrected positive used the upstream own-process URL builder, not relaxed authentication.

Runtime additions were downloaded as binary wheels in credential-free isolated download processes and installed offline to private targets, never globally. Direct upstream pins: uvicorn0.54.0, websockets15.0.1, fastapi0.133.1, starlette1.9.0 and python-multipart0.0.32. Resolved transitive versions and exact wheel SHA256 are recorded in `HERMES-NATIVE-LISTENER-RUNTIME.json`.

Native diagnostics disclosed SQLite DELETE-journal fallback, absent synthetic Nous authentication and missing `snowballstemmer` during session assembly. No real login, provider call, host SQLite upgrade or toolchain change was performed. Native `lazy=true` is an observed info field, not instrumentation proving that no auxiliary constructor ran.

## Remaining obligations

Full interactive CLI invocation, gateway factory attachment of the stable prefix, unrelated-session/worker exclusion, native resources/hooks, fork/resume, fresh USER-PROJECT tail and integrated Vibe launcher remain open. The remote-endpoint branch skips local hook-consent recording; existing allowlist semantics must be preserved and never auto-granted. No global/class/module patch, copied native prompt tail, HOME/SOUL substitution or replacement chat framework is justified by these results.

The designated coding execution lane remains unresolved after a single no-tools timeout; successful native gateway verification is unrelated to that lane's availability. No coding fallback or product edits are inferred.

Evidence: `/data/git/vibevm-native-carrier-20261119/HERMES-NATIVE-CLIENT-INGRESS-RESULT.json` lists actual calls and ten preserved log SHA256 values; `HERMES-CLI-ENDPOINT-SOURCE-REVIEW.json` records the source contract; `HERMES-NATIVE-LISTENER-RUNTIME.json` records wheel fingerprints and runtime boundaries. No credentials are included in this report. No external push or publication.
