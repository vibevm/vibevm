//! The accumulated verdict, and how it prints itself.
//!
//! The last thing every run prints is [`NOT_CHECKED`] — the list of questions
//! this tool does not answer. It is not a disclaimer: it is the review, and it
//! is printed last so it is the part still on screen when the run ends.

#[derive(Default)]
pub(crate) struct Report {
    checks: Vec<(String, bool, String)>,
    surfaced: Vec<String>,
}

impl Report {
    pub(super) fn ok(&mut self, name: &str, detail: impl Into<String>) {
        self.checks.push((name.into(), true, detail.into()));
    }
    pub(super) fn fail(&mut self, name: &str, detail: impl Into<String>) {
        self.checks.push((name.into(), false, detail.into()));
    }
    pub(super) fn note(&mut self, line: impl Into<String>) {
        self.surfaced.push(line.into());
    }
    pub fn failed(&self) -> bool {
        self.checks.iter().any(|(_, ok, _)| !ok)
    }
    /// Print the report, ending with what it did not check.
    pub(crate) fn emit(&self) {
        println!("{}", "=".repeat(72));
        for (name, ok, detail) in &self.checks {
            println!(
                "  {}  {name:<22} {detail}",
                if *ok { "PASS" } else { "FAIL" }
            );
        }
        if !self.surfaced.is_empty() {
            println!("\n  SURFACED (not judged):");
            for line in &self.surfaced {
                println!("    {line}");
            }
        }
        println!("\n  THIS TOOL DID NOT CHECK:");
        print!("{NOT_CHECKED}");
        println!("{}", "=".repeat(72));
        println!(
            "  {}",
            if self.failed() {
                "MECHANICAL CHECKS FAILED"
            } else {
                "mechanical checks clean -- now read the diff"
            }
        );
    }

    #[cfg(test)]
    pub(crate) fn caught(&self) -> Vec<String> {
        self.checks
            .iter()
            .filter(|(_, ok, _)| !ok)
            .map(|(n, _, _)| n.clone())
            .collect()
    }
}

const NOT_CHECKED: &str =
    "  - whether a split preserved SENSE (words survive; meaning is not a token stream)
  - whether an anchor NAME is good, or its register (UPPER vs kebab) is right
  - whether an @unknown is honest or an evasion
  - whether a structural insertion is a repair or a content edit
  - whether a stage/state is CORRECT -- only that it is spellable
  - whether a reported semantic problem is real
  - whether the BRIEF was right: its scope, its counts, its predictions
  - emphasis changes (see word_stream's declared blind spots)
";
