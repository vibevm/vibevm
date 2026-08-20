# `vibe search`

Search configured registry indexes by free text, or perform an exact Package URL lookup.

## Usage

```text
vibe search [OPTIONS] <QUERY...>
vibe search [OPTIONS] --purl <PURL>
```

Free-text tokens are normalized by the server; at least one non-stopword token of two or more characters must remain. `--purl` is mutually exclusive with free text.

## Options

| Option | Effect |
| --- | --- |
| `--purl <PURL>` | Find package or subskill bindings to an exact Package URL. |
| `--kind <KIND>` | Filter free-text hits to one package kind. |
| `--registry <REGISTRY>` | Search one configured registry by name. |
| `--limit <LIMIT>` | Maximum hits fetched per registry; default `20`. |
| `--full-scan` | For indexless GitHub registries, fall back to a slower organization walk. |
| `--no-cache` | Bypass and do not update `~/.vibe/search-cache/`. |
| `--cache-ttl <SECONDS>` | Override the default one-hour search-cache TTL. |
| `--path <PATH>` | Select the project containing `vibe.toml`; default `.`. |
| `--offline` | Forbid network access; unavailable remote data becomes a hard error. |

The common `--json`, `--quiet`, `--invoked-by`, and `--unattended` options are also available.

## Examples

```bash
vibe search write ahead log
vibe search wal --kind flow --registry vibespecs
vibe search --purl pkg:cargo/serde
vibe search wal --no-cache --limit 50
```

Registry index locations come from `VIBEVM_INDEX_URL_<REGISTRY>`, then `[[registry]].index_url`, then the built-in default when one exists.

## Related

- [`vibe registry`](registry.md)
- [`vibe install`](install.md)

