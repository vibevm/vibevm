//! A translation's contract against the documentation it adapts
//! ([PROP-057](../../../../vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml)
//! §5).
//!
//! A translation is a package of its own — different authors, different
//! rhythms, officiality per language — so the two facts that bind it to
//! its source have to be checked across a package boundary, and both
//! are checked here.
//!
//! **It must say which language it is in.** There is no `lang` field:
//! the language of a `doc` package is `[i18n].canonical` of PROP-003
//! §2.7 (`##LOC-LANGUAGE-FIELD`), whose default is `en`. A default is
//! exactly what a translation must not lean on — silence would make
//! every adaptation claim English — so the key must be WRITTEN. That is
//! a question about the file's text, not about the parsed manifest
//! (which fills the default in), and it is the one question this cell
//! asks of the raw TOML: a single key lookup, never a second copy of the
//! manifest grammar.
//!
//! **It must document the same subjects as its source**
//! (`##LOC-DOCUMENTS-MATCH`). A translation that documents something
//! else is not an adaptation of that documentation; the site would then
//! show two manuals for one subject, disagreeing about what the subject
//! even is. The source is looked for where a source can honestly be
//! looked for offline — the project's own `vibevm/vibepacks/` and the
//! machine store — and when it is in neither, the finding is a WARNING
//! naming the reason, never a silent pass and never an error blaming the
//! author for a package that is simply not on this machine.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#localization");

use std::path::{Path, PathBuf};

use specmark::cell;
use vibe_core::manifest::{DocumentsDecl, Manifest, TranslatesDecl};

use crate::{Check, CheckId, CheckOptions, CheckReport};

/// The [`CheckId::DocTranslation`] cell.
#[cell(seam = "Check", variant = "doc-translation")]
pub struct DocTranslationCheck;

/// `true` when the manifest TEXT writes `canonical` under `[i18n]`.
///
/// The parsed [`Manifest`] cannot answer this: `I18nDecl::canonical`
/// carries `#[serde(default)]`, so an absent key and an explicit `en`
/// arrive identically. The distinction is the whole rule here, so the
/// presence is read from the document — one key, by path, with no
/// grammar restated.
fn canonical_is_written(manifest_text: &str) -> bool {
    manifest_text
        .parse::<toml::Table>()
        .ok()
        .and_then(|table| {
            table
                .get("i18n")?
                .as_table()?
                .get("canonical")
                .map(|_| true)
        })
        .unwrap_or(false)
}

/// The directories a source documentation may honestly be found in
/// offline: the project's own package root first (an in-tree package is
/// the source of truth while it is being developed), then the machine
/// store.
fn source_roots(project_root: &Path) -> Vec<PathBuf> {
    let mut roots = vec![project_root.join(vibe_core::layout::current_packages_root())];
    if let Ok(store) = vibe_registry::store::store_root() {
        roots.push(store);
    }
    roots
}

/// The manifest of the newest version of `<group>/<name>` that
/// satisfies `constraint`, searched under `roots` in order. `None` when
/// no root holds a matching version — the caller turns that into a
/// warning that names the reason, not into an accusation.
fn read_source_manifest(roots: &[PathBuf], coordinate: &str, constraint: &str) -> Option<Manifest> {
    let (group, name) = coordinate.split_once('/')?;
    let requirement = semver::VersionReq::parse(constraint.trim()).ok()?;
    for root in roots {
        let package_dir = root.join(group).join(name);
        let Ok(entries) = std::fs::read_dir(&package_dir) else {
            continue;
        };
        let mut best: Option<(semver::Version, PathBuf)> = None;
        for entry in entries.filter_map(|e| e.ok()) {
            let file_name = entry.file_name();
            let Some(version_text) = file_name.to_str().and_then(|n| n.strip_prefix('v')) else {
                continue;
            };
            let Ok(version) = semver::Version::parse(version_text) else {
                continue;
            };
            if !requirement.matches(&version) {
                continue;
            }
            if best.as_ref().is_none_or(|(best, _)| version > *best) {
                best = Some((version, entry.path()));
            }
        }
        if let Some((_, dir)) = best
            && let Ok(manifest) = Manifest::read(dir.join(Manifest::FILENAME))
        {
            return Some(manifest);
        }
    }
    None
}

/// One subject as the comparison sees it: the coordinate and its
/// constraint, sorted, so two manifests that list the same subjects in a
/// different order still match.
fn subject_set(documents: &[DocumentsDecl]) -> Vec<String> {
    let mut set: Vec<String> = documents
        .iter()
        .map(|d| format!("{}@{}", d.package, d.version))
        .collect();
    set.sort();
    set
}

/// The rendered list for a message — `none` reads better than an empty
/// pair of brackets when the source genuinely documents nothing.
fn render(subjects: &[String]) -> String {
    if subjects.is_empty() {
        "none".to_string()
    } else {
        subjects.join(", ")
    }
}

impl Check for DocTranslationCheck {
    fn id(&self) -> CheckId {
        CheckId::DocTranslation
    }

    fn run(&self, project_root: &Path, _opts: &CheckOptions, report: &mut CheckReport) {
        let manifest_file = project_root.join(Manifest::FILENAME);
        let Ok(manifest) = Manifest::read(&manifest_file) else {
            return;
        };
        let Some(translates) = manifest.translates.as_ref() else {
            return;
        };
        let manifest_path = Path::new(Manifest::FILENAME).to_path_buf();

        let written = std::fs::read_to_string(&manifest_file)
            .map(|text| canonical_is_written(&text))
            .unwrap_or(false);
        if !written {
            report.err(
                CheckId::DocTranslation,
                Some(manifest_path.clone()),
                None,
                format!(
                    "this package adapts `{source}` but never says which language it is in — a \
                     translation must write `[i18n] canonical = \"<BCP-47 tag>\"`, because the \
                     default is `en` and silence would make every adaptation claim English. \
                     There is no `lang` field: the language of a `doc` package IS \
                     `[i18n].canonical` \
                     (violates spec://org.vibevm.core/vibevm/common/PROP-057#LOC-LANGUAGE-FIELD; \
                     fix: add an [i18n] table with `canonical` set to this adaptation's language)",
                    source = translates.package,
                ),
            );
        }

        self.compare_subjects(project_root, &manifest, translates, &manifest_path, report);
    }
}

impl DocTranslationCheck {
    /// `##LOC-DOCUMENTS-MATCH` — the subjects of a translation and of
    /// its source are the same set, or the reader is told which way they
    /// differ; an unreachable source is a warning that names why.
    fn compare_subjects(
        &self,
        project_root: &Path,
        manifest: &Manifest,
        translates: &TranslatesDecl,
        manifest_path: &Path,
        report: &mut CheckReport,
    ) {
        let roots = source_roots(project_root);
        let Some(source) = read_source_manifest(&roots, &translates.package, &translates.version)
        else {
            report.warn(
                CheckId::DocTranslation,
                Some(manifest_path.to_path_buf()),
                None,
                format!(
                    "cannot compare `[[documents]]` with the source: no version of `{source}` \
                     matching `{constraint}` is readable offline — neither in this project's \
                     `{packages}` nor in the machine store. The rule is unchecked here, not \
                     satisfied \
                     (spec://org.vibevm.core/vibevm/common/PROP-057#LOC-DOCUMENTS-MATCH; \
                     fix: `vibe cache add {source}` to warm the source, then re-run)",
                    source = translates.package,
                    constraint = translates.version,
                    packages = vibe_core::layout::current_packages_root()
                        .to_string_lossy()
                        .replace('\\', "/"),
                ),
            );
            return;
        };

        let ours = subject_set(&manifest.documents);
        let theirs = subject_set(&source.documents);
        if ours == theirs {
            return;
        }
        report.err(
            CheckId::DocTranslation,
            Some(manifest_path.to_path_buf()),
            None,
            format!(
                "this translation documents {ours}, its source `{source}` documents {theirs} — \
                 an adaptation that documents something else is not an adaptation of that \
                 documentation, and the site would show two manuals disagreeing about one \
                 subject \
                 (violates spec://org.vibevm.core/vibevm/common/PROP-057#LOC-DOCUMENTS-MATCH; \
                 fix: copy the source's `[[documents]]` verbatim, constraints included)",
                ours = render(&ours),
                theirs = render(&theirs),
                source = translates.package,
            ),
        );
    }
}

#[cfg(test)]
#[path = "doc_translation/tests.rs"]
mod tests;
