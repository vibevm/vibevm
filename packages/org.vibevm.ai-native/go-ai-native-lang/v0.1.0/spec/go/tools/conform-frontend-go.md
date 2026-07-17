# Tool Spec: `go-ai-native-conform-frontend` + `go-extract` — the Go frontend for the language-neutral conform engine
*Status: **SHIPPED with this package** (GO-AI-NATIVE-PLAN Phases 4–5) —
`crates/go-ai-native-conform-frontend` (`id = "go-extract"`) +
`crates/go-ai-native-conform` (binary **`go-ai-native-conform`**), fed by
the stdlib-only extractor at `tools/go-extract/` through the
`go-ai-native-extract-bridge` NDJSON protocol. The Go counterpart of
`rust-ai-native-conform-frontend` (in-process syn) and
`typescript-ai-native-conform-frontend` (node sidecar): it gives `.go`
code the SAME structural discipline gate, by feeding Go facts into the
language-neutral engine — never by re-implementing the rules in a Go
linter.*

## 1. The division of labour with the native Go tooling {#division}

`req r1`

Go's own toolchain already carries the **type / correctness** half:
`go build` (the compile gate), `go vet` (shipped correctness census),
staticcheck and the `exhaustive` linter as evidence providers (guide
§1, §5). Those answer *"is this well-typed and locally sane?"* — the
half the language does natively and well.

This frontend answers the **other** half — the *structural /
architectural* rules no Go tool expresses, the ones `conform check`
already enforces for Rust and TypeScript:

- the file-length budget (position is a resource, guide §3);
- cell isolation (a cell imports seams + core only, never sibling
  cells; the registry is the only cell importer — guide §2/§6);
- the ban census as facts (`init()` in a cell, blank imports, ambient
  defaults, naked `go` statements, error-string matching, reasonless
  suppressions, `t.Skip` — guide §7's theater list) surfaced as conform
  findings in the Class-F `violates REQ …; fix surface: …` grammar;
- the seam-error contract (`seam-error-cites-req` — a seam's closed
  error set carries its REQ URI, guide §5);
- the deviation escape hatch (`//spec:deviates … reason="…"`), honoured
  the way `#[spec(deviates)]` is for Rust.

Routing these through conform keeps **one rule engine, one finding
grammar, one ratchet baseline** across all three languages, with the
rules defined once over `conform_core::Fact` and fed by any frontend —
a rule cannot drift between projections.

## 2. What go-extract is {#extractor}

`req r1`

A fact producer: parse a `.go` file and emit the language-neutral fact
stream the rules consume. **Stdlib-only by construction** — `go/parser`,
`go/ast`, `go/token`, `encoding/json`, nothing else — so
`go run extract.go` works with no module context, no `go.mod`, no
network, on any machine that carries the toolchain the floor already
requires. This is the owner's "minimum external tooling" ideal made
structural: the language parses itself, one file, zero dependencies.

- **Delivery:** embedded in the Rust bridge crate (`include_str!`),
  materialised content-addressed to
  `<project>/target/conform/go-extract/extract-<hash16>.go` before
  spawn — the proven ts-extract mechanism. Because exactly one file is
  materialised, the source stays import-free of sibling tool files.
- **Protocol:** NDJSON on stdio — a `{proto, files: [...]}` request, one
  `{file, facts: [...], markers: [...]}` record per input, `PROTOCOL =
  1`, additive evolution, unparseable file → zero facts + a `degraded`
  note, never a crash (B5).
- **Fact kinds** (the ts-extract vocabulary, Go-shaped): `item`
  (func / method / type / const / var; exported flag; receiver;
  attached `//spec:` directives), `import` (path, blank flag),
  `go_unsafe` (the ban census sites with their kind:
  `init_decl` / `blank_import` / `ambient_call` / `naked_go` /
  `error_string_match` / `t_skip` / `reasonless_suppression`),
  `file_metrics` (physical lines), and `marker` (the `//spec:`
  directive stream: tag, uri, r, reason, attached symbol, line).
- **One extraction, two consumers:** the conform frontend consumes
  `facts`; the specmap scanner consumes `markers`; the tcg relay
  consumes both (TCG-PROTOCOL-GO §3). One parser, one vocabulary, no
  drift.

## 3. The frontend crate {#frontend}

`req r1`

`go-ai-native-conform-frontend` implements the engine's `Frontend`
trait: an `id()` of `"go-extract"`, a `version()` that bumps when the
fact schema grows (retiring cache slots wholesale, exactly as the
sibling frontends do), and `extract(file, package, module, text) ->
Vec<Fact>` that round-trips through the bridge. Facts are keyed
`(file content-hash, frontend id+version)` in the engine's
content-addressed store — a 1-file diff re-extracts 1 file (A2).

## 4. Topology: the `[go]` policy section {#topology}

`req r1`

`conform.toml` gains a `[go]` section, written by `go-ai-native init`
from the module layout: `roots` (source roots to scan), `cells_dir`
(default `internal/cells`), `seams_pkg` (default `internal/seams`),
`registry_pkg` (default `internal/registry`), `gated_packages` /
`[[exempt]]` (the expand-as-you-conform ratchet — a package enters the
gate only at zero findings), and the file budget. The
every-package-gated-or-exempt invariant is enforced by the engine on
every check, exactly as for the sibling stacks.

## 5. The honest note {#honesty}

The structural gate is only as good as its facts, and go-extract is a
PARSER, not a type checker: it sees syntax and directives, not
resolved types. Rules that would need type information (e.g. "this
call's receiver is a seam type") are out of this tool's scope and
belong to the vet/staticcheck/exhaustive evidence tier or the oracle.
The division is deliberate — the same three-tier split (T-lex/T-syn/
T-sem) ENGINE-CONFORM §1 defines; go-extract is the T-syn tier, and
Go's T-sem tier is the toolchain itself.
