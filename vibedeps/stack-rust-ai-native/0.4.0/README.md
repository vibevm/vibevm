# AI-Native Rust (stack:org.vibevm/rust-ai-native)

The Rust projection of the AI-Native Code Discipline — and the **runnable
toolchain** that enforces it (PROP-024 code-bearing packages): installing
this stack yields working checkers and procedures, not descriptions of
them. The language-neutral method (manifesto, playbooks, mechanism specs)
comes from its dependency `flow:org.vibevm/discipline-core`.

## What ships

- **Three binaries** (this package's own Cargo workspace, `crates/`):
  - `discipline-rust` — the umbrella tool: `init` (bootstrap policies +
    registries), `floor` (the portable verification floor), `conform`,
    `specmap`, `trace`, `test-gate`, `tripwire`, `health`, `fast-loop`,
    `codemod`.
  - `conform-rust` — the conformance gate alone (ENGINE-CONFORM).
  - `specmap-rust` — the traceability engine alone (PROP-014).
- **The Rust guide and cards** (`spec/rust/GUIDE-AI-NATIVE-RUST.md`,
  `spec/cards/` — the nine scaffolds in their Rust shape, Band-3 ops
  blocks for weak readers).
- **Two agent skills** (`vibe skill install` projects them):
  `/terraform-rust` (brownfield adoption per BROWNFIELD-PROTOCOL) and
  `/discipline-sweep` (the recurring sweep per the Sweep Playbook).
- **The specmark proc-macro** (`crates/specmark`) — the inert
  `#[spec]`/`scope!` tags your code carries.
- `schemas/specmap.jtd.json` — the wire schema of `specmap.json` (the
  generated types in `specmap-core/src/generated/` derive from it;
  regeneration is a maintainer dev-op in the package's dev repo).

## Running the tools

Three supported forms, from your project root (where `vibedeps/` is):

```sh
# (a) vibe-native (PROP-025) — build once in the slot, dispatch through
#     the project's lockfile (two projects on different versions get
#     different binaries):
vibe bin build            # or: vibe bin exec discipline-rust -- floor
vibe bin exec discipline-rust -- floor

# (b) install once onto PATH — then just `discipline-rust …`
cargo install --path vibedeps/<stack-slot>/crates/discipline-cli

# (c) zero-install, run in place
cargo run --manifest-path vibedeps/<stack-slot>/Cargo.toml \
    -p discipline-cli --bin discipline-rust -- floor
```

`<stack-slot>` is this package's materialised directory (e.g.
`stack-rust-ai-native/0.4.0` — check your `vibe.lock`). Building in the
slot drops a `target/` there; add `vibedeps/**/target/` to your
`.gitignore` (build output is already excluded from the package's content
hash, PROP-024 §2.2).

## The lifecycle

```sh
vibe install                       # materialise this stack into vibedeps/
discipline-rust init               # policies + registries + external spec resolution
# … write spec units, tag code (GUIDE §13), adopt crate by crate …
discipline-rust floor              # the gate panel, one exit code
/discipline-sweep                  # the recurring sweep (agent skill)
/terraform-rust                    # brownfield adoption (agent skill)
```

The wiring recipe — the specmark path-dep, the first spec unit, the
expand-as-you-conform rhythm — is GUIDE §13; the sweep idioms are GUIDE
§14. The policies (`conform.toml`, `specmap.toml`) stay with YOUR project:
this package ships engines, never policy (PROP-024 §2.2).
