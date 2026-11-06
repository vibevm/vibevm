//! Consumer install/check/repeat preserves selected authored package members.
#![cfg(test)]
mod common;

use common::UserScratch;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
use vibe_core::manifest::{Lockfile, SpecFormat};

fn write(root: &Path, path: impl AsRef<Path>, body: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, body).unwrap();
}

fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(root).unwrap().into(),
                    fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

fn command(user: &UserScratch, project: &Path, verb: &str) {
    let mut cmd = user.vibe();
    cmd.current_dir(project)
        .env("HOME", project.join("home"))
        .env("USERPROFILE", project.join("home"))
        .env_remove("VIBE_AGENT_COMMAND")
        .env_remove("VIBE_AGENT_ARGS")
        .env_remove("VIBE_INVOKED_BY")
        .args([
            verb,
            "--path",
            project.to_str().unwrap(),
            "--offline",
            "--json",
        ]);
    if verb == "install" {
        cmd.arg("--assume-yes");
    }
    let output = cmd.output().unwrap();
    assert!(
        output.status.success(),
        "{verb}: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-009#ARTIFACTS-PAIR"
)]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-035#FORMAT-SIMPLE"
)]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-workspace/PROP-038#UNIT-ARTIFACT-STATIC-CONSUMER-ELIDES"
)]
fn install_check_repeat_and_lock_reread_preserve_source_members_and_authored_fragments() {
    for (format, spec_format) in [
        ("xml", SpecFormat::Xml),
        ("markdown", SpecFormat::Markdown),
        ("mixed", SpecFormat::Mixed),
    ] {
        let temp = tempfile::tempdir().unwrap();
        let project = temp.path().join("project");
        let registry = project.join("registry");
        let user = UserScratch::new();
        for (name, child) in [
            ("parent", Some("middle")),
            ("middle", Some("leaf")),
            ("leaf", None),
        ] {
            let package = registry.join("org.g3").join(name).join("v1.0.0");
            let extra = if name == "parent" {
                format!(
                    "[[boot_snippet.fragment]]\nsource='{}'\n",
                    common::spec_rel("boot/extra.md")
                )
            } else {
                String::new()
            };
            let requires = child
                .map(|child| {
                    format!(
                        "[requires.packages]\n'org.g3/{child}'={{version='=1.0.0',link='static'}}\n"
                    )
                })
                .unwrap_or_default();
            write(
                &package,
                "vibe.toml",
                &format!(
                    "[package]\ngroup='org.g3'\nname='{name}'\nversion='1.0.0'\nkind='flow'\nformat='simple'\n[boot_snippet]\nsource='{}'\n{extra}{requires}",
                    common::spec_rel("boot/main.md")
                ),
            );
            write(
                &package,
                common::spec_rel("boot/main.md"),
                &format!("# {name} {{#root}}\nG3_CLI_{name}\n"),
            );
            if name == "parent" {
                write(
                    &package,
                    common::spec_rel("boot/extra.md"),
                    "# Extra {#extra}\nG3_CLI_EXTRA\n",
                );
            }
            for file in ["STATIC.xml", "STATIC.md", "INDEX.md", ".vibe-boot-stage"] {
                write(
                    &package,
                    common::boot_dir().join(file),
                    "GENERATED_POISON_G3\n",
                );
            }
            write(
                &package,
                "AGENTS.md",
                "<vibevm>\nsource-owned unmatched opening\n",
            );
        }
        write(
            &project,
            "vibe.toml",
            &format!(
                "[project]\ngroup='org.g3.host'\nname='host'\nversion='1.0.0'\nspec_format='{format}'\n[workspace]\nmembers=['registry/org.g3/parent/v1.0.0','registry/org.g3/middle/v1.0.0','registry/org.g3/leaf/v1.0.0']\n[requires.packages]\n'org.g3/parent'={{version='=1.0.0',link='static'}}\n[[registry]]\nname='local'\nurl='file://{}'\n",
                registry.display()
            ),
        );
        let before = snapshot(&registry);
        command(&user, &project, "install");
        assert_eq!(
            snapshot(&registry),
            before,
            "{format}: install wrote sources"
        );
        let lane_path = project.join(vibe_workspace::boot_artifacts::static_path(spec_format));
        let lane = fs::read_to_string(&lane_path).unwrap();
        for marker in [
            "G3_CLI_parent",
            "G3_CLI_middle",
            "G3_CLI_leaf",
            "G3_CLI_EXTRA",
        ] {
            assert_eq!(
                lane.matches(marker).count(),
                1,
                "{format}: {marker}: {lane}"
            );
        }
        assert!(!lane.contains("GENERATED_POISON_G3"));
        command(&user, &project, "check");
        command(&user, &project, "install");
        assert_eq!(fs::read_to_string(&lane_path).unwrap(), lane);
        assert_eq!(
            snapshot(&registry),
            before,
            "{format}: check/repeat wrote sources"
        );
        let lock = Lockfile::read(project.join("vibe.lock")).unwrap();
        assert_eq!(lock.packages.len(), 3);
        let parent = registry.join("org.g3/parent/v1.0.0/vibe.toml");
        let manifest = fs::read_to_string(&parent)
            .unwrap()
            .replace("version='1.0.0'", "version='1.0.1'");
        fs::write(parent, manifest).unwrap();
        let drifted = snapshot(&registry);
        let ws = vibe_workspace::Workspace::load(&project).unwrap();
        let nodes =
            vibe_workspace::install::regenerate_boot_with_spec_format(&ws, spec_format).unwrap();
        assert_eq!(nodes, ["."]);
        assert_eq!(
            snapshot(&registry),
            drifted,
            "{format}: durable lock reread wrote sources"
        );
        assert_eq!(fs::read_to_string(&lane_path).unwrap(), lane);
    }
}
