//! Actual install → declared facts adoption, including no-write refusals.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-OBSERVATION-LAW");

mod common;

use common::UserScratch;
use std::fs;
use std::path::PathBuf;

const PACKAGE: &str = "org.example/declared-facts";
const XML: &str = "<spec xmlns=\"https://vibevm.org/spec/1\"><p><REAL fact=\"true\" status=\"impl/done\">Real rule.</REAL></p></spec>";
const FAKE: &str = "# Ordinary\n\n@fact:FAKE Ordinary text. @status:impl/done\n";

struct Fixture {
    user: UserScratch,
    project: tempfile::TempDir,
    _registry: tempfile::TempDir,
    source: PathBuf,
    slot: PathBuf,
}

fn declaration(file: &str, media: &str, interpretation: &str, role: &str) -> String {
    format!(
        "\n[[package.sources]]\nfile='{file}'\nmedia='{media}'\ninterpretation='{interpretation}'\nrole='{role}'\nloading='scan'\n"
    )
}

impl Fixture {
    fn install(policy: &str) -> Self {
        let user = UserScratch::new();
        let project = tempfile::tempdir().unwrap();
        let registry = tempfile::tempdir().unwrap();
        let source = registry.path().join("org.example/declared-facts/v1.0.0");
        fs::create_dir_all(source.join(common::specs_root())).unwrap();
        fs::write(source.join("vibe.toml"), format!(
            "[package]\ngroup='org.example'\nname='declared-facts'\nkind='flow'\nversion='1.0.0'\nepoch=1\nformat='normal'\n{policy}"
        )).unwrap();
        fs::write(source.join(common::spec_rel("rule.md")), XML).unwrap();
        fs::write(source.join(common::spec_rel("guide.md")), FAKE).unwrap();
        if !policy.is_empty() {
            fs::write(
                source.join(common::spec_rel("rule.xml")),
                XML.replace("REAL", "FAKE_PAIR"),
            )
            .unwrap();
            fs::write(
                source.join(common::spec_rel("opaque.md")),
                FAKE.replace("FAKE", "FAKE_OPAQUE"),
            )
            .unwrap();
        }
        fs::write(
            source.join(common::spec_rel("data.xml")),
            XML.replace("REAL", "FAKE_XML"),
        )
        .unwrap();
        fs::write(
            source.join(common::spec_rel("unmatched.md")),
            FAKE.replace("FAKE", "UNMATCHED"),
        )
        .unwrap();
        fs::write(
            project.path().join("vibe.toml"),
            "[project]\ngroup='org.consumer'\nname='demo'\nversion='1.0.0'\nspec_format='mixed'\n",
        )
        .unwrap();
        let output = user
            .vibe()
            .args(["install", "org.example/declared-facts@=1.0.0", "--path"])
            .arg(project.path())
            .arg("--registry")
            .arg(registry.path())
            .arg("--assume-yes")
            .output()
            .unwrap();
        assert!(output.status.success(), "install: {output:?}");
        let slot = project
            .path()
            .join(common::slot_dir("org.example.declared-facts", "1.0.0"));
        Self {
            user,
            project,
            _registry: registry,
            source,
            slot,
        }
    }
    fn explicit() -> Self {
        let policy = declaration(
            &common::spec_rel("rule.md"),
            "application/xml",
            "vibe-spec",
            "spec",
        ) + &declaration(
            &common::spec_rel("guide.md"),
            "text/markdown",
            "commonmark",
            "documentation",
        ) + &declaration(
            &common::spec_rel("data.xml"),
            "application/xml",
            "xml-data",
            "resource",
        ) + &declaration(
            &common::spec_rel("rule.xml"),
            "application/xml",
            "xml-data",
            "resource",
        ) + &declaration(
            &common::spec_rel("opaque.md"),
            "application/octet-stream",
            "opaque",
            "resource",
        );
        Self::install(&policy)
    }
    fn adopt(&self) -> std::process::Output {
        self.user
            .vibe()
            .current_dir(self.project.path())
            .args(["facts", "adopt", "--package", PACKAGE])
            .output()
            .unwrap()
    }
    fn facts_path(&self) -> PathBuf {
        self.project
            .path()
            .join(common::facts_rel("org.example.declared-facts.toml"))
    }
    fn assert_refused(&self, diagnostic: &str) {
        let before = fs::read(self.facts_path()).ok();
        let source_before = fs::read(self.source.join(common::spec_rel("rule.md"))).unwrap();
        let output = self.adopt();
        assert!(!output.status.success(), "{output:?}");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(diagnostic),
            "{output:?}"
        );
        assert_eq!(
            fs::read(self.facts_path()).ok(),
            before,
            "refusal changed registry"
        );
        assert_eq!(
            fs::read(self.source.join(common::spec_rel("rule.md"))).unwrap(),
            source_before
        );
    }
}

#[test]
fn actual_mixed_install_adopts_declared_media_and_repeat_keeps_existing() {
    let f = Fixture::explicit();
    let before = fs::read(f.slot.join(common::spec_rel("rule.md"))).unwrap();
    let output = f.adopt();
    assert!(output.status.success(), "{output:?}");
    assert!(String::from_utf8_lossy(&output.stdout).contains("added=1 kept=0"));
    let facts = fs::read_to_string(f.facts_path()).unwrap();
    assert!(facts.contains("/rule#REAL"), "{facts}");
    assert!(
        !facts.contains("FAKE") && !facts.contains("UNMATCHED"),
        "{facts}"
    );
    let repeat = f.adopt();
    assert!(repeat.status.success(), "{repeat:?}");
    assert!(String::from_utf8_lossy(&repeat.stdout).contains("added=0 kept=1"));
    assert_eq!(fs::read_to_string(f.facts_path()).unwrap(), facts);
    assert_eq!(
        fs::read(f.slot.join(common::spec_rel("rule.md"))).unwrap(),
        before
    );
    assert_eq!(
        fs::read_to_string(f.source.join(common::spec_rel("rule.md"))).unwrap(),
        XML
    );
}

#[test]
fn explicit_empty_adopts_zero_despite_raw_fact_markers() {
    let f = Fixture::install("sources=[]\n");
    let output = f.adopt();
    assert!(output.status.success(), "{output:?}");
    assert!(String::from_utf8_lossy(&output.stdout).contains("added=0 kept=0"));
    assert!(!f.facts_path().exists());
}

#[test]
fn absent_descriptors_preserve_legacy_suffix_adoption() {
    let f = Fixture::install("");
    let output = f.adopt();
    assert!(output.status.success(), "{output:?}");
    let facts = fs::read_to_string(f.facts_path()).unwrap();
    assert!(
        facts.contains("#FAKE") && facts.contains("#UNMATCHED"),
        "{facts}"
    );
}

#[test]
fn removed_declarations_do_not_bypass_recorded_manifest_drift() {
    let f = Fixture::explicit();
    let path = f.slot.join("vibe.toml");
    let manifest = fs::read_to_string(&path).unwrap();
    fs::write(path, manifest.split("[[package.sources]]").next().unwrap()).unwrap();
    f.assert_refused("recorded slot payload");
}

#[test]
fn malformed_policy_refuses_without_legacy_fallback() {
    let f = Fixture::explicit();
    let path = f.slot.join("vibe.toml");
    let manifest = fs::read_to_string(&path).unwrap();
    fs::write(path, manifest.replace("vibe-spec", "guess-by-suffix")).unwrap();
    f.assert_refused("source policy is invalid");
}

#[test]
fn directory_and_ambiguous_policy_refuse_even_with_consistent_editable_record() {
    for directory in [false, true] {
        let f = Fixture::explicit();
        let path = f.slot.join("vibe.toml");
        let manifest = fs::read_to_string(&path).unwrap();
        let altered = if directory {
            manifest.replace(
                &format!("file='{}'", common::spec_rel("rule.md")),
                &format!("directory='{}'", common::specs_str()),
            )
        } else {
            manifest
                + &declaration(
                    &common::spec_rel("rule.md"),
                    "application/xml",
                    "xml-data",
                    "resource",
                )
        };
        fs::write(path, &altered).unwrap();
        // The record is editable consistency metadata, not authentication.
        // Keep its manifest witness consistent to exercise semantic refusal.
        let mut record = vibe_workspace::vibedeps::read_slot_record(&f.slot).unwrap();
        record
            .files
            .iter_mut()
            .find(|row| row.path == "vibe.toml")
            .unwrap()
            .sha256 = vibe_workspace::vibedeps::sha256_bytes(altered.as_bytes());
        vibe_workspace::vibedeps::write_slot_record(&f.slot, &record).unwrap();
        f.assert_refused(if directory {
            "DirectoryCensusRequired"
        } else {
            "incomplete"
        });
    }
}

#[test]
fn transformed_record_refuses_even_when_well_formed() {
    let f = Fixture::explicit();
    let mut record = vibe_workspace::vibedeps::read_slot_record(&f.slot).unwrap();
    record.spec_format = vibe_core::manifest::SpecFormat::Xml;
    record.converter_recipe = Some("test-transformed-record".into());
    record.derived_hash = Some(record.source_hash.clone());
    for row in &mut record.files {
        row.source = Some(row.path.clone());
        row.disposition = Some(vibe_workspace::vibedeps::SlotFileDisposition::Copied);
    }
    vibe_workspace::vibedeps::write_slot_record(&f.slot, &record).unwrap();
    f.assert_refused("unchanged Mixed slot record");
}

#[test]
fn malformed_user_config_is_admitted_before_any_registry_write() {
    let f = Fixture::explicit();
    let config = f.user.settings.join("bad-config.toml");
    fs::write(&config, "[broken").unwrap();
    let output = f
        .user
        .vibe()
        .env("VIBEVM_USER_CONFIG", config)
        .current_dir(f.project.path())
        .args(["facts", "adopt", "--package", PACKAGE])
        .output()
        .unwrap();
    assert!(!output.status.success(), "{output:?}");
    assert!(!f.facts_path().exists());
}

#[test]
fn declared_document_missing_or_drift_refuses_before_registry_write() {
    for missing in [false, true] {
        let f = Fixture::explicit();
        let path = f.slot.join(common::spec_rel("rule.md"));
        if missing {
            fs::remove_file(path).unwrap();
        } else {
            fs::write(path, XML.replace("Real rule", "Drifted rule")).unwrap();
        }
        f.assert_refused(if missing {
            "missing"
        } else {
            "recorded slot payload"
        });
    }
}

#[test]
fn declared_manifest_record_and_lock_drift_refuse_before_registry_write() {
    for drift in ["manifest", "record", "lock"] {
        let f = Fixture::explicit();
        let path = match drift {
            "manifest" => f.slot.join("vibe.toml"),
            "record" => f.slot.join(vibe_workspace::vibedeps::SLOT_RECORD_FILENAME),
            _ => f.project.path().join("vibe.lock"),
        };
        let before = fs::read_to_string(&path).unwrap();
        let after = match drift {
            "manifest" => format!("{before}\n# changed manifest\n"),
            _ => before.replace("sha256:", "sha256:0"),
        };
        fs::write(path, after).unwrap();
        f.assert_refused(if drift == "manifest" {
            "recorded slot payload"
        } else {
            "source hash"
        });
    }
}

#[test]
fn changed_effective_target_and_invalid_root_config_refuse_before_registry_write() {
    for malformed in [false, true] {
        let f = Fixture::explicit();
        let path = f.project.path().join("vibe.toml");
        let before = fs::read_to_string(&path).unwrap();
        fs::write(
            path,
            if malformed {
                "[broken".to_owned()
            } else {
                before.replace("mixed", "xml")
            },
        )
        .unwrap();
        f.assert_refused(if malformed {
            "parse"
        } else {
            "effective Mixed"
        });
    }
}

#[cfg(unix)]
#[test]
fn selected_slot_and_registry_symlinks_refuse_without_touching_redirects() {
    for registry in [false, true] {
        let f = Fixture::explicit();
        let outside = tempfile::tempdir().unwrap();
        if registry {
            let target = f.project.path().join(common::facts_root());
            fs::create_dir_all(target.parent().unwrap()).unwrap();
            std::os::unix::fs::symlink(outside.path(), target).unwrap();
        } else {
            let moved = outside.path().join("slot");
            fs::rename(&f.slot, &moved).unwrap();
            std::os::unix::fs::symlink(moved, &f.slot).unwrap();
        }
        f.assert_refused(if registry { "symlink" } else { "no-follow" });
        assert!(
            !outside
                .path()
                .join("org.example.declared-facts.toml")
                .exists()
        );
    }
}
