//! Usage errors are refusals of PROP-067 `#refusal`.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-067#refusal");

use std::ffi::OsString;
use std::io::IsTerminal;

use crate::output::language::Refusal;

/// Turn clap's plain diagnostic into a refusal, retaining original operands.
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-067#refusal")]
pub(crate) fn usage_refusal(rendered: &str, args: &[OsString]) -> Refusal {
    let mut lines = rendered.lines();
    let first = lines.next().unwrap_or_default();
    let mut headline = first.strip_prefix("error: ").unwrap_or(first).to_owned();
    for line in lines.take_while(|line| !line.trim().is_empty()) {
        headline.push(' ');
        headline.push_str(line.trim());
    }
    // Only repair the two known rejected shapes. Suggestions are text, never run.
    for words in args.windows(3) {
        let Some(operand) = words[2].to_str() else { continue };
        // A bare suggestion operand must be one literal shell word on its own.
        // Anything requiring quoting retains clap's generic --help fallback.
        if operand.is_empty() || operand.starts_with('-')
            || !operand.bytes().all(|byte| byte.is_ascii_alphanumeric()
                || matches!(byte, b'/' | b'.' | b'_' | b'-'))
        {
            continue;
        }
        let (command, headline, fix) = if words[0] == "kit"
            && words[1] == "init"
            && !operand.starts_with('-')
            && headline == format!("unexpected argument '{operand}' found")
        {
            ("vibe kit init", format!("unexpected argument {operand}"),
             format!("vibe kit init --file {operand}"))
        } else if words[0] == "registry"
            && words[1] == "remove"
            && headline == format!("unrecognized subcommand '{operand}'")
        {
            ("vibe registry remove", format!("unrecognized subcommand {operand}"),
             format!("vibe registry remove registry {operand}"))
        } else {
            continue;
        };
        return Refusal {
            code: Some("usage".to_owned()),
            headline,
            why: None,
            fix: vec![fix],
            see: Some(format!("{command} --help")),
        };
    }
    let tips: Vec<_> = rendered.lines()
        .filter_map(|line| line.trim().strip_prefix("tip: "))
        .collect();
    let command = rendered.lines()
        .find_map(|line| line.strip_prefix("Usage: "))
        .map(|usage| usage.split_whitespace()
            .take_while(|word| !word.starts_with(['-', '[', '<']))
            .collect::<Vec<_>>().join(" "))
        .unwrap_or_else(|| "vibe".to_owned());
    Refusal {
        code: Some("usage".to_owned()),
        headline,
        why: (!tips.is_empty()).then(|| tips.join("; ")),
        fix: vec![format!("{command} --help")],
        see: None,
    }
}

/// Preserve clap's help/version paths and exit status; render other errors once.
pub(crate) fn exit(error: clap::Error, args: &[OsString]) -> ! {
    use clap::error::ErrorKind;
    if matches!(error.kind(), ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
        | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand)
    {
        error.exit();
    }
    let plain = args.iter().any(|arg| arg == "--plain" || arg == "--json" || arg == "--quiet");
    let vocabulary = std::env::var("TERM").map_or(true, |term| !term.eq_ignore_ascii_case("dumb"));
    crate::output::configure_language(
        std::io::stdout().is_terminal() && std::io::stderr().is_terminal(),
        vocabulary && !plain,
    );
    eprintln!("{}", usage_refusal(&error.render().to_string(), args).block().render());
    std::process::exit(error.exit_code());
}
