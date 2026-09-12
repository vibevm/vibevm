//! The three pages a level-0 render writes
//! (PROP-057 `##LEVEL-ZERO`).
//!
//! The manifest as a reference page, the README, and the boot snippet
//! marked «read by the session». Everything else `##LEVEL-ZERO` names —
//! the specifications with their anchors — is the package's own text and
//! is copied at its own path rather than composed here.
//!
//! ## They are written into the spec root, under reserved names
//!
//! A generated page is a page: it has an address, it appears in the
//! manifest, it is projected into Markdown and XML like every other one.
//! Writing it anywhere else would make it a fourth kind of artefact with
//! its own rules. The names are reserved rather than owned — a package
//! that already carries a document called `readme.xml` keeps its own,
//! and the run says so. An authored page is always the better page.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#LEVEL-ZERO");

use std::path::Path;

use super::card::Manifest;
use super::readme::{self, Block};
use super::{Composed, Related};
use crate::error::{DocError, Result};
use crate::pages::SPEC_ROOT;

/// Where the manifest reference page is served from.
pub const MANIFEST_PAGE: &str = "manifest.xml";

/// Where the README is served from.
pub const README_PAGE: &str = "readme.xml";

/// Where the boot snippet is served from.
pub const BOOT_SNIPPET_PAGE: &str = "boot-snippet.xml";

/// Write the composed pages of a level-0 render.
pub fn compose(
    source: &Path,
    work: &Path,
    manifest: &Manifest,
    related: &Related,
    composed: &mut Composed,
) -> Result<()> {
    write(
        work,
        MANIFEST_PAGE,
        manifest_page(manifest, related),
        composed,
    )?;
    if let Some(text) = read_readme(source) {
        write(work, README_PAGE, readme_page(manifest, &text), composed)?;
    }
    if let Some(snippet) = manifest.table("boot_snippet") {
        write(
            work,
            BOOT_SNIPPET_PAGE,
            boot_snippet_page(manifest, snippet),
            composed,
        )?;
    }
    Ok(())
}

/// Write one composed page, unless the package already carries a
/// document at that address.
fn write(work: &Path, name: &str, body: String, composed: &mut Composed) -> Result<()> {
    let path = work.join(SPEC_ROOT).join(name);
    if path.is_file() {
        composed.notes.push(format!(
            "`{name}` is a page this package wrote, so the generated one was not \
             written — an authored page is the better page"
        ));
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| DocError::io("creating", parent, e))?;
    }
    std::fs::write(&path, body).map_err(|e| DocError::io("writing", &path, e))?;
    composed.generated.push(name.to_string());
    Ok(())
}

/// The README of a package, under either spelling of the name.
fn read_readme(source: &Path) -> Option<String> {
    for name in ["README.md", "readme.md"] {
        if let Ok(text) = std::fs::read_to_string(source.join(name)) {
            return Some(text);
        }
    }
    None
}

/// The manifest as a reference page: the card's facts as a table, then
/// what the package declares it ships, then the manifest itself.
///
/// The verbatim manifest is at the end and not instead of the table.
/// The table answers the questions a reader has — what is this, whose is
/// it, under what licence — and the fence answers the one a reader has
/// after that, which is «show me». A page that only quoted the file
/// would make every reader parse TOML to learn the licence.
fn manifest_page(manifest: &Manifest, related: &Related) -> String {
    let mut out = open("Package manifest", "user,dev");
    out.push_str(&paragraph(&format!(
        "What `{}/{}@{}` declares about itself. Everything on this page is the \
         package's own manifest, read and shown — nothing here is an opinion about \
         the package.",
        manifest.group, manifest.name, manifest.version
    )));

    let mut rows = vec![
        (
            "Coordinate",
            format!("`{}/{}`", manifest.group, manifest.name),
        ),
        ("Version", format!("`{}`", manifest.version)),
        ("Kind", format!("`{}`", manifest.kind())),
    ];
    for (label, key) in [
        ("Licence", "license"),
        ("Homepage", "homepage"),
        ("Summary", "description"),
    ] {
        if let Some(value) = manifest.field(key) {
            rows.push((label, value));
        }
    }
    for (label, key) in [("Authors", "authors"), ("Keywords", "keywords")] {
        if let Some(list) = list_field(manifest, key) {
            rows.push((label, list));
        }
    }
    out.push_str(&table(&rows));

    for (table_name, title, id) in [
        ("skill", "Skills", "skills"),
        ("binary", "Binaries", "binaries"),
        ("mcp_server", "MCP servers", "mcp-servers"),
        ("media", "Images", "images"),
    ] {
        if let Some(value) = manifest.table(table_name) {
            out.push_str(&section(
                id,
                title,
                &fence(Some("toml"), &emit(table_name, value)),
            ));
        }
    }

    out.push_str(&shelf(
        "documentation",
        "Documentation",
        "Packages that document this one. A ★ means both ends agree — the \
         package named this subject and this subject named it back; without \
         one, only the documentation's own side of the edge exists, which is \
         a state and not a lesser one.",
        &related.documentation,
    ));
    out.push_str(&shelf(
        "translations",
        "Other languages",
        "Adaptations of this documentation. A ★ here means «named by the \
         author of this documentation», never «approved by the subject».",
        &related.translations,
    ));
    if let Some(adaptation) = &related.adaptation {
        out.push_str(&section(
            "adapts",
            "What this adapts",
            &paragraph(&mirror(adaptation)),
        ));
    }
    out.push_str(&shelf(
        "dependants",
        "Depended on by",
        "Packages that require this one, as the catalog holds them now.",
        &related.dependants,
    ));

    out.push_str(&section(
        "the-manifest",
        "The manifest",
        &fence(Some("toml"), &manifest.text),
    ));
    out.push_str(CLOSE);
    out
}

/// What an adaptation's page says about its source, in words.
///
/// Structure, and never a distance. «Behind by three revisions» would
/// need a history the project does not keep (`##LOC-NO-REVISION`); «the
/// same pages, anchors and blocks» is a question a machine can answer,
/// and whether the prose around them still reads is a question only a
/// person can (`##OBS-MAINTENANCE-TOOLS`).
fn mirror(adaptation: &super::Adaptation) -> String {
    let stands = match adaptation.divergences {
        0 => "Every page mirrors it: the same paths, the same anchors, the same \
              blocks."
            .to_string(),
        1 => "One page does not mirror it.".to_string(),
        n => format!("{n} pages do not mirror it."),
    };
    format!(
        "This is an adaptation of `{}` into another language, carrying {} page(s). \
         {stands} That is a check of STRUCTURE and not of meaning: nothing here \
         records how far the text has drifted, because nothing in a translation \
         remembers which revision of the source it was made from.",
        adaptation.source, adaptation.pages,
    )
}

/// One shelf, or nothing at all.
///
/// A shelf with no rows is left out rather than printed empty: «nobody
/// has documented this» is what an absent shelf says on a package page,
/// and a heading over an empty table says it less clearly while taking
/// more room.
fn shelf(id: &str, title: &str, why: &str, rows: &[crate::site::shelves::Row]) -> String {
    if rows.is_empty() {
        return String::new();
    }
    let mut table = String::from(
        "  <table>\n    <tr><td>Standing</td><td>Title</td><td>Coordinate</td>\
         <td>Publisher and language</td></tr>\n",
    );
    for row in rows {
        table.push_str(&format!(
            "    <tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>\n",
            escape(format!("{} {}", row.rank.star(), row.rank.word()).trim()),
            escape(&row.title),
            escape(format!("`{}@{}`", row.coordinate, row.version)),
            escape(format!("{} · {}", row.publisher, row.lang)),
        ));
    }
    table.push_str("  </table>\n");
    section(id, title, &format!("{}{table}", paragraph(why)))
}

/// The README as a page.
fn readme_page(manifest: &Manifest, text: &str) -> String {
    let mut out = open(&format!("{} — README", manifest.title()), "user,dev");
    out.push_str(&paragraph(&format!(
        "The README of `{}/{}@{}`, as the package ships it.",
        manifest.group, manifest.name, manifest.version
    )));
    let mut taken: Vec<String> = Vec::new();
    let mut heading: Option<(String, String)> = None;
    let mut body = String::new();
    for block in readme::blocks(text) {
        match block {
            Block::Heading { text } => {
                flush(&mut out, &mut heading, &mut body);
                heading = Some((unique(&slug(&text), &mut taken), text));
            }
            Block::Paragraph { text } => body.push_str(&paragraph(&text)),
            Block::Fence { lang, body: code } => body.push_str(&fence(lang.as_deref(), &code)),
        }
    }
    flush(&mut out, &mut heading, &mut body);
    out.push_str(CLOSE);
    out
}

/// Close the section a heading opened, or emit the blocks that stood
/// before the first heading at the top level of the page.
fn flush(out: &mut String, heading: &mut Option<(String, String)>, body: &mut String) {
    match heading.take() {
        Some((id, title)) => out.push_str(&section(&id, &title, body)),
        None => out.push_str(body),
    }
    body.clear();
}

/// The boot snippet, marked as what it is.
fn boot_snippet_page(manifest: &Manifest, snippet: &toml::Value) -> String {
    let mut out = open("Boot snippet", "user,agent");
    out.push_str(&paragraph(&format!(
        "`{}/{}@{}` contributes a boot snippet: text that a session reads before it \
         starts work, placed into the project's generated boot lane by `vibe \
         install`. It is not read on this page — it is read by the agent.",
        manifest.group, manifest.name, manifest.version
    )));
    out.push_str(&fence(Some("toml"), &emit("boot_snippet", snippet)));
    out.push_str(CLOSE);
    out
}

/// The opening of a page: the declaration, the root title and the
/// status that says who it is for.
fn open(title: &str, audience: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <spec xmlns=\"https://vibevm.org/spec/1\">\n  \
         <title id=\"root\">{}</title>\n  \
         <status stage=\"doc\" state=\"work\" audience=\"{audience}\"/>\n",
        escape(title)
    )
}

/// The close of a page.
const CLOSE: &str = "</spec>\n";

fn paragraph(text: &str) -> String {
    format!("  <p>{}</p>\n", escape(text))
}

fn fence(lang: Option<&str>, body: &str) -> String {
    match lang {
        Some(lang) => format!(
            "  <fence lang=\"{}\">{}</fence>\n",
            escape(lang),
            escape(body)
        ),
        None => format!("  <fence>{}</fence>\n", escape(body)),
    }
}

fn section(id: &str, title: &str, body: &str) -> String {
    format!("  <{id} title=\"{}\">\n{body}  </{id}>\n", escape(title))
}

/// A two-column table, with its header written out.
///
/// The header is not decoration. The Markdown projection reads the FIRST
/// row of a table as its header, so a table that opened with data lost
/// its first fact to the header line — measured on a live render, where
/// the package's own coordinate was the row that disappeared.
fn table(rows: &[(&str, String)]) -> String {
    let mut out = String::from("  <table>\n    <tr><td>Field</td><td>Value</td></tr>\n");
    for (label, value) in rows {
        out.push_str(&format!(
            "    <tr><td>{}</td><td>{}</td></tr>\n",
            escape(label),
            escape(value)
        ));
    }
    out.push_str("  </table>\n");
    out
}

/// A list-valued `[package]` field, comma-joined.
fn list_field(manifest: &Manifest, key: &str) -> Option<String> {
    let values = manifest
        .value
        .get("package")?
        .get(key)?
        .as_array()?
        .iter()
        .filter_map(|v| v.as_str())
        .collect::<Vec<_>>();
    if values.is_empty() {
        return None;
    }
    Some(values.join(", "))
}

/// One table of the manifest, re-emitted as TOML under its own name.
fn emit(name: &str, value: &toml::Value) -> String {
    let mut document = toml::map::Map::new();
    document.insert(name.to_string(), value.clone());
    toml::to_string_pretty(&toml::Value::Table(document)).unwrap_or_default()
}

/// An element name from a heading: the anchor a reader links to.
///
/// It has to be a legal XML name and it has to be STABLE, because an
/// anchor is immutable once published (campaign rule R-06): the same
/// heading must produce the same anchor on every render, which is why it
/// is a slug of the text and never a position.
fn slug(text: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            dash = false;
        } else if !out.is_empty() && !dash {
            out.push('-');
            dash = true;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    // A name must start with a letter, and a heading may start with a
    // number («2. Install»); the prefix is what keeps such a heading from
    // becoming an illegal element name.
    if trimmed.is_empty() || !trimmed.starts_with(|c: char| c.is_ascii_alphabetic()) {
        return format!("s-{trimmed}");
    }
    trimmed
}

/// The same heading twice is legal in a README and illegal as an anchor.
fn unique(id: &str, taken: &mut Vec<String>) -> String {
    let mut candidate = id.to_string();
    let mut n = 1;
    while taken.contains(&candidate) {
        n += 1;
        candidate = format!("{id}-{n}");
    }
    taken.push(candidate.clone());
    candidate
}

/// XML text, escaped. `&` first, or the escapes would escape each other.
fn escape(text: impl AsRef<str>) -> String {
    text.as_ref()
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests;
