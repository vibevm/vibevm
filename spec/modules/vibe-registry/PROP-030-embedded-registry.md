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

**Enumeration reaches the network by default; two flags opt out.** Precedence
(above) governs which side *wins* a coordinate and which side a package is
*fetched* from — the fetch path is first-served (embedded-first stops at the
embedded copy). But **version enumeration** (the candidate set the solver picks
from) **unions across embedded *and* declared** by default, so the solver can
see a newer published version even for a package the embedded registry already
carries. That union is deliberate — it keeps a source developer from silently
pinning stale versions — but it means a declared network `[[registry]]` is
contacted (a `git ls-remote`) even when the embedded registry could answer
alone. Two opt-in flags trade that freshness check for zero network:

- **`--offline`** — resolve strictly offline: the declared network walk is not
  opened at all, so the embedded registry (plus explicit `--registry` / path /
  git sources) answers alone. No git host is contacted; a coordinate absent
  locally fails **without a single network request** (and thus without any
  credential prompt). With no embedded registry and no `--registry`, the command
  bails with an actionable message rather than silently resolving nothing.
- **`--embedded-short-circuit`** — keep the declared walk available, but
  short-circuit version enumeration at the embedded registry for any coordinate
  it serves: the network is reached **only** for packages the embedded registry
  lacks. A fully-embedded dependency graph resolves with zero network access
  (no enumeration round-trip, no credential prompt), while a genuinely missing
  package is still fetched from the network. Implies embedded-first precedence;
  mutually exclusive with `--no-prefer-embedded`.

Neither flag is the default: a bare `vibe install` still unions embedded with the
declared walk. Note the interaction with PROP-002 §2.2.1 — a public
(`auth = "none"`) registry now silences credential prompts unconditionally, so
even the default union path never raises a login dialog for a missing public
package; the two flags above additionally spare the network round-trip itself.

### 3.2 Terminology — `embedded` vs `local` {#terminology}

**`embedded`** names packages that ship *inside vibevm itself* — the in-tree
`packages/` of a source build, or a distribution's bundle. The flag is
`--prefer-embedded`; the lock `source_kind` is `embedded` (§4).

**`local`** names packages that ship *inside the current project* — the
in-tree `<project_root>/packages/` of any vibe project (§3.3). The flag is
`--prefer-local` / `--no-prefer-local`; the lock `source_kind` is `local`
(§4). Originally reserved (see §9 D2 historical text) for a broader
"user-own-repos" feature, the name now lands for the narrower
project-packages semantics first; arbitrary user-repos remain a future
expansion under a different name.

### 3.3 Project-local sources {#project-local}

REQ. A project carrying `<project_root>/packages/` (where `project_root` is
the directory holding the project's `vibe.toml`, resolved by
`resolve_project_root`) gets that directory auto-opened as a `LocalRegistry`
and composed into the local-registry family alongside the vibe-embedded
registry. No `[[registry]]` block, no `--registry <path>`, no
`~/.vibe/registry.toml` machine entry needed.

REQ. Project-local discovery is **independent of the running vibe's install
origin**. Unlike vibe-embedded (§2), it works from a `cargo run`, a test
binary, a distribution install, and a source install alike — every kind of
`vibe` invocation that targets a project with a `packages/` directory
discovers it. The feature exists for the downstream consumer's project, not
for the tool.

REQ. The local family is ordered **project-local first**, then
vibe-embedded, so a developer's own in-tree packages win a clash inside the
family (the project is the source of truth for its own deps). This ordering
is internal to the `LocalCompositeProvider`; against the declared
`[[registry]]` walk, the family as a whole composes at the existing
`EmbeddedPrecedence` (§3).

REQ. The CI-off gate (`CI` / `VIBE_NO_DEFAULT_REGISTRY`, §5) does **NOT**
suppress project-local — it is per-project and portable (every checkout
carries the same `packages/`), so a project-local lock is reproducible
across machines and CI. The gate continues to suppress the vibe-embedded
half (the machine-local one).

REQ. `--no-prefer-local` suppresses project-packages discovery for one
command (use when a project's `packages/` is stale, broken, or deliberately
bypassed). It does NOT suppress vibe-embedded — `--no-default-registry`
remains the knob for that. `--prefer-local` is the explicit affirmation of
the default (project-local wins the local family); mutually exclusive with
`--no-prefer-local`.

REQ. A package resolved from project-local records `source_kind = "local"`
in `vibe.lock` (§4) — distinct from `embedded`. Unlike `embedded`, it is
**portable** and the reproducibility guard (§5) does NOT warn on it: every
checkout of the project resolves the same `packages/` to the same content.

## 4. The lock {#lock}

A package resolved from the embedded registry records `source_kind = "embedded"`
in `vibe.lock` (a [PROP-002](PROP-002-decentralized-registry.md) `SourceKind`
variant beside `registry` / `git` / `override` / `path`). Its `source_url` is the
`file://` path into `<source_path>/packages`.

A package resolved from project-local (§3.3) records `source_kind = "local"`
(another `SourceKind` variant). Its `source_url` is the `file://` path into
`<project_root>/packages`.

`source_kind = "embedded"` is the marker the reproducibility guard keys on (§5):
it says "this entry resolved from a machine-local, source-install-derived
directory," which a different machine — a teammate, CI — cannot reproduce.
`source_kind = "local"` is portable (per-project, §3.3) and the guard does NOT
key on it.

## 5. Reproducibility guard {#guard}

A `file://<source_path>/packages/...` entry (the vibe-embedded registry, §2) is
**machine-local**: a checkout on another box, or CI, has no such path. Left
unguarded, an embedded-resolved lock committed to a shared repo breaks for
everyone else. The guard, at the **warn + CI-off** strength the owner chose:

- **CI-off.** In `--frozen` (and any non-interactive CI resolution), the
  vibe-embedded registry is **disabled by default** — CI must resolve from
  declared registries (and, since §3.3, project-local), so a machine-local lock
  cannot silently pass there. Project-local is NOT suppressed by this gate — it
  is per-project and portable.
- **Warn.** `vibe check` **warns** (does not fail) when the lock carries any
  `source_kind = "embedded"` entry: "this lockfile depends on the embedded
  registry of a source install and is not portable; publish or vendor these
  packages before sharing the lock." A `source_kind = "local"` entry is
  portable and does NOT warn.

This keeps the embedded-registry convenience strictly a **developer-machine**
affordance and stops a non-portable lock from leaking into a shared commit
unnoticed. Project-local (§3.3) is the *portable* counterpart — it has the same
convenience without the portability caveat.

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
  **D2 revised (§3.3 amendment):** the reserved `--prefer-local` /
  `source_kind = "local"` name now **lands for the narrower project-packages
  semantics** — a project's own in-tree `<project_root>/packages/`. The
  narrower scope (a single well-known directory per project, portable across
  checkouts) is distinct from the original "arbitrary user-owned-repos"
  concern, which remains a future expansion under a different name. The
  reservation was the right call to avoid colliding the two; the §3.3
  feature takes the name now that the narrow semantics is shipped and the
  broader one is still unscoped.
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
