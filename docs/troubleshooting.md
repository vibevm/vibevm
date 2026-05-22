# Troubleshooting

First-aid for the errors `vibe` surfaces. Each entry: what you see, what it means, what to do.

If you don't find your error here, the message is probably from a layer below `vibe` — usually `git`. Run with `RUST_LOG=vibe_registry=debug,vibe_publish=debug vibe …` to see the underlying command line and stderr.

---

## Install errors

### `package `…` is already installed at version `…` — use `vibe update` instead`

**What.** `vibe install <pkgref>` was called for a `(kind, name)` that already has a lockfile entry, and the freshly-fetched `content_hash` matches the locked one — same content, no work to do.

**Why.** vibevm refuses to silently overwrite an installed package. `vibe install` is for new installs; reinstalling a different version is `vibe update` (M1.2, not yet shipped) or, today, `vibe uninstall <pkgref> && vibe install <pkgref>@<version>`.

**Action.** If you wanted a different version, `vibe uninstall` first. If you wanted to verify the install, `vibe list` is the read-only path.

### `content drift on `…@…`: lockfile pins `sha256:…` but the source served `sha256:…`…`

**What.** The lockfile pins `content_hash = X` for `(kind, name, version)`, and a fresh fetch from the registry produced a different hash `Y`. PROP-002 §2.1 makes content_hash the identity, so this is a hard fail before any write.

**Why.** Three common causes:

1. **An upstream maintainer force-pushed the version tag** to a different commit. Refuse the install — the bytes you'd be installing are not the bytes the lockfile remembered.
2. **A `[[mirror]]` is serving different bytes** than canonical. The mirror is broken, malicious, or out of date.
3. **An `[[override]]` was added** that points at content whose hash disagrees with what the lockfile pinned previously.

**Action.**

- Compare the two hashes against `git ls-remote` and the actual tag commit upstream — work out *why* the registry's content changed.
- If upstream legitimately re-tagged (uncommon — bad practice), `vibe uninstall <pkgref> && vibe install <pkgref>` to re-pin to the new content.
- If a mirror is broken, remove the `[[mirror]]` entry from `vibe.toml` and reinstall.
- If you added `[[override]]` deliberately, the divergence may be intentional — `--trust-mirror` (M1.6, reserved) is the future escape hatch.

Never edit `vibe.lock` by hand to silence this — that defeats the integrity check.

### `malformed <vibevm> block in `…``

**What.** An agent instruction file (`CLAUDE.md`, `AGENTS.md`, or `GEMINI.md`) does not contain exactly one well-formed managed block — vibevm needs exactly one `<vibevm>` line followed later by exactly one `</vibevm>` line. Two of either marker, an opener with no closer, a closer with no opener, or a closer before its opener is malformed. Caught at plan time, before any `vibedeps/` materialisation; exit code `3`.

**Why.** vibevm owns only the delimited `<vibevm>` block of those shared files and writes only between the markers ([PROP-012](../spec/modules/vibe-workspace/PROP-012-managed-redirect-block.md)). A malformed block is ambiguous — vibevm never guesses which of two regions is canonical, and never auto-deletes a stray marker.

**Action.** Open the named file and repair the markers by hand so there is exactly one ordered `<vibevm>` … `</vibevm>` pair, each tag alone on its own line. Then re-run. `vibe check` reports the same defect, so you can find it before the next install.

### `package `…` is not installed`

**What.** `vibe uninstall <pkgref>` was called for a package that's not in `vibe.lock`. Exit code `1`.

**Action.** `vibe list` to see what's actually installed; check spelling.

### `the materialised vibedeps/ tree is incomplete — … slot(s) missing`

**What.** `vibe reinstall` (without `--force`) found that a package the lockfile pins has no `vibedeps/<kind>-<name>/<version>/` slot on disk. Regenerate mode reads only the materialised tree — it cannot conjure missing content. Exit code `1`.

**Action.** Re-run with `--force`: `vibe reinstall --force` re-fetches every locked package's content from source at the pinned version and re-materialises the `vibedeps/` tree. See [`docs/commands/reinstall.md`](commands/reinstall.md).

### `package declares a [boot_snippet].source `…` that does not exist in the package`

**What.** The package's `[boot_snippet]` table names a `source` file that is not actually present in the package payload.

**Action.** Either the published package is corrupt (rare; report upstream) or the cache is corrupted. Try `rm -rf <project>/.vibe/cache && vibe install <pkgref>` to force a re-fetch. (Note: the loading model retired the per-file `[writes]` list — a package's footprint is its verbatim `vibedeps/` slot — so there is no longer a per-file write-target conflict or user-owned-path write error; a dependency's content lands in `vibedeps/`, never in a node's authored `spec/`.)

### `user declined the install plan`

**What.** Interactive confirmation came back "no". Exit code `5`.

**Action.** None — the system did exactly what you asked. Re-run when you actually want to install. `--assume-yes` skips the prompt for non-interactive use; `--json` auto-approves on the assumption a script is driving.

---

## Registry errors

### `package `kind:name` is not in the registry`

**What.** `vibe install <kind>:<name>` was called for a package that doesn't exist in any configured `[[registry]]`. The resolver walked every registry and got a `RepoNotFound` from each.

**Action.**

- Spelling check — kebab-case, no version typos in `<name>`.
- `git ls-remote git@<host>:<org>/<kind>-<name>.git` to manually probe whether the per-package repo exists. If you get auth errors, the registry sees you but the package doesn't exist; if you get `Repository not found`, the org-and-repo combination is wrong.
- For private registries: confirm your token / SSH key has read access to the org.

### `no version of `kind:name` matches `…``

**What.** The package exists in the registry but no published version satisfies your constraint. E.g. you asked for `flow:wal@^0.5` and the registry only has `v0.1.0` / `v0.2.0`.

**Action.** `git ls-remote --tags git@<host>:<org>/<kind>-<name>.git` to see what's actually published. Loosen the constraint (`vibe install flow:wal` for latest stable) or pin a real version.

### `no registry configured. Pass `--registry <path>` or add a `[[registry]]` entry to `vibe.toml`.`

**What.** `vibe install` ran without `--registry <path>` and `vibe.toml` has no `[[registry]]` section. This typically means you ran `vibe init --no-registry` and never added a registry afterwards.

**Action.** Either pass `--registry <path>` for an offline / fixture-driven install, or edit `vibe.toml` to add a `[[registry]]` entry. The default GitVerse registry is `git@gitverse.ru:vibespecs` (per-package) — `vibe init` writes that automatically unless you opted out.

### `registry root `…` does not exist or is not a directory`

**What.** `--registry <path>` pointed at a path that doesn't exist or isn't a directory.

**Action.** `ls <path>` to confirm. The local-registry layout is `<path>/<kind>/<name>/v<ver>/...`; if your path doesn't match that shape, point `--registry` somewhere else or add the right tree.

### `registry meta file at `…` is malformed: …`

**What.** The cache's bookkeeping file (`<cache>/<hash>/meta.toml`) is corrupt. Usually means a partial write was interrupted.

**Action.** `rm -rf ~/.vibe/registries/<hash>` (or set `VIBE_REGISTRY_CACHE` and clean that path). The next install repopulates the cache from scratch.

---

## Git-backend errors

### `the `git` executable is not available on PATH; install git …`

**What.** `vibe-registry` shells out to system `git` for every operation. The check `git --version` failed.

**Action.** Install git for your platform per [RUNTIME-GUIDE.md §2.1](../RUNTIME-GUIDE.md). Verify with `git --version`.

### `remote repository `…` not found (does it exist? is access granted?)`

**What.** Either the repo URL is wrong or your credentials lack read access. `git` itself emitted "Repository not found" or similar.

**Action.**

- Check the URL: `git ls-remote <url>` to reproduce manually.
- Verify your SSH key / token: `ssh -T git@gitverse.ru` should authenticate, or `gh auth status` for token-based auth.
- For private registries: confirm membership in the org.

### `ssh authentication failed for `…` — check your ssh-agent / keys`

**What.** `git` connected to the host but couldn't authenticate.

**Action.**

- `ssh-add -l` lists keys currently loaded in the agent. Empty? Run `ssh-add ~/.ssh/<your-key>`.
- Confirm the *right* key is loaded — your GitVerse identity may use a different key than your default.
- On Windows / Git Bash: the OpenSSH agent may not be running. `Start-Service ssh-agent` (PowerShell, admin) or use Git Bash's `eval $(ssh-agent -s) && ssh-add`.

### `unable to reach `…` (network or DNS error)`

**What.** No route to the host.

**Action.** `ping gitverse.ru`, `dig gitverse.ru`, or `curl -I https://gitverse.ru` to confirm. If you're behind a corporate proxy, set `HTTPS_PROXY` / `HTTP_PROXY` so child `git` subprocesses inherit them.

### `branch / ref `…` not found on `…``

**What.** The expected ref doesn't exist on the remote — typically a missing version tag.

**Action.** `git ls-remote --tags <url>` to see what's actually published. The package may not yet have the requested version, or the publisher mistyped the tag.

### `file `…` not found in `…` at ref `…``

**What.** `git archive` against an existing ref didn't find the requested file. Most often: a manifest at a tag is missing for some reason, or the package layout is malformed.

**Action.** Check the per-package repo's tree at the named tag — `git archive --remote=<url> --format=tar <tag> <path>` reproduces. If the file truly is absent at that tag, the upstream is broken — report.

### `remote `…` does not support `git archive` for fetching individual files (uploadarch service refused).`

**What.** Some hosts disable the upload-archive service for clients. The resolver's "read manifest without cloning" optimization can't run.

**Action.** This is an upstream configuration issue; report to the host. As a workaround, you can `vibe install` against the package — install paths through `bootstrap` which is always supported. The optimization only matters during dep-walk for non-installed candidates.

### `git `…` exited with status …`

**What.** Generic git-failure catch-all when the stderr didn't match any classified pattern. The message includes the full command line and stderr.

**Action.** Run the printed command line manually to reproduce; the verbatim git error usually points at the cause.

---

## Publish errors

(These come from `vibe registry publish`. Full surface in [`docs/commands/registry-publish.md`](commands/registry-publish.md).)

### `publish refused: token lacks `repo:create` permission in organization `…` on `…`.`

**What.** The publish token doesn't have the right scope to create new repositories in the target org. HTTP `401` / `403` from the host.

**Action.** Mint a new token with `repo:create` (or equivalent) scope, or ask an org owner to elevate the existing one. Token is read from `VIBEVM_PUBLISH_TOKEN` first, then `~/.vibevm/git.publish.token`.

### `publish refused: no token available for host `…`. Set `VIBEVM_PUBLISH_TOKEN` or write a token to `~/.vibevm/git.publish.token`.`

**What.** No token was found in either source.

**Action.** Either `export VIBEVM_PUBLISH_TOKEN=<token>` (one-shot for this shell) or write the token to `~/.vibevm/git.publish.token` (persistent, recommended for repeat publishes).

### `publish refused: organization `…` does not exist on `…``

**What.** API returned `404` on the org-level endpoint. Distinguished from a permission error so you can tell a typo from an access issue.

**Action.** Check the spelling of the `[[registry]].url` in `vibe.toml`. The org segment is everything after the host: `git@gitverse.ru:vibespecs` → `vibespecs`.

### `publish refused: tag `…` already exists on `…`. Pick a new version`

**What.** The version tag already exists in the per-package repo. `vibe registry publish` never force-pushes.

**Action.** Bump the version in the package's `vibe.toml` `[package]` table and re-publish. If you genuinely need to overwrite a published version (almost always wrong — consumers may have lockfile-pinned content_hash), do it manually with `git push --force` and accept that downstream consumers will hit `ContentDrift`.

### `publish refused: no push access to `…``

**What.** API call to create the repo succeeded (or skipped because it existed), but `git push` was denied.

**Action.** For an existing repo: ask a repo maintainer for push access. For a freshly-created one: this should not happen unless your token has `repo:create` but not `repo:write` — re-mint with both.

### `publish refused: host `…` is unreachable (network or DNS error).`

**What.** Network problem, same as the git-backend version.

**Action.** Same as above — check connectivity and proxy settings.

### `publish refused: source directory `…` does not look like a vibevm package — …`

**What.** The directory you pointed at doesn't have a parseable `vibe.toml` carrying a `[package]` table.

**Action.** `cat <source>/vibe.toml` and verify it parses and carries a `[package]` table; check it's the package directory (with the manifest at root), not a directory of packages.

### `publish refused: cannot derive an organization segment from `…`.`

**What.** The `[[registry]].url` couldn't be parsed for an org segment. Most often: it's just a host name without a path, e.g. `https://gitverse.ru`.

**Action.** Add the org to the URL: `https://gitverse.ru/vibespecs`. The org segment is mandatory.

### `unexpected response from `…` (status …): …`

**What.** The host returned an HTTP status the publish flow doesn't classify. Body of the response is included.

**Action.** This is a "report it" case — either the host's API surface changed (a `GitVerseCreator` update is in order) or something genuinely unusual happened. Capture the full message and file an issue.

---

## Resolver errors

### `version conflict on `…`: already chose `…`, but a later constraint requires `…``

**What.** Two paths through the dep graph picked different constraint versions for the same `(kind, name)`. NaiveDepSolver is single-pass and first-pick-wins — when a later requirement disagrees with the earlier pick, it surfaces the conflict.

**Action.**

- Pin a single constraint that satisfies both paths in your `vibe install` arguments.
- Or use `[[override]]` in `vibe.toml` to break the tie deliberately.
- Future SAT-style solvers (resolvo / libsolv) will narrow constraints before picking; until then, naive resolution requires an explicit decision.

### `package `…` declares `[conflicts]` against `…`, which is also being installed in this graph`

**What.** A package has `[conflicts].packages = [...]` listing another package that's already in the resolution graph. They cannot coexist by author intent.

**Action.** Pick one. Often the conflict is an old/new pair (`feat:foo` vs `feat:foo-legacy`); pick the modern one. If you genuinely need both, file an issue with the conflicting package's author — the conflict declaration may be too aggressive.

### `capability `…` required by `…` is not provided by any package in the resolved graph.`

**What.** A `feat` (typically) declares a capability requirement (`[requires].capabilities = ["ui:landing-page-host@^0.1"]`), and no package in the resolved graph provides it.

**Action.** Install a stack package whose `[provides].capabilities` matches. The requirer's `vibe.toml` typically lists candidate stacks in `[[requires_any]]`; install one of those. If you've already installed the stack and still see this, check the version match — `^0.1` won't accept a `0.2.x` provider.

### `all alternatives in `[[requires_any]]` declared by `…` failed to resolve`

**What.** A package has a `[[requires_any]] one_of = [...]` and none of the alternatives can be resolved.

**Action.** At least one alternative needs to be available in your registries. If this is a fresh project, install one of the alternatives explicitly first.

---

## CLI / general errors

### `no `vibe.toml` in `…`; run `vibe init` first`

**What.** A project-scoped command (`install`, `reinstall`, `list`, `uninstall`, `registry sync`, `registry publish`) ran in a directory without `vibe.toml`.

**Action.** `cd` into the right directory, or `vibe init --path <project>` to scaffold one. Use `--path <dir>` (or, for `vibe reinstall`, the positional path) to point at a project from elsewhere.

### `no TTY available for confirmation; re-run with `--assume-yes` to apply this plan non-interactively`

**What.** `vibe install` / `uninstall` / `reinstall` was run in a non-TTY environment (CI, pipe, redirect) without `--assume-yes`, and would otherwise hang waiting for interactive confirmation.

**Action.** Add `--assume-yes` (or its alias `--yes`) for non-interactive runs. `--json` also auto-approves on the assumption a script is driving.

---

## When the message isn't here

1. Re-run with `RUST_LOG=debug` or scope it: `RUST_LOG=vibe_registry=trace,vibe_publish=debug vibe …` to see git command lines and HTTP request shapes.
2. The error type usually comes from one of three crates — `vibe-core` (parse / IO), `vibe-registry` (git, registry), `vibe-install` (plan, apply). The message prefix usually points at the right one.
3. If the cause is a layer below `vibe`, the error message includes the underlying tool's stderr verbatim. Run that tool's command line manually to reproduce.
4. File an issue at <https://gitverse.ru/anarchic/vibevm/issues> with: `vibe --version`, `git --version`, the full command, the full error output, and `cat vibe.toml` / `cat vibe.lock` minus any secrets.

## Related

- [`docs/commands/`](commands/) — per-command reference; each page lists the exit codes the command can produce.
- [`docs/lockfile-format.md`](lockfile-format.md) — for understanding what `vibe.lock` integrity checks against.
- [`docs/architecture.md`](architecture.md) — for the bigger picture of where each error originates.
- [`PROP-002 §2.10`](../spec/modules/vibe-registry/PROP-002-decentralized-registry.md#publish) — the publish-error surface design.
