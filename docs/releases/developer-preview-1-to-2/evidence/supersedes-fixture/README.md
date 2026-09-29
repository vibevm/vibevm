# Selected Supersedes relation fixture

Two ordinary scratch-Git cases exercised the existing checker binary with
generated request JSON on stdin. The fixture records and passing evidence marker
are mock input; they assert no real contribution acceptance or product outcome.

Selecting both original and replacement records with an explicit `supersedes`
relation returned exit 0 and 25 byte/delta claims. Source-review, interface,
native and semantic findings remained pending. Omitting the related record from
the selected IDs/digests, while leaving its file present, returned exit 1 with
`record-relation`. Both expected outcomes passed the fixture assertions.

The retained driver creates a fresh toy repository at its explicitly bound
scratch path and refuses an existing repository occupant. Its original scratch
repository, requests and captures remain at
`/tmp/vibevm-next-supersedes-proof/`; the shared active transition now holds the
negative selection. Saved per-case transitions retain the actual earlier inputs.
For independent reproduction, bind a fresh scratch directory and the actual
checker binary in the driver before running it. Do not rerun against an occupied
fixture or accepted product records.

The summary retains exact commit, input, output and binary identities. This is
selected relation-membership evidence, not net supersession semantics, complete
native linkage, documentation or release proof. No production source mutation,
Cargo test suite or broad gate was run.
