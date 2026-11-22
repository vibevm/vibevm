# ADVANCED-ARCHITECTURE: the base, then the platform on top

Stage 1 of the campaign (2026-11-22, main `9d8428d07`), revised twice that day
as the owner stated the product direction (`CONTRACT-01.md`). This is the
planner's map. Executors never read it; they get entries in the plan tree.

## Two stages

The owner's split (2026-11-22): first build the base, VibeVM as a good system
for spec-driven development with a clear architecture. Then build clustering,
images and the OS on top of it.

```
VIBEVM-ROADMAP
  NST                     accepted
  ADVANCED-ARCHITECTURE   the base: VibeVM for spec-driven development,
                          drivable by any client, on every desktop OS
  VIBEVM-IDE-MVP          terminal IDE on the base
  VIBEVM-PLATFORM         on the base: targets, images, cluster, the AI OS
                          (ends with a major release, about 2.0.0)
```

The base does not build any target, image or cluster feature. It keeps the
invariants that let the platform be built on it without rework (below).

## How this was found

1. All 222 tasks of the original architecture map
   (`campaigns/next-semantic-transfer-2026-11/architecture/map.json`) were
   checked against today's code with file and test evidence:
   `research/inventory.json`. First-pass recommendations (code facts only,
   before intent recovery): 7 clear "do" items, 35 awaiting a product decision,
   130 marked drop or defer (80 drop, 50 defer), 50 already done. The 130 were
   mostly judged by their outdated form.
2. A second pass recovered each task's user intent for today's architecture
   and grouped it into 41 capabilities: `research/capabilities.json` (105 IDs
   in a capability, 69 already served, 25 owned elsewhere, 23 without user
   intent).
3. A third pass judged everything through the product direction:
   `research/direction-review.json`.

Historical IDs stay in the research files; nothing in the plan is named after
them.

## The base: architecture in one page

The base has six parts. Every capability below belongs to exactly one of them.

| Part | What it is | State today |
|---|---|---|
| Package core | resolve, lock, install, update, uninstall, configuration, registries, apps catalog | works; a few user-facing bugs |
| Spec layer | specs and their addresses, conversion, boot lanes, layers and overrides, facts and progress gates, authored changes | strong core; reading by address and the change workflow have no user surface yet |
| Agent clients | deploy, skills, MCP registration for claude, codex, opencode, qwen | works; never checked against the real clients |
| Machine control | the contract every client uses: `--json` reports, codes, progress events, cancellation, unattended answers, operation records, capability discovery, MCP | used by the Library app; gaps listed below |
| Shared read surfaces | what the IDEs, the Library and agents read: spec sections, the package tree, later editor buffers | CLI only for most of it |
| Release and trust | release bundles, bootstrap, smoke checks, provenance | macOS-only smoke, no ARM Linux |

### Invariants the base keeps for the platform

1. CLI plus JSON is the control transport for every client. No daemon, no
   second protocol. The same contract later travels over SSH or the mesh.
2. One event vocabulary: MCP progress mirrors the CLI `vibe-progress-event/1`
   stream.
3. Nothing assumes a single host. Operation records and refusals say which
   machine and home they concern.
4. Credentials are referenced by handle through the existing secrets owner,
   never stored inline.
5. Everything works on Windows, macOS and ordinary Linux.
6. No new engines, stores or resolvers: every capability extends the owner
   named in the research files.

## Base milestones

Sizes: S < 150 changed lines, M < 500, L < 1500.

### ADV-FIX: bugs users hit today

`vibe bin` runs the newest build (S); legacy feature flags applied (S); install
warns when a spec document was not converted (M); exact boot sizes in
`vibe tree` (S); publish dry-run and docs tell the truth (M); documented
examples checked by self-check (S); each release states what changed (S).

### ADV-CONTROL: drivable by any client

How the Library drives vibe today: it runs the user's `vibe` with
`--json --unattended --invoked-by vibevm-app` (+ `--progress-format json`),
gates features by two hard-coded versions, cancels with SIGINT (impossible on
Windows), and loses a project operation's outcome when it restarts.

| Capability | Size | Core |
|---|---|---|
| Tell the Library what already exists | S | Most app asks are already in 1.0.12. One handoff note with flags and recorded JSON; close B-232, B-254. Done by the planner, not an entry. |
| A client asks vibe what it supports | M | `vibe capabilities --json`: version, commit, platform, commands and flags from the `vibe doc surface` walk, formats with epochs, a tested feature list. |
| Stable codes and remedies | M | Under `--json`, parse errors and refusals are envelopes with `code`, `refusal`, `remedy.args`; exit codes unchanged. |
| Stop from any client on every platform | M | `--stop-file` for application verbs and `install -g`. |
| Runs without a terminal | M | Every prompt has a flag or answers-file equivalent; `--unattended` never blocks. |
| Operations outlive the client | L | Operation id, recorded state and outcome, `vibe ops list|show`, "busy" names the holder, retry with the same id returns the earlier outcome. |
| Strict MCP arguments | M | serde decoding with unknown fields refused, every tool. |
| MCP progress and cancel | L | Same events as the CLI stream; cancellation honoured. |
| Uninstall a configured package | M | The Library's Uninstall works for every package. |
| A checked contract | L | Conformance suite: every mutating command has `--json`, events and codes; outputs validate against registered schemas. |

### ADV-SDD: the spec layer gets its user surface

| Capability | Size | Core |
|---|---|---|
| Read one spec section by address | M | `vibe show section spec://…#anchor` + MCP `read_spec`, over the existing resolver. |
| Package tree over MCP | M | MCP `tree` returns the `vibe tree --json` document. |
| Show and check an authored change | L | `vibe change list|show|check` (+ MCP `change_query`) over the PROP-071 reader: validation, roles, origin pins against what is installed, digests, work items. |
| Start a change and record questions | L | `vibe change new [--from spec://…]`, `ask`, `answer`. Depends on the owner's answer on authored changes. |
| Tracked local copy of a dependency file or tree | L | `vibe derive … --to`, `list|show|diff|rebase`. |
| Rename a package across the project | L | `vibe refactor rename-package`. |
| Detect the stack in `vibe init` | L | Onboarding an existing repository for vibecoders. |
| Preview the boot change before install | L | `--plan-only --boot-delta`: what a package adds to the agent's context. |

### ADV-NATIVE: other package managers as native (owner, 2026-11-22)

vibe manages the packages of other package managers as its own: the
operating system's (pacman first, then apt, brew, winget) and language
ecosystems', above all global npm and pip packages. Today's base for this is
the ecosystem adapter of PROP-075 (`#ECOSYSTEM-ADAPTER`, `crates/vibe-app/src/ecosystem.rs`):
a package declares four verbs (install, list, remove, self-upgrade), the code
holds no ecosystem word, and the npm adapter ships in `org.vibevm.node/node`.
Today an adapter runs inside one installed runtime version. The capability
lifts it to a platform adapter:

| Capability | Size | Core |
|---|---|---|
| Platform adapters for system package managers | L | An adapter package declares a system package manager (verbs, privilege needs, how to detect it). `vibe` lists, installs, removes and upgrades its packages with the usual `--json`, events, codes and consent; privilege escalation is an explicit, refusable step. No package-manager word in vibe's code. |
| pacman adapter | M | First system adapter, as a package. |
| Global npm and pip packages as first-class | M | The existing runtime adapters show and manage global packages in the same view and the same Library screens. |

This stays data-driven: adding apt, brew or winget is a new adapter package,
not a vibe release. On the VibeVM OS the same adapter serves image builds and,
in developer mode, the running system.

### ADV-CLIENTS: real agent clients

Live check of deploy and skill install against real claude, codex, opencode
and qwen (M); bridge skills whose scripts actually run (M).

### ADV-RELEASE: release safety (owner's word: CI edits)

Smoke the installed bootstrap on all targets (S); Linux aarch64 release target
(S); refuse to replace a published release by accident (S); build provenance
attestations (S); deep Windows project roots (S).

### Not in the base, kept for later

Closing a change with evidence and waivers; compile-trace reader; Redbook
continuity as a configuration group; conditional requirements with
configuration; `[features]` migrator; stale-input reason code; human manifest
migration verb.

## VIBEVM-PLATFORM (separate campaign, after the base)

Planned now so the base keeps its invariants; not decomposed until the base is
done. Central decision: **run the same vibe inside the target** instead of
computing host-side where files land. One global flag carries any command:

```
vibe --on ssh:node7     app install ripgrep --json --progress-format json
vibe --on rootfs:./img  app import apps.vibe-apps
vibe --on group:gpu     app install ollama
vibe --on mesh:<node>   …    (when the mesh moves into VibeVM)
```

Milestones: TARGETS (named targets, `--on ssh`, `--on rootfs`, system scope
home, target-keyed lifecycle reuse); IMAGES (OCI image from an app set, build
record and reproducibility, generations of an app set, mkosi system images);
CLUSTER (fan-out to groups, peer mirrors, offline nodes, signatures, app
permissions, host-fact requirements); OS (Arch ownership boundary, app set at
boot, atomic A/B updates and rollback). Early decisions and open owner
questions for it are in `research/direction-review.json` and in the owner
conversation of 2026-11-22.

## Owned elsewhere

VIBEVM-IDE-MVP: editor-buffer analysis (built as a shared vibe surface when
needed). CORP-ENV / AGENT-ADAPTER-CONSOLIDATION: package-declared agent
clients. External repositories: bridge content. Release policy: the historical
"1.1.0 floor". Planning packages: tracker export.

## Owner decisions (2026-11-22)

The owner agreed with every recommendation:

1. An operation finishes when its client disappears and keeps a record; a
   retry with the same id returns the earlier outcome.
2. Authored changes are in the base: reading first, then creating; closing
   later.
3. Linux aarch64 becomes a release target now.
4. The CI batch is approved, as separate commits: smoke on all targets,
   aarch64, the published-release guard, attestations after the smoke.
5. The first real user of package configuration is an app with a choice
   visible in the Library, not Redbook.
6. System package managers come through platform adapters (ADV-NATIVE).

Platform-stage recommendations stay working assumptions until that stage
starts. The Library is built by a separate agent; coordination is in
`LIBRARY-COORDINATION.md`.

## Planner decisions taken while the owner was away (2026-11-22)

The owner may overturn any of these. Each one unblocked a decomposed entry.

- **aarch64 rollout.** Every released vibe refuses a DISTRIBUTIONS.json that
  lists a target it does not know. So a release containing
  ADV-RELEASE-AARCH64-1 (an installed vibe skips unknown targets) goes out
  before ADV-RELEASE-AARCH64-2 adds the target. AARCH64-2 waits for that
  release; AARCH64-5 waits for one build-only workflow dispatch by the owner.
  Only the musl ABI ships for aarch64.
- **Boot delta.** `--plan-only --boot-delta` is an explicit, opt-in exception
  to the preview law: it may fill the package store and registry cache, trusts
  no dependency and asks nothing; when a dependency's boot contribution would
  have to run, the delta is reported unavailable. Copying the workspace (about
  290 MB here) is accepted for the first version.
- **Tracked copies.** Recorded in a separate committed `vibe-derive.toml` with a
  two-way diff against the installed upstream and per-file base hashes.
- **First configured app.** `org.vibevm.agents/opencode@2.3.2` with a "Build"
  choice; the configured slot is published only with a vibe release that
  parses installer configuration.
- **Authored changes.** An origin capture is the digest of the whole file;
  `vibe change check` passes when an origin is unavailable and reports it.
- **Stack detection.** An unattended run with several or no detected stacks
  refuses and writes nothing. Recording `[active].stack` on install is a
  backlog item.
- **Platform adapters.** Their own consent spelling (`--elevate` family) now;
  lifting it into `remedy.args` is ADV-CONTROL's job. The full CLI path is
  proven on Linux; the gate logic on every platform by unit tests.
- **Live client checks.** A Windows-injectable home and the clients that turn
  out red on the first run are backlog findings after the run.
- **Attestations.** Assume the public GitHub repository; the owner confirms.
- **Generated files.** `specmap.json` is regenerated by the lead on `main`
  after an executor's commit is applied, never in a task worktree.

## Order of work in the base

1. ADV-FIX and ADV-CONTROL, in parallel.
2. ADV-SDD and ADV-CLIENTS.
3. ADV-NATIVE (needs ADV-CONTROL's contract: events, codes, consent).
3. ADV-RELEASE as soon as the owner approves the CI edits.
