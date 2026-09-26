//! The `doc` package's own contract — what documentation must ship and
//! what it may never declare
//! ([PROP-057](../../../../vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml)
//! §3).
//!
//! Three rules, one idea: **documentation is read, not run.** A `doc`
//! package is never installed, never boots, never dispatches a binary
//! and never serves an MCP tool (`##KIND-DOC-MUST-NOT-EXECUTE`); it
//! ships a front door a human can open without any of that
//! (`##KIND-DOC-MUST-DOCUMENT` — the README is where a reader who found
//! the repository rather than the site lands). The fourth finding is a
//! warning and belongs to the other end of the edge: a subject naming
//! its `primary` documentation in a FOREIGN group is legal and
//! sometimes right, but it is also how a typo looks, so it is said out
//! loud once (`##REL-DOCUMENTATION-UNVERSIONED`).
//!
//! **What this cell deliberately does NOT restate.** `[[documents]]`,
//! `title` and `abstract` are refused by the manifest grammar itself
//! (`Manifest::validate`, PROP-057 `##REL-DOCUMENTS-REQUIRED`,
//! `##CARD-TITLE`, `##CARD-DESCRIPTION-AND-ABSTRACT`), so a `doc`
//! package missing any of them does not parse and `vibe check` already
//! errors on it through `manifest_validity`, citing the same anchors and
//! the fix. Re-deriving those three here would mean a second copy of the
//! manifest grammar living in the linter — the exact rot the index
//! scanner's own history records — for a message the user already gets.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#kinds");

use std::path::Path;

use specmark::cell;
use vibe_core::PackageKind;
use vibe_core::manifest::{Manifest, NavigationDecl};
// Where a package's pages live (PROP-052), borrowed from the pipeline
// that walks them rather than spelled a second time here.
use vibe_doc::pages::SPEC_ROOT;

use crate::{Check, CheckId, CheckOptions, CheckReport};

/// The [`CheckId::DocPackageContract`] cell.
#[cell(seam = "Check", variant = "doc-package-contract")]
pub struct DocPackageContractCheck;

/// The front door of a package tree, and the one file this cell asks a
/// `doc` package for by name.
const README: &str = "README.md";

/// The extension a page is authored in. A pin names the DOCUMENT, so
/// this is added back to find the file the pin points at.
const PAGE_EXTENSION: &str = "xml";

/// Read the project's own manifest, or `None` when there is none or it
/// is refused. A refused manifest is `manifest_validity`'s finding, not
/// this cell's: reporting a doc-contract defect on a manifest nobody
/// could read would be guessing at bytes.
fn manifest_of(project_root: &Path) -> Option<Manifest> {
    Manifest::read(project_root.join(Manifest::FILENAME)).ok()
}

/// The group half of a `<group>/<name>` coordinate.
fn group_of(coordinate: &str) -> Option<&str> {
    coordinate.split_once('/').map(|(group, _)| group)
}

/// The declared learning path against the page tree it claims to cover
/// (PROP-057 `##NAV-CHAPTERS-CHECKED`).
///
/// Two findings and they are each other's mirror: a chapter naming a page
/// the package does not have, and a page of the package no chapter names.
/// The second is the one the rule exists for — a page added later must be
/// GIVEN its place on the path, and without this it would simply be
/// missing from the reader's contents with nothing said.
///
/// Three things this deliberately does not do. It does not run on a
/// package that declared no path: a documentation with no chapters is not
/// a documentation with an incomplete one, and every manual written before
/// the rows existed is in that state. It does not run on a translation:
/// an adaptation takes its source's path and names no pages at all
/// (`##NAV-CHAPTERS-TRANSLATION`), so measuring its coverage would be
/// measuring the source's. And it does not restate the manifest grammar —
/// a blank id, a duplicate id, a page named twice and a path spelled with
/// an extension are refused by `Manifest::validate` before this cell sees
/// the file.
fn check_learning_path(
    project_root: &Path,
    manifest: &Manifest,
    navigation: &NavigationDecl,
    manifest_path: &Path,
    report: &mut CheckReport,
) {
    if navigation.chapters.is_empty() || manifest.translates.is_some() {
        return;
    }
    // «Which pages does this package have» is a question about a
    // directory, asked of the pipeline that owns the answer. A page the
    // pivot would refuse is still a page of the tree and still owes the
    // path a place, so the addresses are walked rather than the documents
    // read.
    let Ok(present) = vibe_doc::pages::documents(project_root) else {
        return;
    };
    let mut placed: Vec<&str> = Vec::new();
    for chapter in &navigation.chapters {
        for page in chapter.pages.iter().flatten() {
            placed.push(page);
            if present.iter().any(|document| document == page) {
                continue;
            }
            report.err(
                CheckId::DocPackageContract,
                Some(manifest_path.to_path_buf()),
                None,
                format!(
                    "the learning path's chapter `{id}` names `{page}`, and this package carries \
                     no page at `{SPEC_ROOT}/{page}.{PAGE_EXTENSION}` — a path that walks through \
                     a page nobody wrote leads a reader to a dead link and numbers a chapter one \
                     page longer than it is \
                     (violates \
                     spec://org.vibevm.core/vibevm/common/PROP-057#NAV-CHAPTERS-CHECKED; \
                     fix: correct the path, or drop it from the chapter if the page is gone)",
                    id = chapter.id,
                ),
            );
        }
    }
    for document in &present {
        if placed.contains(&document.as_str()) {
            continue;
        }
        report.err(
            CheckId::DocPackageContract,
            Some(manifest_path.to_path_buf()),
            None,
            format!(
                "this package carries `{SPEC_ROOT}/{document}.{PAGE_EXTENSION}` and no chapter of \
                 the learning path holds it — followed from the first page of the first chapter \
                 to the last, the path meets every page of the package, so a page added later \
                 must be given its place on it rather than left out of the contents in silence \
                 (violates \
                 spec://org.vibevm.core/vibevm/common/PROP-057#NAV-CHAPTERS-CHECKED; \
                 fix: add `{document}` to the chapter it belongs to, in the order a reader should \
                 meet it)"
            ),
        );
    }
}

/// The declared glossary against the package that declared it
/// (PROP-057 `##GLOSSARY-CHECKED`).
///
/// Three findings, and the library owns all three: the page has to be
/// there, every entry has to name its term, and every entry has to state a
/// definition. The judgment lives in `vibe_doc::glossary` rather than here
/// because the reader shows those same entries in its cards and the style
/// linter measures those same terms — three surfaces, one reading of what a
/// glossary is.
///
/// A package that declares none yields nothing: a documentation without a
/// glossary is not a documentation with a broken one, and the path
/// `glossary/index.xml` means nothing by itself (`##GLOSSARY-DECLARED`).
fn check_glossary(project_root: &Path, manifest_path: &Path, report: &mut CheckReport) {
    // The pages are read here rather than walked: whether an entry states a
    // definition is a question about a DOCUMENT, and the pivot is what
    // answers it. A page the pivot refuses is reported as that.
    let Ok(set) = vibe_doc::pages::read_package(project_root) else {
        return;
    };
    let Ok(defects) = vibe_doc::glossary::check(project_root, &set) else {
        return;
    };
    for defect in defects {
        report.err(
            CheckId::DocPackageContract,
            Some(manifest_path.to_path_buf()),
            None,
            format!(
                "{} \
                 (violates \
                 spec://org.vibevm.core/vibevm/common/PROP-057#GLOSSARY-CHECKED; \
                 fix: correct the page or the entry, or drop [glossary] if this documentation \
                 defines no terms)",
                defect.render()
            ),
        );
    }
}

impl Check for DocPackageContractCheck {
    fn id(&self) -> CheckId {
        CheckId::DocPackageContract
    }

    fn run(&self, project_root: &Path, _opts: &CheckOptions, report: &mut CheckReport) {
        let Some(manifest) = manifest_of(project_root) else {
            return;
        };
        let Some(package) = manifest.package.as_ref() else {
            return;
        };
        let manifest_path = Path::new(Manifest::FILENAME).to_path_buf();

        // The subject's end of the edge, checked in a package of ANY
        // kind: `[documentation]` is legal everywhere, and a `primary`
        // pointing outside the subject's own group is the shape a typo
        // takes. It stays a warning — a group may genuinely hand its
        // documentation to another publisher.
        if let Some(documentation) = manifest.documentation.as_ref()
            && let Some(primary) = documentation.primary.as_deref()
            && let Some(primary_group) = group_of(primary)
            && primary_group != package.group.as_str()
        {
            report.warn(
                CheckId::DocPackageContract,
                Some(manifest_path.clone()),
                None,
                format!(
                    "[documentation].primary names `{primary}`, published by `{primary_group}` \
                     rather than this package's own group `{own}` — legal, and sometimes \
                     deliberate, but this is also what a mistyped coordinate looks like \
                     (spec://org.vibevm.core/vibevm/common/PROP-057#REL-DOCUMENTATION-UNVERSIONED; \
                     fix: correct the coordinate, or keep it if the other group really does \
                     publish this package's primary documentation)",
                    own = package.group.as_str(),
                ),
            );
        }

        if package.kind != PackageKind::Doc {
            return;
        }

        // Documentation never enters a boot lane and never executes.
        // Each forbidden section is its own finding: an author who
        // declared two of them should be told about both, not led
        // through one recheck per section.
        let forbidden: [(&str, bool); 3] = [
            ("[boot_snippet]", manifest.boot_snippet.is_some()),
            ("[[binary]]", !manifest.binaries.is_empty()),
            ("[[mcp_server]]", !manifest.mcp_servers.is_empty()),
        ];
        for (section, present) in forbidden {
            if !present {
                continue;
            }
            report.err(
                CheckId::DocPackageContract,
                Some(manifest_path.clone()),
                None,
                format!(
                    "a `doc` package declares `{section}` — documentation is read, not run: it \
                     never enters a boot lane, dispatches a binary or serves a tool \
                     (violates \
                     spec://org.vibevm.core/vibevm/common/PROP-057#KIND-DOC-MUST-NOT-EXECUTE; \
                     fix: drop `{section}`, or ship the executable part as a package of its own \
                     kind and point at it with [documentation])"
                ),
            );
        }

        // A pin names a page, and the manifest grammar can only check
        // that it LOOKS like one: whether the page is there is a
        // question about a tree, and this is where a tree is read. The
        // path is spelled as the pin spells it — without the extension —
        // because that is the spelling the author has to correct.
        if let Some(navigation) = manifest.navigation.as_ref() {
            for pinned in &navigation.pinned {
                if project_root
                    .join(SPEC_ROOT)
                    .join(format!("{pinned}.{PAGE_EXTENSION}"))
                    .is_file()
                {
                    continue;
                }
                report.err(
                    CheckId::DocPackageContract,
                    Some(manifest_path.clone()),
                    None,
                    format!(
                        "[navigation].pinned names `{pinned}`, and this package carries no page \
                         at `{SPEC_ROOT}/{pinned}.{PAGE_EXTENSION}` — a pin moves a page to the \
                         top of the navigation, so a pin at nothing moves nothing and hides a \
                         typo in plain sight \
                         (violates \
                         spec://org.vibevm.core/vibevm/common/PROP-057#NAV-PINNED; \
                         fix: correct the path, or drop the pin if the page is gone)"
                    ),
                );
            }
            check_learning_path(project_root, &manifest, navigation, &manifest_path, report);
        }

        // The glossary a documentation declares: the page has to be there,
        // and its sections have to be entries a reader can use.
        if manifest.glossary.is_some() {
            check_glossary(project_root, &manifest_path, report);
        }

        // The front door. A reader who arrived at the repository rather
        // than the site has nothing to open without it, and the
        // publication gate would refuse the package later anyway.
        if !project_root.join(README).is_file() {
            report.err(
                CheckId::DocPackageContract,
                Some(Path::new(README).to_path_buf()),
                None,
                format!(
                    "a `doc` package has no `{README}` — the pages live under `vibevm/vibespecs/` \
                     and the site renders them, but a reader who arrives at the repository needs \
                     a front door here \
                     (violates \
                     spec://org.vibevm.core/vibevm/common/PROP-057#KIND-DOC-MUST-DOCUMENT; \
                     fix: add `{README}` naming what this documentation covers and where its \
                     entry page is)"
                ),
            );
        }
    }
}

#[cfg(test)]
#[path = "doc_package_contract/tests.rs"]
mod tests;
