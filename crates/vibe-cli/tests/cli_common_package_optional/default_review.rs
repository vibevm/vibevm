//! Narrow ordinary defaults use shared policy; changed reliance stays closed.
use super::*;

#[test]
fn ordinary_closure_fallback_actual_install_boundary_and_requires_precedence() {
    for reversed in [false, true] {
        for world in [
            "explicit",
            "requires",
            "explicit-fallback-member",
            "requires-fallback-member",
            "fallback",
        ] {
            let f = Fixture::new(false);
            let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
            let original_source = fs::read_to_string(&source).unwrap();
            let prefix = original_source
                .split("[[configuration.features]]")
                .next()
                .unwrap();
            let declarations = [
                "[[configuration.features]]\nid='enabled'\nname='Fallback member'\neffects=[{kind='dependency',target='org.vibevm/oracle-extra'}]\n",
                "[[configuration.features]]\nid='disabled'\nname='Explicit or implied member'\n",
                "[[configuration.features]]\nid='trigger'\nname='Trigger'\nrequires=['disabled']\n",
            ];
            let ordered = if reversed {
                declarations.iter().rev().copied().collect::<String>()
            } else {
                declarations.concat()
            };
            let text = format!(
                "{prefix}{ordered}\n[[configuration.groups]]\nmode='closure'\nid='extra'\nname='Closure support'\ncardinality='exactly-one'\nmembers=['enabled','disabled']\nfallback=['enabled']\n"
            );
            let text = if world == "requires-fallback-member" {
                text.replace("requires=['disabled']", "requires=['enabled']")
            } else {
                text
            };
            let manifest = Manifest::parse_str(&text).unwrap();
            write(&source, &text);
            let seeds = match world {
                "explicit" => "['disabled']",
                "requires" | "requires-fallback-member" => "['trigger']",
                "explicit-fallback-member" => "['enabled']",
                _ => "[]",
            };
            let request = f.project.join("vibe.toml");
            let input = format!(
                "configuration={{policy='require-selection',default_features=false,features={seeds}}}"
            );
            let authored=fs::read_to_string(&request).unwrap().replace("version='^1'","version='=1.0.0'").replace("configuration={policy='require-selection',default_features=false,selections={extra={intent='exact',members=['disabled']}}}",&input);
            write(&request, &authored);
            let original = Manifest::read(&request).unwrap().requires.configurations
                [&coordinate(DRIVER)]
                .clone();
            let definition = manifest.configuration.as_ref().unwrap();
            let normalized = definition
                .normalize_input(&original, "./vibe.toml")
                .unwrap();
            let evaluated = definition
                .compile_choices()
                .unwrap()
                .evaluate(&normalized.features, &[normalized.choices]);
            let vibe_core::configuration::ChoiceOutcome::Resolved { features } = evaluated.outcome
            else {
                panic!("kernel must resolve {world}")
            };
            let fallback = features.implied.iter().any(|i| {
                matches!(
                    i.reason,
                    vibe_core::configuration::ImplicationReason::GroupDefault { .. }
                )
            });
            assert_eq!(fallback, world == "fallback");
            let enabled = matches!(
                world,
                "fallback" | "explicit-fallback-member" | "requires-fallback-member"
            );
            assert_eq!(features.selected.contains(&"enabled".into()), enabled);
            let out = f.install();
            let show = f
                .user
                .vibe()
                .args([
                    "show",
                    "features",
                    "--path",
                    f.project.to_str().unwrap(),
                    "--json",
                ])
                .output()
                .unwrap();
            assert!(
                show.status.success(),
                "public inspection must admit actual installed history"
            );
            if world == "fallback" {
                assert!(
                    out.status.success(),
                    "{}",
                    String::from_utf8_lossy(&out.stderr)
                );
                let lock = f.lock();
                let record = &lock.configurations[&coordinate(DRIVER)];
                assert_eq!(lock.meta.schema_version, 19);
                let row = lock.find(&GROUP.parse().unwrap(), DRIVER).unwrap();
                assert_eq!(row.version.to_string(), "1.0.0");
                assert_eq!(row.registry.as_deref(), Some("fixture"));
                assert_eq!(
                    row.content_hash.as_str(),
                    vibe_registry::compute_content_hash(source.parent().unwrap()).unwrap()
                );
                assert_eq!(record.consumers["./vibe.toml"], original);
                assert!(record.automatic_basis.is_none());
                assert_eq!(
                    record.closure_fallback_basis.as_ref().unwrap().groups,
                    std::collections::BTreeMap::from([("extra".into(), "enabled".into())])
                );
                assert!(f.slot(EXTRA).exists());
                assert!(f.static_boot().contains(EXTRA_MARKER));
                assert_eq!(f.hooks().get(EXTRA), Some(&1));
                let before = super::workspace::semantic_snapshot(&f.project);
                fs::rename(&f.registry, f.root.join("registry-unavailable")).unwrap();
                f.install_ok();
                assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
                assert_eq!(f.hooks().get(EXTRA), Some(&1));
            } else {
                assert!(
                    out.status.success(),
                    "{}",
                    String::from_utf8_lossy(&out.stderr)
                );
                let lock = f.lock();
                let record = &lock.configurations[&coordinate(DRIVER)];
                assert_eq!(record.consumers["./vibe.toml"], original);
                assert!(record.automatic_basis.is_none());
                assert!(record.closure_fallback_basis.is_none());
                assert_eq!(lock.meta.schema_version, 8);
                assert_eq!(record.features.contains(&"disabled".into()), !enabled);
                assert_eq!(record.features.contains(&"enabled".into()), enabled);
                assert_eq!(
                    lock.packages.iter().any(|p| p.name.as_str() == EXTRA),
                    enabled
                );
                assert_eq!(f.slot(EXTRA).exists(), enabled);
                assert_eq!(f.static_boot().contains(EXTRA_MARKER), enabled);
                assert_eq!(f.hooks().get(EXTRA), if enabled { Some(&1) } else { None });
                let before = super::workspace::semantic_snapshot(&f.project);
                fs::rename(&f.registry, f.root.join("registry-unavailable")).unwrap();
                f.install_ok();
                assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
            }
            let before = super::workspace::semantic_snapshot(&f.project);
            let prior = std::env::var_os("NST_OPTIONAL_BOOL_OLD_CLI")
                .expect("explicit actual prior reader/writer");
            for operation in ["reader", "writer"] {
                let mut cmd = std::process::Command::new(&prior);
                cmd.env_clear()
                    .env("HOME", f.root.join("home"))
                    .env("VIBE_SETTINGS", &f.user.settings)
                    .env("PATH", "/usr/bin:/bin")
                    .env("VIBE_NO_DEFAULT_REGISTRY", "1");
                if operation == "reader" {
                    cmd.args(["show", "features"]);
                } else {
                    cmd.args(["install", "--offline", "--assume-yes"]);
                }
                let out = cmd
                    .args(["--path", f.project.to_str().unwrap(), "--json"])
                    .output()
                    .unwrap();
                assert_eq!(
                    out.status.success(),
                    world != "fallback",
                    "prior {world}/{operation}: {}",
                    String::from_utf8_lossy(&out.stderr)
                );
                assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
            }
            let hidden = f.root.join("registry-unavailable");
            fs::rename(&hidden, &f.registry).unwrap();
            if world == "fallback" {
                let good = f.lock();
                for mutation in [
                    "missing",
                    "empty",
                    "wrong-member",
                    "wrong-group",
                    "fingerprint",
                    "old-epoch",
                ] {
                    let mut changed = good.clone();
                    let record = changed.configurations.get_mut(&coordinate(DRIVER)).unwrap();
                    match mutation {
                        "missing" => record.closure_fallback_basis = None,
                        "empty" => record
                            .closure_fallback_basis
                            .as_mut()
                            .unwrap()
                            .groups
                            .clear(),
                        "wrong-member" => {
                            record
                                .closure_fallback_basis
                                .as_mut()
                                .unwrap()
                                .groups
                                .insert("extra".into(), "disabled".into());
                        }
                        "wrong-group" => {
                            record
                                .closure_fallback_basis
                                .as_mut()
                                .unwrap()
                                .groups
                                .insert("invented".into(), "enabled".into());
                        }
                        "fingerprint" => {
                            record
                                .closure_fallback_basis
                                .as_mut()
                                .unwrap()
                                .definition_fingerprint = "forged".into()
                        }
                        "old-epoch" => changed.meta.schema_version = 18,
                        _ => unreachable!(),
                    }
                    write(
                        &f.project.join("vibe.lock"),
                        toml::to_string(&changed).unwrap(),
                    );
                    let before = super::workspace::semantic_snapshot(&f.project);
                    assert!(
                        Lockfile::read(f.project.join("vibe.lock")).is_err(),
                        "{mutation}"
                    );
                    assert!(!f.install().status.success());
                    assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
                }
                write(
                    &f.project.join("vibe.lock"),
                    toml::to_string(&good).unwrap(),
                );
                let next = f.registry.join(GROUP).join(DRIVER).join("v1.1.0");
                write(
                    &next.join("vibe.toml"),
                    fs::read_to_string(&source)
                        .unwrap()
                        .replace("version='1.0.0'", "version='1.1.0'")
                        .replace("fallback=['enabled']", "fallback=['disabled']"),
                );
                write(&next.join("boot/rules.md"), "REJECTED_CLOSURE_DEFAULT\n");
                let before = super::workspace::semantic_snapshot(&f.project);
                f.install_ok();
                assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
                assert!(!f.static_boot().contains("REJECTED_CLOSURE_DEFAULT"));
                assert_eq!(
                    f.lock().configurations[&coordinate(DRIVER)].closure_fallback_basis,
                    good.configurations[&coordinate(DRIVER)].closure_fallback_basis
                );
                write(
                    &request,
                    authored.replace("version='=1.0.0'", "version='=1.1.0'"),
                );
                let before = super::workspace::semantic_snapshot(&f.project);
                let out = update(&f);
                assert!(!out.status.success());
                assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
                assert!(!f.static_boot().contains("REJECTED_CLOSURE_DEFAULT"));
                let scalar = review_fixture();
                let proposal = update(&scalar);
                let proposal: serde_json::Value = serde_json::from_slice(&proposal.stderr).unwrap();
                let token = proposal["proposal"]["review_id"].as_str().unwrap();
                assert!(!respond(&f, token).status.success());
                assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
                assert!(
                    !respond(
                        &f,
                        &format!("package-member-set-review/2:sha256:{}", "0".repeat(64))
                    )
                    .status
                    .success()
                );
                assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
            } else {
                let mut changed = f.lock();
                changed.meta.schema_version = 19;
                changed
                    .configurations
                    .get_mut(&coordinate(DRIVER))
                    .unwrap()
                    .closure_fallback_basis = Some(vibe_core::manifest::ClosureFallbackBasis {
                    definition_fingerprint: definition.semantic_fingerprint().unwrap(),
                    groups: std::collections::BTreeMap::from([("extra".into(), "enabled".into())]),
                });
                assert!(
                    changed.validate_configurations().is_err(),
                    "explicit/derived cannot receive invented reason"
                );
            }
            assert_eq!(
                fs::read_to_string(&request).unwrap(),
                if world == "fallback" {
                    authored.replace("version='=1.0.0'", "version='=1.1.0'")
                } else {
                    authored
                }
            );
            println!(
                "closure-fallback world={world} reversed={reversed} kernel-fallback={fallback} public-success={}",
                out.status.success()
            );
        }
    }
}

fn configure(fixture: &Fixture, enabled: bool) {
    let path = fixture.project.join("vibe.toml");
    let original = fs::read_to_string(&path).unwrap();
    let explicit = "configuration={policy='require-selection',default_features=false,selections={extra={intent='exact',members=['disabled']}}}";
    assert!(original.contains(explicit));
    write(
        &path,
        original.replace(
            explicit,
            "configuration={policy='use-declared-defaults',default_features=false}",
        ),
    );
    if enabled {
        let source = fixture
            .registry
            .join(GROUP)
            .join(DRIVER)
            .join("v1.0.0/vibe.toml");
        let text = fs::read_to_string(&source).unwrap();
        write(
            &source,
            text.replace("defaults = [\"disabled\"]", "defaults = [\"enabled\"]"),
        );
    }
}
fn update(f: &Fixture) -> Output {
    f.user
        .vibe()
        .current_dir(&f.project)
        .env("HOME", f.root.join("home"))
        .env("USERPROFILE", f.root.join("home"))
        .args([
            "update",
            "--all",
            "--offline",
            "--assume-yes",
            "--json",
            "--path",
            f.project.to_str().unwrap(),
        ])
        .output()
        .unwrap()
}
#[test]
fn omitted_exactly_one_defaults_on_and_off_install_and_cold_replay_without_rewrite() {
    for enabled in [false, true] {
        let fixture = Fixture::new(false);
        configure(&fixture, enabled);
        let authored = fs::read(fixture.project.join("vibe.toml")).unwrap();
        fixture.install_ok();
        let lock = fixture.lock();
        assert_eq!(lock.meta.schema_version, 12);
        let record = &lock.configurations[&coordinate(DRIVER)];
        assert!(record.consumers.values().all(|i| i.selections.is_empty()));
        let basis = record.automatic_basis.as_ref().unwrap();
        assert_eq!(basis.consumer_choices.len(), record.consumers.len());
        for consumer in record.consumers.keys() {
            assert_eq!(
                basis.consumer_choices[consumer]["extra"],
                if enabled { "enabled" } else { "disabled" }
            );
        }
        assert_eq!(fixture.slot(EXTRA).exists(), enabled);
        let before = fs::read(fixture.project.join("vibe.lock")).unwrap();
        fixture.install_ok();
        assert_eq!(fs::read(fixture.project.join("vibe.lock")).unwrap(), before);
        assert_eq!(
            fs::read(fixture.project.join("vibe.toml")).unwrap(),
            authored
        );
    }
}
#[test]
fn changed_relied_on_default_refuses_update_before_effects() {
    let fixture = Fixture::new(false);
    configure(&fixture, false);
    fixture.install_ok();
    let before = fs::read(fixture.project.join("vibe.lock")).unwrap();
    let boot = fixture.static_boot();
    let hooks = fixture.hooks();
    // A new local version is source change, not mutation of a pinned cache.
    let old = fixture.registry.join(GROUP).join(DRIVER).join("v1.0.0");
    let new = fixture.registry.join(GROUP).join(DRIVER).join("v1.1.0");
    write(
        &new.join("vibe.toml"),
        fs::read_to_string(old.join("vibe.toml"))
            .unwrap()
            .replace("version='1.0.0'", "version='1.1.0'")
            .replace("defaults = [\"disabled\"]", "defaults = [\"enabled\"]"),
    );
    write(
        &new.join("boot/rules.md"),
        fs::read(old.join("boot/rules.md")).unwrap(),
    );
    let result = update(&fixture);
    let diagnostic = format!(
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(!result.status.success(), "{diagnostic}");
    assert!(
        diagnostic.contains("package configuration review"),
        "{diagnostic}"
    );
    assert_eq!(fs::read(fixture.project.join("vibe.lock")).unwrap(), before);
    assert_eq!(fixture.static_boot(), boot);
    assert_eq!(fixture.hooks(), hooks);
    assert!(!fixture.slot(EXTRA).exists());
    let control = Fixture::new(false);
    control.install_ok();
    write(
        &control
            .registry
            .join(GROUP)
            .join(DRIVER)
            .join("v1.1.0/vibe.toml"),
        fs::read(new.join("vibe.toml")).unwrap(),
    );
    write(
        &control
            .registry
            .join(GROUP)
            .join(DRIVER)
            .join("v1.1.0/boot/rules.md"),
        fs::read(new.join("boot/rules.md")).unwrap(),
    );
    let result = update(&control);
    assert!(
        result.status.success(),
        "explicit control failed: {}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(!control.slot(EXTRA).exists());
    assert_eq!(
        control
            .lock()
            .find(&GROUP.parse().unwrap(), DRIVER)
            .unwrap()
            .version
            .to_string(),
        "1.1.0"
    );
    let unchanged = Fixture::new(false);
    configure(&unchanged, false);
    unchanged.install_ok();
    let old = unchanged.registry.join(GROUP).join(DRIVER).join("v1.0.0");
    let new = unchanged.registry.join(GROUP).join(DRIVER).join("v1.1.0");
    write(
        &new.join("vibe.toml"),
        fs::read_to_string(old.join("vibe.toml"))
            .unwrap()
            .replace("version='1.0.0'", "version='1.1.0'"),
    );
    write(
        &new.join("boot/rules.md"),
        fs::read(old.join("boot/rules.md")).unwrap(),
    );
    let result = update(&unchanged);
    assert!(
        result.status.success(),
        "unchanged basis update failed: {}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        unchanged
            .lock()
            .find(&GROUP.parse().unwrap(), DRIVER)
            .unwrap()
            .version
            .to_string(),
        "1.1.0"
    );
    assert!(!unchanged.slot(EXTRA).exists());
}

#[test]
fn governed_update_rejects_corrupt_basis_and_named_route_before_effects() {
    let fixture = Fixture::new(false);
    configure(&fixture, false);
    fixture.install_ok();
    let lock_path = fixture.project.join("vibe.lock");
    let before = fs::read(&lock_path).unwrap();
    let boot = fixture.static_boot();
    let hooks = fixture.hooks();
    let named = fixture
        .user
        .vibe()
        .current_dir(&fixture.project)
        .env("HOME", fixture.root.join("home"))
        .args([
            "update",
            &coordinate(DRIVER),
            "--offline",
            "--assume-yes",
            "--json",
            "--path",
            fixture.project.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!named.status.success());
    assert_eq!(fs::read(&lock_path).unwrap(), before);
    let text = String::from_utf8(before.clone()).unwrap();
    let record = fixture.lock();
    let fingerprint = &record.configurations[&coordinate(DRIVER)]
        .automatic_basis
        .as_ref()
        .unwrap()
        .definition_fingerprint;
    assert!(text.contains(fingerprint));
    write(
        &lock_path,
        text.replace(fingerprint, "configuration-semantic/1:sha256:invalid"),
    );
    let corrupted = fs::read(&lock_path).unwrap();
    let result = update(&fixture);
    assert!(!result.status.success());
    assert_eq!(fs::read(&lock_path).unwrap(), corrupted);
    assert_eq!(fixture.static_boot(), boot);
    assert_eq!(fixture.hooks(), hooks);
    assert!(!fixture.slot(EXTRA).exists());
}

#[test]
fn explicit_unchanged_input_cannot_leave_unreachable_old_dependency_on_update() {
    let fixture = Fixture::new(false);
    fixture.install_ok();
    let lock = fs::read(fixture.project.join("vibe.lock")).unwrap();
    let boot = fixture.static_boot();
    let hooks = fixture.hooks();
    let old = fixture.registry.join(GROUP).join(DRIVER).join("v1.0.0");
    let new = fixture.registry.join(GROUP).join(DRIVER).join("v1.1.0");
    let text = fs::read_to_string(old.join("vibe.toml"))
        .unwrap()
        .replace("version='1.0.0'", "version='1.1.0'")
        .replace("\"org.vibevm/oracle-base\" = \"^1\"", "");
    // Remove the dormant effect mentioning the removed mandatory declaration
    // as well, so the replacement is valid rather than malformed metadata.
    let text = text.replace(
        "effects = [{ kind = \"dependency\", target = \"org.vibevm/oracle-base\" }]",
        "",
    );
    Manifest::parse_str(&text).unwrap();
    write(&new.join("vibe.toml"), text);
    write(
        &new.join("boot/rules.md"),
        fs::read(old.join("boot/rules.md")).unwrap(),
    );
    let result = update(&fixture);
    let diagnostic = format!(
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(!result.status.success(), "{diagnostic}");
    assert!(
        diagnostic.contains("graph membership reconciliation"),
        "{diagnostic}"
    );
    assert_eq!(fs::read(fixture.project.join("vibe.lock")).unwrap(), lock);
    assert_eq!(
        fixture
            .lock()
            .find(&GROUP.parse().unwrap(), DRIVER)
            .unwrap()
            .version
            .to_string(),
        "1.0.0"
    );
    assert_eq!(fixture.static_boot(), boot);
    assert_eq!(fixture.hooks(), hooks);
    assert!(fixture.slot(BASE).exists());
    assert!(
        !vibe_workspace::vibedeps::slot_abs_path(
            &fixture.project,
            &GROUP.parse().unwrap(),
            DRIVER,
            &"1.1.0".parse().unwrap()
        )
        .exists()
    );
}

#[test]
fn changed_default_response_shows_exact_review_subject_without_implicit_acceptance() {
    let fixture = Fixture::new(false);
    configure(&fixture, false);
    fixture.install_ok();
    let before = fs::read(fixture.project.join("vibe.lock")).unwrap();
    let boot = fixture.static_boot();
    let hooks = fixture.hooks();
    let old = fixture.registry.join(GROUP).join(DRIVER).join("v1.0.0");
    let new = fixture.registry.join(GROUP).join(DRIVER).join("v1.1.0");
    write(
        &new.join("vibe.toml"),
        fs::read_to_string(old.join("vibe.toml"))
            .unwrap()
            .replace("version='1.0.0'", "version='1.1.0'")
            .replace("defaults = [\"disabled\"]", "defaults = [\"enabled\"]"),
    );
    write(
        &new.join("boot/rules.md"),
        fs::read(old.join("boot/rules.md")).unwrap(),
    );
    let result = update(&fixture); // Includes --assume-yes: never acknowledgment.
    assert!(!result.status.success());
    assert!(
        result.stdout.is_empty(),
        "ordinary failure does not emit a proposal document"
    );
    let error: serde_json::Value = serde_json::from_slice(&result.stderr).unwrap();
    assert_eq!(error["ok"], false);
    assert_eq!(error["code"], "package_configuration_review_required");
    assert_eq!(error["proposal"]["from_member"], "disabled");
    assert_eq!(error["proposal"]["to_member"], "enabled");
    assert_eq!(error["proposal"]["group"], "extra");
    assert_eq!(
        error["proposal"]["added_dependencies"],
        serde_json::json!([coordinate(EXTRA)])
    );
    assert!(error.get("expected_definition").is_none());
    assert_eq!(fs::read(fixture.project.join("vibe.lock")).unwrap(), before);
    assert_eq!(fixture.static_boot(), boot);
    assert_eq!(fixture.hooks(), hooks);
    assert!(!fixture.slot(EXTRA).exists());
    if let Some(dir) = std::env::var_os("NST_PACKAGE_ACK_EVIDENCE") {
        let dir = PathBuf::from(dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("stdout"), &result.stdout).unwrap();
        fs::write(dir.join("stderr.json"), &result.stderr).unwrap();
        fs::write(
            dir.join("exit"),
            result.status.code().unwrap_or(-1).to_string(),
        )
        .unwrap();
    }
}

fn respond(f: &Fixture, review: &str) -> Output {
    f.user
        .vibe()
        .current_dir(&f.project)
        .env("HOME", f.root.join("home"))
        .args([
            "update",
            "--all",
            "--offline",
            "--assume-yes",
            "--json",
            "--package-configuration-review",
            review,
            "--path",
            f.project.to_str().unwrap(),
        ])
        .output()
        .unwrap()
}
fn review_fixture() -> Fixture {
    let f = Fixture::new(false);
    configure(&f, false);
    boundaries::add_root_for_review(&f, EXTRA);
    f.install_ok();
    let old = f.registry.join(GROUP).join(DRIVER).join("v1.0.0");
    let new = f.registry.join(GROUP).join(DRIVER).join("v1.1.0");
    write(
        &new.join("vibe.toml"),
        fs::read_to_string(old.join("vibe.toml"))
            .unwrap()
            .replace("version='1.0.0'", "version='1.1.0'")
            .replace("defaults = [\"disabled\"]", "defaults = [\"enabled\"]")
            + "\n[hooks]\npre-install='hooks/review-once'\n",
    );
    write(
        &new.join("boot/rules.md"),
        fs::read(old.join("boot/rules.md")).unwrap(),
    );
    write(
        &new.join("hooks/review-once.sh"),
        "set -eu\nprintf '%s\\n' 'review-driver' >> \"$VIBE_PROJECT_ROOT/effects.log\"\n",
    );
    f
}
#[test]
fn exact_review_applies_once_and_wrong_workspace_or_prior_lock_is_stale() {
    let f = review_fixture();
    let path = f.project.join("vibe.lock");
    let prior = fs::read(&path).unwrap();
    let hooks = f.hooks();
    let shown = update(&f);
    assert_eq!(shown.status.code(), Some(1));
    let proposal: serde_json::Value = serde_json::from_slice(&shown.stderr).unwrap();
    let token = proposal["proposal"]["review_id"].as_str().unwrap();
    assert_eq!(fs::read(&path).unwrap(), prior);
    let other = review_fixture();
    let before = fs::read(other.project.join("vibe.lock")).unwrap();
    let stale = respond(&other, token);
    assert_eq!(stale.status.code(), Some(1));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&stale.stderr).unwrap()["code"],
        "package_configuration_review_stale"
    );
    assert_eq!(fs::read(other.project.join("vibe.lock")).unwrap(), before);
    // The prior complete lock snapshot is part of the identity, including its
    // preserved bytes; this synthetic comment cannot authorize a prior token.
    let mut changed = prior.clone();
    changed.extend_from_slice(b"\n# changed prior snapshot\n");
    write(&path, &changed);
    let stale = respond(&f, token);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&stale.stderr).unwrap()["code"],
        "package_configuration_review_stale"
    );
    assert_eq!(fs::read(&path).unwrap(), changed);
    write(&path, &prior);
    let accepted = respond(&f, token);
    assert!(
        accepted.status.success(),
        "{}{}",
        String::from_utf8_lossy(&accepted.stdout),
        String::from_utf8_lossy(&accepted.stderr)
    );
    assert_eq!(
        f.lock()
            .find(&GROUP.parse().unwrap(), DRIVER)
            .unwrap()
            .version
            .to_string(),
        "1.1.0"
    );
    assert!(f.slot(EXTRA).exists());
    let lock = f.lock();
    let record = &lock.configurations[&coordinate(DRIVER)];
    let row = lock.find(&GROUP.parse().unwrap(), DRIVER).unwrap();
    assert_eq!(row.features, record.features);
    assert!(row.features.contains(&"enabled".to_owned()));
    let mut edges: Vec<_> = row
        .dependencies
        .iter()
        .map(|p| p.qualified_name())
        .collect();
    edges.sort();
    assert_eq!(edges, record.active_dependencies);
    assert_eq!(
        record
            .automatic_basis
            .as_ref()
            .unwrap()
            .consumer_choices
            .values()
            .next()
            .unwrap()["extra"],
        "enabled"
    );
    let after = fs::read(&path).unwrap();
    let hook_after = f.hooks();
    assert_eq!(hook_after.get("review-driver"), Some(&1));
    for (name, before) in &hooks {
        assert_eq!(
            hook_after[name], *before,
            "unchanged leaf does not repeat {name}"
        );
    }
    f.install_ok();
    assert_eq!(fs::read(&path).unwrap(), after);
    assert_eq!(f.hooks(), hook_after);
    let stale = respond(&f, token);
    assert!(!stale.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&stale.stderr).unwrap()["code"],
        "package_configuration_review_stale"
    );
    assert_eq!(fs::read(&path).unwrap(), after);
}

#[test]
fn review_candidate_drift_and_membership_change_cannot_reuse_authority() {
    let f = review_fixture();
    let path = f.project.join("vibe.lock");
    let before = fs::read(&path).unwrap();
    let boot = f.static_boot();
    let hooks = f.hooks();
    let shown = update(&f);
    let proposal: serde_json::Value = serde_json::from_slice(&shown.stderr).unwrap();
    let token = proposal["proposal"]["review_id"].as_str().unwrap();
    let old = f.registry.join(GROUP).join(DRIVER).join("v1.1.0");
    let new = f.registry.join(GROUP).join(DRIVER).join("v1.2.0");
    write(
        &new.join("vibe.toml"),
        fs::read_to_string(old.join("vibe.toml"))
            .unwrap()
            .replace("version='1.1.0'", "version='1.2.0'"),
    );
    write(
        &new.join("boot/rules.md"),
        fs::read(old.join("boot/rules.md")).unwrap(),
    );
    let stale = respond(&f, token);
    assert_eq!(stale.status.code(), Some(1));
    let error: serde_json::Value = serde_json::from_slice(&stale.stderr).unwrap();
    assert_eq!(error["code"], "package_configuration_review_stale");
    assert_eq!(error["proposal"]["to_version"], "1.2.0");
    assert_ne!(error["proposal"]["review_id"], token);
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(f.static_boot(), boot);
    assert_eq!(f.hooks(), hooks);
    let malformed = respond(&f, "package-default-review/1:sha256:bad");
    assert_eq!(malformed.status.code(), Some(2));
    assert_eq!(fs::read(&path).unwrap(), before);

    // Without the independent root, accepting the exact shown subject must
    // still refuse unsupported package membership changes.
    let g = Fixture::new(false);
    configure(&g, false);
    g.install_ok();
    let source = g.registry.join(GROUP).join(DRIVER).join("v1.1.0");
    write(
        &source.join("vibe.toml"),
        fs::read(old.join("vibe.toml")).unwrap(),
    );
    write(
        &source.join("boot/rules.md"),
        fs::read(old.join("boot/rules.md")).unwrap(),
    );
    let before = fs::read(g.project.join("vibe.lock")).unwrap();
    let boot = g.static_boot();
    let hooks = g.hooks();
    let shown = update(&g);
    let proposal: serde_json::Value = serde_json::from_slice(&shown.stderr).unwrap();
    let token = proposal["proposal"]["review_id"].as_str().unwrap();
    let refused = respond(&g, token);
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("graph membership reconciliation"));
    assert_eq!(fs::read(g.project.join("vibe.lock")).unwrap(), before);
    assert_eq!(g.static_boot(), boot);
    assert_eq!(g.hooks(), hooks);
    assert!(!g.slot(EXTRA).exists());
}

#[test]
fn selected_config_data_false_is_distinct_from_absent_and_cold_inspection_is_read_only() {
    for enabled in [false, true] {
        let fixture = Fixture::new(enabled);
        let path = fixture
            .registry
            .join(GROUP)
            .join(DRIVER)
            .join("v1.0.0/vibe.toml");
        let text = fs::read_to_string(&path).unwrap().replace("id = \"disabled\"\nname = \"No extra\"", "id = \"disabled\"\nname = \"No extra\"\neffects = [{ kind = \"config-value\", name = \"verbose\" }]");
        let text = text + "\n[configuration.config_values]\nverbose = false\n";
        Manifest::parse_str(&text).unwrap();
        write(&path, text);
        fixture.install_ok();
        let before = fs::read(fixture.project.join("vibe.lock")).unwrap();
        let slot = fs::read(fixture.slot(DRIVER).join("vibe.toml")).unwrap();
        let hooks = fixture.hooks();
        let inspect = || {
            fixture
                .user
                .vibe()
                .current_dir(&fixture.project)
                .args([
                    "show",
                    "configuration-data",
                    "--path",
                    fixture.project.to_str().unwrap(),
                    "--json",
                ])
                .output()
                .unwrap()
        };
        let output = inspect();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["protocol"], "vibe-package-configuration-data/1");
        let values = &report["packages"][0]["values"];
        if enabled {
            assert_eq!(values, &serde_json::json!({}));
        } else {
            assert_eq!(values["verbose"], false);
        }
        assert_eq!(fixture.slot(EXTRA).exists(), enabled);
        // Presentation-only live source edits must not replace historical data.
        write(
            &path,
            fs::read_to_string(&path)
                .unwrap()
                .replace("No extra", "Renamed presentation only"),
        );
        let cold = inspect();
        assert!(cold.status.success());
        assert_eq!(cold.stdout, output.stdout);
        assert_eq!(fs::read(fixture.project.join("vibe.lock")).unwrap(), before);
        assert_eq!(
            fs::read(fixture.slot(DRIVER).join("vibe.toml")).unwrap(),
            slot
        );
        assert_eq!(fixture.hooks(), hooks);
        if let Some(old) = std::env::var_os("NST_CONFIG_DATA_OLD_CLI") {
            let result = std::process::Command::new(&old)
                .env_clear()
                .env("HOME", fixture.root.join("home"))
                .env("VIBE_SETTINGS", &fixture.user.settings)
                .env("PATH", "/usr/bin:/bin")
                .args([
                    "show",
                    "features",
                    "--path",
                    fixture.project.to_str().unwrap(),
                    "--json",
                ])
                .output()
                .unwrap();
            if !enabled {
                assert!(!result.status.success());
                assert_eq!(fs::read(fixture.project.join("vibe.lock")).unwrap(), before);
                let writer = std::process::Command::new(&old)
                    .env_clear()
                    .env("HOME", fixture.root.join("home"))
                    .env("VIBE_SETTINGS", &fixture.user.settings)
                    .env("PATH", "/usr/bin:/bin")
                    .args([
                        "install",
                        "--path",
                        fixture.project.to_str().unwrap(),
                        "--offline",
                        "--assume-yes",
                        "--json",
                    ])
                    .output()
                    .unwrap();
                assert!(!writer.status.success());
                assert_eq!(fs::read(fixture.project.join("vibe.lock")).unwrap(), before);
                assert_eq!(
                    fs::read(fixture.slot(DRIVER).join("vibe.toml")).unwrap(),
                    slot
                );
                assert_eq!(fixture.hooks(), hooks);
            }
        }
        // Missing retained record must never be derived from the live manifest.
        let mut lock = fixture.lock();
        lock.configurations.clear();
        write(
            &fixture.project.join("vibe.lock"),
            toml::to_string(&lock).unwrap(),
        );
        let missing = inspect();
        assert!(!missing.status.success());
    }
}

#[test]
fn config_data_unknown_and_duplicate_keys_refuse_before_publication() {
    for unknown in [false, true] {
        let f = Fixture::new(false);
        let path = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
        let text=fs::read_to_string(&path).unwrap().replace("id = \"disabled\"\nname = \"No extra\"","id = \"disabled\"\nname = \"No extra\"\neffects = [{ kind = \"config-value\", name = \"verbose\" }]");
        let text = if unknown {
            text + "\n[configuration.config_values]\nother = true\n"
        } else {
            text + "\n[configuration.config_values]\nverbose = true\nverbose = false\n"
        };
        write(&path, text);
        let authored = fs::read(f.project.join("vibe.toml")).unwrap();
        let out = f.install();
        assert!(!out.status.success());
        assert_eq!(fs::read(f.project.join("vibe.toml")).unwrap(), authored);
        assert!(!f.project.join("vibe.lock").exists());
        assert!(!f.slot(DRIVER).exists());
        assert!(!f.project.join("effects.log").exists());
    }
}

#[test]
fn named_parameterless_profile_preserves_original_and_cold_replay_matches_explicit_control() {
    for profile in [Some("with-extra"), Some("unknown"), None] {
        let f = Fixture::new(true);
        let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
        let text = fs::read_to_string(&source).unwrap()
            + "\n[[configuration.profiles]]\nid='with-extra'\nname='With extra'\nselections.extra={intent='exact',members=['enabled']}\n";
        write(&source, &text);
        let request = f.project.join("vibe.toml");
        let explicit = fs::read_to_string(&request).unwrap();
        if let Some(profile) = profile {
            let replacement = format!(
                "configuration={{policy='require-selection',default_features=false,profile='{profile}'}}"
            );
            let original = "configuration={policy='require-selection',default_features=false,selections={extra={intent='exact',members=['enabled']}}}";
            assert!(explicit.contains(original));
            write(&request, explicit.replace(original, &replacement));
        }
        let authored = fs::read(&request).unwrap();
        let manifest = Manifest::read(&request).unwrap();
        let definition = Manifest::read(&source).unwrap().configuration.unwrap();
        let input = manifest.requires.configurations.values().next().unwrap();
        let normalized = definition.normalize_input(input, "./vibe.toml");
        if profile == Some("with-extra") {
            let normalized = normalized.unwrap();
            assert!(
                matches!(&normalized.choices.answers["extra"].selection,vibe_core::configuration::Selection::Explicit(values) if values==&vec!["enabled".to_owned()])
            );
            assert!(input.selections.is_empty());
        } else if profile == Some("unknown") {
            assert!(normalized.is_err());
        }
        let output = f.install();
        let diagnostic = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        match profile {
            Some("with-extra") => {
                assert!(output.status.success(), "{diagnostic}");
                assert!(f.slot(EXTRA).exists());
                let lock = f.lock();
                let record = &lock.configurations[&coordinate(DRIVER)];
                assert!(
                    record
                        .consumers
                        .values()
                        .all(|input| input.profile.as_deref() == Some("with-extra")
                            && input.selections.is_empty())
                );
                assert!(
                    record.automatic_basis.is_none(),
                    "profile is not a default-group receipt"
                );
                let before = fs::read(f.project.join("vibe.lock")).unwrap();
                let hooks = f.hooks();
                f.install_ok();
                assert_eq!(fs::read(f.project.join("vibe.lock")).unwrap(), before);
                assert_eq!(f.hooks(), hooks);
                if let Some(old) = std::env::var_os("NST_PROFILE_OLD_CLI") {
                    for args in [
                        vec![
                            "show",
                            "features",
                            "--path",
                            f.project.to_str().unwrap(),
                            "--json",
                        ],
                        vec![
                            "install",
                            "--path",
                            f.project.to_str().unwrap(),
                            "--offline",
                            "--assume-yes",
                            "--json",
                        ],
                    ] {
                        let result = std::process::Command::new(&old)
                            .env_clear()
                            .env("HOME", f.root.join("home"))
                            .env("VIBE_SETTINGS", &f.user.settings)
                            .env("PATH", "/usr/bin:/bin")
                            .args(args)
                            .output()
                            .unwrap();
                        assert!(!result.status.success());
                        assert_eq!(fs::read(f.project.join("vibe.lock")).unwrap(), before);
                        assert_eq!(f.hooks(), hooks);
                    }
                }
            }
            Some(_) => {
                assert!(!output.status.success());
                assert!(diagnostic.contains("unknown profile"), "{diagnostic}");
            }
            None => {
                assert!(output.status.success(), "{diagnostic}");
                assert!(f.slot(EXTRA).exists());
                let before = fs::read(f.project.join("vibe.lock")).unwrap();
                f.install_ok();
                assert_eq!(fs::read(f.project.join("vibe.lock")).unwrap(), before);
            }
        }
        assert_eq!(fs::read(&request).unwrap(), authored);
        if profile == Some("unknown") {
            assert!(!f.project.join("vibe.lock").exists());
            assert!(!f.slot(DRIVER).exists());
            assert!(!f.project.join("effects.log").exists());
        }
    }
}

#[test]
fn named_profile_meaning_drift_refuses_update_but_presentation_change_does_not() {
    for changed in [false, true] {
        let f = Fixture::new(true);
        let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
        let text = fs::read_to_string(&source).unwrap()
            + "\n[[configuration.profiles]]\nid='with-extra'\nname='With extra'\nselections.extra={intent='exact',members=['enabled']}\n";
        write(&source, text);
        let path = f.project.join("vibe.toml");
        let text=fs::read_to_string(&path).unwrap().replace("configuration={policy='require-selection',default_features=false,selections={extra={intent='exact',members=['enabled']}}}","configuration={policy='require-selection',default_features=false,profile='with-extra'}");
        write(&path, text);
        f.install_ok();
        let lock = fs::read(f.project.join("vibe.lock")).unwrap();
        let boot = f.static_boot();
        let hooks = f.hooks();
        let new = f.registry.join(GROUP).join(DRIVER).join("v1.1.0");
        let mut text = fs::read_to_string(&source)
            .unwrap()
            .replace("version='1.0.0'", "version='1.1.0'")
            .replace("name='With extra'", "name='Presentation changed'");
        if changed {
            text = text.replace(
                "selections.extra={intent='exact',members=['enabled']}",
                "selections.extra={intent='exact',members=['disabled']}",
            );
        }
        write(&new.join("vibe.toml"), text);
        write(
            &new.join("boot/rules.md"),
            fs::read(source.parent().unwrap().join("boot/rules.md")).unwrap(),
        );
        let out = update(&f);
        if changed {
            assert!(!out.status.success());
            assert!(String::from_utf8_lossy(&out.stderr).contains("retained configuration"));
            assert_eq!(fs::read(f.project.join("vibe.lock")).unwrap(), lock);
            assert_eq!(f.static_boot(), boot);
            assert_eq!(f.hooks(), hooks);
        } else {
            assert!(
                out.status.success(),
                "{}",
                String::from_utf8_lossy(&out.stderr)
            );
            assert_eq!(
                f.lock()
                    .find(&GROUP.parse().unwrap(), DRIVER)
                    .unwrap()
                    .version
                    .to_string(),
                "1.1.0"
            );
            assert!(f.slot(EXTRA).exists());
        }
    }
}

#[test]
fn public_boolean_parameter_false_omitted_and_wrong_type_have_distinct_normalization() {
    for value in [
        Some("false"),
        Some("true"),
        None,
        Some("'not-a-boolean'"),
        Some("control"),
    ] {
        let f = Fixture::new(false);
        let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
        if value != Some("control") {
            write(
                &source,
                fs::read_to_string(&source).unwrap()
                    + "\n[[configuration.parameters]]\nid='verbose'\nname='Verbose'\ntype='bool'\nrequired=true\n",
            );
        }
        let request = f.project.join("vibe.toml");
        if let Some(value) = value.filter(|v| *v != "control") {
            let text=fs::read_to_string(&request).unwrap().replace("configuration={policy='require-selection',default_features=false,selections={extra={intent='exact',members=['disabled']}}}",&format!("configuration={{policy='require-selection',default_features=false,selections={{extra={{intent='exact',members=['disabled']}}}},parameters={{verbose={value}}}}}"));
            write(&request, text);
        }
        let authored = fs::read(&request).unwrap();
        let manifest = Manifest::read(&request).unwrap();
        let definition = Manifest::read(&source).unwrap().configuration.unwrap();
        let input = manifest.requires.configurations.values().next().unwrap();
        let normalized = definition.normalize_input(input, "./vibe.toml");
        match value {
            Some("false" | "true") => assert_eq!(
                normalized.unwrap().parameters["verbose"],
                vibe_core::manifest::ParameterValue::Bool(value == Some("true"))
            ),
            None => assert!(!normalized.unwrap().parameters.contains_key("verbose")),
            Some("control") => assert!(normalized.unwrap().parameters.is_empty()),
            _ => assert!(normalized.is_err()),
        }
        let output = f.install();
        let diagnostic = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        if matches!(value, Some("control" | "false" | "true")) {
            assert!(output.status.success(), "{diagnostic}");
            let before = fs::read(f.project.join("vibe.lock")).unwrap();
            let hooks = f.hooks();
            let slot = fs::read(f.slot(DRIVER).join("vibe.toml")).unwrap();
            f.install_ok();
            assert_eq!(fs::read(f.project.join("vibe.lock")).unwrap(), before);
            assert_eq!(f.hooks(), hooks);
            let inspected = f
                .user
                .vibe()
                .args([
                    "show",
                    "configuration-data",
                    "--path",
                    f.project.to_str().unwrap(),
                    "--json",
                ])
                .output()
                .unwrap();
            assert!(
                inspected.status.success(),
                "{}",
                String::from_utf8_lossy(&inspected.stderr)
            );
            let report: serde_json::Value = serde_json::from_slice(&inspected.stdout).unwrap();
            let row = &report["packages"][0];
            assert_eq!(row["values"], serde_json::json!({}));
            if value == Some("control") {
                assert!(row.get("parameters").is_none());
            } else {
                assert_eq!(
                    row["parameters"],
                    serde_json::json!({"verbose":value == Some("true")})
                );
                assert!(
                    f.lock().configurations[&coordinate(DRIVER)]
                        .consumers
                        .values()
                        .all(|input| input.parameters["verbose"]
                            == vibe_core::manifest::ParameterValue::Bool(value == Some("true")))
                );
                if let Some(old) = std::env::var_os("NST_BOOLEAN_OLD_CLI") {
                    for args in [
                        vec![
                            "show",
                            "features",
                            "--path",
                            f.project.to_str().unwrap(),
                            "--json",
                        ],
                        vec![
                            "install",
                            "--path",
                            f.project.to_str().unwrap(),
                            "--offline",
                            "--assume-yes",
                            "--json",
                        ],
                    ] {
                        let result = std::process::Command::new(&old)
                            .env_clear()
                            .env("HOME", f.root.join("home"))
                            .env("VIBE_SETTINGS", &f.user.settings)
                            .env("PATH", "/usr/bin:/bin")
                            .args(args)
                            .output()
                            .unwrap();
                        assert!(!result.status.success());
                        assert_eq!(fs::read(f.project.join("vibe.lock")).unwrap(), before);
                        assert_eq!(fs::read(f.slot(DRIVER).join("vibe.toml")).unwrap(), slot);
                        assert_eq!(f.hooks(), hooks);
                    }
                }
            }
            assert_eq!(fs::read(f.project.join("vibe.lock")).unwrap(), before);
            assert_eq!(fs::read(f.slot(DRIVER).join("vibe.toml")).unwrap(), slot);
            assert_eq!(f.hooks(), hooks);
        } else {
            assert!(!output.status.success());
            if value == Some("'not-a-boolean'") {
                assert!(
                    diagnostic.contains("boolean") || diagnostic.contains("bool"),
                    "{diagnostic}"
                );
            } else {
                assert!(
                    diagnostic.contains("missing required public parameter `verbose`")
                        && diagnostic.contains("vibe.toml"),
                    "{diagnostic}"
                );
            }
            assert!(!f.project.join("vibe.lock").exists());
            assert!(!f.slot(DRIVER).exists());
            assert!(!f.project.join("effects.log").exists());
        }
        assert_eq!(fs::read(&request).unwrap(), authored);
    }
}

#[test]
fn public_boolean_semantic_drift_refuses_update_before_effects() {
    let f = Fixture::new(false);
    let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
    write(
        &source,
        fs::read_to_string(&source).unwrap()
            + "\n[[configuration.parameters]]\nid='verbose'\nname='Verbose'\ntype='bool'\nrequired=true\n",
    );
    let request = f.project.join("vibe.toml");
    let text = fs::read_to_string(&request).unwrap();
    write(
        &request,
        text.replace(
            "members=['disabled']}}}",
            "members=['disabled']}},parameters={verbose=false}}",
        ),
    );
    f.install_ok();
    let before = fs::read(f.project.join("vibe.lock")).unwrap();
    let hooks = f.hooks();
    let boot = f.static_boot();
    let new = f.registry.join(GROUP).join(DRIVER).join("v1.1.0");
    write(
        &new.join("vibe.toml"),
        fs::read_to_string(&source)
            .unwrap()
            .replace("version='1.0.0'", "version='1.1.0'")
            .replace("required=true", "required=false"),
    );
    write(
        &new.join("boot/rules.md"),
        fs::read(source.parent().unwrap().join("boot/rules.md")).unwrap(),
    );
    assert!(!update(&f).status.success());
    assert_eq!(fs::read(f.project.join("vibe.lock")).unwrap(), before);
    assert_eq!(f.hooks(), hooks);
    assert_eq!(f.static_boot(), boot);
}

#[test]
fn public_boolean_does_not_admit_other_parameter_modes_or_unknown_inputs() {
    for (declaration, answer) in [
        ("type='bool'\nrequired=false", "verbose=false"),
        ("type='bool'\nrequired=true\nsecret=true", "verbose=false"),
        ("type='integer'\nrequired=false", "verbose=1"),
        (
            "type='path'\nrequired=true\nmust_exist=true",
            "verbose='/synthetic'",
        ),
        ("type='bool'\nrequired=true", "unknown=false"),
    ] {
        let f = Fixture::new(false);
        let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
        write(
            &source,
            fs::read_to_string(&source).unwrap()
                + &format!(
                    "\n[[configuration.parameters]]\nid='verbose'\nname='Verbose'\n{declaration}\n"
                ),
        );
        let request = f.project.join("vibe.toml");
        write(
            &request,
            fs::read_to_string(&request).unwrap().replace(
                "members=['disabled']}}}",
                &format!("members=['disabled']}}}},parameters={{{answer}}}}}"),
            ),
        );
        let authored = fs::read(&request).unwrap();
        let out = f.install();
        assert!(!out.status.success());
        assert_eq!(fs::read(&request).unwrap(), authored);
        assert!(!f.project.join("vibe.lock").exists());
        assert!(!f.slot(DRIVER).exists());
        assert!(!f.project.join("effects.log").exists());
    }
}

#[test]
fn public_parameter_and_config_value_keep_independent_same_named_values() {
    let f = Fixture::new(false);
    let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
    let text = fs::read_to_string(&source).unwrap().replace("id = \"disabled\"\nname = \"No extra\"", "id = \"disabled\"\nname = \"No extra\"\neffects = [{ kind = \"config-value\", name = \"verbose\" }]");
    write(
        &source,
        text + "\n[configuration.config_values]\nverbose=true\n\n[[configuration.parameters]]\nid='verbose'\nname='Verbose'\ntype='bool'\nrequired=true\n",
    );
    let request = f.project.join("vibe.toml");
    write(
        &request,
        fs::read_to_string(&request).unwrap().replace(
            "members=['disabled']}}}",
            "members=['disabled']}},parameters={verbose=false}}",
        ),
    );
    f.install_ok();
    let before = fs::read(f.project.join("vibe.lock")).unwrap();
    let out = f
        .user
        .vibe()
        .args([
            "show",
            "configuration-data",
            "--path",
            f.project.to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["packages"][0]["values"]["verbose"], true);
    assert_eq!(report["packages"][0]["parameters"]["verbose"], false);
    assert_eq!(fs::read(f.project.join("vibe.lock")).unwrap(), before);
}

#[test]
fn selected_subskill_refusal_is_distinct_from_dormant_captured_bytes_and_boot() {
    const SKILL: &str = "# Synthetic audit\nORACLE_SELECTED_SUBSKILL_BODY\n";
    for selection in [Some(false), Some(true), None] {
        let selected = selection == Some(true);
        let f = Fixture::new(selected);
        let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
        let text = fs::read_to_string(&source).unwrap().replace(
            "effects = [{ kind = \"dependency\", target = \"org.vibevm/oracle-extra\" }]",
            "effects = [{ kind = \"dependency\", target = \"org.vibevm/oracle-extra\" }, { kind = \"subskill\", path = \"skills/audit/SKILL.md\" }]",
        );
        assert!(text.contains("kind = \"subskill\""));
        if selection.is_some() {
            write(&source, text);
        }
        write(
            &source.parent().unwrap().join("skills/audit/SKILL.md"),
            SKILL,
        );
        let authored = fs::read(f.project.join("vibe.toml")).unwrap();
        let launchers: Vec<_> = ["AGENTS.md", "CLAUDE.md", "GEMINI.md"]
            .iter()
            .map(|p| (*p, fs::read(f.project.join(p)).unwrap()))
            .collect();
        let out = f.install();
        if selected {
            assert!(!out.status.success());
            let diagnostic = format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            assert!(
                diagnostic.contains("selected subskill loading is not connected"),
                "{diagnostic}"
            );
            assert!(!f.project.join("vibe.lock").exists());
            assert!(!f.slot(DRIVER).exists());
            assert!(!f.project.join("effects.log").exists());
            for (path, before) in launchers {
                assert_eq!(fs::read(f.project.join(path)).unwrap(), before);
            }
        } else {
            assert!(
                out.status.success(),
                "{}",
                String::from_utf8_lossy(&out.stderr)
            );
            assert_eq!(
                fs::read_to_string(f.slot(DRIVER).join("skills/audit/SKILL.md")).unwrap(),
                SKILL
            );
            let boot = f.static_boot();
            assert!(boot.contains(DRIVER_MARKER));
            assert!(!boot.contains("ORACLE_SELECTED_SUBSKILL_BODY"));
            let lock = fs::read(f.project.join("vibe.lock")).unwrap();
            let hooks = f.hooks();
            let inspect = f
                .user
                .vibe()
                .args([
                    "show",
                    "subskills",
                    "--path",
                    f.project.to_str().unwrap(),
                    "--json",
                ])
                .output()
                .unwrap();
            assert!(inspect.status.success());
            let report: serde_json::Value = serde_json::from_slice(&inspect.stdout).unwrap();
            assert_eq!(report["total"], 0);
            assert_eq!(report["packages"], serde_json::json!([]));
            f.install_ok();
            assert_eq!(fs::read(f.project.join("vibe.lock")).unwrap(), lock);
            assert_eq!(f.static_boot(), boot);
            assert_eq!(f.hooks(), hooks);
            assert_eq!(
                fs::read_to_string(f.slot(DRIVER).join("skills/audit/SKILL.md")).unwrap(),
                SKILL
            );
        }
        assert_eq!(fs::read(f.project.join("vibe.toml")).unwrap(), authored);
    }
}

fn declared_subskill(f: &Fixture, selected: bool, body: &str) {
    let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0");
    let manifest = source.join("vibe.toml");
    write(&manifest, fs::read_to_string(&manifest).unwrap().replace(
        "effects = [{ kind = \"dependency\", target = \"org.vibevm/oracle-extra\" }]",
        "effects = [{ kind = \"dependency\", target = \"org.vibevm/oracle-extra\" }, { kind = \"subskill\", path = \"subskills/audit/SKILL.md\" }]",
    ));
    write(
        &source.join("subskills/audit/vibe-subskill.toml"),
        "[subskill]\npath='audit'\ndelivery='lazy-pull'\ndescription='Synthetic audit guidance'\n[content]\nfiles_written=['SKILL.md']\n",
    );
    write(&source.join("subskills/audit/SKILL.md"), body);
    assert_eq!(
        selected,
        fs::read_to_string(f.project.join("vibe.toml"))
            .unwrap()
            .contains("members=['enabled']")
    );
}
fn read_subskill(f: &Fixture) -> serde_json::Value {
    use vibe_mcp::tools::{McpTool, ReadSubskillMcpTool};
    let context =
        vibe_mcp::ServerContext::with_store_root(&f.project, f.user.settings.join("cache"));
    match ReadSubskillMcpTool.run(
        &serde_json::json!({"package":coordinate(DRIVER),"subskill_path":"audit"}),
        &context,
    ) {
        Ok(result) => result.structured().clone(),
        Err(error) => serde_json::json!({"refused":error.to_string()}),
    }
}
#[test]
fn declared_selected_lazy_pull_has_actual_read_and_cold_replay_but_no_boot_injection() {
    for selected in [false, true] {
        let f = Fixture::new(selected);
        declared_subskill(&f, selected, "ORACLE_DECLARED_LAZY_PULL\n");
        f.install_ok();
        let before = fs::read(f.project.join("vibe.lock")).unwrap();
        let boot = f.static_boot();
        let hooks = f.hooks();
        let lock = f.lock();
        assert_eq!(lock.meta.schema_version, if selected { 13 } else { 8 });
        let row = lock.find(&GROUP.parse().unwrap(), DRIVER).unwrap();
        assert_eq!(row.subskills_active.len(), usize::from(selected));
        let discovery = f
            .user
            .vibe()
            .args([
                "show",
                "subskills",
                "--path",
                f.project.to_str().unwrap(),
                "--json",
            ])
            .output()
            .unwrap();
        assert!(discovery.status.success());
        let discovered: serde_json::Value = serde_json::from_slice(&discovery.stdout).unwrap();
        assert_eq!(discovered["total"], usize::from(selected));
        let read = read_subskill(&f);
        if selected {
            assert!(
                read["content"]
                    .as_str()
                    .unwrap()
                    .contains("ORACLE_DECLARED_LAZY_PULL"),
                "{read}"
            );
            assert!(row.subskills_active[0].source_witness.is_some());
            let transport = f.user.vibe().args(["--offline","mcp","serve","--path",f.project.to_str().unwrap()]).write_stdin(serde_json::json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"read_subskill","arguments":{"package":coordinate(DRIVER),"subskill_path":"audit"}}}).to_string()+"\n").output().unwrap();
            assert!(transport.status.success());
            assert!(
                String::from_utf8_lossy(&transport.stdout).contains("ORACLE_DECLARED_LAZY_PULL")
            );
        } else {
            assert!(read.get("refused").is_some());
        }
        assert!(!boot.contains("ORACLE_DECLARED_LAZY_PULL"));
        f.install_ok();
        assert_eq!(fs::read(f.project.join("vibe.lock")).unwrap(), before);
        assert_eq!(f.static_boot(), boot);
        assert_eq!(f.hooks(), hooks);
    }
}

#[test]
fn declared_lazy_pull_drift_missing_and_materialisation_refuse_without_writes() {
    use vibe_mcp::tools::{MaterialiseSubskillMcpTool, McpTool};
    for mutation in [
        "missing",
        "member",
        "declaration",
        "forged-row",
        "no-witness",
    ] {
        let f = Fixture::new(true);
        declared_subskill(&f, true, "ORIGINAL_SELECTED_MEMBER\n");
        f.install_ok();
        let lock_path = f.project.join("vibe.lock");
        let before = fs::read(&lock_path).unwrap();
        let boot = f.static_boot();
        let hooks = f.hooks();
        let context =
            vibe_mcp::ServerContext::with_store_root(&f.project, f.user.settings.join("cache"));
        for force in [false, true] {
            assert!(MaterialiseSubskillMcpTool.run(&serde_json::json!({"package":coordinate(DRIVER),"subskill_path":"audit","force":force}),&context).is_err());
            assert!(!f.project.join("SKILL.md").exists());
        }
        let cache = f
            .user
            .settings
            .join("cache")
            .join(GROUP)
            .join(DRIVER)
            .join("v1.0.0/subskills/audit");
        match mutation {
            "missing" => {
                fs::rename(cache.join("SKILL.md"), cache.join("preserved-member.md")).unwrap();
            }
            "member" => write(&cache.join("SKILL.md"), "REPLACEMENT_MUST_NOT_LEAK"),
            "declaration" => write(
                &cache.join("vibe-subskill.toml"),
                "[subskill]\npath='other'\n",
            ),
            _ => {
                let mut lock = f.lock();
                let row = lock
                    .packages
                    .iter_mut()
                    .find(|p| p.name.as_str() == DRIVER)
                    .unwrap();
                if mutation == "forged-row" {
                    row.subskills_active.clear();
                } else {
                    row.subskills_active[0].source_witness = None;
                }
                write(&lock_path, toml::to_string(&lock).unwrap());
            }
        }
        let read = read_subskill(&f);
        assert!(read.get("refused").is_some(), "{mutation}: {read}");
        assert!(!read.to_string().contains("REPLACEMENT_MUST_NOT_LEAK"));
        if matches!(mutation, "missing" | "member" | "declaration") {
            assert_eq!(fs::read(&lock_path).unwrap(), before);
        }
        assert_eq!(f.static_boot(), boot);
        assert_eq!(f.hooks(), hooks);
    }
}

#[test]
fn declared_lazy_pull_invalid_declaration_refuses_before_project_effects() {
    for declaration in [
        "[subskill]\npath='other'\ndelivery='lazy-pull'\ndescription='x'\n[content]\nfiles_written=['SKILL.md']",
        "[subskill]\npath='audit'\ndelivery='eager'\n[content]\nfiles_written=['SKILL.md']",
        "[subskill]\npath='audit'\ndelivery='lazy-pull'\ndescription='x'\n[content]\nfiles_written=['SKILL.md','other.md']",
        "[subskill]\npath='audit'\ndelivery='lazy-pull'\ndescription='x'\n[content]\nfiles_written=['../escape']",
    ] {
        let f = Fixture::new(true);
        declared_subskill(&f, true, "INERT\n");
        write(
            &f.registry
                .join(GROUP)
                .join(DRIVER)
                .join("v1.0.0/subskills/audit/vibe-subskill.toml"),
            declaration,
        );
        let authored = fs::read(f.project.join("vibe.toml")).unwrap();
        assert!(!f.install().status.success());
        assert!(!f.project.join("vibe.lock").exists());
        assert!(!f.slot(DRIVER).exists());
        assert!(!f.project.join("effects.log").exists());
        assert_eq!(fs::read(f.project.join("vibe.toml")).unwrap(), authored);
    }
}

#[test]
fn declared_lazy_pull_prior_binary_refuses_reader_writer_and_registered_mcp() {
    let Some(old) = std::env::var_os("NST_SUBSKILL_OLD_CLI") else {
        return;
    };
    let f = Fixture::new(true);
    declared_subskill(&f, true, "PINNED_MEMBER_NEVER_FOR_OLD_READER\n");
    f.install_ok();
    let before = fs::read(f.project.join("vibe.lock")).unwrap();
    let boot = f.static_boot();
    let hooks = f.hooks();
    let script=serde_json::json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"read_subskill","arguments":{"package":coordinate(DRIVER),"subskill_path":"audit"}}}).to_string()+"\n";
    for args in [
        vec![
            "show",
            "subskills",
            "--path",
            f.project.to_str().unwrap(),
            "--json",
        ],
        vec![
            "install",
            "--offline",
            "--assume-yes",
            "--json",
            "--path",
            f.project.to_str().unwrap(),
        ],
        vec!["mcp", "serve", "--path", f.project.to_str().unwrap()],
    ] {
        use std::io::Write;
        let mcp = args[0] == "mcp";
        let mut child = std::process::Command::new(&old)
            .env_clear()
            .env("HOME", f.root.join("home"))
            .env("VIBE_SETTINGS", &f.user.settings)
            .env("PATH", "/usr/bin:/bin")
            .args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        if mcp {
            child
                .stdin
                .as_mut()
                .unwrap()
                .write_all(script.as_bytes())
                .unwrap();
        }
        drop(child.stdin.take());
        let out = child.wait_with_output().unwrap();
        let diagnostic = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(!diagnostic.contains("PINNED_MEMBER_NEVER_FOR_OLD_READER"));
        if mcp {
            assert!(
                diagnostic.contains("schema") || diagnostic.contains("unknown field"),
                "{diagnostic}"
            );
        } else {
            assert!(!out.status.success(), "{diagnostic}");
        }
        assert_eq!(fs::read(f.project.join("vibe.lock")).unwrap(), before);
        assert_eq!(f.static_boot(), boot);
        assert_eq!(f.hooks(), hooks);
    }
}

#[test]
fn declared_lazy_pull_two_projects_cannot_read_replaced_same_version_cache() {
    let a = Fixture::new(true);
    declared_subskill(&a, true, "PROJECT_A_PINNED\n");
    a.install_ok();
    assert!(
        read_subskill(&a)["content"]
            .as_str()
            .unwrap()
            .contains("PROJECT_A_PINNED")
    );
    let a_lock = fs::read(a.project.join("vibe.lock")).unwrap();
    let a_boot = a.static_boot();
    let a_hooks = a.hooks();
    let b = Fixture::new(true);
    declared_subskill(&b, true, "PROJECT_B_REPLACEMENT\n");
    // Share only the isolated settings/cache, not project source or lock authority.
    let out = a
        .user
        .vibe()
        .current_dir(&b.project)
        .env("HOME", b.root.join("home"))
        .args([
            "install",
            "--offline",
            "--assume-yes",
            "--json",
            "--path",
            b.project.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let a_read = read_subskill(&a);
    assert!(a_read.get("refused").is_some(), "{a_read}");
    assert!(!a_read.to_string().contains("PROJECT_B_REPLACEMENT"));
    use vibe_mcp::tools::{McpTool, ReadSubskillMcpTool};
    let context =
        vibe_mcp::ServerContext::with_store_root(&b.project, a.user.settings.join("cache"));
    let b_read = ReadSubskillMcpTool
        .run(
            &serde_json::json!({"package":coordinate(DRIVER),"subskill_path":"audit"}),
            &context,
        )
        .unwrap();
    assert!(
        b_read["content"]
            .as_str()
            .unwrap()
            .contains("PROJECT_B_REPLACEMENT")
    );
    assert_eq!(fs::read(a.project.join("vibe.lock")).unwrap(), a_lock);
    assert_eq!(a.static_boot(), a_boot);
    assert_eq!(a.hooks(), a_hooks);
}

#[test]
fn declared_lazy_pull_selected_file_limits_are_inclusive_and_not_package_limits() {
    for (member_len, declaration_len, accepted) in [
        (1024 * 1024, 64 * 1024, true),
        (1024 * 1024 + 1, 128, false),
        (64, 64 * 1024 + 1, false),
    ] {
        let f = Fixture::new(true);
        declared_subskill(&f, true, &"X".repeat(member_len));
        let root = f.registry.join(GROUP).join(DRIVER).join("v1.0.0");
        let path = root.join("subskills/audit/vibe-subskill.toml");
        let mut text = fs::read_to_string(&path).unwrap();
        // Pad a valid declaration with a comment, never truncate its semantic bytes.
        let target = declaration_len.max(text.len() + 2);
        text.push_str("\n#");
        text.extend(std::iter::repeat_n('x', target - text.len()));
        write(&path, text);
        write(
            &root.join("unrelated-large.bin"),
            vec![0_u8; 2 * 1024 * 1024],
        );
        let out = f.install();
        assert_eq!(
            out.status.success(),
            accepted,
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        if accepted {
            assert!(read_subskill(&f)["content"].as_str().unwrap().len() >= member_len);
        } else {
            assert!(!f.project.join("vibe.lock").exists());
            assert!(!f.slot(DRIVER).exists());
            assert!(!f.project.join("effects.log").exists());
        }
    }
}

#[test]
fn declared_lazy_pull_missing_historical_witness_cannot_be_rebuilt_from_cache() {
    let f = Fixture::new(true);
    declared_subskill(&f, true, "ORIGINAL_AUTHORITY\n");
    f.install_ok();
    let accepted_lock = fs::read(f.project.join("vibe.lock")).unwrap();
    let accepted_boot = f.static_boot();
    let accepted_hooks = f.hooks();
    assert!(!update(&f).status.success());
    assert_eq!(
        fs::read(f.project.join("vibe.lock")).unwrap(),
        accepted_lock
    );
    assert_eq!(f.static_boot(), accepted_boot);
    assert_eq!(f.hooks(), accepted_hooks);
    let mut lock = f.lock();
    lock.packages
        .iter_mut()
        .find(|p| p.name.as_str() == DRIVER)
        .unwrap()
        .subskills_active[0]
        .source_witness = None;
    let path = f.project.join("vibe.lock");
    write(&path, toml::to_string(&lock).unwrap());
    let before = fs::read(&path).unwrap();
    let boot = f.static_boot();
    let hooks = f.hooks();
    assert!(!f.install().status.success());
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(f.static_boot(), boot);
    assert_eq!(f.hooks(), hooks);
}

#[test]
fn selected_package_forwarding_refuses_while_dormant_and_direct_target_controls_work() {
    for world in ["selected", "dormant", "direct"] {
        let f = Fixture::new(world == "selected");
        let driver = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
        write(&driver,fs::read_to_string(&driver).unwrap().replace(
            "effects = [{ kind = \"dependency\", target = \"org.vibevm/oracle-extra\" }]",
            "effects = [{ kind = \"dependency\", target = \"org.vibevm/oracle-extra\" }, { kind = \"forward\", target = \"org.vibevm/oracle-extra\", feature = \"forwarded\", strength = \"strong\" }]",
        ));
        let target = f.registry.join(GROUP).join(EXTRA).join("v1.0.0/vibe.toml");
        write(
            &target,
            fs::read_to_string(&target).unwrap()
                + "\n[configuration.config_values]\nforwarded=true\n[[configuration.features]]\nid='forwarded'\nname='Forwarded target feature'\neffects=[{kind='config-value',name='forwarded'}]\n",
        );
        let request = f.project.join("vibe.toml");
        if world == "direct" {
            let authored = format!(
                "[project]\nname='direct-target-control'\nversion='0.1.0'\n[requires.packages.'{GROUP}/{EXTRA}']\nversion='^1'\nlink='static-transitive'\nconfiguration={{policy='require-selection',default_features=false,features=['forwarded']}}\n[[registry]]\nname='fixture'\nurl='file://{}'\n",
                f.registry.display()
            );
            write(&request, authored);
        }
        let authored = fs::read(&request).unwrap();
        let launchers: Vec<_> = ["AGENTS.md", "CLAUDE.md", "GEMINI.md"]
            .into_iter()
            .map(|p| (p, fs::read(f.project.join(p)).unwrap()))
            .collect();
        let out = f.install();
        if world == "selected" {
            let diagnostic = format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            assert!(!out.status.success());
            assert!(diagnostic.contains("selected cross-package forwarding requires candidate-conditioned incoming consumers"),"{diagnostic}");
            assert!(!f.project.join("vibe.lock").exists());
            assert!(!f.slot(DRIVER).exists());
            assert!(!f.slot(EXTRA).exists());
            assert!(!f.project.join("effects.log").exists());
            for (p, before) in launchers {
                assert_eq!(fs::read(f.project.join(p)).unwrap(), before);
            }
        } else {
            assert!(
                out.status.success(),
                "{}",
                String::from_utf8_lossy(&out.stderr)
            );
            let lock = f.lock();
            let coordinate = coordinate(if world == "direct" { EXTRA } else { DRIVER });
            let record = &lock.configurations[&coordinate];
            assert_eq!(record.consumers.len(), 1);
            assert!(
                record
                    .consumers
                    .keys()
                    .all(|key| key.ends_with("vibe.toml"))
            );
            if world == "direct" {
                assert_eq!(record.features, vec!["forwarded"]);
                assert_eq!(
                    record.selected_data(vibe_core::PackageKind::Flow).unwrap()["forwarded"],
                    vibe_core::manifest::ParameterValue::Bool(true)
                );
                assert!(f.static_boot().contains(EXTRA_MARKER));
                assert!(!f.slot(DRIVER).exists());
            } else {
                assert!(!f.slot(EXTRA).exists());
                assert!(!f.static_boot().contains(EXTRA_MARKER));
            }
            let before = fs::read(f.project.join("vibe.lock")).unwrap();
            let boot = f.static_boot();
            let hooks = f.hooks();
            f.install_ok();
            assert_eq!(fs::read(f.project.join("vibe.lock")).unwrap(), before);
            assert_eq!(f.static_boot(), boot);
            assert_eq!(f.hooks(), hooks);
        }
        assert_eq!(fs::read(&request).unwrap(), authored);
    }
}

fn exact_forward_fixture() -> Fixture {
    let f = Fixture::new(true);
    let driver = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
    write(&driver,fs::read_to_string(&driver).unwrap().replace("version = \"^1\", optional = true","version = \"=1.0.0\", optional = true").replace(
        "effects = [{ kind = \"dependency\", target = \"org.vibevm/oracle-extra\" }]",
        "effects = [{ kind = \"forward\", target = \"org.vibevm/oracle-extra\", feature = \"forwarded\", strength = \"strong\" }]",
    ));
    let target = f.registry.join(GROUP).join(EXTRA).join("v1.0.0/vibe.toml");
    write(
        &target,
        fs::read_to_string(&target).unwrap()
            + "\n[configuration.config_values]\nforwarded=true\n[[configuration.features]]\nid='forwarded'\nname='Forwarded target feature'\neffects=[{kind='config-value',name='forwarded'}]\n",
    );
    let request = f.project.join("vibe.toml");
    write(
        &request,
        fs::read_to_string(&request)
            .unwrap()
            .replace("version='^1'", "version='=1.0.0'"),
    );
    f
}

#[test]
fn compulsory_exact_forwarding_retains_package_owned_input_and_cold_replays() {
    let f = exact_forward_fixture();
    let request = f.project.join("vibe.toml");
    let original = fs::read(&request).unwrap();
    f.install_ok();
    let lock = f.lock();
    assert_eq!(lock.meta.schema_version, 14);
    let parent = &lock.configurations[&coordinate(DRIVER)];
    let child = &lock.configurations[&coordinate(EXTRA)];
    assert_eq!(parent.consumers.len(), 1);
    assert!(parent.forwarded_inputs.is_empty());
    assert!(child.consumers.is_empty());
    assert_eq!(child.forwarded_inputs.len(), 1);
    assert_eq!(child.forwarded_inputs[0].source_name.as_str(), DRIVER);
    assert_eq!(
        child.selected_data(vibe_core::PackageKind::Flow).unwrap()["forwarded"],
        vibe_core::manifest::ParameterValue::Bool(true)
    );
    assert!(f.static_boot().contains(EXTRA_MARKER));
    let before = fs::read(f.project.join("vibe.lock")).unwrap();
    let boot = f.static_boot();
    let hooks = f.hooks();
    f.install_ok();
    assert_eq!(fs::read(f.project.join("vibe.lock")).unwrap(), before);
    assert_eq!(f.static_boot(), boot);
    assert_eq!(f.hooks(), hooks);
    assert_eq!(fs::read(&request).unwrap(), original);
}

#[test]
fn compulsory_forwarding_tampered_provenance_and_update_refuse_unchanged() {
    for mutation in [
        "removed",
        "wrong-index",
        "wrong-parent",
        "wrong-pin",
        "duplicate",
        "old-epoch",
    ] {
        let f = exact_forward_fixture();
        f.install_ok();
        let path = f.project.join("vibe.lock");
        let boot = f.static_boot();
        let hooks = f.hooks();
        assert!(!update(&f).status.success());
        let mut lock = f.lock();
        let child = lock.configurations.get_mut(&coordinate(EXTRA)).unwrap();
        match mutation {
            "removed" => child.forwarded_inputs.clear(),
            "wrong-index" => child.forwarded_inputs[0].declaration_index = 999,
            "wrong-parent" => child.forwarded_inputs[0].source_name = "other".parse().unwrap(),
            "wrong-pin" => {
                child.forwarded_inputs[0].source_content_hash =
                    vibe_core::ContentHash::parse("sha256:aa").unwrap()
            }
            "duplicate" => child
                .forwarded_inputs
                .push(child.forwarded_inputs[0].clone()),
            "old-epoch" => lock.meta.schema_version = 13,
            _ => unreachable!(),
        }
        write(&path, toml::to_string(&lock).unwrap());
        let before = fs::read(&path).unwrap();
        assert!(Lockfile::read(&path).is_err(), "{mutation}");
        assert!(!f.install().status.success(), "{mutation}");
        assert_eq!(fs::read(&path).unwrap(), before);
        assert_eq!(f.static_boot(), boot);
        assert_eq!(f.hooks(), hooks);
    }
}

#[test]
fn compulsory_forwarding_unsupported_ranges_and_recursive_refuse_before_effects() {
    for mutation in [
        "parent-range",
        "target-range",
        "recursive",
        "unknown",
        "wrong-kind",
    ] {
        let f = exact_forward_fixture();
        let driver = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
        let target = f.registry.join(GROUP).join(EXTRA).join("v1.0.0/vibe.toml");
        match mutation {
            "parent-range" => {
                let p = f.project.join("vibe.toml");
                write(
                    &p,
                    fs::read_to_string(&p)
                        .unwrap()
                        .replace("version='=1.0.0'", "version='^1'"),
                );
            }
            "target-range" => write(
                &driver,
                fs::read_to_string(&driver)
                    .unwrap()
                    .replace("version = \"=1.0.0\"", "version = \"^1\""),
            ),
            "unknown" => write(
                &driver,
                fs::read_to_string(&driver)
                    .unwrap()
                    .replace("feature = \"forwarded\"", "feature = \"unknown\""),
            ),
            "wrong-kind" => write(
                &driver,
                fs::read_to_string(&driver).unwrap().replace(
                    "\"org.vibevm/oracle-extra\" =",
                    "\"app:org.vibevm/oracle-extra\" =",
                ),
            ),
            "recursive" => write(
                &target,
                fs::read_to_string(&target).unwrap()
                    + "\n[requires.packages]\n'org.vibevm/oracle-base'='=1.0.0'\n[[configuration.features]]\nid='recursive'\nname='Recursive'\neffects=[{kind='forward',target='org.vibevm/oracle-base',feature='other',strength='strong'}]\n",
            ),
            _ => unreachable!(),
        }
        let authored = fs::read(f.project.join("vibe.toml")).unwrap();
        let out = f.install();
        assert!(
            !out.status.success(),
            "{mutation}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(!f.project.join("vibe.lock").exists());
        assert!(!f.slot(DRIVER).exists());
        assert!(!f.slot(EXTRA).exists());
        assert!(!f.project.join("effects.log").exists());
        assert_eq!(fs::read(f.project.join("vibe.toml")).unwrap(), authored);
    }
}

#[test]
fn compulsory_forwarding_prior_reader_and_install_writer_refuse_without_mutation() {
    let Some(old) = std::env::var_os("NST_FORWARD_OLD_CLI") else {
        return;
    };
    let f = exact_forward_fixture();
    f.install_ok();
    let before = fs::read(f.project.join("vibe.lock")).unwrap();
    let boot = f.static_boot();
    let hooks = f.hooks();
    for args in [
        vec![
            "show",
            "features",
            "--path",
            f.project.to_str().unwrap(),
            "--json",
        ],
        vec![
            "install",
            "--path",
            f.project.to_str().unwrap(),
            "--offline",
            "--assume-yes",
            "--json",
        ],
    ] {
        let out = std::process::Command::new(&old)
            .env_clear()
            .env("HOME", f.root.join("home"))
            .env("VIBE_SETTINGS", &f.user.settings)
            .env("PATH", "/usr/bin:/bin")
            .args(args)
            .output()
            .unwrap();
        assert!(!out.status.success());
        assert_eq!(fs::read(f.project.join("vibe.lock")).unwrap(), before);
        assert_eq!(f.static_boot(), boot);
        assert_eq!(f.hooks(), hooks);
    }
}

#[test]
fn compulsory_forwarding_merges_direct_consumer_and_rejects_conflict_before_effects() {
    for compatible in [false, true] {
        let f = exact_forward_fixture();
        let target = f.registry.join(GROUP).join(EXTRA).join("v1.0.0/vibe.toml");
        write(
            &target,
            fs::read_to_string(&target).unwrap()
                + "\n[[configuration.features]]\nid='other'\nname='Other target feature'\n[[configuration.groups]]\nmode='choice'\nid='mode'\nname='Mode'\ncardinality='exactly-one'\nmembers=['forwarded','other']\n",
        );
        let request = f.project.join("vibe.toml");
        write(
            &request,
            fs::read_to_string(&request).unwrap()
                + &format!(
                    "\n[requires.packages.'{GROUP}/{EXTRA}']\nversion='=1.0.0'\nlink='static-transitive'\nconfiguration={{policy='require-selection',default_features=false,selections={{mode={{intent='exact',members=['{}']}}}}}}\n",
                    if compatible { "forwarded" } else { "other" }
                ),
        );
        let authored = fs::read(&request).unwrap();
        let out = f.install();
        if compatible {
            assert!(
                out.status.success(),
                "{}",
                String::from_utf8_lossy(&out.stderr)
            );
            let mut lock = f.lock();
            let record = &lock.configurations[&coordinate(EXTRA)];
            assert_eq!(record.consumers.len(), 1);
            assert_eq!(record.forwarded_inputs.len(), 1);
            let before = fs::read(f.project.join("vibe.lock")).unwrap();
            let hooks = f.hooks();
            f.install_ok();
            assert_eq!(fs::read(f.project.join("vibe.lock")).unwrap(), before);
            assert_eq!(f.hooks(), hooks);
            // Removing the row must refuse even though direct intent selects the same feature.
            lock.configurations
                .get_mut(&coordinate(EXTRA))
                .unwrap()
                .forwarded_inputs
                .clear();
            let path = f.project.join("vibe.lock");
            write(&path, toml::to_string(&lock).unwrap());
            assert!(Lockfile::read(&path).is_err());
            assert!(!f.install().status.success());
        } else {
            assert!(!out.status.success());
            assert!(!f.project.join("vibe.lock").exists());
            assert!(!f.slot(DRIVER).exists());
            assert!(!f.slot(EXTRA).exists());
            assert!(!f.project.join("effects.log").exists());
        }
        assert_eq!(fs::read(&request).unwrap(), authored);
    }
}

#[test]
fn unselected_source_candidate_does_not_forward_into_target() {
    let f = Fixture::new(false);
    let old = f.registry.join(GROUP).join(DRIVER).join("v1.0.0");
    let alternate = f.registry.join(GROUP).join(DRIVER).join("v1.1.0");
    write(&alternate.join("vibe.toml"),fs::read_to_string(old.join("vibe.toml")).unwrap().replace("version='1.0.0'","version='1.1.0'").replace("name = \"No extra\"","name = \"No extra\"\neffects=[{kind='forward',target='org.vibevm/oracle-extra',feature='unknown',strength='strong'}]"));
    write(&alternate.join("boot/rules.md"), "UNSELECTED_PARENT_BODY\n");
    let request = f.project.join("vibe.toml");
    write(
        &request,
        fs::read_to_string(&request)
            .unwrap()
            .replace("version='^1'", "version='=1.0.0'"),
    );
    f.install_ok();
    assert!(!f.slot(EXTRA).exists());
    assert!(!f.static_boot().contains("UNSELECTED_PARENT_BODY"));
    assert_eq!(f.lock().configurations.len(), 1);
    assert!(
        f.lock()
            .configurations
            .values()
            .all(|record| record.forwarded_inputs.is_empty())
    );
}

#[test]
fn boolean_default_omitted_true_false_and_no_default_controls_have_distinct_public_boundaries() {
    for (has_default, answer) in [
        (true, None),
        (true, Some(true)),
        (true, Some(false)),
        (false, None),
        (false, Some(false)),
    ] {
        let f = Fixture::new(false);
        let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
        write(
            &source,
            fs::read_to_string(&source).unwrap()
                + "\n[[configuration.parameters]]\nid='verbose'\nname='Verbose'\ntype='bool'\nrequired=true\n"
                + if has_default { "default=true\n" } else { "" },
        );
        let request = f.project.join("vibe.toml");
        if let Some(answer) = answer {
            let original = fs::read_to_string(&request).unwrap();
            write(
                &request,
                original.replace(
                    "members=['disabled']}}}",
                    &format!("members=['disabled']}}}},parameters={{verbose={answer}}}}}"),
                ),
            );
        }
        let authored = fs::read(&request).unwrap();
        let static_before: Vec<_> = [common::static_xml_rel(), common::static_md_rel()]
            .into_iter()
            .map(|relative| {
                let path = f.project.join(relative);
                let before = if path.exists() {
                    Some(fs::read(&path).unwrap())
                } else {
                    None
                };
                (path, before)
            })
            .collect();
        let launchers: Vec<_> = ["AGENTS.md", "CLAUDE.md", "GEMINI.md"]
            .into_iter()
            .map(|p| (p, fs::read(f.project.join(p)).unwrap()))
            .collect();
        // Parse authored syntax first; no internal constructor can bypass a refusal.
        let package = Manifest::read(&source).unwrap();
        let consumer = Manifest::read(&request).unwrap();
        let definition = package.configuration.unwrap();
        let input = consumer.requires.configurations.values().next().unwrap();
        assert_eq!(
            input.parameters.get("verbose"),
            answer.as_ref().map(|value| if *value {
                &vibe_core::manifest::ParameterValue::Bool(true)
            } else {
                &vibe_core::manifest::ParameterValue::Bool(false)
            })
        );
        let normalized = definition.normalize_input(input, "./vibe.toml").unwrap();
        assert_eq!(
            normalized.parameters.get("verbose"),
            input.parameters.get("verbose"),
            "normalization does not silently fill an omitted parameter default"
        );
        let out = f.install();
        let diagnostic = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        if !has_default && answer.is_none() {
            assert!(!out.status.success(), "{diagnostic}");
            assert!(
                diagnostic.contains("missing required public parameter `verbose`"),
                "{diagnostic}"
            );
            assert!(!f.project.join("vibe.lock").exists());
            assert!(!f.slot(DRIVER).exists());
            assert!(!f.slot(BASE).exists());
            assert!(!f.slot(EXTRA).exists());
            assert!(!f.project.join("effects.log").exists());
            for (p, before) in launchers {
                assert_eq!(fs::read(f.project.join(p)).unwrap(), before);
            }
            for (path, before) in static_before {
                let after = if path.exists() {
                    Some(fs::read(&path).unwrap())
                } else {
                    None
                };
                assert_eq!(after, before, "generated boot changed on refusal");
            }
        } else {
            assert!(out.status.success(), "{diagnostic}");
            let before = fs::read(f.project.join("vibe.lock")).unwrap();
            let boot = f.static_boot();
            let hooks = f.hooks();
            let lock = f.lock();
            assert!(
                lock.configurations[&coordinate(DRIVER)]
                    .consumers
                    .values()
                    .all(|original| original.parameters.get("verbose")
                        == input.parameters.get("verbose"))
            );
            let show = f
                .user
                .vibe()
                .args([
                    "show",
                    "configuration-data",
                    "--path",
                    f.project.to_str().unwrap(),
                    "--json",
                ])
                .output()
                .unwrap();
            assert!(show.status.success());
            let report: serde_json::Value = serde_json::from_slice(&show.stdout).unwrap();
            assert_eq!(
                report["packages"][0]["parameters"]["verbose"],
                answer.unwrap_or(true)
            );
            f.install_ok();
            assert_eq!(fs::read(f.project.join("vibe.lock")).unwrap(), before);
            assert_eq!(f.static_boot(), boot);
            assert_eq!(f.hooks(), hooks);
        }
        assert_eq!(fs::read(&request).unwrap(), authored);
    }
}

#[test]
fn boolean_default_false_and_explicit_true_are_distinct_and_old_clients_refuse() {
    for answer in [None, Some(false), Some(true)] {
        let f = Fixture::new(false);
        let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
        write(
            &source,
            fs::read_to_string(&source).unwrap()
                + "\n[[configuration.parameters]]\nid='verbose'\nname='Verbose'\ntype='bool'\nrequired=true\ndefault=false\n",
        );
        let request = f.project.join("vibe.toml");
        if let Some(answer) = answer {
            write(
                &request,
                fs::read_to_string(&request).unwrap().replace(
                    "members=['disabled']}}}",
                    &format!("members=['disabled']}}}},parameters={{verbose={answer}}}}}"),
                ),
            );
        }
        f.install_ok();
        let before = fs::read(f.project.join("vibe.lock")).unwrap();
        let boot = f.static_boot();
        let hooks = f.hooks();
        let slot = fs::read(f.slot(DRIVER).join("vibe.toml")).unwrap();
        let lock = f.lock();
        let record = &lock.configurations[&coordinate(DRIVER)];
        assert_eq!(
            record
                .selected_parameters(vibe_core::PackageKind::Flow)
                .unwrap()["verbose"],
            vibe_core::manifest::ParameterValue::Bool(answer.unwrap_or(false))
        );
        assert_eq!(
            record
                .consumers
                .values()
                .next()
                .unwrap()
                .parameters
                .contains_key("verbose"),
            answer.is_some()
        );
        if let Some(old) = std::env::var_os("NST_BOOL_DEFAULT_OLD_CLI") {
            for args in [
                vec![
                    "show",
                    "features",
                    "--path",
                    f.project.to_str().unwrap(),
                    "--json",
                ],
                vec![
                    "install",
                    "--path",
                    f.project.to_str().unwrap(),
                    "--offline",
                    "--assume-yes",
                    "--json",
                ],
            ] {
                let out = std::process::Command::new(&old)
                    .env_clear()
                    .env("HOME", f.root.join("home"))
                    .env("VIBE_SETTINGS", &f.user.settings)
                    .env("PATH", "/usr/bin:/bin")
                    .args(args)
                    .output()
                    .unwrap();
                assert!(!out.status.success());
                assert_eq!(fs::read(f.project.join("vibe.lock")).unwrap(), before);
                assert_eq!(fs::read(f.slot(DRIVER).join("vibe.toml")).unwrap(), slot);
                assert_eq!(f.static_boot(), boot);
                assert_eq!(f.hooks(), hooks);
            }
        }
        f.install_ok();
        assert_eq!(fs::read(f.project.join("vibe.lock")).unwrap(), before);
        assert_eq!(f.static_boot(), boot);
        assert_eq!(f.hooks(), hooks);
    }
}

#[test]
fn boolean_default_does_not_grant_parameter_forwarding() {
    let f = exact_forward_fixture();
    let target = f.registry.join(GROUP).join(EXTRA).join("v1.0.0/vibe.toml");
    write(
        &target,
        fs::read_to_string(&target).unwrap()
            + "\n[[configuration.parameters]]\nid='verbose'\nname='Verbose'\ntype='bool'\nrequired=true\ndefault=true\n",
    );
    let request = fs::read(f.project.join("vibe.toml")).unwrap();
    assert!(!f.install().status.success());
    assert!(!f.project.join("vibe.lock").exists());
    assert!(!f.slot(DRIVER).exists());
    assert!(!f.slot(EXTRA).exists());
    assert!(!f.project.join("effects.log").exists());
    assert_eq!(fs::read(f.project.join("vibe.toml")).unwrap(), request);
    for relative in [common::static_md_rel(), common::static_xml_rel()] {
        assert!(!f.project.join(relative).exists());
    }
}

#[test]
fn boolean_default_change_refuses_update_without_turning_explicit_answer_into_reliance() {
    for explicit in [false, true] {
        let f = Fixture::new(false);
        let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
        write(
            &source,
            fs::read_to_string(&source).unwrap()
                + "\n[[configuration.parameters]]\nid='verbose'\nname='Verbose'\ntype='bool'\nrequired=true\ndefault=true\n",
        );
        if explicit {
            let request = f.project.join("vibe.toml");
            write(
                &request,
                fs::read_to_string(&request).unwrap().replace(
                    "members=['disabled']}}}",
                    "members=['disabled']}},parameters={verbose=true}}",
                ),
            );
        }
        f.install_ok();
        let before = fs::read(f.project.join("vibe.lock")).unwrap();
        let boot = f.static_boot();
        let hooks = f.hooks();
        let slot = fs::read(f.slot(DRIVER).join("vibe.toml")).unwrap();
        let new = f.registry.join(GROUP).join(DRIVER).join("v1.1.0");
        write(
            &new.join("vibe.toml"),
            fs::read_to_string(&source)
                .unwrap()
                .replace("version='1.0.0'", "version='1.1.0'")
                .replace("default=true", "default=false"),
        );
        write(
            &new.join("boot/rules.md"),
            fs::read(source.parent().unwrap().join("boot/rules.md")).unwrap(),
        );
        let out = update(&f);
        assert!(!out.status.success());
        assert_eq!(fs::read(f.project.join("vibe.lock")).unwrap(), before);
        assert_eq!(fs::read(f.slot(DRIVER).join("vibe.toml")).unwrap(), slot);
        assert_eq!(f.static_boot(), boot);
        assert_eq!(f.hooks(), hooks);
        let record = &f.lock().configurations[&coordinate(DRIVER)];
        assert_eq!(
            record
                .consumers
                .values()
                .next()
                .unwrap()
                .parameters
                .contains_key("verbose"),
            explicit
        );
    }
}

#[test]
fn named_profile_and_explicit_boolean_composition_has_four_distinct_public_worlds() {
    for world in [
        "profile-false",
        "profile-true",
        "profile-missing",
        "profile-only",
        "boolean-only",
    ] {
        let f = Fixture::new(true);
        let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
        let mut text = fs::read_to_string(&source).unwrap()
            + "\n[[configuration.profiles]]\nid='with-extra'\nname='With extra'\nselections.extra={intent='exact',members=['enabled']}\n";
        if world != "profile-only" {
            text.push_str("\n[[configuration.parameters]]\nid='verbose'\nname='Verbose'\ntype='bool'\nrequired=true\n");
        }
        write(&source, text);
        let request = f.project.join("vibe.toml");
        let old = fs::read_to_string(&request).unwrap();
        let configuration = match world {
            "profile-false" => {
                "configuration={policy='require-selection',default_features=false,profile='with-extra',parameters={verbose=false}}"
            }
            "profile-true" => {
                "configuration={policy='require-selection',default_features=false,profile='with-extra',parameters={verbose=true}}"
            }
            "profile-missing" | "profile-only" => {
                "configuration={policy='require-selection',default_features=false,profile='with-extra'}"
            }
            _ => {
                "configuration={policy='require-selection',default_features=false,selections={extra={intent='exact',members=['enabled']}},parameters={verbose=false}}"
            }
        };
        write(&request,old.replace("configuration={policy='require-selection',default_features=false,selections={extra={intent='exact',members=['enabled']}}}",configuration));
        let authored = fs::read(&request).unwrap();
        let boot_before: Vec<_> = [common::static_md_rel(), common::static_xml_rel()]
            .into_iter()
            .map(|p| {
                let path = f.project.join(p);
                let bytes = if path.exists() {
                    Some(fs::read(&path).unwrap())
                } else {
                    None
                };
                (path, bytes)
            })
            .collect();
        let package = Manifest::read(&source).unwrap();
        let consumer = Manifest::read(&request).unwrap();
        let input = consumer.requires.configurations.values().next().unwrap();
        let normalized = package
            .configuration
            .as_ref()
            .unwrap()
            .normalize_input(input, "./vibe.toml")
            .unwrap();
        assert_eq!(
            normalized.parameters.contains_key("verbose"),
            matches!(world, "profile-false" | "profile-true" | "boolean-only")
        );
        let out = f.install();
        let diagnostic = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        if world == "profile-missing" {
            assert!(!out.status.success());
            assert!(
                diagnostic.contains("missing required public parameter `verbose`"),
                "{diagnostic}"
            );
            assert!(!f.project.join("vibe.lock").exists());
            assert!(!f.slot(DRIVER).exists());
            assert!(!f.slot(BASE).exists());
            assert!(!f.slot(EXTRA).exists());
            assert!(!f.project.join("effects.log").exists());
            for (path, before) in boot_before {
                assert_eq!(
                    if path.exists() {
                        Some(fs::read(path).unwrap())
                    } else {
                        None
                    },
                    before
                );
            }
        } else {
            assert!(out.status.success(), "{diagnostic}");
            let lock = f.lock();
            let record = &lock.configurations[&coordinate(DRIVER)];
            assert!(record.features.contains(&"enabled".into()));
            assert!(f.slot(EXTRA).exists());
            assert!(f.static_boot().contains(EXTRA_MARKER));
            assert_eq!(record.consumers.values().next().unwrap(), input);
            assert_eq!(
                record.consumers.values().next().unwrap().profile.as_deref(),
                if world != "boolean-only" {
                    Some("with-extra")
                } else {
                    None
                }
            );
            let show = f
                .user
                .vibe()
                .args([
                    "show",
                    "configuration-data",
                    "--path",
                    f.project.to_str().unwrap(),
                    "--json",
                ])
                .output()
                .unwrap();
            assert!(show.status.success());
            let report: serde_json::Value = serde_json::from_slice(&show.stdout).unwrap();
            if world != "profile-only" {
                assert_eq!(
                    report["packages"][0]["parameters"]["verbose"],
                    world == "profile-true"
                );
            } else {
                assert!(report["packages"][0].get("parameters").is_none());
            }
            let before = fs::read(f.project.join("vibe.lock")).unwrap();
            let boot = f.static_boot();
            let hooks = f.hooks();
            f.install_ok();
            assert_eq!(fs::read(f.project.join("vibe.lock")).unwrap(), before);
            assert_eq!(f.static_boot(), boot);
            assert_eq!(f.hooks(), hooks);
        }
        assert_eq!(fs::read(&request).unwrap(), authored);
    }
}

fn profile_boolean_fixture() -> Fixture {
    let f = Fixture::new(true);
    let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
    write(
        &source,
        fs::read_to_string(&source).unwrap()
            + "\n[[configuration.parameters]]\nid='verbose'\nname='Verbose'\ntype='bool'\nrequired=true\n[[configuration.profiles]]\nid='with-extra'\nname='With extra'\nselections.extra={intent='exact',members=['enabled']}\n",
    );
    let request = f.project.join("vibe.toml");
    write(&request,fs::read_to_string(&request).unwrap().replace("configuration={policy='require-selection',default_features=false,selections={extra={intent='exact',members=['enabled']}}}","configuration={policy='require-selection',default_features=false,profile='with-extra',parameters={verbose=false}}"));
    f
}
#[test]
fn profile_boolean_old_reader_writer_and_profile_drift_refuse_unchanged() {
    let f = profile_boolean_fixture();
    f.install_ok();
    let before = fs::read(f.project.join("vibe.lock")).unwrap();
    let boot = f.static_boot();
    let hooks = f.hooks();
    let slot = fs::read(f.slot(DRIVER).join("vibe.toml")).unwrap();
    let record = &f.lock().configurations[&coordinate(DRIVER)];
    assert!(record.automatic_basis.is_none());
    assert!(
        record
            .consumers
            .values()
            .all(|input| input.profile.as_deref() == Some("with-extra")
                && input.selections.is_empty())
    );
    if let Some(old) = std::env::var_os("NST_PROFILE_BOOL_OLD_CLI") {
        for args in [
            vec![
                "show",
                "features",
                "--path",
                f.project.to_str().unwrap(),
                "--json",
            ],
            vec![
                "install",
                "--path",
                f.project.to_str().unwrap(),
                "--offline",
                "--assume-yes",
                "--json",
            ],
        ] {
            let out = std::process::Command::new(&old)
                .env_clear()
                .env("HOME", f.root.join("home"))
                .env("VIBE_SETTINGS", &f.user.settings)
                .env("PATH", "/usr/bin:/bin")
                .args(args)
                .output()
                .unwrap();
            assert!(!out.status.success());
            assert_eq!(fs::read(f.project.join("vibe.lock")).unwrap(), before);
            assert_eq!(fs::read(f.slot(DRIVER).join("vibe.toml")).unwrap(), slot);
            assert_eq!(f.static_boot(), boot);
            assert_eq!(f.hooks(), hooks);
        }
    }
    let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0");
    let new = f.registry.join(GROUP).join(DRIVER).join("v1.1.0");
    write(
        &new.join("vibe.toml"),
        fs::read_to_string(source.join("vibe.toml"))
            .unwrap()
            .replace("version='1.0.0'", "version='1.1.0'")
            .replace(
                "selections.extra={intent='exact',members=['enabled']}",
                "selections.extra={intent='exact',members=['disabled']}",
            ),
    );
    write(
        &new.join("boot/rules.md"),
        fs::read(source.join("boot/rules.md")).unwrap(),
    );
    assert!(!update(&f).status.success());
    assert_eq!(fs::read(f.project.join("vibe.lock")).unwrap(), before);
    assert_eq!(fs::read(f.slot(DRIVER).join("vibe.toml")).unwrap(), slot);
    assert_eq!(f.static_boot(), boot);
    assert_eq!(f.hooks(), hooks);
}
#[test]
fn profile_boolean_unknown_wrong_type_preset_and_default_refuse_before_effects() {
    for mutation in ["unknown", "wrong-type", "default", "private"] {
        let f = profile_boolean_fixture();
        let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
        let request = f.project.join("vibe.toml");
        match mutation {
            "unknown" => write(
                &request,
                fs::read_to_string(&request)
                    .unwrap()
                    .replace("profile='with-extra'", "profile='unknown'"),
            ),
            "wrong-type" => write(
                &request,
                fs::read_to_string(&request)
                    .unwrap()
                    .replace("verbose=false", "verbose='wrong'"),
            ),
            "default" => write(
                &source,
                fs::read_to_string(&source)
                    .unwrap()
                    .replace("required=true", "required=true\ndefault=true"),
            ),
            "private" => write(
                &source,
                fs::read_to_string(&source)
                    .unwrap()
                    .replace("required=true", "required=true\nsecret=true"),
            ),
            _ => unreachable!(),
        }
        let authored = fs::read(&request).unwrap();
        let out = f.install();
        assert!(!out.status.success(), "{mutation}");
        assert!(!f.project.join("vibe.lock").exists());
        for name in [DRIVER, BASE, EXTRA] {
            assert!(!f.slot(name).exists());
        }
        assert!(!f.project.join("effects.log").exists());
        for relative in [common::static_md_rel(), common::static_xml_rel()] {
            assert!(!f.project.join(relative).exists());
        }
        assert_eq!(fs::read(&request).unwrap(), authored);
    }
}

#[test]
fn profile_boolean_preset_omitted_equal_opposite_and_explicit_control_are_separate() {
    for (preset, answer) in [
        (true, None),
        (true, Some(true)),
        (true, Some(false)),
        (false, Some(false)),
    ] {
        let f = profile_boolean_fixture();
        let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
        if preset {
            write(
                &source,
                fs::read_to_string(&source).unwrap() + "parameters.verbose=true\n",
            );
        }
        let request = f.project.join("vibe.toml");
        let text = fs::read_to_string(&request).unwrap();
        write(
            &request,
            match answer {
                None => text.replace(",parameters={verbose=false}", ""),
                Some(value) => text.replace("verbose=false", &format!("verbose={value}")),
            },
        );
        let authored = fs::read(&request).unwrap();
        let source_bytes = fs::read(&source).unwrap();
        let package = Manifest::read(&source).unwrap();
        let consumer = Manifest::read(&request).unwrap();
        let definition = package.configuration.as_ref().unwrap();
        let input = consumer.requires.configurations.values().next().unwrap();
        assert_eq!(
            definition.profiles[0].parameters.contains_key("verbose"),
            preset
        );
        assert_eq!(input.parameters.contains_key("verbose"), answer.is_some());
        let normalized = definition.normalize_input(input, "./vibe.toml").unwrap();
        assert_eq!(
            normalized.parameters["verbose"],
            vibe_core::manifest::ParameterValue::Bool(answer.unwrap_or(true))
        );
        let out = f.install();
        let diagnostic = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        {
            assert!(out.status.success(), "{diagnostic}");
            let lock = f.lock();
            let record = &lock.configurations[&coordinate(DRIVER)];
            assert_eq!(record.consumers.values().next().unwrap(), input);
            assert!(f.slot(EXTRA).exists());
            assert!(f.static_boot().contains(EXTRA_MARKER));
            let show = f
                .user
                .vibe()
                .args([
                    "show",
                    "configuration-data",
                    "--path",
                    f.project.to_str().unwrap(),
                    "--json",
                ])
                .output()
                .unwrap();
            assert!(show.status.success());
            let report: serde_json::Value = serde_json::from_slice(&show.stdout).unwrap();
            assert_eq!(
                report["packages"][0]["parameters"]["verbose"],
                answer.unwrap_or(true)
            );
            let before = fs::read(f.project.join("vibe.lock")).unwrap();
            let boot = f.static_boot();
            let hooks = f.hooks();
            f.install_ok();
            assert_eq!(fs::read(f.project.join("vibe.lock")).unwrap(), before);
            assert_eq!(f.static_boot(), boot);
            assert_eq!(f.hooks(), hooks);
        }
        assert_eq!(fs::read(&request).unwrap(), authored);
        assert_eq!(fs::read(&source).unwrap(), source_bytes);
    }
}

#[test]
fn profile_preset_false_override_old_clients_and_update_drift_are_lossless() {
    for answer in [None, Some(false), Some(true)] {
        let f = profile_boolean_fixture();
        let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
        write(
            &source,
            fs::read_to_string(&source).unwrap() + "parameters.verbose=false\n",
        );
        let request = f.project.join("vibe.toml");
        let text = fs::read_to_string(&request).unwrap();
        write(
            &request,
            match answer {
                None => text.replace(",parameters={verbose=false}", ""),
                Some(value) => text.replace("verbose=false", &format!("verbose={value}")),
            },
        );
        f.install_ok();
        let before = fs::read(f.project.join("vibe.lock")).unwrap();
        let boot = f.static_boot();
        let hooks = f.hooks();
        let slot = fs::read(f.slot(DRIVER).join("vibe.toml")).unwrap();
        let lock = f.lock();
        let record = &lock.configurations[&coordinate(DRIVER)];
        assert_eq!(
            record
                .consumers
                .values()
                .next()
                .unwrap()
                .parameters
                .contains_key("verbose"),
            answer.is_some()
        );
        assert_eq!(
            record
                .selected_parameters(vibe_core::PackageKind::Flow)
                .unwrap()["verbose"],
            vibe_core::manifest::ParameterValue::Bool(answer.unwrap_or(false))
        );
        if let Some(old) = std::env::var_os("NST_PRESET_OLD_CLI") {
            for args in [
                vec![
                    "show",
                    "features",
                    "--path",
                    f.project.to_str().unwrap(),
                    "--json",
                ],
                vec![
                    "install",
                    "--path",
                    f.project.to_str().unwrap(),
                    "--offline",
                    "--assume-yes",
                    "--json",
                ],
            ] {
                let out = std::process::Command::new(&old)
                    .env_clear()
                    .env("HOME", f.root.join("home"))
                    .env("VIBE_SETTINGS", &f.user.settings)
                    .env("PATH", "/usr/bin:/bin")
                    .args(args)
                    .output()
                    .unwrap();
                assert!(!out.status.success());
                assert_eq!(fs::read(f.project.join("vibe.lock")).unwrap(), before);
                assert_eq!(fs::read(f.slot(DRIVER).join("vibe.toml")).unwrap(), slot);
                assert_eq!(f.static_boot(), boot);
                assert_eq!(f.hooks(), hooks);
            }
        }
        f.install_ok();
        assert_eq!(fs::read(f.project.join("vibe.lock")).unwrap(), before);
        assert_eq!(f.static_boot(), boot);
        assert_eq!(f.hooks(), hooks);
        let new = f.registry.join(GROUP).join(DRIVER).join("v1.1.0");
        write(
            &new.join("vibe.toml"),
            fs::read_to_string(&source)
                .unwrap()
                .replace("version='1.0.0'", "version='1.1.0'")
                .replace("parameters.verbose=false", "parameters.verbose=true"),
        );
        write(
            &new.join("boot/rules.md"),
            fs::read(source.parent().unwrap().join("boot/rules.md")).unwrap(),
        );
        assert!(!update(&f).status.success());
        assert_eq!(fs::read(f.project.join("vibe.lock")).unwrap(), before);
        assert_eq!(fs::read(f.slot(DRIVER).join("vibe.toml")).unwrap(), slot);
        assert_eq!(f.static_boot(), boot);
        assert_eq!(f.hooks(), hooks);
    }
}
#[test]
fn unselected_profile_preset_cannot_supply_missing_required_boolean() {
    let f = profile_boolean_fixture();
    let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
    write(
        &source,
        fs::read_to_string(&source).unwrap()
            + "\n[[configuration.profiles]]\nid='other'\nname='Other'\nparameters.verbose=true\n",
    );
    let request = f.project.join("vibe.toml");
    write(
        &request,
        fs::read_to_string(&request)
            .unwrap()
            .replace(",parameters={verbose=false}", ""),
    );
    let before = fs::read(&request).unwrap();
    let out = f.install();
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr)
            .contains("missing required public parameter `verbose`")
    );
    assert!(!f.project.join("vibe.lock").exists());
    for name in [BASE, DRIVER, EXTRA] {
        assert!(!f.slot(name).exists());
    }
    for relative in [common::static_md_rel(), common::static_xml_rel()] {
        assert!(!f.project.join(relative).exists());
    }
    assert!(!f.project.join("effects.log").exists());
    assert_eq!(fs::read(&request).unwrap(), before);
}

#[test]
fn uninstalled_ordinary_configuration_existing_cli_and_mcp_do_not_describe_or_evaluate() {
    let f = Fixture::new(false);
    let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
    write(
        &source,
        fs::read_to_string(&source).unwrap()
            + "\n[[configuration.parameters]]\nid='verbose'\nname='Verbose'\ntype='bool'\nrequired=true\n",
    );
    let request = f.project.join("vibe.toml");
    write(
        &request,
        fs::read_to_string(&request)
            .unwrap()
            .replace("version='^1'", "version='=1.0.0'"),
    );
    let authored = fs::read(&request).unwrap();
    let source_before = fs::read(&source).unwrap();
    let help = f.user.vibe().args(["--help"]).output().unwrap();
    assert!(help.status.success());
    let grammar = String::from_utf8_lossy(&help.stdout);
    assert!(grammar.contains("show") && grammar.contains("install"));
    let install_help = f.user.vibe().args(["install", "--help"]).output().unwrap();
    assert!(install_help.status.success());
    assert!(String::from_utf8_lossy(&install_help.stdout).contains("--configuration-input"));
    let submission = serde_json::json!({"protocol":"vibe-configuration-input/1","input":{"intent":{"policy":"require-selection","defaultFeatures":false,"features":[],"selections":{"extra":{"intent":"exact","members":["disabled"]}},"parameters":{"verbose":{"kind":"bool","value":false}}},"disabledFeatures":[]}});
    let bytes = serde_json::to_vec(&submission).unwrap();
    vibe_app::configuration::parse_submission(&bytes)
        .expect("valid existing generated external input");
    let submitted = f.root.join("submitted.json");
    write(&submitted, &bytes);
    let trial = f
        .user
        .vibe()
        .args([
            "install",
            "--offline",
            "--assume-yes",
            "--json",
            "--path",
            f.project.to_str().unwrap(),
            "--configuration-input",
            submitted.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!trial.status.success());
    let scope_diagnostic = format!(
        "{}{}",
        String::from_utf8_lossy(&trial.stdout),
        String::from_utf8_lossy(&trial.stderr)
    );
    assert!(
        scope_diagnostic.contains("belongs to a selected app installer"),
        "{scope_diagnostic}"
    );
    assert_eq!(fs::read(&submitted).unwrap(), bytes);
    println!("ordinary external submission scope refusal: {scope_diagnostic}");
    let show = f
        .user
        .vibe()
        .args([
            "show",
            "configuration-data",
            "--path",
            f.project.to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();
    assert!(!show.status.success());
    let show_diagnostic = format!(
        "{}{}",
        String::from_utf8_lossy(&show.stdout),
        String::from_utf8_lossy(&show.stderr)
    );
    assert!(show_diagnostic.contains("vibe.lock"), "{show_diagnostic}");
    println!("cold retained-data refusal: {show_diagnostic}");
    let script=[serde_json::json!({"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}).to_string(),serde_json::json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"query_package","arguments":{"name":coordinate(DRIVER)}}}).to_string()].join("\n")+"\n";
    let transport = f
        .user
        .vibe()
        .args([
            "--offline",
            "mcp",
            "serve",
            "--path",
            f.project.to_str().unwrap(),
        ])
        .write_stdin(script)
        .output()
        .unwrap();
    assert!(transport.status.success());
    let rows: Vec<serde_json::Value> = String::from_utf8(transport.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let tools = rows.iter().find(|row| row["id"] == 1).unwrap()["result"]["tools"]
        .as_array()
        .unwrap();
    let names: Vec<_> = tools
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"query_package"));
    // Historical absence receipt is retained in the qualification report;
    // the new source-bound tool does not turn installed query into preview.
    assert!(names.contains(&"configuration_preview"));
    let query = rows.iter().find(|row| row["id"] == 2).unwrap();
    assert!(query["result"]["isError"] == true || query.get("error").is_some());
    assert!(
        query.to_string().contains("lockfile") || query.to_string().contains("vibe.lock"),
        "{query}"
    );
    assert!(!query.to_string().contains("invalid arguments"), "{query}");
    println!(
        "registered tools: {}",
        serde_json::to_string(&names).unwrap()
    );
    println!("cold registered query_package refusal: {query}");
    assert_eq!(fs::read(&request).unwrap(), authored);
    assert_eq!(fs::read(&source).unwrap(), source_before);
    assert!(!f.project.join("vibe.lock").exists());
    for name in [BASE, DRIVER, EXTRA] {
        assert!(!f.slot(name).exists());
    }
    for relative in [common::static_md_rel(), common::static_xml_rel()] {
        assert!(!f.project.join(relative).exists());
    }
    assert!(!f.project.join("effects.log").exists());
    // Separate control: the same definition is installable after authored answers.
    let control = Fixture::new(false);
    let target = control
        .registry
        .join(GROUP)
        .join(DRIVER)
        .join("v1.0.0/vibe.toml");
    write(&target, source_before);
    let request = control.project.join("vibe.toml");
    write(
        &request,
        fs::read_to_string(&request).unwrap().replace(
            "members=['disabled']}}}",
            "members=['disabled']}},parameters={verbose=false}}",
        ),
    );
    control.install_ok();
    let data = control
        .user
        .vibe()
        .args([
            "show",
            "configuration-data",
            "--path",
            control.project.to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();
    assert!(data.status.success());
    let data: serde_json::Value = serde_json::from_slice(&data.stdout).unwrap();
    assert_eq!(data["packages"][0]["parameters"]["verbose"], false);
}

// Complete scratch-tree observation: absence differs from an empty root and
// no `.vibe` entry is filtered as it is in project semantic snapshots.
fn preview_scratch_snapshot(root: &Path) -> Option<BTreeMap<PathBuf, super::workspace::Entry>> {
    use std::os::unix::fs::PermissionsExt;
    fn walk(root: &Path, path: &Path, out: &mut BTreeMap<PathBuf, super::workspace::Entry>) {
        use super::workspace::Entry;
        let meta = fs::symlink_metadata(path).unwrap();
        let entry = if meta.file_type().is_symlink() {
            Entry::Symlink(fs::read_link(path).unwrap())
        } else if meta.is_dir() {
            Entry::Directory
        } else {
            Entry::File(meta.permissions().mode() & 0o777, fs::read(path).unwrap())
        };
        out.insert(path.strip_prefix(root).unwrap().to_owned(), entry);
        if meta.is_dir() {
            for child in fs::read_dir(path).unwrap() {
                walk(root, &child.unwrap().path(), out);
            }
        }
    }
    match fs::symlink_metadata(root) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => panic!("scratch root cannot be observed: {error}"),
        Ok(_) => {
            let mut out = BTreeMap::new();
            walk(root, root, &mut out);
            Some(out)
        }
    }
}
fn preview_fixture() -> Fixture {
    let mut f = Fixture::new(false);
    let registry = f.project.join("registry");
    fs::rename(&f.registry, &registry).unwrap();
    f.registry = registry;
    let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
    let text = fs::read_to_string(&source).unwrap()
        .replace("# This feature is never selected; base must remain a mandatory dependency.\n[[configuration.features]]\nid = \"mention-required\"\nname = \"Mention mandatory dependency\"\neffects = [{ kind = \"dependency\", target = \"org.vibevm/oracle-base\" }]\n", "")
        .replace("defaults = [\"disabled\"]", "");
    write(
        &source,
        text + "\n[[configuration.parameters]]\nid='verbose'\nname='Verbose'\ntype='bool'\nrequired=true\n",
    );
    write(
        &f.project.join("vibe.toml"),
        format!(
            "[project]\nname='preview'\nversion='0.1.0'\n[requires.packages.'{GROUP}/{DRIVER}']\nversion='=1.0.0'\nlink='static-transitive'\n[[registry]]\nname='fixture'\nurl='file://{}'\n",
            f.registry.display()
        ),
    );
    f
}
fn preview_cli(f: &Fixture, input: Option<&Path>) -> Output {
    let mut cmd = f.user.vibe();
    cmd.args([
        "show",
        "configuration-preview",
        &coordinate(DRIVER),
        "--path",
        f.project.to_str().unwrap(),
        "--json",
    ]);
    if let Some(input) = input {
        cmd.args(["--input", input.to_str().unwrap()]);
    }
    cmd.output().unwrap()
}
fn preview_mcp(f: &Fixture, arguments: serde_json::Value) -> serde_json::Value {
    let script = serde_json::json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"configuration_preview","arguments":arguments}}).to_string()+"\n";
    let out = f
        .user
        .vibe()
        .args([
            "--offline",
            "mcp",
            "serve",
            "--path",
            f.project.to_str().unwrap(),
        ])
        .write_stdin(script)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .find(|row| row["id"] == 1)
        .unwrap()
}
#[test]
fn ordinary_preview_cold_cli_and_registered_stdio_mcp_match_install_witness() {
    let f = preview_fixture();
    fs::remove_dir(&f.user.cache).unwrap();
    fs::remove_dir(&f.user.search_cache).unwrap();
    let before = super::workspace::semantic_snapshot(&f.project);
    let cache_before = preview_scratch_snapshot(&f.user.cache);
    let search_before = preview_scratch_snapshot(&f.user.search_cache);
    let cli = preview_cli(&f, None);
    assert!(
        cli.status.success(),
        "{}{}",
        String::from_utf8_lossy(&cli.stdout),
        String::from_utf8_lossy(&cli.stderr)
    );
    let described: serde_json::Value = serde_json::from_slice(&cli.stdout).unwrap();
    assert_eq!(described["questions"].as_array().unwrap().len(), 2);
    assert!(described.get("original").is_none() && described.get("resolved").is_none());
    let mcp = preview_mcp(&f, serde_json::json!({"package":coordinate(DRIVER)}));
    assert_eq!(mcp["result"]["structuredContent"], described, "{mcp}");
    let answers =
        serde_json::json!({"selections":{"extra":"disabled"},"parameters":{"verbose":false}});
    let input = f.root.join("answers.json");
    write(&input, serde_json::to_vec(&answers).unwrap());
    let cli = preview_cli(&f, Some(&input));
    assert!(
        cli.status.success(),
        "{}{}",
        String::from_utf8_lossy(&cli.stdout),
        String::from_utf8_lossy(&cli.stderr)
    );
    let evaluated: serde_json::Value = serde_json::from_slice(&cli.stdout).unwrap();
    assert_eq!(evaluated["original"], answers);
    assert_eq!(evaluated["resolved"]["parameters"]["verbose"], false);
    assert_eq!(
        evaluated["resolved"]["features"],
        serde_json::json!(["disabled"])
    );
    assert_eq!(evaluated["questions"], serde_json::json!([]));
    let mcp = preview_mcp(
        &f,
        serde_json::json!({"package":coordinate(DRIVER),"input":answers}),
    );
    assert_eq!(mcp["result"]["structuredContent"], evaluated, "{mcp}");
    assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
    assert!(!f.user.settings.join("cache").exists());
    assert_eq!(preview_scratch_snapshot(&f.user.cache), cache_before);
    assert_eq!(
        preview_scratch_snapshot(&f.user.search_cache),
        search_before
    );
    let control = preview_fixture();
    let request = control.project.join("vibe.toml");
    write(&request, fs::read_to_string(&request).unwrap().replace("link='static-transitive'", "link='static-transitive'\nconfiguration={policy='require-selection',default_features=false,selections={extra={intent='exact',members=['disabled']}},parameters={verbose=false}}"));
    control.install_ok();
    let lock = control.lock();
    let row = lock
        .packages
        .iter()
        .find(|row| row.name.as_str() == DRIVER)
        .unwrap();
    assert_eq!(
        evaluated["source"]["content_hash"],
        row.content_hash.to_string()
    );
    assert_eq!(
        lock.configurations[&coordinate(DRIVER)]
            .selected_parameters(row.kind)
            .unwrap()["verbose"],
        vibe_core::manifest::ParameterValue::Bool(false)
    );
}
#[test]
fn ordinary_preview_refuses_invalid_answers_and_source_without_partial_effects() {
    for input in [
        serde_json::Value::Null,
        serde_json::json!({"selections":{"unknown":"disabled"},"parameters":{"verbose":false}}),
        serde_json::json!({"selections":{"extra":"unknown"},"parameters":{"verbose":false}}),
        serde_json::json!({"selections":{"extra":"disabled"},"parameters":{"verbose":"false"}}),
        serde_json::json!({"selections":{"extra":"disabled"},"parameters":{}}),
        serde_json::json!({"protocol":"vibe-configuration-input/1","input":{}}),
    ] {
        let f = preview_fixture();
        let before = super::workspace::semantic_snapshot(&f.project);
        let cache_before = preview_scratch_snapshot(&f.user.cache);
        let search_before = preview_scratch_snapshot(&f.user.search_cache);
        let path = f.root.join("answers.json");
        write(&path, serde_json::to_vec(&input).unwrap());
        assert!(!preview_cli(&f, Some(&path)).status.success());
        let mcp = preview_mcp(
            &f,
            serde_json::json!({"package":coordinate(DRIVER),"input":input}),
        );
        assert!(
            mcp["result"]["isError"] == true || mcp.get("error").is_some(),
            "{mcp}"
        );
        assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
        assert!(!f.user.settings.join("cache").exists());
        assert_eq!(preview_scratch_snapshot(&f.user.cache), cache_before);
        assert_eq!(
            preview_scratch_snapshot(&f.user.search_cache),
            search_before
        );
    }
    for mutation in [
        "external",
        "unavailable",
        "ambiguous",
        "symlink",
        "ambient",
        "store",
    ] {
        let f = preview_fixture();
        let manifest = f.project.join("vibe.toml");
        match mutation {
            "external" => {
                let outside = f.root.join("outside");
                fs::rename(&f.registry, &outside).unwrap();
                write(
                    &manifest,
                    fs::read_to_string(&manifest)
                        .unwrap()
                        .replace(f.registry.to_str().unwrap(), outside.to_str().unwrap()),
                );
            }
            "unavailable" => {
                fs::rename(&f.registry, f.root.join("unavailable")).unwrap();
            }
            "ambiguous" => write(
                &manifest,
                fs::read_to_string(&manifest).unwrap()
                    + "\n[[registry]]\nname='other'\nurl='file:///unavailable'\n",
            ),
            "symlink" => {
                let held = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/boot");
                let outside = f.root.join("outside");
                fs::rename(&held, &outside).unwrap();
                std::os::unix::fs::symlink(&outside, &held).unwrap();
            }
            "ambient" => write(
                &f.user.settings.join("registry.toml"),
                "synthetic competing layer",
            ),
            "store" => fs::create_dir_all(f.user.settings.join("cache")).unwrap(),
            _ => unreachable!(),
        }
        let before = super::workspace::semantic_snapshot(&f.project);
        let cache_before = preview_scratch_snapshot(&f.user.cache);
        let search_before = preview_scratch_snapshot(&f.user.search_cache);
        assert!(!preview_cli(&f, None).status.success(), "{mutation}");
        let mcp = preview_mcp(&f, serde_json::json!({"package":coordinate(DRIVER)}));
        assert!(
            mcp["result"]["isError"] == true || mcp.get("error").is_some(),
            "{mutation}: {mcp}"
        );
        assert_eq!(preview_scratch_snapshot(&f.user.cache), cache_before);
        assert_eq!(
            preview_scratch_snapshot(&f.user.search_cache),
            search_before
        );
        assert_eq!(
            super::workspace::semantic_snapshot(&f.project),
            before,
            "{mutation}"
        );
    }
}

#[test]
fn ordinary_preview_requires_explicit_host_mode_without_startup_seeding() {
    for ci_only in [false, true] {
        let f = preview_fixture();
        let before = super::workspace::semantic_snapshot(&f.project);
        let cache_before = preview_scratch_snapshot(&f.user.cache);
        let search_before = preview_scratch_snapshot(&f.user.search_cache);
        let settings = preview_scratch_snapshot(&f.user.settings);
        let mut cli = f.user.vibe();
        cli.env_remove("VIBE_NO_DEFAULT_REGISTRY").env_remove("CI");
        if ci_only {
            cli.env("CI", "1");
        }
        let out = cli
            .args([
                "show",
                "configuration-preview",
                &coordinate(DRIVER),
                "--path",
                f.project.to_str().unwrap(),
                "--json",
            ])
            .output()
            .unwrap();
        assert!(!out.status.success());
        let diagnostic = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            diagnostic.contains("isolated no-default-registry posture"),
            "{diagnostic}"
        );
        let mut mcp = f.user.vibe();
        mcp.env_remove("VIBE_NO_DEFAULT_REGISTRY").env_remove("CI");
        if ci_only {
            mcp.env("CI", "1");
        }
        let script=serde_json::json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"configuration_preview","arguments":{"package":coordinate(DRIVER)}}}).to_string()+"\n";
        let out = mcp
            .args([
                "--offline",
                "mcp",
                "serve",
                "--path",
                f.project.to_str().unwrap(),
            ])
            .write_stdin(script)
            .output()
            .unwrap();
        assert!(out.status.success());
        let diagnostic = String::from_utf8(out.stdout).unwrap();
        assert!(
            diagnostic.contains("isolated no-default-registry posture"),
            "{diagnostic}"
        );
        assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
        assert_eq!(preview_scratch_snapshot(&f.user.settings), settings);
        assert!(!f.user.settings.join("cache").exists());
        assert_eq!(preview_scratch_snapshot(&f.user.cache), cache_before);
        assert_eq!(
            preview_scratch_snapshot(&f.user.search_cache),
            search_before
        );
    }
}

#[test]
fn ordinary_preview_refuses_effective_remap_layers_without_source_or_cache_changes() {
    for mode in [
        "user-settings",
        "project-settings",
        "argument",
        "environment",
    ] {
        let f = preview_fixture();
        let remap = "[[remap]]\nprefix='file:///'\ndeny=true\nreason='synthetic preview refusal'\n";
        let remap_file = f.root.join("remap.toml");
        write(&remap_file, remap);
        match mode {
            "user-settings" => write(&f.user.settings.join("settings.toml"), remap),
            "project-settings" => write(&f.project.join(".vibe/settings.local.toml"), remap),
            _ => {}
        }
        let project = preview_scratch_snapshot(&f.project);
        let settings = preview_scratch_snapshot(&f.user.settings);
        let cache = preview_scratch_snapshot(&f.user.cache);
        let search = preview_scratch_snapshot(&f.user.search_cache);
        let mut cli = f.user.vibe();
        let mut mcp = f.user.vibe();
        if mode == "argument" {
            cli.args(["--remap", remap_file.to_str().unwrap()]);
            mcp.args(["--remap", remap_file.to_str().unwrap()]);
        }
        if mode == "environment" {
            cli.env("VIBE_REMAP", &remap_file);
            mcp.env("VIBE_REMAP", &remap_file);
        }
        let out = cli
            .args([
                "show",
                "configuration-preview",
                &coordinate(DRIVER),
                "--path",
                f.project.to_str().unwrap(),
                "--json",
            ])
            .output()
            .unwrap();
        assert!(!out.status.success(), "{mode}");
        let diagnostic = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            diagnostic.contains("remap") || diagnostic.contains("ambient source layer"),
            "{mode}: {diagnostic}"
        );
        let script=serde_json::json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"configuration_preview","arguments":{"package":coordinate(DRIVER)}}}).to_string()+"\n";
        let out = mcp
            .args([
                "--offline",
                "mcp",
                "serve",
                "--path",
                f.project.to_str().unwrap(),
            ])
            .write_stdin(script)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{mode}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let response: serde_json::Value = String::from_utf8(out.stdout)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
            .find(|row| row["id"] == 1)
            .unwrap();
        assert_eq!(response["result"]["isError"], true, "{mode}: {response}");
        assert_eq!(preview_scratch_snapshot(&f.project), project, "{mode}");
        assert_eq!(
            preview_scratch_snapshot(&f.user.settings),
            settings,
            "{mode}"
        );
        assert_eq!(preview_scratch_snapshot(&f.user.cache), cache, "{mode}");
        assert_eq!(
            preview_scratch_snapshot(&f.user.search_cache),
            search,
            "{mode}"
        );
    }
}

#[test]
fn ordinary_console_pty_unanswered_input_is_distinct_from_serialized_preview() {
    let f = preview_fixture();
    let scope = f.root.join("pty-control");
    fs::create_dir_all(&scope).unwrap();
    let source=include_str!("../common/pty-runner.c")
        .replace("#include <errno.h>","#include <errno.h>\n#include <poll.h>\n#include <signal.h>\n#include <time.h>")
        .replace("    char buffer[4096];", "    time_t started = time(NULL);\n    char buffer[4096];")
        .replace("        ssize_t length = read(master, buffer, sizeof(buffer));", "        if (time(NULL) - started > 15) { kill(-child, SIGKILL); waitpid(child, NULL, 0); return 131; }\n        struct pollfd ready = { .fd = master, .events = POLLIN };\n        if (poll(&ready, 1, 1000) == 0) continue;\n        ssize_t length = read(master, buffer, sizeof(buffer));");
    let c = scope.join("runner.c");
    let runner = scope.join("runner");
    write(&c, source);
    let compiler = std::env::var_os("CC").unwrap_or_else(|| "cc".into());
    let build = std::process::Command::new(compiler)
        .args(["-O0", "-o"])
        .arg(&runner)
        .arg(&c)
        .arg("-lutil")
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let project = preview_scratch_snapshot(&f.project);
    let settings = preview_scratch_snapshot(&f.user.settings);
    let cache = preview_scratch_snapshot(&f.user.cache);
    let search = preview_scratch_snapshot(&f.user.search_cache);
    let output = std::process::Command::new(&runner)
        .arg(env!("CARGO_BIN_EXE_vibe"))
        .args([
            "install",
            "--offline",
            "--path",
            f.project.to_str().unwrap(),
        ])
        .current_dir(&f.project)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", f.root.join("home"))
        .env("TERM", "xterm")
        .env("VIBE_SETTINGS", &f.user.settings)
        .env("VIBE_REGISTRY_CACHE", &f.user.cache)
        .env("VIBEVM_SEARCH_CACHE_DIR", &f.user.search_cache)
        .env("VIBE_NO_DEFAULT_REGISTRY", "1")
        .output()
        .unwrap();
    let transcript = String::from_utf8_lossy(&output.stdout);
    println!(
        "ordinary cold PTY status={} transcript={transcript}",
        output.status
    );
    assert!(!output.status.success());
    assert_ne!(
        output.status.code(),
        Some(131),
        "PTY timed out instead of reaching actual refusal"
    );
    assert!(
        transcript.contains("this operation does not yet evaluate common configuration"),
        "{transcript}"
    );
    assert!(
        transcript.contains("PROP-069#AUTHORED-SCOPES"),
        "{transcript}"
    );
    let mut after_install = preview_scratch_snapshot(&f.project).unwrap();
    assert_eq!(
        after_install.remove(Path::new(".vibe")),
        Some(super::workspace::Entry::Directory)
    );
    assert!(
        matches!(after_install.remove(Path::new(".vibe/lifecycle.lock")), Some(super::workspace::Entry::File(_, bytes)) if bytes.is_empty())
    );
    assert_eq!(
        Some(after_install),
        project,
        "only exact lease infrastructure may differ"
    );
    let project = preview_scratch_snapshot(&f.project);
    println!(
        "install refusal infrastructure delta: .vibe directory plus empty .vibe/lifecycle.lock; all other project bytes unchanged"
    );
    assert_eq!(preview_scratch_snapshot(&f.user.settings), settings);
    assert_eq!(preview_scratch_snapshot(&f.user.cache), cache);
    assert_eq!(preview_scratch_snapshot(&f.user.search_cache), search);
    let input = f.root.join("serialized.json");
    let answers =
        serde_json::json!({"selections":{"extra":"disabled"},"parameters":{"verbose":false}});
    write(&input, serde_json::to_vec(&answers).unwrap());
    let control = preview_cli(&f, Some(&input));
    assert!(
        control.status.success(),
        "{}",
        String::from_utf8_lossy(&control.stderr)
    );
    let result: serde_json::Value = serde_json::from_slice(&control.stdout).unwrap();
    let mcp = preview_mcp(
        &f,
        serde_json::json!({"package":coordinate(DRIVER),"input":answers}),
    );
    assert_eq!(mcp["result"]["structuredContent"], result);
    assert_eq!(result["original"], answers);
    assert_eq!(result["resolved"]["parameters"]["verbose"], false);
    println!("separate serialized control={result}");
    assert_eq!(preview_scratch_snapshot(&f.project), project);
    assert_eq!(preview_scratch_snapshot(&f.user.settings), settings);
    assert_eq!(preview_scratch_snapshot(&f.user.cache), cache);
    assert_eq!(preview_scratch_snapshot(&f.user.search_cache), search);
}

#[test]
fn ordinary_interactive_preview_actual_pty_answers_cancel_interrupt_and_source_drift() {
    for mode in ["false", "true", "cancel", "interrupt", "drift"] {
        let f = preview_fixture();
        let driver = f.root.join("interactive-driver.py");
        write(&driver, include_str!("../common/ordinary-preview-pty.py"));
        let source = f
            .registry
            .join(GROUP)
            .join(DRIVER)
            .join("v1.0.0/payload.txt");
        let before = preview_scratch_snapshot(&f.project);
        let settings = preview_scratch_snapshot(&f.user.settings);
        let cache = preview_scratch_snapshot(&f.user.cache);
        let search = preview_scratch_snapshot(&f.user.search_cache);
        let out = std::process::Command::new("python3")
            .args(["-I"])
            .arg(&driver)
            .arg(env!("CARGO_BIN_EXE_vibe"))
            .arg(&f.project)
            .arg(mode)
            .arg(&source)
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", f.root.join("home"))
            .env("TERM", "xterm")
            .env("VIBE_SETTINGS", &f.user.settings)
            .env("VIBE_REGISTRY_CACHE", &f.user.cache)
            .env("VIBEVM_SEARCH_CACHE_DIR", &f.user.search_cache)
            .env("VIBE_NO_DEFAULT_REGISTRY", "1")
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{mode}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        println!("PTY {mode}: {report}");
        let transcript = report["transcript"].as_str().unwrap();
        assert_eq!(report["terminal_restored"], true, "{mode}: {report}");
        assert_eq!(report["cursor_visible"], true, "{mode}: {report}");
        if mode == "false" || mode == "true" {
            assert_eq!(report["status"], 0, "{report}");
            assert_eq!(report["terminal_restored"], true, "{report}");
            let start = transcript
                .find("{\n")
                .or_else(|| transcript.find("{\r\n"))
                .expect("actual result object");
            let result: serde_json::Value = serde_json::from_str(&transcript[start..]).unwrap();
            let answers = serde_json::json!({"selections":{"extra":"disabled"},"parameters":{"verbose":mode=="true"}});
            assert_eq!(result["original"], answers);
            let input = f.root.join("answers.json");
            write(&input, serde_json::to_vec(&answers).unwrap());
            let cli = preview_cli(&f, Some(&input));
            assert!(cli.status.success());
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&cli.stdout).unwrap(),
                result
            );
            assert_eq!(
                preview_mcp(
                    &f,
                    serde_json::json!({"package":coordinate(DRIVER),"input":answers})
                )["result"]["structuredContent"],
                result
            );
        } else {
            assert_ne!(report["status"], 0, "{report}");
            assert!(!transcript.contains("\"resolved\""), "{report}");
            if mode == "cancel" {
                assert_eq!(report["terminal_restored"], true, "{report}");
            }
            if mode == "drift" {
                assert!(
                    transcript.contains("source changed after questions"),
                    "{report}"
                );
                let original = match before
                    .as_ref()
                    .unwrap()
                    .get(&source.strip_prefix(&f.project).unwrap().to_path_buf())
                    .unwrap()
                {
                    super::workspace::Entry::File(_, bytes) => bytes,
                    _ => panic!("source file"),
                };
                write(&source, original);
            }
        }
        assert_eq!(preview_scratch_snapshot(&f.project), before, "{mode}");
        assert_eq!(
            preview_scratch_snapshot(&f.user.settings),
            settings,
            "{mode}"
        );
        assert_eq!(preview_scratch_snapshot(&f.user.cache), cache, "{mode}");
        assert_eq!(
            preview_scratch_snapshot(&f.user.search_cache),
            search,
            "{mode}"
        );
    }
}

#[test]
fn ordinary_interactive_preview_incompatible_flags_refuse_before_prompt_or_effects() {
    for mode in [
        "input",
        "json",
        "unattended",
        "environment-unattended",
        "nonterminal",
    ] {
        let f = preview_fixture();
        let input = f.root.join("input.json");
        write(&input, "null");
        let project = preview_scratch_snapshot(&f.project);
        let settings = preview_scratch_snapshot(&f.user.settings);
        let cache = preview_scratch_snapshot(&f.user.cache);
        let search = preview_scratch_snapshot(&f.user.search_cache);
        let mut command = f.user.vibe();
        command.args([
            "show",
            "configuration-preview",
            &coordinate(DRIVER),
            "--path",
            f.project.to_str().unwrap(),
            "--interactive",
        ]);
        match mode {
            "input" => {
                command.args(["--input", input.to_str().unwrap()]);
            }
            "json" => {
                command.arg("--json");
            }
            "unattended" => {
                command.arg("--unattended");
            }
            "environment-unattended" => {
                command.env("VIBE_UNATTENDED", "1");
            }
            _ => {}
        }
        let out = command.output().unwrap();
        assert!(!out.status.success(), "{mode}");
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(!text.contains("Extra support (extra)"), "{mode}: {text}");
        assert!(!text.contains("\"resolved\""), "{mode}: {text}");
        assert_eq!(preview_scratch_snapshot(&f.project), project, "{mode}");
        assert_eq!(
            preview_scratch_snapshot(&f.user.settings),
            settings,
            "{mode}"
        );
        assert_eq!(preview_scratch_snapshot(&f.user.cache), cache, "{mode}");
        assert_eq!(
            preview_scratch_snapshot(&f.user.search_cache),
            search,
            "{mode}"
        );
    }
}

fn dormant_guard_fixture() -> Fixture {
    let f = Fixture::new(true);
    let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
    write(
        &source,
        package_header(DRIVER)
            + r#"
[requires.packages]
"org.vibevm/oracle-base"="^1"
"org.vibevm/oracle-extra"={version="^1",optional=true}
[[configuration.features]]
id='on'
name='Guard on'
[[configuration.features]]
id='off'
name='Guard off'
[[configuration.features]]
id='selected'
name='Remembered selection'
effects=[{kind='dependency',target='org.vibevm/oracle-extra'}]
[[configuration.groups]]
mode='choice'
id='switch'
name='Switch'
cardinality='exactly-one'
members=['on','off']
[[configuration.groups]]
mode='choice'
id='detail'
name='Guarded detail'
cardinality='exactly-one'
members=['selected']
guards=[{member='on'}]
"#,
    );
    set_guard_input(&f, true, true);
    f
}
fn set_guard_input(f: &Fixture, on: bool, retain: bool) {
    let detail = if retain {
        ",detail={intent='exact',members=['selected']}"
    } else {
        ""
    };
    write(
        &f.project.join("vibe.toml"),
        format!(
            "[project]\nname='guard-retention'\nversion='0.1.0'\n[requires.packages.'{GROUP}/{DRIVER}']\nversion='=1.0.0'\nlink='static-transitive'\nconfiguration={{policy='require-selection',default_features=false,selections={{switch={{intent='exact',members=['{}']}}{detail}}}}}\n[[registry]]\nname='fixture'\nurl='file://{}'\n",
            if on { "on" } else { "off" },
            f.registry.display()
        ),
    );
}
#[test]
fn ordinary_dormant_guard_switch_retains_authored_choice_or_names_public_boundary() {
    for retain in [true, false] {
        let f = dormant_guard_fixture();
        let source =
            Manifest::read(f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml")).unwrap();
        let definition = source.configuration.as_ref().unwrap();
        f.install_ok();
        let initial = f.lock();
        let initial_input = Manifest::read(f.project.join("vibe.toml"))
            .unwrap()
            .requires
            .configurations[&coordinate(DRIVER)]
            .clone();
        assert_eq!(
            initial.configurations[&coordinate(DRIVER)]
                .consumers
                .values()
                .next()
                .unwrap(),
            &initial_input
        );
        assert!(initial.packages.iter().any(|p| p.name.as_str() == EXTRA));
        assert!(f.slot(EXTRA).exists());
        assert!(f.static_boot().contains(EXTRA_MARKER));
        let hooks_active = f.hooks();
        assert!(hooks_active.get(EXTRA).copied().unwrap_or(0) > 0);
        set_guard_input(&f, false, retain);
        let off_authored = Manifest::read(f.project.join("vibe.toml"))
            .unwrap()
            .requires
            .configurations[&coordinate(DRIVER)]
            .clone();
        assert_eq!(off_authored.selections.contains_key("detail"), retain);
        let normalized = definition
            .normalize_input(&off_authored, "./vibe.toml")
            .unwrap();
        assert_eq!(normalized.choices.answers.contains_key("detail"), retain);
        let evaluated = definition
            .compile_choices()
            .unwrap()
            .evaluate(&normalized.features, &[normalized.choices]);
        let detail = evaluated
            .controls
            .iter()
            .find(|control| control.id == "detail")
            .unwrap();
        assert!(!detail.active);
        assert_eq!(
            detail
                .projections
                .iter()
                .any(|projection| projection.dormant.contains(&"selected".to_string())),
            retain
        );
        // The historical update refusal is retained in the qualification
        // receipt. Keep this install-only control independent of the newly
        // supported checked-update tests below.
        // Ordinary install on the existing project is its own admitted public
        // reconfiguration route, not an implementation bypass of update.
        let off = f.install();
        assert!(
            off.status.success(),
            "{}{}",
            String::from_utf8_lossy(&off.stdout),
            String::from_utf8_lossy(&off.stderr)
        );
        let dormant = f.lock();
        let record = &dormant.configurations[&coordinate(DRIVER)];
        assert_eq!(record.consumers.values().next().unwrap(), &off_authored);
        assert!(!record.features.contains(&"selected".to_string()));
        assert!(!dormant.packages.iter().any(|p| p.name.as_str() == EXTRA));
        assert!(!f.slot(EXTRA).exists());
        assert!(!f.static_boot().contains(EXTRA_MARKER));
        assert_eq!(f.hooks().get(EXTRA), hooks_active.get(EXTRA));
        let before = super::workspace::semantic_snapshot(&f.project);
        f.install_ok();
        assert_eq!(
            super::workspace::semantic_snapshot(&f.project),
            before,
            "off replay"
        );
        // Source declaration/effect remains present while inactive; only its
        // effective graph/boot/hook contribution disappears.
        assert!(
            fs::read_to_string(f.slot(DRIVER).join("vibe.toml"))
                .unwrap()
                .contains("Remembered selection")
        );
        set_guard_input(&f, true, retain);
        let before = super::workspace::semantic_snapshot(&f.project);
        let back = f.install();
        let diagnostic = format!(
            "{}{}",
            String::from_utf8_lossy(&back.stdout),
            String::from_utf8_lossy(&back.stderr)
        );
        println!(
            "guard back install retain={retain} status={} {diagnostic}",
            back.status
        );
        if retain {
            assert!(back.status.success(), "{diagnostic}");
            let lock = f.lock();
            assert!(lock.packages.iter().any(|p| p.name.as_str() == EXTRA));
            assert!(f.slot(EXTRA).exists());
            assert!(f.static_boot().contains(EXTRA_MARKER));
            assert!(f.hooks().get(EXTRA) > hooks_active.get(EXTRA));
            assert!(
                lock.configurations[&coordinate(DRIVER)]
                    .consumers
                    .values()
                    .all(|input| input.selections.contains_key("detail"))
            );
            let before = super::workspace::semantic_snapshot(&f.project);
            f.install_ok();
            assert_eq!(
                super::workspace::semantic_snapshot(&f.project),
                before,
                "back replay"
            );
        } else {
            assert!(
                !back.status.success(),
                "omission cannot revive remembered intent"
            );
            assert!(
                diagnostic.contains("configuration") || diagnostic.contains("selection"),
                "{diagnostic}"
            );
            assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
            assert!(!f.slot(EXTRA).exists());
            assert!(!f.static_boot().contains(EXTRA_MARKER));
        }
    }
}

#[test]
fn guarded_leaf_update_retires_proved_slot_retains_backup_and_reactivates() {
    let f = dormant_guard_fixture();
    f.install_ok();
    let active = f.lock();
    let source_hash = active
        .packages
        .iter()
        .find(|row| row.name.as_str() == EXTRA)
        .unwrap()
        .content_hash
        .clone();
    set_guard_input(&f, false, true);
    let off = update(&f);
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&off.stdout),
        String::from_utf8_lossy(&off.stderr)
    );
    assert!(off.status.success(), "{text}");
    let report: serde_json::Value = serde_json::Deserializer::from_slice(&off.stdout)
        .into_iter::<serde_json::Value>()
        .map(Result::unwrap)
        .find(|row| row["command"] == "update")
        .unwrap();
    println!("guarded leaf off report={report}");
    assert_eq!(report["ok"], true);
    let backup = &report["retained_backups"][0];
    assert_eq!(backup["state"], "published_backup_retained");
    assert_eq!(
        backup["slot"],
        f.slot(EXTRA)
            .strip_prefix(&f.project)
            .unwrap()
            .to_str()
            .unwrap()
    );
    let backup = PathBuf::from(backup["path"].as_str().unwrap());
    assert!(backup.join("slot/payload.txt").exists());
    assert!(!f.slot(EXTRA).exists());
    assert!(!f.static_boot().contains(EXTRA_MARKER));
    assert!(
        !f.lock()
            .packages
            .iter()
            .any(|row| row.name.as_str() == EXTRA)
    );
    let before = super::workspace::semantic_snapshot(&f.project);
    let hooks = f.hooks();
    let replay = update(&f);
    assert!(
        replay.status.success(),
        "{}{}",
        String::from_utf8_lossy(&replay.stdout),
        String::from_utf8_lossy(&replay.stderr)
    );
    let replay: serde_json::Value = serde_json::Deserializer::from_slice(&replay.stdout)
        .into_iter::<serde_json::Value>()
        .map(Result::unwrap)
        .find(|row| row["command"] == "update")
        .unwrap();
    assert!(replay.get("retained_backups").is_none());
    assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
    assert_eq!(f.hooks(), hooks);
    set_guard_input(&f, true, true);
    let on = update(&f);
    assert!(
        on.status.success(),
        "{}{}",
        String::from_utf8_lossy(&on.stdout),
        String::from_utf8_lossy(&on.stderr)
    );
    assert!(f.slot(EXTRA).exists());
    assert!(f.static_boot().contains(EXTRA_MARKER));
    assert_eq!(
        f.lock()
            .packages
            .iter()
            .find(|row| row.name.as_str() == EXTRA)
            .unwrap()
            .content_hash,
        source_hash
    );
    assert!(backup.join("slot/payload.txt").exists());
}

#[test]
fn guarded_leaf_update_foreign_slots_refuse_and_other_root_keeps_the_leaf() {
    for mutation in ["edited", "extra-file", "empty-directory"] {
        let f = dormant_guard_fixture();
        f.install_ok();
        set_guard_input(&f, false, true);
        match mutation {
            "edited" => write(&f.slot(EXTRA).join("payload.txt"), "edited"),
            "extra-file" => write(&f.slot(EXTRA).join("foreign"), "foreign"),
            _ => fs::create_dir(f.slot(EXTRA).join("foreign-empty")).unwrap(),
        }
        let before = super::workspace::semantic_snapshot(&f.project);
        let out = update(&f);
        assert!(!out.status.success(), "{mutation}");
        assert_eq!(
            super::workspace::semantic_snapshot(&f.project),
            before,
            "{mutation}"
        );
    }
    let f = dormant_guard_fixture();
    let path = f.project.join("vibe.toml");
    write(
        &path,
        fs::read_to_string(&path).unwrap().replace(
            "[[registry]]",
            &format!("[requires.packages.'{GROUP}/{EXTRA}']\nversion='=1.0.0'\nlink='static-transitive'\n[[registry]]"),
        ),
    );
    f.install_ok();
    let hooks = f.hooks();
    set_guard_input(&f, false, true);
    write(
        &path,
        fs::read_to_string(&path).unwrap().replace(
            "[[registry]]",
            &format!("[requires.packages.'{GROUP}/{EXTRA}']\nversion='=1.0.0'\nlink='static-transitive'\n[[registry]]"),
        ),
    );
    let out = update(&f);
    assert!(
        out.status.success(),
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(f.slot(EXTRA).exists());
    assert!(f.static_boot().contains(EXTRA_MARKER));
    assert_eq!(
        f.hooks(),
        hooks,
        "retained root does not reinstall the unchanged leaf hook"
    );
    let reports = serde_json::Deserializer::from_slice(&out.stdout)
        .into_iter::<serde_json::Value>()
        .map(Result::unwrap)
        .collect::<Vec<_>>();
    assert!(
        reports
            .iter()
            .find(|row| row["command"] == "update")
            .unwrap()
            .get("retained_backups")
            .is_none()
    );
}

#[test]
fn guarded_leaf_update_equal_bytes_changed_registry_refuses_before_effects() {
    let f = dormant_guard_fixture();
    f.install_ok();
    set_guard_input(&f, false, true);
    let other = f.root.join("other-registry");
    fs::rename(&f.registry, &other).unwrap();
    let request = f.project.join("vibe.toml");
    write(
        &request,
        fs::read_to_string(&request)
            .unwrap()
            .replace(f.registry.to_str().unwrap(), other.to_str().unwrap()),
    );
    let before = super::workspace::semantic_snapshot(&f.project);
    let out = f
        .user
        .vibe()
        .args([
            "update",
            "--all",
            "--offline",
            "--assume-yes",
            "--reselect-source",
            "--json",
            "--path",
            f.project.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
}

#[test]
fn guarded_leaf_update_removed_answer_and_ordinary_version_movement_remain_distinct() {
    let f = dormant_guard_fixture();
    f.install_ok();
    set_guard_input(&f, false, false);
    let off = update(&f);
    assert!(
        off.status.success(),
        "{}{}",
        String::from_utf8_lossy(&off.stdout),
        String::from_utf8_lossy(&off.stderr)
    );
    assert!(!f.slot(EXTRA).exists());
    set_guard_input(&f, true, false);
    let before = super::workspace::semantic_snapshot(&f.project);
    assert!(!update(&f).status.success());
    assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
    let f = Fixture::new(false);
    f.install_ok();
    let old = f.registry.join(GROUP).join(DRIVER).join("v1.0.0");
    let new = f.registry.join(GROUP).join(DRIVER).join("v1.1.0");
    write(
        &new.join("vibe.toml"),
        fs::read_to_string(old.join("vibe.toml"))
            .unwrap()
            .replace("version='1.0.0'", "version='1.1.0'"),
    );
    write(
        &new.join("boot/rules.md"),
        fs::read(old.join("boot/rules.md")).unwrap(),
    );
    let out = update(&f);
    assert!(
        out.status.success(),
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        f.lock()
            .packages
            .iter()
            .find(|row| row.name.as_str() == DRIVER)
            .unwrap()
            .version
            .to_string(),
        "1.1.0"
    );
}

#[test]
fn weak_forwarding_declaring_parent_edge_four_worlds_and_strong_control() {
    for world in [
        "inactive",
        "independent-root",
        "absent-payload",
        "activated-parent",
        "mandatory-parent",
    ] {
        let f = exact_forward_fixture();
        let driver = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
        let mut declaration = fs::read_to_string(&driver)
            .unwrap()
            .replace("strength = \"strong\"", "strength = \"weak\"");
        if world == "activated-parent" {
            declaration=declaration.replace("effects = [{ kind = \"forward\"", "effects = [{ kind = \"dependency\", target = \"org.vibevm/oracle-extra\" }, { kind = \"forward\"");
        }
        if world == "mandatory-parent" {
            declaration = declaration.replace(
                "version = \"=1.0.0\", optional = true",
                "version = \"=1.0.0\"",
            );
        }
        write(&driver, declaration);
        if world == "independent-root" {
            let request = f.project.join("vibe.toml");
            write(&request,fs::read_to_string(&request).unwrap().replace("[[registry]]",&format!("[requires.packages.'{GROUP}/{EXTRA}']\nversion='=1.0.0'\nlink='static-transitive'\nconfiguration={{policy='require-selection',default_features=false}}\n[[registry]]")));
        }
        if world == "absent-payload" {
            fs::remove_dir_all(f.registry.join(GROUP).join(EXTRA)).unwrap();
        }
        let authored = fs::read(f.project.join("vibe.toml")).unwrap();
        let consumer = Manifest::read(f.project.join("vibe.toml")).unwrap();
        let parent = Manifest::read(&driver).unwrap();
        let optional = parent.requires.optional.get(&coordinate(EXTRA)) == Some(&true);
        assert_eq!(optional, world != "mandatory-parent");
        assert_eq!(
            consumer
                .requires
                .packages
                .iter()
                .any(|row| row.name.as_str() == EXTRA),
            world == "independent-root"
        );
        let selected = parent
            .configuration
            .as_ref()
            .unwrap()
            .features
            .iter()
            .find(|feature| feature.id == "enabled")
            .unwrap();
        assert_eq!(selected.effects.iter().any(|effect|matches!(effect,vibe_core::manifest::ConfigurationEffect::Dependency{target} if target==&coordinate(EXTRA))),world=="activated-parent");
        let source_before = preview_scratch_snapshot(&f.registry);
        let out = f.install();
        let diagnostic = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        println!(
            "weak world={world} status={} diagnostic={diagnostic}",
            out.status
        );
        assert!(out.status.success(), "{world}: {diagnostic}");
        let lock = f.lock();
        let active = matches!(world, "activated-parent" | "mandatory-parent");
        let present = active || world == "independent-root";
        let parent = &lock.configurations[&coordinate(DRIVER)];
        assert_eq!(
            parent.active_dependencies.contains(&coordinate(EXTRA)),
            active
        );
        assert_eq!(lock.meta.schema_version, if active { 14 } else { 8 });
        if present {
            let target = &lock.configurations[&coordinate(EXTRA)];
            assert_eq!(target.forwarded_inputs.len(), usize::from(active));
            assert_eq!(
                target
                    .selected_data(vibe_core::PackageKind::Flow)
                    .unwrap()
                    .contains_key("forwarded"),
                active
            );
            if active {
                assert_eq!(
                    target.forwarded_inputs[0].strength,
                    vibe_core::manifest::ForwardStrength::Weak
                );
                assert_eq!(
                    target.selected_data(vibe_core::PackageKind::Flow).unwrap()["forwarded"],
                    vibe_core::manifest::ParameterValue::Bool(true)
                );
            }
        } else {
            assert!(!lock.configurations.contains_key(&coordinate(EXTRA)));
        }
        assert_eq!(f.slot(EXTRA).exists(), present);
        assert_eq!(f.static_boot().contains(EXTRA_MARKER), present);
        assert_eq!(f.hooks().get(EXTRA).copied().unwrap_or(0) > 0, present);
        assert_eq!(
            preview_scratch_snapshot(&f.registry),
            source_before,
            "{world}"
        );
        let before = super::workspace::semantic_snapshot(&f.project);
        let hooks = f.hooks();
        f.install_ok();
        assert_eq!(
            super::workspace::semantic_snapshot(&f.project),
            before,
            "{world}"
        );
        assert_eq!(f.hooks(), hooks);
        assert_eq!(fs::read(f.project.join("vibe.toml")).unwrap(), authored);
        let infrastructure = preview_scratch_snapshot(&f.project.join(".vibe"));
        println!(
            "weak world={world} coordination paths={:?}",
            infrastructure
                .as_ref()
                .map(|entries| entries.keys().collect::<Vec<_>>())
        );
    }
    let f = exact_forward_fixture();
    let request = fs::read(f.project.join("vibe.toml")).unwrap();
    f.install_ok();
    let lock = f.lock();
    let parent = &lock.configurations[&coordinate(DRIVER)];
    assert!(parent.active_dependencies.contains(&coordinate(EXTRA)));
    let target = &lock.configurations[&coordinate(EXTRA)];
    assert_eq!(target.forwarded_inputs.len(), 1);
    assert_eq!(
        target.selected_data(vibe_core::PackageKind::Flow).unwrap()["forwarded"],
        vibe_core::manifest::ParameterValue::Bool(true)
    );
    assert!(f.slot(EXTRA).exists());
    assert!(f.static_boot().contains(EXTRA_MARKER));
    assert!(f.hooks().get(EXTRA).copied().unwrap_or(0) > 0);
    let before = super::workspace::semantic_snapshot(&f.project);
    let hooks = f.hooks();
    f.install_ok();
    assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
    assert_eq!(f.hooks(), hooks);
    assert_eq!(fs::read(f.project.join("vibe.toml")).unwrap(), request);
}

#[test]
fn weak_forwarding_unknown_active_target_and_tampered_lock_refuse() {
    for mutation in ["unknown-feature", "removed-input", "wrong-strength"] {
        let f = exact_forward_fixture();
        let driver = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
        let text = fs::read_to_string(&driver)
            .unwrap()
            .replace("strength = \"strong\"", "strength = \"weak\"")
            .replace(
                "version = \"=1.0.0\", optional = true",
                "version = \"=1.0.0\"",
            );
        write(
            &driver,
            if mutation == "unknown-feature" {
                text.replace("feature = \"forwarded\"", "feature = \"unknown\"")
            } else {
                text
            },
        );
        if mutation == "unknown-feature" {
            let before = super::workspace::semantic_snapshot(&f.project);
            let out = f.install();
            assert!(!out.status.success());
            assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
            continue;
        }
        // Direct selection independently supplies the effective feature: deleting
        // provenance must still fail rather than silently reproduce the value.
        let request = f.project.join("vibe.toml");
        write(
            &request,
            fs::read_to_string(&request).unwrap()
                + &format!(
                    "\n[requires.packages.'{GROUP}/{EXTRA}']\nversion='=1.0.0'\nlink='static-transitive'\nconfiguration={{policy='require-selection',default_features=false,features=['forwarded']}}\n"
                ),
        );
        f.install_ok();
        let mut lock = f.lock();
        let target = lock.configurations.get_mut(&coordinate(EXTRA)).unwrap();
        match mutation {
            "removed-input" => target.forwarded_inputs.clear(),
            "wrong-strength" => {
                target.forwarded_inputs[0].strength = vibe_core::manifest::ForwardStrength::Strong
            }
            _ => unreachable!(),
        }
        write(
            &f.project.join("vibe.lock"),
            toml::to_string(&lock).unwrap(),
        );
        let before = super::workspace::semantic_snapshot(&f.project);
        assert!(
            Lockfile::read(f.project.join("vibe.lock")).is_err(),
            "{mutation}"
        );
        assert!(!f.install().status.success(), "{mutation}");
        assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
    }
}

#[test]
fn weak_forwarding_prior_reader_writer_preserve_active_dormant_and_unselected() {
    let old = std::env::var_os("NST_WEAK_OLD_CLI")
        .expect("mandatory prior reader/writer gate requires an explicitly mounted pre-weak CLI");
    for world in ["active", "dormant", "independent-root", "unselected"] {
        for operation in ["reader", "writer"] {
            let f = exact_forward_fixture();
            let driver = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
            let mut text = fs::read_to_string(&driver)
                .unwrap()
                .replace("strength = \"strong\"", "strength = \"weak\"");
            if world == "active" {
                text = text.replace(
                    "version = \"=1.0.0\", optional = true",
                    "version = \"=1.0.0\"",
                );
            }
            write(&driver, text);
            let request = f.project.join("vibe.toml");
            if world == "unselected" {
                write(
                    &request,
                    fs::read_to_string(&request)
                        .unwrap()
                        .replace("members=['enabled']", "members=['disabled']"),
                );
            }
            if world == "independent-root" {
                write(
                    &request,
                    fs::read_to_string(&request).unwrap()
                        + &format!(
                            "\n[requires.packages.'{GROUP}/{EXTRA}']\nversion='=1.0.0'\nlink='static-transitive'\nconfiguration={{policy='require-selection',default_features=false}}\n"
                        ),
                );
            }
            f.install_ok();
            let before = super::workspace::semantic_snapshot(&f.project);
            let args = if operation == "reader" {
                vec![
                    "show",
                    "features",
                    "--path",
                    f.project.to_str().unwrap(),
                    "--json",
                ]
            } else {
                vec![
                    "install",
                    "--path",
                    f.project.to_str().unwrap(),
                    "--offline",
                    "--assume-yes",
                    "--json",
                ]
            };
            let out = std::process::Command::new(&old)
                .env_clear()
                .env("HOME", f.root.join("home"))
                .env("VIBE_SETTINGS", &f.user.settings)
                .env("PATH", "/usr/bin:/bin")
                .env("VIBE_NO_DEFAULT_REGISTRY", "1")
                .args(args)
                .output()
                .unwrap();
            let diagnostic = format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            println!(
                "prior weak world={world} operation={operation} status={} diagnostic={diagnostic}",
                out.status
            );
            if world == "unselected" {
                assert!(out.status.success(), "{diagnostic}");
            } else {
                assert!(!out.status.success(), "{world}/{operation}: {diagnostic}");
                assert!(diagnostic.contains("selected cross-package forwarding requires candidate-conditioned incoming consumers") || diagnostic.contains("invalid or duplicate candidate-owned forwarded request"), "{world}/{operation}: {diagnostic}");
            }
            assert_eq!(
                super::workspace::semantic_snapshot(&f.project),
                before,
                "{world}/{operation}"
            );
        }
    }
}

#[test]
fn weak_forwarding_dormant_forgery_and_wrong_declaration_refuse() {
    for mutation in [
        "forged",
        "wrong-declaration",
        "dormant-unknown-feature",
        "activation-after-forward",
    ] {
        let f = exact_forward_fixture();
        let driver = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
        let mut text = fs::read_to_string(&driver)
            .unwrap()
            .replace("strength = \"strong\"", "strength = \"weak\"");
        if mutation == "wrong-declaration" {
            text = text.replace(
                "target = \"org.vibevm/oracle-extra\"",
                "target = \"org.vibevm/not-declared\"",
            );
        } else if mutation == "dormant-unknown-feature" {
            text = text.replace("feature = \"forwarded\"", "feature = \"does-not-exist\"");
        } else if mutation == "activation-after-forward" {
            text = text.replace("strength = \"weak\" }", "strength = \"weak\" }, { kind = \"dependency\", target = \"org.vibevm/oracle-extra\" }");
        }
        write(&driver, text);
        if mutation == "wrong-declaration" {
            let before = super::workspace::semantic_snapshot(&f.project);
            let out = f.install();
            let diagnostic = format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            assert!(!out.status.success(), "{diagnostic}");
            assert!(diagnostic.contains("not-declared"), "{diagnostic}");
            assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
            continue;
        }
        if mutation == "forged" {
            let request = f.project.join("vibe.toml");
            write(
                &request,
                fs::read_to_string(&request).unwrap()
                    + &format!(
                        "\n[requires.packages.'{GROUP}/{EXTRA}']\nversion='=1.0.0'\nlink='static-transitive'\nconfiguration={{policy='require-selection',default_features=false,features=['forwarded']}}\n"
                    ),
            );
        }
        f.install_ok();
        let mut lock = f.lock();
        if mutation == "forged" {
            let parent = lock.find(&GROUP.parse().unwrap(), DRIVER).unwrap();
            let record = &lock.configurations[&coordinate(DRIVER)];
            assert!(!record.active_dependencies.contains(&coordinate(EXTRA)));
            let index = record
                .declarations
                .packages
                .iter()
                .position(|edge| edge.qualified_name() == coordinate(EXTRA))
                .unwrap();
            let input = vibe_core::manifest::ForwardedInput {
                source_group: parent.group.clone(),
                source_name: parent.name.clone(),
                source_version: parent.version.clone(),
                source_content_hash: parent.content_hash.clone(),
                source_feature: "enabled".into(),
                declaration_index: index,
                target: record.declarations.packages[index].clone(),
                target_feature: "forwarded".into(),
                strength: vibe_core::manifest::ForwardStrength::Weak,
            };
            lock.configurations
                .get_mut(&coordinate(EXTRA))
                .unwrap()
                .forwarded_inputs
                .push(input);
            lock.meta.schema_version = 14;
            write(
                &f.project.join("vibe.lock"),
                toml::to_string(&lock).unwrap(),
            );
            let before = super::workspace::semantic_snapshot(&f.project);
            assert!(Lockfile::read(f.project.join("vibe.lock")).is_err());
            assert!(!f.install().status.success());
            assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
        } else {
            assert_eq!(
                f.slot(EXTRA).exists(),
                mutation == "activation-after-forward"
            );
            let before = super::workspace::semantic_snapshot(&f.project);
            f.install_ok();
            assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
        }
    }
}

#[test]
fn zero_or_more_defaults_omitted_empty_matching_and_require_selection_public_boundary() {
    for world in [
        "omitted-default",
        "explicit-empty",
        "explicit-match",
        "omitted-required",
    ] {
        let f = Fixture::new(false);
        let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
        let text = fs::read_to_string(&source)
            .unwrap()
            .replace(
                "cardinality = \"exactly-one\"",
                "cardinality = \"zero-or-more\"",
            )
            .replace("defaults = [\"disabled\"]", "defaults = [\"enabled\"]");
        write(&source, text);
        let request = f.project.join("vibe.toml");
        let input = match world {
            "omitted-default" => {
                "configuration={policy='use-declared-defaults',default_features=false}"
            }
            "explicit-empty" => {
                "configuration={policy='use-declared-defaults',default_features=false,selections={extra={intent='exact',members=[]}}}"
            }
            "explicit-match" => {
                "configuration={policy='use-declared-defaults',default_features=false,selections={extra={intent='exact',members=['enabled']}}}"
            }
            "omitted-required" => {
                "configuration={policy='require-selection',default_features=false}"
            }
            _ => unreachable!(),
        };
        write(&request, fs::read_to_string(&request).unwrap().replace(
            "configuration={policy='require-selection',default_features=false,selections={extra={intent='exact',members=['disabled']}}}", input));
        let authored = fs::read(&request).unwrap();
        let consumer = Manifest::read(&request).unwrap();
        let original = &consumer.requires.configurations[&coordinate(DRIVER)];
        assert!(!original.default_features);
        let definition = Manifest::read(&source).unwrap().configuration.unwrap();
        let normalized = definition.normalize_input(original, "./vibe.toml").unwrap();
        match world {
            "explicit-empty" => assert!(
                matches!(&normalized.choices.answers["extra"].selection, vibe_core::configuration::Selection::Explicit(values) if values.is_empty())
            ),
            "explicit-match" => assert!(
                matches!(&normalized.choices.answers["extra"].selection, vibe_core::configuration::Selection::Explicit(values) if values == &vec!["enabled".to_owned()])
            ),
            _ => assert!(!normalized.choices.answers.contains_key("extra")),
        }
        let evaluated = definition
            .compile_choices()
            .unwrap()
            .evaluate(&normalized.features, &[normalized.choices]);
        println!(
            "zero-or-more world={world} pure outcome={:?}",
            evaluated.outcome
        );
        let vibe_core::configuration::ChoiceOutcome::Resolved { features } = &evaluated.outcome
        else {
            panic!("{world}: unresolved pure outcome");
        };
        assert_eq!(
            features.selected.contains(&"enabled".to_owned()),
            matches!(world, "omitted-default" | "explicit-match")
        );
        assert_eq!(!evaluated.defaults.is_empty(), world == "omitted-default");
        let source_before = preview_scratch_snapshot(&f.registry);
        let out = f.install();
        let diagnostic = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        println!(
            "zero-or-more world={world} status={} diagnostic={diagnostic}",
            out.status
        );
        {
            assert!(out.status.success(), "{world}: {diagnostic}");
            let lock = f.lock();
            let record = &lock.configurations[&coordinate(DRIVER)];
            assert_eq!(record.consumers.len(), 1);
            assert_eq!(record.consumers.values().next().unwrap(), original);
            if world == "omitted-default" {
                let basis = record.automatic_basis.as_ref().unwrap();
                assert!(basis.consumer_choices.is_empty());
                assert_eq!(
                    basis.consumer_choice_sets.values().next().unwrap()["extra"],
                    vec!["enabled"]
                );
                assert_eq!(lock.meta.schema_version, 15);
            } else {
                assert!(record.automatic_basis.is_none());
                assert_eq!(lock.meta.schema_version, 8);
            }
            let selected = matches!(world, "omitted-default" | "explicit-match");
            assert_eq!(record.features.contains(&"enabled".to_owned()), selected);
            assert_eq!(
                record.active_dependencies.contains(&coordinate(EXTRA)),
                selected
            );
            assert_eq!(f.slot(EXTRA).exists(), selected);
            assert_eq!(f.static_boot().contains(EXTRA_MARKER), selected);
            assert_eq!(f.hooks().get(EXTRA).copied().unwrap_or(0) > 0, selected);
            let installed = super::workspace::semantic_snapshot(&f.project);
            let hooks = f.hooks();
            f.install_ok();
            assert_eq!(super::workspace::semantic_snapshot(&f.project), installed);
            assert_eq!(f.hooks(), hooks);
        }
        assert_eq!(fs::read(&request).unwrap(), authored);
        assert_eq!(preview_scratch_snapshot(&f.registry), source_before);
        println!(
            "zero-or-more world={world} coordination paths={:?}",
            preview_scratch_snapshot(&f.project.join(".vibe"))
                .as_ref()
                .map(|entries| entries.keys().collect::<Vec<_>>())
        );
    }
}

fn multi_default_fixture(defaults: &str) -> Fixture {
    let f = Fixture::new(false);
    configure(&f, true);
    let path = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
    write(
        &path,
        fs::read_to_string(&path)
            .unwrap()
            .replace(
                "cardinality = \"exactly-one\"",
                "cardinality = \"zero-or-more\"",
            )
            .replace(
                "defaults = [\"enabled\"]",
                &format!("defaults = {defaults}"),
            ),
    );
    f
}

#[test]
fn zero_or_more_multi_default_basis_is_lossless_and_tamper_refuses() {
    for mutation in [
        "none",
        "empty-declared",
        "duplicate",
        "missing",
        "unknown",
        "stale",
        "old-epoch",
    ] {
        let f = multi_default_fixture(if mutation == "empty-declared" {
            "[]"
        } else {
            "[\"enabled\",\"disabled\"]"
        });
        let original = fs::read(f.project.join("vibe.toml")).unwrap();
        f.install_ok();
        let mut lock = f.lock();
        assert_eq!(lock.meta.schema_version, 15);
        let record = lock.configurations.get_mut(&coordinate(DRIVER)).unwrap();
        assert!(
            record
                .consumers
                .values()
                .all(|input| input.selections.is_empty())
        );
        let basis = record.automatic_basis.as_mut().unwrap();
        assert!(basis.consumer_choices.is_empty());
        let members = basis
            .consumer_choice_sets
            .values_mut()
            .next()
            .unwrap()
            .get_mut("extra")
            .unwrap();
        assert_eq!(
            *members,
            if mutation == "empty-declared" {
                Vec::<String>::new()
            } else {
                vec!["disabled".to_owned(), "enabled".to_owned()]
            }
        );
        match mutation {
            "duplicate" => members.push("enabled".into()),
            "missing" => {
                members.pop();
            }
            "unknown" => members.push("unknown".into()),
            "stale" => basis.definition_fingerprint = "stale".into(),
            "old-epoch" => lock.meta.schema_version = 14,
            _ => {}
        }
        if matches!(mutation, "none" | "empty-declared") {
            assert_eq!(f.slot(EXTRA).exists(), mutation == "none");
            let before = super::workspace::semantic_snapshot(&f.project);
            f.install_ok();
            assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
        } else {
            write(
                &f.project.join("vibe.lock"),
                toml::to_string(&lock).unwrap(),
            );
            let before = super::workspace::semantic_snapshot(&f.project);
            assert!(
                Lockfile::read(f.project.join("vibe.lock")).is_err(),
                "{mutation}"
            );
            assert!(!f.install().status.success());
            assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
        }
        assert_eq!(fs::read(f.project.join("vibe.toml")).unwrap(), original);
    }
}

#[test]
fn zero_or_more_unselected_version_leaves_no_basis_and_changed_defaults_refuse_update() {
    for changed in [false, true] {
        let f = multi_default_fixture("[\"enabled\"]");
        let old = f.registry.join(GROUP).join(DRIVER).join("v1.0.0");
        let next = f.registry.join(GROUP).join(DRIVER).join("v1.1.0");
        write(
            &next.join("vibe.toml"),
            fs::read_to_string(old.join("vibe.toml"))
                .unwrap()
                .replace("version='1.0.0'", "version='1.1.0'")
                .replace("defaults = [\"enabled\"]", "defaults = [\"disabled\"]"),
        );
        write(&next.join("boot/rules.md"), "UNSELECTED_VERSION_MARKER\n");
        let request = f.project.join("vibe.toml");
        write(
            &request,
            fs::read_to_string(&request)
                .unwrap()
                .replace("version='^1'", "version='=1.0.0'"),
        );
        f.install_ok();
        let lock = f.lock();
        assert_eq!(
            lock.find(&GROUP.parse().unwrap(), DRIVER)
                .unwrap()
                .version
                .to_string(),
            "1.0.0"
        );
        assert_eq!(
            lock.configurations[&coordinate(DRIVER)]
                .automatic_basis
                .as_ref()
                .unwrap()
                .consumer_choice_sets
                .values()
                .next()
                .unwrap()["extra"],
            vec!["enabled"]
        );
        assert!(!f.static_boot().contains("UNSELECTED_VERSION_MARKER"));
        assert!(f.slot(EXTRA).exists());
        if changed {
            write(
                &request,
                fs::read_to_string(&request)
                    .unwrap()
                    .replace("version='=1.0.0'", "version='^1'"),
            );
            let before = super::workspace::semantic_snapshot(&f.project);
            let out = update(&f);
            let diagnostic = format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            assert!(!out.status.success(), "{diagnostic}");
            assert!(
                diagnostic.contains("multi-member automatic default renewal is not connected"),
                "{diagnostic}"
            );
            assert!(!diagnostic.contains("package-default-review/1:sha256:"));
            assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
            let scalar = review_fixture();
            let shown = update(&scalar);
            let proposal: serde_json::Value = serde_json::from_slice(&shown.stderr).unwrap();
            let token = proposal["proposal"]["review_id"].as_str().unwrap();
            let rejected = respond(&f, token);
            assert!(!rejected.status.success());
            let diagnostic = format!(
                "{}{}",
                String::from_utf8_lossy(&rejected.stdout),
                String::from_utf8_lossy(&rejected.stderr)
            );
            assert!(
                diagnostic.contains("multi-member automatic default renewal is not connected"),
                "{diagnostic}"
            );
            assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
        } else {
            let before = super::workspace::semantic_snapshot(&f.project);
            f.install_ok();
            assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
        }
    }
}

#[test]
fn multi_member_defaults_exact_version_drift_and_explicit_equal_answer() {
    for explicit in [false, true] {
        let f = multi_default_fixture("[\"enabled\",\"disabled\"]");
        let request = f.project.join("vibe.toml");
        let mut authored = fs::read_to_string(&request)
            .unwrap()
            .replace("version='^1'", "version='=1.0.0'");
        if explicit {
            authored = authored.replace(
                "configuration={policy='use-declared-defaults',default_features=false}",
                "configuration={policy='require-selection',default_features=false,selections={extra={intent='exact',members=['disabled','enabled']}}}",
            );
        }
        write(&request, &authored);
        f.install_ok();
        let old = f.lock();
        let original = old.configurations[&coordinate(DRIVER)].clone();
        assert_eq!(original.automatic_basis.is_none(), explicit);
        if !explicit {
            assert_eq!(
                original
                    .automatic_basis
                    .as_ref()
                    .unwrap()
                    .consumer_choice_sets["./vibe.toml"]["extra"],
                vec!["disabled", "enabled"]
            );
            assert!(original.consumers["./vibe.toml"].selections.is_empty());
        }
        assert!(f.slot(EXTRA).exists());
        // A separate CLI process with the registry unavailable must replay held bytes.
        let hidden = f.root.join("registry-unavailable");
        fs::rename(&f.registry, &hidden).unwrap();
        let before = super::workspace::semantic_snapshot(&f.project);
        f.install_ok();
        assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
        fs::rename(&hidden, &f.registry).unwrap();
        let prior = f.registry.join(GROUP).join(DRIVER).join("v1.0.0");
        let next = f.registry.join(GROUP).join(DRIVER).join("v1.1.0");
        write(
            &next.join("vibe.toml"),
            fs::read_to_string(prior.join("vibe.toml"))
                .unwrap()
                .replace("version='1.0.0'", "version='1.1.0'")
                .replace(
                    "defaults = [\"enabled\",\"disabled\"]",
                    "defaults = [\"disabled\"]",
                ),
        );
        write(&next.join("boot/rules.md"), "NEW_EXACT_SET_VERSION\n");
        write(&next.join("unused-source.txt"), UNUSED);
        let changed = authored.replace("version='=1.0.0'", "version='=1.1.0'");
        assert_ne!(changed, authored);
        write(&request, &changed);
        let before = super::workspace::semantic_snapshot(&f.project);
        let out = update(&f);
        let diagnostic = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(out.status.success(), explicit, "{diagnostic}");
        if explicit {
            let next = f.lock();
            assert_eq!(
                next.find(&GROUP.parse().unwrap(), DRIVER)
                    .unwrap()
                    .version
                    .to_string(),
                "1.1.0"
            );
            let record = &next.configurations[&coordinate(DRIVER)];
            assert_eq!(record.consumers, original.consumers);
            assert!(record.automatic_basis.is_none());
            assert_eq!(record.features, original.features);
            assert!(f.slot(EXTRA).exists());
            assert!(f.static_boot().contains("NEW_EXACT_SET_VERSION"));
        } else {
            assert!(
                diagnostic.contains("multi-member automatic default renewal is not connected"),
                "{diagnostic}"
            );
            assert!(!diagnostic.contains("package-default-review/1:sha256:"));
            assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
            assert_eq!(f.lock().configurations[&coordinate(DRIVER)], original);
            assert!(!f.static_boot().contains("NEW_EXACT_SET_VERSION"));
        }
        assert_eq!(fs::read_to_string(&request).unwrap(), changed);
        println!(
            "multi-set explicit={explicit} cold=registry-unavailable exact-next=1.1.0 success={} originals-preserved=true",
            out.status.success()
        );
    }
}

fn set_review_fixture(explicit: bool) -> Fixture {
    let f = multi_default_fixture("[\"enabled\",\"disabled\"]");
    let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
    let text = fs::read_to_string(&source).unwrap().replace(
        "effects = [{ kind = \"dependency\", target = \"org.vibevm/oracle-extra\" }]",
        "effects = [{kind='config-value',name='enabled'}]",
    );
    write(
        &source,
        text + "\n[configuration.config_values]\nenabled=true\n",
    );
    let request = f.project.join("vibe.toml");
    let mut text = fs::read_to_string(&request)
        .unwrap()
        .replace("version='^1'", "version='=1.0.0'");
    if explicit {
        text = text.replace("configuration={policy='use-declared-defaults',default_features=false}", "configuration={policy='require-selection',default_features=false,selections={extra={intent='exact',members=['disabled','enabled']}}}");
    }
    write(&request, text);
    f.install_ok();
    let next = f.registry.join(GROUP).join(DRIVER).join("v1.1.0");
    write(
        &next.join("vibe.toml"),
        fs::read_to_string(&source)
            .unwrap()
            .replace("version='1.0.0'", "version='1.1.0'")
            .replace(
                "defaults = [\"enabled\",\"disabled\"]",
                "defaults = [\"disabled\"]",
            ),
    );
    write(&next.join("boot/rules.md"), "SET_REVIEW_NEW_VERSION\n");
    write(&next.join("unused-source.txt"), UNUSED);
    write(
        &request,
        fs::read_to_string(&request)
            .unwrap()
            .replace("version='=1.0.0'", "version='=1.1.0'"),
    );
    f
}

#[test]
fn member_set_v2_review_actual_preview_apply_and_refusals() {
    for world in [
        "apply",
        "explicit",
        "forged",
        "cross-project",
        "payload-drift",
        "definition-drift",
        "scalar",
        "reverse-scalar",
        "malformed",
        "prior-drift",
        "graph-drift",
    ] {
        let f = set_review_fixture(world == "explicit");
        let old = f.lock();
        let original = old.configurations[&coordinate(DRIVER)].consumers.clone();
        let values = |f: &Fixture| {
            let out = f
                .user
                .vibe()
                .args([
                    "show",
                    "configuration-data",
                    "--path",
                    f.project.to_str().unwrap(),
                    "--json",
                ])
                .output()
                .unwrap();
            assert!(
                out.status.success(),
                "{}",
                String::from_utf8_lossy(&out.stderr)
            );
            let data: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
            data["packages"]
                .as_array()
                .unwrap()
                .iter()
                .find(|p| p["package"] == coordinate(DRIVER))
                .expect("actual selected driver config-data row")["values"]
                .clone()
        };
        assert_eq!(values(&f)["enabled"], true);
        let hooks = f.hooks();
        let before = super::workspace::semantic_snapshot(&f.project);
        let out = update(&f);
        if world == "explicit" {
            assert!(
                out.status.success(),
                "{}",
                String::from_utf8_lossy(&out.stderr)
            );
            assert!(
                f.lock().configurations[&coordinate(DRIVER)]
                    .automatic_basis
                    .is_none()
            );
            assert_eq!(
                f.lock().configurations[&coordinate(DRIVER)].consumers,
                original
            );
            assert_eq!(values(&f)["enabled"], true);
            assert_eq!(f.hooks(), hooks);
            continue;
        }
        assert!(!out.status.success());
        assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
        let shown: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
        let proposal = &shown["proposal"];
        assert_eq!(proposal["schema"], 2);
        assert_eq!(proposal["protocol"], "vibe-package-member-set-review/2");
        assert_eq!(proposal["consumer"], "./vibe.toml");
        assert_eq!(proposal["group"], "extra");
        assert_eq!(proposal["from_version"], "1.0.0");
        assert_eq!(proposal["to_version"], "1.1.0");
        assert_eq!(proposal["acknowledgement"], proposal["review_id"]);
        assert_ne!(proposal["from_content_pin"], proposal["to_content_pin"]);
        assert_eq!(
            proposal["from_members"],
            serde_json::json!(["disabled", "enabled"])
        );
        assert_eq!(proposal["to_members"], serde_json::json!(["disabled"]));
        let mut token = proposal["review_id"].as_str().unwrap().to_owned();
        assert!(token.starts_with("package-member-set-review/2:sha256:"));
        if world == "reverse-scalar" {
            let scalar = review_fixture();
            let before = super::workspace::semantic_snapshot(&scalar.project);
            assert!(!respond(&scalar, &token).status.success());
            assert_eq!(super::workspace::semantic_snapshot(&scalar.project), before);
            continue;
        }
        match world {
            "malformed" => token = "package-member-set-review/2:sha256:bad".into(),
            "prior-drift" => {
                let lock = f.project.join("vibe.lock");
                write(
                    &lock,
                    fs::read_to_string(&lock).unwrap() + "\n# changed retained subject\n",
                );
            }
            "forged" => token = format!("package-member-set-review/2:sha256:{}", "0".repeat(64)),
            "cross-project" => {
                let other = set_review_fixture(false);
                let out = update(&other);
                let p: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
                token = p["proposal"]["review_id"].as_str().unwrap().to_owned();
            }
            "payload-drift" => write(
                &f.registry
                    .join(GROUP)
                    .join(DRIVER)
                    .join("v1.1.0/unused-source.txt"),
                "CHANGED_AFTER_PREVIEW\n",
            ),
            "definition-drift" => {
                let path = f.registry.join(GROUP).join(DRIVER).join("v1.1.0/vibe.toml");
                write(
                    &path,
                    fs::read_to_string(&path)
                        .unwrap()
                        .replace("enabled=true", "enabled=false"),
                );
            }
            "scalar" => {
                let scalar = review_fixture();
                let out = update(&scalar);
                let p: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
                token = p["proposal"]["review_id"].as_str().unwrap().to_owned();
            }
            "graph-drift" => {
                let path = f.registry.join(GROUP).join(DRIVER).join("v1.1.0/vibe.toml");
                write(
                    &path,
                    fs::read_to_string(&path)
                        .unwrap()
                        .replace("optional = true", "optional = false"),
                );
            }
            _ => {}
        }
        let refusal_before = if world == "prior-drift" {
            super::workspace::semantic_snapshot(&f.project)
        } else {
            before
        };
        let out = respond(&f, &token);
        assert_eq!(
            out.status.success(),
            world == "apply",
            "world={world} {}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        if world == "apply" {
            let lock = f.lock();
            let record = &lock.configurations[&coordinate(DRIVER)];
            assert_eq!(record.consumers, original);
            assert_eq!(
                record
                    .automatic_basis
                    .as_ref()
                    .unwrap()
                    .consumer_choice_sets["./vibe.toml"]["extra"],
                vec!["disabled"]
            );
            assert_eq!(
                lock.find(&GROUP.parse().unwrap(), DRIVER)
                    .unwrap()
                    .version
                    .to_string(),
                "1.1.0"
            );
            assert!(!f.slot(EXTRA).exists());
            assert_eq!(values(&f), serde_json::json!({}));
            assert_eq!(f.hooks(), hooks);
            assert!(f.static_boot().contains("SET_REVIEW_NEW_VERSION"));
            let after = super::workspace::semantic_snapshot(&f.project);
            assert!(!respond(&f, &token).status.success());
            assert_eq!(super::workspace::semantic_snapshot(&f.project), after);
            fs::rename(&f.registry, f.root.join("registry-unavailable")).unwrap();
            f.install_ok();
            assert_eq!(super::workspace::semantic_snapshot(&f.project), after);
        } else {
            assert_eq!(
                super::workspace::semantic_snapshot(&f.project),
                refusal_before
            );
        }
        println!("set-review-v2 world={world} actual-status={}", out.status);
    }
}

#[test]
fn zero_or_more_prior_reader_writer_refuse_new_basis_and_preserve_scalar_bytes() {
    let old = std::env::var_os("NST_MULTI_OLD_CLI")
        .expect("mandatory multi-default prior CLI must be explicitly mounted");
    for world in ["multi", "empty", "scalar"] {
        for operation in ["reader", "writer"] {
            let f = if world == "scalar" {
                let f = Fixture::new(false);
                configure(&f, true);
                f
            } else {
                multi_default_fixture(if world == "empty" {
                    "[]"
                } else {
                    "[\"enabled\",\"disabled\"]"
                })
            };
            f.install_ok();
            let lock = fs::read_to_string(f.project.join("vibe.lock")).unwrap();
            if world == "scalar" {
                assert!(!lock.contains("consumer_choice_sets"));
                assert_eq!(f.lock().meta.schema_version, 12);
            } else {
                assert!(lock.contains("consumer_choice_sets"));
                assert_eq!(f.lock().meta.schema_version, 15);
            }
            let before = super::workspace::semantic_snapshot(&f.project);
            let args = if operation == "reader" {
                vec![
                    "show",
                    "features",
                    "--path",
                    f.project.to_str().unwrap(),
                    "--json",
                ]
            } else {
                vec![
                    "install",
                    "--path",
                    f.project.to_str().unwrap(),
                    "--offline",
                    "--assume-yes",
                    "--json",
                ]
            };
            let out = std::process::Command::new(&old)
                .env_clear()
                .env("HOME", f.root.join("home"))
                .env("VIBE_SETTINGS", &f.user.settings)
                .env("VIBE_NO_DEFAULT_REGISTRY", "1")
                .env("PATH", "/usr/bin:/bin")
                .args(args)
                .output()
                .unwrap();
            let diagnostic = format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            println!(
                "prior multi world={world} operation={operation} status={} diagnostic={diagnostic}",
                out.status
            );
            if world == "scalar" {
                assert!(out.status.success(), "{diagnostic}");
            } else {
                assert!(!out.status.success(), "{diagnostic}");
                assert!(
                    diagnostic.contains("consumer_choice_sets") || diagnostic.contains("schema"),
                    "{diagnostic}"
                );
            }
            assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
        }
    }
}

#[test]
fn zero_or_more_absent_default_and_guarded_automatic_remain_closed() {
    for world in ["absent", "guarded"] {
        let f = multi_default_fixture("[]");
        let path = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
        let text = fs::read_to_string(&path).unwrap();
        write(
            &path,
            if world == "absent" {
                text.replace("defaults = []", "")
            } else {
                text + "\nguards=[{member='gate-on'}]\n[[configuration.features]]\nid='gate-on'\nname='Gate on'\n[[configuration.groups]]\nmode='choice'\nid='gate'\nname='Gate'\ncardinality='exactly-one'\nmembers=['gate-on']\ndefaults=['gate-on']\n"
            },
        );
        let before = super::workspace::semantic_snapshot(&f.project);
        let out = f.install();
        let diagnostic = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(!out.status.success(), "{world}: {diagnostic}");
        assert!(
            diagnostic.contains(if world == "absent" {
                "automatic choice needs a declared exactly-one default"
            } else {
                "only one unguarded non-scalar choice default group"
            }),
            "{world}: {diagnostic}"
        );
        assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
    }
}

#[test]
fn remaining_choice_cardinalities_omitted_empty_one_two_public_qualification() {
    for cardinality in ["zero-or-one", "one-or-more"] {
        for world in [
            "omitted-default",
            "omitted-required",
            "explicit-empty",
            "explicit-one",
            "explicit-two",
        ] {
            let f = Fixture::new(false);
            let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
            let defaults = if cardinality == "zero-or-one" {
                "[\"enabled\"]"
            } else {
                "[\"enabled\",\"disabled\"]"
            };
            let text = fs::read_to_string(&source)
                .unwrap()
                .replace(
                    "cardinality = \"exactly-one\"",
                    &format!("cardinality = \"{cardinality}\""),
                )
                .replace(
                    "defaults = [\"disabled\"]",
                    &format!("defaults = {defaults}"),
                )
                + "\n[configuration.config_values]\nenabled=true\ndisabled=false\n";
            write(&source, text.replace("effects = [{ kind = \"dependency\", target = \"org.vibevm/oracle-extra\" }]", "effects = [{ kind = \"dependency\", target = \"org.vibevm/oracle-extra\" }, {kind='config-value',name='enabled'}]")
                .replace("name = \"No extra\"", "name = \"No extra\"\neffects=[{kind='config-value',name='disabled'}]"));
            let answer = match world {
                "omitted-default" => "policy='use-declared-defaults',default_features=false",
                "omitted-required" => "policy='require-selection',default_features=false",
                "explicit-empty" => {
                    "policy='require-selection',default_features=false,selections={extra={intent='exact',members=[]}}"
                }
                "explicit-one" => {
                    "policy='require-selection',default_features=false,selections={extra={intent='exact',members=['enabled']}}"
                }
                "explicit-two" => {
                    "policy='require-selection',default_features=false,selections={extra={intent='exact',members=['enabled','disabled']}}"
                }
                _ => unreachable!(),
            };
            let request = f.project.join("vibe.toml");
            write(&request, fs::read_to_string(&request).unwrap().replace(
                "configuration={policy='require-selection',default_features=false,selections={extra={intent='exact',members=['disabled']}}}", &format!("configuration={{{answer}}}")));
            let original = Manifest::read(&request).unwrap().requires.configurations
                [&coordinate(DRIVER)]
                .clone();
            let definition = Manifest::read(&source).unwrap().configuration.unwrap();
            let normalized = definition
                .normalize_input(&original, "./vibe.toml")
                .unwrap();
            if world.starts_with("omitted") {
                assert!(!normalized.choices.answers.contains_key("extra"));
            } else {
                assert!(
                    matches!(&normalized.choices.answers["extra"].selection, vibe_core::configuration::Selection::Explicit(values) if values.len() == match world {"explicit-empty"=>0,"explicit-one"=>1,_=>2})
                );
            }
            let pure = definition
                .compile_choices()
                .unwrap()
                .evaluate(&normalized.features, &[normalized.choices]);
            println!("cardinality={cardinality} world={world} pure={pure:?}");
            let kernel_resolves = world == "omitted-default"
                || world == "explicit-one"
                || (cardinality == "zero-or-one"
                    && matches!(world, "omitted-required" | "explicit-empty"))
                || (cardinality == "one-or-more" && world == "explicit-two");
            assert_eq!(
                matches!(
                    pure.outcome,
                    vibe_core::configuration::ChoiceOutcome::Resolved { .. }
                ),
                kernel_resolves
            );
            assert_eq!(!pure.defaults.is_empty(), world == "omitted-default");
            assert_eq!(
                !pure.questions.is_empty(),
                cardinality == "one-or-more"
                    && matches!(world, "omitted-required" | "explicit-empty")
            );
            let succeeds = world == "omitted-default"
                || world == "explicit-one"
                || (cardinality == "zero-or-one"
                    && matches!(world, "omitted-required" | "explicit-empty"))
                || (cardinality == "one-or-more" && world == "explicit-two");
            let before = super::workspace::semantic_snapshot(&f.project);
            let source_before = preview_scratch_snapshot(&f.registry);
            let authored = fs::read(&request).unwrap();
            let out = f.install();
            let diagnostic = format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            println!(
                "cardinality={cardinality} world={world} status={} diagnostic={diagnostic}",
                out.status
            );
            if succeeds {
                assert!(out.status.success(), "{diagnostic}");
                let lock = f.lock();
                let record = &lock.configurations[&coordinate(DRIVER)];
                assert_eq!(record.consumers.values().next().unwrap(), &original);
                if world == "omitted-default" {
                    let basis = record.automatic_basis.as_ref().unwrap();
                    assert!(basis.consumer_choices.is_empty());
                    let expected = if cardinality == "one-or-more" {
                        vec!["disabled", "enabled"]
                    } else {
                        vec!["enabled"]
                    };
                    assert_eq!(
                        basis.consumer_choice_sets.values().next().unwrap()["extra"],
                        expected
                    );
                    assert_eq!(lock.meta.schema_version, 15);
                } else {
                    assert!(record.automatic_basis.is_none());
                    assert_eq!(lock.meta.schema_version, 8);
                }
                let selected = matches!(world, "omitted-default" | "explicit-one" | "explicit-two");
                let data = record.selected_data(vibe_core::PackageKind::Flow).unwrap();
                assert_eq!(data.contains_key("enabled"), selected);
                assert_eq!(
                    data.contains_key("disabled"),
                    world == "explicit-two"
                        || (world == "omitted-default" && cardinality == "one-or-more")
                );
                assert_eq!(
                    record.active_dependencies.contains(&coordinate(EXTRA)),
                    selected
                );
                assert_eq!(f.slot(EXTRA).exists(), selected);
                assert_eq!(f.static_boot().contains(EXTRA_MARKER), selected);
                assert_eq!(f.hooks().get(EXTRA).copied().unwrap_or(0) > 0, selected);
                let installed = super::workspace::semantic_snapshot(&f.project);
                f.install_ok();
                assert_eq!(super::workspace::semantic_snapshot(&f.project), installed);
            } else {
                assert!(!out.status.success(), "{diagnostic}");
                if world == "omitted-default" {
                    assert!(
                        diagnostic.contains(
                            "only unguarded exactly-one single-member defaults are connected"
                        ),
                        "{diagnostic}"
                    );
                } else if cardinality == "one-or-more"
                    && matches!(world, "omitted-required" | "explicit-empty")
                {
                    assert!(
                        diagnostic.contains("requires configuration answers")
                            && diagnostic.contains("OneOrMore"),
                        "{diagnostic}"
                    );
                } else {
                    assert!(
                        diagnostic.contains("conflicts=[Cardinality")
                            || diagnostic.contains("invalid answers=["),
                        "{diagnostic}"
                    );
                }
                assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
                assert!(!f.project.join("vibe.lock").exists());
                assert!(!f.project.join("effects.log").exists());
                for name in [DRIVER, BASE, EXTRA] {
                    assert!(!f.slot(name).exists());
                }
            }
            assert_eq!(fs::read(&request).unwrap(), authored);
            assert_eq!(preview_scratch_snapshot(&f.registry), source_before);
            println!(
                "cardinality={cardinality} world={world} coordination={:?}",
                preview_scratch_snapshot(&f.project.join(".vibe"))
                    .as_ref()
                    .map(|entries| entries.keys().collect::<Vec<_>>())
            );
        }
    }
}

fn remaining_default_fixture(cardinality: &str, defaults: &str) -> Fixture {
    let f = multi_default_fixture(defaults);
    let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
    write(
        &source,
        fs::read_to_string(&source).unwrap().replace(
            "cardinality = \"zero-or-more\"",
            &format!("cardinality = \"{cardinality}\""),
        ),
    );
    let request = f.project.join("vibe.toml");
    write(
        &request,
        fs::read_to_string(&request)
            .unwrap()
            .replace("version='^1'", "version='=1.0.0'"),
    );
    f
}

#[test]
fn remaining_defaults_optional_empty_absent_matching_and_tamper() {
    for cardinality in ["zero-or-one", "one-or-more"] {
        for world in [
            "normal",
            "empty",
            "absent",
            "matching",
            "missing",
            "extra",
            "wrong-consumer",
            "wrong-members",
            "stale",
        ] {
            let defaults = if world == "empty" {
                "[]"
            } else if cardinality == "zero-or-one" {
                "[\"enabled\"]"
            } else {
                "[\"enabled\",\"disabled\"]"
            };
            let f = remaining_default_fixture(cardinality, defaults);
            let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
            if world == "absent" {
                write(
                    &source,
                    fs::read_to_string(&source)
                        .unwrap()
                        .replace(&format!("defaults = {defaults}"), ""),
                );
            }
            if world == "matching" {
                let request = f.project.join("vibe.toml");
                let members = if cardinality == "zero-or-one" {
                    "['enabled']"
                } else {
                    "['enabled','disabled']"
                };
                write(&request,fs::read_to_string(&request).unwrap().replace("configuration={policy='use-declared-defaults',default_features=false}",&format!("configuration={{policy='use-declared-defaults',default_features=false,selections={{extra={{intent='exact',members={members}}}}}}}")));
            }
            let authored = fs::read(f.project.join("vibe.toml")).unwrap();
            let before = super::workspace::semantic_snapshot(&f.project);
            let out = f.install();
            if world == "absent" || (world == "empty" && cardinality == "one-or-more") {
                assert!(!out.status.success());
                assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
                continue;
            }
            assert!(
                out.status.success(),
                "{cardinality}/{world}: {}",
                String::from_utf8_lossy(&out.stderr)
            );
            let mut lock = f.lock();
            let record = lock.configurations.get_mut(&coordinate(DRIVER)).unwrap();
            if world == "matching" {
                assert!(record.automatic_basis.is_none());
                assert_eq!(lock.meta.schema_version, 8);
            } else {
                let basis = record.automatic_basis.as_mut().unwrap();
                match world {
                    "missing" => {
                        basis.consumer_choice_sets.clear();
                    }
                    "extra" => {
                        basis
                            .consumer_choice_sets
                            .values_mut()
                            .next()
                            .unwrap()
                            .insert("not-a-group".into(), vec![]);
                    }
                    "wrong-consumer" => {
                        let value = basis.consumer_choice_sets.pop_first().unwrap().1;
                        basis
                            .consumer_choice_sets
                            .insert("./foreign.toml".into(), value);
                    }
                    "wrong-members" => {
                        basis
                            .consumer_choice_sets
                            .values_mut()
                            .next()
                            .unwrap()
                            .insert(
                                "extra".into(),
                                vec!["disabled".into(), "enabled".into(), "unknown".into()],
                            );
                    }
                    "stale" => basis.definition_fingerprint = "stale".into(),
                    _ => {}
                }
            }
            if matches!(
                world,
                "missing" | "extra" | "wrong-consumer" | "wrong-members" | "stale"
            ) {
                write(
                    &f.project.join("vibe.lock"),
                    toml::to_string(&lock).unwrap(),
                );
                let before = super::workspace::semantic_snapshot(&f.project);
                assert!(Lockfile::read(f.project.join("vibe.lock")).is_err());
                assert!(!f.install().status.success());
                assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
            } else {
                assert_eq!(f.slot(EXTRA).exists(), world != "empty");
                let before = super::workspace::semantic_snapshot(&f.project);
                f.install_ok();
                assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
            }
            assert_eq!(fs::read(f.project.join("vibe.toml")).unwrap(), authored);
        }
    }
}

#[test]
fn remaining_defaults_prior_reader_writer_refuse_without_loss() {
    let old = std::env::var_os("NST_REMAINING_OLD_CLI")
        .expect("mandatory explicitly mounted pre-extension CLI");
    for cardinality in ["zero-or-one", "one-or-more"] {
        for empty in [false, true] {
            if empty && cardinality == "one-or-more" {
                continue;
            }
            for operation in ["reader", "writer"] {
                let defaults = if empty {
                    "[]"
                } else if cardinality == "zero-or-one" {
                    "[\"enabled\"]"
                } else {
                    "[\"enabled\",\"disabled\"]"
                };
                let f = remaining_default_fixture(cardinality, defaults);
                f.install_ok();
                let before = super::workspace::semantic_snapshot(&f.project);
                let args = if operation == "reader" {
                    vec![
                        "show",
                        "features",
                        "--path",
                        f.project.to_str().unwrap(),
                        "--json",
                    ]
                } else {
                    vec![
                        "install",
                        "--path",
                        f.project.to_str().unwrap(),
                        "--offline",
                        "--assume-yes",
                        "--json",
                    ]
                };
                let out = std::process::Command::new(&old)
                    .env_clear()
                    .env("HOME", f.root.join("home"))
                    .env("VIBE_SETTINGS", &f.user.settings)
                    .env("VIBE_NO_DEFAULT_REGISTRY", "1")
                    .env("PATH", "/usr/bin:/bin")
                    .args(args)
                    .output()
                    .unwrap();
                let diagnostic = format!(
                    "{}{}",
                    String::from_utf8_lossy(&out.stdout),
                    String::from_utf8_lossy(&out.stderr)
                );
                println!(
                    "prior remaining {cardinality} empty={empty} {operation} status={} diagnostic={diagnostic}",
                    out.status
                );
                assert!(!out.status.success(), "{diagnostic}");
                assert!(
                    diagnostic.contains(
                        "only unguarded exactly-one single-member defaults are connected"
                    ),
                    "{diagnostic}"
                );
                assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
            }
        }
    }
}

#[test]
fn remaining_defaults_nonselected_candidate_and_update_drift_refuse() {
    for cardinality in ["zero-or-one", "one-or-more"] {
        let f = remaining_default_fixture(
            cardinality,
            if cardinality == "zero-or-one" {
                "[\"enabled\"]"
            } else {
                "[\"enabled\",\"disabled\"]"
            },
        );
        let old = f.registry.join(GROUP).join(DRIVER).join("v1.0.0");
        let next = f.registry.join(GROUP).join(DRIVER).join("v1.1.0");
        let text = fs::read_to_string(old.join("vibe.toml"))
            .unwrap()
            .replace("version='1.0.0'", "version='1.1.0'");
        let text = if cardinality == "zero-or-one" {
            text.replace("defaults = [\"enabled\"]", "defaults = [\"disabled\"]")
        } else {
            text.replace(
                "defaults = [\"enabled\",\"disabled\"]",
                "defaults = [\"disabled\"]",
            )
        };
        write(&next.join("vibe.toml"), text);
        write(
            &next.join("boot/rules.md"),
            "UNSELECTED_DEFAULT_CANDIDATE\n",
        );
        f.install_ok();
        let lock = f.lock();
        assert_eq!(
            lock.find(&GROUP.parse().unwrap(), DRIVER)
                .unwrap()
                .version
                .to_string(),
            "1.0.0"
        );
        assert!(!f.static_boot().contains("UNSELECTED_DEFAULT_CANDIDATE"));
        assert!(f.slot(EXTRA).exists());
        let before = super::workspace::semantic_snapshot(&f.project);
        f.install_ok();
        assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
        let request = f.project.join("vibe.toml");
        write(
            &request,
            fs::read_to_string(&request)
                .unwrap()
                .replace("version='=1.0.0'", "version='^1'"),
        );
        let before = super::workspace::semantic_snapshot(&f.project);
        let scalar = review_fixture();
        let shown = update(&scalar);
        let proposal: serde_json::Value = serde_json::from_slice(&shown.stderr).unwrap();
        let token = proposal["proposal"]["review_id"].as_str().unwrap();
        for response in [None, Some(token)] {
            let out = if let Some(token) = response {
                respond(&f, token)
            } else {
                update(&f)
            };
            let diagnostic = format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            assert!(!out.status.success());
            assert!(
                diagnostic.contains("multi-member automatic default renewal is not connected"),
                "{diagnostic}"
            );
            assert!(!diagnostic.contains("package-default-review/1:sha256:"));
            assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
        }
    }
}

#[test]
fn required_public_integer_bounds_zero_missing_and_bool_control_qualification() {
    for world in [
        "zero",
        "minimum",
        "maximum",
        "outside",
        "quoted",
        "missing",
        "bool-false",
    ] {
        let f = Fixture::new(false);
        let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
        let parameter = if world == "bool-false" {
            "\n[[configuration.parameters]]\nid='count'\nname='Count'\ntype='bool'\nrequired=true\n"
        } else {
            "\n[[configuration.parameters]]\nid='count'\nname='Count'\ntype='integer'\nrequired=true\nmin=-2\nmax=3\n"
        };
        write(&source, fs::read_to_string(&source).unwrap() + parameter);
        let request = f.project.join("vibe.toml");
        let value = match world {
            "zero" => Some("0"),
            "minimum" => Some("-2"),
            "maximum" => Some("3"),
            "outside" => Some("4"),
            "quoted" => Some("'0'"),
            "missing" => None,
            "bool-false" => Some("false"),
            _ => unreachable!(),
        };
        let mut text = fs::read_to_string(&request)
            .unwrap()
            .replace("version='^1'", "version='=1.0.0'");
        if let Some(value) = value {
            text=text.replace("selections={extra={intent='exact',members=['disabled']}}}",&format!("selections={{extra={{intent='exact',members=['disabled']}}}},parameters={{count={value}}}}}"));
        }
        write(&request, text);
        let original =
            Manifest::read(&request).unwrap().requires.configurations[&coordinate(DRIVER)].clone();
        let definition = Manifest::read(&source).unwrap().configuration.unwrap();
        let normalized = definition.normalize_input(&original, "./vibe.toml");
        println!("integer world={world} original={original:?} normalized={normalized:?}");
        match world {
            "zero" | "minimum" | "maximum" => assert_eq!(
                normalized.as_ref().unwrap().parameters["count"],
                vibe_core::manifest::ParameterValue::Integer(match world {
                    "minimum" => -2,
                    "maximum" => 3,
                    _ => 0,
                })
            ),
            "bool-false" => assert_eq!(
                normalized.as_ref().unwrap().parameters["count"],
                vibe_core::manifest::ParameterValue::Bool(false)
            ),
            "missing" => assert!(
                !normalized
                    .as_ref()
                    .unwrap()
                    .parameters
                    .contains_key("count")
            ),
            _ => assert!(normalized.is_err()),
        }
        let before = super::workspace::semantic_snapshot(&f.project);
        let registry = preview_scratch_snapshot(&f.registry);
        let authored = fs::read(&request).unwrap();
        let out = f.install();
        let diagnostic = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        println!(
            "integer world={world} status={} diagnostic={diagnostic}",
            out.status
        );
        if matches!(world, "bool-false" | "zero" | "minimum" | "maximum") {
            assert!(out.status.success(), "{diagnostic}");
            let lock = f.lock();
            let record = &lock.configurations[&coordinate(DRIVER)];
            assert_eq!(record.consumers.values().next().unwrap(), &original);
            assert_eq!(
                record
                    .selected_parameters(vibe_core::PackageKind::Flow)
                    .unwrap()["count"],
                if world == "bool-false" {
                    vibe_core::manifest::ParameterValue::Bool(false)
                } else {
                    vibe_core::manifest::ParameterValue::Integer(match world {
                        "minimum" => -2,
                        "maximum" => 3,
                        _ => 0,
                    })
                }
            );
            assert!(record.automatic_basis.is_none());
            assert_eq!(lock.meta.schema_version, 8);
            let shown = f
                .user
                .vibe()
                .args([
                    "show",
                    "configuration-data",
                    "--path",
                    f.project.to_str().unwrap(),
                    "--json",
                ])
                .output()
                .unwrap();
            assert!(shown.status.success());
            let report: serde_json::Value = serde_json::from_slice(&shown.stdout).unwrap();
            assert_eq!(
                report["packages"][0]["parameters"]["count"],
                if world == "bool-false" {
                    serde_json::json!(false)
                } else {
                    serde_json::json!(match world {
                        "minimum" => -2,
                        "maximum" => 3,
                        _ => 0,
                    })
                }
            );
            assert!(
                record
                    .selected_data(vibe_core::PackageKind::Flow)
                    .unwrap()
                    .is_empty()
            );
            assert!(f.slot(DRIVER).exists());
            assert!(f.static_boot().contains(DRIVER_MARKER));
            assert!(f.hooks().get(BASE).copied().unwrap_or(0) > 0);
            let installed = super::workspace::semantic_snapshot(&f.project);
            f.install_ok();
            assert_eq!(super::workspace::semantic_snapshot(&f.project), installed);
        } else {
            assert!(!out.status.success(), "{diagnostic}");
            if world == "missing" {
                assert!(
                    diagnostic.contains("missing required public parameter `count`"),
                    "{diagnostic}"
                );
            } else if world == "outside" {
                assert!(
                    diagnostic.contains("configuration.parameters.count")
                        && diagnostic.contains("4 is outside -2..=3"),
                    "{diagnostic}"
                );
            } else {
                assert!(
                    diagnostic.contains("configuration.parameters.count")
                        && diagnostic.contains("not a integer value"),
                    "{diagnostic}"
                );
            }
            assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
            assert!(!f.project.join("vibe.lock").exists());
            assert!(!f.project.join("effects.log").exists());
            for name in [DRIVER, BASE, EXTRA] {
                assert!(!f.slot(name).exists());
            }
        }
        assert_eq!(fs::read(&request).unwrap(), authored);
        assert_eq!(preview_scratch_snapshot(&f.registry), registry);
        println!(
            "integer world={world} coordination={:?}",
            preview_scratch_snapshot(&f.project.join(".vibe"))
                .as_ref()
                .map(|entries| entries.keys().collect::<Vec<_>>())
        );
    }
}

fn integer_fixture(value: i64) -> Fixture {
    let f = Fixture::new(false);
    let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
    write(
        &source,
        fs::read_to_string(&source).unwrap()
            + "\n[[configuration.parameters]]\nid='count'\nname='Count'\ntype='integer'\nrequired=true\nmin=-2\nmax=3\n",
    );
    let request = f.project.join("vibe.toml");
    write(&request,fs::read_to_string(&request).unwrap().replace("version='^1'","version='=1.0.0'").replace("selections={extra={intent='exact',members=['disabled']}}}",&format!("selections={{extra={{intent='exact',members=['disabled']}}}},parameters={{count={value}}}}}")));
    f
}

#[test]
fn integer_profile_preset_override_and_explicit_public_boundary() {
    for world in ["preset", "override", "explicit"] {
        let f = integer_fixture(0);
        let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
        write(
            &source,
            fs::read_to_string(&source).unwrap()
                + "\n[[configuration.profiles]]\nid='counted'\nname='Counted'\nparameters.count=-1\n",
        );
        let request = f.project.join("vibe.toml");
        let original_text = fs::read_to_string(&request).unwrap();
        let text = match world {
            "preset" => original_text.replace(",parameters={count=0}", "").replace(
                "default_features=false,",
                "default_features=false,profile='counted',",
            ),
            "override" => original_text.replace(
                "default_features=false,",
                "default_features=false,profile='counted',",
            ),
            _ => original_text,
        };
        write(&request, &text);
        let original =
            Manifest::read(&request).unwrap().requires.configurations[&coordinate(DRIVER)].clone();
        let definition = Manifest::read(&source).unwrap().configuration.unwrap();
        let normalized = definition
            .normalize_input(&original, "./vibe.toml")
            .unwrap();
        assert_eq!(
            normalized.parameters["count"],
            vibe_core::manifest::ParameterValue::Integer(if world == "preset" { -1 } else { 0 })
        );
        assert_eq!(original.profile.is_some(), world != "explicit");
        assert_eq!(original.parameters.contains_key("count"), world != "preset");
        let out = f.install();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let lock = f.lock();
        let record = &lock.configurations[&coordinate(DRIVER)];
        assert_eq!(lock.meta.schema_version, 8);
        assert_eq!(record.consumers["./vibe.toml"], original);
        assert!(record.automatic_basis.is_none());
        assert!(record.closure_fallback_basis.is_none());
        assert!(f.slot(BASE).exists());
        assert!(!f.slot(EXTRA).exists());
        assert_eq!(f.hooks().get(BASE), Some(&1));
        let before = super::workspace::semantic_snapshot(&f.project);
        let show = f
            .user
            .vibe()
            .args([
                "show",
                "configuration-data",
                "--path",
                f.project.to_str().unwrap(),
                "--json",
            ])
            .output()
            .unwrap();
        assert!(show.status.success());
        let shown: serde_json::Value = serde_json::from_slice(&show.stdout).unwrap();
        assert_eq!(
            shown["packages"][0]["parameters"]["count"],
            if world == "preset" { -1 } else { 0 }
        );
        assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
        fs::rename(&f.registry, f.root.join("registry-unavailable")).unwrap();
        f.install_ok();
        assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
        println!(
            "integer-profile world={world} normalized={} actual-install=success show=typed cold=unchanged",
            if world == "preset" { -1 } else { 0 }
        );
    }
    // Existing no-profile integer control omits the unrelated profile definition.
    let control = integer_fixture(0);
    control.install_ok();
    let before = super::workspace::semantic_snapshot(&control.project);
    control.install_ok();
    assert_eq!(
        super::workspace::semantic_snapshot(&control.project),
        before
    );
}

#[test]
fn integer_profile_effective_consumer_merge_prior_and_preset_drift() {
    for world in ["agree", "conflict", "unselected-missing", "prior", "drift"] {
        let f = integer_fixture(0);
        let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
        write(
            &source,
            fs::read_to_string(&source).unwrap()
                + "\n[[configuration.profiles]]\nid='counted'\nname='Counted'\nparameters.count=-1\n",
        );
        let request = f.project.join("vibe.toml");
        let text = fs::read_to_string(&request).unwrap();
        if matches!(world, "agree" | "conflict") {
            let registry = text.split("[[registry]]").nth(1).unwrap();
            write(
                &request,
                format!(
                    "[project]\nname='profile-integer-workspace'\nversion='0.1.0'\n[workspace]\nmembers=['left','right']\n[[registry]]{registry}"
                ),
            );
            for (member, profile, value) in [
                ("left", true, None),
                (
                    "right",
                    false,
                    Some(if world == "conflict" { 0 } else { -1 }),
                ),
            ] {
                let intent = format!(
                    "configuration={{policy='require-selection',default_features=false,{}selections={{extra={{intent='exact',members=['disabled']}}}}{}}}",
                    if profile { "profile='counted'," } else { "" },
                    value
                        .map(|v| format!(",parameters={{count={v}}}"))
                        .unwrap_or_default()
                );
                write(
                    &f.project.join(member).join("vibe.toml"),
                    format!(
                        "[project]\nname='{member}'\nversion='0.1.0'\n[requires.packages.'{GROUP}/{DRIVER}']\nversion='=1.0.0'\nlink='static-transitive'\n{intent}\n"
                    ),
                );
            }
        } else {
            let text = if world == "unselected-missing" {
                text.replace(",parameters={count=0}", "")
            } else {
                text.replace(",parameters={count=0}", "").replace(
                    "default_features=false,",
                    "default_features=false,profile='counted',",
                )
            };
            write(&request, text);
        }
        let before = super::workspace::semantic_snapshot(&f.project);
        let out = f.install();
        if matches!(world, "conflict" | "unselected-missing") {
            assert!(!out.status.success());
            assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
            let error = String::from_utf8_lossy(&out.stderr);
            assert!(
                error.contains(if world == "conflict" {
                    "conflicting explicit public integer"
                } else {
                    "missing required public parameter"
                }),
                "{error}"
            );
            continue;
        }
        assert!(
            out.status.success(),
            "{world} {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let lock = f.lock();
        let record = &lock.configurations[&coordinate(DRIVER)];
        assert_eq!(lock.meta.schema_version, 8);
        assert_eq!(
            record
                .selected_parameters(vibe_core::PackageKind::Flow)
                .unwrap()["count"],
            vibe_core::manifest::ParameterValue::Integer(-1)
        );
        if world == "agree" {
            assert_eq!(record.consumers.len(), 2);
            assert!(record.consumers["left/vibe.toml"].parameters.is_empty());
            assert_eq!(
                record.consumers["right/vibe.toml"].parameters["count"],
                vibe_core::manifest::ParameterValue::Integer(-1)
            );
        }
        let before = super::workspace::semantic_snapshot(&f.project);
        f.install_ok();
        assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
        if world == "prior" {
            let old = std::env::var_os("NST_OPTIONAL_BOOL_OLD_CLI").unwrap();
            for command in ["reader", "writer"] {
                let mut cmd = std::process::Command::new(&old);
                cmd.env_clear()
                    .env("HOME", f.root.join("home"))
                    .env("VIBE_SETTINGS", &f.user.settings)
                    .env("PATH", "/usr/bin:/bin")
                    .env("VIBE_NO_DEFAULT_REGISTRY", "1");
                if command == "reader" {
                    cmd.args(["show", "configuration-data"]);
                } else {
                    cmd.args(["install", "--offline", "--assume-yes"]);
                }
                let out = cmd
                    .args(["--path", f.project.to_str().unwrap(), "--json"])
                    .output()
                    .unwrap();
                assert!(!out.status.success());
                assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
            }
        }
        if world == "drift" {
            let next = f.registry.join(GROUP).join(DRIVER).join("v1.1.0");
            write(
                &next.join("vibe.toml"),
                fs::read_to_string(&source)
                    .unwrap()
                    .replace("version='1.0.0'", "version='1.1.0'")
                    .replace("parameters.count=-1", "parameters.count=1"),
            );
            write(&next.join("boot/rules.md"), "CHANGED_INTEGER_PRESET\n");
            write(
                &request,
                fs::read_to_string(&request)
                    .unwrap()
                    .replace("version='=1.0.0'", "version='=1.1.0'"),
            );
            let before = super::workspace::semantic_snapshot(&f.project);
            assert!(!update(&f).status.success());
            assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
            assert!(!f.static_boot().contains("CHANGED_INTEGER_PRESET"));
        }
        println!("integer-profile extra-world={world} effective=-1 originals=retained");
    }
}

#[test]
fn integer_real_consumer_agreement_and_conflict_preserve_originals() {
    for conflict in [false, true] {
        let f = integer_fixture(0);
        let root = f.project.join("vibe.toml");
        let text = fs::read_to_string(&root).unwrap();
        let registry = text.split("[[registry]]").nth(1).unwrap();
        write(
            &root,
            format!(
                "[project]\nname='integer-workspace'\nversion='0.1.0'\n[workspace]\nmembers=['left','right']\n[[registry]]{registry}"
            ),
        );
        for (member, value) in [("left", 0), ("right", if conflict { 3 } else { 0 })] {
            write(
                &f.project.join(member).join("vibe.toml"),
                format!(
                    "[project]\nname='{member}'\nversion='0.1.0'\n[requires.packages.'{GROUP}/{DRIVER}']\nversion='=1.0.0'\nlink='static-transitive'\nconfiguration={{policy='require-selection',default_features=false,selections={{extra={{intent='exact',members=['disabled']}}}},parameters={{count={value}}}}}\n"
                ),
            );
        }
        let before = super::workspace::semantic_snapshot(&f.project);
        let out = f.install();
        let diagnostic = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        if conflict {
            assert!(!out.status.success());
            assert!(
                diagnostic.contains("conflicting explicit public integer parameter `count`"),
                "{diagnostic}"
            );
            assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
        } else {
            assert!(out.status.success(), "{diagnostic}");
            let lock = f.lock();
            let record = &lock.configurations[&coordinate(DRIVER)];
            assert_eq!(record.consumers.len(), 2);
            assert!(
                record
                    .consumers
                    .values()
                    .all(|input| input.parameters["count"]
                        == vibe_core::manifest::ParameterValue::Integer(0))
            );
            assert_eq!(
                record
                    .selected_parameters(vibe_core::PackageKind::Flow)
                    .unwrap()["count"],
                vibe_core::manifest::ParameterValue::Integer(0)
            );
            let before = super::workspace::semantic_snapshot(&f.project);
            f.install_ok();
            assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
        }
    }
}

#[test]
fn integer_old_reader_writer_refuse_exact_prior_without_loss() {
    let old = std::env::var_os("NST_INTEGER_OLD_CLI")
        .expect("mandatory explicitly mounted pre-integer CLI");
    for value in [-2, 0, 3] {
        for operation in ["reader", "writer"] {
            let f = integer_fixture(value);
            f.install_ok();
            let before = super::workspace::semantic_snapshot(&f.project);
            let args = if operation == "reader" {
                vec![
                    "show",
                    "configuration-data",
                    "--path",
                    f.project.to_str().unwrap(),
                    "--json",
                ]
            } else {
                vec![
                    "install",
                    "--path",
                    f.project.to_str().unwrap(),
                    "--offline",
                    "--assume-yes",
                    "--json",
                ]
            };
            let out = std::process::Command::new(&old)
                .env_clear()
                .env("HOME", f.root.join("home"))
                .env("VIBE_SETTINGS", &f.user.settings)
                .env("VIBE_NO_DEFAULT_REGISTRY", "1")
                .env("PATH", "/usr/bin:/bin")
                .args(args)
                .output()
                .unwrap();
            let diagnostic = format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            println!(
                "prior integer={value} {operation} status={} diagnostic={diagnostic}",
                out.status
            );
            assert!(!out.status.success());
            assert!(diagnostic.contains("only explicitly supplied required public bool parameters without defaults are connected"),"{diagnostic}");
            assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
        }
    }
}

#[test]
fn integer_unselected_bounds_and_maintenance_invalidating_value_refuse() {
    for exact_new in [false, true] {
        let f = integer_fixture(-2);
        let old = f.registry.join(GROUP).join(DRIVER).join("v1.0.0");
        let next = f.registry.join(GROUP).join(DRIVER).join("v1.1.0");
        write(
            &next.join("vibe.toml"),
            fs::read_to_string(old.join("vibe.toml"))
                .unwrap()
                .replace("version='1.0.0'", "version='1.1.0'")
                .replace("min=-2", "min=0"),
        );
        write(&next.join("boot/rules.md"), "UNSELECTED_INTEGER_BOUNDS\n");
        f.install_ok();
        assert_eq!(
            f.lock()
                .find(&GROUP.parse().unwrap(), DRIVER)
                .unwrap()
                .version
                .to_string(),
            "1.0.0"
        );
        assert!(!f.static_boot().contains("UNSELECTED_INTEGER_BOUNDS"));
        let request = f.project.join("vibe.toml");
        write(
            &request,
            fs::read_to_string(&request).unwrap().replace(
                "version='=1.0.0'",
                if exact_new {
                    "version='=1.1.0'"
                } else {
                    "version='^1'"
                },
            ),
        );
        let before = super::workspace::semantic_snapshot(&f.project);
        let out = update(&f);
        let diagnostic = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        if exact_new {
            assert!(!out.status.success(), "{diagnostic}");
            assert!(diagnostic.contains("-2 is outside 0..=3"), "{diagnostic}");
        } else {
            assert!(out.status.success(), "{diagnostic}");
            assert_eq!(
                f.lock()
                    .find(&GROUP.parse().unwrap(), DRIVER)
                    .unwrap()
                    .version
                    .to_string(),
                "1.0.0"
            );
            assert_eq!(
                f.lock().configurations[&coordinate(DRIVER)]
                    .selected_parameters(vibe_core::PackageKind::Flow)
                    .unwrap()["count"],
                vibe_core::manifest::ParameterValue::Integer(-2)
            );
        }
        assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
    }
}

#[test]
fn public_text_and_choice_parameters_literal_values_qualification() {
    for kind in ["string", "choice"] {
        let worlds: &[&str] = if kind == "string" {
            &["missing", "empty", "spaced", "wrong-type"]
        } else {
            &["missing", "alpha", "beta", "unknown", "label"]
        };
        for world in worlds {
            let f = Fixture::new(false);
            let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
            let choices = if kind == "choice" {
                "choices=[{value='alpha',name='Friendly Alpha'},{value='beta',name='Friendly Beta'}]\n"
            } else {
                ""
            };
            write(
                &source,
                fs::read_to_string(&source).unwrap()
                    + &format!(
                        "\n[[configuration.parameters]]\nid='text'\nname='Text'\ntype='{kind}'\nrequired=true\n{choices}"
                    ),
            );
            let value = match *world {
                "missing" => None,
                "empty" => Some("''"),
                "spaced" => Some("'  literal value  '"),
                "wrong-type" => Some("0"),
                "alpha" => Some("'alpha'"),
                "beta" => Some("'beta'"),
                "unknown" => Some("'other'"),
                "label" => Some("'Friendly Alpha'"),
                _ => unreachable!(),
            };
            let request = f.project.join("vibe.toml");
            let mut text = fs::read_to_string(&request)
                .unwrap()
                .replace("version='^1'", "version='=1.0.0'");
            if let Some(value) = value {
                text=text.replace("selections={extra={intent='exact',members=['disabled']}}}",&format!("selections={{extra={{intent='exact',members=['disabled']}}}},parameters={{text={value}}}}}"));
            }
            write(&request, text);
            let original = Manifest::read(&request).unwrap().requires.configurations
                [&coordinate(DRIVER)]
                .clone();
            let definition = Manifest::read(&source).unwrap().configuration.unwrap();
            let normalized = definition.normalize_input(&original, "./vibe.toml");
            let valid = !matches!(*world, "wrong-type" | "unknown" | "label");
            assert_eq!(normalized.is_ok(), valid);
            if valid {
                if *world == "missing" {
                    assert!(!normalized.as_ref().unwrap().parameters.contains_key("text"));
                } else {
                    assert_eq!(
                        normalized.as_ref().unwrap().parameters["text"],
                        vibe_core::manifest::ParameterValue::Text(
                            match *world {
                                "empty" => "",
                                "spaced" => "  literal value  ",
                                "alpha" => "alpha",
                                _ => "beta",
                            }
                            .into()
                        )
                    );
                }
            }
            println!(
                "text kind={kind} world={world} original={original:?} normalized={normalized:?}"
            );
            let before = super::workspace::semantic_snapshot(&f.project);
            let registry = preview_scratch_snapshot(&f.registry);
            let authored = fs::read(&request).unwrap();
            let out = f.install();
            let diagnostic = format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            println!(
                "text kind={kind} world={world} status={} diagnostic={diagnostic}",
                out.status
            );
            if valid && *world != "missing" {
                assert!(out.status.success(), "{diagnostic}");
                let lock = f.lock();
                let record = &lock.configurations[&coordinate(DRIVER)];
                assert_eq!(record.consumers.values().next().unwrap(), &original);
                assert_eq!(
                    record
                        .selected_parameters(vibe_core::PackageKind::Flow)
                        .unwrap()["text"],
                    original.parameters["text"]
                );
                assert!(
                    record
                        .selected_data(vibe_core::PackageKind::Flow)
                        .unwrap()
                        .is_empty()
                );
                assert!(record.automatic_basis.is_none());
                assert_eq!(lock.meta.schema_version, 8);
                let shown = f
                    .user
                    .vibe()
                    .args([
                        "show",
                        "configuration-data",
                        "--path",
                        f.project.to_str().unwrap(),
                        "--json",
                    ])
                    .output()
                    .unwrap();
                assert!(shown.status.success());
                let report: serde_json::Value = serde_json::from_slice(&shown.stdout).unwrap();
                let vibe_core::manifest::ParameterValue::Text(value) = &original.parameters["text"]
                else {
                    panic!("wrong original type")
                };
                assert_eq!(
                    report["packages"][0]["parameters"]["text"],
                    serde_json::json!(value)
                );
                assert!(f.slot(DRIVER).exists());
                assert!(f.static_boot().contains(DRIVER_MARKER));
                assert!(f.hooks().get(BASE).copied().unwrap_or(0) > 0);
                let installed = super::workspace::semantic_snapshot(&f.project);
                f.install_ok();
                assert_eq!(super::workspace::semantic_snapshot(&f.project), installed);
            } else {
                assert!(!out.status.success(), "{diagnostic}");
                let reason = match *world {
                    "missing" => "missing required public parameter `text`",
                    "wrong-type" => "not a string value",
                    "unknown" => "`other` is not one of alpha, beta",
                    "label" => "`Friendly Alpha` is not one of alpha, beta",
                    _ => unreachable!(),
                };
                assert!(diagnostic.contains(reason), "{diagnostic}");
                assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
                assert!(!f.project.join("vibe.lock").exists());
                assert!(!f.project.join("effects.log").exists());
                for name in [DRIVER, BASE, EXTRA] {
                    assert!(!f.slot(name).exists());
                }
            }
            assert_eq!(fs::read(&request).unwrap(), authored);
            assert_eq!(preview_scratch_snapshot(&f.registry), registry);
            println!(
                "text kind={kind} world={world} coordination={:?}",
                preview_scratch_snapshot(&f.project.join(".vibe"))
                    .as_ref()
                    .map(|entries| entries.keys().collect::<Vec<_>>())
            );
        }
    }
    let f = integer_fixture(0);
    f.install_ok();
    let lock = f.lock();
    assert_eq!(
        lock.configurations[&coordinate(DRIVER)]
            .selected_parameters(vibe_core::PackageKind::Flow)
            .unwrap()["count"],
        vibe_core::manifest::ParameterValue::Integer(0)
    );
    let before = super::workspace::semantic_snapshot(&f.project);
    f.install_ok();
    assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
}

fn text_choice_fixture(kind: &str, value: &str) -> Fixture {
    let f = Fixture::new(false);
    let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
    let choices = if kind == "choice" {
        "choices=[{value='alpha',name='Friendly Alpha'},{value='beta',name='Friendly Beta'}]\n"
    } else {
        ""
    };
    write(
        &source,
        fs::read_to_string(&source).unwrap()
            + &format!(
                "\n[[configuration.parameters]]\nid='text'\nname='Text'\ntype='{kind}'\nrequired=true\n{choices}"
            ),
    );
    let request = f.project.join("vibe.toml");
    write(&request,fs::read_to_string(&request).unwrap().replace("version='^1'","version='=1.0.0'").replace("selections={extra={intent='exact',members=['disabled']}}}",&format!("selections={{extra={{intent='exact',members=['disabled']}}}},parameters={{text='{value}'}}}}")));
    f
}

#[test]
fn text_choice_equal_and_whitespace_or_value_conflicting_real_consumers() {
    for kind in ["string", "choice"] {
        for conflict in [false, true] {
            let first = if kind == "string" {
                " literal "
            } else {
                "alpha"
            };
            let f = text_choice_fixture(kind, first);
            let root = f.project.join("vibe.toml");
            let text = fs::read_to_string(&root).unwrap();
            let registry = text.split("[[registry]]").nth(1).unwrap();
            write(
                &root,
                format!(
                    "[project]\nname='text-workspace'\nversion='0.1.0'\n[workspace]\nmembers=['left','right']\n[[registry]]{registry}"
                ),
            );
            for (member, value) in [
                ("left", first),
                (
                    "right",
                    if conflict {
                        if kind == "string" { "literal" } else { "beta" }
                    } else {
                        first
                    },
                ),
            ] {
                write(
                    &f.project.join(member).join("vibe.toml"),
                    format!(
                        "[project]\nname='{member}'\nversion='0.1.0'\n[requires.packages.'{GROUP}/{DRIVER}']\nversion='=1.0.0'\nlink='static-transitive'\nconfiguration={{policy='require-selection',default_features=false,selections={{extra={{intent='exact',members=['disabled']}}}},parameters={{text='{value}'}}}}\n"
                    ),
                );
            }
            let before = super::workspace::semantic_snapshot(&f.project);
            let out = f.install();
            let diagnostic = format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            if conflict {
                assert!(!out.status.success());
                assert!(
                    diagnostic.contains(&format!(
                        "conflicting explicit public {kind} parameter `text`"
                    )),
                    "{diagnostic}"
                );
                assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
            } else {
                assert!(out.status.success(), "{diagnostic}");
                let lock = f.lock();
                assert_eq!(lock.configurations[&coordinate(DRIVER)].consumers.len(), 2);
                let before = super::workspace::semantic_snapshot(&f.project);
                f.install_ok();
                assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
            }
        }
    }
}

#[test]
fn text_choice_actual_prior_reader_writer_refuse_without_loss() {
    let old =
        std::env::var_os("NST_TEXT_OLD_CLI").expect("mandatory explicitly mounted pre-text CLI");
    for (kind, value) in [
        ("string", ""),
        ("string", " literal "),
        ("choice", "alpha"),
        ("choice", "beta"),
    ] {
        for operation in ["reader", "writer"] {
            let f = text_choice_fixture(kind, value);
            f.install_ok();
            let before = super::workspace::semantic_snapshot(&f.project);
            let args = if operation == "reader" {
                vec![
                    "show",
                    "configuration-data",
                    "--path",
                    f.project.to_str().unwrap(),
                    "--json",
                ]
            } else {
                vec![
                    "install",
                    "--path",
                    f.project.to_str().unwrap(),
                    "--offline",
                    "--assume-yes",
                    "--json",
                ]
            };
            let out = std::process::Command::new(&old)
                .env_clear()
                .env("HOME", f.root.join("home"))
                .env("VIBE_SETTINGS", &f.user.settings)
                .env("VIBE_NO_DEFAULT_REGISTRY", "1")
                .env("PATH", "/usr/bin:/bin")
                .args(args)
                .output()
                .unwrap();
            let diagnostic = format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            println!(
                "prior text {kind} value={value:?} {operation} status={} diagnostic={diagnostic}",
                out.status
            );
            assert!(!out.status.success());
            assert!(diagnostic.contains("only explicitly supplied required public bool parameters without defaults are connected"),"{diagnostic}");
            assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
        }
    }
}

#[test]
fn choice_new_allowed_values_forced_update_refuses_but_valid_fallback_survives() {
    for exact_new in [false, true] {
        let f = text_choice_fixture("choice", "alpha");
        let old = f.registry.join(GROUP).join(DRIVER).join("v1.0.0");
        let next = f.registry.join(GROUP).join(DRIVER).join("v1.1.0");
        write(
            &next.join("vibe.toml"),
            fs::read_to_string(old.join("vibe.toml"))
                .unwrap()
                .replace("version='1.0.0'", "version='1.1.0'")
                .replace("{value='alpha',name='Friendly Alpha'},", ""),
        );
        write(&next.join("boot/rules.md"), "UNSELECTED_CHOICE_VALUES\n");
        f.install_ok();
        assert!(!f.static_boot().contains("UNSELECTED_CHOICE_VALUES"));
        let request = f.project.join("vibe.toml");
        write(
            &request,
            fs::read_to_string(&request).unwrap().replace(
                "version='=1.0.0'",
                if exact_new {
                    "version='=1.1.0'"
                } else {
                    "version='^1'"
                },
            ),
        );
        let before = super::workspace::semantic_snapshot(&f.project);
        let out = update(&f);
        let diagnostic = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        if exact_new {
            assert!(!out.status.success());
            assert!(
                diagnostic.contains("`alpha` is not one of beta"),
                "{diagnostic}"
            );
        } else {
            assert!(out.status.success(), "{diagnostic}");
            assert_eq!(
                f.lock()
                    .find(&GROUP.parse().unwrap(), DRIVER)
                    .unwrap()
                    .version
                    .to_string(),
                "1.0.0"
            );
        }
        assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
    }
}

#[test]
fn public_path_lexical_and_host_boundary_qualification() {
    for world in [
        "missing",
        "empty",
        "wrong-type",
        "relative",
        "absolute",
        "present-required",
        "absent-required",
        "present-optional",
        "absent-optional",
        "string-control",
    ] {
        for alternate_cwd in [false, true] {
            if alternate_cwd && world != "relative" {
                continue;
            }
            let f = Fixture::new(false);
            let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
            let kind = if world == "string-control" {
                "string"
            } else {
                "path"
            };
            let required = world.ends_with("required");
            let must_exist = if required {
                "must_exist=true\n"
            } else if kind == "path" {
                "must_exist=false\n"
            } else {
                ""
            };
            write(
                &source,
                fs::read_to_string(&source).unwrap()
                    + &format!(
                        "\n[[configuration.parameters]]\nid='location'\nname='Location'\ntype='{kind}'\nrequired=true\n{must_exist}"
                    ),
            );
            write(
                &f.project.join("present path"),
                "SYNTHETIC_PATH_TARGET_NEVER_READ\n",
            );
            let absolute = f.project.join("present path");
            let value = match world {
                "missing" => None,
                "empty" => Some("''".to_owned()),
                "wrong-type" => Some("0".to_owned()),
                "absolute" => Some(format!("'{}'", absolute.display())),
                "present-required" | "present-optional" => Some("'present path'".to_owned()),
                "absent-required" | "absent-optional" => Some("'absent path'".to_owned()),
                _ => Some("'relative path with spaces'".to_owned()),
            };
            let request = f.project.join("vibe.toml");
            let mut text = fs::read_to_string(&request)
                .unwrap()
                .replace("version='^1'", "version='=1.0.0'");
            if let Some(value) = &value {
                text=text.replace("selections={extra={intent='exact',members=['disabled']}}}",&format!("selections={{extra={{intent='exact',members=['disabled']}}}},parameters={{location={value}}}}}"));
            }
            write(&request, text);
            let original = Manifest::read(&request).unwrap().requires.configurations
                [&coordinate(DRIVER)]
                .clone();
            let definition = Manifest::read(&source).unwrap().configuration.unwrap();
            let normalized = definition.normalize_input(&original, "./vibe.toml");
            assert_eq!(normalized.is_ok(), !matches!(world, "empty" | "wrong-type"));
            if let Ok(normalized) = &normalized {
                if world == "missing" {
                    assert!(!normalized.parameters.contains_key("location"));
                } else {
                    assert_eq!(
                        normalized.parameters["location"],
                        original.parameters["location"]
                    );
                }
            }
            println!(
                "path world={world} alternate_cwd={alternate_cwd} original={original:?} normalized={normalized:?}"
            );
            let other = f.root.join("different-cwd");
            fs::create_dir_all(&other).unwrap();
            write(
                &other.join("relative path with spaces"),
                "DIFFERENT_CWD_SENTINEL\n",
            );
            let before = super::workspace::semantic_snapshot(&f.project);
            let registry = preview_scratch_snapshot(&f.registry);
            let other_before = preview_scratch_snapshot(&other);
            let authored = fs::read(&request).unwrap();
            let cwd = if alternate_cwd { &other } else { &f.project };
            let out = f
                .user
                .vibe()
                .current_dir(cwd)
                .env("HOME", f.root.join("home"))
                .args([
                    "install",
                    "--path",
                    f.project.to_str().unwrap(),
                    "--offline",
                    "--assume-yes",
                    "--json",
                ])
                .output()
                .unwrap();
            let diagnostic = format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            println!(
                "path world={world} alternate_cwd={alternate_cwd} status={} diagnostic={diagnostic}",
                out.status
            );
            if world == "string-control"
                || (!required && !matches!(world, "missing" | "empty" | "wrong-type"))
            {
                assert!(out.status.success(), "{diagnostic}");
                let lock = f.lock();
                let record = &lock.configurations[&coordinate(DRIVER)];
                assert_eq!(
                    record
                        .selected_parameters(vibe_core::PackageKind::Flow)
                        .unwrap()["location"],
                    original.parameters["location"]
                );
                assert!(
                    record
                        .selected_data(vibe_core::PackageKind::Flow)
                        .unwrap()
                        .is_empty()
                );
                assert_eq!(lock.meta.schema_version, 8);
                assert!(record.automatic_basis.is_none());
                let shown = f
                    .user
                    .vibe()
                    .args([
                        "show",
                        "configuration-data",
                        "--path",
                        f.project.to_str().unwrap(),
                        "--json",
                    ])
                    .output()
                    .unwrap();
                assert!(shown.status.success());
                let report: serde_json::Value = serde_json::from_slice(&shown.stdout).unwrap();
                let vibe_core::manifest::ParameterValue::Text(value) =
                    &original.parameters["location"]
                else {
                    panic!("not text")
                };
                assert_eq!(
                    report["packages"][0]["parameters"]["location"],
                    serde_json::json!(value)
                );
                assert!(f.slot(DRIVER).exists());
                assert!(f.static_boot().contains(DRIVER_MARKER));
                let before = super::workspace::semantic_snapshot(&f.project);
                f.install_ok();
                assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
            } else {
                assert!(!out.status.success(), "{diagnostic}");
                if matches!(world, "empty" | "wrong-type") {
                    assert!(
                        diagnostic.contains("configuration.parameters.location")
                            && diagnostic.contains(if world == "empty" {
                                "a path must be nonempty"
                            } else {
                                "not a path value"
                            }),
                        "{diagnostic}"
                    );
                } else if world == "missing" {
                    assert!(
                        diagnostic.contains("missing required public parameter `location`"),
                        "{diagnostic}"
                    );
                } else {
                    assert!(diagnostic.contains("public path must_exist=true requires a separately specified planning-host base and check"),"{diagnostic}");
                }
                assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
                assert!(!f.project.join("vibe.lock").exists());
                assert!(!f.project.join("effects.log").exists());
                for name in [DRIVER, BASE, EXTRA] {
                    assert!(!f.slot(name).exists());
                }
            }
            assert_eq!(fs::read(&request).unwrap(), authored);
            assert_eq!(preview_scratch_snapshot(&f.registry), registry);
            assert_eq!(preview_scratch_snapshot(&other), other_before);
            println!(
                "path world={world} coordination={:?}",
                preview_scratch_snapshot(&f.project.join(".vibe"))
                    .as_ref()
                    .map(|entries| entries.keys().collect::<Vec<_>>())
            );
        }
    }
}

fn lexical_path_fixture(value: &str) -> Fixture {
    text_choice_fixture("path", value)
}

#[test]
fn host_required_path_install_cold_metadata_and_other_routes() {
    for world in [
        "file",
        "directory",
        "cwd-only",
        "missing",
        "absolute",
        "traversal",
        "terminal-link",
        "ancestor-link",
        "in-place",
        "cold-source-kind",
        "disappear",
    ] {
        let f = lexical_path_fixture("target path");
        let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
        let mut text = fs::read_to_string(&source).unwrap();
        let start = text.find("[[configuration.features]]").unwrap();
        let parameter = text.find("[[configuration.parameters]]").unwrap();
        text.replace_range(start..parameter, "");
        text = text.replace("must_exist=false", "must_exist=true");
        if !text.contains("must_exist=true") {
            text += "\nmust_exist=true\n";
        }
        if world == "in-place" {
            text = text.replace("kind='flow'", "kind='flow'\nmaterialization='in-place'");
        }
        write(&source, text);
        let request = f.project.join("vibe.toml");
        let text = fs::read_to_string(&request).unwrap().replace(
            "selections={extra={intent='exact',members=['disabled']}},",
            "",
        );
        write(&request, text);
        let target = f.project.join("target path");
        let cwd = f.root.join("different-cwd");
        fs::create_dir(&cwd).unwrap();
        write(&cwd.join("target path"), "CWD_ONLY_SENTINEL\n");
        match world {
            "directory" => fs::create_dir(&target).unwrap(),
            "file" | "disappear" | "in-place" | "cold-source-kind" => {
                write(&target, "TARGET_CONTENT_NOT_A_READ_GRANT\n")
            }
            "terminal-link" => {
                std::os::unix::fs::symlink(cwd.join("target path"), &target).unwrap()
            }
            "ancestor-link" => {
                std::os::unix::fs::symlink(&cwd, f.project.join("alias")).unwrap();
                write(
                    &request,
                    fs::read_to_string(&request)
                        .unwrap()
                        .replace("target path", "alias/target path"),
                );
            }
            "absolute" => write(
                &request,
                fs::read_to_string(&request)
                    .unwrap()
                    .replace("target path", &cwd.join("target path").to_string_lossy()),
            ),
            "traversal" => write(
                &request,
                fs::read_to_string(&request)
                    .unwrap()
                    .replace("target path", "../different-cwd/target path"),
            ),
            _ => {}
        }
        let before = super::workspace::semantic_snapshot(&f.project);
        let out = f
            .user
            .vibe()
            .current_dir(&cwd)
            .args([
                "install",
                "--path",
                f.project.to_str().unwrap(),
                "--offline",
                "--assume-yes",
                "--json",
            ])
            .output()
            .unwrap();
        let success = matches!(
            world,
            "file" | "directory" | "disappear" | "cold-source-kind"
        );
        assert_eq!(
            out.status.success(),
            success,
            "world={world} {}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        if !success {
            assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
            continue;
        }
        let lock = f.lock();
        assert_eq!(lock.meta.schema_version, 8);
        assert_eq!(
            lock.configurations[&coordinate(DRIVER)].consumers["./vibe.toml"].parameters["text"],
            vibe_core::manifest::ParameterValue::Text("target path".into())
        );
        let before = super::workspace::semantic_snapshot(&f.project);
        let prior =
            std::env::var_os("NST_OPTIONAL_BOOL_OLD_CLI").expect("explicit pre-host-path client");
        for command in ["show", "install"] {
            let mut cmd = std::process::Command::new(&prior);
            cmd.env_clear()
                .env("HOME", f.root.join("home"))
                .env("VIBE_SETTINGS", &f.user.settings)
                .env("PATH", "/usr/bin:/bin")
                .env("VIBE_NO_DEFAULT_REGISTRY", "1");
            if command == "show" {
                cmd.args(["show", "configuration-data"]);
            } else {
                cmd.args(["install", "--offline", "--assume-yes"]);
            }
            let out = cmd
                .args(["--path", f.project.to_str().unwrap(), "--json"])
                .output()
                .unwrap();
            assert!(
                !out.status.success(),
                "prior must not silently claim host readiness"
            );
            assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
        }
        for args in [vec!["update", "--all"], vec!["reinstall"]] {
            let out = f
                .user
                .vibe()
                .args(args)
                .args([
                    "--path",
                    f.project.to_str().unwrap(),
                    "--offline",
                    "--assume-yes",
                    "--json",
                ])
                .output()
                .unwrap();
            assert!(!out.status.success());
            assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
        }
        if world == "disappear" {
            fs::remove_file(&target).unwrap();
        }
        if world == "cold-source-kind" {
            let mut lock = f.lock();
            lock.packages
                .iter_mut()
                .find(|p| p.name.as_str() == DRIVER)
                .unwrap()
                .source_kind = Some(vibe_core::manifest::SourceKind::Git);
            write(
                &f.project.join("vibe.lock"),
                toml::to_string(&lock).unwrap(),
            );
            assert!(target.is_file());
        }
        let before = super::workspace::semantic_snapshot(&f.project);
        let out = f.install();
        assert_eq!(
            out.status.success(),
            !matches!(world, "disappear" | "cold-source-kind")
        );
        assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
        let shown = f
            .user
            .vibe()
            .args([
                "show",
                "configuration-data",
                "--path",
                f.project.to_str().unwrap(),
                "--json",
            ])
            .output()
            .unwrap();
        assert!(shown.status.success(), "history show is not readiness");
        println!(
            "host-path world={world} install=true cold={} history-show=true",
            out.status.success()
        );
    }
}

#[test]
fn lexical_path_spelling_consumers_and_target_disappearance() {
    for world in ["disappears", "equal", "different-spelling"] {
        let f = lexical_path_fixture("target path");
        let target = f.project.join("target path");
        write(&target, "SYNTHETIC_REFERENCE_CONTENT\n");
        if world != "disappears" {
            let request = f.project.join("vibe.toml");
            let text = fs::read_to_string(&request).unwrap();
            let registry = text.split("[[registry]]").nth(1).unwrap();
            write(
                &request,
                format!(
                    "[project]\nname='lexical-workspace'\nversion='0.1.0'\n[workspace]\nmembers=['left','right']\n[[registry]]{registry}"
                ),
            );
            for (member, value) in [
                ("left", "target path"),
                (
                    "right",
                    if world == "equal" {
                        "target path"
                    } else {
                        "./target path"
                    },
                ),
            ] {
                write(
                    &f.project.join(member).join("vibe.toml"),
                    format!(
                        "[project]\nname='{member}'\nversion='0.1.0'\n[requires.packages.'{GROUP}/{DRIVER}']\nversion='=1.0.0'\nlink='static-transitive'\nconfiguration={{policy='require-selection',default_features=false,selections={{extra={{intent='exact',members=['disabled']}}}},parameters={{text='{value}'}}}}\n"
                    ),
                );
            }
        }
        let before = super::workspace::semantic_snapshot(&f.project);
        let out = f.install();
        let diagnostic = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        if world == "different-spelling" {
            assert!(!out.status.success());
            assert!(
                diagnostic.contains("conflicting explicit public path parameter `text`"),
                "{diagnostic}"
            );
            assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
        } else {
            assert!(out.status.success(), "{diagnostic}");
            assert_eq!(fs::read(&target).unwrap(), b"SYNTHETIC_REFERENCE_CONTENT\n");
            if world == "disappears" {
                fs::remove_file(&target).unwrap();
            }
            let before = super::workspace::semantic_snapshot(&f.project);
            f.install_ok();
            assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
        }
    }
}

#[test]
fn lexical_path_actual_prior_reader_writer_refuse_without_loss() {
    let old =
        std::env::var_os("NST_PATH_OLD_CLI").expect("mandatory explicitly mounted pre-path CLI");
    for value in [
        "relative path",
        "/synthetic/absolute path",
        "~/literal/$NOT_EXPANDED",
    ] {
        for operation in ["reader", "writer"] {
            let f = lexical_path_fixture(value);
            f.install_ok();
            let before = super::workspace::semantic_snapshot(&f.project);
            let args = if operation == "reader" {
                vec![
                    "show",
                    "configuration-data",
                    "--path",
                    f.project.to_str().unwrap(),
                    "--json",
                ]
            } else {
                vec![
                    "install",
                    "--path",
                    f.project.to_str().unwrap(),
                    "--offline",
                    "--assume-yes",
                    "--json",
                ]
            };
            let out = std::process::Command::new(&old)
                .env_clear()
                .env("HOME", f.root.join("home"))
                .env("VIBE_SETTINGS", &f.user.settings)
                .env("VIBE_NO_DEFAULT_REGISTRY", "1")
                .env("PATH", "/usr/bin:/bin")
                .args(args)
                .output()
                .unwrap();
            let diagnostic = format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            println!(
                "prior path={value:?} {operation} status={} diagnostic={diagnostic}",
                out.status
            );
            assert!(!out.status.success());
            assert!(diagnostic.contains("only explicitly supplied required public bool parameters without defaults are connected"),"{diagnostic}");
            assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
        }
    }
}

fn synthetic_submitted_history_fixture() -> Fixture {
    let f = preview_fixture();
    let request = f.project.join("vibe.toml");
    let raw = fs::read_to_string(&request).unwrap();
    write(&request,raw.replace("link='static-transitive'","link='static-transitive'\nconfiguration={policy='require-selection',default_features=false,selections={extra={intent='exact',members=['disabled']}},parameters={verbose=false}}"));
    f.install_ok();
    let mut lock = f.lock();
    write(&request, &raw);
    let witness = vibe_core::manifest::submitted_project_witness(
        &f.project.canonicalize().unwrap(),
        raw.as_bytes(),
    )
    .unwrap();
    let row = lock
        .packages
        .iter()
        .find(|row| row.name.as_str() == DRIVER)
        .unwrap();
    let expected_source = vibe_core::manifest::SubmittedSource {
        registry: row.registry.clone().unwrap(),
        content_hash: row.content_hash.clone(),
    };
    let record = lock.configurations.get_mut(&coordinate(DRIVER)).unwrap();
    let original = record.consumers.remove("./vibe.toml").unwrap();
    assert!(record.consumers.is_empty());
    record.submitted_inputs.insert(
        "./vibe.toml".into(),
        vibe_core::manifest::SubmittedInput {
            original,
            requirement: Manifest::read(&request).unwrap().requires.packages[0].clone(),
            project_witness: witness,
            expected_source,
        },
    );
    lock.meta.schema_version = 16;
    // Stage A deliberately handcrafts reader data; no public submitted writer.
    write(
        &f.project.join("vibe.lock"),
        toml::to_string(&lock).unwrap(),
    );
    f
}

#[test]
fn submitted_history_stage_a_reader_freshness_and_writer_withheld() {
    for mutation in [
        "none",
        "epoch",
        "owner",
        "overlap",
        "removed",
        "extra",
        "requirement",
        "project-edit",
        "project-copy",
        "authored",
        "source-binding",
        "graph",
    ] {
        let f = synthetic_submitted_history_fixture();
        let mut lock = f.lock();
        let record = lock.configurations.get_mut(&coordinate(DRIVER)).unwrap();
        match mutation {
            "epoch" => lock.meta.schema_version = 15,
            "owner" => {
                let value = record.submitted_inputs.pop_first().unwrap().1;
                record.submitted_inputs.insert("./other.toml".into(), value);
            }
            "overlap" => {
                record.consumers.insert(
                    "./vibe.toml".into(),
                    record.submitted_inputs["./vibe.toml"].original.clone(),
                );
            }
            "removed" => record.submitted_inputs.clear(),
            "extra" => {
                record.submitted_inputs.insert(
                    "./other.toml".into(),
                    record.submitted_inputs["./vibe.toml"].clone(),
                );
            }
            "requirement" => {
                record
                    .submitted_inputs
                    .get_mut("./vibe.toml")
                    .unwrap()
                    .requirement
                    .version = vibe_core::VersionSpec::parse("=2.0.0").unwrap()
            }
            "project-edit" => {
                let p = f.project.join("vibe.toml");
                write(&p, fs::read_to_string(&p).unwrap() + "\n# changed\n");
            }
            "authored" => {
                let p = f.project.join("vibe.toml");
                write(&p,fs::read_to_string(&p).unwrap().replace("link='static-transitive'","link='static-transitive'\nconfiguration={policy='require-selection',default_features=false,selections={extra={intent='exact',members=['disabled']}},parameters={verbose=false}}"));
            }
            "source-binding" => {
                lock.packages
                    .iter_mut()
                    .find(|p| p.name.as_str() == DRIVER)
                    .unwrap()
                    .registry = Some("foreign".into())
            }
            "graph" => record.features.push("enabled".into()),
            _ => {}
        }
        write(
            &f.project.join("vibe.lock"),
            toml::to_string(&lock).unwrap(),
        );
        let read = Lockfile::read(f.project.join("vibe.lock"));
        if matches!(
            mutation,
            "epoch"
                | "owner"
                | "overlap"
                | "removed"
                | "extra"
                | "requirement"
                | "graph"
                | "source-binding"
        ) {
            assert!(read.is_err(), "{mutation}");
            continue;
        }
        let read = read.unwrap();
        let original = &read.configurations[&coordinate(DRIVER)];
        assert!(original.consumers.is_empty());
        assert_eq!(original.submitted_inputs.len(), 1);
        assert_eq!(
            original
                .selected_parameters(vibe_core::PackageKind::Flow)
                .unwrap()["verbose"],
            vibe_core::manifest::ParameterValue::Bool(false)
        );
        let target = f.root.join("writer-disabled.lock");
        assert!(read.write(&target).is_err());
        assert!(!target.exists());
        let root = if mutation == "project-copy" {
            let other = f.root.join("copied-project");
            fs::create_dir_all(&other).unwrap();
            write(
                &other.join("vibe.toml"),
                fs::read(f.project.join("vibe.toml")).unwrap(),
            );
            other
        } else {
            f.project.clone()
        };
        let workspace = vibe_workspace::Workspace::discover(&root).unwrap();
        let fresh = vibe_workspace::freshness::configuration_freshness(&workspace, &read);
        assert_eq!(
            fresh.is_fresh(),
            mutation == "none",
            "{mutation}: {fresh:?}"
        );
    }
}

#[test]
fn submitted_history_stage_a_real_prior_reader_writer_refuse_without_loss() {
    let old = std::env::var_os("NST_SUBMITTED_OLD_CLI")
        .expect("mandatory explicitly mounted pre-submission CLI");
    for operation in ["reader", "writer"] {
        let f = synthetic_submitted_history_fixture();
        assert!(
            f.lock().configurations[&coordinate(DRIVER)]
                .consumers
                .is_empty()
        );
        let before = super::workspace::semantic_snapshot(&f.project);
        let args = if operation == "reader" {
            vec![
                "show",
                "configuration-data",
                "--path",
                f.project.to_str().unwrap(),
                "--json",
            ]
        } else {
            vec![
                "install",
                "--path",
                f.project.to_str().unwrap(),
                "--offline",
                "--assume-yes",
                "--json",
            ]
        };
        let out = std::process::Command::new(&old)
            .env_clear()
            .env("HOME", f.root.join("home"))
            .env("VIBE_SETTINGS", &f.user.settings)
            .env("VIBE_NO_DEFAULT_REGISTRY", "1")
            .env("PATH", "/usr/bin:/bin")
            .args(args)
            .output()
            .unwrap();
        let diagnostic = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        println!(
            "prior submitted {operation} status={} diagnostic={diagnostic}",
            out.status
        );
        assert!(!out.status.success());
        assert!(diagnostic.contains("submitted_inputs"), "{diagnostic}");
        assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
    }
}

#[test]
fn submitted_history_stage_a_source_present_inspection_refuses_drift() {
    for mutation in [
        "none",
        "same-prefix",
        "source-bytes",
        "lock-pin",
        "missing-source",
    ] {
        let f = synthetic_submitted_history_fixture();
        match mutation {
            "same-prefix" => {
                let mut lock = f.lock();
                lock.packages
                    .iter_mut()
                    .find(|row| row.name.as_str() == DRIVER)
                    .unwrap()
                    .source_url = vibe_core::provenance::SourceUrl::new(format!(
                    "file://{}/wrong-source",
                    f.registry.display()
                ));
                write(
                    &f.project.join("vibe.lock"),
                    toml::to_string(&lock).unwrap(),
                );
            }
            "source-bytes" => write(
                &f.registry
                    .join(GROUP)
                    .join(DRIVER)
                    .join("v1.0.0/payload-new.txt"),
                "CHANGED_SAME_VERSION\n",
            ),
            "lock-pin" => {
                let mut lock = f.lock();
                lock.packages
                    .iter_mut()
                    .find(|row| row.name.as_str() == DRIVER)
                    .unwrap()
                    .content_hash = vibe_core::ContentHash::parse("sha256:aa").unwrap();
                write(
                    &f.project.join("vibe.lock"),
                    toml::to_string(&lock).unwrap(),
                );
            }
            "missing-source" => {
                fs::rename(&f.registry, f.root.join("retained-registry-disabled")).unwrap();
            }
            _ => {}
        }
        let before = super::workspace::semantic_snapshot(&f.project);
        let out = f
            .user
            .vibe()
            .args([
                "show",
                "configuration-data",
                "--path",
                f.project.to_str().unwrap(),
                "--json",
            ])
            .output()
            .unwrap();
        let diagnostic = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        println!(
            "submitted source {mutation} status={} diagnostic={diagnostic}",
            out.status
        );
        assert_eq!(out.status.success(), mutation == "none", "{diagnostic}");
        if mutation == "none" {
            let result: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
            assert_eq!(result["packages"][0]["parameters"]["verbose"], false);
        }
        assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
    }
}

fn ordinary_answer_file(f: &Fixture, selected: &str, flag: bool) -> PathBuf {
    let preview = preview_cli(f, None);
    assert!(
        preview.status.success(),
        "{}",
        String::from_utf8_lossy(&preview.stderr)
    );
    let shown: serde_json::Value = serde_json::from_slice(&preview.stdout).unwrap();
    let file = f.root.join("ordinary-answers.json");
    write(&file,serde_json::to_vec(&serde_json::json!({"protocol":"vibe-ordinary-install-answers/1","package":coordinate(DRIVER),"project_witness":shown["project_witness"],"source":shown["source"],"input":{"selections":{"extra":selected},"parameters":{"verbose":flag}}})).unwrap());
    file
}
#[test]
fn ordinary_answer_file_install_and_registry_independent_cold_replay() {
    for world in ["off", "on", "corrupt-store", "missing-store", "no-consent"] {
        let f = preview_fixture();
        let selected = if world == "on" { "enabled" } else { "disabled" };
        let file = ordinary_answer_file(&f, selected, world == "on");
        let mut command = f.user.vibe();
        command.args([
            "install",
            "--ordinary-answers",
            file.to_str().unwrap(),
            "--path",
            f.project.to_str().unwrap(),
            "--offline",
            "--json",
        ]);
        if world != "no-consent" {
            command.arg("--assume-yes");
        }
        let before = super::workspace::semantic_snapshot(&f.project);
        let out = command.output().unwrap();
        let diagnostic = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        println!(
            "ordinary file {world} status={} diagnostic={diagnostic}",
            out.status
        );
        if world == "no-consent" {
            assert!(!out.status.success());
            assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
            continue;
        }
        assert!(out.status.success(), "{diagnostic}");
        let lock = f.lock();
        assert_eq!(lock.meta.schema_version, 16);
        let record = &lock.configurations[&coordinate(DRIVER)];
        assert!(record.consumers.is_empty());
        assert_eq!(record.submitted_inputs.len(), 1);
        assert_eq!(
            record
                .selected_parameters(vibe_core::PackageKind::Flow)
                .unwrap()["verbose"],
            vibe_core::manifest::ParameterValue::Bool(world == "on")
        );
        assert_eq!(f.slot(EXTRA).exists(), world == "on");
        fs::remove_file(&file).unwrap();
        fs::rename(&f.registry, f.root.join("retained-registry-unavailable")).unwrap();
        let store = f
            .user
            .settings
            .join("cache")
            .join(GROUP)
            .join(DRIVER)
            .join("v1.0.0");
        if world == "corrupt-store" {
            write(
                &store.join("unused-source.txt"),
                "CORRUPTED_PAYLOAD_WITH_UNCHANGED_MANIFEST\n",
            );
        }
        if world == "missing-store" {
            fs::rename(&store, f.root.join("retained-store-disabled")).unwrap();
        }
        let before = super::workspace::semantic_snapshot(&f.project);
        let replay = f.install();
        let diagnostic = format!(
            "{}{}",
            String::from_utf8_lossy(&replay.stdout),
            String::from_utf8_lossy(&replay.stderr)
        );
        println!(
            "ordinary replay {world} status={} diagnostic={diagnostic}",
            replay.status
        );
        assert_eq!(
            replay.status.success(),
            matches!(world, "on" | "off"),
            "{diagnostic}"
        );
        assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
    }
}

#[test]
fn ordinary_interactive_install_explicit_apply_cancel_and_post_plan_drift() {
    for mode in ["false", "true", "cancel", "source-drift", "root-drift"] {
        let f = preview_fixture();
        let script = f.root.join("ordinary-install-pty.py");
        write(&script, include_str!("../common/ordinary-install-pty.py"));
        let source = f
            .registry
            .join(GROUP)
            .join(DRIVER)
            .join("v1.0.0/unused-source.txt");
        let source_before = fs::read(&source).unwrap();
        let root_before = fs::read(f.project.join("vibe.toml")).unwrap();
        let before = super::workspace::semantic_snapshot(&f.project);
        let output = std::process::Command::new("python3")
            .args([
                "-I",
                script.to_str().unwrap(),
                env!("CARGO_BIN_EXE_vibe"),
                f.project.to_str().unwrap(),
                mode,
                source.to_str().unwrap(),
            ])
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", f.root.join("home"))
            .env("VIBE_SETTINGS", &f.user.settings)
            .env("VIBE_REGISTRY_CACHE", &f.user.cache)
            .env("VIBEVM_SEARCH_CACHE_DIR", &f.user.search_cache)
            .env("VIBE_NO_DEFAULT_REGISTRY", "1")
            .env("TERM", "xterm")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let evidence: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        println!("ordinary PTY {mode}={evidence}");
        assert_eq!(evidence["terminal_restored"], true);
        assert_eq!(evidence["cursor_visible"], true);
        assert_eq!(evidence["stage"], 3);
        if matches!(mode, "false" | "true") {
            assert_eq!(evidence["status"], 0, "{evidence}");
            let lock = f.lock();
            assert!(
                lock.configurations[&coordinate(DRIVER)]
                    .consumers
                    .is_empty()
            );
            assert_eq!(
                lock.configurations[&coordinate(DRIVER)]
                    .selected_parameters(vibe_core::PackageKind::Flow)
                    .unwrap()["verbose"],
                vibe_core::manifest::ParameterValue::Bool(mode == "true")
            );
            assert!(f.slot(DRIVER).exists());
            assert!(f.static_boot().contains(DRIVER_MARKER));
            assert!(!f.slot(EXTRA).exists());
            assert!(f.hooks().get(BASE).copied().unwrap_or(0) > 0);
            let control = preview_fixture();
            let file = ordinary_answer_file(&control, "disabled", mode == "true");
            let result = control
                .user
                .vibe()
                .args([
                    "install",
                    "--ordinary-answers",
                    file.to_str().unwrap(),
                    "--path",
                    control.project.to_str().unwrap(),
                    "--offline",
                    "--assume-yes",
                    "--json",
                ])
                .output()
                .unwrap();
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
            let other = control.lock();
            assert_eq!(graph(&lock), graph(&other));
            assert_eq!(f.hooks(), control.hooks());
            assert_eq!(
                lock.configurations[&coordinate(DRIVER)].submitted_inputs["./vibe.toml"].original,
                other.configurations[&coordinate(DRIVER)].submitted_inputs["./vibe.toml"].original
            );
            assert_eq!(
                lock.configurations[&coordinate(DRIVER)].features,
                other.configurations[&coordinate(DRIVER)].features
            );
            let installed = super::workspace::semantic_snapshot(&f.project);
            fs::rename(&f.registry, f.root.join("retained-pty-registry-disabled")).unwrap();
            f.install_ok();
            // The registry is nested in this project: account only for the
            // test-owned rename before comparing the replay's semantic world.
            fs::rename(f.root.join("retained-pty-registry-disabled"), &f.registry).unwrap();
            let replayed = super::workspace::semantic_snapshot(&f.project);
            assert_eq!(replayed, installed);
        } else {
            assert_ne!(evidence["status"], 0, "{evidence}");
            if mode == "source-drift" {
                write(&source, &source_before);
            }
            if mode == "root-drift" {
                write(&f.project.join("vibe.toml"), &root_before);
            }
            assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
            assert!(!f.project.join("vibe.lock").exists());
            assert!(!f.project.join("effects.log").exists());
        }
    }
}

#[test]
fn ordinary_answer_file_invalid_null_stale_and_conflicting_subjects_refuse() {
    for world in [
        "null",
        "missing",
        "unknown",
        "false-missing",
        "source-drift",
        "root-drift",
        "authored",
        "cross-project",
    ] {
        let f = preview_fixture();
        let file = ordinary_answer_file(&f, "disabled", false);
        let mut envelope: serde_json::Value =
            serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
        match world {
            "null" => envelope["input"] = serde_json::Value::Null,
            "missing" => {
                envelope.as_object_mut().unwrap().remove("input");
            }
            "unknown" => envelope["unexpected"] = true.into(),
            "false-missing" => {
                envelope["input"]["parameters"]
                    .as_object_mut()
                    .unwrap()
                    .remove("verbose");
            }
            "source-drift" => {
                let p = f
                    .registry
                    .join(GROUP)
                    .join(DRIVER)
                    .join("v1.0.0/unused-source.txt");
                write(
                    &p,
                    fs::read(&p)
                        .unwrap()
                        .into_iter()
                        .chain(b"CHANGED".iter().copied())
                        .collect::<Vec<_>>(),
                );
            }
            "root-drift" => {
                let p = f.project.join("vibe.toml");
                write(&p, fs::read_to_string(&p).unwrap() + "\n# changed root\n");
            }
            "authored" => {
                let p = f.project.join("vibe.toml");
                write(&p,fs::read_to_string(&p).unwrap().replace("link='static-transitive'","link='static-transitive'\nconfiguration={policy='require-selection',default_features=false,selections={extra={intent='exact',members=['disabled']}},parameters={verbose=false}}"));
            }
            "cross-project" => {
                let other = preview_fixture();
                let otherfile = ordinary_answer_file(&other, "disabled", false);
                envelope = serde_json::from_slice(&fs::read(otherfile).unwrap()).unwrap();
            }
            _ => {}
        }
        write(&file, serde_json::to_vec(&envelope).unwrap());
        let before = super::workspace::semantic_snapshot(&f.project);
        let out = f
            .user
            .vibe()
            .args([
                "install",
                "--ordinary-answers",
                file.to_str().unwrap(),
                "--path",
                f.project.to_str().unwrap(),
                "--offline",
                "--assume-yes",
                "--json",
            ])
            .output()
            .unwrap();
        let diagnostic = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        println!(
            "ordinary invalid {world} status={} diagnostic={diagnostic}",
            out.status
        );
        assert!(!out.status.success(), "{diagnostic}");
        assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
        assert!(!f.project.join("vibe.lock").exists());
        assert!(!f.project.join("effects.log").exists());
    }
}

// Qualification only: rejection does not establish ephemeral secret delivery.
#[test]
fn ordinary_secret_boundary_and_diagnostics_qualification() {
    use sha2::{Digest, Sha256};
    fn contains_canary(bytes: &[u8], canary: &str, digest: &str) -> bool {
        bytes.windows(canary.len()).any(|w| w == canary.as_bytes())
            || bytes.windows(digest.len()).any(|w| w == digest.as_bytes())
    }
    fn no_canary(bytes: &[u8], canary: &str, digest: &str, surface: &str) {
        assert!(
            !contains_canary(bytes, canary, digest),
            "synthetic secret disclosure on {surface}; value suppressed"
        );
    }
    fn scan(path: &Path, excluded: &[PathBuf], canary: &str, digest: &str, files: &mut usize) {
        if excluded.iter().any(|p| path == p) {
            return;
        }
        let meta = match fs::symlink_metadata(path) {
            Ok(meta) => meta,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
            Err(_) => panic!("framework scan cannot observe a state entry; details suppressed"),
        };
        if meta.file_type().is_symlink() {
            use std::os::unix::ffi::OsStrExt;
            *files += 1;
            no_canary(
                fs::read_link(path).unwrap().as_os_str().as_bytes(),
                canary,
                digest,
                "framework symlink target",
            );
            return;
        }
        if meta.is_dir() {
            for child in fs::read_dir(path).unwrap() {
                scan(&child.unwrap().path(), excluded, canary, digest, files);
            }
        } else if meta.is_file() {
            *files += 1;
            // Do not print paths: even the diagnostic surface stays value-free.
            no_canary(
                &fs::read(path).unwrap(),
                canary,
                digest,
                "persisted framework state",
            );
        }
    }
    for route in ["authored", "preview", "envelope", "mcp"] {
        for world in [
            "public-bool",
            "public-text",
            "secret-bool-omitted",
            "secret-bool-typed",
            "secret-bool-wrong",
            "secret-text-omitted",
            "secret-text-typed",
            "secret-text-wrong",
        ] {
            if world == "public-text" && route != "authored" {
                continue; // The preview/envelope contract remains bool-only.
            }
            let f = preview_fixture();
            let canary = format!(
                "synthetic-private-answer-{}",
                f.root.file_name().unwrap().to_string_lossy()
            );
            let digest = format!("{:x}", Sha256::digest(canary.as_bytes()));
            assert!(
                contains_canary(canary.as_bytes(), &canary, &digest),
                "raw scanner self-test failed"
            );
            assert!(
                contains_canary(digest.as_bytes(), &canary, &digest),
                "digest scanner self-test failed"
            );
            assert!(
                !contains_canary(b"safe synthetic control", &canary, &digest),
                "scanner negative control failed"
            );
            let input = f.root.join("qualification-input.json");
            // Obtain a real admitted envelope before changing the definition.
            // A later secret refusal is not evidence that this old pin was admitted.
            let envelope_file = ordinary_answer_file(&f, "disabled", false);
            let mut envelope: serde_json::Value =
                serde_json::from_slice(&fs::read(&envelope_file).unwrap()).unwrap();
            let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
            let secret = world.starts_with("secret-");
            let text = world.contains("text");
            let mut definition = fs::read_to_string(&source).unwrap();
            if text {
                definition = definition.replace("type='bool'", "type='string'");
            }
            if secret {
                definition = definition.replace("required=true", "required=true\nsecret=true");
            }
            write(&source, definition);
            let answer = if world.ends_with("omitted") {
                None
            } else if world.ends_with("wrong") {
                Some(if text {
                    serde_json::json!([canary])
                } else {
                    serde_json::json!(canary)
                })
            } else if text {
                Some(serde_json::json!(canary))
            } else {
                Some(serde_json::json!(false))
            };
            let mut parameters = serde_json::Map::new();
            if let Some(value) = &answer {
                parameters.insert("verbose".into(), value.clone());
            }
            let request = f.project.join("vibe.toml");
            let out = match route {
                "authored" => {
                    let parameter = match &answer {
                        None => String::new(),
                        Some(serde_json::Value::Bool(_)) => ",parameters={verbose=false}".into(),
                        Some(serde_json::Value::String(_)) => {
                            format!(",parameters={{verbose='{canary}'}}")
                        }
                        Some(_) => format!(",parameters={{verbose=['{canary}']}}"),
                    };
                    write(&request, fs::read_to_string(&request).unwrap().replace("link='static-transitive'", &format!("link='static-transitive'\nconfiguration={{policy='require-selection',default_features=false,selections={{extra={{intent='exact',members=['disabled']}}}}{parameter}}}")));
                    f.install()
                }
                "preview" => {
                    write(&input, serde_json::to_vec(&serde_json::json!({"selections":{"extra":"disabled"},"parameters":parameters})).unwrap());
                    preview_cli(&f, Some(&input))
                }
                "mcp" => {
                    use std::os::unix::process::ExitStatusExt;
                    let script = serde_json::json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"configuration_preview","arguments":{"package":coordinate(DRIVER),"input":{"selections":{"extra":"disabled"},"parameters":parameters}}}}).to_string()+"\n";
                    let mut out = f
                        .user
                        .vibe()
                        .args([
                            "--offline",
                            "mcp",
                            "serve",
                            "--path",
                            f.project.to_str().unwrap(),
                        ])
                        .write_stdin(script)
                        .output()
                        .unwrap();
                    assert!(
                        out.status.success(),
                        "MCP transport failed; details suppressed"
                    );
                    let rows: Vec<serde_json::Value> = String::from_utf8_lossy(&out.stdout)
                        .lines()
                        .map(|line| serde_json::from_str(line).expect("MCP response shape invalid"))
                        .collect();
                    let row = rows
                        .iter()
                        .find(|row| row["id"] == 1)
                        .expect("MCP response missing");
                    if row.get("error").is_some() || row["result"]["isError"] == true {
                        out.status = std::process::ExitStatus::from_raw(1 << 8);
                    }
                    out
                }
                "envelope" => {
                    envelope["input"]["parameters"] = parameters.into();
                    write(&envelope_file, serde_json::to_vec(&envelope).unwrap());
                    f.user
                        .vibe()
                        .args([
                            "install",
                            "--ordinary-answers",
                            envelope_file.to_str().unwrap(),
                            "--path",
                            f.project.to_str().unwrap(),
                            "--offline",
                            "--assume-yes",
                            "--json",
                        ])
                        .output()
                        .unwrap()
                }
                _ => unreachable!(),
            };
            // Never include the captured diagnostic, raw input or digest in a panic/log.
            if secret {
                no_canary(&out.stdout, &canary, &digest, "stdout");
                no_canary(&out.stderr, &canary, &digest, "stderr");
                let excluded = [
                    request.clone(),
                    input.clone(),
                    envelope_file.clone(),
                    f.registry.clone(),
                ];
                let mut scanned = 0;
                scan(&f.root, &excluded, &canary, &digest, &mut scanned);
                // Include settings, registry/store cache, search cache and coordination records.
                scan(
                    f.user.settings.parent().unwrap(),
                    &[],
                    &canary,
                    &digest,
                    &mut scanned,
                );
                assert!(scanned > 0, "framework scan unexpectedly empty");
                let diagnostic = String::from_utf8_lossy(&out.stdout).to_string()
                    + &String::from_utf8_lossy(&out.stderr);
                assert!(
                    diagnostic.contains("public bool")
                        || diagnostic.contains("public string")
                        || diagnostic.contains("invalid shape")
                        || diagnostic.contains("invalid type")
                        || diagnostic.contains("invalid value")
                        || diagnostic.contains("did not match any variant")
                        || diagnostic.contains("TOML parse error")
                        || diagnostic
                            .contains("secret parameter requires an ephemeral operation input"),
                    "refusal lost safe reason class: {route}/{world}"
                );
                println!(
                    "secret scan route={route} world={world} files-scanned={scanned} lock=absent hooks=absent slots=absent"
                );
                assert!(
                    !out.status.success(),
                    "secret world unexpectedly admitted: {route}/{world}"
                );
                assert!(
                    !f.project.join("vibe.lock").exists(),
                    "secret refusal published history"
                );
                assert!(
                    !f.project.join("effects.log").exists(),
                    "secret refusal ran hooks"
                );
                for name in [DRIVER, BASE, EXTRA] {
                    assert!(!f.slot(name).exists(), "secret refusal published slot");
                }
            } else {
                assert!(out.status.success(), "public control failed: {route}");
                if route != "preview" && route != "mcp" {
                    assert!(f.project.join("vibe.lock").exists());
                }
            }
            println!(
                "secret qualification route={route} world={world} success={} raw-and-digest-scan={}",
                out.status.success(),
                if secret { "clean" } else { "public-control" }
            );
        }
    }
}

#[test]
fn ordinary_decode_safe_errors_cover_cli_envelope_and_registered_mcp() {
    use sha2::{Digest, Sha256};
    for route in ["preview", "envelope", "mcp"] {
        for world in ["unknown-key", "digest-key", "wrong-value", "malformed"] {
            let f = preview_fixture();
            let marker = format!(
                "synthetic-boundary-{}",
                f.root.file_name().unwrap().to_string_lossy()
            );
            let digest = format!("{:x}", Sha256::digest(marker.as_bytes()));
            let mut input = serde_json::json!({"selections":{"extra":"disabled"},"parameters":{"verbose":false}});
            match world {
                "unknown-key" => input[&marker] = true.into(),
                "digest-key" => input[&digest] = true.into(),
                "wrong-value" => input["parameters"]["verbose"] = marker.clone().into(),
                "malformed" => input = marker.clone().into(),
                _ => unreachable!(),
            }
            let out = match route {
                "preview" => {
                    let file = f.root.join("bad-preview.json");
                    let bytes = if world == "malformed" {
                        format!("{{\"{marker}\"").into_bytes()
                    } else {
                        serde_json::to_vec(&input).unwrap()
                    };
                    write(&file, bytes);
                    preview_cli(&f, Some(&file))
                }
                "envelope" => {
                    let file = ordinary_answer_file(&f, "disabled", false);
                    let mut envelope: serde_json::Value =
                        serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
                    envelope["input"] = input;
                    let bytes = if world == "malformed" {
                        format!("{{\"{marker}\"").into_bytes()
                    } else {
                        serde_json::to_vec(&envelope).unwrap()
                    };
                    write(&file, bytes);
                    f.user
                        .vibe()
                        .args([
                            "install",
                            "--ordinary-answers",
                            file.to_str().unwrap(),
                            "--path",
                            f.project.to_str().unwrap(),
                            "--offline",
                            "--assume-yes",
                            "--json",
                        ])
                        .output()
                        .unwrap()
                }
                "mcp" => {
                    let script = serde_json::json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"configuration_preview","arguments":{"package":coordinate(DRIVER),"input":input}}}).to_string()+"\n";
                    f.user
                        .vibe()
                        .args([
                            "--offline",
                            "mcp",
                            "serve",
                            "--path",
                            f.project.to_str().unwrap(),
                        ])
                        .write_stdin(script)
                        .output()
                        .unwrap()
                }
                _ => unreachable!(),
            };
            for bytes in [&out.stdout, &out.stderr] {
                let text = String::from_utf8_lossy(bytes);
                assert!(
                    !text.contains(&marker) && !text.contains(&digest),
                    "decode error disclosure: {route}/{world}; value suppressed"
                );
            }
            let text = String::from_utf8_lossy(&out.stdout).to_string()
                + &String::from_utf8_lossy(&out.stderr);
            assert!(
                text.contains("invalid shape") && text.contains("parameters boolean map"),
                "safe diagnostic missing: {route}/{world}"
            );
            if route != "mcp" {
                assert!(!out.status.success(), "invalid input unexpectedly admitted");
            } else {
                let rows: Vec<serde_json::Value> = String::from_utf8_lossy(&out.stdout)
                    .lines()
                    .map(|line| serde_json::from_str(line).expect("MCP response shape invalid"))
                    .collect();
                let row = rows
                    .iter()
                    .find(|row| row["id"] == 1)
                    .expect("MCP response missing");
                assert!(
                    row.get("error").is_some() || row["result"]["isError"] == true,
                    "MCP invalid argument response missing"
                );
            }
            assert!(!f.project.join("vibe.lock").exists());
            assert!(!f.project.join("effects.log").exists());
            println!(
                "safe decode route={route} world={world} class=invalid-shape schema=static raw-and-digest=clean"
            );
        }
    }
}

#[test]
fn ordinary_feat_explicit_choice_public_install_show_and_cold_replay_qualification() {
    for kind in ["flow", "feat"] {
        for enabled in [false, true] {
            let f = Fixture::new(enabled);
            let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
            if kind == "feat" {
                write(
                    &source,
                    fs::read_to_string(&source)
                        .unwrap()
                        .replace("kind='flow'", "kind='feat'"),
                );
            }
            let selected_manifest = Manifest::read(&source).unwrap();
            assert_eq!(
                selected_manifest.require_package().unwrap().kind,
                if kind == "feat" {
                    vibe_core::PackageKind::Feat
                } else {
                    vibe_core::PackageKind::Flow
                }
            );
            let authored = fs::read(f.project.join("vibe.toml")).unwrap();
            let out = f.install();
            let diagnostic = String::from_utf8_lossy(&out.stdout).to_string()
                + &String::from_utf8_lossy(&out.stderr);
            println!(
                "kind qualification kind={kind} enabled={enabled} install-success={} diagnostic={diagnostic}",
                out.status.success()
            );
            assert!(
                out.status.success(),
                "kind qualification install refused: {kind}"
            );
            if kind == "flow" {
                f.assert_world(enabled);
            } else {
                // Feat acceptance is its selected dependency/physical effect,
                // not Flow's generated boot obligation.
                let lock = f.lock();
                let expected = if enabled {
                    BTreeSet::from([coordinate(DRIVER), coordinate(BASE), coordinate(EXTRA)])
                } else {
                    BTreeSet::from([coordinate(DRIVER), coordinate(BASE)])
                };
                assert_eq!(
                    lock.packages
                        .iter()
                        .map(|p| format!("{}/{}", p.group, p.name))
                        .collect::<BTreeSet<_>>(),
                    expected
                );
                assert_eq!(f.slot(EXTRA).exists(), enabled);
                assert!(f.slot(DRIVER).join("payload.txt").is_file());
                assert!(f.slot(BASE).join("payload.txt").is_file());
                assert_eq!(
                    f.hooks(),
                    if enabled {
                        BTreeMap::from([(BASE.into(), 1), (EXTRA.into(), 1)])
                    } else {
                        BTreeMap::from([(BASE.into(), 1)])
                    }
                );
            }
            let lock = f.lock();
            let row = lock
                .packages
                .iter()
                .find(|row| row.name.as_str() == DRIVER)
                .unwrap();
            assert_eq!(
                row.kind,
                if kind == "feat" {
                    vibe_core::PackageKind::Feat
                } else {
                    vibe_core::PackageKind::Flow
                }
            );
            let expected_edges = if enabled {
                BTreeSet::from([coordinate(BASE), coordinate(EXTRA)])
            } else {
                BTreeSet::from([coordinate(BASE)])
            };
            assert_eq!(
                row.dependencies
                    .iter()
                    .map(|edge| edge.qualified_name())
                    .collect::<BTreeSet<_>>(),
                expected_edges
            );
            let boot = f.static_boot();
            println!(
                "kind boot observation kind={kind} enabled={enabled} driver={} base={} extra={}",
                boot.contains(DRIVER_MARKER),
                boot.contains(BASE_MARKER),
                boot.contains(EXTRA_MARKER)
            );
            let record = &lock.configurations[&coordinate(DRIVER)];
            let selected = if enabled { "enabled" } else { "disabled" };
            assert_eq!(
                record.consumers["./vibe.toml"].selections["extra"]
                    .members
                    .as_deref(),
                Some(&[selected.to_owned()][..])
            );
            assert!(record.submitted_inputs.is_empty());
            let data = f
                .user
                .vibe()
                .args([
                    "show",
                    "configuration-data",
                    "--path",
                    f.project.to_str().unwrap(),
                    "--json",
                ])
                .output()
                .unwrap();
            assert!(
                data.status.success(),
                "kind configuration-data refused: {kind}"
            );
            let data: serde_json::Value = serde_json::from_slice(&data.stdout).unwrap();
            assert!(
                data["packages"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|p| p["package"] == coordinate(DRIVER)),
                "configured package absent from public view"
            );
            let before_lock = fs::read(f.project.join("vibe.lock")).unwrap();
            let before = super::workspace::semantic_snapshot(&f.project);
            let hooks = f.hooks();
            f.install_ok();
            assert_eq!(fs::read(f.project.join("vibe.lock")).unwrap(), before_lock);
            assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
            assert_eq!(f.hooks(), hooks);
            assert_eq!(fs::read(f.project.join("vibe.toml")).unwrap(), authored);
            println!(
                "kind qualification kind={kind} enabled={enabled} show=ok cold-replay=unchanged originals=authored physical-graph=matched"
            );
        }
    }
}

#[test]
fn ordinary_remaining_kinds_install_show_and_cold_replay_qualification() {
    for kind in ["stack", "tool", "mcp", "lang", "app"] {
        for enabled in [false, true] {
            let f = Fixture::new(enabled);
            let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
            let mut text = fs::read_to_string(&source)
                .unwrap()
                .replace("kind='flow'", &format!("kind='{kind}'"));
            if kind == "mcp" {
                // Valid remote declaration only: no registration/server/tool launch.
                text = text.replace("^1", "=1.0.0");
                text.push_str("\n[[mcp_server]]\nname='synthetic-never-started'\nurl='https://example.invalid/mcp'\ntransport='streamable-http'\n");
            }
            if kind == "app" {
                text.push_str("\n[application]\nid='synthetic-ordinary-app'\n");
            }
            write(&source, text);
            let manifest = Manifest::read(&source).expect("kind-specific fixture must be valid");
            assert_eq!(manifest.require_package().unwrap().kind.to_string(), kind);
            let before = super::workspace::semantic_snapshot(&f.project);
            let authored = fs::read(f.project.join("vibe.toml")).unwrap();
            let out = f.install();
            let diagnostic = String::from_utf8_lossy(&out.stdout).to_string()
                + &String::from_utf8_lossy(&out.stderr);
            println!(
                "remaining kind={kind} enabled={enabled} install-success={} diagnostic={diagnostic}",
                out.status.success()
            );
            let app_refusal = kind == "app";
            if app_refusal {
                assert!(
                    !out.status.success() && diagnostic.contains("not globally installable"),
                    "App preflight refusal class changed"
                );
                assert!(
                    super::workspace::semantic_snapshot(&f.project) == before,
                    "App preflight changed package state"
                );
                assert!(!f.project.join("vibe.lock").exists());
                assert!(!f.project.join("effects.log").exists());
                assert!(!f.project.join("vibevm/vibeopt").exists());
                println!(
                    "remaining kind=app enabled={enabled} result=preflight-refused effects=absent"
                );
                continue;
            } else {
                assert!(
                    out.status.success(),
                    "ordinary kind unexpectedly refused: {kind}"
                );
            }
            let lock = f.lock();
            let row = lock
                .packages
                .iter()
                .find(|row| row.name.as_str() == DRIVER)
                .unwrap();
            assert_eq!(row.kind.to_string(), kind);
            let installed = Manifest::read(f.slot(DRIVER).join("vibe.toml")).unwrap();
            assert_eq!(installed.require_package().unwrap().kind.to_string(), kind);
            let expected_edges = if enabled {
                BTreeSet::from([coordinate(BASE), coordinate(EXTRA)])
            } else {
                BTreeSet::from([coordinate(BASE)])
            };
            assert_eq!(
                row.dependencies
                    .iter()
                    .map(|edge| edge.qualified_name())
                    .collect::<BTreeSet<_>>(),
                expected_edges
            );
            let expected = if enabled {
                BTreeSet::from([coordinate(DRIVER), coordinate(BASE), coordinate(EXTRA)])
            } else {
                BTreeSet::from([coordinate(DRIVER), coordinate(BASE)])
            };
            assert_eq!(
                lock.packages
                    .iter()
                    .map(|p| format!("{}/{}", p.group, p.name))
                    .collect::<BTreeSet<_>>(),
                expected
            );
            assert_eq!(f.slot(EXTRA).exists(), enabled);
            assert!(f.slot(DRIVER).join("payload.txt").exists());
            assert!(f.slot(BASE).join("payload.txt").exists());
            let record = &lock.configurations[&coordinate(DRIVER)];
            let selected = if enabled { "enabled" } else { "disabled" };
            assert_eq!(
                record.consumers["./vibe.toml"].selections["extra"]
                    .members
                    .as_deref(),
                Some(&[selected.to_owned()][..])
            );
            let data = f
                .user
                .vibe()
                .args([
                    "show",
                    "configuration-data",
                    "--path",
                    f.project.to_str().unwrap(),
                    "--json",
                ])
                .output()
                .unwrap();
            assert!(
                data.status.success(),
                "kind public inspection refused after successful install: {kind}"
            );
            let data: serde_json::Value = serde_json::from_slice(&data.stdout).unwrap();
            assert!(
                data["packages"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|p| p["package"] == coordinate(DRIVER))
            );
            let before_lock = fs::read(f.project.join("vibe.lock")).unwrap();
            let before = super::workspace::semantic_snapshot(&f.project);
            let hooks = f.hooks();
            let boot = f.static_boot();
            if app_refusal {
                let replay = f.install();
                let diagnostic = String::from_utf8_lossy(&replay.stdout).to_string()
                    + &String::from_utf8_lossy(&replay.stderr);
                assert!(
                    !replay.status.success() && diagnostic.contains("not_globally_installable"),
                    "App replay refusal class changed"
                );
            } else {
                f.install_ok();
            }
            assert_eq!(fs::read(f.project.join("vibe.lock")).unwrap(), before_lock);
            assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
            assert_eq!(f.hooks(), hooks);
            assert_eq!(fs::read(f.project.join("vibe.toml")).unwrap(), authored);
            println!(
                "remaining kind={kind} enabled={enabled} whole-success={} package-state=published show=ok replay-package-state=unchanged boot-driver={} boot-base={} boot-extra={} hook-base={} hook-extra={}",
                !app_refusal,
                boot.contains(DRIVER_MARKER),
                boot.contains(BASE_MARKER),
                boot.contains(EXTRA_MARKER),
                hooks.get(BASE).copied().unwrap_or(0),
                hooks.get(EXTRA).copied().unwrap_or(0)
            );
        }
    }
}

#[test]
fn project_app_preflight_preserves_fresh_and_historically_partial_worlds() {
    let old = std::env::var("NST_APP_PREFLIGHT_OLD_CLI")
        .expect("exact historical preflight CLI required");
    for enabled in [false, true] {
        for history in [false, true] {
            for explicit in [false, true] {
                let f = Fixture::new(enabled);
                let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
                write(
                    &source,
                    fs::read_to_string(&source)
                        .unwrap()
                        .replace("kind='flow'", "kind='app'")
                        + "\n[application]\nid='synthetic-preflight'\n",
                );
                if history {
                    let out = std::process::Command::new(&old)
                        .env_clear()
                        .env("HOME", f.root.join("home"))
                        .env("USERPROFILE", f.root.join("home"))
                        .env("VIBE_SETTINGS", &f.user.settings)
                        .env("VIBE_REGISTRY_CACHE", &f.user.cache)
                        .env("VIBEVM_SEARCH_CACHE_DIR", &f.user.search_cache)
                        .env("VIBE_NO_DEFAULT_REGISTRY", "1")
                        .env("PATH", "/usr/bin:/bin")
                        .current_dir(&f.project)
                        .args([
                            "install",
                            "--offline",
                            "--assume-yes",
                            "--json",
                            "--path",
                            f.project.to_str().unwrap(),
                        ])
                        .output()
                        .unwrap();
                    assert!(!out.status.success(), "prior CLI unexpectedly succeeds");
                    assert!(
                        f.project.join("vibe.lock").exists(),
                        "prior did not construct actual partial subject"
                    );
                    assert!(f.slot(DRIVER).exists());
                }
                let before = super::workspace::semantic_snapshot(&f.project);
                let before_home = preview_scratch_snapshot(&f.project.join("vibevm/vibeopt"));
                let mut cmd = f.user.vibe();
                cmd.args([
                    "install",
                    "--offline",
                    "--assume-yes",
                    "--json",
                    "--path",
                    f.project.to_str().unwrap(),
                ]);
                if explicit {
                    cmd.arg(format!("{GROUP}/{DRIVER}@=1.0.0"));
                }
                let out = cmd.output().unwrap();
                let diagnostic = String::from_utf8_lossy(&out.stdout).to_string()
                    + &String::from_utf8_lossy(&out.stderr);
                assert!(
                    !out.status.success() && diagnostic.contains("not globally installable"),
                    "project app preflight refusal missing"
                );
                assert!(
                    super::workspace::semantic_snapshot(&f.project) == before,
                    "preflight changed lock/manifest/slot/boot/hook state"
                );
                assert_eq!(
                    preview_scratch_snapshot(&f.project.join("vibevm/vibeopt")),
                    before_home
                );
                println!(
                    "app preflight enabled={enabled} historical={history} explicit={explicit} refused=true package-state=unchanged app-home=unchanged"
                );
            }
        }
    }
}

#[test]
fn project_app_missing_identity_refuses_without_package_publication() {
    for explicit in [false, true] {
        let f = Fixture::new(false);
        let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
        write(
            &source,
            fs::read_to_string(&source)
                .unwrap()
                .replace("kind='flow'", "kind='app'"),
        );
        let before = super::workspace::semantic_snapshot(&f.project);
        let mut cmd = f.user.vibe();
        cmd.args([
            "install",
            "--offline",
            "--assume-yes",
            "--json",
            "--path",
            f.project.to_str().unwrap(),
        ]);
        if explicit {
            cmd.arg(format!("{GROUP}/{DRIVER}@=1.0.0"));
        }
        let out = cmd.output().unwrap();
        let diagnostic = String::from_utf8_lossy(&out.stdout).to_string()
            + &String::from_utf8_lossy(&out.stderr);
        assert!(!out.status.success() && diagnostic.contains("declares no [application]"));
        assert!(
            super::workspace::semantic_snapshot(&f.project) == before,
            "malformed App published package state"
        );
        assert!(!f.project.join("vibevm/vibeopt").exists());
    }
}

fn read_consent_data_definition() -> &'static str {
    "\n[configuration.config_values]\nselected=true\n[[configuration.features]]\nid='off'\nname='Off'\n[[configuration.features]]\nid='on'\nname='On'\neffects=[{kind='config-value',name='selected'}]\n[[configuration.groups]]\nmode='choice'\nid='mode'\nname='Mode'\ncardinality='exactly-one'\nmembers=['off','on']\n"
}
fn read_consent_kernel_choice(
    manifest: &Manifest,
    on: bool,
) -> BTreeMap<String, vibe_core::manifest::ParameterValue> {
    let intent: vibe_core::manifest::ConfigurationIntentDecl = toml::from_str(&format!("policy='require-selection'\ndefault_features=false\nselections.mode={{intent='exact',members=['{}']}}\n",if on {"on"} else {"off"})).unwrap();
    let record = vibe_core::manifest::PackageConfiguration::evaluate(
        manifest,
        &BTreeMap::from([("./vibe.toml".into(), intent)]),
    )
    .unwrap();
    record
        .selected_data(manifest.require_package().unwrap().kind)
        .unwrap()
}

#[test]
fn registered_doc_read_data_choice_returns_source_bound_values() {
    let f = Fixture::new(false);
    // This subject uses no package installation; the Doc is authored in-tree.
    let docs = f.project.join("vibevm/vibepacks/org.example/manual/v1.0.0");
    let source = docs.join("vibe.toml");
    write(
        &source,
        format!(
            "[package]\ngroup='org.example'\nname='manual'\nversion='1.0.0'\nkind='doc'\ntitle='Manual'\nabstract='A valid synthetic documentation page.'\n[[documents]]\npackage='org.example/subject'\nversion='*'\n{}",
            read_consent_data_definition()
        ),
    );
    write(
        &docs.join("vibevm/vibespecs/guide.xml"),
        "<spec xmlns='https://vibevm.org/spec/1'><title id='root'>Manual</title><p>DOC_READ_SYNTHETIC_PAGE</p></spec>",
    );
    let listing_script =
        serde_json::json!({"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}).to_string()
            + "\n";
    let listed = f
        .user
        .vibe()
        .args([
            "--offline",
            "mcp",
            "serve",
            "--path",
            f.project.to_str().unwrap(),
        ])
        .write_stdin(listing_script)
        .output()
        .unwrap();
    assert!(listed.status.success());
    let rows: Vec<serde_json::Value> = String::from_utf8(listed.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let descriptor = rows.iter().find(|r| r["id"] == 1).unwrap()["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|tool| tool["name"] == "read_doc")
        .unwrap();
    assert_eq!(descriptor["inputSchema"]["additionalProperties"], false);
    assert!(
        descriptor["inputSchema"]["properties"]
            .get("input")
            .is_some()
    );
    let manifest = Manifest::read(&source).unwrap();
    assert!(read_consent_kernel_choice(&manifest, false).is_empty());
    assert_eq!(
        read_consent_kernel_choice(&manifest, true)["selected"],
        vibe_core::manifest::ParameterValue::Bool(true)
    );
    let before = preview_scratch_snapshot(&f.project);
    let settings_before = preview_scratch_snapshot(f.user.settings.parent().unwrap());
    let call = |on: Option<bool>| {
        let mut args =
            serde_json::json!({"address":"spec://org.example/manual/guide","format":"md"});
        if let Some(on) = on {
            args["input"] = serde_json::json!({"selections":{"mode":{"intent":"exact","members":[if on {"on"} else {"off"}]}}});
        }
        let script=serde_json::json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"read_doc","arguments":args}}).to_string()+"\n";
        let out = f
            .user
            .vibe()
            .args([
                "--offline",
                "mcp",
                "serve",
                "--path",
                f.project.to_str().unwrap(),
            ])
            .write_stdin(script)
            .output()
            .unwrap();
        assert!(out.status.success());
        String::from_utf8(out.stdout)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
            .find(|row| row["id"] == 1)
            .unwrap()
    };
    let base = call(None);
    assert_ne!(base["result"]["isError"], true, "valid Doc read refused");
    let content = &base["result"]["structuredContent"];
    assert!(
        content.get("selected_values").is_none(),
        "no-input Doc response acquired new field"
    );
    assert_eq!(
        call(None),
        base,
        "fresh-process no-input Doc response changed"
    );
    assert_eq!(content["source"], "in-tree");
    assert_eq!(content["format"], "md");
    assert!(
        content["page"]
            .as_str()
            .unwrap()
            .contains("DOC_READ_SYNTHETIC_PAGE")
    );
    for on in [false, true] {
        let attempted = call(Some(on));
        assert_ne!(attempted["result"]["isError"], true, "answered Doc refused");
        let mut actual = attempted["result"]["structuredContent"].clone();
        let values = actual
            .as_object_mut()
            .unwrap()
            .remove("selected_values")
            .expect("selected Doc values missing");
        assert_eq!(
            values,
            if on {
                serde_json::json!({"selected":true})
            } else {
                serde_json::json!({})
            }
        );
        assert_eq!(
            actual, base["result"]["structuredContent"],
            "Doc page/provenance changed under request-local choice"
        );
        println!(
            "doc registered choice={on} kernel-data-changed=true public-input=consumed selected-values=separate page/provenance=unchanged"
        );
    }
    assert_eq!(preview_scratch_snapshot(&f.project), before);
    assert_eq!(
        preview_scratch_snapshot(f.user.settings.parent().unwrap()),
        settings_before
    );
    assert!(!f.project.join("vibe.lock").exists());
    assert!(!f.project.join("effects.log").exists());
}

#[test]
fn offline_remap_read_and_explicit_consent_do_not_consume_common_choice() {
    for on in [false, true] {
        let f = Fixture::new(false);
        let authored = f.root.join("authored-remap.toml");
        write(
            &authored,
            format!(
                "[package]\ngroup='org.example'\nname='policy'\nversion='1.0.0'\nkind='remap'\n[[remap]]\nprefix='https://example.invalid/input/'\ndeny=true\nreason='Synthetic deny offer'\n{}",
                read_consent_data_definition()
            ),
        );
        let manifest = Manifest::read(&authored).unwrap();
        let selected = read_consent_kernel_choice(&manifest, on);
        assert_eq!(selected.contains_key("selected"), on);
        // Explicit typed fixture seed, not a claim of actual installation.
        let offer = vibe_core::remap::offer_from_manifest(&manifest, "sha256:abc").unwrap();
        let mut installed = vibe_core::remap::InstalledRemaps::default();
        installed.upsert(vibe_core::remap::InstalledRemap {
            package: offer,
            registry: None,
            installed: "2026-11-20T00:00:00Z".into(),
        });
        installed.store(&f.user.settings).unwrap();
        let before_project = preview_scratch_snapshot(&f.project);
        let before_home = preview_scratch_snapshot(f.user.settings.parent().unwrap());
        let run = |args: &[&str]| {
            f.user
                .vibe()
                .current_dir(&f.project)
                .args(["--offline"])
                .args(args)
                .output()
                .unwrap()
        };
        let list = run(&["remap", "list", "--json"]);
        assert!(list.status.success());
        let text = String::from_utf8(list.stdout).unwrap();
        assert!(text.contains("offered"));
        let explain = run(&[
            "remap",
            "explain",
            "https://example.invalid/input/file",
            "--json",
        ]);
        assert!(explain.status.success());
        let before_explain: serde_json::Value = serde_json::from_slice(&explain.stdout).unwrap();
        assert_eq!(
            preview_scratch_snapshot(f.user.settings.parent().unwrap()),
            before_home,
            "read operation wrote consent or state"
        );
        let rejected_input = run(&[
            "remap",
            "list",
            "--json",
            "--ordinary-answers",
            authored.to_str().unwrap(),
        ]);
        assert!(
            !rejected_input.status.success(),
            "read route unexpectedly accepts install choice grammar"
        );
        assert_eq!(
            preview_scratch_snapshot(f.user.settings.parent().unwrap()),
            before_home
        );
        let accept = run(&["remap", "accept", "org.example/policy", "--json"]);
        assert!(accept.status.success());
        let ledger = vibe_core::remap::ConsentLedger::load(&f.user.settings).unwrap();
        assert_eq!(
            ledger.state("org.example/policy", "sha256:abc"),
            vibe_core::remap::OfferState::Accepted
        );
        let accepted_home = preview_scratch_snapshot(f.user.settings.parent().unwrap());
        let list = run(&["remap", "list", "--json"]);
        assert!(list.status.success());
        assert!(String::from_utf8(list.stdout).unwrap().contains("accepted"));
        let explain = run(&[
            "remap",
            "explain",
            "https://example.invalid/input/file",
            "--json",
        ]);
        assert!(explain.status.success());
        let after_explain: serde_json::Value = serde_json::from_slice(&explain.stdout).unwrap();
        assert_ne!(
            before_explain, after_explain,
            "explicit consent did not change active policy projection"
        );
        assert_eq!(
            preview_scratch_snapshot(f.user.settings.parent().unwrap()),
            accepted_home,
            "read after acceptance mutated state"
        );
        assert_eq!(preview_scratch_snapshot(&f.project), before_project);
        assert!(!f.project.join("vibe.lock").exists());
        assert!(!f.project.join("effects.log").exists());
        assert!(
            !f.project
                .join(vibe_core::layout::current_vibedeps_root())
                .exists()
        );
        let reject = run(&["remap", "reject", "org.example/policy", "--json"]);
        assert!(reject.status.success());
        let ledger = vibe_core::remap::ConsentLedger::load(&f.user.settings).unwrap();
        assert_eq!(
            ledger.state("org.example/policy", "sha256:abc"),
            vibe_core::remap::OfferState::Rejected
        );
        let rejected_home = preview_scratch_snapshot(f.user.settings.parent().unwrap());
        assert!(run(&["remap", "list", "--json"]).status.success());
        assert_eq!(
            preview_scratch_snapshot(f.user.settings.parent().unwrap()),
            rejected_home
        );
        println!(
            "remap choice={on} kernel-data-present={on} list/explain=read-only offer=seeded explicit-acceptance=active explicit-rejection=inactive common-input=unsupported"
        );
    }
}

#[test]
fn registered_doc_request_local_data_refuses_invalid_and_ambiguous_sources() {
    for world in [
        "unknown",
        "null",
        "wrong-type",
        "extra-member",
        "lang",
        "two-versions",
        "pinned-old",
        "mismatched-name",
        "invalid-version-directory",
    ] {
        let f = Fixture::new(false);
        let docs = f.project.join("vibevm/vibepacks/org.example/manual/v1.0.0");
        let manifest = format!(
            "[package]\ngroup='org.example'\nname='manual'\nversion='1.0.0'\nkind='doc'\ntitle='Manual'\nabstract='Synthetic valid manual source.'\n[[documents]]\npackage='org.example/subject'\nversion='*'\n{}",
            read_consent_data_definition()
        );
        write(&docs.join("vibe.toml"), &manifest);
        write(
            &docs.join("vibevm/vibespecs/guide.xml"),
            "<spec xmlns='https://vibevm.org/spec/1'><title id='root'>Manual</title><p>DOC_BOUND_PAGE</p></spec>",
        );
        let mut args = serde_json::json!({"address":"spec://org.example/manual/guide","input":{"selections":{"mode":{"intent":"exact","members":["on"]}}}});
        match world {
            "unknown" => args["foreign"] = true.into(),
            "null" => args["input"] = serde_json::Value::Null,
            "wrong-type" => args["input"]["selections"]["mode"]["members"] = true.into(),
            "extra-member" => {
                args["input"]["selections"]["mode"]["members"] = serde_json::json!(["off", "on"])
            }
            "lang" => args["lang"] = "en".into(),
            "two-versions" | "pinned-old" => {
                let newer = docs.parent().unwrap().join("v2.0.0");
                write(
                    &newer.join("vibe.toml"),
                    manifest.replace("version='1.0.0'", "version='2.0.0'"),
                );
                write(
                    &newer.join("vibevm/vibespecs/guide.xml"),
                    "<spec xmlns='https://vibevm.org/spec/1'><title id='root'>Newer</title><p>OTHER_PAGE</p></spec>",
                );
                if world == "pinned-old" {
                    args["address"] = "spec://org.example/manual@=1.0.0/guide".into();
                }
            }
            "mismatched-name" => write(
                &docs.join("vibe.toml"),
                manifest.replace("name='manual'", "name='different'"),
            ),
            "invalid-version-directory" => {
                let duplicate = docs.parent().unwrap().join("v01.0.0");
                write(&duplicate.join("vibe.toml"), &manifest);
            }
            _ => {}
        }
        let before = preview_scratch_snapshot(&f.project);
        let home = preview_scratch_snapshot(f.user.settings.parent().unwrap());
        let script=serde_json::json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"read_doc","arguments":args}}).to_string()+"\n";
        let out = f
            .user
            .vibe()
            .args([
                "--offline",
                "mcp",
                "serve",
                "--path",
                f.project.to_str().unwrap(),
            ])
            .write_stdin(script)
            .output()
            .unwrap();
        assert!(out.status.success());
        let row = String::from_utf8(out.stdout)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
            .find(|r| r["id"] == 1)
            .unwrap();
        if world == "pinned-old" {
            assert_ne!(
                row["result"]["isError"], true,
                "exact pin failed to select actual older Doc"
            );
            assert!(
                row["result"]["structuredContent"]["page"]
                    .as_str()
                    .unwrap()
                    .contains("DOC_BOUND_PAGE")
            );
            assert_eq!(
                row["result"]["structuredContent"]["selected_values"]["selected"],
                true
            );
        } else {
            assert!(
                row.get("error").is_some() || row["result"]["isError"] == true,
                "Doc refusal missing: {world}"
            );
            assert!(
                row["result"].get("structuredContent").is_none(),
                "Doc refusal returned partial values/page"
            );
        }
        assert_eq!(preview_scratch_snapshot(&f.project), before);
        assert_eq!(
            preview_scratch_snapshot(f.user.settings.parent().unwrap()),
            home
        );
        println!(
            "Doc source world={world} admitted={} state=unchanged",
            world == "pinned-old"
        );
    }
}

#[test]
fn remap_preview_local_owned_offer_keeps_choice_separate_from_consent() {
    for world in [
        "valid",
        "stale-manifest",
        "stale-payload",
        "stale-index",
        "missing-tree",
        "duplicate-offer",
        "symlink",
        "invalid-input",
    ] {
        let f = Fixture::new(false);
        let tree = vibe_core::remap::package_dir(&f.user.settings, "org.example/policy");
        write(
            &tree.join("vibe.toml"),
            format!(
                "[package]\ngroup='org.example'\nname='policy'\nversion='1.0.0'\nkind='remap'\n[[remap]]\nprefix='https://example.invalid/input/'\ndeny=true\nreason='Synthetic offer'\n{}",
                read_consent_data_definition()
            ),
        );
        write(&tree.join("payload.txt"), "original local offer bytes");
        let manifest = Manifest::read(tree.join("vibe.toml")).unwrap();
        let pin = vibe_registry::compute_content_hash(&tree).unwrap();
        let offer = vibe_core::remap::offer_from_manifest(&manifest, &pin).unwrap();
        let mut installed = vibe_core::remap::InstalledRemaps::default();
        installed.upsert(vibe_core::remap::InstalledRemap {
            package: offer,
            registry: None,
            installed: "2026-11-20T00:00:00Z".into(),
        });
        if world == "duplicate-offer" {
            installed.packages.push(installed.packages[0].clone());
        }
        if world == "stale-index" {
            installed.packages[0].package.section.description = Some("different index".into());
        }
        installed.store(&f.user.settings).unwrap();
        match world {
            "stale-manifest" => write(
                &tree.join("vibe.toml"),
                fs::read_to_string(tree.join("vibe.toml")).unwrap() + "\n# source changed\n",
            ),
            "stale-payload" => write(&tree.join("payload.txt"), "changed local bytes"),
            "missing-tree" => {
                fs::rename(&tree, f.root.join("retained-tree")).unwrap();
            }
            "symlink" => {
                fs::remove_file(tree.join("payload.txt")).unwrap();
                std::os::unix::fs::symlink(f.root.join("elsewhere"), tree.join("payload.txt"))
                    .unwrap();
            }
            _ => {}
        }
        let input = f.root.join("preview-input.json");
        let run = |args: &[&str]| {
            f.user
                .vibe()
                .current_dir(&f.project)
                .args(["--offline"])
                .args(args)
                .output()
                .unwrap()
        };
        for state in ["offered", "accepted", "rejected"] {
            if world != "valid" && state != "offered" {
                continue;
            }
            if state != "offered" {
                let verb = if state == "accepted" {
                    "accept"
                } else {
                    "reject"
                };
                assert!(
                    run(&["remap", verb, "org.example/policy", "--json"])
                        .status
                        .success()
                );
            }
            for on in [false, true] {
                write(
                    &input,
                    if world == "invalid-input" {
                        b"{\"selections\":null}".to_vec()
                    } else {
                        serde_json::to_vec(&serde_json::json!({"selections":{"mode":{"intent":"exact","members":[if on{"on"}else{"off"}]}}})).unwrap()
                    },
                );
                let project = preview_scratch_snapshot(&f.project);
                let home = preview_scratch_snapshot(f.user.settings.parent().unwrap());
                let listing = run(&["remap", "list", "--json"]);
                assert!(listing.status.success());
                let explain = run(&[
                    "remap",
                    "explain",
                    "https://example.invalid/input/item",
                    "--json",
                ]);
                assert!(explain.status.success());
                let out = run(&[
                    "remap",
                    "preview",
                    "org.example/policy",
                    "--input",
                    input.to_str().unwrap(),
                    "--json",
                ]);
                if world == "valid" {
                    assert!(out.status.success(), "valid preview refused");
                    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
                    assert_eq!(value["protocol"], "vibe-remap-preview/1");
                    assert_eq!(value["source"]["kind"], "local-owned-offer");
                    assert_eq!(value["source"]["content_hash"], pin);
                    assert_eq!(
                        value["selected_values"],
                        if on {
                            serde_json::json!({"selected":true})
                        } else {
                            serde_json::json!({})
                        }
                    );
                    assert!(value["source"].get("source_url").is_none());
                } else {
                    assert!(
                        !out.status.success(),
                        "invalid local preview admitted: {world}"
                    );
                }
                assert_eq!(run(&["remap", "list", "--json"]).stdout, listing.stdout);
                assert_eq!(
                    run(&[
                        "remap",
                        "explain",
                        "https://example.invalid/input/item",
                        "--json"
                    ])
                    .stdout,
                    explain.stdout
                );
                assert_eq!(preview_scratch_snapshot(&f.project), project);
                assert_eq!(
                    preview_scratch_snapshot(f.user.settings.parent().unwrap()),
                    home
                );
                println!(
                    "remap preview world={world} state={state} on={on} admitted={} project/home/policy=unchanged",
                    world == "valid"
                );
            }
        }
    }
}

#[test]
fn guarded_automatic_defaults_actual_public_qualification() {
    for guarded in [true, false] {
        for gate in [false, true] {
            for world in [
                "omitted-defaults",
                "omitted-required",
                "matching",
                "contrary",
            ] {
                let f = Fixture::new(false);
                let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
                let guard = if guarded {
                    "guards=[{member='gate-on'}]\n"
                } else {
                    ""
                };
                write(
                    &source,
                    package_header(DRIVER)
                        + &format!(
                            "\n[requires.packages]\n'org.vibevm/oracle-base'='=1.0.0'\n'org.vibevm/oracle-extra'={{version='=1.0.0',optional=true}}\n[[configuration.features]]\nid='gate-off'\nname='Gate off'\n[[configuration.features]]\nid='gate-on'\nname='Gate on'\n[[configuration.features]]\nid='child-off'\nname='Child off'\n[[configuration.features]]\nid='child-on'\nname='Child on'\neffects=[{{kind='dependency',target='org.vibevm/oracle-extra'}}]\n[[configuration.groups]]\nmode='choice'\nid='gate'\nname='Gate'\ncardinality='exactly-one'\nmembers=['gate-off','gate-on']\n[[configuration.groups]]\nmode='choice'\nid='child'\nname='Child'\ncardinality='exactly-one'\nmembers=['child-off','child-on']\ndefaults=['child-on']\n{guard}"
                        ),
                );
                let policy = if world == "omitted-required" {
                    "require-selection"
                } else {
                    "use-declared-defaults"
                };
                let child = match world {
                    "matching" => ",child={intent='exact',members=['child-on']}",
                    "contrary" => ",child={intent='exact',members=['child-off']}",
                    _ => "",
                };
                write(
                    &f.project.join("vibe.toml"),
                    format!(
                        "[project]\nname='guarded-default'\nversion='0.1.0'\n[requires.packages.'{GROUP}/{DRIVER}']\nversion='=1.0.0'\nlink='static-transitive'\nconfiguration={{policy='{policy}',default_features=false,selections={{gate={{intent='exact',members=['{}']}}{child}}}}}\n[[registry]]\nname='fixture'\nurl='file://{}'\n",
                        if gate { "gate-on" } else { "gate-off" },
                        f.registry.display()
                    ),
                );
                let manifest = Manifest::read(&source).unwrap();
                let root = Manifest::read(f.project.join("vibe.toml")).unwrap();
                let original = &root.requires.configurations[&coordinate(DRIVER)];
                let definition = manifest.configuration.as_ref().unwrap();
                let normalized = definition.normalize_input(original, "./vibe.toml").unwrap();
                let evaluated = definition
                    .compile_choices()
                    .unwrap()
                    .evaluate(&normalized.features, &[normalized.choices]);
                let active = !guarded || gate;
                let child_control = evaluated.controls.iter().find(|c| c.id == "child").unwrap();
                assert_eq!(child_control.active, active);
                let omitted = world.starts_with("omitted");
                assert_eq!(original.selections.contains_key("child"), !omitted);
                let default_used = active && world == "omitted-defaults";
                assert_eq!(
                    evaluated.defaults.iter().any(|d| d.group == "child"),
                    default_used
                );
                assert_eq!(
                    evaluated.questions.iter().any(|q| q.group == "child"),
                    active && world == "omitted-required"
                );
                let before = super::workspace::semantic_snapshot(&f.project);
                let out = f.install();
                let diagnostic = String::from_utf8_lossy(&out.stdout).to_string()
                    + &String::from_utf8_lossy(&out.stderr);
                let expected_success = !(active && world == "omitted-required");
                println!(
                    "guarded-default guarded={guarded} gate={gate} world={world} active={active} default-used={default_used} install-success={} diagnostic={diagnostic}",
                    out.status.success()
                );
                assert_eq!(
                    out.status.success(),
                    expected_success,
                    "observed boundary differs guarded={guarded} gate={gate} world={world}"
                );
                if !out.status.success() {
                    assert!(super::workspace::semantic_snapshot(&f.project) == before);
                    assert!(!f.project.join("vibe.lock").exists());
                    assert!(!f.project.join("effects.log").exists());
                    assert!(diagnostic.contains("requires configuration answers"));
                    continue;
                }
                let extra = active && world != "contrary";
                let lock = f.lock();
                let record = &lock.configurations[&coordinate(DRIVER)];
                assert_eq!(record.consumers["./vibe.toml"], *original);
                if guarded && !gate && world == "omitted-defaults" {
                    let basis = record
                        .automatic_basis
                        .as_ref()
                        .expect("dormant reason missing");
                    assert_eq!(
                        basis.consumer_dormant_choices["./vibe.toml"],
                        vec!["child".to_owned()]
                    );
                    assert!(
                        basis.consumer_choices.is_empty() && basis.consumer_choice_sets.is_empty()
                    );
                    assert_eq!(lock.meta.schema_version, 17);
                } else if default_used {
                    let basis = record
                        .automatic_basis
                        .as_ref()
                        .expect("actual default reliance missing");
                    assert_eq!(basis.consumer_choices["./vibe.toml"]["child"], "child-on");
                    assert_eq!(lock.meta.schema_version, 12);
                } else {
                    assert!(
                        record.automatic_basis.is_none(),
                        "explicit/dormant input invented automatic reliance"
                    );
                    assert_eq!(lock.meta.schema_version, 8);
                }
                assert_eq!(record.features.contains(&"child-on".to_owned()), extra);
                assert_eq!(f.slot(EXTRA).exists(), extra);
                assert_eq!(f.static_boot().contains(EXTRA_MARKER), extra);
                assert_eq!(
                    f.hooks().get(EXTRA).copied().unwrap_or(0),
                    usize::from(extra)
                );
                assert_eq!(f.hooks().get(BASE).copied(), Some(1));
                let row = lock
                    .packages
                    .iter()
                    .find(|p| p.name.as_str() == DRIVER)
                    .unwrap();
                assert_eq!(
                    row.dependencies.iter().any(|p| p.name.as_str() == EXTRA),
                    extra
                );
                let show = f
                    .user
                    .vibe()
                    .args([
                        "show",
                        "configuration-data",
                        "--path",
                        f.project.to_str().unwrap(),
                        "--json",
                    ])
                    .output()
                    .unwrap();
                assert!(show.status.success());
                let shown: serde_json::Value = serde_json::from_slice(&show.stdout).unwrap();
                assert!(
                    shown["packages"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|p| p["package"] == coordinate(DRIVER))
                );
                let before = super::workspace::semantic_snapshot(&f.project);
                let bytes = fs::read(f.project.join("vibe.lock")).unwrap();
                f.install_ok();
                assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
                assert_eq!(fs::read(f.project.join("vibe.lock")).unwrap(), bytes);
                println!(
                    "guarded-default accepted guarded={guarded} gate={gate} world={world} cold=unchanged dormant-original={}",
                    !active && !omitted
                );
            }
        }
    }
}

fn guarded_singleton_default_fixture(on: bool) -> Fixture {
    let f = dormant_guard_fixture();
    let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
    write(
        &source,
        fs::read_to_string(&source).unwrap().replace(
            "members=['selected']",
            "members=['selected','alternate']\ndefaults=['selected']",
        ) + "\n[[configuration.features]]\nid='alternate'\nname='Alternate valid singleton'\n",
    );
    set_guard_input(&f, on, false);
    let path = f.project.join("vibe.toml");
    write(
        &path,
        fs::read_to_string(&path).unwrap().replace(
            "policy='require-selection'",
            "policy='use-declared-defaults'",
        ),
    );
    f
}
#[test]
fn guarded_defaults_actual_prior_reader_writer_and_tampered_basis_refuse() {
    let old = std::env::var("NST_GUARDED_DEFAULT_OLD_CLI").expect("actual prior client required");
    for on in [false, true] {
        for (guarded, operation) in [
            (true, "show"),
            (true, "install"),
            (false, "show"),
            (false, "install"),
        ] {
            let f = guarded_singleton_default_fixture(on);
            if !guarded {
                let path = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
                write(
                    &path,
                    fs::read_to_string(&path)
                        .unwrap()
                        .replace("guards=[{member='on'}]", ""),
                );
            }
            f.install_ok();
            let before = super::workspace::semantic_snapshot(&f.project);
            let mut cmd = std::process::Command::new(&old);
            cmd.env_clear()
                .env("HOME", f.root.join("home"))
                .env("USERPROFILE", f.root.join("home"))
                .env("VIBE_SETTINGS", &f.user.settings)
                .env("VIBE_REGISTRY_CACHE", &f.user.cache)
                .env("VIBEVM_SEARCH_CACHE_DIR", &f.user.search_cache)
                .env("VIBE_NO_DEFAULT_REGISTRY", "1")
                .env("PATH", "/usr/bin:/bin");
            if operation == "show" {
                cmd.args([
                    "show",
                    "features",
                    "--json",
                    "--path",
                    f.project.to_str().unwrap(),
                ]);
            } else {
                cmd.args([
                    "install",
                    "--offline",
                    "--assume-yes",
                    "--json",
                    "--path",
                    f.project.to_str().unwrap(),
                ]);
            }
            let out = cmd.output().unwrap();
            let diagnostic = String::from_utf8_lossy(&out.stdout).to_string()
                + &String::from_utf8_lossy(&out.stderr);
            println!(
                "prior guarded={guarded} on={on} operation={operation} status={} diagnostic={diagnostic}",
                out.status
            );
            assert!(
                out.status.success() != guarded,
                "prior compatibility disagrees with guarded={guarded}: {diagnostic}"
            );
            assert!(
                super::workspace::semantic_snapshot(&f.project) == before,
                "prior changed current guarded history/state"
            );
        }
        for mutation in [
            "missing",
            "wrong",
            "extra",
            "duplicate",
            "cross-map",
            "fingerprint",
            "epoch",
        ] {
            let f = guarded_singleton_default_fixture(on);
            f.install_ok();
            let path = f.project.join("vibe.lock");
            let mut lock: toml::Value =
                toml::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
            let basis = &mut lock["configurations"][coordinate(DRIVER)]["automatic_basis"];
            match mutation {
                "missing" => {
                    basis.as_table_mut().unwrap().remove(if on {
                        "consumer_choices"
                    } else {
                        "consumer_dormant_choices"
                    });
                }
                "wrong" => {
                    if on {
                        basis["consumer_choices"]["./vibe.toml"]["detail"] = "on".into()
                    } else {
                        basis["consumer_dormant_choices"]["./vibe.toml"] =
                            toml::Value::Array(vec!["switch".into()])
                    }
                }
                "extra" => {
                    basis.as_table_mut().unwrap().insert(
                        "consumer_dormant_choices".into(),
                        toml::Value::Table(toml::Table::from_iter([(
                            "./vibe.toml".into(),
                            toml::Value::Array(vec!["detail".into(), "unknown".into()]),
                        )])),
                    );
                }
                "duplicate" => {
                    basis.as_table_mut().unwrap().insert(
                        "consumer_dormant_choices".into(),
                        toml::Value::Table(toml::Table::from_iter([(
                            "./vibe.toml".into(),
                            toml::Value::Array(vec!["detail".into(), "detail".into()]),
                        )])),
                    );
                }
                "cross-map" => {
                    basis["consumer_choices"] = toml::Value::Table(toml::Table::from_iter([(
                        "./vibe.toml".into(),
                        toml::Value::Table(toml::Table::from_iter([(
                            "detail".into(),
                            "selected".into(),
                        )])),
                    )]));
                    basis.as_table_mut().unwrap().insert(
                        "consumer_dormant_choices".into(),
                        toml::Value::Table(toml::Table::from_iter([(
                            "./vibe.toml".into(),
                            toml::Value::Array(vec!["detail".into()]),
                        )])),
                    );
                }
                "fingerprint" => basis["definition_fingerprint"] = "wrong".into(),
                "epoch" => {
                    if !on {
                        lock["meta"]["schema_version"] = 16.into()
                    } else {
                        lock["meta"]["schema_version"] = 8.into()
                    }
                }
                _ => unreachable!(),
            }
            write(&path, toml::to_string_pretty(&lock).unwrap());
            let before = super::workspace::semantic_snapshot(&f.project);
            assert!(
                !f.install().status.success(),
                "tampered guarded history admitted: on={on} {mutation}"
            );
            assert!(super::workspace::semantic_snapshot(&f.project) == before);
        }
    }
}

#[test]
fn guarded_defaults_gate_only_on_off_on_and_combined_drift_refusal() {
    let f = guarded_singleton_default_fixture(true);
    let set = |f: &Fixture, on| {
        set_guard_input(f, on, false);
        let path = f.project.join("vibe.toml");
        write(
            &path,
            fs::read_to_string(&path).unwrap().replace(
                "policy='require-selection'",
                "policy='use-declared-defaults'",
            ),
        );
    };
    let mut previous_hooks = 0;
    for on in [true, false, true] {
        set(&f, on);
        f.install_ok();
        let hooks = f.hooks().get(EXTRA).copied().unwrap_or(0);
        if on {
            assert!(
                hooks > previous_hooks,
                "active installation executes leaf hook"
            );
        } else {
            assert_eq!(hooks, previous_hooks, "dormant group executes no leaf hook");
        }
        previous_hooks = hooks;
        let lock = f.lock();
        let record = &lock.configurations[&coordinate(DRIVER)];
        assert!(
            !record.consumers["./vibe.toml"]
                .selections
                .contains_key("detail")
        );
        assert_eq!(lock.packages.iter().any(|p| p.name.as_str() == EXTRA), on);
        assert_eq!(f.slot(EXTRA).exists(), on);
        assert_eq!(f.static_boot().contains(EXTRA_MARKER), on);
        assert_eq!(record.features.contains(&"selected".into()), on);
        assert_eq!(lock.meta.schema_version, if on { 12 } else { 17 });
        let before = super::workspace::semantic_snapshot(&f.project);
        f.install_ok();
        assert_eq!(
            super::workspace::semantic_snapshot(&f.project),
            before,
            "cold replay on={on}"
        );
    }
    // Each singleton is valid in an independent actual fresh installation.
    let alternate = guarded_singleton_default_fixture(true);
    let source = alternate
        .registry
        .join(GROUP)
        .join(DRIVER)
        .join("v1.0.0/vibe.toml");
    let original = fs::read_to_string(&source).unwrap();
    let changed = original.replace("defaults=['selected']", "defaults=['alternate']");
    assert_ne!(original, changed);
    write(&source, changed);
    alternate.install_ok();
    assert!(
        alternate.lock().configurations[&coordinate(DRIVER)]
            .features
            .contains(&"alternate".into())
    );
    assert!(!alternate.slot(EXTRA).exists());
    // Unchanged history may replay its pinned store even if live source moves.
    let replay = guarded_singleton_default_fixture(true);
    replay.install_ok();
    let source = replay
        .registry
        .join(GROUP)
        .join(DRIVER)
        .join("v1.0.0/vibe.toml");
    write(
        &source,
        fs::read_to_string(&source)
            .unwrap()
            .replace("defaults=['selected']", "defaults=['alternate']"),
    );
    let before = super::workspace::semantic_snapshot(&replay.project);
    replay.install_ok();
    assert_eq!(
        super::workspace::semantic_snapshot(&replay.project),
        before,
        "unchanged pinned replay does not rebind live source"
    );
    for mutation in [
        "default",
        "guard",
        "payload",
        "source",
        "other-root",
        "additional-input",
    ] {
        let f = guarded_singleton_default_fixture(true);
        f.install_ok();
        set(&f, false);
        let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0");
        let manifest = source.join("vibe.toml");
        let request = f.project.join("vibe.toml");
        match mutation {
            "default" => {
                let old = fs::read_to_string(&manifest).unwrap();
                let changed = old.replace("defaults=['selected']", "defaults=['alternate']");
                assert_ne!(changed, old, "negative must actually mutate source");
                let parsed = Manifest::parse_str(&changed).unwrap();
                let old_input = Manifest::read(f.project.join("vibe.toml"))
                    .unwrap()
                    .requires
                    .configurations[&coordinate(DRIVER)]
                    .clone();
                let mut active = old_input;
                active.selections.get_mut("switch").unwrap().members = Some(vec!["on".into()]);
                let actual = vibe_core::manifest::PackageConfiguration::evaluate(
                    &parsed,
                    &BTreeMap::from([("./vibe.toml".into(), active)]),
                )
                .unwrap();
                assert!(
                    actual.features.contains(&"alternate".into()),
                    "replacement is a valid active singleton"
                );
                write(&manifest, changed);
            }
            "guard" => write(
                &manifest,
                fs::read_to_string(&manifest)
                    .unwrap()
                    .replace("guards=[{member='on'}]", "guards=[{member='off'}]"),
            ),
            "payload" => write(&source.join("payload.txt"), "changed same-version bytes\n"),
            "source" => {
                let other = f.root.join("other-registry");
                fs::rename(&f.registry, &other).unwrap();
                write(
                    &request,
                    fs::read_to_string(&request)
                        .unwrap()
                        .replace(f.registry.to_str().unwrap(), other.to_str().unwrap()),
                );
            }
            "other-root" => write(
                &request,
                fs::read_to_string(&request).unwrap().replace(
                    "[[registry]]",
                    &format!(
                        "[requires.packages.'{GROUP}/{BASE}']\nversion='=1.0.0'\n[[registry]]"
                    ),
                ),
            ),
            "additional-input" => write(
                &request,
                fs::read_to_string(&request).unwrap().replace(
                    "default_features=false",
                    "default_features=false,features=['off']",
                ),
            ),
            _ => unreachable!(),
        }
        let before = super::workspace::semantic_snapshot(&f.project);
        let out = f.install();
        assert!(
            !out.status.success(),
            "combined gate+{mutation} unexpectedly accepted"
        );
        assert_eq!(
            super::workspace::semantic_snapshot(&f.project),
            before,
            "combined gate+{mutation} effects"
        );
    }
}

#[test]
fn implicit_feature_default_originals_kernel_and_public_install_boundary() {
    for world in ["omitted", "true", "false", "explicit-addition"] {
        let f = Fixture::new(false);
        let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
        write(
            &source,
            package_header(DRIVER)
                + "\n[requires.packages]\n'org.vibevm/oracle-base'='=1.0.0'\n'org.vibevm/oracle-extra'={version='=1.0.0',optional=true}\n[[configuration.features]]\nid='enabled'\nname='Implicit default feature'\ndefault=true\neffects=[{kind='dependency',target='org.vibevm/oracle-extra'}]\n",
        );
        let input = match world {
            "omitted" => "policy='require-selection'",
            "true" => "policy='require-selection',default_features=true",
            "false" => "policy='require-selection',default_features=false",
            _ => "policy='require-selection',default_features=false,features=['enabled']",
        };
        write(
            &f.project.join("vibe.toml"),
            format!(
                "[project]\nname='implicit-default-qualification'\nversion='0.1.0'\n[requires.packages.'{GROUP}/{DRIVER}']\nversion='=1.0.0'\nlink='static-transitive'\nconfiguration={{{input}}}\n[[registry]]\nname='fixture'\nurl='file://{}'\n",
                f.registry.display()
            ),
        );
        let actual_source = Manifest::read(&source).unwrap();
        let original = Manifest::read(f.project.join("vibe.toml"))
            .unwrap()
            .requires
            .configurations[&coordinate(DRIVER)]
            .clone();
        let definition = actual_source.configuration.as_ref().unwrap();
        assert!(definition.groups.is_empty() && definition.profiles.is_empty());
        assert_eq!(
            original.default_features,
            matches!(world, "omitted" | "true")
        );
        let normalized = definition
            .normalize_input(&original, "./vibe.toml")
            .unwrap();
        let evaluation = definition
            .compile_choices()
            .unwrap()
            .evaluate(&normalized.features, &[normalized.choices]);
        let vibe_core::configuration::ChoiceOutcome::Resolved { features } = evaluation.outcome
        else {
            panic!("feature kernel must resolve")
        };
        let extra = world != "false";
        assert_eq!(features.selected.contains(&"enabled".into()), extra);
        assert!(
            evaluation.defaults.is_empty(),
            "feature default is not a choice DefaultUse"
        );
        let output = f.install();
        let diagnostic = String::from_utf8_lossy(&output.stdout).to_string()
            + &String::from_utf8_lossy(&output.stderr);
        println!(
            "implicit-feature world={world} kernel-extra={extra} accepted={} diagnostic={diagnostic}",
            output.status.success()
        );
        assert!(output.status.success(), "{world}: {diagnostic}");
        let lock = f.lock();
        let record = &lock.configurations[&coordinate(DRIVER)];
        assert_eq!(record.consumers["./vibe.toml"], original);
        if original.default_features {
            let basis = record
                .automatic_basis
                .as_ref()
                .expect("feature reliance missing");
            assert_eq!(
                basis.consumer_feature_defaults["./vibe.toml"],
                vec!["enabled".to_owned()]
            );
            assert!(
                basis.consumer_choices.is_empty()
                    && basis.consumer_choice_sets.is_empty()
                    && basis.consumer_dormant_choices.is_empty()
            );
            assert_eq!(lock.meta.schema_version, 18);
            assert!(record.consumers["./vibe.toml"].features.is_empty());
        } else {
            assert!(record.automatic_basis.is_none());
            assert_eq!(lock.meta.schema_version, 8);
        }
        assert_eq!(record.features.contains(&"enabled".into()), extra);
        assert_eq!(
            lock.packages.iter().any(|p| p.name.as_str() == EXTRA),
            extra
        );
        assert_eq!(f.slot(EXTRA).exists(), extra);
        assert_eq!(f.static_boot().contains(EXTRA_MARKER), extra);
        assert_eq!(
            f.hooks().get(EXTRA).copied().unwrap_or(0),
            usize::from(extra)
        );
        assert_eq!(f.hooks().get(BASE).copied(), Some(1));
        let row = lock
            .packages
            .iter()
            .find(|p| p.name.as_str() == DRIVER)
            .unwrap();
        assert_eq!(row.registry.as_deref(), Some("fixture"));
        assert_eq!(
            row.content_hash.as_str(),
            vibe_registry::compute_content_hash(source.parent().unwrap()).unwrap()
        );
        assert!(row.source_url.as_str().starts_with("file://"));
        let shown = f
            .user
            .vibe()
            .args([
                "show",
                "features",
                "--path",
                f.project.to_str().unwrap(),
                "--json",
            ])
            .output()
            .unwrap();
        assert!(shown.status.success());
        let shown: serde_json::Value = serde_json::from_slice(&shown.stdout).unwrap();
        let packages = shown["packages"].as_array().unwrap();
        let shown = packages.iter().find(|p| p["package"] == coordinate(DRIVER));
        // Existing show features omits packages whose effective set is empty.
        assert_eq!(
            shown.is_some_and(|p| p["features"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v == "enabled")),
            extra
        );
        let before = super::workspace::semantic_snapshot(&f.project);
        f.install_ok();
        assert_eq!(
            super::workspace::semantic_snapshot(&f.project),
            before,
            "cold {world}"
        );
    }
}

fn feature_default_fixture(defaults: bool, base: bool, addition: bool) -> Fixture {
    let f = Fixture::new(false);
    let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
    write(
        &source,
        package_header(DRIVER)
            + &format!(
                "\n[requires.packages]\n'org.vibevm/oracle-base'='=1.0.0'\n'org.vibevm/oracle-extra'={{version='=1.0.0',optional=true}}\n[[configuration.features]]\nid='enabled'\nname='Default seed'\ndefault={defaults}\neffects=[{{kind='dependency',target='org.vibevm/oracle-extra'}}]\n[[configuration.features]]\nid='other'\nname='Other seed'\n"
            ),
    );
    write(
        &f.project.join("vibe.toml"),
        format!(
            "[project]\nname='feature-default-first'\nversion='0.1.0'\n[requires.packages.'{GROUP}/{DRIVER}']\nversion='=1.0.0'\nlink='static-transitive'\nconfiguration={{policy='require-selection',default_features={base},features={}}}\n[[registry]]\nname='fixture'\nurl='file://{}'\n",
            if addition { "['enabled']" } else { "[]" },
            f.registry.display()
        ),
    );
    f
}

#[test]
fn feature_defaults_empty_equal_addition_prior_clients_and_history_tamper() {
    let old = std::env::var("NST_FEATURE_DEFAULT_OLD_CLI").expect("actual prior client required");
    for defaults in [false, true] {
        for addition in [false, true] {
            let f = feature_default_fixture(defaults, true, addition);
            f.install_ok();
            let lock = f.lock();
            let record = &lock.configurations[&coordinate(DRIVER)];
            assert_eq!(
                record
                    .automatic_basis
                    .as_ref()
                    .unwrap()
                    .consumer_feature_defaults["./vibe.toml"],
                if defaults {
                    vec!["enabled".to_owned()]
                } else {
                    vec![]
                }
            );
            assert_eq!(
                record.consumers["./vibe.toml"].features,
                if addition {
                    vec!["enabled".to_owned()]
                } else {
                    vec![]
                }
            );
            assert_eq!(lock.meta.schema_version, 18);
            assert_eq!(f.slot(EXTRA).exists(), defaults || addition);
            let before = super::workspace::semantic_snapshot(&f.project);
            f.install_ok();
            assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
        }
        for base in [false, true] {
            for operation in ["show", "install"] {
                let f = feature_default_fixture(defaults, base, true);
                f.install_ok();
                let before = super::workspace::semantic_snapshot(&f.project);
                let mut cmd = std::process::Command::new(&old);
                cmd.env_clear()
                    .env("HOME", f.root.join("home"))
                    .env("VIBE_SETTINGS", &f.user.settings)
                    .env("VIBE_REGISTRY_CACHE", &f.user.cache)
                    .env("VIBEVM_SEARCH_CACHE_DIR", &f.user.search_cache)
                    .env("VIBE_NO_DEFAULT_REGISTRY", "1")
                    .env("PATH", "/usr/bin:/bin");
                if operation == "show" {
                    cmd.args([
                        "show",
                        "features",
                        "--json",
                        "--path",
                        f.project.to_str().unwrap(),
                    ]);
                } else {
                    cmd.args([
                        "install",
                        "--offline",
                        "--assume-yes",
                        "--json",
                        "--path",
                        f.project.to_str().unwrap(),
                    ]);
                }
                let out = cmd.output().unwrap();
                assert_eq!(
                    out.status.success(),
                    !base,
                    "prior defaults={defaults} base={base} op={operation}: {}{}",
                    String::from_utf8_lossy(&out.stdout),
                    String::from_utf8_lossy(&out.stderr)
                );
                assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
            }
        }
        for mutation in [
            "missing",
            "wrong",
            "extra-consumer",
            "duplicate",
            "fingerprint",
            "epoch",
        ] {
            let f = feature_default_fixture(defaults, true, true); // effective set equal even with forged seeds
            f.install_ok();
            let path = f.project.join("vibe.lock");
            let mut lock: toml::Value =
                toml::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
            let basis = &mut lock["configurations"][coordinate(DRIVER)]["automatic_basis"];
            match mutation {
                "missing" => {
                    basis
                        .as_table_mut()
                        .unwrap()
                        .remove("consumer_feature_defaults");
                }
                "wrong" => {
                    basis["consumer_feature_defaults"]["./vibe.toml"] =
                        toml::Value::Array(vec!["other".into()])
                }
                "extra-consumer" => {
                    basis["consumer_feature_defaults"]
                        .as_table_mut()
                        .unwrap()
                        .insert("./foreign.toml".into(), toml::Value::Array(vec![]));
                }
                "duplicate" => {
                    basis["consumer_feature_defaults"]["./vibe.toml"] =
                        toml::Value::Array(vec!["enabled".into(), "enabled".into()])
                }
                "fingerprint" => basis["definition_fingerprint"] = "wrong".into(),
                "epoch" => lock["meta"]["schema_version"] = 17.into(),
                _ => unreachable!(),
            }
            write(&path, toml::to_string_pretty(&lock).unwrap());
            let before = super::workspace::semantic_snapshot(&f.project);
            assert!(
                !f.install().status.success(),
                "tamper {mutation} defaults={defaults}"
            );
            assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
        }
    }
}

#[test]
fn feature_defaults_definition_drift_cannot_renew_with_choice_token() {
    let reviewed = review_fixture();
    let shown = update(&reviewed);
    let proposal: serde_json::Value = serde_json::from_slice(&shown.stderr).unwrap();
    let token = proposal["proposal"]["review_id"].as_str().unwrap();
    for defaults in [false, true] {
        for change in ["default", "definition"] {
            let f = feature_default_fixture(defaults, true, true);
            f.install_ok();
            let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
            let original = fs::read_to_string(&source).unwrap();
            let changed = if change == "default" {
                original.replace(
                    &format!("default={defaults}"),
                    &format!("default={}", !defaults),
                )
            } else {
                original.replace("id='other'", "id='different'")
            };
            assert_ne!(original, changed);
            write(&source, changed);
            let before = super::workspace::semantic_snapshot(&f.project);
            let out = respond(&f, token);
            assert!(!out.status.success(), "choice token renewed {change}");
            assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
        }
    }
}

#[test]
fn feature_defaults_nonselected_version_cannot_supply_selected_seeds() {
    let f = feature_default_fixture(true, true, false);
    let old = f.registry.join(GROUP).join(DRIVER).join("v1.0.0");
    let next = f.registry.join(GROUP).join(DRIVER).join("v1.1.0");
    let text = fs::read_to_string(old.join("vibe.toml"))
        .unwrap()
        .replace("version='1.0.0'", "version='1.1.0'")
        .replace("default=true", "default=false");
    write(&next.join("vibe.toml"), text);
    write(&next.join("boot/rules.md"), "UNSELECTED_FEATURE_DEFAULT\n");
    f.install_ok();
    let lock = f.lock();
    assert_eq!(
        lock.find(&GROUP.parse().unwrap(), DRIVER)
            .unwrap()
            .version
            .to_string(),
        "1.0.0"
    );
    assert_eq!(
        lock.configurations[&coordinate(DRIVER)]
            .automatic_basis
            .as_ref()
            .unwrap()
            .consumer_feature_defaults["./vibe.toml"],
        vec!["enabled".to_owned()]
    );
    assert!(f.slot(EXTRA).exists());
    assert!(!f.static_boot().contains("UNSELECTED_FEATURE_DEFAULT"));
    let before = super::workspace::semantic_snapshot(&f.project);
    f.install_ok();
    assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
}

#[test]
fn feature_default_seeds_are_not_inferred_from_implied_closure() {
    let f = feature_default_fixture(true, true, false);
    let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
    let original = fs::read_to_string(&source).unwrap();
    let changed = original.replace(
        "name='Default seed'",
        "name='Default seed'\nrequires=['other']",
    );
    assert_ne!(original, changed);
    write(&source, changed);
    f.install_ok();
    let lock = f.lock();
    let record = &lock.configurations[&coordinate(DRIVER)];
    assert!(
        record.features.contains(&"enabled".into()) && record.features.contains(&"other".into())
    );
    assert_eq!(
        record
            .automatic_basis
            .as_ref()
            .unwrap()
            .consumer_feature_defaults["./vibe.toml"],
        vec!["enabled".to_owned()]
    );
    assert!(record.consumers["./vibe.toml"].features.is_empty());
    let before = super::workspace::semantic_snapshot(&f.project);
    f.install_ok();
    assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
}

#[test]
fn optional_public_bool_without_default_actual_boundary_and_required_control() {
    for world in ["omitted", "false", "true", "wrong-type", "required-control"] {
        let f = Fixture::new(false);
        let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
        let required = world == "required-control";
        write(
            &source,
            package_header(DRIVER)
                + &format!(
                    "\n[requires.packages]\n'org.vibevm/oracle-base'='=1.0.0'\n[configuration]\n[[configuration.parameters]]\nid='verbose'\nname='Verbose'\ntype='bool'\nrequired={required}\n"
                ),
        );
        let parameters = match world {
            "omitted" => "",
            "true" => ",parameters={verbose=true}",
            "wrong-type" => ",parameters={verbose='not-bool'}",
            _ => ",parameters={verbose=false}",
        };
        write(
            &f.project.join("vibe.toml"),
            format!(
                "[project]\nname='optional-bool-qualification'\nversion='0.1.0'\n[requires.packages.'{GROUP}/{DRIVER}']\nversion='=1.0.0'\nlink='static-transitive'\nconfiguration={{policy='require-selection',default_features=false{parameters}}}\n[[registry]]\nname='fixture'\nurl='file://{}'\n",
                f.registry.display()
            ),
        );
        let original = Manifest::read(f.project.join("vibe.toml"))
            .unwrap()
            .requires
            .configurations[&coordinate(DRIVER)]
            .clone();
        assert_eq!(
            original.parameters.contains_key("verbose"),
            world != "omitted"
        );
        let definition = Manifest::read(&source).unwrap().configuration.unwrap();
        assert!(definition.parameters[0].default.is_none());
        let normalized = definition.normalize_input(&original, "./vibe.toml");
        match world {
            "omitted" => assert!(!normalized.unwrap().parameters.contains_key("verbose")),
            "wrong-type" => assert!(normalized.is_err()),
            _ => assert_eq!(
                normalized.unwrap().parameters["verbose"],
                vibe_core::manifest::ParameterValue::Bool(world == "true")
            ),
        }
        let before = super::workspace::semantic_snapshot(&f.project);
        let out = f.install();
        let diagnostic = String::from_utf8_lossy(&out.stdout).to_string()
            + &String::from_utf8_lossy(&out.stderr);
        println!(
            "optional-bool world={world} accepted={} diagnostic={diagnostic}",
            out.status.success()
        );
        if world != "wrong-type" {
            assert!(out.status.success(), "{diagnostic}");
            let lock = f.lock();
            assert_eq!(lock.meta.schema_version, 8);
            assert_eq!(
                lock.configurations[&coordinate(DRIVER)].consumers["./vibe.toml"],
                original
            );
            if world == "omitted" {
                assert!(
                    !lock.configurations[&coordinate(DRIVER)].consumers["./vibe.toml"]
                        .parameters
                        .contains_key("verbose")
                );
            } else {
                assert_eq!(
                    lock.configurations[&coordinate(DRIVER)].consumers["./vibe.toml"].parameters["verbose"],
                    vibe_core::manifest::ParameterValue::Bool(world == "true")
                );
            }
            assert!(f.slot(BASE).exists() && !f.slot(EXTRA).exists());
            assert_eq!(f.hooks().get(BASE).copied(), Some(1));
            let shown = f
                .user
                .vibe()
                .args([
                    "show",
                    "configuration-data",
                    "--path",
                    f.project.to_str().unwrap(),
                    "--json",
                ])
                .output()
                .unwrap();
            assert!(shown.status.success());
            let report: serde_json::Value = serde_json::from_slice(&shown.stdout).unwrap();
            if world == "omitted" {
                assert!(report["packages"][0].get("parameters").is_none());
            } else {
                assert_eq!(
                    report["packages"][0]["parameters"],
                    serde_json::json!({"verbose":world=="true"})
                );
            }
            let before = super::workspace::semantic_snapshot(&f.project);
            f.install_ok();
            assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
        } else {
            assert!(
                !out.status.success(),
                "optional unexpectedly accepted {world}"
            );
            if world == "wrong-type" {
                assert!(
                    diagnostic.contains("boolean") || diagnostic.contains("bool"),
                    "{diagnostic}"
                );
            } else {
                assert!(
                    diagnostic.contains("only explicitly supplied required public bool"),
                    "{diagnostic}"
                );
            }
            assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
            assert!(
                !f.project.join("vibe.lock").exists() && !f.project.join("effects.log").exists()
            );
            assert!(!f.slot(DRIVER).exists() && !f.slot(BASE).exists());
        }
    }
}

fn optional_bool_fixture() -> Fixture {
    let f = Fixture::new(false);
    write(
        &f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml"),
        package_header(DRIVER)
            + "\n[requires.packages]\n'org.vibevm/oracle-base'='=1.0.0'\n[configuration]\n[[configuration.parameters]]\nid='verbose'\nname='Verbose'\ntype='bool'\nrequired=false\n",
    );
    f
}
fn optional_bool_request(f: &Fixture, value: Option<bool>) -> String {
    format!(
        "[project]\nname='optional-bool'\nversion='0.1.0'\n[requires.packages.'{GROUP}/{DRIVER}']\nversion='=1.0.0'\nlink='static-transitive'\nconfiguration={{policy='require-selection',default_features=false{}}}\n[[registry]]\nname='fixture'\nurl='file://{}'\n",
        value
            .map(|v| format!(",parameters={{verbose={v}}}"))
            .unwrap_or_default(),
        f.registry.display()
    )
}
#[test]
fn optional_bool_two_consumers_absence_agreement_conflict_and_cold() {
    for (left, right) in [
        (None, None),
        (None, Some(false)),
        (Some(true), None),
        (Some(false), Some(false)),
        (Some(false), Some(true)),
    ] {
        let f = optional_bool_fixture();
        write(
            &f.project.join("vibe.toml"),
            format!(
                "[project]\nname='optional-bool-workspace'\nversion='0.1.0'\n[workspace]\nmembers=['left','right']\n[[registry]]\nname='fixture'\nurl='file://{}'\n",
                f.registry.display()
            ),
        );
        for (member, value) in [("left", left), ("right", right)] {
            write(
                &f.project.join(member).join("vibe.toml"),
                optional_bool_request(&f, value),
            );
        }
        let before = super::workspace::semantic_snapshot(&f.project);
        let out = f.install();
        let conflict = left.is_some() && right.is_some() && left != right;
        assert_eq!(
            out.status.success(),
            !conflict,
            "left={left:?} right={right:?}: {}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        if conflict {
            assert!(
                String::from_utf8_lossy(&out.stderr)
                    .contains("conflicting explicit public boolean")
            );
            assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
            continue;
        }
        let lock = f.lock();
        let record = &lock.configurations[&coordinate(DRIVER)];
        assert_eq!(record.consumers.len(), 2);
        for (member, value) in [("left", left), ("right", right)] {
            let input = &record.consumers[&format!("{member}/vibe.toml")];
            assert_eq!(
                input.parameters.get("verbose"),
                value.as_ref().map(|v| if *v {
                    &vibe_core::manifest::ParameterValue::Bool(true)
                } else {
                    &vibe_core::manifest::ParameterValue::Bool(false)
                })
            );
        }
        let expected = left.or(right);
        let data = record
            .selected_parameters(vibe_core::PackageKind::Flow)
            .unwrap();
        assert_eq!(
            data.get("verbose"),
            expected
                .map(vibe_core::manifest::ParameterValue::Bool)
                .as_ref()
        );
        let shown = f
            .user
            .vibe()
            .args([
                "show",
                "configuration-data",
                "--path",
                f.project.to_str().unwrap(),
                "--json",
            ])
            .output()
            .unwrap();
        assert!(shown.status.success());
        let report: serde_json::Value = serde_json::from_slice(&shown.stdout).unwrap();
        if let Some(value) = expected {
            assert_eq!(report["packages"][0]["parameters"]["verbose"], value);
        } else {
            assert!(report["packages"][0].get("parameters").is_none());
        }
        assert_eq!(lock.meta.schema_version, 8);
        assert!(f.slot(BASE).exists() && !f.slot(EXTRA).exists());
        let before = super::workspace::semantic_snapshot(&f.project);
        f.install_ok();
        assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
    }
}
#[test]
fn optional_bool_actual_prior_reader_writer_and_definition_drift() {
    let old = std::env::var("NST_OPTIONAL_BOOL_OLD_CLI").expect("actual prior client required");
    for value in [None, Some(false), Some(true)] {
        for operation in ["show", "install"] {
            let f = optional_bool_fixture();
            write(
                &f.project.join("vibe.toml"),
                optional_bool_request(&f, value),
            );
            f.install_ok();
            let before = super::workspace::semantic_snapshot(&f.project);
            let mut cmd = std::process::Command::new(&old);
            cmd.env_clear()
                .env("HOME", f.root.join("home"))
                .env("VIBE_SETTINGS", &f.user.settings)
                .env("VIBE_REGISTRY_CACHE", &f.user.cache)
                .env("VIBEVM_SEARCH_CACHE_DIR", &f.user.search_cache)
                .env("VIBE_NO_DEFAULT_REGISTRY", "1")
                .env("PATH", "/usr/bin:/bin");
            if operation == "show" {
                cmd.args([
                    "show",
                    "configuration-data",
                    "--path",
                    f.project.to_str().unwrap(),
                    "--json",
                ]);
            } else {
                cmd.args([
                    "install",
                    "--offline",
                    "--assume-yes",
                    "--path",
                    f.project.to_str().unwrap(),
                    "--json",
                ]);
            }
            let out = cmd.output().unwrap();
            assert!(
                !out.status.success(),
                "prior silently accepted optional history"
            );
            assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
        }
        let f = optional_bool_fixture();
        write(
            &f.project.join("vibe.toml"),
            optional_bool_request(&f, value),
        );
        f.install_ok();
        let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0/vibe.toml");
        let next = f.registry.join(GROUP).join(DRIVER).join("v1.1.0");
        write(
            &next.join("vibe.toml"),
            fs::read_to_string(&source)
                .unwrap()
                .replace("version='1.0.0'", "version='1.1.0'")
                .replace("required=false", "required=true"),
        );
        write(&next.join("boot/rules.md"), "CHANGED_REQUIRED_DEFINITION\n");
        let request = f.project.join("vibe.toml");
        write(
            &request,
            fs::read_to_string(&request)
                .unwrap()
                .replace("version='=1.0.0'", "version='=1.1.0'"),
        );
        let before = super::workspace::semantic_snapshot(&f.project);
        let out = update(&f);
        assert!(
            !out.status.success(),
            "definition changed optional meaning value={value:?}: {}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
    }
}

#[test]
fn integrated_forward_subskill_and_config_data_share_one_actual_selection() {
    const BODY: &str = "INERT_INTEGRATED_SUBSKILL_BODY\n";
    for selected in [false, true] {
        let f = exact_forward_fixture();
        let request = f.project.join("vibe.toml");
        if !selected {
            write(
                &request,
                fs::read_to_string(&request)
                    .unwrap()
                    .replace("members=['enabled']", "members=['disabled']"),
            );
        }
        let source = f.registry.join(GROUP).join(DRIVER).join("v1.0.0");
        let manifest = source.join("vibe.toml");
        let original = fs::read_to_string(&manifest).unwrap();
        let needle = "effects = [{ kind = \"forward\", target = \"org.vibevm/oracle-extra\", feature = \"forwarded\", strength = \"strong\" }]";
        let combined = "effects = [{ kind = \"forward\", target = \"org.vibevm/oracle-extra\", feature = \"forwarded\", strength = \"strong\" }, {kind='subskill',path='subskills/audit/SKILL.md'}, {kind='config-value',name='integrated'}]";
        assert!(
            original.contains(needle),
            "fixture exact effect replacement required"
        );
        write(
            &manifest,
            original.replace(needle, combined)
                + "\n[configuration.config_values]\nintegrated=false\n",
        );
        write(
            &source.join("subskills/audit/vibe-subskill.toml"),
            "[subskill]\npath='audit'\ndelivery='lazy-pull'\ndescription='Inert integration control'\n[content]\nfiles_written=['SKILL.md']\n",
        );
        write(&source.join("subskills/audit/SKILL.md"), BODY);
        let authored = fs::read(&request).unwrap();
        f.install_ok();
        let lock = f.lock();
        let parent = &lock.configurations[&coordinate(DRIVER)];
        let row = lock.find(&GROUP.parse().unwrap(), DRIVER).unwrap();
        assert!(parent.forwarded_inputs.is_empty());
        assert_eq!(parent.consumers.len(), 1);
        assert_eq!(f.slot(EXTRA).exists(), selected);
        assert_eq!(f.static_boot().contains(EXTRA_MARKER), selected);
        assert!(!f.static_boot().contains(BODY.trim()));
        assert_eq!(f.hooks().get(BASE).copied(), Some(1));
        assert_eq!(
            f.hooks().get(EXTRA).copied().unwrap_or(0),
            usize::from(selected)
        );
        assert_eq!(row.subskills_active.len(), usize::from(selected));
        assert_eq!(
            parent
                .selected_data(vibe_core::PackageKind::Flow)
                .unwrap()
                .get("integrated"),
            selected.then_some(&vibe_core::manifest::ParameterValue::Bool(false))
        );
        if selected {
            let child = &lock.configurations[&coordinate(EXTRA)];
            assert!(child.consumers.is_empty());
            assert_eq!(child.forwarded_inputs.len(), 1);
            let forwarded = &child.forwarded_inputs[0];
            assert_eq!(forwarded.source_name.as_str(), DRIVER);
            assert_eq!(forwarded.source_feature, "enabled");
            assert_eq!(forwarded.source_content_hash, row.content_hash);
            assert_eq!(forwarded.target_feature, "forwarded");
            assert_eq!(
                child.selected_data(vibe_core::PackageKind::Flow).unwrap()["forwarded"],
                vibe_core::manifest::ParameterValue::Bool(true)
            );
            let witness = row.subskills_active[0].source_witness.as_ref().unwrap();
            assert_eq!(witness.package_content_hash, row.content_hash);
            assert_eq!(witness.member.path, "subskills/audit/SKILL.md");
            assert!(
                read_subskill(&f)["content"]
                    .as_str()
                    .unwrap()
                    .contains(BODY.trim())
            );
        } else {
            assert!(!lock.configurations.contains_key(&coordinate(EXTRA)));
            assert!(read_subskill(&f).get("refused").is_some());
            assert!(!f.project.join("subskills/audit/SKILL.md").exists());
        }
        let before = super::workspace::semantic_snapshot(&f.project);
        let show = f
            .user
            .vibe()
            .args([
                "show",
                "configuration-data",
                "--path",
                f.project.to_str().unwrap(),
                "--json",
            ])
            .output()
            .unwrap();
        assert!(show.status.success());
        let report: serde_json::Value = serde_json::from_slice(&show.stdout).unwrap();
        let parent = report["packages"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["package"] == coordinate(DRIVER))
            .unwrap();
        assert_eq!(
            parent["values"].get("integrated"),
            selected.then_some(&serde_json::Value::Bool(false))
        );
        let transport=f.user.vibe().args(["--offline","mcp","serve","--path",f.project.to_str().unwrap()]).write_stdin(serde_json::json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"read_subskill","arguments":{"package":coordinate(DRIVER),"subskill_path":"audit"}}}).to_string()+"\n").output().unwrap();
        assert!(transport.status.success());
        assert_eq!(
            String::from_utf8_lossy(&transport.stdout).contains(BODY.trim()),
            selected
        );
        f.install_ok();
        assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
        assert_eq!(fs::read(&request).unwrap(), authored);
        assert_eq!(
            read_subskill(&f)
                .get("content")
                .and_then(serde_json::Value::as_str)
                .is_some(),
            selected
        );
    }
}

#[test]
fn actual_dependency_conflict_backtracks_without_default_or_effect_leakage() {
    const SHARED: &str = "oracle-shared-conflict";
    const NEW_EFFECT: &str = "oracle-new-candidate-effect";
    for conflict in [true, false] {
        let f = Fixture::new(false);
        leaf(&f.registry, SHARED, "SHARED_ONE_MARKER");
        leaf(&f.registry, NEW_EFFECT, "REJECTED_NEW_EFFECT_MARKER");
        let shared2 = f.registry.join(GROUP).join(SHARED).join("v2.0.0");
        let shared1 = f.registry.join(GROUP).join(SHARED).join("v1.0.0");
        write(
            &shared2.join("vibe.toml"),
            fs::read_to_string(shared1.join("vibe.toml"))
                .unwrap()
                .replace("version='1.0.0'", "version='2.0.0'"),
        );
        write(&shared2.join("boot/rules.md"), "SHARED_TWO_MARKER\n");
        write(&shared2.join("payload.txt"), "SHARED_TWO_BYTES\n");
        write(
            &shared2.join("hooks/record.sh"),
            format!("set -eu\nprintf '%s\\n' '{SHARED}' >> \"$VIBE_PROJECT_ROOT/effects.log\"\n"),
        );
        for (version, shared, selected, marker) in [
            ("1.0.0", "=1.0.0", "old", "EARLY_SELECTED_MARKER"),
            ("1.1.0", "=2.0.0", "new", "NEW_CANDIDATE_MARKER"),
        ] {
            let source = f
                .registry
                .join(GROUP)
                .join(DRIVER)
                .join(format!("v{version}"));
            let text = package_header(DRIVER)
                .replace("version='1.0.0'", &format!("version='{version}'"))
                + &format!(
                    "\n[requires.packages]\n'{GROUP}/{SHARED}'='{shared}'\n'{GROUP}/{EXTRA}'={{version='=1.0.0',optional=true}}\n'{GROUP}/{NEW_EFFECT}'={{version='=1.0.0',optional=true}}\n[[configuration.features]]\nid='old'\nname='Earlier default'\neffects=[{{kind='dependency',target='{GROUP}/{EXTRA}'}}]\n[[configuration.features]]\nid='new'\nname='New default'\neffects=[{{kind='dependency',target='{GROUP}/{NEW_EFFECT}'}}]\n[[configuration.groups]]\nmode='choice'\nid='mode'\nname='Selected default'\ncardinality='exactly-one'\nmembers=['old','new']\ndefaults=['{selected}']\n"
                );
            write(&source.join("vibe.toml"), text);
            write(&source.join("boot/rules.md"), format!("{marker}\n"));
            write(&source.join("payload.txt"), format!("DRIVER_{version}\n"));
        }
        let root_shared = if conflict { "=1.0.0" } else { "=2.0.0" };
        write(
            &f.project.join("vibe.toml"),
            format!(
                "[project]\nname='actual-backtracking'\nversion='0.1.0'\n[requires.packages.'{GROUP}/{DRIVER}']\nversion='^1'\nlink='static-transitive'\nconfiguration={{policy='use-declared-defaults',default_features=false}}\n[requires.packages.'{GROUP}/{SHARED}']\nversion='{root_shared}'\nlink='static-transitive'\n[[registry]]\nname='fixture'\nurl='file://{}'\n",
                f.registry.display()
            ),
        );
        let request = Manifest::read(f.project.join("vibe.toml")).unwrap();
        assert!(
            request
                .requires
                .packages
                .iter()
                .find(|p| p.name.as_str() == DRIVER)
                .unwrap()
                .version
                .matches(&"1.1.0".parse().unwrap()),
            "newer is in actual root range"
        );
        f.install_ok();
        let lock = f.lock();
        let driver = lock.find(&GROUP.parse().unwrap(), DRIVER).unwrap();
        assert_eq!(
            driver.version.to_string(),
            if conflict { "1.0.0" } else { "1.1.0" }
        );
        let selected = if conflict { "old" } else { "new" };
        let record = &lock.configurations[&coordinate(DRIVER)];
        assert_eq!(
            record.automatic_basis.as_ref().unwrap().consumer_choices["./vibe.toml"]["mode"],
            selected
        );
        assert_eq!(record.features, vec![selected.to_owned()]);
        let exact = f
            .registry
            .join(GROUP)
            .join(DRIVER)
            .join(format!("v{}", driver.version));
        assert_eq!(
            driver.content_hash.as_str(),
            vibe_registry::compute_content_hash(&exact).unwrap()
        );
        assert_eq!(f.slot(EXTRA).exists(), conflict);
        let new_slot = f
            .project
            .join("vibevm/vibedeps")
            .join(format!("{GROUP}.{NEW_EFFECT}/1.0.0"));
        assert_eq!(new_slot.exists(), !conflict);
        let boot = f.static_boot();
        assert_eq!(boot.contains("EARLY_SELECTED_MARKER"), conflict);
        assert_eq!(boot.contains("NEW_CANDIDATE_MARKER"), !conflict);
        assert_eq!(boot.contains("REJECTED_NEW_EFFECT_MARKER"), !conflict);
        assert_eq!(
            f.hooks().get(EXTRA).copied().unwrap_or(0),
            usize::from(conflict)
        );
        assert_eq!(
            f.hooks().get(NEW_EFFECT).copied().unwrap_or(0),
            usize::from(!conflict)
        );
        assert_eq!(f.hooks().get(SHARED).copied(), Some(1));
        assert!(
            !f.project
                .join("vibevm/vibedeps")
                .join(format!(
                    "{GROUP}.{DRIVER}/{}",
                    if conflict { "1.1.0" } else { "1.0.0" }
                ))
                .exists()
        );
        let before = super::workspace::semantic_snapshot(&f.project);
        f.install_ok();
        assert_eq!(super::workspace::semantic_snapshot(&f.project), before);
    }
}
