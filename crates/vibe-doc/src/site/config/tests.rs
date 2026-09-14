//! What a configuration means, including the halves nobody wrote down.

use super::*;

/// The configuration a deployment writes when it wants the public site,
/// spelled the way the example file spells it.
const FULL: &str = r#"
schema = 1

[[source.registry]]
name = "vibespecs"
url = "https://github.com/vibespecs/"
index_url = "https://github.com/vibespecs/index"
naming = "kind/name"

[source.host]
git = "https://github.com/vibevm/vibevm"
ref = "main"
checkout = "checkout"
debounce_minutes = 60

[site]
base_path = "/doc"
origin = "https://vibevm.org/"
default_theme = "system"

[site.analytics]
website_id = ""
host_url = ""
"#;

fn read(text: &str) -> Result<Site> {
    let parsed: DocSiteConfig = toml::from_str(text).expect("the fixture parses");
    Site::resolve(Path::new("/deploy/site.toml"), parsed)
}

#[test]
fn a_full_configuration_reads_every_value_it_declares() {
    let site = read(FULL).unwrap();
    assert_eq!(site.registries.len(), 1);
    let registry = &site.registries[0];
    assert_eq!(registry.name, "vibespecs");
    // The trailing slash is removed: `<url>/index` must not become
    // `<url>//index`.
    assert_eq!(registry.url, "https://github.com/vibespecs");
    assert_eq!(
        registry.index_url.as_deref(),
        Some("https://github.com/vibespecs/index")
    );
    assert_eq!(registry.naming, NamingConvention::KindSlashName);

    let host = site.host.as_ref().unwrap();
    assert_eq!(host.git, "https://github.com/vibevm/vibevm");
    assert_eq!(host.git_ref, "main");
    assert_eq!(host.debounce_minutes, 60);
    assert_eq!(host.checkout, Path::new("/deploy").join("checkout"));

    assert_eq!(site.base, "/doc/");
    assert_eq!(site.origin, "https://vibevm.org");
    assert_eq!(site.default_theme, Theme::System);
    assert!(site.analytics.is_silent());
}

/// The whole point of the resolved layer: an absent member has ONE
/// meaning, and it is written down here rather than in each reader.
#[test]
fn every_absent_member_has_one_meaning() {
    let site = read(
        r#"
schema = 1
[source.host]
git = "https://example.invalid/host"
"#,
    )
    .unwrap();
    let host = site.host.as_ref().unwrap();
    assert_eq!(host.git_ref, DEFAULT_HOST_REF);
    assert_eq!(host.debounce_minutes, DEFAULT_DEBOUNCE_MINUTES);
    // A checkout nobody named is the directory the configuration sits in.
    assert_eq!(host.checkout, Path::new("/deploy"));
    assert_eq!(site.base, crate::content::SITE_BASE);
    assert_eq!(site.origin, DEFAULT_ORIGIN);
    assert_eq!(site.default_theme, Theme::System);
    assert!(site.analytics.is_silent());
    // A first-party tag reports to the domain it is served by.
    assert_eq!(site.analytics.host_url, DEFAULT_ORIGIN);
}

#[test]
fn a_registry_with_no_naming_takes_the_group_aware_convention() {
    let site = read(
        r#"
schema = 1
[[source.registry]]
name = "vibespecs"
url = "https://github.com/vibespecs"
"#,
    )
    .unwrap();
    assert_eq!(site.registries[0].naming, NamingConvention::default());
    assert!(site.registries[0].index_url.is_none());
}

/// `/doc`, `doc/` and `/doc/` are one value written three ways, and the
/// rest of the pipeline accepts exactly the third.
#[test]
fn the_mount_path_comes_out_in_one_spelling() {
    for declared in ["/doc", "doc", "doc/", "/doc/"] {
        assert_eq!(mount(Some(declared)), "/doc/", "for `{declared}`");
    }
    assert_eq!(mount(Some("/")), "/");
    assert_eq!(mount(None), crate::content::SITE_BASE);
}

/// A build with nothing to read is a mistake in the configuration, and an
/// empty output deployed over the previous render is what refusing it
/// prevents.
#[test]
fn a_configuration_with_no_source_is_refused() {
    let message = read("schema = 1\n").unwrap_err().to_string();
    assert!(message.contains("names no source"), "{message}");
    assert!(message.contains("[[source.registry]]"), "{message}");
    assert!(message.contains("[source.host]"), "{message}");
}

#[test]
fn a_schema_this_build_does_not_read_is_refused_by_both_numbers() {
    let message = read(
        r#"
schema = 99
[source.host]
git = "https://example.invalid/host"
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(message.contains("schema = 99"), "{message}");
    assert!(message.contains(&SCHEMA_VERSION.to_string()), "{message}");
}

#[test]
fn a_naming_convention_nobody_defined_is_refused_with_the_ones_that_exist() {
    let message = read(
        r#"
schema = 1
[[source.registry]]
name = "r"
url = "https://example.invalid/r"
naming = "reverse-dns"
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(message.contains("reverse-dns"), "{message}");
    for word in ["fqdn", "kind-name", "name", "kind/name"] {
        assert!(message.contains(word), "`{word}` missing from: {message}");
    }
}

#[test]
fn a_theme_nobody_defined_is_refused_with_the_ones_that_exist() {
    let message = read(
        r#"
schema = 1
[source.host]
git = "https://example.invalid/host"
[site]
default_theme = "sepia"
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(message.contains("sepia"), "{message}");
    for word in ["system", "light", "dark"] {
        assert!(message.contains(word), "`{word}` missing from: {message}");
    }
}

/// An origin is not optional in the sense that it may be nothing:
/// `canonical`, `hreflang` and the sitemap are absolute addresses, and an
/// empty origin builds a page that claims to live at `/`.
#[test]
fn an_empty_origin_is_refused() {
    let message = read(
        r#"
schema = 1
[source.host]
git = "https://example.invalid/host"
[site]
origin = "/"
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(message.contains("origin"), "{message}");
}

#[test]
fn an_absolute_checkout_is_left_where_it_was_written() {
    let site = read(
        r#"
schema = 1
[source.host]
git = "https://example.invalid/host"
checkout = "/srv/vibevm"
"#,
    )
    .unwrap();
    assert_eq!(site.host.unwrap().checkout, Path::new("/srv/vibevm"));
}

#[test]
fn an_analytics_property_that_names_an_id_is_not_silent() {
    let site = read(
        r#"
schema = 1
[source.host]
git = "https://example.invalid/host"
[site.analytics]
website_id = "abc"
host_url = "https://stats.example.invalid/"
"#,
    )
    .unwrap();
    assert!(!site.analytics.is_silent());
    assert_eq!(site.analytics.website_id, "abc");
    assert_eq!(site.analytics.host_url, "https://stats.example.invalid");
}

/// The report is the defence against a misspelled key, so it has to show
/// the values nobody wrote as well as the ones they did.
#[test]
fn the_report_prints_what_was_resolved_and_not_only_what_was_written() {
    let report = read(
        r#"
schema = 1
[source.host]
git = "https://example.invalid/host"
"#,
    )
    .unwrap()
    .render();
    assert!(report.contains("registry none"), "{report}");
    assert!(report.contains("@main"), "{report}");
    assert!(report.contains("debounce 60 min"), "{report}");
    assert!(report.contains("/doc/"), "{report}");
    assert!(report.contains("no website id"), "{report}");
}

/// The featured list is an editorial ranking, so it keeps the order it
/// was written in; a deployment that names none features none.
#[test]
fn the_featured_documentations_are_read_in_the_order_they_were_written() {
    let site = read(
        r#"
schema = 1
[source.host]
git = "https://example.invalid/host"
[site]
featured = ["org.vibevm.core/vibevm-docs", " org.vibevm.core/vibevm ", ""]
"#,
    )
    .unwrap();
    assert_eq!(
        site.featured,
        vec!["org.vibevm.core/vibevm-docs", "org.vibevm.core/vibevm"]
    );
    assert!(
        site.render()
            .contains("featured org.vibevm.core/vibevm-docs")
    );

    let silent = read(
        r#"
schema = 1
[source.host]
git = "https://example.invalid/host"
"#,
    )
    .unwrap();
    assert!(silent.featured.is_empty());
    assert!(
        silent.render().contains("featured none"),
        "{}",
        silent.render()
    );
}

/// A coordinate nothing publishes is a warning and never a refusal: the
/// registry moves without this file, and a renamed package must not cost
/// the domain a render.
#[test]
fn a_featured_coordinate_no_source_publishes_is_reported_and_not_refused() {
    let site = read(
        r#"
schema = 1
[source.host]
git = "https://example.invalid/host"
[site]
featured = ["org.vibevm.core/vibevm", "org.vibevm.core/gone"]
"#,
    )
    .unwrap();
    let published = BTreeSet::from(["org.vibevm.core/vibevm".to_string()]);
    assert_eq!(
        site.featured_absent(&published),
        vec!["org.vibevm.core/gone"]
    );
    assert!(site.featured_absent(&BTreeSet::new()).len() == 2);
}

/// A misspelled table is a source that is not there, and the report is
/// where that becomes visible in the first line of a run.
#[test]
fn a_misspelled_table_shows_up_as_an_absent_source() {
    let message = read(
        r#"
schema = 1
[souce.host]
git = "https://example.invalid/host"
"#,
    )
    .unwrap_err()
    .to_string();
    assert!(message.contains("names no source"), "{message}");
}
