# Selected-subject identity fixture {#root}

@fact:identity The identity operation returns its argument unchanged. @status:spec/done

The Rust realization accepts a `u64` and is checked at zero and `u64::MAX`.
The independent Lean theorem states reflexive equality for `Nat`.
Both refer to this selected subject. A successful Lean check has grade
`g1_model`; passing the Rust tests does not establish a `Nat`/`u64`
correspondence theorem, Rust refinement, or grade G3.
