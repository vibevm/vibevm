# E8-R3-GO-FLOOR — census of the Go-floor vs. `fixtures/dirty`

Read-only census of the `go-ai-native floor` mechanics, taken to scope the
B-003 fix (the floor gating the deliberately-broken `fixtures/dirty`
extractor fixture as if it were a source). Every factual claim carries a
`path:line`; lines from the captured run are quoted verbatim. "Not found"
is recorded explicitly as a fact about the perimeter.

Roots are repo-relative. `go-pkg` abbreviates
`packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0`.

---

## Q1 — `floor.rs` mechanics

The floor is a seven-step pipeline driven by one entry point,
`run_floor(root, opts)` (`go-pkg/crates/go-ai-native-cli/src/floor.rs:55`).
The step list is the constant `STEPS` (`floor.rs:28-36`):
`gofmt, vet, tests, staticcheck, conform, specmap, test-gate`. Each step is
guarded by `is_disabled(step)` (`floor.rs:70`), which reads
`config.go.floor_disable` (`floor.rs:57`) — the **only** part of the `[go]`
policy the floor itself reads. Outcomes are recorded and the floor bails on
the first red step unless `opts.keep_going` (`floor.rs:73-79`, `109`).

How each step gathers its target set, and whether it has any exclusion:

1. **gofmt** — `floor.rs:84-112`. Builds the command via
   `crate::tools::gofmt_command(root)` (`floor.rs:86`) and adds
   `["-l", "."]` (`floor.rs:87`), then runs `cmd.output()`. `gofmt_command`
   (`go-pkg/crates/go-ai-native-cli/src/tools.rs:23-40`) returns a bare
   `gofmt` (or `gofmt.exe`) `Command` with `current_dir(root)` — no args, no
   excludes, derived as the resolved `go` binary's sibling. `gofmt -l .`
   walks the whole tree under `root` listing every unformatted `.go` file;
   any non-empty output is a failure (`floor.rs:90-95`). **No exclusion
   mechanism at all** — `gofmt` has no ignore flag, and the floor passes no
   filter. This is the total hole.

2. **vet** — `floor.rs:115-123`. `crate::tools::go_command(root)` +
   `["vet", "./..."]` (`floor.rs:117-118`); run via `run_tool_step`
   (`floor.rs:44-52`). Target set is whatever the `go` tool resolves under
   `./...`. **No floor-level exclusion.** Scoping is Go's own: `go vet
   ./...` only reaches packages of the current module. In the captured run
   the package root has no `go.mod`, so the step failed at module resolution
   before reaching any file (see Q2).

3. **tests** — `floor.rs:127-135`. `go_command` + `["test", "./..."]`
   (`floor.rs:129-130`). Same `./...` module scoping as vet; **no
   floor-level exclusion**.

4. **staticcheck + exhaustive** — `floor.rs:139-162`. `path_tool(root,
   "staticcheck")` with `"./..."` (`floor.rs:142-145`) and `path_tool(root,
   "exhaustive")` with `"./..."` (`floor.rs:151-154`). `path_tool`
   (`tools.rs:45-49`) is a PATH-resolved `Command` with `current_dir(root)`.
   Same `./...` module scoping; **no floor-level exclusion**.

5. **conform** — `floor.rs:165-177`. Calls
   `go_ai_native_conform::run_check(root, DEFAULT_GO_BASELINE, None)`
   (`floor.rs:168`). This step **does** carry an exclusion: inside `run_check`
   the engine builds `Store::for_go(root, config)` and runs `extract_go`,
   which filters the file walk by `config.go.exclude_substrings`
   (`packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-conform/src/lib.rs:73`,
   `:75-81`, `:118-126`). The exclusion is the conform engine's, not the
   floor's — and the default list does not cover `/fixtures/` (see Q3).

6. **specmap** — `floor.rs:180-191`. `go_ai_native_specmap::run_specmap_go(
   root, true)` (`floor.rs:182`). Its own scan/scoping lives in the specmap
   crate; the floor does not filter it.

7. **test-gate** — `floor.rs:195-215`. Only arms when
   `root.join(crate::DEFAULT_TESTS_BASELINE)` exists (`floor.rs:196-197`);
   otherwise prints a note. Not reached in the captured run (no baseline).

Net: of the seven steps, **only `conform` has any exclusion**, and it lives
in the conform engine (`exclude_substrings`), not in the floor. Steps 1–4
rely on each tool's own `./...`/`.` scoping; only `gofmt -l .` walks the
filesystem independent of any Go module, so it is the one that reaches a
fixture directory even on a module-less root.

---

## Q2 — The fixture tree, and which steps touch it

The Go package's fixture tree lives under
`go-pkg/tools/go-extract/test/fixtures/` and has two siblings:

- **`fixtures/clean/`** — the clean fixture: `conform.toml`,
  `internal/cells/greet/greet.go`, `spec/PROP-001.md`, `specmap.json`,
  `specmap.toml` (Glob of `tools/go-extract/test/fixtures/**/*`).
- **`fixtures/dirty/`** — the deliberately-broken fixture: `conform.toml`,
  `internal/cells/plan/plan.go`, `internal/cells/plan/plan_test.go`,
  `internal/registry/registry.go`, `spec/PROP-001.md`, `specmap.json`,
  `specmap.toml` (same Glob).

The four `.go` files under `fixtures/` are:
`fixtures/clean/internal/cells/greet/greet.go`,
`fixtures/dirty/internal/cells/plan/plan.go`,
`fixtures/dirty/internal/cells/plan/plan_test.go`,
`fixtures/dirty/internal/registry/registry.go` (Glob of
`.../fixtures/**/*.go`).

**No `testdata/` directory exists anywhere in the Go package** (Glob of
`.../testdata/**/*.go` → "No files found"). This matters: the conform
engine's default Go skip-list is `testdata`-based (Q3, Q1's `GO_SKIP_DIRS`),
so a directory the package actually ships (`fixtures`) falls outside both
the conform default and the structural skip list.

Both fixture `conform.toml` files are identical and carry no
`exclude_substrings` key — they set only `roots = []` at the top table and
`[go] roots = ["."]`, `cells_dir = "internal/cells"`
(`go-pkg/tools/go-extract/test/fixtures/dirty/conform.toml:1-7`,
`fixtures/clean/conform.toml:1-6`). With no `exclude_substrings` set, the
Go default `["/testdata/", "/vendor/"]` is in force, which does not match
the `/fixtures/` path.

**Which steps touched the fixtures in the captured run** — verbatim lines
from `campaigns/packages-2026-09/harvest/go-ai-native-lang-floor.md`:

- gofmt reached and flagged the dirty fixture (harvest line 11):
  ```
    gofmt: unformatted: tools\go-extract\test\fixtures\dirty\internal\cells\plan\plan.go
  ```
  followed by `floor: \`gofmt\` FAILED` (harvest line 12). `gofmt -l .`
  reached all four fixture `.go` files; only `plan.go` was unformatted, so
  only it was listed — `greet.go`, `registry.go`, `plan_test.go` were
  gofmt-clean.
- vet and tests failed at module resolution before reaching any source
  (harvest lines 14-21), because the package root has no `go.mod`:
  ```
  === go vet ./... ===
  pattern ./...: directory prefix . does not contain main module or its selected dependencies
  floor: `vet` FAILED
  ```
  and identically for `go test ./...`. Structurally, were a `go.mod`
  present at the root, `go vet ./...` / `go test ./...` / `staticcheck
  ./...` would compile and analyse the fixture packages too (they are valid
  Go) — they are scoped only by Go's module boundary, not by any floor
  filter.
- conform reached the dirty fixture and produced five findings
  (harvest lines 31-35):
  ```
    go-ai-native-conform: NEW go-unsafe-in-domain tools/go-extract/test/fixtures/dirty/internal/cells/plan/plan.go:17 — violates REQ discipline://go-ai-native-lang/guide#errors: a seam error type without a Spec field cannot cite its REQ; fix surface: carry the violated spec:// URI (Code + Spec + Err) and render it
    go-ai-native-conform: NEW go-unsafe-in-domain tools/go-extract/test/fixtures/dirty/internal/cells/plan/plan.go:36 — violates REQ discipline://go-ai-native-lang/guide#errors: matching on an error's string couples to prose, not contract; fix surface: consume the seam's closed error set via errors.As on its Code
    go-ai-native-conform: NEW go-unsafe-in-domain tools/go-extract/test/fixtures/dirty/internal/cells/plan/plan.go:39 — violates REQ discipline://go-ai-native-lang/guide#errors: matching on an error's string couples to prose, not contract; fix surface: consume the seam's closed error set via errors.As on its Code
    go-ai-native-conform: NEW go-unsafe-in-domain tools/go-extract/test/fixtures/dirty/internal/cells/plan/plan.go:42 — violates REQ discipline://go-ai-native-lang/guide#bans: a suppression without a reason is unrecorded testimony; fix surface: append the reason (`//lint:ignore <Check> <reason>`), or fix the finding
    go-ai-native-conform: NEW go-unsafe-in-domain tools/go-extract/test/fixtures/dirty/internal/cells/plan/plan_test.go:6 — violates REQ discipline://go-ai-native-lang/guide#replacement: `t.Skip` hides both regressions and healings; fix surface: record the failure in discipline/registry/tests-baseline.json instead
  ```
  summarised by (harvest line 36):
  ```
  go-ai-native-conform check: 5 finding(s) in scope <workspace> ({"go-unsafe-in-domain": 5}), 0 frozen in baseline, 5 new; SARIF at target\conform\report-go.sarif.
  ```
  The conform step also reported "NO conform.toml" at the package root
  (harvest line 29) — i.e. the topology default was in force, which is how
  the `/testdata/`-only skip list came to miss `/fixtures/`. The clean
  fixture was scanned too but produced no findings (it is clean).

So both the **gofmt** and **conform** steps reach the fixture tree; the
difference is that conform has an exclusion mechanism that simply does not
list `/fixtures/`, while gofmt has none at all.

---

## Q3 — `exclude_substrings`: semantics, readers, values, and whether the gofmt step honours it

**Semantics.** `exclude_substrings` is a list of substring patterns; a
source file whose repo-relative path **contains** any of them is skipped
during the conform fact-extraction walk. The match is a plain `String::contains`
on the forward-slashed repo-relative path
(`core-ai-native/v0.8.0/crates/core-ai-native-conform/src/store.rs:269` for
the Rust layout, `:360` for TypeScript, and `:442` for Go):
```rust
if exclude.iter().any(|s| file.contains(s.as_str())) {
    continue;
}
```
The Go walk additionally never descends into a fixed set of directory names,
`GO_SKIP_DIRS = ["vendor", "testdata", "node_modules", ".git", "vibedeps",
"target"]` (`store.rs:372-379`, applied at `:419-428`). Note: `fixtures` is
**not** in `GO_SKIP_DIRS` either.

**Who reads the key.** `exclude_substrings` is consumed **only** by the
conform engine's `Store`, in three language views: `Store::at_repo` copies
the top-level `config.exclude_substrings` (`store.rs:56`);
`Store::for_typescript` copies `config.typescript.exclude_substrings`
(`store.rs:68`); `Store::for_go` copies `config.go.exclude_substrings`
(`store.rs:79`). Outside the engine, two more readers exist in the Go CLI:
the `init` generator writes the starter value (`go-pkg/crates/go-ai-native-cli/src/init.rs:140`),
and the `health` command filters its record list with it
(`go-pkg/crates/go-ai-native-cli/src/health.rs:42-52`, the `.contains` at
`:48-50`). The field is declared at
`core-ai-native/v0.8.0/crates/core-ai-native-conform/src/config.rs:42`
(top-level), `:111` (`[go]`), and `:162` (`[typescript]`).

**Values in the Go package's conform.toml files.** There is **no
`conform.toml` at the Go package root** (Glob → "No files found"; harvest
line 29 reports `NO conform.toml`), so the topology default is in force. The
default for `[go].exclude_substrings` is:
```rust
exclude_substrings: vec!["/testdata/".into(), "/vendor/".into()],
```
(`config.rs:136`). The two fixture `conform.toml` files likewise set no
`exclude_substrings` (`fixtures/dirty/conform.toml:1-7`,
`fixtures/clean/conform.toml:1-6`), so the same default applies inside a
fixture-rooted run. The asymmetry with the other languages: the top-level
default is `["/generated/"]` (`config.rs:79`) and the TypeScript default is
`["/fixtures/"]` (`config.rs:191`) — **only the Go default omits
`/fixtures/`**. (The Go package vendors a copy of conform-core under
`go-pkg/crates/vendor/core-ai-native-conform/`; its `config.rs:136` and
`store.rs:79` carry the identical Go default and reader, verified by grep.)

**Does the gofmt step honour this key?** No — **measured, not assumed.**
The floor loads the config (`floor.rs:56`) but reads only
`config.go.floor_disable` (`floor.rs:57`); a grep of `floor.rs` for
`exclude_substrings` returns no hit, and the gofmt step builds a raw
`gofmt -l .` (`floor.rs:86-88`, `tools.rs:23-40`) that has no access to the
policy at all. `gofmt` itself has no ignore mechanism. So the key filters
**only** the conform step's walk (and the `health` command); it does not
and cannot steer the gofmt, vet, tests, or staticcheck steps.

---

## Q4 — The host precedent: `DEFAULT_EXCLUDES`

`crates/progress-core/src/scope.rs` defines the always-on exclusion list:
```rust
pub const DEFAULT_EXCLUDES: [&str; 8] = [
    "vibedeps",
    ".vibe",
    "refs",
    "fixtures",
    "campaigns",
    "target",
    "node_modules",
    "vendor",
];
```
(`scope.rs:13-22`). Semantics, verbatim from the doc comment
(`scope.rs:11-12`): "The always-applied exclusions — even under explicit
includes. Matched against **path components**, so each entry names a
directory." The matcher is `is_excluded`, which tests **any path component**
(`scope.rs:157-162`):
```rust
fn is_excluded(rel: &Path) -> bool {
    rel.components().any(|c| {
        let s = c.as_os_str().to_string_lossy();
        DEFAULT_EXCLUDES.iter().any(|e| s == *e)
    })
}
```

Key properties of the precedent, all stated in the file:

- **Always on, even under an explicit include.** `observed_files_reported`
  applies `is_excluded` (and the file-name half `is_excluded_file`) to every
  candidate regardless of the include globs (`scope.rs:145-147`); the
  docstring at `:128-131` fixes the order as "expand the include globs →
  drop `DEFAULT_EXCLUDES` by path component → drop `DEFAULT_EXCLUDE_FILES`
  by file name → drop the config `exclude` by glob."
- **Not overridable by an explicit include** — there is no path by which a
  `fixtures` component re-enters the corpus; the structural rule sits before
  the project's own `exclude` and is not a per-project choice
  (`scope.rs:106-118`).
- **Component match, not substring** — it matches a whole directory name
  anywhere in the path (`scope.rs:234-238` asserts `vibedeps`,
  `campaigns`, `vendor` drop under arbitrary nesting).
- A sibling file-name rule, `DEFAULT_EXCLUDE_FILES = ["LICENSE.xml"]`
  (`scope.rs:35`), is matched against the file name alone
  (`scope.rs:167-172`).

So the host's posture for `fixtures` is: a **structural, always-on,
path-component** exclusion. This is the contrast point for the Go floor,
which has no equivalent — `fixtures` appears in none of `DEFAULT_EXCLUDES`'s
siblings on the Go side (`GO_SKIP_DIRS` at `store.rs:372-379` omits it, and
the Go `exclude_substrings` default at `config.rs:136` omits it).

---

## Q5 — The Rust and TS floors: same class of hole?

**Rust floor** — `packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/crates/rust-ai-native-cli/src/floor.rs`.
Its formatting step is `cargo fmt --all --check`
(`rust-.../floor.rs:57-61`), run via `run_cargo(root, &["fmt", "--all",
"--check"])` (`floor.rs:36-42`). This is **not** a raw recursive walk of the
tree: `cargo fmt --all` formats only workspace members as resolved by cargo,
so a `fixtures/` directory that is not a workspace crate is never reached.
The Rust package also ships **no `fixtures/` directory at all** (Glob of
`rust-ai-native-lang/v0.7.0/**/fixtures/**` → "No files found"). The Rust
conform step uses the top-level `exclude_substrings` whose default is
`["/generated/"]` (`config.rs:79`), and the Rust scanner walks only `src/`
and `tests/` of each crate (`store.rs:254`), so fixtures outside those are
not reached either. **The Rust floor does not have the gofmt class of
hole** — its fmt step is cargo-scoped rather than a filesystem walk, and
the package has no fixture tree to be walked.

**TypeScript floor** — `packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/crates/typescript-ai-native-cli/src/floor.rs`.
This floor **does** have the same structural class of hole on two of its
steps. The formatting step is `prettier --check .`
(`typescript-.../floor.rs:78-97`, the args at `:82`) and the lint step is
`eslint .` (`floor.rs:141-160`, the arg at `:145`) — both walk `.` and are
**not** scoped to the policy roots, exactly like `gofmt -l .`. The TS
package ships real fixture trees that these steps would walk:
`tools/ts-extract/test/fixtures/clean/` and `.../dirty/`, and
`tools/ts-oracle/test/fixtures/proj/` (Glob of
`typescript-ai-native-lang/v0.6.0/**/fixtures/**`), and there is **no
`.prettierignore` in the TS package** (Glob of
`typescript-ai-native-lang/**/*.prettierignore` → "No files found"). So the
prettier/eslint steps are the TS-side analogue of the gofmt hole. Where the
TS floor differs from the Go floor is the other steps: the TS **tests**
step was already hardened to scope to the policy roots
(`floor.rs:121-138`, `cmd.args(crate::tools::test_globs(ts_root))` at
`:131`, with the comment at `:121-124` that "Unscoped, node would walk into
vibedeps/ and run the installed packages' own fixtures — the demo walk
caught exactly that"), and the TS **conform** step's default
`exclude_substrings` is `["/fixtures/"]` (`config.rs:191`), so the TS
conform gate skips fixtures by default — the very default the Go conform
gate lacks. Net: the TS floor has the prettier/eslint-side hole but not the
conform-side hole; the Go floor has both (gofmt has no exclusion at all,
conform's default misses `/fixtures/`).

---

## Q6 — Fix surface (facts only, no design)

**Functions that would carry an exclusion.** On the floor side, the gofmt
step is `run_floor`'s block at `floor.rs:84-112`, which assembles its
command in `gofmt_command` (`tools.rs:23-40`) and adds `["-l", "."]` at
`floor.rs:87`. The other unscoped steps (vet `floor.rs:115-123`, tests
`floor.rs:127-135`, staticcheck `floor.rs:139-162`) take `./...` and are
module-scoped by Go. The conform step already funnels through
`run_check` → `Store::for_go` (`go-.../lib.rs:73`, `:75-81`), whose
`exclude` is `config.go.exclude_substrings` (`store.rs:79`) applied in
`go_sources` (`store.rs:442`) — so any change to the conform half of the
hole is a change to the `[go].exclude_substrings` default
(`config.rs:136`) and/or the structural `GO_SKIP_DIRS` list
(`store.rs:372-379`). The floor itself never reads `exclude_substrings`
(`floor.rs:57` reads only `floor_disable`).

**Existing floor tests — none.** `floor.rs` has **no `mod tests` and no
`#[test]`** (Grep of the file for `fn .*floor|#\[test\]|mod tests` returns
only the `run_floor` definition at `floor.rs:55`). The only test files in
the CLI crate are unit tests in `init.rs` (`init.rs:237 mod tests`,
`:240 #`) and `gotest.rs` (`gotest.rs:53 mod tests`, `:72 #`), neither of
which exercises the floor.

**The package's self-test (integration test).** The one integration test is
`go-pkg/crates/go-ai-native-cli/tests/fresh_go_project.rs`, described in its
header (`fresh_go_project.rs:1-5`) as "The fresh-Go-project acceptance,
frozen (the wiring §14 walk, engine calls end to end)". The single test
`init_then_gates_catch_violations_and_the_tagged_tree_passes`
(`fresh_go_project.rs:15-16`) bootstraps a temp Go module, writes one
tagged cell + one untagged registry export, and drives `run_init`
(`:67-74`) → `run_check` expecting one finding (`:83-86`) →
`run_specmap_go` mint (`:91`) and `--check` blocking on the naked export
(`:92-94`) → re-tag → green (`:96-109`). It exercises **init, conform
(`run_check`), and specmap (`run_specmap_go`) on a synthetic hand-built
tree**. It does **not** call `run_floor`, does **not** touch the
`tools/go-extract/test/fixtures/` tree, and does **not** assert anything
about fixture exclusion. There is **no dedicated `self_check` / `selftest`
symbol** in the CLI sources (Grep of `go-ai-native-cli/src` for
`self_check|selftest|self-test|fn run_self` → no hits). The package's
effective self-check is the `floor` command itself (the run captured in the
harvest), not a covered unit/integration test.

Not found / explicit gaps in the perimeter:
- No `testdata/` directory anywhere in the Go package (stated in Q2).
- No `conform.toml` at the Go package root (Q3; harvest line 29).
- No `.prettierignore` in the TS package and no `fixtures/` in the Rust
  package (Q5).
- No unit tests for `run_floor` (this Q6).
