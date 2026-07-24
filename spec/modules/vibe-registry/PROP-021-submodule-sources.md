# PROP-021 — Submodule sources {#root}

<status stage="spec" state="done" action="continue" comment="B0 2026-07-24: proposed 2026-06-24; one of the four bridge-packages specs"/>

**Status:** proposed 2026-06-24 — owner-requested design session. One of four
orthogonal specs from the bridge-packages design (siblings:
[PROP-020](../vibe-workspace/PROP-020-install-hooks.md) install hooks,
[PROP-022](../vibe-workspace/PROP-022-materialization-modes.md) materialization
modes, [PROP-023](PROP-023-bridge-packages.md) bridge packages). Submodules
serve any package that wants to embed another repository — not only bridges.
**Related:** [PROP-001](PROP-001-git-backend.md) (the git backend whose clone
gains `--recurse-submodules`), [PROP-002](PROP-002-decentralized-registry.md)
(one git repo = one package — a submodule is *not* a second package),
[PROP-010](PROP-010-local-package-cache.md) (the live-git cache the submodule
is fetched into), [PROP-022 §2](../vibe-workspace/PROP-022-materialization-modes.md)
(how submodule content reaches the slot differs by mode),
[PROP-003](../vibe-resolver/PROP-003-dep-evolution.md) (the dependency grammar
the future dependency-declared form would extend).

---

## 1. Motivation {#motivation}

### 1.1 The problem — embedded repos do not come along {#problem}

A package may want to carry another repository inside it. Git offers two ways:
a **submodule** (a reference — `.gitmodules` + a pinned gitlink commit) or a
**vendored copy** ("git in git" — the foreign tree committed into the package).
The vendored case already works: it is just files in the package tree. The
submodule case does **not** — vibevm's clone is a bare
`git clone --branch <ref>` ([PROP-001](PROP-001-git-backend.md)) with no
`--recurse-submodules`, and the `.git`-stripping materialise copies the empty
submodule stub. The referenced content silently never arrives.

The forcing case is bridge packages ([PROP-023](PROP-023-bridge-packages.md)),
where a maintainer submodules the upstream repo they steward. But the mechanism
is general: any package may legitimately embed a pinned dependency repo.

### 1.2 What this is — submodule as a first-class embedded source {#what}

vibevm fetches a package's submodules when it fetches the package, updates them
when it updates the package, and makes the submodule content available wherever
the package is materialised — embedded into the snapshot for the
copy-based modes, or living natively for `in-place`
([PROP-022](../vibe-workspace/PROP-022-materialization-modes.md)).

## 2. Decisions {#decisions}

### 2.1 Fetch and update recurse into submodules {#fetch}

`req r1`

The git backend's bootstrap and update recurse:

- **Bootstrap** — `git clone --recurse-submodules --branch <ref> -- <url>
  <dest>`. The clone lands in the live-git cache
  ([PROP-010](PROP-010-local-package-cache.md)) with submodule working trees
  populated.
- **Update** — after the existing `fetch --prune --tags` + `reset --hard
  <ref>`, run `git submodule update --init --recursive` so the gitlink commits
  the new superproject ref points at are checked out. (A removed submodule is
  pruned by the reset; a moved one re-inits.)

This applies identically to the registry cache clone and to an `in-place`
slot clone ([PROP-022 §2.4](../vibe-workspace/PROP-022-materialization-modes.md#in-place)).

### 2.2 Submodule is an abstract embedded source — git now, dependency later {#source-abstraction}

`req r1`

A submodule is modelled as an **embedded source**: content that lives at a
subpath of the package and is resolved from elsewhere. There are two
declaration forms:

- **git-native** (`.gitmodules`) — the only form **implemented now**. vibevm
  reads no `.gitmodules` itself; git does, via §2.1.
- **dependency-declared** (future) — a package's manifest names an embedded
  source in its dependency section, so a submodule can be expressed even for a
  package whose own distribution is not a live git checkout (e.g. a future
  binary package). This form is **specified as the extension point and
  stubbed**, not built (§4) — the abstraction exists so the git path is not a
  one-off.

Either way, the embedded repo is **not** a second vibevm package: it is git
content, never entered into the dependency resolver
([PROP-002](PROP-002-decentralized-registry.md): one git repo = one package;
the submodule is part of *this* package's content, not a node).

### 2.3 Snapshot materialisation embeds the submodule content {#snapshot-embedding}

`req r1`

How submodule content reaches the slot depends on the materialization mode
([PROP-022](../vibe-workspace/PROP-022-materialization-modes.md)):

- **`snapshot` / `hardlink`** — the submodule's checked-out working tree is
  copied into the slot as ordinary files; nested `.git` directories and gitlink
  pointers are stripped (the same exclusion the top-level `.git` already gets).
  The submodule content is thus **vendored into the snapshot** and participates
  in the package `content_hash`.
- **`in-place`** — nothing is copied; the submodule lives natively inside the
  slot's own git checkout, managed by git (§2.1).

### 2.4 The lockfile pins submodule state via the superproject commit {#lock}

`req r1`

Reproducibility rides on the package's `resolved_commit` already recorded in
the lockfile: a superproject commit fixes the exact gitlink commit of every
submodule, so a re-clone at `resolved_commit` with `--recurse-submodules`
reconstructs byte-identical submodule content. No new lockfile field is
required for the git-native form. (Explicit per-submodule pins are a possible
future refinement, tied to the dependency-declared form of §2.2.)

## 3. Rejected alternatives {#rejected}

- **Resolving a submodule as a vibevm package** through the depsolver —
  rejected: it is git content under one package, not a registry node; treating
  it as a node would double-count identity and break PROP-002's one-repo /
  one-package rule.
- **Shallow submodule clones (`--depth 1`) by default** — rejected as the
  default: a shallow submodule can miss the exact gitlink commit and fail
  checkout; depth control is a possible opt-in, not the baseline.
- **vibevm parsing `.gitmodules` itself** — rejected: git already resolves
  submodules correctly across auth and nesting; reimplementing it would be a
  fragile re-do of solved work.

## 4. Out of scope {#out-of-scope}

- **The dependency-declared submodule form** (§2.2) — specified as the
  extension point, not implemented. It waits on a real consumer (binary /
  non-git packages), per the owner's "spec the abstraction, build only the git
  path" direction.
- **Submodules under non-git package sources** — a path- or future
  binary-source package has no git context to recurse; embedded sources there
  await the dependency-declared form.
- **Recursive vibevm resolution inside a submodule** — a submodule's own
  `vibe.toml`, if any, is not honoured; the submodule is opaque content.

## 5. Acceptance {#acceptance}

- A package whose repo declares a submodule is cloned with its submodule
  working tree populated; `update` re-checks-out submodule content for the new
  superproject ref.
- Under `snapshot`/`hardlink`, submodule content appears in the slot as plain
  files with no nested `.git`; it contributes to `content_hash`.
- Under `in-place`, the submodule lives natively in the slot's git checkout.
- Re-cloning at the lockfile's `resolved_commit` reconstructs identical
  submodule content with no extra lockfile field.
- A vendored ("git in git") package needs none of this — it is plain files and
  installs unchanged.
- Full `self-check.sh` green; conform 0/0/0; specmap clean.
