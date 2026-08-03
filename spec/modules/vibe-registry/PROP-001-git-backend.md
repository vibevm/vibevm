# PROP-001: Git-backed registry for `vibe-registry` {#root}

<status stage="impl" state="done" comment="B0 2026-07-24: accepted and shipped 2026-04-22; partially superseded by PROP-002"/>

##milestone-line **Milestone:** M1.1 ([`ROADMAP.md`](../../../ROADMAP.md#m11--git-backed-registry)). @impl/done

##status-line **Status:** accepted 2026-04-22, shipped 2026-04-22. **Partially superseded by [PROP-002](PROP-002-decentralized-registry.md) (2026-04-24).** See the "Superseded parts" block below. @impl/done

##supersedes-line **Supersedes:** nothing. @spec/done

##related **Related:** [spec://org.vibevm.core/vibevm/common/PROP-000#registry](../../common/PROP-000.md#registry), [`VIBEVM-SPEC.md` §8](../../../VIBEVM-SPEC.md), [PROP-002](PROP-002-decentralized-registry.md). @spec/done

## Superseded parts (by PROP-002)

##superseded-lead The following decisions in this PROP were revised by [PROP-002](PROP-002-decentralized-registry.md) when the registry model moved from monorepo-as-registry to decentralized per-package repos. Use PROP-002 as the authoritative source for these: @spec/done

- ##SUP-REGISTRY-TRAIT **§2.3 `Registry` trait** — the single-registry trait is extended by a `MultiRegistryResolver` coordinating several `[[registry]]` entries, each wrapped as a `GitPackageRegistry`. The monorepo-era `GitRegistry` is retired. @spec/done
- ##SUP-CACHE-LAYOUT **§2.4 Cache layout** — `~/.vibe/registries/<hash>/clone/` (one clone per registry URL) is replaced by `~/.vibe/registries/<canonical-url-hash>/packages/<kind>-<name>/{clone,meta.toml}` (one clone per package). @spec/done
- ##SUP-SOURCE-URI **§2.6 Lockfile `source_uri` format** — `git+<transport>://<host>/<path>.git#<kind>/<name>/v<ver>` (path-in-monorepo) is replaced by full lockfile fields: `registry`, `source_url`, `source_ref`, `resolved_commit`, `content_hash`; `#fragment` is no longer used. @spec/done

##NOT-SUPERSEDED What is **not** superseded (and remains authoritative here): §2.1 (shell-out-to-git backend choice), §2.2 (`GitBackend` trait), §2.5 (1-hour freshness TTL), §2.7 (Windows UX and stderr classification). @spec/done

- ##ARG-PRUNED Additionally, the size-footprint argument in §2.1 is pruned by [PROP-000 §15](../../common/PROP-000.md#dep-weight) (dependency weight is not a decision factor). @spec/done
- ##remaining-args The remaining arguments against `git2` (Windows SSH-auth lottery, diagnostic clarity of shell-out error messages) still carry the decision for M1 — but the argument tree is narrower now. @spec/done
- ##revisit-narrowed Revisit when a concrete reason arises, e.g. programmatic object reads that shell-out can't do cheaply. @spec/done

---

## 1. Motivation {#motivation}

##m0-state M0 shipped with a local-directory registry only. @impl/done

##M1-GOAL M1 makes the registry a git
repository hosted on GitVerse (default
`git@gitverse.ru:anarchic/vibespecs.git`). The implementation must: @impl/done

- ##REQ-CLONE-PULL Clone the registry into `~/.vibe/registries/<hash>/` on first use and
  `git pull` on subsequent use (`VIBEVM-SPEC.md` §8.3). @impl/done
- ##REQ-PRESERVE-LOCAL Preserve the existing `LocalRegistry` code path so tests and the
  `--registry <path>` override keep working. @impl/done
- ##REQ-SSH-AUTH Authenticate against GitVerse using the SSH identity the user has
  already configured (see [`spec/boot/90-user.md`](../../boot/90-user.md)). @impl/done
- ##REQ-CROSS-PLATFORM Run on Windows, macOS, and Linux with no per-platform build hoops. @impl/done
- ##REQ-FOOTPRINT Carry its operational weight on constrained dev machines without
  bloating the `vibe` binary or adding a C toolchain requirement. @impl/done

##mechanics-in-readme This PROP records the architectural decisions. The mechanics
(`Registry` trait surface, error variants, wire-level command lines)
live in the crate's module documentation — `lib.rs` doc comments and the error
strings that cite `spec://` anchors — not in a README; `crates/vibe-registry`
has none. @impl/done

---

## 2. Decisions {#decisions}

### 2.1 Backend: shell out to `git`, not `git2` {#backend}

##SHELL-OUT **Decision:** `vibe-registry` performs all git operations by spawning the
system `git` binary via `std::process::Command`. We do **not** link
against `libgit2` (via the `git2` crate) in v1. @impl/done

##backend-why-lead **Why:** @impl/done

1. ##WHY-SSH-WINDOWS **SSH on Windows is the killer.** GitVerse authenticates via SSH.
   Git for Windows ships OpenSSH and a working `ssh-agent`; the user's
   identity (`olegchir@UNIT-2040`) is already loaded and the push to
   `gitverse.ru` is proven (see [`spec/boot/90-user.md`](../../boot/90-user.md)).
   `libgit2` uses `libssh2` for SSH, which talks to `ssh-agent`
   through a named-pipe protocol that is fragile on Windows and
   routinely requires `SSH_AUTH_SOCK` juggling or explicit key paths.
   Cargo itself falls back to the system `git` on auth failure for
   this exact reason. Shell-out makes that fallback the primary path
   and retires the class of bug. @impl/done

2. ##WHY-FOOTPRINT **Dependency footprint.** `git2` pulls `libgit2-sys`,
   `libssh2-sys`, `libz-sys`, and `openssl-sys` (or a vendored
   alternative). Non-vendored builds demand a working C toolchain on
   every developer and CI machine; vendored builds add 3–8 MB to the
   release binary. Shell-out adds zero bytes and zero build-time
   native dependencies. @impl/done

3. ##WHY-DEBUGGABILITY **Feature parity and debuggability.** The user's `git` is by
   definition current. Errors surface with the full native message;
   `tracing` logs the exact argv so a user can re-run the failing
   command by hand. `libgit2`'s error strings (`ERROR class=Net (12):
   unexpected http status code: 401`) are harder to diagnose. @impl/done

4. ##WHY-NO-PROGRAMMATIC **We do not need programmatic git.** The v1 operations are
   `git clone`, `git fetch`, `git pull --ff-only`, and `git
   --version` for preflight. No partial clone, no in-memory object
   reads, no custom refspecs, no progress UI. Shell-out handles this
   trivially. @impl/done

5. ##WHY-LICENSING **Licensing.** `git` is GPL v2, but shell-out is `exec` not
   linkage — the GNU FAQ explicitly separates these. Our binary stays
   unambiguously permissive.
   `libgit2` is GPL v2 with a Linking Exception (permissive for our
   purposes), but shell-out leaves the entire conversation at the
   door. @impl/done

##risks-lead **Risks accepted:** @impl/done

- ##RISK-GIT-IN-PATH **Runtime dependency on `git` in `PATH`.** Acceptable: our target
  audience is developers who already have git installed. We perform a
  preflight `git --version` check and emit an actionable error (with
  a pointer to `https://git-scm.com/downloads`) if it is missing. @impl/done
- ##RISK-STDERR-PARSING **stderr parsing for fine-grained error classification.** We
  mitigate by running git with `LC_ALL=C` and keying off exit
  code + substring markers (`fatal: ` prefix, `Permission denied
  (publickey)`, `Repository not found`). See §2.7. @impl/done

##revisit-lead **When to revisit:** if and when we need one of: @spec/done
- ##revisit-sparse partial/sparse clone with custom filters, @spec/done
- ##revisit-object-reads programmatic object reads (e.g. to fetch a `latest` marker file
  without a working-tree checkout), @spec/done
- ##revisit-credential-store OS-credential-store integration that can't be delegated to `git`, @spec/done
- ##revisit-bundling running on a platform where bundling `git` is easier than requiring
  it. @spec/done

##REVISIT-PATH At that point, add a `libgit2` feature behind the `GitBackend` trait
(§2.2). The trait is designed so the switch costs one `impl` block
and one line in the factory, and nothing else in the codebase moves. @spec/done

### 2.2 `GitBackend` trait {#backend-trait}

##GITBACKEND-TRAIT **Decision:** `vibe-registry::git_backend::GitBackend` is the single
interface through which the registry layer touches git. It has exactly
the operations we use: @impl/done

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

##METHOD-NAME-NOTE **Method-name note.** The "make a fresh clone" operation is called
`bootstrap` rather than the obvious `clone` or `clone_into` because
the backend is held as `Arc<dyn GitBackend>` at its call sites and
both of those names collide with blanket-impl methods from the
standard library (`std::clone::Clone::clone`,
`std::borrow::ToOwned::clone_into`), forcing ugly `<T as
GitBackend>::…` disambiguations at every call. `bootstrap` is
semantically accurate — it's how we initialise the registry cache
from empty state — and has no std-library namesake. @impl/done

##WHY-NARROW **Why narrow.** The narrower the trait, the cheaper the backend swap.
If we need `ls_remote` or `fetch_ref` later, we add a method — that
addition is a visible, deliberate change, not a quiet interface drift. @impl/done

##implementations-lead **Implementations:** @impl/done

- ##IMPL-SHELLGIT `ShellGit` — default, built from `std::process::Command`. See §2.7. @impl/done
- ##IMPL-LIBGIT2-RESERVED `LibGit2` — reserved. Not implemented in M1; the trait is the entry
  point for a future feature-gated addition. @spec/done

##NO-MOCK The `vibe-registry` crate does not expose a mock implementation.
Tests use `ShellGit` against a bare git repository created in a
`tempdir` — exercising the production code path end-to-end. @impl/done

### 2.3 `Registry` trait {#registry-trait}

##REGISTRY-TRAIT **Decision:** introduce a `vibe-registry::Registry` trait that both
`LocalRegistry` and `GitRegistry` implement: @spec/done

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

##CONSUMERS-UNCHANGED `vibe-install` and `vibe-cli` continue to consume `ResolvedPackage` /
`CachedPackage` exactly as in M0; the only change is that the concrete
type is chosen at CLI-arg-parse time. @spec/done

##SELECTION-RULE **Selection rule.** CLI precedence stays as defined in `VIBEVM-SPEC.md`
§9.1: `--registry <path>` (explicit, always a local directory) wins
over the `[registry]` section in `vibe.toml` (a URL — git or
`file://`). @impl/done

### 2.4 Cache layout {#cache-layout}

##CACHE-LAYOUT **Decision:** the on-disk layout under `~/.vibe/registries/` is: @spec/done

```
~/.vibe/registries/
└── <hash>/
    ├── clone/        ← the git working tree
    └── meta.toml     ← { url, ref, last_pulled_at }
```

- ##LAYOUT-HASH `<hash>` = lowercase hex of the first 16 bytes of
  `sha256(normalized_url)`. 16 hex chars is enough to avoid realistic
  collisions while keeping the directory name tab-completable (same
  trick Cargo uses for its git cache). The full hash lives in
  `meta.toml` for audit. @spec/done
- ##LAYOUT-NORMALIZED-URL `normalized_url` strips a trailing `.git` and lowercases the
  scheme + host so `git@gitverse.ru:anarchic/vibespecs.git` and
  `ssh://git@gitverse.ru/anarchic/vibespecs` hash to the same
  registry. @spec/done
- ##LAYOUT-META `meta.toml` is written after each successful clone or update. It
  carries the url (for debugging), the ref, and the UTC RFC3339
  timestamp of the last successful fetch. @spec/done
- ##LAYOUT-CLONE-DELEGATE The `clone/` subdirectory is the registry working tree. `GitRegistry`
  internally wraps a `LocalRegistry::new(clone_dir)` and delegates
  `resolve` / `list_versions` / `fetch` to it — the packaged layout
  (`<kind>/<name>/v<ver>/…`) is identical in both worlds. @spec/done

##PER-PROJECT-CACHE-UNCHANGED Per-project package cache (`<project>/.vibe/cache/<kind>/<name>/<ver>/`)
is unchanged from M0. @spec/done

### 2.5 Freshness policy {#freshness}

##FRESHNESS-TTL **Decision:** the default freshness TTL is **1 hour**, checked against
`meta.toml.last_pulled_at`. An install whose registry cache is older
than the TTL triggers an implicit `update`. An install whose cache is
younger skips the pull. `vibe registry sync` forces an update
regardless of age. @impl/done

##ttl-why **Why 1 hour:** short enough to pick up new package versions within
one working session, long enough to amortise network round-trips over
a burst of installs. Revisit once real usage arrives. @spec/done

##NO-OFFLINE-YET **Superseded — `--offline` shipped.** This recorded the M1 state: a
network failure during an implicit update failed the install with a clear
message. Offline resolution landed with [PROP-002 §2.2.2.1](PROP-002-decentralized-registry.md)
(`url_is_local`) and the PROP-030 flag, and is live in `vibe install --help`. @impl/done

### 2.6 Lockfile `source_uri` format {#source-uri}

##SOURCE-URI-FORMAT **Decision:** when a package originates from a git registry, the
lockfile records its source as @spec/done

```
git+ssh://git@gitverse.ru/anarchic/vibespecs.git#<kind>/<name>/v<ver>
```

##fragment-note The `#fragment` names the package directory inside the registry
relative to the registry root. The scheme prefix (`git+ssh` /
`git+https` / `git+file`) encodes the transport. Local-directory
registries continue to produce `file://…` URIs as in M0. @spec/done

##scheme-prefix-why **Why a scheme prefix.** `pip` and Cargo both use `git+…` prefixes to
disambiguate a git source from a plain URL; it reads obviously in
the lockfile. @spec/done

### 2.7 Windows UX and stderr parsing {#windows-ux}

##NO-WINDOW-FLAG **Decision:** on Windows, every `git` subprocess is spawned with the
`CREATE_NO_WINDOW` creation flag (`0x08000000`) via
`std::os::windows::process::CommandExt::creation_flags`. @impl/done

##no-window-why **Why.** If `vibe` ever runs inside a process without a console of its
own (a GUI launcher, IDE plugin, Windows service), a child with
`CREATE_CONSOLE` semantics would flash a separate black window. The
flag costs nothing in the console-attached case (stdio still
inherits), and covers the hypothetical hostless case for free. @impl/done

##LOCALE-STDERR **Decision:** every `git` invocation runs with `LC_ALL=C` and
`LANG=C` in the environment so error strings are stable across user
locales. We key error classification off: @impl/done

- ##CLASS-EXIT-CODE exit code (zero vs non-zero), @impl/done
- ##CLASS-STDERR-SUBSTRINGS stderr substrings: `fatal: repository … not found`,
  `Permission denied (publickey)`, `Could not resolve host`,
  `Repository .* is empty`, `unable to access`. @impl/done

##CATCH-ALL Anything unmatched is reported as a generic "git command failed"
with the raw stderr attached. Stable classification covers the
diagnoses we hand-hold the user through; the catch-all covers the
rest without hiding information. @impl/done

---

## 3. Rejected alternatives {#rejected}

### 3.1 `git2` crate as the primary backend

##REJ-GIT2 Rejected for M1. See §2.1. The decision is reversible via
`GitBackend` (§2.2). @spec/done

### 3.2 Hybrid `git2` + shell-out fallback

##REJ-HYBRID Cargo does this. Rejected for v1 because it doubles the surface area
(two backends under one implementation), makes error messages
conditional on which path fired, and provides zero benefit on our
target matrix. Revisit only if we ever take the `libgit2` branch and
need auth fallback to system `git`. @spec/done

### 3.3 Sparse / partial clone in M1

##REJ-SPARSE Rejected: `vibespecs` is tiny. Optimisation is M2. The `GitBackend`
trait is narrow enough that adding a `clone_sparse` method later is a
one-line extension. @spec/done

### 3.4 Hosting the registry cache under the project

##REJ-PROJECT-CACHE Rejected: cache-per-project duplicates the same git clone across every
project on the same machine. `VIBEVM-SPEC.md` §8.3 already pins the
cache at `~/.vibe/registries/<hash>/` for this reason. @spec/done

### 3.5 Vendoring `git` with the `vibe` binary

##REJ-VENDORING Rejected: vendoring a full git is the antithesis of "single Rust
binary". If we ever want zero runtime dependencies, the answer is
`libgit2`, not a bundled git. @spec/done

---

## 4. Out of scope for M1.1 {#out-of-scope}

- ##OOS-HTTPS-AUTH Authentication for HTTPS registries with token / PAT
  (M2, PROP later). @spec/done
- ##OOS-PUBLISH `vibe publish` (`VIBEVM-SPEC.md` §8.4 pins this to v2+). @spec/done
- ##OOS-LLM-REVIEW LLM-based install review (`VIBEVM-SPEC.md` §8.5, M2). @spec/done
- ##OOS-PROGRESS-UI Progress UI for long clones. @spec/done
- ##OOS-MULTI-REGISTRY Multiple registries per project. @spec/done
- ##OOS-OFFLINE `--offline` flag. @spec/done

---

## 5. Acceptance (for M1.1 implementation) {#acceptance}

##acceptance-lead Code-complete and live on 2026-04-22. Every box below ticks; the
milestone is shippable. @impl/done

- ##ACC-REGISTRY-TRAIT [x] `vibe-registry` exposes a `Registry` trait and two
  implementations (`LocalRegistry`, `GitRegistry`). @impl/done
- ##ACC-GITBACKEND [x] `GitBackend` trait + `ShellGit` implementation land in
  `vibe-registry::git_backend`. @impl/done
- ##ACC-PREFLIGHT [x] `ShellGit` preflight (`git --version`) runs once per instance
  (cached via `OnceLock`) and emits `GitError::NotInstalled` with
  an actionable message if absent. @impl/done
- ##ACC-BOOTSTRAP-UPDATE [x] `ShellGit::bootstrap` and `ShellGit::update` succeed against
  a bare fixture repo in an integration test. @impl/done
- ##ACC-CACHE-PATH [x] Cache lives at `~/.vibe/registries/<hash>/{clone,meta.toml}`. @impl/done
- ##ACC-META-TIMESTAMP [x] `meta.toml` gains a well-formed `last_pulled_at` after each
  fetch. @impl/done
- ##ACC-FRESHNESS [x] Freshness policy: ≤1h skips pull; >1h pulls; `vibe registry
  sync` always pulls (TTL=0 uses `>=` so same-second wallclock
  still triggers). @impl/done
- ##ACC-E2E-INSTALL [x] End-to-end install against a `git+file://…` registry seeded
  with the canonical `flow:wal@0.1.0` fixture succeeds; the
  lockfile records a `git+…#flow/wal/v0.1.0` source URI. @impl/done
- ##ACC-MANUAL-SMOKE [x] Manual smoke-test against the real
  `git@gitverse.ru:anarchic/vibespecs.git` (commit `98e51fc`)
  ran 2026-04-22 on Windows / Git Bash; every step matched the
  expected output, including the
  `git+ssh://git@gitverse.ru/anarchic/vibespecs.git#flow/wal/v0.1.0`
  lockfile source URI. Procedure and last-pass metadata live in
  [`manual-tests/M1.1-git-registry-smoke.md`](../../../manual-tests/M1.1-git-registry-smoke.md). @impl/done
- ##ACC-SYNC-FORCE [x] `vibe registry sync` (no args) force-pulls the configured
  registry. @impl/done
- ##ACC-NO-WINDOW [x] Windows: every spawned git carries `CREATE_NO_WINDOW`; no
  stray console windows from a hostless parent. @impl/done
- ##ACC-TESTS-GREEN [x] `cargo test --workspace` green (77 tests). @impl/done
- ##ACC-CLIPPY [x] `cargo clippy --workspace --all-targets -- -D warnings` clean. @impl/done

---

## 6. Open questions {#open-questions}

##parking-lot-lead None blocking. Parking lot: @spec/work

- ##OPEN-GIT-BINARY-PATH **Resolved — shipped as proposed.** The `VIBE_GIT_BINARY`
  PATH override lives in `git_backend/shell.rs` and its comment cites §6 of this
  PROP; the env-var form was chosen over a CLI flag exactly to keep the CLI
  surface stable. @impl/done
- ##OPEN-CACHE-LOCK Does the registry cache need a lock file against concurrent `vibe`
  invocations? Probably yes for M2; a crash mid-clone leaves a
  half-populated `clone/`. For M1, document the behaviour ("if a
  clone fails, delete the cache dir and retry") rather than
  mechanising it. @spec/work
