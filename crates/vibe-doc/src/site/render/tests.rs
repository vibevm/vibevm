//! One package rendered, and one package refused.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use chrono::{TimeZone, Utc};

use super::*;

/// A composition root that answers both questions the way a test needs:
/// the bytes are already where they are, and no product is run.
struct Handy {
    source: PathBuf,
}

impl Prepare for Handy {
    fn warm(&self, _pair: &Pair) -> Result<PathBuf> {
        Ok(self.source.clone())
    }

    fn derived(&self, _pair: &Pair, _package_dir: &Path) -> BTreeMap<String, String> {
        BTreeMap::new()
    }
}

/// A composition root that cannot bring the bytes — the state a broken
/// registry entry puts the builder in.
struct Unreachable;

impl Prepare for Unreachable {
    fn warm(&self, pair: &Pair) -> Result<PathBuf> {
        Err(DocError::Site {
            message: format!(
                "`{}` could not be fetched: the mirror is down",
                pair.spelled()
            ),
        })
    }

    fn derived(&self, _pair: &Pair, _package_dir: &Path) -> BTreeMap<String, String> {
        BTreeMap::new()
    }
}

fn pair() -> Pair {
    Pair {
        source: "vibespecs".into(),
        group: "org.example".into(),
        name: "wal".into(),
        version: "1.2.0".into(),
        content_hash: "sha256:aa".into(),
        origin: Origin::Registry,
        entry: None,
    }
}

fn package(root: &Path) {
    fs::create_dir_all(root).expect("the package");
    fs::write(
        root.join("vibe.toml"),
        "[package]\nname = \"wal\"\ngroup = \"org.example\"\nversion = \"1.2.0\"\nkind = \"flow\"\n\
         description = \"A log.\"\n",
    )
    .expect("the manifest");
    fs::write(root.join("README.md"), "# wal\n\nA log.\n").expect("the README");
}

/// A page that quotes a rule — the one block a site build is likeliest to
/// be unable to fill, because the registry publishes a documentation
/// whether or not this machine holds its subject.
const CITING_PAGE: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<spec xmlns=\"https://vibevm.org/spec/1\">\n  \
  <title id=\"root\">What it quotes</title>\n  \
  <status stage=\"doc\" state=\"work\" audience=\"user\"/>\n  \
  <p>The rule below is cited from a specification nothing here carries.</p>\n  \
  <the-rule title=\"The rule\">\n    \
    <rule ref=\"spec://com.example/subject/common/PROP-001#A-RULE\"/>\n  \
  </the-rule>\n\
</spec>\n";

/// The same package, carrying one page whose citation no world here
/// resolves.
fn citing_package(root: &Path) {
    package(root);
    let dir = root.join(crate::pages::SPEC_ROOT).join("guide");
    fs::create_dir_all(&dir).expect("the spec root");
    fs::write(dir.join("cited.xml"), CITING_PAGE).expect("the page");
}

/// A package the catalog says nothing about — `static`, not `const`,
/// because the options borrow it and a `const` is a fresh temporary at
/// every mention.
static NOTHING: level0::Related = level0::Related {
    documentation: Vec::new(),
    translations: Vec::new(),
    dependants: Vec::new(),
    adaptation: None,
};

fn options<'a>(work: &'a Path, sources: &'a SpecSources) -> Options<'a> {
    Options {
        base: "/doc/",
        sources,
        work,
        rendered_at: Utc.with_ymd_and_hms(2026, 9, 12, 12, 0, 0).unwrap(),
        related: &NOTHING,
    }
}

#[test]
fn a_package_is_rendered_in_every_projection_the_site_serves() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    let source = tmp.path().join("src");
    package(&source);
    let work = tmp.path().join("trees");
    let sources = SpecSources::new();

    let rendered = render(
        &pair(),
        &Handy {
            source: source.clone(),
        },
        &options(&work, &sources),
    )
    .expect("a render");

    assert!(rendered.ok(), "{:?}", rendered.failed);
    assert_eq!(rendered.trees.len(), FORMATS.len());
    for (tree, format) in rendered.trees.iter().zip(FORMATS) {
        assert!(tree.is_dir(), "{tree:?}");
        assert!(
            tree.ends_with(format.as_str()),
            "{tree:?} is not the {format:?} tree"
        );
    }
    assert!(rendered.files > 0);
}

/// The address map is the whole point: a page has to land where a
/// citation to it would look.
#[test]
fn the_pages_land_at_the_address_the_site_mounts_them_under() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    let source = tmp.path().join("src");
    package(&source);
    let work = tmp.path().join("trees");
    let sources = SpecSources::new();
    let rendered = render(&pair(), &Handy { source }, &options(&work, &sources)).expect("a render");

    let html = &rendered.trees[0];
    assert!(
        html.join("org.example/wal/1.2.0/readme/index.html")
            .is_file(),
        "the README page is not at its address"
    );
    assert!(
        html.join("org.example/wal/1.2.0/manifest/index.html")
            .is_file(),
        "the manifest page is not at its address"
    );
    assert!(html.join("manifest.json").is_file());
}

/// A reference this render could not fill travels out with the render,
/// counted ONCE for the pair.
///
/// Three projections come out of one composed package, one world and one
/// set of generated texts, so the three builds agree by construction and a
/// count that added them up would report a page's single unfilled citation
/// as three. Saying it three times is the same defect as not saying it: the
/// number stops being something an operator can compare against the page.
#[test]
fn a_reference_a_render_cannot_fill_is_counted_once_for_the_pair() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    let source = tmp.path().join("src");
    citing_package(&source);
    let work = tmp.path().join("trees");
    let sources = SpecSources::new();

    let rendered = render(&pair(), &Handy { source }, &options(&work, &sources)).expect("a render");

    assert!(rendered.ok(), "{:?}", rendered.failed);
    assert_eq!(rendered.trees.len(), FORMATS.len());
    assert_eq!(
        rendered.unresolved,
        build::Unresolved {
            examples: 0,
            rules: 1,
            derived: 0,
        }
    );
}

/// A registry of hundreds will always hold one broken package, and a
/// builder that stopped for it would publish nothing at all.
#[test]
fn a_package_that_cannot_be_fetched_becomes_a_page_and_not_a_stop() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    let work = tmp.path().join("trees");
    let sources = SpecSources::new();

    let rendered =
        render(&pair(), &Unreachable, &options(&work, &sources)).expect("a page, not a refusal");

    assert!(!rendered.ok());
    let reason = rendered.failed.as_deref().expect("a reason");
    assert!(reason.contains("the mirror is down"), "{reason}");

    let html = &rendered.trees[0];
    let page = html.join("org.example/wal/1.2.0/render-failed/index.html");
    assert!(page.is_file(), "the failure has no page");
    let text = fs::read_to_string(page).expect("the page");
    assert!(text.contains("the mirror is down"), "{text}");
    // The version's own address carries a card that says what happened,
    // so the package page is not an empty shelf.
    let manifest = fs::read_to_string(html.join("manifest.json")).expect("the manifest");
    assert!(manifest.contains("did not render"), "{manifest}");
}

/// A package that renders after a failure must leave no trace of the
/// failure behind.
#[test]
fn a_retry_that_succeeds_replaces_the_failure_page() {
    let tmp = tempfile::tempdir().expect("a temporary directory");
    let source = tmp.path().join("src");
    let work = tmp.path().join("trees");
    let sources = SpecSources::new();

    render(&pair(), &Unreachable, &options(&work, &sources)).expect("the failure");
    package(&source);
    let rendered =
        render(&pair(), &Handy { source }, &options(&work, &sources)).expect("the retry");

    assert!(rendered.ok());
    let html = &rendered.trees[0];
    assert!(
        !html
            .join("org.example/wal/1.2.0/render-failed/index.html")
            .exists(),
        "the failure page survived a successful render"
    );
}

/// A renderer's message may hold anything at all, and none of it may
/// become markup of the page that shows it.
#[test]
fn a_reason_with_markup_in_it_is_shown_as_text() {
    let page = failed_page(&pair(), "<script>alert(1)</script> & \"quotes\"");
    assert!(!page.contains("<script>"), "{page}");
    assert!(page.contains("&lt;script&gt;"), "{page}");
    vibe_specdoc::from_xml_with(&page, vibe_specdoc::doc::Vocabulary::Doc)
        .unwrap_or_else(|e| panic!("the pivot refused the failure page: {e}\n{page}"));
}

#[test]
fn a_reason_with_a_quotation_mark_still_makes_a_manifest() {
    let written = failed_manifest(&pair(), "the path \"a\\b\" is not there");
    let back: toml::Value = toml::from_str(&written).expect("the manifest parses");
    let abstract_ = back
        .get("package")
        .and_then(|p| p.get("abstract"))
        .and_then(toml::Value::as_str)
        .expect("the abstract");
    assert!(abstract_.contains("\"a\\b\""), "{abstract_}");
}

/// Two runs must key one coordinate to one directory, or an unchanged
/// package would be handed to the static build under a new name every
/// time.
#[test]
fn a_coordinate_keys_one_directory_at_every_render() {
    let work = Path::new("/work");
    assert_eq!(
        tree_dir(work, &pair(), Format::Html),
        tree_dir(work, &pair(), Format::Html)
    );
    assert_ne!(
        tree_dir(work, &pair(), Format::Html),
        tree_dir(work, &pair(), Format::Md)
    );
}
