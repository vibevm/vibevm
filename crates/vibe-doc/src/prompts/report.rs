//! What a `--prompts` run says (PROP-057 `##STYLE-PROMPT-FIRST`).
//!
//! A red prompt has to be readable by a person who was not watching. The
//! report therefore carries three things a prompt check cannot do
//! without: which assert failed and with what code, the TAIL of what the
//! agent said, and the fixture it all started from. The tail is for a
//! human and never for a comparison — an agent's words are not a golden
//! output, and pretending otherwise is how this check would start
//! failing on the weather.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#STYLE-PROMPT-FIRST");

/// How much of the agent's output a report keeps.
pub const TAIL_LINES: usize = 12;

/// One assert, run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssertOutcome {
    /// The command as the page wrote it.
    pub command: String,
    /// Its exit code. Zero is the only pass.
    pub code: i32,
    /// What it said, normalised and cut.
    pub output: String,
}

impl AssertOutcome {
    pub fn passed(&self) -> bool {
        self.code == 0
    }
}

/// What became of one prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// The runner finished and every assert exited zero.
    Held,
    /// The runner finished and an assert did not.
    Broke,
    /// The runner itself could not run, or was killed.
    Failed { message: String },
    /// The prompt was not run, with the reason: an illustrative prompt
    /// with no assert, a declared skip, or a sample that did not take it.
    Skipped { reason: String },
}

impl Verdict {
    /// The four-letter tag a report line opens with.
    pub fn tag(&self) -> &'static str {
        match self {
            Verdict::Held => "ok  ",
            Verdict::Broke => "BROKE",
            Verdict::Failed { .. } => "FAIL",
            Verdict::Skipped { .. } => "skip",
        }
    }

    /// Whether this verdict fails the check.
    pub fn is_red(&self) -> bool {
        matches!(self, Verdict::Broke | Verdict::Failed { .. })
    }
}

/// One prompt's line in the report.
#[derive(Debug, Clone)]
pub struct Outcome {
    /// The page's address inside the package.
    pub page: String,
    pub id: String,
    /// The fixture the sandbox was built from.
    pub fixture: String,
    /// What the agent must have, as the page states it.
    pub needs: Option<String>,
    /// The runner's own exit code, when it ran.
    pub runner_code: Option<i32>,
    /// The tail of what the agent said — for a person, never compared.
    pub tail: String,
    pub asserts: Vec<AssertOutcome>,
    pub verdict: Verdict,
}

impl Outcome {
    /// The address a report and a `--only` filter both use.
    pub fn address(&self) -> String {
        format!("{}#{}", self.page, self.id)
    }
}

/// A whole run.
#[derive(Debug, Clone)]
pub struct Report {
    pub outcomes: Vec<Outcome>,
    /// Pages the pivot refused: they carry prompts nobody can run.
    pub unreadable: Vec<String>,
    /// The command the prompts were handed to, for the record.
    pub runner: String,
}

impl Report {
    /// How many prompts ended with each verdict.
    pub fn counts(&self) -> Counts {
        let mut c = Counts::default();
        for outcome in &self.outcomes {
            match &outcome.verdict {
                Verdict::Held => c.held += 1,
                Verdict::Broke => c.broke += 1,
                Verdict::Failed { .. } => c.failed += 1,
                Verdict::Skipped { .. } => c.skipped += 1,
            }
        }
        c
    }

    /// Green when nothing broke, nothing failed and every page read.
    pub fn ok(&self) -> bool {
        self.unreadable.is_empty() && !self.outcomes.iter().any(|o| o.verdict.is_red())
    }

    /// The human form: one line per prompt, its asserts under it, then
    /// the tail of every red one, then the summary.
    pub fn render(&self) -> String {
        let mut out = String::new();
        for outcome in &self.outcomes {
            out.push_str(&format!(
                "  {} {} [{}]{}\n",
                outcome.verdict.tag(),
                outcome.address(),
                outcome.fixture,
                match outcome.runner_code {
                    Some(code) => format!(" runner exit {code}"),
                    None => String::new(),
                }
            ));
            if let Verdict::Skipped { reason } = &outcome.verdict {
                out.push_str(&format!("       {reason}\n"));
            }
            for (index, a) in outcome.asserts.iter().enumerate() {
                out.push_str(&format!(
                    "       assert {} exit {} — `{}`\n",
                    index + 1,
                    a.code,
                    a.command
                ));
                if !a.passed() && !a.output.trim().is_empty() {
                    out.push_str(&format!("         {}\n", a.output.trim()));
                }
            }
        }
        for page in &self.unreadable {
            out.push_str(&format!("  FAIL {page} does not parse\n"));
        }
        for outcome in &self.outcomes {
            match &outcome.verdict {
                Verdict::Failed { message } => {
                    out.push_str(&format!("\n{} — {message}\n", outcome.address()));
                }
                Verdict::Broke => {
                    out.push_str(&format!(
                        "\n{} — the agent's last words:\n{}\n",
                        outcome.address(),
                        indent(&outcome.tail)
                    ));
                }
                _ => {}
            }
        }
        let c = self.counts();
        out.push_str(&format!(
            "\nprompts: {} held, {} broke, {} failed, {} skipped, {} unreadable page(s) \
             [runner: {}]\n",
            c.held,
            c.broke,
            c.failed,
            c.skipped,
            self.unreadable.len(),
            self.runner
        ));
        out
    }
}

fn indent(text: &str) -> String {
    text.lines()
        .map(|line| format!("  | {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The tally of a run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    pub held: usize,
    pub broke: usize,
    pub failed: usize,
    pub skipped: usize,
}

/// The last [`TAIL_LINES`] lines of what a runner said.
///
/// ```
/// let text = (1..=20).map(|n| format!("line {n}\n")).collect::<String>();
/// let tail = vibe_doc::prompts::report::tail(&text);
/// assert!(tail.starts_with("line 9"));
/// assert!(tail.ends_with("line 20"));
/// ```
pub fn tail(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let from = lines.len().saturating_sub(TAIL_LINES);
    lines[from..].join("\n")
}

#[cfg(test)]
mod tests;
