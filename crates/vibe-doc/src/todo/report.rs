//! The queue as a person reads it — the week's report
//! (`MAINTENANCE.md` §2.2, §7).
//!
//! Two audiences, one document. The eight numbers come first, because a
//! monthly review reads the trend and nothing else; the sections follow,
//! because a weekly loop picks up to five small repairs out of them and
//! needs the rows.
//!
//! A number nobody measured prints as a dash and says why underneath. A
//! section nobody measured prints as a line saying so. Neither prints as
//! a zero, because a zero is an achievement and an unasked question is
//! not.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#OBS-MAINTENANCE-TOOLS");

use vibe_wire::generated::doc_todo::{DocTodo, ItemSeverity, SectionName, TodoSection};

/// How many rows of one section the week's report prints before it says
/// «and so on». The report is read to choose five repairs, and a hundred
/// rows of one kind hide the other kinds.
pub const ROWS_PER_SECTION: usize = 12;

/// The queue as Markdown.
pub fn render_md(queue: &DocTodo) -> String {
    let mut out = format!("# Maintenance queue — `{}`\n\n", queue.package);
    out.push_str("| metric | value |\n|---|---|\n");
    let metrics = &queue.metrics;
    let row = |out: &mut String, name: &str, value: String| {
        out.push_str(&format!("| {name} | {value} |\n"));
    };
    row(
        &mut out,
        "gaps (coverage, examples, citations)",
        metrics.gaps.to_string(),
    );
    row(
        &mut out,
        "median days since a page was read aloud",
        number(metrics.page_age_median_days),
    );
    row(
        &mut out,
        "adaptation divergences",
        number(metrics.adaptation_divergences),
    );
    row(
        &mut out,
        "coverage of obligations, per cent",
        number(metrics.coverage_percent),
    );
    row(
        &mut out,
        "style findings per 1000 words",
        rate(metrics.tics_per_k_words),
    );
    row(
        &mut out,
        "documentation debt at P1",
        metrics.debt_p1.to_string(),
    );
    row(
        &mut out,
        "journal entries owing a decision",
        number(metrics.findings_without_decision),
    );
    row(
        &mut out,
        "days since the last full reconciliation",
        number(metrics.days_since_reconcile),
    );
    out.push('\n');
    if !queue.examples_measured {
        out.push_str(
            "The documented examples did not run: they build a sandbox per fixture and \
             cost minutes. Pass `--examples` to fold their result in — until then the \
             first number is short by however many are red.\n\n",
        );
    }
    for section in &queue.sections {
        out.push_str(&render_section(section));
    }
    out
}

/// One section of the queue.
fn render_section(section: &TodoSection) -> String {
    let mut out = format!("## {}\n\n", heading(&section.name));
    if !section.measured {
        out.push_str("Not measured in this run.\n\n");
        return out;
    }
    if section.items.is_empty() {
        out.push_str("Nothing in the queue.\n\n");
        return out;
    }
    out.push_str(&format!("{} row(s).\n\n", section.items.len()));
    for entry in section.items.iter().take(ROWS_PER_SECTION) {
        out.push_str(&format!(
            "- {} `{}` — {}\n",
            word(&entry.severity),
            entry.subject,
            entry.reason
        ));
    }
    if section.items.len() > ROWS_PER_SECTION {
        out.push_str(&format!(
            "- …and {} more; the full list is the check this row came from\n",
            section.items.len() - ROWS_PER_SECTION
        ));
    }
    out.push('\n');
    out
}

/// What a section is called in the report.
fn heading(name: &SectionName) -> &'static str {
    match name {
        SectionName::Coverage => "Obligations nobody tells",
        SectionName::Examples => "Documented examples that no longer match",
        SectionName::Citations => "Citations that no longer resolve",
        SectionName::Adaptations => "Adaptations that do not mirror",
        SectionName::PageAge => "Pages owed a reading",
        SectionName::Debt => "Documentation debt",
        SectionName::Style => "What the style linter found",
        SectionName::VersionChange => "Pages a version change reaches",
    }
}

fn word(severity: &ItemSeverity) -> &'static str {
    match severity {
        ItemSeverity::Error => "ERROR  ",
        ItemSeverity::Warning => "warning",
        ItemSeverity::Info => "info   ",
    }
}

/// A measured number, or a dash.
fn number(value: Option<u32>) -> String {
    match value {
        Some(value) => value.to_string(),
        None => "— (not measured)".to_owned(),
    }
}

/// The style rate at the unit the metric table states — hundredths, put
/// back together for a person.
fn rate(value: Option<u32>) -> String {
    match value {
        Some(value) => format!("{}.{:02}", value / 100, value % 100),
        None => "— (not measured)".to_owned(),
    }
}

/// A multi-line rendering flattened into one queue row.
pub fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rate_reads_at_the_unit_the_metric_table_states() {
        assert_eq!(rate(Some(1234)), "12.34");
        assert_eq!(rate(Some(7)), "0.07");
        assert_eq!(rate(None), "— (not measured)");
    }

    #[test]
    fn an_unmeasured_number_is_never_a_zero() {
        assert_eq!(number(None), "— (not measured)");
        assert_eq!(number(Some(0)), "0");
    }

    #[test]
    fn a_section_nobody_asked_says_so() {
        let section = TodoSection {
            name: SectionName::Examples,
            measured: false,
            items: Vec::new(),
        };
        assert!(render_section(&section).contains("Not measured in this run."));
    }

    #[test]
    fn a_section_with_nothing_in_it_says_that_instead() {
        let section = TodoSection {
            name: SectionName::Citations,
            measured: true,
            items: Vec::new(),
        };
        assert!(render_section(&section).contains("Nothing in the queue."));
    }

    #[test]
    fn a_multi_line_finding_becomes_one_row() {
        assert_eq!(
            one_line("  MISSING PAGE x\n    the source\n has it"),
            "MISSING PAGE x the source has it"
        );
    }
}
