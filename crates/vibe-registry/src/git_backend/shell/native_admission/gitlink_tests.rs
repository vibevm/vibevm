//! Real exact-gitlink acquisition and composed native staging.
#![cfg(all(test, unix))]
use super::*;

#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-ROUTES-LAW")]
fn fresh_https_child_without_package_manifest_is_exact_and_denied_child_has_no_tcp() {
    exact_children(false, false);
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-ROUTES-LAW")]
fn nested_relative_children_stage_and_bind_exact_commits() {
    exact_children(true, false);
}
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-073#NATIVE-ROUTES-LAW")]
fn authenticated_child_inherits_bound_parent_token() {
    exact_children(false, true);
}
#[cfg(test)]
fn exact_children(nested: bool, authenticated: bool) {
    let f = Fixture::new();
    let temp = tempfile::tempdir().unwrap();
    let child = temp.path().join("child");
    std::fs::create_dir(&child).unwrap();
    assert!(fixture_git(&f.info, &["init"], &child).status.success());
    if nested {
        let leaf = temp.path().join("leaf");
        std::fs::create_dir(&leaf).unwrap();
        assert!(fixture_git(&f.info, &["init"], &leaf).status.success());
        std::fs::write(leaf.join("leaf-payload"), "leaf bytes\n").unwrap();
        assert!(fixture_git(&f.info, &["add", "."], &leaf).status.success());
        assert!(
            fixture_git(
                &f.info,
                &[
                    "-c",
                    "user.name=Fixture",
                    "-c",
                    "user.email=f@invalid",
                    "commit",
                    "-m",
                    "leaf"
                ],
                &leaf
            )
            .status
            .success()
        );
        let oid = String::from_utf8(fixture_git(&f.info, &["rev-parse", "HEAD"], &leaf).stdout)
            .unwrap()
            .trim()
            .to_owned();
        let bare = Path::new(f.get("root")).join("web/leaf.git");
        assert!(
            fixture_git(
                &f.info,
                &[
                    "clone",
                    "--bare",
                    leaf.to_str().unwrap(),
                    bare.to_str().unwrap()
                ],
                temp.path()
            )
            .status
            .success()
        );
        assert!(
            fixture_git(&f.info, &["update-server-info"], &bare)
                .status
                .success()
        );
        std::fs::write(
            child.join(".gitmodules"),
            "[submodule \"leaf\"]\n path=leaf\n url=../leaf.git\n",
        )
        .unwrap();
        assert!(
            fixture_git(
                &f.info,
                &[
                    "update-index",
                    "--add",
                    "--cacheinfo",
                    &format!("160000,{oid},leaf")
                ],
                &child
            )
            .status
            .success()
        );
    }
    std::fs::write(child.join("payload"), "pinned child\n").unwrap();
    if nested {
        assert!(
            fixture_git(&f.info, &["add", ".gitmodules"], &child)
                .status
                .success()
        );
    }
    assert!(
        fixture_git(&f.info, &["add", "payload"], &child)
            .status
            .success()
    );
    assert!(
        fixture_git(
            &f.info,
            &[
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=f@invalid",
                "commit",
                "-m",
                "pinned"
            ],
            &child
        )
        .status
        .success()
    );
    let pinned = String::from_utf8(fixture_git(&f.info, &["rev-parse", "HEAD"], &child).stdout)
        .unwrap()
        .trim()
        .to_owned();
    std::fs::write(child.join("payload"), "later child\n").unwrap();
    assert!(
        fixture_git(&f.info, &["add", "payload"], &child)
            .status
            .success()
    );
    assert!(
        fixture_git(
            &f.info,
            &[
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=f@invalid",
                "commit",
                "-m",
                "later"
            ],
            &child
        )
        .status
        .success()
    );
    let bare = Path::new(f.get("root")).join(if authenticated {
        "web/auth/child.git"
    } else {
        "web/child.git"
    });
    std::fs::create_dir_all(bare.parent().unwrap()).unwrap();
    assert!(
        fixture_git(
            &f.info,
            &[
                "clone",
                "--bare",
                child.to_str().unwrap(),
                bare.to_str().unwrap()
            ],
            temp.path()
        )
        .status
        .success()
    );
    assert!(
        fixture_git(&f.info, &["update-server-info"], &bare)
            .status
            .success()
    );
    let parent = Path::new(f.get("work"));
    std::fs::write(
        parent.join(".gitmodules"),
        format!(
            "[submodule \"child\"]\n path=child\n url={}{}/child.git\n",
            f.get(if nested || authenticated { "a" } else { "b" }),
            if authenticated { "/auth" } else { "" }
        ),
    )
    .unwrap();
    assert!(
        fixture_git(&f.info, &["add", ".gitmodules"], parent)
            .status
            .success()
    );
    assert!(
        fixture_git(
            &f.info,
            &[
                "update-index",
                "--add",
                "--cacheinfo",
                &format!("160000,{pinned},child")
            ],
            parent
        )
        .status
        .success()
    );
    assert!(
        fixture_git(
            &f.info,
            &[
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=f@invalid",
                "commit",
                "-m",
                "gitlink"
            ],
            parent
        )
        .status
        .success()
    );
    assert!(
        fixture_git(&f.info, &["tag", "v3.0.0"], parent)
            .status
            .success()
    );
    let parent_bare = Path::new(f.get("root")).join("web/repo.git");
    assert!(
        fixture_git(
            &f.info,
            &[
                "fetch",
                parent.to_str().unwrap(),
                "refs/tags/v3.0.0:refs/tags/v3.0.0"
            ],
            &parent_bare
        )
        .status
        .success()
    );
    assert!(
        fixture_git(&f.info, &["update-server-info"], &parent_bare)
            .status
            .success()
    );
    let parent_url = if authenticated {
        format!(
            "https://x-access-token:SYNTHETIC_N1_AUTH@{}/auth.git",
            f.get("a").strip_prefix("https://").unwrap()
        )
    } else {
        format!("{}/repo.git", f.get("a"))
    };
    for (denied, bind_source) in [(false, false), (false, true), (true, false)] {
        if authenticated && denied {
            continue;
        }
        let before = f.connections("b");
        let selected =
            NativeConfigSelection::new(temp.path().into(), Path::new(f.get("root")).join("home"))
                .unwrap()
                .with_variable("GIT_CONFIG_NOSYSTEM", "1".into())
                .unwrap()
                .with_variable("GIT_CONFIG_GLOBAL", "/dev/null".into())
                .unwrap()
                .with_variable("GIT_ATTR_NOSYSTEM", "1".into())
                .unwrap();
        let selected = if nested {
            selected
                .with_variable("GIT_CONFIG_COUNT", "1".into())
                .unwrap()
                .with_variable(
                    "GIT_CONFIG_KEY_0",
                    format!("url.{}/leaf.git.insteadOf", f.get("b")).into(),
                )
                .unwrap()
                .with_variable(
                    "GIT_CONFIG_VALUE_0",
                    format!("{}/leaf.git", f.get("a")).into(),
                )
                .unwrap()
        } else {
            selected
        };
        let profile = NativeGitAdmission::direct_https(
            Arc::new(
                crate::HttpAdmission::new(if denied { vec!["127.0.0.2"] } else { vec![] }).unwrap(),
            ),
            GitInstallation::trusted("/usr/bin/git".into(), "/usr/lib/git-core".into()).unwrap(),
            TlsTrust::Pem(std::fs::read(f.get("ca")).unwrap()),
        )
        .with_config_selection(selected);
        let profile = Arc::new(profile);
        if authenticated {
            let denied_git = ShellGit::new()
                .with_native_admission(profile.clone())
                .bound_for_registry_auth(vibe_core::manifest::AuthKind::None)
                .unwrap()
                .unwrap();
            let before = f.connections("a");
            let denied = denied_git.prepare_ref(
                &parent_url,
                "v3.0.0",
                None,
                &temp.path().join("no-authority.git"),
                &temp.path().join("no-authority-slot"),
            );
            assert!(denied.is_err());
            assert_eq!(f.connections("a"), before);
        }
        let git = ShellGit::new().with_native_admission(profile);
        let git = git
            .bound_for_registry_auth(if authenticated {
                vibe_core::manifest::AuthKind::TokenEnv
            } else {
                vibe_core::manifest::AuthKind::None
            })
            .unwrap()
            .unwrap();
        let objects = temp.path().join(if denied {
            "denied.git"
        } else if bind_source {
            "bound.git"
        } else {
            "owned.git"
        });
        let slot = temp.path().join(if denied {
            "denied-slot"
        } else if bind_source {
            "bound-slot"
        } else {
            "slot"
        });
        let result = if bind_source {
            git.prepare_native_source(&parent_url, "v3.0.0", None, &objects)
                .and_then(|source| source.unwrap().bind(&slot))
        } else {
            git.prepare_ref(&parent_url, "v3.0.0", None, &objects, &slot)
        };
        if denied {
            assert!(result.is_err());
            assert_eq!(f.connections("b"), before);
            assert!(!slot.exists());
            continue;
        }
        let revision = result.unwrap();
        let stage = git
            .stage_native_materialisation(
                &objects,
                &revision,
                crate::git_backend::PayloadBytes::AsConfigured,
            )
            .unwrap()
            .unwrap();
        assert!(!slot.exists());
        assert_eq!(
            std::fs::read(stage.staged_path().join("child/payload")).unwrap(),
            b"pinned child\n"
        );
        assert!(!stage.staged_path().join("child/vibe.toml").exists());
        if nested {
            assert_eq!(
                std::fs::read(stage.staged_path().join("child/leaf/leaf-payload")).unwrap(),
                b"leaf bytes\n"
            );
        }
        let head = fixture_git(
            &f.info,
            &["rev-parse", "HEAD"],
            &stage.staged_path().join("child"),
        );
        assert!(head.status.success());
        assert_eq!(String::from_utf8(head.stdout).unwrap().trim(), pinned);
        let status = fixture_git(&f.info, &["submodule", "status"], stage.staged_path());
        assert!(
            status.status.success(),
            "{}",
            String::from_utf8_lossy(&status.stderr)
        );
        assert!(
            String::from_utf8(status.stdout)
                .unwrap()
                .starts_with(&format!(" {pinned} "))
        );
        stage.validate_before_publication().unwrap();
        if !bind_source && !authenticated {
            for absorbed in [false, true] {
                for scenario in ["same-dirty", "other-clean", "other-conflict", "occupied"] {
                    if nested && scenario != "same-dirty" {
                        continue;
                    }
                    let warm = temp.path().join(format!("{scenario}-{absorbed}"));
                    stage.copy_for_cache(&warm).unwrap();
                    if absorbed {
                        assert!(
                            fixture_git(&f.info, &["submodule", "absorbgitdirs"], &warm)
                                .status
                                .success()
                        );
                    }
                    let child = warm.join("child");
                    if scenario != "same-dirty" && scenario != "occupied" {
                        let later = String::from_utf8(
                            fixture_git(&f.info, &["rev-parse", "HEAD"], &bare).stdout,
                        )
                        .unwrap()
                        .trim()
                        .to_owned();
                        assert!(
                            fixture_git(&f.info, &["checkout", "--detach", &later], &child)
                                .status
                                .success()
                        );
                    }
                    if scenario != "other-clean" {
                        std::fs::write(child.join("payload"), "dirty child\n").unwrap();
                        std::fs::write(child.join("untracked"), "untracked child\n").unwrap();
                    }
                    if scenario == "occupied" {
                        let metadata = child.join(".git");
                        if metadata.is_dir() {
                            std::fs::remove_dir_all(metadata).unwrap();
                        } else {
                            std::fs::remove_file(metadata).unwrap();
                        }
                    }
                    let child_connections = f.connections("b");
                    let witness = materialisation::tree(&warm).unwrap();
                    let acquired = temp.path().join(format!("{scenario}-{absorbed}.git"));
                    let revision =
                        git.prepare_ref(&parent_url, "v3.0.0", Some(&warm), &acquired, &warm);
                    if scenario == "occupied" {
                        assert!(revision.is_err());
                        assert_eq!(f.connections("b"), child_connections);
                        assert_eq!(materialisation::tree(&warm).unwrap(), witness);
                        continue;
                    }
                    let revision = revision.unwrap();
                    let staged = git.stage_native_materialisation(
                        &acquired,
                        &revision,
                        crate::git_backend::PayloadBytes::AsConfigured,
                    );
                    assert_eq!(materialisation::tree(&warm).unwrap(), witness);
                    if scenario == "other-conflict" {
                        assert!(staged.is_err());
                    } else {
                        let staged = staged.unwrap().unwrap();
                        assert_eq!(
                            std::fs::read(staged.staged_path().join("child/payload")).unwrap(),
                            if scenario == "same-dirty" {
                                b"dirty child\n".as_slice()
                            } else {
                                b"pinned child\n".as_slice()
                            }
                        );
                        if scenario == "same-dirty" {
                            assert_eq!(
                                std::fs::read(staged.staged_path().join("child/untracked"))
                                    .unwrap(),
                                b"untracked child\n"
                            );
                        }
                        staged.validate_before_publication().unwrap();
                        if nested {
                            let leaf = staged.staged_path().join("child/leaf");
                            let output =
                                fixture_git(&f.info, &["rev-parse", "--show-toplevel"], &leaf);
                            assert!(
                                output.status.success(),
                                "{}",
                                String::from_utf8_lossy(&output.stderr)
                            );
                            assert_eq!(
                                String::from_utf8(output.stdout).unwrap().trim(),
                                leaf.to_str().unwrap()
                            );
                            assert_eq!(
                                std::fs::read(leaf.join("leaf-payload")).unwrap(),
                                b"leaf bytes\n"
                            );
                        }
                        if absorbed {
                            assert!(staged.staged_path().join("child/.git").is_file());
                            let output = fixture_git(
                                &f.info,
                                &["rev-parse", "--show-toplevel"],
                                &staged.staged_path().join("child"),
                            );
                            assert!(
                                output.status.success(),
                                "{}",
                                String::from_utf8_lossy(&output.stderr)
                            );
                            assert_eq!(
                                String::from_utf8(output.stdout).unwrap().trim(),
                                staged.staged_path().join("child").to_str().unwrap()
                            );
                        }
                    }
                }
            }
        }
        if bind_source {
            std::fs::create_dir_all(slot.join("child")).unwrap();
            std::fs::write(slot.join("child/foreign"), "foreign").unwrap();
            assert!(stage.validate_before_publication().is_err());
        }
    }
}

#[path = "gitlink_userinfo_tests.rs"]
mod userinfo;
