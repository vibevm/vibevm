# R7 MCP lifecycle surfaces — implementation architecture v0.1

Status: accepted central implementation design, 2026-08-28. Three independent
`claudez` audits and one Opus/max adversarial adjudication were reviewed by the
central coordinator before implementation. Semantic authority remains
PROP-054 `##AGENT-HANDSHAKE` and `##REF-AGENT-RESUME`. The omnichannel law is
the installed `flow:org.vibevm.world/omnichannel`: the lifecycle operation
lives in a library; CLI and MCP are sibling adapters. This document fixes the
implementation seams and atom order. It does not amend the product spec.

Execution status remains in
[`LIFECYCLE-EXTENSIONS-IMPLEMENTATION-LEDGER.md`](LIFECYCLE-EXTENSIONS-IMPLEMENTATION-LEDGER.md).

## 1. Outcome and non-goals

R7.4 ships two MCP tools:

- `lifecycle_run` executes one selected default-lifecycle chain in hosted agent
  mode and returns the same typed lifecycle result the CLI renders;
- `lifecycle_tasks` reads the current durable handoff and returns only tasks
  owned by validated lifecycle state.

Both consume the existing `.vibe/lifecycle.toml` and
`.vibe/agentic/outbox/<run>/…` files. There is no MCP mailbox, MCP run id,
MCP-specific resume verb, CLI subprocess, stdout parser, or duplicated
lifecycle planner. Repeating `lifecycle_run` with the same requested phase is
the resume operation. The first unsatisfied agent row parks the run and stops
the whole downstream chain; a satisfied row continues through the same engine.

R7.4 never constructs `vibe-llm`, reads a credential, or calls a provider.
Provider-backed CLI agent execution remains the terminal adapter selected only
for `agent_mode = cli`. MCP is always the hosted no-spend adapter.

Clean is not exposed by `lifecycle_run` in epoch 1. The current clean epoch is
state-untracked, so it has no durable run identity or continuation against
which a hosted task can resume; R7.3 correctly refuses hosted rows before the
wipe. Clean itself leaves `.vibe/agentic` untouched. Adding a tracked clean
handoff is separate debt, not a hidden special case in MCP.

## 2. State I/O and command ownership close before MCP

### 2.1 One workspace-global mutation lease

`.vibe/lifecycle.toml` is a workspace-global single-writer record. The current
`Arc<Mutex<LifecycleRun>>` protects one process only; two CLI/MCP processes can
read the same prior state, independently allocate/adopt, then last-writer-wins
the other's row through two individually atomic renames. Atomic publication is
not compare-and-swap.

Every mutating lifecycle surface therefore acquires the same capability-safe,
nonblocking `.vibe/lifecycle.lock` at the canonical workspace root **before**
world/state/identity reads and holds it through the final state/task/report
outcome. This includes default phase verbs, direct install, update, reinstall,
clean and MCP `lifecycle_run`. A composed command acquires once at its outer
boundary and passes a non-cloneable lease through install callbacks; nested
code never re-enters the lock. Busy is a typed refusal with no lifecycle or
outbox mutation.

The lock uses the strengthened `vibe-safefs` lock primitive and is not a second
implementation. Its persistent empty lock file is infrastructure of a
mutating command, not lifecycle state. `clean` already removes only dependency
slots plus generated boot artifacts; it never removes `.vibe`, so the live
lock's name survives the clean epoch. `lifecycle.lock` is the outermost project
lock: code holding `compile-trace.lock`, `package-skills.lock` or
`vibe-boot-artifacts.lock` may never acquire it. Nested acquisition is a typed
busy refusal on the supported hosts and is pinned by a RED, not used as a
reentrancy mechanism.

Locating the canonical workspace root is a read-only preflight, not the
execution snapshot. The command resolves the selected node, discovers the
workspace root, acquires the lease, and only then loads the manifest, effective
world and state that execution will consume. This prevents a just-finished
concurrent mutator from making a pre-lease world snapshot stale.

The concrete owner is a non-`Clone`, non-`Copy` `LifecycleLease(LockGuard)`.
One `Arc<LifecycleLease>` may travel through the existing shared
`LifecycleRun`/install callback channel; cloning that `Arc` proves the one OS
acquisition and can never reacquire the file. `LifecycleStateStore::begin`
requires a lease proof by type, while the outer command retains the owner for
untracked clean paths on which no store exists. Putting acquisition inside the
store is insufficient: identity selection reads state before the store exists.

`lifecycle_tasks` remains physically read-only and does **not** create a lock
file. It uses a bounded optimistic `state → exact task files → state` read:

1. safe-read state bytes (or observe absence);
2. parse/validate and safe-read every exact state-owned task;
3. safe-read state again and require byte identity;
4. retry exactly three times on change — `MAX_RETRIES = 3`, so the initial
   attempt plus retries 1–3 are at most four complete
   `state → tasks → state` attempts; only an unchanged snapshot may return.

An absent first read may return `absent`: that result linearizes immediately
before a concurrent writer creates state. A missing task is rechecked against
state before becoming an error — if a concurrent resume completed the row, the
reader retries and returns `idle`; if identical state still owns the missing
task, it refuses honestly.

State and each task are capped at 8 MiB. The reader also enforces a 16 MiB
aggregate across the state bytes and all task documents and refuses more than
64 delegated rows. The current engine parks at most one row per invocation;
these wider ceilings bound hostile state without imposing a workflow policy.

### 2.2 Harden the lifecycle state file itself

Before selected-node identity or MCP lands, `LifecycleStateStore` moves from
ambient `read_to_string`/rename to the shared filesystem cell:

- capability-relative no-follow reads from the canonical workspace root;
- regular, single-link state only;
- an 8 MiB read ceiling checked before allocation and again while reading at
  most `cap + 1` bytes through the already-open handle;
- generated parse plus lifecycle semantic validation;
- staged atomic replace through the pinned project capability;
- the command lease around every read/clone/commit transition.

Publication stage remains part of the state transaction. After
`PossiblyPublished`, the store re-reads under the held lease: exact candidate
bytes become both durable and in-memory state; exact prior bytes retain the
prior in memory; anything else poisons/refuses the store without another
write. A post-step fault is retained as a typed diagnostic in every branch.
Thus candidate state and memory never disagree merely because rename succeeded
before an error was reported.

The filesystem cell also gains one shared bounded safe-read primitive. It
checks ordinary single-link metadata before allocation and reads at most
`cap + 1` bytes through the pinned handle. Before returning it reopens the
same final name capability-relatively with no-follow, rechecks ordinary
single-link metadata, and compares the two handles' OS file identities. A
held handle's metadata is not enough: on POSIX the opened inode can be renamed
away (still one link) while a new regular single-link inode takes its old
name. The reopen is through the already pinned directory capability, never an
ambient path. Lifecycle state uses 8 MiB; task documents use the existing
`TASK_CAP`. An after-the-fact `Vec::len()` check is not a bound.

REDs cover a `.vibe` link/reparse point, a state symlink, hardlink, directory,
oversized file, final-name replacement/disappearance/rebinding race and
post-publication fault. The replacement RED is a real rename-old/create-new
race on Unix and a deterministic identity-comparison override on Windows,
where sharing rules may make the physical race unreachable. A malformed or
unsafe state remains erasable cache but is never followed, partially read or
silently replaced.

### 2.3 Selected-node identity

Lifecycle state lives at the workspace root, while an outbox task and its
declared outputs are relative to the selected workspace node. Today the run
header records requested phase, chain, start and run id, but not the selected
node. Two members of one workspace can therefore present the same
requested/chain tuple; the second member can adopt the first member's parked
run id and then look for its task under the wrong root.

The fix is an additive epoch-1 run member:

```text
selected = "." | "members/tool"       # canonical workspace-relative RelPath
```

The selected identity is computed from canonical workspace root plus canonical
selected node, written on every new run, and compared before adoption. Both
ends are canonicalised before relativisation, so a Windows drive-case or 8.3
entry alias reaches the same workspace node; the persisted value remains the
portable forward-slashed node rel, never an inode/file-index key. A different
stored spelling is a mismatch, not something the reader case-folds by guess.
Persisted bytes are validated AS WRITTEN before any `RelPath` construction:
the infallible `RelPath::new` normalises backslashes/trailing separators and
maps the empty string to `"."`, so applying it to attacker-editable state could
repair a foreign spelling into workspace-root ownership. The state validator
accepts only raw `"."` or nonempty forward-slashed normal components and never
normalises a stored identity.

- A newly delegated row requires `run_id` and `selected` together.
- A pre-R7.4 state with no delegated row remains readable and is refreshed on
  the next begin.
- A delegated legacy state with no selected identity refuses as erasable
  ambiguous state; it is never adopted by guess.
- A prior park owned by another selected node is a typed busy/ownership
  refusal even under CLI force: a member may not silently discard a sibling's
  live handoff. Same-node force/changed-chain displacement keeps the existing
  supersession behavior. Foreign-node refusal deliberately does not
  terminalise or supersede the owning node's trace.
- `select_run_identity` adopts only when mode, force, requested, complete
  chain, selected node, valid run id and a delegated row all agree.
- The CLI derives selected identity once from its already prepared workspace
  snapshot and carries it beside the prelude; it never rediscovers merely to
  answer ownership. A composed clean's mandatory post-wipe rediscovery is a
  second topology epoch, so its selected node must still equal that carried
  identity before the remaining phases may proceed. A mismatch is a typed
  late-boundary refusal that does not erase already reported wipe/scratch
  effects.
- `lifecycle_tasks` discovers the workspace from the MCP server's selected
  root and refuses a state owned by another selected node before reading any
  task.

These are R7.3 hardening atoms and land before either MCP tool.

## 3. `lifecycle_tasks` is the first independent cut

### 3.1 Lower operation

`vibe-lifecycle` gains one read-only operation, conceptually:

```rust
pending_hosted_tasks(selected_root: &Path)
    -> Result<LifecycleTasksReport, LifecycleTasksError>
```

The selected-root input first crosses one `vibe-workspace` seam that returns
the canonical selected root, its discovered `Workspace`, and the authored
`RelPath` together. This is one epoch, not a public canonicalizer plus a later
rediscovery; `node_rel_of` continues to accept only an already canonical node.

It performs, in this order:

1. canonicalise the selected root and discover its workspace;
2. compute the canonical workspace-relative selected identity;
3. run the bounded optimistic safe-read protocol above at the workspace root —
   never `begin` and never a lock-file-creating read;
4. validate the generated state and its selected identity;
5. select only rows whose typed status is `delegated`;
6. require the one state-owned task path each row already carries; the state
   validator is the one ownership gate which recomputes
   `outbox_task_path(run_id, execution_key)`, while the reader asserts that
   validated relation rather than inventing a second path law;
7. read that exact project-relative file through `vibe-safefs::Project`
   (capability-relative, no-follow, regular single-link and final-name
   identity-stable under the strengthened bounded read);
8. decode UTF-8, re-read state for byte identity and return a generated report.
   Missing, linked, hardlinked, replaced, non-UTF8 or oversized state/task data
   refuses only against an unchanged owning state; no orphan scan is attempted.

Ordering comes from the durable chain's phase order and execution key as a
deterministic tie-break. Directory enumeration and filename parsing are never
ordering or ownership inputs. R7.3 parks at most one row per invocation, but
the reader remains total over a validator-green state carrying more than one
typed delegated scope.

Task failures are provisional until the second state read. A missing/unsafe/
non-UTF8/oversized task under changed owning state is discarded and retried;
the same failure under byte-identical owning state is returned. If state
changes on all four attempts the typed result is `UnstableSnapshot`. The hard
ceilings remain 64 delegated rows, 8 MiB per state/task document and 16 MiB
aggregate; the aggregate remaining budget is the cap passed into each bounded
task read, so the reader never allocates beyond it.

### 3.2 JTD report

Add one JTD root `lifecycle_tasks` before Rust code. Its epoch-1 shape is:

```text
LifecycleTasksReport {
  schema: u32 = 1,
  status: absent | idle | parked,
  run?: {
    run_id?: string,
    requested: string,
    chain: [string],
    selected?: string,
    started: string
  },
  tasks: [ {
    execution: string,
    phase: string,
    scope: phase | slot,
    path: string,
    document: string
  } ]
}
```

The task document already contains the ordered output contract in serialized
frontmatter plus the exact system/request prose. The report does not parse and
restate those outputs into a second DTO. `absent` means no state file; `idle`
means valid state with no delegated rows; `parked` requires a nonempty task
list and valid run id. An existing state carries its run header for both
`idle` and `parked`; only `absent` omits it. `tasks[].path` is relative to the
selected **node** root, never the workspace state root. The carried `selected`
rel is what lets a consumer relate those two roots.

This new machine-to-machine payload is JTD-first despite older hand-shaped MCP
tools: add `schemas/lifecycle_tasks.jtd.json`, a registered
`mcp-lifecycle-tasks` format and authored corpus/relational behavior cell.
`lifecycle_run` reuses the registered lifecycle report schema and adds parked
and executed-failure corpus examples; it does not mint a second report root.

The schema enters the registry/corpus/codegen in the ordinary JTD-first atom.
Its behavior cell enforces `absent|idle|parked` against optional run/nonempty
tasks and validates every identity/path scalar. No `formats/EPOCHS.toml` edit
and no hand-written generated Rust.

### 3.3 MCP adapter

The `lifecycle_tasks` input schema is an empty object. Omitted or JSON `null`
arguments normalize to `{}`; an explicit object is decoded through a
deny-unknown empty struct, so even `{ "path": "other" }` refuses before any
read. Project selection comes only from `ServerContext.project_root`; a tool
argument cannot escape the MCP server's declared project. The tool calls the lower operation and serializes
the generated report into `structuredContent`. Its text content remains a
human-legible projection of the same value; it is never a separately computed
task list.

## 4. The complete run belongs above lifecycle and install

The current whole-command algorithm lives in the binary crate under
`vibe-cli/src/commands/lifecycle*`. It plans the effective world, performs the
validate/install barrier, drives slot and phase contributions, reconciles
package-skill bindings, truncates at a park and assembles reports. `vibe-mcp`
cannot call it: CLI already depends on MCP, and shelling out would make CLI the
reference implementation.

The durable home is a new library crate `vibe-orchestrator`:

```text
vibe-core ───────────────┐
vibe-workspace ──────────┤
vibe-lifecycle ──────────┼──> vibe-orchestrator <── vibe-cli
vibe-install ────────────┤                         <── vibe-mcp
vibe-agent-projection ───┘
```

`vibe-orchestrator` depends on neither surface and does not depend on
`vibe-llm`. It owns:

- selected-world loading and lifecycle plan construction;
- run identity selection and metadata;
- validate/install barrier and slot continuation;
- phase dispatch and removed-row reconciliation;
- package-binding plan/backend composition;
- report-neutral success/failure/park values.

The CLI owns argument parsing, provider construction, terminal/JSON rendering
and interactive policy. MCP owns JSON-RPC tool description/arguments and
structured result rendering. Both supply an execution policy and optional
observer to the same operation.

## 5. Move project skill projection out of the MCP surface

The existing package-skill implementation is behavior, not MCP transport, but
today its types live at `vibe_mcp::pkgskill`; CLI world planning consequently
depends upward on the MCP crate. That placement would create a cycle as soon as
MCP consumes the orchestrator.

Extract the minimum cycle-breaking implementation unchanged into a lower
library `vibe-agent-projection`: `agents.rs`, `pkgskill.rs`,
`pkgskill/{exact_path,projection,receipt}/**` and the seven-line pure
`preview_status` helper. It owns project-only Claude/Codex/OpenCode skill
projection, receipts, recovery, planning types and the package-binding adapter.
`agent_config.rs`, `pkg_servers.rs`, MCP `install.rs` apart from that helper,
`agentic.rs` and every transport/config surface stay in `vibe-mcp`.
`vibe-mcp` re-exports compatibility names for one transition if public callers
need them, but no behavior remains duplicated. CLI and the orchestrator depend
on the lower crate. MCP needs a direct dependency only while it provides that
compatibility re-export; no existing default MCP tool itself consumes package
skill projection.

This move does not turn project projection into user deployment. R8 still owns
portable client artifacts and explicit local deploy.

## 6. Prompt resolution is credential-free shared behavior

The current `CliAgentBackend` combines two operations:

1. resolve the selected provider's `spec://` prompt and recursive `#embed`
   closure against the lock-selected world;
2. construct/read the paid provider and complete the request.

Split the first into a lower credential-free resolver usable by both surfaces.
It belongs in `vibe-lifecycle::agent`, beside the existing
`AgentBackend::resolve_prompt` contract. The hosted MCP adapter uses that resolver and has
no completion capability. The CLI adapter composes the same resolver with
`vibe-llm` completion.

No credentials, endpoint, model or response body enter `vibe-orchestrator` or
`vibe-mcp`. Tests count provider construction/completion calls and require zero
for every MCP run, including failures and resumes.

## 7. Surface-neutral library command API

The library input for the default chain is conceptually:

```rust
LifecycleCommand {
  selected_root,
  requested: Phase,
  force,
  offline,
  agent_mode,
  assume_yes,
}
```

Those are library facts, not MCP wire members. The MCP adapter pins
`force = false`, `agent_mode = agent`, `assume_yes = true`; it resolves offline
posture through the ordinary user/environment policy with no client override.
No LLM provider, credential reader or transport is constructed. Ordinary
algorithmic package resolution remains the same operation the CLI uses.

Surface-specific package arguments and prepared install inputs enter through a
typed input/port rather than a closure capturing CLI state. Process stream mode
and narration are execution policy/observer ports. The result is a typed
`LifecycleCommandResult` carrying the generated `LifecycleReport` data and the
same optional `Delegation` R7.3 produced. A failure carries the `ok:false`
executed-prefix report plus the typed source error, so CLI JSON and MCP render
the same facts without emitting a partial document inside the library.

The MCP tool seam gains an output value distinct from transport failure:
`{ structured, text, is_error }`, with text mandatory. Existing tools use a
success constructor which reproduces today's String passthrough / pretty-JSON
projection byte-for-byte.
A lifecycle handler/install failure returns a tool output with
`isError: true` **and the generated `LifecycleReport` as the single
`structuredContent` root**; malformed JSON-RPC/tool arguments remain ordinary
tool/transport errors. Thus MCP never loses earlier successful contributions
merely because a later row failed.

The CLI adapter becomes parse → call → render. Existing byte/golden/e2e output
is a characterization oracle for the extraction; no CLI grammar changes in the
move.

## 8. `lifecycle_run` tool grammar

Epoch-1 MCP input is deliberately narrow:

```json
{
  "phase": "validate|install|generate|build|test|create|verify|package|deploy"
}
```

`phase` is the only member and is required. The descriptor's JSON
Schema is documentation for the host, not runtime authority: before any lock,
`.vibe`, world or state access, a `#[serde(deny_unknown_fields)]` input type
strictly decodes the object and the closed `Phase` vocabulary.
Wrong types, missing phase and unknown members are negative no-mutation REDs.
There is no `path`
(the server context is the authority), no provider/model/agent-mode option
(MCP is always hosted agent mode), no `resume` flag (same phase is resume), and
no clean flag in epoch 1. The adapter sets noninteractive confirmation policy
without granting any operation beyond the requested default chain.

The result is the existing generated `LifecycleReport`, including exact chain,
executed prefix, contribution rows and optional delegation. Parking is a
successful tool result with a nonempty delegation; it is not an MCP error and
does not report later phases as executed. Malformed state is an in-band MCP
tool error. Install/handler failure is `isError:true` with the generated
executed-prefix report retained as structured content and text derived from the
same typed failure the CLI receives.

The report's existing `delegation.resume` remains the exact CLI command because
the generated report is surface-identical. MCP-native guidance (call the same
`lifecycle_run` tool again with the same phase) appears only in the textual MCP
projection; no surface-conditional wire member or shell subprocess is added.

## 9. Acceptance matrix

### Selected-node identity

1. Two workspace members with identical requested phase/chain never adopt one
   another's parked run.
2. The original member adopts its exact run and start. On that same selected
   node, `--force`, CLI mode or a changed chain allocates fresh identity. A
   different selected node allocates fresh only when no live delegated row is
   owned elsewhere; a foreign live park is the ownership refusal in §2.3,
   even under force.
3. Delegated legacy state without selected identity refuses; nondelegated
   legacy state remains readable and upgrades on begin.
   - Invalid raw spellings — especially empty, backslashed, drive-like,
     leading/trailing/doubled slash and dot-segment forms — refuse rather than
     being normalised; empty can never become `"."`.
   - A composed clean whose post-wipe workspace reload maps the selected root
     to a different node refuses before any remaining phase uses the stale
     pre-wipe identity.

### `lifecycle_tasks`

4. Absent state → `absent`, no filesystem mutation.
5. Complete valid state → `idle`, empty tasks.
6. Parked phase and slot rows return exact state-owned path/document/scope.
7. Missing, symlinked, hardlinked, non-UTF8, oversized or replaced task
   refuses without allocating beyond `TASK_CAP + 1`.
8. An orphan outbox file not named by state is invisible.
9. State for another selected workspace node refuses before task read.
10. Repeated calls reload state; completion becomes `idle` without server
    restart.

### Shared orchestration and `lifecycle_run`

11. CLI and MCP over the same fixture produce equivalent normalized lifecycle
    report/state/outbox bytes; only surface framing differs.
12. First agent row parks with zero provider calls and no downstream row.
13. Unsatisfied repeat reparks idempotently; satisfied repeat continues; two
    sequential agent rows park in order under one run id.
14. Slot post-install park/resume rebuilds the persisted continuation even when
    the lockfile is fresh.
15. Force reparks; different phase/chain/node never steals a prior run.
16. `tools/list` names both tools; `tools/call` returns generated structured
    content through `MemoryTransport`.
17. Handler/install failure sets MCP `isError:true` while retaining the same
    generated `ok:false` executed-prefix report in `structuredContent`.
18. Unknown/wrong-typed/missing tool arguments refuse before any filesystem
    mutation, even when they also carry a valid `phase` member.
19. Two mutating processes cannot both own lifecycle state; the loser is busy
    before run-id/outbox/state mutation, and the winner's park cannot be lost
    or resurrected.
20. Optimistic `lifecycle_tasks` reads return one unchanged, final-name-stable
    state/task snapshot without creating `.vibe`/lock state; a concurrent
    completion yields retry→idle rather than a false missing-task error, the
    fourth stable attempt succeeds after three changes, and a fourth change
    refuses as unstable.
21. A post-publication state fault adopts exact candidate bytes in memory,
    retains exact prior bytes, and poisons any third state; no report is built
    from memory that disagrees with disk.
22. Existing CLI commissioning, hosted cancellation/progress/sequential and
    project-skill tests remain byte/behavior green.
23. Nested lifecycle-lease acquisition refuses on every supported host, and
    lock order is lifecycle → compile-trace/package-skill/boot-artifact, never
    the reverse.
24. A clean invocation holds the same lease while removing only dependency
    slots/generated boot; `.vibe/lifecycle.lock` remains named and a concurrent
    mutator cannot enter.

## 10. Landing order

1. Characterise CLI install/build/park/failure bytes, row order, quiet output
   and error identity before extraction.
2. In parallel: add the capped safefs read; add `lifecycle_tasks` JTD/registry/
   corpus/generated behavior cell; extract the minimal `vibe-agent-projection`;
   evolve `McpTool` to the byte-compatible typed output seam.
3. Harden state I/O through the pinned capability, including
   possibly-published recovery and poison-on-third-state REDs.
4. Add the outermost lifecycle lease to every existing mutating CLI surface;
   lock-order, nested-acquire, busy-zero-mutation and clean-survival REDs first.
5. Add selected-node state identity + adoption/legacy/workspace-member REDs.
6. First bind the bounded safefs read to the final pathname identity, then add
   the one-epoch selected-workspace seam, lower bounded optimistic pending-task
   reader and strict MCP `lifecycle_tasks` adapter. Run the first coherent
   boundary panel here.
7. Introduce `RunObserver`, `ConfirmGate` and `ResolverFactory` ports in place
   under the CLI characterization oracles; no behavior moves yet.
8. Flip CLI package-skill imports to `vibe-agent-projection`, retaining MCP
   compatibility reexports.
9. Move the already-port-shaped planning values into `vibe-orchestrator`.
10. In one atomic move-and-rewire commit, move dispatch/phase/callback plus the
    install execution core; never leave a broken CLI or two live planners.
11. Move trace-funnel values while presentation/adapters stay in CLI.
12. Extract credential-free prompt resolution into `vibe-lifecycle::agent`.
13. Add strict MCP `lifecycle_run`, typed executed failure, cross-surface
    parity and hosted/no-provider e2e. Run the full R7.4 boundary panel.

Each item is an independently gated atomic commit. Schema/codegen atoms land
before their consumers. Full panel runs only after the coherent R7.4 batch;
workers run exact affected tests only.
