//! Focused native package-provider adapter tests.

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use vibe_core::manifest::{
    ArtifactInput, ArtifactKind, ArtifactOutput, ArtifactPackageTarget, MechanismRoutes,
};
use vibe_extension_registry::SelectionStep;
use vibe_extension_registry::resolve_mechanism;
use vibe_wire::behaviour::native_package::{self, RetainedPackage};
use vibe_wire::generated::artifact_record::ArtifactShape;
use vibe_wire::generated::native::e1::{package_reply as reply, package_request as request};
use vibe_wire::generated::shared::NativeDeployArtifactKind as WireKind;

use crate::mechanism::contain::{digest_file, tree_digest};
use crate::mechanism::record::{
    RecordFreshness, RecordInputs, build_record, read_record, write_record,
};
use crate::native::{
    NativeArtifactOrigin, NativeArtifactRecordRoot, NativeMechanismBinding, NativePlatform,
    PreparedNativeMechanism, PreparedNativeMechanisms,
};

use super::native::{NativePackageCalls, execute_with};
use super::support::{config, empty_world, execution, key, pin, registry, temp, write};
use super::{PackageTargetRequest, execute_prepared_package_targets, inputs::resolve_inputs};

const PIN: &str = "org.example/packagers#skill-v2";

fn must<T, E: std::fmt::Debug>(value: Result<T, E>, context: &str) -> T {
    match value {
        Ok(value) => value,
        Err(error) => panic!("{context}: {error:?}"),
    }
}

fn must_some<T>(value: Option<T>, context: &str) -> T {
    match value {
        Some(value) => value,
        None => panic!("{context}"),
    }
}

fn target(directory_root: bool) -> ArtifactPackageTarget {
    ArtifactPackageTarget {
        id: "native-package".to_owned(),
        mechanism: key("package:static-skill"),
        provider: Some(pin(PIN)),
        when: None,
        inputs: Some(vec![ArtifactInput::Artifact {
            artifact: "built-input".to_owned(),
        }]),
        outputs: if directory_root {
            vec![ArtifactOutput {
                id: "tree-out".to_owned(),
                kind: ArtifactKind::Directory,
                select: None,
            }]
        } else {
            vec![
                ArtifactOutput {
                    id: "file-out".to_owned(),
                    kind: ArtifactKind::File,
                    select: Some(config("name='file'")),
                },
                ArtifactOutput {
                    id: "tree-out".to_owned(),
                    kind: ArtifactKind::Directory,
                    select: None,
                },
            ]
        },
        config: Some(config("mode='native'")),
    }
}

fn binding(pin: &str) -> NativeMechanismBinding {
    NativeMechanismBinding {
        target: "native-package".to_owned(),
        key: key("package:static-skill"),
        pin: pin.to_owned(),
        descriptor_id: "skill-v2".to_owned(),
        protocol: 1,
        via: SelectionStep::TargetPin,
        displaced_default: Some("org.vibevm/vibe#static-skill".to_owned()),
    }
}

fn entry(pin: &str) -> PreparedNativeMechanism {
    PreparedNativeMechanism {
        bindings: vec![binding(pin)],
        provider: "org.example/packagers".to_owned(),
        provider_version: "1.2.3".to_owned(),
        provider_hash: Some(format!("sha256:{}", "d".repeat(64))),
        provider_root: PathBuf::from("C:/provider"),
        record_root: NativeArtifactRecordRoot::Project,
        platform: must(NativePlatform::current(), "current platform"),
        origin: NativeArtifactOrigin::Prebuilt,
        record: None,
        image: PathBuf::from("C:/provider/package.dll"),
        digest: "a".repeat(64),
        bytes: 1,
    }
}

fn write_build_record(root: &Path) {
    write(root, "target/built-input.bin", "built bytes");
    let path = root.join("target/built-input.bin");
    let (digest, _) = must(digest_file(&path), "input digest");
    let mechanism = key("build:cargo");
    let config = "c".repeat(64);
    let toolchain = "b".repeat(64);
    let absolute = vibe_core::machine_json_path(&path);
    let record = must(
        build_record(&RecordInputs {
            target: "build-input",
            mechanism: &mechanism,
            provider_key: "org.vibevm/vibe#cargo",
            provider_version: None,
            provider_hash: None,
            output_id: "built-input",
            kind: ArtifactKind::Executable,
            shape: ArtifactShape::File,
            digest: &digest,
            path_absolute: &absolute,
            path_relative: "target/built-input.bin",
            freshness: RecordFreshness {
                inputs: None,
                config: Some(&config),
                toolchain: Some(&toolchain),
            },
            platform: None,
            media_type: None,
            created_at: "2026-09-08T00:00:00Z",
            evidence: "fixture build".to_owned(),
        }),
        "build record",
    );
    must(write_record(root, &record), "write build record");
}

fn domain<'a>(
    root: &'a Path,
    target: &'a ArtifactPackageTarget,
    inputs: &'a [super::protocol::ResolvedInput],
) -> PackageTargetRequest<'a> {
    PackageTargetRequest {
        target,
        project_root: root,
        package_root: super::PackageExecution::default_package_root(),
        inputs,
    }
}

#[derive(Debug, Clone, Copy)]
enum Mode {
    Happy,
    DirectoryRoot,
    Fail(&'static str),
    Fault(&'static str),
    Mismatch,
    WrongKind,
    WrongMedia,
    WrongPath,
    WrongBytes,
    WrongFiles,
    Extra,
    Overlap,
}

struct FakeCalls {
    root: PathBuf,
    mode: Mode,
    kinds: Vec<WireKind>,
    calls: RefCell<Vec<&'static str>>,
    plan_input: RefCell<Option<request::ResolvedInput>>,
}

impl FakeCalls {
    fn new(root: &Path, mode: Mode) -> Self {
        Self {
            root: root.to_path_buf(),
            mode,
            kinds: vec![WireKind::File, WireKind::Directory],
            calls: RefCell::new(Vec::new()),
            plan_input: RefCell::new(None),
        }
    }

    fn response(&self, operation: &'static str, result: Value) -> reply::PackageReply {
        must(
            serde_json::from_value(
                json!({"operation":operation,"envelope":1,"protocol":1,"result":result}),
            ),
            "typed package reply",
        )
    }

    fn fail(&self, operation: &'static str) -> reply::PackageReply {
        self.response(
            operation,
            json!({"status":"fail","message":"fixture refusal"}),
        )
    }

    fn outputs(&self) -> Value {
        match self.mode {
            Mode::DirectoryRoot => json!([
                {"id":"tree-out","kind":"directory","shape":"directory","path_relative":"."}
            ]),
            Mode::Overlap => json!([
                {"id":"file-out","kind":"file","shape":"file","path_relative":"tree/file.bin","media_type":"text/plain"},
                {"id":"tree-out","kind":"directory","shape":"directory","path_relative":"tree"}
            ]),
            Mode::WrongKind => json!([
                {"id":"file-out","kind":"archive","shape":"file","path_relative":"file.bin","media_type":"text/plain"},
                {"id":"tree-out","kind":"directory","shape":"directory","path_relative":"tree"}
            ]),
            Mode::WrongMedia => json!([
                {"id":"file-out","kind":"file","shape":"file","path_relative":"file.bin","media_type":"text/plain"},
                {"id":"tree-out","kind":"directory","shape":"directory","path_relative":"tree","media_type":"text/plain"}
            ]),
            _ => json!([
                {"id":"file-out","kind":"file","shape":"file","path_relative":"file.bin","media_type":"text/plain"},
                {"id":"tree-out","kind":"directory","shape":"directory","path_relative":"tree"}
            ]),
        }
    }

    fn write_outputs(&self) {
        let stage = self.root.join("target/vibe-package/native-package");
        if matches!(self.mode, Mode::DirectoryRoot) {
            write(
                &self.root,
                "target/vibe-package/native-package/inside.txt",
                "tree",
            );
        } else {
            write(
                &self.root,
                "target/vibe-package/native-package/file.bin",
                "file",
            );
            write(
                &self.root,
                "target/vibe-package/native-package/tree/inside.txt",
                "tree",
            );
        }
        if matches!(self.mode, Mode::Extra) {
            must(
                std::fs::write(stage.join("extra.bin"), b"extra"),
                "extra output",
            );
        }
    }
}

impl NativePackageCalls for FakeCalls {
    fn kinds(&self) -> &[WireKind] {
        &self.kinds
    }

    fn invoke(
        &self,
        _target: &str,
        value: &request::PackageRequest,
        retained: RetainedPackage<'_>,
    ) -> Result<reply::PackageReply, String> {
        let operation = must(
            native_package::validate_request(
                value,
                PIN,
                "package:static-skill",
                "native-package",
                retained,
            ),
            "adapter request",
        );
        let operation = match operation {
            native_package::PackageOperation::Plan => "plan",
            native_package::PackageOperation::Fingerprint => "fingerprint",
            native_package::PackageOperation::Apply => "apply",
            native_package::PackageOperation::Verify => "verify",
        };
        self.calls.borrow_mut().push(operation);
        if let request::PackageRequest::Plan(plan) = value {
            self.plan_input.replace(plan.inputs.first().cloned());
        }
        if matches!(self.mode, Mode::Fault(fault) if fault == operation) {
            return Err("malformed loader response".to_owned());
        }
        if matches!(self.mode, Mode::Fail(fail) if fail == operation) {
            return Ok(self.fail(operation));
        }
        let reply = match operation {
            "plan" => self.response(
                "plan",
                json!({"status":"ok","plan":{"summary":"native plan","outputs":self.outputs()}}),
            ),
            "fingerprint" => self.response(
                "fingerprint",
                json!({"status":"ok","fingerprint":{"digest":"b".repeat(64),"counted_inputs":"1"}}),
            ),
            "apply" => {
                self.write_outputs();
                let staged = if matches!(self.mode, Mode::DirectoryRoot) {
                    json!([{"id":"tree-out","path_relative":"."}])
                } else if matches!(self.mode, Mode::WrongPath) {
                    json!([{"id":"file-out","path_relative":"other.bin"},{"id":"tree-out","path_relative":"tree"}])
                } else {
                    json!([{"id":"file-out","path_relative":"file.bin"},{"id":"tree-out","path_relative":"tree"}])
                };
                self.response(
                    "apply",
                    json!({"status":"ok","staged":staged,"evidence":"native apply"}),
                )
            }
            "verify" => {
                let stage = self.root.join("target/vibe-package/native-package");
                let verified = if matches!(self.mode, Mode::DirectoryRoot) {
                    let tree = must(tree_digest(&stage), "root tree digest");
                    json!([{"id":"tree-out","path_relative":".","digest":tree.digest,"bytes":tree.bytes.to_string(),"files":tree.files.to_string()}])
                } else {
                    let (digest, bytes) = must(digest_file(&stage.join("file.bin")), "file digest");
                    let tree = must(tree_digest(&stage.join("tree")), "tree digest");
                    let digest = if matches!(self.mode, Mode::Mismatch) {
                        "c".repeat(64)
                    } else {
                        digest
                    };
                    let file_bytes = if matches!(self.mode, Mode::WrongBytes) {
                        bytes + 1
                    } else {
                        bytes
                    };
                    let tree_files = if matches!(self.mode, Mode::WrongFiles) {
                        tree.files + 1
                    } else {
                        tree.files
                    };
                    json!([
                        {"id":"file-out","path_relative":"file.bin","digest":digest,"bytes":file_bytes.to_string(),"files":"1"},
                        {"id":"tree-out","path_relative":"tree","digest":tree.digest,"bytes":tree.bytes.to_string(),"files":tree_files.to_string()}
                    ])
                };
                self.response(
                    "verify",
                    json!({"status":"ok","verified":verified,"evidence":"native verify"}),
                )
            }
            _ => unreachable!(),
        };
        native_package::validate_exchange(
            value,
            PIN,
            "package:static-skill",
            "native-package",
            retained,
            &reply,
        )
        .map_err(|error| error.to_string())?;
        Ok(reply)
    }
}

fn run(
    root: &Path,
    target: &ArtifactPackageTarget,
    calls: &FakeCalls,
) -> Result<Vec<super::PackagedArtifact>, super::PackageError> {
    let world = empty_world();
    let plane = registry(&world);
    let routes = MechanismRoutes::default();
    let inputs = resolve_inputs(root, target)?;
    execute_with(
        &execution(root, std::slice::from_ref(target), &plane, &routes),
        &domain(root, target, &inputs),
        &entry(PIN),
        &binding(PIN),
        calls,
    )
}

#[test]
fn exact_recorded_input_and_four_operation_chain_produce_verified_records() {
    let root = temp();
    write_build_record(root.path());
    let target = target(false);
    let calls = FakeCalls::new(root.path(), Mode::Happy);
    let produced = must(run(root.path(), &target, &calls), "native package");
    assert_eq!(
        *calls.calls.borrow(),
        ["plan", "fingerprint", "apply", "verify"]
    );
    let input = must_some(calls.plan_input.borrow().clone(), "plan input");
    assert_eq!(input.name, "built-input");
    assert_eq!(input.reference, "artifact:built-input");
    assert_eq!(input.path_relative, "target/built-input.bin");
    assert_eq!(input.bytes, "11");
    let source_record = must_some(
        must(read_record(root.path(), "built-input"), "source record"),
        "source record exists",
    );
    assert_eq!(input.digest, source_record.digest.value);
    assert!(
        matches!(input.origin, request::InputOrigin::ArtifactRecord(origin) if origin.recorded_kind == WireKind::Executable)
    );
    assert_eq!(produced.len(), 2);
    let file = must_some(
        must(read_record(root.path(), "file-out"), "file record"),
        "file record exists",
    );
    let tree = must_some(
        must(read_record(root.path(), "tree-out"), "tree record"),
        "tree record exists",
    );
    assert_eq!(file.producer.provider.key, PIN);
    assert_eq!(file.producer.provider.version.as_deref(), Some("1.2.3"));
    assert_eq!(
        file.producer.provider.content_hash.as_deref(),
        Some(format!("sha256:{}", "d".repeat(64)).as_str())
    );
    assert_eq!(file.platform, None);
    assert_eq!(file.media_type.as_deref(), Some("text/plain"));
    assert_eq!(tree.media_type, None);
    assert_eq!(file.freshness.inputs.as_ref().map(String::len), Some(64));
    assert_eq!(file.freshness.config.as_ref().map(String::len), Some(64));
    assert_eq!(
        file.freshness.toolchain.as_deref(),
        Some("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
    );
    assert!(
        tree.verification
            .evidence
            .unwrap_or_default()
            .contains("sha256-tree/1")
    );
    let evidence = file.verification.evidence.unwrap_or_default();
    assert!(evidence.contains(&"a".repeat(64)));
    assert!(evidence.contains(entry(PIN).platform.key()));
}

#[test]
fn native_outcome_retains_exact_selected_route_and_displaced_default() {
    let target = target(false);
    let world = super::support::world_with_plugin();
    let plane = registry(&world);
    let routes = MechanismRoutes::default();
    let selection = must(
        resolve_mechanism(&plane, &target.mechanism, target.provider.as_ref(), &routes),
        "native package selection",
    );
    let outcome = super::native_outcome(&target, &selection, &binding(PIN), Vec::new());
    assert_eq!(outcome.provider, PIN);
    assert_eq!(outcome.mechanism, "package:static-skill");
    assert_eq!(outcome.via, SelectionStep::TargetPin.to_string());
    assert_eq!(
        outcome.displaced_default.as_deref(),
        Some("org.vibevm/vibe#static-skill")
    );
}

#[test]
fn directory_dot_is_the_exact_output_root() {
    let root = temp();
    write_build_record(root.path());
    let target = target(true);
    let calls = FakeCalls::new(root.path(), Mode::DirectoryRoot);
    let produced = must(run(root.path(), &target, &calls), "directory package");
    assert_eq!(
        produced[0].path_relative,
        "target/vibe-package/native-package"
    );
    assert_eq!(produced[0].files, 1);
}

#[test]
fn provider_faults_claim_mismatch_extra_and_overlap_write_no_records() {
    for mode in [
        Mode::Fail("plan"),
        Mode::Fail("fingerprint"),
        Mode::Fail("apply"),
        Mode::Fail("verify"),
        Mode::Fault("plan"),
        Mode::Mismatch,
        Mode::WrongKind,
        Mode::WrongMedia,
        Mode::WrongPath,
        Mode::WrongBytes,
        Mode::WrongFiles,
        Mode::Extra,
        Mode::Overlap,
    ] {
        let root = temp();
        write_build_record(root.path());
        let target = target(false);
        let calls = FakeCalls::new(root.path(), mode);
        let error = match run(root.path(), &target, &calls) {
            Err(error) => error,
            Ok(value) => panic!("{mode:?} unexpectedly accepted: {value:?}"),
        };
        if matches!(mode, Mode::Overlap) {
            assert!(error.to_string().contains("overlap"));
            assert_eq!(calls.calls.borrow().as_slice(), &["plan"]);
        }
        if matches!(mode, Mode::Extra) {
            assert!(error.to_string().contains("no planned output owns"));
        }
        assert!(
            !root
                .path()
                .join(".vibe/state/artifacts/file-out.json")
                .exists()
        );
        assert!(
            !root
                .path()
                .join(".vibe/state/artifacts/tree-out.json")
                .exists()
        );
    }
}

#[test]
fn unsupported_kind_and_missing_or_stale_binding_never_reset_or_fallback() {
    let root = temp();
    write_build_record(root.path());
    let target = target(false);
    let inputs = must(resolve_inputs(root.path(), &target), "resolved inputs");
    let world = empty_world();
    let plane = registry(&world);
    let routes = MechanismRoutes::default();
    let run = execution(root.path(), std::slice::from_ref(&target), &plane, &routes);
    let mut calls = FakeCalls::new(root.path(), Mode::Happy);
    calls.kinds = vec![WireKind::File];
    assert!(
        execute_with(
            &run,
            &domain(root.path(), &target, &inputs),
            &entry(PIN),
            &binding(PIN),
            &calls
        )
        .is_err()
    );
    assert!(calls.calls.borrow().is_empty());

    let selected_world = super::support::world_with_plugin();
    let selected_registry = registry(&selected_world);
    let selected = execution(
        root.path(),
        std::slice::from_ref(&target),
        &selected_registry,
        &routes,
    );
    assert!(
        execute_prepared_package_targets(&selected, &PreparedNativeMechanisms::default()).is_err()
    );
    let stale = PreparedNativeMechanisms {
        entries: vec![entry("org.example/packagers#stale")],
    };
    assert!(execute_prepared_package_targets(&selected, &stale).is_err());
    let mut wrong_descriptor = entry(PIN);
    wrong_descriptor.bindings[0].descriptor_id = "other".to_owned();
    assert!(
        execute_prepared_package_targets(
            &selected,
            &PreparedNativeMechanisms {
                entries: vec![wrong_descriptor],
            },
        )
        .is_err()
    );
    let duplicate = PreparedNativeMechanisms {
        entries: vec![entry(PIN), entry(PIN)],
    };
    assert!(execute_prepared_package_targets(&selected, &duplicate).is_err());
    let mut wrong_role = entry(PIN);
    wrong_role.bindings[0].key = key("build:cargo");
    assert!(
        execute_prepared_package_targets(
            &selected,
            &PreparedNativeMechanisms {
                entries: vec![wrong_role],
            },
        )
        .is_err()
    );
    assert!(!root.path().join("target/vibe-package").exists());
}
