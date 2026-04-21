# PROP-001: Git-backed registry for `vibe-registry` {#root}

**Milestone:** M1.1 ([`ROADMAP.md`](../../../ROADMAP.md#m11--git-backed-registry)).
**Status:** accepted 2026-04-22, implementation in progress.
**Supersedes:** nothing.
**Related:** [spec://vibevm/common/PROP-000#registry](../../common/PROP-000.md#registry), [`VIBEVM-SPEC.md` §8](../../../VIBEVM-SPEC.md).

---

## 1. Motivation {#motivation}

M0 shipped with a local-directory registry only. M1 makes the registry a git
repository hosted on GitVerse (default
`git@gitverse.ru:anarchic/vibespecs.git`). The implementation must:

- Clone the registry into `~/.vibe/registries/<hash>/` on first use and
  `git pull` on subsequent use (`VIBEVM-SPEC.md` §8.3).
- Preserve the existing `LocalRegistry` code path so tests and the
  `--registry <path>` override keep working.
- Authenticate against GitVerse using the SSH identity the user has
  already configured (see [`spec/boot/90-user.md`](../../boot/90-user.md)).
- Run on Windows, macOS, and Linux with no per-platform build hoops.
- Carry its operational weight on constrained dev machines without
  bloating the `vibe` binary or adding a C toolchain requirement.

This PROP records the architectural decisions. The mechanics
(`Registry` trait surface, error variants, wire-level command lines)
live in the module's README as they land.

---

## 2. Decisions {#decisions}

### 2.1 Backend: shell out to `git`, not `git2` {#backend}

**Decision:** `vibe-registry` performs all git operations by spawning the
system `git` binary via `std::process::Command`. We do **not** link
against `libgit2` (via the `git2` crate) in v1.

**Why:**

1. **SSH on Windows is the killer.** GitVerse authenticates via SSH.
   Git for Windows ships OpenSSH and a working `ssh-agent`; the user's
   identity (`olegchir@UNIT-2040`) is already loaded and the push to
   `gitverse.ru` is proven (see [`spec/boot/90-user.md`](../../boot/90-user.md)).
   `libgit2` uses `libssh2` for SSH, which talks to `ssh-agent`
   through a named-pipe protocol that is fragile on Windows and
   routinely requires `SSH_AUTH_SOCK` juggling or explicit key paths.
   Cargo itself falls back to the system `git` on auth failure for
   this exact reason. Shell-out makes that fallback the primary path
   and retires the class of bug.

2. **Dependency footprint.** `git2` pulls `libgit2-sys`,
   `libssh2-sys`, `libz-sys`, and `openssl-sys` (or a vendored
   alternative). Non-vendored builds demand a working C toolchain on
   every developer and CI machine; vendored builds add 3–8 MB to the
   release binary. Shell-out adds zero bytes and zero build-time
   native dependencies.

3. **Feature parity and debuggability.** The user's `git` is by
   definition current. Errors surface with the full native message;
   `tracing` logs the exact argv so a user can re-run the failing
   command by hand. `libgit2`'s error strings (`ERROR class=Net (12):
   unexpected http status code: 401`) are harder to diagnose.

4. **We do not need programmatic git.** The v1 operations are
   `git clone`, `git fetch`, `git pull --ff-only`, and `git
   --version` for preflight. No partial clone, no in-memory object
   reads, no custom refspecs, no progress UI. Shell-out handles this
   trivially.

5. **Licensing.** `git` is GPL v2, but shell-out is `exec` not
   linkage — the GNU FAQ explicitly separates these. Our binary stays
   unambiguously permissive.
   `libgit2` is GPL v2 with a Linking Exception (permissive for our
   purposes), but shell-out leaves the entire conversation at the
   door.

**Risks accepted:**

- **Runtime dependency on `git` in `PATH`.** Acceptable: our target
  audience is developers who already have git installed. We perform a
  preflight `git --version` check and emit an actionable error (with
  a pointer to `https://git-scm.com/downloads`) if it is missing.
- **stderr parsing for fine-grained error classification.** We
  mitigate by running git with `LC_ALL=C` and keying off exit code
  + substring markers (`fatal: ` prefix, `Permission denied
  (publickey)`, `Repository not found`). See §2.7.

**When to revisit:** if and when we need one of:
- partial/sparse clone with custom filters,
- programmatic object reads (e.g. to fetch a `latest` marker file
  without a working-tree checkout),
- OS-credential-store integration that can't be delegated to `git`,
- running on a platform where bundling `git` is easier than requiring
  it.

At that point, add a `libgit2` feature behind the `GitBackend` trait
(§2.2). The trait is designed so the switch costs one `impl` block
and one line in the factory, and nothing else in the codebase moves.

### 2.2 `GitBackend` trait {#backend-trait}

**Decision:** `vibe-registry::git_backend::GitBackend` is the single
interface through which the registry layer touches git. It has exactly
the operations we use:

```rust
pub trait GitBackend: Send + Sync {
    /// Clone `url` (checked out at `refname`) into `dest`.
    /// Caller guarantees `dest` is either empty or absent.
    fn bootstrap(&self, url: &str, refname: &str, dest: &Path) -> Result<(), GitError>;

    /// Fast-forward `dest` to the tip of `refname` on origin.
    /// No-op if already up to date.
    fn update(&self, dest: &Path, refname: &str) -> Result<(), GitError>;
}
```

**Method-name note.** The "make a fresh clone" operation is called
`bootstrap` rather than the obvious `clone` or `clone_into` because
the backend is held as `Arc<dyn GitBackend>` at its call sites and
both of those names collide with blanket-impl methods from the
standard library (`std::clone::Clone::clone`,
`std::borrow::ToOwned::clone_into`), forcing ugly `<T as
GitBackend>::…` disambiguations at every call. `bootstrap` is
semantically accurate — it's how we initialise the registry cache
from empty state — and has no std-library namesake.

**Why narrow.** The narrower the trait, the cheaper the backend swap.
If we need `ls_remote` or `fetch_ref` later, we add a method — that
addition is a visible, deliberate change, not a quiet interface drift.

**Implementations:**

- `ShellGit` — default, built from `std::process::Command`. See §2.7.
- `LibGit2` — reserved. Not implemented in M1; the trait is the entry
  point for a future feature-gated addition.

The `vibe-registry` crate does not expose a mock implementation.
Tests use `ShellGit` against a bare git repository created in a
`tempdir` — exercising the production code path end-to-end.

### 2.3 `Registry` trait {#registry-trait}

**Decision:** introduce a `vibe-registry::Registry` trait that both
`LocalRegistry` and `GitRegistry` implement:

```rust
pub trait Registry {
    fn list_versions(&self, kind: PackageKind, name: &str)
        -> Result<Vec<semver::Version>, RegistryError>;
    fn resolve(&self, pkgref: &PackageRef)
        -> Result<ResolvedPackage, RegistryError>;
    fn fetch(&self, resolved: &ResolvedPackage, cache_root: &Path)
        -> Result<CachedPackage, RegistryError>;
}
```

`vibe-install` and `vibe-cli` continue to consume `ResolvedPackage` /
`CachedPackage` exactly as in M0; the only change is that the concrete
type is chosen at CLI-arg-parse time.

**Selection rule.** CLI precedence stays as defined in `VIBEVM-SPEC.md`
§9.1: `--registry <path>` (explicit, always a local directory) wins
over the `[registry]` section in `vibe.toml` (a URL — git or
`file://`).

### 2.4 Cache layout {#cache-layout}

**Decision:** the on-disk layout under `~/.vibe/registries/` is:

```
~/.vibe/registries/
└── <hash>/
    ├── clone/        ← the git working tree
    └── meta.toml     ← { url, ref, last_pulled_at }
```

- `<hash>` = lowercase hex of the first 16 bytes of
  `sha256(normalized_url)`. 16 hex chars is enough to avoid realistic
  collisions while keeping the directory name tab-completable (same
  trick Cargo uses for its git cache). The full hash lives in
  `meta.toml` for audit.
- `normalized_url` strips a trailing `.git` and lowercases the scheme
  + host so `git@gitverse.ru:anarchic/vibespecs.git` and
  `ssh://git@gitverse.ru/anarchic/vibespecs` hash to the same
  registry.
- `meta.toml` is written after each successful clone or update. It
  carries the url (for debugging), the ref, and the UTC RFC3339
  timestamp of the last successful fetch.
- The `clone/` subdirectory is the registry working tree. `GitRegistry`
  internally wraps a `LocalRegistry::new(clone_dir)` and delegates
  `resolve` / `list_versions` / `fetch` to it — the packaged layout
  (`<kind>/<name>/v<ver>/…`) is identical in both worlds.

Per-project package cache (`<project>/.vibe/cache/<kind>/<name>/<ver>/`)
is unchanged from M0.

### 2.5 Freshness policy {#freshness}

**Decision:** the default freshness TTL is **1 hour**, checked against
`meta.toml.last_pulled_at`. An install whose registry cache is older
than the TTL triggers an implicit `update`. An install whose cache is
younger skips the pull. `vibe registry sync` forces an update
regardless of age.

**Why 1 hour:** short enough to pick up new package versions within
one working session, long enough to amortise network round-trips over
a burst of installs. Revisit once real usage arrives.

**No `--offline` flag yet.** If the network is down during an implicit
update, the pull fails and the install fails with a clear message.
True offline-first mode is M2 polish.

### 2.6 Lockfile `source_uri` format {#source-uri}

**Decision:** when a package originates from a git registry, the
lockfile records its source as

```
git+ssh://git@gitverse.ru/anarchic/vibespecs.git#<kind>/<name>/v<ver>
```

The `#fragment` names the package directory inside the registry
relative to the registry root. The scheme prefix (`git+ssh` /
`git+https` / `git+file`) encodes the transport. Local-directory
registries continue to produce `file://…` URIs as in M0.

**Why a scheme prefix.** `pip` and Cargo both use `git+…` prefixes to
disambiguate a git source from a plain URL; it reads obviously in
the lockfile.

### 2.7 Windows UX and stderr parsing {#windows-ux}

**Decision:** on Windows, every `git` subprocess is spawned with the
`CREATE_NO_WINDOW` creation flag (`0x08000000`) via
`std::os::windows::process::CommandExt::creation_flags`.

**Why.** If `vibe` ever runs inside a process without a console of its
own (a GUI launcher, IDE plugin, Windows service), a child with
`CREATE_CONSOLE` semantics would flash a separate black window. The
flag costs nothing in the console-attached case (stdio still
inherits), and covers the hypothetical hostless case for free.

**Decision:** every `git` invocation runs with `LC_ALL=C` and
`LANG=C` in the environment so error strings are stable across user
locales. We key error classification off:

- exit code (zero vs non-zero),
- stderr substrings: `fatal: repository … not found`,
  `Permission denied (publickey)`, `Could not resolve host`,
  `Repository .* is empty`, `unable to access`.

Anything unmatched is reported as a generic "git command failed"
with the raw stderr attached. Stable classification covers the
diagnoses we hand-hold the user through; the catch-all covers the
rest without hiding information.

---

## 3. Rejected alternatives {#rejected}

### 3.1 `git2` crate as the primary backend

Rejected for M1. See §2.1. The decision is reversible via
`GitBackend` (§2.2).

### 3.2 Hybrid `git2` + shell-out fallback

Cargo does this. Rejected for v1 because it doubles the surface area
(two backends under one implementation), makes error messages
conditional on which path fired, and provides zero benefit on our
target matrix. Revisit only if we ever take the `libgit2` branch and
need auth fallback to system `git`.

### 3.3 Sparse / partial clone in M1

Rejected: `vibespecs` is tiny. Optimisation is M2. The `GitBackend`
trait is narrow enough that adding a `clone_sparse` method later is a
one-line extension.

### 3.4 Hosting the registry cache under the project

Rejected: cache-per-project duplicates the same git clone across every
project on the same machine. `VIBEVM-SPEC.md` §8.3 already pins the
cache at `~/.vibe/registries/<hash>/` for this reason.

### 3.5 Vendoring `git` with the `vibe` binary

Rejected: vendoring a full git is the antithesis of "single Rust
binary". If we ever want zero runtime dependencies, the answer is
`libgit2`, not a bundled git.

---

## 4. Out of scope for M1.1 {#out-of-scope}

- Authentication for HTTPS registries with token / PAT
  (M2, PROP later).
- `vibe publish` (`VIBEVM-SPEC.md` §8.4 pins this to v2+).
- LLM-based install review (`VIBEVM-SPEC.md` §8.5, M2).
- Progress UI for long clones.
- Multiple registries per project.
- `--offline` flag.

---

## 5. Acceptance (for M1.1 implementation) {#acceptance}

Code-complete on 2026-04-22. The remaining `[ ]` item is a manual
smoke-test that cannot run in the unit / integration harness.

- [x] `vibe-registry` exposes a `Registry` trait and two
      implementations (`LocalRegistry`, `GitRegistry`).
- [x] `GitBackend` trait + `ShellGit` implementation land in
      `vibe-registry::git_backend`.
- [x] `ShellGit` preflight (`git --version`) runs once per instance
      (cached via `OnceLock`) and emits `GitError::NotInstalled` with
      an actionable message if absent.
- [x] `ShellGit::bootstrap` and `ShellGit::update` succeed against
      a bare fixture repo in an integration test.
- [x] Cache lives at `~/.vibe/registries/<hash>/{clone,meta.toml}`.
- [x] `meta.toml` gains a well-formed `last_pulled_at` after each
      fetch.
- [x] Freshness policy: ≤1h skips pull; >1h pulls; `vibe registry
      sync` always pulls (TTL=0 uses `>=` so same-second wallclock
      still triggers).
- [x] End-to-end install against a `git+file://…` registry seeded
      with the canonical `flow:wal@0.1.0` fixture succeeds; the
      lockfile records a `git+…#flow/wal/v0.1.0` source URI.
- [ ] **Manual** smoke-test against the real
      `git@gitverse.ru:anarchic/vibespecs.git` configured in
      `vibe.toml` still to be run — no automated CI against GitVerse
      yet.
- [x] `vibe registry sync` (no args) force-pulls the configured
      registry.
- [x] Windows: every spawned git carries `CREATE_NO_WINDOW`; no
      stray console windows from a hostless parent.
- [x] `cargo test --workspace` green (77 tests).
- [x] `cargo clippy --workspace --all-targets -- -D warnings` clean.

---

## 6. Open questions {#open-questions}

None blocking. Parking lot:

- Should we expose `ShellGit::git_binary: PathBuf` for users who have
  git outside `PATH`? Probably yes, via an env var (`VIBE_GIT_BINARY`)
  rather than a CLI flag, to keep the CLI surface stable. Defer to
  first user request.
- Does the registry cache need a lock file against concurrent `vibe`
  invocations? Probably yes for M2; a crash mid-clone leaves a
  half-populated `clone/`. For M1, document the behaviour ("if a
  clone fails, delete the cache dir and retry") rather than
  mechanising it.
