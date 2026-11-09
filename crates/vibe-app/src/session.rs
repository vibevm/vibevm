//! Session choices: the `VIBE_APP_USE` value `slot=key;slot=key`. The CLI and
//! the PATH proxy read the variable at their process boundary; this module
//! only parses and renders the value, so a choice never touches the disk.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-059#session-selection");

use std::collections::BTreeMap;

use specmark::spec;

use crate::error::AppError;

/// The environment variable that carries a session's provider choices.
pub const SESSION_VARIABLE: &str = "VIBE_APP_USE";

/// The choice that switches a slot off for one session
/// (`PROP-059#slot-off`): `VIBE_APP_USE=slot=none`. An installation key
/// always carries a `/`, so the bare word can never be one.
pub const SESSION_OFF: &str = "none";

/// Per-slot provider choices of one session; a later entry wins per slot.
///
/// ```
/// use vibe_app::SessionChoices;
///
/// let mut choices = SessionChoices::parse(" tool = org.example/a@1.0.0 ;; ").unwrap();
/// choices.apply("tool=org.example/b@1.0.0").unwrap();
/// choices.apply("editor=org.example/vim@9.1.0").unwrap();
/// assert_eq!(choices.provider("tool"), Some("org.example/b@1.0.0"));
/// assert_eq!(
///     choices.render(),
///     "editor=org.example/vim@9.1.0;tool=org.example/b@1.0.0"
/// );
/// assert!(SessionChoices::parse("tool").is_err());
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-059#session-selection")]
pub struct SessionChoices {
    choices: BTreeMap<String, String>,
}

impl SessionChoices {
    /// Parse a whole `VIBE_APP_USE` value: entries separated by `;`,
    /// whitespace trimmed, empty entries ignored.
    pub fn parse(text: &str) -> Result<Self, AppError> {
        let mut choices = Self::default();
        for entry in text.split(';').filter(|entry| !entry.trim().is_empty()) {
            choices.apply(entry)?;
        }
        Ok(choices)
    }

    /// Apply one `SLOT=KEY` entry, replacing an earlier choice for the slot.
    pub fn apply(&mut self, entry: &str) -> Result<(), AppError> {
        let malformed = || AppError::SessionChoice {
            entry: entry.trim().to_owned(),
        };
        let (slot, key) = entry.split_once('=').ok_or_else(malformed)?;
        let (slot, key) = (slot.trim(), key.trim());
        if slot.is_empty() || key.is_empty() || slot.contains(';') || key.contains([';', '=']) {
            return Err(malformed());
        }
        self.choices.insert(slot.to_owned(), key.to_owned());
        Ok(())
    }

    /// The installation key chosen for `slot`, if any; [`SESSION_OFF`]
    /// when the session switched the slot off.
    pub fn provider(&self, slot: &str) -> Option<&str> {
        self.choices.get(slot).map(String::as_str)
    }

    /// Whether this session switched `slot` off (`PROP-059#slot-off`).
    pub fn is_off(&self, slot: &str) -> bool {
        self.provider(slot) == Some(SESSION_OFF)
    }

    pub fn is_empty(&self) -> bool {
        self.choices.is_empty()
    }

    /// The canonical `slot=key;slot=key` value, slots in sorted order.
    pub fn render(&self) -> String {
        self.choices
            .iter()
            .map(|(slot, key)| format!("{slot}={key}"))
            .collect::<Vec<_>>()
            .join(";")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries_trim_skip_empties_and_later_wins() {
        let choices = SessionChoices::parse(" ; a = k1 ;b=k2;; a=k3 ;").expect("well-formed value");
        assert_eq!(choices.provider("a"), Some("k3"));
        assert_eq!(choices.provider("b"), Some("k2"));
        assert_eq!(choices.render(), "a=k3;b=k2");
        assert!(SessionChoices::parse("   ").unwrap().is_empty());
    }

    #[test]
    fn malformed_entries_are_named() {
        for (value, named) in [
            ("a=k;broken", "broken"),
            ("=k", "=k"),
            ("a=", "a="),
            ("a=b=c", "a=b=c"),
        ] {
            let error = SessionChoices::parse(value).unwrap_err().to_string();
            assert!(error.contains(&format!("`{named}`")), "{value}: {error}");
        }
        let mut choices = SessionChoices::default();
        assert!(choices.apply("a=k;b=j").is_err(), "one --use is one entry");
    }
}
