//! Public preset identity round trips without launching a harness or rewriting legacy input.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-064#presets");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-HARNESS-EXECUTABLE");

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

use serde_json::{Value, json};
use vibe_core::agent_ids::AgentId;

struct Fixture {
    root: tempfile::TempDir,
    home: PathBuf,
    settings: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let home = root.path().join("home");
        let settings = root.path().join("settings");
        for path in [&home, &settings] {
            fs::create_dir_all(path).unwrap();
        }
        Self {
            root,
            home,
            settings,
        }
    }

    fn run(&self, args: &[&str]) -> Output {
        // No inherited configuration, credentials, registrations or store overrides.
        // Both the default home-derived roots and the explicit settings/cache roots
        // belong to this fixture, and settings are deliberately outside HOME.
        Command::new(env!("CARGO_BIN_EXE_vibe"))
            .env_clear()
            .env("HOME", &self.home)
            .env("USERPROFILE", &self.home)
            .env("VIBE_SETTINGS", &self.settings)
            .env(
                "VIBE_REGISTRY_CACHE",
                self.root.path().join("registry-cache"),
            )
            .env(
                "VIBEVM_SEARCH_CACHE_DIR",
                self.root.path().join("search-cache"),
            )
            .env("VIBEVM_INSTALL_ROOT", self.root.path().join("applications"))
            .env("VIBE_NO_DEFAULT_REGISTRY", "1")
            .env("NO_COLOR", "1")
            .current_dir(self.root.path())
            .args(args)
            .output()
            .unwrap()
    }

    fn text(&self, args: &[&str]) -> String {
        let output = self.run(args);
        assert!(
            output.status.success(),
            "{args:?}: status={} stdout={} stderr={}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    }

    fn json(&self, args: &[&str]) -> Value {
        serde_json::from_str(&self.text(args)).unwrap()
    }
}

fn named<'a>(document: &'a Value, table: &str, name: &str) -> &'a Value {
    document[table]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"] == name)
        .unwrap_or_else(|| panic!("missing {table}.{name}: {document}"))
}

#[test]
fn canonical_preset_name_roundtrips() {
    let fixture = Fixture::new();
    // Discover the accepted preset from the clean human launcher listing,
    // before any definition exists to supply a coincidental preset annotation.
    let clean = fixture.text(&["agent", "list"]);
    let launcher_row = clean
        .lines()
        .take_while(|line| !line.contains("agent definitions"))
        .find(|line| {
            line.split_whitespace().next() == Some("claude") && line.contains("[built-in]")
        })
        .expect("the clean built-in claude launcher row advertises its preset");
    let canonical = launcher_row
        .split_once("preset ")
        .and_then(|(_, metadata)| metadata.split_whitespace().next())
        .expect("the built-in launcher identifies an accepted preset");
    assert_eq!(canonical, "claude-code");
    let discovered = fixture.json(&[
        "agent",
        "add",
        "discovered",
        "--preset",
        canonical,
        "--json",
    ]);
    let discovered = named(&discovered, "agents", "discovered");
    assert_eq!(discovered["preset"], canonical);
    assert_eq!(discovered["command"], "claude");
    assert!(discovered["problem"].is_null());

    let presets = fixture.json(&["agent", "presets", "--json"]);
    assert_eq!(presets["protocol"], "vibe-agent-presets/1");
    let claude = named(&presets, "presets", "claude-code");
    assert_eq!(claude["command"], "claude");
    assert_eq!(AgentId::Claude.row().preset, Some("claude-code"));
    assert_eq!(AgentId::Claude.row().harness, Some("claude"));

    // Use the names advertised by the public presets operation, not executable
    // spellings or an independently invented canonical vocabulary.
    for preset in presets["presets"].as_array().unwrap() {
        let name = preset["name"].as_str().unwrap();
        assert!(AgentId::from_preset(name).is_some());
        let added = fixture.json(&["agent", "add", name, "--preset", name, "--json"]);
        assert_eq!(added["protocol"], "vibe-agent-list/1");
        let agent = named(&added, "agents", name);
        assert_eq!(agent["preset"], name);
        assert_eq!(agent["command"], preset["command"]);
        assert_eq!(agent["interactive"], preset["interactive"]);
        assert_eq!(agent["unattended"], preset["unattended"]);
        assert!(agent["problem"].is_null());
    }

    fixture.json(&[
        "agent",
        "add",
        "personal-alias",
        "--preset",
        "claude-code",
        "--command",
        "personal-wrapper",
        "--extra-arg=--standing",
        "--json",
    ]);
    let registry = fixture.settings.join("agents.toml");
    let before = fs::read(&registry).unwrap();
    let listed = fixture.json(&["agent", "list", "--json"]);
    let agent = named(&listed, "agents", "claude-code");
    assert_eq!(agent["preset"], "claude-code");
    assert_eq!(agent["command"], "claude");
    let alias = named(&listed, "agents", "personal-alias");
    assert_eq!(alias["preset"], "claude-code");
    assert_eq!(alias["command"], "personal-wrapper");
    assert_eq!(alias["extra_args"], json!(["--standing"]));

    // Launcher identity is the existing harness key, not the preset spelling;
    // neither its flag dialect nor its program is renamed by accepting a preset.
    let launcher = named(&listed, "launchers", "claude");
    assert_eq!(launcher["kind"], "claude");
    assert_eq!(launcher["command"], json!(["claude"]));
    assert_eq!(launcher["source"], "built-in");
    assert!(
        fixture
            .text(&["agent", "list"])
            .contains("preset claude-code")
    );
    assert_eq!(fs::read(&registry).unwrap(), before);

    // An old, hand-authored document remains compatible, including a user alias
    // whose name happens to equal the executable and a wrapper's standing argv.
    let legacy = b"# keep this comment and formatting on read\ndefault = 'claude'\n\n\
[agent.claude]\npreset = 'claude-code'\ncommand = 'legacy-wrapper'\n\
extra_args = ['--legacy', 'two words']\n\n\
[launchers.personal-launcher]\nkind = 'claude'\n\
command = ['legacy-launcher', '--standing', 'two words']\n\n\
[launchers.claude]\nkind = 'claude'\ncommand = ['overridden-wrapper', '--kept']\n";
    fs::write(&registry, legacy).unwrap();
    let legacy_list = fixture.json(&["agent", "list", "--json"]);
    assert_eq!(legacy_list["protocol"], "vibe-agent-list/1");
    assert_eq!(legacy_list["default"], "claude");
    let agent = named(&legacy_list, "agents", "claude");
    assert_eq!(agent["preset"], "claude-code");
    assert_eq!(agent["command"], "legacy-wrapper");
    assert_eq!(agent["extra_args"], json!(["--legacy", "two words"]));
    assert_eq!(agent["interactive"], json!(["{prompt}"]));
    assert_eq!(agent["unattended"], json!(["-p", "{prompt}"]));
    assert!(agent["problem"].is_null());
    let launcher = named(&legacy_list, "launchers", "personal-launcher");
    assert_eq!(launcher["kind"], "claude");
    assert_eq!(
        launcher["command"],
        json!(["legacy-launcher", "--standing", "two words"])
    );
    assert_eq!(launcher["source"], registry.to_str().unwrap());
    assert_eq!(
        named(&legacy_list, "launchers", "claude")["command"],
        json!(["overridden-wrapper", "--kept"])
    );
    let human = fixture.text(&["agent", "list"]);
    assert!(human.contains("preset claude-code"));
    assert!(human.contains("personal-launcher"));
    assert_eq!(fs::read(&registry).unwrap(), legacy);
    fixture.json(&["agent", "presets", "--json"]);
    assert_eq!(fs::read(&registry).unwrap(), legacy);
    assert!(!fixture.home.join(".vibe").exists());
}
