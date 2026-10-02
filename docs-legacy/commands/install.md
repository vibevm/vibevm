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
| `-g`, `--global` | Install one user-scoped application or MCP package. MCP packages use a dedicated project under the Vibe settings root, independent of the current directory. |
| `--agent <AGENT>` | For `-g mcp:...`, register the server with the selected agent (`claude`, `cursor`, `opencode`, `codex`, `qwen-code`, or `all`; `claude-desktop` supports local stdio servers only). Without it, an interactive terminal asks. Scripts must pass it. |
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

# Install and register a remote MCP package for this user.
vibe install -g mcp:ai.lev/fpf-mcp --agent qwen-code
```

Fetched content enters the machine store at `~/.vibe/cache/`; project materialisation lives in `vibedeps/`, and the exact graph is recorded in `vibe.lock`.

For a project MCP package, `vibe install mcp:<group>/<name>` only materialises the package. Register it with a client separately using [`vibe mcp install`](mcp-install.md). `-g mcp:<group>/<name>` materialises it in the dedicated user MCP project and registers it with the selected user-scope agent. The `mcp:` prefix and full `<group>/<name>` are required for global routing; a bare coordinate is ambiguous with global applications. A remote URL-only package requires no binary build. The user's MCP client, rather than vibe, contacts its HTTPS endpoint.

## Related

- [`vibe update`](update.md)
- [`vibe uninstall`](uninstall.md)
- [`vibe cache`](cache.md)
