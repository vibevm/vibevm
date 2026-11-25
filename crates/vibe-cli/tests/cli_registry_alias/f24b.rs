//! Alias authoring refusals use the terminal display contract.
use super::*;

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#short-name"
)]
fn alias_refusals_name_the_failed_operation() {
    let site = Site::new();
    let check = |output: Output, headline: &str, fixes: &[&str], see: &str| {
        assert!(!output.status.success(), "expected refusal");
        assert!(stdout(&output).is_empty(), "refusal wrote to stdout");
        let said = stderr(&output);
        let lines: Vec<_> = said.lines().collect();
        let first = format!("✗ {headline}");
        assert!(
            lines.first().copied() == Some(first.as_str()),
            "refusal headline mismatch"
        );
        assert_eq!(
            said.matches("✗ ").count(),
            1,
            "refusal display contract mismatch"
        );
        assert!(!said.contains('`'), "refusal display contract mismatch");
        let mut previous = 0;
        for (i, fix) in fixes.iter().enumerate() {
            let line = if i == 0 {
                format!("  fix   {fix}")
            } else {
                format!("        {fix}")
            };
            let position = lines
                .iter()
                .position(|value| *value == line)
                .unwrap_or_else(|| panic!("required fix line missing"));
            assert!(position > previous, "refusal display contract mismatch");
            previous = position;
        }
        assert!(
            lines.last().copied() == Some(format!("  see   {see}").as_str()),
            "refusal reference mismatch"
        );
        said
    };
    let short = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#short-name";
    check(
        site.alias_in("absent", &["list"]),
        "registry_not_found  no registry named absent is configured",
        &["vibe registry list"],
        short,
    );
    check(
        site.alias(&["remove", "qwen"]),
        "alias_not_issued  registry corp issues no short name qwen",
        &["vibe registry alias corp list"],
        short,
    );
    assert!(
        site.alias(&["add", "qwen", "org.vibevm.qwen/qwen-code"])
            .status
            .success(),
        "seed alias failed"
    );
    check(
        site.alias(&["add", "qwen", "org.vibevm.qwen/other"]),
        "alias_exists  qwen already means org.vibevm.qwen/qwen-code in registry corp",
        &["vibe registry alias corp add qwen org.vibevm.qwen/other --replace"],
        short,
    );
    let grammar = check(
        site.alias(&["add", "Bad_Name", "org.vibevm.qwen/qwen-code"]),
        "invalid_alias  Bad_Name is not a legal package name",
        &["vibe registry alias corp add bad-name org.vibevm.qwen/qwen-code"],
        "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#pkgref",
    );
    assert!(
        grammar.contains("lowercase letters, digits and inner single hyphens"),
        "grammar explanation missing"
    );
    let before = fs::read(site.path("reg/_index/aliases.toml")).unwrap();
    assert!(!String::from_utf8_lossy(&before).contains("Bad_Name"));
    fs::write(
        site.path("reg/_index/aliases.toml"),
        "schema = 1\n[aliases]\nBad_Name = 'org.vibevm.qwen/qwen-code'\nmissing = 'org.vibevm.qwen/absent'\n",
    ).unwrap();
    let faults = site.alias(&["add", "valid", "org.vibevm.qwen/qwen-code"]);
    assert!(!faults.status.success());
    let said = stderr(&faults);
    assert!(
        said.contains("Bad_Name is not a legal package name"),
        "refusal display contract mismatch"
    );
    assert!(
        said.contains("points INTO the registry"),
        "refusal display contract mismatch"
    );
    assert!(
        said.contains("org.vibevm.qwen/absent"),
        "refusal display contract mismatch"
    );
    fs::write(site.path("reg/_index/aliases.toml"), before).unwrap();
    fs::write(
        site.path("settings/registry.toml"),
        "[[registry]]\nname = 'gh'\ntype = 'github'\nurl = 'https://github.com/example'\n\
         [source_policy]\nexclude_hosting = ['github', 'gitverse']\n",
    )
    .unwrap();
    let unsupported = check(
        site.alias_in("gh", &["list"]),
        "alias_table_unsupported  registry gh is a github registry and carries no alias table",
        &["vibe-index alias"],
        short,
    );
    assert!(
        unsupported.contains("only localdir and monorepo"),
        "unsupported-layout explanation missing"
    );
    // A local, non-repository tree forces a clone failure without contacting a service.
    fs::create_dir_all(site.path("not-a-repository")).unwrap();
    fs::write(
        site.path("settings/registry.toml"),
        format!(
            "[[registry]]\nname = 'corp4'\ntype = 'monorepo'\nurl = 'file://{}'\n\
                 [source_policy]\nexclude_hosting = ['github', 'gitverse']\n",
            site.path("not-a-repository").display()
        ),
    )
    .unwrap();
    let unreachable = check(
        site.alias_in("corp4", &["list"]),
        "registry_unreachable  could not open registry corp4 to read its alias table",
        &["vibe registry test", "vibe dl --test-proxy"],
        "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator",
    );
    assert!(
        unreachable.contains("  why   git "),
        "registry-open diagnostic contract mismatch"
    );
    assert!(
        !unreachable.contains("fatal:"),
        "registry-open diagnostic contract mismatch"
    );
    assert!(
        !unreachable.contains("file://"),
        "registry-open diagnostic contract mismatch"
    );
    assert!(
        !unreachable.contains("mirror"),
        "registry-open diagnostic contract mismatch"
    );
    assert!(
        !unreachable.contains("registry snapshot candidate failed"),
        "default fallback warning leaked"
    );
    // Repeat the identical alias operation and local directory with explicit
    // warning logging: F19 owns visibility, not the alias refusal adapter.
    let diagnostic = Command::new(env!("CARGO_BIN_EXE_vibe"))
        .current_dir(site.root.path())
        .args(["registry", "alias", "corp4", "list"])
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("HOME", site.path("home"))
        .env("USERPROFILE", site.path("home"))
        .env("VIBE_SETTINGS", site.path("settings"))
        .env("VIBE_REGISTRY_CACHE", site.path("cache"))
        .env("VIBE_NO_DEFAULT_REGISTRY", "1")
        .env("CI", "1")
        .env("NO_COLOR", "1")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", site.path("empty"))
        .env("VIBE_LOG", "warn")
        .output()
        .expect("explicit diagnostic alias child failed to start");
    assert!(
        !diagnostic.status.success(),
        "explicit diagnostic operation must refuse"
    );
    assert!(
        diagnostic.stdout.is_empty(),
        "explicit diagnostic refusal wrote stdout"
    );
    let diagnostic = stderr(&diagnostic);
    assert!(
        diagnostic.contains("registry snapshot candidate failed"),
        "explicit warning diagnostic was suppressed"
    );
    assert!(
        diagnostic.contains("registry_unreachable"),
        "explicit diagnostic lost alias refusal"
    );
}
