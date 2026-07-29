# AI-Native Go (stack:org.vibevm.ai-native/go-ai-native-lang) {#root}

<status stage="doc" state="done" audience="user"/>

##GO-PROJECTION-SHIPS-A-RUNNABLE-TOOLCHAIN The Go projection of the AI-Native Code Discipline — and the **runnable
toolchain** that enforces it (PROP-024 code-bearing packages):
installing this stack yields working checkers and procedures, not
descriptions of them. @impl/done

##THIRD-SUPPORTED-LANGUAGE-ON-THE-NEUTRAL-CORE Go is the Discipline's third supported language,
after Rust (the pilot) and TypeScript; the language-neutral method
comes from its dependency `flow:org.vibevm.ai-native/core-ai-native`
(^0.8 — the first edition carrying the Go fact/config/rule support in
the neutral engine). @impl/done

## What ships {#what-ships}

- ##SHIPS-FOUR-BINARIES **Four binaries** (this package's own Cargo workspace, `crates/`;
  names carry the `go-ai-native` family prefix per PROP-028 §2.4): @impl/done
  - ##SHIPS-GO-AI-NATIVE-UMBRELLA `go-ai-native` — the umbrella tool: `init` (bootstrap policies +
    registries), `floor` (the seven-step verification floor: gofmt →
    vet → tests → staticcheck+exhaustive → conform → specmap →
    test-gate), `conform`, `specmap`, `trace`, `test-gate` (xfail-strict
    over `go test -json`), `tripwire`, `health` (with the package-grain
    Example-coverage join), `fast-loop`, `codemod add-cell`. @impl/done
  - ##SHIPS-GO-AI-NATIVE-CONFORM `go-ai-native-conform` — the structural gate alone: the go-extract
    facts through the language-neutral engine (cell isolation, the
    §2/§5/§7 ban census with deviation testimony, file budget). @impl/done
  - ##SHIPS-GO-AI-NATIVE-SPECMAP `go-ai-native-specmap` — the traceability engine alone (PROP-014):
    `//spec:` directives → the committed index + the package-grain
    orphan ratchet. @impl/done
  - ##SHIPS-GO-AI-NATIVE-TCG `go-ai-native-tcg` — the agentic type oracle (TCG-ORACLE-GO /
    TCG-PROTOCOL-GO): a persistent enriching `serve` relay for MCP
    hosts plus one-shot `validate` / `scope` / `complete` / `type` /
    `bench`, answered by the CONSUMER's own gopls over in-memory
    overlays with the gate's own conform rules and the `//spec:` marker
    stream merged in. **Prerequisites:** go ≥ 1.24 and gopls
    (`go install golang.org/x/tools/gopls@latest`). Honesty: gopls
    stands on go/types — the reference implementation of the spec,
    tighter than rust-analyzer↔rustc, still not the compiler;
    `go-ai-native floor` stays the truth. @impl/done
- ##SHIPS-STDLIB-ONLY-FACT-EXTRACTOR **The stdlib-only fact extractor** (`tools/go-extract/extract.go`):
  go/parser + go/ast, zero third-party imports — embedded in the
  bridge, materialised content-addressed with a go.mod cut-off so a
  consumer's `./...` never compiles it as project code. @impl/done
- ##SHIPS-GUIDE-AND-CARDS **The Go guide and cards** (`spec/go/GUIDE-AI-NATIVE-GO.md`,
  `spec/cards/` — the nine scaffolds in their Go shape, Band-3 ops
  blocks for weak readers). @impl/done
- ##SHIPS-TWO-AGENT-SKILLS **Two agent skills** (`vibe skill install` projects them):
  `/go-ai-native-terraform` (brownfield adoption per
  BROWNFIELD-PROTOCOL) and `/go-ai-native-sweep` (the recurring sweep). @impl/done

## External tooling — the complete list {#external-tooling}

##external-tooling-lead Everything the stack touches outside its own crates, consolidated
(normative homes: GUIDE §1 baseline, GUIDE §14 wiring, TCG-ORACLE-GO §1): @impl/done

| Tool | Role | License | Required? | Resolution / recipe |
|---|---|---|---|---|
| ##ROW-TOOL-GO **go ≥ 1.24** (gofmt ships with it) @spec/done | floor steps gofmt/vet/tests/test-gate; `go run` for go-extract; bench @spec/done | BSD-3 @spec/done | **MUST** — absence is a recipe-carrying failure, never a skip @spec/done | PATH, or env `GO_AI_NATIVE_GO` pointing at the binary @spec/done |
| ##ROW-TOOL-GOPLS **gopls** @spec/done | the agentic tcg oracle (validate/scope/complete/type over overlays) @spec/done | BSD-3 @spec/done | **MUST** for the tcg surface @spec/done | env `GO_AI_NATIVE_GOPLS` → PATH → `GOBIN` → `GOPATH/bin`; `go install golang.org/x/tools/gopls@latest` @spec/done |
| ##ROW-TOOL-STATICCHECK **staticcheck** @spec/done | correctness evidence provider (floor step `staticcheck`) @spec/done | MIT @spec/done | policy-gated — disable with a reason in `[go].floor_disable`; the disablement prints every run @spec/done | `go install honnef.co/go/tools/cmd/staticcheck@latest` @spec/done |
| ##ROW-TOOL-EXHAUSTIVE **exhaustive** @spec/done | THE carrier of closed-set switch exhaustiveness (Go has no sum types — GUIDE §5) @spec/done | BSD-2 @spec/done | policy-gated, same step @spec/done | `go install github.com/nishanths/exhaustive/cmd/exhaustive@latest` — note: v0.12.0 does not compile under go ≥ 1.26 (its pinned x/tools); build from master with a bumped x/tools until a release lands @spec/done |
| ##ROW-TOOL-GOVULNCHECK **govulncheck** @spec/done | supply-chain scan @spec/done | BSD-3 @spec/done | CI-posture only (network-touching — never a floor step) @spec/done | `go install golang.org/x/vuln/cmd/govulncheck@latest` @spec/done |
| ##ROW-TOOL-GIT **git** @spec/done | tripwire's change-set collection @spec/done | GPLv2 (tool, spawned) @spec/done | needed by `tripwire` only @spec/done | any PATH git @spec/done |
| ##ROW-TOOL-CARGO **cargo / Rust toolchain** @spec/done | building the stack's own binaries from the slot @spec/done | MIT/Apache-2.0 @spec/done | build-time only (a vibevm code-bearing-package property, not a Go one) @spec/done | rustup @spec/done |

##deliberately-absent-lead **Deliberately absent:** @impl/done

- ##ABSENT-GOLANGCI-LINT golangci-lint (GPL-3.0 — banned by the
  licensing flow; at most a personal separate-process dev tool), @impl/done
- ##ABSENT-NODE-NPM node/npm (the TS stack's need, not ours), @impl/done
- ##ABSENT-RUST-ANALYZER rust-analyzer, @impl/done
- ##ABSENT-LLVM-CLANG-MACHINERY any
  LLVM/clang-class machinery. @impl/done

##EXTRACTOR-IS-PURE-GO-STDLIB The fact extractor is **pure Go stdlib**
(zero third-party imports), so the only external process on the
critical path is the language's own official analyzer. @impl/done

## Running the tools {#running-the-tools}

##running-forms-lead Three supported forms, from your project root (where `vibedeps/` is): @impl/done

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

##STACK-SLOT-IS-THE-MATERIALISED-DIRECTORY `<stack-slot>` is this package's materialised directory — check your
`vibe.lock`. @impl/done

##SLOT-BUILD-DROPS-A-TARGET-DIRECTORY Building in the slot drops a `target/` there; add
`vibedeps/**/target/` to your `.gitignore`. @impl/done

## The lifecycle {#the-lifecycle}

```sh
vibe install                 # materialise this stack into vibedeps/
go-ai-native init            # policies + registries + external spec resolution
# … write spec units, tag packages (//spec:scope in doc.go — GUIDE §8),
#   adopt package by package …
go-ai-native floor           # the gate panel, one exit code
/go-ai-native-sweep          # the recurring sweep (agent skill)
/go-ai-native-terraform      # brownfield adoption (agent skill)
```

##wiring-and-sweep-pointers The wiring recipe is GUIDE §14; the sweep idioms are GUIDE §15. @impl/done

##POLICIES-STAY-WITH-THE-CONSUMER-PROJECT The policies (`conform.toml` with its `[go]` table, `specmap.toml`) stay
with YOUR project: this package ships engines, never policy. @impl/done

##WORKED-PILOT-IS-RESEARCH-GO-DEMO The worked pilot lives in the vibevm dev tree at `research/go-demo` — a
miniature reconciler carrying the whole consumer layout: `go.mod`, cells
under `internal/cells/`, `conform.toml`, `specmap.toml` and a populated
`discipline/`. @impl/done
