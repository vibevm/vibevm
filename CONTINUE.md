# CONTINUE.md — cold-resume checkpoint

_Written 2026-07-13, session close. A very large session: **MCP repair** (the
discipline servers were down; root-caused, rebuilt, back up), the **fractality
delegation rules were tightened** (runs are no longer "paid"; every non-trivial
task now carries an out-loud delegation verdict; sessions announce their
harness), a **`debug>release` slot-binary resolver** landed, the
**persistent-worker gap was filed** (E-ENH-001), and — the headline —
**PROP-030 (the embedded registry) was implemented end to end, 5/5 slices,
each gate-green and pushed.** Everything is committed and PUSHED to both
mirrors (head `92e0668`); tree clean; self-check green at close._

> **`spec/WAL.md` is the canonical living state**; if this snapshot and the WAL
> disagree, the WAL wins. The **git log is the authoritative per-item record.**
> Boot first (`CLAUDE.md` → `spec/boot/INLINE.md` if present →
> `spec/boot/INDEX.md` → its files → `spec/WAL.md`), then read this.

---

## TL;DR

vibevm is a spec-driven package manager (`vibe` CLI, Rust workspace under
`crates/`, packages under `packages/`). This session:

1. **MCP repair.** `rust-ai-native` / `typescript-ai-native` discipline MCP
   servers were down — `.mcp.json` pointed at `vibedeps/…/target/release/*.exe`
   that a prior `vibe install` re-materialise had wiped. Rebuilt both release
   binaries, smoke-passed, regenerated `.mcp.json` (byte-identical). User
   reconnected via `/mcp`; **both servers are up** (their `mcp__*` tools live).
2. **Fractality grant (A) — `944528e`.** Ledger "Free vs paid" flipped: fractality
   runs are **pre-authorised, not paid** — do not ask before spawning. Rule 4 red
   lines + never-delegate set still bind. Byte-identical across CLAUDE/AGENTS/GEMINI.
3. **Persistent-worker gap (B) — `2aa6533`.** Investigated: fractality **cannot**
   keep a warm worker across tasks (one run == one pod == one one-shot
   `claude --print`). Filed `E-ENH-001` in the fractality specspace with cites +
   hook points.
4. **`debug>release` resolver (C) — `51c2d91`.** `DeclaredBinary::artifact()`
   (`crates/vibe-workspace/src/bins.rs`) now resolves the slot binary
   debug-first, release-fallback (was hard-coded release). Fixes the exact
   MCP-down failure mode for the future; affects `.mcp.json` generation +
   `vibe bin exec`.
5. **Delegation rules (`e7e4598`).** Two boot-contract additions (CLAUDE/AGENTS/
   GEMINI, identical): (a) **every non-trivial task must state its
   parallelization/delegation verdict out loud before executing** (native
   agent-spawn only under Claude Code; else fractality, fractality preferred);
   (b) **announce the harness in the first response of every session**, cached
   for the delegation analysis.
6. **PROP-030 — the embedded registry: COMPLETE (5/5).** See below.

## PROP-030 — embedded registry (the headline; DONE)

**Goal (owner directive):** a source-**installed** `vibe` resolves its own
in-tree `packages/` automatically for any project — no `--registry`, no
`[[registry]]`. Spec: `spec/modules/vibe-registry/PROP-030-embedded-registry.md`.

Five slices, each a separate commit, each full-self-check-green, all pushed:

| slice | commit | content |
|---|---|---|
| 1 | `097c200` | **discovery** — `commands/vvm/embedded.rs`: from the active VVM install (`origin=external` + `source_path` + `<sp>/packages`) derive the embedded root |
| 2 | `a06fa3d` | **`EmbeddedProvider`** — `vibe-resolver/src/embedded_provider.rs`: a combining `DepProvider`/`VersionEnumerator` cell; per-coordinate precedence (developer=embedded-first / distribution=embedded-last), `list_versions` unions, fetch serves precedence-first-that-has-it, absent falls through, real failure propagates |
| 3 | `3eb7f80` | **`InstallResolver::Embedded` + R-001 seam** — `resolver.rs` variant + `InstallSource`; `registry.rs` `ProviderResource::Embedded` + 3 `dep_solver` arms + `ProviderCell`; `build_install_resolver` composes + lifts the empty-`[[registry]]` bail; discovery threaded through `main.rs` into install/update/reinstall (§7) |
| 4 | `e5226af` | **reproducibility guard** — tag `CachedPackage.is_embedded` in the embedded fetch (→ `record.rs` writes `source_kind="embedded"`); CI-off (no embedded when `$CI` set); `vibe check` warns on `source_kind=embedded` lock entries |
| 5 | `92e0668` | **flags + doctor** — `--prefer-embedded`/`--no-prefer-embedded`/`--no-default-registry` (+`VIBE_NO_DEFAULT_REGISTRY`), mutually-excl validated; `vibe self doctor` reports the embedded registry |

**What now works:** source-installed `vibe install <pkg>` (no flags) auto-resolves
in-tree `packages/`; developer precedence embedded-first; the negation + suppress
flags + CI env behave; non-portable locks warn; a `cargo run`/test binary gets no
embedded registry (gated on `self_loc`).

## Where work stands

- **Branch `main`**, tree clean, **local == origin == github @ `92e0668`**
  (pushed with `git push origin main` + `git push github main`).
- **`self-check` all green (exit 0)** at close (ran per slice; 5× green).
- **MCP discipline servers up** (`mcp__rust-ai-native__*`,
  `mcp__typescript-ai-native__*` reconnected this session).

## Active blocker

**None.** The next major direction is a **VERY BIG REFACTORING** (owner-declared
at session close; scope TBD — the owner defines it next session). The three
PROP-030 follow-ups below are **deferred behind that refactoring** — do not start
them until the refactoring lands (or the owner re-prioritises).

## Backlog — deferred until AFTER the VERY BIG REFACTORING

1. **Fractality test-expansion (earmarked delegation).** The PROP-030 flag /
   composition logic (`build_install_resolver` branches: embedded-only lifts
   bail; embedded+declared; `--no-default-registry` suppresses; mutual-excl bail;
   precedence from `--no-prefer-embedded`) + the `InstallResolver::Embedded`
   `InstallSource` behaviour (resolve_and_fetch precedence, candidate_groups
   union, `is_embedded` tagging) are **compile-covered but have no dedicated unit
   tests**. `EmbeddedProvider`'s brain IS tested (slice 2). These tests must be
   **in-crate** (both types are `pub(crate)`). This is the clean fractality
   `big`-worker task — give it a precise packet (exact file, test cases, the
   `differential_oracle.rs` LocalRegistry fixture pattern, the `resolver_args()`
   InstallArgs template, acceptance `cargo test -p vibe-cli`).
2. **E2E verification (`/verify`).** `vibe self update` → `vibe install` in a
   throwaway project **with no `--registry`** → prove embedded actually resolves
   the in-tree `packages/`. The real proof beyond unit tests.
3. **Resolution-output naming.** "resolved `X` from the embedded registry" in the
   install pipeline's per-package emit (PROP-030 §6). Cosmetic; touches the
   install pipeline output.

## Next-steps recipe (for whoever picks up cold)

1. **Boot + resume-report only** (do NOT auto-execute): `восстанови сессию`.
2. The owner will define the **VERY BIG REFACTORING**. That is the next work.
3. The three items above wait behind it. When the owner clears item 1, the
   fractality packet is: add tests to `crates/vibe-cli/src/commands/install/
   resolver.rs` (unit `mod tests`) — build `InstallArgs` via a `resolver_args()`-
   style literal, a `Manifest` via `Manifest::parse_str`, an embedded
   `LocalRegistry` via the `seed_local_package` shape
   (`crates/vibe-resolver/tests/differential_oracle.rs:92`).

## Non-obvious findings (this session)

- **Embedded discovery must gate on the RUNNING install, not the `current`
  pointer.** PROP-030 §2 says "the record whose slot holds `current_exe`." First
  cut used `store.active()` at a `~/opt` fallback root, so a **test binary**
  (`current_exe` = `target/debug/deps/…`, not a VVM slot) picked up the
  developer's real `~/opt` install → every `vibe install` in the test suite
  silently resolved through the checkout's `packages/`, flipping git-source
  installs' `source_kind` to `registry` (4 red tests). Fix: gate on
  `self_loc.is_some()` (`derive_self(current_exe)`), so only a source-INSTALLED
  vibe discovers embedded; `cargo run`/tests get `None`. (`main.rs` closure.)
- **conform caught what a GLM worker would have missed** (validates keeping the
  core boss-side): slice 2 tripped `no-unwrap-in-domain` (two `.expect()` →
  `.unwrap_or_else` through the error enum) and `cell-has-oracle` (a `#[cell]`
  needs an **integration** test driving it, not just unit tests of its free
  functions — added `tests/embedded_provider.rs`).
- **`is_embedded` tagging was the slice-3 gap that slice 4 closed.** The embedded
  fetch returns a `LocalRegistry` `CachedPackage` with `is_embedded=false`; you
  must set `cached.is_embedded = true` in the embedded arm so `record.rs` writes
  `source_kind="embedded"` and the guard can key on it.
- **Adding a field to a clap `Args` struct breaks its literal constructors.**
  `InstallArgs` is built literally in `reinstall.rs::resolver_args()` and
  `update.rs::install_args_from()` — new flags must be added there too.
  `cargo check -p vibe-cli --all-targets` enumerates such sites (plain
  `cargo build` misses tests).
- **MCP-down root cause:** `vibe install` re-materialises `vibedeps/` from source
  but does **not** rebuild `target/release/*.exe`, so the `.mcp.json` release
  paths went missing while debug builds sat in `packages/…/target/debug/`. The
  `vibevm` product server survived because it's a PATH command (`vibe`), not a
  slot file.
- **`vibe mcp install` builds nothing** (`install.rs` "Nothing is built here") —
  regenerating `.mcp.json` with running servers is safe (no locked-exe rebuild).
- **fractality has no warm worker** (E-ENH-001): one-shot `claude --print` per
  run; `max_concurrent` is a slot limit, not a pool. So small per-task
  delegations pay a full cold spawn — prefer inline for small work until the gap
  closes.

## Repository map (top level)

```
vibevm/
├─ CLAUDE.md / AGENTS.md / GEMINI.md   boot contract (byte-identical): Rules 1–4,
│                                       Delegation-first (fractality-not-paid grant,
│                                       out-loud analysis + harness-announce rules)
├─ SPECSPACES.md                       specspace registry (`default: host`; fractality row)
├─ crates/                             the vibe product (Rust workspace)
│   ├─ vibe-core/      manifest+lockfile (SourceKind::Embedded, LockedPackage.source_kind)
│   ├─ vibe-registry/  registries+resolvers (CachedPackage.is_embedded; LocalRegistry M0)
│   ├─ vibe-resolver/  DepProvider/solver seam; **embedded_provider.rs** (slice 2)
│   ├─ vibe-install/   record.rs (is_embedded→source_kind=embedded), plan.rs
│   ├─ vibe-check/     `vibe check` cells; lockfile_files.rs (embedded warn, slice 4)
│   └─ vibe-cli/       install/{resolver.rs,mod.rs}, registry.rs (R-001 seam),
│                      cli/pkg.rs (flags), commands/vvm/{mod.rs,embedded.rs}, main.rs
├─ packages/
│   ├─ org.vibevm.world/       redbook family + wal + wal-specspaces + the rest
│   ├─ org.vibevm.ai-native/   discipline toolchain (rust 0.7 / ts 0.6 / core 0.7 + 2 MCP)
│   └─ org.vibevm.fractality/  the fractality specspace (own contract/WAL); plans/external/E-ENH-001
├─ spec/
│   ├─ modules/vibe-registry/PROP-030-embedded-registry.md   ← the shipped feature's spec
│   └─ WAL.md, boot/*
└─ tools/self-check.sh   the gate (fmt → test → clippy → vibe check → conform → sync-engines → specmap)
```

## Standing decisions in force

- **Fractality runs are pre-authorised, not paid** (grant A, 2026-07-13): don't
  ask before spawning; Rule 4 red lines + never-delegate set still bind.
- **Out-loud delegation analysis** for every non-trivial task; **announce the
  harness** in a session's first response (both in the boot contract now).
- **PROP-030 precedence is origin-selected:** today only `external` (developer)
  installs have an embedded registry → always embedded-first; `--no-prefer-embedded`
  flips it; distribution/embedded-last awaits the future bundle feature (§3.2).
- **Embedded discovery is gated on a source-INSTALLED vibe** (`self_loc`), never a
  `cargo run`/test binary; CI (`$CI`) and `--no-default-registry` /
  `VIBE_NO_DEFAULT_REGISTRY` suppress it.
- **Rule 1 (attribution) absolute** — human-authored surface; no AI attribution.
  Rules 2–4 unchanged.
- **Machine quirks (this box):** edits via editor tools (PS 5.1 corrupts
  UTF-8-no-BOM); commits via `git commit -F - <<'MSG'` heredoc; `self-check.sh`
  through Git Bash, check the real exit code; never read/echo token files.

## Recent commit chain (newest first)

```
92e0668 feat(cli): embedded-registry flags and the doctor line          (PROP-030 s5)
e5226af feat(cli): guard the embedded registry against non-portable locks (PROP-030 s4)
3eb7f80 feat(cli): resolve installs through the embedded registry        (PROP-030 s3)
a06fa3d feat(resolver): compose embedded + declared registries by precedence (PROP-030 s2)
097c200 feat(vvm): discover the embedded registry from the active install (PROP-030 s1)
e7e4598 docs(delegation): mandate out-loud delegation analysis + harness announce
51c2d91 feat(workspace): resolve slot binaries debug-first, else release
2aa6533 docs(fractality): file E-ENH-001 — persistent worker request
944528e docs(delegation): pre-authorise fractality runs (not paid)
fdb3b27 docs(wal): session-end checkpoint — specspaces, URLs, tool/app, PROP-030   (prior session)
92bb4f9 docs(continue): cold-resume checkpoint — specspaces, URL migration, PROP-030 (prior)
2528a68 feat(core): add the embedded source_kind (PROP-030 scaffold)               (prior)
```

## Quick-start (verify the tree)

```sh
bash tools/self-check.sh; echo "EXIT=$?"    # must be 0

git log --oneline -8                         # the PROP-030 chain above

# PROP-030 surface:
grep -n 'no_default_registry\|prefer_embedded' crates/vibe-cli/src/cli/pkg.rs
sed -n '/InstallResolver::Embedded/p' crates/vibe-cli/src/commands/install/resolver.rs
```

The WAL supersedes this snapshot wherever they diverge. Next session: **the owner
defines the VERY BIG REFACTORING** — the three PROP-030 follow-ups wait behind it.
