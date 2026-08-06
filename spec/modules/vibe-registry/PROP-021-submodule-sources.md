# PROP-021 — Submodule sources {#root}

<status stage="impl" state="done" comment="C 2026-07-25: submodule sources ship (recurse clone/update + snapshot embedding + in-place native); motivation, rejected and out-of-scope facts stay spec-stage; fact grain 2026-07-24"/>

@fact:status-line **Status: IMPLEMENTED** (specified 2026-06-24 in an owner-requested design
session; verified against the tree 2026-07-25 by the spec-actualization
campaign). The git backend clones with `--recurse-submodules` and runs
`submodule update --init --recursive`, snapshot embedding lands in
`git_package_registry/fetch.rs`, the in-place native form rides the PROP-022
machinery, and `resolved_commit` carries lockfile reproducibility. One of four
orthogonal specs from the bridge-packages design (siblings:
[PROP-020](../vibe-workspace/PROP-020-install-hooks.md) install hooks,
[PROP-022](../vibe-workspace/PROP-022-materialization-modes.md) materialization
modes, [PROP-023](PROP-023-bridge-packages.md) bridge packages). Submodules
serve any package that wants to embed another repository — not only bridges. @status:impl/done

@fact:related **Related:** [PROP-001](PROP-001-git-backend.md) (the git backend whose clone
gains `--recurse-submodules`), [PROP-002](PROP-002-decentralized-registry.md)
(one git repo = one package — a submodule is *not* a second package),
[PROP-010](PROP-010-local-package-cache.md) (the live-git cache the submodule
is fetched into), [PROP-022 §2](../vibe-workspace/PROP-022-materialization-modes.md)
(how submodule content reaches the slot differs by mode),
[PROP-003](../vibe-resolver/PROP-003-dep-evolution.md) (the dependency grammar
the future dependency-declared form would extend). @status:spec/done

---

## 1. Motivation {#motivation}

### 1.1 The problem — embedded repos do not come along {#problem}

- @fact:two-embed-ways A package may want to carry another repository inside it. Git offers two ways:
  a **submodule** (a reference — `.gitmodules` + a pinned gitlink commit) or a
  **vendored copy** ("git in git" — the foreign tree committed into the package). @status:spec/done
- @fact:vendored-works The vendored case already works: it is just files in the package tree. @status:spec/done
- @fact:submodule-broken The submodule case does **not** — vibevm's clone is a bare
  `git clone --branch <ref>` ([PROP-001](PROP-001-git-backend.md)) with no
  `--recurse-submodules`, and the `.git`-stripping materialise copies the empty
  submodule stub. The referenced content silently never arrives. @status:impl/done

- @fact:forcing-case-bridges The forcing case is bridge packages ([PROP-023](PROP-023-bridge-packages.md)),
  where a maintainer submodules the upstream repo they steward. @status:spec/done
- @fact:mechanism-general But the mechanism
  is general: any package may legitimately embed a pinned dependency repo. @status:spec/done

### 1.2 What this is — submodule as a first-class embedded source {#what}

@fact:WHAT-VVM-DOES vibevm fetches a package's submodules when it fetches the package, updates them
when it updates the package, and makes the submodule content available wherever
the package is materialised — embedded into the snapshot for the
copy-based modes, or living natively for `in-place`
([PROP-022](../vibe-workspace/PROP-022-materialization-modes.md)). @status:impl/done

## 2. Decisions {#decisions}

### 2.1 Fetch and update recurse into submodules {#fetch}

@fact:req-fetch `req r1` @status:impl/done

@fact:RECURSE-LEAD The git backend's bootstrap and update recurse: @status:impl/done

- @fact:BOOTSTRAP-RECURSE **Bootstrap** — `git clone --recurse-submodules --branch <ref> -- <url>
  <dest>`. The clone lands in the live-git cache
  ([PROP-010](PROP-010-local-package-cache.md)) with submodule working trees
  populated. @status:impl/done
- @fact:UPDATE-RECURSE **Update** — after the existing `fetch --prune --tags` + `reset --hard
  <ref>`, run `git submodule update --init --recursive` so the gitlink commits
  the new superproject ref points at are checked out. (A removed submodule is
  pruned by the reset; a moved one re-inits.) @status:impl/done

@fact:APPLIES-BOTH-CLONES This applies identically to the registry cache clone and to an `in-place`
slot clone ([PROP-022 §2.4](../vibe-workspace/PROP-022-materialization-modes.md#in-place)). @status:impl/done

### 2.2 Submodule is an abstract embedded source — git now, dependency later {#source-abstraction}

@fact:req-source-abstraction `req r1` @status:impl/done

@fact:EMBEDDED-SOURCE-MODEL A submodule is modelled as an **embedded source**: content that lives at a
subpath of the package and is resolved from elsewhere. There are two
declaration forms: @status:impl/done

- @fact:FORM-GIT-NATIVE **git-native** (`.gitmodules`) — the only form **implemented now**. vibevm
  reads no `.gitmodules` itself; git does, via §2.1. @status:impl/done
- @fact:FORM-DEPENDENCY-DECLARED **dependency-declared** (future) — a package's manifest names an embedded
  source in its dependency section, so a submodule can be expressed even for a
  package whose own distribution is not a live git checkout (e.g. a future
  binary package). This form is **specified as the extension point and
  stubbed**, not built (§4) — the abstraction exists so the git path is not a
  one-off. @status:spec/done

@fact:NOT-A-PACKAGE Either way, the embedded repo is **not** a second vibevm package: it is git
content, never entered into the dependency resolver
([PROP-002](PROP-002-decentralized-registry.md): one git repo = one package;
the submodule is part of *this* package's content, not a node). @status:impl/done

### 2.3 Snapshot materialisation embeds the submodule content {#snapshot-embedding}

@fact:req-snapshot-embedding `req r1` @status:impl/done

@fact:MODE-DEPENDENT How submodule content reaches the slot depends on the materialization mode
([PROP-022](../vibe-workspace/PROP-022-materialization-modes.md)): @status:impl/done

- @fact:EMBED-SNAPSHOT **`snapshot` / `hardlink`** — the submodule's checked-out working tree is
  copied into the slot as ordinary files; nested `.git` directories and gitlink
  pointers are stripped (the same exclusion the top-level `.git` already gets).
  The submodule content is thus **vendored into the snapshot** and participates
  in the package `content_hash`. @status:impl/done
- @fact:EMBED-IN-PLACE **`in-place`** — nothing is copied; the submodule lives natively inside the
  slot's own git checkout, managed by git (§2.1). @status:impl/done

### 2.4 The lockfile pins submodule state via the superproject commit {#lock}

@fact:req-lock `req r1` @status:impl/done

- @fact:LOCK-VIA-SUPERPROJECT Reproducibility rides on the package's `resolved_commit` already recorded in
  the lockfile: a superproject commit fixes the exact gitlink commit of every
  submodule, so a re-clone at `resolved_commit` with `--recurse-submodules`
  reconstructs byte-identical submodule content. No new lockfile field is
  required for the git-native form. @status:impl/done
- @fact:future-per-sub-pins Explicit per-submodule pins are a possible
  future refinement, tied to the dependency-declared form of §2.2. @status:spec/done

## 3. Rejected alternatives {#rejected}

- @fact:REJ-RESOLVE-AS-PACKAGE **Resolving a submodule as a vibevm package** through the depsolver —
  rejected: it is git content under one package, not a registry node; treating
  it as a node would double-count identity and break PROP-002's one-repo /
  one-package rule. @status:spec/done
- @fact:REJ-SHALLOW-DEFAULT **Shallow submodule clones (`--depth 1`) by default** — rejected as the
  default: a shallow submodule can miss the exact gitlink commit and fail
  checkout; depth control is a possible opt-in, not the baseline. @status:spec/done
- @fact:REJ-PARSE-GITMODULES **vibevm parsing `.gitmodules` itself** — rejected: git already resolves
  submodules correctly across auth and nesting; reimplementing it would be a
  fragile re-do of solved work. @status:spec/done

## 4. Out of scope {#out-of-scope}

- @fact:OOS-DEPENDENCY-FORM **The dependency-declared submodule form** (§2.2) — specified as the
  extension point, not implemented. It waits on a real consumer (binary /
  non-git packages), per the owner's "spec the abstraction, build only the git
  path" direction. @status:spec/done
- @fact:OOS-NON-GIT-SOURCES **Submodules under non-git package sources** — a path- or future
  binary-source package has no git context to recurse; embedded sources there
  await the dependency-declared form. @status:spec/done
- @fact:OOS-RECURSIVE-RESOLUTION **Recursive vibevm resolution inside a submodule** — a submodule's own
  `vibe.toml`, if any, is not honoured; the submodule is opaque content. @status:spec/done

## 5. Acceptance {#acceptance}

- @fact:ACC-CLONE-POPULATED A package whose repo declares a submodule is cloned with its submodule
  working tree populated; `update` re-checks-out submodule content for the new
  superproject ref. @status:impl/done
- @fact:ACC-SNAPSHOT-FILES Under `snapshot`/`hardlink`, submodule content appears in the slot as plain
  files with no nested `.git`; it contributes to `content_hash`. @status:impl/done
- @fact:ACC-IN-PLACE-NATIVE Under `in-place`, the submodule lives natively in the slot's git checkout. @status:impl/done
- @fact:ACC-LOCK-REPRODUCES Re-cloning at the lockfile's `resolved_commit` reconstructs identical
  submodule content with no extra lockfile field. @status:impl/done
- @fact:ACC-VENDORED-UNCHANGED A vendored ("git in git") package needs none of this — it is plain files and
  installs unchanged. @status:impl/done
- @fact:ACC-FLOOR-GREEN Full `self-check.sh` green; conform 0/0/0; specmap clean. @status:impl/done
