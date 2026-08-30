use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier, Mutex};

use serde_json::json;
use tempfile::TempDir;
use vibe_core::lifecycle::ExtensionPoint;
use vibe_wire::generated::native::e1::context::Context;

use super::*;

type ErrorClassifier = fn(&NativeLoadError) -> bool;
type ManifestCase = (FakeManifest, Option<u32>, ErrorClassifier);
type ResponseCase = (FakeCall, ErrorClassifier, usize);
type ReplyCase = (Vec<u8>, ErrorClassifier);

#[derive(Clone)]
enum FakeManifest {
    Bytes(Vec<u8>),
    Null,
}

#[derive(Clone)]
struct FakeCall {
    status: i32,
    bytes: Option<Vec<u8>>,
    len: usize,
}

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
    abi: u32,
    manifest: FakeManifest,
    call: Mutex<FakeCall>,
    manifest_count: AtomicUsize,
    invoke_count: AtomicUsize,
    free_count: Arc<AtomicUsize>,
    drop_count: Arc<AtomicUsize>,
}

impl FakeLibrary {
    fn new(abi: u32, manifest: FakeManifest, call: FakeCall) -> Arc<Self> {
        Arc::new(Self {
            abi,
            manifest,
            call: Mutex::new(call),
            manifest_count: AtomicUsize::new(0),
            invoke_count: AtomicUsize::new(0),
            free_count: Arc::new(AtomicUsize::new(0)),
            drop_count: Arc::new(AtomicUsize::new(0)),
        })
    }
}

impl Drop for FakeLibrary {
    fn drop(&mut self) {
        self.drop_count.fetch_add(1, Ordering::SeqCst);
    }
}

impl ffi::LibraryHandle for FakeLibrary {
    fn abi(&self) -> u32 {
        self.abi
    }

    fn manifest_bytes(&self, path: &str) -> Result<Vec<u8>, NativeLoadError> {
        self.manifest_count.fetch_add(1, Ordering::SeqCst);
        match &self.manifest {
            FakeManifest::Bytes(bytes) => Ok(bytes.clone()),
            FakeManifest::Null => Err(NativeLoadError::ManifestPointerNull {
                path: path.to_owned(),
            }),
        }
    }

    fn invoke(&self, _request: &[u8], owner: Arc<dyn ffi::LibraryHandle>) -> ffi::CallResult {
        self.invoke_count.fetch_add(1, Ordering::SeqCst);
        let call = self.call.lock().expect("fake call lock").clone();
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
    missing: Option<&'static str>,
    open_count: AtomicUsize,
}

impl FakeOpener {
    fn new(library: Arc<FakeLibrary>) -> Arc<Self> {
        Arc::new(Self {
            library,
            missing: None,
            open_count: AtomicUsize::new(0),
        })
    }

    fn missing(library: Arc<FakeLibrary>, symbol: &'static str) -> Arc<Self> {
        Arc::new(Self {
            library,
            missing: Some(symbol),
            open_count: AtomicUsize::new(0),
        })
    }
}

impl ffi::LibraryOpener for FakeOpener {
    fn open(
        &self,
        _canonical_path: &Path,
        display_path: &str,
    ) -> Result<Arc<dyn ffi::LibraryHandle>, NativeLoadError> {
        self.open_count.fetch_add(1, Ordering::SeqCst);
        if let Some(symbol) = self.missing {
            return Err(NativeLoadError::MissingSymbol {
                path: display_path.to_owned(),
                symbol,
            });
        }
        Ok(Arc::clone(&self.library) as Arc<dyn ffi::LibraryHandle>)
    }
}

fn manifest(point: &str, schema: Option<u32>) -> Vec<u8> {
    let mut row = json!({"id": "selected", "point": point});
    if let Some(schema) = schema {
        row["ir_schema"] = json!(schema);
    }
    serde_json::to_vec(&json!({"extensions": [row]})).expect("manifest JSON")
}

fn valid_reply(status: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "artifacts": [],
        "envelope": 1,
        "status": status
    }))
    .expect("reply JSON")
}

fn context() -> Context {
    serde_json::from_value(json!({
        "artifacts": [],
        "envelope": 1,
        "execution": {"id": "selected", "package": "org.example/plugin", "config": {}},
        "io": {"scratch": ".vibe/lifecycle/run/selected"},
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
    .expect("context matches generated wire")
}

fn phase_build() -> ExtensionPoint {
    "phase:build".parse().expect("typed phase point")
}

fn fake_file() -> (TempDir, PathBuf) {
    let directory = tempfile::tempdir().expect("temp dir");
    let path = directory.path().join("plugin.bin");
    std::fs::write(&path, b"fake").expect("fake library file");
    (directory, path)
}

fn invoke(
    loader: &NativeLoader,
    path: &Path,
    extension_id: &str,
    point: ExtensionPoint,
    schema: Option<u32>,
) -> Result<vibe_wire::generated::native::e1::reply::Reply, NativeLoadError> {
    loader.invoke(NativeInvocation {
        library: path,
        extension_id,
        point,
        ir_schema: schema,
        context: &context(),
    })
}

fn loader_for(
    manifest: FakeManifest,
    call: FakeCall,
) -> (NativeLoader, Arc<FakeLibrary>, Arc<FakeOpener>) {
    let library = FakeLibrary::new(1, manifest, call);
    let opener = FakeOpener::new(Arc::clone(&library));
    let loader = NativeLoader::with_opener(Arc::clone(&opener) as Arc<dyn ffi::LibraryOpener>);
    (loader, library, opener)
}

#[test]
fn paths_must_be_absolute_existing_files() {
    let error = validate_path(Path::new("relative.dll")).expect_err("relative path refuses");
    assert!(matches!(error, NativeLoadError::PathNotAbsolute { .. }));

    let directory = tempfile::tempdir().expect("temp dir");
    let error = validate_path(directory.path()).expect_err("directory refuses");
    assert!(matches!(error, NativeLoadError::PathNotFile { .. }));

    let missing = directory.path().join("missing.dll");
    let error = validate_path(&missing).expect_err("missing path refuses");
    assert!(matches!(error, NativeLoadError::PathUnavailable { .. }));
}

#[test]
fn all_four_exact_missing_symbol_names_refuse() {
    assert_eq!(
        ffi::REQUIRED_SYMBOLS,
        [
            "vibe_ext_abi",
            "vibe_ext_manifest",
            "vibe_ext_invoke",
            "vibe_ext_free"
        ]
    );
    let (_directory, path) = fake_file();
    for symbol in ffi::REQUIRED_SYMBOLS {
        let library = FakeLibrary::new(
            1,
            FakeManifest::Bytes(manifest("phase:build", None)),
            FakeCall::published(0, valid_reply("ok")),
        );
        let opener = FakeOpener::missing(Arc::clone(&library), symbol);
        let loader = NativeLoader::with_opener(opener as Arc<dyn ffi::LibraryOpener>);
        let error = invoke(&loader, &path, "selected", phase_build(), None)
            .expect_err("missing symbol refuses");
        assert!(
            matches!(error, NativeLoadError::MissingSymbol { symbol: found, .. } if found == symbol)
        );
        assert_eq!(library.manifest_count.load(Ordering::SeqCst), 0);
        assert_eq!(library.invoke_count.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn wrong_abi_stops_before_manifest_and_invoke() {
    let (_directory, path) = fake_file();
    let library = FakeLibrary::new(
        9,
        FakeManifest::Bytes(manifest("phase:build", None)),
        FakeCall::published(0, valid_reply("ok")),
    );
    let opener = FakeOpener::new(Arc::clone(&library));
    let loader = NativeLoader::with_opener(opener as Arc<dyn ffi::LibraryOpener>);
    let error =
        invoke(&loader, &path, "selected", phase_build(), None).expect_err("wrong ABI refuses");
    assert!(matches!(
        error,
        NativeLoadError::AbiMismatch { actual: 9, .. }
    ));
    assert!(error.to_string().contains("rebuild: vibe build"));
    assert_eq!(library.manifest_count.load(Ordering::SeqCst), 0);
    assert_eq!(library.invoke_count.load(Ordering::SeqCst), 0);
}

#[test]
fn hostile_manifest_branches_never_invoke() {
    let cases: Vec<ManifestCase> = vec![
        (FakeManifest::Null, None, |e| matches!(e, NativeLoadError::ManifestPointerNull { .. })),
        (FakeManifest::Bytes(vec![b'x'; MANIFEST_CAP]), None, |e| matches!(e, NativeLoadError::ManifestTooLarge { .. })),
        (FakeManifest::Bytes(vec![0xff]), None, |e| matches!(e, NativeLoadError::ManifestUtf8 { .. })),
        (FakeManifest::Bytes(b"{".to_vec()), None, |e| matches!(e, NativeLoadError::ManifestJson { .. })),
        (FakeManifest::Bytes(br#"{"extensions":[{"id":"selected","point":"phase:build"},{"id":"selected","point":"phase:test"}]}"#.to_vec()), None, |e| matches!(e, NativeLoadError::DuplicateExtensionId { .. })),
        (FakeManifest::Bytes(br#"{"extensions":[]}"#.to_vec()), None, |e| matches!(e, NativeLoadError::MissingExtensionId { .. })),
        (FakeManifest::Bytes(manifest("unknown:build", None)), None, |e| matches!(e, NativeLoadError::InvalidExtensionPoint { .. })),
        (FakeManifest::Bytes(manifest("phase:test", None)), None, |e| matches!(e, NativeLoadError::ExtensionPointMismatch { .. })),
        (FakeManifest::Bytes(manifest("phase:build", Some(1))), None, |e| matches!(e, NativeLoadError::IrSchemaMismatch { .. })),
        (FakeManifest::Bytes(manifest("phase:build", Some(2))), Some(1), |e| matches!(e, NativeLoadError::IrSchemaMismatch { .. })),
    ];
    for (manifest, schema, classify) in cases {
        let (_directory, path) = fake_file();
        let (loader, library, _opener) =
            loader_for(manifest, FakeCall::published(0, valid_reply("ok")));
        let error = invoke(&loader, &path, "selected", phase_build(), schema)
            .expect_err("hostile manifest refuses");
        assert!(classify(&error), "unexpected error: {error}");
        assert_eq!(library.invoke_count.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn exact_schema_and_typed_point_admission_reaches_invoke() {
    let (_directory, path) = fake_file();
    let (loader, library, _opener) = loader_for(
        FakeManifest::Bytes(manifest("phase:build", Some(7))),
        FakeCall::published(0, valid_reply("skip")),
    );
    let reply = invoke(&loader, &path, "selected", phase_build(), Some(7)).expect("admitted");
    assert_eq!(
        serde_json::to_value(reply).expect("reply JSON")["status"],
        "skip"
    );
    assert_eq!(library.invoke_count.load(Ordering::SeqCst), 1);
    assert_eq!(library.free_count.load(Ordering::SeqCst), 1);
}

#[test]
fn response_status_pointer_length_matrix_is_typed_and_frees_published_memory() {
    let cases: Vec<ResponseCase> = vec![
        (
            FakeCall::null(0, 0),
            |e| matches!(e, NativeLoadError::MissingResponse { .. }),
            0,
        ),
        (
            FakeCall::null(0, 4),
            |e| matches!(e, NativeLoadError::NullResponseWithLength { .. }),
            0,
        ),
        (
            FakeCall {
                status: 0,
                bytes: Some(vec![1]),
                len: 0,
            },
            |e| matches!(e, NativeLoadError::ZeroLengthResponse { .. }),
            1,
        ),
        (
            FakeCall::null(3, 0),
            |e| matches!(e, NativeLoadError::PluginStatus { status: 3, .. }),
            0,
        ),
        (
            FakeCall::null(3, 4),
            |e| matches!(e, NativeLoadError::PluginStatusWithLength { status: 3, .. }),
            0,
        ),
        (
            FakeCall::published(3, valid_reply("ok")),
            |e| {
                matches!(
                    e,
                    NativeLoadError::PluginStatusWithResponse { status: 3, .. }
                )
            },
            1,
        ),
        (
            FakeCall {
                status: 0,
                bytes: Some(vec![1]),
                len: REPLY_CAP + 1,
            },
            |e| matches!(e, NativeLoadError::ReplyTooLarge { .. }),
            1,
        ),
    ];
    for (call, classify, frees) in cases {
        let (_directory, path) = fake_file();
        let (loader, library, _opener) =
            loader_for(FakeManifest::Bytes(manifest("phase:build", None)), call);
        let error = invoke(&loader, &path, "selected", phase_build(), None)
            .expect_err("invalid response matrix refuses");
        assert!(classify(&error), "unexpected error: {error}");
        assert_eq!(library.free_count.load(Ordering::SeqCst), frees);
    }
}

#[test]
fn reply_validation_is_strict_and_frees_once_on_every_path() {
    let replies: Vec<ReplyCase> = vec![
        (vec![0xff], |e: &NativeLoadError| {
            matches!(e, NativeLoadError::ReplyUtf8 { .. })
        }),
        (b"{".to_vec(), |e: &NativeLoadError| {
            matches!(e, NativeLoadError::ReplyJson { .. })
        }),
        (
            br#"{"artifacts":[],"envelope":1,"status":"ok","extra":true}"#.to_vec(),
            |e: &NativeLoadError| matches!(e, NativeLoadError::ReplyJson { .. }),
        ),
        (
            br#"{"artifacts":[],"envelope":2,"status":"ok"}"#.to_vec(),
            |e: &NativeLoadError| matches!(e, NativeLoadError::ReplyEnvelope { actual: 2, .. }),
        ),
    ];
    for (bytes, classify) in replies {
        let (_directory, path) = fake_file();
        let (loader, library, _opener) = loader_for(
            FakeManifest::Bytes(manifest("phase:build", None)),
            FakeCall::published(0, bytes),
        );
        let error = invoke(&loader, &path, "selected", phase_build(), None)
            .expect_err("invalid reply refuses");
        assert!(classify(&error), "unexpected error: {error}");
        assert_eq!(library.free_count.load(Ordering::SeqCst), 1);
    }
}

#[test]
fn canonical_aliases_share_one_strong_cache_entry() {
    let (_directory, path) = fake_file();
    let parent = path.parent().expect("parent");
    std::fs::create_dir(parent.join("alias-hop")).expect("alias hop directory");
    let alias = parent
        .join("alias-hop")
        .join("..")
        .join(path.file_name().expect("name"));
    let (loader, _library, opener) = loader_for(
        FakeManifest::Bytes(manifest("phase:build", None)),
        FakeCall::published(0, valid_reply("ok")),
    );
    invoke(&loader, &path, "selected", phase_build(), None).expect("first invoke");
    invoke(&loader, &alias, "selected", phase_build(), None).expect("alias invoke");
    assert_eq!(opener.open_count.load(Ordering::SeqCst), 1);
}

#[test]
fn concurrent_first_use_opens_once() {
    let (_directory, path) = fake_file();
    let (loader, _library, opener) = loader_for(
        FakeManifest::Bytes(manifest("phase:build", None)),
        FakeCall::published(0, valid_reply("ok")),
    );
    let loader = Arc::new(loader);
    let barrier = Arc::new(Barrier::new(8));
    std::thread::scope(|scope| {
        for _ in 0..8 {
            let loader = Arc::clone(&loader);
            let barrier = Arc::clone(&barrier);
            let path = path.clone();
            scope.spawn(move || {
                barrier.wait();
                invoke(&loader, &path, "selected", phase_build(), None).expect("thread invoke");
            });
        }
    });
    assert_eq!(opener.open_count.load(Ordering::SeqCst), 1);
}

#[test]
fn strong_library_handle_lives_until_loader_drop() {
    let (_directory, path) = fake_file();
    let library = FakeLibrary::new(
        1,
        FakeManifest::Bytes(manifest("phase:build", None)),
        FakeCall::published(0, valid_reply("ok")),
    );
    let drop_count = Arc::clone(&library.drop_count);
    let opener = FakeOpener::new(Arc::clone(&library));
    let loader = NativeLoader::with_opener(Arc::clone(&opener) as Arc<dyn ffi::LibraryOpener>);
    drop(library);
    drop(opener);

    invoke(&loader, &path, "selected", phase_build(), None).expect("cached invocation");
    assert_eq!(drop_count.load(Ordering::SeqCst), 0);
    drop(loader);
    assert_eq!(drop_count.load(Ordering::SeqCst), 1);
}

#[test]
fn diagnostics_bound_paths_and_scalars_without_echoing_bodies() {
    let long = "x".repeat(400);
    let path = path_preview(Path::new(&long));
    let scalar = scalar_preview(&long);
    assert!(path.chars().count() <= PATH_PREVIEW_CHARS + 1);
    assert!(scalar.chars().count() <= SCALAR_PREVIEW_CHARS + 1);
    assert!(path.ends_with('…'));
    assert!(scalar.ends_with('…'));

    let (_directory, file) = fake_file();
    let secret = b"{secret-request-or-reply-body".to_vec();
    let (loader, _library, _opener) = loader_for(
        FakeManifest::Bytes(manifest("phase:build", None)),
        FakeCall::published(0, secret.clone()),
    );
    let message = invoke(&loader, &file, "selected", phase_build(), None)
        .expect_err("malformed reply")
        .to_string();
    assert!(!message.contains(std::str::from_utf8(&secret).expect("ASCII secret")));
}
