//! The T10B pins on the boot adapter's typed providers: every reachable arm
//! is named exactly, and `Undetermined` is unreachable for every input this
//! adapter builds.
//!
//! The unreachability half is asserted twice, deliberately. Once as a
//! property of the ONE function that decides a provider — it never returns
//! that arm, for any provenance and any node coordinate — and once as a
//! property of the CELL, so a future edit cannot reintroduce the arm through
//! a different route (a fallback, a legacy constructor) while the first
//! assertion keeps passing.

use specmark::verifies;
use vibe_core::manifest::{LinkType, PackageFormat};

use super::*;
use crate::boot::BootBand;

/// One dependency-declared static entry.
fn dependency_entry() -> BootEntry {
    BootEntry {
        path: "vibedeps/tools/1.0.0/vibevm/vibespecs/boot/entry.md".to_string(),
        band: BootBand::Dependency,
        link: LinkType::Static,
        when: None,
        // Display carries the hoist suffix; identity does not.
        origin: "org.demo/tools [shared by org.demo/a, org.demo/b]".to_string(),
        provenance: BootProvenance::Dependency {
            group: Group::parse("org.demo").expect("a valid test group"),
            name: "tools".to_string(),
        },
        use_ref: false,
        format: PackageFormat::Simple,
        unit_substituted: false,
        elided: false,
    }
}

/// One node-declared (authored boot) static entry.
fn node_entry() -> BootEntry {
    BootEntry {
        provenance: BootProvenance::Node,
        origin: ".".to_string(),
        ..dependency_entry()
    }
}

/// Every node coordinate shape a workspace root can have.
fn coordinates() -> Vec<(SelfCoordinate, DocumentProvider)> {
    vec![
        (
            SelfCoordinate::new(Some("org.vibevm.core".into()), "vibevm".into()),
            DocumentProvider::HostCoordinate {
                group: Group::parse("org.vibevm.core").expect("a valid group"),
                name: PackageName::parse("vibevm").expect("a valid name"),
            },
        ),
        (
            SelfCoordinate::new(None, "demo".into()),
            DocumentProvider::HostUngrouped {
                name: "demo".to_string(),
            },
        ),
        (
            SelfCoordinate::new(None, String::new()),
            DocumentProvider::HostVirtualWorkspace,
        ),
    ]
}

/// Every reachable arm, named exactly — and the dependency arm proves the
/// `[shared by …]` display suffix never reaches identity.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-SELECTOR")]
fn every_reachable_provider_arm_is_named_from_the_typed_pair() {
    let any_coordinate = SelfCoordinate::new(Some("org.vibevm.core".into()), "vibevm".into());
    assert_eq!(
        document_provider(&dependency_entry(), &any_coordinate).expect("a typed dependency"),
        DocumentProvider::Dependency {
            group: Group::parse("org.demo").expect("a valid group"),
            name: PackageName::parse("tools").expect("a valid name"),
        },
        "the suffix is display; the provider is the pair carried beside it"
    );

    for (coordinate, expected) in coordinates() {
        assert_eq!(
            document_provider(&node_entry(), &coordinate).expect("a typed host"),
            expected,
            "the host arms mirror the world adapter's own projection"
        );
    }
}

/// `Undetermined` is unreachable, as a property of the deciding function:
/// every provenance, crossed with every node coordinate shape, answers a
/// determinate provider.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-SELECTOR")]
fn undetermined_is_unreachable_for_every_input_the_adapter_builds() {
    for entry in [dependency_entry(), node_entry()] {
        for (coordinate, _) in coordinates() {
            let provider =
                document_provider(&entry, &coordinate).expect("every fixture is well typed");
            assert_ne!(
                provider,
                DocumentProvider::Undetermined,
                "a declared boot document always has a determinate provider"
            );
        }
    }
}

/// Identity comes from the typed pair EVEN WHEN the display string
/// disagrees. In production the two are authored from the same values and
/// cannot diverge; this fixture forces a divergence precisely to pin the
/// priority — a `document_provider` that answered the display coordinate
/// here would be recovering identity from a rendering, which is the exact
/// mutation the typed carriage exists to make impossible.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-SELECTOR")]
fn identity_comes_from_the_typed_pair_even_when_display_disagrees() {
    let entry = BootEntry {
        origin: "org.decoy/visual".to_string(),
        ..dependency_entry()
    };
    let coordinate = SelfCoordinate::new(Some("org.vibevm.core".into()), "vibevm".into());
    assert_eq!(
        document_provider(&entry, &coordinate).expect("the typed pair is well formed"),
        DocumentProvider::Dependency {
            group: Group::parse("org.demo").expect("a valid group"),
            name: PackageName::parse("tools").expect("a valid name"),
        },
        "the decoy display spelling moved nothing: identity is the carried pair"
    );
}

/// A component the install model still carries as a bare string is REFUSED
/// when it does not spell its grammar — never defaulted, never panicked on.
/// That refusal is what makes the unreachability above a law rather than a
/// property of well-formed fixtures.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-SELECTOR")]
fn an_untypeable_component_refuses_instead_of_falling_back() {
    let entry = BootEntry {
        provenance: BootProvenance::Dependency {
            group: Group::parse("org.demo").expect("a valid group"),
            name: "Not A Name".to_string(),
        },
        ..dependency_entry()
    };
    let coordinate = SelfCoordinate::new(Some("org.vibevm.core".into()), "vibevm".into());
    let error = document_provider(&entry, &coordinate)
        .expect_err("a non-kebab package name is not an identity");
    let WorkspaceError::UntypedBootProvenance {
        component,
        spelling,
        ..
    } = &error
    else {
        panic!("the typed-provenance refusal has its own arm: {error}")
    };
    assert_eq!(*component, "dependency name");
    assert_eq!(spelling, "Not A Name");
}

/// Every line of the cell that the compiler actually sees — comment lines
/// dropped.
///
/// The fence below is a substring fence, so it needs prose kept out of its
/// way: this cell's own doc comments legitimately NAME the undetermined arm
/// while explaining why nothing here may produce one. Dropping `//` lines is
/// the whole distinction, and it is mechanical: a comment line carries no
/// Rust. The one gap a substring fence keeps — a needle hidden inside a
/// string literal — is closed by the assertion that this cell contains no
/// string literal at all beyond its error vocabulary.
fn code_of(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The cell-level half of the unreachability pin: the two document-producing
/// kinds are built through the TYPED constructors, and the cell's code never
/// names the undetermined arm at all — so no fallback, no legacy call and no
/// "just this once" can reintroduce it while the property test above still
/// passes.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-SELECTOR")]
fn the_adapter_cell_names_no_undetermined_and_no_untyped_document_constructor() {
    let source = code_of(include_str!("inputs.rs"));
    let source = source.as_str();
    for typed in [
        "ArtifactInput::normal_declared_by(",
        "ArtifactInput::simple_declared_by(",
    ] {
        assert!(
            source.contains(typed),
            "the document-producing kinds are built through `{typed}`"
        );
    }
    for untyped in ["ArtifactInput::normal(", "ArtifactInput::simple("] {
        assert!(
            !source.contains(untyped),
            "`{untyped}` mints `Undetermined`; the adapter has the typed pair and must use it"
        );
    }
    assert!(
        !source.contains("Undetermined"),
        "the adapter never names the undetermined arm — not as a value, not as a fallback"
    );
    // The document-less kinds keep their untyped constructors, and that is
    // the point: they produce no document to judge.
    for document_less in ["ArtifactInput::elided(", "ArtifactInput::hoisted("] {
        assert!(source.contains(document_less));
    }
    // The fence's own precondition: with comment lines gone, every remaining
    // `"` opens a diagnostic string, and none of them may carry a needle.
    for line in source.lines().filter(|line| line.contains('"')) {
        assert!(
            !line.contains("Undetermined"),
            "a string literal must not hide the arm from this fence: {line}"
        );
    }
}

#[test]
fn bound_simple_boot_projects_only_selected_vibe_and_refuses_raw_same_stem() {
    use std::fs;

    use vibe_core::ContentHash;
    use vibe_core::manifest::SpecFormat;
    use vibe_spec::{CompilePlans, TransformPlan};

    let workspace = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    let group = Group::parse("org.example").unwrap();
    let version = semver::Version::parse("1.1.0").unwrap();
    let specs = vibe_core::layout::current_specs_root();
    let vibe = specs.join("common/rule.xml");
    let raw = specs.join("common/rule.md");
    let xml_data = specs.join("common/data.xml");
    let opaque = specs.join("common/opaque.xml");
    fs::create_dir_all(source.path().join(&vibe).parent().unwrap()).unwrap();
    fs::write(source.path().join(&vibe), b"<spec xmlns='https://vibevm.org/spec/1'><p><RULE fact='true'>selected simple boot</RULE></p></spec>\n").unwrap();
    fs::write(
        source.path().join(&raw),
        b"# Raw\n\n@fact:FAKE stays literal.\n",
    )
    .unwrap();
    for path in [&xml_data, &opaque] {
        fs::write(
            source.path().join(path),
            b"<spec xmlns='https://vibevm.org/spec/1'><p><FAKE fact='true'>raw</FAKE></p></spec>\n",
        )
        .unwrap();
    }
    let wire = |path: &Path| path.to_string_lossy().replace('\\', "/");
    fs::write(source.path().join("vibe.toml"), format!(
        "[package]\nname='simple'\ngroup='org.example'\nkind='flow'\nversion='1.1.0'\nformat='simple'\n\
         [[package.sources]]\nfile='{}'\nmedia='application/xml'\ninterpretation='vibe-spec'\nrole='spec'\nloading='scan'\n\
         [[package.sources]]\nfile='{}'\nmedia='text/markdown'\ninterpretation='commonmark'\nrole='documentation'\nloading='scan'\n\
         [[package.sources]]\nfile='{}'\nmedia='application/xml'\ninterpretation='xml-data'\nrole='resource'\nloading='scan'\n\
         [[package.sources]]\nfile='{}'\nmedia='application/octet-stream'\ninterpretation='opaque'\nrole='resource'\nloading='scan'\n",
        wire(&vibe), wire(&raw), wire(&xml_data), wire(&opaque)
    )).unwrap();
    let hash = ContentHash::parse(&format!("sha256:{}", "1".repeat(64))).unwrap();
    crate::vibedeps::materialise_with_spec_format(
        workspace.path(),
        &group,
        "simple",
        &version,
        source.path(),
        crate::vibedeps::CopyMode::Copy,
        SpecFormat::Mixed,
        &hash,
    )
    .unwrap();
    let slot = crate::vibedeps::slot_rel_path(&group, "simple", &version);
    let witness = SelectedSlotWitness {
        group: group.clone(),
        name: "simple".into(),
        version,
        root: workspace.path().join(&slot),
        source_hash: Some(hash),
        spec_format: SpecFormat::Mixed,
    };
    let entry = BootEntry {
        path: layout_paths::slot_specs(slot.clone(), "common/rule.xml"),
        band: BootBand::Dependency,
        link: LinkType::Static,
        when: None,
        origin: "org.example/simple".into(),
        provenance: BootProvenance::Dependency {
            group,
            name: "simple".into(),
        },
        use_ref: false,
        format: PackageFormat::Simple,
        unit_substituted: false,
        elided: false,
    };
    let coordinate = SelfCoordinate::new(Some("org.host".into()), "host".into());
    let plans = || CompilePlans::transforms_only(TransformPlan::empty());
    let render = |entry: BootEntry, witness: &SelectedSlotWitness| {
        super::super::render_static_observed_selected(
            &crate::boot::EffectiveBoot {
                entries: vec![entry],
            },
            workspace.path(),
            &coordinate,
            SpecFormat::Mixed,
            None,
            plans(),
            std::slice::from_ref(witness),
        )
    };
    let rendered = render(entry.clone(), &witness).unwrap().unwrap();
    assert!(rendered.contains("selected simple boot"), "{rendered}");
    for raw_path in ["common/rule.md", "common/data.xml", "common/opaque.xml"] {
        let raw_entry = BootEntry {
            path: layout_paths::slot_specs(slot.clone(), raw_path),
            ..entry.clone()
        };
        let raw_error = render(raw_entry, &witness).unwrap_err().to_string();
        assert!(
            raw_error.contains("not a selected VibeSpec"),
            "{raw_path}: {raw_error}"
        );
    }
    let materialized_manifest = witness.root.join("vibe.toml");
    let original_manifest = fs::read(&materialized_manifest).unwrap();
    fs::write(
        &materialized_manifest,
        "[package]\nname='simple'\ngroup='org.example'\nkind='flow'\nversion='1.1.0'\nformat='simple'\n",
    )
    .unwrap();
    let stripped = render(entry.clone(), &witness).unwrap_err().to_string();
    assert!(
        stripped.contains("installed manifest bytes differ from the slot record"),
        "{stripped}"
    );
    fs::write(materialized_manifest, original_manifest).unwrap();
    let mut foreign = witness.clone();
    foreign.source_hash = None;
    let missing = render(entry.clone(), &foreign).unwrap_err().to_string();
    assert!(
        missing.contains("independent selected source hash"),
        "{missing}"
    );
    foreign.source_hash = Some(ContentHash::parse(&format!("sha256:{}", "2".repeat(64))).unwrap());
    let foreign_error = render(entry, &foreign).unwrap_err().to_string();
    assert!(
        foreign_error.contains("source hash or format differs"),
        "{foreign_error}"
    );
}
