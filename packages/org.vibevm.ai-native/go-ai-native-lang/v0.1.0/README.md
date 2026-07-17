# AI-Native Go (stack:org.vibevm.ai-native/go-ai-native-lang)

The Go projection of the AI-Native Code Discipline — and the **runnable
toolchain** that enforces it (PROP-024 code-bearing packages):
installing this stack yields working checkers and procedures, not
descriptions of them. Go is the Discipline's third supported language,
after Rust (the pilot) and TypeScript; the language-neutral method
comes from its dependency `flow:org.vibevm.ai-native/core-ai-native`
(^0.8 — the first edition carrying the Go fact/config/rule support in
the neutral engine).

## What ships

- **Four binaries** (this package's own Cargo workspace, `crates/`;
  names carry the `go-ai-native` family prefix per PROP-028 §2.4):
  - `go-ai-native` — the umbrella tool: `init` (bootstrap policies +
    registries), `floor` (the seven-step verification floor: gofmt →
    vet → tests → staticcheck+exhaustive → conform → specmap →
    test-gate), `conform`, `specmap`, `trace`, `test-gate` (xfail-strict
    over `go test -json`), `tripwire`, `health` (with the package-grain
    Example-coverage join), `fast-loop`, `codemod add-cell`.
  - `go-ai-native-conform` — the structural gate alone: the go-extract
    facts through the language-neutral engine (cell isolation, the
    §2/§5/§7 ban census with deviation testimony, file budget).
  - `go-ai-native-specmap` — the traceability engine alone (PROP-014):
    `//spec:` directives → the committed index + the package-grain
    orphan ratchet.
  - `go-ai-native-tcg` — the agentic type oracle (TCG-ORACLE-GO /
    TCG-PROTOCOL-GO): a persistent enriching `serve` relay for MCP
    hosts plus one-shot `validate` / `scope` / `complete` / `type` /
    `bench`, answered by the CONSUMER's own gopls over in-memory
    overlays with the gate's own conform rules and the `//spec:` marker
    stream merged in. **Prerequisites:** go ≥ 1.24 and gopls
    (`go install golang.org/x/tools/gopls@latest`). Honesty: gopls
    stands on go/types — the reference implementation of the spec,
    tighter than rust-analyzer↔rustc, still not the compiler;
    `go-ai-native floor` stays the truth.
- **The stdlib-only fact extractor** (`tools/go-extract/extract.go`):
  go/parser + go/ast, zero third-party imports — embedded in the
  bridge, materialised content-addressed with a go.mod cut-off so a
  consumer's `./...` never compiles it as project code.
- **The Go guide and cards** (`spec/go/GUIDE-AI-NATIVE-GO.md`,
  `spec/cards/` — the nine scaffolds in their Go shape, Band-3 ops
  blocks for weak readers).
- **Two agent skills** (`vibe skill install` projects them):
  `/go-ai-native-terraform` (brownfield adoption per
  BROWNFIELD-PROTOCOL) and `/go-ai-native-sweep` (the recurring sweep).

## Running the tools

Three supported forms, from your project root (where `vibedeps/` is):

```sh
# (a) vibe-native (PROP-025) — build once in the slot, dispatch through
#     the project's lockfile:
vibe bin build
vibe bin exec go-ai-native -- floor

# (b) install once onto PATH — then just `go-ai-native …`
cargo install --path vibedeps/<stack-slot>/crates/go-ai-native-cli

# (c) zero-install, run in place
cargo run --manifest-path vibedeps/<stack-slot>/Cargo.toml \
    -p go-ai-native-cli --bin go-ai-native -- floor
```

`<stack-slot>` is this package's materialised directory — check your
`vibe.lock`. Building in the slot drops a `target/` there; add
`vibedeps/**/target/` to your `.gitignore`.

## The lifecycle

```sh
vibe install                 # materialise this stack into vibedeps/
go-ai-native init            # policies + registries + external spec resolution
# … write spec units, tag packages (//spec:scope in doc.go — GUIDE §8),
#   adopt package by package …
go-ai-native floor           # the gate panel, one exit code
/go-ai-native-sweep          # the recurring sweep (agent skill)
/go-ai-native-terraform      # brownfield adoption (agent skill)
```

The wiring recipe is GUIDE §14; the sweep idioms are GUIDE §15. The
policies (`conform.toml` with its `[go]` table, `specmap.toml`) stay
with YOUR project: this package ships engines, never policy. The
worked pilot lives in the vibevm dev tree at `research/go-demo` — a
miniature reconciler with the whole chain green.
