# App-versions campaign: format and behaviour changes

Append-only log of user-visible changes made by the campaign in `PLAN.md`, material for release notes. Entries are `AV-FMT-NNN`; numbers are never reused.

Regime at writing (`formats/EPOCHS.toml`, 2026-11-10): `public = false`, `break_window_open = true` — breaking is free and unmigrated, the standing user recipe is «regenerate / re-init / re-fetch», and no break note under `formats/breaks/` is required. Every entry below still states the format registry record it touches and whether its epoch moves, because `FormatId` cannot express an unregistered format.

Design authority for all of it: `spec://org.vibevm.core/vibevm/common/PROP-075#FORMAT-CHANGES`.

## AV-FMT-001 — `[package] template` and the `[template]` table (Phase 1)

Status: **delivered 2026-11-10** (phase P1). As implemented it matches the
entry below, with four clarifications recorded in
`spec://org.vibevm.core/vibevm/common/PROP-075#TEMPLATE-DOCUMENT-READING`:
the render rewrites `[package].version` to the produced version and removes
`[package].template`, the `[template]` table and every `template_optional`
key, so a rendered manifest carries no template vocabulary; a template
document is read as TOML text and its `[template]` table deserialised on its
own, because the numeric-coercion spelling (`size = "{size:…}"`) puts a string
where the typed manifest wants a number; a skeleton render writes sixty-four
zeros into every digest-bearing field, which is a valid digest and so needs no
validator exemption; and `{meta.PATH}` admits a string, an integer or a
boolean, refusing an array, a table, `null` and a non-integral number by
naming the path. `[template] version_list_ttl` is admitted as a
`<count><s|m|h>` duration on the table itself (the user-config default stays
AV-FMT-005, phase P2).

- `[format.manifest]` (`schema = "none"`, authored TOML, `foreign_parsers = "many"`), epoch **1 → 1**. Every addition is optional and omitted when absent, so existing manifests serialise byte-identically.
- New `[package] template` boolean, default `false`. Admitted only on `kind = "app"` in this campaign; any other kind refuses with a message naming the restriction. Touches `PackageMeta`, `PackageMetaWire` (`deny_unknown_fields`) and both `From` arms.
- New top-level table `[template]`, which must be added to `Manifest`, to `ManifestWire` (`deny_unknown_fields`) and to both `TryFrom` arms, or the table is refused as unknown. Members: `[template.versions]` (`kind = "json" | "list"`, with `url`, `select`, `version_field`, `strip_prefix`, `min`, `max`, `prerelease` for `json`, and `[[template.versions.item]]` rows of `version` + free `meta` for `list`), `[template.digests]` (`kind = "sha256sums" | "json"`, with `url` and, for `json`, `select`, `file_field`, `digest_field`, `size_field`), `[[template.channel]]` rows (`rule = "truthy"` with `name`, or `rule = "value"` with `prefix`; both with `from`), and optional `version_list_ttl`.
- New optional `template_optional` boolean on `[[installer_options]]`: such a row is dropped from the rendered manifest when a digest variable it references has no value for that version; without it, a missing digest is a render error.
- Refusals, each naming its field as a TOML path (PROP-059 `#declaration`): `[template]` without `template = true`; `template = true` without `[template]`; `template = true` on a non-`app` kind; a digest variable outside the digest-bearing fields; `{platform}` / `{arch}` outside an `[[installer_options]]` row; a `{meta.…}` path resolving to an array or object; a `{size:…}` variable with a digest source that carries no sizes; a `{` opening no known variable.
- `vibe check` gains one check, `template_render`: a template whose skeleton render does not validate as an ordinary manifest is an error naming the rendered field. The check is **offline by construction**, so it renders a probe version — the newest declared `[[template.versions.item]]` of a `list` source, the declared `min` of a `json` source, else `1.0.0` — and a template whose grammar needs a `{meta.…}` value only the fetched version document carries is reported *unchecked* (a warning), never broken.
- Rendering itself is not a format: it is a pure function in the new crate `vibe-template`, producing TOML text that the ordinary `Manifest::parse_str` reads. Its purity is a test, not a comment: `crates/vibe-template/tests/purity.rs` pins the crate's dependency table to a declared list and refuses `std::fs`, `std::net`, `std::env`, `std::process`, `std::time`, `include_str!`, `vibe_net`, `vibe_registry` and `vibe_resolver` anywhere in its sources.
- **Visible loss, the same one AV-FMT-006 records:** a template over a `sha256sums` digest source cannot carry `size`, so a manifest rendered from the Node template is the committed one minus its `size` lines. `{size:FILE}` with a source that carries no sizes is a refusal rather than a zero.

## AV-FMT-002 — lockfile: `[template]` per package, `via_channel`, schema 10 (Phase 2)

Status: **planned**.

- `[format.lockfile]` (`schema = "none"`, authored TOML, `foreign_parsers = "ours"`), registry epoch **1 → 1**, following the precedent of the 7→8→9 steps, which did not move it. The lockfile's own `meta.schema_version` goes **9 → 10** and `admit` accepts **7..=10**; 11 and above still refuse.
- `LockedPackage` gains an optional table `template` with `revision` (the template package's own version), `rendered_sha256` (SHA-256 of the full render's TOML bytes), `digest_source_sha256` (SHA-256 of the fetched digest document's bytes), and `source_url` / `digest_url` as resolved **after** remap.
- `LockedPackage` gains an optional `via_channel` naming the channel a requirement was lowered through, so `vibe.toml` keeps saying `lts` while `vibe.lock` says the concrete version and `vibe update` re-lowers.
- Both fields are omitted when absent, so a lock with no template package is byte-identical except for `schema_version`.
- Behaviour: before a locked version is installed, updated, reinstalled or modified, the full render is recomputed and `rendered_sha256` compared; a difference refuses before any effect with code `template_render_changed`, naming the package, the version, the template revision then and now, both hashes, and the digest-bearing fields that differ. There is **no** `--trust-template` flag. A changed template *revision* alone, with the same digests, is not an alarm: the recorded revision is updated in place with a note.

## AV-FMT-003 — index per-version record: `template`, `channels` (Phase 2)

Status: **planned**.

- `schemas/index/e1/entry.jtd.json` is a bare `ref` to the **shared vocabulary** `version_entry`, so the two new optional members reach `primary.jsonl`, `by-name/<name>.json` and the journal at once. Records touched: `[format.index-primary]`, `[format.index-by-name]`, `[format.index-entry]`, all epoch **1 → 1** (`recoverable = true`, `foreign_parsers = "many"`): the members are optional and absent means byte-identical documents.
- New optional `template: boolean` — the published version is a template revision rather than a concrete version. This is how a client splits a package's published versions in one index read and finds the active template (the newest such revision), without reading a manifest per version.
- New optional `channels: string[]` — publisher-assigned labels on a concrete version, so resolving a channel for a non-template package is an index read and never a scan of every manifest.
- Written by publish and by the layout projection (`vibe_index::layout::project`, which derives both from the manifests on disk, so an indexed registry and a layout registry agree by construction).
- One new golden document each under `formats/corpora/index/e1` exercising the members; the existing corpus bytes must not move (verified by `cargo xtask rebuild --check`).
- Consequence, stated as a limitation: a template is **not** discoverable in a per-package git registry with no index, where listing versions is a tag list. Such a registry refuses with `template_index_required`, naming the registry and the fix.

## AV-FMT-004 — two remap artifact kinds: `template-index`, `template-digest` (Phase 2)

Status: **planned**.

- PROP-066 `#locations` artifact-kind vocabulary, epoch **1 → 1** (one open vocabulary gains two members).
- A template's version-list document and its per-version digest document are fetched through the operation's controlled transport and the remap Locator like every other byte, so a closed network redirects them with the rows it already uses and `vibe remap explain <url>` answers for them. The payload inputs of a rendered version keep the existing `app-input` kind and the drop convention of PROP-066 `#drops` unchanged.
- No new routing mechanism and no new config table: this is two vocabulary members, exactly as `distribution` and `app-input` are.

## AV-FMT-005 — `~/.vibe/config.toml` `[template] version_list_ttl` (Phase 2)

Status: **planned**.

- User config (`schema = "none"`, authored TOML), epoch **1 → 1**. One new table with one key.
- `version_list_ttl` — how long a cached template version list is used before it is refetched. Default **15 minutes**. Digest documents are **never** revalidated (a released version's checksums are immutable, and a change upstream is precisely the AV-FMT-002 alarm), so the TTL applies to the version list alone.
- Cache layout, new: `<settings>/cache/templates/<group>/<name>/versions.json`, `…/digests/<version>`, `…/rendered/v<version>/`. `--refresh` on any resolving command refetches the list; `--offline` / `VIBE_OFFLINE` / `[net] offline` read the cache whatever its age and refuse a miss with `template_cache_missing`, naming the package, the exact path and the command that would fill it.

## AV-FMT-006 — `org.vibevm.node/node` becomes a template package (Phase 3)

Status: **planned**. Package-side only; no vibe format changes.

- `/fast/git/vibe-pkg-node` is republished as one template package at a new template revision, with `[template.versions]` pointing at `https://nodejs.org/dist/index.json` (`select = ""`, `version_field = "version"`, `strip_prefix = "v"`), `[template.digests]` at `https://nodejs.org/dist/v{version}/SHASUMS256.txt`, and two `[[template.channel]]` rows giving `lts` (`rule = "truthy"`) and `lts/<codename>` (`rule = "value"`). The seven installer options stay as they are written today, with `linux-musl-x64` and `windows-arm64` marked `template_optional = true` because upstream's `files[]` does not carry them for every version.
- The two existing concrete tags (`v24.22.2`, `v22.23.4`) stay published, so "a concrete version wins over the template's rendering of the same version" is exercised in production from the first day.
- **Visible loss, deliberate:** the rendered manifests no longer carry `size` on their archive steps, because nodejs.org publishes digests but not sizes and the retired generator took sizes from a hand-made `SIZES` file. The step panel falls back to the server's `Content-Length`, which PROP-065 `#sizes` / `#display` already allow, and `vibe registry mirror --dry-run` reports its byte total as a lower bound and says so.
- `org.vibevm.node/vibevm-package-generator` is superseded and is not republished; its repository records that.
- `org.vibevm.codkit/codkit` is **untouched** by this campaign.

## AV-FMT-007 — `mirror.toml`, declarative mirror selection, recorded selection (Phase 4)

Status: **planned**.

- New authored document `mirror.toml` (`schema = 1`; any other value refuses, as `_index/mirrors/<source>.toml` already does). `[select]` with `all`, `groups`, `packages`, `exclude`; one `[[template]]` row per template package with `package`, `versions`, `channels`, `newest`, `platforms`, `archs`, `installers`, `payloads`; `[payloads]` with `max_total` and `into`.
- `vibe registry mirror` gains `--config <file>`. Today's `--package` / `--group` keep working and **override** `[select]` wholesale rather than intersecting with it. `all = false` with no groups and no packages is a usage refusal, never a silent empty run. This generalises today's `Filters { packages, groups }` (exact matching, no excludes, no version ranges, no newest-N).
- `_index/mirrors/<source>.toml` gains a `[select]` table and `[[template]]` rows beside its existing `[source]`, `[sync]` and `[[version]]`; its `schema` stays **1** because the tables are additive and a reader that ignores them behaves as today. A later bare `vibe registry mirror <source> --into <target>` repeats the recorded selection and adds only what newly matches.
- `vibe registry mirror` gains `--prune`, `--prune-payloads` and `--yes`. A prune is always a plan first (one row per removal with its reason) and removes nothing without a yes; `--dry-run --prune` prints the plan and stops. A version recorded as copied from a *different* source is never pruned by this source's run. A pruned version's index rows and journal facts are rewritten in the same commit, so the target never advertises bytes it no longer has. No run without `--prune` ever deletes anything, however narrow the selection became.
- `[payloads] into` defaults to the first configured `[artifacts] packages` drop; writing payloads into the registry target is **refused by name**. Payloads are written at the first rung of the existing drop lookup, `<into>/<group>/<name>/<version>/<file>`, and verified against the digest the render produced. An input already present with the right digest is skipped; one present with a wrong digest stops the run with `digest_mismatch` and is not replaced. `max_total` is checked **before the first byte is written** and over-budget stops with `payload_budget_exceeded`, naming the budget, the projected total and what does not fit.

## AV-FMT-008 — template snapshot under `_index/templates/` (Phase 4)

Status: **planned**. New format.

- New record `[format.index-templates]`, epoch **1**, `schema = "schemas/index/e1/templates.jtd.json"`, `recoverable = true`, `foreign_parsers = "many"`, corpus `formats/corpora/index/e1`, `sunset = "none"`.
- Layout under the target registry root: `_index/templates/<group>/<name>/snapshot.toml` (`schema = 1`), `…/versions.json` (the upstream source document restricted to the selected versions, in the same envelope the client cache uses, with the record array filtered and nothing else altered), and `…/digests/<version>` (the upstream digest document verbatim).
- `snapshot.toml` records `[source]` (the template coordinate and revision, the version-list url and the digest url template as declared, before remap), `[sync]` (the writing vibe and the first and last sync time), and one `[[version]]` per snapshotted version with `version`, `digest_sha256` and `rendered_sha256`.
- Written inside the mirror's single commit through the existing `BatchConfig.files` channel, so the snapshot's digests are fixed by the git commit itself.
- A client never revalidates a snapshot and never contacts upstream from one. `vibe app versions` reports the snapshot's `[sync] last` so a user can see how old the list is.
- A re-run adds versions and leaves existing ones untouched; a `digest_sha256` that changed for an already-snapshotted version stops the run before anything is written, exactly as PROP-002 `#MIRROR-RULES` stops on a changed hash of a frozen version.
- `_index/templates/` is a new directory the index writer must own: `WRITER_DIRS` (`crates/vibe-index/src/index/mod.rs`) learns it, and the layout projection must know it so the index does not read `Stale`.

## AV-FMT-009 — mirror report: `templates[]`, `pruned[]` (Phase 4)

Status: **planned**.

- `[format.cli-registry-mirror-report]`, `schemas/registry_mirror_report.jtd.json`, epoch **1 → 1**. Both arrays are optional; every existing member is unchanged, including the `command: "registry:mirror"` discriminator and the absence of a `protocol` member.
- `templates[]`: per `[[template]]` row — the template coordinate and revision, the versions selected with the first and last of them, the channels that resolved and to what, the installer options kept per version, the rows dropped by `template_optional` with the missing file, and, with payloads, the input count, the bytes already present in the drop, the bytes to fetch, the total and a flag for every input of unknown size.
- `pruned[]`: per removal — what was removed, from where, and the reason that reached it.
- Human output draws a template row as one node with its versions as children, in the display language of PROP-067 as PROP-065 `#plan` does.

## AV-FMT-010 — `repomd.json` `files` gains the `_index/templates/` key (Phase 4)

Status: **planned**.

- `[format.index-repomd]`, `schemas/index/e1/repomd.jtd.json`, epoch **1 → 1**. The `files` member is an open map and gains one more key, exactly as it carries `by-name/` today; no shape change, no member added or removed. Note the existing break-003 rule that `file.size` is a decimal **string**.

## AV-FMT-011 — `[[application.version_file]]` (Phase 5a)

Status: **planned**.

- `[format.manifest]`, epoch **1 → 1**. Optional rows; absent means byte-identical.
- `format = "plain"` with `names` (an ordered array of file names, `[".nvmrc", ".node-version"]`): the first non-empty, non-comment line, trimmed, with `#` and a CR tolerated, read as a version requirement **or** a channel name.
- `format = "tool-versions"` with `key` (the package's key in asdf's `.tool-versions`, `nodejs`): whitespace-separated `<key> <version>…` lines, `#` comments ignored, the first version for the key taken and the rest reported as unused.
- Nothing else is a format: `.python-version` is a `plain` row, and `package.json` `engines` is not read.
- Precedence, highest first: an explicit `--use SLOT=VER` / `SLOT@VER`; the project's `vibe.toml` requirement on that application package; the nearest declared version file, searched from the working directory upward and stopping at the project root, ties inside one directory broken by the `names` declaration order; the slot's global default; a slot that is off stays off.
- A version file naming a version that is not installed is **not** an error for a listing or a prompt hook: it is reported as `not installed` with the exact install command.
- Resolutions are cached by the modification time and size of every file consulted, in the new `<home>/cache/version-files.json`.

## AV-FMT-012 — `vibe app env` and `vibe-app-env/1` (Phase 5a)

Status: **planned**. New command, new format.

- New record `[format.cli-application-env]`, epoch **1**, `schema = "schemas/application_env.jtd.json"`, in-document protocol tag `vibe-app-env/1`.
- `vibe app env [--use SLOT=VER]… [--project] [--shell bash|zsh|fish|pwsh|cmd|posix] [--unset] [--json]` prints, on stdout and nothing else, the lines that switch the current shell. It sets exactly `PATH` (this selection's payload directories prepended, the withdrawn slots' link directories removed, composed by the same function `vibe shell` uses) and `VIBE_APP_USE`.
- Per shell: bash / zsh / `posix` → `export NAME='value';` with POSIX single-quote escaping; fish → `set -gx NAME 'value';` and `set -e NAME;`; pwsh → `$env:NAME = 'value';`; **cmd → bare `NAME=VALUE` lines**, no quoting and no `set` keyword, the `for /f`-usable form, with the one consuming incantation shown in the help.
- The existing `Shell` enum of `vibe self env` (`crates/vibe-cli/src/commands/vvm/env.rs`) is promoted to a shared module and gains a `Cmd` variant; `vibe self env`'s output is unchanged, asserted by a golden test written **before** the move. This remains the one place in the tree that writes shell syntax.
- Invariant, asserted directly against the library: for one selection, `vibe app env` and `vibe shell --use` produce the same `PATH` and the same `VIBE_APP_USE`.
- `--json` answers the resolved selection, the composed PATH as an array and the variables as a table, for a client that sets the environment itself.

## AV-FMT-013 — `vibe hook <shell>` (Phase 5a)

Status: **planned**. New command. No wire format.

- `vibe hook bash|zsh|fish|pwsh [--print] [--yes]` writes one managed block (`MANAGED-BLOCKS-PROTOCOL`) into that shell's rc file — the same mechanism and the same consent PROP-065 `#env-step` already uses for `env.sh`, never a byte outside the block — and `--print` writes it to stdout instead. cmd gets no hook; `vibe shell` is the way there.
- The block registers a function on `PROMPT_COMMAND` (bash), `precmd` (zsh), `--on-variable PWD` (fish) or the prompt function (pwsh). It keeps the last resolved directory and the applied lines in a shell variable; an unchanged `$PWD` returns immediately with builtins only. On a change it walks up with the shell's own file test over the name list the installed packages declare — the block is regenerated when that declared set changes, so the walk is a fixed number of builtin tests — and calls `vibe app env --project --shell X` once, and only when a candidate file exists or a previous selection must be undone.
- **Cost budget, as a checkable gate:** zero subprocesses on a prompt in an unchanged directory and on a `cd` into a directory with no candidate version file; at most one `vibe` invocation on a `cd` that has one; that invocation makes no network call, no registry call and no index-server read, and completes in under 50 ms warm, measured by a fixture.
- A hook never installs, never downloads, never writes the index and never fails the prompt: a version that is not installed prints one line naming the exact install command and leaves the shell as it was.

## AV-FMT-014 — `vibe app versions` and `vibe-app-versions/1` (Phase 5b)

Status: **planned**. New command, new format.

- New record `[format.cli-application-versions]`, epoch **1**, `schema = "schemas/application_versions.jtd.json"`, protocol tag `vibe-app-versions/1`.
- `vibe app versions PACKAGE|SLOT [--channel NAME] [--installed] [--limit N] [--all] [--refresh] [--registry DIR] [--json]`. One row per version, newest first: the version, its origin (`template`, `published`, `published (overrides template)`), the channels it belongs to, and its local state (`installed`, `installed (default)`, `installed (hidden)`, `installed (held)`, or empty). Trailer: the slot, its default, whether the slot is off, and in a closed network the snapshot's sync time.
- Without `--all`, the newest 20 with a count of the rest (Node's domain is 878 records).
- `--json` adds the resolved channel table and, for a template, the template coordinate, revision and version-list fetch time.
- The command renders nothing: it is the skeleton path and costs at most one HTTP request, and none within the TTL or under `--offline`.

## AV-FMT-015 — `[[application.migration]]` (Phase 6)

Status: **planned**.

- `[format.manifest]`, epoch **1 → 1**. Optional rows; absent means byte-identical.
- Rows carry `id`, `name`, optional `description`, `default`, and the two tables `export` and `import`. **Each is exactly one step declaration of the existing grammar, restricted to the kinds `recipe` and `shell`**, `kind` required; `env`, `archive`, `sources`, `native`, `copy` and `agent` refuse, naming the kind. No new `StepKind` variant is added.
- Inputs, beside what PROP-065 `#templates` already passes: `VIBE_MIGRATION_ID`; `VIBE_MIGRATION_WORK` = `<home>/cache/app-migrate/<slot>/<from>-to-<to>/<timestamp>/`; `VIBE_FROM_APP_ROOT`, `VIBE_FROM_VERSION`, `VIBE_TO_APP_ROOT`, `VIBE_TO_VERSION`; and the usual `VIBE_PACKAGE`, `VIBE_PLATFORM`, `VIBE_ARCH`. `VIBE_PACKAGE` is the package tree of the version that owns the step. The export's PATH leads with the **old** installation's payload and proxy directories and the import's with the **new** one's, built by the same function a step requirement uses. `VIBE_OUT` is **not** set and `$out` is not expandable.
- `FROM == TO` is admitted, which is what makes a maintenance entry such as "upgrade the bundled npm" expressible without a second mechanism.

## AV-FMT-016 — `vibe app migrate` and `vibe-app-migration/1` (Phase 6)

Status: **planned**. New command, new format.

- New record `[format.cli-application-migration]`, epoch **1**, `schema = "schemas/application_migration.jtd.json"`, protocol tag `vibe-app-migration/1`.
- `vibe app migrate SLOT FROM TO [--only ID]… [--all] [--yes] [--resume] [--json]`. Entries run independently in declaration order. An attended terminal offers them with the `default` ones preselected; an unattended run without `--yes` refuses `consent_required` and prints the exact command. `install -g` / `update -g` that gives a slot a new version, and `vibe app use` that moves its default, make the same offer and never migrate on their own.
- **Failure semantics.** A failed `export` stops that entry before its import runs and changes nothing (`export_failed`). A failed `import` **leaves the new version installed and the slot's default exactly where the user put it**, never rolls back, never uninstalls and never re-runs the export (`import_failed`). One entry's failure does not stop the others. The work directory is **always** kept, success or failure, and is listed and cleaned by `vibe app work` like any other.
- The report names, per entry, the outcome and — for a failure — **what was not carried**: the entry's name, the failing step, its log path, the work directory, and the exact `--only ID --resume` that retries it. `--resume` reuses the kept work directory and re-runs only the import when the export already succeeded.
- Exit 0 when every selected entry succeeded, 1 when any failed. Outcomes are recorded with the **new** installation.

## AV-FMT-017 — application index protocol `vibe-user-applications/7` (Phase 2, visible from Phase 6)

Status: **planned**.

- `[format.application-index]`, epoch **5 → 6**, new schema generation `schemas/application/e6/index.jtd.json` beside `e1`–`e5`, corpus `formats/corpora/application/e6`, `recoverable = false`, `foreign_parsers = "none"`, `unknown_fields = "deny"`. In-document protocol tag `vibe-user-applications/7`.
- `Installation` gains the optional `template` table of AV-FMT-002 (template revision, produced version, rendered-manifest hash, digest-document hash) and the recorded migration outcomes of AV-FMT-016.
- The writer still writes **the lowest protocol that carries what the home actually retains**, so a home with no template installation and no migration record keeps writing `/6` and existing PATH proxies keep working untouched.
- `vibe-app-proxy` must admit `/7` in its closed protocol set and answer `vibe-app-proxy --supports-index vibe-user-applications/7` with exit 0 and that tag, and the application mutation owner must admit and refresh the capable proxy image before publishing a `/7` index — PROP-059 `#proxy-index-upgrade`, unchanged in substance and extended to the new reader.

## AV-FMT-018 — `vibe app describe`: the template provenance (Phase 2)

Status: **planned**.

- `[format.cli-application-describe]`, `schemas/application_describe.jtd.json`, epoch **1 → 1**. One optional member; existing documents byte-identical.
- Optional `template { revision, produced, rendered_sha256 }` on the described package, so a graphical tool or an agent can see that the manifest it is reading was rendered and from what, plus the installer-option rows dropped by `template_optional` with the version and the missing file.

## AV-FMT-019 — `--write-version-file` on `vibe app use` and `vibe install -g` (Phase 7)

Status: **planned**.

- No wire format. One new flag on two existing commands: it writes the resolved version into the slot's declared `plain` version file (AV-FMT-011) in the current project. This is the general feature behind nvm's `--save` / `-w`, and it exists because a version manager that reads a version file should be able to write it. It refuses to write through a symbolic link and refuses when the slot declares no `plain` version file, naming both.

## AV-FMT-020 — the vibenvm conformance suite's authored documents (Phase 7)

Status: **planned**. New authored documents; no vibe wire format.

- `conformance/vibenvm/exclusions.toml`, `schema = 1`: one `[[exclusion]]` row per accepted gap with `subcommand`, `flag` (empty = the whole subcommand), `reason`, `approved_by` and `approved`. **A row without `approved_by = "owner"` and a date does not count as approved and the coverage gate fails on it**, so the set of accepted gaps is visible in the git log of one file and cannot be widened by a worker.
- `conformance/vibenvm/mirror.lock`, `schema = 1`: the pinned SHA-256 of every file of the local Node release mirror. A mismatch refuses to run (exit 3).
- `conformance/vibenvm/nvm-tests.allow`: the recorded subset of the pinned nvm-sh `test/fast/` suite that runs offline against the shim. Reusing nvm's own tests is a bonus, never the gate — the pinned suite is *near*-offline, not offline, and has no suite-wide mirror override.
- `conformance/vibenvm/normalise.rules`: the declared, mechanical normalisations — sandbox paths to `<HOME>`, the tool root to `<TOOLROOT>`, ANSI SGR stripped, `\d+(\.\d+)?(ms|s)` to `<T>`, a listed set of progress and notice line regexes dropped, version lists sorted. **Observation records are compared field by field; stdout and stderr text are not compared at all** unless a case opts in, and then only after these rules. No model's judgement enters the verdict.
- Verdict = exit code: `0` every case passed and coverage complete; `1` a differential mismatch (naming case, step, field, both values); `2` coverage incomplete (naming each uncovered pair) or an unapproved exclusion; `3` a harness error (mirror digest mismatch, a non-loopback connection attempt, a missing pinned tag).
