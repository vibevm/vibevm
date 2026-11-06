use super::*;
#[cfg(test)]
mod cases {
    use super::*;
    use crate::{CapturedDeclaredDocument, observe_declared_document, observe_declared_source};
    const RAW: &[u8] = b"# Guide\n## Intro\n@fact:FAKE #use spec://org.fake/pkg/RULE and @requires:plan.\n# Other\n## Intro\nOther.\n";
    fn package(headings: &str) -> PackageMeta {
        toml::from_str(&format!("name='demo'\ngroup='org.demo'\nkind='flow'\nversion='1.0.0'\n[[sources]]\nfile='guide.xml'\nmedia='text/markdown'\ninterpretation='commonmark'\nrole='documentation'\nloading='explicit'\n[[source_exports]]\nname='intro'\nfile='guide.xml'\nheadings={headings}\n")).unwrap()
    }
    #[test]
    fn source_export_selects_exact_parent_path_from_same_captured_snapshot_without_facts() {
        let package = package("['Guide','Intro']");
        let observation = observe_declared_document(&package, "guide.xml", Some(RAW)).unwrap();
        let resolved = resolve_declared_export(&package, "intro", &observation).unwrap();
        assert_eq!(resolved.export_name, "intro");
        assert_eq!(resolved.resource_identity, "org.demo/demo:guide.xml");
        assert_eq!(resolved.witness, SourceFileWitness::of("guide.xml", RAW));
        assert_eq!(
            resolved.selection.raw().as_bytes(),
            &RAW[resolved.selection.span()]
        );
        assert!(resolved.selection.raw().contains("@fact:FAKE"));
        assert!(!resolved.selection.raw().contains("Other."));
        assert_eq!(resolved.selection.document().source().as_bytes(), RAW);
        let tree = observe_declared_source(
            &package,
            &[CapturedDeclaredDocument {
                path: "guide.xml",
                bytes: Some(RAW),
            }],
        )
        .unwrap();
        assert!(tree.complete);
        assert!(tree.facts.is_empty());
    }
    #[test]
    fn source_export_ambiguity_and_missing_keep_raw_readable() {
        for (headings, raw) in [
            (
                "['Guide','Intro']",
                b"# Guide\n## Intro\nA\n## Intro\nB\n".as_slice(),
            ),
            ("['Absent']", RAW),
        ] {
            let package = package(headings);
            let observation = observe_declared_document(&package, "guide.xml", Some(raw)).unwrap();
            assert!(matches!(
                resolve_declared_export(&package, "intro", &observation),
                Err(DeclaredExportError::Selector(_))
            ));
            let DeclaredDocumentObservation::OrdinaryMarkdown { bytes, .. } = observation else {
                panic!("ordinary observation")
            };
            assert_eq!(bytes, raw);
        }
    }
    #[test]
    fn source_export_requires_current_descriptor_and_declared_name() {
        let package = package("['Guide','Intro']");
        let observation = observe_declared_document(&package, "guide.xml", Some(RAW)).unwrap();
        assert!(matches!(
            resolve_declared_export(&package, "other", &observation),
            Err(DeclaredExportError::UndeclaredExport)
        ));
        let mut export_only = package.clone();
        export_only.sources = None;
        assert!(export_only.has_explicit_source_policy());
        assert!(matches!(
            resolve_declared_export(&export_only, "intro", &observation),
            Err(DeclaredExportError::UnselectedSource)
        ));
        let mut changed = package.clone();
        changed.sources = Some(vec![toml::from_str("file='guide.xml'\nmedia='text/markdown'\ninterpretation='commonmark'\nrole='resource'\nloading='scan'").unwrap()]);
        assert!(matches!(
            resolve_declared_export(&changed, "intro", &observation),
            Err(DeclaredExportError::WrongResource)
        ));
        let mut wrong = package.clone();
        wrong.sources = Some(vec![xml_data_descriptor()]);
        assert!(matches!(
            resolve_declared_export(&wrong, "intro", &observation),
            Err(DeclaredExportError::UnselectedSource)
        ));
    }
    fn xml_data_descriptor() -> vibe_core::manifest::SourceDescriptor {
        toml::from_str("file='guide.xml'\nmedia='application/xml'\ninterpretation='xml-data'\nrole='resource'\nloading='explicit'").unwrap()
    }
    #[test]
    fn source_export_rejects_wrong_resource_stale_bytes_and_descriptor_provenance() {
        let package = package("['Guide','Intro']");
        let original = observe_declared_document(&package, "guide.xml", Some(RAW)).unwrap();
        for kind in 0..4 {
            let mut observation = original.clone();
            if let DeclaredDocumentObservation::OrdinaryMarkdown {
                provenance, bytes, ..
            } = &mut observation
            {
                match kind {
                    0 => provenance.identity = "org.demo/demo:other".into(),
                    1 => provenance.witness = Some(SourceFileWitness::of("guide.xml", b"older")),
                    2 => *bytes = b"changed",
                    _ => provenance.disposition = DeclaredSourceDisposition::Unclassified,
                }
            }
            assert!(matches!(
                resolve_declared_export(&package, "intro", &observation),
                Err(DeclaredExportError::WrongResource | DeclaredExportError::ChangedWitness)
            ));
        }
    }
    #[test]
    fn ordinary_structure_failure_does_not_invalidate_native_facts() {
        let mut package = package("['Guide','Intro']");
        package.sources.as_mut().unwrap().push(toml::from_str("file='rule.md'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='scan'").unwrap());
        let raw = b"\xff\x00";
        let native = b"# Rule {#root}\n\n@fact:ONE Real. @status:impl/done\n";
        let tree = observe_declared_source(
            &package,
            &[
                CapturedDeclaredDocument {
                    path: "guide.xml",
                    bytes: Some(raw),
                },
                CapturedDeclaredDocument {
                    path: "rule.md",
                    bytes: Some(native),
                },
            ],
        )
        .unwrap();
        assert!(tree.complete);
        assert_eq!(tree.facts.len(), 1);
        let ordinary = &tree
            .documents
            .iter()
            .find(|(path, _)| path == "guide.xml")
            .unwrap()
            .1;
        assert!(
            matches!(ordinary,DeclaredDocumentObservation::OrdinaryMarkdown{bytes,structure:Err(crate::OrdinaryStructureError::InvalidUtf8),..} if *bytes==raw)
        );
        assert!(matches!(
            resolve_declared_export(&package, "intro", ordinary),
            Err(DeclaredExportError::StructureUnavailable)
        ));
    }
}
