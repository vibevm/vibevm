use super::*;
use crate::GlobalRegistrySource;
use crate::manifest::HostingSelector;
use specmark::verifies;

const CORP: &str = r#"# the corporate profile
schema = 1
name = "corp"

[[registry]]
name = "corp"
type = "LocalDir"
url = "file:///srv/vibe-packages"

[[mirror]]
of = "corp"
url = "file:///mnt/mirror"

[[remap]]
prefix = "https://example.invalid/"
url = "https://mirror.corp.example/"

[source_policy]
exclude_hosting = ["github", "gitverse"]

[artifacts]
vibe = "/srv/vibe-releases"
packages = "/srv/vibe-artifacts"

[self]
source = "localdir"

[proxy]
https = "http://proxy.corp.example:3128"

[net]
offline = true

[path]
fix = "always"
"#;

fn corp() -> SiteProfile {
    SiteProfile::parse(CORP, "corp.toml").unwrap()
}

fn refused(text: &str) -> String {
    SiteProfile::parse(text, "corp.toml")
        .unwrap_err()
        .to_string()
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-019#site-profile")]
fn every_table_of_a_full_profile_is_read() {
    let profile = corp();
    assert_eq!(profile.name.as_deref(), Some("corp"));
    assert_eq!(profile.registries.len(), 1);
    assert_eq!(profile.mirrors.len(), 1);
    assert_eq!(profile.remap.len(), 1);
    assert_eq!(
        profile.source_policy.exclude_hosting,
        [HostingSelector::Github, HostingSelector::Gitverse]
    );
    assert!(profile.declares_release_source());
    assert!(profile.net.offline);
    assert_eq!(profile.path_fix(), PathFixPolicy::Always);
    assert_eq!(profile.remap_rows("corp.toml").unwrap().rows.len(), 1);
    let user = profile.user_config_layer();
    assert_eq!(
        user.proxy.https.as_deref(),
        Some("http://proxy.corp.example:3128")
    );
    assert_eq!(profile.registry_layer().mirrors[0].of, "corp");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-019#site-profile")]
fn each_invalid_value_is_refused_naming_file_key_and_fix() {
    let cases = [
        ("schema = 2\n", "schema = 2"),
        ("name = 'x'\n", "schema"),
        ("schema = 1\n[artifact]\nvibe = '/x'\n", "artifact"),
        (
            "schema = 1\n[artifacts]\nvibe = 'relative/dir'\n",
            "artifacts.vibe",
        ),
        ("schema = 1\n[self]\npath = 'rel'\n", "self.path"),
        (
            "schema = 1\n[self]\ninstall_root = 'opt'\n",
            "self.install_root",
        ),
        // Another account's home is not a directory vibe resolves, while
        // `~` is per-user absolute and accepted (the test below).
        (
            "schema = 1\n[self]\ninstall_root = '~bob/opt'\n",
            "self.install_root",
        ),
        (
            "schema = 1\n[[registry]]\nname='a'\nurl='file:///a'\n[[registry]]\nname='a'\nurl='file:///b'\n",
            "registry[1].name",
        ),
        (
            "schema = 1\n[[registry]]\nname='a'\ntype='localdir'\nurl='https://git.corp/x'\n",
            "registry[0].type",
        ),
        (
            "schema = 1\n[source_policy]\nexclude_hosting = ['github', 'github']\n",
            "source_policy",
        ),
        (
            "schema = 1\n[[remap]]\nprefix = 'https://a/'\n",
            "[[remap]]",
        ),
        ("schema = 1\n[proxy]\nhttps = ''\n", "proxy.https"),
        ("schema = 1\n[path]\nfix = 'sometimes'\n", "sometimes"),
    ];
    for (text, needle) in cases {
        let message = refused(text);
        assert!(message.contains("corp.toml"), "{text}: {message}");
        assert!(message.contains(needle), "{text}: {message}");
        assert!(
            message.contains("spec://org.vibevm.core/vibevm/common/PROP-019#site-profile"),
            "{message}"
        );
        assert!(message.contains("fix:"), "{message}");
    }
}

#[test]
fn windows_drive_paths_are_absolute_for_a_profile_validated_anywhere() {
    let profile = SiteProfile::parse(
        "schema = 1\n[artifacts]\nvibe = 'D:/corp/vibe-releases'\npackages = '\\\\\\\\nas\\\\drop'\n",
        "corp.toml",
    )
    .unwrap();
    assert!(profile.artifacts.vibe.is_some());
}

fn write(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, body).unwrap();
    path
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#registry-layers")]
fn user_beats_site_and_site_beats_the_built_in_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let site = write(
        dir.path(),
        SITE_PROFILE_FILE,
        "schema = 1\n[[registry]]\nname = 'corp'\nurl = 'file:///site/corp'\n\
         [[registry]]\nname = 'shared'\nurl = 'file:///site/shared'\n\
         [source_policy]\nexclude_hosting = ['github']\nprefer_hosting = ['gitverse']\n",
    );
    let defaults = GlobalRegistryConfig {
        registries: crate::default_registries(),
        ..Default::default()
    };

    // No user file: the site's registries replace the built-in defaults.
    let missing = dir.path().join("registry.toml");
    let source = GlobalRegistrySource::deferred_with_site(
        Some(missing.clone()),
        Some(site.clone()),
        defaults.clone(),
    );
    let names: Vec<String> = source
        .catalog()
        .unwrap()
        .registries
        .into_iter()
        .map(|r| r.name)
        .collect();
    assert_eq!(names, ["corp", "shared"]);
    assert_eq!(
        source.policy().unwrap().exclude_hosting,
        [HostingSelector::Github]
    );

    // A user file: its rows come first and hide the site's of the same name.
    write(
        dir.path(),
        "registry.toml",
        "[[registry]]\nname = 'shared'\nurl = 'file:///mine'\n\
         [source_policy]\nprefer_hosting = ['github']\nexclude_hosting = ['gitverse']\n",
    );
    let source =
        GlobalRegistrySource::deferred_with_site(Some(missing), Some(site), defaults.clone());
    let catalog = source.catalog().unwrap();
    let rows: Vec<(&str, &str)> = catalog
        .registries
        .iter()
        .map(|r| (r.name.as_str(), r.url.as_str()))
        .collect();
    assert_eq!(
        rows,
        [("shared", "file:///mine"), ("corp", "file:///site/corp")]
    );
    let policy = source.policy().unwrap();
    assert_eq!(policy.prefer_hosting, Some(vec![HostingSelector::Github]));
    assert_eq!(
        policy.exclude_hosting,
        [HostingSelector::Gitverse, HostingSelector::Github]
    );

    // A site without registries leaves the defaults in place.
    let bare = write(dir.path(), "bare.toml", "schema = 1\n");
    let source = GlobalRegistrySource::deferred_with_site(
        Some(dir.path().join("absent.toml")),
        Some(bare),
        defaults.clone(),
    );
    assert_eq!(source.catalog().unwrap().registries, defaults.registries);
}

#[test]
fn a_stored_profile_edited_into_an_invalid_one_is_an_error_not_silence() {
    let dir = tempfile::tempdir().unwrap();
    assert!(
        installed_at(&dir.path().join(SITE_PROFILE_FILE))
            .unwrap()
            .is_none()
    );
    let broken = write(dir.path(), SITE_PROFILE_FILE, "schema = 1\n[nope]\n");
    let error = installed_at(&broken).unwrap_err().to_string();
    assert!(error.contains(&broken.display().to_string()), "{error}");
    let source = GlobalRegistrySource::deferred_with_site(None, Some(broken), Default::default());
    assert!(source.catalog().is_err());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-019#site-profile")]
fn the_user_config_keeps_its_own_values_over_the_site() {
    let site = corp();
    let mine: UserConfig = toml::from_str(
        "[self]\nsource = 'github'\n[proxy]\nhttps = 'http://mine:1'\n[artifacts]\npackages = '/home/me/drop'\n",
    )
    .unwrap();
    let effective = mine.over_site(&site);
    assert_eq!(
        effective.self_source.source,
        Some(crate::user_config::ReleaseSourceKind::Github)
    );
    assert_eq!(
        effective.artifacts.vibe, None,
        "the release decision is the user's"
    );
    assert_eq!(effective.proxy.https.as_deref(), Some("http://mine:1"));
    assert_eq!(
        effective.artifacts.packages.as_deref(),
        Some(Path::new("/home/me/drop"))
    );
    assert!(effective.net.offline, "the site turned the network off");

    let effective = UserConfig::default().over_site(&site);
    assert_eq!(
        effective.artifacts.vibe.as_deref(),
        Some(Path::new("/srv/vibe-releases"))
    );
    assert_eq!(
        effective.artifacts.packages.as_deref(),
        Some(Path::new("/srv/vibe-artifacts"))
    );
}

#[test]
fn the_receipt_round_trips_and_keeps_a_comment_on_rewrite() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(SITE_RECEIPT_FILE);
    let receipt = SiteReceipt {
        schema: 1,
        source: "/srv/corp.toml".into(),
        sha256: "ab".into(),
        applied_at: "2026-11-08T00:00:00Z".into(),
        vibe_version: "1.0.0".into(),
        name: Some("corp".into()),
        owns: vec![SITE_PROFILE_FILE.into(), SITE_RECEIPT_FILE.into()],
        path: Some(ReceiptPath {
            shim_dir: "/home/u/.vibe/opt/bin".into(),
            outcome: "applied".into(),
            target: Some("~/.bashrc".into()),
        }),
    };
    std::fs::write(
        &path,
        format!("# kept\n{}", receipt.render(None, &path).unwrap()),
    )
    .unwrap();
    assert_eq!(
        SiteReceipt::load_from(&path).unwrap(),
        Some(receipt.clone())
    );
    let again = SiteReceipt {
        sha256: "cd".into(),
        ..receipt
    };
    let existing = std::fs::read_to_string(&path).unwrap();
    let rendered = again.render(Some(&existing), &path).unwrap();
    assert!(rendered.starts_with("# kept\n"), "{rendered}");
    assert!(rendered.contains("sha256 = \"cd\""), "{rendered}");
}

/// `[self] install_root` says where vibe installs ITSELF and
/// `[self] source` where it reads releases FROM. They layer apart
/// (`PROP-019#install-root`): a profile that only names the install root
/// declares no release source, so a user who keeps the public GitHub
/// releases still inherits the one directory their network allows.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-019#site-profile")]
fn an_install_root_alone_is_not_a_release_source_decision() {
    let profile = SiteProfile::parse(
        "schema = 1\n[self]\ninstall_root = '/srv/vibe'\n",
        "corp.toml",
    )
    .unwrap();
    assert_eq!(
        profile.self_source.install_root.as_deref(),
        Some(std::path::Path::new("/srv/vibe"))
    );
    assert!(!profile.declares_release_source());

    // The user's own release source does not cost them the site's root.
    let mut own = crate::user_config::UserConfig::default();
    own.self_source.source = Some(crate::user_config::ReleaseSourceKind::Github);
    let merged = own.over_site(&profile);
    assert_eq!(
        merged.self_source.source,
        Some(crate::user_config::ReleaseSourceKind::Github)
    );
    assert_eq!(
        merged.self_source.install_root.as_deref(),
        Some(std::path::Path::new("/srv/vibe"))
    );

    // A root of their own wins over the site's.
    let mut own = crate::user_config::UserConfig::default();
    own.self_source.install_root = Some("/home/me/vibe".into());
    let merged = own.over_site(&profile);
    assert_eq!(
        merged.self_source.install_root.as_deref(),
        Some(std::path::Path::new("/home/me/vibe"))
    );
}

/// `[self] install_root = "~"` is the one path a profile may root at the
/// reader's own home: it names one directory on every machine that applies
/// the profile, which is exactly what a profile needs
/// (`PROP-019#install-root`). It validates here and is expanded where the
/// root is resolved — never stored expanded, because the profile is read
/// by every machine.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-019#site-profile")]
fn an_install_root_may_start_at_the_readers_own_home() {
    let home = std::path::PathBuf::from("/home/me");
    for written in ["~", "~/opt", "~/work/vibe"] {
        let profile = SiteProfile::parse(
            &format!("schema = 1\n[self]\ninstall_root = '{written}'\n"),
            "corp.toml",
        )
        .unwrap_or_else(|error| panic!("`{written}`: {error}"));
        assert_eq!(
            profile.self_source.install_root.as_deref(),
            Some(std::path::Path::new(written)),
            "stored exactly as written"
        );
        let resolved = profile
            .self_source
            .install_root_at(Some(&home))
            .expect("declared")
            .expect("resolvable");
        assert!(resolved.starts_with(&home), "{}", resolved.display());
        assert_eq!(
            resolved,
            written
                .strip_prefix("~/")
                .map_or_else(|| home.clone(), |rest| home.join(rest))
        );
    }
    // No home to expand against is its own refusal, not a path called `~`.
    let profile =
        SiteProfile::parse("schema = 1\n[self]\ninstall_root = '~'\n", "corp.toml").unwrap();
    assert!(
        profile
            .self_source
            .install_root_at(None)
            .expect("declared")
            .is_err()
    );
}

/// The site layer of the one trust mechanism (PROP-065 `#ordinary-trust`):
/// an administrator's `[trust]` table is stored verbatim, its entries are
/// validated by the same parser the user's file goes through, and a bad
/// entry names its own key.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-019#site-profile")]
fn the_trust_table_is_stored_verbatim_and_validated_entry_by_entry() {
    let profile = SiteProfile::parse(
        "schema = 1\nname = 'corp'\n[trust]\ntrusted_groups = ['org.corp.*']\n\
         untrusted_groups = ['org.vibevm.ai-native']\n",
        "corp.toml",
    )
    .unwrap();
    assert_eq!(profile.trust.trusted_groups, ["org.corp.*"]);
    assert_eq!(profile.trust.untrusted_groups, ["org.vibevm.ai-native"]);

    // A profile that says nothing about trust carries the empty lists and
    // serialises without the table at all.
    let silent = SiteProfile::parse("schema = 1\n", "corp.toml").unwrap();
    assert!(silent.trust.is_empty());
    assert!(!toml::to_string(&silent).unwrap().contains("[trust]"));

    for (text, key) in [
        (
            "trusted_groups = ['org.*.tools']",
            "trust.trusted_groups[0]",
        ),
        ("untrusted_groups = ['', 'x']", "trust.untrusted_groups[0]"),
        (
            "untrusted_groups = ['org.ok', '**']",
            "trust.untrusted_groups[1]",
        ),
    ] {
        let error = SiteProfile::parse(&format!("schema = 1\n[trust]\n{text}\n"), "corp.toml")
            .unwrap_err()
            .to_string();
        assert!(error.contains(key), "`{text}` must name `{key}`: {error}");
        assert!(error.contains("corp.toml"), "{error}");
    }
}
