//! Actual Git-over-loopback sources qualify same-version preference and unchanged domain.
use super::*;
fn git(root: &Path, arguments: &[&str]) {
    let output = std::process::Command::new("/usr/bin/git")
        .current_dir(root)
        .env("HOME", root)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "Fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
        .env("GIT_COMMITTER_NAME", "Fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
        .args(arguments)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
}
fn manifest(root: &Path, version: &str) {
    fs::write(
        root.join("vibe.toml"),
        format!("[package]\ngroup='org.policy'\nname='demo'\nversion='{version}'\nkind='tool'\n"),
    )
    .unwrap();
    git(root, &["add", "vibe.toml"]);
    git(root, &["commit", "-m", "fixture"]);
    git(root, &["tag", &format!("v{version}")]);
}
#[test]
fn hosting_preference_changes_same_version_owner_but_not_default_version_domain() {
    let files = tempfile::tempdir().unwrap();
    let a = files.path().join("a");
    let b = files.path().join("b");
    let work = files.path().join("work");
    for path in [&a, &b, &work] {
        fs::create_dir_all(path).unwrap();
    }
    git(&work, &["init", "-b", "main"]);
    manifest(&work, "1.0.0");
    let repo_a = a.join("org.policy.demo.git");
    let repo_b = b.join("org.policy.demo.git");
    git(
        &work,
        &[
            "clone",
            "--bare",
            work.to_str().unwrap(),
            repo_b.to_str().unwrap(),
        ],
    );
    manifest(&work, "2.0.0");
    git(
        &work,
        &[
            "clone",
            "--bare",
            work.to_str().unwrap(),
            repo_a.to_str().unwrap(),
        ],
    );
    git(&work, &["checkout", "-B", "alternate", "v1.0.0"]);
    manifest(&work, "3.0.0");
    git(
        &work,
        &["push", repo_b.to_str().unwrap(), "refs/tags/v3.0.0"],
    );
    for repo in [&repo_a, &repo_b] {
        git(repo, &["update-server-info"]);
    }
    let first = Server::directory("127.0.0.1", a);
    let preferred = Server::directory("127.0.0.2", b);
    let user = quiet_user();
    for (constraint, registry, version) in
        [("=1.0.0", "preferred", "1.0.0"), ("*", "first", "2.0.0")]
    {
        let project = tempfile::tempdir().unwrap();
        fs::write(project.path().join("vibe.toml"),format!("[project]\nname='hosting'\nversion='1.0.0'\n[requires.packages]\n'org.policy/demo'='{constraint}'\n[[registry]]\nname='first'\nurl='{}'\nindex_url='none'\n[[registry]]\nname='preferred'\nurl='{}'\nindex_url='none'\n",first.url,preferred.url)).unwrap();
        let mut command = user.vibe();
        command
            .current_dir(project.path())
            .env("HOME", &user.settings)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .args([
                "install",
                "--assume-yes",
                "--no-default-registry",
                "--prefer-hosting",
                "host:127.0.0.2",
            ]);
        // This fixture owns only numeric loopback; neither source may use ambient proxies.
        for key in [
            "HTTP_PROXY",
            "HTTPS_PROXY",
            "ALL_PROXY",
            "http_proxy",
            "https_proxy",
            "all_proxy",
        ] {
            command.env_remove(key);
        }
        success(&command.output().unwrap());
        let lock: toml::Value =
            toml::from_str(&fs::read_to_string(project.path().join("vibe.lock")).unwrap()).unwrap();
        assert_eq!(lock["package"][0]["registry"].as_str(), Some(registry));
        assert_eq!(lock["package"][0]["version"].as_str(), Some(version));
    }
    assert!(first.count() > 0);
    assert!(preferred.count() > 0);
}
/// One plain-HTTP dumb Git registry serving `org.policy/demo` 1.0.0.
fn plain_http_registry(files: &Path) -> std::path::PathBuf {
    let served = files.join("served");
    let work = files.join("work");
    for path in [&served, &work] {
        fs::create_dir_all(path).unwrap();
    }
    git(&work, &["init", "-b", "main"]);
    manifest(&work, "1.0.0");
    let repo = served.join("org.policy.demo.git");
    git(
        &work,
        &[
            "clone",
            "--bare",
            work.to_str().unwrap(),
            repo.to_str().unwrap(),
        ],
    );
    git(&repo, &["update-server-info"]);
    served
}
fn controlled_install(user: &UserScratch, registry: &str, excluded: &str) -> std::process::Output {
    let project = tempfile::tempdir().unwrap();
    fs::write(project.path().join("vibe.toml"),format!("[project]\nname='plain'\nversion='1.0.0'\n[requires.packages]\n'org.policy/demo'='*'\n[[registry]]\nname='internal'\nurl='{registry}'\nindex_url='none'\n")).unwrap();
    let mut command = user.vibe();
    command
        .current_dir(project.path())
        .env("HOME", &user.settings)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .args([
            "install",
            "--json",
            "--assume-yes",
            "--no-default-registry",
            "--exclude-hosting",
            excluded,
            "--path",
        ])
        .arg(project.path());
    for key in [
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "ALL_PROXY",
        "http_proxy",
        "https_proxy",
        "all_proxy",
    ] {
        command.env_remove(key);
    }
    let output = command.output().unwrap();
    if output.status.success() {
        let lock: toml::Value =
            toml::from_str(&fs::read_to_string(project.path().join("vibe.lock")).unwrap()).unwrap();
        assert_eq!(lock["package"][0]["registry"].as_str(), Some("internal"));
        assert_eq!(lock["package"][0]["version"].as_str(), Some("1.0.0"));
    }
    output
}
fn refused_as_excluded(output: &std::process::Output) {
    assert!(!output.status.success());
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(text.contains("source_host_excluded"), "{text}");
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#SOURCE-POLICY-LAW")]
fn plain_http_registry_works_under_an_unrelated_exclusion_and_never_contacts_an_excluded_host() {
    let files = tempfile::tempdir().unwrap();
    let registry = Server::directory("127.0.0.1", plain_http_registry(files.path()));
    let user = quiet_user();
    // An exclusion of some other host selects the controlled transport; the
    // admitted plain-HTTP registry still installs through it.
    success(&controlled_install(&user, &registry.url, "host:127.0.0.3"));
    let admitted = registry.count();
    assert!(admitted > 0);
    // The registry's own host excluded: typed refusal with zero contacts.
    refused_as_excluded(&controlled_install(&user, &registry.url, "host:127.0.0.1"));
    assert_eq!(registry.count(), admitted);
    // A redirect from an admitted HTTP registry to the excluded host is refused
    // before the excluded host is contacted.
    let redirector = Server::redirect("127.0.0.2", registry.url.clone());
    refused_as_excluded(&controlled_install(
        &user,
        &redirector.url,
        "host:127.0.0.1",
    ));
    assert!(redirector.count() > 0);
    assert_eq!(registry.count(), admitted);
}
/// A served directory of dumb-HTTP Git repositories, one per `org.policy`
/// package name, each tagged `v1.0.0`.
fn served_packages(files: &Path, names: &[&str]) -> std::path::PathBuf {
    let served = files.join("served");
    fs::create_dir_all(&served).unwrap();
    for name in names {
        let work = files.join(format!("work-{name}"));
        fs::create_dir_all(&work).unwrap();
        git(&work, &["init", "-b", "main"]);
        fs::write(
            work.join("vibe.toml"),
            format!("[package]\ngroup='org.policy'\nname='{name}'\nversion='1.0.0'\nkind='tool'\n"),
        )
        .unwrap();
        git(&work, &["add", "vibe.toml"]);
        git(&work, &["commit", "-m", "fixture"]);
        git(&work, &["tag", "v1.0.0"]);
        let repo = served.join(format!("org.policy.{name}.git"));
        git(
            &work,
            &[
                "clone",
                "--bare",
                work.to_str().unwrap(),
                repo.to_str().unwrap(),
            ],
        );
        git(&repo, &["update-server-info"]);
    }
    served
}
fn plan_hostings(user: &UserScratch, project: &Path, excluded: Option<&str>) -> Vec<Value> {
    let mut command = user.vibe();
    command
        .current_dir(project)
        .env("HOME", &user.settings)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .args([
            "install",
            "--plan-only",
            "--json",
            "--no-default-registry",
            "--path",
        ])
        .arg(project);
    if let Some(excluded) = excluded {
        command.args(["--exclude-hosting", excluded]);
    }
    for key in [
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "ALL_PROXY",
        "http_proxy",
        "https_proxy",
        "all_proxy",
    ] {
        command.env_remove(key);
    }
    let output = command.output().unwrap();
    success(&output);
    assert!(!project.join("vibe.lock").exists(), "a plan writes no lock");
    let plan: Value = serde_json::from_slice(&output.stdout).unwrap();
    let _: vibe_wire::generated::install::e2::plan::InstallPlan =
        serde_json::from_value(plan.clone()).expect("canonical reader decodes the plan");
    plan["hostings"].as_array().unwrap().clone()
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#HOSTING-OBSERVATION")]
fn project_plan_lists_every_hosting_it_would_contact_and_marks_an_excluded_one() {
    let files = tempfile::tempdir().unwrap();
    let a = Server::directory(
        "127.0.0.1",
        served_packages(&files.path().join("a"), &["demo"]),
    );
    let b = Server::directory(
        "127.0.0.2",
        served_packages(&files.path().join("b"), &["demo", "other"]),
    );
    let user = quiet_user();
    let project = tempfile::tempdir().unwrap();
    fs::write(project.path().join("vibe.toml"),format!("[project]\nname='hostings'\nversion='1.0.0'\n[requires.packages]\n'org.policy/demo'='*'\n'org.policy/other'='*'\n[[registry]]\nname='a'\nurl='{}'\nindex_url='none'\n[[registry]]\nname='b'\nurl='{}'\nindex_url='none'\n",a.url,b.url)).unwrap();
    let rows = plan_hostings(&user, project.path(), None);
    let summary: Vec<_> = rows
        .iter()
        .map(|row| {
            (
                row["hosting"].as_str().unwrap(),
                row["excluded"].as_bool().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        summary,
        [("host:127.0.0.1", false), ("host:127.0.0.2", false)],
        "{rows:?}"
    );
    assert!(
        rows[0]["packages"]
            .as_array()
            .unwrap()
            .contains(&"org.policy/demo".into())
    );
    assert!(
        rows[1]["packages"]
            .as_array()
            .unwrap()
            .contains(&"org.policy/other".into())
    );
    assert!(a.count() > 0 && b.count() > 0);
    // The switch's effect before the press: host A is listed as excluded and
    // receives zero contacts; B serves the whole closure.
    let contacted = a.count();
    let rows = plan_hostings(&user, project.path(), Some("host:127.0.0.1"));
    let excluded = rows
        .iter()
        .find(|row| row["hosting"] == "host:127.0.0.1")
        .unwrap_or_else(|| panic!("the excluded hosting is listed: {rows:?}"));
    assert_eq!(excluded["excluded"], true);
    let admitted = rows
        .iter()
        .find(|row| row["hosting"] == "host:127.0.0.2")
        .unwrap();
    assert_eq!(admitted["excluded"], false);
    assert_eq!(
        a.count(),
        contacted,
        "an excluded hosting has zero contacts"
    );
}
