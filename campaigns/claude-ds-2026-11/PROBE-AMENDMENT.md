# DS-PROBE-01 — explicit minimal service and model probes

Date: 2026-11-16. Narrow amendment to DS-PACKAGE-01, not whole-package acceptance.
Builder relays owner request for direct lightweight API checks before full harness
diagnostics. Builder owns the scoped probe implementation/security review and live
execution under that explicit instruction. Architect does not duplicate live calls.
No generic direct-coding exception, automatic polling/retry, host installation,
credential change or publication follows.

## Exact input and write perimeter

Contract baseline: 320d2665cdddc80fbb6ae65710e7a72806cb2b1c plus kind-app correction
 d7c2f5d0ed88a05e6a6046efc58d37b7232a8898. Bind this amendment's exact delivery commit.
Unaccepted public scaffold:
`/fast/tmp/ds-package-scaffold-hurgz4rp/source/vibevm/vibepacks/org.vibevm.world/claude-ds/v1.0.12`.
Input manifest `/fast/tmp/claude-ds-probe-input-manifest.json`, SHA-256
`fc006b48ad8e0080dff67f72556d483dc3c927b4bf82bbe4296ad33e7c3d133b`.
Architect verified manifest hash and ten listed regular-file content hashes.
This is precise probe rework input, not acceptance/promotion of existing config,
copy_payload, launcher, runtime or packaging security. No private wrapper input.

Only package-relative paths may change:
- `src/claude_ds_probe.py` (new)
- `tests/test_probe.py` (new)
- `src/claude_ds_cli.py` (explicit mode dispatch only)
- `tooling/copy_payload.py` (add probe to existing payload allowlist only)
- `README.md` (generic probe usage and limitations)

Config reader `src/claude_ds_config.py` remains unchanged. Any required config/schema,
credential-reader or enforcement repair outside these five paths stops for precise
scope review, not reuse of an unsafe existing seam. Other candidate files remain
unchanged; preserve original scaffold and bind output hashes. No new dependency.

## Protocol and safety

Official primary documentation actually fetched by architect with web.run in the
amendment-authoring turn:
- `https://docs.litellm.ai/docs/proxy/health`: GET /health/liveliness requires no auth,
  makes no LLM call, and proves process liveness, not model/provider readiness.
- `https://docs.litellm.ai/docs/anthropic_unified`: Anthropic Messages endpoint.

Documentation does not prove the private deployment's version or health. No real
endpoint/model/token value enters source, fixtures, reports or this contract.

1. Explicit mutually exclusive `--probe-service` and `--probe-model`; neither starts
   the harness. Both require explicit private config. Existing --check-config stays
   offline, without credential-content read or network. Help/version stay config-free.
2. Service: one HTTPS GET to the admitted origin's /health/liveliness, NO auth read
   or header. Unsupported reverse-proxy path layout refuses explicitly; do not
   silently discard a configured prefix or scan alternate routes.
3. Model: one explicitly requested selected-route POST, synthetic user `ping`,
   max_tokens 16, streaming false, no task/tools/project content. Derive /v1/messages
   from the admitted protocol base without doubled /v1 or lost prefix. Use only
   the designated own-service credential, x-api-key for the documented
   anthropic_unified curl route, Content-Type JSON and anthropic-version 2023-06-01.
   Do not send an Authorization header as a second guess. No
   auth-header guessing/dual-header fallback or other auth-store discovery.
4. Read credential only in model mode through an existing demonstrably safe bounded
   no-follow seam; reject malformed/header-injection content. No key in argv/env
   dumps/diagnostics. Inspect neither real secrets nor private launcher contents to
   test exclusion. Synthetic sentinel tests prove both output channels stay clean.
5. HTTPS/TLS validation required. Reject URL userinfo, malformed origins/fragments,
   redirects, environment proxies, endpoint fallback, SDK retries and automatic
   route enumeration. One overall 15-second deadline includes DNS/connect/read;
   per-read socket timeout is not a hard overall deadline. Bounded response 64 KiB;
   oversize refuses. No surviving probe child/process after deadline/cancellation.
6. Closed sanitized result: fixed status enum, nullable HTTP integer, elapsed time;
   stable success/failure exit. Never print/store response body/headers, raw exception,
   URL/model/config/credential path or environment. HTTP200 model success requires
   bounded valid Messages response, not an error envelope/HTML or arbitrary JSON.
   Service success uses the documented liveness response. No generated reply output.
7. Probe success does not establish full harness/tool availability, actual model
   identity, absence of gateway fallback or health of every provider. Client sends
   only the selected route; no broad /health or all-model scan. No server config
   changes to disable its internal retries/fallbacks are authorized.

## Gates and delivery

Synthetic tests: auth never read for service; model uses only designated synthetic
credential; absent/invalid config refuses before effects; safe URL composition and
prefix behavior; TLS/redirect/proxy rejection; auth failure/HTTP500/timeout/DNS stall/
slow-drip/oversize; HTTP200 error versus valid Messages; both output channels and
exception paths exclude sentinel credential/config/body/header values. Preserve
existing --check-config no-auth/no-network and help/version behavior. Verify probe
is included in payload without widening copy allowlist or executing at install time.

Live verification only under the relayed owner's explicit scoped instruction;
finite calls, sanitized output, no duplicated architect call. Report actual result,
unknowns and unrun gates. Do not let probe execution interrupt the independent
artifact worker or infer acceptance from shell-wrapper exit. Return frozen exact
candidate, five-path diff, test commands/counts and limitations for review. Package
Part A/runtime/installer and observer acceptance remain separate and incomplete.
