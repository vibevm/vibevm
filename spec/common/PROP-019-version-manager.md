# PROP-019 — VibeVM Version Manager (VVM) {#root}

**Status:** proposed 2026-06-17 — owner-requested design session; **revised
to v2 the same day** after the owner found two architectural flaws in v1
(see §9): (a) making `$VIBEVM_HOME` the single source of truth forced a
console reload on every switch/reinstall, and (b) replacing the running
distribution locks its files (the `.exe`, and any future DLLs). v2 keeps the
v1 command surface but reworks the internals: a live `current` pointer file
+ `current_exe()` ground truth (env demoted to advisory), the *whole
distribution directory* as the immutable unit of install/switch, content-
cheap diff-copy between instances, sources held *by reference* (never
copied), and a new `vibe vars` reconciliation command. §9 records every
decision and the questions explored to reach them.
**Related:** [PROP-018](PROP-018-agentic-standalone-modes.md) (VVM is a
second *standalone-mode* citizen after `vibe skill` — pure algorithm, no
LLM; §2.1), [PROP-016](PROP-016-source-mirrors.md) (the source mirrors VVM
clones from when run outside a source tree), [PROP-000 §7](PROP-000.md#registry)
and [PROP-000 §20](PROP-000.md#token-secrecy) (the publish token VVM never
touches), [`VIBEVM-SPEC.md`](../../VIBEVM-SPEC.md) (CLI-first posture), and
the repo's `rust-toolchain.toml` (the pin VVM honours when building).

---

## 1. Motivation {#motivation}

### 1.1 The problem — vibevm cannot install itself {#problem}

Every other capability of vibevm assumes a `vibe` binary already exists.
Getting it there is, today, an unspecified manual act: clone, `cargo build`,
find the artifact, put it on `PATH`. There is no story for *which* version
you built, switching between versions, reclaiming the disk a Rust build tree
eats, or doing any of it on a clean machine.

Two further forces shaped v2 (§9): the owner iterates fast and must not have
to **reload the console** after each `self install`/`use`; and a distribution
is **more than one file** (today a `vibe.exe`, tomorrow DLLs and bundled
assets), all of which lock while running.

### 1.2 What VVM is — a self-distribution manager {#what}

VVM is a command group, `vibe self`, described as
**"VibeVM Version Manager / VVM"**. It builds a selected version of vibevm
from git, installs the resulting *distribution* under a managed prefix,
exposes it on `PATH` through a stable shim, and lets the user switch the
active version (with no console reload), list, garbage-collect, and remove.
It runs on Windows, macOS, and Linux, across the shells those platforms use.

### 1.3 What this is NOT — not `vibe install` {#not-install}

`req r1`

`vibe install` is the **package** manager (PROP-003 / PROP-017): it resolves
packages a *project* depends on into that project. VVM manages *the vibevm
tool itself* on *the machine* — a user-global prefix, not a project's
`vibedeps/`. The two never share code paths; `vibe self` is its own command
group so the package verbs stay uncontaminated.

## 2. Decisions {#decisions}

### 2.1 VVM is a standalone, algorithmic capability {#standalone}

`req r1`

VVM needs no LLM and no host agent. In PROP-018's terms it is a
**standalone-mode** capability — the second after `vibe skill` — behaving
identically with or without an agent. It is fully scriptable: every
interactive prompt has a non-interactive flag equivalent, so VVM works from
a bare terminal, CI, or an agent transcript.

### 2.2 Command surface — `vibe self` (+ `vibe vars`) {#surface}

`req r2`

`vibe self` — named after rustup's `self` (a tool that manages its own
versions), and unambiguous where `man` collided with the Unix manual page:

- `self install <selector>` — build and install a version (§2.7). Flags:
  `--release` / `--profile <debug|release>` (default **debug**, a single
  source-of-truth constant, §7); `--mirror <gitverse|github>` (clone path
  only, §2.7); `--force` (fresh instance, bypass the diff-copy dedup-skip);
  `-y`/`--yes`.
- `self update` — rebuild and activate the latest in-tree version; a
  shorthand for `self install latest` (§2.7), carrying `--release` /
  `--profile` / `--force`.
- `self use <selector>` — make a version active by repointing the live
  `current` file — **no console reload** (§2.5). `--eval` prints the shell
  line for the current shell instead of touching the durable environment.
- `self ls` (alias `list`) — list installed versions, marking the active one.
- `self current` / `self which` — the active selector / the active binary path.
- `self remove <selector>` (aliases `rm`, `del`, `uninstall`) — safe by
  default (§2.9).
- `self gc` — reclaim disk (§2.10).
- `self doctor` (+ `--fix`) — verify the install and environment (§2.11).
- `self env` — print activation lines for a shell.
- `self relocate <path>` — repoint source provenance to a moved checkout and
  clear the instances built from the abandoned tree (§2.17). Flags:
  `--from <old-path>` (override the inferred old location); `-y`/`--yes`
  (non-interactive); `--dry-run`.

Top-level **`vibe vars`** (§2.14) prints the runtime variable context —
the values vibevm *actually* uses (derived from `current_exe`) versus what
the environment says — so scripts never break on a stale `$VIBEVM_HOME`.

### 2.3 Version selectors and resolution {#selectors}

`req r1`

A *selector* names what to install or use; resolution is deterministic:

- `latest` → tip of branch `main`.
- `stable` → highest semantic-version git tag (the newest release).
- `X.Y.Z` → a tag; tries `X.Y.Z` then `vX.Y.Z`.
- a hex commit-ish → a commit.
- the canonical `<kind>:<id>` form (as `self ls` prints) → that exact id.
- any other bare name → branch, then tag, then commit (hex commits and
  `X.Y.Z` tags are classified before this point).

`--tag` / `--branch` / `--commit` force interpretation, mapping to
fully-qualified git refs so a name that is both never resolves by accident.
No selector: `install` → `latest`; `use`/`remove` → the current active.

### 2.4 On-disk layout — instances, the `current` pointer, manifests {#layout}

`req r1`

The **unit of install and switch is a whole distribution directory**
(an *instance*: the binary plus future DLLs/assets), not a single file
(§9.3). A version has a **canonical id** `<kind>:<id>` (`kind ∈ {tag, branch,
commit}`); each id may have several instances (one per install).

```
$VIBEVM_INSTALL_ROOT/            install base — default: home dir
                                 (Windows: %USERPROFILE%); tests pin to temp
└─ opt/
   ├─ bin/                       ← on PATH; stable shims, content never changes
   │   ├─ vibe                   POSIX shim (Git Bash / macOS / Linux)
   │   └─ vibe.cmd               cmd / PowerShell shim
   └─ vibevm/
       ├─ current                ← live pointer: the active instance dir
       ├─ state.toml             inventory: every instance + its metadata
       ├─ versions/<kind>/<id>/<instance>/   immutable distribution dirs
       │       vibe[.exe], *.dll/.so, assets…
       │       vibeterm/   the packaged vibeterm Electron app (runtime + resources/app
       │                   + node_modules; §2.7) — present only when npm/electron packaged it
       │       .vvm-manifest.toml   file list (rel,size,mtime,hash?) for diff-copy
       ├─ build/                 shared cargo --target-dir (gc-able)
       └─ src/<kind>/<id>/       MANAGED clones only (clone path); never the
                                 committer's own tree (§2.7)
```

`$VIBEVM_INSTALL_ROOT` defaults to the home dir → root `~/opt` in normal
use. One env var relocates everything; tests pin it to a temp dir.
`<instance>` is a monotonic counter (§9.4) — never a hash of the payload
(§9.2). The shim dir is stable; switching repoints `current`, never the
shim. Sources are held **by reference** (§2.7, §2.16), never bulk-copied
into the root (a checkout's `target/` is tens of GB).

### 2.5 Activation — live `current` file + `current_exe` truth {#activation}

`req r1`

**Switching must not reload the console and must not overwrite a running
file.** (v1's "env is truth" violated the first; see §9.1.) The model has
three layers:

1. **`current_exe()` → the running process's truth.** A managed `vibe` lives
   at `…/opt/vibevm/versions/<kind>/<id>/<instance>/vibe[.exe]`, so it
   derives its own version id, `VIBEVM_HOME` (= its instance dir), and
   `VIBEVM_INSTALL_ROOT` (walk up to `opt`) from its own path — no env var
   needed. Outside a managed location (dev `cargo run`, a bare copy), it
   falls back to env, then defaults.
2. **`current` file → the live active instance.** The shim reads
   `$shimdir/../vibevm/current` on **every** launch and execs that instance.
   `self use` rewrites `current` → the **next** `vibe` in the **same shell**
   uses it. No reload (the shim reads a file, not the shell's frozen env).
3. **`$VIBEVM_HOME` / `$VIBEVM_INSTALL_ROOT` (env) → advisory.** Still set
   durably for external `JAVA_HOME`-style tools, but no longer the source of
   truth. They may lag (new shells only); `vibe vars` (§2.14) reconciles,
   and a managed `vibe` whose `current_exe`-derived home disagrees with the
   env prints a one-line stderr warning at startup (suppressed outside a
   managed run).

The shims (`bin/{vibe,vibe.cmd}`) are minimal: resolve `current`, exec; if
absent, fall back to `$VIBEVM_HOME`, else print "no active vibevm — run
`vibe self use <selector>`". Both POSIX and `.cmd` shims exist (Git Bash
won't resolve `.cmd`; cmd/PowerShell won't run an extensionless script).

### 2.6 PATH and durable environment management {#path}

`req r1`

VVM detects OS and shell and manages durable settings under strict rules:

- **The shim dir on `PATH`** (stable; set once).
- **`VIBEVM_HOME` / `VIBEVM_INSTALL_ROOT`** as *advisory* env (§2.5) —
  repointed on `self use` for external tools; truth lives in `current` +
  `current_exe`.

Rules: **idempotent** (a marker guards the edit; no duplicate lines/entries),
**never clobber** (only our entry is added; the rest of `PATH` is
preserved), **OS/shell-aware** (Windows: `HKCU\Environment` via PowerShell's
`[Environment]` API, which broadcasts to new processes; POSIX: a marked
block in the detected shell's rc — bash/zsh/fish/`.profile`), and **consent
+ honesty** (mutating edits need a confirm / `-y` / `self doctor --fix`,
print the diff, and say the change reaches only new shells). The durable
writer is an injectable seam so tests exercise the POSIX rc path in a temp
file and never mutate the real machine.

### 2.7 The install pipeline — sources by reference, diff-copy to an instance {#build}

`req r1`

Installing a version is: locate the source (by reference, never copied),
build it incrementally, place the distribution into a **new immutable
instance** by diff-copy, record provenance, flip `current`.

- **Locate source (§2.16).** Two origins, never bulk-copied:
  - *external* — `self install` run inside a committer's own checkout
    (outside the install root): build it **in place**, never touch its git
    state, and record its canonical absolute path as provenance so a later
    `self install <id>` can rebuild from the remembered location (a *linked
    source*).
  - *managed* — a clone vibevm owns under `src/<kind>/<id>`: created once,
    then updated **incrementally** with `git fetch`/`checkout` (or
    `pull`; stash first if dirty), never re-cloned, so a full rebuild
    (hours, in a large future) is avoided.
- **Resolve.** The selector (§2.3) → a concrete commit, recorded.
- **Build.** `cargo build [--release] -p vibe-cli` into the **shared**
  `build/` target dir (§9.3 — never the source tree's `target/`; load-
  bearing on Windows and keeps the dev tree clean), honouring
  `rust-toolchain.toml`.
- **Package vibeterm (optional).** When `node`/`npm` are on PATH, `apps/vibeterm`
  is built into a relocatable dir (electron-packager, with node-pty rebuilt to
  Electron's ABI) and added to the dist set as the `vibeterm/` subtree. This
  runs on the target host (node-pty's native addon and Electron's runtime are
  OS/arch-specific — no cross-OS build). Skipped gracefully on a Rust-only box
  — the instance still installs; `vibe term` then names the missing setup step
  (PROP-042 §5).
- **Place by diff-copy (§2.15).** The built distribution is placed into a
  fresh instance dir, copying only files that changed versus the previous
  instance and hardlinking the rest — so a 2 GB distribution where only
  `vibe.exe` changed costs one file copy, never a full re-copy or a payload
  hash (§9.2). If nothing changed, no new instance is made.
- **Record + flip.** `state.toml` gets the instance (id, instance, commit,
  toolchain, profile, time, origin, source_path); `current` is flipped
  atomically to the new instance.

Because every install writes a **new** instance dir and switching is a
pointer flip, **no in-use file is ever overwritten** — the running process
keeps its instance dir intact; no lock, no reload, for the `.exe` or any
DLL, on any OS (§9.3).

### 2.8 Required toolchain — a single source of truth {#tools}

`req r1`

A from-source build needs **git**, a **Rust toolchain** (rustc + cargo,
stable ≥ 1.93, edition 2024 — via rustup so the pin resolves), and a
**system linker / C toolchain** (Windows: VS Build Tools; macOS: Xcode CLT;
Linux: `build-essential`). OpenSSL is deliberately not required (rustls).
The list lives once as a `REQUIRED_TOOLS` table — `(name, min_version,
check_command, help_url)` — read by `self doctor` (§2.11) and asserted by a
test; it is the runnable form of "how to update the stack" (§7). The publish
token is **never** in this set (§2.13).

### 2.9 Removal — safe by default {#remove}

`req r1`

`self remove` never silently wipes everything:

- `self remove <selector>` — remove that version's instances (and, with
  `--src`, the managed source). `--bin`/`--src`/`--both` (default both).
- `self remove` with no selector — an **interactive picker**; a non-
  interactive context errors with a hint, never a wipe.
- `self remove --all` — every version, behind the flag **and** a re-confirm.
- The **active** version and the **running** instance are protected:
  removing the active needs `--force`; a running instance's files are never
  deleted out from under it (best-effort, skipped if locked).

External sources (committer trees) are **never** removed — VVM only forgets
their provenance record; managed `src/<kind>/<id>` clones are VVM's to drop.

### 2.10 Garbage collection — `self gc` {#gc}

`req r1`

`self gc` reclaims disk:

- `--build` — clean the shared Rust build cache (`build/`); forces a rebuild
  next install but touches no installed instance.
- `--prune-others` — remove every instance except the active (and managed
  sources), behind a re-confirm.

Instances are pruned **best-effort**: a dir still locked by a running
process is skipped and collected on a later run (on POSIX the unlink
succeeds and the inode lives until the process exits). Hardlinked files are
refcount-safe — removing one instance never corrupts another that shares
inodes (§2.15). **Auto-prune on install** is enabled **only for binary
artifacts** (§9.5); source builds keep their instances until a manual `gc`
(cheap once hardlink-sharing lands). `self gc` operates **only** inside the
install root and **never** touches the shared `~/.cargo` caches.

### 2.11 Introspection — `doctor`, `ls`, `current`, `which`, `env` {#introspection}

`req r1`

`self doctor` verifies end to end: the shim dir is on `PATH`; the
`REQUIRED_TOOLS` are present with adequate versions; `current` resolves to
an installed instance whose binary exists. It prints a panel with
remediation; `--fix` performs the PATH / env edits (§2.6) with consent.
`self ls` / `current` / `which` read the **`current` file** for the active
selection (not the env). `self env` prints shell-specific activation lines.

### 2.12 Cold-start (bootstrap) {#bootstrap}

`req r1`

VVM installs `vibe`, so the first binary cannot come from `vibe self`. The
cold-start path is `git clone <mirror> && cd vibevm && cargo run -p vibe-cli
-- self install` — one `cargo run` from a fresh clone bootstraps the managed
install (as nvm is installed by a script, not by node). A generated one-line
bootstrap script is far-backlog (§6).

### 2.13 Security and trust {#security}

`req r1`

Building an arbitrary ref is arbitrary code execution — inherent to a build
tool the user invokes deliberately, and accepted. Constraints: host-key
(SSH) / TLS verification never disabled on clone; the publish token is
**never** read by VVM nor shown by `vibe vars`; VVM operates only inside the
install root and the declared, consented environment edits; the committer's
own source tree is **never** mutated (§2.7, §2.16).

### 2.14 `vibe vars` — reconciling actual vs environment {#vars}

`req r1`

Scripts must know the **real** runtime context even when `$VIBEVM_HOME` is
stale (§9.1). `vibe vars` prints the project's env-configurable variables —
`VIBEVM_INSTALL_ROOT`, `VIBEVM_HOME` (whose *actual* values are derived from
`current_exe`, §2.5), plus `VIBE_INVOKED_BY`, `VIBE_UNATTENDED`, `VIBE_LOG`
— in `NAME=VALUE` form. The publish token is deliberately excluded.

- `vibe vars` — **actual** values, one `NAME=VALUE` per line.
- `vibe vars diff` — `NAME=VALUE [ENV_VALUE]`; the bracket appears only when
  the environment differs from the actual.
- `vibe vars full` — two tables, `# ACTUAL` then `# ENVIRONMENT`.
- `vibe vars full diff` — both tables, differing names marked
  `NAME=VALUE [*]`.

"actual" for the VVM vars is the `current_exe`-derived value (falling back
to env/default outside a managed run); "environment" is the raw env. A
script reads `vibe vars` and knows exactly the context it runs in.

### 2.15 Distribution instances and diff-copy {#instances}

`req r1`

Placing a built distribution into a new instance copies only what changed
and hardlinks the rest, **without hashing gigabytes** (§9.2):

- Each instance carries `.vvm-manifest.toml`: per dist file `(rel, size,
  mtime, hash?)`. The build dir is **persistent** (shared `--target-dir`),
  so cargo preserves the mtime of unchanged outputs across builds.
- On install, for each dist file: compare to the previous instance's
  manifest entry — by **cheap content hash for small files** (≤ a
  threshold) and by **`(size, mtime)` for large files** (stat only, never
  read). Unchanged → **hardlink** the previous instance's file into the new
  one (zero copy). Changed/new → **copy** from the build output. Hardlink
  failure (cross-volume / unsupported) → copy.
- If **every** file is unchanged, no new instance is made — `current` stays
  ("already up to date"). `--force` always makes a fresh instance.
- The new instance is staged then atomically renamed, then `current` flips.
- gc is refcount-safe (§2.10); instances are immutable after publish.

This scales to a multi-GB distribution: an 80 GB asset that did not change
is shared by hardlink; only the changed `vibe.exe` is copied (§9.2, §9.6).
The `vibeterm/` subtree (~220 MB, ~3-4 k files) participates in the same
diff-copy: small files hashed by the ≤16 MiB rule, the Electron binary by
`(size, mtime)`; an unchanged vibeterm is hardlinked file-by-file free, and a
rebuild that changed nothing dedup-skips the whole instance.

### 2.16 Source provenance and linked sources {#provenance}

`req r1`

Sources are never bulk-copied into the install root (a checkout's `target/`
is tens of GB). Each instance records its **origin**:

- `managed` — a VVM-owned clone at `src/<kind>/<id>` (VVM updates it via git
  and may drop it on `remove`/`gc`).
- `external` — a committer's own checkout, identified by its **canonical
  absolute path** (`source_path`); VVM never modifies or removes it, only
  remembers where it is.
- `binary` (far-backlog) — a prebuilt artifact identified by the publisher's
  digest (computed once at publish, never re-hashed locally).

The remembered `source_path` makes an external source a **linked source**:
`self install <id>` can rebuild from the recorded location from anywhere,
without being in the checkout and without copying it. The installed instance
is self-contained (it runs without the source); the path is needed only to
rebuild, and a clear error is given if it has moved.

### 2.17 Relocate — repointing provenance after a checkout move {#relocate}

`req r1`

A committer's checkout is not pinned in place: it is cloned, moved, renamed,
re-organised on disk. When it moves, every *external* instance's remembered
`source_path` (§2.16) goes stale — a later linked-source rebuild would miss —
and the pile of instances built from the abandoned tree clutters `self ls`.
`self relocate <new-path>` is the maintenance verb for that move.

- **Validate the new location.** `<new-path>` must resolve to a real vibevm
  source tree (the `find_source_root` shape — workspace `Cargo.toml` +
  `crates/vibe-cli`); a path that is not a checkout is refused before anything
  mutates. The new path is canonicalised and `\\?\`-stripped exactly as install
  records it (§2.16), so the rewritten `source_path` matches the form every
  other record carries.
- **Infer the old location.** With no `--from`, the old path is the source
  provenance already recorded on the installed external instances (the common
  value when one checkout moved). `--from <old-path>` states it explicitly for
  an ambiguous inventory. There is nothing to relocate when no external
  instance records a source tree — the command says so and exits, never invents
  a move.
- **Repoint, then prune.** Two effects, in one atomic `state.toml` rewrite:
  *(a)* every external instance whose `source_path` is the old location is
  **repointed** to the new one — so linked-source rebuilds (§2.16) resolve to
  the live tree; *(b)* the **built instance directories** sourced from the old
  tree are **removed** — they are provenance-stale artifacts of the abandoned
  checkout, and their records are forgotten.
- **The active instance is never deleted.** Relocating must not pull the
  running binary out from under the machine: the active instance's directory is
  kept and only its `source_path` is repointed. Removing the active (or any
  version) is `self remove`'s job (§2.9). A stale instance dir still locked by
  a running process is skipped best-effort and collected later (§2.10).
- **Consent and scriptability.** Removing instances is irreversible, so the
  default is an **interactive warning** that lists what is repointed and what
  is removed, behind a confirm. `-y`/`--yes` (or `--unattended`) skips it for
  scripts and CI; a non-TTY run without `--yes` errors rather than silently
  applying (the same contract as `self remove`/`gc`, §2.9, §2.10). `--dry-run`
  prints the plan and changes nothing. `--json` emits the plan and the applied
  result. A no-op (old already equals new) is reported, not an error.

Relocate touches only `state.toml` and the install root's own `versions/`
instance dirs. It never touches a committer's source tree (external sources are
held by reference, §2.16), never the shared `build/` cache (that is `self gc`,
§2.10), and never `~/.cargo`.

## 3. Architecture — seams and cells {#architecture}

`req r1`

VVM is built from testable seams so the slow, machine-mutating parts are
mockable and unit tests never clone, build, or edit the real environment:

- `VersionStore` — the install-root layout (§2.4), instances, `current`,
  `state.toml`, manifests.
- `SourceProvider` — git: resolve a selector to a commit; clone/update a
  managed source; record external provenance.
- `Builder` — runs `cargo` for a profile/toolchain; mocked in tests.
- `Placer` — the diff-copy of a distribution into a new instance (§2.15).
- `EnvPersister` — the durable `PATH`/env edits (§2.6), injectable.
- `ToolDoctor` — the `REQUIRED_TOOLS` table and checks (§2.8).
- `vars` — the actual-vs-environment resolver (§2.14), `current_exe`-aware.

A managed `vibe` resolves its root/active from `current_exe` + the `current`
file; env is the fallback. The command lives as `cli/vvm.rs` + `cli` for
`vibe vars`, with logic under `commands/vvm/` (split across module-grain
files to hold the file-length budget). conform and specmap stay green.

## 4. MVP scope {#mvp}

The full verb set on all three platforms: `self install` (external in-place +
managed clone paths, debug + release, diff-copy into instances), `self use`
(live `current`, no reload), `self ls`/`current`/`which`, `self remove` (safe
+ `--all`), `self gc` (build cache + prune), `self doctor` (+ `--fix`),
`self env`, `self relocate` (§2.17), and `vibe vars`. Selector resolution per §2.3; durable
PATH/advisory-env per §2.6 across Windows (cmd/PowerShell/Git Bash), macOS
(zsh/bash), Linux (bash/zsh/fish). diff-copy with hardlink sharing is in
scope (§2.15). Linked sources (§2.16) are in scope (the `source_path`
record + rebuild-from-remembered).

## 5. Out of scope (now) {#out-of-scope}

Prebuilt-binary installs (`--binary`); offline / vendored builds;
cryptographic signature verification; reflink/CoW placement (hardlink is the
portable choice). These are §6.

## 6. Far backlog {#far-backlog}

- `self install --binary` — fetch a prebuilt artifact keyed by the
  publisher's digest (counter instance, full copy, auto-prune on, §9.5).
- A generated one-line bootstrap script for cold-start (§2.12).
- Offline builds via vendoring or a registry mirror.
- Reflink/CoW placement where the filesystem supports it (§2.15).
- Signature/provenance verification of the resolved ref.

## 7. Maintenance & evolution — updating the stack {#maintenance}

`req r1`

Knowledge is runnable, so updates are mechanical: the **required tools** are
the `REQUIRED_TOOLS` table (§2.8, asserted by a test); the **default
profile** is one constant (§2.2); the **Rust pin** is `rust-toolchain.toml`
(read, not hard-coded); the **clone mirrors** are PROP-016's `mirrors.toml`.

## 8. Acceptance {#acceptance}

`req r1`

- From a fresh clone, `cargo run -p vibe-cli -- self install` produces a
  working managed install and a `vibe` on `PATH` (after the printed
  activation step) on Windows, macOS, Linux.
- `self use` switches the active version and the **next** `vibe` in the
  **same shell** is the new one — no reload (`current` file).
- Reinstalling the running version replaces no in-use file (new instance +
  pointer flip); the running process is unharmed.
- A distribution where only `vibe.exe` changed copies one file; unchanged
  files are hardlinked (`.vvm-manifest.toml` diff); no payload is hashed in
  bulk.
- `vibe vars` reports actual vs environment; `vibe vars diff`/`full`/`full
  diff` per §2.14; the publish token never appears.
- `self remove` never wipes without `--all` + reconfirm; `self gc` never
  touches `~/.cargo`; external sources are never modified or removed.
- `self relocate <new>` repoints external `source_path` records and removes the
  stale instance dirs built from the old tree, keeping the active instance; the
  active's source is repointed, not deleted. `--dry-run` changes nothing; a
  non-TTY run without `--yes` errors.
- Full `self-check.sh` green; conform 0/0/0; specmap clean.

## 9. Design rationale & questions explored {#rationale}

The decisions above were reached by working through several sharp questions;
recording them so a cold reader sees *why*, not just *what*.

### 9.1 Why `current` file + `current_exe`, not `$VIBEVM_HOME` (v1) {#rationale-truth}

v1 made `$VIBEVM_HOME` the single source of truth for the active version.
Environment variables are inherited at process start, so a shell's
`$VIBEVM_HOME` is frozen until the shell is reloaded — every `self use`/
reinstall forced "open a new terminal". The fix: the **shim reads a live
`current` file** each launch (filesystem is live → instant switch in the
same shell), and a running `vibe` derives its own identity from
**`current_exe()`** (it *is* the binary, so it knows its path). `$VIBEVM_HOME`
stays only as an advisory/compat env for external tools, reconciled by
`vibe vars` (§2.14) and a startup divergence warning. This reverses v1's
decision deliberately; env-as-truth was the cause of the reload friction.

### 9.2 Why not content-hash the distribution {#rationale-no-hash}

A natural instance key is a content hash of the built distribution
(dedup + self-describing). It does not scale: a distribution may grow to
gigabytes and ship as binaries (merged projects), and hashing 2 GB+ on every
install would be prohibitive. So the instance key never reads the payload
(§9.4), and change detection for diff-copy hashes only **small** files,
trusting `(size, mtime)` for large ones (§2.15). Future binary artifacts are
keyed by the **publisher's** digest (computed once at publish), never
re-hashed locally.

### 9.3 Why whole-directory instances, not in-place file replace {#rationale-instances}

The first idea for "reinstall over the running binary" was the Windows
*rename-aside* trick (rename the running `.exe`, write the new one;
empirically verified to work). It handles one file; a distribution is many
(exe + DLLs + assets), all locked while running. So the unit of install and
switch became the **whole immutable instance directory**: each install
writes a *new* dir and switching is a pointer flip, so **nothing in use is
ever overwritten** — no lock for any file on any OS, and no reload.
rename-aside was dropped as unnecessary.

### 9.4 Why a monotonic counter for the instance key {#rationale-counter}

With content-hash rejected (§9.2), the instance key is a monotonic counter:
always unique, O(1), independent of distribution size. "Did anything change"
is answered cheaply by the diff-copy manifest (§2.15), which also yields the
*dedup-skip* (no new instance when every file is unchanged) without hashing
the payload. `--force` bypasses the skip.

### 9.5 Why auto-prune only for binary artifacts {#rationale-prune}

Auto-pruning old instances on install bounds disk. It is enabled only for
**binary** installs (full copies, no source/git context). Source rebuilds
keep their instances until a manual `gc`: they are cheap once hardlink-
sharing lands (instances share unchanged files), and a committer may want
several around. (The owner chose this split.)

### 9.6 Why sources are held by reference, never copied {#rationale-sources}

Copying a checkout into the install root is untenable — a working tree's
cargo `target/` is already tens of GB. So managed sources are git clones
VVM updates incrementally, and external (committer) sources are referenced
by their absolute path and built **in place**, never touched. This also
gives *linked sources* (§2.16): rebuild from a remembered location without
being in it. The built distribution *is* copied (small relative to source;
diff-copy keeps even that minimal), but never the source.

### 9.7 Why the binary, not the shim, emits the divergence warning {#rationale-warning}

When `current` and `$VIBEVM_HOME` disagree, the warning is emitted by the
`vibe` **binary** at startup, not by the sh/cmd shim: the binary has
`current_exe` ground truth and the `vibe vars` formatter, and keeps the
shims trivial. The warning is suppressed outside a managed run (a dev
`cargo run` has no managed location and should not be nagged).
