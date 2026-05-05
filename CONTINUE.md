# CONTINUE — cold-resume checkpoint

_Written: 2026-05-05. Owner-readable, self-contained. Pick this up with zero prior context._

---

## TL;DR (executive summary)

**M1.7 vibe-mcp complete (non-LLM scope) + PROP-003 r2 fully landed end-to-end with omnibus integration proof.** Across the most recent push window: feature-aware install + subskill materialisation + conditional dependencies (cascading fixed-point loop) + BCP-47 i18n + lockfile schema v3 + `vibe show features|subskills|purls` + `vibe outdated` + `vibe-check` activation_conflict heuristic + three integration fixture packages + Model Context Protocol server (`vibe mcp serve` / `install` / `status`) with cache-precise `read_subskill` and on-demand `materialise_subskill`.

Workspace state at HEAD (`0566dbf`):

- **403 tests** across the workspace, all green.
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
- `tools/self-check.sh` green.

The most recent commit chain (newest first) covers M1.7 slice 3 — lazy-pull runtime — plus the slice 2 agent-config writers and slice 1 server itself:

```
0566dbf docs(wal): record M1.7 slice 3 landing — lazy-pull runtime closed
3c9e710 feat(vibe-mcp): cache-precise read_subskill + materialise_subskill tool
390fc3a feat(vibe-core,vibe-install): per-subskill files index + lazy-pull becomes truly lazy
37d0ceb docs(wal): record M1.7 slice 2 landing — vibe mcp install + status
98fec82 feat(vibe-cli): vibe mcp install — auto-detect agents + write MCP config
eaefab7 docs(wal): record M1.7 vibe-mcp slice 1 landing
416ac74 feat(vibe-cli): vibe mcp serve — wire Model Context Protocol over stdio
c2977fa feat(vibe-mcp): new crate — Model Context Protocol server (M1.7 slice 1)
```

Push to `gitverse.ru:anarchic/vibevm` is current.

---

## Where we are right now

- **Branch:** `main`. Working tree clean.
- **Latest checkpoint:** `0566dbf` (the M1.7 slice 3 WAL update).
- **Live registry:** `https://github.com/vibespecs` carries the original three v0.1.0 demo flows (`flow:wal`, `flow:sync-from-code`, `flow:atomic-commits`). Untouched in this session.
- **Integration fixtures:** `fixtures/registry/{flow/integration-alpha, flow/integration-beta, stack/integration-rust}` exercise every PROP-003 r2 surface in combination. Not yet published to vibespecs; ready when wanted.

---

## What landed in this session (chronological, newest first)

### M1.7 vibe-mcp — three slices

**Slice 3 — lazy-pull runtime (closes M2.8 dependency for read paths).** `LockedSubskill` schema v3 gains `files_written: Vec<PathBuf>` (project-relative paths a subskill specifically contributed; empty for lazy-pull) and `cache_files: Vec<PathBuf>` (subskill-root-relative paths inside the package cache; populated for every delivery mode). `vibe-install` no longer materialises `delivery=lazy-pull` subskills into the project tree; they live in the cache and surface only via MCP. `read_subskill` upgraded to be cache-precise — reads from project for eager/lazy-push, from cache for lazy-pull. New `materialise_subskill(package, subskill_path, force?)` tool promotes a lazy-pull subskill into the project tree on demand; refuses overwrite without `force=true` (preserves user edits, same discipline as `vibe update`'s `UserEditedFile` gate).

**Slice 2 — agent detection + MCP config writers (closes M1.11).** New `vibe mcp install [--agent claude|cursor|all] [--dry-run] [--force]` detects supported agents in the project tree (`.claude/` / `CLAUDE.md` for Claude Code, `.cursor/` / `.cursorrules` for Cursor), writes `mcpServers.vibevm` block into `.claude/settings.json` / `.cursor/mcp.json`. Idempotent on bytes (matching → unchanged, divergent → updated, missing → created). Foreign keys preserved on JSON merge. `vibe mcp status` is the read-only counterpart.

**Slice 1 — server itself.** New `vibe-mcp` crate. JSON-RPC 2.0 over stdio (line-delimited). `Server<T: Transport>` is transport-agnostic — production wires `StdioTransport`, tests use `MemoryTransport`. Two tools at slice 1: `query_package(name)` returns full lockfile entry, `read_subskill(package, subskill_path)` returns concatenated text. `vibe mcp serve --path <dir>` CLI command. `PROTOCOL_VERSION = "2024-11-05"`.

### PROP-003 r2 — six slices (complete end-to-end)

**Omnibus integration (`fixtures/registry/{integration-alpha,integration-beta,integration-rust}/`).** Three fixture packages exercising every PROP-003 r2 surface in combination — features, subskills, all four activation channels (manual / if_present / if_provides / if_files / if_command / if_env / if_describes_match / if_language), all three delivery modes, BCP-47 i18n, conditional deps, package-level + subskill-level `describes` PURLs. Six omnibus e2e tests verify the byte-level shape end-to-end. Two real integration bugs found and fixed in the process (`tailor_feature_request` per root + the same fix in the conditional-deps re-fetch path).

**Slice 4 — fixed-point conditional + activation-conflict heuristic.** Conditional-dep expansion promoted from single-pass to fixed-point loop with iteration cap = 5; cascading conditional deps now work. `vibe-check::ActivationConflict` runs Jaccard keyword-overlap (≥70% after stopword filtering) on subskill descriptions in the same package's lazy-push/lazy-pull set, mirroring Tessl's review-rubric "activation distinctiveness" axis without LLM dependency.

**Slice 3 — conditional dependencies + `vibe outdated`.** `[target."context(<key>)".dependencies]` schema in `vibe-core`; `vibe-resolver::conditional` parses + evaluates predicates (`Present(<key>)` form today, richer `if_files = '...'` and boolean composition flagged as `Unsupported`). `vibe-cli/install.rs` runs single-pass expansion (later promoted to fixed-point in slice 4). New `vibe outdated` registry-side update preview with JSON envelope.

**Slice 2 — feature-aware install + subskill materialisation.** `InstallOptions` extended with `feature_expansion`/`activation_context`/`describes`. Subskill discovery + activation + eager materialisation in `plan_install_with_options`. Two-phase install pipeline: fetch all → activation context → plan all. `--features` / `--no-default-features` / `--all-features` CLI. `register_installed_with_metadata` writes v3 lockfile fields. Three new `vibe show` subcommands (features / subskills / purls).

**Slice 1 — schema + activation evaluator.** `manifest::purl` (Package URL parser, npm-`@scope/name`-aware via `rsplit_once('@')`). `manifest::i18n` (BCP-47 sidecar pattern, fallback chain, `localised_path`, `resolve_localised`). `manifest::subskill` (`vibe-subskill.toml` schema with `[subskill]`/`[activation]`/`[recommends]`/`[conflicts]`/`[content]`, `DeliveryMode` enum, static validation). Lockfile schema bumped to v3. `vibe-resolver::features` + `vibe-resolver::activation` modules.

---

## Repository map

```
vibevm/                                     (this repo — gitverse.ru:anarchic/vibevm)
├── CLAUDE.md / AGENTS.md / GEMINI.md       ← byte-identical, the four rules + memory discipline
├── CONTINUE.md                             ← THIS FILE
├── DEV-GUIDE.md / RUNTIME-GUIDE.md         ← contributor / end-user setup
├── MEMORY.md                               ← pointer to spec/boot/90-user.md
├── ROADMAP.md / TASKS.md
├── VIBEVM-SPEC.md                          ← owner-frozen v1.0
├── tools/self-check.sh                     ← cargo test + clippy + vibe check, one entry point
├── tools/jtd-codegen/                      ← JTD codegen toolchain (binary not committed)
├── vibe.toml / vibe.lock                   ← bootstrap manifest so `vibe check` runs against vibevm itself
│
├── crates/                                 (Rust workspace — 13 crates, 3 placeholders)
│   ├── vibe-core/      ← manifest types, lockfile (schema v3), PURL, i18n, subskill, features, conditional
│   ├── vibe-cli/       ← `vibe` binary
│   ├── vibe-registry/  ← LocalRegistry + GitPackageRegistry + MultiRegistryResolver
│   ├── vibe-resolver/  ← NaiveDepSolver + features expansion + activation evaluator + conditional predicate
│   ├── vibe-install/   ← plan/apply/register install, subskill discovery + materialisation
│   ├── vibe-publish/   ← GitHub / GitVerse RepoCreator adapters, token redaction
│   ├── vibe-check/     ← spec linter (manifest_validity / wal_freshness / wal_wellformed /
│   │                     boot_directory / lockfile_files / review_aging / features_graph /
│   │                     subskill_structure / i18n_coverage / activation_conflict)
│   ├── vibe-mcp/       ← Model Context Protocol server (JSON-RPC over stdio, query_package /
│   │                     read_subskill / materialise_subskill tools)
│   ├── vibe-wire/      ← JTD-generated wire types (init_report fully migrated; rest hand-rolled)
│   ├── vibe-llm/       ← M0 placeholder (M1.5)
│   ├── vibe-graph/     ← M0 placeholder
│   └── xtask/          ← `cargo xtask codegen` / `check-codegen`
│
├── fixtures/registry/                      (LocalRegistry layout)
│   ├── flow/{wal, sync-from-code, atomic-commits}/v0.1.0/   ← legacy demo flows
│   ├── flow/{integration-alpha, integration-beta}/v0.1.0/   ← PROP-003 r2 omnibus
│   └── stack/integration-rust/v0.1.0/
│
├── docs/                                   ← user-facing reference per command
├── manual-tests/                           ← runnable smoke protocols
├── schemas/                                ← JTD wire-contract schemas
├── spec/
│   ├── boot/00-core.md … 90-user.md        ← session-boot foundation
│   ├── WAL.md                              ← canonical living state
│   ├── common/PROP-000.md
│   ├── modules/vibe-registry/PROP-001-...md / PROP-002-...md
│   ├── modules/vibe-resolver/PROP-003-dep-evolution.md   (r2)
│   └── research/PROP-004-tessl-comparative-research.md
│
├── refs/                                   (.gitignored — cargo / dnf / dnf5 study sources)
└── packages/                               (reserved for vibevm-using-vibevm dogfooding; empty)
```

---

## Quick-start commands

```bash
# Workspace health.
bash tools/self-check.sh

# Install the omnibus alpha + beta + stack from the in-tree fixture.
cargo run -p vibe-cli -- init --path /tmp/demo
cargo run -p vibe-cli -- install \
    stack:integration-rust flow:integration-alpha \
    --registry "$(pwd)/fixtures/registry" \
    --path /tmp/demo \
    --features extra-discipline \
    --language ru \
    --assume-yes

# Drive the MCP server manually.
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | \
  cargo run -p vibe-cli -- mcp serve --path /tmp/demo

# Wire the project's coding agent.
cargo run -p vibe-cli -- mcp install --path /tmp/demo --agent claude
```

---

## What's still open

By size and priority:

1. **M1.8 — `vibe review` static quality scoring.** New `vibe-eval` crate with three-axis scoring (validation / implementation / activation). Static portion only; LLM-judge mode lands in M2.7. Tessl-parity with their review rubric without the LLM dependency. Smallest immediate win — ~1 weekend.

2. **M2.10 — `vibe search` registry inspector.** Walks every configured `[[registry]]` URL, lists packages whose `vibe-package.toml` description matches a query. Naive at first; indexing later. Useful at 20+ packages, essential at 100+.

3. **M1.5 — LLM provider abstraction + `vibe build`.** The Big One. ROADMAP §M1.5.1–§M1.5.5. 3-6 weekends. Requires explicit owner sign-off per CLAUDE.md Rule 4 (non-routine, scope decisions about LLM provider, tool-use sandbox, cost reporting, etc.). Once `vibe-llm` is real, M2.7 (`--optimize` + multi-model A/B) and M2.9 (scenario generation from real commits) light up.

4. **libsolv FFI / `SatDepSolver`** (PROP-003 §2.1, Phase A). Standalone slice — adds the BSD-3-Clause libsolv dependency, Rust FFI bindings, rule encoding. NaiveDepSolver augmented with feature-awareness covers every current fixture, but fairs poorly on disjunction-rich graphs. ~2-3 weekends.

5. **M2.9 — scenario generation from real commits.** Depends on M1.5.1 + M1.8. Tessl's most architecturally distinctive primitive. Format pinned to be drop-in compatible with their `task.md`/`criteria.json`/`scenario.json` triple.

6. **`vibe update` feature-awareness.** Known gap: `vibe install foo --features X` followed by `vibe update foo` loses X. `plan_update_with_options` mirroring `plan_install_with_options` would fix it. ~1 weekend.

7. **vibe-mcp follow-ups.** Gemini / Codex / Copilot agent writers (need their public MCP-config conventions). `list_capabilities` / `query_capabilities` discovery tool. User-level config (`~/.config/claude/...`) for `vibe mcp install`.

8. **Documentation files for new commands.** `docs/commands/{features,subskills,purls,outdated,mcp-serve,mcp-install,mcp-status}.md`. Mostly mechanical translation of `--help` text into reference shape.

9. **Conditional-dep cleanup on uninstall** — orphan auto-remove when a trigger goes away. Conceptually similar to npm/cargo's auto-prune. Park.

10. **`vibe outdated --upstream`** PURL probe. Requires per-ecosystem HTTP clients (npm / pypi / cargo.io / crates.io). Tied to the M3.1 security threat-model question (what does "vulnerability" mean for spec content?).

---

## Standing rules / pointers

- **Read on session boot, in this order:** `CLAUDE.md`, every file in `spec/boot/` in filename order, `spec/WAL.md`, then any PROP under `spec/common/` or `spec/modules/` for the task at hand, then start work.
- **Four non-negotiable rules** (CLAUDE.md / [PROP-000 §12](spec/common/PROP-000.md#commits)): (1) human-only attribution, (2) Conventional Commits, (3) group commits by meaning, (4) autonomy on routine changes only.
- **Memory discipline:** project facts in repo, machine-local in user-memory.
- **Setup-docs obligation** ([PROP-000 §19](spec/common/PROP-000.md#setup-docs)): toolchain / prereqs / env / paths changes → `DEV-GUIDE.md` or `RUNTIME-GUIDE.md` in same commit.
- **Vocabulary lock:** `flow` / `feat` / `stack` / `tool`; never `lifecycle` / `phase` / `goal` / `plugin`.
- **Token secrecy** ([PROP-000 §20](spec/common/PROP-000.md#token-secrecy)): never display, never persist outside `~/.vibevm/`, never commit.
- **User-owned files** (`vibe install` / `uninstall` never modifies): `spec/boot/00-core.md`, `spec/boot/90-user.md`, `spec/WAL.md`, `VIBEVM-SPEC.md`, `refs/book/**`, any 00-09 or 90-99 boot file.

---

## If something has changed since this checkpoint

This file is frozen at 2026-05-05. Before acting on it:

- Re-read `spec/WAL.md` (it gets updated more often, and at session-end at the same time).
- `git log origin/main..HEAD --oneline` to see local-only work (should be empty after a clean session-end).
- `git status` to see the working tree.
- `bash tools/self-check.sh` to confirm the workspace is shippable.

If the WAL and this file disagree, **trust the WAL**.
