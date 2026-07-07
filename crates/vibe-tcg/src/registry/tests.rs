//! Registry behavioural oracles: dispatch through doubles,
//! respawn-once, consent refusals, per-language recipes — no
//! process spawns anywhere (the tests-out split keeps the cell
//! inside the §2 position budget).

use super::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[cfg(test)]
struct H(std::path::PathBuf);
#[cfg(test)]
impl TcgHost for H {
    fn project_root(&self) -> &Path {
        &self.0
    }
}

#[cfg(test)]
/// A scripted link: answers, or dies once.
struct DoubleLink {
    die_first: bool,
    calls: Arc<AtomicUsize>,
}
#[cfg(test)]
impl OracleLink for DoubleLink {
    fn request(&mut self, frame: serde_json::Value) -> Result<serde_json::Value, TcgError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.die_first {
            self.die_first = false;
            return Err(TcgError::OracleGone {
                language: "typescript".to_string(),
                binary: "tcg-typescript",
                detail: "scripted death".to_string(),
            });
        }
        Ok(serde_json::json!({
            "echo_op": frame["op"],
            "echo_file": frame["params"]["file"],
        }))
    }
}

#[cfg(test)]
/// A fixture project whose lockfile declares the RUST stack with a
/// pre-"built" tcg-rust artifact — the twin of the TS fixture, so
/// dispatch is proven per language, not per accident.
fn fixture_rust_project_with_artifact() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("vibe.toml"),
        "[project]\nname=\"x\"\nversion=\"0.0.1\"\n",
    )
    .expect("vibe.toml");
    std::fs::write(
        dir.path().join("vibe.lock"),
        r#"
[meta]
generated_by = "vibe-test"
generated_at = "2026-07-07T00:00:00Z"
schema_version = 5

[[package]]
kind = "stack"
group = "org.vibevm"
name = "rust-ai-native"
version = "0.5.0"
registry = "vibespecs"
source_url = "file://packages"
source_ref = "v0.5.0"
content_hash = "sha256:deadbeef"
files_written = []
"#,
    )
    .expect("vibe.lock");
    let slot = dir
        .path()
        .join("vibedeps")
        .join("stack-rust-ai-native")
        .join("0.5.0");
    let release = slot.join("target").join("release");
    std::fs::create_dir_all(&release).expect("release dir");
    std::fs::write(
        slot.join("vibe.toml"),
        r#"[package]
name = "rust-ai-native"
group = "org.vibevm"
kind = "stack"
version = "0.5.0"
authors = ["x"]
license = "EULA"
description = "fixture"
keywords = []

[[binary]]
name = "tcg-rust"
crate = "crates/tcg-cli-rust"
"#,
    )
    .expect("slot manifest");
    let artifact = release.join(if cfg!(windows) {
        "tcg-rust.exe"
    } else {
        "tcg-rust"
    });
    std::fs::write(&artifact, b"fake").expect("artifact");
    dir
}

#[cfg(test)]
fn fixture_project_with_artifact() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("vibe.toml"),
        "[project]\nname=\"x\"\nversion=\"0.0.1\"\n",
    )
    .expect("vibe.toml");
    std::fs::write(
        dir.path().join("vibe.lock"),
        r#"
[meta]
generated_by = "vibe-test"
generated_at = "2026-07-07T00:00:00Z"
schema_version = 5

[[package]]
kind = "stack"
group = "org.vibevm"
name = "typescript-ai-native"
version = "0.4.0"
registry = "vibespecs"
source_url = "file://packages"
source_ref = "v0.4.0"
content_hash = "sha256:deadbeef"
files_written = []
"#,
    )
    .expect("vibe.lock");
    let slot = dir
        .path()
        .join("vibedeps")
        .join("stack-typescript-ai-native")
        .join("0.4.0");
    let release = slot.join("target").join("release");
    std::fs::create_dir_all(&release).expect("release dir");
    std::fs::write(
        slot.join("vibe.toml"),
        r#"[package]
name = "typescript-ai-native"
group = "org.vibevm"
kind = "stack"
version = "0.4.0"
authors = ["x"]
license = "EULA"
description = "fixture"
keywords = []

[[binary]]
name = "tcg-typescript"
crate = "crates/tcg-cli-typescript"
"#,
    )
    .expect("slot manifest");
    // a pre-"built" artifact so resolve_artifact never runs cargo
    let artifact = release.join(if cfg!(windows) {
        "tcg-typescript.exe"
    } else {
        "tcg-typescript"
    });
    std::fs::write(&artifact, b"fake").expect("artifact");
    dir
}

#[test]
fn requests_relay_through_the_spawned_link() {
    let dir = fixture_project_with_artifact();
    let host = H(dir.path().to_path_buf());
    let calls = Arc::new(AtomicUsize::new(0));
    let calls_in = calls.clone();
    let registry = OracleRegistry::with_spawner(Box::new(move |_l, _a, _r| {
        Ok(Box::new(DoubleLink {
            die_first: false,
            calls: calls_in.clone(),
        }) as Box<dyn OracleLink>)
    }));
    let out = registry
        .request(
            "typescript",
            &host,
            "validate",
            serde_json::json!({"file": "src/a.ts"}),
        )
        .expect("relayed");
    assert_eq!(out["echo_op"], "validate");
    assert_eq!(out["echo_file"], "src/a.ts");
    // second call reuses the SAME link (lazy, persistent)
    let _ = registry
        .request(
            "typescript",
            &host,
            "scope",
            serde_json::json!({"file": "src/a.ts"}),
        )
        .expect("relayed again");
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[test]
fn a_dead_link_is_respawned_exactly_once() {
    let dir = fixture_project_with_artifact();
    let host = H(dir.path().to_path_buf());
    let spawns = Arc::new(AtomicUsize::new(0));
    let spawns_in = spawns.clone();
    let registry = OracleRegistry::with_spawner(Box::new(move |_l, _a, _r| {
        let n = spawns_in.fetch_add(1, Ordering::SeqCst);
        Ok(Box::new(DoubleLink {
            die_first: n == 0, // the first link dies on its first request
            calls: Arc::new(AtomicUsize::new(0)),
        }) as Box<dyn OracleLink>)
    }));
    let out = registry
        .request(
            "typescript",
            &host,
            "type",
            serde_json::json!({"file": "src/a.ts"}),
        )
        .expect("survived one death");
    assert_eq!(out["echo_op"], "type");
    assert_eq!(spawns.load(Ordering::SeqCst), 2, "exactly one respawn");
}

#[test]
fn rust_requests_dispatch_through_their_own_binary() {
    let dir = fixture_rust_project_with_artifact();
    let host = H(dir.path().to_path_buf());
    let seen = Arc::new(Mutex::new(Vec::new()));
    let seen_in = seen.clone();
    let calls = Arc::new(AtomicUsize::new(0));
    let calls_in = calls.clone();
    let registry = OracleRegistry::with_spawner(Box::new(move |language, artifact, _r| {
        seen_in
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push((language.to_string(), artifact.to_path_buf()));
        Ok(Box::new(DoubleLink {
            die_first: false,
            calls: calls_in.clone(),
        }) as Box<dyn OracleLink>)
    }));
    let out = registry
        .request(
            "rust",
            &host,
            "validate",
            serde_json::json!({"file": "src/lib.rs"}),
        )
        .expect("relayed");
    assert_eq!(out["echo_op"], "validate");
    let seen = seen
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].0, "rust");
    assert!(
        seen[0].1.to_string_lossy().contains("tcg-rust"),
        "the rust row resolves ITS binary: {}",
        seen[0].1.display()
    );
}

#[test]
fn absent_stacks_name_their_own_language_recipe() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("vibe.toml"),
        "[project]\nname=\"x\"\nversion=\"0.0.1\"\n",
    )
    .expect("vibe.toml");
    let host = H(dir.path().to_path_buf());
    let registry = OracleRegistry::with_spawner(Box::new(|_l, _a, _r| {
        panic!("must not spawn without a resolved artifact")
    }));
    let ts_err = registry
        .request("typescript", &host, "validate", serde_json::json!({}))
        .expect_err("ts not installed");
    assert!(
        ts_err.to_string().contains("typescript-ai-native"),
        "{ts_err}"
    );
    let rust_err = registry
        .request("rust", &host, "validate", serde_json::json!({}))
        .expect_err("rust not installed");
    assert!(
        rust_err.to_string().contains("rust-ai-native\" = \"^0.5"),
        "the rust refusal names the RUST requires line: {rust_err}"
    );
    assert!(
        !rust_err.to_string().contains("typescript-ai-native"),
        "never another language's fix surface: {rust_err}"
    );
}

#[test]
fn stack_absent_names_the_requires_line() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("vibe.toml"),
        "[project]\nname=\"x\"\nversion=\"0.0.1\"\n",
    )
    .expect("vibe.toml");
    let host = H(dir.path().to_path_buf());
    let registry = OracleRegistry::with_spawner(Box::new(|_l, _a, _r| {
        panic!("must not spawn without a resolved artifact")
    }));
    let err = registry
        .request(
            "typescript",
            &host,
            "validate",
            serde_json::json!({"file": "src/a.ts"}),
        )
        .expect_err("not installed");
    assert!(matches!(err, TcgError::StackNotInstalled { .. }));
    assert!(err.to_string().contains("vibe install"), "{err}");
}

#[test]
fn third_party_unbuilt_is_refused_with_the_recipe() {
    let dir = fixture_project_with_artifact();
    // make the package foreign and remove the artifact
    let slot = dir
        .path()
        .join("vibedeps")
        .join("stack-typescript-ai-native")
        .join("0.4.0");
    let manifest = std::fs::read_to_string(slot.join("vibe.toml")).expect("read");
    std::fs::write(
        slot.join("vibe.toml"),
        manifest.replace("group = \"org.vibevm\"", "group = \"com.example\""),
    )
    .expect("rewrite");
    std::fs::remove_file(slot.join("target").join("release").join(if cfg!(windows) {
        "tcg-typescript.exe"
    } else {
        "tcg-typescript"
    }))
    .expect("rm artifact");
    // the lockfile group must match the manifest group for the walk
    let lock = std::fs::read_to_string(dir.path().join("vibe.lock")).expect("lock");
    std::fs::write(
        dir.path().join("vibe.lock"),
        lock.replace("group = \"org.vibevm\"", "group = \"com.example\""),
    )
    .expect("rewrite lock");

    let host = H(dir.path().to_path_buf());
    let registry = OracleRegistry::with_spawner(Box::new(|_l, _a, _r| {
        panic!("must not spawn an unbuilt third-party binary")
    }));
    let err = registry
        .request(
            "typescript",
            &host,
            "validate",
            serde_json::json!({"file": "src/a.ts"}),
        )
        .expect_err("refused");
    assert!(matches!(err, TcgError::NotBuiltThirdParty { .. }));
    assert!(err.to_string().contains("--assume-yes"), "{err}");
}
