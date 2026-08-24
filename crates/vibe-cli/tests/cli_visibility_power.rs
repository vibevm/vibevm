specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-050#override");

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;

use common::UserScratch;
use tempfile::TempDir;

const VERSION: &str = "1.0.0";
const ROOT_NAME: &str = "power-root";

struct PackageSpec<'a> {
    name: &'a str,
    manifest_tail: &'a str,
    marker: &'a str,
}

struct FixtureWorld {
    _temp: TempDir,
    user: UserScratch,
    project: PathBuf,
}

impl FixtureWorld {
    fn new(group: &str, root_tail: &str, packages: &[PackageSpec<'_>]) -> Self {
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
                 name = \"{ROOT_NAME}\"\n\
                 group = \"{group}\"\n\
                 version = \"0.1.0\"\n\
                 spec_format = \"markdown\"\n\
                 {root_tail}\n\
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
        self.install_output(true);
    }

    fn install_output(&self, json: bool) -> Output {
        let mut command = self.user.vibe();
        if json {
            command.arg("--json");
        }
        let output = command
            .args(["install", "--path"])
            .arg(&self.project)
            .arg("--assume-yes")
            .output()
            .expect("spawn vibe install");
        assert_success("vibe install", &output);
        output
    }

    fn lock(&self) -> String {
        self.read("vibe.lock")
    }

    fn index(&self) -> String {
        self.read(&common::index_rel())
    }

    fn static_lane(&self) -> String {
        [common::static_md_rel(), common::static_xml_rel()]
            .iter()
            .filter_map(|path| fs::read_to_string(self.project.join(path)).ok())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn slot(&self, group: &str, name: &str) -> PathBuf {
        self.project
            .join(common::deps_root())
            .join(format!("{group}.{name}"))
            .join(VERSION)
    }

    fn slot_boot(&self, group: &str, name: &str) -> String {
        fs::read_to_string(
            self.slot(group, name)
                .join(common::boot_dir())
                .join(format!("10-flow-{name}.md")),
        )
        .expect("read materialised boot snippet")
    }

    fn read(&self, path: &str) -> String {
        fs::read_to_string(self.project.join(path)).expect("read project artifact")
    }
}

#[test]
fn root_override_opens_a_foreign_private_edge_e2e() {
    const GROUP: &str = "org.w6.power.root";
    const B: &str = "carrier-b";
    const W: &str = "hidden-w";
    const W_MARKER: &str = "W6_ROOT_OVERRIDE_W_BOOT_MARKER";

    let world = private_edge_world(GROUP, B, W, W_MARKER, true);
    world.install();

    let lock = world.lock();
    let w = lock_package_block(&lock, W).expect("override must admit W into the lock");
    assert!(w.contains(&format!("via_override = \"{GROUP}/{ROOT_NAME}\"")));
    assert_present(&world, GROUP, W, W_MARKER);
}

#[test]
fn midgraph_override_reshapes_for_its_consumers() {
    const GROUP: &str = "org.w6.power.midgraph";
    const A: &str = "outer-a";
    const B: &str = "carrier-b";
    const W: &str = "hidden-w";
    const W_MARKER: &str = "W6_MIDGRAPH_OVERRIDE_W_BOOT_MARKER";

    let root_tail = requires_one(GROUP, A, "{ version = \"^1\" }");
    let a_tail = format!(
        "\n[requires.packages]\n\"{GROUP}/{B}\" = \"^1\"\n\n\
         [override]\n\"{GROUP}/{B} -> {GROUP}/{W}\" = {{ access = \"public\" }}\n"
    );
    let b_tail = private_requirement(GROUP, W);
    let world = FixtureWorld::new(
        GROUP,
        &root_tail,
        &[
            PackageSpec {
                name: A,
                manifest_tail: &a_tail,
                marker: "W6_MIDGRAPH_A_BOOT_MARKER",
            },
            PackageSpec {
                name: B,
                manifest_tail: &b_tail,
                marker: "W6_MIDGRAPH_B_BOOT_MARKER",
            },
            PackageSpec {
                name: W,
                manifest_tail: "",
                marker: W_MARKER,
            },
        ],
    );
    world.install();

    let lock = world.lock();
    let w = lock_package_block(&lock, W).expect("A's override must admit W");
    assert!(w.contains(&format!("via_override = \"{GROUP}/{A}\"")));
    assert_present(&world, GROUP, W, W_MARKER);
}

#[test]
fn root_override_beats_midgraph() {
    const GROUP: &str = "org.w6.power.precedence";
    const A: &str = "outer-a";
    const B: &str = "carrier-b";
    const W: &str = "hidden-w";
    const W_MARKER: &str = "W6_PRECEDENCE_W_BOOT_MARKER";

    let root_tail = format!(
        "{}\n[override]\n\"{GROUP}/{B} -> {GROUP}/{W}\" = {{ exclude = true }}\n",
        requires_one(GROUP, A, "{ version = \"^1\" }")
    );
    let a_tail = format!(
        "\n[requires.packages]\n\"{GROUP}/{B}\" = \"^1\"\n\n\
         [override]\n\"{GROUP}/{B} -> {GROUP}/{W}\" = {{ access = \"public\" }}\n"
    );
    let b_tail = private_requirement(GROUP, W);
    let world = FixtureWorld::new(
        GROUP,
        &root_tail,
        &[
            PackageSpec {
                name: A,
                manifest_tail: &a_tail,
                marker: "W6_PRECEDENCE_A_BOOT_MARKER",
            },
            PackageSpec {
                name: B,
                manifest_tail: &b_tail,
                marker: "W6_PRECEDENCE_B_BOOT_MARKER",
            },
            PackageSpec {
                name: W,
                manifest_tail: "",
                marker: W_MARKER,
            },
        ],
    );
    world.install();

    assert_absent(&world, GROUP, W, W_MARKER);
}

#[test]
fn override_node_entry_unseals_allow_friends_e2e() {
    // The seal gates FRIENDSHIP, not delivery (PROP-050 §2.8): P freely
    // delivers G to its circle (P could have marked the edge public), but a
    // rejected grant keeps G out of C(R), so G's own friends-only inner
    // content stays shut. The observable difference is H, never G.
    const GROUP: &str = "org.w6.power.unseal";
    const P: &str = "friend-p";
    const G: &str = "sealed-g";
    const H: &str = "inner-h";
    const G_MARKER: &str = "W6_UNSEAL_G_BOOT_MARKER";
    const H_MARKER: &str = "W6_UNSEAL_H_BOOT_MARKER";

    let sealed = sealed_friend_world(GROUP, P, G, G_MARKER, H, H_MARKER, false);
    sealed.install();
    assert_present(&sealed, GROUP, G, G_MARKER);
    assert_absent(&sealed, GROUP, H, H_MARKER);

    let unsealed = sealed_friend_world(GROUP, P, G, G_MARKER, H, H_MARKER, true);
    unsealed.install();
    assert_present(&unsealed, GROUP, G, G_MARKER);
    assert_present(&unsealed, GROUP, H, H_MARKER);
}

#[test]
fn break_in_is_quiet() {
    const GROUP: &str = "org.w6.power.quiet";
    const B: &str = "carrier-b";
    const W: &str = "hidden-w";
    const W_MARKER: &str = "W6_QUIET_W_BOOT_MARKER";

    let world = private_edge_world(GROUP, B, W, W_MARKER, true);
    let plain = world.install_output(false);
    let json = world.install_output(true);
    let streams = format!(
        "{}\n{}\n{}\n{}",
        String::from_utf8_lossy(&plain.stdout),
        String::from_utf8_lossy(&plain.stderr),
        String::from_utf8_lossy(&json.stdout),
        String::from_utf8_lossy(&json.stderr)
    )
    .to_lowercase();
    for forbidden in ["override", "break-in", "break in"] {
        assert!(
            !streams.contains(forbidden),
            "install output must stay quiet about the break-in; found {forbidden:?}:\n{streams}"
        );
    }

    let lock = world.lock();
    let w = lock_package_block(&lock, W).expect("quiet override must still admit W");
    assert!(w.contains(&format!("via_override = \"{GROUP}/{ROOT_NAME}\"")));
}

#[test]
fn sealed_circle_admits_exactly_the_list() {
    // Same seal semantics as above: both worlds deliver G itself; only the
    // listed friend's grant opens G's circle, so the inner H exists for the
    // listed giver and never for the outsider.
    const GROUP: &str = "org.w6.power.circle";
    const P: &str = "friend-p";
    const P2: &str = "outsider-p2";
    const G: &str = "sealed-g";
    const H: &str = "inner-h";
    const G_MARKER: &str = "W6_CIRCLE_G_BOOT_MARKER";
    const H_MARKER: &str = "W6_CIRCLE_H_BOOT_MARKER";

    let allowed = listed_friend_world(GROUP, P, P, G, G_MARKER, H, H_MARKER);
    allowed.install();
    assert_present(&allowed, GROUP, G, G_MARKER);
    assert_present(&allowed, GROUP, H, H_MARKER);

    let denied = listed_friend_world(GROUP, P2, P, G, G_MARKER, H, H_MARKER);
    denied.install();
    assert_present(&denied, GROUP, G, G_MARKER);
    assert_absent(&denied, GROUP, H, H_MARKER);
}

fn private_edge_world(
    group: &str,
    carrier: &str,
    hidden: &str,
    hidden_marker: &str,
    root_override: bool,
) -> FixtureWorld {
    let mut root_tail = requires_one(group, carrier, "{ version = \"^1\" }");
    if root_override {
        root_tail.push_str(&format!(
            "\n[override]\n\"{group}/{carrier} -> {group}/{hidden}\" = \
             {{ access = \"public\" }}\n"
        ));
    }
    let carrier_tail = private_requirement(group, hidden);
    FixtureWorld::new(
        group,
        &root_tail,
        &[
            PackageSpec {
                name: carrier,
                manifest_tail: &carrier_tail,
                marker: "W6_PRIVATE_EDGE_CARRIER_BOOT_MARKER",
            },
            PackageSpec {
                name: hidden,
                manifest_tail: "",
                marker: hidden_marker,
            },
        ],
    )
}

fn sealed_friend_world(
    group: &str,
    friend: &str,
    sealed: &str,
    sealed_marker: &str,
    inner: &str,
    inner_marker: &str,
    unseal: bool,
) -> FixtureWorld {
    let mut root_tail = requires_one(group, friend, "{ version = \"^1\", friend = true }");
    if unseal {
        root_tail.push_str(&format!(
            "\n[override]\n\"{group}/{sealed}\" = {{ allow-friends = \"*\" }}\n"
        ));
    }
    let friend_tail = friends_only_requirement(group, sealed);
    let sealed_tail = format!(
        "\n[visibility]\nallow-friends = []\n{}",
        friends_only_requirement(group, inner)
    );
    FixtureWorld::new(
        group,
        &root_tail,
        &[
            PackageSpec {
                name: friend,
                manifest_tail: &friend_tail,
                marker: "W6_UNSEAL_P_BOOT_MARKER",
            },
            PackageSpec {
                name: sealed,
                manifest_tail: &sealed_tail,
                marker: sealed_marker,
            },
            PackageSpec {
                name: inner,
                manifest_tail: "",
                marker: inner_marker,
            },
        ],
    )
}

fn listed_friend_world(
    group: &str,
    grant_giver: &str,
    allowed_friend: &str,
    sealed: &str,
    sealed_marker: &str,
    inner: &str,
    inner_marker: &str,
) -> FixtureWorld {
    let root_tail = requires_one(group, grant_giver, "{ version = \"^1\", friend = true }");
    let giver_tail = friends_only_requirement(group, sealed);
    let sealed_tail = format!(
        "\n[visibility]\nallow-friends = [\"{group}/{allowed_friend}\"]\n{}",
        friends_only_requirement(group, inner)
    );
    FixtureWorld::new(
        group,
        &root_tail,
        &[
            PackageSpec {
                name: grant_giver,
                manifest_tail: &giver_tail,
                marker: "W6_CIRCLE_GRANT_GIVER_BOOT_MARKER",
            },
            PackageSpec {
                name: sealed,
                manifest_tail: &sealed_tail,
                marker: sealed_marker,
            },
            PackageSpec {
                name: inner,
                manifest_tail: "",
                marker: inner_marker,
            },
        ],
    )
}

fn requires_one(group: &str, name: &str, declaration: &str) -> String {
    format!("\n[requires.packages]\n\"{group}/{name}\" = {declaration}\n")
}

fn private_requirement(group: &str, name: &str) -> String {
    requires_one(group, name, "{ version = \"^1\", access = \"private\" }")
}

fn friends_only_requirement(group: &str, name: &str) -> String {
    requires_one(
        group,
        name,
        "{ version = \"^1\", access = \"friends-only\" }",
    )
}

fn assert_present(world: &FixtureWorld, group: &str, name: &str, marker: &str) {
    assert!(
        lock_package_block(&world.lock(), name).is_some(),
        "{name} must be locked"
    );
    assert!(world.slot(group, name).exists(), "{name} must have a slot");
    assert!(world.slot_boot(group, name).contains(marker));
    assert!(world.index().contains(&slot_slug(group, name)));
}

fn assert_absent(world: &FixtureWorld, group: &str, name: &str, marker: &str) {
    assert!(
        lock_package_block(&world.lock(), name).is_none(),
        "{name} must not be locked"
    );
    assert!(
        !world.slot(group, name).exists(),
        "{name} must not have a slot"
    );
    assert!(!world.index().contains(&slot_slug(group, name)));
    assert!(!world.static_lane().contains(marker));
}

fn write_package(registry: &Path, group: &str, package: &PackageSpec<'_>) {
    let source = registry
        .join(group)
        .join(package.name)
        .join(format!("v{VERSION}"));
    let boot_dir = source.join(common::spec_rel("boot"));
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
             source = \"{}/10-flow-{}.md\"\n\
             category = \"flow\"\n\
             link = \"dynamic\"\n\
             {}",
            package.name,
            common::boot_str(),
            package.name,
            package.manifest_tail
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
