# Needs 1–3: current-main source capability audit

Read-only source audit, 2026-11-07. No runtime commands, Cargo, tests, network, Git mutations or app edits. Findings are observations of current files, not executed behaviour. Input: `/fast/git/vibevm-app-r123/campaigns/architecture-2026-10/VIBE-NEEDS-2026-11.md`; its 1.0.11 claims were treated as hypotheses. Source hashes accompany this report in JSON.

## Coverage

| Owner need | Current source evidence | Verdict |
|---|---|---|
| 1: project plan-only, full closure | `vibe-cli/src/cli/pkg.rs:124` still requires global for plan_only. `vibe-install/src/plan.rs` already owns closure planning, conditional expansion, fetched nodes and prepared in-place acquisition. | User surface missing; shared internal mechanism exists. Do not create another solver. |
| 1: per-package source in real plan | `vibe-cli/src/commands/install/report.rs:132` PlanEntry has only package/version. `MultiResolution` and `CachedPackage` already hold registry/source/ref and acquisition commit. | Projection missing. |
| 1: skipped candidates/reasons | `RegistryWalkAttempt` records only NotFound/Public401 for aggregate failure; successful walk returns early. | Incomplete. Unvisited candidates cannot truthfully be called older, unreachable or available. |
| 1: requirement registry pin | `InlinePackageDepWire` has version/path/git/ref/auth/link etc, no registry. Existing override is URL replacement, not a named registry pin. | Missing; requires edge constraint semantics, not CLI-only routing. |
| 1: hosting preference | `PackageSourceOptions` supports local/embedded ordering and auth/offline controls, not hosting. `MultiRegistryResolver::list_versions_inner` returns first registry's nonempty version list. | Missing. Reordering registries before version choice would alter version selection, contrary to same-version-only preference. |
| 2: global installed source | `vibe-app/src/resolve.rs:129` reuses build_install_resolver and resolve_and_fetch, then from_manifest discards CachedPackage provenance and sets AcquiredStore. `index/wire.rs` has no durable source provenance. | Missing despite newer continuation metadata. |
| 2: global plan/list/describe source | application_plan schema has no selected-source field; list/describe LauncherSource means launcher origin, not package registry origin. ApplicationPendingSource stores directory/kind/optional registry/project for continuation. | Missing; do not mislabel continuation or launcher source as installed provenance. |
| 3: excluded hosting, no contact | Existing offline local-only filtering and remap Locator denial provide useful owners, but no exclude-hosting option/policy found. Resolver construction can probe index HTTP before resolve. | Missing; filtering solved output is too late. |

B-232 (source plan, requirement pin, hosting order) remains open/high priority; source-related B-254 (formerly a duplicate B-233) remains candidate. B-222 application export/import is done and should round-trip or intentionally omit source policy with explicit semantics; do not call it the 222-task campaign itself.

## Existing owners to extend

* `vibe-core` manifest/lockfile/global_registry: authored requirement and registry vocabulary, effective project-first settings merge, canonical source identity; `lock_provenance` separates actual fetched_from from canonical source_url.
* `vibe-core::remap::Locator`: existing mirror/override/remap candidate mechanism. `multi_registry_resolver/clone_chain.rs` explicitly unifies mirror and remap routes. Add policy around this mechanism, never a second source-routing world.
* `vibe-package-source`: surface-neutral PackageSourceOptions/build_install_resolver and InstallResolver already compose local, embedded, declared and offline sources; app uses this owner too.
* `vibe-registry`: version discovery, concrete source resolution, redirect/fetch and provenance. MultiResolution describes selected source before full acquisition; CachedPackage carries acquired commit, canonical URL and actual fetched_from.
* `vibe-resolver`: single dependency solve, all providers must observe the same constrained candidate relation.
* `vibe-install` and `vibe-orchestrator`: existing closure/conditional/configuration/admission and ready/apply separation. CLI only renders their result.
* `vibe-app` index/resolve/operations: durable application state and installation lifecycle; preserve shared source evidence at the acquisition seam and through replacement, rollback, import, resume and update.
* `schemas/*.jtd.json` and generated vibe-wire: contract authority. Change schema first; regenerate generated structs, never manually fork a JSON response DTO.

## Fundamental contract needed (proposal, not accepted implementation)

1. One shared source decision/evidence value: package coordinate, canonical registry alias/source URL/ref/kind, optional observed commit, optional actual acquisition route, optional classified hosting, selection reason, and observed/unvisited/excluded candidate states. Keep canonical origin separate from actual contact; content identity stays unchanged. Legacy unknown is explicit absence/unknown, never inferred from current settings.
2. One effective source policy at source construction and request admission: preferred host order and exclusions, with deterministic project/user/invocation precedence. Hosting classification parses actual URL hosts including SCP SSH spelling; self-hosted/custom hosts stay exact or unknown, not guessed as GitHub. Any explicit hosting label is presentation metadata, not authority to bypass actual destination checks.
3. Registry pin belongs to a requirement edge and resolves against a defined declaring registry-name scope. Thread it through metadata/provider paths and conditional/transitive roots. Multiple incoming incompatible pins require a typed conflict; do not silently let last hashmap writer win. Define relation to override/path/git explicitly: incompatible combinations must refuse rather than silently bypass the user's pin. Pin constrains canonical registry; exclusion independently constrains actual contacts, including its mirrors.
4. Planning effect contract separates semantic project writes, shared-cache acquisition and temporary acquisition. Existing planning fetches into store; registry constructors mkdir cache buckets, global app resolution may create default global config, and recorded migration semantics exist. Wrapping it as 'writes nothing' is false. For literal zero filesystem writes, use read-only metadata/cache plus in-memory remote observations and refuse when unsupported; otherwise explicitly negotiate a plan allowing cache/temp writes. Never silently relax owner wording. Conditional metadata/embedded-source acquisition and in-place preparation need the same effect budget.
5. Captured decision is not a promise that a future mutable tag still resolves to the same bytes. Return commit only when observed; preview unknown remains unknown. Apply consumes prepared/captured source or detects drift and replans/refuses. Separate selected canonical origin from possible fallback contacts; a fallback fetch can change actual route while preserving hash.

## Exclusion boundary

Gate before index probe in MultiRegistryResolver::from_manifest, version enumeration, dependency manifest fetch, clones, direct git/override, redirect target, mirror/remap targets, fetch retries and canonical fallback. Effective settings includes global entries. Redirects must recheck each hop. An allowed original URL remapped/redirected to excluded host must not contact it; an excluded canonical origin may remain usable via an allowed mirror only if all discovery and content operations can stay allowed. Reuse offline no-network test techniques, not its all-or-nothing semantics.

Transport redirects and SSH aliases can obscure final destination; either enforce at transport or state exact URL-level guarantee and refuse unsupported indirection. Arbitrary recipe scripts are outside current remap fetch authority (application_plan schema already calls this out). A claim that the entire process can never contact a host needs egress enforcement, not just resolver policy.

Without contacting an excluded source, a fresh resolver cannot prove it alone contains a matching version. Emit policy-constrained-unsatisfied naming the requirement and excluded routes, plus cached evidence when available; never fabricate 'only exists there'. Likewise a contact preview can be complete for declared routes but must label unresolved redirect/recipe contacts unknown.

## Dependency order and bounded first slice

First settle source-evidence and effect semantics, pin scope/conflict law, and same-version preference compatibility with first-registry version enumeration. Next implement one generated source-evidence projection from existing MultiResolution/CachedPackage and existing lock facts into retained install plan/application result; preserve it in application inventory with backward unknown. This bounded first implementation establishes truthful provenance end-to-end using the existing owners; it does not claim project plan-only, pin or exclusion complete. It should include durable actual-source evidence rather than just a decorative pre-fetch label.

Then extend the one provider candidate relation with requirement pin and effective policy, retaining diagnostics; gate contacts before all discovery/fetch. After effect-budget support, expose project plan-only using the same closure engine, and map the same decision to actual install plan events and global plan. Preference should choose equivalent candidates at fixed version without changing existing unconfigured selection; do not silently replace first-registry semantics with global-highest-version semantics. Finally cover update/apply-manifest/import/resume and frontend parity.

Root decides whether this sequencing or the full P1 capability vertical slice is the next authorized atom. No semantic blocker prevents the audit; contract choices above block a truthful implementation claim.

## Required tests (proposed, not run)

* Two registries, same package/version: preference chooses hosting; equal preference keeps registry order. Distinct versions prove preference does not unexpectedly alter version semantics. Auth/malformed failures preserve current failure law.
* Pin success/miss/unknown alias, no fallback; conflicting transitive pins and conditional edges; path/git/override conflicts; global/project alias scope.
* Fake HTTP/Git backends panic on excluded destination across probe, ls-remote, dependency reads, redirect, remap, mirror and canonical retry. Allowed mirror can succeed without forbidden canonical contact. Include URL parser SCP/ports/custom hosts and redirects.
* Full closure plan preserves manifest/lock/slots/agent files/home/global config/store exactly under literal no-write mode. If a cache-enabled mode exists, assert only explicitly declared effects and report them. Exercise case-c migration, cold cache, in-place packages, conditional deps and embedded sources.
* Commit unknown before acquisition remains unknown; acquired commit and fetched_from are preserved. Mutable-ref drift between plan/apply refuses or replans; canonical identity does not mutate to mirror URL.
* App inventory backward read produces unknown; fresh source survives install/list/describe/update/replacement/rollback/export/import/resume without reconstruction from current registries. Plans and real events share generated field semantics.
* Report selected/observed/not-probed/excluded distinctions; no false 'older' or 'unreachable' for an uncontacted source. Secrets never enter URLs or reports.

## 222-task overlap (map evidence, not task acceptance)

`architecture/TASKS.md` links M-16-B.1 strict operation/capability contracts and M-16-B.2 entity/snapshot/source-location views; M-16-C.2 retained outcomes and M-16-C.2.2 direct package effects; M-16-C.4 typed input/admission adapters; M-17-A.2 legacy/admin partial-effect truth and M-17-A.4 schema/model coverage. These are integration seams for shared plan/source responses. M-13-B.1/B.2 cover source policy/lifecycle semantics; M-15-B.2 future realization supply and M-15-K.3 independent egress boundary overlap broader reuse but are not delivered by source exclusion. F05 and M-01 are document address/source selection, not registry choice; do not claim them as fulfillment of need 1. Existing B-222 app portability is relevant migration coverage, not a new task. Root must bind any accepted work into its exact plan; this audit does not adjust the 222 denominator.
