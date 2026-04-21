# WAL — Project Continuation State
_Updated: 2026-04-22_

## Current phase

**Milestone M1.1 — git-backed registry: code-complete.** `vibe-registry`
now speaks git. The `Registry` trait is implemented by both
`LocalRegistry` (M0 code path, still used for `--registry <path>` and
`file://` URLs) and `GitRegistry` (new, clones into
`~/.vibe/registries/<hash>/clone/` and persists freshness metadata in
`meta.toml`). A new subcommand `vibe registry sync` force-refreshes
the configured git registry. All design decisions — shell-out vs
libgit2, the narrow two-method `GitBackend` trait (`bootstrap` /
`update`), cache layout, the `git+<transport>://host/path#kind/name/vN`
lockfile scheme, and the Windows `CREATE_NO_WINDOW` posture — are
pinned in [spec://vibevm/modules/vibe-registry/PROP-001](modules/vibe-registry/PROP-001-git-backend.md).

End-to-end integration test (`install_from_git_registry`) spins up a
bare git repo with the canonical `flow:wal@0.1.0` fixture, points a
project's `vibe.toml` at it via `git+file://…`, runs `vibe install
flow:wal`, and asserts the lockfile carries a `git+…#flow/wal/v0.1.0`
source URI, that `~/.vibe/registries/<hash>/` was populated with
`clone/.git` + `meta.toml`, and that `vibe registry sync` succeeds on
top. 77 tests green across the workspace, 0 warnings, clippy clean.

**Next active work:** M1.1 acceptance items remaining, then M1.2.

## Constraints (do not violate without discussion)

- **Language:** Rust only for the CLI. See [spec://vibevm/common/PROP-000#language](common/PROP-000.md#language).
- **License:** proprietary EULA placeholder (see [`LICENSE.md`](../LICENSE.md)); eventual target is UPL 1.0 — owner's decision, not final. See [spec://vibevm/common/PROP-000#license](common/PROP-000.md#license).
- **Manifest format:** TOML only.
- **Vocabulary lock:** only `flow`, `feat`, `stack`, `tool`. Never `lifecycle`, `phase`, `goal`, `plugin` (except as passing synonym for `package`).
- **User-owned files (never modified by `vibe install`/`uninstall`):** `spec/boot/00-core.md`, `spec/boot/90-user.md`, `spec/WAL.md`, `VIBEVM-SPEC.md`, `refs/book/**`, any 00-09 or 90-99 boot file.
- **Four project rules (authoritative in [spec://vibevm/common/PROP-000#commits](common/PROP-000.md#commits), copied into `CLAUDE.md` / `AGENTS.md` / `GEMINI.md`):**
  1. **Attribution** — this repository is human-authored; never mark any artefact as machine-authored.
  2. **Conventional Commits** — short subject, long explanatory body about *why*.
  3. **Group by meaning** — one logical unit per commit; split mixed working trees.
  4. **Autonomy on routine changes only** — commit and push routine work; stop for history rewrites, force-push, large blobs, CI/signing changes, anything whose reversal costs work.
- **Git backend:** shell-out to the system `git`, behind the `GitBackend` trait so a future `libgit2` swap costs one new impl block. Full rationale in [spec://vibevm/modules/vibe-registry/PROP-001](modules/vibe-registry/PROP-001-git-backend.md). Method name is `bootstrap` (not `clone` / `clone_into`) to dodge std `Clone` / `ToOwned` blanket impls when the backend is held as `Arc<dyn GitBackend>`.
- **Cache root:** `~/.vibe/registries/` by default. `VIBE_REGISTRY_CACHE` env-var overrides (used by tests and users with non-standard home layouts).
- **Work in staging order.** M0 (done), M1 (active), M1.5. No jumping ahead.
- **REVIEW marker discipline:** when the spec is silent, pick the conservative interpretation, mark with `<!-- REVIEW: … -->`, surface in the session report.
- **`refs/` is not committed.** Contents are upstream reference material (book + cloned study repos); kept out of the vibevm distribution both to respect upstream copyright and to keep the repo lean.

## Remotes

- **vibevm source (this repo):** `git@gitverse.ru:anarchic/vibevm.git` (SSH) / `https://gitverse.ru/anarchic/vibevm` (web).
- **Package registry:** `git@gitverse.ru:anarchic/vibespecs.git` (SSH). Seeded with the hand-written `flow:wal@0.1.0`; two more demo packages (`flow:sync-from-code`, `flow:atomic-commits`) planned as the M1.5-gate content.

## Done

### M0 — walking skeleton (complete, published)

- [x] `VIBEVM-SPEC.md` received (v1.0), book and reference sources read.
- [x] Project rules landed in `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` and [PROP-000 §12](common/PROP-000.md#commits).
- [x] `git init`, `.gitignore`, `LICENSE.md` (proprietary EULA placeholder).
- [x] Boot snippets, `VIBEVM-SPEC.md` URL reconciliation, PROP-000 foundation.
- [x] Cargo workspace with 7 crates.
- [x] `vibe-core`, `vibe-registry`, `vibe-install`, `vibe-cli` — full plan / apply / register / uninstall loop against a local-directory registry, with boot-snippet numeric-prefix conflict detection, user-owned path guards, and the `flow:wal` canonical payload. 64 tests green at the M0 tag.

### M1.1 — git-backed registry (code-complete)

- [x] [PROP-001](modules/vibe-registry/PROP-001-git-backend.md) written: shell-out design, `GitBackend` trait shape, cache layout, freshness policy, source-URI format, Windows UX (`CREATE_NO_WINDOW` + `LC_ALL=C`), rejected alternatives, acceptance checklist.
- [x] Timestamp helper hoisted from `vibe-cli` to [`vibe_core::timestamp`](../crates/vibe-core/src/timestamp.rs) (adds `parse_unix_utc` for freshness comparisons).
- [x] [`git_backend::GitBackend`](../crates/vibe-registry/src/git_backend/mod.rs) trait + [`ShellGit`](../crates/vibe-registry/src/git_backend/shell.rs) impl with:
  - `LC_ALL=C` + `LANG=C` + `GIT_TERMINAL_PROMPT=0` on every spawn.
  - Per-instance `OnceLock` preflight cache.
  - `CREATE_NO_WINDOW` on Windows.
  - stderr classification into `RepoNotFound` / `AuthFailed` / `NetworkUnreachable` / `RefNotFound`, with a `CommandFailed` catch-all.
  - Integration tests against a bare fixture repo (skip cleanly without git on PATH).
- [x] `Registry` trait at the crate root; both `LocalRegistry` and [`GitRegistry`](../crates/vibe-registry/src/git_registry.rs) implement it.
- [x] `GitRegistry`: normalized-URL hashing (16-hex prefix, full hash in `meta.toml`), `clone/` + `meta.toml` layout under `~/.vibe/registries/<hash>/`, 1-hour freshness TTL (`>=` comparison so TTL=0 always pulls), `sync()` that forces `fetch`+`reset --hard origin/<ref>`, `fetch()` that stamps the lockfile with a `git+<transport>://host/path#kind/name/vN` URI.
- [x] `vibe install` consumes a `Box<dyn Registry>` picked by `resolve_registry`: `--registry <path>` or `file://` → `LocalRegistry`; anything else → `GitRegistry::open`. `strip_git_plus_prefix` peels the `git+` wrapper before handing the URL to git.
- [x] [`vibe registry sync [--path]`](../crates/vibe-cli/src/commands/registry.rs): forces a pull on the configured git registry; no-ops with a note on a `file://` registry.
- [x] `VIBE_REGISTRY_CACHE` env-var override on `default_cache_root` so tests and non-standard setups don't touch `~/.vibe/`.
- [x] End-to-end test `install_from_git_registry` covers the full install + sync loop against a bare `git+file://…` registry seeded with the canonical fixture.
- [x] 77 tests green, 0 warnings, clippy clean.

## In progress

Nothing active — M1.1 code-complete as of the 2026-04-22 commit burst.

## Next

**M1.1 acceptance — remaining items before tagging M1.1:**
1. Smoke-test against the real `git@gitverse.ru:anarchic/vibespecs.git` from a fresh project: `vibe init`, `vibe.toml` with the real `[registry]`, `vibe install flow:wal`. This is manual — no automated CI runs against GitVerse yet (M2 scope). Record the exact steps in `spec/boot/90-user.md` once proven.
2. Publish two more demo packages to the registry (`flow:sync-from-code` + `flow:atomic-commits`), which will double as regression fixtures for boot-snippet numeric-prefix collision and for multi-package lockfile content.

**M1.2 (after M1.1 sign-off):** `vibe update <pkgref>` / `vibe update --all` — re-fetch, show a per-file diff against the current install, confirm, apply. Three-way guard if the user edited a previously-installed file.

**M1.3 / M1.4:** `vibe check` (the 10 §12 checks) and `vibe show effective` / `graph` / `config` — pure inspection, no registry work beyond what M1.1 shipped.

## Known issues

- **`install:update-lockfile` ordering on partial failure.** Apply rolls back written files best-effort and does NOT touch the lockfile. Documented M0 behaviour; unchanged in M1.1.
- **`tessl-mcp` clone was effectively empty.** Not blocking; Tessl ideas are covered by public docs and the book.
- **M0 boot-snippet validator** rejects `NN` prefixes outside `10..90` with a terse "reserved range" message. Error-message polish is an M2 item.
- **Path display on Windows** strips `\\?\` UNC prefixes for human-readable output; lockfile stores forward-slash relative paths, so lockfiles are portable across OSes.
- **Line-ending warnings** on every commit — `.gitattributes` with `* text=auto eol=lf` would silence them. Listed as a side-quest in [`ROADMAP.md`](../ROADMAP.md).
- **Registry cache locking** — if two `vibe` invocations race on the same registry hash, both may attempt to clone / fetch into the same `~/.vibe/registries/<hash>/`. Noted in PROP-001 §6 as an M2 hardening item; M1.1 behaviour is "if a clone fails, delete the cache dir and retry".

## Session context

- **Entry point for next session:** read `CLAUDE.md`, then this WAL, then [PROP-000](common/PROP-000.md), then [PROP-001](modules/vibe-registry/PROP-001-git-backend.md). Pick the M1.1 acceptance smoke-test (above) as the next actionable.
- **Do NOT touch:** `VIBEVM-SPEC.md` (owner-frozen), `refs/book/**`, `spec/boot/00-core.md`, `spec/boot/90-user.md`, or the `packages/flow/wal/v0.1.0/` fixture (canonical test payload — changes must be a new version).
- **Key commands to know:**
  - `cargo test --workspace` — all green (77 tests, 2026-04-22).
  - `cargo clippy --workspace --all-targets -- -D warnings` — clean.
  - `cargo run -p vibe-cli -- init --path <dir>` — scaffold a project.
  - `cargo run -p vibe-cli -- install flow:wal --registry $(pwd)/packages --assume-yes --path <project>` — install from the in-repo local fixture (M0 path).
  - `cargo run -p vibe-cli -- registry sync --path <project>` — force-refresh the git registry configured in `<project>/vibe.toml`.
  - `VIBE_REGISTRY_CACHE=$(pwd)/tmp-cache cargo run -p vibe-cli -- install flow:wal --path <project>` — run install with a test-isolated registry cache.
  - `git push origin main` — routine push to gitverse. Force-push / history rewrite need owner approval (rule 4).
