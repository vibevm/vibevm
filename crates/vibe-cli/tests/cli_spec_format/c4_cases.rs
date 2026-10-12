//! Public CLI conversion warnings, recorded check and strict Fresh refusal.

use super::*;

const BAD: &str = "# Rule {#root}\n\n@fact:GOOD Good. @status:impl/done\n\nPlain prose.\n\n@status:impl/done\n\nFollowing prose.\n";

fn fixture() -> (tempfile::TempDir, UserScratch, tempfile::TempDir) {
    assert!(git_available());
    let registry = tempfile::tempdir().unwrap();
    for (name, body) in [
        (
            "good",
            "# Good {#root}\n\n@fact:GOOD Good. @status:impl/done\n",
        ),
        ("bad", BAD),
    ] {
        let source = registry.path().join(format!("source-{name}"));
        let rel = common::spec_rel(&format!("flows/{name}/SPEC.md"));
        fs::create_dir_all(source.join(&rel).parent().unwrap()).unwrap();
        fs::write(source.join(&rel), body).unwrap();
        fs::write(source.join("README.md"), "# Ordinary Markdown\n").unwrap();
        fs::write(source.join("vibe.toml"), format!(
            "[package]\ngroup='{XML_GROUP}'\nname='{name}'\nkind='flow'\nversion='{VERSION}'\n\n[[package.sources]]\nfile='{}'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n",
            rel.replace('\\', "/")
        )).unwrap();
        publish(registry.path(), &coordinate(&source), &source);
    }
    let user = UserScratch::new();
    let project = tempfile::tempdir().unwrap();
    user.init_project(project.path());
    fs::write(project.path().join("vibe.toml"), format!(
        "[project]\nname='c4-consumer'\ngroup='{XML_GROUP}'\nversion='0.0.1'\nspec_format='xml'\n\n[requires.packages]\n'flow:{XML_GROUP}/good'={{version='={VERSION}',link='dynamic'}}\n'flow:{XML_GROUP}/bad'={{version='={VERSION}',link='dynamic'}}\n\n[[registry]]\nname='c4'\nurl='git+file://{}'\n",
        registry.path().to_string_lossy().replace('\\', "/")
    )).unwrap();
    (registry, user, project)
}

fn run_install(user: &UserScratch, project: &Path, json: bool, strict: bool) -> Output {
    let mut command = user.vibe();
    if json {
        command.arg("--json");
    }
    command
        .args(["install", "--path"])
        .arg(project)
        .arg("--assume-yes");
    if strict {
        command.arg("--strict-conversion");
    }
    command.output().unwrap()
}

fn committed_state(project: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn collect(project: &Path, path: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        if !path.exists() {
            return;
        }
        if path.is_dir() {
            for child in fs::read_dir(path).unwrap() {
                collect(project, &child.unwrap().path(), out);
            }
        } else {
            out.insert(
                path.strip_prefix(project).unwrap().to_path_buf(),
                fs::read(path).unwrap(),
            );
        }
    }
    let mut out = BTreeMap::new();
    for path in [
        PathBuf::from("vibe.toml"),
        PathBuf::from("vibe.lock"),
        vibe_core::layout::current_vibedeps_root(),
        vibe_core::layout::current_boot_dir(),
    ] {
        collect(project, &project.join(path), &mut out);
    }
    out
}

#[test]
fn default_cli_warns_once_and_check_reports_stored_issue_then_strict_fresh_refuses() {
    let (_registry, user, project) = fixture();
    let installed = run_install(&user, project.path(), false, false);
    assert_success("default install", &installed);
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&installed.stdout),
        String::from_utf8_lossy(&installed.stderr)
    );
    assert_eq!(
        text.matches("warning: requested Vibe conversion failed at")
            .count(),
        1,
        "{text}"
    );
    assert!(
        text.contains("[Stranded]") && text.contains("bad/SPEC.md"),
        "{text}"
    );
    assert!(text.contains("1 converted, 1 copied-unconverted"), "{text}");
    let checked = user
        .vibe()
        .args(["--json", "check", "--path"])
        .arg(project.path())
        .output()
        .unwrap();
    assert_success("check recorded issue", &checked);
    let value: Value = serde_json::from_slice(&checked.stdout).unwrap();
    let findings = value["findings"].as_array().unwrap();
    let issues: Vec<_> = findings
        .iter()
        .filter(|finding| {
            finding["message"]
                .as_str()
                .is_some_and(|message| message.contains("[Stranded]"))
        })
        .collect();
    assert_eq!(issues.len(), 1, "{value}");
    assert!(issues[0]["path"].as_str().unwrap().ends_with("bad/SPEC.md"));
    assert_eq!(issues[0]["line"], 7);
    let before = committed_state(project.path());
    let refused = run_install(&user, project.path(), true, true);
    assert!(
        !refused.status.success(),
        "strict Fresh accepted failed conversion"
    );
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&refused.stdout),
        String::from_utf8_lossy(&refused.stderr)
    );
    assert!(
        text.contains("Stranded") && text.contains("strict conversion refused"),
        "{text}"
    );
    assert!(
        text.contains("in org.vibevm.test/bad"),
        "expected retained Fresh refusal: {text}"
    );
    assert_eq!(committed_state(project.path()), before);
}

#[test]
fn json_applied_closure_counts_converted_and_failed_once() {
    let (_registry, user, project) = fixture();
    let installed = run_install(&user, project.path(), true, false);
    assert_success("default JSON install", &installed);
    let docs: Vec<Value> = serde_json::Deserializer::from_slice(&installed.stdout)
        .into_iter()
        .collect::<Result<_, _>>()
        .unwrap();
    let diffs: Vec<_> = docs
        .iter()
        .filter(|doc| doc["command"] == "install:closure-diff")
        .collect();
    assert_eq!(diffs.len(), 1, "{docs:?}");
    assert_eq!(diffs[0]["converted"], 1);
    assert_eq!(diffs[0]["copied_unconverted"], 1);
    let issues = diffs[0]["conversion_issues"].as_array().unwrap();
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0]["issues"][0]["class"], "Stranded");
    let bad = slot(
        project.path(),
        &Coordinate {
            group: XML_GROUP.into(),
            name: "bad".into(),
            version: VERSION.into(),
        },
    );
    let record = read_slot_record(&bad).unwrap();
    let ordinary = record
        .files
        .iter()
        .find(|file| file.path == "README.md")
        .unwrap();
    assert_eq!(
        ordinary.disposition,
        Some(vibe_workspace::vibedeps::SlotFileDisposition::Copied)
    );
}
