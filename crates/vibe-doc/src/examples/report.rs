//! What a run of the example checks says, and how it says it.
//!
//! A red example has to be actionable from the panel line alone: which
//! page, which id, which fixture, and the first place the two streams
//! part. A report that only counts failures sends its reader back to run
//! the command by hand, which is the work the runner exists to do once.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#OBS-EXAMPLES-GOLDEN");

use std::path::PathBuf;

use crate::pages::UnreadablePage;

/// What became of one example.
#[derive(Debug, Clone)]
pub enum Verdict {
    /// The captured output equals the golden, after the declared
    /// normalisation.
    Match,
    /// `--accept` recorded a capture on the page. `filled` distinguishes
    /// a first capture from a deliberate re-blessing.
    Accepted { filled: bool },
    /// The capture and the golden differ. This is red, always.
    Differ { diff: String },
    /// The example is declared as not captured yet, with its reason. Not
    /// red: an honest skip beats an empty golden pretending to assert
    /// silence.
    Skipped { reason: String },
    /// The runner could not get as far as a comparison — a fixture that
    /// would not build, a command it will not run.
    Failed { message: String },
}

impl Verdict {
    /// The four-letter tag a report line opens with.
    pub fn tag(&self) -> &'static str {
        match self {
            Verdict::Match => "ok  ",
            Verdict::Accepted { .. } => "took",
            Verdict::Differ { .. } => "DIFF",
            Verdict::Skipped { .. } => "skip",
            Verdict::Failed { .. } => "FAIL",
        }
    }

    /// Whether this verdict fails the check.
    pub fn is_red(&self) -> bool {
        matches!(self, Verdict::Differ { .. } | Verdict::Failed { .. })
    }
}

/// One `--json` document met on an example's stdout, and what the
/// fixture's schema map had to say about it.
#[derive(Debug, Clone)]
pub struct JsonVerdict {
    /// The document's `command` field, its identity in the schema map.
    pub command: String,
    /// The schema it was checked against, if the fixture named one.
    pub schema: Option<String>,
    /// Empty when valid; a document with no schema is UNCHECKED and says
    /// so, never «passed».
    pub violations: Vec<String>,
}

/// One example's line in the report.
#[derive(Debug, Clone)]
pub struct Outcome {
    /// The page's address inside the package.
    pub page: String,
    pub id: String,
    pub fixture: String,
    pub run: String,
    pub verdict: Verdict,
    pub json: Vec<JsonVerdict>,
}

/// A whole run.
#[derive(Debug, Clone)]
pub struct Report {
    pub outcomes: Vec<Outcome>,
    /// Pages that would not parse. They carry examples nobody can run, so
    /// they are red in their own right.
    pub unreadable: Vec<UnreadablePage>,
    pub sandbox_root: PathBuf,
}

impl Report {
    /// How many examples ended with each verdict.
    pub fn counts(&self) -> Counts {
        let mut c = Counts::default();
        for outcome in &self.outcomes {
            match &outcome.verdict {
                Verdict::Match => c.matched += 1,
                Verdict::Accepted { filled: true } => c.captured += 1,
                Verdict::Accepted { filled: false } => c.reblessed += 1,
                Verdict::Differ { .. } => c.differ += 1,
                Verdict::Skipped { .. } => c.skipped += 1,
                Verdict::Failed { .. } => c.failed += 1,
            }
        }
        c
    }

    /// Green when nothing diverged, nothing failed and every page read.
    pub fn ok(&self) -> bool {
        self.unreadable.is_empty() && !self.outcomes.iter().any(|o| o.verdict.is_red())
    }

    /// The human form: one line per example, then the detail of every red
    /// one, then the summary.
    pub fn render(&self) -> String {
        let mut out = String::new();
        for outcome in &self.outcomes {
            out.push_str(&format!(
                "  {} {}#{} [{}]\n",
                outcome.verdict.tag(),
                outcome.page,
                outcome.id,
                outcome.fixture
            ));
            if let Verdict::Skipped { reason } = &outcome.verdict {
                out.push_str(&format!("       {reason}\n"));
            }
            for doc in &outcome.json {
                out.push_str(&format!("       {}\n", render_json(doc)));
            }
        }
        for page in &self.unreadable {
            out.push_str(&format!("  FAIL {} does not parse\n", page.rel));
            out.push_str(&format!("       {}\n", page.message));
        }
        for outcome in &self.outcomes {
            match &outcome.verdict {
                Verdict::Differ { diff } => {
                    out.push_str(&format!(
                        "\n{}#{} — `{}`\n{diff}",
                        outcome.page, outcome.id, outcome.run
                    ));
                }
                Verdict::Failed { message } => {
                    out.push_str(&format!(
                        "\n{}#{} — `{}`\n  {message}\n",
                        outcome.page, outcome.id, outcome.run
                    ));
                }
                _ => {}
            }
        }
        let c = self.counts();
        out.push_str(&format!(
            "\nexamples: {} matched, {} captured, {} re-blessed, {} differ, {} failed, \
             {} skipped, {} unreadable page(s)\n",
            c.matched,
            c.captured,
            c.reblessed,
            c.differ,
            c.failed,
            c.skipped,
            self.unreadable.len()
        ));
        out
    }
}

fn render_json(doc: &JsonVerdict) -> String {
    match (&doc.schema, doc.violations.is_empty()) {
        (None, _) => format!(
            "`{}`: no schema declared — UNCHECKED, not passed",
            doc.command
        ),
        (Some(schema), true) => format!("`{}`: valid against {schema}", doc.command),
        (Some(schema), false) => format!(
            "`{}`: INVALID against {schema} — {}",
            doc.command,
            doc.violations.join("; ")
        ),
    }
}

/// The tally of a run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    pub matched: usize,
    pub captured: usize,
    pub reblessed: usize,
    pub differ: usize,
    pub failed: usize,
    pub skipped: usize,
}

/// A line diff of two streams: the common head and tail are trimmed, and
/// what is left is shown with `-` for the golden and `+` for the capture.
/// Bounded, because a page whose whole output changed is a one-line story
/// («it all changed»), not forty screens of one.
pub fn diff(label: &str, expected: &str, actual: &str) -> String {
    let want: Vec<&str> = expected.split('\n').collect();
    let got: Vec<&str> = actual.split('\n').collect();
    let mut head = 0usize;
    while head < want.len() && head < got.len() && want[head] == got[head] {
        head += 1;
    }
    let mut tail = 0usize;
    while tail < want.len() - head
        && tail < got.len() - head
        && want[want.len() - 1 - tail] == got[got.len() - 1 - tail]
    {
        tail += 1;
    }
    let mut out = format!("  {label}: first difference at line {}\n", head + 1);
    for line in want[head..want.len() - tail].iter().take(12) {
        out.push_str(&format!("  - {line}\n"));
    }
    if want.len() - tail - head > 12 {
        out.push_str(&format!(
            "  - … {} more golden line(s)\n",
            want.len() - tail - head - 12
        ));
    }
    for line in got[head..got.len() - tail].iter().take(12) {
        out.push_str(&format!("  + {line}\n"));
    }
    if got.len() - tail - head > 12 {
        out.push_str(&format!(
            "  + … {} more captured line(s)\n",
            got.len() - tail - head - 12
        ));
    }
    out
}

#[cfg(test)]
mod tests;
