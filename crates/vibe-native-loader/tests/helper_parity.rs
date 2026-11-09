//! NATIVE-STOP-LAW: the helper transport changes where an ABI-1 call runs,
//! never what admission sees. Every family answers identically in-process and
//! through a helper session; unadmitted or changed images never load.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::json;
use sha2::{Digest, Sha256};
use vibe_core::lifecycle::{CompilePoint, ExtensionPoint};
use vibe_core::manifest::MechanismKey;
use vibe_native_loader::helper::{self, Direction, HelperChannel, HelperFault};
use vibe_native_loader::{
    NativeCompileInvocation, NativeInvocation, NativeLoadError, NativeLoader,
    NativeMechanismInvocation,
};
use vibe_wire::generated::native::e1::context::Context;
use vibe_wire::generated::native::e1::deploy_request::DeployRequest;

/// One helper session served by a thread of this test process.
struct ThreadChannel {
    scratch: tempfile::TempDir,
    session: [u8; helper::SESSION_BYTES],
    next: Mutex<u64>,
    closed: Arc<AtomicBool>,
    server: Option<std::thread::JoinHandle<Result<(), String>>>,
}

impl ThreadChannel {
    fn start() -> Arc<Self> {
        let scratch = tempfile::tempdir().unwrap();
        let session = [0x5a; helper::SESSION_BYTES];
        let closed = Arc::new(AtomicBool::new(false));
        let server = {
            let dir = scratch.path().to_path_buf();
            let closed = Arc::clone(&closed);
            let hex = helper::session_hex(&session);
            std::thread::spawn(move || helper::serve_until(&dir, &hex, &closed))
        };
        Arc::new(Self {
            scratch,
            session,
            next: Mutex::new(1),
            closed,
            server: Some(server),
        })
    }
}

impl HelperChannel for ThreadChannel {
    fn exchange(&self, request: Vec<u8>) -> Result<Vec<u8>, HelperFault> {
        let mut next = self.next.lock().unwrap();
        let sequence = *next;
        *next += 1;
        let dir = self.scratch.path();
        helper::publish(dir, Direction::Request, &self.session, sequence, &request)
            .map_err(HelperFault::Transport)?;
        loop {
            if let Some(reply) = helper::take(dir, Direction::Reply, &self.session, sequence)
                .map_err(HelperFault::Transport)?
            {
                return Ok(reply);
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }
}

impl Drop for ThreadChannel {
    fn drop(&mut self) {
        self.closed.store(true, Ordering::SeqCst);
        if let Some(server) = self.server.take() {
            server.join().unwrap().unwrap();
        }
    }
}

fn library(stem: &str) -> PathBuf {
    let executable = std::env::current_exe().unwrap();
    let deps = executable.parent().unwrap();
    let profile = deps.parent().unwrap();
    let exact = format!(
        "{}{stem}{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    );
    // `deps/` first: the copy this test binary was built with.
    [deps, profile]
        .into_iter()
        .flat_map(|directory| std::fs::read_dir(directory).unwrap())
        .map(|entry| entry.unwrap().path())
        .find(|path| path.file_name().is_some_and(|name| name == exact.as_str()))
        .unwrap_or_else(|| panic!("fixture cdylib `{exact}`"))
}

fn helper_loader(images: &[&Path]) -> (Arc<ThreadChannel>, NativeLoader) {
    let channel = ThreadChannel::start();
    let loader = NativeLoader::with_helper(Arc::clone(&channel) as Arc<dyn HelperChannel>);
    for image in images {
        let bytes = std::fs::read(image).unwrap();
        loader
            .expect_image(
                image,
                &format!("{:x}", Sha256::digest(&bytes)),
                bytes.len() as u64,
            )
            .unwrap();
    }
    (channel, loader)
}

fn lifecycle_context() -> Context {
    serde_json::from_value(json!({
        "artifacts": [],
        "envelope": 1,
        "execution": {"id": "fixture", "package": "org.example/fixture", "config": {}},
        "io": {"scratch": ".vibe/lifecycle/run/fixture"},
        "point": "phase:build",
        "project": {"root": ".", "name": "host", "version": "1.0.0", "kind": "flow",
            "manifest": "vibe.toml", "spec_roots": ["vibevm/vibespecs"]},
        "run": {
            "requested": "build", "chain": ["validate", "install", "generate", "build"],
            "phase": "build", "offline": true, "assume_yes": false,
            "agent_mode": "cli", "force": false
        },
        "world": {"lockfile": "vibe.lock", "deps_root": "vibevm/vibedeps", "packages": []}
    }))
    .unwrap()
}

fn compile_request(id: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "envelope": 2,
        "execution": {"id": id, "package": "org.example/compiler", "config": {}},
        "io": {"scratch": ".vibe/compile/run"},
        "payload": {
            "shape": "source-document", "ir_schema": 2, "level": "source",
            "cardinality": "document",
            "doc": {
                "address": {"kind": "static-entry", "origin": "fixture", "path": "fixture.md"},
                "format": "markdown", "text": "fixture compiler input",
                "subject": {"declared_path": "fixture.md", "provider": {"kind": "unclaimed"}}
            }
        },
        "point": "compile:pass",
        "project": {"root": ".", "name": "host", "version": "1.0.0", "kind": "flow",
            "manifest": "vibe.toml", "spec_roots": ["vibevm/vibespecs"]},
        "world": {"lockfile": "vibe.lock", "deps_root": "vibevm/vibedeps", "packages": []}
    }))
    .unwrap()
}

#[test]
fn every_native_family_answers_identically_through_the_helper() {
    let lifecycle = library("vibe_native_loader_fixture");
    let compiler = library("vibe_native_loader_compiler_fixture");
    let mechanism = library("vibe_native_loader_mechanism_fixture");
    let in_process = NativeLoader::new();
    let (_channel, hosted) = helper_loader(&[&lifecycle, &compiler, &mechanism]);

    let context = lifecycle_context();
    let lifecycle_call = |loader: &NativeLoader| {
        loader
            .invoke(NativeInvocation {
                library: &lifecycle,
                extension_id: "fixture",
                point: "phase:build".parse::<ExtensionPoint>().unwrap(),
                ir_schema: None,
                context: &context,
            })
            .map(|reply| serde_json::to_value(reply).unwrap())
            .map_err(|error| error.to_string())
    };
    let local = lifecycle_call(&in_process);
    assert_eq!(
        local.as_ref().unwrap()["message"],
        "fixture handled phase:build"
    );
    assert_eq!(lifecycle_call(&hosted), local);

    for id in [
        "compiler-ok",
        "compiler-fail",
        "compiler-panic",
        "compiler-schema-one",
    ] {
        let request = compile_request(id);
        let compile_call = |loader: &NativeLoader| {
            loader
                .invoke_compile(NativeCompileInvocation {
                    library: &compiler,
                    extension_id: id,
                    point: CompilePoint::Pass,
                    request: &request,
                })
                .map_err(|error| error.to_string())
        };
        assert_eq!(compile_call(&hosted), compile_call(&in_process), "{id}");
    }

    let request: DeployRequest = serde_json::from_value(json!({
        "operation": "remove", "envelope": 1, "protocol": 1,
        "identity": {"provider": "org.example/provider", "mechanism": "fixture-deploy",
            "target": "tool", "profile": "default"},
        "resources": ["bin/tool"]
    }))
    .unwrap();
    let key: MechanismKey = "deploy:fixture".parse().unwrap();
    let mechanism_call = |loader: &NativeLoader| {
        loader
            .invoke_mechanism(NativeMechanismInvocation {
                library: &mechanism,
                provider: "org.example/provider",
                mechanism_id: "fixture-deploy",
                logical_key: &key,
                request: &request,
            })
            .map(|reply| serde_json::to_value(reply).unwrap())
            .map_err(|error| error.to_string())
    };
    let local = mechanism_call(&in_process);
    assert!(local.is_ok());
    assert_eq!(mechanism_call(&hosted), local);
}

#[test]
fn the_helper_never_loads_an_unadmitted_or_changed_image() {
    let source = library("vibe_native_loader_compiler_fixture");
    let request = compile_request("compiler-ok");
    let call = |loader: &NativeLoader, library: &Path| {
        loader.invoke_compile(NativeCompileInvocation {
            library,
            extension_id: "compiler-ok",
            point: CompilePoint::Pass,
            request: &request,
        })
    };
    let (_channel, unadmitted) = helper_loader(&[]);
    assert!(matches!(
        call(&unadmitted, &source),
        Err(NativeLoadError::HelperImageUnadmitted { .. })
    ));

    let copy = tempfile::tempdir().unwrap();
    let image = copy.path().join(source.file_name().unwrap());
    std::fs::copy(&source, &image).unwrap();
    let (_channel, changed) = helper_loader(&[&image]);
    let mut bytes = std::fs::read(&image).unwrap();
    bytes.push(0);
    std::fs::write(&image, bytes).unwrap();
    assert!(matches!(
        call(&changed, &image),
        Err(NativeLoadError::HelperImageChanged { .. })
    ));
}
