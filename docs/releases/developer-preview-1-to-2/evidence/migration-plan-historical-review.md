# Historical materialization input provenance

Candidate evidence only; no runtime execution, migration acceptance or repository write.

Rename commit: `d4d6c4754e5a07f60419e35f8d19d6adbd777352`; exact parent: `6f4b575082ceae133c3564e9920d5f6a0ea2328f`. The commit is titled `feat(core): the default materialisation mode says copy` and explicitly states no manifest declared materialization at rest. An independent exact-path grep of all 172 tracked `vibe.toml` files at the parent found zero declarations (exit1 = no match). An earlier wildcard-shaped query under literal-pathspec mode selected no actual paths; it was not evidence and was replaced by that exact path list.

The retained rename test `materialization_refuses_the_legacy_snapshot_name_with_the_recipe` has exact literal input `v = "snapshot"`. Both decoded input bytes and original Rust string lexeme are retained, with enclosing immutable test blob, full commit/path/OID/SHA256 provenance in `provenance.json`. The parent's test source is also retained: its kebab-case roundtrip table recognizes Snapshot and builds the same scalar shape through its existing formatter.

This is a synthetic historical **scalar fixture**, not a real authored package manifest or full `[package]` input. Its only root key is `v:string`; no epoch/package/requirement/identity field exists. Do not enlarge its proof into complete pre-epoch grammar support.

The separate retained `package_meta_parses_materialization_and_bridge` raw fixture preserves all six fields exactly: name/group/kind/version/materialization/bridge. It uses in-place and bridge=true and has no `[package]` header. It demonstrates that this historical PackageMeta shape exceeds the candidate's optional fields (bridge), but does not supply snapshot package migration proof. Root must either bind this shape explicitly with honest scope or reject it; dropping bridge or inserting a header/snapshot value would fabricate a corpus member.

A retained legacy fixture exists; an explicit snapshot `vibe.toml` does not exist in the selected pre-rename manifest tree. No wider history search, source simplification, invented manifest, test execution or grammar implementation was performed. The current source rename recipe and task binding remain separate inputs; product1.0→1.1 and snapshot-as-unfrozen-version semantics do not select this grammar.
