# A2/A3 target decisions after A1

Design approved for implementation; no A2/A3 runtime acceptance yet. Root verified all51 pinned audit bindings and both complete original task objects in source-a23-review.json. Main baseline264b07466; prototype7fe6376c.

## A2: one ordinary grammar, one real export join

The existing progress-core frontend is the normative Vibe grammar. Ordinary CommonMark needs its own standard grammar adapter; disabling a few fact markers in that scanner would not implement CommonMark. Use exact pulldown-cmark0.13.6 with default optional HTML/CLI features disabled and tables enabled explicitly. The prototype used0.13.4. Upstream published [security backports in0.13.5](https://github.com/pulldown-cmark/pulldown-cmark/releases/tag/v0.13.5) and [a nested-image wikilink complexity fix in0.13.6](https://github.com/pulldown-cmark/pulldown-cmark/releases/tag/v0.13.6); the target chooses the maintained patch rather than requiring prototype dependency equality. No HTML rendering, URL callbacks or manifest discovery in this parser. Existing native Vibe parsing remains its own unchanged owner.

Represent an inert heading/section/fence/link view of the exact source. Preserve UTF8/CRLF offsets; visible heading text has deterministic whitespace normalization. Test headings inside lists/quotes, their container ends, image alt text, inline code and setext forms instead of inheriting the prototype's unexplored container behavior. Minimal container-aware outline is sufficient; a full HTML AST/renderer is not required. A source slice is not guaranteed to render independently with the same surrounding Markdown context.

Node indexes are local to one document and are not durable fact IDs. Use checked access and document-borrowed selection where useful; bounds checking alone cannot prove an index came from that document. The host pairs a selected node with existing SourceFileWitness. No parser-local second content-hash scheme or global mutable identity counter.

Core owns strict SourceExport name/file/heading declaration and duplicate names. Specdoc owns only heading structure/resolution; facts owns the actual descriptor-selected captured join and existing source witness. Package source_exports uses Option presence like sources: either key's presence opts into explicit policy, including[]. Export-only metadata grants no implicit semantic source and must produce UnselectedSource for a missing descriptor, never legacy suffix fallback. One PackageMeta explicit-policy predicate governs every consumer; project fields are unchanged. A2 must prove an actual installed Mixed captured document→named export→zero normative facts, not just a parser and metadata field.

Ordinary structure or export failure keeps raw bytes available and cannot invalidate unrelated native facts. Duplicate siblings are ambiguous; exact parent heading paths distinguish equal names under different parents. No ordinal/first-match resolution or implicit Vibe interpretation.

## A3 responsibilities retained for the next joined implementation

A3 includes raw preservation plus actual install/derive/read roundtrip and the first selected installed boot-reader map. Those cannot be shifted wholesale to A4. General lifecycle-agent, citation, query, progress, skill and project-source rollout remains A4 where the original contract places it.

Use existing quick-xml for optional non-resolving XML inspection. A clearly named root/count/depth summary can satisfy the useful optional structure boundary; do not invent a DOM without a consumer need or call the summary a complete validating XML tree. Explicit structural budgets may return unavailable structure, but never cap or hide raw retrieval. Preserve A1 hardlink-safe reads, capture/recheck and permission behavior. Do not transplant prototype global manifest/source/map caps or state-reader single-link assumptions.

A3 must extend current converter/materializer/selected-reader owners and carry independently selected version/hash/format into held output reads. Only selected native source can claim semantic addresses; raw same-stem resources coexist unless an actual output/address collision exists. Reject unsupported declared-native conversion honestly; later CopiedUnconverted/replay machinery is not an automatic prerequisite. Converter recipe compatibility needs evidence from the actual emitted-byte/input changes before deciding a bump; no silent unrelated recipe change is approved here.

## Execution split

migration_surfaces owns the pure CommonMark adapter, specdoc registration/dependency and its PROP045 grammar section. audit_source_foundation owns core export grammar, shared policy predicate, facts export join, PROP072 and FMT028. Cargo/Conform handoffs and actual dependency freezes are explicit; no global index generation until the joined candidate freezes. A3 is separately scoped after ordinary-view interfaces are concrete. No task completion follows from this design approval.
