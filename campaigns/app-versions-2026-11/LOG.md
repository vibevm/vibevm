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
