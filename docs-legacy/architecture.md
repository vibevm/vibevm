# vibevm — architecture

This is the contributor-facing map of the 1.0.0 tree. The source code is the authority for current implementation detail; [`VIBEVM-SPEC.md`](../VIBEVM-SPEC.md) and the PROP documents under [`spec/`](../spec/) carry the design contracts.

## System model

Five layers explain the product:

1. **Identity.** A package coordinate is `(group, name, version)` and its delivered identity also carries `content_hash`. `kind` is metadata, not identity; the current kinds are `flow`, `feat`, `stack`, `tool`, `mcp`, and `lang`.
2. **Registry.** A project declares ordered `[[registry]]` entries, optional mirrors and overrides, and an optional `index_url`. Registry indexes accelerate discovery; fetched bytes still have to satisfy the package identity.
3. **Machine store.** Fetched package trees are stored once under `~/.vibe/cache/`, keyed by `(group, name, version)`. The store is shared by every project on the machine and can answer resolution even when an upstream registry no longer does.
4. **Project materialisation.** The selected graph is copied into the workspace-root `vibedeps/`; exact resolution and provenance live in the single workspace `vibe.lock`.
5. **Computed boot.** Installed package contributions are projected into the generated `spec/boot/STATIC.md` and `spec/boot/INDEX.md` lanes. Authored project content remains separate and is never replaced by dependency content.

```text
registry index / registry repos / local sources
                    │
                    ▼
          resolver + dependency solver
                    │
                    ▼
       ~/.vibe/cache/<group>/<name>/v<version>/
                    │
                    ▼
        <workspace>/vibedeps/ + vibe.lock
                    │
                    ▼
        computed boot lanes for each entry point
```

With `--offline`, the top network leg is disabled. Resolution must be satisfied by the machine store, embedded/project-local packages, `file://` mirrors, or already materialised locked content; a miss is loud.

## Workspace crate map

The repository keeps capabilities in libraries and exposes them through thin surfaces.

| Area | Crates | Responsibility |
| --- | --- | --- |
| Core vocabulary | `vibe-core` | Package/capability identity, manifests, lockfile, content hashes, shared errors and values. |
| Spec and trace model | `vibe-spec`, `vibe-trace` | `spec://` parsing/routing and fresh code↔spec traceability queries. |
| Wire contracts | `vibe-wire` | Generated Rust types plus the behavior layer for registered machine formats. |
| Preferences and actions | `vibe-settings`, `vibe-actions` | Layered user/repository settings and frontend-independent addressable actions. |
| Status engine | `progress-core` | Parser, validator, roll-up, reports, and campaign state for inline `<status>` markup. |
| Registry and store | `vibe-registry` | Registry access, git transport, mirrors/overrides, redirects, the clone cache, and the machine-global package store. |
| Dependency solving | `vibe-resolver` | The `DepProvider` / `DepSolver` seams and `resolvo`, SAT, and naive solver cells. `resolvo` is the CLI default. |
| Installation | `vibe-install`, `vibe-workspace` | Plan/apply orchestration, workspace discovery, materialisation, boot computation, and lockfile recording. |
| Publishing and index | `vibe-publish`, `vibe-index` | GitHub/GitVerse publication, redirects, post-publish hooks, and the optional searchable registry catalog/server. |
| Operator surfaces | `vibe-cli`, `vibe-mcp` | The `vibe` command line and the stdio MCP server/integration manager. |
| Inspection | `vibe-check` | Deterministic project checks with structured findings. |
| Test infrastructure | `vibe-test-support` | Isolation of per-user settings/cache state in tests. |
| Reserved runtime slots | `vibe-graph`, `vibe-llm` | Deliberate placeholders that are not part of the 1.0.0 operator workflow. |
| Repository tooling | `xtask` | Code generation/checks, specmap, engine synchronization, source mirroring, and other maintainer gates. |

The normal dependency direction is surface → orchestrator → seam/library → core types. Authentication and filesystem choices are resolved at composition roots; domain libraries return typed values rather than prompting or formatting terminal output.

## Main install path

`vibe install` follows this sequence:

1. Discover the absolute workspace root and load `vibe.toml`, `vibe.lock`, user configuration, registries, mirrors, overrides, and local package sources.
2. Qualify root package references and build provider cells.
3. Solve the complete graph through the selected `DepSolver`; `resolvo` is the default, with `sat` and `naive` selectable.
4. Resolve and fetch every selected identity. A machine-store hit is reusable local content; a miss walks allowed sources and inserts the extracted tree into the store without rewriting an existing entry.
5. Build a plan, validate managed instruction-file blocks, and obtain confirmation unless a documented non-interactive mode supplies it.
6. Materialise the graph into `vibedeps/`, regenerate computed boot artifacts, and prune stale materialised slots.
7. Record the exact graph, source provenance, active features/subskills, and content hashes in `vibe.lock`; update direct requirements when package arguments were supplied.
8. Render one of the human, quiet, or registered JSON outputs.

`vibe update` re-enters the same resolution/materialisation system with update intent. `vibe uninstall` removes a project declaration/materialisation. `vibe cache clean` is the separate operator action that removes machine-store content.

## Important seams

- `GitBackend` isolates git operations. The production implementation shells out to the system git, preserving existing SSH-agent and credential-helper behavior.
- `Registry` represents version enumeration, resolution, and fetching across local and git-backed sources.
- `MultiRegistryResolver` owns ordered registry walking, mirrors, overrides, authentication discrimination, offline posture, and store-aware fetches.
- `DepProvider` is the solver's view of available package/version facts; `DepSolver` turns roots into a resolved graph.
- `InstallSource` separates the install transaction from concrete registry/solver cell construction.
- `RepoCreator` isolates host-specific organization/repository creation. GitHub and GitVerse adapters are present in the live tree.

## Storage layout

The two per-user stores have different jobs:

```text
~/.vibe/
├── cache/                              # extracted, identity-keyed package store
│   └── <group>/<name>/
│       ├── v<version>/                 # package tree, write-once by vibe
│       └── v<version>.sha256           # integrity sidecar, outside the hashed tree
└── registries/                         # git clone/cache state for registry transport
```

`$VIBE_SETTINGS` relocates the settings home and therefore the package store with it. Registry clone-cache configuration remains separate. A project's `.vibe/` directory may contain settings and agentic state, but 1.0.0 does not keep a second project-local package cache there.

Project state is:

```text
<workspace>/
├── vibe.toml                           # authored declarations
├── vibe.lock                           # exact resolved graph
├── vibedeps/                           # materialised dependency trees
└── spec/
    ├── boot/STATIC.md                  # generated priority lane
    ├── boot/INDEX.md                   # generated ordered manifest
    └── WAL.md                          # authored living checkpoint
```

See [the loading model](loading-model.md) for boot ordering and ownership rules.

## Wire and authored formats

- Human-authored configuration and lock state use TOML.
- Machine outputs and HTTP/MCP contracts use registered schemas and generated types from `schemas/` and `vibe-wire`.
- Format breaks are recorded under [`formats/breaks/`](../formats/breaks/). While the project remains in the pre-publication alpha regime, derived formats are rebuilt rather than migrated.

## Reading order for a contributor

1. [`README.md`](../README.md) and [`docs/ALPHA-NOTES.md`](ALPHA-NOTES.md).
2. The repository instruction file and the generated boot manifest it names.
3. [`spec/WAL.md`](../spec/WAL.md) for current state.
4. [`VIBEVM-SPEC.md`](../VIBEVM-SPEC.md) and the PROP governing the subsystem being changed.
5. The relevant crate-level module documentation and tests.

For the public operator surface, prefer the live `vibe <command> --help` and the short pages under [`docs/commands/`](commands/).
