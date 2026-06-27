# CONTINUE.md — cold-resume checkpoint

_Written 2026-06-27 (continued). This session **extended incremental in-place to
the general `vibe install` re-resolve** (the last bridge-packages deferral) and
ran a **discipline sweep that turned the conform gate green and wired it into
`self-check.sh`** so it can never drift silently again. **11 commits**
(`60bf03b`→`a68de7c`) on `main`. Floor fully green: `self-check.sh` exit 0 — now
**5 steps** including conform 0/0. The 42 ahead commits are **MIRRORED to
GitVerse + GitHub** as of this checkpoint (`cargo xtask mirror`)._

> **`spec/WAL.md` is the canonical living state**; if this snapshot and the WAL
> disagree, the WAL wins. The **git log is the authoritative per-item record**.
> Boot first (`CLAUDE.md` → `spec/boot/INDEX.md` → its files → `spec/WAL.md`),
> then read this.

---

## TL;DR

Two things landed on top of the already-complete bridge-packages feature:

1. **Incremental in-place on the general install** (`feat 60bf03b`). The
   documented residual is closed: a full-pipeline `vibe install` re-resolve
   (explicit `<pkg>`, or a stale-lockfile bare install) used to **re-clone** an
   already-present `in-place` giant. Now the **plan defers** such a node (reads
   the existing slot's manifest locally, no re-clone, slot untouched — the
   read-mostly contract holds), and **apply** runs the incremental `git fetch`
   (`materialise_in_place`) post-confirmation, folding the fresh
   manifest/commit/hash back into the lockfile + resolution. This extends the
   canonical `vibe update <pkg>` path to the general install.

2. **Discipline sweep → conform gate green + wired into self-check.** The
   conform gate (`cargo xtask conform check`) was **silently red** — it is the
   Class-F/G + file-length + unwrap discipline checker, and it was never in
   `self-check.sh`, so it drifted across the bridge-packages sessions (green in
   the 2026-06-17 RAID, then 11 findings unnoticed). The sweep: doctested 3
   bridge-packages seams (Class G), **split all 7 over-budget files into
   module-grain cells** (≤600 lines), applied the no-unwrap `#[cfg(test)]`
   idiom, and **added conform as `self-check.sh`'s 5th invariant**. Gate now 0
   findings; it fails the same session a violation lands, not three later.

There is **no open blocker**. Floor green, gate green, and now mirrored.

## Where work stands

- **Branch `main`**, tip `a68de7c`. **42 commits ahead of `origin/main`** at
  session open; **now MIRRORED** to both source hosts (`cargo xtask mirror`,
  ff-only) — GitVerse `anarchic/vibevm` and GitHub `anarchic-pro/vibevm` both
  fast-forwarded from `5bdf35c` to `a68de7c`.
- Working tree **clean** (before these two session-end doc commits).
- Floor **green**: `bash tools/self-check.sh` exit 0 — **5 steps**: fmt, all
  tests + doctests, clippy `-D warnings`, `vibe check` 0/0/0, **`cargo xtask
  conform check` 0 findings**. Specmap clean: **597 units / 595 tagged / 608
  edges / 0 suspects / 0 warnings / 0 orphans**, `--check` no drift.

## What landed this session (11 commits, gate-green)

| Theme | commit | What |
|---|---|---|
| Incremental in-place | `60bf03b` | `feat(install)` plan defers an already-present in-place node (provisional `Fetched` from the slot, `in_place_incremental` flag); `apply` takes `source` + `materialise_deferred_in_place` runs the post-confirm incremental fetch. New mock-`InstallSource` test `tests/incremental_in_place.rs`. |
| — paired | `4ce2fd5` | `chore(specmap)` for the `PROP-022#in-place` edge on `try_in_place_incremental`. |
| Seam doc | `6fdd2e7` | `docs(install)` — `InstallSource::materialise_in_place` now names both consumers (update + general install). |
| Sweep G | `4c5d014` | `docs(workspace,registry)` — canonical doctests on `InPlaceMaterialised`, `InterpreterProbe`, `HookRunner` (the 3 `seam-has-doctest` findings). |
| — paired | `585911a` | `chore(specmap)` for the seam doctests. |
| File-length | `4cc37dd` | `refactor(core)` — `package.rs` (669→451): capability vocab → `package/capabilities.rs`. |
| File-length | `cc8e2a0` | `refactor(registry)` — `shell.rs` (603→542) → `shell/tar.rs`; `shell/tests.rs` (623→411) → `shell/tests_pure.rs`. |
| File-length | `040be26` | `refactor(mcp)` — `pkgskill.rs` (626→398): inline tests → `pkgskill/tests.rs`. |
| File-length | `172112c` | `refactor(workspace)` — `install.rs` (763→520) → `install/bootgen.rs`; `install/tests.rs` (884→344) → `tests_hooks.rs` + shared `test_helpers.rs`; `vibedeps.rs` (687→377) → `vibedeps/tests.rs`; + no-unwrap `#[cfg(test)]` fixes. |
| Process | `ab84fe7` | `build(self-check)` — conform is now the 5th `self-check.sh` invariant. |
| — paired | `a68de7c` | `chore(specmap)` for the cell splits. |

## Architecture decisions in force

- **Incremental in-place = defer in plan, fetch in apply.** Plan must stay
  read-mostly (a declined install must NOT advance the slot's commit), so it
  reads the *existing* slot's manifest (network-free) for a provisional
  `Fetched` with `cache_dir == slot` (the "already-placed" signal) and the
  `in_place_incremental` flag. The slot mutation (`git fetch` to the resolved
  ref) is deferred to `apply` (post-confirm), which then folds the fresh
  manifest/commit/hash into both `fetched` (→ lockfile, the resolved commit per
  §2.5) and `resolution` (→ boot + hooks). `apply` gained a `source: &S` param
  for this; the one call site (the CLI) already holds the resolver.
- **Honest residual (documented):** the provisional features / conditional-deps
  come from the *pre-fetch* slot manifest, so a mode/feature change in the
  bumped version is recorded one run late — the same staleness class scoped
  `vibe update` already carries, self-healing on the next resolve, and
  irrelevant for the giant repos in-place exists to serve. A fresh in-place
  install (no slot yet) and every snapshot/hardlink package are untouched.
- **File splits are behaviour-preserving cells.** Each over-budget file split
  along a responsibility seam; all public paths preserved via re-exports. Key
  gotcha: the boot-gen cell is named **`bootgen`**, not `boot`, to avoid
  shadowing `crate::boot`. Shared test scaffolding lives in `pub(super)` cells
  (`install/test_helpers.rs`), not duplicated. The capability vocab cell carries
  the same `specmark::scope!` its parent had, so spec-map edges are preserved.
- **conform is now a floor gate.** Wiring `cargo xtask conform check` into
  `self-check.sh` (last, so it reuses the build cache) is the structural fix for
  the silent drift — the baseline (0) was never grown to accommodate the
  bridge-packages findings (that would game the gate and violate its shrink-only
  rule); the findings were *fixed*.

## Non-obvious findings (this session)

- **`[lib] test = false` disables only the unit-test harness, NOT doctests.**
  vibe-install's doctests (the `InstallSource` canonical impl, `PlanObserver`)
  DO compile + run under `cargo test --doc`; the crate's `Cargo.toml` comment is
  about the UAC-named unit harness, not `--doc`. So Class-G doctests are live
  there, and `apply`'s canonical use is covered by the compiled integration test
  (the G card's "examples/ cell" allowance), not a bloated `no_run` doctest.
- **conform was red and nobody knew** — it is not in `self-check.sh`, so the
  bridge-packages code breached file-length + seam-doctest gates unnoticed.
  Always run `cargo xtask conform check` after a substantive change (now
  automatic via self-check).
- **Out-of-line `#[path] mod tests;` files are fact-extracted standalone** — the
  conform frontend doesn't see the parent's `#[cfg(test)]`, so a non-`#[test]`
  helper's `unwrap`s read as domain code. The idiom (documented in
  `install/tests.rs`): put `#[cfg(test)]` on each such helper. `#[test]` fns are
  fine (their own attribute marks them).
- **The file-length budget (600) counts every line incl. tests**; the only
  legitimate fix is structural cell-splitting. `conform freeze` is for a
  new-rule landing or after shrinking — growing the baseline to swallow new
  findings is gaming.
- **`cargo xtask mirror` is the rollout** (ff-only, both hosts, never
  `--force`); `git push origin` hits GitVerse only. `--check` reports drift
  without pushing.
- **Machine quirks (unchanged):** edit via Edit/Write, never PS `Set-Content`
  (UTF-8 corruption); `git commit` via `-F - <<'MSG'`; `self-check.sh` through
  Git Bash; for line-precise file surgery, byte-faithful `sed`/`head` via Git
  Bash is the right tool (Edit/Write can't truncate). Recover an
  overwritten-mid-split file from `git show HEAD:<path>`.

## Repository map (key new cells noted)

```
vibevm/                      Rust workspace; binary = `vibe`; tooling = cargo xtask
├─ spec/modules/vibe-workspace/  PROP-020 (hooks), PROP-022 (materialization)
├─ spec/modules/vibe-registry/   PROP-021 (submodule), PROP-023 (bridge)
├─ crates/
│   ├─ vibe-core/src/manifest/    package.rs (split) + package/capabilities.rs (NEW:
│   │     Provides/Requires/RequiresAny/Obsoletes/ConflictsList/ConditionalTarget)
│   ├─ vibe-registry/src/git_backend/
│   │     shell.rs (split) + shell/tar.rs (NEW: USTar extractor)
│   │     + shell/tests_pure.rs (NEW: pure-fn tests)
│   ├─ vibe-install/src/    plan.rs (try_in_place_incremental, fetch_or_defer),
│   │     apply.rs (apply<S>(source,…), materialise_deferred_in_place),
│   │     fetched.rs (in_place_incremental flag);
│   │     tests/incremental_in_place.rs (NEW: mock-InstallSource deferral proof)
│   ├─ vibe-workspace/src/  install.rs (split) + install/bootgen.rs (NEW: boot gen)
│   │     + install/tests_hooks.rs + install/test_helpers.rs (NEW: shared scaffolding)
│   │     vibedeps.rs (split) + vibedeps/tests.rs (NEW)
│   ├─ vibe-mcp/src/         pkgskill.rs (split) + pkgskill/tests.rs (NEW)
│   └─ vibe-cli/src/commands/install/mod.rs  (apply(&resolver, …))
├─ xtask/src/conform.rs      the Class-F/G + file-length + unwrap gate
├─ tools/self-check.sh       NOW 5 steps (conform is #5)
└─ specmap.json              traceability index (597 units / 608 edges)
```

## Recent commit chain (newest first)

```
a68de7c chore(specmap): regen for the cell splits
ab84fe7 build(self-check): run the conform discipline gate
172112c refactor(workspace): split install + vibedeps into cells
040be26 refactor(mcp): move pkgskill tests to an out-of-line cell
cc8e2a0 refactor(registry): split the shell backend into tar + test cells
4cc37dd refactor(core): extract the capability vocabulary into a cell
585911a chore(specmap): regen for the seam doctests
4c5d014 docs(workspace,registry): doctest the bridge-packages seams
6fdd2e7 docs(install): note the second materialise_in_place consumer
4ce2fd5 chore(specmap): regen for the general-install in-place edge
60bf03b feat(install): incremental in-place on general re-resolve
96d0893 docs(wal): session-end checkpoint — bridge packages complete  (prior)
91d5ea5 docs(continue): cold-resume — bridge packages complete  (prior)
```

## Quick-start

```sh
bash tools/self-check.sh                 # 5 steps incl. conform; check $?, currently green
cargo xtask conform check                # the discipline gate alone (0 findings)
cargo xtask specmap --check              # clean (597 units / 608 edges)
cargo test -p vibe-install --test incremental_in_place   # the general-install deferral proof
cargo xtask mirror --check               # confirm GitVerse + GitHub in sync (now @ a68de7c)
```

The WAL supersedes this snapshot wherever they diverge. Session-resume phrase:
`восстанови сессию`. The candidate next work below is not a standing mandate.

## Candidate next steps (owner decides)

1. `/code-review` (or `code-review ultra`) the incremental-in-place diff and the
   discipline-sweep splits, now that they are mirrored and reviewable.
2. Optional follow-up: a live giant-repo in-place acceptance smoke (PROP-022 §5)
   against a real big repo — still a manual test, not in CI.
3. Optional: extend the same drift-proofing to `cargo xtask specmap --check`
   (a 6th self-check step) so specmap can't drift silently either.
