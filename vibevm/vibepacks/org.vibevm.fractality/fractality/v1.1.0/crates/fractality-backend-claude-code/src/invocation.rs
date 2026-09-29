//! The headless Claude Code invocation (plan D2, flags pinned live on
//! CC 2.1.202: `--print`, `--output-format stream-json`, `--verbose`,
//! `--model`, `--permission-mode`, `--max-turns`,
//! `--disallowed-tools <tools...>`).
//!
//! The prompt is the packet's goal plus the output contract (D4/D7):
//! the worker's final report lands in the packet-named result file. In
//! print mode CC reads the prompt from **stdin** (smoke-verified on
//! 2.1.202), so the prompt rides [`WorkerSpec::stdin`], never argv:
//! Windows command lines cap at 32 KiB and `.cmd`-shim spawns forbid
//! newline-carrying arguments — both fatal to big one-shot goals (F14).
//!
//! [`WorkerSpec::stdin`]: fractality_core::worker::WorkerSpec

use fractality_core::Packet;
use fractality_core::profile::Profile;

specmark::scope!("spec://fractality/PROP-001#architecture");

/// The worker-facing task text: the goal, verbatim, plus the output
/// contract the collection phase relies on. Fed to the worker's stdin
/// ([`WorkerSpec::stdin`]), never passed as an argument.
///
/// [`WorkerSpec::stdin`]: fractality_core::worker::WorkerSpec
///
/// ```
/// use fractality_backend_claude_code::invocation::build_prompt;
/// use fractality_core::Packet;
///
/// let packet = Packet::from_toml_str(
///     "schema = 1\n[task]\ntitle = \"t\"\ngoal = \"Do the thing.\"\n[routing]\nprofile = \"glm\"\n",
/// )
/// .expect("packet parses");
/// let prompt = build_prompt(&packet, false);
/// assert!(prompt.starts_with("Do the thing."));
/// assert!(prompt.contains("result.md"), "output contract rides the prompt");
/// assert!(!prompt.contains("ask_boss"), "no question protocol unless served");
/// assert!(build_prompt(&packet, true).contains("ask_boss"));
/// ```
pub fn build_prompt(packet: &Packet, ask_boss: bool) -> String {
    let question_protocol = if ask_boss {
        "\nQuestion protocol: you have an `ask_boss` tool. When you are genuinely \
         stuck, missing a decision only your supervisor can make, or about to do \
         anything destructive or irreversible — call ask_boss with ONE precise, \
         answerable question and wait for the reply instead of guessing. Do not \
         use it for things you can decide yourself."
    } else {
        ""
    };
    format!(
        "{goal}\n\n---\nOutput contract (mandatory): when you are done, write your final \
         report to `{result}`, relative to your working directory. Create the file even if \
         the task failed or was only partially done — state plainly what happened, what you \
         changed, and what remains. Do not ask for confirmation; this is a non-interactive \
         run.{question_protocol}",
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
/// let argv = build_argv(&packet, profile, "m-small", None);
/// assert_eq!(argv[0], "claude");
/// assert_eq!(argv[1], "--print");
/// assert_eq!(argv[2], "--output-format", "no prompt positional — the prompt rides stdin");
/// assert!(argv.contains(&"stream-json".to_owned()));
/// assert!(argv.ends_with(&["--disallowed-tools".to_owned(), "WebFetch".to_owned(), "WebSearch".to_owned()]));
/// ```
pub fn build_argv(
    packet: &Packet,
    profile: &Profile,
    model_id: &str,
    mcp_config: Option<&camino::Utf8Path>,
) -> Vec<String> {
    let mut argv = vec![
        profile.claude_binary.clone(),
        "--print".to_owned(),
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
    if let Some(path) = mcp_config {
        argv.push("--mcp-config".to_owned());
        argv.push(path.to_string());
    }
    let broker_allow = mcp_config.map(|_| "mcp__fractality__ask_boss".to_owned());
    if !profile.permissions.allow_tools.is_empty() || broker_allow.is_some() {
        // Variadic flags: each list ends at the next `--flag`, so the
        // allow list rides first and the deny list stays the final tail.
        argv.push("--allowed-tools".to_owned());
        argv.extend(profile.permissions.allow_tools.iter().cloned());
        argv.extend(broker_allow);
    }
    if !profile.permissions.deny_tools.is_empty() {
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

    /// The broker wiring (D18): --mcp-config names the pod-written file
    /// and the ask_boss tool joins the allow list, before the deny tail.
    #[test]
    fn mcp_config_rides_with_its_allow_entry() {
        let profiles = profiles();
        let profile = profiles.get("glm").expect("glm");
        let path = camino::Utf8PathBuf::from("runs/x/mcp-broker.json");
        let argv = build_argv(&packet(), profile, "m-small", Some(&path));
        let joined = argv.join(" ");
        assert!(joined.contains("--mcp-config runs/x/mcp-broker.json"));
        assert!(joined.contains("--allowed-tools mcp__fractality__ask_boss"));
        assert!(
            joined.ends_with("--disallowed-tools WebFetch WebSearch"),
            "deny list stays the final tail: {joined}"
        );
    }

    #[test]
    fn argv_carries_the_pinned_headless_surface() {
        let profiles = profiles();
        let profile = profiles.get("glm").expect("glm");
        let argv = build_argv(&packet(), profile, "m-small", None);
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
    fn no_argument_carries_the_prompt() {
        let profiles = profiles();
        let profile = profiles.get("glm").expect("glm");
        let argv = build_argv(&packet(), profile, "m-small", None);
        assert_eq!(argv[1], "--print");
        assert_eq!(argv[2], "--output-format");
        assert!(
            !argv.iter().any(|a| a.contains("Write hello.")),
            "the goal must ride stdin, never argv (F14)"
        );
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
        let argv = build_argv(&packet(), profile, "b", None);
        assert!(!argv.contains(&"--disallowed-tools".to_owned()));
    }
}
