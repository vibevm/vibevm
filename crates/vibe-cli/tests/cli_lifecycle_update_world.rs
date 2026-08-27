mod common;
mod trace_support;

use std::fs;
use std::path::{Path, PathBuf};

use common::{UserScratch, git_available, run_git, write_project_with_per_package_registry};
use trace_support::{index_of, run_directories, trace_dir, trace_member};
use vibe_wire::generated::compiler_trace_index::e1::index::RunStatus;

fn registry_url(registry: &Path) -> String {
    format!(
        "git+file://{}",
        registry.to_string_lossy().replace('\\', "/")
    )
}

/// Publish one source repo as a bare per-package registry entry.
///
/// `extension` adds the watching provider row; `boot` makes the package
/// BOOT-BEARING — a version-distinct `boot/40-target.md` plus the
/// `[boot_snippet]` declaration that puts it in the flow band — which is what
/// gives a STATICALLY linked consumer something to compile and a traced run
/// real scopes, events and snapshots to record.
fn publish_repo(
    registry: &Path,
    group: &str,
    name: &str,
    versions: &[&str],
    extension: bool,
    boot: bool,
) {
    let source = registry.join(format!("src-{name}"));
    fs::create_dir_all(source.join("hooks")).unwrap();
    if boot {
        fs::create_dir_all(source.join("boot")).unwrap();
        // The snippet is PARSED as spec Markdown on the consumer side, so pin
        // LF: a checkout that mangles line endings would make the fixture's
        // validity depend on the host's git configuration.
        fs::write(source.join(".gitattributes"), "* text=auto eol=lf\n").unwrap();
    }
    run_git(&source, &["init", "--initial-branch=main"]);
    run_git(&source, &["config", "user.email", "t@example.com"]);
    run_git(&source, &["config", "user.name", "Test"]);
    for version in versions {
        let extra = if extension {
            r#"
[[extension]]
id = "watch-target"
point = "slot:pre-install"
handler = { kind = "script", base = "hooks/watch" }
applies_to = { packages = ["org.world/target"] }
"#
        } else {
            ""
        };
        let boot_section = if boot {
            "\n[boot_snippet]\nsource = \"boot/40-target.md\"\ncategory = \"flow\"\n"
        } else {
            ""
        };
        fs::write(
            source.join("vibe.toml"),
            format!(
                "[package]\ngroup='{group}'\nname='{name}'\nkind='tool'\nversion='{version}'\n{extra}{boot_section}"
            ),
        )
        .unwrap();
        fs::write(source.join("payload.txt"), format!("{name}-{version}\n")).unwrap();
        if boot {
            // Version-DISTINCT on purpose: the 0.1.0 -> 0.2.0 bump must change
            // the compiled bytes, or a regeneration that silently kept the old
            // artifact would still fingerprint as fresh.
            fs::write(
                source.join("boot/40-target.md"),
                format!("# Target {{#root}}\n\nTARGET BOOT {version}\n"),
            )
            .unwrap();
        }
        if extension {
            fs::write(
                source.join("hooks/watch.sh"),
                "set -eu\ncp \"$VIBE_CONTEXT\" \"$VIBE_PROJECT_ROOT/.vibe/update-context.json\"\n",
            )
            .unwrap();
            fs::write(
                source.join("hooks/watch.ps1"),
                "Copy-Item -LiteralPath $env:VIBE_CONTEXT -Destination (Join-Path $env:VIBE_PROJECT_ROOT '.vibe/update-context.json')\n",
            )
            .unwrap();
        }
        run_git(&source, &["add", "-A"]);
        run_git(
            &source,
            &["commit", "-m", &format!("{group}/{name}@{version}")],
        );
        run_git(&source, &["tag", &format!("v{version}")]);
    }
    let bare = registry.join(format!("{group}.{name}.git"));
    run_git(
        registry,
        &[
            "clone",
            "--bare",
            source.to_str().unwrap(),
            bare.to_str().unwrap(),
        ],
    );
    run_git(&bare, &["symbolic-ref", "HEAD", "refs/heads/main"]);
}

fn setup() -> Option<(UserScratch, tempfile::TempDir, PathBuf)> {
    if !git_available() {
        return None;
    }
    let registry = tempfile::tempdir().unwrap().keep();
    publish_repo(&registry, "org.world", "provider", &["0.1.0"], true, false);
    publish_repo(
        &registry,
        "org.world",
        "target",
        &["0.1.0", "0.2.0"],
        false,
        true,
    );
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    write_project_with_per_package_registry(project.path(), &registry_url(&registry));
    user.vibe()
        .arg("install")
        .args(["org.world/provider@=0.1.0", "org.world/target@=0.1.0"])
        .arg("--path")
        .arg(project.path())
        .arg("--assume-yes")
        .assert()
        .success();
    assert!(
        !trace_dir(project.path()).exists(),
        "the seed is untraced and creates no trace run",
    );
    let manifest_path = project.path().join("vibe.toml");
    let mut manifest = vibe_core::manifest::Manifest::read(&manifest_path).unwrap();
    let target = manifest
        .requires
        .packages
        .iter_mut()
        .find(|package| package.name == "target")
        .unwrap();
    *target = vibe_core::PackageRef::parse("org.world/target@*").unwrap();
    // The STATIC link is what makes the widened `@*` a compilation rather
    // than a resolution-only move: a dynamically linked target contributes an
    // INDEX line and no compiled artifact, and the traced run below would
    // record nothing.
    manifest.requires.links.insert(
        "org.world/target".to_string(),
        vibe_core::manifest::LinkType::Static,
    );
    manifest.write(&manifest_path).unwrap();
    let mut text = fs::read_to_string(&manifest_path).unwrap();
    text.push_str(
        r#"
[[extensions.use]]
ref = "org.world/provider#watch-target"
"#,
    );
    fs::write(manifest_path, text).unwrap();
    Some((user, project, registry))
}

#[test]
fn scoped_update_uses_full_lock_world_and_unchanged_activated_provider() {
    let Some((user, project, _registry)) = setup() else {
        eprintln!("skipping: git unavailable");
        return;
    };
    let output = user
        .vibe()
        .args([
            "update",
            "org.world/target",
            "--json",
            "--trace-compile",
            "--path",
        ])
        .arg(project.path())
        .arg("--assume-yes")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let context: serde_json::Value = serde_json::from_slice(
        &fs::read(project.path().join(".vibe/update-context.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(context["execution"]["package"], "org.world/provider");
    assert_eq!(context["slot_target"]["name"], "target");
    let lock = vibe_core::manifest::Lockfile::read(project.path().join("vibe.lock")).unwrap();
    let lock_order = lock
        .packages
        .iter()
        .map(|package| package.name.as_str())
        .collect::<Vec<_>>();
    let envelope_order = context["world"]["packages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|package| package["name"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(envelope_order, lock_order);
    assert!(envelope_order.contains(&"provider"));
    let docs = serde_json::Deserializer::from_slice(&output.stdout)
        .into_iter::<serde_json::Value>()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    // The rows live on the ONE registered `update` root.
    //
    // They used to be read from standalone `{"command":"lifecycle"}` documents
    // — a per-row echo that put a second and third document on the same stdout,
    // and that R3.4 deliberately removed. The rows themselves did not go
    // anywhere: `vibe update` is the outermost command on this path, so its
    // single report is the only place either kind of row can be observed. This
    // still goes red if the Update owner drops the slot row, because that row
    // is now the report's own `contributions` member.
    let update_roots: Vec<&serde_json::Value> = docs
        .iter()
        .filter(|doc| doc["command"] == "update")
        .collect();
    assert_eq!(
        update_roots.len(),
        1,
        "EXACTLY one update root — a first rowful root followed by a rowless          duplicate would satisfy a `find`, and would be two documents where the          command owes one: {docs:#?}",
    );
    // No lifecycle echo and no standalone trace object ever comes back: the
    // removed per-row roots stay removed, and the member rides the one root.
    assert!(
        docs.iter().all(|doc| doc["command"] != "lifecycle"),
        "no standalone lifecycle document: {docs:#?}",
    );
    assert!(
        docs.iter()
            .all(|doc| doc.get("run_id").is_none() && doc["command"] != "compile-trace"),
        "no standalone trace document beside the root: {docs:#?}",
    );
    // The FINAL document, which is also that root: nothing may follow the one
    // registered report a command emits.
    let update_root = docs.last().expect("at least one document");
    assert_eq!(
        update_root["command"], "update",
        "and it is the LAST document on stdout: {docs:#?}",
    );
    let contributions = update_root["contributions"]
        .as_array()
        .expect("the Update root carries its contribution rows");
    assert_eq!(
        contributions.len(),
        1,
        "the unchanged activated provider is the root's only contribution: {update_root:#}",
    );
    let slot_rows = contributions
        .iter()
        .filter(|row| row["point"] == "slot:pre-install")
        .collect::<Vec<_>>();
    assert_eq!(
        slot_rows.len(),
        1,
        "unchanged provider emits no target event: {update_root:#}"
    );
    assert_eq!(slot_rows[0]["provider"], "org.world/provider");
    assert_eq!(slot_rows[0]["tier"], "host-activation");
    assert_eq!(update_root["hooks"], serde_json::json!([]));

    // ---- the SCOPED shape and the exact bump --------------------------
    assert_eq!(update_root["scope"], "scoped", "{update_root:#?}");
    assert_eq!(
        update_root["packages"],
        serde_json::json!(["org.world/target"])
    );
    assert_eq!(
        update_root["version_bumps"],
        serde_json::json!(["org.world/target 0.1.0 -> 0.2.0"]),
        "the widened `@*` moved exactly this package to exactly 0.2.0",
    );

    // ---- the traced run behind it -------------------------------------
    let trace = trace_member(update_root).expect("a traced scoped update carries its member");
    assert_eq!(trace["status"], "ok");
    assert_eq!(trace["finalised"], true, "{trace:#?}");
    let run_id = trace["run_id"].as_str().unwrap().to_string();
    assert_eq!(
        run_directories(project.path()),
        vec![run_id.clone()],
        "EXACTLY one run — the seed opened none and the update opened one",
    );

    let index = index_of(project.path(), &run_id);
    assert!(
        matches!(index.status, RunStatus::Ok),
        "the index is terminal Ok: {:?}",
        index.status,
    );
    assert!(
        !index.scopes.is_empty(),
        "the static target really compiled — the run has scopes",
    );
    assert!(!index.events.is_empty());
    // One dense global sequence: the proof this is one run rather than
    // several stitched together.
    let mut sequences: Vec<u32> = index.events.iter().map(|event| event.sequence).collect();
    sequences.sort_unstable();
    assert_eq!(
        sequences,
        (0..u32::try_from(sequences.len()).unwrap()).collect::<Vec<_>>(),
        "one dense global sequence: {sequences:?}",
    );
    // The member counts exactly what the index holds — no rounding, no drift.
    let snapshots = index
        .events
        .iter()
        .filter(|event| event.snapshot.is_some())
        .count();
    assert!(
        snapshots > 0,
        "the run really wrote certified snapshots, not just pass rows",
    );
    assert_eq!(
        trace["events"].as_str().unwrap(),
        index.events.len().to_string()
    );
    assert_eq!(trace["snapshots"].as_str().unwrap(), snapshots.to_string());
    // Every snapshot an event NAMES exists below the run directory: an index
    // that pointed at a file the writer never landed would be an audit trail
    // of nothing.
    let run_dir = trace_dir(project.path()).join(&run_id);
    for name in index
        .events
        .iter()
        .filter_map(|event| event.snapshot.as_ref())
    {
        assert!(
            run_dir.join(name).is_file(),
            "the named snapshot `{name}` exists below the run directory",
        );
    }
}
