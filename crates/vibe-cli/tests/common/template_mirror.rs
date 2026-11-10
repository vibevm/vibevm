//! The template-snapshot fixture `cli_registry_mirror_templates` drives:
//! a `localdir` source carrying the production `org.vibevm.node/node`
//! template at its revision, upstream's frozen documents laid out as
//! upstream lays them out, a `localdir` target to mirror into, and the one
//! `[[remap]]` row that reads `https://nodejs.org/dist/` off this
//! filesystem — so no run opens a socket (PROP-066 `#locator`).
//!
//! The version list is a **four-record** subset of upstream's own index,
//! taken from the same frozen document P3's render-fidelity suite uses:
//! every version a snapshot selects is fully rendered, so the set must be
//! the versions whose digest documents the fixtures carry. P3's
//! `cli_template_node` keeps the whole 410-record window for the
//! questions that need it.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

use super::UserScratch;

pub const TEMPLATE: &str = include_str!("../../../vibe-template/tests/fixtures/node/vibe.toml");
pub const REVISION: &str = "1.0.0";
pub const PACKAGE: &str = "org.vibevm.node/node";
const ORIGIN: &str = "https://nodejs.org/dist/";

/// The frozen digest documents of the four versions this suite selects
/// from, upstream's own bytes.
pub const DIGESTS: [(&str, &str); 4] = [
    (
        "26.13.0",
        include_str!("../../../vibe-template/tests/fixtures/node/SHASUMS256-26.13.0.txt"),
    ),
    (
        "24.22.2",
        include_str!("../../../vibe-template/tests/fixtures/node/SHASUMS256-24.22.2.txt"),
    ),
    (
        "22.23.4",
        include_str!("../../../vibe-template/tests/fixtures/node/SHASUMS256-22.23.4.txt"),
    ),
    (
        "12.0.0",
        include_str!("../../../vibe-template/tests/fixtures/node/SHASUMS256-12.0.0.txt"),
    ),
];

/// The four records, verbatim from the frozen index, newest first as
/// upstream writes them. `lts` is upstream's own: three of the four are
/// LTS lines and `12.0.0` is not, which is what makes `channels = ["lts"]`
/// decide something here.
const INDEX: &str = r#"[
{"version":"v26.13.0","date":"2026-10-28","files":["linux-x64","osx-arm64-tar","win-x64-zip"],"npm":"11.6.2","v8":"14.4.253.26","uv":"1.52.0","zlib":"1.3.1","openssl":"3.5.4","modules":"140","lts":"Lithium","security":false},
{"version":"v24.22.2","date":"2026-10-14","files":["linux-x64","osx-arm64-tar","win-x64-zip"],"npm":"11.6.1","v8":"13.6.233.10","uv":"1.51.0","zlib":"1.3.1","openssl":"3.0.17","modules":"137","lts":"Krypton","security":false},
{"version":"v22.23.4","date":"2026-09-30","files":["linux-x64","osx-arm64-tar","win-x64-zip"],"npm":"10.9.4","v8":"12.4.254.21","uv":"1.51.0","zlib":"1.3.1","openssl":"3.0.17","modules":"127","lts":"Jod","security":false},
{"version":"v12.0.0","date":"2019-04-23","files":["linux-x64","win-x64-zip"],"npm":"6.9.0","v8":"7.4.288.21","uv":"1.28.0","zlib":"1.2.11","openssl":"1.1.1b","modules":"72","lts":false,"security":false}
]
"#;

/// A source `localdir` registry carrying the template at its revision,
/// the frozen upstream documents laid out as upstream lays them out, and
/// a target `localdir` to mirror into.
pub struct Fixture {
    pub root: tempfile::TempDir,
}

impl Fixture {
    pub fn new() -> Self {
        let fixture = Fixture {
            root: tempfile::tempdir().unwrap(),
        };
        let dir = fixture.source().join(PACKAGE).join(format!("v{REVISION}"));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("vibe.toml"), TEMPLATE).unwrap();
        fs::write(dir.join("README.md"), "# Node.js, from a template\n").unwrap();
        fs::create_dir_all(fixture.target()).unwrap();
        fixture.write_upstream();
        fs::write(
            fixture.root.path().join("remap.toml"),
            format!(
                "[[remap]]\nprefix = \"{ORIGIN}\"\npath = \"{}\"\n\
                 reason = \"the suite's frozen copy of the Node.js release host\"\n",
                fixture.dist().display()
            ),
        )
        .unwrap();
        fixture
    }

    pub fn source(&self) -> PathBuf {
        self.root.path().join("source")
    }

    pub fn target(&self) -> PathBuf {
        self.root.path().join("target")
    }

    pub fn dist(&self) -> PathBuf {
        self.root.path().join("dist")
    }

    /// `index.json` and `v<V>/SHASUMS256.txt`, mirrored by path.
    fn write_upstream(&self) {
        let dist = self.dist();
        fs::create_dir_all(&dist).unwrap();
        fs::write(dist.join("index.json"), INDEX).unwrap();
        for (version, document) in DIGESTS {
            let release = dist.join(format!("v{version}"));
            fs::create_dir_all(&release).unwrap();
            fs::write(release.join("SHASUMS256.txt"), document).unwrap();
        }
    }

    /// The project whose registry layers name the source and the target.
    pub fn project(&self) -> PathBuf {
        let project = self.root.path().join("project");
        fs::create_dir_all(&project).unwrap();
        fs::write(
            project.join("vibe.toml"),
            format!(
                "[project]\nname='consumer'\nversion='0.1.0'\n\
                 [[registry]]\nname='src'\ntype='localdir'\nurl='file://{}'\n\
                 [[registry]]\nname='dst'\ntype='localdir'\nurl='file://{}'\n",
                self.source().display(),
                self.target().display()
            ),
        )
        .unwrap();
        project
    }

    /// A project that can see **only** the mirrored target — the closed
    /// network's own view.
    pub fn closed_project(&self) -> PathBuf {
        let project = self.root.path().join("closed");
        fs::create_dir_all(&project).unwrap();
        fs::write(
            project.join("vibe.toml"),
            format!(
                "[project]\nname='inside'\nversion='0.1.0'\n\
                 [[registry]]\nname='dst'\ntype='localdir'\nurl='file://{}'\n",
                self.target().display()
            ),
        )
        .unwrap();
        project
    }

    pub fn config(&self, body: &str) -> PathBuf {
        let path = self.root.path().join("mirror.toml");
        fs::write(&path, body).unwrap();
        path
    }

    /// The snapshot directory inside the target.
    pub fn snapshot(&self) -> PathBuf {
        self.target().join("_index/templates").join(PACKAGE)
    }
}

pub fn vibe(user: &UserScratch, project: &Path, args: &[&str]) -> Output {
    user.vibe()
        .current_dir(project)
        .env("VIBE_NO_DEFAULT_REGISTRY", "1")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .args(args)
        .output()
        .unwrap()
}

pub fn ok(output: &Output) -> String {
    assert!(
        output.status.success(),
        "expected success:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).to_string()
}

pub fn refusal(output: &Output) -> String {
    assert!(
        !output.status.success(),
        "expected a refusal:\n{}",
        String::from_utf8_lossy(&output.stdout)
    );
    String::from_utf8_lossy(&output.stderr).to_string()
}

/// Mirror with `--config`, through the remap row that reads upstream off
/// disk.
pub fn mirror(user: &UserScratch, fixture: &Fixture, config: &Path, extra: &[&str]) -> Output {
    let remap = fixture.root.path().join("remap.toml");
    let project = fixture.project();
    let mut args: Vec<&str> = vec![
        "--remap",
        remap.to_str().unwrap(),
        "--json",
        "registry",
        "mirror",
        "src",
        "--into",
        "dst",
        "--config",
        config.to_str().unwrap(),
    ];
    args.extend_from_slice(extra);
    vibe(user, &project, &args)
}

/// `[[version]]` rows of the written snapshot, in file order.
pub fn snapshot_versions(fixture: &Fixture) -> Vec<String> {
    let record: toml::Value =
        toml::from_str(&fs::read_to_string(fixture.snapshot().join("snapshot.toml")).unwrap())
            .unwrap();
    record["version"]
        .as_array()
        .map(|rows| {
            rows.iter()
                .map(|row| row["version"].as_str().unwrap().to_string())
                .collect()
        })
        .unwrap_or_default()
}

/// `([sync] first, [sync] last)` of the written snapshot.
pub fn sync_last(fixture: &Fixture) -> (String, String) {
    let record: toml::Value =
        toml::from_str(&fs::read_to_string(fixture.snapshot().join("snapshot.toml")).unwrap())
            .unwrap();
    (
        record["sync"]["first"].as_str().unwrap().to_string(),
        record["sync"]["last"].as_str().unwrap().to_string(),
    )
}

/// Every file under `dir`, recursively.
pub fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return found;
    };
    for entry in entries.flatten() {
        if entry.path().is_dir() {
            found.extend(walk(&entry.path()));
        } else {
            found.push(entry.path());
        }
    }
    found
}
