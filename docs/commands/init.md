# `vibe init`

Scaffold a project, a package, or a package group.

## Usage

```text
vibe init [OPTIONS] [POSITIONAL]...
```

The live positional forms are:

```text
vibe init
vibe init <project-name>
vibe init <group> <project-name>
vibe init <group>/<package>
vibe init package <group>/<package> [path]
vibe init group <group> [path]
```

## Options

| Option | Effect |
| --- | --- |
| `--path <PATH>` | Back-compatible target-directory form; a positional path wins. |
| `--name <NAME>` | Set the project name; otherwise use the target basename. |
| `--stack <STACK>` | Pre-set the active stack name; it does not install the stack. |
| `--registry-url <URL>` | Override the registry URL written to `vibe.toml`. |
| `--registry-ref <REF>` | Override the default registry ref `main`. |
| `--no-registry` | Do not write a registry declaration. |
| `--kind <KIND>` | Set package kind; default `tool`. |
| `--version <VERSION>` | Set package/project version. |
| `--author <AUTHOR>` | Set an author; repeatable. |
| `--license <LICENSE>` | Set the package license; default `UPL-1.0`. |
| `--description <TEXT>` | Set the one-line package description. |
| `--format <simple\|normal>` | Select package format; default `simple`. |
| `--link <static\|dynamic>` | Set boot-snippet link type. |

The common `--json`, `--quiet`, `--invoked-by`, `--unattended`, and `--offline` options are also available.

## Examples

```bash
vibe init hello-vibe
vibe init org.example hello-vibe
vibe init package org.example/tools ./hello-vibe
vibe init group org.example ./hello-vibe
```

## Related

- [`vibe install`](install.md)
- [`vibe check`](check.md)
