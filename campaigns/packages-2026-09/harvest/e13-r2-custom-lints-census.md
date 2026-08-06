# Census — the custom REQ-citing lints layer (B-037)

Read-only census of the third structural-diagnostics channel promised by the
Discipline guides (Scaffold F) but built for no language: **custom lints whose
messages name the rule and the remedy**. Measurements only — no design
recommendations. All paths are repo-relative; `vibedeps/**` (regenerated
dependency mirrors) and the nested `packages/org.vibevm.fractality/**/vibedeps/**`
mirrors are excluded throughout — they duplicate the cited canonical source. The
engine is `packages/org.vibevm.ai-native/core-ai-native/v0.8.0` unless a version
is stated. "Not found" is stated as an explicit fact with a count.

The three structured-diagnostics channels the guides promise, and their state:
- **Channel 1 — error messages cite the REQ.** Built. The conform engine exports
  the REQ-citation half-rules: `ErrorEnumCitesReq`, `ErrorMessageCitesReq`
  (`…/core-ai-native/v0.8.0/crates/core-ai-native-conform/src/rules/mod.rs:24`),
  `TsSeamErrorCitesReq` (`…/mod.rs:29`), `GoSeamErrorCitesReq`
  (`…/mod.rs:26`).
- **Channel 2 — findings reported in SARIF.** Built. `…/core-ai-native-conform/src/sarif.rs`
  exists; SARIF is referenced from `store.rs`, `finding.rs`, `lib.rs`,
  `rules/go_parity.rs`.
- **Channel 3 — custom lints (clippy / @typescript-eslint / Go analyzer).** Not
  built for any language. This census measures the inputs needed to build it.

## Q1 — The promise, verbatim ×3, plus the R3-011 grammar sites

### Rust — vehicle named: "custom clippy lints"

`packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/spec/rust/GUIDE-AI-NATIVE-RUST.md:72`
(`##SCAFFOLD-F-STRUCTURED-DIAGNOSTICS`), quoted in full:

> **F — Structured, REQ-citing diagnostics** (`scaffold-f-structured-diagnostics`).
> `thiserror` messages carry a `spec://` REQ URI + a one-line fix-surface hint;
> conform emits SARIF; **custom clippy lints name the rule and the remedy**.
> *Rule:* every custom check emits "violates REQ-X: <why>; fix surface: <where>",
> never bare free text (R3-011). The parity this rests on … is a discipline law
> in the manifesto (`spec://org.vibevm.ai-native/core-ai-native/00-MANIFESTO#parity-across-projections`).
> @status:impl/done

**Vehicle (verbatim):** "custom clippy lints" — a custom **clippy** lint class.
The clause is marked `@impl/done`, i.e. the guide asserts it as shipped; the
census below shows the channel is in fact unbuilt (Q3).

### TypeScript — vehicle named: "Custom @typescript-eslint rules"

`packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/spec/typescript/GUIDE-AI-NATIVE-TYPESCRIPT.md:141`
(`##SCAFFOLD-F-STRUCTURED-DIAGNOSTICS`), quoted in full:

> **F — Structured, REQ-citing diagnostics** (`scaffold-f-structured-diagnostics`).
> Custom `@typescript-eslint` rules whose messages cite the violated `spec://`
> REQ and the fix surface; the Compiler API's diagnostics are already coded
> (TS2322 etc.) — wrap them with REQ context. *Rule:* every custom check emits
> "violates REQ <uri>: <why>; fix surface: <where>", never bare free text
> (R3-011). Error text is the agent's percept. @status:impl/done

**Vehicle (verbatim):** "Custom `@typescript-eslint` rules". Note the grammar
token is `<uri>` here (a full URI), whereas the Rust guide writes `REQ-X`.

### Go — NO vehicle named; "custom checks" only

`packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/spec/go/GUIDE-AI-NATIVE-GO.md:284`
(`##SCAFFOLD-F-STRUCTURED-DIAGNOSTICS`), quoted in full:

> **F — Structured, REQ-citing diagnostics** (`scaffold-f-structured-diagnostics`).
> Seam error types render `violates REQ <spec-uri>: <why>; fix surface: <where>`
> (§5); custom checks emit the same grammar; conform emits SARIF. *Rule:* every
> custom check and every seam error is agent-actionable — REQ URI + fix surface,
> never bare free text (R3-011). @status:impl/done

**Vehicle (verbatim):** "custom checks emit the same grammar" — the Go guide
names **no** vehicle. It does not say `go vet`-analyzer, `staticcheck`, a
`golang.org/x/tools/go/analysis` `Analyzer`, or any other concrete carrier. The
Go §0 thesis (`GUIDE-AI-NATIVE-GO.md:87-90`) states the Discipline "carries
proportionally more weight in **linter-borne rules**", and §1
(`GUIDE-AI-NATIVE-GO.md:133-138`) names the *stock* evidence tier (`go vet` MUST;
`staticcheck` MUST; `exhaustive`; `golangci-lint` GPL-3.0, banned) — but the
custom-lint channel's carrier is left unspecified. The Go §5 seam-error example
(`GUIDE-AI-NATIVE-GO.md:331-333`) renders only a **partial** grammar
(`fmt.Sprintf("plan: %v: violates REQ %s", e.Code, e.Spec)`) with no `fix surface`
half; §5:338 says the fix-surface is appended "at the boundary rendering".

### The R3-011 grammar — every authored site (non-vendored)

The grammar is `violates REQ <uri>: <why>; fix surface: <where>`. Authoritative
sites, by reading the content:

1. **Engine definition (the contract).**
   `packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-conform/src/rules/mod.rs:33`
   (doc grammar), `…/mod.rs:54` (the `format!` string), `…/mod.rs:66-73` (the
   `matches_req_grammar` acceptor). This is the single source of truth; see Q2.
2. **The roster entry.**
   `packages/org.vibevm.ai-native/core-ai-native/v0.8.0/spec/appendix/ATLAS.md:115-116`
   (`##FINDING-R3-011` — "Tool output is agent food: structured, requirement-citing
   diagnostics"). The superseded `core-ai-native/v0.7.0/spec/appendix/ATLAS.md`
   carries the same id; `core-ai-native/v0.7.0/…/rules/mod.rs` carries the older
   grammar string.
3. **Rust guide.** `rust-ai-native-lang/v0.7.0/spec/rust/GUIDE-AI-NATIVE-RUST.md:72`.
4. **Rust scaffold-F card.**
   `rust-ai-native-lang/v0.7.0/spec/cards/scaffold-f-structured-diagnostics.md:19`
   (full grammar) and `:45` (the card's `checker:` line names conform
   `diagnostic-cites-req`).
5. **TypeScript guide.**
   `typescript-ai-native-lang/v0.6.0/spec/typescript/GUIDE-AI-NATIVE-TYPESCRIPT.md:141`.
6. **TypeScript scaffold-F card.**
   `typescript-ai-native-lang/v0.6.0/spec/cards/scaffold-f-structured-diagnostics.md:19`
   and `:45` (the card's `checker:` names `@typescript-eslint diagnostic-cites-req`).
7. **Go guide.** `go-ai-native-lang/v0.1.0/spec/go/GUIDE-AI-NATIVE-GO.md:284`
   (full grammar) and `:331-333` (the partial `Error()` render).
8. **Go scaffold-F card.**
   `go-ai-native-lang/v0.1.0/spec/cards/scaffold-f-structured-diagnostics.md:19`,
   `:21` (structure lists the grammar), `:45` (`checker:` names conform
   `seam-error-cites-req`).

A `fix surface` count grep over the non-vendored tree returns hits in the above
files plus prose mentions in host `crates/vibe-workspace/src/bins.rs`, campaign
evidence/harvest JSON, and `terraform/adopt-v0.3/*`. `rust-ai-native-lang/v0.7.0/spec/rust/tools/rust-ai-native-tcg.md`
does **not** contain `fix surface` (a content grep returns no match). The grammar
is mirrored into every lang package's vendored engine copy
(`…/<lang>/v<ver>/crates/vendor/core-ai-native-conform/src/rules/mod.rs`) — those
are byte-identical sync mirrors of site 1, not independent definitions.

## Q2 — The message-rendering helper (the contract future custom lints must speak)

The renderer lives in the engine, exactly where the packet guessed:

`packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-conform/src/rules/mod.rs:53-55`:

```rust
pub fn req_message(uri: &str, why: &str, fix_surface: &str) -> String {
    format!("violates REQ {uri}: {why}; fix surface: {fix_surface}")
}
```

The grammar acceptor beside it, `…/mod.rs:66-74`:

```rust
pub fn matches_req_grammar(message: &str) -> bool {
    let Some(rest) = message.strip_prefix("violates REQ ") else { return false; };
    let known_scheme = ["spec://", "discipline://", "misra://"]
        .iter().any(|s| rest.starts_with(s));
    known_scheme && rest.contains(": ") && rest.contains("; fix surface: ")
}
```

**Contract (input → output):**
- **Input:** three `&str` — `uri` (must begin with `spec://`, `discipline://`, or
  `misra://` per the acceptor's closed scheme set), `why` (one-line reason),
  `fix_surface` (where/what to change).
- **Output:** `String` = `"violates REQ {uri}: {why}; fix surface: {fix_surface}"`.
  The acceptor requires, in order: the literal prefix `violates REQ `, a known
  scheme, the separator `: `, and the literal `; fix surface: `.

Two consequences a custom lint must respect: (a) the first token after `REQ` is a
**scheme-prefixed URI**, not the bare `REQ-X` shorthand the Rust guide's prose
uses — a message literal `violates REQ-X: …` would FAIL the acceptor; (b) the
closed scheme set is `spec:// | discipline:// | misra://` (`mod.rs:70`).

**Callers — how many rules.** `req_message` is called at **19 production
call-sites** across all 7 rule-family modules of the engine (test/doctest sites
excluded): `rules/budget.rs:100,167,262,378`;
`rules/diagnostics.rs:87,184,268,350`; `rules/structure.rs:56,129,217`;
`rules/go.rs:156,258`; `rules/go_parity.rs:103,237`; `rules/typescript.rs:96,220,338`;
`rules/typescript_parity.rs:84`. Those modules export **19 rule types**
(`mod.rs:23-29` re-exports: `AmbientEnv`, `FileLength`, `NoUnwrapInDomain`,
`UnsafeGate`, `ErrorEnumCitesReq`, `ErrorMessageCitesReq`, `PubDoctest`,
`SeamHasDoctest`, `GoCellIsolation`, `GoUnsafeInDomain`, `GoConformanceAssertion`,
`GoSeamErrorCitesReq`, `CellHasOracle`, `CellIsolation`, `FlagSites`,
`TsCellIsolation`, `TsFlagSites`, `TsUnsafeInDomain`, `TsSeamErrorCitesReq`).
`matches_req_grammar` is the self-check acceptor used by the parity rules
(`typescript.rs:498`, `typescript_parity.rs:158`, `go_parity.rs:281,299,382`,
`go.rs:391`) and the test harness (`rules/tests.rs:167-177,258`). The renderer is
documented as the convention every conform rule must use (`mod.rs:31-39`,
citing `spec/discipline/README.md` for the URI convention).

## Q3 — Rust today: clippy

**Panel (`tools/self-check.sh`) — host clippy, `-D warnings`:**
- `tools/self-check.sh:306-307` — step 3: `cargo clippy --workspace --all-targets --quiet -- -D warnings`.
- `tools/self-check.sh:374-375` — step 7 (core-ai-native pkg): `cargo clippy --manifest-path "$CORE_MANIFEST" --workspace --all-targets --quiet -- -D warnings`.
- `tools/self-check.sh:387-388` — step 8 (rust-ai-native-lang pkg): same shape against `$PKG_MANIFEST`.
- (Steps 8 and 10 repeat the same clippy shape for the TS and Go lang packages
  and all three mcp packages: `self-check.sh:393-394,399-400,431-432,439-440,447-448`.)

**Floor (`rust-ai-native floor`) — consumer clippy, `-D warnings`:**
- The step dictionary `STEPS`:
  `packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/crates/rust-ai-native-cli/src/floor.rs:36-44`
  — `["fmt","test","clippy","conform","specmap","test-gate","fast-loop"]`.
- The clippy step:
  `…/floor.rs:126-142` — `cargo clippy --workspace --all-targets --quiet -- -D warnings`, gated by `is_step_disabled("clippy", …)`.

**Lint configuration — none of the usual levers exist:**
- `clippy.toml` — **0 files** in the non-vendored tree (`find … -name clippy.toml`).
- `[lints]` / `[workspace.lints]` tables in any `Cargo.toml` — **0** (content grep
  over non-vendored `Cargo.toml` returns no matches).
- Attribute lints in `.rs`: scattered stock suppressions/mandates — the only
  `#![deny(...)]` is `crates/vibe-cli/src/main.rs:5` (`#![deny(unsafe_code)]`);
  `#[allow(clippy::too_many_arguments)]`, `#![allow(clippy::unwrap_used)]`,
  `#[allow(clippy::enum_variant_names)]` appear in host `crates/vibe-*` (e.g.
  `crates/vibe-registry/src/git_package_registry/mod.rs:180`, `crates/vibe-cli/src/commands/workspace/publish.rs:427`)
  and one in the vendored engine mirror `rust-ai-native-lang/v0.7.0/crates/vendor/core-ai-native-specmap/src/ratchet.rs:128`.
  All `clippy::` references are **stock** clippy names (33 files; none custom, none REQ-citing).

**Toolchain — decisive input for dylint:**
- `rust-toolchain.toml` (host root), `[toolchain] channel = "stable"`, `components = ["rustfmt","clippy"]` — `rust-toolchain.toml:2-3`.
- **Exactly one** `rust-toolchain.toml` in the non-vendored tree; **none** in the
  lang packages (`find … -name 'rust-toolchain*'` returns only the host file).
  dylint libraries must be built against the **same nightly** rustc the consumer
  uses; a `stable` pin therefore blocks dylint for a consumer unless the consumer
  adopts a pinned nightly.

**Absence of any custom-Rust-lint machinery (whole tree minus `vibedeps/`):**
- `dylint` — **0** in `.rs` and **0** in `.toml`. The 11 non-vendored mentions are
  all prose/state: `BACKLOG.md` (B-037 itself), `CONTINUE.md`, `TOOLING-MAP.md`,
  `spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md`, and campaign
  baseline/run-state/evidence JSON. No `[dependencies] dylint*` anywhere.
- `declare_lint!` / `declare_tool_lint!` — **0** in source (the only string hit is
  inside the prior census `campaigns/.../harvest/d7d-stacks-sync-reverify.md:98`,
  quoting a grep).
- `LateLintPass` / `EarlyLintPass` — **0** in source (same prior-census mention at
  `d7d-stacks-sync-reverify.md:98,1033`).
- `rustc_private` — **0** in the non-vendored tree.

## Q4 — TypeScript today: eslint

**ESLint configs (non-vendored): exactly one** —
`research/ts-demo/eslint.config.js` (the consumer demo; `find` for `.eslintrc*` /
`eslint.config.*` returns only this file). Its content is **stock recommended**:
`research/ts-demo/eslint.config.js:4-17` imports `typescript-eslint` and exports
`...tseslint.configs.recommended` (flat config; eslint 9). Its own comment
(`:1-3`) states "the conform gate owns the discipline-specific structural rules;
eslint owns the generic lint layer beneath them" — i.e. no custom rule is wired
in.

**`@typescript-eslint` presence:** present **only** in the consumer demo, as a
stock devDependency — `research/ts-demo/package.json:19`
(`"typescript-eslint": "^8.46.0"`); the lockfile resolves
`@typescript-eslint/eslint-plugin` / `parser` / `utils` / `scope-manager` /
`type-utils` / `typescript-estree` at 8.63.0 (`research/ts-demo/package-lock.json:255-417`).
`createRule` / `RuleCreator` / `ESLintUtils.RuleCreator` — **0** anywhere in the
non-vendored tree (content grep returns no code hits; the only matches are the
`typescript-eslint` string in `BACKLOG.md`, `CONTINUE.md`, `TOOLING-MAP.md`, the
terraform doc, and the demo's package files).

**The stack itself carries NO eslint dependency.** The stack's TS tooling
`packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/tools/ts-extract/package.json:9-12`
has `devDependencies` of **only** `typescript ^6.0.0` (no `eslint`, no
`typescript-eslint`). Same for `tools/ts-oracle/package.json`. So the discipline
ships no eslint config and no custom plugin; the consumer brings their own per
the guide's WIRE-3 (`GUIDE-AI-NATIVE-TYPESCRIPT.md:284` — "`npm install -D
typescript prettier eslint typescript-eslint`").

**How the TS floor lints.** The floor step dictionary
`typescript-ai-native-lang/v0.6.0/crates/typescript-ai-native-cli/src/floor.rs:26-34`:
`["prettier","tsc","tests","eslint","conform","specmap","test-gate"]`. The eslint
step (`…/floor.rs:141-160`) locates the project's own `eslint` binary
(`crate::tools::tool_command(root, "eslint")`) and runs **`eslint .`** — no
`--rule`, no `--rulesdir`, no `--plugin` flag. Any custom `@typescript-eslint`
plugin would be picked up purely through the **project's `eslint.config.js`**,
not through any CLI argument the floor passes. An absent `eslint` is a **hard
step failure** carrying the recipe `npm install -D eslint typescript-eslint`
(`floor.rs:149-153`), never a skip.

**`package.json` files (non-vendored), 7 total:**
`typescript-ai-native-lang/v0.6.0/tools/ts-extract/package.json`,
`…/tools/ts-oracle/package.json`,
`…/tools/ts-oracle/test/fixtures/proj/package.json`,
`typescript-ai-native-mcp/v0.6.0/tools/ts-extract/package.json`,
`typescript-ai-native-mcp/v0.6.0/tools/ts-oracle/package.json`,
`typescript-ai-native-mcp/v0.6.0/tools/ts-oracle/test/fixtures/proj/package.json`,
`research/ts-demo/package.json`. None of the stack's own packages declares
`eslint` or `typescript-eslint`; only the demo does.

## Q5 — Go today

**Go floor** (`go-ai-native floor`) —
`packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/crates/go-ai-native-cli/src/floor.rs`.
Step dictionary `STEPS` (`floor.rs:28-36`):
`["gofmt","vet","tests","staticcheck","conform","specmap","test-gate"]`. Steps and
exact commands:
- **gofmt** — `floor.rs:102-130`: `gofmt -l .` (any listed file = failure), with
  output filtered through `[go].exclude_substrings` (`/testdata/`, `/vendor/`,
  `/fixtures/` — `floor.rs:254-260`).
- **vet** — `floor.rs:133-141`: `go vet ./...` (unscoped `./...`).
- **tests** — `floor.rs:145-153`: `go test ./...` (unscoped `./...`; note the
  guide §1 `GUIDE-AI-NATIVE-GO.md:139-140` calls `go test -race` the MUST config
  for goroutine packages, but the floor step runs plain `go test ./...`).
- **staticcheck** — `floor.rs:157-180`: runs **both** `staticcheck ./...` and
  `exhaustive ./...` (`github.com/nishanths/exhaustive`), each a single binary on
  PATH via `crate::tools::path_tool(root, "staticcheck")` / `"exhaustive"`
  (`floor.rs:161,170`). Absent = hard failure with the install recipe
  (`go install honnef.co/go/tools/cmd/staticcheck@latest`,
  `go install github.com/nishanths/exhaustive/cmd/exhaustive@latest`,
  `floor.rs:165-166,174-175`). The two are combined into one step; both must pass.
- conform, specmap, test-gate follow (`floor.rs:182-233`).

**Custom analyzer / config absence (whole tree minus `vibedeps/`):**
- `analysis.Analyzer` / `singlechecker` / `multichecker` /
  `golang.org/x/tools/go/analysis` — **0** in any `.go`/`.rs` source. The only
  non-vendored hits are prose: `go-ai-native-lang/v0.1.0/spec/cards/scaffold-i-codemods.md:11`
  (names `golang.org/x/tools/go/analysis` as "the framework tier") and campaign
  evidence/corpus/cache/baseline JSON.
- `staticcheck.conf` — **0 files** (`find … -name staticcheck.conf`).
- `golangci-lint` — **0 usage**. 13 non-vendored mentions, all documentary: the
  guide ban `GUIDE-AI-NATIVE-GO.md:137` and `go-ai-native-lang/v0.1.0/README.md:70`
  ("GPL-3.0 — banned by the licensing flow"), the legacy plan
  `legacy-spec/terraforms/GO-AI-NATIVE-PLAN-v0.1.md:96`, and campaign JSON.

So Go today runs **stock** `go vet`, `staticcheck`, `exhaustive` as single-binary
analyzers distributed by `go install`. The custom-lint carrier for Go — a custom
`analysis.Analyzer` distributed the same way — is authored nowhere, and the guide
names no vehicle for it (Q1).

## Q6 — Where a new crate/package lands

**Crates that exist inside each lang package** (`ls …/crates/`, non-vendored
authored + a `vendor/` mirror dir):
- `rust-ai-native-lang/v0.7.0/crates/`: `rust-ai-native-cli`,
  `rust-ai-native-conform`, `rust-ai-native-conform-frontend`,
  `rust-ai-native-env-audit`, `rust-ai-native-specmap`, `rust-ai-native-tcg`,
  `rust-ai-native-tcg-bridge`, `vendor`.
- `typescript-ai-native-lang/v0.6.0/crates/`: `typescript-ai-native-cli`,
  `typescript-ai-native-conform`, `typescript-ai-native-conform-frontend`,
  `typescript-ai-native-extract-bridge`, `typescript-ai-native-specmap`,
  `typescript-ai-native-specmap-scan`, `typescript-ai-native-tcg`,
  `typescript-ai-native-tcg-bridge`, `vendor`.
- `go-ai-native-lang/v0.1.0/crates/`: `go-ai-native-cli`, `go-ai-native-conform`,
  `go-ai-native-conform-frontend`, `go-ai-native-extract-bridge`,
  `go-ai-native-specmap`, `go-ai-native-specmap-scan`, `go-ai-native-tcg`,
  `go-ai-native-tcg-bridge`, `vendor`.

**Workspace membership.**
- The host root `Cargo.toml:7` **excludes** `packages` and `vibedeps`; its members
  (`Cargo.toml:8-28`) are host crates (`crates/vibe-*`, `crates/progress-core`) +
  `xtask`. It reaches the Rust stack only as **external path-deps**
  (`Cargo.toml:97-104`): `conform-core`, `rust-ai-native-conform-frontend`,
  `rust-ai-native-conform`, `rust-ai-native-cli`, `rust-ai-native-env-audit`,
  `specmap-core`, `rust-ai-native-specmap`, `specmark`, all pointing at
  `packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/crates/…`.
- The package's OWN workspace:
  `rust-ai-native-lang/v0.7.0/Cargo.toml:9-23`, `[workspace] members` = the 7
  authored crates + the 4 vendored engine copies under `crates/vendor/`
  (`core-ai-native-conform`, `-specmap`, `-specmark`, `-specmark-grammar`).
- A new Rust crate would join this `members` list
  (`rust-ai-native-lang/v0.7.0/Cargo.toml:11-23`).

**Package manifest (how a lang package declares shippables to vibevm).**
`rust-ai-native-lang/v0.7.0/vibe.toml`: `[package]` (`name`, `group`, `kind =
"stack"`, `version` — `vibe.toml:1-9`); `[requires] packages =
{ "flow:org.vibevm.ai-native/core-ai-native" = "^0.8" }` (`vibe.toml:15-16`);
shippable binaries via `[[binary]]` tables each with `name` + `crate` (path):
`rust-ai-native`→`crates/rust-ai-native-cli`, `rust-ai-native-conform`→`crates/rust-ai-native-conform`,
`rust-ai-native-specmap`→`crates/rust-ai-native-specmap`,
`rust-ai-native-tcg`→`crates/rust-ai-native-tcg` (`vibe.toml:29-47`); plus
`[[skill]]` tables (`vibe.toml:49-57`). **There is no `[[crate]]` table** — a
library crate that is not a standalone binary is declared nowhere in `vibe.toml`;
it is only a workspace member + path-dep.

**What `cargo xtask sync-engines` does** (`xtask/src/sync_engines.rs`):
- Reads `sync-engines.toml` (`MANIFEST` const `sync_engines.rs:55`; loader
  `load_manifest` `:133-151`). Each `[[sync]]` set = `source_root` + `crates`
  (dir-name list) + `targets` (vendor-dir list) (`SyncSet` `:44-53`).
- `sync_all` (`:244-281`): for each `set × target × crate`,
  `src = source_root/<crate>`, `dst = target/<crate>`; mirror mode copies
  byte-identically, writing only differing files and removing extras
  (`mirror_crate` `:221-240`); `--check` mode (`:258-273`) byte-diffs and exits
  non-zero on drift. A denominator guard (`uncovered_vendor_dirs` `:117-131`)
  asserts every `crates/vendor` under the family is a declared target.
- **What it mirrors (the lists in `sync-engines.toml`):**
  - The **neutral engine** (`core-ai-native-conform`, `-specmap`, `-specmark`,
    `-specmark-grammar`) authored in `core-ai-native/v0.8.0/crates`
    (`sync-engines.toml:13-25`) is vendored into **all 6** targets: the 3 stacks'
    `crates/vendor` (`:21-25`) and the 3 mcp packages' `crates/vendor`
    (`:42,69,105`).
  - Each **lang-specific toolchain** set (rust `:47-57`, typescript `:73-85`, go
    `:109-121`) mirrors that language's own crates into **one** target — its mcp
    twin's `crates/`. The TS/Go `tools/` dirs mirror to their mcp twin too
    (`:90-93`, `:126-129`).

**Does a new crate ripple to six copies and the mcp twins?** Only if it is part
of the **neutral engine** (a `core-ai-native-*` crate) — then `sync-engines.toml`
copies it to all 6 (3 stacks + 3 mcp) and the `--check` gate enforces it. A
**language-specific** lint artifact (Rust dylint crate, TS eslint plugin, Go
analyzer) is inherently not neutral: it would be authored in one lang package and
mirrored to **one** mcp twin via a lang-specific `[[sync]]` set, not to the other
languages. A TS npm-package plugin is not a Cargo crate at all and is outside the
neutral-engine sync — it rides the `tools/` mirror set (`sync-engines.toml:90-93`).

## Q7 — The panel: where a new step mounts

`tools/self-check.sh` in execution order (`run_step` calls), with the
package-workspace steps flagged:
- 0b `check_floor_denominator` — asserts `GATED_SLOTS` (`self-check.sh:126`) ==
  the live newest-slot set (`:163-195`).
- 0c `check_instruction_triple` — CLAUDE.md = AGENTS.md = GEMINI.md (`:208-220`).
- 0 user-home tripwire baseline (`:279-286`).
- **1** `cargo fmt --all --check` (`:295`).
- **2** `cargo test --workspace` (`:298`).
- 2b user-home tripwire (`:303`).
- **3** `cargo clippy --workspace --all-targets -- -D warnings` (`:306-307`) — **host clippy**.
- 4 `cargo run -p vibe-cli -- check --path . --quiet` (`:316-317`).
- 5 `cargo xtask conform check` (`:325`).
- 6 `cargo xtask sync-engines --check` (`:332`) — **walks package workspaces**.
- 6b `cargo xtask check-codegen` (`:342`).
- 7 core-ai-native pkg: fmt (`:370`), test (`:372`), **clippy `-D warnings`** (`:374-375`) — **walks `core-ai-native/v0.8.0`**.
- 8 lang stacks: for rust (`:383-388`), typescript (`:389-394`), go
  (`:395-400`) — each fmt + test + **clippy `-D warnings`** — **walks the 3 lang-package workspaces**.
- 9 traceability self-traces `rust-ai-native-specmap --gate` over core, rust, go
  slots (`:415-420`) — **walks package workspaces**.
- 10 mcp packages: for rust (`:427-434`), typescript (`:435-442`), go
  (`:443-448`) — each fmt + test + **clippy `-D warnings`** + self-trace — **walks the 3 mcp-package workspaces**.
- 11b `check_lane_citations` (B-011) — grep over `spec/ packages/ crates/` (`:464-479`).
- 12 user-home tripwire, whole run (`:485`).

**`clippy -D warnings`** appears at `:306,374,387,393,399,431,439,447`.

**`floor_disable` mechanics (the three CLIs).** Each consumer floor validates a
`[[<lang>.floor_disable]]` list against its `STEPS` dictionary; an unknown step
name is a **hard failure** (never a silent skip), and a disabled step prints
`DISABLED by policy — <reason>` every run:
- Rust: `rust-ai-native-cli/src/floor.rs:36-44` (`STEPS`), `:66-76`
  (`validate_floor_disable`), `:81-83` (`is_step_disabled`); config section
  `[rust].floor_disable`.
- TypeScript: `typescript-ai-native-cli/src/floor.rs:26-34` (`STEPS`), `:53-65`
  (inline unknown-step hard-fail), `:66` (`is_disabled`); config section
  `[typescript].floor_disable`.
- Go: `go-ai-native-cli/src/floor.rs:28-36` (`STEPS`), `:76-87` (inline
  unknown-step hard-fail), `:88` (`is_disabled`); config section
  `[go].floor_disable`.

**Does any step today run a PACKAGE (consumer) linter rather than host
tooling?** No. The host panel runs only **host clippy** over the Discipline's own
Rust crates (steps 3, 7, 8, 10). It runs **no** eslint, **no** `go vet`, **no**
`staticcheck`, and **no** consumer clippy. Those package/consumer linters are
invoked solely by the consumer-facing `*-ai-native floor` commands (Q3–Q5), which
`self-check.sh` does not call. So a new custom-lint step would mount either in the
per-language `floor` `STEPS` dictionary (consumer surface) or as a new host
self-check step over the Discipline's own crates — there is no existing
package-linter step to extend.

## Q8 — Cost of each vehicle (facts only, no recommendation)

For each: what must appear in the tree, what external dependencies are required
(name + purpose), what toolchain/runtime is required, and what already exists.

### Rust — dylint-class custom clippy lints
- **Must appear in tree:** a new crate (e.g. a dylint library) under
  `rust-ai-native-lang/v0.7.0/crates/`, added to that package's `[workspace]
  members` (`rust-ai-native-lang/v0.7.0/Cargo.toml:11-23`); a `[[binary]]` entry
  in `vibe.toml` only if it ships a driver binary (`vibe.toml:29-47` shape);
  wired as a new `rust-ai-native floor` `STEPS` entry or into the `clippy` step
  (`rust-ai-native-cli/src/floor.rs:36,126`); mirrored to the rust mcp twin via a
  new lang-specific `[[sync]]` set in `sync-engines.toml` (it is not neutral, so
  not the 6-way set).
- **External dependencies required:** `cargo-dylint` / `dylint` driver + the
  lint library links against rustc internals (`rustc_lint`, `rustc_session`,
  `clippy_utils`) via `#![feature(rustc_private)]` — these are not crates.io-stable
  and are tied to a specific rustc commit.
- **Toolchain requirement:** dylint libraries must build against the **same
  nightly** rustc the consumer uses. The repo pins `channel = "stable"`
  (`rust-toolchain.toml:2`) with no nightly component and exactly one toolchain
  file (none in the lang packages) — so a consumer on the shipped stable toolchain
  cannot build/run a dylint library without adopting a pinned nightly (or the
  Discipline shipping prebuilt libs per toolchain).
- **Already exists:** stock clippy with `-D warnings` (floor `floor.rs:126-142`;
  self-check `:306`); the `req_message`/`matches_req_grammar` contract
  (`mod.rs:53,66`); SARIF emit (`core-ai-native-conform/src/sarif.rs`). **Not**
  present: any dylint dep, `declare_lint!`/`declare_tool_lint!`,
  `LateLintPass`/`EarlyLintPass`, `rustc_private` — all 0 in source (Q3). The only
  supported external path to a "custom clippy lint" is dylint
  (`declare_tool_lint!` is internal to the clippy/rustc tree).

### TypeScript — custom @typescript-eslint plugin
- **Must appear in tree:** a new npm package (e.g. an eslint plugin) under
  `typescript-ai-native-lang/v0.6.0/tools/` (sibling of `ts-extract`/`ts-oracle`),
  with a `package.json` and rules authored with `createRule` /
  `ESLintUtils.RuleCreator`; the consumer wires it into their `eslint.config.js`
  `plugins`/`rules` (the demo is `research/ts-demo/eslint.config.js`); the floor's
  `eslint .` step (`typescript-ai-native-cli/src/floor.rs:141`) needs **no CLI
  change** — it picks up whatever the project config declares. Mirrored to the TS
  mcp twin via the `tools/` sync set (`sync-engines.toml:90-93`).
- **External dependencies required:** `typescript-eslint` (the
  `@typescript-eslint/utils` package supplies `RuleCreator`/`createRule`);
  `typescript` (the project's own — already required by the floor's `tsc` step and
  WIRE-3); eslint 9 flat config.
- **Runtime/toolchain:** Node ≥ 22.6 (already the floor's node requirement,
  `research/ts-demo/package.json:7-8`); npm. No nightly/cargo constraint.
- **Already exists:** `typescript-eslint ^8.46.0` is **already** a devDep in the
  demo (`research/ts-demo/package.json:19`), resolving `@typescript-eslint/utils`
  8.63.0 et al. (`package-lock.json:255-417`); the demo `eslint.config.js`
  consumes `...tseslint.configs.recommended` (flat config). The runtime, parser,
  and rule-authoring utils are present in the demo. **Not** present: any
  `createRule`/`RuleCreator` (0); the stack's own `tools/ts-extract` carries no
  eslint dep (`tools/ts-extract/package.json:9-12`).

### Go — custom analysis.Analyzer
- **Must appear in tree:** a new Go module/binary registering
  `*analysis.Analyzer` and run via `singlechecker.Main` (or a `multichecker`),
  distributed by `go install <module>@latest` like staticcheck/exhaustive; wired as
  a new `go-ai-native floor` `STEPS` entry, or appended to the `staticcheck` step;
  invoked through `crate::tools::path_tool(root, "<name>")`
  (`go-ai-native-cli/src/floor.rs:161`); mirrored to the go mcp twin via a
  lang-specific `[[sync]]` set (not neutral, not 6-way).
- **External dependencies required:** `golang.org/x/tools/go/analysis` (BSD-3 —
  the analyzer framework; **not** a current dependency anywhere — 0 in source).
  staticcheck itself is built on this framework.
- **Runtime/toolchain:** Go ≥ 1.24 (already the floor's go requirement,
  `GUIDE-AI-NATIVE-GO.md:127`); the consumer runs `go install`.
- **Already exists:** the floor already runs `staticcheck ./...` +
  `exhaustive ./...` as single-binary analyzers via `go install`
  (`go-ai-native-cli/src/floor.rs:157-180`) — the **distribution + invocation
  pattern** for a custom analyzer binary is already exercised by the floor.
  **Not** present: any `analysis.Analyzer`/`singlechecker`/`multichecker` (0); any
  `staticcheck.conf` (0). The Go guide names no vehicle for the custom-lint
  channel (Q1); the closest existing precedent is the single-binary analyzer
  pattern the staticcheck step already uses.
