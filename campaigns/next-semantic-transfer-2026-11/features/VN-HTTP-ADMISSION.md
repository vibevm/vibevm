# HTTP-only admission — bounded foundation

Accepted internal library component. It is not a user-facing exclusion setting, complete Need3, or a substitute for safely admitted native Git/submodule execution and allowed-mirror availability. No CLI/settings/parser flags are exposed by this slice.

## Contract

An explicit immutable HttpAdmission binding contains exact normalized HTTP host keys, not hosting display labels. It reaches the actual IndexClient and ShellGit/raw-client construction before probes. Pools, rebindings and anonymous backend clones retain the same binding; there is no new mutable global policy. Unbound callers keep the existing behavior.

The constrained GET owner checks the original addressed URL before request construction, the fully constructed request URL, and each redirect/retry destination. It disables automatic redirects, ambient proxies, Referer generation and reqwest protocol retries. Bound requests support HTTP(S) only, limit followed redirects to10, and refuse invalid/missing Location, downgrade, unsupported schemes and URL credentials before the next contact. Authorization is initially attached only under the existing HTTPS rule. Crossing scheme/host/effective-port permanently strips Authorization/Cookie/Proxy-Authorization for that redirect chain.

Refusals remain typed through index probe, by-name/PURL/search/catalog/freshness, raw optimization/retry and registry lookup. They are not absent/unreachable results, an optimization None or a native fallback. Both primary and mirror refusal are terminal in this bounded atom; future admitted alternate-route aggregation is still required. Raw optimization decline retains its old meaning for other cases.

Every managed native ShellGit launch owner refuses before spawn while this HTTP-only binding is present, including an empty denied-host set and local Git. This deliberately restricts the capability; it does not claim to solve native Git configuration, helpers, redirects or submodules. Positive admitted HTTP works. LocalRegistry filesystem behavior is unchanged; this is not a DNS/IP/process-wide egress sandbox.

## API and compatibility

`MultiRegistryResolver::from_manifest_with_http_admission` constructs the bound backend before source open. IndexError/GitError/RegistryError preserve typed refusal beside existing transport/auth/format errors. `by_name_is_current` and `resolve_name_candidates` now return Result; the internal caller and two package-source call sites propagate the result without changing ordinary unbound best-effort behavior.

No persisted/wire schema, dependency version or report hosting model changed. Existing open/from_manifest and the bound construction were extracted together into construction.rs after the600-line gate failed; original annotations remain and the child retains registry-model scope.

## Evidence

Final frozen v3 HTTP suite:24/24 pass, including initial zero-contact denial, A→B redirect denial with B=0, positive allowed A→B, all index read families, policy-isolated pools/clones, raw bytes/rewrites/retries, no-sleep refusal, native-process sentinel, ten-hop boundary, credentials and typed hostless/PURL rejection. Separate mirror-refusal regression passes. Earlier full registry library303/303 is a pre-final regression receipt, not a claim that the frozen full workspace was run.

Shared host-key normalization additionally closes terminal DNS root-dot bypasses, with core4/4 and explicit loopback-resolver zero-connect evidence. Exact URL-origin credential stripping remains unchanged; original source URI strings are not rewritten. No ordinary-DNS failing probe was executed.

Final CLI all-target offline check, registry/package-source strict Clippy, scoped Conform and formatting pass. Registry Conform has0new findings while retaining3 existing records; package-source Conform has0. Only bounded numeric loopback servers and local fixtures were used; no public-network or live TLS measurement is claimed. HTTPS auth decisions have pure tests plus local header-chain transport controls.

A deliberate removed-gate mutation let forbidden B receive a request and failed the oracle; this is not labeled an incumbent legacy bug. Two actual typed-refusal defects were reproduced and fixed: request construction could erase unsupported initial addresses as Absent; PURL construction could erase them as ordinary Http. Original, successor and review receipts remain distinct. Root verifies final v3 hashes, committed blobs and specification-map continuity separately.

## Mandatory remaining work

Implement admitted native Git/SSH execution and submodule handling without relying on ambient rewrites/helpers/proxies; preserve permitted Git functionality rather than treating universal native refusal as the final feature. Then integrate route skips and allowed mirrors, independent index URLs, canonical marker fetches, refresh/composition roots, policy diagnostics and the public operation input. Preference, transitive pins, no-write preview and complete Need1/3 remain open. Accepted Need2 and historical113 counters are unchanged.
