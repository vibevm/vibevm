//! Paths and settings a site profile may name that only a Windows host can
//! use (`PROP-019#site-profile`): a profile written for Windows desktops is
//! validated, and stored, on any host.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#site-profile");

use std::path::Path;

use super::SiteProfile;

impl SiteProfile {
    /// What a valid profile names that only a Windows host can use, when it
    /// is checked on another host (`windows = false`): a Windows drive or
    /// UNC path, which is accepted and stored but read only on Windows.
    /// Each note names the key and what happens on this host; empty on
    /// Windows.
    ///
    /// `[proxy] pac = "system"` is **not** one of them: since
    /// `PROP-068#proxy-system` resolves it on every host — the Windows
    /// Internet Options, the macOS network settings, the Linux desktops
    /// and the environment — a profile that asks for it is as usable
    /// here as it is there.
    ///
    /// ```
    /// use vibe_core::site_profile::SiteProfile;
    /// let profile = SiteProfile::parse(
    ///     "schema = 1\n[artifacts]\nvibe = '\\\\fs\\rel'\n[proxy]\nmode = 'px'\npac = 'system'\n",
    ///     "corp.toml",
    /// )
    /// .unwrap();
    /// let notes = profile.windows_only_notes(false);
    /// assert_eq!(notes.len(), 1);
    /// assert!(notes[0].contains("artifacts.vibe"));
    /// assert!(notes.iter().all(|note| !note.contains("proxy.pac")));
    /// assert!(profile.windows_only_notes(true).is_empty());
    /// ```
    pub fn windows_only_notes(&self, windows: bool) -> Vec<String> {
        if windows {
            return Vec::new();
        }
        let mut notes = Vec::new();
        for (key, path) in [
            ("artifacts.vibe", self.artifacts.vibe.as_ref()),
            ("artifacts.packages", self.artifacts.packages.as_ref()),
            ("self.path", self.self_source.path.as_ref()),
            ("self.install_root", self.self_source.install_root.as_ref()),
            ("proxy.ca_file", self.proxy.ca_file.as_ref()),
            ("proxy.px_path", self.proxy.px_path.as_ref()),
        ] {
            if let Some(path) = path.filter(|path| is_windows_absolute(path)) {
                notes.push(format!(
                    "`{key} = \"{}\"` is a Windows path: it is accepted, but only a Windows \
                     host reads it — on this host it is not opened",
                    path.display()
                ));
            }
        }
        notes
    }
}

/// An absolute path on this host, or a Windows drive / UNC path read on
/// Unix (a profile written for Windows desktops is still validated on the
/// administrator's Linux box).
///
/// ```
/// use std::path::Path;
/// use vibe_core::site_profile::is_absolute_anywhere;
/// assert!(is_absolute_anywhere(Path::new("/srv/vibe")));
/// assert!(is_absolute_anywhere(Path::new("C:/Work")));
/// assert!(is_absolute_anywhere(Path::new(r"\\server\share\releases")));
/// assert!(!is_absolute_anywhere(Path::new("releases")));
/// ```
pub fn is_absolute_anywhere(path: &Path) -> bool {
    path.is_absolute() || is_windows_absolute(path)
}

/// A Windows drive path (`C:\x`, `C:/x`) or a backslash UNC path
/// (`\\server\share\x`), whatever the host. Only a Windows host can read
/// one; elsewhere it is validated and carried, never opened.
///
/// ```
/// use std::path::Path;
/// use vibe_core::site_profile::is_windows_absolute;
/// assert!(is_windows_absolute(Path::new(r"\\server\share")));
/// assert!(is_windows_absolute(Path::new("D:\\vibe")));
/// assert!(!is_windows_absolute(Path::new("/srv/vibe")));
/// ```
pub fn is_windows_absolute(path: &Path) -> bool {
    let text = path.to_string_lossy();
    let bytes = text.as_bytes();
    (bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'/' | b'\\'))
        || text.starts_with("\\\\")
}

/// A path rooted at the reader's own home: `~`, `~/<rest>`, and `~\<rest>`
/// on Windows (`PROP-019#install-root`). Such a path is per-user absolute
/// — it names one directory on every machine that reads the profile — so a
/// profile may carry it where it may not carry a relative path; `~user` is
/// not one of these, and is refused with its own reason.
///
/// ```
/// use std::path::Path;
/// use vibe_core::site_profile::starts_at_home;
/// assert!(starts_at_home(Path::new("~")));
/// assert!(starts_at_home(Path::new("~/opt")));
/// assert!(!starts_at_home(Path::new("~bob/opt")));
/// assert!(!starts_at_home(Path::new("opt")));
/// assert!(!starts_at_home(Path::new("/srv/vibe")));
/// ```
pub fn starts_at_home(path: &Path) -> bool {
    matches!(
        path.components().next(),
        Some(std::path::Component::Normal(first)) if first == std::ffi::OsStr::new("~")
    )
}
