//! The per-user paths a site profile may root at the reader's own home
//! (`PROP-019#install-root`, `PROP-066#drops`).
//!
//! The directories a profile cannot write out in full are the ones under
//! the home of whoever applies it, and those are exactly the ones a Mac or
//! Linux site wants: `install_root = "~"` gives every developer vibe in
//! their own `~/opt`, `packages = "~/opt/drop/packages"` their own drop.
//! `~` and `~/<rest>` are per-user absolute — one directory on every
//! machine that reads the profile — so they pass where a relative path
//! does not, and are resolved where the layer is read. `~user` is nobody
//! vibe can resolve and is refused by the key that carries it.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#install-root");

use std::path::Path;

use super::SiteProfile;
use super::windows_paths::{is_absolute_anywhere, starts_at_home};
use crate::user_config::artifacts::expand_home;

impl SiteProfile {
    /// This profile with its per-user paths — the artifact drops,
    /// `[self] path` and `[self] install_root` — resolved against `home`
    /// (`PROP-019#install-root`, `PROP-066#drops`).
    ///
    /// The stored profile keeps the `~` the administrator wrote, because
    /// every machine that applies it reads a different home; only what is
    /// read into memory is resolved.
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use vibe_core::site_profile::SiteProfile;
    ///
    /// let profile = SiteProfile::parse(
    ///     "schema = 1\n[artifacts]\npackages = '~/opt/drop/packages'\n",
    ///     "corp.toml",
    /// )
    /// .unwrap();
    /// let home = PathBuf::from("/home/me");
    /// assert_eq!(
    ///     profile.with_home(Some(&home)).artifacts.packages.as_deref(),
    ///     Some(home.join("opt/drop/packages").as_path())
    /// );
    /// ```
    #[must_use]
    pub fn with_home(&self, home: Option<&Path>) -> Self {
        SiteProfile {
            artifacts: self.artifacts.with_home(home),
            self_source: self.self_source.with_home(home),
            ..self.clone()
        }
    }
}

/// Every path of the profile that must name a directory, with its key.
fn declared(profile: &SiteProfile) -> [(&'static str, Option<&Path>); 6] {
    [
        ("artifacts.vibe", profile.artifacts.vibe.as_deref()),
        ("artifacts.packages", profile.artifacts.packages.as_deref()),
        ("self.path", profile.self_source.path.as_deref()),
        (
            "self.install_root",
            profile.self_source.install_root.as_deref(),
        ),
        ("proxy.ca_file", profile.proxy.ca_file.as_deref()),
        ("proxy.px_path", profile.proxy.px_path.as_deref()),
    ]
}

/// Which of the four per-user keys may start at the home directory. The
/// other two name files this host opens as written.
fn per_user(key: &str) -> bool {
    matches!(
        key,
        "artifacts.vibe" | "artifacts.packages" | "self.path" | "self.install_root"
    )
}

/// The first path that names no directory on any machine, as the key, the
/// reason and the fix the profile's refusal carries.
pub(super) fn check(profile: &SiteProfile) -> Result<(), (&'static str, String, &'static str)> {
    for (key, path) in declared(profile) {
        let Some(path) = path else { continue };
        // `~user` is another account's home, which vibe does not resolve —
        // wherever it appears.
        if per_user(key) && expand_home(path, Some(Path::new("/anywhere"))).is_err() {
            return Err((
                key,
                format!("`{}` is not a directory vibe can resolve", path.display()),
                "write `~` for the reader's own home, or the directory in full",
            ));
        }
        // A `~`-rooted per-user path IS absolute, per user; anything else
        // relative would be read from wherever a command happened to run.
        if is_absolute_anywhere(path) || (per_user(key) && starts_at_home(path)) {
            continue;
        }
        return Err((
            key,
            format!("`{}` is relative", path.display()),
            "give an absolute directory (a profile is read on every machine)",
        ));
    }
    Ok(())
}
