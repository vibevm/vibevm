# Tool Spec: `go-ai-native-conform-frontend` + `go-extract` — the Go frontend for the language-neutral conform engine {#root}

<status stage="spec" state="done"/>

##status-line *Status: **SHIPPED with this package** (GO-AI-NATIVE-PLAN Phases 4–5) —
`crates/go-ai-native-conform-frontend` (`id = "go-extract"`) +
`crates/go-ai-native-conform` (binary **`go-ai-native-conform`**), fed by
the stdlib-only extractor at `tools/go-extract/` through the
`go-ai-native-extract-bridge` NDJSON protocol.* @impl/done

##GO-COUNTERPART-OF-THE-SIBLING-FRONTENDS *The Go counterpart of
`rust-ai-native-conform-frontend` (in-process syn) and
`typescript-ai-native-conform-frontend` (node sidecar): it gives `.go`
code the SAME structural discipline gate, by feeding Go facts into the
language-neutral engine — never by re-implementing the rules in a Go
linter.* @impl/done

## 1. The division of labour with the native Go tooling {#division}

##kind-line-division `req r1` @impl/done

##GO-TOOLCHAIN-CARRIES-THE-TYPE-CORRECTNESS-HALF Go's own toolchain already carries the **type / correctness** half:
`go build` (the compile gate), `go vet` (shipped correctness census),
staticcheck and the `exhaustive` linter as evidence providers (guide
§1, §5). @spec/done

##THAT-HALF-IS-WELL-TYPED-AND-LOCALLY-SANE Those answer *"is this well-typed and locally sane?"* — the
half the language does natively and well. @spec/done

##frontend-answers-the-structural-half-lead This frontend answers the **other** half — the *structural /
architectural* rules no Go tool expresses, the ones `conform check`
already enforces for Rust and TypeScript: @impl/done

- ##RULE-FILE-LENGTH-BUDGET the file-length budget (position is a resource, guide §3); @impl/done
- ##RULE-CELL-ISOLATION cell isolation (a cell imports seams + core only, never sibling
  cells; the registry is the only cell importer — guide §2/§6); @impl/done
- ##RULE-BAN-CENSUS-AS-FACTS the ban census as facts (`init()` in a cell, blank imports, ambient
  defaults, naked `go` statements, error-string matching, reasonless
  suppressions, `t.Skip` — guide §7's theater list) surfaced as conform
  findings in the Class-F `violates REQ …; fix surface: …` grammar; @impl/done
- ##RULE-SEAM-ERROR-CONTRACT the seam-error contract (`go-seam-error-cites-req` — a seam's closed
  error set carries its REQ URI, guide §5): one rule, both halves — the structure half flags a `*Error` type that owns an `Error()` but has no `Spec` field; the message half flags an `Error()` whose rendered text carries no REQ, where a REQ is counted as cited by the literal `spec://` OR the `violates REQ` marker (Go renders the URI out of the `Spec` field, while the format string itself carries the `violates REQ %s` marker); @impl/done
- ##RULE-DEVIATION-ESCAPE-HATCH the deviation escape hatch (`//spec:deviates … reason="…"`), honoured
  the way `#[spec(deviates)]` is for Rust. @impl/done

##ONE-ENGINE-ONE-GRAMMAR-ONE-BASELINE Routing these through conform keeps **one rule engine, one finding
grammar, one ratchet baseline** across all three languages, with the
rules defined once over `conform_core::Fact` and fed by any frontend —
a rule cannot drift between projections. @impl/done

## 2. What go-extract is {#extractor}

##kind-line-extractor `req r1` @impl/done

##GO-EXTRACT-IS-A-FACT-PRODUCER A fact producer: parse a `.go` file and emit the language-neutral fact
stream the rules consume. @impl/done

##GO-EXTRACT-IS-STDLIB-ONLY **Stdlib-only by construction** — `go/parser`,
`go/ast`, `go/token`, `encoding/json`, nothing else — so
`go run extract.go` works with no module context, no `go.mod`, no
network, on any machine that carries the toolchain the floor already
requires. @impl/done

##MINIMUM-EXTERNAL-TOOLING-MADE-STRUCTURAL This is the owner's "minimum external tooling" ideal made
structural: the language parses itself, one file, zero dependencies. @impl/done

- ##DELIVERY-EMBEDDED-AND-CONTENT-ADDRESSED **Delivery:** embedded in the Rust bridge crate (`include_str!`),
  materialised content-addressed to
  `<project>/target/conform/go-extract/extract-<hash16>.go` before
  spawn — the proven ts-extract mechanism. Because exactly one file is
  materialised, the source stays import-free of sibling tool files. @impl/done
- ##PROTOCOL-NDJSON-ON-STDIO **Protocol:** NDJSON on stdio — a `{proto, files: [...]}` request, one
  `{file, facts: [...], markers: [...]}` record per input, `PROTOCOL =
  1`, additive evolution, unparseable file → zero facts + a `degraded`
  note, never a crash (B5). @impl/done
- ##FACT-KINDS **Fact kinds** (the ts-extract vocabulary, Go-shaped): `item`
  (func / method / type / const / var; exported flag; receiver;
  attached `//spec:` directives), `import` (path, blank flag),
  `go_unsafe` (the ban census sites with their kind:
  `init_decl` / `blank_import` / `ambient_call` / `naked_go` /
  `error_string_match` / `t_skip` / `reasonless_suppression`),
  `file_metrics` (physical lines), and `marker` (the `//spec:`
  directive stream: tag, uri, r, reason, attached symbol, line). @impl/done
- ##ONE-EXTRACTION-TWO-CONSUMERS **One extraction, two consumers:** the conform frontend consumes
  `facts`; the specmap scanner consumes `markers`; the tcg relay
  consumes both (TCG-PROTOCOL-GO §3). One parser, one vocabulary, no
  drift. @impl/done

## 3. The frontend crate {#frontend}

##kind-line-frontend `req r1` @impl/done

##FRONTEND-IMPLEMENTS-THE-ENGINE-TRAIT `go-ai-native-conform-frontend` implements the engine's `Frontend`
trait: an `id()` of `"go-extract"`, a `version()` that bumps when the
fact schema grows (retiring cache slots wholesale, exactly as the
sibling frontends do), and `extract(file, package, module, text) ->
Vec<Fact>` that round-trips through the bridge. @impl/done

##FACTS-ARE-CONTENT-ADDRESSED Facts are keyed
`(file content-hash, frontend id+version)` in the engine's
content-addressed store — a 1-file diff re-extracts 1 file (A2). @impl/done

## 4. Topology: the `[go]` policy section {#topology}

##kind-line-topology `req r1` @impl/done

##CONFORM-TOML-GAINS-A-GO-SECTION `conform.toml` gains a `[go]` section, written by `go-ai-native init`
from the module layout: `roots` (source roots to scan), `cells_dir`
(default `internal/cells`), `seams_pkg` (default `internal/seams`),
`registry_pkg` (default `internal/registry`), `[go] gated` /
`[[go.exempt]]` `{unit, reason}` (the package is Go's gate unit — the
expand-as-you-conform ratchet: a package enters `gated` only at zero
findings, every other package carrying an exempt reason). The file-length and `invariant-comment-position` rules are fed by **root**
`conform.toml` keys (NOT under `[go]` — they are language-neutral, beside each
other): `max_file_lines` feeds `file-length`; `invariant_comment_markers` /
`invariant_comment_min_file_lines` feed `invariant-comment-position`; and
`sarif_reports` feeds the SARIF ingest whose `LintDiagnosis` facts the
`lint-suppression-needs-reason` rule cites. What those root keys mean, and what
they default to, is described once, in `ENGINE-CONFORM §6` (the policy file) —
this surface names which ones the Go rules read, not their values. @impl/done

##THE-SARIF-CITATION-PATH-IS-FED-BY-FOREIGN-LINTERS-NOT-BY-GO-EXTRACT The
`lint-suppression-needs-reason` rule is mounted in this gate too, and it is
deliberately absent from the rule list above: its facts come from SARIF
ingest — `go vet`, `staticcheck`, `golangci-lint` — not from go-extract. It is
the T-sem citation path (§5), where a Discipline rule cites a foreign linter's
diagnosis as its own evidence, and it is the one rule here the extractor does
not feed. @impl/done

##EVERY-PACKAGE-GATED-OR-EXEMPT The
every-package-gated-or-exempt invariant is enforced by the engine on
every check, exactly as for the sibling stacks. @impl/done

## 5. The honest note {#honesty}

##GO-EXTRACT-IS-A-PARSER-NOT-A-TYPE-CHECKER The structural gate is only as good as its facts, and go-extract is a
PARSER, not a type checker: it sees syntax and directives, not
resolved types. @impl/done

##TYPE-DEPENDENT-RULES-ARE-OUT-OF-SCOPE Rules that would need type information (e.g. "this
call's receiver is a seam type") are out of this tool's scope and
belong to the vet/staticcheck/exhaustive evidence tier or the oracle. @impl/done

##THE-DIVISION-IS-THE-THREE-TIER-SPLIT The division is deliberate — the same three-tier split (T-lex/T-syn/
T-sem) ENGINE-CONFORM §1 defines; go-extract is the T-syn tier, and
Go's T-sem tier is the toolchain itself. @impl/done
