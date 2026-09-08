use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::json;
use tempfile::TempDir;
use vibe_core::manifest::{
    ArtifactBuildTarget, ArtifactInput, ArtifactKind, ArtifactOutput, ExtensionConfig,
    ExtensionHandler, ExtensionsControl, MechanismDecl, MechanismFreshness, MechanismKey,
    MechanismRole, MechanismRoutes,
};
use vibe_extension_registry::{MechanismRegistry, SelectionStep, collect_mechanisms};
use vibe_wire::behaviour::native_build::RetainedBuild;
use vibe_wire::generated::artifact_record::{ArtifactKind as RecordKind, ArtifactShape};
use vibe_wire::generated::native::e1::{build_reply as reply, build_request as request};
use vibe_wire::generated::shared::NativeDeployArtifactKind as WireKind;

use crate::mechanism::contain::{digest_file, tree_digest};
use crate::native::{
    NativeArtifactOrigin, NativeArtifactRecordRoot, NativeMechanismBinding, NativePlatform,
    PreparedNativeMechanism, PreparedNativeMechanisms,
};
use crate::{ExtensionWorld, HostExtensionSource, HostIdentity, HostProvider};

use super::native::{NativeBuildCalls, execute_with};
use super::record::config_fingerprint;
use super::{BuildExecution, execute_prepared_build_targets};

const PIN: &str = "__host__/demo#builder";

fn must<T, E: std::fmt::Debug>(value: Result<T, E>, context: &str) -> T {
    match value {
        Ok(value) => value,
        Err(error) => panic!("{context}: {error:?}"),
    }
}

fn key() -> MechanismKey {
    must("build:cargo".parse(), "build key")
}

fn config(value: &str) -> ExtensionConfig {
    must(
        value
            .parse::<toml::Table>()
            .map(ExtensionConfig::from_table),
        "fixture config",
    )
}

fn target() -> ArtifactBuildTarget {
    ArtifactBuildTarget {
        id: "native-demo".to_owned(),
        mechanism: key(),
        provider: None,
        workdir: ".".to_owned(),
        inputs: Some(vec![ArtifactInput::Path {
            path: PathBuf::from("src/input.txt"),
        }]),
        outputs: vec![
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
        ],
        config: Some(config("mode='native'")),
    }
}

fn binding(pin: &str) -> NativeMechanismBinding {
    NativeMechanismBinding {
        target: "native-demo".to_owned(),
        key: key(),
        pin: pin.to_owned(),
        descriptor_id: "builder".to_owned(),
        protocol: 1,
        via: SelectionStep::HostRoute,
        displaced_default: Some("org.vibevm/vibe#cargo".to_owned()),
    }
}

fn entry(pin: &str) -> PreparedNativeMechanism {
    PreparedNativeMechanism {
        bindings: vec![binding(pin)],
        provider: "__host__/demo".to_owned(),
        provider_version: "1.2.3".to_owned(),
        provider_hash: Some(format!("sha256:{}", "d".repeat(64))),
        provider_root: PathBuf::from("C:/provider"),
        record_root: NativeArtifactRecordRoot::Project,
        platform: must(NativePlatform::current(), "current platform"),
        origin: NativeArtifactOrigin::Prebuilt,
        record: None,
        image: PathBuf::from("C:/provider/builder.dll"),
        digest: "a".repeat(64),
        bytes: 1,
    }
}

fn registry(root: &Path) -> MechanismRegistry {
    must(
        collect_mechanisms(&ExtensionWorld {
            installed: Vec::new(),
            host: HostExtensionSource {
                provider: HostProvider {
                    identity: HostIdentity::ungrouped_project("demo"),
                    root: root.to_path_buf(),
                    version: "1.2.3".to_owned(),
                    kind: None,
                    content_hash: None,
                },
                declarations: Vec::new(),
                controls: ExtensionsControl::default(),
                mechanisms: vec![MechanismDecl {
                    id: "builder".to_owned(),
                    role: MechanismRole::Build,
                    name: "cargo".to_owned(),
                    handler: ExtensionHandler::Native {
                        crate_dir: None,
                        prebuilt: Some(BTreeMap::new()),
                    },
                    protocol: 1,
                    config_schema: PathBuf::from("schema.jtd.json"),
                    freshness: MechanismFreshness::Provider,
                }],
            },
            effective_stack: None,
        }),
        "mechanism registry",
    )
}

fn execution<'a>(
    root: &'a Path,
    targets: &'a [ArtifactBuildTarget],
    registry: &'a MechanismRegistry,
    routes: &'a MechanismRoutes,
) -> BuildExecution<'a> {
    BuildExecution {
        project_root: root,
        targets,
        registry,
        routes,
        build_root: "target",
        offline: true,
        created_at: "2026-09-08T00:00:00Z",
    }
}

#[derive(Clone, Copy)]
enum Mode {
    Happy,
    Fail(&'static str),
    Fault(&'static str),
    Mismatch,
    Extra,
    Overlap,
}

struct FakeCalls {
    root: PathBuf,
    mode: Mode,
    kinds: Vec<WireKind>,
    calls: RefCell<Vec<&'static str>>,
}

impl FakeCalls {
    fn new(root: &Path, mode: Mode) -> Self {
        Self {
            root: root.to_path_buf(),
            mode,
            kinds: vec![WireKind::File, WireKind::Directory],
            calls: RefCell::new(Vec::new()),
        }
    }

    fn response(&self, operation: &'static str, result: serde_json::Value) -> reply::BuildReply {
        must(
            serde_json::from_value(
                json!({"operation":operation,"envelope":1,"protocol":1,"result":result}),
            ),
            "typed build reply",
        )
    }

    fn fail(&self, operation: &'static str) -> reply::BuildReply {
        self.response(
            operation,
            json!({"status":"fail","message":format!("{operation} refused")}),
        )
    }

    fn write_outputs(&self) {
        let stage = self.root.join("target/vibe-native/native-demo");
        must(
            std::fs::write(stage.join("file.bin"), b"file-bytes"),
            "file output",
        );
        must(std::fs::create_dir_all(stage.join("tree")), "tree output");
        must(
            std::fs::write(stage.join("tree/a.txt"), b"tree-bytes"),
            "tree member",
        );
        if matches!(self.mode, Mode::Extra) {
            must(
                std::fs::write(stage.join("extra.txt"), b"extra"),
                "extra output",
            );
        }
    }
}

impl NativeBuildCalls for FakeCalls {
    fn kinds(&self) -> &[WireKind] {
        &self.kinds
    }

    fn invoke(
        &self,
        _target: &str,
        value: &request::BuildRequest,
        retained: RetainedBuild<'_>,
    ) -> Result<reply::BuildReply, String> {
        let operation = match value {
            request::BuildRequest::Plan(_) => {
                assert!(retained.target.is_none() && retained.plan.is_none());
                "plan"
            }
            request::BuildRequest::Fingerprint(value) => {
                assert_eq!(retained.target, Some(&value.target));
                assert_eq!(retained.plan, Some(&value.plan));
                "fingerprint"
            }
            request::BuildRequest::Apply(value) => {
                assert_eq!(retained.fingerprint, Some(&value.fingerprint));
                assert!(retained.staging.is_none());
                "apply"
            }
            request::BuildRequest::Verify(value) => {
                assert_eq!(retained.staging, Some(&value.staging));
                assert_eq!(retained.staged, Some(value.staged.as_slice()));
                "verify"
            }
        };
        self.calls.borrow_mut().push(operation);
        if matches!(self.mode, Mode::Fault(at) if at == operation) {
            return Err(format!("malformed loader reply at {operation}"));
        }
        if matches!(self.mode, Mode::Fail(at) if at == operation) {
            return Ok(self.fail(operation));
        }
        Ok(match operation {
            "plan" => {
                let outputs = if matches!(self.mode, Mode::Overlap) {
                    json!([
                        {"id":"file-out","kind":"file","shape":"file","path_relative":"tree/file.bin"},
                        {"id":"tree-out","kind":"directory","shape":"directory","path_relative":"tree"}
                    ])
                } else {
                    json!([
                        {"id":"file-out","kind":"file","shape":"file","path_relative":"file.bin"},
                        {"id":"tree-out","kind":"directory","shape":"directory","path_relative":"tree"}
                    ])
                };
                self.response("plan", json!({"status":"ok","plan":{"summary":"native plan","outputs":outputs}}))
            }
            "fingerprint" => self.response(
                "fingerprint",
                json!({"status":"ok","fingerprint":{"digest":"b".repeat(64),"summary":"native toolchain"}}),
            ),
            "apply" => {
                self.write_outputs();
                self.response(
                    "apply",
                    json!({"status":"ok","staged":[
                        {"id":"file-out","path_relative":"file.bin","fresh":false},
                        {"id":"tree-out","path_relative":"tree","fresh":true}
                    ],"evidence":"native apply"}),
                )
            }
            "verify" => {
                let stage = self.root.join("target/vibe-native/native-demo");
                let (file_digest, file_bytes) = must(digest_file(&stage.join("file.bin")), "file digest");
                let tree = must(tree_digest(&stage.join("tree")), "tree digest");
                let digest = if matches!(self.mode, Mode::Mismatch) {
                    "c".repeat(64)
                } else {
                    file_digest
                };
                self.response(
                    "verify",
                    json!({"status":"ok","verified":[
                        {"id":"file-out","path_relative":"file.bin","digest":digest,"bytes":file_bytes.to_string()},
                        {"id":"tree-out","path_relative":"tree","digest":tree.digest,"bytes":tree.bytes.to_string()}
                    ],"evidence":"native verify"}),
                )
            }
            _ => unreachable!(),
        })
    }
}

#[test]
fn native_file_and_directory_are_independently_verified_then_recorded() {
    let root = TempDir::new().unwrap();
    let registry = registry(root.path());
    let routes = MechanismRoutes::default();
    let targets = [target()];
    let execution = execution(root.path(), &targets, &registry, &routes);
    let entry = entry(PIN);
    let binding = binding(PIN);
    let calls = FakeCalls::new(root.path(), Mode::Happy);

    let produced = execute_with(&execution, &targets[0], &entry, &binding, &calls).unwrap();

    assert_eq!(
        *calls.calls.borrow(),
        ["plan", "fingerprint", "apply", "verify"]
    );
    assert_eq!(produced.len(), 2);
    assert!(!root.path().join("cargo-ran").exists());
    let file = crate::mechanism::record::read_record(root.path(), "file-out")
        .unwrap()
        .unwrap();
    let tree = crate::mechanism::record::read_record(root.path(), "tree-out")
        .unwrap()
        .unwrap();
    assert_eq!(file.kind, RecordKind::File);
    assert_eq!(file.shape, ArtifactShape::File);
    assert_eq!(tree.kind, RecordKind::Directory);
    assert_eq!(tree.shape, ArtifactShape::Directory);
    assert_eq!(file.producer.provider.key, PIN);
    assert_eq!(file.producer.provider.version.as_deref(), Some("1.2.3"));
    assert_eq!(file.platform.as_deref(), Some(entry.platform.key()));
    assert!(file.freshness.inputs.is_none());
    assert_eq!(file.freshness.config.as_ref().map(String::len), Some(64));
    let fingerprint = "b".repeat(64);
    assert_eq!(
        file.freshness.toolchain.as_deref(),
        Some(fingerprint.as_str())
    );
    assert!(
        tree.verification
            .evidence
            .unwrap()
            .contains("sha256-tree/1")
    );
    assert!(!produced[0].fresh && produced[1].fresh);
}

#[test]
fn every_provider_failure_mismatch_extra_and_overlap_leave_zero_records() {
    for mode in [
        Mode::Fail("plan"),
        Mode::Fail("fingerprint"),
        Mode::Fail("apply"),
        Mode::Fail("verify"),
        Mode::Fault("plan"),
        Mode::Mismatch,
        Mode::Extra,
        Mode::Overlap,
    ] {
        let root = TempDir::new().unwrap();
        let registry = registry(root.path());
        let routes = MechanismRoutes::default();
        let targets = [target()];
        let execution = execution(root.path(), &targets, &registry, &routes);
        let calls = FakeCalls::new(root.path(), mode);
        let error = execute_with(&execution, &targets[0], &entry(PIN), &binding(PIN), &calls)
            .expect_err("fault must refuse native build");
        match mode {
            Mode::Extra => assert!(error.to_string().contains("no planned output owns")),
            Mode::Overlap => {
                assert!(error.to_string().contains("overlap"));
                assert_eq!(calls.calls.borrow().as_slice(), &["plan"]);
            }
            _ => {}
        }
        assert!(!root.path().join(".vibe/state/artifacts").exists());
        assert!(!root.path().join("cargo-ran").exists());
    }
}

#[test]
fn unsupported_kind_and_missing_or_stale_binding_refuse_without_fallback() {
    let root = TempDir::new().unwrap();
    let registry = registry(root.path());
    let mut routes = MechanismRoutes::default();
    routes.insert(key(), vibe_core::manifest::ProviderPin::parse(PIN).unwrap());
    let targets = [target()];
    let execution = execution(root.path(), &targets, &registry, &routes);
    let mut calls = FakeCalls::new(root.path(), Mode::Happy);
    calls.kinds = vec![WireKind::File];
    assert!(execute_with(&execution, &targets[0], &entry(PIN), &binding(PIN), &calls).is_err());
    assert!(calls.calls.borrow().is_empty());

    assert!(
        execute_prepared_build_targets(&execution, &PreparedNativeMechanisms::default()).is_err()
    );
    let stale = PreparedNativeMechanisms {
        entries: vec![entry("__host__/demo#stale")],
    };
    assert!(execute_prepared_build_targets(&execution, &stale).is_err());
    assert!(!root.path().join("target").exists());
    assert!(!root.path().join(".vibe").exists());
}

#[test]
fn engine_fingerprint_covers_every_authored_build_axis_and_exact_pin() {
    let original = target();
    let baseline = must(config_fingerprint(&original, PIN), "baseline fingerprint");
    let variants = [
        {
            let mut value = original.clone();
            value.workdir = "native".to_owned();
            value
        },
        {
            let mut value = original.clone();
            value.config = Some(config("mode='other'"));
            value
        },
        {
            let mut value = original.clone();
            value.inputs = None;
            value
        },
        {
            let mut value = original.clone();
            value.outputs[0].select = Some(config("name='other'"));
            value
        },
    ];
    for variant in variants {
        assert_ne!(
            must(config_fingerprint(&variant, PIN), "variant fingerprint"),
            baseline
        );
    }
    assert_ne!(
        must(
            config_fingerprint(&original, "__host__/other#builder"),
            "pin fingerprint"
        ),
        baseline
    );
}
