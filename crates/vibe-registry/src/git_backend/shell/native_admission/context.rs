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
            .env("PATH", &self.profile.installation.helpers)
            .env("GIT_EXEC_PATH", &self.profile.installation.helpers)
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
                    None => "",
                },
            )
            .env("GIT_TEMPLATE_DIR", root.join("empty"))
            .stdin(Stdio::null());
        // All entries originate here, never from inherited -c/env/config input.
        for value in [
            "credential.helper=",
            "core.askPass=",
            "http.proxy=",
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
        ] {
            cmd.args(["-c", value]);
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
