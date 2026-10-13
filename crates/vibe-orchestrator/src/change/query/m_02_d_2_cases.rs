use super::*;
use std::fs;
const ID: &str = "chg-0123456789abcdef0123456789abcdef";
#[test]
fn actual_saved_capture_returns_useful_unclassified_local_intent() {
    let root = fixture();
    let request = serde_json::json!({"schema":1,"mode":"show","id":ID});
    let response = crate::machine_operation::MachineOperationService::read_change_json(
        root.path(),
        None,
        &request,
    );
    let Response::Success(row) = response else {
        panic!("actual response: {response:?}")
    };
    assert_eq!(row.title, "Distinct local intent");
    assert!(!row.scope_complete);
    assert!(
        row.gaps
            .iter()
            .any(|g| g.code == "unclassified-local-facts")
    );
    assert!(row.facts.is_empty());
    assert_eq!(row.roles.len(), 2);
    assert!(row.roles.iter().all(|r| r.raw_hex.is_some()));
    assert!(!root.path().join(".vibe").exists());
    println!("EARLY_CHANGE {}", serde_json::to_string(&row).unwrap());
}

fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("vibe.toml"),
        "[project]\ngroup='org.demo'\nname='consumer'\nversion='1.0.0'\n",
    )
    .unwrap();
    let dir = root.path().join("vibevm/vibespecs/changes").join(ID);
    fs::create_dir_all(&dir).unwrap();
    let text = format!(
        "schema=1\n[change]\nid='{ID}'\nselected_node='.'\nslug='local'\ntitle='Distinct local intent'\n[[origins]]\npackage='flow:org.demo/dep@=1.0.0'\nselected_content_hash='sha256:{}'\nmanifest_sha256='sha256:{}'\n[[origins.documents]]\naddress='spec://org.demo/dep/DOC#ONE'\nsha256='sha256:{}'\n[[documents]]\nrole='intent'\npath='intent.md'\nnormative=true\ninput=true\n[[documents]]\nrole='acceptance'\npath='acceptance.md'\nnormative=true\ninput=true\n",
        "a".repeat(64),
        "b".repeat(64),
        "c".repeat(64)
    );
    fs::write(dir.join("change.toml"), text).unwrap();
    fs::write(
        dir.join("intent.md"),
        "@fact:LOCAL Local. @status:impl/done\n",
    )
    .unwrap();
    fs::write(dir.join("acceptance.md"), "Local acceptance\n").unwrap();
    root
}

#[test]
fn captured_disk_and_adoption_drift_refuse_with_exact_old_basis_retained() {
    for changed_registry in [false, true] {
        let root = fixture();
        let service = CapturedChangeQuery::prepare(root.path(), None, ID).unwrap();
        let request =
            wire::read_request(&serde_json::json!({"schema":1,"mode":"show","id":ID})).unwrap();
        let before = serde_json::to_value(service.query(&request)).unwrap();
        assert_eq!(before["status"], "success");
        if changed_registry {
            let mut registry = vibe_facts::Registry::load(root.path()).unwrap();
            registry
                .upsert(
                    root.path(),
                    vibe_facts::FactEntry {
                        address: "spec://org.demo/dep/DOC#ONE".into(),
                        origin: vibe_facts::FactOrigin::Package,
                        package: Some("org.demo/dep".into()),
                        status: None,
                        comment: None,
                    },
                )
                .unwrap();
        } else {
            fs::write(
                root.path()
                    .join("vibevm/vibespecs/changes")
                    .join(ID)
                    .join("intent.md"),
                "Later raw bytes\n",
            )
            .unwrap();
        }
        let after = serde_json::to_value(service.query(&request)).unwrap();
        assert_eq!(after["status"], "refused");
        assert_eq!(after["code"], "stale-observation");
        assert!(
            before["roles"]
                .as_array()
                .unwrap()
                .iter()
                .all(|r| r["raw_hex"].is_string())
        );
        println!(
            "STALE_CAPTURE {}",
            serde_json::json!({"registry":changed_registry,"before":before,"after":after})
        );
    }
}

#[test]
fn declared_role_filter_is_exact_and_unknown_role_has_candidates() {
    let root = fixture();
    let service = CapturedChangeQuery::prepare(root.path(), None, ID).unwrap();
    let request = wire::read_request(
        &serde_json::json!({"schema":1,"mode":"show","id":ID,"roles":["intent"]}),
    )
    .unwrap();
    let value = serde_json::to_value(service.query(&request)).unwrap();
    assert_eq!(value["status"], "success");
    assert_eq!(value["roles"].as_array().unwrap().len(), 1);
    assert_eq!(value["roles"][0]["role"], "intent");
    assert_eq!(value["omitted_roles"], serde_json::json!(["acceptance.md"]));
    let request =
        wire::read_request(&serde_json::json!({"schema":1,"mode":"show","id":ID,"roles":["work"]}))
            .unwrap();
    let value = serde_json::to_value(service.query(&request)).unwrap();
    assert_eq!(value["code"], "unknown-role");
    assert_eq!(
        value["candidates"],
        serde_json::json!(["intent", "acceptance"])
    );
}
