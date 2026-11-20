//! First durable consumer through separate public processes, synthetic only.
use std::{
    fs,
    io::Write,
    process::{Command, Stdio},
};
const VALUE: &str = "SYNTHETIC_AI_REFERENCE_KEY_NEVER_OUTPUT";
struct F(tempfile::TempDir);
impl F {
    fn new() -> Self {
        use std::os::unix::fs::PermissionsExt;
        Self(
            tempfile::Builder::new()
                .permissions(fs::Permissions::from_mode(0o700))
                .tempdir()
                .unwrap(),
        )
    }
    fn run(&self, args: &[&str], input: Option<&[u8]>) -> std::process::Output {
        let mut c = Command::new(env!("CARGO_BIN_EXE_vibe"));
        c.env_clear()
            .env("HOME", self.0.path())
            .env("VIBE_SETTINGS", self.0.path())
            .env("PATH", "/usr/bin:/bin")
            .args(args)
            .arg("--json")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut c = c.spawn().unwrap();
        if let Some(v) = input {
            c.stdin.take().unwrap().write_all(v).unwrap();
        } else {
            drop(c.stdin.take());
        }
        let o = c.wait_with_output().unwrap();
        assert!(!String::from_utf8_lossy(&o.stdout).contains(VALUE));
        assert!(!String::from_utf8_lossy(&o.stderr).contains(VALUE));
        o
    }
    fn json(&self, args: &[&str], input: Option<&[u8]>) -> serde_json::Value {
        let o = self.run(args, input);
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        serde_json::from_slice(&o.stdout).unwrap()
    }
}
#[cfg(unix)]
#[test]
fn connection_reference_follows_rotation_blocks_delete_and_explicit_unbinding_releases_it() {
    let f = F::new();
    let empty = f.json(&["ai", "connections", "list"], None);
    assert_eq!(empty["items"], serde_json::json!({}));
    assert!(!f.0.path().join("secrets").exists());
    let secret = f.json(
        &[
            "secrets",
            "create",
            "--kind",
            "api-key",
            "--label",
            "Synthetic",
            "--value-stdin",
        ],
        Some(VALUE.as_bytes()),
    );
    let id = secret["items"][0]["id"].as_str().unwrap();
    let rev = secret["items"][0]["current_revision"].as_str().unwrap();
    fs::write(f.0.path().join("foreign"), b"KEEP").unwrap();
    let added = f.json(
        &[
            "ai",
            "connections",
            "add",
            "--label",
            "Synthetic connection",
            "--endpoint",
            "http://127.0.0.1:32123/v1",
            "--api-kind",
            "openai-chat-completions",
            "--default-model",
            "synthetic-model",
            "--secret-ref",
            id,
        ],
        None,
    );
    let rows = added["items"].as_object().unwrap();
    let (connection, row) = rows.iter().next().unwrap();
    assert_eq!(row["secret_ref"], id);
    let connrev = row["current_revision"].as_str().unwrap();
    let version = added["store_revision"].as_u64().unwrap().to_string();
    assert_eq!(
        f.json(&["ai", "connections", "details", connection], None),
        added
    );
    let rotated = f.json(
        &[
            "secrets",
            "rotate",
            id,
            "--expect-current",
            rev,
            "--value-stdin",
        ],
        Some(VALUE.as_bytes()),
    );
    let newrev = rotated["items"][0]["current_revision"].as_str().unwrap();
    assert_ne!(newrev, rev);
    let details = f.json(&["ai", "connections", "details", connection], None);
    assert_eq!(details["items"][connection]["secret_ref"], id);
    let before = fs::read(f.0.path().join("secrets/index.toml")).unwrap();
    let store = rotated["store_revision"].as_u64().unwrap().to_string();
    assert!(
        !f.run(
            &[
                "secrets",
                "remove",
                id,
                "--expect-store",
                &store,
                "--expect-current",
                newrev,
                "--yes"
            ],
            None
        )
        .status
        .success()
    );
    assert_eq!(
        fs::read(f.0.path().join("secrets/index.toml")).unwrap(),
        before
    );
    assert!(
        !f.run(
            &[
                "ai",
                "connections",
                "remove",
                connection,
                "--expect-store",
                "0",
                "--expect-current",
                connrev,
                "--yes"
            ],
            None
        )
        .status
        .success()
    );
    f.json(
        &[
            "ai",
            "connections",
            "remove",
            connection,
            "--expect-store",
            &version,
            "--expect-current",
            connrev,
            "--yes",
        ],
        None,
    );
    f.json(
        &[
            "secrets",
            "remove",
            id,
            "--expect-store",
            &store,
            "--expect-current",
            newrev,
            "--yes",
        ],
        None,
    );
    assert_eq!(fs::read(f.0.path().join("foreign")).unwrap(), b"KEEP");
    assert!(
        f.0.path()
            .join("secrets/items")
            .join(id)
            .join(newrev)
            .join("value")
            .exists()
    );
}
#[cfg(unix)]
#[test]
fn unknown_secret_and_credential_url_refuse_without_binding() {
    let f = F::new();
    let unknown = "11111111-1111-4111-8111-111111111111";
    for endpoint in [
        "https://user:pass@example.invalid/v1",
        "https://example.invalid/v1?token=bad",
        "http://example.invalid/v1",
        "http://127.0.0.1:32123/v1",
    ] {
        let out = f.run(
            &[
                "ai",
                "connections",
                "add",
                "--label",
                "Synthetic",
                "--endpoint",
                endpoint,
                "--api-kind",
                "openai-chat-completions",
                "--default-model",
                "synthetic",
                "--secret-ref",
                unknown,
            ],
            None,
        );
        assert!(!out.status.success());
        assert!(!f.0.path().join("secrets/ai").exists());
    }
}

#[cfg(unix)]
#[test]
fn corrupted_metadata_and_missing_registered_index_fail_closed() {
    for corruption in [0, 1, 2] {
        let f = F::new();
        let secret = f.json(
            &[
                "secrets",
                "create",
                "--kind",
                "api-key",
                "--label",
                "Synthetic",
                "--value-stdin",
            ],
            Some(VALUE.as_bytes()),
        );
        let id = secret["items"][0]["id"].as_str().unwrap();
        let added = f.json(
            &[
                "ai",
                "connections",
                "add",
                "--label",
                "Synthetic",
                "--endpoint",
                "http://127.0.0.1:32123/v1",
                "--api-kind",
                "openai-chat-completions",
                "--default-model",
                "synthetic",
                "--secret-ref",
                id,
            ],
            None,
        );
        let (connection, row) = added["items"].as_object().unwrap().iter().next().unwrap();
        let selected = f.json(&["secrets", "details", id], None);
        let version = selected["store_revision"].as_u64().unwrap().to_string();
        let revision = selected["items"][0]["current_revision"].as_str().unwrap();
        if corruption == 2 {
            fs::write(
                f.0.path()
                    .join("secrets/ai/connections")
                    .join(connection)
                    .join(row["current_revision"].as_str().unwrap())
                    .join("config.toml"),
                b"CORRUPTED",
            )
            .unwrap();
        } else if corruption == 1 {
            fs::write(
                f.0.path().join("secrets/ai/index.toml"),
                b"schema = 1\nstore_revision = 1\n",
            )
            .unwrap();
        } else {
            fs::rename(
                f.0.path().join("secrets/ai/index.toml"),
                f.0.path().join("saved-index"),
            )
            .unwrap();
        }
        let before = fs::read(f.0.path().join("secrets/index.toml")).unwrap();
        let refused = f.run(
            &[
                "secrets",
                "remove",
                id,
                "--expect-store",
                &version,
                "--expect-current",
                revision,
                "--yes",
            ],
            None,
        );
        assert!(!refused.status.success());
        assert_eq!(
            fs::read(f.0.path().join("secrets/index.toml")).unwrap(),
            before
        );
        assert!(!f.run(&["ai", "connections", "list"], None).status.success());
        assert!(
            f.0.path()
                .join("secrets/items")
                .join(id)
                .join(revision)
                .join("value")
                .exists()
        );
    }
}

#[cfg(unix)]
#[test]
fn default_selection_is_metadata_only_revision_checked_and_blocks_selected_removal() {
    let f = F::new();
    let shown = f.json(&["ai", "connections", "default", "show"], None);
    assert!(shown["connection"].is_null());
    assert_eq!(shown["store_revision"], 0);
    assert!(!f.0.path().join("secrets").exists());
    let secret = f.json(
        &[
            "secrets",
            "create",
            "--kind",
            "api-key",
            "--label",
            "Synthetic",
            "--value-stdin",
        ],
        Some(VALUE.as_bytes()),
    );
    let secret_id = secret["items"][0]["id"].as_str().unwrap();
    let add = |label: &str| {
        f.json(
            &[
                "ai",
                "connections",
                "add",
                "--label",
                label,
                "--endpoint",
                "http://127.0.0.1:32123/v1",
                "--api-kind",
                "openai-chat-completions",
                "--default-model",
                "synthetic",
                "--secret-ref",
                secret_id,
            ],
            None,
        )
    };
    let a = add("A");
    let aid = a["items"]
        .as_object()
        .unwrap()
        .keys()
        .next()
        .unwrap()
        .clone();
    let b = add("B");
    let bid = b["items"]
        .as_object()
        .unwrap()
        .keys()
        .next()
        .unwrap()
        .clone();
    let brev = b["items"][&bid]["current_revision"].as_str().unwrap();
    let path = f.0.path().join("secrets/ai/index.toml");
    let original = fs::read(&path).unwrap();
    let version = b["store_revision"].as_u64().unwrap().to_string();
    let cleared = f.json(
        &[
            "ai",
            "connections",
            "default",
            "clear",
            "--expect-store",
            &version,
            "--yes",
        ],
        None,
    );
    assert_eq!(cleared["store_revision"], b["store_revision"]);
    assert_eq!(fs::read(&path).unwrap(), original);
    let selected = f.json(
        &[
            "ai",
            "connections",
            "default",
            "set",
            &aid,
            "--expect-store",
            &version,
        ],
        None,
    );
    assert_eq!(selected["connection"], aid);
    assert_eq!(
        f.json(&["ai", "connections", "default", "show"], None),
        selected
    );
    let ver = selected["store_revision"].as_u64().unwrap().to_string();
    let before = fs::read(&path).unwrap();
    assert!(String::from_utf8_lossy(&before).contains("schema = 2"));
    assert_eq!(
        f.json(
            &[
                "ai",
                "connections",
                "default",
                "set",
                &aid,
                "--expect-store",
                &ver
            ],
            None
        ),
        selected
    );
    assert_eq!(fs::read(&path).unwrap(), before);
    assert!(
        !f.run(
            &[
                "ai",
                "connections",
                "default",
                "set",
                &bid,
                "--expect-store",
                &version
            ],
            None
        )
        .status
        .success()
    );
    assert_eq!(fs::read(&path).unwrap(), before);
    let selected_b = f.json(
        &[
            "ai",
            "connections",
            "default",
            "set",
            &bid,
            "--expect-store",
            &ver,
        ],
        None,
    );
    assert_eq!(selected_b["connection"], bid);
    let ver = selected_b["store_revision"].as_u64().unwrap().to_string();
    let before = fs::read(&path).unwrap();
    assert!(
        !f.run(
            &[
                "ai",
                "connections",
                "remove",
                &bid,
                "--expect-store",
                &ver,
                "--expect-current",
                brev,
                "--yes"
            ],
            None
        )
        .status
        .success()
    );
    assert_eq!(fs::read(&path).unwrap(), before);
    assert!(
        !f.run(
            &[
                "ai",
                "connections",
                "default",
                "clear",
                "--expect-store",
                &ver
            ],
            None
        )
        .status
        .success()
    );
    assert_eq!(fs::read(&path).unwrap(), before);
    let cleared = f.json(
        &[
            "ai",
            "connections",
            "default",
            "clear",
            "--expect-store",
            &ver,
            "--yes",
        ],
        None,
    );
    assert!(cleared["connection"].is_null());
    assert_eq!(
        f.json(&["ai", "connections", "default", "show"], None),
        cleared
    );
    let ver = cleared["store_revision"].as_u64().unwrap().to_string();
    f.json(
        &[
            "ai",
            "connections",
            "remove",
            &bid,
            "--expect-store",
            &ver,
            "--expect-current",
            brev,
            "--yes",
        ],
        None,
    );
    let listing = f.json(&["ai", "connections", "list"], None);
    assert!(listing["items"].get(&aid).is_some());
    assert!(listing["items"].get(&bid).is_none());
    assert!(
        f.json(&["secrets", "details", secret_id], None)["items"][0]["value_present"]
            .as_bool()
            .unwrap()
    );
}

#[cfg(unix)]
#[test]
fn v2_missing_unknown_or_dangling_selection_refuses_without_rewrite() {
    let f = F::new();
    let secret = f.json(
        &[
            "secrets",
            "create",
            "--kind",
            "api-key",
            "--label",
            "Synthetic",
            "--value-stdin",
        ],
        Some(VALUE.as_bytes()),
    );
    let id = secret["items"][0]["id"].as_str().unwrap();
    let added = f.json(
        &[
            "ai",
            "connections",
            "add",
            "--label",
            "Synthetic",
            "--endpoint",
            "http://127.0.0.1:32123/v1",
            "--api-kind",
            "openai-chat-completions",
            "--default-model",
            "synthetic",
            "--secret-ref",
            id,
        ],
        None,
    );
    let (cid, _) = added["items"].as_object().unwrap().iter().next().unwrap();
    let version = added["store_revision"].as_u64().unwrap().to_string();
    f.json(
        &[
            "ai",
            "connections",
            "default",
            "set",
            cid,
            "--expect-store",
            &version,
        ],
        None,
    );
    let path = f.0.path().join("secrets/ai/index.toml");
    let original = fs::read_to_string(&path).unwrap();
    for mode in [0, 1, 2] {
        let mut index: toml::Value = toml::from_str(&original).unwrap();
        if mode == 0 {
            index.as_table_mut().unwrap().remove("selection");
        }
        if mode == 1 {
            index["selection"]["state"] = toml::Value::String("future".into());
        }
        if mode == 2 {
            index["selection"]["connection"] =
                toml::Value::String("11111111-1111-4111-8111-111111111111".into());
        }
        let bytes = toml::to_string(&index).unwrap();
        fs::write(&path, &bytes).unwrap();
        assert!(
            !f.run(&["ai", "connections", "default", "show"], None)
                .status
                .success()
        );
        assert!(
            !f.run(
                &[
                    "ai",
                    "connections",
                    "default",
                    "clear",
                    "--expect-store",
                    "2",
                    "--yes"
                ],
                None
            )
            .status
            .success()
        );
        assert_eq!(fs::read(&path).unwrap(), bytes.as_bytes());
    }
}

#[cfg(unix)]
#[test]
fn default_selection_does_not_fabricate_per_agent_modes() {
    let f = F::new();
    let secret = f.json(
        &[
            "secrets",
            "create",
            "--kind",
            "api-key",
            "--label",
            "Synthetic",
            "--value-stdin",
        ],
        Some(VALUE.as_bytes()),
    );
    let id = secret["items"][0]["id"].as_str().unwrap();
    let added = f.json(
        &[
            "ai",
            "connections",
            "add",
            "--label",
            "Synthetic",
            "--endpoint",
            "http://127.0.0.1:32123/v1",
            "--api-kind",
            "openai-chat-completions",
            "--default-model",
            "synthetic",
            "--secret-ref",
            id,
        ],
        None,
    );
    let (cid, _) = added["items"].as_object().unwrap().iter().next().unwrap();
    let version = added["store_revision"].as_u64().unwrap().to_string();
    f.json(
        &[
            "ai",
            "connections",
            "default",
            "set",
            cid,
            "--expect-store",
            &version,
        ],
        None,
    );
    let ai = fs::read(f.0.path().join("secrets/ai/index.toml")).unwrap();
    let key = fs::read(f.0.path().join("secrets/index.toml")).unwrap();
    let parsed: toml::Value = toml::from_str(std::str::from_utf8(&ai).unwrap()).unwrap();
    assert!(parsed.get("agents").is_none());
    assert!(parsed.get("bindings").is_none());
    let result = f.json(&["ai", "bindings", "show", "pi"], None);
    assert_eq!(result["binding"]["mode"], "native");
    assert_eq!(result["binding"]["resolution"], "unmanaged");
    assert_eq!(
        fs::read(f.0.path().join("secrets/ai/index.toml")).unwrap(),
        ai
    );
    assert_eq!(
        fs::read(f.0.path().join("secrets/index.toml")).unwrap(),
        key
    );
    assert!(!f.0.path().join("agents.toml").exists());
}

#[cfg(unix)]
#[test]
fn canonical_binding_modes_resolve_inheritance_explicit_and_native_without_launch() {
    let f = F::new();
    assert_eq!(
        f.json(&["ai", "bindings", "show", "pi"], None)["binding"]["mode"],
        "native"
    );
    assert!(!f.0.path().join("secrets").exists());
    let secret = f.json(
        &[
            "secrets",
            "create",
            "--kind",
            "api-key",
            "--label",
            "Synthetic",
            "--value-stdin",
        ],
        Some(VALUE.as_bytes()),
    );
    let id = secret["items"][0]["id"].as_str().unwrap();
    let add = |label: &str| {
        f.json(
            &[
                "ai",
                "connections",
                "add",
                "--label",
                label,
                "--endpoint",
                "http://127.0.0.1:32123/v1",
                "--api-kind",
                "openai-chat-completions",
                "--default-model",
                "synthetic",
                "--secret-ref",
                id,
            ],
            None,
        )
    };
    let a = add("A");
    let aid = a["items"]
        .as_object()
        .unwrap()
        .keys()
        .next()
        .unwrap()
        .clone();
    let b = add("B");
    let bid = b["items"]
        .as_object()
        .unwrap()
        .keys()
        .next()
        .unwrap()
        .clone();
    let brev = b["items"][&bid]["current_revision"].as_str().unwrap();
    let version = b["store_revision"].as_u64().unwrap().to_string();
    let inherited = f.json(
        &[
            "ai",
            "bindings",
            "set",
            "pi",
            "--mode",
            "inherit-default",
            "--expect-store",
            &version,
        ],
        None,
    );
    assert!(inherited["binding"]["connection"].is_null());
    assert!(
        inherited["binding"]["resolution"]
            .as_str()
            .unwrap()
            .starts_with("unavailable")
    );
    let version = inherited["binding"]["store_revision"]
        .as_u64()
        .unwrap()
        .to_string();
    let selected = f.json(
        &[
            "ai",
            "connections",
            "default",
            "set",
            &aid,
            "--expect-store",
            &version,
        ],
        None,
    );
    let version = selected["store_revision"].as_u64().unwrap().to_string();
    assert_eq!(
        f.json(&["ai", "bindings", "show", "pi"], None)["binding"]["connection"],
        aid
    );
    let explicit = f.json(
        &[
            "ai",
            "bindings",
            "set",
            "qwen",
            "--mode",
            "connection",
            "--connection",
            &bid,
            "--expect-store",
            &version,
        ],
        None,
    );
    let version = explicit["binding"]["store_revision"]
        .as_u64()
        .unwrap()
        .to_string();
    let selected = f.json(
        &[
            "ai",
            "connections",
            "default",
            "set",
            &bid,
            "--expect-store",
            &version,
        ],
        None,
    );
    let version = selected["store_revision"].as_u64().unwrap().to_string();
    assert_eq!(
        f.json(&["ai", "bindings", "show", "pi"], None)["binding"]["connection"],
        bid
    );
    assert_eq!(
        f.json(&["ai", "bindings", "show", "qwen"], None)["binding"]["connection"],
        bid
    );
    let cleared = f.json(
        &[
            "ai",
            "connections",
            "default",
            "clear",
            "--expect-store",
            &version,
            "--yes",
        ],
        None,
    );
    let version = cleared["store_revision"].as_u64().unwrap().to_string();
    assert!(f.json(&["ai", "bindings", "show", "pi"], None)["binding"]["connection"].is_null());
    assert_eq!(
        f.json(&["ai", "bindings", "show", "qwen"], None)["binding"]["connection"],
        bid
    );
    let path = f.0.path().join("secrets/ai/index.toml");
    let before = fs::read(&path).unwrap();
    assert!(
        !f.run(
            &[
                "ai",
                "connections",
                "remove",
                &bid,
                "--expect-store",
                &version,
                "--expect-current",
                brev,
                "--yes"
            ],
            None
        )
        .status
        .success()
    );
    assert_eq!(fs::read(&path).unwrap(), before);
    assert!(
        !f.run(
            &[
                "ai",
                "bindings",
                "set",
                "claude",
                "--mode",
                "inherit-default",
                "--expect-store",
                &version
            ],
            None
        )
        .status
        .success()
    );
    assert_eq!(fs::read(&path).unwrap(), before);
    assert!(
        !f.run(
            &[
                "ai",
                "bindings",
                "set",
                "pi",
                "--mode",
                "native",
                "--expect-store",
                "0"
            ],
            None
        )
        .status
        .success()
    );
    assert_eq!(fs::read(&path).unwrap(), before);
    fs::write(
        f.0.path().join("agents.toml"),
        "[launchers.helper]\nkind='qwen'\ncommand=['never-executed']\n",
    )
    .unwrap();
    assert_eq!(
        f.json(&["ai", "bindings", "show", "helper"], None)["binding"]["harness"],
        "qwen"
    );
    let native = f.json(
        &[
            "ai",
            "bindings",
            "set",
            "helper",
            "--mode",
            "native",
            "--expect-store",
            &version,
        ],
        None,
    );
    assert_eq!(native["binding"]["mode"], "native");
    assert_eq!(
        f.json(&["ai", "bindings", "show", "pi"], None)["binding"]["mode"],
        "inherit-default"
    );
}

#[cfg(unix)]
#[test]
fn malformed_binding_maps_and_unknown_managed_identities_fail_closed() {
    let f = F::new();
    let native = f.json(
        &[
            "ai",
            "bindings",
            "set",
            "claude",
            "--mode",
            "native",
            "--expect-store",
            "0",
        ],
        None,
    );
    assert_eq!(native["binding"]["mode"], "native");
    assert!(!f.0.path().join("secrets").exists());
    let secret = f.json(
        &[
            "secrets",
            "create",
            "--kind",
            "api-key",
            "--label",
            "Synthetic",
            "--value-stdin",
        ],
        Some(VALUE.as_bytes()),
    );
    let id = secret["items"][0]["id"].as_str().unwrap();
    let a = f.json(
        &[
            "ai",
            "connections",
            "add",
            "--label",
            "Synthetic",
            "--endpoint",
            "http://127.0.0.1:32123/v1",
            "--api-kind",
            "openai-chat-completions",
            "--default-model",
            "synthetic",
            "--secret-ref",
            id,
        ],
        None,
    );
    let (cid, _) = a["items"].as_object().unwrap().iter().next().unwrap();
    let version = a["store_revision"].as_u64().unwrap().to_string();
    let selected = f.json(
        &[
            "ai",
            "bindings",
            "set",
            "pi",
            "--mode",
            "connection",
            "--connection",
            cid,
            "--expect-store",
            &version,
        ],
        None,
    );
    let version = selected["binding"]["store_revision"]
        .as_u64()
        .unwrap()
        .to_string();
    let path = f.0.path().join("secrets/ai/index.toml");
    let original = fs::read_to_string(&path).unwrap();
    assert_eq!(
        f.json(
            &[
                "ai",
                "bindings",
                "set",
                "pi",
                "--mode",
                "connection",
                "--connection",
                cid,
                "--expect-store",
                &version
            ],
            None
        ),
        selected
    );
    assert_eq!(fs::read_to_string(&path).unwrap(), original);
    for harness in ["codex", "opencode", "hermes", "claude", "not-a-harness"] {
        assert!(
            !f.run(
                &[
                    "ai",
                    "bindings",
                    "set",
                    harness,
                    "--mode",
                    "inherit-default",
                    "--expect-store",
                    &version
                ],
                None
            )
            .status
            .success()
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), original);
    }
    for mode in [0, 1, 2, 3] {
        let mut index: toml::Value = toml::from_str(&original).unwrap();
        if mode == 0 {
            index.as_table_mut().unwrap().remove("bindings");
        }
        if mode == 1 {
            index["bindings"]["pi"]["mode"] = toml::Value::String("future".into());
        }
        if mode == 2 {
            index["bindings"]["pi"]["connection"] =
                toml::Value::String("11111111-1111-4111-8111-111111111111".into());
        }
        if mode == 3 {
            let row = index["bindings"]["pi"].clone();
            index["bindings"]
                .as_table_mut()
                .unwrap()
                .insert("unknown".into(), row);
        }
        let bytes = toml::to_string(&index).unwrap();
        fs::write(&path, &bytes).unwrap();
        assert!(
            !f.run(&["ai", "bindings", "show", "pi"], None)
                .status
                .success()
        );
        assert!(
            !f.run(
                &[
                    "ai",
                    "bindings",
                    "set",
                    "pi",
                    "--mode",
                    "native",
                    "--expect-store",
                    &version
                ],
                None
            )
            .status
            .success()
        );
        assert_eq!(fs::read(&path).unwrap(), bytes.as_bytes());
    }
}

#[cfg(unix)]
#[test]
fn malformed_setup_selection_does_not_mutate_existing_bindings() {
    let f = F::new();
    let secret = f.json(
        &[
            "secrets",
            "create",
            "--kind",
            "api-key",
            "--label",
            "Synthetic",
            "--value-stdin",
        ],
        Some(VALUE.as_bytes()),
    );
    let id = secret["items"][0]["id"].as_str().unwrap();
    let a = f.json(
        &[
            "ai",
            "connections",
            "add",
            "--label",
            "Synthetic",
            "--endpoint",
            "http://127.0.0.1:32123/v1",
            "--api-kind",
            "openai-chat-completions",
            "--default-model",
            "synthetic",
            "--secret-ref",
            id,
        ],
        None,
    );
    let (cid, _) = a["items"].as_object().unwrap().iter().next().unwrap();
    let version = a["store_revision"].as_u64().unwrap().to_string();
    f.json(
        &[
            "ai",
            "bindings",
            "set",
            "qwen",
            "--mode",
            "connection",
            "--connection",
            cid,
            "--expect-store",
            &version,
        ],
        None,
    );
    let ai = fs::read(f.0.path().join("secrets/ai/index.toml")).unwrap();
    let key = fs::read(f.0.path().join("secrets/index.toml")).unwrap();
    let out = f.run(
        &["ai", "setup", "--agents", "pi,qwen", "--connection", cid],
        None,
    );
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("unexpected argument"));
    assert_eq!(
        fs::read(f.0.path().join("secrets/ai/index.toml")).unwrap(),
        ai
    );
    assert_eq!(
        fs::read(f.0.path().join("secrets/index.toml")).unwrap(),
        key
    );
    assert_eq!(
        f.json(&["ai", "bindings", "show", "qwen"], None)["binding"]["connection"],
        cid.as_str()
    );
    assert_eq!(
        f.json(&["ai", "bindings", "show", "claude"], None)["binding"]["mode"],
        "native"
    );
    assert!(!f.0.path().join("apps").exists());
}

#[cfg(unix)]
#[test]
fn setup_preview_apply_is_one_revision_and_stale_or_bad_last_row_never_partially_writes() {
    let f = F::new();
    let secret = f.json(
        &[
            "secrets",
            "create",
            "--kind",
            "api-key",
            "--label",
            "Synthetic",
            "--value-stdin",
        ],
        Some(VALUE.as_bytes()),
    );
    let id = secret["items"][0]["id"].as_str().unwrap();
    let a = f.json(
        &[
            "ai",
            "connections",
            "add",
            "--label",
            "A",
            "--endpoint",
            "http://127.0.0.1:32123/v1",
            "--api-kind",
            "openai-chat-completions",
            "--default-model",
            "synthetic",
            "--secret-ref",
            id,
        ],
        None,
    );
    let (cid, _) = a["items"].as_object().unwrap().iter().next().unwrap();
    let args = [
        "ai",
        "setup",
        "--agent",
        "pi",
        "--agent",
        "qwen",
        "--mode",
        "connection",
        "--connection",
        cid,
    ];
    let ai = f.0.path().join("secrets/ai/index.toml");
    let key = f.0.path().join("secrets/index.toml");
    let before = fs::read(&ai).unwrap();
    let keybefore = fs::read(&key).unwrap();
    let preview = f.json(&args, None);
    assert_eq!(preview["applied"], false);
    assert_eq!(preview["preview"]["rows"].as_array().unwrap().len(), 2);
    assert_eq!(fs::read(&ai).unwrap(), before);
    let token = preview["preview"]["preview_id"].as_str().unwrap();
    let bad = f.run(
        &[
            "ai",
            "setup",
            "--agent",
            "pi",
            "--agent",
            "claude",
            "--mode",
            "connection",
            "--connection",
            cid,
        ],
        None,
    );
    assert!(!bad.status.success());
    assert_eq!(fs::read(&ai).unwrap(), before);
    assert_eq!(fs::read(&key).unwrap(), keybefore);
    let mut apply = args.to_vec();
    apply.extend(["--apply-preview", token]);
    let result = f.json(&apply, None);
    assert_eq!(result["applied"], true);
    assert_eq!(
        result["store_revision"].as_u64().unwrap(),
        preview["store_revision"].as_u64().unwrap() + 1
    );
    for harness in ["pi", "qwen"] {
        assert_eq!(
            f.json(&["ai", "bindings", "show", harness], None)["binding"]["connection"],
            cid.as_str()
        );
    }
    assert_eq!(
        f.json(&["ai", "bindings", "show", "claude"], None)["binding"]["mode"],
        "native"
    );
    let after = fs::read(&ai).unwrap();
    assert!(!f.run(&apply, None).status.success());
    assert_eq!(fs::read(&ai).unwrap(), after);
    let fresh = f.json(&args, None);
    let mut noop = args.to_vec();
    noop.extend([
        "--apply-preview",
        fresh["preview"]["preview_id"].as_str().unwrap(),
    ]);
    let result = f.json(&noop, None);
    assert_eq!(result["store_revision"], fresh["store_revision"]);
    assert_eq!(fs::read(&ai).unwrap(), after);
    assert_eq!(fs::read(&key).unwrap(), keybefore);
    // The captured alias subject cannot silently become a different agent set.
    fs::write(
        f.0.path().join("agents.toml"),
        "[launchers.helper]\nkind='pi'\ncommand=['never-executed']\n",
    )
    .unwrap();
    let request = [
        "ai",
        "setup",
        "--agent",
        "helper",
        "--mode",
        "connection",
        "--connection",
        cid,
    ];
    let shown = f.json(&request, None);
    fs::write(
        f.0.path().join("agents.toml"),
        "[launchers.helper]\nkind='qwen'\ncommand=['never-executed']\n",
    )
    .unwrap();
    let mut changed = request.to_vec();
    changed.extend([
        "--apply-preview",
        shown["preview"]["preview_id"].as_str().unwrap(),
    ]);
    assert!(!f.run(&changed, None).status.success());
    assert_eq!(fs::read(&ai).unwrap(), after);
}

#[cfg(unix)]
#[test]
fn setup_subset_preserves_unselected_explicit_binding_and_stale_default_rejects() {
    let f = F::new();
    let secret = f.json(
        &[
            "secrets",
            "create",
            "--kind",
            "api-key",
            "--label",
            "Synthetic",
            "--value-stdin",
        ],
        Some(VALUE.as_bytes()),
    );
    let id = secret["items"][0]["id"].as_str().unwrap();
    let add = |label: &str| {
        f.json(
            &[
                "ai",
                "connections",
                "add",
                "--label",
                label,
                "--endpoint",
                "http://127.0.0.1:32123/v1",
                "--api-kind",
                "openai-chat-completions",
                "--default-model",
                "synthetic",
                "--secret-ref",
                id,
            ],
            None,
        )
    };
    let a = add("A");
    let aid = a["items"].as_object().unwrap().keys().next().unwrap();
    let b = add("B");
    let bid = b["items"].as_object().unwrap().keys().next().unwrap();
    let version = b["store_revision"].as_u64().unwrap().to_string();
    let explicit = f.json(
        &[
            "ai",
            "bindings",
            "set",
            "qwen",
            "--mode",
            "connection",
            "--connection",
            bid,
            "--expect-store",
            &version,
        ],
        None,
    );
    let version = explicit["binding"]["store_revision"]
        .as_u64()
        .unwrap()
        .to_string();
    let selected = f.json(
        &[
            "ai",
            "connections",
            "default",
            "set",
            aid,
            "--expect-store",
            &version,
        ],
        None,
    );
    let version = selected["store_revision"].as_u64().unwrap().to_string();
    let args = ["ai", "setup", "--agent", "pi", "--mode", "inherit-default"];
    let preview = f.json(&args, None);
    f.json(
        &[
            "ai",
            "connections",
            "default",
            "set",
            bid,
            "--expect-store",
            &version,
        ],
        None,
    );
    let path = f.0.path().join("secrets/ai/index.toml");
    let before = fs::read(&path).unwrap();
    let mut stale = args.to_vec();
    stale.extend([
        "--apply-preview",
        preview["preview"]["preview_id"].as_str().unwrap(),
    ]);
    assert!(!f.run(&stale, None).status.success());
    assert_eq!(fs::read(&path).unwrap(), before);
    let fresh = f.json(&args, None);
    let mut apply = args.to_vec();
    apply.extend([
        "--apply-preview",
        fresh["preview"]["preview_id"].as_str().unwrap(),
    ]);
    f.json(&apply, None);
    let qwen = f.json(&["ai", "bindings", "show", "qwen"], None);
    assert_eq!(qwen["binding"]["mode"], "connection");
    assert_eq!(qwen["binding"]["connection"], bid.as_str());
    assert_eq!(
        f.json(&["ai", "bindings", "show", "pi"], None)["binding"]["connection"],
        bid.as_str()
    );
}
