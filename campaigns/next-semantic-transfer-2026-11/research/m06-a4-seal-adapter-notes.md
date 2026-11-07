# A4 seal native-source adapter packet notes

Read-only preparation, 2026-11-07. No code/tests/Cargo/Git mutation. Active progress worker owns ground/common effects; do not edit those concurrently. Seal is a separate source/mutation adapter and is not completed by ground activation.

## Actual operation and gap

progress.rs dispatches Seal directly to progress/seal.rs::seal_cmd. It canonicalizes --path root, resolves one campaign, loads cache.json (unreadable cache refuses), then calls one() per explicitly named path. relative() currently canonicalizes the FILE path, one() observes ancestor policy, reads UTF8 text directly and parses progress_core::parse_document. Explicit/empty source policy currently refuses. Successful Recorded results mutate the in-memory Cache; after all paths, a single cache.touch/store persists it. Refused files do not stop processing later paths. Current output says sealed before that store occurs.

The pure progress_core::seal function never performs verification or creates verdicts. It requires an observed cache entry and verdicts for every addressable marker; missing verdict gives Unjudged, missing cache record Unobserved, matching content_hash plus processed_hash gives Current without timestamp change. Recorded updates content_hash, processed_hash and verified_at only. Preserve these human-judgment semantics and gate separation.

## Required selected read

The request is exactly the named files, not progress.toml glob expansion, whole-host inventory or automatic boot scan. However the cache key/projection must agree with the currently accepted progress grounding adapter. Reuse its narrow selected native read result after the active worker freezes: original logical/cache path, physical diagnostic path, declared media/interpretation, captured source bytes/projected text and retained recheck. If that seam is private, coordinate a small existing-owner extraction; do not duplicate a source world.

Original host input uses actual outer/registered-node policy and exact held read; native scan or explicit may be selected by this explicit request. Ordinary/CommonMark/XML-data/opaque and explicit-empty refuse native sealing even if literal markers appear or a stale cache has verdicts. A nested random manifest within an outer declared scope is not new authority; real registered node/package selection remains meaningful. Do not fabricate group for path-based ungrouped host input.

Installed input, if the command admits it, requires independent lock kind/group/name/version/hash and trusted selected format with accepted output→original mapping. Never treat a materialized slot as an original host or infer format from its record. Retain current precise unsupported boundary if this selected reader has not yet been supplied; record it as unfinished rather than fake support.

Do not canonicalize a declared FILE into a different source before policy selection: preserve lexical path relative to admitted caller root, then use held no-follow descendants. Caller-root aliases retain established behavior. Existing legacy path resolution compatibility can stay a separate branch; the new declared route must not hide a symlink by selecting its target's manifest instead.

## Hash meaning and XML

Seal must parse exactly the same canonical frontend text that ground/cache parsed. Markdown's text hash remains its existing parser content hash. XML (including arbitrary suffix) must use selected media and existing canonical projection before parse; raw original XML SHA is a different witness and cannot replace content_hash/processed_hash. The current raw read/parse path does not establish parity for XML; a zero-marker parse must not falsely vouch for cached native verdicts. Carry raw witness for guard checks, projected text for parser/cache hash, and existing XML projection diagnostics.

No new cache schema or evidence hash recipe is required if the same parser/projection owner is reused. If formerly raw legacy XML seals acquired a different processed_hash, correcting them to the shared projection is an intentional behavior correction requiring a compatibility note and discriminator, not a silent new digest algorithm. A repeated seal must retain the first timestamp when already current.

## Publication boundary without global transaction redesign

All parsing/source admission happens before seal::seal mutates a candidate cache entry. Retain successful declared-source observations until the actual cache store; an immediate post-parse recheck alone misses changes while later paths are processed. Recheck before the one flush. Preserve existing per-file failures/continuation for ordinary admission/Unjudged/Unobserved results, allowing independently valid files to seal as today.

A late retained-input failure must not persist that file's stale seal. Either remove its staged entry change while retaining other valid per-file results, or refuse the final flush with explicit wording that no pending seal was persisted; root should choose the minimal behavior consistent with the existing per-file law. Do not print a newly recorded seal as successfully durable if the final flush is then withheld. Existing late I/O failures remain reported; no all-files rollback/CAS framework is requested. Cache destination/root checks should reuse the actual command/cache writer constraints, not invent a new registry.

## Minimal perimeter and fixtures

Own only progress/seal.rs plus a narrow child if necessary and dedicated CLI tests; core seal.rs stays unchanged unless an actual parity defect requires it. Any shared read-adapter accessor is coordinated after current grounding freeze. Do not modify gate, regenerate verdicts, run tests on behalf of the human, or treat successful parsing as judgment.

Existing useful fixture: cli_host_sources.rs::progress_seal_refuses_nested_explicit_policy_without_changing_valid_verdict_cache seeds real Cache/verdicts and proves current refusal plus legacy positive. Preserve its raw-resource refusal; add native declared positives rather than deleting it. Core seal.rs has already-current/unjudged/hash tests; those are algorithm controls, not command source-admission proof.

Actual command discriminators:
1. Seed cache through the accepted projection/parser with human verdicts, change native declared XML under arbitrary suffix, run `vibe progress seal --path <root> --campaign <name> <file>`; processed_hash equals the SAME projected parser hash used by ground, source bytes unchanged. Repeat preserves verified_at.
2. Native Markdown scan/explicit and selected directory-default file work; ordinary raw directive-looking input refuses with cache unchanged, even if cached verdict names match. No fabricated coordinate.
3. Missing verdict, unobserved path and unreadable cache retain exact legacy outcomes. Mixed valid/invalid named paths preserve documented per-file continuation.
4. Retained policy/bytes/parent change between capture and cache flush refuses stale publication using a bounded existing test seam. Selected symlink must not be hidden by canonicalization. Do not add broad global hooks.
5. If installed support is advertised, actual transformed install plus independent binding supplies the same cache identity/projection; wrong format/hash/record refuses before cache mutation. Otherwise keep the limitation explicit.

Runtime was not run. Root must reconcile this packet with the progress worker's actual completed read seam before implementation dispatch. Full A4 remains open.
