# B047 — Census of host surfaces: where the logic is nailed to the CLI, where it lives in a shared crate

> This is a **census of evidence, not a verdict.** It measures where each
> top-level capability actually keeps its substance. What to do with the
> finding is the boss's call — there are no «build an MCP twin» conclusions
> below; that phrasing is a verdict and it is not this file's to make.

## The norm and why it is measured

Standing norm (`BACKLOG.md` B-047, owner critique 2026-08-02, verbatim):
«logic shared between MCP and CLI should be formulated abstractly in some
library or crate, so different surfaces reuse it.» A user capability lives
in a shared crate; the CLI is one thin surface over it, the MCP server is
another; neither is the «base.»

The stacks already hold the norm (logic in bridge/engine crates; the CLI
binary and the MCP server are two thin wrappers). **The host side is
unmeasured** — and this file is the measurement, not the construction.

**Perimeter measured (verified by reading, not re-counted from the brief):**

- **29 top-level commands** — every variant of `pub enum Command` at
  `crates/vibe-cli/src/cli.rs:95`. Dispatch is one `match` in
  `crates/vibe-cli/src/main.rs:105-229` (one arm per variant).
- **5 MCP tools** — `default_tools()` at `crates/vibe-mcp/src/tools.rs:44`,
  registering `QueryPackageMcpTool`, `ReadSubskillMcpTool`,
  `MaterialiseSubskillMcpTool`, `AgenticExplainMcpTool`, `ExplainMcpTool`
  (tool names `query_package`, `read_subskill`, `materialise_subskill`,
  `agentic_explain`, `explain`).
- **19 host crates** in `crates/` — `vibe-cli` plus 18 siblings; the ones
  `vibe-cli` actually links (`crates/vibe-cli/Cargo.toml`): `vibe-core`,
  `vibe-actions`, `vibe-graph`, `vibe-install`, `vibe-registry`,
  `vibe-resolver`, `progress-core`, `vibe-check`, `vibe-publish`,
  `vibe-workspace`, `vibe-mcp`, `vibe-settings`, `vibe-wire`, `vibe-spec`,
  `vibe-trace`, `specmap-core`.

## Refinement №1 — the capability boundary (one criterion, applied everywhere)

**Criterion: one row = one variant of `pub enum Command`** (a top-level
surface). A group (`Mcp`, `Registry`, …) is one variant, therefore one row;
the count of its subcommands is carried in the *note* column, not as extra
rows. `Bin` is one struct variant (`Bin { cmd }`), so it too is one row.
Applied uniformly this yields exactly **29 rows = 29 top-level surfaces**,
matching the brief's «all 29.» Expanding any group would double-count
(variant + subcommands) and break «all 29»; collapsing leaf commands would
lose surfaces. The unit is the enum variant because *that is the definition
of a top-level surface here* — the dispatch in `main.rs` has one arm per
variant and nothing coarser.

Groups that carry subcommands (counted from each group's `…Args::command`
match / the `cli/<group>.rs` enum), 10 of the 29:

| group | subcommands |
|---|---|
| `aiui` | 11 (render, state, open, send, snapshot, wait, close, inspect, pty-start, pty-stop, scrollbar) — `commands/aiui/mod.rs:21` |
| `registry` | 11 (sync, publish, list, add, set-mirror, remove, vendor, test, redirect, redirect-sync, redirect-update) — `commands/registry/mod.rs:32` |
| `self` (vvm) | 11 (install, update, use, ls, current, which, doctor, remove, gc, env, relocate) — `cli/vvm.rs:16` |
| `progress` | 10 (scan, check, report, mirror, weave, rescan, baseline, resume, gate, seal) — `commands/progress.rs:51` |
| `prefs` | 7 (get, set, list, check, migrate, show-origins, ui) — `commands/prefs/mod.rs:41` |
| `mcp` | 5 (serve, install, status, upgrade, uninstall) — `commands/mcp/mod.rs:83` |
| `show` | 5 (effective, config, features, subskills, purls) — `commands/show/mod.rs:37` |
| `bin` | 4 (list, build, path, exec) — `cli.rs:268 BinCmd` |
| `skill` | 3 (list, install, uninstall) — `commands/skill/mod.rs:29` |
| `workspace` | 1 (publish) — `commands/workspace/mod.rs:45` |

The remaining **19 are leaf commands** (no subcommand dispatch): `init`,
`list`, `install`, `outdated`, `search`, `term`, `frame`, `agentic`,
`command`, `uninstall`, `update`, `reinstall`, `check`, `explain`,
`specmap`, `tree`, `vars`, `trace`, `version`.

## Refinement №2 — «home of logic» can be more than one

The *home* column names the single crate where the **substance** of the work
lives. Where a handler calls several crates, that one is the substance and
the rest are listed in the note. Where there is **no** substance anywhere
outside `vibe-cli`, the home is `vibe-cli` — and that *is* the finding
(worth more attention than a delegated one).

## The census — one row per top-level command

Columns: **команда** (name as in CLI) · **дом логики** (crate holding the
substance, or `vibe-cli` if it lives in the handler) · **толщина
обработчика** (`wc -l` of the file `main.rs` dispatches to, and
delegates/computes) · **MCP-близнец** (the `tools.rs` tool sharing a crate
with it, or `—`) · **замечание** (what the handler is busy with if not thin).

| команда | дом логики | толщина обработчика | MCP-близнец | замечание |
|---|---|---|---|---|
| `vibe init` | **vibe-cli** | 296 (`init/mod.rs`); computes | — | 3 forms (project/package/group); scaffolding in `init/*` + `helpers/package/prompts`; `vibe-core` gives only `Manifest` types |
| `vibe list` | **vibe-cli** | 196 (`list.rs`); computes | — | iterates the `vibe-core` lockfile; the table/JSON rendering is the substance and lives here. `query_package` (MCP) shares the *data* (same `Lockfile`) but a **separate** rendering — see §MCP |
| `vibe install` | `vibe-install` | 293 (`install/mod.rs`); delegates | — | «thin CLI layer over the `vibe-install` orchestrator»; CLI owns input-norm + confirm + rendering, the pipeline is `vibe_install::{Plan, PlanObserver}` (`mod.rs:1`). Also `vibe-resolver`, `vibe-workspace::hooks` |
| `vibe outdated` | `vibe-registry` | 212 (`outdated.rs`); delegates | — | `probe_latest` via `MultiRegistryResolver` (`outdated.rs:165`); rendering in CLI. Reads lockfile from `vibe-core` |
| `vibe search` | `vibe-registry` | 466 (`search.rs`); delegates | — | `vibe_registry::search::{cache, full_scan, query}` + `IndexClient` are the substance (`search.rs:30`); CLI = env + rendering |
| `vibe mcp` | `vibe-mcp` | 496 (`mcp/mod.rs`); delegates (but the module is large) | — | 5 subcommands; «library lives in `vibe-mcp`; this module is dispatch + per-agent config writers» (`mod.rs:12`). The 496-line module holds the CLI-side agent-config writers over `vibe_mcp::agents` |
| `vibe aiui` | **vibe-cli** | 77 (`aiui/mod.rs`); delegates locally | — | 11 subcommands; `render`/`state` call `commands::tree::snapshot/state` (vibe-cli); control plane (`open`/`send`/…) in `aiui/control.rs` (vibe-cli) |
| `vibe term` | **vibe-cli** | 475 (`term.rs`); computes | — | detect shell, resolve vibeterm's Electron binary, spawn detached. No `vibe-*` crate — launch logic nailed to the CLI |
| `vibe frame` | **vibe-cli** | 475 (shares `term.rs`); computes | — | `run_frame` (`term.rs:34`) — the same launch logic as `term`, different app name |
| `vibe skill` | `vibe-mcp` | 380 (`skill/mod.rs`); delegates | — | 3 subcommands; enumeration (`collect_skills`) in vibe-cli, the per-(agent,scope) write is `vibe_mcp::pkgskill` + `vibe_mcp::agents` (`mod.rs:21`). Also `vibe-workspace` |
| `vibe agentic` | `vibe-mcp::agentic` | 127 (`agentic/mod.rs`); delegates | **`agentic_explain`** | «the library (`vibe_mcp::agentic`) owns the relay» (`mod.rs:9`); `explain_intent` + `RelayBackend` shared one-to-one with the MCP tool. CLI = outcome rendering |
| `vibe command` | `vibe-mcp::agentic` | 127 (shares `agentic/mod.rs`); delegates | — | `run_command` (`agentic/mod.rs:97`); `drain_intent` from `vibe_mcp::agentic`. The MCP face of this relay is `agentic_explain` |
| `vibe uninstall` | `vibe-workspace` | 229 (`uninstall.rs`); delegates | — | `regenerate_boot`, `guard_destructive`, `vibedeps::slot_rel_path` are the substance (`uninstall.rs:20`); prompt + rendering in CLI. Reads `vibe-core` |
| `vibe update` | `vibe-workspace` | 479 (`update.rs`); delegates | — | `materialise_subtree`/`regenerate_boot`/`run_post_install_hooks` (`update.rs:29`). `--all`/no-args **delegates to `install`** (→ `vibe-install`); scoped path is the `vibe-workspace` install cell |
| `vibe reinstall` | `vibe-workspace` | 419 (`reinstall.rs`); delegates | — | `regenerate_boot`/`apply_resolution` are the substance (`reinstall.rs:41`); `--force` re-fetches via `vibe-install`. `vibe-resolver` too |
| `vibe check` | `vibe-check` | 204 (`check.rs`); delegates | — | `vibe_check::check_project` is the linter (`check.rs:26`); appends `vibe_workspace::install::verify_boot_graph` findings (`check.rs:186`). Rendering/exit-code in CLI |
| `vibe show` | **vibe-cli** | 61 (`show/mod.rs`); computes (in subfiles) | — | 5 subcommands; `mod.rs` is a thin dispatch; `effective`/`config`/`features`/`subskills`/`purls` live in `show/*` and touch **only** `vibe-core` types (`grep use vibe_` → `vibe_core::manifest` only) |
| `vibe prefs` | `vibe-settings` | 217 (`prefs/mod.rs`); delegates | — | 7 subcommands; «the *logic* lives in `vibe-settings::cli`; this module is the *surface*» (`mod.rs:5`); `loader`/`schema`/`persist` from `vibe-settings`. `vibe-core::settings` chokepoint |
| `vibe tree` | **vibe-cli** | 273 (`tree/mod.rs`); computes | — | the engine `build::build_tree` is **local** to `commands/tree/` — no shared crate (`mod.rs:57`); `vibe-settings` for prefs, rat-salsa for the TUI |
| `vibe registry` | `vibe-registry` / `vibe-publish` | 59 (`registry/mod.rs`); delegates (subfiles) | — | 11 subcommands; `mod.rs` is a thin dispatch; per `grep use vibe_`: sync/vendor → `vibe-registry` (`MultiRegistryResolver`, `vendor`), redirect/publish → `vibe-publish` (`redirect_sync`, host/org segments) |
| `vibe workspace` | `vibe-workspace` / `vibe-publish` | 49 (`workspace/mod.rs`); delegates | — | 1 subcommand (publish); «selection/ordering lives in `vibe-workspace`, the per-package publish machinery is reused from `vibe-publish`» (`mod.rs:11`) |
| `vibe self` | **vibe-cli** | 482 (`vvm/mod.rs`); computes | — | 11 subcommands; the **whole VVM** (build/install/switch/remove/relocate) lives in `commands/vvm/*` — subfiles call **zero** `vibe-*`/`progress-core`/`specmap-core` crates (`grep` empty). A large capability nailed entirely to the CLI |
| `vibe bin` | `vibe-workspace` | 92 (`bin.rs`); delegates | — | 4 subcommands; «the resolution/build cell lives in `vibe_workspace::bins`» (`bin.rs:1`), shared with the tcg oracle; `exec` is a subprocess |
| `vibe explain` | `vibe-trace` | 98 (`explain.rs`); delegates | **`explain`** | «a thin surface over `vibe_trace::explain`» (`explain.rs:1`); shared **one-to-one** with the MCP `explain` tool — duplicates no build/render logic |
| `vibe specmap` | `specmap-core` | 425 (`specmap.rs`); delegates | — | «a thin surface over `specmap-core` (the same engine `vibe explain` and `cargo xtask specmap` drive)» (`specmap.rs:4`); coordinate minting + rendering in CLI |
| `vibe trace` | external binary `rust-ai-native` | 55 (`trace.rs`); delegates (subprocess) | — | a **pure delegator**: spawns `rust-ai-native trace` (`trace.rs:14`); the engine is **not in the host** — it ships in the project's pinned stack. Contrast `explain` (= `vibe-trace` crate): two *different* engines behind two look-alike commands |
| `vibe vars` | **vibe-cli** | 126 (`vars.rs`); computes | — | the `render` (actual/env/diff) is the substance and lives here; the row values are assembled in `main.rs:156` from `vvm` + env |
| `vibe progress` | `progress-core` | 380 (`progress.rs`); delegates | — | 10 subcommands; «the vibevm adapter over `progress-core`. All markup knowledge lives in the core» (`progress.rs:1`); CLI = path/campaign-zone/rendering |
| `vibe version` | **vibe-cli** | inline ~4 lines (`main.rs:225-228`); computes | — | no handler file — `println!` directly in the dispatch arm |

## 1. How «thickness» was measured

**Criterion applied uniformly:** `wc -l` of the file `main.rs` dispatches to
(one arm → one file), taken straight off the disk with no edits. The file
*is* the handler's footprint; «delegates/computes» is read from that file's
`use` lines + the body of its `run`. One command (`version`) has no file —
its handler is four lines inline in `main.rs:225-228`.

**Boundaries I acknowledge, with examples:**

- **`wc -l` counts tests too.** `explain.rs` is 98 lines but ~46 are
  `#[cfg(test)]`; `trace.rs` is 55 with ~17 test. The number is the file's
  footprint, not the runtime path length. I did not strip tests, because the
  rule is «lines of the handler file» and stripping is a judgement call that
  would make the number non-reproducible.
- **A small `mod.rs` can hide a large capability — and vice-versa.**
  `workspace/mod.rs` is 49 lines (thin dispatch into `publish::run_publish`)
  yet `publish` is real work; `self`/`vvm/mod.rs` is 482 lines and *is* the
  whole VVM. For the **group** rows the handler file is the dispatcher; the
  substance is in the named subfiles/crate. The thickness column therefore
  reads the dispatcher's size for groups, and the note says where the real
  weight is.
- **Thickness can mislead about «thin».** `trace.rs` (55) is trivially thin
  *because* it spawns an external binary — it delegates *more* than a
  crate-delegating handler does. `term.rs` (475) is large but filled with
  spawn/detach variants, not task-substance branching. The
  delegates/computes flag is the load-bearing signal; the line count is the
  footprint.

Where the boundary is contestable I left the raw number and put the reading
in the note, rather than round it away.

## 2. The five MCP tools vs. their CLI kin (the reverse-direction measure)

This measures from the MCP side and catches a different class of gap: an
MCP tool whose substance has **no** CLI sibling (asymmetry), or one whose
substance is **duplicated** rather than shared.

| MCP tool (`tools.rs`) | CLI kin | shared logic? |
|---|---|---|
| `explain` (`ExplainMcpTool`, `tools.rs:491`) | `vibe explain` (`explain.rs`) | **shared, one crate, one function.** Both call `vibe_trace::explain` / `vibe_trace::fragment` (`tools.rs:522`, `explain.rs:38`). This is the norm held. |
| `agentic_explain` (`AgenticExplainMcpTool`, `tools.rs:425`) | `vibe agentic explain` + `vibe command` (`agentic/mod.rs`) | **shared, one crate.** Both reach `vibe_mcp::agentic::explain_intent` behind the `InferenceBackend` seam (`tools.rs:441`, `agentic/mod.rs:17`); CLI parks to the relay mailbox, MCP returns inline (the only difference, by design). |
| `query_package` (`QueryPackageMcpTool`, `tools.rs:69`) | `vibe list` (nearest) | **not shared — two implementations.** Both read the same `vibe-core` `Lockfile`/`LockedPackage`, but each builds its own JSON by hand: `list.rs:42 JsonEntry` vs `tools.rs:122 json!{…}`. The *type* is shared; the *«show a package» logic* is not. No single function is reused. |
| `read_subskill` (`ReadSubskillMcpTool`, `tools.rs:156`) | **none** | no CLI command reads a subskill's body. The substance (eager-vs-lazy-pull file walk, `tools.rs:215-258`) lives **only** in `vibe-mcp/tools.rs`. Asymmetric — MCP-only surface. |
| `materialise_subskill` (`MaterialiseSubskillMcpTool`, `tools.rs:287`) | **none** (install materialises at install time) | no CLI command materialises one subskill on demand. The cache→tree copy (`tools.rs:316-405`) lives **only** in `vibe-mcp/tools.rs`. Asymmetric — MCP-only surface. |

**Reading:** of the five tools, **2** (`explain`, `agentic_explain`) hold the
norm — one crate, called by both surfaces. **1** (`query_package`) shares
the data type but duplicates the rendering. **2** (`read_subskill`,
`materialise_subskill`) have no CLI sibling at all — their entire substance
is nailed to `vibe-mcp/tools.rs`. (The host also has the *inverse* asymmetry:
`vibe list` has no MCP twin that lists *all* packages, and most CLI
capabilities — `init`, `tree`, `self`, `term`, `vars`, … — have no MCP
surface, which is expected for human/interactive work.)

## 3. What remained unmeasured

Each item with the reason it could not be settled by reading:

- **The *weight* of a group's subfiles vs. its dispatcher.** For the 10
  group rows I read the dispatcher (`mod.rs`/`commands.rs`) and resolved the
  home from the subfiles' `use vibe_*` lines (a `grep`, not a line-count of
  each subfile). I did **not** `wc -l` every subfile, so «thickness» for a
  group is the dispatcher's size, not the subcommand total. To get the total
  one would `wc -l commands/<group>/**`, which I did not run for all ten.
- **`registry` / `show` / `prefs` subcommand homes individually.** I named
  the home *per group* from the aggregate `use` grep
  (`registry` → `vibe-registry`+`vibe-publish`; `show` → `vibe-core` only;
  `prefs` → `vibe-settings`). I did not read each subcommand file, so the
  per-subcommand home (e.g. does `registry test` reach `vibe-registry` or
  `vibe-core`?) is not in the table — only the group-level verdict is.
- **Whether `query_package`/`list` could share a renderer.** I established
  they do **not** today (two hand-built JSON shapes). Whether a shared
  `vibe-core` renderer is *feasible* is a design question, not a reading —
  out of scope for a census.
- **The MCP server's *transport* vs. its *tools*.** I measured the 5 tools
  (`tools.rs`). `vibe mcp serve` (`mcp/mod.rs:93 run_serve`) wires
  `vibe_mcp::{Server, ServerContext}`; I did not trace the JSON-RPC dispatch
  inside `vibe-mcp` to confirm it routes only these 5 — I took
  `default_tools()` (`tools.rs:44`) as the registration point, which is what
  the file comment claims.
- **`vibe trace`'s delegatee identity at runtime.** `trace.rs` spawns
  `rust-ai-native`; I read the spawn, not the installed binary's contents
  (it is not in `crates/`). The home «external binary» is certain; the
  binary's internal structure is out of perimeter.

## The counts (counted, not estimated)

Each number with the command that reproduces it.

1. **Commands with a home outside `vibe-cli`: 19 / 29.**
   `install, outdated, search, mcp, skill, agentic, command, uninstall,
   update, reinstall, check, explain, specmap, prefs, registry, workspace,
   progress, bin, trace`.
   Reproduce: the *home* column of the table above (a classification from
   reading; the 10 non-matches are the next count).
2. **Commands whose home is `vibe-cli` (substance nailed to the CLI): 10 / 29.**
   `init, list, aiui, term, frame, show, tree, self(vvm), vars, version`.
   Reproduce: `grep -nE '\*\*vibe-cli\*\*' campaigns/packages-2026-09/harvest/g6-b047-surfaces-census.md` → 10 matches in the table.
3. **Commands that share a crate+function with an MCP tool (a confirmed twin): 2 / 29.**
   `explain` ↔ `explain` (`vibe-trace`); `agentic` ↔ `agentic_explain`
   (`vibe-mcp::agentic`).
   Reproduce: `grep -cE '\*\*`explain`\*\*|\*\*`agentic_explain`\*\*' campaigns/packages-2026-09/harvest/g6-b047-surfaces-census.md` → 2.
4. **MCP tools with no CLI sibling at all: 2 / 5.**
   `read_subskill`, `materialise_subskill` (both substance-only in
   `vibe-mcp/tools.rs`). Reproduce: §2 table, «none» rows.
5. **Total lines of the 26 distinct handler files: 6666.**
   The 25 files from the dispatch `match` + `vvm/mod.rs`.
   Reproduce:
   ```
   wc -l crates/vibe-cli/src/commands/{init/mod,list,install/mod,outdated,search,mcp/mod,aiui/mod,term,skill/mod,agentic/mod,uninstall,update,reinstall,check,explain,specmap,show/mod,prefs/mod,tree/mod,registry/mod,workspace/mod,vars,progress,bin,trace}.rs \
         crates/vibe-cli/src/commands/vvm/mod.rs
   ```
   → 6666 total (the 25-file partial is 6184; `vvm/mod.rs` adds 482).
   `vibe frame` reuses `term.rs` (475) and `vibe command` reuses
   `agentic/mod.rs` (127); `vibe version` has no file (4 inline lines in
   `main.rs:225-228`) — none are double-counted in the 6666.
