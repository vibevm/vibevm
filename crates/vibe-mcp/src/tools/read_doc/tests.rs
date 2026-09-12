//! What the tool decides, which is only ever three things: the argument
//! shape, the projection asked for, and where to look.
//!
//! What a projection CONTAINS is `vibe_doc::agent`'s law and is tested
//! there. Restating it here would be a second opinion about the same
//! bytes.

use std::fs;
use std::path::Path;

use super::*;
use crate::ServerContext;

const PAGE: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<spec xmlns=\"https://vibevm.org/spec/1\">\n  \
  <title id=\"root\">Boot lane</title>\n  \
  <p>The boot lane is what a session reads first.</p>\n\
</spec>\n";

fn project(root: &Path) {
    fs::write(
        root.join("vibe.toml"),
        "[project]\nname = \"host\"\ngroup = \"com.example\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();
    let dir = root.join("vibevm/vibepacks/com.example/thing-docs/v0.1.0");
    fs::create_dir_all(dir.join("vibevm/vibespecs/model")).unwrap();
    fs::write(
        dir.join("vibe.toml"),
        "[package]\nname = \"thing-docs\"\ngroup = \"com.example\"\nkind = \"doc\"\n\
         version = \"0.1.0\"\ntitle = \"Thing\"\nabstract = \"What it covers.\"\n\
         [i18n]\ncanonical = \"en\"\n\
         [[documents]]\npackage = \"com.example/thing\"\nversion = \"^1.0\"\n",
    )
    .unwrap();
    fs::write(dir.join("vibevm/vibespecs/model/boot-lane.xml"), PAGE).unwrap();
}

fn context(tmp: &tempfile::TempDir) -> ServerContext {
    project(tmp.path());
    ServerContext::with_store_root(tmp.path().to_path_buf(), tmp.path().join("store"))
}

const ADDRESS: &str = "spec://com.example/thing-docs/model/boot-lane";

/// The tool in one call: an address in, a page and the facts about it
/// out.
#[test]
fn an_address_comes_back_as_a_page_and_its_provenance() {
    let tmp = tempfile::tempdir().unwrap();
    let out = ReadDocMcpTool
        .run(&json!({ "address": ADDRESS }), &context(&tmp))
        .expect("the page is in the checkout's own packages");
    let value = out.structured();
    assert_eq!(value["format"], "md");
    assert_eq!(value["lang"], "en");
    assert_eq!(value["source"], "in-tree");
    assert!(
        value["page"].as_str().unwrap_or_default().contains("[p01]"),
        "{value}"
    );
}

/// The island is for a browser. A tool that returned it to an agent
/// would be charging tokens for markup nothing here reads.
#[test]
fn html_is_refused_by_name_rather_than_silently_substituted() {
    let tmp = tempfile::tempdir().unwrap();
    let e = ReadDocMcpTool
        .run(
            &json!({ "address": ADDRESS, "format": "html" }),
            &context(&tmp),
        )
        .expect_err("html is not a projection an agent reads");
    assert!(format!("{e:?}").contains("md"), "{e:?}");
}

/// An address is required, and its absence is an argument error rather
/// than an empty answer.
#[test]
fn a_missing_address_is_an_argument_error() {
    let tmp = tempfile::tempdir().unwrap();
    ReadDocMcpTool
        .run(&json!({}), &context(&tmp))
        .expect_err("`address` is required");
}

/// The descriptor is the whole contract an agent sees, so it names the
/// two projections and the fallback rule rather than leaving either to
/// be discovered.
#[test]
fn the_descriptor_teaches_the_projections_and_the_language_fallback() {
    let d = ReadDocMcpTool.descriptor();
    assert_eq!(d.name, "read_doc");
    assert!(d.description.contains("`md`"), "{}", d.description);
    assert!(d.description.contains("`xml`"), "{}", d.description);
    assert!(d.description.contains("falls back"), "{}", d.description);
    assert!(
        d.description.contains("vibe cache add"),
        "{}",
        d.description
    );
}
