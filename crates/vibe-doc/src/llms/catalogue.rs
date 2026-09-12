//! The catalogue — many documentations, one listing, in the shape of an
//! arXiv search result (PROP-057 `##SEO-LLMS-FILES`, `##CARD-DECISION`,
//! `##MANDATE-CARD`).
//!
//! The owner asked for this shape by name: «обычно для библиотеки
//! документации лучше иметь человекочитаемое название и человекочитаемый
//! абстракт — так же как это делают поисковики по arxiv.org». So a row is
//! a title a person reads, the publisher beside it, the language, the
//! audiences, and the abstract that says what the thing covers and for
//! whom — enough to choose without opening anything.
//!
//! ## Three signals that agree
//!
//! `##DISC-THREE-SIGNALS` requires the star, the caption and the order to
//! say the same thing, and never to contradict each other. Here they do:
//! a row carries ★ when a subject confirmed it, the caption spells
//! `primary`, `official` or `community` in words, and the rows are sorted
//! primary, official, community. Nothing is hidden — the community shelf
//! shows everything that was found, and the protection against
//! impersonation is the publisher, visible on every row
//! (`##DISC-NO-MODERATION`, `##DISC-PUBLISHER-VISIBLE`).
//!
//! A translation's star means «named by the author of this
//! documentation», not «approved by the subject», and the row says so in
//! words rather than leaving the mark to be guessed at
//! (`##DISC-FOUR-COMBINATIONS`).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#SEO-LLMS-FILES");

use vibe_wire::generated::doc_manifest::{DocManifest, TranslationStatus};

use super::{audience_list, one_line, package_link, star, status_word};
use crate::manifest::status::rank;

/// Render a catalogue of documentations.
///
/// The listing is what a registry's own `llms.txt` carries and what a
/// package page's shelves show: every documentation an index fold found
/// for a subject, in the order a reader should consider them.
///
/// ```
/// use vibe_doc::llms::catalogue;
///
/// // An empty registry still renders a document, and says it is empty
/// // rather than printing a heading over nothing.
/// let text = catalogue(&[], "/doc/");
/// assert!(text.contains("No documentation"));
/// ```
pub fn catalogue(manifests: &[DocManifest], base: &str) -> String {
    let mut out = String::from("# Documentation\n\n");
    if manifests.is_empty() {
        out.push_str(
            "No documentation was found. A package of kind `doc` appears here as soon as \
             it declares a subject in `[[documents]]`.\n",
        );
        return out;
    }
    out.push_str(
        "> Every documentation package found, primary first. A ★ means the subject (or, \
         for a translation, the documentation it adapts) named it; everything else is the \
         work of the community, shown with its publisher.\n",
    );
    let mut ordered: Vec<&DocManifest> = manifests.iter().collect();
    ordered.sort_by(|a, b| {
        rank(&b.package.status)
            .cmp(&rank(&a.package.status))
            .then_with(|| a.package.title.cmp(&b.package.title))
            .then_with(|| a.package.name.cmp(&b.package.name))
    });
    for manifest in ordered {
        out.push_str(&row(manifest, base));
    }
    out
}

/// One catalogue row.
fn row(manifest: &DocManifest, base: &str) -> String {
    let package = &manifest.package;
    let mut out = format!("\n## {}{}\n\n", star(&package.status), package.title);
    let audiences = audience_list(&package.audiences);
    let audiences = if audiences.is_empty() {
        "no audience marked".to_owned()
    } else {
        audiences
    };
    out.push_str(&format!(
        "{}/{} {} · {} · {} · {} · {}\n",
        package.group,
        package.name,
        package.version,
        status_word(&package.status),
        package.publisher,
        package.lang,
        audiences
    ));
    if let Some(translation) = &package.translation {
        out.push_str(&format!(
            "An adaptation of {}, named {} by the author of that documentation — not by \
             the subject.\n",
            translation.package,
            match translation.status {
                TranslationStatus::Official => "official",
                TranslationStatus::Community => "a community translation",
            }
        ));
    }
    out.push_str(&format!("\n{}\n\n", one_line(&package.abstract_)));
    out.push_str(&format!("→ {}\n", package_link(manifest, base)));
    out
}

#[cfg(test)]
mod tests;
