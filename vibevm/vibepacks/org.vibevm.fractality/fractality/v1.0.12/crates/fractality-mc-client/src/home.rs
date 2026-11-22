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
    Ok(user_home()?.join(".fractality"))
}

/// The user's profile directory (`USERPROFILE` on Windows, `HOME` on
/// POSIX). This module is the recorded composition surface for that
/// read — everything else takes the value as a parameter.
pub fn user_home() -> Result<Utf8PathBuf, String> {
    let profile = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .ok_or_else(|| {
            "neither USERPROFILE nor HOME is set; pass --home or set FRACTALITY_HOME".to_owned()
        })?;
    let s = profile
        .into_string()
        .map_err(|_| "user profile path is not valid UTF-8".to_owned())?;
    Ok(Utf8PathBuf::from(s))
}

/// Expands a leading `~/` (or bare `~`) against `user_home`; anything
/// else passes through untouched. Pure — the caller supplies the home,
/// tests never touch the ambient environment.
pub fn expand_user(path: &camino::Utf8Path, user_home: &camino::Utf8Path) -> Utf8PathBuf {
    let s = path.as_str();
    if s == "~" {
        return user_home.to_owned();
    }
    if let Some(rest) = s.strip_prefix("~/").or_else(|| s.strip_prefix("~\\")) {
        return user_home.join(rest);
    }
    path.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_home_wins_over_everything() {
        let home = resolve(Some(camino::Utf8Path::new("C:/x/y"))).expect("resolves");
        assert_eq!(home, Utf8PathBuf::from("C:/x/y"));
    }

    #[test]
    fn tilde_expansion_is_pure_and_literal() {
        let uh = camino::Utf8Path::new("C:/Users/dev");
        assert_eq!(
            expand_user(camino::Utf8Path::new("~/.vibevm/zai.api.token"), uh),
            Utf8PathBuf::from("C:/Users/dev/.vibevm/zai.api.token")
        );
        assert_eq!(
            expand_user(camino::Utf8Path::new("~"), uh),
            Utf8PathBuf::from("C:/Users/dev")
        );
        assert_eq!(
            expand_user(camino::Utf8Path::new("C:/abs/path"), uh),
            Utf8PathBuf::from("C:/abs/path"),
            "absolute paths pass through"
        );
        assert_eq!(
            expand_user(camino::Utf8Path::new("rel/~inside"), uh),
            Utf8PathBuf::from("rel/~inside"),
            "mid-path tildes are literal"
        );
    }
}
