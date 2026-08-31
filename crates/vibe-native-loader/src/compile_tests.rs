use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::json;
use tempfile::TempDir;
use vibe_core::lifecycle::{CompilePoint, ExtensionPoint};
use vibe_wire::generated::native::e1::context::Context;

use super::*;

type ErrorClassifier = fn(&NativeLoadError) -> bool;
type LifecycleReplyCase = (Vec<u8>, ErrorClassifier);
type CompilerManifestCase = (&'static str, CompilePoint, Vec<u8>, ErrorClassifier);

#[derive(Clone)]
struct FakeCall {
    status: i32,
    bytes: Option<Vec<u8>>,
    len: usize,
}

type ResponseMatrixCase = (FakeCall, &'static str, usize);

impl FakeCall {
    fn published(status: i32, bytes: Vec<u8>) -> Self {
        let len = bytes.len();
        Self {
            status,
            bytes: Some(bytes),
            len,
        }
    }

    fn null(status: i32, len: usize) -> Self {
        Self {
            status,
            bytes: None,
            len,
        }
    }
}

struct FakeLibrary {
    manifest: Vec<u8>,
    calls: Mutex<VecDeque<FakeCall>>,
    invoke_count: AtomicUsize,
    free_count: Arc<AtomicUsize>,
    requests: Mutex<Vec<Vec<u8>>>,
}

impl ffi::LibraryHandle for FakeLibrary {
    fn abi(&self) -> u32 {
        1
    }

    fn manifest_bytes(&self, _path: &str) -> Result<Vec<u8>, NativeLoadError> {
        Ok(self.manifest.clone())
    }

    fn invoke(&self, request: &[u8], owner: Arc<dyn ffi::LibraryHandle>) -> ffi::CallResult {
        self.invoke_count.fetch_add(1, Ordering::SeqCst);
        self.requests
            .lock()
            .expect("request lock")
            .push(request.to_vec());
        let call = self
            .calls
            .lock()
            .expect("call lock")
            .pop_front()
            .expect("one fake call per admitted invocation");
        let response = call.bytes.map(|bytes| {
            ffi::PublishedResponse::fake(bytes, call.len, Arc::clone(&self.free_count), owner)
        });
        ffi::CallResult {
            status: call.status,
            response,
            len: call.len,
        }
    }
}

struct FakeOpener {
    library: Arc<FakeLibrary>,
    open_count: AtomicUsize,
}

impl ffi::LibraryOpener for FakeOpener {
    fn open(
        &self,
        _canonical_path: &Path,
        _display_path: &str,
    ) -> Result<Arc<dyn ffi::LibraryHandle>, NativeLoadError> {
        self.open_count.fetch_add(1, Ordering::SeqCst);
        Ok(Arc::clone(&self.library) as Arc<dyn ffi::LibraryHandle>)
    }
}

fn loader_for(
    manifest: Vec<u8>,
    calls: impl IntoIterator<Item = FakeCall>,
) -> (NativeLoader, Arc<FakeLibrary>, Arc<FakeOpener>) {
    let library = Arc::new(FakeLibrary {
        manifest,
        calls: Mutex::new(calls.into_iter().collect()),
        invoke_count: AtomicUsize::new(0),
        free_count: Arc::new(AtomicUsize::new(0)),
        requests: Mutex::new(Vec::new()),
    });
    let opener = Arc::new(FakeOpener {
        library: Arc::clone(&library),
        open_count: AtomicUsize::new(0),
    });
    let loader = NativeLoader::with_opener(Arc::clone(&opener) as Arc<dyn ffi::LibraryOpener>);
    (loader, library, opener)
}

fn manifest(rows: &[(&str, &str, Option<u32>)]) -> Vec<u8> {
    let rows = rows
        .iter()
        .map(|(id, point, schema)| {
            let mut row = json!({"id": id, "point": point});
            if let Some(schema) = schema {
                row["ir_schema"] = json!(schema);
            }
            row
        })
        .collect::<Vec<_>>();
    serde_json::to_vec(&json!({"extensions": rows})).expect("manifest JSON")
}

fn fake_file() -> (TempDir, PathBuf) {
    let directory = tempfile::tempdir().expect("temp dir");
    let path = directory.path().join("compiler.bin");
    std::fs::write(&path, b"fake").expect("fake library file");
    (directory, path)
}

fn invoke_compile(
    loader: &NativeLoader,
    path: &Path,
    id: &str,
    point: CompilePoint,
    request: &[u8],
) -> Result<Vec<u8>, NativeLoadError> {
    loader.invoke_compile(NativeCompileInvocation {
        library: path,
        extension_id: id,
        point,
        request,
    })
}

fn context() -> Context {
    serde_json::from_value(json!({
        "artifacts": [],
        "envelope": 1,
        "execution": {"id": "life", "package": "org.example/life", "config": {}},
        "io": {"scratch": ".vibe/lifecycle/run/life"},
        "point": "phase:build",
        "project": {
            "root": ".", "name": "host", "version": "1.0.0", "kind": "flow",
            "manifest": "vibe.toml", "spec_roots": ["vibevm/vibespecs"]
        },
        "run": {
            "requested": "build", "chain": ["validate", "install", "generate", "build"],
            "phase": "build", "offline": true, "assume_yes": false,
            "agent_mode": "cli", "force": false
        },
        "world": {"lockfile": "vibe.lock", "deps_root": "vibevm/vibedeps", "packages": []}
    }))
    .expect("context")
}

fn invoke_lifecycle(loader: &NativeLoader, path: &Path) -> Result<Reply, NativeLoadError> {
    loader.invoke(NativeInvocation {
        library: path,
        extension_id: "life",
        point: "phase:build"
            .parse::<ExtensionPoint>()
            .expect("lifecycle point"),
        ir_schema: None,
        context: &context(),
    })
}

fn valid_lifecycle_reply() -> Vec<u8> {
    br#"{"artifacts":[],"envelope":1,"status":"ok","message":"exact"}"#.to_vec()
}

fn error_kind(error: &NativeLoadError) -> &'static str {
    match error {
        NativeLoadError::MissingResponse { .. } => "missing",
        NativeLoadError::NullResponseWithLength { .. } => "null-length",
        NativeLoadError::ZeroLengthResponse { .. } => "zero",
        NativeLoadError::PluginStatus { .. } => "status",
        NativeLoadError::PluginStatusWithLength { .. } => "status-length",
        NativeLoadError::PluginStatusWithResponse { .. } => "status-response",
        NativeLoadError::ReplyTooLarge { .. } => "oversize",
        other => panic!("unexpected response error: {other}"),
    }
}

#[test]
fn compile_returns_exact_opaque_bytes_and_forwards_request_without_decode() {
    for response in [vec![0xff, 0xfe, 0xfd], b"not compiler JSON".to_vec()] {
        let (_directory, path) = fake_file();
        let request = br#"{"opaque-request":true}"#;
        let (loader, library, _opener) = loader_for(
            manifest(&[("compiler", "compile:pass", Some(1))]),
            [FakeCall::published(0, response.clone())],
        );
        let actual = invoke_compile(&loader, &path, "compiler", CompilePoint::Pass, request)
            .expect("opaque response accepted");
        assert_eq!(actual, response);
        assert_eq!(&*library.requests.lock().expect("requests"), &[request]);
        assert_eq!(library.free_count.load(Ordering::SeqCst), 1);
    }
}

#[test]
fn lifecycle_and_compile_share_the_response_refusal_and_free_matrix() {
    let cases: [ResponseMatrixCase; 7] = [
        (FakeCall::null(0, 0), "missing", 0),
        (FakeCall::null(0, 4), "null-length", 0),
        (
            FakeCall {
                status: 0,
                bytes: Some(vec![1]),
                len: 0,
            },
            "zero",
            1,
        ),
        (FakeCall::null(7, 0), "status", 0),
        (FakeCall::null(7, 4), "status-length", 0),
        (FakeCall::published(7, vec![1]), "status-response", 1),
        (
            FakeCall {
                status: 0,
                bytes: Some(vec![1; REPLY_CAP + 1]),
                len: REPLY_CAP + 1,
            },
            "oversize",
            1,
        ),
    ];
    for (case, expected_kind, expected_frees) in cases {
        for compile in [false, true] {
            let (_directory, path) = fake_file();
            let rows = if compile {
                [("compiler", "compile:pass", Some(1))]
            } else {
                [("life", "phase:build", None)]
            };
            let (loader, library, _opener) = loader_for(manifest(&rows), [case.clone()]);
            let error = if compile {
                invoke_compile(&loader, &path, "compiler", CompilePoint::Pass, b"request")
                    .expect_err("matrix refuses")
            } else {
                invoke_lifecycle(&loader, &path).expect_err("matrix refuses")
            };
            assert_eq!(error_kind(&error), expected_kind);
            assert_eq!(
                library.free_count.load(Ordering::SeqCst),
                expected_frees,
                "published-response free count for compile={compile}, error={expected_kind}"
            );
        }
    }
}

#[test]
fn lifecycle_strict_reply_paths_and_success_remain_typed_and_exact() {
    let cases: [LifecycleReplyCase; 4] = [
        (vec![0xff], |e| {
            matches!(e, NativeLoadError::ReplyUtf8 { .. })
        }),
        (b"{".to_vec(), |e| {
            matches!(e, NativeLoadError::ReplyJson { .. })
        }),
        (
            br#"{"artifacts":[],"envelope":1,"status":"ok","extra":true}"#.to_vec(),
            |e| matches!(e, NativeLoadError::ReplyJson { .. }),
        ),
        (
            br#"{"artifacts":[],"envelope":2,"status":"ok"}"#.to_vec(),
            |e| matches!(e, NativeLoadError::ReplyEnvelope { actual: 2, .. }),
        ),
    ];
    for (bytes, classify) in cases {
        let (_directory, path) = fake_file();
        let (loader, library, _opener) = loader_for(
            manifest(&[("life", "phase:build", None)]),
            [FakeCall::published(0, bytes)],
        );
        let error = invoke_lifecycle(&loader, &path).expect_err("strict reply refuses");
        assert!(classify(&error));
        assert_eq!(library.free_count.load(Ordering::SeqCst), 1);
    }

    let (_directory, path) = fake_file();
    let (loader, library, _opener) = loader_for(
        manifest(&[("life", "phase:build", None)]),
        [FakeCall::published(0, valid_lifecycle_reply())],
    );
    let reply = invoke_lifecycle(&loader, &path).expect("typed lifecycle success");
    assert_eq!(
        serde_json::to_value(reply).expect("reply JSON"),
        json!({"artifacts": [], "envelope": 1, "status": "ok", "message": "exact"})
    );
    assert_eq!(library.free_count.load(Ordering::SeqCst), 1);
}

#[test]
fn compiler_manifest_requires_exact_id_point_and_fixed_schema_one() {
    let cases: Vec<CompilerManifestCase> = vec![
        (
            "missing",
            CompilePoint::Pass,
            manifest(&[("compiler", "compile:pass", Some(1))]),
            |e| matches!(e, NativeLoadError::MissingExtensionId { .. }),
        ),
        (
            "compiler",
            CompilePoint::Emitted,
            manifest(&[("compiler", "compile:pass", Some(1))]),
            |e| matches!(e, NativeLoadError::ExtensionPointMismatch { .. }),
        ),
        (
            "compiler",
            CompilePoint::Pass,
            manifest(&[("compiler", "compile:pass", None)]),
            |e| matches!(e, NativeLoadError::IrSchemaMismatch { .. }),
        ),
        (
            "compiler",
            CompilePoint::Pass,
            manifest(&[("compiler", "compile:pass", Some(2))]),
            |e| matches!(e, NativeLoadError::IrSchemaMismatch { .. }),
        ),
    ];
    for (id, point, manifest, classify) in cases {
        let (_directory, path) = fake_file();
        let (loader, library, _opener) =
            loader_for(manifest, [FakeCall::published(0, b"unused".to_vec())]);
        let error = invoke_compile(&loader, &path, id, point, b"request")
            .expect_err("manifest mismatch refuses");
        assert!(classify(&error), "unexpected error: {error}");
        assert_eq!(library.invoke_count.load(Ordering::SeqCst), 0);
    }

    let (_directory, path) = fake_file();
    let (loader, library, _opener) = loader_for(
        manifest(&[
            ("other", "compile:source", Some(1)),
            ("compiler", "compile:pass", Some(1)),
        ]),
        [FakeCall::published(0, b"exact".to_vec())],
    );
    assert_eq!(
        invoke_compile(&loader, &path, "compiler", CompilePoint::Pass, b"request")
            .expect("exact compiler row admits"),
        b"exact"
    );
    assert_eq!(library.invoke_count.load(Ordering::SeqCst), 1);
}

#[test]
fn homogeneous_multi_entry_images_admit_and_mixed_images_never_invoke() {
    let (_directory, path) = fake_file();
    let (loader, lifecycle, _opener) = loader_for(
        manifest(&[
            ("life", "phase:build", None),
            ("slot", "slot:pre-install", None),
        ]),
        [FakeCall::published(0, valid_lifecycle_reply())],
    );
    invoke_lifecycle(&loader, &path).expect("lifecycle-only image admits");
    assert_eq!(lifecycle.invoke_count.load(Ordering::SeqCst), 1);

    let (_directory, path) = fake_file();
    let (loader, compiler, _opener) = loader_for(
        manifest(&[
            ("other", "compile:source", Some(1)),
            ("compiler", "compile:pass", Some(1)),
        ]),
        [FakeCall::published(0, b"ok".to_vec())],
    );
    invoke_compile(&loader, &path, "compiler", CompilePoint::Pass, b"request")
        .expect("compiler-only image admits");
    assert_eq!(compiler.invoke_count.load(Ordering::SeqCst), 1);

    for rows in [
        [
            ("life", "phase:build", None),
            ("compiler", "compile:pass", Some(1)),
        ],
        [
            ("compiler", "compile:pass", Some(1)),
            ("life", "phase:build", None),
        ],
    ] {
        for compile in [false, true] {
            let (_directory, path) = fake_file();
            let (loader, library, _opener) = loader_for(
                manifest(&rows),
                [FakeCall::published(0, b"unused".to_vec())],
            );
            let error = if compile {
                invoke_compile(&loader, &path, "compiler", CompilePoint::Pass, b"request")
                    .expect_err("mixed compiler image refuses")
            } else {
                invoke_lifecycle(&loader, &path).expect_err("mixed lifecycle image refuses")
            };
            assert!(matches!(
                error,
                NativeLoadError::ManifestFamilyMismatch { .. }
            ));
            assert_eq!(library.invoke_count.load(Ordering::SeqCst), 0);
        }
    }
}

#[test]
fn every_point_is_typed_before_selection_and_duplicates_keep_precedence() {
    for rows in [
        [
            ("compiler", "compile:pass", Some(1)),
            ("bad", "unknown:any", Some(1)),
        ],
        [
            ("bad", "unknown:any", Some(1)),
            ("compiler", "compile:pass", Some(1)),
        ],
    ] {
        let (_directory, path) = fake_file();
        let (loader, library, _opener) = loader_for(
            manifest(&rows),
            [FakeCall::published(0, b"unused".to_vec())],
        );
        let error = invoke_compile(&loader, &path, "compiler", CompilePoint::Pass, b"request")
            .expect_err("invalid non-selected point refuses");
        assert!(matches!(
            error,
            NativeLoadError::InvalidExtensionPoint { .. }
        ));
        assert_eq!(library.invoke_count.load(Ordering::SeqCst), 0);
    }

    let duplicate_then_invalid = manifest(&[
        ("same", "compile:pass", Some(1)),
        ("same", "unknown:any", Some(1)),
    ]);
    let (_directory, path) = fake_file();
    let (loader, library, _opener) = loader_for(
        duplicate_then_invalid,
        [FakeCall::published(0, b"unused".to_vec())],
    );
    let error = invoke_compile(
        &loader,
        &path,
        "same",
        CompilePoint::Pass,
        b"secret-request",
    )
    .expect_err("duplicate precedence");
    assert!(matches!(
        error,
        NativeLoadError::DuplicateExtensionId { .. }
    ));
    let message = error.to_string();
    assert!(!message.contains("secret-request"));
    assert_eq!(library.invoke_count.load(Ordering::SeqCst), 0);
}

#[test]
fn invalid_point_precedes_mixed_family_in_both_family_orders() {
    for rows in [
        [
            ("life", "phase:build", None),
            ("compiler", "compile:pass", Some(1)),
            ("bad", "unknown:any", Some(1)),
        ],
        [
            ("compiler", "compile:pass", Some(1)),
            ("life", "phase:build", None),
            ("bad", "unknown:any", Some(1)),
        ],
    ] {
        for compile in [false, true] {
            let (_directory, path) = fake_file();
            let (loader, library, _opener) = loader_for(
                manifest(&rows),
                [FakeCall::published(0, b"unused".to_vec())],
            );
            let error = if compile {
                invoke_compile(&loader, &path, "compiler", CompilePoint::Pass, b"request")
                    .expect_err("invalid point precedes compiler family mismatch")
            } else {
                invoke_lifecycle(&loader, &path)
                    .expect_err("invalid point precedes lifecycle family mismatch")
            };
            assert!(matches!(
                error,
                NativeLoadError::InvalidExtensionPoint { .. }
            ));
            assert_eq!(library.invoke_count.load(Ordering::SeqCst), 0);
        }
    }
}

#[test]
fn repeated_compile_use_keeps_one_canonical_cache() {
    let (_directory, path) = fake_file();
    let (loader, library, opener) = loader_for(
        manifest(&[("compiler", "compile:pass", Some(1))]),
        [
            FakeCall::published(0, b"first".to_vec()),
            FakeCall::published(0, b"second".to_vec()),
        ],
    );
    assert_eq!(
        invoke_compile(&loader, &path, "compiler", CompilePoint::Pass, b"one")
            .expect("first compile"),
        b"first"
    );
    assert_eq!(
        invoke_compile(&loader, &path, "compiler", CompilePoint::Pass, b"two")
            .expect("second compile"),
        b"second"
    );
    assert_eq!(opener.open_count.load(Ordering::SeqCst), 1);
    assert_eq!(library.invoke_count.load(Ordering::SeqCst), 2);
    assert_eq!(library.free_count.load(Ordering::SeqCst), 2);
}

#[test]
fn compile_and_lifecycle_are_structurally_fenced_to_shared_loader_paths() {
    let product = include_str!("lib.rs");
    assert_eq!(product.matches("fn admit_library(").count(), 1);
    assert_eq!(product.matches("fn invoke_admitted(").count(), 1);
    assert_eq!(product.matches("fn admitted_response(").count(), 1);
    for method in ["pub fn invoke(&self", "pub fn invoke_compile("] {
        let start = product.find(method).expect("public method");
        let tail = &product[start..];
        let end = tail.find("\n    }").expect("method end");
        let body = &tail[..end];
        assert!(body.contains("self.admit_library("));
        assert!(body.contains("invoke_admitted("));
    }
    assert!(!product.contains("compile_reply"));
    assert!(!include_str!("admission.rs").contains("compile_reply"));
    assert!(!include_str!("error.rs").contains("compile_reply"));
}
