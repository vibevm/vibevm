specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-050#example");

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

use common::UserScratch;
use tempfile::TempDir;

const VERSION: &str = "1.0.0";

struct PackageSpec<'a> {
    name: &'a str,
    requires: &'a str,
    marker: &'a str,
}

struct FixtureWorld {
    _temp: TempDir,
    user: UserScratch,
    project: PathBuf,
}

impl FixtureWorld {
    fn new(group: &str, root_requires: &str, packages: &[PackageSpec<'_>]) -> Self {
        let temp = TempDir::new().expect("tempdir");
        let registry = temp.path().join("registry");
        for package in packages {
            write_package(&registry, group, package);
        }

        let user = UserScratch::new();
        let project = temp.path().join("project");
        fs::create_dir_all(&project).expect("create project");
        user.init_project(&project);
        fs::write(
            project.join("vibe.toml"),
            format!(
                "[project]\n\
                 name = \"visibility-lanes\"\n\
                 group = \"{group}\"\n\
                 version = \"0.1.0\"\n\
                 spec_format = \"markdown\"\n\n\
                 [requires.packages]\n\
                 {root_requires}\n\
                 [[registry]]\n\
                 name = \"fixture\"\n\
                 url = \"{}\"\n",
                file_url(&registry)
            ),
        )
        .expect("write project manifest");

        Self {
            _temp: temp,
            user,
            project,
        }
    }

    fn install(&self) {
        let output = self
            .user
            .vibe()
            .args(["--json", "install", "--path"])
            .arg(&self.project)
            .arg("--assume-yes")
            .output()
            .expect("spawn vibe install");
        assert_success("vibe install", &output);
    }

    fn lock(&self) -> String {
        self.read("vibe.lock")
    }

    fn index(&self) -> String {
        self.read("spec/boot/INDEX.md")
    }

    fn static_lane(&self) -> String {
        ["spec/boot/STATIC.md", "spec/boot/STATIC.xml"]
            .iter()
            .filter_map(|path| fs::read_to_string(self.project.join(path)).ok())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn slot(&self, group: &str, name: &str) -> PathBuf {
        self.project
            .join("vibedeps")
            .join(format!("{group}.{name}"))
            .join(VERSION)
    }

    fn slot_boot(&self, group: &str, name: &str) -> String {
        fs::read_to_string(
            self.slot(group, name)
                .join(format!("spec/boot/10-flow-{name}.md")),
        )
        .expect("read materialised boot snippet")
    }

    fn read(&self, path: &str) -> String {
        fs::read_to_string(self.project.join(path)).expect("read project artifact")
    }
}

#[test]
fn private_edge_never_reaches_the_lanes() {
    const GROUP: &str = "org.w3.privateedge";
    const B: &str = "carrier-private-b";
    const W: &str = "hidden-private-w";
    const B_MARKER: &str = "W3_PRIVATE_VISIBLE_B_BOOT_MARKER";
    const W_MARKER: &str = "W3_PRIVATE_HIDDEN_W_BOOT_MARKER";

    let world = FixtureWorld::new(
        GROUP,
        &format!("\"{GROUP}/{B}\" = \"^1\"\n"),
        &[
            PackageSpec {
                name: B,
                requires: &format!(
                    "\n[requires.packages]\n\"{GROUP}/{W}\" = {{ version = \"^1\", access = \"private\" }}\n"
                ),
                marker: B_MARKER,
            },
            PackageSpec {
                name: W,
                requires: "",
                marker: W_MARKER,
            },
        ],
    );
    world.install();

    let lock = world.lock();
    let index = world.index();
    let static_lane = world.static_lane();
    assert!(lock_package_block(&lock, B).is_some(), "B must be locked");
    assert!(
        lock_package_block(&lock, W).is_none(),
        "private W must not be locked"
    );
    assert!(world.slot_boot(GROUP, B).contains(B_MARKER));
    assert!(!world.slot(GROUP, W).exists(), "private W has no slot");
    assert!(index.contains(&slot_slug(GROUP, B)), "B must reach INDEX");
    assert!(
        !index.contains(&slot_slug(GROUP, W)),
        "private W must not reach INDEX"
    );
    assert!(!static_lane.contains(W_MARKER));
    assert!(!static_lane.contains(&format!("{GROUP}/{W}")));
}

#[test]
fn friends_chain_reaches_the_lanes() {
    const GROUP: &str = "org.w3.friends";
    const B: &str = "friend-carrier-b";
    const C: &str = "friend-chain-c";
    const C_MARKER: &str = "W3_FRIENDS_CHAIN_C_BOOT_MARKER";

    let world = FixtureWorld::new(
        GROUP,
        &format!("\"{GROUP}/{B}\" = {{ version = \"^1\", friend = true }}\n"),
        &[
            PackageSpec {
                name: B,
                requires: &format!(
                    "\n[requires.packages]\n\"{GROUP}/{C}\" = {{ version = \"^1\", access = \"friends-only\" }}\n"
                ),
                marker: "W3_FRIENDS_VISIBLE_B_BOOT_MARKER",
            },
            PackageSpec {
                name: C,
                requires: "",
                marker: C_MARKER,
            },
        ],
    );
    world.install();

    let lock = world.lock();
    let c = lock_package_block(&lock, C).expect("C must be locked");
    assert!(c.contains("admitted_by = \"friends-chain\""));
    assert!(world.slot_boot(GROUP, C).contains(C_MARKER));
    assert!(world.index().contains(&slot_slug(GROUP, C)));
}

#[test]
fn static_transitive_does_not_pierce_private() {
    const GROUP: &str = "org.w3.staticprivate";
    const B: &str = "static-carrier-b";
    const W: &str = "static-hidden-w";
    const B_MARKER: &str = "W3_STATIC_VISIBLE_B_BOOT_MARKER";
    const W_MARKER: &str = "W3_STATIC_HIDDEN_W_BOOT_MARKER";

    let world = FixtureWorld::new(
        GROUP,
        &format!("\"{GROUP}/{B}\" = {{ version = \"^1\", link = \"static-transitive\" }}\n"),
        &[
            PackageSpec {
                name: B,
                requires: &format!(
                    "\n[requires.packages]\n\"{GROUP}/{W}\" = {{ version = \"^1\", access = \"private\" }}\n"
                ),
                marker: B_MARKER,
            },
            PackageSpec {
                name: W,
                requires: "",
                marker: W_MARKER,
            },
        ],
    );
    world.install();

    let static_lane = world.static_lane();
    assert!(static_lane.contains(B_MARKER), "visible B must be static");
    assert!(
        !static_lane.contains(W_MARKER),
        "static forcing must not widen visibility"
    );
    assert!(lock_package_block(&world.lock(), W).is_none());
    assert!(!world.slot(GROUP, W).exists());
}

#[test]
fn exclude_prunes_lane_delivery_through_the_edge() {
    let one_path = diamond_world("org.w3.excludeone", false);
    one_path.install();
    let one_lock = one_path.lock();
    assert!(
        lock_package_block(&one_lock, "diamond-d").is_some(),
        "the C path keeps D alive"
    );
    assert!(
        one_path
            .slot_boot("org.w3.excludeone", "diamond-d")
            .contains("W3_EXCLUDE_DIAMOND_D_BOOT_MARKER")
    );
    assert!(
        one_path
            .index()
            .contains(&slot_slug("org.w3.excludeone", "diamond-d"))
    );

    let both_paths = diamond_world("org.w3.excludeboth", true);
    both_paths.install();
    let both_lock = both_paths.lock();
    let both_index = both_paths.index();
    let both_static = both_paths.static_lane();
    assert!(lock_package_block(&both_lock, "diamond-d").is_none());
    assert!(!both_paths.slot("org.w3.excludeboth", "diamond-d").exists());
    assert!(!both_index.contains(&slot_slug("org.w3.excludeboth", "diamond-d")));
    assert!(!both_static.contains("W3_EXCLUDE_DIAMOND_D_BOOT_MARKER"));
}

#[test]
fn root_private_edge_is_the_dev_world() {
    const GROUP: &str = "org.w3.rootprivate";
    const T: &str = "dev-tool-t";
    const T_MARKER: &str = "W3_ROOT_PRIVATE_T_BOOT_MARKER";

    let world = FixtureWorld::new(
        GROUP,
        &format!("\"{GROUP}/{T}\" = {{ version = \"^1\", access = \"private\" }}\n"),
        &[PackageSpec {
            name: T,
            requires: "",
            marker: T_MARKER,
        }],
    );
    world.install();

    let lock = world.lock();
    let t = lock_package_block(&lock, T).expect("root-private T must be locked");
    assert!(t.contains("admitted_by = \"root-edge\""));
    assert!(world.slot_boot(GROUP, T).contains(T_MARKER));
    assert!(world.index().contains(&slot_slug(GROUP, T)));
}

fn diamond_world(group: &str, exclude_both: bool) -> FixtureWorld {
    let d = "diamond-d";
    let c_requirement = if exclude_both {
        format!("\"{group}/diamond-c\" = {{ version = \"^1\", exclude = [\"{group}/{d}\"] }}\n")
    } else {
        format!("\"{group}/diamond-c\" = \"^1\"\n")
    };
    let root_requires = format!(
        "\"{group}/diamond-b\" = {{ version = \"^1\", exclude = [\"{group}/{d}\"] }}\n{c_requirement}"
    );
    let requires_d = format!("\n[requires.packages]\n\"{group}/{d}\" = \"^1\"\n");
    FixtureWorld::new(
        group,
        &root_requires,
        &[
            PackageSpec {
                name: "diamond-b",
                requires: &requires_d,
                marker: "W3_EXCLUDE_DIAMOND_B_BOOT_MARKER",
            },
            PackageSpec {
                name: "diamond-c",
                requires: &requires_d,
                marker: "W3_EXCLUDE_DIAMOND_C_BOOT_MARKER",
            },
            PackageSpec {
                name: d,
                requires: "",
                marker: "W3_EXCLUDE_DIAMOND_D_BOOT_MARKER",
            },
        ],
    )
}

fn write_package(registry: &Path, group: &str, package: &PackageSpec<'_>) {
    let source = registry
        .join(group)
        .join(package.name)
        .join(format!("v{VERSION}"));
    let boot_dir = source.join("spec/boot");
    fs::create_dir_all(&boot_dir).expect("create package boot directory");
    fs::write(
        source.join("vibe.toml"),
        format!(
            "[package]\n\
             group = \"{group}\"\n\
             name = \"{}\"\n\
             kind = \"flow\"\n\
             version = \"{VERSION}\"\n\n\
             [boot_snippet]\n\
             source = \"spec/boot/10-flow-{}.md\"\n\
             category = \"flow\"\n\
             link = \"dynamic\"\n\
             {}",
            package.name, package.name, package.requires
        ),
    )
    .expect("write package manifest");
    fs::write(
        boot_dir.join(format!("10-flow-{}.md", package.name)),
        format!("# {}\n\n{}\n", package.name, package.marker),
    )
    .expect("write package boot snippet");
}

fn file_url(path: &Path) -> String {
    format!("file:///{}", path.to_string_lossy().replace('\\', "/"))
}

fn slot_slug(group: &str, name: &str) -> String {
    format!("{group}.{name}")
}

fn lock_package_block<'a>(lock: &'a str, name: &str) -> Option<&'a str> {
    let needle = format!("name = \"{name}\"");
    lock.split("[[package]]")
        .find(|block| block.lines().any(|line| line.trim() == needle))
}

fn assert_success(command: &str, output: &Output) {
    assert!(
        output.status.success(),
        "{command} failed ({:?})\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
