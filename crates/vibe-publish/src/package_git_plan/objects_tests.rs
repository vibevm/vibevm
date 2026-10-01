use super::*;

#[test]
fn canonical_object_bytes_bind_attribution_time_and_exact_messages() {
    let plan = absent();
    let commit = String::from_utf8(plan.canonical_commit_bytes().unwrap().unwrap()).unwrap();
    assert!(commit.starts_with(&format!("tree {}\nauthor Oleg Chirukhin <oleg@example.test> 1700000000 +0300\ncommitter Oleg Chirukhin <oleg@example.test> 1700000000 +0300\n\n", plan.staged.git_tree_oid)));
    assert!(commit.ends_with("Release wal@1.1.0\n"));
    let tag = String::from_utf8(plan.canonical_tag_bytes().unwrap().unwrap()).unwrap();
    assert!(tag.starts_with(&format!("object {}\ntype commit\ntag v1.1.0\ntagger Oleg Chirukhin <oleg@example.test> 1700000000 +0300\n\n", plan.staged.commit.oid())));
    assert!(tag.ends_with("wal@1.1.0\n"));

    let mut changed = plan.clone();
    changed
        .staged
        .commit_metadata
        .as_mut()
        .unwrap()
        .author
        .timestamp_seconds += 1;
    assert!(
        changed.validate().is_err(),
        "stale OID cannot hide changed time"
    );
    changed.derive_staged_object_ids().unwrap();
    assert_ne!(changed.digest().unwrap(), plan.digest().unwrap());
    assert_ne!(changed.staged.commit.oid(), plan.staged.commit.oid());

    let mut changed = plan.clone();
    changed.staged.tag_metadata.as_mut().unwrap().message = "different tag\n".into();
    assert!(
        changed.validate().is_err(),
        "stale tag OID cannot hide changed message"
    );
    changed.derive_staged_object_ids().unwrap();
    assert_ne!(
        changed.staged.annotated_tag_oid,
        plan.staged.annotated_tag_oid
    );
}

#[test]
fn object_headers_and_messages_refuse_control_injection() {
    let mut invalids = Vec::new();
    let mut bad = absent();
    bad.staged.commit_metadata.as_mut().unwrap().author.email = "a@b\0".into();
    invalids.push(bad);
    let mut bad = absent();
    bad.staged.commit_metadata.as_mut().unwrap().committer.name = "a\nparent deadbeef".into();
    invalids.push(bad);
    let mut bad = absent();
    bad.staged.tag_metadata.as_mut().unwrap().tagger.email = "a@b\x1f".into();
    invalids.push(bad);
    let mut bad = absent();
    bad.staged.commit_metadata.as_mut().unwrap().message = "bad\rmessage\n".into();
    invalids.push(bad);
    let mut bad = absent();
    bad.staged.tag_metadata.as_mut().unwrap().message = "x".repeat(4097);
    invalids.push(bad);
    for mut bad in invalids {
        assert!(bad.validate().is_err());
        assert!(bad.derive_staged_object_ids().is_err());
    }
}

#[test]
fn old_or_wrong_plan_epoch_refuses() {
    let plan = absent();
    let bytes = serde_json::to_vec(&plan).unwrap();
    let mut old: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    old.as_object_mut().unwrap().remove("epoch");
    assert!(serde_json::from_value::<PackageGitPlan>(old).is_err());
    let mut wrong = plan;
    wrong.epoch = 1;
    assert!(wrong.validate().is_err());
}

#[test]
fn git_object_framing_has_fixed_sha1_and_sha256_vectors() {
    let sha1 = absent();
    assert_eq!(
        sha1.staged.commit.oid(),
        "f57bb9fe216dfdd93818899c1061a99fecde557f"
    );
    assert_eq!(
        sha1.staged.annotated_tag_oid.as_deref(),
        Some("ffbe462764c3c72efb1c4f4bb5facd5bc6381eaa")
    );
    let mut sha256 = sha1;
    sha256.oid_hex_len = 64;
    sha256.staged.git_tree_oid = "c".repeat(64);
    sha256.derive_staged_object_ids().unwrap();
    assert_eq!(
        sha256.staged.commit.oid(),
        "55f80d7b86a1b4d405f6644d9fdd0004cc2ed73f13ea600d163a12fb703ef301"
    );
    assert_eq!(
        sha256.staged.annotated_tag_oid.as_deref(),
        Some("04eb81130a45cc29a5aa6395749789a980922a073a85b2eb95f01f9edb7fb299")
    );
}
