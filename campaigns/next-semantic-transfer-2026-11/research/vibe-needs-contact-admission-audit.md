# VN-CONTACT-ADMISSION: observed contact inventory

2026-11-07, read-only source inspection. No execution, tests, network or edits to source. Hosting-report writer's files were not changed. Need2 stays accepted. Need1 source choice and Need3 exclusion remain separate from unresolved transitive pins.

## Observed paths (all relative to repository)

| Entry/owner | Actual call or boundary | Admission consequence |
|---|---|---|
| `vibe-package-source/src/builder.rs:183` | `MultiRegistryResolver::open(...)` precedes `.with_offline/.with_progress` | A new `.with_policy` after open is too late. Immutable operation policy must reach construction. |
| `vibe-registry/src/multi_registry_resolver/mod.rs:352,379,399,407` | Build Locator clone chain; open per-package registry; resolve independent index URL; call IndexClient::probe | Canonical registry filtering alone misses index env overrides and host rewrite. Index probe is a managed contact before normal solve. |
| `index_client/locate.rs:88,108,131,151` | Env > manifest > default; env bypasses Locator; GitHub org default becomes `raw.githubusercontent.com/<owner>/index/<ref>` | Exclusion must govern actual derived host regardless of source-policy precedence. Two-host report classifier is not full exclusion authority. |
| `index_client/mod.rs:179` / `handshake.rs:47` | GET hello at base/v1/index and base; then GET repomd at both | Gate each concrete request and HTTP redirect before send. Constructor's HTTP failures currently become Absent. |
| `index_client/mod.rs:468` / `pooled.rs:24,38` | Build probe/file/server reqwest clients with timeout/user-agent/auth; no explicit admission or redirect callback | All three clients need the same immutable policy. Pool key/context must not mix policy generations. |
| `index_client/by_name.rs:74,123`; `catalog.rs:117`; `mod.rs` lookup_purl/search | Direct `client.get(...).send()` | Policy-aware base is insufficient if derived URLs/hops escape it. Catalog creates TransferRecord **after** response, so progress is too late as gate. |
| `handshake.rs:156,172` | World path appended to file base; successor reported, not followed | Recheck concrete file requests; do not invent a successor fetch path where none exists. |
| `git_package_registry/lookup.rs:88` | try_lookup calls primary first, then mirrors; all failures return primary error | Skip excluded attempts before callback; keep allowed mirrors. Denial cannot become “primary not found”. |
| `lookup.rs:31` / `git_backend/shell.rs:383` | Manifest fetch → raw HTTPS fast path before Git preflight → remote archive → caller clone fallback | Gate raw rewritten destination, archive and clone separately; one outer Git URL check misses fast path. |
| `git_backend/shell/raw_http.rs:84,158,303` | Address derivation → bounded retry loop → request.send; client has no explicit admission/redirect gate | Policy denial must not retry or become None (which silently enables Git fallback). Retry checks remain on same bound policy. |
| `multi_registry_resolver/redirect_follow.rs:49,103,236` | Stub marker read; open external target; target marker probe before manifest; target clone acquisition | Hop-limit check itself contacts target. Gate before target marker read. Registry marker probe builds canonical package URL even when prior lookup used a mirror. |
| `multi_registry_resolver/sources.rs:16,37` | Locator.choose_url → direct git-source single-package registry | Existing chosen URL remains route authority; admission follows choice and covers eventual HTTP/Git rewrite. |
| `override_resolve.rs:74`; `dispatch.rs:223`; `refresh.rs:80` | Direct backend manifest read / ensure_clone_at from override URL | Overrides bypass ordinary registry walk; they must not bypass exclusion. |
| `git_package_registry/fetch.rs:54,251` | primary/mirror clone/refresh loop and content-hash-gated fetch | Keep integrity checks. Skip denied route without cache publication; preserve allowed mirror and actual serving-route evidence. |
| `home_refresh.rs:41`; `refresh.rs:24` | Enumerate cached packages/list versions; refresh registry/override clones | Sync/home refresh are other consumers of same bound transport, not an install-only flag feature. |
| `git_backend/shell.rs:200,228,356`; `shell/reported.rs:51` | Command::output for normal/raw Git; ls-remote; independent Command::spawn for clone/fetch | Admission must cover all three subprocess launch paths. Gating only run() misses reported transfers. |
| `shell/checkout.rs:12,76,157` | clone --recurse-submodules; fetch origin; `submodule update --init --recursive` | Top-level URL is not complete managed contact set. Nested submodule URLs need admission or explicit route refusal before recursion. |
| `shell/prepared.rs:26,38,61` | effective `remote get-url origin` versus recorded config origin; bare preparation/fetch; empty hook dir for preparation | Existing recorded/effective distinction is useful; neither one initial query nor preparation's hook restriction proves all Git routing fixed. |
| `shell/env.rs:39` | Locale, prompts, credential-helper suppression, Windows path flags only | Does not itself suppress insteadOf/include config, SSH alias/ProxyCommand, custom remote helpers, HTTP redirects/proxies or recursive submodule routes. |
| `shell.rs:506` | anonymized_for_public reconstructs ShellGit, shares raw HTTP client | Future policy must be retained by anonymous clone and its shared pool; never fall back to an ungated backend. |
| `vibe-core/src/remap/active.rs:26` | Process-global OnceLock Locator, default empty | Preserve routing owner, but do not add another mutable global policy. Operation-bound clients/backends are the admission authority. |

Table `vibe-registry/...` paths are under `crates/`; index_client/git_backend/git_package_registry/multi_registry_resolver rows use `crates/vibe-registry/src/`. Line references identify inspected code, not runtime coverage.

## Concrete counterexamples and diagnostic requirements

1. Registry GitVerse, `VIBEVM_INDEX_URL_*` points at GitHub: filtering registry origins still probes forbidden host before solve.
2. GitHub origin → raw.githubusercontent.com default index/raw manifest read: exclusion “github” cannot be implemented as the report classifier's exact github.com label only. Its managed host-family mapping must include this **observed** backend route, without guessing unrelated corporate domains.
3. Allowed URL → HTTP 302 to excluded destination: checking before initial send is insufficient; gate every redirect attempt before connect.
4. Excluded canonical origin + allowed mirror: deleting the registry loses a valid resolution. Existing primary-first metadata/fetch and canonical redirect-marker paths must become admitted-route aware. A denied optional index can skip to permitted Git/mirror metadata; it must remain reported as excluded/not-probed, not absent.
5. Allowed Git URL + insteadOf/SSH alias/helper/proxy changes effective destination; allowed parent + excluded recursive submodule. Current environment setup supplies no exclusion guarantee. Observed hostname label is not authority.
6. A redirect target is excluded: its marker must not be fetched just to discover that it is another redirect.
7. Policy error folded into raw-HTTP None or index Absent: lower layer then tries another forbidden path. Preserve typed policy refusal across fallbacks. A **different admitted** mirror/registry may still be tried; do not turn route refusal into unconditional whole-operation halt.
8. All eligible routes unavailable: return requirement + policy-constrained-unsatisfied and known excluded routes. Without contact do not assert that excluded host contains the desired package or is its only supplier.

Diagnostics need distinct observed absence/unreachable, excluded-before-contact, opaque-route-unsupported and not-probed. Keep network/auth/integrity failure law distinct. No secrets in errors; classify before credential injection where possible, but the transport gate must validate the actual attempted destination too.

## Minimal implementation vertical supported by this inventory

**One immutable operation binding**, constructed before source open; thread it into existing IndexClient, ShellGit and resolver instances. No new source resolver and no new process-global policy. Locator continues producing routes; admission checks the concrete route and each managed transport hop. No public exclusion option until all paths in its advertised scope use this binding.

1. Add the typed admission/error contract with live constructors as consumers. Existing default constructors preserve no-policy behaviour; policy-bearing construction must never silently choose an ungated transport. Preserve policy in anonymous backend clones, prepared acquisition and both index pools. Separate denied route from absent package at Index/Registry/Git error seams.
2. Complete managed HTTP gate: index probe and all file/server GETs, raw HTTP rewritten requests, retries and redirects. Use one bound request/admission helper per existing HTTP owner (or a shared existing transport seam if the next implementation packet identifies one), not a generic orchestration layer. A redirect policy alone does not gate the initial request. Preserve auth scoping and classify failed policy as policy, not transport noise.
3. Complete Git gate: before ls-remote/archive/clone/fetch and recursive submodule invocation, the command must execute an admitted, stable transport posture. Ambient URL rewrite, SSH/helper/proxy/redirect behavior must be bounded or explicitly unsupported under active policy. Merely running `remote get-url` once is not a proof: command execution and nested remotes can differ. Exact enforcement posture is an unresolved implementation proof, **not** a solved list of magic `git -c` flags. Do not edit machine Git/SSH config. Explicit unsupported-route errors may preserve safety while trying other routes, but refusing every allowed Git route is not full availability fulfillment of Need3.
4. Thread route skips through primary/mirror lookup, canonical marker/manifest lookup, full fetch, redirect target and refresh. Retain registry identity even if canonical host is excluded; a usable mirror can provide every needed read and content. Preserve expected hash/content checks and source provenance. Update any_package_source/preflight to share the same effective interpretation without performing speculative contacts.
5. Only then expose one policy setting/neutral input across promised operations and test the advertised scope. Public syntax/merge decisions remain separate; transitive pins and fixed-version preference are not required to implement exclusion and are not claimed complete here.

Smallest useful next executable atom: **typed bound HTTP admission and constructor/index/raw-HTTP gate with fake local redirect tests**, retaining explicit unimplemented Git-route status internally and keeping public exclusion unavailable. It is a real no-contact slice, not acceptance of all Need3. Full exclusion vertical must additionally close Git/submodule and composition-root coverage; this audit does not hide that cost.

## Existing test evidence and required deltas

Observed test code only; none run:

* `multi_registry_resolver/offline_tests.rs:82`: store resolve/list/fetch with fake backend and zero registries; `:132` asserts bootstrap_count == 0. Useful reuse fixture, **not** a full all-transport zero-contact oracle; it does not prove index probes or every backend method were never called.
* `git_backend/shell/tests_pure.rs:154,200`: credential-silencing command args and anonymized backend tests. Extend to policy retention and checked execution posture; existing tests do not assert exclusion.
* `git_backend/shell/tests.rs`: local bare-repository clone/fetch/tag/archive fixtures. Extend with recorded effective URL/config/submodule adversaries; do not use live hosts.
* `index_client/tests.rs:30,79`: pooled client reuse and Authorization scheme gate. Extend with forbidden initial/probe/redirect host counter, different policy client instances, env index override and policy-denial discriminants.
* `index_client/locate.rs:309,351`: raw GitHub default and lookalike rejection; use as exact route-map baseline.
* `redirect_follow.rs:344,411`: fake redirect success and hop-two rejection. Add target forbidden-before-marker and allowed-mirror canonical-marker case.

New no-contact oracle must count/panic on **every** fake backend primitive and every HTTP initial/redirect attempt, not merely bootstrap. Include anonymous clone, raw retry fallback, store reuse, index disabled/excluded, warmed checkout origin rewrite, submodule recursion, allowed mirror with excluded origin, all routes excluded and one allowed alternate registry. No-policy controls must preserve existing candidate order/failure law.

## Unresolved proof/inspection boundaries

The managed Git execution posture is the major proof obligation. Public arbitrary recipe subprocesses and network filesystems are not made safe by registry admission; do not market it as process-wide egress sandboxing. Composition roots outside the source-builder path must be inventoried before broad install/update/apply-manifest/download coverage is asserted. Source policy vocabulary/merge is still unaccepted. Current SourceEvidence and host-report fields must not become admission tokens.

Final authorized leaf inspection completed:

* `shell/raw_http/hosts.rs:69,78,117,153`: actual table maps github.com to raw.githubusercontent.com and GitVerse to its contents API. Remapped raw base feeds the concrete request URL. Existing Locator refusal becomes `None` (`.ok()`), intentionally declining the optimization and passing control to Git. **Hosting denial needs its own retained discriminant**; it cannot reuse this optimization-decline contract. `raw_http/retry.rs:36` caps attempts at 3 with a bounded pause budget; policy denial must consume zero retries.
* `fetch/bring.rs:53,68`: an existing clone's RefreshExisting path calls `backend.update(clone_dir, refname)` and does not consume the supplied candidate URL. Its real origin may be a previous mirror. A gate on the loop's URL therefore proves nothing about this fetch; admit the actual recorded/effective checkout origin in bound execution. Mirror source switch bootstraps beside old clone and preserves its existing safety behavior.
* `fetch/prepared.rs` retains before-state origin and calls `prepare_ref` against selected candidate/reuse paths; admission must reach both initial prepare and retained apply, including actual origin of reused objects. Preserve captured revision/content and recovery guarantees.
* `multi_registry_resolver/test_support.rs:100,104,119`: fake counts bootstrap/update but does not count list_tags/fetch_file_at_ref. Existing offline bootstrap_count==0 is genuinely narrower than zero contacts. Extend fake method recording or use a new strict wrapper over the same GitBackend seam.
* `raw_http/tests.rs:30,71,96,306,321`: useful exact host rewrites, unsupported SSH/HTTP/unknown fast path, retry classification/budget fixtures; no hosting-policy gate is demonstrated by those tests.

No framework or separate resolver is proposed to bridge these gaps.
