# vibevm

**The disciplined runtime for spec-driven vibecoding.**

`vibe` is a CLI software project manager for spec-driven AI-assisted development. It resolves, installs, updates, inspects, and serves reusable specification packages, then computes the boot material that coding agents read inside a project.

The installable kinds are `flow`, `feat`, `stack`, `tool`, `mcp`, and `lang`.

## Status

The current release is **1.0.0**. This is a closed alpha, not a compatibility promise: **1.0.0 will break** while `public = false`. Until the owner declares the first public presentation, breaking changes may ship without migrations; the recovery path is re-init / re-fetch. Read [Alpha notes](docs/ALPHA-NOTES.md) before adopting the release and [CHANGELOG.md](CHANGELOG.md) before updating.

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

The full core-command reference starts at [`docs/commands/`](docs/commands/).

## Registries and search

Projects declare registries as an ordered array. `index_url` is optional; it points search and index-backed lookups at the registry index. The environment override `VIBEVM_INDEX_URL_<REGISTRY>` wins, and the literal value `"none"` disables index lookup for that registry.

```toml
[[registry]]
name = "vibespecs"
url = "https://github.com/vibespecs"
naming = "fqdn"
index_url = "https://github.com/vibespecs/index"
```

Use `vibe registry list` to inspect effective project registry declarations, `vibe registry test` to probe them, and `vibe search <query>` to query configured indexes.

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

`--offline` is also available through `VIBE_OFFLINE` and the user setting `[net].offline`. An offline miss is a hard, actionable error; it never silently falls back to a partial result. `vibe cache clean` removes content only after the operator chooses `--package`, `--older-than`, or `--all`.

## What a project contains

- `vibe.toml` — authored project/package/workspace declarations and direct requirements.
- `vibe.lock` — the exact resolved package graph and content identities.
- `vibedeps/` — per-project materialised package content, copied from the machine store.
- `spec/boot/STATIC.md` and `spec/boot/INDEX.md` — the computed agent boot lanes.
- `spec/WAL.md` — the project's living session checkpoint.

`vibe` keeps authored project specs separate from materialised dependencies. See [the loading model](docs/loading-model.md) and [architecture](docs/architecture.md) for the full layout.

## Documentation

- [Alpha notes](docs/ALPHA-NOTES.md) — compatibility posture and recovery after breaking updates.
- [Core command reference](docs/commands/) — operator-facing CLI pages checked against live `--help`.
- [Architecture](docs/architecture.md) — crate boundaries, seams, and data flow.
- [Runtime guide](RUNTIME-GUIDE.md) — machine requirements and runtime setup.
- [Developer guide](DEV-GUIDE.md) — clone, build, test, and contributor setup.
- [Changelog](CHANGELOG.md) — milestone history and release changes.
- [Site manifest](docs/SITE-MANIFEST.toml) — machine-readable documentation inventory.

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
