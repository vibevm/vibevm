//! Public recursive placement uses the existing checked undo through metadata completion.
use super::*;
use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;

fn git(info: &serde_json::Value, cwd: &Path, args: &[&str]) -> String {
    let output = Command::new("/usr/bin/git")
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env(
            "HOME",
            Path::new(info["root"].as_str().unwrap()).join("home"),
        )
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .args([
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=f@invalid",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().into()
}
fn write(root: &Path, name: &str, body: &str) {
    let path = root.join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, body).unwrap();
}
fn publish(info: &serde_json::Value) {
    let work = Path::new(info["work"].as_str().unwrap());
    git(info, work, &["commit", "-m", "recursive fixture"]);
    git(info, work, &["tag", "-f", "v1.0.0"]);
    let bare = Path::new(info["root"].as_str().unwrap()).join("web/repo.git");
    git(
        info,
        &bare,
        &[
            "fetch",
            "--force",
            work.to_str().unwrap(),
            "refs/tags/v1.0.0:refs/tags/v1.0.0",
        ],
    );
    git(info, &bare, &["update-server-info"]);
}
fn ready(source: &Source, project: &Path) -> Box<vibe_install::PlannedInstall> {
    let request = InstallRequest {
        roots: vec![PackageRef::parse("org.fixture/demo@=1.0.0").unwrap()],
        features: FeatureRequest::default(),
        language: None,
        exact: false,
        generated_by: "recursive fixture".into(),
        offline: false,
    };
    match vibe_install::plan(source, project, request, &NullObserver).unwrap() {
        Plan::Ready(plan) => plan,
        Plan::Fresh => panic!("explicit install must plan"),
    }
}
fn snapshot(root: &Path) -> BTreeMap<PathBuf, (u32, Vec<u8>)> {
    fn walk(root: &Path, at: &Path, result: &mut BTreeMap<PathBuf, (u32, Vec<u8>)>) {
        for entry in fs::read_dir(at).unwrap() {
            let path = entry.unwrap().path();
            let metadata = fs::symlink_metadata(&path).unwrap();
            let bytes = if metadata.file_type().is_symlink() {
                fs::read_link(&path)
                    .unwrap()
                    .as_os_str()
                    .as_encoded_bytes()
                    .into()
            } else if metadata.is_file() {
                fs::read(&path).unwrap()
            } else {
                vec![]
            };
            result.insert(
                path.strip_prefix(root).unwrap().into(),
                (metadata.permissions().mode(), bytes),
            );
            if metadata.is_dir() {
                walk(root, &path, result);
            }
        }
    }
    let mut result = BTreeMap::new();
    walk(root, root, &mut result);
    result
}
fn quote(path: &Path) -> String {
    format!("'{}'", path.to_str().unwrap().replace('\'', "'\"'\"'"))
}

fn recovery(cut: &str) {
    let mut server = Server(
        Command::new("python3")
            .args([
                "-u",
                "-c",
                include_str!(
                    "../../../vibe-registry/src/git_backend/shell/native_admission/tls_fixture.py"
                ),
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let mut line = String::new();
    BufReader::new(server.0.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    let info: serde_json::Value = serde_json::from_str(&line).unwrap();
    let root = tempfile::tempdir().unwrap();
    let project = root.path().join("project");
    fs::create_dir(&project).unwrap();
    let work = Path::new(info["work"].as_str().unwrap());
    let child = root.path().join("child");
    fs::create_dir(&child).unwrap();
    git(&info, &child, &["init"]);
    write(&child, "child.txt", "pinned child\n");
    git(&info, &child, &["add", "."]);
    git(&info, &child, &["commit", "-m", "child"]);
    let child_oid = git(&info, &child, &["rev-parse", "HEAD"]);
    let child_bare = Path::new(info["root"].as_str().unwrap()).join("web/child.git");
    git(
        &info,
        root.path(),
        &[
            "clone",
            "--bare",
            child.to_str().unwrap(),
            child_bare.to_str().unwrap(),
        ],
    );
    git(&info, &child_bare, &["update-server-info"]);
    git(&info, work, &["reset", "--hard", "v1.0.0"]);
    write(
        work,
        "vibe.toml",
        "[package]\ngroup='org.fixture'\nname='demo'\nkind='feat'\nversion='1.0.0'\nmaterialization='in-place'\n",
    );
    write(
        work,
        ".gitmodules",
        &format!(
            "[submodule \"child\"]\npath=child\nurl={}/child.git\n",
            info["b"].as_str().unwrap()
        ),
    );
    git(&info, work, &["add", "vibe.toml", ".gitmodules"]);
    git(
        &info,
        work,
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            &format!("160000,{child_oid},child"),
        ],
    );
    publish(&info);
    let authored = format!(
        "[project]\nname='recursive'\nversion='0.1.0'\n[[registry]]\nname='recursive'\nurl='{}/public'\nauth='none'\n",
        info["a"].as_str().unwrap()
    );
    write(&project, "vibe.toml", &authored);
    let manifest = Manifest::parse_str(&authored).unwrap();
    let selection = NativeConfigSelection::new(
        root.path().into(),
        Path::new(info["root"].as_str().unwrap()).join("home"),
    )
    .unwrap()
    .with_variable("GIT_CONFIG_NOSYSTEM", "1".into())
    .unwrap()
    .with_variable("GIT_CONFIG_GLOBAL", "/dev/null".into())
    .unwrap()
    .with_variable("GIT_ATTR_NOSYSTEM", "1".into())
    .unwrap();
    let profile = NativeGitAdmission::direct_https(
        Arc::new(vibe_registry::HttpAdmission::new(Vec::<String>::new()).unwrap()),
        GitInstallation::trusted("/usr/bin/git".into(), "/usr/lib/git-core".into()).unwrap(),
        TlsTrust::Pem(fs::read(info["ca"].as_str().unwrap()).unwrap()),
    )
    .with_config_selection(selection);
    let source = Source {
        resolver: MultiRegistryResolver::from_manifest_with_native_admission(
            &manifest.registries,
            &manifest.mirrors,
            &[],
            root.path().join("registry"),
            0,
            Arc::new(profile),
        )
        .unwrap(),
        store: root.path().join("store"),
    };
    let policy = vibe_workspace::hooks::HookPolicy {
        allowed_groups: vec!["org.fixture".into()],
        allow_hooks: false,
    };
    vibe_install::apply(
        &source,
        *ready(&source, &project),
        SlotIntegrity::Verify,
        &policy,
    )
    .unwrap();
    let slot = project
        .join(vibe_core::layout::current_vibedeps_root())
        .join("org.fixture.demo");
    assert_eq!(
        git(&info, &slot.join("child"), &["rev-parse", "HEAD"]),
        child_oid
    );
    assert!(!slot.join("child/vibe.toml").exists());
    git(&info, &slot, &["submodule", "absorbgitdirs"]);
    write(&slot, "child/child.txt", "dirty child\n");
    write(&slot, "child/untracked", "keep me\n");
    write(&slot, ".git/modules/child/sentinel", "metadata\n");
    let observed = root.path().join("observed");
    let displaced = root.path().join("displaced");
    let mut script = format!("printf 'published' > {}\n", quote(&observed));
    match cut {
        "hook" => script.push_str("exit 37\n"),
        "lock" => script.push_str(&format!("chmod 555 {}\n", quote(&project))),
        "foreign" => script.push_str(&format!(
            "mv {} {}\nmkdir {}\nprintf 'foreign' > {}/FOREIGN\nexit 37\n",
            quote(&slot),
            quote(&displaced),
            quote(&slot),
            quote(&slot)
        )),
        "boot" => {}
        _ => panic!("unknown failure cut"),
    }
    let mut package = fs::read_to_string(work.join("vibe.toml")).unwrap();
    if cut == "boot" {
        package.push_str("format='normal'\n");
    }
    package.push_str("\n[hooks]\npre-install='hooks/failure'\n");
    if cut == "boot" {
        package.push_str("\n[boot_snippet]\nsource='vibevm/vibespecs/boot/broken.md'\n");
        write(
            work,
            "vibevm/vibespecs/boot/broken.md",
            "# Broken {#root}\n#use spec://org.fixture/ghost/boot/base#root\nBROKEN\n",
        );
        write(
            &project,
            "vibe.toml",
            &format!(
                "{authored}\n[requires.packages]\n'org.fixture/demo'={{version='^1.0',link='static'}}\n"
            ),
        );
    }
    write(work, "vibe.toml", &package);
    write(work, "hooks/failure.sh", &script);
    git(&info, work, &["add", "vibe.toml", "hooks"]);
    if cut == "boot" {
        git(&info, work, &["add", "vibevm"]);
    }
    publish(&info);
    let before = snapshot(&project);
    let old_slot = snapshot(&slot);
    let plan = ready(&source, &project);
    assert_eq!(snapshot(&project), before);
    let result = vibe_install::apply(&source, *plan, SlotIntegrity::Verify, &policy);
    if cut == "lock" {
        fs::set_permissions(&project, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let message = result.unwrap_err().to_string();
    assert_eq!(
        fs::read_to_string(observed).unwrap(),
        "published",
        "failure before placement: {message}"
    );
    if cut == "foreign" {
        assert!(
            message.contains("rollback conflict") && message.contains("backup retained"),
            "{message}"
        );
        assert_eq!(fs::read(slot.join("FOREIGN")).unwrap(), b"foreign");
        let stash = fs::read_dir(project.join("vibevm"))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .find(|path| {
                path.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with(".vibe-install-rollback-")
            })
            .unwrap();
        let backup = fs::read_dir(stash)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .find(|path| path.join("child/untracked").is_file())
            .unwrap();
        assert_eq!(snapshot(&backup), old_slot);
    } else {
        if cut == "lock" {
            assert!(message.contains("vibe.lock"), "{message}");
        }
        if cut == "boot" {
            assert!(
                message.contains("compile") || message.contains("ghost"),
                "{message}"
            );
        }
        assert_eq!(snapshot(&project), before, "{cut}: {message}");
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-ROUTES-LAW")]
fn recursive_parent_and_child_restore_across_public_failure_cuts() {
    for cut in ["hook", "boot", "lock", "foreign"] {
        recovery(cut);
    }
}

#[path = "provenance.rs"]
mod provenance;
