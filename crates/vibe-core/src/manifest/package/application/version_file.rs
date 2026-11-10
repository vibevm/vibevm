//! `[[application.version_file]]` — the per-project version files a package
//! understands (PROP-075 `#VERSION-FILE-FORMATS`, `#VERSION-FILE-KEYED`).
//!
//! The set of formats is closed, deliberately: a `.python-version` is a
//! `plain` row and a `package.json` `engines` field is not read at all. A
//! package that needs more waits for a fact amending
//! `PROP-075#VERSION-FILE-FORMATS`.
//!
//! `tool-versions` is the **short spelling** of exactly one `keyed` row —
//! `separator = "whitespace"`, `comment = "hash"`, `extra = "first"` — and
//! the expansion happens here, in the deserialiser, so every reader and the
//! one writer see a `keyed` row and the two spellings cannot drift
//! (PROP-075 `#VERSION-FILE-KEYED`). The conversion back is the same
//! function read the other way, so a manifest that authored the short
//! spelling serialises to the short spelling again.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-FORMATS");

use serde::{Deserialize, Serialize};
use specmark::spec;

/// Where a keyed line puts the version after the key
/// (PROP-075 `#VERSION-FILE-KEYED`). No default: an author states it.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-KEYED")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyedSeparator {
    /// `key=value` — `.sdkmanrc`.
    Equals,
    /// `key value…` — `.tool-versions`.
    Whitespace,
    /// `key: value`.
    Colon,
}

impl KeyedSeparator {
    const VOCABULARY: &'static [(&'static str, Self)] = &[
        ("equals", Self::Equals),
        ("whitespace", Self::Whitespace),
        ("colon", Self::Colon),
    ];

    /// The declared spelling of this separator.
    ///
    /// ```
    /// use vibe_core::manifest::KeyedSeparator;
    /// assert_eq!(KeyedSeparator::Equals.spelling(), "equals");
    /// ```
    pub fn spelling(self) -> &'static str {
        match self {
            Self::Equals => "equals",
            Self::Whitespace => "whitespace",
            Self::Colon => "colon",
        }
    }

    /// The character a line's key and its values are separated by.
    pub fn character(self) -> char {
        match self {
            Self::Equals => '=',
            Self::Whitespace => ' ',
            Self::Colon => ':',
        }
    }

    /// How a line this writer creates spells the separator — the spelling
    /// each of the three formats actually uses in the wild.
    ///
    /// ```
    /// use vibe_core::manifest::KeyedSeparator;
    /// assert_eq!(KeyedSeparator::Colon.written(), ": ");
    /// ```
    pub fn written(self) -> &'static str {
        match self {
            Self::Equals => "=",
            Self::Whitespace => " ",
            Self::Colon => ": ",
        }
    }
}

/// What starts a comment on a keyed line (PROP-075 `#VERSION-FILE-KEYED`).
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-KEYED")]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum KeyedComment {
    /// `#` — the default.
    #[default]
    Hash,
    /// `;`.
    Semicolon,
    /// The format has no comments; a `#` is part of the value.
    None,
}

impl KeyedComment {
    const VOCABULARY: &'static [(&'static str, Self)] = &[
        ("hash", Self::Hash),
        ("semicolon", Self::Semicolon),
        ("none", Self::None),
    ];

    /// The declared spelling of this comment syntax.
    ///
    /// ```
    /// use vibe_core::manifest::KeyedComment;
    /// assert_eq!(KeyedComment::default().spelling(), "hash");
    /// ```
    pub fn spelling(self) -> &'static str {
        match self {
            Self::Hash => "hash",
            Self::Semicolon => "semicolon",
            Self::None => "none",
        }
    }

    /// The character a comment starts with, for a format that has them.
    pub fn character(self) -> Option<char> {
        match self {
            Self::Hash => Some('#'),
            Self::Semicolon => Some(';'),
            Self::None => None,
        }
    }
}

/// What a second value on the key's line means
/// (PROP-075 `#VERSION-FILE-KEYED`).
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-KEYED")]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum KeyedExtra {
    /// The line is reported and unusable — the default, and what
    /// `java=21.0.4-tem extra` should be.
    #[default]
    Refuse,
    /// Take the first value and report the rest unused — asdf's rule.
    First,
}

impl KeyedExtra {
    const VOCABULARY: &'static [(&'static str, Self)] =
        &[("refuse", Self::Refuse), ("first", Self::First)];

    /// The declared spelling of this rule.
    ///
    /// ```
    /// use vibe_core::manifest::KeyedExtra;
    /// assert_eq!(KeyedExtra::First.spelling(), "first");
    /// ```
    pub fn spelling(self) -> &'static str {
        match self {
            Self::Refuse => "refuse",
            Self::First => "first",
        }
    }
}

/// One word of a closed vocabulary, by its declared spelling.
fn parse_word<T: Copy>(spelling: &str, vocabulary: &[(&str, T)]) -> Option<T> {
    vocabulary
        .iter()
        .find(|(declared, _)| *declared == spelling)
        .map(|(_, word)| *word)
}

/// The line syntax of a `keyed` version file, declared from a closed
/// vocabulary and never as a pattern (PROP-075 `#VERSION-FILE-KEYED`).
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-KEYED")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyedSyntax {
    pub separator: KeyedSeparator,
    pub comment: KeyedComment,
    pub extra: KeyedExtra,
}

impl KeyedSyntax {
    /// The syntax asdf's `.tool-versions` carries, which is the one the
    /// short spelling expands to.
    pub const TOOL_VERSIONS: Self = Self {
        separator: KeyedSeparator::Whitespace,
        comment: KeyedComment::Hash,
        extra: KeyedExtra::First,
    };
}

/// One version-file declaration of an application package.
///
/// ```
/// use vibe_core::manifest::VersionFileDecl;
///
/// let plain: VersionFileDecl =
///     toml::from_str("format = \"plain\"\nnames = [\".nvmrc\", \".node-version\"]").unwrap();
/// assert_eq!(plain.names(), [".nvmrc", ".node-version"]);
/// // `tool-versions` is the short spelling of one keyed row, expanded here.
/// let asdf: VersionFileDecl =
///     toml::from_str("format = \"tool-versions\"\nkey = \"nodejs\"").unwrap();
/// assert_eq!(asdf.names(), [".tool-versions"]);
/// assert_eq!(asdf.format(), "tool-versions");
/// let long: VersionFileDecl = toml::from_str(
///     "format = \"keyed\"\nnames = [\".tool-versions\"]\nkey = \"nodejs\"\n\
///      separator = \"whitespace\"\ncomment = \"hash\"\nextra = \"first\"",
/// )
/// .unwrap();
/// assert_eq!(asdf, long);
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-075#VERSION-FILE-FORMATS")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "AuthoredVersionFile", into = "AuthoredVersionFile")]
pub enum VersionFileDecl {
    /// A one-line version file named by the package (`.nvmrc`,
    /// `.node-version`, `.python-version`, `.java-version` and the like).
    /// The declaration order of `names` breaks a tie inside one directory.
    Plain { names: Vec<String> },
    /// A file of `key`/`version` pairs in which each slot edits only its
    /// own line (`.sdkmanrc`, `.tool-versions`, `mise.toml`).
    Keyed {
        names: Vec<String>,
        key: String,
        syntax: KeyedSyntax,
    },
}

/// The one file name every `tool-versions` row reads.
pub const TOOL_VERSIONS_FILE: &str = ".tool-versions";

/// The authored spellings, which is what `deny_unknown_fields` is enforced
/// on; `tool-versions` lowers into `VersionFileDecl::Keyed` on the way in
/// and back out again.
///
/// The three closed vocabularies and the two fields a `keyed` row cannot
/// omit arrive here as text, so the refusal is ours and names its TOML path
/// (PROP-059 `#declaration`) rather than being serde's own report of an
/// unknown variant, which names only the line.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "format", rename_all = "kebab-case", deny_unknown_fields)]
enum AuthoredVersionFile {
    Plain {
        names: Vec<String>,
    },
    ToolVersions {
        key: String,
    },
    Keyed {
        names: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        key: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        separator: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        comment: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        extra: Option<String>,
    },
}

/// The TOML path of the whole table, for a row read on its own.
const ROWS_PATH: &str = "application.version_file";

/// One value of a closed vocabulary, refused by its TOML path.
fn word<T: Copy>(
    at: &str,
    field: &str,
    value: Option<&str>,
    default: Option<T>,
    vocabulary: &[(&str, T)],
) -> Result<T, String> {
    let Some(value) = value else {
        return default.ok_or_else(|| {
            format!(
                "{at}.{field}: a `keyed` row states its {field}, one of {}",
                spellings(vocabulary)
            )
        });
    };
    parse_word(value, vocabulary).ok_or_else(|| {
        format!(
            "{at}.{field}: `{value}` is not a {field}; one of {}",
            spellings(vocabulary)
        )
    })
}

fn spellings<T>(vocabulary: &[(&str, T)]) -> String {
    vocabulary
        .iter()
        .map(|(spelling, _)| format!("`{spelling}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

impl AuthoredVersionFile {
    /// Lower one authored row, read at the TOML path `at`, into the one
    /// declared shape every reader and the writer see.
    fn lower(self, at: &str) -> Result<VersionFileDecl, String> {
        match self {
            Self::Plain { names } => Ok(VersionFileDecl::Plain { names }),
            Self::ToolVersions { key } => Ok(VersionFileDecl::tool_versions(key)),
            Self::Keyed {
                names,
                key,
                separator,
                comment,
                extra,
            } => Ok(VersionFileDecl::Keyed {
                names,
                key: key.ok_or_else(|| {
                    format!("{at}.key: a `keyed` row states this package's key in the file")
                })?,
                syntax: KeyedSyntax {
                    separator: word(
                        at,
                        "separator",
                        separator.as_deref(),
                        None,
                        KeyedSeparator::VOCABULARY,
                    )?,
                    comment: word(
                        at,
                        "comment",
                        comment.as_deref(),
                        Some(KeyedComment::default()),
                        KeyedComment::VOCABULARY,
                    )?,
                    extra: word(
                        at,
                        "extra",
                        extra.as_deref(),
                        Some(KeyedExtra::default()),
                        KeyedExtra::VOCABULARY,
                    )?,
                },
            }),
        }
    }
}

impl TryFrom<AuthoredVersionFile> for VersionFileDecl {
    type Error = String;

    fn try_from(authored: AuthoredVersionFile) -> Result<Self, Self::Error> {
        authored.lower(ROWS_PATH)
    }
}

/// `[[application.version_file]]`, lowered row by row so every refusal
/// names the row's own index (PROP-075 `#VERSION-FILE-KEYED`).
pub(super) fn rows<'de, D>(deserializer: D) -> Result<Vec<VersionFileDecl>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    Vec::<AuthoredVersionFile>::deserialize(deserializer)?
        .into_iter()
        .enumerate()
        .map(|(position, row)| row.lower(&format!("{ROWS_PATH}[{position}]")))
        .collect::<Result<Vec<_>, _>>()
        .map_err(D::Error::custom)
}

impl From<VersionFileDecl> for AuthoredVersionFile {
    fn from(row: VersionFileDecl) -> Self {
        match row {
            VersionFileDecl::Plain { names } => Self::Plain { names },
            VersionFileDecl::Keyed { names, key, syntax } if is_tool_versions(&names, &syntax) => {
                Self::ToolVersions { key }
            }
            VersionFileDecl::Keyed { names, key, syntax } => Self::Keyed {
                names,
                key: Some(key),
                separator: Some(syntax.separator.spelling().to_owned()),
                comment: Some(syntax.comment.spelling().to_owned()),
                extra: Some(syntax.extra.spelling().to_owned()),
            },
        }
    }
}

/// Whether this keyed row is exactly the one `format = "tool-versions"`
/// expands to. A pure function of the row, so nothing has to remember which
/// spelling an author used and the two can never drift.
fn is_tool_versions(names: &[String], syntax: &KeyedSyntax) -> bool {
    *syntax == KeyedSyntax::TOOL_VERSIONS && names == [TOOL_VERSIONS_FILE]
}

impl VersionFileDecl {
    /// asdf's `.tool-versions` row for `key`, as the short spelling writes
    /// it.
    ///
    /// ```
    /// use vibe_core::manifest::VersionFileDecl;
    /// let row = VersionFileDecl::tool_versions("nodejs");
    /// assert_eq!(row.format(), "tool-versions");
    /// assert_eq!(row.key(), Some("nodejs"));
    /// ```
    pub fn tool_versions(key: impl Into<String>) -> Self {
        Self::Keyed {
            names: vec![TOOL_VERSIONS_FILE.to_owned()],
            key: key.into(),
            syntax: KeyedSyntax::TOOL_VERSIONS,
        }
    }

    /// The stable `format` spelling of this row — the short one for a row
    /// `format = "tool-versions"` would author.
    pub fn format(&self) -> &'static str {
        match self {
            Self::Plain { .. } => "plain",
            Self::Keyed { .. } if self.is_tool_versions() => "tool-versions",
            Self::Keyed { .. } => "keyed",
        }
    }

    /// Whether this row is the short spelling's expansion.
    ///
    /// ```
    /// use vibe_core::manifest::VersionFileDecl;
    /// assert!(VersionFileDecl::tool_versions("nodejs").is_tool_versions());
    /// assert!(!VersionFileDecl::Plain { names: vec![".nvmrc".into()] }.is_tool_versions());
    /// ```
    pub fn is_tool_versions(&self) -> bool {
        match self {
            Self::Plain { .. } => false,
            Self::Keyed { names, syntax, .. } => is_tool_versions(names, syntax),
        }
    }

    /// The file names this row looks for, in search order.
    pub fn names(&self) -> Vec<&str> {
        match self {
            Self::Plain { names } | Self::Keyed { names, .. } => {
                names.iter().map(String::as_str).collect()
            }
        }
    }

    /// The package's key in a keyed file, for a row of that format.
    pub fn key(&self) -> Option<&str> {
        match self {
            Self::Plain { .. } => None,
            Self::Keyed { key, .. } => Some(key),
        }
    }

    /// The line syntax of a keyed row — the one thing a reader and the
    /// writer need beyond the key.
    pub fn syntax(&self) -> Option<KeyedSyntax> {
        match self {
            Self::Plain { .. } => None,
            Self::Keyed { syntax, .. } => Some(*syntax),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One vocabulary, two spellings would be two sources of truth: the
    /// parse table and `spelling()` must agree word for word.
    #[test]
    fn every_declared_word_round_trips_through_its_spelling() {
        for (spelling, word) in KeyedSeparator::VOCABULARY {
            assert_eq!(word.spelling(), *spelling);
        }
        for (spelling, word) in KeyedComment::VOCABULARY {
            assert_eq!(word.spelling(), *spelling);
        }
        for (spelling, word) in KeyedExtra::VOCABULARY {
            assert_eq!(word.spelling(), *spelling);
        }
    }

    #[test]
    fn a_refused_word_names_its_toml_path_and_the_vocabulary() {
        let refusal = word(
            "application.version_file[2]",
            "separator",
            Some("tabs"),
            None,
            KeyedSeparator::VOCABULARY,
        )
        .expect_err("`tabs` is not a separator");
        assert!(
            refusal.starts_with("application.version_file[2].separator:"),
            "{refusal}"
        );
        assert!(refusal.contains("`whitespace`"), "{refusal}");
    }
}
