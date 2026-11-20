//! One page, by address, for a machine (PROP-057 `##OBS-AUDIENCE-AGENT`,
//! the vision's §7.3 agent endpoints).
//!
//! The web serves a page at `/doc/<group>/<name>/<version>/<document>/`
//! and its projections beside it as files; an agent reading over MCP has
//! no HTTP at all, and asks for the same page by the `spec://` address it
//! would cite. That is the whole of this module: one address in, one
//! projection out, resolved through the same four sources a citation
//! resolves through, so «which copy did I read» has an answer.
//!
//! ## Why no HTML
//!
//! The island is for a reader with a browser. An agent paying by the
//! token wants either the Markdown — every cited rule's CURRENT text
//! substituted in, so the page is self-contained — or the dialect XML,
//! which keeps the addresses so the agent resolves them itself and
//! spends nothing on rules it will not read.
//!
//! ## Why a language is a fallback and not a switch
//!
//! One documentation package is one language (`##LOC-LANGUAGE-FIELD`).
//! So `lang` cannot select inside a package: it names the adaptation
//! published as `<name>-<lang>` (`##LOC-OFFICIAL-TRANSLATION`), and when
//! that package is not on this machine the source answers and says which
//! language it is. The site does the same thing for the same reason
//! (`##READER-LANGUAGE-SWITCH-KEEPS-PLACE`): a reader who asked for
//! Russian and got a missing page is worse served than one who got
//! English and was told so.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#OBS-AUDIENCE-AGENT");

use std::collections::BTreeMap;

use vibe_spec::{Authority, SpecAddress};

use crate::build::{self, Format};
use crate::citations::{self, Source, SpecSources};
use crate::content::Content;
use crate::error::{DocError, Result};
use crate::pages::{Page, PageSet};

/// One page, fetched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchedPage {
    /// The address that was resolved — the caller's, or the adaptation's
    /// when a language moved it.
    pub address: String,
    /// The language the returned page is actually written in.
    pub lang: String,
    /// Which of the four sources answered.
    pub source: Source,
    /// The projection.
    pub text: String,
    /// Present only for an answered, source-bound Doc read.
    pub selected_values: Option<BTreeMap<String, vibe_core::manifest::ParameterValue>>,
}

/// Read the page `address` names, in `format`.
///
/// `lang`, when given, asks for the adaptation first and falls back to
/// the address as written.
///
/// ```no_run
/// use vibe_doc::agent::read_page;
/// use vibe_doc::build::Format;
/// use vibe_doc::citations::SpecSources;
///
/// let world = SpecSources::for_checkout("/repo", Some("org.vibevm.core"), "vibevm");
/// let page = read_page(
///     "spec://org.vibevm.core/vibevm-docs/model/boot-lane",
///     Format::Md,
///     None,
///     &world,
/// )?;
/// assert!(page.text.contains("[p01]"));
/// # Ok::<(), vibe_doc::DocError>(())
/// ```
#[specmark::spec(
    implements = "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-DOC-POINT",
    r = 1
)]
pub fn read_page(
    address: &str,
    format: Format,
    lang: Option<&str>,
    sources: &SpecSources,
) -> Result<FetchedPage> {
    read_page_with_input(address, format, lang, sources, None)
}

/// Request-local data from the SAME captured page source. None retains the
/// incumbent source/language behavior and never synthesizes selected values.
pub fn read_page_with_input(
    address: &str,
    format: Format,
    lang: Option<&str>,
    sources: &SpecSources,
    input: Option<&vibe_wire::generated::read_doc_request::Input>,
) -> Result<FetchedPage> {
    let asked = parse(address)?;
    if let Some(input) = input {
        if lang.is_some() {
            return Err(DocError::Page {
                path: Default::default(),
                message: "read_doc input with lang is unsupported".into(),
            });
        }
        let captured = sources
            .capture_document_with_choice(&asked)
            .map_err(|message| DocError::Page {
                path: Default::default(),
                message,
            })?
            .ok_or_else(|| DocError::Page {
                path: Default::default(),
                message: "answered Doc unavailable".into(),
            })?;
        let values = request_data(&captured, input)?;
        return fetch(address, &asked, format, sources, captured, Some(values));
    }
    // The adaptation first when a language was asked for. A miss is not
    // a failure: it is the state the site calls a fallback, and the
    // answer says which language came back.
    if let Some(lang) = lang
        && let Some(adapted) = adaptation_of(address, lang)
        && let Ok(parsed) = parse(&adapted)
        && let Some(captured) =
            sources
                .capture_document(&parsed)
                .map_err(|message| DocError::Citation {
                    page: String::new(),
                    line: 0,
                    uri: adapted.clone(),
                    message,
                })?
    {
        return fetch(&adapted, &parsed, format, sources, captured, None);
    }
    let captured = sources
        .capture_document(&asked)
        .map_err(|message| DocError::Citation {
            page: String::new(),
            line: 0,
            uri: address.into(),
            message,
        })?
        .ok_or_else(|| DocError::Citation {
            page: String::new(),
            line: 0,
            uri: address.into(),
            message: format!(
                "no source holds `{address}` — looked in {}",
                sources.description()
            ),
        })?;
    fetch(address, &asked, format, sources, captured, None)
}

/// Parse an address and drop its anchor: this returns a whole page, and
/// a fragment that looked like a selector would be a promise nothing
/// here keeps.
fn parse(address: &str) -> Result<SpecAddress> {
    SpecAddress::parse(address).map_err(|e| DocError::Citation {
        page: String::new(),
        line: 0,
        uri: address.to_owned(),
        message: e.to_string(),
    })
}

/// `spec://<group>/<name>/…` → `spec://<group>/<name>-<lang>/…`.
///
/// Spelled on the ADDRESS rather than on the parsed value because the
/// naming convention is about the coordinate's text, and rebuilding an
/// address from its parts would have to reproduce every optional piece
/// the grammar allows.
fn adaptation_of(address: &str, lang: &str) -> Option<String> {
    let rest = address.strip_prefix("spec://")?;
    let (group, after) = rest.split_once('/')?;
    let (name, tail) = after.split_once('/')?;
    // A name already carrying the tag is the adaptation; asking for it
    // again would name `<name>-ru-ru`.
    if name.ends_with(&format!("-{lang}")) {
        return None;
    }
    // The version pin travels with the name, and the tag goes before it.
    let (name, version) = match name.split_once('@') {
        Some((name, version)) => (name, format!("@{version}")),
        None => (name, String::new()),
    };
    Some(format!("spec://{group}/{name}-{lang}{version}/{tail}"))
}

/// Locate one page and project it.
fn fetch(
    address: &str,
    parsed: &SpecAddress,
    format: Format,
    sources: &SpecSources,
    located: crate::citations::sources::CapturedDocument,
    selected_values: Option<BTreeMap<String, vibe_core::manifest::ParameterValue>>,
) -> Result<FetchedPage> {
    if located.interpretation != vibe_core::manifest::SourceInterpretation::VibeDoc {
        return Err(DocError::Page {
            path: located.path.clone(),
            message: "agent page requires an actual Doc package source".into(),
        });
    }
    let doc = located.parse().map_err(|e| DocError::Page {
        path: located.path.clone(),
        message: e.to_string(),
    })?;
    // The original logical source names this page even when the installed
    // physical output has a different extension.
    let rel = located
        .relative
        .strip_prefix(&format!("{}/", crate::pages::SPEC_ROOT))
        .or_else(|| {
            located
                .relative
                .strip_prefix(&format!("{}/", vibe_core::layout::LEGACY_SPECS_DIR))
        })
        .unwrap_or(&located.relative)
        .to_owned();
    // When the page belongs to a translation, the examples it borrows come
    // from the source page at the SAME address. The package root is found
    // the way `locate` found the file — through the chain, by the
    // authority's coordinate — so the two cannot disagree about which
    // instance answered. One page is read, not the source package.
    let examples = borrowed_by(&rel, parsed, sources)?;
    let page = Page {
        rel: rel.clone(),
        path: located.path.clone(),
        doc,
    };

    // Only this page's citations are resolved. A manual cites hundreds of
    // rules, and resolving the corpus to answer for one page would make
    // the cheap question expensive.
    let one = PageSet {
        pages: vec![page.clone()],
        unreadable: Vec::new(),
    };
    let content = Content {
        rules: citations::resolve_rules(&one, sources),
        derived: BTreeMap::new(),
        examples: BTreeMap::from([(rel, examples)]),
        base: crate::content::SITE_BASE.to_owned(),
        // The language `locate` answered with: the edition this page
        // belongs to is the instance that held it, and a second read of
        // its manifest could only disagree (`##LOC-LANGUAGE-FIELD`).
        lang: located.lang.clone(),
        // No glossary, and the MCP surface is why: it answers for ONE page
        // by address and refuses the island outright, because the island is
        // for a reader with a browser. The definitions of a glossary are a
        // browse of the whole package, and what an agent reads here — the
        // `.md` and the `.xml` — carries the glossary page whole, in the
        // layer law's order, beside every other page
        // (`##READER-GLOSSARY-CARD` is a reader's affordance).
        glossary: None,
    };
    let text = build::render_page(&page, &content, format);
    located.recheck().map_err(|message| DocError::Page {
        path: located.path.clone(),
        message,
    })?;
    Ok(FetchedPage {
        address: address.to_owned(),
        lang: located.lang,
        source: located.source,
        text,
        selected_values,
    })
}

/// The example bodies the page at `rel` borrows, or none at all.
///
/// An undotted authority names no package, so it names no root to read a
/// source from, and an address the chain cannot place lends nothing —
/// both answer with an empty map rather than a refusal, because a
/// reference this run could not fill renders as the marked gap it is
/// ([`crate::translations::borrowed_by`]).
fn borrowed_by(
    rel: &str,
    parsed: &SpecAddress,
    sources: &SpecSources,
) -> Result<BTreeMap<String, crate::content::ExampleBody>> {
    let Authority::Package { group, name, .. } = &parsed.authority else {
        return Ok(BTreeMap::new());
    };
    sources
        .instance_of(group, name)
        .map(|instance| crate::translations::borrowed_by(&instance.root, rel, sources))
        .unwrap_or_else(|| Ok(BTreeMap::new()))
}

#[cfg(test)]
mod tests;

fn request_data(
    captured: &crate::citations::sources::CapturedDocument,
    input: &vibe_wire::generated::read_doc_request::Input,
) -> Result<BTreeMap<String, vibe_core::manifest::ParameterValue>> {
    use vibe_core::manifest::{
        ConfigurationEffect, ConfigurationFeatureVisibility, ConfigurationGroupDecl,
        ConfigurationIntentDecl, ConfigurationSelection, PackageConfiguration,
    };
    let invalid = || {
        DocError::Page{path:Default::default(),message:"read_doc input: unsupported definition or invalid exact choice; expected one public unguarded exactly-one group with data-only effects".into()}
    };
    let manifest = captured
        .choice_manifest()
        .map_err(|message| DocError::Page {
            path: Default::default(),
            message,
        })?;
    let definition = manifest.configuration.as_ref().ok_or_else(invalid)?;
    let [
        ConfigurationGroupDecl::Choice {
            id,
            members,
            guards,
            cardinality,
            defaults,
            ..
        },
    ] = definition.groups.as_slice()
    else {
        return Err(invalid());
    };
    if *cardinality != vibe_core::configuration::Cardinality::ExactlyOne
        || !guards.is_empty()
        || defaults.is_some()
        || !definition.parameters.is_empty()
        || !definition.profiles.is_empty()
        || input.selections.len() != 1
    {
        return Err(invalid());
    }
    let answer = input.selections.get(id).ok_or_else(invalid)?;
    if answer.members.len() != 1
        || !members.contains(&answer.members[0])
        || definition.features.len() != members.len()
        || definition.features.iter().any(|f| {
            f.visibility != ConfigurationFeatureVisibility::Public
                || !members.contains(&f.id)
                || f.parent.is_some()
                || f.required
                || f.default
                || !f.requires.is_empty()
                || !f.guards.is_empty()
                || f.effects
                    .iter()
                    .any(|e| !matches!(e, ConfigurationEffect::ConfigValue { .. }))
        })
    {
        return Err(invalid());
    }
    let original = ConfigurationIntentDecl {
        policy: vibe_core::configuration::ChoicePolicy::RequireSelection,
        profile: None,
        features: vec![],
        default_features: false,
        selections: BTreeMap::from([(
            id.clone(),
            ConfigurationSelection {
                intent: vibe_core::configuration::MergeIntent::Exact,
                members: Some(answer.members.clone()),
            },
        )]),
        parameters: BTreeMap::new(),
    };
    // An operation-local consumer is not authored or persisted project history.
    let record = PackageConfiguration::evaluate(
        manifest,
        &BTreeMap::from([("read_doc request".into(), original)]),
    )
    .map_err(|_| invalid())?;
    record
        .selected_data(vibe_core::PackageKind::Doc)
        .map_err(|_| invalid())
}
