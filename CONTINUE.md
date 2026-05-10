# CONTINUE — cold-resume checkpoint

_Written: 2026-05-10 late-session (M1.15 + M1.16 implementation landed; CLI helper for stub creation + production smoke walk pending). Owner-readable, self-contained. Pick this up with zero prior context._

---

## TL;DR (executive summary)

**The 2026-05-10 push lands two complete features in sequence — M1.15 (git-source dependencies, consumer-side) and M1.16 (registry redirect, org-side delegation).** Together they extend `vibevm`'s package-source surface with the Cargo / npm / Poetry / Bundler git-dep idiom (M1.15) plus a Linux-distro-style virtual `Provides:` mechanism for org owners to delegate package hosting to external authors (M1.16) — both with full M1.14 token-discipline preserved through the new code paths.

**M1.16 implementation commits this session (newest first; on top of M1.15):**

```
<pending> docs(commands,registry-redirect,wal): user-facing redirect reference + checkpoint
6e861ac feat(vibe-registry): MultiRegistryResolver follows vibe-redirect.toml stubs
b37e1b3 feat(vibe-core,vibe-registry,vibe-install): vibe-redirect.toml parser + via_redirect lockfile field
```

**M1.15 implementation commits this session (also today):**

```
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

**The 2026-05-10 push lands the full M1.15 implementation — git-source dependencies, the Cargo / npm / Poetry / Bundler-style "whole repo = one package" affordance.** Six commits across vibe-core (schema), vibe-registry (single-package constructor + resolver dispatch), vibe-install (lockfile `source_kind`), vibe-cli (CLI flags + wiring). Wire-form changes from `packages = ["flow:wal@^0.3"]` (legacy array) to `[requires.packages] "flow:wal" = "^0.3"` (modern map) — both parse forever, only the map form is written. Inline-table values declare git-source: `"flow:internal" = { git = "...", tag = "v0.1.0" }`. The earlier 2026-05-09 push closed the M1.14 deferred-list and validated the full registry-auth runtime against a live private GitHub repo; that work remains live.

**Six M1.15 implementation commits this session (newest-first; on top of the 2 PROPOSED spec commits from yesterday):**

```
90bf10b feat(vibe-cli): vibe install --git/--tag/--branch/--rev for git-source declarations
a7dce7f feat(vibe-core,vibe-registry,vibe-install): lockfile source_kind field for git/override discriminant
153f3a2 feat(vibe-cli): wire git-source declarations through install/update/outdated
161b7b1 feat(vibe-registry): MultiRegistryResolver dispatches to git-source declarations
c313ebd feat(vibe-registry): GitPackageRegistry::open_single_package for git-source
2544d76 feat(vibe-core): [requires.packages] table-form schema with git-source slot
```

**The 2026-05-09 push closed the entire M1.14 deferred-list and validated the full registry-auth runtime against a live private GitHub repo.** Three commits land on top of yesterday's M1.12 + M1.13 + M1.14 (.1 / .2 / .3) push: a new `vibe registry test` diagnostic command, a structured-JSON error envelope for "package not found in any registry" failures (`error_kind` + `attempts` array), and the corner-case finish on inline-comment preservation inside `[[registry]]` blocks. Plus the production walk against `olegchir/vibevm-private-probe` (private GitHub repo) confirmed end-to-end that `auth = "token-env"` works, `MissingToken` precheck fires, `vibe registry test` correctly classifies token states, and the token never lands on disk in `.git/config` after a clone. The full M1.14 surface is feature-complete for v0.

Workspace state at HEAD `2541083` (after this session's three commits):
- vibe-cli e2e: **89 hermetic + 3 ignored** (no count change — `vibe registry test` is wired through CLI dispatch but its end-to-end coverage came from the live walk, not new hermetic e2e tests).
- vibe-core: **116 hermetic** (was 115; +1 inline-kv comment-preservation test).
- vibe-registry: **94 hermetic** (unchanged; `RegistryWalkAttempt` / `WalkAttemptStatus` made public-with-serde but no behavioural diff to test).
- vibe-cli bin: **93 hermetic** (unchanged).
- `cargo test --workspace` all green; `cargo clippy --workspace --all-targets -- -D warnings` clean; `vibe check --path . --quiet` 0/0/0.

Working tree is clean. `origin/main` is still at `8ab5c9c` until this session's three commits + the docs / WAL / CONTINUE updates push. No active blockers. **The M1.14 deferred-list is now empty.** Next major surface is M1.5 (LLM generation) — non-routine, requires owner sign-off.

---

## Where we are right now

- **Branch:** `main`. Working tree clean (after this session's three code commits — docs / WAL / CONTINUE update is the next planned commit).
- **This session's commits (newest-first; on top of yesterday's 25):**

  ```
  2541083 feat(vibe-cli): vibe registry test diagnostic command
  41f567b feat(resolver,registry,cli): structured per-registry attempts in JSON error envelope
  568825b feat(vibe-core): preserve inline-key comments inside vibe.toml writes
  ```

- **Latest commits previous session (newest-first; full session was 25 commits):**

  ```
  ec77471 docs(wal): session-end checkpoint — pointer to CONTINUE.md
  d4ee973 docs(continue): cold-resume checkpoint at 2026-05-08 session-end
  8ab5c9c docs(roadmap,changelog): catch up on M1.12 / M1.13 / M1.14 milestones
  a915b12 docs(commands,wal): surface-consistency closing slice
  1f58e71 feat(vibe-cli): surface consistency — MCP --yes wired, --auth-required + --exact reach
  5c2b504 docs(commands,wal): closing-slice landings — strict-auth, aggregated report, comment preservation
  cac03fe feat(vibe-core): toml_edit-based comment-preserving writes for vibe.toml
  d7bf8bb feat(vibe-registry,vibe-cli): --auth-required + aggregated per-registry error report
  bf4111d docs(registry-auth,wal): user-facing reference + production-ready checkpoint
  1210268 feat(vibe-registry): per-auth walk-vs-halt + auth plumbing in resolver
  8942ee7 feat(vibe-registry): token injection + bootstrap-with-scrub for auth=token-env
  6dc8747 feat(vibe-registry): classifier + GitBackend::set_remote_url
  41efc0c feat(vibe-registry): TTY-aware credential helper silencing
  e65c73e feat(vibe-cli): --auth and --token-env on `vibe registry add`
  97753f7 feat(vibe-core): AuthKind enum + RegistrySection.auth/token_env
  5f296d9 docs(spec): per-registry auth axis (PROP-002 §2.2.1) + 401 classifier rules
  c9c18d7 docs(commands): document --unattended flag for scripted runs
  8420df5 feat(vibe-cli): --unattended global flag + VIBE_UNATTENDED env-var
  1572a11 docs(commands/mcp-install): provisioning recipe + best-effort `--scope both`
  b4cdcd7 feat(vibe-cli/mcp): --scope both is best-effort on the project leg
  01c5531 docs(versions): user-facing version-syntax reference
  8e84b6b docs(spec,commands,roadmap,wal): cargo-shape version syntax + --exact
  7992bca feat(vibe-cli/install): caret default constraint + --exact flag
  a158475 refactor(vibe-core,vibe-resolver): bare semver follows Cargo (caret) not exact
  d719457 feat(vibe-cli/search): hint that install bypasses index when search is empty
  e41a478 docs(vibe-cli/mcp): SKILL.md happy-path + --assume-yes + search/registry guards
  1697f5a docs(commands,roadmap,wal): refresh install/uninstall + checkpoint
  ```

- **Active blocker:** none.

---

## What to do first in the next session

The M1.14 deferred-list is now empty. Pick whichever matches the owner's interest:

### Option 1 — M1.5 (LLM generation)

The next major milestone per ROADMAP. **Non-routine — needs explicit owner sign-off before starting.** M1.5 is what makes vibevm "produce software" rather than "manage specs." Scope, design constraints, and entry points need to be discussed before any code lands. The whole previous session converged on "registry-auth surface is feature-complete; M1.5 is next" — this is the natural pickup.

### Option 2 — re-run the production walk

Useful as a smoke test whenever the auth pipeline changes. Recipe (also recorded in `spec/WAL.md` under M1.14.4):

1. Confirm `olegchir/vibevm-private-probe` still exists on GitHub (or recreate from `manual-tests/` recipe — to be added if the repo gets deleted).
2. Fresh consumer project: `vibe init`, then add the registry: `vibe registry add probe "https://github.com/olegchir" --auth token-env`.
3. `vibe registry test --path .` — expect `missing-token` for the probe registry.
4. `export VIBEVM_REGISTRY_TOKEN_GITHUB_COM=<PAT>` (read scope on the private repo).
5. `vibe registry test --path .` — expect `reachable`.
6. `vibe install flow:vibevm-private-probe --assume-yes` — expect a clean install with `spec/flows/private-probe/PROBE.md` materialised.
7. `grep -r x-access-token ~/.vibe/registries/` — must come up empty.
8. Inspect `~/.vibe/registries/<hash>/packages/flow-vibevm-private-probe/clone/.git/config` — `remote.origin.url` must be the plain credential-free form.

### Option 3 — Tag v0.1.0

CHANGELOG `[Unreleased]` carries M1.12 / M1.13 / M1.14 / M1.14.x. ROADMAP marks all four as SHIPPED. The full registry-auth surface is now feature-complete. If the owner is ready to cut the first tagged release, the steps are: lift `[Unreleased]` to `[0.1.0]` in CHANGELOG, finalise `Cargo.toml` versions across crates, `git tag v0.1.0`, push tag, optionally publish to a binary release channel.

### Option 4 — A fresh full-project audit

After three intense days of feature work, a "step back and read what we've got" pass would be high-value. Does `docs/` cover every surface? Are there orphan TODOs? Is the spec tree internally consistent? Possible lightweight scope: walk every `docs/commands/*.md`, every `spec/modules/**/*.md`, run `vibe check`, look for FIXMEs. Half-day exercise; sets up M1.5 with a clean baseline.

---

## Non-obvious findings from this session

These cost time / hit edge cases — write them down so a future session does not re-derive.

### Rust 2024 edition forbids `unsafe` env-var mutation; tests need a workaround

`#![forbid(unsafe_code)]` at the crate level (which `vibe-registry` and `vibe-cli` both carry) blocks `std::env::set_var` because Rust 2024 marks it `unsafe`. Tests that mutate process env to exercise env-var-driven code paths cannot use `set_var` directly. Two approaches in this codebase:

1. **`Mutex<()>`-serialised env tests** in `vibe-cli::output::tests` — `INVOKED_BY_LOCK` and `UNATTENDED_LOCK` static `Mutex<()>` ensure parallel tests don't observe each other's transient env writes; inside the lock, tests use `EnvGuard` / `UnattendedGuard` RAII patterns wrapping `unsafe { ... set_var ... }` blocks. The crate must allow `unsafe_code` at the test scope for this to work.
2. **Test-only doc-hidden constructors** — `GitPackageRegistry::open_with_explicit_token` takes the resolved token directly (`Option<String>`) instead of reading an env-var. Production code calls `open_with_auth` (env-driven); tests call the explicit-token sibling. This avoids env mutation entirely. Cleaner; preferred when feasible.

If a future test needs to drive env-var resolution and neither approach fits, opening a test-only API like `open_with_explicit_token` is the path of least resistance.

### `apply_common_env` order matters: env BEFORE args

`ShellGit::run` prepends `-c credential.helper= -c core.askPass=` flags via `apply_common_env`. Those are global git options that **must** come before the subcommand name (`git -c k=v ls-remote ...`, not `git ls-remote -c k=v`). Every callsite of `Command::new("git")` followed by `apply_common_env(&mut cmd)` followed by `cmd.args(args)` needs that exact order. The ShellGit private methods (`run`, `run_raw`, `preflight`) and the test helper `run_or_panic` were all reordered in commit `41efc0c`; if a new git-spawning callsite is added, it must follow the same order or git will refuse the args as sub-command parameters.

### `GIT_ASKPASS=""` (empty value) confuses git's startup probe on some platforms

Setting `GIT_ASKPASS` to an empty string on Windows can produce a `cmd /C ""` invocation that git interprets as "askpass available, exec it" → fails the startup. Solution: don't set `GIT_ASKPASS` at all when silencing — `core.askPass=` (empty value via `-c`) plus `GIT_TERMINAL_PROMPT=0` plus `credential.helper=` is sufficient. The original silencing block tried `cmd.env("GIT_ASKPASS", "")`, hit this exact failure on every git invocation in the test suite, and was reverted to "leave GIT_ASKPASS alone."

### Token must NOT persist in `.git/config` after bootstrap

The token-discipline invariant is "token never on disk via vibevm-controlled paths." A naive `bootstrap(credentialed_url)` saves the credentialed URL into `<dest>/.git/config` as `remote.origin.url = https://x-access-token:<TOKEN>@...`, which violates the invariant. The fix in `update_clone_at_ref` is to immediately call `backend.set_remote_url(clone_dir, "origin", plain_url)` after a successful bootstrap — git `remote set-url` is a config write, not a network operation, and overwrites the credentialed URL with the plain one. Subsequent `update` calls hit the plain origin; if it returns 401 (still-private host), `ensure_clone_against_sources` wipes and re-bootstraps. Slight perf cost on stale-cache-against-private-host paths; acceptable trade.

### bare semver in Cargo crate IS caret, not exact

This was the reason for the M1.13 parser collapse. `semver::VersionReq::parse("0.3.0")` returns a caret-comparator, not an exact one. The pre-this-session vibevm parser explicitly converted bare semver to `=0.3.0` via `format!("={version}")` — that wrapper made vibevm diverge from cargo / npm / Poetry. Removing the wrapper (commit `a158475`) brought us in line. Pre-1.0 caret semantics are tighter than post-1.0 (`^0.3.0` matches only `0.3.x`, not `0.4.0`); since every vibevm package today is `0.x.y`, this is the intended behaviour.

### `vibe.toml` is mutated by `vibe install` / `uninstall` / `registry add` (M1.12+)

Pre-M1.12 `vibe.toml` was append-once: `vibe init` wrote it, the operator hand-edited it, no command rewrote it. M1.12 introduced `[requires].packages` writes from `vibe install` and `vibe uninstall`. M1.14.1 added `vibe registry add --auth ...` write. M1.14.2 layered `toml_edit` on top to preserve comments. Future commands that need to mutate `vibe.toml` should use the same `ProjectManifest::write` path — it goes through the comment-preserving merge automatically.

### Public-401 walk-past + GitVerse policy

GitVerse returns 401 (not 404) for missing public repos. Without the M1.14 walk-past rule, vibevm would halt the first time it hit a non-existent package against a project that has `vibespecs-gitverse` (the default GitVerse registry). PROP-002 §2.3.1 reclassifies "401 against `auth = "none"`" as `UnknownPackage` so the resolver walks to the next registry. `--auth-required` flips this back to halt-on-public-401 for CI / cron use cases where a public substitute would be wrong.

### MCP commands are TTY-confirm, NOT non-TTY-confirm

The M1.14.3 closer wires MCP `--yes` to a real apply-confirm prompt — but ONLY on a TTY. Non-TTY callers (CI / opencode) get the pre-existing zero-confirm behaviour preserved. Operators on a TTY without an explicit skip-flag now see `[y/N]` before any MCP-config / SKILL.md write. The TTY-gate condition is:

```rust
if args.yes || ctx.is_unattended() || args.auto || ctx.is_json()
   || !console::user_attended() {
    // approved, no prompt
}
```

The `!console::user_attended()` short-circuit at the bottom is what preserves CI-script compat. If a future change wants to make MCP commands strictly confirm in non-TTY too, that condition is the one place to touch.

### `#[error(transparent)]` does NOT propagate `downcast_ref` through anyhow chain depth

Discovered while wiring the JSON-mode structured error envelope (M1.14.4). The setup: `SolveError::Provider(#[from] DepProviderError)` is `#[error(transparent)]` so that the `Display` impl forwards to the inner error. Surface intuition: walking `anyhow::Error::chain()` and calling `cause.downcast_ref::<DepProviderError>()` on each link should find the inner type at some depth. **It does not.** Empirically the chain stops at `SolveError`; the `transparent` attribute affects only `Display` / `source()`, not the type identity that `downcast_ref` operates on.

Fix shape: when the chain walk wants to reach the inner type behind a transparent wrapper, downcast to the wrapper itself and pattern-match:

```rust
let candidate: Option<&DepProviderError> =
    if let Some(d) = cause.downcast_ref::<DepProviderError>() { Some(d) }
    else if let Some(SolveError::Provider(d)) = cause.downcast_ref::<SolveError>() { Some(d) }
    else { None };
```

Same pattern will apply to any future structured-error work that crosses `#[error(transparent)]` wrappers. Search for `#[error(transparent)]` in the codebase before adding a new chain-walking error stamper — every transparent wrapper is a hop the walk can't transparently make.

### Token-discipline invariant is now production-validated, not just unit-tested

The hermetic suite asserts that `set_remote_url` rewrites `.git/config` to the plain URL after bootstrap. The 2026-05-09 production walk confirmed the same on a real filesystem against `olegchir/vibevm-private-probe` (private GitHub): after a successful `vibe install flow:vibevm-private-probe`, the recursive `grep -r x-access-token ~/.vibe/registries/` came up empty and the `clone/.git/config` recorded the plain URL. This is the canonical smoke recipe — re-run when touching anything in the auth pipeline.

---

## Repository map

```
vibevm/
├── CLAUDE.md / AGENTS.md / GEMINI.md   # Three identical copies of the four rules.
├── CONTINUE.md                          # This file. Cold-resume snapshot.
├── ROADMAP.md                           # Milestone-oriented plan; M1.14 closed via this push.
├── CHANGELOG.md                         # Milestone chronicle; [Unreleased] holds M1.12/M1.13/M1.14.
├── VIBEVM-SPEC.md                       # Owner-frozen spec; do not edit without explicit instruction.
├── DEV-GUIDE.md / RUNTIME-GUIDE.md      # Per-machine setup docs.
├── crates/
│   ├── vibe-cli/                        # `vibe` binary entry point. clap dispatch + per-subcommand modules.
│   │   └── src/commands/
│   │       ├── install.rs               # Resolve+plan+apply pipeline. M1.12 [requires] writes,
│   │       │                            # M1.13 caret-default + --exact, M1.14 --auth-required.
│   │       ├── update.rs                # Re-resolve+diff+apply. M1.14.3 --exact + --auth-required reach.
│   │       ├── uninstall.rs             # Symmetric to install. M1.12 [requires] cleanup.
│   │       ├── outdated.rs              # Read-only upstream-newer probe. M1.14.3 --auth-required reach.
│   │       ├── mcp.rs                   # Five-agent matrix; install/upgrade/uninstall/status/serve.
│   │       │                            # M1.14.3: walk_install/upgrade/uninstall extracted +
│   │       │                            # TTY-gated confirm prompt + --assume-yes alias.
│   │       ├── registry.rs              # add (with --auth/--token-env) / remove / list / set-mirror /
│   │       │                            # sync / vendor / publish.
│   │       ├── search.rs                # PROP-005 index-aware discovery; auth-naive (read-only).
│   │       └── skill_template.md        # Vendored two-state SKILL.md (Section A bootstrap + B + Common).
│   ├── vibe-core/                       # Manifests, lockfile schema v3, AuthKind, version_spec.
│   │   └── src/manifest/
│   │       ├── project.rs               # ProjectManifest + RegistrySection.auth/token_env (M1.14.1).
│   │       ├── package.rs               # PackageManifest (vibe-package.toml) + Requires/Provides.
│   │       ├── lockfile.rs              # vibe.lock schema v3 with full provenance.
│   │       └── mod.rs                   # write_toml() — comment-preserving via toml_edit (M1.14.2).
│   ├── vibe-graph/                      # In-memory dep graph helpers.
│   ├── vibe-registry/                   # The big crate — git_backend, GitPackageRegistry,
│   │   │                                # MultiRegistryResolver. Auth runtime lives here.
│   │   └── src/
│   │       ├── git_backend/
│   │       │   ├── mod.rs               # GitBackend trait + set_remote_url method (M1.14).
│   │       │   └── shell.rs             # ShellGit + apply_common_env (TTY-aware silencing).
│   │       ├── git_package_registry.rs  # Per-registry instance with auth + effective_token +
│   │       │                            # token_env_name; bootstrap-with-scrub flow.
│   │       ├── multi_registry_resolver.rs # Walk + per-auth walk-vs-halt + strict_auth gate.
│   │       └── lib.rs                   # RegistryError including MissingToken +
│   │                                    # PackageNotFoundEverywhere variants.
│   ├── vibe-resolver/                   # Feature expansion + activation evaluation (PROP-003).
│   ├── vibe-install/                    # Install pipeline: plan_install → apply → register.
│   ├── vibe-llm/                        # LLM provider abstraction. Skeleton — real impls land in M1.5.
│   ├── vibe-mcp/                        # JSON-RPC MCP server. 3 tools today.
│   ├── vibe-check/                      # Spec-consistency linter.
│   ├── vibe-publish/                    # GitHubCreator / GitVerseCreator / DirectGitCreator publishers.
│   └── vibe-wire/                       # JTD-codegen'd wire types.
├── services/
│   └── vibe-index/                      # Standalone PROP-005 utility: per-org package index. Own workspace.
├── spec/
│   ├── boot/{00-core,90-user}.md        # Read at every session start.
│   ├── WAL.md                           # Living checkpoint of project state. Authoritative if it
│   │                                    # diverges from this file.
│   ├── common/PROP-000…PROP-006         # Foundation policy + operating modes.
│   ├── modules/                         # Per-crate PROPs.
│   │   └── vibe-registry/PROP-002       # §2.2.1 (auth axis), §2.3.1 (failure classifier).
│   └── research/PROP-004                # Tessl comparative research.
├── docs/
│   ├── README.md                        # User-doc index; gained "Version syntax" + "Registry auth".
│   ├── architecture.md / lockfile-format.md / glossary.md / troubleshooting.md
│   ├── version-syntax.md                # NEW (M1.13) — operator reference for semver constraints.
│   ├── registry-auth.md                 # NEW (M1.14) — operator reference for the four auth regimes.
│   ├── commands/                        # Per-subcommand reference. install / update / mcp-* / registry-*
│   │                                    # all updated with --auth-required / --exact / --unattended notes.
│   ├── guides/                          # Long-form walkthroughs.
│   └── authoring-{flow,feat,stack}.md
├── manual-tests/                        # Runnable smoke protocols.
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

15. **Token never on disk via vibevm-controlled paths** (M1.14). Tokens loaded once at registry-open from env-var, held in memory only, scrubbed from `.git/config` immediately after `bootstrap` via `set_remote_url(.., "origin", plain_url)`.

16. **TTY-aware credential silencing** (M1.14). `apply_common_env` silences GCM / `credential.helper` / `core.askPass` in non-TTY / `--unattended` runs. Interactive TTY without `--unattended` leaves them alone — operator might genuinely want a one-off password prompt.

17. **`--unattended` global flag** + `VIBE_UNATTENDED` env-var (truthy: `1`, `true`, `yes`, `on`). Implies skip-confirm everywhere; refuses to open wizards in MCP install; stamps `unattended: true` on every JSON envelope.

18. **MCP command confirm-prompt is TTY-gated** (M1.14.3). Non-TTY callers see the pre-this-version zero-confirm behaviour preserved; TTY callers see a real `[y/N]` summary unless they pass `--yes` / `--unattended` / `--auto` / `--json`.

19. **comment-preserving `vibe.toml` writes** (M1.14.2). Three layers via `toml_edit`: header comments, per-table prefix, document trailing. Inline-comments inside tables not yet preserved (deferred enhancement).

---

## Recent commit chain (last 25, newest first)

```
8ab5c9c docs(roadmap,changelog): catch up on M1.12 / M1.13 / M1.14 milestones
a915b12 docs(commands,wal): surface-consistency closing slice
1f58e71 feat(vibe-cli): surface consistency — MCP --yes wired, --auth-required + --exact reach
5c2b504 docs(commands,wal): closing-slice landings — strict-auth, aggregated report, comment preservation
cac03fe feat(vibe-core): toml_edit-based comment-preserving writes for vibe.toml
d7bf8bb feat(vibe-registry,vibe-cli): --auth-required + aggregated per-registry error report
bf4111d docs(registry-auth,wal): user-facing reference + production-ready checkpoint
1210268 feat(vibe-registry): per-auth walk-vs-halt + auth plumbing in resolver
8942ee7 feat(vibe-registry): token injection + bootstrap-with-scrub for auth=token-env
6dc8747 feat(vibe-registry): classifier + GitBackend::set_remote_url
41efc0c feat(vibe-registry): TTY-aware credential helper silencing
e65c73e feat(vibe-cli): --auth and --token-env on `vibe registry add`
97753f7 feat(vibe-core): AuthKind enum + RegistrySection.auth/token_env
5f296d9 docs(spec): per-registry auth axis (PROP-002 §2.2.1) + 401 classifier rules
c9c18d7 docs(commands): document --unattended flag for scripted runs
8420df5 feat(vibe-cli): --unattended global flag + VIBE_UNATTENDED env-var
1572a11 docs(commands/mcp-install): provisioning recipe + best-effort `--scope both`
b4cdcd7 feat(vibe-cli/mcp): --scope both is best-effort on the project leg
01c5531 docs(versions): user-facing version-syntax reference
8e84b6b docs(spec,commands,roadmap,wal): cargo-shape version syntax + --exact
7992bca feat(vibe-cli/install): caret default constraint + --exact flag
a158475 refactor(vibe-core,vibe-resolver): bare semver follows Cargo (caret) not exact
d719457 feat(vibe-cli/search): hint that install bypasses index when search is empty
e41a478 docs(vibe-cli/mcp): SKILL.md happy-path + --assume-yes + search/registry guards
1697f5a docs(commands,roadmap,wal): refresh install/uninstall + checkpoint
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

# Public install, scripted, no prompts ever:
vibe --unattended install flow:wal

# Private registry on a fresh user account, one-time setup:
vibe registry add internal "https://gitlab.example/vibespecs" --auth token-env
export VIBEVM_REGISTRY_TOKEN_GITLAB_EXAMPLE=ghp_...
vibe --unattended install flow:internal-helper

# MCP provisioning (no project yet — Section A in SKILL.md):
vibe --unattended mcp install --agent opencode --scope both --what both
```

---

## Pointer

`spec/WAL.md` is the canonical **living** checkpoint. If anything in this `CONTINUE.md` disagrees with the top of `spec/WAL.md`, trust the WAL — it gets bumped every session.
