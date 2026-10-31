//! A local directory registry of application packages for the PROP-065
//! lifecycle fixtures (env steps, modify, requirements, tree removal,
//! update and rollback): packages written as step lists, a scratch user
//! home and HOME, and the `vibe` invocations with their outcome checks.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

use serde_json::Value;

use super::UserScratch;

/// A registry of `org.example/*` application packages, a scratch settings
/// root and a scratch HOME for the startup files env steps edit.
pub struct AppRegistry {
    pub root: tempfile::TempDir,
    pub user: UserScratch,
}

impl Default for AppRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl AppRegistry {
    pub fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("home")).unwrap();
        Self {
            root,
            user: UserScratch::new(),
        }
    }

    pub fn registry(&self) -> PathBuf {
        self.root.path().join("registry")
    }

    /// The scratch HOME every `vibe` run sees.
    pub fn home(&self) -> PathBuf {
        self.root.path().join("home")
    }

    pub fn settings(&self) -> &Path {
        &self.user.settings
    }

    /// Write `org.example/<name>@<version>` in slot `slot`: one installer
    /// option `main` for this host whose body is `option` (steps, launchers,
    /// features …). Returns the package directory.
    pub fn package(&self, name: &str, version: &str, slot: &str, option: &str) -> PathBuf {
        let dir = self
            .registry()
            .join(format!("org.example/{name}/v{version}"));
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("vibe.toml"),
            format!(
                "[package]\nname = \"{name}\"\ngroup = \"org.example\"\nkind = \"app\"\nversion = \"{version}\"\n\n\
                 [application]\nid = \"{slot}\"\n\n\
                 [[installer_options]]\nid = \"main\"\nname = \"Main\"\nplatform = \"linux\"\nplatform_name = \"Linux\"\n\
                 arch = \"{arch}\"\n{option}\n",
                arch = std::env::consts::ARCH,
            ),
        )
        .unwrap();
        dir
    }

    /// Write a package-relative executable script `bin/<command>` printing
    /// `<marker> <args>` under `payload/` of the package directory.
    pub fn script(&self, package: &Path, command: &str, marker: &str) {
        let bin = package.join("payload/bin");
        fs::create_dir_all(&bin).unwrap();
        let path = bin.join(command);
        fs::write(&path, format!("#!/bin/sh\necho \"{marker} $*\"\n")).unwrap();
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    /// Run `vibe args…` in the registry root with the scratch HOME.
    pub fn vibe(&self, args: &[&str]) -> Output {
        let mut command = self.user.vibe();
        command
            .current_dir(self.root.path())
            .env("HOME", self.home())
            .args(args);
        for variable in [
            "VIBE_AGENT_COMMAND",
            "VIBE_AGENT_ARGS",
            "VIBE_INVOKED_BY",
            "VIBE_APP_USE",
        ] {
            command.env_remove(variable);
        }
        command.output().unwrap()
    }

    /// `vibe <verb…> <package> --registry <registry> <extra…>`.
    pub fn app(&self, verb: &[&str], package: &str, extra: &[&str]) -> Output {
        let registry = self.registry();
        let mut args = verb.to_vec();
        args.extend_from_slice(&[package, "--registry", registry.to_str().unwrap()]);
        args.extend_from_slice(extra);
        self.vibe(&args)
    }

    /// The raw application index.
    pub fn index(&self) -> Value {
        serde_json::from_slice(&fs::read(self.settings().join("applications/index.json")).unwrap())
            .unwrap()
    }

    /// One row of `vibe app list --json -g` by key.
    pub fn listed(&self, key: &str) -> Value {
        let list = json(&self.vibe(&["app", "list", "-g", "--json"]));
        list["installations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["key"] == key)
            .cloned()
            .unwrap_or(Value::Null)
    }
}

/// The stdout of a successful run.
pub fn ok(output: &Output) -> String {
    assert!(
        output.status.success(),
        "status={} stdout={} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// The stderr of a failed run.
pub fn failed(output: &Output) -> String {
    assert!(
        !output.status.success(),
        "unexpected success: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    String::from_utf8_lossy(&output.stderr).into_owned()
}

pub fn json(output: &Output) -> Value {
    serde_json::from_str(&ok(output)).unwrap()
}
