//! The version-file formats and what one line asks for
//! (PROP-075 `#VERSION-FILE-FORMATS`, `#VERSION-FILE-KEYED`). Pure text in,
//! data out: nothing here touches the filesystem, so every shape is a unit
//! test.
//!
//! There is one keyed reader, because `tool-versions` is the short spelling
//! of one `keyed` row and `vibe_core` expands it before a row reaches this
//! module. The same line splitter answers for the writer, so what it
//! preserves and what the reader takes can never disagree.

use specmark::spec;
use vibe_core::VersionSpec;
use vibe_core::manifest::{KeyedExtra, KeyedSeparator, KeyedSyntax};

/// The channel grammar of PROP-075 `#CHANNEL-SYNTAX`: `[a-z][a-z0-9.-]*`,
/// optionally one `/` and a second such segment.
pub fn is_channel_name(text: &str) -> bool {
    // One grammar, in one place: the manifest layer's own, which
    // `[[template.channel]]` validation and `VersionSpec::parse` both
    // read (PROP-075 `#CHANNEL-SYNTAX`).
    vibe_core::manifest::channel_name_is_valid(text)
}

/// What a version file's line, or a `SLOT@VER` argument, asks for.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-FORMATS")]
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
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-FORMATS")]
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
        // `VersionSpec::parse` itself tries `semver::VersionReq` first and
        // only then the channel grammar (PROP-075 `#CHANNEL-SYNTAX`), so
        // one parser decides, and a channel it recognises is carried here
        // as a channel rather than as a constraint that matches nothing.
        if let Ok(spec) = VersionSpec::parse(bare) {
            return Ok(match spec {
                VersionSpec::Channel(channel) => Self {
                    kind: RequestKind::Channel(channel.as_str().to_owned()),
                    authored,
                },
                spec => Self {
                    authored,
                    kind: RequestKind::Spec(spec),
                },
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

/// What a keyed version file says about one key
/// (PROP-075 `#VERSION-FILE-KEYED`).
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-KEYED")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyedSelection {
    /// The first version on the key's line — the one that is taken.
    pub taken: String,
    /// The rest, reported as unused under `extra = "first"`.
    pub unused: Vec<String>,
    /// The 1-based line the key was found on, so a report, a refusal and
    /// the writer can name it.
    pub line: usize,
}

/// One line of a keyed file as the regions a writer must keep apart, so the
/// value can be replaced and every other byte of the line stays where it
/// was (PROP-075 `#VERSION-FILE-WRITE`).
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-WRITE")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyedLine<'a> {
    /// The 1-based line number.
    pub line: usize,
    /// Everything up to the first value: the indent, the key as written,
    /// the separator and the spacing around it.
    pub head: &'a str,
    /// The values, without surrounding whitespace.
    pub values: &'a str,
    /// Whatever follows them — the spacing and the line's comment.
    pub tail: &'a str,
}

impl KeyedLine<'_> {
    /// The versions this line lists after the key, in order.
    pub fn versions(&self) -> Vec<&str> {
        self.values.split_whitespace().collect()
    }

    /// This line with `version` in place of its values, which is the whole
    /// of a keyed write (PROP-075 `#VERSION-FILE-WRITE`).
    pub fn rewritten(&self, version: &str) -> String {
        format!("{}{version}{}", self.head, self.tail)
    }
}

/// Every line of `text` that carries `key` under `syntax`, in file order. A
/// line whose key is another slot's is not here and is **not** a fault:
/// another key in a shared file is not a defect
/// (PROP-075 `#VERSION-FILE-KEYED`).
pub fn keyed_lines<'a>(text: &'a str, key: &str, syntax: KeyedSyntax) -> Vec<KeyedLine<'a>> {
    text.lines()
        .enumerate()
        .filter_map(|(index, raw)| split_keyed(raw, key, syntax, index + 1))
        .collect()
}

/// One line split at its key, or `None` when the line does not carry `key`.
fn split_keyed<'a>(
    raw: &'a str,
    key: &str,
    syntax: KeyedSyntax,
    line: usize,
) -> Option<KeyedLine<'a>> {
    let raw = raw.strip_suffix('\r').unwrap_or(raw);
    let body = match syntax.comment.character().and_then(|ch| raw.find(ch)) {
        Some(at) => &raw[..at],
        None => raw,
    };
    let start = body.len() - body.trim_start().len();
    let after = match syntax.separator {
        KeyedSeparator::Whitespace => {
            let word = body[start..].split_whitespace().next()?;
            if word != key {
                return None;
            }
            start + word.len()
        }
        separator => {
            let at = body.find(separator.character())?;
            if body[start..at].trim_end() != key {
                return None;
            }
            at + separator.character().len_utf8()
        }
    };
    let region = &body[after..];
    let lead = region.len() - region.trim_start().len();
    let head = &raw[..after + lead];
    let values = region[lead..].trim_end();
    Some(KeyedLine {
        line,
        head,
        values,
        tail: &raw[head.len() + values.len()..],
    })
}

/// Read a keyed file for `key`: the key's line under the declared syntax,
/// the **first** value taken and the rest reported as unused under
/// `extra = "first"`. `Err` is a reported, unusable line — what
/// `extra = "refuse"` says a second value is. A key line with no value at
/// all is not a selection; `Ok(None)` then, so the upward search keeps
/// walking rather than failing a prompt (PROP-075 `#VERSION-FILE-KEYED`).
pub fn keyed(text: &str, key: &str, syntax: KeyedSyntax) -> Result<Option<KeyedSelection>, String> {
    for found in keyed_lines(text, key, syntax) {
        let versions = found.versions();
        let Some((taken, unused)) = versions.split_first() else {
            continue;
        };
        if !unused.is_empty() && syntax.extra == KeyedExtra::Refuse {
            return Err(format!(
                "line {} lists {} after `{taken}` for key `{key}`, and the format declares \
                 `extra = \"refuse\"`; the line is unusable",
                found.line,
                unused
                    .iter()
                    .map(|version| format!("`{version}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        return Ok(Some(KeyedSelection {
            taken: (*taken).to_owned(),
            unused: unused.iter().map(|value| (*value).to_owned()).collect(),
            line: found.line,
        }));
    }
    Ok(None)
}
