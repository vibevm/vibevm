//! Descriptor-selected adoption without changing legacy package scanning.

mod common;

use std::fs;

use common::UserScratch;

const PACKAGE: &str = "org.example/declared-facts";

struct World {
    user: UserScratch,
    project: tempfile::TempDir,
    _registry: tempfile::TempDir,
}

impl World {
    fn new(rows: &str, spec_format: &str, include_declared: bool) -> Self {
        Self::with_extra(rows, spec_format, include_declared, &[], DEFAULT_DECLARED)
    }

    fn with_extra(
        rows: &str,
        spec_format: &str,
        include_declared: bool,
        extra: &[(&str, &[u8])],
        declared_body: &str,
    ) -> Self {
        let user = UserScratch::new();
        let project = tempfile::tempdir().unwrap();
        let registry = tempfile::tempdir().unwrap();
        let package = registry.path().join("org.example/declared-facts/v1.1.0");
        fs::create_dir_all(package.join(common::spec_rel(""))).unwrap();
        let declared_path = common::spec_rel("DECLARED.xml");
        let raw = format!(
            "[package]\ngroup = 'org.example'\nname = 'declared-facts'\nkind = 'flow'\nversion = '1.1.0'\nepoch = 1\n{rows}"
        )
        .replace("$DECLARED", &declared_path)
        .replace("$RAW", &common::spec_rel("DECLARED.md"))
        .replace("$DATA", &common::spec_rel("DATA.xml"));
        fs::write(package.join("vibe.toml"), raw).unwrap();
        if include_declared {
            fs::write(package.join(&declared_path), declared_body).unwrap();
        }
        fs::write(package.join(common::spec_rel("UNDECLARED.xml")),
            "<spec xmlns=\"https://vibevm.org/spec/1\"><p><fact id=\"FAKE\" status=\"impl/done\">fake</fact></p></spec>"
        ).unwrap();
        for (path, bytes) in extra {
            let destination = package.join(path);
            fs::create_dir_all(destination.parent().unwrap()).unwrap();
            fs::write(destination, bytes).unwrap();
        }
        fs::write(project.path().join("vibe.toml"), format!(
            "[project]\ngroup = 'org.consumer'\nname = 'demo'\nversion = '1.1.0'\nspec_format = '{spec_format}'\n"
        )).unwrap();
        user.vibe()
            .args(["install", "org.example/declared-facts@=1.1.0", "--path"])
            .arg(project.path())
            .arg("--registry")
            .arg(registry.path())
            .arg("--assume-yes")
            .assert()
            .success();
        Self {
            user,
            project,
            _registry: registry,
        }
    }

    fn adopt(&self) -> std::process::Output {
        self.user
            .vibe()
            .current_dir(self.project.path())
            .args(["facts", "adopt", "--package", PACKAGE])
            .output()
            .unwrap()
    }

    fn report(&self) -> std::process::Output {
        self.user
            .vibe()
            .current_dir(self.project.path())
            .args(["facts", "report", "--package", PACKAGE])
            .output()
            .unwrap()
    }

    fn slot(&self) -> std::path::PathBuf {
        self.project
            .path()
            .join(common::slot_dir("org.example.declared-facts", "1.1.0"))
    }

    fn registry_file(&self) -> std::path::PathBuf {
        self.project
            .path()
            .join(common::facts_rel("org.example.declared-facts.toml"))
    }
}

const DECLARED: &str = "[[package.sources]]\nfile = '$DECLARED'\nmedia = 'application/xml'\ninterpretation = 'vibe-spec'\nrole = 'spec'\nloading = 'scan'\n";
const DEFAULT_DECLARED: &str = "<spec xmlns=\"https://vibevm.org/spec/1\"><p><fact id=\"REAL\" status=\"impl/done\">real</fact></p></spec>";
const RAW_MARKDOWN: &str = "[[package.sources]]\nfile = '$RAW'\nmedia = 'text/markdown'\ninterpretation = 'commonmark'\nrole = 'resource'\nloading = 'scan'\n";
const RAW_XML: &str = "[[package.sources]]\nfile = '$DATA'\nmedia = 'application/xml'\ninterpretation = 'xml-data'\nrole = 'resource'\nloading = 'explicit'\n";

#[test]
fn mixed_slot_adopts_only_exact_declared_fact() {
    let world = World::new(DECLARED, "mixed", true);
    let result = world.adopt();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(String::from_utf8_lossy(&result.stdout).contains("added=1"));
    let stored = fs::read_to_string(world.registry_file()).unwrap();
    assert!(stored.contains("spec://org.example/declared-facts/DECLARED#REAL"));
    assert!(!stored.contains("FAKE"));
}

#[test]
fn missing_declared_file_after_install_refuses_before_registry_write() {
    let world = World::new(DECLARED, "mixed", true);
    fs::remove_file(world.slot().join(common::spec_rel("DECLARED.xml"))).unwrap();
    for result in [world.adopt(), world.report()] {
        assert!(!result.status.success());
        assert!(
            String::from_utf8_lossy(&result.stderr).contains("incomplete"),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    assert!(!world.registry_file().exists());
}

#[test]
fn transformed_slot_with_original_media_descriptor_refuses() {
    let world = World::new(DECLARED, "markdown", true);
    let result = world.adopt();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("Mixed slot"));
    assert!(!world.registry_file().exists());
}

#[test]
fn mixed_same_stem_raw_resources_do_not_mint_facts_or_false_collisions() {
    let rows = format!("{DECLARED}{RAW_MARKDOWN}{RAW_XML}");
    let markdown = b"# Ordinary\n\n@fact:FALSE This is literal CommonMark. @status:impl/done\n";
    let xml_data = b"<data><not-vibe /></data>";
    let world = World::with_extra(
        &rows,
        "mixed",
        true,
        &[
            (&common::spec_rel("DECLARED.md"), markdown),
            (&common::spec_rel("DATA.xml"), xml_data),
        ],
        DEFAULT_DECLARED,
    );
    let adopted = world.adopt();
    assert!(
        adopted.status.success(),
        "{}",
        String::from_utf8_lossy(&adopted.stderr)
    );
    let stored = fs::read_to_string(world.registry_file()).unwrap();
    assert!(stored.contains("DECLARED#REAL"));
    assert!(!stored.contains("FALSE"));
    assert!(!stored.contains("FAKE"));
    let reported = world.report();
    assert!(
        reported.status.success(),
        "{}",
        String::from_utf8_lossy(&reported.stderr)
    );
    assert!(String::from_utf8_lossy(&reported.stdout).contains("1/1 (indeterminate 0)"));
    assert_eq!(
        fs::read(world.slot().join(common::spec_rel("DECLARED.md"))).unwrap(),
        markdown
    );
    assert_eq!(
        fs::read(world.slot().join(common::spec_rel("DATA.xml"))).unwrap(),
        xml_data
    );
}

#[test]
fn report_preserves_selected_status_and_void_fact_cardinality() {
    let both = "<spec xmlns=\"https://vibevm.org/spec/1\"><p><fact id=\"REAL\" status=\"impl/done\">real</fact></p><p><fact id=\"VOID\">not yet adopted</fact></p></spec>";
    let world = World::with_extra(DECLARED, "mixed", true, &[], both);
    let adopted = world.adopt();
    assert!(
        adopted.status.success(),
        "{}",
        String::from_utf8_lossy(&adopted.stderr)
    );
    assert!(String::from_utf8_lossy(&adopted.stdout).contains("added=1"));
    let stored = fs::read_to_string(world.registry_file()).unwrap();
    assert!(stored.contains("DECLARED#REAL"));
    assert!(!stored.contains("DECLARED#VOID"));
    let reported = world.report();
    assert!(
        reported.status.success(),
        "{}",
        String::from_utf8_lossy(&reported.stderr)
    );
    assert!(String::from_utf8_lossy(&reported.stdout).contains("1/2 (indeterminate 0)"));
}

#[test]
fn foreign_slot_source_hash_and_changed_selected_bytes_refuse_both_consumers() {
    let world = World::new(DECLARED, "mixed", true);
    let record_path = world
        .slot()
        .join(vibe_workspace::vibedeps::SLOT_RECORD_FILENAME);
    let original = fs::read_to_string(&record_path).unwrap();
    let record = vibe_workspace::vibedeps::read_slot_record(&world.slot()).unwrap();
    let foreign = original.replacen(
        record.source_hash.as_str(),
        &format!("sha256:{}", "0".repeat(64)),
        1,
    );
    assert_ne!(foreign, original);
    fs::write(&record_path, foreign).unwrap();
    for outcome in [world.adopt(), world.report()] {
        assert!(!outcome.status.success());
        assert!(String::from_utf8_lossy(&outcome.stderr).contains("lock-selected source hash"));
    }
    assert!(!world.registry_file().exists());

    fs::write(&record_path, original).unwrap();
    fs::write(
        world.slot().join(common::spec_rel("DECLARED.xml")),
        b"changed output",
    )
    .unwrap();
    for outcome in [world.adopt(), world.report()] {
        assert!(!outcome.status.success());
        assert!(
            String::from_utf8_lossy(&outcome.stderr).contains("differs from recorded slot payload")
        );
    }
    assert!(!world.registry_file().exists());
}

#[test]
fn removed_descriptor_rows_in_materialized_slot_never_downgrade_to_suffix_facts() {
    let world = World::new(DECLARED, "mixed", true);
    let manifest = world.slot().join("vibe.toml");
    fs::write(
        manifest,
        "[package]\ngroup='org.example'\nname='declared-facts'\nkind='flow'\nversion='1.1.0'\nepoch=1\n",
    )
    .unwrap();
    for outcome in [world.adopt(), world.report()] {
        assert!(!outcome.status.success());
        assert!(
            String::from_utf8_lossy(&outcome.stderr).contains("differs from recorded slot payload"),
            "{}",
            String::from_utf8_lossy(&outcome.stderr)
        );
    }
    assert!(!world.registry_file().exists());
}
