//! `[navigation]` — the grammar of what a documentation says about the
//! order its own pages are read in (PROP-057 `##NAV-PINNED`,
//! `##NAV-CHAPTERS`, `##NAV-CHAPTERS-CHECKED`,
//! `##NAV-CHAPTERS-TRANSLATION`).
//!
//! Out of line from the rest of the document's validation per the
//! file-length budget, and along a real seam rather than an arbitrary
//! cut: everything here answers one question — is this a well-formed
//! statement about a page tree — and nothing else in `validate` asks it.
//!
//! ## What this checks, and what `vibe check` checks
//!
//! Only what the manifest can answer ALONE: that a path looks like a
//! document path, that a section is one folder under a name a reader
//! sees, that a chapter has an id and a title, that no id is used twice,
//! that no page is named twice on the whole path, and that a translation
//! renames chapters instead of re-declaring their pages.
//!
//! Whether a named page EXISTS, and whether every page of the package
//! found a chapter, are questions about a directory. A grammar reads no
//! directories, so those live in `vibe check`
//! (`vibe_check::checks::doc_package_contract`) — the same split a pin's
//! existence has had since `##NAV-PINNED`.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#NAV-PINNED");

use crate::PackageKind;
use crate::error::{Error, Result};
use crate::manifest::NavigationDecl;
use crate::manifest::package::{
    chapter_id_form_is_valid, pinned_path_form_is_valid, section_id_form_is_valid,
};

/// The whole `[navigation]` grammar, in the order an author meets it.
///
/// `kind` is the package's declared kind when it has one, for the refusal
/// that names what this manifest actually is; `is_translation` is the
/// presence of `[translates]`, which is what turns the chapter rows from a
/// declaration of the path into a renaming of somebody else's.
pub(super) fn validate(
    navigation: &NavigationDecl,
    is_doc: bool,
    kind: Option<PackageKind>,
    is_translation: bool,
) -> Result<()> {
    if !is_doc {
        return Err(Error::InvalidManifest {
            reason: format!(
                "[navigation] is legal only in `doc`-kind packages (this manifest is {}) \
                 — it names pages and sections, and only documentation has a page tree \
                 (violates spec://org.vibevm.core/vibevm/common/PROP-057#NAV-PINNED; \
                  fix: set [package] kind = \"doc\", or drop the [navigation] table)",
                kind.map_or("not a package".to_string(), |k| format!("kind = \"{k}\"")),
            ),
        });
    }
    for path in &navigation.pinned {
        if !pinned_path_form_is_valid(path) {
            return Err(Error::InvalidManifest {
                reason: format!(
                    "[navigation].pinned `{path}` is not a document path — a pin names a \
                     page under the spec root with forward slashes and WITHOUT its \
                     extension, because one document is served as three projections \
                     (violates spec://org.vibevm.core/vibevm/common/PROP-057#NAV-PINNED; \
                      fix: write the path alone, e.g. `start/what-vibevm-is`)"
                ),
            });
        }
    }
    for section in &navigation.sections {
        if !section_id_form_is_valid(&section.id) {
            return Err(Error::InvalidManifest {
                reason: format!(
                    "[[navigation.section]].id `{}` is not a folder — a section is one \
                     top-level folder of the page tree, named as the page paths spell it \
                     (violates spec://org.vibevm.core/vibevm/common/PROP-057#NAV-PINNED; \
                      fix: write the first segment alone, e.g. `start`)",
                    section.id
                ),
            });
        }
        if section.title.trim().is_empty() {
            return Err(Error::InvalidManifest {
                reason: format!(
                    "[[navigation.section]] `{}` carries an empty `title` — the title is \
                     the whole reason a section is named, and an empty one would show a \
                     blank heading over the pages \
                     (violates spec://org.vibevm.core/vibevm/common/PROP-057#NAV-PINNED; \
                      fix: give the section its name in this package's own language)",
                    section.id
                ),
            });
        }
    }
    chapters(navigation, is_translation)
}

/// `[[navigation.chapter]]` — the learning path's own grammar
/// (PROP-057 `##NAV-CHAPTERS`, `##NAV-CHAPTERS-CHECKED`,
/// `##NAV-CHAPTERS-TRANSLATION`).
fn chapters(navigation: &NavigationDecl, is_translation: bool) -> Result<()> {
    let mut seen_ids: Vec<&str> = Vec::new();
    let mut seen_pages: Vec<&str> = Vec::new();
    for chapter in &navigation.chapters {
        if !chapter_id_form_is_valid(&chapter.id) {
            return Err(Error::InvalidManifest {
                reason: "[[navigation.chapter]] carries an empty `id` — the id is what a \
                         translation names the chapter by, and a row without one cannot be \
                         renamed or reported on \
                         (violates spec://org.vibevm.core/vibevm/common/PROP-057#NAV-CHAPTERS-CHECKED; \
                          fix: give the chapter an id of its own, e.g. `id = \"start\"`)"
                    .to_string(),
            });
        }
        if chapter.title.trim().is_empty() {
            return Err(Error::InvalidManifest {
                reason: format!(
                    "[[navigation.chapter]] `{}` carries an empty `title` — the contents shows \
                     the title over the chapter's pages, and an empty one would number a blank \
                     line \
                     (violates spec://org.vibevm.core/vibevm/common/PROP-057#NAV-CHAPTERS-CHECKED; \
                      fix: give the chapter its name in this package's own language)",
                    chapter.id
                ),
            });
        }
        if seen_ids.contains(&chapter.id.as_str()) {
            return Err(Error::InvalidManifest {
                reason: format!(
                    "[[navigation.chapter]] uses the id `{}` twice — a chapter id is the one \
                     handle a translation and a report have on a chapter, and two rows under it \
                     make every mention of it ambiguous \
                     (violates spec://org.vibevm.core/vibevm/common/PROP-057#NAV-CHAPTERS-CHECKED; \
                      fix: give each chapter an id of its own)",
                    chapter.id
                ),
            });
        }
        seen_ids.push(&chapter.id);

        let Some(pages) = &chapter.pages else {
            continue;
        };
        if is_translation {
            return Err(Error::InvalidManifest {
                reason: format!(
                    "[[navigation.chapter]] `{}` lists `pages`, and this package declares \
                     [translates] — a translation takes the learning path of the documentation \
                     it adapts and only names its chapters in its own language; a second copy of \
                     the path would be a second copy of one fact, and the day they disagreed the \
                     two languages would be two different manuals \
                     (violates spec://org.vibevm.core/vibevm/common/PROP-057#NAV-CHAPTERS-TRANSLATION; \
                      fix: keep `id` and `title` and drop `pages`; a chapter this package does \
                      not name keeps the source's title)",
                    chapter.id
                ),
            });
        }
        for page in pages {
            if !pinned_path_form_is_valid(page) {
                return Err(Error::InvalidManifest {
                    reason: format!(
                        "[[navigation.chapter]] `{}` names `{page}`, which is not a document path \
                         — a chapter holds pages spelled exactly as a pin spells them, under the \
                         spec root with forward slashes and WITHOUT the extension, because one \
                         document is served as three projections \
                         (violates spec://org.vibevm.core/vibevm/common/PROP-057#NAV-CHAPTERS-CHECKED; \
                          fix: write the path alone, e.g. `start/what-vibevm-is`)",
                        chapter.id
                    ),
                });
            }
            if seen_pages.contains(&page.as_str()) {
                return Err(Error::InvalidManifest {
                    reason: format!(
                        "the learning path names `{page}` twice — followed from the first page of \
                         the first chapter to the last, the path meets every page of the package \
                         ONCE, and a page in two chapters gives a reader two places to be told \
                         one thing \
                         (violates spec://org.vibevm.core/vibevm/common/PROP-057#NAV-CHAPTERS-CHECKED; \
                          fix: leave the page in the chapter it belongs to and remove the other \
                          mention)"
                    ),
                });
            }
            seen_pages.push(page);
        }
    }
    Ok(())
}
