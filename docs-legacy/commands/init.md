# `vibe init`

Scaffold a project or package root, or add a nested package or group to a
project. A new root declares exactly one of `[project]` and `[package]` in its
`vibe.toml`; both use the same target-directory scaffold.

## Usage

```text
vibe init [OPTIONS] [POSITIONAL]...
```

The root and legacy nested forms are:

```text
vibe init [directory] [--type project|package] [--group <group>] [--name <name>]
vibe init package <group>/<package> [path]
vibe init group <group> [path]
```

An interactive new root explains the `group/name` coordinate, then asks for
group, name, root role (`project` or `package`), and remaining metadata. It
requires a valid reverse-DNS group for either role. In a script, omit `--type`
to keep the project default; a noninteractive project may omit `--group`, but
a package root requires it. Explicit flags skip their matching questions.
The older `init package` and `init group` forms still create nested content;
root role flags cannot be combined with them. A root package does not create
a second nested package.

## Options

| Option | Effect |
| --- | --- |
| `--path <PATH>` | Back-compatible target-directory form; a positional path wins. |
| `--type project\|package` | Select the root declaration. Without it, scripts create a project; an attended new root asks. |
| `--group <GROUP>` | Set the reverse-DNS group. Required for a package root; optional for a noninteractive project. |
| `--name <NAME>` | Set the root project/package name; otherwise use the target basename. |
| `--stack <STACK>` | Pre-set the active stack name; it does not install the stack. |
| `--registry-url <URL>` | Override the registry URL written to `vibe.toml`. |
| `--registry-ref <REF>` | Override the default registry ref `main`. |
| `--no-registry` | Do not write a registry declaration. |
| `--kind <KIND>` | Set package kind; default `tool` for a package. An MCP root without a server declaration is refused. |
| `--translates <COORDINATE>` | Scaffold a doc translation in the legacy nested `init package` form. |
| `--version <VERSION>` | Set the root or nested package version; default project `0.0.1`, package `0.1.0`. |
| `--author <AUTHOR>` | Set an author; repeatable. |
| `--license <LICENSE>` | Set the package license; default `UPL-1.0`. |
| `--description <TEXT>` | Set the one-line package description. |
| `--format <simple\|normal>` | Select package format; default `normal`. |
| `--link <static\|dynamic>` | Set the nested package boot-snippet link type; the nested `init package` default is `dynamic`. |

The common `--json`, `--quiet`, `--invoked-by`, `--unattended`, and `--offline` options are also available.

## Examples

```bash
vibe init ./hello-vibe --type project --group org.example --name hello-vibe
vibe --unattended init ./package-name --type package --group org.example --name package-name --kind tool
vibe init package org.example/tools ./hello-vibe --kind tool
vibe init group org.example ./hello-vibe
```

New package names may contain an interior single hyphen (`package-name`);
leading, trailing, or doubled hyphens are refused. Re-running `init` preserves
authored manifest and boot files. A request to change an existing root's role,
group, or name is refused instead of silently converting it.

## Related

- [`vibe install`](install.md)
- [`vibe check`](check.md)
