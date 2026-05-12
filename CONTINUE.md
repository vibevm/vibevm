# CONTINUE — cold-resume checkpoint

_Written: 2026-05-12 session-end (M1.15 + M1.16 ship-complete + test-fixture re-homing). Owner-readable, self-contained. Pick this up with zero prior context._

---

## TL;DR (executive summary)

**This session closes M1.15 + M1.16 end to end AND cleans up the test-fixture footprint so the canonical `vibespecs` org stays populated with real installable packages only.** Three distinct slices landed:

1. **M1.16 finalisation (2026-05-10).** Seven commits closed the M1.16 deferred-list: `vibe registry redirect` + `vibe registry redirect-sync` CLI commands, four hermetic redirect e2e tests, four git-source corner-case e2e tests, plus two bug fixes (uninstall git-source cleanup; `fetch_manifest_at_ref` archive→clone fall-back on GitHub) and a redirect-aware `DepProvider::fetch_manifest`. M1.15 also gained its deferred production smoke walk.
2. **Test-fixture re-homing (2026-05-12).** Five GitHub repos + one GitVerse repo migrated out of canonical `vibespecs` + `olegchir` personal namespace into three dedicated test orgs (`vibespecstest1/2/3`). `cli_live_e2e.rs` rewritten to provision custom `[[registry]]` blocks pointing at the test orgs; both manual-test recipes (M1.15 + M1.16) reprovisioned. The five old smoke artefacts deleted via GitHub API.
3. **Documentation catch-up.** CHANGELOG / ROADMAP / WAL / CONTINUE / `docs/registry-redirect.md` / `docs/commands/registry-redirect{,-sync}.md` / two new `manual-tests/M1.{15,16}-*-smoke.md` recipes. ROADMAP flips M1.15 + M1.16 to `✅ SHIPPED (2026-05-10)`.

**Total this session: 10 commits, all on `main`, all pushed to `origin/main`.** Workspace clean, `cargo test --workspace` zelf, clippy `-D warnings` clean, `vibe check --path . --quiet` 0/0/0. **No active blockers.**

### Test-org map (live)

- `https://github.com/vibespecstest1` — GitHub registry-side fixtures:
  - `flow-vibevm-github-smoke` (live-e2e GitHub leg)
  - `feat-helper` (M1.16 redirect stub, marker points at `vibespecstest2/vibevm-m1-smoke-feat-helper`)
- `https://github.com/vibespecstest2` — GitHub external-author / target fixtures:
  - `vibevm-m1-smoke-flow-internal` (M1.15 git-source target)
  - `vibevm-m1-smoke-feat-helper` (M1.16 redirect target)
  - `vibevm-private-probe` (M1.14.4 private-probe target, kept private)
- `https://gitverse.ru/vibespecstest3` — GitVerse fixtures:
  - `vibevm-direct-push-smoke` (live-e2e GitVerse leg, reached over SSH)

### Canonical org state (after cleanup)

- `https://github.com/vibespecs` carries only real packages: `flow-wal`, `flow-sync-from-code`, `flow-atomic-commits`.
- `https://gitverse.ru/vibespecs` still hosts `vibevm-direct-push-smoke` — **owner needs to delete via web UI** (vibevm has no DELETE affordance for GitVerse; no API endpoint).

---

## Where we are right now

- **Branch:** `main`. Working tree clean (only `.claude/settings.local.json` untracked).
- **Origin:** `origin/main` matches local `4e852f0`.
- **Active blocker:** none. Owner has one pending manual step on GitVerse (see "Canonical org state" above).

### This session's commits (newest-first)

```
4e852f0 docs(registry-redirect,changelog,wal,continue): note test-org re-home
dbba8d7 test(cli,manual-tests): move live + smoke fixtures to test orgs
ad9b8b3 docs(continue,wal): M1.15 + M1.16 ship-complete checkpoint
9b22adb docs(commands,registry-redirect,manual-tests,changelog,roadmap): M1.15 + M1.16 ship reference
af1f320 test(vibe-cli): hermetic e2e for git-source repeats + redirect resolves
e10dda6 feat(vibe-cli): vibe registry redirect + redirect-sync commands
36a5847 feat(vibe-publish): publish helpers for stub creation + tag mirroring
dd87674 feat(vibe-registry,vibe-resolver): redirect-aware fetch_manifest
a1dc2b3 fix(vibe-registry): archive→clone fall-back in fetch_manifest_at_ref
5b9a2dc fix(vibe-cli/uninstall): drop git-source declarations on uninstall
```

### Workspace state

- vibe-cli e2e: **97 hermetic + 3 ignored** (+8 from previous session: 4 git-source corners + 4 redirect cases).
- vibe-cli bin: **103 hermetic** (+10 redirect/redirect-sync helper unit tests).
- vibe-registry: **102 hermetic** (unchanged in this session).
- vibe-core: **139 hermetic** (unchanged).
- Live e2e (`cli_live_e2e`, `--ignored` by default): **3 passing** against `vibespecstest1` (GitHub) + `vibespecstest3` (GitVerse SSH).

---

## What to do first in the next session

The M1.15 + M1.16 deferred-lists are empty. The test-fixture migration also closed cleanly. Pick whichever matches the owner's interest:

### Option 1 — M1.5 (LLM generation)

**Non-routine** — needs explicit owner sign-off. M1.5 is what makes vibevm "produce software" rather than "manage specs." Scope, design constraints, and entry points must be discussed before any code lands. Three of the last sessions converged here; this is the natural pickup.

### Option 2 — Tag v0.1.0

CHANGELOG `[Unreleased]` carries M1.12 / M1.13 / M1.14 / M1.15 / M1.16 plus the 2026-05-12 test-fixture re-home note. ROADMAP marks all five milestones `✅ SHIPPED`. Package-management surface is feature-complete; if the owner is ready to cut the first tagged release, the steps are: rename `[Unreleased]` to `[0.1.0]` with today's date, finalise per-crate `Cargo.toml` versions, `git tag v0.1.0`, push tag, optionally publish a binary release.

### Option 3 — Re-run the production walks

Useful as a smoke test whenever auth / git-source / redirect resolution paths change. Recipes:

- `manual-tests/M1.15-git-source-smoke.md` — against `vibespecstest2/vibevm-m1-smoke-flow-internal`.
- `manual-tests/M1.16-redirect-smoke.md` — through `vibespecstest1/feat-helper` to `vibespecstest2/vibevm-m1-smoke-feat-helper`.
- (M1.14.4 private-probe walk should be ported to `vibespecstest2/vibevm-private-probe` — recipe file does not exist yet; would be ~150 lines.)

### Option 4 — Editing-an-existing-stub command

`vibe registry redirect` only creates fresh stubs. Updating an existing stub's marker (e.g. to change `target_url` after the external author migrated their hosting) is a manual `git clone` / edit / push procedure for v0. A separate `vibe registry redirect-update <pkgref> --to <new-url>` command would close this — small slice, ~3-5 commits.

### Option 5 — Owner-side GitVerse cleanup follow-through

The one outstanding manual step: delete `https://gitverse.ru/vibespecs/vibevm-direct-push-smoke` via the GitVerse web UI. After that, `gitverse.ru/vibespecs` is empty (no real packages live there yet; canonical packages are on GitHub). If the owner wants the GitVerse vibespecs org to mirror real packages, that is a separate workflow.

### Option 6 — Full-project audit before M1.5

Three intensive days (2026-05-08 → 2026-05-12) shipped M1.12 → M1.16. A "step back and read what we've got" pass would be high-value before plunging into M1.5. Scope: walk every `docs/commands/*.md`, every `spec/modules/**/*.md`, run `vibe check`, look for orphan TODOs. Half-day exercise; cleans the baseline.

---

## Non-obvious findings from this session

These cost time / hit edge cases — write them down so a future session does not re-derive.

### GitHub disables `upload-archive` server-side; clone is the fall-back

`git archive --remote=https://github.com/<user>/<repo> <ref> -- <file>` returns:

```
remote: fatal: 'archive' is not enabled in this repository
remote: error: upload-archive: archiver died with error
```

GitHub disables `upload-archive` by policy. The vibevm code path that reads a single file from a remote without cloning (`backend.fetch_file_at_ref` → `git archive --remote`) cannot do its job against GitHub. Three places needed an archive→clone fall-back in this session:

1. `GitPackageRegistry::fetch_dep_manifest` — already had the fall-back (M1.1 era).
2. `GitPackageRegistry::fetch_manifest_at_ref` — added in this push (commit `a1dc2b3`). The M1.15 git-source path and M1.16 redirect-follow path both go through this method.
3. `try_fetch_redirect_for_url` — added in this push. The marker-probe path also needed clone fall-back so redirect detection works on GitHub-hosted stubs.

Without the fall-backs, M1.15 / M1.16 were `file://`-only.

### Hop-limit check must fire before manifest fetch

`MultiRegistryResolver::follow_redirect` originally read the target's `vibe-package.toml` first, then probed for `vibe-redirect.toml` to detect chains. If the target is itself a stub, the manifest fetch returned `FileNotFoundInRef` and the chain detection never got to run. Fixed by swapping the order: probe marker first; if found at hop 2, raise `RedirectChainNotAllowed` immediately.

### DepProvider::fetch_manifest needed redirect awareness

Pre-this-session, `MultiRegistryProvider::fetch_manifest` walked `MultiRegistryResolver::registries()` directly. Worked only for direct registry-served packages, not for stub-only repos (M1.16), pinned-policy redirects, or git-source declarations (M1.15). The new `MultiRegistryResolver::fetch_manifest(kind, name, version)` re-runs `resolve()` with `=<version>` to converge on the same `MultiResolution` and reads from whichever URL the resolution recorded.

### Pinned-policy fall-back: re-resolve constraint-free, verify version match

For pinned-policy redirects, the depsolver pins `version = 1.0.0` (target's version) but the stub's tag list may not contain `v1.0.0`. The pinned re-resolve `resolve(=1.0.0)` returns `NoMatchingVersion`. Fix: on that error, retry with a constraint-free `latest` resolve and verify the result version matches what the depsolver pinned.

### Uninstall was a one-list-only walk (M1.15 bug)

`drop_from_manifest_requires` removed from `requires.packages` but not `requires.git_packages`. After M1.15 added `git_packages`, the uninstall code never got updated. Surfaced by the e2e `uninstall_removes_git_source_from_manifest_and_lockfile`. Fixed in commit `5b9a2dc`.

### GitVerse HTTPS demands credentials for fresh orgs

The canonical `gitverse.ru/vibespecs` org happens to be publicly readable over HTTPS. A freshly-created GitVerse org isn't — HTTPS reads against `gitverse.ru/vibespecstest3/<repo>.git` get a `could not read Username` prompt and fail. SSH (`git@gitverse.ru:vibespecstest3/<repo>.git`) works because the local SSH key is registered. The `cli_live_e2e` test was updated to use SSH form for the GitVerse registry URL. Matches the operator path documented in `spec/boot/90-user.md`.

### `tempfile` moved from dev-dep to regular dep in `vibe-cli`

`vibe registry redirect` builds the stub source dir in a `TempDir` at runtime, not just in tests. Had to move `tempfile` from `dev-dependencies` to regular `dependencies` in `crates/vibe-cli/Cargo.toml`.

### Test-fixture re-homing recipe (clone-mirror + push-mirror works for any host)

For migrating live git repos to a new namespace without losing tags / branches:

```bash
git clone --mirror <old-url>  m-<name>.git
git --git-dir=m-<name>.git push --mirror "https://x-access-token:$TOKEN@host.com/<new-org>/<new-name>.git"
```

`--mirror` on clone copies every ref (heads, tags, notes); `--mirror` on push reproduces the same set on the destination. Token-discipline: embed only in the push URL once; never echo. Validated against four public GitHub repos plus one private (`olegchir/vibevm-private-probe` → `vibespecstest2/vibevm-private-probe`).

For GitVerse (no API repo-create), `vibe registry publish --repo-url <ssh-url>` direct-pushes a local fixture into an already-provisioned repo. Used to seed `vibespecstest3/vibevm-direct-push-smoke` from `fixtures/manual-test-packages/flow-vibevm-direct-push-smoke/`.

### Updating an existing stub's marker is a manual procedure

`vibe registry redirect` refuses to clobber an existing stub. To rewrite the marker (this session's `feat-helper` stub had to be repointed from `olegchir` to `vibespecstest2`):

```bash
git clone <stub-url> tmp && cd tmp
# edit vibe-redirect.toml
git add vibe-redirect.toml && git commit -m "stub: retarget"
git tag -d v0.1.0 && git tag -a v0.1.0 -m "v0.1.0"
git push --force <stub-url> main
git push <stub-url> :refs/tags/v0.1.0 && git push <stub-url> v0.1.0
```

This is one of the candidate v1 features ("Option 4 — Editing-an-existing-stub command").

---

## Repository map

```
vibevm/
├── CLAUDE.md / AGENTS.md / GEMINI.md   # Three identical copies of the four rules.
├── CONTINUE.md                          # This file. Cold-resume snapshot.
├── ROADMAP.md                           # Milestone plan; M1.15+M1.16 SHIPPED on this push.
├── CHANGELOG.md                         # [Unreleased] holds M1.12/M1.13/M1.14/M1.15/M1.16
│                                        # + 2026-05-12 test-fixture re-home note.
├── VIBEVM-SPEC.md                       # Owner-frozen spec.
├── DEV-GUIDE.md / RUNTIME-GUIDE.md      # Per-machine setup docs.
├── crates/
│   ├── vibe-cli/                        # `vibe` binary entry point.
│   │   └── src/commands/
│   │       ├── install.rs               # M1.12 [requires], M1.13 caret-default,
│   │       │                            # M1.14 --auth-required, M1.15 --git/--tag/...
│   │       ├── uninstall.rs             # M1.15 fix: drops from both packages
│   │       │                            # and git_packages.
│   │       ├── registry.rs              # add / publish / list / sync / ... +
│   │       │                            # redirect / redirect-sync (M1.16).
│   │       └── ...
│   ├── vibe-core/                       # Manifests + lockfile + redirect.toml schema.
│   │   └── src/manifest/redirect.rs     # vibe-redirect.toml parser (M1.16).
│   ├── vibe-registry/                   # GitPackageRegistry + MultiRegistryResolver.
│   │   └── src/multi_registry_resolver.rs # redirect-follow + redirect-aware
│   │                                    # fetch_manifest (M1.16).
│   ├── vibe-resolver/                   # Depsolver + DepProvider adapters.
│   │   └── src/multi_registry_provider.rs # Delegates fetch_manifest to resolver.
│   ├── vibe-publish/                    # RepoCreator + push helpers.
│   │   └── src/git_publish.rs           # push_initial / ls_remote_tags /
│   │                                    # push_tag_only / shallow_clone (M1.16).
│   └── ...
├── spec/
│   ├── boot/{00-core,90-user}.md
│   ├── WAL.md                           # Living checkpoint — authoritative.
│   ├── common/PROP-000…PROP-006
│   └── modules/vibe-registry/PROP-002   # §2.4.1 git-source, §2.4.2 redirect.
├── docs/
│   ├── commands/registry-redirect.md           # M1.16 CLI ref.
│   ├── commands/registry-redirect-sync.md      # M1.16 sync CLI ref.
│   ├── git-source-dependencies.md              # M1.15 operator ref.
│   ├── registry-redirect.md                    # M1.16 operator ref.
│   ├── registry-auth.md                        # M1.14 operator ref.
│   └── ...
├── manual-tests/                        # Runnable smoke protocols.
│   ├── M1.15-git-source-smoke.md        # → vibespecstest2 target (re-homed).
│   ├── M1.16-redirect-smoke.md          # → vibespecstest1 stub + vibespecstest2
│   │                                    # target (re-homed).
│   └── ...
├── fixtures/registry/                   # Hermetic per-package registry fixtures.
└── ...
```

---

## Architectural / policy decisions still in force

In rough order of how often they bite a fresh contributor:

1. **Four non-negotiable rules** ([PROP-000 §12](spec/common/PROP-000.md#commits)):
   1. **No AI / machine-author attribution** anywhere.
   2. **Conventional Commits.** Subject ≤ 60 chars (hard limit 72), body explains WHY.
   3. **Group commits by meaning**, never by file or by time.
   4. **Autonomy on routine changes.** Non-routine red lines still require explicit sign-off.

2. **Memory discipline.** Project facts live in the repo. Per-machine facts only in user-memory.

3. **Vocabulary lock.** Only `flow`, `feat`, `stack`, `tool`. Never `lifecycle` / `phase` / `goal` / `plugin`.

4. **Language: Rust.** Permissive licenses only.

5. **Manifest format: TOML** for human-edited; **JTD+codegen** for wire contracts.

6. **Identity: `(kind, name, version, content_hash)`.** URL is informational.

7. **Token secrecy** (PROP-000 §20). Never printed in any vibevm-produced output. Modern git auto-redacts; vibevm relies on that as the second line of defence.

8. **Repository hosts.** vibevm source = GitVerse. Package registry = GitHub `vibespecs` (primary) + GitVerse `vibespecs` (secondary).

9. **Test fixtures live in dedicated test orgs** (2026-05-12). Canonical `vibespecs` org reserves all slots for real installable packages. Smoke fixtures live in `vibespecstest1` (registry-side), `vibespecstest2` (external-target), `vibespecstest3` (GitVerse).

10. **User-owned files** (vibevm install/uninstall NEVER touches): `spec/boot/00-core.md`, `spec/boot/90-user.md`, `spec/WAL.md`, `VIBEVM-SPEC.md`, `refs/book/**`.

11. **PROP-006 codewords.** `«move fast and break things»` is the first; never overrides the four rules.

12. **Cargo-shape version syntax** (M1.13). Bare semver `0.3.0` = caret `^0.3.0`. Use `=0.3.0` for strict equal.

13. **`[requires]` is the source of truth for declared deps** (M1.12). `vibe.toml` carries the human's input; `vibe.lock` carries the resolved materialisation.

14. **Per-registry `auth` axis** (M1.14, PROP-002 §2.2.1). Four regimes: `none` / `token-env` / `credential-helper` / `ssh`.

15. **Auth-aware 401 classifier** (PROP-002 §2.3.1). 401 on `auth = "none"` walks past as `UnknownPackage`; 401 on authenticated registries halts. `--auth-required` flips public-401 to halt.

16. **Token never on disk via vibevm-controlled paths** (M1.14, M1.16). Tokens loaded once at registry-open from env-var, held in memory only, scrubbed from `.git/config` immediately after `bootstrap` via `set_remote_url(.., "origin", plain_url)`. Same discipline applies to redirect-target registries.

17. **TTY-aware credential silencing** (M1.14). `apply_common_env` silences GCM / `credential.helper` / `core.askPass` in non-TTY / `--unattended` runs.

18. **`--unattended` global flag** + `VIBE_UNATTENDED` env-var.

19. **MCP command confirm-prompt is TTY-gated** (M1.14.3).

20. **Comment-preserving `vibe.toml` writes** (M1.14.2). Three layers via `toml_edit`.

21. **`[requires.packages]` table-form schema** (M1.15, PROP-002 §2.4.1). Map values are either a version-constraint string OR an inline table for git-source.

22. **Resolution priority: override > git-source > registry-walk** (M1.15).

23. **`source_kind` discriminant in lockfile** (M1.15). New enum `Registry` / `Git` / `Override` per `[[package]]`.

24. **Registry redirect via stub repo** (M1.16, PROP-002 §2.4.2). A registry org's `<kind>-<name>` slot may carry `vibe-redirect.toml` instead of `vibe-package.toml`. Hop limit = 1; identity check on target's `[package]`. Lockfile records `via_redirect` alongside `source_url`.

25. **Two-layer auth for redirect** (M1.16). Stub auth = registry's auth; target auth = `[redirect].auth` (independent).

26. **GitHub `upload-archive` refusal → clone fall-back** (M1.16 fix). `git archive --remote=https://github.com/...` is server-side disabled. `fetch_manifest_at_ref` and `try_fetch_redirect_for_url` both fall back to `refresh_package` on `ArchiveUnsupported`.

27. **GitVerse new orgs require SSH for reads** (2026-05-12). Canonical `vibespecs` happens to be HTTPS-readable; new orgs are not. Live tests and operator docs use SSH form for GitVerse URLs.

---

## Recent commit chain (last 25, newest first)

```
4e852f0 docs(registry-redirect,changelog,wal,continue): note test-org re-home
dbba8d7 test(cli,manual-tests): move live + smoke fixtures to test orgs
ad9b8b3 docs(continue,wal): M1.15 + M1.16 ship-complete checkpoint
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
```

---

## Quick-start commands

```powershell
# Build everything.
cargo build --workspace

# Full hermetic test gate (matches CI).
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p vibe-cli -- check --path . --quiet

# Or one-shot via the bundled script.
bash tools/self-check.sh

# Live e2e (hits real internet — vibespecstest1/2/3).
cargo test -p vibe-cli --test cli_live_e2e -- --ignored

# Install vibe into ~/.cargo/bin/.
cargo install --path crates/vibe-cli --locked

# Signature recipes from this session:

# Add a git-source dep (M1.15):
vibe install flow:internal-helper \
  --git https://github.com/me/flow-internal-helper \
  --tag v0.1.0

# Create a registry-redirect stub (M1.16):
vibe registry redirect flow:internal-helper \
  --to https://github.com/external-author/flow-internal-helper \
  --description "Delegated to external-author"

# Mirror target tags into the stub (M1.16):
vibe registry redirect-sync flow:internal-helper

# Migrate a repo to a new org (no vibevm command — raw git + curl):
git clone --mirror <old-url> m.git
git --git-dir=m.git push --mirror "https://x-access-token:$TOKEN@github.com/<new-org>/<new-name>.git"
```

---

## Pointer

`spec/WAL.md` is the canonical **living** checkpoint. If anything in this `CONTINUE.md` disagrees with the top of `spec/WAL.md`, trust the WAL — it gets bumped every session.
