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
