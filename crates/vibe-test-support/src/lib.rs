//! Test-only isolation of the per-user vibevm settings home.
//!
//! The developer's real `~/.vibe` holds publish tokens and API keys next
//! to `config.toml`, `settings.toml` and `registry.toml`. A test that
//! reads it asserts against one machine; a test that writes it corrupts
//! the operator's state. Three findings in a row (F-055, F-056, F-057)
//! were that same forgotten discipline, each caught by accident.
//!
//! This crate makes the safe path the default rather than a convention,
//! in two moves:
//!
//! 1. Linking it installs a load-time constructor that points
//!    `$VIBE_SETTINGS`, `$VIBE_REGISTRY_CACHE` and
//!    `$VIBEVM_SEARCH_CACHE_DIR` at a per-process temp tree **before the
//!    first `#[test]` body runs**. `Command::cargo_bin` hands the child a
//!    copy of the test process's environment, so a bare `cargo_bin` call
//!    in an isolated binary is isolated too — see [`isolate`] for why
//!    that has to be a constructor and not a helper.
//! 2. [`UserScratch`] remains for the tests that need to *name* the
//!    scratch paths — seed a fixture `registry.toml`, read a clone-cache
//!    bucket layout back — rather than merely be kept away from the real
//!    home.
//!
//! Neither layer is the guarantee. `tools/user-home-tripwire.sh`, wired
//! into `tools/self-check.sh`, hashes the real settings home before and
//! after the suite and fails the floor if anything moved. This crate is
//! the ergonomics that keep the tripwire from ever firing.

mod isolate;

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;

pub use isolate::{REGISTRY_CACHE_ENV, SEARCH_CACHE_ENV, isolated_home};

/// A `Command` for a workspace binary, in a process the constructor has
/// already isolated.
///
/// Referencing this function is what links the constructor in, so a test
/// that calls it gets isolation whether or not it goes on to build a
/// [`UserScratch`]. Prefer it over `assert_cmd::Command::cargo_bin` at
/// every call site in `crates/*/tests/**`.
pub fn cargo_bin(name: &str) -> Command {
    Command::cargo_bin(name).unwrap_or_else(|e| panic!("`{name}` binary built: {e}"))
}

/// A `vibe` command with global-registry seeding suppressed.
///
/// Suppressing the seeding keeps a machine that has no `registry.toml`
/// from gaining one, and keeps real-world registries out of the
/// resolution/cache shape a test asserts against.
///
/// NOTE: this only stops the CLI from *writing* a fresh `registry.toml`.
/// It does nothing about one that already exists — for that, build
/// commands through [`UserScratch::vibe`], or rely on the constructor,
/// which has already moved the settings home this process points at.
pub fn vibe() -> Command {
    let mut cmd = cargo_bin("vibe");
    cmd.env("VIBE_NO_DEFAULT_REGISTRY", "1");
    cmd
}

/// A scratch stand-in for the per-user state a `vibe` subprocess reads out
/// of the developer's home directory. One per test: it owns its temp tree,
/// so it dies with the test and no two tests ever share one.
///
/// The load-time constructor already keeps this process away from the real
/// home. `UserScratch` is for the stronger need: a test that has to *name*
/// the scratch paths — seed a fixture into them, or read the bucket layout
/// back — and a test that wants a home no sibling test in the same binary
/// can observe.
///
/// # All three environment variables are load-bearing — do not delete any
///
/// * **`VIBE_SETTINGS`** relocates the whole per-user settings dir
///   (`~/.vibe`) verbatim, and the file that matters is `registry.toml`:
///   its `[[registry]]` entries are *merged* into every resolution
///   (PROP-002 §2.2.2 `#global-config`, via `merge_effective`). A developer
///   who has real global registries therefore resolves against more
///   registries — and mints more clone-cache buckets — than the test ever
///   declared, so a hermetic assertion goes red on their machine and
///   nowhere else. That was F-055: `assert_eq!(clone_dirs.len(), 1)` in
///   `cli_pkg_cycle.rs` saw three buckets the moment a real
///   `~/.vibe/registry.toml` appeared. The same dir also carries
///   `config.toml` (`[init] last_author`) and `settings.toml`, so `vibe
///   init` stops picking up the developer's name too. That was F-056:
///   every `vibe init` in `cli_init.rs` resolved its author out of the
///   developer's real `config.toml` (`prompts.rs` `--author` →
///   `init.last_author` → `detect_git_author()`), and on a machine where
///   `last_author` is *unset* while `git config user.name` is set — a
///   fresh checkout, or CI — the run wrote the detected name straight
///   back into it.
/// * **`VIBE_REGISTRY_CACHE`** pins the git clone cache at a path the test
///   knows by name so it can read the bucket layout back. `VIBE_SETTINGS`
///   alone would already move the cache (to `<settings>/registries`), but
///   only this makes the location the test's to assert on.
/// * **`VIBEVM_SEARCH_CACHE_DIR`** does the same for the `vibe search`
///   result cache. `VIBE_SETTINGS` alone likewise already moves it (to
///   `<settings>/search-cache`, via `vibe_core::settings::search_cache_dir`),
///   but the cache-hit/TTL tests read the bucket layout back by name, so
///   the location has to be the test's. That was F-057: four bare
///   `vibe()` sites in `cli_search.rs` cached into the developer's real
///   `~/.vibe/search-cache/`, where one test run's entries outlive the
///   test and seed the next one.
///
/// Never point any of them at the real home: a test that *reads* real
/// user state is as broken as one that writes it.
pub struct UserScratch {
    /// Owns the temp tree; dropping it removes `settings`, `cache` and
    /// `search_cache`.
    _root: tempfile::TempDir,
    /// `$VIBE_SETTINGS` — stands in for `~/.vibe`. Starts empty; a test
    /// that deliberately exercises global settings seeds a fixture here.
    pub settings: PathBuf,
    /// `$VIBE_REGISTRY_CACHE` — the per-package git clone cache root.
    pub cache: PathBuf,
    /// `$VIBEVM_SEARCH_CACHE_DIR` — the `vibe search` result cache root.
    pub search_cache: PathBuf,
}

impl UserScratch {
    /// A fresh, empty per-user scratch home.
    pub fn new() -> Self {
        let root = tempfile::tempdir().expect("scratch user home");
        let settings = root.path().join("settings");
        let cache = root.path().join("cache");
        let search_cache = root.path().join("search-cache");
        fs::create_dir_all(&settings).expect("scratch settings dir");
        fs::create_dir_all(&cache).expect("scratch cache dir");
        fs::create_dir_all(&search_cache).expect("scratch search-cache dir");
        Self {
            _root: root,
            settings,
            cache,
            search_cache,
        }
    }

    /// A `vibe` command that reads this scratch instead of the developer's
    /// home. Every `Command` a resolution/cache/registry test builds must
    /// come from here rather than from the bare [`vibe`].
    pub fn vibe(&self) -> Command {
        let mut cmd = vibe();
        cmd.env(vibe_core::settings::SETTINGS_DIR_ENV, &self.settings);
        cmd.env(REGISTRY_CACHE_ENV, &self.cache);
        cmd.env(SEARCH_CACHE_ENV, &self.search_cache);
        cmd
    }

    /// `vibe init` under this scratch — the isolated counterpart of a
    /// bare `vibe init`.
    pub fn init_project(&self, dir: &Path) {
        self.vibe()
            .arg("init")
            .arg("--path")
            .arg(dir)
            .assert()
            .success();
    }
}

impl Default for UserScratch {
    fn default() -> Self {
        Self::new()
    }
}

/// One real lifecycle lease for fixtures that must carry the proof as a VALUE
/// (an `InstallRunContext` built by hand).
///
/// Acquired ONCE per test process; these fixtures never dispatch through it,
/// so sharing one acquisition across them is exactly the `Arc` proof the
/// production channel carries. The owner RETAINS its `TempDir` (a plain
/// `keep()` leak would orphan the directory even when nothing else wants it).
/// Statics are never dropped in today's Rust, so process-exit cleanup is not
/// delivered — the temp filesystem's own sweeper is the backstop — but the
/// ownership is honest: the day static destructors run, this directory is
/// cleaned with the rest.
pub fn retained_lifecycle_lease() -> std::sync::Arc<vibe_lifecycle::LifecycleLease> {
    /// The retained owner: the lease AND the directory it was taken over, so
    /// both live (and, if statics ever drop, die) together.
    struct LeaseOwner {
        lease: std::sync::Arc<vibe_lifecycle::LifecycleLease>,
        _dir: tempfile::TempDir,
    }
    static OWNER: std::sync::OnceLock<LeaseOwner> = std::sync::OnceLock::new();
    OWNER
        .get_or_init(|| {
            let dir = tempfile::tempdir().expect("a temp root for the test lease");
            let lease = vibe_lifecycle::LifecycleLease::acquire(dir.path())
                .expect("the retained test root is leasable");
            LeaseOwner {
                lease: std::sync::Arc::new(lease),
                _dir: dir,
            }
        })
        .lease
        .clone()
}
