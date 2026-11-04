# F05 — Truthful source selection

Source review: four M01 commits and the surviving pinned routes, research/history-m01-address-review.md. Target reproduction: current-source public tests at0e98486da,10cases with6controls/characterizations and4intended failures;21observations. Root design f6856d522 explicitly resolves the group/version compatibility choice rather than copying group-blind prototype behavior. Actual initial XML control assumption was corrected; XML global duplicate-ID validation was already working.

## Implemented candidate

Requested Markdown tree paths select uniquely in each actual search scope; missing and ambiguous differ. Unrelated duplicates and qualified leaves under different parents remain valid. The additive DocTree::resolve_unique_path API and AnchorAmbiguity error are used at filesystem source/embedding/prompt address boundaries; low-level first-match methods retain their separate contract. XML validation and held source-byte observation remain unchanged.

Patterns use the fixed group and actual point-lookup world. Explicitly selected mode enumerates only selected dependency roots, preserving exact versions and mismatch diagnostics. Ordinary mode uses the existing held/no-follow layout and newest comparator; an explicit pin is honored instead of erased. Only true absence removes a member. Broken/ambiguous matched sources refuse. A private InstalledSlots lookup context shares the existing owner; obsolete pathname-only lookup/flattening helpers were removed. Public canonical_doc_path remains unchanged.

No manifest/schema/dependency changes or ResolveError variant. FORMAT-CHANGES FMT-010 states selection/error and additive Rust API effects. Versionless ordinary addresses retain their existing later-lookup freshness; no whole-pattern/closure snapshot or source authentication is claimed. Portable sibling aliases and the complete source-query/exports/proof service remain separate obligations.

## Evidence and acceptance boundary

Frozen candidate12files,264distinct selected tests/docs plus clippy/fmt/diff passed. Root reviewed production changes and test assertions; requested explicit assertions of both selected bodies before/after unrelated disk mutation, then re-reviewed the amended17case suite and unchanged production hashes. All12files matched exact reviewed SHA256 and unchanged main source base before integration;41evidence files archived. Main public17/17 and actual lifecycle prompt14/14 passed after integration. Integrated acceptance: fmt, both scoped conform gates (0new; lifecycle2existing ambient observations) and specmap --check passed. Map9062units/4740items/3980edges,0blocking findings and43known warnings. All12source hashes match. This closes the bounded truthful-selection increment; no broad platform/full-source-query completion is inferred.
