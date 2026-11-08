//! When a project operation asks before it stops, as pure decisions.
//!
//! The stop input of an attended terminal turns the first Ctrl-C into a
//! question; every other input — `--stop-file`, `SIGTERM`, Ctrl-Break, a
//! Ctrl-C with nobody at the terminal — requests the stop at once. These
//! functions hold that law without a terminal, a signal or a thread, so the
//! whole matrix is unit-tested; `super::watch` only carries them out.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-073#PROJECT-COOPERATIVE-STOP");

/// The question an attended Ctrl-C asks. Its default is no.
pub(crate) const QUESTION: &str = "Stop the operation? Changes made so far will be rolled back.";

/// How this invocation answers a Ctrl-C.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Posture {
    /// Someone is at the terminal: the first Ctrl-C asks.
    Ask,
    /// Nobody can answer: a Ctrl-C requests the stop at once.
    Immediate,
}

/// What decides whether a person can answer the question.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Terminal {
    /// Standard input is a terminal: the answer can be typed.
    pub(crate) stdin: bool,
    /// Standard error is a terminal: the question can be seen.
    pub(crate) stderr: bool,
    /// `--unattended` or `VIBE_UNATTENDED`: nobody is there by contract.
    pub(crate) unattended: bool,
    /// `--json`: a machine reads this run, and machine runs never prompt.
    pub(crate) machine: bool,
}

/// The posture of one invocation, decided once before the operation runs.
///
/// ```text
/// stdin tty · stderr tty · attended · human   ->  Ask
/// anything else                               ->  Immediate
/// ```
pub(crate) fn posture(terminal: Terminal) -> Posture {
    if terminal.stdin && terminal.stderr && !terminal.unattended && !terminal.machine {
        Posture::Ask
    } else {
        Posture::Immediate
    }
}

/// Where the stop input stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Phase {
    /// The operation runs.
    Running,
    /// The question is on the terminal; no new checkpoint is passed.
    Asking,
    /// The stop is requested; nothing after it changes that.
    Stopping,
}

/// One input to the stop owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Event {
    /// Ctrl-C: `SIGINT`, or the console's `CTRL_C_EVENT`.
    Interrupt,
    /// `SIGTERM` or Ctrl-Break: never a question.
    Terminate,
    /// The person answered the question.
    Answered(Answer),
}

/// What the question returned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Answer {
    /// `y`.
    Stop,
    /// `n`, Enter (the default), or the end of input.
    Continue,
    /// Ctrl-C again while the question was shown.
    Interrupted,
}

/// What the stop owner does next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Action {
    /// Request the stop on the operation's token.
    Stop,
    /// Show the question.
    Ask,
    /// Let held checkpoints pass again.
    Resume,
    /// Nothing changes.
    Nothing,
}

/// The one transition of the stop owner.
pub(crate) fn step(posture: Posture, phase: Phase, event: Event) -> (Phase, Action) {
    match (phase, event) {
        (Phase::Stopping, _) => (Phase::Stopping, Action::Nothing),
        (_, Event::Terminate) => (Phase::Stopping, Action::Stop),
        (Phase::Running, Event::Interrupt) => match posture {
            Posture::Ask => (Phase::Asking, Action::Ask),
            Posture::Immediate => (Phase::Stopping, Action::Stop),
        },
        // A second Ctrl-C while the question is shown stops without waiting.
        (Phase::Asking, Event::Interrupt) => (Phase::Stopping, Action::Stop),
        (Phase::Asking, Event::Answered(Answer::Stop | Answer::Interrupted)) => {
            (Phase::Stopping, Action::Stop)
        }
        (Phase::Asking, Event::Answered(Answer::Continue)) => (Phase::Running, Action::Resume),
        // An answer with no question open is stale.
        (Phase::Running, Event::Answered(_)) => (Phase::Running, Action::Nothing),
    }
}

/// The answer a finished question gives: a typed yes stops; a Ctrl-C
/// inside the question stops; no, Enter and the end of input continue.
pub(crate) fn answer(result: &dialoguer::Result<bool>) -> Answer {
    match result {
        Ok(true) => Answer::Stop,
        Ok(false) => Answer::Continue,
        Err(dialoguer::Error::IO(error)) if error.kind() == std::io::ErrorKind::Interrupted => {
            Answer::Interrupted
        }
        Err(_) => Answer::Continue,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ATTENDED: Terminal = Terminal {
        stdin: true,
        stderr: true,
        unattended: false,
        machine: false,
    };

    #[test]
    #[specmark::verifies("spec://org.vibevm.core/vibevm/common/PROP-073#PROJECT-COOPERATIVE-STOP")]
    fn only_an_attended_human_terminal_is_asked() {
        assert_eq!(posture(ATTENDED), Posture::Ask);
        for terminal in [
            Terminal {
                stdin: false,
                ..ATTENDED
            },
            Terminal {
                stderr: false,
                ..ATTENDED
            },
            Terminal {
                unattended: true,
                ..ATTENDED
            },
            Terminal {
                machine: true,
                ..ATTENDED
            },
        ] {
            assert_eq!(posture(terminal), Posture::Immediate, "{terminal:?}");
        }
    }

    #[test]
    fn the_first_attended_ctrl_c_asks_and_the_second_stops() {
        let (phase, action) = step(Posture::Ask, Phase::Running, Event::Interrupt);
        assert_eq!((phase, action), (Phase::Asking, Action::Ask));
        assert_eq!(
            step(Posture::Ask, phase, Event::Interrupt),
            (Phase::Stopping, Action::Stop)
        );
    }

    #[test]
    fn an_unattended_ctrl_c_and_every_terminate_stop_at_once() {
        assert_eq!(
            step(Posture::Immediate, Phase::Running, Event::Interrupt),
            (Phase::Stopping, Action::Stop)
        );
        for posture in [Posture::Ask, Posture::Immediate] {
            for phase in [Phase::Running, Phase::Asking] {
                assert_eq!(
                    step(posture, phase, Event::Terminate),
                    (Phase::Stopping, Action::Stop),
                    "{posture:?} {phase:?}"
                );
            }
        }
    }

    #[test]
    fn the_answer_stops_or_resumes_and_a_stale_one_changes_nothing() {
        for (answer, expected) in [
            (Answer::Stop, (Phase::Stopping, Action::Stop)),
            (Answer::Interrupted, (Phase::Stopping, Action::Stop)),
            (Answer::Continue, (Phase::Running, Action::Resume)),
        ] {
            assert_eq!(
                step(Posture::Ask, Phase::Asking, Event::Answered(answer)),
                expected
            );
            assert_eq!(
                step(Posture::Ask, Phase::Running, Event::Answered(answer)),
                (Phase::Running, Action::Nothing)
            );
        }
    }

    #[test]
    fn nothing_undoes_a_requested_stop() {
        for event in [
            Event::Interrupt,
            Event::Terminate,
            Event::Answered(Answer::Continue),
        ] {
            assert_eq!(
                step(Posture::Ask, Phase::Stopping, event),
                (Phase::Stopping, Action::Nothing)
            );
        }
    }

    #[test]
    fn yes_and_ctrl_c_stop_while_no_enter_and_end_of_input_continue() {
        let io = |kind| Err(dialoguer::Error::IO(std::io::Error::new(kind, "read")));
        assert_eq!(answer(&Ok(true)), Answer::Stop);
        assert_eq!(answer(&Ok(false)), Answer::Continue);
        assert_eq!(
            answer(&io(std::io::ErrorKind::Interrupted)),
            Answer::Interrupted
        );
        assert_eq!(
            answer(&io(std::io::ErrorKind::UnexpectedEof)),
            Answer::Continue
        );
    }
}
