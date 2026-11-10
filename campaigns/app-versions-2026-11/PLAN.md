# Campaign: template packages and version management for every application

**Status:** planned 2026-11-10; written against `main` at `7d495d137`. Campaign
node `APP-VERSIONS`: starts after `CORP-ENV` closes, before `SYS-PROMPT`.
**Phase 0 spikes S1, S2, S3 and S5 ran 2026-11-10 and phases P1, P5a, P2, P3
P5b and P6 are implemented** — the answers and what they changed are in `LOG.md`,
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

### P4 — mirror config, template snapshot, payload projection, prune — **L** — **DONE 2026-11-10**

**Goal.** `vibe registry mirror --config mirror.toml` selects declaratively,
snapshots a template's inputs into the target's `_index/`, projects payloads
into the drop under a budget, and prunes only when told.

**As built.** All three commits are landed. **(a)** the
`--config mirror.toml` reader with every refusal, the selection model
replacing `Filters`, and the recorded selection in `_index/mirrors/<source>.toml`
(`AV-FMT-007`). **(b)** the `_index/templates/` snapshot writer with its
registered format `[format.index-templates]` and golden corpus, the priming by
which a network-denied client renders from a snapshot, and
`vibe app versions` reporting the snapshot's `[sync] last` (`AV-FMT-008`).
**(c)** the removal channel (`BatchConfig.removed` with the `Removal` /
`RemovedVersion` types, applied by all three backends so a pruned version's
bytes, its index rows and its `Event::Removed` fact leave in ONE commit), then
payload projection with its pre-write `max_total` budget, the `--dry-run`
counts and bytes, the `--prune` / `--prune-payloads` / `--yes` surface with
its plan, and `AV-FMT-009`'s two report arrays plus the `removed` count the
design did not name. Every fact of section 3 is `impl/done`; the twenty-seven
amendments are PROP-075 `#P4-AS-BUILT`.

The three that cost the most to settle, in short: **payload projection is the
installer's own Locator fetch** with a destination the caller chooses
(`vibe_app::fetch_declared_input`), so the digest check, the remap route and
the transfer wrapper are the ones every install already uses and the mirror is
not a new HTTP reader; **the projection reads the renders the snapshot stage
already performed**, kept only for a row with `payloads = true`, which is also
why the per-row facts commit (b) deliberately dropped had to come back as a
carrier; and **an unanswered prune plan refuses the whole run**, because a plan
is read against one reading of the target and applied against a possibly newer
one, so a run that printed a plan and then changed the tree it described would
have printed a document about nothing.

**What the removal channel decided:** the order is
**delete, then re-project**, because `vibe_index::layout::refresh` projects
the catalog from `version_dirs(root)` — a version still on disk is projected
straight back in, and the journal fact alone would not drop it. For a
`monorepo` the deleted paths must also be **staged** (`git add -A -- <path>`)
or the working tree loses the file and the commit keeps it; that is the one
backend where a removal can silently not happen, and it has its own test. A
removal path is refused unless it is a `<group>/<name>/v<version>` directory
that **spells the coordinate it claims**, or a path inside `_index/`; a path
the tree does not hold is dropped rather than refused, because a plan is read
against one state and applied against a possibly newer one. A removal
therefore takes the **version directory** and leaves an emptied
`<group>/<name>/` behind, which carries no version, no catalog row and no
journal fact (and on a `monorepo` is not even visible, since git tracks no
empty directory).

**Amendments decided against the code (read before implementing (b) or (c)).**

1. **`WRITER_DIRS` must *not* learn `templates`**, against what the file list
   below and `AV-FMT-008` say. `WRITER_DIRS` (`crates/vibe-index/src/index/mod.rs:63`)
   is the whitelist of what `Index::write_to` itself produces — consumed by
   `rebuild.rs:151` `catalog_file_set` and `index/memory/projection_compare.rs:37`
   `projection_matches`. The mirror writes the snapshot through
   `BatchConfig.files`, not through the index writer, so `_index/templates/`
   stays outside the whitelist exactly as `README.md`, `.gitignore` and
   `state/` do. Teaching the whitelist about it would make the projection
   comparison expect the index writer to produce a tree it never writes.
2. **The "Stale" risk below does not exist.** `listing.txt` is
   `render_listing(version_dirs, journal_digest)`
   (`crates/vibe-core/src/layout_registry.rs:135`) and `version_dirs` skips
   `_index`, so a new `_index/templates/` cannot move it. Close the risk with
   a test proving a registry that grows `_index/templates/` still reads
   `Fresh`, not with a projection change.
3. **No `templates` entry in `repomd.json` `files`** (central session,
   2026-11-10), against `AV-FMT-010`: the snapshot lives at a fixed path a
   client already knows, and teaching the projection a new first-class input
   is out of this phase's scope.
4. **`BatchConfig` gains a removal channel** (e.g. `removed: Vec<String>`),
   so prune's deletion, the index rewrite and the journal facts
   (`Event::Removed` already exists, `crates/vibe-index/src/journal/record.rs:70`)
   land in ONE commit — the amendment to the file list below that `#MIRROR-PRUNE`'s
   "rewritten in the same commit as the removal" actually requires.
5. **Snapshots only for directory-shaped sources.** `TemplateLayer::for_directory`
   is what reads a template out of a registry, so a `[[template]]` row against a
   per-package Git source is refused by name (`#TEMPLATE-DISCOVERY`). Landed
   with (a).
6. **`Filters::admits` / `Filters::packages()` was the whole seam**
   `mirror/source.rs` read, which is why the selection model is a superset of
   those two methods and listing did not change. Landed with (a).
7. **`versions.json`'s envelope is `CachedVersionList { source_url, sha256,
   fetched_at, document }`**; the snapshot restricts `document` to the selected
   versions, so `sha256` must be **recomputed over the filtered document**
   rather than carried over from the upstream fetch.

**Files.** `crates/vibe-cli/src/cli/registry.rs` (`--config` ✔, `--prune` ✔,
`--prune-payloads` ✔, `--yes` ✔); `crates/vibe-cli/src/commands/registry/mirror.rs`
and its `mirror/{history,plan,record,source,target}.rs` (the selection model
replacing `Filters` ✔, the recorded selection ✔); new `mirror/config.rs` (the
`mirror.toml` reader ✔) and `mirror/selection.rs` (what it means for a run ✔);
new `mirror/templates.rs` ✔ (snapshot writing through the existing
`BatchConfig.files` channel); new `mirror/payloads.rs` ✔ with
`payloads/report.rs` ✔ (the projection, the budget, the fetch, and both
answers a row owes); new `mirror/prune.rs` ✔ (the refusals, the plan, its
question and the drop's own removal); new `mirror/render.rs` ✔ (the report's
version rows and summary, moved out so the run file keeps its budget); new
`crates/vibe-app/src/installer/declared_input.rs` ✔ (the one public entry by
which a caller outside an install fetches a declared input through the
Locator);
`crates/vibe-publish/src/layout_publish.rs` (`BatchConfig`'s removal channel,
amendment 4) with `layout_publish/{index_plan,localdir,monorepo,working_copy}.rs`;
`crates/vibe-registry/src/templates/snapshot.rs` ✔ (the snapshot layout, read
and written from one place) with the priming hook in `templates/discovery.rs`
✔ (one mechanism, so the two document readers learn nothing);
`crates/vibe-cli/src/commands/app/versions.rs` ✔ (`snapshot_synced`, the one
field P5b declared in the wire and left unfilled);
`schemas/registry_mirror_report.jtd.json` (`templates[]`, `pruned[]`);
`formats/REGISTRY.toml` ✔ (`[format.index-templates]`) with
`formats/corpora/index-templates/e1` ✔. **Not**
`crates/vibe-index/src/index/mod.rs` and **not** the `repomd.json` `files`
projection — amendments 1 and 3.

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
**As delivered**, the payload and prune suites are
`crates/vibe-cli/tests/cli_registry_mirror_payloads.rs` (7) and
`…/cli_registry_mirror_prune.rs` (4), plus unit suites for the projection
arithmetic and the prune refusals. The payload fixture declares **its own**
template over its own four-kilobyte payloads and computes the digest documents
from them: upstream's recorded checksums describe real multi-megabyte
tarballs, so a suite that verifies a real digest of real bytes cannot reuse
them — and the fixture's template declares `size` on one option and not on the
other, which is the two halves of `#MIRROR-DRY-RUN-OUTPUT`. The closed-network
proof installs for real from the projected drop (a `sources` input, copied
rather than extracted), with the drop configured, no remap row to the release
host and `--offline`. The monorepo prune is checked against the **pushed bare
repository's** `ls-tree` and its committed journal, which is the only state
that tells a staged deletion from one that merely left the working tree.

**Acceptance.** Green 2026-11-10, the whole phase:
```sh
cargo test -p vibe-cli --test cli_registry_mirror --offline --locked
cargo test -p vibe-cli --test cli_registry_mirror_api --offline --locked
cargo test -p vibe-cli --test cli_registry_mirror_config --offline --locked
cargo test -p vibe-cli --test cli_registry_mirror_templates --offline --locked
cargo test -p vibe-cli --test cli_registry_mirror_payloads --offline --locked
cargo test -p vibe-cli --test cli_registry_mirror_prune --offline --locked
cargo test -p vibe-publish -p vibe-registry -p vibe-core --offline --locked
cargo xtask check-codegen && cargo xtask wire-diff
```

**Risks.** The test that a client installs from the snapshot **with the
network denied** is the only honest proof that the snapshot is complete —
`cli_registry_mirror_templates::a_client_with_only_the_target_and_no_network_renders_from_the_snapshot`,
green 2026-11-10: a fresh home, only the target registry configured, no
`[[remap]]` row and `--offline`, which leaves the snapshot as the only place
its inputs can come from.
*(The `listing.txt` / `Stale` risk this entry carried is withdrawn —
amendment 2 above: `version_dirs` skips `_index`, so a snapshot directory
cannot move the listing. The withdrawal is itself to be proven by a test.)*

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

### P5b — `vibe app versions` — **S** — *delivered 2026-11-10*

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

**As delivered.** The command exists and does exactly the goal sentence.
The library is `crates/vibe-app/src/versions.rs` with its unit suite
(`versions/tests.rs`, 11 tests); it joins a domain with this home's index
across one trait, `Domains`, which it never implements — so the join
reaches no registry, no index server and no socket, and `--installed` is
free of the network *by construction* rather than by care, because it is
handed a cache-only implementor. The CLI adapter is
`crates/vibe-cli/src/commands/app/versions.rs`, and the suite is
`crates/vibe-cli/tests/cli_app_versions.rs` (7 tests): the first three
drive the real `org.vibevm.node/node` template over upstream's own frozen
documents behind one `[[remap]]` row — origins (`template` /
`published (overrides template)`), the channel table, `--channel lts/jod`,
the refusal of `lts/argon`, the newest-20 trailer, `--limit`, `--all` —
and assert that **nothing was rendered**, which is the skeleton-path
claim as a check rather than a comment; the last four install real
`org.example` packages offline and assert the local states, the trailer,
the off slot, `--installed` answering with the registry deleted outright,
a slot with two packages under two headings, and the `unknown_target`
refusal. Two things the plan's file list did not foresee, both named in
`PROP-075#P5B-AS-BUILT`: the enumeration a listing needs is a **new**
read-only seam on the app resolver
(`versions_from_global_with_operation`,
`directory_roots_from_global_with_operation`,
`global_resolver`, all in `crates/vibe-app/src/resolve.rs`), because the
existing one only ever resolved *one* version; and four new refusal codes
join the `vibe app errors` table (`unknown_target`, `ambiguous_target`,
`unknown_channel`, `version_domain_unreadable`). `--refresh` and `--json`
are the root flags `AV-FMT-022` already delivered, not new ones on this
command. Two of the planned test cases changed shape: the plan asserts
`--installed` makes no network call "by a denying transport", and as
delivered it is asserted more strongly — the registry directory is
**deleted** before the listing runs, so there is nothing to read a domain
from at all; and "a snapshot-backed registry shows the snapshot's sync
time" cannot be written yet, because the `_index/templates/` tree it
reads is P4's to write. `snapshot_synced` is declared in the wire and
left unfilled until then, and that case belongs with P4's own suite. The
hand acceptance line is not run here: this machine's `node` slot is
whatever its own home holds, and the suite drives the same code through
the real binary.

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

### P7 — the owner's decisions of 2026-11-10, in six delegable units

**What changed.** P7 was one phase: the vibenvm shim plus its differential
suite, carrying the two general features the nvm mapping named. The owner's
review of 2026-11-10 changed its shape twice. It **approved twelve of the
fourteen proposed exclusions and rejected two** (PROP-075
`#VIBENVM-EXCLUSIONS-APPROVED`), so `--skip-default-packages` and
`$NVM_DIR/default-packages` are features the suite must cover rather than gaps
it may declare; and it **required that nothing added for nvm know anything
about npm or Node**, because the next pass replaces sdkman for Java and must
land on the same machinery. Two further general features were decided the same
day and are not demanded by nvm at all. So P7 is six ordered units, each a
delegable task with its own files, acceptance and risks. The arithmetic and
the correction of the campaign's "exactly three features" prediction are
PROP-075 `#VIBENVM-GAPS`.

**Sizes and parallelism at a glance.**

| unit | what | size | needs | may run beside |
|---|---|---|---|---|
| P7a | keyed version files + `--write-version-file` | **M** | — | P7d, P7e |
| P7b | ecosystem adapters (declaration + four verbs + `vibe app extras` verbs) | **L** | — | P7d, P7e |
| P7c | the user's extras list, apply-after-install, `--no-extras` | **M** | P7b, P7d | P7a |
| P7d | `[recommends]` for applications + the general `--no-recommends` | **M** | — | P7a, P7b, P7e |
| P7e | steps for ordinary packages (+ B-188) | **L** | — | P7a, P7b, P7c, P7d |
| P7f | vibenvm shim + the differential suite | **L** | P7a, P7b, P7c | — |

**Where the file conflicts are, so a parallel run is a decision and not a
hope.** P7a and P7b both touch
`crates/vibe-core/src/manifest/package/application.rs` and the clap definitions
under `crates/vibe-cli/src/cli/`, so running them together costs one
rebase of two files; P7c and P7d both touch
`crates/vibe-cli/src/commands/app/global.rs` (the flag surface) and
`crates/vibe-app/src/index/wire.rs` (the inventory move), which is why P7c
names P7d as a prerequisite rather than a neighbour. **P7e touches neither
`crates/vibe-app/` nor `crates/vibe-core/src/manifest/package/application*`**
— it lives in `crates/vibe-core/src/manifest/extension.rs`,
`crates/vibe-install/` and `crates/vibe-lifecycle/` — so it is the one unit
that is conflict-free against every other and is the best candidate to run
first in parallel with anything.

### P7a — keyed version files and `--write-version-file` — **M**

**Goal.** A third version-file format of `key`/`version` pairs, with
`tool-versions` reduced to its short spelling, and `--write-version-file` on
`vibe app use` and `vibe install -g` for both formats.

**Authority.** PROP-075 `#VERSION-FILE-KEYED`, `#VERSION-FILE-WRITE`;
`AV-FMT-025` and `AV-FMT-019` (amended in place).

**Files.** `crates/vibe-core/src/manifest/package/application/version_file.rs`
(the `keyed` variant with `separator` / `comment` / `extra`, and the
`tool-versions` → `keyed` expansion, which must happen **before** any reader
sees a row so there is one reader and one writer);
`…/version_file_validation.rs` (the four refusals, each naming its TOML path);
`crates/vibe-app/src/versionfile/formats.rs` (read a keyed file, ignore foreign
keys silently, take `extra` into account) and a new `…/versionfile/write.rs`
(the in-place single-line edit, the symlink refusal, the duplicate-key
refusal); `crates/vibe-app/src/versionfile/search.rs` (which file a write with
no value lands in); `crates/vibe-cli/src/cli/app_activation.rs` and
`crates/vibe-cli/src/cli/pkg.rs` (the flag on `app use` and on `install -g`);
`crates/vibe-cli/src/commands/app/alternatives.rs` and
`…/commands/app/global.rs` (the call and the report line).

**Commands.** `vibe app use node 22.23.4 --write-version-file`,
`vibe install -g org.vibevm.node/node@lts --write-version-file=.sdkmanrc`.

**Tests.** `crates/vibe-core` unit: `tool-versions` and its expanded `keyed`
row parse to the identical declared row (the no-drift proof); each refusal by
TOML path. `crates/vibe-app` unit: a `.sdkmanrc` holding `java=21` and
`maven=3.9.9` is read for `java` only, and writing `java` leaves the `maven`
line and the comments **byte-identical** — asserted as bytes, not as a parse;
a key absent from the file is appended; an absent file is created; a duplicate
key refuses naming both line numbers; a `.tool-versions` with two versions on
the key's line takes the first under `extra = "first"` and is unusable under
`refuse`. CLI fixture: the two commands above against a fixture package
declaring one `plain` and one `keyed` row.

**Acceptance.**
```sh
cd /fast/git/vibevm
cargo test -p vibe-core --offline --locked version_file
cargo test -p vibe-app --offline --locked versionfile
cargo test -p vibe-cli --offline --locked --test cli_app_version_files
```

**Risks.** (1) The byte-identity claim is the whole value of `keyed`, and a
writer that round-trips through a parser will quietly normalise spacing — the
test must compare bytes of the untouched lines. (2) `tool-versions` already
ships (`AV-FMT-011`) and the real Node package declares it, so the expansion
must keep that spelling working; a break here is invisible until a package is
installed. (3) Deciding the write target when no file exists is the one
genuinely ambiguous rule — it is decided in `#VERSION-FILE-WRITE` and must not
be re-decided in code.

### P7b — ecosystem adapters — **L**

**Goal.** `[[application.ecosystem]]` with its four verbs, the argv-only item
contract, the run record and work directory, the author-recommends shorthand
as synthesised features, the synthesised migration entry, and the explicit
`vibe app extras` verbs that do not need the user's list.

**Authority.** PROP-075 `#ECOSYSTEM-ADAPTER`, `#ECOSYSTEM-RECOMMENDS`,
`#ECOSYSTEM-MIGRATION`, `#PACKAGE-ACTIONS` (which records what is *not* built);
`AV-FMT-026`, `AV-FMT-027`, and the `describe` half of `AV-FMT-032`.

**Files.** `crates/vibe-core/src/manifest/package/application.rs` + a new
`…/application/ecosystem.rs` and `…/application/ecosystem_validation.rs` (the
rows, the `recipe`-only refusal for `install` / `remove`, the reserved
`ecosystem:` migration namespace, the recommends→feature synthesis);
`crates/vibe-app/src/steps/features.rs` (admit the synthesised rows in every
option); a new `crates/vibe-app/src/ecosystem/` module beside
`crates/vibe-app/src/migrate/` (the verb runner over `StepDecl`, the work
directory, the `VIBE_ECOSYSTEM_*` environment, the `list` file reader, the
`vibe-app-ecosystem/1` record); `crates/vibe-app/src/migrate.rs` (synthesise
the `ecosystem:<id>` entry beside the declared ones);
`crates/vibe-app/src/describe.rs`; `crates/vibe-cli/src/cli/app.rs` and a new
`crates/vibe-cli/src/commands/app/extras.rs`;
`schemas/application_ecosystem.jtd.json`, `schemas/application_extras.jtd.json`
and their `formats/REGISTRY.toml` records with golden corpora.

**Commands.** `vibe app extras list node --json`,
`vibe app extras remove node typescript`,
`vibe app extras self-upgrade node --ecosystem npm`,
`vibe app migrate node 22.23.4 24.22.2 --only ecosystem:npm`.

**Tests.** Manifest: a `shell` `install` verb refuses naming the field; two
rows with one `id` refuse; a recommends row naming an undeclared ecosystem
refuses; a recommends `id` colliding with an authored feature refuses; an
authored migration id in the `ecosystem:` namespace refuses. **The injection
test is the phase's own gate**: an item whose text is `; rm -rf $HOME` and one
containing a newline, a quote and a `$(…)` reach the recipe as exactly those
argv elements and nothing is evaluated — asserted by a fixture recipe that
writes `"$@"` one per line and a comparison against the input list. Runner:
the `list` file is read and trimmed; a missing `list` file is an empty list and
not an error; the work directory is kept on success and on failure and is
listed by `vibe app work`. Migration: the synthesised entry appears in
`vibe app describe`, runs `list` on the old version and `install` on the new
with the observed items, and `migration = false` suppresses it.

**Acceptance.**
```sh
cd /fast/git/vibevm
cargo test -p vibe-core --offline --locked ecosystem
cargo test -p vibe-app --offline --locked ecosystem
cargo test -p vibe-cli --offline --locked --test cli_app_extras
cargo xtask formats check     # the two new registry records and their corpora
```

**Risks.** (1) The feature synthesis touches the feature resolution of
PROP-065 `#feature-tree`, whose fixed-point rules are delicate: a synthesised
row must be an ordinary row in every respect or the resolution order changes
for packages that declare no adapter at all — the regression gate is the
existing feature-tree suite, run unchanged. (2) `install` must be `recipe`, and
the Node package's own adapter is the first real recipe to receive argv: a
`"$@"` mistake in the package is invisible to vibe's tests, so the fixture
recipe is the contract. (3) The synthesised migration entry is offered after
`install -g`, and `migration_offer` answers nothing when a slot has no earlier
version — correct for a migration, and the reason `#PACKAGE-ACTIONS` refuses to
express a maintenance act as one; do not "fix" it here.

### P7c — the user's extras list and apply-after-install — **M**

**Goal.** `[slot.<slot>.extras.<ecosystem>]` in `~/.vibe/config.toml`, the
final-list arithmetic, the apply after `install -g` / `update -g` of a new
version, `--no-extras`, `--no-recommends`'s ecosystem half, `vibe app extras
apply|add|drop|import`, and the failure that never touches the runtime.

**Authority.** PROP-075 `#SLOT-EXTRAS`; `AV-FMT-028`, `AV-FMT-029`.

**Files.** `crates/vibe-core/src/user_config.rs` (the new top-level `[slot]`
table — the struct is `deny_unknown_fields`, so without this a user who writes
the table gets a parse error); `crates/vibe-app/src/ecosystem/` (the final-list
computation and the apply entry point); `crates/vibe-app/src/index/wire.rs`
and `crates/vibe-app/src/index.rs` + `schemas/application/e7/index.jtd.json`
(the per-adapter record; the generation step is shared with P7d's weak edge);
`crates/vibe-cli/src/commands/app/global.rs` (the call after a successful
install, beside the existing `migrate::offer` call, and the two flags);
`crates/vibe-cli/src/commands/app/extras.rs`.

**Commands.** `vibe install -g org.vibevm.node/node@lts`,
`… --no-extras`, `… --no-recommends`, `vibe app extras apply node`,
`vibe app extras import node ~/.nvm/default-packages`.

**Tests.** The arithmetic, as a table-driven unit test: enabled author
recommendations, minus drops, plus `add`, de-duplicated first-wins — including
the case the owner's wording left ambiguous and the design decided, that a
`drop` is honoured **under `--no-extras`**. An unknown slot, ecosystem or
recommendation id in the settings is reported once and ignored. CLI fixture: a
failing `install` verb leaves the version installed, the slot default where it
was, exit status 0, one warning naming the retry command, and the outcome in
the record; `import` reads a `#`-commented file into `add`. **This phase
introduces the first read of `UserConfig` from `crates/vibe-app/`** — a test
must prove a home with no `[slot]` table behaves exactly as today.

**Acceptance.**
```sh
cd /fast/git/vibevm
cargo test -p vibe-core --offline --locked user_config
cargo test -p vibe-app --offline --locked extras
cargo test -p vibe-cli --offline --locked --test cli_app_extras_apply
```

**Risks.** (1) `crates/vibe-app/` reads no user config today; adding that seam
in a crate whose state is otherwise entirely its own settings root is the
phase's real architectural cost, and the alternative (a slot-keyed file beside
`applications/index.json`) must be rejected deliberately, not by accident —
the owner said `~/.vibe` settings. (2) The inventory move to
`vibe-user-applications/8` needs the `vibe-app-proxy` admission PROP-059
`#proxy-index-upgrade` requires, which `AV-FMT-017` already paid once for `/7`;
budget for it. (3) Applying extras after an install makes the first install of
a runtime do ecosystem work, which lengthens it visibly — the step panel must
show it as its own rows, or users will read the pause as a hang.

### P7d — `[recommends]` for applications and the general `--no-recommends` — **M**

**Goal.** An application's `[recommends]` installed best-effort by the
application installer; `--no-recommends` as one global flag honoured by every
plane that solves or installs; `[suggests]` reported and never installed.

**Authority.** PROP-077 (`#APP-RECOMMENDS`, `#APP-RECOMMENDS-MECHANICS`,
`#APP-RECOMMENDS-CONSENT`, `#NO-RECOMMENDS-FLAG`, `#SUGGESTS-UNCHANGED`);
`AV-FMT-030`, and the weak edge of `AV-FMT-029`.

**Files.** `crates/vibe-cli/src/cli.rs` (the global flag) and
`crates/vibe-cli/src/commands/app/global.rs` (its refusal where nothing
solves); `crates/vibe-resolver/src/resolvo_engine/mod.rs` and
`crates/vibe-resolver/src/source_solve.rs` (skip the greedy weak expansion —
the only resolver change, and it is a branch, not a rewrite);
`crates/vibe-app/src/operations/install.rs` and `…/operations/mod.rs` (the
breadth-first walk after `commit`, the visited set, the `auto` + weak edge);
`crates/vibe-app/src/index/wire.rs` + `schemas/application/e7/index.jtd.json`
(the weak attribute); `crates/vibe-app/src/steps/plan_view.rs` and
`crates/vibe-cli/src/commands/app/plan_text.rs` (the recommended subtree in
`--plan-only`); `schemas/install_report.jtd.json`.

**Commands.** `vibe install -g org.example/jdk` (brings maven),
`… --no-recommends`, `vibe install --no-recommends` (a project),
`vibe install -g … --plan-only`.

**Tests.** A fixture app recommending a second fixture app installs both, the
second recorded `auto` with a weak edge; `--no-recommends` installs one.
Each skip reason is its own case: not `kind = "app"`, no option for this
platform, unsatisfiable, absent registry, a failing recommended install — each
reported, each leaving exit 0. Transitivity with a cycle (`a` recommends `b`,
`b` recommends `a`) terminates and installs both once. Removal: `uninstall -g`
of the recommender succeeds and reports what it leaves; `uninstall -g` of the
recommended succeeds although the recommender is installed; `autoremove` takes
an orphaned recommendation and never one the user installed by name. Project
plane: `--no-recommends` changes the solve and **not** a lock replay — asserted
by installing with the flag, then installing again without it from the same
lock and getting the same tree.

**Acceptance.**
```sh
cd /fast/git/vibevm
cargo test -p vibe-resolver --offline --locked recommends
cargo test -p vibe-app --offline --locked recommends
cargo test -p vibe-cli --offline --locked --test cli_app_recommends
cargo test -p vibe-cli --offline --locked --test cli_install_no_recommends
```

**Risks.** (1) A recommendation is an install the user did not name; the plan
and the report are the whole mitigation, and a run that installs something it
did not print first is a defect of this phase, not a cosmetic one. (2) The
weak edge rides the same inventory generation as P7c's adapter record — one
generation step, two phases, so whichever lands first owns the `e7` schema and
the proxy admission. (3) The project plane's arbitrary-code consent gate must
cover a recommended package exactly as it covers a named one; a recommendation
that walks past a consent the direct request would have needed is the one
unacceptable outcome.

### P7e — structured steps for ordinary packages — **L**

**Goal.** An ordinary package may declare several named things to run when it
is installed, with its prerequisites checked first, through PROP-054's `slot:`
points and no second grammar; `B-188` is the consumer and the acceptance.

**Authority.** PROP-065 `#ordinary-reach`, `#ordinary-grammar`,
`#ordinary-plan`, `#ordinary-failure`, `#ordinary-consumer`; PROP-054
`#SLOT-CONTRIBUTION-JUDGMENT`; `AV-FMT-031`, `AV-FMT-032`.

**Files.** `crates/vibe-core/src/manifest/extension.rs` (the four new optional
fields and the fields refused by name on a `slot:` point);
`crates/vibe-core/src/lifecycle/point.rs` (unchanged — the point namespace
stays closed, and a test should assert that);
a new shared judgment module (the condition evaluator, the needs preflight and
the plan projection) extracted from `crates/vibe-app/src/steps/plan.rs` so both
planes call one implementation — **this extraction is the phase's main
structural act and should be its own commit**;
`crates/vibe-install/src/lifecycle.rs` and `…/lifecycle/plan.rs` (carry the
four fields through the synthetic-contribution path `[hooks]` already uses);
`crates/vibe-install/src/apply.rs` (the preflight refusal before step 8, the
panel rows, the evidence directory);
`crates/vibe-cli/src/commands/install/report.rs` and `…/install/draft.rs`;
`schemas/install_steps.jtd.json` + its registry record and corpus.

**Commands.** `vibe install --plan-only` (shows the `slot:` rows),
`vibe install org.vibevm.ai-native/rust-ai-native`,
`vibe reinstall org.vibevm.ai-native/rust-ai-native` (the retry).

**Tests.** The refusals, each naming its field: `requires`, `phase`, `result`,
and a `$out` / `$sources` / `$app_root` template on a `slot:` contribution.
`needs` preflight: a contribution needing an absent command refuses the
package's installation **before the slot is populated**, naming the
contribution and the need; an `optional` one with an unmet need is skipped and
reported. Conditions: `when = { feature }` reads the package-scope
configuration of PROP-069 and a false condition reports `skip` with the
deciding condition. Failure: a failing `pre-install` contribution rolls the
slot back and registers nothing; a failing `post-install` leaves the package
installed and flagged; an interrupt marks the contribution `interrupted`,
marks nothing `done`, and runs nothing after it. **Regression gate**: the whole
existing `[hooks]` suite runs unchanged, because `[hooks]` is the one-field
spelling of the same thing. **B-188 fixture**: a package declaring two
`slot:post-install` contributions with `needs` and `title`s produces the plan,
the two logs and the `vibe-install-steps/1` record.

**Acceptance.**
```sh
cd /fast/git/vibevm
cargo test -p vibe-core --offline --locked extension
cargo test -p vibe-install --offline --locked lifecycle
cargo test -p vibe-cli --offline --locked --test cli_install_slot_steps
cargo test -p vibe-workspace --offline --locked hooks   # the unchanged [hooks] gate
```

**Risks.** (1) The shared judgment module is an extraction out of a crate whose
step planner is entangled with installation keys and payload roots; if the
extraction cannot be made clean, the honest fallback is a second small
evaluator with a shared test corpus, and that decision belongs in this phase's
report rather than in silence. (2) `[hooks]` is in use today and its failure
law differs from PROP-065's; carrying it unchanged is the acceptance, and the
temptation to unify the two failure laws must be refused. (3) `needs`
refusing an installation up front turns a late failure into an early refusal —
better, but it will refuse installs that "worked" before by failing late, so
the release note must say so.

### P7f — vibenvm and the algorithmic differential proof — **L**

**Goal.** `org.vibevm.node/vibenvm` plus a committed, rerunnable suite whose
exit code is the verdict, with complete coverage of the pinned inventory and
exactly the twelve owner-approved exclusions.

**Authority.** PROP-075 `#VIBENVM`, `#VIBENVM-ALGORITHMIC-PROOF`,
`#VIBENVM-PIN`, `#VIBENVM-MAPPING`, `#VIBENVM-SUITE`, `#VIBENVM-EXCLUSIONS`,
`#VIBENVM-EXCLUSIONS-APPROVED`; `AV-FMT-020`.

**Files.** New `conformance/vibenvm/` in this repository:
`run.sh`, `fetch-mirror.sh`, `mirror.lock`, `extract-inventory.sh`,
`coverage.py`, `normalise.rules`, `observe.sh`, `exclusions.toml`,
`nvm-tests.allow`, `cases/*.toml`, `README.md`, and a `.gitignore` for
`mirror/`; `xtask/src/main.rs` + a new `xtask/src/vibenvm_conform.rs`
(`cargo xtask vibenvm-conform`). The shim package itself is a new repository
`/fast/git/vibe-pkg-vibenvm` (`nvm.sh`-shaped `vibenvm.sh`, a `[[application]]`
of kind `app` with no `path_dirs`, installed by `vibe install -g`). The real
`org.vibevm.node/node` package at `/fast/git/vibe-pkg-node` gains the npm
adapter of P7b and **drops its hand-written `npm-globals` migration entry**.

**The twelve approved exclusion rows**, written with `approved_by = "owner"`
and `approved = "2026-11-10"` and no others: the five alias rows (`alias`,
`unalias`, `install --alias=`, `ls --no-alias`, `$NVM_DIR/alias/*`), the two
source-build rows (`install -s` / `-b`, `install -j`), the two category and
internal-state rows (`install --only=` written as `subcommand = "*"`, and
`$NVM_ONLY` with `.lts-status`, the install locks, `$NVM_AUTH_HEADER` and the
third-party hook written as `(environment)` / `(internal)`), and the three
hygiene rows (`use --delete-prefix`, `set-colors`, `debug`). The three row
rules the file needs — one row per extracted pair, `subcommand = "*"`, and a
recorded-but-not-required `(environment)` row — are
`#VIBENVM-EXCLUSIONS-APPROVED` and belong in `coverage.py`.

**Commands.** `cargo xtask vibenvm-conform [--cases GLOB] [--update-inventory]`.

**Tests.** The suite *is* the test. Its own meta-tests, which must exist or the
verdict is not trustworthy: a deliberately wrong shim fails with exit 1 naming
the field; a deliberately uncovered flag fails with exit 2 naming the pair; an
exclusion row without `approved_by = "owner"` fails with exit 2; a thirteenth
exclusion row fails with exit 2; a tampered `mirror.lock` fails with exit 3; a
non-loopback connection attempt fails with exit 3. New differential cases the
owner's rejections create: `install --skip-default-packages` against
`--no-extras`, and a `$NVM_DIR/default-packages` file against
`vibe app extras import` + the user's own list, compared through the `globals`
observation in both sandboxes.

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
default-packages cases are new and they are the only ones that exercise an
ecosystem adapter end to end against a real `npm install -g`, so they are also
the slowest; they need their own `wall_secs` budget and a small item list.
(3) A differential suite is only as strong as its observation list; the list is
the owner's own, and adding to it later is cheap because the comparison is
field-by-field. (4) Linux only, stated as such — this is not a cross-platform
claim. (5) The `--only=<categories>` risk the previous plan carried is
**withdrawn**: the owner approved the exclusion, so no "version class" feature
is built.

**Parallel.** Needs P2, P3, P5a, P5b, P6, P7a, P7b, P7c. Last by construction.

### sdkman-readiness checklist — what the Java pass will need

The owner's binding requirement was that nothing in P7 may block the sdkman
replacement. This checklist is that claim, made checkable. None of these is
P7 work; each names the P7 decision that leaves room for it.

| what Java needs | status after P7 |
|---|---|
| **Vendor inside the version** (`21.0.4-tem`, `17-zulu`, `temurin-21`): a default vendor, and translation of sdkman's identifiers | **Not blocked, not built.** A produced version is semver after `strip_prefix` (`#TEMPLATE-SOURCE-KINDS`), and a vendor is expressible today as a prerelease/build segment or as separate packages per vendor; a first-class vendor axis is a later amendment of that fact and touches no P7 decision. |
| **`JAVA_HOME` and other `*_HOME`**, not only PATH; macOS `java_home` | **Not blocked.** `[[installer_options.steps]]` already has `kind = "env"` with `set` / `path_prepend` over `$app_root` (PROP-065 `#env-step`), which is exactly a `*_HOME`; `vibe app env` prints per-shell lines for any variable, not only PATH (`#ACTIVATION-ENV-OUTPUT`). Nothing in P7 narrows either. |
| **Registering an already-installed JDK** under a name (`sdk install java x /path`, `jenv add`), whose removal must not delete the files | **Not blocked.** `result = { external = "<app_root>" }` already registers a root vibe does not own, and PROP-065 `#uninstall-tree` already states that removal never touches files outside the payload. What is missing is only the command spelling, and no P7 decision occupies it. |
| **Several candidates** (java, maven, gradle, kotlin, scala, sbt), each with its own `*_HOME`, several named in one `.sdkmanrc` | **Delivered by P7a.** `format = "keyed"` exists precisely for this, and `#VERSION-FILE-KEYED` states that one file resolves several slots from their own keys while `#VERSION-FILE-WRITE` edits one line in place so two slots can share the file. |
| **`.sdkmanrc` read and written** | **Delivered by P7a**, as the `keyed` format's first real consumer. |
| **No ecosystem adapter** (Java has no bundled package manager; default packages and self-upgrade are not wanted) | **Correct by construction.** `[[application.ecosystem]]` is optional and a package that declares none is the test that the mechanism is optional (`#ECOSYSTEM-ADAPTER`); `#SLOT-EXTRAS` applies per declared adapter and does nothing where there is none. |
| **Companions installed together** (sdkman's java + maven + gradle) | **Delivered by P7d.** `[recommends]` on an application is exactly "maven comes with the JDK", with `--no-recommends` as the decline (PROP-077 `#APP-RECOMMENDS`). |
| `sdk env install` / `clear`, offline mode, cache cleaning, "what is outdated" | **Not blocked.** Each maps onto an existing surface — the version-file resolution of `#VERSION-FILE-FORMATS`, the global `--offline`, `vibe cache`, `vibe app outdated` — and none is touched by a P7 decision. |

**Conclusion, stated so the next pass can rely on it:** no P7a–P7f decision
blocks the Java pass, and two of them (the `keyed` version-file format and
application `[recommends]`) exist in their general form *because* Java needs
them. The one thing the Java pass must decide for itself is the vendor axis,
and it is deliberately left open rather than guessed at here.

### Dependency order, as a graph

```
P0 spikes
  |
  +--> P1 ----+--> P2 --+--> P3 --+
  |           |         |         |
  |           |         +--> P4   |
  |           |         |         |
  |           |         +--> P5b -+
  +--> P5a ---------------------- +--> P7a --+
  +--> P6 ----------------------- +--> P7b --+--> P7c --> P7f
                                  +--> P7d --+
                                  +--> P7e      (joins no one)
```

P1, P5a and P6 may run concurrently from the start. P3, P4 and P5b may run
concurrently once P2 lands. **P1 through P6 are delivered.** Of the six units
of P7, **P7a, P7b, P7d and P7e may all start at once**; P7c needs P7b and P7d
(the inventory generation and the flag), and P7f needs P7a, P7b and P7c. P7e
joins nothing: it is the only unit that touches neither `crates/vibe-app/` nor
the application manifest tables, so it never conflicts and never blocks.

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
  general machinery.* **Half held, half falsified, 2026-11-10, before a case
  was written — by the owner's review rather than by the code.** The absorption
  held: nvm's surface still yields three gaps and nothing else, and every other
  row of the mapping is `maps` or an owner-approved `exclude`. The *count of
  features* did not: the owner rejected two proposed exclusions (so the third
  gap is the default-packages pair) and required that nothing added for nvm
  know anything about npm, which turns three gaps into four general features —
  and then added two more features nvm does not ask for at all. Phase P7's
  total is six, and `PROP-075#VIBENVM-GAPS` carries the corrected arithmetic
  rather than a reinterpretation of this sentence. The prediction's successor,
  for P7f to answer: **no seventh feature is discovered while writing the
  cases.**

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

To be written in P7f as `campaigns/app-versions-2026-11/accept.sh`. It runs, in
one network-isolated sandbox with a fresh HOME: install Node from the template
at a channel; `vibe app versions`; switch with `vibe app env` and with the
prompt hook; `vibe app exec` at a second version; migrate the global npm
packages between them; mirror the template into a local registry with
`mirror.toml` and install from the snapshot with the network denied; and
finally `cargo xtask vibenvm-conform`. It exits non-zero on any step's
failure, any non-loopback connection attempt, and any unverified artifact use.
The owner's decisions of 2026-11-10 add four legs to it, one per new feature
the user can see: writing a `keyed` version file and reading it back for two
slots; applying a user extras list to a newly installed version and observing
it through the adapter's own `list`; installing an application that recommends
another and declining it with `--no-recommends`; and installing an ordinary
package whose `slot:post-install` contributions appear in `--plan-only` and
refuse up front on a missing `needs`.

## 8. Open points

The design's open questions are
`spec://org.vibevm.core/vibevm/common/PROP-075#OWNER-QUESTIONS`, each with the
default it takes unless the owner objects: (Q1) the fourteen proposed nvm
exclusions needed row-by-row owner approval, because an unapproved row is a
gate failure by construction — **answered 2026-11-10: twelve approved, two
rejected**, and the rejected pair became the ecosystem-adapter and slot-extras
feature (`PROP-075#VIBENVM-EXCLUSIONS-APPROVED`); (Q2) `--only=<categories>`
was the largest exclusion — **answered 2026-11-10: excluded as proposed**, so
no general "version class" feature is built and P7's risk of growing by a
phase is withdrawn; (Q3) the 15-minute version-list TTL; (Q4) templates
undiscoverable in an unindexed per-package git registry; (Q5) no
`--trust-template` escape hatch for the changed-digest alarm. Q3, Q4 and Q5
stand on their defaults and block nothing; nothing blocks a phase any more.
Four new questions, all P7d's and none blocking, are `PROP-077#WEAK-OPEN`.

## 9. Log

See `LOG.md`.

## 10. Report

Empty until close.
