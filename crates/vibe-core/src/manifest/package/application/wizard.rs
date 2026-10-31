//! The wizard surface of an installer option (`PROP-065`): named feature
//! profiles, typed parameters and the license a user accepts. All of it is
//! data; the planner resolves it, a graphical tool draws it.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#parameters");

use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use specmark::spec;

/// One `[[installer_options.profiles]]` row: a named feature set.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#profiles")]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileDecl {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub default: bool,
}

/// The closed set of parameter types.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#parameters")]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParameterType {
    #[default]
    String,
    Bool,
    Integer,
    Choice,
    Path,
}

impl ParameterType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::String => "string",
            Self::Bool => "bool",
            Self::Integer => "integer",
            Self::Choice => "choice",
            Self::Path => "path",
        }
    }
}

/// A default as written: `true`, `42` or `"text"`.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#parameters")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ParameterValue {
    Bool(bool),
    Integer(i64),
    Text(String),
}

impl fmt::Display for ParameterValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bool(value) => write!(f, "{value}"),
            Self::Integer(value) => write!(f, "{value}"),
            Self::Text(value) => f.write_str(value),
        }
    }
}

/// One `{ value, name }` entry of a `choice` parameter.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#parameters")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChoiceDecl {
    pub value: String,
    pub name: String,
}

/// One `[[installer_options.parameters]]` row: a typed question.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#parameters")]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParameterDecl {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub kind: ParameterType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<ParameterValue>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<i64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub choices: Vec<ChoiceDecl>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub must_exist: bool,
    /// Passed to children, never written anywhere.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub secret: bool,
}

impl ParameterDecl {
    /// Validate `raw` against the type and return its canonical text:
    /// `true` / `false`, a decimal integer within `min` / `max`, a declared
    /// choice value, or the text itself. Whether a `path` exists is a host
    /// fact the planner checks.
    ///
    /// ```
    /// use vibe_core::manifest::{ParameterDecl, ParameterType};
    ///
    /// let port = ParameterDecl { id: "port".into(), name: "Port".into(), kind: ParameterType::Integer,
    ///     min: Some(1), max: Some(65535), ..Default::default() };
    /// assert_eq!(port.canonical(" 080").unwrap(), "80");
    /// assert!(port.canonical("0").is_err());
    /// let flag = ParameterDecl { kind: ParameterType::Bool, ..port };
    /// assert_eq!(flag.canonical("Yes").unwrap(), "true");
    /// ```
    pub fn canonical(&self, raw: &str) -> Result<String, String> {
        if raw.chars().any(char::is_control) {
            return Err("the value contains control characters".into());
        }
        match self.kind {
            ParameterType::String => Ok(raw.to_owned()),
            ParameterType::Path if raw.trim().is_empty() => Err("a path must be nonempty".into()),
            ParameterType::Path => Ok(raw.to_owned()),
            ParameterType::Bool => match raw.trim().to_ascii_lowercase().as_str() {
                "true" | "yes" | "on" | "1" => Ok("true".into()),
                "false" | "no" | "off" | "0" => Ok("false".into()),
                _ => Err(format!("`{raw}` is not a boolean (true / false)")),
            },
            ParameterType::Integer => {
                let value: i64 = raw
                    .trim()
                    .parse()
                    .map_err(|_| format!("`{raw}` is not an integer"))?;
                if self.min.is_some_and(|min| value < min)
                    || self.max.is_some_and(|max| value > max)
                {
                    return Err(format!(
                        "{value} is outside {}..={}",
                        self.min.map_or_else(String::new, |min| min.to_string()),
                        self.max.map_or_else(String::new, |max| max.to_string())
                    ));
                }
                Ok(value.to_string())
            }
            ParameterType::Choice => match self.choices.iter().find(|choice| choice.value == raw) {
                Some(choice) => Ok(choice.value.clone()),
                None => Err(format!(
                    "`{raw}` is not one of {}",
                    self.choices
                        .iter()
                        .map(|choice| choice.value.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )),
            },
        }
    }
}

/// `[installer_options.license]`: a package-relative text the user accepts.
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#license")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LicenseDecl {
    pub file: PathBuf,
    pub name: String,
}
