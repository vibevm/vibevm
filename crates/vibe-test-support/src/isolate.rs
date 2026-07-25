//! Load-time isolation of the per-user settings home.
//!
//! # Why this runs before `main`, and not from a helper
//!
//! `Command::cargo_bin` gives the child a copy of the **test process's**
//! environment. So the cheapest way to make a forgotten `UserScratch`
//! harmless is to isolate the parent: once `$VIBE_SETTINGS` and friends
//! point into a per-process temp tree, a bare `cargo_bin` child is
//! isolated too, and the discipline stops being something a test author
//! has to remember.
//!
//! That has to happen before the first `#[test]` body, and libtest gives
//! no hook — so this module registers a platform constructor
//! (`.CRT$XCU` on MSVC, `__DATA,__mod_init_func` on Mach-O,
//! `.init_array` on ELF), the same mechanism the `ctor` crate packages.
//! It is hand-rolled rather than pulled in as a dependency because it is
//! twelve lines and a test-support crate should not grow a supply chain.
//!
//! A lazy `isolate()` called from the helpers would be the obvious
//! alternative, and it is **wrong**: libtest runs test bodies on many
//! threads, and `std::env::set_var` from one thread while another reads
//! the environment is the exact undefined behaviour that made these
//! functions `unsafe` in edition 2024. A constructor runs single-threaded
//! before `main`, which is the one moment mutation is sound. That is why
//! there is no runtime entry point here — [`isolated_home`] only reads
//! back what the constructor already did.
//!
//! # Scope: linkage is the opt-in, and the opt-out
//!
//! The constructor lives in this crate's object code, so it runs in every
//! test binary that **links** this crate — a `use vibe_test_support…`
//! anywhere in the binary is enough. A test binary that deliberately needs
//! the operator's real state (`vibe-cli`'s `cli_live_e2e.rs`) simply does
//! not reference this crate, and says so at its `cargo_bin` site. Linkage
//! is therefore the whole opt-in/opt-out mechanism; there is no env
//! escape hatch to get wrong.
//!
//! That also bounds what this layer can promise: a *new* test binary that
//! references nothing from here is not isolated. Catching that one is the
//! job of `tools/user-home-tripwire.sh`, wired into `tools/self-check.sh`.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use vibe_core::settings::SETTINGS_DIR_ENV;

/// `vibe_registry::registry_cache`'s override for the git clone cache.
/// Spelled out rather than imported: `vibe-registry` is not a dependency
/// of this crate, and adding one to reach a `&str` would drag the whole
/// registry stack into every test binary.
pub const REGISTRY_CACHE_ENV: &str = "VIBE_REGISTRY_CACHE";

/// `vibe_registry::search::cache::CACHE_ROOT_ENV` — the `vibe search`
/// result-cache override. Spelled out for the same reason.
pub const SEARCH_CACHE_ENV: &str = "VIBEVM_SEARCH_CACHE_DIR";

/// Parent of every per-process home, under the system temp dir. Named so
/// a human who finds it knows what it is and that deleting it is safe.
const HOMES_DIR: &str = "vibevm-test-homes";

/// How long a per-process home may sit before a later run reclaims it.
/// Far longer than any test binary lives, so pruning can never race a
/// live run; short enough that the directory does not accumulate.
const STALE_AFTER: Duration = Duration::from_secs(6 * 60 * 60);

/// The per-process settings home this binary was pointed at, or `None`
/// when the constructor did not run (which means this binary is **not**
/// isolated — a thing worth being able to assert).
static ISOLATED_HOME: OnceLock<PathBuf> = OnceLock::new();

/// The temp settings home this test process was redirected to.
///
/// `None` means the load-time constructor did not run — the binary is
/// reading the operator's real `~/.vibe`. Tests that care about the
/// isolation itself assert on this.
pub fn isolated_home() -> Option<&'static Path> {
    ISOLATED_HOME.get().map(PathBuf::as_path)
}

/// The platform constructor slot. `#[used]` keeps the symbol through
/// dead-code elimination; the section places it in the pre-`main`
/// initialiser table the platform's startup code walks.
#[used]
#[cfg_attr(windows, unsafe(link_section = ".CRT$XCU"))]
#[cfg_attr(
    target_vendor = "apple",
    unsafe(link_section = "__DATA,__mod_init_func")
)]
#[cfg_attr(
    all(unix, not(target_vendor = "apple")),
    unsafe(link_section = ".init_array")
)]
static ISOLATE_BEFORE_MAIN: extern "C" fn() = isolate_this_process;

/// Point the three per-user state roots at a fresh temp tree.
///
/// Runs before `main`, so it must not panic: a panic here unwinds through
/// platform startup code, before the Rust runtime is initialised, and the
/// process dies with no usable diagnostic. Every fallible step is
/// therefore best-effort — a failure to create the tree still leaves the
/// variables pointed away from the real home, which is the part that
/// matters.
extern "C" fn isolate_this_process() {
    let root = per_process_home();
    let settings = root.join("settings");
    let registries = root.join("registries");
    let search_cache = root.join("search-cache");

    let _ = std::fs::create_dir_all(&settings);
    let _ = std::fs::create_dir_all(&registries);
    let _ = std::fs::create_dir_all(&search_cache);

    // SAFETY: this runs from the platform's pre-`main` initialiser table,
    // where the process is still single-threaded — no other thread can be
    // in `getenv` concurrently, which is the unsoundness edition 2024
    // marks these calls for. Nothing in this crate mutates the
    // environment at any later point, by construction: there is no
    // runtime entry point that reaches these calls.
    unsafe {
        std::env::set_var(SETTINGS_DIR_ENV, &settings);
        std::env::set_var(REGISTRY_CACHE_ENV, &registries);
        std::env::set_var(SEARCH_CACHE_ENV, &search_cache);
    }

    let _ = ISOLATED_HOME.set(settings);
}

/// `<temp>/vibevm-test-homes/p<pid>-<nanos>` — unique per run, so a
/// recycled pid never inherits a previous run's state.
fn per_process_home() -> PathBuf {
    let base = std::env::temp_dir().join(HOMES_DIR);
    let _ = std::fs::create_dir_all(&base);
    prune_stale(&base);
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    base.join(format!("p{}-{}", std::process::id(), stamp))
}

/// Drop homes left behind by earlier runs. Nothing here owns a
/// `TempDir` — a constructor has no drop point — so the litter is
/// collected on the way in instead. Entirely best-effort.
fn prune_stale(base: &Path) {
    let Some(cutoff) = SystemTime::now().checked_sub(STALE_AFTER) else {
        return;
    };
    let Ok(entries) = std::fs::read_dir(base) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(meta) = entry.metadata() else { continue };
        if !meta.is_dir() {
            continue;
        }
        let Ok(modified) = meta.modified() else {
            continue;
        };
        if modified < cutoff {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The constructor is the one load-bearing, silently-breakable part of
    /// layer 1: a toolchain or linker change that stops honouring `#[used]`
    /// in a section would drop it, and every test in the workspace would go
    /// back to reading the operator's real `~/.vibe` with nothing failing.
    /// This unit test lives in the crate that carries the constructor, so it
    /// always links it, and it fails loudly the moment the mechanism dies.
    #[test]
    fn the_constructor_ran_before_this_test_body() {
        let home = isolated_home().expect(
            "the pre-`main` constructor did not run — this test process is NOT isolated \
             from the real `~/.vibe`, and neither is any other test binary",
        );
        assert_eq!(
            std::env::var_os(SETTINGS_DIR_ENV)
                .map(PathBuf::from)
                .as_deref(),
            Some(home),
            "$VIBE_SETTINGS does not point at the per-process home"
        );
        for key in [REGISTRY_CACHE_ENV, SEARCH_CACHE_ENV] {
            let value = std::env::var_os(key).map(PathBuf::from);
            let value = value.unwrap_or_else(|| panic!("${key} unset"));
            assert!(
                value.starts_with(std::env::temp_dir().join(HOMES_DIR)),
                "${key} = {} escapes the per-process home",
                value.display()
            );
        }
    }
}
