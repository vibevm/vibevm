//! Extra HTTP request headers for one invocation's installer downloads
//! (`PROP-075#DOWNLOAD-HEADERS`): read from a file, never from an argument.
//!
//! A header value is a credential — a mirror behind an authenticating
//! gateway is the whole reason this exists — and every local user can read
//! `/proc/<pid>/cmdline`, so the value may not be an argv element. The
//! command line names a *path*; the bytes stay in a file the caller owns.
//!
//! The file is one header per line, `Name: value`. A `#` line and a blank
//! line are skipped. Everything else is refused **by name, with its line
//! number**, before any request is made.
//!
//! ```
//! use vibe_core::download_headers::DownloadHeaders;
//!
//! let headers = DownloadHeaders::parse(
//!     "# the mirror's gateway\nAuthorization: Bearer t0ken\n\nX-Mirror: eu\n",
//!     "headers.txt",
//! )
//! .unwrap();
//! assert_eq!(headers.names().collect::<Vec<_>>(), ["Authorization", "X-Mirror"]);
//! // The names are useful and are not secret; the values never print.
//! assert!(!format!("{headers:?}").contains("t0ken"));
//! ```

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-075#DOWNLOAD-HEADERS");

use std::path::Path;

/// One `Name: value` line of the file. [`Debug`] shows the name only: a
/// value reaches no log, no journal, no error and no `--json` document.
///
/// ```
/// use vibe_core::download_headers::DownloadHeaders;
///
/// let headers = DownloadHeaders::parse("Authorization: Bearer t0ken\n", "h").unwrap();
/// let header = headers.iter().next().unwrap();
/// assert_eq!(header.name(), "Authorization");
/// assert_eq!(header.value(), "Bearer t0ken");
/// assert!(!format!("{header:?}").contains("t0ken"));
/// ```
#[derive(Clone, PartialEq, Eq)]
pub struct DownloadHeader {
    name: String,
    value: String,
}

impl DownloadHeader {
    /// The field name, which may be shown.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The field value. Every caller of this sends it; none prints it.
    pub fn value(&self) -> &str {
        &self.value
    }
}

impl std::fmt::Debug for DownloadHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DownloadHeader")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

/// The headers one invocation attaches to the installer's own `http://` and
/// `https://` downloads, in the order the file declares them.
///
/// ```
/// use vibe_core::download_headers::DownloadHeaders;
///
/// let headers = DownloadHeaders::parse("Authorization: Bearer t0ken\n", "h").unwrap();
/// assert_eq!(headers.len(), 1);
/// assert_eq!(headers.iter().next().unwrap().value(), "Bearer t0ken");
/// ```
#[derive(Clone, PartialEq, Eq, Default)]
pub struct DownloadHeaders {
    headers: Vec<DownloadHeader>,
}

impl std::fmt::Debug for DownloadHeaders {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DownloadHeaders")
            .field("names", &self.names().collect::<Vec<_>>())
            .finish()
    }
}

impl DownloadHeaders {
    /// Read and validate `path`. Every refusal names the problem and the
    /// line it is on, and happens before anything is contacted.
    pub fn read(path: &Path) -> Result<Self, DownloadHeaderError> {
        let shown = path.display().to_string();
        let text =
            std::fs::read_to_string(path).map_err(|error| DownloadHeaderError::Unreadable {
                path: shown.clone(),
                reason: error.to_string(),
            })?;
        Self::parse(&text, &shown)
    }

    /// [`Self::read`] on text already in hand; `path` only names the file
    /// in a refusal.
    pub fn parse(text: &str, path: &str) -> Result<Self, DownloadHeaderError> {
        let mut headers = Vec::new();
        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            let trimmed = raw.trim_matches(|c: char| c == ' ' || c == '\t' || c == '\r');
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let (name, value) =
                trimmed
                    .split_once(':')
                    .ok_or_else(|| DownloadHeaderError::Malformed {
                        path: path.to_owned(),
                        line,
                    })?;
            let name = name.trim_matches(|c: char| c == ' ' || c == '\t');
            if name.is_empty() || !name.bytes().all(is_field_name_byte) {
                return Err(DownloadHeaderError::InvalidName {
                    path: path.to_owned(),
                    line,
                });
            }
            let value = value.trim_matches(|c: char| c == ' ' || c == '\t');
            if value.bytes().any(is_control_byte) {
                return Err(DownloadHeaderError::ControlCharacter {
                    path: path.to_owned(),
                    line,
                    name: name.to_owned(),
                });
            }
            headers.push(DownloadHeader {
                name: name.to_owned(),
                value: value.to_owned(),
            });
        }
        if headers.is_empty() {
            return Err(DownloadHeaderError::Empty {
                path: path.to_owned(),
            });
        }
        Ok(Self { headers })
    }

    pub fn iter(&self) -> std::slice::Iter<'_, DownloadHeader> {
        self.headers.iter()
    }

    /// The field names, which a transfer record or a refusal may show.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.headers.iter().map(DownloadHeader::name)
    }

    pub fn len(&self) -> usize {
        self.headers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.headers.is_empty()
    }
}

impl<'a> IntoIterator for &'a DownloadHeaders {
    type Item = &'a DownloadHeader;
    type IntoIter = std::slice::Iter<'a, DownloadHeader>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// An HTTP field-name character (RFC 9110 `tchar`).
fn is_field_name_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
        || matches!(
            byte,
            b'!' | b'#'
                | b'$'
                | b'%'
                | b'&'
                | b'\''
                | b'*'
                | b'+'
                | b'-'
                | b'.'
                | b'^'
                | b'_'
                | b'`'
                | b'|'
                | b'~'
        )
}

/// A byte no field value may carry — a newline among them, which is what
/// a header-splitting attempt looks like.
fn is_control_byte(byte: u8) -> bool {
    byte < 0x20 || byte == 0x7f
}

/// Why a download-header file was refused. **No variant carries a value**:
/// the file's whole point is that its values are credentials.
///
/// ```
/// use vibe_core::download_headers::{DownloadHeaderError, DownloadHeaders};
///
/// let error = DownloadHeaders::parse("Authorization: ok\nBearer t0ken\n", "h.txt").unwrap_err();
/// assert_eq!(error, DownloadHeaderError::Malformed { path: "h.txt".into(), line: 2 });
/// assert!(error.to_string().contains("line 2"));
/// assert!(!error.to_string().contains("t0ken"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-075#DOWNLOAD-HEADERS")]
pub enum DownloadHeaderError {
    #[error(
        "the download header file `{path}` could not be read: {reason} \
         (violates spec://org.vibevm.core/vibevm/common/PROP-075#DOWNLOAD-HEADERS; \
          fix: point `--download-header-file` at a readable UTF-8 file of `Name: value` lines)"
    )]
    Unreadable { path: String, reason: String },
    #[error(
        "the download header file `{path}` declares no header \
         (violates spec://org.vibevm.core/vibevm/common/PROP-075#DOWNLOAD-HEADERS; \
          fix: write one `Name: value` per line, or leave the flag out)"
    )]
    Empty { path: String },
    #[error(
        "`{path}` line {line} is not a header \
         (violates spec://org.vibevm.core/vibevm/common/PROP-075#DOWNLOAD-HEADERS; \
          fix: write the line as `Name: value`, or start it with `#` to comment it out)"
    )]
    Malformed { path: String, line: usize },
    #[error(
        "`{path}` line {line} has no valid HTTP header name \
         (violates spec://org.vibevm.core/vibevm/common/PROP-075#DOWNLOAD-HEADERS; \
          fix: name the header with letters, digits and `!#$%&'*+-.^_`|~` only)"
    )]
    InvalidName { path: String, line: usize },
    #[error(
        "`{path}` line {line}: the value of `{name}` contains a control character \
         (violates spec://org.vibevm.core/vibevm/common/PROP-075#DOWNLOAD-HEADERS; \
          fix: write the value on one line, with no control character in it)"
    )]
    ControlCharacter {
        path: String,
        line: usize,
        name: String,
    },
}

#[cfg(test)]
#[path = "download_headers/tests.rs"]
mod tests;
