# E8-R1-CONF-SURFACE — census of the conform engine's config surface

Read-only census of what the `conform` engine's config surface is **today**,
taken on branch `wt/E8-R1-CONF-SURFACE` (HEAD `6537cf43`). Every factual claim
carries a `path:line`, relative to the worktree root. "Not found" is recorded
explicitly as a fact about the perimeter, never silently omitted. This is an
evidence file for the B-029 + B-034 design (see `BACKLOG.md` `{#b-029}`,
`{#b-034}`), not a spec — no spec markers.

The canonical engine is `core-ai-native-conform` v0.8.0 at
`packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-conform/`.
Unless a copy is named otherwise, every `config.rs` / `lib.rs` line citation
below refers to that canonical v0.8.0 tree; vendor and `vibedeps/` copies are
byte-identical mirrors (see Q2) and are only cited separately where their
existence matters.

---

## Q1 — Full inventory of `Config`

Source: `packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-conform/src/config.rs`.

### Root table `Config` (struct `config.rs:36`; container attr `config.rs:35` = `#[serde(default, deny_unknown_fields)]`; `Default` impl `config.rs:75-92`)

| field | Rust type | line | default | read by |
|---|---|---|---|---|
| `roots` | `Vec<String>` | `config.rs:39` | `["crates/*"]` `config.rs:78` | engine + Rust frontend |
| `exclude_substrings` | `Vec<String>` | `config.rs:42` | `["/generated/"]` `config.rs:79` | engine + Rust frontend |
| `gated_crates` | `Vec<String>` | `config.rs:44` | `[]` `config.rs:80` | engine + Rust frontend only |
| `gated_pub_doctest` | `Vec<String>` | `config.rs:47` | `[]` `config.rs:81` | Rust frontend only |
| `audit_crates` | `Vec<String>` | `config.rs:50` | `[]` `config.rs:82` | Rust frontend only |
| `env_roots` | `Vec<String>` | `config.rs:53` | `[]` `config.rs:83` | Rust frontend only |
| `registry_file` | `Option<String>` | `config.rs:56` | `None` `config.rs:84` | Rust frontend only |
| `registry_gated_crate` | `Option<String>` | `config.rs:58` | `None` `config.rs:85` | Rust frontend only |
| `max_file_lines` | `u32` | `config.rs:60` | `600` `config.rs:86` | all three frontends |
| `exempt` | `Vec<ExemptEntry>` | `config.rs:64` | `[]` `config.rs:87` | engine + Rust frontend |
| `typescript` | `TsConfig` | `config.rs:68` | `TsConfig::default()` `config.rs:88` | TS frontend |
| `go` | `GoConfig` | `config.rs:72` | `GoConfig::default()` `config.rs:89` | Go frontend |

### `[go]` table `GoConfig` (struct `config.rs:106`; attr `config.rs:105` = `#[serde(default, deny_unknown_fields)]`; `Default` `config.rs:132-143`)

| field | Rust type | line | default |
|---|---|---|---|
| `roots` | `Vec<String>` | `config.rs:108` | `["."]` `config.rs:135` |
| `exclude_substrings` | `Vec<String>` | `config.rs:111` | `["/testdata/", "/vendor/"]` `config.rs:136` |
| `cells_dir` | `Option<String>` | `config.rs:118` | `None` `config.rs:137` |
| `seams_pkg` | `Option<String>` | `config.rs:121` | `None` `config.rs:138` |
| `registry_pkg` | `Option<String>` | `config.rs:125` | `None` `config.rs:139` |
| `floor_disable` | `Vec<FloorDisable>` | `config.rs:129` | `[]` `config.rs:140` |

### `[typescript]` table `TsConfig` (struct `config.rs:157`; attr `config.rs:156` = `#[serde(default, deny_unknown_fields)]`; `Default` `config.rs:187-197`)

| field | Rust type | line | default |
|---|---|---|---|
| `roots` | `Vec<String>` | `config.rs:159` | `["src"]` `config.rs:190` |
| `exclude_substrings` | `Vec<String>` | `config.rs:162` | `["/fixtures/"]` `config.rs:191` |
| `cells_dir` | `Option<String>` | `config.rs:165` | `None` `config.rs:192` |
| `seam` | `String` | `config.rs:167` | `"index"` `config.rs:193` |
| `floor_disable` | `Vec<FloorDisable>` | `config.rs:173` | `[]` `config.rs:194` |

### `FloorDisable` (struct `config.rs:179`; attr `config.rs:178` = `#[serde(deny_unknown_fields)]` — **no `default`, so both fields are required**)

| field | Rust type | line |
|---|---|---|
| `step` | `String` | `config.rs:182` |
| `reason` | `String` | `config.rs:184` |

### `ExemptEntry` (struct `config.rs:210`; attr `config.rs:209` = `#[derive(Debug, Clone, Deserialize)]` only — **no `deny_unknown_fields`, no `default`**)

| field | Rust type | line | serde |
|---|---|---|---|
| `crate_name` | `String` | `config.rs:213` | `#[serde(rename = "crate")]` `config.rs:212` |
| `reason` | `String` | `config.rs:215` | — |

### Supporting types

- `ConfigOrigin` enum (`config.rs:225-232`): `Loaded` / `Defaulted`.
- `Config::load` (`config.rs:236-240`); `Config::load_or_default` (`config.rs:247-257`).
- `validate_against_tree` (`config.rs:266-326`); `vacuously_gated` (`config.rs:349-357`).
- `lib.rs:32` re-exports `Config, ConfigOrigin, ExemptEntry, FloorDisable, GoConfig, TsConfig`.

### v0.7.0 vs v0.8.0 — one-line verdict

The v0.7.0 copy at
`packages/org.vibevm.ai-native/core-ai-native/v0.7.0/crates/core-ai-native-conform/src/config.rs`
differs from v0.8.0 in **exactly one way**: v0.7.0 has no `go` field on `Config`
(its `Config` ends at the `typescript` field, v0.7.0 `config.rs:68`) and no
`GoConfig` struct. Every root-table field, `TsConfig`, `FloorDisable`,
`ExemptEntry`, `validate_against_tree`, and `vacuously_gated` are byte-identical
between the two versions. v0.8.0 added `go: GoConfig` (`config.rs:72`) plus the
`GoConfig` struct/`Default` (`config.rs:106-143`); the corresponding scan view
`Store::for_go` lives in `store.rs:75-81`.

---

## Q2 — Vendor copies of `core-ai-native-conform`

Every directory below contains a copy of the `core-ai-native-conform` crate
(marked by `src/config.rs`). Grouped by role.

### Canonical package slots (the authoritative sources)

1. `packages/org.vibevm.ai-native/core-ai-native/v0.8.0/crates/core-ai-native-conform/` — canonical engine, **v0.8.0** (has `[go]`).
2. `packages/org.vibevm.ai-native/core-ai-native/v0.7.0/crates/core-ai-native-conform/` — canonical engine, **v0.7.0** (no `[go]`).

### In-package vendor copies (each language / MCP package vendors the engine under `crates/vendor/core-ai-native-conform/`)

3. `packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/crates/vendor/core-ai-native-conform/` (v0.8.0 engine body)
4. `packages/org.vibevm.ai-native/rust-ai-native-mcp/v0.7.0/crates/vendor/core-ai-native-conform/`
5. `packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/crates/vendor/core-ai-native-conform/`
6. `packages/org.vibevm.ai-native/go-ai-native-mcp/v0.1.0/crates/vendor/core-ai-native-conform/`
7. `packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/crates/vendor/core-ai-native-conform/`
8. `packages/org.vibevm.ai-native/typescript-ai-native-mcp/v0.6.0/crates/vendor/core-ai-native-conform/`

### Host `vibedeps/` copies (regenerated dep copies at the repo root)

9. `vibedeps/flow-core-ai-native/0.8.0/crates/core-ai-native-conform/` (v0.8.0)
10. `vibedeps/flow-core-ai-native/0.7.0/crates/core-ai-native-conform/` (reachable via `vibedeps/flow-delegation-rules/0.1.0/vibedeps/flow-core-ai-native/0.7.0/...`)
11. `vibedeps/stack-rust-ai-native-lang/0.7.0/crates/vendor/core-ai-native-conform/`
12. `vibedeps/mcp-rust-ai-native-mcp/0.7.0/crates/vendor/core-ai-native-conform/`
13. `vibedeps/stack-typescript-ai-native-lang/0.6.0/crates/vendor/core-ai-native-conform/`
14. `vibedeps/mcp-typescript-ai-native-mcp/0.6.0/crates/vendor/core-ai-native-conform/`
15. `vibedeps/flow-delegation-rules/0.1.0/vibedeps/{flow-core-ai-native/0.7.0, mcp-rust-ai-native-mcp/0.7.0, stack-rust-ai-native-lang/0.7.0}/...` (nested copies of #10/#12/#11)

### `fractality` specspace `vibedeps/` copies

16. `packages/org.vibevm.fractality/fractality/v0.1.0/vibedeps/{flow-core-ai-native/0.7.0, mcp-rust-ai-native-mcp/0.7.0, stack-rust-ai-native-lang/0.7.0}/...`
17. `packages/org.vibevm.fractality/delegation-rules/v0.1.0/vibedeps/{flow-core-ai-native/0.7.0, mcp-rust-ai-native-mcp/0.7.0, stack-rust-ai-native-lang/0.7.0}/...`

Net: **2 canonical slots** (core v0.7.0 + v0.8.0), **6 in-package vendor copies**, and the rest are host/specspace `vibedeps/` regenerated mirrors of those same two bodies. Any engine change must be replayed across the canonical slots and then re-vendored; the `vibedeps/` copies regenerate from manifests on install.

---

## Q3 — Read points per key (who reads each key)

Conventions: "engine" = the `core-ai-native-conform` crate itself; "Rust FE" =
`rust-ai-native-conform` driver; "Go FE" = `go-ai-native-conform`; "TS FE" =
`typescript-ai-native-conform`. Rule structs hold values **cloned out of the
config at `build_rules` time**; the config *read* is the `build_rules` line, and
the rule body then uses `self.<field>`.

| key | read by | path:line |
|---|---|---|
| `roots` (root) | engine | `store.rs:55` (`Store::at_repo`); `config.rs:298` (validate iterates `self.roots`); `config.rs:247-256` (`load_or_default` topology default) |
| `roots` (root) | Rust FE | `rust-ai-native-conform/src/lib.rs:120` (`Store::at_repo`) |
| `exclude_substrings` (root) | engine | `store.rs:56` |
| `gated_crates` | engine | `config.rs:269` (validate); `config.rs:352` (vacuously_gated) |
| `gated_crates` | Rust FE | `lib.rs:69,75,79,85,88` (→ rules SeamHasDoctest/ErrorEnumCitesReq/ErrorMessageCitesReq/NoUnwrapInDomain/AmbientEnv); `lib.rs:170` (`.len()`); `rust-ai-native-cli/src/health.rs:86,247,346` |
| `gated_crates` | Go FE / TS FE | **not read** — neither `build_rules` references it |
| `gated_pub_doctest` | Rust FE | `lib.rs:72` (→ PubDoctest); `health.rs:249,348` |
| `audit_crates` | engine (cloned into rules) | `rules/budget.rs:68,345` (`self.audit_crates`) |
| `audit_crates` | Rust FE | `lib.rs:66,89` (→ UnsafeGate, AmbientEnv); `health.rs:150,157` |
| `env_roots` | Rust FE | `lib.rs:90` (→ AmbientEnv); `health.rs:92` |
| `registry_file` | engine (cloned) | `rules/structure.rs:45,63` (`self.registry_file` — FlagSites rule) |
| `registry_file` | Rust FE | `lib.rs:55` (→ FlagSites) |
| `registry_gated_crate` | Rust FE | `lib.rs:57` (→ FlagSites) |
| `max_file_lines` | Rust FE | `lib.rs:82` (→ FileLength) |
| `max_file_lines` | Go FE | `go-ai-native-conform/src/lib.rs:60` (→ FileLength) |
| `max_file_lines` | TS FE | `typescript-ai-native-conform/src/lib.rs:58` (→ FileLength) |
| `exempt` | engine | `config.rs:270` (validate) |
| `exempt` | Rust FE | `lib.rs:171` (`.len()`); `health.rs:248,347` |
| `[typescript].roots` | engine | `store.rs:67` (`Store::for_typescript`) |
| `[typescript].roots` | TS FE | `typescript-ai-native-conform/src/lib.rs:71` |
| `[typescript].exclude_substrings` | engine | `store.rs:68` |
| `[typescript].cells_dir` | TS FE | `typescript-ai-native-conform/src/lib.rs:51` (→ TsCellIsolation) |
| `[typescript].seam` | TS FE | `typescript-ai-native-conform/src/lib.rs:54` (→ TsCellIsolation) |
| `[typescript].floor_disable` | TS floor runner | `typescript-ai-native-cli/src/floor.rs:52` (**not** read by the conform driver) |
| `[go].roots` | engine | `store.rs:78` (`Store::for_go`) |
| `[go].roots` | Go FE | `go-ai-native-conform/src/lib.rs:73` |
| `[go].exclude_substrings` | engine | `store.rs:79` |
| `[go].cells_dir` | Go FE | `go-ai-native-conform/src/lib.rs:54,57` (→ GoUnsafeInDomain, GoCellIsolation) |
| `[go].seams_pkg` | Go TCG oracle | `go-ai-native-tcg/src/lib.rs:299` (`config.go.seams_pkg`); test `go-ai-native-tcg/src/lib/tests.rs:20`. **Not read by the conform driver.** |
| `[go].registry_pkg` | **nobody** | defined `config.rs:125`, written by `go-ai-native-cli/src/init.rs:120-123`, but **no code reader found** (grep `\.registry_pkg\b` over `*.rs` = 0 matches). Dead key today — "carries no rule" per its own doc `config.rs:122-125`. |
| `[go].floor_disable` | Go floor runner | `go-ai-native-cli/src/floor.rs:57` (**not** read by the conform driver) |

### Summary by consumer

- **Rust FE reads:** `roots`, `exclude_substrings`, `gated_crates`, `gated_pub_doctest`, `audit_crates`, `env_roots`, `registry_file`, `registry_gated_crate`, `max_file_lines`, `exempt`.
- **Go FE reads:** `[go].roots`, `[go].exclude_substrings`, `[go].cells_dir`, `max_file_lines`. (The Go TCG oracle additionally reads `[go].seams_pkg`; the Go floor runner reads `[go].floor_disable`.)
- **TS FE reads:** `[typescript].roots`, `[typescript].exclude_substrings`, `[typescript].cells_dir`, `[typescript].seam`, `max_file_lines`. (The TS floor runner reads `[typescript].floor_disable`.)
- **Engine-only (no frontend reads directly):** none — every key the engine reads (`roots`, `exclude_substrings`, `gated_crates`, `exempt` via validate/vacuously/store) is also consumed by the Rust FE.
- **MCP packages:** each `*-ai-native-mcp` package vendors the same driver + CLI + TCG crates (e.g. `rust-ai-native-mcp/v0.7.0/crates/rust-ai-native-conform/`, `go-ai-native-mcp/v0.1.0/crates/go-ai-native-tcg/`), so config reading inside an MCP package is identical to its `*-lang` sibling. The MCP server binaries (`tools_discipline.rs`, `tools_tcg.rs`) relay to those vendored crates; no separate config-key reading was found in the server crates themselves.

---

## Q4 — `validate_against_tree`

### Definition

`Config::validate_against_tree(&self, root: &Path) -> Result<()>` at
`config.rs:266-326`.

### Inputs

- `&self` — the loaded `Config`.
- `root: &Path` — the project root the policy is checked against.

### Unit of classification

A **crate directory**: the basename of any subdirectory that (a) sits under a
`<dir>/*` root and contains a `Cargo.toml` (`config.rs:300-305`, the `on_disk`
set), or (b) is a literal root resolved through `store::crate_dir_name`
(`config.rs:307-309`, the `literals` set; `crate_dir_name` at `store.rs:212-217`).
So the unit is Rust-crate-shaped (a `Cargo.toml`-bearing dir); the function is
crate-oriented by construction (see B-034 consequence below).

### Algorithm (in order)

1. Build `gated: BTreeSet<&str>` from `self.gated_crates` (`config.rs:269`) and `exempt: BTreeSet<&str>` from `self.exempt[].crate_name` (`config.rs:270`).
2. Duplicate in `gated_crates` (`config.rs:271-273`) → `bail!("conform.toml: \`gated_crates\` carries a duplicate crate name")`.
3. Duplicate in `[[exempt]]` (`config.rs:274-276`) → `bail!("conform.toml: \`[[exempt]]\` carries a duplicate crate name")`.
4. `gated ∩ exempt` non-empty (`config.rs:277-280`) → `bail!("conform.toml: crates both gated and exempt: {both:?}")`.
5. Any `exempt` entry whose `reason.trim()` is empty (`config.rs:281-289`) → `bail!("conform.toml: \`{name}\` is exempt without a recorded reason — …")`.
6. Expand roots: `<dir>/*` → `read_dir`, dirs containing `Cargo.toml` → `on_disk` (`config.rs:298-306`); literal root → `crate_dir_name(root.join(entry))` → `literals` (`config.rs:307-309`).
7. For each `on_disk` crate in neither `gated` nor `exempt` (`config.rs:311-317`) → `bail!("conform.toml: crate \`{c}\` is neither gated nor exempt — classify it")`.
8. For each `gated ∪ exempt` name in neither `on_disk` nor `literals` (`config.rs:318-324`) → `bail!("conform.toml: \`{c}\` is listed but no crate directory matches it — typo?")`.
9. `Ok(())` (`config.rs:325`).

### Output

`Result<()>`. On violation: `anyhow::bail!` with one of the **six** distinct
error strings above (steps 2–5, 7, 8). No findings/SARIF — this is a
policy-shape check, not a source-scan rule; it runs before extraction.

### Call-sites (production; vendor/`vibedeps` mirrors omitted)

- `rust-ai-native-conform/src/lib.rs:119` — `run_check`, canonical path
  `packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/crates/rust-ai-native-conform/src/lib.rs:119`.
- `rust-ai-native-conform/src/lib.rs:188` — `run_freeze`, same file.
- `rust-ai-native-cli/src/init.rs:335` and `:375` — the `rust-ai-native init`
  command's own tests (canonical
  `packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/crates/rust-ai-native-cli/src/init.rs:335,375`).
- `xtask/src/conform.rs:33` — the host invariant test `every_crate_is_gated_or_exempt`.
- Engine self-tests: `core-ai-native-conform/src/config.rs:397,406,415,423,435,454,460,466` (v0.8.0).

The `rust-ai-native-mcp` package additionally vendors `rust-ai-native-conform`
(`lib.rs:119,188`) and `rust-ai-native-cli` (`init.rs:335,375`) — the same
call-sites inside those vendored copies.

### B-034 finding (central)

The Go and TS frontend drivers do **not** call `validate_against_tree`.
Confirmed by reading both files in full:
`go-ai-native-conform/src/lib.rs` (`run_check` `lib.rs:86-132`, `run_freeze`
`lib.rs:137-160`) and `typescript-ai-native-conform/src/lib.rs` (`run_check`
`lib.rs:84-135`, `run_freeze` `lib.rs:140-163`) only load config, build rules,
extract, `check`, and baseline-diff — there is no `validate_against_tree` call.
So the gated-or-exempt tree invariant is enforced **only on the Rust path**
(Rust driver check/freeze + Rust `init` + host xtask). Extending it to Go/TS
(B-034) means adding the call to the Go/TS drivers, and the function itself is
crate-shaped (`Cargo.toml` dirs, `config.rs:302`) so a Go/TS-applicable form
needs a non-crate unit (package dir / TS root dir — see the `go_sources` /
`typescript_sources` "crate = root dir name" derivation at `store.rs:415` and
`store.rs:332`).

---

## Q5 — `conform.toml` instances

16 files in the tree. Sections/keys each **actually uses** (the migration
surface if a key is renamed):

| file | sections / keys used |
|---|---|
| `conform.toml` (host root) | `roots` `:13`, `exclude_substrings` `:14`, `registry_file` `:17`, `registry_gated_crate` `:18`, `audit_crates` `:33`, `max_file_lines` `:36`, `gated_crates` `:41` (13 names), `gated_pub_doctest` `:61` (2), `env_roots` `:71` (many), `[[exempt]]` ×6 (`:106,:110,:114,:118,:122,:126`, each `crate`+`reason`). **No `[go]`/`[typescript]`.** |
| `packages/org.vibevm.fractality/fractality/v0.1.0/conform.toml` | `roots` `:8`, `exclude_substrings` `:9`, `max_file_lines` `:10`, `gated_crates` `:15` (7), `gated_pub_doctest` `:26` (`[]`), `audit_crates` `:27` (`[]`), `env_roots` `:35` (7). No exempt/go/ts. |
| `research/rust-demo/conform.toml` | `roots` `:8`, `exclude_substrings` `:9`, `max_file_lines` `:10`, `gated_crates` `:16` (`["rust-demo"]`), `gated_pub_doctest` `:17`, `audit_crates` `:18` (`[]`), `env_roots` `:19` (`[]`). |
| `research/go-demo/conform.toml` | `roots` `:9` (`[]`), `max_file_lines` `:10`, `[go]` `:12` {`roots` `:13`, `exclude_substrings` `:14`, `cells_dir` `:15`, `seams_pkg` `:16`, `registry_pkg` `:17`}. **No `gated_crates`/`[[exempt]]`.** |
| `research/ts-demo/conform.toml` | `roots` `:9` (`[]`), `max_file_lines` `:10`, `[typescript]` `:12` {`roots` `:13`, `exclude_substrings` `:14`, `cells_dir` `:15`, `seam` `:16`}. **No `gated_crates`/`[[exempt]]`.** |
| `packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/tools/go-extract/test/fixtures/{clean,dirty}/conform.toml` | `roots` (`[]`), `[go]` {`roots=["."]`, `cells_dir="internal/cells"`}. (clean `:4-6`, dirty `:5-7`) |
| `packages/org.vibevm.ai-native/go-ai-native-mcp/v0.1.0/tools/go-extract/test/fixtures/{clean,dirty}/conform.toml` | identical to the `lang` fixtures above |
| `packages/org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0/tools/ts-extract/test/fixtures/{clean,dirty}/conform.toml` | `roots` (`[]`), `[typescript]` {`roots=["src"]`, `exclude_substrings=[]`, `cells_dir="src/cells"`, `seam="index"`} |
| `packages/org.vibevm.ai-native/typescript-ai-native-mcp/v0.6.0/tools/ts-extract/test/fixtures/{clean,dirty}/conform.toml` | identical to the `lang` fixtures above |
| `vibedeps/mcp-typescript-ai-native-mcp/0.6.0/...` and `vibedeps/stack-typescript-ai-native-lang/0.6.0/...` ts fixtures | regenerated mirrors of the two ts fixtures above |

### Migration surface on renaming `gated_crates`

Only **three** files actually carry the key today: host `conform.toml:41`,
`packages/org.vibevm.fractality/fractality/v0.1.0/conform.toml:15`,
`research/rust-demo/conform.toml:16`. The Go/TS demos and all eight fixtures
carry **no** `gated_crates`. The Rust `init` template emits `gated_crates = []`
(`rust-ai-native-cli/src/init.rs:179`) plus the `[[exempt]]` block
(`init.rs:186`); the Go `init` template (`go-ai-native-cli/src/init.rs:126-144`)
and TS `init` template (`typescript-ai-native-cli/src/init.rs:114-131`) emit
**neither** `gated_crates` nor `[[exempt]]`.

Because every config container is `deny_unknown_fields`
(`config.rs:35,105,156,178`), a file using an old key spelling **hard-fails to
parse** (it is not silently ignored). A rename is therefore a breaking config
change: the three live files + the Rust `init` template must move in lockstep,
or every Rust consumer's `conform check` parse-errors at `config.rs:239`.

`floor_disable` is carried by **no** `conform.toml` in the tree today (only the
struct `config.rs:129,173`, the floor runners, and the Go `README.md:62`
mention it).

---

## Q6 — Tests / fixtures / docs textually tied to the key spellings

### Code (`.rs`) tied to the spelling `gated_crates`

- The field definition + Default + validate + vacuously: `config.rs:44,80,269,272,352`.
- Rust FE: `rust-ai-native-conform/src/lib.rs:69,75,79,85,88,170`; `rust-ai-native-cli/src/health.rs:86,247,346`; `rust-ai-native-cli/src/init.rs:179,186` (template writes `gated_crates = []` and the exempt block); `xtask/src/conform.rs:33`; engine self-tests `config.rs:397,406,415,423`.
- Go FE / TS FE: **no** `gated_crates` references (their `build_rules` never name it).

### Docs / skills / guides tied to `gated_crates` (what reads wrong on rename)

- `packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/spec/skills/rust-ai-native-terraform/SKILL.md:82` — "add the crate to conform.toml's `gated_crates`" (mirrors at the `vibedeps/.../stack-rust-ai-native-lang/0.7.0/spec/skills/rust-ai-native-terraform/SKILL.md:69`, and the fractality specspace copies).
- `packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/spec/skills/rust-ai-native-sweep/SKILL.md:79` — "flip a crate into `gated_crates`" (mirrors at `:59`).
- `packages/org.vibevm.ai-native/rust-ai-native-lang/v0.7.0/spec/rust/GUIDE-AI-NATIVE-RUST.xml:169` — gated_crates / `[[exempt]]` / "every-crate-gated-or-exempt invariant enforced on every check"; `:181` — gated_crates / `gated_pub_doctest` flip-only-after-drain (mirrors at `:107,:119`).
- `packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/spec/go/tools/conform-frontend-go.xml:110` — names `registry_pkg`, `gated_crates`.
- `packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/spec/go/GUIDE-AI-NATIVE-GO.xml:626` — "a package enters `gated_crates` only at zero findings (the key is the shared neutral engine's — one spelling across the language stacks today)".
- `packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/spec/skills/go-ai-native-sweep/SKILL.md:89` — "flip a package into `gated_crates`".
- `packages/org.vibevm.ai-native/go-ai-native-lang/v0.1.0/README.md:62` — `[go].floor_disable` spelling.
- `BACKLOG.md:769` (B029-LOCATOR), `BACKLOG.md:838` (B034-LOCATOR) — the items this census feeds.
- Historical / evidence references (campaigns + terraforms): `campaigns/packages-2026-09/harvest/d1-go-ai-native-lang-repairs.md:461-512,945`; `…/d7d-stacks-sync-reverify.md:1636-1682`; `…/d8a-stacks-package-own-release-reverify.md:1089-1345`; `…/d9-release-corrections-prepared.md:1073-1162`; `spec/terraforms/PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.xml:2706,2777,4275`; `legacy-spec/terraforms/{TRACEABILITY-RELOCATION-PLAN-v0.1.md:248, SETTINGS-SYSTEM-IMPL-PLAN-v0.1.md:44,87, SELF-SUFFICIENCY-PLAN-v0.1.md:319,408,536,583, DISCIPLINE-SWEEP-v0.2.md:42}`.

### What breaks on a `gated_crates` rename

1. **Parse failures** (hard, `deny_unknown_fields`): host `conform.toml:41`,
   fractality `conform.toml:15`, rust-demo `conform.toml:16` — all three
   Rust-side files stop parsing at `config.rs:239`.
2. **Template drift**: `rust-ai-native-cli/src/init.rs:179` would emit the old
   name, so freshly-init'd Rust projects would parse-fail.
3. **Operator-facing docs read wrong**: the rust `terraform`/`sweep` skills
   instruct operators to edit `gated_crates`; the Rust GUIDE documents the
   invariant under that name; the Go GUIDE already pins it as "one spelling
   across stacks" (`GUIDE-AI-NATIVE-GO.xml:626`).
4. **No code breakage** beyond the mechanical struct-field rename in
   `config.rs` + the `build_rules`/`health`/`init` references — the field has no
   serde alias today (Q7), so there is no graceful transition path in place.

The Go side already carries a recorded terminology tension: the config key is
the Rust-flavoured `gated_crates` even in the Go stack, and the Go error string
says "duplicate **crate** name" (`config.rs:272`) — see
`campaigns/.../d1-go-ai-native-lang-repairs.md:494,945` and
`GUIDE-AI-NATIVE-GO.xml:626`. B-029 is precisely the reconciliation of this.

---

## Q7 — Precedent for serde aliases

**There is no `#[serde(alias = …)]` anywhere in this tree.** Two independent
searches confirm it:

- `grep "serde\(.*alias"` over `*.rs` → **0 matches**.
- `grep "\balias\b"` over `*.rs` → every match is unrelated (the `@!ARULE`
  *address alias* in `crates/vibe-workspace/tests/dynamic_lane.rs`, and the git
  remote named `"alias"` in `xtask/src/mirror.rs:499,503`). None is a serde
  attribute.

Enumerating **all** `#[serde(` attribute usage in the tree, the only serde
features in use are: `default`, `deny_unknown_fields`, `rename`, and
`skip_serializing_if`. The closest precedent to an alias is `#[serde(rename =
"crate")]` on `ExemptEntry.crate_name` (`config.rs:212`), and the heavy
`#[serde(rename = …)]` use in the specmap generated model
(`vibedeps/flow-core-ai-native/0.8.0/crates/core-ai-native-specmap/src/generated/specmap/mod.rs:12-197`).

### Observed coexistence facts (no theory)

- `default` + `deny_unknown_fields` combined on one line: `Config`
  (`config.rs:35`), `GoConfig` (`config.rs:105`), `TsConfig` (`config.rs:156`).
- `deny_unknown_fields` alone: `FloorDisable` (`config.rs:178`).
- `rename` with **neither** `default` **nor** `deny_unknown_fields`:
  `ExemptEntry` (`config.rs:209-212`).
- **No** example in this tree of `alias` + `deny_unknown_fields` coexisting.
  Whether `deny_unknown_fields` would reject an aliased name is therefore
  **not exercised by any code here** — recorded as a fact, not a claim about
  serde's behaviour.

---

## Q8 — Behaviour on an empty scope

### Rust (empty `gated_crates` and/or empty `roots`)

- Defaults: `gated_crates = []` (`config.rs:80`), `roots = ["crates/*"]`
  (`config.rs:78`). A missing `conform.toml` → `load_or_default`
  (`config.rs:247`) returns `Config::default()` with topology-detected roots
  and `gated_crates = []`, tagged `ConfigOrigin::Defaulted`.
- `validate_against_tree` with empty `gated` + empty `exempt`: the on-disk loop
  (`config.rs:311-317`) **fails** for every crate dir the roots resolve
  ("neither gated nor exempt"). So a present Rust policy with real crate dirs
  and an empty gate list **fails the invariant** (correct). But if the roots
  resolve to **zero** crate dirs (e.g. `roots = []`, or pointing nowhere),
  `on_disk` and `literals` are both empty, both loops are vacuous, and the
  function returns `Ok(())` — a **silent pass** (`config.rs:311-325`).
- `run_check` calls `validate_against_tree` (`lib.rs:119`) then
  `warn_vacuously_gated` (`lib.rs:131` → `config.rs:349-357`), which warns for
  each gated crate with zero scanned sources. With `gated_crates = []` there is
  no vacuous warning. The driver always prints the `ConfigOrigin` line; a
  defaulted run announces "NO conform.toml — topology default in force, nothing
  is gated" (`lib.rs:33-36`).
- **False-green vector (Rust):** a *present* `conform.toml` whose `roots`
  resolve to no real crate dirs and with `gated_crates = []` → validate passes,
  0 facts (`workspace_sources`, `store.rs:225`), 0 findings, clean baseline
  diff → **green**, with only the "policy loaded" line as a signal. The host
  guard `every_crate_is_gated_or_exempt` (`xtask/src/conform.rs:29-35`) catches
  unclassified on-disk crates, but only for crates the roots actually reach.

### Go (empty `[go].roots`)

- Default `[go].roots = ["."]` (`config.rs:135`). `go_sources`
  (`store.rs:388-449`) with `roots = []` → `root_dirs` empty → **0 facts**.
- The Go driver (`go-ai-native-conform/src/lib.rs:86-132`) does **not** call
  `validate_against_tree` and does **not** call any vacuous-warning helper. It
  prints "extracted N file(s)" (`lib.rs:76`) — `N = 0` is visible in that line
  but **not flagged** as a problem. 0 facts → 0 findings → clean baseline →
  **green**.
- **False-green vector (Go):** an empty or mis-scoped `[go].roots` silently
  scans nothing and passes, with no warning. There is no `gated_crates`
  concept for Go, so the Rust vacuously-gated guard does not apply either.

### TypeScript (empty `[typescript].roots`)

- Symmetric to Go. Default `[typescript].roots = ["src"]` (`config.rs:190`).
  `typescript_sources` (`store.rs:305-367`) with `roots = []` → 0 facts. The TS
  driver (`typescript-ai-native-conform/src/lib.rs:84-135`) does **not** call
  `validate_against_tree` and has no vacuous warning; it prints "extracted N
  file(s)" (`lib.rs:74`). A defaulted (no-file) run is announced
  (`typescript-ai-native-conform/src/lib.rs:28-32`).
- **False-green vector (TS):** an empty or mis-scoped `[typescript].roots` →
  silent green, same as Go.

### Asymmetry (the B-029 / B-034 surface)

Rust validates the policy shape *and* warns on vacuous gates; Go and TS do
neither — an empty language-root scope passes silently. Closing that gap (so a
Go/TS run cannot masquerade as green over an empty scan) is exactly the
"enrichment of the surface" (B-029) and the "gated-or-exempt invariant for
Go/TS" (B-034) the design is preparing for.
