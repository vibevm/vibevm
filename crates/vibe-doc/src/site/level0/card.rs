//! The card of a package that never wrote one
//! (PROP-057 `##CARD-FIELDS`, `##LEVEL-ZERO`).
//!
//! `title` and `abstract` are required of a `doc` package and optional
//! for every other kind, and level 0 renders every kind. So the card is
//! composed rather than demanded: the title falls back to the name, and
//! the abstract to the one-line `description` and then to a sentence
//! that says what the package is and admits that nobody wrote one. An
//! empty card would put an empty heading on a page; a composed card puts
//! the truth on it.
//!
//! Everything else is carried across unchanged — the relation tables,
//! the language, the images — because those are the package's own
//! statements and a level-0 render is a projection of them, never an
//! opinion about them.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#CARD-FIELDS");

use std::path::Path;

use crate::error::{DocError, Result};

/// A package's manifest, read as the data a level-0 render needs.
///
/// Read as TOML data and not through the strict manifest parser: the
/// site renders packages it did not write, and a strict parse would
/// refuse a whole version over a field this render never looks at. The
/// price is that a malformed VALUE reads as absent, which for a card is
/// the same outcome as not declaring it.
#[derive(Debug, Clone)]
pub struct Manifest {
    pub value: toml::Value,
    pub text: String,
    pub group: String,
    pub name: String,
    pub version: String,
}

impl Manifest {
    /// One string from `[package]`.
    pub fn field(&self, key: &str) -> Option<String> {
        self.value
            .get("package")
            .and_then(|p| p.get(key))
            .and_then(toml::Value::as_str)
            .map(str::to_owned)
    }

    /// A top-level table, when the package declares one.
    pub fn table(&self, key: &str) -> Option<&toml::Value> {
        self.value.get(key)
    }

    /// One boolean from `[package]`, false unless the package said so.
    pub fn flag(&self, key: &str) -> bool {
        self.value
            .get("package")
            .and_then(|p| p.get(key))
            .and_then(toml::Value::as_bool)
            .unwrap_or(false)
    }

    /// One list of strings from `[package]`, empty unless the package
    /// wrote one.
    pub fn strings(&self, key: &str) -> Vec<String> {
        self.value
            .get("package")
            .and_then(|p| p.get(key))
            .and_then(toml::Value::as_array)
            .map(|entries| {
                entries
                    .iter()
                    .filter_map(toml::Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The kind the package declares, or `pack` — the neutral word for a
    /// package whose manifest does not say.
    pub fn kind(&self) -> String {
        self.field("kind").unwrap_or_else(|| "pack".to_string())
    }

    /// The display name of `##CARD-TITLE`: the declared one, or the
    /// package's name, which is what a shelf would show anyway.
    pub fn title(&self) -> String {
        self.field("title").unwrap_or_else(|| self.name.clone())
    }

    /// The four answers of `##CARD-DESCRIPTION-AND-ABSTRACT`, or what
    /// can honestly be said without them.
    pub fn abstract_(&self) -> String {
        if let Some(declared) = self.field("abstract") {
            return declared.trim().to_string();
        }
        if let Some(description) = self.field("description") {
            return description.trim().to_string();
        }
        format!(
            "`{}/{}` is a package of kind `{}`. Its author wrote no abstract, so this \
             page is what the package itself says: its manifest, its README and the \
             specifications it carries.",
            self.group,
            self.name,
            self.kind(),
        )
    }
}

/// Read a package's manifest.
pub fn read(source: &Path) -> Result<Manifest> {
    let path = source.join("vibe.toml");
    let text = std::fs::read_to_string(&path).map_err(|e| DocError::io("reading", &path, e))?;
    let value: toml::Value = toml::from_str(&text)
        .map_err(|e| DocError::manifest(&path, format!("does not parse: {e}")))?;
    // A `[project]` answers the same three questions a `[package]` does,
    // and the host's root is exactly that: the site addresses it by
    // `<group>/<name>/<version>` like everything else.
    let read = |table: &str, key: &str| {
        value
            .get(table)
            .and_then(|t| t.get(key))
            .and_then(toml::Value::as_str)
            .map(str::to_owned)
    };
    let coordinate = |key: &str| read("package", key).or_else(|| read("project", key));
    let (Some(group), Some(name), Some(version)) = (
        coordinate("group"),
        coordinate("name"),
        coordinate("version"),
    ) else {
        return Err(DocError::manifest(
            &path,
            "names no `<group>/<name>/<version>` coordinate, and the site addresses \
             every version by one",
        ));
    };
    Ok(Manifest {
        value,
        text,
        group,
        name,
        version,
    })
}

/// The manifest of the documentation package a level-0 render is.
///
/// It carries the coordinate of the package it renders, because that
/// coordinate IS the address (`##SITE-MOUNT`); the composed card; and
/// the package's own relation tables, unchanged, so that the officiality
/// of its documentation and of its adaptations is computed from the
/// edges the author declared and from nothing this module invents
/// (`##REL-NO-OFFICIAL-FLAG`).
pub fn synthesise(manifest: &Manifest) -> String {
    let mut out = String::new();
    out.push_str(
        "# Written by `vibe doc build-site`: the level-0 view of one published version\n\
         # (PROP-057 `##LEVEL-ZERO`). It is a render input and never a package.\n\n",
    );
    out.push_str("[package]\n");
    out.push_str(&format!("name = {}\n", quote(&manifest.name)));
    out.push_str(&format!("group = {}\n", quote(&manifest.group)));
    out.push_str(&format!("version = {}\n", quote(&manifest.version)));
    out.push_str(&format!("kind = {}\n", quote(&manifest.kind())));
    out.push_str(&format!("title = {}\n", quote(&manifest.title())));
    out.push_str(&format!("abstract = {}\n", quote(&manifest.abstract_())));
    if let Some(description) = manifest.field("description") {
        out.push_str(&format!("description = {}\n", quote(&description)));
    }
    // Who wrote the prose is the package's own statement about its text
    // (`##CARD-AUTHORSHIP`), so it travels with the card like the title:
    // a render that dropped it would show no badge on a documentation
    // that had declared one.
    if let Some(authorship) = manifest.field("authorship") {
        out.push_str(&format!("authorship = {}\n", quote(&authorship)));
    }
    // A bridge's page has two names to show and must never merge them
    // (PROP-023 `##AUTHORSHIP-SEPARATION`). The flag and the wrapper's
    // own authors travel here; the upstream names travel with
    // `[[embedded_source]]` below, which is where the package wrote
    // them.
    if manifest.flag("bridge") {
        out.push_str("bridge = true\n");
    }
    let authors = manifest.strings("authors");
    if !authors.is_empty() {
        out.push_str(&format!("authors = {}\n", quote_all(&authors)));
    }
    for table in CARRIED {
        if let Some(value) = manifest.table(table) {
            out.push('\n');
            out.push_str(&emit(table, value));
        }
    }
    out
}

/// The tables a level-0 render carries across from the package it
/// renders. Each is a statement the author made about relations,
/// language, pictures, navigation or provenance, and each is read
/// further down the pipeline — the card's images by `media::slots`, the
/// relations by the manifest's officiality, the language by every
/// address that carries one, the navigation and the upstream authors of
/// `[[embedded_source]]` by the page manifest the site reads.
const CARRIED: &[&str] = &[
    "i18n",
    "documents",
    "documentation",
    "translates",
    "media",
    "navigation",
    "embedded_source",
];

/// Emit one carried table in TOML, under its own name.
fn emit(name: &str, value: &toml::Value) -> String {
    // `toml::to_string` of a document with one key writes the table
    // header for us, including the `[[array]]` form, which is the one
    // place the two spellings must not be confused.
    let mut document = toml::map::Map::new();
    document.insert(name.to_string(), value.clone());
    toml::to_string(&toml::Value::Table(document)).unwrap_or_default()
}

/// A TOML array of string literals, quoted the same way one string is.
fn quote_all(values: &[String]) -> String {
    let quoted: Vec<String> = values.iter().map(|value| quote(value)).collect();
    format!("[{}]", quoted.join(", "))
}

/// A TOML string literal. Basic strings with the four escapes TOML asks
/// for; a composed abstract carries prose and prose carries quotes.
fn quote(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}

/// Copy the card's declared images to the same relative paths, so that
/// `media::slots` finds them where the carried `[media]` table says they
/// are.
///
/// An image that is not there is a note and not a refusal: the role
/// falls back to a generated placeholder, which is exactly what
/// `##CARD-PLACEHOLDERS-GENERATED` does for a role that declared
/// nothing, and one missing picture must not cost a package its whole
/// render.
pub fn copy_media(
    source: &Path,
    work: &Path,
    manifest: &Manifest,
    notes: &mut Vec<String>,
) -> Result<()> {
    let Some(media) = manifest.table("media").and_then(toml::Value::as_table) else {
        return Ok(());
    };
    for (role, declared) in media {
        let Some(declared) = declared.as_str() else {
            continue;
        };
        let relative = declared.replace('\\', "/");
        let from = source.join(&relative);
        if !from.is_file() {
            notes.push(format!(
                "the card declares `{role} = \"{relative}\"` and the package does not \
                 carry it — the role falls back to a generated picture"
            ));
            continue;
        }
        let to = work.join(&relative);
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent).map_err(|e| DocError::io("creating", parent, e))?;
        }
        std::fs::copy(&from, &to).map_err(|e| DocError::io("copying", &from, e))?;
    }
    Ok(())
}

/// Carry the read-aloud record across when the package keeps one, so the
/// manifest's per-page reading dates survive a level-0 render.
pub fn copy_reviews(source: &Path, work: &Path) -> Result<()> {
    let from = source.join(crate::manifest::REVIEWS);
    if !from.is_file() {
        return Ok(());
    }
    let to = work.join(crate::manifest::REVIEWS);
    std::fs::copy(&from, &to)
        .map(|_| ())
        .map_err(|e| DocError::io("copying", &from, e))
}

#[cfg(test)]
mod tests;
