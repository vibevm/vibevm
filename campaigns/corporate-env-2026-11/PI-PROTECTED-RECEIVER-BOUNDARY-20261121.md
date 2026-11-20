# Pi protected credential receiver: bounded contract inputs

2026-11-21 research/proposal only after `609f1548fab042dca8ea0e25d20242334a4b3fba`. No key resolution, native/provider execution, auth-store inspection or authorization to ship an adapter follows from this document.

## One exact candidate recipient

Canonical package `org.vibevm.agents/pi@1.5.2`, Linux x86_64 standalone option. Retained official archive SHA-256 `811c6f2c9e7aaee252b5404252e556fb713aa209c1f7f924226aa77d4ef70cb5`; actual regular executable archive member `pi/pi` SHA-256 `6a2ac1c5b0d08b91ce6095591c1960b469d0665c7fc5f32812960a4a17ca0c57`, computed from retained bytes without execution. Selecting harness kind `pi`, a filename, alias, claimed version or standalone archive hash alone does not authorize delivery to a user wrapper. The proposed execution owner must bind the actual installed receipt, selected launcher and held executable identity before resolving any value. Unknown/different executable, symlink substitution and wrapper composition must refuse rather than receive credentials or silently fall back.

The earlier accepted Pi proxy proof used a pinned Node CLI layout with a synthetic key in fixture models.json; it does not prove safe managed-secret delivery to this standalone binary. Retained source snapshot `942842439e1f1d65f68a61857ddb7590b80c5a92`, coding-agent package version1.5.2, supplies inspected semantics; source/artifact relationship and bundled dependencies must remain explicitly bounded, not assumed from a version string.

## Inspected native seams

- `packages/coding-agent/src/cli/args.ts:119-120` parses `--api-key`; `src/main.ts:907` calls runtime key override. Raw credential argv is forbidden for this adapter; this route is rejected even though native supports it.
- `src/core/runtime-credentials.ts` provides a nonpersistent runtime overlay; this SDK API is not itself an admitted CLI injection channel or permission to replace native execution with another framework.
- `src/core/resolve-config-value.ts` resolves configuration values using explicit environment/template syntax, literals or leading `!` commands. The proposed adapter may use only a fixed invocation-scoped environment reference; never a command-valued credential, raw key in models.json or generic template-to-secret substitution.
- `src/core/provider-composer.ts` chooses extension `apiKey` before configured key and can construct `Authorization: Bearer`. Native extensions/configuration therefore participate in the trust boundary. Do not expose a resolved value to arbitrary installed extensions or unknown wrappers by changing a global HOME.
- `packages/ai/src/api/openai-completions.ts` constructs an OpenAI client using `model.baseUrl`, resolved key and optional fetch. Redirect handling for the exact shipped runtime was not established by this bounded read. A client default cannot be assumed to preserve the approved destination.

## Proposed admissible invocation and required proof

An explicit managed binding is necessary but insufficient: retain one immutable connection revision, its endpoint/model/API kind and the referenced current secret revision under the existing coordinated owner. The receiver operation must be scoped to that exact endpoint and trusted executable, never an opaque ID bearer grant. Endpoint change or selected source/launcher drift invalidates the subject before resolution.

A possible native channel is a private invocation-owned model description containing only a fixed environment-variable reference, with the value supplied only to the exact admitted child. This remains a proposal, not implemented permission. It must preserve or explicitly refuse conflicting native model/provider/API-key flags, configured user overrides and unknown standing arguments; default binding must not silently replace an explicitly native invocation. User native auth/config is neither imported nor overwritten. Invocation-only files contain no key; no global environment mutation, argv value, trace/header dump or credential-bearing progress event is allowed.

Redirect protection must be proven before delivery is admitted. A local mock test should witness the intended request and boolean credential match only, then return cross-origin and same-origin redirect controls. The unapproved destination must observe no credential, and an unknown redirect policy must refuse, not silently follow. Do not introduce a general proxy/gateway merely to mask an unsupported native boundary. If a finite safe route is unavailable, retain the missing capability and ask for the exact architecture decision instead of relaxing endpoint authority.

Required synthetic-only controls: exact trusted native positive; selected custom-wrapper/different executable negative before resolution; explicit native override precedence; endpoint/config revision drift; no ambient auth fallback for missing/dangling reference; redirect no-delivery; parent/environment/native-auth preservation; no key/digest in stdout/stderr/receipts/argv/files beyond the sanctioned private keyring. No real provider, corporate endpoint or production credential is involved. Receipt may contain only IDs/nonsecret outcome data and boolean marker-match/leakage results.

## Executed native gate: raw environment channel rejected

An exact native Pi1.5.2 subject ran only inside an unshared filesystem/PID/network sandbox with a generated nonfunctional marker and local loopback SSE/provider/sink. No keyring, actual credential or external provider was present. Three distinct observed outcomes are retained in `pi-recipient-sentinel-r1/receipt.json`: (1) intended loopback native request exited0 and the intended receiver recorded boolean marker-match=true; (2) a307 redirect caused a request to the second loopback destination, but marker-match=false there, and Pi exited1; (3) a controlled explicitly loaded extension could read the marker from process environment and its ordinary child inherited it, while the native request still exited0. Raw header/value/digest was not emitted.

Architect rejected the process-environment channel on these actual results. The keyring remains disconnected. Lack of credential forwarding on this one cross-origin redirect is not strict destination isolation: the unapproved destination was contacted. Extension/child visibility alone already violates the requested recipient boundary. Native/unmanaged behavior was not changed. The separately prepared fixture-only transport-owner prototype was not run, not integrated and not accepted as a product; it is retained as unexecuted preparation, not evidence.

## Existing transport owner inspection, not a new broker

`vibe-net::http::route_in` configures Vibe-owned reqwest clients for route/TLS. `NetworkSession` owns px route/lifetime. The native-agent `AgentRoute::apply` projects route environment into the selected child and `route_child` supervises its lifetime. None owns the native Pi HTTP request or can bind/inject a model Authorization header inside native Pi's TLS session. Proxy credentials and px CONNECT are not a model-token receiver.

`vibe-llm::ChatTransport::post_json(&Endpoint, Option<&ApiKey>, body)` and `ReqwestChatTransport` do own Vibe's separate bounded Chat Completions requests, disable redirects/retries and keep bearer formatting internal. That is a real existing interface, but native Pi does not submit its requests to it. The inspected native launcher invokes the supplier process; replacing it with Vibe's LLM request path or adding a credential broker is not an already implemented managed-Pi interface. No existing bridge was found in these bounded owners. Therefore managed Pi key delivery under the requested exact-recipient/destination/extension-child isolation remains an architectural limitation, not implemented auth and not permission to export a key.

## Separate owner-only distribution result

The unapproved pair is the exact Pi1.5.2 supplier standalone archive (and its actual bundled component/notice inventory) incorporated into a corporate installation drop or hosted/published package target. No independently authorized destination or redistribution closure is established here. The same distinction applies to other held third-party artifacts. Official acquisition/integrity and local runtime tests grant neither hosting nor redistribution rights. Do not perform those outward-facing operations; continue independently authorized local implementation work.
