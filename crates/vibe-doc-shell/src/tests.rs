use std::collections::BTreeMap;

use super::*;

/// Write a minimal but real shell into `root`.
fn place_shell(root: &Path, template: &str) -> Index {
    std::fs::create_dir_all(root.join("assets")).unwrap();
    std::fs::write(root.join(index::PAGE_TEMPLATE), template).unwrap();
    std::fs::write(root.join("assets").join("style.css"), "body{}").unwrap();
    let mut files: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    files.insert(
        index::PAGE_TEMPLATE.to_string(),
        template.as_bytes().to_vec(),
    );
    files.insert("assets/style.css".to_string(), b"body{}".to_vec());
    // The index is written last and is not part of its own digest, the
    // same way a lock file is not part of what it locks.
    let index = Index {
        schema: index::INDEX_SCHEMA,
        package: "org.vibevm.doc/web".to_string(),
        version: "0.1.0".to_string(),
        base: DEFAULT_BASE.to_string(),
        island_marker: ISLAND_MARKER.to_string(),
        files: files.len() as u32,
        sha256: digest::of(&files),
    };
    std::fs::write(root.join(index::INDEX_FILE), index.to_json()).unwrap();
    index
}

#[test]
fn the_bare_shell_is_a_readable_page_with_no_script_in_it() {
    let shell = Shell::bare();
    assert_eq!(shell.provenance(), Provenance::Fallback);
    let page = shell.page_template();
    assert!(page.contains(ISLAND_MARKER));
    assert!(!page.contains("<script"));
    // The bare shell needs no policy widening: it has no inline script.
    assert!(template::hashes(&page).is_empty());
}

#[test]
fn the_bare_shell_serves_its_stylesheet_and_nothing_else() {
    let shell = Shell::bare();
    assert!(shell.asset(fallback::STYLESHEET).is_some());
    assert!(shell.asset("assets/anything.js").is_none());
}

#[cfg(not(feature = "embedded-shell"))]
#[test]
fn nothing_embedded_and_nothing_named_is_the_bare_shell() {
    assert_eq!(Shell::open(None).provenance(), Provenance::Fallback);
}

#[cfg(feature = "embedded-shell")]
#[test]
fn the_embedded_shell_wins_over_everything_and_measures_to_its_pin() {
    let shell = Shell::open(None);
    assert_eq!(shell.provenance(), Provenance::Embedded);
    assert!(shell.page_template().contains(ISLAND_MARKER));
    let report = shell.report();
    assert!(
        report.matches_pin(),
        "the embedded shell does not match the pin:\n{}",
        report.render()
    );
    // The shell it carries is the real one: chunks, styles, faces.
    assert!(report.file_count > 1);
    assert!(!template::hashes(&shell.page_template()).is_empty());
}

#[test]
fn a_store_shell_is_opened_and_reports_where_it_came_from() {
    let tmp = tempfile::tempdir().unwrap();
    let placed = place_shell(
        tmp.path(),
        "<!doctype html><title>t</title><!--vibe-doc-island-->",
    );
    let shell = Shell::inspect(Some(tmp.path())).unwrap();
    // Under the embedded feature the compiled-in shell wins, which is
    // the point of the order; the store lane is what this asserts.
    if shell.provenance() == Provenance::Store {
        assert_eq!(shell.index().sha256, placed.sha256);
        assert_eq!(shell.measured_digest(), placed.sha256);
        assert!(shell.page_template().contains(ISLAND_MARKER));
    }
}

#[test]
fn a_named_store_directory_without_a_shell_is_a_refusal_not_a_fallback() {
    let tmp = tempfile::tempdir().unwrap();
    let outcome = Shell::inspect(Some(tmp.path()));
    #[cfg(not(feature = "embedded-shell"))]
    assert!(matches!(outcome, Err(ShellError::NoShell { .. })));
    // …and `open` never refuses: a reader always starts.
    let _ = outcome;
    assert!(matches!(
        Shell::open(Some(tmp.path())).provenance(),
        Provenance::Fallback | Provenance::Embedded
    ));
}

#[test]
fn an_asset_path_that_is_not_a_sequence_of_names_never_reaches_the_filesystem() {
    let tmp = tempfile::tempdir().unwrap();
    place_shell(tmp.path(), "<!--vibe-doc-island-->");
    std::fs::write(tmp.path().join("..").join("outside.txt"), "secret").ok();
    let shell = Shell::open(Some(tmp.path()));
    for spelling in [
        "../outside.txt",
        "assets/../../outside.txt",
        "assets\\style.css",
        "",
        "assets//style.css",
        // A drive prefix. `PathBuf::push` REPLACES the path on one of
        // these rather than extending it, so a name that looks like one
        // segment walks off the volume.
        "C:",
        "C:/Windows/win.ini",
        "assets/C:/Windows/win.ini",
    ] {
        assert!(
            shell.asset(spelling).is_none(),
            "`{spelling}` must not resolve"
        );
    }
    // …and the ordinary name beside them still does, so the refusals are
    // a rule about escapes and not a rule about reading anything. Only
    // on the store lane: under the feature the compiled-in shell wins and
    // carries its own names, which is the order working as intended.
    if shell.provenance() == Provenance::Store {
        assert!(shell.asset("assets/style.css").is_some());
    }
}

/// The join itself is checked, not only the name: the name says
/// «ordinary», and this says «still under the root the shell was placed
/// at» after the platform has had its say.
#[test]
fn a_join_that_leaves_the_shell_root_answers_nothing() {
    let root = Path::new("shell-root").join("sub");
    assert!(under(&root, "assets/style.css").is_some());
    assert!(under(&root, "C:").is_none() || !cfg!(windows));
}

#[test]
fn the_pin_parses_and_names_the_web_package() {
    let pin = Pin::compiled_in().unwrap();
    assert_eq!(pin.schema, pin::PIN_SCHEMA);
    assert_eq!(pin.package, "org.vibevm.doc/web");
}

#[test]
fn a_pin_of_another_schema_is_refused_rather_than_read() {
    let outcome = Pin::parse("schema = 99\npackage = \"a/b\"\nversion = \"1\"\nsha256 = \"\"\n");
    assert!(matches!(outcome, Err(ShellError::Pin { .. })));
}

#[test]
fn the_fallback_report_agrees_with_its_pin_vacuously() {
    let report = Shell::bare().report();
    assert!(report.matches_pin());
    assert!(report.render().contains("bare shell"));
}

#[test]
fn the_digest_is_a_property_of_the_tree_not_of_the_walk() {
    let tmp = tempfile::tempdir().unwrap();
    place_shell(tmp.path(), "<!--vibe-doc-island-->");
    let once = digest::read_tree(tmp.path()).unwrap();
    let twice = digest::read_tree(tmp.path()).unwrap();
    assert_eq!(digest::of(&once), digest::of(&twice));

    // One byte moved between two files, same total bytes.
    let mut moved = once.clone();
    moved.insert(index::PAGE_TEMPLATE.to_string(), b"ab".to_vec());
    moved.insert("assets/style.css".to_string(), b"c".to_vec());
    let mut other = once.clone();
    other.insert(index::PAGE_TEMPLATE.to_string(), b"a".to_vec());
    other.insert("assets/style.css".to_string(), b"bc".to_vec());
    assert_ne!(digest::of(&moved), digest::of(&other));
}

#[test]
fn inline_scripts_are_what_a_browser_would_execute() {
    let html = concat!(
        "<script async type=\"module\" src=\"/doc/build/q-a.js\"></script>",
        "<script type=\"module\" async>let b=fetch(\"/doc/assets/g.json\");</script>",
        "<script q:container=\"html\">var t=1;</script>",
        "<script type=\"qwik/state\">{\"refs\":{}}</script>",
        "<script type=\"application/ld+json\">{}</script>",
    );
    let bodies = template::inline_scripts(html);
    assert_eq!(bodies.len(), 2);
    assert!(bodies[0].starts_with("let b=fetch"));
    assert_eq!(bodies[1], "var t=1;");
    assert_eq!(template::hashes(html).len(), 2);
}

#[test]
fn a_tag_whose_attribute_carries_a_bracket_is_still_one_tag() {
    // The framework writes a script's own source into an attribute; a
    // scan to the first `>` would end the tag inside it.
    let html = "<script data-src=\"a>b\">var t=1;</script>";
    assert_eq!(template::inline_scripts(html), vec!["var t=1;"]);
}

#[test]
fn data_src_is_not_src() {
    let html = "<style data-src=\"/a.css\"></style><script data-src=\"/a.js\">var t=1;</script>";
    assert_eq!(template::inline_scripts(html), vec!["var t=1;"]);
}

#[test]
fn the_island_replaces_the_marker_once_and_the_title_follows_the_page() {
    let page = template::glue(
        "<title>Built page</title><main><!--vibe-doc-island--></main>",
        ISLAND_MARKER,
        "<h1>Real</h1>",
    );
    assert_eq!(page, "<title>Built page</title><main><h1>Real</h1></main>");
    let retitled = template::retitle(&page, "Real & true");
    assert!(retitled.contains("<title>Real &amp; true</title>"));
}

#[test]
fn the_templates_fixture_projections_are_replaced_by_the_served_pages_own() {
    let html = concat!(
        "<head><title>t</title>",
        "<link rel=\"alternate\" type=\"text/markdown\" href=\"/doc/fixture.md\">",
        "<link rel=\"stylesheet\" href=\"/doc/assets/a.css\">",
        "</head>",
    );
    let out = template::relink(
        html,
        &[template::Alternate {
            media_type: "text/markdown",
            href: "/doc/org.example/a/1.0.0/guide.md",
            title: "This page as Markdown",
        }],
    );
    assert!(!out.contains("/doc/fixture.md"));
    assert!(out.contains("href=\"/doc/assets/a.css\""));
    assert!(out.contains("href=\"/doc/org.example/a/1.0.0/guide.md\""));
    assert_eq!(out.matches("rel=\"alternate\"").count(), 1);
}

#[test]
fn rebasing_moves_the_shells_own_addresses_and_leaves_an_equal_base_alone() {
    let html = "<link href=\"/doc/assets/a.css\"><script src=\"/doc/build/b.js\">";
    assert_eq!(template::rebase(html, "/doc/", "/doc/"), html);
    let moved = template::rebase(html, "/doc/", "/manual/");
    assert!(moved.contains("/manual/assets/a.css"));
    assert!(!moved.contains("/doc/"));
}
