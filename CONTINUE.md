# CONTINUE — cold-resume checkpoint

_Written: 2026-05-10 finalisation (M1.15 + M1.16 ship-complete with hermetic e2e + production smoke walks). Owner-readable, self-contained. Pick this up with zero prior context._

---

## TL;DR (executive summary)

**The 2026-05-10 finalisation push closes M1.16 from "resolver wired" to "ship-complete with operator UX + hermetic e2e + production smoke walks against real GitHub."** Seven commits on top of `3cf3b01` deliver the two missing CLI helpers (`vibe registry redirect`, `vibe registry redirect-sync`), four hermetic redirect e2e tests + four git-source corner-case e2e tests, two bug fixes that surfaced during the production walks, and the operator-facing reference (commands docs + manual-tests recipes + CHANGELOG / ROADMAP flips).

The session also closes M1.15's deferred production smoke walk against `olegchir/vibevm-m1-smoke-flow-internal`. Both M1.15 (git-source dependencies) and M1.16 (registry redirect via stub repo) are now feature-complete with hermetic tests + production smoke walks; they are flipped to `✅ SHIPPED (2026-05-10)` in `ROADMAP.md`.

**Commits this session (newest-first; on top of `3cf3b01`):**

```
9b22adb docs(commands,registry-redirect,manual-tests,changelog,roadmap): M1.15 + M1.16 ship reference
af1f320 test(vibe-cli): hermetic e2e for git-source repeats + redirect resolves
e10dda6 feat(vibe-cli): vibe registry redirect + redirect-sync commands
36a5847 feat(vibe-publish): publish helpers for stub creation + tag mirroring
dd87674 feat(vibe-registry,vibe-resolver): redirect-aware fetch_manifest
a1dc2b3 fix(vibe-registry): archive→clone fall-back in fetch_manifest_at_ref
5b9a2dc fix(vibe-cli/uninstall): drop git-source declarations on uninstall
```

**Production smoke walk artefacts left on GitHub** (safe to delete, recreatable from `manual-tests/`):

- `https://github.com/olegchir/vibevm-m1-smoke-flow-internal` — M1.15 git-source smoke target.
- `https://github.com/olegchir/vibevm-m1-smoke-feat-helper` — M1.16 redirect smoke target.
- `https://github.com/vibespecs/feat-helper` — M1.16 redirect stub.

**Workspace state** at HEAD (after the seven commits above):

- vibe-cli e2e: **97 hermetic + 3 ignored** (was 89; +8 — 4 git-source + 4 redirect).
- vibe-cli bin: **103 hermetic** (was 93; +10 redirect/redirect-sync helpers).
- vibe-registry: **102 hermetic** (unchanged — the resolver-uplifted hermetic redirect tests landed in `058ff41`).
- vibe-core: **139 hermetic** (unchanged — `vibe-redirect.toml` parser tests landed in `b37e1b3`).
- `cargo test --workspace` all green; `cargo clippy --workspace --all-targets -- -D warnings` clean; `vibe check --path . --quiet` reports 0/0/0.

Working tree is clean apart from the WAL / CONTINUE update + commit. **No active blockers.** The whole M1.16 deferred-list is empty.

---

## Where we are right now

- **Branch:** `main`. Working tree clean (after this checkpoint commits).
- **Origin:** `origin/main` is at `3cf3b01` until the seven commits above (plus this WAL / CONTINUE update) push.
- **Last 10 commits (newest-first):**

  ```
  9b22adb docs(commands,registry-redirect,manual-tests,changelog,roadmap): M1.15 + M1.16 ship reference
  af1f320 test(vibe-cli): hermetic e2e for git-source repeats + redirect resolves
  e10dda6 feat(vibe-cli): vibe registry redirect + redirect-sync commands
  36a5847 feat(vibe-publish): publish helpers for stub creation + tag mirroring
  dd87674 feat(vibe-registry,vibe-resolver): redirect-aware fetch_manifest
  a1dc2b3 fix(vibe-registry): archive→clone fall-back in fetch_manifest_at_ref
  5b9a2dc fix(vibe-cli/uninstall): drop git-source declarations on uninstall
  3cf3b01 docs(continue): late-session checkpoint at 2026-05-10 (M1.15 + M1.16)
  058ff41 docs(wal): M1.16 implementation checkpoint
  c4b3f72 docs(registry-redirect,readme): operator reference for vibe-redirect.toml stubs
  ```

- **Active blocker:** none.

---

## What to do first in the next session

The M1.15 + M1.16 deferred-lists are now empty. Pick whichever matches the owner's interest:

### Option 1 — M1.5 (LLM generation)

The next major milestone per ROADMAP. **Non-routine — needs explicit owner sign-off before starting.** M1.5 is what makes vibevm "produce software" rather than "manage specs." Scope, design constraints, and entry points need to be discussed before any code lands. Three of the last sessions have closed off package-management work (M1.12 / M1.13 / M1.14 / M1.15 / M1.16); this is the natural pickup.

### Option 2 — Tag v0.1.0

CHANGELOG `[Unreleased]` carries M1.12 / M1.13 / M1.14 / M1.15 / M1.16. ROADMAP marks all five as SHIPPED. The full package-management surface (manifest schema, version constraints, registry auth, git-source declarations, registry redirects, publish flow, lockfile + cache invariants) is now feature-complete. If the owner is ready to cut the first tagged release, the steps are: lift `[Unreleased]` to `[0.1.0]` in CHANGELOG with the date, finalise `Cargo.toml` versions across crates, `git tag v0.1.0`, push tag, optionally publish to a binary release channel.

### Option 3 — Re-run the production walks

Useful as a smoke test whenever the auth pipeline or the git-source / redirect resolution path changes. Recipes:

- `manual-tests/M1.14-…` — original M1.14.4 private-probe walk against `olegchir/vibevm-private-probe` (still up).
- `manual-tests/M1.15-git-source-smoke.md` — git-source against `olegchir/vibevm-m1-smoke-flow-internal`.
- `manual-tests/M1.16-redirect-smoke.md` — redirect through `vibespecs/feat-helper` to `olegchir/vibevm-m1-smoke-feat-helper`.

### Option 4 — A fresh full-project audit

After three intense days (2026-05-08 → 2026-05-10) shipping M1.12 → M1.16, a "step back and read what we've got" pass would be high-value. Does `docs/` cover every surface? Are there orphan TODOs? Is the spec tree internally consistent? Possible lightweight scope: walk every `docs/commands/*.md`, every `spec/modules/**/*.md`, run `vibe check`, look for FIXMEs. Half-day exercise; sets up M1.5 / v0.1.0 with a clean baseline.

### Option 5 — Editing-an-existing-stub command

The one M1.16 affordance not delivered: `vibe registry redirect <pkgref> --to <url>` only creates fresh stubs (refuses if the slot is occupied). Updating an existing stub's marker (e.g. to change `target_url` after the external author migrated their hosting) is a manual `git clone` / edit / push procedure for v0. A separate `vibe registry redirect-update <pkgref> --to <new-url>` command would close this — small slice, ~3-5 commits.

### Option 6 — Pinned-policy bridging in production smoke walks

The hermetic `install_via_redirect_pinned_policy_uses_pinned_ref` test works against a stub whose tag set equals `{pinned_ref}` because the install-pipeline's pinned re-resolve requires the stub to surface the resolved version. The pure "stub_tag != pinned_ref" case (operator wants every consumer to resolve to v1.0.0 of target regardless of stub's v9.9.9 tag) works at the resolver level (FakeBackend test `resolve_redirect_pinned_uses_pinned_ref`) but not through the install pipeline. Bridging this would need the install pipeline to remember the redirect-discovery rather than re-resolve through `=<version>` — a small but invasive refactor of the depsolver→install handoff.

---

## Non-obvious findings from this session

These cost time / hit edge cases — write them down so a future session does not re-derive.

### GitHub disables `upload-archive` server-side; clone is the fall-back

`git archive --remote=https://github.com/<user>/<repo> <tag> -- vibe-package.toml` returns:

```
remote: fatal: 'archive' is not enabled in this repository
remote: error: upload-archive: archiver died with error
```

GitHub disables `upload-archive` by policy. The vibevm code path that reads a single file from a remote without cloning (`backend.fetch_file_at_ref` → `git archive --remote`) cannot do its job against GitHub.

Two places needed an archive→clone fall-back:

1. `GitPackageRegistry::fetch_dep_manifest` — already had the fall-back (M1.1 era). On `ArchiveUnsupported` it `refresh_package`s the per-package clone and reads `vibe-package.toml` from the working tree.
2. `GitPackageRegistry::fetch_manifest_at_ref` — added in this push. Same shape; the M1.15 git-source path and the M1.16 redirect-follow path both go through this method.
3. `try_fetch_redirect_for_url` — added in this push. The marker-probe path (one-off `vibe-redirect.toml` read) also needed clone fall-back so redirect detection works on GitHub-hosted stubs.

Without the fall-backs, M1.15 / M1.16 were `file://`-only. Production walks failed at resolution time with "remote does not support `git archive`".

### Hop-limit check must fire before manifest fetch

`MultiRegistryResolver::follow_redirect` originally read the target's `vibe-package.toml` first, then probed for `vibe-redirect.toml` to detect chains. If the target is itself a stub (the chain case), the manifest fetch returns `FileNotFoundInRef` because stub-only repos have no `vibe-package.toml`. The chain detection never got to run; the operator saw a misleading "file not found" instead of "redirect chain not allowed".

Fixed by swapping the order: probe marker first; if found at hop 2, raise `RedirectChainNotAllowed` immediately. If absent, proceed with the manifest fetch as before.

### DepProvider::fetch_manifest needed redirect awareness

Pre-this-push, `MultiRegistryProvider::fetch_manifest` walked `MultiRegistryResolver::registries()` directly and called each `reg.fetch_dep_manifest`. That works only for direct registry-served packages. Three failure modes:

- **Stub-only repos** (M1.16): no `vibe-package.toml` to fetch.
- **Pinned-policy redirects**: `version` is the target's version, not the stub's tag. Stub doesn't have a `=version` tag.
- **Git-source declarations** (M1.15): registry walk doesn't apply at all.

The new `MultiRegistryResolver::fetch_manifest(kind, name, version)` is the single entry point. It re-runs `resolve()` with `=<version>` to converge on the same `MultiResolution` the install pipeline already saw, then reads the manifest from whichever URL the resolution recorded — target_url for redirects, dep.url for git-source, registry's URL otherwise. Same shape as `fetch_with_expected_hash` (which has been redirect-aware since M1.16 +1); the depsolver-side now agrees with the install-side on every shape.

### Pinned-policy fall-back: re-resolve constraint-free, verify version match

For pinned-policy redirects, the depsolver pins `version = 1.0.0` (target's version) but the stub's tag list doesn't contain `v1.0.0`. The pinned re-resolve `resolve(=1.0.0)` returns `NoMatchingVersion`.

Fix: when `resolve(=version)` fails with `NoMatchingVersion` / `PackageNotFoundEverywhere` / `UnknownPackage`, retry with a constraint-free `latest` resolve. If the result version matches what the depsolver pinned, use it. Otherwise propagate the original error.

This handles the pinned case correctly. There is one shape it doesn't handle: when the stub's tag list contains the resolved version but ALSO contains a higher tag that pinned policy would override. The hermetic test deliberately keeps the stub at a single tag = `pinned_ref` to side-step this; the FakeBackend resolver test (`resolve_redirect_pinned_uses_pinned_ref`) covers the pure decoupled-tag case.

### Uninstall was a one-list-only walk

`drop_from_manifest_requires` (in `vibe-cli/src/commands/uninstall.rs`) removed the entry from `requires.packages` but not `requires.git_packages`. M1.15 added `git_packages` as a parallel list; the uninstall code never updated. The hermetic e2e `uninstall_removes_git_source_from_manifest_and_lockfile` surfaced this on the first run.

Fix: retain-not on both lists; return true iff anything was removed from either. Pkgrefs match on `(kind, name)` only — version constraint / git ref policy don't enter into uninstall identity.

### `tempfile` moved from dev-dep to regular dep in `vibe-cli`

`vibe registry redirect` builds the stub source dir in a `TempDir` at runtime, not just in tests. `tempfile` had to move from `dev-dependencies` to `dependencies` in `crates/vibe-cli/Cargo.toml`. Other commands already used `tempfile::tempdir()` only via the resolver / publish crates' internals; the redirect command is the first CLI command to need it directly.

### Token-discipline invariant survives both new paths

The 2026-05-10 production walks confirmed that `~/.vibe/registries/` carries no token bytes after a successful M1.15 git-source install (`olegchir/vibevm-m1-smoke-flow-internal`) AND after a successful M1.16 redirect install (`olegchir/vibevm-m1-smoke-feat-helper` via `vibespecs/feat-helper`). `grep -r x-access-token ~/.vibe/registries/` came up empty in both cases. The newly-introduced clone fall-backs reuse `set_remote_url(.., "origin", plain_url)` post-bootstrap, same as M1.14's bootstrap-with-scrub. The `redirect-target-{kind}-{name}` synthetic registry built by the redirect path inherits the M1.14 token-injection plumbing through `GitPackageRegistry::open_single_package`'s `auth` / `token_env` parameters.

---

## Repository map

```
vibevm/
├── CLAUDE.md / AGENTS.md / GEMINI.md   # Three identical copies of the four rules.
├── CONTINUE.md                          # This file. Cold-resume snapshot.
├── ROADMAP.md                           # Milestone plan; M1.15+M1.16 SHIPPED on this push.
├── CHANGELOG.md                         # Milestone chronicle; [Unreleased] holds M1.12/M1.13/M1.14/M1.15/M1.16.
├── VIBEVM-SPEC.md                       # Owner-frozen spec; do not edit without explicit instruction.
├── DEV-GUIDE.md / RUNTIME-GUIDE.md      # Per-machine setup docs.
├── crates/
│   ├── vibe-cli/                        # `vibe` binary entry point. clap dispatch + per-subcommand modules.
│   │   └── src/commands/
│   │       ├── install.rs               # Resolve+plan+apply pipeline. M1.12 [requires] writes,
│   │       │                            # M1.13 caret-default + --exact, M1.14 --auth-required,
│   │       │                            # M1.15 --git/--tag/--branch/--rev.
│   │       ├── update.rs                # Re-resolve+diff+apply. M1.14.3 --exact + --auth-required reach.
│   │       ├── uninstall.rs             # Symmetric to install. M1.15 fix: drops both packages and
│   │       │                            # git_packages from manifest.
│   │       ├── outdated.rs              # Read-only upstream-newer probe. M1.14.3 --auth-required reach.
│   │       ├── mcp.rs                   # Five-agent matrix; install/upgrade/uninstall/status/serve.
│   │       ├── registry.rs              # add / remove / set-mirror / sync / vendor / publish / test /
│   │       │                            # redirect / redirect-sync (M1.16).
│   │       ├── search.rs                # PROP-005 index-aware discovery; auth-naive (read-only).
│   │       └── skill_template.md        # Vendored two-state SKILL.md (Section A bootstrap + B + Common).
│   ├── vibe-core/                       # Manifests, lockfile schema v3, AuthKind, version_spec.
│   │   └── src/manifest/
│   │       ├── project.rs               # ProjectManifest + RegistrySection.auth/token_env (M1.14.1).
│   │       ├── package.rs               # PackageManifest (vibe-package.toml) + Requires/Provides.
│   │       ├── lockfile.rs              # vibe.lock schema v3 with full provenance.
│   │       ├── redirect.rs              # vibe-redirect.toml schema (M1.16).
│   │       └── mod.rs                   # write_toml() — comment-preserving via toml_edit (M1.14.2).
│   ├── vibe-graph/                      # In-memory dep graph helpers.
│   ├── vibe-registry/                   # The big crate — git_backend, GitPackageRegistry,
│   │   │                                # MultiRegistryResolver. Auth + redirect + git-source live here.
│   │   └── src/
│   │       ├── git_backend/
│   │       │   ├── mod.rs               # GitBackend trait + set_remote_url (M1.14).
│   │       │   └── shell.rs             # ShellGit + apply_common_env (TTY-aware silencing).
│   │       ├── git_package_registry.rs  # Per-registry instance with auth + effective_token +
│   │       │                            # token_env_name; bootstrap-with-scrub flow. M1.16 fix:
│   │       │                            # fetch_manifest_at_ref archive→clone fall-back.
│   │       ├── multi_registry_resolver.rs # Walk + per-auth walk-vs-halt + strict_auth gate +
│   │       │                            # redirect-follow (M1.16) + redirect-aware fetch_manifest.
│   │       └── lib.rs                   # RegistryError including MissingToken +
│   │                                    # PackageNotFoundEverywhere variants.
│   ├── vibe-resolver/                   # Feature expansion + activation evaluation (PROP-003).
│   ├── vibe-install/                    # Install pipeline: plan_install → apply → register.
│   ├── vibe-llm/                        # LLM provider abstraction. Skeleton — real impls land in M1.5.
│   ├── vibe-mcp/                        # JSON-RPC MCP server. 3 tools today.
│   ├── vibe-check/                      # Spec-consistency linter.
│   ├── vibe-publish/                    # GitHubCreator / GitVerseCreator / DirectGitCreator publishers.
│   │   └── src/                         # New M1.16 helpers in git_publish.rs: push_initial,
│   │                                    # ls_remote_tags, push_tag_only, shallow_clone.
│   └── vibe-wire/                       # JTD-codegen'd wire types.
├── services/
│   └── vibe-index/                      # Standalone PROP-005 utility: per-org package index. Own workspace.
├── spec/
│   ├── boot/{00-core,90-user}.md        # Read at every session start.
│   ├── WAL.md                           # Living checkpoint of project state. Authoritative if it
│   │                                    # diverges from this file.
│   ├── common/PROP-000…PROP-006         # Foundation policy + operating modes.
│   ├── modules/                         # Per-crate PROPs.
│   │   └── vibe-registry/PROP-002       # §2.2.1 (auth axis), §2.3.1 (failure classifier),
│   │                                    # §2.4.1 (git-source), §2.4.2 (redirect).
│   └── research/PROP-004                # Tessl comparative research.
├── docs/
│   ├── README.md                        # User-doc index.
│   ├── commands/                        # Per-subcommand reference. M1.16: registry-redirect.md +
│   │                                    # registry-redirect-sync.md.
│   ├── git-source-dependencies.md       # M1.15 operator reference.
│   ├── registry-auth.md                 # M1.14 operator reference.
│   ├── registry-redirect.md             # M1.16 operator reference. CLI workflow + manual fallback.
│   ├── version-syntax.md                # M1.13 operator reference.
│   ├── architecture.md / lockfile-format.md / glossary.md / troubleshooting.md
│   └── guides/
├── manual-tests/                        # Runnable smoke protocols. M1.15 + M1.16 added in this push.
├── fixtures/registry/                   # Hermetic per-package registry fixtures.
├── tools/                               # self-check.sh + jtd-codegen install README.
└── xtask/                               # `cargo xtask codegen` / `check-codegen`.
```

---

## Architectural / policy decisions still in force

In rough order of how often they bite a fresh contributor:

1. **Four non-negotiable rules** ([PROP-000 §12](spec/common/PROP-000.md#commits)):
   1. **No AI / machine-author attribution** anywhere.
   2. **Conventional Commits.** Subject ≤ 60 chars (hard limit 72), body explains WHY.
   3. **Group commits by meaning**, never by file or by time.
   4. **Autonomy on routine changes.** Non-routine red lines (history rewrite, `--force` push, large blobs, CI / signing / secrets, irreversible ops) STILL require explicit owner sign-off.

2. **Memory discipline.** Project facts live in the repo. Per-machine facts only live in tool-specific user-memory.

3. **Vocabulary lock.** Only `flow`, `feat`, `stack`, `tool`. Never `lifecycle` / `phase` / `goal` / `plugin`.

4. **Language: Rust.** Permissive licenses only. `dependency weight is not a decision factor` per PROP-000 §15.

5. **Manifest format: TOML** for human-edited; **JTD+codegen** for wire contracts.

6. **Identity: `(kind, name, version, content_hash)`.** URL is informational.

7. **Token secrecy** (PROP-000 §20). Never printed in any vibevm-produced output. Modern git (≥ 2.31) auto-redacts; vibevm relies on that as the second line of defence.

8. **Repository hosts.** vibevm source = GitVerse. Package registry = GitHub `vibespecs` (primary) + GitVerse `vibespecs` (secondary).

9. **User-owned files** (vibevm install/uninstall NEVER touches): `spec/boot/00-core.md`, `spec/boot/90-user.md`, `spec/WAL.md`, `VIBEVM-SPEC.md`, `refs/book/**`.

10. **PROP-006 codewords.** `«move fast and break things»` is the first; never overrides the four rules.

11. **Cargo-shape version syntax** (M1.13). Bare semver `0.3.0` = caret `^0.3.0`. Use `=0.3.0` for strict equal.

12. **`[requires]` is the source of truth for declared deps** (M1.12). `vibe.toml` carries the human's input list (constraints); `vibe.lock` carries the resolved materialisation (exact pins + content hashes). `meta.root_dependencies` in the lockfile is a mirror, not authoritative.

13. **Per-registry `auth` axis** (M1.14, PROP-002 §2.2.1). Four regimes: `none` (default, public read-only) / `token-env` (PAT from env-var) / `credential-helper` (system git helpers, opt-in) / `ssh` (delegated to ssh-agent).

14. **Auth-aware 401 classifier** (PROP-002 §2.3.1). 401 on `auth = "none"` walks past as `UnknownPackage`; 401 on authenticated registries halts. `--auth-required` flips public-401 to halt for strict CI gating.

15. **Token never on disk via vibevm-controlled paths** (M1.14, M1.16). Tokens loaded once at registry-open from env-var, held in memory only, scrubbed from `.git/config` immediately after `bootstrap` via `set_remote_url(.., "origin", plain_url)`. Same discipline applies to the redirect-target single-package registry synthesised by `fetch_via_redirect`.

16. **TTY-aware credential silencing** (M1.14). `apply_common_env` silences GCM / `credential.helper` / `core.askPass` in non-TTY / `--unattended` runs. Interactive TTY without `--unattended` leaves them alone.

17. **`--unattended` global flag** + `VIBE_UNATTENDED` env-var (truthy: `1`, `true`, `yes`, `on`). Implies skip-confirm everywhere; refuses to open wizards in MCP install; stamps `unattended: true` on every JSON envelope.

18. **MCP command confirm-prompt is TTY-gated** (M1.14.3). Non-TTY callers see the pre-this-version zero-confirm behaviour preserved.

19. **comment-preserving `vibe.toml` writes** (M1.14.2). Three layers via `toml_edit`: header comments, per-table prefix, document trailing.

20. **`[requires.packages]` table-form schema** (M1.15, PROP-002 §2.4.1). Map values are either a version-constraint string (registry-resolved) or an inline table (registry-resolved with options, OR git-source with `git = "..."` + exactly one of `tag`/`branch`/`rev`). Legacy array-of-strings shape parses transparently; round-trip writes table form.

21. **Resolution priority: override > git-source > registry-walk** (M1.15). Same shape as Cargo's `[patch] foo` overriding `[dependencies] foo = { git = "..." }` overriding `[dependencies] foo = "*"`.

22. **`source_kind` discriminant in lockfile** (M1.15). New enum `Registry` / `Git` / `Override` per `[[package]]`. Wire-compatible — `Option<SourceKind>` defaults to `None` for pre-M1.15 lockfiles.

23. **Registry redirect via stub repo** (M1.16, PROP-002 §2.4.2). A registry org's `<kind>-<name>` slot may carry `vibe-redirect.toml` instead of `vibe-package.toml`; the resolver follows the marker to an external `target_url` transparently. Hop limit = 1; identity check on target's `[package]` against requested pkgref. Lockfile records `via_redirect = <stub_url>` alongside `source_url = <target_url>`.

24. **Two-layer auth for redirect** (M1.16). Stub auth = registry's `[[registry]] auth`; target auth = `[redirect].auth` (independent). Stub may be public, target may be private; tokens flow through M1.14 plumbing on both sides.

25. **GitHub `upload-archive` refusal → clone fall-back** (M1.16 fix). `git archive --remote=https://github.com/...` is server-side disabled by GitHub policy. `fetch_manifest_at_ref` and `try_fetch_redirect_for_url` both fall back to `refresh_package` (shallow clone) on `ArchiveUnsupported`. The fall-back is also the install path's pre-warm for the imminent fetch.

---

## Recent commit chain (last 25, newest first)

```
9b22adb docs(commands,registry-redirect,manual-tests,changelog,roadmap): M1.15 + M1.16 ship reference
af1f320 test(vibe-cli): hermetic e2e for git-source repeats + redirect resolves
e10dda6 feat(vibe-cli): vibe registry redirect + redirect-sync commands
36a5847 feat(vibe-publish): publish helpers for stub creation + tag mirroring
dd87674 feat(vibe-registry,vibe-resolver): redirect-aware fetch_manifest
a1dc2b3 fix(vibe-registry): archive→clone fall-back in fetch_manifest_at_ref
5b9a2dc fix(vibe-cli/uninstall): drop git-source declarations on uninstall
3cf3b01 docs(continue): late-session checkpoint at 2026-05-10 (M1.15 + M1.16)
058ff41 docs(wal): M1.16 implementation checkpoint
c4b3f72 docs(registry-redirect,readme): operator reference for vibe-redirect.toml stubs
6e861ac feat(vibe-registry): MultiRegistryResolver follows vibe-redirect.toml stubs
b37e1b3 feat(vibe-core,vibe-registry,vibe-install): vibe-redirect.toml parser + via_redirect lockfile field
f9ce420 test(vibe-cli): e2e coverage for vibe install --git --tag and --branch
540f6c0 docs(continue): mid-session checkpoint at 2026-05-10 (M1.15 implementation)
5c3751c docs(wal): M1.15 implementation checkpoint
0411f2b docs(commands,git-source,readme): user-facing reference for git-source declarations
90bf10b feat(vibe-cli): vibe install --git/--tag/--branch/--rev for git-source declarations
a7dce7f feat(vibe-core,vibe-registry,vibe-install): lockfile source_kind field for git/override discriminant
153f3a2 feat(vibe-cli): wire git-source declarations through install/update/outdated
161b7b1 feat(vibe-registry): MultiRegistryResolver dispatches to git-source declarations
c313ebd feat(vibe-registry): GitPackageRegistry::open_single_package for git-source
2544d76 feat(vibe-core): [requires.packages] table-form schema with git-source slot
ec77471 docs(wal): session-end checkpoint — pointer to CONTINUE.md
d4ee973 docs(continue): cold-resume checkpoint at 2026-05-08 session-end
8ab5c9c docs(roadmap,changelog): catch up on M1.12 / M1.13 / M1.14 milestones
```

---

## Quick-start commands

```powershell
# Build everything.
cargo build --workspace

# Full test gate (matches CI).
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p vibe-cli -- check --path . --quiet

# Or one-shot via the bundled script.
bash tools/self-check.sh

# Install vibe into ~/.cargo/bin/.
cargo install --path crates/vibe-cli --locked

# Three signature recipes from this push:

# Add a git-source dep without hand-editing vibe.toml (M1.15):
vibe install flow:internal-helper \
  --git https://github.com/me/flow-internal-helper \
  --tag v0.1.0

# Create a registry-redirect stub (M1.16):
vibe registry redirect flow:internal-helper \
  --to https://github.com/external-author/flow-internal-helper \
  --description "Delegated to external-author"

# Mirror target tags into the stub (M1.16):
vibe registry redirect-sync flow:internal-helper
```

---

## Pointer

`spec/WAL.md` is the canonical **living** checkpoint. If anything in this `CONTINUE.md` disagrees with the top of `spec/WAL.md`, trust the WAL — it gets bumped every session.
