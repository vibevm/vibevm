# F17 — Normal closure once-coverage

Implementation commit: `2a9da5c8b2e9ba6b73d2192df1f6b6a541a7f373`; all210integration blobs verified.

Accepted at the documented implementation boundary: runtime, scoped static, deterministic code generation and traceability gates passed. The intended ordinary install behavior is now demonstrated, including rebuild of an older unstamped unit cache. This feature co-lands with F19 because source suppression must preserve typed embed meaning.

During default normal linking, a specification reached through an umbrella and directly contributes each identical source region once. Uncovered preamble, siblings, unanchored regions and legitimate contract/source :add continuations survive. Distinct documents with equal prose, different captured text/reference meaning and separate artifact compilations remain distinct.

The existing link pass resolves every full occurrence before coverage, so a repeated string cannot hide another link binding or an ambiguity. One private coverage owner compares parsed source identity, resolved text and meaningful generated provenance. Output positions and fresh occurrence IDs do not define source identity. The full input digest and the same validation replay remain authoritative. Open fence boundaries conservatively remain verbatim. Authorized later lane/opaque-byte transforms retain their distinct working-surface contracts.

## Semantic review and evidence

Initial XML probes reproduced duplicate umbrella/direct sections and partial-to-whole preamble/sibling duplication. The first implementation wrongly deduplicated before link resolution; a real discriminator lost the second A/B short-link binding. Moving coverage after resolution fixed it. Re-parsing then lost terminal-newline form; the incumbent test caught that and raw retained line endings were restored. No old oracle was weakened.

Actual CLI tests use a local registry and normal umbrella/alpha/beta with real dependency edges and direct static roots. They check XML bodies once, nested source transit, unit compiled→skipped trace, unchanged bytes/mtime, an unstamped-cache rebuild followed by reuse, tree v1, deleted-lane reinstall, authored INDEX literal behavior and unchanged registry source bytes. The empty repeated source contribution retains truthful source ownership. Separate boot tests preserve uncovered/continued regions, independent equal prose, source placement and typed unresolved #use refusal with old artifact bytes unchanged. Existing simple-normal and embed controls remain.

Final corrected compiler family1012 passed at its checkpoint, with subsequent focused locator/merge/current-wire gates. Broad boot41, public tree15, actual install/upgrade/reinstall2 and rollback8 pass; counts overlap, not additive completion totals. The runtime/scoped static evidence and source hashes are archived in the current local context under evidence/2026-11-06/xml-ir2-acceptance.

## Original task boundary

M-06-G.1's exact contract was re-read at pin7fe6376c. Its normal/simple distinction, cross-package root/solution and source traversal, source placement, authored INDEX reading, typed no-publication refusal and disposable evidence are covered. A node-owned INDEX entry kind=static means unconditional read timing, not normal package compilation. #source is recognized after optional leading whitespace on an unmasked line; no last-line rule is invented. Canonical docs are PROP009#NORMAL-CLOSURE-ONCE/#AUTHORED-INDEX-BOUNDARY and PROP035#SOURCE-DIRECTIVE-PLACEMENT.

Native Windows/macOS execution is not established; the historical native-Windows qualification remains explicitly open as the original task permits. G3 payload filtering/source-member publication and full debugger work remain separate. No authored vibe.toml/install-g syntax change belongs to once-coverage itself; F19 records the necessary compiler wire/generated-framing evolution.
