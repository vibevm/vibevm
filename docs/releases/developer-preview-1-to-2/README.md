# Developer Preview 1 → Developer Preview 2

The [source capture](evidence/dp1-source-capture.json) records the exact
before-state for this implementation campaign. It identifies commit
`30d217bfce47bcaccadabd9468f00e315dbcfead` and its Git tree, the root
manifests and locks, 48 owned package slots, and the CLI and MCP registrations
declared by source. Package subtree IDs include every tracked file under each
slot. The capture is reproducible from that commit even though build outputs
were removed by the requested `cargo clean`.

The [transition index](transition.json) is provisional. Its DP2,
documentation, migration, and verification fields remain pending. The
registered format and validator are established in the next accounting step;
this capture does not claim they already exist or that source declarations
prove runtime behavior. `vibe.lock` is retained byte-for-byte by its file
hash, while its local source URLs and content hashes remain historical lock
claims until checked against the selected source.

Product deltas exclude the temporary `campaigns/next/` and
`vibevm/vibespecs/terraforms/next/` scaffolding, but the raw Git tree still
captures their exact bytes. Derived `vibevm/vibedeps/` slots and Cargo build
outputs are outside authored product source. Independent documentation has
not yet been bound to an exact source revision.
