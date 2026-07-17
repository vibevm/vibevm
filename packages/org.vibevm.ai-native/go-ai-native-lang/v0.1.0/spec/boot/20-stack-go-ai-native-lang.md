# AI-Native Go (Discipline v0.2) — boot snippet

Go code in this project follows the AI-Native Go guide
(`go/GUIDE-AI-NATIVE-GO.md` in this package). Read the guide when
authoring or reviewing structure; per-edit work needs only the card
whose trigger fires.

Card registry for Go: `cards/INDEX.md` in this package (trigger → card;
the nine executable scaffolds A–I in their Go shape). This stack ships
its own `cards/` projection — the weak-reader runtime surface for a
`.go` edit is a Go card's Band-3 ops block, never another language's.

Standing rules at the surface level:

- Ordinary idiomatic Go at the token level — no invented dialect; the
  language's own prescriptions (gofmt, one idiom culture, errors as
  values) are taken whole. Strictness lives in the envelope: closed
  error sets at seams, loud interface conformance, owned goroutines,
  `//spec:` metadata, per-cell fast verification
  (`go test ./internal/cells/<cell>/ -race`, < ~60s).
- Cells: one cell = one package under `internal/cells/<name>` with a
  `New(...)` constructor as the surface; cells import seams + core
  only, never sibling cells. `init()`, blank imports, and package-level
  mutable state are banned in cells; capabilities (clock, env, net,
  fs, randomness) are injected as narrow consumer-side interfaces.
  Every cell carries `var _ Seam = (*Impl)(nil)`.
- Each seam owns a closed, enumerated error set (`Code + Spec + Err`,
  `errors.As`-consumed); error messages cite the violated `spec://`
  REQ and a fix surface. Expected failures are never panics; panic is
  the invariant-violation channel only. Exhaustive handling of closed
  const-enum sets is carried by the `exhaustive` linter — the one rule
  a linter carries entirely, named honestly.
- Every goroutine has an owner (errgroup / WaitGroup + context); naked
  `go` with cell-outliving lifetime is banned; `go test -race` gates
  any package that starts one.
- Every public seam carries one `Example` function (compiled AND run;
  `// Output:` diffed). Replacing a non-trivial cell requires a
  differential fuzz oracle with a committed seed corpus.
- Uniformity is load-bearing: one idiom per operation; exceptions are
  marked (`//spec:deviates … reason`), or they propagate as false
  training signal.

The shipped toolchain (this stack materialises it; no dev tree needed):
`go-ai-native` — `init` (bootstrap policies + registries), `floor`
(gofmt→vet→test→staticcheck+exhaustive→conform→specmap→test-gate, one
exit code), `health` (the sweep's fact collector), `test-gate`
(xfail-strict over `go test -json`) / `tripwire` / `trace` /
`fast-loop` / `codemod`; plus the narrow `go-ai-native-conform` and
`go-ai-native-specmap` engines, and the agentic type oracle
`go-ai-native-tcg` (also served over MCP by
`mcp:org.vibevm.ai-native/go-ai-native-mcp` — PROP-027; persistent
enriching `serve` relay + one-shot `validate`/`scope`/`complete`/
`type`/`bench`: check an edit against in-memory overlays BEFORE
writing it, answered by the CONSUMER's own gopls with the SAME conform
rules as the gate — GUIDE §13, §14 move 5; prerequisites go ≥ 1.24 +
`go install golang.org/x/tools/gopls@latest`; honesty: gopls stands on
go/types, the reference implementation of the spec — tighter than
rust-analyzer↔rustc, still not the compiler; the floor stays the
truth). Run vibe-natively (`vibe bin exec go-ai-native -- <args>` —
PROP-025 lockfile dispatch; `vibe bin build` pre-builds), from PATH
(`cargo install --path vibedeps/<stack-slot>/crates/go-ai-native-cli`),
or in place via `cargo run --manifest-path
vibedeps/<stack-slot>/Cargo.toml -p go-ai-native-cli --bin
go-ai-native -- <args>`. Wiring recipe: GUIDE §14; sweep idioms:
GUIDE §15. Procedures as agent skills: `/go-ai-native-sweep`
(recurring), `/go-ai-native-terraform` (brownfield adoption) —
`vibe skill install` projects them.
