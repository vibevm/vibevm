//! The typed, serialisable, **named**-parameter schema and values, plus their
//! invoke-time validation (PROP-039 §5).
//!
//! A [`ParamSchema`] declares each parameter's name, type, optionality, and
//! optional default; [`ParamValues`] is the name→value bag a caller supplies;
//! [`validate`] rejects a type mismatch, a missing required parameter, or an
//! unknown parameter with a typed [`ParamError`] *before* the action body runs
//! (§5.2 — closes the incumbents' `unknown[]` / phantom-key gap).
//!
//! Spec: [PROP-039 §5](../../../../spec/modules/vibe-actions/PROP-039-action-system.md#parameters).

specmark::scope!("spec://vibevm/modules/vibe-actions/PROP-039#parameters");

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// The tag half of a typed parameter — the small set of value shapes an action
/// input may take. Closed on purpose; a new shape is a spec change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ParamType {
    /// A UTF-8 string.
    String,
    /// A 64-bit signed integer.
    Int,
    /// A boolean flag.
    Bool,
}

impl ParamType {
    /// The wire spelling of the tag.
    pub const fn as_str(self) -> &'static str {
        match self {
            ParamType::String => "string",
            ParamType::Int => "int",
            ParamType::Bool => "bool",
        }
    }

    /// Does `value` carry this type?
    pub fn matches(self, value: &ParamValue) -> bool {
        self == value.type_of()
    }
}

impl fmt::Display for ParamType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A typed parameter value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParamValue {
    /// A string value.
    String(String),
    /// An integer value.
    Int(i64),
    /// A boolean value.
    Bool(bool),
}

impl ParamValue {
    /// The [`ParamType`] tag this value carries.
    pub const fn type_of(&self) -> ParamType {
        match self {
            ParamValue::String(_) => ParamType::String,
            ParamValue::Int(_) => ParamType::Int,
            ParamValue::Bool(_) => ParamType::Bool,
        }
    }
}

impl From<&str> for ParamValue {
    fn from(s: &str) -> Self {
        ParamValue::String(s.to_owned())
    }
}

impl From<String> for ParamValue {
    fn from(s: String) -> Self {
        ParamValue::String(s)
    }
}

impl From<i64> for ParamValue {
    fn from(n: i64) -> Self {
        ParamValue::Int(n)
    }
}

impl From<bool> for ParamValue {
    fn from(b: bool) -> Self {
        ParamValue::Bool(b)
    }
}

/// One named parameter in a [`ParamSchema`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParamSpec {
    name: String,
    ty: ParamType,
    required: bool,
    default: Option<ParamValue>,
}

impl ParamSpec {
    /// A **required** parameter — the caller must supply a value.
    pub fn required(name: impl Into<String>, ty: ParamType) -> Self {
        ParamSpec {
            name: name.into(),
            ty,
            required: true,
            default: None,
        }
    }

    /// An **optional** parameter — the caller may omit it.
    pub fn optional(name: impl Into<String>, ty: ParamType) -> Self {
        ParamSpec {
            name: name.into(),
            ty,
            required: false,
            default: None,
        }
    }

    /// An optional parameter carrying a `default` used when the caller omits
    /// it. The default's type is assumed to match `ty` (the declaration site's
    /// responsibility).
    pub fn with_default(name: impl Into<String>, ty: ParamType, default: ParamValue) -> Self {
        ParamSpec {
            name: name.into(),
            ty,
            required: false,
            default: Some(default),
        }
    }

    /// The parameter name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The declared type.
    pub const fn ty(&self) -> ParamType {
        self.ty
    }

    /// Whether the caller must supply this parameter.
    pub const fn is_required(&self) -> bool {
        self.required
    }

    /// The default value, if any.
    pub fn default_value(&self) -> Option<&ParamValue> {
        self.default.as_ref()
    }
}

/// A typed, serialisable, named-parameter schema. An action with no inputs
/// declares an empty schema (`ParamSchema::default()`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ParamSchema {
    params: Vec<ParamSpec>,
}

impl ParamSchema {
    /// An empty schema — the action takes no parameters.
    pub fn empty() -> Self {
        ParamSchema::default()
    }

    /// Build a schema from a list of specs.
    pub fn new(params: Vec<ParamSpec>) -> Self {
        ParamSchema { params }
    }

    /// Append a spec, chaining.
    #[must_use]
    pub fn with(mut self, spec: ParamSpec) -> Self {
        self.params.push(spec);
        self
    }

    /// The declared parameters, in declaration order.
    pub fn params(&self) -> &[ParamSpec] {
        &self.params
    }

    /// The spec for `name`, if declared.
    pub fn get(&self, name: &str) -> Option<&ParamSpec> {
        self.params.iter().find(|p| p.name == name)
    }

    /// Whether the schema declares no parameters.
    pub fn is_empty(&self) -> bool {
        self.params.is_empty()
    }

    /// The number of declared parameters.
    pub fn len(&self) -> usize {
        self.params.len()
    }

    /// Validate `values` against this schema (delegates to [`validate`]).
    pub fn validate(&self, values: &ParamValues) -> Result<(), ParamError> {
        validate(self, values)
    }
}

/// A caller-supplied bag of named parameter values.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ParamValues {
    map: BTreeMap<String, ParamValue>,
}

impl ParamValues {
    /// An empty value bag.
    pub fn new() -> Self {
        ParamValues::default()
    }

    /// Insert (or replace) a named value.
    pub fn insert(&mut self, name: impl Into<String>, value: impl Into<ParamValue>) {
        self.map.insert(name.into(), value.into());
    }

    /// Insert a named value, chaining.
    #[must_use]
    pub fn with(mut self, name: impl Into<String>, value: impl Into<ParamValue>) -> Self {
        self.insert(name, value);
        self
    }

    /// The value bound to `name`, if any.
    pub fn get(&self, name: &str) -> Option<&ParamValue> {
        self.map.get(name)
    }

    /// Whether `name` is present.
    pub fn contains(&self, name: &str) -> bool {
        self.map.contains_key(name)
    }

    /// The parameter names present, in sorted order.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.map.keys().map(String::as_str)
    }

    /// Iterate over `(name, value)` pairs, in sorted order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &ParamValue)> {
        self.map.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Whether the bag is empty.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// The number of values present.
    pub fn len(&self) -> usize {
        self.map.len()
    }
}

/// A parameter validation failure (PROP-039 §5.2).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[specmark::spec(implements = "spec://vibevm/modules/vibe-actions/PROP-039#param-validation")]
pub enum ParamError {
    /// A required parameter was not supplied.
    #[error(
        "missing required parameter `{name}` \
         (violates spec://vibevm/modules/vibe-actions/PROP-039#param-validation; \
          fix: supply a value for `{name}`)"
    )]
    MissingRequired {
        /// The absent parameter's name.
        name: String,
    },

    /// A supplied value's type does not match the schema.
    #[error(
        "parameter `{name}` expects `{expected}` but got `{got}` \
         (violates spec://vibevm/modules/vibe-actions/PROP-039#param-validation; \
          fix: pass a `{expected}` value for `{name}`)"
    )]
    TypeMismatch {
        /// The parameter's name.
        name: String,
        /// The declared type.
        expected: ParamType,
        /// The supplied value's type.
        got: ParamType,
    },

    /// A supplied parameter is not declared by the schema.
    #[error(
        "unknown parameter `{name}` — not declared by the action's schema \
         (violates spec://vibevm/modules/vibe-actions/PROP-039#param-validation; \
          fix: remove `{name}` or add it to the schema)"
    )]
    UnknownParam {
        /// The undeclared parameter's name.
        name: String,
    },
}

/// Validate `values` against `schema` (PROP-039 §5.2): every supplied value is
/// declared and type-correct, and every required parameter is present. The
/// first violation is returned; a clean bag returns `Ok(())`.
pub fn validate(schema: &ParamSchema, values: &ParamValues) -> Result<(), ParamError> {
    // Unknown parameters — a supplied name the schema does not declare.
    for name in values.names() {
        if schema.get(name).is_none() {
            return Err(ParamError::UnknownParam {
                name: name.to_owned(),
            });
        }
    }

    // Type mismatches and missing required parameters.
    for spec in schema.params() {
        match values.get(spec.name()) {
            Some(value) => {
                if !spec.ty().matches(value) {
                    return Err(ParamError::TypeMismatch {
                        name: spec.name().to_owned(),
                        expected: spec.ty(),
                        got: value.type_of(),
                    });
                }
            }
            None => {
                if spec.is_required() {
                    return Err(ParamError::MissingRequired {
                        name: spec.name().to_owned(),
                    });
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema() -> ParamSchema {
        ParamSchema::empty()
            .with(ParamSpec::required("by", ParamType::String))
            .with(ParamSpec::optional("desc", ParamType::Bool))
            .with(ParamSpec::with_default(
                "limit",
                ParamType::Int,
                ParamValue::Int(50),
            ))
    }

    #[test]
    fn validates_a_well_formed_bag() {
        let values = ParamValues::new()
            .with("by", "name")
            .with("desc", true)
            .with("limit", 10_i64);
        assert!(validate(&schema(), &values).is_ok());
    }

    #[test]
    fn optional_params_may_be_omitted() {
        let values = ParamValues::new().with("by", "name");
        assert!(validate(&schema(), &values).is_ok());
    }

    #[test]
    fn rejects_missing_required() {
        let values = ParamValues::new().with("desc", false);
        assert_eq!(
            validate(&schema(), &values),
            Err(ParamError::MissingRequired {
                name: "by".to_owned()
            })
        );
    }

    #[test]
    fn rejects_type_mismatch() {
        let values = ParamValues::new().with("by", 7_i64); // want String, got Int
        assert_eq!(
            validate(&schema(), &values),
            Err(ParamError::TypeMismatch {
                name: "by".to_owned(),
                expected: ParamType::String,
                got: ParamType::Int,
            })
        );
    }

    #[test]
    fn rejects_unknown_param() {
        let values = ParamValues::new().with("by", "name").with("bogus", true);
        assert_eq!(
            validate(&schema(), &values),
            Err(ParamError::UnknownParam {
                name: "bogus".to_owned()
            })
        );
    }

    #[test]
    fn empty_schema_accepts_empty_bag() {
        assert!(validate(&ParamSchema::empty(), &ParamValues::new()).is_ok());
    }

    #[test]
    fn empty_schema_rejects_any_value() {
        let values = ParamValues::new().with("x", 1_i64);
        assert!(matches!(
            validate(&ParamSchema::empty(), &values),
            Err(ParamError::UnknownParam { .. })
        ));
    }

    #[test]
    fn schema_method_delegates_to_free_fn() {
        let values = ParamValues::new().with("by", "name");
        assert!(schema().validate(&values).is_ok());
    }

    #[test]
    fn values_serde_round_trip() {
        let values = ParamValues::new().with("by", "name").with("limit", 3_i64);
        let json = serde_json::to_string(&values).unwrap();
        let back: ParamValues = serde_json::from_str(&json).unwrap();
        assert_eq!(values, back);
    }
}
