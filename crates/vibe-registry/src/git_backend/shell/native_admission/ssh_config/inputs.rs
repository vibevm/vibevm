//! Freeze and audit Include input before the selected OpenSSH evaluator runs.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#SSH-CONFIG-CAPTURE");
use super::*;
use std::process::{Command, Stdio};
pub(super) struct Frozen {
    _root: tempfile::TempDir,
    path: PathBuf,
    pub(super) witness: Witness,
}
/// Only structural lexical decoding is owned here; Host matching/precedence is
/// evaluated by OpenSSH. Unsupported quoting never becomes an audit bypass.
pub(super) fn words(line: &str) -> Result<Vec<String>, GitError> {
    words_for(line, cfg!(windows))
}
/// [`words`] for a platform. On Windows a backslash before anything but
/// `\`, `"`, `'` or whitespace is a literal path separator (`C:\Users`),
/// exactly as every Windows OpenSSH reads it; elsewhere such an escape is
/// still refused as unqualified.
pub(super) fn words_for(line: &str, windows: bool) -> Result<Vec<String>, GitError> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quoted = false;
    let mut escaped = false;
    let mut separator = false;
    for c in line.chars() {
        if escaped {
            escaped = false;
            if c != '\\' && c != '"' && !c.is_ascii_whitespace() {
                if !windows || c == '\'' {
                    return Err(unsupported("unqualified SSH configuration escape"));
                }
                // A Windows path separator: the backslash stays, and this
                // character is read as usual below.
                word.push('\\');
            } else {
                word.push(c);
                continue;
            }
        }
        if c == '\\' {
            escaped = true;
            continue;
        }
        if c == '"' {
            quoted = !quoted;
            continue;
        }
        if !quoted && c == '#' {
            if !word.is_empty() {
                return Err(unsupported(
                    "unquoted SSH path/comment boundary is ambiguous",
                ));
            }
            break;
        }
        if !quoted
            && c == '='
            && !separator
            && (words.is_empty() || (words.len() == 1 && word.is_empty()))
        {
            if words.is_empty() && word.is_empty() {
                return Err(unsupported("SSH config has no keyword before separator"));
            }
            if !word.is_empty() {
                words.push(std::mem::take(&mut word));
            }
            separator = true;
            continue;
        }
        if !quoted && c.is_whitespace() {
            if !word.is_empty() {
                words.push(std::mem::take(&mut word));
            }
        } else {
            if c.is_control() {
                return Err(unsupported("unsupported SSH config control character"));
            }
            word.push(c);
        }
    }
    if escaped && windows && !quoted {
        // A trailing Windows separator (`Include C:\ssh\`) is literal.
        word.push('\\');
        escaped = false;
    }
    if escaped || quoted {
        return Err(unsupported(
            "unsupported SSH config quoting or continuation",
        ));
    }
    if !word.is_empty() {
        words.push(word);
    }
    Ok(words)
}
impl Frozen {
    #[specmark::spec(
        implements = "spec://org.vibevm.core/vibevm/common/PROP-073#SSH-CONFIG-CAPTURE-LAW"
    )]
    pub(super) fn capture(input: &SshConfigSelection) -> Result<Self, GitError> {
        let mut builder = tempfile::Builder::new();
        builder.prefix("vibe-ssh-config-");
        let root = match &input.acquisition_root {
            None => builder.tempdir(),
            Some(path) => {
                std::fs::create_dir_all(path).map_err(io)?;
                builder.tempdir_in(path)
            }
        }
        .map_err(io)?;
        let mut witness = Witness::default();
        let mut count = Budget { files: 0, bytes: 0 };
        let mut combined = String::new();
        for (path, base) in [
            (Some(&input.user), input.home.join(".ssh")),
            (
                input.system.as_ref(),
                input
                    .system
                    .as_ref()
                    .and_then(|p| p.parent())
                    .unwrap_or(&input.home)
                    .to_path_buf(),
            ),
        ] {
            if let Some(path) = path
                && let Some(frozen) =
                    freeze_file(input, path, &base, root.path(), &mut witness, &mut count, 0)?
            {
                combined.push_str(&format!(
                    "Host *\nInclude {}\n",
                    super::super::ssh::config_path(&frozen)?
                ));
            }
        }
        // The ordinary user trust defaults follow the explicitly selected home,
        // just as deferred ~ identity paths do. Earlier selected values win.
        combined.push_str(&format!(
            "Host *\nUserKnownHostsFile {} {}\n",
            super::super::ssh::config_path(&input.home.join(".ssh/known_hosts"))?,
            super::super::ssh::config_path(&input.home.join(".ssh/known_hosts2"))?
        ));
        let path = root.path().join("root.config");
        std::fs::write(&path, combined).map_err(io)?;
        Ok(Self {
            _root: root,
            path,
            witness,
        })
    }
}
struct Budget {
    files: usize,
    bytes: usize,
}
fn freeze_file(
    input: &SshConfigSelection,
    path: &Path,
    base: &Path,
    root: &Path,
    witness: &mut Witness,
    count: &mut Budget,
    depth: usize,
) -> Result<Option<PathBuf>, GitError> {
    if depth > 16 || count.files >= 256 {
        return Err(unsupported("SSH Include closure exceeds capture bound"));
    }
    let Some(bytes) = witness.content(path)? else {
        return Ok(None);
    };
    witness.config_permissions(path, root)?;
    let text = std::str::from_utf8(&bytes).map_err(|_| unsupported("SSH config is not UTF-8"))?;
    let destination = root.join(format!("input-{}.config", count.files));
    count.files += 1;
    count.bytes += bytes.len();
    if count.bytes > 4 * 1024 * 1024 {
        return Err(unsupported(
            "SSH configuration closure exceeds total byte bound",
        ));
    }
    let mut frozen = String::new();
    for line in text.lines() {
        let args = words(line)?;
        let Some(key) = args.first() else {
            frozen.push('\n');
            continue;
        };
        let key = key.to_ascii_lowercase();
        if key == "match" {
            audit_match(&args[1..])?;
        }
        if key == "canonicalizehostname" && args.get(1).is_none_or(|v| v != "no" && v != "false") {
            return Err(unsupported(
                "SSH DNS canonicalization is not supported during capture",
            ));
        }
        if key == "include" {
            if args.len() < 2 {
                return Err(unsupported("SSH Include has no path"));
            }
            for pattern in &args[1..] {
                let pattern = values::path(pattern, input, None)?;
                let pattern = if pattern.is_absolute() {
                    pattern
                } else {
                    base.join(pattern)
                };
                let pattern = pattern
                    .to_str()
                    .ok_or_else(|| unsupported("SSH Include path is not UTF-8"))?;
                for child in witness.glob(pattern)? {
                    if let Some(copy) =
                        freeze_file(input, &child, base, root, witness, count, depth + 1)?
                    {
                        frozen.push_str(&format!(
                            "Include {}\n",
                            super::super::ssh::config_path(&copy)?
                        ));
                    }
                }
            }
        } else if matches!(
            key.as_str(),
            "identityfile" | "userknownhostsfile" | "globalknownhostsfile" | "identityagent"
        ) {
            let mut lowered = Vec::new();
            for value in &args[1..] {
                if matches!(key.as_str(), "userknownhostsfile" | "globalknownhostsfile")
                    && value.chars().any(char::is_whitespace)
                {
                    return Err(unsupported(
                        "whitespace in SSH trust paths lacks an unambiguous evaluator representation",
                    ));
                }
                let value = if key == "identityagent"
                    && (value == "SSH_AUTH_SOCK"
                        || (value.starts_with('$') && !value.starts_with("${")))
                {
                    value.clone()
                } else {
                    values::deferred_path(value, input)?
                };
                if value.chars().any(char::is_control) {
                    return Err(unsupported("SSH path contains control syntax"));
                }
                let value = if cfg!(windows) {
                    value.replace('\\', "/")
                } else {
                    value
                };
                lowered.push(format!(
                    "\"{}\"",
                    value.replace('\\', "\\\\").replace('\"', "\\\"")
                ));
            }
            frozen.push_str(&format!("{key} {}\n", lowered.join(" ")));
        } else {
            frozen.push_str(line);
            frozen.push('\n');
        }
    }
    std::fs::write(&destination, frozen).map_err(io)?;
    Ok(Some(destination))
}
fn audit_match(args: &[String]) -> Result<(), GitError> {
    let mut args = args.iter();
    while let Some(key) = args.next() {
        match key.trim_start_matches('!').to_ascii_lowercase().as_str() {
            "all" | "final" | "canonical" => (),
            "host" | "originalhost" | "user" | "localuser" => {
                if args.next().is_none() {
                    return Err(unsupported("incomplete data-only SSH Match"));
                }
            }
            _ => {
                return Err(unsupported(
                    "executable or unsupported dynamic SSH Match cannot be evaluated safely",
                ));
            }
        }
    }
    Ok(())
}
pub(super) fn evaluate(
    input: &SshConfigSelection,
    frozen: &Frozen,
    authority: &Authority,
) -> Result<String, GitError> {
    let mut command = Command::new(&input.executable);
    command
        .env_clear()
        .env(
            "PATH",
            input.executable.parent().unwrap_or(Path::new("/usr/bin")),
        )
        .env("HOME", &input.home)
        .env("LC_ALL", "C")
        .envs(super::super::platform::platform_variables(
            input
                .environment
                .iter()
                .map(|(k, v)| (std::ffi::OsStr::new(k), std::ffi::OsStr::new(v))),
            cfg!(windows),
        ))
        .args(["-G", "-F"])
        .arg(&frozen.path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if let Some(user) = &authority.user {
        command.args(["-l", user]);
    }
    if let Some(port) = authority.port {
        command.arg("-p").arg(port.to_string());
    }
    command.arg("--").arg(&authority.host);
    let control = crate::git_backend::transfers::current()
        .progress
        .unwrap_or_default()
        .cancellation()
        .clone();
    let output = crate::git_backend::shell::controlled::bounded_output(
        &mut command,
        &control,
        std::time::Duration::from_secs(5),
        262144,
    )
    .map_err(|error| {
        crate::git_backend::shell::controlled::git_error(
            error,
            "audited SSH config evaluator".into(),
        )
    })?;
    let status = output.status;
    let bytes = output.stdout;
    if !status.success() || bytes.len() > 262144 {
        return Err(unsupported(
            "audited SSH config evaluation failed or exceeded its output bound",
        ));
    }
    String::from_utf8(bytes)
        .map_err(|_| unsupported("SSH evaluator returned non-UTF-8 configuration"))
}

#[cfg(test)]
mod lexer_tests {
    use super::words_for;

    #[test]
    #[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-WINDOWS-LAW")]
    fn windows_backslashes_are_path_separators_only_on_windows() {
        let line = r"IdentityFile C:\Users\me\.ssh\id_ed25519";
        assert_eq!(
            words_for(line, true).unwrap(),
            ["IdentityFile", r"C:\Users\me\.ssh\id_ed25519"]
        );
        assert!(words_for(line, false).is_err());
        // Quoted, with a space, and a trailing separator.
        assert_eq!(
            words_for(r#"Include "C:\Program Files\ssh\conf.d\*""#, true).unwrap(),
            ["Include", r"C:\Program Files\ssh\conf.d\*"]
        );
        assert_eq!(
            words_for(r"Include C:\ssh\", true).unwrap(),
            ["Include", r"C:\ssh\"]
        );
        // The qualified escapes keep their meaning on both platforms.
        for windows in [false, true] {
            assert_eq!(
                words_for(r#"IdentityFile "a\"b" c\ d"#, windows).unwrap(),
                ["IdentityFile", r#"a"b"#, "c d"]
            );
        }
        // A quote escape is still refused.
        assert!(words_for(r"IdentityFile a\'b", true).is_err());
    }
}
