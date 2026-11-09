//! Paths and settings a site profile may name that only a Windows host can
//! use (`PROP-019#site-profile`): a profile written for Windows desktops is
//! validated, and stored, on any host.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-019#site-profile");

use std::path::Path;

use super::SiteProfile;

impl SiteProfile {
    /// What a valid profile names that only a Windows host can use, when it
    /// is checked on another host (`windows = false`): a Windows drive or
    /// UNC path, which is accepted and stored but read only on Windows, and
    /// `[proxy] pac = "system"`, which reads the Windows Internet Options.
    /// Each note names the key and what happens on this host; empty on
    /// Windows.
    ///
    /// ```
    /// use vibe_core::site_profile::SiteProfile;
    /// let profile = SiteProfile::parse(
    ///     "schema = 1\n[artifacts]\nvibe = '\\\\fs\\rel'\n[proxy]\nmode = 'px'\npac = 'system'\n",
    ///     "corp.toml",
    /// )
    /// .unwrap();
    /// let notes = profile.windows_only_notes(false);
    /// assert_eq!(notes.len(), 2);
    /// assert!(notes[0].contains("artifacts.vibe"));
    /// assert!(notes[1].contains("proxy.pac"));
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
        if self.proxy.pac.as_deref().is_some_and(|pac| {
            matches!(
                crate::user_config::PacSource::parse(pac),
                Ok(crate::user_config::PacSource::System)
            )
        }) {
            notes.push(
                "`proxy.pac = \"system\"` reads the Windows Internet Options: it works only \
                 on Windows — on this host give the PAC URL or an absolute PAC file path"
                    .to_owned(),
            );
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
