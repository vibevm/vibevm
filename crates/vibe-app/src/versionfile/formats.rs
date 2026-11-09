//! The two version-file formats and what one line asks for
//! (PROP-075 `#VERSION-FILE-FORMATS`). Pure text in, data out: nothing here
//! touches the filesystem, so every shape is a unit test.

use vibe_core::VersionSpec;

/// The channel grammar of PROP-075 `#CHANNEL-SYNTAX`: `[a-z][a-z0-9.-]*`,
/// optionally one `/` and a second such segment.
pub fn is_channel_name(text: &str) -> bool {
    let segment = |part: &str| {
        let mut chars = part.chars();
        chars.next().is_some_and(|first| first.is_ascii_lowercase())
            && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '-')
    };
    match text.split_once('/') {
        Some((first, second)) => segment(first) && segment(second),
        None => segment(text),
    }
}

/// What a version file's line, or a `SLOT@VER` argument, asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestKind {
    /// An ordinary version requirement (`22`, `^22`, `=22.23.4`, `22.x`).
    Spec(VersionSpec),
    /// A channel name (`lts`, `lts/jod`, `latest`, `stable`), lowered by
    /// PROP-075 `#CHANNEL-SYNTAX`.
    Channel(String),
}

/// One request with the spelling it was authored in, so a report can quote
/// the file rather than a normalised form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionRequest {
    pub authored: String,
    pub kind: RequestKind,
}

impl VersionRequest {
    /// Read one line. A requirement is tried first and only a string that
    /// fails `semver::VersionReq` **and** matches the channel grammar
    /// becomes a channel, so no existing requirement changes meaning
    /// (PROP-075 `#CHANNEL-SYNTAX`). A single leading `v` is tolerated
    /// before the requirement, because `v22.23.4` is the spelling upstreams
    /// and `.nvmrc` files actually carry.
    pub fn parse(text: &str) -> Result<Self, String> {
        let authored = text.trim().to_owned();
        if authored.is_empty() {
            return Err("empty version request".to_owned());
        }
        let bare = authored
            .strip_prefix('v')
            .filter(|rest| rest.starts_with(|c: char| c.is_ascii_digit()))
            .unwrap_or(&authored);
        if let Ok(spec) = VersionSpec::parse(bare) {
            return Ok(Self {
                authored,
                kind: RequestKind::Spec(spec),
            });
        }
        if is_channel_name(&authored) {
            return Ok(Self {
                kind: RequestKind::Channel(authored.clone()),
                authored,
            });
        }
        Err(format!(
            "`{authored}` is neither a version requirement nor a channel name \
             ([a-z][a-z0-9.-]* with at most one `/`)"
        ))
    }

    /// The channel this request names, if any.
    pub fn channel(&self) -> Option<&str> {
        match &self.kind {
            RequestKind::Channel(name) => Some(name),
            RequestKind::Spec(_) => None,
        }
    }

    /// The version requirement this request names, if any.
    pub fn spec(&self) -> Option<&VersionSpec> {
        match &self.kind {
            RequestKind::Spec(spec) => Some(spec),
            RequestKind::Channel(_) => None,
        }
    }
}

/// The content of a `plain` version file: its first non-empty, non-comment
/// line, trimmed, with a `#` comment and a CR tolerated. `None` for a file
/// that holds nothing but blanks and comments.
pub fn plain_line(text: &str) -> Option<&str> {
    text.lines()
        .map(|line| line.strip_suffix('\r').unwrap_or(line))
        .map(|line| line.split('#').next().unwrap_or("").trim())
        .find(|line| !line.is_empty())
}

/// What a `.tool-versions` file says about one key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolVersions {
    /// The first version listed for the key — the one that is taken.
    pub taken: String,
    /// The rest, reported as unused (PROP-075 `#VERSION-FILE-FORMATS`).
    pub unused: Vec<String>,
}

/// Read asdf's `.tool-versions` for `key`: whitespace-separated
/// `<key> <version>…` lines, `#` comments ignored, the **first** version
/// listed for the key taken and the rest reported as unused. A key line
/// with no version at all is not a selection; `None` then, so the upward
/// search keeps walking rather than failing a prompt.
pub fn tool_versions(text: &str, key: &str) -> Option<ToolVersions> {
    for line in text.lines() {
        let line = line.strip_suffix('\r').unwrap_or(line);
        let line = line.split('#').next().unwrap_or("");
        let mut words = line.split_whitespace();
        if words.next() != Some(key) {
            continue;
        }
        let mut versions = words.map(str::to_owned);
        let taken = versions.next()?;
        return Some(ToolVersions {
            taken,
            unused: versions.collect(),
        });
    }
    None
}
