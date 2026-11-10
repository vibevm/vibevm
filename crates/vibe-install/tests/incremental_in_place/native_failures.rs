//! Actual two-slot apply failure cuts, with owned and foreign-state controls.
use super::*;
fn quote(path: &Path) -> String {
    format!("'{}'", path.to_str().unwrap().replace('\'', "'\"'\"'"))
}
pub(super) fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, path: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for item in fs::read_dir(path).unwrap() {
            let path = item.unwrap().path();
            let rel = path.strip_prefix(root).unwrap().to_owned();
            if path.is_dir() {
                out.insert(rel, vec![]);
                walk(root, &path, out)
            } else {
                out.insert(rel, fs::read(path).unwrap());
            }
        }
    }
    let mut result = BTreeMap::new();
    walk(root, root, &mut result);
    result
}
pub(super) fn ready(f: &Fixture) -> Box<vibe_install::PlannedInstall> {
    let request = InstallRequest {
        roots: f
            .source
            .names
            .iter()
            .map(|name| PackageRef::parse(&format!("org.vibevm/{name}")).unwrap())
            .collect(),
        features: FeatureRequest::default(),
        language: None,
        exact: false,
        generated_by: "native failure fixture".into(),
        offline: true,
        no_recommends: false,
    };
    match vibe_install::plan(&f.source, &f.project, request, &NullObserver).unwrap() {
        Plan::Ready(p) => p,
        Plan::Fresh => panic!("explicit roots require planning"),
    }
}
pub(super) fn publish(info: &serde_json::Value, name: &str, script: &str, broken_boot: bool) {
    let work = Path::new(
        info[if name == "giant" {
            "work"
        } else {
            "second_work"
        }]
        .as_str()
        .unwrap(),
    );
    git(work, &["reset", "--hard", "v1.0.0"]);
    let mut body = fs::read_to_string(work.join("vibe.toml")).unwrap();
    body.push_str("\n[hooks]\npre-install='hooks/native-test'\n");
    if broken_boot {
        body = body.replace(
            "materialization='in-place'",
            "materialization='in-place'\nformat='normal'",
        );
        body.push_str("\n[boot_snippet]\nsource='vibevm/vibespecs/boot/broken.md'\n");
        write(
            work,
            "vibevm/vibespecs/boot/broken.md",
            "# Broken {#root}\n#use spec://org.vibevm/ghost/boot/base#root\nBROKEN\n",
        );
    }
    write(work, "vibe.toml", &body);
    write(work, "hooks/native-test.sh", script);
    git(work, &["add", "-A"]);
    git(work, &["commit", "-m", "native failure fixture"]);
    git(work, &["tag", "-f", "v1.0.0"]);
    let bare = Path::new(info["root"].as_str().unwrap())
        .join("web")
        .join(if name == "giant" {
            "repo.git"
        } else {
            "second.git"
        });
    git(
        &bare,
        &[
            "fetch",
            "--force",
            work.to_str().unwrap(),
            "refs/tags/v1.0.0:refs/tags/v1.0.0",
        ],
    );
    git(&bare, &["update-server-info"]);
}
fn case(cut: &str) {
    let (_server, info, mut f) = fixture(false);
    f.source.names = vec!["giant".into(), "second".into()];
    vibe_install::apply(&f.source, *ready(&f), SlotIntegrity::Verify, &policy()).unwrap();
    let second = f.slot.with_file_name("org.vibevm.second");
    for (name, slot) in [("giant", &f.slot), ("second", &second)] {
        write(slot, ".git/sentinel", name);
        write(slot, "untracked.tmp", "old untracked bytes");
        write(slot, "ignored.tmp", "old ignored bytes");
        write(slot, ".git/info/exclude", "ignored.tmp\n");
        let head = git(slot, &["rev-parse", "HEAD"]);
        git(slot, &["update-ref", "refs/heads/private", &head]);
    }
    let observed = f._temp.path().join("pre-observed");
    let external = f._temp.path().join("displaced");
    let first_script = format!("printf 'giant\\n' >> {}\n", quote(&observed));
    let mut second_script = format!("printf 'second\\n' >> {}\n", quote(&observed));
    match cut {
        "pre" => second_script.push_str("exit 37\n"),
        "foreign" => second_script.push_str(&format!(
            "mv {} {}\nmkdir {}\nprintf 'foreign replacement' > {}/FOREIGN\nexit 37\n",
            quote(&second),
            quote(&external),
            quote(&second),
            quote(&second)
        )),
        "lock" => second_script.push_str(&format!("chmod 555 {}\n", quote(&f.project))),
        "boot" => {}
        _ => panic!("unknown cut"),
    }
    publish(&info, "giant", &first_script, false);
    publish(&info, "second", &second_script, cut == "boot");
    if cut == "boot" {
        write(
            &f.project,
            "vibe.toml",
            "[project]\nname='demo'\nversion='0.0.1'\n[requires.packages]\n'org.vibevm/giant'={version='^1.0',link='static'}\n'org.vibevm/second'={version='^1.0',link='static'}\n",
        );
    }
    let plan = ready(&f);
    let before = snapshot(&f.project);
    let old_first = snapshot(&f.slot);
    let old_second = snapshot(&second);
    let result = vibe_install::apply(&f.source, *plan, SlotIntegrity::Verify, &policy());
    // The fixture owns the chmod fault. Permission rollback is not an install claim.
    if cut == "lock" {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&f.project, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let error = result.unwrap_err();
    let message = error.to_string();
    assert_eq!(
        fs::read_to_string(&observed).unwrap(),
        "giant\nsecond\n",
        "failure happened before both actual placements: {message}"
    );
    if cut == "foreign" {
        assert!(
            message.contains("rollback conflict") && message.contains("backup retained"),
            "{message}"
        );
        assert_eq!(snapshot(&f.slot), old_first);
        assert_eq!(
            fs::read_to_string(second.join("FOREIGN")).unwrap(),
            "foreign replacement"
        );
        let stash = fs::read_dir(f.project.join("vibevm"))
            .unwrap()
            .map(|e| e.unwrap().path())
            .find(|p| {
                p.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with(".vibe-install-rollback-")
            })
            .unwrap();
        let old = fs::read_dir(&stash)
            .unwrap()
            .map(|e| e.unwrap().path())
            .find(|p| p.join(".git/sentinel").is_file())
            .unwrap();
        assert_eq!(snapshot(&old), old_second);
    } else {
        if cut == "lock" {
            assert!(
                message.contains("vibe.lock"),
                "expected final lock-write failure, got {message}"
            );
        }
        if cut == "boot" {
            assert!(
                message.contains("compile") || message.contains("ghost"),
                "expected boot failure, got {message}"
            );
        }
        assert_eq!(snapshot(&f.project), before, "{cut}: {message}");
    }
}
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-MATERIALISATION-LAW"
)]
fn two_native_slots_restore_after_preinstall_failure() {
    case("pre")
}
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-MATERIALISATION-LAW"
)]
fn two_native_slots_restore_after_boot_failure() {
    case("boot")
}
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-MATERIALISATION-LAW"
)]
fn two_native_slots_restore_after_lock_failure() {
    case("lock")
}
#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-MATERIALISATION-LAW"
)]
fn actual_foreign_slot_before_undo_is_preserved_with_backup() {
    case("foreign")
}
