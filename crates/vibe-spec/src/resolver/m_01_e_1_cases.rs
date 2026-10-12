//! Owned E1 filesystem fixtures; no customer or ambient state.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-070#SOURCE-OBSERVATION-FIXTURES");
use super::{FileResolver, ResolveError, SelfCoordinate, SpecAddress, portable};
use crate::{FsSectionSource, SectionSource};
use sha2::{Digest, Sha256};
use std::fs;
fn state(root: &std::path::Path) -> Vec<(std::path::PathBuf, String, Vec<u8>)> {
    fn walk(
        root: &std::path::Path,
        path: &std::path::Path,
        out: &mut Vec<(std::path::PathBuf, String, Vec<u8>)>,
    ) {
        let m = fs::symlink_metadata(path).unwrap();
        let mut identity = format!(
            "{:?}:{}:{}",
            m.file_type(),
            m.len(),
            m.permissions().readonly()
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            identity.push_str(&format!(
                ":{}:{}:{}:{}",
                m.dev(),
                m.ino(),
                m.nlink(),
                m.mode()
            ));
        }
        let bytes = if m.is_symlink() {
            fs::read_link(path)
                .unwrap()
                .as_os_str()
                .as_encoded_bytes()
                .to_vec()
        } else if m.is_file() {
            fs::read(path).unwrap()
        } else {
            Vec::new()
        };
        out.push((
            path.strip_prefix(root).unwrap().to_path_buf(),
            identity,
            bytes,
        ));
        if m.is_dir() {
            let mut children: Vec<_> = fs::read_dir(path)
                .unwrap()
                .map(|e| e.unwrap().path())
                .collect();
            children.sort();
            for child in children {
                walk(root, &child, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out
}

fn resolver(root: &std::path::Path) -> FileResolver {
    FileResolver::new(
        root,
        SelfCoordinate::new(Some("org.demo".into()), "self".into()),
    )
}
fn address(doc: &str) -> SpecAddress {
    let mut addr = SpecAddress::parse("spec://org.demo/self/common/DOC").unwrap();
    addr.doc_path = doc.to_owned();
    addr
}
#[test]
fn unique_nested_source_reaches_held_consumer() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("spec/common/nested/PROP-042-only.md");
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    let body = b"# Nested {#root}\n\nEXACT_NESTED_E1\n";
    fs::write(&file, body).unwrap();
    let before = state(root.path());
    let r = resolver(root.path());
    let mut addr = address("common/nested/PROP-042");
    addr.anchor = vec!["root".into()];
    assert_eq!(r.resolve_file(&addr).unwrap(), file);
    let observed = r.read_observed_source(&addr, &file).unwrap();
    assert_eq!(observed.bytes, body);
    assert_eq!(observed.sha256, format!("{:x}", Sha256::digest(body)));
    assert!(
        FsSectionSource::new(r)
            .section_text(&addr)
            .unwrap()
            .contains("EXACT_NESTED_E1")
    );
    assert_eq!(state(root.path()), before);
}
#[test]
fn portable_components_refuse_before_source_observation() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("spec")).unwrap();
    fs::write(root.path().join("outside.md"), "OUTSIDE").unwrap();
    let before = state(root.path());
    let reads = std::rc::Rc::new(std::cell::Cell::new(0));
    let count = reads.clone();
    vibe_safefs::arm_before_bounded_read(Some(Box::new(move |_, _| count.set(count.get() + 1))));
    let r = resolver(root.path());
    for doc in [
        "../outside",
        "C:/outside",
        "//server/share",
        "\\\\server\\share",
        "common/DOC:ads",
        "common/CON",
        "common/LPT9.txt",
        "common/trailing.",
        "common/bell\u{7}",
    ] {
        assert!(
            matches!(
                r.resolve_file(&address(doc)),
                Err(ResolveError::UnsafeSourcePath { .. })
            ),
            "{doc:?}"
        );
    }
    vibe_safefs::arm_before_bounded_read(None);
    assert_eq!(reads.get(), 0);
    assert_eq!(state(root.path()), before);
}
#[test]
fn unicode_and_case_aliases_preserve_exact_fixture_spelling() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join("spec/Common")).unwrap();
    fs::write(root.path().join("spec/Common/Guide.md"), "# Guide\n").unwrap();
    fs::write(root.path().join("spec/Common/éditions.md"), "# Unicode\n").unwrap();
    let before = state(root.path());
    let r = resolver(root.path());
    for doc in ["common/Guide", "Common/guide", "Common/e\u{301}ditions"] {
        match r.resolve_file(&address(doc)) {
            Err(ResolveError::PortableSourceAlias {
                requested,
                observed,
            }) => assert_ne!(requested, observed),
            other => panic!("alias {doc:?}: {other:?}"),
        }
    }
    assert_eq!(state(root.path()), before);
}
#[test]
fn duplicate_anchor_refuses_both_candidates() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join("spec/common")).unwrap();
    fs::write(
        root.path().join("spec/common/DOC.md"),
        "# First {#same}\n\nFIRST_E1\n\n# Second {#same}\n\nSECOND_E1\n",
    )
    .unwrap();
    let before = state(root.path());
    let mut addr = address("common/DOC");
    addr.anchor = vec!["same".into()];
    let error = FsSectionSource::new(resolver(root.path()))
        .section_text(&addr)
        .unwrap_err();
    assert_eq!(
        error,
        format!(
            "ambiguous anchor in {}",
            root.path().join("spec/common/DOC.md").display()
        )
    );
    assert_eq!(state(root.path()), before);
}
#[test]
fn failed_enumeration_is_unreadable_not_absent() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("vibedeps")).unwrap();
    let file = root.path().join("vibedeps/org.demo.widget");
    fs::write(&file, "NOT_A_DIRECTORY").unwrap();
    let before = state(root.path());
    match portable::entries(&file) {
        Err(ResolveError::SourceUnreadable { path, reason }) => {
            assert_eq!(path, file.display().to_string());
            assert!(!reason.is_empty());
        }
        other => panic!("read_dir(file) must be unreadable: {other:?}"),
    }
    let addr = SpecAddress::parse("spec://org.demo/widget/common/DOC").unwrap();
    assert!(matches!(
        resolver(root.path()).resolve_file(&addr),
        Err(ResolveError::SourceUnreadable { .. })
    ));
    assert_eq!(state(root.path()), before);
}
#[cfg(unix)]
#[test]
fn same_byte_replacement_refuses_without_query_writes() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join("spec/common")).unwrap();
    let file = root.path().join("spec/common/DOC.md");
    fs::write(&file, "# Doc {#root}\n\nSAME_E1\n").unwrap();
    let r = resolver(root.path());
    let addr = address("common/DOC");
    let path = root.path().to_path_buf();
    let injected = std::rc::Rc::new(std::cell::RefCell::new(None));
    let capture = injected.clone();
    vibe_safefs::arm_before_bounded_read(Some(Box::new(move |_, _| {
        let from = path.join("spec/common/DOC.md");
        let bytes = fs::read(&from).unwrap();
        fs::rename(&from, path.join("spec/common/old.md")).unwrap();
        fs::write(&from, bytes).unwrap();
        *capture.borrow_mut() = Some(state(&path));
    })));
    let result = r.read_observed_source(&addr, &file);
    vibe_safefs::arm_before_bounded_read(None);
    match result {
        Err(ResolveError::SourceObservationUnavailable { reason, .. }) => {
            assert!(reason.contains("final-name race"), "{reason}")
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(state(root.path()), *injected.borrow().as_ref().unwrap());
}
