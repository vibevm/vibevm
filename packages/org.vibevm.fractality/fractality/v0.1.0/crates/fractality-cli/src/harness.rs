//! `fractality harness install|status|remove claude-code` (Campaign 2
//! D4): writing our hook entries into a settings file we do not own.
//!
//! The managed-blocks law, adapted to JSON: **the command string is the
//! ownership marker.** We create, update, and remove exactly the
//! entries a deterministic scan recognizes as ours (a quoted
//! `fractality*` executable followed by `hook <event>` or
//! `statusline`), and never touch a byte of anyone else's
//! configuration. A malformed settings file is a hard stop with a
//! precise report — never an auto-repair. Default target is
//! `.claude/settings.local.json` (machine-scoped, like
//! `~/.fractality/profiles.toml`); `--project` opts into the committed
//! `.claude/settings.json` (RP3).

use camino::Utf8PathBuf;
use clap::Subcommand;
use serde_json::{Value, json};

use crate::{EXIT_INFRA, EXIT_NEGATIVE, EXIT_OK, fail_code};

specmark::scope!("spec://fractality/PROP-001#sessions");

/// The `fractality harness <verb>` grammar (lives with its cell).
#[derive(Subcommand)]
pub(crate) enum HarnessCmd {
    /// Write our hook + statusline entries (default target:
    /// .claude/settings.local.json — machine-scoped; RP3).
    Install {
        /// Harness name (only `claude-code` today).
        harness: String,
        /// Write the committed .claude/settings.json instead.
        #[arg(long)]
        project: bool,
        /// Project directory (defaults to the current one).
        #[arg(long, value_name = "DIR")]
        target: Option<Utf8PathBuf>,
    },
    /// Report what is installed, stale, foreign, or absent.
    Status {
        harness: String,
        #[arg(long)]
        project: bool,
        #[arg(long, value_name = "DIR")]
        target: Option<Utf8PathBuf>,
    },
    /// Remove exactly our entries; foreign configuration survives.
    Remove {
        harness: String,
        #[arg(long)]
        project: bool,
        #[arg(long, value_name = "DIR")]
        target: Option<Utf8PathBuf>,
    },
}

/// The hook events the adapter owns, with their matchers and timeouts
/// (seconds; every budget is far above the measured ~6 ms exe spawn,
/// far below the harness's own caps — F22).
const EVENTS: &[(&str, Option<&str>, &str, u64)] = &[
    ("SessionStart", None, "hook session-start", 10),
    ("UserPromptSubmit", None, "hook user-prompt-submit", 5),
    (
        "PostToolUse",
        Some("Bash|Edit|Write|MultiEdit|NotebookEdit"),
        "hook post-tool-use",
        5,
    ),
    ("Stop", None, "hook stop", 5),
    ("SessionEnd", None, "hook session-end", 5),
];

/// True when a hook-entry command string is ours: a first token whose
/// file stem starts with `fractality`, followed exactly by one of our
/// verb suffixes. Deterministic byte scan — no heuristics, no regex.
fn is_ours(command: &str) -> bool {
    let (exe, rest) = match split_command(command) {
        Some(pair) => pair,
        None => return false,
    };
    let stem = exe
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(exe)
        .trim_end_matches(".exe");
    if !stem.starts_with("fractality") {
        return false;
    }
    rest == "statusline" || EVENTS.iter().any(|(_, _, verb, _)| rest == *verb)
}

/// Splits `"C:\path with spaces\fractality.exe" hook stop` (or the
/// unquoted form) into the executable token and the rest.
fn split_command(command: &str) -> Option<(&str, &str)> {
    let s = command.trim();
    if let Some(stripped) = s.strip_prefix('"') {
        let end = stripped.find('"')?;
        Some((&stripped[..end], stripped[end + 1..].trim()))
    } else {
        let end = s.find(' ')?;
        Some((&s[..end], s[end..].trim()))
    }
}

fn quoted(exe: &str, suffix: &str) -> String {
    format!("\"{exe}\" {suffix}")
}

/// Installs/updates our entries in the settings document. Pure over
/// the JSON value; returns human notices. Errors only on structural
/// impossibility (non-object where an object must be) — the caller
/// hard-stops.
fn upsert(doc: &mut Value, exe: &str) -> Result<Vec<String>, String> {
    let mut notices = Vec::new();
    let root = doc
        .as_object_mut()
        .ok_or("the settings root is not a JSON object")?;
    let hooks = root
        .entry("hooks")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or("`hooks` is not a JSON object")?;
    for (event, matcher, verb, timeout) in EVENTS {
        let groups = hooks
            .entry(*event)
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .ok_or_else(|| format!("`hooks.{event}` is not an array"))?;
        strip_ours_from_groups(groups);
        let mut group = serde_json::Map::new();
        if let Some(m) = matcher {
            group.insert("matcher".into(), json!(m));
        }
        group.insert(
            "hooks".into(),
            json!([{
                "type": "command",
                "command": quoted(exe, verb),
                "timeout": timeout,
            }]),
        );
        groups.push(Value::Object(group));
    }
    match root.get("statusLine") {
        None => {
            root.insert(
                "statusLine".into(),
                json!({
                    "type": "command",
                    "command": quoted(exe, "statusline"),
                    "refreshInterval": 30,
                }),
            );
        }
        Some(existing) => {
            let existing_cmd = existing.get("command").and_then(Value::as_str);
            if existing_cmd.is_some_and(is_ours) {
                root.insert(
                    "statusLine".into(),
                    json!({
                        "type": "command",
                        "command": quoted(exe, "statusline"),
                        "refreshInterval": 30,
                    }),
                );
            } else {
                notices.push(
                    "statusLine is configured by someone else — left untouched (our \
                     scoreboard line is available via `fractality scoreboard --line`)"
                        .to_owned(),
                );
            }
        }
    }
    Ok(notices)
}

/// Removes our entries. Pure; empty containers left behind by the
/// removal are dropped so a clean uninstall restores a foreign-only
/// (or empty) document.
fn strip(doc: &mut Value) -> Result<(), String> {
    let Some(root) = doc.as_object_mut() else {
        return Err("the settings root is not a JSON object".into());
    };
    if let Some(hooks) = root.get_mut("hooks").and_then(Value::as_object_mut) {
        for (event, ..) in EVENTS {
            if let Some(groups) = hooks.get_mut(*event).and_then(Value::as_array_mut) {
                strip_ours_from_groups(groups);
                if groups.is_empty() {
                    hooks.remove(*event);
                }
            }
        }
    }
    if root
        .get("hooks")
        .and_then(Value::as_object)
        .is_some_and(serde_json::Map::is_empty)
    {
        root.remove("hooks");
    }
    if root
        .get("statusLine")
        .and_then(|s| s.get("command"))
        .and_then(Value::as_str)
        .is_some_and(is_ours)
    {
        root.remove("statusLine");
    }
    Ok(())
}

/// Drops our commands out of every matcher group; drops groups that
/// end up empty. Foreign commands inside mixed groups survive.
fn strip_ours_from_groups(groups: &mut Vec<Value>) {
    for group in groups.iter_mut() {
        if let Some(cmds) = group.get_mut("hooks").and_then(Value::as_array_mut) {
            cmds.retain(|h| {
                !h.get("command")
                    .and_then(Value::as_str)
                    .is_some_and(is_ours)
            });
        }
    }
    groups.retain(|g| {
        g.get("hooks")
            .and_then(Value::as_array)
            .is_none_or(|c| !c.is_empty())
    });
}

/// One status line per event: `installed` (ours, current exe),
/// `stale` (ours, another exe), `absent`, plus foreign counts.
fn report(doc: &Value, exe: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let hooks = doc.get("hooks");
    for (event, _, verb, _) in EVENTS {
        let mut state = "absent".to_owned();
        let mut foreign = 0usize;
        if let Some(groups) = hooks.and_then(|h| h.get(*event)).and_then(Value::as_array) {
            for group in groups {
                let Some(cmds) = group.get("hooks").and_then(Value::as_array) else {
                    continue;
                };
                for h in cmds {
                    let Some(cmd) = h.get("command").and_then(Value::as_str) else {
                        continue;
                    };
                    if is_ours(cmd) {
                        state = if cmd == quoted(exe, verb) {
                            "installed".to_owned()
                        } else {
                            "stale (another fractality build)".to_owned()
                        };
                    } else {
                        foreign += 1;
                    }
                }
            }
        }
        if foreign > 0 {
            state.push_str(&format!(" · {foreign} foreign entr(y/ies)"));
        }
        lines.push(format!("{event:<16} {state}"));
    }
    let sl = match doc
        .get("statusLine")
        .and_then(|s| s.get("command"))
        .and_then(Value::as_str)
    {
        None => "absent".to_owned(),
        Some(cmd) if cmd == quoted(exe, "statusline") => "installed".to_owned(),
        Some(cmd) if is_ours(cmd) => "stale (another fractality build)".to_owned(),
        Some(_) => "foreign — untouched".to_owned(),
    };
    lines.push(format!("{:<16} {sl}", "statusLine"));
    if doc
        .get("disableAllHooks")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        lines.push("WARNING: disableAllHooks=true — hooks AND the statusline are off".into());
    }
    lines
}

fn settings_path(target: Option<&camino::Utf8Path>, project: bool) -> camino::Utf8PathBuf {
    let dir = target
        .map(camino::Utf8Path::to_path_buf)
        .unwrap_or_default();
    let file = if project {
        "settings.json"
    } else {
        "settings.local.json"
    };
    dir.join(".claude").join(file)
}

fn read_doc(path: &camino::Utf8Path) -> Result<Value, (u8, String)> {
    match std::fs::read_to_string(path.as_std_path()) {
        Ok(text) => serde_json::from_str(&text).map_err(|e| {
            (
                EXIT_NEGATIVE,
                format!(
                    "`{path}` is not valid JSON ({e}) — fix it by hand; \
                     nothing was touched (no auto-repair, by law)"
                ),
            )
        }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(json!({})),
        Err(e) => Err((EXIT_INFRA, format!("reading `{path}`: {e}"))),
    }
}

fn current_exe() -> Result<String, (u8, String)> {
    let exe = std::env::current_exe()
        .map_err(|e| (EXIT_INFRA, format!("resolving the fractality exe: {e}")))?;
    Ok(exe.to_string_lossy().replace('\\', "/"))
}

/// `fractality harness install claude-code [--project] [--target DIR]`.
pub(crate) fn install(target: Option<&camino::Utf8Path>, project: bool) -> u8 {
    let path = settings_path(target, project);
    let (exe, mut doc) = match (current_exe(), read_doc(&path)) {
        (Ok(exe), Ok(doc)) => (exe, doc),
        (Err((c, m)), _) | (_, Err((c, m))) => return fail_code(c, &m),
    };
    let notices = match upsert(&mut doc, &exe) {
        Ok(n) => n,
        Err(m) => return fail_code(EXIT_NEGATIVE, &format!("`{path}`: {m}")),
    };
    if let Some(parent) = path.parent()
        && let Err(e) = std::fs::create_dir_all(parent.as_std_path())
    {
        return fail_code(EXIT_INFRA, &format!("creating `{parent}`: {e}"));
    }
    let text = match serde_json::to_string_pretty(&doc) {
        Ok(t) => t + "\n",
        Err(e) => return fail_code(EXIT_INFRA, &format!("encoding `{path}`: {e}")),
    };
    if let Err(e) = std::fs::write(path.as_std_path(), text) {
        return fail_code(EXIT_INFRA, &format!("writing `{path}`: {e}"));
    }
    println!("installed into {path}");
    for n in notices {
        eprintln!("note: {n}");
    }
    EXIT_OK
}

/// `fractality harness remove claude-code [--project] [--target DIR]`.
pub(crate) fn remove(target: Option<&camino::Utf8Path>, project: bool) -> u8 {
    let path = settings_path(target, project);
    let mut doc = match read_doc(&path) {
        Ok(d) => d,
        Err((c, m)) => return fail_code(c, &m),
    };
    if let Err(m) = strip(&mut doc) {
        return fail_code(EXIT_NEGATIVE, &format!("`{path}`: {m}"));
    }
    let text = match serde_json::to_string_pretty(&doc) {
        Ok(t) => t + "\n",
        Err(e) => return fail_code(EXIT_INFRA, &format!("encoding `{path}`: {e}")),
    };
    if let Err(e) = std::fs::write(path.as_std_path(), text) {
        return fail_code(EXIT_INFRA, &format!("writing `{path}`: {e}"));
    }
    println!("removed from {path}");
    EXIT_OK
}

/// `fractality harness status claude-code [--project] [--target DIR]`.
pub(crate) fn status(target: Option<&camino::Utf8Path>, project: bool) -> u8 {
    let path = settings_path(target, project);
    let (exe, doc) = match (current_exe(), read_doc(&path)) {
        (Ok(exe), Ok(doc)) => (exe, doc),
        (Err((c, m)), _) | (_, Err((c, m))) => return fail_code(c, &m),
    };
    println!("{path}");
    for line in report(&doc, &exe) {
        println!("{line}");
    }
    EXIT_OK
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXE: &str = "C:/tools/fractality.exe";

    #[test]
    fn install_into_an_empty_document_then_remove_restores_empty() {
        let mut doc = json!({});
        let notices = upsert(&mut doc, EXE).expect("upserts");
        assert!(notices.is_empty());
        assert_eq!(
            doc["hooks"]["SessionStart"][0]["hooks"][0]["command"],
            json!("\"C:/tools/fractality.exe\" hook session-start")
        );
        assert_eq!(
            doc["hooks"]["PostToolUse"][0]["matcher"],
            json!("Bash|Edit|Write|MultiEdit|NotebookEdit")
        );
        assert_eq!(doc["statusLine"]["refreshInterval"], json!(30));

        strip(&mut doc).expect("strips");
        assert_eq!(doc, json!({}), "a clean uninstall leaves no residue");
    }

    #[test]
    fn install_is_idempotent_and_updates_a_moved_exe() {
        let mut doc = json!({});
        upsert(&mut doc, EXE).expect("first");
        let once = doc.clone();
        upsert(&mut doc, EXE).expect("second");
        assert_eq!(doc, once, "reinstall is byte-identical");

        upsert(&mut doc, "D:/new/fractality.exe").expect("moved exe");
        assert_eq!(
            doc["hooks"]["SessionStart"][0]["hooks"][0]["command"],
            json!("\"D:/new/fractality.exe\" hook session-start"),
            "stale entries are replaced, not duplicated"
        );
        assert_eq!(
            doc["hooks"]["SessionStart"].as_array().map(Vec::len),
            Some(1)
        );
    }

    #[test]
    fn foreign_entries_survive_install_and_remove() {
        let mut doc = json!({
            "permissions": {"allow": ["Bash(git *)"]},
            "hooks": {
                "SessionStart": [
                    {"hooks": [{"type": "command", "command": "python theirs.py"}]}
                ]
            },
            "statusLine": {"type": "command", "command": "their-status.sh"}
        });
        let notices = upsert(&mut doc, EXE).expect("upserts");
        assert_eq!(notices.len(), 1, "foreign statusline produces a notice");
        assert_eq!(
            doc["statusLine"]["command"],
            json!("their-status.sh"),
            "a foreign statusline is never clobbered"
        );
        assert_eq!(
            doc["hooks"]["SessionStart"].as_array().map(Vec::len),
            Some(2),
            "their group + ours"
        );

        strip(&mut doc).expect("strips");
        assert_eq!(
            doc["hooks"]["SessionStart"][0]["hooks"][0]["command"],
            json!("python theirs.py"),
            "removal leaves the foreign entry alone"
        );
        assert_eq!(doc["permissions"]["allow"][0], json!("Bash(git *)"));
        assert_eq!(doc["statusLine"]["command"], json!("their-status.sh"));
    }

    #[test]
    fn recognition_is_exact_not_heuristic() {
        assert!(is_ours("\"C:/x/fractality.exe\" hook stop"));
        assert!(is_ours("/usr/bin/fractality statusline"));
        assert!(is_ours("\"C:/b/fractality-dev.exe\" hook session-end"));
        assert!(!is_ours("python fractality-fake.py hook stop"));
        assert!(!is_ours("\"C:/x/fractality.exe\" spawn --packet t.toml"));
        assert!(!is_ours("\"C:/x/other.exe\" hook stop"));
        assert!(!is_ours("fractality.exe"));
    }

    #[test]
    fn mixed_groups_lose_only_our_commands() {
        let mut doc = json!({
            "hooks": {
                "Stop": [
                    {"hooks": [
                        {"type": "command", "command": "python theirs.py"},
                        {"type": "command", "command": "\"C:/old/fractality.exe\" hook stop"}
                    ]}
                ]
            }
        });
        upsert(&mut doc, EXE).expect("upserts");
        let groups = doc["hooks"]["Stop"].as_array().expect("array");
        assert_eq!(groups.len(), 2, "their mixed group survives + our group");
        assert_eq!(
            groups[0]["hooks"].as_array().map(Vec::len),
            Some(1),
            "only the stale fractality command left the mixed group"
        );
    }
}
