# A4 installed package requirements query — accepted slice

Accepted in `12ee51b0ebcb185ede2ac94f08ded0de7694caa4`; root verified all26committed blobs. **Original M06A4 remains partial; old113counts stay12/31/31/39.** Governing contract: PROP-072 SOURCE-QUERY-READ reqr1. The sibling JSON records scope, exact gates and compatibility.

Actual Mixed/Markdown/Xml materialization now feeds requirements queries through A3's selected installed binding. Independent lock identity/hash and trusted configuration-selected input format are checked; original logical fact addresses survive conversion. Raw resources cannot mint native facts. Original and installed Spec/Doc facts agree on marked/unmarked metadata, status, void and requires; Doc note/figure facts are included.

The existing native AST parser is reused directly. Syntax failures retain Invalid with complete captured materialized witnesses; untrustworthy binding/read yields Unavailable without a fabricated digest. Interpretation rows are borrowed from the same resolver that reads bytes, eliminating accidental independent metadata pairing. Actual CLI and MCP tool routes preserve strict grammar-before-I/O, read-only/no-lease behavior, provider selection and report construction.

## Evidence

Requirements53, facts7 plus missing-binding1, preparation11, MCP11, actual CLI2 and docs10+6 pass (counts overlap). Strict all-target Clippy and Conform pass across seven affected crates/scopes; formatting/diff pass. Final library/bound-facts tests and all-target compilation cover the last API tightening; surface runtime runs preceded only removal of its independent binding parameter. Independent review plus root follow-up inspected that tightening.

Specmap/check:9148units/5514items/4663edges, zero gated orphans or unresolved host edges,44unchanged warning meanings; all9146prior occurrences retained. Wire-diff passes with unchanged watched schemas/formats. No codec generation was necessary. Cargo.lock adds only the existing requirements-to-spec dependency edge.

The initial actual-install regression failed at Markdown because query sought original-path input instead of recorded output. Later compile errors were PackageName/String seam corrections. Root corrected its own initial recommendation to use compiler projection: that introduced an unnecessary XML→Markdown→AST roundtrip, replaced with the existing direct facts parser and tested parity. No parser rewrite or parallel slot-record validator was introduced.

## Compatibility and remaining work

FMT-030 records the new trusted Rust QueryContext field and diagnostic reason value; no wire query field, report schema or manifest grammar is added. Recorded legacy metadata is now checked for manifest/selection coherence before compatibility fallback; clean recorded and unrecorded legacy remain supported, corrupt recorded metadata yields Unavailable. Witness hashes describe observed materialized bytes under original logical labels, not unobserved original-file content.

Host project.sources/cited sync is the next sequential slice. Remaining facts adoption and check/docs/skills/host consumers, whole mixed-source matrix and tutorials remain A4 obligations. Declared in-place stays an explicit unsupported boundary. No public push, release, temporary worktree, global snapshot guarantee or platform certification is claimed.
