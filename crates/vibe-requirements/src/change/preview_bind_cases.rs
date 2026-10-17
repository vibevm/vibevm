use super::*;
use sha2::{Digest, Sha256};
use std::{fs, path::Path};
use vibe_core::manifest::{
    Manifest,
    change::{ChangeContext, ChangeId},
};
#[test]
fn all_retained_records_saved_and_captured_preserve_exact_bytes() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let census: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../formats/corpora/change-manifest/e1/retained-preview.json"
    )))
    .unwrap();
    let node = Manifest::parse_str(&fs::read_to_string(root.join("vibe.toml")).unwrap()).unwrap();
    let context = ChangeContext::for_node(&node, ".", None).unwrap();
    let rows = census["records"].as_array().unwrap();
    assert_eq!(rows.len(), 71);
    for row in rows {
        let id = row["change_id"].as_str().unwrap();
        let bytes = fs::read(root.join(row["path"].as_str().unwrap())).unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(&bytes)),
            row["sha256"].as_str().unwrap()
        );
        assert_eq!(bytes.len() as u64, row["byte_len"].as_u64().unwrap());
        let saved = read_saved_change(&root, ".", None, id).unwrap();
        assert_eq!(saved.input_observations.len(), 1);
        assert_eq!(saved.input_observations[0].1.bytes, bytes);
        assert!(saved.identity_analysis().is_err());
        let records = saved.release_impacts().unwrap();
        assert_eq!(records[0].change_id, id);
        let text = std::str::from_utf8(&saved.manifest_observation.bytes).unwrap();
        let inputs = [(saved.input_observations[0].0.as_path(), bytes.as_slice())];
        let captured =
            analyze_change_bytes(text, &context, saved.manifest.directory(), &inputs).unwrap();
        assert!(captured.identities.is_none());
        assert_eq!(captured.release_impacts, records);
        let mut unknown: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        unknown["schema"] = 99.into();
        let invalid = serde_json::to_vec(&unknown).unwrap();
        assert!(
            analyze_change_bytes(
                text,
                &context,
                saved.manifest.directory(),
                &[(inputs[0].0, &invalid)]
            )
            .is_err()
        );
        assert_eq!(
            fs::read(root.join(row["path"].as_str().unwrap())).unwrap(),
            bytes
        );
    }
}
#[test]
fn evidence_update_after_contribution_preserves_subject_and_retained_observation() {
    let root = tempfile::tempdir().unwrap();
    let node = "[project]\ngroup='org.fixture'\nname='alpha'\nversion='1.0.0'\n";
    fs::write(root.path().join("vibe.toml"), node).unwrap();
    let context = ChangeContext::for_node(
        &Manifest::parse_str(node).unwrap(),
        ".",
        Some(Path::new("custom/specs")),
    )
    .unwrap();
    let id = "chg-0123456789abcdef0123456789abcdef";
    let dir = context.directory(&ChangeId::parse(id).unwrap());
    fs::create_dir_all(root.path().join(&dir)).unwrap();
    let text = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../formats/corpora/change-manifest/e1/valid/representative.toml"
    ));
    let path = root.path().join(&dir);
    let text = format!(
        "{text}\n[[documents]]\nrole='work'\npath='work.toml'\nnormative=false\ninput=true\n"
    );
    fs::write(path.join("change.toml"), &text).unwrap();
    fs::write(
        path.join("work.toml"),
        "schema=1\n[[items]]\nid='work-a'\npredecessors=[]\nneeds=[]\ntouches=[]\n",
    )
    .unwrap();
    fs::write(path.join("intent.md"), "intent").unwrap();
    fs::write(path.join("acceptance.md"), "acceptance").unwrap();
    let read = || read_saved_change(root.path(), ".", Some(Path::new("custom/specs")), id).unwrap();
    let original_subject = read();
    let before = original_subject.identity_analysis().unwrap();
    let contribution = before
        .graph
        .contribution_identity(&original_subject.manifest, &["work-a"])
        .unwrap();
    let text = format!(
        "{text}\n[[documents]]\nrole='release-impact'\npath='release-impact.json'\nnormative=false\ninput=true\n"
    );
    fs::write(path.join("change.toml"), &text).unwrap();
    let mut record: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../formats/corpora/release-impact/e1/valid/internal-refactor.json"
    )))
    .unwrap();
    record["change_id"] = id.into();
    record["acceptance"]["state"] = "accepted".into();
    let original = serde_json::to_vec(&record).unwrap();
    fs::write(path.join("release-impact.json"), &original).unwrap();
    let retained = read();
    record["user_impact"]["reason"] = "Updated accounting explanation".into();
    let changed = serde_json::to_vec(&record).unwrap();
    fs::write(path.join("release-impact.json"), &changed).unwrap();
    let updated_subject = read();
    let after = updated_subject.identity_analysis().unwrap();
    assert_eq!(
        contribution,
        after
            .graph
            .contribution_identity(&updated_subject.manifest, &["work-a"])
            .unwrap()
    );
    assert_eq!(before.intent, after.intent);
    assert_eq!(before.work, after.work);
    assert_eq!(retained.identity_analysis().unwrap().intent, before.intent);
    assert_eq!(
        retained.release_impacts().unwrap()[0].user_impact.reason,
        serde_json::from_slice::<serde_json::Value>(&original).unwrap()["user_impact"]["reason"]
            .as_str()
            .unwrap()
    );
    let self_path = vibe_core::machine_json_path(&dir.join("release-impact.json"));
    record["evidence"][0]["source"] = self_path.clone().into();
    record["acceptance"]["review_evidence"] = self_path.into();
    fs::write(
        path.join("release-impact.json"),
        serde_json::to_vec(&record).unwrap(),
    )
    .unwrap();
    assert!(read_saved_change(root.path(), ".", Some(Path::new("custom/specs")), id).is_err());
}
