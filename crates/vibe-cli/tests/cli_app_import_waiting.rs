//! Public waiting responses preserve completed work without claiming terminal success.
#![cfg(target_os = "linux")]
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-HISTORY");
mod common;

use common::app_registry::{AppRegistry, ok};
use serde_json::{Value, json};
use std::fs;
use vibe_wire::generated::application_import::ApplicationImport;
use vibe_wire::generated::shared::{ApplicationImportTarget, ApplicationImportWaitingInput};

const PREFIX: &str = "org.example/a-prefix@1.0.0";
const WAITING: &str = "org.example/z-waiting@1.0.0";
const CANARY: &str = "SYNTHETIC_IMPORT_WAITING_SECRET_0917";
const COPY: &str =
    "[[installer_options.steps]]\nid='place'\ntitle='Place payload'\nkind='copy'\nfrom='payload'\n";

fn old_response() -> Value {
    json!({
        "protocol": "vibe-app-import/1", "home": "/scratch/home", "dry_run": false,
        "installed": [], "skipped": [], "refused": [], "licenses": []
    })
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-WAITING-REPORT"
)]
fn old_response_without_pending_round_trips_without_a_new_member() {
    let old = old_response();
    let decoded: ApplicationImport = serde_json::from_value(old.clone()).unwrap();
    assert!(decoded.pending.is_none());
    assert_eq!(serde_json::to_value(decoded).unwrap(), old);
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-WAITING-REPORT"
)]
fn public_pending_projects_future_members_but_private_fragments_stay_closed() {
    let target = json!({"home":"/scratch/home","operation":"exact-operation","future":true});
    let input = json!({"key":WAITING,"parameters":["token"],"future":true});
    assert!(serde_json::from_value::<ApplicationImportTarget>(target.clone()).is_err());
    assert!(serde_json::from_value::<ApplicationImportWaitingInput>(input.clone()).is_err());
    let mut response = old_response();
    response["pending"] = json!({"target":target,"input":input,"future":true});
    let decoded: ApplicationImport = serde_json::from_value(response.clone()).unwrap();
    let pending = decoded.pending.unwrap();
    assert_eq!(pending.target.operation, "exact-operation");
    assert_eq!(pending.input.key, WAITING);
    assert_eq!(pending.input.parameters, ["token"]);
    response["pending"]["target"]
        .as_object_mut()
        .unwrap()
        .remove("operation");
    assert!(serde_json::from_value::<ApplicationImport>(response.clone()).is_err());
    response["pending"]["target"]["operation"] = json!("exact-operation");
    response["pending"]["input"]["parameters"] = json!([7]);
    assert!(serde_json::from_value::<ApplicationImport>(response).is_err());
}

fn source() -> (AppRegistry, std::path::PathBuf) {
    let source = AppRegistry::new();
    for name in ["a-prefix", "z-waiting"] {
        let configuration = if name == "z-waiting" {
            "[[installer_options.configuration.parameters]]\nid='token'\nname='Token'\ntype='string'\nsecret=true\nrequired=false\ndefault='AUTHORED_PUBLIC_DEFAULT'\n"
        } else {
            "[installer_options.configuration]\n"
        };
        let package = source.package(name, "1.0.0", name, &format!("{COPY}{configuration}"));
        source.script(&package, name, name);
        let setter = format!("token={CANARY}");
        let mut extra = vec!["--offline", "--unattended", "--json"];
        if name == "z-waiting" {
            extra.extend(["--set", &setter]);
        }
        ok(&source.app(&["install", "-g"], &format!("org.example/{name}"), &extra));
    }
    let file = source.root.path().join("full.json");
    ok(&source.vibe(&[
        "app",
        "export",
        "--configuration",
        "--json",
        "--output",
        file.to_str().unwrap(),
    ]));
    let text = fs::read_to_string(&file).unwrap();
    assert!(!text.contains(CANARY));
    let document: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(document["roots"], json!([PREFIX, WAITING]));
    (source, file)
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-WAITING-REPORT"
)]
fn full_import_waits_with_exact_target_and_completed_prefix_in_json() {
    let (source, file) = source();
    let target = AppRegistry::new();
    let result = target.vibe(&[
        "app",
        "import",
        file.to_str().unwrap(),
        "--registry",
        source.registry().to_str().unwrap(),
        "--offline",
        "--unattended",
        "--json",
    ]);
    assert_eq!(
        result.status.code(),
        Some(2),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    let response: Value = serde_json::from_slice(&result.stdout).unwrap();
    let typed: ApplicationImport = serde_json::from_value(response.clone()).unwrap();
    let pending = typed
        .pending
        .expect("waiting must be a typed import response");
    assert_eq!(
        pending.target.home,
        target
            .settings()
            .canonicalize()
            .unwrap()
            .display()
            .to_string()
    );
    assert!(!pending.target.operation.is_empty());
    assert_eq!(pending.input.key, WAITING);
    assert_eq!(pending.input.parameters, ["token"]);
    assert_eq!(response["installed"], json!([PREFIX]));
    assert_eq!(response["skipped"], json!([]));
    assert_eq!(response["refused"], json!([]));
    assert_ne!(response["ok"], true);
    assert!(!String::from_utf8_lossy(&result.stdout).contains(CANARY));
    assert!(!String::from_utf8_lossy(&result.stderr).contains(CANARY));
    assert!(target.index()["installations"].get(PREFIX).is_some());
    assert!(target.index()["installations"].get(WAITING).is_none());
    let repeated = target.vibe(&[
        "app",
        "import",
        "--resume",
        &pending.target.operation,
        "--offline",
        "--unattended",
        "--json",
    ]);
    assert_eq!(
        repeated.status.code(),
        Some(2),
        "{}",
        String::from_utf8_lossy(&repeated.stderr)
    );
    let resumed: Value = serde_json::from_slice(&repeated.stdout).unwrap();
    assert_eq!(resumed["pending"], response["pending"]);
    assert_eq!(resumed["installed"], json!([PREFIX]));
    assert_eq!(resumed["refused"], json!([]));
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-WAITING-REPORT"
)]
fn human_waiting_names_the_exact_resume_operation_and_required_ids() {
    let (source, file) = source();
    let target = AppRegistry::new();
    let result = target.vibe(&[
        "app",
        "import",
        file.to_str().unwrap(),
        "--registry",
        source.registry().to_str().unwrap(),
        "--offline",
        "--unattended",
    ]);
    assert_eq!(
        result.status.code(),
        Some(2),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let text = String::from_utf8(result.stdout).unwrap();
    assert!(text.contains(PREFIX), "{text}");
    assert!(text.contains(WAITING), "{text}");
    assert!(text.contains("token"), "{text}");
    assert!(text.contains("vibe app import --resume "), "{text}");
    assert!(text.contains("--secret-input <file>"), "{text}");
    assert!(!text.contains(CANARY));
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-PREFLIGHT")]
fn full_dry_run_reports_missing_input_and_matching_rows_without_creating_work() {
    let (source, file) = source();
    let target = AppRegistry::new();
    let preview = target.vibe(&[
        "app",
        "import",
        file.to_str().unwrap(),
        "--registry",
        source.registry().to_str().unwrap(),
        "--dry-run",
        "--offline",
        "--unattended",
        "--json",
    ]);
    assert_eq!(
        preview.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&preview.stderr)
    );
    let view: Value = serde_json::from_slice(&preview.stdout).unwrap();
    assert_eq!(view["installed"], json!([PREFIX]));
    assert_eq!(view["refused"][0]["key"], WAITING);
    assert!(
        view.get("pending").is_none(),
        "a preview has no operation target"
    );
    assert!(!target.settings().join("cache/app-work").exists());
    assert!(!target.settings().join("applications/index.json").exists());
    let before = fs::read(source.settings().join("applications/index.json")).unwrap();
    let same = source.vibe(&[
        "app",
        "import",
        file.to_str().unwrap(),
        "--registry",
        source.registry().to_str().unwrap(),
        "--dry-run",
        "--offline",
        "--unattended",
        "--json",
    ]);
    ok(&same);
    let view: Value = serde_json::from_slice(&same.stdout).unwrap();
    assert_eq!(view["installed"], json!([]));
    assert_eq!(view["skipped"].as_array().unwrap().len(), 2);
    assert_eq!(view["refused"], json!([]));
    assert_eq!(
        fs::read(source.settings().join("applications/index.json")).unwrap(),
        before
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-SECRET-INPUT")]
fn future_optional_secret_offer_survives_two_waits_without_cross_key_broadcast() {
    let source = AppRegistry::new();
    const FIRST: &str = "org.example/a-first@1.0.0";
    const LATER: &str = "org.example/z-later@1.0.0";
    for name in ["a-first", "z-later"] {
        let package = source.package(name, "1.0.0", name, &format!("{COPY}[[installer_options.configuration.parameters]]\nid='token'\nname='Token'\ntype='string'\nsecret=true\nrequired=false\ndefault='AUTHORED_DEFAULT'\n"));
        source.script(&package, name, name);
        let mut flags = vec!["--offline", "--unattended", "--json"];
        if name == "a-first" {
            flags.extend(["--set", "token=SOURCE_EXPLICIT"]);
        }
        ok(&source.app(&["install", "-g"], &format!("org.example/{name}"), &flags));
    }
    let portable = source.root.path().join("full.json");
    ok(&source.vibe(&[
        "app",
        "export",
        "--configuration",
        "--json",
        "--output",
        portable.to_str().unwrap(),
    ]));
    let target = AppRegistry::new();
    let input = target.root.path().join("secrets.json");
    let write = |key: &str| {
        fs::write(&input, serde_json::to_vec(&json!({"protocol":"vibe-app-import-secrets/1","applications":{(key):{"token":CANARY}}})).unwrap()).unwrap()
    };
    write(LATER);
    let first = target.vibe(&[
        "app",
        "import",
        portable.to_str().unwrap(),
        "--registry",
        source.registry().to_str().unwrap(),
        "--secret-input",
        input.to_str().unwrap(),
        "--offline",
        "--unattended",
        "--json",
    ]);
    assert_eq!(
        first.status.code(),
        Some(2),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let first: Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(first["pending"]["input"]["key"], FIRST);
    let id = first["pending"]["target"]["operation"]
        .as_str()
        .unwrap()
        .to_owned();
    fs::remove_file(&portable).unwrap();
    write(FIRST);
    let second = target.vibe(&[
        "app",
        "import",
        "--resume",
        &id,
        "--secret-input",
        input.to_str().unwrap(),
        "--offline",
        "--unattended",
        "--json",
    ]);
    assert_eq!(
        second.status.code(),
        Some(2),
        "{}",
        String::from_utf8_lossy(&second.stderr)
    );
    let second: Value = serde_json::from_slice(&second.stdout).unwrap();
    assert_eq!(second["installed"], json!([FIRST]));
    assert_eq!(
        second["pending"]["input"]["key"], LATER,
        "lost future explicit input must not become authored default, nor receive FIRST's same-id answer"
    );
    let work = target
        .settings()
        .join("cache/app-work/imports")
        .join(&id)
        .join("import.json");
    let saved = fs::read_to_string(&work).unwrap();
    assert!(!saved.contains(CANARY));
    write(LATER);
    let third = target.vibe(&[
        "app",
        "import",
        "--resume",
        &id,
        "--secret-input",
        input.to_str().unwrap(),
        "--offline",
        "--unattended",
        "--json",
    ]);
    ok(&third);
    let third: Value = serde_json::from_slice(&third.stdout).unwrap();
    assert_eq!(third["installed"], json!([FIRST, LATER]));
    assert!(third.get("pending").is_none());
    assert!(!work.exists());
    assert!(
        !serde_json::to_string(&target.index())
            .unwrap()
            .contains(CANARY)
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PORTABLE-WAITING-REPORT"
)]
fn secret_file_locations_and_malformed_values_never_enter_public_errors() {
    let (source, portable) = source();
    let target = AppRegistry::new();
    let missing = target.root.path().join("SECRET_FILENAME_CANARY_90214.json");
    let check = |output: &std::process::Output| {
        assert!(!output.status.success());
        let text = format!(
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!text.contains("SECRET_FILENAME_CANARY_90214"), "{text}");
        assert!(!text.contains(CANARY), "{text}");
    };
    let first = target.vibe(&[
        "app",
        "import",
        portable.to_str().unwrap(),
        "--registry",
        source.registry().to_str().unwrap(),
        "--secret-input",
        missing.to_str().unwrap(),
        "--offline",
        "--unattended",
        "--json",
    ]);
    check(&first);
    assert!(!target.settings().join("cache/app-work").exists());
    let waiting = target.vibe(&[
        "app",
        "import",
        portable.to_str().unwrap(),
        "--registry",
        source.registry().to_str().unwrap(),
        "--offline",
        "--unattended",
        "--json",
    ]);
    assert_eq!(waiting.status.code(), Some(2));
    let waiting: Value = serde_json::from_slice(&waiting.stdout).unwrap();
    let id = waiting["pending"]["target"]["operation"].as_str().unwrap();
    let state_path = target
        .settings()
        .join("cache/app-work/imports")
        .join(id)
        .join("import.json");
    let before = fs::read(&state_path).unwrap();
    let resume = || {
        target.vibe(&[
            "app",
            "import",
            "--resume",
            id,
            "--secret-input",
            missing.to_str().unwrap(),
            "--offline",
            "--unattended",
            "--json",
        ])
    };
    check(&resume());
    assert_eq!(fs::read(&state_path).unwrap(), before);
    fs::write(&missing,format!("{{\"protocol\":\"vibe-app-import-secrets/1\",\"applications\":{{\"{WAITING}\":{{\"token\":{{\"{CANARY}\":true}}}}}}}}" )).unwrap();
    check(&resume());
    assert_eq!(fs::read(&state_path).unwrap(), before);
}
