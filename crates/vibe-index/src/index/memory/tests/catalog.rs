//! Documentation packages: the card and the two edges.
//!
//! PROP-057 `##REL-INDEX-FIELDS` puts `title`, `abstract`, `documents`,
//! `documentation`, `translates` and `media` into the catalog record, and
//! `##REL-REVERSE-QUERIES-SITE-SIDE` says what that buys: the reverse
//! questions are answered by ONE fold over `primary.jsonl`, in memory, at
//! build time — the index gains fields and no routes.
//!
//! File-backed submodule of [`super`] so every cell stays inside the
//! AI-Native file budget. Fixtures and the fixed clock are [`super`]'s;
//! nothing moved but the file.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#identity");

use super::*;

/// A documentation entry: the manual's own card plus the edge to the
/// subject it documents. `at` is the standing fixed instant, like every
/// other fixture here.
#[cfg(test)]
fn doc_entry(name: &str, version: &str, subject: &str) -> VersionEntry {
    let mut e = entry(PackageKind::Doc, org(), name, version);
    e.title = Some(format!("The {name} manual"));
    e.abstract_ = Some("What it covers, for whom, what it assumes, what it leaves out.".into());
    e.documents = vec![crate::types::DocumentsEntry {
        package: subject.to_string(),
        version: "^1.0".to_string(),
    }];
    e.media = Some(crate::types::MediaEntry {
        icon: Some("media/icon.png".into()),
        banner: None,
        preview: None,
    });
    e
}

/// The card and both edges survive `write_to` → `load_from` unchanged.
/// The round trip is the whole point of indexing them: a shelf card, a
/// language selector and an officiality badge must render from the
/// catalog alone, with no package downloaded.
#[test]
fn a_doc_packages_card_and_relations_survive_the_catalog_round_trip() {
    let tmp = tempdir().unwrap();
    let mut idx = fresh_index();

    let mut source = doc_entry("wal-docs", "0.1.0", "org.vibevm/wal");
    source.i18n = Some(crate::types::I18nEntry {
        available: vec!["en".into()],
        default: Some("en".into()),
    });

    let mut adaptation = doc_entry("wal-docs-ru", "0.1.0", "org.vibevm/wal");
    adaptation.translates = Some(crate::types::TranslatesEntry {
        package: "org.vibevm/wal-docs".to_string(),
        version: "^0.1".to_string(),
    });
    // The language of a documentation is `[i18n].canonical` and nothing
    // else (PROP-057 `##LOC-LANGUAGE-FIELD`) — there is no `lang` field
    // to carry, here or in the manifest.
    adaptation.i18n = Some(crate::types::I18nEntry {
        available: vec!["ru".into()],
        default: Some("ru".into()),
    });

    // The subject writes the other end of the edge, and it is a `flow`.
    let mut subject = entry(PackageKind::Flow, org(), "wal", "1.0.0");
    subject.documentation = Some(crate::types::DocumentationEntry {
        primary: Some("org.vibevm/wal-docs".to_string()),
        official: vec!["org.vibevm/wal-tutorials".to_string()],
    });

    idx.upsert(source);
    idx.upsert(adaptation);
    idx.upsert(subject);
    idx.write_to(tmp.path(), &write_ctx()).unwrap();

    let back = Index::load_from(tmp.path()).unwrap();
    let docs = back
        .iter_versions()
        .find(|v| v.name == "wal-docs")
        .expect("the manual is in the catalog");
    assert_eq!(docs.kind, PackageKind::Doc);
    assert_eq!(docs.title.as_deref(), Some("The wal-docs manual"));
    assert!(docs.abstract_.is_some(), "the abstract rides the wire");
    assert_eq!(docs.documents.len(), 1);
    assert_eq!(docs.documents[0].package, "org.vibevm/wal");
    assert_eq!(docs.documents[0].version, "^1.0");
    assert_eq!(
        docs.media.as_ref().and_then(|m| m.icon.as_deref()),
        Some("media/icon.png")
    );

    let ru = back
        .iter_versions()
        .find(|v| v.name == "wal-docs-ru")
        .expect("the adaptation is in the catalog");
    let translates = ru.translates.as_ref().expect("the source edge survives");
    assert_eq!(translates.package, "org.vibevm/wal-docs");
    assert_eq!(
        ru.i18n.as_ref().and_then(|i| i.default.as_deref()),
        Some("ru")
    );

    let wal = back
        .iter_versions()
        .find(|v| v.name == "wal")
        .expect("the subject is in the catalog");
    let documentation = wal.documentation.as_ref().expect("the subject's pointer");
    assert_eq!(
        documentation.primary.as_deref(),
        Some("org.vibevm/wal-docs")
    );
    assert_eq!(documentation.official, vec!["org.vibevm/wal-tutorials"]);
}

/// The reverse questions are a fold, not a route: «who documents X» and
/// «which translations does Y have» are answered from the loaded
/// catalog in memory, and the written tree gains no `by-documents/` or
/// `by-translates/` folder to maintain
/// (PROP-057 `##REL-REVERSE-QUERIES-SITE-SIDE`).
#[test]
fn reverse_documentation_questions_are_a_fold_and_add_no_files() {
    let tmp = tempdir().unwrap();
    let mut idx = fresh_index();
    let mut ru = doc_entry("wal-docs-ru", "0.1.0", "org.vibevm/wal");
    ru.translates = Some(crate::types::TranslatesEntry {
        package: "org.vibevm/wal-docs".to_string(),
        version: "^0.1".to_string(),
    });
    idx.upsert(doc_entry("wal-docs", "0.1.0", "org.vibevm/wal"));
    idx.upsert(ru);
    idx.write_to(tmp.path(), &write_ctx()).unwrap();

    let written: std::collections::BTreeSet<String> = std::fs::read_dir(tmp.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    for absent in ["by-documents", "by-translates"] {
        assert!(
            !written.contains(absent),
            "the index answers this by folding `primary.jsonl`, not by \
             writing `{absent}/`: {written:?}"
        );
    }

    let back = Index::load_from(tmp.path()).unwrap();

    // «Who documents org.vibevm/wal» — one pass over the versions.
    let documenting: Vec<&str> = back
        .iter_versions()
        .filter(|v| v.documents.iter().any(|d| d.package == "org.vibevm/wal"))
        .map(|v| v.name.as_str())
        .collect();
    assert_eq!(documenting, vec!["wal-docs", "wal-docs-ru"]);

    // «Which translations does org.vibevm/wal-docs have» — the same pass
    // over the other edge; the source stores no list of its own
    // (PROP-057 `##LOC-NO-TRANSLATIONS-TABLE`).
    let adaptations: Vec<&str> = back
        .iter_versions()
        .filter(|v| {
            v.translates
                .as_ref()
                .is_some_and(|t| t.package == "org.vibevm/wal-docs")
        })
        .map(|v| v.name.as_str())
        .collect();
    assert_eq!(adaptations, vec!["wal-docs-ru"]);
}
