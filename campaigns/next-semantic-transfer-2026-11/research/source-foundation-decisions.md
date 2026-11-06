# Source interpretation — target decisions before implementation

Design decisions, not runtime acceptance. Prototype pin7fe6376ccef7f321302a8f18610fcb0c9cc2686f; target after accepted M13C1/M02B1/B2. Existing evidence: history-source-descriptors-review.md S1/S2 and the current bounded A1/C2/D1 source audit. No source declaration is activated by this document.

## Presence is meaningful for packages

For a package, presence of `sources` opts into explicit classification, including `sources = []`. An explicit empty list means no declared semantic sources; unmatched files remain raw/unclassified. It must not reactivate legacy suffix-based scanning in facts, citation or other readers. Absence preserves the selected legacy compatibility route only when neither source metadata key is present. The typed manifest and serializer must preserve this difference; a default empty Vec with skip-empty serialization is insufficient.

This deliberately preserves the prototype materializer/map's existing explicit-empty meaning and fixes disagreement with its facts/citation consumers. The original package rule was unresolved, so this is an explicit target decision rather than an assertion that every prototype caller already agreed.

Project behavior remains a separate compatibility contract: absent and empty project.sources retain the documented legacy behavior; nonempty declarations select explicit classification. Do not infer package behavior from the project rule or make readers invent their own opt-in predicates.

Future source_exports-only presence must not enable implicit legacy semantic scanning. An export needs a real CommonMark descriptor/selection and heading resolution before use; its declaration does not itself assign interpretation, media or Vibe fact status. Exports are a later A2 binding/consumer responsibility; A1 does not gain a placeholder export API merely to reserve the name. The eventual typed presence owner must account for export-only input consistently when that capability lands.

## One interpretation decision, actual consumers

Exact-file versus directory precedence, conflicting equal-specificity refusal and declared media/role/loading come from the shared source selector. Physical filename suffix is not permission to reinterpret explicitly raw content. Preserve distinct raw same-stem files and keep source path, output path, semantic address, original package identity and observed output identity separate.

Do not expose a newly accepted descriptor declaration to a still-legacy suffix converter silently. Before activation, each affected supported route must consume the same classification/selected bytes or refuse the unsupported operation precisely before semantic mutation. Such refusal is an honest intermediate boundary, not completion of the full consumer obligations. Preserve existing unopted-in behavior and main's stronger held/no-follow source reading; no reopen-after-validation shortcut.

## A1 is a joined migration and adoption task

The exact original A1 notes and tests require source-plan → actual application, repeated application → NoOp, stale-source refusal without journal creation, and the first facts-adopt consumer over Mixed exact-file/pinned-original slots. A descriptor parser plus pure proposal does not close A1.

Accepted M13C1 supplies neutral exact-token planning, not saved apply/recovery. The dependency route is strict neutral C2 request/report transport and thin CLI/MCP planning first, then the actual one-file apply/replay/journal/recovery capability required by A1, then the descriptor reader/source recipe and joined adoption/materialization path. The source audit must name exact interfaces; importing all of M16 is not justified by a final prototype module name. Conversely, in-memory FileGuard rollback cannot be called process-exit recovery.

M13D1's full interruption/platform/surface obligations remain explicit even if A1 exercises only two focused apply cases. Completing that focused A1 journey cannot silently certify all D1 recovery or the whole source interpretation family. Source A2/A3/A4 downstream obligations and prototype gaps remain on the campaign map.

## Implementation-scope clarification

Main's project section currently rejects a sources key, including an empty one. The project absent/empty compatibility rule above is the required policy when that later field lands, not a claim that A1 already accepts project.sources. A1 introduces package declarations only; a valid project-only manifest without that field remains legacy. Unknown project fields refuse rather than silently selecting a policy.

Portable reserved-source admission and payload ownership are separate laws. Source declarations must refuse proven ASCII case aliases of reserved generated boot paths, derived from the canonical path/prefix owner. Exact-case G3 payload filtering remains unchanged: rejecting a declaration alias does not claim that every such physical spelling is generated on Linux.
