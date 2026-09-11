//! PROP-023 — bridge packages carry enough local provenance to make the
//! maintainer/upstream and licence boundaries visible before publication.
//!
//! This cell is deliberately source-only: it reads package manifests and
//! root documentation, never follows an upstream URL and never consults a
//! registry. Reference-backed sources are already structurally validated by
//! `vibe-core`; the checks here turn that schema into bridge-specific,
//! repair-oriented diagnostics.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-023#maintainer-model");

use std::fs;
use std::path::{Path, PathBuf};

use specmark::cell;
use vibe_core::manifest::{EmbeddedSourceDecl, Manifest};

use super::scan_local_packages;
use crate::{Check, CheckId, CheckOptions, CheckReport};

/// The [`CheckId::BridgeProvenance`] cell.
#[cell(seam = "Check", variant = "bridge-provenance")]
pub struct BridgeProvenanceCheck;

impl Check for BridgeProvenanceCheck {
    fn id(&self) -> CheckId {
        CheckId::BridgeProvenance
    }

    fn run(&self, project_root: &Path, _opts: &CheckOptions, report: &mut CheckReport) {
        for (package_root, source_label) in scan_local_packages(project_root) {
            let manifest_path = package_root.join(Manifest::FILENAME);
            let manifest = match Manifest::read(&manifest_path) {
                Ok(manifest) => manifest,
                Err(_) => continue, // ManifestValidity owns malformed manifests.
            };
            let Some(package) = manifest.package.as_ref() else {
                continue;
            };
            if !package.bridge {
                continue;
            }

            let finding_path = manifest_path
                .strip_prefix(project_root)
                .ok()
                .map(portable_path);
            let coordinate = format!("{}/{}@{}", package.group, package.name, package.version);

            if package.describes.is_none() {
                report.err(
                    CheckId::BridgeProvenance,
                    finding_path.clone(),
                    None,
                    format!(
                        "[{source_label}] bridge `{coordinate}` does not identify the upstream artefact. Fix: add `[package].describes = \"pkg:<type>/<namespace>/<name>@<version>\"`."
                    ),
                );
            }

            match package.license.as_deref() {
                None | Some("") => report.err(
                    CheckId::BridgeProvenance,
                    finding_path.clone(),
                    None,
                    format!(
                        "[{source_label}] bridge `{coordinate}` has no package licence. Fix: set `[package].license` to the SPDX expression for the maintainer-authored bridge payload (for VibeVM bridge packages: `UPL-1.0`)."
                    ),
                ),
                Some(expression) if !is_spdx_expression(expression) => report.err(
                    CheckId::BridgeProvenance,
                    finding_path.clone(),
                    None,
                    format!(
                        "[{source_label}] bridge `{coordinate}` has non-SPDX package licence `{expression}`. Fix: write a valid SPDX expression in `[package].license` (for VibeVM bridge packages: `UPL-1.0`)."
                    ),
                ),
                Some(_) => {}
            }

            if !has_root_licence_file(&package_root) {
                report.err(
                    CheckId::BridgeProvenance,
                    finding_path.clone(),
                    None,
                    format!(
                        "[{source_label}] bridge `{coordinate}` has no root `LICENSE`/`LICENCE` file in its shippable package tree. Fix: add the full licence text as `LICENSE`, `LICENSE.md`, `LICENSE.txt`, `LICENCE`, `LICENCE.md`, or `LICENCE.txt` beside `vibe.toml`."
                    ),
                );
            }

            for source in &manifest.embedded_sources {
                check_embedded_source(
                    source,
                    &source_label,
                    &coordinate,
                    finding_path.clone(),
                    report,
                );
            }

            // A reference-backed bridge carries machine-readable provenance.
            // Legacy vendored/submodule bridges do not, so retain the existing
            // human-readable Upstream-section convention as a warning.
            if manifest.embedded_sources.is_empty() && !has_upstream_section(&package_root) {
                report.warn(
                    CheckId::BridgeProvenance,
                    finding_path,
                    None,
                    format!(
                        "[{source_label}] bridge `{coordinate}` has no machine-readable `[[embedded_source]]` and no root Markdown `Upstream` section. Fix: document the original repository under an `## Upstream` heading (or use an immutable `[[embedded_source]]` reference)."
                    ),
                );
            }
        }
    }
}

fn check_embedded_source(
    source: &EmbeddedSourceDecl,
    source_label: &str,
    coordinate: &str,
    finding_path: Option<PathBuf>,
    report: &mut CheckReport,
) {
    if !is_spdx_expression(&source.upstream_license) {
        report.err(
            CheckId::BridgeProvenance,
            finding_path.clone(),
            None,
            format!(
                "[{source_label}] bridge `{coordinate}` embedded source `{}` has non-SPDX upstream licence `{}`. Fix: set `upstream_license` to the upstream project's SPDX expression.",
                source.name, source.upstream_license
            ),
        );
    }
    if source.license_path.as_os_str().is_empty() {
        report.err(
            CheckId::BridgeProvenance,
            finding_path.clone(),
            None,
            format!(
                "[{source_label}] bridge `{coordinate}` embedded source `{}` has no licence path. Fix: set `license_path` to the licence file inside the authenticated upstream tree.",
                source.name
            ),
        );
    }
    if !source.license_url.starts_with("https://")
        || !source
            .license_url
            .split('/')
            .any(|segment| segment == source.commit)
    {
        report.err(
            CheckId::BridgeProvenance,
            finding_path,
            None,
            format!(
                "[{source_label}] bridge `{coordinate}` embedded source `{}` has no immutable licence URL. Fix: set `license_url` to a public HTTPS URL containing the exact commit `{}` as a path segment.",
                source.name, source.commit
            ),
        );
    }
}

/// SPDX expression grammar without legal interpretation or compatibility
/// judgement. Identifiers remain open-ended (as SPDX permits LicenseRef-*),
/// while prose and malformed boolean expressions are rejected.
fn is_spdx_expression(value: &str) -> bool {
    if value.is_empty() || value.trim() != value || !value.is_ascii() {
        return false;
    }
    let tokens = tokenize_spdx(value);
    if tokens.is_empty() {
        return false;
    }
    let mut parser = SpdxParser { tokens, cursor: 0 };
    parser.expression() && parser.cursor == parser.tokens.len()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SpdxToken<'a> {
    Identifier(&'a str),
    And,
    Or,
    With,
    Open,
    Close,
}

fn tokenize_spdx(value: &str) -> Vec<SpdxToken<'_>> {
    let mut tokens = Vec::new();
    let bytes = value.as_bytes();
    let mut cursor = 0;
    while cursor < bytes.len() {
        match bytes[cursor] {
            b' ' | b'\t' => cursor += 1,
            b'(' => {
                tokens.push(SpdxToken::Open);
                cursor += 1;
            }
            b')' => {
                tokens.push(SpdxToken::Close);
                cursor += 1;
            }
            _ => {
                let start = cursor;
                while cursor < bytes.len() && !matches!(bytes[cursor], b' ' | b'\t' | b'(' | b')') {
                    cursor += 1;
                }
                let word = &value[start..cursor];
                let token = match word {
                    "AND" => SpdxToken::And,
                    "OR" => SpdxToken::Or,
                    "WITH" => SpdxToken::With,
                    _ if valid_spdx_identifier(word) => SpdxToken::Identifier(word),
                    _ => return Vec::new(),
                };
                tokens.push(token);
            }
        }
    }
    tokens
}

fn valid_spdx_identifier(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'+' | b':'))
}

struct SpdxParser<'a> {
    tokens: Vec<SpdxToken<'a>>,
    cursor: usize,
}

impl SpdxParser<'_> {
    fn expression(&mut self) -> bool {
        if !self.term() {
            return false;
        }
        while matches!(self.peek(), Some(SpdxToken::And | SpdxToken::Or)) {
            self.cursor += 1;
            if !self.term() {
                return false;
            }
        }
        true
    }

    fn term(&mut self) -> bool {
        match self.peek() {
            Some(SpdxToken::Identifier(_)) => self.cursor += 1,
            Some(SpdxToken::Open) => {
                self.cursor += 1;
                if !self.expression() || self.peek() != Some(SpdxToken::Close) {
                    return false;
                }
                self.cursor += 1;
            }
            _ => return false,
        }
        if self.peek() == Some(SpdxToken::With) {
            self.cursor += 1;
            if !matches!(self.peek(), Some(SpdxToken::Identifier(_))) {
                return false;
            }
            self.cursor += 1;
        }
        true
    }

    fn peek(&self) -> Option<SpdxToken<'_>> {
        self.tokens.get(self.cursor).copied()
    }
}

fn has_root_licence_file(package_root: &Path) -> bool {
    let Ok(entries) = fs::read_dir(package_root) else {
        return false;
    };
    entries.filter_map(Result::ok).any(|entry| {
        let name = entry.file_name().to_string_lossy().to_ascii_uppercase();
        matches!(
            name.as_str(),
            "LICENSE" | "LICENSE.MD" | "LICENSE.TXT" | "LICENCE" | "LICENCE.MD" | "LICENCE.TXT"
        ) && entry.file_type().is_ok_and(|kind| kind.is_file())
    })
}

fn has_upstream_section(package_root: &Path) -> bool {
    let Ok(entries) = fs::read_dir(package_root) else {
        return false;
    };
    entries.filter_map(Result::ok).any(|entry| {
        if !entry.file_type().is_ok_and(|kind| kind.is_file()) {
            return false;
        }
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        if name == "upstream.md" || name == "upstream.markdown" {
            return true;
        }
        if !name.ends_with(".md") && !name.ends_with(".markdown") {
            return false;
        }
        fs::read_to_string(entry.path())
            .ok()
            .is_some_and(|text| text.lines().any(is_upstream_heading))
    })
}

fn is_upstream_heading(line: &str) -> bool {
    let trimmed = line.trim();
    let Some(after_hashes) = trimmed.strip_prefix('#') else {
        return false;
    };
    let heading = after_hashes.trim_start_matches('#').trim();
    heading.eq_ignore_ascii_case("upstream")
        || heading
            .strip_prefix("Upstream ")
            .or_else(|| heading.strip_prefix("upstream "))
            .is_some_and(|tail| tail.starts_with(['—', '-', ':']))
}

fn portable_path(path: &Path) -> PathBuf {
    PathBuf::from(path.display().to_string().replace('\\', "/"))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use tempfile::tempdir;

    use super::{BridgeProvenanceCheck, is_spdx_expression};
    use crate::test_support::{opts, write_minimal_project};
    use crate::{Check, CheckId, CheckReport, Severity};

    const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";
    const TREE_HASH: &str =
        "sha256-tree/1:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn findings(project: &Path) -> Vec<crate::Finding> {
        let mut report = CheckReport::default();
        BridgeProvenanceCheck.run(project, &opts(), &mut report);
        report.findings
    }

    fn write_bridge(project: &Path, tail: &str, with_licence_file: bool) {
        let package = project
            .join(vibe_core::layout::current_packages_root())
            .join("org.example")
            .join("bridge")
            .join("v1.0.0");
        fs::create_dir_all(&package).unwrap();
        fs::write(
            package.join("vibe.toml"),
            format!(
                "[package]\ngroup = \"org.example\"\nname = \"bridge\"\nkind = \"feat\"\nversion = \"1.0.0\"\nepoch = 1\nbridge = true\n{tail}"
            ),
        )
        .unwrap();
        if with_licence_file {
            fs::write(package.join("LICENSE.md"), "bridge licence\n").unwrap();
        }
    }

    #[test]
    fn missing_describes_has_an_exact_repair() {
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        write_bridge(project.path(), "license = \"UPL-1.0\"\n", true);
        let hits = findings(project.path());
        assert!(
            hits.iter().any(|finding| {
                finding.check == CheckId::BridgeProvenance
                    && finding.severity == Severity::Error
                    && finding.message.contains("[package].describes")
                    && finding
                        .message
                        .contains("pkg:<type>/<namespace>/<name>@<version>")
            }),
            "got: {hits:?}"
        );
    }

    #[test]
    fn missing_package_licence_and_root_file_are_separate_errors() {
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        write_bridge(
            project.path(),
            "describes = \"pkg:github/example/upstream@1.0.0\"\n",
            false,
        );
        let hits = findings(project.path());
        assert!(
            hits.iter().any(|finding| {
                finding.severity == Severity::Error
                    && finding.message.contains("has no package licence")
                    && finding.message.contains("UPL-1.0")
            }),
            "got: {hits:?}"
        );
        assert!(
            hits.iter().any(|finding| {
                finding.severity == Severity::Error
                    && finding.message.contains("no root `LICENSE`/`LICENCE` file")
            }),
            "got: {hits:?}"
        );
    }

    #[test]
    fn legacy_bridge_without_upstream_section_warns_and_documented_one_is_green() {
        let project = tempdir().unwrap();
        write_minimal_project(project.path());
        write_bridge(
            project.path(),
            "license = \"MIT\"\ndescribes = \"pkg:github/example/upstream@1.0.0\"\n",
            true,
        );
        let hits = findings(project.path());
        assert_eq!(hits.len(), 1, "got: {hits:?}");
        assert_eq!(hits[0].severity, Severity::Warning);
        assert!(hits[0].message.contains("## Upstream"));

        let package = project
            .path()
            .join(vibe_core::layout::current_packages_root())
            .join("org.example/bridge/v1.0.0");
        fs::write(
            package.join("README.md"),
            "# Bridge\n\n## Upstream\n\nhttps://github.com/example/upstream\n",
        )
        .unwrap();
        assert!(findings(project.path()).is_empty());
    }

    #[test]
    fn reference_backed_shapes_need_no_copied_upstream_or_markdown_section() {
        for (name, describes, path) in [
            ("spec-kit", "pkg:github/github/spec-kit@1.0.6", ".specify"),
            (
                "matt-pocock-skills",
                "pkg:github/mattpocock/skills@1.2.3",
                "skills/engineering/codebase-design",
            ),
        ] {
            let project = tempdir().unwrap();
            write_minimal_project(project.path());
            let package = project
                .path()
                .join(vibe_core::layout::current_packages_root())
                .join("org.example")
                .join(name)
                .join("v1.0.0");
            fs::create_dir_all(&package).unwrap();
            fs::write(package.join("LICENSE"), "UPL bridge licence\n").unwrap();
            fs::write(
                package.join("vibe.toml"),
                format!(
                    r#"[package]
group = "org.example"
name = "{name}"
kind = "feat"
version = "1.0.0"
epoch = 1
bridge = true
license = "UPL-1.0"
describes = "{describes}"

[[embedded_source]]
name = "upstream"
kind = "git"
url = "https://github.com/example/upstream.git"
commit = "{COMMIT}"
content_hash = "{TREE_HASH}"
ref_hint = "refs/tags/v1.0.0"
upstream_license = "MIT"
license_path = "LICENSE"
license_url = "https://github.com/example/upstream/blob/{COMMIT}/LICENSE"

[[skill]]
name = "{name}"
source = "upstream"
path = "{path}"
"#
                ),
            )
            .unwrap();
            let hits = findings(project.path());
            assert!(hits.is_empty(), "{name}: {hits:?}");
        }
    }

    #[test]
    fn spdx_expression_parser_accepts_composition_and_rejects_prose() {
        assert!(is_spdx_expression("MIT"));
        assert!(is_spdx_expression("UPL-1.0"));
        assert!(is_spdx_expression("MIT OR (Apache-2.0 AND BSD-3-Clause)"));
        assert!(is_spdx_expression(
            "GPL-2.0-only WITH Classpath-exception-2.0"
        ));
        assert!(!is_spdx_expression("see the LICENSE file"));
        assert!(!is_spdx_expression("MIT OR"));
        assert!(!is_spdx_expression("(MIT"));
    }
}
