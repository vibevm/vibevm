//! Officiality, computed (PROP-057 `##REL-OFFICIAL-IS-CONVERGENCE`,
//! `##REL-DEFAULT-CONVENTION`, `##LOC-OFFICIAL-TRANSLATION`, D-19).
//!
//! The owner's fear was three contributors publishing three documentation
//! packages for one subject and nobody able to say which the site should
//! show. The answer is that the SUBJECT assigns officiality, and that it
//! is never written down anywhere as a flag: a field `official = true` in
//! a manifest or an index is a design error by name
//! (`##REL-NO-OFFICIAL-FLAG`). So it is computed here, at every build,
//! from where two edges meet.
//!
//! ## The two levels
//!
//! **Documentation of a subject.** The documentation declares the subject
//! in `[[documents]]`; that edge is given, because this library is
//! reading that very manifest. The other edge comes from the subject:
//! `[documentation].primary` names one package, `[documentation].official`
//! names any number. A subject that declares no `[documentation]` at all
//! falls back to the naming convention — `<name>-docs` in the subject's
//! own group counts as primary — and a subject that declares one replaces
//! the convention entirely, including by leaving the conventional name
//! out.
//!
//! **Translation of a documentation.** Nothing is fetched: the rule is a
//! rule about NAMES. A translation is official when the same group that
//! publishes the source publishes it as `<docname>-<lang>`; everything
//! else is a community translation. Only the group's owner can publish
//! into the group, so officiality cannot be forged through a name.
//!
//! ## Why a subject nobody can read still falls to the convention
//!
//! The convention is a rule about NAMES, and that is what makes it usable
//! when the subject is out of reach: a local reader holding a manual
//! warmed into the store, with no checkout of the subject anywhere, must
//! compute the same standing the site computes from its index fold
//! (`##REL-REVERSE-QUERIES-SITE-SIDE`). If an unreachable subject meant
//! `community`, one manual would be starred on the web and unstarred on a
//! reader's own machine — and `##DISC-THREE-SIGNALS` requires the signals
//! never to contradict each other.
//!
//! So the two states that end at the convention are «the subject declares
//! no `[documentation]`» and «the subject cannot be read»: one has not
//! spoken, the other cannot be heard, and the convention is the right
//! answer for both. What remains `community` is what the rule actually
//! means by it — a package the subject DID speak about and did not name,
//! or one whose name and group the convention does not reach.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#REL-OFFICIAL-IS-CONVERGENCE");

use vibe_wire::generated::doc_manifest::{
    DocumentationStatus, DocumentedSubject, TranslationStatus,
};

use crate::citations::SpecSources;
use crate::derived::manifest::MANIFEST;

/// The suffix the official-by-default documentation of a subject carries
/// (`##COMPANION-NAME`).
pub const DOCS_SUFFIX: &str = "-docs";

/// Where the documentation `<self_group>/<self_name>` stands for the
/// subject at the coordinate `subject`.
///
/// ```
/// use vibe_doc::citations::SpecSources;
/// use vibe_doc::manifest::status::documentation_status;
/// use vibe_wire::generated::doc_manifest::DocumentationStatus;
///
/// // The convention needs no source: `<name>-docs` in the subject's own
/// // group is primary, which is what lets a manual read out of the store
/// // carry the standing the site gives it.
/// let status = documentation_status(
///     "org.vibevm.core/vibevm",
///     "org.vibevm.core",
///     "vibevm-docs",
///     &SpecSources::new(),
/// );
/// assert_eq!(status, DocumentationStatus::Primary);
///
/// // A name the convention does not reach is community until a subject
/// // says otherwise.
/// let status = documentation_status(
///     "org.vibevm.core/vibevm",
///     "com.example",
///     "my-vibe-book",
///     &SpecSources::new(),
/// );
/// assert_eq!(status, DocumentationStatus::Community);
/// ```
pub fn documentation_status(
    subject: &str,
    self_group: &str,
    self_name: &str,
    sources: &SpecSources,
) -> DocumentationStatus {
    let Some((subject_group, subject_name)) = subject.split_once('/') else {
        return DocumentationStatus::Community;
    };
    let me = format!("{self_group}/{self_name}");
    match declared_documentation(subject_group, subject_name, sources) {
        // A declared `[documentation]` replaces the convention entirely,
        // which is the whole reason a subject bothers to declare one.
        Some(declared) => {
            if declared.primary.as_deref() == Some(me.as_str()) {
                DocumentationStatus::Primary
            } else if declared.official.iter().any(|o| o == &me) {
                DocumentationStatus::Official
            } else {
                DocumentationStatus::Community
            }
        }
        // The convention: `<name>-docs` in the subject's own group counts
        // as official AND primary, so a subject need not re-release for
        // the obvious case.
        None => {
            let conventional = format!("{subject_name}{DOCS_SUFFIX}");
            if self_group == subject_group && self_name == conventional {
                DocumentationStatus::Primary
            } else {
                DocumentationStatus::Community
            }
        }
    }
}

/// Where the translation `<self_group>/<self_name>`, written in `lang`,
/// stands for the source documentation at `source`.
///
/// ```
/// use vibe_doc::manifest::status::translation_status;
/// use vibe_wire::generated::doc_manifest::TranslationStatus;
///
/// let official = translation_status(
///     "org.vibevm.core/vibevm-docs",
///     "org.vibevm.core",
///     "vibevm-docs-ru",
///     "ru",
/// );
/// assert_eq!(official, TranslationStatus::Official);
///
/// // Same name, another group: nobody but the source's publisher can
/// // make a translation official.
/// let community = translation_status(
///     "org.vibevm.core/vibevm-docs",
///     "com.example",
///     "vibevm-docs-ru",
///     "ru",
/// );
/// assert_eq!(community, TranslationStatus::Community);
/// ```
pub fn translation_status(
    source: &str,
    self_group: &str,
    self_name: &str,
    lang: &str,
) -> TranslationStatus {
    let Some((source_group, source_name)) = source.split_once('/') else {
        return TranslationStatus::Community;
    };
    // The tag is lower-cased in the name by law (`##COMPANION-TRANSLATION-NAME`),
    // while BCP-47 itself is case-insensitive, so a package declaring
    // `pt-BR` and named `…-pt-br` is the same official translation.
    let conventional = format!("{source_name}-{}", lang.to_ascii_lowercase());
    if self_group == source_group && self_name.eq_ignore_ascii_case(&conventional) {
        TranslationStatus::Official
    } else {
        TranslationStatus::Community
    }
}

/// The strongest standing a documentation holds over any of its subjects.
///
/// A `doc` package may document several subjects and stand differently
/// for each — primary for the one that named it, community for a
/// neighbour it also covers. A shelf, a star and a catalogue row need one
/// value, and the strongest is the honest one: the package IS the primary
/// documentation of something.
pub fn strongest(subjects: &[DocumentedSubject]) -> DocumentationStatus {
    let mut best = DocumentationStatus::Community;
    for subject in subjects {
        if rank(&subject.status) > rank(&best) {
            best = subject.status.clone();
        }
    }
    best
}

/// The order `##DISC-THREE-SIGNALS` prints: primary, official, community.
/// Higher is stronger.
pub fn rank(status: &DocumentationStatus) -> u8 {
    match status {
        DocumentationStatus::Primary => 2,
        DocumentationStatus::Official => 1,
        DocumentationStatus::Community => 0,
    }
}

/// What a subject declares about its own documentation, or `None` when it
/// declares nothing — which is the state the naming convention answers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Declared {
    pub primary: Option<String>,
    pub official: Vec<String>,
}

/// Read `[documentation]` out of the subject's manifest.
///
/// The subject is found through the same four sources a citation resolves
/// against, nearest first, so the answer is about the instance this
/// reader actually has. A subject no source holds returns `None`, exactly
/// as a subject that declares nothing does: both end at the convention,
/// and the convention is the right answer for both — one has not spoken,
/// and the other cannot be heard.
fn declared_documentation(group: &str, name: &str, sources: &SpecSources) -> Option<Declared> {
    let instance = sources.instance_of(group, name)?;
    let text = std::fs::read_to_string(instance.root.join(MANIFEST)).ok()?;
    let parsed: toml::Value = toml::from_str(&text).ok()?;
    let table = parsed.get("documentation")?;
    Some(Declared {
        primary: table
            .get("primary")
            .and_then(toml::Value::as_str)
            .map(str::to_owned),
        official: table
            .get("official")
            .and_then(toml::Value::as_array)
            .map(|entries| {
                entries
                    .iter()
                    .filter_map(toml::Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests;
