# AI-Native Rust (stack:org.vibevm/rust-ai-native-lang)

The Rust projection of the AI-Native Code Discipline — and the **runnable
toolchain** that enforces it (PROP-024 code-bearing packages): installing
this stack yields working checkers and procedures, not descriptions of
them. The language-neutral method (manifesto, playbooks, mechanism specs)
comes from its dependency `flow:org.vibevm/core-ai-native`.

## What ships

- **Four binaries** (this package's own Cargo workspace, `crates/`;
  crate names carry the `-rust` suffix per the GUIDE §2 language-suffix
  rule):
  - `rust-ai-native` — the umbrella tool: `init` (bootstrap policies +
    registries), `floor` (the portable verification floor), `conform`,
    `specmap`, `trace`, `test-gate`, `tripwire`, `health`, `fast-loop`,
    `codemod`.
  - `rust-ai-native-conform` — the conformance gate alone (ENGINE-CONFORM).
  - `rust-ai-native-specmap` — the traceability engine alone (PROP-014).
  - `rust-ai-native-tcg` — the agentic type oracle (TCG-ORACLE-RUST /
    TCG-PROTOCOL-RUST): a persistent enriching `serve` relay for MCP
    hosts plus one-shot `validate` / `scope` / `complete` / `type` /
    `bench`, answered by the CONSUMER's own rust-analyzer over
    in-memory overlays with the gate's own conform rules merged in.
    **Prerequisite:** installing this stack obliges the machine to
    carry rust-analyzer (`rustup component add rust-analyzer`).
    Honesty: rust-analyzer is not rustc — the oracle shortens the
    distance to green; `rust-ai-native floor` stays the truth.
- **The Rust guide and cards** (`spec/rust/GUIDE-AI-NATIVE-RUST.md`,
  `spec/cards/` — the nine scaffolds in their Rust shape, Band-3 ops
  blocks for weak readers).
- **Two agent skills** (`vibe skill install` projects them):
  `/rust-ai-native-terraform` (brownfield adoption per BROWNFIELD-PROTOCOL) and
  `/rust-ai-native-sweep` (the recurring sweep per the Sweep Playbook).
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
vibe bin build            # or: vibe bin exec rust-ai-native -- floor
vibe bin exec rust-ai-native -- floor

# (b) install once onto PATH — then just `rust-ai-native …`
cargo install --path vibedeps/<stack-slot>/crates/rust-ai-native-cli

# (c) zero-install, run in place
cargo run --manifest-path vibedeps/<stack-slot>/Cargo.toml \
    -p rust-ai-native-cli --bin rust-ai-native -- floor
```

`<stack-slot>` is this package's materialised directory (e.g.
`stack-rust-ai-native-lang/0.7.0` — check your `vibe.lock`). Building in the
slot drops a `target/` there; add `vibedeps/**/target/` to your
`.gitignore` (build output is already excluded from the package's content
hash, PROP-024 §2.2).

## The lifecycle

```sh
vibe install                       # materialise this stack into vibedeps/
rust-ai-native init               # policies + registries + external spec resolution
# … write spec units, tag code (GUIDE §13), adopt crate by crate …
rust-ai-native floor              # the gate panel, one exit code
/rust-ai-native-sweep                  # the recurring sweep (agent skill)
/rust-ai-native-terraform                    # brownfield adoption (agent skill)
```

The wiring recipe — the specmark path-dep, the first spec unit, the
expand-as-you-conform rhythm — is GUIDE §13; the sweep idioms are GUIDE
§14. The policies (`conform.toml`, `specmap.toml`) stay with YOUR project:
this package ships engines, never policy (PROP-024 §2.2).
