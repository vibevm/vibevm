use super::*;

fn identity() -> GitIdentity {
    GitIdentity {
        commit: COMMIT.to_string(),
        tree: TREE.to_string(),
        source_date_epoch: "1".to_string(),
    }
}

fn prepare(host: &MockHost) -> Result<GithubRelease> {
    prepare_with(host, &Version::parse("1.0.0").unwrap(), &identity())
}

#[test]
fn prepare_validates_the_prepositioned_tag_before_replacing_the_release() {
    let host = MockHost::draft();
    host.release.borrow_mut().as_mut().unwrap().draft = false;
    host.seed("old.zip".to_string(), b"old".to_vec());
    let result = prepare(&host).unwrap();
    assert!(result.draft);
    assert_eq!(result.id, 8);
    assert_eq!(host.moved_to.borrow().as_deref(), None);
    assert!(host.assets.borrow().is_empty());
    assert_eq!(
        host.operations.borrow().as_slice(),
        ["tag-find", "find", "delete:7", "create"]
    );
}

#[test]
fn prepare_without_a_tag_creates_it_before_the_draft() {
    let host = MockHost::draft();
    *host.release.borrow_mut() = None;
    host.tag_present.set(false);

    let result = prepare(&host).unwrap();

    assert!(result.draft);
    assert_eq!(result.id, 8);
    assert_eq!(host.moved_to.borrow().as_deref(), Some(COMMIT));
    assert_eq!(
        host.operations.borrow().as_slice(),
        ["tag-find", "tag-create", "find", "create"]
    );
}

#[test]
fn prepare_refuses_a_tag_update_response_with_wrong_provenance() {
    let host = MockHost::draft();
    *host.release.borrow_mut() = None;
    *host.tag_kind.borrow_mut() = "tag".to_string();
    let error = prepare(&host)
        .expect_err("an indirect or wrong tag object must be refused")
        .to_string();
    assert!(error.contains("tag provenance mismatch"));
    assert!(host.release.borrow().is_none());
    assert_eq!(host.operations.borrow().as_slice(), ["tag-find"]);
}

#[test]
fn prepare_moves_a_leftover_tag_when_its_release_is_a_draft() {
    // A failed run left `v1.0.0` and its draft at an older commit.
    let host = MockHost::draft();
    *host.moved_to.borrow_mut() = Some(TREE.to_string());
    host.seed("old.zip".to_string(), b"old".to_vec());

    let result = prepare(&host).unwrap();

    assert!(result.draft);
    assert_eq!(result.id, 8);
    assert_eq!(result.target_commitish, COMMIT);
    assert_eq!(host.moved_to.borrow().as_deref(), Some(COMMIT));
    assert!(host.assets.borrow().is_empty());
    assert_eq!(
        host.operations.borrow().as_slice(),
        ["tag-find", "find", "tag-move", "delete:7", "create"]
    );
}

#[test]
fn prepare_refuses_a_foreign_tag_of_a_published_release() {
    let host = MockHost::draft();
    host.release.borrow_mut().as_mut().unwrap().draft = false;
    *host.moved_to.borrow_mut() = Some(TREE.to_string());

    let error = prepare(&host)
        .expect_err("a published release's tag is never moved")
        .to_string();

    assert!(error.contains("tag provenance mismatch"));
    assert_eq!(host.moved_to.borrow().as_deref(), Some(TREE));
    assert!(
        host.release.borrow().as_ref().is_some_and(|r| !r.draft),
        "published release must remain untouched"
    );
    assert!(!host.operations.borrow().iter().any(|op| op == "tag-move"));
    assert_eq!(host.operations.borrow().as_slice(), ["tag-find", "find"]);
}

#[test]
fn prepare_refuses_a_foreign_tag_without_a_release() {
    let host = MockHost::draft();
    *host.release.borrow_mut() = None;
    *host.moved_to.borrow_mut() = Some(TREE.to_string());

    let error = prepare(&host)
        .expect_err("an unknown tag is never moved")
        .to_string();

    assert!(error.contains("tag provenance mismatch"));
    assert_eq!(host.moved_to.borrow().as_deref(), Some(TREE));
    assert!(host.release.borrow().is_none());
    assert!(!host.operations.borrow().iter().any(|op| op == "tag-move"));
    assert_eq!(host.operations.borrow().as_slice(), ["tag-find", "find"]);
}
