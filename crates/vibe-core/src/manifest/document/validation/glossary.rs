//! `[glossary]` — the grammar of a documentation's statement about which
//! of its pages defines its terms (PROP-057 `##GLOSSARY-DECLARED`).
//!
//! A sibling of [`super::navigation`] and out of line for the same two
//! reasons: the file-length budget, and a real seam. Everything here
//! answers one question — is this a well-formed statement about a page of
//! this package — and nothing else in `validate` asks it.
//!
//! ## What this checks, and what `vibe check` checks
//!
//! Only what the manifest can answer ALONE: that the table belongs in a
//! package of this kind, and that `page` looks like a document path.
//!
//! Whether that page EXISTS, and whether its sections are entries a reader
//! can use — a term and a definition — are questions about a directory and
//! about prose. A grammar reads neither, so they live in `vibe check`
//! (`vibe_check::checks::doc_package_contract`, `##GLOSSARY-CHECKED`), the
//! same split a pin's existence has had since `##NAV-PINNED`.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#GLOSSARY-DECLARED");

use crate::PackageKind;
use crate::error::{Error, Result};
use crate::manifest::GlossaryDecl;
use crate::manifest::package::pinned_path_form_is_valid;

/// The whole `[glossary]` grammar.
///
/// `kind` is the package's declared kind when it has one, for a refusal
/// that names what this manifest actually is.
pub(super) fn validate(
    glossary: &GlossaryDecl,
    is_doc: bool,
    kind: Option<PackageKind>,
) -> Result<()> {
    if !is_doc {
        return Err(Error::InvalidManifest {
            reason: format!(
                "[glossary] is legal only in `doc`-kind packages (this manifest is {}) \
                 — it names the page that defines a documentation's terms, and only \
                 documentation has pages \
                 (violates spec://org.vibevm.core/vibevm/common/PROP-057#GLOSSARY-DECLARED; \
                  fix: set [package] kind = \"doc\", or drop the [glossary] table)",
                kind.map_or("not a package".to_string(), |k| format!("kind = \"{k}\"")),
            ),
        });
    }
    if !pinned_path_form_is_valid(&glossary.page) {
        return Err(Error::InvalidManifest {
            reason: format!(
                "[glossary].page `{}` is not a document path — the glossary is named as a \
                 pin and a chapter row name a page, under the spec root with forward \
                 slashes and WITHOUT its extension, because one document is served as \
                 three projections \
                 (violates spec://org.vibevm.core/vibevm/common/PROP-057#GLOSSARY-DECLARED; \
                  fix: write the path alone, e.g. `glossary/index`)",
                glossary.page
            ),
        });
    }
    Ok(())
}
