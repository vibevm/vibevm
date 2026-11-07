use super::*;
use crate::vibedeps::{SlotFile, SlotRecord, write_slot_record};
use std::collections::BTreeMap;
use std::fs;
use vibe_core::{ContentHash, Group, PackageKind};
use vibe_spec::{
    FileResolver, FsSectionSource, SectionSource, SelectedPackage, SelfCoordinate, SpecAddress,
};

const HEADER: &str =
    "[package]\nname='native'\ngroup='org.example'\nkind='flow'\nversion='1.0.0'\n";
const NATIVE: &str = "[[package.sources]]\nfile='vibevm/vibespecs/start.md'\nmedia='application/xml'\ninterpretation='vibe-spec'\nrole='spec'\nloading='scan'\n";
const XML: &[u8] =
    b"<spec xmlns=\"https://vibevm.org/spec/1\"><title id=\"root\">Hello</title><p>body</p></spec>";

struct Fixture {
    _tmp: tempfile::TempDir,
    project: Arc<Project>,
    slot: Pinned,
    relative: String,
    selected: ResolvedDep,
    record: SlotRecord,
}
impl Fixture {
    fn new(format: SpecFormat) -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let project = Arc::new(Project::open(tmp.path()).unwrap());
        let group = Group::parse("org.example").unwrap();
        let version = "1.0.0".parse().unwrap();
        let relative = slot_rel_path(&group, "native", &version);
        fs::create_dir_all(tmp.path().join(&relative).join("vibevm/vibespecs")).unwrap();
        let slot = project
            .dir(&relative.split('/').collect::<Vec<_>>(), false)
            .unwrap();
        let manifest = format!("{HEADER}{NATIVE}");
        let parsed = Manifest::parse_str(&manifest).unwrap();
        let output = if format == SpecFormat::Xml {
            "vibevm/vibespecs/start.xml"
        } else {
            "vibevm/vibespecs/start.md"
        };
        fs::write(slot.join("vibe.toml"), &manifest).unwrap();
        fs::write(slot.join(output), XML).unwrap();
        let source_hash = ContentHash::parse(&format!("sha256:{}", "a".repeat(64))).unwrap();
        let transformed = format != SpecFormat::Mixed;
        let record = SlotRecord {
            schema: 1,
            source_hash: source_hash.clone(),
            spec_format: format,
            converter_recipe: transformed.then(|| CONVERTER_RECIPE.into()),
            derived_hash: transformed.then(|| source_hash.clone()),
            overlay_hash: None,
            files: vec![
                SlotFile {
                    path: "vibe.toml".into(),
                    sha256: sha256_bytes(manifest.as_bytes()),
                    source: transformed.then(|| "vibe.toml".into()),
                    disposition: transformed.then_some(SlotFileDisposition::Copied),
                },
                SlotFile {
                    path: output.into(),
                    sha256: sha256_bytes(XML),
                    source: transformed.then(|| "vibevm/vibespecs/start.md".into()),
                    disposition: transformed.then_some(SlotFileDisposition::Converted),
                },
            ],
        };
        write_slot_record(slot.path(), &record).unwrap();
        let selected = ResolvedDep {
            kind: PackageKind::Flow,
            group,
            name: "native".into(),
            version,
            content_dir: tmp.path().join("not-an-authority"),
            source_hash: Some(source_hash),
            embedded_sources: vec![],
            manifest: parsed,
            requires: vec![],
            admitted_by: None,
            via_override: None,
            source_mutable: false,
            in_place_changed: None,
            configuration: None,
        };
        Self {
            _tmp: tmp,
            project,
            slot,
            relative,
            selected,
            record,
        }
    }
    fn prepare(
        &self,
        format: Option<SpecFormat>,
    ) -> Result<Option<PreparedInstalledSource>, WorkspaceError> {
        prepare_declared_slot_read(
            &self.project,
            &self.slot,
            &self.relative,
            &self.selected,
            format,
        )
    }
    fn resolver(&self, binding: PreparedInstalledSource, installed_self: bool) -> FileResolver {
        let selected = BTreeMap::from([(
            ("org.example".into(), "native".into()),
            SelectedPackage::new("1.0.0", self.slot.path()),
        )]);
        FileResolver::with_selected_world(
            if installed_self {
                self.slot.path()
            } else {
                self.project.root_path()
            },
            self.project.root_path(),
            selected,
            SelfCoordinate::new(
                Some("org.example".into()),
                if installed_self { "native" } else { "host" }.into(),
            ),
        )
        .with_installed_source_bindings(
            self.project.clone(),
            vec![binding],
            installed_self.then(|| ("org.example".into(), "native".into())),
        )
        .unwrap()
    }
}

#[test]
fn prepare_and_read_mixed_wrong_suffix_and_transformed_original_address() {
    for format in [SpecFormat::Mixed, SpecFormat::Xml] {
        let f = Fixture::new(format);
        let binding = f.prepare(Some(format)).unwrap().unwrap();
        let row = binding
            .output_for_source("vibevm/vibespecs/start.md")
            .unwrap();
        assert_eq!(row.native_address.as_deref(), Some("start"));
        assert_eq!(row.media, "application/xml");
        let resolver = f.resolver(binding, true);
        assert_eq!(
            resolver
                .read_installed_source("org.example", "native", "vibevm/vibespecs/start.md", None)
                .unwrap(),
            XML
        );
        let source = FsSectionSource::new(resolver);
        assert!(
            source
                .section_text(&SpecAddress::parse("spec://org.example/native/start#root").unwrap())
                .unwrap()
                .contains("Hello")
        );
    }
}

#[test]
fn prepare_requires_independent_hash_format_and_current_recipe() {
    let mut f = Fixture::new(SpecFormat::Mixed);
    assert!(f.prepare(None).is_err());
    f.selected.source_hash = None;
    assert!(f.prepare(Some(SpecFormat::Mixed)).is_err());
    let mut f = Fixture::new(SpecFormat::Xml);
    f.record.converter_recipe = Some("unknown/1".into());
    write_slot_record(f.slot.path(), &f.record).unwrap();
    assert!(f.prepare(Some(SpecFormat::Xml)).is_err());
}

#[test]
fn removing_declarations_never_hides_recorded_manifest_drift() {
    let mut f = Fixture::new(SpecFormat::Mixed);
    fs::write(f.slot.join("vibe.toml"), HEADER).unwrap();
    assert!(
        f.prepare(Some(SpecFormat::Mixed)).is_err(),
        "independent declared presence must agree"
    );
    f.selected.manifest = Manifest::parse_str(HEADER).unwrap();
    assert!(
        f.prepare(Some(SpecFormat::Mixed)).is_err(),
        "even expected legacy cannot hide stale recorded manifest"
    );
    f.record.files[0].sha256 = sha256_bytes(HEADER.as_bytes());
    write_slot_record(f.slot.path(), &f.record).unwrap();
    assert!(f.prepare(None).unwrap().is_none());
    fs::remove_file(f.slot.join(SLOT_RECORD_FILENAME)).unwrap();
    assert!(
        f.prepare(None).unwrap().is_none(),
        "qualified unrecorded legacy stays compatible"
    );
}

#[test]
fn prepared_installed_read_rechecks_manifest_record_and_output() {
    for path in [
        "vibe.toml",
        SLOT_RECORD_FILENAME,
        "vibevm/vibespecs/start.md",
    ] {
        let f = Fixture::new(SpecFormat::Mixed);
        let binding = f.prepare(Some(SpecFormat::Mixed)).unwrap().unwrap();
        let resolver = f.resolver(binding, false);
        fs::write(f.slot.join(path), "changed").unwrap();
        assert!(
            resolver
                .read_installed_source("org.example", "native", "vibevm/vibespecs/start.md", None)
                .is_err(),
            "{path}"
        );
    }
}

#[test]
fn preparation_rejects_a_different_supplied_slot() {
    let f = Fixture::new(SpecFormat::Mixed);
    let root = f.project.root_dir().unwrap();
    assert!(
        prepare_declared_slot_read(
            &f.project,
            &root,
            &f.relative,
            &f.selected,
            Some(SpecFormat::Mixed)
        )
        .is_err()
    );
}

#[test]
fn converted_native_address_comes_from_original_not_output_suffix() {
    let mut f = Fixture::new(SpecFormat::Xml);
    let manifest = format!("{HEADER}{}", NATIVE.replace("start.md", "start.blob"));
    f.selected.manifest = Manifest::parse_str(&manifest).unwrap();
    fs::write(f.slot.join("vibe.toml"), &manifest).unwrap();
    f.record.files[0].sha256 = sha256_bytes(manifest.as_bytes());
    f.record.files[1].source = Some("vibevm/vibespecs/start.blob".into());
    write_slot_record(f.slot.path(), &f.record).unwrap();
    let binding = f.prepare(Some(SpecFormat::Xml)).unwrap().unwrap();
    assert_eq!(
        binding
            .output_for_source("vibevm/vibespecs/start.blob")
            .unwrap()
            .native_address
            .as_deref(),
        Some("start.blob")
    );
    let resolver = f.resolver(binding, false);
    assert!(
        resolver
            .resolve_file(&SpecAddress::parse("spec://org.example/native/start").unwrap())
            .is_err()
    );
    assert!(
        FsSectionSource::new(resolver)
            .section_text(&SpecAddress::parse("spec://org.example/native/start.blob#root").unwrap())
            .unwrap()
            .contains("Hello")
    );
}

#[test]
fn raw_same_stem_preserves_invalid_bytes_without_stealing_native_address() {
    let mut f = Fixture::new(SpecFormat::Mixed);
    let raw = b"\xff<broken";
    let manifest = format!(
        "{HEADER}{NATIVE}[[package.sources]]\nfile='vibevm/vibespecs/start.xml'\nmedia='application/xml'\ninterpretation='xml-data'\nrole='resource'\nloading='explicit'\n"
    );
    f.selected.manifest = Manifest::parse_str(&manifest).unwrap();
    fs::write(f.slot.join("vibe.toml"), &manifest).unwrap();
    fs::write(f.slot.join("vibevm/vibespecs/start.xml"), raw).unwrap();
    f.record.files[0].sha256 = sha256_bytes(manifest.as_bytes());
    f.record.files.push(SlotFile {
        path: "vibevm/vibespecs/start.xml".into(),
        sha256: sha256_bytes(raw),
        source: None,
        disposition: None,
    });
    write_slot_record(f.slot.path(), &f.record).unwrap();
    let binding = f.prepare(Some(SpecFormat::Mixed)).unwrap().unwrap();
    assert!(
        binding
            .output_for_source("vibevm/vibespecs/start.xml")
            .unwrap()
            .native_address
            .is_none()
    );
    let resolver = f.resolver(binding, false);
    assert_eq!(
        resolver
            .read_installed_source("org.example", "native", "vibevm/vibespecs/start.xml", None)
            .unwrap(),
        raw
    );
    assert!(
        resolver
            .resolve_file(&SpecAddress::parse("spec://org.example/native/start").unwrap())
            .unwrap()
            .ends_with("start.md")
    );
    let native_manifest = manifest.replace(
        "interpretation='xml-data'\nrole='resource'",
        "interpretation='vibe-spec'\nrole='spec'",
    );
    f.selected.manifest = Manifest::parse_str(&native_manifest).unwrap();
    fs::write(f.slot.join("vibe.toml"), &native_manifest).unwrap();
    fs::write(f.slot.join("vibevm/vibespecs/start.xml"), XML).unwrap();
    f.record.files[0].sha256 = sha256_bytes(native_manifest.as_bytes());
    f.record.files[2].sha256 = sha256_bytes(XML);
    write_slot_record(f.slot.path(), &f.record).unwrap();
    let error = f.prepare(Some(SpecFormat::Mixed)).unwrap_err().to_string();
    assert!(error.contains("ambiguous"), "{error}");
}

#[test]
fn copied_native_rows_cannot_bypass_target_media_suffix_or_doc_vocabulary() {
    for markdown in [false, true] {
        let mut f = Fixture::new(SpecFormat::Xml);
        let manifest = format!(
            "{HEADER}{}",
            if markdown {
                NATIVE.replace("application/xml", "text/markdown")
            } else {
                NATIVE.into()
            }
        );
        f.selected.manifest = Manifest::parse_str(&manifest).unwrap();
        fs::write(f.slot.join("vibe.toml"), &manifest).unwrap();
        let bytes = if markdown {
            b"# Text {#root}\n".as_slice()
        } else {
            XML
        };
        fs::write(f.slot.join("vibevm/vibespecs/start.md"), bytes).unwrap();
        f.record.files[0].sha256 = sha256_bytes(manifest.as_bytes());
        f.record.files[1].path = "vibevm/vibespecs/start.md".into();
        f.record.files[1].sha256 = sha256_bytes(bytes);
        f.record.files[1].disposition = Some(SlotFileDisposition::Copied);
        write_slot_record(f.slot.path(), &f.record).unwrap();
        let error = f.prepare(Some(SpecFormat::Xml)).unwrap_err().to_string();
        assert!(error.contains("native media/output"), "{error}");
    }
    let mut f = Fixture::new(SpecFormat::Xml);
    let manifest = format!(
        "{}title='Doc'\nabstract='A documented package'\n{}\n[[documents]]\npackage='org.example/host'\nversion='^1'\n",
        HEADER.replace("kind='flow'", "kind='doc'"),
        NATIVE
            .replace("interpretation='vibe-spec'", "interpretation='vibe-doc'")
            .replace("role='spec'", "role='documentation'")
    );
    f.selected.kind = PackageKind::Doc;
    f.selected.manifest = Manifest::parse_str(&manifest).unwrap();
    fs::write(f.slot.join("vibe.toml"), &manifest).unwrap();
    fs::write(f.slot.join("vibevm/vibespecs/start.md"), XML).unwrap();
    f.record.spec_format = SpecFormat::Markdown;
    f.record.files[0].sha256 = sha256_bytes(manifest.as_bytes());
    f.record.files[1].path = "vibevm/vibespecs/start.md".into();
    f.record.files[1].disposition = Some(SlotFileDisposition::Copied);
    write_slot_record(f.slot.path(), &f.record).unwrap();
    assert!(
        f.prepare(Some(SpecFormat::Markdown))
            .unwrap_err()
            .to_string()
            .contains("unsupported documentation")
    );
}

#[test]
fn equal_package_metadata_cannot_hide_selected_boot_intent_drift() {
    let mut f = Fixture::new(SpecFormat::Mixed);
    let manifest = format!(
        "{HEADER}{NATIVE}[boot_snippet]\nsource='vibevm/vibespecs/start.md'\nlink='dynamic'\n"
    );
    let observed = Manifest::parse_str(&manifest).unwrap();
    assert_eq!(observed.package, f.selected.manifest.package);
    assert_ne!(observed.boot_snippet, f.selected.manifest.boot_snippet);
    fs::write(f.slot.join("vibe.toml"), &manifest).unwrap();
    f.record.files[0].sha256 = sha256_bytes(manifest.as_bytes());
    write_slot_record(f.slot.path(), &f.record).unwrap();
    assert!(
        f.prepare(Some(SpecFormat::Mixed))
            .unwrap_err()
            .to_string()
            .contains("full manifest")
    );
}

#[test]
fn native_filename_cannot_invent_an_address_fragment() {
    let mut f = Fixture::new(SpecFormat::Mixed);
    let manifest = format!("{HEADER}{}", NATIVE.replace("start.md", "part#one.md"));
    f.selected.manifest = Manifest::parse_str(&manifest).unwrap();
    fs::write(f.slot.join("vibe.toml"), &manifest).unwrap();
    fs::write(f.slot.join("vibevm/vibespecs/part#one.md"), XML).unwrap();
    f.record.files[0].sha256 = sha256_bytes(manifest.as_bytes());
    f.record.files[1].path = "vibevm/vibespecs/part#one.md".into();
    write_slot_record(f.slot.path(), &f.record).unwrap();
    assert!(
        f.prepare(Some(SpecFormat::Mixed))
            .unwrap_err()
            .to_string()
            .contains("URI selectors")
    );
}

#[test]
fn empty_and_export_only_policy_never_restore_native_addresses() {
    for policy in ["sources=[]\n", "source_exports=[]\n"] {
        let mut f = Fixture::new(SpecFormat::Mixed);
        let manifest = format!("{HEADER}{policy}");
        f.selected.manifest = Manifest::parse_str(&manifest).unwrap();
        fs::write(f.slot.join("vibe.toml"), &manifest).unwrap();
        f.record.files[0].sha256 = sha256_bytes(manifest.as_bytes());
        write_slot_record(f.slot.path(), &f.record).unwrap();
        let binding = f.prepare(Some(SpecFormat::Mixed)).unwrap().unwrap();
        assert!(
            binding
                .outputs()
                .iter()
                .all(|row| row.native_address.is_none())
        );
        let resolver = f.resolver(binding, false);
        assert!(
            resolver
                .resolve_file(&SpecAddress::parse("spec://org.example/native/start").unwrap())
                .is_err()
        );
        assert!(
            resolver
                .expand_pattern(&SpecAddress::parse("spec://org.example/*/start").unwrap())
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            resolver
                .read_installed_source("org.example", "native", "vibevm/vibespecs/start.md", None)
                .unwrap(),
            XML
        );
        fs::write(f.slot.join(SLOT_RECORD_FILENAME), "changed").unwrap();
        assert!(
            resolver
                .expand_pattern(&SpecAddress::parse("spec://org.example/*/start").unwrap())
                .is_err()
        );
    }
}
