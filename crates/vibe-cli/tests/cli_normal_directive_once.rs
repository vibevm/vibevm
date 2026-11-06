//! Ordinary install/repeat/reinstall of a normal XML consumer with real unit caches.
#![cfg(test)]
mod common;
mod trace_support;
use common::UserScratch;
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Output,
};
use vibe_core::{
    Group,
    manifest::{Lockfile, SpecFormat},
};
use vibe_wire::generated::compiler_trace_index::e1::index::{ScopeKind, ScopeStatus};

fn write(root: &Path, path: impl AsRef<Path>, body: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, body).unwrap();
}
fn package(registry: &Path, name: &str, snippet: &str, requires: &str) -> PathBuf {
    let root = registry.join("org.g1").join(name).join("v1.0.0");
    write(
        &root,
        "vibe.toml",
        &format!(
            "[package]\ngroup='org.g1'\nname='{name}'\nversion='1.0.0'\nkind='flow'\nformat='normal'\n[boot_snippet]\nsource='{}'\ncategory='flow'\n{requires}",
            common::spec_rel(snippet)
        ),
    );
    root
}
fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, here: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(here).unwrap() {
            let p = entry.unwrap().path();
            if p.is_dir() {
                walk(root, &p, out);
            } else {
                out.insert(p.strip_prefix(root).unwrap().into(), fs::read(p).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}
fn run(user: &UserScratch, project: &Path, args: &[&str], label: &str) -> Output {
    let mut command = user.vibe();
    command
        .current_dir(project)
        .env("HOME", project.parent().unwrap().join("home"))
        .env("USERPROFILE", project.parent().unwrap().join("home"))
        .env_remove("VIBE_AGENT_COMMAND")
        .env_remove("VIBE_AGENT_ARGS")
        .env_remove("VIBE_INVOKED_BY")
        .args(args)
        .args(["--offline", "--json"]);
    if matches!(args.first(), Some(&"install" | &"reinstall")) {
        command.arg("--assume-yes");
    }
    let output = command.output().unwrap();
    if let Some(path) = std::env::var_os("NST_G1_EVIDENCE") {
        let path = PathBuf::from(path);
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join(format!("{label}.stdout")), &output.stdout).unwrap();
        fs::write(path.join(format!("{label}.stderr")), &output.stderr).unwrap();
    }
    assert!(
        output.status.success(),
        "{label}: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}
fn lane(project: &Path) -> String {
    let body = fs::read_to_string(
        project.join(vibe_workspace::boot_artifacts::static_path(SpecFormat::Xml)),
    )
    .unwrap();
    vibe_spec::minify_emitted_xml(&body).unwrap();
    for marker in [
        "CLI_G1_UMBRELLA",
        "CLI_G1_ALPHA",
        "CLI_G1_UNCOVERED",
        "CLI_G1_BETA",
        "CLI_G1_SOURCE",
        "CLI_G1_OUTER",
        "CLI_G1_INNER",
    ] {
        assert_eq!(body.matches(marker).count(), 1, "{marker}: {body}");
    }
    if let Some(path) = std::env::var_os("NST_G1_EVIDENCE") {
        fs::write(PathBuf::from(path).join("installed-STATIC.xml"), &body).unwrap();
    }
    body
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#NORMAL-CLOSURE-ONCE"
)]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#AUTHORED-INDEX-BOUNDARY"
)]
fn xml_install_repeat_and_unit_reuse_keep_normal_closure_once_and_nested_attribution() {
    installed_embed_attribution(false);
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#EMBED-EXACT-RULE"
)]
fn xml_standalone_generated_embed_comments_survive_install_and_reinstall() {
    installed_embed_attribution(true);
}

fn installed_embed_attribution(standalone: bool) {
    let temp = tempfile::tempdir().unwrap();
    let registry = temp.path().join("registry");
    let project = temp.path().join("project");
    let user = UserScratch::new();
    let umbrella = package(
        &registry,
        "umbrella",
        "boot/entry.md",
        "[requires.packages]\n'org.g1/alpha'={version='=1.0.0',link='static'}\n'org.g1/beta'={version='=1.0.0',link='static'}\n",
    );
    let alpha = package(
        &registry,
        "alpha",
        "contract/root.md",
        "[requires.packages]\n'org.g1/beta'={version='=1.0.0',link='static'}\n",
    );
    let beta = package(&registry, "beta", "contract/solution.md", "");
    write(
        &umbrella,
        common::spec_rel("boot/entry.md"),
        "# Umbrella {#root}\n#use spec://org.g1/alpha/contract/root#root\nCLI_G1_UMBRELLA\n",
    );
    write(
        &alpha,
        common::spec_rel("contract/root.md"),
        "# Alpha {#root}\n#use spec://org.g1/beta/contract/solution#solution\nCLI_G1_ALPHA\n# Uncovered {#uncovered}\nCLI_G1_UNCOVERED\n",
    );
    write(
        &beta,
        common::spec_rel("contract/solution.md"),
        if standalone {
            "# Beta {#solution}\n#source spec://org.g1/beta/source/implementation#implementation\nCLI_G1_BETA\n\n#embed spec://org.g1/beta/payload/outer#outer\n\n"
        } else {
            "# Beta {#solution}\n#source spec://org.g1/beta/source/implementation#implementation\nCLI_G1_BETA\n#embed spec://org.g1/beta/payload/outer#outer\n"
        },
    );
    write(
        &beta,
        common::spec_rel("source/implementation.md"),
        "# Implementation {#implementation}\nCLI_G1_SOURCE\n",
    );
    write(
        &beta,
        common::spec_rel("payload/outer.md"),
        if standalone {
            "# Outer {#outer}\nCLI_G1_OUTER\n\n#embed spec://org.g1/beta/payload/inner#inner\n\n"
        } else {
            "# Outer {#outer}\nCLI_G1_OUTER\n#embed spec://org.g1/beta/payload/inner#inner\n"
        },
    );
    write(
        &beta,
        common::spec_rel("payload/inner.md"),
        if standalone {
            "# Inner {#inner}\nCLI_G1_INNER\n\n"
        } else {
            "# Inner {#inner}\nCLI_G1_INNER\n"
        },
    );
    let before = snapshot(&registry);
    write(
        &project,
        "vibe.toml",
        &format!(
            "[project]\nname='consumer'\ngroup='org.g1.host'\nversion='1.0.0'\nspec_format='xml'\n[requires.packages]\n'org.g1/umbrella'={{version='=1.0.0',link='static'}}\n'org.g1/alpha'={{version='=1.0.0',link='static'}}\n'org.g1/beta'={{version='=1.0.0',link='static'}}\n[[registry]]\nname='local'\nurl='file://{}'\n",
            registry.display()
        ),
    );
    for name in ["AGENTS.md", "CLAUDE.md", "GEMINI.md"] {
        write(&project, name, "# Consumer\n");
    }
    let authored_boot =
        "# Authored host boot\n#use spec://org.g1/not-installed/doc#root\nHOST_AUTHORED_LITERAL\n";
    let authored_path = common::boot_dir().join("00-core.md");
    write(&project, &authored_path, authored_boot);
    let first = run(
        &user,
        &project,
        &[
            "install",
            "--path",
            project.to_str().unwrap(),
            "--trace-compile",
        ],
        "install-first",
    );
    let first_report = trace_support::sole_root(&first.stdout, "install");
    let first_id = trace_support::sole_run(&project, &first_report);
    let first_trace = trace_support::index_of(&project, &first_id);
    let original_lane = lane(&project);
    assert!(!original_lane.contains("HOST_AUTHORED_LITERAL"));
    assert_eq!(
        fs::read_to_string(project.join(&authored_path)).unwrap(),
        authored_boot
    );
    let index: toml::Value =
        toml::from_str(&fs::read_to_string(project.join(common::index_rel())).unwrap()).unwrap();
    assert!(
        index["entry"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["path"].as_str() == authored_path.to_str()
                && e["kind"].as_str() == Some("static")),
        "{index}"
    );

    let lock = Lockfile::read(project.join("vibe.lock")).unwrap();
    assert_eq!(lock.packages.len(), 3);
    let mut units = BTreeMap::new();
    let mut unit_indexes = BTreeMap::new();
    for name in ["umbrella", "alpha"] {
        let slot = vibe_workspace::vibedeps::slot_abs_path(
            &project,
            &Group::parse("org.g1").unwrap(),
            name,
            &"1.0.0".parse().unwrap(),
        );
        let index = slot.join(common::index_rel());
        let artifact = slot.join(common::static_xml_rel());
        assert!(
            vibe_workspace::boot_artifacts::read_fingerprint(&fs::read_to_string(&index).unwrap())
                .is_some()
        );
        let text = fs::read_to_string(&index).unwrap();
        unit_indexes.insert(
            name,
            (
                index,
                vibe_workspace::boot_artifacts::read_fingerprint(&text).unwrap(),
            ),
        );
        units.insert(
            name,
            (
                artifact.clone(),
                fs::read(&artifact).unwrap(),
                fs::metadata(&artifact).unwrap().modified().unwrap(),
            ),
        );
    }
    assert!(
        first_trace
            .scopes
            .iter()
            .filter(|s| s.kind == ScopeKind::Unit && s.status == ScopeStatus::Compiled)
            .count()
            >= 2
    );
    let second = run(
        &user,
        &project,
        &[
            "install",
            "--path",
            project.to_str().unwrap(),
            "--trace-compile",
        ],
        "install-repeat",
    );
    let second_report = trace_support::sole_root(&second.stdout, "install");
    let second_id = trace_support::sole_run(&project, &second_report);
    let second_trace = trace_support::index_of(&project, &second_id);
    assert_ne!(first_id, second_id);
    assert!(
        second_trace
            .scopes
            .iter()
            .filter(|s| s.kind == ScopeKind::Unit && s.status == ScopeStatus::Skipped)
            .count()
            >= 2,
        "{:?}",
        second_trace.scopes
    );
    assert_eq!(lane(&project), original_lane);
    for (path, bytes, mtime) in units.values() {
        assert_eq!(fs::read(path).unwrap(), *bytes);
        assert_eq!(fs::metadata(path).unwrap().modified().unwrap(), *mtime);
    }
    // Historical cached units had the same graph fingerprint but no compiler
    // recipe. Removing only that new comment preserves the exact old metadata
    // layout; a real subsequent install must compile, then reuse the new result.
    for (index, fingerprint) in unit_indexes.values() {
        let text = fs::read_to_string(index).unwrap();
        assert_eq!(
            text.lines()
                .filter(|line| line.starts_with("# vibe:compiler-recipe "))
                .count(),
            1
        );
        let historical = text
            .lines()
            .filter(|line| !line.starts_with("# vibe:compiler-recipe "))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        assert_eq!(
            vibe_workspace::boot_artifacts::read_fingerprint(&historical).as_deref(),
            Some(fingerprint.as_str())
        );
        fs::write(index, historical).unwrap();
    }
    let upgraded = run(
        &user,
        &project,
        &[
            "install",
            "--path",
            project.to_str().unwrap(),
            "--trace-compile",
        ],
        "install-unstamped-cache",
    );
    let upgraded_report = trace_support::sole_root(&upgraded.stdout, "install");
    let upgraded_id = trace_support::sole_run(&project, &upgraded_report);
    let upgraded_trace = trace_support::index_of(&project, &upgraded_id);
    assert!(
        upgraded_trace
            .scopes
            .iter()
            .filter(|scope| scope.kind == ScopeKind::Unit && scope.status == ScopeStatus::Compiled)
            .count()
            >= 2
    );
    for (index, fingerprint) in unit_indexes.values() {
        let text = fs::read_to_string(index).unwrap();
        assert_eq!(
            vibe_workspace::boot_artifacts::read_fingerprint(&text).as_deref(),
            Some(fingerprint.as_str())
        );
        assert!(text.contains(&format!(
            "# vibe:compiler-recipe {}",
            vibe_spec::COMPILER_ARTIFACT_RECIPE
        )));
    }
    let reused = run(
        &user,
        &project,
        &[
            "install",
            "--path",
            project.to_str().unwrap(),
            "--trace-compile",
        ],
        "install-upgraded-cache-repeat",
    );
    let reused_report = trace_support::sole_root(&reused.stdout, "install");
    let reused_id = trace_support::sole_run(&project, &reused_report);
    let reused_trace = trace_support::index_of(&project, &reused_id);
    assert!(
        reused_trace
            .scopes
            .iter()
            .filter(|scope| scope.kind == ScopeKind::Unit && scope.status == ScopeStatus::Skipped)
            .count()
            >= 2
    );
    assert_eq!(lane(&project), original_lane);
    let tree = run(
        &user,
        &project,
        &["tree", "--path", project.to_str().unwrap()],
        "tree",
    );
    let mut doc: Value = serde_json::from_slice(&tree.stdout).unwrap();
    assert_eq!(doc["schema_version"], 1);
    for key in ["ok", "command", "invoked_by", "unattended"] {
        doc.as_object_mut().unwrap().remove(key);
    }
    let schema: Value =
        serde_json::from_str(include_str!("../resources/package-tree.schema.v1.json")).unwrap();
    assert!(jsonschema::validator_for(&schema).unwrap().is_valid(&doc));
    let contributions = doc["boot"]["static_md"]["contributions"]
        .as_array()
        .unwrap();
    let shared = contributions
        .iter()
        .find(|c| {
            c["origin"]
                .as_str()
                .is_some_and(|s| s.starts_with("org.g1/beta [shared by "))
        })
        .expect("suppressed shared normal contribution retains provenance");
    let beta = contributions
        .iter()
        .find(|c| c["origin"] == "org.g1/beta")
        .unwrap();
    assert_eq!(shared["source_path"], beta["source_path"]);
    assert!(
        shared["lines"].as_u64().unwrap() > 0,
        "empty XML body still has a truthful frame"
    );
    assert!(shared["embeds"].as_array().unwrap().is_empty());
    let addresses = doc["boot"]["static_md"]["contributions"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|c| c["embeds"].as_array().unwrap())
        .map(|e| e["address"].as_str().unwrap())
        .collect::<Vec<_>>();
    fs::remove_file(project.join(vibe_workspace::boot_artifacts::static_path(SpecFormat::Xml)))
        .unwrap();
    run(
        &user,
        &project,
        &["reinstall", project.to_str().unwrap(), "--trace-compile"],
        "reinstall",
    );
    assert_eq!(lane(&project), original_lane);
    assert_eq!(
        snapshot(&registry),
        before,
        "install/compile never edits authored package sources"
    );
    // Named unmet XML-attribution assertion remains last, so the independent
    // reinstall and source-preservation checks execute even while it is RED.
    for address in [
        "spec://org.g1/beta/payload/outer#outer",
        "spec://org.g1/beta/payload/inner#inner",
    ] {
        assert_eq!(
            addresses.iter().filter(|a| **a == address).count(),
            1,
            "{addresses:?}"
        );
    }
}
