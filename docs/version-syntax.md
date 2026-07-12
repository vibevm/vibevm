# Version syntax in vibevm

Everywhere a vibevm command takes a package reference (`vibe install <pkgref>`, `[requires.packages]` in `vibe.toml`, `[provides].capabilities` in a package's `vibe.toml`, `[[override]] pkgref = ...`), the version part follows **Cargo / npm / Poetry conventions**. If you've used any of those, the syntax here is identical. If not, this doc covers everything you need.

## TL;DR

- Bare semver (`flow:wal@0.3.0`) is shorthand for **caret** (`^0.3.0`). To get strict-equal behaviour, write `=0.3.0`.
- `vibe install flow:wal` (no version) resolves to a concrete version and writes the **caret** form to `vibe.toml`. Same default as `cargo add` and `npm install`.
- The lockfile (`vibe.lock`) always pins to one exact version per package — that's the *materialisation*. The manifest carries the *declaration*.
- Use `--exact` when you want the manifest itself to pin strictly.

## Two-file model

vibevm has two files that together describe the project's package set:

| File | Role | Format example |
| --- | --- | --- |
| `vibe.toml` | **Declaration.** What the human asked for. Constraints, written in semver-syntax form (`^0.3.0`, `~1.2`, `=0.4.0`, etc.). Edited by humans, reviewed in PRs, mostly stable across versions. | `flow:wal@^0.1.0` |
| `vibe.lock` | **Materialisation.** The exact version the resolver picked, plus content hashes, source URLs, transitive graph. Regenerated on every install / update; mostly noise to humans, mostly signal to tooling. | `version = "0.1.3"` |

Same shape as Cargo (`Cargo.toml` ↔ `Cargo.lock`), npm (`package.json` ↔ `package-lock.json`), Poetry, Bundler. If you understand one, you understand the other.

When the two disagree (operator hand-edits `vibe.toml`, deletes `vibe.lock`, etc.), the manifest wins — the resolver re-derives the lockfile from scratch.

## Pkgref syntax

A package reference is `[<kind>:][<group>/]<name>[@<version>]`. Manifests store the qualified `<group>/<name>` form (`org.vibevm.world/wal`); the short `<kind>:<name>` form used in the examples below is CLI sugar resolved through the package index (see [`VIBEVM-SPEC.md` §7.1](../VIBEVM-SPEC.md)). The version part accepts every form `semver::VersionReq` accepts:

| Form | Meaning | Matches | Doesn't match |
| --- | --- | --- | --- |
| `flow:wal` | No constraint — latest stable. | `0.1.0`, `0.1.5`, `0.2.0`, `1.0.0` | — |
| `flow:wal@0.3.0` | **Caret shorthand** — same as `^0.3.0`. | `0.3.0`, `0.3.5` (pre-1.0 patch range) | `0.4.0`, `0.2.9` |
| `flow:wal@^0.3.0` | Caret, written explicitly. Same meaning. | `0.3.0`, `0.3.5` | `0.4.0`, `0.2.9` |
| `flow:wal@^0.3` | Caret with omitted patch. | `0.3.0`, `0.3.5` | `0.4.0`, `0.2.9` |
| `flow:wal@~0.3.1` | **Tilde** — patch-only within the same minor. | `0.3.1`, `0.3.5` | `0.4.0`, `0.3.0` (below) |
| `flow:wal@=0.3.0` | **Strict equal** — only that version. | `0.3.0` | everything else |
| `flow:wal@>=0.3, <1.0` | Compound range. | `0.3.0`, `0.5.7`, `0.9.99` | `0.2.9`, `1.0.0` |
| `flow:wal@*` | Any version (wildcard). Same as bare `flow:wal`. | everything | — |

`<kind>` is one of `flow`, `feat`, `stack`, `tool`. `<name>` is kebab-case.

## Caret in detail

Caret is the most common shape and the default for new installs. The rules differ between pre-1.0 and post-1.0 — this is where most surprises happen.

### Post-1.0 caret (`^1.2.3`)

Allows minor and patch bumps, no major:

```
^1.2.3 = >=1.2.3, <2.0.0
```

Matches `1.2.3`, `1.2.4`, `1.5.0`, `1.99.99`. Doesn't match `1.2.2` (below), `2.0.0` (next major).

### Pre-1.0 caret (`^0.x.y`)

Patch-only inside the same minor, because in semver pre-1.0 a minor bump is allowed to break API:

```
^0.3.0 = >=0.3.0, <0.4.0
^0.3.5 = >=0.3.5, <0.4.0
```

Matches `0.3.0`, `0.3.5`, `0.3.99`. Doesn't match `0.4.0`, `0.2.9`.

This is the regime every vibevm package today operates under (everyone is `0.x.y`). When packages cross 1.0, caret automatically widens.

### Pre-0.1 caret (`^0.0.z`)

Strict equal — `^0.0.3` matches only `0.0.3`. Same Cargo / npm / Poetry behaviour.

## Tilde in detail

Tilde is more conservative than caret — it allows patch only, regardless of pre-1.0 / post-1.0.

```
~1.2.3 = >=1.2.3, <1.3.0    (only patch in the 1.2.* line)
~0.3.1 = >=0.3.1, <0.4.0    (same as caret pre-1.0)
~1.2   = >=1.2.0, <1.3.0    (with omitted patch)
~1     = >=1.0.0, <2.0.0    (very wide — matches caret behaviour)
```

Reach for tilde when you've discovered an API quirk in a particular minor and want to stay in it without explicitly upgrading.

## Strict equal (`=`)

`=0.3.0` matches exactly `0.3.0` and nothing else. This is what you want when you need byte-for-byte reproducibility regardless of what's published later, and you don't trust caret's compatibility promise.

The `=` operator is required because **bare semver is caret**, not exact. `flow:wal@0.3.0` is `^0.3.0` — to pin strictly, write `flow:wal@=0.3.0`.

This is the one place vibevm differs from older or simpler tools. If you're coming from a `requirements.txt`-style world where `foo==1.2.3` is exact, the equivalent here is `foo@=1.2.3`.

## What lands in `vibe.toml` after `vibe install`

The manifest's `[requires].packages` records the **constraint** the user declared. The lockfile records the resolved version. Three rules govern what the manifest gets:

### Rule 1 — No version on CLI → caret of resolved version

```bash
vibe install flow:wal --assume-yes
```

Resolver picks `0.1.0` (latest stable). Manifest gets:

```toml
[requires.packages]
"org.vibevm.world/wal" = "^0.1.0"
```

This is the **default**, and matches `cargo add` / `npm install` / `poetry add`.

### Rule 2 — Explicit constraint on CLI → preserved verbatim

```bash
vibe install flow:wal@^0.1 --assume-yes
```

Manifest stores exactly what was typed:

```toml
[requires.packages]
"org.vibevm.world/wal" = "^0.1"
```

We don't tighten `^0.1` to `^0.1.0`, even though the resolver produces a concrete patch. The operator's wider declaration wins. Same for tilde, equal, and ranges:

| You typed | Manifest stores |
| --- | --- |
| `flow:wal@^0.1` | `"org.vibevm.world/wal" = "^0.1"` |
| `flow:wal@~0.1.0` | `"org.vibevm.world/wal" = "~0.1.0"` |
| `flow:wal@=0.1.0` | `"org.vibevm.world/wal" = "=0.1.0"` |
| `flow:wal@>=0.1, <0.3` | `"org.vibevm.world/wal" = ">=0.1, <0.3"` |

### Rule 3 — `--exact` overrides everything

```bash
vibe install flow:wal --exact --assume-yes
```

or

```bash
vibe install flow:wal@^0.1 --exact --assume-yes
```

Manifest pins to `=<resolved>`, regardless of CLI form:

```toml
[requires.packages]
"org.vibevm.world/wal" = "=0.1.0"
```

`--exact` is npm's `--save-exact` shape. Use it when you want strict reproducibility from the moment of install and don't want a later `vibe update` to wander.

## What `vibe update` does

`vibe update` walks the manifest's `[requires].packages`, re-resolves each constraint against the registry, and writes new pins into the lockfile when a newer matching version is available.

Since the manifest carries constraints (not pins), `vibe update` respects them:

- `flow:wal@^0.1.0` → may bump from `0.1.0` to `0.1.5` (caret allows patch).
- `flow:wal@~0.1.0` → same, only patch.
- `flow:wal@=0.1.0` → no-op, the constraint pins exactly.
- `flow:wal` (no version, legacy) → may bump anywhere up to latest.

`vibe update` never edits `vibe.toml` itself. To bump the manifest's constraint (`^0.1` → `^0.2`), re-run `vibe install flow:wal@^0.2`.

## What's in `vibe.lock`

The lockfile pins each package to one concrete version with a content hash:

```toml
[[package]]
kind = "flow"
name = "wal"
group = "org.vibevm"
version = "0.1.5"
registry = "vibespecs"
source_url = "https://github.com/vibespecs/flow-wal.git"
source_ref = "v0.1.5"
resolved_commit = "abc123…"
content_hash = "sha256:def456…"
dependencies = []
```

`version` is a single concrete `semver::Version`, not a constraint. The lockfile is the byte-level commitment; identity is `(group, name, version, content_hash)`. See [`lockfile-format.md`](lockfile-format.md) for the full schema.

`vibe.lock` also carries `[meta].root_dependencies` — a mirror of the manifest's `[requires].packages`. It's there so the lockfile is a self-contained snapshot of the solve state; the manifest is still authoritative for *what the user wants*.

## Comparison table — vibevm vs other ecosystems

| Concept | vibevm | Cargo | npm / Yarn / pnpm | Poetry | Bundler |
| --- | --- | --- | --- | --- | --- |
| Bare `1.2.3` means | `^1.2.3` (caret) | `^1.2.3` | `1.2.3` (exact, but written with caret in manifest by default) | `^1.2.3` | `1.2.3` (varies) |
| Caret operator | `^1.2.3` | `^1.2.3` | `^1.2.3` | `^1.2.3` | `~> 1.2.3` (different syntax, same idea) |
| Tilde operator | `~1.2.3` | `~1.2.3` | `~1.2.3` | `~1.2.3` | `~> 1.2.3` |
| Strict equal | `=1.2.3` | `=1.2.3` | `1.2.3` (exact write) or `--save-exact` | `==1.2.3` | `1.2.3` (exact) |
| Default on `add` | caret of resolved | caret of resolved | caret of resolved | caret of resolved | pessimistic of resolved |
| Exact-pin flag | `--exact` | `--locked` (different concept) | `--save-exact` | `--lock` (different concept) | — |
| Lockfile name | `vibe.lock` | `Cargo.lock` | `package-lock.json` / `yarn.lock` | `poetry.lock` | `Gemfile.lock` |

If you want one mental model: vibevm follows **Cargo's** rules to the byte. The semver crate it uses is the same one Cargo uses.

## FAQ

### Why caret by default and not exact?

Three reasons, none new:

1. **Patch fixes ship for free.** `^0.1.0` lets `vibe update` pick up `0.1.5` automatically — security fixes, bug fixes, doc improvements. Without caret, every patch update is a manual re-pin.
2. **Industry default.** Operators arriving from any of Cargo / npm / Poetry / Yarn already expect this shape. Diverging would be surprising for no benefit.
3. **You can still pin.** `--exact`, or write `@=0.1.0` explicitly. The default doesn't take options away; it picks the most useful one.

### Why does `flow:wal@0.3.0` not match `0.3.0` only?

Because bare semver in vibevm is caret shorthand, exactly like in Cargo. `flow:wal@0.3.0` means `^0.3.0`, which (pre-1.0) means `>=0.3.0, <0.4.0`. To pin only `0.3.0`, write `flow:wal@=0.3.0`.

### How does this interact with the registry's available versions?

vibevm asks the registry for the list of published versions, then walks them top-down looking for the highest one that matches the constraint. So `^0.1.0` against a registry that has `0.1.0`, `0.1.3`, `0.2.0` resolves to `0.1.3`. The resolver always prefers the highest matching stable version (pre-release versions are skipped unless the constraint explicitly requests one).

### What if I already have a `"flow:wal"` (no version) in my `vibe.toml`?

It keeps working — `vibe install` and `vibe update` both treat it as "any version". vibevm doesn't auto-rewrite legacy entries; new installs write caret, but existing entries are left to the operator. To upgrade a legacy entry to caret, re-run `vibe install flow:wal --assume-yes` and let it write the modern shape.

### Can I use this syntax in `[provides].capabilities`?

Yes — same parser. A package that declares `provides.capabilities = ["ui:landing-page@0.3.0"]` is saying "I provide version 0.3.0 of `ui:landing-page`". Other packages can require `ui:landing-page@^0.3` and the resolver will match.

For `provides`, an exact-form like `=0.3.0` and a caret-form like `0.3.0` carry the same practical meaning ("this package provides version 0.3.0"), because the resolver reads the version off the first comparator regardless of operator. Caret on the provides side is the cleaner shape.

### Can I omit `.minor` or `.patch`?

Yes:

```
flow:wal@1     → ^1.0.0    (matches 1.x.x)
flow:wal@1.2   → ^1.2.0    (matches 1.2.x — caret with omitted patch)
flow:wal@~1.2  → ~1.2.0    (same window for tilde)
```

### Where is the canonical specification of the syntax?

The version operators are exactly those of the [Rust `semver` crate](https://docs.rs/semver), which itself follows Cargo's conventions. vibevm parses every constraint string through `semver::VersionReq::parse`. Anything that crate accepts, vibevm accepts.

## See also

- [`commands/install.md`](commands/install.md) — full reference for `vibe install`, including the write-side decision matrix.
- [`commands/update.md`](commands/update.md) — how `vibe update` traverses constraints.
- [`lockfile-format.md`](lockfile-format.md) — the resolved-pin shape in `vibe.lock`.
- [`VIBEVM-SPEC.md`](../VIBEVM-SPEC.md) §7.1 / §7.5 — the spec definitions for pkgref / project manifest.
- [Cargo book — Specifying Dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html) — vibevm follows the same rules.
