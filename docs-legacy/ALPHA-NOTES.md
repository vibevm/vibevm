# vibevm 1.0.0 alpha notes

## The compatibility promise

The owner's semver mandate is literal: **«1.0.0 будет ломаться»**.

Version 1.0.0 is a closed alpha. The format registry remains in the pre-publication regime (`public = false`), and only the owner can end that regime by declaring the first public presentation. A tag, source push, registry entry, or download count does not flip it.

While `public = false`, breaking changes are free and unmigrated. Manifests, lockfiles, package layouts, registries, wire formats, command details, and generated projections may be replaced without a compatibility reader, codemod, parallel format world, or sunset window. This is intentional: alpha users re-create derived state instead of carrying migration machinery for shapes that are still changing quickly.

## Recovery after a breaking update

Start with the normal refresh:

```bash
vibe update --all
```

If the break changed the project scaffold or generated boot plumbing, re-run the idempotent initializer before fetching again:

```bash
vibe init --path .
vibe install
```

If a break prevents the old materialised graph or lockfile from being reconciled, preserve any authored work, then remove only the derived package tree and lockfile and resolve again:

```bash
rm -rf vibedeps
rm -f vibe.lock
vibe install
```

On PowerShell, the equivalent removal is:

```powershell
Remove-Item -Recurse -Force -LiteralPath vibedeps
Remove-Item -Force -LiteralPath vibe.lock
vibe install
```

If the machine-global package store itself contains surprising or obsolete content, inspect it first with `vibe cache check`. Then clean the narrowest useful target and re-fetch:

```bash
vibe cache clean --package org.vibevm.world/wal
# Or, for a deliberate full reset (confirmation-gated):
vibe cache clean --all
vibe install
```

`vibe cache clean` always requires one of `--package`, `--older-than`, or `--all`; the store is never evicted automatically.

## Identity rules are not API stability

Package groups, names, kinds, coordinates, manifest keys, and identity layouts may still be refactored during the alpha. Do not build automation around the assumption that today's spelling or file shape will survive.

Two honesty rules still apply through those changes:

- an existing identifier is never silently reused for a different meaning; a rename, forwarding record, tombstone, or explicit break must make the change visible;
- bytes at a frozen coordinate are not silently replaced. New work gets a new coordinate, or a loud conflict.

These rules prevent a wrong answer from looking correct; they do not promise that an old answer remains supported.

## Known alpha limitations (2026-08-20)

*(The three limitations recorded here on 2026-08-20 — `vibe tree
--quiet` printing the full tree (B-097), the non-TUI `vibe prefs`
surface running schema-less (B-096), and the stale `--show-origins`
flag spelling in two specs (B-098; the real surface is the
`vibe prefs show-origins [key]` subcommand) — were all fixed the same
day by the P2 backlog wave and are no longer shipped behaviour.)*

## Where to look before updating

- [`CHANGELOG.md`](../CHANGELOG.md) — release and milestone changes, including operator-visible behavior.
- [`formats/breaks/`](../formats/breaks/) — precise notes for format breaks and their recovery implications.
- `vibe <command> --help` — the live CLI contract for the binary you are actually running.

When these notes and an older document disagree about current behavior, prefer the live binary and the newest changelog entry.
