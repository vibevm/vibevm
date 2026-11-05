//! Git, not a pathname guess, interprets origin spellings in their original context.
#![cfg(test)]
#![cfg(unix)]
use super::*;

#[test]
fn existing_relative_and_tilde_origins_prepare_in_the_slots_git_context() {
    for origin in [
        "../../org/org.vibevm.giant.git",
        "~/org/org.vibevm.giant.git",
    ] {
        let f = Fixture::new();
        f.publish("1.0.0", "OLD", false);
        f.seed();
        let home = f._root.path().to_string_lossy().replace('\'', "'\\''");
        fs::write(f._root.path().join("fixture-git"),format!("#!/bin/sh\nexport HOME='{home}'\nexport GIT_CONFIG_NOSYSTEM=1\nexport GIT_CONFIG_GLOBAL=/dev/null\nexec git -c core.hooksPath=/dev/null -c protocol.file.allow=always \"$@\"\n")).unwrap();
        git(&f.slot, &["remote", "set-url", "origin", origin]);
        let commit = f.publish("1.1.0", "NEW", false);
        let r = f.registry();
        let before = tree(&f.slot);
        let p = r
            .prepare_in_place(&f.selected(&r, "1.1.0"), &f.slot)
            .unwrap();
        assert_eq!(tree(&f.slot), before);
        p.materialise().unwrap();
        assert_eq!(git(&f.slot, &["rev-parse", "HEAD"]), commit);
        assert_eq!(
            git(&f.slot, &["config", "--get", "remote.origin.url"]),
            origin
        );
    }
}

#[test]
fn fresh_registry_origin_spelling_matches_incumbent_clone_behavior() {
    for tilde in [false, true] {
        let f = Fixture::new();
        let home = f._root.path().to_string_lossy().replace('\'', "'\\''");
        fs::write(f._root.path().join("fixture-git"),format!("#!/bin/sh\nexport HOME='{home}'\nexport GIT_CONFIG_NOSYSTEM=1\nexport GIT_CONFIG_GLOBAL=/dev/null\nexec git -c core.hooksPath=/dev/null -c protocol.file.allow=always \"$@\"\n")).unwrap();
        f.publish("1.0.0", "OLD", false);
        let mut relative = PathBuf::new();
        for _ in std::env::current_dir()
            .unwrap()
            .components()
            .filter(|p| matches!(p, std::path::Component::Normal(_)))
        {
            relative.push("..");
        }
        relative.push(f.upstream.parent().unwrap().strip_prefix("/").unwrap());
        let org = if tilde {
            "~/org"
        } else {
            relative.to_str().unwrap()
        };
        let url = format!("{org}/org.vibevm.giant.git");
        let control = f._root.path().join("ordinary-clone");
        let ordinary = f.backend.bootstrap(&url, "v1.0.0", &control);
        let r = registry_with(&f.cache, org, NamingConvention::Fqdn, f.backend.clone());
        let prepared = r.prepare_in_place(&f.selected(&r, "1.0.0"), &f.slot);
        if let Err(error) = ordinary {
            // Git's existing fetch supports ~/ origins, but the installed
            // Git's fresh clone may reject that literal spelling. Preserve
            // the observed old boundary rather than inventing clone support.
            assert!(
                tilde && error.to_string().contains("does not exist"),
                "{error}"
            );
            assert!(prepared.is_err());
            assert!(!f.slot.exists());
            continue;
        }
        let expected = git(&control, &["config", "--get", "remote.origin.url"]);
        let p = prepared.unwrap();
        p.materialise().unwrap();
        drop(p);
        assert_eq!(
            git(&f.slot, &["config", "--get", "remote.origin.url"]),
            expected
        );
        f.publish("1.1.0", "NEW", false);
        let p = r
            .prepare_in_place(&f.selected(&r, "1.1.0"), &f.slot)
            .unwrap();
        p.materialise().unwrap();
        assert_eq!(
            fs::read_to_string(f.slot.join("payload.txt")).unwrap(),
            "NEW"
        );
    }
}

// Every file outside object acquisition and FETCH_HEAD remains exact, including
// HEAD, index, config, refs, packed-refs, reflogs and repository hook definitions.
#[cfg(test)]
fn semantic_git_files(slot: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, path: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            let relative = path.strip_prefix(root).unwrap();
            if relative.starts_with("objects") || relative == Path::new("FETCH_HEAD") {
                continue;
            }
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                out.insert(relative.into(), fs::read(path).unwrap());
            }
        }
    }
    let root = slot.join(".git");
    let mut out = BTreeMap::new();
    walk(&root, &root, &mut out);
    out
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION")]
fn native_transport_context_preserves_actual_source_and_semantic_git_state() {
    for common in [true, false] {
        for conditional in [false, true] {
            let actual = Fixture::new();
            actual.publish("1.0.0", "OLD", false);
            actual.seed();
            let expected = actual.publish("1.1.0", "ACTUAL", common);
            let decoy = Fixture::new();
            decoy.publish("1.1.0", "WRONG_SOURCE", false);
            let quote = |p: &Path| format!("'{}'", p.to_string_lossy().replace('\'', "'\\''"));
            let ssh = |name: &str, source: &Path| {
                let path = actual._root.path().join(name);
                fs::write(
                    &path,
                    format!("#!/bin/sh\nexec git-upload-pack {}\n", quote(source)),
                )
                .unwrap();
                fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
                path
            };
            let actual_ssh = ssh("actual-ssh", &actual.upstream);
            let wrong_ssh = ssh("wrong-ssh", &decoy.upstream);
            let wrapper = actual._root.path().join("fixture-git");
            fs::write(&wrapper,format!("#!/bin/sh\nexport GIT_CONFIG_NOSYSTEM=1\nexport GIT_CONFIG_GLOBAL=/dev/null\nexport GIT_SSH={}\nexport GIT_SSH_VARIANT=ssh\nexec git -c protocol.file.allow=always \"$@\"\n", quote(&wrong_ssh))).unwrap();
            git(
                &actual.slot,
                &["remote", "set-url", "origin", "fixture:repo"],
            );
            if conditional {
                let config = actual._root.path().join("transport-config");
                git(
                    actual._root.path(),
                    &[
                        "config",
                        "--file",
                        config.to_str().unwrap(),
                        "core.sshCommand",
                        &quote(&actual_ssh),
                    ],
                );
                let key = format!(
                    "includeIf.gitdir:{}.path",
                    actual.slot.join(".git").display()
                );
                git(&actual.slot, &["config", &key, config.to_str().unwrap()]);
            } else {
                git(
                    &actual.slot,
                    &["config", "core.sshCommand", &quote(&actual_ssh)],
                );
            }
            let native_transport = Command::new(&wrapper)
                .args(["config", "--get", "core.sshCommand"])
                .current_dir(&actual.slot)
                .output()
                .unwrap();
            assert!(
                native_transport.status.success(),
                "conditional transport fixture must be active"
            );
            assert_eq!(
                String::from_utf8(native_transport.stdout).unwrap().trim(),
                quote(&actual_ssh)
            );
            for (key, value) in [
                ("fetch.prune", "true"),
                ("fetch.pruneTags", "true"),
                ("fetch.writeCommitGraph", "true"),
                ("gc.auto", "1"),
                ("maintenance.auto", "true"),
            ] {
                git(&actual.slot, &["config", key, value]);
            }
            git(&actual.slot, &["tag", "local-only"]);
            fs::write(actual.slot.join("payload.txt"), "DIRTY").unwrap();
            fs::write(actual.slot.join("untracked.tmp"), "KEEP").unwrap();
            let hook = actual.slot.join(".git/hooks/reference-transaction");
            let canary = actual._root.path().join("hook-ran");
            fs::write(
                &hook,
                format!("#!/bin/sh\nprintf hook >> {}\n", quote(&canary)),
            )
            .unwrap();
            fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();
            let r = actual.registry();
            let selected = actual.selected(&r, "1.1.0");
            let before = tree(&actual.slot);
            let metadata = semantic_git_files(&actual.slot);
            let prepared = r.prepare_in_place(&selected, &actual.slot).unwrap();
            assert_eq!(
                prepared.resolved_commit(),
                expected,
                "common={common}, conditional={conditional}"
            );
            assert_eq!(prepared.manifest().configuration.is_some(), common);
            assert_eq!(tree(&actual.slot), before);
            assert_eq!(semantic_git_files(&actual.slot), metadata);
            assert!(!canary.exists());
            if !common {
                // Applying the admitted legacy source must not consult either
                // remote transport again, even after the actual source is gone.
                fs::rename(&actual.upstream, actual.upstream.with_extension("gone")).unwrap();
                prepared.materialise().unwrap();
                drop(prepared);
                assert_eq!(git(&actual.slot, &["rev-parse", "HEAD"]), expected);
                assert_eq!(
                    fs::read_to_string(actual.slot.join("payload.txt")).unwrap(),
                    "ACTUAL"
                );
                git(&actual.slot, &["fsck", "--full"]);
            }
        }
    }
}

#[cfg(test)]
fn acquired_source_failure_stays_terminal(case: &str) -> RegistryError {
    let f = Fixture::new();
    f.publish("1.0.0", "OLD", false);
    f.seed();
    // Resolution metadata and a valid fallback both name the canonical source.
    // Existing-slot refresh actually acquires the other origin below.
    f.publish("1.1.0", "VALID_CANONICAL_MIRROR", false);
    let actual = Fixture::new();
    actual.publish("1.1.0", "ACTUAL", false);
    match case {
        "missing" => fs::remove_file(actual.upstream.join("vibe.toml")).unwrap(),
        "invalid" => fs::write(actual.upstream.join("vibe.toml"), "[package\n").unwrap(),
        "attributes" => fs::create_dir(f.slot.join(".git/info/attributes")).unwrap(),
        _ => panic!("unknown fixture case"),
    }
    if case != "attributes" {
        git(&actual.upstream, &["add", "-A"]);
        git(
            &actual.upstream,
            &["commit", "-m", "broken acquired manifest"],
        );
        git(&actual.upstream, &["tag", "-f", "v1.1.0"]);
    }
    git(
        &f.slot,
        &[
            "remote",
            "set-url",
            "origin",
            actual.upstream.to_str().unwrap(),
        ],
    );
    let org = f.upstream.parent().unwrap().to_str().unwrap();
    let r = registry_with_mirrors(
        &f.cache,
        org,
        NamingConvention::Fqdn,
        vec![org.to_owned()],
        f.backend.clone(),
    );
    let selected = f.selected(&r, "1.1.0");
    let before = tree(&f.slot);
    let metadata = semantic_git_files(&f.slot);
    let error = r
        .prepare_in_place(&selected, &f.slot)
        .expect_err("an acquired invalid source must not become a valid legacy mirror's success");
    assert_eq!(tree(&f.slot), before);
    assert_eq!(semantic_git_files(&f.slot), metadata);
    error
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION")]
fn acquired_missing_manifest_cannot_fall_back_to_valid_legacy_mirror() {
    let error = acquired_source_failure_stays_terminal("missing");
    assert!(
        matches!(error, RegistryError::Git(GitError::CommandFailed { .. })),
        "{error}"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION")]
fn acquired_invalid_manifest_cannot_fall_back_to_valid_legacy_mirror() {
    let error = acquired_source_failure_stays_terminal("invalid");
    assert!(matches!(error, RegistryError::Core(_)), "{error}");
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION")]
fn acquired_attribute_read_failure_cannot_fall_back_to_valid_legacy_mirror() {
    let error = acquired_source_failure_stays_terminal("attributes");
    assert!(
        matches!(&error, RegistryError::Git(GitError::PreparedValidation { source }) if matches!(source.as_ref(), GitError::Io { cmd, .. } if cmd.contains("attribute"))),
        "{error}"
    );
}

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-069#PACKAGE-ADMISSION")]
fn acquisition_transport_failure_still_allows_prepared_legacy_mirror() {
    let f = Fixture::new();
    f.publish("1.0.0", "OLD", false);
    f.seed();
    let expected = f.publish("1.1.0", "VALID_MIRROR", false);
    let unavailable = f._root.path().join("unavailable-origin.git");
    git(
        &f.slot,
        &["remote", "set-url", "origin", unavailable.to_str().unwrap()],
    );
    let org = f.upstream.parent().unwrap().to_str().unwrap();
    let r = registry_with_mirrors(
        &f.cache,
        org,
        NamingConvention::Fqdn,
        vec![org.to_owned()],
        f.backend.clone(),
    );
    let before = tree(&f.slot);
    let metadata = semantic_git_files(&f.slot);
    let prepared = r
        .prepare_in_place(&f.selected(&r, "1.1.0"), &f.slot)
        .unwrap();
    assert_eq!(prepared.resolved_commit(), expected);
    assert!(prepared.manifest().configuration.is_none());
    assert_eq!(tree(&f.slot), before);
    assert_eq!(semantic_git_files(&f.slot), metadata);
}
