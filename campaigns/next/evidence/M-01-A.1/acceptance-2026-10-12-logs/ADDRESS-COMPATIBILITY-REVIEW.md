# M-01-A.1 address compatibility review — current public paths

2026-10-12. Supersedes the earlier partial-file review. All proposed resolver/corpus files now exist and were read. No builds/tests or source edits performed. Executor is still writing the candidate; reviewed hashes appear below. Root owns acceptance.

**Decision: one concrete reachable strict-value validation defect remains in the reviewed source: public legacy conversion accepts manually altered invalid anchor segments. Withdraw the earlier private export-name concern as a blocker; the actual public export path is protected by the incumbent validated declaration. No new framework or A.2/A.3 work is required to fix the anchor defect.**

## Concrete finding: public legacy fields bypass anchor validation

`SpecAddress` exposes `anchor: Vec<String>` publicly (`address.rs:43–57`). `FileResolver::convert_legacy_vibe_address` (`resolver/resource.rs:33`) accepts that value and passes it to `canonical_vibe`. The latter checks raw percent spelling, revision, doc path and selected authority, then resolves the source file; it does not validate anchors (`:40–69`). File resolution uses authority/doc path and does not inspect anchors. Finally `ResourceAddress::vibe` directly joins the supplied vector (`address/resource.rs:32–59`).

A concrete public-path counterexample using the existing `selected_fixture` in the corpus:

```rust
let (_temp, resolver) = selected_fixture();
let mut legacy = SpecAddress::parse(
    "spec://org.demo/guide/common/PROP-042#root"
).unwrap();
legacy.anchor = vec!["bad anchor".into()];
let canonical = resolver.convert_legacy_vibe_address(&legacy).unwrap();
// Source inspection predicts canonical.as_str() is:
// spec://org.demo/guide@1.0.0/common/PROP-042#bad anchor
```

This is an inferred result from the complete current call path, **not an executed failure**. `vec![String::new()]` similarly constructs a trailing `#`; `vec!["root.child".into()]` silently changes one invalid segment into two URI segments. These violate PROP-070's strict grammar and A.1's validated-construction requirement. Strict raw parsing is protected by the old parser, but explicit legacy conversion must handle the public-field ABI too.

**Minimum fix:** in `ResourceAddress::vibe`, validate each anchor segment with the existing parent `address.rs::is_valid_anchor_segment`; the child module can reuse that predicate without changing the legacy struct/parser ABI. Return the existing typed `InvalidCharacter` (or the established empty-component refusal for an empty element) before constructing the URI. Add one corpus case through `convert_legacy_vibe_address` using valid source/selection and mutated public anchor fields, including invalid whitespace, empty segment and embedded dot. Anchor existence/uniqueness remains outside this spelling check and belongs to the later reader.

## Export concern resolved by the actual caller

The public method changed during this review from `SourceExport` to **`PackageMeta`**. The last reviewed `canonical_export_resource` (`resolver/resource.rs:74–139`) checks selected-world availability, exact selected version, matching package group/name/version, then finds the URI name among `package_meta.source_exports` before invoking the crate-private string helper.

`vibe-core/src/manifest/package/source_export.rs:15–39,53–64` has private fields and validated `new`/serde conversion. Every reachable row enforces `[A-Za-z][A-Za-z0-9._-]*`, exact normalized file path and nonempty normalized heading path. The private helper's weaker name validator therefore does **not** create a reachable invalid-name path from this public API. No duplicated export grammar is required.

The supplied package metadata is a validated logical declaration with matching coordinate/version; it is not authenticated installed-manifest bytes. The API returns identity only and performs no resource physical read. That distinction is appropriate to A.1. The contract/docs should continue to avoid claiming captured source, matching CommonMark descriptor/media, physical containment, or resolved heading bytes from this logical construction alone. Those later owner checks are explicitly acknowledged by PROP-070/072.

## Compatibility and authority assessed

- New strict dependency construction calls `selected_authority` before `spec_root` or file resolution. No selected world yields `SelectedWorldRequired`; an absent coordinate yields `UnselectedPackage`; an explicit mismatch yields `WrongSelectedVersion`. It cannot fall into the legacy ambient newest-installed scan for dependencies.
- The self-coordinate branch remains unversioned and distinct; strict Vibe construction also requires the descriptor map. Absence of a descriptor does not route strict input through suffix classification.
- Explicit legacy conversion may use the incumbent undeclared-source compatibility route, but still requires selected dependency authority. It is clearly named and not silently used by strict `canonical_vibe_resource`.
- Legacy `FileResolver` no-selection behavior in `resolver.rs:391–415,450–496` is retained, matching PROP-035's newest-installed rule. No redesign of SpecAddress fields or old compiler/boot calling convention is introduced.
- Legacy `@=version` conversion strips only one `=` in the new authority helper. Strict input does not strip it. Current exact-version validation now reuses `vibe_core::VersionSpec` rather than the earlier semver implementation. This review does not infer a new core dependency or cycle; the core dependency predated A.1.
- Revision pins are refused before `without_pin` or source resolution; strict raw malformed revision gets a distinct grammar refusal. Path percent/empty/traversal spelling refuses before lookup.
- `ValidatedDescriptorMap` remains a logical mapping supplied by a host that owns physical checks. Its presence can establish the selected interpreted logical row; neither its constructor nor canonical identity claims byte capture. No arbitrary physical export service exists here.

## Current corpus coverage and limits

`address/m_01_a_1_cases.rs` defines five actual tests:

1. `selected_vibe_and_unique_abbreviated_stem_canonicalize`: descriptor-backed abbreviated Vibe ID, omitted/explicit selected version equivalence.
2. `legacy_conversion_is_explicit_and_selected`: single-equals conversion, strict refusal of equals, undotted authority, missing selection.
3. `refused_percent_empty_revision_and_wrong_version_are_distinct`: percent, empty path, wrong/malformed version, supported-but-unavailable revision and malformed revision.
4. `duplicate_stem_and_two_forms_refuse_without_first_match`: selected legacy conversion preserves `AmbiguousDoc` then `PairCollision`.
5. `named_export_uses_separate_scheme_and_selected_authority`: valid package export, wrong name, wrong selected version and foreign package metadata refusal.

These tests do not exercise the mutated public anchor counterexample. Add that targeted regression. No executed candidate receipt was supplied to this review, so source inspection is not a green test verdict. Existing baseline inventory reports four pre-edit exact cases passed; legacy no-selection newest-installed behavior remains an independent compatibility obligation, not something the new selection test proves.

Do not expand A.1 into portable component containment, OS qualification, physical source authentication or blanket caller migration. The task's safe stop permits address/corpus work before public query; its accepted strict values still need the one spelling fix above.

## Reviewed current source hashes

| File | SHA256 |
|---|---|
| `crates/vibe-spec/src/resolver/resource.rs` | `46b6135e5670c5a6f7a09951b2404e74158a9765e1bb5068b0b036feb2592dbe` |
| `crates/vibe-spec/src/address/resource.rs` | `f3a21c362c3242bc92c01b20295cffc1b441273806c78d83a41d300cc90686e5` |
| `crates/vibe-spec/src/address/m_01_a_1_cases.rs` | `d856574664e37e5a565fee680ab6569bf544ac74a1bd7cb4412da053364c9469` |
| `crates/vibe-spec/src/address.rs` | `658c6e7e0ce2b3517ac61efbc4da0263e67a706071d3fe603b841eb3d8a2f813` |
| `crates/vibe-core/src/manifest/package/source_export.rs` | `107dc5dac91094687058c847349e81b75ce93b699bea5b72a54192ccb558a7fc` |
| `vibevm/vibespecs/common/PROP-070-resource-resolution.xml` | `f970a858d282be1d66dbecd8ed28da61e985975c436dfa55f0414414da56d7b9` |

No previously missing candidate file remains missing. Hashes identify this review stage only; later executor changes need their own matching evidence.
