# Root execution disposition

Approved a complete reporting facet across existing project event and global app plan/list/describe (including common-configuration counterparts), not only project output. One shared derived helper and report-only HostingView; persisted SourceEvidence/index/steps/import unchanged. Wire labels are extensible strings, currently produced only for exact verified github.com and gitverse.ru. No source-policy settings, transport safety, contact inventory or whole Need1/3 acceptance. The original read-only proposal follows.

---

# Next hosting foundation: URL observation with a real reporting consumer

Read-only design, 2026-11-07. No source edits/tests/build/network. Baseline supplied by root: SourceFacts accepted `2df122323`, project event accepted `c600cb539`, Need2 accepted against its four obligations. **Hosting is not a newly invented missing Need2 requirement.** Remaining scope here is Need1 hosting-if-known/preference and Need3 excluded managed destinations. Pin/backtracking remains a separate explicit unresolved obligation.

## Recommendation

Implement a small **observed endpoint classification → derived reporting beside existing SourceEvidence** vertical first. Follow it with a separately reviewed complete contact-admission vertical, then fixed-version source choice. Do not bundle a new generic destination registry, standalone policy DTO or inactive parser flags into the foundation. The root's shared identity direction is sound only if “identity” means a normalized host observation; it must not become another source resolver or imply a verified network destination.

The first vertical's production consumer is `crates/vibe-registry/src/source_evidence.rs`: it already projects lock/CachedPackage facts into generated SourceEvidence used by plans and installed history. Attach deterministically known hosting to **the particular recorded URL**. This completes a small Need1 reporting facet, not Need1 overall or Need3.

## Existing ownership and why not reuse the wrong function

* `remap/Locator::locate` is the existing pure candidate authority: remap targets first, origin last unless denied (`locator.rs:130`). It does not open sockets. Preserve it as the only source-order/remap owner.
* `progress/transfer/record.rs:267` `safe_url` and `:290` `safe_remote` redact display strings. Do not treat a redacted string as executable source identity or proof of transport routing. Do not modify their current output as a side effect of this feature.
* `global_registry.rs:353` `url_is_local` is the historical offline heuristic, not a destination parser or security boundary. For example its recognized forms and fallback were designed for offline filtering; replacing it wholesale risks changing absent-policy behaviour.
* `source_evidence.rs` preserves `source_url`, `via_redirect`, `fetched_from`, acquisition and store reuse separately. Enrich these observations rather than make a parallel report model. Existing `progress::transfer::Destination` means local write destination (Store/Cache/Work/Home), so do not reuse that name for network host identity.

## Minimal contract (proposal for this vertical)

One pure classifier under the existing core remap/source boundary, e.g. **new** `crates/vibe-core/src/remap/endpoint.rs`, re-exported by `remap/mod.rs`. This location keeps Locator and consumers on one URL vocabulary; no new crate, service, global cache or registry catalog.

Its result distinguishes:

1. **Observed network authority:** normalized host, optional explicit port, transport spelling/category, optional known hosting label. This is syntax-derived observation of a URL/remote, not a successful connection or authenticated publisher identity.
2. **Local path/file source:** no network hosting claim. UNC paths and non-local file authorities must not be silently called contact-free; either distinguish them as opaque or as transport requiring further admission.
3. **Opaque/unrecognized remote:** no guessed host/hosting, with a bounded reason enum usable internally. Invalid/custom helper/ambiguous strings stay unknown; ordinary absent-policy operations retain current handling.

Parsing rules: reuse the already available `url::Url` parser for hierarchical URLs; handle the existing single `git+` wrapper and conservative SCP-style SSH spelling explicitly; distinguish Windows drives from schemes. Normalize host case/IDNA through the URL parser. Treat IPv6 as a host, retain explicit port without making hosting depend on it. Recognize **exact** github.com → github, gitverse.ru → gitverse; do not use substring or arbitrary suffix matching. Custom hostnames are valid observed authorities with unknown hosting. Known service-domain additions require explicit verified entries, not corporate-ownership guesses. Do not use DNS, SSH config, network probes or global Git config to produce a reporting label.

A syntactically recognized SSH host is still only the spelled host: SSH alias/config, ProxyCommand and Git insteadOf can change actual transport. Thus separate “observed host known” from “route enforceable”. No `safe=true` field or executable authorization token belongs in the classifier. A destination admission gate later uses this observation **plus the transport's actual admitted route**, not observation alone.

### Concrete report-only projection (root compatibility correction)

**Keep persisted SourceEvidence byte/schema meaning unchanged.** It is embedded in closed index5/steps3/import3 contracts. Adding optional persisted fields is not automatically backward compatible with strict old readers. This foundation does not bump or rewrite those storage contracts, and does not alter the accepted Need2 boundary.

Add one generated **report-only** HostingView alongside `source_evidence` in the current install plan-event report, derived from the same held source facts. Proposed fields: `selected_origin` and `differing_acquisition_route`, each optional known label; an optional `redirect_stub` label can be omitted from the initial public view because the required source is already distinct from the stub. No separately supplied URL catalog or settings lookup.

* `selected_origin`: classification of held `SourceEvidence.source_url`, meaning the canonical/selected source URL spelling for this package. A redirected package's source_url is the target, while via_redirect is the stub. This label does not mean the canonical host was contacted.
* `differing_acquisition_route`: classification of held `SourceEvidence.fetched_from` only, meaning a distinct acquisition-route URL recorded by the acquisition owner. Missing fetched_from produces missing label, **not** fallback to source_url. It is neither a complete contact list nor proof of a socket; transfer/transport observations remain the contact authority.
* Local, missing, malformed, unknown-domain and opaque SSH alias forms give unknown/absent hosting. `git@github.com:…` has an observable known URL-host label, but effective SSH/Git transport is still unverified; `git@company-alias:…` does not become github by guess or DNS.
* Store reuse retains canonical origin observation if present, but does not create a new acquisition/contact label. Do not infer an “actual host” from acquisition enum or current registry settings. Historical lack of evidence stays unknown.

Canonical github source + held gitverse mirror URL therefore yields selected_origin github and differing_acquisition_route gitverse. Exact URL string outputs/ref/commit are untouched. Do not call the second field simply `actual_host`: available evidence is route provenance, not network observation.

Real production consumer: `crates/vibe-cli/src/commands/install/report.rs:135` already consumes `planned.package_sources()` and generated `install::e2::plan::{InstallPlan,PlanEntry}`. Compute HostingView from that exact held evidence and emit it beside unchanged source_evidence. Put the shared report projection helper in the existing `vibe-registry/src/source_evidence.rs`; the CLI remains a renderer, not the classifier owner. Plans/global reports may reuse it later, but this atom does not need to migrate every app storage generation.

Exact first vertical perimeter: new `vibe-core/src/remap/endpoint.rs`; `remap/mod.rs` re-export; shared derived helper in `vibe-registry/src/source_evidence.rs`; `vibe-cli/src/commands/install/report.rs`; authoritative **install plan event/report** JTD and generated counterpart; bounded classifier/projection/event tests. Locate the exact JTD owner of `vibe_wire::generated::install::e2::plan` in the next packet before edits. If the closed report requires a successor generation, retain its old reader and use the existing report compatibility mechanism; do not casually add a field to a closed e2 schema either. SourceEvidence's owning shared schema and index5/steps3/import3 remain untouched. No Locator order, resolver candidate-set, global settings or CLI option changes in this initial atom.

## Next admission vertical: real consumer before any public exclusion switch

Normalize one effective operation policy from project/user/invocation when that vertical is implemented; earlier `[source_policy]` and merge/spelling ideas remain proposals, not accepted syntax. Bind it before resolver construction. Use Locator candidates to determine possible routes, filter/admit each target and preserve explicit exclusion diagnostics. Preserve existing Locator deny precedence; an allowed lower source cannot bypass an existing deny boundary. Never delete a registry solely because its canonical origin is excluded when an allowed mirror can satisfy all necessary metadata/content reads.

There are two mandatory consumer layers: **pre-open discovery** and **transport admission at every managed contact/hop**. A filtered catalog or Locator explanation alone is not enough. `MultiRegistryResolver::from_manifest` obtains clone_chain at `:352`, opens GitPerPackageRegistry at `:379`, resolves an independently configurable index URL at `:399`, then calls IndexClient::probe at `:407`. A builder-style `.with_policy(...)` after open is too late. Policy must arrive at construction or the transport before that probe. Index URL environment override can be on a different host from canonical registry.

Git remotes, HTTP redirects, package redirect stubs, remaps, mirror retries and canonical fallback must each pass admission before contact. An allowed URL that redirects to an excluded host stops before following it. Opaque remote-helper/SSH/Git-rewrite routes under an active exclusion must be qualified by the transport or refused **for that route**, while other eligible routes remain available. An arbitrary recipe script is not made controlled by this classifier or Locator; keep that limitation explicit rather than promise process-wide egress enforcement.

If only excluded candidates remain, report unsatisfied-under-policy with requirement and excluded known routes; without contact do not assert those routes contain the missing version. “Not probed” differs from “not found”. Local/store reuse with retained acceptable source semantics requires no fabricated contact.

### Exact files for the next bounded inspection packet

Already identified concrete constructor and routing owners:

* `crates/vibe-package-source/src/builder.rs`, `options.rs`: effective policy before open/explicit-local branch; inspect `any_package_source` consistency.
* `crates/vibe-registry/src/multi_registry_resolver/mod.rs`, `clone_chain.rs`, `walk.rs`, `dispatch.rs`, `redirect_follow.rs`, `override_resolve.rs`, `sources.rs`, `refresh.rs`, `home_refresh.rs`: constructor probe, direct sources, redirects, refresh/fallback coverage.
* `crates/vibe-registry/src/git_package_registry/mod.rs`, `lookup.rs`, `fetch.rs`, `urls.rs`, `auth.rs`: derive actual attempt URLs including fallback; do not check only org URL.
* `crates/vibe-registry/src/index_client/mod.rs` and `crates/vibe-registry/src/git_backend/mod.rs`: exact entry modules needed to identify probe/HTTP and ShellGit implementations and their redirect/config seams. They are entry paths, **not a claim that all transport logic resides in them**. Read their module declarations, then request the exact named leaf files rather than inventing a blanket transport perimeter.
* `crates/vibe-core/src/remap/artifact.rs`, `row.rs`, `active.rs`: candidate target/error shape and operation lifetime needed to carry filtering reasons through existing Locator. Existing `mod.rs` and `locator.rs` remain in scope.
* `crates/vibe-core/src/progress/transfer/mod.rs`: identify the actual managed HTTP transport/retry leaf from declarations. `record.rs` only records observations; it is not the fetch gate.

These additional files are **next-packet requests**, not files inspected in this audit. Further direct acquisition clients must be inventoried from these entry points before declaring Need3 complete. For the first reporting atom, identify the exact current install e2 plan JTD/compatibility owner and its event test files; do not modify the persisted SourceEvidence schema.

## Preference follows admission, at fixed version

Keep unconfigured resolver ordering and first-registry version enumeration unchanged. Preference alone does not reorder the catalog before enumeration and cannot introduce a newer version available only on another registry. At a selected version, rank equivalent admitted source candidates by hosting preference, then original registry order; retain existing mirror/remap integrity and precedence semantics. Do not reorder source provenance after metadata was solved: chosen manifest/evidence/acquisition must refer to the same admitted candidate or verified equivalent bytes. Fixed-version ranking is a real subsequent consumer of the same host classifier, not a second URL resolver. Pins remain unimplemented until the selected-edge/backtracking proof is settled.

## Proposed gates and acceptance wording

Initial vertical tests: HTTP(S)/git+/SCP/IPv6/IDNA/port/Windows paths/local-file/UNC/opaque forms; exact known-host mapping and hostile lookalikes; credential-bearing URLs never leak into labels; canonical/mirror/redirect distinctions; warm-store and legacy missing-route cases; same generated projection on real install event. Run appropriate tests later, none executed here.

Admission tests later must use fake transports that fail on any forbidden destination and cover index probe before resolve, environment index override, redirect hop, remap, mirror retry and canonical fallback, allowed mirror with excluded origin, and opaque-route refusal without rejecting unrelated allowed registries. Preference gates compare distinct-version and same-version cases and verify absent-policy behavior.

Accept the first atom as **deterministic observed hosting labels in a derived install report view beside unchanged shared source evidence** only. Do not claim full source plan, preference selection, hosting exclusion, transitive pins, or reopened Need2. This gives the foundational classifier an immediate meaningful consumer and leaves all authority decisions in the existing owners.

## Root decisions for execution packet

Settled: report-only derived view, held canonical/differing acquisition URLs, unchanged persisted SourceEvidence, no DNS/alias guesses, no new policy flags, and no claim of transport safety. Initial consumer is the current real project install plan event. Next inspect the event schema/compatibility owner before selecting its report generation. Hosting classification table remains exact github.com/gitverse.ru with unknown fallback; no extensible provider framework or host-alias config is needed for this bounded atom. Exclusion/preference policy syntax and transitive pins remain separate future decisions. Stop design here and implement only after root grants the concrete packet.
