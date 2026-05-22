# vibevm — roadmap

> **Status snapshot (2026-04-29):** M0 is complete. M1.1 (git-backed
> registry) shipped 2026-04-22 against a monorepo-shaped registry at
> `git@gitverse.ru:anarchic/vibespecs.git`. The M1.5-gate **content**
> slice shipped on 2026-04-23: three demo flows live there —
> `flow:wal@0.1.0`, `flow:sync-from-code@0.1.0`,
> `flow:atomic-commits@0.1.0`. 81 tests green, clippy clean.
>
> **Active slice (M1.1-revision):** redesign of the registry model
> around decentralized per-package repos, `[[registry]]` array +
> `[[mirror]]` + `[[override]]` in `vibe.toml`, content-addressed
> identity, transitive depsolver (`resolvo`), and a `vibe registry
> publish` maintainer utility. Design lock: [PROP-002](spec/modules/vibe-registry/PROP-002-decentralized-registry.md).
>
> **Host migration (2026-04-29).** The `vibespecs` registry
> organization moved from GitVerse to GitHub
> (<https://github.com/vibespecs>) because GitVerse's public REST API
> does not expose org-scoped repo creation, blocking `vibe registry
> publish`'s create-leg. The vibevm tool source itself stays on
> GitVerse — only the registry org migrates. New `GitHubCreator`
> adapter behind the existing `RepoCreator` trait drives the publish
> flow; `GitVerseCreator` remains in tree for any future Gitea-shape
> host that fully supports the org-scoped POST. Token path rotates
> to `~/.vibevm/<host>.publish.token` (per-host); legacy
> `~/.vibevm/git.publish.token` is the fallback. See
> [PROP-000 §7](spec/common/PROP-000.md#registry) and
> [PROP-002 §2.10](spec/modules/vibe-registry/PROP-002-decentralized-registry.md#publish).
>
> M1.5-gate docs landed (commands + authoring + glossary +
> CHANGELOG). M1.2 / M1.3 / M1.4 open. M1.6 queued (polished
> multi-registry / mirror UX, `vibe vendor`, richer publish
> adapters).
>
> **Status snapshot (2026-05-04, post-M1.4 + Tessl research).**
> M1.2 / M1.3 / M1.4 SHIPPED v0; M1.6 Phase B v0 SHIPPED. Both
> live smokes (M1.5-gate-v2, M1.6-mirror-vendor) walked end-to-end
> pass. Schema v3 design proposal in
> [PROP-003](spec/modules/vibe-resolver/PROP-003-dep-evolution.md)
> covers SAT solver via libsolv, cargo-style features, vibevm-native
> subskills, BCP-47 i18n. New milestones M1.7–M1.11 and M2.7–M2.10
> derive from
> [PROP-004 Tessl comparative research](spec/research/PROP-004-tessl-comparative-research.md):
> `vibe-mcp` (Claude-native context provider) leads the bunch.

This document is the long-form version of `VIBEVM-SPEC.md` §11 (staging
plan). It keeps the "why" and "nuance" that a compressed staging table
cannot carry. `VIBEVM-SPEC.md` remains authoritative on scope; if this
file and the spec disagree, the spec wins and this file is updated.

**Reading order.** Take it top-to-bottom. Each milestone is
self-contained — if work stops after M1, the tool is useful on its own
(a package manager). If work stops after M1.5, the tool is useful on
its own (a package manager + code generator). M2 makes it safe to ship
to other humans. M3 is speculation.

**Non-negotiable rule.** Build in staging order. M0 → M1 → M1.5 → M2.
Do not work on M1.5 before M1 is done; do not start M2 before M1.5 is
done. The temptation to skip to the "shiny" LLM milestone is
particularly strong and must be resisted — the walking-skeleton
discipline is what the whole project is about.

---

## M0 — Walking skeleton ✅ COMPLETE

**Landed.** A `vibe` CLI that scaffolds a project, installs / lists /
uninstalls packages from a local-directory registry, updates the
lockfile, and respects user-owned files. The package model works
end-to-end: hand-written `flow:wal@0.1.0` installs cleanly, uninstalls
cleanly, and user edits in `00-core.md` / `90-user.md` survive both
sides of the cycle.

**Shipped commands.**
- `vibe init [--path] [--name] [--stack]` — idempotent project
  scaffold.
- `vibe install <kind>:<name>[@version] … [--registry] [--assume-yes]`
  — plan → confirm → apply → lockfile update.
- `vibe list [--kind]` — lockfile display as table / `--json` /
  `--quiet` one-liner.
- `vibe uninstall <kind>:<name> [--assume-yes]` — reverse install,
  never touches user-owned files.

**Proven mechanics.**
- TOML manifest parsing with `deny_unknown_fields` everywhere.
- Semver-based package identity with `Latest | Req(VersionReq)`.
- Content-addressed cache under `.vibe/cache/<kind>/<name>/<version>/`
  with deterministic sha256 (forward-slashed relative paths for
  cross-OS stability).
- Boot-snippet conflict detection — both exact filename and numeric
  `NN-` prefix conflicts (matching `VIBEVM-SPEC.md` §6.2 intent).
- User-owned path guards enforced at plan time, not apply time.
- Exit codes per §9.4: 3 for conflict, 5 for declined confirmation.

**Not in M0.** No git registry, no LLM, no build, no sync, no check,
no update, no formal graph runner (workflows are procedural).

---

## M1 — The package manager

**Thesis.** Turn the walking skeleton into a real package manager:
fetch from a git registry, refresh the cache on demand, update
installed packages, lint the project's spec corpus, and give the user
introspection commands.

**Recommended entry point.** Git backend in `vibe-registry`. Without
it, nothing else in M1 has weight — `vibe update` is pointless against
a local dir, `vibe registry sync` is a no-op, and `vibe check`
works fine without a remote. Adding git first means every subsequent
M1 feature ships against a realistic remote from day one.

### M1.1 — Git-backed registry ✅ SHIPPED (2026-04-22)

**Shipped — code, publish, live smoke.** All items below landed;
design decisions pinned in
[PROP-001](spec/modules/vibe-registry/PROP-001-git-backend.md);
procedure for the live validation lives in
[`manual-tests/M1.1-git-registry-smoke.md`](manual-tests/M1.1-git-registry-smoke.md).

- `vibe-registry` gained a `Registry` trait implemented by both
  `LocalRegistry` (M0 code path, kept for tests and `--registry
  <path>`) and `GitRegistry` (new).
- First-use clone into `~/.vibe/registries/<hash>/clone/`, where
  `<hash>` is the 16-hex prefix of sha256 over the normalized
  registry URL. The full hash is stashed in `meta.toml` alongside the
  clone for audit. `VIBE_REGISTRY_CACHE` overrides the default root.
- Freshness policy: ≤1h skips the pull; >1h (or TTL=0 — i.e. `vibe
  registry sync`) triggers `git fetch --prune origin` + `git reset
  --hard origin/<ref>`. The reset-hard is deliberate: the registry
  cache is a read-only mirror, so a surprise merge commit would be a
  bug.
- `[registry]` in `vibe.toml` is now actually consumed (M0 only had
  `--registry`). `file://` URLs still route through `LocalRegistry`;
  anything else (SSH shorthand, `ssh://`, `https://`, `git+…://`)
  routes through `GitRegistry::open`.
- Lockfile `source_uri` switched to
  `git+<transport>://<host>/<path>#<kind>/<name>/v<ver>` for git
  sources; local registries still emit `file://`.
- `vibe registry sync [--path]` force-refreshes the configured git
  registry; no-ops with a note on a `file://` registry.

**Decisions made during the slice.**

- **Shell-out to `git`, not `git2`.** Headline reasons: SSH auth on
  Windows via OpenSSH-agent "just works" through the user's existing
  `git`, while `libgit2`/`libssh2` on Windows is a known lottery; no
  C toolchain or native-lib weight in the build; native error
  messages for free. Reversible via the narrow `GitBackend` trait
  (method names: `bootstrap` + `update` — the first dodges collisions
  with `Clone::clone` / `ToOwned::clone_into` on `Arc<dyn
  GitBackend>`). Full rationale: PROP-001 §2.1.
- **Windows posture.** Every git subprocess is spawned with
  `CREATE_NO_WINDOW`, `LC_ALL=C`, `LANG=C`, `GIT_TERMINAL_PROMPT=0`
  — no stray console windows, stable stderr for classification, no
  interactive prompts blocking CI.
- **Auth for M1.** SSH-agent (delegated to the user's git). Token /
  credential-helper HTTPS is M2.

**Done. Nothing remaining to tag M1.1.** The two more demo packages
(`flow:sync-from-code`, `flow:atomic-commits`) are on the path to
M1.5-gate, not to M1.1 — see the M1.5-gate subsection below.

### M1.1-revision — Decentralized per-package registry (Phase A DONE 2026-04-29 on GitHub `vibespecs`; host migrated from GitVerse mid-slice)

**Why.** The original M1.1 shape (monorepo-as-registry, `[registry]`
singleton, `#fragment` paths in lockfile `source_uri`) was fine for
three hand-written demo packages but would become a hostage-taking
architecture at scale — same failure mode that ties Nix to GitHub.
Before anyone downstream pins anything to the v1 shape, it is
cheapest to redesign once, properly.

**Scope.** Full design lock in [PROP-002](spec/modules/vibe-registry/PROP-002-decentralized-registry.md) (supersedes PROP-001 §2.3 / §2.4 / §2.6). Phase A of that PROP is this slice's shippable surface:

- **Decentralized registry** — each package is its own git repository under a hosting organization (`vibespecs/flow-wal`, `vibespecs/flow-sync-from-code`, …). Versions are git tags. Repo-naming convention is a property of the registry, not the CLI.
- **Multi-registry schema** — `vibe.toml` carries `[[registry]]` as an array (priority-ordered), with `[[mirror]]` and `[[override]]` entries. Even with one registry in practice today, the schema and code path support the full shape from day one.
- **Content-addressed identity** — lockfile `source_url` is informational; identity is `(kind, name, version, content_hash)`. Mirror-switching and host-migration never churn the lockfile. Integrity verified on every install. (Tested in anger by the 2026-04-29 GitVerse → GitHub host migration.)
- **Transitive depsolver** — `resolvo` crate, wrapped behind a `DepSolver` trait so `libsolv` remains a documented fallback. Capability-based `[provides]` / `[requires]` / `[[requires_any]]` / `[obsoletes]` / `[conflicts]` become semantic, not advisory.
- **`vibe registry publish <path>`** — maintainer utility creating a new package repo via the host's public API, pushing content, tagging version. Error surface tuned for non-admin contributors (clear 401/403/push-denied messaging). Host adapters: `GitHubCreator` (primary, `vibespecs` org); `GitVerseCreator` (legacy, retained for Gitea-shape hosts).
- **Live migration** — three demo packages (`flow:wal@0.1.0`, `flow:sync-from-code@0.1.0`, `flow:atomic-commits@0.1.0`) move from `anarchic/vibespecs` monorepo into `https://github.com/vibespecs/<kind>-<name>` per-package repos via the new publish utility. Original migration target was GitVerse's `vibespecs` org; the host changed mid-migration when GitVerse's missing org-scoped POST endpoint blocked end-to-end automation.
- **JTD wire-contract foundation** — host-API and `vibe --json` event shapes are schema-first (JTD), with `jtd-codegen` producing Rust types. Future LLM provider wrappers land on the same pattern.
- **Local fixtures relocate** from `packages/` to `fixtures/registry/` to separate test fixtures from the project's own future dogfooded `packages/`.

**Task breakdown** lives in [`TASKS.md`](TASKS.md) at repo root.

**Acceptance.** See §M1 acceptance (additive) in `VIBEVM-SPEC.md` §16 — the list there grew to cover per-package resolution, mirror fallback, override pin, publish error surface, lockfile schema v1→v2 migration.

### M1.2 — `vibe update` ✅ SHIPPED v0 (2026-05-04)

- `vibe update <pkgref>...` and `vibe update --all`: re-fetch the
  registry (if stale), re-resolve the package against its original
  root constraint, if a newer version satisfies the constraint show
  a diff (Added / Removed / Modified / Identical per file), confirm,
  apply. Reference docs at
  [`docs/commands/update.md`](docs/commands/update.md).
- File-modification case: if the project file matches the
  install-time cache (pristine), overwrite from the new cache.
  Otherwise refuse with `UserEditedFile` and a 3-way-diff hint
  pointing the operator at `vibe uninstall && vibe install` to
  consciously discard or back up the edit.
- Lockfile entry rewritten in v2 shape — `version`, `content_hash`,
  `source_url`, `source_ref`, `resolved_commit`, `boot_snippet`,
  `files_written`. `dependencies` and `overridden` preserved.
- v0 limits (queued for follow-up): refuses dep-graph evolution
  (`DependencyShapeChanged` when `[requires]` shape changes between
  versions); non-root transitives re-resolve at their exact locked
  version (only move on a force-push).

### M1.3 — `vibe check` (spec linter) ✅ SHIPPED v0 (2026-05-04)

v0 covers six of the ten checks listed below; reference docs at
[`docs/commands/check.md`](docs/commands/check.md).

Implements the full `VIBEVM-SPEC.md` §12 check list:
1. Manifest validity (`vibe.toml`, `vibe.lock` parse and match schema).
2. Dead `spec://` references.
3. Orphan `{#anchor}`s.
4. Anchor uniqueness within a spec file.
5. WAL freshness (modification timestamp < 24h, warn if older).
6. WAL well-formedness (required sections present).
7. Boot directory consistency (NN-name.md pattern, no number clashes).
8. Lockfile consistency (no orphan files in `spec/flows/*` etc.).
9. REVIEW marker aging (default 14-day threshold).
10. Implementation coverage (files with `build` history carry
    `Implements: spec://…` markers). This last check becomes
    meaningful only after M1.5 ships — in M1 it can be a warning-only
    noop.

**v0 implements:** 1, 5, 6, 7, 8, 9. **v1+ defers:** 2, 3, 4, 10
(require markdown-anchor parsing or `vibe build` provenance).

`vibe check --fix` is a narrow subset: remove dead anchor references
we can identify safely, nothing that loses information. **Not
shipped in v0** — fix candidates only land once the deferred
checks (#2 / #3 / #4) come online.

### M1.4 — `vibe show …` ✅ SHIPPED v0 (2026-05-04)

v0 ships `vibe show effective` and `vibe show config`. The
runner-aware subcommands defer to M1.5 (they need the LLM-build
pipeline's task-graph runner before they have anything meaningful to
render). Reference docs at [`docs/commands/show.md`](docs/commands/show.md).

Pure inspection, no mutation:
- `vibe show effective` ✓ — materialize the full spec corpus as one
  stream, with provenance (which package contributed what). The
  `EffectiveSpec` typed value from §5.3 finally gets a consumer.
- `vibe show graph [<workflow>]` *(deferred to M1.5)* — textual
  render of the task graph. Helps debug the install subgraph and,
  later, build.
- `vibe show node <name>` *(deferred to M1.5)* — details of a
  single node (inputs, outputs, cacheability).
- `vibe show config` ✓ — effective configuration with provenance
  (which flag / env var / vibe.toml value won).
- `vibe show plan <workflow> [args...]` *(deferred to M1.5)* —
  dry-run. Prints what would happen without executing.

### M1.5-gate — registry publish

Content landed 2026-04-22 / 2026-04-23; docs remain.

- [x] `flow:wal@0.1.0` → `git@gitverse.ru:anarchic/vibespecs.git` at
      `flow/wal/v0.1.0/`, commit `98e51fc` (2026-04-22).
- [x] `flow:sync-from-code@0.1.0` at `flow/sync-from-code/v0.1.0/`,
      commit `47582af` (2026-04-23). Derived from book chapter 3
      ("Архитектура памяти", subsection "Протокол Sync-from-Code").
      Boot-snippet prefix `20-`.
- [x] `flow:atomic-commits@0.1.0` at `flow/atomic-commits/v0.1.0/`,
      commit `2203239` (2026-04-23). Derived from book chapter 2
      ("Shared state: файлы как IPC", subsection "Атомарность") +
      Conventional Commits 1.0.0. Boot-snippet prefix `30-`.
- [x] Registry-level `README.md` lists all three v0.1.0 packages.
- [x] Three-package coexistence verified end-to-end against the real
      registry on 2026-04-23 via
      [`M1.5-gate-multi-package-smoke.md`](manual-tests/M1.5-gate-multi-package-smoke.md):
      distinct `10-`/`20-`/`30-` prefixes install side-by-side, one
      shared clone under `~/.vibe/registries/<hash>/`, symmetric
      uninstall, user-owned files byte-identical.
- [ ] Docs: `docs/commands/*.md` for every user-facing command;
      `docs/authoring-flow.md`, `docs/authoring-feat.md`,
      `docs/authoring-stack.md` for package authors. Independent of
      M1.2–M1.4 — can be done in parallel.

### M1 acceptance (from §16 of the spec)

- [x] `vibe install` resolves packages from git per `vibe.toml`. ✅ M1.1 (smoke-verified 2026-04-22 against real `vibespecs.git`)
- [x] Registry cache lives at `~/.vibe/registries/<hash>/`. ✅ M1.1
- [x] `vibe registry sync` refreshes. ✅ M1.1
- [ ] `vibe update <pkgref>` and `--all` work with diff display. *(M1.2)*
- [ ] `vibe check` runs every §12 check. *(M1.3)*
- [ ] `vibe check --fix` autofixes only safe issues. *(M1.3)*
- [ ] `vibe show effective` / `graph` / `config` all produce useful
      output. *(M1.4)*
- [x] Public registry on GitVerse with ≥ 3 packages. ✅ M1.5-gate (3/3, 2026-04-23)
- [ ] Documentation in `docs/` covers every command plus authoring
      guide per kind. *(M1.5-gate, open)*

**Estimated effort.** 2–4 weekends. The git backend is the biggest
lift; the rest is straightforward with `vibe-core` already in place.

### M1.6 — Multi-registry, mirror, and vendoring polish

**Thesis.** M1.1-revision laid the schema and code paths for
multi-registry, mirrors, and overrides — but the v1 release reasonably
exercises only a single `[[registry]]`. M1.6 brings the remaining
multi-source story to production quality.

**Scope.**

- **Real multi-registry** — `[[registry]]` array with >1 entry, each a
  separate hosting organization. Priority-ordered resolution exercised
  end-to-end against a second live registry.
- **Mirror fallback chain** — `[[mirror]]` tried before canonical per
  registry; integrity-checked against the lockfile; hard-fail on
  content drift, escape-hatch `--trust-mirror` for deliberate
  mirror-vs-upstream divergence during an upstream outage.
- **`vibe registry vendor [--out <dir>] [--force]`** — generate a
  local mirror directory containing every package referenced by the
  current lockfile, shaped so it can be used as a `file://`-scheme
  `[[mirror]]`. Enables air-gapped / offline installs without code
  changes elsewhere. **Shipped (Phase B v0).** Reference docs at
  [`docs/commands/registry-vendor.md`](docs/commands/registry-vendor.md).
- **CLI surface for registry management** — `vibe registry add <name>
  <url>`, `vibe registry list`, `vibe registry set-mirror <of> <url>`,
  `vibe registry remove <name>`, `vibe registry status` (which
  registry answered which package last).
- **Publish adapters beyond GitVerse** — GitHub, Gitea, Forgejo on
  adopter demand. Adding each is one new `RepoCreator` impl; no
  consumer-side change.
- **Resolver performance** — full dep-graph cache keyed by content
  hash of inputs, parallel `git ls-remote` via rayon, `git archive`
  single-file manifest prefetch landed in M1.1-revision stays, gains
  concurrency.
- **Supply-chain attestation (optional, ambitious)** — signed tags or
  sigstore-style attestations per release; consumer verifies on
  install. This is the kind of thing a principal-engineer lens
  (PROP-000 §17) prompts us to design early even if we don't ship v1.

**Estimated effort.** 2–3 weekends on top of M1.1-revision.

### M1.7 — `vibe-mcp` server (Claude-native context provider)

**Thesis.** Today vibevm only writes files into the project tree; the agent reads everything that happens to be there. To position vibevm as a *Claude-native* package manager, it must speak Model Context Protocol so the agent can query packages, capabilities, and subskills on demand — like Tessl's `query_library_docs`, but built around vibevm's decentralised content-hashed primitives.

Source: [PROP-004 §5.1](spec/research/PROP-004-tessl-comparative-research.md#mcp-server). Targets [`https://modelcontextprotocol.io`](https://modelcontextprotocol.io).

**Scope (slices 1–5 shipped).**

- ✅ `vibe-mcp` crate exposing an MCP server over stdio (slice 1).
- ✅ Tools: `query_package`, `read_subskill`, `materialise_subskill` (slices 1+3).
- ✅ Per-subskill files index in lockfile schema v3, lazy-pull works end-to-end (slice 3).
- ✅ Agent auto-detection + config writers — slice 2 (Claude Code, Cursor); slice 4 extends to Claude Desktop, OpenCode, Codex.
- ✅ `vibe mcp install` UX — slice 4 first-pass; slice 5 reshape to `--scope project|user|both` × `--what mcp|skill|both` axes with three-question wizard. Per-format mergers (JSON for Claude Code/Desktop/Cursor/OpenCode, TOML for Codex).
- ✅ `vibevm` SKILL.md — slice 4 first version (inside-project only); slice 5 rewrite into two-state form (Section A bootstrap-mode for empty directories, Section B inside-project, common section for both). Vendored at compile-time. Description widened to trigger on bootstrap intents.
- ✅ Global `--invoked-by <agent>` flag + `VIBE_INVOKED_BY` env var (slice 4). Stamps every JSON envelope with the calling agent's identity.
- ✅ **Bootstrap mode (slice 5).** `vibe mcp install --scope user` works without `vibe.toml` — the operator can wire vibevm into agents globally on first install, then let the agent itself create vibevm projects on demand. MCP entry under user-scope omits `--path` so one global config serves every project.
- ✅ **Lifecycle commands (slice 5).** New `vibe mcp upgrade` (refresh stale installs after `cargo install` — does NOT create new installations) + `vibe mcp uninstall` (mirror of install — drops `vibevm` block, deletes SKILL.md, foreign keys preserved). `vibe mcp status` extended with skill-drift report.
- New manual smoke `manual-tests/M1.7-mcp-claude-code-smoke.md` walking a full Claude Code → MCP → vibevm round-trip — pending. Operator-walked smoke for OpenCode + Codex covered by [`docs/guides/agent-mcp-quickstart-opencode.md`](docs/guides/agent-mcp-quickstart-opencode.md) acceptance checklist.

**Open follow-ups.** Plan-preview + apply-confirm prompt before writes (currently the wizard skips straight to apply). `query_capabilities` / `list_subskills` discovery MCP-tools. Gemini agent + Copilot CLI/VSCode. Integration with the LLM virtual-capability emission story (Phase F, post-M1.5). Preserving comments in handcrafted Codex `config.toml` (would require switching from `toml` to `toml_edit`).

**Estimated effort.** Slices 1–5 done. Remaining follow-ups roll into M1.5 dependencies.

### M1.8 — `vibe review` static quality scoring

**Thesis.** vibevm has `vibe check` (binary findings, structural). It has no quality score, no LLM-judge component, no aggregate "this package is 87% production-ready" surface. Tessl ships exactly that and ties it to publish gates. Static portion is buildable today without LLM.

Source: [PROP-004 §5.2](spec/research/PROP-004-tessl-comparative-research.md#quality-evaluation).

**Scope (static portion only).**

- New `vibe-eval` crate.
- Three-axis scoring: validation (frontmatter / line-count / structural), implementation (heuristics — content density, code-block balance), activation (description specificity heuristics).
- `vibe review <pkgref>` outputs 0-100 per axis + aggregate.
- Threshold conventions: 90%+ ready, 70–89% ship-with-warnings, <70% blocks publish unless `--accept-low-quality`.
- `vibe review --json` for CI consumption.

LLM-judge mode and `--optimize` auto-edit loop land in M2.7 once `vibe-llm` is real.

**Estimated effort.** 1 weekend (static).

### M1.9 — `describes` PURL linkage to upstream packages

**Thesis.** Tessl's headline marketing — "version-matched documentation for 10K+ packages" — rides on a single field: `describes = "pkg:pypi/fastapi@0.116.1"`. We need the equivalent so vibevm packages can declare which external library they're authored against.

Source: [PROP-004 §5.3](spec/research/PROP-004-tessl-comparative-research.md#purl-describes). Targets the [Package URL spec](https://github.com/package-url/purl-spec).

**Scope.**

- Optional `[package].describes` field in `vibe-package.toml` accepting PURL syntax.
- PURL parser in `vibe-core`.
- Lockfile records the upstream PURL.
- `vibe check` warns when project-declared upstream version differs from a `describes` package's pinned upstream.

**Estimated effort.** 1 weekend.

### M1.10 — `vibe outdated`

**Thesis.** Trivial UX win that scales with adoption. Cargo / npm / dnf all have it. We don't.

Source: [PROP-004 §5.13](spec/research/PROP-004-tessl-comparative-research.md#outdated).

**Scope.**

- `vibe outdated` reads lockfile, calls `MultiRegistryResolver::list_versions` per package, renders a status table.
- `--json` for CI.
- `--upstream` mode walks `describes` PURL targets (composes with M1.9).

**Estimated effort.** 1 weekend.

### M1.11 — Agent auto-detection at `vibe init`

**Thesis.** Tessl's `tessl init` detects which coding agent is in use and writes appropriate config. We currently write all three (`CLAUDE.md` / `AGENTS.md` / `GEMINI.md`) regardless. With M1.7 in flight, agent detection becomes load-bearing — different agents need different MCP-config files.

Source: [PROP-004 §5.7](spec/research/PROP-004-tessl-comparative-research.md#agent-auto-detect).

**Scope (effectively closed alongside M1.7 slices 2 + 4).**

- ✅ Probes for Claude Code, Claude Desktop, Cursor, OpenCode, Codex (slice 4). Project markers + user-level config-dir host probe. `--force` pivot to install in agents absent from this machine.
- ✅ Per-detected-agent MCP-server config writer — JSON for Claude Code/Desktop/Cursor/OpenCode, TOML for Codex. Foreign keys preserved on merge.
- Gemini and Copilot CLI/VSCode integrations remain open follow-ups; the [`Agent`](crates/vibe-cli/src/commands/mcp.rs) enum has the per-agent profile slot ready.
- Instruction-file fan-out (`CLAUDE.md` / `AGENTS.md` / `GEMINI.md` per `vibe init`) is a separate concern from MCP integration and stays at the current "write all three" default until a concrete bug surfaces.

**Estimated effort.** Closed alongside M1.7 slice 4. Gemini / Copilot follow-ups land per-demand.

### M1.12 — `vibe.toml` `[requires]` section + cargo-shape install ✅ SHIPPED (2026-05-08)

**Thesis.** Bring vibevm in line with cargo / npm / Poetry / Bundler:
the project manifest carries the *declaration* of dependencies
(human-readable, in semver-constraint form), the lockfile carries the
*materialisation* (resolved versions, content hashes, transitive
graph). Pre-`[requires]` schema kept the user's pkgrefs only in
`vibe.lock`, which made `vibe install` (no arguments) a no-op and made
PR diffs unreadable (a one-line dep change manifested as dozens of
hash/source/ref lines in the lockfile).

**Scope (closed in one slice).**

- ✅ `ProjectManifest` gains `[requires]` section, reusing the existing `Requires` type from `vibe-package.toml`. Round-trips through serde with the modern `packages = ["flow:wal@^0.1", …]` shape; pre-`[requires]` manifests parse cleanly with the section absent.
- ✅ `vibe install <pkgref>` writes the user-supplied pkgref to `vibe.toml` `[requires].packages` after a successful apply (de-duplicated by `(kind, name)`; constraint change overwrites prior entry).
- ✅ `vibe install` with no arguments reads `[requires].packages` and installs every entry — the cargo `cargo build` / npm `npm install` shape. First-run migration path: when `[requires]` is empty but `vibe.lock` `meta.root_dependencies` is not, the manifest is seeded from the lockfile snapshot before resolving.
- ✅ `vibe uninstall <pkgref>` drops the matching entry from `vibe.toml` `[requires].packages` symmetrically with the lockfile cleanup. Pure transitives (never declared) leave the manifest untouched.
- ✅ Spec updated: `VIBEVM-SPEC.md` §5.6 install workflow gains an `install:update-manifest` node + the install-from-manifest mode; §7.4 reframes `meta.root_dependencies` as a mirror of `vibe.toml`; §7.5 adds the `[requires]` section and the two-file model paragraph. `PROP-002 §2.7` refactored accordingly.

**Estimated effort.** One slice. Tests: 4 e2e + 4 unit + 2 schema round-trip.

### M1.13 — Cargo-shape version constraints (caret default + `--exact`) ✅ SHIPPED (2026-05-08)

**Thesis.** M1.12 made `[requires].packages` the source of truth for declared deps, but it stored whatever the CLI form was — bare `flow:wal` (no version) round-tripped as `"flow:wal"` with `VersionSpec::Latest`, which made every subsequent `vibe install` / `vibe update` potentially pull a breaking-change major. Cargo / npm / Poetry / Bundler all solve this the same way: resolve to a concrete version at install time, write a caret constraint in the manifest. M1.13 adopts that convention and aligns the parser with Cargo's shorthand rules.

**Scope (closed in one slice).**

- ✅ `VersionSpec::parse` simplified to a single `semver::VersionReq::parse` call. Bare semver (`0.3.0`) is now Cargo shorthand for caret (`^0.3.0`); `=0.3.0` is the explicit-equal form. Same parser Cargo / npm / Poetry use.
- ✅ `vibe install <pkgref>` (no version) resolves to a concrete version and writes the caret constraint to `vibe.toml` `[requires].packages` (`flow:wal@^0.1.0`). Explicit CLI constraints (`@^0.1`, `@~0.1.0`, `@=0.1.0`, ...) are preserved verbatim — operator's intent wins.
- ✅ New `--exact` flag (npm `--save-exact` shape): always pins the manifest to `=<resolved-version>`, overriding any CLI constraint form. For operators who want strict reproducibility regardless of how they typed the pkgref.
- ✅ `vibe-resolver`'s `capability_version_for_provider` updated to read `(major, minor, patch)` from the first `Comparator` of any constraint shape — bare `0.3.0`, `=0.3.0`, `^0.3.0`, `~0.3.0` all anchor at version 0.3.0 for capability matching. Previously it relied on the `=`-prefix string trick which broke when the parser stopped emitting `=` for bare semver.
- ✅ Spec updated: `VIBEVM-SPEC.md` §7.1 documents the Cargo-style version syntax and the caret-default + `--exact` write-side rules; §7.5 example shows `^0.1.0` constraints; install.md gets a full pkgref-syntax table and an `--exact` example.

**Migration policy.** Old `"flow:wal"` (no-version) entries already on disk in `vibe.toml` are left untouched — `vibe.toml` is human-edited and we don't auto-rewrite without explicit operator action. New installs write caret. The two coexist cleanly: `Latest` and `^x.y.z` are both valid `VersionSpec` shapes.

**Estimated effort.** One slice. Tests: 6 unit on `finalize_pkgref_for_manifest` + 3 e2e (caret default / explicit preservation / `--exact`) + 3 unit on the new bare-semver-as-caret + tilde + eq parsing in `package_ref` and `capability_ref`.

### M1.14 — Authenticated registries (production-ready private repos) ✅ SHIPPED (2026-05-08)

**Thesis.** A real-world walk against opencode + glm-flash on a fresh machine surfaced a UX hole: `vibe install` of a non-existent package produced a Git Credential Manager Core popup on Windows when GitVerse returned 401 for the missing public repo. Two underlying gaps: (1) no declarative model for "is this registry public or authenticated"; (2) no runtime policy on what 401 means in each case. M1.14 closes both end-to-end and brings vibevm in line with cargo / npm / Poetry on the auth axis. Twelve sub-slices total, landed across the day:

**Scope (all closed across `5f296d9..a915b12`).**

- ✅ Spec contract — PROP-002 §2.2.1 (per-registry auth axis: `none` / `token-env` / `credential-helper` / `ssh`); PROP-002 §2.3.1 (auth-aware 401 classifier — public-401 walks past, authenticated-401 halts).
- ✅ Schema — `AuthKind` enum, `RegistrySection.auth/token_env`, `resolve_token_env_name()` host-derivation helper.
- ✅ CLI — `vibe registry add --auth --token-env`; flags table updated.
- ✅ TTY-aware credential-helper silencing — non-TTY / `--unattended` runs silence GCM, `credential.helper`, `core.askPass` so a 401 cannot become a blocking GUI window.
- ✅ Stderr classifier — `could not read Username/Password`, `User cancelled dialog`, `HTTP 401/403`, `401 Unauthorized`, `403 Forbidden` all classify as `AuthFailed`.
- ✅ Token injection — `inject_token(plain_url, token)` helper applies `https://x-access-token:<TOKEN>@host` shape to https URLs only; `GitPackageRegistry::open_with_auth` resolves env-var at construction time; `effective_token_value` getter for closure capture.
- ✅ Bootstrap-with-scrub — token never persists in `.git/config`. After `backend.bootstrap(credentialed_url, ...)` succeeds, `backend.set_remote_url(clone_dir, "origin", plain_url)` rewrites origin to credential-free form. New `GitBackend::set_remote_url` trait method with default impl.
- ✅ MissingToken precheck — `auth = "token-env"` without env-var fails before spawning git, with hint naming the exact env-var to set.
- ✅ Per-auth walk-vs-halt in resolver — `auth = "none"` + 401 → walk; `token-env` / `credential-helper` + 401 → halt; `MissingToken` always halts.
- ✅ `--auth-required` strict-auth gate — flips public-401 from walk-past to halt for CI / cron use cases. Reaches `install` (M1.14.2) and `update` / `outdated` (M1.14.3 — surface consistency).
- ✅ Aggregated per-registry error report — `RegistryError::PackageNotFoundEverywhere { kind, name, summary }` carries pre-formatted multi-line per-registry status; surfaces inline through the standard `error: ...` chain.
- ✅ `toml_edit`-based comment-preserving writes — operator's hand-edited comments in `vibe.toml` survive `vibe install` / `uninstall` / `registry add` writes.
- ✅ Surface consistency — MCP `--yes` flag wired to actual TTY confirm prompt (was vestigial); `--assume-yes` alias on every MCP confirm-skip flag; `--exact` extends from `install` to `update`; `--auth-required` extends from `install` to `update` + `outdated`.
- ✅ User-facing reference — `docs/registry-auth.md` (250+ lines) with TL;DR table, per-regime walkthroughs, token-discipline checks, troubleshooting; `docs/version-syntax.md` for semver constraints.

**Estimated effort.** Took the full 2026-05-08 push (twelve commits across the day from `5f296d9` to `a915b12`). Production-ready for v0; no known auth-related gaps for private read-only registries.

---

### M1.15 — Git-source dependencies (whole-repo-as-package) ✅ SHIPPED (2026-05-10)

**Thesis.** Cargo / npm / Poetry / Bundler / Go modules all let you declare a dependency as "this whole git repository is the package" — `[dependencies] foo = { git = "..." }`, `"foo": "git+https://...#tag"`, `gem 'foo', git: '...'`. vibevm's existing `[[override]]` mechanism technically does this, but it is semantically a *patch* (replace a registry-resolved pkg with a fork) — not a *first-class declaration*. The use cases for first-class git-source are different: a single private/internal package without a multi-package `[[registry]]` org behind it, an active fork that **is** the source (not a patch on top), or a cross-organisation pull. Spec contract per [PROP-002 §2.4.1](spec/modules/vibe-registry/PROP-002-decentralized-registry.md#git-source).

**Scope.**

- Spec — PROP-002 §2.4.1 ([git-source declarations](spec/modules/vibe-registry/PROP-002-decentralized-registry.md#git-source)) + `VIBEVM-SPEC.md` §7.5 update (table-form `[requires.packages]` example with git-source). Done.
- Schema — `Requires.packages` becomes `BTreeMap<PackageRef, PackageDep>` instead of `Vec<PackageRef>`. New `PackageDep` enum: `Constraint(VersionSpec)` (registry-resolved) | `Git { url, ref_kind: GitRef, version: Option<VersionSpec>, auth, token_env }`. Custom `Deserialize` accepts both legacy array-of-strings form and modern table form; on round-trip writes table form.
- Wire grammar — exactly one of `tag` / `rev` / `branch` required for git-source; zero or two+ rejected at parse time. `version` optional (verification only). `auth` defaults to `"none"`; per-source explicit (no host-match against `[[registry]]`).
- Resolver — `MultiRegistryResolver` learns to short-circuit on a git-source declaration the same way it does for `[[override]]`. Resolution order: override > git-source > registry-walk. Transitives of a git-source package resolve through the consuming project's `[[registry]]` (existing override path).
- Single-package `GitPackageRegistry` constructor — direct repo URL instead of `org_url + naming`. Reuses the existing token-injection / bootstrap-with-scrub / content-hash discipline.
- Lockfile — schema bumps to v3; new `source_kind = "registry" | "git" | "override"` field per `[[package]]`. v2 lockfiles read transparently and migrate on next install.
- `vibe install` / `uninstall` — accept `vibe install <pkgref> --git <url> --tag/branch/rev <ref>` to add a git-source declaration to `[requires.packages]` without hand-editing `vibe.toml`. Uninstall identical to registry-resolved.
- Tests — hermetic for parser (legacy/modern wire-form round-trip, missing/conflicting refs, identity mismatch); resolver (override > git > registry order, transitive resolution); production walk against a real private GitHub repo (extending the M1.14.4 recipe).
- Docs — new `docs/git-source-dependencies.md` operator reference; update `docs/commands/install.md` with new flags; update `docs/registry-auth.md` to clarify per-source auth vs `[[registry]]` auth.

**Out of scope.** Multiple git-source entries against the same `(kind, name)` (rejected as `DuplicateDeclaration`). `vibe registry test` probing git-source (registry-only diagnostic by design — git-source has no fall-through walk). Mirror chain for git-source (`[[mirror]]` is registry-only). git-source-as-mirror-of-registry-package (collapses semantically with `[[override]]`).

**Estimated effort.** Medium. Schema is the largest change; Resolver/registry/lockfile are surgical. Expected ~8–12 commits across one day. Backwards compat is the most subtle aspect — every existing manifest must continue to parse and round-trip.

---

### M1.16 — Registry redirect (delegated package via stub repo) ✅ SHIPPED (2026-05-10)

**Thesis.** A registry org's stub repo for a package may carry a marker file pointing at an external git repo where the package actually lives, instead of carrying the package content itself. Use case: an org owner wants the package to live in their namespace (so consumers find it via the standard `[[registry]]` walk without knowing about the external author) but offload hosting / PRs / permissions to a different team. Spec contract per [PROP-002 §2.4.2](spec/modules/vibe-registry/PROP-002-decentralized-registry.md#redirect).

**Scope.**

- Spec — PROP-002 §2.4.2 ([registry redirect](spec/modules/vibe-registry/PROP-002-decentralized-registry.md#redirect)) + `VIBEVM-SPEC.md` §7 update with marker-file shape. Done.
- Marker file — new `vibe-redirect.toml` schema in `vibe-core` (parallel to `vibe-package.toml`). Required `target_url`; optional `ref_policy = pass-through-tag | pinned`, `pinned_ref` (required if pinned), `auth` / `token_env` (target-side, mirrors PROP-002 §2.2.1), `description`.
- Registry resolver — `GitPackageRegistry::fetch_dep_manifest` falls through to `vibe-redirect.toml` when `vibe-package.toml` is absent. New `RegistryError::AmbiguousStub` (both files present). Hop limit = 1 — if the target is itself a stub, raise `RedirectChainNotAllowed`. Apply `[redirect].auth` (independent of stub auth) when fetching the target.
- Tag visibility — `list_versions(stub_url)` returns stub tags (not target tags). Org owner gates which versions enter their namespace.
- Identity — `content_hash` over target content (not stub). Force-pushed target tag detected by hash mismatch on next install. New `--trust-redirect` flag to accept deliberate target switches (parallel to `--trust-mirror`).
- Lockfile — new `via_redirect` field per `[[package]]` records the stub URL when redirect was followed. Same v3 schema bump as M1.15 (additive `via_redirect = null` for non-redirected entries).
- Cache — stub and target are separate entries in the per-user registry cache (`~/.vibe/registries/<canonical-url-hash>/...`), independent freshness windows.
- CLI — `vibe registry redirect <pkgref> --to <url>` creates a stub repo via `RepoCreator`. Symmetric to `vibe registry publish`; mutually exclusive on the same pkgref slot. `vibe registry redirect-sync <pkgref>` mirrors target tags into the stub for ergonomic version gating.
- `vibe show <pkgref>` and `vibe registry list` surface `via_redirect` URL and the redirect's `description` field.
- Tests — hermetic for marker-file parser (round-trip, missing fields, ambiguous-stub); resolver (pass-through-tag, pinned, hop-limit=1 rejection, auth-on-target, identity over target content); production walk against a stub→target pair on real GitHub.
- Docs — new `docs/registry-redirect.md` operator reference; update `docs/registry-auth.md` to clarify two-layer auth (stub + target).

**Out of scope.** Redirect chains (rejected at hop=2). Signed redirect markers (target-attestation by org owner's key). Auto-deprecation forwarding (`[redirect.deprecated] new_pkgref = ...`) for renames — separate feature. `[[mirror]]` interaction with stubs.

**Estimated effort.** Medium-small. Less invasive than M1.15 (no schema change to existing manifests). Resolver fall-through + new marker parser + lockfile field + CLI command. Expected ~6–10 commits.

**Order vs M1.15.** M1.15 (git-source) and M1.16 (registry redirect) are independent surfaces and may land in either order. Reasonable shape: M1.15 first because it shares the `GitPackageRegistry::open_single_package` constructor that M1.16's redirect-follow path also wants (target_url is a single-package URL). Landing M1.15 first means M1.16 reuses already-tested machinery.

---

### M1.17 — Workspace (multi-package projects) — 🚧 Phases 1–5 shipped (2026-05-21)

**Thesis.** Bring vibevm in line with cargo `[workspace]` / Maven multi-module: a project decomposes into modules, each published independently — or not at all — with the whole structure declared in one manifest. Design lock: [PROP-007](spec/modules/vibe-workspace/PROP-007-workspace.md).

**Shipped (2026-05-21)** — the workspace data model and tooling, in six phases:

- **Phase 1** — one unified `vibe.toml` per node (`vibe-package.toml` retired); `[project]` ⊕ `[package]`; `[workspace]` / `[origin]` / `[package].publish` parsed; all manifest legacy deleted.
- **Phase 2** — the `vibe-workspace` crate: `Workspace::discover` (bubble to the absolute root), recursive nesting, glob members, cycle detection.
- **Phase 3** — `path`-source cross-member deps with dual-form `{ path, version }`; resolution priority `override > path > git-source > registry`; `vibe.lock` schema v4 (`source_kind = "path"`; legacy readers removed).
- **Phase 4** — `[workspace.versions]` named placeholders, resolved recursively (matryoshka, nearest wins).
- **Phase 5** — selective publish (`[package].publish`); `vibe workspace publish` — topological walk, `[origin]` marker, "contribute upstream" signalling.
- **Phase 6** — `VIBEVM-SPEC.md` §4.2 / §7.6 workspace documentation; docs sweep.

**Remaining.** Wiring `vibe install` / `vibe build` to discover the workspace and run unified multi-member resolution — a follow-up milestone, gated on a per-member materialisation decision PROP-007 §2.4 / §3 leaves open; the path-source resolver it builds on is already implemented and tested. Smaller deferrals: `version = { workspace = true }` member-version inheritance (PROP-007 §2.6 names no source table) and the `--archive` host-API publish lockdown.

**Order.** PROP-007 has no dependency on the index and landed before M1.18. It delivers the bulk of the multi-package request on its own.

### M1.18 — Loading model (PROP-009) — ✅ SHIPPED (2026-05-22)

**Thesis.** Replace the flat `spec/boot/NN-*.md` boot model with a
computed loading model: two physically separate trees — authored
`spec/` and a committed `vibedeps/` — with each node's boot sequence
*computed* from the unified resolution and projected into generated
`INLINE.md` / `INDEX.md` artifacts. Answers PROP-007 §6 question 3 and
subsumes the workspace-aware `vibe install` left open by M1.17. Design
lock: [PROP-009](spec/modules/vibe-workspace/PROP-009-loading-model.md).

**Shipped (2026-05-22), phases 1–7.** Phases 1–6 — the schema (`link`
types, boot `category`, retired `[writes]` and `NN-` prefix), the
`vibedeps/` materialisation tree, the computed-view boot engine,
`INLINE.md` / `INDEX.md` / redirect generation, workspace-aware
`vibe install` (plus five follow-ups), `vibe reinstall`, and
published-copy boot regeneration in `vibe workspace publish`. Phase 7 —
the vibevm self-migration, the `VIBEVM-SPEC.md` consistency pass (owner
sanction granted), the `docs/` sweep, and
[PROP-012](spec/modules/vibe-workspace/PROP-012-managed-redirect-block.md)
(the managed `<vibevm>` redirect block) folded in. The dynamic-entry
`when` gate — an OS condition (`when = "os:<name>"`) on a
`[boot_snippet]` — landed 2026-05-22, closing the PROP-009 §2.3 contract
gap Phase 4 had flagged.

**Remaining.** Phase 8 — the effective-spec view — is v1.5 scope; it
shares the computed-view engine and rides with the M1.5 milestone.

**Order.** Followed M1.17 directly; no dependency on the index.

### M1.19 — Qualified package naming (PROP-008) — ✅ SHIPPED (2026-05-22)

**Thesis.** Replace the flat `<kind>:<name>` namespace with reverse-FQDN `group` qualification (Maven `groupId` shape), keeping short names as CLI sugar. Design lock: [PROP-008](spec/modules/vibe-registry/PROP-008-qualified-naming.md).

**Shipped (2026-05-22) — the whole milestone, under MFBT.**

- Mandatory `[package].group`; identity tuple is `(group, name, version, content_hash)`; `kind` left identity and is now pure metadata.
- pkgref grammar `[kind:][group/]name[@version]` — `kind` prefix optional, validated when present; manifests store the qualified `group/name` form.
- `naming = "fqdn"` repo names (`org.vibevm.wal`), the new default; `kind` left the repository name. Registry resolution keys on `(group, name)`.
- The package index is group-native (Phase 7) — the entry carries `group` + `workspace_origin`, the `by-name/` layer is the candidate-set file `by-name/<name>.json`.
- Lockfile schema v5 (v4 was PROP-007's); each `[[package]]` carries `group`.
- Index-backed short-name resolution (Phase 5) — `vibe install wal` is qualified to `org.vibevm/wal` at the CLI input boundary, lockfile-first then index/scan; the local-directory registry path is scanned directly, the multi-registry path walks each registry's `by-name/<name>.json`.
- Collision detection + exit code `7` (Phase 6) — a short name matching two groups fails as `AmbiguousPackage` with the numbered qualified alternatives, never a guess.
- `VIBEVM-SPEC.md` §7 / §8 / §9.4 and `docs/` reconciled with qualified naming (Phase 8 docs).

**Owner-only follow-up.** The live `vibespecs` repo renames to the `fqdn` shape are outward work and gate nothing in-repo — every hermetic test is self-contained and green.

**Order.** Needs the short-name-resolution machinery from [PROP-005](spec/modules/vibe-index/PROP-005-package-index.md) (the index), which is implemented. Sequence: M1.17 → M1.18 → PROP-005 → M1.19.

### M1.20 — Local package cache (PROP-010) — DRAFT design

**Thesis.** Elevate the registry cache to a first-class,
machine-global, accretive package store with an offline mode — so a
new workspace member or an entirely new project resolves and
materialises its dependencies from the cache with no network, reusing
whatever earlier, unrelated projects pulled. Design lock:
[PROP-010](spec/modules/vibe-registry/PROP-010-local-package-cache.md).

**Scope (DRAFT — five §5 open questions pending an owner design
session).** The cache keyed by PROP-008 qualified package identity, so
it is registry-config-independent; a `--offline` policy flag
(`VIBE_OFFLINE`); offline resolution against the cache; a user-level
default registry configuration that seeds new projects; a `vibe cache`
management surface (`path` / `list` / `add` / `clean`).

**Order.** The identity-keyed cache depends on PROP-008 (M1.19);
sequenced after it.

### M1.21 — Incremental install (PROP-011) — ✅ SHIPPED (2026-05-22)

**Thesis.** Refine PROP-009's whole-tree `vibe install` into an
incremental operation: skip the depsolver when `vibe.lock` is fresh —
which also makes `vibe install` lockfile-respecting — and
re-materialise only the `vibedeps/` slots that actually changed, so
`vibe install` on a large workspace stops paying whole-tree cost.
Boot-artifact regeneration deliberately stays whole-tree: it is the
cheap phase. Design lock:
[PROP-011](spec/modules/vibe-workspace/PROP-011-incremental-install.md).

**Shipped (2026-05-22), four phases.** A cargo-style lockfile-freshness
check (`vibe-workspace::freshness`) that lets a bare `vibe install`
skip the depsolver outright; the slot-present materialisation skip plus
the `slot_integrity` user-config setting; minimum-churn re-resolution
that holds the locked version of every untouched dependency; the
`VIBEVM-SPEC.md` §9.1 contract edit under owner sanction.

**Deferred.** Skipping the registry walk for an unchanged subtree —
true incremental re-resolution — needs the depsolver's pin-preference
machinery (PROP-003 §2.1) and rides with the SAT solver.

**Order.** No dependency beyond PROP-009 (M1.18, shipped).

### M1.22 — Managed redirect block (PROP-012) — ✅ SHIPPED (2026-05-22, within M1.18)

**Thesis.** `vibe install` overwrote the whole of `CLAUDE.md` /
`AGENTS.md` / `GEMINI.md` on every run — destroying hand-authored
instructions, other tools' content, and vibevm's own four rules.
vibevm now owns only a `<vibevm>`-delimited block and leaves the rest
to its co-tenants. Design lock:
[PROP-012](spec/modules/vibe-workspace/PROP-012-managed-redirect-block.md).

**Shipped.** A machine-locatable `<vibevm>` … `</vibevm>` block;
exactly one per file, a hard stop on a malformed file; absent →
create, present → splice; plan-time validation; migration of the old
whole-file generated redirect. PROP-012 corrects a defect shipped in
PROP-009 Phase 4, so it was a prerequisite for the Phase-7 redirect
rewrite and landed **inside M1.18 Phase 7**. The M1.22 number is
nominal — the work is part of M1.18.

---

## M1.5 — Generation

**Thesis.** vibevm earns its tagline — "the disciplined runtime for
spec-driven vibecoding" — only when it can actually produce working
code from a `feat × stack` pairing. This milestone is where the tool
makes the jump from "manages specs" to "produces software."

**Sequencing — deferred (2026-05-22, owner decision).** M1.5 is
deferred behind a base-stabilisation phase. The package machinery —
the PROP-005 → PROP-008 → PROP-010 chain, plus the testing and
structural refactors that follow — is to reach relative stability
before generation is layered on top. Generation, and not only LLM
generation, belongs on a base solid enough to absorb large refactors;
moving into that knowledge domain before the base is condition-stable
would build on sand. M1.5 resumes once the base is in place.

### M1.5.1 — LLM provider abstraction

- `vibe-llm` gets real. `LLMProvider` trait with methods `chat` and
  `chat_with_tools`. First implementation: Anthropic via the Messages
  API.
- `ProviderConfig` read from `vibe.toml` `[llm]` section: default
  provider, default model, `api_key_env`. Per-step overrides (`[llm.build]`,
  `[llm.sync]`) supported per spec §7.5.
- Streaming (`stream_chat`) is out of scope for M1.5; add in M2 when
  CLI output polish lands.
- OpenAI, OpenRouter, Ollama providers land in a second slice after
  Anthropic works — they all share the Messages-or-ChatCompletions
  shape plus a tool-use loop, so the incremental cost per provider is
  small.

### M1.5.2 — Tool-use loop

- The build loop (pseudocode in spec §10.4) runs against an explicit
  tool set: `read_file`, `write_file`, `list_dir`, `run_test`,
  `run_shell` (restricted to a short allowlist). Every tool
  invocation is sandboxed to project root — no `..` escape, no
  absolute-path reads.
- Tool-use traces are recorded for debugging and cost reporting.

### M1.5.3 — `vibe build`

- `vibe build <feat-pkgref> [--stack <name>]`. Loads the effective
  spec (all active flows + active stack + the named feat + WAL),
  invokes the LLM to produce a `BuildPlan`, asks for confirmation,
  then runs the tool-use loop to generate code files.
- Generated code carries `// Implements: spec://…` markers so `vibe
  check`'s implementation-coverage check can verify traceability.
- `vibe build --with-install` composes install + build for the
  fast-prototyping path.

### M1.5.4 — `vibe sync` (Sync-from-Code)

- Per book chapter 3's Sync-from-Code protocol: detect `git diff
  HEAD` changes to code, ask the LLM to summarise intent, propose
  corresponding spec updates, show the user, apply on approval.
- Pure reconciliation — never rewrites code to match stale spec; that
  direction is `vibe build` territory.

### M1.5.5 — Working example

- `stack:rust-cli@0.1.0` (hand-written) published to the registry.
- `feat:welcome-page@0.1.0` (hand-written).
- `vibe init → install stack:rust-cli → install feat:welcome-page →
  build feat:welcome-page --stack rust-cli` produces a running Rust
  CLI that prints a welcome page. This is the M1.5 demo.

### M1.5 acceptance (from §16)

- [ ] LLM provider abstraction supports Anthropic + OpenAI +
      OpenRouter + Ollama.
- [ ] `vibe build` produces working code from `feat:welcome-page ×
      stack:rust-cli`.
- [ ] Generated code has `Implements: spec://…` markers.
- [ ] Build subgraph respects `user-confirm` before mutation.
- [ ] `vibe sync` produces a clean spec-delta proposal from a code
      change.
- [ ] Tool-use loops are sandboxed to project root.
- [ ] LLM API errors surfaced clearly.
- [ ] LLM costs reported in the build's structured output.

**Estimated effort.** 3–6 weekends. Tool-use loops need real-world
hardening — the first working version is not the shippable version.

---

## M2 — Production-readiness

**Thesis.** Everything needed for someone other than the author to
use vibevm safely. Up through M1.5, the author is the only user and
"it works on my machine" is acceptable. M2 closes that gap.

### M2.1 — LLM-based install review

- `install:review` stops being a no-op. Before applying writes, the
  LLM reviews the fetched package contents and emits a safety
  analysis: does this look benign? does it try to exfiltrate
  anything? is it doing something inconsistent with what the
  manifest claims?
- The user sees both the mechanical plan and the semantic review
  before confirming. If the review flags a concern, confirmation
  requires an explicit `--accept-review` flag (never silent).

### M2.2 — Plugin contribution model v2

- Packages gain the ability to contribute actual graph nodes, not
  just files. A `flow:wal` package gets to register a
  `wal:checkpoint` node that runs automatically after
  `build:compile`. This is the point where `vibe-graph` earns the
  runner sophistication §5.2 hints at.
- Tooling to author and test contributed nodes.
- Type-checking at graph-build time gets teeth — type mismatches
  reject the graph with an `EXIT 4` before any mutation runs.

### M2.3 — Private registries

- Token-based authentication for `[registry]` URLs. `api_key_env`
  pattern extended to `token_env`.
- Per-registry cache keys so tokens don't leak across registries.

### M2.4 — Cross-platform CI

- GitHub Actions (or equivalent on GitVerse if available) matrix:
  macOS / Ubuntu / Windows, stable Rust.
- Pre-built binaries per platform on tag. Homebrew formula.
  Scoop manifest for Windows.

### M2.5 — Error-message polish

- Every user-facing error carries: what went wrong, where (file +
  line if applicable), and what to do about it.
- `vibe doctor` — inspects a project and reports common issues: WAL
  staleness, orphan anchors, missing implements-markers, registry
  cache older than N days.
- Colour/glyph output refined with a `--no-color` escape hatch.

### M2.6 — Structured telemetry (optional)

- Opt-in (`[telemetry] enabled = false` by default). Reports crash
  frequencies and common error paths. Gives the author signal on
  what to harden next.

**No M2 acceptance list in the spec** — §11.4 says "open-ended;
depends on adoption signals." Treat M2 as a rolling quality bar.

### M2.7 — `vibe review --optimize` and multi-model comparison

**Thesis.** Once `vibe-llm` exists (M1.5.1), the static review surface from M1.8 grows two LLM-driven extensions: a judge for the implementation and activation axes, and a `--optimize` auto-edit loop analogous to Tessl's. Plus per-model A/B comparison (`--agents=<m1>,<m2>`).

Source: [PROP-004 §5.2](spec/research/PROP-004-tessl-comparative-research.md#quality-evaluation) (LLM-judge portion), [§5.6](spec/research/PROP-004-tessl-comparative-research.md#multi-model).

**Estimated effort.** 2 weekends on top of M1.5.1 + M1.8.

### M2.8 — Lazy-push / lazy-pull runtime plumbing

**Thesis.** PROP-003 r2 already lands the three delivery modes (eager / lazy-push / lazy-pull) in the manifest schema and the lockfile from day one — so they don't require a v3-to-v4 lockfile migration later. What M2.8 covers is the **runtime side**: making lazy-push and lazy-pull actually do something, by plumbing them through `vibe-mcp` (M1.7) so the agent sees content at the right moment.

Source: [PROP-003 §2.5.0](spec/modules/vibe-resolver/PROP-003-dep-evolution.md#delivery-modes), [PROP-004 §5.4](spec/research/PROP-004-tessl-comparative-research.md#three-modes).

**Scope.**

- `vibe-mcp` (M1.7) gains lazy-push: when an agent's `query_package` reveals a lazy-push subskill whose `description` matches the agent's task, materialise into MCP context (not on disk).
- `vibe-mcp` gains `read_subskill` / lazy-pull: agent-driven on-demand fetch of subskill content. No materialisation; the bytes only ever live in agent context.
- `vibe-install` continues to materialise eager subskills as today; lazy-push and lazy-pull installs leave nothing on disk (just register the subskill with `vibe-mcp`'s pool).
- Acceptance: a `delivery=lazy-push` subskill installed against a Claude Code session never appears in `spec/...` but is observably present in the agent's context when the trigger description matches.

**Estimated effort.** 2 weekends. Depends on M1.7.

### M2.9 — Scenario generation from real commits

**Thesis.** Tessl's most architecturally distinctive primitive: scenarios generated from a project's actual git history, not synthetic tests. Once `vibe-eval` exists (M1.8) and `vibe-llm` is real (M1.5.1), `vibe scenario generate <repo> --commits=<...>` reads diffs, generates `task.md` + `criteria.json` + `scenario.json` triples, runs them as evals.

Source: [PROP-004 §5.5](spec/research/PROP-004-tessl-comparative-research.md#codebase-readiness).

**Scope.**

- `vibe scenario generate <repo> --commits=<sha1>,<sha2>` and `--prs <num1>,<num2>` modes.
- `vibe scenario download / view / list` analogous to Tessl's CLI surface.
- Format pinned to be drop-in compatible with Tessl's scenario layout (`task.md` + `criteria.json` + `scenario.json` per scenario directory) so cross-tooling is possible.
- `vibe eval run ./evals/` runs scenarios, scores, reports per-criterion + aggregate + cost-per-model.

**Estimated effort.** 4–6 weekends. Depends on M1.5.1, M1.8.

### M2.10 — `vibe search` registry inspector — ✅ SHIPPED (2026-05-22)

**Thesis.** With ~3 packages today, `vibe install` is fine; with 100+ it won't be. Tessl ships `tessl search` as a registry-side feature; vibevm's decentralised model needs an index to make search tractable at scale.

Source: [PROP-004 §5.12](spec/research/PROP-004-tessl-comparative-research.md#search), [PROP-005](spec/modules/vibe-index/PROP-005-package-index.md).

**Shipped.**

- `vibe search <query>` queries each configured registry's PROP-005 index (`IndexClient::search` against `/v1/packages?q=`); `--full-scan` falls back to the indexless registry walk for registries without an index.
- `vibe search --purl <purl>` answers "which packages describe this upstream library?" via the index's `by-purl` lane.
- Results cached in `~/.vibe/search-cache/`.

The index that makes this tractable is [PROP-005](spec/modules/vibe-index/PROP-005-package-index.md), implemented alongside — the `vibe-index` server + CLI in `crates/vibe-index/`, and the consumer-side `IndexClient` fast path in `vibe-registry`.

---

## M3+ — Speculative directions

None of these are funded. They are listed so the M0 / M1 / M1.5 /
M2 decisions keep these futures open rather than foreclosing them.

- **Interpret mode.** `vibe run <feat-pkgref>` executes the spec
  directly via an LLM runtime — no code generation. Useful for
  one-shot scripts and for exploring a feat before committing it to
  a stack.
- **Multi-stack composition.** One feat compiled for multiple stacks
  simultaneously (e.g. a UI feat for web + mobile). Requires the
  stack abstraction to be richer than the current §4.1.
- **Skill layer.** Distributable Claude Code / Codex / OpenCode skills
  that wrap the CLI for native slash-command access, so users don't
  have to leave their editor.
- **Hosted registry.** Replace git-as-registry with a proper package
  registry server: metadata index, search, signed publishes, a web
  UI. Only worth building if the community shape signals it.
- **vibevm registry explorer.** A browsable visualisation over the
  per-org index (PROP-005): a reverse-FQDN group tree with drill-down,
  Maven-Central-style — and richer. Beyond Maven Central: filter by
  `kind`, a capability graph (`[provides]`/`[requires]`),
  `describes`/PURL links to upstream libraries, redirect-stub
  delegation, the full dependency DAG, and workspace provenance
  ("sub-package of X", from PROP-007's `[origin]` marker). A separate
  optional layer over the index — PROP-005 §2.10 already reserves the
  hook (`vibe-index serve`, CORS-open read endpoints). Recorded in
  [PROP-008 §2.9](spec/modules/vibe-registry/PROP-008-qualified-naming.md).

### M3.1 — Security review threat model (research-only)

**Thesis.** Standard package-manager security (CVE feeds, dependency-vulnerability scanners) doesn't fit vibevm's surface — packages are spec-content, not arbitrary code. The threat surface is *prompt injection / capability misrepresentation / data-exfiltration via subtle wording*. Needs a research slice to define the threat model before any scanner can be built.

Source: [PROP-004 §5.9](spec/research/PROP-004-tessl-comparative-research.md#security).

Not action-eligible. Park as research; revisit when adoption surfaces real threats.

---

## Side quests (independent of milestones)

These are small-to-medium polish items that are not on the critical
path of any milestone. Take them when a session has 30–60 free minutes
and you want to close a loop that's bugging you.

- **`.gitattributes`** with `* text=auto eol=lf`. The M0 commits
  produced 70+ "LF will be replaced by CRLF" warnings because the
  repo doesn't pin a line-ending policy. Left unchecked this
  eventually causes content-hash drift on Windows. Fix once, forget.
- **`git config gc.auto 0`** on the repo. The book (chapter 4) warns
  that Git's automatic garbage collector can fire mid-session and
  corrupt worktree indexes. Disable auto-gc and document a manual
  `git gc --prune=now` after each big commit burst.
- **Workspace README.md.** A top-level README explaining what vibevm
  is, how to build, where to start reading, how to contribute. Right
  now the project has `VIBEVM-SPEC.md` (spec) and `ROADMAP.md` (this
  file) but nothing for a first-time visitor landing on the repo
  page.
- **CHANGELOG.md.** Conventional Commits make this trivially
  generable. Nice for M1 onward when external users start tracking
  versions.
- **Clippy lint set promotion.** Upgrade `clippy::all` to `-D` (deny)
  and pick a tighter lint set (`clippy::pedantic` selectively) for
  the library crates. Warnings-as-errors in CI.
- **`cargo deny` in CI.** Licence-check automated: fail the build if a
  dep with a non-permissive licence sneaks in. Matches PROP-000 §3's
  "permissive only" rule.
- **Docs site.** Eventually `https://gitverse.ru/anarchic/vibevm` is
  enough — but once user-facing docs exist under `docs/`, render them
  through mdBook or Zola so URIs are clickable.

## Known outstanding review items

Nothing active. Historical:

- `vibe-install/src/lib.rs` carried a REVIEW about mirror package
  layout — resolved 2026-04-17 by pinning the convention in
  `VIBEVM-SPEC.md` §13.1 and in `PROP-000` §13.

---

## Cadence and review

- **Per milestone:** walk the acceptance checklist in §16 of the
  spec. If any item fails, fix before claiming completion. Tag the
  release (`v0.1.0-m0`, `v0.1.0-m1`, etc.) and update `spec/WAL.md`
  to reflect the new "Current phase."
- **Per session:** read `CLAUDE.md`, then `spec/WAL.md`, then the
  relevant PROP/FEAT for the task at hand. Update the WAL at session
  end. Commit in grouped units per `CLAUDE.md` Rule 3.
- **Per week:** re-read the spec sections relevant to the active
  milestone. Catch drift before it hardens.

---

*End of roadmap.*
