//! Tests for [`super`] — the user-level config file and its one home.
//! Split out of `user_config.rs` (DISCIPLINE-SWEEP §1a tests-out) and
//! included via `#[path]`; the module stays private test code.

use std::error::Error;
use std::process::Command;

use super::*;
use tempfile::tempdir;

/// Planted into the file at the former location. If it ever shows up on a
/// stream, the notice printed contents.
const PLANTED_MARKER: &str = "planted-config-body-must-never-be-printed";

fn error_chain_text(error: &(dyn Error + 'static)) -> String {
    let mut out = format!("{error:?}\n{error}");
    let mut source = error.source();
    while let Some(next) = source {
        out.push('\n');
        out.push_str(&format!("{next:?}\n{next}"));
        source = next.source();
    }
    out
}

/// Set on a re-executed copy of this test binary to put it in child mode —
/// see [`a_config_planted_at_the_former_location_is_never_resolved`].
const CHILD_MARKER: &str = "VIBE_TEST_ECHO_USER_CONFIG_PATH";

/// The child runs exactly this test, by libtest name filter.
const CHILD_TEST: &str = "a_config_planted_at_the_former_location_is_never_resolved";

/// Where [`former_config_path`] looks, given a stand-in home. Mirrors its
/// `cfg!(windows)` split so the plant lands where the probe reads, on
/// either platform.
fn former_config_under(home: &Path) -> PathBuf {
    if cfg!(windows) {
        home.join("vibe").join("config.toml")
    } else {
        home.join(".config").join("vibe").join("config.toml")
    }
}

#[test]
fn default_is_empty_env() {
    let cfg = UserConfig::default();
    assert!(cfg.env.is_empty());
}

#[test]
fn load_from_missing_file_is_default() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let cfg = UserConfig::load_from(&path).unwrap();
    assert_eq!(cfg, UserConfig::default());
}

#[test]
fn load_from_parses_env_block() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(
        &path,
        r#"[env]
VIBE_REGISTRY_CACHE = "/custom/cache"
VIBE_LOG = "vibe_registry=debug"
"#,
    )
    .unwrap();
    let cfg = UserConfig::load_from(&path).unwrap();
    assert_eq!(
        cfg.env.get("VIBE_REGISTRY_CACHE").map(String::as_str),
        Some("/custom/cache")
    );
    assert_eq!(
        cfg.env.get("VIBE_LOG").map(String::as_str),
        Some("vibe_registry=debug")
    );
}

#[test]
fn load_from_rejects_unknown_top_level_section() {
    // `deny_unknown_fields` keeps the schema strict so a typo
    // surfaces instead of a silent no-op.
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(
        &path,
        r#"[envv]
VIBE_REGISTRY_CACHE = "/typo"
"#,
    )
    .unwrap();
    let err = UserConfig::load_from(&path).unwrap_err();
    assert!(matches!(err, UserConfigError::Parse { .. }));
}

#[test]
fn load_from_malformed_toml_errors() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "this is = not = toml").unwrap();
    let err = UserConfig::load_from(&path).unwrap_err();
    assert!(matches!(err, UserConfigError::Parse { .. }));
}

// --- PROP-011 §5.2 — the `[install]` section ------------------------------

#[test]
fn slot_integrity_defaults_to_trust_presence() {
    let cfg = UserConfig::default();
    assert_eq!(cfg.install.slot_integrity, SlotIntegrity::TrustPresence);
    assert!(cfg.install.spec_format.is_none());
    assert!(cfg.install.is_default());
}

#[test]
fn load_from_parses_install_slot_integrity() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "[install]\nslot_integrity = \"verify\"\n").unwrap();
    let cfg = UserConfig::load_from(&path).unwrap();
    assert_eq!(cfg.install.slot_integrity, SlotIntegrity::Verify);
    assert!(!cfg.install.is_default());
}

#[test]
fn install_section_round_trips() {
    let cfg = UserConfig {
        install: InstallConfig {
            slot_integrity: SlotIntegrity::Verify,
            spec_format: Some(crate::manifest::SpecFormat::Xml),
        },
        ..Default::default()
    };
    let rendered = toml::to_string_pretty(&cfg).unwrap();
    assert!(
        rendered.contains("slot_integrity = \"verify\""),
        "{rendered}"
    );
    assert!(rendered.contains("spec_format = \"xml\""), "{rendered}");
    let back: UserConfig = toml::from_str(&rendered).unwrap();
    assert_eq!(cfg, back);
}

#[test]
fn load_from_parses_install_spec_format() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "[install]\nspec_format = \"markdown\"\n").unwrap();
    let cfg = UserConfig::load_from(&path).unwrap();
    assert_eq!(
        cfg.install.spec_format,
        Some(crate::manifest::SpecFormat::Markdown)
    );
}

#[test]
fn load_from_rejects_an_unknown_install_key() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "[install]\nbogus = true\n").unwrap();
    assert!(matches!(
        UserConfig::load_from(&path).unwrap_err(),
        UserConfigError::Parse { .. }
    ));
}

// --- PROP-010 §2.5 — the `[net]` section ------------------------------------

#[test]
fn net_defaults_to_online() {
    let cfg = UserConfig::default();
    assert!(!cfg.net.offline);
    assert!(cfg.net.is_default());
}

#[test]
fn load_from_parses_net_offline() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "[net]\noffline = true\n").unwrap();
    let cfg = UserConfig::load_from(&path).unwrap();
    assert!(cfg.net.offline);
    assert!(!cfg.net.is_default());
}

#[test]
fn net_section_round_trips() {
    let cfg = UserConfig {
        net: NetConfig { offline: true },
        ..Default::default()
    };
    let rendered = toml::to_string_pretty(&cfg).unwrap();
    assert!(rendered.contains("[net]"), "{rendered}");
    assert!(rendered.contains("offline = true"), "{rendered}");
    let back: UserConfig = toml::from_str(&rendered).unwrap();
    assert_eq!(cfg, back);
}

/// `deny_unknown_fields` holds INSIDE the section too: an unknown key
/// under `[net]` is a parse error, not a silent no-op — a typo'd
/// `offine` must not leave the operator offline-blind while they
/// believe the posture is set.
#[test]
fn load_from_rejects_an_unknown_net_key() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "[net]\nbogus = true\n").unwrap();
    assert!(matches!(
        UserConfig::load_from(&path).unwrap_err(),
        UserConfigError::Parse { .. }
    ));
}

// --- PROP-000 §16 / PROP-054 §6.4 — the user `[llm]` section ----------

#[test]
fn llm_section_parses_and_round_trips_all_user_owned_fields() {
    let raw = r#"
[llm]
provider = "openai-compatible"
model = "demo-chat"
endpoint = "https://example.invalid/v1/chat/completions"
token_file = "credentials/provider.token"
"#;
    let cfg: UserConfig = toml::from_str(raw).unwrap();
    let llm = cfg.llm.as_ref().unwrap();
    assert_eq!(llm.provider, "openai-compatible");
    assert_eq!(llm.model, "demo-chat");
    assert_eq!(llm.endpoint, "https://example.invalid/v1/chat/completions");
    assert_eq!(
        llm.token_file.as_deref(),
        Some(Path::new("credentials/provider.token"))
    );

    let rendered = toml::to_string_pretty(&cfg).unwrap();
    let back: UserConfig = toml::from_str(&rendered).unwrap();
    assert_eq!(back, cfg);
}

#[test]
fn llm_token_file_is_optional_and_unknown_fields_refuse() {
    let without_file: UserConfig = toml::from_str(
        "[llm]\nprovider = \"openai-compatible\"\nmodel = \"m\"\nendpoint = \"https://example.invalid/v1/chat/completions\"\n",
    )
    .unwrap();
    assert!(without_file.llm.unwrap().token_file.is_none());

    let unknown = "[llm]\nprovider = \"openai-compatible\"\nmodel = \"m\"\nendpoint = \"https://example.invalid/v1/chat/completions\"\napi_key = \"not-a-supported-field\"\n";
    assert!(toml::from_str::<UserConfig>(unknown).is_err());
}

#[test]
fn project_llm_section_cannot_supply_a_token_file() {
    let manifest = r#"
[project]
name = "demo"
version = "1.0.0"

[llm]
default_provider = "openai-compatible"
default_model = "demo-chat"
token_file = "project-secret.token"
"#;
    assert!(crate::manifest::Manifest::parse_str(manifest).is_err());
}

#[test]
fn llm_debug_and_public_parse_errors_never_retain_raw_config_canaries() {
    const QUERY_CANARY: &str = "query-config-canary-must-never-appear";
    const PROJECT_TOKEN_CANARY: &str = "project-token-canary-must-never-appear";

    let llm = LlmConfig {
        provider: "openai-compatible".into(),
        model: "demo".into(),
        endpoint: format!("https://example.invalid/v1/chat/completions?secret={QUERY_CANARY}"),
        token_file: Some("provider.token".into()),
    };
    assert!(!format!("{llm:?}").contains(QUERY_CANARY));
    let cfg = UserConfig {
        llm: Some(llm),
        ..UserConfig::default()
    };
    assert!(!format!("{cfg:?}").contains(QUERY_CANARY));

    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(
        &path,
        format!(
            "[llm]\nprovider = \"openai-compatible\"\nmodel = \"demo\"\nendpoint = \"https://example.invalid/v1/chat/completions?secret={QUERY_CANARY}\n"
        ),
    )
    .unwrap();
    let error = UserConfig::load_from(&path).unwrap_err();
    match &error {
        UserConfigError::Parse { diagnostic, .. } => {
            assert!(diagnostic.span().is_some());
            assert!(diagnostic.line().is_some());
            assert!(diagnostic.column().is_some());
            assert!(!diagnostic.message().is_empty());
        }
        other => panic!("expected safe parse diagnostic, got {other:?}"),
    }
    let rendered = error_chain_text(&error);
    assert!(!rendered.contains(QUERY_CANARY), "{rendered}");
    assert!(rendered.contains("line") && rendered.contains("column"));
    assert!(rendered.contains(&path.display().to_string()));

    let manifest = format!(
        "[project]\nname = \"demo\"\nversion = \"1.0.0\"\n\
         [llm]\ndefault_provider = \"openai-compatible\"\ndefault_model = \"demo\"\n\
         api_key = \"{PROJECT_TOKEN_CANARY}\"\n"
    );
    let error = crate::manifest::Manifest::parse_str(&manifest).unwrap_err();
    match &error {
        crate::Error::ParseToml { diagnostic, .. } => {
            assert!(diagnostic.span().is_some());
            assert!(diagnostic.message().contains("unknown field"));
        }
        other => panic!("expected safe manifest diagnostic, got {other:?}"),
    }
    let rendered = error_chain_text(&error);
    assert!(!rendered.contains(PROJECT_TOKEN_CANARY), "{rendered}");
    assert!(rendered.contains("unknown field"), "{rendered}");
}

// --- One config home: the former location is named, never read ------------

#[test]
fn the_one_candidate_is_the_settings_dir_file() {
    // The disk surface is exactly one leg. The pre-consolidation location
    // was a second one that `$VIBE_SETTINGS` could not relocate; it is no
    // longer a candidate, so redirecting the settings dir now redirects the
    // whole user-config layer.
    let root = crate::settings::settings_dir().expect("home dir present in test env");
    let candidates = config_file_candidates();
    assert_eq!(candidates.len(), 1, "expected exactly one on-disk leg");
    assert!(candidates[0].ends_with("config.toml"));
    assert!(
        candidates[0].starts_with(&root),
        "{} escapes the settings dir {}",
        candidates[0].display(),
        root.display()
    );
}

#[test]
fn notice_names_both_paths_on_one_line_and_never_the_contents() {
    let former_home = tempdir().unwrap();
    let former = former_config_under(former_home.path());
    std::fs::create_dir_all(former.parent().unwrap()).unwrap();
    std::fs::write(&former, format!("[env]\nVIBE_LOG = \"{PLANTED_MARKER}\"\n")).unwrap();
    let settings = tempdir().unwrap();
    let canonical = settings.path().join("config.toml");

    let line = left_behind_notice(Some(&canonical), Some(&former)).expect("a notice is due");
    assert_eq!(line.lines().count(), 1, "must be one line: {line}");
    assert!(line.contains(&former.display().to_string()), "{line}");
    assert!(line.contains(&canonical.display().to_string()), "{line}");
    assert!(
        !line.contains(PLANTED_MARKER),
        "printed the contents: {line}"
    );
}

#[test]
fn no_notice_when_the_canonical_config_exists() {
    // Nothing stopped being read, so there is nothing to say.
    let former_home = tempdir().unwrap();
    let former = former_config_under(former_home.path());
    std::fs::create_dir_all(former.parent().unwrap()).unwrap();
    std::fs::write(&former, "[env]\n").unwrap();
    let settings = tempdir().unwrap();
    let canonical = settings.path().join("config.toml");
    std::fs::write(&canonical, "[env]\n").unwrap();

    assert_eq!(left_behind_notice(Some(&canonical), Some(&former)), None);
}

#[test]
fn no_notice_when_nothing_was_left_behind() {
    let former_home = tempdir().unwrap();
    let settings = tempdir().unwrap();
    let former = former_config_under(former_home.path());
    let canonical = settings.path().join("config.toml");
    assert_eq!(left_behind_notice(Some(&canonical), Some(&former)), None);
    // No settings dir resolvable ⇒ nowhere to point the operator at.
    assert_eq!(left_behind_notice(None, Some(&former)), None);
}

/// The behaviour the removed leg broke: with `$VIBE_SETTINGS` pointed at an
/// empty temp dir and a config planted at the former location, the loader
/// resolves the canonical path and never the planted file — it only names
/// it, once, without its contents.
///
/// Driven through a re-executed copy of this test binary because the
/// scenario is env-shaped and this crate is `#![forbid(unsafe_code)]`:
/// edition 2024 marks `std::env::set_var` unsafe, and libtest runs bodies on
/// many threads, which is the hazard that marking exists for. A child
/// process takes its environment at spawn, so `Command::env` is both safe
/// and honest — and it exercises the real `default_path`, the real notice,
/// and the real process-once guard rather than a stand-in.
#[test]
fn a_config_planted_at_the_former_location_is_never_resolved() {
    if std::env::var_os(CHILD_MARKER).is_some() {
        // Child. Three resolutions, so the parent can tell "once per
        // process" from "once per call".
        let resolved = UserConfig::default_path();
        let _ = UserConfig::default_path();
        let _ = UserConfig::default_path();
        println!(
            "RESOLVED={}",
            resolved
                .map(|p| p.display().to_string())
                .unwrap_or_default()
        );
        return;
    }

    let settings = tempdir().unwrap();
    let former_home = tempdir().unwrap();
    let former = former_config_under(former_home.path());
    std::fs::create_dir_all(former.parent().unwrap()).unwrap();
    std::fs::write(&former, format!("[env]\nVIBE_LOG = \"{PLANTED_MARKER}\"\n")).unwrap();

    let mut cmd = Command::new(std::env::current_exe().expect("test binary path"));
    cmd.arg(CHILD_TEST)
        .arg("--nocapture")
        .arg("--test-threads=1")
        .env(CHILD_MARKER, "1")
        .env(crate::settings::SETTINGS_DIR_ENV, settings.path())
        .env_remove("VIBEVM_USER_CONFIG");
    // Point the former location at the stand-in home the same way the probe
    // finds it, and leave the operator's real one untouched.
    if cfg!(windows) {
        cmd.env("APPDATA", former_home.path());
    } else {
        cmd.env("HOME", former_home.path());
    }
    let out = cmd.output().expect("re-exec the test binary");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "child failed\n{stdout}\n{stderr}");

    // `--nocapture` interleaves the child's own output onto libtest's
    // "test <name> ... " line, so match the marker anywhere in a line rather
    // than at its start.
    let resolved = stdout
        .lines()
        .find_map(|l| l.split_once("RESOLVED=").map(|(_, rest)| rest.trim()))
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("child printed no RESOLVED line\n{stdout}\n{stderr}"));

    assert_ne!(
        resolved, former,
        "the loader resolved the planted file at the former location"
    );
    assert_eq!(resolved, settings.path().join("config.toml"));
    assert!(resolved.starts_with(settings.path()));

    // Said once, naming both paths, contents nowhere on either stream.
    let notices: Vec<&str> = stderr
        .lines()
        .filter(|l| l.contains("no longer read"))
        .collect();
    assert_eq!(notices.len(), 1, "expected one notice, got: {stderr}");
    assert!(
        notices[0].contains(&former.display().to_string()),
        "{stderr}"
    );
    assert!(
        notices[0].contains(&settings.path().join("config.toml").display().to_string()),
        "{stderr}"
    );
    assert!(!stdout.contains(PLANTED_MARKER), "contents on stdout");
    assert!(!stderr.contains(PLANTED_MARKER), "contents on stderr");
}

/// `[artifacts]` and `[self]` round-trip, the source type is
/// case-insensitive on read and lower-case on write
/// (`PROP-019#release-sources`, `PROP-066#drops`).
#[test]
fn artifacts_and_self_sections_round_trip() {
    let cfg: UserConfig = toml::from_str(
        "[artifacts]\nvibe = \"/srv/rel\"\npackages = \"/srv/pay\"\n\
         [self]\nsource = \"LocalDir\"\npath = \"/srv/rel\"\n",
    )
    .unwrap();
    assert_eq!(
        cfg.self_source.source,
        Some(super::ReleaseSourceKind::Localdir)
    );
    assert_eq!(
        cfg.artifacts.packages.as_deref(),
        Some(std::path::Path::new("/srv/pay"))
    );
    let rendered = toml::to_string_pretty(&cfg).unwrap();
    assert!(rendered.contains("[self]"), "{rendered}");
    assert!(rendered.contains("source = \"localdir\""), "{rendered}");
    assert!(rendered.contains("[artifacts]"), "{rendered}");
    let back: UserConfig = toml::from_str(&rendered).unwrap();
    assert_eq!(back, cfg);
    let unknown = toml::from_str::<UserConfig>("[self]\nsource = \"s3\"\n").unwrap_err();
    assert!(unknown.to_string().contains("unknown release source type"));
}

/// The leading `~` of a configured per-user path, and nothing else: `~`
/// and `~/<rest>` resolve against the home the caller already resolved,
/// `~user` is refused because vibe resolves nobody else's home, and every
/// other path comes back exactly as written (`PROP-019#install-root`,
/// `PROP-066#drops`).
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-019#install-root")]
fn the_home_prefix_is_expanded_and_only_that() {
    use super::artifacts::expand_home;
    use std::path::{Path, PathBuf};

    let home = PathBuf::from("/home/me");
    assert_eq!(expand_home(Path::new("~"), Some(&home)).unwrap(), home);
    assert_eq!(
        expand_home(Path::new("~/opt"), Some(&home)).unwrap(),
        home.join("opt")
    );
    assert_eq!(
        expand_home(Path::new("~/opt/drop/packages"), Some(&home)).unwrap(),
        home.join("opt/drop/packages")
    );
    // Untouched: absolute, Windows-absolute, and a relative path that is
    // somebody else's problem to refuse.
    for written in ["/srv/vibe", r"C:\Work", r"\\server\share\vibe", "opt"] {
        assert_eq!(
            expand_home(Path::new(written), Some(&home)).unwrap(),
            PathBuf::from(written),
            "`{written}` is not a home-rooted path"
        );
    }
    // A file whose own name begins with `~` is not `~` either.
    assert!(matches!(
        expand_home(Path::new("~bob/opt"), Some(&home)),
        Err(super::InstallRootError::OtherUser { .. })
    ));
    assert!(matches!(
        expand_home(Path::new("~"), None),
        Err(super::InstallRootError::NoHome { .. })
    ));
}

/// The user's own per-slot, per-ecosystem extras list (PROP-075
/// `#SLOT-EXTRAS`). The table had to exist here before a user could write
/// it at all: `UserConfig` is `deny_unknown_fields`, so a file carrying
/// `[slot.node.extras.npm]` was a parse error on **every** command, which
/// is why this is a `vibe-core` change and not only a `vibe-app` one.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS")]
fn the_slot_extras_table_parses_round_trips_and_keeps_its_own_order() {
    let body = "[slot.node.extras.npm]\n\
                add = [\"typescript\", \"@scope/pkg@^1.2\", \"; rm -rf $HOME\"]\n\
                drop = [\"corepack\"]\n\
                [slot.java.extras.maven]\nadd = [\"org.example:plugin:1.0\"]\n";
    let cfg: UserConfig = toml::from_str(body).unwrap();
    let npm = cfg.extras_of("node", "npm").unwrap();
    // The order is the file's, and an item is opaque — vibe does not make
    // a shell's sense of it.
    assert_eq!(
        npm.add,
        ["typescript", "@scope/pkg@^1.2", "; rm -rf $HOME"],
        "the user's own order, verbatim"
    );
    assert_eq!(npm.drop, ["corepack"]);
    assert!(cfg.extras_of("java", "maven").unwrap().drop.is_empty());
    // A slot or ecosystem the file never mentions is absent, never an error.
    assert!(cfg.extras_of("node", "cargo").is_none());
    assert!(cfg.extras_of("python", "npm").is_none());
    let rendered = toml::to_string_pretty(&cfg).unwrap();
    assert!(rendered.contains("[slot.node.extras.npm]"), "{rendered}");
    assert_eq!(toml::from_str::<UserConfig>(&rendered).unwrap(), cfg);
}

/// A file with no `[slot]` table at all is the whole of today's world, and
/// it must stay byte-identical through the struct: this is the regression
/// the `vibe-app` → user-config seam of P7c is measured against.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS")]
fn a_config_with_no_slot_table_is_unchanged_and_an_unknown_slot_key_still_refuses() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "[net]\noffline = true\n").unwrap();
    let cfg = UserConfig::load_from(&path).unwrap();
    assert!(cfg.slot.is_empty());
    assert!(cfg.extras_of("node", "npm").is_none());
    // The sections `UserConfig` already carried are untouched.
    assert!(cfg.net.offline);
    assert_eq!(cfg, UserConfig::load_from(&path).unwrap());
    // `deny_unknown_fields` still bites inside the new tables, so a typo
    // in the user's own list is reported rather than silently dropped.
    assert!(
        toml::from_str::<UserConfig>("[slot.node.extras.npm]\nadds = [\"x\"]\n").is_err(),
        "a mis-spelled member of the extras table must refuse"
    );
    assert!(
        toml::from_str::<UserConfig>("[slot.node]\nextra = {}\n").is_err(),
        "a mis-spelled member of the slot table must refuse"
    );
}

/// The settings editors (`vibe app extras add|drop|import`) append, keep
/// the user's order and never duplicate an exact string.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS")]
fn the_extras_editors_append_in_place_and_an_import_reads_one_item_per_line() {
    let mut cfg = UserConfig::default();
    assert_eq!(
        cfg.extras_mut("node", "npm")
            .add_items(["typescript".to_owned(), "pnpm@9".to_owned()]),
        2
    );
    assert_eq!(
        cfg.extras_mut("node", "npm")
            .add_items(["pnpm@9".to_owned()]),
        0
    );
    assert_eq!(
        cfg.extras_mut("node", "npm")
            .drop_ids(["corepack".to_owned()]),
        1
    );
    assert_eq!(
        cfg.extras_of("node", "npm").unwrap().add,
        ["typescript", "pnpm@9"]
    );
    assert_eq!(cfg.extras_of("node", "npm").unwrap().drop, ["corepack"]);
    // `$NVM_DIR/default-packages`, as a file: one item per line, blank
    // lines and `#` comments ignored, everything else verbatim.
    assert_eq!(
        read_item_lines("# my globals\n\n  typescript  \n@scope/pkg@^1.2\npkg#tag\n\n"),
        ["typescript", "@scope/pkg@^1.2", "pkg#tag"]
    );
    assert!(read_item_lines("# nothing but comments\n\n").is_empty());
}

/// Saving keeps the file's comments and unchanged bytes, so a user whose
/// `config.toml` is hand-written does not lose it to `extras add`.
#[test]
#[specmark::spec(verifies = "spec://org.vibevm.core/vibevm/common/PROP-075#SLOT-EXTRAS")]
fn saving_an_edited_extras_list_preserves_the_rest_of_the_users_file() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "# my own notes\n[net]\noffline = true\n").unwrap();
    let mut cfg = UserConfig::load_from(&path).unwrap();
    cfg.extras_mut("node", "npm")
        .add_items(["typescript".to_owned()]);
    cfg.save_to(&path).unwrap();
    let body = std::fs::read_to_string(&path).unwrap();
    assert!(body.contains("# my own notes"), "{body}");
    assert!(body.contains("typescript"), "{body}");
    let back = UserConfig::load_from(&path).unwrap();
    assert!(back.net.offline);
    assert_eq!(back.extras_of("node", "npm").unwrap().add, ["typescript"]);
}
