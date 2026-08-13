# PROP-022 — Materialization modes {#root}

<status stage="impl" state="done" comment="C 2026-07-25: the Materialization system ships (enum + doctests + install machinery + destructive guard); motivation, rejected and out-of-scope facts stay spec-stage; fact grain 2026-07-24"/>

@fact:status-line **Status: IMPLEMENTED** (specified 2026-06-24 in an owner-requested design
session; verified against the tree 2026-07-25 by the spec-actualization
campaign). The `Materialization` enum ships with `Snapshot` as the default,
`InPlace` beside it and a doctested `is_in_place` (`vibe-core` `package.rs`); the
hardlink / in-place machinery runs through `vibe-install` (`plan.rs` /
`fetched.rs` / `apply.rs`, the `materialise_in_place` seam); submodule
snapshot-embedding lands in `git_package_registry/fetch.rs`; and the destructive
guard sits in `commands/uninstall.rs`. One of four orthogonal specs from the
bridge-packages design (siblings:
[PROP-020](PROP-020-install-hooks.md) install hooks,
[PROP-021](../vibe-registry/PROP-021-submodule-sources.md) submodule sources,
[PROP-023](../vibe-registry/PROP-023-bridge-packages.md) bridge packages).
Materialization mode is a property of *any* package; a huge git package wants
`in-place` with no bridge in sight. @status:impl/done

@fact:related **Related:** [PROP-009](PROP-009-loading-model.md) (the materialise step into
`vibedeps/`), [PROP-007](PROP-007-workspace.md) (`vibedeps/` layout),
[PROP-010](../vibe-registry/PROP-010-local-package-cache.md) (the live-git
cache + `.git`-stripped snapshot the copy modes draw from),
[PROP-019 §2.15](../../common/PROP-019-version-manager.md#instances) (the VVM
`placer` diff-copy/hardlink — direct prior art for `hardlink`),
[PROP-019 §2.16](../../common/PROP-019-version-manager.md#provenance) (VVM
"sources by reference" — prior art for `in-place`),
[PROP-020 §2.1](PROP-020-install-hooks.md#phases) (hook-edit reset rides on the
mode). @status:spec/done

---

## 1. Motivation {#motivation}

### 1.1 The problem — one materialisation policy does not fit every package {#problem}

- @fact:today-single-policy Today every package is materialised the same way: clone into the live-git
  cache, strip `.git` into a snapshot, then **full recursive copy** of that tree
  into the `vibedeps/<group>.<name>/<version>/` slot (identity-keyed — owner ruling 2026-08-13; the slot carried `<kind>-<name>` before that ruling, which collided same-named packages of different groups and moved a package on a kind change). @status:impl/done
- @fact:snapshot-right-for-ordinary This is right for ordinary
  packages and gives the lockfile a stable `content_hash` and a committable,
  offline-reproducible vendored slot. @status:impl/done

@fact:two-scales-lead It fails at two different scales of "big": @status:spec/done

1. @fact:BIG-IN-BYTES **Big in bytes** — a package with a few large binary assets pays a full byte
   copy per install/update of data that did not change. @status:spec/done
2. @fact:BIG-IN-FILE-COUNT **Big in file count** — a package with *millions* of small files (think the
   Chromium source tree) is killed not by bytes but by **per-file syscalls**:
   copying — or even hardlinking — five million files takes hours, barely
   faster than re-fetching over the network. The full-tree walk (`content_hash`
   reads every file too) is itself the cost. @status:spec/done

@fact:FIX-DECLARED-MODE The fix is to make materialisation a **declared mode** on the package, and to
borrow the two cost-avoidance primitives VVM already proved
([PROP-019 §2.15/§2.16](../../common/PROP-019-version-manager.md#instances)). @status:impl/done

## 2. Decisions {#decisions}

### 2.1 Three modes, declared in the descriptor {#modes}

@fact:req-modes `req r1` @status:impl/done

@fact:MODE-FIELD `[package].materialization` selects how the package lands on disk: @status:impl/done

```toml
[package]
materialization = "snapshot"   # default | "hardlink" | "in-place"
```

- @fact:SNAPSHOT-DEFAULT `snapshot` is the default and the only mode an ordinary package needs. @status:impl/done
- @fact:MODE-IN-DESCRIPTOR The
  mode is published in the descriptor so a consumer sees, before installing, how
  a package will be placed. @status:impl/done

### 2.2 `snapshot` — the vendored full copy (default) {#snapshot}

@fact:req-snapshot `req r1` @status:impl/done

- @fact:SNAPSHOT-PIPELINE The status quo: live-git cache → `.git`-stripped snapshot → full recursive
  copy into the slot. @status:impl/done
- @fact:STRIP-EXTENDED [PROP-024 §2.2](../../common/PROP-024-code-bearing-packages.md#shippable-tree)
  extends the `.git` strip to build output (`.vibe/`, `target/`, `node_modules/`,
  `.vibeignore` globs) for code-bearing packages, so such a package vendors its
  source, never its build artifacts. @status:impl/done
- @fact:SLOT-SELF-CONTAINED The slot is a self-contained tree, identified by
  `content_hash` (§2.5), vendored into the project's git (§2.7). @status:impl/done
- @fact:SUBMODULE-EMBEDDED Submodule
  content is embedded into the snapshot
  ([PROP-021 §2.3](../vibe-registry/PROP-021-submodule-sources.md#snapshot-embedding)). @status:impl/done
- @fact:HOOK-RESET-REMATERIALISE A hook's edits are reset on update by re-materialising the slot from cache
  (network-free). @status:impl/done

### 2.3 `hardlink` — per-file hardlink, copy on change {#hardlink}

@fact:req-hardlink `req r1` @status:impl/done

- @fact:HARDLINK-MODE For packages **big in bytes but modest in file count**. Instead of copying
  file bytes, materialise hardlinks each file from the cached snapshot into the
  slot; on update, only changed files are re-linked, the rest are left — the VVM
  `placer` algorithm ([PROP-019 §2.15](../../common/PROP-019-version-manager.md#instances)):
  a per-file manifest of `(rel, size, mtime, hash-for-small-files)`, large files
  compared by `(size, mtime)` only (never read). @status:impl/done
- @fact:HARDLINK-FALLBACK A hardlink that fails
  (cross-volume / unsupported filesystem) falls back to copy. @status:impl/done
- @fact:HARDLINK-CONTRACT The slot still presents a **full tree** (the contract is unchanged from
  `snapshot`); identity stays `content_hash`, computed cheaply via the manifest
  for large files. @status:impl/done
- @fact:HARDLINK-NOT-GIANT This mode does **not** help the file-count case — the per-file
  syscall remains — so it is not the giant-repo answer (§2.4 is). @status:impl/done

### 2.4 `in-place` — git-native, project-local, no copy {#in-place}

@fact:req-in-place `req r1` @status:impl/done

@fact:IN-PLACE-LEAD For packages **big in file count** (and incidentally bytes), where even one
full tree walk is unacceptable. vibevm never walks the tree: @status:impl/done

- @fact:IP-CLONE-DIRECT **`git clone --recurse-submodules` lands directly in the slot**, bypassing
  both the cache clone and the snapshot copy — **one** physical copy on the
  machine, not three (decisive when disk cannot hold several copies of a giant). @status:impl/done
- @fact:IP-GIT-MANAGES **git manages it in place**: update is `git fetch` + checkout (incremental,
  touches only changed objects/files); a hook's edits are reset with
  `git clean -dfx` in the slot. @status:impl/done
- @fact:IP-PROJECT-LOCAL **Project-local, never shared.** Each project gets its own clone in its own
  `vibedeps/`; there is deliberately no cross-project sharing, which removes
  the concurrent-mutation problem a shared global clone would create. @status:impl/done
- @fact:IP-UNVERSIONED-PATH **The slot path is not version-qualified.** An `in-place` slot is
  `vibedeps/<group>.<name>/` (no `/<version>/`): one working clone whose version
  is the current git ref. Versioning the path would mean two on-disk copies of
  the giant — the opposite of the goal. @status:impl/done
- @fact:IP-REQUIRES-GIT **Requires a git source.** Incremental update and `git clean` reset both need
  git; a non-git source has no `in-place` story (§4). @status:impl/done

### 2.5 Identity follows the mode {#identity}

@fact:req-identity `req r1` @status:impl/done

- @fact:ID-COPY-MODES **`snapshot` / `hardlink`** — `content_hash` over the slot tree (the existing
  identity), `hardlink` computing it cheaply via the diff manifest. @status:impl/done
- @fact:ID-IN-PLACE **`in-place`** — **`resolved_commit`**, not `content_hash`. The slot is a
  mutable git working tree (hooks edit it), so a content hash is neither stable
  nor affordable to compute; the git commit *is* the identity, known in O(1).
  The lockfile already records `resolved_commit`, so no new field is needed. @status:impl/done

### 2.6 Destructive operations on an `in-place` slot need confirmation {#destructive-guard}

@fact:req-destructive `req r1` @status:impl/done

- @fact:DESTRUCTIVE-CONFIRM An `in-place` slot may be a multi-hour download. Any **destructive** operation
  on it — `uninstall`, `reinstall --force`, a version switch that requires a
  re-clone, or slot removal — must be confirmed: interactively a `y/n`, and in a
  non-interactive run it requires an explicit flag (`--force`) or it **aborts**
  rather than silently deleting an expensive resource. @status:impl/done
- @fact:HOOKS-EXEMPT **Hooks and their reset
  (`git clean -dfx`) are exempt** — they are routine and trusted (the hook author
  is assumed competent, [PROP-020](PROP-020-install-hooks.md)). @status:impl/done
- @fact:GUARD-PURPOSE The guard
  protects against accidental loss, not against the package's own lifecycle. @status:impl/done

### 2.7 Vendoring differs by mode {#vendoring}

@fact:req-vendoring `req r1` @status:impl/done

- @fact:VENDORED-COPY-MODES **`snapshot` / `hardlink`** are vendored — the slot is committed into the
  project's git and is offline-reproducible from it (a `hardlink` slot's bytes
  are materialised into git on `git add` like any file). @status:impl/done
- @fact:IN-PLACE-NOT-VENDORED **`in-place`** is **not** vendored — the slot (a nested `.git` plus possibly
  millions of files) is `.gitignore`d in the project; restoration is a re-clone
  at the lockfile's `resolved_commit`. The honest trade: `in-place` packages
  need the network to restore, where `snapshot` packages do not. @status:impl/done

## 3. Rejected alternatives {#rejected}

- @fact:REJ-HASH-IN-PLACE **Content-hashing an `in-place` package** — rejected for the same reason VVM
  rejected hashing distributions ([PROP-019 §9.2](../../common/PROP-019-version-manager.md#rationale-no-hash)):
  hashing millions of files / gigabytes per operation is prohibitive, and the
  tree is mutable anyway. Identity is the commit (§2.5). @status:spec/done
- @fact:REJ-SHARED-GLOBAL-CLONE **A shared global `in-place` clone** (like the registry cache) — rejected:
  two projects mutating one giant working tree via hooks would collide;
  project-local clones make the problem disappear (§2.4). @status:spec/done
- @fact:REJ-HARDLINK-FOR-GIANTS **Hardlink as the giant-repo answer** — rejected: per-file syscalls still
  cost hours at millions of files; only `in-place` (no per-file work) solves
  the file-count axis (§2.4). @status:spec/done
- @fact:REJ-REFLINK **reflink / CoW placement** — deferred (§4): not portable; hardlink is the
  portable byte-sharing primitive, matching VVM's choice. @status:spec/done

## 4. Out of scope {#out-of-scope}

- @fact:OOS-REFLINK **reflink / CoW** placement where the filesystem supports it — far-backlog,
  as in [PROP-019 §6](../../common/PROP-019-version-manager.md#far-backlog). @status:spec/done
- @fact:OOS-NON-GIT-IN-PLACE **`in-place` for a non-git source** — needs git for incremental update and
  `git clean` reset; a binary/path source has no `in-place` mode. @status:spec/done
- @fact:OOS-CACHE-GC **Automatic cache garbage collection** for the live-git cache — owned by
  [PROP-010](../vibe-registry/PROP-010-local-package-cache.md). @status:spec/done

## 5. Acceptance {#acceptance}

- @fact:ACC-FIELD-PARSES `[package].materialization` parses to `snapshot` (default) / `hardlink` /
  `in-place`; an unknown value is a manifest error. @status:impl/done
- @fact:ACC-MODE-BEHAVIORS `snapshot` behaves exactly as today; `hardlink` shares unchanged files by
  link with copy fallback and presents a full tree; `in-place` clones once into
  an unversioned, `.gitignore`d slot managed by git. @status:impl/done
- @fact:ACC-IN-PLACE-IDENTITY `in-place` identity is `resolved_commit`; no full-tree hash is computed. @status:impl/done
- @fact:ACC-DESTRUCTIVE-GUARD A destructive op on an `in-place` slot confirms interactively / requires
  `--force` non-interactively; hooks and `git clean` are exempt. @status:impl/done
- @fact:ACC-FLOOR-GREEN Full `self-check.sh` green; conform 0/0/0; specmap clean. @status:impl/done
