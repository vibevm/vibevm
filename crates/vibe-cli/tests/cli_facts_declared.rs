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
        let user = UserScratch::new();
        let project = tempfile::tempdir().unwrap();
        let registry = tempfile::tempdir().unwrap();
        let package = registry.path().join("org.example/declared-facts/v1.1.0");
        fs::create_dir_all(package.join(common::spec_rel(""))).unwrap();
        let declared_path = common::spec_rel("DECLARED.xml");
        let raw = format!(
            "[package]\ngroup = 'org.example'\nname = 'declared-facts'\nkind = 'flow'\nversion = '1.1.0'\nepoch = 1\n{rows}"
        ).replace("$DECLARED", &declared_path);
        fs::write(package.join("vibe.toml"), raw).unwrap();
        if include_declared {
            fs::write(package.join(&declared_path),
                "<spec xmlns=\"https://vibevm.org/spec/1\"><p><fact id=\"REAL\" status=\"impl/done\">real</fact></p></spec>"
            ).unwrap();
        }
        fs::write(package.join(common::spec_rel("UNDECLARED.xml")),
            "<spec xmlns=\"https://vibevm.org/spec/1\"><p><fact id=\"FAKE\" status=\"impl/done\">fake</fact></p></spec>"
        ).unwrap();
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

    fn registry_file(&self) -> std::path::PathBuf {
        self.project
            .path()
            .join(common::facts_rel("org.example.declared-facts.toml"))
    }
}

const DECLARED: &str = "[[package.sources]]\nfile = '$DECLARED'\nmedia = 'application/xml'\ninterpretation = 'vibe-spec'\nrole = 'spec'\nloading = 'scan'\n";

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
fn missing_or_ambiguous_declared_file_refuses_before_registry_write() {
    let ambiguous = format!(
        "{DECLARED}[[package.sources]]\nfile = '$DECLARED'\nmedia = 'application/xml'\ninterpretation = 'opaque'\nrole = 'resource'\nloading = 'explicit'\n"
    );
    for (rows, include_declared) in [(DECLARED, false), (&ambiguous, true)] {
        let world = World::new(rows, "mixed", include_declared);
        let result = world.adopt();
        assert!(!result.status.success());
        assert!(
            String::from_utf8_lossy(&result.stderr).contains("incomplete"),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(!world.registry_file().exists());
    }
}

#[test]
fn transformed_slot_with_original_media_descriptor_refuses() {
    let world = World::new(DECLARED, "markdown", true);
    let result = world.adopt();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("Mixed slot"));
    assert!(!world.registry_file().exists());
}
