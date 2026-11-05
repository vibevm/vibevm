//! Actual workspace declarations merge into one package instance and durable input.
#![cfg(test)]
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#CONSUMER-MERGE");
use super::*;
use std::os::unix::fs::PermissionsExt;
use vibe_core::manifest::ConfigurationIntentDecl;

const SECOND: &str = "oracle-second";
const SECOND_MARKER: &str = "ORACLE_SECOND_LOADED";
const MEMBERS: [(&str, &str); 2] = [("left", "alpha"), ("right", "beta")];

fn member_manifest(member: &str, feature: &str, intent: &str) -> String {
    format!(
        "[project]\nname='{member}'\nversion='0.1.0'\n\
         [requires.packages.'{GROUP}/{DRIVER}']\nversion='^1'\nlink='static-transitive'\n\
         configuration={{policy='require-selection',default_features=false,selections={{components={{intent='{intent}',members=['{feature}']}}}}}}\n"
    )
}
fn configure_members(fixture: &Fixture, intent: &str) {
    for (member, feature) in MEMBERS {
        let text = member_manifest(member, feature, intent);
        Manifest::parse_str(&text).expect("valid actual workspace consumer declaration");
        write(&fixture.project.join(member).join("vibe.toml"), text);
    }
}
fn workspace_fixture() -> Fixture {
    let fixture = Fixture::new(false);
    leaf(&fixture.registry, SECOND, SECOND_MARKER);
    package(
        &fixture.registry,
        DRIVER,
        r#"
[requires.packages]
"org.vibevm/oracle-base" = "^1"
"org.vibevm/oracle-extra" = { version = "^1", optional = true }
"org.vibevm/oracle-second" = { version = "^1", optional = true }

[[configuration.features]]
id = "alpha"
name = "First component"
effects = [{ kind = "dependency", target = "org.vibevm/oracle-extra" }]

[[configuration.features]]
id = "beta"
name = "Second component"
effects = [{ kind = "dependency", target = "org.vibevm/oracle-second" }]

[[configuration.groups]]
mode = "choice"
id = "components"
name = "Components"
cardinality = "zero-or-more"
members = ["alpha", "beta"]
"#,
        DRIVER_MARKER,
    );
    let root = format!(
        "[project]\nname='shared-driver-workspace'\nversion='0.1.0'\n\
         [workspace]\nmembers=['left','right']\n\
         [[registry]]\nname='fixture'\nurl='file://{}'\n",
        fixture.registry.display()
    );
    Manifest::parse_str(&root).unwrap();
    write(&fixture.project.join("vibe.toml"), root);
    configure_members(&fixture, "additive");
    for (member, _) in MEMBERS {
        for launcher in ["AGENTS.md", "CLAUDE.md", "GEMINI.md"] {
            write(
                &fixture.project.join(member).join(launcher),
                "# Workspace member fixture\n",
            );
        }
    }
    fixture
}
fn authored_consumers(fixture: &Fixture) -> BTreeMap<String, ConfigurationIntentDecl> {
    MEMBERS
        .into_iter()
        .map(|(member, _)| {
            let manifest = Manifest::read(fixture.project.join(member).join("vibe.toml")).unwrap();
            (
                format!("{member}/vibe.toml"),
                manifest.requires.configurations[&coordinate(DRIVER)].clone(),
            )
        })
        .collect()
}
fn member_boot(fixture: &Fixture) -> BTreeMap<String, String> {
    MEMBERS
        .into_iter()
        .map(|(member, _)| {
            let bodies: Vec<_> = [common::static_md_rel(), common::static_xml_rel()]
                .into_iter()
                .filter_map(|file| {
                    let path = fixture.project.join(member).join(file);
                    path.is_file().then(|| fs::read_to_string(path).unwrap())
                })
                .collect();
            assert!(
                !bodies.is_empty(),
                "{member} needs actual generated static-transitive boot"
            );
            (member.into(), bodies.join("\n"))
        })
        .collect()
}
fn assert_combined_world(fixture: &Fixture) {
    let lock = fixture.lock();
    assert_eq!(lock.meta.schema_version, 8);
    assert_eq!(lock.packages.len(), 4);
    assert_eq!(
        lock.packages
            .iter()
            .filter(|p| p.name.as_str() == DRIVER && p.group.as_str() == GROUP)
            .count(),
        1
    );
    let expected = BTreeSet::from([
        coordinate(DRIVER),
        coordinate(BASE),
        coordinate(EXTRA),
        coordinate(SECOND),
    ]);
    assert_eq!(
        lock.packages
            .iter()
            .map(|p| format!("{}/{}", p.group, p.name))
            .collect::<BTreeSet<_>>(),
        expected
    );
    assert!(
        lock.packages
            .iter()
            .all(|p| p.version.to_string() == VERSION)
    );
    let driver = lock
        .packages
        .iter()
        .find(|p| p.name.as_str() == DRIVER)
        .unwrap();
    let dependencies = BTreeSet::from([coordinate(BASE), coordinate(EXTRA), coordinate(SECOND)]);
    assert_eq!(
        driver
            .dependencies
            .iter()
            .map(|p| p.qualified_name())
            .collect::<BTreeSet<_>>(),
        dependencies
    );
    assert_eq!(lock.configurations.len(), 1);
    let record = &lock.configurations[&coordinate(DRIVER)];
    assert_eq!(
        record.consumers,
        authored_consumers(fixture),
        "retain both original actual workspace declarations"
    );
    assert_eq!(record.consumers.len(), 2);
    assert_eq!(
        record.declarations.optional.get(&coordinate(EXTRA)),
        Some(&true)
    );
    assert_eq!(
        record.declarations.optional.get(&coordinate(SECOND)),
        Some(&true)
    );
    assert!(record.consumers.values().all(|i| i.features.is_empty()));
    assert_eq!(
        record
            .features
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["alpha", "beta"])
    );
    assert_eq!(
        record
            .active_dependencies
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>(),
        dependencies
    );
    assert_eq!(record.dependency_causes[&coordinate(EXTRA)], ["alpha"]);
    assert_eq!(record.dependency_causes[&coordinate(SECOND)], ["beta"]);
    assert!(record.dependency_causes[&coordinate(BASE)].is_empty());
    for name in [BASE, EXTRA, SECOND] {
        assert_eq!(
            fs::read_to_string(fixture.slot(name).join("payload.txt")).unwrap(),
            format!("PAYLOAD {name}\n")
        );
    }
    assert_eq!(
        fs::read_to_string(fixture.slot(DRIVER).join("unused-source.txt")).unwrap(),
        UNUSED
    );
    let expected_hooks: BTreeMap<String, usize> =
        BTreeMap::from([(BASE.into(), 1), (EXTRA.into(), 1), (SECOND.into(), 1)]);
    assert_eq!(fixture.hooks(), expected_hooks);
    for (member, body) in member_boot(fixture) {
        for marker in [DRIVER_MARKER, BASE_MARKER, EXTRA_MARKER, SECOND_MARKER] {
            assert!(
                body.contains(marker),
                "{member} boot is missing combined effect {marker}"
            );
        }
        assert!(
            !fixture
                .project
                .join(&member)
                .join(common::slot_dir(&format!("{GROUP}.{DRIVER}"), VERSION))
                .exists(),
            "workspace consumers must not install independent driver copies"
        );
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Entry {
    Directory,
    File(u32, Vec<u8>),
    Symlink(PathBuf),
}
fn semantic_snapshot(root: &Path) -> BTreeMap<PathBuf, Entry> {
    fn walk(root: &Path, path: &Path, output: &mut BTreeMap<PathBuf, Entry>) {
        let relative = path.strip_prefix(root).unwrap().to_owned();
        let metadata = fs::symlink_metadata(path).unwrap();
        if metadata.file_type().is_symlink() {
            output.insert(relative, Entry::Symlink(fs::read_link(path).unwrap()));
        } else if metadata.is_dir() {
            output.insert(relative, Entry::Directory);
            for entry in fs::read_dir(path).unwrap() {
                walk(root, &entry.unwrap().path(), output);
            }
        } else {
            output.insert(
                relative,
                Entry::File(
                    metadata.permissions().mode() & 0o777,
                    fs::read(path).unwrap(),
                ),
            );
        }
    }
    let mut output = BTreeMap::new();
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        // The existing command trace/lease metadata is not package semantic state.
        if path.file_name().unwrap() != ".vibe" {
            walk(root, &path, &mut output);
        }
    }
    output
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#CONSUMER-MERGE")]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PERSISTENCE")]
fn additive_workspace_consumers_share_one_driver_and_cold_replay_both_original_inputs() {
    let fixture = workspace_fixture();
    let root_before = fs::read(fixture.project.join("vibe.toml")).unwrap();
    let consumers_before = authored_consumers(&fixture);
    fixture.install_ok();
    assert_combined_world(&fixture);
    let first = fixture.lock();
    let boot = member_boot(&fixture);
    let hooks = fixture.hooks();
    fixture.install_ok();
    assert_combined_world(&fixture);
    let cold = fixture.lock();
    assert_eq!(cold.configurations, first.configurations);
    assert_eq!(graph(&cold), graph(&first));
    assert_eq!(member_boot(&fixture), boot);
    assert_eq!(fixture.hooks(), hooks);
    assert_eq!(authored_consumers(&fixture), consumers_before);
    assert_eq!(
        fs::read(fixture.project.join("vibe.toml")).unwrap(),
        root_before
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#CONSUMER-MERGE")]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#EFFECT-CONSUMERS")]
fn incompatible_exact_workspace_inputs_name_both_consumers_and_leave_the_world_unchanged() {
    let fixture = workspace_fixture();
    fixture.install_ok();
    assert_combined_world(&fixture);
    // Same real workspace/package instance: each exact answer alone is valid,
    // but their distinct sets cannot be reconciled as one shared configuration.
    configure_members(&fixture, "exact");
    let requested = authored_consumers(&fixture);
    assert_eq!(requested.len(), 2);
    let before = semantic_snapshot(&fixture.project);
    let output = fixture.install();
    let diagnostic = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !output.status.success(),
        "incompatible exact inputs succeeded: {diagnostic}"
    );
    assert!(diagnostic.contains(&coordinate(DRIVER)), "{diagnostic}");
    for consumer in requested.keys() {
        assert!(
            diagnostic.contains(consumer),
            "missing actual consumer {consumer}: {diagnostic}"
        );
    }
    assert_eq!(
        semantic_snapshot(&fixture.project),
        before,
        "conflict changed authored manifests, lock, slots, generated boot/launchers or hook evidence"
    );
}
