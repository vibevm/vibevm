//! Human artifact locations, kept separate from resolution and the wire report.

use vibe_wire::generated::artifacts_where_report::{ArtifactsWhereReport, DropStatus};

use crate::commands::vvm::bundle::release_source;
use crate::output::{self, language};

pub(super) fn render(ctx: &output::Context, report: &ArtifactsWhereReport, package: Option<&str>) {
    use std::io::IsTerminal;

    let width = std::io::stdout().is_terminal().then(language::viewport);
    let text = report_text(report, package, width);
    if ctx.is_quiet() {
        ctx.summary(&text);
    } else {
        ctx.block(&text);
    }
}

fn report_text(
    report: &ArtifactsWhereReport,
    package: Option<&str>,
    width: Option<usize>,
) -> String {
    let release = &report.release;
    let separator = language::heading_separator();
    let label = format!(
        "vibe releases {separator} {} {}",
        release_source::kind_label(release.source.clone()),
        release.location
    );
    let mut sections = vec![language::rule_at(
        &label,
        Some(&release.from),
        width.unwrap_or(80),
    )];
    let mut files = Vec::new();
    if let Some(dir) = &release.example_dir {
        let version = dir
            .trim_end_matches(['/', '\\'])
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or(dir);
        for file in &release.files {
            files.extend(body_lines(&format!("{version}/{file}"), width));
        }
        files.extend(body_lines(
            "a bare vibevm-<version>-<target>.zip at the top is accepted too",
            width,
        ));
    } else {
        files.extend(body_lines(
            "no release drop: releases are downloaded from the source",
            width,
        ));
    }
    sections.push(files.join("\n"));
    sections.push(match &report.packages {
        Some(drop) => {
            let mut lines = body_lines(&format!("package payloads: {drop}"), width);
            lines.extend(body_lines(
                "an input of G/N@V named F (sha256 D) is looked for at <drop>/G/N/V/F, <drop>/G/N/F, <drop>/D/F, <drop>/F",
                width,
            ));
            lines.join("\n")
        }
        None => language::warning(
            "package payloads have no drop: set [artifacts] packages = \"<dir>\" in config.toml",
        ),
    });
    if let Some(package) = package {
        sections.push(language::rule_at(
            &format!("inputs of {package}"),
            None,
            width.unwrap_or(80),
        ));
        let option_width = report
            .inputs
            .iter()
            .map(|i| cells(&i.option))
            .max()
            .unwrap_or(0);
        let file_width = report
            .inputs
            .iter()
            .map(|i| cells(i.file.as_deref().unwrap_or("<unnamed>")))
            .max()
            .unwrap_or(0);
        let mut rows = Vec::new();
        for input in &report.inputs {
            let file = input.file.as_deref().unwrap_or("<unnamed>");
            let digest: String = input.sha256.chars().take(8).collect();
            let end = if language::rule_glyph().to_string() == "-" {
                "..."
            } else {
                "…"
            };
            let status = match input.status {
                DropStatus::Present => "present",
                DropStatus::Missing => "missing",
                DropStatus::Mismatch => "mismatch",
                DropStatus::NoDrop => "no drop",
            };
            let metadata = format!("sha256 {digest}{end}  [{status}]");
            let row = format!(
                "{}{}    {}{}    {metadata}",
                input.option,
                " ".repeat(option_width - cells(&input.option)),
                file,
                " ".repeat(file_width - cells(file)),
            );
            if width.is_none_or(|w| cells(&row) + 2 <= w) {
                rows.push(format!("  {row}"));
            } else {
                rows.extend(body_lines(&input.option, width));
                rows.extend(body_lines(file, width));
                rows.extend(body_lines(&metadata, width));
            }
            if input.status != DropStatus::Present
                && let Some(first) = input.searched.first()
            {
                rows.extend(indented_lines(&format!("put it at {first}"), width, 4));
            }
        }
        if rows.is_empty() {
            rows.push("  no downloaded inputs".to_owned());
        }
        sections.push(rows.join("\n"));
    }
    sections.join("\n\n")
}

fn cells(text: &str) -> usize {
    console::measure_text_width(text)
}

// Wrap without eliding identifiers or filenames. A redirected stream has no
// viewport, so it keeps each row intact. Even an unbroken filename survives.
fn body_lines(text: &str, width: Option<usize>) -> Vec<String> {
    indented_lines(text, width, 2)
}

fn indented_lines(text: &str, width: Option<usize>, indent: usize) -> Vec<String> {
    let padding = " ".repeat(indent);
    let Some(width) = width else {
        return vec![format!("{padding}{text}")];
    };
    let available = width.saturating_sub(indent).max(1);
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if !line.is_empty() && cells(&line) + 1 + cells(word) > available {
            lines.push(format!("{padding}{line}"));
            line.clear();
        }
        if !line.is_empty() {
            line.push(' ');
        }
        for ch in word.chars() {
            if cells(&line) + cells(ch.encode_utf8(&mut [0; 4])) > available {
                lines.push(format!("{padding}{line}"));
                line.clear();
            }
            line.push(ch);
        }
    }
    if !line.is_empty() {
        lines.push(format!("{padding}{line}"));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use vibe_wire::generated::artifacts_where_report::{
        InputWhere, ReleaseSourceKind, ReleaseWhere,
    };

    #[test]
    fn artifacts_where_sections_and_input_rows() {
        let sha = format!("518674de{}", "a".repeat(56));
        let mut report = ArtifactsWhereReport {
            protocol: "vibe-artifacts-where/1".into(),
            release: ReleaseWhere {
                source: ReleaseSourceKind::Localdir,
                location: "/drop".into(),
                from: "[self] in /settings/config.toml".into(),
                example_dir: Some("/drop/v1.0.12".into()),
                files: vec![
                    "DISTRIBUTIONS.json".into(),
                    "vibevm-1.0.12-x86_64-unknown-linux-gnu.zip".into(),
                ],
            },
            packages: None,
            inputs: vec![InputWhere {
                option: "linux-x64".into(),
                step: "download".into(),
                input: "node".into(),
                file: Some("node-v26.13.1-linux-x64.tar.gz".into()),
                sha256: sha.clone(),
                url: None,
                searched: Vec::new(),
                status: DropStatus::NoDrop,
            }],
        };
        let mut second = report.inputs[0].clone();
        second.option = "linux-arm64".into();
        second.status = DropStatus::Missing;
        second.searched = vec!["/packages/node.tar.gz".into()];
        report.inputs.push(second);
        let text = report_text(&report, Some("org.vibevm.node/node"), Some(100));
        assert_eq!(
            text.lines().next().unwrap(),
            "── vibe releases · localdir /drop ────────────────────────────────── [self] in /settings/config.toml"
        );
        assert!(text.contains(&format!(
            "\n\n{}\n\n",
            language::rule_at("inputs of org.vibevm.node/node", None, 100)
        )));
        assert_eq!(text.matches("inputs of org.vibevm.node/node").count(), 1);
        assert!(text.contains(
            "\n\n  v1.0.12/DISTRIBUTIONS.json\n  v1.0.12/vibevm-1.0.12-x86_64-unknown-linux-gnu.zip"
        ));
        assert!(text.contains("sha256 518674de…  [no drop]"));
        assert!(text.contains("[missing]\n    put it at /packages/node.tar.gz"));
        assert!(!text.contains(&sha));
        let wire = serde_json::to_value(&report).unwrap();
        assert_eq!(wire["inputs"][0]["sha256"], sha);
        assert_eq!(wire["inputs"][1]["searched"][0], "/packages/node.tar.gz");
        let long_file = "node-extraordinarily-long-archive-v26.13.1-linux-x64.tar.gz";
        report.inputs[0].file = Some(long_file.into());
        report.inputs[0].option = "linux-x64-long-option-identifier".into();
        let narrow = report_text(&report, Some("node"), Some(36));
        let rows = narrow.split("inputs of node").last().unwrap();
        let joined = rows.lines().map(str::trim).collect::<String>();
        assert!(joined.contains(long_file));
        assert!(joined.contains("linux-x64-long-option-identifier"));
        assert!(joined.find(long_file).unwrap() < joined.find("sha256").unwrap());
        // Rules and the warning have their own language-owned width policy;
        // the input rows must retain their data within the narrow viewport.
        for line in rows.lines().skip(2) {
            assert!(cells(line) <= 36, "{line}");
        }
        for status in [DropStatus::Present, DropStatus::Mismatch] {
            report.inputs[0].status = status.clone();
            let text = report_text(&report, Some("node"), None);
            assert!(text.contains(match status {
                DropStatus::Present => "[present]",
                _ => "[mismatch]",
            }));
        }
        report.inputs.clear();
        report.release.example_dir = None;
        report.packages = Some("/packages".into());
        let empty = report_text(&report, Some("node"), None);
        assert!(empty.ends_with("\n\n  no downloaded inputs"));
        assert!(empty.contains("no release drop: releases are downloaded from the source"));
        assert!(empty.contains("<drop>/G/N/V/F, <drop>/G/N/F, <drop>/D/F, <drop>/F"));
        assert!(!report_text(&report, None, None).contains("inputs of"));
    }
}
