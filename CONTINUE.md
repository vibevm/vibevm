# CONTINUE — rolling cold-resume checkpoint, 2026-08-26

> Canonical living state is `vibevm/vibespecs/WAL.xml`; if this snapshot and
> the WAL diverge, the WAL wins. The granular campaign truth is
> `campaigns/packages-2026-09/LIFECYCLE-EXTENSIONS-IMPLEMENTATION-LEDGER.md`.

## TL;DR

The lifecycle/extensions epic is active and far from complete, but no landed
implementation was lost during the multi-hour network outage. R1 and R2 are
complete. R3.1/R3.2, R7.1 and R8.1 are on `main`. Four implementation worktrees
are now active in parallel: R3.3 verifier, compiler-IR JTD wire, R7.2 CLI agent,
and R8 artifact/mechanism grammar. Root owns review, commits and integration.

The old WAL/CONTINUE/TASKS/TZ header incorrectly said implementation had never
started. This recovery batch repairs them and creates a durable reconciliation
ledger. At the audit base `b4bfe6bd`, `main` was ten commits ahead of both
mirrors; mirror immediately after the current unchanged full panel is green.

## Current repository state

- Branch: `main`.
- Audit base: `b4bfe6bd docs(agent): trust workers and standardize handoff`.
- Both `origin/main` and `github/main` were at `302a3509`; `main` was ahead 10
  before this recovery checkpoint's docs commits.
- Host tracked tree was clean; `cache/` is untracked and unignored.
- `cargo test --workspace --locked`: green after repairs `ae1a90af` and
  `7ac27240`.
- `cargo xtask specmap --check`: 6769 units / 1670 code items / 1526 edges /
  0 suspects / 0 gated orphans / 0 unresolved host edges / 21 standing warnings.
- Full panel: not yet green for this final tree. Two attempts stopped inside
  workspace tests and found real integration defects; both exact reds are now
  green. Run the next panel only after this docs checkpoint is committed.
- Health audit: the durable active finding subset is in `AUDIT.md`; do not
  mirror it here.
- Judging debt is unmeasured. Host `judging-debt.py` says 19,496 facts,
  0 unjudged, 0 orphaned, 35 stale — but from pre-relayout mirror/cache paths;
  a detached worktree sees no mirrors. Direct census proves all 502 judged paths
  moved: 98 only through the root relayout and 404 also `.md`→`.xml`.
  `text-stability.py` mislabels that as hash-recipe drift. Resolve B-107, migrate
  paths/verdicts with anchor equality, regenerate mirror, then measure again.

Re-measure rather than trusting these numbers after further work:

```powershell
git status --short --branch
git rev-list --left-right --count origin/main...main
python campaigns/packages-2026-09/tasks/judging-debt.py
cargo xtask specmap --check
```

The current judging command is diagnostic of the old state only until that
migration lands; do not quote its zero as a live-corpus verdict.

## Active worktrees / processes

| Worktree | Task | Worker/handoff |
|---|---|---|
| `.wt/r3-verifier` | R3.3 immutable verifier-each | `claudez`; unstaged diff + `cache/agents/R3.3-IMPL/…report.md` |
| `.wt/r6-ir-wire` | six-carrier JTD compiler IR schema/corpus/generated wire | `claudez`; no `vibe-spec` writes |
| `.wt/r7-agent-cli` | R7.2 CLI agent/output contract | Opus 5/max |
| `.wt/r8-mechanism-grammar` | pure manifest grammar/validation for mechanisms/artifacts/deploy profiles | `claudez` |
| `.wt/opus-campaign-audit` | read-only completeness audit | Opus report needs root corrections before acceptance |

Every writing worker may use normal tools but must leave product state as an
ordinary unstaged diff. Root reads the diff/report, reruns decisive tests,
issues same-cwd `-c` repair when needed, then stages/commits.

## What is landed

- R1: slot record, diff materialisation, mutable hash skip, exact reconcile
  report, verify-heal and hook-on-nonempty-diff.
- R2: full non-agent lifecycle engine, five handler grammar, two world epochs,
  phase/slot execution, freshness, controls/order, script/binary, presets,
  query surface and owner scenario §10.1.
- R3.1/R3.2: explicit Source/Document/Documents/Closure/Lane/Emitted carriers,
  named schedule through real emit backends, one whole artifact and crash-safe
  boot publication.
- R4 prerequisites only: strict XML minify kernel and reversible generated XML
  comment codec. No production transform binding.
- R7.1: real OpenAI-compatible provider seam, JTD request/response, layered
  config and endpoint/redirect/proxy/body/timeout/redaction gates.
- R8.1: automatic project-scope skill package binding, strict JTD ownership
  receipt, durable intent/recovery/CAS, link-safe mutation and project
  projections for Claude/Codex/OpenCode.
- Build/package/deploy architecture and every owner addition are committed in
  `BUILD-PACKAGE-DEPLOY-ARCHITECTURE-v0.1.md` and the R7/R8 spec-debt queue.

Everything else stays open. Do not collapse “pure kernel/type grammar exists”
into “wave complete”; use the ledger's row-by-row evidence.

## Exact next-step recipe

1. Validate this recovery-doc atom:

   ```powershell
   $null = [xml](Get-Content -Raw vibevm/vibespecs/WAL.xml)
   cargo xtask specmap
   cargo xtask specmap --check
   git diff --check
   ```

2. Commit the ledger/TZ/TASKS and WAL/CONTINUE in separate topic commits.
3. Run the current full panel through Git Bash:

   ```powershell
   $env:CARGO_BUILD_JOBS='4'
   & 'C:\Program Files\Git\bin\bash.exe' tools/self-check.sh
   ```

4. Prove a no-transform boot regeneration is a no-op and run host `vibe check`.
5. `cargo xtask mirror` the accumulated batch; stop on any non-fast-forward.
6. Poll the four active worktrees, review them as PRs and integrate in this
   order: verifier → IR wire → agent/manifest atoms (their Cargo/format-registry
   overlaps are resolved centrally).
7. Commission a bounded B-107 campaign-state migration: include package XML,
   map all 502 old paths to live paths, prove every verdict anchor survives,
   then regenerate mirror/cache and re-run both debt instruments.
8. Continue the three lanes exactly as the implementation ledger §7 prescribes.

## Active blockers and owner action

There is no owner-decision blocker. The earlier apparent choices have been
closed from accepted project authority:

- R3.4 uses the pulled-forward R6 compiler-IR JTD wire, not a handwritten
  trace DTO; workspace owns run directory/id and artifact subdirectories.
- R4 extracts the pure registry below lifecycle/workspace and uses the accepted
  plural `[[extensions.use]]` activation/order.
- R7 config merge is already fixed in the R7/R8 spec-debt continuation; hosted
  resume adds a durable run id to existing lifecycle state.
- R8 follows the committed mechanism/artifact/profile design; future VibeVM OS
  resources are compatibility constraints, not current implementation scope.

If `claudez`/Opus hits a five-hour limit, retain its worktree/report and finish
the task in root or another launcher. Never restart the whole batch blindly.

## Non-obvious findings from this run

- `bash` in PowerShell resolves to the broken WSL shim on this box. Use the
  absolute Git Bash path above.
- The R3 boot transaction leaves a persistent `.vibe-boot-artifacts.lock` in
  the boot directory. `vibe-check` now recognizes exactly that filename; crash
  journals/stages and near misses remain warnings.
- The safe TOML diagnostic intentionally strips authored source lines because
  a manifest may contain secrets. The downstream oracle now requires parser
  diagnosis/position and asserts the raw line is absent.
- `--allowedTools` is auto-approval, not the tool universe. More importantly,
  the owner trusts Claudez/Opus with normal machine tools; protect the
  reviewable worktree handoff rather than starving search/shell/history.
- First Opus 5/max audits were run through ordinary Claude Code CLI, not a
  direct API. Fresh: `claude -p --model opus --effort max`; correction:
  same-cwd `claude -c -p`. JSON confirmed `claude-opus-5`, 1M context.
- Subscription launcher runs receive no dollar cap. Nominal JSON cost is
  telemetry, not marginal spend. Deep research is a separate expensive class.
- R3 verifier cycle semantics are per edge kind: contract-only Use/Source
  cycles can be legal; Embed cycles cannot; the union is not one recursion
  graph.
- Important old worker reports live under untracked `cache/`. Accepted
  architecture is now synthesised into the committed ledger; do not depend on
  cache as the sole memory.

## Architectural policy still in force

1. Lifecycle phases keep separate ownership: install dependencies, generate
   source, build code, test, create explicit agent outputs, verify, package
   portable artifacts, deploy destinations.
2. LLM capability is optional advanced functionality. Algorithmic paths stay
   complete; enhancements use explicit `off|assist|required`, lazy provider
   construction and visible usage budgets.
3. One extension/mechanism plane: qualified providers, deterministic routing,
   builtins replaceable, no language autodetection.
4. Desired target, produced artifact and applied deployment are separate
   records. External effects carry scope, plan, intent, independent verify and
   receipt; third-digest mutation refuses.
5. Static skills, Agent Plugins, classic Cargo apps, client installs,
   `~/.vibe/bin`, remote/server providers and future system package managers
   share the same artifact/provider model.
6. JTD is SSOT for every machine wire. Do not touch `formats/EPOCHS.toml` or
   hand-edit generated Rust.
7. R3/R4/R6 cardinality is per addressed document before gather and per final
   artifact afterwards. Compatibility fragment APIs do not define production
   cardinality.
8. Workers execute; root architects, reviews, stages, commits, integrates and
   runs final gates. Keep all repository attribution human-authored.

## Repository map

- `crates/vibe-core` — manifest/domain vocabulary and common paths/errors.
- `crates/vibe-workspace` — workspace/world/materialisation/boot artifacts,
  binaries and filesystem ownership.
- `crates/vibe-lifecycle` — phase plan, registry, state and handler execution.
- `crates/vibe-spec` / `vibe-specdoc` — compiler IR/passes/backends and source
  dialects.
- `crates/vibe-wire` + `schemas/` + `formats/` — generated machine contracts,
  registry and corpora.
- `crates/vibe-llm` — provider/config/transport seam.
- `crates/vibe-mcp` — MCP tools and skill/client projection library.
- `crates/vibe-cli` — thin command adapters and end-to-end fixtures.
- `vibevm/vibespecs/` — authoritative specs, boot lane and WAL.
- `campaigns/packages-2026-09/` — active TZ, ledger, spec-debt and campaign
  evidence.
- `distribution/windows/` — existing Windows distribution ingredients; not yet
  a lifecycle package mechanism.
- `.wt/` — root-provisioned isolated worker worktrees; many older snapshots
  remain intentionally until the epic's final loss audit.

## Recent commit chain at checkpoint

```text
b4bfe6bd docs(agent): trust workers and standardize handoff
7ac27240 fix(vibe-check): accept the boot transaction lock
ae1a90af test(vibe-check): assert redacted parser diagnostics
a9bfa6d0 chore(campaign): harden worker packet discipline
9275f373 feat(vibe-lifecycle): bind declared project skills
4403cb55 refactor(vibe-spec): emit whole artifacts transactionally
5f7c9ce2 docs(agent): codify claudez packet contract
518be400 docs(campaign): design build package and deploy
3ac08a3c docs(agent): scope ChatGPT campaign execution
e2392893 feat(vibe-llm): the provider seam gets a real provider
302a3509 refactor(vibe-spec): assemble one whole lane
c0fa49be refactor(vibe-skill): projection inventory leaves the CLI surface
f42334ff feat(vibe-wire): openai-compatible chat starts from schemas
6f3fa61a refactor(vibe-spec): make the artifact the compilation unit
fbbd5140 fix(boot): encode generated XML comments reversibly
3137990c test(cli): the phase-announcer plugin — the commissioning scenario is green
516b49b7 feat(cli): vibe extensions — the machine is a query surface
6de7ef05 refactor(vibe-spec): make linking composable
21ff9da7 feat(vibe-lifecycle): presets live in declared data
2feef271 refactor(vibe-spec): make absorption composable
e653654d refactor(vibe-spec): make qualification composable
ec7ea7fe refactor(vibe-spec): the embed phase becomes a pass
e53b9a4e refactor(vibe-spec): the merge phase becomes a pass
07941230 feat(vibe-lifecycle): every execution is skip-when-fresh
96eef07d refactor(vibe-spec): the close phase becomes a pass
```

## Quick start

```powershell
cargo test --workspace --locked
cargo xtask specmap --check
cargo xtask check-codegen
$env:CARGO_BUILD_JOBS='4'; & 'C:\Program Files\Git\bin\bash.exe' tools/self-check.sh
```
