//! `vibe-index config <data-dir>` — print the effective value of
//! every configuration-ladder member with the source of each: flag >
//! env (`VIBE_INDEX_*`) > `<data-dir>/state/config.toml` > built-in
//! default (PROP-005 §3.5, `BACKLOG.md` B-086). Same shape the
//! consumer side's `vibe show config` uses — per-value provenance —
//! because a ladder an operator cannot interrogate adds a way to be
//! wrong without a way to notice.
//!
//! The log-level row shows this invocation's own `--log-level` as the
//! flag rung when it was passed; the other members' flag rungs
//! (`--api-base`, `--format`) belong to their own verbs, so those
//! rows resolve from env / file / default here.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root");

use std::path::{Path, PathBuf};

use clap::Parser;
use serde::Serialize;

use super::LogLevel;
use crate::config::{self, Ladder, Member, Resolved};
use crate::error::{Error, Result};

#[derive(Debug, Parser)]
#[command(about = "Print effective configuration values with the source of each.")]
pub struct Args {
    pub data_dir: PathBuf,

    /// Emit JSON instead of human-readable text.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Serialize)]
struct ConfigFileSummary {
    /// Path the file rung was read from (or would be).
    path: String,
    /// `false` = absent — legal, the common case; defaults apply.
    present: bool,
}

#[derive(Debug, Serialize)]
struct MemberReport {
    /// The `config.toml` key.
    key: &'static str,
    /// The flag spelling of the top rung, when one exists.
    flag: Option<&'static str>,
    /// Env names of the env rung, narrowest first.
    env: &'static [&'static str],
    /// Effective value.
    value: String,
    /// Which rung supplied it — `"flag --log-level"` /
    /// `"env VIBE_INDEX_GIT"` / `"config file <path>"` / `"default"`.
    source: String,
}

#[derive(Debug, Serialize)]
struct Report {
    ok: bool,
    command: &'static str,
    data_dir: String,
    config_file: ConfigFileSummary,
    /// The precedence the members were resolved by, top rung first.
    precedence: [&'static str; 4],
    members: Vec<MemberReport>,
}

pub fn run(args: Args, log_level: Option<LogLevel>) -> Result<()> {
    let ladder = Ladder::load(&args.data_dir)?;
    let report = build_report(&ladder, log_level, &config::live_env, &args.data_dir)?;

    if args.json {
        let payload = serde_json::to_string_pretty(&report)
            .map_err(|e| Error::Malformed(format!("could not serialise config report: {e}")))?;
        println!("{payload}");
        return Ok(());
    }
    print!("{}", render(&report));
    Ok(())
}

/// Resolve every member through the SAME resolvers the wired verbs
/// use, so what this verb prints is what the verbs act on — and an
/// invalid value refuses here exactly as it would there. `env` is
/// injected for the tests; production passes [`config::live_env`].
fn build_report(
    ladder: &Ladder,
    log_level: Option<LogLevel>,
    env: &dyn Fn(&str) -> Option<String>,
    data_dir: &Path,
) -> Result<Report> {
    let rows: Vec<Resolved> = vec![
        config::resolve_log_filter(ladder, log_level, env)?,
        config::resolve_git(ladder, env)?,
        config::resolve_api_base(ladder, None, env)?,
        config::resolve_dump_format(ladder, None, env)?.1,
    ];
    debug_assert_eq!(rows.len(), Member::ALL.len());
    let members: Vec<MemberReport> = Member::ALL
        .iter()
        .zip(rows)
        .map(|(m, r)| MemberReport {
            key: m.key,
            flag: m.flag,
            env: m.env_names,
            value: r.value,
            source: r.source.label(),
        })
        .collect();

    Ok(Report {
        ok: true,
        command: "config",
        data_dir: data_dir.display().to_string(),
        config_file: ConfigFileSummary {
            path: ladder.path().display().to_string(),
            present: ladder.file_present(),
        },
        precedence: ["flag", "env", "config-file", "default"],
        members,
    })
}

/// The human-readable rendering — pure, so the snapshot test asserts
/// on the exact string without capturing stdout.
fn render(report: &Report) -> String {
    let mut out = String::new();
    let state = if report.config_file.present {
        "present"
    } else {
        "absent — legal; defaults apply"
    };
    out.push_str(&format!(
        "Config file: {}  ({state})\n\n",
        report.config_file.path
    ));
    out.push_str("Ladder members (flag > env VIBE_INDEX_* > config file > default):\n");
    let width = report
        .members
        .iter()
        .map(|m| m.key.len())
        .max()
        .unwrap_or(0);
    for m in &report.members {
        out.push_str(&format!(
            "  {:<width$} = {}  [source: {}]\n",
            m.key, m.value, m.source
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_of(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> + use<> {
        let pairs: Vec<(String, String)> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |name| {
            pairs
                .iter()
                .find(|(k, _)| k.as_str() == name)
                .map(|(_, v)| v.clone())
        }
    }

    fn ladder_with(body: &str) -> Ladder {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        std::fs::write(state.join("config.toml"), body).unwrap();
        Ladder::load(dir.path()).unwrap()
    }

    /// The snapshot the packet asks for: every member on its own line,
    /// each with the rung that supplied it.
    #[test]
    fn human_render_names_every_member_with_its_source() {
        let ladder = ladder_with("dump-format = \"json\"\n");
        let report = build_report(
            &ladder,
            Some(LogLevel::Debug),
            &env_of(&[("VIBE_INDEX_GIT", "C:/tools/git.exe")]),
            Path::new("D:/idx"),
        )
        .unwrap();

        let text = render(&report);
        assert!(text.contains("Config file:"), "{text}");
        assert!(text.contains("dump-format = json"), "{text}");
        // All four members, four distinct sources.
        assert!(text.contains("log-level   = debug"), "{text}");
        assert!(text.contains("[source: flag --log-level]"), "{text}");
        assert!(text.contains("git         = C:/tools/git.exe"), "{text}");
        assert!(text.contains("[source: env VIBE_INDEX_GIT]"), "{text}");
        assert!(
            text.contains("api-base    = https://api.github.com"),
            "{text}"
        );
        assert!(text.contains("[source: default]"), "{text}");
        assert!(text.contains("[source: config file"), "{text}");
    }

    #[test]
    fn report_carries_all_members_in_declared_order() {
        let dir = tempfile::tempdir().unwrap();
        let ladder = Ladder::load(dir.path()).unwrap();
        let report = build_report(&ladder, None, &env_of(&[]), dir.path()).unwrap();
        let keys: Vec<&str> = report.members.iter().map(|m| m.key).collect();
        assert_eq!(keys, ["log-level", "git", "api-base", "dump-format"]);
        assert!(!report.config_file.present);
        assert_eq!(report.precedence, ["flag", "env", "config-file", "default"]);
    }

    #[test]
    fn an_invalid_member_value_refuses_instead_of_rendering() {
        let ladder = ladder_with("log-level = \"loud\"\n");
        assert!(build_report(&ladder, None, &env_of(&[]), Path::new("D:/idx")).is_err());
    }
}
