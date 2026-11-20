//! Narrow ordinary defaults use shared policy; changed reliance stays closed.
use super::*;
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
        ("type='integer'\nrequired=true", "verbose=1"),
        ("type='path'\nrequired=true", "verbose='/synthetic'"),
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
fn compulsory_forwarding_unsupported_ranges_weak_and_recursive_refuse_before_effects() {
    for mutation in [
        "parent-range",
        "target-range",
        "weak",
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
            "weak" => write(
                &driver,
                fs::read_to_string(&driver)
                    .unwrap()
                    .replace("strength = \"strong\"", "strength = \"weak\""),
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
    assert!(!names.iter().any(|name| name.contains("configuration")));
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
