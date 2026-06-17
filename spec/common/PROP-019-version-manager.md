# PROP-019 — VibeVM Version Manager (VVM) {#root}

**Status:** proposed 2026-06-17 — owner-requested design session. The MVP
slice this PROP authorises is named in §4; heavier work is parked in §6
(far backlog). This is the spec home for vibevm's *self-distribution* — the
ability of the `vibe` binary to build, install, switch, and remove its own
versions on the current machine, in the spirit of Node Version Manager
(nvm) but in our own execution.
**Related:** [PROP-018](PROP-018-agentic-standalone-modes.md) (VVM is a
second *standalone-mode* citizen after `vibe skill` — pure algorithm, no
LLM; §2.1), [PROP-016](PROP-016-source-mirrors.md) (the source mirrors VVM
clones from when run outside a source tree — GitVerse + GitHub, the target
list is `mirrors.toml`), [PROP-000 §7](PROP-000.md#registry) (the
source/registry split-host posture) and [PROP-000 §20](PROP-000.md#token-secrecy)
(the publish token — which VVM never touches), [`VIBEVM-SPEC.md`](../../VIBEVM-SPEC.md)
(the CLI-first, agent-agnostic posture VVM extends to the install step
itself), and the repo's own `rust-toolchain.toml` (the pin VVM honours when
building).

---

## 1. Motivation {#motivation}

### 1.1 The problem — vibevm cannot install itself {#problem}

Every other capability of vibevm assumes a `vibe` binary already exists on
the machine. Getting that binary there is, today, an unspecified manual
act: clone the source, run `cargo build`, find the artifact, put it
somewhere on `PATH`. There is no story for *which* version you built, for
switching between versions, for reclaiming the disk a Rust build tree eats,
or for doing any of this on a clean machine where `PATH` is not yet set up.

This is the one workflow a spec-driven tool should make first-class: a user
clones the sources and, from inside the tree or from a single managed
command, gets a working, switchable, removable `vibe`. The model is nvm
(`nvm install`, `nvm use`, `nvm ls`, `nvm uninstall`) — adapted to a tool
that is *built from source* rather than downloaded as a prebuilt artifact.

### 1.2 What VVM is — a self-distribution manager {#what}

VVM is a command group, `vibe man` (alias `vibe manager`), described to the
user as **"VibeVM Version Manager / VVM"**. It builds a selected version of
vibevm from git, installs the resulting binary under a managed prefix,
exposes it on `PATH` through a stable shim, and lets the user switch the
active version, list what is installed, garbage-collect Rust build debris,
and remove versions. It runs on Windows, macOS, and Linux, across the
shells those platforms actually use (§2.6).

### 1.3 What this is NOT — not `vibe install` {#not-install}

`req r1`

`vibe install` is the **package** manager (PROP-003 / PROP-017): it
resolves and installs *packages a project depends on* into a project. VVM
manages *the vibevm tool itself* on *the machine*. Different domain,
different state location (a user-global prefix, not a project's
`vibedeps/`), different lifecycle. The two never share code paths or
semantics; `vibe man` is its own command group precisely so the package
verbs stay uncontaminated.

## 2. Decisions {#decisions}

### 2.1 VVM is a standalone, algorithmic capability {#standalone}

`req r1`

VVM requires no LLM and no host agent: resolving a git ref, running
`cargo`, copying a binary, and editing `PATH` are pure algorithm. In
PROP-018's terms it is a **standalone-mode** capability — the second after
`vibe skill` — and behaves identically whether or not an agent is present.
It must therefore be fully scriptable: every interactive prompt this PROP
introduces has a non-interactive flag equivalent (§2.9, §2.10), so VVM
works from a bare terminal, a CI job, or an agent transcript.

### 2.2 Command surface — `vibe man` {#surface}

`req r1`

The group is `vibe man` (visible alias `vibe manager`). Its subcommands:

- `man install <selector>` — build and install a version (§2.7). Flags:
  `--release` / `--profile <debug|release>` (default **debug** today — a
  single source-of-truth constant, §7, to be flipped to `release` later);
  `--mirror <gitverse|github>` (default `gitverse`) used only when a clone
  is needed (§2.7); `--force` (rebuild even if present); `-y`/`--assume-yes`.
- `man use <selector>` — make a version active (§2.5). `--eval` prints the
  shell line for instant current-shell activation instead of mutating the
  durable environment.
- `man ls` (alias `list`) — list installed versions, marking the active one
  and showing each one's resolved commit, profile, and build time.
- `man current` — print the active selector; `man which` — print the
  absolute path of the active `vibe` binary.
- `man remove <selector>` (visible aliases `rm`, `del`, `uninstall`) —
  remove a version, safe by default (§2.9).
- `man gc` — reclaim Rust build debris (§2.10).
- `man doctor` — verify the install and the environment (§2.11);
  `man env` — print activation lines for a shell (the `--eval` helper).

Terse single-letter aliases (`d`, `r`) are deliberately **not** provided:
they hurt discoverability and risk future collisions. The destructive
two-word form `clear garbage` is replaced by the unambiguous `man gc`.

### 2.3 Version selectors and resolution {#selectors}

`req r1`

A *selector* is the user-facing string naming what to install or use. Its
resolution is deterministic:

- `latest` → the tip of branch `main` (a moving target; the resolved commit
  is recorded at install time, §2.7).
- `stable` → the highest semantic-version git tag (the newest *release*).
  This is added because users coming from nvm expect "latest" to mean the
  newest release; `latest` here means main-tip, so `stable` covers the
  other expectation.
- `X.Y.Z` → a release tag; resolution tries `X.Y.Z` then `vX.Y.Z`.
- a hex string that resolves as a commit-ish → a commit.
- any other bare name → resolved by precedence **commit > branch > tag**
  when more than one matches (the owner's chosen order).

`--tag` / `--branch` / `--commit` force the interpretation of an ambiguous
name, mapping to fully-qualified git refs (`refs/tags/…`, `refs/heads/…`)
so a name that is both a branch and a tag never resolves by accident. If no
selector is given: `install` defaults to `latest`; `use`/`remove`/`gc`
default to the **current** active version.

### 2.4 On-disk layout and the canonical version id {#layout}

`req r1`

Every version has a **canonical id** `<kind>:<id>`, where `kind ∈ {tag,
branch, commit}`. This fixes a collision in the naive "flat `bin/$version`"
layout: a tag `1.2.3` and a branch `1.2.3` would otherwise want the same
directory. The id namespaces them, and the same `<kind>/<id>` path segment
is used for both binaries and sources so the two always agree.

```
$VIBEVM_INSTALL_ROOT/         install base — default: the user's home dir
                              (Windows: %USERPROFILE%); tests pin it to a temp dir
└─ opt/                       the VVM root ($VIBEVM_INSTALL_ROOT/opt)
   ├─ bin/                    ← on PATH; stable shims, content never changes
   │   ├─ vibe                POSIX shim (Git Bash / macOS / Linux)
   │   └─ vibe.cmd            cmd / PowerShell shim
   └─ vibevm/
       ├─ state.toml          inventory: every install + its metadata
       ├─ versions/<kind>/<id>/vibe[.exe]   the built binaries (immutable)
       ├─ build/              shared cargo --target-dir for builds (gc-able)
       └─ src/<kind>/<id>/…   cloned source trees (clone path; gc-able)
```

The install base is **`$VIBEVM_INSTALL_ROOT`**, defaulting to the user's
home directory, so the VVM root is `$VIBEVM_INSTALL_ROOT/opt` — i.e. `~/opt`
in normal use (the owner-specified layout). That single env var relocates
everything: tests set it to a temp directory so an install never touches the
real `~/opt`. (This deliberately differs from the existing `~/.vibevm/`
token home and the project-local `.vibe/` cache; VVM is a machine-global
tool.) The binaries live under `versions/` — not the originally sketched
flat `bin/$version` — to namespace by kind and to not clash with the
`opt/bin` shim dir. Builds use a **separate managed `build/` target dir**,
never the source tree's own `target/`: this keeps the dev tree clean and,
load-bearing on Windows, stops cargo from relinking a `vibe.exe` that is the
currently-running binary (§2.7). Keeping the slim built binary while gc-ing
the heavy `build/` and `src/.../target/` is an explicit feature (§2.10).

### 2.5 Activation — the shim plus the `VIBEVM_HOME` env var {#activation}

`req r1`

Switching the active version must not move or replace a running binary
(impossible to do to an open `.exe` on Windows) and must not require
symlink privileges (not granted by default on Windows). Both constraints
are met by a **shim plus an environment variable**, the "`JAVA_HOME` model":

- The PATH entry is a **stable shim** (`$VIBEVM_INSTALL_ROOT/opt/bin/{vibe,vibe.cmd}`)
  whose content never changes. The POSIX shim is a `sh` script; the
  Windows shim is a `.cmd` — both are needed because Git Bash does not
  resolve `.cmd` on a bare `vibe` while cmd/PowerShell do not run an
  extensionless script.
- The active version is named by **`VIBEVM_HOME`**, an environment variable
  that points to the active version's prefix (e.g.
  `~/opt/vibevm/versions/tag/1.2.3`). It is persisted durably in the OS
  environment (Windows user environment via the registry; the appropriate
  shell rc on POSIX) and **repointed by `man use`** — its value changes
  with the active version.
- Each shim reads `VIBEVM_HOME` and execs `"$VIBEVM_HOME"/vibe[.exe]
  "$@"`. If `VIBEVM_HOME` is unset, the shim prints a clear message
  ("no active vibevm — run `vibe man use <selector>` or `vibe man install
  latest`").

`VIBEVM_HOME` is the single source of truth for "which version is active";
`state.toml` is the inventory of what is installed. Because a durable
environment edit only reaches **new** shells, `man use` prints the
activation hint, and `man env` / `man use --eval` emit the line to `eval`
for instant effect in the current shell.

### 2.6 PATH and environment management {#path}

`req r1`

VVM detects the OS and shell and manages two durable settings — the shim
directory on `PATH`, and `VIBEVM_HOME` — under strict rules:

- **Idempotent.** A marker guards the edit; re-running never appends a
  duplicate line or PATH entry.
- **Never clobber.** Only our entry is added; the user's existing `PATH`
  order and content are preserved. The whole variable is never rewritten.
- **OS/shell-aware.** Windows: the user `PATH` and `VIBEVM_HOME` in
  `HKCU\Environment`, broadcasting `WM_SETTINGCHANGE` so new processes see
  it (the rustup approach); `~` resolves to `%USERPROFILE%`. POSIX: the
  right rc for the detected shell — bash (`.bashrc` / `.bash_profile` /
  `.profile`), zsh (`.zshrc` / `.zshenv`), fish (`fish_add_path` /
  `config.fish`), and PowerShell Core (`$PROFILE`).
- **Consent and honesty.** A mutating edit happens only with consent (an
  interactive confirm, or `-y`, or `man doctor --fix`), prints the exact
  diff, and tells the user that the change reaches only new shells — print
  the `source ~/.bashrc` / "open a new terminal" instruction. Full
  cross-platform coverage (Windows cmd/PowerShell/Git Bash; macOS
  zsh/bash; Linux bash/zsh/fish) is in scope per the owner's decision.

### 2.7 The build pipeline {#build}

`req r1`

Installing a version is: locate the source, build it, atomically publish
the binary, record metadata.

- **Locate source.** If `vibe man install` runs from inside a vibevm source
  tree (a working copy), that tree is built as-is. Otherwise VVM clones the
  chosen mirror (default GitVerse, else GitHub — interactive choice or
  `--mirror`) with the system `git` into `src/<kind>/<id>`, using
  `--recurse-submodules` for forward-safety (the repo has no submodules
  today, so this is currently a no-op — it is not claimed to be required).
- **Resolve.** The selector (§2.3) is resolved against the source to a
  concrete commit; that commit is recorded so a branch install is
  reproducible after the fact.
- **Build.** `cargo build [--release] -p vibe-cli`, honouring
  `rust-toolchain.toml` (channel `stable`, pin ≥ 1.93). No build scripts or
  codegen tools are needed — vibevm's generated crate (`vibe-wire`) is
  checked in, and no workspace crate has a `build.rs`.
- **Publish atomically.** The built `vibe[.exe]` is staged and only on
  success moved into `versions/<kind>/<id>/`. A lock prevents two
  concurrent installs from racing. A failed build never leaves a version
  marked installed.
- **Record.** `state.toml` gets the install: canonical id, resolved commit,
  toolchain version, profile, and timestamp.

Offline builds are not supported today: there is no dependency vendoring or
registry mirror in `.cargo/config.toml`, so `cargo` reaches crates.io.

### 2.8 Required toolchain — a single source of truth {#tools}

`req r1`

The tools a build needs are, grounded in the actual workspace: **git**
(clone), a **Rust toolchain** (rustc + cargo, stable ≥ 1.93, edition 2024 —
best installed via rustup so the pin resolves automatically), and a
**system linker / C toolchain** (Windows: VS Build Tools "Desktop
development with C++"; macOS: Xcode Command Line Tools; Linux:
`build-essential`). OpenSSL is deliberately **not** required — vibevm uses
`rustls` (reqwest with `rustls-tls`).

This list lives once, in code, as a `REQUIRED_TOOLS` table — each entry a
`(name, min_version, check_command, help_url)` — read by `man doctor`
(§2.11) and asserted by a test against what the build actually uses. When a
tool is missing, VVM names it and prints its `help_url` (rustup.rs;
git-scm.com; the platform C-toolchain page). The table is the runnable form
of "how to update the stack" (§7).

### 2.9 Removal — safe by default {#remove}

`req r1`

`man remove` never silently wipes everything. Behaviour:

- `man remove <selector>` — remove that version; an interactive choice (or
  `--bin` / `--src` / `--both`, default both) selects whether to drop the
  built binary, the source+target tree, or both.
- `man remove` with no selector — present an **interactive picker** of
  installed versions to choose what to remove; in a non-interactive context
  this is an error printing a hint, not a wipe.
- `man remove --all` — remove every version, behind an explicit flag **and**
  a re-confirmation.
- The version backing the **current process** and the **active** version
  are protected: removing the active version requires `--force` and warns,
  and the running binary is never deleted out from under itself.

Sources may live under different `<kind>` paths; removal resolves the exact
`src/<kind>/<id>` and `versions/<kind>/<id>` for the selector rather than
guessing.

### 2.10 Garbage collection — `man gc` {#gc}

`req r1`

`man gc` reclaims the disk a Rust build tree eats. Interactively it offers,
and via flags it exposes:

- `--build` — clean the **Rust build cache**. Because every build shares one
  managed `--target-dir` (`build/`, §2.4 — the fix for the Windows
  running-binary relink), the originally sketched per-version "current" vs
  "all targets" distinction collapses to this single cache; clearing it
  forces a rebuild on the next install but touches no installed binary.
- `--prune-others` — remove all versions except the current, **including**
  their sources and binaries (and the build cache); behind a
  re-confirmation ("точно?").

With no flag, an interactive menu offers the two; a non-interactive run must
pass one. `man gc` operates **only** inside `$VIBEVM_INSTALL_ROOT/opt`. It
must never touch the shared `~/.cargo/registry` or `~/.cargo/git` caches —
those belong to every Rust project on the machine, and cleaning them would
damage unrelated work.

### 2.11 Introspection — `doctor`, `ls`, `current`, `which`, `env` {#introspection}

`req r1`

`man doctor` verifies the install end to end: `$VIBEVM_ROOT/bin` exists and
is on `PATH`; `VIBEVM_HOME` is set and points at an installed version; the
`REQUIRED_TOOLS` are present with adequate versions; the active binary runs.
It prints a green/red panel with remediation, and `--fix` performs the
PATH / `VIBEVM_HOME` edits (§2.6) with consent. `man ls`, `man current`,
and `man which` report inventory and the active selection; `man env` prints
the shell-specific activation lines (the `--eval` helper).

### 2.12 Cold-start (bootstrap) {#bootstrap}

`req r1`

VVM is a feature *of* `vibe`, yet its job is to install `vibe`, so the very
first binary cannot come from `vibe man`. The specified cold-start path,
for a machine with neither a binary nor a checkout, is:

```
git clone <mirror> && cd vibevm && cargo run -p vibe-cli -- man install latest
```

i.e. one `cargo run` from a fresh clone bootstraps the managed install,
exactly as nvm is itself installed by a script rather than by node. This
path is documented as first-class; a generated one-line bootstrap
script (`.sh` / `.ps1`) that performs it is a far-backlog convenience
(§6), not part of the MVP.

### 2.13 Security and trust {#security}

`req r1`

Building an arbitrary branch or commit is arbitrary code execution — that
is inherent to a build-from-source tool the user invokes deliberately, and
is accepted. The constraints: host-key (SSH) and TLS verification are never
disabled on clone; the publish token (`~/.vibevm/github.publish.token`) is
**never** read or used by VVM — clones use SSH or public HTTPS, keeping the
PROP-000 §20 token discipline intact; VVM operates only within
`$VIBEVM_ROOT` and the declared environment edits, never elsewhere.

## 3. Architecture — seams and cells {#architecture}

`req r1`

VVM is built from testable seams so the slow, machine-mutating parts are
mockable and the unit tests never actually clone, build, or edit the real
environment:

- `VersionStore` — owns `$VIBEVM_ROOT`, the canonical-id layout (§2.4),
  `state.toml`, and metadata.
- `SourceProvider` — git: resolve a selector to a commit, clone a mirror.
- `Builder` — runs `cargo` for a profile and toolchain; mocked in tests.
- `PathManager` — OS/shell detection and the idempotent durable edits of
  `PATH` and `VIBEVM_HOME` (§2.6).
- `ToolDoctor` — the `REQUIRED_TOOLS` table and the checks (§2.8, §2.11).

Each public seam carries one compiled doctest of canonical use; a
non-trivial seam that replaces an existing behaviour carries a differential
oracle. The command lives as `cli/man.rs` (clap derive: subcommands and
aliases) plus `commands/man/mod.rs` (logic), following the established
`cli/<cmd>.rs` ↔ `commands/<cmd>/mod.rs` split. conform and specmap stay
green; a new `#[cell]` gets its name-reference oracle.

## 4. MVP scope — what this PROP authorises now {#mvp}

The MVP is the full set of verbs above on all three platforms:
`man install` (in-tree and clone paths, debug + release), `man use` (shim +
`VIBEVM_HOME`), `man ls` / `current` / `which`, `man remove` (safe
default + `--all`), `man gc`, `man doctor` (+ `--fix`), `man env`.
Selector resolution per §2.3 (`latest`, `stable`, tag, branch, commit, with
`--tag/--branch/--commit`). PATH / `VIBEVM_HOME` management per §2.6 across
Windows (cmd/PowerShell/Git Bash), macOS (zsh/bash), Linux (bash/zsh/fish).

## 5. Out of scope (now) {#out-of-scope}

Prebuilt-binary installs (download instead of build); offline / vendored
builds; registering an arbitrary dev tree as a named version (`man link`);
cryptographic verification of tags/commits. These are §6.

## 6. Far backlog {#far-backlog}

- `man install --binary` — fetch a prebuilt release artifact instead of
  building, once a release pipeline produces artifacts.
- A generated one-line bootstrap script (`.sh` / `.ps1`) for cold-start
  (§2.12).
- Offline builds via dependency vendoring or a registry mirror.
- `man link` — register an existing working tree as a named version for dev.
- Signature/provenance verification of the resolved ref.

## 7. Maintenance & evolution — updating the stack {#maintenance}

`req r1`

When the toolchain story changes, the update is mechanical because the
knowledge is runnable, not prose:

- **The required-tools list** is the `REQUIRED_TOOLS` table (§2.8). Add or
  bump a tool there — its name, minimum version, check command, and help
  URL — and `man doctor` and the docs follow. A test asserts the table
  matches what the build actually invokes, so drift fails CI.
- **The default build profile** is one constant (§2.2). Flipping the
  default from `debug` to `release` when the time comes is a one-line
  change plus its test.
- **The Rust pin** is `rust-toolchain.toml`; VVM reads it rather than
  hard-coding a version, so bumping the pin needs no VVM change.
- **The clone mirrors** are PROP-016's `mirrors.toml` target set; VVM reads
  that list rather than hard-coding hosts.

## 8. Acceptance {#acceptance}

`req r1`

- From a fresh clone, `cargo run -p vibe-cli -- man install latest` produces
  a working managed install and a `vibe` on `PATH` (after the printed
  activation step) on Windows, macOS, and Linux.
- `man install <tag|branch|commit>` resolves and builds the right ref;
  `man ls` shows it with its resolved commit; `man use` switches the active
  version; `man which` points at it.
- `man remove` never wipes everything without `--all` + reconfirm; `man gc`
  reclaims target trees without touching the shared cargo caches.
- `man doctor` reports missing tools with help URLs and, with `--fix`,
  idempotently puts `$VIBEVM_ROOT/bin` on `PATH` and sets `VIBEVM_HOME`.
- Full `self-check.sh` green; conform 0/0/0; specmap clean (the new units
  and any `#[cell]` oracles included).
