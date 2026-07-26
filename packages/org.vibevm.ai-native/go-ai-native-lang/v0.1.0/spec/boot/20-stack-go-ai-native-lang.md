# AI-Native Go (Discipline v0.2) — boot snippet {#root}

<status stage="impl" state="done"/>

##GO-CODE-FOLLOWS-THE-GO-GUIDE Go code in this project follows the AI-Native Go guide
(`go/GUIDE-AI-NATIVE-GO.md` in this package). @impl/done

##READ-THE-GUIDE-FOR-STRUCTURE Read the guide when
authoring or reviewing structure; per-edit work needs only the card
whose trigger fires. @impl/done

##CARD-REGISTRY-FOR-GO Card registry for Go: `cards/INDEX.md` in this package (trigger → card;
the nine executable scaffolds A–I in their Go shape). @impl/done

##STACK-SHIPS-ITS-OWN-CARDS-PROJECTION This stack ships
its own `cards/` projection — the weak-reader runtime surface for a
`.go` edit is a Go card's Band-3 ops block, never another language's. @impl/done

##standing-rules-lead Standing rules at the surface level: @impl/done

- ##RULE-ORDINARY-IDIOMATIC-GO-SURFACE Ordinary idiomatic Go at the token level — no invented dialect; the
  language's own prescriptions (gofmt, one idiom culture, errors as
  values) are taken whole. Strictness lives in the envelope: closed
  error sets at seams, loud interface conformance, owned goroutines,
  `//spec:` metadata, per-cell fast verification
  (`go test ./internal/cells/<cell>/ -race`, < ~60s). @impl/done
- ##RULE-CELLS Cells: one cell = one package under `internal/cells/<name>` with a
  `New(...)` constructor as the surface; cells import seams + core
  only, never sibling cells. `init()`, blank imports, and package-level
  mutable state are banned in cells; capabilities (clock, env, net,
  fs, randomness) are injected as narrow consumer-side interfaces.
  Every cell carries `var _ Seam = (*Impl)(nil)`. @impl/done
- ##RULE-CLOSED-SEAM-ERROR-SETS Each seam owns a closed, enumerated error set (`Code + Spec + Err`,
  `errors.As`-consumed); error messages cite the violated `spec://`
  REQ and a fix surface. Expected failures are never panics; panic is
  the invariant-violation channel only. Exhaustive handling of closed
  const-enum sets is carried by the `exhaustive` linter — the one rule
  a linter carries entirely, named honestly. @impl/done
- ##RULE-OWNED-GOROUTINES Every goroutine has an owner (errgroup / WaitGroup + context); naked
  `go` with cell-outliving lifetime is banned; `go test -race` gates
  any package that starts one. @impl/done
- ##RULE-EXAMPLE-PER-SEAM Every public seam carries one `Example` function (compiled AND run;
  `// Output:` diffed). Replacing a non-trivial cell requires a
  differential fuzz oracle with a committed seed corpus. @impl/done
- ##RULE-UNIFORMITY-IS-LOAD-BEARING Uniformity is load-bearing: one idiom per operation; exceptions are
  marked (`//spec:deviates … reason`), or they propagate as false
  training signal. @impl/done

##shipped-toolchain-lead The shipped toolchain (this stack materialises it; no dev tree needed): @impl/done

- ##TOOLCHAIN-GO-AI-NATIVE-UMBRELLA `go-ai-native` — `init` (bootstrap policies + registries), `floor`
  (gofmt→vet→test→staticcheck+exhaustive→conform→specmap→test-gate, one
  exit code), `health` (the sweep's fact collector), `test-gate`
  (xfail-strict over `go test -json`) / `tripwire` / `trace` /
  `fast-loop` / `codemod`; @impl/done
- ##TOOLCHAIN-NARROW-ENGINES plus the narrow `go-ai-native-conform` and
  `go-ai-native-specmap` engines, @impl/done
- ##TOOLCHAIN-AGENTIC-TYPE-ORACLE and the agentic type oracle
  `go-ai-native-tcg` (also served over MCP by
  `mcp:org.vibevm.ai-native/go-ai-native-mcp` — PROP-027; persistent
  enriching `serve` relay + one-shot `validate`/`scope`/`complete`/
  `type`/`bench`: check an edit against in-memory overlays BEFORE
  writing it, answered by the CONSUMER's own gopls with the SAME conform
  rules as the gate — GUIDE §13, §14 move 5; prerequisites go ≥ 1.24 +
  `go install golang.org/x/tools/gopls@latest`; honesty: gopls stands on
  go/types, the reference implementation of the spec — tighter than
  rust-analyzer↔rustc, still not the compiler; the floor stays the
  truth). @impl/done

##RUN-VIBE-NATIVELY-FROM-PATH-OR-IN-PLACE Run vibe-natively (`vibe bin exec go-ai-native -- <args>` —
PROP-025 lockfile dispatch; `vibe bin build` pre-builds), from PATH
(`cargo install --path vibedeps/<stack-slot>/crates/go-ai-native-cli`),
or in place via `cargo run --manifest-path
vibedeps/<stack-slot>/Cargo.toml -p go-ai-native-cli --bin
go-ai-native -- <args>`. @impl/done

##wiring-and-sweep-pointers Wiring recipe: GUIDE §14; sweep idioms:
GUIDE §15. @impl/done

##PROCEDURES-AS-AGENT-SKILLS Procedures as agent skills: `/go-ai-native-sweep`
(recurring), `/go-ai-native-terraform` (brownfield adoption) —
`vibe skill install` projects them. @impl/done
