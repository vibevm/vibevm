//! Comment-preserving rewrites keep every byte the edit did not change.

use super::render_preserving;
use crate::GlobalRegistryConfig;
use crate::manifest::{MirrorSection, NamingConvention, RegistrySection};
use crate::user_config::UserConfig;

const REGISTRY_FILE: &str = "\
# Machine registries — hand-written, keep these notes.

[[registry]]
# the canonical org
name = 'vibespecs'
url = 'https://github.com/vibespecs'   # the public one

[[registry]]
name = \"corp\"
url = \"https://git.corp.example/vibe\"
naming = \"name\"

# Mirrors below.
[[mirror]]
of = 'vibespecs'
url = 'https://nexus.corp.example/vibespecs'

[source_policy]
# never github on this machine
exclude_hosting = ['github']

# footer note
";

fn registry(name: &str, url: &str) -> RegistrySection {
    RegistrySection {
        name: name.into(),
        r#type: None,
        url: url.into(),
        r#ref: "main".into(),
        naming: NamingConvention::Fqdn,
        auth: None,
        token_env: None,
        enabled: true,
        index_url: None,
    }
}

fn parsed() -> GlobalRegistryConfig {
    toml::from_str(REGISTRY_FILE).unwrap()
}

#[test]
fn saving_an_unchanged_config_is_byte_identical() {
    let rendered = render_preserving(Some(REGISTRY_FILE), &parsed()).unwrap();
    assert_eq!(rendered, REGISTRY_FILE);
}

#[test]
fn adding_a_registry_appends_it_and_keeps_every_other_byte() {
    let mut config = parsed();
    config
        .registries
        .push(registry("local", "file:///srv/registry"));
    let rendered = render_preserving(Some(REGISTRY_FILE), &config).unwrap();
    // Everything up to the end of the existing `[[registry]]` run is
    // untouched, then the new element, then the rest of the file verbatim.
    let split = REGISTRY_FILE.find("\n# Mirrors below.").unwrap() + 1;
    let (head, tail) = REGISTRY_FILE.split_at(split);
    assert!(rendered.starts_with(head), "{rendered}");
    assert!(rendered.ends_with(tail), "{rendered}");
    let added = &rendered[head.len()..rendered.len() - tail.len()];
    assert!(added.contains("[[registry]]"), "{added}");
    assert!(added.contains("name = \"local\""), "{added}");
    let reparsed: GlobalRegistryConfig = toml::from_str(&rendered).unwrap();
    assert_eq!(reparsed, config);
}

#[test]
fn removing_a_registry_removes_only_its_table() {
    let mut config = parsed();
    config.registries.retain(|registry| registry.name != "corp");
    let rendered = render_preserving(Some(REGISTRY_FILE), &config).unwrap();
    let expected = REGISTRY_FILE.replace(
        "[[registry]]\nname = \"corp\"\nurl = \"https://git.corp.example/vibe\"\nnaming = \"name\"\n\n",
        "",
    );
    assert_eq!(rendered, expected);
}

#[test]
fn removing_the_last_mirror_keeps_the_unrelated_tables() {
    let mut config = parsed();
    config.mirrors.clear();
    let rendered = render_preserving(Some(REGISTRY_FILE), &config).unwrap();
    assert!(!rendered.contains("[[mirror]]"), "{rendered}");
    assert!(rendered.starts_with(
        "# Machine registries — hand-written, keep these notes.\n\n[[registry]]\n# the canonical org\n"
    ));
    assert!(rendered.contains("url = 'https://github.com/vibespecs'   # the public one\n"));
    assert!(rendered.ends_with(
        "[source_policy]\n# never github on this machine\nexclude_hosting = ['github']\n\n# footer note\n"
    ));
}

#[test]
fn a_changed_key_keeps_its_neighbours_and_its_comment() {
    let mut config = parsed();
    config.registries[0].url = "https://github.com/vibespecs-fork".into();
    let rendered = render_preserving(Some(REGISTRY_FILE), &config).unwrap();
    let expected = REGISTRY_FILE.replace(
        "url = 'https://github.com/vibespecs'   # the public one",
        "url = \"https://github.com/vibespecs-fork\"   # the public one",
    );
    assert_eq!(rendered, expected);
}

#[test]
fn a_new_mirror_lands_after_the_existing_mirror() {
    let mut config = parsed();
    config.mirrors.push(MirrorSection {
        publication: Default::default(),
        of: "corp".into(),
        url: "https://nexus.corp.example/corp".into(),
        priority: 1,
    });
    let rendered = render_preserving(Some(REGISTRY_FILE), &config).unwrap();
    let first = rendered.find("of = 'vibespecs'").unwrap();
    let second = rendered.find("of = \"corp\"").unwrap();
    let policy = rendered.find("[source_policy]").unwrap();
    assert!(first < second && second < policy, "{rendered}");
    assert!(rendered.ends_with(
        "[source_policy]\n# never github on this machine\nexclude_hosting = ['github']\n\n# footer note\n"
    ));
}

#[test]
fn no_existing_file_is_the_plain_rendering() {
    let config = parsed();
    assert_eq!(
        render_preserving(None, &config).unwrap(),
        toml::to_string_pretty(&config).unwrap()
    );
}

#[test]
fn a_malformed_existing_file_falls_back_to_the_plain_rendering() {
    let config = parsed();
    assert_eq!(
        render_preserving(Some("this is [not toml"), &config).unwrap(),
        toml::to_string_pretty(&config).unwrap()
    );
}

const USER_CONFIG: &str = "\
# my vibe settings
[init]
# who I am
last_author = 'Me <me@example.com>'

[self]
source = 'GitHub' # case as I typed it

[artifacts]
packages = '/srv/drop' # the USB stick
";

#[test]
fn setting_the_release_source_rewrites_only_the_self_table() {
    let mut config: UserConfig = toml::from_str(USER_CONFIG).unwrap();
    config.self_source.source = Some("nexus".parse().unwrap());
    config.self_source.url = Some("https://nexus.corp.example/vibe".into());
    let rendered = render_preserving(Some(USER_CONFIG), &config).unwrap();
    let (head, _) = USER_CONFIG.split_at(USER_CONFIG.find("[self]").unwrap());
    let (_, tail) = USER_CONFIG.split_at(USER_CONFIG.find("\n[artifacts]").unwrap());
    assert!(rendered.starts_with(head), "{rendered}");
    assert!(rendered.ends_with(tail), "{rendered}");
    assert!(
        rendered.contains("source = \"nexus\" # case as I typed it\n"),
        "{rendered}"
    );
    assert!(
        rendered.contains("url = \"https://nexus.corp.example/vibe\""),
        "{rendered}"
    );
    let reparsed: UserConfig = toml::from_str(&rendered).unwrap();
    assert_eq!(reparsed, config);
}

#[test]
fn resetting_the_release_source_removes_the_self_table_only() {
    let mut config: UserConfig = toml::from_str(USER_CONFIG).unwrap();
    config.self_source = Default::default();
    let rendered = render_preserving(Some(USER_CONFIG), &config).unwrap();
    assert_eq!(
        rendered,
        USER_CONFIG.replace("[self]\nsource = 'GitHub' # case as I typed it\n\n", "")
    );
}

/// A comment above the first key belongs to that key's line; changing the
/// key's value must not take the file header with it.
#[test]
fn a_header_comment_on_a_changed_first_key_survives() {
    let existing = "# my settings\n[init]\n# me\nlast_author = 'Old' # was\n";
    let mut config: UserConfig = toml::from_str(existing).unwrap();
    config.init.last_author = Some("New".into());
    assert_eq!(
        render_preserving(Some(existing), &config).unwrap(),
        "# my settings\n[init]\n# me\nlast_author = \"New\" # was\n"
    );
}
