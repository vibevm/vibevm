use super::*;
use std::io::Write;

fn write_json(path: &Path, value: serde_json::Value) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut file = File::create(path).unwrap();
    file.write_all(serde_json::to_string(&value).unwrap().as_bytes())
        .unwrap();
}

#[cfg(unix)]
#[test]
fn direct_sidecar_keeps_fixed_and_caller_arguments() {
    let root = tempfile::tempdir().unwrap();
    let sidecar = root.path().join("demo.proxy.json");
    let lease = root
        .path()
        .join("applications/leases")
        .join(format!("{}.lock", "a".repeat(64)));
    fs::create_dir_all(lease.parent().unwrap()).unwrap();
    write_json(
        &sidecar,
        serde_json::json!({
            "protocol": PROXY_PROTOCOL, "mode": "direct", "target": "/bin/echo",
            "args": ["fixed value"], "cwd": root.path(),
            "lease": lease
        }),
    );
    let resolved = resolve(&sidecar).unwrap();
    assert!(matches!(resolved.descriptor, Proxy::Direct { .. }));
    assert!(validate_direct(Path::new("/bin/echo"), Some(root.path()), &lease).is_ok());
    assert!(
        matches!(read_json::<Proxy>(&sidecar, MAX_SIDECAR_BYTES).unwrap(), Proxy::Direct { args, .. } if args == ["fixed value"])
    );
    let caller = [
        OsString::from("$(touch unsafe)"),
        OsString::from("two words"),
    ];
    let output = direct_command(
        Path::new("/bin/echo"),
        &["fixed value".into()],
        Some(root.path()),
        &caller,
    )
    .output()
    .unwrap();
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "fixed value $(touch unsafe) two words\n"
    );
    assert!(!root.path().join("unsafe").exists());
    let cwd = direct_command(Path::new("/bin/pwd"), &[], Some(root.path()), &[])
        .output()
        .unwrap();
    let actual_cwd = String::from_utf8(cwd.stdout).unwrap();
    assert_eq!(
        fs::canonicalize(actual_cwd.trim()).unwrap(),
        fs::canonicalize(root.path()).unwrap()
    );
    assert_eq!(execute(&resolved, &[]).unwrap(), 0);
}

#[test]
fn slot_pointer_selects_immutable_projection_descriptor() {
    let root = tempfile::tempdir().unwrap();
    let slot_dir = root.path().join("opt/alternatives/demo");
    let current = slot_dir.join("current.json");
    let alias = root.path().join("bin/demo.proxy.json");
    write_json(
        &alias,
        serde_json::json!({"protocol":PROXY_PROTOCOL,"mode":"slot","slot":"demo","command":"demo","current":current}),
    );
    write_json(
        &current,
        serde_json::json!({"protocol":CURRENT_PROTOCOL,"projection":"a1b2"}),
    );
    let descriptor = slot_dir.join("projections/a1b2/commands/demo.json");
    write_json(
        &descriptor,
        serde_json::json!({"protocol":PROXY_PROTOCOL,"mode":"unavailable","reason":"removed"}),
    );
    let resolved = resolve(&alias).unwrap();
    assert_eq!(resolved.descriptor_path, descriptor);
    assert!(matches!(resolved.descriptor, Proxy::Unavailable { .. }));
}

#[test]
fn view_pointer_selects_exact_installation_projection() {
    let root = tempfile::tempdir().unwrap();
    let key = "org.example/demo@1.0.0";
    let encoded = format!("{:x}", Sha256::digest(key.as_bytes()));
    let view = root
        .path()
        .join("opt/alternatives/demo/installations")
        .join(encoded);
    let current = view.join("current.json");
    let alias = view.join("bin/demo.proxy.json");
    write_json(
        &alias,
        serde_json::json!({
            "protocol":PROXY_PROTOCOL,"mode":"view","slot":"demo",
            "installation":key,"command":"demo","current":current
        }),
    );
    write_json(
        &current,
        serde_json::json!({"protocol":CURRENT_PROTOCOL,"projection":"a1b2"}),
    );
    let descriptor = view.join("projections/a1b2/commands/demo.json");
    write_json(
        &descriptor,
        serde_json::json!({"protocol":PROXY_PROTOCOL,"mode":"unavailable","reason":"not ready"}),
    );
    assert_eq!(resolve(&alias).unwrap().descriptor_path, descriptor);
    write_json(
        &alias,
        serde_json::json!({
            "protocol":PROXY_PROTOCOL,"mode":"view","slot":"demo",
            "installation":"org.example/demo@2.0.0","command":"demo","current":current
        }),
    );
    assert!(resolve(&alias).is_err());
}

#[test]
fn rejects_traversal_and_malformed_descriptors() {
    let root = tempfile::tempdir().unwrap();
    let alias = root.path().join("demo.proxy.json");
    write_json(
        &alias,
        serde_json::json!({"protocol":PROXY_PROTOCOL,"mode":"slot","slot":"demo","command":"demo","current":"../current.json"}),
    );
    assert!(resolve(&alias).is_err());
    write_json(
        &alias,
        serde_json::json!({"protocol":"bad","mode":"unavailable","reason":"gone"}),
    );
    assert!(resolve(&alias).is_err());
    fs::write(&alias, vec![b'x'; MAX_SIDECAR_BYTES as usize + 1]).unwrap();
    assert!(resolve(&alias).is_err());
}
