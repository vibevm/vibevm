//! The fractality home directory (`~/.fractality`, D4/D10).
//!
//! One machine-global home holds the lockfile, the journal, the runs
//! root, and the daemon log. `FRACTALITY_HOME` overrides it — that is
//! how tests get hermetic homes and how a future multi-daemon setup
//! would split.

use camino::Utf8PathBuf;

specmark::scope!("spec://fractality/PROP-001#architecture");

/// Resolves the home directory: explicit override → `FRACTALITY_HOME` →
/// `<user profile>/.fractality`.
pub fn resolve(explicit: Option<&camino::Utf8Path>) -> Result<Utf8PathBuf, String> {
    if let Some(p) = explicit {
        return Ok(p.to_owned());
    }
    if let Some(env) = std::env::var_os("FRACTALITY_HOME") {
        let s = env
            .into_string()
            .map_err(|_| "FRACTALITY_HOME is not valid UTF-8".to_owned())?;
        if !s.trim().is_empty() {
            return Ok(Utf8PathBuf::from(s));
        }
    }
    let profile = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .ok_or_else(|| {
            "neither USERPROFILE nor HOME is set; pass --home or set FRACTALITY_HOME".to_owned()
        })?;
    let s = profile
        .into_string()
        .map_err(|_| "user profile path is not valid UTF-8".to_owned())?;
    Ok(Utf8PathBuf::from(s).join(".fractality"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_home_wins_over_everything() {
        let home = resolve(Some(camino::Utf8Path::new("C:/x/y"))).expect("resolves");
        assert_eq!(home, Utf8PathBuf::from("C:/x/y"));
    }
}
