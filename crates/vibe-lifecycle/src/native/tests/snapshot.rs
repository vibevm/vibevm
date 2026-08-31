use super::*;

use vibe_core::lifecycle::ExtensionPoint;
use vibe_native_loader::{NativeInvocation, NativeLoader};
use vibe_wire::generated::native::e1::context::Context;

use crate::native::path::publish_load_image;

fn write_plugin(root: &Path, message: &str) -> Result<(), Box<dyn std::error::Error>> {
    let vibe_ext = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or_else(|| std::io::Error::other("fixture manifest has no parent"))?
        .join("vibe-ext")
        .display()
        .to_string()
        .replace('\\', "/");
    fs::create_dir_all(root.join("native/src"))?;
    fs::write(
        root.join("native/Cargo.toml"),
        format!(
            "[package]\nname='snapshot-fixture'\nversion='0.1.0'\nedition='2024'\n\n[lib]\ncrate-type=['cdylib']\n\n[dependencies]\nvibe-ext={{path={vibe_ext:?}}}\n"
        ),
    )?;
    fs::write(
        root.join("native/src/lib.rs"),
        format!(
            r#"use vibe_ext::{{Context, Manifest, ManifestExtension, Reply, ReplyStatus}};

fn handle(_context: Context) -> Reply {{
    Reply {{
        artifacts: Vec::new(),
        envelope: 1,
        status: ReplyStatus::Ok,
        message: Some({message:?}.to_owned()),
    }}
}}

vibe_ext::vibe_extension!(
    manifest = Manifest {{
        extensions: vec![ManifestExtension {{
            id: "first".to_owned(),
            point: "phase:build".to_owned(),
            ir_schema: None,
        }}],
    }},
    handler = handle,
);
"#
        ),
    )?;
    Ok(())
}

fn invocation_context(root: &Path) -> Result<Context, serde_json::Error> {
    let root = root.display().to_string().replace('\\', "/");
    serde_json::from_value(serde_json::json!({
        "artifacts": [],
        "envelope": 1,
        "execution": {"id": "first", "package": "__host__/snapshot", "config": {}},
        "io": {"scratch": format!("{root}/.vibe/lifecycle/run/first")},
        "point": "phase:build",
        "project": {
            "root": root, "name": "snapshot", "version": "0.1.0", "kind": "project",
            "manifest": "vibe.toml", "spec_roots": []
        },
        "run": {
            "requested": "build", "chain": ["build"], "phase": "build",
            "offline": true, "assume_yes": true, "agent_mode": "cli", "force": false
        },
        "world": {"lockfile": "vibe.lock", "deps_root": "vibedeps", "packages": []}
    }))
}

fn invoke(loader: &NativeLoader, path: &Path, context: &Context) -> Result<String, String> {
    loader
        .invoke(NativeInvocation {
            library: path,
            extension_id: "first",
            point: "phase:build"
                .parse::<ExtensionPoint>()
                .map_err(|error| error.to_string())?,
            ir_schema: None,
            context,
        })
        .map_err(|error| error.to_string())?
        .message
        .ok_or_else(|| "fixture reply has no message".to_owned())
}

#[test]
fn same_loader_observes_rebuilt_bytes_at_a_new_load_image_path() {
    let root = tempdir().unwrap();
    write_plugin(root.path(), "version A").unwrap();
    let declaration = native_decl("first", Some("native"), None, None);
    let owned = world(root.path(), vec![declaration]);
    let extensions = collect_extensions(owned.clone()).unwrap();
    let mechanisms = collect_mechanisms(&owned).unwrap();
    let candidates = extensions.rows().iter().collect::<Vec<_>>();
    let routes = MechanismRoutes::default();
    let execution = NativeBuildExecution {
        candidates: &candidates,
        selected_project_root: root.path(),
        registry: &mechanisms,
        routes: &routes,
        platform: current_platform(),
        offline: true,
        created_at: "2026-08-31T00:00:00Z",
    };
    build_native_sources(&execution).unwrap();
    let first = resolve_native_artifact(&execution, candidates[0]).unwrap();
    let first_path = publish_load_image(
        root.path(),
        Path::new(&first.path_absolute),
        &first.digest,
        first.bytes,
    )
    .unwrap();
    assert!(
        first_path.starts_with(
            root.path()
                .canonicalize()
                .unwrap()
                .join(".vibe/native-load/e1")
        )
    );
    let context = invocation_context(root.path()).unwrap();
    let loader = NativeLoader::new();
    assert_eq!(invoke(&loader, &first_path, &context).unwrap(), "version A");

    std::thread::sleep(Duration::from_millis(1_100));
    write_plugin(root.path(), "version B has distinct bytes").unwrap();
    build_native_sources(&execution).unwrap();
    let second = resolve_native_artifact(&execution, candidates[0]).unwrap();
    let second_path = publish_load_image(
        root.path(),
        Path::new(&second.path_absolute),
        &second.digest,
        second.bytes,
    )
    .unwrap();

    assert_ne!(first.digest, second.digest);
    assert_eq!(first.path_absolute, second.path_absolute);
    assert_ne!(first_path, second_path);
    assert!(first_path.is_file(), "loaded version A remains immutable");
    assert_eq!(
        invoke(&loader, &second_path, &context).unwrap(),
        "version B has distinct bytes"
    );
}

#[test]
fn resolver_source_path_has_no_toolchain_or_build_process_call() {
    let source = include_str!("../mod.rs");
    let resolver = source
        .split_once("pub fn resolve_native_artifact")
        .unwrap()
        .1
        .split_once("fn source_groups")
        .unwrap()
        .0;
    assert!(
        !resolver.contains("toolchain("),
        "resolver must not probe Cargo/rustc"
    );
    assert!(
        !resolver.contains("build_cdylib("),
        "resolver must not build lazily"
    );
}

#[cfg(unix)]
#[test]
fn existing_load_image_symlink_is_refused_even_inside_project() {
    use std::os::unix::fs::symlink;

    let root = tempdir().unwrap();
    let source = root
        .path()
        .join(format!("plugin{}", std::env::consts::DLL_SUFFIX));
    fs::write(&source, b"native bytes").unwrap();
    let (digest, bytes) = crate::mechanism::contain::digest_file(&source).unwrap();
    let directory = root.path().join(".vibe/native-load/e1").join(&digest);
    fs::create_dir_all(&directory).unwrap();
    symlink(&source, directory.join(source.file_name().unwrap())).unwrap();

    assert!(matches!(
        publish_load_image(root.path(), &source, &digest, bytes),
        Err(NativeArtifactError::LoadImage { .. })
    ));
}
