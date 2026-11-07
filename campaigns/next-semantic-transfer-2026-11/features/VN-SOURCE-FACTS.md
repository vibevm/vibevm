# VN-SOURCE-FACTS — shared package-source evidence

Accepted scope: one source-evidence vocabulary from existing acquisition owners into application plans, inventory, descriptions, retained work and existing lock projections. This satisfies the four explicit obligations of VIBE-NEEDS2 (qualified Linux/source evidence); it is a foundation, not completion, for needs1/3 and does not complete an additional old113 task. The later exact-requirement audit corrected the earlier overly broad partial label; see research/vibe-needs-02-acceptance-audit.md.

## User-visible behavior

- Global application plan/list expose `source_evidence`; ordinary and common-configuration surfaces use the same meaning.
- Describe separates the source of the currently acquired authored package (`source_evidence`) from the matching installed record (`installed_source`). An inventory query does not reconstruct history from current registry settings.
- Captured source survives supported update/replacement/rollback, pending root/child resume and import continuation. Fresh portable import records the new acquisition; skipping an installed application retains its original record.
- Fields carry known registry alias, canonical URL/ref, observed commit, selection kind, redirect/differing acquisition locator, read-route class and actual store-reuse observation. Missing fields mean unknown. Credentials are redacted through the existing sanitizer; display fields are not resolver identity or authority.
- `git_repository` does not mean network contact or fresh download. Package-source evidence does not describe arbitrary installer payload downloads.
- Explicit local registry and embedded discovery remain distinct. Their established precedence is unchanged. Store-backed reads preserve recorded source kind or unknown; the shared lock writer no longer invents Registry from empty route flags.

## Compatibility and formats

Index protocol5 (application schema epoch4) retains frozen3/4 readers and minimum compatible writer. Managed copied proxies are admitted using the exact captured image and really decode3/4/5. Steps protocol3 and import-work protocol3 retain exact-key evidence with frozen earlier readers and minimum writers; existing state owners are reused, with no second inventory or new sidecar.

Strict durable readers remain strict; CLI evidence uses the established permissive reader projection. Frozen format IDs retain their bindings. The existing optional lock source_kind field changes its unknown/known handling; no new vibe.toml grammar or lock epoch is introduced. FMT-044 records the precise changes.

Plan acquisition can read/fetch packages and write cache/configuration. It does not execute installer steps, but it is not the promised no-write project preview. Source descriptions and schema language now say so explicitly.

## Evidence and qualification

Frozen candidate:150 source hashes and76 evidence hashes in the local acceptance archive. One subsequent four-line specification annotation correction restores the old full_store::validate trace edge while keeping encode's new edge; executable behavior did not change.

Passed controls: acquisition7; actual lock write/read1; application library221; wire corpora10; source CLI6; actual copied proxy3; import/portable/resume CLI11+8+4; embedded caller1; source doctests3; application doctests72. Strict Clippy passes. Nine Conform scopes have0new findings while retaining32 existing in-source suppressed findings; no baseline or suppression weakened. Earlier failed fixtures and the first unsuccessful integration runs are qualified separately in the handoff; one early overwritten app-failure log does not survive and is not presented as preserved evidence.

Independent compatibility, acquisition and retained-work reviews found no remaining blocker in their named scopes. Root verifies final specification/code mapping, source seals and committed blobs separately. Linux CLI fixtures do not claim native Windows/macOS execution or arbitrary crash/power-loss coverage.

## Engineering findings

The common registry layer already knew much of the source information; app conversion discarded it. A new resolver would not have fixed that boundary. Integration also exposed a dropped named-local-registry alias, a proxy capability advertisement without its actual decoder, and child receipt transfer needing evidence ownership to follow its existing operation ownership. Correct fixes belong to those owners, not parallel adapters that fabricate information later.

Tests must assert the intended lifecycle state. A finalized finish failure is not the same as retained pending work. The final resume control uses the existing pending-work mechanism and proves old records remain unknown after current registry settings change.

## Remaining work

Project install:plan still has a resolution-only observer and a `packages`/`plans` producer/schema mismatch. Project no-write planning, registry pins, hosting preference/exclusion, candidate reasons and hosting classification remain separate requirements. Historical commit/redirect information is not blindly copied into a current store-read route. The feature does not infer unavailable historical facts, authenticate publishers, change source selection or complete the prototype transfer campaign.
