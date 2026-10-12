# M-01-A.1 candidate for Root review

2026-10-12. HEAD `20e952e331ad6695971e404dc253f81526f11013`. Candidate is unstaged. Root owns acceptance, incident ledger, and atomic commit. `FIXED-ISSUES.md` is Root's separate uncommitted edit; this candidate does not edit it.

## Bounded source result

- Created `vibevm/vibespecs/common/PROP-070-resource-resolution.xml` with seven checked anchors for strict Vibe `spec://` documents, distinct named CommonMark `resource://` exports, selected authority, revision refusal, explicit legacy conversion, and typed refusal corpus.
- Added `crates/vibe-spec/src/address/resource.rs` and registered it from `address.rs`/`lib.rs`. `ResourceAddress` is a validated logical identity with a private URI, separate kind, and typed errors. It rejects percent encoding, empty/dot segments, physical Vibe serialization suffixes, malformed authority/version, and unsupported revision. A selected dependency's canonical spelling includes its exact selected version; self remains unversioned. This is not a file handle or captured bytes.
- Added `crates/vibe-spec/src/resolver/resource.rs`, registered from `resolver.rs`. Strict Vibe construction needs a selected world, validated Vibe descriptor map and unique incumbent ID lookup. Explicit `SpecAddress` conversion handles legacy `@=exact` and a uniquely witnessed descriptive physical stem, but does not accept undotted host, missing selection, revision pins, or raw same-stem resources. `resolver/source_map.rs::owns_vibe_output` checks that legacy physical-stem fallback actually belongs to the selected Vibe map.
- Named export construction requires matching selected package identity/version plus the named export in parsed `PackageMeta`; it does not resolve export bytes or headings. `crates/vibe-spec/src/address/m_01_a_1_cases.rs` contains seven selected positive/refusal cases. All new Rust files are under 600 lines; no Cargo manifest or lockfile change remains.

## Deciding evidence

The four original exact baseline cases passed 1/1 each before edits (`baseline-*.log`). `address-family-after.log` records 7/7 new cases, 0 ignored, exit 0. Two affected incumbent cases passed 1/1 each, 0 ignored, exit 0 (`post-anchor-descriptor_map_excludes_same_stem_raw_resources_without_changing_legacy_pairing.log`, `post-anchor-a_selected_world_enforces_the_exact_locked_version.log`). All used `cargo +nightly test -p vibe-spec --lib --locked --offline`, private `CARGO_TARGET_DIR=/fast/git/v/.build/c4-first-agent-target`.

Independent source review (`ADDRESS-COMPATIBILITY-REVIEW.md`) found that public `SpecAddress.anchor` fields could be mutated after parsing. `anchor-before.log`: exact new regression failed 0/1, exit 101, accepting `bad anchor`. `address/resource.rs` now validates each segment with incumbent `is_valid_anchor_segment`; empty is `EmptyComponent`, invalid grammar is `InvalidCharacter`. `anchor-final.log`: exact same case 1/1, exit 0. `address-family-after.log` also includes that fix. Source review's earlier export-name concern was discharged against the actual validated `PackageMeta` path.

`package-fmt-final.log`: `cargo +nightly fmt -p vibe-spec -- --check` exit 0. `git diff --check` exit 0. `xml-anchors.log`: XML parse and all seven anchors present, exit 0. Final relevant new-file hashes: address resource `043256e54305d72aa2573f46c5d6321caf2e0165f62ea6635842c81079d08e22`, address cases `ec9b890f3cc967ebe0ff89e26054c8d1c73acb9efe32d7d5847f4adce4b8a96f`, resolver resource `95bd7332206742218c8d7ee96ac091bba7442fe5722227207dd642dcd40b70b0`, resolver source map `e09a123277524753a84f45f0cb307d5faac47ac6dd72b8e409d10709ce6edfc6`, PROP-070 `d21a0f224e765b25c774736638d5acd2b0eda6bee781f3ddef5c022dba0297dc`.

## Actual failed attempts and remaining limits

- `selected-vibe-first.log`: first `--locked` compile failed after I added a semver workspace dependency requiring an out-of-perimeter lockfile update. I removed that dependency and used existing `vibe_core::VersionSpec`; `selected-vibe-compile.log` and later exact tests passed. No Cargo diff remains.
- `anchor-before.log`: real 0/1 defect, fixed above.
- `cargo-fmt.log`: `cargo fmt --all -- --check` exit 1 on 15 existing unrelated hunks outside `vibe-spec`. This is not a green full-workspace format gate; no unrelated source was changed. The package-scoped format check is green.
- An ad hoc XML check first invoked unavailable `python` (exit 127); `python3` rerun in `xml-anchors.log` passed. This was an operator command error, not an XML failure.
- `ResourceAddress` proves logical identity and selected declaration only. It does not claim physical containment, authentic manifest bytes, readable source, actual anchor/heading existence, or successful content reads. Those remain A.2/A.3 and later reader work. Incumbent legacy resolver behavior remains for existing compiler/boot callers; it is not a strict-resource fallback.
- No Windows, broad suite, mutation, release, public Git, or stopped Zap+Lean work was run. `vibe specmap` has no read-only check mode and would write an out-of-perimeter map; Root should decide its integration run after source review of the two new specmark edges. No claim of that gate being green.

## Next action

Root reviews the bounded source/contract and evidence, then integrates the accepted atom and incident ledger in an atomic Conventional Commit. If review finds a concrete A.1 counterexample, keep correction in these incumbent owners with an exact nonzero case; do not start A.2 implicitly.
