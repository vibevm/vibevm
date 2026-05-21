# CONTINUE — cold-resume checkpoint

_Written: 2026-05-21 checkpoint. Owner-readable, self-contained. Pick this up with zero prior context._

---

## TL;DR

**This session reopened the one question M1.17 deferred — and it became the design of the next milestone, M1.18.** PROP-007 §6 q3 ("when a dependency is resolved for member M, which member's `spec/` does its content land in?") proved to be a redesign of vibevm's whole **loading model**, not a directory choice. The session closed the **design + contract** phase:

- **[`spec/design/loading-and-boot-model.md`](spec/design/loading-and-boot-model.md)** — the non-normative rationale (the static/dynamic-linking metaphor, four principles, the fork-by-fork record).
- **[PROP-009](spec/modules/vibe-workspace/PROP-009-loading-model.md)** — the contract. DRAFT, but every §5 open question is resolved — **ready for M1.18 implementation**.

**Next session: implement M1.18.** See "What to do next" — it is a full-milestone effort, the scale of PROP-007's implementation, and must be done *additively* to keep the build green.

The branch is **`m1.17-workspace`**, local — M1.17 (shipped) plus this session's six docs commits. Not pushed, not merged: the owner's call.

The canonical living state is **[`spec/WAL.md`](spec/WAL.md)** — trust it over this file if they diverge.

## Where work stands

- **Branch:** `m1.17-workspace` — local only, not on origin. Carries M1.17 (the shipped workspace milestone, `b794e7a..3cb2a03`), the PROP-009 design + contract (`b48ba7f`, `1c1c19c`, `72ac624`), and this checkpoint.
- **`main`** is untouched at `8de20c2`.
- **Working tree:** clean (only `.claude/settings.local.json` untracked — pre-existing).
- **Gates:** the full workspace test suite is green this session (vibe-core 142, vibe-cli 124 + 111 + 11 + 15, vibe-registry 106 + 5 + 7, vibe-publish 51 + 5, vibe-resolver 48, vibe-workspace 31, vibe-check 25, vibe-mcp 22; vibe-install 18 — see Non-obvious finding #1). The PROP-009 work is docs-only — no code touched.
- **Active blocker:** none. M1.18 implementation can begin.

## What this session produced — the loading model

The flat boot model (`VIBEVM-SPEC.md` §6 — one `spec/boot/NN-*.md` directory, one entry point) does not survive a workspace, which has N nodes, N entry points, N boot sequences. PROP-009 replaces it. The owner's hard constraint: installing a dependency must never modify any node's authored spec.

**The model in one breath.** Two physically separate trees — authored `spec/` (only the author writes it) and a committed `vibedeps/` (only `vibe` writes it; one slot `vibedeps/<kind>-<name>/<version>/` per resolved package, the package's tree verbatim). The boot sequence is *computed* per node from the unified resolution — inherited foundation + own boot + dependency boot + overrides. `vibe install` generates, per entry-point node, `spec/boot/INLINE.md` (verbatim concatenation of `inline`-typed contributions, read first — the priority lane) and `spec/boot/INDEX.md` (a TOML manifest of `static` paths + `dynamic` INCLUDE pointers). Three inclusion types — `inline` / `static` / `dynamic` — set per dependency in `vibe.toml` (`link = …`, default `static`). The `NN-` prefix is retired; `vibe` orders by category. `[writes]` is retired. `vibe reinstall [<path>] [--force]` regenerates. One computed-view engine serves both boot and the effective spec. The model is uniform — a single-package project is a degenerate workspace.

The four design forks and the eight §5 contract questions were all resolved with the owner — the normative record is PROP-009 §5; the lore is the design doc §5.

## What to do next — implement M1.18

PROP-009 §7 lists eight phases. **Read PROP-009 §2 (decisions) and §7 (phase plan) first.** The load-bearing implementation note discovered this session:

> The §7 phases are a *logical* decomposition, **not** a sequence of independently-green commits. "Retire `[writes]`" (Phase 1) cannot land with a green build on its own — `vibe install` (`crates/vibe-install/src/lib.rs`) is built entirely on `manifest.writes.files`. **Implement additively:** add the new model (the `link` field, `[boot_snippet].category`, the `vibedeps/` materialisation, the computed-view engine, `INLINE.md` / `INDEX.md` generation) *beside* the old `[writes]` / `NN-` path; switch `vibe install` over; then delete the old path. This is the strategy PROP-007 Phase 3 used for path-source. The first realistic green slice is schema + materialisation + artifact generation together.

The computed-view engine likely extends `vibe-workspace` (it already owns `Workspace::discover` and the `[workspace.versions]` finalize pass) or lands as a new `vibe-boot` crate — PROP-009 §3 leaves this to implementation time.

**Gate before Phase 7:** the `VIBEVM-SPEC.md` edits (§6, §4.2, §4.6, §13.1) need explicit owner sanction — not yet granted. Phases 1–6 do not need it.

## Non-obvious findings

1. **`os error 740` is NOT Windows Defender — it is UAC installer detection.** `cargo test -p vibe-install` fails on this machine because Windows heuristically treats any unsigned, unmanifested executable whose name contains `install` (the test binary is `vibe_install-<hash>.exe`) as a legacy installer requiring elevation. **Proof:** the identical binary copied to a name without "install" runs all 18 tests cleanly. Disabling Windows Defender did not help — Defender was never the cause. Past checkpoints and PROP-007 §9.5 carried the wrong "Defender" diagnosis — corrected this session. Fix: `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System\EnableInstallerDetection = 0` (registry, admin, once), or run that crate's tests from an elevated shell. Linux / CI unaffected.
2. **The PROP-009 phase plan is entangled** — see "What to do next". Implement additively, or the build breaks between phases.
3. **vibevm dogfoods itself.** The vibevm repository is a vibevm project; PROP-009's migration (PROP-009 §4) changes how *this* repository boots — `spec/boot/00-core.md` and `90-user.md` stay user-owned; generated `INLINE.md` / `INDEX.md` join them.

## Backlog — parked behind PROP-009

PROP-007 §9.3's deferred items: **workspace-aware `vibe install`** in the old framing is *subsumed* — it is the install half of PROP-009. **`version = { workspace = true }`** member-version inheritance and the **publish-signalling polish** (`--archive`, `has_issues = false`, `published_repos`) are *parked* behind M1.18 — recorded, not dropped. **PROP-008** (qualified naming) is unchanged: it follows PROP-005 (index); the milestone numbering shifts — PROP-009 takes M1.18, PROP-008 moves to M1.19.

## Repository map

```
vibevm/
├── CLAUDE.md / AGENTS.md / GEMINI.md   # Three identical copies of the four rules.
├── CONTINUE.md                          # This file. Cold-resume snapshot.
├── ROADMAP.md                           # M1.17 shipped; M1.18 to be renumbered to PROP-009.
├── CHANGELOG.md                         # [Unreleased] holds the M1.17 milestone entry.
├── VIBEVM-SPEC.md                       # Owner-frozen spec; PROP-009 will edit §6/§4.2/§4.6/§13.1 under sanction.
├── vibe.lock / vibe.toml                # The repo's own manifest + lockfile (schema v4).
├── crates/
│   ├── vibe-core/                       # Manifest + lockfile schema. manifest/document.rs = unified
│   │   │                                #   `Manifest`; package.rs = package-role sections (BootSnippet,
│   │   │                                #   WritesSection, Requires); lockfile.rs = schema v4.
│   ├── vibe-workspace/                  # Workspace discovery + member model + publish staging.
│   │   │                                #   Likely host of the PROP-009 computed-view engine.
│   ├── vibe-registry/                   # MultiRegistryResolver — registry / git / path / override.
│   ├── vibe-resolver/                   # Depsolver + DepProvider adapters.
│   ├── vibe-publish/                    # RepoCreator + push helpers.
│   ├── vibe-install/                    # install/uninstall/update. Built on `manifest.writes` —
│   │   │                                #   PROP-009 Phase 1-5 reshapes this crate heavily.
│   ├── vibe-cli/                        # `vibe` binary. commands/*.rs.
│   └── ... (vibe-check, vibe-mcp, vibe-graph, vibe-llm, vibe-wire)
├── spec/
│   ├── boot/{00-core,90-user}.md         # User-owned. NOTE: 00-core.md line 38 is stale (see WAL).
│   ├── WAL.md                            # Living checkpoint — authoritative, supersedes this file.
│   ├── common/PROP-000…PROP-006
│   ├── modules/
│   │   ├── vibe-workspace/PROP-007-workspace.md   # The workspace contract + §9 implementation record.
│   │   ├── vibe-workspace/PROP-009-loading-model.md  # NEW — the loading-model contract (DRAFT, §5 resolved).
│   │   ├── vibe-registry/PROP-002, PROP-008
│   │   ├── vibe-resolver/PROP-003, vibe-index/PROP-005
│   ├── research/PROP-004
│   └── design/
│       ├── workspace-and-qualified-naming.md      # PROP-007/008 design lore.
│       └── loading-and-boot-model.md              # NEW — PROP-009 design lore.
├── docs/                                 # User guides.
├── manual-tests/                         # Runnable smoke protocols.
└── services/vibe-index/                  # Separate index service (PROP-005); not in the cargo workspace.
```

## Architectural / policy decisions in force

1. **Four non-negotiable rules** ([PROP-000 §12](spec/common/PROP-000.md#commits)): no AI/machine-author attribution anywhere; Conventional Commits (subject ≤ 60, body explains WHY); group commits by meaning; autonomy on routine changes only.
2. **Memory discipline.** Project facts live in the repo, not in per-machine user-memory.
3. **One unified manifest (M1.17).** Every node carries one `vibe.toml`; the role is the set of sections present. No `vibe-package.toml`.
4. **No legacy, by design.** vibevm is pre-release; removed manifest / lockfile forms are hard errors. PROP-009 continues this — its migration (§4) has no compatibility shim.
5. **The loading model (PROP-009, M1.18).** Authored `spec/` and a committed `vibedeps/` are separate trees; the boot sequence is computed per node; `INLINE.md` / `INDEX.md` are generated; inclusion types `inline` / `static` / `dynamic`; `[writes]` and `NN-` retired. See PROP-009.
6. **Vocabulary lock.** Only `flow`, `feat`, `stack`, `tool`. Never `lifecycle` / `phase` / `goal` / `plugin`.
7. **Language: Rust.** Permissive licenses only.
8. **Lockfile is schema v4** at the absolute workspace root; one per workspace.
9. **Token secrecy** ([PROP-000 §20](spec/common/PROP-000.md#token-secrecy)). Never printed in any vibevm output.
10. **Repository hosts.** vibevm source = GitVerse. Package registry = GitHub `vibespecs` (primary) + GitVerse `vibespecs` (secondary).
11. **User-owned files** vibevm never touches: `spec/boot/00-core.md`, `spec/boot/90-user.md`, `spec/WAL.md`, `VIBEVM-SPEC.md` (edited only under recorded owner sanction).
12. **Resolution priority:** `[[override]]` > path-source > git-source > registry-walk.
13. **Owner sanction for `VIBEVM-SPEC.md` edits** — granted for the workspace + qualified-naming refactor; **not yet granted for PROP-009** (needed at M1.18 Phase 7).

## Recent commit chain (newest first)

```
72ac624 docs(spec): resolve PROP-009 §5 open questions
1c1c19c docs(spec): draft PROP-009 — loading model contract
b48ba7f docs(spec): loading & boot model design rationale
ec3d614 docs(continue): cold-resume checkpoint — M1.17
17e6c1d docs(wal): session-end checkpoint — M1.17
0be50ef docs(spec): PROP-007 §9 — M1.17 implementation record
3cb2a03 docs(wal): M1.17 Phases 1-5 checkpoint
10406a1 docs: document the M1.17 workspace model
047f92d build: sync Cargo.lock — vibe-cli now depends on vibe-workspace
b673d2b feat(cli,workspace): vibe workspace publish
98795e8 feat(core,workspace): [workspace.versions] placeholders
e9a15d2 docs(spec): VIBEVM-SPEC §7.4 — lockfile v4
ff21de3 feat(core,registry): path-source deps + lockfile v4
ece30a6 feat(workspace): discovery and the member model
9a190ff docs(spec): VIBEVM-SPEC §7 — unified vibe.toml manifest
b794e7a feat(core): unify manifests into a single vibe.toml
8de20c2 docs(wal): session-end checkpoint 2026-05-20          <- main HEAD
```

Plus this checkpoint's three commits: the `os error 740` diagnosis correction, this WAL update, this `CONTINUE.md`.

## Quick-start commands

```powershell
# Build everything.
cargo build --workspace

# Test gate. NOTE: `cargo test -p vibe-install` fails with `os error 740` —
# Windows UAC installer detection (the binary name contains "install"), NOT a
# code bug (Non-obvious finding #1). Either run that crate's tests from an
# elevated shell, set EnableInstallerDetection=0, or test the rest:
cargo test --workspace --exclude vibe-install
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p vibe-cli -- check --path . --quiet

# The new loading-model spec.
#   spec/modules/vibe-workspace/PROP-009-loading-model.md   (the contract)
#   spec/design/loading-and-boot-model.md                   (the rationale)
```

## Pointer

[`spec/WAL.md`](spec/WAL.md) is the canonical **living** checkpoint. If anything here disagrees with the top of the WAL, trust the WAL. The contract for the next milestone is [PROP-009](spec/modules/vibe-workspace/PROP-009-loading-model.md); its rationale is [`spec/design/loading-and-boot-model.md`](spec/design/loading-and-boot-model.md); the workspace milestone it builds on is [PROP-007 §9](spec/modules/vibe-workspace/PROP-007-workspace.md#impl).
