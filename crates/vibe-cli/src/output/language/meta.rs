//! Metadata after a headline (PROP-067 `#meta`): the ` · ` join, the
//! shortest exact time, sizes, bracketed state words and paths shortened
//! in the middle.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-067#meta");

use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::time::Duration;

use super::roles::{Role, bold, paint};

/// How wide a shortened path may be outside the trailer.
const PATH_WIDTH: usize = 48;

/// The separator between a headline and each metadata item.
pub(super) const SEPARATOR: &str = " · ";

static HOME: RwLock<Option<PathBuf>> = RwLock::new(None);

pub(super) fn set_home(home: Option<PathBuf>) {
    if let Ok(mut held) = HOME.write() {
        *held = home;
    }
}

/// One item of metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Meta {
    /// An ordinary annotation, dim.
    Note(String),
    /// A count or size that changes what the reader does: text, bold.
    Count(String),
    /// A state word, set in brackets at the end of the row, dim.
    State(String),
    /// An elapsed time in its shortest exact form, dim.
    Time(Duration),
    /// An annotation in the alert role (`exit 2`, `failed: …`).
    Alert(String),
    /// An annotation in the warn role (`unencrypted`, `broad`).
    Warn(String),
}

impl Meta {
    /// A note; the most common item, spelled short.
    pub(crate) fn note(text: impl Into<String>) -> Self {
        Self::Note(text.into())
    }

    fn painted(&self) -> String {
        match self {
            Meta::Note(text) => paint(Role::Dim, text),
            Meta::Count(text) => bold(Role::Text, text),
            Meta::State(text) => paint(Role::Dim, &format!("[{text}]")),
            Meta::Time(duration) => paint(Role::Dim, &elapsed(*duration)),
            Meta::Alert(text) => paint(Role::Alert, text),
            Meta::Warn(text) => paint(Role::Warn, text),
        }
    }
}

/// The metadata of one row: every item but the state words after ` · `,
/// then the state words in brackets at the end, two spaces off.
///
/// ```text
/// [Time(3s), Count("12.4 MB"), State("installed")]
///   ->  " · 3s · 12.4 MB  [installed]"
/// ```
#[specmark::spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-067#meta")]
pub(crate) fn join_meta(items: &[Meta]) -> String {
    let mut text = String::new();
    for item in items.iter().filter(|item| !matches!(item, Meta::State(_))) {
        text.push_str(&paint(Role::Dim, SEPARATOR));
        text.push_str(&item.painted());
    }
    for item in items.iter().filter(|item| matches!(item, Meta::State(_))) {
        text.push_str("  ");
        text.push_str(&item.painted());
    }
    text
}

/// The shortest exact form of a duration: `0s`, `3s`, `2m 10s`, `2m`,
/// `1h 4m`.
pub(crate) fn elapsed(duration: Duration) -> String {
    let seconds = duration.as_secs();
    let (hours, minutes, rest) = (seconds / 3600, seconds / 60 % 60, seconds % 60);
    match (hours, minutes, rest) {
        (0, 0, rest) => format!("{rest}s"),
        (0, minutes, 0) => format!("{minutes}m"),
        (0, minutes, rest) => format!("{minutes}m {rest}s"),
        (hours, 0, _) => format!("{hours}h"),
        (hours, minutes, _) => format!("{hours}h {minutes}m"),
    }
}

/// A byte count as a reader says it: `512 B`, `12.4 MB`.
pub(crate) fn bytes(count: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = count as f64;
    let mut unit = 0;
    while value >= 1000.0 && unit < UNITS.len() - 1 {
        value /= 1000.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{count} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

/// A path as metadata: the home spelled `~`, then shortened in the middle
/// to [`PATH_WIDTH`] columns, keeping its head and its last component.
pub(crate) fn short_path(path: &str) -> String {
    shorten(&tilde(path), PATH_WIDTH)
}

/// The home directory spelled `~`.
fn tilde(path: &str) -> String {
    let home = HOME.read().ok().and_then(|home| home.clone());
    match home {
        Some(home) => match Path::new(path).strip_prefix(&home) {
            Ok(rest) if rest.as_os_str().is_empty() => "~".to_owned(),
            Ok(rest) => format!("~/{}", rest.display()),
            Err(_) => path.to_owned(),
        },
        None => path.to_owned(),
    }
}

/// `path` cut in its middle to `width` columns: as many leading components
/// as fit beside the last one, joined by `…`. An identifier is never
/// shortened, and a path that already fits is returned as written.
pub(super) fn shorten(path: &str, width: usize) -> String {
    if path.chars().count() <= width {
        return path.to_owned();
    }
    let separator = if path.contains('\\') && !path.contains('/') {
        '\\'
    } else {
        '/'
    };
    let trailing = path.ends_with(separator);
    let parts: Vec<&str> = path.trim_end_matches(separator).split(separator).collect();
    let Some((last, head)) = parts.split_last() else {
        return path.to_owned();
    };
    let end = if trailing {
        separator.to_string()
    } else {
        String::new()
    };
    let tail = format!("{separator}…{separator}{last}{end}");
    let mut kept = String::new();
    for count in 1..=head.len() {
        let candidate = head[..count].join(&separator.to_string());
        if candidate.chars().count() + tail.chars().count() > width {
            break;
        }
        kept = candidate;
    }
    format!("{kept}{tail}")
}
