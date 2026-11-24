//! One soft-launch warning, followed by its repair prose and commands.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-SOFT-LAUNCH");

use crate::commands::prompt::Undeliverable;
use crate::output::language::{self, Role, paint};

/// Say which launcher starts without the lane, and how to repair delivery.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-SOFT-LAUNCH")]
pub(super) fn soft_warning(name: &str, undeliverable: &Undeliverable, width: usize) -> String {
    let reason = undeliverable.reason.replace('`', "");
    let wrapped = language::wrap(
        &format!("{name} starts without this project's boot lane: {reason}"),
        width.saturating_sub(2).max(20),
    );
    let mut lines = Vec::new();
    for (index, line) in wrapped.iter().enumerate() {
        lines.push(if index == 0 {
            language::warning(line)
        } else {
            format!("  {}", paint(Role::Warn, line))
        });
    }
    let (sentence, commands) = fix_lines(&undeliverable.fix);
    if let Some(sentence) = sentence {
        lines.push(format!("  {}", language::hint(&sentence)));
    }
    for command in commands {
        lines.push(format!("  {}", paint(Role::Accent, &command)));
    }
    lines.join("\n")
}

/// Extract only backticked vibe commands, keeping any substantive repair prose.
fn fix_lines(fix: &str) -> (Option<String>, Vec<String>) {
    let commands: Vec<String> = fix
        .split('`')
        .enumerate()
        .filter(|(index, span)| index % 2 == 1 && span.starts_with("vibe "))
        .map(|(_, span)| span.to_owned())
        .collect();
    let sentence = fix.replace('`', "");
    let mut remainder = sentence.clone();
    for command in &commands {
        remainder = remainder.replace(command, "");
    }
    let substantive = remainder
        .split(|character: char| !character.is_alphanumeric())
        .any(|word| !matches!(word, "" | "or" | "and" | "then" | "else"));
    (substantive.then_some(sentence), commands)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soft_launch_prints_single_warning() {
        let outside = Undeliverable::hard(
            "this directory is not a vibe project".to_owned(),
            "`vibe init`".to_owned(),
        );
        assert_eq!(
            soft_warning("fake", &outside, 100),
            "! fake starts without this project's boot lane: this directory is not a vibe project\n  vibe init"
        );

        let prose = Undeliverable::hard(
            "the probe failed".to_owned(),
            "write the number `codex --version` prints".to_owned(),
        );
        assert_eq!(
            soft_warning("codex", &prose, 100),
            "! codex starts without this project's boot lane: the probe failed\n  write the number codex --version prints"
        );

        let both = Undeliverable::hard(
            "no base set".to_owned(),
            "install it (`vibe install -g org.vibevm.opencode/opencode`), or point a variable at it"
                .to_owned(),
        );
        let text = soft_warning("opencode", &both, 100);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 3, "{text}");
        assert_eq!(
            lines[1],
            "  install it (vibe install -g org.vibevm.opencode/opencode), or point a variable at it"
        );
        assert_eq!(lines[2], "  vibe install -g org.vibevm.opencode/opencode");

        let long = Undeliverable::hard("word ".repeat(30), "`vibe init`".to_owned());
        let text = soft_warning("fake", &long, 40);
        assert_eq!(
            text.lines().filter(|line| line.starts_with("! ")).count(),
            1
        );
        for (index, line) in text.lines().enumerate() {
            assert!(console::measure_text_width(line) <= 40, "{line}");
            assert!(index == 0 || line.starts_with("  "), "{line}");
        }
    }

    #[test]
    fn fix_lines_preserves_prose_and_extracts_only_vibe_commands() {
        assert_eq!(
            fix_lines("`vibe init`, or `vibe prompt build`; and then else."),
            (
                None,
                vec!["vibe init".to_owned(), "vibe prompt build".to_owned()]
            )
        );
        assert_eq!(
            fix_lines("run `vibe prompt build`"),
            (
                Some("run vibe prompt build".to_owned()),
                vec!["vibe prompt build".to_owned()]
            )
        );
        assert_eq!(
            fix_lines("use `codex --version` or `vibex init`"),
            (Some("use codex --version or vibex init".to_owned()), vec![])
        );
        let reason = Undeliverable::hard("missing `base`".to_owned(), "`vibe init`".to_owned());
        assert!(!soft_warning("fake", &reason, 100).contains('`'));
    }
}
