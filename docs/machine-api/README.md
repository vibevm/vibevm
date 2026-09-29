# Machine operation architecture

The [authority map](authority-map.md) assigns retained model families and planned
operations to their domain owners, projections, extraction gaps and proposed
conformance obligations. The governing source is
[PROP-080](../../vibevm/vibespecs/common/PROP-080-machine-operations-and-observability.xml).
The retained map also names current bounded source and model owners. The shared
machine service currently admits one pure migration-planner route; the other
inventoried operations remain explicit extraction gaps.

The map binds the retained [Coverage inventory](../../formats/capability-coverage/coverage-e1.json).
Its source identities describe the selected current source observation and 35
model families, including explicit unbound planned families. The selected
capture is finite and does not authenticate every current source byte or an
effect grant. The map keeps authoritative records, derived views and CLI-local
logic separate, and assigns a proposed, unrun conformance obligation to every
bound model or planned-family row.

Check its finite references with the implemented developer tool:

```sh
cargo run --quiet --locked -p xtask --bin authority-map-check -- \
  --inventory formats/capability-coverage/coverage-e1.json \
  --map docs/machine-api/authority-map.md
```

The tool reads Coverage through its generated reader and pure validator, and the
map through the canonical Markdown reader. It compares exact model/family IDs,
copied inventory owners and source-ID sets, then checks conformance references,
duplicates, orphan rows and the `PROPOSED UNRUN` status. It prints both input
digests. Exit 0 establishes these joins; exit 1 reports a read, parse or join
failure. Argument errors follow clap's normal exit behavior.

The 35-model authority/input/projection matrix and the meaning of each proposed domain
owner, epoch boundary, observation axis and conformance obligation require
independent semantic review. The checker does not perform domain operations,
live source remeasurement or conformance execution. A successful check grants
neither acceptance nor authorization.

The bound helper cases can be run independently:

```sh
cargo test --locked -p xtask --bin authority-map-check
```

The earlier root xtask command census excludes standalone developer binaries.
Their registration and external-access closure remains explicit in the map's
coverage obligation. Registration metadata, library construction and actual
effects remain distinct.
