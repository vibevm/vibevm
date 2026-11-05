//! Ordinary authored package choices must change the actual installed graph.
//! Candidate integration target: pair with typed original lock-record assertions
//! before treating the full persistence contract as accepted (see integration notes).
#![cfg(target_os = "linux")]
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#EFFECT-CONSUMERS");
mod common;

use common::UserScratch;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;
use vibe_core::manifest::{Lockfile, Manifest};

const GROUP: &str = "org.vibevm";
const DRIVER: &str = "oracle-driver";
const BASE: &str = "oracle-base";
const EXTRA: &str = "oracle-extra";
const VERSION: &str = "1.0.0";
const DRIVER_MARKER: &str = "ORACLE_DRIVER_LOADED";
const BASE_MARKER: &str = "ORACLE_BASE_LOADED";
const EXTRA_MARKER: &str = "ORACLE_EXTRA_LOADED";
const UNUSED: &str = "Keep full package source in both configurations.\n";

fn write(path: &Path, bytes: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}
fn coordinate(name: &str) -> String {
    format!("{GROUP}/{name}")
}
fn package_header(name: &str) -> String {
    format!(
        "[package]\ngroup='{GROUP}'\nname='{name}'\nversion='{VERSION}'\nkind='flow'\n\
         [boot_snippet]\nsource='boot/rules.md'\ncategory='flow'\n"
    )
}
fn package(root: &Path, name: &str, body: &str, marker: &str) -> PathBuf {
    let path = root.join(GROUP).join(name).join(format!("v{VERSION}"));
    let manifest = package_header(name) + body;
    // Parser failure is an explicit declaration RED, never a skipped test.
    let parsed = Manifest::parse_str(&manifest).expect("ordinary authored package must parse");
    if let Some(definition) = &parsed.configuration {
        definition
            .compile_choices()
            .expect("valid common definition");
    }
    write(&path.join("vibe.toml"), manifest);
    write(
        &path.join("boot/rules.md"),
        format!("# {name}\n\n{marker}\n"),
    );
    write(&path.join("payload.txt"), format!("PAYLOAD {name}\n"));
    path
}
fn leaf(registry: &Path, name: &str, marker: &str) {
    let path = package(
        registry,
        name,
        "\n[hooks]\npre-install='hooks/record'\n",
        marker,
    );
    write(
        &path.join("hooks/record.sh"),
        format!("set -eu\nprintf '%s\\n' '{name}' >> \"$VIBE_PROJECT_ROOT/effects.log\"\n"),
    );
}

struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    project: PathBuf,
    registry: PathBuf,
    user: UserScratch,
}
impl Fixture {
    fn new(enabled: bool) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().to_owned();
        let project = root.join("project");
        let registry = root.join("registry");
        fs::create_dir_all(&project).unwrap();
        fs::create_dir_all(root.join("home")).unwrap();
        leaf(&registry, BASE, BASE_MARKER);
        leaf(&registry, EXTRA, EXTRA_MARKER);
        let driver = package(
            &registry,
            DRIVER,
            r#"
[requires.packages]
"org.vibevm/oracle-base" = "^1"
"org.vibevm/oracle-extra" = { version = "^1", optional = true }

[[configuration.features]]
id = "enabled"
name = "Enable extra"
effects = [{ kind = "dependency", target = "org.vibevm/oracle-extra" }]

[[configuration.features]]
id = "disabled"
name = "No extra"

# This feature is never selected; base must remain a mandatory dependency.
[[configuration.features]]
id = "mention-required"
name = "Mention mandatory dependency"
effects = [{ kind = "dependency", target = "org.vibevm/oracle-base" }]

[[configuration.groups]]
mode = "choice"
id = "extra"
name = "Extra support"
cardinality = "exactly-one"
members = ["enabled", "disabled"]
defaults = ["disabled"]
"#,
            DRIVER_MARKER,
        );
        write(&driver.join("unused-source.txt"), UNUSED);
        let selection = if enabled { "enabled" } else { "disabled" };
        let manifest = format!(
            "[project]\nname='optional-{selection}'\nversion='0.1.0'\n\
             [requires.packages.'{GROUP}/{DRIVER}']\nversion='^1'\nlink='static-transitive'\n\
             configuration={{policy='require-selection',default_features=false,selections={{extra={{intent='exact',members=['{selection}']}}}}}}\n\
             [[registry]]\nname='fixture'\nurl='file://{}'\n",
            registry.display()
        );
        Manifest::parse_str(&manifest).expect("actual requires input must parse");
        write(&project.join("vibe.toml"), manifest);
        for launcher in ["AGENTS.md", "CLAUDE.md", "GEMINI.md"] {
            write(&project.join(launcher), "# Fixture launcher\n");
        }
        Self {
            _temp: temp,
            root,
            project,
            registry,
            user: UserScratch::new(),
        }
    }
    fn install(&self) -> Output {
        let mut cmd = self.user.vibe();
        cmd.current_dir(&self.project)
            .env("HOME", self.root.join("home"))
            .env("USERPROFILE", self.root.join("home"));
        for variable in ["VIBE_AGENT_COMMAND", "VIBE_AGENT_ARGS", "VIBE_INVOKED_BY"] {
            cmd.env_remove(variable);
        }
        cmd.args([
            "install",
            "--path",
            self.project.to_str().unwrap(),
            "--offline",
            "--assume-yes",
            "--json",
        ])
        .output()
        .unwrap()
    }
    fn install_ok(&self) {
        let output = self.install();
        assert!(
            output.status.success(),
            "status={} stdout={} stderr={}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        if let Some(directory) = std::env::var_os("NST_PACKAGE_CONFIGURATION_EVIDENCE") {
            let directory = PathBuf::from(directory);
            fs::create_dir_all(&directory).unwrap();
            let id = format!(
                "{}-{}",
                self.root.file_name().unwrap().to_string_lossy(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            );
            fs::write(directory.join(format!("{id}.stdout")), &output.stdout).unwrap();
            fs::write(directory.join(format!("{id}.stderr")), &output.stderr).unwrap();
        }
        // The existing install contract emits deferred plan/closure documents
        // followed by its final report. Parse the entire stream, including EOF.
        let documents = serde_json::Deserializer::from_slice(&output.stdout)
            .into_iter::<serde_json::Value>()
            .collect::<Result<Vec<_>, _>>()
            .unwrap_or_else(|error| {
                panic!(
                    "invalid install JSON stream: {error}; stdout={}",
                    String::from_utf8_lossy(&output.stdout)
                )
            });
        let report = documents.last().expect("final install report");
        assert_eq!(
            documents
                .iter()
                .filter(|d| d["command"] == "install")
                .count(),
            1
        );
        assert!(documents[..documents.len() - 1].iter().all(|d| matches!(
            d["command"].as_str(),
            Some("install:plan" | "lifecycle:plan" | "install:closure-diff")
        )));
        assert_eq!(report["ok"], true, "{report}");
        assert_eq!(report["complete"], true, "{report}");
        assert_eq!(report["command"], "install", "{report}");
    }
    fn slot(&self, name: &str) -> PathBuf {
        self.project
            .join(common::slot_dir(&format!("{GROUP}.{name}"), VERSION))
    }
    fn lock(&self) -> Lockfile {
        Lockfile::read(self.project.join("vibe.lock")).expect("real install must publish a lock")
    }
    fn hooks(&self) -> BTreeMap<String, usize> {
        let mut counts = BTreeMap::new();
        for line in fs::read_to_string(self.project.join("effects.log"))
            .unwrap()
            .lines()
        {
            *counts.entry(line.to_owned()).or_default() += 1;
        }
        counts
    }
    fn static_boot(&self) -> String {
        let paths = [common::static_md_rel(), common::static_xml_rel()];
        let bodies: Vec<_> = paths
            .iter()
            .filter_map(|path| {
                let path = self.project.join(path);
                path.is_file().then(|| fs::read_to_string(path).unwrap())
            })
            .collect();
        assert!(
            !bodies.is_empty(),
            "actual static-transitive boot must be generated"
        );
        bodies.join("\n")
    }
    fn assert_world(&self, enabled: bool) {
        let lock = self.lock();
        let actual: BTreeSet<_> = lock
            .packages
            .iter()
            .map(|p| format!("{}/{}", p.group, p.name))
            .collect();
        let mut expected = BTreeSet::from([coordinate(DRIVER), coordinate(BASE)]);
        if enabled {
            expected.insert(coordinate(EXTRA));
        }
        assert_eq!(
            actual, expected,
            "optional selection must change the actual solved lock set"
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
        let edges: BTreeSet<_> = driver
            .dependencies
            .iter()
            .map(|p| p.qualified_name())
            .collect();
        let mut expected_edges = BTreeSet::from([coordinate(BASE)]);
        if enabled {
            expected_edges.insert(coordinate(EXTRA));
        }
        assert_eq!(
            edges, expected_edges,
            "selected owner edges, not only package presence"
        );
        assert_eq!(
            fs::read_to_string(self.slot(BASE).join("payload.txt")).unwrap(),
            format!("PAYLOAD {BASE}\n")
        );
        assert_eq!(
            fs::read_to_string(self.slot(DRIVER).join("unused-source.txt")).unwrap(),
            UNUSED
        );
        assert_eq!(
            self.slot(EXTRA).exists(),
            enabled,
            "an inactive optional must not be materialized"
        );
        if enabled {
            assert_eq!(
                fs::read_to_string(self.slot(EXTRA).join("payload.txt")).unwrap(),
                format!("PAYLOAD {EXTRA}\n")
            );
        }
        let expected_hooks: BTreeMap<String, usize> = if enabled {
            BTreeMap::from([(BASE.into(), 1), (EXTRA.into(), 1)])
        } else {
            BTreeMap::from([(BASE.into(), 1)])
        };
        assert_eq!(
            self.hooks(),
            expected_hooks,
            "actual leaf effects must match activation exactly"
        );
        let boot = self.static_boot();
        assert!(boot.contains(DRIVER_MARKER));
        assert!(
            boot.contains(BASE_MARKER),
            "mandatory base must load despite its mentioning feature being off"
        );
        assert_eq!(
            boot.contains(EXTRA_MARKER),
            enabled,
            "selected graph must reach actual generated loading"
        );
        assert!(self.project.join(common::index_rel()).is_file());
    }
}

/// Actual selected graph projection; intentionally not a surrogate for the
/// required typed original-intent record assertion documented for integration.
fn graph(lock: &Lockfile) -> BTreeMap<String, BTreeSet<String>> {
    lock.packages
        .iter()
        .map(|p| {
            (
                format!("{}/{}@{}", p.group, p.name, p.version),
                p.dependencies.iter().map(|d| d.to_string()).collect(),
            )
        })
        .collect()
}
fn assert_original_record(lock: &Lockfile, authored: &[u8], enabled: bool) {
    let manifest =
        vibe_core::manifest::Manifest::parse_str(std::str::from_utf8(authored).unwrap()).unwrap();
    let original = &manifest.requires.configurations[&coordinate(DRIVER)];
    assert_eq!(lock.meta.schema_version, 8);
    assert_eq!(lock.configurations.len(), 1);
    let record = &lock.configurations[&coordinate(DRIVER)];
    assert_eq!(record.consumers.len(), 1);
    let (consumer, retained) = record.consumers.iter().next().unwrap();
    assert!(
        consumer.ends_with("/vibe.toml"),
        "actual workspace consumer: {consumer}"
    );
    assert_eq!(retained, original);
    assert!(
        retained.features.is_empty(),
        "choice answer is not a direct feature request"
    );
    let selected = if enabled { "enabled" } else { "disabled" };
    assert_eq!(record.features, [selected]);
    assert_eq!(
        record.declarations.optional.get(&coordinate(EXTRA)),
        Some(&true)
    );
    assert_eq!(
        record.active_dependencies.contains(&coordinate(EXTRA)),
        enabled
    );
    assert!(record.active_dependencies.contains(&coordinate(BASE)));
    assert!(record.dependency_causes[&coordinate(BASE)].is_empty());
    if enabled {
        assert_eq!(record.dependency_causes[&coordinate(EXTRA)], ["enabled"]);
    }
}

fn journey(enabled: bool) {
    let fixture = Fixture::new(enabled);
    let authored = fs::read(fixture.project.join("vibe.toml")).unwrap();
    let driver_source = fs::read(
        fixture
            .registry
            .join(GROUP)
            .join(DRIVER)
            .join("v1.0.0/vibe.toml"),
    )
    .unwrap();
    fixture.install_ok();
    fixture.assert_world(enabled);
    assert_original_record(&fixture.lock(), &authored, enabled);
    assert_eq!(
        fs::read(fixture.project.join("vibe.toml")).unwrap(),
        authored
    );
    let first_graph = graph(&fixture.lock());
    let first_boot = fixture.static_boot();
    let first_hooks = fixture.hooks();
    // This is a distinct CLI process, with the existing real lock and source.
    fixture.install_ok();
    fixture.assert_world(enabled);
    assert_original_record(&fixture.lock(), &authored, enabled);
    assert_eq!(graph(&fixture.lock()), first_graph);
    assert_eq!(fixture.static_boot(), first_boot);
    assert_eq!(
        fixture.hooks(),
        first_hooks,
        "cold install must not repeat package hooks"
    );
    assert_eq!(
        fs::read(fixture.project.join("vibe.toml")).unwrap(),
        authored
    );
    assert_eq!(
        fs::read(
            fixture
                .registry
                .join(GROUP)
                .join(DRIVER)
                .join("v1.0.0/vibe.toml")
        )
        .unwrap(),
        driver_source
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#EFFECT-CONSUMERS")]
fn authored_optional_off_keeps_mandatory_edge_and_cold_world_without_extra_effects() {
    journey(false);
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#EFFECT-CONSUMERS")]
fn authored_optional_on_materializes_executes_loads_and_cold_replays_selected_graph() {
    journey(true);
}

#[path = "cli_common_package_optional/maintenance.rs"]
mod maintenance;

#[path = "cli_common_package_optional/boundaries.rs"]
mod boundaries;
#[path = "cli_common_package_optional/workspace.rs"]
mod workspace;
