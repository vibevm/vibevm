//! Project preview must enumerate the same selected closure without publication.
//! These are command-level gates; acquisition observations need not equal apply.
mod common;

use common::UserScratch;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[path = "cli_project_preview/support.rs"]
mod support;
use support::*;

#[test]
#[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#OPERATION-EFFECTS")]
fn cold_project_preview_reports_full_closure_without_writing_any_persistent_root() {
    let fixture = Fixture::new();
    let before = fixture.snapshot();
    let docs = fixture.invoke(true, true);
    assert_eq!(
        before,
        fixture.snapshot(),
        "preview changed a persistent root"
    );
    let value = plan(&docs);
    let _: vibe_wire::generated::install::e2::plan::InstallPlan =
        serde_json::from_value(value.clone()).expect("shared generated plan vocabulary");
    for package in value["packages"].as_array().unwrap() {
        let candidates = package["source_candidates"]
            .as_array()
            .expect("candidate observations are present");
        let chosen = candidates
            .iter()
            .find(|candidate| candidate["decision"] == "selected")
            .expect("selected source has an explicit decision");
        assert_eq!(chosen["observation"], "available");
        assert_eq!(
            chosen["source_evidence"]["registry"],
            package["source_evidence"]["registry"]
        );
    }
    let selected = selection(value);
    assert_eq!(
        selected.keys().map(String::as_str).collect::<Vec<_>>(),
        ["org.example/child", "org.example/root"]
    );
    for facts in selected.values() {
        assert_eq!(facts["registry"], "selected");
        assert_eq!(facts["source_kind"], "registry");
    }
    assert!(
        docs.iter().all(|doc| doc["command"] != "install"),
        "preview claimed apply completion"
    );
    assert!(!fixture.project.join("vibe.lock").exists());
}

#[test]
fn preview_and_real_install_use_the_same_selected_source_rows() {
    let fixture = Fixture::new();
    let preview = fixture.invoke(true, true);
    let applied = fixture.invoke(false, true);
    assert_eq!(selection(plan(&preview)), selection(plan(&applied)));
    // Do not compare acquisition/store_reused: preview never asserts a payload
    // acquisition merely because metadata was read from the same source.
}

#[test]
fn fresh_and_cached_project_preview_still_returns_the_entire_locked_closure() {
    let fixture = Fixture::new();
    let applied = fixture.invoke(false, true);
    let selected = selection(plan(&applied));
    // Remove the registry payload, forcing the no-network warm route.
    fs::remove_dir_all(fixture.registry.join("org.example")).unwrap();
    let before = fixture.snapshot();
    let preview = fixture.invoke(true, false);
    assert_eq!(before, fixture.snapshot());
    assert_eq!(selection(plan(&preview)), selected);
}

#[test]
fn empty_project_preview_is_a_complete_empty_plan_without_creating_lifecycle_state() {
    let fixture = Fixture::new();
    let before = fixture.snapshot();
    let preview = fixture.invoke(true, false);
    assert_eq!(before, fixture.snapshot());
    assert_eq!(plan(&preview)["packages"], serde_json::json!([]));
}

#[test]
fn failed_preview_cleans_owned_scratch_without_recording_partial_install() {
    let fixture = Fixture::new();
    fs::write(
        fixture.registry.join("org.example/child/v1.0.0/vibe.toml"),
        "this is not a manifest",
    )
    .unwrap();
    let before = fixture.snapshot();
    let output = fixture
        .user
        .vibe()
        .args([
            "install",
            "org.example/root@=1.0.0",
            "--plan-only",
            "--json",
            "--offline",
            "--path",
        ])
        .arg(&fixture.project)
        .env("HOME", &fixture.home)
        .env("USERPROFILE", &fixture.home)
        .env("CODEX_HOME", fixture.home.join(".codex"))
        .env("CLAUDE_CONFIG_DIR", fixture.home.join(".claude"))
        .env("XDG_CONFIG_HOME", fixture.home.join(".config"))
        .env("XDG_DATA_HOME", fixture.home.join(".local/share"))
        .env("XDG_CACHE_HOME", fixture.home.join(".cache"))
        .env("XDG_STATE_HOME", fixture.home.join(".local/state"))
        .env("XDG_RUNTIME_DIR", fixture.home.join(".runtime"))
        .env("TMPDIR", fixture.root.path().join("temporary"))
        .env("TMP", fixture.root.path().join("temporary"))
        .env("TEMP", fixture.root.path().join("temporary"))
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(before, fixture.snapshot());
}

// The shared sshd fixture uses the audited Linux installation paths.
#[cfg(target_os = "linux")]
#[path = "cli_project_preview/ssh.rs"]
mod ssh;

#[test]
fn requested_cancellation_cleans_preview_scratch_and_preserves_stop_input() {
    let fixture = Fixture::new();
    let stop = fixture.root.path().join("stop-preview");
    fs::write(&stop, "stop\n").unwrap();
    let before = fixture.snapshot();
    let output = fixture
        .user
        .vibe()
        .arg("--stop-file")
        .arg(&stop)
        .args([
            "install",
            "org.example/root@=1.0.0",
            "--plan-only",
            "--json",
            "--offline",
            "--path",
        ])
        .arg(&fixture.project)
        .env("HOME", &fixture.home)
        .env("USERPROFILE", &fixture.home)
        .env("CODEX_HOME", fixture.home.join(".codex"))
        .env("CLAUDE_CONFIG_DIR", fixture.home.join(".claude"))
        .env("XDG_CONFIG_HOME", fixture.home.join(".config"))
        .env("XDG_DATA_HOME", fixture.home.join(".local/share"))
        .env("XDG_CACHE_HOME", fixture.home.join(".cache"))
        .env("XDG_STATE_HOME", fixture.home.join(".local/state"))
        .env("XDG_RUNTIME_DIR", fixture.home.join(".runtime"))
        .env("TMPDIR", fixture.root.path().join("temporary"))
        .env("TMP", fixture.root.path().join("temporary"))
        .env("TEMP", fixture.root.path().join("temporary"))
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(130),
        "cancel request was ignored"
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["protocol"], "vibe-project-cancellation/1");
    assert_eq!(report["cache_writes"], "none");
    assert_eq!(before, fixture.snapshot());
}

#[cfg(unix)]
#[test]
fn preview_accepts_a_read_only_project_home_and_persistent_caches() {
    use std::os::unix::fs::PermissionsExt;
    struct Restore(Vec<(PathBuf, fs::Permissions)>);
    impl Drop for Restore {
        fn drop(&mut self) {
            for (path, permissions) in self.0.iter().rev() {
                fs::set_permissions(path, permissions.clone()).unwrap();
            }
        }
    }
    fn protect(path: &Path, restore: &mut Restore) {
        let metadata = fs::symlink_metadata(path).unwrap();
        if metadata.is_dir() {
            for entry in fs::read_dir(path).unwrap() {
                protect(&entry.unwrap().path(), restore);
            }
        }
        restore.0.push((path.to_owned(), metadata.permissions()));
        fs::set_permissions(
            path,
            fs::Permissions::from_mode(metadata.permissions().mode() & !0o222),
        )
        .unwrap();
    }
    let fixture = Fixture::new();
    let before = fixture.snapshot();
    let mut restore = Restore(Vec::new());
    for path in [
        &fixture.project,
        &fixture.home,
        &fixture.registry,
        &fixture.user.settings,
        &fixture.user.cache,
        &fixture.user.search_cache,
    ] {
        protect(path, &mut restore);
    }
    let docs = fixture.invoke(true, true);
    assert_eq!(plan(&docs)["packages"].as_array().unwrap().len(), 2);
    assert_eq!(before, fixture.snapshot());
    drop(restore);
}

#[test]
fn required_remap_is_reported_as_an_offer_without_registration_or_activation() {
    let fixture = Fixture::new();
    let package = fixture.registry.join("org.example/mirror/v1.0.0");
    fs::create_dir_all(&package).unwrap();
    fs::write(package.join("vibe.toml"), "[package]\ngroup='org.example'\nname='mirror'\nversion='1.0.0'\nkind='remap'\n[[remap]]\nprefix='https://example.invalid/'\nurl='https://mirror.invalid/'\n").unwrap();
    let manifest = fixture.project.join("vibe.toml");
    fs::write(
        &manifest,
        format!(
            "{}\n[requires.remaps]\n'org.example/mirror'='=1.0.0'\n",
            fs::read_to_string(&manifest).unwrap()
        ),
    )
    .unwrap();
    let before = fixture.snapshot();
    let docs = fixture.invoke(true, false);
    assert_eq!(before, fixture.snapshot());
    let report = plan(&docs);
    assert_eq!(report["packages"], serde_json::json!([]));
    assert_eq!(report["preview_complete"], true);
    let row = &report["remaps"][0];
    assert_eq!(row["package"], "org.example/mirror");
    assert_eq!(row["outcome"], "offer-only");
    assert_eq!(row["source_evidence"]["registry"], "selected");
}

#[test]
fn legacy_remap_without_locked_endpoint_is_missing_input_not_catalog_fallback() {
    let fixture = Fixture::new();
    let manifest = fixture.project.join("vibe.toml");
    fs::write(
        &manifest,
        format!(
            "{}\n[requires.remaps]\n'org.example/mirror'='=1.0.0'\n",
            fs::read_to_string(&manifest).unwrap()
        ),
    )
    .unwrap();
    let mut locked = vibe_core::manifest::Lockfile::empty("fixture", "2026-11-08T00:00:00Z");
    locked.remaps.push(toml::from_str("package='org.example/mirror'\nversion='1.0.0'\ncontent_hash='sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'\nregistry='selected'\n").unwrap());
    locked.write(fixture.project.join("vibe.lock")).unwrap();
    let before = fixture.snapshot();
    let docs = fixture.invoke(true, false);
    assert_eq!(before, fixture.snapshot());
    let report = plan(&docs);
    assert_eq!(report["preview_complete"], false);
    assert_eq!(report["remaps"][0]["outcome"], "needs-input");
    assert_eq!(
        report["remaps"][0]["input_required"],
        "missing-locked-source"
    );
    assert!(report["remaps"][0]["source_evidence"]["source_url"].is_null());
}

#[test]
fn conditional_metadata_uses_the_same_historical_activation_before_acquisition() {
    let fixture = Fixture::new();
    let root = fixture.registry.join("org.example/root/v1.0.0/vibe.toml");
    let additional = r#"
[target."context(not tool:later)".dependencies.packages]
'org.example/later'='=1.0.0'
[target."context(tool:later)".dependencies.packages]
'org.example/tail'='=1.0.0'
[target."context(tool:never)".dependencies.packages]
'org.example/unavailable'='=1.0.0'
"#;
    fs::write(
        &root,
        format!("{}{}", fs::read_to_string(&root).unwrap(), additional),
    )
    .unwrap();
    for name in ["later", "tail"] {
        let slot = fixture.registry.join(format!("org.example/{name}/v1.0.0"));
        fs::create_dir_all(&slot).unwrap();
        fs::write(
            slot.join("vibe.toml"),
            format!(
                "[package]\ngroup='org.example'\nname='{name}'\nversion='1.0.0'\nkind='tool'\n"
            ),
        )
        .unwrap();
    }
    let before = fixture.snapshot();
    let preview = fixture.invoke(true, true);
    assert_eq!(before, fixture.snapshot());
    let selected = selection(plan(&preview));
    assert_eq!(
        selected.keys().map(String::as_str).collect::<Vec<_>>(),
        [
            "org.example/child",
            "org.example/later",
            "org.example/root",
            "org.example/tail"
        ]
    );
    let installed = fixture.invoke(false, true);
    assert_eq!(selected, selection(plan(&installed)));
}

#[test]
fn preview_does_not_dispatch_hooks_or_compile_boot() {
    let fixture = Fixture::new();
    let package = fixture.registry.join("org.example/root/v1.0.0");
    let manifest_path = package.join("vibe.toml");
    let manifest = fs::read_to_string(&manifest_path)
        .unwrap()
        .replace("kind='tool'", "kind='flow'\nformat='normal'");
    let snippet = common::boot_rel("preview.md");
    fs::create_dir_all(package.join(&snippet).parent().unwrap()).unwrap();
    fs::write(
        package.join(&snippet),
        "# Contract\n\n#use spec://org.missing/absent/flows/missing#root\n",
    )
    .unwrap();
    fs::create_dir_all(package.join("hooks")).unwrap();
    fs::write(
        package.join("hooks/canary"),
        "This hook must never be dispatched by a preview.\n",
    )
    .unwrap();
    fs::write(&manifest_path, format!("{manifest}\n[boot_snippet]\nsource='{snippet}'\nlink='static'\n[hooks]\npre-install='hooks/canary'\n")).unwrap();
    let before = fixture.snapshot();
    let docs = fixture.invoke(true, true);
    assert_eq!(plan(&docs)["preview_complete"], true);
    assert_eq!(before, fixture.snapshot());
}

#[test]
fn incompatible_held_request_never_contacts_the_edited_current_registry() {
    use std::io::Write;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    };
    let fixture = Fixture::new();
    fixture.invoke(false, true);
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let contacts = Arc::new(AtomicUsize::new(0));
    let done = Arc::new(AtomicBool::new(false));
    let worker_contacts = Arc::clone(&contacts);
    let worker_done = Arc::clone(&done);
    let server = std::thread::spawn(move || {
        while !worker_done.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    worker_contacts.fetch_add(1, Ordering::SeqCst);
                    let _ = stream.write_all(
                        b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    );
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
                Err(error) => panic!("fixture accept: {error}"),
            }
        }
    });
    fs::write(fixture.project.join("vibe.toml"), format!(
        "[project]\nname='preview'\nversion='1.0.0'\n[requires.packages]\n'org.example/root'='=1.0.0'\n[[registry]]\nname='selected'\nurl='{endpoint}/registry'\nindex_url='{endpoint}/index'\n"
    )).unwrap();
    let before = fixture.snapshot();
    let output = fixture
        .user
        .vibe()
        .args([
            "install",
            "org.example/root@=2.0.0",
            "--plan-only",
            "--json",
            "--no-default-registry",
            "--path",
        ])
        .arg(&fixture.project)
        .env("HOME", &fixture.home)
        .env("USERPROFILE", &fixture.home)
        .env("CODEX_HOME", fixture.home.join(".codex"))
        .env("CLAUDE_CONFIG_DIR", fixture.home.join(".claude"))
        .env("XDG_CONFIG_HOME", fixture.home.join(".config"))
        .env("XDG_DATA_HOME", fixture.home.join(".local/share"))
        .env("XDG_CACHE_HOME", fixture.home.join(".cache"))
        .env("XDG_STATE_HOME", fixture.home.join(".local/state"))
        .env("XDG_RUNTIME_DIR", fixture.home.join(".runtime"))
        .env("VIBE_NO_DEFAULT_REGISTRY", "1")
        .env("NO_PROXY", "127.0.0.1,localhost")
        .env_remove("HTTP_PROXY")
        .env_remove("HTTPS_PROXY")
        .env_remove("ALL_PROXY")
        .env("TMPDIR", fixture.root.path().join("temporary"))
        .env("TMP", fixture.root.path().join("temporary"))
        .env("TEMP", fixture.root.path().join("temporary"))
        .output()
        .unwrap();
    done.store(true, Ordering::SeqCst);
    server.join().unwrap();
    assert!(
        !output.status.success(),
        "ordinary preview released the held version"
    );
    let diagnostic = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
    .to_ascii_lowercase();
    assert!(
        diagnostic.contains("locked") || diagnostic.contains("held"),
        "failure did not identify held authority: {diagnostic}"
    );
    assert_eq!(
        contacts.load(Ordering::SeqCst),
        0,
        "current registry was contacted despite held authority"
    );
    assert_eq!(before, fixture.snapshot());
}

#[test]
fn active_conditional_constraint_on_existing_target_must_not_be_dropped() {
    let fixture = Fixture::new();
    let root = fixture.registry.join("org.example/root/v1.0.0/vibe.toml");
    fs::write(&root, format!("{}\n[target.\"context(tool:root)\".dependencies.packages]\n'org.example/child'='=2.0.0'\n",
        fs::read_to_string(&root).unwrap())).unwrap();
    let before = fixture.snapshot();
    let output = fixture
        .user
        .vibe()
        .args([
            "install",
            "org.example/root@=1.0.0",
            "--plan-only",
            "--json",
            "--offline",
            "--path",
        ])
        .arg(&fixture.project)
        .env("HOME", &fixture.home)
        .env("USERPROFILE", &fixture.home)
        .env("CODEX_HOME", fixture.home.join(".codex"))
        .env("CLAUDE_CONFIG_DIR", fixture.home.join(".claude"))
        .env("XDG_CONFIG_HOME", fixture.home.join(".config"))
        .env("XDG_DATA_HOME", fixture.home.join(".local/share"))
        .env("XDG_CACHE_HOME", fixture.home.join(".cache"))
        .env("XDG_STATE_HOME", fixture.home.join(".local/state"))
        .env("XDG_RUNTIME_DIR", fixture.home.join(".runtime"))
        .env("TMPDIR", fixture.root.path().join("temporary"))
        .env("TMP", fixture.root.path().join("temporary"))
        .env("TEMP", fixture.root.path().join("temporary"))
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "active =2 constraint disappeared because child@1 was already present"
    );
    assert_eq!(before, fixture.snapshot());
    let diagnostic = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        diagnostic.contains("unsatisfiable")
            || diagnostic.contains("constraint")
            || diagnostic.contains("conflict"),
        "expected a real constraint conflict: {diagnostic}"
    );
}

#[test]
fn compatible_conditional_constraint_intersects_the_existing_target_range() {
    let fixture = Fixture::new();
    let root = fixture.registry.join("org.example/root/v1.0.0/vibe.toml");
    fs::write(&root, format!("{}\n[target.\"context(tool:root)\".dependencies.packages]\n'org.example/child'='>=1, <2'\n",
        fs::read_to_string(&root).unwrap())).unwrap();
    let before = fixture.snapshot();
    let preview = fixture.invoke(true, true);
    assert_eq!(before, fixture.snapshot());
    let rows = selection(plan(&preview));
    assert_eq!(rows["org.example/child"]["version"], "1.0.0");
    let applied = fixture.invoke(false, true);
    assert_eq!(rows, selection(plan(&applied)));
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#OPERATION-EFFECTS-LAW")]
fn cold_current_lock_without_configuration_rows_classifies_only_in_preview_scratch() {
    for schema in [
        vibe_core::manifest::CONFIGURATION_SCHEMA_VERSION,
        vibe_core::manifest::CURRENT_SCHEMA_VERSION,
    ] {
        let fixture = Fixture::new();
        fixture.invoke(false, true);
        let path = fixture
            .project
            .join(vibe_core::manifest::Lockfile::FILENAME);
        let mut lock = vibe_core::manifest::Lockfile::read(&path).unwrap();
        assert_eq!(
            lock.meta.schema_version,
            vibe_core::manifest::LEGACY_SCHEMA_VERSION
        );
        assert!(!lock.packages.is_empty());
        lock.configurations.clear();
        lock.meta.schema_version = schema;
        fs::write(&path, toml::to_string(&lock).unwrap()).unwrap();
        assert_eq!(
            vibe_core::manifest::Lockfile::read(&path)
                .unwrap()
                .meta
                .schema_version,
            schema
        );
        let slots = fixture
            .project
            .join(vibe_core::layout::current_vibedeps_root());
        if slots.exists() {
            fs::remove_dir_all(&slots).unwrap();
        }
        let store = fixture.user.settings.join("cache");
        if store.exists() {
            fs::remove_dir_all(&store).unwrap();
        }
        assert!(!store.exists());
        assert!(!slots.exists());
        let before = fixture.snapshot();
        let docs = fixture.invoke(true, false);
        assert!(!plan(&docs)["packages"].as_array().unwrap().is_empty());
        assert_eq!(
            before,
            fixture.snapshot(),
            "retained configuration classification escaped preview scratch"
        );
        assert!(!store.exists());
    }
}
