# Campaign: template packages and version management for every application

**Status:** planned 2026-11-10; written against `main` at `7d495d137`. Campaign
node `APP-VERSIONS`: starts after `CORP-ENV` closes, before `SYS-PROMPT`.
**Phase 0 spikes S1, S2, S3 and S5 ran 2026-11-10 and phases P1, P5a, P2, P3
and P6 are implemented** — the answers and what they changed are in `LOG.md`,
and the facts those phases deliver are `impl/done` in PROP-075. S4 and S6 are still
open; S4 belongs with P5a and S6 with P7.
Phase 0 (spikes) and Phases 5a / 6 may run while `CORP-ENV` finishes, because
their files are disjoint from it.
Cold start: read this file, then `LOG.md` in this directory, then
`spec://org.vibevm.core/vibevm/common/PROP-075#root`.

## 1. Mandate

The owner's decisions are
[`PROP-075`](../../vibevm/vibespecs/common/PROP-075-template-packages-and-versions.xml)
sections 1–5, adopted 2026-11-09 and unchanged. The design settled against the
code on 2026-11-10 is the decision facts of the same document, sections 2–7.
This file does not restate either; it is the order of work, the files, the
gates and the risks.

Two sentences carry the whole campaign. **A template package is a manifest
written once with variables plus a declared place its versions and digests come
from, rendered by a pure function on the client at resolution time.** **Every
application gets nvm's job — list, install, switch, per-project files, prompt
activation, run-one-command, carry state forward — from general machinery in
vibe, with everything tool-specific in the tool's own package.**

The gate on the second sentence is algorithmic and non-negotiable
(`PROP-075#VIBENVM-ALGORITHMIC-PROOF`): a differential suite against a pinned
nvm-sh, verdict = exit code, no model's judgement anywhere in it.

## 2. Baseline — verified facts at writing time (code, not specs)

Verified by a read-only survey of the tree on 2026-11-10; references are to
that tree.

- **Manifest model.** `PackageMeta` (`crates/vibe-core/src/manifest/package/meta.rs:33`)
  has no serde derive; serialisation goes through `PackageMetaWire`
  (`…/package/wire.rs:25`, `deny_unknown_fields`) and two `From` arms. The
  whole-document closed world is `ManifestWire`
  (`…/package/visibility.rs:299`, `deny_unknown_fields`) with
  `TryFrom` both ways in `…/visibility/conversion.rs`. **There is no
  raw-TOML-preserving manifest type** — `Manifest` is typed, and comments are
  recovered by a post-serialisation `toml_edit` decoration merge
  (`…/manifest/decor.rs`, `…/manifest/mod.rs:212`). `toml_edit` is already a
  `vibe-core` dependency and is already used for codemods
  (`…/manifest/migration.rs`).
- **Resolver seams.** Exactly two: `VersionEnumerator::list_versions`
  (`crates/vibe-resolver/src/lib.rs:294`) and `DepProvider::fetch_manifest`
  (same file, `:241`). Production implementor `MultiRegistryDepProvider`
  (`…/multi_registry_provider.rs:20`) over `MultiRegistryResolver`
  (`crates/vibe-registry/src/multi_registry_resolver/`): enumeration at
  `walk.rs:25`/`:44`, manifest at `walk/manifest.rs:23`/`:34`. Per-backend
  `Registry` trait is `list_versions` / `resolve(pkgref) -> ResolvedPackage
  { source_dir }` / `fetch(resolved, store_root) -> CachedPackage`
  (`crates/vibe-registry/src/lib.rs:124`) — **a package version is a
  directory**, which is what lets a rendered manifest be an ordinary package.
- **`install -g` does not use `vibe-resolver`.** It builds a synthetic
  `[project]` manifest and calls
  `vibe_package_source::build_install_resolver_with_operation` then
  `resolver.resolve_and_fetch` (`crates/vibe-app/src/resolve.rs:181-232`), which
  reaches the same `MultiRegistryResolver`. One template layer therefore serves
  project resolution and `install -g` both.
- **Requirements.** `crates/vibe-requirements/` is *not* the dependency
  grammar — it is the PROP-054 spec-facts query library. The grammar is
  `PackageRef::parse` / `VersionSpec` (`crates/vibe-core/src/package_ref.rs:279`,
  `:412`). `VersionSpec` is `Latest | Req(semver::VersionReq)` and the `Req`
  arm is a hard `semver::VersionReq`. **There is no channel, label or dist-tag
  notion anywhere** (`SemverVersionSet` mirrors the two variants,
  `…/resolvo_engine/version_set.rs:17`). Git registries silently drop every tag
  that is not `v<semver>` (`git_package_registry/lookup.rs:170-187`).
- **Lockfile.** `crates/vibe-core/src/manifest/lockfile.rs`:
  `CURRENT_SCHEMA_VERSION = 9` (`:42`), `admit` accepts only 7/8/9
  (`lockfile/io.rs:24-39`), `Lockfile` and `LockedPackage` both
  `deny_unknown_fields`. **No JTD schema describes `vibe.lock`**;
  `[format.lockfile]` is `schema = "none"`, epoch 1, and the 7→8→9 steps did
  not move that epoch.
- **Application inventory.** `INDEX_PROTOCOL = "vibe-user-applications/6"`
  (`crates/vibe-app/src/index.rs:23`) over the **e5** schema
  (`schemas/application/e5/index.jtd.json`, `[format.application-index]`
  epoch 5): the writer writes the lowest protocol that carries what the home
  retains. The PATH proxy admits a closed set `/3`…`/6`
  (`crates/vibe-cli/src/bin/vibe-app-proxy/index.rs:40-65`) and answers
  `--supports-index` (PROP-059 `#proxy-index-upgrade`).
- **Index.** `_index/` holds `hello.json`, `repomd.json`, `primary.jsonl`(+`.gz`),
  `by-name/`, `by-cap/`, `by-purl/`, `listing.txt`, `state/journal/`,
  `mirrors/`. The writer's ownership whitelist is `WRITER_FILES` /
  `WRITER_DIRS` (`crates/vibe-index/src/index/mod.rs:55-64`). The per-version
  record is the **shared vocabulary** `version_entry`
  (`schemas/index/e1/entry.jtd.json` is a bare `ref`), so one optional member
  reaches `primary`, `by-name` and the journal at once.
- **`vibe registry mirror`.** Implemented in `vibe-cli`, not `vibe-registry`
  (`crates/vibe-cli/src/commands/registry/mirror.rs:56`, pipeline
  `list → plan → fetch → publish`). Selection today is `Filters { packages,
  groups }` with exact matching and **no excludes, no version ranges, no
  newest-N** (`mirror.rs:303-348`). `--dry-run` exists. Provenance record
  `_index/mirrors/<source>.toml` `schema = 1`
  (`mirror/record.rs:15-78`), with `[source]`, `[sync]`, `[[version]]`;
  a non-1 schema is a hard refusal. Report
  `schemas/registry_mirror_report.jtd.json`, `[format.cli-registry-mirror-report]`
  epoch 1, discriminator `command: "registry:mirror"` (no `protocol` member).
  **The reverse mirror is not implemented.**
- **Artifact drops.** `crates/vibe-core/src/remap/drop.rs:176`
  `package_lookup` — four rungs `G/N/V/F`, `G/N/F`, `D/F`, `F`; digest always
  verified, a wrong file refused and not kept
  (`crates/vibe-app/src/installer/drop.rs:98-128`).
- **Step engine.** Kinds are the closed `StepKind`
  (`crates/vibe-core/src/manifest/package/application/steps.rs:195`): `env`,
  `sources`, `archive`, `recipe`, `shell`, `native`, `copy`, `agent`.
  **No `export`/`import` kind exists.** Per-step entry
  `steps::kinds::run(cx: &StepContext, step: &StepDecl)`
  (`crates/vibe-app/src/steps/kinds.rs:99`); the context carries `work`,
  `package_root`, `cache_root`, `app_root`, `phase`, `vars`, `env`, `remap`,
  `io`, `transfer` (`kinds.rs:28-59`); child env is set at `kinds.rs:240-252`.
  Requirement PATH composition is `StepPaths::child_env`
  (`crates/vibe-app/src/steps/env.rs:36`) — which is exactly the shape a
  migration step's PATH needs.
- **Slot links and the off switch.** `crates/vibe-app/src/paths.rs`:
  reconciliation `sync_locked` (`:215`) and `apply_locked` (`:311`), link paths
  `<store>/paths/<slot>/<i>` (`settings.rs:246-261`), ownership in
  `<home>/applications/paths.toml` (`paths/ownership.rs:69`), session dirs
  `session_dirs` (`:409`) and `slot_link_dirs` (`:434`). `vibe shell --use`
  composes the child PATH with `compose_path`
  (`crates/vibe-cli/src/commands/shell/mod.rs:165`) and sets `VIBE_APP_USE`
  from `SessionChoices::render()` (`crates/vibe-app/src/session.rs:84`).
- **Shell syntax already exists, for `vibe self` only.** `Shell` with
  `detect` / `parse` / `export_line` / `rc_path` and the three quoting helpers
  is `crates/vibe-cli/src/commands/vvm/env.rs:88-160`; `--shell` on
  `vibe self env` is the **only** `--shell` flag in the CLI
  (`crates/vibe-cli/src/cli/vvm.rs:383`). **There is no `vibe hook` and nothing
  prints activation lines for applications** — `ProfileEnvironment` *writes*
  `<settings>/env.sh` / `env.fish` and one managed block
  (`crates/vibe-app/src/environment.rs:89-113`,
  `environment/blocks.rs:8-27`), and `vibe shell` composes in-process and prints
  nothing. There is no `Cmd` variant.
- **Adding an `app` subcommand is four touch points**: `AppCommand`
  (`crates/vibe-cli/src/cli/app.rs:193`), the dispatch arm
  (`crates/vibe-cli/src/commands/app.rs:59…`), the progress classification
  (`crates/vibe-cli/src/command_progress.rs:170-200`, non-exhaustive match =
  compile error if forgotten), and the JSON envelope
  `output::Context::emit_json` (`crates/vibe-cli/src/output.rs:509`) over a
  `vibe_wire::generated::*` type. `commands/app/alternatives.rs` is the clean
  80-line template of all four.
- **CLI test isolation.** `crates/vibe-test-support` (linking it is the whole
  opt-in: a load-time constructor redirects `$VIBE_SETTINGS`,
  `$VIBE_REGISTRY_CACHE`, `$VIBEVM_SEARCH_CACHE_DIR` before the first test
  body), plus `UserScratch` (`lib.rs:110`) and the app fixture
  `crates/vibe-cli/tests/common/app_registry.rs` (`AppRegistry::package`,
  `::app`, `::index`, `::vibe_in`, which also sets a scratch `HOME` and
  removes `VIBE_APP_USE`). Backstop `tools/user-home-tripwire.sh` in
  `tools/self-check.sh`.
- **Format discipline.** `formats/EPOCHS.toml` is `public = false`,
  `break_window_open = true`: breaking is free and unmigrated, **no break note
  under `formats/breaks/` is required**. A format must exist in
  `formats/REGISTRY.toml` or `FormatId` cannot express it
  (`cargo xtask codegen`, `check-codegen`, the `format_id_completeness` test).
  `cargo xtask wire-diff` reports a `schemas/**` + `formats/**` shift and is
  green while `public = false`.
- **The real packages.** `/fast/git/vibe-pkg-node` is one rendered `vibe.toml`
  per version (tags `v24.22.2`, `v22.23.4`) with **seven** installer options
  differing only in platform/arch/filename/`sha256`/`size`/`payload_root` and
  the Windows executable spelling; `path_dirs = ["bin"]` (`["."]` on Windows).
  Its generator `/fast/git/vibe-pkg-node-generator/payload/generate-package-node.py`
  (215 lines) renders from `SHASUMS256.txt` **plus a hand-made `SIZES` file**
  (`stat -c '%s %n' *`) — upstream publishes no sizes, which is why the
  template omits `size`. `/fast/git/vibe-pkg-codkit` is the second shape:
  `executable = "command:node"`, `public_commands`, no `path_dirs`, and **it is
  not templated by this campaign**.
- **`https://nodejs.org/dist/index.json`** (fetched 2026-11-10): a top-level
  JSON array, 878 records, each `{version: "vX.Y.Z", date, files[], npm, v8,
  uv, zlib, openssl, modules, lts: false | "<Codename>", security}`. Twelve
  distinct LTS codenames. `files[]` differs by version, which is why
  `template_optional` exists.
- **nvm-sh** pinned at **v0.41.0**, tag object `43d410caa64ba7928cef750e31fccbf5790c210c`,
  commit `4e8758a801eea9479b32a904f31093e410ddaba4`. 23 dispatch cases
  (`nvm.sh:4875-6742`), help block at `:4741-4858`, `--only=<categories>` on
  nearly every verb. Its `test/fast/` suite is *near*-offline (it stubs
  `nvm_download` and compares against `test/fast/Unit tests/mocks/`) but has
  **no suite-wide mirror override and no local fixture server**, and a few
  `fast` tests do reach `nodejs.org`; everything else
  (`slow/`, `installation_*/`, `sourcing/`, `install_script/`, `xenial/`)
  needs the network by construction.

## 3. Phase 0 — spikes (no commits)

Each spike answers one question that would change the shape of a later phase.
Run them with the isolated runner the repo uses
(`python3 /fast/vibevm-test-tmp/claude-int/run_test.py <label> …`).

- **S1 — the renderer on the real manifest.** *(done 2026-11-10 — all four
  questions answered yes; `LOG.md`.)* Turn
  `/fast/git/vibe-pkg-node`'s `vibe.toml` into a template by hand and render
  `24.22.2` with `toml_edit`. Questions: can `{platform}` / `{arch}` be bound
  per `[[installer_options]]` row by descent alone; does the lone-variable
  numeric coercion work as a node replacement; does
  `Manifest::parse_str(rendered)` accept it; and is the output byte-identical
  to the committed `vibe.toml` modulo the generator's provenance header **and
  the `size` lines** — the loss PROP-075 `#TEMPLATE-DIGEST-SOURCE` records,
  because nodejs.org publishes digests but no sizes?
  *That last comparison is the whole correctness proof of P1 and P3.*
- **S2 — one choke point.** *(done 2026-11-10 — no fifth call site; the four
  PROP-075 `#TEMPLATE-DOMAIN-SEAM` names are exactly the ones the three paths
  use, and which of them a path uses depends on the path; `LOG.md`.)*
  Instrument `MultiRegistryResolver::list_versions`
  and `fetch_manifest_inner` with a tracing span, then run `vibe install -g
  --plan-only` and a project `vibe install`, both against a local registry, and
  confirm both paths pass through. If `install -g` reaches a registry without
  going through those two, P2 gains a fifth call site.
- **S3 — the shared index vocabulary.** *(done 2026-11-10 — the golden bytes
  do not move, so AV-FMT-003 stays epoch 1 → 1; `LOG.md`.)* Add an optional
  `template: boolean` to
  `version_entry`, run `cargo xtask codegen`, `check-codegen`,
  `rebuild --check` and `wire-diff`, and confirm the
  `formats/corpora/index/e1` golden bytes are unchanged when the member is
  absent. If they are not, the index change becomes an epoch bump.
- **S4 — the hook's cost.** **Done 2026-11-10; the budget is met and nothing
  was renegotiated.** Builtin-only fast path in bash: 15 µs on an unchanged
  `$PWD`, 205 µs for a 24-test upward walk, zero subprocesses in both. The one
  `vibe app env --project` call: **21 ms warm** in a debug build against a
  24 ms process floor and a 50 ms budget. zsh and fish are not installed on
  this host, so their generated bodies are covered by unit tests on the text
  and by shell tests that skip loudly. Numbers and method: `LOG.md`.
- **S5 — proxy protocol /7.** *(done 2026-11-10 — confirmed on all three
  counts; `LOG.md`.)* Confirm that a home with no template
  installation still writes `vibe-user-applications/6`, that
  `vibe-app-proxy --supports-index vibe-user-applications/7` plumbing is a
  one-line set change, and that the admission list is the only gate
  (PROP-059 `#proxy-index-upgrade`).
- **S6 — the conformance sandbox.** Stand up the two-sandbox harness skeleton
  with a loopback `python3 -m http.server` over four pinned Node versions and
  run `nvm install <v>` in sandbox A offline end to end. Confirm
  `NVM_NODEJS_ORG_MIRROR` is enough for A and a remap row is enough for B.
  If A cannot be made fully offline, the suite's "no internet" claim is
  renegotiated before P7, not after.

## 4. Phases and gates

Every phase after 0 ends with the same gate: targeted tests for the touched
owners, `cargo fmt --check`, strict clippy, `cargo xtask conform check` (0 new),
`cargo xtask check-codegen`, `cargo xtask wire-diff`,
`cargo check --workspace --all-targets --target x86_64-pc-windows-gnu`; the
final phase runs the full workspace suite plus `tools/self-check.sh`. Each
phase is one or a few atomic commits, and every user-visible change gets an
`AV-FMT-NNN` entry in this directory's `FORMAT-CHANGES.md`.

### P1 — template manifest, pure rendering, validation — **M** — *delivered 2026-11-10*

**Goal.** `[package] template = true` and `[template]` parse, validate and
render, with no resolver, registry, network or filesystem involvement at all.

**Files, as delivered.** New crate `crates/vibe-template/` (`error.rs`;
`render.rs` with `render/grammar.rs` and `render/substitute.rs` — no `std::fs`,
no `vibe-net`, pinned by `tests/purity.rs` over the crate's dependency table
*and* its sources; `source.rs` — the typed `[template]` read out of a template
document plus fetched version/digest documents turned into pure inputs;
`tests/node_golden.rs` with `tests/fixtures/node/`).
`crates/vibe-core/src/manifest/package/meta.rs` + `…/package/wire.rs` (the
`template` flag, all four sites); `…/manifest/document.rs`,
`…/package/visibility.rs`, `…/visibility/conversion.rs` (the `[template]`
table); a new `…/package/template.rs` (+ `template/validation.rs`,
`template/tests.rs`) for the typed section;
`…/manifest/document/validation.rs` and the new
`…/document/validation/template.rs` (the refusals, each naming its TOML path);
`…/package/application.rs` (`template_optional` on `[[installer_options]]`),
`…/application/site.rs` + `…/application/validation.rs` +
`…/application/step_validation.rs` (the digest-field exemption, carried on
`Site`); `crates/vibe-check/src/checks/template_render.rs` (the new check),
`crates/vibe-check/src/lib.rs` + `…/checks/mod.rs` (the `CheckId` and the
registration), `conform.toml` (the new crate, gated); `Cargo.toml`.

**New types.** `TemplateSection { versions: VersionSource, digests: DigestSource,
channels: Vec<ChannelRule>, version_list_ttl: Option<Duration> }`;
`VersionSource::{Json { url, select, version_field, strip_prefix, min, max,
prerelease }, List { items }}`; `DigestSource::{Sha256Sums { url },
Json { url, select, file_field, digest_field, size_field }}`;
`ChannelRule::{Truthy { name, from }, Value { prefix, from }}`;
`VersionRecord { version, raw, meta: BTreeMap<String, Scalar> }`;
`DigestTable`; `Rendered { toml: String, sha256, dropped: Vec<DroppedOption>,
stage: Skeleton | Full }`.

**Commands.** None new. `vibe check` gains the template check.

**Formats.** `AV-FMT-001`.

**Tests.** Unit, in `vibe-template`: the whole variable grammar including
`{{`/`}}`, the one-level nesting under `sha256:`, the lone-variable numeric
coercion, a `{` that opens nothing, a `{meta.…}` resolving to an array, a
digest variable outside a digest-bearing field, `template_optional` dropping a
row, a missing digest for a non-optional row, `{platform}` outside an installer
option. Property test: rendering is deterministic — the same inputs give
byte-identical output, which is the generator's own stated invariant
(`rendering_is_deterministic_over_many_inputs`, 36 renders, no two alike).
Golden test: the real Node template rendered at `24.22.2` and `22.23.4` equals
the two committed `vibe.toml` files from `/fast/git/vibe-pkg-node`, modulo the
generator's provenance header **and the `size` lines** (fixtures copied into
`crates/vibe-template/tests/fixtures/`, provenance in that directory's
`README.md`; the test asserts the count of stripped `size` lines so no second
divergence hides in the normalisation). Manifest-layer tests: the flag and the
table round-trip; `[template]` without the flag refuses; the flag on a non-`app`
kind refuses; every refusal names a TOML path. Purity test: `vibe-template`'s
own dependency table and sources reach no filesystem, socket, clock, child
process or registry.

**Acceptance.**
```sh
cargo test -p vibe-template -p vibe-core -p vibe-check --offline --locked
# the golden comparison must be in that run and must be exact:
cargo test -p vibe-template --test node_golden renders_the_committed_node_manifests -- --exact
```

**Risks, as they turned out.** Both are closed. The numeric coercion works —
the renderer replaces the whole TOML node — but a template that uses it does
not deserialise into the typed `Manifest`, so the renderer reads the document
as TOML text and the `[template]` table on its own; recorded in the new
`PROP-075#TEMPLATE-DOCUMENT-READING`, which also names the one visible
consequence (`vibe check`'s manifest-validity cell reports such a template as
unparseable; admitting a lone variable in a typed numeric field is a separate,
later change to the typed layer, and neither real package needs it).
`{platform}` / `{arch}` bind per row by descent alone, so the fallback of
dropping them from the grammar was not taken.

**Parallel.** With P5a and P6.

### P2 — resolution, channels, lock, trust, offline cache — **L** — *delivered 2026-11-10*

**Goal.** A template's versions are resolvable, installable, lockable and
alarmed; `@lts` works; nothing touches the network twice.

**Files.** New `crates/vibe-registry/src/templates.rs` (discovery, fetch,
cache, store materialisation) wired at
`multi_registry_resolver/walk.rs` (`list_versions_in_registry`),
`multi_registry_resolver/walk/manifest.rs` (`fetch_manifest_inner`),
`multi_registry_resolver/` resolve/fetch, and
`crates/vibe-registry/src/local_registry.rs` (`list_versions`, `resolve`).
`crates/vibe-core/src/package_ref.rs` (`VersionSpec::Channel`, `parse`,
`matches`, `is_exact_pin`, `Display`); `crates/vibe-resolver/src/resolvo_engine/version_set.rs`
(a channel maps to the empty set — fail closed);
`crates/vibe-core/src/manifest/lockfile.rs` + `lockfile/io.rs` (the `template`
table, `via_channel`, `schema_version` 9 → 10, `admit` 7..=10);
`crates/vibe-core/src/remap/` (two artifact kinds);
`crates/vibe-core/src/user_config/` (`[template] version_list_ttl`);
`schemas/index/e1/` (`version_entry` gains `template`, `channels`) and
`crates/vibe-index/src/scanner/tree_entry.rs` (projection writes them);
`crates/vibe-publish/` (publish writes them).

**New types / functions.** `TemplateLayer` with `discover(group, name) ->
Option<ActiveTemplate>`, `domain(&ActiveTemplate) -> VersionDomain`,
`skeleton(&ActiveTemplate, version) -> Manifest`,
`materialise(&ActiveTemplate, version) -> ResolvedPackage`;
`lower_channel(&mut PackageRef, &dyn ChannelResolver)`;
`LockedTemplate { revision, rendered_sha256, digest_source_sha256, source_url,
digest_url }`.

**Commands.** `--refresh` on the resolving commands.

**Formats.** `AV-FMT-002` (lockfile, internal schema 9 → 10),
`AV-FMT-003` (index per-version record), `AV-FMT-004` (two remap artifact
kinds), `AV-FMT-005` (user config `[template] version_list_ttl`),
`AV-FMT-017` (application index `vibe-user-applications/7`, schema generation
`e6`, and the `vibe-app-proxy` admission PROP-059 `#proxy-index-upgrade`
requires), `AV-FMT-018` (`vibe app describe` template provenance).

**Tests.** Unit: the channel grammar and the semver-first precedence
(`^22`, `=1.2.3`, `lts`, `lts/jod`, `latest`, `stable`, and `1.x` still a
`VersionReq`); an unlowered channel reaching the solver resolves nothing;
domain union with concrete-wins; the template revision excluded from the
domain. CLI with an isolated HOME (`AppRegistry` + a loopback `http_dir`
fixture serving a fake version list and digest document): `vibe install -g
org.example/tool@^2` installs a template-produced version and the index record
carries the template table; a second run is offline and makes zero requests;
the digest document mutated upstream refuses with `template_render_changed`
naming the differing field; a concretely published version of the same number
wins and no digest is fetched; `--offline` with a cold cache refuses with
`template_cache_missing` naming the path; a locked project install makes
exactly one request (asserted by a counting fixture server);
`template_index_required` on an unindexed git registry. Lock round-trip:
schema 10 reads, 9 still reads, 11 refuses.

**Acceptance.**
```sh
cargo test -p vibe-core -p vibe-registry -p vibe-resolver --offline --locked
cargo test -p vibe-cli --test cli_template_resolution --offline --locked
```
*As delivered the trust cases live in `cli_template_resolution` rather than a
second file: the alarm fires from the same `install -g` the resolution cases
drive, over the same fixture, and splitting it would have duplicated the
fixture to assert two of its seven cases
(`a_digest_document_that_changed_upstream_is_the_alarm_before_any_effect`,
`a_template_fixed_without_touching_a_digest_is_not_an_alarm`).*

**Risks, as they turned out.** Both closed. `VersionSpec` is read across
eighteen files, not the eight the survey named — and the compiler found every
one of them, because the enum is matched exhaustively everywhere, which turned
the risk into a build error instead of a silent wrong install; each site now either lowers first or fails
closed, and the two solver-side ones are tested directly
(`an_unlowered_channel_matches_nothing`,
`SemverVersionSet::from_spec` → `None`). The lockfile bump strands nothing
observable, because the writer still writes the lowest epoch that carries what
the lock holds: a project with no template package and no channel requirement
writes byte-identically to before.

**As delivered.** Everything in the goal sentence works end-to-end through the
real binary, offline: `crates/vibe-cli/tests/cli_template_resolution.rs` (7
tests) installs a template-produced version through all four call sites, lowers
`lts`, `lts/boron` and `stable`, proves a concretely published version is never
rendered for, proves a cached digest document is never revalidated, and fires
the `template_render_changed` alarm before any effect while leaving a template
*fixed* without touching a digest free. Two deliverables of the phase are
deliberately **not** in it, each with its reason in `FORMAT-CHANGES.md`:
`AV-FMT-017` (the `vibe-user-applications/7` inventory) and `AV-FMT-018`
(`vibe app describe`'s provenance, which reads the record `/7` carries), because
the `Installation` record `/7` defines must also carry P6's migration outcomes
and the writer's protocol ladder means nothing a user sees waits on them.
**P6 delivered both halves of that move on 2026-11-10** — the `/7` inventory
with its `e6` generation and the `vibe-app-proxy` admission — so AV-FMT-017 is
delivered and AV-FMT-018 is unblocked but still unwritten.
Template **rendering** from a per-package git registry is the one part of
`#TEMPLATE-DISCOVERY` left to the phase that first needs it; the index member
that carries the fact (`AV-FMT-003`) and the refusal (`template_index_required`)
are both delivered, and every directory-shaped registry — `--registry DIR`, a
`localdir` source, a `monorepo` working copy, a mirrored layout registry —
resolves templates today. The eight as-built amendments are
`PROP-075#P2-AS-BUILT`.

**Parallel.** Needs P1. P4, P5b and P3 all need P2.

### P3 — Node as a template package, replacing the generator — **M** — *delivered 2026-11-10*

**Goal.** `org.vibevm.node/node` is one template package with channels;
the generator is retired; **codkit is untouched**.

**Files.** Mostly **outside this repository**:
`/fast/git/vibe-pkg-node` gains a `[template]` manifest at a new template
revision and keeps its two concrete tags as published versions (so
`#TEMPLATE-CONCRETE-WINS` is exercised in production from day one);
`/fast/git/vibe-pkg-node-generator` is marked superseded in its README and its
package is not republished. In this repository: a fixture copy of the template
under `crates/vibe-template/tests/fixtures/node/` and the campaign's own
`accept.sh` step that installs Node from it.

**Commands.** None new.

**Formats.** `AV-FMT-006` (the Node package's shape changes; no vibe format).

**Tests.** The P1 golden comparison is the regression gate: the template must
render the two committed manifests exactly. Plus a channel test:
`lts` resolves to the newest record whose `lts` field is truthy and
`lts/jod` to the newest of that codename, against a frozen copy of
`index.json` (committed as a fixture, not fetched).

**Acceptance.**
```sh
# offline, against the frozen index fixture and a local registry:
cargo test -p vibe-cli --test cli_template_node --offline --locked
# and on this machine, once, by hand:
vibe app versions org.vibevm.node/node --json | head
vibe install -g org.vibevm.node/node@lts && node -v && npm -v
```

**Risks, as they turned out.** The `template_optional` risk **fired**, and the
case the plan prescribed for it is what caught it: `files[]` varies by version,
and not only on the two rows named above — upstream's Apple-silicon archive
begins at 16.0.0, so `macos-arm64` is a third optional row and without it every
release below 16.0.0 (a third of the declared window) refuses to render. It
surfaced while preparing that very case — the oldest version in the `min`
window — which is now a test in both suites. Measured
over the 406 produced versions: `linux-x64-musl` absent from 389,
`osx-arm64-tar` from 132, `win-arm64-zip` from 234, all seven present together
only from 26.8.0. The size loss is as designed and recorded.

**As delivered.** The package repository carries the template at revision
`1.0.0` (tag `v1.0.0`, commit `1a52f3d`) and keeps `v24.22.2` / `v22.23.4`
published concretely; the generator repository records its own supersession
(`c160ba3`). Nothing is published or pushed, for the one reason `AV-FMT-006`
now states: rendering a template found in a per-package **git** registry is the
part of `#TEMPLATE-DISCOVERY` P2 deferred, and P3 does not need it — the suite
and the hand install both use directory-shaped registries. In this repository
the fixtures became upstream's own frozen documents (a leading, byte-verbatim
slice of `index.json` plus four verbatim `SHASUMS256.txt` files; the two golden
ones hash to exactly the inputs the committed manifests' provenance headers
name, which `node_golden.rs` asserts), and `cli_template_node.rs` drives six
cases through the real binary — plus one that pins the template under test to
the golden fixture byte for byte: `lts` → 26.13.0, `lts/jod` → 22.23.4, an
undeclared `lts/argon` refused by name, the oldest version in the window
rendered with four rows instead of seven, both named optional rows refused with
`no_option` for 12.0.0 and accepted for 24.22.2, concrete-wins over the real
published tag, and `--offline` resolving with the mirror deleted. The hand
acceptance ran with a throwaway `HOME` / `VIBE_SETTINGS` under `/fast/tmp`:
`vibe install -g org.vibevm.node/node@lts` installed 26.13.0 over the real
network and `node -v` answered `v26.13.0` (`npm -v` → `11.20.0`); the
`vibe app versions` line of the acceptance is **not** run, because that command
is P5b and does not exist yet. The four as-built amendments are
`PROP-075#P3-AS-BUILT`.

**Parallel.** Needs P1 and P2. Independent of P4, P5, P6.

### P4 — mirror config, template snapshot, payload projection, prune — **L**

**Goal.** `vibe registry mirror --config mirror.toml` selects declaratively,
snapshots a template's inputs into the target's `_index/`, projects payloads
into the drop under a budget, and prunes only when told.

**Files.** `crates/vibe-cli/src/cli/registry.rs` (`--config`, `--prune`,
`--prune-payloads`, `--yes`); `crates/vibe-cli/src/commands/registry/mirror.rs`
and its `mirror/{plan,record,source,target}.rs` (the new selection model
replacing `Filters`, the template rows, the prune plan);
new `mirror/config.rs` (the `mirror.toml` reader) and `mirror/templates.rs`
(snapshot writing through the existing `BatchConfig.files` channel);
`crates/vibe-index/src/index/mod.rs` (`WRITER_DIRS` learns
`templates`), `crates/vibe-index/src/layout.rs` and the `repomd.json` `files`
projection; `schemas/index/e1/templates.jtd.json` (new);
`schemas/registry_mirror_report.jtd.json` (`templates[]`, `pruned[]`);
`formats/REGISTRY.toml` (`[format.index-templates]`).

**Commands.** `vibe registry mirror <source> --into <target> --config mirror.toml
[--prune] [--prune-payloads] [--dry-run] [--yes]`.

**Formats.** `AV-FMT-007` (mirror selection + provenance record),
`AV-FMT-008` (template snapshot, new format),
`AV-FMT-009` (mirror report), `AV-FMT-010` (`repomd` `files` key).

**Tests.** Unit: the `mirror.toml` reader including every refusal (bad schema,
empty selection, `[payloads] into` missing, payloads into the registry target);
`newest = N` per major against a frozen index; `max_total` arithmetic with
unknown sizes. CLI with two local registries: a template mirrored with
`versions = [">=22"]` and `channels = ["lts"]` writes exactly the expected
snapshot tree; a client with **only** the target registry configured and the
network denied renders and installs a snapshotted version; a re-run with a
changed upstream digest for an already-snapshotted version stops before writing
with the alarm; `--dry-run` writes nothing and prints the projection;
`--prune` after narrowing prints a plan and removes nothing without `--yes`;
a version recorded as copied from another source is never pruned.

**Acceptance.**
```sh
cargo test -p vibe-cli --test cli_registry_mirror --offline --locked
cargo test -p vibe-cli --test cli_registry_mirror_templates --offline --locked
cargo test -p vibe-index --offline --locked
cargo xtask check-codegen && cargo xtask wire-diff
```

**Risks.** `_index/` has an explicit ownership whitelist and a `listing.txt`
written last; a new directory that the projection does not know about makes the
index read `Stale`. The test that a client installs from the snapshot **with
the network denied** is the only honest proof that the snapshot is complete.

**Parallel.** Needs P1 and P2. Independent of P3, P5, P6.

### P5a — version files, `app env`, `vibe hook`, `app exec` — **L** — **DONE 2026-11-10**

**Goal.** Per-project version selection, shell activation and run-one-command,
none of which needs a template at all.

**As built.** The nine amendments are PROP-075 `#P5A-AS-BUILT`; the two
formats the plan did not foresee are `AV-FMT-101` (the declared version files
beside the index, because retaining them *in* it would drag `AV-FMT-017`'s
`/7` move out of P2) and `AV-FMT-102` (the resolution cache). Channel lowering
beyond the reserved `latest` / `stable` belongs to P2, so
`#VERSION-FILE-FORMATS` and `#VERSION-EXEC-SPELLING` are `impl/work`. The CLI
test target is `cli_app_env_activation`, not `cli_app_env`: that name was
already taken by the PROP-065 env-step fixture.

**Files.** `crates/vibe-core/src/manifest/package/application.rs` +
`…/application/validation.rs` (`[[application.version_file]]`);
new `crates/vibe-app/src/versionfile.rs` (the two formats, the upward search,
the mtime+size cache in `<home>/cache/version-files.json`);
new `crates/vibe-app/src/activation.rs` (resolve a selection for a directory:
flags > project requirement > version file > slot default > off);
`crates/vibe-cli/src/commands/vvm/env.rs` → promoted to a shared
`crates/vibe-cli/src/shell_syntax.rs` with a new `Cmd` variant (and `vibe self
env` rewired to it, behaviour unchanged);
new `crates/vibe-cli/src/commands/app/env.rs`, `…/app/exec.rs`,
new `crates/vibe-cli/src/commands/hook.rs`;
`crates/vibe-cli/src/cli/app.rs`, `…/cli/command.rs`,
`…/commands/app.rs`, `…/command_progress.rs` (the four touch points, twice);
`crates/vibe-app/src/environment/blocks.rs` (the hook's managed block, reusing
the existing marker machinery);
`schemas/application_env.jtd.json` (new), `formats/REGISTRY.toml`.

**Commands.** `vibe app env [--use SLOT=VER]… [--project] [--shell …]
[--unset] [--json]`; `vibe app exec SLOT[@VER] [--project] -- CMD …`;
`vibe hook bash|zsh|fish|pwsh [--print] [--yes]`.

**Formats.** `AV-FMT-011` (version-file declaration),
`AV-FMT-012` (`vibe-app-env/1`), `AV-FMT-013` (`vibe hook`'s managed block).

**Tests.** Unit: both version-file formats including comments, CRLF, an empty
file, a channel name, a `.tool-versions` with several versions for one key;
the precedence table as a matrix; the upward search stopping at the project
root; the mtime cache invalidating on a touch. Unit: one golden line per shell
for `app env`, `--unset` included, with quoting of a path containing a space
and a single quote. **The invariant test**: for the same selection,
`vibe app env` and `vibe shell --use` produce the same `PATH` and the same
`VIBE_APP_USE` — asserted against the library, not by parsing text. CLI in a
pseudo-terminal on Linux (as PROP-059 `#acceptance` does for `vibe shell`):
`eval "$(vibe app env --use …)"` then `command -v <cmd>` resolves to the chosen
installation; `vibe app exec` passes the exit status and forwards `SIGTERM`;
`vibe hook bash --print | bash` then two `cd`s switch versions. **The cost
gate**: a fixture counts subprocesses in an unchanged directory (must be 0) and
on a `cd` into a version-file directory (must be 1), and asserts
`vibe app env --project` makes no network call and no registry read.

**Acceptance.**
```sh
cargo test -p vibe-app -p vibe-core --offline --locked
cargo test -p vibe-cli --test cli_app_env_activation --offline --locked
cargo test -p vibe-cli --test cli_app_exec --offline --locked
cargo test -p vibe-cli --test cli_hook_linux --offline --locked
# the budget, as a command:
cargo test -p vibe-cli --test cli_hook_linux hook_spawns_nothing_in_an_unchanged_directory
```

**Risks.** The hook is the only thing in this campaign that runs on every
prompt of every shell of every user, so its failure mode must be "do nothing
and say one line", never "fail the `cd`"; the pseudo-terminal test asserts that
with a broken index, a missing version and an unreadable version file. Promoting
`Shell` out of `vvm/` must not change `vibe self env`'s output — a golden test
on the existing lines goes in first, before the move.

**Parallel.** With P1 and P6. Needs neither.

### P5b — `vibe app versions` — **S**

**Goal.** One listing over the whole domain.

**Files.** New `crates/vibe-app/src/versions.rs`, new
`crates/vibe-cli/src/commands/app/versions.rs`, the four CLI touch points,
`schemas/application_versions.jtd.json` (new), `formats/REGISTRY.toml`.

**Commands.** `vibe app versions PACKAGE|SLOT [--channel NAME] [--installed]
[--limit N] [--all] [--refresh] [--registry DIR] [--json]`.

**Formats.** `AV-FMT-014` (`vibe-app-versions/1`).

**Tests.** CLI: a template package lists source and published versions with the
right origin marks and channel memberships; `--installed` needs no network
(asserted by a denying transport); a slot with two packages lists both;
`--limit` and the "and N more" trailer; a snapshot-backed registry shows the
snapshot's sync time.

**Acceptance.**
```sh
cargo test -p vibe-cli --test cli_app_versions --offline --locked
vibe app versions node --installed --json   # on this machine, zero network
```

**Parallel.** Needs P2. Independent of P4 and P6.

### P6 — migration — **M** — *delivered 2026-11-10*

**Goal.** `vibe app migrate` carries user state between two installed
versions, is offered and never run without consent, and reports exactly what
was not carried.

**As built.** The ten amendments are PROP-075 `#P6-AS-BUILT`. The phase also
delivered the `vibe-user-applications/7` inventory P2 deferred (AV-FMT-017,
schema generation `e6`, the frozen `e5` reader registered as
`[format.application-index-v6]`, and the `vibe-app-proxy` admission of `/7`
PROP-059 `#proxy-index-upgrade` requires), because the record it defines is
where the outcomes go. One format the plan did not foresee: `AV-FMT-103`
(`vibe app work` lists and cleans a migration's kept work directory). The
answered question P2 left for this phase: **the declared version files stay in
their sidecar** (`AV-FMT-101`) and `/7` does not absorb them — the sidecar is
`recoverable` derived state and folding it in would force every home that
declares a version file onto `/7`.

**Files, as delivered.**
`crates/vibe-core/src/manifest/package/application.rs` +
`…/application/validation.rs` + new `…/application/migration.rs` and
`…/application/migration_validation.rs` (`[[application.migration]]`, kinds
restricted to `recipe`/`shell`), `…/manifest/mod.rs`,
`…/manifest/document.rs` + new `…/document/tests_migration.rs`;
new `crates/vibe-app/src/migrate.rs` with `migrate/record.rs` (the work
directory and the one document that is both the report and the resumable
record), `migrate/run.rs` (the two sides over the existing step engine) and
`migrate/tests.rs`;
`crates/vibe-app/src/steps/kinds.rs` (`MigrationEnv` and the migration
environment; no new step kind), `…/steps/template.rs` (`$out` and `$sources`
refuse), `…/steps/mod.rs`, `…/steps/run.rs`, `…/steps/work.rs` (AV-FMT-103),
`…/settings.rs` (`app_migrate_dir`), `…/error.rs` + `…/codes.rs` (five codes),
`…/lib.rs`;
the `/7` move: `crates/vibe-app/src/index.rs`, new `…/index/migration.rs`
(the three retained records), new `…/index/upgrade_six.rs`, `…/index/wire.rs`,
`…/index/launcher_wire.rs`, `…/operations/apply.rs` (the template provenance
and the declared entries at publication), `…/operations/pending/identity.rs`,
`schemas/application/e6/index.jtd.json` (new),
`formats/corpora/application/e6/index.json` (new), `formats/REGISTRY.toml`,
`crates/vibe-cli/src/bin/vibe-app-proxy.rs` + `…/vibe-app-proxy/index.rs`,
`crates/vibe-wire/tests/source_evidence_corpora.rs`,
`crates/vibe-cli/tests/cli_app_proxy_index_versions.rs`;
the CLI: new `crates/vibe-cli/src/cli/app_migration.rs`, `…/cli/app.rs`,
new `crates/vibe-cli/src/commands/app/migrate.rs`, `…/commands/app.rs`,
`…/command_progress.rs` (the four touch points),
`…/commands/app/global.rs` (the offer after `install -g` / `update -g`) and
`…/commands/app/alternatives.rs` (the offer after `app use`);
`schemas/application_migration.jtd.json` (new).

**Commands.** `vibe app migrate SLOT FROM TO [--only ID]… [--all] [--yes]
[--resume] [--registry DIR] [--json]` — `--registry` as built, because a
migration step is the package's own script and the inventory retains no
package tree.

**Formats.** `AV-FMT-015` (migration declaration),
`AV-FMT-016` (`vibe-app-migration/1`), `AV-FMT-017` (the
`vibe-user-applications/7` inventory, deferred by P2 and delivered here
because the record it defines is where the outcomes go), and the unforeseen
`AV-FMT-103`. `AV-FMT-018` is no longer blocked and still not delivered: the
record it reads exists and `install -g` writes it, but projecting it into
`vibe app describe`'s document belongs to the phase that owns that document.

**Tests.** Unit: the declaration validates; a forbidden kind refuses naming it;
`FROM == TO` is admitted (the `latest-npm` shape). CLI with two installed
versions of a fixture package whose export writes a marker file and whose
import reads it: the happy path carries it; a failing export changes nothing
and never runs the import; **a failing import leaves the new version installed,
leaves the default where it was, keeps the work directory and names what was
not carried**; one entry's failure does not stop another's; `--resume` re-runs
only the import; unattended without `--yes` refuses `consent_required` and
prints the exact command; `install -g` of a new version offers and does not
migrate.

**Acceptance.**
```sh
cargo test -p vibe-app -p vibe-core --offline --locked
cargo test -p vibe-cli --test cli_app_migrate --offline --locked
cargo test -p vibe-cli --test cli_app_migrate failed_import_keeps_the_new_version -- --exact
```

**Risks, as they turned out.** Both closed. The PATH was wiring, exactly as
predicted: `StepPaths::child_env` over one `Exposure` per installation, and
the fixture proves it by having each side call the slot's own command and
record which marker answered (`who=st-1.0.0` in the export, `who=st-2.0.0` in
the import). The hazard of a script that destroys user state is unchanged and
is why nothing is rolled back and the work directory is never replaced — a
second run of the same pair takes a new timestamp rather than overwriting the
evidence of the first.

**Parallel.** With P1, P4, P5a.

### P7 — vibenvm and the algorithmic differential proof — **L**

**Goal.** `org.vibevm.node/vibenvm` plus a committed, rerunnable suite whose
exit code is the verdict, with complete coverage of the pinned inventory.

**Files.** New `conformance/vibenvm/` in this repository:
`run.sh`, `fetch-mirror.sh`, `mirror.lock`, `extract-inventory.sh`,
`coverage.py`, `normalise.rules`, `observe.sh`, `exclusions.toml`,
`nvm-tests.allow`, `cases/*.toml`, `README.md`, and a `.gitignore` for
`mirror/`; `xtask/src/main.rs` + a new `xtask/src/vibenvm_conform.rs`
(`cargo xtask vibenvm-conform`). The shim package itself is a new repository
`/fast/git/vibe-pkg-vibenvm` (`nvm.sh`-shaped `vibenvm.sh`, a `[[application]]`
of kind `app` with no `path_dirs`, installed by `vibe install -g`). The two
general features the mapping named land here if they have not already:
`--write-version-file` on `vibe app use` / `install -g` (P5a's files), and the
`latest-npm` migration entry admitted by P6.

**Commands.** `cargo xtask vibenvm-conform [--cases GLOB] [--update-inventory]`.

**Formats.** `AV-FMT-019` (`--write-version-file` on `vibe app use` and
`vibe install -g`), `AV-FMT-020` (the suite's authored documents:
`exclusions.toml`, `mirror.lock`, `nvm-tests.allow`, `normalise.rules`, and
the exit-code verdict).

**Tests.** The suite *is* the test. Its own meta-tests, which must exist or the
verdict is not trustworthy: a deliberately wrong shim fails with exit 1 naming
the field; a deliberately uncovered flag fails with exit 2 naming the pair; an
exclusion row without `approved_by = "owner"` fails with exit 2; a tampered
`mirror.lock` fails with exit 3; a non-loopback connection attempt fails with
exit 3.

**Acceptance.**
```sh
cd /fast/git/vibevm
conformance/vibenvm/fetch-mirror.sh          # once, the only networked step
cargo xtask vibenvm-conform ; echo "verdict=$?"   # must print verdict=0
# and the meta-tests, which prove the verdict can fail:
conformance/vibenvm/run.sh --self-test ; echo "selftest=$?"
```

**Risks.** The honest ones, named now. (1) nvm's own `fast` suite is *near*-
offline, not offline, and has no mirror override — so reusing it is bounded to
a recorded allow-list and the reuse is a bonus, never the gate. (2) The
inventory of the pinned v0.41.0 includes `--only=<categories>` on nearly every
verb; if the owner rejects that exclusion (Q2 below) the campaign gains a
general "version class" feature and P7 grows by a phase. (3) A differential
suite is only as strong as its observation list; the list is the owner's own,
and adding to it later is cheap because the comparison is field-by-field.
(4) Linux only, stated as such — this is not a cross-platform claim.

**Parallel.** Needs P2, P3, P5a, P5b, P6. Last by construction.

### Dependency order, as a graph

```
P0 spikes
  |
  +--> P1 ----+--> P2 --+--> P3 --+
  |           |         |         |
  |           |         +--> P4   +--> P7
  |           |         |         |
  |           |         +--> P5b -+
  +--> P5a ---------------------- +
  +--> P6 ----------------------- +
```

P1, P5a and P6 may run concurrently from the start. P3, P4 and P5b may run
concurrently once P2 lands.

## 5. Predictions (falsifiable)

- **P1** The template renders the two committed `/fast/git/vibe-pkg-node`
  manifests byte-for-byte modulo the generator's header. *If false, the
  variable grammar is wrong, not the manifests.* **Held, with the claim
  narrowed by one recorded loss: modulo the header *and* the `size` lines,
  which nodejs.org does not publish (PROP-075 `#TEMPLATE-DIGEST-SOURCE`,
  AV-FMT-006). Nothing else differs, for either version.**
- **P2** The template layer is four call sites and one new module; no file
  under `crates/vibe-resolver/src/resolvo_engine/` changes except
  `version_set.rs`. **Held on both halves, with the second half exact:
  `version_set.rs` is the only file under `resolvo_engine/` that changed.** The
  first half held in substance and grew in spelling: the four call sites are the
  four `#TEMPLATE-DOMAIN-SEAM` names, and the "one new module" is one module
  *directory* — `crates/vibe-registry/src/templates.rs` plus eight files under
  `templates/`, because the 600-line budget does not admit discovery, the cache,
  the transport, the domain, the two render stages and the errors in one file.
  Three call sites outside the four were needed and are named here rather than
  discovered later: `vibe-app`'s containment check had to admit the render root
  (a render has no home in a registry), `vibe-app`'s activation needed the
  channel-membership seam P5a left open, and the composition root installs the
  layer. None of them learns that a manifest was rendered — they learn only
  where a rendered tree lives and what a channel holds.
- **P3** `vibe install -g org.vibevm.node/node@lts` makes exactly two HTTP
  requests on a cold cache (the version list and one digest document) and one
  payload download. **Held in its observable form, not instrumented
  (2026-11-10):** the hand install on a cold settings root ended with exactly
  two documents cached — `versions.json` (whose recorded `sha256` is the live
  `index.json`'s, `6b27fe57…`) and `digests/26.13.0`, nothing else — plus one
  unpacked archive, so the layer fetched no document beyond those two and
  rendered one version. The *request* count itself was not counted: nothing in
  the tree instruments it, and the offline suites reach the documents through a
  `[[remap]]` row rather than a counting server. The strict form belongs with
  P4, which has to deny non-loopback traffic anyway.
- **P4** A client with only the mirrored registry configured and all
  non-loopback traffic denied installs a snapshotted Node version.
- **P5a** The prompt hook spawns zero processes on a prompt in an unchanged
  directory, measured. **Held (2026-11-10):** zero on an unchanged prompt,
  zero on a `cd` with no candidate file, exactly one on a `cd` that has one —
  counted by a shim named `vibe` on PATH in
  `cli_hook_linux::hook_spawns_nothing_in_an_unchanged_directory`.
- **P7** The mapping yields exactly the three new vibe features
  `PROP-075#VIBENVM-GAPS` names, and no fourth is discovered while writing the
  cases. *This is the campaign's central claim: nvm's surface is absorbed by
  general machinery.*

## 6. Quick start (cold)

```sh
cd /fast/git/vibevm
git log --oneline -3
cat campaigns/app-versions-2026-11/PLAN.md campaigns/app-versions-2026-11/LOG.md
# the owner's decisions and this design:
sed -n '1,40p' vibevm/vibespecs/common/PROP-075-template-packages-and-versions.xml
# the two real packages this is built for:
cat /fast/git/vibe-pkg-node/vibe.toml | head -60
# Phase 0 spikes use the isolated runner:
python3 /fast/vibevm-test-tmp/claude-int/run_test.py <label> cargo test -p <crate> --offline --locked
```

## 7. Acceptance script (whole campaign)

To be written in P7 as `campaigns/app-versions-2026-11/accept.sh`. It runs, in
one network-isolated sandbox with a fresh HOME: install Node from the template
at a channel; `vibe app versions`; switch with `vibe app env` and with the
prompt hook; `vibe app exec` at a second version; migrate the global npm
packages between them; mirror the template into a local registry with
`mirror.toml` and install from the snapshot with the network denied; and
finally `cargo xtask vibenvm-conform`. It exits non-zero on any step's
failure, any non-loopback connection attempt, and any unverified artifact use.

## 8. Open points

The design's open questions are
`spec://org.vibevm.core/vibevm/common/PROP-075#OWNER-QUESTIONS`, each with the
default it takes unless the owner objects: (Q1) the fourteen proposed nvm
exclusions need row-by-row owner approval, because an unapproved row is a gate
failure by construction; (Q2) `--only=<categories>` is the largest exclusion;
(Q3) the 15-minute version-list TTL; (Q4) templates undiscoverable in an
unindexed per-package git registry; (Q5) no `--trust-template` escape hatch
for the changed-digest alarm. Only Q1 blocks a phase, and only P7.

## 9. Log

See `LOG.md`.

## 10. Report

Empty until close.
