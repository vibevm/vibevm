# vibevm

**The disciplined runtime for spec-driven vibecoding.**

`vibe` is a CLI software project manager for spec-driven AI-assisted development. It resolves, installs, updates, inspects, and serves reusable specification packages, then computes the boot material that coding agents read inside a project.

There are eight package kinds: `flow`, `feat`, `stack`, `tool`, `mcp`, `lang`, `doc` and `app`. A `doc` package documents other packages and is read, never installed.

## Status

The current release is **1.0.0**. This is a closed alpha, not a compatibility promise: **1.0.0 will break** while `public = false`. Until the owner declares the first public presentation, breaking changes may ship without migrations; the recovery path is re-init / re-fetch. Read [Alpha notes](docs-legacy/ALPHA-NOTES.md) before adopting the release and [CHANGELOG.md](CHANGELOG.md) before updating.

## Install from the Windows distributive

The release ships as `vibe-<version>-windows-x86_64.zip` (a static-CRT
`vibe.exe` — no VC++ Redistributable needed — plus `install.ps1`,
`uninstall.ps1`, `README-INSTALL.md`, `LICENSE.md`, `ALPHA-NOTES.md` and a
`SHA256SUMS.txt` integrity manifest). Extract it, then:

```powershell
powershell -ExecutionPolicy Bypass -File .\install.ps1
```

The installer imports the binary into the managed store under
`~/.vibe/opt`, activates it, and puts the shim directory on your user
`PATH`; open a new terminal and run `vibe --version`. The authored
sources of the installer live in [`distribution/windows/`](distribution/windows/).

## Install vibevm from this checkout

The first-run scripts build the checkout, install it through the VibeVM Version Manager, create the `vibe` shims, and update `PATH`:

```powershell
.\tools\first-run.ps1
```

```bash
bash tools/first-run.sh
```

Open a new terminal, then verify the result:

```bash
vibe --version
vibe self doctor
```

The default managed installation root is `~/.vibe/opt`: shims live in `~/.vibe/opt/bin/`, installed versions in `~/.vibe/opt/vibevm/`, and the active version is selected by the `current` pointer. To perform the initial install manually from the source tree:

```bash
cargo run -p vibe-cli -- self install
cargo run -p vibe-cli -- self doctor --fix
```

A ready-built `vibe` executable (for example, from the release zip) enters the
same managed inventory without a Rust toolchain:

```bash
vibe self import ./vibe.exe --tag 1.0.0 --use
```

Run `vibe self --help` for version switching, upgrades, removal, garbage collection, and relocation.

## Quick start

```bash
# Create a project in hello-vibe/.
vibe init hello-vibe

# Install a package and record it in vibe.toml + vibe.lock.
vibe install org.vibevm.world/wal --path hello-vibe

# Inspect and validate the result.
vibe list --path hello-vibe
vibe tree --plain --path hello-vibe
vibe check --path hello-vibe
```

`vibe install` with no package arguments reads `[requires].packages` from `vibe.toml`, which is the normal command after cloning an existing project:

```bash
vibe install --path hello-vibe
```

The manual's [command reference](vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0/vibevm/vibespecs/reference/commands.xml) lists every command with its options, and [the newcomer's route](vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0/vibevm/vibespecs/start/index.xml) walks from an empty folder to a project an agent understands.

## Registries and search

Projects declare registries as an ordered array. `index_url` is optional; it points search and index-backed lookups at the registry index. The environment override `VIBEVM_INDEX_URL_<REGISTRY>` wins, and the literal value `"none"` disables index lookup for that registry.

```toml
[[registry]]
name = "vibespecs"
url = "https://github.com/vibespecs"
naming = "fqdn"
index_url = "https://github.com/vibespecs/index"
```

Use `vibe registry list` to inspect effective project registry declarations, `vibe registry test` to probe them, and `vibe search <query>` to query configured indexes. The manual explains [what a registry is](vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0/vibevm/vibespecs/model/registries.xml) and [how to use a private one](vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0/vibevm/vibespecs/howto/use-a-private-registry.xml).

## Machine-global package store and offline work

Fetched package content is kept in the machine-global store at `~/.vibe/cache/`. This store is distinct from the registry clone cache under `~/.vibe/registries/`; the old project-local `.vibe/cache/` is not part of the 1.0.0 layout.

```bash
# Inspect and pre-warm the package store.
vibe cache path
vibe cache list
vibe cache add org.vibevm.world/wal --path hello-vibe

# Verify store integrity, repairing explicitly if requested.
vibe cache check
vibe cache check --repair --path hello-vibe

# Resolve and materialise without network access.
vibe install --offline --path hello-vibe
```

`--offline` is also available through `VIBE_OFFLINE` and the user setting `[net].offline`. An offline miss is a hard, actionable error; it never silently falls back to a partial result. `vibe cache clean` removes content only after the operator chooses `--package`, `--older-than`, or `--all`. The manual covers [the lock file and the store](vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0/vibevm/vibespecs/model/lock-and-store.xml) and [working offline](vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0/vibevm/vibespecs/howto/work-offline.xml).

## What a project contains

- `vibe.toml` — authored project/package/workspace declarations and direct requirements.
- `vibe.lock` — the exact resolved package graph and content identities.
- `vibedeps/` — per-project materialised package content, copied from the machine store.
- `vibevm/vibespecs/boot/STATIC.xml` and `vibevm/vibespecs/boot/INDEX.md` — the computed agent boot lanes.
- `vibevm/vibespecs/WAL.xml` — the project's living session checkpoint.

`vibe` keeps authored project specs separate from materialised dependencies. The manual explains the rule in [Two trees](vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0/vibevm/vibespecs/model/two-trees.xml), the computed reading order in [The boot lane](vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0/vibevm/vibespecs/model/boot-lane.xml), and the crates behind them in [How vibe is built](vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0/vibevm/vibespecs/architecture/how-vibe-is-built.xml).

## Documentation

The manual is itself a package, [`org.vibevm.core/vibevm-docs`](vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0/README.md) of kind `doc`, and lives in this checkout. Its pages are written in the same XML dialect as the specifications; the local reader and, later, the site render them. Read them on your own machine with nothing sent anywhere:

```bash
vibe cache add org.vibevm.core/vibevm-docs
vibe doc serve
```

The pages are grouped by what the reader is doing:

- [Start here](vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0/vibevm/vibespecs/start/index.xml) — what VibeVM is, installing `vibe`, the first project, what a project contains.
- [The model](vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0/vibevm/vibespecs/model/two-trees.xml) — two trees, packages and kinds, registries, versions, the lock file and the store, the boot lane.
- [How to](vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0/vibevm/vibespecs/howto/install-a-package.xml) — install, update and remove packages, work offline, use a private registry, publish, set up a workspace, read documentation locally.
- [Agents](vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0/vibevm/vibespecs/agent/ask-your-agent.xml) — ask your agent to do the work, give it the skill, how agents read this manual.
- [Lifecycle](vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0/vibevm/vibespecs/lifecycle/phases.xml) — the phases, build, package and deploy, scrape, extensions and providers.
- [Reference](vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0/vibevm/vibespecs/reference/commands.xml) — commands, the manifest, the lock file, settings and environment, machine formats.
- [Authoring](vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0/vibevm/vibespecs/authoring/write-a-flow.xml) — flows, feats and stacks, lang packages, tools and MCP servers, specs an agent can cite, documentation and its translations.
- [Architecture](vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0/vibevm/vibespecs/architecture/how-vibe-is-built.xml), [Glossary](vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0/vibevm/vibespecs/glossary/index.xml), [FAQ](vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0/vibevm/vibespecs/faq/index.xml) and [Diagnostics](vibevm/vibepacks/org.vibevm.core/vibevm-docs/v1.0.0/vibevm/vibespecs/diagnostics/errors.xml).

Every example on those pages runs against the real binary, every rule they quote resolves against the current specifications, and `vibe doc check` is the gate; [DEV-GUIDE.md §8](DEV-GUIDE.md#8-documentation) says how to run it.

Beside the manual:

- [Runtime guide](RUNTIME-GUIDE.md) — machine requirements, paths and environment for running `vibe`.
- [Developer guide](DEV-GUIDE.md) — clone, build, test, and contributor setup.
- [Changelog](CHANGELOG.md) — milestone history and release changes.
- [`docs-legacy/`](docs-legacy/) — the archived 1.0.0 alpha layer: alpha notes, the old command pages, architecture notes and the site manifest. Readable, not normative.

## Build and test from source

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

Source mirrors: [GitHub](https://github.com/vibevm/vibevm) and [GitVerse](https://gitverse.ru/vibevm/vibevm). Package registries are configured independently per project.

## License

vibevm is licensed under the [Universal Permissive License 1.0](LICENSE.md). Third-party components retain their own permissive licenses.
