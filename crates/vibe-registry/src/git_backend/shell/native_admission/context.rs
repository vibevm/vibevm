//! Operation-owned cwd, HOME, config, hooks and TLS inputs.
use super::*;
use std::process::{Command, Output, Stdio};
// Only this module can construct a command carrying native transport authority.
pub(in crate::git_backend::shell) struct AdmittedCommand(Command, NativeProtocol);
impl AdmittedCommand {
    pub(in crate::git_backend::shell) fn into_parts(self) -> (Command, NativeProtocol) {
        (self.0, self.1)
    }
}
/// Every configuration entry a controlled child is given outright. All
/// entries originate here, never from inherited `-c`, environment or
/// configuration input, and all but one are a refusal.
///
/// The exception is `core.longpaths`, and it is a platform fact rather
/// than a policy: Git for Windows refuses to create a working-tree path
/// longer than `MAX_PATH` (260 characters), reporting `Filename too long`
/// and exiting 128. A controlled child never reads the system or global
/// configuration (`GIT_CONFIG_NOSYSTEM`), so the installation's own
/// `core.longpaths` cannot reach it, and its staging checkout lives under
/// the operation's acquisition root — some ninety characters of settings
/// home before the repository's own deepest path. A mirrored monorepo
/// carrying nested dependency trees therefore could not be captured on
/// Windows **at all** until this entry was given to the child, and the
/// failure read as a refused source rather than as a path length
/// (PROP-073 `#NATIVE-WINDOWS`, PROP-001 `#windows-ux`). It is given only
/// on Windows: elsewhere `MAX_PATH` does not exist and the entry means
/// nothing.
fn owned_config(windows: bool) -> Vec<&'static str> {
    let mut entries = vec![
        "credential.helper=",
        "core.askPass=",
        "http.followRedirects=false",
        "http.sslVerify=true",
        "http.extraHeader=",
        "http.cookieFile=",
        "http.saveCookies=false",
        "fetch.recurseSubmodules=false",
        "submodule.recurse=false",
        "maintenance.auto=false",
        "gc.auto=0",
        "core.fsmonitor=false",
    ];
    if windows {
        entries.push("core.longpaths=true");
    }
    entries
}

pub(super) struct Context<'a> {
    profile: &'a NativeGitAdmission,
    root: tempfile::TempDir,
}
impl<'a> Context<'a> {
    pub(super) fn new(profile: &'a NativeGitAdmission) -> Result<Self, GitError> {
        let root = profile.temporary("vibe-native-")?;
        for dir in ["home", "empty", "hooks"] {
            std::fs::create_dir(root.path().join(dir)).map_err(io)?;
        }
        std::fs::write(root.path().join("config"), b"").map_err(io)?;
        Ok(Self { profile, root })
    }
    fn command(
        &self,
        repo: Option<&Path>,
        remote: Option<&AdmittedRemote>,
    ) -> Result<Command, GitError> {
        let root = self.root.path();
        let mut cmd = Command::new(&self.profile.installation.executable);
        cmd.env_clear()
            .current_dir(root)
            .env("HOME", root.join("home"))
            .env("XDG_CONFIG_HOME", root.join("home"))
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_SYSTEM", root.join("config"))
            .env("GIT_CONFIG_GLOBAL", root.join("config"))
            .env("GIT_ATTR_NOSYSTEM", "1")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_NO_LAZY_FETCH", "1")
            .env("GIT_NO_REPLACE_OBJECTS", "1")
            .env("GIT_OPTIONAL_LOCKS", "0")
            .env("LC_ALL", "C")
            .env("LANG", "C")
            .env(
                "GIT_ALLOW_PROTOCOL",
                match remote.map(AdmittedRemote::protocol) {
                    Some(NativeProtocol::Http) => "http",
                    Some(NativeProtocol::Https) => "https",
                    Some(NativeProtocol::Ssh) => "ssh",
                    Some(NativeProtocol::File) => "file",
                    None => "",
                },
            )
            .env("GIT_TEMPLATE_DIR", root.join("empty"))
            .stdin(Stdio::null());
        self.profile.installation.prepare(&mut cmd);
        for value in owned_config(cfg!(windows)) {
            cmd.args(["-c", value]);
        }
        // PROP-068 `#proxy`: the configured route is the only proxy this
        // child may use; without one, an empty `http.proxy` keeps it direct.
        let network = match remote {
            Some(AdmittedRemote::Https { url, .. }) => {
                vibe_net::git::current_for_remote(url.as_str())
                    .map_err(|_| unsupported("the configured network route is unavailable"))?
            }
            _ => vibe_net::git::GitNetwork::default(),
        };
        if !network
            .config
            .iter()
            .any(|value| value.starts_with("http.proxy="))
            && !network.env.iter().any(|(name, _)| name == "HTTPS_PROXY")
        {
            cmd.args(["-c", "http.proxy="]);
        }
        network.apply(&mut cmd);
        if matches!(remote, Some(AdmittedRemote::Https { url, .. }) if url.scheme() == "https")
            && let Some(backend) = platform::windows_tls_backend(
                cfg!(windows),
                self.profile.selects_ca_file()
                    || network
                        .config
                        .iter()
                        .any(|value| value.starts_with("http.sslCAInfo=")),
            )
        {
            cmd.args(["-c", backend]);
        }
        if let Some(AdmittedRemote::Local { .. }) = remote {
            // A shared repository (an NFS export, a `\\server\share`) is
            // usually owned by another account; Git's ownership check would
            // refuse its `upload-pack`. This child runs no hooks and reads no
            // system or global configuration, and the repository is the one
            // the user configured, so the check is waived for it alone.
            cmd.args(["-c", "safe.directory=*"]);
        }
        cmd.arg("-c")
            .arg(format!("core.hooksPath={}", root.join("hooks").display()));
        cmd.arg("-c").arg(format!(
            "core.attributesFile={}",
            root.join("config").display()
        ));
        if matches!(remote, Some(AdmittedRemote::Https { url, .. }) if url.scheme() == "https")
            && let Some(bytes) = self.profile.selected_pem()?
        {
            std::fs::write(root.join("ca.pem"), bytes.as_slice()).map_err(io)?;
            cmd.env("GIT_SSL_CAINFO", root.join("ca.pem"));
        }
        if let Some(AdmittedRemote::Https {
            url,
            authorization: Some(auth),
        }) = remote
        {
            // Hidden redirects are disabled, and this exact captured origin is
            // the only remote reachable by this child. Never write this config.
            cmd.env("GIT_CONFIG_COUNT", "1")
                .env(
                    "GIT_CONFIG_KEY_0",
                    format!("http.{}/.extraHeader", url.origin().ascii_serialization()),
                )
                .env("GIT_CONFIG_VALUE_0", format!("Authorization: {auth}"));
        }
        if let Some(AdmittedRemote::Ssh(remote)) = remote {
            remote.configure(&self.profile.ssh, &mut cmd, root)?;
        }
        if let Some(repo) = repo {
            cmd.arg("--git-dir").arg(repo);
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x0800_0000);
        }
        Ok(cmd)
    }
    // Private to this bounded owner: callers supply only the fixed operations
    // below/metadata/preparation, never a public arbitrary-argv "safe" flag.
    pub(super) fn run(
        &self,
        args: &[&str],
        repo: Option<&Path>,
        remote: Option<&AdmittedRemote>,
    ) -> Result<Output, GitError> {
        let mut command = self.command(repo, remote)?;
        command.args(args);
        let control = crate::git_backend::transfers::current()
            .progress
            .unwrap_or_default()
            .cancellation()
            .clone();
        let output = crate::git_backend::shell::controlled::output(&mut command, &control)
            .map_err(|error| {
                crate::git_backend::shell::controlled::git_error(
                    error,
                    "admitted native Git operation".into(),
                )
            })?;
        if output.status.success() {
            return Ok(output);
        }
        if matches!(remote, Some(AdmittedRemote::Ssh(_)))
            && let Some(error) = ssh::failure(&output.stderr)
        {
            return Err(error);
        }
        // Remote stderr is untrusted and may reflect authorization. Do not
        // publish it from this credential-aware path, even after token scrubbing.
        Err(GitError::CommandFailed {
            cmd: "admitted native Git operation".into(),
            status: output.status.code().unwrap_or(-1),
            stderr:
                "direct Git operation failed; redirects and ambient credential routes are disabled"
                    .into(),
        })
    }
    pub(super) fn transfer(
        &self,
        reporter: &super::super::ShellGit,
        args: &[&str],
        repo: Option<&Path>,
        remote: &AdmittedRemote,
        objects: &Path,
    ) -> Result<Output, GitError> {
        let mut argv = Vec::with_capacity(args.len() + 1);
        argv.extend(args.first());
        argv.push("--progress");
        argv.extend(args.iter().skip(1));
        let mut cmd = self.command(repo, Some(remote))?;
        cmd.args(&argv);
        reporter.run_admitted_transfer(
            AdmittedCommand(cmd, remote.protocol()),
            &argv,
            remote.address(),
            objects,
        )
    }
    /// A controlled conversion probe. No filter command is present in this
    /// owned config; required-only drivers discriminate reserved value strings.
    pub(super) fn filter_probe(&self, args: &[&str], repo: &Path) -> Result<bool, GitError> {
        self.command(Some(repo), None)?
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .map_err(io)
    }
    pub(super) fn config(&self, path: &Path) -> Result<Vec<(String, String)>, GitError> {
        let path = path
            .to_str()
            .ok_or_else(|| unsupported("config path is not UTF-8"))?;
        let result = self.run(
            &[
                "config",
                "--file",
                path,
                "--no-includes",
                "--null",
                "--list",
            ],
            None,
            None,
        )?;
        let mut values = Vec::new();
        for field in result.stdout.split(|b| *b == 0).filter(|s| !s.is_empty()) {
            let value = std::str::from_utf8(field).map_err(|_| unsupported("non-UTF-8 config"))?;
            let (key, value) = value.split_once('\n').unwrap_or((value, ""));
            values.push((key.to_owned(), value.to_owned()));
        }
        Ok(values)
    }
}

#[cfg(test)]
mod tests {
    use super::owned_config;

    #[test]
    #[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-WINDOWS-LAW")]
    fn a_windows_child_may_write_a_path_past_max_path_and_nothing_else_changes() {
        let posix = owned_config(false);
        let windows = owned_config(true);
        // The refusals are the same recipe on both platforms.
        assert_eq!(windows[..posix.len()], posix[..]);
        assert!(posix.contains(&"credential.helper="));
        assert!(posix.contains(&"http.followRedirects=false"));
        // Only Windows is told about `MAX_PATH`, and in the one spelling
        // Git for Windows reads. Without it the staging checkout of a
        // mirrored monorepo dies with `Filename too long` and exit 128.
        assert!(!posix.contains(&"core.longpaths=true"));
        assert_eq!(windows.last(), Some(&"core.longpaths=true"));
    }
}
