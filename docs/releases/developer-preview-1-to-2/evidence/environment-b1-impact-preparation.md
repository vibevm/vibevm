# M-14-B.1 release-impact preparation — candidate only

This review worksheet names the ten B.1 product subjects from the accepted
[B.1 contract review](environment-b1/central-review.md) and the exact
[B/C Git inventory](environment-bc-product-subjects.json). It is neither a
`release-impact.json` record nor an accepted user/documentation disposition.
The candidate B before/after captures remain in temporary storage pending
B-226 (large-blob import or equivalent evidence-contract decision).

| Subject | Observed contribution | Candidate documentation topic / final-review question |
|---|---|---|
| `6513cfff9` | PROP-076 exact lock, payload and realized-generation identity laws | Explain source-supplied identity versus physical/authenticated evidence; decide grouping with later code records. |
| `c7517ffd0` | PROP-076 macOS lexical path policy | Document POSIX-byte lexical checks without claiming APFS/HFS alias or native execution proof. |
| `92d30ff72` | PROP-076 selected target role/platform in exact lock identity | Explain why target selection changes identity and unsupported target facts refuse. |
| `aa5fc8d7e` | Pure normalized, domain-separated source-supplied digest producers | Name the library API and its missing physical capture owner; no existing lock is silently upgraded. |
| `85e50713a` | Corrected computed digest agreement with wire value | Compare before/after public value behavior and decide whether it joins the producer record or needs a separate correction record. |
| `85358c52b` | Selected target role/platform participate in hashing | Pair positive target-change and negative unknown-target cases with the corresponding source law. |
| `a93bd20b2` | Captured configuration bytes participate in hashing | Explain exact byte identity and absent-capture refusal; do not infer a config filesystem read from the pure producer. |
| `87023f56a` | Shared shippable-tree recipe in the exact producer | Review the diff for behavior equivalence before proposing a no-user/no-documentation-impact disposition; a `refactor` subject alone is not proof. |
| `0de05ea0d` | Six epoch-one environment schema/corpus/generated reader surfaces | Identify the public wire API and compatibility/new-field guidance; generated paths are one coherent format contribution. |
| `e1e1473d2` | ExactLock wire validation against independent typed selection/capture | Explain what validation checks and what source authenticity it does not check. |

The accepted B.1 review retains current-target positive and negative cases:
core exact identity 10, macOS lexical 1, wire semantic 2 and wire contract 3.
The later selected Linux gate rerun confirmed the exact core family 10/10 on
tree `6de6bfdbfc`; it did not rerun the original macOS lexical or wire semantic
cases. The global `wire-diff` limitation on unchanged index `repomd.json`
remains red in the original review and is not turned into a pass here.

No user migration is implied by a pure value producer or schema registration;
whether each correction changes an external consumer's observed value still
needs exact before/after review. Documentation is positive for the new
identity and wire contracts, but canonical page/API/example/build bindings
belong to the final documentation gate. All ten subjects still lack permanent
impact records and accepted capture references.
