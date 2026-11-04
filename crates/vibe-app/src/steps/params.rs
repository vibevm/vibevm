//! Parameters and the license of an installer option (`PROP-065#parameters`,
//! `#license`): values from `--set`, the recorded choice of an update, an
//! attended prompt or the default, validated at planning time; the license
//! accepted before anything is fetched.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#parameters");
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-065#license");

use std::collections::BTreeMap;
use std::path::Path;

use specmark::spec;
use vibe_core::manifest::{InstallerOptionDecl, ParameterDecl, ParameterType};

use crate::error::StepError;

use super::plan::HostFacts;

/// An attended typed question: the parameter and the value preselected;
/// returns the typed text, or `None` to keep the preselection.
pub type ParameterPrompt<'a> =
    &'a dyn Fn(&ParameterDecl, Option<&str>) -> Result<Option<String>, String>;

/// An attended license question: the license name and its full text;
/// returns whether the user accepts.
pub type LicensePrompt<'a> = &'a dyn Fn(&str, &str) -> Result<bool, String>;

/// One resolved parameter value. Secret values remain execution inputs;
/// Debug omits their contents.
///
/// ```
/// let value = vibe_app::ParamValue {
///     id: "token".into(), value: "SYN_DEBUG_INPUT".into(), secret: true,
/// };
/// assert!(!format!("{value:?}").contains("SYN_DEBUG_INPUT"));
/// assert_eq!(value.value, "SYN_DEBUG_INPUT");
/// ```
#[derive(Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#parameters")]
pub struct ParamValue {
    pub id: String,
    pub value: String,
    pub secret: bool,
}

impl std::fmt::Debug for ParamValue {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = if self.secret {
            "[redacted]"
        } else {
            self.value.as_str()
        };
        formatter
            .debug_struct("ParamValue")
            .field("id", &self.id)
            .field("value", &value)
            .field("secret", &self.secret)
            .finish()
    }
}

/// `VIBE_PARAM_<ID>` / `VIBE_FEATURE_<ID>`: upper-cased, `-` and `.` to `_`.
///
/// ```
/// assert_eq!(vibe_app::step_env_name("VIBE_PARAM_", "install-dir"), "VIBE_PARAM_INSTALL_DIR");
/// ```
pub fn step_env_name(prefix: &str, id: &str) -> String {
    vibe_core::manifest::step_env_name(prefix, id)
}

/// Execution values plus value-free proof of actual explicit secret provision.
#[derive(Debug)]
pub(crate) struct ParameterResolution {
    pub values: Vec<ParamValue>,
    pub supplied_secrets: Vec<String>,
    pub secret_origins:
        BTreeMap<String, vibe_wire::generated::shared::ConfigurationParameterOrigin>,
}
impl std::ops::Deref for ParameterResolution {
    type Target = [ParamValue];
    fn deref(&self) -> &Self::Target {
        &self.values
    }
}

/// Resolve every declared parameter: `--set`, else the recorded value of
/// an update, else an attended prompt, else the default. A required
/// parameter without a value, a value the type refuses, a `path` that must
/// exist and does not, or a `--set` naming no parameter is a refusal.
pub(crate) fn resolve_parameters(
    option: &InstallerOptionDecl,
    set: &[(String, String)],
    previous: Option<&BTreeMap<String, String>>,
    facts: HostFacts<'_>,
    prompt: Option<ParameterPrompt<'_>>,
    origins: Option<&BTreeMap<String, vibe_wire::generated::shared::ConfigurationParameterOrigin>>,
) -> Result<ParameterResolution, StepError> {
    let invalid = |parameter: &str, reason: String| StepError::ParameterInvalid {
        option: option.id.clone(),
        parameter: parameter.into(),
        reason,
    };
    for (id, _) in set {
        if !option
            .parameters
            .iter()
            .any(|parameter| &parameter.id == id)
        {
            let declared: Vec<&str> = option
                .parameters
                .iter()
                .map(|parameter| parameter.id.as_str())
                .collect();
            return Err(invalid(
                id,
                format!(
                    "no such parameter is declared; declared: {}",
                    declared.join(", ")
                ),
            ));
        }
    }
    let mut values = Vec::new();
    let mut supplied_secrets = Vec::new();
    let mut secret_origins = BTreeMap::new();
    for parameter in &option.parameters {
        let needs_resupply = parameter.secret
            && origins.and_then(|o| o.get(&parameter.id)).is_some_and(|o| {
                matches!(
                    o,
                    vibe_wire::generated::shared::ConfigurationParameterOrigin::Explicit(_)
                )
            });
        let default = if needs_resupply {
            None
        } else {
            parameter.default.as_ref().map(ToString::to_string)
        };
        let named = set
            .iter()
            .rev()
            .find(|(id, _)| id == &parameter.id)
            .map(|(_, value)| value.clone());
        let recorded = if parameter.secret {
            None
        } else {
            previous.and_then(|previous| previous.get(&parameter.id).cloned())
        };
        let (raw, provided) = match (named, recorded, prompt) {
            (Some(value), _, _) => (Some(value), true),
            (None, Some(value), _) => (Some(value), false),
            (None, None, Some(ask)) => {
                let supplied = ask(parameter, default.as_deref()).map_err(|reason| {
                    invalid(
                        &parameter.id,
                        if parameter.secret {
                            "the prompt failed for a secret parameter".into()
                        } else {
                            format!("the prompt failed: {reason}")
                        },
                    )
                })?;
                let provided = supplied.is_some();
                (supplied.or(default), provided)
            }
            (None, None, None) => (default, false),
        };
        if needs_resupply && !provided {
            return Err(invalid(
                &parameter.id,
                "a previously supplied secret must be provided again for this operation".into(),
            ));
        }
        if parameter.secret {
            use vibe_wire::generated::shared as w;
            let origin = if provided {
                w::ConfigurationParameterOrigin::Explicit(Box::new(
                    w::ConfigurationParameterOriginExplicit {},
                ))
            } else if raw.is_some() {
                w::ConfigurationParameterOrigin::Default(Box::new(
                    w::ConfigurationParameterOriginDefault {},
                ))
            } else {
                w::ConfigurationParameterOrigin::Absent(Box::new(
                    w::ConfigurationParameterOriginAbsent {},
                ))
            };
            secret_origins.insert(parameter.id.clone(), origin);
        }
        let Some(raw) = raw else {
            if parameter.required {
                return Err(invalid(
                    &parameter.id,
                    "it is required and has no value".into(),
                ));
            }
            continue;
        };
        let mut value = parameter
            .canonical(&raw)
            .map_err(|reason| invalid(&parameter.id, reason))?;
        if parameter.kind == ParameterType::Path {
            let path = (facts.resolve_path)(&value);
            if parameter.must_exist && !path.exists() {
                return Err(invalid(
                    &parameter.id,
                    if parameter.secret {
                        "the secret path does not exist".into()
                    } else {
                        format!("`{}` does not exist", path.display())
                    },
                ));
            }
            value = path.to_string_lossy().into_owned();
        }
        if parameter.secret && provided {
            supplied_secrets.push(parameter.id.clone());
        }
        values.push(ParamValue {
            id: parameter.id.clone(),
            value,
            secret: parameter.secret,
        });
    }
    Ok(ParameterResolution {
        values,
        supplied_secrets,
        secret_origins,
    })
}

fn declarations(option: &InstallerOptionDecl) -> &[ParameterDecl] {
    option
        .configuration
        .as_ref()
        .map_or(option.parameters.as_slice(), |c| c.parameters.as_slice())
}

/// `param = "ID"`: a `bool` that is true, or any non-empty value.
pub(crate) fn param_holds(option: &InstallerOptionDecl, values: &[ParamValue], id: &str) -> bool {
    let bool_typed = declarations(option)
        .iter()
        .any(|parameter| parameter.id == id && parameter.kind == ParameterType::Bool);
    values
        .iter()
        .find(|value| value.id == id)
        .is_some_and(|value| {
            if bool_typed {
                value.value == "true"
            } else {
                !value.value.is_empty()
            }
        })
}

/// `param_equals = { id, value }`, comparing canonical forms.
pub(crate) fn param_equals(
    option: &InstallerOptionDecl,
    values: &[ParamValue],
    id: &str,
    expected: &str,
) -> bool {
    let expected = declarations(option)
        .iter()
        .find(|parameter| parameter.id == id)
        .and_then(|parameter| parameter.canonical(expected).ok())
        .unwrap_or_else(|| expected.to_owned());
    values
        .iter()
        .any(|value| value.id == id && value.value == expected)
}

/// The license of an installation as planned.
#[derive(Debug, Clone, PartialEq, Eq)]
#[spec(documents = "spec://org.vibevm.core/vibevm/common/PROP-065#license")]
pub struct PlanLicense {
    pub name: String,
    /// The absolute path of the license text in the package tree.
    pub file: String,
}

/// Admit the declared license: `--accept-license`, the same license
/// accepted by the recorded installation, or a yes in an attended terminal
/// after the text is shown; anything else refuses before any download.
pub(crate) fn admit_license(
    option: &InstallerOptionDecl,
    key: &str,
    package_root: &Path,
    accepted: bool,
    previously: Option<&str>,
    prompt: Option<LicensePrompt<'_>>,
) -> Result<Option<PlanLicense>, StepError> {
    let Some(license) = &option.license else {
        return Ok(None);
    };
    let file = package_root.join(&license.file);
    let text = std::fs::read_to_string(&file).map_err(|error| StepError::LicenseUnreadable {
        key: key.into(),
        file: file.display().to_string(),
        reason: error.to_string(),
    })?;
    let required = || StepError::LicenseRequired {
        key: key.into(),
        license: license.name.clone(),
        file: file.display().to_string(),
    };
    let admitted = accepted
        || previously == Some(license.name.as_str())
        || match prompt {
            Some(ask) => ask(&license.name, &text).map_err(|_| required())?,
            None => false,
        };
    if !admitted {
        return Err(required());
    }
    Ok(Some(PlanLicense {
        name: license.name.clone(),
        file: file.display().to_string(),
    }))
}

#[cfg(test)]
#[path = "params/tests.rs"]
mod tests;
