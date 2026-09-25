//! The `llms.txt` family — the corpus as a machine reads it
//! (PROP-057 `##SEO-LLMS-FILES`, the owner's fourth mandate point).
//!
//! Four files, one manifest, one order. `llms.txt` is the index with a
//! one-line summary per page; `llms-full.txt` is the whole corpus as
//! Markdown; `llms-small.txt` and `llms-medium.txt` are the same corpus
//! cut to a token budget. All four are rendered from the page manifest
//! ([`crate::manifest`]) rather than from a second walk of the package,
//! which is what keeps them from disagreeing with the navigation.
//!
//! ## Two orders, and both are the norm's
//!
//! The corpus tiers follow the layer law: stable text before text that
//! moves with the product ([`crate::manifest::layer`]). The INDEX puts
//! one thing ahead of that order — the pages written for an audience of
//! agents, which `##OBS-AUDIENCE-AGENT` says the site serves «first in
//! `llms.txt`». An agent that reads three lines and stops should read the
//! three lines addressed to it.
//!
//! ## The budget is an estimate, and says so
//!
//! Nothing here runs a tokenizer: there is no single tokenizer to run,
//! and a build that depended on one model's vocabulary would produce a
//! corpus sized for that model alone. The tiers count UTF-8 bytes at
//! [`BYTES_PER_TOKEN`] and admit whole pages only — half a page is worse
//! than an absent one, because an agent cannot tell a truncated rule from
//! a complete one. What was left out is stated at the end of the file, so
//! the reader of a budgeted tier knows it is holding a part.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#SEO-LLMS-FILES");

pub mod catalogue;

use std::collections::BTreeMap;

use vibe_wire::generated::doc_manifest::{Audience, DocManifest, DocPage, DocumentationStatus};

use crate::content::Content;
use crate::md;
use crate::numbering::{expand_derived, number_blocks};
use crate::pages::PageSet;

pub use catalogue::catalogue;

/// The token budget of `llms-small.txt` — a corpus meant to fit beside a
/// task in a small context window.
pub const SMALL_TOKEN_BUDGET: usize = 32_000;

/// The token budget of `llms-medium.txt` — a corpus meant to fit a
/// mainstream context window whole.
pub const MEDIUM_TOKEN_BUDGET: usize = 128_000;

/// Bytes per token, the estimate the budgeted tiers count with. Four is
/// the ordinary rule of thumb for English text in a byte-pair vocabulary;
/// it is stated as a constant, not hidden in an expression, because it is
/// the one number in this module that is an approximation rather than a
/// law.
pub const BYTES_PER_TOKEN: usize = 4;

/// Which file is being rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    /// `llms.txt` — the index: the card, then one line per page.
    Index,
    /// `llms-small.txt` — the corpus under [`SMALL_TOKEN_BUDGET`].
    Small,
    /// `llms-medium.txt` — the corpus under [`MEDIUM_TOKEN_BUDGET`].
    Medium,
    /// `llms-full.txt` — the whole corpus, no budget.
    Full,
}

impl Tier {
    /// Every tier, in the order a site writes them.
    pub const ALL: &'static [Tier] = &[Tier::Index, Tier::Small, Tier::Medium, Tier::Full];

    /// The file name this tier is published under.
    pub fn file_name(self) -> &'static str {
        match self {
            Tier::Index => "llms.txt",
            Tier::Small => "llms-small.txt",
            Tier::Medium => "llms-medium.txt",
            Tier::Full => "llms-full.txt",
        }
    }

    /// The tier's token budget, or `None` when it carries the corpus
    /// whole.
    pub fn budget(self) -> Option<usize> {
        match self {
            Tier::Small => Some(SMALL_TOKEN_BUDGET),
            Tier::Medium => Some(MEDIUM_TOKEN_BUDGET),
            Tier::Index | Tier::Full => None,
        }
    }
}

/// Every page of a package as numbered Markdown, keyed by its address —
/// the bodies the corpus tiers carry.
///
/// The pipeline is the norm's: expand `derived`, number the blocks, then
/// render (`##PIPE-NUMBERING`). The numbers are the same ones the island
/// and the `.xml` projection carry, which is the whole point — a human
/// quoting `p12` from the web and an agent quoting `p12` from this file
/// quote one place (`##READER-NUMBERED-BLOCKS`, R-26).
pub fn bodies(set: &PageSet, content: &Content) -> BTreeMap<String, String> {
    set.pages
        .iter()
        .map(|page| {
            let expanded = expand_derived(&page.doc, content);
            let numbering = number_blocks(&expanded);
            (
                page.rel.clone(),
                md::to_markdown_numbered(&expanded, &page.rel, content, &numbering),
            )
        })
        .collect()
}

/// Render one tier.
///
/// `base` is the path the documentation is served under — `/doc/` on the
/// public site, whatever a local reader or a webview mounts it at. The
/// language segment of a translated site (`/doc/ru/`) belongs to the
/// site's route table, not to a package's manifest, so a caller serving a
/// translation passes the base it serves from and the links come out
/// right without this library learning to route.
pub fn render(
    tier: Tier,
    manifest: &DocManifest,
    bodies: &BTreeMap<String, String>,
    base: &str,
) -> String {
    match tier {
        Tier::Index => index(manifest, base),
        Tier::Small | Tier::Medium | Tier::Full => corpus(manifest, bodies, base, tier.budget()),
    }
}

/// Render every tier, ready to be written out beside one another.
pub fn tiers(
    manifest: &DocManifest,
    bodies: &BTreeMap<String, String>,
    base: &str,
) -> Vec<(&'static str, String)> {
    Tier::ALL
        .iter()
        .map(|tier| (tier.file_name(), render(*tier, manifest, bodies, base)))
        .collect()
}

/// `llms.txt` — the card, then one line per page.
pub fn index(manifest: &DocManifest, base: &str) -> String {
    let mut out = head(manifest, base);
    let (for_agents, rest): (Vec<&DocPage>, Vec<&DocPage>) = manifest
        .pages
        .iter()
        .partition(|p| p.audiences.contains(&Audience::Agent));
    if !for_agents.is_empty() {
        out.push_str("\n## For agents\n\n");
        for page in for_agents {
            out.push_str(&line(manifest, page, base));
        }
    }
    if !rest.is_empty() {
        out.push_str("\n## Pages\n\n");
        for page in rest {
            out.push_str(&line(manifest, page, base));
        }
    }
    out
}

/// One page's line in the index: title, link, leading fact, reading time
/// and audiences.
///
/// The reading time is on the LINE, not only in the manifest, because the
/// line is what an agent budgeting a context window reads before it
/// decides which pages to fetch.
fn line(manifest: &DocManifest, page: &DocPage, base: &str) -> String {
    let audiences = audience_list(&page.audiences);
    let audiences = if audiences.is_empty() {
        String::new()
    } else {
        format!(" · {audiences}")
    };
    format!(
        "- [{}]({}): {} ({} min{})\n",
        page.title,
        page_link(manifest, &page.path, base),
        one_line(&page.summary),
        page.reading_time_min,
        audiences
    )
}

/// `llms-full.txt` and its budgeted tiers — the card, then every page
/// whole, in layer order.
fn corpus(
    manifest: &DocManifest,
    bodies: &BTreeMap<String, String>,
    base: &str,
    budget: Option<usize>,
) -> String {
    let mut out = head(manifest, base);
    let ceiling = budget.map(|tokens| tokens.saturating_mul(BYTES_PER_TOKEN));
    let mut left_out = 0usize;
    for page in &manifest.pages {
        let Some(body) = bodies.get(&page.path) else {
            // A page the caller did not render is left out silently here
            // and counted at the end: the corpus is a projection, and
            // which pages are missing is the checks' question.
            left_out += 1;
            continue;
        };
        let section = format!(
            "\n---\n\n<!-- {} -->\nSource: {}\nAddress: {}\n\n{}\n",
            page.path,
            page_link(manifest, &page.path, base),
            page_address(manifest, &page.path),
            body.trim_end()
        );
        match ceiling {
            // Whole pages only: an agent cannot tell a truncated rule
            // from a complete one, and a corpus that lies about that is
            // worse than a shorter corpus.
            Some(max) if out.len() + section.len() > max => left_out += 1,
            _ => out.push_str(&section),
        }
    }
    if left_out > 0 {
        out.push_str(&format!(
            "\n---\n\n{left_out} page(s) are not in this file. The whole corpus is \
             `{base}{}/{}/{}/llms-full.txt`.\n",
            manifest.package.group, manifest.package.name, manifest.package.version
        ));
    }
    out
}

/// The card every tier opens with.
fn head(manifest: &DocManifest, base: &str) -> String {
    let package = &manifest.package;
    let mut out = format!("# {}\n\n", package.title);
    if let Some(description) = &package.description {
        out.push_str(&format!("> {}\n\n", one_line(description)));
    }
    out.push_str(&format!(
        "{}{} documentation of {}, published by {}, in {}.\n",
        star(&package.status),
        match package.status {
            DocumentationStatus::Primary => "The primary",
            DocumentationStatus::Official => "An official",
            DocumentationStatus::Community => "A community",
        },
        subjects(manifest),
        package.publisher,
        package.lang
    ));
    if let Some(translation) = &package.translation {
        out.push_str(&format!(
            "An adaptation of {} ({} translation).\n",
            translation.package,
            match translation.status {
                vibe_wire::generated::doc_manifest::TranslationStatus::Official => "an official",
                vibe_wire::generated::doc_manifest::TranslationStatus::Community => "a community",
            }
        ));
    }
    out.push_str(&format!(
        "Read it at {}. Rendered {}.\n",
        package_link(manifest, base),
        package.rendered_at.format("%Y-%m-%d")
    ));
    if let Some(published) = &package.published_at {
        out.push_str(&format!("Published {}.\n", published.format("%Y-%m-%d")));
    }
    out.push_str(&format!("\n{}\n", package.abstract_.trim()));
    out
}

/// The subjects this documentation documents, as a sentence fragment.
fn subjects(manifest: &DocManifest) -> String {
    let names: Vec<&str> = manifest
        .package
        .subjects
        .iter()
        .map(|s| s.package.as_str())
        .collect();
    if names.is_empty() {
        return "nothing it declares".to_owned();
    }
    names.join(", ")
}

/// The star `##DISC-THREE-SIGNALS` puts on what a subject confirmed, and
/// nothing where it did not.
pub fn star(status: &DocumentationStatus) -> &'static str {
    match status {
        DocumentationStatus::Primary | DocumentationStatus::Official => "★ ",
        DocumentationStatus::Community => "",
    }
}

/// The caption beside the star — the second of the three signals that
/// must agree (`##DISC-THREE-SIGNALS`).
pub fn status_word(status: &DocumentationStatus) -> &'static str {
    match status {
        DocumentationStatus::Primary => "primary",
        DocumentationStatus::Official => "official",
        DocumentationStatus::Community => "community",
    }
}

/// The audiences of a page or a package, spelled for a reader.
pub fn audience_list(audiences: &[Audience]) -> String {
    audiences
        .iter()
        .map(|a| match a {
            Audience::User => "user",
            Audience::Author => "author",
            Audience::Dev => "dev",
            Audience::Agent => "agent",
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// The package's own page on the site.
pub fn package_link(manifest: &DocManifest, base: &str) -> String {
    format!(
        "{base}{}/{}/{}/",
        manifest.package.group, manifest.package.name, manifest.package.version
    )
}

/// One page's address on the site — the deterministic map of
/// `##SITE-MOUNT`, which needs no index and so can live in a pure
/// function. The trailing slash is the norm's (`##SITE-TRAILING-SLASH`).
pub fn page_link(manifest: &DocManifest, page_path: &str, base: &str) -> String {
    format!(
        "{base}{}/{}/{}/{}/",
        manifest.package.group,
        manifest.package.name,
        manifest.package.version,
        document_of(page_path)
    )
}

/// One page's `spec://` address — what the «for agent» button copies and
/// what a citation to this manual is written as.
pub fn page_address(manifest: &DocManifest, page_path: &str) -> String {
    format!(
        "spec://{}/{}@{}/{}",
        manifest.package.group,
        manifest.package.name,
        manifest.package.version,
        document_of(page_path)
    )
}

/// A page's address inside the package as a `spec://` doc-path: the same
/// path without its extension, which is how the addressing grammar
/// spells a document.
fn document_of(page_path: &str) -> &str {
    page_path.strip_suffix(".xml").unwrap_or(page_path)
}

/// Text as one line — a summary that wrapped in the source must not
/// break a Markdown list item in two.
pub(crate) fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests;
