# PROP-020 — Install hooks {#root}

**Status:** proposed 2026-06-24 — owner-requested design session. One of four
orthogonal specs carved from the bridge-packages design (the others:
[PROP-021](../vibe-registry/PROP-021-submodule-sources.md) submodule sources,
[PROP-022](PROP-022-materialization-modes.md) materialization modes,
[PROP-023](../vibe-registry/PROP-023-bridge-packages.md) bridge packages). The
four compose to solve bridge packages but each stands alone — hooks exist for
*any* package, not only bridges.
**Related:** [PROP-009](PROP-009-loading-model.md) (the install/materialise
pipeline hooks slot into), [PROP-007](PROP-007-workspace.md) (workspace +
`vibedeps/`), [PROP-022](PROP-022-materialization-modes.md) (a hook's
working tree is the materialised slot; how its edits are reset on update is a
materialization-mode property), [PROP-015 §2.6](../vibe-mcp/PROP-015-mcp-integration.md#skill)
(skill projection reads the slot a hook prepared), [PROP-019 §2.13](../../common/PROP-019-version-manager.md#security)
(the same "a build/script the user installs is code they chose to run" trust
posture), [PROP-000 §20](../../common/PROP-000.md#token-secrecy) (the publish
token a hook never sees).

---

## 1. Motivation {#motivation}

### 1.1 The problem — install is pure file I/O, with no preparation step {#problem}

Today `vibe install` resolves, fetches, materialises a package's tree into
`vibedeps/`, regenerates boot artefacts, and writes the lockfile — all pure
file copying. A package that needs a *preparation* step after its content
lands (normalise a vendored layout, generate a derived file, assemble a clean
skill subtree out of an upstream repo's mess) has nowhere to put it.

The forcing case is bridge packages ([PROP-023](../vibe-registry/PROP-023-bridge-packages.md)):
a maintainer wraps someone else's repository whose structure does not match
vibevm conventions, and needs to *bring it into order* before vibevm's skill
machinery reads it. But the need is general — any package may want a
post-materialise step — so hooks are a **universal** mechanism, not a
bridge-only feature.

### 1.2 What this is — declared lifecycle scripts, run per package {#what}

A package may declare `pre-install` / `post-install` scripts in its manifest.
vibevm runs them at fixed points in the install pipeline, in the package's own
materialised slot, choosing the right interpreter for the host OS. Their
effects are **ephemeral**: re-installing or updating the package resets the
slot first (per [PROP-022](PROP-022-materialization-modes.md)), then re-runs
the hooks, so a hook is a pure function of the package content, never an
accreting pile of edits.

## 2. Decisions {#decisions}

### 2.1 Two phases, anchored to the materialise pipeline {#phases}

`req r1`

A package declares at most one script per phase:

- **`pre-install`** — runs immediately after the package's slot is fully
  populated (content materialised, submodules fetched per
  [PROP-021](../vibe-registry/PROP-021-submodule-sources.md)) and **before**
  vibevm uses the slot (before boot regeneration, before any later
  `vibe skill` projection reads it). This is the "bring the tree into order"
  hook.
- **`post-install`** — runs after the install run is durable for that package
  (lockfile written, boot artefacts regenerated). For finalisation that needs
  the package already registered.

The hook's **working directory is the package's materialised slot**; it sees
exactly the tree vibevm will use.

**On update / reinstall, hook effects are reset, then hooks re-run.** The slot
is first returned to its pristine materialised state — for `snapshot`/
`hardlink` modes by re-materialising from cache, for `in-place` mode by
`git clean -dfx` ([PROP-022 §2.4](PROP-022-materialization-modes.md#in-place))
— so a previous run's edits never compound. This reset is a
materialization-mode property; hooks only define *when* the re-run happens.

### 2.2 Interpreter selection is OS-derived {#script-selection}

`req r1`

A package ships a phase script as `<base>.sh` (portable, POSIX shell) and/or
`<base>.ps1` (PowerShell). The runner picks per host:

- **Unix (macOS / Linux):** run `<base>.sh` via `bash`. A `.ps1` is ignored.
- **Windows:** prefer `<base>.sh` via **Git Bash** when a `bash` is found
  (one cross-platform script for `.sh` packages); else fall back to
  `<base>.ps1` via **PowerShell** when one is found. A phase that declares a
  script but finds no usable interpreter is a hard error with a remediation
  hint — never a silent skip.

The runner passes a documented environment: `VIBE_PACKAGE_GROUP`,
`VIBE_PACKAGE_NAME`, `VIBE_PACKAGE_VERSION`, `VIBE_PACKAGE_KIND`,
`VIBE_PACKAGE_DIR` (the slot, also CWD), `VIBE_HOOK_PHASE`.
([PROP-024 §2.3](../../common/PROP-024-code-bearing-packages.md#build) adds
`VIBE_PROJECT_ROOT`, the workspace absolute root, so a build hook can target a
gitignored build dir *outside* the slot; it lands with that work.) The publish token
([PROP-000 §20](../../common/PROP-000.md#token-secrecy)) is **never** placed in
a hook's environment.

The process runner is an injectable seam (`HookRunner`) so tests assert the
selection logic and argument/env shape without spawning real processes.

### 2.3 Trust gate — allow-list of groups, plus first-run consent {#trust-gate}

`req r1`

Running a package's hook is running third-party code at install time. Until a
content-scanning gate exists (§4), trust is governed cheaply:

- **Allow-listed groups run silently.** A config key (global
  `~/.vibe/config.toml` `[hooks].allowed_groups`, with a project-level
  override) lists trusted package groups. **`org.vibevm` is in the allow-list
  by default.** A package whose group is allow-listed runs its hooks with no
  prompt.
- **Other groups need consent.** On the first hook run of a non-allow-listed
  package, vibevm prints what will run (phase, script path, group) and asks
  `y/n`. Declining skips the hook and marks the package install as
  hooks-skipped (surfaced, not silent).
- **Non-interactive safety.** With `--assume-yes` / in CI, allow-listed
  packages still run; a non-allow-listed package's hook is **not** run
  silently — the install **aborts** with a hint to either allow-list the group
  or pass an explicit `--allow-hooks` opt-in. A script must never execute
  unseen third-party code by default.

### 2.4 Hooks are declared in the manifest {#manifest}

`req r1`

Hooks live in a package-role `[hooks]` table in `vibe.toml`:

```toml
[hooks]
pre-install  = "hooks/prepare"   # base path, relative to package root
post-install = "hooks/finalise"
```

The value is a **base path without extension**; the runner resolves `.sh` /
`.ps1` beside it per §2.2. The table is package-only (its presence on a
`[project]`-role manifest is a validation error, like the other package-only
sections). An empty/absent `[hooks]` means no hooks — the common case.

### 2.5 Failure semantics are phase-specific {#failure}

`req r1`

A hook's stdout/stderr stream to the user. A non-zero exit is handled by
phase:

- **`pre-install` failure → the package install aborts.** The slot is rolled
  back (removed) and the install reports the failing package; vibevm never
  registers or projects from a package whose preparation failed.
- **`post-install` failure → the package is installed but flagged.** The
  package is already durable (lockfile written); the failure surfaces as a
  warning with the captured output, never a silent success.

## 3. Rejected alternatives {#rejected}

- **Inline `command = "..."` strings in the manifest** instead of files —
  rejected: a versioned script file is auditable, diffable, and platform-split
  (`.sh`/`.ps1`); an inline string hides the code in TOML and resists review.
- **Running every matching extension on Windows** (`.sh` *and* `.ps1`) —
  rejected: one logical hook per phase keeps behaviour predictable; the
  selection is a single deterministic choice (§2.2).
- **A general lifecycle (`pre-uninstall`, `pre-build`, …)** now — deferred:
  only the two phases the bridge case needs are specified; more can be added
  later behind their own anchors without disturbing these.

## 4. Out of scope {#out-of-scope}

- **Content scanning / the LLM "antivirus".** A future gate that inspects a
  package's hooks (and code) for malicious behaviour is far-backlog. Until it
  lands, hook execution is an **explicitly accepted risk** governed only by
  §2.3's allow-list + consent. This is the deliberate posture, not an
  oversight — recorded here as the project's stance.
- **Sandboxing / capability-limiting** hooks (containers, seccomp). Hooks run
  with the user's privileges, like `cargo build` scripts or `npm postinstall`.
- **Non-git slot reset for `in-place`** — a hook over an `in-place`
  ([PROP-022 §2.4](PROP-022-materialization-modes.md#in-place)) package whose
  source is not git has no cheap reset; `in-place` therefore requires a git
  source (PROP-022 §4).

## 5. Acceptance {#acceptance}

- A package with `[hooks].pre-install` runs the script in its slot before boot
  regeneration; `post-install` runs after the lockfile is written.
- Interpreter selection follows §2.2 on each OS; a declared hook with no usable
  interpreter errors rather than silently skipping.
- An allow-listed group (incl. `org.vibevm` by default) runs hooks with no
  prompt; a non-allow-listed package prompts interactively and **aborts** in a
  non-interactive run without `--allow-hooks`.
- A `pre-install` non-zero exit rolls back the slot and fails the package; a
  `post-install` failure installs-but-flags.
- Updating a package resets the prior hook's edits (per the materialization
  mode) before re-running the hooks.
- The `HookRunner` seam lets tests assert selection/env/failure paths without
  spawning real processes.
- Full `self-check.sh` green; conform 0/0/0; specmap clean.
