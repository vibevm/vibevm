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
