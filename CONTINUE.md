# CONTINUE.md — cold-resume checkpoint

_Written 2026-06-27 (session save). Three pieces of owner-directed work landed
this session, **11 commits** (`fc1915b`→`bdde0f2`) on `main`, **local and NOT
mirrored**: (1) the **AI-Native TypeScript stack** at parity with Rust; (2) the
**card migration** to full Rust↔TS symmetry; (3) a **`vibe install` fix** so
in-repo `packages/` edits are picked up automatically (PROP-011 §2.6). Floor
green: `self-check.sh` exit 0, specmap 598/596/609/0._

> **`spec/WAL.md` is the canonical living state**; if this snapshot and the WAL
> disagree, the WAL wins. The **git log is the authoritative per-item record**.
> Boot first (`CLAUDE.md` → `spec/boot/INDEX.md` → its files → `spec/WAL.md`),
> then read this.

---

## TL;DR

1. **AI-Native TypeScript stack** (`fc1915b`→`9445ef0`, 4 commits). New
   `stack:org.vibevm/typescript-ai-native@0.2.0` at parity with the Rust stack:
   a GUIDE that is a strict **superset** of the Rust guide (15 sections, every
   Rust §0–12 mirrored + TS-specifics raised to top level), **nine TS cards +
   INDEX** at line-for-line depth parity, packaged + installed + wired into the
   project `vibe.toml` (boot now bilingual). The manifesto §8 package-map update
   (the language-agnostic "Discipline update") landed separately. EXCLUDED per
   owner: `vibe-tcg-ts` depth (carried as a conscious stub) and any
   checker-implementation/measurement (the forthcoming VibeVM TS code is the
   pilot; cards carry `specified` checkers).

2. **Card migration — β full symmetry** (`f6ab191`→`bb666d4`, 4 commits). The
   nine Rust cards + INDEX moved `flow-discipline-core/cards/` →
   `stack-rust-ai-native/cards/`, so the core is now purely language-neutral
   (manifesto, format, scaffold catalog, RAID, appendix) and **both** stacks own
   their `cards/`. Conform's REQ citations re-namespaced
   `discipline://core/cards/…` → `discipline://rust-ai-native/cards/…`.

3. **`vibe install` picks up in-repo `packages/` edits** (`97dd167`→`bdde0f2`,
   3 commits, PROP-011 §2.6). Editing the self-hosting `packages/` registry was
   not picked up — `vibe install` re-used the stale `vibedeps/` slot (a manual
   `rm -rf` was the workaround). Now an **in-workspace `file://`** source (under
   the workspace root, not `in-place`) is mutable: freshness returns `Stale`
   (re-resolve) and its slot is re-materialised. e2e-proven on real Windows
   paths.

There is **no open blocker**. The one pending decision is the owner's: **mirror**
the 11 commits (see below).

## Where work stands

- **Branch `main`**, tip `bdde0f2`. **11 commits ahead of `origin/main`**,
  **NOT pushed/mirrored** — held for the owner's word (publishing is
  outward-facing; asked twice this session, awaiting an explicit "mirror").
- Working tree **clean** (after this session-save's `CONTINUE.md` commit).
- Floor **green**: `bash tools/self-check.sh` exit 0 (fmt, all tests + doctests,
  clippy `-D warnings`, `vibe check` 0/0/0, `cargo xtask conform check` 0
  findings). Specmap clean: **598 units / 596 tagged / 609 edges / 0 suspects /
  0 warnings / 0 orphans**.

## The one pending decision (owner)

**Mirror the 11 commits to both source hosts** (the PROP-016 rollout):

```sh
cargo xtask mirror --check     # confirm GitVerse + GitHub are at 5f9688f (in sync)
cargo xtask mirror             # ff-only push of main to both, never --force
```

The mirrors were in sync at `5f9688f` at session start; the 11 new commits
fast-forward both. `git push origin main` hits **GitVerse only** — prefer
`cargo xtask mirror` (memory: it is the standard rollout).

## Architecture / policy decisions in force (long form)

- **The Discipline is four layers, not two.** An AI-Native language = **L1** T1
  core (`flow:org.vibevm/discipline-core`: manifesto, card FORMAT, scaffold
  CATALOG, RAID, appendix — language-neutral) + **L2** per-language GUIDE +
  tcg (the strong-author artifact, in `stack:org.vibevm/<lang>-ai-native`) +
  **L3** the per-language CARDS' Band-3 (`<stack>/cards/` — the weak-swarm
  RUNTIME surface delivered per edit) + **L4** implemented checkers + a pilot
  codebase. After the migration, **each stack owns its `cards/`** (L3) and the
  core is purely L1.
- **TS cards carry `specified` checkers, not implemented ones.** There is **no
  TS pilot codebase yet** — the forthcoming VibeVM TypeScript surface (UI +
  scripting, the second primary language) is the pilot, exactly the state the
  Rust cards were in before the terraform implemented their checkers. The
  standing open question (does scaffolding help *modification*, not just
  *generation* — C-7) is inherited by TS.
- **TS GUIDE = strict superset of Rust**, not a divergent doc: mirror Rust's
  section spine for a consistent way to write code, then raise the TS-specific
  levers (tsconfig-as-discipline, the erasure boundary + single-source runtime
  validation, branding over structural typing, the `unsafe` set, type-level
  testing) to the top level.
- **PROP-011 §2.6 — in-workspace `file://` is mutable.** A registry dep whose
  `source_url` is `file://` **under the workspace root** and **not `in-place`**
  is never version-immutable-fast-pathed (freshness `Stale`) nor
  presence-trusted (its slot re-materialised, gated by `ResolvedDep.source_mutable`).
  **External/static local registries + mirrors (`file://` outside the
  workspace) keep the §2.2/§2.3 fast path**; `in-place` (PROP-022) giants are
  excluded (they refresh through `vibe update`). The discriminator is
  `freshness::is_in_workspace_file_source` (own cell `freshness/source.rs`),
  component-wise + case-insensitive on Windows. The scope is in-workspace, **not
  all `file://`**, deliberately (the broad rule disabled the optimisation for
  legitimately-immutable static local registries — see findings).
- **Mirror, not `git push origin`** (PROP-016): both GitVerse
  (`anarchic/vibevm`) and GitHub (`anarchic-pro/vibevm`) are canonical for
  reading; `cargo xtask mirror` (ff-only, `mirrors.toml`) is the rollout.
- **`/code-review` is never to be suggested** to this owner (recorded in
  global memory this session). Offer manual review or plain git-review instead.

## Non-obvious findings (this session)

- **vibe treats a package version as immutable content — so editing a local
  registry in place is invisible to `vibe install`.** Both `vibe install` and
  `vibe update` short-circuit on a "fresh lock" (the dependency graph is
  unchanged), and PROP-011 §2.3 skips re-copying a *present* slot — neither
  re-hashes the local source. Forcing a fresh materialisation before the §2.6
  fix meant **removing the slot** (`rm -rf vibedeps/<slot>`) so it was absent.
  This is now fixed for the in-workspace case.
- **The fix's path logic is the fragile part — and it works on real Windows
  paths** (e2e-verified): `workspace.root` is canonicalised + `\\?\`-stripped by
  `Workspace::load`; the `file://` URL is decoded (drop the leading `/` before a
  `DRIVE:`) and compared component-wise, **case-insensitively on Windows** (the
  drive-letter case need not match). A `git+file://` URL does not match the
  `file://` prefix (it is a content-addressed git source).
- **A test breakage was the best design signal.** "All `file://` mutable" (the
  literal first cut) compiled and passed unit tests, but broke two *deliberate*
  §2.2/§2.3 fast-path CLI tests — they use a local `file://` fixture registry,
  and the broad rule disabled the optimisation for ALL local registries,
  including static fixtures/mirrors that are legitimately immutable. That
  breakage drove the in-workspace refinement (which the owner chose via a
  structured question).
- **`vibe install <single-pkgref>` does a SCOPED install** — it resolves only
  that pkgref's subtree and **prunes other `vibedeps/` slots**. Use a bare
  `vibe install` (no args) to re-materialise every `[requires].packages` entry.
- **The discipline's own gates caught my work.** Adding the §2.6 helper pushed
  `freshness.rs` (653) and `plan.rs` (603) over the 600-line budget → split the
  helper into its own cell `freshness/source.rs` and trimmed `plan.rs` (599).
  The new pub seam added a specmap unit → regenerated (`cargo xtask specmap`).
- **Machine quirks (unchanged):** edit via Edit/Write, never PS `Set-Content`
  (UTF-8 corruption); `git commit` via `-F - <<'MSG'` heredoc; `self-check.sh`
  through Git Bash. Recover an overwritten file from `git show HEAD:<path>`.

## Repository map

```
vibevm/                      Rust workspace; binary = `vibe`; tooling = cargo xtask
├─ CLAUDE.md / AGENTS.md / GEMINI.md   the four rules + boot directives (kept identical)
├─ MEMORY.md → spec/boot/90-user.md    user-owned boot snippet
├─ VIBEVM-SPEC.md            owner-frozen spec (do not edit without the owner)
├─ spec/
│   ├─ boot/                 00-core, 90-user (owned); INDEX.md (generated by `vibe`)
│   ├─ WAL.md                CANONICAL living state (this session's 3 sections at top)
│   ├─ common/               PROP-000.. (cross-cutting: registry, mirrors, modes…)
│   ├─ modules/              per-subsystem PROPs (vibe-workspace/PROP-011 §2.6 NEW)
│   ├─ discipline/           the 4 retained mechanism specs vibevm implements
│   └─ research/             DISCOVERY_PROMPT.md (the research-mode user prompt)
├─ packages/org.vibevm/      the in-repo authoring registry (`--registry packages`)
│   ├─ discipline-core/v0.2.0/      L1: manifesto, 01-format, 02-scaffolds, 03-raid,
│   │      appendix/, boot/10, legacy-projections/ (NO cards/ after the migration)
│   ├─ rust-ai-native/v0.2.0/       L2+L3: rust/GUIDE + tools/vibe-tcg, cards/ (9+INDEX),
│   │      boot/20  (cards migrated here this session)
│   └─ typescript-ai-native/v0.2.0/ NEW L2+L3: typescript/GUIDE + tools/vibe-tcg-ts(stub),
│          cards/ (9+INDEX), boot/20
├─ vibedeps/                 the materialised install (git-TRACKED): flow-discipline-core,
│      stack-rust-ai-native, stack-typescript-ai-native
├─ crates/
│   ├─ vibe-core/            manifest/lockfile types (LockedPackage, SourceKind, Materialization)
│   ├─ vibe-workspace/       Workspace; freshness.rs (§2.2/§2.6) + freshness/source.rs (NEW),
│   │      install.rs (materialise + the §2.3 skip, ResolvedDep.source_mutable NEW), vibedeps.rs
│   ├─ vibe-install/         plan.rs (builds ResolvedDep), apply.rs
│   ├─ vibe-registry/        git backends, CachedPackage (source_uri)
│   ├─ vibe-cli/             commands/{install,update,reinstall,…}; cli_pkg_cycle tests
│   ├─ conform-core/         the discipline's Class-F/G + length + unwrap gate (cites cards)
│   └─ … (vibe-resolver, vibe-mcp, specmap-core, specmark, …)
├─ xtask/                    cargo xtask {conform, specmap, mirror, fast-loop, …}
├─ tools/self-check.sh       the floor gate (5 steps incl. conform)
├─ mirrors.toml              the source-mirror targets (GitVerse + GitHub)
├─ vibe.toml / vibe.lock     project manifest (requires the 3 discipline packages) + lockfile
├─ vibevm.discipline.lock    the pilot reproducibility anchor (Rust pilot pins)
└─ specmap.json              traceability index (598 units / 609 edges)
```

## Recent commit chain (newest first)

```
bdde0f2 docs(wal): checkpoint — in-workspace file:// sources are mutable
ccc5b7a chore(specmap): regen for the §2.6 in-workspace-source helper
97dd167 fix(install): re-resolve in-workspace file:// sources on every install
bb666d4 docs(wal): checkpoint — full Rust↔TS card symmetry
1bddcfc build(deps): re-materialise vibedeps + relock for the card migration
688b349 refactor(conform): cite the cards in the rust-ai-native namespace
f6ab191 refactor(discipline): move the cards into the language stacks
9445ef0 docs(wal): checkpoint — AI-Native TypeScript stack at parity
221f3bb build(deps): install the TypeScript stack into the project
2632c52 feat(discipline): AI-Native TypeScript stack — guide, nine cards, tcg stub
fc1915b docs(discipline): add TypeScript to the package map (§8)
5f9688f docs(wal): checkpoint — general-install in-place + conform gate green  (mirror tip)
df737a1 docs(continue): cold-resume — incremental in-place + discipline sweep  (prior)
a68de7c chore(specmap): regen for the cell splits  (prior)
```

## Quick-start

```sh
bash tools/self-check.sh                 # the 5-step floor gate; check $?, currently green
cargo xtask specmap --check              # clean (598 units / 609 edges)
cargo test -p vibe-workspace             # freshness + install (incl. the §2.6 tests)
cargo run -p vibe-cli -- install --registry packages --assume-yes   # bilingual install; §2.6 fires
cargo xtask mirror --check               # confirm GitVerse + GitHub in sync (currently @ 5f9688f)
```

## Next-steps recipe (whoever picks up)

1. **Owner's call: mirror** the 11 commits (`cargo xtask mirror`) — the only
   pending step; held for explicit approval.
2. **TS pilot (future):** when VibeVM grows TS code (UI/scripting), implement
   the card checkers (`@typescript-eslint` rules, `tsd`/`expectTypeOf`,
   Twoslash, the `fast-check` harness) on it and validate the
   generation→modification transfer. That is L4 — deliberately deferred (no TS
   code yet).
3. **Optional symmetry follow-up:** none outstanding — the card migration
   already achieved full Rust↔TS symmetry; the core is language-neutral.
4. **`vibe-tcg-ts`:** bring the conscious stub to Rust-brief parity when the tcg
   line resumes (owner deferred it this session).

The WAL supersedes this snapshot wherever they diverge. Session-resume phrase:
`восстанови сессию`. The candidate next work above is not a standing mandate.
