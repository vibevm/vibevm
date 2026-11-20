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
