//! The fake coding agent of the PROP-064 acceptance fixture: `fake-agent.sh`
//! (and its PowerShell twin `fake-agent.ps1`) beside this module record what
//! a hook was given and answer as told. A test registers it through
//! `VIBE_AGENT_COMMAND` / `VIBE_AGENT_ARGS` or as an `agents.toml`
//! definition, and reads the record back from here.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

use assert_cmd::Command;

use serde_json::Value;

use super::UserScratch;

/// The POSIX script, started as `sh <script>`.
pub fn script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/common/fake-agent.sh")
}

/// The PowerShell twin, started as `pwsh -NoProfile -File <script>`.
pub fn powershell_script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/common/fake-agent.ps1")
}

/// What one fake agent recorded.
pub struct Record {
    pub dir: PathBuf,
}

impl Record {
    pub fn new(dir: &Path) -> Self {
        Self {
            dir: dir.to_path_buf(),
        }
    }

    /// How many times the agent ran.
    pub fn calls(&self) -> usize {
        fs::read_to_string(self.dir.join("calls"))
            .map(|text| text.lines().count())
            .unwrap_or(0)
    }

    /// The arguments of the last run, one per element.
    pub fn args(&self) -> Vec<String> {
        self.read("args").lines().map(str::to_owned).collect()
    }

    /// One recorded variable of the last run (`""` when unset).
    pub fn env(&self, name: &str) -> String {
        self.read("env")
            .lines()
            .find_map(|line| line.strip_prefix(&format!("{name}=")).map(str::to_owned))
            .unwrap_or_default()
    }

    pub fn read(&self, file: &str) -> String {
        fs::read_to_string(self.dir.join(file)).unwrap_or_default()
    }
}

// ---- The agent-hook fixture package and its command helpers -------------

pub const NAME: &str = "org.example/hooked";
pub const KEY: &str = "org.example/hooked@1.0.0";
pub const WORK: &str = "cache/app-work/org.example.hooked@1.0.0";

pub struct Fixture {
    pub root: tempfile::TempDir,
    pub user: UserScratch,
    pub registry: PathBuf,
    pub log: PathBuf,
    pub record: Record,
}

impl Fixture {
    /// The package: a copy step laying out the payload, the agent step
    /// `fix` (with `extra` appended to its table), then a shell step that
    /// logs `AFTER`.
    pub fn new(extra: &str) -> Self {
        let root = tempfile::tempdir().unwrap();
        let registry = root.path().join("registry");
        let log = root.path().join("steps.log");
        let package = registry.join("org.example/hooked/v1.0.0");
        fs::create_dir_all(package.join("bin")).unwrap();
        fs::write(package.join("bin/tool"), "#!/bin/sh\necho tool\n").unwrap();
        fs::write(
            package.join("vibe.toml"),
            format!(
                "[package]\nname = \"hooked\"\ngroup = \"org.example\"\nkind = \"app\"\nversion = \"1.0.0\"\n\n\
                 [application]\nid = \"hooked\"\n\n[[installer_options]]\nid = \"main\"\nname = \"Main\"\n\
                 platform = \"linux\"\nplatform_name = \"Linux\"\narch = \"{arch}\"\n\n\
                 [[installer_options.steps]]\nid = \"payload\"\ntitle = \"Lay out the payload\"\nkind = \"copy\"\nfrom = \"bin\"\nto = \"bin\"\n\n\
                 [[installer_options.steps]]\nid = \"fix\"\ntitle = \"Let an agent adapt the payload\"\nkind = \"agent\"\n\
                 description = \"the payload needs a host-specific touch\"\n\
                 prompt = \"\"\"\nAdapt the payload in $out.\nKeep $$HOME untouched.\n\"\"\"\n{extra}\n\
                 [[installer_options.steps]]\nid = \"after\"\ntitle = \"Log the order\"\nkind = \"shell\"\n\
                 command = \"echo AFTER >> '{log}'\"\n",
                arch = std::env::consts::ARCH,
                log = log.display(),
            ),
        )
        .unwrap();
        let record = Record::new(&root.path().join("agent-record"));
        Self {
            root,
            user: UserScratch::new(),
            registry,
            log,
            record,
        }
    }

    /// A `vibe` command with no agent posture inherited from the caller.
    pub fn command(&self, args: &[&str]) -> Command {
        let mut command = self.user.vibe();
        command.current_dir(self.root.path()).args(args);
        for variable in [
            "VIBE_AGENT_COMMAND",
            "VIBE_AGENT_ARGS",
            "VIBE_INVOKED_BY",
            "VIBE_UNATTENDED",
            "FAKE_AGENT_STATUS",
            "FAKE_AGENT_EXIT",
            "FAKE_AGENT_NAME",
        ] {
            command.env_remove(variable);
        }
        command.env("FAKE_AGENT_RECORD", &self.record.dir);
        command
    }

    /// `vibe <verb> hooked --registry R <extra>`.
    pub fn app(&self, verb: &[&str], extra: &[&str]) -> Command {
        let registry = self.registry.to_str().unwrap();
        let mut args = verb.to_vec();
        args.extend_from_slice(&[NAME, "--registry", registry]);
        args.extend_from_slice(extra);
        self.command(&args)
    }

    pub fn work(&self) -> PathBuf {
        self.user.settings.join(WORK)
    }

    pub fn log(&self) -> String {
        fs::read_to_string(&self.log).unwrap_or_default()
    }

    pub fn index(&self) -> Value {
        serde_json::from_slice(
            &fs::read(self.user.settings.join("applications/index.json")).unwrap(),
        )
        .unwrap()
    }
}

/// Register the fake agent as the external command of this process.
pub fn registered(mut command: Command) -> Command {
    command.env("VIBE_AGENT_COMMAND", "sh").env(
        "VIBE_AGENT_ARGS",
        serde_json::to_string(&[script().display().to_string()]).unwrap(),
    );
    command
}

pub fn run(mut command: Command) -> Output {
    command.output().unwrap()
}

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

/// The `{"ok":false,…}` envelope of a `--json` command, with its exit.
pub fn refused(output: &Output) -> (Option<i32>, Value) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let envelope = stderr
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .find(|value| value["ok"] == false)
        .unwrap_or_else(|| panic!("no error envelope in: {stderr}"));
    (output.status.code(), envelope)
}

pub fn steps_json(dir: &Path) -> Value {
    serde_json::from_slice(&fs::read(dir.join("steps.json")).unwrap()).unwrap()
}

pub fn step<'a>(state: &'a Value, id: &str) -> &'a Value {
    state["steps"]
        .as_array()
        .unwrap()
        .iter()
        .find(|step| step["id"] == id)
        .unwrap_or_else(|| panic!("no step `{id}` in {state}"))
}

/// The newest run record directory of the key.
pub fn newest_record(fixture: &Fixture) -> PathBuf {
    let runs = fixture
        .user
        .settings
        .join("cache/app-runs/org.example.hooked@1.0.0");
    let mut dirs: Vec<PathBuf> = fs::read_dir(runs)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    dirs.sort();
    dirs.pop().unwrap()
}
