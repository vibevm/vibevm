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
