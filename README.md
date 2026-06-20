# vibevm

**The disciplined runtime for spec-driven vibecoding.**

`vibe` is a CLI software project manager for spec-driven AI-assisted development. It manages installable building blocks — `flow`s (process disciplines), `feat`s (functional features), `stack`s (language/framework targets), `tool`s (utilities) — and assembles them into project-level spec content that AI agents read at session boot. Eventually (v1.5) it also drives the LLM-backed code-generation that turns those specs into working software.

The headline pitch: keep the discipline of structured specifications, the leverage of AI-assisted coding, and the reproducibility of a real package manager — all in one binary.

## Status

Pre-1.0 development. Phase A of M1.1-revision is complete on `main`:

- M0 walking skeleton: `vibe init` / `install` / `list` / `uninstall` against a local-directory registry.
- M1.1: git-backed registry with shell-out `GitBackend`, `~/.vibe/registries/` cache.
- M1.1-revision: decentralized per-package registry (`vibespecs/<kind>-<name>` under an org URL), `[[registry]]` array + `[[mirror]]` + `[[override]]` schema, content-addressed identity, lockfile schema v2, `MultiRegistryResolver`, `vibe registry sync` walking per-package clones, transitive dependency resolution via `NaiveDepSolver`, content_hash integrity check on plan, `vibe registry publish` maintainer command with GitVerse adapter, JTD wire-contract scaffolding.

170+ tests across the workspace, clippy clean with `-D warnings` on every commit.

What's still open: live migration of the three demo packages into the `vibespecs` org (one push away — needs owner sign-off), then the M1.2 (`vibe update`) / M1.3 (`vibe check`) / M1.4 (`vibe show`) command slices, then M1.5 (`vibe build` — the LLM-backed code-generation milestone). Full plan in [`ROADMAP.md`](ROADMAP.md).

## Quick start

```bash
# Build the binary.
cargo build --release --workspace

# Scaffold a new project tree.
target/release/vibe init --path my-project

# Install the canonical Write-Ahead Log discipline flow from the local fixture.
target/release/vibe install flow:wal \
    --registry fixtures/registry \
    --path my-project \
    --assume-yes

# Inspect what's installed.
target/release/vibe list --path my-project
```

For the live registry path (against `git@gitverse.ru:anarchic/vibespecs.git`, the M1.1 monorepo, until live migration to the per-package model lands), drop the `--registry` flag — `vibe init` writes the default registry into `vibe.toml`.

Full command reference: [`docs/commands/`](docs/commands/). Authoring guides for new packages: [`docs/authoring-{flow,feat,stack}.md`](docs/).

## First run — install vibevm with VVM

vibevm distributes itself: the `vibe` binary manages its own versions through the **VibeVM Version Manager** (VVM — `vibe self`, [PROP-019](spec/common/PROP-019-version-manager.md)). You're in the source tree, so nothing needs cloning.

**Fastest path** — a bootstrap script that does everything below (build, install the first version, write the shims, put `vibe` on PATH):

```bash
bash tools/first-run.sh           # bash · Git Bash · macOS · Linux
```
```powershell
.\tools\first-run.ps1             # Windows PowerShell
```

Then open a **new terminal** and run `vibe self ls`. The script edits your durable PATH; to try VVM *without* touching `~/opt`, use the isolated one-liner at the end of this section instead.

Prefer to run the steps yourself? Here they are:

```bash
# Build the current checkout and install it as your first version.
cargo run -p vibe-cli -- self install

# See it — the active version is marked with `*`.
cargo run -q -p vibe-cli -- self ls
```

`self install` compiles the checkout and publishes it as **instance 1** under `~/opt/vibevm/versions/branch/<current-branch>/1/`, then flips the live `current` pointer to it. That instance is now the active version.

To run plain `vibe` from any shell, set up the shims and PATH once:

```bash
# Write the shims into ~/opt/bin and put ~/opt/bin on PATH (asks for consent).
cargo run -p vibe-cli -- self doctor --fix
```

Open a **new terminal** and `vibe self ls` works. From then on the loop is fast: `vibe self install` rebuilds, flips `current`, and the next `vibe` in the same shell picks it up — no console reload, and the running version is never locked while you reinstall.

Good to know on the first run:

- **No selector means `latest`, which in-tree means *this checkout*.** VVM records it as an *external* source and remembers this tree's path, so a later `vibe self install` from anywhere rebuilds from here (a *linked rebuild*) — your sources are never copied into the install root.
- **The first build is a full build.** It compiles into a managed `~/opt/vibevm/build` target dir, kept separate from this repo's own `target/` (so it never relinks a `vibe` that is running); later builds are incremental, and a byte-identical rebuild makes no new instance.
- **Switch and inspect:** `vibe self use <selector>` switches the active version live, `vibe self current` / `vibe self which` show it, and `vibe vars` prints the variables vibevm actually uses versus your environment.
- **Try it without touching `~/opt`:** prefix the install with an isolated root — `VIBEVM_INSTALL_ROOT="$(mktemp -d)" cargo run -p vibe-cli -- man install`.

Already built the binary (`cargo build --release --workspace`)? Use it directly instead of `cargo run` — e.g. `target/release/vibe self install`.

## Documentation map

| File | Audience | Purpose |
| --- | --- | --- |
| [`VIBEVM-SPEC.md`](VIBEVM-SPEC.md) | implementers / reviewers | The full project specification — package model, registry, CLI surface, build pipeline, acceptance checklists. Owner-frozen; amendments require explicit approval. |
| [`ROADMAP.md`](ROADMAP.md) | implementers / reviewers | Long-form milestone plan with the "why" each milestone exists. |
| [`spec/`](spec/) | implementers | PROP / FEAT documents with the binding architectural decisions. Start at [`spec/common/PROP-000.md`](spec/common/PROP-000.md), then any module-specific PROP under `spec/modules/`. |
| [`docs/`](docs/) | end users | CLI reference + per-kind authoring guides. |
| [`RUNTIME-GUIDE.md`](RUNTIME-GUIDE.md) | end users | What you need on your machine to run `vibe`. |
| [`DEV-GUIDE.md`](DEV-GUIDE.md) | contributors | What you need to clone, build, test, and publish from this repo. |
| [`CLAUDE.md`](CLAUDE.md) (and identical `AGENTS.md` / `GEMINI.md`) | AI agents working in the repo | The four non-negotiable rules + memory discipline + boot read-order. |
| [`manual-tests/`](manual-tests/) | maintainers | Human-runnable smoke-tests; one file per scenario, walked before tagging a milestone. |
| [`TASKS.md`](TASKS.md) | active contributors | Live checklist for the current work-slice. |
| [`spec/WAL.md`](spec/WAL.md) | active contributors | Project-state checkpoint; rewritten each session, not appended. |
| [`CHANGELOG.md`](CHANGELOG.md) | everyone | Curated milestone-by-milestone history of what landed when. |

## The four kinds

Every installable artefact in vibevm is exactly one of:

- **`flow`** — a discipline / process module. Specs read at session boot that govern *how the team works* (commit conventions, WAL protocol, code-review rules). [Authoring](docs/authoring-flow.md).
- **`feat`** — a functional feature. The *what* of a project, expressed as specification — purpose, behaviour rules, acceptance criteria. Stack-agnostic at authoring time. [Authoring](docs/authoring-feat.md).
- **`stack`** — a language / framework target. The *how* a feat becomes real software — language, framework, conventions, capabilities provided. [Authoring](docs/authoring-stack.md).
- **`tool`** — utilities. Reserved for v2+; not yet authorable.

## Workspace layout

```
crates/
├── vibe-cli/           # The `vibe` binary entry point (clap, output, dispatch).
├── vibe-core/          # Manifest schemas, package identity, capabilities, errors.
├── vibe-graph/         # Task graph builder + runner (M1.5 build pipeline).
├── vibe-registry/      # Git-backed registry: ShellGit, GitPackageRegistry, MultiRegistryResolver.
├── vibe-resolver/      # DepProvider / DepSolver traits, NaiveDepSolver impl.
├── vibe-install/       # plan_install / apply_install / register_installed pipeline.
├── vibe-publish/       # RepoCreator trait, GitVerseCreator, vibe registry publish.
├── vibe-llm/           # LLM provider abstraction (M1.5 — stubs today).
├── vibe-check/         # Spec linter (M1.3 — stubs today).
└── vibe-wire/          # JTD-codegen'd wire types (populated by `cargo xtask codegen`).
xtask/                  # `cargo xtask codegen` and check-codegen tooling.
schemas/                # JTD source-of-truth for every wire contract.
fixtures/registry/      # Hermetic e2e test fixture (M0 monorepo layout).
manual-tests/           # Live smoke-tests — one file per scenario.
docs/                   # End-user reference docs.
spec/                   # PROP / FEAT documents and the WAL.
tools/                  # Project-local toolchain binaries (gitignored content).
```

## Building and testing

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
```

Codegen for JTD types (after the one-time install of `jtd-codegen` per [`tools/jtd-codegen/README.md`](tools/jtd-codegen/README.md)):

```bash
cargo xtask codegen          # regenerate from schemas/
cargo xtask check-codegen    # CI uses this to assert no schema drift
```

## Contributing

Read [`CLAUDE.md`](CLAUDE.md) before your first commit — the four non-negotiable rules (attribution, Conventional Commits, group by meaning, autonomy on routine changes only) apply to every contribution. Setup procedure is in [`DEV-GUIDE.md`](DEV-GUIDE.md). Process disciplines for the project itself live under `spec/boot/` and are loaded at every session start.

Issues / PRs: this repo lives at `https://gitverse.ru/anarchic/vibevm`. The package registry is a separate org at `https://gitverse.ru/vibespecs` (currently transitioning from the M1.1 monorepo at `anarchic/vibespecs`).

## License

vibevm itself ships under the proprietary EULA placeholder in [`LICENSE.md`](LICENSE.md) for the moment; the eventual target is UPL 1.0. Third-party dependencies are permissive-only (MIT / Apache-2.0 / BSD / Unlicense; MPL-2.0 case-by-case; GPL / AGPL / LGPL forbidden) per [PROP-000 §3](spec/common/PROP-000.md).
