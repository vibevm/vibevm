//! Where the user-config file is, and the one notice about where it no
//! longer is.
//!
//! Split out of `user_config.rs`: the parsing and layering of the file is
//! that module's subject, and *which path is the file at all* is this
//! one's. The split matters because this is the module whose single-entry
//! candidate list is the property `$VIBE_SETTINGS` isolation rests on.

use std::path::{Path, PathBuf};
use std::sync::Once;

/// The `VIBEVM_USER_CONFIG` override: an explicit **file**, not a home.
///
/// It is the first of the user-config layer's two legs, and it is read
/// here so that every ambient read of this layer is in one file.
pub(super) fn override_path() -> Option<PathBuf> {
    std::env::var_os("VIBEVM_USER_CONFIG").map(PathBuf::from)
}

/// The ordered on-disk user-config candidates: exactly one,
/// `<settings-dir>/config.toml` in the canonical settings dir (`~/.vibe`,
/// or `$VIBE_SETTINGS`).
///
/// This is the entire disk surface of the user-config layer and its single
/// authority. [`UserConfig::default_path`] resolves its non-override answer
/// from here, and `settings`'s
/// `every_accessor_is_rooted_in_the_one_settings_dir` walks the same list —
/// so the two cannot drift, and a candidate added here that is not rooted
/// in the settings dir goes red in a gate rather than surviving until a
/// campaign goes looking for it.
///
/// It stays a list rather than collapsing into a bare path because a second
/// entry is the exact defect removed on 2026-07-26. One directory means
/// `$VIBE_SETTINGS` relocates every candidate at once, so an isolated run
/// cannot reach the operator's real config — the property a second,
/// separately-rooted home silently took away.
pub(crate) fn config_file_candidates() -> Vec<PathBuf> {
    crate::settings::user_config_path().into_iter().collect()
}

/// Guards [`warn_once_about_a_left_behind_config`]. `default_path` runs on
/// every invocation and more than once in some (`vibe show config` reports
/// the path and then loads it), so an unguarded diagnostic would repeat
/// per call. Once per process, whatever the call pattern.
static LEFT_BEHIND_NOTICE: Once = Once::new();

/// Emit [`left_behind_notice`] on stderr, at most once per process.
pub(super) fn warn_once_about_a_left_behind_config(canonical: Option<&Path>) {
    LEFT_BEHIND_NOTICE.call_once(|| {
        if let Some(line) = left_behind_notice(canonical, former_config_path().as_deref()) {
            eprintln!("{line}");
        }
    });
}

/// The one-line notice for a config left at the pre-consolidation
/// location. `Some(line)` only when no canonical config exists *and* a file
/// does sit at the old one — i.e. exactly when removing that read stopped a
/// file that used to be read from being read. Switching silently is the
/// failure this prevents; it is the one thing the 2026-07-26 change added
/// rather than removed, so it is one line and no more.
///
/// Pure, and it takes both paths instead of resolving them, so the
/// message's shape is assertable without an environment.
///
/// What it does **not** do is the point. It does not read the file, does
/// not copy it, does not move it, and prints no byte of its contents: a
/// `[env]` table is a plausible place for someone to have parked a
/// credential, so the rule `vibe_cli::promote_user_config_env` follows for
/// refused names holds here too — once, by path, never the contents.
pub(super) fn left_behind_notice(
    canonical: Option<&Path>,
    former: Option<&Path>,
) -> Option<String> {
    let (canonical, former) = (canonical?, former?);
    if canonical.exists() || !former.is_file() {
        return None;
    }
    Some(format!(
        "vibe: warning: `{}` is no longer read; move it to `{}` for it to take effect",
        former.display(),
        canonical.display(),
    ))
}

/// The pre-consolidation user-config location: `%APPDATA%\vibe\config.toml`
/// on Windows, else `<home>/.config/vibe/config.toml`.
///
/// **Not a resolution leg.** Nothing here reads this path, nothing writes
/// it, and [`UserConfig::default_path`] never returns it. It exists so
/// [`left_behind_notice`] can name a file that used to be read, and for no
/// other purpose — keeping it out of [`config_file_candidates`] is the
/// whole change.
///
/// It deliberately does not consult the XDG config-home variable, which the
/// removed leg checked first. A probe steered by a redirect variable would
/// re-create the removed shape one step weaker: a run isolated by
/// `$VIBE_SETTINGS` would still resolve a path from an ambient variable the
/// harness does not control. Home is read the one way the settings dir
/// reads it, and nothing redirects it. The cost, named rather than hidden:
/// an operator who had pointed the XDG config home somewhere non-default
/// gets no notice — only the switch to `<settings-dir>/config.toml`, which
/// is correct either way.
pub(super) fn former_config_path() -> Option<PathBuf> {
    // Windows precedence: `%APPDATA%` wins over `~/.config`, which is not
    // the canonical Windows shape.
    if cfg!(windows)
        && let Some(appdata) = std::env::var_os("APPDATA").filter(|s| !s.is_empty())
    {
        return Some(PathBuf::from(appdata).join("vibe").join("config.toml"));
    }
    Some(home_dir()?.join(".config").join("vibe").join("config.toml"))
}

/// Best-effort home-directory detection. Reads `HOME` on Unix and
/// `USERPROFILE` on Windows (or `HOME` as a fallback for Git Bash
/// / WSL shells that set both). Avoids pulling in the `dirs` crate
/// for one lookup.
fn home_dir() -> Option<PathBuf> {
    if let Some(h) = std::env::var_os("HOME").filter(|s| !s.is_empty()) {
        return Some(PathBuf::from(h));
    }
    if cfg!(windows)
        && let Some(p) = std::env::var_os("USERPROFILE").filter(|s| !s.is_empty())
    {
        return Some(PathBuf::from(p));
    }
    None
}
