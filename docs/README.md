# vibevm — user documentation

End-user reference for the `vibe` CLI and the package-authoring formats. For the full project specification and design decisions, see [`VIBEVM-SPEC.md`](../VIBEVM-SPEC.md) at the repo root and the PROP / FEAT documents under [`spec/`](../spec/).

## Commands

The `vibe` binary is the single entry point for every operation. Global flags `--json` (machine-readable output) and `--quiet` (one-line summary) work on every subcommand.

| Command | Purpose |
| --- | --- |
| [`vibe init`](commands/init.md) | Scaffold a new vibevm project tree. |
| [`vibe install`](commands/install.md) | Resolve and apply one or more packages from a registry. |
| [`vibe list`](commands/list.md) | Show the packages currently locked into the project. |
| [`vibe uninstall`](commands/uninstall.md) | Remove a package and its files (user-owned files preserved). |
| [`vibe registry sync`](commands/registry-sync.md) | Refresh per-package registry clones referenced by the lockfile. |
| [`vibe registry publish`](commands/registry-publish.md) | Maintainer-side: publish a package directory as a tagged release. |
| [`vibe version`](commands/version.md) | Print the binary's version. |

## Authoring a package

Three of the four package kinds in vibevm have their own authoring guide. The fourth (`tool`) is reserved for v2+ and not yet documented.

| Kind | Guide | Purpose |
| --- | --- | --- |
| `flow` | [authoring-flow.md](authoring-flow.md) | Discipline / process modules — protocols and conventions an AI session reads at boot. |
| `feat` | [authoring-feat.md](authoring-feat.md) | Functional features — the *what* of an application built from specs. |
| `stack` | [authoring-stack.md](authoring-stack.md) | Language / framework targets — the *how* feats compile against. |

## Related setup documentation

- [`RUNTIME-GUIDE.md`](../RUNTIME-GUIDE.md) — what an end-user needs on their machine to run `vibe`.
- [`DEV-GUIDE.md`](../DEV-GUIDE.md) — what a contributor to vibevm needs to clone, build, test, publish.
- [`manual-tests/`](../manual-tests/) — runnable smoke-tests used before tagging a milestone.
