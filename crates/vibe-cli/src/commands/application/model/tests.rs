use super::*;
use serde_json::json;

fn current_index() -> serde_json::Value {
    json!({
        "protocol": INDEX_PROTOCOL,
        "revision": "fixture-revision",
        "applications": {
            "org.demo/demo@1.0.0": {
                "application": {
                    "id": "demo",
                    "package": {"group": "org.demo", "name": "demo", "version": "1.0.0"},
                    "installerPackage": {"group": "org.demo", "name": "installer", "version": "1.0.0"},
                    "commands": ["demo"]
                },
                "hostRoot": "/settings/opt/apps/demo/org.demo/demo/1.0.0",
                "management": {"runtime": "node", "entry": "/settings/opt/apps/demo/org.demo/demo/1.0.0/manage.js"},
                "status": "ready",
                "publicCommands": []
            }
        },
        "alternatives": {"demo": {"globalDefault": "org.demo/demo@1.0.0", "reservedCommands": [], "runners": []}}
    })
}

#[test]
fn index_record_loads_optional_fields() {
    let wire: index_wire::ApplicationIndex = serde_json::from_value(current_index()).unwrap();
    let index = ApplicationIndex::from_wire(wire).unwrap();
    let record = &index.applications["org.demo/demo@1.0.0"];
    assert_eq!(record.app_root, None);
    assert_eq!(record.install_kind, None);
    assert_eq!(record.selected_option_id, None);
    assert_eq!(record.selected_platform, None);
    assert!(record.user_launchers.is_empty());
}

#[test]
fn index_round_trips_payload_and_user_launcher_templates() {
    let wire: index_wire::ApplicationIndex = serde_json::from_value(current_index()).unwrap();
    let mut index = ApplicationIndex::from_wire(wire).unwrap();
    let record = index.applications.get_mut("org.demo/demo@1.0.0").unwrap();
    record.app_root = Some(PathBuf::from("/external payload/Пример"));
    record.install_kind = Some(InstallKind::TarGz);
    record.selected_option_id = Some("homebrew".into());
    record.selected_platform = Some("macos/aarch64".into());
    record.management.runtime = ManagementRuntime::Native;
    record.user_launchers.push(UserLauncher {
        id: "main".into(),
        name: "Example".into(),
        description: None,
        executable: "$app_root/bin/example".into(),
        args: vec!["--literal=$project_root".into()],
        working_directory: Some("$project_root".into()),
        icon: None,
        terminal: false,
        default: true,
    });
    let encoded = serde_json::to_value(index.to_wire()).unwrap();
    assert_eq!(
        encoded["applications"]["org.demo/demo@1.0.0"]["appRoot"],
        "/external payload/Пример"
    );
    assert_eq!(
        encoded["applications"]["org.demo/demo@1.0.0"]["installKind"],
        "tar_gz"
    );
    assert_eq!(
        encoded["applications"]["org.demo/demo@1.0.0"]["selectedOptionId"],
        "homebrew"
    );
    assert_eq!(
        encoded["applications"]["org.demo/demo@1.0.0"]["selectedPlatform"],
        "macos/aarch64"
    );
    assert_eq!(
        encoded["applications"]["org.demo/demo@1.0.0"]["management"]["runtime"],
        "native"
    );
    assert_eq!(
        encoded["applications"]["org.demo/demo@1.0.0"]["userLaunchers"][0]["executable"],
        "$app_root/bin/example"
    );
    let decoded = ApplicationIndex::from_wire(serde_json::from_value(encoded).unwrap()).unwrap();
    assert_eq!(decoded, index);
}

#[test]
fn old_installer_reply_has_no_payload_root() {
    let wire: reply_wire::ApplicationReply = serde_json::from_value(json!({
        "protocol": RESULT_PROTOCOL,
        "operation": "install",
        "applicationId": "demo",
        "status": "ready",
        "hostRoot": "/settings/opt/apps/demo",
        "management": {"runtime": "node", "entry": "/settings/opt/apps/demo/manage.js"},
        "commands": ["demo"],
        "message": "ready"
    }))
    .unwrap();
    assert_eq!(ApplicationReply::from_wire(wire).app_root, None);
}
