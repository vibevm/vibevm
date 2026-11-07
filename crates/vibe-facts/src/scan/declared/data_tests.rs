//! Optional summary failure is independent of captured native fact observation.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-DATA-INSPECTION");
use super::*;
use crate::{CapturedDeclaredDocument, observe_declared_source};
use vibe_specdoc::xml_data::{DataInspection, InspectionLimits, inspect_xml_data, preserve_opaque};
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-DATA-INSPECTION",
    r = 1
)]
fn optional_data_inspection_preserves_raw_and_unrelated_native_facts() {
    let package: PackageMeta=toml::from_str("name='data-example'\ngroup='org.example'\nkind='flow'\nversion='1.0.0'\n[[sources]]\nfile='data.xml'\nmedia='application/xml'\ninterpretation='xml-data'\nrole='resource'\nloading='explicit'\n[[sources]]\nfile='native.bin'\nmedia='application/xml'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n").unwrap();
    let native=b"<spec xmlns=\"https://vibevm.org/spec/1\"><p><REAL fact=\"true\" status=\"spec/done\">one native fact</REAL></p></spec>";
    for raw in [
        b"<broken>".as_slice(),
        b"\xff\0<not-utf8>",
        b"<!DOCTYPE r SYSTEM 'https://example.invalid/not-fetched'><r/>",
    ] {
        let inputs = [
            CapturedDeclaredDocument {
                path: "data.xml",
                bytes: Some(raw),
            },
            CapturedDeclaredDocument {
                path: "native.bin",
                bytes: Some(native),
            },
        ];
        let before = observe_declared_source(&package, &inputs).unwrap();
        assert!(before.complete);
        assert_eq!(before.facts.len(), 1);
        assert!(before.facts[0].address.ends_with("#REAL"));
        let inspected = inspect_xml_data(raw, InspectionLimits::unlimited());
        assert!(matches!(
            inspected.inspection(),
            DataInspection::Unavailable(_)
        ));
        assert!(std::ptr::eq(inspected.raw(), raw));
        assert_eq!(preserve_opaque(raw).raw(), raw);
        let after = observe_declared_source(&package, &inputs).unwrap();
        assert_eq!(before, after);
        let (_, ordinary) = after
            .documents
            .iter()
            .find(|(path, _)| path == "data.xml")
            .unwrap();
        assert!(matches!(ordinary,DeclaredDocumentObservation::Raw{bytes,..} if *bytes==raw));
    }
}
#[test]
fn xml_looking_vibe_fact_tags_remain_only_an_ordinary_data_summary() {
    let package: PackageMeta=toml::from_str("name='ordinary'\ngroup='org.example'\nkind='flow'\nversion='1.0.0'\n[[sources]]\nfile='ordinary.xml'\nmedia='application/xml'\ninterpretation='xml-data'\nrole='resource'\nloading='explicit'\n").unwrap();
    let raw=b"<spec xmlns=\"https://vibevm.org/spec/1\"><p><FAKE fact=\"true\" status=\"impl/done\">not a native claim</FAKE></p></spec>";
    let summary = inspect_xml_data(raw, InspectionLimits::unlimited());
    assert!(
        matches!(summary.inspection(),DataInspection::Available(outline) if outline.root=="spec" && outline.elements==3)
    );
    let tree = observe_declared_source(
        &package,
        &[CapturedDeclaredDocument {
            path: "ordinary.xml",
            bytes: Some(summary.raw()),
        }],
    )
    .unwrap();
    assert!(tree.complete);
    assert!(tree.facts.is_empty());
}
