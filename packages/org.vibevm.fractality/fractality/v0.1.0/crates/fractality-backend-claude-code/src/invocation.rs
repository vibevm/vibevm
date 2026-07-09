//! The headless Claude Code invocation (plan D2, flags pinned live on
//! CC 2.1.202: `--print`, `--output-format stream-json`, `--verbose`,
//! `--model`, `--permission-mode`, `--max-turns`,
//! `--disallowed-tools <tools...>`).
//!
//! The prompt is the packet's goal plus the output contract (D4/D7):
//! the worker's final report lands in the packet-named result file. The
//! variadic `--disallowed-tools` flag comes last so it cannot swallow
//! the prompt positional.

use fractality_core::Packet;
use fractality_core::profile::Profile;

specmark::scope!("spec://fractality/PROP-001#architecture");

/// The worker-facing task text: the goal, verbatim, plus the output
/// contract the collection phase relies on.
///
/// ```
/// use fractality_backend_claude_code::invocation::build_prompt;
/// use fractality_core::Packet;
///
/// let packet = Packet::from_toml_str(
///     "schema = 1\n[task]\ntitle = \"t\"\ngoal = \"Do the thing.\"\n[routing]\nprofile = \"glm\"\n",
/// )
/// .expect("packet parses");
/// let prompt = build_prompt(&packet);
/// assert!(prompt.starts_with("Do the thing."));
/// assert!(prompt.contains("result.md"), "output contract rides the prompt");
/// ```
pub fn build_prompt(packet: &Packet) -> String {
    format!(
        "{goal}\n\n---\nOutput contract (mandatory): when you are done, write your final \
         report to `{result}`, relative to your working directory. Create the file even if \
         the task failed or was only partially done — state plainly what happened, what you \
         changed, and what remains. Do not ask for confirmation; this is a non-interactive run.",
        goal = packet.task.goal.trim_end(),
        result = packet.output.result,
    )
}

/// The full argv for a headless worker run (`argv[0]` is the binary).
///
/// ```
/// use fractality_backend_claude_code::invocation::build_argv;
/// use fractality_core::Packet;
/// use fractality_core::profile::ProfilesFile;
///
/// let profiles = ProfilesFile::from_toml_str(
///     "schema = 1\n[profile.glm]\nbackend = \"claude-code\"\nbase_url = \"http://gw\"\ntoken_file = \"t\"\n[profile.glm.models]\nbig = \"m-big\"\nsmall = \"m-small\"\nhaiku_slot = \"m-small\"\n[profile.glm.permissions]\nmode = \"acceptEdits\"\ndeny_tools = [\"WebFetch\", \"WebSearch\"]\n",
/// )
/// .expect("profiles parse");
/// let profile = profiles.get("glm").expect("glm");
/// let packet = Packet::from_toml_str(
///     "schema = 1\n[task]\ntitle = \"t\"\ngoal = \"g\"\n[routing]\nprofile = \"glm\"\nmodel = \"small\"\n",
/// )
/// .expect("packet parses");
///
/// let argv = build_argv(&packet, profile, "m-small");
/// assert_eq!(argv[0], "claude");
/// assert_eq!(argv[1], "--print");
/// assert!(argv.contains(&"stream-json".to_owned()));
/// assert!(argv.ends_with(&["--disallowed-tools".to_owned(), "WebFetch".to_owned(), "WebSearch".to_owned()]));
/// ```
pub fn build_argv(packet: &Packet, profile: &Profile, model_id: &str) -> Vec<String> {
    let mut argv = vec![
        profile.claude_binary.clone(),
        "--print".to_owned(),
        build_prompt(packet),
        "--output-format".to_owned(),
        "stream-json".to_owned(),
        // stream-json requires --verbose in print mode (F4).
        "--verbose".to_owned(),
        "--model".to_owned(),
        model_id.to_owned(),
        "--permission-mode".to_owned(),
        profile.permissions.mode.clone(),
        "--max-turns".to_owned(),
        packet.budget.max_turns.to_string(),
    ];
    if !profile.permissions.deny_tools.is_empty() {
        // Variadic flag: keep it last so it cannot swallow later args.
        argv.push("--disallowed-tools".to_owned());
        argv.extend(profile.permissions.deny_tools.iter().cloned());
    }
    argv
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packet() -> Packet {
        Packet::from_toml_str(
            r#"
                schema = 1
                [task]
                title = "t"
                goal = "Write hello."
                [budget]
                max_turns = 7
                [routing]
                profile = "glm"
                model = "small"
            "#,
        )
        .expect("packet parses")
    }

    fn profiles() -> fractality_core::profile::ProfilesFile {
        fractality_core::profile::ProfilesFile::from_toml_str(
            r#"
                schema = 1
                [profile.glm]
                backend = "claude-code"
                base_url = "http://gw"
                token_file = "t"
                [profile.glm.models]
                big = "m-big"
                small = "m-small"
                haiku_slot = "m-small"
                [profile.glm.permissions]
                deny_tools = ["WebFetch", "WebSearch"]
            "#,
        )
        .expect("profiles parse")
    }

    #[test]
    fn argv_carries_the_pinned_headless_surface() {
        let profiles = profiles();
        let profile = profiles.get("glm").expect("glm");
        let argv = build_argv(&packet(), profile, "m-small");
        let joined = argv.join(" ");
        assert!(joined.contains("--print"));
        assert!(joined.contains("--output-format stream-json"));
        assert!(joined.contains("--verbose"));
        assert!(joined.contains("--model m-small"));
        assert!(joined.contains("--permission-mode acceptEdits"));
        assert!(joined.contains("--max-turns 7"));
        assert!(joined.ends_with("--disallowed-tools WebFetch WebSearch"));
    }

    #[test]
    fn prompt_rides_as_one_positional_right_after_print() {
        let profiles = profiles();
        let profile = profiles.get("glm").expect("glm");
        let argv = build_argv(&packet(), profile, "m-small");
        assert_eq!(argv[1], "--print");
        assert!(argv[2].starts_with("Write hello."));
        assert!(argv[2].contains("Output contract"));
    }

    #[test]
    fn no_deny_tools_means_no_variadic_tail() {
        let profiles = fractality_core::profile::ProfilesFile::from_toml_str(
            r#"
                schema = 1
                [profile.p]
                backend = "claude-code"
                base_url = "http://gw"
                token_file = "t"
                [profile.p.models]
                big = "a"
                small = "b"
                haiku_slot = "b"
            "#,
        )
        .expect("profiles parse");
        let profile = profiles.get("p").expect("p");
        let argv = build_argv(&packet(), profile, "b");
        assert!(!argv.contains(&"--disallowed-tools".to_owned()));
    }
}
