# CONTINUE.md — cold-resume checkpoint

_Written 2026-06-27. This session **completed the bridge-packages feature** —
all four orthogonal mechanisms, their canonical compositions, and every
acknowledged deferral now land gate-green. **14 commits this session**
(`a9fad47`→`ac1f2f1`) on `main`. Floor fully green (`self-check.sh` exit 0,
specmap clean). The commits are **local — not yet mirrored** (owner's call)._

> **`spec/WAL.md` is the canonical living state**; if this snapshot and the WAL
> disagree, the WAL wins. The **git log is the authoritative per-item record**.
> Boot first (`CLAUDE.md` → `spec/boot/INDEX.md` → its files → `spec/WAL.md`),
> then read this.

---

## TL;DR

**Bridge packages are done.** A bridge is a maintainer's wrapper around someone
else's repo, decomposed (owner's call) into four orthogonal mechanisms, each
usable alone, each its own spec + tests:

- **PROP-020** install-hooks · **PROP-021** submodule-sources ·
  **PROP-022** materialization-modes (`snapshot`/`hardlink`/`in-place`) ·
  **PROP-023** bridge-packages · **PROP-015 §2.8** `#skill-include`.

The prior session (2026-06-24) landed the **specs + 6 impl slices** (schema,
submodule fetch, skill projection, the hook *runner cell*, hardlink). **This
session finished everything that was planned-not-built**, plus the canonical
compositions and all three "acknowledged deferrals":

- **Destructive guard + lockfile `materialization` field** (slice 3).
- **Hook pipeline-wiring + CLI consent** (slice 2).
- **`resolved_commit` population** (slice 1 foundation — also satisfies
  PROP-021 §2.4, whose acceptance was previously unmet).
- **`in-place` clone-path materialization** (slice 1) — the move-based one-copy
  design.
- **Hooks over in-place slots** (the canonical PROP-023 §2.3 bridge composition).
- **Hooks on scoped `vibe update`** (deferral #3 — a real PROP-020 §2.1 gap).
- **Incremental in-place update** (deferral #1 — `git fetch` the slot instead of
  re-clone on a version bump). Deferral #2 (token-env in-place) was never broken.

There is **no open blocker**; the feature is complete and green.

## Where work stands

- **Branch `main`**, tip `ac1f2f1`. **29 commits ahead of `origin/main`**,
  0 behind — local only, **not yet mirrored** (`cargo xtask mirror` not run;
  the owner has repeatedly reserved the mirror decision, so it was not done
  automatically). `origin/main` is at `5bdf35c` (the prior "mcp fix + `vibe
  self` rename" checkpoint).
- Working tree **clean** (before these two session-end doc commits).
- Floor **green**: `self-check.sh` exit 0 (fmt, all tests + doctests, clippy
  `-D warnings`, `vibe check` 0 errors / 1 pre-existing warning). Specmap clean:
  **597 units / 591 tagged / 604 edges / 0 suspects / 0 warnings / 0 orphans**,
  `--check` no drift.

## What landed this session (gate-green, 14 commits)

| Theme | feat commit | What |
|---|---|---|
| Slice 3 | `a9fad47` | `LockedPackage.materialization` (serde-default snapshot); pure `vibe-workspace::materialization::guard_destructive` (PROP-022 §2.6); wired into `uninstall`. |
| Slice 2 | `1423754` | Hook runner cell wired into `apply_resolution` (pre-install) + `vibe-install::apply` (post-install); `resolve_hook_policy` + `--allow-hooks` + consent in `vibe-cli`. |
| Slice 1 foundation | `e8c353a` | `GitBackend::head_commit` → `resolved_commit` populated in the per-package fetch (PROP-021 §2.4 / PROP-022 §2.5). |
| Slice 1 | `d554266` | `in-place` materialization: fetch skips cache-copy + tree-walk hash; `apply_resolution` **moves** the live clone (with `.git`) into the unversioned `.gitignore`d slot; boot/prune/uninstall/reinstall/update in-place-aware. |
| Composition | `60bae76` | Pre/post-install hooks run over an in-place slot (PROP-023 §2.3 canonical bridge). |
| Deferral #3 | `7a2ad0c` | Scoped `vibe update <pkg>` routed through the shared hook-bearing `materialise_subtree` (no prune / no boot). |
| Deferral #1 | `653cd49` | `registry::materialise_in_place` (direct slot clone / incremental `git fetch`) via the `InstallSource` seam; scoped update uses it for in-place-locked packages; the `content_dir == slot` "already-placed" signal makes the materialise pass skip the move but still run the hook. |

(Each `feat` is paired with a `chore(specmap)` regen — the repo convention.)

## Architecture decisions in force

- **Option B (registry-decoupling preserved).** `vibe-workspace` never touches
  git/URL/auth. The in-place clone is driven from the install/registry layer.
  The `apply_resolution` in-place path is a pure `fs::rename` (move clone →
  slot, copy-fallback cross-volume) — `.git` preserved, one physical copy on
  the common same-volume path.
- **Fresh = move, update = incremental.** A *fresh* in-place install clones to
  the cache `clone_dir` then moves it into the slot (one clone; the plan cannot
  know a package is in-place before fetching its manifest). A *version bump via
  `vibe update <pkg>`* reads the lockfile's `materialization` (slice 3), skips
  the re-fetch, and `git fetch`-es the existing slot incrementally via
  `materialise_in_place` (reusing `bootstrap_or_update_at` — **auth untouched**,
  so token-env private sources keep working).
- **`content_dir == slot` signal.** An incrementally-updated in-place dep is
  folded back as a `CachedPackage` whose `cache_dir` IS the slot; that equality
  tells `materialise_resolution` "already placed" → run the hook, skip the move.
- **In-place identity = `resolved_commit`** (§2.5); lockfile `content_hash` for
  in-place is a cheap `sha256(commit)`, never a tree walk. In-place slot is
  unversioned (`vibedeps/<kind>-<name>/`), `.gitignore`d (not vendored, §2.7),
  destruction-guarded (§2.6).

## Honest residuals (documented in the commits, intentionally scoped out)

- **General `vibe install` re-resolve of an in-place package re-clones** (move),
  not incremental — the canonical incremental path is **`vibe update <pkg>`**
  (it reads the lockfile materialization). Extending incrementality to the
  general plan needs a riskier plan/fetch restructure.
- **`reinstall --force` re-clones in-place** — by design (`--force` IS "re-fetch
  from source"); its own `--force` is the §2.6 opt-in.
- **Scoped `vibe update` runs no hooks for *non-installed* packages** — it only
  refreshes installed ones (unchanged pre-existing behaviour).

## Non-obvious findings (this session)

- **`cache_dir` is `.git`-stripped.** The live git clone with `.git` lives at
  the per-package registry `clone_dir`; `CachedPackage.cache_dir` is a
  `.git`-stripped copy. So the in-place "move" must take the clone (with `.git`),
  which is why the fetch hands `clone_dir` back as the content dir for in-place.
- **`resolved_commit` was always `None`** until this session — a documented
  git-backend gap. Populating it (via `head_commit`) unblocked in-place identity
  AND made PROP-021 §2.4's submodule-pin claim real.
- **Token-env in-place was never broken** — the move path re-clones through the
  auth-aware `bootstrap_or_update_at` every time. The prior deferral note was
  over-cautious.
- **`cargo check` skips `#[cfg(test)]` code** — a stale 3-arg `apply_resolution`
  call in tests slipped past `check` and only surfaced under `cargo test`. Use
  `cargo check --all-targets` when changing widely-called signatures.
- **Machine quirks (unchanged):** edit via Edit/Write, never PS `Set-Content`
  (UTF-8 corruption); `git commit` via `-F - <<'MSG'`; `self-check.sh` through
  Git Bash; mirrors via `cargo xtask mirror` (ff-only), never `git push origin`.
- **Em-dash gotcha for the Edit tool:** matching a line that contains `—`
  occasionally fails; anchor on ASCII-only neighbours.

## Repository map (unchanged shape; key in-place files noted)

```
vibevm/                      Rust workspace; binary = `vibe`; tooling = cargo xtask
├─ spec/modules/vibe-workspace/  PROP-020 (hooks), PROP-022 (materialization)
├─ spec/modules/vibe-registry/   PROP-021 (submodule), PROP-023 (bridge)
├─ spec/modules/vibe-mcp/PROP-015  + §2.8 #skill-include
├─ crates/
│   ├─ vibe-core/src/manifest/    package.rs (Materialization, bridge),
│   │     lockfile.rs (materialization field), package/hooks.rs (HooksDecl)
│   ├─ vibe-registry/src/
│   │     git_backend/{mod,shell}.rs  (head_commit)
│   │     git_package_registry/fetch.rs  (in-place fetch branch,
│   │         materialise_in_place, bootstrap_chain_into, commit_content_hash)
│   │     multi_registry_resolver/dispatch.rs  (materialise_in_place routing)
│   │     lib.rs  (InPlaceMaterialised, RegistryError::InPlaceUnsupported)
│   ├─ vibe-workspace/src/
│   │     materialization.rs   (destructive guard cell)
│   │     vibedeps.rs          (in_place_slot_*, materialise_in_place move,
│   │         ensure_gitignored, is_in_place_slot, remove_in_place_slot)
│   │     install.rs           (apply_resolution in-place branch, hooks,
│   │         materialise_subtree, run_post_install_hooks, already-placed signal)
│   │     hooks.rs             (runner cell, HookPolicy, decide_trust)
│   ├─ vibe-install/src/       lib.rs (InstallSource::materialise_in_place),
│   │     apply.rs (post-install), plan.rs, record.rs
│   └─ vibe-cli/src/commands/  install/ (resolve_hook_policy, resolver.rs seam),
│         update.rs (scoped incremental in-place), uninstall.rs, reinstall.rs
└─ specmap.json              traceability index (597 units / 604 edges)
```

## Recent commit chain (newest first)

```
ac1f2f1 chore(specmap): regen for the incremental in-place update edges
653cd49 feat(registry,cli): incremental in-place update on the slot
1d4cc3b chore(specmap): regen for the materialise_subtree edge
7a2ad0c feat(workspace,cli): run install hooks on scoped `vibe update`
e9c7198 chore(specmap): regen for the hooks-over-in-place edges
60bae76 feat(workspace): run install hooks over in-place slots
4fe08e2 chore(specmap): regen for the in-place materialization edges
d554266 feat(workspace,registry): in-place clone-path materialization
a4226c2 chore(specmap): regen for the head_commit verifies edge
e8c353a feat(registry): record resolved_commit from the fetched clone
66bccb7 chore(specmap): regen for the install-hook wiring edges
1423754 feat(workspace,cli): wire install hooks into the pipeline + consent
ff77a1f chore(specmap): regen for the destructive-guard edges
a9fad47 feat(workspace,core): in-place destructive guard + lockfile mode field
e885467 docs(continue): cold-resume — bridge packages mid-feature  (prior)
9b0976c docs(wal): checkpoint — bridge packages, specs + 6 impl slices  (prior)
```

## Quick-start

```sh
bash tools/self-check.sh                 # via Git Bash — check $?, currently green
cargo xtask specmap --check              # clean (597 units / 604 edges)
cargo test -p vibe-workspace             # the materialize + hook + in-place cells
cargo test -p vibe-registry              # head_commit + materialise_in_place
cargo xtask mirror                       # fan main+tags to both mirrors (NOT run yet)
```

The 29 session commits are **local**; mirror them with `cargo xtask mirror` when
the owner decides to. Session-resume phrase: `восстанови сессию`. The WAL
supersedes this snapshot wherever they diverge; the candidate next work
(below) is not a standing mandate.

## Candidate next steps (owner decides)

1. **Mirror** the 29 commits (`cargo xtask mirror`) — outward-facing, owner's call.
2. `/code-review` the bridge-packages diff (or `code-review ultra`).
3. Optional follow-ups: incremental in-place on the general `vibe install`
   re-resolve; a live end-to-end in-place acceptance smoke (PROP-022 §5) against
   a real giant repo.
