# App-versions campaign log

Append-only. Newest entry last. One entry per phase step, spike or decision
that a later session would otherwise have to re-derive.

## 2026-11-10 — campaign planned

`PLAN.md` and `FORMAT-CHANGES.md` written against `main` at `7d495d137`;
`spec://org.vibevm.core/vibevm/common/PROP-075#root` rewritten from the
owner's decided shape into a settled design with the open section replaced by
decisions. Nothing implemented.

Facts established while planning, which the plan's §2 records in full and
which cost a survey to find:

- `install -g` does not go through `vibe-resolver`; it reaches the same
  `MultiRegistryResolver` through `vibe-package-source`, so one template layer
  serves both paths (`crates/vibe-app/src/resolve.rs:181-232`).
- `crates/vibe-requirements/` is the PROP-054 spec-facts query library, **not**
  the dependency-requirement grammar; that is
  `PackageRef::parse` / `VersionSpec` in `crates/vibe-core/src/package_ref.rs`.
- `vibe check` does **not** validate spec-XML dialect well-formedness: a
  deliberately malformed probe spec under `vibevm/vibespecs/common/` left
  `vibe check` at `0 errors, 42 warnings`. Whoever edits a spec XML must
  validate it another way (the projector in `crates/vibe-specdoc`), and the
  campaign should consider adding that check — filed as a candidate, not done.
- The per-version index record is a **shared vocabulary** (`version_entry`), so
  one optional member reaches `primary.jsonl`, `by-name` and the journal at
  once (spike S3 confirms the golden bytes do not move).
- `formats/EPOCHS.toml` is `public = false`, `break_window_open = true`:
  breaking is free and no `formats/breaks/` note is required.
- nvm-sh is pinned at v0.41.0 (`4e8758a801eea9479b32a904f31093e410ddaba4`),
  whose surface is wider than the 0.40.x line (`--only=<categories>`,
  `install --offline`, `--save`/`-w`, install locks, `$NVM_AUTH_HEADER`); its
  own `test/fast/` suite is *near*-offline and has no suite-wide mirror
  override, so reuse is bounded by a recorded allow-list.
- nodejs.org publishes digests but no sizes, so the Node template omits `size`
  — the one deliberate loss against the retired generator
  (`AV-FMT-006`).

Open to the owner: `PROP-075#OWNER-QUESTIONS` Q1–Q5. Only Q1 (row-by-row
approval of the proposed nvm exclusions) blocks a phase, and only P7.

## 2026-11-10 — phase 0 spikes S1, S2, S3, S5

Run on `main` at `7d495d137` with the repository's isolated runner
(`python3 /fast/vibevm-test-tmp/claude-int/run_test.py …`). Every spike's
instrumentation and schema probe was reverted afterwards; the working tree
holds only P1's product work.

**S1 — the renderer on the real manifest. All four questions: yes.**
The template was authored by hand from the committed `v24.22.2` manifest and
lives at `crates/vibe-template/tests/fixtures/node/vibe.toml` (that
directory's `README.md` records how every fixture was derived).

- `{platform}` / `{arch}` bind per `[[installer_options]]` row by descent
  alone: the renderer reads the row's own `platform` / `arch` string values
  before walking its subtree, and refuses the two variables anywhere else by
  naming the TOML path. The fallback of dropping them from the grammar was
  not needed.
- The lone-variable numeric coercion works: the renderer replaces the whole
  `toml_edit` node with a typed value, so `size = "{size:…}"` renders
  `size = 1234`, not `size = "1234"`. **But** a template that uses it does not
  deserialise into the typed `Manifest` (a string where a `u64` is expected),
  so the renderer reads the document as TOML text and the `[template]` table
  on its own. Recorded as `PROP-075#TEMPLATE-DOCUMENT-READING`, with the one
  visible consequence: `vibe check`'s manifest-validity cell reports such a
  template as unparseable, and admitting a lone variable in a typed numeric
  field is a separate, later change to the typed layer that neither real
  package needs.
- `Manifest::parse_str(rendered)` accepts the output for both versions, and
  the render strips `[package].template`, the `[template]` table and every
  `template_optional` key, so the rendered document carries no template
  vocabulary at all.
- **Byte-identity holds, modulo the generator's four-line provenance header
  *and* the seven `size` lines** — the loss `#TEMPLATE-DIGEST-SOURCE` already
  records. Nothing else differs, for `24.22.2` or `22.23.4`. The test strips
  exactly those and asserts the count of stripped `size` lines, so a second
  silent divergence cannot hide in the normalisation. PLAN.md §5's prediction
  P1 and §3's S1 question were amended to say so.

Two further decisions the spike forced, both in
`#TEMPLATE-DOCUMENT-READING`: the render rewrites `[package].version` to the
produced version (the template's own `version` is the template revision, so no
variable may spell it), and a skeleton render writes sixty-four zeros —
`vibe_template::SKELETON_DIGEST` — plus `0` for a size, which is a *valid*
digest and therefore needs no validator exemption for the skeleton path.
`{meta.PATH}` was narrowed to a string, an integer or a boolean: an array, a
table, `null` and a non-integral number are refusals naming the path, which
keeps the typed section comparable and a rendered number exact.

**S2 — one choke point. No fifth call site; the four PROP-075
`#TEMPLATE-DOMAIN-SEAM` names are the right four, but which of them a path
uses depends on the path.** Measured with `eprintln!` probes in
`MultiRegistryResolver::{list_versions_in_registry, resolve_in_registry,
fetch_manifest_inner}`, `MultiRegistryResolver::fetch` and
`LocalRegistry::{list_versions, resolve}`, driven from a throwaway
`crates/vibe-cli/tests/cli_s2_choke_point.rs` over the `AppRegistry` fixture.

- `vibe install -g org.example/tool --registry DIR --plan-only` reaches
  **only** `LocalRegistry::resolve` and `LocalRegistry::list_versions`. It
  does **not** touch `MultiRegistryResolver` at all — `vibe-app`'s
  `resolve::from_local_registry` is a separate path from
  `resolve::from_global_with_operation`. That is precisely why
  `#TEMPLATE-DOMAIN-SEAM` names `LocalRegistry::{list_versions, resolve}` as
  its fourth site; without that site the whole `--registry DIR` surface would
  silently see no template.
- `vibe install -g org.example/tool --plan-only` against a **configured**
  `file://` `localdir` registry (written into `<settings>/registry.toml`)
  reaches `MultiRegistryResolver::resolve_in_registry` **and**
  `fetch_manifest_inner`, each of which delegates down to
  `LocalRegistry::{resolve, list_versions}`.
- A project `vibe install --assume-yes` of an exactly pinned requirement
  reaches `list_versions_in_registry` and `resolve_in_registry`, and **not**
  `fetch_manifest_inner`: the depsolver's manifest adapter is reached by the
  configured global path, not by an exact pin whose resolution already carries
  the manifest.

So the template layer must be wired at all four, and P2's prediction
("four call sites and one new module") stands. The instrumented test was
deleted with the probes.

**S3 — the shared index vocabulary. The golden bytes do not move, so
AV-FMT-003 stays epoch 1 → 1.** An optional `template: boolean` was added to
`version_entry` in `formats/vocabularies.json` and `cargo xtask codegen` run.

- The generated field is `pub template: bool` with
  `#[serde(default, skip_serializing_if = "std::ops::Not::not")]`, so an
  absent member is byte-identical.
- `formats/corpora/index/e1` was byte-for-byte unchanged (md5 of the 18
  files' md5s identical before and after codegen), and
  `cargo xtask rebuild --check formats/corpora/index/e1` reported the catalog
  byte-identical to its journal's projection (16 files).
- `cargo xtask wire-diff` was green (exit 0, REPORTING): shift classes
  **schema 0, corpus 0, other formats 1** — only `formats/vocabularies.json`
  itself moved.
- `cargo xtask check-codegen` requires the regenerated
  `crates/vibe-wire/src/generated/shared/mod.rs` to be committed together with
  the schema change, **plus exactly two struct-literal sites**:
  `crates/vibe-wire/src/behaviour/records.rs` and
  `crates/vibe-index/src/scanner/tree_entry.rs` (where the value is
  `pkg.template`). That is P2's complete mechanical cost for AV-FMT-003.

Everything was reverted; `formats/`, `crates/vibe-wire/` and
`crates/vibe-index/` are untouched by P1.

**S5 — proxy protocol /7. Confirmed on all three counts.**

- The writer writes **the lowest protocol that carries what the home
  retains** (`Index::minimum_reader`, ladder 3/4/5/6 keyed on
  `requires_reader_four` / `source_evidence` / `requires_reader_six`), so a
  home with no `/7`-only data never writes `/7`. The existing fixture
  `cli_app_proxy_index_versions::installing_common_configuration_keeps_legacy_and_rich_path_commands_working`
  shows that ladder live: a home that only retains a configuration writes
  `/5`, not `/6`.
- `vibe-app-proxy --supports-index` is one closed `match` arm list
  (`crates/vibe-cli/src/bin/vibe-app-proxy.rs`, the `"vibe-user-applications/3"
  | …/4 | …/5 | …/6` pattern): adding `/7` is one line there and one line in
  the reader's admission `match`
  (`crates/vibe-cli/src/bin/vibe-app-proxy/index.rs`), plus the new `e6`
  wire module. `--probe` keeps answering `vibe-app-proxy/2` — the proxy
  protocol and the index protocol are separate tags.
- Those two `match` arms are the **only** gate: nothing else in the proxy
  inspects the protocol string (`proxies/ownership.rs` matches only the
  family prefix `vibe-user-applications/` to classify an image as ours).
  Verified green: `cargo test -p vibe-cli --test
  cli_app_proxy_index_versions` — 3 passed, including
  `original_image_reports_exact_reader_capabilities_without_changing_probe`
  (`/3`…`/6` echoed, `/999` refused).

## 2026-11-10 — phase P1 implemented

`AV-FMT-001` delivered. New crate `crates/vibe-template` (pure render: TOML
text in, TOML text out, purity pinned by `tests/purity.rs`), the manifest
`[package] template` flag, the typed `[template]` section with its refusals
each naming a TOML path, `template_optional` on `[[installer_options]]`, the
digest-field exemption that admits a `{sha256:FILE}` variable in a template
manifest only, and the `template_render` check in `vibe check`. The golden
proof is `cargo test -p vibe-template --test node_golden`.

Facts moved to `impl/done` in PROP-075: `#TEMPLATE-FLAG`,
`#TEMPLATE-VERSIONS`, `#TEMPLATE-VARIABLES`, `#TEMPLATE-SPELLING`,
`#TEMPLATE-SOURCE-KINDS`, `#TEMPLATE-SELECTOR`, `#TEMPLATE-DIGEST-SOURCE`,
`#TEMPLATE-VARIABLE-GRAMMAR`, `#TEMPLATE-OPTION-GATING`,
`#TEMPLATE-RENDER-LOCATION`, `#TEMPLATE-TWO-STAGE`, plus the new
`#TEMPLATE-DOCUMENT-READING`. The spec XML was validated by the PROP-045
projector (`vibe_specdoc::project_spec_text` → `XmlProjected`), since
`vibe check` does not read the dialect — the gap this log recorded when the
campaign was planned.

Carried forward, not done: a template that uses the numeric coercion is
reported unparseable by `vibe check`'s manifest-validity cell (a lone variable
in a typed numeric field would have to be admitted by the typed manifest
layer). Neither real package needs it; filed here rather than in `BACKLOG.md`
because it is this campaign's own follow-up.
## 2026-11-10 — spike S4 (the hook's cost): the budget is met, nothing renegotiated

Measured on this machine (Linux, 32 cores) with a **debug** `vibe` (454 MB binary), which is
the pessimistic case: a release build is smaller and starts faster.

**Part A — the builtin-only fast path** (`bash` 5.x, no subprocess at all; the prototype is the
same shape the generated block uses, 20 000 iterations each, timed with the `$EPOCHREALTIME`
builtin):

| case | cost |
| --- | --- |
| prompt in an unchanged `$PWD` (early return) | **15 µs** |
| `cd`, no candidate file anywhere up the tree (8 dirs × 3 names = 24 builtin `[ -f ]` tests) | **205 µs** |
| `cd` into a directory that has one | **67 µs** |

Zero subprocesses in every case: the body uses only `[`, parameter expansion and array
iteration. zsh and fish were not installed on this host, so their bodies are covered by the
unit tests on the generated text plus the gracefully-skipping shell tests.

**Part B — the one `vibe` call** a `cd` with a candidate file makes, against a home with two
versions of a `path_dirs` runtime installed and a `.runtime-version` four directories up
(40 iterations each):

| command | warm |
| --- | --- |
| `vibe --version` (process floor) | 24 ms |
| `vibe app env --project --shell bash` | **21 ms** |
| the same with the resolution cache deleted before each call | 33 ms |

The resolution itself is inside the process floor's noise: what the budget pays for is starting
a debug `vibe`, not resolving a version. **The `< 50 ms` budget of
`PROP-075#ACTIVATION-HOOK` is met with better than 2× headroom, so neither the budget nor the
hook design was renegotiated.** The first measurement, taken before P5a existed, used
`vibe app alternatives <slot> --json` as the nearest existing analogue (16 ms against an 11 ms
floor) and agreed.

The budget is now a test, not a measurement:
`cargo test -p vibe-cli --test cli_hook_linux hook_spawns_nothing_in_an_unchanged_directory`
puts a counting shim named `vibe` on PATH and asserts 0 / 0 / 1 invocations for the three cases
above.

## 2026-11-10 — phase P5a implemented

`[[application.version_file]]`, the upward search with its mtime+size cache, the activation
precedence, `vibe app env`, `vibe hook bash|zsh|fish|pwsh`, `vibe app exec` and the promotion of
the `Shell` enum out of `commands/vvm/`. `AV-FMT-011`, `AV-FMT-012`, `AV-FMT-013` filled;
`AV-FMT-101` and `AV-FMT-102` added for two home documents the plan did not foresee. PROP-075
gained `#P5A-AS-BUILT` with the nine amendments; `#VERSION-FILES`, `#ACTIVATION`,
`#VERSION-EXEC`, `#ACTIVATION-ENV-OUTPUT` and `#ACTIVATION-HOOK` are `impl/done`, and
`#VERSION-FILE-FORMATS` and `#VERSION-EXEC-SPELLING` are `impl/work` because channel lowering
beyond `latest` / `stable` belongs to P2.

Facts a later session would otherwise re-derive:

- **The declared version files could not go in the application index.** `#ACTIVATION-HOOK` wants
  the hook's one call to read the home's index and the version files only, so the home must
  retain the declarations — but adding them to `Installation` moves the inventory to
  `vibe-user-applications/7`, which drags in the whole of `AV-FMT-017` (a new `application/e6`
  schema generation, its corpus, a `/6 → /7` promotion, and the `vibe-app-proxy` admission
  PROP-059 `#proxy-index-upgrade` requires) — P2's deliverable, not P5a's. The sidecar
  `applications/version-files.json` (`AV-FMT-101`) keeps the index at `/6` and every existing
  proxy working. `PLAN.md` §4 P5a's file list did not name `crates/vibe-app/src/index.rs`, which
  is consistent with the sidecar and not with retaining the rows in the record.
- **`commands::app::home::project_at` does not walk up.** It answers "is *this* directory a
  project", for the project-home scope of PROP-059 `#project-scope`. A version-file search needs
  the nearest *ancestor* holding a `vibe.toml`, so `commands::app::env::enclosing_project` is a
  second, different resolver. Confusing the two makes the search run to the filesystem root from
  inside a project.
- **An installation key and a channel both carry a `/`.** `org.example/node@22.0.0` and
  `lts/jod` are told apart by the `@`, never by the slash; the first attempt read `lts/jod` as a
  key and refused `not_installed`.
- **`[requires]` is a TOML *table*, not an array**: `[requires.packages]` with
  `"org.example/runtime" = "=1.0.0"`. A project fixture also needs `version` under `[project]`.
- **`cargo xtask check-codegen` diffs the generated tree against `git`, not against disk.** With
  a new schema staged but not committed it reports drift that a `cargo xtask codegen` cannot fix;
  commit, then check.
- **`tools/jtd-codegen/jtd-codegen` is git-ignored**, so a fresh worktree has no code generator
  until the binary is copied in from another checkout.
- **The test target `cli_app_env` was already taken** by the PROP-065 env-step fixture
  (`crates/vibe-cli/tests/cli_app_env.rs`). P5a's CLI test is `cli_app_env_activation`, and
  `PLAN.md` §4's acceptance line for P5a is amended to name it.

## 2026-11-10 — phase P2 implemented

`AV-FMT-002`, `AV-FMT-003`, `AV-FMT-004` and `AV-FMT-005` delivered, plus the
four records the phase turned out to need: `AV-FMT-021` (a channel where a
version requirement is written), `AV-FMT-022` (`--refresh`), `AV-FMT-023`
(`<settings>/cache/templates/`) and `AV-FMT-024` (a channel in a version file
resolves, completing P5a's `unresolved_channel`). Facts moved to `impl/done` in
PROP-075: `#TEMPLATE-RESOLUTION`, `#TEMPLATE-TRUST`, `#CHANNELS`,
`#TEMPLATE-DISCOVERY`, `#TEMPLATE-DOMAIN-SEAM`, `#TEMPLATE-CONCRETE-WINS`,
`#TEMPLATE-LOCK-RECORD`, `#TEMPLATE-CACHE`, `#CHANNEL-SYNTAX`, plus the new
`#P2-AS-BUILT`. The spec XML was validated by the PROP-045 projector, as P1's
entry records it must be.

Facts a later session would otherwise re-derive:

- **`VersionSpec` is read at eighteen sites, not the eight the survey named** —
  and every one of them is an exhaustive `match`, so adding the `Channel`
  variant turned «a missed lowering site becomes a wrong install» into a build
  error. That is the whole reason the fail-closed design costs nothing: the
  compiler enumerates the sites and each one is then a two-line decision.
- **Lowering happens at the registry resolution seam, not at each entry path.**
  `#CHANNEL-SYNTAX` lists the entry paths; as built the one named function
  `lower_channel` is called inside `LocalRegistry::resolve`, which spike S2
  already proved all three real paths reach. Strictly stronger (no entry path
  can forget) and the solver is untouched either way.
- **`ResolvedPackage` must not grow a field.** It is the type the four call
  sites hand back, and widening it means touching every construction site in
  the tree. The template layer instead *notes* `(package, version) → {template
  lock record, via_channel}` for the invocation, and the fetch reads the note
  onto `CachedPackage`. Both halves are `None` for an ordinary package, which
  is what keeps a lock byte-identical.
- **Raising `CURRENT_SCHEMA_VERSION` to 10 broke `validate_remap_sources`**,
  which read its epoch floor off that constant. The floor is now
  `REMAP_SOURCE_SCHEMA_VERSION = 9`. The next epoch step will meet the same
  hazard; this is the only place the 7→8→9 precedent had not already been
  followed.
- **`reqwest` cannot fetch a `file://` URL, and a closed network needs exactly
  that.** A template's documents are declared as credential-free HTTPS
  (`#TEMPLATE-DIGEST-SOURCE` refuses anything else), and a `[[remap]]` row
  relocates them onto the filesystem — a `path` target yields a bare path, a
  `url` target may yield `file://`. The transport reads both off disk, joining
  the declared URL's last path segment when the target is a directory. This is
  also what makes the offline CLI suite possible without a loopback server, and
  what `#MIRROR-TEMPLATE-SNAPSHOT` will need in P4.
- **`vibe-app`'s containment check refuses a rendered tree** («the package tree
  escapes the selected registry»): a render lives under the settings cache
  because `#TEMPLATE-RENDER-LOCATION` forbids writing one into a registry. It
  admits that one root by name (`vibe_registry::templates::is_rendered_tree`),
  never by widening the check.
- **The alarm has a call site that needs no lock.** `#TEMPLATE-LOCK-RECORD`'s
  comparison against `rendered_sha256` is `verify_recorded`, which the project
  lock path will use; the alarm *also* fires from materialisation, by comparing
  the digest-bearing fields of a rendered directory already on disk with the
  fresh render. Since a digest document is cached write-once and immutable, a
  difference there can only mean the document changed upstream — which is the
  alarm — while a difference outside those fields is a template fixed without
  touching a produced version, which TEMPLATE-FLAG exists to allow and which
  rewrites silently. Both behaviours are tested through the real binary.
- **A channel can be lowered from the cache alone**, which is what lets a
  `.nvmrc` holding `lts/jod` resolve inside `#ACTIVATION-HOOK`'s budget: the
  cache keeps the template *document* beside the version list
  (`template.toml`), so the `[[template.channel]]` rules are two file reads
  away and no registry root is needed. A declaration is not an output, so
  `#TEMPLATE-RENDER-LOCATION`'s ban on caching a render is untouched.

Carried forward, not done, each with its reason in `FORMAT-CHANGES.md`:
`AV-FMT-017` and `AV-FMT-018` (the `/7` inventory and the describe member that
reads it — `/7` must also carry P6's migration outcomes, and the writer's
protocol ladder means nothing a user sees waits on them); the two extra golden
documents `AV-FMT-003` proposed under `formats/corpora/index/e1` (a corpus
document is a journal fact and S3 already proved the bytes do not move); and
template **rendering** from a per-package git registry, where the index member
and the `template_index_required` refusal are delivered but the fetch-and-render
path is not — every directory-shaped registry resolves templates today.

## 2026-11-10 — phase P3 implemented

`AV-FMT-006` delivered: `org.vibevm.node/node` is one template package.
Outside this repository, `/fast/git/vibe-pkg-node` carries the `[template]`
manifest at template revision `1.0.0` — commit `1a52f3d`, new tag `v1.0.0` —
and keeps `v24.22.2` and `v22.23.4` published concretely;
`/fast/git/vibe-pkg-node-generator` records its own supersession in its README
(`c160ba3`). **Nothing was published or pushed**, and `org.vibevm.codkit/codkit`
was not touched. In this repository the Node fixtures became upstream's own
frozen documents and `crates/vibe-cli/tests/cli_template_node.rs` drives the
channels, the optional rows and the oldest version in the `min` window through
the real binary. No fact of PROP-075 sections 2–5 remained for P3 to move to
`impl/done` — P1 and P2 closed every one of them — so the spec change is the
new `#P3-AS-BUILT` (four amendments) plus the fifth `changelog` entry and a
header that no longer claims P1 is the only implemented phase. The spec XML was
validated by the PROP-045 projector, as P1's entry records it must be.

The hand acceptance, with a throwaway `HOME` and `VIBE_SETTINGS` under
`/fast/tmp` (the real `~/.vibe` was never read or written) and the real
network, against a directory registry holding the committed template plus both
concrete tags:

```text
vibe install -g org.vibevm.node/node@lts --registry <dir> --assume-yes
  -> plan for org.vibevm.node/node@26.13.0, installer linux-x64, 1 step
  -> installed org.vibevm.node/node@26.13.0 via installer `linux-x64`
<settings>/opt/paths/node/0/node -v   -> v26.13.0
<settings>/opt/paths/node/0/npm  -v   -> 11.20.0
vibe app exec node -- node -v         -> v26.13.0
```

The `vibe app versions org.vibevm.node/node --json` line of the plan's
acceptance was **skipped: that command is phase P5b and does not exist yet.**

Facts a later session would otherwise re-derive:

- **`template_optional` needed a third row, found while preparing exactly the
  test the plan prescribed for that risk.** Measuring the frozen index for the
  oldest version in the `min` window showed that
  `node-v12.0.0-darwin-arm64.tar.gz` does not exist: upstream's Apple-silicon
  archive begins at Node 16.0.0, so with `min = "12.0.0"` every release below
  16 refuses to render unless `macos-arm64` is optional too. Measured over the
  406 produced versions of the frozen index: `linux-x64-musl` absent from 389,
  `osx-arm64-tar` from 132, `win-arm64-zip` from 234; all seven targets present
  together only from 26.8.0 up. The general rule, now in the package's own boot
  document: a row upstream stops building must *become* optional, never
  disappear, or old versions stop installing.
- **`files[]` in `index.json` is not what gates a row — `SHASUMS256.txt` is.**
  The renderer drops a `template_optional` row when the *digest document* has
  no entry for its archive, so `files[]` is a survey instrument for choosing
  which rows to mark (which is how the three were chosen, and what the
  planning survey measured) and not the authority for any single version. The
  four versions whose checksum documents are frozen here agree with their
  `files[]` records; nothing says every release does, and only the digest
  document decides.
- **The frozen digest documents are the retired generator's own inputs**, which
  closes the provenance chain the golden proof was missing: upstream's
  `SHASUMS256.txt` for 24.22.2 and 22.23.4 hash to `aec92ae9…` and
  `c90c3f14…`, exactly the values the committed manifests' four-line provenance
  headers record, and `node_golden.rs` asserts that equality. P1's hand-derived
  seven-line excerpts were verified against them first (0 mismatches) and then
  replaced. The live install confirmed the freeze from the other side: the
  `index.json` the real run fetched hashes to the committed subset's source
  document, `6b27fe57…`.
- **A real upstream index can be frozen without trimming its meaning.**
  nodejs.org writes one record per line, newest first, so the committed subset
  is a *leading slice* — down to and including the declared `min`, plus the
  next three records so the `min` filter is still exercised — with every kept
  record byte-verbatim: 410 of 878 records, 175 KiB instead of 329 KiB, and
  because the document is sorted the subset is the template's **whole** version
  domain. Every channel answer the suite checks is therefore upstream's real
  answer. One thing is lost with the hand-written excerpt and recorded in the
  fixture README: upstream has no non-semver record, so the "skipped with a
  reason" path is covered by the unit tests alone.
- **A requirement is not a pin, and for a template that now chooses the
  document to fetch.** `@12.0.0` is `^12.0.0` and resolved to 12.22.12 — the
  first run of the oldest-version test failed asking for a digest document the
  fixture deliberately does not carry. Harmless for a published package with
  three versions; for a template whose domain is 406, an inexact spelling
  silently moves both the release and its digest document. `@=12.0.0` is the
  spelling that means one release.
- **`install -g --plan-only` performs the FULL render, not the skeleton.**
  `vibe_app::resolve::from_global_with_operation` resolves through
  `resolve_and_fetch`, so the plan already costs the digest document and
  materialises the rendered tree; the skeleton stage serves the depsolver's
  `fetch_manifest`. That is what lets every case in `cli_template_node.rs` be a
  `--plan-only` run and still assert what was rendered, and it is worth
  remembering before reading `#TEMPLATE-TWO-STAGE` as a promise about the
  global path.
- **One `[[remap]]` row makes the whole Node template offline.** A `prefix` row
  with a `path` target joins the URL remainder onto the directory
  (`PathLookup::Mirror`), so `prefix = "https://nodejs.org/dist/"` →
  `<dir>` serves both `index.json` and `v<version>/SHASUMS256.txt` from a tree
  that mirrors upstream's layout. Without the remainder join, every version's
  digest document would collide on one file name.
- **Why the package is not published yet.** Rendering a template found in a
  **per-package git registry** is the part of `#TEMPLATE-DISCOVERY` P2 left to
  the phase that first needs it, and P3 does not need it: the suite and the
  hand install both use a directory-shaped registry, which is what
  `--registry DIR`, a `localdir` source, a `monorepo` working copy and a
  mirrored layout registry all are. Publishing before that gap closes would
  expose the template revision to a template-unaware client as if it were a
  Node.js version; the revision is numbered `1.0.0`, below every produced
  version, so even such a client prefers a real one.
- **Disk, on this machine:** a cold `CARGO_TARGET_DIR` for this workspace does
  not fit in the free space `/fast` had (30 GB free; the main tree's `debug/`
  alone is 138 GB). Every cargo command of this phase therefore ran with
  `CARGO_PROFILE_DEV_DEBUG=0` beside the mandated `CARGO_BUILD_JOBS=6`, which
  kept the whole target directory at 4.9 GB and changes nothing a gate
  observes.

## 2026-11-10 — phase P6 implemented

`AV-FMT-015` and `AV-FMT-016` delivered, plus **`AV-FMT-017`** — the
`vibe-user-applications/7` inventory P2 deferred — and the one record the phase
turned out to need, `AV-FMT-103` (`vibe app work` lists and cleans a migration's
kept work directory). Facts moved to `impl/done` in PROP-075: `#MIGRATION`,
`#MIGRATION-DECLARATION`, `#MIGRATION-FAILURE`, plus the new `#P6-AS-BUILT`.
The spec XML was validated by the PROP-045 projector, as P1's entry records it
must be. The question P2 left open is answered: **the declared version files
stay in their sidecar** and `/7` does not absorb them.

Facts a later session would otherwise re-derive:

- **The inventory retains no package tree, so a migration must re-resolve
  both versions.** A migration step is the package's own script
  (`#MIGRATION-DECLARATION`: `VIBE_PACKAGE` is "the package tree of the version
  that owns the step"), and nothing in `Installation` points at one — that is
  deliberate, which is why `AuthoredCommands` exists for uninstall. So
  `vibe app migrate` resolves `FROM` and `TO` through the ordinary registry
  chain exactly as `vibe app modify` does, and therefore takes `--registry DIR`
  like every other package-reading `app` subcommand. The spelling in `PLAN.md`
  §4 is amended to include it.
- **The offer after `vibe app use` is why the index retains the *declared*
  entries.** `install -g` and `update -g` have the new version's
  `ResolvedPackage` in hand and could read its declaration directly; `app use`
  resolves nothing at all. Retaining `{id, name, description, default}` on the
  installation makes all three offers one function over the inventory, and makes
  `--only ID` checkable without a registry. It rides in the `/7` move rather
  than forcing a second one.
- **The template provenance needed no new field on `ResolvedPackage`.** P2's
  amendment (3) refused to widen that type, and it stays refused: the
  publication path reads `vibe_registry::templates::active()` and asks the
  installed layer for its `noted(group, name, version)` — the same note the lock
  path reads — so a global install records what rendered it without the resolver
  carrying the fact. This is the one piece of AV-FMT-018 that P6 delivers; the
  *reader* (`vibe app describe`) is left to the phase that owns that document,
  because describe describes a **package resolved from a registry**, not an
  installation.
- **Two sub-claims of `#MIGRATION-DECLARATION` are honoured and one is
  substituted.** "Both run through the ordinary engine — the step runner, the
  interpreters, `needs`, the display panel of `#display`, the cancellation of
  `#cancellation` and the run records of `#runs`" is literally true of the step
  runner, the interpreters, `needs` (evaluated per side, so an export whose
  `command:` is missing refuses *before* its child starts and the entry has
  changed nothing) and cancellation (the caller's interrupt reaches the child
  through `ChildIo`). It is **not** literally true of two: a migration uses the
  flat display rather than the step panel, because two steps per entry do not
  make a panel; and it writes no `app-runs` record, because a `RunRecord` is
  keyed to an installation and one of four `Command` verbs. The evidence lives
  in the always-kept work directory instead — per-side logs plus the
  `vibe-app-migration/1` record — which is strictly stronger than `#runs`,
  whose records are prunable by `vibe app runs --prune`.
- **`AppError` has a size budget, enforced by clippy.** Adding a seven-field
  `MigrationFailed` variant to `StepError` pushed `AppError` past
  `clippy::result_large_err` and turned ~200 unrelated functions across the
  crate into errors. The payload is now a boxed `MigrationFailure` struct with
  `#[error(transparent)]`; the next variant with more than about four `String`s
  will meet the same wall, and boxing is the answer, not an allow.
- **`consent_required` was taken.** `#MIGRATION-FAILURE` names the refusal
  `consent_required`; `AgentError::ConsentRequired` (PROP-064) already holds that
  code, and `codes.rs`'s own
  `every_code_is_unique_snake_case_and_explained` makes the codes of this
  subsystem globally unique by construction. The migration refusal is
  `migration_consent_required`, same `Refusal` class, same exit 2, same printed
  command.
- **The shared work directory's top level belongs to the scripts.** The first
  version put an entry's logs at `<work>/<entry id>/`, and the CLI fixture
  caught the collision on the first run: an entry called `second` could not write
  a file called `second`. The logs are `<work>/logs/<entry id>/export.log` and
  `import.log`, and the fixture now *asserts* that an entry may name a file after
  itself.
- **Each side's PATH is one `Exposure` through `StepPaths::child_env`**, exactly
  as the plan predicted — and the fixture proves it from the outside rather than
  by reading the code: each side calls the slot's own command and records which
  marker answered (`who=st-1.0.0` in the export, `who=st-2.0.0` in the import).
  The same fixture asserts `VIBE_OUT` and `VIBE_SOURCES` are **unset**, which is
  `#MIGRATION-DECLARATION`'s "a migration produces no payload" from the child's
  own point of view.
- **What `--resume` can and cannot know, exactly.** The record is written
  after each entry, so an entry whose *import* failed is recorded `import_failed`
  and a later `--resume` re-runs only its import — the specified case. A run
  **killed** between a successful export and its import leaves no record of that
  export, so a resume re-runs it; that is safe, because an export is the
  package's own script writing into the shared directory, and adding a second
  mechanism (a sentinel file beside the logs) for an unspecified case was
  deliberately not done. A resume also now *carries forward* the kept record of
  an entry it was not asked to run, so `--only A --resume` followed by
  `--only B --resume` does not lose A's state, and the run's verdict is computed
  over the entries **this** run selected — otherwise a successful resume of one
  entry would exit 1 because another failed an hour earlier.
- **The report *is* the resumable record.** One document,
  `vibe-app-migration/1`, is both the `--json` answer and `<work>/migration.json`,
  rewritten after every side. That is what `--resume` reads, so there is one
  registered format instead of two, and an interrupted run leaves a true record
  behind. A corrupt one is no record and costs one re-run, never an error.
- **A `/7` home is still routable, measured.** `vibe-app-proxy` admits `/7` in
  its closed set and answers `--supports-index vibe-user-applications/7`;
  `proxies::upgrade::probe` already loops `3..=reader`, so the admission and the
  refresh of existing readers needed no new mechanism. The `e5` reader is
  registered frozen as `[format.application-index-v6]` and
  `source_evidence_corpora.rs` now asserts it **refuses** the `e6` corpus.
- **Spike S5's estimate held.** The `/7` move cost the two `match` arms it
  predicted plus the `e6` wire module — and three mechanical sweeps it did not:
  `Installation`'s three new fields through every test fixture that builds one
  by hand (ten files), and the two `e5`-typed helpers outside `index/wire.rs`
  (`index/launcher_wire.rs`, `operations/pending/identity.rs`). The predecessor
  digest is unchanged for a `/6`-shaped record, because the new members are
  skipped when empty.
- **Disk, on this machine:** `/fast` had 30 GB free throughout; every cargo
  command ran with the mandated `CARGO_BUILD_JOBS=8` and `--test-threads=8`.

## 2026-11-10 — phase P5b implemented

`vibe app versions PACKAGE|SLOT [--channel NAME] [--installed] [--limit N]
[--all] [--refresh] [--registry DIR] [--json]` exists, answers
`vibe-app-versions/1`, and renders nothing. `AV-FMT-014` is delivered;
`#VERSIONS-LIST` and `#VERSIONS-LIST-OUTPUT` are `impl/done`; the five
amendments are `PROP-075#P5B-AS-BUILT`.

What implementing it taught, beyond the as-built record:

- **The listing wanted an enumeration seam the tree did not have.** Every
  existing path into the app resolver resolves exactly **one** version —
  `from_global_with_operation` goes straight to `resolve_and_fetch`, and
  `vibe app outdated` asks it for the newest and nothing else. A listing needs
  the domain, so the resolver gained three read-only functions beside the
  resolving one: `global_resolver` (the chain `from_global_with_operation`
  already built, now named so a second caller can have it unchanged),
  `versions_from_global_with_operation` and
  `directory_roots_from_global_with_operation`. They live in `vibe-app`, not in
  `vibe-package-source`, for a reason worth keeping: that crate's dependency
  set is an **exact** R-001 fence (`fence_tests.rs`), and naming
  `semver::Version` in its public API would have widened it. `vibe-app` already
  depends on both `semver` and the resolver, so the match over
  `InstallResolver`'s four variants sits there and the fence stays as it was.
- **`--installed` is free of the network by construction, not by discipline.**
  The join takes its domain through a `Domains` trait; `--installed` hands it an
  implementor that reads the template cache alone. There is therefore no code
  path from `--installed` to a registry to forget to guard, and the suite proves
  it the only honest way — by deleting the registry directory outright and
  asserting the listing still answers.
- **Closed JTD vocabularies cannot spell the design's own words.** PROP-075
  writes `published (overrides template)` and `installed (default)`. A closed
  enum member may not carry a space or a parenthesis, so the wire tokens are
  `overrides` and `installed_default`, and the parenthesised forms are the human
  rendering. Worth stating in `AV-FMT-014` rather than leaving a reader of the
  fact to discover it in a schema.
- **The rows must be a union, not a filter.** The first shape filtered the
  domain by the installed set; a version whose registry row is gone then
  vanished from a listing of a home that has it installed. The rows are now the
  union, and a version only the home carries is `published` with its local
  state. There is a test for exactly that, because the failure mode is a
  listing that lies about the machine it runs on.
- **A listing must not render, and that is checkable.** `cli_app_versions.rs`
  asserts `cache/templates/org.vibevm.node/node/rendered` does not exist after
  listing the whole Node domain — the skeleton-path claim of
  `#TEMPLATE-TWO-STAGE` as a check rather than a comment. (Contrast P3's note
  that `install -g --plan-only` performs the *full* render: the two surfaces
  differ, and only one of them is the skeleton path.)
- **Four error arms were nineteen lines too many.** Adding the four refusals to
  `crates/vibe-app/src/error.rs` in place carried it from 591 to 619 lines, past
  the conform `max_file_lines = 600` — the only new conform finding the phase
  produced beside a single `.expect()`. They became their own `VersionsError`
  family in `error_versions.rs`, which is also the better shape: the sibling
  `error_paths` and `error_lifecycle` families exist for exactly this reason,
  `AppError` delegates transparently so no caller's `match` changes, and
  `codes.rs` gains one more table rather than four more rows. Worth knowing
  before the next phase adds an error arm to that file — P6 adds several.
- **Disk, on this machine:** `/fast` fell from 53 GB to 6.7 GB free while three
  workers built concurrently, and the test profile rebuilds the whole tree
  rather than reusing the `cargo build` artifacts. 13.6 GB of that was this
  worker's own `debug/incremental`, which is pure cache: deleting it and running
  every later command with `CARGO_INCREMENTAL=0` beside the mandated
  `CARGO_BUILD_JOBS=6` and `CARGO_PROFILE_DEV_DEBUG=0` returned the free space
  to 22 GB and changes nothing a gate observes. Worth doing *before* the first
  build on a shared disk, not after.

## 2026-11-10 — phase P4, commit (a): the declarative mirror selection

`vibe registry mirror --config mirror.toml` exists. `AV-FMT-007` is delivered;
`#MIRROR-CONFIG` and `#MIRROR-CONFIG-SCHEMA` are `impl/done`; the four
amendments it needed are in `AV-FMT-007` and `PROP-075#P4-AS-BUILT`. The
phase's other two commits — **(b)** the `_index/templates/` snapshot writer and
**(c)** payload projection, the budget and prune — are **open**, and the seven
amendments now in `PLAN.md`'s P4 section bind them.

What implementing it taught, beyond the as-built record:

- **The old selection was two methods, and that is why listing did not
  change.** `Filters` looked like a type the source side was built around; it
  was read at exactly `admits(group, name)` and `packages()` (plus
  `tombstone_admitted`, which is `packages()` again). Making the new
  `Selection` a **superset of those two methods** meant `mirror/source.rs` and
  `mirror/source/api_listing.rs` changed by rename alone — no new branch, no
  second listing path — while `all`, the excludes, the `[[template]]` rows and
  the payload plan ride beside them. A seam worth measuring before replacing.
- **An empty axis cannot mean two things.** `Filters` read «no packages and no
  groups» as «everything», which is right for a flag nobody passed. Under
  `[select]` it is wrong: a configuration whose only content is `[[template]]`
  rows would have selected the whole source. The fix is one early return —
  with neither ordinary axis naming anything, only the template rows select —
  and it costs nothing because a run with no selection at all never reaches
  that rule: it is `all` from the start. The test that caught it was the one
  asserting a template row does *not* admit its package's siblings.
- **Recording a command-line flag would have been a trap, and a real test
  found it.** The entry said «the effective selection — merged from the file
  and the flags». Implemented literally, `--package org.corp/demo` wrote a
  `[select]` table, and `cli_registry_mirror_api`'s third leg went red: a run
  that had deliberately passed no `--package`, to prove the organisation-API
  listing opens once a token is configured, now repeated the narrowing a
  *previous* leg had recorded and listed by tags instead. That is exactly the
  user-facing trap — `--package` also exists to make a source that can be
  listed neither by index nor by API mirrorable one package at a time, so
  recording it would turn a workaround into a permanent narrowing. Only a
  `--config` selection is recorded now (`AV-FMT-007` amendment 1), which also
  keeps the record byte-identical for every pre-`--config` workflow and so
  keeps `schema = 1` honest.
- **The target must be read before the source is listed.** The recorded
  selection lives in `_index/mirrors/<source>.toml`, inside the target's tree,
  and it decides what the source is asked for. `read_target` therefore moved
  ahead of `reader.list`; for a `monorepo` that is the same shallow fetch, one
  stage earlier.
- **`WRITER_DIRS` and `repomd.json` are not on this phase's path after all.**
  Both were read while preparing (b) and both came back the other way: the
  whitelist describes what the index writer *produces*, and the snapshot is
  written through `BatchConfig.files`; `listing.txt` is rendered over
  `version_dirs`, which skips `_index` entirely, so the `Stale` risk the plan
  carried cannot happen. `AV-FMT-008`'s last bullet is withdrawn and
  `AV-FMT-010` is withdrawn whole. Two files the plan named are not touched,
  and one of them was the phase's stated risk.

## 2026-11-10 — phase P4, commit (b): the template snapshot

`vibe registry mirror --config` now writes a template's external inputs into the
target's `_index/templates/<group>/<name>/`, and a client inside a closed network
renders from them with no upstream request at all. `AV-FMT-008` is delivered;
`#MIRROR-TEMPLATE-SNAPSHOT` and `#MIRROR-SNAPSHOT-LAYOUT` are `impl/done`; the six
amendments it needed are `PROP-075#P4-AS-BUILT` 11–16. Commit **(c)** — payload
projection, the budget, `--dry-run` counts and prune — is still open.

What implementing it taught, beyond the as-built record:

- **The client half turned out to be one act, not a reader.** The first shape in mind
  was a snapshot-aware path inside `version_document` and `digest_document`: ask the
  registry first, fall back to the cache. That is two readers for one question, which is
  precisely the duplication `#TEMPLATE-RENDER-LOCATION` refuses for rendering. The shape
  that fits is **priming**: discovery copies the snapshot's `versions.json` and
  `digests/*` into this machine's own template cache, and every later read is the
  ordinary cache path. Nothing below `discover_in_directory` learns that a registry was
  involved, so `#MIRROR-TEMPLATE-SNAPSHOT`'s "with the same code as outside" is literal.
  The diff is about forty lines in `discovery.rs` and no change at all to the two
  readers.
- **`versions.json` had to be the client cache's own envelope, and that forced a
  recomputation.** The design already said "the same envelope `#TEMPLATE-CACHE` caches,
  with the record array filtered"; what it did not say is that `CachedVersionList`
  carries `sha256` **of the document**. Filter the records and the hash stops describing
  the bytes beside it — so the snapshot writer recomputes it, and the reader now checks
  it, because an envelope whose hash is a lie would put bytes in the cache under a
  provenance that is not theirs. That check is a test of its own
  (`an_envelope_whose_sha256_does_not_describe_its_document_is_ignored`), written against
  exactly the mistake the amendment prevents.
- **Every way a snapshot can be broken had to be decided before the writer, not after.**
  A snapshot is a cache of someone else's bytes, and a registry serves every package, not
  only the templated one. So a wrong `schema`, an unparseable record, a bad envelope hash
  and a `[[version]]` row whose digest file is missing or does not hash to it all read as
  **absent**, each with a warning where it is detected. The alternative — refusing — would
  let one half-finished mirror run make a whole registry unusable for packages that have
  no template at all.
- **The alarm needed a faithful fixture, and the first one was not.** Tampering the
  digest document by replacing it wholesale made the run fail earlier, as a *render*
  error: the document no longer named the files the template references. The alarm only
  fires once a version renders, so the fixture now keeps every filename and moves one
  digest — which is what "upstream replaced a released version's checksums" actually looks
  like. A test that fails for the wrong reason proves nothing, and this one was one line
  away from passing for the wrong reason.
- **A new `_index/` directory cannot make the index stale, and that is now tested twice.**
  The plan carried the opposite as the phase's stated risk. `listing.txt` is
  `render_listing(version_dirs, journal_digest)` and `version_dirs` skips `_index`, so the
  focused test in `vibe_core::layout_registry` builds the whole snapshot layout — including
  a directory named like a group and one named like a version, the shape that would catch a
  listing that walked `_index/` — and asserts the listing is byte-identical and the state
  `Current`. End-to-end, an identical second mirror run reports `facts_recorded = 0` and
  `index_refreshed = false`, which is the same claim from the other side.
- **Four records beat four hundred for a selection test.** P3's frozen index is the whole
  410-record declared window, and this suite renders every version it selects — so a
  `versions = [">=22"]` against that index would need a digest document for each of
  dozens of versions, and the fixtures carry four. The suite therefore uses a four-record
  subset built from the same frozen document, upstream's own bytes and its own `lts`
  codenames, which is enough to make `channels = ["lts"]` and `newest` decide something
  real. The render-fidelity questions stay with P3's suite, which is where the big index
  belongs.

## 2026-11-10 — phase P4, commit (c) part 1: the removal channel

`BatchConfig` has the removal channel `PLAN.md`'s P4 amendment 4 named, and it
does what `#MIRROR-PRUNE` requires: a pruned version's bytes, its index rows and
its journal fact leave in **one** commit. Nothing user-facing changed — no run
deletes anything, because the mirror passes `removed: vec![]` and nothing else
can reach the channel — so there is no `AV-FMT` id and no changelog line for a
user to read. `#MIRROR-PRUNE` stays `spec/plan` until the command surface exists.

Still open in (c): payload projection with digest verification and the pre-write
`max_total` budget, `--dry-run` counts and bytes, the `--prune` /
`--prune-payloads` / `--yes` surface with its plan, and `AV-FMT-009`'s two
report arrays.

What implementing it taught:

- **The order is delete-then-reproject, and the journal fact alone is not
  enough.** `vibe_index::layout::refresh` projects the catalog from
  `layout::version_dirs(root)` — the tree, not the journal — so a version whose
  directory is still on disk when the index is rewritten is projected straight
  back in, `Event::Removed` or no `Event::Removed`. The fact is the *record* of
  the removal; the deletion is what makes the catalog stop advertising it. Both
  belong to the same `write_index` call, which is exactly why this had to be a
  batch channel and not a step beside the batch.
- **`monorepo` is the backend where a removal can silently not happen.**
  `commit_batch` stages a computed list of changed paths, so a deletion that is
  not in that list leaves the working tree without the file and the *commit*
  with it — the push then succeeds and the registry still advertises the bytes.
  The removed paths now join `changed`, and the test asserts against the pushed
  bare repository's `ls-tree` and its committed `primary.jsonl`, not against the
  temporary working tree, because only the pushed state can tell the two cases
  apart.
- **A removal path is validated against the coordinate it claims.** The channel
  carries a path *and* the `(group, name, version)` whose journal fact it
  implies, and those two can disagree — a caller could delete `v1.0.0` while
  recording that `v9.9.9` went. Then the bytes and the index would describe
  different worlds. So the path must spell the coordinate exactly, a path that
  is neither a version directory nor inside `_index/` is refused outright
  (a batch has no business deleting the rest of the tree, and the mistake is
  unrecoverable), and every refusal happens before anything is deleted.
- **A path the tree does not hold is dropped, not refused.** A prune plan is
  computed against one reading of the target and applied against a possibly
  newer one; a version someone else already removed is the outcome the caller
  asked for, not an error. That is the one place the channel is deliberately
  permissive, and it is tested.

## 2026-11-10 — phase P4, commit (c) rest: payloads, the budget, prune, the report

P4 is **delivered**. `vibe registry mirror --config` now projects a
`[[template]]` row's payloads into the package drop, weighs them against
`[payloads] max_total` before writing a byte, shows the projection on a dry run,
and — only when told, and only after a plan — removes what the recorded
selection no longer matches. `AV-FMT-009` is delivered with both report arrays;
`#MIRROR-PAYLOAD-PROJECTION`, `#MIRROR-PAYLOAD-BUDGET`, `#MIRROR-DRY-RUN-OUTPUT`,
`#MIRROR-PRUNE` and `#MIRROR-NO-SILENT-PRUNE` are `impl/done`, and
`PROP-075#P4-AS-BUILT` gains amendments 18–27.

What implementing it taught, beyond the as-built record:

- **The projection had no business being a new fetch, and proving that was the
  design.** The fact says the mirror fetches each input "through the operation's
  controlled transport" and verifies the digest — which is a description of the
  installer's own Locator fetch with a different destination. Writing a second
  reader would also have put a new line in `PROP-068#coverage`'s exemption list,
  which is the gate that exists to notice exactly this. So `vibe-app` grew **one**
  public entry, `fetch_declared_input`, wrapping the `fetch_located` the installer
  already uses: the remapped candidates in winning order, the digest on whichever
  served the bytes, the transfer journal. The mirror's own module sends no HTTP
  request at all, so the coverage test never had to hear about it — the right
  outcome, arrived at by not writing the code.
- **The renders were already there, and keeping them had to be conditional.** A
  payload projection needs each selected version's rendered manifest, which the
  snapshot stage renders anyway. Keeping them is free for a four-version row and
  four hundred manifests for a four-hundred-version one, so the stage keeps a
  render **only** when the row says `payloads = true`. That one condition is what
  makes "read the renders you already have" a saving rather than a leak.
- **Commit (b)'s deliberate omission had to be undone, and it was right to be
  deliberate.** (b) reported a template row where it was produced and carried
  nothing out, on the stated ground that a carrier with one reader would be a
  half-built wire surface. Two readers then arrived in the same commit — the
  projection and `templates[]` — and with them the reason a row must be drawn
  once rather than twice. The omission cost one small refactor and bought a
  commit with no unread state in it; the reverse order would have cost a review.
- **A declared `size` is the only thing a budget can weigh, and the report has
  to say so.** The production Node template declares no `size` on its archive
  steps, so its projected total is 0 bytes with N inputs of unknown size — and
  `max_total` cannot refuse a selection it cannot measure. That is correct and it
  is also a trap: a report that printed `0 bytes` would read as "nothing to
  fetch". So `unknown` is a member of its own, `total_bytes` is documented as a
  lower bound whenever it is non-zero, and the human line says
  "(a lower bound: N input(s) declare no size)".
- **The payload suite could not reuse the frozen Node fixtures, and that was a
  design decision rather than a convenience.** Upstream's `SHASUMS256.txt`
  describes real multi-megabyte release tarballs; a suite that verifies a real
  digest of real bytes therefore cannot use them, and faking the digest would
  have tested nothing. The fixture declares its own template over its own
  four-kilobyte payloads and computes the digest documents from them — so the
  verification is real, the byte counts in the budget assertions are real, and
  the whole suite still opens no socket. The frozen fixtures stay where render
  fidelity is the question.
- **The closed-network proof needed an input that is copied, not extracted.** The
  honest end of payload projection is a client installing from the drop with no
  route to upstream, and a fabricated tar.gz cannot be extracted. The fixture's
  template therefore carries a third installer option built from `sources` +
  `recipe` — the shape `cli_artifact_drops` already uses — so the install really
  runs, and the assertion is that the installed payload is byte-identical to the
  release host's own file. It also exercised the `sources[]` arm of the input
  reader, which the archive options do not.
- **"Nothing is removed without `--yes`" and "nothing is written without `--yes`"
  are different promises, and the weaker one is a lie.** The fact admits a run
  that copies and declines to prune. But a prune plan is read against one reading
  of the target and applied against a possibly newer one — and a run that printed
  a plan and then changed the very tree it described would have printed a document
  about nothing. So an unattended `--prune` without `--yes` refuses with
  `prune_consent_required` and writes nothing at all. `--dry-run --prune` is the
  way to see the plan with a zero exit.
- **`pruned[]` alone cannot say whether the plan happened.** A dry run is marked
  by `dry_run`, but an attended "no" is not a dry run and still renders a report
  carrying the plan. So the array is the **plan** — which is what makes "never
  silent" true for a JSON client, not only for a person — and a separate optional
  `removed` count says how many of its rows went. That member is not in the
  design; it is what the design's own "shown as a plan first" needs in a document.
- **A snapshot's prune belongs to the snapshot writer.** Dropping one version
  means three changes that have to agree: the digest file goes, `snapshot.toml`
  loses its row, and `versions.json` is re-filtered with its `sha256` recomputed
  over the new bytes. Computing that beside the writer would have duplicated the
  record logic that amendments 9 and 14 already paid for, so the stage takes a
  `prune` flag and the plan is built from the stale set it reports.
- **A removal leaves an empty parent, and widening the channel to fix it would
  have been the wrong trade.** `org.other/tool/` survives its last version, and
  the channel refuses any path that is not a version directory or inside
  `_index/` precisely so a batch cannot delete the rest of the tree. An empty
  directory carries no version, no catalog row and no journal fact, and on a
  monorepo it does not exist at all — the harmless side of a refusal worth
  keeping.

## 2026-11-10 — the owner's P7 decisions, and the design that follows from them

The campaign's last blocking question is answered and phase P7 is redesigned
around the answer. Nothing implemented; four documentation commits.

**The owner's answers.** Of the fourteen proposed nvm exclusions, **twelve are
approved** with the reasons the design gave and **two are rejected**:
`nvm install --skip-default-packages` and `$NVM_DIR/default-packages` are not
gaps vibe may declare but a general feature. That answers `#OWNER-QUESTIONS`
Q1 and Q2 and withdraws the risk P7 carried that a rejected `--only=` would
grow the campaign by a phase. Beside it came one binding requirement —
**nothing added for nvm may know anything about npm or Node**, because the next
pass replaces sdkman for Java and must land on the same machinery — and four
rulings: named user aliases stay not-done (a survey of 17 switchers found them
built in to only 5, absent from 6, and absent from the newest tools), the Node
source build stays a backlog item (`B-253`), `[recommends]` extends to
application packages with a general `--no-recommends`, and PROP-065's
multi-step installers extend to ordinary packages.

**The refinement that changed the biggest piece.** The first design answered
"default packages" with a "slot extras" feature: a package action taking a
list, plus a user list per slot. The owner replaced it with a sharper model —
an **ecosystem adapter** — and the replacement is better for a reason worth
recording: it separates three layers that the first design had blurred.
`[recommends]` is **vibe packages**; an author's recommended **ecosystem items**
(corepack, pnpm — npm packages, not vibe packages) is a second layer; and the
**user's own list** is a third. Layers two and three differ only in who wrote
the list, so they share one mechanism from the adapter down, and layer two is
modelled as PROP-065 **features** rather than as a new concept — a recommends
row is shorthand for "a feature that adds an item to that adapter's list", so
`--no-feature`, profiles, the attended multi-select and the recorded chosen set
all come for free.

**Five things the design decided that the decisions left open.**

- **The injection rule became structural rather than advisory.** The owner's
  rule is that item strings are opaque and are passed as separate argv
  elements, never interpolated into a shell string. A `shell` step is an
  authored command *string*, into which a list can only be spliced — so
  `install` and `remove` **must** declare `kind = "recipe"`, and a `shell`
  spelling is a validation refusal. The rule is then enforced by the grammar
  instead of by asking every package author to quote correctly, and the
  phase's own gate is a fixture item reading `; rm -rf $HOME`.
- **A `list` verb answers into a file, not onto stdout.** Vibe must never parse
  an ecosystem's listing format, and stdout carries a package manager's
  notices. So the step writes one item per line into `VIBE_ECOSYSTEM_LIST` and
  vibe trims and reads that, which keeps the step's output what it is — a log.
- **The general "package action" verb is not built, and that is the decision
  the refinement asked for.** Every maintenance act the survey found is one of
  the adapter's four verbs: `--latest-npm` and `install-latest-npm` are
  `self_upgrade`, `--reinstall-packages-from` is `list` then `install`, and
  `corepack` is an *item*. A general verb would be a second spelling for the
  same four acts. One reason to record, because it is not obvious: building it
  on `[[application.migration]]` would have been **wrong**, not merely
  redundant — the post-install offer is `migration_offer`, which looks for the
  version to carry *from* and answers nothing when a slot has no earlier
  version, so a maintenance entry expressed as a migration could never run
  after the **first** install of a runtime, which is exactly when
  `--latest-npm` is asked for.
- **`--no-extras` and a `drop` had to be separated.** The owner's wording says
  `--no-extras` skips the user's own list, and a `drop` is part of that list —
  but a `drop` is a *withdrawal* of consent to an author recommendation, so
  skipping it would make `--no-extras` install **more** than the user asked
  for. A drop is therefore honoured in every case, including under
  `--no-extras`. This is the one place the design did not follow the decision
  literally, and it is flagged rather than buried.
- **The `npm-globals` migration becomes the general case.** An adapter that
  declares both `list` and `install` already *is* the migration strategy, so
  vibe synthesises one `ecosystem:<id>` entry per adapter and the real Node
  package drops its hand-written entry. The id namespace `ecosystem:` is
  reserved so a synthesised entry cannot be shadowed.

**Where decisions 9 and 10 went, and why not into PROP-075.** Both belong to
the general package model, so PROP-075 would have been the wrong authority.

- **PROP-077 is new**: `[recommends]` on application packages, installed
  best-effort by the application installer after the requested package is
  committed, recorded as a **weak** requirer edge on the existing `auto` edge
  so no new removal mechanism is needed; and `--no-recommends` as one global
  flag for every kind. It persists nothing — the flag changes a solve, never a
  replay of a lock, which is why no lock field and no flapping. It also
  records what already ships, measured rather than assumed: the resolver's
  greedy expansion exists, the flag does not exist anywhere, and the
  application plane never reads `[recommends]` at all because `install -g`
  resolves exactly one package and never enters the solver.
- **PROP-065 gains five facts** — and the design that reached them is the part
  a later session would otherwise re-derive. The first instinct was a new
  package-scope step table. That is wrong twice: PROP-054 `#REJ-TWO-GRAMMARS`
  names a second extension grammar as the failure mode, and the mechanism
  already exists — `slot:pre-install` / `slot:post-install` take **many**
  contributions per point, each with its own id and handler, and `[hooks]` is
  already sugar for exactly two of them (`crates/vibe-install/src/lifecycle/plan.rs`
  builds the synthetic `ExtensionDecl`). So an ordinary package's multi-step
  installer *is* that grammar, and what it was missing is not a declaration
  but the **judgment half**: a `title`, conditions over the closed vocabulary,
  a `needs` preflight that refuses before anything starts, and `optional`,
  with a plan, a panel, per-contribution logs and an evidence record. The
  judgment half becomes one shared library; **execution stays separate on
  purpose**, because the application step runner is keyed to an installation
  key, a library and a payload root an ordinary package has none of. The
  `[[installer_options]]` refusal therefore stays, with its reason now written
  down: an installer option is a per-platform alternative that produces a
  payload.

**Facts verified in the code while designing, with their citations, so nobody
re-checks them.** `[[installer_options]]` is refused outside `kind = "app"` at
`crates/vibe-core/src/manifest/document/validation.rs:224-242`, and the step
runner is reachable only from `vibe_app::{install, update, modify}`
(`crates/vibe-app/src/operations/apply.rs:77,116`), so no non-app package can
reach a step today. `from == to` is admitted by the migration machinery
(`crates/vibe-app/src/migrate.rs:125-126,294,300`) and `migration_offer` is the
post-install offer (`:169`), whose "already carried" dedupe is at `:204-209`.
The application install resolves exactly one package through `resolve_and_fetch`
(`crates/vibe-app/src/resolve.rs:374`) — no solver, hence no recommends —
while the project plane's greedy weak expansion is
`crates/vibe-resolver/src/resolvo_engine/mod.rs:86-110,250-279`. There is **no**
`--no-recommends` anywhere in the tree. `UserConfig` is `deny_unknown_fields`
with exactly ten tables (`crates/vibe-core/src/user_config.rs:81-139`) and
`crates/vibe-app/` never reads it, so the user's extras list is the **first**
user-authored per-slot configuration and introduces that read as a new seam.
`PROP-054`'s `slot:` family is implemented (`crates/vibe-core/src/lifecycle/point.rs:150-169`,
`crates/vibe-core/src/manifest/extension.rs:240-253`) although its facts still
read `spec/plan`.

**Format records.** `AV-FMT-025`…`AV-FMT-032` are new, taken from the planning
range because the plan now foresees them, and `AV-FMT-019` is **amended in
place** rather than renumbered because it is the same flag on the same two
commands, now carrying both version-file formats. The application inventory
moves to `vibe-user-applications/8` once, for two phases' additions — the
per-adapter record and the weak requirer edge — so whichever of P7c and P7d
lands first owns the `e7` schema and the `vibe-app-proxy` admission.

**One defect noticed and not acted on**: `BACKLOG.md` carries **two** entries
numbered `B-233` — the owner's environment/meta-distribution investigation and
"an installed application records no source". The ids are supposed to be unique
and one of them needs renumbering; it is not touched here because another
worker holds that file.

## 2026-11-10 — owner accepts the P7 design's recommendations

The owner answered the five discussion points of the P7 design with "согласен с рекомендациями" — the recommended option of each:

1. **Recommendations install by default**, shown in the plan and the report before and after; `--no-recommends` opts out (PROP-077 `#APP-RECOMMENDS-CONSENT`, `#WEAK-OPEN` W1/W2 as designed).
2. **A recommended application is installed exposed** (PROP-077 `#WEAK-OPEN` W3 as designed).
3. **P7e adds a trust policy for steps of ordinary packages**: an `[[extension]]` contribution of a project dependency runs without a question only when the package's group is trusted (the same trusted-groups mechanism agent steps already use); otherwise an attended install asks and an unattended one refuses naming the group and the remedy. To be specified in PROP-065/PROP-054 by P7e before the code.
4. **Adapter `install` / `remove` are `recipe` only**, so ecosystem items always arrive as separate argv elements.
5. **The user's extras list lives in `~/.vibe/config.toml`** (`[slot.<slot>.extras.<ecosystem>]`), accepting the new `vibe-app` → user-config seam.

Implementation of P7a–P7f is authorised from this entry.

## 2026-11-10 — P7a implemented: `keyed` version files and `--write-version-file`

The first of the six P7 units landed. `format = "keyed"` is the third version-file
format — `names`, `key`, and the line syntax from three closed vocabularies
(`separator` ∈ equals/whitespace/colon with no default, `comment` ∈
hash/semicolon/none, `extra` ∈ refuse/first) — and `tool-versions` became its
short spelling. `--write-version-file[=PATH]` writes the resolved version on
`vibe app use` and `vibe install -g`, as one line for `plain` and as an
in-place single-line edit for `keyed`. `PROP-075#VERSION-FILE-KEYED` and
`#VERSION-FILE-WRITE` are `impl/done`; `AV-FMT-025` and `AV-FMT-019` are
delivered; the as-built record is `PROP-075#P7A-AS-BUILT`.

**Four things the implementation settled that the design left open.**

- **The sugar had to be symmetric.** The fact says `tool-versions` expands into
  one `keyed` row before anything reads a file, and says nothing about the way
  out. A one-way expansion would have rewritten every manifest and every
  derived sidecar into the long form on the next write, which is exactly the
  "keeps working byte-for-byte" the risk list named. So the conversion back is
  the same rule read the other way, as a pure function of the row: a `Keyed`
  row that is exactly asdf's shape serialises as `format = "tool-versions"`.
  Nothing remembers which spelling an author used, so the two cannot drift, and
  the no-drift proof is an equality of two parsed manifests rather than a
  comment.
- **serde cannot name a TOML path, so the vocabularies are refused by hand.**
  Measured rather than assumed: an unknown enum variant inside
  `[[application.version_file]]` comes out of `toml` as «unknown variant
  `tabs`, … in `application.version_file` at line 11» — the table and the line,
  never the member. The fact requires the refusal to name its TOML path, so the
  three words and the two fields a `keyed` row cannot omit arrive as text and
  are lowered row by row, each refusal reading
  `application.version_file[0].separator: …`. This is the one place where a
  closed vocabulary in this tree is **not** a serde enum, and the reason is
  worth keeping: the existing `format = "engines"` refusal names its field only
  because the member happens to be called `format`.
- **A keyed write preserves the trailing comment too.** The fact names the
  separator and the spacing. A real `.sdkmanrc` line is
  `maven = 3.9.9  # pinned`, so the writer splits a line into head / values /
  tail and replaces only the values, which keeps the comment, the spacing on
  both sides of it, and the line's own ending. The byte-identity tests compare
  bytes and not a re-parse, because a round trip through a parser would
  normalise the spacing of every line it never meant to touch and a re-parse
  would not notice.
- **The flag refuses wherever it would be ignored.** `--write-version-file` on
  a project install, a remap package, a global MCP package, `--plan-only` or
  `vibe app use --none` is a usage refusal naming the flag, not a flag that
  does nothing. The write runs after the command has settled, so the version
  written is the one the slot now resolves to.

**One premise of the plan's own risk list did not hold, and it is recorded
rather than quietly dropped.** Risk (2) says `tool-versions` already ships and
"the real Node package declares it", so a break would be invisible until a
package is installed. Measured at `240b8abaa`: neither `/fast/git/vibe-pkg-node`
nor any manifest in this tree declares `[[application.version_file]]` at all —
the format ships, no package uses it yet. The compatibility of the short
spelling is therefore proved by tests (`vibe-core`'s no-drift equality, the
sidecar round trip, and the `.tool-versions` cases in `vibe-app` and
`cli_app_env_activation`) rather than observed on a shipped package. Whoever
adds the row to the Node package inherits a proof, not a hope.

**One deviation from the test list, named here.** The plan asks for "each
refusal by TOML path" as a `vibe-core` unit test and that is what landed, with
one message retained deliberately: a `keyed` row's file-name refusals name
`format` rather than `names` when the row is the short spelling's expansion,
because such a row has no `names` array an author could have written. That
keeps every pre-existing `tool-versions` refusal message exactly as it was, and
the two existing refusal tests pass unchanged.

## 2026-11-10 — phase P7e implemented (judgment, trust, B-188); the evidence record deferred

`AV-FMT-031` delivered with three recorded narrowings and one field the
record did not foresee; `AV-FMT-032` **not** delivered and the reason is
written into it. Facts moved in PROP-065: `#ordinary-reach`,
`#ordinary-grammar` and the new `#ordinary-trust` to `impl/done`,
`#ordinary-plan` and `#ordinary-consumer` to `impl/work`, `#ordinary-failure`
left at `spec/plan`. PROP-054 `#SLOT-CONTRIBUTION-JUDGMENT` to `impl/done`.
The spec XML was validated by the PROP-045 projector, as P1's entry records
it must be.

**The owner's decision 3 was specified before the code, then corrected by the
code.** `#ordinary-trust` was written first, claiming that leaving `[hooks]`
alone costs nothing because PROP-020 `#trust-gate` and the new gate are
behaviourally identical. Measuring before implementing showed that is false,
and it is the whole compatibility argument, so the fact was rewritten and the
correction committed on its own. What the code does: on the canonical install
path `[hooks]` is lowered to a synthetic `slot:` script contribution and run
through the lifecycle handler engine, which consults **no** `HookPolicy` and
asks nothing — `crates/vibe-install/src/apply.rs` says so in as many words
(«install itself is consent, so this path has no `HookPolicy` or prompt
surface») and **nothing in `vibe-cli` ever constructs one**. PROP-020's
allow-list, its question and its unattended refusal are reached only by the
legacy `apply` / `apply_with_spec_format` entry points, which no command
uses. So gating `[hooks]` would newly ask, and unattended newly refuse, for
every published package that declares one; they are left exactly as they
behave, and the gate binds only an authored `[[extension]]` row at a `slot:`
point. **No package in this repository declares either**, so the tightening
reaches nobody today. That PROP-020 `#trust-gate` is dead on the path every
command takes is a defect of that gate, recorded in the fact and filed, not
fixed inside this phase.

Facts a later session would otherwise re-derive:

- **The judgment extraction was clean, so the design's fallback was not
  needed** — and it was clean for a reason worth recording: `Condition`,
  `ParamEquals` and `Need` already lived in `vibe-core`, so nothing had to
  move. What was entangled was never the vocabulary, it was `PlanInput`. The
  shared library is `vibe_core::judgment` (the condition evaluator, the needs
  check, the verdict), and every host-dependent answer is a caller-supplied
  closure, so an application's payload root and an ordinary package's
  materialised slot are the same parameter. `vibe-app` keeps exactly three
  things, each genuinely app-only: the installer option a parameter's *type*
  is declared in, the template expansion a `file_present` path goes through
  (done up front, so the typed refusal still reaches the caller), and whether
  an offline run can serve a step's inputs from the cache, which branches on
  step kinds the ordinary plane has none of. The skip reasons are
  byte-identical to the ones the planner printed before, which is what makes
  the extraction invisible to every existing plan.
- **PROP-069 `#PACKAGE-ADMISSION` makes `when = { feature }` unanswerable
  today, and the design did not know it.** `crates/vibe-install/src/configuration_admission.rs:53`
  refuses **any** package manifest that declares `[configuration]`
  (`Error::ConfigurationUnsupported`, asserted by six tests), so a package
  cannot declare a feature and be installed at all. The three configuration
  conditions therefore answer `false` and the plan prints the deciding
  condition. Lifting that refusal is a PROP-069 decision with `impl/done`
  facts and tests behind it, not a P7e edit, which is why it was not taken.
  The evaluator reads the snapshot through a closure, so the day it is lifted
  this plane needs no change. PLAN.md's P7e test «`when = { feature }` reads
  the package-scope configuration» is therefore satisfied as a unit test of
  the library against a non-empty snapshot plus a plane test that the
  condition is false and says so — not as an end-to-end install.
- **One table is one conjunction, so no array spelling was added.** PROP-065
  `#conditions` gives a step `when` as a table *or an array of them*. The
  `[[extension]]` grammar's `when` anchor is a single `toml::Table`
  (`ExtensionWhen`), and admitting an array would have meant a new
  `EqTomlValue`, a changed public accessor and a lossier
  `vibe extensions --json`. Reading **every key of the one table as one
  condition** gives the conjunction the array existed for
  (`when = { command_present = "cargo", offline = false }`) at zero type
  churn. Recorded as a narrowing in `AV-FMT-031`.
- **The four fields had to become one carrier, and that is better than the
  record's four.** `ExtensionDecl` is constructed as a literal in fifteen
  places and destructured exhaustively, without `..`, by the declaration
  fingerprint — so each added field costs fifteen edits and an epoch ruling.
  One optional `judgment` carrier costs one. It also answers the ambiguity
  the record left: a slot row's `when` parses into the carrier and the opaque
  guard stays `None`, so the two meanings of `when` are never both populated.
  The fingerprint classifies the carrier as **excluded**, beside `auto` and
  the selectors — it decides plan membership and presentation, never the work
  delivered — so no epoch moved.
- **The refused-by-name fields had to be *admitted* to be refused.** The wire
  struct is `deny_unknown_fields`, so `requires` / `phase` / `result` were
  already refused — with serde's generic «unknown field, expected one of …».
  Naming them with their reason means adding them to the wire struct and
  rejecting them in the conversion. And that refusal must stay at the TOML
  boundary while **field legality stays semantic**: moving `validate()` into
  `TryFrom` broke
  `semantic_extension_errors_follow_legacy_role_precedence` immediately,
  because a role error must precede a field error. The split is therefore:
  what must be decided to *build* the value (the vocabulary, the refused
  keys) at the boundary; what is a *law about* the value in `validate()`.
- **The trust gate reaches `vibe install` through `ConfirmGate`, which cost
  nothing.** Threading a context from `vibe-cli` through `vibe-orchestrator`
  looked like a dozen signatures. The port that already answers «may this run
  touch the operator's machine» is `ConfirmGate`, which every surface already
  implements and every install path already holds; one defaulted trait method
  (`slot_judgment`) wires the CLI and leaves every other implementor — hosted
  MCP, four test fakes — untouched. The default consents and decides nothing,
  which is correct for a surface with no operator to ask.
- **A resume must not re-ask.** `ResumeRequest` carries the context its
  original run was started with, because a continuation finishes a run that
  was already consented to.
- **`SlotLifecycleSeams` gained the context as a required field**, for the
  same reason that type has no `Default`: a defaulted one consents on the
  operator's behalf. The named `SlotJudgmentContext::consenting()` is what a
  fixture writes, so the choice is visible in the diff — the same device
  `SlotLifecycleSeams::refusing()` already uses for the agent backend.
- **B-188 went into the two packages that own the acts, not into the
  aggregator.** `#ordinary-consumer` says «two `slot:post-install`
  contributions» of `rust-ai-native`; the aggregator is content-minimal by
  PROP-028 and has no place to put a script. The build is declared by
  `rust-ai-native-lang`, which declares the four binaries, and the
  registration by `rust-ai-native-mcp`, which declares the server. One
  install of the family still shows exactly two rows, so the acceptance reads
  the same from outside.
- **What P7e did not deliver, named rather than implied.** The
  `vibe-install-steps/1` evidence directory, the per-contribution log files
  and the attended step *panel* of `#display` on this plane. The outcome rows
  they would serialise already exist in memory as `SlotLifecycleReport`, now
  carrying `skip` with its deciding condition; what is missing is the
  registered format (schema, registry row, generated reader, corpus) and the
  log plumbing. `#ordinary-plan` and `#ordinary-consumer` therefore sit at
  `impl/work`, not `impl/done`, and `AV-FMT-032` stays undelivered with that
  reason written into it.
- **One semantic hole, recorded rather than guessed.** `agent_configured` is
  reported `false` on this plane: the lifecycle agent backend is injected per
  run rather than read from a selection at the CLI boundary, so an
  `agent_configured = true` condition on a `slot:` contribution always skips.
  The seam is the field, not a branch.
- **Disk, on this machine:** `/fast` had 125 GB free throughout; every cargo
  command ran with the mandated `CARGO_TARGET_DIR=/fast/tmp/p45-target`,
  `CARGO_BUILD_JOBS=6` and `--test-threads=6`.

## 2026-11-10 — P7b implemented: ecosystem adapters, their verbs and `vibe app extras`

The second of the six P7 units landed. `[[application.ecosystem]]` declares an
adapter — an opaque `id`, a human `name`, an optional `description`, an
optional `migration` flag, and up to four verbs (`install` required, `list`,
`remove`, `self_upgrade`), each exactly one `StepDecl` restricted to `recipe`
and `shell` — with `install` and `remove` restricted further to `recipe`,
because the items are appended to the script's argv as separate elements and a
`shell` step is a command *string* into which a list could only be spliced.
`[[application.ecosystem.recommends]]` is shorthand for one ordinary installer
feature per row, and an adapter that declares a `list` verb synthesises the
`ecosystem:<id>` migration entry instead of asking the package to write the
same two scripts twice. `vibe app extras list|remove|self-upgrade` are the
explicit verbs that need no user list. `PROP-075#ECOSYSTEM-RECOMMENDS`,
`#ECOSYSTEM-MIGRATION` and `#PACKAGE-ACTIONS` are `impl/done`; `AV-FMT-026`
and `AV-FMT-027` are delivered, `AV-FMT-029` and `AV-FMT-032` partly; the
as-built record is `PROP-075#P7B-AS-BUILT`.

**Five things the implementation settled that the design left open.**

- **The recommends shorthand had to be symmetric, for the same reason P7a's
  `tool-versions` sugar did.** The fact says each row synthesises a feature row
  in every installer option and says nothing about the way out. A one-way
  synthesis would have written those rows into the document the next time a
  manifest was serialised — `vibe init` serialises one — and the next parse
  would then have refused them as colliding with themselves. So the rows go in
  in `TryFrom<ManifestWire> for Manifest` and come out again in
  `TryFrom<Manifest> for ManifestWire`, with one never-authored,
  never-serialised `origin` marker on `FeatureDecl` to tell the two apart. Doing
  it in the wire conversion rather than in a dozen readers is what makes the
  synthesised row *ordinary in every respect*, which was the phase's own risk
  (1): the existing feature-tree suite runs unchanged and passes, and so does
  every other `vibe-app` test.
- **Exactly one pre-existing refusal had to be narrowed.** An option in the
  method form (`installation`, no `steps`) refuses `features`, and a
  recommendation reaches every option — method-form ones included — because it
  is a property of the package and not of one platform's archive. That refusal
  now reads the author's own rows only. An author who writes `features` on a
  method option is refused exactly as before.
- **The reserved-namespace refusal has to be tested before the token shape.**
  `ecosystem:npm` is not a portable lowercase token, so the pre-existing
  refusal would have fired first and never mentioned the namespace the author
  actually tripped over.
- **A synthesised migration entry runs through the ordinary runner with one
  new input, not a second runner.** Its two sides *are* adapter verbs, so the
  step context carries an ecosystem environment beside the migration one: the
  export runs `list` in the old version with `VIBE_ECOSYSTEM_LIST` inside the
  shared work directory, and the import runs `install` in the new one with
  exactly the lines of that file as its argv. The file lives at
  `<work>/logs/ecosystem-<id>/items`, with the reserved `ecosystem:` prefix
  spelled `ecosystem-` in the directory name because a colon is not a portable
  path component.
- **A named adapter refuses, an unnamed one skips.** `--ecosystem ID` naming an
  adapter the package does not declare, or one that declares no such verb, is a
  refusal naming what it does declare; with no `--ecosystem` the same shape is
  reported `skipped`, because only `install` is required of an adapter.

**The injection rule is a test, not a comment.** The phase's own gate is
`cli_app_extras::items_reach_the_recipe_as_exactly_those_argv_elements_and_nothing_is_evaluated`:
four items — `; rm -rf $HOME`, `a"b$(touch $HOME/pwned)c`, one carrying a real
newline, and `@scope/pkg@^1.2` — are handed to `vibe app extras remove` and the
fixture recipe writes its `"$@"` back out with a separator after each element.
The comparison is against the input list, element for element; `$HOME` is
intact, the substitution ran nowhere, and the record carries the four strings
verbatim. The same fixture proves `VIBE_ECOSYSTEM_ITEMS`, the item count, the
absence of `VIBE_OUT` / `VIBE_SOURCES`, that the PATH leads with the configured
installation (`rt` is *that* version's), and that the working directory is the
run's own.

**The inventory generation is NOT moved by this phase, and that is a decision.**
The P7 design note says the application inventory moves to
`vibe-user-applications/8` once, for two phases' additions, and whichever of
P7c and P7d lands first owns the `e7` schema and the `vibe-app-proxy`
admission. P7b was checked against that and needs **no** new state: the
synthesised migration entries go into the `migrations` list the `/7` generation
already carries, and the per-adapter record (`AV-FMT-029`'s last applied and
last observed list) is `#SLOT-EXTRAS` state that P7c owns together with the
final-list arithmetic. So the home still writes the lowest protocol it can,
the proxy admission is still unpaid, and **P7c or P7d owns the bump**. The one
visible consequence is that `vibe app describe`'s new `ecosystems[]` carries
the declaration only; the last applied and last observed lists appear when P7c
lands the record.

**Two notes on the plan's own text.**

- Its acceptance block names `cargo xtask formats check`, which is not a
  subcommand of this tree's xtask; the registry/corpus gate here is
  `cargo xtask wire-diff` (with `check-codegen` for schema freshness), and that
  is what was run. Both new records carry `corpus = "none"`, as the migration
  record does.
- Its file list names `schemas/application_extras.jtd.json` under P7b while
  `AV-FMT-029` assigns `vibe-app-extras/1` to P7c. The format record and the
  half of the document a listing can fill without the user's settings landed
  here, because `vibe app extras list --json` needs an answer; P7c adds the
  user's `add` / `drop` and the record-sourced lists as optional members at the
  same epoch.

## 2026-11-10 — P7e's open items: the evidence record, the failure law, the agent posture

The three items P7e named as not delivered, taken in three commits. `AV-FMT-032`'s
`vibe-install-steps/1` half is delivered (its `vibe app describe` half remains
P7b's); `PROP-065#ordinary-failure` moves to `impl/done`; `#ordinary-plan` and
`#ordinary-consumer` stay `impl/work` behind **one** named residue, which is
smaller than the three they stood behind before. The spec XML was validated by
the PROP-045 projector, which reports `dry-run ir-stable-loss` for `PROP-065`
before and after — the same verdict, unchanged by these edits.

Facts a later session would otherwise re-derive:

- **`agent_configured` had no value to read, and the fix was a selection, not a
  backend.** P7e recorded the field as the constant `false`. The install
  boundary holds an injected `AgentBackend` on every run, so asking *it*
  whether an agent is configured answers «an object exists» — true on a machine
  with no agent at all. The honest answer is the same one
  `AgentHooks::available` gives the application plane, from the same two facts:
  vibe runs inside a hosting agent, or an agent is selected (the external
  registration, else `agents.toml`'s `default`). `agents.toml` is now read
  **once** for both halves of the judgment — the trusted groups and the
  selection — so the two cannot disagree. An unresolvable `default` answers
  false: the question is whether a contribution could be delivered, and the
  refusal belongs to the command that selects an agent.
- **The fixture had to stop inheriting the developer's agent.** An exported
  `VIBE_AGENT_COMMAND` selects an external agent, so the slot-steps installs
  now drop it and `VIBE_AGENT_ARGS` from the child. Without that, the same test
  passes on CI and fails on a machine whose owner uses an agent host.
- **`optional = true` meant nothing once a contribution actually ran.** The
  dispatcher knew the declaration's `optional` at plan time and did not carry it
  into execution, so an optional failure aborted the installation exactly as a
  required one did. The three-way decision is now one pure classifier
  (`lifecycle/failure.rs`) over the three facts the dispatcher holds, tested as
  a table.
- **An interrupt is readable without touching the engine.** `LifecycleRunError`
  has no cancellation variant and the transition statuses have no `interrupted`,
  so "the outcome is recorded `interrupted`" looked like an engine change. It is
  not: the pass owns the cancellation token, so a failure *under a requested
  stop* is an interrupt, and that is both cheap and precise. `optional` may not
  excuse continuing after one — the law says nothing after it runs.
- **The record's start times needed a seam, and only a seam.** The engine
  reports what a contribution DID; when it began is a clock reading, and the
  engine is handed no clock. `SlotLifecycleObserver::started` is defaulted, so
  every existing implementor — the null observer, four fixtures — is untouched,
  and it is called only for a row that will run, because a skip is decided and
  never started.
- **The evidence record is sealed on `Drop`, not on success.** A refused, failed
  or parked install is exactly the one whose rows a reader comes looking for, and
  no engine event marks "the run is over". Dropping the observer does.
- **The log is the one place the facts and the engine still disagree.**
  `#ordinary-plan` wants the running contribution's child output kept off the
  terminal and every byte in its own log. `StreamMode::Inherit` — the attended
  default — hands vibe no bytes at all, and `read_capped` bounds a capture at
  one mebibyte. So a `--json` run's rows carry real logs (asserted end to end:
  `NOISY-OUT` and `NOISY-ERR` both reach `logs/noisy.log`) and an attended run's
  carry none. Changing `stream_mode` to `Capture` for attended runs would have
  delivered the log and **regressed `[hooks]`**: a long contribution's output
  would vanish from the terminal until it finished, because `#display`'s
  `--log`/`--log-tail` fold does not exist on this plane. Both halves are
  recorded as the residue instead of half-done quietly.
- **The panel and the line records are mutually exclusive on purpose.** Where a
  terminal can draw, each running contribution is a panel row; where it cannot,
  the flat line record stays. Emitting both would print every contribution twice
  in exactly the case a person is watching, so the observer emits the line only
  when the progress handle was inert (a task with no id).
- **The vocabulary census is untouched.** The new schema adds no fragment to
  `formats/vocabularies.json`, so `xtask`'s `the_real_vocabulary_home_walks_to_its_sites`
  census is unaffected — it was already red on main at 105 against 104 before
  this work, which is filed as a backlog row rather than silenced here.
- **Disk, on this machine:** `/fast` had over 100 GB free throughout; every
  cargo command ran with `CARGO_TARGET_DIR=/fast/tmp/p45-target`,
  `CARGO_BUILD_JOBS=6` and `--test-threads=6`.

## 2026-11-10 — P7f's harness half: the differential conformance suite, sandbox A complete

The half of P7f that does not need the shim package landed: `conformance/vibenvm/`
plus `cargo xtask vibenvm-conform [--cases GLOB] [--update-inventory]`. Sandbox A
— the pinned nvm-sh v0.41.0 against a local Node release mirror over loopback —
runs for real, and the suite's own meta-tests pass, so the verdict is a measured
one rather than a promise. `PROP-075#VIBENVM-SUITE` and `#VIBENVM-EXCLUSIONS`
move to `impl/work` and carry their as-built notes; the campaign's acceptance
command is `conformance/vibenvm/fetch-mirror.sh` once, then
`cargo xtask vibenvm-conform`.

**The arithmetic, since the gate is arithmetic.** The re-extraction of the pinned
`nvm.sh` and its `nvm --help` yields **76 `(subcommand, flag)` pairs over 23
subcommands**. 24 of them are excluded by the owner's twelve approved items
(`exclusions.toml`, fourteen rows), and the remaining **52 are required and all
52 are covered** by nine cases, every one of which passes sandbox A against
itself. Sandbox B is wired — its own `HOME` and `VIBE_SETTINGS`, the `[[remap]]`
row for `nodejs.org`, `vibe install -g org.vibevm.node/vibenvm` from a local
registry — and reports a **harness error (exit 3) naming the missing package**,
which is the shim half's work and nothing else's.

**Facts a later session would otherwise re-derive.**

- **"No network" needed two guards the design did not foresee, and both are
  nvm's own defaults.** nvm's default category set includes `iojs`, so
  `ls-remote` asks nodejs.org's io.js sibling unless `NVM_IOJS_ORG_MIRROR` is
  set — which would have been a non-loopback `connect(2)` and therefore a
  verdict of 3 on a case that is otherwise fine. And `--latest-npm` /
  `install-latest-npm` reach the npm registry by construction. Both now point
  at empty loopback directories of the mirror, so each gets a deterministic 404
  instead of a forbidden connection. Measured, because the guess would have
  been wrong: `--latest-npm` and `install-latest-npm` do make the request, npm
  reports `E404` against loopback, and nvm then reports `npm upgraded to:` the
  version it already had and **exits 0** — so the case compares what the
  attempt did to the runtime, not whether the internet was reachable.
- **The mirror's indexes are filtered to the pinned version set**, and that is
  what makes `mirror.lock` possible at all: upstream's `index.json` changes with
  every Node release, so its digest could never be pinned, while the filtered
  one is a function of the pinned set alone. The set is `v23.1.0`, `v22.11.0`
  (Jod), `v22.9.0`, `v20.18.0` (Iron) — two in one major, an older major, two
  LTS lines — so `--lts`, `--lts=<name>` and the `node` channel all resolve to
  something the mirror holds. `index.tab` is mirrored beside `index.json`
  because nvm reads the former and vibe the latter; both `tar.xz` and `tar.gz`
  are mirrored because nvm chooses by what the host's `tar` and `xz` support.
- **A case runs in one shell, not one shell per step.** The state a version
  manager exists to change is `PATH`, and a fresh shell per step would have
  thrown away exactly the thing under test.
- **The observer takes the tool's root from the harness, not from `$NVM_DIR`.**
  `nvm unload` unsets its own variables, so an observer reading the shell would
  go blind at precisely the step that matters, and sandbox B — whose settings
  root survives its own unload — would then differ for a reason that is not a
  divergence.
- **The observation record gained one field family, `file:<path>`**, driven by a
  case's `observe_files`. Without it `install --save` could only ever be
  observed as "exited 0": the `.nvmrc` it wrote would not be compared. The case
  now shows `v22.9.0` appearing and `absent` after an `rm`, in both sandboxes.
- **Coverage is derived, never declared.** `coverage.py` reads the command lines
  the cases actually ran, from passing cases only, and attributes flags to the
  tokens directly after the subcommand — which is nvm's own rule for its
  optional arguments and is what keeps a trailing `node -v` out of the
  accounting.
- **Two spelling rules the extraction had to settle.** A flag carries no `=` and
  no value (`--lts` and `--lts=*` are one flag), and a branch's flags are
  recorded against the branch's **first** subcommand spelling, because the alias
  enters the same branch and the flags are parsed once; an alias (`i`, `list`,
  `copy-packages`, `-v`, `help`) therefore gets a bare pair of its own, so the
  gate still demands a case per alias.
- **The owner's twelve approved exclusion items are fourteen rows**, by
  `#VIBENVM-EXCLUSIONS-APPROVED`'s own row rule (1): `install -s` and
  `install -b` are two rows sharing one reason, and the internal-state item is
  recorded under both the `(environment)` and the `(internal)` spelling that
  fact prescribes. Rows carry an added `item` key so the twelve stay countable,
  and `coverage.py` holds the closed row set — a thirteenth gap, a missing
  approval or a quietly reworded row is exit 2.
- **`python3 -m http.server` must be run with `-u`**, or its "Serving HTTP on …
  port N" line sits in a buffer and the harness cannot learn the port it just
  bound.
- **The default-packages case is real, not nominal.** `npm install -g` of a
  dependency-free tarball succeeds with the registry answering 404, so the
  fixture package shows up in `globals` for the version installed with the
  default-packages list and is absent from the one installed with
  `--skip-default-packages` — which is exactly the pair of surfaces the owner
  refused to let the suite exclude.

**Gates.** `cargo fmt --all --check` clean; `cargo clippy -p xtask --all-targets
-- -D warnings` clean; `cargo test -p xtask` 339 passed with **one** failure,
`codegen::optional_shapes::tests::stitch::the_real_vocabulary_home_walks_to_its_sites`
(105 against 104), which is `B-260`, red on main before this work and
deliberately not touched here; `cargo xtask conform check` 138 findings, **0
new**; `conformance/vibenvm/run.sh --self-test` exit 0 with all nine cases and
all six meta-tests; the PROP-075 XML reports `dry-run ir-stable-loss` before and
after the edit — the same verdict, unchanged. `shellcheck` is not installed on
this machine, so the four new shell scripts are unlinted and that is recorded
rather than claimed. Every cargo command ran with
`CARGO_TARGET_DIR=/fast/tmp/p45-target`, `CARGO_BUILD_JOBS=6` and
`--test-threads=6`; `/fast` stayed above 70 GB free.

**What remains for the shim half.** Build `org.vibevm.node/vibenvm`, publish it
into a local registry, and pass `--vibenvm-registry`. Three things to confirm
when it lands, and nothing else is expected to move: that the package publishes
a `vibenvm.sh` under the settings root for the driver to source; that sandbox
B's observation adapter reads the field names vibe's JSON actually uses; and
that the `<TOOLROOT>@v<version>` layout collapse covers vibe's own layout, which
it is written for but has only ever been exercised against nvm's.

## 2026-11-10 — P7d implemented: `[recommends]` for applications and the general `--no-recommends`

The fourth of the six P7 units landed, in three product commits and this
record. An application's `[recommends]` is now installed by the application
installer — breadth-first in declaration order over a visited set, each
installation the ordinary one, each run after the recommender's own record is
committed — and the companion is recorded `auto`, exposed, with a **weak**
requirer edge. `--no-recommends` is one global flag for every package kind.
`[suggests]` gained a voice and no behaviour. `PROP-077`'s facts are
`impl/done` except `#SUGGESTS-UNCHANGED`, which is `impl/work` for exactly one
clause; `AV-FMT-030` is delivered and `AV-FMT-029`'s inventory half is; the
as-built record is `PROP-077#P7D-AS-BUILT`.

**The inventory generation is paid, and P7c inherits it finished.** The P7
design note said whichever of P7c and P7d landed first owns the
`[format.application-index]` 6 → 7 move, the `vibe-user-applications/8` tag and
the `vibe-app-proxy` admission PROP-059 `#proxy-index-upgrade` requires. P7d
landed first and paid all three: `schemas/application/e7/index.jtd.json` with
its corpus, `[format.application-index-v7]` as the frozen previous reader, the
typed `e6 → e7` promotion that fabricates no weak edge, and the proxy's `/8`
arm with its `--supports-index` answer. The generation's only addition is
`weak` on a `requirement` row, so the writer's ladder gained one rung: a home
with no recommendation goes on writing `/7`, `/6` or `/3`, and every existing
PATH proxy goes on reading it. **P7c adds its per-adapter extras record as
optional members of that same `e7` generation at epoch 7 → 7** — no second
generation step, no second proxy admission — and owes one clause in
`Index::requires_reader_eight`, which is written as the ladder's rung for
exactly that reason.

**Five things the implementation settled that the design left open.**

- **`required_by` had to answer weakness.** A weak edge keeps its target out of
  the orphan set and blocks a removal in neither direction, so one boolean
  reverse edge could not serve both readers. `Index::required_by` now answers a
  `ReverseEdge { owner, scope, weak }`, and an owner recording both a
  requirement and a recommendation of one target is not weak — the strong edge
  decides.
- **A satisfied recommendation still records the edge, and the walk stops
  there.** Read literally, "skipped as satisfied" would record nothing; but a
  recommender's `requires` list is rewritten from its plan on every install, so
  a second `install -g` would drop the edge and orphan the companion. The edge
  is therefore recorded for a reference that ends up *present* — and the walk
  goes no further down it, exactly as `--plan-only` goes no further, because the
  plan must be the whole truth about the walk. The same rule means a
  `--no-recommends` the user spent on the companion's own install is not undone
  later by installing something that recommends it.
- **The walk's own publication needs the proxy image.** The weak edge is the
  first thing in a home that needs the `/8` reader, so it goes through
  `proxies::for_publication` + `publish_locked` as a migration's record does.
  The raw `save_locked` refuses a protocol move with existing commands rather
  than stranding them, and that refusal is what the first run measured.
- **The project home's consent gate is honoured by narrowing.** That gate reads
  the lock's app rows and a recommendation is not one, so rather than inventing
  a second gate the walk is not entered on the project-home plane unless the run
  already carries explicit consent. Strictly stronger than the gate: without
  consent, nothing the user did not name is installed there.
- **The flag's path is wider than the plan's file list.** The list named the
  resolver branch as "the only resolver change", which is true of the resolver;
  the posture also travels through `vibe-package-source`, `vibe-install` and
  `vibe-orchestrator` as one decided answer beside the offline posture, and the
  seam gained `solve_without_recommends` with a masked sibling so a visibility
  re-solve cannot bring back what the first solve declined.

**One finding the phase did not act on, and deliberately.** `PROP-077`
`#WEAK-TODAY` recorded, as `impl/done`, that the project plane already honours
`[recommends]` through the resolver's greedy expansion. Measured end to end
rather than at the solver seam, it does not: the recommended package enters the
solved graph and the PROP-050 `#EFFECTIVE-SET` visibility projection then
prunes it again — it is not a root and no manifest declares an edge to it, so
`analyze` finds it unreachable and `prune_graph` drops it before anything is
fetched, locked or materialised. So `[recommends]` has no observable effect on
a project install today, by any route. `#WEAK-TODAY` is corrected in place and
`#P7D-AS-BUILT` (9) states why this phase did not fix it: admitting a weak
graph member into a consumer's effective set is a PROP-050 decision, not a
reach decision, and it wants the same pass as the project plane's own
`suggested[]` report channel. The new `cli_install_no_recommends` suite pins
the measured behaviour so the day it changes is visible.

**Tests.** `vibe-resolver`: `recommends::no_recommends_runs_the_hard_solve_and_nothing_else`
and `…::a_solver_without_weak_expansion_answers_with_its_ordinary_solve`.
`vibe-app`: `index::tests::the_writer_reaches_protocol_eight_only_for_a_weak_requirer_edge`
and `…::a_weak_edge_is_reported_weak_and_an_owner_of_both_edges_is_not`.
`vibe-wire`: the `/7` corpus round-trip and the frozen `e6` reader's refusal of
a document carrying a weak edge, and the report's two new members. `vibe-cli`:
the ten cases of `cli_app_recommends` (installed exposed `auto` with a weak
edge and its idempotence, `--no-recommends`, each skip reason, a failing
recommended install at exit 0, a cycle, transitivity, removal in both
directions, autoremove and the manual mark, the plan's subtree and the
suggestion line, and the plan and the walk stopping at the same place) and the two of `cli_install_no_recommends`, plus the `/8` arm
in `cli_app_proxy_index_versions`.

## 2026-11-10 — one trust mechanism: `org.vibevm.*` by default, and `untrusted_groups` to refuse it

The owner's decision of the day, in two sentences taken verbatim into the
specs: «Доверять по умолчанию всему org.vibevm.*, и сделать этот механизм
конфигурируемым (базовые trusted_groups для vibevm, которые у пользователя
эффективно конкатенируются с содержимым его agents.toml)» and «я бы сделал еще
механизм недоверенных групп в agents.toml, чтобы можно было запретить доверять
встроенному». `AV-FMT-104` records the formats; `PROP-064#trust-entries` is the
new design authority, with `PROP-065#ordinary-trust`,
`PROP-019#SITE-PROFILE-TRUST` and `PROP-020#ALLOW-LIST-SUCCESSOR` amended for
it.

What landed, and the two decisions inside it a later session would otherwise
re-take:

- **The mechanism is `vibe_core::trust`, and it is the only one.** P7e's
  `#ordinary-trust` had to *union* two lists — `agents.toml`'s
  `trusted_groups` and `vibe-workspace`'s `DEFAULT_ALLOWED_GROUPS` — and said
  so as a reluctance, because the sugar must not be more privileged than the
  grammar it lowers into. Making the base configurable made that union
  untenable: two lists with two owners and three consumers would have become
  two *configurable* lists. So the constant in `vibe-workspace` is **deleted**,
  the base list is `BASE_TRUSTED_GROUPS = ["org.vibevm.*"]` in `vibe-core`, and
  `AgentHooks`, `SlotJudgmentContext` and `HookPolicy` all carry a resolved
  `TrustPolicy` instead of a `Vec<String>`. `trusts()` in all three planes is
  now one function call. The pure half (patterns, layers, deny-wins, the
  refusals) reads no file and is tested over values; the one place that reads
  files is `vibe_app::effective_trust`.
- **Deny-wins replaced the opt-out boolean, and that was the owner's second
  sentence.** The planned shape was `inherit_base_trusted_groups = false`. The
  owner asked for an untrusted list instead, which is strictly better and not
  merely different: it is the same grammar as the trusted list, it works per
  group as well as for the whole base (`org.vibevm.ai-native` versus
  `org.vibevm.*`), it is available at the site layer for free, and — stated as
  a law rather than left to the reader — because a deny beats every trusted
  entry of every layer, the answer does not depend on the order the layers are
  read in. The boolean was never committed.
- **A subtree covers its own root.** `org.vibevm.*` matches `org.vibevm`,
  because the decision is about everything vibevm publishes and the core
  packages' own group is not a special case. The boundary is a group label and
  not a string prefix, so `org.vibevmx` is a different organisation — asserted
  in both directions. An **exact** deny entry denies exactly that group, so
  denying a subtree is written `org.vibevm.ai-native.*`; the entry syntax is
  deliberately identical in both lists, and the test says so where a reader
  would otherwise guess.
- **An unreadable entry refuses the run.** A `trusted_groups` /
  `untrusted_groups` entry that is not a group or a subtree is a refusal naming
  the entry, the reason and the fix — not a dropped row. Dropping it would
  either grant trust nobody wrote or withhold trust somebody did, and both are
  silent. This made `ConfirmGate::slot_judgment` fallible (`vibe-orchestrator`),
  which is the one signature this work changed outside the trust files.
- **What did NOT change.** `[hooks]` still runs unasked on the canonical path,
  for the measured reason `#ordinary-trust` already records: that path consults
  no `HookPolicy` at all. So widening the base list to a subtree is invisible to
  every published package that declares a hook, and `B-258` stays open — this
  work closes only the *second* half of its disposition, the unification of the
  two allow-lists, and says so in the backlog entry.

**Tests.** `vibe-core`: five laws in `trust/tests.rs` (the shipped base parses
as an operator could have written it; the subtree and its non-matches;
deny-wins from four directions; the verdict and what a deny overrides; every
refused spelling) plus 28 doctests on the public surface, and one site-profile
case for the `[trust]` table. `vibe-app`: two file-backed cases over
`effective_trust` (the three layers, and a bad entry naming its file).
`vibe-cli`: the two consequence cases the owner asked for —
`a_dependency_under_the_shipped_base_runs_unasked_where_another_group_is_refused`
installs `org.vibevm.ai-native/hooked` **unattended with no `--assume-yes`** and
asserts the contribution ran, and
`an_untrusted_groups_entry_restores_the_question_for_a_group_the_base_trusts`
writes `untrusted_groups = ["org.vibevm.ai-native"]`, asserts the same install
is refused naming the group, `untrusted_groups` and `--assume-yes`, and then
installs `org.vibevm.node/hooked` to show the sibling group is untouched.

## 2026-11-10 — P7c implemented: the user's own extras list, its apply and its editors

The fifth of the six P7 units landed, in four product commits and this
record. `[slot.<slot>.extras.<ecosystem>]` in `~/.vibe/config.toml` carries
`add` (opaque items) and `drop` (recommendation ids); the final list is the
author's enabled recommendations in declaration order, minus every drop,
plus the user's `add` in their own order, de-duplicated by exact string
first-wins; it goes into a slot's **new** version after `install -g`,
`update -g` and `update -g --all`, once, through the P7b runner. The two
decline flags are not the same flag: `--no-extras` skips the user's own
`add`, `--no-recommends` (PROP-077's one global flag, consumed and not
redefined) the author's half, and a `drop` is honoured in every case
including under `--no-extras` — which is the clause the owner's wording
left ambiguous and the design decided. `vibe app extras apply|add|drop|import` complete the
family, and the inventory now keeps, per declared adapter, the last applied
and last observed list with its time and outcome. `PROP-075#SLOT-EXTRAS`
and `#ECOSYSTEM-ADAPTER` are `impl/done`; `AV-FMT-028`, `AV-FMT-029` and
`AV-FMT-032` are delivered; the as-built record is `PROP-075#P7C-AS-BUILT`.

**The seam the phase existed to pay, and the shape that made it cheap.**
The plan called the first read of `UserConfig` from `crates/vibe-app/` the
phase's real architectural cost, and it is paid where the plan said: that
crate now reads and interprets the user's file. But the *path* arrives on
`OperationContext` from the caller, because `vibe-app`'s own standing rule
is that it reads no environment — the chokepoint resolving `$VIBE_SETTINGS`
/ `VIBEVM_USER_CONFIG` stays in `vibe-cli`, beside the PATH and agent facts
it already gathers. That is not tidiness. It makes the property the plan
demanded a test for — "a home with no `[slot]` table behaves exactly as
today" — **structural**: a context with no path reads nothing, so no
command that does not consume extras changed, and no test can reach the
operator's real `config.toml` by forgetting to isolate something.

**Six things the implementation settled that the design left open.**

- **"Placed a new version" is measured as a key the home did not hold.**
  The fact says the apply follows an install that placed a new version and
  not a reinstall of one already installed, and gives no procedure. The
  condition is the set of installation keys captured *before* the
  operation; the request then names *that key* rather than the slot, since
  the slot's default is the user's to move and an install need not have
  moved it — a `--hidden` install of a second version gets its extras in
  its own version.
- **The same call is a warning after an install and an exit status when
  typed.** `vibe app extras apply` is a command the user asked for, so its
  status follows the run. The identical call after an install reports and
  returns nothing: the version stays installed, the slot default stays put,
  the install's status is untouched. A package declaring no adapter at all
  — the great majority of applications — is silent rather than warned
  about, and that one refusal is recognised by its error **code**, never by
  its prose.
- **A `config.toml` that will not parse is a notice and an empty list.**
  The fact makes a stale *line* non-fatal and is silent about an
  unparseable file. "Failure never touches the runtime" decides it: the
  extras list must not be able to fail an install, and the malformed file
  is already reported by the startup read every command performs.
- **The listing answers the same arithmetic under the same flags.** P7b's
  half computed the author's enabled rows only, and the fact gives `list`
  no flags. `list` now takes `--no-extras`, honours `--no-recommends`, and
  its computed list *is* the final list — so a listing is an honest preview
  of the apply rather than a second opinion, and `--no-recommends`'
  admission widened to `app extras list|apply` and to those two verbs
  only: on `remove` and `self-upgrade` it would decide nothing, and a flag
  silently ignored is worse than a flag refused.
- **The editors take `SLOT` first and infer the ecosystem.** The fact
  spells them `add ITEM…` / `drop ID…`, which cannot address a table keyed
  by slot *and* ecosystem; every verb of the family takes `SLOT` first, as
  the other four already did. `--ecosystem` stays optional because the nvm
  case the fact itself names reads badly with it: the user's own file
  decides when it already holds exactly one for that slot (so a second
  edit needs no package tree, registry or network), else the slot's current
  version when it declares exactly one, else a refusal naming what there
  is. Ambiguity is never guessed.
- **A recommendation is routed by its `ecosystem` field here too, and that
  fixed a delivered defect.** `#P7B-AS-BUILT` (3) settled that the field is
  authoritative and that a row naming a *different* declared adapter is
  admitted; the delivered listing nevertheless read
  `EcosystemDecl::recommends` — the rows as authored — so such a row's item
  would have reached the wrong adapter's `install` verb.
  `ApplicationDecl::ecosystem_recommends_of` is now the single reader, and
  the final list and the listing share it.

**The generation was inherited finished, exactly as P7d promised.** The
per-adapter record went in as optional members of the same `e7` generation
at epoch 7 → 7: one clause in `Index::requires_reader_eight`, one arm of
the reader's generation guard, two new corpus rows, no new generation, no
new protocol tag, no second `vibe-app-proxy` admission. A record that
retains **nothing** is neither written nor counted, so the writer and the
predicate agree on one question and an adapter that only ever skipped
cannot push a home onto a reader its PATH proxies may not have. A `list`
run fills the observed half of the same record, a failed observation leaves
the previous list standing and names itself, and `remove` / `self_upgrade`
change what a version carries without being an apply — so they never
rewrite the applied list, and only a fresh `list` can say what is there now.

**Tests.** `vibe-core`: `user_config::slot_tests` — the table parses,
round-trips and keeps the user's order; a home with no `[slot]` table is
unchanged and a mis-spelled member still refuses; the editors append
first-wins; `read_item_lines` is the `$NVM_DIR/default-packages` format;
and a save preserves the comments of a hand-written file. `vibe-app`:
`ecosystem::extras_tests` — the arithmetic as an eight-case table
(including the owner-ambiguous case, that a `drop` is honoured **under
`--no-extras`**), the three stale-line notices, the routing by `ecosystem`
field, what a run remembers per adapter and that a skip remembers nothing,
and `the_users_list_is_read_from_the_file_the_caller_names_and_nowhere_else`
for the seam; `index::tests::the_writer_reaches_protocol_eight_for_an_adapter_record_too_but_not_for_an_empty_one`
for the ladder. `vibe-cli`: the six cases of `cli_app_extras_apply` — the
apply after a new version and not on a reinstall, the two flags apart, a
failing apply at exit 0 with its retry and its record, the three editors
with an `import` of a `#`-commented file, a stale settings line reported
once with `describe` answering off the record, and a home with no `[slot]`
table with a package that declares no adapter.

**Four unrelated files reached the 600-line budget on the way** and were
split along their own seams, which is worth recording because the seams are
real: `user_config/location.rs` (where the file is, against parsing it —
and now the whole ambient surface of that layer, so `conform.toml`'s env
root narrows from `user_config.rs` to it), `index/protocol.rs` gained
`carries` (which protocol holds which fact, which is that module's
subject), `global/observation.rs` (the commands against what one run
observes), and the two test modules.

**What P7f's shim half can rely on.** `[slot.node.extras.npm] add = [...]`
parses and is applied after `vibe install -g` of a new Node version, which
is `$NVM_DIR/default-packages`; `--no-extras` is the decline, which is
`nvm install --skip-default-packages`; `vibe app extras import node
<file>` moves an existing nvm list over in one command; and
`conformance/vibenvm/cases/16-default-packages.toml`'s `setup_b` was
checked against the delivered spelling and needs no change — it writes
exactly `[slot.node.extras.npm]` with `add = ["…"]`. What the suite still
needs from the Node package itself is an `[[application.ecosystem]]`
declaration with id `npm`: no package in the tree declares one yet, so the
`globals` observation has nothing to compare until P7f (or a Node package
change) adds it.

## 2026-11-11 — P7f's shim half: `org.vibevm.node/vibenvm`, and the proof's measured verdict

The campaign's last unit landed, and with it its final proof. `org.vibevm.node/vibenvm`
exists — one sourced `vibenvm.sh` defining `nvm` — `org.vibevm.node/node` gained
at template revision **1.1.0** the npm ecosystem adapter and the two `plain`
version files the shim needs, and the differential suite now runs sandbox B for
real. **The verdict is 0, measured, over nine cases and all 52 required
`(subcommand, flag)` pairs.** Every fact of PROP-075 section 5 is `impl/done`;
what the shim needed and what it found is `#VIBENVM-MAPPING`'s own table plus
`#P7F-AS-BUILT`.

**The verbatim run** (`cargo xtask vibenvm-conform`, with
`VIBENVM_REGISTRY` and `VIBE_BIN` set as the suite's README says):

```text
mirror:     17 files match mirror.lock; nvm-sh v0.41.0 pinned at 43d410caa64ba7928cef750e31fccbf5790c210c
mirror:     served over loopback at http://127.0.0.1:45335
sandboxes:  a vs b
cases:      9 matching *.toml
pass: help-and-version (6 steps)
pass: cache-and-unload (4 steps)
pass: install-use-uninstall (16 steps)
pass: dispatch-aliases (7 steps)
pass: lts-and-remote (14 steps)
pass: install-options (7 steps)
pass: reinstall-packages-from (3 steps)
pass: version-file (9 steps)
pass: default-packages (3 steps)
inventory:  76 pairs from nvm v0.41.0
excluded:   24 pairs, 14 owner-approved rows
            (3 recorded, not required)
cases:      9 passed, 0 not passed
covered:    52 of 52 required pairs
coverage:   complete
verdict:    0 (9 case(s) passed, coverage complete)
```

**The one thing a later session would most want to know.** nvm's `22.9.0` is
*one release*; vibe's `@22.9.0` is a *requirement*. Measured against the
suite's pinned version set, `vibe install -g org.vibevm.node/node@22.9.0`
installed **22.11.0**, and `vibe app exec node@22.9.0 -- node -v` ran 22.11.0.
So every version a target names — in the shim and in the suite's own observer
— is spelled `=<version>`, and nvm's prefix match becomes a requirement by
counting the fields the user typed: `22.9.0` → `=22.9.0`, `22.9` → `~22.9`,
`22` → `22`, `node`/`stable` → `latest`, `lts` and `lts/*` → `lts`,
`lts/jod` → `lts/jod`. That table is the only translation the shim carries;
everything else a user types is handed to vibe as written, so `^22` and
`>=20 <23` work too.

**What the shim is, and what makes "it holds no version logic" checkable.**
One file, one public function `nvm`, helpers in the reserved `__vibenvm_`
namespace, nothing written and nothing cached. Every version is resolved by
`vibe app env --use` (installed) or `vibe install -g … --plan-only --json`
(available), every install is `vibe install -g`, every activation is
`vibe app env`, every listing is `vibe app versions`, and what a `.nvmrc` says
is read by `vibe app env --project --json` and never by the shim. `nvm unload`
unsets the function, its helpers and its three variables. Three declared
documents answered what it needed: `vibe-app-env/1`'s `version_file` selection
carries `requested` whether or not that version is installed (a bare
`nvm install`), `vibe-app-plan/1`'s `package.version` resolves a requirement
against everything the registries and the template offer while downloading
nothing (`nvm version-remote`), and `vibe-app-list/1` is the home's own
inventory, needing no registry and no request.

**Four shortfalls of existing features, reported and not patched** — the
VIBENVM rule is that a missing capability is a vibe feature to report, never
something to implement inside the shim. They are `B-262`…`B-265`:
`vibe install -g` of an application with no version named does **not** consult
the declared version file, though this document's own precedence puts it
there, so the shim composes two commands; `vibe app versions` has no
version-pattern filter, so `nvm ls <pattern>` answers with the one version a
requirement resolves to rather than every match; `--write-version-file` lives
only on commands that change state, so `nvm use --save` — which in nvm changes
the shell and the file and *not* the global default — moves the slot default
with it; and a version file's bare `22.9.0` is read as a caret requirement, so
a `.nvmrc` that `--write-version-file` has just written stops pinning the
version it recorded as soon as a newer minor is installed. The last two the
suite cannot catch, and the backlog rows say why.

**Two decisions in the Node package that a reader would otherwise reverse-engineer.**
(1) The adapter's `list` leaves out `npm` and `corepack`: both live inside
every Node distribution's own `lib/node_modules`, so they are the runtime's
managers and not packages anybody added, and carrying them into another
version would overwrite that version's own tested pair. This was found by a
mismatch — with corepack in the list, `vibe app migrate … --only ecosystem:npm`
carried a bundled manager, failed against the loopback npm registry and exited
1, while nvm (whose own list excludes npm and predates corepack) swallowed the
identical failure and exited 0. (2) `self_upgrade` asks for the newest npm
*this* node line can run, from upstream's own ceilings rather than a bare
`npm@latest` that cannot start on an older runtime, and its exit status answers
**"does this installation have a usable package manager"** and not "was the
network up": it succeeds with a notice when the attempt could not be made and
npm still runs, and fails when npm does not. Measured: npm reports `E404`
against the loopback registry and both tools exit 0 having changed nothing.

**The harness half's three open confirmations: one held, two did not.** The
installed package does publish a `vibenvm.sh` to source, and `run.sh` now
takes it from under `<settings>/opt/` rather than by name anywhere under the
settings root — the package cache holds a copy of the same bytes, and sourcing
*that* would let a failed installation pass. The `<TOOLROOT>@v<version>`
collapse did **not** cover vibe's layout: it required the version to be a path
component (`versions/node/v22.9.0`) while vibe's is the version half of a
coordinate (`apps/node/org.vibevm.node.node@22.9.0`), so the rule now accepts
either separator. And sandbox B's readers named fields vibe does not emit —
`vibe app env --json` carries no `default`, and `app versions --json` nests its
versions under `packages[]` — so they were rewritten against the documents vibe
actually emits.

**Three comparisons are now of resolved answers rather than of each tool's
storage, and one case names nvm's `node` flavour.** `default` follows nvm's
alias chain exactly as `nvm_resolve_alias` does and reports `absent` when what
it lands on is not installed, because nvm's `default` survives the uninstall of
the version it names and a default naming a version that is gone provides
nothing to a new shell. `aliases` reports what each name resolves to, with
vibe's `lts` under nvm's spelling `lts/*`, and asks for the channels
**offline** — so the observation makes no request of its own and reports only
the names the tool has already fetched, which is exactly what nvm's alias store
is; before any remote call both tools answer `none`. A second normalisation
drops a leading `v` from a version that starts a field or a member of one,
because nvm writes `v22.9.0` where vibe writes `22.9.0` and nvm's own
`nvm_version` adds and strips it freely. And `cases/12`'s two whole-list
`ls-remote` spellings now name nvm's own `node` flavour: a bare
`nvm ls-remote` lists "node AND io.js" and returns the io.js half's status,
which against the suite's empty loopback io.js mirror is **3** — so a required
pair's exit status would have been decided entirely by nvm's io.js category,
an owner-approved accepted gap that vibe has no concept of at all. The case
says so in its own summary, and the pair is still covered because the coverage
gate reads flags only from the tokens directly after the subcommand.

**How sandbox B is wired, and why none of it is a test-only flag.** Its own
`HOME` and `VIBE_SETTINGS`; the `[[remap]]` row the harness half already had;
a `[[registry]]` row declaring the local registry as that home's **own**
(`type = "localdir"`, a `file://` url), because the shim calls
`vibe install -g org.vibevm.node/node@…` exactly as a user's shell would and
must not carry a flag a user would not; and `vibe` on PATH in a directory of
that sandbox, deliberately **outside** the tool root so it never appears in the
`owned_path` observation. The two packages live in their own repositories, so
publishing them into that registry is a step of the acceptance rather than a
fixture of the suite: `conformance/vibenvm/make-registry.sh` does it, and the
suite's README gives the five commands end to end.

**Gates.** `cargo xtask vibenvm-conform` → `verdict=0` (above);
`conformance/vibenvm/run.sh --self-test` → 0, all nine cases A-against-A and
all six fail-fixtures. **No Rust changed in this unit** — the work is two
package repositories, four shell/TOML files of the suite and the documents —
so the Rust gate set does not apply and was not run beyond the build of
`vibe-cli` the suite needs; `cargo xtask conform check` was run for the
repository's own sake and reports 138 findings, 0 new. The PROP-075 XML reports
`dry-run ir-stable-loss` before and after the edit, the same verdict unchanged.
`shellcheck` is still not installed on this machine, so the shim and the four
new shell files are syntax-checked with `sh -n` and `bash -n` and otherwise
unlinted, which is recorded rather than claimed. Every cargo command ran with
`CARGO_TARGET_DIR=/fast/tmp/sp-target`, `CARGO_BUILD_JOBS=6`,
`CARGO_INCREMENTAL=0`; `/fast` stayed above 36 GB free, with the suite's kept
work directories (3–4 GB each) deleted as they were read.

**One obligation of the plan is deliberately not done here, and is not in this
unit's packet:** PLAN §7's whole-campaign `campaigns/app-versions-2026-11/accept.sh`
— the nine-leg end-to-end script across P1–P7 — is assigned to P7f by the plan
and was not requested of this unit, which was the shim plus the proof. It
remains unwritten, and the campaign's close-out owes it.

## 2026-11-10 — `[recommends]` works on the project plane: the owner's «Вариант 1», built

The one fork P7d left for the owner is answered and built. P7d measured that a
project dependency's recommendation entered the solved graph and the PROP-050
`#EFFECTIVE-SET` projection then pruned it again — not a root, no manifest edge
to it, so `analyze` found it unreachable and `prune_graph` dropped it before
anything was fetched, locked or materialised. The owner's word on the fork:
**«Вариант 1 делай»** — a package recommended by a project dependency is
installed and **visible to its recommender as if it were that package's
ordinary dependency**, best-effort, with `--no-recommends` as the decline. The
decision is PROP-050 `#WEAK-EDGES-ARE-ORDINARY-EDGES`, the plane's build is
PROP-077 `#PROJECT-RECOMMENDS` / `#PROJECT-RECOMMENDS-REPORT`, and the formats
are `AV-FMT-105`. `#SUGGESTS-UNCHANGED` is `impl/done` with them, and
`#WEAK-TODAY`'s two gaps — the missing flag and the unread plane — are both
shut.

**One edge, not one exception.** The whole fix is that an accepted
`[recommends]` reference now *is* an edge — recommender → recommended, carrying
`weak` — and the effective-set walk reads it with the same three rules and the
same defaults (`access = "public"`, no friendship) it reads a declared edge
with. Nothing downstream of visibility had to learn the word: `exclude` through
the declaring edge kills it, `override` reshapes it, the seal still gates
friendship, `vibe why` explains it, the closure diff prints it. The alternative
— an exception in `prune_graph` for «graph members nobody declares» — would have
put a second notion of membership beside `E(R)` and left every consumer control
unable to touch it.

**What the implementation decided that the design did not say.**

- **Acceptance is read off the world, not carried from the solver.** The
  projection turns a `[recommends]` reference into an edge exactly when the
  recommended coordinate is a member of the graph being analysed. That is one
  rule for both solve paths (the source-aware metadata closure and the plain
  `solve_visible_graph` of `vibe update <pkg>` / `--sources-only`), it needs no
  new solver output, and it is automatically right for `--no-recommends`: the
  expansion never ran, so there is no member and no edge. The same rule serves
  the installed-world reader (`vibe why`), where the lock's members are the
  world — so `vibe why` and the lock cannot disagree about a companion.
- **Weakness needs a flag on the arrival, and the strong edge wins.** A
  breadth-first walk can reach a coordinate weakly before it reaches it
  strongly, so `Provenance.weak` is not «the last edge was weak» but «nothing
  but a recommendation admits this», and a strong chain replaces a weak witness
  whatever order the declarations are read in. The walk state carries the flag
  (and keys on it, so both routes stay alive). A member reached only *through* a
  weak hop is itself weak, which is the honest answer for what a recommendation
  brought in.
- **No lock field.** The lock already records the admitting rule; `admitted_by`
  gains the value `weak-chain` beside `root-edge` / `public-chain` /
  `friends-chain`, and that is the whole lock change — `LockedPackage` is
  `deny_unknown_fields` with 38 literal constructions in the tree, so a new
  field would have been a wide, forward-incompatible change to record one bit
  the existing member already has a place for.
- **Replay keeps it; only a solve reconsiders it.** The lock is the record of
  what was installed, so a locked weak member is materialised again by a bare
  `vibe install` and by `vibe reinstall` whichever way the flag is passed — and
  the freshness fast path never sees it, because it compares declared roots with
  `meta.root_dependencies` and then checks slot presence. An explicit
  `vibe install <pkg> --no-recommends` recomputes the whole effective set and
  the declined member leaves the lock with its slot pruned (measured). A bare
  `vibe update` is a **scoped** update over every declared root: it moves what
  it re-resolves and removes no member outside it, so it admits the flag without
  reconsidering a locked companion. That asymmetry is recorded in
  `#PROJECT-RECOMMENDS` rather than changed.
- **The report channel P7d deferred is built, and both levels share it.**
  `[format.cli-install-report]` gains `recommended[]` (`package` · `by` ·
  `outcome` ∈ `installed | satisfied | skipped` · optional `reason`) and
  `suggested[]` (`package` · `by`), filled from the finished metadata closure
  and carried through `PlannedInstall` → `InstallRun` → the draft. A skip is
  also a named warning on stderr with its reason, through a new
  `PlanEvent::RecommendationSkipped`. A replay carries neither member, because
  it resolved nothing and reached no weak table — a report assembled from the
  lock would be inventing the account of a solve that did not run.

**Open, deliberately.** A *project's own* `[recommends]` table is still not
expanded: the solver's greedy expansion walks the selected packages' manifests,
and the consumer root is not one of them. The projection would admit such an
edge the moment the solver produced the member, so this is a resolver-side
addition, not a visibility one. Filed as the one thing this pass found and did
not do.

**Tests.** `vibe-core`: `visibility::tests::a_weak_edge_admits_its_target_like_an_ordinary_one`,
`…::a_weak_edge_grants_no_friendship`,
`…::a_strong_chain_beats_a_weak_one_in_either_declaration_order`,
`…::a_weak_target_is_excludable_through_the_declaring_edge`, plus the
`append_recommended_edges` doctests. `vibe-wire`:
`the_weak_members_are_rows_with_closed_outcomes_and_vanish_when_empty`.
`vibe-cli` (`cli_install_no_recommends.rs`, rewritten from the suite that
pinned the broken behaviour):
`a_recommended_package_is_installed_visible_to_its_recommender_and_never_a_root`
(lock row with `admitted_by = "weak-chain"`, materialised slot, the report
lines, the untouched `[requires]`, the root set, and `vibe why` naming the
chain), `an_unsatisfiable_or_conflicting_recommendation_warns_and_is_skipped`
(both best-effort shapes: a requirement nothing satisfies, and a companion
declaring `[conflicts]` on its own recommender),
`the_flag_declines_the_whole_expansion_and_says_so`,
`a_locked_weak_member_is_replayed_and_only_a_re_solve_reconsiders_it`,
`the_json_report_carries_a_row_per_recommendation_and_suggestion`, and the
retained flag-refusal case.

## 2026-11-11 — the campaign's close-out: the four shortfalls as one rule, and §7's acceptance script

The last two debts of the campaign are paid. `B-262`…`B-265` — the four
shortfalls of existing features P7f's shim found and, by the VIBENVM rule, did
not patch — are closed, and PLAN §7's whole-campaign
`campaigns/app-versions-2026-11/accept.sh` exists and passes **all thirteen
legs**. The shim is simpler for it, and the differential verdict is **0**
again. `PROP-075#VERSION-FILE-VALUE` is new, `#VERSION-FILE-FORMATS`,
`#VERSION-FILE-WRITE` and `#VERSIONS-LIST-OUTPUT` are amended,
`#P7-CLOSE-AS-BUILT` records what the code decided, and the whole of it is
`AV-FMT-106`.

**Four rows, one rule, and that is the finding.** They read as four gaps and
are one question: *what does a spelling that names versions mean, and where*.
A **version file records a choice**; a **command line asks vibe to choose**.
Once that is stated — a complete version in a version file selects exactly
that version, a partial one selects the newest version beginning with it, a
channel keeps its channel meaning, every other spelling keeps its requirement
meaning — B-265 **is** the rule, B-262 is one more reader of it
(`vibe install -g` with no version), B-263 is one more writer of the same
spelling (`vibe app versions [PATTERN]`), and B-264 is the flag that closes
the round trip (`vibe app env --write-version-file`, which writes the resolved
version that now reads back as itself). Taken one row at a time each would
have grown its own matcher; taken together the implementation is one function
that counts components — three → `=X.Y.Z`, two → `~X.Y`, one → `^X` — called
by the version-file search, by the resolution cache's re-read of the authored
value, and by the listing's pattern. That shared call is why a cache hit and a
cold read cannot disagree, and why a listing cannot narrow by a different rule
than an activation selects by.

**The CLI spelling is deliberately unchanged, and that is the boundary the
owner's design drew.** `SLOT@VER`, `PKG@VER` and `--use SLOT=VER` stay
requirements: `vibe install -g org.vibevm.node/node@22.9.0` still resolves
22.9.0 as `^22.9.0`, `@=22.9.0` is still the pin, and nothing recorded in a
manifest, a lock or an index changes meaning. The shim therefore keeps the
`=<version>` translation of `#P7F-AS-BUILT` (1) for every *target* it builds,
and a test asserts both halves side by side: a `.nvmrc` holding `1.0.0` selects
1.0.0 with 1.1.0 installed, and `--use alpha=1.0.0` selects 1.1.0.

**Three facts the implementation settled that the rows had left open.**

- **B-262's chicken-and-egg was a false problem, and it is worth saying why.**
  The row feared that a package's declared version files are unknown until the
  package is resolved, and that resolving needs a version. They are not
  unknown: the home's `vibe-app-version-files/1` sidecar records what every
  **installed** application declares, which is the same document an activation
  reads. So the install consults a file exactly when this home already has a
  provider of that package's slot — which is exactly when "what does this
  project want" has an answer — and a first install of a package the home
  never held resolves as it always did. Nothing resolves twice, and no new
  document was needed.
- **B-264's open decision, taken: exactly one `--use`.** `vibe app env` can
  resolve several slots in one call, so the flag names the slot by naming the
  version. No `--use`, several, `--unset`, `SLOT=none`, and a `--use` that
  resolved to nothing installed are five usage refusals naming the flag. The
  written path goes to **stderr**, because `app env`'s stdout is the only
  thing a shell evaluates, and `--json` carries it as the one new optional
  member of `vibe-app-env/1`.
- **B-263 became a positional, not a flag, and it absorbed the `@VER` the
  target spelling already parsed.** `vibe app versions node 22` is nvm's
  `ls 22`; `vibe app versions 'org.vibevm.node/node@22'` — accepted and then
  silently ignored, which was half of the row's own complaint — now means the
  same thing. A spelling that is neither a version pattern nor a channel is the
  new refusal `bad_version_pattern`, before any version is listed.

**The shim got smaller in three places, and that is the check that these are
the right features.** `nvm install` with no version is now one
`vibe install -g` with no version at all — the shim no longer reads the file's
value through `app env --project` and re-translates it, which was version logic
applied to a file vibe had already read. `nvm use --save` is one
`vibe app env --use … --write-version-file`, so it no longer moves the slot
default as a side effect nvm does not have. And `nvm ls <pattern>` /
`ls-remote <pattern>` hand the spelling over as the user typed it and list
**every** match, where each used to answer with the one version a requirement
resolved to. `org.vibevm.node/vibenvm` is revision **1.1.0** for it. The one
thing left in the shim that touches versions is the `=<version>` target
translation, and nvm's "nothing matched is exit 3" status mapping.

**§7's acceptance, and how its own count resolves.** PLAN §7's scenario is nine
legs — install at a channel; the listing; `vibe app env`; the prompt hook;
`vibe app exec`; `vibe app migrate`; the mirror from `mirror.toml`; the install
from that snapshot with the network denied; the differential proof — and the
owner's decisions of 2026-11-10 added four, one per feature a user can see: a
`keyed` file written and read back for two slots, the user's extras list
reaching a new version, a recommendation declined with `--no-recommends`, and
an ordinary package's judged `slot:post-install` contribution with its unmet
`needs`. Thirteen legs, all passing. It is isolated the way
`campaigns/sys-prompt-2026-11/accept.sh` is — one re-exec into
`bwrap --unshare-net`, every `vibe` call under `strace -e trace=connect`, any
non-loopback `connect(2)` failing the run — and it reaches **no** upstream
host: Node's release files come from the one documented mirror this repository
pins, served over loopback and redirected by a `[[remap]]` row, which is the
mechanism a closed network uses anyway. The two packages that live in their own
repositories come from `conformance/vibenvm/make-registry.sh`, so both it and
`fetch-mirror.sh` are prerequisites rather than parts of the script, named in
the exit-2 message when one is missing.

**Four things writing that script measured, each recorded rather than
asserted.**

- **"Install from the snapshot with the network denied" needs the bytes, not
  only the manifest.** The first attempt mirrored with `payloads = false` and
  the offline install failed naming the archive it could not fetch — correctly.
  The leg now sets `payloads = true` with `[payloads] into`, and the closed
  home reads them through an `[artifacts] packages` drop; the install's own
  plan then says «1 input(s) from the [artifacts] packages drop», which is the
  proof that `#MIRROR-PAYLOAD-PROJECTION` and PROP-066 `#drops` meet rather
  than two facts that happen to agree on paper.
- **§7 asks for the ordinary package's contributions in `--plan-only`, and
  that surface does not exist.** Measured: the project plane's `--plan-only`
  document carries `packages`, `preview_complete`, `remaps` and `hostings`, and
  no contribution rows — because PROP-065 `#ordinary-plan` is `impl/work` and
  `AV-FMT-032` was deliberately not delivered by P7e. The leg therefore reads
  the row where this build reports it, the install's own `contributions` array
  with its point, handler and outcome, and says so in its own comment. A leg
  that asserted the plan preview would have been a false green.
- **The migration leg needs an npm registry that can actually serve the
  package it carries.** nvm's own `--reinstall-packages-from` case in the
  differential suite passes with nothing carried, because a carry reinstalls
  **by name** and the pinned mirror's `npm-registry/` is deliberately empty
  and digest-locked. So the acceptance serves a second loopback registry of its
  own holding one dependency-free package with its sha1 packument, and the
  `globals` the adapter's own `list` verb observes after the carry is what the
  leg compares. Nothing is written into the pinned mirror, which could not then
  be verified again.
- **A `vibe app` verb run inside a project resolves that project's home.** The
  first draft never changed directory, so every leg ran in the vibevm checkout
  and `app extras add node` answered `unknown_slot` for a slot the user home
  plainly had — and the `--write-version-file=.sdkmanrc` of leg 10 wrote into
  the repository root. Each leg now names its own working directory, and the
  legs about the user's home run in a directory that is deliberately not a
  project. Stdout and stderr are kept apart for the same class of reason: a
  mutation prints its plan on stderr so `--json` keeps one envelope on stdout,
  and the first draft merged them and could not read its own answers.

**Gates.** `cargo fmt --all --check` clean; `cargo clippy --workspace
--all-targets -D warnings` clean, and the same for `vibe-app` / `vibe-cli` on
`x86_64-pc-windows-gnu`; the full `vibe-app` and `vibe-cli` suites pass, with
the new tests named in the report; `cargo xtask conform check` 138 findings, 0
new; `cargo xtask check-codegen` clean after the schema edit; the built binary's
`vibe check` 0 new; the PROP-075 XML reports `dry-run ir-stable-loss` before and
after the edit, the same verdict unchanged; `cargo xtask vibenvm-conform` →
`verdict=0` and `conformance/vibenvm/run.sh --self-test` → 0 against the
simplified shim; `campaigns/app-versions-2026-11/accept.sh` → 13 legs passed, 0
failed. `shellcheck` is still not installed on this machine, so `accept.sh` and
the shim are syntax-checked with `bash -n` / `sh -n` and otherwise unlinted,
which is recorded rather than claimed.

## 2026-11-11 — the corporate proof's three defects: one vibe bug, two shim bugs, and what the suite cannot see

The corporate-network proof exercised the shipped surface against
`~/corp/admin/vibevm-repo` with the network denied and found **three** defects
that nine passing differential cases, a 13-leg acceptance script and a green
`vibe check` had all missed. Two are the shim's and one is vibe's. They are
fixed before the 1.0.12 build: the shim is `org.vibevm.node/vibenvm@1.1.2`,
`PROP-075` gains `#VIBENVM-ARGUMENT-ORDER`, `#VIBENVM-BUILT-IN-ALIASES` and
`#VERSIONS-CHANNEL-DISCOVERY` with `#CORP-PROOF-FIX-AS-BUILT` recording what
closing them decided, and `BACKLOG.md#b-266`…`#b-269` carry the rows.

**1. `nvm use 22 --save` exited 0 and wrote nothing** (`B-266`). One
`*) break` per option loop: the version stopped the scan and every flag behind
it was discarded — silently, with a success status, on the one command whose
whole observable effect is the file it writes. The fix is not a bigger loop,
it is the discipline stated: nvm reads a **different argument list per
branch**, and the shim now takes each branch's rule from the pinned `nvm.sh`
v0.41.0 rather than imposing a uniform one. `install` reads two passes and
answers **exit 6** for a version-selecting flag written after the version,
which is nvm's own answer and not a dropped flag; `use` and `which` scan the
whole list with the last version winning; `ls`, `ls-remote`, `version` and
`version-remote` scan it with the first pattern winning; `run` and `exec`
stop at the version because the rest is the command's own argv. The audit that
rule demanded found three more divergences in the same loops, all closed with
it: `--reinstall-packages-from <v>` was accepted where nvm refuses the space
spelling, a repeated `--save` was last-one-wins where nvm exits 6, and an
unknown long option was ignored where `ls`, `ls-remote`, `version-remote` and
`exec` refuse it.

**2. `nvm use default` exited 3** (`B-267`), with the slot default set, and so
did `which` / `run` / `exec` / `version` for the same word. `#VIBENVM-MAPPING`
had already decided that `default` **is** the slot default while named user
aliases stay deferred; the shim honoured three of the four built-ins and
handed `default` to vibe as a requirement, where it reads as a channel nothing
declares. It is now one more spelling of the shim's single translation table —
`=<the slot's default version>`, read out of vibe's own
`vibe app versions <slot> --installed --json`, which costs no registry and no
request — so it works wherever a version does and nothing downstream knows it
was ever a word. An `lts/` **pattern** became an LTS selection in the same
place, as it is for nvm, so `nvm ls-remote lts/jod` and `--lts=jod` are one
question again.

**3. `nvm ls-remote --lts` answered `N/A`, and that one was not the shim**
(`B-268`). `vibe app versions <coordinate> --channel lts` refused with
`unknown_channel … channels: (none)` while `vibe app versions node --installed
--all` showed `[lts, lts/krypton]` in the same home. The asymmetry was never
about the argument: `--installed` answers from the template **cache**, which
an install had primed, while every other path enumerates the directory roots
of the configured chain — and that enumeration knew only `localdir`, though
its own doc comment named the `monorepo` working copy. In the corporate shape
the monorepo is the *only* registry, so a closed network had **no directory
root at all**: no template was discovered, the listing fell back to the
published version union, and the answer was a package that declares no
channels. The versions were right, which is exactly what made it survive
every gate. Which sources have a directory root is now the registry layer's
own decision, taken per variant beside the variants
(`RegistrySource::directory_root`), so a kind of registry cannot be left out
of it again.

**The second defect was hidden inside the third, and it is the one worth
remembering.** Adding the monorepo root did not fix anything: the symptom was
byte-identical afterwards, because a monorepo root is a `tempfile::TempDir`
capture the registry **owns**, and the enumeration was handing back a path
that the drop of its own resolver deleted before the caller read it. A
read-only enumeration that returns paths into something it opened must return
the lease with them — the retention `LayoutRoot` already has for the same
reason (PROP-073 `#NATIVE-PREPARATION`) — so the roots now travel as one value
with the resolver that captured them. The general lesson is that one, not a
detail of monorepos; and the reason it took a second measurement to see is
that the right version list hides both halves of this defect equally well.

**What the suite cannot see, said out loud.** Two of the three were missable
and one was unreachable, and the distinction matters for what was added. The
flag-after-the-version bug was **missable**: not one of nine cases asked for a
flag on that side of the version, and
`conformance/vibenvm/cases/17-flags-after-the-version.toml` now asks every
wish in both positions with the version file in the observation record. The
`default` bug was **missable** the same way: the cases reached the slot default
only through `install --default`, never by naming it, and
`cases/18-default-and-channels.toml` now asks each subcommand for it by name.
The monorepo bug is **structurally outside the suite's reach**: sandbox B
publishes into a `localdir` registry, so the registry type the corporate shape
uses cannot appear there at all. Closing that gap by changing the suite would
have meant teaching the harness a second registry type for one case; it is
closed where it can be measured instead —
`crates/vibe-cli/tests/cli_app_versions_monorepo.rs` drives the real binary
over a real local `monorepo` with **no** `--registry` on the command line, two
unit tests cover the root decision for all three source variants, and the
corporate proof re-runs the three commands itself.

**Evidence, before and after, against the proof's own home.** Before:
`vibe app versions org.vibevm.node/node --channel lts` → `✗ unknown_channel
… channels: (none)`, and the unfiltered listing's 406 rows all read
`(published)` with no provenance line. After: `177 version(s)`, newest
`26.13.0 (template) [lts, lts/lithium]`, `24.22.2 (published (overrides
template)) [lts, lts/krypton] — installed`, and a trailer that finally names
the slot's default **and** the mirror snapshot it read the domain from
(`mirror snapshot synced 2026-11-11T01:32:28Z`) — byte-for-byte the answer the
same home gives when the directory root is passed as `--registry`, which is
the invariant the fix is about.
