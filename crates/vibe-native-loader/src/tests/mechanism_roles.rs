use super::*;
use vibe_wire::behaviour::{native_build, native_package};
use vibe_wire::generated::native::e1::{
    build_reply::BuildReply, build_request::BuildRequest, package_reply::PackageReply,
    package_request::PackageRequest,
};

const PROVIDER: &str = "org.example/plugin#selected";
const BUILD_KEY: &str = "build:cargo";
const PACKAGE_KEY: &str = "package:archive";

fn must_ok<T, E: std::fmt::Debug>(result: Result<T, E>, context: &str) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("{context}: {error:?}"),
    }
}

fn must_object(value: &mut serde_json::Value) -> &mut serde_json::Map<String, serde_json::Value> {
    match value {
        serde_json::Value::Object(object) => object,
        other => panic!("fixture root must be an object, got {other:?}"),
    }
}

fn digest() -> String {
    "a".repeat(64)
}

fn build_target() -> serde_json::Value {
    json!({"id":"demo-build","workdir":".","outputs":[{"id":"demo","kind":"executable"}]})
}

fn build_authority() -> serde_json::Value {
    json!({"project_root_absolute":"C:/work","build_root_absolute":"C:/work/target/build","build_root_relative":"target/build","offline":true})
}

fn build_plan() -> serde_json::Value {
    json!({"summary":"build demo","outputs":[{"id":"demo","kind":"executable","shape":"file","path_relative":"demo"}]})
}

fn build_fingerprint() -> serde_json::Value {
    json!({"digest":digest(),"summary":"exact"})
}

fn build_staging() -> serde_json::Value {
    json!({"root_absolute":"C:/work/target/build/stage","root_relative":"target/build/stage"})
}

fn build_staged() -> serde_json::Value {
    json!([{"id":"demo","kind":"executable","shape":"file","path_relative":"demo","fresh":false}])
}

fn build_request(operation: &str, mechanism: &str, provider: &str) -> BuildRequest {
    let tail = match operation {
        "plan" => json!({}),
        "fingerprint" => json!({"plan":build_plan()}),
        "apply" => {
            json!({"plan":build_plan(),"fingerprint":build_fingerprint(),"staging":build_staging()})
        }
        "verify" => {
            json!({"plan":build_plan(),"fingerprint":build_fingerprint(),"staging":build_staging(),"staged":build_staged()})
        }
        _ => unreachable!(),
    };
    let mut value = tail;
    let object = must_object(&mut value);
    object.insert("operation".into(), operation.into());
    object.insert("envelope".into(), 1.into());
    object.insert("protocol".into(), 1.into());
    object.insert(
        "identity".into(),
        json!({"provider":provider,"mechanism":mechanism,"target":"demo-build"}),
    );
    object.insert("target".into(), build_target());
    object.insert("authority".into(), build_authority());
    must_ok(serde_json::from_value(value), "build request fixture")
}

fn build_reply(operation: &str) -> Vec<u8> {
    let result = match operation {
        "plan" => json!({"status":"ok","plan":build_plan()}),
        "fingerprint" => json!({"status":"ok","fingerprint":build_fingerprint()}),
        "apply" => {
            json!({"status":"ok","staged":[{"id":"demo","path_relative":"demo","fresh":false}],"evidence":"built"})
        }
        "verify" => {
            json!({"status":"ok","verified":[{"id":"demo","path_relative":"demo","digest":digest(),"bytes":"7"}],"evidence":"verified"})
        }
        _ => unreachable!(),
    };
    must_ok(
        serde_json::to_vec(
            &json!({"operation":operation,"envelope":1,"protocol":1,"result":result}),
        ),
        "build reply fixture",
    )
}

struct BuildAccepted {
    target: vibe_wire::generated::native::e1::build_request::BuildTarget,
    authority: vibe_wire::generated::native::e1::build_request::BuildAuthority,
    plan: vibe_wire::generated::native::e1::build_request::BuildPlan,
    fingerprint: vibe_wire::generated::native::e1::build_request::BuildFingerprint,
    staging: vibe_wire::generated::native::e1::build_request::StagingAuthority,
    staged: Vec<vibe_wire::generated::native::e1::build_request::StagedOutput>,
}

fn build_accepted() -> BuildAccepted {
    BuildAccepted {
        target: must_ok(
            serde_json::from_value(build_target()),
            "build target fixture",
        ),
        authority: must_ok(
            serde_json::from_value(build_authority()),
            "build authority fixture",
        ),
        plan: must_ok(serde_json::from_value(build_plan()), "build plan fixture"),
        fingerprint: must_ok(
            serde_json::from_value(build_fingerprint()),
            "build fingerprint fixture",
        ),
        staging: must_ok(
            serde_json::from_value(build_staging()),
            "build staging fixture",
        ),
        staged: must_ok(
            serde_json::from_value(build_staged()),
            "build staged fixture",
        ),
    }
}

fn retained_build<'a>(
    operation: &str,
    value: &'a BuildAccepted,
) -> native_build::RetainedBuild<'a> {
    match operation {
        "plan" => native_build::RetainedBuild::default(),
        "fingerprint" => native_build::RetainedBuild {
            target: Some(&value.target),
            authority: Some(&value.authority),
            plan: Some(&value.plan),
            ..native_build::RetainedBuild::default()
        },
        "apply" => native_build::RetainedBuild {
            target: Some(&value.target),
            authority: Some(&value.authority),
            plan: Some(&value.plan),
            fingerprint: Some(&value.fingerprint),
            ..native_build::RetainedBuild::default()
        },
        "verify" => native_build::RetainedBuild {
            target: Some(&value.target),
            authority: Some(&value.authority),
            plan: Some(&value.plan),
            fingerprint: Some(&value.fingerprint),
            staging: Some(&value.staging),
            staged: Some(&value.staged),
        },
        _ => unreachable!(),
    }
}

fn package_target() -> serde_json::Value {
    json!({"id":"bundle","outputs":[{"id":"bundle","kind":"archive"}]})
}

fn package_inputs() -> serde_json::Value {
    json!([{"name":"binary","reference":"artifact:binary","path_absolute":"C:/work/artifacts/bin","path_relative":"artifacts/bin","digest":digest(),"bytes":"7","shape":"file","origin":{"kind":"artifact-record","recorded_kind":"executable"}}])
}

fn package_authority() -> serde_json::Value {
    json!({"project_root_absolute":"C:/work","package_root_absolute":"C:/work/target/packages","package_root_relative":"target/packages","output_root_absolute":"C:/work/target/packages/bundle","output_root_relative":"target/packages/bundle"})
}

fn package_plan() -> serde_json::Value {
    json!({"summary":"package bundle","outputs":[{"id":"bundle","kind":"archive","shape":"file","path_relative":"bundle.tar","media_type":"application/x-tar"}]})
}

fn package_fingerprint() -> serde_json::Value {
    json!({"digest":digest(),"counted_inputs":"1"})
}

fn package_staging() -> serde_json::Value {
    json!({"root_absolute":"C:/work/target/packages/stage","root_relative":"target/packages/stage"})
}

fn package_staged() -> serde_json::Value {
    json!([{"id":"bundle","kind":"archive","shape":"file","path_relative":"bundle.tar","media_type":"application/x-tar"}])
}

fn package_request(operation: &str, mechanism: &str, provider: &str) -> PackageRequest {
    let mut value = match operation {
        "plan" => json!({}),
        "fingerprint" => json!({"plan":package_plan()}),
        "apply" => {
            json!({"plan":package_plan(),"fingerprint":package_fingerprint(),"staging":package_staging()})
        }
        "verify" => {
            json!({"plan":package_plan(),"fingerprint":package_fingerprint(),"staging":package_staging(),"staged":package_staged()})
        }
        _ => unreachable!(),
    };
    let object = must_object(&mut value);
    object.insert("operation".into(), operation.into());
    object.insert("envelope".into(), 1.into());
    object.insert("protocol".into(), 1.into());
    object.insert(
        "identity".into(),
        json!({"provider":provider,"mechanism":mechanism,"target":"bundle"}),
    );
    object.insert("target".into(), package_target());
    object.insert("inputs".into(), package_inputs());
    object.insert("authority".into(), package_authority());
    must_ok(serde_json::from_value(value), "package request fixture")
}

fn package_reply(operation: &str) -> Vec<u8> {
    let result = match operation {
        "plan" => json!({"status":"ok","plan":package_plan()}),
        "fingerprint" => json!({"status":"ok","fingerprint":package_fingerprint()}),
        "apply" => {
            json!({"status":"ok","staged":[{"id":"bundle","path_relative":"bundle.tar"}],"evidence":"packed"})
        }
        "verify" => {
            json!({"status":"ok","verified":[{"id":"bundle","path_relative":"bundle.tar","digest":digest(),"bytes":"7","files":"1"}],"evidence":"verified"})
        }
        _ => unreachable!(),
    };
    must_ok(
        serde_json::to_vec(
            &json!({"operation":operation,"envelope":1,"protocol":1,"result":result}),
        ),
        "package reply fixture",
    )
}

struct PackageAccepted {
    target: vibe_wire::generated::native::e1::package_request::PackageTarget,
    inputs: Vec<vibe_wire::generated::native::e1::package_request::ResolvedInput>,
    authority: vibe_wire::generated::native::e1::package_request::PackageAuthority,
    plan: vibe_wire::generated::native::e1::package_request::PackagePlan,
    fingerprint: vibe_wire::generated::native::e1::package_request::PackageFingerprint,
    staging: vibe_wire::generated::native::e1::package_request::StagingAuthority,
    staged: Vec<vibe_wire::generated::native::e1::package_request::StagedOutput>,
}

fn package_accepted() -> PackageAccepted {
    PackageAccepted {
        target: must_ok(
            serde_json::from_value(package_target()),
            "package target fixture",
        ),
        inputs: must_ok(
            serde_json::from_value(package_inputs()),
            "package inputs fixture",
        ),
        authority: must_ok(
            serde_json::from_value(package_authority()),
            "package authority fixture",
        ),
        plan: must_ok(
            serde_json::from_value(package_plan()),
            "package plan fixture",
        ),
        fingerprint: must_ok(
            serde_json::from_value(package_fingerprint()),
            "package fingerprint fixture",
        ),
        staging: must_ok(
            serde_json::from_value(package_staging()),
            "package staging fixture",
        ),
        staged: must_ok(
            serde_json::from_value(package_staged()),
            "package staged fixture",
        ),
    }
}

fn retained_package<'a>(
    operation: &str,
    value: &'a PackageAccepted,
) -> native_package::RetainedPackage<'a> {
    let prefix = native_package::RetainedPackage {
        target: Some(&value.target),
        inputs: Some(&value.inputs),
        authority: Some(&value.authority),
        ..native_package::RetainedPackage::default()
    };
    match operation {
        "plan" => native_package::RetainedPackage::default(),
        "fingerprint" => native_package::RetainedPackage {
            plan: Some(&value.plan),
            ..prefix
        },
        "apply" => native_package::RetainedPackage {
            plan: Some(&value.plan),
            fingerprint: Some(&value.fingerprint),
            ..prefix
        },
        "verify" => native_package::RetainedPackage {
            plan: Some(&value.plan),
            fingerprint: Some(&value.fingerprint),
            staging: Some(&value.staging),
            staged: Some(&value.staged),
            ..prefix
        },
        _ => unreachable!(),
    }
}

fn loader_for_role(
    role: &str,
    name: &str,
    reply: Vec<u8>,
) -> (TempDir, PathBuf, NativeLoader, Arc<FakeLibrary>) {
    let (directory, path) = fake_file();
    let (loader, library, _) = loader_for(
        FakeManifest::Bytes(role_manifest(
            role,
            name,
            &["plan", "fingerprint", "apply", "verify"],
        )),
        FakeCall::published(0, reply),
    );
    (directory, path, loader, library)
}

#[test]
fn build_and_package_all_four_operations_use_one_strict_call_path() {
    let build_accepted = build_accepted();
    let package_accepted = package_accepted();
    for operation in ["plan", "fingerprint", "apply", "verify"] {
        let (_directory, path, loader, library) =
            loader_for_role("build", "cargo", build_reply(operation));
        let key: MechanismKey = BUILD_KEY.parse().unwrap();
        let handle = loader
            .admit_mechanism(&path, PROVIDER, "selected", &key)
            .unwrap();
        let reply = handle
            .invoke_build(
                "demo-build",
                &build_request(operation, BUILD_KEY, PROVIDER),
                retained_build(operation, &build_accepted),
            )
            .unwrap();
        assert!(matches!(
            (operation, reply),
            ("plan", BuildReply::Plan(_))
                | ("fingerprint", BuildReply::Fingerprint(_))
                | ("apply", BuildReply::Apply(_))
                | ("verify", BuildReply::Verify(_))
        ));
        assert_eq!(library.invoke_count.load(Ordering::SeqCst), 1);
        assert_eq!(library.free_count.load(Ordering::SeqCst), 1);

        let (_directory, path, loader, library) =
            loader_for_role("package", "archive", package_reply(operation));
        let key: MechanismKey = PACKAGE_KEY.parse().unwrap();
        let handle = loader
            .admit_mechanism(&path, PROVIDER, "selected", &key)
            .unwrap();
        let reply = handle
            .invoke_package(
                "bundle",
                &package_request(operation, PACKAGE_KEY, PROVIDER),
                retained_package(operation, &package_accepted),
            )
            .unwrap();
        assert!(matches!(
            (operation, reply),
            ("plan", PackageReply::Plan(_))
                | ("fingerprint", PackageReply::Fingerprint(_))
                | ("apply", PackageReply::Apply(_))
                | ("verify", PackageReply::Verify(_))
        ));
        assert_eq!(library.invoke_count.load(Ordering::SeqCst), 1);
        assert_eq!(library.free_count.load(Ordering::SeqCst), 1);
    }
}

#[test]
fn exact_identity_retained_drift_and_wrong_role_refuse_before_invoke() {
    let accepted = build_accepted();
    for (name, provider, target, retained) in [
        (
            BUILD_KEY,
            "org.example/plugin#other",
            "demo-build",
            retained_build("plan", &accepted),
        ),
        (
            BUILD_KEY,
            PROVIDER,
            "other",
            retained_build("plan", &accepted),
        ),
        (
            BUILD_KEY,
            PROVIDER,
            "demo-build",
            native_build::RetainedBuild::default(),
        ),
    ] {
        let (_directory, path, loader, library) =
            loader_for_role("build", "cargo", build_reply("fingerprint"));
        let key: MechanismKey = BUILD_KEY.parse().unwrap();
        let handle = loader
            .admit_mechanism(&path, PROVIDER, "selected", &key)
            .unwrap();
        let request = build_request("fingerprint", name, provider);
        assert!(handle.invoke_build(target, &request, retained).is_err());
        assert_eq!(library.invoke_count.load(Ordering::SeqCst), 0);
    }

    let (_directory, path, loader, library) =
        loader_for_role("build", "other", build_reply("plan"));
    let other_key: MechanismKey = "build:other".parse().unwrap();
    let handle = loader
        .admit_mechanism(&path, PROVIDER, "selected", &other_key)
        .unwrap();
    assert!(
        handle
            .invoke_build(
                "demo-build",
                &build_request("plan", BUILD_KEY, PROVIDER),
                native_build::RetainedBuild::default(),
            )
            .is_err()
    );
    assert_eq!(library.invoke_count.load(Ordering::SeqCst), 0);

    let (_directory, path) = fake_file();
    let (loader, library, _) = loader_for(
        FakeManifest::Bytes(manifest("selected")),
        FakeCall::published(0, reply()),
    );
    let handle = loader
        .admit_mechanism(&path, PROVIDER, "selected", &key())
        .unwrap();
    assert!(
        handle
            .invoke_build(
                "demo-build",
                &build_request("plan", BUILD_KEY, PROVIDER),
                native_build::RetainedBuild::default(),
            )
            .is_err()
    );
    assert_eq!(library.invoke_count.load(Ordering::SeqCst), 0);
}

#[test]
fn build_and_package_malformed_or_oversize_replies_free_once() {
    let build_accepted = build_accepted();
    let package_accepted = package_accepted();
    for oversized in [false, true] {
        let call = if oversized {
            FakeCall {
                status: 0,
                bytes: Some(vec![1]),
                len: REPLY_CAP + 1,
            }
        } else {
            FakeCall::published(0, b"{".to_vec())
        };
        let (_directory, path) = fake_file();
        let (loader, library, _) = loader_for(
            FakeManifest::Bytes(role_manifest(
                "build",
                "cargo",
                &["plan", "fingerprint", "apply", "verify"],
            )),
            call.clone(),
        );
        let key: MechanismKey = BUILD_KEY.parse().unwrap();
        let handle = loader
            .admit_mechanism(&path, PROVIDER, "selected", &key)
            .unwrap();
        assert!(
            handle
                .invoke_build(
                    "demo-build",
                    &build_request("plan", BUILD_KEY, PROVIDER),
                    retained_build("plan", &build_accepted),
                )
                .is_err()
        );
        assert_eq!(library.free_count.load(Ordering::SeqCst), 1);

        let (loader, library, _) = loader_for(
            FakeManifest::Bytes(role_manifest(
                "package",
                "archive",
                &["plan", "fingerprint", "apply", "verify"],
            )),
            call,
        );
        let key: MechanismKey = PACKAGE_KEY.parse().unwrap();
        let handle = loader
            .admit_mechanism(&path, PROVIDER, "selected", &key)
            .unwrap();
        assert!(
            handle
                .invoke_package(
                    "bundle",
                    &package_request("plan", PACKAGE_KEY, PROVIDER),
                    retained_package("plan", &package_accepted),
                )
                .is_err()
        );
        assert_eq!(library.free_count.load(Ordering::SeqCst), 1);
    }
}
