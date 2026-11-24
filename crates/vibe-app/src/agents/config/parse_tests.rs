//! Disk-only registry diagnostics under an isolated settings root.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-064#agent-definition");

use super::*;

#[test]
fn registry_parse_reason_is_one_line() {
    let dir = tempfile::tempdir().unwrap();
    let settings = SettingsRoot::new(dir.path());
    let path = settings.agents_path();
    fs::write(
        &path,
        "[launchers.fake]\nkind = \"pi\"\ncommand = \"private-program\"\n",
    )
    .unwrap();
    let error = AgentsConfig::load(&settings).unwrap_err();
    assert_eq!(error.code(), "agents_config_invalid");
    assert!(
        matches!(&error, AgentError::Config { path: actual, .. } if actual == &path.display().to_string())
    );
    let display = error.to_string();
    assert_eq!(
        display,
        format!(
            "not a valid agent registry: `{}` \
             (violates spec://org.vibevm.core/vibevm/common/PROP-064#agent-definition; \
             fix: correct the TOML syntax and field types in the file): \
             line 3, column 11: command must be a list of strings, not a string",
            path.display()
        )
    );
    assert!(!display.contains("private-program"));
    assert_eq!(display.lines().count(), 1);

    let malformed = "[launchers.fake]\nkind = \"pi\"\ncommand = [\n";
    fs::write(&path, malformed).unwrap();
    let error = AgentsConfig::load(&settings).unwrap_err();
    assert_eq!(error.code(), "agents_config_invalid");
    let display = error.to_string();
    assert_eq!(display.lines().count(), 1);
    let reason = display.split_once("): ").unwrap().1;
    assert!(reason.starts_with("line ") && reason.contains(", column "));
    assert!(!reason.split_once(": ").unwrap().1.trim().is_empty());
    assert!(!display.contains("command = ["));
    assert!(!display.contains("kind ="));

    // A multibyte prefix must not turn the character column into a byte column.
    let unicode = "default = \"λ\" ?";
    fs::write(&path, unicode).unwrap();
    let parsed = toml::from_str::<AgentsConfig>(unicode).unwrap_err();
    let offset = parsed.span().unwrap().start;
    let column = unicode[..offset].chars().count() + 1;
    assert!(column < offset + 1);
    let display = AgentsConfig::load(&settings).unwrap_err().to_string();
    assert!(display.contains(&format!("line 1, column {column}: ")));
    assert_eq!(display.lines().count(), 1);
    assert!(!display.contains(unicode));

    // Agent definitions deliberately use scalar commands, unlike launchers.
    fs::write(&path, "[agent.fake]\ncommand = \"legitimate-program\"\n").unwrap();
    let loaded = AgentsConfig::load(&settings).unwrap();
    assert_eq!(
        loaded.agent["fake"].command.as_deref(),
        Some("legitimate-program")
    );
}

#[test]
fn registry_save_failure_names_write_not_parse() {
    let dir = tempfile::tempdir().unwrap();
    let settings = SettingsRoot::new(dir.path());
    let path = settings.agents_path();
    // A directory at the destination makes rename fail even for root.
    fs::create_dir(&path).unwrap();
    let error = AgentsConfig::default().save(&settings).unwrap_err();
    assert_eq!(error.code(), "agents_config_invalid");
    assert!(
        matches!(&error, AgentError::Config { path: actual, reason } if actual == &path.display().to_string() && reason.starts_with(CONFIG_WRITE))
    );
    let display = error.to_string();
    let AgentError::Config { reason, .. } = &error else {
        panic!("expected a registry error");
    };
    let cause = reason.strip_prefix(CONFIG_WRITE).unwrap();
    assert!(!cause.is_empty());
    assert_eq!(
        display,
        format!(
            "could not write agent registry: `{}` \
         (violates spec://org.vibevm.core/vibevm/common/PROP-064#agent-definition; \
         fix: check write permissions and that the target can be atomically replaced): {cause}",
            path.display()
        )
    );
    assert_eq!(display.matches(&path.display().to_string()).count(), 1);
    assert_eq!(display.matches("(violates ").count(), 1);
    for forbidden in [
        "not a valid",
        "parse",
        "correct the file",
        "delete",
        "remove",
        "vibe agent use",
    ] {
        assert!(!display.contains(forbidden));
    }
    assert!(path.is_dir());
}

#[test]
fn registry_quoted_command_keeps_value_private() {
    let dir = tempfile::tempdir().unwrap();
    let settings = SettingsRoot::new(dir.path());
    let path = settings.agents_path();
    for key in ["\"command\"", "'command'"] {
        fs::write(
            &path,
            format!("[launchers.fake]\nkind=\"pi\"\n{key} = \"SYNTHETIC_PRIVATE_PROGRAM\"\n"),
        )
        .unwrap();
        let error = AgentsConfig::load(&settings).unwrap_err();
        assert_eq!(error.code(), "agents_config_invalid");
        let display = error.to_string();
        assert!(!display.contains("SYNTHETIC_PRIVATE_PROGRAM"), "{display}");
        assert!(
            display.ends_with("line 3, column 13: command must be a list of strings, not a string"),
            "{display}"
        );
    }
}

#[test]
fn registry_unicode_command_key_keeps_value_private() {
    let dir = tempfile::tempdir().unwrap();
    let settings = SettingsRoot::new(dir.path());
    fs::write(
        settings.agents_path(),
        "[launchers.fake]\nkind=\"pi\"\n\"comm\\u0061nd\" = \"SYNTHETIC_PRIVATE_PROGRAM\"\n",
    )
    .unwrap();
    let error = AgentsConfig::load(&settings).unwrap_err();
    assert_eq!(error.code(), "agents_config_invalid");
    let display = error.to_string();
    assert!(!display.contains("SYNTHETIC_PRIVATE_PROGRAM"), "{display}");
    assert!(
        display.ends_with("line 3, column 18: expected a list of strings, not a string"),
        "{display}"
    );
}

#[test]
fn registry_external_reason_stays_neutral() {
    // Operation words inside arbitrary text must not select an internal repair.
    let reason = "external detail: writing agent registry: unexpected parse issue";
    let error = AgentError::Config {
        path: "synthetic/agents.toml".into(),
        reason: reason.into(),
    };
    assert_eq!(error.code(), "agents_config_invalid");
    assert_eq!(
        error.to_string(),
        format!(
            "agent registry operation failed: `synthetic/agents.toml` \
         (violates spec://org.vibevm.core/vibevm/common/PROP-064#agent-definition; \
         fix: check the reported cause and registry settings): {reason}"
        )
    );
}
