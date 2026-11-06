# XML embed attribution — preexisting provenance-loss boundary and proposed owner fix

Read-only audit. No source edits, tests/builds, Git mutations or extra agents. G1's writer retains all current code. Specs read: PROP-035, PROP-036 and campaign F08-boot-attribution.md. Evidence/source hashes and HEAD comparisons: `/tmp/nst-xml-embed-attribution-evidence.json`. Compared HEAD `adfaa780cf11a1cd65c58e2771ec15b960aad3b1` and immutable prototype `7fe6376ccef7f321302a8f18610fcb0c9cc2686f` via read-only Git blobs.

## Finding and evidence boundary

This is a **preexisting loss of generated embed provenance through the XML pivot**, newly exposed by G1's real install/tree fixture; it is not evidence that G1 introduced XML escaping. The expander, compiler embed owner, XML preparation/emitter/validator, pivot parser/writer and tree scanners examined here are byte-identical to HEAD. G1's coverage module and new CLI fixture differ. The prototype has the same raw embed comment→Markdown pivot→XML writer path; no superior same-route preservation was found in those pinned bodies. No old binary was run, so historical runtime behavior is source inference, not a new measured RED.

`cli-second.log` reports actual public test exit101 at the embeds count assertion:0 versus1. `cli-artifacts-second/installed-STATIC.xml` has exactly one each of BETA/OUTER/INNER/SOURCE/ALPHA/UNCOVERED/UMBRELLA payloads. Generated embed markers are escaped inside paragraph text at lines33,36,40–41. Its nonempty valid tree.stdout contains4contributions, all embeds=[]. The earlier cli-artifacts XML is byte-identical, but its tree.stdout is EMPTY; it cannot support a tree JSON claim. I initially tried parsing that stale empty capture, observed a JSONDecodeError, then recorded the distinction explicitly. No artifact was rewritten.

F08's accepted claim was narrower and said so: its public fixture calls real FsSectionSource→expand_embeds, then adds a synthetic `vibe:static` wrapper (`tree_nested_attribution.rs:165–202`). It does not traverse install→Lane→from_markdown→to_xml. F08 repaired occurrence decoding/fence/framing, not compiler generation. Schema-valid tree JSON was never sufficient to prove this missing producer path.

## Actual data flow and two distinct losses

1. `vibe-spec/src/embed.rs:154–217` recursively expands actual directives and writes `<!-- embed: ... -->` / closer directly into a String. Public expand_embeds is a compatibility string API. The callback identifies source+ordinal+target edges, not the eventual placed output interval.
2. `compiler/embed.rs` passes that String through DocTree::parse. ClosureDocument (`compiler/ir.rs:305+`) has tree/aliases but no embed occurrence annotation. ClosureEdge::Embed stores graph relation/requested target, not nested occurrence placement, multiplicity in an emitted contribution or output boundaries.
3. `compiler/ir/lane.rs:44–89` has typed NormalOpen/Close framing, but an embedded occurrence remains inside LaneNode.body:String. Those existing normal markers cannot substitute for embedded subspans.
4. `compiler/emit.rs:236–265,276–290` flattens body chunks into Markdown, applies the line-preserving continued-heading adjustment, and calls vibe_specdoc::from_markdown. `md_in.rs:85–89` **drops BlockKind::Comment** as layout. A standalone comment block can therefore disappear entirely.
5. A marker within a paragraph is not that separate comment block. It enters Unit text; `xml_out_emit.rs:29+` escapes `<`, `>` and `&`. The actual G1 fixture is this second case. An XML unescape patch could not recover the first case and would promote ordinary authored examples in the second.
6. `static_xml.rs:77` renders only the prepared SpecDoc. `emit/validate/xml.rs:observe_contribution` compares parsed emitted XML with that already-lossy prepared document, so both paths agree on the omission/escaped prose. The witness currently has no independent generated embed occurrence set to check.
7. `tree/artifacts/embeds.rs:control/scan` correctly accepts actual standalone reserved comments outside the canonical fence mask. Escaped paragraph text is not a control. `markers.rs` already owns c1 static provenance decoding; it must not be changed to infer embeds by unescaping arbitrary XML text.

The standalone-comment loss is source-proven in this audit, not separately executed here. The paragraph-escaped loss is present in the saved real second-run artifact.

## Smallest correct typed carrier

Add a richer INTERNAL expansion result in the existing embed owner, while retaining public `expand_embeds(...) -> String` as a compatibility renderer:

- expanded body text/lines;
- ordered `EmbedOccurrence` records minted by successful recursive expansion, with a stable operation-local occurrence identity/path, canonical requested target, parent occurrence and half-open expanded-body line-gap interval;
- original directive/source locator retained separately from mutable output placement.

These records come from the recursion stack and output append positions, never from rediscovering marker-looking text. Repeated equal addresses remain distinct occurrences. A graph edge is not an occurrence. No random sentinel, placeholder roundtrip, comment-pattern authority or new global source map/store is needed.

The compiler's richer path carries this body/annotation pair through ClosureDocument→LinkOccurrence→LaneNode. The compatibility string renderer can spell today's legacy pairs from those same records. Built-in compiler output must instead render its generated framing from typed records, separately from ordinary authored text. Occurrence IDs are compiler bookkeeping, not a claim of source authenticity and not necessarily an on-disk field.

### Built-in transformations and offsets

| Stage | Concrete treatment |
| --- | --- |
| Source merge | The canonical pipeline merges BEFORE embed. `compiler/merge.rs::fold_root` may reorder/remove lines, but generated embed annotations have not yet been minted. Expand over the actual merged tree; do not pretend old source offsets survive fold. |
| Recursive embed | Compose child occurrence intervals while appending the child's actual expanded text. Parent/child/order derive from recursion, including repeated addresses. Canonical directive/fence owner decides which source tokens are actual directives. |
| Qualify/aliases | `compiler/qualify.rs:309–321,396–412` and `qualify.rs:77–134` rewrite within lines and reparse the tree. Built-in line counts remain stable; line-gap intervals can be carried after an executable count/map check. Byte offsets cannot be carried unchanged. |
| Link and continued headings | `compiler/link.rs:402–418` and `emit/continued_headings.rs` likewise preserve line boundaries while rewriting labels. Preserve/check gap maps. Do not derive occurrence identity from changed marker strings. |
| Normal once coverage | G1 `link/normal_coverage.rs::body` already computes a union of original line spans before deletion. Return the retained line-range map alongside the new body. Use it to remap surviving annotations; do not compute offsets from already-shortened text. |
| Assemble | Copy the enriched linked body into the existing LaneNode/closed chunk stream; preserve normal contribution ownership, ordering and empty shared-owner contribution behavior. No second compile/link pass. |

**Suppression law:** fully suppressed payload occurrence loses both boundaries and its fully suppressed descendants; an empty shared contribution must not retain an attribution ghost. For a partially retained occurrence, project only its retained fragments, retain ordering/nesting and place boundaries around the surviving emitted portion; fully suppressed nested occurrences disappear. If a deletion would create unrepresentable/crossing placement, existing validation must refuse rather than invent a span. Intrinsically empty successful embed is different from content removed by once coverage; retain an explicit disposition so `body.is_empty()` does not confuse them. Coverage identity must not incorporate fresh occurrence IDs and defeat once behavior.

## XML projection is the remaining real implementation seam

Existing SpecDoc has no provenance/source-position slots (`specdoc/doc.rs:38+,Block`); comment-only blocks intentionally disappear. Simply adding ranges to LaneIr and still passing a plain String to from_markdown does not finish the fix.

Add a narrow **annotation-aware projection/rendering entry in vibe-specdoc**, using its actual progress-core block/unit mapping. It accepts explicit generated boundary annotations from the compiler, maps them to semantic document positions, and returns a prepared annotated XML document/tape. Ordinary from_markdown/to_xml behavior stays unchanged. Do not add an XML raw-text bypass or a second Markdown parser.

Paragraph-internal boundaries are mandatory: an embed adjacent to prose can begin/end inside one semantic paragraph. The projection must retain those positions, not relocate every boundary to the nearest heading/block, drop it or split the paragraph arbitrarily. Generated XML comments and their line framing are emitted from annotation events. Any inserted formatting newlines must have an explicit annotated-artifact observation rule; the ordinary `from_xml(to_xml(doc)) == doc` text-preservation contract cannot be silently weakened. The current plain SpecDoc API supplies no such mapping, so this is genuine work, not an assumed existing capability.

Recommended observation split: validate the projected semantic document AND independently the exact ordered generated occurrence tape against the pre-emission witness, including body association. The annotated compiler tape may own framing whitespace, but the general prose/XML reader must not acquire an “unescape controls” rule. Preserve payload text and fact/section semantics with roundtrip assertions, especially the no-blank-line paragraph case. Existing emission byte accounting (`emit/attribution.rs`) must use the same annotated rendering as the emitter, not the old unannotated to_xml length.

## Wire, transforms and invalidation are part of the atom

Native/custom transforms can replace bodies arbitrarily; built-in line preservation is not their contract. Bind annotations to their exact body/digest. Existing Closure/Link/Lane intrinsic and transition validators must check range bounds, stable identity, balanced/noncrossing ancestry, occurrence order and body association. A transform changing a body must return a valid mapped annotation result or explicitly invalidate/rebuild the applicable derived state through the existing manager sequence. Stale annotations cannot silently survive and cannot be guessed back from raw text. A legitimate transform suppression must carry that removal, not forge/reassign origin.

The actual serialized boundary exists: `compiler/wire/{closure,lane}.rs`, link typestate conversion and generated compiler-ir epoch1 (`formats/REGISTRY.toml:758`, schema `schemas/compiler_ir/e1/ir.jtd.json`). Therefore this is NOT just a private Rust field addition. Root must choose compatible compiler-IR wire evolution; update the canonical schema/vocabulary/generator, strict readers and hostile/roundtrip fixtures together. Old transform replies cannot be accepted as provenance-preserving merely because they omit the new field. Existing digest owners (`link` input/result, lane/chunks, pre-emission witness) must bind the new annotation meaning. No new durable sidecar is needed.

## On-disk format and literal law

Tree JSON v1 can retain the existing EmbedSpan address/start_line/end_line shape and opening-order semantics. Compiler-internal occurrence IDs need not leak into it. XML-generated comment payloads should use the existing c1 codec so valid address spelling containing XML-comment-hostile `--` remains legal. That requires a bounded tree embed reader extension to decode canonical c1 embed payloads, while retaining scoped legacy raw-pair compatibility. It is an on-disk generated framing extension to document/test, not a reason to accept arbitrary XML prose as controls.

Generated and authored lookalikes must remain separate in BOTH targets:

- XML: ordinary authored text stays text/escaped content or ordinary comment-layout under its existing parser; only supplied generated annotations become provenance events. Fenced/inline/CDATA-looking examples must not mint annotation records.
- Markdown compiler output: typed events render canonical real controls. Authored standalone reserved-looking text must not accidentally become a generated occurrence. A target-local literal rendering policy is needed for those reserved source spellings (for example escaping the control opener), using the existing fence owner; fenced/inline examples retain their established literal behavior. This policy must be explicit because raw-byte identity and “same bytes are not a control” cannot both hold for a reserved standalone marker on committed tape.
- The read-only tree decoder continues recognizing existing committed legacy reserved controls under F08's framing/fence law. This is compatibility over editable artifacts, not authentication that every old comment was really compiler-generated. Do not promise to distinguish two byte-identical legacy tape comments by origin it never recorded.

## Executable implementation order and discriminators

First test-first atom: extend the real G1 install→repeat/unit reuse→tree→reinstall fixture with both no-blank-line and standalone-comment-block layouts, retaining nested outer/inner exact counts/lines, once bodies, source immutability and empty shared-owner contribution. Saved second-run output is existing RED evidence for one layout; run the standalone-drop case before claiming its runtime reproduction. F08 synthetic scanner fixtures remain as reader controls, not substitutes for this route.

Then implement ONE complete typed path: internal expansion carrier/compatibility renderer; Closure→Link→Lane mapping with once suppression; annotation-aware specdoc XML projection/render/observation; c1 tree decode; wire/digest/transform validation. Intermediate internal carrier tests are not completion until actual installed XML is attributed correctly. Ownership spans the existing embed/compiler IR+passes, specdoc projection/writer, compiler wire/generated schema and tree decoder; this is a bounded cross-layer correction, not a regex-only G1 patch.

Required discriminators:

1. XML/Markdown real compile with nested and repeated equal-address embeds; inclusive file-relative emitted boundaries in opening order, valid tree v1.
2. Generated markers with/without adjacent blank lines; paragraph-internal open/close plus heading-crossing nested content; payload/fact semantics retained, not merely marker count.
3. Generated embed beside fenced and inline literal raw/c1/escaped lookalikes, including an unmatched literal example; source examples never create provenance or new malformed-control errors.
4. Whole occurrence suppression, partially retained parent with removed child, multiple surviving fragments, and genuine empty embed; no orphan closer/ghost span; simple contribution semantics unchanged.
5. Qualify/alias/link length changes and repeated-heading XML continuation preserve occurrence association; scope/range maps checked.
6. Same annotation with stale body, crossed intervals, forged parent/target/order or stripped annotation in a custom transform reply is rejected by existing validation/wire gates; valid mapped transform roundtrips.
7. XML-comment-hostile target characters use codec and recover exact address; malformed actual canonical controls still fail line-located; existing legacy raw reader/F08 fence behavior retained.
8. Emission observer validates provenance independently of prepared semantic document and reconciles byte accounting with the annotated writer.

No implementation is started by this report. Root must approve the carrier, annotated paragraph/text framing contract, compiler-IR evolution and owner allocation. The evidence does not justify calling G1 a new escaping regression or unescaping saved XML into counterfeit spans.
