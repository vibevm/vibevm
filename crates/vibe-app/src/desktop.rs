//! A small reader of freedesktop desktop entries (`PROP-059#desktop-entries`):
//! groups, localised keys `Key[xx]`, string escapes, booleans, `;`-lists and
//! the `Exec` quoting with its field codes. Nothing else of the format is
//! interpreted here; [`crate::derive`] maps an entry onto a launcher.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#desktop-entries");

use std::collections::BTreeMap;

use specmark::spec;

/// One parsed desktop entry file: every group by name, every key of a group
/// with its raw (still escaped) value. A localised key keeps its
/// `Key[xx]` spelling.
///
/// ```
/// use vibe_app::DesktopFile;
///
/// let file = DesktopFile::parse(
///     "[Desktop Entry]\nType=Application\nName=Edit\nName[de]=Bearbeiten\nTerminal=true\nCategories=Utility;TextEditor;\n",
/// )
/// .unwrap();
/// let main = DesktopFile::MAIN;
/// assert_eq!(file.string(main, "Name").as_deref(), Some("Edit"));
/// assert_eq!(file.localised(main, "Name")["de"], "Bearbeiten");
/// assert_eq!(file.boolean(main, "Terminal"), Some(true));
/// assert_eq!(file.list(main, "Categories"), ["Utility", "TextEditor"]);
/// ```
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#desktop-entries")]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DesktopFile {
    pub groups: BTreeMap<String, BTreeMap<String, String>>,
}

impl DesktopFile {
    /// The main group of a desktop entry.
    pub const MAIN: &'static str = "Desktop Entry";

    /// Parse the text of one file. Blank lines and `#` comments are
    /// skipped; a key before any group, a line that is neither a group
    /// header nor `Key=Value`, or a duplicate key is refused.
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut file = Self::default();
        let mut current: Option<String> = None;
        for (number, line) in text.lines().enumerate() {
            let line = line.trim_end_matches('\r').trim_start();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some(name) = line
                .strip_prefix('[')
                .and_then(|rest| rest.strip_suffix(']'))
            {
                if file.groups.contains_key(name) {
                    return Err(format!("line {}: group [{name}] is repeated", number + 1));
                }
                file.groups.insert(name.to_owned(), BTreeMap::new());
                current = Some(name.to_owned());
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                return Err(format!("line {}: expected Key=Value", number + 1));
            };
            let Some(group) = current.as_ref().and_then(|name| file.groups.get_mut(name)) else {
                return Err(format!("line {}: a key before any group", number + 1));
            };
            let key = key.trim_end();
            if group
                .insert(key.to_owned(), value.trim_start().to_owned())
                .is_some()
            {
                return Err(format!("line {}: key `{key}` is repeated", number + 1));
            }
        }
        Ok(file)
    }

    fn raw(&self, group: &str, key: &str) -> Option<&str> {
        self.groups.get(group)?.get(key).map(String::as_str)
    }

    /// A string value with `\s`, `\n`, `\t`, `\r` and `\\` unescaped.
    pub fn string(&self, group: &str, key: &str) -> Option<String> {
        self.raw(group, key).map(unescape)
    }

    /// Every `Key[locale]` value of a key by its locale, the locale written
    /// as a language tag (`de_DE` becomes `de-DE`, an encoding is dropped).
    pub fn localised(&self, group: &str, key: &str) -> BTreeMap<String, String> {
        let prefix = format!("{key}[");
        self.groups
            .get(group)
            .into_iter()
            .flatten()
            .filter_map(|(name, value)| {
                let locale = name.strip_prefix(&prefix)?.strip_suffix(']')?;
                Some((language_tag(locale), unescape(value)))
            })
            .collect()
    }

    /// A boolean value (`true` / `false`); anything else is absent.
    pub fn boolean(&self, group: &str, key: &str) -> Option<bool> {
        match self.raw(group, key)?.trim() {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        }
    }

    /// A `;`-separated list with `\;` kept as a literal semicolon; empty
    /// items are dropped.
    pub fn list(&self, group: &str, key: &str) -> Vec<String> {
        let Some(raw) = self.raw(group, key) else {
            return Vec::new();
        };
        let mut items = Vec::new();
        let mut item = String::new();
        let mut chars = raw.chars();
        while let Some(ch) = chars.next() {
            match ch {
                '\\' => match chars.next() {
                    Some(';') => item.push(';'),
                    Some(next) => {
                        item.push('\\');
                        item.push(next);
                    }
                    None => item.push('\\'),
                },
                ';' => items.push(std::mem::take(&mut item)),
                other => item.push(other),
            }
        }
        items.push(item);
        items
            .into_iter()
            .map(|item| unescape(&item))
            .filter(|item| !item.trim().is_empty())
            .collect()
    }
}

fn unescape(raw: &str) -> String {
    let mut out = String::new();
    let mut chars = raw.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('s') => out.push(' '),
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('\\') => out.push('\\'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

/// `lang_COUNTRY.ENCODING@MODIFIER` as a language tag `lang-COUNTRY@MODIFIER`.
fn language_tag(locale: &str) -> String {
    let (base, modifier) = match locale.split_once('@') {
        Some((base, modifier)) => (base, Some(modifier)),
        None => (locale, None),
    };
    let base = base.split('.').next().unwrap_or(base).replace('_', "-");
    match modifier {
        Some(modifier) => format!("{base}@{modifier}"),
        None => base,
    }
}

/// Split an `Exec` value (already string-unescaped) into words: whitespace
/// separates words outside double quotes; inside them `\"`, `` \` ``, `\$`
/// and `\\` are literal characters. Field codes become the launcher
/// placeholders (`%f` `{file}`, `%F` `{files}`, `%u` `{url}`, `%U`
/// `{urls}`), `%%` a literal `%`; `%i`, `%c`, `%k` and the deprecated codes
/// are dropped, and a word made only of dropped codes disappears.
///
/// ```
/// use vibe_app::exec_words;
///
/// assert_eq!(
///     exec_words(r#"edit --title "My \"Doc\"" %i %F"#).unwrap(),
///     ["edit", "--title", "My \"Doc\"", "{files}"],
/// );
/// assert_eq!(exec_words("tool --open=%u 100%%").unwrap(), ["tool", "--open={url}", "100%"]);
/// assert!(exec_words("tool \"unterminated").is_err());
/// ```
pub fn exec_words(exec: &str) -> Result<Vec<String>, String> {
    let mut words: Vec<String> = Vec::new();
    let mut word = String::new();
    let mut started = false;
    let mut chars = exec.chars();
    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                started = true;
                loop {
                    match chars.next() {
                        Some('"') => break,
                        Some('\\') => match chars.next() {
                            Some(next @ ('"' | '`' | '$' | '\\')) => word.push(next),
                            Some(next) => {
                                word.push('\\');
                                word.push(next);
                            }
                            None => return Err("Exec ends inside a quoted argument".into()),
                        },
                        Some(next) => word.push(next),
                        None => return Err("Exec has an unterminated quoted argument".into()),
                    }
                }
            }
            '\\' => {
                started = true;
                if let Some(next) = chars.next() {
                    word.push(next);
                }
            }
            ch if ch.is_whitespace() => {
                if started {
                    words.push(std::mem::take(&mut word));
                    started = false;
                }
            }
            other => {
                started = true;
                word.push(other);
            }
        }
    }
    if started {
        words.push(word);
    }
    let mut out = Vec::new();
    for word in words {
        let (mapped, only_dropped) = field_codes(&word);
        if !(only_dropped && mapped.is_empty()) {
            out.push(mapped);
        }
    }
    if out.is_empty() {
        return Err("Exec names no program".into());
    }
    Ok(out)
}

/// Map the field codes of one word; the flag tells whether the word held
/// nothing but codes that are dropped.
fn field_codes(word: &str) -> (String, bool) {
    let mut out = String::new();
    let mut dropped_any = false;
    let mut other = false;
    let mut chars = word.chars();
    let whole = |code: char| word.len() == 2 && word.ends_with(code);
    while let Some(ch) = chars.next() {
        if ch != '%' {
            other = true;
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('%') => {
                other = true;
                out.push('%');
            }
            Some('f') => out.push_str("{file}"),
            Some('u') => out.push_str("{url}"),
            Some('F') => out.push_str(if whole('F') { "{files}" } else { "{file}" }),
            Some('U') => out.push_str(if whole('U') { "{urls}" } else { "{url}" }),
            Some('i' | 'c' | 'k' | 'd' | 'D' | 'n' | 'N' | 'v' | 'm') => dropped_any = true,
            Some(unknown) => {
                other = true;
                out.push('%');
                out.push(unknown);
            }
            None => {
                other = true;
                out.push('%');
            }
        }
    }
    (out, dropped_any && !other)
}

#[cfg(test)]
#[path = "desktop/tests.rs"]
mod tests;
