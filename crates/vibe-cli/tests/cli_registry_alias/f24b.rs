//! Alias authoring refusals use the terminal display contract.
use super::*;

#[test]
#[specmark::spec(
    verifies = "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#short-name"
)]
fn alias_refusals_name_the_failed_operation() {
    let site = Site::new();
    let check = |output: Output, headline: &str, fixes: &[&str], see: &str| {
        assert!(!output.status.success(), "{}", stdout(&output));
        assert!(stdout(&output).is_empty(), "{}", stdout(&output));
        let said = stderr(&output);
        let lines: Vec<_> = said.lines().collect();
        let first = format!("✗ {headline}");
        assert_eq!(lines.first().copied(), Some(first.as_str()), "{said}");
        assert_eq!(said.matches("✗ ").count(), 1, "{said}");
        assert!(!said.contains('`'), "{said}");
        let mut previous = 0;
        for (i, fix) in fixes.iter().enumerate() {
            let line = if i == 0 {
                format!("  fix   {fix}")
            } else {
                format!("        {fix}")
            };
            let position = lines.iter().position(|value| *value == line).unwrap_or_else(|| {
                panic!("missing {line:?}: {said}")
            });
            assert!(position > previous, "{said}");
            previous = position;
        }
        assert_eq!(lines.last().copied(), Some(format!("  see   {see}").as_str()), "{said}");
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
    ok(&site.alias(&["add", "qwen", "org.vibevm.qwen/qwen-code"]), "seed alias");
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
    assert!(grammar.contains("lowercase letters, digits and inner single hyphens"), "{grammar}");
    let before = fs::read(site.path("reg/_index/aliases.toml")).unwrap();
    assert!(!String::from_utf8_lossy(&before).contains("Bad_Name"));
    fs::write(
        site.path("reg/_index/aliases.toml"),
        "schema = 1\n[aliases]\nBad_Name = 'org.vibevm.qwen/qwen-code'\nmissing = 'org.vibevm.qwen/absent'\n",
    ).unwrap();
    let faults = site.alias(&["add", "valid", "org.vibevm.qwen/qwen-code"]);
    assert!(!faults.status.success());
    let said = stderr(&faults);
    assert!(said.contains("Bad_Name is not a legal package name"), "{said}");
    assert!(said.contains("points INTO the registry"), "{said}");
    assert!(said.contains("org.vibevm.qwen/absent"), "{said}");
    fs::write(site.path("reg/_index/aliases.toml"), before).unwrap();
    fs::write(
        site.path("settings/registry.toml"),
        "[[registry]]\nname = 'gh'\ntype = 'github'\nurl = 'https://github.com/example'\n\
         [source_policy]\nexclude_hosting = ['github', 'gitverse']\n",
    ).unwrap();
    let unsupported = check(
        site.alias_in("gh", &["list"]),
        "alias_table_unsupported  registry gh is a github registry and carries no alias table",
        &["vibe-index alias"],
        short,
    );
    assert!(unsupported.contains("only localdir and monorepo"), "{unsupported}");
    // A local, non-repository tree forces a clone failure without contacting a service.
    fs::create_dir_all(site.path("not-a-repository")).unwrap();
    fs::write(
        site.path("settings/registry.toml"),
        format!("[[registry]]\nname = 'corp4'\ntype = 'monorepo'\nurl = 'file://{}'\n\
                 [source_policy]\nexclude_hosting = ['github', 'gitverse']\n",
                site.path("not-a-repository").display()),
    ).unwrap();
    let unreachable = check(
        site.alias_in("corp4", &["list"]),
        "registry_unreachable  could not open registry corp4 to read its alias table",
        &["vibe registry test", "vibe dl --test-proxy"],
        "spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-002#failure-discriminator",
    );
    assert!(unreachable.contains("  why   git "), "{unreachable}");
    assert!(!unreachable.contains("fatal:"), "{unreachable}");
    assert!(!unreachable.contains("file://"), "{unreachable}");
    assert!(!unreachable.contains("mirror"), "{unreachable}");
}
