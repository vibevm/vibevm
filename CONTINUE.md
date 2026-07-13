# CONTINUE.md — cold-resume checkpoint

_Written 2026-07-13, session close. This session did three linked things:
the **`org.vibevm` → `org.vibevm.ai-native` / `org.vibevm.world` group
restructure**, the **PROP-029 fully-qualified-address invariant** (so package
addresses can be refactored by an algorithm, not an LLM), and the **`wal`
name-collision kill** — the last of which was the **first real host-side
fractality delegation** (a cheap GLM worker did the ~40-edit test migration;
the boss reviewed and finished). Everything is committed and PUSHED to both
mirrors (`5fb38c5`); the tree is clean; self-check was green at close._

> **`spec/WAL.md` is the canonical living state**; if this snapshot and the
> WAL disagree, the WAL wins. The **git log is the authoritative per-item
> record.** Boot first (`CLAUDE.md` → `spec/boot/INLINE.md` if present →
> `spec/boot/INDEX.md` → its files → `spec/WAL.md`), then read this.

---

## TL;DR

vibevm is a spec-driven package manager (`vibe` CLI, Rust workspace under
`crates/`, packages under `packages/`). This session made package addresses
**mechanically refactorable** and removed the one thing that blocked that: a
duplicated package name. `org.vibevm` was a junk-drawer group; it split into
`org.vibevm.ai-native` (the discipline toolchain) and `org.vibevm.world`
(the redbook family + everything else). Every address was rewritten to its
full `(group, name)` coordinate under **one invariant — the group↔name joiner
is never `.`**: `/` where a path exists (pkgrefs, `spec://`), `_` where a flat
token is required (repo names). Then the `wal` fixture that duplicated the real
`org.vibevm.world/wal` package name was deleted and its tests migrated to
dogfood the real package — the migration delegated to a GLM worker, reviewed
and finished by the boss. All green, all pushed.

## Where work stands

- **Branch `main`**, tree clean, **local == origin == github @ `5fb38c5`**
  (`cargo xtask mirror` synced both — routine per Rule 4).
- **`self-check` all green (exit 0)** at close.
- **Two real top-level groups** now hold every package:
  - `packages/org.vibevm.ai-native/` — the discipline toolchain:
    `core-ai-native`, `rust-ai-native{,-lang,-mcp}`,
    `typescript-ai-native{,-lang,-mcp}`.
  - `packages/org.vibevm.world/` — everything else: the redbook family
    (`wal`, `redbook`, `sync-from-code`, `atomic-commits`, `addressable-specs`,
    … ~23 packages) + `wal-workspaces`.
  - `packages/org.vibevm.fractality/` — the fractality workspace (its own
    contract/WAL; a separate product incubated here).
  - Bare `org.vibevm` is now **fixture-only** — the test registry
    (`fixtures/registry/org.vibevm/…`) uses it; no real package does.
- **PROP-029 is in force** (`spec/common/PROP-029-fully-qualified-addresses.md`):
  every on-disk address is fully qualified; the joiner is never `.`.
- **The `wal` collision is fully killed** — see the four commits below.
  `name = "wal"` is now single-valued among real packages.

## Active blocker

**None.** Everything is complete, verified, and mirrored; the tree is clean.
The remaining items are owner-court (a remote-repo cleanup the boss must not
do itself, and cosmetics) — nothing is blocked or half-done.

## Open items (owner-court — none is a standing mandate)

1. **Delete the stale published trio** on `github.com/vibespecs` + GitVerse:
   `org.vibevm.wal`, `org.vibevm.sync-from-code`, `org.vibevm.atomic-commits`.
   The owner stated they have no users and are disposable; local packages moved
   to `org.vibevm.world`, so republish under the new group at public release.
   **Owner-side** (web UI / token) — the boss does not touch remote repos.
2. **Cosmetic — golden dir name.** `crates/vibe-index/fixtures/golden-flow-wal-0.1.0/`
   still carries "wal" as a *filesystem label*; the package identity inside is
   already `com.example/golden-pkg` (de-collided). Rename the dir for full
   de-wal if wanted: `git mv` + update two path refs (`content_hash_parity.rs`
   `.join(...)` and `manifest.rs` `include_bytes!(...)`).
3. **`VIBEVM-SPEC.md:939`** has one owner-frozen `org.vibevm.wal.git` occurrence
   in a naming example — owner's to update (it was left frozen; §11/§13 wal
   claims WERE updated this session under the owner's explicit un-freeze).
4. **Pre-existing product open items** (carried from prior campaigns; WAL has
   the detail): registry publish of the discipline families (rust **0.7.0** /
   ts **0.6.0** / core **0.7.0** / the two `-mcp`); a TS-STACK step in
   `tools/self-check.sh`; colon-free fact-store slot names (today
   `sha256:<hex>.json` lands as an NTFS alternate data stream);
   `vibe install --refresh` ergonomics; the `app` kind; Stage-B delivery
   experiments; vibe-mcp rebase onto mcp-core; PROP-025 v2 shims.

## Non-obvious findings (this session)

- **The joiner invariant is the whole game for mechanical refactoring.** A dot
  is ambiguous because groups are dotted reverse-DNS (`org.vibevm.world.wal`
  can't be split without a resolver). `/` (path surfaces) and `_` (flat repo
  names) are each a character in **neither** the group `[a-z0-9.-]` **nor** the
  name `[a-z0-9-]`, so an algorithm splits the coordinate deterministically.
  `spec://` needed no grammar change — resolution is full-URI-string exact
  match; the authority text only *constructs* unit URIs.
- **There are TWO fqdn repo-name renders**, and they must stay in lockstep:
  `crates/vibe-core/src/manifest/project.rs` (`NamingConvention::Fqdn`) and the
  parallel port in `crates/vibe-index/src/types/kinds.rs`. The underscore
  change had to touch both; a grep that only covers vibe-core misses vibe-index.
- **A duplicate package *name* forces an LLM back into the loop**, even when the
  full coordinates differ — the owner's reason for the wal-collision kill.
  Mechanical package ops need `name` to resolve to exactly one package; a
  fixture reusing a real name breaks that. Test fixtures that mimic real
  packages should use synthetic/reserved names (`com.example/…`) or dogfood the
  real package.
- **Dogfooding vs isolated fixtures — the owner's testing philosophy.** For a
  monorepo where packages and the package-manager co-evolve, tests SHOULD
  install the real product package; a test breaking on package evolution is
  *signal* (a real regression), and a stale fixture copy is *false coverage*.
  Caveat: assert on invariants (installs, lockfile coordinate, content-hash),
  not incidental version strings, so breakage means a real regression — and
  put the test registry OUTSIDE the project dir (an in-project registry, or a
  double `make_wal_dir_registry(project.path())`, re-copies files and defeats
  the re-install freshness / skip fast path — the two edge-case failures the
  boss fixed).
- **fractality delegated-run mechanics** (now in the CLAUDE.md ledger): a
  `worktree` worker gets its own **cold `target/`** → give it `cargo check`,
  not the full suite; **`max_turns` failures can be complete work** (review the
  worktree, don't discard); `show`/`ps` **usage tokens don't flush until
  terminal** (in=0/out=0 mid-run is not a stall — judge by
  `runs/<id>/worker-stdout.jsonl` growth + `git -C runs/<id>/wt status`); review
  path is `git -C runs/<id>/wt diff` → `git apply` into the host tree →
  self-check → boss commits; **workers don't `cargo fmt`** → run it before the
  fmt gate.

## Repository map (top level)

```
vibevm/
├─ CLAUDE.md / AGENTS.md / GEMINI.md   boot contract (byte-identical): Rules 1–4,
│                                       delegation-first + in-place fractality ledger
├─ VIBEVM-SPEC.md                       product spec (owner-frozen except the wal
│                                       edits made this session under explicit word)
├─ crates/                              the vibe product (Rust workspace)
│   ├─ vibe-core/      manifest + lockfile types, NamingConvention::Fqdn render
│   ├─ vibe-registry/  git + local registries, resolvers, vendor
│   ├─ vibe-index/     package index + scanner (parallel content_hash port;
│   │                  the golden hash-anchor fixture lives here)
│   ├─ vibe-resolver/ vibe-workspace/ vibe-install/ vibe-cli/ vibe-publish/ vibe-mcp/ …
├─ packages/
│   ├─ org.vibevm.ai-native/   discipline toolchain (conform/specmap/specmark;
│   │                          rust 0.7.0 / ts 0.6.0 / core 0.7.0; two MCP servers)
│   ├─ org.vibevm.world/        redbook family + wal + wal-workspaces + the rest
│   └─ org.vibevm.fractality/   the fractality workspace (own contract/WAL) +
│                               fractality.ps1 / .sh launchers
├─ fixtures/registry/org.vibevm/   the hermetic TEST registry (fixture-only group):
│                                   integration-*, pin-server, pin-stack, feat-pkg
│                                   (the wal/sync-from-code/atomic-commits fixtures
│                                    were deleted this session)
├─ spec/                            PROP/FEAT docs, spec/WAL.md, spec/boot/*
│   ├─ common/PROP-029-…            fully-qualified addresses + mechanical refactoring
│   └─ modules/vibe-registry/PROP-008  qualified naming (§2.5 repo-name render)
└─ tools/self-check.sh              the gate (fmt → clippy → test → conform → specmap)
```

## Standing decisions in force (long form)

- **PROP-029 — fully-qualified addresses.** Every on-disk address carries its
  full `(group, name)`; nothing stores a bare name (short names are CLI-only,
  resolved at the input boundary). The joiner is **never `.`** — `/` on path
  surfaces (pkgref, `spec://`), `_` on flat repo names. This is the precondition
  for a future deterministic rename engine (grep-zero the old coordinate). Test
  fixtures and `spec://demo/…` grammar examples are out of scope.
- **Global name-uniqueness for mechanical ops.** Real package names do not
  collide across groups where it can be helped, and test fixtures must not reuse
  a real package's name — otherwise a bare name is two-valued and only an LLM
  can disambiguate.
- **Delegation-first** (owner-commissioned): every substantial task first asks
  "can this go to fractality?" — cheap GLM workers carry the token-heavy grind;
  the boss keeps architecture, judgment, spec authoring, secrets, and the review
  of every delegated result. Never let the boss carry token-heavy execution.
  The calculus is `delegation-rules` (read in-place); the in-place run recipe +
  operating facts are the CLAUDE.md fractality ledger (owner-authorised to keep
  current autonomously).
- **License:** the shipped surface is **fully UPL-1.0**; the `"EULA"` strings
  that remain are all off-limits for relicensing (third-party `refs/**`,
  regenerated `vibedeps/**` + `.vibe/cache/**`, `fixtures/**` + `crates/**` test
  data, the `licensing` eula-template package, owner-frozen specs).
- **Rule 1 (attribution) is absolute:** the authored surface stays
  human-authored — no AI attribution anywhere (commits, trailers, branches,
  comments); workers are tools, never credited. Rules 2–4 (Conventional Commits,
  topic-grouped commits, autonomy-on-routine-only) unchanged.
- **Machine quirks (this box):** edits via editor tools only (PowerShell 5.1
  corrupts UTF-8-no-BOM round-trips); commits via `git commit -F - <<'MSG'`
  heredoc only; push via `cargo xtask mirror` (GitVerse origin + GitHub mirror,
  ff-only, SSH URLs). Never read/echo token files.

## Recent commit chain (this session, newest first)

```
5fb38c5 docs(delegation): record the first host delegated-run mechanics in the ledger
e3c95c8 test(vibe-cli): migrate the wal integration tests onto the real package
a17658b test(vibe-index): de-collide the golden hash-anchor from the wal name
596f706 docs(spec): align VIBEVM-SPEC wal references with the shipped package
e170884 test(fixtures): delete the dead wal-sibling fixtures that duplicated real names
12ad64c fix(docs): render org.vibevm.world/wal as org.vibevm.world_wal, not org.vibevm_wal
9aff183 refactor(registry): fqdn repo name joins group and name with _ (not .)
2b02996 refactor(spec): spec:// authority joins group and name with / (not .)
9e0ef52 fix(refactor): rustfmt the typescript discipline packages' Rust crates
4fdabb1 fix(rust-ai-native): external-spec namespace is the fully-qualified <group>.<name>
a42f5cc fix(refactor): repoint hardcoded discipline paths in scripts to the new group
4abc8fe fix(refactor): fmt all discipline members; fix stale group-dir prose
e529008 fix(refactor): reformat authored engines and re-sync vendored write-throughs
1a40097 fix(refactor): keep host test/fixture data bare; reformat discipline
d52bf02 refactor(spec): repoint every reference to the new package groups
6970828 refactor(packages): move the remaining packages to org.vibevm.world
788e67c refactor(packages): move the AI-Native discipline to org.vibevm.ai-native
2bad078 docs(spec): PROP-029 — fully-qualified addresses + mechanical refactoring
4ca473b docs(spec): record the fractality bug convention and full-UPL state
0a4d8aa docs(fractality): file E-BUG-001 and record the MT-05 relicense run
5086c5b chore(license): relicense vibevm to UPL-1.0
09920c5 chore(fractality): in-place launcher for the working-tree CLI
cadca12 docs(spec): delegation-first directive in the boot contract
```

(Before these: the redbook collection + fractality-ignition sessions — see the
WAL's dated section headers and the git log.)

## Quick-start (verify the tree)

```sh
bash tools/self-check.sh; echo "EXIT=$?"          # must be 0 (fmt→clippy→test→conform→specmap)

# Grep-zero the killed collision (expect no real package named wal outside org.vibevm.world):
git grep -nE 'name = "wal"' -- packages fixtures crates   # only com.example/golden-pkg + the real one

# Run vibe (working-tree build), e.g. install the real wal into a scratch project:
cargo build -p vibe-cli
# see crates/vibe-cli/tests/cli_pkg_cycle.rs for the dogfood pattern (make_wal_dir_registry)

# Drive a fractality delegation (paid — needs the owner's word):
#   cd packages/org.vibevm.fractality && ./fractality.ps1 spawn --packet <task.toml>
#   ./fractality.ps1 wait <id>   (bg → completion notification); review runs/<id>/wt diff
```

The WAL supersedes this snapshot wherever they diverge. Session-resume phrase:
`восстанови сессию` (boots into a status report and waits — the open items above
are the owner's call, not a standing mandate).
