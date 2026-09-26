//! `[glossary]` — the grammar of the page a documentation defines its
//! terms on (PROP-057 `##GLOSSARY-DECLARED`).
//!
//! A file of its own beside `tests_documentation.rs` for the reason the
//! validator is a module of its own: the documentation half of the manifest
//! stands at its length budget, and the glossary is one table with its own
//! seam.

use super::*;

/// A `doc` package with everything required and nothing more, as a TOML
/// body the tests append to.
const DOC_HEAD: &str = "[package]\nname = \"vibevm-docs\"\ngroup = \"org.vibevm.core\"\n\
                        kind = \"doc\"\nversion = \"0.1.0\"\ntitle = \"VibeVM Manual\"\n\
                        abstract = \"What it covers, for whom, what it assumes known, what it \
                        leaves out.\"\n";

const SUBJECT: &str = "\n[[documents]]\npackage = \"org.vibevm.core/vibevm\"\nversion = \"^1.0\"\n";

fn doc_manifest(extra: &str) -> Result<Manifest> {
    Manifest::parse_str(&format!("{DOC_HEAD}{SUBJECT}{extra}"))
}

fn refusal(extra: &str) -> String {
    doc_manifest(extra)
        .expect_err("this manifest must be refused")
        .to_string()
}

/// A documentation names the page that defines its terms, and the page is
/// spelled as every other document path in the manifest is.
#[test]
fn a_documentation_may_declare_its_glossary() {
    let manifest = doc_manifest("\n[glossary]\npage = \"glossary/index\"\n")
        .expect("a declared glossary parses");
    let declared = manifest.glossary.as_ref().expect("the table is there");
    assert_eq!(declared.page, "glossary/index");

    // And a documentation that declares none has none: the path means
    // nothing by itself (`##GLOSSARY-DECLARED`).
    assert!(
        doc_manifest("")
            .expect("no table is legal")
            .glossary
            .is_none()
    );
}

/// The extension is the pipeline's business — one document is served as
/// three projections — so a path that carries one is refused where the
/// author can still fix the spelling.
#[test]
fn a_glossary_page_carrying_an_extension_is_refused() {
    for bad in [
        "glossary/index.xml",
        "/glossary/index",
        "../glossary/index",
        "glossary//index",
        "glossary/ index",
        "",
    ] {
        let message = refusal(&format!("\n[glossary]\npage = \"{bad}\"\n"));
        assert!(message.contains("[glossary].page"), "`{bad}`: {message}");
        assert!(
            message.contains("spec://org.vibevm.core/vibevm/common/PROP-057#GLOSSARY-DECLARED"),
            "`{bad}`: {message}"
        );
    }
}

/// A field this table does not have is a field the author meant for
/// somewhere else, and silence would hide the mistake.
#[test]
fn an_unknown_glossary_field_is_refused() {
    let message = refusal("\n[glossary]\npage = \"glossary/index\"\ntitle = \"Terms\"\n");
    assert!(message.contains("title"), "{message}");
}

/// Only documentation has pages, so only documentation may name one as its
/// glossary.
#[test]
fn only_a_doc_package_may_declare_a_glossary() {
    let message = Manifest::parse_str(
        "[package]\nname = \"wal\"\ngroup = \"org.vibevm\"\nkind = \"flow\"\nversion = \"0.1.0\"\n\
         \n[glossary]\npage = \"glossary/index\"\n",
    )
    .expect_err("a flow has no pages")
    .to_string();
    assert!(message.contains("kind = \"flow\""), "{message}");
    assert!(
        message.contains("spec://org.vibevm.core/vibevm/common/PROP-057#GLOSSARY-DECLARED"),
        "{message}"
    );
}

/// A translation declares the same glossary as the documentation it
/// adapts, which the grammar cannot check — it has one manifest in hand —
/// so what it does here is let the table through unchanged.
#[test]
fn a_translation_declares_its_own_glossary_table() {
    let manifest = doc_manifest(
        "\n[translates]\npackage = \"org.vibevm.core/vibevm-docs\"\nversion = \"^1.0\"\n\
         \n[glossary]\npage = \"glossary/index\"\n",
    )
    .expect("an adaptation declares the same page");
    assert_eq!(
        manifest.glossary.as_ref().map(|g| g.page.as_str()),
        Some("glossary/index")
    );
}
