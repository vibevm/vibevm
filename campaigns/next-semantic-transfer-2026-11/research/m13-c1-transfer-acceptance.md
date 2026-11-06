# M-13-C.1 — pure migration planner acceptance review

**Accepted** in `f56a4cba60e502416d194f6fc3677e51a1ad2e45`. Root verified all9committed blobs and all27 audit source bindings, inspected production/tests/specification, and independently recomputed the exact token/hash vector. Final19behaviour tests/10doctests, scoped strict Clippy/fmt/Conform and specmap/check pass;9097units/5268items/4430edges,0orphans/0unresolvedhost,44existing warnings. Original113 partition:5full/24partial/18not-transferred/66unknown.

## Original boundary and user benefit

A caller can ask for an inspectable migration proposal without rewriting an authored file. The real initial transition changes the legacy `package.materialization` value `snapshot` to current `copy`. Comments, key ordering, unrelated selections, UTF-8 and line endings outside that token are preserved exactly. Current absent-epoch and epoch1 profiles have explicit byte-identical no-op plans. This is a pure library boundary, not a new CLI command or permission to apply a proposal.

Original C1 explicitly defers SourceDescriptor and feature converters until their own reader atoms. Their final-prototype branches, plus Redbook recipes and saved apply/transport, remain campaign obligations. They are neither stubbed nor counted as transferred by this atom.

| Original requirement | Candidate evidence reviewed by root |
|---|---|
| Frozen recognized source grammar and deterministic registered transition; unknown input refuses | Three immutable named profiles and three explicit allowed edges; finite key/type/vocabulary checking independent of current Manifest parser. Unknown profile/recipe and incompatible edge have separate typed reasons. No product-version or missing-epoch inference. |
| Syntax-preserving representation | Immutable TOML syntax spans feed exact original-coordinate replay. Only selected token bytes change; no full-document serialization. Tests compare complete expected output across inline/header tables, quote families, escaped strings, multiline initial newline, UTF-8 and CRLF. |
| Validated in-memory before/after and typed NeedsInput | Compatible incomplete selection returns named choices even when one choice remains. Complete selection parses/checks source, replays edits, checks frozen target and uses current Manifest validation. No provider, filesystem, cache or lock owner in this call path. |
| Genuine legacy transform and current no-op | Synthetic package fixtures exercise real snapshot→copy; each current profile produces no edits and identical bytes. Legacy is not silently accepted as current. Current output is an explicit planning fixed point. |
| Unknown version, ambiguity and malformed syntax before effects | Unknown/incompatible supplied selection refuses; incomplete compatible selection returns NeedsInput before parsing; malformed syntax with complete selection refuses. No implicit choice or LLM path. |
| Repeat exact input gives exact output/diff identity | Equality/fixed-point/comment-change tests and independent literal golden vector. Root independently recomputed all3 expected hashes and the selected byte range using Python hashlib/u64be framing; no production hashing helper used. |
| Safe stop is pure executable planner | Input is borrowed text, output is owned proposal/refusal; public proposal fields are data and not authenticated write authority. Actual saved apply remains separate. |

## Critical findings addressed

The prototype's canonical historical-rehearsal statement overclaims its evidence: its retained historical scalar fixture deliberately refuses, while successful package examples are synthetic. Target specification and tests state that distinction explicitly; broad real-file migration qualification is not claimed.

The generic replay error in the prototype always named package.materialization. Target replay uses the edit's actual field; independent negative cases use change.schema and check the diagnostic. Hash framing has one owner rather than duplicated main/Redbook code. Replay independently rejects stale token text, reversed/out-of-range/overlapping/out-of-order ranges, split UTF-8, empty insertions and repeated insertion positions.

The final prototype feature recipe writes typed `[features]`, whereas accepted main uses `[configuration]`. It is deliberately not imported as a competing reader. Later feature migration must preserve useful intent through the accepted target owner. Source recipes likewise await their actual descriptor reader. This is the separation required by the original C1 contract, not abandonment of those later tasks.

## Formats

No authored vibe.toml grammar, lock/wire epoch or vibe install -g syntax changes here. The library adds named finite migration profiles and in-memory result types. A later apply path must revalidate the proposal and source before writing; current legacy materialization admission is unchanged. Explicit unknown profile revisions refuse. See FORMAT-CHANGES.md FMT-023.
