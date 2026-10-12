# M-01-C.3 trace locator and hash seam review

Read-only research, 2026-10-12. No code edits, builds, tests, Git operations, boot or stewardship reads. Findings describe the snapshot identified below; this is not acceptance of the executor's implementation.

## Observed boundary

- `core-ai-native-specmap/src/index.rs:27-31` currently emits schema **3**. Schema 2 introduced house-style paths and `spec_unit.file`; schema 3 introduced code-item span/fingerprint.
- `mdspec.rs:220-251` already provides source-neutral `doc_path`: strip specs root and `.md`/`.xml`, abbreviate PROP/FEAT descriptive stems. Together with `anchor`/`uri`, this is the existing logical locator. It is intentionally not an absolute path or a physical extension. PROP-070 VIBE-DOCUMENT-ADDRESS adds unique-resolution obligations; canonicalization alone does not prove uniqueness.
- `mdspec.rs:192-199,356-418` hashes the authored span (joined source lines); `xmlspec.rs:178-183,224,403-409` hashes a canonical Markdown projection of its internal XDoc/XSection. `lib.rs:65-76` normalizes line endings only. Therefore these hashes are neither raw-file hashes nor canonical SpecDoc IR node hashes. Equivalent authored Markdown with different blank-line/list-marker spelling can hash differently from XML projection. They must not be described as cross-form semantic identity.
- `generated/specmap/mod.rs:185 onward` and `schemas/specmap.jtd.json` require `file`, `line`, `content_hash`, `doc_path`, `anchor`; only CodeItem has `end_line`. SpecUnit line is a source-decoration value, not a guaranteed installed slot line. XML scanner internally knows section/document end lines (`xmlspec.rs:50,65`), but drops them from the wire unit; facts carry only the opening line. Host projection loader explicitly says XML diagnostics are projection-relative (`vibe-specdoc/src/load.rs:29-57,147-162`). These values cannot be rebadged as native materialized spans.
- SpecDoc itself has the proper semantics (`doc.rs:49-63,478-510`): title, sections, block kinds/order, unit content, fact requirements/status, bound fences; no file or line metadata. `xml_out.rs:81` is a deterministic backend over that IR. No existing canonical IR node-hash API was exposed in the named `vibe-specdoc/src/lib.rs`. Conversion identity is independently `specdoc/4`.
- There are two independent representations today: host SpecDoc and specmap's XDoc (`xmlspec.rs:43 onward`). The latter is not SpecDoc and must not be called canonical SpecDoc IR merely because it projects similar Markdown. `xmlspec/doc.rs:11` explicitly says the engine remains separable with parity tests.
- `index.rs:242-285` compares legacy `contentHash` directly to emit unbumped-hash drift; `vibe-trace/src/composition.rs:241-245` also directly compares it to Lean claim `unit_hash`. Silently substituting a new recipe changes both consumers. `fingerprint.rs` is code-token identity (`tok1`), unrelated to spec-node semantics and should stay untouched.

## Smallest bounded change suggested

1. Add a pure canonical-node API at SpecDoc's owner. Parse declared Markdown or Vibe XML to the same IR, select exactly one anchored document/section/fact, and hash a named, versioned deterministic encoding of that node. Include the node's semantic subtree and fact-bound fence where applicable. Exclude source path, extension, line, and layout. Reusing deterministic `to_xml` on a precisely defined node envelope is a small candidate, provided its recipe is named/versioned and node kind boundaries are explicit; do not hash Debug output or raw XML. This yields structural IR identity, not proof that arbitrary paraphrases have equal meaning.
2. Preserve existing logical `doc_path + anchor` rather than inventing another addressing grammar. At query time resolve that locator in the already selected slot, refuse ambiguity, parse the actually observed materialization, then return actual extension and native start/end range separately. A source-only path or projected line stays explicitly source/projection metadata. An unavailable native range stays unavailable, not a guessed range. Canonicalization must not silently choose the first duplicate anchor.
3. Use a new schema generation (next local specmap generation is 4) and a distinct canonical-node hash field/recipe, keeping schema-3 `content_hash` meaning intact. A hash-recipe prefix alone does not satisfy the task's schema-version requirement. Update the schema's real owning JTD and generated binding together. If host-produced enriched maps carry the new field, plain legacy scanners must not claim enrichment they cannot calculate. A narrow host augmentation/seam is preferable to duplicating the SpecDoc semantics inside XDoc or introducing an engine-to-host dependency implicitly.
4. Select the canonical field for cross-form comparisons only when both operands have the supported same recipe. Old schema/legacy content hashes remain legacy or unavailable for this comparison. `classify_drift` must not report a changed hash across incomparable recipes/schema generations as semantic drift. Keep composition's existing claim contract on its current hash until that consumer is explicitly versioned; changing `contentHash` in place would silently invalidate claims.
5. Keep raw observed digest, canonical node identity, and representation/output digest as separate typed fields. No source-membership or derivation strength is conferred by canonical equality; that is the other worker's proof owner.

This is a recommendation, not an implementation claim. Root subsequently authorized exact `crates/vibe-trace/src/foreign.rs`. Its `resolve_foreign` loads a carried map and slot, while `try_foreign` sends only the carried map to `render_foreign`. Rendering passes the map unchanged to the engine and adds only a carried-map provenance cue; it does not resolve actual spec materialization paths/ranges. `parse_map` is serde deserialization with no schema-version validation. This is the concrete consumer seam requiring location resolution or an explicitly weaker carried-source presentation. Discovery separately picks the highest-version map-bearing slot (`reduce`), not a prepared selected-world slot, so the new strict source query should consume the already selected slot instead of reusing that discovery as authority. This is an observed difference, not a request to redesign all legacy explain discovery during C.3.

## One deciding positive/negative test pair

Positive: authored MD uses a noncanonical list marker and extra blank lines, with one anchored section and a fact plus its bound fence; equivalent Vibe XML parses to equal SpecDoc IR. Assert equal versioned canonical node hashes, unequal raw/output digests where expected, then install only XML and query through the same extensionless locator. Assert the returned path ends in the actual `.xml` and the native start/end range slices that XML anchor, not the authored MD lines or projected Markdown. Use the source capture's original MD path as a deliberate competing stale decoration.

Negative: keep locator and source-only decorations unchanged but change fact text (or bound fence text) in the installed materialization. Assert canonical node hash changes and source-only file/line plus old raw digest cannot produce a same-semantic or proven-location verdict. Also feed a schema-3 legacy hash versus the new field: refuse comparison/mark unavailable rather than infer semantic drift solely from recipe difference. These are proposed checks only; none were run.

## Snapshot SHA-256

Paths below are repository-relative. `engine/` abbreviates `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v1.1.0/crates/core-ai-native-specmap/src/`.

- `crates/vibe-specdoc/src/doc.rs`: `6ea28341467568037b9587394fdc36ee76021db4660fbe45b548b95c6a6ae41a`
- `crates/vibe-specdoc/src/load.rs`: `814e2be930dbecf36bce70ecbe48b8de9aeecbd02ba800c5ddc755537701c728`
- `crates/vibe-specdoc/src/lib.rs`: `fa7eb9d177e339361a3621b9faf8e6751bd2e920813258a546f1dc0a049ddd92`
- `crates/vibe-specdoc/src/xml_in.rs`: `cdca32d18aae1ff9142f72de9e567c7e4571e402b3de08723b49d8e8d828a5d2`
- `crates/vibe-specdoc/src/xml_out.rs`: `6f637efa54f56f7fb7a48224f9b726000dae3c02007a48f390e5696848e8b2e4`
- `crates/vibe-trace/src/composition.rs`: `28deb2e658b402e2e35433378eb1d0c16cff2d581f1590228fca18c117e5a079`
- `crates/vibe-trace/src/lib.rs`: `603ef54175a3ea7f4443f5411ae2184462bbf81bc56fec9f2a15ceda5916122e`
- `engine/lib.rs`: `73846811f1a54bf8e8aa3073aa44551cde1ee5108a896160143a993faca5e9b2`
- `engine/index.rs`: `7329984b45b10242f714f8a079dd74787f55e71ddd91b81e90b9336f08534ab1`
- `engine/mdspec.rs`: `2cc203d7cbd31aa934e2c6a2ce074cfefefdf06ba94c40b5cc34db36775b711f`
- `engine/xmlspec.rs`: `c2a13559f4109b8f7f13eab98015babc8af3b058da1b03abf66dc6ba80b7c952`
- `engine/xmlspec/doc.rs`: `e737145a42efe05e15662564b858286a085e173c1043412a6d99da1ddbda0978`
- `engine/generated/specmap/mod.rs`: `f32d3ece661f5acd708d486828dbaca6a2a1679e2d9aa95d94c13dbfa575ccad`
- `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v1.1.0/schemas/specmap.jtd.json`: `3a47f08375ed86c095e1dc2116c24f331e59055449cf85c23791445e08eebbcc`

- `crates/vibe-trace/src/foreign.rs`: `baa9148ba6ecf9beffcfe317424265578adfb5e6c121a1e00a1ba69d810eb25c`

## Candidate review follow-up (2026-10-12)

Read-only review of current implementation, no independent commands beyond file reads/hashes and this report append. No acceptance claim.

Observed implemented boundary: schema 4 introduces optional canonical_node and materialized_locator and documents legacy content_hash/file/line unchanged. canonical_node_hash uses actual SpecDoc IR projected through deterministic XML with recipe sha256-specdoc-node/1. Title, section, facts, carrier kind and adjacent bound fence are covered; source path/line are excluded. Selected trace computes its hash and native range from the same retained output bytes and resolves doc_path within prepared dependency descriptor captures. It does not call legacy highest-version discovery. The selected XML positive case preserves carried MD file/line/hash and adds an actual XML slot path/range.

Concrete findings:

1. **Materialized format is taken from slot mode, not actual output media.** membership.rs recorded_output assigns format: record.spec_format; canonical_node_of_output has no Mixed arm and selected_materialized.rs explicitly refuses Mixed. A valid selected Mixed slot holding an unchanged declared Markdown or XML Vibe document therefore cannot produce canonical identity/location. Deciding fixture: reuse trace fixture with SpecFormat::Mixed and unchanged DOC.md output; same doc_path+anchor should locate Markdown natively. Fix is to carry the actual descriptor/materialized format, retaining whole-slot mode separately. This is a format selection defect, not an authentication concern.

2. **Declared VibeDoc is admitted by semantic parser but rejected by range parser.** membership.rs admits XML VibeDoc using Vocabulary::Doc, then selected_materialized.rs calls engine xmlspec::native_anchor_range, whose closed XDoc reader accepts the Spec vocabulary only (the authorized xmlspec/doc.rs rejects unknown blocks). Deciding fixture: valid XML Doc with title id=root and a note kind=tip. Canonical hashing succeeds, native range lookup refuses the note. Either use the matching vocabulary-aware native span owner or explicitly restrict the public supported surface; silently claiming all admitted VibeDoc outputs work is wrong.

3. **Incumbent explain route remains unintegrated.** vibe-trace/lib.rs:154 explain still delegates installed targets to foreign::try_foreign, which renders a carried map unchanged. lib.rs only exports selected_materialized module; the inspected intended baseline route never invokes the new API. Thus tested behavior is a new explicit prepared-world entry point, not changed CLI/MCP legacy explain behavior. If C3 accepts that reusable API boundary, this is a stated integration limit; if actual incumbent explain is the required query-time locator caller, a callpath fixture must demonstrate its wiring. No all-channel coverage demand is made.

Evidence logs: trace-selected-initial.log reports 2/2 pass; workspace-canonical-initial.log 2/2 pass; specmap-native-ranges-canonical.log 3/3 pass; specmap-schema4-check-exact.log ends successful dev build without errors. These logs predate the current fourth engine @fact range test and some current canonical-node cases, so they do not validate those later additions. Source checks show duplicate XML anchors refuse and ordinary interpretation is rejected before semantic parsing; the provided trace fixture has no ambiguous selected-document case.

Reviewed snapshot hashes (current edits may advance):
- `crates/vibe-trace/src/selected_materialized.rs`: `8fb5cec5c14091aef1f2de826baff2f9dab61ee6deb369caa2ed962bc2f13000`
- `crates/vibe-trace/src/selected_materialized/m_01_c_3_cases.rs`: `322e16729f96d776d41366cd36587ecb16230823de1586ebde6e1f6d33e9669d`
- `crates/vibe-workspace/src/source_query/capture/continuity/membership.rs`: `28379593bcfc59b318662a6bbd9367a16c74ee6e60d604e837931805029228f9`
- `crates/vibe-specdoc/src/canonical_node.rs`: `8fa75d648312f243cda14bf1095400e469a0e936899b6b5b0307ae157f5a3f98`
- `crates/vibe-trace/src/lib.rs`: `e7d6dae7b2b884c680ee7b47a3a35e530449118ed7fbbfcc4152292df975c594`
- `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v1.1.0/crates/core-ai-native-specmap/src/index.rs`: `7b42aac602a4d9d9dec788ab0e163fb832410275b25b54fa687356c4b42777f7`
- `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v1.1.0/crates/core-ai-native-specmap/src/explain.rs`: `adf8a3a792b974f85bcf28aafe2be2ce5f2222017dd348e8ff1ffb286e169944`
- `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v1.1.0/crates/core-ai-native-specmap/src/mdspec.rs`: `bc770533d847a90507f38ee57f082bfc70423985903d73162852a88f26d5e8af`
- `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v1.1.0/crates/core-ai-native-specmap/src/xmlspec.rs`: `321c91890b0c2b591a4a28d4ae846bbe628d43eeeb6549c88597277bcb695442`
- `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v1.1.0/crates/core-ai-native-specmap/src/xmlspec/facts.rs`: `acfa79d4d87758024a4c0875ef5a1a7e57f020e4bd31863dee302b2ae6c2e828`
- `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v1.1.0/crates/core-ai-native-specmap/src/m_01_c_3_cases.rs`: `1d5eb087fdd8200ed90b86dd869a13d81d7822b78d2fd6e55f214979703e92ef`
- `vibevm/vibepacks/org.vibevm.ai-native/core-ai-native/v1.1.0/crates/core-ai-native-specmap/src/generated/specmap/mod.rs`: `d7ee80f8d4b48c4a4358e4f00ba847e83d1660c0c0bcf98fcbbcee6f7d1c4fce`

### Exact path correction and descriptor confirmation

Root clarified and authorized source_query/mod.rs and source_query/m_01_c_3_cases.rs, then descriptor_map.rs. The canonical workspace positive test truly compares equal IR identities for root/law/ITEM across held MD/XML snapshots with different raw SHA-256; the ordinary-data negative uses interpretation None and correctly refuses. It constructs RecordedOutput directly, so it does not exercise slot-mode-to-output-media dispatch.

Minimum incumbent fixes, with source lines:

- `membership.rs:282` currently copies `record.spec_format` into RecordedOutput.format; `:326-341` consumes that as actual document media. `selected_materialized.rs:55-57` repeats the same mistaken dispatch. `descriptor_map.rs:219-231` already distinguishes Mixed originals and selects the authoritative source descriptor; `:280` constructs the output row but currently retains only interpretation, not its media. Keep slot format as transformation policy, and derive/carry actual output media from the selected descriptor plus proven conversion/disposition. Mixed means preserve each declared source media, not unknown document format. Converted output uses its actual target; copied output keeps declared original media. This avoids a fresh suffix guess and also supports descriptor-classified Vibe documents with unconventional physical suffixes. A schema/field correction may belong at an imported type owner; root/executor must bind that exact owner rather than locally fake it.
- `membership.rs:327-329` expressly opens Vocabulary::Doc. `selected_materialized.rs:55` then calls the Spec-only engine reader through xmlspec::native_anchor_range. Minimum fix is to expose/use a native span result from the same vocabulary-aware SpecDoc frontend (or a vocabulary-aware equivalent that accepts exactly that frontend's document), so accepted VibeDoc plus a note does not fail on the second parse. Narrowing away VibeDoc would not fix the admitted existing contract. The fixture is title id=root followed by `<note kind="tip">Tip</note>` in selected XML Doc. For an anchored section the same note inside it exercises start/end.

Native range tests and current API duplicate checks cover duplicate anchors in supplied text; selected_document_output's second-match branch is visible, but the supplied selected trace fixture does not establish the ambiguous descriptor-output case. Add the bounded fixture that has two captured, declared active outputs canonicalizing to one doc_path and assert refusal before a locator is published. This is a proposed deciding check, not a test executed here.

- `crates/vibe-workspace/src/source_query/mod.rs`: `e4ddd0fe9248413b04c4813d4c8584acaa3fb24d4d33272b8dce5f83f696a1d8`
- `crates/vibe-workspace/src/source_query/m_01_c_3_cases.rs`: `66f0d06da91f4b263db372cd8edfd8f782715ee2c250069bba9997adfcc38a78`
- `crates/vibe-workspace/src/vibedeps/descriptor_map.rs`: `2ca032fccdb32200e7eb2fc8ae0abd9ddbba1967f3bdd940a03a418f3473fe71`

## Final corrected selected-locator review (2026-10-12)

**No concrete remaining defect found in the commissioned prepared selected-locator boundary.** This closes the two earlier concrete findings for the reviewed snapshot; it is review evidence for Root, not acceptance. No tests/builds/Git/source edits were performed by this reviewer.

The actual native helper is `native_anchor_range_with` in already-authorized `crates/vibe-specdoc/src/xml_in.rs`; the proposed xml_native_range.rs file does not exist.

- Mixed/media finding closed: RecordedOutput now separates slot_format from MaterializedVibeMedia. recorded_output selects source media through the retained manifest's source descriptor and interpretation; copied/Mixed output retains that media, while Converted output takes the target format. Both canonical parsing and native selection consume actual media, not whole-slot Mixed policy. The selected Mixed Markdown fixture exercises the real world/map path and resolves DOC.md line 3.
- VibeDoc vocabulary finding closed: native_xml_range_of_output chooses the same selected Spec/Doc vocabulary as canonical_node_of_output and calls the SpecDoc XML frontend's native helper. The helper first validates the entire document under that vocabulary, then attaches native event spans. The actual selected Doc fixture contains a note and resolves XML section lines 3–5, while explicitly asserting the former Spec-only engine range parser rejects that same valid Doc input.
- Multiline opening line fixed: event collection keeps separate begin positions for ranges and old after-event positions for diagnostics. Supplied before log shows actual `(5,7)` versus required `(3,7)` failure, after log shows the exact test passing. Text-event merging keeps the new positions vector aligned when merged entity/text runs do not add events.
- Schema separation remains correct: the selected entry reads canonical stem+anchor, rejects duplicate carried units, observes its output only via the prepared selected descriptor capture, and adds canonical_node/materialized_locator without rewriting contentHash/file/line. No legacy highest-slot discovery is called along this explicit entry path. Source-only committed map optional fields remain absent. The legacy foreign route now labels its file/line/hash as carried source/projection and explicitly sets selected_materialization=false.

Observed supplied logs: selected trace 4/4, workspace raw-versus-canonical cases 2/2, canonical/native SpecDoc cases 7/7; multiline repair exact 1/1 after the recorded 0/1 failure. These are executor logs read here, not independently executed checks. Original-source authentication/converter/overlay proof belongs to the separate C2/C3 proof review.

Exact limit accepted in this packet: C3 callable boundary is explain_selected_materialized(world,...), not legacy explain(root), and no full CLI/MCP integration or all-IR/native platform coverage is claimed. Schema generation/integration and acceptance remain Root-owned. No outstanding earlier integration warning is being promoted into a new blocker under this clarified boundary.

Reviewed files below all match the supplied terminal SOURCE-HASHES.sha256:
- `crates/vibe-trace/src/selected_materialized.rs`: `454ada5e79107f92eaf2c4812d299724be5d03faabd8f0875c058bbab335446c`; candidate manifest match: True.
- `crates/vibe-trace/src/selected_materialized/m_01_c_3_cases.rs`: `9deabf1c4d71235a5ecc00538c7eb39edf8517ad2bb20b4ec1901aada5fd02b4`; candidate manifest match: True.
- `crates/vibe-trace/src/foreign.rs`: `d073b8908c8dfad41a319b71ad39e858727fe240efcb29b86aeca187d5c91832`; candidate manifest match: True.
- `crates/vibe-workspace/src/source_query/capture/continuity/membership.rs`: `ed94fb543fdf1fd092ad25c40a6ebff74356f7b36083e006101765db4439f9b9`; candidate manifest match: True.
- `crates/vibe-workspace/src/source_query/m_01_c_3_cases.rs`: `16cc4cfb80457f794293d897cc2203d4f867c46a57449c4c16ff37e9e0e81ce1`; candidate manifest match: True.
- `crates/vibe-specdoc/src/canonical_node.rs`: `d79055f3aebfa4701235dd2af42c22437ad9e14756d04e9d541222f7abe25a20`; candidate manifest match: True.
- `crates/vibe-specdoc/src/xml_in.rs`: `36758a2a5ab31d16f44f4067f36cfc7da2d2a8cb05306e8fa939a218dc867a07`; candidate manifest match: True.
