//! PROP-049 §4 — foreign discipline concepts require structural guards.
//! The genre is advisory (PROP-050 ##CONCEPTS-GATE-SOFTENED): a violation is
//! a warning, homonymy is lawful (declaring the lexeme in the package's own
//! `[boot_snippet].concepts` owns the word in its world), several owners of
//! one lexeme dedup into a single warning silenced by any one lawful
//! relation, and an `[visibility].ignore-concept-warnings` entry — the
//! package's own or the project root's — mutes a named concept outright.
//! The dependency exemption covers only seeping edges (PROP-050
//! ##DEPS-EXEMPTION-NARROWS): a private `[requires]` edge reaches no
//! consumer, so presupposing it in unconditional text still warns.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-049#gate");

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use specmark::cell;
use vibe_core::manifest::{AccessLevel, BootSnippet, Manifest, WhenCondition};

use crate::{Check, CheckId, CheckOptions, CheckReport};

/// The [`CheckId::SnippetPresupposition`] cell.
#[cell(seam = "Check", variant = "snippet-presupposition")]
pub struct SnippetPresuppositionCheck;

impl Check for SnippetPresuppositionCheck {
    fn id(&self) -> CheckId {
        CheckId::SnippetPresupposition
    }

    fn run(&self, project_root: &Path, _opts: &CheckOptions, report: &mut CheckReport) {
        let dictionary = concept_dictionary(project_root);
        if dictionary.is_empty() {
            return;
        }
        let root_ignored = root_ignored_concepts(project_root);
        for package in authored_snippets(project_root) {
            scan_source(
                project_root,
                &package,
                &package.snippet.source,
                None,
                &dictionary,
                &root_ignored,
                report,
            );
            for fragment in &package.snippet.fragments {
                scan_source(
                    project_root,
                    &package,
                    &fragment.source,
                    fragment.when.as_ref(),
                    &dictionary,
                    &root_ignored,
                    report,
                );
            }
        }
    }
}

#[derive(Debug)]
struct AuthoredSnippet {
    root: PathBuf,
    owner: String,
    snippet: BootSnippet,
    /// `group/name` of every `[requires]` package reached over a seeping
    /// edge (public or friends-only): those edges carry the target toward
    /// consumers, so presupposing them is lawful (PROP-049 §4, the
    /// dependency exemption). A private edge reaches no consumer and
    /// exempts nothing (PROP-050 ##DEPS-EXEMPTION-NARROWS).
    requires_seeping: BTreeSet<String>,
    /// PROP-050 ##CONCEPTS-GATE-SOFTENED (d): lexemes the package's own
    /// `[visibility].ignore-concept-warnings` mutes for its whole world.
    ignored: BTreeSet<String>,
}

type ConceptDictionary = BTreeMap<String, BTreeSet<String>>;

fn concept_dictionary(project_root: &Path) -> ConceptDictionary {
    let authored = manifest_paths(
        &project_root.join(vibe_core::layout::current_packages_root()),
        4,
        true,
    );
    let installed = manifest_paths(
        &project_root.join(vibe_core::layout::current_vibedeps_root()),
        3,
        false,
    );
    let mut dictionary = ConceptDictionary::new();
    for path in authored.into_iter().chain(installed) {
        let Ok(manifest) = Manifest::read(&path) else {
            continue;
        };
        let (Some(package), Some(snippet)) = (manifest.package, manifest.boot_snippet) else {
            continue;
        };
        let owner = format!("{}/{}", package.group, package.name);
        for concept in snippet.concepts {
            if !concept.is_empty() {
                dictionary.entry(concept).or_default().insert(owner.clone());
            }
        }
    }
    dictionary
}

fn authored_snippets(project_root: &Path) -> Vec<AuthoredSnippet> {
    // Only the NEWEST authored version slot of each package is scanned:
    // superseded slots are frozen history, never the shipped snippet
    // (PROP-049 §4 — the gate guards what a consumer can install today).
    let mut newest: BTreeMap<String, (semver::Version, AuthoredSnippet)> = BTreeMap::new();
    for path in manifest_paths(
        &project_root.join(vibe_core::layout::current_packages_root()),
        4,
        true,
    ) {
        let Some(root) = path.parent().map(Path::to_path_buf) else {
            continue;
        };
        let Ok(manifest) = Manifest::read(&path) else {
            continue;
        };
        let (Some(package), Some(snippet)) = (manifest.package, manifest.boot_snippet) else {
            continue;
        };
        let version = package.version.clone();
        // PROP-050 ##DEPS-EXEMPTION-NARROWS: only seeping edges (public,
        // friends-only) justify the dependency exemption — a private
        // edge never reaches a consumer, so it contributes no coordinate.
        let requires_seeping = manifest
            .requires
            .iter_pkgrefs()
            .filter_map(|(group, name)| {
                let group = group?;
                (manifest.requires.access_for(group, name) != AccessLevel::Private)
                    .then(|| format!("{group}/{name}"))
            })
            .collect();
        let ignored = manifest
            .visibility
            .map(|visibility| visibility.ignore_concept_warnings.into_iter().collect())
            .unwrap_or_default();
        let owner = format!("{}/{}", package.group, package.name);
        let candidate = AuthoredSnippet {
            root,
            owner: owner.clone(),
            snippet,
            requires_seeping,
            ignored,
        };
        match newest.get(&owner) {
            Some((held, _)) if *held >= version => {}
            _ => {
                newest.insert(owner, (version, candidate));
            }
        }
    }
    newest.into_values().map(|(_, snippet)| snippet).collect()
}

/// PROP-050 ##CONCEPTS-GATE-SOFTENED (d): the project root's
/// `[visibility].ignore-concept-warnings` mutes the named lexemes for
/// the entire tree. A missing or unreadable root manifest is not an
/// error — the mute list is simply empty.
fn root_ignored_concepts(project_root: &Path) -> BTreeSet<String> {
    Manifest::read(project_root.join(Manifest::FILENAME))
        .ok()
        .and_then(|manifest| manifest.visibility)
        .map(|visibility| visibility.ignore_concept_warnings.into_iter().collect())
        .unwrap_or_default()
}

/// Locate only the manifest shapes named by PROP-049: authored
/// `packages/*/*/v*/vibe.toml` and installed `vibedeps/*/*/vibe.toml`.
fn manifest_paths(base: &Path, depth: usize, authored: bool) -> Vec<PathBuf> {
    if !base.is_dir() {
        return Vec::new();
    }
    let mut paths: Vec<PathBuf> = walkdir::WalkDir::new(base)
        .min_depth(depth)
        .max_depth(depth)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file() && entry.file_name() == Manifest::FILENAME)
        .filter(|entry| {
            !authored
                || entry
                    .path()
                    .parent()
                    .and_then(Path::file_name)
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with('v'))
        })
        .map(|entry| entry.into_path())
        .collect();
    paths.sort();
    paths
}

fn scan_source(
    project_root: &Path,
    package: &AuthoredSnippet,
    source: &Path,
    guard: Option<&WhenCondition>,
    dictionary: &ConceptDictionary,
    root_ignored: &BTreeSet<String>,
    report: &mut CheckReport,
) {
    let path = package.root.join(source);
    let Ok(text) = fs::read_to_string(&path) else {
        return;
    };
    let rel = path
        .strip_prefix(project_root)
        .unwrap_or(&path)
        .to_path_buf();
    let rel_label = rel.display().to_string().replace('\\', "/");
    let mut fence: Option<char> = None;
    for (line_index, line) in text.lines().enumerate() {
        if let Some(marker) = fence_marker(line) {
            if fence == Some(marker) {
                fence = None;
            } else if fence.is_none() {
                fence = Some(marker);
            }
            continue;
        }
        if fence.is_some() {
            continue;
        }
        let visible = mask_inline_code(line);
        for (concept, owners) in dictionary {
            if !contains_concept(&visible, concept) {
                continue;
            }
            // PROP-050 ##CONCEPTS-GATE-SOFTENED (d): the lexeme is muted by
            // the scanned package's own
            // `[visibility].ignore-concept-warnings` or by the project
            // root's — the root mutes the whole tree.
            if package.ignored.contains(concept) || root_ignored.contains(concept) {
                continue;
            }
            // Homonymy is lawful: a package declaring the lexeme in its own
            // concepts owns the word in its own world (PROP-050
            // ##CONCEPTS-GATE-SOFTENED) — check before the owner loop.
            if package.snippet.concepts.contains(concept) {
                continue;
            }
            // Owner-dedup: one lawful relation to ANY owner legitimises the
            // mention, and an unexplained use warns once naming all owners.
            let lawful = owners.iter().any(|owner| {
                owner == &package.owner
                    || package.requires_seeping.contains(owner)
                    || guarded_by(guard, owner)
            });
            if lawful {
                continue;
            }
            let owner_list = owners
                .iter()
                .map(|owner| format!("`{owner}`"))
                .collect::<Vec<_>>()
                .join(", ");
            let line_number = line_index + 1;
            report.warn(
                CheckId::SnippetPresupposition,
                Some(rel.clone()),
                Some(line_number),
                format!(
                    "{rel_label}:{line_number}: foreign concept `{concept}` belongs to package(s) \
                     {owner_list} (PROP-050 ##CONCEPTS-GATE-SOFTENED; fix: move the mention into a \
                     [[boot_snippet.fragment]] guarded by when = \"installed:<one of the owners>\", \
                     declare the lexeme in your own [boot_snippet].concepts, add the lexeme to \
                     [visibility].ignore-concept-warnings, or drop it)"
                ),
            );
        }
    }
}

fn guarded_by(guard: Option<&WhenCondition>, owner: &str) -> bool {
    guard
        .and_then(WhenCondition::installed_identity)
        .is_some_and(|(group, name)| owner == format!("{group}/{name}"))
}

fn fence_marker(line: &str) -> Option<char> {
    let trimmed = line.trim_start();
    if trimmed.starts_with("```") {
        Some('`')
    } else if trimmed.starts_with("~~~") {
        Some('~')
    } else {
        None
    }
}

fn mask_inline_code(line: &str) -> String {
    let mut masked = String::with_capacity(line.len());
    let mut in_code = false;
    for character in line.chars() {
        if character == '`' {
            in_code = !in_code;
            masked.push(' ');
        } else if in_code {
            masked.push(' ');
        } else {
            masked.push(character);
        }
    }
    mask_anchor_lexemes(&masked)
}

/// Anchor lexemes are addresses, not prose: a `@fact:<ID>` opener and a
/// heading `{#anchor}` may carry a concept word inside their identifier
/// (`MEMBER-WAL-SPECSPACES`) without saying anything to a reader. Blank
/// them before the concept scan.
fn mask_anchor_lexemes(line: &str) -> String {
    let mut masked = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(at) = rest.find("@fact:").or_else(|| rest.find("{#")) {
        let (head, tail) = rest.split_at(at);
        masked.push_str(head);
        let end = tail
            .char_indices()
            .find(|(_, c)| c.is_whitespace() || *c == '}')
            .map_or(tail.len(), |(i, _)| i);
        masked.extend(std::iter::repeat_n(' ', end));
        rest = &tail[end..];
    }
    masked.push_str(rest);
    masked
}

fn contains_concept(line: &str, concept: &str) -> bool {
    if concept.contains('/') {
        return line.contains(concept);
    }
    line.match_indices(concept).any(|(index, _)| {
        let before = line[..index].chars().next_back();
        let after = line[index + concept.len()..].chars().next();
        before.is_none_or(|character| !is_word_character(character))
            && after.is_none_or(|character| !is_word_character(character))
    })
}

fn is_word_character(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

#[cfg(test)]
mod tests;
