//! Resolving a launcher's program the way the operating system starts one
//! (PROP-076 `#SYS-HARNESS-EXECUTABLE`).
//!
//! **Why this exists at all.** `Command::new("claude")` finds `claude.exe`
//! on Windows and nothing else, while Claude Code installed by npm is
//! `claude.cmd` — so the built-in `claude` launcher failed with
//! «starting `claude`: program not found» on a machine where Claude Code
//! was installed and on `PATH` (measured on Windows 11, vibe 1.0.12, row
//! C8 of the P9 live check). The fix is the resolution Windows itself
//! performs: the `PATH` entries in order, each with the `PATHEXT`
//! extensions appended, and the resolved FILE spawned.
//!
//! **What is deliberately not done here.** A `.bat` / `.cmd` target is
//! handed to `std::process::Command` as the resolved file and nothing
//! else: Rust's standard library (≥ 1.77) wraps such a target in
//! `cmd.exe` itself and escapes — or refuses — every argument it cannot
//! pass safely, which is the fix for CVE-2024-24576. Hand-building a
//! `cmd /c "…"` string here would replace that audited escaping with our
//! own. A `.ps1` shim is **not** executed either: Windows cannot start one
//! without a PowerShell host, and choosing one for the user would be this
//! command starting a program nobody named.
//!
//! **An explicit path is used as written.** A launcher's `command` that
//! carries a separator is the user naming a file, so it is never looked
//! for on `PATH` — the same rule `commands/tool/which.rs` applies to
//! `vibe tool px exec <command>` and `vibe-app`'s `command:NAME`
//! launchers. The three resolvers stay separate — each sits on its own
//! spec address and its own crate seam — and must stay in agreement.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-HARNESS-EXECUTABLE");

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};

/// The extensions Windows starts a program by when the environment names
/// none. Windows' own documented fallback, spelled once.
pub(crate) const DEFAULT_PATHEXT: &str = ".COM;.EXE;.BAT;.CMD";

/// The one extension vibe refuses to start even when `PATHEXT` lists it:
/// a PowerShell script needs a host process, and picking one would start a
/// program the user never named.
const NOT_STARTED: &str = ".ps1";

/// Whether `program` is a bare name rather than a path. A path is run as
/// written and never looked for on `PATH`.
fn is_bare(program: &str) -> bool {
    !program.contains('/')
        && !program.contains('\\')
        && Path::new(program).components().count() == 1
}

/// The extensions to try, in the order `PATHEXT` gives them.
///
/// Entries that are not extensions are ignored rather than widening the
/// search, and `.ps1` is dropped wherever it appears.
fn extensions(pathext: Option<&OsStr>) -> Vec<String> {
    let listed = pathext
        .map(|value| value.to_string_lossy().into_owned())
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_PATHEXT.to_owned());
    listed
        .split(';')
        .map(str::trim)
        .filter(|extension| extension.starts_with('.') && extension.len() > 1)
        .filter(|extension| !extension.eq_ignore_ascii_case(NOT_STARTED))
        .map(ToOwned::to_owned)
        .collect()
}

/// The file names tried for `program`, in order: the name as it stands
/// when it already ends in one of the listed extensions, and otherwise the
/// name with each of them appended — never the bare name, because Windows
/// cannot start an extensionless file and a directory on `PATH` may well
/// hold one (Node ships `npm` beside `npm.cmd`).
///
/// Each extension is tried **in the case `PATHEXT` spells it and, when
/// that differs, in lower case**, adjacently, so the `PATHEXT` order still
/// decides between two extensions. On Windows neither probe can be the
/// wrong one — its paths are case-insensitive, and `PATHEXT` is
/// conventionally upper case while the shims on disk are not (npm writes
/// `claude.cmd`). On a case-SENSITIVE filesystem the second probe is what
/// makes the resolution answer at all, which is what lets this law be
/// tested off Windows instead of asserted about it.
fn candidates(program: &str, extensions: &[String]) -> Vec<String> {
    if extensions.iter().any(|extension| {
        program
            .to_ascii_lowercase()
            .ends_with(&extension.to_ascii_lowercase())
    }) {
        return vec![program.to_owned()];
    }
    let mut names = Vec::with_capacity(extensions.len());
    for extension in extensions {
        names.push(format!("{program}{extension}"));
        let lower = extension.to_ascii_lowercase();
        if &lower != extension {
            names.push(format!("{program}{lower}"));
        }
    }
    names
}

/// Resolve `program` against the given `PATH` and `PATHEXT` — PURE, so the
/// law is tested over injected directories on every platform rather than
/// over whatever the machine running the tests happens to have installed.
///
/// `Err` is the sentence a caller prints, and it keeps the standard
/// library's own words — «program not found» — while naming the search
/// that did not find it, because «not found» without the extensions tried
/// is exactly the message that left the C8 reader guessing.
pub(crate) fn resolve_in(
    program: &str,
    path: Option<&OsStr>,
    pathext: Option<&OsStr>,
) -> std::result::Result<PathBuf, String> {
    if !is_bare(program) {
        return Ok(PathBuf::from(program));
    }
    let extensions = extensions(pathext);
    let names = candidates(program, &extensions);
    for directory in std::env::split_paths(path.unwrap_or(OsStr::new(""))) {
        // An empty entry is skipped rather than read as the working
        // directory, which is not on `PATH` on either platform.
        if directory.as_os_str().is_empty() {
            continue;
        }
        for name in &names {
            let candidate = directory.join(name);
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }
    Err(format!(
        "program not found: no directory on PATH holds `{program}` under any extension PATHEXT \
         lists ({}). Windows starts a program only through one of them — Claude Code installed \
         by npm is `{program}.cmd` — and a PowerShell `.ps1` shim is not started. Put the \
         program's own directory on PATH, or name the file itself in the launcher's `command`",
        extensions.join(", ")
    ))
}

/// The file this machine would start for `program`.
///
/// Off Windows the answer is the name itself: a bare name there is the
/// kernel's own `PATH` search, which `Command::new` already performs, and
/// resolving it here would change nothing while hiding a `PATH` the child
/// was meant to be looked up in. The branch is a runtime `cfg!`, not a
/// `#[cfg]`, so both halves compile — and are linted — on both platforms.
#[specmark::spec(
    deviates = "spec://core-ai-native/mechanisms/ENGINE-CONFORM-v0.1#rules",
    reason = "ambient-env: `PATH` and `PATHEXT` are the fact being asked — \
              where WOULD this machine's loader find this program, at the \
              moment this launch is about to start it. Resolving them at \
              the composition root and threading them in would answer a \
              different question, the search path as it stood before the \
              launcher was chosen, and `vibe agent` is the one verb that \
              becomes the program it resolves. The resolution itself is \
              PURE (`resolve_in`) over injected values; only these two \
              reads are ambient, exactly as `write_profile`'s `$CODEX_HOME` \
              read in the same command family"
)]
pub(crate) fn resolve(program: &str) -> Result<PathBuf> {
    if !cfg!(windows) {
        return Ok(PathBuf::from(program));
    }
    resolve_in(
        program,
        std::env::var_os("PATH").as_deref(),
        std::env::var_os("PATHEXT").as_deref(),
    )
    .map_err(|reason| anyhow!(reason))
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::*;

    fn file(directory: &Path, name: &str) -> PathBuf {
        let path = directory.join(name);
        std::fs::write(&path, b"shim").expect("writing a shim");
        path
    }

    fn search(directories: &[&Path]) -> OsString {
        std::env::join_paths(directories).expect("joining PATH")
    }

    /// The C8 defect itself: npm installs Claude Code as `claude.cmd`, and
    /// the bare name is not a file at all.
    #[test]
    fn an_npm_cmd_shim_resolves_where_the_bare_name_is_not_a_file() {
        let home = tempfile::tempdir().unwrap();
        let shim = file(home.path(), "claude.cmd");
        let path = search(&[home.path()]);

        assert_eq!(
            resolve_in("claude", Some(&path), None).unwrap(),
            shim,
            "the default PATHEXT must reach a `.cmd` shim"
        );
    }

    /// `PATHEXT` is an ORDER, not a set: the first extension it lists that
    /// exists is the program Windows would start.
    #[test]
    fn the_pathext_order_decides_between_two_shims_of_one_name() {
        let home = tempfile::tempdir().unwrap();
        let cmd = file(home.path(), "x.cmd");
        let exe = file(home.path(), "x.exe");
        let path = search(&[home.path()]);

        let resolved = |pathext: &str| {
            resolve_in("x", Some(&path), Some(OsStr::new(pathext))).expect("one of the two")
        };
        assert_eq!(resolved(DEFAULT_PATHEXT), exe);
        assert_eq!(resolved(".CMD;.EXE"), cmd);
        // Entries that are not extensions are ignored, never widened into
        // a bare-name candidate.
        assert_eq!(resolved(".EXE; ;junk;."), exe);
    }

    /// Directories first, extensions second — the order the loader walks.
    #[test]
    fn the_first_path_directory_that_holds_a_candidate_wins() {
        let home = tempfile::tempdir().unwrap();
        let (first, second) = (home.path().join("a"), home.path().join("b"));
        std::fs::create_dir_all(&first).unwrap();
        std::fs::create_dir_all(&second).unwrap();
        let shim = file(&first, "claude.cmd");
        file(&second, "claude.exe");
        let path = search(&[&first, &second]);

        assert_eq!(resolve_in("claude", Some(&path), None).unwrap(), shim);
        // An empty PATH entry is not the working directory.
        assert!(resolve_in("claude", Some(OsStr::new("")), None).is_err());
    }

    /// A name that already carries a listed extension is the name: it is
    /// tried as it stands and never grown a second one.
    #[test]
    fn a_name_that_already_carries_a_listed_extension_is_tried_as_it_stands() {
        let home = tempfile::tempdir().unwrap();
        let shim = file(home.path(), "claude.cmd");
        file(home.path(), "claude.cmd.exe");
        let path = search(&[home.path()]);

        assert_eq!(resolve_in("claude.cmd", Some(&path), None).unwrap(), shim);
    }

    /// An explicit path is the user naming a file, so it is used as
    /// written — no `PATH`, no `PATHEXT`, no existence test.
    #[test]
    fn an_explicit_path_is_used_as_written() {
        for named in ["./claude", "../bin/claude", r"C:\nvm4w\nodejs\claude.cmd"] {
            assert_eq!(
                resolve_in(named, Some(OsStr::new("/nowhere")), None).unwrap(),
                PathBuf::from(named),
                "`{named}` is a path, not a name to search for"
            );
        }
    }

    /// A PowerShell shim is not a program vibe starts, even when the
    /// operator's own `PATHEXT` lists `.PS1`.
    #[test]
    fn a_powershell_shim_is_never_the_resolved_program() {
        let home = tempfile::tempdir().unwrap();
        file(home.path(), "claude.ps1");
        let path = search(&[home.path()]);

        let refusal = resolve_in("claude", Some(&path), Some(OsStr::new(".CMD;.PS1")))
            .expect_err("`.ps1` is not started");
        assert!(refusal.contains("program not found"), "{refusal}");
        assert!(!refusal.contains(".PS1"), "{refusal}");
    }

    /// The refusal keeps the standard library's words and adds the search
    /// that failed, which is what row C8's message did not say.
    #[test]
    fn a_name_nobody_installed_names_the_pathext_search() {
        let home = tempfile::tempdir().unwrap();
        let path = search(&[home.path()]);

        let refusal = resolve_in("claude", Some(&path), None).expect_err("nothing is installed");
        assert!(refusal.starts_with("program not found:"), "{refusal}");
        assert!(refusal.contains("PATHEXT"), "{refusal}");
        assert!(refusal.contains(".CMD"), "{refusal}");
        assert!(refusal.contains("`claude.cmd`"), "{refusal}");
    }

    /// Off Windows nothing is resolved: `Command::new` searches `PATH`
    /// itself, and this cell must not change that.
    #[test]
    fn off_windows_the_program_is_the_name_as_given() {
        if cfg!(windows) {
            return;
        }
        assert_eq!(resolve("claude").unwrap(), PathBuf::from("claude"));
    }
}
