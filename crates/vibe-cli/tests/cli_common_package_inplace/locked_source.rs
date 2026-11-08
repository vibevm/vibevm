//! Actual commands replay held source authority after user registry edits.
#![cfg(all(test, unix))]
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE");
use super::*;

#[cfg(test)]
fn current_registry(f: &Fixture, org: &Path) {
    let source = format!("git+file://{}", org.display());
    write(
        &f.user.settings.join("registry.toml"),
        format!("[[registry]]\nname='default'\nurl='{source}'\nindex_url='none'\n"),
    );
    write(
        &f.project.join("vibe.toml"),
        "[project]\nname='demo'\nversion='0.0.1'\n[requires.packages]\n'org.vibevm/held'='^1'\n",
    );
}
#[cfg(test)]
fn annotated_lock(f: &Fixture) -> Vec<u8> {
    let path = f.project.join("vibe.lock");
    let mut bytes = fs::read(&path).unwrap();
    bytes.extend_from_slice(b"\n# preserve this exact shared lock spelling\n");
    write(&path, &bytes);
    bytes
}
#[cfg(test)]
fn alternate(f: &Fixture) -> PathBuf {
    let directory = f.root.join("edited-registry");
    publish(
        &directory.join("org.vibevm.held.git"),
        "held",
        "1.0.0",
        false,
    );
    write(
        &directory.join("org.vibevm.held.git/payload.txt"),
        "WRONG CURRENT REGISTRY\n",
    );
    git(
        &directory.join("org.vibevm.held.git"),
        &["add", "payload.txt"],
    );
    git(
        &directory.join("org.vibevm.held.git"),
        &["commit", "-m", "different source"],
    );
    git(
        &directory.join("org.vibevm.held.git"),
        &["tag", "-f", "v1.0.0"],
    );
    directory
}
#[cfg(test)]
fn cold(f: &Fixture) {
    fs::remove_dir_all(f.slot("held")).unwrap();
    fs::remove_dir_all(&f.user.cache).unwrap();
    fs::create_dir_all(&f.user.cache).unwrap();
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE-LAW")]
fn edited_user_registry_cannot_retarget_warm_or_cold_explicit_install() {
    let f = Fixture::new(&["held"]);
    let pinned = git(&f.slot("held"), &["rev-parse", "HEAD"]);
    let previous = annotated_lock(&f);
    let other = alternate(&f);
    current_registry(&f, &other);
    let sidecar = f.project.join("vibe.lock.provenance");
    if sidecar.exists() {
        fs::remove_file(&sidecar).unwrap();
    }
    let before = snapshot(&f.slot("held"));
    f.success(&f.run("held-warm", &["install", "org.vibevm/held@^1"]));
    assert_eq!(before, snapshot(&f.slot("held")));
    assert_eq!(previous, fs::read(f.project.join("vibe.lock")).unwrap());
    cold(&f);
    f.success(&f.run("held-cold", &["install", "org.vibevm/held@^1"]));
    assert_eq!(pinned, git(&f.slot("held"), &["rev-parse", "HEAD"]));
    assert_eq!(previous, fs::read(f.project.join("vibe.lock")).unwrap());
    let trace = fs::read_to_string(f.root.join("evidence/held-cold.git-trace.jsonl")).unwrap();
    assert!(
        !trace.contains(other.to_str().unwrap()),
        "current registry was contacted: {trace}"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE-LAW")]
fn missing_locked_source_fails_without_current_registry_fallback_or_writes() {
    let f = Fixture::new(&["held"]);
    let previous = annotated_lock(&f);
    let other = alternate(&f);
    current_registry(&f, &other);
    cold(&f);
    fs::rename(f.repo("held"), f.root.join("unreachable-held.git")).unwrap();
    let before = snapshot(&f.project);
    let result = f.run("held-unreachable", &["install", "org.vibevm/held@^1"]);
    assert!(!result.status.success());
    assert_eq!(before, snapshot(&f.project));
    assert_eq!(previous, fs::read(f.project.join("vibe.lock")).unwrap());
    let trace =
        fs::read_to_string(f.root.join("evidence/held-unreachable.git-trace.jsonl")).unwrap();
    assert!(
        !trace.contains(other.to_str().unwrap()),
        "current registry was contacted: {trace}"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE-LAW")]
fn mirrored_checkout_replays_offline_without_sidecar_or_origin_authority() {
    let f = Fixture::new(&["held"]);
    let previous = annotated_lock(&f);
    let mirror = f.root.join("old-mirror.git");
    git(
        &f.root,
        &[
            "clone",
            "--bare",
            f.repo("held").to_str().unwrap(),
            mirror.to_str().unwrap(),
        ],
    );
    git(
        &f.slot("held"),
        &["remote", "set-url", "origin", mirror.to_str().unwrap()],
    );
    current_registry(&f, &alternate(&f));
    let sidecar = f.project.join("vibe.lock.provenance");
    if sidecar.exists() {
        fs::remove_file(&sidecar).unwrap();
    }
    let before = snapshot(&f.slot("held"));
    f.success(&f.run(
        "held-offline",
        &["install", "org.vibevm/held@^1", "--offline"],
    ));
    assert_eq!(before, snapshot(&f.slot("held")));
    assert_eq!(previous, fs::read(f.project.join("vibe.lock")).unwrap());
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE-LAW")]
fn deliberate_update_selects_new_version_at_the_held_endpoint() {
    let f = Fixture::new(&["held"]);
    let old = Lockfile::read(f.project.join("vibe.lock")).unwrap();
    let expected = publish(&f.repo("held"), "held", "1.1.0", false);
    let other = alternate(&f);
    publish(&other.join("org.vibevm.held.git"), "held", "1.9.0", false);
    current_registry(&f, &other);
    f.success(&f.run("held-update", &["update", "org.vibevm/held"]));
    let updated = Lockfile::read(f.project.join("vibe.lock")).unwrap();
    assert_eq!(updated.packages[0].source_url, old.packages[0].source_url);
    assert_eq!(updated.packages[0].version.to_string(), "1.1.0");
    assert_eq!(git(&f.slot("held"), &["rev-parse", "HEAD"]), expected);
}

// These are valid current-schema held rows. The fixture changes only authored
// source facts; original commit/content still identify the already installed tree.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE-LAW")]
fn absent_kind_and_ref_remain_absent_in_schema_seven_and_eight_replay() {
    for schema in [7, 8] {
        let f = Fixture::new(&["held"]);
        let path = f.project.join("vibe.lock");
        let mut lock = Lockfile::read(&path).unwrap();
        lock.packages[0].source_kind = None;
        lock.packages[0].source_ref = None;
        lock.write(&path).unwrap();
        let text = fs::read_to_string(&path)
            .unwrap()
            .replace("schema_version = 7", &format!("schema_version = {schema}"));
        write(&path, text);
        assert_eq!(Lockfile::read(&path).unwrap().meta.schema_version, schema);
        let original = annotated_lock(&f);
        current_registry(&f, &alternate(&f));
        f.success(&f.run("unknown-kind-warm", &["install", "org.vibevm/held@^1"]));
        assert_eq!(original, fs::read(&path).unwrap());
        cold(&f);
        f.success(&f.run("unknown-kind-cold", &["install", "org.vibevm/held@^1"]));
        assert_eq!(original, fs::read(&path).unwrap());
        assert_eq!(Lockfile::read(&path).unwrap().packages[0].source_kind, None);
        assert_eq!(Lockfile::read(&path).unwrap().packages[0].source_ref, None);
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE-LAW")]
fn held_git_and_override_replay_preserve_facts_then_update_the_same_branch_without_tags() {
    use vibe_core::manifest::SourceKind;
    for kind in [SourceKind::Git, SourceKind::Override] {
        let f = Fixture::new(&["held"]);
        let path = f.project.join("vibe.lock");
        let mut lock = Lockfile::read(&path).unwrap();
        let row = &mut lock.packages[0];
        row.source_kind = Some(kind);
        row.overridden = kind == SourceKind::Override;
        row.registry = None;
        row.source_ref = Some("refs/heads/main".into());
        // vibe records Git/Override rows with the bare transport URL; the
        // `git+` spelling belongs only to registry-served rows.
        let bare = row
            .source_url
            .as_str()
            .trim_start_matches("git+")
            .to_owned();
        row.source_url = vibe_core::SourceUrl::new(bare);
        lock.write(&path).unwrap();
        let original = annotated_lock(&f);
        current_registry(&f, &alternate(&f));
        cold(&f);
        f.success(&f.run("explicit-source-replay", &["install", "org.vibevm/held@^1"]));
        assert_eq!(original, fs::read(&path).unwrap());
        git(&f.repo("held"), &["tag", "-d", "v1.0.0"]);
        let expected = publish(&f.repo("held"), "held", "1.1.0", false);
        git(&f.repo("held"), &["tag", "-d", "v1.1.0"]);
        assert!(git(&f.repo("held"), &["tag", "--list"]).is_empty());
        f.success(&f.run("explicit-source-update", &["update", "org.vibevm/held"]));
        let updated = Lockfile::read(&path).unwrap();
        let row = &updated.packages[0];
        assert_eq!(row.source_kind, Some(kind));
        assert_eq!(row.overridden, kind == SourceKind::Override);
        assert_eq!(row.registry, None);
        assert_eq!(row.source_ref.as_deref(), Some("refs/heads/main"));
        assert_eq!(row.source_url, lock.packages[0].source_url);
        assert_eq!(row.version.to_string(), "1.1.0");
        assert_eq!(git(&f.slot("held"), &["rev-parse", "HEAD"]), expected);
    }
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE-LAW")]
fn undemanded_missing_current_local_registry_does_not_block_held_replay() {
    let f = Fixture::new(&["held"]);
    let original = annotated_lock(&f);
    current_registry(&f, &alternate(&f));
    write(
        &f.user.settings.join("registry.toml"),
        format!(
            "[[registry]]\nname='missing'\nurl='file://{}'\n",
            f.root.join("missing-directory").display()
        ),
    );
    f.success(&f.run("missing-catalog-held", &["install", "org.vibevm/held@^1"]));
    assert_eq!(original, fs::read(f.project.join("vibe.lock")).unwrap());
    assert!(
        !f.run("missing-catalog-new", &["install", "org.vibevm/new-name"])
            .status
            .success()
    );
    assert_eq!(original, fs::read(f.project.join("vibe.lock")).unwrap());
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE-LAW")]
fn selected_member_offline_metadata_uses_the_owning_workspace_checkout() {
    let f = Fixture::new(&["held"]);
    let member = f.project.join("member");
    write(
        &member.join("vibe.toml"),
        "[project]\nname='member'\nversion='0.0.1'\n[requires.packages]\n'org.vibevm/held'='^1'\n",
    );
    let root = fs::read_to_string(f.project.join("vibe.toml")).unwrap();
    write(
        &f.project.join("vibe.toml"),
        format!("{root}\n[workspace]\nmembers=['member']\n"),
    );
    let original = annotated_lock(&f);
    let before = snapshot(&f.slot("held"));
    f.success(&f.run_at(
        "member-held-offline",
        &["install", "org.vibevm/held@^1", "--offline"],
        &member,
    ));
    assert_eq!(before, snapshot(&f.slot("held")));
    assert_eq!(original, fs::read(f.project.join("vibe.lock")).unwrap());
    assert!(
        !member
            .join(common::deps_root())
            .join("org.vibevm.held")
            .exists()
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE-LAW")]
fn legacy_unqualified_direct_ref_still_requires_authored_clarification_for_update() {
    let f = Fixture::new(&["held"]);
    let path = f.project.join("vibe.lock");
    let mut lock = Lockfile::read(&path).unwrap();
    let row = &mut lock.packages[0];
    row.source_kind = Some(vibe_core::manifest::SourceKind::Git);
    row.registry = None;
    row.source_ref = Some("main".into());
    lock.write(&path).unwrap();
    current_registry(&f, &alternate(&f));
    let before = fs::read(&path).unwrap();
    let output = f.run("legacy-ambiguous-update", &["update", "org.vibevm/held"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("legacy direct ref"));
    assert_eq!(before, fs::read(path).unwrap());
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#LOCKED-SOURCE-LAW")]
fn member_scoped_offline_preview_reuses_the_owning_workspace_in_place_slot() {
    let f = Fixture::new(&["held"]);
    let member = f.project.join("member");
    fs::create_dir(&member).unwrap();
    write(
        &member.join("vibe.toml"),
        "[project]\nname='member'\nversion='1.0.0'\n[requires.packages]\n'org.vibevm/held'='=1.0.0'\n",
    );
    let root = f.project.join("vibe.toml");
    let mut authored = fs::read_to_string(&root).unwrap();
    authored.push_str("\n[workspace]\nmembers=['member']\n");
    write(&root, authored);
    let lock = fs::read(f.project.join("vibe.lock")).unwrap();
    let head = git(&f.slot("held"), &["rev-parse", "HEAD"]);
    if f.user.cache.exists() {
        fs::remove_dir_all(&f.user.cache).unwrap();
        fs::create_dir_all(&f.user.cache).unwrap();
    }
    let store = f.user.settings.join("cache");
    if store.exists() {
        fs::remove_dir_all(&store).unwrap();
    }
    fs::rename(f.repo("held"), f.root.join("unreachable-held-preview.git")).unwrap();
    let before = snapshot(&f.project);
    let output = f
        .user
        .vibe()
        .current_dir(&f.project)
        .args([
            "install",
            "--plan-only",
            "--offline",
            "--json",
            "--unattended",
            "--path",
        ])
        .arg(&member)
        .env("HOME", f.root.join("home"))
        .env("USERPROFILE", f.root.join("home"))
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_ALLOW_PROTOCOL", "file")
        .output()
        .unwrap();
    f.success(&output);
    assert_eq!(before, snapshot(&f.project));
    assert_eq!(lock, fs::read(f.project.join("vibe.lock")).unwrap());
    assert_eq!(head, git(&f.slot("held"), &["rev-parse", "HEAD"]));
    assert!(
        !member
            .join(vibe_core::layout::current_vibedeps_root())
            .exists()
    );
    assert!(!store.exists());
}
