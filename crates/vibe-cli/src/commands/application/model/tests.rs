use super::*;
use serde_json::json;

fn old_index() -> serde_json::Value {
    json!({
        "protocol": INDEX_PROTOCOL,
        "applications": {
            "demo": {
                "application": {
                    "id": "demo",
                    "package": {"group": "org.demo", "name": "demo", "version": "1.0.0"},
                    "installerPackage": {"group": "org.demo", "name": "installer", "version": "1.0.0"},
                    "commands": ["demo"]
                },
                "hostRoot": "/settings/opt/apps/demo",
                "management": {"runtime": "node", "entry": "/settings/opt/apps/demo/manage.js"},
                "status": "ready"
            }
        }
    })
}

#[test]
fn old_index_record_loads_without_new_fields() {
    let wire: index_wire::ApplicationIndex = serde_json::from_value(old_index()).unwrap();
    let index = ApplicationIndex::from_wire(wire).unwrap();
    let record = &index.applications["demo"];
    assert_eq!(record.app_root, None);
    assert_eq!(record.install_kind, None);
    assert_eq!(record.selected_option_id, None);
    assert_eq!(record.selected_platform, None);
    assert!(record.user_launchers.is_empty());
}

#[test]
fn index_round_trips_payload_and_user_launcher_templates() {
    let wire: index_wire::ApplicationIndex = serde_json::from_value(old_index()).unwrap();
    let mut index = ApplicationIndex::from_wire(wire).unwrap();
    let record = index.applications.get_mut("demo").unwrap();
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
        encoded["applications"]["demo"]["appRoot"],
        "/external payload/Пример"
    );
    assert_eq!(encoded["applications"]["demo"]["installKind"], "tar_gz");
    assert_eq!(
        encoded["applications"]["demo"]["selectedOptionId"],
        "homebrew"
    );
    assert_eq!(
        encoded["applications"]["demo"]["selectedPlatform"],
        "macos/aarch64"
    );
    assert_eq!(
        encoded["applications"]["demo"]["management"]["runtime"],
        "native"
    );
    assert_eq!(
        encoded["applications"]["demo"]["userLaunchers"][0]["executable"],
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
