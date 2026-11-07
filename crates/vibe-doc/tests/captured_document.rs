//! Actual materializer-to-lock-to-document reads, including selected failure.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-DOC-POINT",
    r = 1
);
use std::fs;
use vibe_core::manifest::SpecFormat;
use vibe_doc::citations::{SpecSources, resolve};

const URI: &str = "spec://com.example/native/guide.payload#LAW";
const XML: &str = "<spec xmlns='https://vibevm.org/spec/1'><title id='root'>Guide</title><p><LAW fact='true' status='spec/done'>Current captured rule.</LAW></p></spec>";

#[test]
fn bound_installed_census_preserves_original_page_identity_and_retains_write_guard() {
    for format in [SpecFormat::Mixed, SpecFormat::Xml] {
        let (_temp, slot, sources) = installed_kind(format, true);
        assert!(vibe_doc::pages::read_package(&slot).is_err());
        let captured = sources.capture_pages(&slot).unwrap();
        assert_eq!(captured.documents(), ["guide.payload"]);
        assert_eq!(captured.set().pages[0].rel, "guide.payload");
        let options = vibe_doc::build::Options {
            format: vibe_doc::build::Format::Md,
            base: "/doc/".into(),
            manifest: vibe_doc::manifest::Options::at(chrono::Utc::now()),
            derived: Default::default(),
        };
        let prepared = vibe_doc::build::prepare_observed(
            &slot,
            &sources,
            &options,
            &vibe_core::progress::Progress::default(),
        )
        .unwrap();
        assert!(
            prepared
                .built()
                .files
                .iter()
                .any(|file| file.path.ends_with("guide.payload.md"))
        );
        fs::write(
            slot.join(if format == SpecFormat::Mixed {
                "guide.payload"
            } else {
                "guide.xml"
            }),
            "changed",
        )
        .unwrap();
        let out = slot.join("refused-output");
        assert!(prepared.write(&out).is_err());
        assert!(!out.exists());
    }
}

#[test]
fn installed_census_refuses_missing_hash_record_and_format_before_output() {
    for fault in [
        "hash",
        "missing-hash",
        "record",
        "wrong-record",
        "format",
        "missing-format",
    ] {
        let (temp, slot, mut sources) = installed_kind(SpecFormat::Mixed, true);
        let workspace = temp.path().join("workspace");
        match fault {
            "hash" => {
                let path = workspace.join("vibe.lock");
                let text = fs::read_to_string(&path).unwrap();
                fs::write(path, text.replace(&"a".repeat(64), &"b".repeat(64))).unwrap();
            }
            "record" => {
                fs::remove_file(slot.join(vibe_workspace::vibedeps::SLOT_RECORD_FILENAME)).unwrap();
            }
            "missing-hash" => {
                let path = workspace.join("vibe.lock");
                let text = fs::read_to_string(&path).unwrap();
                fs::write(
                    path,
                    text.lines()
                        .filter(|line| !line.starts_with("content_hash="))
                        .collect::<Vec<_>>()
                        .join("\n"),
                )
                .unwrap();
            }
            "wrong-record" => {
                fs::write(
                    slot.join(vibe_workspace::vibedeps::SLOT_RECORD_FILENAME),
                    "invalid record",
                )
                .unwrap();
            }
            "format" => sources = sources.with_materialized_format(SpecFormat::Xml),
            "missing-format" => sources = SpecSources::new().with_lock(&workspace),
            _ => unreachable!(),
        }
        assert!(sources.capture_pages(&slot).is_err(), "{fault}");
        let out = temp.path().join("output");
        fs::create_dir(&out).unwrap();
        fs::write(out.join("prior"), "untouched output").unwrap();
        let options = vibe_doc::build::Options {
            format: vibe_doc::build::Format::Md,
            base: "/doc/".into(),
            manifest: vibe_doc::manifest::Options::at(chrono::Utc::now()),
            derived: Default::default(),
        };
        assert!(
            vibe_doc::build::prepare_observed(
                &slot,
                &sources,
                &options,
                &vibe_core::progress::Progress::default()
            )
            .is_err(),
            "{fault}"
        );
        assert_eq!(
            fs::read_to_string(out.join("prior")).unwrap(),
            "untouched output"
        );
        assert_eq!(fs::read_dir(out).unwrap().count(), 1);
    }
}

#[test]
fn independently_bound_legacy_slot_retains_its_qualified_page_route() {
    let (temp, slot, sources) = installed_policy(SpecFormat::Mixed, true, false);
    assert!(
        slot.join(vibe_workspace::vibedeps::SLOT_RECORD_FILENAME)
            .is_file()
    );
    assert!(
        vibe_doc::pages::read_package(&slot)
            .unwrap_err()
            .to_string()
            .contains("requires independent lock binding")
    );
    assert_eq!(
        vibe_doc::pages::read_package(&temp.path().join("source"))
            .unwrap()
            .pages[0]
            .rel,
        "guide.xml"
    );
    let captured = sources.capture_pages(&slot).unwrap();
    assert_eq!(captured.documents(), ["guide"]);
    assert_eq!(captured.set().pages[0].rel, "guide.xml");
    captured.recheck().unwrap();
}

fn installed(format: SpecFormat) -> (tempfile::TempDir, std::path::PathBuf, SpecSources) {
    installed_kind(format, false)
}

fn installed_kind(
    format: SpecFormat,
    doc: bool,
) -> (tempfile::TempDir, std::path::PathBuf, SpecSources) {
    installed_policy(format, doc, true)
}

fn installed_policy(
    format: SpecFormat,
    doc: bool,
    declared: bool,
) -> (tempfile::TempDir, std::path::PathBuf, SpecSources) {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    let workspace = temp.path().join("workspace");
    fs::create_dir_all(&source).unwrap();
    fs::create_dir_all(&workspace).unwrap();
    let (kind, interpretation, role) = if doc {
        ("doc", "vibe-doc", "documentation")
    } else {
        ("flow", "vibe-spec", "spec")
    };
    fs::write(source.join("vibe.toml"), format!("[package]\ngroup='com.example'\nname='native'\nversion='1.0.0'\nkind='{kind}'\ntitle='Native guide'\nabstract='Captured source fixture.'\n[[package.sources]]\nfile='guide.payload'\nmedia='application/xml'\ninterpretation='{interpretation}'\nrole='{role}'\nloading='explicit'\n")).unwrap();
    if doc {
        let path = source.join("vibe.toml");
        let text = fs::read_to_string(&path).unwrap();
        fs::write(
            path,
            format!("{text}\n[[documents]]\npackage='com.example/subject'\nversion='^1.0'\n"),
        )
        .unwrap();
    }
    vibe_core::manifest::Manifest::read(source.join("vibe.toml")).unwrap();
    fs::write(source.join("guide.payload"), XML).unwrap();
    if !declared {
        let path = source.join("vibe.toml");
        let text = fs::read_to_string(&path).unwrap();
        let (package, rows) = text.split_once("[[package.sources]]").unwrap();
        let (_, relation) = rows.split_once("[[documents]]").unwrap();
        fs::write(path, format!("{package}[[documents]]{relation}")).unwrap();
        fs::create_dir_all(source.join("vibevm/vibespecs")).unwrap();
        fs::rename(
            source.join("guide.payload"),
            source.join("vibevm/vibespecs/guide.xml"),
        )
        .unwrap();
    }
    fs::write(source.join("guide.md"), "literal raw neighbor").unwrap();
    fs::write(
        workspace.join("vibe.toml"),
        "[project]\ngroup='com.example'\nname='host'\nversion='1.0.0'\n",
    )
    .unwrap();
    let group = vibe_core::Group::parse("com.example").unwrap();
    let hash = vibe_core::ContentHash::parse(&format!("sha256:{}", "a".repeat(64))).unwrap();
    let version = "1.0.0".parse().unwrap();
    vibe_workspace::vibedeps::materialise_with_spec_format(
        &workspace,
        &group,
        "native",
        &version,
        &source,
        vibe_workspace::vibedeps::CopyMode::Copy,
        format,
        &hash,
    )
    .unwrap();
    fs::write(workspace.join("vibe.lock"), format!("[meta]\nschema_version=7\ngenerated_by='test'\ngenerated_at='fixed'\n[[package]]\nkind='{kind}'\ngroup='com.example'\nname='native'\nversion='1.0.0'\nsource_url='file:///fixture'\ncontent_hash='{hash}'\n")).unwrap();
    let slot = vibe_workspace::vibedeps::slot_abs_path(&workspace, &group, "native", &version);
    let sources = SpecSources::new()
        .with_lock(&workspace)
        .with_materialized_format(format);
    (temp, slot, sources)
}

#[test]
fn captured_materialized_mixed_and_xml_resolve_and_recheck() {
    for format in [SpecFormat::Mixed, SpecFormat::Xml] {
        let (_temp, slot, sources) = installed(format);
        assert_eq!(
            resolve(URI, &sources).unwrap().text,
            "Current captured rule."
        );
        let captured = sources
            .capture_document(&vibe_spec::SpecAddress::parse(URI).unwrap())
            .unwrap()
            .unwrap();
        let output = if format == SpecFormat::Mixed {
            "guide.payload"
        } else {
            "guide.xml"
        };
        fs::write(slot.join(output), "changed").unwrap();
        assert!(captured.recheck().is_err());
        assert!(resolve(URI, &sources).is_err());
    }
}

#[test]
fn captured_installed_selection_failures_are_not_masked_by_store() {
    for fault in [
        "hash",
        "record",
        "output",
        "format",
        "missing-format",
        "coordinate",
        "version",
        "lock",
    ] {
        let (temp, slot, sources) = installed(SpecFormat::Mixed);
        let workspace = temp.path().join("workspace");
        let store = temp.path().join("store");
        let fallback = store.join("com.example/native/v1.0.0");
        fs::create_dir_all(fallback.join("vibevm/vibespecs")).unwrap();
        fs::write(
            fallback.join("vibe.toml"),
            "[package]\ngroup='com.example'\nname='native'\nversion='1.0.0'\nkind='flow'\n",
        )
        .unwrap();
        fs::write(fallback.join("vibevm/vibespecs/guide.payload.xml"), XML).unwrap();
        assert_eq!(
            resolve(URI, &SpecSources::new().with_store(&store))
                .unwrap()
                .text,
            "Current captured rule."
        );
        let mut sources = sources.with_store(&store);
        let lock = workspace.join("vibe.lock");
        match fault {
            "hash" => {
                let text = fs::read_to_string(&lock).unwrap();
                fs::write(&lock, text.replace(&"a".repeat(64), &"b".repeat(64))).unwrap();
            }
            "record" => {
                fs::remove_file(slot.join(vibe_core::layout::SLOT_RECORD_FILENAME)).unwrap()
            }
            "output" => fs::write(slot.join("guide.payload"), "bad").unwrap(),
            "format" => sources = sources.with_materialized_format(SpecFormat::Xml),
            "missing-format" => {
                sources = SpecSources::new().with_lock(&workspace).with_store(&store)
            }
            "coordinate" => {
                let path = slot.join("vibe.toml");
                let text = fs::read_to_string(&path).unwrap();
                fs::write(path, text.replace("name='native'", "name='other'")).unwrap();
            }
            "version" => {
                assert!(resolve(&URI.replace("native/", "native@2.0.0/"), &sources).is_err());
                continue;
            }
            "lock" => fs::write(lock, "[invalid").unwrap(),
            _ => unreachable!(),
        }
        assert!(resolve(URI, &sources).is_err(), "fault {fault} was masked");
    }
}

#[test]
fn relocated_materialized_original_is_not_an_authored_source() {
    let (_temp, slot, _) = installed(SpecFormat::Mixed);
    let sources = SpecSources::for_checkout(slot, Some("com.example"), "native");
    assert!(
        resolve(URI, &sources)
            .unwrap_err()
            .contains("independent lock binding")
    );
}

#[test]
fn documentation_interpretation_requires_actual_doc_kind_and_xml_media() {
    let temp = tempfile::tempdir().unwrap();
    let manifest = temp.path().join("vibe.toml");
    let policy = "[[package.sources]]\nfile='guide.payload'\nmedia='text/markdown'\ninterpretation='vibe-doc'\nrole='documentation'\nloading='explicit'\n";
    let base = "[package]\ngroup='com.example'\nname='manual'\nkind='doc'\nversion='1.0.0'\ntitle='Guide'\nabstract='Native media fixture.'\n[[documents]]\npackage='com.example/subject'\nversion='^1.0'\n";
    vibe_core::manifest::Manifest::parse_str(&format!(
        "{base}{}",
        policy.replace("text/markdown", "application/xml")
    ))
    .unwrap();
    fs::write(&manifest, format!("{base}{policy}")).unwrap();
    fs::write(temp.path().join("guide.payload"), "# Guide {#root}\n").unwrap();
    let sources = SpecSources::for_checkout(temp.path(), Some("com.example"), "manual");
    assert!(
        vibe_doc::agent::read_page(
            "spec://com.example/manual/guide.payload",
            vibe_doc::build::Format::Md,
            None,
            &sources
        )
        .unwrap_err()
        .to_string()
        .contains("manifest or source policy is invalid")
    );
    fs::write(&manifest, "[project]\ngroup='com.example'\nname='manual'\nversion='1.0.0'\n[[project.sources]]\nfile='guide.payload'\nmedia='application/xml'\ninterpretation='vibe-doc'\nrole='documentation'\nloading='explicit'\n").unwrap();
    fs::write(temp.path().join("guide.payload"), XML).unwrap();
    assert!(
        vibe_doc::agent::read_page(
            "spec://com.example/manual/guide.payload",
            vibe_doc::build::Format::Xml,
            None,
            &sources
        )
        .is_err()
    );
}

#[test]
fn installed_doc_agent_and_missing_adaptation_preserve_validated_selection() {
    for format in [SpecFormat::Mixed, SpecFormat::Xml] {
        let (temp, _, sources) = installed_kind(format, true);
        let address = "spec://com.example/native/guide.payload";
        assert!(
            vibe_doc::agent::read_page(address, vibe_doc::build::Format::Xml, None, &sources)
                .unwrap()
                .text
                .contains("Current captured rule.")
        );
        let workspace = temp.path().join("workspace");
        let adapted = temp.path().join("adapted");
        fs::create_dir_all(&adapted).unwrap();
        let source_manifest = fs::read_to_string(temp.path().join("source/vibe.toml")).unwrap();
        fs::write(
            adapted.join("vibe.toml"),
            source_manifest
                .replace("name='native'", "name='native-ru'")
                .replace("guide.payload", "other.payload"),
        )
        .unwrap();
        fs::write(adapted.join("other.payload"), XML).unwrap();
        let group = vibe_core::Group::parse("com.example").unwrap();
        let version = "1.0.0".parse().unwrap();
        let hash = vibe_core::ContentHash::parse(&format!("sha256:{}", "b".repeat(64))).unwrap();
        vibe_workspace::vibedeps::materialise_with_spec_format(
            &workspace,
            &group,
            "native-ru",
            &version,
            &adapted,
            vibe_workspace::vibedeps::CopyMode::Copy,
            format,
            &hash,
        )
        .unwrap();
        let lock = workspace.join("vibe.lock");
        let old = fs::read_to_string(&lock).unwrap();
        fs::write(&lock, format!("{old}\n[[package]]\nkind='doc'\ngroup='com.example'\nname='native-ru'\nversion='1.0.0'\nsource_url='file:///fixture'\ncontent_hash='{hash}'\n")).unwrap();
        assert_eq!(
            vibe_doc::agent::read_page(address, vibe_doc::build::Format::Xml, Some("ru"), &sources)
                .unwrap()
                .address,
            address
        );
        let slot =
            vibe_workspace::vibedeps::slot_abs_path(&workspace, &group, "native-ru", &version);
        fs::remove_file(slot.join(if format == SpecFormat::Mixed {
            "other.payload"
        } else {
            "other.xml"
        }))
        .unwrap();
        assert!(
            vibe_doc::agent::read_page(address, vibe_doc::build::Format::Xml, Some("ru"), &sources)
                .is_err()
        );
    }
}

#[test]
fn invalid_lock_selected_adaptation_cannot_fall_back_to_valid_canonical_page() {
    let (temp, _, _) = installed(SpecFormat::Mixed);
    let workspace = temp.path().join("workspace");
    let lock = workspace.join("vibe.lock");
    let text = fs::read_to_string(&lock).unwrap();
    fs::write(&lock, text.replace("name='native'", "name='native-ru'")).unwrap();
    let store = temp.path().join("store");
    let canonical = store.join("com.example/native/v1.0.0");
    fs::create_dir_all(canonical.join("vibevm/vibespecs")).unwrap();
    fs::write(canonical.join("vibe.toml"), "[package]\ngroup='com.example'\nname='native'\nkind='doc'\nversion='1.0.0'\ntitle='Canonical guide'\nabstract='A canonical page fixture.'\n[[documents]]\npackage='com.example/subject'\nversion='^1.0'\n").unwrap();
    vibe_core::manifest::Manifest::read(canonical.join("vibe.toml")).unwrap();
    fs::write(canonical.join("vibevm/vibespecs/guide.xml"), "<spec xmlns='https://vibevm.org/spec/1'><title id='root'>Canonical</title><p>Valid canonical page.</p></spec>").unwrap();
    let sources = SpecSources::new()
        .with_lock(&workspace)
        .with_store(&store)
        .with_materialized_format(SpecFormat::Mixed);
    let address = "spec://com.example/native/guide";
    assert!(
        vibe_doc::agent::read_page(address, vibe_doc::build::Format::Xml, None, &sources)
            .unwrap()
            .text
            .contains("Valid canonical page.")
    );
    assert!(
        vibe_doc::agent::read_page(address, vibe_doc::build::Format::Xml, Some("ru"), &sources)
            .is_err()
    );
}
