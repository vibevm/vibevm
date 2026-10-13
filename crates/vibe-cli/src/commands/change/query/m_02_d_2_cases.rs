use super::*;
use vibe_mcp::tools::{ChangeQueryMcpTool, McpTool};
#[test]
fn actual_cli_mcp_service_refusal_payloads_match_before_preparation() {
    let root = tempfile::tempdir().unwrap();
    let context =
        vibe_mcp::ServerContext::with_store_root(root.path(), root.path().join("empty-cache"));
    let request = json!({"schema":99,"mode":"show","id":"chg-0123456789abcdef0123456789abcdef"});
    let response = execute(root.path(), None, &request);
    let value = serde_json::to_value(&response).unwrap();
    let error = ChangeQueryMcpTool.run(&request, &context).unwrap_err();
    let vibe_mcp::ToolError::StructuredPreflight { report, .. } = error else {
        panic!("wrong channel")
    };
    assert_eq!(report, value);
    assert_eq!(value["code"], "unknown-epoch");
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

fn semantic(mut value: Value) -> Value {
    // Only M16 observation IDs differ; origin content captures stay exact.
    value.as_object_mut().unwrap().remove("capture");
    if let Some(facts) = value.get_mut("facts").and_then(Value::as_array_mut) {
        for fact in facts {
            if fact["scope"] == "local" {
                fact.as_object_mut().unwrap().remove("capture");
            }
        }
    }
    value
}
#[test]
fn d1_default_changes_share_origin_with_distinct_intent_across_real_adapters() {
    use super::super::NewArgs;
    use std::fs;
    let root = super::super::m_02_d_1_cases::fixture();
    let host = root.path().join("host");
    let ids = [
        "chg-0123456789abcdef0123456789abcdef",
        "chg-fedcba9876543210fedcba9876543210",
    ];
    for (n, id) in ids.iter().enumerate() {
        let args = NewArgs {
            from: "flow:org.demo/dep@=1.0.0".into(),
            id: (*id).into(),
            slug: format!("local-{n}"),
            title: format!("Local {n}"),
            path: host.clone(),
            spec_root: None,
            fact: vec![],
            apply: true,
        };
        let report = super::super::execute(&args).unwrap();
        assert!(matches!(report, super::super::report::Report::Completed(_)));
    }
    fs::write(
        host.join("vibevm/vibespecs/changes")
            .join(ids[1])
            .join("refinement.md"),
        "Independent beta refinement bytes\n",
    )
    .unwrap();
    let context = vibe_mcp::ServerContext::with_store_root(&host, root.path().join("empty-cache"));
    let mut values = vec![];
    for id in ids {
        let request = json!({"schema":1,"mode":"show","id":id});
        let service = execute(&host, None, &request);
        let service = wire::read_response(&serde_json::to_value(&service).unwrap()).unwrap();
        let output = ChangeQueryMcpTool.run(&request, &context).unwrap();
        let mcp = wire::read_response(output.structured()).unwrap();
        let service = serde_json::to_value(service).unwrap();
        assert_eq!(
            semantic(service.clone()),
            semantic(serde_json::to_value(mcp).unwrap())
        );
        assert_eq!(service["status"], "success");
        assert_eq!(service["scope_complete"], false);
        assert_eq!(service["terminal"], "not-evaluated");
        assert_eq!(service["facts"].as_array().unwrap().len(), 1);
        assert!(
            service["gaps"]
                .as_array()
                .unwrap()
                .iter()
                .any(|gap| gap["code"] == "unclassified-local-facts")
        );
        println!(
            "EARLY_PARITY {}",
            json!({"service":service,"mcp":output.structured()})
        );
        values.push(service);
    }
    assert_eq!(values[0]["origins"], values[1]["origins"]);
    assert_ne!(values[0]["intent_hash"], values[1]["intent_hash"]);
    let raw = values[1]["roles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["role"] == "refinement")
        .unwrap()["raw_hex"]
        .as_str()
        .unwrap();
    assert_eq!(
        raw,
        b"Independent beta refinement bytes\n"
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    assert!(!root.path().join("empty-cache").exists());
}

fn snapshot(root: &std::path::Path) -> Vec<(std::path::PathBuf, Option<Vec<u8>>)> {
    let mut result = vec![];
    let mut todo = vec![root.to_owned()];
    while let Some(path) = todo.pop() {
        for item in std::fs::read_dir(path).unwrap() {
            let path = item.unwrap().path();
            if path.is_dir() {
                todo.push(path.clone());
                result.push((path.strip_prefix(root).unwrap().into(), None));
            } else {
                result.push((
                    path.strip_prefix(root).unwrap().into(),
                    Some(std::fs::read(&path).unwrap()),
                ));
            }
        }
    }
    result.sort();
    result
}
#[test]
fn classified_local_work_adoption_and_missing_evidence_have_exact_pure_parity() {
    let root = classified_fixture();
    let host = root.path().join("host");
    let id = "chg-0123456789abcdef0123456789abcdef";
    let empty = root.path().join("empty-cache");
    let context = vibe_mcp::ServerContext::with_store_root(&host, &empty);
    let before = snapshot(root.path());
    for mode in ["check", "show", "coverage"] {
        let request = json!({"schema":1,"mode":mode,"id":id});
        let response = execute(&host, None, &request);
        let value = serde_json::to_value(&response).unwrap();
        let output = ChangeQueryMcpTool.run(&request, &context).unwrap();
        assert_eq!(
            semantic(value.clone()),
            semantic(output.structured().clone())
        );
        assert_eq!(value["status"], "success");
        assert_eq!(value["scope_complete"], true);
        assert_eq!(value["terminal"], "not-evaluated");
        assert_eq!(value["artifact_evidence"], "not-observed");
        assert_eq!(value["work_items"].as_array().unwrap().len(), 1);
        let facts = value["facts"].as_array().unwrap();
        assert_eq!(facts.len(), 4);
        let local = facts
            .iter()
            .find(|f| {
                f["metadata"]["address"]
                    .as_str()
                    .unwrap()
                    .ends_with("#LOCAL-ACCEPTANCE")
            })
            .unwrap();
        assert_eq!(local["scope"], "local");
        assert_eq!(local["metadata"]["authoring"]["status"]["state"], "done");
        assert_eq!(local["metadata"]["adoption"]["presence"], "not-applicable");
        let original = facts.iter().find(|f| f["scope"] == "origin").unwrap();
        assert_eq!(original["metadata"]["adoption"]["presence"], "recorded");
        if mode == "coverage" {
            assert!(
                value["relation_sources"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|s| s["state"] == "unavailable")
            );
        }
        let after = snapshot(root.path());
        assert_eq!(before, after);
        println!(
            "D2_PURE_PAIR {}",
            json!({"mode":mode,"before":before,"after":after,"service":value,"mcp":output.structured()})
        );
    }
}

fn classified_fixture() -> tempfile::TempDir {
    use super::super::NewArgs;
    use std::fs;
    let root = super::super::m_02_d_1_cases::fixture();
    let host = root.path().join("host");
    let id = "chg-0123456789abcdef0123456789abcdef";
    let args = NewArgs {
        from: "flow:org.demo/dep@=1.0.0".into(),
        id: id.into(),
        slug: "native".into(),
        title: "Native local intent".into(),
        path: host.clone(),
        spec_root: None,
        fact: vec![],
        apply: true,
    };
    super::super::execute(&args).unwrap();
    let directory = host.join("vibevm/vibespecs/changes").join(id);
    let node = host.join("vibe.toml");
    let mut text = fs::read_to_string(&node).unwrap();
    for name in ["intent.md", "refinement.md", "acceptance.md"] {
        text.push_str(&format!("\n[[project.sources]]\nfile='vibevm/vibespecs/changes/{id}/{name}'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n"));
    }
    fs::write(node, text).unwrap();
    fs::write(directory.join("acceptance.md"),"# Acceptance\n\n@fact:LOCAL-ACCEPTANCE Local claimed done. @requires:implementation @status:impl/done\n").unwrap();
    let mut text = fs::read_to_string(directory.join("change.toml")).unwrap();
    text.push_str("\n[[documents]]\nrole='work'\npath='work.toml'\nnormative=false\ninput=true\n");
    fs::write(directory.join("change.toml"), text).unwrap();
    fs::write(directory.join("work.toml"),"schema=1\n[[items]]\nid='work-local'\npredecessors=[]\ntouches=['src/local.rs']\n[[items.needs]]\nkind='implementation'\ntarget='spec://org.demo/dep/DOC#DELIVERY'\n").unwrap();
    // Adoption setup is explicit; subsequent query must never update it.
    let mut registry = vibe_facts::Registry::load(&host).unwrap();
    registry
        .upsert(
            &host,
            vibe_facts::FactEntry {
                address: "spec://org.demo/dep/DOC#DELIVERY".into(),
                origin: vibe_facts::FactOrigin::Package,
                package: Some("org.demo/dep".into()),
                status: Some(vibe_facts::FactStatus::parse("impl/done").unwrap()),
                comment: None,
            },
        )
        .unwrap();
    let empty = root.path().join("empty-cache");
    fs::create_dir(&empty).unwrap();
    root
}

#[test]
fn save_owned_classified_fixture_for_production_processes() {
    let root = classified_fixture().keep();
    println!("D2_PROCESS_ROOT {}", root.display());
}

#[test]
fn unknown_fields_roles_and_invalid_identity_have_exact_refusal_parity() {
    let root = classified_fixture();
    let host = root.path().join("host");
    let context = vibe_mcp::ServerContext::with_store_root(&host, root.path().join("empty-cache"));
    let before = snapshot(root.path());
    for (patch, code) in [
        (json!({"schema":99}), "unknown-epoch"),
        (json!({"root":"/elsewhere"}), "invalid-request"),
        (json!({"id":"../escape"}), "invalid-id"),
        (json!({"roles":["bogus"]}), "invalid-request"),
        (json!({"roles":["intent","intent"]}), "invalid-request"),
        (json!({"roles":["closure"]}), "unknown-role"),
    ] {
        let mut request =
            json!({"schema":1,"mode":"check","id":"chg-0123456789abcdef0123456789abcdef"});
        request
            .as_object_mut()
            .unwrap()
            .extend(patch.as_object().unwrap().clone());
        let value = serde_json::to_value(execute(&host, None, &request)).unwrap();
        assert_eq!(value["code"], code);
        let vibe_mcp::ToolError::StructuredPreflight { report, .. } =
            ChangeQueryMcpTool.run(&request, &context).unwrap_err()
        else {
            panic!("wrong channel")
        };
        assert_eq!(value, report);
        assert_eq!(before, snapshot(root.path()));
        println!("D2_REFUSAL {}", json!({"request":request,"response":value}));
    }
}

#[test]
fn actual_current_relations_are_traceability_not_artifact_validity() {
    use std::fs;
    let root = classified_fixture();
    let host = root.path().join("host");
    let id = "chg-0123456789abcdef0123456789abcdef";
    fs::create_dir_all(host.join("crates/demo/src")).unwrap();
    fs::write(
        host.join("crates/demo/Cargo.toml"),
        "[package]\nname='demo'\nversion='0.1.0'\nedition='2024'\n",
    )
    .unwrap();
    fs::write(
        host.join("specmap.toml"),
        "namespace='org.demo/consumer'\nscan_roots=['crates/*']\nspec_roots=['vibevm/vibespecs']\n",
    )
    .unwrap();
    let address = format!("spec://org.demo/consumer/changes/{id}/acceptance#LOCAL-ACCEPTANCE");
    fs::write(host.join("crates/demo/src/lib.rs"),format!("#[specmark::spec(implements = \"{address}\")]\npub fn local_implementation() {{}}\n#[specmark::spec(verifies = \"{address}\")]\npub fn local_verification() {{}}\n")).unwrap();
    let context = vibe_mcp::ServerContext::with_store_root(&host, root.path().join("empty-cache"));
    let before = snapshot(root.path());
    let request = json!({"schema":1,"mode":"coverage","id":id});
    let value = serde_json::to_value(execute(&host, None, &request)).unwrap();
    let mcp = ChangeQueryMcpTool.run(&request, &context).unwrap();
    assert_eq!(semantic(value.clone()), semantic(mcp.structured().clone()));
    assert_eq!(value["status"], "success");
    println!("D2_CURRENT_RELATION_ATTEMPT {}", value);
    let fact = value["facts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["metadata"]["address"] == address)
        .unwrap();
    let relations = fact["metadata"]["relations"].as_array().unwrap();
    assert_eq!(relations.len(), 2);
    assert!(relations.iter().any(|r| r["verb"] == "implements"));
    assert!(relations.iter().any(|r| r["verb"] == "verifies"));
    let source = value["relation_sources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["package"] == "org.demo/consumer")
        .unwrap();
    assert_eq!(source["state"], "current");
    assert_eq!(value["artifact_evidence"], "not-observed");
    assert_eq!(value["terminal"], "not-evaluated");
    assert_eq!(before, snapshot(root.path()));
    println!("D2_CURRENT_RELATIONS {}", value);
}

#[test]
fn complete_origin_scope_exceeds_public_query_cap_through_real_adapters() {
    use std::fs;
    let root = super::super::m_02_d_1_cases::fixture();
    let host = root.path().join("host");
    let source = root.path().join("source");
    let mut body = String::from("# Large {#root}\n\n");
    let expected: Vec<String> = (0..320)
        .map(|n| format!("spec://org.demo/dep/DOC#DELIVERY-{n:03}"))
        .collect();
    for n in 0..320 {
        body.push_str(&format!(
            "@fact:DELIVERY-{n:03} Exact origin {n}. @status:spec\n\n"
        ));
    }
    fs::write(source.join("vibevm/vibespecs/DOC.md"), body).unwrap();
    let hash = vibe_core::ContentHash::parse(&format!("sha256:{}", "a".repeat(64))).unwrap();
    vibe_workspace::vibedeps::materialise_with_spec_format(
        &host,
        &"org.demo".parse().unwrap(),
        "dep",
        &"1.0.0".parse().unwrap(),
        &source,
        vibe_workspace::vibedeps::CopyMode::Copy,
        vibe_core::manifest::SpecFormat::Mixed,
        &hash,
    )
    .unwrap();
    let id = "chg-0123456789abcdef0123456789abcdef";
    super::super::execute(&super::super::NewArgs {
        from: "flow:org.demo/dep@=1.0.0".into(),
        id: id.into(),
        slug: "large".into(),
        title: "Large exact origin".into(),
        path: host.clone(),
        spec_root: None,
        fact: vec![],
        apply: true,
    })
    .unwrap();
    let mut node = fs::read_to_string(host.join("vibe.toml")).unwrap();
    for name in ["intent.md", "refinement.md", "acceptance.md"] {
        node.push_str(&format!("\n[[project.sources]]\nfile='vibevm/vibespecs/changes/{id}/{name}'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n"));
    }
    fs::write(host.join("vibe.toml"), node).unwrap();
    let context = vibe_mcp::ServerContext::with_store_root(&host, root.path().join("empty-cache"));
    let before = snapshot(root.path());
    let request = json!({"schema":1,"mode":"check","id":id});
    let value = serde_json::to_value(execute(&host, None, &request)).unwrap();
    assert_eq!(value["status"], "success");
    assert_eq!(value["scope_complete"], true);
    let actual: Vec<_> = value["facts"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["scope"] == "origin")
        .map(|f| f["metadata"]["address"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(actual, expected);
    assert_eq!(value["facts"].as_array().unwrap().len(), 323);
    let mcp = ChangeQueryMcpTool.run(&request, &context).unwrap();
    assert_eq!(semantic(value.clone()), semantic(mcp.structured().clone()));
    assert_eq!(before, snapshot(root.path()));
    println!(
        "D2_LARGE {}",
        json!({"expected":expected,"response":value,"snapshots_equal":true})
    );
}

#[test]
fn unbound_drift_and_missing_origin_remain_incomplete_with_precise_gaps() {
    use std::fs;
    for kind in ["unbound", "drift", "missing"] {
        let root = classified_fixture();
        let host = root.path().join("host");
        let id = "chg-0123456789abcdef0123456789abcdef";
        let manifest = host
            .join("vibevm/vibespecs/changes")
            .join(id)
            .join("change.toml");
        if kind == "missing" {
            fs::remove_dir_all(vibe_workspace::vibedeps::slot_abs_path(
                &host,
                &"org.demo".parse().unwrap(),
                "dep",
                &"1.0.0".parse().unwrap(),
            ))
            .unwrap();
        } else {
            let text = fs::read_to_string(&manifest).unwrap();
            let old = format!("selected_content_hash = \"sha256:{}\"", "a".repeat(64));
            assert!(text.contains(&old));
            let replacement = if kind == "unbound" {
                String::new()
            } else {
                format!("selected_content_hash = \"sha256:{}\"", "f".repeat(64))
            };
            fs::write(manifest, text.replacen(&old, &replacement, 1)).unwrap();
        }
        let before = snapshot(root.path());
        let request = json!({"schema":1,"mode":"check","id":id});
        let value = serde_json::to_value(execute(&host, None, &request)).unwrap();
        assert_eq!(value["status"], "success");
        assert_eq!(value["scope_complete"], false);
        assert_eq!(value["terminal"], "not-evaluated");
        assert_eq!(
            value["origins"][0]["state"],
            if kind == "missing" {
                "unavailable"
            } else {
                kind
            }
        );
        assert!(
            value["facts"]
                .as_array()
                .unwrap()
                .iter()
                .all(|f| f["scope"] == "local")
        );
        assert!(
            value["gaps"]
                .as_array()
                .unwrap()
                .iter()
                .any(|g| g["scope"] == "origin")
        );
        let context =
            vibe_mcp::ServerContext::with_store_root(&host, root.path().join("empty-cache"));
        let mcp = ChangeQueryMcpTool.run(&request, &context).unwrap();
        assert_eq!(semantic(value.clone()), semantic(mcp.structured().clone()));
        assert_eq!(before, snapshot(root.path()));
        println!("D2_ORIGIN_GAP {}", json!({"kind":kind,"response":value}));
    }
}

#[test]
fn escaping_authored_role_refuses_shared_grammar_without_query_effects() {
    let root = classified_fixture();
    let host = root.path().join("host");
    let id = "chg-0123456789abcdef0123456789abcdef";
    let path = host
        .join("vibevm/vibespecs/changes")
        .join(id)
        .join("change.toml");
    let text = std::fs::read_to_string(&path).unwrap();
    let old = "path = \"intent.md\"";
    assert!(text.contains(old));
    std::fs::write(path, text.replacen(old, "path = \"../escape.md\"", 1)).unwrap();
    let before = snapshot(root.path());
    let request = json!({"schema":1,"mode":"check","id":id});
    let value = serde_json::to_value(execute(&host, None, &request)).unwrap();
    assert_eq!(value["code"], "invalid-change");
    assert!(value["message"].as_str().unwrap().contains("path"));
    let context = vibe_mcp::ServerContext::with_store_root(&host, root.path().join("empty-cache"));
    let vibe_mcp::ToolError::StructuredPreflight { report, .. } =
        ChangeQueryMcpTool.run(&request, &context).unwrap_err()
    else {
        panic!("wrong channel")
    };
    assert_eq!(value, report);
    assert_eq!(before, snapshot(root.path()));
}
