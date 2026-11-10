# App-versions campaign: format and behaviour changes

Append-only log of user-visible changes made by the campaign in `PLAN.md`, material for release notes. Entries are `AV-FMT-NNN`; numbers are never reused.

Regime at writing (`formats/EPOCHS.toml`, 2026-11-10): `public = false`, `break_window_open = true` — breaking is free and unmigrated, the standing user recipe is «regenerate / re-init / re-fetch», and no break note under `formats/breaks/` is required. Every entry below still states the format registry record it touches and whether its epoch moves, because `FormatId` cannot express an unregistered format.

Design authority for all of it: `spec://org.vibevm.core/vibevm/common/PROP-075#FORMAT-CHANGES`.

**Id ranges.** `AV-FMT-001`…`AV-FMT-020` are the ids `PLAN.md` assigned to the phases while
planning; a phase fills its own. Ids from **`AV-FMT-101`** up are for a format a phase turned
out to need and the plan did not foresee, so two phases running concurrently cannot pick the
same number. P5a's own unforeseen records are `AV-FMT-101` and `AV-FMT-102`; P2's are
`AV-FMT-021`…`AV-FMT-024`, taken from the planning range below 100 because P2 owns it. The owner's
decisions of 2026-11-10 redesigned phase P7 into six phases, and the formats they need are
`AV-FMT-025`…`AV-FMT-032`, also taken from the planning range because the plan now foresees them;
`AV-FMT-019` (`--write-version-file`) is **amended in place** rather than renumbered, because it is
the same flag on the same two commands, now carrying both version-file formats.

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

Status: **delivered 2026-11-10** (phase P2). As implemented it matches the entry
below, with two clarifications the 9 → 10 step forced. (1) The epoch floor a
`[[remap]]` source binding needs is now its own named constant,
`REMAP_SOURCE_SCHEMA_VERSION = 9`, instead of being read off
`CURRENT_SCHEMA_VERSION`: without that split, raising `CURRENT` to 10 would have
silently raised the floor a remap binding requires and refused every schema-9
lock that carries one. (2) The writer still writes **the lowest epoch that
carries what the lock actually holds** — the rule the 7 → 8 → 9 steps already
followed — so a project with no template package and no channel requirement
writes exactly the bytes it wrote before PROP-075, and a template record or a
`via_channel` under an older epoch is **corruption, never legacy input**
(`validate_template_records`).
(3) The alarm has **two** call sites, not one. The comparison against the
recorded `rendered_sha256` is `verify_recorded`, for the path that has a lock
record. It also fires from materialisation, with no lock at all: a rendered
directory already on disk is the same produced version rendered before, and its
digest-bearing fields came from a digest document that is cached write-once and
immutable — so a difference in those fields can only mean the document changed
upstream, which is the alarm, while a difference *outside* them is a template
fixed without touching a produced version, which TEMPLATE-FLAG exists to allow
and which rewrites silently. Both behaviours are asserted through the real
binary in `crates/vibe-cli/tests/cli_template_resolution.rs`.

- `[format.lockfile]` (`schema = "none"`, authored TOML, `foreign_parsers = "ours"`), registry epoch **1 → 1**, following the precedent of the 7→8→9 steps, which did not move it. The lockfile's own `meta.schema_version` goes **9 → 10** and `admit` accepts **7..=10**; 11 and above still refuse.
- `LockedPackage` gains an optional table `template` with `revision` (the template package's own version), `rendered_sha256` (SHA-256 of the full render's TOML bytes), `digest_source_sha256` (SHA-256 of the fetched digest document's bytes), and `source_url` / `digest_url` as resolved **after** remap.
- `LockedPackage` gains an optional `via_channel` naming the channel a requirement was lowered through, so `vibe.toml` keeps saying `lts` while `vibe.lock` says the concrete version and `vibe update` re-lowers.
- Both fields are omitted when absent, so a lock with no template package is byte-identical except for `schema_version`.
- Behaviour: before a locked version is installed, updated, reinstalled or modified, the full render is recomputed and `rendered_sha256` compared; a difference refuses before any effect with code `template_render_changed`, naming the package, the version, the template revision then and now, both hashes, and the digest-bearing fields that differ. There is **no** `--trust-template` flag. A changed template *revision* alone, with the same digests, is not an alarm: the recorded revision is updated in place with a note.

## AV-FMT-003 — index per-version record: `template`, `channels` (Phase 2)

Status: **delivered 2026-11-10** (phase P2), with one deliberate omission: the
two extra golden documents under `formats/corpora/index/e1` were **not** added.
Spike S3 had already proved the point they would make — the corpus bytes do not
move, `cargo xtask rebuild --check` reports the catalog byte-identical to its
journal's projection (16 files), and `wire-diff` sees `schema 0, corpus 0` — and
a corpus document is a journal fact, so adding one is a separate, owner-visible
act rather than a side effect of a member. Filed as this phase's one open item.
The mechanical cost S3 predicted held exactly: the regenerated
`crates/vibe-wire/src/generated/shared/mod.rs` plus two struct-literal sites
(`crates/vibe-wire/src/behaviour/records.rs`, where a behaviour record is a
concrete published version, and `crates/vibe-index/src/scanner/tree_entry.rs`,
where both members are read off the manifest).

- `schemas/index/e1/entry.jtd.json` is a bare `ref` to the **shared vocabulary** `version_entry`, so the two new optional members reach `primary.jsonl`, `by-name/<name>.json` and the journal at once. Records touched: `[format.index-primary]`, `[format.index-by-name]`, `[format.index-entry]`, all epoch **1 → 1** (`recoverable = true`, `foreign_parsers = "many"`): the members are optional and absent means byte-identical documents.
- New optional `template: boolean` — the published version is a template revision rather than a concrete version. This is how a client splits a package's published versions in one index read and finds the active template (the newest such revision), without reading a manifest per version.
- New optional `channels: string[]` — publisher-assigned labels on a concrete version, so resolving a channel for a non-template package is an index read and never a scan of every manifest.
- Written by publish and by the layout projection (`vibe_index::layout::project`, which derives both from the manifests on disk, so an indexed registry and a layout registry agree by construction).
- One new golden document each under `formats/corpora/index/e1` exercising the members; the existing corpus bytes must not move (verified by `cargo xtask rebuild --check`).
- Consequence, stated as a limitation: a template is **not** discoverable in a per-package git registry with no index, where listing versions is a tag list. Such a registry refuses with `template_index_required`, naming the registry and the fix.

## AV-FMT-004 — two remap artifact kinds: `template-index`, `template-digest` (Phase 2)

Status: **delivered 2026-11-10** (phase P2), exactly as the entry below describes:
two members of `ArtifactKind` with their kebab-case labels, no new routing
mechanism and no new config table. The template layer passes every declared URL
through `vibe_core::remap::active()` before the fetch, and a Locator refusal is
`remap_denied` naming the row — never a silent fall-through to the origin.

- PROP-066 `#locations` artifact-kind vocabulary, epoch **1 → 1** (one open vocabulary gains two members).
- A template's version-list document and its per-version digest document are fetched through the operation's controlled transport and the remap Locator like every other byte, so a closed network redirects them with the rows it already uses and `vibe remap explain <url>` answers for them. The payload inputs of a rendered version keep the existing `app-input` kind and the drop convention of PROP-066 `#drops` unchanged.
- No new routing mechanism and no new config table: this is two vocabulary members, exactly as `distribution` and `app-input` are.

## AV-FMT-005 — `~/.vibe/config.toml` `[template] version_list_ttl` (Phase 2)

Status: **delivered 2026-11-10** (phase P2). As built, with two additions the
entry below did not spell out. (1) `--refresh` is a **global** flag, beside
`--offline`, and `--offline` wins on conflict: a posture that forbids the network
outranks a request to use it. (2) The cache holds one more file,
`<settings>/cache/templates/<group>/<name>/template.toml` — the template
*document* verbatim, kept beside its inputs. It is a declaration, not an output,
so `#TEMPLATE-RENDER-LOCATION`'s ban on caching a render is untouched; it exists
so a **channel** can be lowered from the cache alone, with no registry root, no
index read and no request, which is the budget `#ACTIVATION-HOOK` gives a prompt
hook. Unlike a digest document it is not write-once: a template can be fixed
without touching a produced version, and the cache must then hold the fix.

- User config (`schema = "none"`, authored TOML), epoch **1 → 1**. One new table with one key.
- `version_list_ttl` — how long a cached template version list is used before it is refetched. Default **15 minutes**. Digest documents are **never** revalidated (a released version's checksums are immutable, and a change upstream is precisely the AV-FMT-002 alarm), so the TTL applies to the version list alone.
- Cache layout, new: `<settings>/cache/templates/<group>/<name>/versions.json`, `…/digests/<version>`, `…/rendered/v<version>/`. `--refresh` on any resolving command refetches the list; `--offline` / `VIBE_OFFLINE` / `[net] offline` read the cache whatever its age and refuse a miss with `template_cache_missing`, naming the package, the exact path and the command that would fill it.

## AV-FMT-006 — `org.vibevm.node/node` becomes a template package (Phase 3)

Status: **delivered 2026-11-10** (phase P3), with one amendment and one
deliberate deferral, both below. Package-side only; no vibe format changes.
The package repository's commit is
`/fast/git/vibe-pkg-node` `1a52f3d` with the new tag `v1.0.0`, and the
generator's is `/fast/git/vibe-pkg-node-generator` `c160ba3`; **nothing is
published or pushed** (see the last bullet for why).

- `/fast/git/vibe-pkg-node` carries one template package at template revision **`1.0.0`** (tag `v1.0.0`), with `[template.versions]` pointing at `https://nodejs.org/dist/index.json` (`select = ""`, `version_field = "version"`, `strip_prefix = "v"`, `min = "12.0.0"`), `[template.digests]` at `https://nodejs.org/dist/v{version}/SHASUMS256.txt`, and two `[[template.channel]]` rows giving `lts` (`rule = "truthy"`) and `lts/<codename>` (`rule = "value"`). The seven installer options are as they were written, rendered byte-identically — `cargo test -p vibe-template --test node_golden` is the gate.
- **Amendment: three optional rows, not two.** `linux-musl-x64`, `windows-arm64` **and `macos-arm64`** are `template_optional = true`. Over the 406 produced versions of the declared window, `linux-x64-musl` is absent from 389, `osx-arm64-tar` from 132 (Apple-silicon archives begin at 16.0.0) and `win-arm64-zip` from 234 (19.9.0); all seven are present together only from 26.8.0 up. Without the third row every release below 16.0.0 refuses to render rather than installing the targets it has. Asking for a dropped target refuses with `no_option`, naming the platform or the `platform/arch` pair — never a failed download of an archive that never existed.
- The two existing concrete tags (`v24.22.2`, `v22.23.4`) stay published, so "a concrete version wins over the template's rendering of the same version" is exercised by the real package from the first day: `cli_template_node.rs::the_published_tag_wins_over_the_template_rendering_of_the_same_version` drives exactly that shape (one revision plus one concrete tag) and asserts that nothing was rendered and no digest document fetched for the published version.
- **Visible loss, deliberate:** the rendered manifests no longer carry `size` on their archive steps, because nodejs.org publishes digests but not sizes and the retired generator took sizes from a hand-made `SIZES` file. The step panel falls back to the server's `Content-Length`, which PROP-065 `#sizes` / `#display` already allow, and `vibe registry mirror --dry-run` reports its byte total as a lower bound and says so.
- **Second deliberate loss:** the rendered manifests carry no provenance header either (the generator wrote four comment lines naming itself and the SHA-256 of both inputs). A render's provenance is the lock and the installation record instead — template revision, rendered-manifest hash, digest-document hash (AV-FMT-002) — which is machine-readable where the header was not.
- `org.vibevm.node/vibevm-package-generator` is superseded and is not republished; its repository's README records that, what is lost with it, and where the proof that the replacement is faithful lives.
- `org.vibevm.codkit/codkit` is **untouched** by this campaign.
- **Deliberately not done: publishing.** The package is not pushed and not republished, because rendering a template found in a **per-package git registry** is the one part of PROP-075 `#TEMPLATE-DISCOVERY` that P2 left to the phase that first needs it — and P3 does not need it: every directory-shaped registry (`--registry DIR`, a `localdir` source, a `monorepo` working copy, a mirrored layout registry) resolves templates today, which is what the offline suite and the hand install both use. Until that gap closes, a client resolving this package through a git registry would see the template revision as if it were a Node.js version; the revision is numbered `1.0.0`, below every produced version, so even such a client prefers a real one.
- **Fixtures, in this repository:** `crates/vibe-template/tests/fixtures/node/` now holds upstream's own frozen documents — `index-excerpt.json` (a leading, byte-verbatim slice of `https://nodejs.org/dist/index.json`, 410 of 878 records: down to the declared `min` plus the three records below it) and four verbatim `SHASUMS256.txt` documents (24.22.2, 22.23.4, 26.13.0, 12.0.0). The two golden ones hash to exactly the inputs the committed manifests' provenance headers name, and the test asserts it. That directory's `README.md` states each file's provenance and the selection rule.

## AV-FMT-007 — `mirror.toml`, declarative mirror selection, recorded selection (Phase 4)

Status: **delivered 2026-11-10** (phase P4, commit a). As implemented it matches the entry
below with four amendments, recorded in
`spec://org.vibevm.core/vibevm/common/PROP-075#P4-AS-BUILT`:

1. **Only a *declared* selection is recorded.** The entry below reads «the effective selection
   — merged from the file and the flags». As built, a `--config` run records its selection and a
   `--package` / `--group` run records nothing. The flag predates this feature and has a second
   job — making a source that can be listed neither by index nor by organisation API mirrorable
   one package at a time — so recording it would turn that workaround into a permanent narrowing
   of every later bare run of that source, and would let one per-run override destroy the
   operator's declaration. It is also what keeps the record byte-identical for every workflow
   that predates `--config`, which is what makes `schema = 1` honest.
2. **A `[[template]]` row against a per-package Git source is refused by name**, before anything
   is listed or written: a snapshot is written from the template package's own tree, which only a
   `monorepo` or `localdir` source has (`TemplateLayer::for_directory`). Rendering from a git
   registry is deferred by `#TEMPLATE-DISCOVERY`, and the refusal names the rows and the fix.
3. **`[select]` with only `[[template]]` rows selects exactly those packages.** The two-axis rule
   the entry generalises reads an empty axis as unconstrained, which would have turned a
   configuration of template rows alone into «everything». A run with no selection at all is
   still everything; it never reaches that rule.
4. **Three more refusals the entry did not name**, each a declaration that cannot mean what it
   says: `newest = 0` (keeps no version of any major), `[payloads] max_total = 0` (admits no
   payload at all) and two `[[template]]` rows for one package.

- New authored document `mirror.toml` (`schema = 1`; any other value refuses, as `_index/mirrors/<source>.toml` already does). `[select]` with `all`, `groups`, `packages`, `exclude`; one `[[template]]` row per template package with `package`, `versions`, `channels`, `newest`, `platforms`, `archs`, `installers`, `payloads`; `[payloads]` with `max_total` and `into`.
- `vibe registry mirror` gains `--config <file>`. Today's `--package` / `--group` keep working and **override** `[select]` wholesale rather than intersecting with it. `all = false` with no groups and no packages is a usage refusal, never a silent empty run. This generalises today's `Filters { packages, groups }` (exact matching, no excludes, no version ranges, no newest-N).
- `_index/mirrors/<source>.toml` gains a `[select]` table and `[[template]]` rows beside its existing `[source]`, `[sync]` and `[[version]]`; its `schema` stays **1** because the tables are additive and a reader that ignores them behaves as today. A later bare `vibe registry mirror <source> --into <target>` repeats the recorded selection and adds only what newly matches.
- `vibe registry mirror` gains `--prune`, `--prune-payloads` and `--yes`. A prune is always a plan first (one row per removal with its reason) and removes nothing without a yes; `--dry-run --prune` prints the plan and stops. A version recorded as copied from a *different* source is never pruned by this source's run. A pruned version's index rows and journal facts are rewritten in the same commit, so the target never advertises bytes it no longer has. No run without `--prune` ever deletes anything, however narrow the selection became.
- `[payloads] into` defaults to the first configured `[artifacts] packages` drop; writing payloads into the registry target is **refused by name**. Payloads are written at the first rung of the existing drop lookup, `<into>/<group>/<name>/<version>/<file>`, and verified against the digest the render produced. An input already present with the right digest is skipped; one present with a wrong digest stops the run with `digest_mismatch` and is not replaced. `max_total` is checked **before the first byte is written** and over-budget stops with `payload_budget_exceeded`, naming the budget, the projected total and what does not fit.

## AV-FMT-008 — template snapshot under `_index/templates/` (Phase 4)

Status: **delivered 2026-11-10** (phase P4, commit b). New format. As implemented it matches the
entry below with three amendments to the registry record alone — the layout, the record's
contents and the staleness rules are as written:

1. **`schema = "none"`, not a JTD** (decided by the central session). One format spans three
   files — a TOML record beside a JSON envelope beside an opaque upstream document — and no
   single JTD describes that. The record is hand-typed serde, exactly like
   `_index/mirrors/<source>.toml` beside it, and the precedent followed is
   `[format.change-manifest]`.
2. **`recoverable = false`.** The entry said `true`. The snapshot exists precisely because a
   client inside a closed network cannot reach the upstream the bytes came from, so **nothing
   inside the network can regenerate them** — which is the definition of unrecoverable. Calling
   it recoverable would claim a repair recipe that cannot exist.
3. **Its own authored golden corpus, `formats/corpora/index-templates/e1`**, not the projected
   `formats/corpora/index/e1`. That home is a registry tree `rebuild --check` proves
   byte-identical to its journal's projection; the snapshot is written by the mirror through
   `BatchConfig.files` and not by the index writer, so putting its goldens there would mix an
   authored document into a projected home. It is the same boundary P4 amendment 1 draws for
   `WRITER_DIRS`. An authored corpus stands on its wire-parity oracle, which is
   `vibe_registry::templates::snapshot::tests::corpus`: the writer emits exactly those bytes and
   the reader round-trips them, with the two «reads as absent» cases kept as bytes too.

- New record `[format.index-templates]`, epoch **1**, `schema = "none"`, `recoverable = false`, `foreign_parsers = "many"`, corpus `formats/corpora/index-templates/e1`, `sunset = "none"`.
- Layout under the target registry root: `_index/templates/<group>/<name>/snapshot.toml` (`schema = 1`), `…/versions.json` (the upstream source document restricted to the selected versions, in the same envelope the client cache uses, with the record array filtered and nothing else altered), and `…/digests/<version>` (the upstream digest document verbatim).
- `snapshot.toml` records `[source]` (the template coordinate and revision, the version-list url and the digest url template as declared, before remap), `[sync]` (the writing vibe and the first and last sync time), and one `[[version]]` per snapshotted version with `version`, `digest_sha256` and `rendered_sha256`.
- Written inside the mirror's single commit through the existing `BatchConfig.files` channel, so the snapshot's digests are fixed by the git commit itself.
- A client never revalidates a snapshot and never contacts upstream from one. `vibe app versions` reports the snapshot's `[sync] last` so a user can see how old the list is. As built that is **one mechanism, not two readers**: discovering a template in a directory registry primes this machine's template cache from the registry's snapshot, so the version-list and digest readers find an ordinary cache hit and never reach the transport — a client inside a closed network therefore renders «with the same code as outside» literally. Priming never overwrites a cache entry that is already there (a digest document is immutable, so the two cannot honestly differ, and overwriting would erase the evidence the changed-digest alarm exists to raise), and a snapshot that cannot be read — wrong `schema`, unparseable record, a `versions.json` whose `sha256` does not hash its own `document`, a `[[version]]` row whose digest file is missing or does not hash to it — reads as **absent** rather than fatal, because a refusal would make the whole registry unusable for every package including the ones with no template at all.
- A re-run adds versions and leaves existing ones untouched; a `digest_sha256` that changed for an already-snapshotted version stops the run before anything is written, exactly as PROP-002 `#MIRROR-RULES` stops on a changed hash of a frozen version.
- ~~`_index/templates/` is a new directory the index writer must own: `WRITER_DIRS` (`crates/vibe-index/src/index/mod.rs`) learns it, and the layout projection must know it so the index does not read `Stale`.~~ **Withdrawn against the code, 2026-11-10 (P4 amendments 1 and 2).** `WRITER_DIRS` is the whitelist of what `Index::write_to` itself *produces* — read by `rebuild.rs` `catalog_file_set` and `index/memory/projection_compare.rs` `projection_matches` — and the snapshot is written through `BatchConfig.files`, not by the index writer, so `_index/templates/` stays outside it exactly as `README.md`, `.gitignore` and `state/` do. Nor can it make the index read `Stale`: `listing.txt` is `render_listing(version_dirs, journal_digest)` and `version_dirs` skips `_index` altogether. The obligation this bullet created is replaced by one test: a registry that grows `_index/templates/` still reads `Fresh`.
- `versions.json` carries the client cache's own envelope, `CachedVersionList { source_url, sha256, fetched_at, document }`. Because the snapshot restricts `document` to the selected versions, `sha256` is **recomputed over the filtered document** rather than carried over from the upstream fetch.

## AV-FMT-009 — mirror report: `templates[]`, `pruned[]` (Phase 4)

Status: **done** 2026-11-10.

- `[format.cli-registry-mirror-report]`, `schemas/registry_mirror_report.jtd.json`, epoch **1 → 1**. Both arrays are optional; every existing member is unchanged, including the `command: "registry:mirror"` discriminator and the absence of a `protocol` member.
- `templates[]`: per `[[template]]` row — the template coordinate and revision, the versions selected with the first and last of them, the channels that resolved and to what, the installer options kept per version, the rows dropped by `template_optional` with the missing file, and, with payloads, the input count, the bytes already present in the drop, the bytes to fetch, the total and a flag for every input of unknown size.
- `pruned[]`: per removal — what was removed, from where, and the reason that reached it.
- Human output draws a template row as one node with its versions as children, in the display language of PROP-067 as PROP-065 `#plan` does.
- **As built:** both arrays are optional with `x-empty: "omit"`, so a run with no `[[template]]` row and no `--prune` prints the document every mirror run before this feature printed, byte for byte. `templates[]` rides in selection order; `pruned[]` carries the **plan**, which is what makes "never silent" true for a client that reads the document rather than the printed rows.
- **As built:** a third member the design did not name, `removed` (optional `uint32`), because `pruned[]` alone cannot say whether the plan was applied: it is zero on a dry run and zero when the plan's question was answered no, and absent without `--prune`. `dry_run` could not stand in for it — an attended "no" is not a dry run.
- **As built:** the per-row payload facts are their own object, `payloads`, present only for a row with `payloads = true`, carrying `inputs`, `present`, `present_bytes`, `fetch_bytes`, `total_bytes`, `unknown` and `into`. `total_bytes` is a **lower bound** whenever `unknown` is non-zero — a template that declares no `size` cannot be weighed — and the member set says so rather than the document guessing. Byte counts use the established `byte_count` convention (`float64` + `x-rust-type: "u64"`, because JTD has no `uint64`).
- **As built:** "the installer options kept per version" became a `versions[]` array of objects — `version`, and for a payload row its `installers`, `inputs` and `present` — which is the same shape the human form draws, one node per row with its versions as children.
- **As built:** `pruned_kind` is a closed vocabulary (`version` / `snapshot` / `payload`), so automation can tell a removal that changed the registry from one that only changed the drop; the drop is the one removal that happens **after** the commit.

## AV-FMT-010 — ~~`repomd.json` `files` gains the `_index/templates/` key~~ — **withdrawn** (Phase 4)

Status: **withdrawn 2026-11-10** by the central session. **No `templates` entry is added to `repomd.json`'s `files` map, and `[format.index-repomd]` is not touched at all** — no epoch move, no schema change, nothing for `wire-diff` to see.

- The reason: the snapshot lives at a **fixed path a client already knows** (`_index/templates/<group>/<name>/`, `AV-FMT-008`), so a client needs no manifest entry to find it, and the manifest is the manifest of what the index *writer* produces. Teaching the `repomd.json` projection a new first-class input — a directory written by the mirror through `BatchConfig.files` rather than by `Index::write_to` — is a change to the index's own model and is out of this phase's scope. It is the same boundary P4 amendment 1 draws for `WRITER_DIRS`.
- What a client does instead is read the snapshot directory directly, exactly as it reads `_index/mirrors/<source>.toml` today without a `files` entry for it. If a later phase wants the snapshot advertised in the manifest, that is a new id against `[format.index-repomd]`, not this one.
- Nothing inherits this id. `AV-FMT-008`'s snapshot layout and `AV-FMT-009`'s report arrays are unaffected; the numbers are never reused.

## AV-FMT-011 — `[[application.version_file]]` (Phase 5a)

Status: **done** 2026-11-10.

- `[format.manifest]`, epoch **1 → 1**. Optional rows; absent means byte-identical.
- `format = "plain"` with `names` (an ordered array of file names, `[".nvmrc", ".node-version"]`): the first non-empty, non-comment line, trimmed, with `#` and a CR tolerated, read as a version requirement **or** a channel name.
- `format = "tool-versions"` with `key` (the package's key in asdf's `.tool-versions`, `nodejs`): whitespace-separated `<key> <version>…` lines, `#` comments ignored, the first version for the key taken and the rest reported as unused.
- Nothing else is a format: `.python-version` is a `plain` row, and `package.json` `engines` is not read.
- Precedence, highest first: an explicit `--use SLOT=VER` / `SLOT@VER`; the project's `vibe.toml` requirement on that application package; the nearest declared version file, searched from the working directory upward and stopping at the project root, ties inside one directory broken by the `names` declaration order; the slot's global default; a slot that is off stays off.
- A version file naming a version that is not installed is **not** an error for a listing or a prompt hook: it is reported as `not installed` with the exact install command.
- Resolutions are cached by the modification time and size of every file consulted, in the new `<home>/cache/version-files.json` — now its own registered format, `AV-FMT-102`.
- **As built:** the rows are a tagged union on `format`, so a member of one format is unknown to the other; a name must be one plain file name of one directory; two rows never claim the same file and a package declares at most one `tool-versions` key. Every refusal names its field as a TOML path in the sentence itself (`application.version_file[1].names: …`).
- **As built:** a single leading `v` is tolerated before a requirement (`v22.23.4`), because that is the spelling `.nvmrc` files carry; the authored spelling is kept in every report. A channel name parses, but only `latest` and `stable` lower in this phase (over the installed set, with no network); every other channel is reported `unresolved_channel` until P2.
- **As built:** the declaration is retained beside the index rather than inside it — `AV-FMT-101`.
- **As built:** a requirement several installed versions satisfy resolves to the newest; `selection_required` is raised only when several distinct installations share that newest version.

## AV-FMT-012 — `vibe app env` and `vibe-app-env/1` (Phase 5a)

Status: **done** 2026-11-10. New command, new format.

- New record `[format.cli-application-env]`, epoch **1**, `schema = "schemas/application_env.jtd.json"`, in-document protocol tag `vibe-app-env/1`.
- `vibe app env [--use SLOT=VER]… [--project] [--shell bash|zsh|fish|pwsh|cmd|posix] [--unset] [--json]` prints, on stdout and nothing else, the lines that switch the current shell. It sets exactly `PATH` (this selection's payload directories prepended, the withdrawn slots' link directories removed, composed by the same function `vibe shell` uses) and `VIBE_APP_USE`.
- Per shell: bash / zsh / `posix` → `export NAME='value';` with POSIX single-quote escaping; fish → `set -gx NAME 'value';` and `set -e NAME;`; pwsh → `$env:NAME = 'value';`; **cmd → bare `NAME=VALUE` lines**, no quoting and no `set` keyword, the `for /f`-usable form, with the one consuming incantation shown in the help.
- The existing `Shell` enum of `vibe self env` (`crates/vibe-cli/src/commands/vvm/env.rs`) is promoted to a shared module and gains a `Cmd` variant; `vibe self env`'s output is unchanged, asserted by a golden test written **before** the move. This remains the one place in the tree that writes shell syntax.
- Invariant, asserted directly against the library: for one selection, `vibe app env` and `vibe shell --use` produce the same `PATH` and the same `VIBE_APP_USE`.
- `--json` answers the resolved selection, the composed PATH as an array and the variables as a table, for a client that sets the environment itself.
- **As built:** `[format.cli-application-env]`, epoch 1, `schemas/application_env.jtd.json`, protocol tag `vibe-app-env/1`. Each selection carries its `source` (`flag`, `project_requirement`, `version_file`, `slot_default`, `slot_off`, `none`), its `state` (`selected`, `not_installed`, `off`, `unresolved_channel`, `ambiguous`), the requirement as authored, the key and version it resolved to, the version file that decided it, the `.tool-versions` versions it did not use, the candidates nothing narrowed, and the exact install command for a version that is not installed.
- **As built:** stdout carries the lines and nothing else; every notice goes to **stderr**, so a prompt hook's `eval` cannot choke on one.
- **As built:** `--project` **retracts** as well as applies — a selection the inherited `VIBE_APP_USE` carries and this resolution does not re-apply is taken out of PATH, which is what lets the hook undo on leaving a project.
- **As built:** the `Shell` enum moved to `crates/vibe-cli/src/shell_syntax.rs` and gained `Cmd`, `set_line` and `unset_line`; `vibe self env`'s bytes are pinned by a golden test written before the move.

## AV-FMT-013 — `vibe hook <shell>` (Phase 5a)

Status: **done** 2026-11-10. New command. No wire format.

- `vibe hook bash|zsh|fish|pwsh [--print] [--yes]` writes one managed block (`MANAGED-BLOCKS-PROTOCOL`) into that shell's rc file — the same mechanism and the same consent PROP-065 `#env-step` already uses for `env.sh`, never a byte outside the block — and `--print` writes it to stdout instead. cmd gets no hook; `vibe shell` is the way there.
- The block registers a function on `PROMPT_COMMAND` (bash), `precmd` (zsh), `--on-variable PWD` (fish) or the prompt function (pwsh). It keeps the last resolved directory and the applied lines in a shell variable; an unchanged `$PWD` returns immediately with builtins only. On a change it walks up with the shell's own file test over the name list the installed packages declare — the block is regenerated when that declared set changes, so the walk is a fixed number of builtin tests — and calls `vibe app env --project --shell X` once, and only when a candidate file exists or a previous selection must be undone.
- **Cost budget, as a checkable gate:** zero subprocesses on a prompt in an unchanged directory and on a `cd` into a directory with no candidate version file; at most one `vibe` invocation on a `cd` that has one; that invocation makes no network call, no registry call and no index-server read, and completes in under 50 ms warm, measured by a fixture.
- A hook never installs, never downloads, never writes the index and never fails the prompt: a version that is not installed prints one line naming the exact install command and leaves the shell as it was.
- **As built:** the block's markers are its own (`# >>> vibe application versions …`), beside the `env`-step block of PROP-065 `#env-step`, which the same `BlockMarkers` machinery now serves generically. A write asks in an attended terminal and refuses `consent_required` unattended without `--yes`; `--print` always writes to stdout and touches no file. `pwsh`'s startup file is its profile (`~/.config/powershell/Microsoft.PowerShell_profile.ps1`), not `.profile`.
- **Measured:** `cli_hook_linux::hook_spawns_nothing_in_an_unchanged_directory` counts the invocations of a `vibe` shim on PATH: **0** on a `cd` into a directory with no candidate file, **0** for two prompts in an unchanged directory, **1** on a `cd` into a directory that has one. The warm cost of that one call is **21 ms** in a debug build (process floor 24 ms), against a 50 ms budget — `LOG.md`, spike S4.

## AV-FMT-014 — `vibe app versions` and `vibe-app-versions/1` (Phase 5b)

Status: **done** 2026-11-10 (phase P5b). New command, new format. As
implemented it matches the entry below, with the five clarifications
recorded in
`spec://org.vibevm.core/vibevm/common/PROP-075#P5B-AS-BUILT`.

- New record `[format.cli-application-versions]`, epoch **1**, `schema = "schemas/application_versions.jtd.json"`, `recoverable = true`, `foreign_parsers = "many"`, `corpus = "none"`, protocol tag `vibe-app-versions/1`.
- `vibe app versions PACKAGE|SLOT [--channel NAME] [--installed] [--limit N] [--all] [--refresh] [--registry DIR] [--json]`. One row per version, newest first: the version, its origin (`template`, `published`, `published (overrides template)`), the channels it belongs to, and its local state (`installed`, `installed (default)`, `installed (hidden)`, `installed (held)`, or empty). Trailer: the slot, its default, whether the slot is off, and in a closed network the snapshot's sync time. `--refresh` and `--json` are the existing root flags (`AV-FMT-022`), not new ones on this command.
- Without `--all`, the newest 20 with a count of the rest (Node's domain is 878 records).
- `--json` adds the resolved channel table and, for a template, the template coordinate, revision and version-list fetch time.
- The command renders nothing: it is the skeleton path and costs at most one HTTP request, and none within the TTL or under `--offline`.
- **Wire shape, as built.** Top level: `protocol`, `target`, `off`, `packages[]`, optional `slot`, `default`, `snapshot_synced`, `notices[]`. Each `packages[]` row: `package`, `total`, `shown`, `more`, `versions[]`, optional `slot`, `channels` (name → members newest first), `template`, `notices[]`. Each `versions[]` row: `version`, `origin` (the closed vocabulary `template` / `published` / `overrides`), `state` (the closed vocabulary `available` / `installed` / `installed_default` / `installed_hidden` / `installed_held`), optional `channels[]` and `key`. `template`: `coordinate`, `revision`, optional `version_list_url`, `fetched_at`, `snapshot_synced`. The two human spellings PROP-075 writes (`published (overrides template)` and `installed (default)`) are the **rendering** of the two tokens, not the tokens themselves, so neither JSON member carries a space or a parenthesis.
- **New refusal codes**, in a new `VersionsError` family that `AppError` delegates to transparently and `vibe app errors` lists beside every other: `unknown_target` (the argument is neither a slot of this home nor an installed package name — the message names the slots that do exist), `ambiguous_target` (a bare name several installed packages carry — the message names the `<group>/<name>` spellings), `unknown_channel` (the package declares no such channel — the message names the ones it does), `version_domain_unreadable` (a registry the listing had to read refused).
- `snapshot_synced` is declared and left unfilled until phase P4 writes a snapshot; nothing reads a `_index/templates/` tree before then.

## AV-FMT-015 — `[[application.migration]]` (Phase 6)

Status: **delivered 2026-11-10** (phase P6). As implemented it matches the entry below, with five
clarifications recorded in `spec://org.vibevm.core/vibevm/common/PROP-075#P6-AS-BUILT`: each side is
**literally** a `StepDecl`, so it carries its own `id` and `title` (the runner uses them in the
record, the progress row and the log name) rather than inheriting a synthesised pair; a migration
step that declares `requires`, `when`, `unless` or `phase` is refused naming the field, because the
runner has no requirement resolution, no feature or parameter selection and no prepare/finish split
to honour them with; a migration step's current directory is the shared work directory on both
sides, the one place the two agree on; `$sources` refuses beside `$out`, for the same reason
`VIBE_SOURCES` is not set; and a step's `needs` rows are evaluated per side, so an export that
declares a `command:` this host lacks refuses before its child starts and the entry has changed
nothing.

- `[format.manifest]`, epoch **1 → 1**. Optional rows; absent means byte-identical.
- Rows carry `id`, `name`, optional `description`, `default`, and the two tables `export` and `import`. **Each is exactly one step declaration of the existing grammar, restricted to the kinds `recipe` and `shell`**, `kind` required; `env`, `archive`, `sources`, `native`, `copy` and `agent` refuse, naming the kind. No new `StepKind` variant is added.
- Inputs, beside what PROP-065 `#templates` already passes: `VIBE_MIGRATION_ID`; `VIBE_MIGRATION_WORK` = `<home>/cache/app-migrate/<slot>/<from>-to-<to>/<timestamp>/`; `VIBE_FROM_APP_ROOT`, `VIBE_FROM_VERSION`, `VIBE_TO_APP_ROOT`, `VIBE_TO_VERSION`; and the usual `VIBE_PACKAGE`, `VIBE_PLATFORM`, `VIBE_ARCH`. `VIBE_PACKAGE` is the package tree of the version that owns the step. The export's PATH leads with the **old** installation's payload and proxy directories and the import's with the **new** one's, built by the same function a step requirement uses. `VIBE_OUT` is **not** set and `$out` is not expandable.
- `FROM == TO` is admitted, which is what makes a maintenance entry such as "upgrade the bundled npm" expressible without a second mechanism.

## AV-FMT-016 — `vibe app migrate` and `vibe-app-migration/1` (Phase 6)

Status: **delivered 2026-11-10** (phase P6). New command, new format. Four as-built differences from
the entry below, each recorded in `PROP-075#P6-AS-BUILT`: the command also takes **`--registry
DIR`**, because a migration step is the package's own script and both versions' package trees must
be reachable — every other package-reading `app` subcommand has the same flag; the **same** record
`vibe-app-migration/1` is both the machine answer and the document kept inside the work directory
(`migration.json`), which is what `--resume` reads, so there is one format and not two; the refusal
code is **`migration_consent_required`**, not `consent_required`, which `AgentError::ConsentRequired`
(PROP-064) already holds and which the inventory test
`every_code_is_unique_snake_case_and_explained` keeps globally unique — the class (a refusal, exit
2) and the printed command are exactly as specified; and the outcomes land in the
`vibe-user-applications/7` inventory this phase delivers (AV-FMT-017) beside the **declared** entries
the same record now retains, so an offer and `--only ID` need no package tree.

- New record `[format.cli-application-migration]`, epoch **1**, `schema = "schemas/application_migration.jtd.json"`, protocol tag `vibe-app-migration/1`.
- `vibe app migrate SLOT FROM TO [--only ID]… [--all] [--yes] [--resume] [--json]`. Entries run independently in declaration order. An attended terminal offers them with the `default` ones preselected; an unattended run without `--yes` refuses `consent_required` and prints the exact command. `install -g` / `update -g` that gives a slot a new version, and `vibe app use` that moves its default, make the same offer and never migrate on their own.
- **Failure semantics.** A failed `export` stops that entry before its import runs and changes nothing (`export_failed`). A failed `import` **leaves the new version installed and the slot's default exactly where the user put it**, never rolls back, never uninstalls and never re-runs the export (`import_failed`). One entry's failure does not stop the others. The work directory is **always** kept, success or failure, and is listed and cleaned by `vibe app work` like any other.
- The report names, per entry, the outcome and — for a failure — **what was not carried**: the entry's name, the failing step, its log path, the work directory, and the exact `--only ID --resume` that retries it. `--resume` reuses the kept work directory and re-runs only the import when the export already succeeded.
- Exit 0 when every selected entry succeeded, 1 when any failed. Outcomes are recorded with the **new** installation.

## AV-FMT-017 — application index protocol `vibe-user-applications/7` (Phase 2, delivered by Phase 6)

Status: **delivered 2026-11-10** (phase P6, as the deferral below said it would be). The record
below is unchanged except for one widening: the `Installation` record gains a **third** optional
member beside the template table and the recorded outcomes — the **declared** `[[application.migration]]`
entries (`id`, `name`, `description`, `default`). Retaining the declarations is what lets the offer
after `install -g` / `update -g` / `app use` and a later `--only ID` work without resolving a package
tree, which the offer after `app use` cannot do at all; it rides in the same move rather than forcing
a second one. The generation is `schemas/application/e6/index.jtd.json` with corpus
`formats/corpora/application/e6`, the frozen `e5` reader is registered as
`[format.application-index-v6]`, and `vibe-app-proxy` admits `/7` in its closed set and answers
`--supports-index vibe-user-applications/7`. The writer's ladder is unchanged and now has four
rungs: a home that retains no template provenance, no declared migration entry and no outcome keeps
writing `/6`, so every existing PATH proxy goes on reading it.

*Historical note (P2's deferral, kept for the record).* The reason for the move was the entry's own
second half: `Installation` gains *both* the template table **and** the recorded migration outcomes
of AV-FMT-016, and the second of those is P6's. A `/7` generation written in P2 would have to be
written again when P6 landed, and the writer's ladder means a home with no template installation and
no migration record keeps writing `/6` either way — so nothing a user saw was waiting on it. P2
therefore left the inventory at `vibe-user-applications/6` and the proxy admission list untouched,
and carried the template provenance of a globally installed application where P2 does bind it: the
project lock (AV-FMT-002). Spike S5 measured the whole cost of the move (two `match` arms plus the
new `e6` wire module), and the phase that took it paid no discovery.

- `[format.application-index]`, epoch **5 → 6**, new schema generation `schemas/application/e6/index.jtd.json` beside `e1`–`e5`, corpus `formats/corpora/application/e6`, `recoverable = false`, `foreign_parsers = "none"`, `unknown_fields = "deny"`. In-document protocol tag `vibe-user-applications/7`.
- `Installation` gains the optional `template` table of AV-FMT-002 (template revision, produced version, rendered-manifest hash, digest-document hash) and the recorded migration outcomes of AV-FMT-016.
- The writer still writes **the lowest protocol that carries what the home actually retains**, so a home with no template installation and no migration record keeps writing `/6` and existing PATH proxies keep working untouched.
- `vibe-app-proxy` must admit `/7` in its closed protocol set and answer `vibe-app-proxy --supports-index vibe-user-applications/7` with exit 0 and that tag, and the application mutation owner must admit and refresh the capable proxy image before publishing a `/7` index — PROP-059 `#proxy-index-upgrade`, unchanged in substance and extended to the new reader.

## AV-FMT-018 — `vibe app describe`: the template provenance (Phase 2)

Status: **still not delivered; no longer blocked.** The `/7` record AV-FMT-017 defines now exists and
`install -g` writes it: the template provenance of a globally installed application is retained, read
off this invocation's template layer at publication time (`vibe_registry::templates::active()`'s note
for the exact resolved version, so `ResolvedPackage` still grows no field — P2's as-built amendment
(3) stands). What is *not* delivered is the **reader**: `vibe app describe PACKAGE` describes a
package resolved from a registry rather than an installation, so printing the member means teaching
describe to consult the home's inventory for the described coordinate, which is a describe-shaped
change and not a migration-shaped one. P6 delivers the record and leaves the projection to the phase
that owns `describe`'s document; recorded in `PROP-075#P6-AS-BUILT`. The record below is unchanged.

- `[format.cli-application-describe]`, `schemas/application_describe.jtd.json`, epoch **1 → 1**. One optional member; existing documents byte-identical.
- Optional `template { revision, produced, rendered_sha256 }` on the described package, so a graphical tool or an agent can see that the manifest it is reading was rendered and from what, plus the installer-option rows dropped by `template_optional` with the version and the missing file.

## AV-FMT-019 — `--write-version-file` on `vibe app use` and `vibe install -g` (Phase 7a)

Status: **delivered 2026-11-10** (phase P7a); amended that day by the owner's generalisation
decision, which carries it to both version-file formats rather than `plain` alone. Design authority:
`spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-WRITE`; the as-built record is
`PROP-075#P7A-AS-BUILT`. As implemented it matches the entry below, with three additions the
flag's two surfaces forced.

- **`--json` carries the path and the version as one optional member of the document each command
  already emits**, rather than as a document of its own: `versionFile { path, version }` on
  `[format.cli-application-report]` (`schemas/application_report.jtd.json`, epoch **1 → 1**) and
  `version_file { path, version }` on `[format.cli-application-alternatives]`
  (`schemas/application_alternatives.jtd.json`, epoch **1 → 1**). Both are absent unless a file was
  written, so every existing document serialises byte-identically.
- **The flag is a usage refusal wherever it would be ignored**, naming itself: a project install, a
  remap package, a global MCP package, `--plan-only` (which resolves a version and installs none)
  and `vibe app use --none` (a slot that is off resolves to no version).
- **The written line keeps the key line's trailing comment** as well as its spacing, because
  `maven = 3.9.9  # pinned` carries one and the rewrite touches only the value region.

- No **new** wire format (see the first addition above: two existing documents each gain one optional member). One new flag, `--write-version-file[=PATH]`, on two existing commands: it writes the **resolved** version — never the requirement or the channel the user typed — into the slot's declared version file, in either the `plain` or the `keyed` format (`AV-FMT-025`). This is the general feature behind nvm's `--save` / `-w`, and it exists because a version manager that reads a version file should be able to write it.
- **Which file.** Without a value: the nearest file the `AV-FMT-011` search already found, in the directory it was found in; with no such file, the first `names` entry of the first declared row, created in the project root inside a project and in the working directory otherwise. With a value: exactly that path, whose format is decided by matching its file name against the declared `names` of every row — a path matching none is a refusal naming the path and the declared names.
- **How each format is written.** `plain` writes the one line and nothing else. `keyed` is an **in-place edit of one line**: every other line — other keys, comments, blank lines, unknown content, the file's own line endings — is preserved byte-for-byte, the key's line is rewritten in the file's own declared separator with the spacing it had, an absent key is appended as the last line, and an absent file is created holding that one line. That is what lets two slots write one `.sdkmanrc` without either destroying the other's answer.
- Refusals, all before any write: a path that is a symbolic link, naming it; a slot whose package declares no version file at all, naming the slot and the package; a `keyed` file carrying the key's line twice, naming both line numbers.

## AV-FMT-020 — the vibenvm conformance suite's authored documents (Phase 7)

Status: **planned**. New authored documents; no vibe wire format.

- `conformance/vibenvm/exclusions.toml`, `schema = 1`: one `[[exclusion]]` row per accepted gap with `subcommand`, `flag` (empty = the whole subcommand), `reason`, `approved_by` and `approved`. **A row without `approved_by = "owner"` and a date does not count as approved and the coverage gate fails on it**, so the set of accepted gaps is visible in the git log of one file and cannot be widened by a worker.
- `conformance/vibenvm/mirror.lock`, `schema = 1`: the pinned SHA-256 of every file of the local Node release mirror. A mismatch refuses to run (exit 3).
- `conformance/vibenvm/nvm-tests.allow`: the recorded subset of the pinned nvm-sh `test/fast/` suite that runs offline against the shim. Reusing nvm's own tests is a bonus, never the gate — the pinned suite is *near*-offline, not offline, and has no suite-wide mirror override.
- `conformance/vibenvm/normalise.rules`: the declared, mechanical normalisations — sandbox paths to `<HOME>`, the tool root to `<TOOLROOT>`, ANSI SGR stripped, `\d+(\.\d+)?(ms|s)` to `<T>`, a listed set of progress and notice line regexes dropped, version lists sorted. **Observation records are compared field by field; stdout and stderr text are not compared at all** unless a case opts in, and then only after these rules. No model's judgement enters the verdict.
- Verdict = exit code: `0` every case passed and coverage complete; `1` a differential mismatch (naming case, step, field, both values); `2` coverage incomplete (naming each uncovered pair) or an unapproved exclusion; `3` a harness error (mirror digest mismatch, a non-loopback connection attempt, a missing pinned tag).

## AV-FMT-101 — `applications/version-files.json`, the declared version files (Phase 5a)

Status: **done** 2026-11-10. New format, not foreseen by `PLAN.md`.

- New record `[format.application-version-files]`, epoch **1**,
  `schema = "schemas/application_version_files.jtd.json"`, `recoverable = true`,
  `foreign_parsers = "none"`, `unknown_fields = "deny"`, `corpus = "none"`. In-document
  protocol tag `vibe-app-version-files/1`.
- `<settings>/applications/version-files.json`: one entry per installation key, each with its
  slot and the `[[application.version_file]]` rows its package declares.
- **Why beside the index and not inside it.** PROP-075 `#ACTIVATION-HOOK` requires the hook's one
  call to read the home's index and the version files only, so the declarations must be retained
  by the home. Retaining them in the `Installation` record would move the inventory to
  `vibe-user-applications/7`, which needs the `vibe-app-proxy` admission PROP-059
  `#proxy-index-upgrade` requires — and that whole move is `AV-FMT-017`, assigned to phase P2.
  A sibling document keeps the index at `/6`, so every existing PATH proxy goes on reading it.
- Derived state, kept consistent by construction: `Index::load` reads the sidecar into the
  records, every publication rewrites it from them, a key the sidecar names and the index does
  not is ignored, and an empty declared set removes the document rather than writing an empty
  one. A missing or corrupt document is an empty declared set — the slot then simply has no
  version file, which is what keeps a prompt hook from failing. Losing it costs a reinstall of
  the application, which is why it is `recoverable` but not rebuildable from the home alone.
- Recorded in PROP-075 `#P5A-AS-BUILT` (1).
- **P6 decision (2026-11-10): the sidecar stays.** The question `AV-FMT-017`'s move reopens is
  whether `/7` should now absorb this document, since the index is moving anyway. It should not, for
  two reasons that outlive the move. First, the two have different recoverability: this record is
  `recoverable = true` derived state that a reinstall rebuilds, while the index is
  `recoverable = false`; folding it in would turn the loss of a version-file row into an index
  corruption. Second, it would cost the writer a rung of its ladder — a home that declares a version
  file but retains no template provenance and no migration record writes `/6` today and would be
  forced to `/7`, which needs the `vibe-app-proxy` admission PROP-059 `#proxy-index-upgrade`
  requires, for no fact the home does not already hold. Recorded in `PROP-075#P6-AS-BUILT`.

## AV-FMT-102 — `cache/version-files.json`, the resolution cache (Phase 5a)

Status: **done** 2026-11-10. New format, not foreseen by `PLAN.md`.

- New record `[format.application-version-file-cache]`, epoch **1**,
  `schema = "schemas/application_version_file_cache.jtd.json"`, `recoverable = true`,
  `foreign_parsers = "none"`, `unknown_fields = "deny"`, `corpus = "none"`. In-document
  protocol tag `vibe-app-version-file-cache/1`.
- `<home>/cache/version-files.json`: one entry per directory resolved, carrying the project root
  the search stopped at, a fingerprint of the declared set, every path consulted with its
  modification time and size **or its recorded absence**, the selection that resulted, the faults
  reported, and when the entry was last a hit.
- A hit needs the same directory, the same project root, the same declared set, every file that
  existed still unchanged and every file that was absent still absent — which is what makes a
  repeated `cd` between two project directories two cache hits (PROP-075 `#ACTIVATION-HOOK`). The
  cache is bounded to the 64 least-recently-used directories.
- A pure cache: a corrupt, unreadable or unwritable document is ignored and costs one re-read,
  never an error. This is the document PROP-075 `#VERSION-FILE-FORMATS` names as "cached by the
  modification time and size of the files consulted"; it is registered because it is persisted.
- Recorded in PROP-075 `#P5A-AS-BUILT` (2).

## AV-FMT-021 — a channel where a version requirement is written (Phase 2)

Status: **delivered 2026-11-10** (phase P2).

- `[format.manifest]` and the pkgref grammar, epoch **1 → 1**. `VersionSpec` gains a third variant `Channel(ChannelName)`, so `group/name@lts`, `node@lts/jod`, `@latest` and `@stable` parse wherever a requirement is written — a `[requires.packages]` value, a CLI pkgref, a `--use SLOT=VER`, a version file's line.
- **Semver first, always.** `VersionSpec::parse` tries `semver::VersionReq` and only a string that fails it **and** matches the channel grammar becomes a channel, so `1.x`, `^22`, `=1.2.3` and every other existing requirement keeps its meaning. A string that is neither is the refusal it already was.
- The grammar is one function in one place, `vibe_core::manifest::channel_name_is_valid`: `[a-z][a-z0-9.-]*`, optionally one `/` and a second such segment. `VersionSpec::parse` reads it, the `[[template.channel]]` validator reads it, and the version-file reader reads it, so a name that is a channel in one is a channel in all three.
- Two names are **reserved** and answered by vibe for every package, template or not: `latest` is the newest version in the domain and `stable` the newest non-prerelease. Neither is asked of a declaration.
- **Fail closed.** A channel is lowered to an exact pin *before* the solve, by the one function `vibe_core::channel::lower_channel`. Everything that reads a `VersionSpec` as a constraint treats an unlowered channel as the **empty** set — `VersionSpec::matches` is `false`, `SemverVersionSet::from_spec` is the match-nothing set, `is_exact_pin` is `false`, and every other read site either lowers first or refuses rather than picks — so a path that forgot to lower reports «no candidate» instead of installing something plausible. The enum is matched **exhaustively** everywhere it is read, so the compiler enumerated those sites rather than a survey: adding the variant turned «a missed lowering site becomes a wrong install» into a build error.
- New `[package] channels = ["lts"]` on a **concrete** manifest: publisher-assigned labels, carried into the per-version index record (AV-FMT-003). Two refusals, each naming its TOML path: a label that is not a channel name, and a label on a template — a template derives its channels from the version records with `[[template.channel]]` rules, and the two mechanisms must not both decide one package's channels.

## AV-FMT-022 — `--refresh`, a global flag (Phase 2)

Status: **delivered 2026-11-10** (phase P2). No wire format.

- `--refresh` refetches a template package's version list regardless of its age, instead of reading the cached copy while it is inside `[template] version_list_ttl` (AV-FMT-005).
- A **digest** document is immutable and is never revalidated, by this flag or anything else: a released version's checksums do not change, and a change upstream is precisely the `template_render_changed` alarm of AV-FMT-002 — which is what makes caching it forever meaningful rather than a race.
- `--offline` wins on conflict. The posture is one value, `Posture::{Cached, Refresh, Offline}`, resolved once at the composition root and installed with the invocation's template layer.

## AV-FMT-023 — `<settings>/cache/templates/`, the template input cache (Phase 2)

Status: **delivered 2026-11-10** (phase P2). New machine-local cache; no registered wire format, because nothing outside vibe reads it and every byte in it is derived.

- Layout: `versions.json` (the envelope: the source URL **after** remap, the fetched bytes' SHA-256, the RFC 3339 fetch time, and the fetched document verbatim), `digests/<version>` (the digest document verbatim, **write-once**), `rendered/v<version>/` (the materialised package directory an ordinary `Registry::fetch` then copies into the store), and `template.toml` (the declaration, AV-FMT-005).
- A **rendered tree lives here and nowhere else**, because `#TEMPLATE-RENDER-LOCATION` forbids writing a render into a registry. `vibe install -g --registry DIR` checks that a resolved package tree is contained in the directory the operator named — against a symbolic-link or `..` escape — so it admits this one root by name (`vibe_registry::templates::is_rendered_tree`), never by widening the check.
- **A template's two documents are declared as credential-free HTTPS and may be read from the filesystem.** `#TEMPLATE-DIGEST-SOURCE` refuses any other spelling in the declaration, and a `[[remap]]` row relocates the bytes — a `path` target yields a bare path, a `url` target may yield `file://`. The transport reads both off disk, joining the declared URL's last path segment when the target is a directory, which is the drop convention of PROP-066 `#drops`. A remap changes where bytes are found, never what a package declared.
- Staleness is **asymmetric because the two documents are**: a version list ages out at the TTL, a digest document never does.
- A corrupt `versions.json` is treated as **absent** rather than fatal: the cache is derived state, re-deriving it costs one request, and refusing would strand a user behind a file they never wrote by hand. A fetch that fails over a merely-aged cache logs once and uses the cached copy — the list is advisory, the digests are what bind.
- `--offline` / `VIBE_OFFLINE` / `[net] offline` read the cache whatever its age and refuse a miss with `template_cache_missing`, naming the package, the exact path and the command that would fill it. A rendered directory is rebuilt rather than reused, because a half-written one from an interrupted run must not be installed from.

## AV-FMT-024 — a channel in a version file resolves (Phase 2, completing Phase 5a)

Status: **delivered 2026-11-10** (phase P2). No wire format.

- P5a shipped `latest` and `stable` over the installed set and reported every other channel as `unresolved_channel` (`crates/vibe-app/src/activation.rs`). P2 completes it: `ActivationRequest` gains one seam, `ChannelMembership`, and a `.nvmrc` holding `lts/jod` now selects the newest installed member of that channel — which is what makes nvm's own `.nvmrc` idiom work unchanged.
- **The hook's budget is kept exactly.** The membership answer is read from the template cache alone — `template.toml` plus `versions.json`, two small file reads — so `vibe app env --project` still makes no network call, no registry call and no index-server read.
- A package that declares no such channel, or whose cache holds nothing, stays `unresolved_channel`; a declared channel with no installed member is `not_installed` with the exact `vibe install -g` command. Neither is an error, because a prompt hook must never fail a `cd`.
- `PROP-075#VERSION-FILE-FORMATS` and `#VERSION-EXEC-SPELLING`, which P5a left `impl/work` "until channel lowering lands in P2", are `impl/done` with this record.

## AV-FMT-103 — `vibe app work` lists and cleans a migration's work directory (Phase 6)

Status: **delivered 2026-11-10** (phase P6). No schema change; one new genre of row in an existing
document.

- `[format.cli-application-work]`, epoch **1 → 1**: `vibe app work` already answers rows of
  `{key, path, state, step, changed, bytes, parts}`, and a migration's kept work directory is now
  one of them, which is what AV-FMT-016's "listed and cleaned by `vibe app work` like any other"
  means in the document.
- The `key` of such a row is `migrate:<slot>:<from>-to-<to>` — a migration belongs to a **pair**,
  not to one package, so `key` is deliberately not an installation key. `state` is read from the
  kept `vibe-app-migration/1` record: `done` when every selected entry carried, `failed` naming the
  failing step, `interrupted` when an export succeeded and its import has not run, `unknown` when
  the directory holds no readable record.
- Because the rows are keyed by a slot and not a package, `vibe app work PACKAGE` never selects one
  and `vibe app work --clean PACKAGE` never removes one. `vibe app work --clean --all` does, which
  is the home-wide clean a user asks for when they want the evidence gone.
- `<home>/cache/app-migrate/` is the one new directory: a migration work directory is **never**
  replaced by a later run, because it is the evidence of what was and was not carried, so each run
  of a pair takes its own timestamp and `--resume` takes the newest.

## AV-FMT-025 — `[[application.version_file]] format = "keyed"` (Phase 7a)

Status: **delivered 2026-11-10** (phase P7a). Design authority:
`spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-KEYED`; the as-built record is
`PROP-075#P7A-AS-BUILT`. As implemented it matches the entry below with one correction: the last
bullet's "no wire change" was half right, and the sidecar **does** grow one mapping.

- `[format.application-version-files]` (`schemas/application_version_files.jtd.json`), epoch
  **1 → 1**: the `row` discriminator gains a `keyed` mapping carrying `names`, `key` and the three
  closed vocabularies, because a genuinely keyed row has five fields the document had no shape for.
  Every existing document is byte-identical regardless, because the short spelling round-trips:
  a declared row that is exactly asdf's shape is written back as `format = "tool-versions"`, which
  is also what `vibe check` and a manifest round trip emit. `vibe-app-version-file-cache/1` is
  unchanged.
- A row whose vocabulary word this build does not know makes the sidecar unknown history, the
  posture it already took for an unreadable document.
- `extra = "refuse"` makes a line with a second value a **reported fault** and no selection, so the
  upward search walks on exactly as it does past an unparseable `plain` line.

- `[format.manifest]` (`schema = "none"`, authored TOML), epoch **1 → 1**. Optional and omitted when absent.
- A third `format` value beside `plain` and `tool-versions`. A `keyed` row carries `names` (an ordered array of file names), `key` (this package's key in the file), and the line syntax from a closed vocabulary: `separator` ∈ {`equals`, `whitespace`, `colon`} (no default — the author states it), `comment` ∈ {`hash` (default), `semicolon`, `none`}, and `extra` ∈ {`refuse` (default), `first`} for a line carrying more than one value after the key.
- **`tool-versions` becomes the short spelling of exactly one `keyed` row** — `separator = "whitespace"`, `comment = "hash"`, `extra = "first"` — expanded before anything reads a file, so there is one reader and one writer and the two spellings cannot drift. The old spelling keeps working byte-for-byte.
- Reading joins the precedence of `AV-FMT-011` unchanged, with two rules the shape forces: a line whose key is not this package's is **ignored and not reported** (another slot's key in a shared file is not a defect), and a key the home has no installed package for is likewise ignored. One file may therefore resolve several slots at once, each from its own key — which is why the format exists (`.sdkmanrc`, `.tool-versions`, `mise.toml`).
- Refusals, each naming its TOML path: an unknown `separator` / `comment` / `extra` value; a `keyed` row with no `key`; a `keyed` row with no `separator`.
- No wire change: `vibe-app-version-files/1` (`AV-FMT-101`) and `vibe-app-version-file-cache/1` (`AV-FMT-102`) already carry a declared row's fields as declared.

## AV-FMT-026 — `[[application.ecosystem]]` and `[[application.ecosystem.recommends]]` (Phase 7b)

Status: **not delivered.** Design authority:
`spec://org.vibevm.core/vibevm/common/PROP-075#ECOSYSTEM-ADAPTER` and `#ECOSYSTEM-RECOMMENDS`.

- `[format.manifest]` (`schema = "none"`, authored TOML), epoch **1 → 1**. Both tables optional and omitted when absent, so every existing manifest serialises byte-identically.
- New `[[application.ecosystem]]` rows: `id` (an opaque token — vibe never interprets it), `name`, optional `description`, optional `migration` (boolean, default `true`), and the verbs `install` (required), `list`, `remove`, `self_upgrade`. Each verb is **exactly one `StepDecl`** restricted to `kind = "recipe" | "shell"`, `kind` required — the restriction `AV-FMT-015` already imposes on a migration side, for the same reason.
- **`install` and `remove` must be `kind = "recipe"`**: items are appended to the step's argv as separate elements and are never interpolated into a command string, and a `shell` step is a command *string* into which a list could only be spliced. A `shell` `install` or `remove` is a validation refusal naming the field and the reason. That is what makes the injection rule structural rather than a style note.
- New `[[application.ecosystem.recommends]]` rows: `ecosystem` (an id the package declares), `id` (a stable token), `item` (an opaque string), `default` (boolean), optional `name` / `reason`. Each row **synthesises one `[[installer_options.features]]` row with that `id` and `default` in every installer option of the package**, so `--feature` / `--no-feature`, the attended multi-select, declared profiles and the recorded chosen set (PROP-065 `#features`) all apply with no new vocabulary.
- Refusals, each naming its TOML path: two `[[application.ecosystem]]` rows with one `id`; an `id` that is not a token; an `[[application.ecosystem]]` with no `install`; a recommends row naming an undeclared `ecosystem`; a recommends `id` colliding with an authored feature of any option; a migration entry whose `id` is in the reserved `ecosystem:` namespace.
- New environment for an adapter verb's child, beside the ones PROP-065 `#templates` passes: `VIBE_ECOSYSTEM_ID`, `VIBE_ECOSYSTEM_ITEMS` (one item per line), `VIBE_ECOSYSTEM_ITEM_COUNT`, and `VIBE_ECOSYSTEM_LIST` (the file a `list` verb writes, one item per line). `VIBE_OUT` is **not** set and `$out` is not expandable: an adapter produces no payload.

## AV-FMT-027 — `vibe-app-ecosystem/1`, the adapter run record (Phase 7b)

Status: **not delivered.** Design authority:
`spec://org.vibevm.core/vibevm/common/PROP-075#ECOSYSTEM-ADAPTER`.

- New format `[format.cli-application-ecosystem]`, `schemas/application_ecosystem.jtd.json`, epoch **new, 1**, protocol tag `vibe-app-ecosystem/1`.
- Written as `<work>/ecosystem.json` in the run's own work directory `<home>/cache/app-ecosystem/<slot>/<version>/<timestamp>/`, which is kept on success and on failure as a migration's is, and is listed and cleaned by `vibe app work` exactly as `AV-FMT-103` does for a migration.
- Carries, per run: the slot, the installation key, the adapter id, the verb, the items asked for (verbatim, opaque), the outcome, the per-verb log path, and for a `list` verb the items observed.
- `recoverable = true`: it is evidence of a run, and a lost record costs one re-run, never an error.

## AV-FMT-028 — `~/.vibe/config.toml` `[slot.<slot>.extras.<ecosystem>]` (Phase 7c)

Status: **not delivered.** Design authority:
`spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS`.

- `[format.user-config]` (`schema = "none"`, authored TOML), epoch **1 → 1**. `UserConfig` is `deny_unknown_fields`, so the new top-level `[slot]` table must be added to the struct or a user who writes it gets a parse error.
- `[slot.<slot>.extras.<ecosystem>]` carries `add` (an array of opaque items) and `drop` (an array of **recommendation ids**, not items — dropping is opting out of a named author recommendation).
- **This is the first user-authored per-slot configuration in vibe**, and `crates/vibe-app/` reads no user config at all today, so the phase introduces that read as a new seam.
- An unknown slot, an unknown ecosystem id or an unknown recommendation id is **reported once and otherwise ignored**, never an error: a hand-written settings file outlives the packages it mentions.
- Two new flags on `vibe install -g` / `vibe update -g`: `--no-extras` (skip the user's own `add` list) and `--no-recommends` (skip the author's recommendations of both kinds — `AV-FMT-030`). A `drop` is honoured in every case, including under `--no-extras`, because a flag that declines extras must never cause **more** to be installed than the user asked for.
- New command family `vibe app extras list|apply|add|drop|remove|self-upgrade|import`, whose `--json` answer is `AV-FMT-029`'s `vibe-app-extras/1`.

## AV-FMT-029 — `vibe-app-extras/1` and the inventory's adapter record and weak edge (Phase 7c/7d)

Status: **not delivered.** Design authority:
`spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS`, `#ECOSYSTEM-ADAPTER`, and
`spec://org.vibevm.core/vibevm/common/PROP-077#APP-RECOMMENDS-MECHANICS`.

- New format `[format.cli-application-extras]`, `schemas/application_extras.jtd.json`, epoch **new, 1**, protocol `vibe-app-extras/1`: the answer of `vibe app extras list --json` — per ecosystem, the author recommendations with their enabled state, the user's `add` and `drop`, the computed final list, and the last observed list with its time.
- `[format.application-index]` moves **6 → 7** (`schemas/application/e7/index.jtd.json`, a new generation beside `e1..e6`), protocol tag `vibe-user-applications/8`. `Installation` gains (a) one record per declared adapter — the last applied list, the last observed list and their times and outcomes — and (b) the **weak** attribute on the requirer edge PROP-065 `#uninstall-tree` already carries, which is the whole of the new state a recommended application needs.
- The writer still writes **the lowest protocol that carries what the home retains**, the ladder `AV-FMT-017` established, so a home with no adapter record and no weak edge keeps writing `/7`.
- `vibe-app-proxy` must admit `/8` and answer `--supports-index` for it, which PROP-059 `#proxy-index-upgrade` requires of every generation.
- A weak edge never blocks a removal in either direction; an installation with no strong and no weak requirer left is an orphan for `vibe app autoremove`, and one the user installed by name stays `manual` and is never autoremoved.

## AV-FMT-030 — `--no-recommends`, a global flag, and `[recommends]` on the application plane (Phase 7d)

Status: **not delivered.** Design authority:
`spec://org.vibevm.core/vibevm/common/PROP-077#NO-RECOMMENDS-FLAG`, `#APP-RECOMMENDS`, `#SUGGESTS-UNCHANGED`.

- **No manifest change at all**: `[recommends]` and `[suggests]` already exist and already parse for every package-role manifest. This is a reach change, not a grammar change.
- New global flag `--no-recommends`, declared once beside `--offline` and `--refresh`. Honoured by `vibe install` / `update` of a project (the solver's greedy weak expansion is not run), `vibe install -g` / `update -g` and an application install into a project home (the recommendation walk is not entered), `vibe reinstall` of either plane, and the **ecosystem** recommendations of `AV-FMT-026`. On a command that neither solves nor installs it is a usage refusal naming the flag.
- **No lock field and no epoch step for the flag**: a project lock already records a recommended package as a non-root graph member, so a lock-replaying install is unaffected and the flag changes a solve, never a replay. There is deliberately no `--with-recommends` and no persisted setting.
- `[format.cli-install-report]` (`schemas/install_report.jtd.json`), epoch **1 → 1**: optional `recommended[]` (installed), `skipped[]` (with the reason — not an app, no option for this platform, unsatisfiable, not carried, install failed) and `suggested[]` (reported, never installed). Existing members unchanged.
- A failed recommended install never changes the exit status of the install the user asked for.

## AV-FMT-031 — `[[extension]]` at a `slot:` point gains `title`, `when`/`unless`, `needs`, `optional` (Phase 7e)

Status: **delivered** 2026-11-10, epoch unchanged (`1 → 1`). Design authority:
`spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-reach` and the four facts beside it;
`spec://org.vibevm.core/vibevm/common/PROP-054#SLOT-CONTRIBUTION-JUDGMENT`.

- `[format.manifest]` (`schema = "none"`, authored TOML), epoch **1 → 1**. Four optional fields on an `[[extension]]` row, omitted when absent, so every existing manifest serialises byte-identically. `[hooks]` keeps working verbatim as the one-contribution short spelling it already is.
- `title` (the human line a plan and a panel print), `when` / `unless` (PROP-065 `#conditions`' closed vocabulary, with `feature` and `param` reading the **package-scope** configuration PROP-069 `#AUTHORED-DEFINITIONS` already admits for every kind), `needs` (PROP-065 `#needs`, checked for every contribution that will run before anything starts, refusing the package's installation up front), `optional`.
- **No widening of `[[installer_options]]`**: its two refusals (`requires [application] identity`, `requires [package] kind = app`) stay, and PROP-065 `#ordinary-reach` now records why — an installer option is a per-platform alternative that produces a payload, which an ordinary package has not got.
- Fields refused **by name** on a `slot:` contribution rather than ignored: `requires` (the application dependencies of PROP-065 `#app-dependencies`), `phase` (the point already is the moment), `result`, and any template variable of `$out`, `$sources`, `$app_root` or `$app_bin`.
- Failure keeps PROP-020's law: a failing `slot:pre-install` aborts that package's install and rolls the slot back; a failing `slot:post-install` leaves it installed and flagged. There is no `--resume` and no `--from-step` on this plane; the retry is `vibe reinstall <group>/<name>`.

**As built**, with the three places the delivery is narrower or wider than the record above:

1. **One table is one conjunction.** `when` / `unless` take a single table whose every key is a condition — all must hold, none may hold — rather than PROP-065 `#conditions`' array of tables. The conjunction the array existed for is expressible in the one table the `when` anchor already carries (`when = { command_present = "cargo", offline = false }`), so no array spelling is admitted and none is needed. A key outside the vocabulary is a refusal naming the key.
2. **The four fields travel as one optional carrier** (`judgment`) on `ExtensionDecl`, so a row that declares none of them keeps `None` and every manifest written before this grammar serialises byte-identically — the record's own promise, now structural. At a `slot:` point `when` is this vocabulary; at every other point it stays the opaque pre-selection guard it has always been, and the two are never both populated. The four are refused by name at any non-`slot:` point.
3. **`feature`, `param` and `param_equals` answer `false` today**, and the plan prints the condition that decided. PROP-069 `#PACKAGE-ADMISSION` still refuses to install any package that declares `[configuration]` at all (`crates/vibe-install/src/configuration_admission.rs`), so the selected set is empty by construction; the evaluator takes it through a closure rather than a branch, so lifting that interim refusal reads the real snapshot with no change on this plane.

**Plus one field the record did not foresee**, because the owner decided it after this record was written: the trust an untrusted dependency's contributions need before any of them runs, `spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-trust`. No manifest field and no new flag: the trusted set is `agents.toml`'s `trusted_groups` unioned with PROP-020's default hook allow-list, the per-run escape is the existing `--assume-yes`, and plain `[hooks]` keeps the behaviour it measurably has (unasked — the lifecycle path consults no `HookPolicy`), so no published package changes behaviour.

## AV-FMT-032 — `vibe-install-steps/1`, the ordinary package's step evidence, and `vibe app describe`'s adapters (Phase 7e/7b)

Status: **not delivered.** The `vibe-install-steps/1` half is P7e's and was deliberately not taken in P7e's first pass: the judgment, the two refusals and the trust gate land first because everything else depends on their verdicts, and the evidence record is a new registered format with its own schema, registry row, generated reader and corpus. The per-contribution outcome rows it would serialise already exist in memory as `SlotLifecycleReport` (now carrying `skip` with its deciding condition), so what remains is the format, not the measurement. The `vibe app describe` half is P7b's throughout. Design authority:
`spec://org.vibevm.core/vibevm/common/PROP-065#ordinary-plan`;
`spec://org.vibevm.core/vibevm/common/PROP-075#ECOSYSTEM-ADAPTER`.

- New format `[format.install-steps]`, `schemas/install_steps.jtd.json`, epoch **new, 1**, protocol `vibe-install-steps/1`: written as `steps.json` in `<home>/cache/install-steps/<group>/<name>/<version>/<timestamp>/`, one row per `slot:` contribution with its `id`, handler kind, outcome, reason and start/finish times, beside `logs/<id>.log`. `recoverable = true`; it is derived evidence, listed and cleaned by the ordinary cache surface, and nothing resumes from it.
- `[format.cli-application-describe]` (`schemas/application_describe.jtd.json`), epoch **1 → 1**: optional `ecosystems[]` carrying the declared adapters (id, name, which verbs are declared) and, per adapter, the last applied and last observed item lists from `AV-FMT-029`'s record — so a GUI or an agent can say what a version carries without running anything. Independent of the still-undelivered `AV-FMT-018`, which is the template provenance in the same document.
