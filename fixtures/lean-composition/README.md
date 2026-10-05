# Selected Rust and Lean subject

This fixture commissions
`spec://org.example/same-subject/Protocol#identity` with two separate facts:

- Rust `rust::identity` has an authored `implements` edge. Two explicitly named
  libtests check zero and `u64::MAX`.
- Lean `lean::Example.identity` has an authored `verifies` edge. Its checked
  theorem is reflexive equality over `Nat`, at grade `g1_model`.

A successful joined check has `correspondence = "not_established"`. It proves
neither that the Rust implementation refines the Lean model nor that a whole
product satisfies its specification. The tests are example evidence. An edge
in a declaration map is not a successful checker result or acceptance decision.

The selected subject is an authored `@fact:identity`, so the real requirements
query exposes its row and both relations. Facts have no `r<N>` revision in the
core grammar: both carriers leave the revision absent, and witnesses bind that
absence plus exact spec content/file hashes. The authored `spec/done` marker
describes the fixture specification, not Rust refinement or proof acceptance.
An invented revision pin refuses rather than being silently ignored.

## One complete declaration map

From a built host checkout, with this directory as `--path`:

```sh
vibe specmap --current --path /absolute/path/to/fixture --json --offline
```

`specmap.toml` opts into schema 1 `rust_lean` composition. The host removes only
that typed table, delegates the remaining configuration to the existing core
parser, and composes its default Rust/JTD scanner with the Lean carrier scanner.
The host validates the whole map before one atomic replacement of `specmap.json`.
The fresh relation provider runs that same composition without writing a file.
Missing sources, malformed/duplicate configuration, wrong namespace and stale
edge pins cannot become a partial-current map.

The neutral core and family writers are unchanged. Their closed configuration
parser refuses `[composition]` before writing. Exercise the actual separately
built Rust and TypeScript front doors with:

```sh
python3 check-legacy-writers.py \
  --rust-binary /absolute/path/to/rust-ai-native-specmap \
  --typescript-binary /absolute/path/to/typescript-ai-native-specmap \
  --receipt /absolute/path/to/refusal-receipt.json
```

Each writer receives its own temporary fixture and sentinel map. Passing means
an unknown-`composition` refusal and byte-identical retained map. The child PATH
is empty: a missing Node/extractor error is not accepted as the required guard.
The canonical Rust symbol follows the scanner's directory identity `rust`.
Lean locations point at the authored carrier (line 1), not a fabricated theorem
source position. This map supports only the selected consumer's own namespace;
foreign inbound proof coverage is explicitly unsupported.

## Uncached executable checks

`proof-policy.toml` enrolls the exact subject, two distinct qualified execution
IDs, exact libtest names, native target, Rust release/commit, manifest and source
list. The carrier separately pins Lean 4.34.1 and its permitted axioms.
Use reproducible toolchain distributions and set their absolute installation
roots at trusted startup; those machine paths do not belong in either policy.

```sh
export RUST_LEAN_RUST_TOOLCHAIN_DIR=/absolute/path/to/rust-toolchain
export LEAN_AI_NATIVE_TOOLCHAIN_DIR=/absolute/path/to/lean-4.34.1
vibe proof-check --path /absolute/path/to/fixture \
  --subject spec://org.example/same-subject/Protocol#identity \
  --witness joined-proof.json --json --offline
vibe proof-verify --path /absolute/path/to/fixture \
  --subject spec://org.example/same-subject/Protocol#identity \
  --witness joined-proof.json --json --offline
```

The selected Rust pin is release `1.101.0-nightly`, commit
`c36f1457196e315bc204b9564a6a5a7fe7f5a51f`, host
`x86_64-unknown-linux-gnu` (commit date 2026-10-01). The trusted directory must
contain its real `bin/cargo`, `bin/rustc` and `lib` closure. The Lean carrier uses
`leanprover/lean4:v4.34.1`; the Lean family verifies its release/commit and kernel
replay as described in that package. A mutable toolchain alias alone is not the
witness identity: both compiled runners capture exact toolchain byte digests.

`proof-check` executes both contributions and atomically saves schema
`vibevm/selected-proof-join/1`. `proof-verify` treats saved JSON as a claim: it
captures its exact bytes, freshly compiles and runs the Rust checks, calls the
existing Lean `VerifyWitness` operation (including kernel replay and axiom
inspection), compares every joined binding, and rereads the saved join. A
`matched` result returns the exact saved-byte digest observed before and after
checking. It does not lock that path against a later or intervening writer.
Verification never refreshes or overwrites a stale witness to make it pass.

Both operations return a typed `Reply` (`ok`, `result` or `error`), including
separate Rust and Lean contribution outcomes, input/spec/toolchain identities,
G1 grade and explicit absent correspondence. Qualified execution IDs identify
selected contributions; they do not authenticate historical process records.
Results contain no automatic project or requirement acceptance flag.

The CLI flags `--lean-toolchain-dir` and `--rust-toolchain-dir` are equivalent
trusted startup overrides. MCP `proof_check` and `proof_verify` take only
`subject` and `witness`; the server supplies its project root and startup
environment. They call the same engine and expose the identical typed Reply in
`structuredContent` with `isError = !ok`. Stdout is a presentation of that Reply,
not the proof authority. Generic lifecycle cache hits cannot satisfy this
uncached operation; this slice does not alter lifecycle reports or cache policy.

## Exact boundary and limitations

The first selected Rust checker supports one native Linux library, without Cargo
dependencies, build scripts, features, custom targets or workspace inheritance.
Its manifest and lock must describe exactly that package. The declared sources
must equal all local `.rs` files (excluding `target` and `.git`). Only captured
inputs enter a temporary build tree. Cargo runs offline/frozen with an empty
private home/target and controlled environment. Compilation cannot count as a
pass by itself: each selected test must exist and report one executed pass.

The selected checker conservatively reserves the token identifiers `include`,
`include_str`, `include_bytes`, `env`, `option_env` and `path`, including inside
macro definitions and conditional attributes, to reject undeclared compile-time
inputs. This is a deliberately narrow source contract. Rust tests and Lean files
remain trusted executable code; the runner is not a sandbox and does not prove
absence of arbitrary runtime I/O. Native OS/runtime and compiler/linker correctness
are trusted. Exact Rust/Lean `bin`/`lib` closures, compiler versions and the host
`cc`/`ld`/`as` bytes are recorded; the operating system is not formally verified.

Each Rust subprocess is limited to 120 seconds and 1 MiB output, with process
group cleanup on timeout. The Lean carrier has its own bounded checker policy.
Inputs are rechecked in both the original tree and the captured snapshot; selected
Rust source inventory and Rust toolchain are checked again after Lean completes.
The entire composed map is then rebuilt and its canonical bytes must equal the
first scan, so added or removed scan/spec-root members cannot leave an old
`map_sha256` marked current. This is the same two-snapshot boundary, without a
promise of future path stability.
Witness parents must already exist; source/map output overlap and links refuse.
No output is published until both actual checks succeed.

The composed host currently links the canonical Lean library by a repository
source path at build time. Family package versions stay at their existing values (Lean 0.1.0,
Rust/core 1.1.0); the host build contains both neutral-core crate versions as
separate dependencies. No neutral family sources were edited. This is evidence
for the selected host capability, not a generic multi-language plugin framework
or cross-package proof join.

## Installed consumer check

Copy this fixture to a disposable consumer before installation. Install both
families from a local source catalog, then build their declared tools in the
materialized slots:

```sh
vibe install --path CONSUMER --offline --no-default-registry --registry CATALOG \
  --exact --assume-yes stack:org.vibevm.ai-native/rust-ai-native@1.1.0 \
  stack:org.vibevm.ai-native/lean-ai-native@0.1.0
cd CONSUMER
vibe bin build --offline --assume-yes rust-ai-native-conform \
  rust-ai-native-specmap lean-ai-native lean-ai-native-mcp
vibe bin exec rust-ai-native-conform -- --path . check --scope rust
vibe bin exec lean-ai-native -- --path . check --carrier lean-proof.toml \
  --subject spec://org.example/same-subject/Protocol#identity
vibe requirements --path . --address-prefix \
  spec://org.example/same-subject/Protocol#identity --relations --json
```

The selected gate builds and runs those installed artifacts with both canonical
VibeVM checkout roots hidden and networking disabled. Installed Lean CLI/MCP
return matching checked evidence; the host join's Lean contribution equals that
installed result. The actual requirements query returns one fact row containing
both current relations. The Rust conform policy gates this one crate without a
frozen baseline; its usage doctest is executable separately with `cargo test
--manifest-path rust/Cargo.toml --doc`.

The host join remains a compiled-in checker: it does not dispatch to the installed
Lean executable or infer checker availability from the installation lock. A new
installed family version does not silently replace that compiled engine. This
separation between build-time linkage, installed package availability and runtime
checking is part of the gate result, not a claim of dynamic plugin discovery.

## Focused checks

Fast composition/protocol tests:

```sh
cargo +nightly test --offline -p vibe-trace composition::tests
cargo +nightly test --offline -p vibe-trace proof::tests
cargo +nightly test --offline -p vibe-cli -p vibe-mcp --test cli_proof --test proof_tools
```

Real engine and surface tests are explicitly ignored unless selected with both
trusted toolchain environment variables above:

```sh
cargo +nightly test --offline -p vibe-trace proof::tests::real_selected_checks_and_saved_join_recheck -- --exact --ignored
cargo +nightly test --offline -p vibe-trace proof::tests::real_selected_join_negative_matrix -- --exact --ignored
cargo +nightly test --offline -p vibe-cli --test cli_proof -- --ignored
```

The negative matrix changes actual fixture inputs and saved claims: missing or
failed checkers/tests/theorems, unexpected axioms, stale revision/source/dependency
bindings, wrong subject, forged outcome/toolchain/axiom assumptions and G3 refusal.
A deterministic injected-boundary test also replaces the saved join while a
checker is running; the final snapshot comparison refuses it.
