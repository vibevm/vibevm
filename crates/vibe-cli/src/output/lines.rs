//! The ordinary lines of a command — a heading, a created or kept file, a
//! summary, a warning, the refusal that ends a failed run — written
//! through the display language (PROP-067).

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-067#refusal");

use super::language::{self, Glyph, Kind, Meta, Refusal, Role, Stream, paint};
use super::{Context, Mode, structured};
use specmark::spec;

impl Context {
    fn draws(&self) -> bool {
        !self.is_json() && !self.is_quiet()
    }

    /// A line that names the section a command is in.
    pub fn heading(&self, text: &str) {
        if !self.draws() {
            return;
        }
        let gap = language::before(Stream::Out, Kind::Block);
        self.suspend_progress(|| println!("{gap}{text}"));
    }

    /// A step of a command, as a body line under the block it belongs to.
    #[allow(dead_code)] // used by install
    pub fn step(&self, text: &str) {
        if !self.draws() {
            return;
        }
        self.suspend_progress(|| println!("  {text}"));
    }

    /// A file the command created: a finished row of the files list.
    pub fn created(&self, path: &str) {
        self.file_row(Glyph::Done, Role::Ok, "created", path, None);
    }

    /// A file the command left as it was, and why.
    pub fn skipped(&self, path: &str, reason: &str) {
        self.file_row(Glyph::Skipped, Role::Dim, "kept", path, Some(reason));
    }

    /// A file the command wrote over, and why: a regenerated artifact.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#list")]
    pub fn updated(&self, path: &str, reason: &str) {
        self.file_row(Glyph::Done, Role::Ok, "updated", path, Some(reason));
    }

    /// A file the command removed.
    #[allow(dead_code)] // used by uninstall
    pub fn removed(&self, path: &str) {
        self.file_row(Glyph::Done, Role::Ok, "removed", path, None);
    }

    fn file_row(&self, glyph: Glyph, role: Role, verb: &str, path: &str, why: Option<&str>) {
        if !self.draws() {
            return;
        }
        let gap = language::before(Stream::Out, Kind::Row("files"));
        let why = why
            .map(|why| paint(Role::Dim, &format!(" · {why}")))
            .unwrap_or_default();
        let line = format!(
            "{} {verb:<8} {}{why}",
            paint(role, glyph.text()),
            if glyph == Glyph::Skipped {
                paint(Role::Dim, path)
            } else {
                path.to_owned()
            }
        );
        self.suspend_progress(|| println!("{gap}{line}"));
    }

    pub fn summary(&self, text: &str) {
        if self.suppress_output {
            return;
        }
        match self.mode {
            Mode::Human | Mode::HumanQuiet => {
                language::mark(Stream::Out, Kind::Block);
                self.suspend_progress(|| println!("{text}"));
            }
            Mode::Json => {}
        }
    }

    /// A block written to stdout with the blank line the page owes it.
    pub(crate) fn block(&self, text: &str) {
        if !self.draws() {
            return;
        }
        let gap = language::before(Stream::Out, Kind::Block);
        self.suspend_progress(|| println!("{gap}{text}"));
    }

    /// A block written to stderr — prose beside a document on stdout.
    pub(crate) fn block_err(&self, text: &str) {
        if !self.draws() {
            return;
        }
        let gap = language::before(Stream::Err, Kind::Block);
        self.suspend_progress(|| eprintln!("{gap}{text}"));
    }

    /// One line saying what vibe decided on the operator's behalf, plain
    /// and unglyphed: `node → org.vibevm.node/node`.
    ///
    /// It is stderr in EVERY mode, for `diagnostic`'s reason and one more:
    /// a decision is not a warning, so it carries no warning mark, and it
    /// survives `--quiet` because quiet's one line is the command's answer
    /// and this says what the command was asked about.
    pub fn decision(&self, text: &str) {
        if self.suppress_output {
            return;
        }
        let gap = language::before(Stream::Err, Kind::Block);
        self.suspend_progress(|| eprintln!("{gap}{text}"));
    }

    /// A bounded diagnostic that belongs to no document: `! <sentence>`.
    ///
    /// It is stderr in EVERY mode on purpose: JSON's stdout is a document
    /// stream and a stray object in it is a parse error for the reader,
    /// while dropping the text in JSON mode would make the machine path the
    /// one that silently knows less.
    pub fn diagnostic(&self, text: &str) {
        if self.suppress_output {
            return;
        }
        let gap = language::before(Stream::Err, Kind::Block);
        self.suspend_progress(|| eprintln!("{gap}{}", language::warning(text)));
    }

    /// The refusal that ends a failed command, with an optional compact
    /// suffix.
    ///
    /// The suffix exists for exactly one shape: a QUIET command that failed.
    /// Quiet's contract is one line, and a failed command's one line is this
    /// one — so a trace summary has nowhere else to go. Human mode draws the
    /// refusal block and JSON carries the member inside its root, so
    /// appending there would be the second copy each of them forbids.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#refusal")]
    pub fn error_with_suffix(&self, err: &anyhow::Error, suffix: Option<&str>) {
        if self.suppress_output {
            return;
        }
        match self.mode {
            Mode::HumanQuiet => {
                self.suspend_progress(|| eprintln!("{}", quiet_line(err, suffix)));
            }
            Mode::Human => {
                let block = refusal_of(err).block().render();
                let gap = language::before(Stream::Err, Kind::Block);
                self.suspend_progress(|| eprintln!("{gap}{block}"));
                self.print_pending_trailer();
            }
            Mode::Json => {
                let mut payload = serde_json::json!({
                    "ok": false,
                    "error": format!("{err:#}"),
                });
                structured::stamp_structured_error(&mut payload, err);
                self.stamp_invoked_by(&mut payload);
                self.stamp_unattended(&mut payload);
                self.suspend_progress(|| eprintln!("{payload}"));
            }
        }
    }

    /// The one line of a command the reader stopped at a question with
    /// Escape or Ctrl-C: the failed glyph and `cancelled`, not a refusal
    /// block.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#question")]
    pub(crate) fn cancelled(&self) {
        if self.suppress_output {
            return;
        }
        let line = cancelled_line();
        self.suspend_progress(|| eprintln!("{line}"));
        // A trailer the command held back (a failed or parked app step)
        // still closes the run, as it does after a refusal.
        if self.mode == Mode::Human {
            self.print_pending_trailer();
        }
    }

    /// The trailer of this command (PROP-067 `#trailer`): what happened,
    /// how many blocks its list holds, how long it took and the run record
    /// in full.
    pub(crate) fn trailer_text(&self, ok: bool, what: &str) -> String {
        self.trailer_text_at(ok, what, None)
    }

    /// The trailer text with `record` — an installed home, a store path —
    /// as the place the run left its result, written in full where the run
    /// directory of a step log would otherwise stand.
    fn trailer_text_at(&self, ok: bool, what: &str, record: Option<&str>) -> String {
        let (rows, run) = self
            .progress_renderer
            .as_ref()
            .map(|renderer| renderer.run_summary())
            .unwrap_or_default();
        let mut meta = Vec::new();
        if rows > 0 {
            let unit = if rows == 1 { "step" } else { "steps" };
            meta.push(Meta::Note(format!("{rows} {unit}")));
        }
        meta.push(Meta::Time(self.started.elapsed()));
        if let Some(record) = record {
            meta.push(Meta::note(record));
            return language::trailer(ok, what, meta, None);
        }
        let run = run.map(|run| run.display().to_string());
        language::trailer(ok, what, meta, run.as_deref())
    }

    /// Write the trailer of a command that did its work: the last block,
    /// and in quiet mode the one line.
    pub(crate) fn trailer(&self, what: &str) {
        self.trailer_on(Stream::Out, true, what);
    }

    /// The trailer of a run that ended with a verdict of its own — a
    /// program's exit status — on `stream`: stderr when stdout belongs to
    /// the program that ran, so a pipe reading it gets the program's bytes
    /// alone.
    pub(crate) fn trailer_on(&self, stream: Stream, ok: bool, what: &str) {
        self.write_trailer(stream, self.trailer_text(ok, what));
    }

    /// The trailer of a run whose result is a place on disk, written last
    /// and in full (`installed branch:main#106 · 2 steps · 41s · <home>`).
    pub(crate) fn trailer_at(&self, what: &str, record: &str) {
        self.write_trailer(Stream::Out, self.trailer_text_at(true, what, Some(record)));
    }

    /// The run verdict with its retained result written last and in full.
    pub(crate) fn trailer_verdict_at(&self, ok: bool, what: &str, record: &str) {
        self.write_trailer(Stream::Out, self.trailer_text_at(ok, what, Some(record)));
    }

    fn write_trailer(&self, stream: Stream, text: String) {
        if self.suppress_output || self.is_json() {
            return;
        }
        let gap = if self.is_quiet() {
            ""
        } else {
            language::before(stream, Kind::Block)
        };
        self.suspend_progress(|| match stream {
            Stream::Out => println!("{gap}{text}"),
            Stream::Err => eprintln!("{gap}{text}"),
        });
    }

    /// The rule that opens a section (PROP-067 `#heading`), as a block of
    /// its own on stdout.
    pub(crate) fn rule(&self, label: &str) {
        self.block(&language::rule(label, None));
    }

    /// A remark that is neither a warning nor a step (the `note:` lines of
    /// old): a dim line at column 2. Consecutive remarks touch, as the rows
    /// of one list, and the group stands off its neighbours.
    pub(crate) fn hint(&self, text: &str) {
        if !self.draws() {
            return;
        }
        let gap = language::before(Stream::Out, Kind::Row("hints"));
        let line = format!("  {}", language::hint(text));
        self.suspend_progress(|| println!("{gap}{line}"));
    }

    /// Hold the trailer of a failed run until its refusal is written: the
    /// trailer is the last block of a command (PROP-067 `#trailer`).
    pub(crate) fn defer_trailer(&self, trailer: String) {
        if let Ok(mut held) = self.pending_trailer.lock() {
            *held = Some(trailer);
        }
    }

    fn print_pending_trailer(&self) {
        let held = self
            .pending_trailer
            .lock()
            .ok()
            .and_then(|mut held| held.take());
        if let Some(trailer) = held {
            let gap = language::before(Stream::Err, Kind::Block);
            self.suspend_progress(|| eprintln!("{gap}{trailer}"));
        }
    }
}

/// The line of a command stopped at a question: the failed glyph, then
/// `cancelled`.
pub(crate) fn cancelled_line() -> String {
    format!("{} cancelled", paint(Role::Alert, Glyph::Failed.text()))
}

/// The refusal an error reads as: its code (the application's own, or a
/// `snake_case:` lead word), the sentence before its `(violates …; fix: …)`
/// tail as the headline, the spec address as `see`, the fix's commands as
/// `fix`, and whatever the chain adds after the tail as `why`.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#refusal")]
pub(crate) fn refusal_of(err: &anyhow::Error) -> Refusal {
    let mut refusal = parse_refusal(
        &format!("{err:#}"),
        structured::app_error(err).map(|error| error.code().to_owned()),
    );
    if refusal.code.as_deref() == Some("px_no_proxy") {
        refusal
            .fix
            .push("vibe tool px exec --allow-direct -- true".to_owned());
    }
    refusal
}

/// Read the nested text shapes of a refusal without manufacturing an error.
fn parse_refusal(text: &str, mut code: Option<String>) -> Refusal {
    let mut text = text;
    if let Some((lead, rest)) = text.split_once(": ")
        && chain::is_code(lead)
    {
        code.get_or_insert_with(|| lead.to_owned());
        if code.as_deref() == Some(lead) {
            text = rest;
        }
    }
    let (reqs, rest) = chain::strip_req(text);
    let mut surfaces = Vec::new();
    let joined;
    if !reqs.is_empty() {
        let mut clauses = Vec::new();
        for segment in chain::split0(chain::drop_repeat(rest)) {
            if let Some(surface) = segment.strip_prefix("fix surface: ") {
                let surface = surface.trim();
                if !surfaces.contains(&surface) {
                    surfaces.push(surface);
                }
            } else {
                clauses.push(segment);
            }
        }
        joined = clauses.join("; ");
        text = &joined;
    }
    // The cause the chain appends may be nothing but a tail of its own.
    let start = if text.starts_with("(violates ") {
        Some(0)
    } else {
        text.find(" (violates ").map(|at| at + 1)
    };
    let Some(start) = start else {
        let mut refusal = Refusal {
            code,
            ..Refusal::default()
        };
        if reqs.is_empty() {
            refusal.headline = text.trim().to_owned();
        } else {
            let segments = chain::split0(text);
            refusal.headline = segments[0].trim().to_owned();
            refusal.why = (!segments[1..].is_empty()).then(|| {
                segments[1..]
                    .iter()
                    .map(|s| s.trim())
                    .collect::<Vec<_>>()
                    .join("; ")
            });
            refusal.fix = surfaces.first().map(|s| fix_lines(s)).unwrap_or_default();
            refusal.see = reqs.last().map(|s| (*s).to_owned());
        }
        return embedded_code(refusal);
    };
    let inner_start = start + "(violates ".len();
    let end = chain::closing(text, inner_start).unwrap_or(text.len());
    if let Some(open) = chain::unmatched_open(&text[..start])
        && end < text.len()
        && let Some(close) = chain::closing(text, end + 1)
    {
        let cause = parse_refusal(&text[open + 1..close], None);
        let outer = format!("{}{}", text[..open].trim_end(), &text[close + 1..]);
        let outer = parse_refusal(&outer, None);
        let why = [Some(cause.headline), cause.why, outer.why]
            .into_iter()
            .flatten()
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("; ");
        let mut fixes = cause.fix;
        for fix in outer.fix {
            if !fixes.contains(&fix) {
                fixes.push(fix);
            }
        }
        return Refusal {
            code: code.or(outer.code),
            headline: outer.headline,
            why: (!why.is_empty()).then_some(why),
            fix: fixes,
            see: outer.see.or(cause.see),
        };
    }
    let inner = &text[inner_start..end];
    let (see, fix) = match inner.split_once("; fix: ") {
        Some((see, fix)) => (see.trim(), Some(fix.trim())),
        None => (inner.trim(), None),
    };
    let see = see.strip_prefix("REQ ").unwrap_or(see);
    let rest = text
        .get(end + 1..)
        .unwrap_or("")
        .trim_start_matches([':', ';', ' ']);
    // The cause's fix is the nearer one, so it comes first.
    let (after_fix, cause) = if let Some(fix) = rest.strip_prefix("fix: ") {
        (fix_lines(fix), None)
    } else {
        (
            Vec::new(),
            (!rest.is_empty()).then(|| parse_refusal(rest, None)),
        )
    };
    let mut fixes = Vec::new();
    if let Some(cause) = &cause {
        fixes.extend(cause.fix.iter().cloned());
    }
    for line in fix
        .map(fix_lines)
        .unwrap_or_default()
        .into_iter()
        .chain(after_fix)
    {
        if !fixes.contains(&line) {
            fixes.push(line);
        }
    }
    if fixes.is_empty()
        && let Some(surface) = surfaces.first()
    {
        fixes = fix_lines(surface);
    }
    embedded_code(Refusal {
        code,
        headline: text[..start].trim().to_owned(),
        why: cause.and_then(Refusal::into_why),
        fix: fixes,
        see: if see.is_empty() {
            reqs.last().map(|s| (*s).to_owned())
        } else {
            Some(see.to_owned())
        },
    })
}

/// Move a coded inner error out of the outer headline into its explanation.
fn embedded_code(mut refusal: Refusal) -> Refusal {
    for i in chain::depth0(&refusal.headline, ": ") {
        if let Some((word, _)) = refusal.headline[i + 2..].split_once(": ")
            && chain::is_code(word)
        {
            let cause = refusal.headline[i + 2..].to_owned();
            refusal.headline.truncate(i);
            refusal.why = Some(match refusal.why {
                Some(why) => format!("{cause}; {why}"),
                None => cause,
            });
            break;
        }
    }
    refusal
}

/// Quiet's single refusal line, naming its code only once.
#[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#refusal")]
pub(crate) fn quiet_line(err: &anyhow::Error, suffix: Option<&str>) -> String {
    let refusal = refusal_of(err);
    let message = format!("{err:#}");
    let message = refusal
        .code
        .as_ref()
        .and_then(|code| message.strip_prefix(&format!("{code}: ")))
        .unwrap_or(&message);
    let code = refusal
        .code
        .map(|code| format!("{code}  "))
        .unwrap_or_default();
    format!(
        "{} {}{message}{}",
        paint(Role::Alert, Glyph::Failed.text()),
        paint(Role::Alert, &code),
        suffix.unwrap_or_default()
    )
}

impl Refusal {
    /// The refusal read as the `why` of the error that wraps it: its own
    /// sentence (behind its code), then the causes behind it, so a deeper
    /// cause — the declared origin a drop miss fell back to — is not lost.
    fn into_why(self) -> Option<String> {
        let headline = (!self.headline.is_empty()).then(|| match self.code {
            Some(code) => format!("{code}: {}", self.headline),
            None => self.headline,
        });
        match (headline, self.why) {
            (Some(headline), Some(deeper)) => Some(format!("{headline}; {deeper}")),
            (headline, deeper) => headline.or(deeper),
        }
    }
}

/// The commands a fix names, or its sentence when it names no commands.
fn fix_lines(fix: &str) -> Vec<String> {
    let mut commands = Vec::new();
    for part in fix
        .split('`')
        .skip(1)
        .step_by(2)
        .filter(|part| part.starts_with("vibe "))
    {
        let command = part.to_owned();
        if !commands.contains(&command) {
            commands.push(command);
        }
    }
    if commands.is_empty() {
        let mut rest = fix;
        let mut plain = String::new();
        while let Some((before, quoted)) = rest.split_once('`') {
            let Some((content, after)) = quoted.split_once('`') else {
                break;
            };
            plain.push_str(before);
            plain.push_str(content);
            rest = after;
        }
        plain.push_str(rest);
        let sentence = plain.trim().to_owned();
        if !sentence.is_empty() {
            commands.push(sentence);
        }
    }
    commands
}

mod chain;

#[cfg(test)]
#[path = "lines/tests.rs"]
mod tests;
