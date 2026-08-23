//! PROP-049 §4 — foreign discipline concepts require structural guards.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-049#gate");

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use specmark::cell;
use vibe_core::manifest::{BootSnippet, Manifest, WhenCondition};

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
        for package in authored_snippets(project_root) {
            scan_source(
                project_root,
                &package,
                &package.snippet.source,
                None,
                &dictionary,
                report,
            );
            for fragment in &package.snippet.fragments {
                scan_source(
                    project_root,
                    &package,
                    &fragment.source,
                    fragment.when.as_ref(),
                    &dictionary,
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
    /// `group/name` of every `[requires]` package: a declared dependency
    /// guarantees co-installation, so presupposing it is lawful
    /// (PROP-049 §4, the dependency exemption).
    requires: BTreeSet<String>,
}

type ConceptDictionary = BTreeMap<String, BTreeSet<String>>;

fn concept_dictionary(project_root: &Path) -> ConceptDictionary {
    let authored = manifest_paths(&project_root.join("packages"), 4, true);
    let installed = manifest_paths(&project_root.join("vibedeps"), 3, false);
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
    for path in manifest_paths(&project_root.join("packages"), 4, true) {
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
        let requires = manifest
            .requires
            .iter_pkgrefs()
            .filter_map(|(group, name)| group.map(|g| format!("{g}/{name}")))
            .collect();
        let owner = format!("{}/{}", package.group, package.name);
        let candidate = AuthoredSnippet {
            root,
            owner: owner.clone(),
            snippet,
            requires,
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
            for owner in owners {
                if owner == &package.owner
                    || package.requires.contains(owner)
                    || guarded_by(guard, owner)
                {
                    continue;
                }
                let line_number = line_index + 1;
                report.err(
                    CheckId::SnippetPresupposition,
                    Some(rel.clone()),
                    Some(line_number),
                    format!(
                        "{rel_label}:{line_number}: foreign concept `{concept}` belongs to package \
                         `{owner}` (violates \
                         spec://org.vibevm.core/vibevm/common/PROP-049#gate; fix: move the mention \
                         into a [[boot_snippet.fragment]] guarded by when = \"installed:{owner}\", \
                         or drop it)"
                    ),
                );
            }
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
mod tests {
    use std::fs;
    use std::path::Path;

    use tempfile::tempdir;

    use super::SnippetPresuppositionCheck;
    use crate::{Check, CheckId, CheckOptions, CheckReport, Severity};

    fn write(root: &Path, rel: &str, body: &str) {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, body).unwrap();
    }

    fn write_owner(root: &Path) {
        write(
            root,
            "packages/org.example/d/v1.0.0/vibe.toml",
            r#"[package]
group = "org.example"
name = "d"
kind = "flow"
version = "1.0.0"

[boot_snippet]
source = "boot/main.md"
concepts = ["WAL"]
"#,
        );
        write(
            root,
            "packages/org.example/d/v1.0.0/boot/main.md",
            "# D\n\nThe WAL discipline.\n",
        );
    }

    fn write_consumer(root: &Path, fragment: bool, guarded: bool, body: &str) {
        let fragment_decl = if fragment {
            format!(
                "\n[[boot_snippet.fragment]]\nsource = \"boot/foreign.md\"{}\n",
                if guarded {
                    "\nwhen = \"installed:org.example/d\""
                } else {
                    ""
                }
            )
        } else {
            String::new()
        };
        write(
            root,
            "packages/org.example/p/v1.0.0/vibe.toml",
            &format!(
                r#"[package]
group = "org.example"
name = "p"
kind = "flow"
version = "1.0.0"

[boot_snippet]
source = "boot/main.md"
{fragment_decl}"#
            ),
        );
        write(
            root,
            "packages/org.example/p/v1.0.0/boot/main.md",
            if fragment { "# P\n" } else { body },
        );
        if fragment {
            write(root, "packages/org.example/p/v1.0.0/boot/foreign.md", body);
        }
    }

    fn run(root: &Path) -> CheckReport {
        let mut report = CheckReport::default();
        SnippetPresuppositionCheck.run(root, &CheckOptions::default(), &mut report);
        report
    }

    #[test]
    fn foreign_concept_in_main_snippet_is_one_error() {
        let project = tempdir().unwrap();
        write_owner(project.path());
        write_consumer(project.path(), false, false, "The WAL is canonical.\n");

        let report = run(project.path());
        assert_eq!(report.findings.len(), 1, "{:?}", report.findings);
        let finding = &report.findings[0];
        assert_eq!(finding.check, CheckId::SnippetPresupposition);
        assert_eq!(finding.severity, Severity::Error);
        assert_eq!(finding.line, Some(1));
        assert!(finding.message.contains("WAL"), "{finding:?}");
        assert!(finding.message.contains("org.example/d"), "{finding:?}");
    }

    #[test]
    fn foreign_concept_in_matching_installed_fragment_is_clean() {
        let project = tempdir().unwrap();
        write_owner(project.path());
        write_consumer(project.path(), true, true, "The WAL is canonical.\n");
        assert!(run(project.path()).findings.is_empty());
    }

    #[test]
    fn empty_concepts_dictionary_is_clean() {
        let project = tempdir().unwrap();
        write_consumer(project.path(), false, false, "The WAL is canonical.\n");
        assert!(run(project.path()).findings.is_empty());
    }

    /// PROP-049 §4, the dependency exemption: a declared `[requires]` on the
    /// concept's owner guarantees co-installation, so the bare mention is
    /// lawful without a fragment.
    #[test]
    fn a_declared_dependency_on_the_owner_makes_the_mention_lawful() {
        let project = tempdir().unwrap();
        write_owner(project.path());
        write(
            project.path(),
            "packages/org.example/p/v1.0.0/vibe.toml",
            r#"[package]
group = "org.example"
name = "p"
kind = "flow"
version = "1.0.0"

[boot_snippet]
source = "boot/main.md"

[requires.packages]
"flow:org.example/d" = "^1.0.0"
"#,
        );
        write(
            project.path(),
            "packages/org.example/p/v1.0.0/boot/main.md",
            "The WAL is canonical here too.\n",
        );
        assert!(run(project.path()).findings.is_empty());
    }
}
