# Root execution disposition

HTTP-only component accepted in57d30367f; this original proposal is preserved below, not a substitute for final code/feature evidence. Actual implementation extends existing IndexError/GitError/RegistryError instead of replacing their transport/auth/format distinctions. Final v3 additionally closes hostless initial/PURL refusal laundering and terminal DNS root-dot matching through shared core normalization, without changing credential-origin equality or source strings. Native Git availability and full operation exclusion remain mandatory subsequent work. See features/VN-HTTP-ADMISSION.md.

---

# Executable atom: bound HTTP admission, native fallback explicitly refused

Design only; no code/tests/network run. No public `--exclude-hosting` or settings. No full Need3 claim. Native execution, submodules and operation composition remain mandatory subsequent work.

## Boundary and API

Use one immutable `Arc<HttpAdmission>` per bound operation, **internal HTTP-only capability**, shared by IndexClient and ShellGit. Absence (`None`) is the legacy path with unchanged behaviour. `Some`, even with an empty denied-host set, means HTTP-only: native Git execution is unavailable under this capability. This makes fallback behaviour explicit and testable instead of silently escaping policy.

Proposed types in new `crates/vibe-registry/src/http_admission.rs` (narrow GET admission owner, not a generic broker):

```rust
pub struct HttpAdmission { /* immutable validated denied HTTP host keys */ }
pub enum ContactRefusal {
    ExcludedHost { endpoint: SafeEndpoint },
    UnsupportedScheme { endpoint: SafeEndpoint },
    NativeTransportUnavailable,
    RedirectLimit,
    InvalidRedirect,
    HttpsDowngrade,
}
enum HttpGetError { Refused(ContactRefusal), Transport(reqwest::Error) }
```

`SafeEndpoint` here denotes an existing safe diagnostic projection (or a private normalized scheme/host/port value), **not** a new public persistent schema. Refusals contain no raw URL credentials/query. Constructor accepts exact normalized hostnames, not hosting labels: eventual github-family expansion must include actual managed raw.githubusercontent.com and belongs to later policy normalization. Reuse the accepted endpoint parser where compatible; report labels do not authorize a request. The implementation packet must include that parser's current exact source.

Add bound constructor variants: `IndexClient::probe_with_admission(base, auth, Arc<HttpAdmission>) -> Result<ProbeOutcome, ContactRefusal>` and `at_with_admission(...)`; `ShellGit::with_http_admission(...)` is allowed only as consuming construction before use and **resets its raw pool**. Existing constructors/methods delegate through the unbound legacy implementation. `MultiRegistryResolver` gets an explicit bound construction entry, using the binding before `from_manifest` opens/probes; it creates/passes the same bound ShellGit. Never attach it after open. No mutable global slot. Anonymous ShellGit reconstruction clones the Arc and only shares a raw client built under the same binding.

Actual caller behaviour: bound `probe` may return allowed HTTP index results; a 404/uninterpretable response may normally want Git, but the bound native owner returns `GitError::ContactRefused(NativeTransportUnavailable)` **before any subprocess**, rather than ls-remote/archive/clone. A valid raw HTTP manifest may succeed without Git. This is useful managed HTTP functionality and explicit fallback refusal, not final installation availability.

## One GET path, redirects and credentials

Factor a narrow GET helper used by both existing HTTP owners. Owners still own URL construction, query parameters, body/status interpretation and retries. The helper receives the **fully constructed** URL (including index queries or rewritten raw host), request headers and immutable binding.

* Unbound: retain existing reqwest configuration/redirect behaviour and error classification.
* Bound: client disables automatic redirects and ambient proxies; helper admits initial URL before `send`, then each resolved Location before another request. Only HTTP/HTTPS are supported. Explicit proxies/opaque routes are unavailable in this atom, not silently inherited. This is a logical HTTP destination guarantee, not a DNS/IP egress sandbox.
* Follow GET redirects 301/302/303/307/308, resolve relative Location against the response URL, maximum **10 followed hops**; attempt 11 refuses before contact. Invalid/missing Location on a redirect is typed InvalidRedirect. HTTPS→HTTP refuses before contact. Loop exhausts the same limit. No DNS/network needed to classify.
* Bound clients have **no bearer default_headers**: attach authorization to the initial authorized HTTPS request in the owner, preserve only on same scheme/host/effective-port redirects. On any origin change strip Authorization/Cookie/Proxy-Authorization and never restore them later in that chain. Never add Referer. Existing initial auth scheme restrictions remain. Cross-origin redirect may proceed without credentials if admitted; its 401 remains ordinary auth failure.
* This avoids reqwest error-source downcasting/string matching: manual hop admission returns `HttpGetError::Refused` directly. Bound client redirect-disabled configuration prevents hidden second requests. Do not claim `.no_proxy()` alone controls every OS/network hop; its purpose is eliminating this client's ambient HTTP proxy indirection.

## Typed propagation, no fallthrough laundering

Add `ContactRefused(ContactRefusal)` to IndexError and GitError; map into a structured RegistryError variant without converting to prose. `probe_with_admission` and handshake preserve the refusal via Result, not ProbeOutcome::Absent/Refused{reason}. Existing legacy auth/format Refused remains distinct.

Raw owner can keep `Option<Result<Bytes,GitError>>`: genuine optimization decline stays None; admission refusal is `Some(Err(GitError::ContactRefused(...)))`. Retry Attempt gets an explicit terminal refusal path: **zero retries and zero fallback** for policy errors. HTTP status/transport retry remains current bounded three-attempt behaviour; every retry is re-admitted and follows the same bound redirect driver. Client-build failure or unsupported raw form can still decline, but subsequent native execution refuses under the binding.

Index `list_candidates` currently catches all client errors and tries Git. Preserve ordinary fallback, but propagate ContactRefused explicitly (or retain it while considering another admitted route); do not recast it as absent/unreachable. For this atom, fail the bound index operation on contact refusal; alternate-registry/mirror route-skip aggregation is the next registry-choice chunk, **not** claimed completed here. `by_name_is_current` currently uses cached `.repomd().ok().flatten()` and returns bool: its bound path must propagate refusal (Result) while retaining existing non-policy fallback semantics. Audit callers before changing that signature. `search_catalog` must not erase refusal on fallback either.

Native refusal goes at all native launch owners: ShellGit `preflight`, `run`, `run_raw`, and `run_transferring_with_prefix`, **before spawning**. This atom deliberately also refuses local native Git under an HTTP-only binding; LocalRegistry filesystem reads are unaffected. This simple boundary prevents hidden checkout/submodule/helper contacts without pretending to have solved Git transport. No-policy branches preserve every current native path.

## Exact patch perimeter / implementation packet

* New `crates/vibe-registry/src/http_admission.rs` (+ bounded tests), `src/lib.rs` module/re-export, `src/error.rs` structured registry refusal; these last two exact files must be included in the implementation packet.
* `index_client/{mod.rs,pooled.rs,handshake.rs,by_name.rs,catalog.rs,auth.rs}`: bound constructors/pools, all GET call sites (lookup_purl/search included in mod), request-scoped auth, no swallowed refusal. Inspect `static_search.rs` for error-preserving fallback, change only if needed.
* `git_backend/{mod.rs,shell.rs}` and `git_backend/shell/{raw_http.rs,reported.rs}`: typed Git error, constructor binding, anonymous clone retention, native pre-spawn guards and raw GET/retry. `raw_http/hosts.rs` remains URL constructor; admit its resulting AddressedRead.url, not repository host. Change address return shape only if needed to retain actual admission refusal; ordinary Locator optimization decline remains separate.
* `multi_registry_resolver/mod.rs`, `git_package_registry/lookup.rs`: bound construction before probe and typed refusal instead of broad index-error fallback. Explicit bound resolver entry is used by tests/embedding only; no CLI/settings yet. If composition is exposed through `vibe-package-source`, options/builder must carry the same Arc before open, not fabricate another policy.
* Tests: `index_client/tests.rs`, `git_backend/shell/{tests_pure.rs,raw_http/tests.rs}`, a new HTTP-admission loopback integration fixture. Include `vibe-core/src/remap/endpoint.rs` read-only to reuse accepted parser meaning; no hosting-report edits. Schema changes are not required: this internal atom adds no public wire policy or persisted source fields.

Before writing, inspect exact current `src/error.rs`, `src/lib.rs`, endpoint parser and callers of the bool helper. If the helper's callers lie outside this packet, request their exact paths; do not quietly retain a swallowing adapter. Confirm current reqwest APIs used for redirect-disabled/no-proxy clients from installed dependency source if necessary; no dependency upgrade is proposed.

## First RED and acceptance matrix

**First RED:** real IndexClient probe against loopback A (`127.0.0.1:ephemeral`) returns Location to loopback B (`127.0.0.2:ephemeral`). Binding denies host 127.0.0.2. Assert exact typed ExcludedHost, A sees one request, B sees **zero**, no later hello/repomd candidate requests. A second case denies A initially and asserts zero requests anywhere. This proves pre-probe and hop gates, not just helper parsing. Servers are bounded local fixtures; never public network. Use numeric loopback addresses, not localhost DNS. If platform cannot bind the second loopback address, fixture must fail/explicitly qualify coverage, not silently skip the invariant.

Then cover: all index GET families; raw actual-base rewrite/redirect/retry; 10-hop limit and downgrade; same-origin header retention/cross-origin permanent stripping (HTTPS-auth rule as pure request-header tests plus local transport integration); separate-policy client pools; anonymous backend same binding; legacy controls unchanged. Bound raw 404/unsupported scheme/pool-build failure must reach **NativeTransportUnavailable with zero native spawn**; use a sentinel Git binary recording any invocation. Bound denial must survive catalog freshness checks and index→Git fallback. Existing raw retry tests plus a denied endpoint assert zero sleeper calls. Positive bound HTTP path returns the exact bytes/parsed result; this cannot be an atom that only refuses everything.

Acceptance wording: **HTTP-scoped admission with typed refusal and no native escape under its immutable binding.** Not full hosting exclusion, not an exposed policy setting, not complete allowed-mirror availability. Native admitted execution/submodule/config proof, alternate-route policy diagnostics and all command composition roots remain mandatory next atoms. Do not leave “all Git unsupported” as the final answer to Need3.
