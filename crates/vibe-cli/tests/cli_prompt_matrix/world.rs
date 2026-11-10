//! The matrix's world: a hermetic project with a real static lane, built
//! from a directory registry, plus an independent reader of the published
//! boot contract.
//!
//! The reader here is a second implementation of the one in
//! `crates/vibe-workspace/tests/system_prompt_parity/classic.rs`, and
//! deliberately so: this crate cannot reach that module, and a reference
//! shared with the code under test is the one thing `PLAN.md` P7's
//! construction rule forbids. `STATIC.*` is chosen by two `exists` calls,
//! `INDEX.md` is parsed as the TOML document the boot block says it is, and
//! the lane's markers are split by a plain line scan.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use crate::common::{self, UserScratch};

/// The authored frame every opted-in project here writes.
pub const TEMPLATE: &str = "# My frame\n\nTEMPLATE-CANARY-7a31\n";

/// A canary in the entrypoint's authored prose, outside the managed block.
pub const ENTRYPOINT_CANARY: &str = "ENTRYPOINT-CANARY-5c08";

/// A canary in a user-local file the authored boot text only *mentions*.
/// Nothing vibe generates may carry it (`#SYS-NO-HOME-EXPANSION`).
pub const USER_LOCAL_CANARY: &str = "USER-LOCAL-CANARY-b914";

/// A canary in the central project's own frame. A worker scoped to another
/// node must not receive it.
pub const CENTRAL_CANARY: &str = "CENTRAL-CANARY-2f66";

/// What a world declares.
#[derive(Clone)]
pub struct Recipe {
    /// `spec_format` for the project: decides the `STATIC.*` spelling.
    pub format: &'static str,
    /// Extra lines appended to the root `[project]` manifest.
    pub manifest_extra: String,
    /// Give the carrier package `os:`-gated fragments, one per platform.
    pub gated: bool,
    /// Give the carrier package an `installed:`-gated fragment naming a
    /// package the resolution does/does not carry.
    pub installed_gate: Option<bool>,
    /// Opt in by writing the authored template.
    pub opted_in: bool,
    /// Add a workspace member node, itself opted in.
    pub member: bool,
    /// Plant user-local files whose contents must reach no artifact.
    pub sentinels: bool,
}

impl Default for Recipe {
    fn default() -> Self {
        Self {
            format: "markdown",
            manifest_extra: String::new(),
            gated: false,
            installed_gate: None,
            opted_in: true,
            member: false,
            sentinels: false,
        }
    }
}

/// The member node's workspace-relative path.
pub const MEMBER_REL: &str = "pkg/member";

/// One installed project under an isolated home.
pub struct World {
    pub user: UserScratch,
    pub temp: tempfile::TempDir,
    pub root: PathBuf,
    pub format: &'static str,
}

impl World {
    /// Build the project, the registry beside it, and install.
    pub fn build(recipe: &Recipe) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("project");
        let registry = root.join("registry");
        let user = UserScratch::new();

        let mut fragments = String::new();
        if recipe.gated {
            // One **dynamically** linked package per platform, because only
            // a dynamic contribution's residual `os:` gate survives into an
            // INDEX row and so into band G. A static link's gated fragment
            // is decided while the lane is compiled and never reaches the
            // artifact a session reads (`#SYS-SESSION-TARGET`).
            for os in ["linux", "macos", "windows"] {
                write(
                    &registry.join(format!("org.vibevm/gated-{os}/v1.0.0")),
                    "vibe.toml",
                    &format!(
                        "[package]\ngroup = \"org.vibevm\"\nname = \"gated-{os}\"\n\
                         kind = \"flow\"\nversion = \"1.0.0\"\nformat = \"simple\"\n\n\
                         [boot_snippet]\nsource = \"boot/{os}.md\"\nwhen = \"os:{os}\"\n"
                    ),
                );
                write(
                    &registry.join(format!("org.vibevm/gated-{os}/v1.0.0")),
                    format!("boot/{os}.md"),
                    &format!("# {os}\n\n{}-GATED-BYTES\n", os.to_uppercase()),
                );
            }
        }
        if recipe.installed_gate.is_some() {
            fragments.push_str(
                "[[boot_snippet.fragment]]\nsource = \"boot/companioned.md\"\n\
                 when = \"installed:org.vibevm/companion\"\n",
            );
            write(
                &registry.join("org.vibevm/carrier/v1.0.0"),
                "boot/companioned.md",
                "# companioned\n\nINSTALLED-GATED-BYTES\n",
            );
        }
        write(
            &registry.join("org.vibevm/carrier/v1.0.0"),
            "vibe.toml",
            &format!(
                "[package]\ngroup = \"org.vibevm\"\nname = \"carrier\"\nkind = \"flow\"\n\
                 version = \"1.0.0\"\nformat = \"simple\"\n\n\
                 [boot_snippet]\nsource = \"boot/main.md\"\n{fragments}"
            ),
        );
        write(
            &registry.join("org.vibevm/carrier/v1.0.0"),
            "boot/main.md",
            "# carrier {#root}\n\nCARRIER-LANE-BYTES\n",
        );
        write(
            &registry.join("org.vibevm/companion/v1.0.0"),
            "vibe.toml",
            "[package]\ngroup = \"org.vibevm\"\nname = \"companion\"\nkind = \"flow\"\n\
             version = \"1.0.0\"\nformat = \"simple\"\n\n\
             [boot_snippet]\nsource = \"boot/companion.md\"\n",
        );
        write(
            &registry.join("org.vibevm/companion/v1.0.0"),
            "boot/companion.md",
            "# companion {#companion}\n\nCOMPANION-LANE-BYTES\n",
        );

        let mut requires =
            "\"org.vibevm/carrier\" = { version = \"=1.0.0\", link = \"static\" }\n".to_owned();
        if recipe.installed_gate == Some(true) {
            requires.push_str(
                "\"org.vibevm/companion\" = { version = \"=1.0.0\", link = \"static\" }\n",
            );
        }
        if recipe.gated {
            for os in ["linux", "macos", "windows"] {
                requires.push_str(&format!(
                    "\"org.vibevm/gated-{os}\" = {{ version = \"=1.0.0\", link = \"dynamic\" }}\n"
                ));
            }
        }
        let members = if recipe.member {
            format!("\n[workspace]\nmembers = [\"{MEMBER_REL}\"]\n")
        } else {
            String::new()
        };
        write(
            &root,
            "vibe.toml",
            &format!(
                "[project]\ngroup = \"org.demo\"\nname = \"demo\"\nversion = \"0.1.0\"\n\
                 spec_format = \"{}\"\n{members}\n[requires.packages]\n{requires}\
                 \n[[registry]]\nname = \"local\"\nurl = \"file://{}\"\n{}",
                recipe.format,
                registry.display(),
                recipe.manifest_extra
            ),
        );
        write(
            &root,
            common::boot_rel("00-core.md"),
            "# own core\n\nOWN-CORE-BYTES\n",
        );
        write(
            &root,
            common::boot_rel("90-user.md"),
            "# own user\n\nOWN-USER-BYTES\n",
        );
        write(
            &root,
            "AGENTS.md",
            &format!("# demo\n\nAuthored prose: {ENTRYPOINT_CANARY}\n"),
        );
        if recipe.opted_in {
            write(
                &root,
                common::boot_rel("SYSTEM-TEMPLATE.md"),
                &format!("{TEMPLATE}\n{CENTRAL_CANARY}\n"),
            );
        }
        if recipe.sentinels {
            write(
                &root,
                common::boot_rel("50-mentions.md"),
                "# mentions\n\nResolve `~/.vibe/steward/contexts/*/plan.toml` and \
                 `~/.vibe/zai.api.token` before starting.\n",
            );
            write(&root, "home/.vibe/steward/plan.toml", USER_LOCAL_CANARY);
            write(&root, "home/.vibe/steward/custody.toml", USER_LOCAL_CANARY);
            write(&root, "home/.vibe/zai.api.token", USER_LOCAL_CANARY);
        }
        if recipe.member {
            let member = root.join(MEMBER_REL);
            write(
                &member,
                "vibe.toml",
                "[package]\ngroup = \"org.demo\"\nname = \"member\"\nkind = \"flow\"\n\
                 version = \"0.1.0\"\n",
            );
            write(
                &member,
                common::boot_rel("00-member.md"),
                "# member\n\nMEMBER-OWN-BYTES\n",
            );
            write(&member, "AGENTS.md", "# member\n\nMember prose.\n");
            write(
                &member,
                common::boot_rel("SYSTEM-TEMPLATE.md"),
                "# Member frame\n\nMEMBER-TEMPLATE-CANARY-8d42\n",
            );
        }

        let world = Self {
            user,
            temp,
            root,
            format: recipe.format,
        };
        world.install();
        world
    }

    /// `vibe install`, offline, under the project's own home.
    pub fn install(&self) {
        self.vibe()
            .args(["install", "--offline", "--assume-yes", "--path"])
            .arg(&self.root)
            .assert()
            .success();
    }

    /// A `vibe` invocation with this world's isolated home.
    pub fn vibe(&self) -> assert_cmd::Command {
        let mut command = self.user.vibe();
        command
            .current_dir(&self.root)
            .env("HOME", self.root.join("home"))
            .env("USERPROFILE", self.root.join("home"));
        command
    }

    /// `vibe prompt <verb> --path <root>`, plus whatever the caller adds.
    pub fn prompt(&self, verb: &str) -> assert_cmd::Command {
        let mut command = self.vibe();
        command.args(["prompt", verb, "--path"]).arg(&self.root);
        command
    }

    /// One path under the root node's boot directory.
    pub fn boot(&self, name: &str) -> PathBuf {
        self.root.join(common::boot_dir()).join(name)
    }

    /// The bytes of one generated artifact.
    pub fn artifact(&self, name: &str) -> Vec<u8> {
        fs::read(self.boot(name)).unwrap_or_else(|error| panic!("`{name}`: {error}"))
    }

    /// The published boot contract of the root node, read independently.
    pub fn classic(&self) -> Classic {
        Classic::read(&self.root, "AGENTS.md")
    }
}

/// Write one file under `root`, creating its parents.
pub fn write(root: &Path, rel: impl AsRef<Path>, body: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, body).unwrap();
}

/// One `[[entry]]` row of `INDEX.md`, verbatim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub path: String,
    pub kind: String,
    pub when: Option<String>,
}

/// The boot a session performs today, read from the published artifacts.
#[derive(Debug, Clone)]
pub struct Classic {
    pub entrypoint: Vec<u8>,
    pub lane: Option<(String, Vec<u8>)>,
    pub rows: Vec<Row>,
}

impl Classic {
    pub fn read(root: &Path, entrypoint_rel: &str) -> Self {
        let boot = root.join(common::boot_dir());
        let xml = boot.join(vibe_core::layout::STATIC_XML);
        let md = boot.join(vibe_core::layout::STATIC_MD);
        let lane = match (xml.exists(), md.exists()) {
            (true, true) => panic!("both `STATIC` spellings: the generator owns one"),
            (true, false) => Some(xml),
            (false, true) => Some(md),
            (false, false) => None,
        };
        let index = fs::read_to_string(boot.join(vibe_core::layout::INDEX_MD)).unwrap();
        let document: toml::Value =
            toml::from_str(&index).expect("a generated INDEX is a TOML document");
        let rows = document
            .get("entry")
            .and_then(|entries| entries.as_array())
            .map(|entries| {
                entries
                    .iter()
                    .map(|entry| Row {
                        path: entry["path"].as_str().unwrap().to_owned(),
                        kind: entry["kind"].as_str().unwrap().to_owned(),
                        when: entry
                            .get("when")
                            .map(|when| when.as_str().unwrap().to_owned()),
                    })
                    .collect()
            })
            .unwrap_or_default();
        Self {
            entrypoint: fs::read(root.join(entrypoint_rel)).unwrap(),
            lane: lane.map(|path| {
                (
                    relative(root, &path),
                    fs::read(&path).expect("the published lane"),
                )
            }),
            rows,
        }
    }

    /// The rows a session reads unconditionally — band U's identity and
    /// order, from the artifact itself.
    pub fn unconditional(&self) -> Vec<&Row> {
        self.rows.iter().filter(|row| row.when.is_none()).collect()
    }

    /// What a launch delivery must carry: `STATIC.*` then band U, each file's
    /// bytes with at most the one LF a marker needs to own its line.
    pub fn delivery(&self, root: &Path) -> Vec<(String, Vec<u8>)> {
        let mut expected = Vec::new();
        if let Some((path, bytes)) = &self.lane {
            expected.push((path.clone(), terminated(bytes.clone())));
        }
        for row in self.unconditional() {
            expected.push((
                row.path.clone(),
                terminated(fs::read(root.join(&row.path)).expect("an INDEX row's file")),
            ));
        }
        expected
    }
}

/// A body as it is delivered between two markers: the file's bytes, plus the
/// one LF the composer inserts when the body does not carry its own.
pub fn terminated(mut bytes: Vec<u8>) -> Vec<u8> {
    if !bytes.is_empty() && !bytes.ends_with(b"\n") {
        bytes.push(b'\n');
    }
    bytes
}

/// Split a delivered lane into `(path, body)` pairs by scanning its marker
/// lines, with no help from `verify_lane_markers`.
pub fn split_lane(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
    let text = std::str::from_utf8(bytes).expect("a composed lane is UTF-8");
    let mut out = Vec::new();
    let mut open: Option<(String, usize)> = None;
    let mut offset = 0usize;
    for line in text.split_inclusive('\n') {
        let next = offset + line.len();
        if let Some(path) = field(line, "vibe:lane-begin") {
            assert!(open.is_none(), "a begin line inside an open segment");
            open = Some((path, next));
        } else if let Some(path) = field(line, "vibe:lane-end") {
            let (begun, start) = open.take().expect("an end line with no begin");
            assert_eq!(begun, path, "a pair whose lines name two files");
            out.push((path, bytes[start..offset].to_vec()));
        }
        offset = next;
    }
    assert!(open.is_none(), "a begin line with no end line");
    out
}

/// The path a marker line names, when the line is one of that keyword.
fn field(line: &str, keyword: &str) -> Option<String> {
    let inner = line
        .trim()
        .strip_prefix("<!-- ")?
        .strip_suffix(" -->")?
        .strip_prefix(keyword)?;
    let fields: Vec<&str> = inner.split_whitespace().collect();
    assert_eq!(fields.len(), 3, "a marker line's fields: {line}");
    Some(fields[1].to_owned())
}

/// A path under `root`, forward-slashed the way a generated artifact spells
/// it.
pub fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .expect("a path under the root")
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}
