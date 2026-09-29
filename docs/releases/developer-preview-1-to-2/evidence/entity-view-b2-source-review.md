# M-16-B.2 finite affected-source review

Immutable source endpoints: `75d6c6416b41a53dfd3a2ab1345da71adb9aa311` → `e73b73881dfe5fc028054d8c622a828a53d403a4`. The selected product files are 3948 → 4015: 67 newly selected, 12 changed, no absent selected path. All 48 canonical package objects are equal. The source-selected set is finite; it is not a native runtime or whole-product semantic census.

The first-parent interval has 20 commits and 124 nonexcluded product path touches over exactly 79 unique endpoint-changed paths. Every touched product path appears in the before/after selected-file union; no transient touched path was omitted. 13 task-packet/evidence touches fall under inherited exclusions. The catalogue hash for excluded `campaigns/next/manifest.json` is separately joined to its immutable Git subject, not falsely attributed to this product capture.

| Candidate change record | Product paths | Implementation commits |
| --- | ---: | ---: |
| `machine-entity-view-source-contract` | 1 | 3 |
| `machine-entity-location-core-model` | 5 | 3 |
| `machine-entity-view-wire-and-reader` | 45 | 4 |
| `machine-location-uri-wire-metadata` | 22 | 1 |
| `machine-composite-page-projection` | 10 | 8 |
| `machine-resource-uri-adapter` | 4 | 2 |
| `machine-entity-view-capability-catalogue` | 3 | 1 |
| `machine-entity-view-traceability` | 1 | 1 |

The path memberships overlap in 12 exact wire files: initial four-root registration and the later source-location URI metadata amendment each changed those bytes in distinct commits. This is shared source history, not two independent domain engines. The source contract revision for entity provenance was bumped to r2 after the explicit content→capture and row→occurrence joins; product package versions remain mutable 1.1.0.

The reviewed implementation is pure supplied-capture model work. Entity and location roots retain owner-provided identity, non-UTF8 octets and raw/projected basis. The composite view rejects missing/stale/conflicting declared witnesses and never claims a global atomic read. A fixed in-memory page session issues one-use tokens and reconstructs its retained authorized row structure; unknown or reused tokens, cross-version entities, wrong parent/occurrence links and false complete children refuse. The client URI is generated only by a scope-bound owner-declared map and explicitly remains unverified against physical source bytes.

This finite review does not authenticate captured bytes, resolve a live URI, reverse-map Markdown positions to XML bytes, persist cursors across requests, prove all-platform native execution, satisfy final documentation or publish Developer Preview 2. Those are later campaign obligations.
