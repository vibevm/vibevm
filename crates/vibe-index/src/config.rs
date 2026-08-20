//! The four-rung configuration ladder — PROP-005 §3.5
//! (`##CONFIG-PRECEDENCE`, owner ruling 2026-08-20 / `BACKLOG.md`
//! B-086). For every configurable member:
//!
//! > explicit CLI flag > env (`VIBE_INDEX_*`) >
//! > `<data-dir>/state/config.toml` (optional) > built-in default
//!
//! and every resolution carries BOTH the value and the source it came
//! from — the "visible source" half the ruling requires, surfaced by
//! the `vibe-index config <data-dir>` verb. A ladder without a way to
//! see which rung supplied a value adds a way to be wrong without a
//! way to notice; that is why [`Source`] rides next to every value.
//!
//! The natural order of the rungs follows the data directory. The
//! file rung lives at `<data-dir>/state/config.toml`, and `<data-dir>`
//! is a required positional on every verb (`##CLI-SURFACE`), so the
//! ladder can only be read AFTER the command line is parsed. In the
//! binary that is: parse → [`Ladder::load`] → resolve the logging
//! member → install the subscriber → dispatch; in a wired verb, the
//! same load happens at the top of `run()`. `data-dir` itself is NOT
//! a member: it is the required positional, and the file lives inside
//! it — the ladder applies only to what has a default.
//!
//! The file is parsed strictly: an unknown key is a loud refusal, not
//! a silent no-op, because a typo'd key otherwise reads as configured
//! while doing nothing — the exact middle state B-086 was filed to
//! end. The known-key set is exactly the members this build resolves;
//! wiring a new member grows the set in one place ([`Member::ALL`]).
//!
//! `VIBE_LOG` is the one recorded exception to the `VIBE_INDEX_*`
//! family naming: it predates the ladder, is documented in `main.rs`
//! and on the consumer side (`vibe show config`), and keeps its full
//! `EnvFilter` directive power. It sits BELOW `VIBE_INDEX_LOG` (the
//! narrow, family-named lever beats the broad one) and ABOVE the file
//! rung (any env beats any file), so nothing that ever worked breaks.

specmark::scope!("spec://org.vibevm.core/vibevm/modules/vibe-index/PROP-005#root");

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

/// The file rung's location: `<data-dir>/state/config.toml`.
pub fn file_path(data_dir: &Path) -> PathBuf {
    data_dir.join("state").join("config.toml")
}

/// Trim, then treat empty as unset — the one dialect every env read
/// in the ladder uses, split out of [`live_env`] so the rule itself is
/// testable without touching the process environment. A
/// `VIBE_INDEX_GIT=` literal in a shell profile must not shadow the
/// rungs below it (the same dialect `vibe-cli` uses for
/// `VIBE_INVOKED_BY`).
fn normalise_env(raw: String) -> Option<String> {
    let trimmed = raw.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

/// Read one environment variable the ladder's way (see
/// [`normalise_env`]).
pub fn live_env(name: &str) -> Option<String> {
    std::env::var(name).ok().and_then(normalise_env)
}

/// Where a resolved value came from. Rendered by the `config` verb so
/// an operator can see which rung supplied every effective value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// An explicit CLI flag; carries its spelling.
    Flag(&'static str),
    /// An environment variable; carries its name.
    Env(String),
    /// The `<data-dir>/state/config.toml` rung; carries the path read.
    ConfigFile(PathBuf),
    /// The built-in default — the last rung, always present.
    Default,
}

impl Source {
    /// Consumer-side shape (`vibe show config`): a short label naming
    /// the rung, specific enough to point at the exact knob.
    pub fn label(&self) -> String {
        match self {
            Source::Flag(flag) => format!("flag {flag}"),
            Source::Env(name) => format!("env {name}"),
            Source::ConfigFile(path) => format!("config file {}", path.display()),
            Source::Default => "default".to_string(),
        }
    }
}

/// One resolved member: the effective value AND where it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    pub value: String,
    pub source: Source,
}

/// One ladder member's identity: its key in `config.toml`, the env
/// names feeding the env rung (narrowest first), the flag spelling for
/// display, and the built-in default that is always the last rung.
/// [`Member::ALL`] is the set this build resolves — the strict file
/// check and the `config` verb both read it, so a new member becomes
/// known everywhere by joining that one list.
pub struct Member {
    pub key: &'static str,
    pub env_names: &'static [&'static str],
    pub flag: Option<&'static str>,
    pub default: &'static str,
}

impl Member {
    /// The logging dial (`--log-level`). `VIBE_INDEX_LOG` is the
    /// family-named lever and takes the same closed set as the flag;
    /// `VIBE_LOG` is the recorded legacy exception and keeps its full
    /// `EnvFilter` directive power (see the module docs).
    pub const LOG_LEVEL: Member = Member {
        key: "log-level",
        env_names: &["VIBE_INDEX_LOG", "VIBE_LOG"],
        flag: Some("--log-level"),
        default: "warn",
    };

    /// The git binary the scanner shells out to. No flag rung exists
    /// today (`##CLI-SURFACE` has no `--git`); the member enters the
    /// ladder at the env rung, where `VIBE_INDEX_GIT` already lived.
    pub const GIT: Member = Member {
        key: "git",
        env_names: &["VIBE_INDEX_GIT"],
        flag: None,
        default: "git",
    };

    /// GitHub REST API base for `reindex --from-github` and
    /// `rescan-org` — one member, two verbs, one key.
    pub const API_BASE: Member = Member {
        key: "api-base",
        env_names: &["VIBE_INDEX_API_BASE"],
        flag: Some("--api-base"),
        default: "https://api.github.com",
    };

    /// `dump --format`.
    pub const DUMP_FORMAT: Member = Member {
        key: "dump-format",
        env_names: &["VIBE_INDEX_DUMP_FORMAT"],
        flag: Some("--format"),
        default: "jsonl",
    };

    /// Every member this build resolves, in display order. This list
    /// is also the strict known-key set the config file is checked
    /// against.
    pub const ALL: &[Member] = &[
        Self::LOG_LEVEL,
        Self::GIT,
        Self::API_BASE,
        Self::DUMP_FORMAT,
    ];
}

/// The loaded file rung. `values = None` means the file is absent —
/// legal, the common case, and indistinguishable from an empty file
/// as far as any resolution is concerned.
#[derive(Debug)]
pub struct Ladder {
    path: PathBuf,
    values: Option<toml::Table>,
}

impl Ladder {
    /// A ladder whose file rung is known absent — for callers that
    /// have no data directory to read (none exist today: every verb
    /// carries the positional).
    pub fn absent() -> Ladder {
        Ladder {
            path: PathBuf::new(),
            values: None,
        }
    }

    /// Load `<data-dir>/state/config.toml`. An absent file is legal
    /// (`values = None`). A present file must parse and must carry
    /// only known keys with string values — anything else is a loud
    /// [`Error::InvalidInput`] naming the file, so a broken or
    /// misspelled config layer can never sit silently inert under a
    /// running binary.
    pub fn load(data_dir: &Path) -> Result<Ladder> {
        let path = file_path(data_dir);
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Ladder { path, values: None });
            }
            Err(e) => {
                return Err(Error::Io {
                    path: path.clone(),
                    message: e.to_string(),
                });
            }
        };
        let table: toml::Table = toml::from_str(&text).map_err(|e| {
            Error::InvalidInput(format!(
                "config file `{}` does not parse as TOML: {e}",
                path.display()
            ))
        })?;
        let known: Vec<&str> = Member::ALL.iter().map(|m| m.key).collect();
        for (key, value) in &table {
            if !known.contains(&key.as_str()) {
                return Err(Error::InvalidInput(format!(
                    "config file `{}` sets unknown key `{key}` — this build resolves {}. \
                     Remove the key or update vibe-index: a key this build does not know \
                     must refuse loudly, not sit silently inert",
                    path.display(),
                    known.join(", ")
                )));
            }
            if value.as_str().is_none() {
                return Err(Error::InvalidInput(format!(
                    "config file `{}` sets `{key}` to a non-string value ({value:?}) — \
                     every ladder member is a string",
                    path.display()
                )));
            }
        }
        Ok(Ladder {
            path,
            values: Some(table),
        })
    }

    /// The path the file rung was (or would be) read from.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Whether the file rung was present.
    pub fn file_present(&self) -> bool {
        self.values.is_some()
    }

    /// Resolve one member: flag > env (narrowest name first) > file >
    /// default. `flag` is the CLI value when the operator passed one
    /// (trimmed; empty counts as not passed). `env` is injected so
    /// tests resolve against a map instead of the process
    /// environment — production callers pass [`live_env`].
    pub fn resolve(
        &self,
        member: &Member,
        flag: Option<&str>,
        env: &dyn Fn(&str) -> Option<String>,
    ) -> Resolved {
        // The flag rung exists only for members that HAVE a flag — the
        // `if let` pairing makes a flag value for a flagless member
        // structurally unable to claim this rung (it falls through to
        // env), so no panic path exists here.
        if let (Some(flag_name), Some(value)) =
            (member.flag, flag.map(str::trim).filter(|v| !v.is_empty()))
        {
            return Resolved {
                value: value.to_string(),
                source: Source::Flag(flag_name),
            };
        }
        for name in member.env_names {
            if let Some(value) = env(name) {
                return Resolved {
                    value,
                    source: Source::Env((*name).to_string()),
                };
            }
        }
        if let Some(table) = &self.values
            && let Some(value) = table.get(member.key).and_then(toml::Value::as_str)
        {
            return Resolved {
                value: value.to_string(),
                source: Source::ConfigFile(self.path.clone()),
            };
        }
        Resolved {
            value: member.default.to_string(),
            source: Source::Default,
        }
    }
}

/// Resolve the logging member for the subscriber. `VIBE_INDEX_LOG`
/// and the file key take the flag's closed set (off error warn info
/// debug trace); a value outside it is a loud refusal. `VIBE_LOG`
/// passes through verbatim — it speaks the full `EnvFilter` directive
/// language and always did, and the subscriber keeps its old `warn`
/// fallback for a directive that does not parse.
pub fn resolve_log_filter(
    ladder: &Ladder,
    flag: Option<crate::cli::LogLevel>,
    env: &dyn Fn(&str) -> Option<String>,
) -> Result<Resolved> {
    let resolved = ladder.resolve(&Member::LOG_LEVEL, flag.map(|l| l.as_filter()), env);
    match &resolved.source {
        // `VIBE_LOG` keeps its directive language — see the module docs.
        Source::Env(name) if name == "VIBE_LOG" => Ok(resolved),
        _ => {
            let level = crate::cli::LogLevel::parse_member(&resolved.value).ok_or_else(|| {
                Error::InvalidInput(format!(
                    "log-level value `{}` (from {}) is not one of: off, error, warn, info, debug, trace",
                    resolved.value,
                    resolved.source.label()
                ))
            })?;
            Ok(Resolved {
                value: level.as_filter().to_string(),
                source: resolved.source,
            })
        }
    }
}

/// Resolve the git-binary member. `git` has no flag rung (no `--git`
/// exists), so env > file > default. The value must be non-empty — an
/// empty binary path is a configuration that cannot work.
pub fn resolve_git(ladder: &Ladder, env: &dyn Fn(&str) -> Option<String>) -> Result<Resolved> {
    let resolved = ladder.resolve(&Member::GIT, None, env);
    require_non_empty(&Member::GIT, &resolved)?;
    Ok(resolved)
}

/// Resolve the api-base member (`--api-base` is the caller's flag
/// rung). Non-empty — an empty API base cannot be a URL.
pub fn resolve_api_base(
    ladder: &Ladder,
    flag: Option<&str>,
    env: &dyn Fn(&str) -> Option<String>,
) -> Result<Resolved> {
    let resolved = ladder.resolve(&Member::API_BASE, flag, env);
    require_non_empty(&Member::API_BASE, &resolved)?;
    Ok(resolved)
}

/// Resolve the dump-format member (`--format` is the caller's flag
/// rung). The value must name a format this build can emit.
pub fn resolve_dump_format(
    ladder: &Ladder,
    flag: Option<crate::cli::dump::DumpFormat>,
    env: &dyn Fn(&str) -> Option<String>,
) -> Result<(crate::cli::dump::DumpFormat, Resolved)> {
    let resolved = ladder.resolve(&Member::DUMP_FORMAT, flag.map(|f| f.as_str()), env);
    let parsed = crate::cli::dump::DumpFormat::parse_member(&resolved.value).ok_or_else(|| {
        Error::InvalidInput(format!(
            "dump-format value `{}` (from {}) is not one of: jsonl, json",
            resolved.value,
            resolved.source.label()
        ))
    })?;
    Ok((parsed, resolved))
}

/// A member whose resolved value is empty refuses loudly — an empty
/// string is not a usable value for any member this build carries.
fn require_non_empty(member: &Member, resolved: &Resolved) -> Result<()> {
    if resolved.value.trim().is_empty() {
        return Err(Error::InvalidInput(format!(
            "value for `{}` (from {}) is empty — every ladder member needs a non-empty value",
            member.key,
            resolved.source.label()
        )));
    }
    Ok(())
}

#[cfg(test)]
#[path = "config/tests.rs"]
mod tests;
