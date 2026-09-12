//! The `doc` package scaffold, and the translation scaffold that mirrors
//! a source documentation
//! ([PROP-057](../../../../../vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml)).
//!
//! A documentation package is not a tool package with different text in
//! it. It has no boot lane and no snippet — documentation is read, never
//! run (`##KIND-DOC-MUST-NOT-EXECUTE`) — and it has two things no other
//! kind is required to carry: a `title` and an `abstract`, the card the
//! site's shelves and the language selector render without downloading
//! anything (`##CARD-TITLE`, `##CARD-DESCRIPTION-AND-ABSTRACT`). So the
//! scaffold is its own, not a variation on the generic one.
//!
//! The `abstract` is written as a comment holding four questions rather
//! than as prose the author will delete unread. The four are the whole
//! definition of the field, and an author who answers them has written
//! the abstract; an author handed a lorem-ipsum paragraph has written
//! nothing and does not know it.
//!
//! **`--translates` mirrors, it does not invent.** A translation must
//! mirror its source file for file — the same paths, the same anchors,
//! the same fact identifiers, the same number and kinds of blocks
//! (`##LOC-MIRROR`) — so the scaffold COPIES the source's pages rather
//! than generating empty ones. A copy is a perfect mirror on the day it
//! is made; the translator then replaces prose in place and the mirror
//! survives by construction. The source is read from where a source can
//! honestly be read offline: this project's own package root, then the
//! machine store.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#kinds");

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use vibe_core::manifest::Manifest;

use super::helpers::*;
use super::prompts::ProjectFields;
use crate::output;

/// The comment that stands where the `abstract`'s four answers go.
const ABSTRACT_TEMPLATE: &str = "\
# The abstract answers four questions and nothing else. A reader decides
# from it whether to open anything at all, so it is read far more often
# than any page. Keep it to three to six sentences, in this package's own
# language, under 1000 characters:
#   1. What does this documentation cover?
#   2. Who is it for?
#   3. What does it assume the reader already knows?
#   4. What does it deliberately leave out?
";

/// Write the `doc` package tree under `pkg_dir`. Returns the outcomes
/// the caller reports, in creation order.
pub(super) fn create_doc_package(
    ctx: &output::Context,
    project_root: &Path,
    pkg_dir: &Path,
    group: &str,
    name: &str,
    fields: &ProjectFields,
    translation: Option<&Translation>,
) -> Result<Vec<Outcome>> {
    ensure_dir(pkg_dir)?;
    let mut outcomes = Vec::new();

    let manifest_text = manifest_text(group, name, fields, translation);
    outcomes.extend(write_once(
        ctx,
        project_root,
        &pkg_dir.join(Manifest::FILENAME),
        &manifest_text,
        "package manifest",
    )?);

    let title = scaffold_title(name, translation);
    outcomes.extend(write_once(
        ctx,
        project_root,
        &pkg_dir.join("README.md"),
        &readme_text(&title, group, name),
        "package README",
    )?);

    // The pages. A translation mirrors its source's tree; a fresh
    // documentation gets one page, so the author has a shape to copy
    // rather than a blank directory.
    let specs_dir = pkg_dir.join(vibe_core::layout::current_specs_root());
    ensure_dir(&specs_dir)?;
    match translation {
        Some(source) => outcomes.extend(mirror_pages(ctx, project_root, source, &specs_dir)?),
        None => outcomes.extend(write_once(
            ctx,
            project_root,
            &specs_dir.join("README.xml"),
            &example_page(&title),
            "example page",
        )?),
    }

    outcomes.extend(write_once(
        ctx,
        project_root,
        &pkg_dir.join("specmap.toml"),
        &specmap_text(group, name),
        "specmap policy",
    )?);

    // The card's images live here, and the directory is created empty on
    // purpose: a package with no image is complete — the site and the
    // local reader generate a placeholder from the coordinate's hash
    // (`##CARD-PLACEHOLDERS-GENERATED`).
    let media_dir = pkg_dir.join("media");
    ensure_dir(&media_dir)?;
    outcomes.extend(write_once(
        ctx,
        project_root,
        &media_dir.join(".gitkeep"),
        "",
        "media directory",
    )?);

    Ok(outcomes)
}

/// Everything the translation scaffold learned about its source.
#[derive(Debug)]
pub(super) struct Translation {
    /// The source's `<group>/<name>` coordinate.
    pub(super) coordinate: String,
    /// The constraint written into `[translates].version`.
    pub(super) constraint: String,
    /// The BCP-47 tag this adaptation is in, from the package name.
    pub(super) language: String,
    /// The source's `[[documents]]`, copied verbatim — a translation's
    /// subjects MUST equal its source's (`##LOC-DOCUMENTS-MATCH`).
    pub(super) documents: Vec<(String, String)>,
    /// Where the source's tree was found.
    pub(super) root: PathBuf,
}

/// Resolve the source documentation `coordinate` and the language of the
/// adaptation named `name`, or explain which of the two could not be
/// settled.
pub(super) fn resolve_translation(
    project_root: &Path,
    coordinate: &str,
    name: &str,
) -> Result<Translation> {
    let (group, source_name) = coordinate.split_once('/').with_context(|| {
        format!(
            "`--translates {coordinate}` is not a `<group>/<name>` coordinate — a translation \
             names the documentation it adapts, without a version \
             (spec://org.vibevm.core/vibevm/common/PROP-057#LOC-PACKAGE-PER-LANGUAGE)"
        )
    })?;

    // The language is the name's own suffix: an official translation is
    // published as `<docname>-<lang>` by the source's group
    // (`##LOC-OFFICIAL-TRANSLATION`), so the convention already carries
    // the one fact a scaffold would otherwise have to ask for.
    let language = name
        .strip_prefix(&format!("{source_name}-"))
        .filter(|tag| !tag.is_empty())
        .with_context(|| {
            format!(
                "cannot tell which language `{name}` adapts `{coordinate}` into — a translation \
                 is named `<docname>-<lang>`, so this package should be called \
                 `{source_name}-<lang>` (for example `{source_name}-ru`) \
                 (spec://org.vibevm.core/vibevm/common/PROP-057#LOC-OFFICIAL-TRANSLATION)"
            )
        })?
        .to_string();

    let (root, manifest) = read_source(project_root, group, source_name).with_context(|| {
        format!(
            "`{coordinate}` is not readable offline — it is in neither this project's `{packages}` \
             nor the machine store. Warm it first: `vibe cache add {coordinate}`",
            packages = display_pathbuf(&vibe_core::layout::current_packages_root()),
        )
    })?;

    let version = manifest
        .package
        .as_ref()
        .map(|p| p.version.clone())
        .context("the source documentation's manifest carries no [package] table")?;

    Ok(Translation {
        coordinate: coordinate.to_string(),
        constraint: format!("^{}.{}", version.major, version.minor),
        language,
        documents: manifest
            .documents
            .iter()
            .map(|d| (d.package.clone(), d.version.clone()))
            .collect(),
        root,
    })
}

/// The newest readable version directory of `<group>/<name>`, searched
/// in the project's own package root first and the machine store second.
fn read_source(project_root: &Path, group: &str, name: &str) -> Option<(PathBuf, Manifest)> {
    let mut roots = vec![project_root.join(vibe_core::layout::current_packages_root())];
    if let Ok(store) = vibe_registry::store::store_root() {
        roots.push(store);
    }
    for root in roots {
        let package_dir = root.join(group).join(name);
        let Ok(entries) = fs::read_dir(&package_dir) else {
            continue;
        };
        let mut best: Option<(semver::Version, PathBuf)> = None;
        for entry in entries.filter_map(|e| e.ok()) {
            let file_name = entry.file_name();
            let Some(text) = file_name.to_str().and_then(|n| n.strip_prefix('v')) else {
                continue;
            };
            let Ok(version) = semver::Version::parse(text) else {
                continue;
            };
            if best.as_ref().is_none_or(|(best, _)| version > *best) {
                best = Some((version, entry.path()));
            }
        }
        if let Some((_, dir)) = best
            && let Ok(manifest) = Manifest::read(dir.join(Manifest::FILENAME))
        {
            return Some((dir, manifest));
        }
    }
    None
}

/// Copy the source's pages into `specs_dir`, path for path. Existing
/// files are kept, so re-running the scaffold never overwrites a
/// translator's work.
fn mirror_pages(
    ctx: &output::Context,
    project_root: &Path,
    source: &Translation,
    specs_dir: &Path,
) -> Result<Vec<Outcome>> {
    let source_specs = source.root.join(vibe_core::layout::current_specs_root());
    let mut outcomes = Vec::new();
    let mut stack = vec![source_specs.clone()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        // Sorted, so a scaffold's report reads the same on every machine.
        let mut children: Vec<PathBuf> = entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();
        children.sort();
        for child in children {
            if child.is_dir() {
                stack.push(child);
                continue;
            }
            let Ok(relative) = child.strip_prefix(&source_specs) else {
                continue;
            };
            let target = specs_dir.join(relative);
            if let Some(parent) = target.parent() {
                ensure_dir(parent)?;
            }
            let bytes = fs::read(&child)?;
            let text = String::from_utf8_lossy(&bytes).into_owned();
            outcomes.extend(write_once(
                ctx,
                project_root,
                &target,
                &text,
                "mirrored page",
            )?);
        }
    }
    if outcomes.is_empty() {
        bail!(
            "`{}` carries no pages under `{}` — there is nothing to mirror",
            source.coordinate,
            display_pathbuf(&vibe_core::layout::current_specs_root()),
        );
    }
    Ok(outcomes)
}

/// The `title` the scaffold starts the author off with. An adaptation
/// takes its SOURCE's name plus its language — `Acme Docs (ru)`, not
/// `Acme Docs Ru (ru)`, because the language suffix in the package name
/// is the convention, not part of what a reader is shown.
fn scaffold_title(name: &str, translation: Option<&Translation>) -> String {
    match translation {
        None => display_title(name),
        Some(source) => {
            let source_name = source
                .coordinate
                .split_once('/')
                .map_or(name, |(_, source_name)| source_name);
            format!("{} ({})", display_title(source_name), source.language)
        }
    }
}

/// `vibevm-docs` → `Vibevm Docs`: a starting point for `title`, which
/// the author is expected to replace with the name a reader sees.
fn display_title(name: &str) -> String {
    name.split(['-', '_'])
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn manifest_text(
    group: &str,
    name: &str,
    fields: &ProjectFields,
    translation: Option<&Translation>,
) -> String {
    let authors_line = if fields.authors.is_empty() {
        String::new()
    } else {
        let quoted: Vec<String> = fields.authors.iter().map(|a| format!("\"{a}\"")).collect();
        format!("authors = [{}]\n", quoted.join(", "))
    };
    // The language of a `doc` package IS `[i18n].canonical` — there is no
    // `lang` field, and writing one is refused by name
    // (`##LOC-LANGUAGE-FIELD`).
    let language = translation.map_or("en", |t| t.language.as_str());
    let title = scaffold_title(name, translation);

    let mut text = format!(
        "# A documentation package (spec://org.vibevm.core/vibevm/common/PROP-057#kinds).\n\
         # It is read, never installed: `vibe cache add {group}/{name}` warms it and its\n\
         # subjects into the machine store. It declares no boot snippet, no binary and\n\
         # no MCP server.\n\
         [package]\ngroup = \"{group}\"\nname = \"{name}\"\nkind = \"doc\"\n\
         version = \"{version}\"\nepoch = 1\n{authors_line}\
         license = \"{license}\"\ndescription = \"{description}\"\nformat = \"{format}\"\n\n\
         # The display name: what the shelves, the language selector and the page\n\
         # heading show. Identity stays the coordinate, so two manuals may share a title.\n\
         title = \"{title}\"\n\n\
         {ABSTRACT_TEMPLATE}\
         abstract = \"\"\"\n\
         What it covers: …\n\
         For whom: …\n\
         What it assumes known: …\n\
         What it leaves out: …\n\
         \"\"\"\n\n\
         # The language this documentation is written in. There is no `lang` field —\n\
         # this is it (spec://org.vibevm.core/vibevm/common/PROP-057#LOC-LANGUAGE-FIELD).\n\
         [i18n]\ncanonical = \"{language}\"\n",
        version = fields.version,
        license = fields.license,
        description = fields.description,
        format = fields.format,
    );

    text.push_str(
        "\n# What this documentation documents. At least one subject, each a\n\
         # `<group>/<name>` coordinate with a constraint on the subject's version\n\
         # (spec://org.vibevm.core/vibevm/common/PROP-057#REL-DOCUMENTS-REQUIRED).\n",
    );
    match translation {
        // A translation's subjects MUST equal its source's, so they are
        // copied rather than stubbed — the gate compares the two sets.
        Some(source) if !source.documents.is_empty() => {
            for (package, version) in &source.documents {
                text.push_str(&format!(
                    "[[documents]]\npackage = \"{package}\"\nversion = \"{version}\"\n"
                ));
            }
        }
        _ => {
            text.push_str("[[documents]]\npackage = \"org.example/subject\"\nversion = \"^1.0\"\n")
        }
    }

    if let Some(source) = translation {
        text.push_str(&format!(
            "\n# The documentation this package adapts. Its pages mirror the source file\n\
             # for file; examples are not re-authored but referenced by id\n\
             # (spec://org.vibevm.core/vibevm/common/PROP-057#LOC-EXAMPLE-REF).\n\
             [translates]\npackage = \"{coordinate}\"\nversion = \"{constraint}\"\n",
            coordinate = source.coordinate,
            constraint = source.constraint,
        ));
    }

    text.push_str(
        "\n# The card's images, optional for every kind. Formats: PNG, JPEG, WebP —\n\
         # SVG is refused. Absent roles get a generated placeholder.\n\
         # [media]\n\
         # icon    = \"media/icon.png\"     # square, 256…1024 px, up to 256 KiB\n\
         # banner  = \"media/banner.jpg\"   # 3:1, 1500×500 recommended, up to 1 MiB\n\
         # preview = \"media/preview.png\"  # 1.91:1, 1200×630 recommended, up to 1 MiB\n",
    );
    text
}

fn readme_text(title: &str, group: &str, name: &str) -> String {
    format!(
        "# {title}\n\n\
         Documentation, as a package of kind `doc`. Its pages live under `{specs}/`.\n\n\
         Read it locally:\n\n\
         ```\nvibe cache add {group}/{name}\n```\n\n\
         A `doc` package is never installed into a project — warming it into the\n\
         machine store is how it is read, and the warm-up brings the packages it\n\
         documents with it so every `spec://` citation resolves offline.\n",
        specs = display_pathbuf(&vibe_core::layout::current_specs_root()),
    )
}

fn example_page(title: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <spec xmlns=\"https://vibevm.org/spec/1\">\n  \
         <title id=\"root\">{title}</title>\n  \
         <status stage=\"doc\" state=\"work\" audience=\"user\"/>\n  \
         <p>One page, to show the shape. Replace this with the entry page of your\n  \
         documentation: what it covers, and where a reader goes next.</p>\n  \
         <first-section title=\"The first section\">\n    \
         <p>Every section carries a stable identifier. Once published, an anchor\n    \
         never changes — a translation mirrors these identifiers one for one.</p>\n  \
         </first-section>\n\
         </spec>\n"
    )
}

fn specmap_text(group: &str, name: &str) -> String {
    format!(
        "# The traceability policy for this documentation package.\n\
         # Its pages are spec documents like any other package's, so the same\n\
         # engine walks them (PROP-014).\n\
         namespace = \"{group}/{name}\"\n\n\
         # A documentation package ships no code to tag.\n\
         scan_roots = []\n\n\
         # The pages.\n\
         spec_roots = [\"{specs}\"]\n",
        specs = display_pathbuf(&vibe_core::layout::current_specs_root()),
    )
}

/// Write `text` at `path` unless something is already there, reporting
/// either way. The scaffold never overwrites: re-running it on a package
/// under edit must be safe.
fn write_once(
    ctx: &output::Context,
    project_root: &Path,
    path: &Path,
    text: &str,
    reason: &'static str,
) -> Result<Vec<Outcome>> {
    let relative = display_pathbuf(path.strip_prefix(project_root).unwrap_or(path));
    if path.exists() {
        ctx.skipped(&relative, "already exists");
        return Ok(vec![Outcome {
            path: relative,
            action: Action::Kept,
            reason,
        }]);
    }
    fs::write(path, text)?;
    ctx.created(&relative);
    Ok(vec![Outcome {
        path: relative,
        action: Action::Created,
        reason,
    }])
}

#[cfg(test)]
#[path = "doc/tests.rs"]
mod tests;
