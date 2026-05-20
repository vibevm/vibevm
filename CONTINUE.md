# CONTINUE — cold-resume checkpoint

_Written: 2026-05-20 session-end. Owner-readable, self-contained. Pick this up with zero prior context._

---

## TL;DR (executive summary)

**This session did two things: closed the last M1.16 tech-debt item, then ran a large design session that locked the requirements for the biggest refactor proposed so far — workspaces + qualified naming.**

1. **`vibe registry redirect-update` shipped (2026-05-20).** Four commits (`f8af587..b44729d`) closed the one remaining M1.16 deferred-list item: a CLI command to rewrite an existing redirect stub's `vibe-redirect.toml` in place (retarget, switch policy, edit description), instead of the manual `git clone` / edit / push procedure. **The M1.16 deferred-list is now empty.** Pushed to `origin/main`.

2. **Workspace + qualified-naming design session (2026-05-20).** Two commits (`ff23a0f`, `4d6775a`) — **not yet pushed by the redirect-update push; this session-end pushes them**. A multi-fork design discussion with the owner produced: **PROP-007** (workspace — multi-package projects, cargo/Maven shape) and **PROP-008** (qualified naming — reverse-FQDN `group`, short aliases, collision detection), both in `DRAFT`; a new non-normative documentation genre **`spec/design/`**; and a full design-rationale record. **Implementation is deliberately deferred to a fresh session** — that is what the next session should pick up.

**Workspace state:** HEAD `4d6775a`, working tree clean (only `.claude/settings.local.json` untracked). **No active blockers.** One environment caveat — see "Non-obvious findings" #1 (Windows AV blocks the `vibe-install` test binary).

---

## Where work stands

- **Branch:** `main`. Working tree clean.
- **Origin:** after this session-end's push, `origin/main` == local HEAD. (Before the session-end docs commits, local was 2 ahead — `ff23a0f`, `4d6775a`.)
- **Active blocker:** none.
- **Owner sanction in force:** the owner granted (2026-05-20) explicit sanction to edit **any** specification, including the owner-frozen `VIBEVM-SPEC.md`, for the workspace + qualified-naming refactor. Recorded in the headers of PROP-007 and PROP-008.

### Outstanding manual step (owner-only, carried from 2026-05-12)

Delete `https://gitverse.ru/vibespecs/vibevm-direct-push-smoke` via the GitVerse web UI. GitVerse has no API DELETE endpoint vibevm could call. Not blocking anything.

---

## What to do first in the next session

**Implement M1.17 — Workspace.** The design is locked; the contract is [`spec/modules/vibe-workspace/PROP-007-workspace.md`](spec/modules/vibe-workspace/PROP-007-workspace.md). Recommended because PROP-007 has **no dependency on the index** and delivers the bulk of the owner's request on its own (multi-package projects, local cross-member deps, selective publish, "entirely local" / "entirely published" extremes).

**Cold-resume recipe:**

1. Read `CLAUDE.md` → `spec/boot/*` → `spec/WAL.md` (per the boot protocol).
2. Read [`spec/modules/vibe-workspace/PROP-007-workspace.md`](spec/modules/vibe-workspace/PROP-007-workspace.md) — the workspace contract.
3. Read [`spec/design/workspace-and-qualified-naming.md`](spec/design/workspace-and-qualified-naming.md) — the *why* and the full fork-by-fork lore of the design session. PROP-007's header links to it under `Design rationale:`. **Read this before implementing — it carries the reasoning a PROP cannot.**
4. PROP-007 §7 gives the phase plan; the `VIBEVM-SPEC.md` edits (§4.2 layout, §7.3–7.5 schemas) are authorised and land at implementation time.
5. PROP-008 ([`spec/modules/vibe-registry/PROP-008-qualified-naming.md`](spec/modules/vibe-registry/PROP-008-qualified-naming.md)) is the **next** milestone (M1.18) — it depends on PROP-005 (index) being implemented for short-name resolution. Do not start it before PROP-007.

**Alternative pickups** (if the owner redirects): tag `v0.1.0` (package-management surface is feature-complete — CHANGELOG `[Unreleased]` holds M1.12–M1.16 + redirect-update); implement PROP-005 (index, already fully spec'd, slices 1–11); M1.5 (LLM generation).

---

## Non-obvious findings from this session

### 1. Windows Defender blocks the `vibe-install` test binary (`os error 740`)

On this machine, `cargo test -p vibe-install` (and therefore `cargo test --workspace`) fails with:

```
could not execute process target\debug\deps\vibe_install-<hash>.exe (never executed)
Caused by: The requested operation requires elevation. (os error 740)
```

This is **not a code problem.** Windows Defender / Smart App Control blocks the freshly-compiled, unsigned test runner. `cargo clean -p vibe-install` and deleting the binary do not help — the same content-hash binary is recreated and re-blocked. The owner said he would resolve the AV side himself. A cold session on this machine must not mistake this for a regression — the `vibe-install` crate was not touched this session. Workaround for verification: `cargo build -p vibe-install --tests` type-checks cleanly; other crates' tests run fine.

### 2. `vibe registry redirect-update` — full apply path is not hermetically testable

`creator_for_url` dispatches only to GitHub / GitVerse; a hermetic mock host is out of scope for v0. The command is covered by 15 unit tests (`compute_updated_redirect_section` + helpers), 2 unit tests on the new `commit_and_push` helper, and 4 hermetic e2e tests on args-level guard rails (`--help`, `--description`/`--clear-description` mutual exclusion, bad pkgref, missing `vibe.toml`). End-to-end validation against a real host is left to a production smoke walk — not done this session.

### 3. `commit_and_push` — new vibe-publish helper

`vibe_publish::git_publish::commit_and_push(working_dir, clone_url, commit_msg)` stages, commits, fast-forward-pushes `main` on an existing clone. Refuses to record an empty commit if `git status --porcelain` is clean after `git add -A`. Symmetric to `push_initial` but for the "existing clone" path.

### 4. The workspace + qualified-naming refactor is the largest yet proposed

It requires editing the owner-frozen `VIBEVM-SPEC.md` (§4.2, §7.1, §7.3–7.5). The owner sanction for that is granted and recorded. PROP-007/008 are `DRAFT` — requirements locked, implementation pending. Do not treat the DRAFT status as "tentative"; the decisions are final, only the code is unwritten.

### 5. `spec/design/` — a new documentation genre was created

Non-normative design rationale for vibevm's own decisions. Distinct from `spec/research/` (external systems) and from the normative PROPs (the contract). See [`spec/design/README.md`](spec/design/README.md). The linking rule: a PROP with a rationale document links to it from its `Related` header, so a session reading the PROP during boot finds the lore.

---

## Repository map

```
vibevm/
├── CLAUDE.md / AGENTS.md / GEMINI.md   # Three identical copies of the four rules.
├── CONTINUE.md                          # This file. Cold-resume snapshot.
├── ROADMAP.md                           # Milestone plan; M1.17 + M1.18 added as DRAFT.
├── CHANGELOG.md                         # [Unreleased] holds M1.12–M1.16 + redirect-update.
├── VIBEVM-SPEC.md                       # Owner-frozen spec; edits now sanctioned for the refactor.
├── crates/
│   ├── vibe-cli/                        # `vibe` binary.
│   │   └── src/commands/registry.rs     # redirect / redirect-sync / redirect-update (M1.16 + this session).
│   ├── vibe-core/                       # Manifests + lockfile + redirect.toml schema.
│   ├── vibe-registry/                   # GitPackageRegistry + MultiRegistryResolver.
│   ├── vibe-resolver/                   # Depsolver + DepProvider adapters.
│   ├── vibe-publish/                    # RepoCreator + push helpers.
│   │   └── src/git_publish.rs           # push_initial / push_tag_only / shallow_clone /
│   │                                    # commit_and_push (new this session) / ls_remote_tags.
│   └── ...
├── spec/
│   ├── boot/{00-core,90-user}.md         # User-owned boot snippets.
│   ├── WAL.md                            # Living checkpoint — authoritative, supersedes this file.
│   ├── common/PROP-000…PROP-006
│   ├── modules/
│   │   ├── vibe-registry/PROP-001/002    # §2.4.2 redirect.
│   │   ├── vibe-registry/PROP-008-qualified-naming.md   # NEW — DRAFT (M1.18).
│   │   ├── vibe-resolver/PROP-003
│   │   ├── vibe-index/PROP-005
│   │   └── vibe-workspace/PROP-007-workspace.md         # NEW — DRAFT (M1.17).
│   ├── research/PROP-004
│   └── design/                           # NEW genre — non-normative design rationale.
│       ├── README.md
│       └── workspace-and-qualified-naming.md   # Full lore of the 2026-05-20 design session.
├── docs/
│   ├── commands/registry-redirect-update.md    # NEW — M1.16 closer reference.
│   ├── commands/registry-redirect{,-sync}.md
│   ├── registry-redirect.md
│   └── ...
├── manual-tests/                         # Runnable smoke protocols.
└── fixtures/registry/                    # Hermetic per-package registry fixtures.
```

---

## Architectural / policy decisions still in force

In rough order of how often they bite a fresh contributor:

1. **Four non-negotiable rules** ([PROP-000 §12](spec/common/PROP-000.md#commits)): no AI/machine-author attribution anywhere; Conventional Commits (subject ≤ 60 chars, body explains WHY); group commits by meaning; autonomy on routine changes only.
2. **Memory discipline.** Project facts live in the repo, not in per-machine user-memory.
3. **Vocabulary lock.** Only `flow`, `feat`, `stack`, `tool`. Never `lifecycle` / `phase` / `goal` / `plugin`.
4. **Language: Rust.** Permissive licenses only.
5. **Identity: `(kind, name, version, content_hash)`.** URL is informational. **Note:** PROP-008 (DRAFT) will change this to `(group, name, version, content_hash)` — `kind` leaves identity — at M1.18 implementation time. Not in force yet.
6. **Token secrecy** (PROP-000 §20). Never printed in any vibevm output.
7. **Repository hosts.** vibevm source = GitVerse. Package registry = GitHub `vibespecs` (primary) + GitVerse `vibespecs` (secondary).
8. **Test fixtures live in dedicated test orgs** (`vibespecstest1/2/3`). Canonical `vibespecs` reserves slots for real packages.
9. **User-owned files** vibevm never touches: `spec/boot/00-core.md`, `spec/boot/90-user.md`, `spec/WAL.md`, `VIBEVM-SPEC.md`, `refs/book/**`.
10. **Cargo-shape version syntax** (M1.13). Bare `0.3.0` = caret `^0.3.0`; `=0.3.0` for strict equal.
11. **`[requires]` is the source of truth for declared deps** (M1.12). `vibe.toml` = human input; `vibe.lock` = resolved materialisation.
12. **Per-registry `auth` axis** (M1.14, PROP-002 §2.2.1): `none` / `token-env` / `credential-helper` / `ssh`.
13. **`[requires.packages]` table-form** (M1.15): values are version-constraint strings OR git-source inline tables.
14. **Resolution priority** (M1.15): override > git-source > registry-walk. **Note:** PROP-007 (DRAFT) inserts `path` (workspace member) between override and git-source.
15. **Registry redirect via stub repo** (M1.16, PROP-002 §2.4.2). `vibe-redirect.toml` marker; hop limit 1. `vibe registry redirect` creates, `redirect-sync` mirrors tags, `redirect-update` (this session) rewrites the marker in place.
16. **`spec/design/` genre** (this session). Non-normative design rationale; the PROP is the contract, design-notes the lore.
17. **Owner sanction for `VIBEVM-SPEC.md` edits** (this session) — granted for the workspace + qualified-naming refactor.

### The DRAFT refactor — locked but unimplemented

PROP-007 (workspace) + PROP-008 (qualified naming) carry final, owner-approved requirements. Headline decisions: one unified `vibe.toml` per node (retires `vibe-package.toml`); `[workspace] members`; recursive nesting with one `vibe.lock` at the absolute root; `path`-source cross-member deps (dual-form `{ path, version }`); `[workspace.versions]` recursive placeholders; selective publish; mandatory reverse-FQDN `group` (`org.vibevm` for first-party packages); identity `(group, name, version, content_hash)`; `kind` demoted to metadata; pkgref `[kind:][group/]name[@version]`; `naming = "fqdn"` repos; index-backed short-name resolution; collision detection with new exit code `7`; lockfile schema v4. Full reasoning: [`spec/design/workspace-and-qualified-naming.md`](spec/design/workspace-and-qualified-naming.md).

---

## Recent commit chain (last 25, newest first)

```
4d6775a docs(spec): add spec/design genre + workspace/naming design rationale
ff23a0f docs(spec): draft PROP-007 + PROP-008 — workspace & qualified naming
b44729d docs(commands,registry-redirect,changelog): redirect-update reference
3553b2e test(vibe-cli): hermetic e2e for redirect-update args-level guard rails
cce61ac feat(vibe-cli): vibe registry redirect-update command
f8af587 feat(vibe-publish): commit_and_push helper for in-place stub updates
9740c10 docs(continue,wal): session-end checkpoint 2026-05-12
4e852f0 docs(registry-redirect,changelog,wal,continue): note test-org re-home
dbba8d7 test(cli,manual-tests): move live + smoke fixtures to test orgs
ad9b8b3 docs(continue,wal): M1.15 + M1.16 ship-complete checkpoint
9b22adb docs(commands,registry-redirect,manual-tests,changelog,roadmap): M1.15 + M1.16 ship reference
af1f320 test(vibe-cli): hermetic e2e for git-source repeats + redirect resolves
e10dda6 feat(vibe-cli): vibe registry redirect + redirect-sync commands
36a5847 feat(vibe-publish): publish helpers for stub creation + tag mirroring
dd87674 feat(vibe-registry,vibe-resolver): redirect-aware fetch_manifest
a1dc2b3 fix(vibe-registry): archive→clone fall-back in fetch_manifest_at_ref
5b9a2dc fix(vibe-cli/uninstall): drop git-source declarations on uninstall
3cf3b01 docs(continue): late-session checkpoint at 2026-05-10 (M1.15 + M1.16)
058ff41 docs(wal): M1.16 implementation checkpoint
c4b3f72 docs(registry-redirect,readme): operator reference for vibe-redirect.toml stubs
6e861ac feat(vibe-registry): MultiRegistryResolver follows vibe-redirect.toml stubs
b37e1b3 feat(vibe-core,vibe-registry,vibe-install): vibe-redirect.toml parser + via_redirect lockfile field
f9ce420 test(vibe-cli): e2e coverage for vibe install --git --tag and --branch
540f6c0 docs(continue): mid-session checkpoint at 2026-05-10 (M1.15 implementation)
5c3751c docs(wal): M1.15 implementation checkpoint
```

---

## Quick-start commands

```powershell
# Build everything.
cargo build --workspace

# Test gate (matches CI). NOTE: `cargo test -p vibe-install` may fail on this
# machine with `os error 740` — Windows AV blocking the test binary, NOT a code
# bug (see Non-obvious findings #1).
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p vibe-cli -- check --path . --quiet

# Install vibe into ~/.cargo/bin/.
cargo install --path crates/vibe-cli --locked

# Rewrite an existing redirect stub's marker (M1.16 closer, this session):
vibe registry redirect-update flow:internal-helper \
  --to https://forgejo.example/internal-helper \
  --trust-redirect --resync
```

---

## Pointer

`spec/WAL.md` is the canonical **living** checkpoint. If anything in this `CONTINUE.md` disagrees with the top of `spec/WAL.md`, trust the WAL — it gets bumped every session. For the workspace + qualified-naming refactor, the authoritative documents are PROP-007, PROP-008, and the design-rationale companion under `spec/design/`.
