# Developer Preview 1 → Developer Preview 2

The [raw source capture](evidence/dp1-source-capture.json) and its
[registered epoch-1 projection](evidence/dp1-capture-e1.json) record the exact
before-state for this implementation campaign. They identify commit
`30d217bfce47bcaccadabd9468f00e315dbcfead` and its Git tree, the root
manifests and locks, 48 owned package slots, and the CLI and MCP registrations
declared by source. Package subtree IDs include every tracked file under each
slot. The capture is reproducible from that commit even though build outputs
were removed by the requested `cargo clean`.

The [transition index](transition.json) now has a registered shape. Its DP2,
documentation, migration, and verification fields remain pending; shape
validation does not establish that source declarations prove runtime behavior.
`vibe.lock` is retained byte-for-byte by its file
hash, while its local source URLs and content hashes remain historical lock
claims until checked against the selected source.

Product deltas exclude the temporary `campaigns/next/` and
`vibevm/vibespecs/terraforms/next/` scaffolding, but the raw Git tree still
captures their exact bytes. Derived `vibevm/vibedeps/` slots and Cargo build
outputs are outside authored product source. Independent documentation has
not yet been bound to an exact source revision.

## Intermediate bootstrap subject

The catalogue also retains `bootstrap-6c2a59ef9`, the tracked product subject at
commit `6c2a59ef9820b12d985d39527a0c86ea817379b4`. Its
[raw observation](evidence/bootstrap-6c2a59ef9-source-capture.json),
[epoch-1 capture](evidence/bootstrap-6c2a59ef9-capture-e1.json) and
[actual capture-check report](evidence/bootstrap-6c2a59ef9-capture-check.json)
bind 48 package slots and 129 selected current files. The selection retains the
ten baseline files plus present changed tracked paths, using the unchanged DP1
scope exclusions. The CLI/MCP subsets are retained only after their source bytes
and package declaration manifests match the reviewed baseline.

The exclusion rationale is inherited from DP1. Its phrase about release evidence
being absent describes that earlier boundary, not this intermediate commit;
the full captured Git tree includes the retained release evidence. The checker
does not establish a current absence claim from this historical text.

This is an intermediate source subject. DP2 remains pending. The report proves
selected bytes, Git identities and declaration linkage; contribution accounting,
complete capability/topic membership, documentation, runtime and final product
proof remain separate obligations. Recorded toolchains are Linux-session
observations; historical Windows claims were not copied into this capture.
