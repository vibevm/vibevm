# `vibe install`

Resolve and install one or more packages into a project. With package arguments, the command also adds or updates the corresponding entries in `vibe.toml`; without arguments, it installs `[requires].packages` already declared there.

## Usage

```text
vibe install [OPTIONS] [PACKAGES]...
```

A package reference accepts the qualified `<group>/<name>[@<version>]` form or CLI short form `<kind>:<name>[@<version>]`. Package kinds are `flow`, `feat`, `stack`, `tool`, `mcp`, and `lang`.

## Important options

| Option | Effect |
| --- | --- |
| `--path <PATH>` | Select the project directory; default `.`. |
| `--assume-yes` | Skip the install confirmation. |
| `--exact` | Record the resolved version as an exact `=x.y.z` constraint. |
| `--features <FEATURES>` | Activate repeatable or comma-separated features on every root package. |
| `--no-default-features` | Do not activate `[features].default`. |
| `--all-features` | Activate every non-private feature; wins over `--features`. |
| `--language <LANGUAGE>` | Override the project's BCP-47 language preference for this install. |
| `--solver <naive\|sat\|resolvo>` | Select the dependency solver; default `resolvo`. |
| `--auth-required` | Treat 401/403 from a public registry as a hard failure instead of walking on. |
| `--offline` | Forbid network access; every dependency must resolve from local sources. |
| `--registry <REGISTRY>` | Use a local-directory registry compatibility path. |
| `--git <URL>` plus one of `--tag`, `--branch`, `--rev` | Install the single positional package from a direct git source. |
| `--git-auth <AUTH>` / `--git-token-env <ENV_VAR>` | Configure auth for that direct git source. |
| `--prefer-embedded`, `--no-prefer-embedded`, `--no-default-registry` | Control the embedded-registry layer. |
| `--prefer-local`, `--no-prefer-local` | Control project-local package precedence. |
| `--allow-hooks` | Run declared install hooks without an interactive consent prompt. |

The common `--json`, `--quiet`, `--invoked-by`, and `--unattended` options are also available. Run `vibe install --help` for the complete mutual-exclusion and precedence text.

## Examples

```bash
# Install one package and record its resolved constraint.
vibe install org.vibevm.world/wal

# Reproduce everything already declared in vibe.toml.
vibe install

# Work from local package content only.
vibe install --offline --path ./my-project

# Install one direct git source.
vibe install tool:example --git https://example.com/example.git --tag v1.0.0
```

Fetched content enters the machine store at `~/.vibe/cache/`; project materialisation lives in `vibedeps/`, and the exact graph is recorded in `vibe.lock`.

## Related

- [`vibe update`](update.md)
- [`vibe uninstall`](uninstall.md)
- [`vibe cache`](cache.md)

