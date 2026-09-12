//! One section per source of queue rows (`MAINTENANCE.md` §3, §5).
//!
//! Each function here answers one question and says whether it asked it.
//! The pattern repeats for a reason: a source that cannot be read is a
//! section marked unmeasured with no items, never a section with no items
//! — the two print the same emptiness and mean opposite things.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#OBS-MAINTENANCE-TOOLS");

use std::path::Path;

use vibe_wire::generated::doc_todo::{ItemSeverity, SectionName, TodoItem, TodoSection};

use super::{EDITS_BEFORE_A_READING, Inputs, PAGE_AGE_LIMIT_DAYS, backlog, reviews, unmeasured};
use crate::error::Result;
use crate::pages::PageSet;
use crate::{citations, coverage, examples, style, surface, translations};

/// A section plus whatever the metric table wants out of it.
pub(super) struct Measured {
    pub(super) section: TodoSection,
    pub(super) percent: Option<u32>,
}

/// The adaptations answer, whose metric is a count and not a percentage.
pub(super) struct Adaptations {
    pub(super) section: TodoSection,
    pub(super) divergences: Option<u32>,
}

/// The page-age answer, whose two metrics both come from `reviews.toml`.
pub(super) struct Ages {
    pub(super) section: TodoSection,
    pub(super) median_days: Option<u32>,
    pub(super) days_since_reconcile: Option<u32>,
}

/// The debt answer, whose metric is the P1 count alone.
pub(super) struct Debt {
    pub(super) section: TodoSection,
    pub(super) p1: u32,
}

/// The style answer, which turns into a rate only once the corpus has
/// been counted.
pub(super) struct Style {
    pub(super) section: TodoSection,
    findings: Option<usize>,
}

impl Style {
    /// Findings per hundred thousand words of prose — the metric table's
    /// rate at a unit that is an exact integer.
    pub(super) fn tics_per_100k(&self, set: &PageSet) -> Option<u32> {
        let findings = self.findings?;
        let words = words(set);
        if words == 0 {
            return None;
        }
        Some(((findings as u64 * 100_000) / words as u64) as u32)
    }
}

fn item(subject: &str, reason: String, severity: ItemSeverity) -> TodoItem {
    TodoItem {
        subject: subject.to_owned(),
        reason,
        severity,
    }
}

/// Obligations nobody tells (`##OBS-COVERAGE-GATE`).
pub(super) fn coverage(package_dir: &Path, inputs: &Inputs) -> Result<Measured> {
    let Some(root) = &inputs.corpus_root else {
        return Ok(Measured {
            section: unmeasured(SectionName::Coverage),
            percent: None,
        });
    };
    let report = coverage::check(
        package_dir,
        &inputs.coordinate,
        &inputs.sources,
        root,
        inputs.obligations.clone(),
        inputs.min,
    )?;
    let items = report
        .gaps
        .iter()
        .map(|gap| {
            item(
                &format!("{} ({})", gap.address, gap.audience.as_str()),
                format!("{} — {}:{}", gap.reason.as_str(), gap.path, gap.line),
                ItemSeverity::Error,
            )
        })
        .collect();
    Ok(Measured {
        percent: Some(u32::from(report.percent())),
        section: TodoSection {
            name: SectionName::Coverage,
            measured: true,
            items,
        },
    })
}

/// Documented commands whose output no longer matches (`##INV-EXAMPLES-RUN`).
pub(super) fn examples(package_dir: &Path, inputs: &Inputs) -> Result<Measured> {
    let Some(runner) = &inputs.examples else {
        return Ok(Measured {
            section: unmeasured(SectionName::Examples),
            percent: None,
        });
    };
    let report = examples::check(package_dir, runner, &examples::Options::default())?;
    // Red and red only. A skipped example is an honest «not captured
    // yet» and a taken capture is not a defect; putting either in the
    // queue would make the queue's first number untrue.
    let items = report
        .outcomes
        .iter()
        .filter(|outcome| outcome.verdict.is_red())
        .map(|outcome| {
            item(
                &format!("{}#{}", outcome.page, outcome.id),
                format!("`{}` — {}", outcome.run, outcome.verdict.tag().trim()),
                ItemSeverity::Error,
            )
        })
        .collect();
    Ok(Measured {
        percent: None,
        section: TodoSection {
            name: SectionName::Examples,
            measured: true,
            items,
        },
    })
}

/// Cited addresses that no longer resolve (`##OBS-RULE-EDGE-UNPINNED`).
pub(super) fn citations(package_dir: &Path, inputs: &Inputs) -> Result<Measured> {
    let report = citations::check(package_dir, &inputs.coordinate, &inputs.sources)?;
    let items = report
        .unresolved
        .iter()
        .map(|u| {
            item(
                &u.uri,
                format!("{}:{} — {}", u.page, u.line, u.reason),
                ItemSeverity::Error,
            )
        })
        .collect();
    Ok(Measured {
        percent: None,
        section: TodoSection {
            name: SectionName::Citations,
            measured: true,
            items,
        },
    })
}

/// An adaptation that no longer mirrors its source (`##LOC-MIRROR`).
///
/// Structure and nothing else. The row the metric table used to hold was
/// a count of revisions behind, and the seventh edition removed it: «how
/// far behind» needs a history this project does not keep.
pub(super) fn adaptations(package_dir: &Path, inputs: &Inputs) -> Result<Adaptations> {
    let Ok(report) = translations::check(package_dir, &inputs.sources) else {
        // An adaptation whose source this machine cannot reach is a
        // question that was not asked, not an adaptation that mirrors.
        return Ok(Adaptations {
            section: unmeasured(SectionName::Adaptations),
            divergences: None,
        });
    };
    if report.adapts.is_none() {
        return Ok(Adaptations {
            section: unmeasured(SectionName::Adaptations),
            divergences: None,
        });
    }
    let adapts = report.adapts.clone().unwrap_or_default();
    let items: Vec<TodoItem> = report
        .problems
        .iter()
        .map(|problem| {
            item(
                problem.page(),
                super::report::one_line(&problem.render(&adapts)),
                ItemSeverity::Error,
            )
        })
        .collect();
    Ok(Adaptations {
        divergences: Some(items.len() as u32),
        section: TodoSection {
            name: SectionName::Adaptations,
            measured: true,
            items,
        },
    })
}

/// Pages nobody has read aloud lately (`MAINTENANCE.md` §2.2, §6).
pub(super) fn page_age(package_dir: &Path, set: &PageSet, inputs: &Inputs) -> Result<Ages> {
    let Some(reviews) = reviews::read(package_dir)? else {
        // No readings recorded at all. Every page is owed one, which is a
        // queue and not a median.
        let items = set
            .pages
            .iter()
            .map(|page| {
                item(
                    &page.rel,
                    "never read aloud — this package records no readings".to_owned(),
                    ItemSeverity::Warning,
                )
            })
            .collect();
        return Ok(Ages {
            median_days: None,
            days_since_reconcile: None,
            section: TodoSection {
                name: SectionName::PageAge,
                measured: true,
                items,
            },
        });
    };
    let ages = reviews::ages(&reviews, set, inputs.corpus_root.as_deref(), package_dir);
    let mut items = Vec::new();
    let mut days: Vec<i64> = Vec::new();
    for age in &ages {
        match age.days(inputs.today) {
            None => items.push(item(
                &age.page,
                "never read aloud".to_owned(),
                ItemSeverity::Warning,
            )),
            Some(since) => {
                days.push(since);
                if since > PAGE_AGE_LIMIT_DAYS {
                    items.push(item(
                        &age.page,
                        format!(
                            "{since} days since {} read it aloud",
                            if age.by.is_empty() {
                                "somebody"
                            } else {
                                &age.by
                            }
                        ),
                        ItemSeverity::Warning,
                    ));
                    continue;
                }
                if age.edits.is_some_and(|n| n >= EDITS_BEFORE_A_READING) {
                    items.push(item(
                        &age.page,
                        format!(
                            "{} edit(s) since it was read aloud — the rule of five",
                            age.edits.unwrap_or_default()
                        ),
                        ItemSeverity::Warning,
                    ));
                }
            }
        }
    }
    Ok(Ages {
        median_days: median(&mut days),
        days_since_reconcile: reviews::days_since_reconcile(&reviews, inputs.today),
        section: TodoSection {
            name: SectionName::PageAge,
            measured: true,
            items,
        },
    })
}

/// The documentation debt of the host (`MAINTENANCE.md` §2.1).
pub(super) fn debt(inputs: &Inputs) -> Result<Debt> {
    let Some(path) = &inputs.backlog else {
        return Ok(Debt {
            section: unmeasured(SectionName::Debt),
            p1: 0,
        });
    };
    if !path.is_file() {
        return Ok(Debt {
            section: unmeasured(SectionName::Debt),
            p1: 0,
        });
    }
    let lines = backlog::read(path)?;
    let p1 = lines
        .iter()
        .filter(|line| line.severity.as_deref() == Some("P1"))
        .count() as u32;
    let items = lines
        .iter()
        .map(|line| {
            item(
                &format!("{}:{}", path.display(), line.line),
                line.text.clone(),
                match line.severity.as_deref() {
                    Some("P1") => ItemSeverity::Error,
                    Some("P2") => ItemSeverity::Warning,
                    _ => ItemSeverity::Info,
                },
            )
        })
        .collect();
    Ok(Debt {
        p1,
        section: TodoSection {
            name: SectionName::Debt,
            measured: true,
            items,
        },
    })
}

/// What the style law's mechanical half found (`##STYLE-LINT`).
///
/// Statistics and not a copy of the linter's report: the queue is read in
/// a weekly loop to decide what to do, and one row per rule answers that.
/// The findings themselves are `vibe doc check --style`, which is one
/// command away.
pub(super) fn style(package_dir: &Path, inputs: &Inputs) -> Style {
    let Ok(report) = style::check(package_dir, inputs.min) else {
        // A package with no banned list for its own language cannot be
        // linted, and a zero here would mean «nothing to find one with».
        return Style {
            section: unmeasured(SectionName::Style),
            findings: None,
        };
    };
    let items = report
        .by_rule()
        .into_iter()
        .map(|(rule, errors, warnings)| {
            item(
                rule.as_str(),
                format!("{errors} error(s), {warnings} warning(s)"),
                if errors > 0 {
                    ItemSeverity::Error
                } else {
                    ItemSeverity::Warning
                },
            )
        })
        .collect();
    Style {
        findings: Some(report.findings.len()),
        section: TodoSection {
            name: SectionName::Style,
            measured: true,
            items,
        },
    }
}

/// The pages a declared version change reaches (`MAINTENANCE.md` §2.5).
///
/// It appears only when the package holds a snapshot of some version
/// other than the one the product declares — which is the single event
/// this project computes a difference by. With no such snapshot there is
/// nothing to compare, and the section says it was not measured rather
/// than reporting a version change of none.
pub(super) fn version_change(package_dir: &Path, inputs: &Inputs) -> Result<TodoSection> {
    let recorded = surface::recorded(package_dir)?;
    let Some(other) = recorded
        .iter()
        .find(|version| *version != &inputs.product_version)
    else {
        return Ok(unmeasured(SectionName::VersionChange));
    };
    let (Some(env), Some(root)) = (&inputs.surface, &inputs.corpus_root) else {
        return Ok(unmeasured(SectionName::VersionChange));
    };
    let old = surface::read(&surface::path_for(package_dir, other))?;
    let new = surface::record(&inputs.product_version, &inputs.obligations, env)?;
    let document = surface::diff::diff(
        &old,
        &new,
        package_dir,
        &inputs.coordinate,
        &inputs.sources,
        root,
    )?;
    let mut items: Vec<TodoItem> = document
        .pages
        .iter()
        .map(|page| {
            item(
                &page.page,
                format!("{}: {}", other, page.reasons.join("; ")),
                ItemSeverity::Error,
            )
        })
        .collect();
    items.extend(document.needs_pages.iter().map(|unplaced| {
        item(
            &unplaced.subject,
            format!("{other}: {}", unplaced.reason),
            ItemSeverity::Error,
        )
    }));
    Ok(TodoSection {
        name: SectionName::VersionChange,
        measured: true,
        items,
    })
}

/// The middle value, or `None` when there is nothing to take a middle of.
fn median(days: &mut [i64]) -> Option<u32> {
    if days.is_empty() {
        return None;
    }
    days.sort_unstable();
    let middle = days[days.len() / 2];
    Some(middle.max(0) as u32)
}

/// How many words of prose the package carries, counted the way the
/// linter reads prose so the rate and the findings are over one corpus.
fn words(set: &PageSet) -> usize {
    set.pages
        .iter()
        .map(|page| {
            let nodes = style::prose::nodes(&page.doc);
            style::prose::joined(&nodes).split_whitespace().count()
        })
        .sum()
}
