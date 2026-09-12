//! Reading a help screen — the two layouts clap prints, and the things
//! that must not reach a snapshot.

use super::*;

const WIDE: &str = "\
Work with a documentation package.

Usage: vibe doc [OPTIONS] <COMMAND>

Commands:
  build    Render a documentation package: every page in the projection
           you ask for
  check    Check a documentation package against the product
  help     Print this message or the help of the given subcommand(s)

Options:
      --path <PATH>  The documentation package. Defaults to the current
                     directory
  -q, --quiet        Say less
  -h, --help         Print help
";

/// clap moves the summary to its own line when the flag column grows —
/// the same screen, laid out differently, and the same surface.
const NEXT_LINE: &str = "\
Usage: vibe doc [OPTIONS]

Options:
      --path <PATH>
          The documentation package. Defaults to the current
          directory

  -h, --help
          Print help
";

#[test]
fn a_wide_screen_reads_as_commands_and_flags() {
    let help = parse(WIDE);
    assert_eq!(
        help.subcommands,
        vec![
            (
                "build".to_string(),
                "Render a documentation package: every page in the projection you ask for"
                    .to_string()
            ),
            (
                "check".to_string(),
                "Check a documentation package against the product".to_string()
            ),
            (
                "help".to_string(),
                "Print this message or the help of the given subcommand(s)".to_string()
            ),
        ]
    );
    assert_eq!(help.flags.len(), 3);
    assert_eq!(help.flags[0].name, "--path");
    assert_eq!(help.flags[0].value, "PATH");
    assert_eq!(
        help.flags[0].summary,
        "The documentation package. Defaults to the current directory"
    );
    assert_eq!(help.flags[1].name, "--quiet");
    assert_eq!(help.flags[1].value, "");
}

#[test]
fn the_two_layouts_of_one_screen_read_the_same() {
    let wide = parse(WIDE);
    let next_line = parse(NEXT_LINE);
    let path = |help: &Help| {
        help.flags
            .iter()
            .find(|f| f.name == "--path")
            .cloned()
            .expect("the flag")
    };
    assert_eq!(
        path(&wide).summary,
        path(&next_line).summary,
        "a summary wrapped by the terminal must not read as a different flag — \
         otherwise a wider window reports the whole product as changed"
    );
    assert_eq!(path(&wide).value, path(&next_line).value);
}

#[test]
fn a_short_only_flag_keeps_its_short_spelling() {
    let help = parse("Usage: vibe\n\nOptions:\n  -V  Print version\n");
    assert_eq!(help.flags[0].name, "-V");
}

#[test]
fn an_alias_does_not_become_a_second_command() {
    let help = parse("Usage: vibe\n\nCommands:\n  list, ls  Show what is installed\n");
    assert_eq!(help.subcommands.len(), 1);
    assert_eq!(help.subcommands[0].0, "list");
}

#[test]
fn the_usage_line_and_the_about_are_not_part_of_the_surface() {
    let help = parse(WIDE);
    assert!(
        !help
            .flags
            .iter()
            .any(|f| f.summary.contains("Usage") || f.name.contains("Usage")),
        "the usage line is a rendering of the surface, not a member of it"
    );
}

/// The layout `vibe doc build --help` actually prints: every option
/// long-only, so every flag stands in the six-space column, with its
/// summary, its default and its possible values under it. Each is its own
/// entry — the defect this test exists for was all of them arriving as
/// one.
#[test]
fn options_in_the_long_only_column_are_separate_entries() {
    let help = concat!(
        "Usage: vibe doc build [OPTIONS]\n\n",
        "Options:\n",
        "      --json\n",
        "          Produce machine-readable JSON output\n\n",
        "      --path <PATH>\n",
        "          The documentation package to render\n",
        "          \n",
        "          [default: .]\n\n",
        "      --agent-mode <MODE>\n",
        "          How this invocation executes contributions\n\n",
        "          Possible values:\n",
        "          - auto: Infer from the resolved value\n",
        "          - cli:  Always call the provider\n\n",
        "  -h, --help\n",
        "          Print help\n",
    );
    let parsed = parse(help);
    let names: Vec<&str> = parsed.flags.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, vec!["--json", "--path", "--agent-mode", "--help"]);
    assert_eq!(parsed.flags[1].value, "PATH");
    assert_eq!(
        parsed.flags[1].summary, "The documentation package to render [default: .]",
        "a default is part of the contract and travels with the flag"
    );
    assert!(
        parsed.flags[2].summary.contains("- cli: Always call"),
        "a possible-values list belongs to the flag above it, not to the next flag"
    );
}

#[test]
fn an_argument_section_is_read_as_nothing() {
    let help = parse("Usage: vibe add <NAME>\n\nArguments:\n  <NAME>  What to add\n");
    assert!(help.subcommands.is_empty());
    assert!(help.flags.is_empty());
}
