//! C1 consumer: projection receives the same held bytes as the raw digest.

use std::fs;

use sha2::{Digest, Sha256};

use crate::{FsSectionSource, SectionSource, SelfCoordinate, SpecAddress};

use super::{FileResolver, ResolveError, project_source, read_stable_with};

fn fixture() -> (tempfile::TempDir, FileResolver, SpecAddress) {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join("spec/common")).unwrap();
    fs::write(
        root.path().join("spec/common/DOC.md"),
        "# Doc {#root}\n\nOBSERVED_C1\n",
    )
    .unwrap();
    let resolver = FileResolver::new(
        root.path(),
        SelfCoordinate::new(Some("org.demo".into()), "self".into()),
    );
    let address = SpecAddress::parse("spec://org.demo/self/common/DOC#root").unwrap();
    (root, resolver, address)
}

#[test]
fn projected_embed_and_raw_digest_use_one_held_source_buffer() {
    let (_root, resolver, address) = fixture();
    let path = resolver.resolve_file(&address).unwrap();
    let (observed, projected) = resolver
        .read_observed_projected_source(&address, &path)
        .unwrap();
    assert_eq!(
        observed.sha256,
        format!("{:x}", Sha256::digest(&observed.bytes))
    );
    assert_eq!(observed.size, observed.bytes.len() as u64);
    assert_eq!(projected, project_source(&path, &observed.bytes).unwrap());
    assert!(projected.contains("OBSERVED_C1"));
    let embedded = FsSectionSource::new(resolver)
        .section_text(&address)
        .unwrap();
    assert!(embedded.contains("OBSERVED_C1"));
}

#[test]
fn shared_source_is_unavailable_to_the_consumer() {
    let (root, resolver, address) = fixture();
    let path = resolver.resolve_file(&address).unwrap();
    fs::hard_link(&path, root.path().join("spec/common/alias.md")).unwrap();
    assert!(matches!(
        resolver.read_observed_source(&address, &path),
        Err(ResolveError::SourceObservationUnavailable { .. })
    ));
}

#[test]
fn historical_two_name_reads_accept_same_byte_replacement() {
    let (root, resolver, address) = fixture();
    let path = resolver.resolve_file(&address).unwrap();
    let moved = root.path().join("spec/common/moved.md");
    let mut reads = 0;
    let result = read_stable_with(&path, |source| {
        reads += 1;
        let bytes = fs::read(source)?;
        if reads == 1 {
            fs::rename(source, &moved)?;
            fs::write(source, &bytes)?;
        }
        Ok(bytes)
    })
    .unwrap();
    assert_eq!(reads, 2);
    assert_eq!(result, fs::read(&moved).unwrap());
    assert_eq!(result, fs::read(&path).unwrap());
    assert!(moved.exists(), "the old object survived under another name");
}

#[cfg(unix)]
#[test]
fn same_byte_replacement_during_actual_embed_read_refuses() {
    let (root, resolver, address) = fixture();
    let path = resolver.resolve_file(&address).unwrap();
    let moved = root.path().join("spec/common/moved.md");
    let (from, to) = (path.clone(), moved.clone());
    vibe_safefs::arm_before_bounded_read(Some(Box::new(move |_, _| {
        let bytes = fs::read(&from).unwrap();
        fs::rename(&from, &to).unwrap();
        fs::write(&from, bytes).unwrap();
    })));
    let outcome = FsSectionSource::new(resolver).section_text(&address);
    vibe_safefs::arm_before_bounded_read(None);
    let error = outcome.unwrap_err();
    assert!(error.contains("final-name race"), "{error}");
    assert_eq!(fs::read(&moved).unwrap(), fs::read(&path).unwrap());
    let project = vibe_safefs::Project::open(root.path()).unwrap();
    let old = project
        .read_file_snapshot_bounded("spec/common/moved.md", 1024)
        .unwrap()
        .unwrap();
    let new = project
        .read_file_snapshot_bounded("spec/common/DOC.md", 1024)
        .unwrap()
        .unwrap();
    assert_ne!(
        old.identity, new.identity,
        "the equal bytes belong to different objects"
    );
}
