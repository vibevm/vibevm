//! Worker-program resolution (F14): from a profile's bare program name
//! to a spawnable path, against the **worker's** PATH.
//!
//! `std::process::Command`/CreateProcess resolves bare names to `.exe`
//! only — an npm-installed Claude Code ships nothing but a `claude.cmd`
//! shim on Windows, so `claude_binary = "claude"` spawns dead unless we
//! resolve extensions ourselves (PATHEXT semantics, narrowed to the
//! shapes that can actually run). Resolution is a pure function over the
//! name and a PATH string — the caller passes the worker env's PATH, so
//! the probe sees exactly what the child will see.

use camino::Utf8PathBuf;

specmark::scope!("spec://fractality/PROP-001#architecture");

/// Candidate extensions on Windows, native binaries first: prefer a real
/// `claude.exe` over the npm `claude.cmd` shim when both are installed.
/// A `.cmd`/`.bat` hit is spawnable: Rust runs it via `cmd.exe /c` with
/// safe escaping (flag-shaped argv only — the prompt rides stdin, F14).
#[cfg(windows)]
const WIN_EXTENSIONS: &[&str] = &["exe", "cmd", "bat"];

/// Resolves a program name against a PATH value.
///
/// - A name carrying a path separator is explicit: returned as-is, the
///   spawn call reports its own errors.
/// - A bare name is searched dir by dir. Windows: a name already ending
///   in a spawnable extension is probed literally, otherwise each dir is
///   probed for `name.exe`, `name.cmd`, `name.bat` (in that order — an
///   extensionless file is not spawnable on Windows). POSIX: the literal
///   name.
/// - No hit is a loud error naming the searched surface and the fix.
pub fn resolve_program(name: &str, path_value: Option<&str>) -> Result<String, String> {
    if name.contains('/') || name.contains('\\') {
        return Ok(name.to_owned());
    }
    let path_value = path_value
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| format!("cannot resolve `{name}`: the worker env carries no PATH"))?;

    let separator = if cfg!(windows) { ';' } else { ':' };
    for dir in path_value.split(separator).filter(|d| !d.trim().is_empty()) {
        for candidate in candidates(name) {
            let probe = Utf8PathBuf::from(dir).join(&candidate);
            if probe.as_std_path().is_file() {
                return Ok(probe.into_string());
            }
        }
    }
    Err(format!(
        "`{name}` not found on the worker PATH (probed {} in every PATH dir); \
         install it there or set the profile's binary to an absolute path",
        candidates(name).join("/"),
    ))
}

#[cfg(windows)]
fn candidates(name: &str) -> Vec<String> {
    let has_spawnable_ext = WIN_EXTENSIONS
        .iter()
        .any(|ext| name.to_ascii_lowercase().ends_with(&format!(".{ext}")));
    if has_spawnable_ext {
        vec![name.to_owned()]
    } else {
        WIN_EXTENSIONS
            .iter()
            .map(|ext| format!("{name}.{ext}"))
            .collect()
    }
}

#[cfg(unix)]
fn candidates(name: &str) -> Vec<String> {
    vec![name.to_owned()]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> Utf8PathBuf {
        let dir =
            std::env::temp_dir().join(format!("fractality-resolve-{tag}-{}", ulid::Ulid::new()));
        std::fs::create_dir_all(&dir).expect("scratch dir");
        Utf8PathBuf::from_path_buf(dir).expect("utf-8 temp dir")
    }

    #[test]
    fn explicit_paths_pass_through_untouched() {
        assert_eq!(
            resolve_program("C:/tools/claude.exe", None).expect("explicit"),
            "C:/tools/claude.exe"
        );
        assert_eq!(
            resolve_program("dir\\claude", Some("ignored")).expect("explicit"),
            "dir\\claude"
        );
    }

    #[test]
    fn missing_path_is_a_loud_error() {
        let err = resolve_program("claude", None).expect_err("no PATH");
        assert!(err.contains("no PATH"), "got: {err}");
        let err = resolve_program("claude", Some("  ")).expect_err("blank PATH");
        assert!(err.contains("no PATH"), "got: {err}");
    }

    #[test]
    fn unfindable_name_names_the_fix() {
        let dir = scratch("empty");
        let err = resolve_program("claude", Some(dir.as_str())).expect_err("nothing to find");
        assert!(err.contains("absolute path"), "got: {err}");
        std::fs::remove_dir_all(dir.as_std_path()).ok();
    }

    #[cfg(windows)]
    #[test]
    fn cmd_shim_is_found_when_no_exe_exists() {
        let dir = scratch("shim");
        std::fs::write(dir.join("claude.cmd").as_std_path(), "@echo off").expect("shim");
        let resolved = resolve_program("claude", Some(dir.as_str())).expect("resolves the shim");
        assert_eq!(Utf8PathBuf::from(resolved), dir.join("claude.cmd"));
        std::fs::remove_dir_all(dir.as_std_path()).ok();
    }

    #[cfg(windows)]
    #[test]
    fn exe_beats_cmd_in_the_same_dir() {
        let dir = scratch("pref");
        std::fs::write(dir.join("claude.cmd").as_std_path(), "@echo off").expect("cmd");
        std::fs::write(dir.join("claude.exe").as_std_path(), "MZ").expect("exe");
        let resolved = resolve_program("claude", Some(dir.as_str())).expect("resolves");
        assert_eq!(Utf8PathBuf::from(resolved), dir.join("claude.exe"));
        std::fs::remove_dir_all(dir.as_std_path()).ok();
    }

    #[cfg(windows)]
    #[test]
    fn earlier_path_dir_wins() {
        let first = scratch("first");
        let second = scratch("second");
        std::fs::write(first.join("claude.cmd").as_std_path(), "@echo off").expect("cmd");
        std::fs::write(second.join("claude.exe").as_std_path(), "MZ").expect("exe");
        let path = format!("{first};{second}");
        let resolved = resolve_program("claude", Some(&path)).expect("resolves");
        assert_eq!(
            Utf8PathBuf::from(resolved),
            first.join("claude.cmd"),
            "PATH order beats extension preference across dirs"
        );
        std::fs::remove_dir_all(first.as_std_path()).ok();
        std::fs::remove_dir_all(second.as_std_path()).ok();
    }

    #[cfg(windows)]
    #[test]
    fn explicit_extension_is_probed_literally() {
        let dir = scratch("literal");
        std::fs::write(dir.join("claude.cmd").as_std_path(), "@echo off").expect("cmd");
        let resolved = resolve_program("claude.cmd", Some(dir.as_str())).expect("literal probe");
        assert_eq!(Utf8PathBuf::from(resolved), dir.join("claude.cmd"));
        assert!(
            resolve_program("claude.exe", Some(dir.as_str())).is_err(),
            "an explicit .exe never falls back to the shim"
        );
        std::fs::remove_dir_all(dir.as_std_path()).ok();
    }
}
