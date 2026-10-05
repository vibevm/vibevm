# Rust and Lean selected-subject fixture

The selected address is
`spec://org.example/same-subject/Protocol#identity`.
Rust implements identity over `u64`; its executable tests cover zero and
`u64::MAX`. The Lean theorem states reflexive equality over `Nat` and is
enrolled for `g1_model` checking. Enrollment alone is not a checked proof;
the legacy-writer receipt does not run Lean. A successful model check plus
the Rust tests still makes no `Nat`/`u64` refinement claim.

`specmap.toml` explicitly opts into `rust_lean` composition. Legacy writers
must refuse that field rather than replace a combined map with partial
language coverage. Exercise the actual separately built writers with:

```sh
python3 check-legacy-writers.py \
  --rust-binary /absolute/path/to/rust-ai-native-specmap \
  --typescript-binary /absolute/path/to/typescript-ai-native-specmap \
  --receipt /absolute/path/to/refusal-receipt.json
```

Each writer receives its own temporary fixture and sentinel map. Success
requires an unknown-`composition` refusal and byte-identical retained map.
The child PATH is empty, preventing an accidental Node/extractor launch;
a missing-tool error does not satisfy the assertion.

The Rust fixture uses nightly inert registered-tool attributes so it can
both compile without a proc-macro dependency and carry the real spec tag:

```sh
CARGO_TARGET_DIR="$PWD/.targets/implementation" \
  cargo +nightly test --manifest-path rust/Cargo.toml --offline
```

The Rust symbol is `rust::identity`, matching both the Cargo package name
and the scanner's directory-based identity. No symbol remapping is needed.
No successful lifecycle adapter is claimed by this fixture until the shared
joining path is implemented and checked.
