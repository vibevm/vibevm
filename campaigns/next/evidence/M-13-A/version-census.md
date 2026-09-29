# Active source version and authority census

The bounded census covers 190 exact files, including 184 manifests. The retained
JSON is source evidence; its candidate acceptance flag does not decide central
acceptance. Independent reconciliation matched all 190 source hashes, all 632
literal field/value rows, eight canonical workspace membership lists and 28
derived-to-original manifest byte identities. The reconciliation returned exit 0
with no faults. This establishes manifest identities and inheritance, without
proving runtime behavior or a complete installed package tree.

| Cohort | Count | Observed version or carrier | Disposition |
| --- | ---: | --- | --- |
| Host workspace declaration | 1 | `1.0.0` | Keep |
| Host project declaration | 1 | `1.0.0` | Keep |
| Host Cargo members | 35 | `package.version.workspace = true` → `1.0.0` | Keep inheritance |
| Canonical Cargo workspaces | 8 | `1.0.0` | Keep |
| Authored canonical member crates | 60 | Owning workspace → `1.0.0` | Keep inheritance |
| Canonical package manifests | 48 | `1.0.0` | Keep |
| Root own local dependency constraints | 33 | `1.0.0` | Keep distinct constraint identity |
| Additional Cargo dependency constraints | 128 | Various library/upstream versions | Preserve independent pins |
| Research development consumers | 2 | `0.1.0` | Preserve research identity |
| Derived vendor member manifests | 27 | Original bytes; workspace → `1.0.0` | Exclude from edits |
| Selected installed specmark manifest and workspace | 2 | Original bytes; workspace → `1.0.0` | Exclude from edits |

Every canonical package epoch is 1; each frozen field is absent. These are
separate format/freeze axes, not product versions. Consumer minimum versions,
exact edition pins, licenses, external application coordinates, retrieval refs,
release tags and historical prose also retain their own meaning. The JSON gives
their exact file/field/value dispositions and source hashes.

D-019 keeps current active owned products/packages at `1.0.0` and permits the
current Redbook roster change at that version. Root/Redbook immutable prose is
source-policy debt for M-13-B.1; it authorizes no inferred freeze or version edit.
The two research projects are owned development consumers outside the delivered
host/canonical package cohort, rather than foreign packages. The retired launcher
source remains historical campaign evidence. No launcher execution, installation,
publication or prototype graduation follows from this census.

The exact numeric edit perimeter for M-13-D.2 is empty. All 632 recorded fields
have a specific no-op disposition; derived copies, fixture/example manifests,
installed projections and foreign/history identities remain outside numeric
editing. A later source change requires affected reconciliation before that
future task uses this perimeter. This census does not close migration runtime or
source-policy work.

Two representation defects were corrected before acceptance: 95 inheritance
rows had an object where their literal boolean field was named; 88 dependency
paths confused scalar shorthand with a table's `.version` subfield. Actual
manifest bytes and resolved versions did not change. The final independent field
reconciliation checked the corrected inputs. No Cargo suite, production mutation
or workspace format gate was needed for this campaign-evidence-only change.
