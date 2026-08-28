use std::fs;

use vibe_core::manifest::Lockfile;

use super::support::{context, project, report, run, write_registry_package};

#[test]
fn a_real_project_local_manifest_dependency_is_resolved_and_materialised() {
    let project = project("\n[requires]\npackages = { \"org.hosted/tool\" = \"^0.1\" }\n");
    write_registry_package(project.path(), "org.hosted", "tool", "0.1.0");

    let output = run(&context(project.path()), "install").unwrap();
    assert!(!output.is_error());
    let report = report(&output);
    assert!(report.ok);
    assert_eq!(report.steps.last().unwrap().phase, "install");
    assert!(project.path().join("vibe.lock").is_file());
    let lock = Lockfile::read(project.path().join("vibe.lock")).unwrap();
    assert!(lock.packages.iter().any(|package| {
        package.group.as_str() == "org.hosted"
            && package.name == "tool"
            && package.version.to_string() == "0.1.0"
    }));
    let slots = project
        .path()
        .join(vibe_core::layout::current_vibedeps_root());
    let manifests = walk_manifests(&slots);
    assert!(
        manifests.iter().any(|path| {
            fs::read_to_string(path).is_ok_and(|body| body.contains("name = \"tool\""))
        }),
        "the real dependency was materialised under {}: {manifests:?}",
        slots.display()
    );
}

fn walk_manifests(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    fn walk(at: &std::path::Path, into: &mut Vec<std::path::PathBuf>) {
        let Ok(entries) = fs::read_dir(at) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, into);
            } else if path.file_name().is_some_and(|name| name == "vibe.toml") {
                into.push(path);
            }
        }
    }
    let mut manifests = Vec::new();
    walk(root, &mut manifests);
    manifests
}
