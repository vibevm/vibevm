# CONTINUE — cold-resume checkpoint

_Written: 2026-05-07 (slice 5 land). Owner-readable, self-contained. Pick this up with zero prior context._

---

## TL;DR (executive summary)

**M1.7 slice 5 landed end-to-end — vibevm now ships a bootstrap-capable, scope-aware, lifecycle-complete MCP integration**. Six commits this session on top of the slice-4 foundation, all pushed to `origin/main`. Working tree clean.

What changed (and why slice 5 was needed):

The slice-4 SKILL.md assumed the agent was always inside an existing vibevm project. Skill installation required `vibe.toml`, project-scope was the only first-class path, and an agent invited to "create a vibevm project" had no actionable guidance — exactly the chicken-and-egg slice 5 closes. Now:

1. **Two axes for everything** — `--scope project|user|both` × `--what mcp|skill|both`. Every install / upgrade / uninstall is a (scope, what) decision matrix. Wizard asks the three questions (Scope / What / Agents); explicit flags skip what's already known.
2. **User-scope is the bootstrap path.** `vibe mcp install --scope user` works without `vibe.toml`. Writes user-level MCP config and (optionally) user-level SKILL.md. The MCP entry omits `--path` so the server resolves CWD per invocation — one global config serves every project the operator ever opens.
3. **Two-state SKILL.md.** Vendored body has Section A (bootstrap, no project — run `vibe init`, install starter packages, transition to Section B) + Section B (inside existing project, follow boot protocol) + Common (MCP tools, `--invoked-by`, `vibe --help` discipline, four rules). Frontmatter description triggers on bootstrap intents too.
4. **Lifecycle complete.** `vibe mcp upgrade` refreshes stale installs to current binary (does NOT create new ones). `vibe mcp uninstall` removes vibevm with foreign-key preservation. `vibe mcp status` extended with skill-drift report.

End-to-end demo (the slice-5 contract): operator runs `vibe mcp install --auto --scope user` once on a clean machine, then in any directory says to opencode "create a vibevm hello-world project". opencode's loaded skill recognises Section A, runs `vibe init`, installs `flow:wal`, materialises hello-world artefacts, updates `spec/WAL.md` per the WAL protocol — all without the operator re-explaining vibevm.

Workspace state at HEAD (`f068a21` for code, plus `35cad9f` SKILL.md, `55d22d9` docs, plus C10 commits this final pass):

- **vibe-cli at 174 hermetic + 3 ignored** (+16 since slice 4's 158).
- `cargo test --workspace` all green across all crates (services/vibe-index included).
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
- `vibe check --path . --quiet` reports `0 errors, 0 warnings, 0 info` (self-host).
- Working tree clean.

Push to `git@gitverse.ru:anarchic/vibevm.git` is current after this session-end commit.

---

## Where we are right now

- **Branch:** `main`. Working tree clean.
- **Latest commits this session (slice 5, newest first):**

  ```
  55d22d9 docs(commands,guides): refresh mcp-* docs + opencode quickstart for slice 5
  35cad9f docs(vibe-cli/mcp): SKILL.md two-state — bootstrap + inside-project
  3c7fced feat(vibe-cli/mcp): vibe mcp status — include skill drift report
  08f8260 feat(vibe-cli/mcp): vibe mcp uninstall — drop vibevm block + delete SKILL.md
  f068a21 feat(vibe-cli/mcp): vibe mcp upgrade — refresh stale installs to current
  3f0e517 feat(vibe-cli/mcp): scope=project|user|both + what + bootstrap mode
  ```

- **Active blocker:** none. Slice 5 landed clean; tests + clippy + self-host check all green.

---

## What to do first in the next session

Pick whichever matches the owner's interest:

### Option 1 — walk the new opencode quickstart on a clean sandbox

`docs/guides/agent-mcp-quickstart-opencode.md` was rewritten under the slice-5 bootstrap flow. The acceptance checklist now pins user-scope MCP without `--path`, two-state SKILL.md with explicit Section A/B headers, agent self-running `vibe init` from a Section-A prompt. Walking it confirms: bootstrap works, agent uses MCP autonomously, lifecycle commands wire correctly.

This is the lowest-risk first step — confirms slice 5 health AND that the documented contract holds.

### Option 2 — plan-preview + apply-confirm prompt

Currently the install wizard runs through Scope → What → Agents and proceeds straight to apply. The slice-5 flag set has `--yes` defined but no opposite "ask before apply" interactive step yet. Would-be flow: after wizard answers + agent detection, render a plan summary (table of what would land where), then `dialoguer::Confirm` apply prompt. Skip with `--yes` / `--auto`. Same shape as the (currently planned but not yet wired) upgrade / uninstall plan-preview flows.

Code anchor: `crates/vibe-cli/src/commands/mcp.rs` `run_install` after step 5 (the `for agent in &targeted` loop), before the actual `apply_install_mcp` calls.

### Option 3 — `query_capabilities` / `list_subskills` MCP tools

The vibe-mcp server exposes three tools (`query_package`, `read_subskill`, `materialise_subskill`). PROP-004 §5.1 also calls out `query_capabilities` and `list_subskills`. Wiring is straightforward, pattern after existing tools at `crates/vibe-mcp/src/tools.rs`. Update SKILL.md template (`crates/vibe-cli/src/commands/skill_template.md`) to mention the new tools so the agent learns to use them.

### Option 4 — extend agent matrix to Gemini / Copilot

`Agent` enum in `crates/vibe-cli/src/commands/mcp.rs:178` has the per-agent profile slot ready. Add new variants the same way slice 4 added Claude Desktop / OpenCode / Codex:

1. Add the variant to enum + `Agent::ALL`.
2. Fill in all per-agent methods including `config_path(scope, project_root)` for both project and user scopes (or `None` if a scope has no surface).
3. Add presence markers + host_present probe.
4. Mirror unit-test patterns (look at `opencode_user_scope_entry_uses_command_array_without_path`, `codex_user_scope_entry_returns_toml_table_without_path`).
5. Update `docs/commands/mcp-install.md` agent matrix table.
6. Add a sibling guide `docs/guides/agent-mcp-quickstart-gemini.md`.

### Option 5 — comment-preserving Codex TOML edits via `toml_edit`

Currently `merge_toml` and `strip_toml_entry` use `toml = "0.9"` round-trip via `toml::Value`. This loses comments in handcrafted `~/.codex/config.toml`. If a Codex operator complains, swap to `toml_edit` — it's a permissive-licensed crate that preserves whitespace + comments.

### Option 6 — Manual smoke walk

`manual-tests/M1.7-mcp-claude-code-smoke.md` was envisioned in M1.7 ROADMAP but never written. With slice 5 closed, the ground truth is `docs/guides/agent-mcp-quickstart-opencode.md` — its acceptance checklist serves as the smoke. Decide whether a separate manual-test file adds value beyond the guide.

---

## Non-obvious findings from slice 5

These cost time / hit edge cases — write them down so a future session does not re-derive.

### `dialoguer::Select`/`MultiSelect::items` accepts owned arrays, not slice references

Clippy under `-D warnings` flags `&["a", "b"]` syntax. Use array literals directly: `.items(["a", "b"])`. Caught me mid-C1.

### `--what` is install/upgrade-only; uninstall uses `--config-only` / `--skill-only`

Tried to use `--what mcp` in uninstall tests — clap rejected it. Decided to keep uninstall's `--config-only` / `--skill-only` (mutually exclusive booleans) instead of duplicating `--what`. The orthogonal toggle reads naturally for the "remove only X" intent ("uninstall the skill but keep the MCP server connection"). Install kept `--what` because "install only X" is a more declarative phrasing.

### Scope::Both is internal-only; expand before per-agent calls

`Agent::config_path(Scope::Both, ...)` and `Agent::skill_path(Scope::Both, ...)` both `bail!` — Both is a wizard / CLI surface, not a per-agent concept. The walker in `run_install` / `run_upgrade` / `run_uninstall` calls `scope.expand()` first, which yields `[Project, User]` for Both and `[s]` for the others, then iterates per concrete scope. Keeps the per-agent methods tight (always concrete) without losing the Both-mode UX.

### User-scope MCP entry MUST omit `--path`

Slice-4 mistake replicated would have hardcoded `--path <whatever was in args.path at install time>` into user-level configs. That's wrong: user-level config should serve every project the agent ever opens, not the directory where install ran. `Agent::build_mcp_entry(Scope::User, _)` builds `["mcp", "serve"]` (no `--path`). The MCP server's `--path` defaults to `.` so it resolves CWD per invocation. Per-agent test pins this contract for each (agent, scope) combination.

### `Agent::config_path` returns `Option<PathBuf>`, not `Result<PathBuf>`, when the scope has no surface

User-only agents (Claude Desktop, Codex) return `None` for `Scope::Project`. Project-only walks must filter on the `is_some()` to skip — the walker in `run_install` does this. The Both-mode walker emits a `skipped` row for the missing leg so JSON consumers see what was attempted. **Don't** treat `None` as an error — it's an expected "this combination is undefined" signal.

### `vibe mcp install --auto` without `--scope` auto-resolves from `vibe.toml` presence

`--auto` alone (no other flags) walks: scope=project if `vibe.toml` in `--path`, else scope=user; what=both; agents=detected. The "auto-resolves to whatever makes sense" behaviour is what makes `vibe mcp install --auto` the safe one-line bootstrap on a clean machine. Without this, scripts would have to either always pass `--scope user` (clunky for inside-project operators) or always pass `--scope project` (breaks bootstrap).

### Output env-guard tests still flake under parallel `cargo test`

Slice-4 known issue, persisted into slice 5: `output::tests::resolve_treats_empty_*_as_absent` set / unset `VIBE_INVOKED_BY` env-var without coordination. Under parallel test execution they sometimes observe each other's transient mutations. `--test-threads=1` makes them deterministic; standard `cargo test` usually passes (race rare in practice). If it flakes in CI, run with `RUST_TEST_THREADS=1` for the output module specifically — or migrate to a process-isolated test harness.

### `mcp upgrade` distinguishes `not-installed` from `unchanged` deliberately

A naive upgrade would treat "vibevm-block absent" as a candidate for install. Slice-5 explicitly separates this — `not-installed` is a hint to use `vibe mcp install`, not an action to take. Keeps cron-style `vibe mcp upgrade --yes` safe: it never auto-promotes installs into agents the operator deliberately skipped.

### Detection in upgrade is best-effort, not all-or-nothing

`vibe mcp upgrade --scope both` with no `vibe.toml` in CWD silently skips the project leg (rather than erroring out as install would). Refresh should be best-effort: if you have user-level installs and project-level installs in different repos, you can run upgrade from anywhere and refresh whichever scope has surfaces here. Install is stricter because creating new installations is a destructive opt-in.

---

## Repository map

```
vibevm/
├── CLAUDE.md / AGENTS.md / GEMINI.md   # Three identical copies of the four rules.
├── CONTINUE.md                          # This file. Cold-resume snapshot.
├── ROADMAP.md                           # Milestone-oriented plan; M1.7 closed via slice 5.
├── VIBEVM-SPEC.md                       # Owner-frozen spec; do not edit without explicit instruction.
├── DEV-GUIDE.md / RUNTIME-GUIDE.md      # Per-machine setup docs.
├── crates/
│   ├── vibe-cli/                        # `vibe` binary entry point. clap dispatch + per-subcommand modules.
│   │   └── src/commands/
│   │       ├── mcp.rs                   # Slices 4+5 home: 5-agent matrix, scope/what axes, install/upgrade/uninstall/status, JSON+TOML mergers, install_skill writer.
│   │       └── skill_template.md        # Vendored two-state SKILL.md (Section A bootstrap + B inside-project + Common).
│   ├── vibe-core/                       # Manifests (vibe.toml, vibe-package.toml), lockfile schema v3, user_config.
│   ├── vibe-graph/                      # In-memory dep graph helpers.
│   ├── vibe-registry/                   # GitPackageRegistry, mirrors, MultiRegistryResolver, IndexClient.
│   ├── vibe-resolver/                   # Feature expansion + activation evaluation (PROP-003).
│   ├── vibe-install/                    # Install pipeline: plan_install → apply → register.
│   ├── vibe-llm/                        # LLM provider abstraction. Skeleton — real impls land in M1.5.
│   ├── vibe-mcp/                        # JSON-RPC MCP server. 3 tools today.
│   ├── vibe-check/                      # Spec-consistency linter.
│   ├── vibe-publish/                    # GitHubCreator / GitVerseCreator / DirectGitCreator publishers.
│   └── vibe-wire/                       # JTD-codegen'd wire types.
├── services/
│   └── vibe-index/                      # Standalone PROP-005 utility: per-org package index. Own Cargo workspace.
├── spec/
│   ├── boot/{00-core,90-user}.md        # Read at every session start.
│   ├── WAL.md                           # Living checkpoint of project state. Authoritative if it diverges from this file.
│   ├── common/PROP-000…PROP-006         # Foundation policy + operating modes.
│   ├── modules/                         # Per-crate PROPs.
│   └── research/PROP-004                # Tessl comparative research.
├── docs/
│   ├── README.md                        # User-doc index.
│   ├── architecture.md / lockfile-format.md / glossary.md / troubleshooting.md
│   ├── commands/                        # Per-subcommand reference. mcp-install/upgrade/uninstall/status/serve.md.
│   ├── guides/                          # Long-form walkthroughs. agent-mcp-quickstart-opencode.md (slice-5 rewrite).
│   └── authoring-{flow,feat,stack}.md
├── manual-tests/                        # Runnable smoke protocols.
├── fixtures/registry/                   # Hermetic per-package registry fixtures.
├── tools/                               # self-check.sh + jtd-codegen install README.
└── xtask/                               # `cargo xtask codegen` / `check-codegen`.
```

---

## Architectural / policy decisions still in force

In rough order of how often they bite a fresh contributor:

1. **Four non-negotiable rules** ([PROP-000 §12](spec/common/PROP-000.md#commits)):
   1. **No AI / machine-author attribution** anywhere.
   2. **Conventional Commits.** Subject ≤ 60 chars (hard limit 72), body explains WHY.
   3. **Group commits by meaning**, never by file or by time.
   4. **Autonomy on routine changes.** Non-routine red lines (history rewrite, `--force` push, large blobs, CI / signing / secrets, irreversible ops) STILL require explicit owner sign-off.

2. **Memory discipline.** Project facts live in the repo. Per-machine facts only live in tool-specific user-memory.

3. **Vocabulary lock.** Only `flow`, `feat`, `stack`, `tool`. Never `lifecycle` / `phase` / `goal` / `plugin`.

4. **Language: Rust.** Permissive licenses only. `dependency weight is not a decision factor` per PROP-000 §15.

5. **Manifest format: TOML for human-edited; JTD+codegen for wire contracts.**

6. **Identity: `(kind, name, version, content_hash)`.** URL is informational.

7. **Token secrecy** (PROP-000 §20). Never printed in any vibevm-produced output.

8. **Repository hosts.** vibevm source = GitVerse. Package registry = GitHub `vibespecs`.

9. **User-owned files** (vibevm install/uninstall NEVER touches): `spec/boot/00-core.md`, `spec/boot/90-user.md`, `spec/WAL.md`, `VIBEVM-SPEC.md`, `refs/book/**`.

10. **PROP-006 codewords.** `«move fast and break things»` is the first; never overrides the four rules.

11. **Slice 5 MCP integration model.** `--scope project|user|both` × `--what mcp|skill|both` is the canonical UX. `--scope user` is the bootstrap path that does NOT require `vibe.toml`. Install creates new installations; upgrade refreshes existing only; uninstall removes with foreign-key preservation. SKILL.md is two-state (Section A bootstrap, Section B inside-project) so the global / user-scope skill works in both contexts.

---

## Recent commit chain (last 25, newest first — slice 5 + slice-4 tail)

```
55d22d9 docs(commands,guides): refresh mcp-* docs + opencode quickstart for slice 5
35cad9f docs(vibe-cli/mcp): SKILL.md two-state — bootstrap + inside-project
3c7fced feat(vibe-cli/mcp): vibe mcp status — include skill drift report
08f8260 feat(vibe-cli/mcp): vibe mcp uninstall — drop vibevm block + delete SKILL.md
f068a21 feat(vibe-cli/mcp): vibe mcp upgrade — refresh stale installs to current
3f0e517 feat(vibe-cli/mcp): scope=project|user|both + what + bootstrap mode
bc26131 docs(wal): session-end checkpoint — slice 4 + opencode quickstart guide
492cbb2 docs(continue): cold-resume checkpoint at 2026-05-07 session-end
3bf2462 docs(guides): opencode + vibevm hello-world quickstart + acceptance gate
7cb1f33 docs(commands,roadmap,wal): M1.7 slice 4 — multi-agent + skill + invoked-by
71229eb feat(vibe-cli/mcp): interactive install + --auto + --with/without-skill
d384a96 feat(vibe-cli/mcp): vibevm SKILL.md template + per-agent writer
2eaf544 feat(vibe-cli): --invoked-by global flag + VIBE_INVOKED_BY env
05ce2e4 feat(vibe-cli/mcp): claude-desktop, opencode, codex + JSON/TOML mergers
8ce7b6a docs(commands): refresh vibe search reference for purl/full-scan/cache
e4000c3 feat(vibe-cli): vibe search --purl + --full-scan + persistent cache
7745b19 feat(vibe-registry): IndexClient::lookup_purl + Serialize on results
c585437 docs(commands): vibe search reference
506dcf2 feat(vibe-cli): vibe search command (ROADMAP §M2.10)
622ea55 feat(vibe-registry): IndexClient::search via /v1/packages?q=
c54fa51 docs(wal): rate-limiter slice + parked §9 open questions
039bd96 feat(services/vibe-index): per-token + per-IP rate limiter
ae990ae docs(wal): PROP-005 trailing-fixup slices 16–19
867ab97 feat(services/vibe-index): structured stub envelope for --from-gitverse
6e7487d feat(services/vibe-index): init writes README.md + .gitignore
```

---

## Quick-start commands

```powershell
# Build everything.
cargo build --workspace

# Full test gate (matches CI).
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p vibe-cli -- check --path . --quiet

# Or one-shot via the bundled script.
bash tools/self-check.sh

# Install vibe into ~/.cargo/bin/ (recommended for any agent integration walk).
cargo install --path crates/vibe-cli --locked

# Slice-5 bootstrap demo: from any directory, no project needed.
vibe mcp install --auto --scope user --invoked-by manual-bootstrap
# Then in a fresh empty directory:
opencode    # tell it: "create a vibevm hello-world project"
# See docs/guides/agent-mcp-quickstart-opencode.md for full walk.
```

---

## Pointer

`spec/WAL.md` is the canonical **living** checkpoint. If anything in this `CONTINUE.md` disagrees with the top of `spec/WAL.md`, trust the WAL — it gets bumped every session.
