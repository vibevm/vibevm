# Corporate-env campaign: format and behaviour changes

Append-only log of user-visible changes made by the campaign in `PLAN.md`, material for release notes. Entries are `CORP-FMT-NNN`; numbers are never reused.

## CORP-FMT-001 — Consented durable-PATH repair core; `self doctor --fix` error text (Phase 8)

Status: **in development** (uncommitted worker change, 2026-11-08).

- New internal entry `ensure_shim_dir_on_durable_path` (`crates/vibe-cli/src/commands/vvm/env_path_fix.rs`) that the coming `vibe self apply` calls: it reports whether the shim directory is on the durable PATH and changes it only with consent. Precedence: explicit flag (`--fix-path` / its negation) → site-profile `[path] fix = "ask" | "always" | "never"` → an interactive question (default **yes on Windows, no elsewhere**); unattended without a flag or `always` never writes. Its JSON observation (for the future `apply` report): `shim_dir`, `outcome` (`already_on_path` | `applied` | `skipped`), `was_on_durable_path`, `on_durable_path`, `consent` (`flag` | `profile` | `interactive` | null), `skip_reason` (`flag_declined` | `profile_never` | `not_interactive` | `declined` | null), `changed`, `activation_hint`, `fix_hint`, `durable_environment` (the existing observation object). No command emits it yet.
- `vibe self doctor --fix` now writes PATH through that core with its existing consent (`--yes` or an interactive yes; unchanged). Visible difference: when the durable write fails, the error now reads ``putting `<shim dir>` on the durable PATH at <target> failed: <cause> (violates REQ spec://org.vibevm.core/vibevm/common/PROP-019#path); fix: …`` instead of the bare cause. A target that is already canonical is no longer rewritten (it was already a byte-level no-op).
- Windows: `HKCU\Environment` logic now sits behind a user-registry seam; real behaviour is unchanged (direct `RegSetValueExW`, kind preserved, `WM_SETTINGCHANGE`, no child process). Manual test: `vibevm/vibespecs/manual-tests/MT-07-windows-user-path.xml`.

## CORP-FMT-002 — `cargo xtask dist gitverse-publish`: mirror a release onto GitVerse (Phase 5b)

Status: **in development** (uncommitted worker change, 2026-11-08). Release tooling only; no `vibe` format changes.

- New explicit step `cargo xtask dist gitverse-publish --version V [--from-dir DIR] [--dry-run] [--replace] [--api-base URL] [--owner O] [--repo R]`. No other `dist` command (and no CI workflow) runs it, so the owner decides when a release appears on GitVerse. Defaults: `https://api.gitverse.ru`, `vibevm/vibevm`; `--api-base` points it at an on-prem GitVerse.
- Source: the published GitHub release `vV`, read anonymously, or a local directory holding the same 19 assets (`--from-dir`). Nothing is sent until `DISTRIBUTIONS.json` names `vV` and every bundle, bootstrap and fragment matches it byte for byte (the `dist finalize` verification); the three installers must be present and non-empty.
- Target: the release for tag `vV` is reused when it exists (by tag, else found in the release list, which also sees a draft) and created otherwise as a public release (`draft: false`, `is_authorized_only: false`, `target_commitish` = the manifest's source commit). Assets go up through `POST /repos/{owner}/{repo}/releases/{id}/assets?name=…` (multipart field `attachment`), `DISTRIBUTIONS.json` last; a final listing checks every name and size.
- Idempotent: a same-named asset of the same size whose downloaded SHA-256 matches is skipped; any other same-named asset stops the run before any upload unless `--replace`, which deletes the differing assets (`DISTRIBUTIONS.json` first) and uploads them again. Duplicate remote names stop the run. Extra remote assets are reported and left alone.
- Token: `VIBEVM_PUBLISH_TOKEN_GITVERSE`, then legacy `VIBEVM_PUBLISH_TOKEN`, then `~/.vibe/gitverse.publish.token`, then legacy `~/.vibe/git.publish.token` (the existing per-host convention), sent as `Authorization: Bearer` with `Accept: application/vnd.gitverse.object+json;version=1`; output names where the token came from, never its value, and error bodies are redacted. `--dry-run` loads no token, contacts no GitVerse endpoint and prints every request it would send.

## CORP-FMT-003 — Artifact drops: `[artifacts]`, package payload drop, `file://` targets, `vibe artifacts where` (Phase 4)

Status: **in development** (uncommitted worker change, 2026-11-08).

- New `~/.vibe/config.toml` table `[artifacts]` with `vibe = "<dir>"` (release drop, see CORP-FMT-004) and `packages = "<dir>"` (application payloads). Paths must be absolute; a relative one is ignored with a warning.
- `packages` lowers into one `local`-layer remap row (provenance source `[artifacts] packages`, key `prefix = ""`, kind `app-input` only) — visible in `vibe remap list` / `explain`. It is the weakest row a user can have: every user, project or invocation row still wins; the declared origin stays the last candidate.
- Lookup for an input `url + sha256` of package `G/N@V`, file name `F` = last URL path segment: `<drop>/G/N/V/F`, `<drop>/G/N/F`, `<drop>/<sha256>/F`, `<drop>/F`. The digest is always verified (`digest_mismatch`, file not kept). A miss is `fetch_failed` naming `F`, the digest and every searched path; when the origin's host is excluded by source policy the drop's miss is now reported first (before: only the `source_contact_refused` refusal).
- Remap behaviour change: a `prefix` row with a `path` target now joins the URL remainder onto the directory (mirror layout; query/fragment dropped, `..` dropped). For application inputs the directory itself is still searched by digest afterwards, so existing digest-layout stores keep working.
- One location rule (`ByteLocation`, PROP-066 `#locations`): `https://`/`http://` remote; `file://` URL and `path` local; anything else refused. A remap `url = "file:///…"` target for an application input is now read locally (before: refused as not package-relative).
- New command `vibe artifacts where [<package>] [--registry DIR] [--json]`; JSON protocol `vibe-artifacts-where/1`: `release {source, location, from, example_dir, files}`, `packages`, `inputs[] {option, step, input, file, sha256, url, searched[], status: present|mismatch|missing|no-drop}`.
- `vibe remap explain <url>` asks about a URL under `https://github.com/vibevm/vibevm/releases` as a `distribution` artifact (before: `app-input`), so release rows show.

## CORP-FMT-004 — Release sources: `[self]`, `vibe self source show|set|reset`, `vibe self releases`, drop/mirror-aware update (Phase 5)

Status: **in development** (uncommitted worker change, 2026-11-08).

- New `~/.vibe/config.toml` table `[self]`: `source = "github" | "gitverse" | "nexus" | "localdir"` (case-insensitive, written lower-case), `url` (nexus base / gitverse repository web URL), `api` (gitverse API base, default `https://api.<host>`), `path` (localdir; default `[artifacts] vibe`).
- Precedence: `$VIBEVM_RELEASE_ORIGIN` → `[self]` → `[artifacts] vibe` → GitHub. The environment variable keeps its invocation-layer row (prefix `https://github.com/vibevm/vibevm/releases`, provenance `VIBEVM_RELEASE_ORIGIN`); its row `reason` text is now `release source \`mirror\` from $VIBEVM_RELEASE_ORIGIN` (was `$VIBEVM_RELEASE_ORIGIN`). `localdir`/`nexus` add one `local`-layer row (prefix `https://github.com/vibevm/vibevm/releases/download`, `distribution` only, source `[self] source` or `[artifacts] vibe`). An unusable source is a warning on every command and a refusal on release verbs.
- `vibe self source` with no subcommand still prints the source tree (unchanged); new `show` (`vibe-self-source/1`: `source, location, from, row{prefix,target,layer}`), `set <type> [--url] [--path] [--api]` (validates, rewrites `config.toml` via the config serializer — comments in that file are not preserved), `reset`.
- New `vibe self releases [--all]` (`vibe-self-releases/1`: `source`, `releases[] {version, installed, active, running}`, `truncated`); newest ten without `--all`.
- Release drop layout `<drop>/v<version>/` with the release page's files; listing = directory scan (a `v<version>/` with `DISTRIBUTIONS.json` or this target's zip, or a bare `vibevm-<version>-<target>.zip` at the root). A bare zip is installed from its embedded `DISTRIBUTION.json` with a hint that it was verified against the embedded manifest only.
- Raw repository listing: optional `<base>/RELEASES.json` (new document: `{"schema_version": 1, "releases": [{"version": "X.Y.Z"}]}`; bare strings / bare array accepted), else Nexus search REST, else refusal. GitVerse: `GET <api>/repos/<owner>/<repo>/releases[?page,per_page]`, `…/releases/tags/v<V>`, assets via `browser_download_url`, `Accept: application/vnd.gitverse.object+json;version=1`, bearer from the per-host token chain when present (never printed, never sent to another origin).
- `self update` on `nexus`/`localdir`/`gitverse` takes the highest listed version (GitHub and `$VIBEVM_RELEASE_ORIGIN` still use `latest/download`).
- Transport: release downloads now go through the operation's controlled transport (the `install -g` client) instead of a private reqwest client: `exclude_hosting = ["github"]` refuses `self update` before contacting github.com; in the controlled posture the bound client's no-implicit-proxy rule applies (as for `install -g`). Requests no longer send `Accept: application/octet-stream`/`Cache-Control`/a `User-Agent` on payload downloads (the cache-busting nonce stays); list API requests send `User-Agent: vibevm/<version>`.
- One location rule for bootstrap/update/install: a remapped release base may be `https://`, `http://` (newly accepted for `self bootstrap`), `file://` or a `path`; local release files carry no `?vvm_nonce` and are read under `--offline` (before: every release verb was refused offline).

## CORP-FMT-005 — Author-side drop inputs `drop = "packages"` + `file` (Phase 6)

Status: **in development** (uncommitted worker change, 2026-11-08).

- Application `sources` inputs gain optional `drop` and `file`: exactly one of `url`, `path`, `drop` (validation message now `must declare exactly one of url, path or drop`); `drop` must be `packages`; `file` is one portable name and only with `drop`.
- Resolved in every configured `[artifacts] packages` drop by the CORP-FMT-003 convention, digest-verified, cached by digest; no drop configured → `fetch_failed` naming `[artifacts] packages`.
- Wire: optional `drop`, `file` added to `source_input` in `schemas/application_describe.jtd.json`, `schemas/application_installer_options.jtd.json`, `schemas/configuration/e1/description.jtd.json` (generated types regenerated); omitted when absent, so existing documents are byte-identical.
- `archive` / `native` step `source` is unchanged (URL or package-relative path).

## CORP-FMT-006 — Registry `type`, the project-local `registry.local.toml` layer, `vibe registry add|list|remove --scope/--type` (Phase 1)

Status: **in development** (uncommitted worker change, 2026-11-08).

- Every `[[registry]]` (in `vibe.toml`, `~/.vibe/registry.toml` and the new `<project>/.vibe/registry.local.toml`) accepts `type = "github" | "gitverse" | "git" | "monorepo" | "localdir"`, case-insensitive on input, written lower-case. Absent, it is inferred: `file:` url → `localdir`, host `github.com` → `github`, host `gitverse.ru` → `gitverse`, anything else → `git`; `monorepo` is never inferred, so every existing file keeps its meaning. An unknown value is refused with the list of values; `type = "localdir"` with a remote url is refused. An explicit `type` now chooses the backend (`git` on a `file:` url clones it as Git; `localdir` on a bare path reads it as a directory).
- New layer `<project>/.vibe/registry.local.toml` (same grammar as `registry.toml`; `vibe init` already ignores it — `.vibe/.gitignore` is `*` and the root `.gitignore` carries `/.vibe/*.local.toml`). Precedence, highest first: invocation `--registry <dir>` → `registry.local.toml` → `vibe.toml` → `~/.vibe/registry.toml` → site profile (reserved, Phase 7) → built-in defaults. A registry `name` in a higher layer hides the same name below; mirrors concatenate in layer order; overrides dedupe by `pkgref`. Applies to install/update/resolution and to the `[[mirror]]` rows the remap Locator reads.
- `vibe registry add|remove registry|remove mirror` gain `--scope project|local|user` (default `project`, today's behaviour); `vibe registry add` gains `--type`, refuses `--naming` for `monorepo`/`localdir`, and accepts a `file:` url for `localdir`. Writing `user`/`local` re-serialises that TOML file (comments in it are not kept). `--scope site` is refused (the site profile is applied as a whole).
- `vibe registry list` without `--scope` now lists the effective rows of every layer (also outside a project: the user file alone) instead of only `vibe.toml`; `--scope X` lists one layer. Each JSON `registries[]` row gains `type` (effective) and `scope` (`local|project|user|site`); `adapter` follows the type (`github`, `gitverse`, `monorepo`, `localdir`; `null` for `git`). Human output gains `type:` and `scope:` lines.
- `vibe registry publish` chooses its target from the same effective layers (project-local, `vibe.toml`, `~/.vibe/registry.toml`); before, only `vibe.toml` was read. A project with no `[[registry]]` of its own now publishes to the first user-file registry instead of refusing.
- Spec: `spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-type`, `#registry-layers`.

## CORP-FMT-007 — `monorepo` registries: resolution, lock rows, replay (Phase 2a/2b)

Status: **in development** (uncommitted worker change, 2026-11-08).

- `type = "monorepo"`: one Git repository (any url Git accepts; SSH uses the ordinary `~/.ssh` configuration) holding packages at `<group>/<name>/v<version>/` on `ref` (default `main`). Cloned on first use — through the Locator (`[[remap]]` rows, then the canonical url) and the operation's Git backend, so source policy and admission apply; under a controlled transport (any `exclude_hosting`/`prefer_hosting`) it is captured with native preparation into an operation-owned snapshot. `--offline` never contacts it. `auth = "token-env"` is refused (`MonorepoTokenAuth`).
- **Lock row format (new meaning, no new field):** `source_kind = "registry"`, `registry = "<name>"`, `source_url = "<registry url verbatim>#<group>/<name>/v<version>"`, `source_ref = "<branch>"`, `resolved_commit = "<commit read>"`. The url is kept byte-for-byte (an scp-form `git@host:path` stays home-relative). A registry row whose `source_url` carries such a fragment matching its own coordinate is a monorepo row.
- Replay (PROP-073 `#LOCKED-SOURCE-LAW`): a held monorepo row installs from the store, or reads its directory at the held commit (never the branch tip; a mismatching hash or commit refuses); `vibe install <pkg>@<req>` / `vibe update` move the version on the held repository and branch (the fragment and commit follow); `vibe update --sources-only` moves a row from another registry onto a monorepo keeping version and bytes. In-place materialisation is refused (`InPlaceUnsupported`).
- The legacy library type `GitMonorepoRegistry` now records provenance on fetch: `registry_name` (when named), `source_ref` = branch, `resolved_commit`, `fetched_from` when a candidate other than the canonical url served; its `source_uri` changed from `git+ssh://host/path#…` to the verbatim url `+ #<group>/<name>/v<version>`.
- Test fixture: `ssh_fixture.py` honours `VIBE_SSH_FIXTURE_WRITABLE=1` (also admits `git-receive-pack` on its one repository).

## CORP-FMT-008 — Publishing into `monorepo`, `localdir` and generic `git` registries; `--replace` (Phase 2b/3)

Status: **in development** (uncommitted worker change, 2026-11-08).

- `vibe registry publish` dispatches on the target registry's type. `monorepo`: shallow-fetch the branch, write `<group>/<name>/v<version>/`, commit (`publish: <group>/<name>@<version>`, or `replace: …`; the user's Git identity, else `vibevm publisher <publish@vibevm.local>`), `git push <url> HEAD:refs/heads/<ref>` — no host API, no organisation, no tag; an empty repository gets the branch. A non-fast-forward rejection is refetched and retried once, then refused (`MonorepoRaceLost`, nothing pushed). `localdir`: copy into a hidden sibling `.v<version>.incoming-*` and rename into place (a replaced version is renamed aside first, then removed); no locking. `git` (generic server): push into the package's existing repository `<url>/<repo name>.git` per `naming`, as `--repo-url` does; before, publish refused the unknown host.
- Immutable versions for `monorepo`/`localdir`: identical content (by content hash, so `.git` and build output never differ) is a no-op reported `unchanged`; different content is refused with `VersionExists` unless `--replace` (new flag). `--dry-run` decides and writes nothing. `--index-checkout`/`--push-index` are refused for these two types.
- New JSON report for these two types: `{ok, command: "registry:publish", mode: "monorepo"|"localdir", registry, target, package, version, package_path, action: "created"|"replaced"|"unchanged", commit?, retried, content_hash, dry_run}`.

## CORP-FMT-009 — `[[mirror]]` rows reach every command (integration fix)

Status: **in development** (uncommitted integration change, 2026-11-08).

- Bug (since the deferred registry catalog, before this campaign; made visible by the drop and release rows): on a verb that reads no registry catalog up front (`install`, `update`, `reinstall`, the lifecycle verbs, …) the clone chain of a registry used the `[[mirror]]` tables only when the invocation's remap Locator was empty — and in the CLI it never is (the built-in single-file rows, a `[[remap]]` row, the `[artifacts] packages` drop row or the release-source row). Every mirror was silently ignored there; only catalog verbs (`registry`, `search`, `outdated`, `cache`, `init`) honoured them.
- Now the catalog composes its effective `[[mirror]]` tables into the invocation Locator's `[[mirror]] / [[override]]` slot whenever it builds a registry's clone chain (`Locator::with_registry_tables`; one lowering, `vibe_core::remap::rows_from_registry_tables`, shared with the composition root). Precedence is unchanged and the same on every verb (`PROP-066#precedence`): the most specific key first; among equals the higher layer, then the later row — so an explicit `[[remap]]` row of the local layer naming the same registry beats the mirror, an invocation `--remap` row beats both, and the drop (`app-input` only) and release (`distribution` only) rows never apply to a registry clone. The declared registry url stays the last candidate.
- Locked replay is unchanged: a held row still consults operator remap rows only (`PROP-073#LOCKED-SOURCE-LAW`).

## CORP-FMT-010 — Comment-preserving writes of `config.toml`, `registry.toml`, `registry.local.toml`

Status: **in development** (uncommitted integration change, 2026-11-08). Supersedes the "comments are not kept" notes of CORP-FMT-004 and CORP-FMT-006.

- `vibe self source set|reset`, `vibe init` (the remembered `[init] last_author`) and `vibe registry add|remove registry|remove mirror --scope user|local` rewrite only the keys and tables whose value changed (`vibe_core::toml_preserve`, over `toml_edit`): unchanged tables, `[[registry]]`/`[[mirror]]` elements and keys keep their bytes, comments and quoting; a removed element disappears with its own lines; a new element is appended after its array; a changed key keeps its trailing comment. Saving an unchanged value is a byte-identical no-op. If the patched text would not read back as exactly the saved value (or the existing file is malformed), the plain serialisation is written as before.
- `--scope project` (`vibe.toml`) keeps its existing decoration-merge writer.

## CORP-FMT-011 — `registry_layout_publish_report` schema; monorepo publish transfer classification

Status: **in development** (uncommitted integration change, 2026-11-08).

- The CORP-FMT-008 JSON report of `vibe registry publish` into `monorepo`/`localdir` is now a registered wire format: `schemas/registry_layout_publish_report.jtd.json`, `formats/REGISTRY.toml` `[format.cli-registry-layout-publish-report]` (epoch 1), generated type `vibe_wire::generated::registry_layout_publish_report::RegistryLayoutPublishReport`. `mode` (`monorepo` | `localdir`) and `action` (`created` | `replaced` | `unchanged`) are closed vocabularies; `commit` is optional (`x-default: null`). Bytes on the wire are unchanged.
- `PROP-068#coverage`: the monorepo publish's `git fetch` is classified as a publish-side read (exempt, like the per-package `git_publish.rs`): it shallow-fetches the branch it is about to push onto, from the same url, so it must not be redirected by a remap or mirror (a mirror's lagging branch would be the base of a push to the origin). Publish paths do not consult source policy (none of them does; the target is the registry the operator named).

