# PROP-030 — The embedded registry {#root}

`spec://vibevm/modules/vibe-registry/PROP-030`

**Status:** proposed (2026-07-13).
**Depends on:** [PROP-002](PROP-002-decentralized-registry.md) (registry walk,
`source_kind`), [PROP-019](../../common/PROP-019-version-manager.md) (VVM install
records, `origin`, `source_path`), [PROP-009](../vibe-workspace/PROP-009-loading-model.md)
(boot assembly consuming the resolution).

## 1. Motivation {#motivation}

vibevm ships its own packages in-tree, under `packages/`. When `vibe` is built
and installed **from that source tree** (`vibe self install` / `self update`,
recorded with `origin = "external"` and a `source_path`), those packages are
sitting on disk right next to the binary — yet a consuming project still has to
name them explicitly with `--registry <path>` on every command, or hand-write a
`[[registry]]` block. With the packages **not published to any network registry**
(the current state), that friction is the *only* way to use them at all.

This PROP makes the in-tree `packages/` of a source-installed `vibe` an
**ambient default registry** — resolved automatically, with zero configuration
in the consuming project.

### 1.1 Two audiences, opposite precedence {#audiences}

The central design fact — **normative, recorded here at the owner's request**:

- **The vibevm developer** runs a `vibe` built from a source tree and is
  *developing vibevm on vibevm*. Their embedded `packages/` is the **source of
  truth**: on a coordinate clash with a network package, **embedded wins**. You
  are testing your local edits; a published copy of the same `(group, name,
  version)` must not silently shadow them.
- **The end user** (a future distribution of vibevm) is *consuming* vibevm.
  Any packages bundled with a distribution are a **fall-through**: declared /
  published registries win, and the bundle only fills gaps (offline defaults).

The same mechanism, opposite precedence, selected by **who you are** — which
`vibe` already knows from the active install's `origin`. This developer↔user
inversion is the reason the precedence is not a single fixed rule.

## 2. The embedded registry {#registry}

An **embedded registry** is a local-directory registry ([PROP-002](PROP-002-decentralized-registry.md#local),
the M0 `LocalRegistry` shape — `packages/<group>/<name>/<version>/`) that `vibe`
derives from its **own active install**, not from the consuming project:

1. `vibe` reads its active VVM install record (`~/opt/vibevm/state.toml`, the
   record whose slot holds `current_exe`).
2. If that record has `origin = "external"` and a `source_path`, and
   `<source_path>/packages` exists, that directory is the embedded registry.
3. It is injected into resolution for **every** project automatically. The
   project's `vibe.toml` is never read for it and never written — the default is
   ambient, carried by the `vibe` binary, not the project.

Because it is derived from the **active** install, `vibe self use <instance>`
re-points the embedded registry at *that* instance's `source_path`. Two source
checkouts installed side by side each carry their own embedded registry;
switching the active version switches the default.

A `managed`-origin install (no source tree) has no embedded registry from a
source path. A future distribution that bundles packages names its bundle
location through the same seam (§3.2), at end-user precedence.

## 3. Precedence {#precedence}

Resolution keeps PROP-002's explicit-source short-circuits **above** the
embedded registry — an explicit per-dependency source or pin is always
deliberate and always wins:

```
[[override]]  >  path-source  >  git-source  >  ⟨registry layer⟩
```

The embedded registry enters the **⟨registry layer⟩**, and its position there
is the developer↔user inversion:

- **`origin = "external"` (developer) — embedded FIRST:**
  `embedded > declared [[registry]] walk`. On a coordinate clash, the embedded
  copy wins. This is the vibevm-on-vibevm case (§1.1).
- **distribution / end-user — embedded LAST:**
  `declared [[registry]] walk > embedded`. The bundle is a fall-through.

If the project declares **no** `[[registry]]` at all and an embedded registry is
available, resolution uses the embedded registry instead of failing — the
`build_install_resolver` "no registry configured" bail (PROP-002) is lifted when
an embedded registry is present.

### 3.1 The knob {#knob}

`--prefer-embedded` / `--no-prefer-embedded` selects the position explicitly.
Its **default follows the install origin**: on for `origin = "external"`, off for
a distribution. `--no-default-registry` (env `VIBE_NO_DEFAULT_REGISTRY=1`)
suppresses the embedded registry entirely for a command. An explicit
`--registry <path>` still shadows everything (PROP-002 M0 exclusivity),
unchanged.

### 3.2 Terminology — `embedded` vs `local` {#terminology}

**`embedded`** names packages that ship *inside vibevm itself* — the in-tree
`packages/` of a source build, or a distribution's bundle. The flag is
`--prefer-embedded`; the lock `source_kind` is `embedded` (§4).

**`--prefer-local` is reserved** and MUST NOT be used for this feature. It names
a *future, distinct* capability: a user — who is **not** a vibevm developer —
pointing vibe at *their own* local package repositories. Embedded packages
(vibevm's own) and a user's local repos are different things with different
precedence stories; conflating them under one flag would foreclose that future.

## 4. The lock {#lock}

A package resolved from the embedded registry records `source_kind = "embedded"`
in `vibe.lock` (a new [PROP-002](PROP-002-decentralized-registry.md) `SourceKind`
variant beside `registry` / `git` / `override` / `path`). Its `source_url` is the
`file://` path into `<source_path>/packages`.

`source_kind = "embedded"` is the marker the reproducibility guard keys on (§5):
it says "this entry resolved from a machine-local, source-install-derived
directory," which a different machine — a teammate, CI — cannot reproduce.

## 5. Reproducibility guard {#guard}

A `file://<source_path>/packages/...` entry is **machine-local**: a checkout on
another box, or CI, has no such path. Left unguarded, an embedded-resolved lock
committed to a shared repo breaks for everyone else. The guard, at the
**warn + CI-off** strength the owner chose:

- **CI-off.** In `--frozen` (and any non-interactive CI resolution), the embedded
  registry is **disabled by default** — CI must resolve from declared registries,
  so a machine-local lock cannot silently pass there.
- **Warn.** `vibe check` **warns** (does not fail) when the lock carries any
  `source_kind = "embedded"` entry: "this lockfile depends on the embedded
  registry of a source install and is not portable; publish or vendor these
  packages before sharing the lock."

This keeps the convenience strictly a **developer-machine** affordance and stops
a non-portable lock from leaking into a shared commit unnoticed.

## 6. Discoverability {#doctor}

The embedded registry is ambient but never silent:

- `vibe doctor` reports it: `embedded registry: <source_path>/packages (active
  install #<n>, origin external, precedence first)` — or `none` for a
  distribution / managed install.
- Resolution output names the source when a package resolves from it ("resolved
  `org.vibevm.world/wal` from the embedded registry").

## 7. Implementation hooks {#impl}

Grounded in the current tree:

- **Origin + source path:** `~/opt/vibevm/state.toml` already records each
  install's `origin` and `source_path`; the active record is `store.active()`
  (`crates/vibe-cli/src/commands/vvm/mod.rs`). Reuse that store to discover the
  embedded path.
- **Injection point:** `build_install_resolver`
  (`crates/vibe-cli/src/commands/install/resolver.rs`). After the explicit
  `--registry` branch, resolve the embedded registry (unless suppressed) and
  compose it with the declared `MultiRegistryResolver` at the origin-selected
  precedence; lift the empty-`[[registry]]` bail when embedded is present.
- **Lock:** add `SourceKind::Embedded`
  (`crates/vibe-core/src/manifest/lockfile.rs`); tag embedded-resolved entries.
- **Guard:** thread a `frozen` flag into resolver construction to drop the
  embedded registry in CI; add the `vibe check` warning on `Embedded` lock
  entries.
- The same composition serves `vibe install`, `update`, and `outdated` (they
  share `MultiRegistryResolver::open`).

## 8. Edge cases {#edges}

- **Source tree deleted / moved:** `<source_path>/packages` missing → the
  embedded registry is skipped with a `vibe doctor` warning; resolution falls
  back to declared registries.
- **Non-vibevm packages:** the embedded registry only answers for coordinates it
  actually contains; anything else falls through to declared registries as
  usual, regardless of precedence.
- **The host vibevm project itself:** with an embedded registry active, the host
  can drop `--registry packages/` from its own dev workflow — its own `packages/`
  becomes the embedded default (full self-dogfood).

## 9. Decisions {#decisions}

- **D1 — precedence is origin-selected, not fixed.** *Rejected:* one global
  precedence. It cannot serve both the developer (embedded must win, to test
  local edits) and the end user (bundle must not shadow published packages). The
  install `origin` already encodes which case applies.
- **D2 — `embedded`, not `local`, in name and `source_kind`.** *Rejected:*
  `--prefer-local` / `source_kind = "local"`. "Local" is reserved for a future
  user-owned-repository feature (§3.2); using it here would collide.
- **D3 — explicit sources stay above embedded.** *Rejected:* embedded above
  overrides / path / git. Those are deliberate per-dependency choices; an ambient
  default must not override a decision the author wrote down.
- **D4 — warn + CI-off, not hard-error.** *Rejected:* hard-error in CI on an
  embedded lock. The owner chose the lighter guard: CI simply does not use the
  embedded registry (so a machine-local lock fails to *resolve* there rather than
  being *rejected*), and `vibe check` warns the developer. Revisit if leaks
  recur.
- **D5 — ambient but visible.** *Rejected:* a purely silent default. `vibe
  doctor` and resolution output always surface the embedded registry, so the
  behaviour is discoverable, not magic.
