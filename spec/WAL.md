# WAL — Project Continuation State
_Updated: 2026-04-23_

## Current phase

**M1.5-gate content slice: 3/3 demo flows published.** The registry
at `git@gitverse.ru:anarchic/vibespecs.git` now carries three
v0.1.0 flows. The live three-package smoke passed on 2026-04-23
(vibespecs commit `2203239`).

Today's additions:

1. **`flow:sync-from-code@0.1.0`** at `flow/sync-from-code/v0.1.0/`,
   vibespecs commit `47582af`. Protocol for reconciling specs with
   code when code changed first. Derived from book chapter 3
   ("Архитектура памяти", subsection "Протокол Sync-from-Code").
   Ships three spec files + boot snippet `20-flow-sync-from-code.md`.
2. **`flow:atomic-commits@0.1.0`** at `flow/atomic-commits/v0.1.0/`,
   vibespecs commit `2203239`. One commit = one idea, in
   Conventional Commits format. Derived from book chapter 2
   ("Shared state: файлы как IPC", subsection "Атомарность"). Ships
   three spec files + boot snippet `30-flow-atomic-commits.md`.

Registry-level `README.md` lists all three v0.1.0 packages. Local
`packages/` fixture under this repo mirrors the registry exactly and
is used by `cli_e2e.rs` tests as a hermetic `--registry <path>`
fixture.

**Live verification (2026-04-23).** `vibe init` → install all three
packages against the real registry on Windows / Git Bash produces:

- Exactly one clone under `~/.vibe/registries/46c4a3dfee00a78d/`
  (all three packages share the same clone — URL normalization works).
- Five files in `spec/boot/`: `00-core`, `10-flow-wal`,
  `20-flow-sync-from-code`, `30-flow-atomic-commits`, `90-user`.
- Three `[[package]]` entries in `vibe.lock`, each with the expected
  `git+ssh://git@gitverse.ru/anarchic/vibespecs.git#flow/<name>/v0.1.0`
  source.
- Symmetric uninstall leaves `spec/boot/00-core.md` /
  `spec/boot/90-user.md` / `spec/WAL.md` byte-identical to their
  pre-install state (verified via `cmp`).

Full procedure pinned in
[`manual-tests/M1.5-gate-multi-package-smoke.md`](../manual-tests/M1.5-gate-multi-package-smoke.md).

**Next active work:** M1.2 — `vibe update <pkgref>` / `vibe update --all`.

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
- **Registry default in `vibe init`.** New projects are scaffolded with `[registry] url = "git@gitverse.ru:anarchic/vibespecs.git"` written into `vibe.toml` automatically. Override with `vibe init --registry-url <URL>` / `--registry-ref <REF>`; opt out with `--no-registry`. Single source of truth for the default: `vibe_core::manifest::DEFAULT_REGISTRY_URL`. See [PROP-000 §7](common/PROP-000.md#registry).
- **Manual-test protocol:** human-runnable smoke-tests live in [`manual-tests/`](../manual-tests/) at the repo root. One file per scenario, named `<milestone>-<slug>.md`, each self-contained with a clean-slate setup (`mktemp -d` + `VIBE_REGISTRY_CACHE`) and a teardown block. Policy pinned in [PROP-000 §14](common/PROP-000.md#manual-tests); authoring rules and index in [`manual-tests/README.md`](../manual-tests/README.md). Run these before tagging a milestone, after touching an integration surface, and as reproducers for user-reported bugs.
- **Work in staging order.** M0 (done), M1 (active), M1.5. No jumping ahead.
- **REVIEW marker discipline:** when the spec is silent, pick the conservative interpretation, mark with `<!-- REVIEW: … -->`, surface in the session report.
- **`refs/` is not committed.** Contents are upstream reference material (book + cloned study repos); kept out of the vibevm distribution both to respect upstream copyright and to keep the repo lean.

## Remotes

- **vibevm source (this repo):** `git@gitverse.ru:anarchic/vibevm.git` (SSH) / `https://gitverse.ru/anarchic/vibevm` (web).
- **Package registry:** `git@gitverse.ru:anarchic/vibespecs.git` (SSH). As of 2026-04-23 the registry holds three v0.1.0 flows at `flow/wal/v0.1.0/`, `flow/sync-from-code/v0.1.0/`, `flow/atomic-commits/v0.1.0/` (HEAD `2203239`). The local `packages/` tree under this repo is a **frozen snapshot** of what's on the registry — it exists because `cli_e2e.rs` tests need a hermetic `--registry <path>` fixture with no network, and must stay on `v0.1.0`. When the registry gains new versions, the fixture does NOT follow.

## Done

### M0 — walking skeleton (complete, published)

- [x] `VIBEVM-SPEC.md` received (v1.0), book and reference sources read.
- [x] Project rules landed in `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` and [PROP-000 §12](common/PROP-000.md#commits).
- [x] `git init`, `.gitignore`, `LICENSE.md` (proprietary EULA placeholder).
- [x] Boot snippets, `VIBEVM-SPEC.md` URL reconciliation, PROP-000 foundation.
- [x] Cargo workspace with 7 crates.
- [x] `vibe-core`, `vibe-registry`, `vibe-install`, `vibe-cli` — full plan / apply / register / uninstall loop against a local-directory registry, with boot-snippet numeric-prefix conflict detection, user-owned path guards, and the `flow:wal` canonical payload. 64 tests green at the M0 tag.

### M1.1 — git-backed registry (shipped 2026-04-22)

- [x] [PROP-001](modules/vibe-registry/PROP-001-git-backend.md), the `GitBackend` trait + `ShellGit` impl, the `Registry` trait at the crate root with both `LocalRegistry` and `GitRegistry` implementations, the normalized-URL hash cache under `~/.vibe/registries/<hash>/{clone,meta.toml}`, 1-hour freshness TTL, `git+<transport>://…` lockfile source URIs, `strip_git_plus_prefix` for handoff to git, `vibe registry sync` command, `VIBE_REGISTRY_CACHE` env-var override.
- [x] End-to-end test `install_from_git_registry`; live smoke
  [`M1.1-git-registry-smoke.md`](../manual-tests/M1.1-git-registry-smoke.md)
  passed against the real registry on 2026-04-22.
- [x] `vibe init` writes `[registry]` pointing at the default GitVerse registry by default; overridable with `--registry-url` / `--registry-ref`, opt-out with `--no-registry`.

### M1.5-gate content — three v0.1.0 demo flows (published 2026-04-22 / 2026-04-23)

- [x] `flow:wal@0.1.0` at vibespecs `98e51fc` (2026-04-22) — the
      canonical flow; reference implementation of mirror layout and
      boot-snippet prefix `10-`.
- [x] `flow:sync-from-code@0.1.0` at vibespecs `47582af` (2026-04-23)
      — Sync-from-Code protocol, boot-snippet prefix `20-`.
- [x] `flow:atomic-commits@0.1.0` at vibespecs `2203239` (2026-04-23)
      — atomic commits + Conventional Commits discipline, boot-snippet
      prefix `30-`.
- [x] Registry-level `README.md` lists all three.
- [x] Local `packages/` fixture in this repo mirrors the registry
      exactly (used by `cli_e2e.rs` tests as hermetic fixture).
- [x] Live multi-package smoke
      [`M1.5-gate-multi-package-smoke.md`](../manual-tests/M1.5-gate-multi-package-smoke.md)
      passed 2026-04-23: three distinct prefixes coexist, one shared
      clone, symmetric uninstall, user-owned files byte-identical.

The remaining M1.5-gate item — **docs**
(`docs/commands/*.md`, `docs/authoring-{flow,feat,stack}.md`) — is
open but does not block M1.2. Can be done in parallel with M1.2.

## In progress

Nothing active. M1.5-gate content is done. M1.5-gate docs open but
not started.

## Next

**Immediate (next session).** Start M1.2 — `vibe update`:

1. Plan the diff format first. `vibe update flow:wal` should show
   per-file adds / removes / modifies against the currently-installed
   version. Modified-file case needs a three-way guard (cache hash vs
   on-disk hash vs new version's hash) to detect local edits.
2. Re-fetch logic reuses `GitRegistry::fetch` + TTL from M1.1; no new
   network plumbing needed.
3. CLI surface: `vibe update <pkgref>` (single) and `vibe update
   --all` (sweep every lockfile entry).
4. Lockfile rewrite is the same rollback-safety pattern as install —
   write the new lockfile only after all files are applied
   successfully.

Acceptance: installing `flow:wal@0.1.0`, manually editing one spec
file in the project, running `vibe update flow:wal` against a
registry that now ships `flow:wal@0.2.0` must refuse cleanly and tell
the user their edit will be overwritten.

**After M1.2.** M1.3 — `vibe check` (the 10 §12 checks); M1.4 —
`vibe show effective` / `graph` / `config`. Pure inspection, no new
registry work.

**M1.5-gate docs (any time).** Can happen in parallel with M1.2 /
M1.3 — they touch different code paths.

## Known issues

- **`install:update-lockfile` ordering on partial failure.** Apply rolls back written files best-effort and does NOT touch the lockfile. Documented M0 behaviour; unchanged in M1.1. Revisit in M1.2 while the lockfile rewrite logic is fresh.
- **`tessl-mcp` clone was effectively empty.** Not blocking; Tessl ideas are covered by public docs and the book.
- **M0 boot-snippet validator** rejects `NN` prefixes outside `10..90` with a terse "reserved range" message. Error-message polish is an M2 item.
- **Path display on Windows** strips `\\?\` UNC prefixes for human-readable output; lockfile stores forward-slash relative paths, so lockfiles are portable across OSes.
- **Line-ending warnings** on every commit — `.gitattributes` with `* text=auto eol=lf` would silence them. Listed as a side-quest in [`ROADMAP.md`](../ROADMAP.md). Publishing new packages to vibespecs today surfaced them again for every markdown file in the new packages.
- **Registry cache locking** — if two `vibe` invocations race on the same registry hash, both may attempt to clone / fetch into the same `~/.vibe/registries/<hash>/`. Noted in PROP-001 §6 as an M2 hardening item; M1.1 behaviour is "if a clone fails, delete the cache dir and retry".

## Session context

- **Entry point for next session:** read `CLAUDE.md`, then this WAL, then [PROP-000](common/PROP-000.md). M1.2 has no PROP yet — drafting one is the first task of the next session.
- **Do NOT touch:** `VIBEVM-SPEC.md` (owner-frozen), `refs/book/**`, `spec/boot/00-core.md`, `spec/boot/90-user.md`, or any `packages/flow/*/v0.1.0/` fixture (canonical test payloads — changes must be a new version `v0.1.1` / `v0.2.0`).
- **Key commands to know:**
  - `cargo test --workspace` — 81 tests green on 2026-04-23, 0 warnings, clippy clean.
  - `cargo clippy --workspace --all-targets -- -D warnings` — clean.
  - `cargo run -p vibe-cli -- init --path <dir>` — scaffold a project.
  - `cargo run -p vibe-cli -- install flow:wal --path <project>` — install against the default registry on gitverse.
  - `cargo run -p vibe-cli -- install flow:sync-from-code --path <project>`.
  - `cargo run -p vibe-cli -- install flow:atomic-commits --path <project>`.
  - `cargo run -p vibe-cli -- install flow:wal --registry $(pwd)/packages --assume-yes --path <project>` — install from the in-repo local fixture (M0 path).
  - `cargo run -p vibe-cli -- registry sync --path <project>` — force-refresh the git registry.
  - `VIBE_REGISTRY_CACHE=$(pwd)/tmp-cache cargo run -p vibe-cli -- install flow:wal --path <project>` — run with a test-isolated registry cache.
  - `git push origin main` — routine push to gitverse for this repo. Force-push / history rewrite need owner approval (rule 4).
