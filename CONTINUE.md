# CONTINUE.md — cold-resume checkpoint

_Written 2026-07-13, session close. A long, multi-feature session: the vibevm
**source repo migrated to `vibevm/vibevm`** on both hosts; the **`wal-workspaces`
flow was renamed to `wal-specspaces`** ("specspace" everywhere) with a
**default-resume bug fix** and a **host dogfood** of the flow; **`tool`/`app`
boot categories** were added (unblocking the fractality install); **all repo
URLs** were repointed to the new org; **`vibe self update`** activated the
in-tree build; and **PROP-030 (the embedded registry)** was designed and its
lockfile scaffold landed. Everything is committed and PUSHED to both mirrors
(head `2528a68`); tree clean; self-check green at close._

> **`spec/WAL.md` is the canonical living state**; if this snapshot and the WAL
> disagree, the WAL wins. The **git log is the authoritative per-item record.**
> Boot first (`CLAUDE.md` → `spec/boot/INLINE.md` if present →
> `spec/boot/INDEX.md` → its files → `spec/WAL.md`), then read this.

---

## TL;DR

vibevm is a spec-driven package manager (`vibe` CLI, Rust workspace under
`crates/`, packages under `packages/`). This session did six linked things,
all committed + pushed to `vibevm/vibevm` on GitVerse **and** GitHub:

1. **Remote migration.** `origin`+`github` repointed to `git@gitverse.ru:vibevm/vibevm.git`
   and `git@github.com:vibevm/vibevm.git` (one org, `vibevm`, on both hosts).
2. **Specspaces.** The nested-project-WAL flow `wal-workspaces` → **`wal-specspaces`**;
   `WORKSPACES.md` → `SPECSPACES.md`; the term "workspace"→"specspace" on the
   live surface. Plus the **default-resume fix** and a **host dogfood**.
3. **`tool`/`app` boot categories** — the vibe `BootCategory` enum gained them,
   so the fractality manifest (`category = "tool"`) parses; fractality then
   materialised `wal-specspaces`.
4. **URL migration** — every `anarchic/vibevm` / `anarchic-pro/vibevm` → `vibevm/vibevm`
   (grep-zero), `mirrors.toml` now targets the two current repos; `vibespecs`
   (the package registry, a *separate* repo) deliberately untouched.
5. **`vibe self update`** — rebuilt the in-tree vibe and activated it (instance 9);
   the PATH shim `~/opt/bin/vibe` is now current.
6. **PROP-030 embedded registry** — DESIGN committed; lockfile SCAFFOLD landed;
   the resolver core is the **next session's job** (see below).

## Where work stands

- **Branch `main`**, tree clean, **local == origin == github @ `2528a68`**
  (pushed directly with `git push origin main` + `git push github main` — NOT
  `cargo xtask mirror`; see finding below).
- **`self-check` all green (exit 0)** at close.
- **vibe self-updated:** `vibe self current` → `branch:main #9`; the working-tree
  build is active on PATH.

## Active blocker

**None.** The one thing to know before working: the **AI-Native discipline MCP
servers were killed this session** (`rust-ai-native-mcp.exe`,
`typescript-ai-native-mcp.exe`) to let `vibe install` re-materialise `vibedeps/`
(they were holding those files). Their tools (`mcp__rust-ai-native__*`,
`mcp__typescript-ai-native__*`) are **disconnected** — **restart Claude Code**
to restore them. Nothing is broken; they just need respawning.

## Next session — the recipe: finish PROP-030 (the embedded registry)

**Goal (owner directive):** when `vibe` is installed from a source tree, the
in-tree `packages/` resolves automatically for any project — no `--registry`,
no `[[registry]]` edit. Full design: `spec/modules/vibe-registry/PROP-030-embedded-registry.md`.
The lockfile scaffold is already in (`2528a68`): `SourceKind::Embedded`
(`crates/vibe-core/src/manifest/lockfile.rs`), `CachedPackage.is_embedded`
(`crates/vibe-registry/src/lib.rs`), and the tagging branch in
`crates/vibe-install/src/record.rs`. It is inert until the resolver wires it.

**Remaining (the resolver core — build in verified slices, self-check each):**

1. **`EmbeddedProvider`** in `crates/vibe-resolver/` (beside
   `local_registry_provider.rs` / `multi_registry_provider.rs`): a `DepProvider`
   + `VersionEnumerator` that wraps an embedded `LocalRegistryProvider` + an
   optional `MultiRegistryProvider`, delegating **per-coordinate** with
   embedded-first (developer) or embedded-last (distribution). It goes through
   the R-001 seam (`#[cell(seam="DepProvider", …)]`) → **specmap registration +
   conform** apply (this is the discipline-machinery layer that makes this
   bigger than a plain 4-crate change).
2. **`ProviderResource::Embedded` + `InstallResolver::Embedded`**
   (`crates/vibe-cli/src/registry.rs` + `commands/install/resolver.rs`) + the
   `dep_solver` cases; implement `InstallSource` (resolve_and_fetch sets
   `is_embedded = true`; solve via the combining provider; materialise_in_place
   errors — a `LocalRegistry` has no git backend; candidate_groups unions both).
3. **Discovery** in `build_install_resolver`: read the active VVM install via
   the `VersionStore` (`crates/vibe-cli/src/commands/vvm/store.rs`, `store.active()`
   → `source_path` + `origin`); if `origin = "external"` and `<source_path>/packages`
   exists, compose the embedded registry at the origin-selected precedence; lift
   the empty-`[[registry]]` bail.
4. **Guard + control:** `--frozen`/CI-off (drop embedded); `vibe check` warning on
   `source_kind=embedded` lock entries; flags `--prefer-embedded` /
   `--no-default-registry` (+ `VIBE_NO_DEFAULT_REGISTRY`); `vibe doctor` line.
5. **Tests** across the combining provider + discovery; `bash tools/self-check.sh`
   = 0 (mind conform/specmap for the new cell).

Task **#11** in the task list carries this same checklist.

## Non-obvious findings (this session)

- **The specspace default-resume fix is the whole point of the rename.** A bare
  `восстанови сессию` at the host root used to sometimes resume a specspace
  (e.g. `fractality`) instead of the host WAL. Now: an explicit name/dir always
  wins; a **bare** phrase takes `SPECSPACES.md`'s `default:` (set to `host`
  here) else the host — never a specspace by accident. Canon lives in the
  installed `flow:org.vibevm.world/wal-specspaces` boot snippet (slot 11) +
  `SPECSPACES-PROTOCOL.md`; the host `CLAUDE.md §Specspaces` is now a signpost.
- **`embedded` ≠ `local` (reserved).** PROP-030 names vibevm's own in-tree
  packages **embedded** (`--prefer-embedded`, `source_kind=embedded`);
  `--prefer-local` is **reserved** for a future user-own-repositories feature.
  Precedence is **origin-selected**: developer (source install, `origin=external`)
  → embedded wins clashes; end-user distribution → embedded is a fall-through.
- **`vibe install` has no incremental mode** — it re-materialises the whole
  closure, so it collides with any process holding `vibedeps/` (hence the
  MCP-server kill). Candidate ergonomics fix / backlog.
- **`vibe self update`** = "rebuild and activate the latest in-tree version"
  (`self install latest`); writes `~/opt/vibevm/versions/branch/<id>/<n>/` and
  repoints the PATH shim. The install ledger `~/opt/vibevm/state.toml` records
  each install's `origin` (`external` = built from a source tree) + `source_path`
  — the hook PROP-030's discovery uses.
- **External projects consume local packages by path:** `vibe install <pkgref>
  --registry C:\Users\olegc\gits\vibevm\packages` (M0 local-directory mode;
  overrides `[[registry]]`; no network). No persistent local-path config today —
  PROP-030 is exactly the fix for that friction.
- **`cargo xtask mirror` is mis-targeted post-migration? No — fixed.** `mirrors.toml`
  now points at `vibevm/vibevm` on both hosts, so `cargo xtask mirror` would work;
  but this session pushed with plain `git push origin main` + `git push github main`
  (both remotes already repointed). Either is fine now.
- **`sed` on ASCII-safe substitutions is byte-preserving** even in files with
  Cyrillic/em-dashes (the pattern/replacement are ASCII; other bytes pass
  through). The machine-quirk "editor-tools-only" rule is specifically about
  PowerShell 5.1 Get/Set-Content, not GNU sed. Used it for the URL sweep +
  verified every diff. **But** a too-broad `sed` anchor (`is_path_source:`) once
  hit sibling structs (`MultiResolution`) — anchor precisely + build to verify.
- **Adding a struct field spreads to test/in-place sites `cargo build` misses;**
  `cargo test --workspace --no-run` is what enumerates them (found `update.rs` +
  `plan.rs` beyond the 7 the lib build showed).

## Repository map (top level)

```
vibevm/
├─ CLAUDE.md / AGENTS.md / GEMINI.md   boot contract (byte-identical): Rules 1–4,
│                                       delegation-first ledger, §Specspaces signpost
├─ SPECSPACES.md                       the specspace registry (was WORKSPACES.md);
│                                       `default: host` + the fractality row
├─ VIBEVM-SPEC.md                      product spec (owner-frozen)
├─ mirrors.toml                        `cargo xtask mirror` targets → vibevm/vibevm ×2
├─ crates/                             the vibe product (Rust workspace)
│   ├─ vibe-core/      manifest+lockfile types (SourceKind::Embedded, BootCategory Tool/App)
│   ├─ vibe-registry/  registries+resolvers (CachedPackage.is_embedded; LocalRegistry M0)
│   ├─ vibe-resolver/  DepProvider/solver seam (EmbeddedProvider goes here next)
│   ├─ vibe-install/   record.rs (source_kind tagging), plan.rs (in-place)
│   ├─ vibe-cli/       install/resolver.rs, registry.rs (R-001 seam), commands/vvm (self-update)
│   └─ vibe-workspace/ boot.rs (BootBand; band_for)
├─ packages/
│   ├─ org.vibevm.world/   redbook family + wal + **wal-specspaces** (renamed) + the rest
│   ├─ org.vibevm.ai-native/  discipline toolchain (rust 0.7.0 / ts 0.6.0 / core 0.7.0 + 2 MCP)
│   └─ org.vibevm.fractality/ the fractality specspace (own contract/WAL); now dogfoods wal-specspaces
├─ spec/
│   ├─ modules/vibe-registry/PROP-030-embedded-registry.md   ← next session's spec
│   ├─ common/PROP-029-… (fully-qualified addresses), PROP-016 (source mirrors)
│   └─ WAL.md, boot/*
└─ tools/self-check.sh   the gate (fmt → test → clippy → vibe check → conform → sync-engines → specmap)
```

## Standing decisions in force

- **Specspace terminology.** The nested-project-WAL concept is **specspace**
  (`flow:org.vibevm.world/wal-specspaces`, `SPECSPACES.md`). The vibe
  `[workspace]` manifest role, the `vibe-workspace` crate, Cargo `[workspace]`,
  and the fractality packet `[workspace] mode` are a **different sense** and stay
  "workspace". A bare session phrase → host, never a specspace by accident.
- **PROP-030 developer↔user precedence.** Embedded (in-tree) packages: **first**
  for a source-installed developer (win clashes — vibevm-on-vibevm), **last** for
  a future distribution's end user. `--prefer-embedded`; `--prefer-local`
  reserved.
- **Repo home.** vibevm source = `vibevm/vibevm` on GitVerse **and** GitHub (one
  org). Package registry = the separate `vibespecs` GitHub org (untouched).
  Old repos (`anarchic/vibevm`, `anarchic-pro/vibevm`, and the stray
  `olegchir/vibevm`) are the **owner's to delete manually**.
- **Delegation-first** (owner-commissioned): spend Claude on architecture,
  planning, judgment, review; delegate execution to fractality/GLM where
  verification is cheaper than generation. This session kept the sensitive
  resolver core boss-side and *checkpointed rather than rushed* it — aligned with
  the production-grade quality bar.
- **Rule 1 (attribution) absolute:** the authored surface stays human-authored —
  no AI attribution in commits, trailers, branches, comments. Rules 2–4 unchanged.
- **Machine quirks (this box):** edits via editor tools (PS 5.1 corrupts
  UTF-8-no-BOM round-trips) — *but* GNU `sed` with ASCII patterns is byte-safe;
  commits via `git commit -F - <<'MSG'` heredoc; `self-check.sh` through Git Bash,
  check the real exit code; never read/echo token files.

## Recent commit chain (this session, newest first)

```
2528a68 feat(core): add the embedded source_kind (PROP-030 scaffold)
e3e74f9 docs(spec): PROP-030 — the embedded registry
350cd8c chore(repo): point all source URLs at the vibevm/vibevm repos
53fc15b build(fractality): materialise wal-specspaces now that tool parses
3e020b0 feat(core): accept tool and app boot_snippet categories
f0748c2 refactor(fractality): adopt the specspace term in the contract
43401cf build(specspaces): dogfood wal-specspaces into the host boot
b59aba8 refactor(specspaces): rename the wal-workspaces flow to wal-specspaces
8549943 docs(wal): session-end checkpoint — restructure, PROP-029, wal collision kill
b6756e6 docs(continue): cold-resume checkpoint — restructure + wal collision kill
5fb38c5 docs(delegation): record the first host delegated-run mechanics in the ledger
```

## Quick-start (verify the tree)

```sh
bash tools/self-check.sh; echo "EXIT=$?"           # must be 0

vibe self current                                   # → branch:main #9 (self-updated)

# The specspace default-resume: a bare `восстанови сессию` at the host root
# resolves to the HOST WAL (SPECSPACES.md `default: host`); name a specspace
# (`восстанови сессию fractality`) to target it.

# Grep-zero the migrated URLs (expect nothing but refs/ third-party):
git grep -n 'anarchic/vibevm\|anarchic-pro' -- . ':(exclude)refs'

# PROP-030 next: cargo build -p vibe-cli ; then wire EmbeddedProvider (see recipe).
```

The WAL supersedes this snapshot wherever they diverge. To pick up PROP-030:
read `spec/modules/vibe-registry/PROP-030-embedded-registry.md`, then task #11 /
the recipe above. Restart Claude Code first to restore the discipline MCP servers.
