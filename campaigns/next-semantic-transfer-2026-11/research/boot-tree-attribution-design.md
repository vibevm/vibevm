# Truthful nested attribution in the existing committed-artifact tree

Root decision,2026-11-04, following actual CLI reproduction at ee3031c51. The existing PROP-036 EMBED-SPANS contract already requires nested spans. Real FsSectionSource→expand_embeds payload A[2..12]/B[6..10] loses A; an authored three-level tape loses both ancestors. A generated fenced documentation example survives expansion unchanged, so importing the prototype's raw stack would still misinterpret that sample. Source evidence and exact runtime limits are in boot-tree-reproduction.md.

## User behavior and existing owner

Extend the current `vibe tree` committed-artifact decompiler; keep the same plain/TUI/JSON owner and v1 JSON shape. Return every well-formed occurrence, including repeated addresses, with its own one-based file-relative inclusive marker boundaries. Emit spans in opening/source order. Nesting is a stack of occurrences, never a map keyed only by address; decoder attribution does not resolve addresses or reject semantic cycles.

Opening and closing addresses must match exactly and be nonempty. A closing address must never rename an opening. Matching cannot cross static-contribution boundaries. Unexpected, mismatched/crossed, empty or unclosed reserved embed markers are explicit parse errors with line context through the existing StaticDecompileError→CLI error route. Do not silently emit a partly invented clean tree, skip unmatched frames, or repair addresses. This intentionally tightens formerly successful malformed artifacts, including the old empty-close shorthand; actual compiler output writes both addresses. Regenerating/fixing the artifact is the corrective action.

Recognize reserved embed controls as complete standalone comments after permitted surrounding whitespace. Normal other comment genres remain ordinary text. Retain the existing c1 provenance codec, multiline provenance and existing malformed-c1/unterminated-comment diagnostics. No independent URI parser, percent decoding or target lookup is needed.

## Literal examples and one fence reader

Boot attribution must not turn fenced examples into provenance. Use the existing vibe-spec FenceTracker grammar, shared with Directives::parse; do not create another backtick/tilde lexer or use parsing success of a Vibe document as permission to inspect comments. A small additive `fenced_code_lines(&str) -> Vec<bool>` query may expose the current canonical fence result without exposing mutable parser state or copying all lines. It includes delimiter lines and uses the same logical-line convention as the decompiler.

Apply that mask consistently to static-provenance discovery and embed controls, so a literal `vibe:static` sample does not split a real contribution. Ignore marker-like comments inside a fenced block, including unpaired embed examples. Preserve the real generated outer/inner spans around the sample. Non-fenced reserved syntax remains subject to the explicit errors above. This decoder follows the generated boot-marker contract and canonical Vibe fence rules; it is not a general XML/CommonMark validator or proof of source authenticity. Compiler generation/format bytes are unchanged.

## Integration and compatibility

Reuse current tree models, errors and read paths. A private attribution submodule is appropriate if artifacts.rs would exceed its existing line budget; separate a real responsibility rather than deleting explanation or weakening checks. Keep public ordinary document inspection, descriptor migration, occurrence-addressable explain APIs, source-query machinery and exact contribution-byte measurement as separate required campaign work. In particular this change does not relabel the existing CRLF-sensitive body byte summary as an exact measure.

No vibe.toml, install -g, lock/index or tree JSON schema change. Existing flat generated artifacts retain their spans/order; nested arrays gain the previously lost occurrences. Malformed reserved-marker artifacts now fail explicitly. Literal fenced marker examples are ignored rather than attributed. The additive fence query is the only intended cross-crate API addition; no dependency or serialized version change is needed. FMT-012 records these semantics before implementation.

## Required proof

Turn both valid nested RED cases green with exact opening-order assertions. Promote the real generated fenced-example discriminator to exact outer/inner assertions and add a fenced static-marker example. Keep same-address sibling and nested-repeat occurrence controls, LF/CRLF, independent contribution boundaries, XML c1 origin/path and v1 schema/read-only checks. Convert malformed characterization cases into the chosen line-diagnostic/error expectations; add empty opener and malformed standalone control cases. Preserve positive ordinary comments and the existing c1 suite. Reuse the actual expansion owner, not a duplicate parser oracle.

Verify the public CLI target and affected tree/compiler fence tests, scoped clippy/fmt/conform and root's final map gate. Record compiled examples/traceability for any new public API from the outset. Candidate hashes, raw RED and final results remain separate; root accepts and removes the temporary worktree after integration. This design does not claim a full boot debugger or whole campaign completion.
