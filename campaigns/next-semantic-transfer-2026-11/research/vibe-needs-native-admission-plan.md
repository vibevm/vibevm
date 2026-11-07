# Root execution disposition

N1 accepted46b6297a3 as direct verified-HTTPS object preparation and bounded metadata. Actual capability uses explicit caller-trusted installation/TLS inputs, tested Linux Git2.53.0 with conservative version floor. It does not authenticate executables or claim all platforms. Warm effective ambient meaning is explicitly unsupported until N2 capture; N2 apply/recovery, N3 SSH, N4 recursion/redirects and full operation composition remain. The original broader research proposal follows; see features/VN-NATIVE-PREPARATION.md for accepted scope and limitations.

---

# Native Git admission: executable next units and bounded evidence

Read-only research, 2026-11-07. Accepted HTTP component: `57d30367f` (root-supplied). No production/repository edits, Cargo, public DNS/network or real HOME/config/profile changes. Experiments used temporary repos/configuration, numeric loopback endpoints and child-only environments. Servers are closed. Exact scripts/results/stdout hashes accompany the JSON receipt.

## Decisive result

**An explicit URL plus several `git -c` flags in the current checkout is insufficient.** Existing `prepared.rs` deliberately fetches in the warm checkout's native configuration context. A local include/insteadOf still redirected an otherwise globally isolated Git command to B. The smallest sound acquisition seam is the existing prepared-object owner, using an **operation-owned transport context** and captured endpoint/auth inputs; the existing guarded apply owner must retain publication/recovery authority.

Do not weaken `with_http_admission` or its native guards. Introduce a **separate native capability constructor/type**, not `allow_native: bool`. Both capabilities share the same immutable normalized denied-host binding. HTTP-only continues refusing every native subprocess exactly as accepted. Unbound mode stays unchanged.

## Experiments: real controls, qualified

Git **2.53.0**. A = numeric 127.0.0.1, B = numeric 127.0.0.2. Native Git operated against local dumb-HTTP Git repositories; results are not mocks of Git's URL handling.

| Experiment | Legacy result | Constrained result |
|---|---|---|
| Global config includes insteadOf A→B | exit0, A=0/B=2 requests | isolated HOME/global/system config + explicit URL: exit0, A=2/B=0 |
| Warm repository local include A→B, global disabled | exit0, B=2 | operation-owned cwd + captured explicit URL: exit0, A=2/B=0 |
| HTTP redirect A→B | exit0, A=1/B=2 | native followRedirects=false: exit128 **after** A=1, B=0; separate direct A positive exit0 |
| Recursive submodule on B | clone exit0, B=5 | bare parent acquisition exit0, B=0; driver reads .gitmodules and refuses child before clone |

All four controls were repeated over **HTTPS** with a freshly generated temporary CA/certificate, numeric IP SANs and normal TLS verification (no sslVerify=false). A direct HTTPS bare clone also succeeded, A=5/B=0, with HEAD equal to fixture source HEAD. This proves tested TLS Git transport remains useful under the isolation recipe. Production trust-store capture, authenticated HTTPS and smart-HTTP POST/pack endpoints are **not yet proved**.

SSH destination-only experiment: real Git→OpenSSH with an explicit child SSH config `HostName B` contacted B once and A zero; controlled `-F /dev/null` + direct endpoint contacted A once and B zero. Both Git invocations exited128 **after** TCP contact because the collectors deliberately closed after the SSH banner. This proves the routing difference, **not** successful SSH authentication, host-key verification or usable SSH Git. No sshd or real user's remote shell/profile was used.

The submodule refusal was in the Python proof driver, not product code. The experiments show why every child needs admission; they do not prove complete recursive-submodule availability. Original and rerun receipts are retained; reruns add full fixture stdout as requested.

## Production contract and exact execution boundaries

Proposed new `NativeGitAdmission` contains the same `Arc<HttpAdmission>` denied-host binding plus a **sealed explicit transport profile**, selected Git/OpenSSH binary identities, operation-owned context paths and explicit auth/trust inputs. Refactor only the shared denied-host predicate out of HTTP scheme validation as needed; the existing HTTP capability still accepts only HTTP(S) and still forbids native execution. DNS root-dot/IDNA/IP normalization must use `normalized_host_key`; credential origins keep exact scheme/host/effective-port semantics. A native allowance is not inferred from a report label.

`AdmittedGitRemote` is a private captured route result, produced after validation, never freely constructible from a URL string. It retains canonical source versus effective fetch endpoint separately. Command dispatch should distinguish **read-only metadata**, **object acquisition**, and **local publication**. Replace “any arbitrary argv becomes safe after one check” with bounded existing backend operations; do not introduce a generic command broker.

### 1. Object acquisition: common direct HTTPS

Use operation-owned bare repo/working directory and child-only HOME/XDG/config environment. Build the child environment from an allowlist; exclude inherited GIT_CONFIG_COUNT/KEY/VALUE/PARAMETERS, GIT_DIR/WORK_TREE/COMMON_DIR/INDEX_FILE, Git/SSH helper overrides, proxy variables, loader injection and uncontrolled exec-path/template configuration. Use trusted explicit Git and builtin-helper paths. Never fetch in an authored/warm checkout merely because its origin string was checked.

This is structural isolation **plus** command controls, not flags alone: operation-owned Git config has no includes/insteadOf/custom helpers/filter definitions; empty template/hooks inputs; explicit allowed protocol; no recursive submodule fetch; no implicit maintenance/hooks; no hidden HTTP redirects/proxies; TLS verification on with approved trust input; explicit captured endpoint rather than named `origin`. Treat partial/promisor object stores and lazy-fetching object reads as unsupported until a no-lazy-fetch proof is in place. Reuse local object stores only under that proven local-object contract; copying/referencing their bytes must not import their config authority.

The exact policy-enabled config/env allowlist is an implementation artifact to test against installed/minimum supported Git. Experiments prove global/local isolation and direct TLS functionality on 2.53.0; they do not prove every environment/configuration escape. Existing `ShellGit::with_binary`/VIBE_GIT_BINARY custom executables are an explicit unsupported route for the admitted capability unless independently admitted, while unbound behaviour remains available.

Credentialed HTTPS must receive a request-owned credential scoped to the admitted **exact** origin, separately from recorded URLs. Do not inherit arbitrary credential helpers, askpass, netrc/cookies or ambient extraHeader. Support the existing token-env credential source via a controlled ephemeral Git auth channel whose value is never argv/log/config persisted in the installed repo; establish an authenticated loopback TLS test before acceptance. If a registry requires an unapproved helper, return typed UnsupportedCredentialRoute rather than silently trying unauthenticated or invoking it. Never disable certificate verification to make the path work.

### 2. Warm origin and metadata-only native calls

Existing `prepared::recorded_origin` reads remote.origin.url; `prepared::origin` asks remote get-url; `fetch/bring.rs` updates the checkout's own origin rather than its supplied loop URL. Preserve these distinctions. Capture the effective warm endpoint and raw recorded origin **before contact**, then admit the effective endpoint and execute it explicitly in the operation-owned transport context. If an existing rewrite points at excluded B, refuse B; do not silently fetch recorded A and claim the old source was preserved. The proof's explicit-A experiment demonstrates isolation, **not** permission to erase a warm rewrite.

Metadata reads in the original repo may be whitelisted narrowly (trusted Git, no pager, config/remote get-url/ref identity operations with controlled environment) only after proving they execute no helpers/hooks/network. Do not run broad `git status`, checkout, arbitrary aliases or arbitrary `-c` inputs as “metadata”. Capture supported config/auth meaning into owned values; unresolved include/SSH/helper semantics cause a precise unsupported-context refusal. Once captured, no later command re-reads ambient transport configuration. Recheck original origin/head ownership as existing prepared/apply invariants demand before publication; drift refuses rather than silently changing source.

### 3. SSH direct route profile (next mandatory proof)

Support direct `ssh://user@fqdn:port/path` and unambiguous SCP FQDN/numeric spellings with the same host gate. Use a trusted OpenSSH binary with an operation-owned `-F` configuration, explicit destination/user/port, no ProxyCommand/ProxyJump/CanonicalizeHostname/Match exec/LocalCommand/forwarding, batch mode and strict host-key checking against an explicit captured known-hosts input. Identity key/agent selection is explicit and credential material is never logged/copied into a public receipt. Do not default to accepting unknown host keys.

Opaque aliases, custom SSH executables, jump/proxy chains and credential helpers are explicit limits until admitted-route support exists. Ignoring an ambient alias while claiming its configured meaning is preserved is incorrect. A direct-profile caller deliberately supplies the effective endpoint; warm SSH origins that need uncaptured config refuse with a reason. Next proof must run successful authenticated Git ls-remote/fetch against an isolated numeric-loopback SSH fixture, verify expected host-key failure, and show forbidden destination/proxy alias gets zero contact. This research did not establish that proof.

### 4. Redirects, submodules and local apply are explicit traversals

Native HTTPS uses `http.followRedirects=false`; the observed direct positive works. A native redirect is not a silent retry. Initial canonical relocation can be followed only through a typed admitted HTTP discovery step retaining each Location and credential-origin transition, then Git is invoked at the captured final endpoint with hidden redirects still disabled. **Do not infer Location from arbitrary Git stderr.** Smart-HTTP upload/pack redirect handling is not proved by a preliminary GET; such routes must explicitly refuse until implemented, not bypass the gate. This bounded direct-route support is useful while redirect support remains a mandatory follow-up.

No `clone --recurse-submodules` or `submodule update --recursive` under admitted native execution. Read the exact selected commit's .gitmodules and gitlink entries as data, resolve relative child URLs against the captured parent route, admit each child and acquire its exact gitlink commit through the same object-preparation path. Track cycles/repeated repositories and ownership; do not let local submodule URL config override the captured child route unnoticed. Publish child state only through the existing prepared/guarded lifecycle. A denied child must not leave a partially published parent. Complete recursive support is mandatory, not a permanent blanket refusal.

Current `prepared.rs::check_manifest_attributes` rejects transforms only on vibe.toml while deliberately preserving other payload filters/EOL meaning. A network-safe apply must additionally prove no arbitrary filter, fsmonitor, hook, lazy object fetch or helper executes on payload checkout/cleanup. Do not simply strip filters and install different bytes (e.g. LFS pointers) while claiming equivalence. Use an operation-owned local materialization context with captured supported transforms; unsupported external transforms are a typed pre-publication refusal until separately implemented. Preserve selected commit/object hash, original slot validation, no named-ref mutation during preparation, exact local transfer, rollback and old-slot survival. Native transport capability alone must not reopen arbitrary old apply commands.

## Minimum executable production unit

**N1: admitted HTTPS object preparation and metadata read, no public option.** Separate constructor/profile, direct allowed HTTPS ls-remote and bare prepare_ref using existing `GitPreparedRevision`; warm effective endpoint captured/refused correctly; no HTTP-only relaxation; no hidden redirect/submodule/helper execution. Existing raw HTTP client can share the same immutable host binding. Return exact retained bytes/commit through existing prepared source owner. Publication stays unavailable through this new capability until N2 local-materialization proof, rather than accidentally calling legacy checkout. This is a real usable native preparation increment, not “all Git unsupported”.

Minimum perimeter:

* `crates/vibe-registry/src/git_backend/shell.rs`, `shell/env.rs`, `shell/reported.rs`: distinct capability constructor, typed command purpose, controlled child environment and every launch path; anonymous clone preserves exact capability.
* New bounded `shell/native_admission.rs` for the immutable native profile/captured route and operation-owned context, not another resolver.
* `git_backend/mod.rs`, `http_admission.rs`: structured native refusals/shared exact host check without changing HTTP-only scope.
* `shell/prepared.rs` and `git_package_registry/fetch/prepared.rs`: acquisition in owned context, captured warm origin/auth, exact revision retained, phase-specific local-read vs apply handling. `fetch/bring.rs` and `shell/checkout.rs` remain guarded until their operations use the admitted path; no bypass via legacy refresh/bootstrap.
* `multi_registry_resolver/construction.rs`: new explicit admitted-native constructor before index probe; existing HTTP constructor unchanged. Existing Locator/routes/integrity owner unchanged.
* Bounded tests beside those owners using the retained numeric-loopback recipes. Implementation packet should include the existing source/registry error owner if adding structured errors; no new JSON schema or public settings are needed for N1.

Mandatory next units: **N2** safe local apply/recovery and full source-metadata parity; **N3** authenticated SSH profile proof/implementation; **N4** explicit recursive submodule and admitted redirect availability; then complete alternate-route/composition/public-policy coverage. These are not claimed done by this research.

## Acceptance matrix for implementation

N1: allowed direct HTTPS ls-remote/bare prepare exit0 and exact source HEAD; denied normalized/root-dot host zero requests; malicious global/include/env and warm local rewrite controls with legacy actual B contact; captured warm effective B refusal rather than silent A substitution; HTTP-only native sentinel remains zero; anonymous clone retains capability; no secrets in logs/config receipts; redirect reaches A then refuses before B; native errors retain typed refused/unsupported versus absent.

N2: old lock/slot/refs unchanged on rejected source/transform; exact retained commit used after origin/HEAD drift; malicious hook/filter/fsmonitor/lazy-fetch sentinel remains unused; supported materialization bytes match existing semantics; rollback survives conflicts and old clone is not deleted on refusal.

N3/N4: successful SSH auth and strict host-key controls; forbidden ProxyCommand/alias route zero B; allowed recursive child installs exact gitlink commit; forbidden nested child zero contact and no parent publication; relative URLs/duplicates/cycles; smart HTTPS redirect/auth cross-origin tests. If unsupported, report precise route capability failure without claiming full Need3.

The current proof supports proceeding with N1 implementation review. It does not authorize edits or establish a complete native security model. All existing accepted HTTP-only guarantees remain a regression gate.
