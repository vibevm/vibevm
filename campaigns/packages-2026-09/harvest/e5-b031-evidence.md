# E5-B031-SWEEP — evidence sweep for the B-031 design (host as a package)

Genre: campaign harvest, **evidence-only**. No verdicts, no recommendations —
counts and verbatim quotes. Every count carries the shell command that produced
it (in backticks, reproducible); every contract/code quote carries an exact
`file:line`.

## Method (applies to every count)

- Tool: `grep (GNU grep) 3.0` (verified). Occurrence counts use
  `grep -ro '<literal>' <paths> | wc -l` (each match on its own line, prefixed
  by `path:`; `wc -l` = number of occurrences).
- Exclusion flags, defined once and reused verbatim in every command:
  `E="--exclude-dir=vibedeps --exclude-dir=.vibe --exclude-dir=refs --exclude-dir=fixtures --exclude-dir=legacy-spec --exclude-dir=run --exclude-dir=target --exclude-dir=.git"`
- Perimeter (section 1): `spec/`, `crates/`, `campaigns/` (minus any `run/`),
  `packages/`, root `*.md`, `tools/`, `xtask/`. Forbidden dirs (`vibedeps/`,
  `.vibe/`, `refs/`, `fixtures/`, `legacy-spec/`) are excluded everywhere via
  `$E`; the single number in §8(b) is the one intentional exception.
- Root `AGENTS.md`, `CLAUDE.md`, `GEMINI.md` are three separate regular files
  (each 21562 bytes, byte-identical — `ls -la` shows no symlinks); all three are
  counted under "root `*.md`", so that area triple-counts the shared CLAUDE body.

---

## 1. `spec://org.vibevm.core/vibevm` quotes — full count

### (a) per-area occurrence counts + total

Command (per area): `grep -ro $E 'spec://org.vibevm.core/vibevm' <area> | wc -l`
Combined: `grep -ro $E 'spec://org.vibevm.core/vibevm' spec/ crates/ campaigns/ packages/ tools/ xtask/ *.md | wc -l`

| area | occurrences |
|---|---|
| `spec/` | 33 |
| `crates/` | 1375 |
| `campaigns/` (no `run/`) | 607 |
| `packages/` | 322 |
| root `*.md` | 12 |
| `tools/` | 0 |
| `xtask/` | 2 |
| **sum / combined** | **2351 / 2351** |

Sum of areas (2351) equals the one-command combined count (2351).

Of those, occurrences carrying a trailing `/`:
`grep -ro $E 'spec://org.vibevm.core/vibevm/' spec/ crates/ campaigns/ packages/ tools/ xtask/ *.md | wc -l` → **2330**.
Bare `spec://org.vibevm.core/vibevm` (no `/`) = 2351 − 2330 = **21** (e.g.
`crates/vibe-spec/src/address.rs:346  SpecAddress::parse("spec://org.vibevm.core/vibevm")`; the
rest are prose mentions like `…host specs use spec://org.vibevm.core/vibevm authority…` in
`campaigns/.../baseline.json`).

### (b) breakdown by 2nd address segment

Exact counts for the task-named segments, via a boundary-anchored regex:
`grep -roE $E "spec://org.vibevm.core/vibevm/<seg>(/|#|$|[\"' )>|])" spec/ crates/ campaigns/ packages/ tools/ xtask/ *.md | wc -l`

| 2nd segment | occurrences |
|---|---|
| `common` | 419 |
| `modules` | 1390 |
| `terraforms` | 12 |
| `flows` | 0 |
| `boot` | 4 |
| `design` | 0 (`spec/design/` exists on disk but is uncited) |
| **named subtotal** | **1825** |

"Other" = 2330 − 1825 = **505**. First-component tally via
`grep -rohE $E 'spec://org.vibevm.core/vibevm/[a-zA-Z0-9_-]+' spec/ crates/ campaigns/ packages/ tools/ xtask/ *.md | sed 's#spec://org.vibevm.core/vibevm/##' | sort | uniq -c | sort -rn`:

- `VIBEVM-SPEC` — **95** — a *real* host doc-id (the root `VIBEVM-SPEC.md`
  registered as `root_spec_docs` in `specmap.toml:30`); form
  `specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#linter")` (e.g.
  `crates/vibe-check/src/checks/activation_conflict.rs:6`).
- Single-letter / synthetic segments — Rust **test fixtures**, not real paths:
  `x` 180, `a` 95, `b` 43, `c` 15, `d` 9 (e.g.
  `crates/vibe-spec/src/address.rs:310  SpecAddress::parse("spec://org.vibevm.core/vibevm/x/y#a~r3")`;
  `crates/vibe-cli/src/commands/tree/artifacts.rs:237  <!-- embed: spec://org.vibevm.core/vibevm/a/b#c -->`).
- Misc small: `impl` 4, `real` 3, `missing` 3, `fake` 3, `discipline` 3,
  `research` 1, `module` 1, `STATIC` 1.
- Template/markdown artifacts (ellipses, `<doc`, trailing backticks) — the
  remainder; extraction residue, not real addresses.

### (c) Rust sources vs markdown vs other

`grep -ro $E --include='<ext>' 'spec://org.vibevm.core/vibevm' spec/ crates/ campaigns/ packages/ tools/ xtask/`

| extension | occurrences |
|---|---|
| `*.rs` (Rust) | 1669 |
| `*.md` (markdown) | 182 |
| `*.json` | 500 |

(Rust + markdown + json = 2351; no other extensions carry the literal.)

Rust form characterisation:
`grep -rn $E --include='*.rs' 'spec://org.vibevm.core/vibevm' crates/ | grep -cE 'spec\(|scope!|specmark'` → **652** lines carry
the `#[spec(...)]` / `specmark::scope!` macro form; total `*.rs` lines in
`crates/` carrying the literal: `grep -rn $E --include='*.rs' 'spec://org.vibevm.core/vibevm' crates/ | wc -l` → **1343** (so ≈1.24
occurrences/line; 1669 occurrences across 1343 lines).

The 500 `*.json` occurrences are campaign evidence/baseline data, not source:
top files `grep -ro $E --include='*.json' 'spec://org.vibevm.core/vibevm' … | awk -F: '{n=split($1,a,"/"); print a[n]}' | sort | uniq -c | sort -rn` →
`baseline.json` 223, `ev-W3a.json` 72, `batch-W3a-1.json` 29, … all under
`campaigns/.../tasks/evidence/` and `campaigns/.../baseline.json`.

---

## 2. Host exceptions in the contracts — verbatim

### PROP-029 — `spec/common/PROP-029-fully-qualified-addresses.xml`

`##SCOPE-HOST` (the host exception), `spec/common/PROP-029-fully-qualified-addresses.xml:43`:

> `- ##SCOPE-HOST The **host vibevm project's own** specs keep the project authority \`spec://org.vibevm.core/vibevm/…\` — the root project is not a package with a group; §1 binds packages. @spec/done`

Sibling scope rules in the same §4: `:44 ##SCOPE-FIXTURES` (test fixtures
`spec://demo/…`, `spec://com.example.shop/…` are out of scope) and `:45 ##SCOPE-GROUP-CHANGE`.

`grep -ni host spec/common/PROP-029-fully-qualified-addresses.xml` → the word
"host" appears on **exactly one line**, `:43`. So SCOPE-HOST is the sole host
carve-out in PROP-029.

`grep -n vibevm spec/common/PROP-029-fully-qualified-addresses.xml` → lines
`:5, :17, :21, :22, :23, :31, :43, :49, :50`. Of these, only `:43` is a
host-scope exception; the rest use `vibevm` as the project name in prose or as
package carrier *examples* (`stack:org.vibevm.ai-native/rust-ai-native-lang`
at `:21`, `spec://org.vibevm.ai-native/rust-ai-native-lang/GUIDE#anchor` at
`:22`) — i.e. the `org.vibevm.*` *group*, not the host spec namespace.

### PROP-035 — `spec/modules/vibe-workspace/PROP-035-spec-compiler.xml`

`##UNIFIED-GRAMMAR`, `spec/modules/vibe-workspace/PROP-035-spec-compiler.xml:94-103`:

> `##UNIFIED-GRAMMAR **Unified grammar** (reconciled with the pkgref grammar of PROP-008): @impl/done`
>
> (code block, `:97`) `spec://<group>/<name>[@<version>]/<doc-path>#<anchor>[.<sub>…][~r<N>]`
>
> `- ##URI-JOINER \`group\` ↔ \`name\` joiner is **\`/\`, never \`.\`** (PROP-029). @impl/done` (`:100`)
> `- ##URI-VERSION-OPTIONAL \`@<version>\` is **optional**; … taken from the lockfile / current install. …` (`:101`)
> `- ##URI-TREE-PATH \`#<anchor>.<sub>…\` is a **tree path** into the document IR (§5). @impl/done` (`:102`)
> `- ##URI-REVISION-PIN \`~r<N>\` pins a spec-unit revision (PROP-014), not a package version. @spec/done` (`:103`)

`##ROUTER-DOC-ID`, `spec/modules/vibe-workspace/PROP-035-spec-compiler.xml:107`:

> `- ##ROUTER-DOC-ID **Doc-id truncation** — \`PROP-NNN\` / \`FEAT-NNN\` in a URI resolve to \`PROP-NNN-<slug>.md\`; other docs use the full stem. (This is \`canonical_doc_path\` in the specmap engine, reused, not reinvented.) @impl/done`

`grep -ni host spec/modules/vibe-workspace/PROP-035-spec-compiler.xml` → **no
matches**. PROP-035 contains no host-special-case language at all; §6's grammar
is written uniformly as `spec://<group>/<name>/…` with no carve-out for the host.

`grep -n vibevm spec/modules/vibe-workspace/PROP-035-spec-compiler.xml` → lines
`:8, :23, :247, :265, :267, :285, :287`. None is a host-scope exception; all are
prose ("Loading vibevm itself", `:23`; "every project, package, and library
vibevm manages", `:247`; "Convert vibevm itself last", `:267`; "aligning vibevm
with the CS static/dynamic-linking standard", `:287`).

---

## 3. The host special-case in code

### (a) `crates/vibe-spec/src/resolver.rs` — where host is handled unlike packages

The branch is `spec_root`, `crates/vibe-spec/src/resolver.rs:66-77`:

```rust
66:    fn spec_root(&self, authority: &Authority) -> Result<PathBuf, ResolveError> {
67:        match authority {
68:            Authority::Host(h) if *h == self.host_namespace => Ok(self.ws_root.join("spec")),
69:            Authority::Host(h) => Err(ResolveError::UnknownHost {
70:                addr_host: h.clone(),
71:                our_host: self.host_namespace.clone(),
72:            }),
73:            Authority::Package { name, version, .. } => {
74:                Ok(self.package_slot(name, version.as_deref())?.join("spec"))
75:            }
76:        }
77:    }
```

- Host authority whose token equals `self.host_namespace` → the authored tree
  `ws_root/spec` (`:68`).
- Any other host token → `UnknownHost` (`:69-72`).
- Package authority → a materialised `vibedeps/` slot (`:73-75`, `package_slot`
  at `:82-109`).

The discriminator is the constructor-supplied `host_namespace` field
(`crates/vibe-spec/src/resolver.rs:25  host_namespace: String,`), set at
`:50-54`:

```rust
48:    /// A resolver rooted at `ws_root`, treating `host_namespace` (e.g.
49:    /// `vibevm`) as the authored host project's authority.
50:    pub fn new(ws_root: impl Into<PathBuf>, host_namespace: impl Into<String>) -> Self {
```

The *parser* decides Host-vs-Package elsewhere — `classify_authority`,
`crates/vibe-spec/src/address.rs:168-196`: a dotted first segment ⇒
`Authority::Package` (`:177-191`); an undotted first segment ⇒ `Authority::Host`
(`:192-195`, `// Undotted first segment is the host namespace.`).

A second host-special-case in display/origin rebuild —
`crates/vibe-spec/src/address.rs:132-157` (`without_pin`):
`:135  Authority::Host(h) => s.push_str(h),` vs
`:136-148  Authority::Package { … } => { push group; '/'; name; ['@'v] }`.
And `node_origin`, `crates/vibe-spec/src/pipeline.rs:209-213`:
`:211  Authority::Host(h) => h.clone(),` vs
`:212  Authority::Package { group, name, .. } => format!("{group}/{name}")`.

### (b) `enum Authority` — `Host` variant + every `Authority::Host` site

Definition, `crates/vibe-spec/src/address.rs:49-61`:

```rust
49: /// The authority half of a `spec://` address.
50: #[derive(Debug, Clone, PartialEq, Eq)]
51: pub enum Authority {
52:     /// The root project's reserved namespace (e.g. `vibevm`) — not a package,
53:     /// has no group (PROP-029 §scope).
54:     Host(String),
55:     /// A package coordinate. `version` is the raw `@`-spec, unparsed.
56:     Package {
57:         group: String,
58:         name: String,
59:         version: Option<String>,
60:     },
61: }
```

`grep -rn 'Authority::Host' crates/` → **6** sites:

| site | role |
|---|---|
| `crates/vibe-spec/src/address.rs:135` | match (Display rebuild in `without_pin`) |
| `crates/vibe-spec/src/address.rs:194` | **construct** (`classify_authority`: undotted ⇒ Host) |
| `crates/vibe-spec/src/address.rs:270` | **construct** (test: `assert_eq!(a.authority, Authority::Host("vibevm".into()))`) |
| `crates/vibe-spec/src/pipeline.rs:211` | match (`node_origin`) |
| `crates/vibe-spec/src/resolver.rs:68` | match arm (accept host) |
| `crates/vibe-spec/src/resolver.rs:69` | match arm (reject unknown host) |

### (c) `HOST_NAMESPACE` constant — definitions, value, usages

`grep -rn 'HOST_NAMESPACE' crates/` → the constant is defined **twice** (two
crates), both with value `"vibevm"`:

- `crates/vibe-cli/src/commands/tree/model.rs:23` — `pub const HOST_NAMESPACE: &str = "vibevm";`
- `crates/vibe-workspace/src/boot_artifacts.rs:67` — `const HOST_NAMESPACE: &str = "vibevm";`

Usages (imports + read sites):

- vibe-cli `model.rs` const, imported/used at:
  `crates/vibe-cli/src/commands/tree/build.rs:22` (import), `:170`
  (`host_namespace: HOST_NAMESPACE.to_string()`); `plain.rs:237`;
  `tui/copy/mod.rs:252, :314`; `tui/dispatch.rs:66, :103`; `tui/flatten.rs:276`;
  `tui/menu/mod.rs:341, :376`; `tui/modes.rs:209`; `tui/search/mod.rs:256, :293`;
  `tui/state.rs:446`.
- vibe-workspace `boot_artifacts.rs` const, used at:
  `crates/vibe-workspace/src/boot_artifacts.rs:298`;
  `crates/vibe-workspace/src/boot_artifacts/normal.rs:19` (`use super::HOST_NAMESPACE;`), `:56`;
  `crates/vibe-workspace/src/boot_artifacts/tests_qualify.rs:202`.

`FileResolver` itself takes `host_namespace` as a constructor **parameter**
(`resolver.rs:50`), not the constant; the constants wire the literal `"vibevm"`
in at every call site (`FileResolver::new(workspace_root, HOST_NAMESPACE)`).

---

## 4. Doc-name truncation — mechanics and population

### (a) how `PROP-000` resolves to `PROP-000-….md`

`resolve_doc`, `crates/vibe-spec/src/resolver.rs:112-148` — a directory
prefix-scan, not an exact match:

```rust
112: /// Resolve a doc-path … inverting the `PROP-NNN` / `FEAT-NNN` truncation by a prefix-scan.
114: fn resolve_doc(base_spec: &Path, doc_path: &str) -> Result<PathBuf, ResolveError> {
115:     let (dir, last) = match doc_path.rsplit_once('/') { … }   // split to last component
120:     if is_id_stem(last) {                                      // only PROP-NNN / FEAT-NNN
121:         let mut matches: Vec<PathBuf> = read_dir_or_empty(&dir)
122:             .map(|e| e.path())
123:             .filter(|p| id_file_matches(p, last))             // stem == id OR starts with "id-"
124:             .collect();
125:         match matches.len() {
126:             0 => Err(ResolveError::DocNotFound { … }),
130:             1 => Ok(matches.pop().unwrap()),
131:             n => Err(ResolveError::AmbiguousDoc { … }),
136:         }
137:     } else {
138:         let candidate = base_spec.join(format!("{doc_path}.md"));   // exact path otherwise
139:         if candidate.is_file() { Ok(candidate) } else { Err(…) }
147:     }
148: }
```

Helpers: `id_file_matches` (`:150-162` — `stem == id || stem.starts_with("id-")`)
and `is_id_stem` (`:164-170`):

```rust
165: fn is_id_stem(s: &str) -> bool {
166:     let Some((kind, num)) = s.split_once('-') else { return false; };
169:     (kind == "PROP" || kind == "FEAT") && !num.is_empty() && num.bytes().all(|b| b.is_ascii_digit())
170: }
```

Only `PROP-NNN` and `FEAT-NNN` truncate (test at `:190`: `assert!(!is_id_stem("DESIGN-1"))`).

### (b) population: truncated vs full, host `common/` + `modules/` citations

Actual file stems on disk, `ls -1 spec/common/*.md spec/modules/*/*.md | sed -E 's#.*/##; s#\.md$##'` → 44 stems
(`PROP-000`, `PROP-001-git-backend`, … `PROP-043-progress-markup`, plus
`OWNER-GUIDE`). Every PROP doc on disk has the full `PROP-NNN-<slug>` form
(except `PROP-000` which is bare `PROP-000.xml`).

Doc-ids cited under `spec://org.vibevm.core/vibevm/(common|modules)/…` in `spec/` + `campaigns/`
(no `run/`). Extraction:
`grep -rohE $E 'spec://org.vibevm.core/vibevm/(common|modules)/[^"'"'"' )>#|`:]+' spec/ campaigns/ | sed -E 's#.*/##; s#\.md$##; s#[^A-Za-z0-9-]+$##'`

| form | occurrences |
|---|---|
| total clean | 559 |
| **truncated** (`PROP-NNN` / `FEAT-NNN` bare) | **541** |
| **full** (`PROP-NNN-<slug>`) | **15** |
| other / non-id | 3 |

Distinct full-form doc-ids actually cited (only two):
`PROP-001-git-backend` and `PROP-009-loading-model`. Top truncated doc-ids:
`PROP-043` 162, `PROP-000` 155, `PROP-008` 57, `PROP-012` 40, `PROP-030` 24,
`PROP-019` 18, `PROP-040` 11, … — i.e. the truncated bare-id form dominates
(541 of 559 ≈ 97% of id-citations; full-slug form is ≈ 15).

### (c) does truncation apply to non-host packages today?

**Mechanics — yes, uniformly.** `resolve_doc` is authority-agnostic:
`resolve_file`, `crates/vibe-spec/src/resolver.rs:60-63`:

```rust
60:    pub fn resolve_file(&self, addr: &SpecAddress) -> Result<PathBuf, ResolveError> {
61:        let base_spec = self.spec_root(&addr.authority)?;   // host spec/ OR package vibedeps slot
62:        resolve_doc(&base_spec, &addr.doc_path)             // SAME scan for both
63:    }
```

The only host/package difference is which `base_spec` `spec_root` returns
(`:66-77`); the truncation scan in `resolve_doc` (`:114-148`) is identical for
both.

**Population — yes, truncated doc-ids are used in package citations.** Package
authorities (dotted first segment) whose doc-id is a bare `PROP-NNN`/`FEAT-NNN`.
`grep -rohE $E 'spec://[^"'"'"' )>#|`]+' spec/ campaigns/ packages/ tools/ xtask/ *.md | awk -F'/' '$3 ~ /\./ {print $NF}' | sed -E 's#\.md$##; s#[^A-Za-z0-9-]+$##' | grep -E '^(PROP|FEAT)-[0-9]+$' | sort | uniq -c | sort -rn`:

| authority | truncated doc-id | occurrences |
|---|---|---|
| `org.vibevm.ai-native` | `PROP-014` | 165 |
| `com.example.shop` | `PROP-001` | 23 |
| `com.olegchir.telegram.oproto` | `PROP-001` | 2 |
| **total** | | **190** |

All 190 are in the prose perimeter (`spec/`, `campaigns/`, `packages/`); **0** in
`crates/`. The 165 real ones cite the specmap doc, e.g.
`spec/common/PROP-031-algorithmic-refactoring.xml:45  [PROP-014 §2.7](spec://org.vibevm.ai-native/core-ai-native/mechanisms/PROP-014#llm-boundary)`.
The 25 `PROP-001` are demo/illustrative authorities (`com.example.shop`,
`com.olegchir.telegram.oproto`).

---

## 5. specmap and namespace

### (a) `specmap.toml` (root) — namespace / vibevm lines

`grep -nE 'namespace|vibevm|host' specmap.toml`:

- `:1  # specmap.toml — vibevm's traceability scan + orphan-ratchet policy.` (comment)
- `:12-14` — the host namespace declaration:
  > `:12 # The spec:// namespace this project's units are minted under — the`
  > `:13 # \`<package>\` segment of every \`spec://org.vibevm.core/vibevm/…\` URI (PROP-014 §2.1).`
  > `:14 namespace = "vibevm"`
- `:39-40, :56` — comments referencing `flow:org.vibevm.ai-native/core-ai-native`
  and `spec://org.vibevm.ai-native/core-ai-native/…`.
- `:60  namespace = "core-ai-native"` — a **second** namespace, inside the
  `[[external_specs]]` table (`:57-61`): installed package spec trees read for
  URI resolution only; `root = "vibedeps/flow-core-ai-native/0.7.0/spec"` (`:61`).

So `vibevm` as a `spec://` namespace-**value** appears at exactly one site:
`specmap.toml:14  namespace = "vibevm"`.

### (b) other carriers of `vibevm` as a namespace-value in configs/manifests

`grep -n vibevm *.toml` + `grep -rn vibevm tools/ xtask/` — candidates, marked by
context (semantics not judged):

| site | line / value | context |
|---|---|---|
| `specmap.toml:14` | `namespace = "vibevm"` | **spec:// namespace-value** (host) |
| `specmap.toml:60` | `namespace = "core-ai-native"` | namespace-value (an installed package, `[[external_specs]]`) |
| `vibe.toml:2` | `name = "vibevm"` | project **identity** name (see §7a), not spec:// namespace |
| `vibe.toml:11,12,19,28,37` | `"stack:org.vibevm.ai-native/…"`, `"flow:org.vibevm.world/…"`, `"flow:org.vibevm.fractality/…"` | dependency **pkgref keys** (the `org.vibevm.*` group) |
| `Cargo.toml:57-58` | `repository`/`homepage = "https://gitverse.ru/vibevm/vibevm"` | repo URL |
| `Cargo.toml:86,90,97-104,125` | comments + `path = "packages/org.vibevm.ai-native/…/…"]` | path-deps / group paths |
| `mirrors.toml:26,33` | `url = "git@gitverse.ru:vibevm/vibevm.git"`, `git@github.com:vibevm/vibevm.git` | repo URLs |
| `conform.toml:1,4,6,22,23,86,128` | prose "vibevm's own …"; `stack:org.vibevm.ai-native/…` | project-name prose + group ref |
| `progress.toml:35,51,89,90,125,126` | `packages/org.vibevm.*/**` globs; scope comments | package **paths** |
| `sync-engines.toml:11,14,22-127` | `packages/org.vibevm.ai-native/…` source_roots/targets | package **paths** |
| `xtask/src/batch_review/refs.rs:31` | comment: `this repository's own namespace is \`org.vibevm\`` | a namespace **claim** (group form, in prose) |
| `tools/self-check.sh:118` | `FAMILY_ROOT="packages/org.vibevm.ai-native"` | package path |
| `tools/*.sh`, `tools/*.ps1` (various) | `# vibevm …` | project-name prose |

The only sites where `vibevm` is the **spec:// namespace-value** are
`specmap.toml:14`. The `xtask/.../refs.rs:31` comment claims the repo namespace
is `org.vibevm` (group form); every other hit is a path, a repo URL, a pkgref
dependency key, or prose.

---

## 6. Tools parsing `spec://org.vibevm.core/vibevm`

Perimeter: `campaigns/packages-2026-09/tasks/*.py`, `tools/`, `xtask/src/`.

Files containing the literal `spec://org.vibevm.core/vibevm`:
`grep -rn 'spec://org.vibevm.core/vibevm' campaigns/packages-2026-09/tasks/*.py tools/ xtask/src/` → **2** hits, both
non-parsing (a comment and a help example):

- `xtask/src/batch_review/text.rs:18` — `// spec://org.vibevm.core/vibevm/modules/vibe-progress/PROP-043#stages / #states / #actions` (comment).
- `xtask/src/main.rs:243` — `///     --spec-uri "spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#solver-upgrade"` (doc-example).

Files referencing `vibevm` / address structure, with role:

- `campaigns/packages-2026-09/tasks/address-repair.py` — **structure-aware**.
  Defines the grammar at `:26  \`spec://<group>/<name>/<doc-path>#<anchor>\` — no \`.md\`, always an anchor.`
  and **assembles** addresses at `:111  address = f"@spec://{group}/{name}/{stem}#{anchor}"`.
  No hardcoded `vibevm` host authority — it emits the package `group/name` form.
  (`:54  OUT_OF_SCOPE_GROUPS = {"org.vibevm.fractality"}` is a pkgref group
  filter, not spec:// parsing.)
- `xtask/src/batch_review/refs.rs` — parses **pkgref** `kind:name` references
  (kinds `flow/feat/stack/tool/mcp`, `:26`), splitting on `/` at `:103
  raw.rsplit('/').next()`. It does **not** parse `spec://` addresses. Namespace
  claim in a comment at `:31`; test fixtures use the `org.vibevm.world` path at
  `:189, :200`.
- `xtask/src/main.rs:265` — `spec_uri: String` field accepts a `spec://` URI as
  CLI input (`--spec-uri`, `:243`) and carries it **opaque** (no authority split;
  passed through at `:363, :371`). References at `:261, :298, :301` are doc-prose.
- `campaigns/packages-2026-09/tasks/source1-join.py:19-22` — explicitly **does
  not** resolve `spec://` URIs; cites them as illustrative
  (`spec://com.example.shop/PROP-001#…`, `spec://oproto/PROP-002#…`).

Summary: the only tools that touch `spec://` **structure** (split/assemble on
`/` and `#`) are `address-repair.py` (assembles the package form) and
`batch_review/refs.rs` (splits pkgrefs, not spec://). No tool hardcodes the
`vibevm` host authority when parsing.

---

## 7. Root manifest and lock

### (a) root `vibe.toml` — identity sections

`vibe.toml` has a `[project]` table only — **no `[package]`, no `[workspace]`,
no `group` field** (`vibe.toml:1-4`):

```toml
1: [project]
2: name = "vibevm"
3: version = "0.1.0-dev"
4: authors = ["Oleg Chirukhin"]
```

The rest is `[requires.packages]` (`:6-37`, dependency pkgrefs) and two
`[[registry]]` blocks (`:54-61`). The host carries a `name` but no `group` and
no package table — matching PROP-029 `##SCOPE-HOST` ("the root project is not a
package with a group").

### (b) `vibe.lock` — does it carry the root's own identity?

`grep -nE 'root_dependencies|name = "vibevm"|kind = "host"|^host|"host"' vibe.lock`:

- The root is represented only by `[meta]` + a `root_dependencies` list
  (`vibe.lock:1-15`): `generated_by`, `generated_at`, `schema_version = 5`
  (`:1-4`), then `root_dependencies = [` (`:5-15`) — a list of pkgref strings
  the root consumes.
- There is **no `[[package]]` block for the host**:
  `grep -c '\[\[package\]\]' vibe.lock` → **36** blocks, all of them dependencies;
  `grep -nE 'name = "vibevm"|kind = "host"' vibe.lock` → **no matches**. The
  lock carries the root's *dependencies*, not the root's own identity.

Notable `root_dependencies` values (`:6-7`):
`"stack:org.vibevm/rust-ai-native@^0.7.0"` and
`"stack:org.vibevm/typescript-ai-native@^0.6.0"` — the **`org.vibevm`** group
(no `.ai-native`), appearing alongside the current
`"stack:org.vibevm.ai-native/…"` form at `:9-10`.
`grep -nE 'group = "org\.vibevm"' vibe.lock` → **no matches**: the bare
`org.vibevm` group exists only as a pkgref string in `root_dependencies`, with
no resolving `[[package]]` block.

### (c) `SPECSPACES.md` and `spec/boot/INDEX.md` — `vibevm`-namespace ties

- `SPECSPACES.md`: `grep -nE 'vibevm|namespace|spec://' SPECSPACES.md` → hits
  at `:5, :6, :22`, all package **paths/groups**: canon
  `flow:org.vibevm.world/wal-specspaces` (`:5`), authored in
  `packages/org.vibevm.world/wal-specspaces/` (`:6`), the `fractality` row
  `packages/org.vibevm.fractality/` (`:22`). No `spec://` hits — ties are to
  the `org.vibevm.*` package groups, not the host spec namespace.
- `spec/boot/INDEX.md`: `grep -nE 'vibevm|namespace|spec://' spec/boot/INDEX.md`
  → **no matches**. The generated boot manifest carries no `vibevm`-namespace
  tie.

---

## 8. Cross-boundaries: packages citing the host

### (a) canonical package sources citing `spec://org.vibevm.core/vibevm/` (host namespace)

Perimeter `packages/org.vibevm.*/**`, excluding each package's own `.vibe/` and
`vibedeps/`. `grep -ro $E 'spec://org.vibevm.core/vibevm/' packages/ | wc -l` → **314**
occurrences. (Bare `spec://org.vibevm.core/vibevm` in `packages/` = 322 − 314 = 8.)

Per-package occurrence counts
(`grep -ro $E 'spec://org.vibevm.core/vibevm/' packages/ | sed -E 's#:.*##; s#(packages/org\.vibevm\.[^/]+/[^/]+/[^/]+)/.*#\1#' | sort | uniq -c | sort -rn`):

| package | occurrences |
|---|---|
| `org.vibevm.ai-native/core-ai-native/v0.8.0` | 51 |
| `org.vibevm.ai-native/core-ai-native/v0.7.0` | 40 |
| `org.vibevm.ai-native/typescript-ai-native-mcp/v0.6.0` | 37 |
| `org.vibevm.ai-native/typescript-ai-native-lang/v0.6.0` | 37 |
| `org.vibevm.ai-native/rust-ai-native-mcp/v0.7.0` | 37 |
| `org.vibevm.ai-native/rust-ai-native-lang/v0.7.0` | 37 |
| `org.vibevm.ai-native/go-ai-native-mcp/v0.1.0` | 37 |
| `org.vibevm.ai-native/go-ai-native-lang/v0.1.0` | 37 |
| `org.vibevm.world/git-conventional-commits/v0.1.0` | 1 |
| **total** | **314** |

Example lines (`grep -rn $E 'spec://org.vibevm.core/vibevm/' packages/ | head -5`):

- `packages/org.vibevm.ai-native/core-ai-native/v0.7.0/crates/core-ai-native-specmap/src/explain.rs:299  const GRAMMAR: &str = "spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#req-conditional-grammar";`
- `packages/org.vibevm.ai-native/core-ai-native/v0.7.0/crates/core-ai-native-specmap/src/explain.rs:301  "spec://org.vibevm.core/vibevm/modules/vibe-resolver/PROP-003#req-conditional-composition";`
- `packages/org.vibevm.ai-native/core-ai-native/v0.7.0/crates/core-ai-native-specmap/src/explain.rs:421  assert!(explain_text(&map, "spec://org.vibevm.core/vibevm/x#nope").is_err());`
- `packages/org.vibevm.ai-native/core-ai-native/v0.7.0/crates/core-ai-native-specmap/src/ledger.rs:243  uri: "spec://org.vibevm.core/vibevm/common/PROP-000#root".into(),`
- `packages/org.vibevm.ai-native/core-ai-native/v0.7.0/crates/core-ai-native-specmap/src/mdspec.rs:150  /// \`spec://org.vibevm.core/vibevm/common/PROP-000#commits\`): relative to \`spec/\`, the`

### (b) installed view `vibedeps/*/spec/**` citing the host

`grep -ro 'spec://org.vibevm.core/vibevm/' vibedeps/ | wc -l` → **295** occurrences
(regenerated materialised copies; the single completeness number requested).

---

## Self-verify (section-1 counts re-run verbatim before writing)

`E="--exclude-dir=vibedeps --exclude-dir=.vibe --exclude-dir=refs --exclude-dir=fixtures --exclude-dir=legacy-spec --exclude-dir=run --exclude-dir=target --exclude-dir=.git"`

(a) per-area + combined + with-slash (re-run):
```
spec/      33
crates/    1375
campaigns/ 607
packages/  322
root *.md  12
tools/     0
xtask/     2
COMBINED   2351
with-slash 2330
```
matches §1(a) exactly.

(b) named-segment exact counts (re-run):
```
common     419
modules    1390
terraforms 12
flows      0
boot       4
design     0
```
matches §1(b) exactly.

(c) extension tally (re-run):
```
rust *.rs   1669
md   *.md   182
json        500
```
matches §1(c) exactly.
